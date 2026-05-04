use crate::types::{
    Av1FrameHeader, Av1Syntax, Av1TileInfo, ColorConfig, Obu, QuantizerParams, SequenceHeader,
};

/// Parse AV1 bitstream into OBU list and extract sequence header, frame headers, tile info.
pub fn parse_av1_bitstream(data: &[u8]) -> Av1Syntax {
    let mut obus = Vec::new();
    let mut sequence_header: Option<SequenceHeader> = None;
    let mut frame_headers = Vec::new();
    let mut tile_info: Option<Av1TileInfo> = None;

    let obu_list = extract_obus(data);

    for (obu_data, obu_offset) in obu_list {
        if obu_data.is_empty() {
            continue;
        }

        let (header, payload_with_size) = match parse_obu_header(obu_data) {
            Some(h) => h,
            None => continue,
        };
        let payload = payload_from_obu(obu_data, &header, payload_with_size).unwrap_or(payload_with_size);

        let obu_type_name = obu_type_to_name(header.obu_type);

        obus.push(Obu {
            obu_type: obu_type_name.clone(),
            obu_size: payload.len(),
            temporal_id: header.temporal_id,
            spatial_id: header.spatial_id,
            offset: obu_offset,
        });

        match header.obu_type {
            1 => {
                // Sequence Header OBU
                if let Some(sh) = parse_sequence_header(payload) {
                    sequence_header = Some(sh);
                }
            }
            3 | 6 => {
                // Frame Header OBU / Frame OBU
                if let Some(ref sh) = sequence_header {
                    if let Some((fh, ti)) = parse_frame_header(payload, sh) {
                        frame_headers.push(fh);
                        if tile_info.is_none() {
                            tile_info = ti;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Av1Syntax {
        obus,
        sequence_header,
        frame_headers,
        tile_info,
    }
}

pub fn parse_av1c_config(data: &[u8]) -> Av1Syntax {
    let config_obus = av1c_config_obus(data).unwrap_or(&[]);
    parse_av1_bitstream(config_obus)
}

pub fn parse_av1c_header(
    data: &[u8],
    frame_width: u32,
    frame_height: u32,
) -> Option<SequenceHeader> {
    let config = av1c_config_fields(data)?;
    Some(SequenceHeader {
        seq_profile: config.seq_profile,
        reduced_still_picture_header: false,
        seq_level_idx_0: config.seq_level_idx_0,
        max_frame_width_minus1: frame_width.saturating_sub(1),
        max_frame_height_minus1: frame_height.saturating_sub(1),
        use_128x128_superblock: false,
        enable_superres: false,
        enable_cdef: false,
        enable_restoration: false,
        color_config: ColorConfig {
            high_bitdepth: config.high_bitdepth,
            twelve_bit: config.twelve_bit,
            mono_chrome: config.monochrome,
            color_description_present_flag: false,
            color_primaries: 2,
            transfer_characteristics: 2,
            matrix_coefficients: 2,
            color_range: false,
            subsampling_x: config.chroma_subsampling_x,
            subsampling_y: config.chroma_subsampling_y,
            chroma_sample_position: Some(config.chroma_sample_position),
            separate_uv_delta_q: false,
        },
    })
}

pub fn merge_av1_syntax(mut base: Av1Syntax, extra: Av1Syntax) -> Av1Syntax {
    base.obus.extend(extra.obus);
    base.frame_headers.extend(extra.frame_headers);
    if base.sequence_header.is_none() {
        base.sequence_header = extra.sequence_header;
    }
    if base.tile_info.is_none() {
        base.tile_info = extra.tile_info;
    }
    base
}

struct ObuHeader {
    obu_type: u8,
    temporal_id: u8,
    spatial_id: u8,
    has_size_field: bool,
    extension_flag: bool,
}

struct Av1CodecConfig {
    seq_profile: u8,
    seq_level_idx_0: u8,
    high_bitdepth: bool,
    twelve_bit: bool,
    monochrome: bool,
    chroma_subsampling_x: bool,
    chroma_subsampling_y: bool,
    chroma_sample_position: u8,
}

fn extract_obus(data: &[u8]) -> Vec<(&[u8], u64)> {
    let mut result = Vec::new();

    // Try Annex B first (start codes: 0x00 0x00 0x01)
    if has_av1_start_code(data) {
        let mut pos = 0;
        while pos + 3 <= data.len() {
            if data[pos] == 0 && data[pos + 1] == 0 && data[pos + 2] == 1 {
                let obu_start = pos + 3;
                if obu_start >= data.len() {
                    break;
                }
                let obu_end = find_next_av1_start_code(data, obu_start).unwrap_or(data.len());
                result.push((&data[obu_start..obu_end], obu_start as u64));
                pos = obu_end;
            } else {
                pos += 1;
            }
        }
    } else {
        // Length-prefixed: 4-byte big-endian length
        let mut pos = 0;
        while pos + 4 <= data.len() {
            let obu_len =
                u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            if obu_len == 0 || obu_len > data.len() - pos - 4 {
                break;
            }
            let obu_start = pos + 4;
            result.push((&data[obu_start..obu_start + obu_len], obu_start as u64));
            pos = obu_start + obu_len;
        }
        // If no length-prefixed OBUs found, treat entire data as single OBU
        if result.is_empty() {
            if extract_obus_from_obu_stream(data, &mut result) {
                return result;
            }
        }
        if result.is_empty() && !data.is_empty() {
            result.push((data, 0));
        }
    }

    result
}

fn extract_obus_from_obu_stream<'a>(data: &'a [u8], result: &mut Vec<(&'a [u8], u64)>) -> bool {
    let mut pos = 0usize;
    let mut parsed_any = false;
    while pos < data.len() {
        let Some((obu, consumed)) = parse_obu_unit(&data[pos..]) else {
            break;
        };
        if consumed == 0 {
            break;
        }
        parsed_any = true;
        result.push((obu, pos as u64));
        pos += consumed;
    }
    parsed_any && pos == data.len()
}

fn has_av1_start_code(data: &[u8]) -> bool {
    for i in 0..data.len().saturating_sub(2) {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            return true;
        }
    }
    false
}

fn find_next_av1_start_code(data: &[u8], from: usize) -> Option<usize> {
    for i in from..data.len().saturating_sub(2) {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            return Some(i);
        }
    }
    None
}

fn parse_obu_header(data: &[u8]) -> Option<(ObuHeader, &[u8])> {
    if data.is_empty() {
        return None;
    }

    let byte0 = data[0];
    // forbidden_bit must be 0
    if (byte0 >> 7) & 1 != 0 {
        return None;
    }

    let obu_type = (byte0 >> 3) & 0x0F;
    let extension_flag = (byte0 >> 2) & 1;
    let has_size_field = ((byte0 >> 1) & 1) != 0;

    let mut temporal_id = 0;
    let mut spatial_id = 0;
    let mut payload_start = 1;

    if extension_flag != 0 {
        if data.len() < 2 {
            return None;
        }
        let byte1 = data[1];
        temporal_id = (byte1 >> 5) & 0x07;
        spatial_id = (byte1 >> 3) & 0x03;
        payload_start = 2;
    }

    // Payload is everything after headers (size field handled by outer extractor)
    let payload = &data[payload_start..];

    Some((
        ObuHeader {
            obu_type,
            temporal_id,
            spatial_id,
            has_size_field,
            extension_flag: extension_flag != 0,
        },
        payload,
    ))
}

fn parse_leb128_bytes(data: &[u8]) -> Option<(usize, usize)> {
    let mut value = 0usize;
    let mut shift = 0usize;
    for (i, byte) in data.iter().copied().enumerate() {
        value |= ((byte & 0x7F) as usize) << shift;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
        if shift >= usize::BITS as usize {
            return None;
        }
    }
    None
}

fn payload_from_obu<'a>(
    obu_data: &'a [u8],
    header: &ObuHeader,
    payload_with_size: &'a [u8],
) -> Option<&'a [u8]> {
    if !header.has_size_field {
        return Some(payload_with_size);
    }
    let size_len = if header.extension_flag { 2 } else { 1 };
    let (_, leb_len) = parse_leb128_bytes(&obu_data[size_len..])?;
    Some(&obu_data[size_len + leb_len..])
}

fn parse_obu_unit(data: &[u8]) -> Option<(&[u8], usize)> {
    let (header, payload) = parse_obu_header(data)?;
    if !header.has_size_field {
        return Some((data, data.len()));
    }
    let size_len = if header.extension_flag { 2 } else { 1 };
    let (payload_len, leb_len) = parse_leb128_bytes(&data[size_len..])?;
    let total_len = size_len + leb_len + payload_len;
    if total_len > data.len() {
        return None;
    }
    let _ = payload;
    Some((&data[..total_len], total_len))
}

fn av1c_config_obus(data: &[u8]) -> Option<&[u8]> {
    if data.len() >= 12 && &data[4..8] == b"av1C" {
        return Some(&data[12..]);
    }
    if data.len() >= 4 {
        return Some(&data[4..]);
    }
    None
}

fn av1c_config_fields(data: &[u8]) -> Option<Av1CodecConfig> {
    let config = if data.len() >= 12 && &data[4..8] == b"av1C" {
        &data[8..12]
    } else if data.len() >= 4 {
        &data[..4]
    } else {
        return None;
    };
    let byte0 = config[0];
    let marker = (byte0 >> 7) & 1;
    let version = byte0 & 0x7F;
    if marker != 1 || version != 1 {
        return None;
    }
    let byte1 = config[1];
    let byte2 = config[2];
    Some(Av1CodecConfig {
        seq_profile: (byte1 >> 5) & 0x07,
        seq_level_idx_0: byte1 & 0x1F,
        high_bitdepth: ((byte2 >> 6) & 1) != 0,
        twelve_bit: ((byte2 >> 5) & 1) != 0,
        monochrome: ((byte2 >> 4) & 1) != 0,
        chroma_subsampling_x: ((byte2 >> 3) & 1) != 0,
        chroma_subsampling_y: ((byte2 >> 2) & 1) != 0,
        chroma_sample_position: byte2 & 0x03,
    })
}

fn av1_level_name(idx: u8) -> String {
    const LEVELS: [&str; 24] = [
        "2.0", "2.1", "2.2", "2.3", "3.0", "3.1", "3.2", "3.3",
        "4.0", "4.1", "4.2", "4.3", "5.0", "5.1", "5.2", "5.3",
        "6.0", "6.1", "6.2", "6.3", "7.0", "7.1", "7.2", "7.3",
    ];
    LEVELS
        .get(idx as usize)
        .map(|v| format!("Level {v}"))
        .unwrap_or_else(|| format!("Level index {}", idx))
}

fn av1_chroma_format(monochrome: bool, subsampling_x: bool, subsampling_y: bool) -> String {
    if monochrome {
        "4:0:0".to_string()
    } else if !subsampling_x && !subsampling_y {
        "4:4:4".to_string()
    } else if subsampling_x && !subsampling_y {
        "4:2:2".to_string()
    } else {
        "4:2:0".to_string()
    }
}

fn obu_type_to_name(t: u8) -> String {
    match t {
        1 => "SEQUENCE_HEADER",
        2 => "TEMPORAL_DELIMITER",
        3 => "FRAME_HEADER",
        4 => "TILE_GROUP",
        5 => "METADATA",
        6 => "FRAME",
        7 => "REDUNDANT_FRAME_HEADER",
        8 => "TILE_LIST",
        15 => "PADDING",
        _ => return format!("OBU_TYPE_{}", t),
    }
    .to_string()
}

// --- AV1 Bit Reader (MSB-first) ---

struct Av1BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: usize, // 0-7 within current byte
}

impl<'a> Av1BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.byte_pos >= self.data.len() {
            return None;
        }
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Some(bit)
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

    fn skip_bits(&mut self, n: usize) -> Option<()> {
        for _ in 0..n {
            self.read_bit()?;
        }
        Some(())
    }

    fn read_leb128(&mut self) -> Option<u32> {
        let mut value = 0u32;
        for i in 0..32 {
            let byte = self.read_bits(8)? as u8;
            value |= ((byte & 0x7F) as u32) << (i * 7);
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }
}

// --- Sequence Header OBU parser ---

fn parse_sequence_header(data: &[u8]) -> Option<SequenceHeader> {
    let mut br = Av1BitReader::new(data);

    let seq_profile = br.read_bits(3)? as u8;
    let _still_picture = br.read_bit()? != 0;
    let reduced_still_picture_header = br.read_bit()? != 0;
    let seq_level_idx_0 = if reduced_still_picture_header {
        br.read_bits(5)? as u8
    } else {
        let timing_info_present_flag = br.read_bit()? != 0;
        if timing_info_present_flag {
            skip_timing_info(&mut br)?;
        }

        let decoder_model_info_present_flag = br.read_bit()? != 0;
        let mut buffer_delay_length_minus_1 = 0usize;
        if decoder_model_info_present_flag {
            buffer_delay_length_minus_1 = parse_decoder_model_info(&mut br)? as usize;
        }

        let initial_display_delay_present_flag = br.read_bit()? != 0;
        let operating_points_cnt_minus_1 = br.read_bits(5)? as u8;

        let mut first_level = None;
        for _ in 0..=operating_points_cnt_minus_1 {
            br.read_bits(12)?; // operating_point_idc
            let seq_level_idx = br.read_bits(5)? as u8;
            if first_level.is_none() {
                first_level = Some(seq_level_idx);
            }
            if seq_level_idx > 7 {
                br.read_bit()?; // seq_tier
            }
            if decoder_model_info_present_flag {
                let present = br.read_bit()? != 0;
                if present {
                    br.skip_bits(buffer_delay_length_minus_1 + 1)?;
                    br.skip_bits(buffer_delay_length_minus_1 + 1)?;
                    br.read_bit()?; // low_delay_mode_flag
                }
            }
            if initial_display_delay_present_flag {
                let present = br.read_bit()? != 0;
                if present {
                    br.read_bits(4)?; // initial_display_delay_minus_1
                }
            }
        }

        first_level?
    };

    let frame_width_bits_minus_1 = br.read_bits(4)? as usize;
    let frame_height_bits_minus_1 = br.read_bits(4)? as usize;
    let frame_width = br.read_bits(frame_width_bits_minus_1 + 1)? + 1;
    let frame_height = br.read_bits(frame_height_bits_minus_1 + 1)? + 1;

    if !reduced_still_picture_header {
        let frame_id_numbers_present_flag = br.read_bit()? != 0;
        if frame_id_numbers_present_flag {
            br.read_bits(4)?; // delta_frame_id_length_minus_2
            br.read_bits(3)?; // additional_frame_id_length_minus_1
        }
    }

    let use_128x128_superblock = br.read_bit()? != 0;
    br.read_bit()?; // enable_filter_intra
    br.read_bit()?; // enable_intra_edge_filter
    let (enable_superres, enable_cdef, enable_restoration) = if reduced_still_picture_header {
        (
            br.read_bit()? != 0,
            br.read_bit()? != 0,
            br.read_bit()? != 0,
        )
    } else {
        (false, false, false)
    };

    if !reduced_still_picture_header {
        br.read_bit()?; // enable_interintra_compound
        br.read_bit()?; // enable_masked_compound
        br.read_bit()?; // enable_warped_motion
        br.read_bit()?; // enable_dual_filter

        let enable_order_hint = br.read_bit()? != 0;
        if enable_order_hint {
            br.read_bit()?; // enable_jnt_comp
            br.read_bit()?; // enable_ref_frame_mvs
        }

        let seq_choose_screen_content_tools = br.read_bit()? != 0;
        let seq_force_screen_content_tools = if seq_choose_screen_content_tools {
            None
        } else {
            Some(br.read_bit()? != 0)
        };

        let force_integer_mv_enabled = if seq_choose_screen_content_tools {
            true
        } else {
            seq_force_screen_content_tools.unwrap_or(false)
        };

        if force_integer_mv_enabled {
            let seq_choose_integer_mv = br.read_bit()? != 0;
            if !seq_choose_integer_mv {
                br.read_bit()?; // seq_force_integer_mv
            }
        }

        if enable_order_hint {
            br.read_bits(3)?; // order_hint_bits_minus_1
        }

        let _ = br.read_bit()?; // enable_superres
        let _ = br.read_bit()?; // enable_cdef
        let _ = br.read_bit()?; // enable_restoration
    }

    let color = parse_color_config(&mut br, seq_profile)?;
    br.read_bit()?; // film_grain_params_present

    Some(SequenceHeader {
        seq_profile,
        reduced_still_picture_header,
        seq_level_idx_0,
        max_frame_width_minus1: frame_width - 1,
        max_frame_height_minus1: frame_height - 1,
        use_128x128_superblock,
        enable_superres,
        enable_cdef,
        enable_restoration,
        color_config: color,
    })
}

fn parse_color_config(br: &mut Av1BitReader<'_>, seq_profile: u8) -> Option<ColorConfig> {
    let high_bitdepth = br.read_bit()? != 0;
    let twelve_bit = if seq_profile == 2 && high_bitdepth {
        br.read_bit()? != 0
    } else {
        false
    };

    let mono_chrome = if seq_profile == 1 {
        false
    } else {
        br.read_bit()? != 0
    };

    let color_description_present_flag = br.read_bit()? != 0;
    let (color_primaries, transfer_characteristics, matrix_coefficients) =
        if color_description_present_flag {
            (
                br.read_bits(8)? as u8,
                br.read_bits(8)? as u8,
                br.read_bits(8)? as u8,
            )
        } else {
            (2, 2, 2)
        };

    if mono_chrome {
        let color_range = br.read_bit()? != 0;
        let separate_uv_delta_q = br.read_bit()? != 0;
        return Some(ColorConfig {
            high_bitdepth,
            twelve_bit,
            mono_chrome,
            color_description_present_flag,
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
            color_range,
            subsampling_x: true,
            subsampling_y: true,
            chroma_sample_position: None,
            separate_uv_delta_q,
        });
    }

    let (color_range, subsampling_x, subsampling_y) =
        if color_primaries == 1 && transfer_characteristics == 13 && matrix_coefficients == 0 {
            (true, false, false)
        } else {
            let color_range = br.read_bit()? != 0;
            if seq_profile == 0 {
                (color_range, true, true)
            } else if seq_profile == 1 {
                (color_range, false, false)
            } else if high_bitdepth && twelve_bit {
                let sx = br.read_bit()? != 0;
                let sy = if sx { br.read_bit()? != 0 } else { false };
                (color_range, sx, sy)
            } else {
                (color_range, true, false)
            }
        };

    let chroma_sample_position = if subsampling_x && subsampling_y {
        Some(br.read_bits(2)? as u8)
    } else {
        None
    };
    let separate_uv_delta_q = br.read_bit()? != 0;

    Some(ColorConfig {
        high_bitdepth,
        twelve_bit,
        mono_chrome,
        color_description_present_flag,
        color_primaries,
        transfer_characteristics,
        matrix_coefficients,
        color_range,
        subsampling_x,
        subsampling_y,
        chroma_sample_position,
        separate_uv_delta_q,
    })
}

fn skip_timing_info(br: &mut Av1BitReader<'_>) -> Option<()> {
    br.read_bits(32)?; // num_units_in_display_tick
    br.read_bits(32)?; // time_scale
    let equal_picture_interval = br.read_bit()? != 0;
    if equal_picture_interval {
        read_uvlc(br)?;
    }
    Some(())
}

fn parse_decoder_model_info(br: &mut Av1BitReader<'_>) -> Option<u8> {
    let buffer_delay_length_minus_1 = br.read_bits(5)? as u8;
    br.read_bits(32)?; // num_units_in_decoding_tick
    br.read_bits(5)?; // buffer_removal_time_length_minus_1
    br.read_bits(5)?; // frame_presentation_time_length_minus_1
    Some(buffer_delay_length_minus_1)
}

fn read_uvlc(br: &mut Av1BitReader<'_>) -> Option<u32> {
    let mut leading_zeros = 0usize;
    while br.read_bit()? == 0 {
        leading_zeros += 1;
        if leading_zeros > 31 {
            return Some(u32::MAX);
        }
    }

    if leading_zeros == 0 {
        return Some(0);
    }

    let value = br.read_bits(leading_zeros)?;
    Some(value + (1u32 << leading_zeros) - 1)
}

// --- Frame Header parser (simplified) ---

fn parse_frame_header(
    data: &[u8],
    sh: &SequenceHeader,
) -> Option<(Av1FrameHeader, Option<Av1TileInfo>)> {
    if sh.reduced_still_picture_header {
        return parse_reduced_still_picture_frame_header(data, sh);
    }

    let mut br = Av1BitReader::new(data);

    let _seen_frame_header = br.read_bit()?;
    let show_existing_frame = false;
    let frame_to_show_map_idx = None;

    let frame_type = br.read_bits(2)?;
    let show_frame = br.read_bit()? != 0;

    let frame_type_name = match frame_type {
        0 => "KEY",
        1 => "INTER",
        2 => "INTRA_ONLY",
        3 => "SWITCH",
        _ => "UNKNOWN",
    }
    .to_string();

    let error_resilient_mode = None;
    let disable_cdf_update = None;
    let allow_screen_content_tools = None;
    let force_integer_mv = None;
    let allow_intrabc = None;
    let refresh_frame_flags = if frame_type == 0 { Some(0xFF) } else { None };

    // For KEY frames, use sequence header dimensions
    let frame_size = if frame_type == 0 {
        Some((sh.max_frame_width_minus1 + 1, sh.max_frame_height_minus1 + 1))
    } else {
        let frame_size_override = br.read_bit()? != 0;
        let frame_size_override_flag = Some(frame_size_override);
        if frame_size_override {
            let w = br.read_bits(16)? as u32 + 1;
            let h = br.read_bits(16)? as u32 + 1;
            let frame_size = Some((w, h));
            return parse_frame_header_tail(
                &mut br,
                frame_type_name,
                show_existing_frame,
                frame_to_show_map_idx,
                show_frame,
                error_resilient_mode,
                disable_cdf_update,
                allow_screen_content_tools,
                force_integer_mv,
                frame_size_override_flag,
                None,
                allow_intrabc,
                refresh_frame_flags,
                frame_size,
                sh.max_frame_width_minus1 + 1,
                sh.max_frame_height_minus1 + 1,
            );
        } else {
            let frame_size = Some((sh.max_frame_width_minus1 + 1, sh.max_frame_height_minus1 + 1));
            return parse_frame_header_tail(
                &mut br,
                frame_type_name,
                show_existing_frame,
                frame_to_show_map_idx,
                show_frame,
                error_resilient_mode,
                disable_cdf_update,
                allow_screen_content_tools,
                force_integer_mv,
                frame_size_override_flag,
                None,
                allow_intrabc,
                refresh_frame_flags,
                frame_size,
                sh.max_frame_width_minus1 + 1,
                sh.max_frame_height_minus1 + 1,
            );
        }
    };

    parse_frame_header_tail(
        &mut br,
        frame_type_name,
        show_existing_frame,
        frame_to_show_map_idx,
        show_frame,
        error_resilient_mode,
        disable_cdf_update,
        allow_screen_content_tools,
        force_integer_mv,
        Some(false),
        None,
        allow_intrabc,
        refresh_frame_flags,
        frame_size,
        sh.max_frame_width_minus1 + 1,
        sh.max_frame_height_minus1 + 1,
    )
}

fn parse_frame_header_tail(
    br: &mut Av1BitReader<'_>,
    frame_type_name: String,
    show_existing_frame: bool,
    frame_to_show_map_idx: Option<u8>,
    show_frame: bool,
    error_resilient_mode: Option<bool>,
    disable_cdf_update: Option<bool>,
    allow_screen_content_tools: Option<bool>,
    force_integer_mv: Option<bool>,
    frame_size_override_flag: Option<bool>,
    render_and_frame_size_different: Option<bool>,
    allow_intrabc: Option<bool>,
    refresh_frame_flags: Option<u8>,
    frame_size: Option<(u32, u32)>,
    sequence_frame_width: u32,
    sequence_frame_height: u32,
) -> Option<(Av1FrameHeader, Option<Av1TileInfo>)> {

    // Simplified: skip intermediate fields and read what we can
    let primary_ref_frame = br.read_bits(3)? as u8;
    let order_hint = br.read_bits(8)? as u8;

    let base_q_idx = br.read_bits(8)? as u8;
    let delta_q_present = br.read_bit()? != 0;
    let delta_q_res = if delta_q_present {
        br.read_bits(2)? as u8
    } else {
        0
    };

    let quantizer_params = Some(QuantizerParams {
        base_q_idx,
        delta_q_present,
        delta_q_res,
    });

    // --- Tile info ---
    let use_128x128_superblock = br.read_bit()? != 0;
    let partition_bits = if use_128x128_superblock { 6 } else { 4 };
    let _ = partition_bits;

    let tile_cols_log2 = br.read_bits(4)?;
    let tile_rows_log2 = br.read_bits(4)?;

    if tile_cols_log2 > 6 || tile_rows_log2 > 6 {
        return Some((
            Av1FrameHeader {
                show_existing_frame,
                frame_to_show_map_idx,
                frame_type: frame_type_name,
                show_frame,
                error_resilient_mode,
                disable_cdf_update,
                allow_screen_content_tools,
                force_integer_mv,
                frame_size_override_flag,
                render_and_frame_size_different,
                allow_intrabc,
                refresh_frame_flags,
                frame_size,
                order_hint,
                primary_ref_frame,
                quantizer_params,
                delta_q_y_dc_coded: None,
                delta_q_u_dc_coded: None,
                delta_q_u_ac_coded: None,
                using_qmatrix: None,
                segmentation_enabled: None,
                reduced_tx_set: None,
                use_128x128_superblock,
                tile_cols_log2: tile_cols_log2 as u8,
                tile_rows_log2: tile_rows_log2 as u8,
                uniform_tile_spacing_flag: false,
            },
            None,
        ));
    }

    let tile_cols = 1u32 << tile_cols_log2;
    let tile_rows = 1u32 << tile_rows_log2;
    let num_tiles = tile_cols * tile_rows;

    let uniform_spacing = br.read_bit()? != 0;
    let mut tile_widths = Vec::new();
    let mut tile_heights = Vec::new();

    if uniform_spacing {
        if let Some((w, h)) = frame_size {
            let base_w = w / tile_cols;
            let remainder_w = w % tile_cols;
            for i in 0..tile_cols {
                tile_widths.push(if i < remainder_w { base_w + 1 } else { base_w });
            }
            let base_h = h / tile_rows;
            let remainder_h = h % tile_rows;
            for i in 0..tile_rows {
                tile_heights.push(if i < remainder_h { base_h + 1 } else { base_h });
            }
        }
    } else {
        let mi_cols = (sequence_frame_width + 63) / 64;
        let remaining_cols = mi_cols << 6;
        let mut col_start = 0u32;
        for i in 0..tile_cols {
            if i == tile_cols - 1 {
                tile_widths.push(remaining_cols - col_start);
            } else {
                let size = br.read_leb128()? + 1;
                tile_widths.push(size);
                col_start += size;
            }
        }

        let mi_rows = (sequence_frame_height + 63) / 64;
        let remaining_rows = mi_rows << 6;
        let mut row_start = 0u32;
        for i in 0..tile_rows {
            if i == tile_rows - 1 {
                tile_heights.push(remaining_rows - row_start);
            } else {
                let size = br.read_leb128()? + 1;
                tile_heights.push(size);
                row_start += size;
            }
        }
    }

    let tile_info = Some(Av1TileInfo {
        num_tiles,
        rows: tile_rows,
        cols: tile_cols,
        tile_width: tile_widths,
        tile_height: tile_heights,
        context_update_tile_id: None,
    });

    Some((
        Av1FrameHeader {
            show_existing_frame,
            frame_to_show_map_idx,
            frame_type: frame_type_name,
            show_frame,
            error_resilient_mode,
            disable_cdf_update,
            allow_screen_content_tools,
            force_integer_mv,
            frame_size_override_flag,
            render_and_frame_size_different,
            allow_intrabc,
            refresh_frame_flags,
            frame_size,
            order_hint,
            primary_ref_frame,
            quantizer_params,
            delta_q_y_dc_coded: None,
            delta_q_u_dc_coded: None,
            delta_q_u_ac_coded: None,
            using_qmatrix: None,
            segmentation_enabled: None,
            reduced_tx_set: None,
            use_128x128_superblock,
            tile_cols_log2: tile_cols_log2 as u8,
            tile_rows_log2: tile_rows_log2 as u8,
            uniform_tile_spacing_flag: uniform_spacing,
        },
        tile_info,
    ))
}

fn parse_reduced_still_picture_frame_header(
    data: &[u8],
    sh: &SequenceHeader,
) -> Option<(Av1FrameHeader, Option<Av1TileInfo>)> {
    let mut br = Av1BitReader::new(data);

    let disable_cdf_update = br.read_bit()? != 0;
    let allow_screen_content_tools = br.read_bit()? != 0;
    let force_integer_mv = if allow_screen_content_tools {
        br.read_bit()? != 0
    } else {
        false
    };
    let render_and_frame_size_different = br.read_bit()? != 0;
    let allow_intrabc = br.read_bit()? != 0;
    let uniform_tile_spacing_flag = br.read_bit()? != 0;
    let tile_cols_log2 = br.read_bit()? as u8;
    let tile_rows_log2 = br.read_bit()? as u8;
    let base_q_idx = br.read_bits(8)? as u8;
    let delta_q_y_dc_coded = br.read_bit()? != 0;
    let delta_q_u_dc_coded = br.read_bit()? != 0;
    let delta_q_u_ac_coded = br.read_bit()? != 0;
    let using_qmatrix = br.read_bit()? != 0;
    let segmentation_enabled = br.read_bit()? != 0;
    let reduced_tx_set = br.read_bit()? != 0;

    let frame_size = Some((sh.max_frame_width_minus1 + 1, sh.max_frame_height_minus1 + 1));
    let tile_cols = 1u32 << tile_cols_log2;
    let tile_rows = 1u32 << tile_rows_log2;
    let num_tiles = tile_cols * tile_rows;
    let mut tile_width = Vec::new();
    let mut tile_height = Vec::new();
    if let Some((w, h)) = frame_size {
        let base_w = w / tile_cols.max(1);
        let remainder_w = w % tile_cols.max(1);
        for i in 0..tile_cols {
            tile_width.push(if i < remainder_w { base_w + 1 } else { base_w });
        }
        let base_h = h / tile_rows.max(1);
        let remainder_h = h % tile_rows.max(1);
        for i in 0..tile_rows {
            tile_height.push(if i < remainder_h { base_h + 1 } else { base_h });
        }
    }

    let tile_info = Some(Av1TileInfo {
        num_tiles,
        rows: tile_rows,
        cols: tile_cols,
        tile_width,
        tile_height,
        context_update_tile_id: None,
    });

    Some((
        Av1FrameHeader {
            show_existing_frame: false,
            frame_to_show_map_idx: None,
            frame_type: "KEY".to_string(),
            show_frame: true,
            error_resilient_mode: Some(true),
            disable_cdf_update: Some(disable_cdf_update),
            allow_screen_content_tools: Some(allow_screen_content_tools),
            force_integer_mv: Some(force_integer_mv),
            frame_size_override_flag: Some(false),
            render_and_frame_size_different: Some(render_and_frame_size_different),
            allow_intrabc: Some(allow_intrabc),
            refresh_frame_flags: Some(0xFF),
            frame_size,
            order_hint: 0,
            primary_ref_frame: 7,
            quantizer_params: Some(QuantizerParams {
                base_q_idx,
                delta_q_present: delta_q_y_dc_coded || delta_q_u_dc_coded || delta_q_u_ac_coded,
                delta_q_res: 0,
            }),
            delta_q_y_dc_coded: Some(delta_q_y_dc_coded),
            delta_q_u_dc_coded: Some(delta_q_u_dc_coded),
            delta_q_u_ac_coded: Some(delta_q_u_ac_coded),
            using_qmatrix: Some(using_qmatrix),
            segmentation_enabled: Some(segmentation_enabled),
            reduced_tx_set: Some(reduced_tx_set),
            use_128x128_superblock: sh.use_128x128_superblock,
            tile_cols_log2,
            tile_rows_log2,
            uniform_tile_spacing_flag,
        },
        tile_info,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TestBitWriter {
        bytes: Vec<u8>,
        bit_pos: usize,
    }

    impl TestBitWriter {
        fn new() -> Self {
            Self {
                bytes: vec![0],
                bit_pos: 0,
            }
        }

        fn write_bit(&mut self, bit: bool) {
            if bit {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 1 << (7 - self.bit_pos);
            }
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.bytes.push(0);
            }
        }

        fn write_bits(&mut self, value: u32, bits: usize) {
            for i in (0..bits).rev() {
                self.write_bit(((value >> i) & 1) != 0);
            }
        }

        fn finish(mut self) -> Vec<u8> {
            if self.bit_pos == 0 {
                let _ = self.bytes.pop();
            }
            self.bytes
        }
    }

    fn sample_reduced_still_picture_sequence_header() -> Vec<u8> {
        let mut bw = TestBitWriter::new();
        bw.write_bits(0, 3); // seq_profile
        bw.write_bit(true); // still_picture
        bw.write_bit(true); // reduced_still_picture_header
        bw.write_bits(1, 5); // seq_level_idx_0
        bw.write_bits(9, 4); // frame_width_bits_minus_1
        bw.write_bits(8, 4); // frame_height_bits_minus_1
        bw.write_bits(639, 10); // max_frame_width_minus_1
        bw.write_bits(479, 9); // max_frame_height_minus_1
        bw.write_bit(false); // use_128x128_superblock
        bw.write_bit(false); // enable_filter_intra
        bw.write_bit(true); // enable_intra_edge_filter
        bw.write_bit(false); // enable_superres
        bw.write_bit(false); // enable_cdef
        bw.write_bit(false); // enable_restoration
        bw.write_bit(false); // high_bitdepth
        bw.write_bit(false); // mono_chrome
        bw.write_bit(false); // color_description_present_flag
        bw.write_bit(false); // color_range
        bw.write_bit(false); // separate_uv_delta_q
        bw.write_bit(false); // film_grain_params_present
        bw.finish()
    }

    fn sample_key_frame_payload() -> Vec<u8> {
        let mut bw = TestBitWriter::new();
        bw.write_bit(false); // disable_cdf_update
        bw.write_bit(false); // allow_screen_content_tools
        bw.write_bit(false); // render_and_frame_size_different
        bw.write_bit(false); // allow_intrabc
        bw.write_bit(true); // uniform_tile_spacing_flag
        bw.write_bit(false); // tile_cols_log2
        bw.write_bit(false); // tile_rows_log2
        bw.write_bits(0, 8); // base_q_idx
        bw.write_bit(false); // delta_q_y_dc.delta_coded
        bw.write_bit(false); // delta_q_u_dc.delta_coded
        bw.write_bit(false); // delta_q_u_ac.delta_coded
        bw.write_bit(false); // using_qmatrix
        bw.write_bit(false); // segmentation_enabled
        bw.write_bit(false); // reduced_tx_set
        bw.finish()
    }

    #[test]
    fn obu_type_name_mapping() {
        assert_eq!(obu_type_to_name(1), "SEQUENCE_HEADER");
        assert_eq!(obu_type_to_name(2), "TEMPORAL_DELIMITER");
        assert_eq!(obu_type_to_name(3), "FRAME_HEADER");
        assert_eq!(obu_type_to_name(4), "TILE_GROUP");
        assert_eq!(obu_type_to_name(6), "FRAME");
        assert_eq!(obu_type_to_name(15), "PADDING");
        assert_eq!(obu_type_to_name(99), "OBU_TYPE_99");
    }

    #[test]
    fn extracts_obus_from_annex_b() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.push(0x0A); // sequence_header, has_size_field=1
        data.push(0x01); // payload size
        data.push(0xAA); // payload
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.push(0x1A); // frame_header, has_size_field=1
        data.push(0x01);
        data.push(0xBB);

        let obus = extract_obus(&data);
        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].0[0], 0x0A);
        assert_eq!(obus[1].0[0], 0x1A);
    }

    #[test]
    fn extracts_obus_from_length_prefixed() {
        let mut data = Vec::new();
        data.extend_from_slice(&3u32.to_be_bytes());
        data.extend_from_slice(&[0x0A, 0x00, 0xAA]);
        data.extend_from_slice(&3u32.to_be_bytes());
        data.extend_from_slice(&[0x1A, 0x00, 0xBB]);

        let obus = extract_obus(&data);
        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].0, &[0x0A, 0x00, 0xAA]);
        assert_eq!(obus[1].0, &[0x1A, 0x00, 0xBB]);
    }

    #[test]
    fn parses_obu_header() {
        let data = [0x0A, 0x02, 0xAA, 0xBB];
        let (header, payload) = parse_obu_header(&data).unwrap();
        assert_eq!(header.obu_type, 1);
        assert_eq!(header.temporal_id, 0);
        assert_eq!(header.spatial_id, 0);
        assert!(header.has_size_field);
        assert_eq!(payload, &[0x02, 0xAA, 0xBB]);
    }

    #[test]
    fn rejects_forbidden_bit() {
        let data = [0x98, 0x00];
        assert!(parse_obu_header(&data).is_none());
    }

    #[test]
    fn av1_bit_reader_msb_first() {
        let data = [0xAB, 0xCD];
        let mut br = Av1BitReader::new(&data);
        assert_eq!(br.read_bits(4), Some(0b1010));
        assert_eq!(br.read_bits(4), Some(0b1011));
        assert_eq!(br.read_bits(4), Some(0b1100));
        assert_eq!(br.read_bits(4), Some(0b1101));
    }

    #[test]
    fn parses_sequence_header_simple() {
        let payload = sample_reduced_still_picture_sequence_header();

        let result = parse_sequence_header(&payload);
        assert!(result.is_some());
        let sh = result.unwrap();
        assert_eq!(sh.seq_profile, 0);
        assert!(sh.reduced_still_picture_header);
        assert_eq!(sh.seq_level_idx_0, 1);
        assert_eq!(sh.max_frame_width_minus1 + 1, 640);
        assert_eq!(sh.max_frame_height_minus1 + 1, 480);
        assert!(!sh.use_128x128_superblock);
        assert!(!sh.enable_superres);
        assert!(!sh.enable_cdef);
        assert!(!sh.enable_restoration);
        assert!(!sh.color_config.high_bitdepth);
        assert!(sh.color_config.subsampling_x);
        assert!(sh.color_config.subsampling_y);
    }

    #[test]
    fn parses_av1_bitstream_annex_b() {
        let mut data = Vec::new();

        // --- Sequence Header OBU ---
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        let sequence_header_payload = sample_reduced_still_picture_sequence_header();
        data.extend_from_slice(&[0x0A, sequence_header_payload.len() as u8]); // OBU header + payload size
        data.extend_from_slice(&sequence_header_payload);

        // --- Temporal Delimiter OBU ---
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.extend_from_slice(&[0x12, 0x00]);

        // --- Frame OBU (KEY frame) ---
        let frame_payload = sample_key_frame_payload();
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.extend_from_slice(&[0x32, frame_payload.len() as u8]); // OBU header + payload size
        data.extend_from_slice(&frame_payload);

        let result = parse_av1_bitstream(&data);
        assert!(!result.obus.is_empty());
        assert_eq!(result.obus.len(), 3);
        assert!(result.sequence_header.is_some());
        assert!(!result.frame_headers.is_empty());
        assert!(result.tile_info.is_some());

        let sh = result.sequence_header.unwrap();
        assert_eq!(sh.seq_profile, 0);
        assert_eq!(sh.max_frame_width_minus1 + 1, 640);
        assert_eq!(sh.max_frame_height_minus1 + 1, 480);

        let fh = &result.frame_headers[0];
        assert_eq!(fh.frame_type, "KEY");
        assert!(fh.show_frame);
        assert_eq!(fh.disable_cdf_update, Some(false));
        assert_eq!(fh.allow_screen_content_tools, Some(false));
        assert_eq!(fh.render_and_frame_size_different, Some(false));
        assert_eq!(fh.allow_intrabc, Some(false));
        assert_eq!(fh.uniform_tile_spacing_flag, true);
        assert_eq!(fh.tile_cols_log2, 0);
        assert_eq!(fh.tile_rows_log2, 0);

        let ti = result.tile_info.unwrap();
        assert_eq!(ti.num_tiles, 1);
        assert_eq!(ti.rows, 1);
        assert_eq!(ti.cols, 1);
    }

    #[test]
    fn parses_real_avif_sample_payload() {
        let path =
            "/Users/liguodu/projects/avif-sample-images/red-at-12-oclock-with-color-profile-8bpc.avif";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let bytes = fs::read(path).unwrap();
        let payload = &bytes[0x32c..];
        let result = parse_av1_bitstream(payload);
        assert!(
            !result.obus.is_empty(),
            "Expected at least one OBU from real AVIF payload"
        );
        let sequence_header = result
            .sequence_header
            .as_ref()
            .expect("Expected sequence header from real AVIF payload");
        assert_eq!(sequence_header.seq_profile, 0);
        assert!(sequence_header.use_128x128_superblock);
        assert!(!sequence_header.enable_superres);
        assert!(!sequence_header.enable_cdef);
        assert!(!sequence_header.enable_restoration);
        assert!(!sequence_header.color_config.high_bitdepth);
        assert!(sequence_header.color_config.subsampling_x);
        assert!(sequence_header.color_config.subsampling_y);
        assert_eq!(sequence_header.color_config.color_primaries, 1);
        assert_eq!(sequence_header.color_config.transfer_characteristics, 13);
        assert_eq!(sequence_header.color_config.matrix_coefficients, 1);
        assert_eq!(sequence_header.max_frame_width_minus1 + 1, 800);
        assert_eq!(sequence_header.max_frame_height_minus1 + 1, 800);
        let frame_header = result
            .frame_headers
            .first()
            .expect("Expected frame header from real AVIF payload");
        assert_eq!(frame_header.frame_type, "KEY");
        assert!(frame_header.show_frame);
        assert_eq!(frame_header.error_resilient_mode, Some(true));
        assert_eq!(frame_header.disable_cdf_update, Some(false));
        assert_eq!(frame_header.allow_screen_content_tools, Some(true));
        assert_eq!(frame_header.force_integer_mv, Some(false));
        assert_eq!(frame_header.render_and_frame_size_different, Some(false));
        assert_eq!(frame_header.allow_intrabc, Some(false));
        assert_eq!(frame_header.refresh_frame_flags, Some(0xFF));
        assert_eq!(frame_header.quantizer_params.as_ref().map(|q| q.base_q_idx), Some(0));
        assert_eq!(frame_header.delta_q_y_dc_coded, Some(false));
        assert_eq!(frame_header.delta_q_u_dc_coded, Some(false));
        assert_eq!(frame_header.delta_q_u_ac_coded, Some(false));
        assert_eq!(frame_header.using_qmatrix, Some(false));
        assert_eq!(frame_header.segmentation_enabled, Some(false));
        assert_eq!(frame_header.reduced_tx_set, Some(false));
        assert!(frame_header.use_128x128_superblock);
        assert_eq!(frame_header.tile_cols_log2, 0);
        assert_eq!(frame_header.tile_rows_log2, 0);
        assert!(frame_header.uniform_tile_spacing_flag);
    }
}
