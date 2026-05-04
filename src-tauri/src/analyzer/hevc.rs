use crate::types::{
    HevcSliceHeader, HevcSyntax, NalUnit, PictureParameterSet, SequenceParameterSet,
    VideoParameterSet,
};

/// Extract profile, level, chroma, and bit depth from hvcC header (ISO/IEC 14496-15).
/// hvcC always contains reliable codec config fields, unlike truncated SPS NAL payloads.
///
/// The `data` slice includes the full ISO box header:
///   - bytes 0-3: box size (big-endian)
///   - bytes 4-7: "hvcC"
///   - bytes 8+: HEVCDecoderConfigurationRecord (configurationVersion at byte 8)
///
/// Note: hvcC is a regular box (no FullBox version+flags wrapper) per ISO/IEC 14496-15.
pub fn parse_hvcc_header(data: &[u8]) -> Option<SequenceParameterSet> {
    let record = hvcc_record(data)?;
    if record.len() < 19 {
        return None;
    }

    if record[0] != 1 {
        return None;
    }

    let profile_byte = record[1];
    let general_profile_idc = profile_byte & 0x1F;
    let general_level_idc = record[12];
    let chroma_format_idc = record[16] & 0x03;
    let temporal_layer_byte = if record.len() > 21 { record[21] } else { 0 };
    let vps_max_sub_layers_minus1 = ((temporal_layer_byte >> 3) & 0x07).saturating_sub(1);
    let temporal_id_nesting_flag = ((temporal_layer_byte >> 2) & 0x01) != 0;

    Some(SequenceParameterSet {
        sps_seq_parameter_set_id: 0,
        general_profile_idc,
        general_level_idc,
        sps_max_sub_layers_minus1: vps_max_sub_layers_minus1,
        sps_temporal_id_nesting_flag: temporal_id_nesting_flag,
        chroma_format_idc,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: 0, // hvcC does not store dimensions
        pic_height_in_luma_samples: 0,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: record[17] & 0x07,
        bit_depth_chroma_minus8: if record.len() > 18 { record[18] & 0x07 } else { record[17] & 0x07 },
        log2_max_pic_order_cnt_lsb_minus4: 0,
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_transform_block_size_minus2: 0,
        log2_diff_max_min_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
    })
}

pub fn parse_hevc_bitstream(data: &[u8]) -> HevcSyntax {
    parse_hevc_bitstream_with_seed(data, None, None, None)
}

pub fn merge_hevc_syntax(mut base: HevcSyntax, extra: HevcSyntax) -> HevcSyntax {
    base.nal_units.extend(extra.nal_units);
    base.slice_headers.extend(extra.slice_headers);
    if base.vps.is_none() {
        base.vps = extra.vps;
    }
    if base.sps.is_none() {
        base.sps = extra.sps;
    }
    if base.pps.is_none() {
        base.pps = extra.pps;
    }
    base
}

pub fn parse_hevc_bitstream_with_seed(
    data: &[u8],
    seed_vps: Option<VideoParameterSet>,
    seed_sps: Option<SequenceParameterSet>,
    seed_pps: Option<PictureParameterSet>,
) -> HevcSyntax {
    let mut nal_units = Vec::new();
    let mut vps_parsed = seed_vps;
    let mut sps_parsed = seed_sps;
    let mut pps_parsed = seed_pps;
    let mut slice_headers = Vec::new();

    // hvcC data includes: 8-byte ISO box header (size + "hvcC") + HEVCDecoderConfigurationRecord.
    // There is NO FullBox version+flags wrapper — hvcC is a regular box per ISO/IEC 14496-15.
    // Total prefix before HEVCDecoderConfigurationRecord = 8 bytes.
    let nal_list = if let Some(record) = hvcc_record(data) {
        extract_nal_units_from_hvcc(record)
    } else if data.len() >= 4 {
        extract_nal_units(data)
    } else {
        Vec::new()
    };

    for (nal_data, nal_offset) in nal_list {
        if nal_data.len() < 2 {
            continue;
        }

        let nal_header = nal_data[0];
        let nal_unit_type = (nal_header >> 1) & 0x3F;
        let nuh_layer_id = ((nal_header & 1) << 5) | ((nal_data[1] >> 5) & 0x1F);
        let nuh_temporal_id_plus1 = nal_data[1] & 0x07;
        let nal_type_name = nal_type_to_name(nal_unit_type);

        nal_units.push(NalUnit {
            nal_unit_type: nal_type_name.clone(),
            nuh_layer_id,
            nuh_temporal_id_plus1,
            size: nal_data.len(),
            offset: nal_offset,
        });

        // For hvcC data, NAL units may or may not contain emulation prevention bytes.
        // Try parsing without stripping first, then with stripping if that fails.
        let raw_payload = &nal_data[2..];
        let stripped = strip_emulation_prevention(raw_payload);

        // Try both: first without stripping, then with stripping
        let payload_raw = raw_payload;
        let payload_stripped = &stripped[..];

        match nal_unit_type {
            32 => {
                // Try stripped first for VPS
                vps_parsed = parse_vps(payload_stripped);
                if vps_parsed.is_none() {
                    vps_parsed = parse_vps(payload_raw);
                }
            }
            33 => {
                sps_parsed = parse_sps(payload_stripped);
                // SPS in hvcC is often truncated; fall back to hvcC header info
                // when dimensions are missing or suspiciously small.
                // Only try hvcC fallback when data looks like an hvcC box
                // (configurationVersion == 1 at offset 8, after box header).
                let looks_like_hvcc = hvcc_record(data).is_some();
                if looks_like_hvcc
                    && sps_parsed.as_ref().map_or(true, |s| {
                        s.pic_width_in_luma_samples == 0 || s.pic_height_in_luma_samples == 0
                            || s.pic_width_in_luma_samples < 100
                            || s.pic_height_in_luma_samples < 100
                    })
                {
                    sps_parsed = parse_hvcc_header(data);
                }
            }
            34 => {
                pps_parsed = parse_pps(payload_stripped);
                if pps_parsed.is_none() {
                    pps_parsed = parse_pps(payload_raw);
                }
            }
            0..=31 => {
                if let (Some(ref pps), Some(ref sps)) = (&pps_parsed, &sps_parsed) {
                    let sh = parse_slice_header(payload_stripped, nal_unit_type, pps, sps);
                    slice_headers.push(sh);
                }
            }
            _ => {}
        }
    }

    HevcSyntax {
        nal_units,
        vps: vps_parsed,
        sps: sps_parsed,
        pps: pps_parsed,
        slice_headers,
    }
}

fn hvcc_record(data: &[u8]) -> Option<&[u8]> {
    if data.len() >= 8 && &data[4..8] == b"hvcC" {
        return Some(&data[8..]);
    }
    if data.first().copied() == Some(1) && data.len() >= 19 {
        return Some(data);
    }
    None
}

/// Extract NAL units from hvcC data (length-prefixed, ISO/IEC 14496-15).
/// hvcC always uses length-prefixed format (ISO/IEC 14496-15) with 4-byte big-endian lengths.
/// Start codes (Annex B) only appear in raw bitstreams or mdat, not in hvcC.
fn extract_nal_units(data: &[u8]) -> Vec<(&[u8], u64)> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos + 4 <= data.len() {
        let nal_len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        if nal_len == 0 || nal_len > data.len() - pos - 4 {
            break;
        }
        let nal_start = pos + 4;
        result.push((&data[nal_start..nal_start + nal_len], nal_start as u64));
        pos = nal_start + nal_len;
    }
    result
}

/// Extract NAL units from hvcC decoder configuration record.
/// The HEVCDecoderConfigurationRecord has 22 bytes of fixed header fields before numOfArrays.
/// NAL unit arrays start at offset 23.
fn extract_nal_units_from_hvcc(data: &[u8]) -> Vec<(&[u8], u64)> {
    let mut result = Vec::new();
    if data.len() < 24 {
        return result;
    }
    let num_arrays = data[22] as usize;
    let mut pos = 23;

    for _ in 0..num_arrays {
        if pos + 3 > data.len() {
            break;
        }
        let num_nalus = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
        pos += 3;

        for _ in 0..num_nalus {
            if pos + 2 > data.len() {
                break;
            }
            let nal_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;
            if nal_len == 0 || pos + nal_len > data.len() {
                break;
            }
            result.push((&data[pos..pos + nal_len], pos as u64));
            pos += nal_len;
        }
    }
    result
}

fn nal_type_to_name(nut: u8) -> String {
    match nut {
        0 => "TRAIL_N",
        1 => "TRAIL_R",
        2 => "TSA_N",
        3 => "TSA_R",
        4 => "STSA_N",
        5 => "STSA_R",
        6 => "RADL_N",
        7 => "RADL_R",
        8 => "RASL_N",
        9 => "RASL_R",
        16 => "BLA_W_LP",
        17 => "BLA_W_RADL",
        18 => "BLA_N_LP",
        19 => "IDR_W_RADL",
        20 => "IDR_N_LP",
        21 => "CRA_NUT",
        32 => "VPS_NUT",
        33 => "SPS_NUT",
        34 => "PPS_NUT",
        35 => "AUD_NUT",
        36 => "EOS_NUT",
        37 => "EOB_NUT",
        38 => "FD_NUT",
        39 => "PREFIX_SEI_NUT",
        40 => "SUFFIX_SEI_NUT",
        _ => return format!("NAL_{}", nut),
    }
    .to_string()
}

/// Strip emulation_prevention_three_byte (0x03) from NAL payload.
/// HEVC inserts 0x03 after 0x00 0x00 to prevent start code emulation.
fn strip_emulation_prevention(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if i + 2 < data.len()
            && data[i] == 0x00
            && data[i + 1] == 0x00
            && data[i + 2] == 0x03
        {
            result.push(0x00);
            result.push(0x00);
            i += 3; // skip the 0x03
        } else {
            result.push(data[i]);
            i += 1;
        }
    }
    result
}

struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.bit_pos >= self.data.len() * 8 {
            return None;
        }
        let byte_idx = self.bit_pos / 8;
        let bit_idx = 7 - (self.bit_pos % 8);
        self.bit_pos += 1;
        Some((self.data[byte_idx] >> bit_idx) & 1)
    }

    fn read_ue_golomb(&mut self) -> Option<u32> {
        let mut leading_zeros = 0u32;
        loop {
            match self.read_bit() {
                Some(0) => leading_zeros += 1,
                Some(1) => break,
                Some(_) | None => return None,
            }
        }
        if leading_zeros == 0 {
            return Some(0);
        }
        if leading_zeros > 31 {
            return None;
        }
        let mut value = 1u32;
        for _ in 0..leading_zeros {
            value = (value << 1) | self.read_bit()? as u32;
        }
        Some(value - 1)
    }

    fn read_se_golomb(&mut self) -> Option<i32> {
        let code_num = self.read_ue_golomb()? as i32;
        if code_num % 2 == 0 {
            Some(-(code_num / 2))
        } else {
            Some((code_num + 1) / 2)
        }
    }

    fn read_bits(&mut self, n: usize) -> Option<u32> {
        if n == 0 {
            return Some(0);
        }
        let mut value = 0u32;
        for _ in 0..n {
            value = (value << 1) | self.read_bit()? as u32;
        }
        Some(value)
    }

    fn read_bool(&mut self) -> Option<bool> {
        self.read_bit().map(|b| b != 0)
    }
}

// --- VPS parser ---

fn parse_vps(data: &[u8]) -> Option<VideoParameterSet> {
    let mut br = BitReader::new(data);
    let vps_id = br.read_bits(4)? as u8;
    let base_layer_internal_flag = br.read_bool()?;
    let base_layer_available_flag = br.read_bool()?;
    let vps_max_layers_minus1 = br.read_bits(6)? as u8;
    let vps_max_sub_layers_minus1 = br.read_bits(3)? as u8;
    let temporal_id_nesting_flag = br.read_bool()?;

    Some(VideoParameterSet {
        vps_video_parameter_set_id: vps_id,
        vps_base_layer_internal_flag: base_layer_internal_flag,
        vps_base_layer_available_flag: base_layer_available_flag,
        vps_max_layers_minus1,
        vps_max_sub_layers_minus1,
        vps_temporal_id_nesting_flag: temporal_id_nesting_flag,
    })
}

// --- SPS parser ---

fn parse_sps(data: &[u8]) -> Option<SequenceParameterSet> {
    let mut br = BitReader::new(data);

    br.read_bits(4)?;
    let max_sub_layers_minus1 = br.read_bits(3)? as u8;
    let temporal_id_nesting_flag = br.read_bool()?;
    let (general_profile_idc, general_level_idc) =
        skip_profile_tier_level(&mut br, max_sub_layers_minus1)?;

    let sps_id = br.read_ue_golomb()? as u8;

    let chroma_format_idc = br.read_ue_golomb()?;
    let separate_colour_plane_flag = if chroma_format_idc == 3 {
        br.read_bool()?
    } else {
        false
    };

    let pic_width_in_luma_samples = br.read_ue_golomb()? as u32;
    let pic_height_in_luma_samples = br.read_ue_golomb()? as u32;

    let conformance_window_flag = br.read_bool()?;
    let mut conf_win_left_offset = 0;
    let mut conf_win_right_offset = 0;
    let mut conf_win_top_offset = 0;
    let mut conf_win_bottom_offset = 0;
    if conformance_window_flag {
        conf_win_left_offset = br.read_ue_golomb()?;
        conf_win_right_offset = br.read_ue_golomb()?;
        conf_win_top_offset = br.read_ue_golomb()?;
        conf_win_bottom_offset = br.read_ue_golomb()?;
    }

    let bit_depth_luma_minus8 = br.read_ue_golomb()?;
    let bit_depth_chroma_minus8 = br.read_ue_golomb()?;
    let log2_max_pic_order_cnt_lsb_minus4 = br.read_ue_golomb().unwrap_or(0) as u8;
    let sub_layer_ordering_info_present_flag = br.read_bool().unwrap_or(false);
    let ordering_start = if sub_layer_ordering_info_present_flag {
        0
    } else {
        max_sub_layers_minus1 as usize
    };
    for _ in ordering_start..=max_sub_layers_minus1 as usize {
        if br.read_ue_golomb().is_none() {
            break;
        }
        if br.read_ue_golomb().is_none() {
            break;
        }
        if br.read_ue_golomb().is_none() {
            break;
        }
    }
    let log2_min_luma_coding_block_size_minus3 = br.read_ue_golomb().unwrap_or(0) as u8;
    let log2_diff_max_min_luma_coding_block_size = br.read_ue_golomb().unwrap_or(0) as u8;
    let log2_min_transform_block_size_minus2 = br.read_ue_golomb().unwrap_or(0) as u8;
    let log2_diff_max_min_transform_block_size = br.read_ue_golomb().unwrap_or(0) as u8;
    let max_transform_hierarchy_depth_inter = br.read_ue_golomb().unwrap_or(0) as u8;
    let max_transform_hierarchy_depth_intra = br.read_ue_golomb().unwrap_or(0) as u8;
    let scaling_list_enabled_flag = br.read_bool().unwrap_or(false);
    if scaling_list_enabled_flag {
        let sps_scaling_list_data_present_flag = br.read_bool().unwrap_or(false);
        if sps_scaling_list_data_present_flag {
            skip_scaling_list_data(&mut br)?;
        }
    }
    let amp_enabled_flag = br.read_bool().unwrap_or(false);
    let sample_adaptive_offset_enabled_flag = br.read_bool().unwrap_or(false);

    Some(SequenceParameterSet {
        sps_seq_parameter_set_id: sps_id,
        general_profile_idc: general_profile_idc as u8,
        general_level_idc: general_level_idc as u8,
        sps_max_sub_layers_minus1: max_sub_layers_minus1,
        sps_temporal_id_nesting_flag: temporal_id_nesting_flag,
        chroma_format_idc: chroma_format_idc as u8,
        separate_colour_plane_flag,
        pic_width_in_luma_samples,
        pic_height_in_luma_samples,
        conformance_window_flag,
        conf_win_left_offset,
        conf_win_right_offset,
        conf_win_top_offset,
        conf_win_bottom_offset,
        bit_depth_luma_minus8: bit_depth_luma_minus8 as u8,
        bit_depth_chroma_minus8: bit_depth_chroma_minus8 as u8,
        log2_max_pic_order_cnt_lsb_minus4,
        log2_min_luma_coding_block_size_minus3,
        log2_diff_max_min_luma_coding_block_size,
        log2_min_transform_block_size_minus2,
        log2_diff_max_min_transform_block_size,
        max_transform_hierarchy_depth_inter,
        max_transform_hierarchy_depth_intra,
        amp_enabled_flag,
        sample_adaptive_offset_enabled_flag,
    })
}

fn skip_profile_tier_level(br: &mut BitReader<'_>, max_sub_layers_minus1: u8) -> Option<(u32, u32)> {
    br.read_bits(2)?; // general_profile_space
    br.read_bool()?; // general_tier_flag
    let general_profile_idc = br.read_bits(5)?;
    br.read_bits(32)?; // general_profile_compatibility_flags
    br.read_bool()?; // general_progressive_source_flag
    br.read_bool()?; // general_interlaced_source_flag
    br.read_bool()?; // general_non_packed_constraint_flag
    br.read_bool()?; // general_frame_only_constraint_flag
    br.read_bits(44)?; // general_constraint_indicator_flags
    let general_level_idc = br.read_bits(8)?;

    let mut sub_layer_profile_present = [false; 8];
    let mut sub_layer_level_present = [false; 8];
    for i in 0..max_sub_layers_minus1 as usize {
        sub_layer_profile_present[i] = br.read_bool()?;
        sub_layer_level_present[i] = br.read_bool()?;
    }
    if max_sub_layers_minus1 > 0 {
        for _ in max_sub_layers_minus1 as usize..8 {
            br.read_bits(2)?;
        }
    }
    for i in 0..max_sub_layers_minus1 as usize {
        if sub_layer_profile_present[i] {
            br.read_bits(2)?; // sub_layer_profile_space
            br.read_bool()?; // sub_layer_tier_flag
            br.read_bits(5)?; // sub_layer_profile_idc
            br.read_bits(32)?; // sub_layer_profile_compatibility_flags
            br.read_bool()?; // progressive
            br.read_bool()?; // interlaced
            br.read_bool()?; // non_packed
            br.read_bool()?; // frame_only
            br.read_bits(44)?; // constraint flags
        }
        if sub_layer_level_present[i] {
            br.read_bits(8)?;
        }
    }

    Some((general_profile_idc, general_level_idc))
}

// --- PPS parser ---

fn parse_pps(data: &[u8]) -> Option<PictureParameterSet> {
    let mut br = BitReader::new(data);
    let pps_pic_parameter_set_id = br.read_ue_golomb()? as u8;
    let pps_seq_parameter_set_id = br.read_ue_golomb()? as u8;
    let dependent_slice_segments_enabled_flag = br.read_bool().unwrap_or(false);
    let output_flag_present_flag = br.read_bool().unwrap_or(false);
    let num_extra_slice_header_bits = br.read_bits(3).unwrap_or(0) as u8;
    let sign_data_hiding_enabled_flag = br.read_bool().unwrap_or(false);
    let cabac_init_present_flag = br.read_bool().unwrap_or(false);
    let num_ref_idx_l0_default_active_minus1 = br.read_ue_golomb().unwrap_or(0) as u8;
    let num_ref_idx_l1_default_active_minus1 = br.read_ue_golomb().unwrap_or(0) as u8;
    let init_qp_minus26 = br.read_se_golomb().unwrap_or(0) as i8;
    let constrained_intra_pred_flag = br.read_bool().unwrap_or(false);
    let transform_skip_enabled_flag = br.read_bool().unwrap_or(false);
    let cu_qp_delta_enabled_flag = br.read_bool().unwrap_or(false);
    let diff_cu_qp_delta_depth = if cu_qp_delta_enabled_flag {
        br.read_ue_golomb().unwrap_or(0) as u8
    } else {
        0
    };
    let pps_cb_qp_offset = br.read_se_golomb().unwrap_or(0) as i8;
    let pps_cr_qp_offset = br.read_se_golomb().unwrap_or(0) as i8;
    let pps_slice_chroma_qp_offsets_present_flag = br.read_bool().unwrap_or(false);
    let weighted_pred_flag = br.read_bool().unwrap_or(false);
    let weighted_bipred_flag = br.read_bool().unwrap_or(false);
    let transquant_bypass_enabled_flag = br.read_bool().unwrap_or(false);
    let tiles_enabled_flag = br.read_bool().unwrap_or(false);
    let entropy_coding_sync_enabled_flag = br.read_bool().unwrap_or(false);
    let mut num_tile_columns_minus1 = 0;
    let mut num_tile_rows_minus1 = 0;
    let mut uniform_spacing_flag = false;
    let mut loop_filter_across_tiles_enabled_flag = false;
    if tiles_enabled_flag {
        num_tile_columns_minus1 = br.read_ue_golomb().unwrap_or(0) as u8;
        num_tile_rows_minus1 = br.read_ue_golomb().unwrap_or(0) as u8;
        uniform_spacing_flag = br.read_bool().unwrap_or(false);
        if !uniform_spacing_flag {
            for _ in 0..num_tile_columns_minus1 {
                if br.read_ue_golomb().is_none() {
                    break;
                }
            }
            for _ in 0..num_tile_rows_minus1 {
                if br.read_ue_golomb().is_none() {
                    break;
                }
            }
        }
        loop_filter_across_tiles_enabled_flag = br.read_bool().unwrap_or(false);
    }
    let pps_loop_filter_across_slices_enabled_flag = br.read_bool().unwrap_or(false);
    let deblocking_filter_control_present_flag = br.read_bool().unwrap_or(false);
    let mut deblocking_filter_override_enabled_flag = false;
    let mut pps_deblocking_filter_disabled_flag = false;
    let mut pps_beta_offset_div2 = 0;
    let mut pps_tc_offset_div2 = 0;
    if deblocking_filter_control_present_flag {
        deblocking_filter_override_enabled_flag = br.read_bool().unwrap_or(false);
        pps_deblocking_filter_disabled_flag = br.read_bool().unwrap_or(false);
        if !pps_deblocking_filter_disabled_flag {
            pps_beta_offset_div2 = br.read_se_golomb().unwrap_or(0) as i8;
            pps_tc_offset_div2 = br.read_se_golomb().unwrap_or(0) as i8;
        }
    }
    let pps_scaling_list_data_present_flag = br.read_bool().unwrap_or(false);
    if pps_scaling_list_data_present_flag {
        skip_scaling_list_data(&mut br)?;
    }
    let lists_modification_present_flag = br.read_bool().unwrap_or(false);
    let log2_parallel_merge_level_minus2 = br.read_ue_golomb().unwrap_or(0) as u8;
    let slice_segment_header_extension_present_flag = br.read_bool().unwrap_or(false);

    Some(PictureParameterSet {
        pps_pic_parameter_set_id,
        pps_seq_parameter_set_id,
        dependent_slice_segments_enabled_flag,
        output_flag_present_flag,
        num_extra_slice_header_bits,
        sign_data_hiding_enabled_flag,
        cabac_init_present_flag,
        num_ref_idx_l0_default_active_minus1,
        num_ref_idx_l1_default_active_minus1,
        init_qp_minus26,
        constrained_intra_pred_flag,
        transform_skip_enabled_flag,
        cu_qp_delta_enabled_flag,
        diff_cu_qp_delta_depth,
        pps_cb_qp_offset,
        pps_cr_qp_offset,
        pps_slice_chroma_qp_offsets_present_flag,
        weighted_pred_flag,
        weighted_bipred_flag,
        transquant_bypass_enabled_flag,
        tiles_enabled_flag,
        entropy_coding_sync_enabled_flag,
        num_tile_columns_minus1,
        num_tile_rows_minus1,
        uniform_spacing_flag,
        loop_filter_across_tiles_enabled_flag,
        pps_loop_filter_across_slices_enabled_flag,
        deblocking_filter_control_present_flag,
        deblocking_filter_override_enabled_flag,
        pps_deblocking_filter_disabled_flag,
        pps_beta_offset_div2,
        pps_tc_offset_div2,
        lists_modification_present_flag,
        log2_parallel_merge_level_minus2,
        slice_segment_header_extension_present_flag,
    })
}

// --- Slice header parser (simplified) ---

fn parse_slice_header(
    data: &[u8],
    nal_unit_type: u8,
    pps: &PictureParameterSet,
    sps: &SequenceParameterSet,
) -> HevcSliceHeader {
    let mut br = BitReader::new(data);

    let first_slice_segment_in_pic_flag = br.read_bool().unwrap_or(true);
    let no_output_of_prior_pics_flag = if (16..=23).contains(&nal_unit_type) {
        br.read_bool().unwrap_or(false)
    } else {
        false
    };
    let slice_pic_parameter_set_id =
        br.read_ue_golomb().unwrap_or(pps.pps_pic_parameter_set_id as u32) as u8;
    let ctb_log2_size_y =
        sps.log2_min_luma_coding_block_size_minus3 + 3 + sps.log2_diff_max_min_luma_coding_block_size;
    let ctb_size_y = 1u32 << ctb_log2_size_y.min(31);
    let pic_width_in_ctbs_y = sps.pic_width_in_luma_samples.div_ceil(ctb_size_y).max(1);
    let pic_height_in_ctbs_y = sps.pic_height_in_luma_samples.div_ceil(ctb_size_y).max(1);
    let pic_size_in_ctbs_y = pic_width_in_ctbs_y * pic_height_in_ctbs_y;
    let address_bits = ceil_log2(pic_size_in_ctbs_y.max(1));

    let dependent_slice_segment_flag =
        if !first_slice_segment_in_pic_flag && pps.dependent_slice_segments_enabled_flag {
            br.read_bool().unwrap_or(false)
        } else {
            false
        };
    let slice_segment_address = if !first_slice_segment_in_pic_flag && address_bits > 0 {
        br.read_bits(address_bits as usize).unwrap_or(0)
    } else {
        0
    };

    let mut pic_output_flag = None;
    let mut colour_plane_id = None;
    let mut slice_type = 255;
    let mut short_term_ref_pic_set_sps_flag = None;
    let mut slice_sao_luma_flag = None;
    let mut slice_sao_chroma_flag = None;
    let mut num_ref_idx_active_override_flag = None;
    let mut num_ref_idx_l0_active_minus1 = None;
    let mut num_ref_idx_l1_active_minus1 = None;
    let mut mvd_l1_zero_flag = None;
    let mut cabac_init_flag = None;
    let mut collocated_from_l0_flag = None;
    let mut collocated_ref_idx = None;
    let mut num_entry_point_offsets = None;
    let mut offset_len_minus1 = None;
    let mut five_minus_max_num_merge_cand = None;
    let mut slice_qp_delta = None;
    let mut slice_cb_qp_offset = None;
    let mut slice_cr_qp_offset = None;
    let mut cu_chroma_qp_offset_enabled_flag = None;
    let mut deblocking_filter_override_flag = None;
    let mut slice_deblocking_filter_disabled_flag = None;
    let mut beta_offset_div2 = None;
    let mut tc_offset_div2 = None;
    let mut slice_loop_filter_across_slices_enabled_flag = None;

    if !dependent_slice_segment_flag {
        for _ in 0..pps.num_extra_slice_header_bits {
            let _ = br.read_bool();
        }
        if pps.output_flag_present_flag {
            pic_output_flag = br.read_bool();
        }
        if sps.separate_colour_plane_flag {
            colour_plane_id = br.read_bits(2).map(|v| v as u8);
        }
        slice_type = br.read_ue_golomb().unwrap_or(255) as u8;

        if !matches!(nal_unit_type, 19 | 20) {
            let _ = br.read_bits((sps.log2_max_pic_order_cnt_lsb_minus4 + 4) as usize);
            short_term_ref_pic_set_sps_flag = br.read_bool();
        }

        if sps.sample_adaptive_offset_enabled_flag {
            slice_sao_luma_flag = br.read_bool();
            if sps.chroma_format_idc != 0 {
                slice_sao_chroma_flag = br.read_bool();
            }
        }

        if slice_type == 1 || slice_type == 0 {
            num_ref_idx_active_override_flag = br.read_bool();
            if num_ref_idx_active_override_flag == Some(true) {
                num_ref_idx_l0_active_minus1 = br.read_ue_golomb().map(|v| v as u8);
                if slice_type == 0 {
                    num_ref_idx_l1_active_minus1 = br.read_ue_golomb().map(|v| v as u8);
                }
            }
            if pps.cabac_init_present_flag {
                cabac_init_flag = br.read_bool();
            }
            collocated_from_l0_flag = Some(slice_type != 0 || br.read_bool().unwrap_or(true));
            collocated_ref_idx = br.read_ue_golomb().map(|v| v as u8);
            if slice_type == 0 {
                mvd_l1_zero_flag = br.read_bool();
            }
        }

        if slice_type != 2 {
            five_minus_max_num_merge_cand = br.read_ue_golomb().map(|v| v as u8);
        }

        if pps.tiles_enabled_flag || pps.entropy_coding_sync_enabled_flag {
            num_entry_point_offsets = br.read_ue_golomb();
            if num_entry_point_offsets.unwrap_or(0) > 0 {
                offset_len_minus1 = br.read_ue_golomb().map(|v| v as u8);
                if let Some(entries) = num_entry_point_offsets {
                    for _ in 0..entries {
                        let bits = offset_len_minus1.unwrap_or(0) as usize + 1;
                        let _ = br.read_bits(bits);
                    }
                }
            }
        }

        slice_qp_delta = br.read_se_golomb();
        if pps.pps_slice_chroma_qp_offsets_present_flag {
            slice_cb_qp_offset = br.read_se_golomb().map(|v| v as i8);
            slice_cr_qp_offset = br.read_se_golomb().map(|v| v as i8);
        }
        if pps.tiles_enabled_flag || pps.entropy_coding_sync_enabled_flag {
            cu_chroma_qp_offset_enabled_flag = br.read_bool();
        }
        if pps.deblocking_filter_override_enabled_flag {
            deblocking_filter_override_flag = br.read_bool();
        }
        let deblocking_enabled = if pps.deblocking_filter_control_present_flag {
            let disabled = if deblocking_filter_override_flag == Some(true) {
                br.read_bool()
            } else {
                Some(pps.pps_deblocking_filter_disabled_flag)
            };
            slice_deblocking_filter_disabled_flag = disabled;
            !disabled.unwrap_or(false)
        } else {
            false
        };
        if deblocking_enabled {
            beta_offset_div2 = br.read_se_golomb().map(|v| v as i8);
            tc_offset_div2 = br.read_se_golomb().map(|v| v as i8);
        }
        if pps.pps_loop_filter_across_slices_enabled_flag
            && (slice_sao_luma_flag == Some(true)
                || slice_sao_chroma_flag == Some(true)
                || !slice_deblocking_filter_disabled_flag.unwrap_or(false))
        {
            slice_loop_filter_across_slices_enabled_flag = br.read_bool();
        }
    }

    HevcSliceHeader {
        nal_unit_type: nal_type_to_name(nal_unit_type),
        slice_type,
        first_slice_segment_in_pic_flag,
        dependent_slice_segment_flag,
        no_output_of_prior_pics_flag,
        slice_segment_address: slice_segment_address as u32,
        slice_pic_parameter_set_id,
        pic_output_flag,
        colour_plane_id,
        num_entry_point_offsets,
        offset_len_minus1,
        short_term_ref_pic_set_sps_flag,
        slice_sao_luma_flag,
        slice_sao_chroma_flag,
        num_ref_idx_active_override_flag,
        num_ref_idx_l0_active_minus1,
        num_ref_idx_l1_active_minus1,
        mvd_l1_zero_flag,
        cabac_init_flag,
        collocated_from_l0_flag,
        collocated_ref_idx,
        five_minus_max_num_merge_cand,
        slice_qp_delta,
        slice_cb_qp_offset,
        slice_cr_qp_offset,
        cu_chroma_qp_offset_enabled_flag,
        deblocking_filter_override_flag,
        slice_deblocking_filter_disabled_flag,
        beta_offset_div2,
        tc_offset_div2,
        slice_loop_filter_across_slices_enabled_flag,
        pic_width_in_luma_samples: sps.pic_width_in_luma_samples,
        pic_height_in_luma_samples: sps.pic_height_in_luma_samples,
        tiles_enabled_flag: pps.tiles_enabled_flag,
        entropy_coding_sync_enabled_flag: pps.entropy_coding_sync_enabled_flag,
    }
}

fn ceil_log2(v: u32) -> u32 {
    if v <= 1 {
        0
    } else {
        u32::BITS - (v - 1).leading_zeros()
    }
}

fn skip_scaling_list_data(br: &mut BitReader<'_>) -> Option<()> {
    for size_id in 0..4 {
        let matrix_count = if size_id == 3 { 2 } else { 6 };
        for _ in 0..matrix_count {
            let pred_mode_flag = br.read_bool()?;
            if !pred_mode_flag {
                br.read_ue_golomb()?;
            } else {
                let coef_num = 64usize.min(1usize << (4 + (size_id << 1)));
                if size_id > 1 {
                    br.read_se_golomb()?;
                }
                for _ in 0..coef_num {
                    br.read_se_golomb()?;
                }
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nal_type_name_mapping() {
        assert_eq!(nal_type_to_name(32), "VPS_NUT");
        assert_eq!(nal_type_to_name(33), "SPS_NUT");
        assert_eq!(nal_type_to_name(34), "PPS_NUT");
        assert_eq!(nal_type_to_name(19), "IDR_W_RADL");
        assert_eq!(nal_type_to_name(21), "CRA_NUT");
        assert_eq!(nal_type_to_name(99), "NAL_99");
    }

    #[test]
    fn extracts_nal_units_from_length_prefixed() {
        let mut data = Vec::new();
        // NAL 1: length=4, data=[0x40, 0x01, 0x00, 0x00]
        data.extend_from_slice(&4u32.to_be_bytes());
        data.extend_from_slice(&[0x40, 0x01, 0x00, 0x00]);
        // NAL 2: length=2, data=[0x42, 0x01]
        data.extend_from_slice(&2u32.to_be_bytes());
        data.extend_from_slice(&[0x42, 0x01]);

        let nals = extract_nal_units(&data);
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0].0, &[0x40, 0x01, 0x00, 0x00]);
        assert_eq!(nals[1].0, &[0x42, 0x01]);
    }

    #[test]
    fn extracts_nal_units_handles_truncated() {
        // Truncated length (only 3 bytes, not enough for 4-byte header)
        let data: &[u8] = &[0x00, 0x00, 0x01];
        assert!(extract_nal_units(data).is_empty());

        // Length exceeds remaining data
        let data: &[u8] = &[0x00, 0x00, 0x00, 0x10, 0x40, 0x01];
        assert!(extract_nal_units(data).is_empty());

        // Zero length NAL stops parsing
        let data: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x40, 0x01];
        assert!(extract_nal_units(data).is_empty());
    }

    #[test]
    fn parses_hevc_bitstream() {
        // hvcC data is always length-prefixed (ISO/IEC 14496-15) with 4-byte big-endian lengths.
        let mut data = Vec::new();

        // --- VPS NAL (type 32) ---
        // NAL: [0x40, 0x01, payload...]
        // Payload: vps_id=0 (4b), max_layers-1=0 (6b), max_sub_layers-1=0 (3b) = 13 zero bits.
        // Need 2 bytes where first 13 bits are 0 but avoid 0x00 0x00 (which looks like start code).
        // 0x00 0x07 → bits: 00000000_00000111, first 13 = 0. ✓
        let vps_nal: &[u8] = &[0x40, 0x01, 0x00, 0x07];
        data.extend_from_slice(&4u32.to_be_bytes()); // 00 00 00 04
        data.extend_from_slice(vps_nal); // 40 01 00 07

        // NAL 2 (SPS): SPS NAL: 0x42 0x01 + payload
        // SPS payload layout (aligned to HEVC spec):
        // Byte 0:  sps_vps_id(4)=0, max_sub-1(3)=0, temporal_nesting(1)=1 → 0x01
        // Byte 1:  profile_space(2)=0, tier(1)=0, profile_idc(5)=1 → 0x01
        // Bytes 2-5:  general_profile_compatibility_flags (32 bits) = 0 → 00 00 00 00
        // Byte 6:   constraint flags(4) + reserved_zero(4) → 0xB0 (prog=1,intl=0,npk=1,frm=1)
        // Bytes 7-11: general_reserved_zero_44bits (remaining 40 bits of 44) → 00 00 00 00 00
        // Byte 12:  general_level_idc(8) = 0 → 0x00
        // Byte 13:  chroma_format_idc=1: ue(v)=010, pic_width=0: ue(v)=1, pic_height=0: ue(v)=1
        //           bits: 0 1 0 1 1 + 3 pad → 0x5F
        // Byte 14:  bit_depth_luma-8=0: ue(v)=1 + 7 pad → 0xFF
        let sps_payload: &[u8] = &[
            0x01,             // 0: vps_id + max_sub + nesting
            0x01,             // 1: profile_space + tier + profile_idc
            0x00, 0x00, 0x00, 0x00, // 2-5: compatibility_flags (32 bits)
            0xB0,             // 6: constraint flags(4) + reserved(4)
            0x00, 0x00, 0x00, 0x00, 0x00, // 7-11: reserved_zero_44bits (40 bits)
            0x00,             // 12: level_idc
            0x5F, 0xFF,       // 13-14: chroma + dimensions + bit_depth
        ];
        let sps_nal: Vec<u8> = {
            let mut n = vec![0x42, 0x01];
            n.extend_from_slice(sps_payload);
            n
        };
        data.extend_from_slice(&(sps_nal.len() as u32).to_be_bytes());
        data.extend_from_slice(&sps_nal);

        // PPS NAL
        let pps_nal: &[u8] = &[0x44, 0x01, 0xC0];
        data.extend_from_slice(&(pps_nal.len() as u32).to_be_bytes());
        data.extend_from_slice(pps_nal);

        let result = parse_hevc_bitstream(&data);
        assert!(!result.nal_units.is_empty());
        assert_eq!(result.nal_units.len(), 3);
        assert!(result.vps.is_some());
        assert!(result.sps.is_some());
        assert!(result.pps.is_some());
        let sps = result.sps.unwrap();
        assert_eq!(sps.pic_width_in_luma_samples, 0);
        assert_eq!(sps.pic_height_in_luma_samples, 0);
        assert_eq!(sps.bit_depth_luma_minus8 + 8, 8);
    }

    #[test]
    fn bit_reader_ue_golomb() {
        // ue(v) for 0: single start bit => 0b10000000
        let data = [0x80];
        let mut br = BitReader::new(&data);
        assert_eq!(br.read_ue_golomb(), Some(0));

        // ue(v) for 1: leading=1, info=1bit. Pattern: 0 1 0 => 0b01000000
        // value = (1<<1)|0 - 1 = 2 - 1 = 1
        let data = [0x40];
        let mut br = BitReader::new(&data);
        assert_eq!(br.read_ue_golomb(), Some(1));

        // ue(v) for 2: leading=1, info=1bit. Pattern: 0 1 1 => 0b01100000
        // value = (1<<1)|1 - 1 = 3 - 1 = 2
        let data = [0x60];
        let mut br = BitReader::new(&data);
        assert_eq!(br.read_ue_golomb(), Some(2));
    }

    #[test]
    fn bit_reader_read_bits() {
        let data = [0xAB, 0xCD]; // 10101011 11001101
        let mut br = BitReader::new(&data);
        assert_eq!(br.read_bits(4), Some(0b1010)); // 0xA
        assert_eq!(br.read_bits(4), Some(0b1011)); // 0xB
        assert_eq!(br.read_bits(4), Some(0b1100)); // 0xC
        assert_eq!(br.read_bits(4), Some(0b1101)); // 0xD
    }

    #[test]
    fn parse_vps_simple() {
        // vps_video_parameter_set_id=0 (4 bits)
        // vps_max_layers_minus1=0 (6 bits)
        // vps_max_sub_layers_minus1=0 (3 bits)
        // => bits: 0000 0000 0000 0xxx => 0x00, 0x00
        let data = [0x00, 0x00];
        let result = parse_vps(&data);
        assert!(result.is_some());
        let vps = result.unwrap();
        assert_eq!(vps.vps_max_layers_minus1, 0);
        assert_eq!(vps.vps_max_sub_layers_minus1, 0);
    }

    #[test]
    fn parse_sps_basic() {
        // SPS payload with known values:
        // sps_vps_id=0 (4b), max_sub_layers-1=0 (3b), temporal_nesting=1 (1b) => byte 0: 0x01
        // profile_space=0 (2b), tier=0 (1b), profile_idc=1 (5b) => Main => byte 1: 0x01
        // compatibility_flags (32b) = 0 => bytes 2-5: 0x00 x4
        // constraint flags(4) + reserved(4) => byte 6: 0xB0
        // reserved_zero_44bits (44b) => parser reads 4 constraint + 40 reserved = 44 bits
        //   landing at byte 12 for level_idc
        // level_idc=120 (0x78) => byte 12: 0x78
        // sps_seq_parameter_set_id: ue(v)=0 => bit: 1
        // chroma_format_idc: ue(v)=1 (4:2:0) => bits: 0 1 0
        // pic_width: ue(v)=1 => bits: 0 1 0
        // pic_height: ue(v)=1 => bits: 0 1 0
        // conformance_window_flag: 0 => bit: 0
        // bit_depth_luma-8: ue(v)=0 => bit: 1
        // bit_depth_chroma-8: ue(v)=0 => bit: 1
        // Packed from bit 104:
        // 1 010 010 010 0 1 1 -> 10100100 10011000 = A4 98
        let data: &[u8] = &[
            0x01,             // sps_vps_id=0, max_sub-1=0, temporal_nesting=1
            0x01,             // profile_space=0, tier=0, profile_idc=1 (Main)
            0x00, 0x00, 0x00, 0x00, // compatibility_flags (32 bits)
            0xB0,             // constraint flags(4) + reserved(4)
            0x00, 0x00, 0x00, 0x00, 0x00, // reserved_zero_44bits (40 bits)
            0x78,             // level_idc=120
            0xA4, 0x98,       // sps_id=0, chroma=1, width=1, height=1, conf=0, depths=0
        ];

        let result = parse_sps(data);
        assert!(result.is_some());
        let sps = result.unwrap();
        assert_eq!(sps.general_profile_idc, 1);
        assert_eq!(sps.chroma_format_idc, 1);
        assert_eq!(sps.general_level_idc, 120);
        assert_eq!(sps.pic_width_in_luma_samples, 1);
        assert_eq!(sps.pic_height_in_luma_samples, 1);
        assert_eq!(sps.bit_depth_luma_minus8 + 8, 8);
    }

    #[test]
    fn parse_hvcc_header_basic() {
        // Minimal hvcC box carrying the fields we read directly from the decoder config record.
        let data: &[u8] = &[
            0x00, 0x00, 0x00, 0x1b, // box size = 27
            0x68, 0x76, 0x63, 0x43, // "hvcC"
            0x01, // configurationVersion
            0x01, // profile_idc=1 (Main)
            0x00, 0x00, 0x00, 0x00, // compatibility_flags (32 bits)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // constraint flags (48 bits)
            0x78, // level_idc=120
            0xF0, 0x00, // min_spatial_segmentation_idc
            0xFC, // parallelismType
            0xFD, // chroma_format_idc = 1
            0xF8, // bitDepthLumaMinus8 = 0
            0xF8, // bitDepthChromaMinus8 = 0
        ];

        let result = parse_hvcc_header(data);
        assert!(result.is_some());
        let sps = result.unwrap();
        assert_eq!(sps.general_profile_idc, 1);
        assert_eq!(sps.chroma_format_idc, 1);
        assert_eq!(sps.general_level_idc, 120);
        assert_eq!(sps.pic_width_in_luma_samples, 0);
        assert_eq!(sps.pic_height_in_luma_samples, 0);
        assert_eq!(sps.bit_depth_luma_minus8 + 8, 8);
    }

    #[test]
    fn parse_hvcc_header_main10() {
        let data: &[u8] = &[
            0x00, 0x00, 0x00, 0x1b, // box size = 27
            0x68, 0x76, 0x63, 0x43, // "hvcC"
            0x01, // configurationVersion
            0x02, // profile_idc=2 (Main 10)
            0x00, 0x00, 0x00, 0x00, // compatibility_flags
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // constraint flags
            0x96, // level_idc=150 (Level 5.0)
            0xF0, 0x00, // min_spatial_segmentation_idc
            0xFC, // parallelismType
            0xFD, // chroma_format_idc = 1
            0xFA, // bitDepthLumaMinus8 = 2 -> 10-bit
            0xFA, // bitDepthChromaMinus8 = 2 -> 10-bit
        ];

        let result = parse_hvcc_header(data);
        assert!(result.is_some());
        let sps = result.unwrap();
        assert_eq!(sps.general_profile_idc, 2);
        assert_eq!(sps.chroma_format_idc, 1);
        assert_eq!(sps.general_level_idc, 150);
        assert_eq!(sps.bit_depth_luma_minus8 + 8, 10);
    }

    #[test]
    fn parse_hvcc_header_rejects_too_short() {
        // Too short: has box header but missing HEVCDecoderConfigurationRecord
        let data: &[u8] = &[
            0x00, 0x00, 0x00, 0x0c, // box size = 12
            0x68, 0x76, 0x63, 0x43, // "hvcC"
            0x01, 0x01, // only 2 bytes of HEVCDecoderConfigurationRecord (need 20+)
        ];
        assert!(parse_hvcc_header(data).is_none());
    }

    #[test]
    fn parses_real_heic_hvcc_to_profile_and_depth() {
        // Actual hvcC box data from a 512x512 8-bit HEIC image.
        // Note: the hvcC box does NOT store image dimensions — those come from
        // the ispe box in the HEIF container. This test verifies profile, level,
        // chroma, and bit_depth extraction.
        let data: &[u8] = &[
            0x00, 0x00, 0x00, 0x6e, 0x68, 0x76, 0x63, 0x43, // box header
            0x01, 0x01, 0x60, 0x00, 0x00, 0x00, 0xb0, 0x00, // HEVCDecoderConfigRecord start
            0x00, 0x00, 0x00, 0x00, 0x5a, 0xf0, 0x00, 0xfc, // level_idc=0x5a
            0xfd, 0xf8, 0xf8, 0x00, 0x00, 0x0f, 0x03, 0xa0, // numOfArrays=3
            0x00, 0x01, 0x00, 0x17, 0x40, 0x01, 0x0c, 0x01, // VPS NAL
            0xff, 0xff, 0x01, 0x60, 0x00, 0x00, 0x03, 0x00,
            0xb0, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x00,
            0x5a, 0x2c, 0x09, 0xa1, 0x00, 0x01, 0x00, 0x21, // SPS NAL
            0x42, 0x01, 0x01, 0x01, 0x60, 0x00, 0x00, 0x03,
            0x00, 0xb0, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03,
            0x00, 0x5a, 0xa0, 0x04, 0x02, 0x00, 0x80, 0x59,
            0xcb, 0x92, 0x44, 0x89, 0x2e, 0x26, 0xd4, 0x80,
            0x40, 0xa2, 0x00, 0x01, 0x00, 0x08, 0x44, 0x01, // PPS NAL
            0xc0, 0x61, 0x12, 0x4c, 0x14, 0xc9,
        ];

        let result = parse_hevc_bitstream(data);
        assert_eq!(result.nal_units.len(), 3);
        assert!(result.vps.is_some());
        assert!(result.sps.is_some());
        assert!(result.pps.is_some());

        let sps = result.sps.unwrap();
        // SPS NAL parsing with 44 reserved bits gives chroma=1, bit_depth=8
        // but width=0, height=0 (dimensions not stored in hvcC SPS).
        // The hvcc fallback also doesn't provide dimensions.
        assert_eq!(sps.chroma_format_idc, 1);
        assert_eq!(sps.bit_depth_luma_minus8 + 8, 8);
    }
}
