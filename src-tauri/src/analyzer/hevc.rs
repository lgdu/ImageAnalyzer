use crate::types::{
    HevcSliceHeader, HevcSyntax, NalUnit, PictureParameterSet,
    SequenceParameterSet, VideoParameterSet,
};

pub fn parse_hevc_bitstream(data: &[u8]) -> HevcSyntax {
    let mut nal_units = Vec::new();
    let mut vps_parsed: Option<VideoParameterSet> = None;
    let mut sps_parsed: Option<SequenceParameterSet> = None;
    let mut pps_parsed: Option<PictureParameterSet> = None;
    let mut slice_headers = Vec::new();

    let nal_list = extract_nal_units(data);

    for (nal_data, nal_offset) in nal_list {
        if nal_data.len() < 2 {
            continue;
        }

        let nal_header = nal_data[0];
        let nal_unit_type = (nal_header >> 1) & 0x3F;
        let nuh_layer_id =
            ((nal_header & 1) << 5) | ((nal_data[1] >> 5) & 0x1F);
        let nuh_temporal_id_plus1 = nal_data[1] & 0x07;
        let nuh_temporal_id = if nuh_temporal_id_plus1 > 0 {
            nuh_temporal_id_plus1 - 1
        } else {
            0
        };

        let nal_type_name = nal_type_to_name(nal_unit_type);

        nal_units.push(NalUnit {
            nal_type: nal_type_name.clone(),
            nuh_layer_id,
            nuh_temporal_id,
            size: nal_data.len(),
            offset: nal_offset,
        });

        let payload = &nal_data[2..];
        match nal_unit_type {
            32 => {
                vps_parsed = parse_vps(payload);
            }
            33 => {
                sps_parsed = parse_sps(payload);
            }
            34 => {
                pps_parsed = parse_pps(payload);
            }
            0..=31 => {
                if let (Some(ref pps), Some(ref sps)) =
                    (&pps_parsed, &sps_parsed)
                {
                    let sh = parse_slice_header(payload, pps, sps);
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

/// Extract NAL units from either Annex B (start codes) or length-prefixed format
fn extract_nal_units(data: &[u8]) -> Vec<(&[u8], u64)> {
    let mut result = Vec::new();

    // Try Annex B first (look for 0x000001 or 0x00000001)
    if has_start_codes(data) {
        let mut pos = 0;
        while pos + 3 <= data.len() {
            let sc_len = find_start_code_at(data, pos);
            if sc_len == 0 {
                pos += 1;
                continue;
            }
            let nal_start = pos + sc_len;
            if nal_start >= data.len() {
                break;
            }
            let nal_end = find_next_start_code(data, nal_start)
                .unwrap_or(data.len());
            result.push((&data[nal_start..nal_end], nal_start as u64));
            pos = nal_end;
        }
    } else {
        // Length-prefixed (4-byte big-endian length)
        let mut pos = 0;
        while pos + 4 <= data.len() {
            let nal_len = u32::from_be_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]) as usize;
            if nal_len == 0 || nal_len > data.len() - pos - 4 {
                break;
            }
            let nal_start = pos + 4;
            result.push((&data[nal_start..nal_start + nal_len], nal_start as u64));
            pos = nal_start + nal_len;
        }
    }

    result
}

fn has_start_codes(data: &[u8]) -> bool {
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            return true;
        }
    }
    false
}

fn find_start_code_at(data: &[u8], pos: usize) -> usize {
    if pos + 4 <= data.len()
        && data[pos] == 0
        && data[pos + 1] == 0
        && data[pos + 2] == 0
        && data[pos + 3] == 1
    {
        return 4;
    }
    if pos + 3 <= data.len()
        && data[pos] == 0
        && data[pos + 1] == 0
        && data[pos + 2] == 1
    {
        return 3;
    }
    0
}

fn find_next_start_code(data: &[u8], from: usize) -> Option<usize> {
    for i in from..data.len().saturating_sub(2) {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            return Some(i);
        }
        if i + 3 < data.len()
            && data[i] == 0
            && data[i + 1] == 0
            && data[i + 2] == 0
            && data[i + 3] == 1
        {
            return Some(i);
        }
    }
    None
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

// --- Exp-Golomb bit reader ---

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
    br.read_bits(4)?; // vps_video_parameter_set_id
    let vps_max_layers_minus1 = br.read_bits(6)? as u8;
    let vps_max_sub_layers_minus1 = br.read_bits(3)? as u8;

    Some(VideoParameterSet {
        vps_id: 0,
        max_layers: vps_max_layers_minus1 + 1,
        max_sub_layers: vps_max_sub_layers_minus1 + 1,
    })
}

// --- SPS parser ---

fn parse_sps(data: &[u8]) -> Option<SequenceParameterSet> {
    let mut br = BitReader::new(data);

    // sps_video_parameter_set_id (4 bits)
    br.read_bits(4)?;

    let _max_sub_layers_minus1 = br.read_bits(3)? as u8;
    let _temporal_id_nesting_flag = br.read_bool()?;

    // profile_tier_level (simplified)
    let _general_profile_space = br.read_bits(2)?;
    let _general_tier_flag = br.read_bool()?;
    let general_profile_idc = br.read_bits(5)?;
    let _general_profile_compatibility_flags = br.read_bits(32)?;
    let _general_progressive_source_flag = br.read_bool()?;
    let _general_interlaced_source_flag = br.read_bool()?;
    let _general_non_packed_constraint_flag = br.read_bool()?;
    let _general_frame_only_constraint_flag = br.read_bool()?;
    let general_level_idc = br.read_bits(8)?;

    let chroma_format_idc = br.read_ue_golomb()?;

    let chroma_format_str = match chroma_format_idc {
        0 => "Mono",
        1 => "4:2:0",
        2 => "4:2:2",
        3 => "4:4:4",
        _ => "Unknown",
    };

    let separate_colour_plane_flag = if chroma_format_idc == 3 {
        br.read_bool()?
    } else {
        false
    };

    let pic_width_in_luma_samples = br.read_ue_golomb()? as u32;
    let pic_height_in_luma_samples = br.read_ue_golomb()? as u32;

    let bit_depth_luma_minus8 = br.read_ue_golomb()? as u8;
    let bit_depth = 8 + bit_depth_luma_minus8;

    let profile_str = match general_profile_idc {
        1 => "Main",
        2 => "Main 10",
        3 => "Main Still Picture",
        _ => return Some(SequenceParameterSet {
            profile: format!("Profile={} (idc={})", chroma_format_str, general_profile_idc),
            level: format!("Level {}", general_level_idc),
            chroma_format: chroma_format_str.to_string(),
            pic_width: if separate_colour_plane_flag {
                pic_width_in_luma_samples
            } else {
                pic_width_in_luma_samples
            },
            pic_height: pic_height_in_luma_samples,
            bit_depth,
        }),
    };

    Some(SequenceParameterSet {
        profile: format!("{} {}", profile_str, chroma_format_str),
        level: format!("Level {}", general_level_idc),
        chroma_format: chroma_format_str.to_string(),
        pic_width: pic_width_in_luma_samples,
        pic_height: pic_height_in_luma_samples,
        bit_depth,
    })
}

// --- PPS parser ---

fn parse_pps(data: &[u8]) -> Option<PictureParameterSet> {
    let mut br = BitReader::new(data);
    let pps_pic_parameter_set_id = br.read_ue_golomb()? as u8;
    let pps_seq_parameter_set_id = br.read_ue_golomb()? as u8;

    Some(PictureParameterSet {
        pps_id: pps_pic_parameter_set_id,
        sps_id: pps_seq_parameter_set_id,
    })
}

// --- Slice header parser (simplified) ---

fn parse_slice_header(
    data: &[u8],
    pps: &PictureParameterSet,
    sps: &SequenceParameterSet,
) -> HevcSliceHeader {
    let mut br = BitReader::new(data);

    let first_slice_segment_in_pic_flag = br.read_bool().unwrap_or(true);
    let _no_output_of_prior_pics_flag = if first_slice_segment_in_pic_flag {
        br.read_bool().unwrap_or(false)
    } else {
        false
    };

    let slice_type = if first_slice_segment_in_pic_flag {
        br.read_ue_golomb().unwrap_or(0) as u8
    } else {
        br.read_ue_golomb().unwrap_or(2) as u8
    };

    let dependent_slice_segment_flag = if first_slice_segment_in_pic_flag {
        false
    } else {
        br.read_bool().unwrap_or(false)
    };

    let slice_segment_address = br.read_bits(16).unwrap_or(0);

    // Skip to pic_width/pic_height from SPS
    HevcSliceHeader {
        slice_type,
        first_slice_segment_in_pic_flag,
        dependent_slice_segment_flag,
        slice_segment_address: slice_segment_address as u32,
        pps_id: pps.pps_id,
        num_entry_point_offsets: None,
        offset_len_minus1: None,
        pic_width: sps.pic_width,
        pic_height: sps.pic_height,
        tile_enabled: false,
    }
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
    fn extracts_nal_units_from_annex_b() {
        // VPS NAL (type 32): header = (32 << 1) = 0x40, layer=0, temporal+1=1
        // Header bytes: [0x40, 0x01]
        let mut data = Vec::new();
        // Start code
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        // NAL header: type=32 (0x40 >> 1), layer=0, temporal+1=1
        data.push(0x40); // nal_unit_type = 32
        data.push(0x01); // nuh_layer_id = 0, nuh_temporal_id_plus1 = 1
        // NAL payload
        data.extend_from_slice(&[0x00, 0x00]); // minimal VPS payload
        // Next start code
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        // SPS NAL (type 33)
        data.push(0x42); // nal_unit_type = 33
        data.push(0x01);
        data.extend_from_slice(&[0x00]); // minimal payload

        let nals = extract_nal_units(&data);
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0].0[0], 0x40); // VPS header
        assert_eq!(nals[1].0[0], 0x42); // SPS header
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
    fn parses_hevc_bitstream() {
        // Use length-prefixed format. Must ensure NO 0x00 0x00 0x01 or 0x00 0x00 0x00 0x01
        // pattern exists anywhere in the data, otherwise has_start_codes() triggers Annex B parsing.
        let mut data = Vec::new();

        // --- VPS NAL (type 32) ---
        // NAL: [0x40, 0x01, payload...]
        // Payload: vps_id=0 (4b), max_layers-1=0 (6b), max_sub_layers-1=0 (3b) = 13 zero bits.
        // Need 2 bytes where first 13 bits are 0 but avoid 0x00 0x00 (which looks like start code).
        // 0x00 0x07 → bits: 00000000_00000111, first 13 = 0. ✓
        let vps_nal: &[u8] = &[0x40, 0x01, 0x00, 0x07];
        // Length = 4, encoded as 0x00 0x00 0x00 0x04 — has 00 00 00 but next byte is 0x04≠0x01,
        // and 00 00 00 0x04 doesn't match start code. However 00 00 00 at bytes 0-2 triggers
        // has_start_codes if followed by anything, since we scan for 00 00 01 at every position.
        // Actually bytes 0-2 = 00 00 00, position 0 check: 00 00 00 ≠ 00 00 01. No match.
        // Position 1: 00 00 04 ≠ 00 00 01. No match.
        // Good — but we need to check ALL positions across the whole buffer.
        // Let's just verify there's no 00 00 01 anywhere by using all non-zero length bytes.
        // We'll prefix each NAL with a single byte marker approach instead.
        // Actually, simplest: use the existing extracts_nal_units_from_length_prefixed test
        // data format but with VPS/SPS/PPS content.

        // Alternative: put non-zero bytes in the length high bytes.
        // NAL 1 length = 4: 0x00 0x00 0x00 0x04
        // Check for 00 00 01: at pos 0: [00,00,00]≠[00,00,01]. pos 1: [00,00,04]≠[00,00,01]. OK.
        data.extend_from_slice(&4u32.to_be_bytes()); // 00 00 00 04
        data.extend_from_slice(vps_nal); // 40 01 00 07

        // NAL 2 (SPS): length needs to encode the SAL size without 00 00 01 in its 4 bytes.
        // SPS NAL: 0x42 0x01 + payload
        // Payload: all zeros for fixed fields (10.5 bytes) + ue(v) fields
        // We'll use 0x01 0x01 for the first payload byte (non-zero start).
        // sps_vps_id=0(4), max_sub-1=0(3), temporal_nesting=1 → 0x01
        // profile_space=0(2), tier=0(1), profile_idc=1(5) → 0x01
        // compat(32)=0 → 00 00 00 00
        // progressive=1, interlaced=0, non_packed=1, frame_only=1 → 0xB0 (first 4 bits)
        // level_idc=0 → 0x00
        // chroma_format_idc=1: ue(v) → 0 1 0
        // pic_width=0: ue(v) → 1
        // pic_height=0: ue(v) → 1
        // bit_depth-8=0: ue(v) → 1
        // SPS payload bytes (after NAL header):
        // [0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0xB0, 0x00, 0x5F]
        // But 0x00 0x00 0x00 0x00 at positions 2-5 of payload could create 00 00 01 patterns.
        // Within the SPS NAL: [0x42, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0xB0, 0x00, 0x5F]
        // Check positions: pos 4: 00 00 00, pos 5: 00 00 00, pos 6: 00 00 B0, pos 7: 00 B0 00
        // pos 7: data[7]=0x00, data[8]=0xB0, data[9]=0x00 → 00 B0 00 ≠ 00 00 01. OK.
        // But pos 5: data[5]=0x00, data[6]=0x00, data[7]=0x00 → 00 00 00 ≠ 00 00 01.
        // pos 6: data[6]=0x00, data[7]=0x00, data[8]=0xB0 → 00 00 B0 ≠ 00 00 01.
        // No 00 00 01 pattern in SPS NAL itself. ✓
        //
        // Now check the full data for 00 00 01 across boundaries:
        // [00 00 00 04] [40 01 00 07] [SPS_len] [SPS_nal] [PPS_len] [PPS_nal]
        // Positions 0-2: 00 00 00 → no. Pos 1-3: 00 00 04 → no. Pos 2-4: 00 04 40 → no.
        // Pos 3-5: 04 40 01 → no. Pos 4-6: 40 01 00 → no. Pos 5-7: 01 00 07 → no.
        // We need SPS length and PPS length to also not create patterns.
        // SPS NAL length = 11 (0x00 0x00 0x00 0x0B). Check boundary: 00 07 00 00 00 0B
        // pos 6: data[6]=0x07, data[7]=0x00, data[8]=0x00 → 07 00 00 ≠ 00 00 01
        // pos 7: data[7]=0x00, data[8]=0x00, data[9]=0x00 → 00 00 00 ≠ 00 00 01
        // pos 8: data[8]=0x00, data[9]=0x00, data[10]=0x0B → 00 00 0B ≠ 00 00 01 ✓
        // PPS length also has 0x00 0x00 0x00 0x03. Check: after SPS, last byte of SPS...
        // This is getting complex. Let me just check programmatically.

        // Build SPS NAL: need at least 10 bytes of payload for all fixed + ue(v) fields.
        // 84 fixed bits = 10.5 bytes, plus 4 more bits for ue(v) fields = ~11 bytes.
        // Byte 0: sps_vps_id(4)=0, max_sub-1(3)=0, temporal_nesting(1)=1 → 0x01
        // Byte 1: profile_space(2)=0, tier(1)=0, profile_idc(5)=1 → 0x01
        // Bytes 2-5: compatibility flags (32 bits) = 0
        // Byte 6: progressive=1, interlaced=0, non_packed=1, frame_only=1, padding=0000 → 0xB0
        // Byte 7: level_idc(8)=0 → 0x00
        // Byte 8: chroma_format_idc=1: ue(v)=010, pic_width=0: ue(v)=1, pic_height=0: ue(v)=1
        //   bits: 0 1 0 1 1 = 5 bits, need 3 more padding bits set to 1 → 01011111 = 0x5F
        // Byte 9: bit_depth_luma-8=0: ue(v)=1, padding=1111111 → 11111111 = 0xFF
        let sps_payload: &[u8] = &[0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0xB0, 0x00, 0x5F, 0xFF];
        let sps_nal: Vec<u8> = {
            let mut n = vec![0x42, 0x01];
            n.extend_from_slice(sps_payload);
            n
        };
        data.extend_from_slice(&(sps_nal.len() as u32).to_be_bytes());
        data.extend_from_slice(&sps_nal);

        // Build PPS NAL
        let pps_nal: &[u8] = &[0x44, 0x01, 0xC0]; // type=34, header, 0xC0
        data.extend_from_slice(&(pps_nal.len() as u32).to_be_bytes());
        data.extend_from_slice(pps_nal);

        // Quick sanity: check no 00 00 01 pattern
        for i in 0..data.len().saturating_sub(2) {
            if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
                panic!("Found 00 00 01 at position {} — will trigger Annex B parsing", i);
            }
        }

        let result = parse_hevc_bitstream(&data);
        assert!(!result.nal_units.is_empty());
        assert_eq!(result.nal_units.len(), 3);
        assert!(result.vps.is_some());
        assert!(result.sps.is_some());
        assert!(result.pps.is_some());
        let sps = result.sps.unwrap();
        assert_eq!(sps.pic_width, 0);
        assert_eq!(sps.pic_height, 0);
        assert_eq!(sps.bit_depth, 8);
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
        assert_eq!(vps.max_layers, 1);
        assert_eq!(vps.max_sub_layers, 1);
    }
}
