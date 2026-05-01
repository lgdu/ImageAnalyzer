use crate::types::{
    Av1FrameHeader, Av1Syntax, Av1TileInfo, ColorConfig, Obu, QuantizerParams,
    SequenceHeader,
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

        let (header, payload) = match parse_obu_header(obu_data) {
            Some(h) => h,
            None => continue,
        };

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
            3 => {
                // Frame Header OBU
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

struct ObuHeader {
    obu_type: u8,
    temporal_id: u8,
    spatial_id: u8,
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
                let obu_end =
                    find_next_av1_start_code(data, obu_start).unwrap_or(data.len());
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
            let obu_len = u32::from_be_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]) as usize;
            if obu_len == 0 || obu_len > data.len() - pos - 4 {
                break;
            }
            let obu_start = pos + 4;
            result
                .push((&data[obu_start..obu_start + obu_len], obu_start as u64));
            pos = obu_start + obu_len;
        }
        // If no length-prefixed OBUs found, treat entire data as single OBU
        if result.is_empty() && !data.is_empty() {
            result.push((data, 0));
        }
    }

    result
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

    let obu_type = (byte0 >> 4) & 0x07;
    let extension_flag = (byte0 >> 3) & 1;

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

    Some((ObuHeader { obu_type, temporal_id, spatial_id }, payload))
}

fn obu_type_to_name(t: u8) -> String {
    match t {
        1 => "SEQUENCE_HEADER",
        2 => "TEMPORAL_DELIMITER",
        3 => "FRAME",
        4 => "TILE_GROUP",
        5 => "METADATA",
        6 => "REDUNDANT_FRAME_HEADER",
        7 => "FRAME_PADDING",
        _ => return format!("OBU_TYPE_{}", t),
    }
    .to_string()
}

// --- AV1 Bit Reader (LSB-first) ---

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
        let bit = (self.data[self.byte_pos] >> self.bit_pos) & 1;
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
        for i in 0..n {
            value |= (self.read_bit()? as u32) << i;
        }
        Some(value)
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
    let still_picture = br.read_bit()? != 0;
    let reduced_still_picture_header = br.read_bit()? != 0;

    let mut level = "Unknown".to_string();

    if reduced_still_picture_header {
        level = "Level 1".to_string();
    } else if still_picture {
        br.read_bits(5)?; // still_picture_level
    } else {
        let _operating_points_cnt = br.read_bits(5)?;
        for i in 0..=31 {
            let op_idx = br.read_bits(8)?;
            let decoder_model = br.read_bit()? != 0;
            if decoder_model {
                br.read_bits(32)?;
                br.read_bits(32)?;
                br.read_bits(8)?;
            }
            if i == 0 {
                level = format!("Level {}", (op_idx >> 4) + 1);
            }
        }
    }

    // Frame size (always present after level info)
    let frame_width = br.read_bits(16)? as u32 + 1;
    let frame_height = br.read_bits(16)? as u32 + 1;

    // Color config
    let mut chroma_format = "4:2:0".to_string();
    let bit_depth: u8;

    if seq_profile <= 2 {
        let high_bitdepth = br.read_bit()? != 0;

        let twelve_bit = if seq_profile == 2 {
            br.read_bit()? != 0
        } else {
            high_bitdepth
        };

        bit_depth = if seq_profile == 0 {
            8
        } else if twelve_bit {
            12
        } else {
            10
        };

        let mono_chrome = if seq_profile >= 1 && bit_depth >= 12 {
            br.read_bit()? != 0
        } else {
            false
        };

        let color_description_present = br.read_bit()? != 0;
        if color_description_present {
            br.read_bits(8)?; // matrix_coefficients
            br.read_bits(8)?; // color_primaries
            br.read_bits(8)?; // transfer_characteristics
        }

        if mono_chrome {
            br.read_bit()?; // color_range
            br.read_bits(2)?; // subsampling_x, subsampling_y
        } else if bit_depth == 12 {
            br.read_bit()?; // color_range
            let subsampling_x = br.read_bit()? != 0;
            let subsampling_y = br.read_bit()? != 0;
            br.read_bits(2)?; // chroma_sample_position
            chroma_format = if !subsampling_x && !subsampling_y {
                "4:4:4"
            } else if subsampling_x && !subsampling_y {
                "4:2:2"
            } else {
                "4:2:0"
            }
            .to_string();
        } else if bit_depth >= 10 || seq_profile >= 1 {
            br.read_bit()?; // color_range
            let subsampling_x = br.read_bit()? != 0;
            let subsampling_y = br.read_bit()? != 0;
            br.read_bits(2)?; // chroma_sample_position
            chroma_format = if !subsampling_x && !subsampling_y {
                "4:4:4"
            } else if subsampling_x && !subsampling_y {
                "4:2:2"
            } else {
                "4:2:0"
            }
            .to_string();
        } else {
            // Profile 0, 8-bit: always 4:2:0
            br.read_bits(4)?; // color_range, subsampling_x, subsampling_y, chroma_sample_position
        }
    } else {
        bit_depth = 8;
    }

    Some(SequenceHeader {
        profile: seq_profile,
        level,
        bit_depth,
        chroma_format: chroma_format.clone(),
        frame_width,
        frame_height,
        color_config: Some(ColorConfig {
            matrix_coefficients: chroma_format.clone(),
        }),
    })
}

// --- Frame Header parser (simplified) ---

fn parse_frame_header(
    data: &[u8],
    sh: &SequenceHeader,
) -> Option<(Av1FrameHeader, Option<Av1TileInfo>)> {
    let mut br = Av1BitReader::new(data);

    let _seen_frame_header = br.read_bit()?;

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

    // For KEY frames, use sequence header dimensions
    let frame_size = if frame_type == 0 {
        Some((sh.frame_width, sh.frame_height))
    } else {
        let frame_size_override = br.read_bit()? != 0;
        if frame_size_override {
            let w = br.read_bits(16)? as u32 + 1;
            let h = br.read_bits(16)? as u32 + 1;
            Some((w, h))
        } else {
            Some((sh.frame_width, sh.frame_height))
        }
    };

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
                frame_type: frame_type_name,
                show_frame,
                frame_size,
                order_hint,
                primary_ref_frame,
                quantizer_params,
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
                tile_heights.push(if i < remainder_h {
                    base_h + 1
                } else {
                    base_h
                });
            }
        }
    } else {
        let mi_cols = (sh.frame_width + 63) / 64;
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

        let mi_rows = (sh.frame_height + 63) / 64;
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
            frame_type: frame_type_name,
            show_frame,
            frame_size,
            order_hint,
            primary_ref_frame,
            quantizer_params,
        },
        tile_info,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obu_type_name_mapping() {
        assert_eq!(obu_type_to_name(1), "SEQUENCE_HEADER");
        assert_eq!(obu_type_to_name(2), "TEMPORAL_DELIMITER");
        assert_eq!(obu_type_to_name(3), "FRAME");
        assert_eq!(obu_type_to_name(4), "TILE_GROUP");
        assert_eq!(obu_type_to_name(7), "FRAME_PADDING");
        assert_eq!(obu_type_to_name(99), "OBU_TYPE_99");
    }

    #[test]
    fn extracts_obus_from_annex_b() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.push(0x18); // type=1, ext=1
        data.push(0x00); // temporal=0, spatial=0
        data.push(0xAA); // payload
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.push(0x38); // type=3, ext=1
        data.push(0x00);
        data.push(0xBB);

        let obus = extract_obus(&data);
        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].0[0], 0x18);
        assert_eq!(obus[1].0[0], 0x38);
    }

    #[test]
    fn extracts_obus_from_length_prefixed() {
        let mut data = Vec::new();
        data.extend_from_slice(&3u32.to_be_bytes());
        data.extend_from_slice(&[0x18, 0x00, 0xAA]);
        data.extend_from_slice(&3u32.to_be_bytes());
        data.extend_from_slice(&[0x38, 0x00, 0xBB]);

        let obus = extract_obus(&data);
        assert_eq!(obus.len(), 2);
        assert_eq!(obus[0].0, &[0x18, 0x00, 0xAA]);
        assert_eq!(obus[1].0, &[0x38, 0x00, 0xBB]);
    }

    #[test]
    fn parses_obu_header() {
        let data = [0x18, 0x00, 0xAA, 0xBB];
        let (header, payload) = parse_obu_header(&data).unwrap();
        assert_eq!(header.obu_type, 1);
        assert_eq!(header.temporal_id, 0);
        assert_eq!(header.spatial_id, 0);
        assert_eq!(payload, &[0xAA, 0xBB]);
    }

    #[test]
    fn rejects_forbidden_bit() {
        let data = [0x98, 0x00];
        assert!(parse_obu_header(&data).is_none());
    }

    #[test]
    fn av1_bit_reader_lsb_first() {
        let data = [0xAB, 0xCD];
        let mut br = Av1BitReader::new(&data);
        assert_eq!(br.read_bits(4), Some(0b1011));
        assert_eq!(br.read_bits(4), Some(0b1010));
        assert_eq!(br.read_bits(4), Some(0b1101));
        assert_eq!(br.read_bits(4), Some(0b1100));
    }

    #[test]
    fn parses_sequence_header_simple() {
        // Profile 0, reduced_still=1, 640x480, 4:2:0
        // LSB-first bit encoding: header(5 bits) then width-1(16), height-1(16), color config
        let payload: &[u8] = &[0xF8, 0x4F, 0xE0, 0x3B, 0x80, 0x01];

        let result = parse_sequence_header(payload);
        assert!(result.is_some());
        let sh = result.unwrap();
        assert_eq!(sh.profile, 0);
        assert_eq!(sh.frame_width, 640);
        assert_eq!(sh.frame_height, 480);
        assert_eq!(sh.bit_depth, 8);
        assert_eq!(sh.chroma_format, "4:2:0");
    }

    #[test]
    fn parses_av1_bitstream_annex_b() {
        let mut data = Vec::new();

        // --- Sequence Header OBU ---
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.extend_from_slice(&[0x18, 0x00]); // OBU header + ext
        // Payload: profile=0, still=1, reduced=1, 640x480, 4:2:0 (LSB-first encoded)
        data.extend_from_slice(&[0xF8, 0x4F, 0xE0, 0x3B, 0x80, 0x01]);

        // --- Temporal Delimiter OBU ---
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.extend_from_slice(&[0x28, 0x00]);

        // --- Frame OBU (KEY frame) ---
        // Frame payload:
        // seen_frame_header(1)=0, frame_type(2)=0(KEY), show_frame(1)=1
        // primary_ref_frame(3)=7, order_hint(8)=0
        // base_q_idx(8)=0, delta_q_present(1)=0
        // use_128(1)=0, tile_cols_log2(4)=0, tile_rows_log2(4)=0
        // uniform_spacing(1)=1
        // LSB first:
        // byte 0: bit0=0(seen), bit1-2=00(type), bit3=1(show), bit4-6=111(ref), bit7=0(order_lo)
        //   = 0|0|0|8|0|16|32|0 = 56 = 0x38
        // byte 1: order_hint bits 1-7 = 0, base_q_idx bit 0 = 0 → 0x00
        // byte 2: base_q_idx bits 1-7 = 0, delta_q_present = 0 → 0x00
        // byte 3: use_128=0, tile_cols=0(4bits), tile_rows=0(3bits) → 0x00
        // byte 4: tile_rows bit3=0, uniform_spacing=1 → 0x02
        data.extend_from_slice(&[0x00, 0x00, 0x01]);
        data.extend_from_slice(&[0x38, 0x00]); // OBU header + ext
        data.push(0x38);
        data.push(0x00);
        data.push(0x00);
        data.push(0x00);
        data.push(0x02);

        let result = parse_av1_bitstream(&data);
        assert!(!result.obus.is_empty());
        assert_eq!(result.obus.len(), 3);
        assert!(result.sequence_header.is_some());
        assert!(!result.frame_headers.is_empty());
        assert!(result.tile_info.is_some());

        let sh = result.sequence_header.unwrap();
        assert_eq!(sh.profile, 0);
        assert_eq!(sh.frame_width, 640);
        assert_eq!(sh.frame_height, 480);

        let fh = &result.frame_headers[0];
        assert_eq!(fh.frame_type, "KEY");
        assert!(fh.show_frame);

        let ti = result.tile_info.unwrap();
        assert_eq!(ti.num_tiles, 1);
        assert_eq!(ti.rows, 1);
        assert_eq!(ti.cols, 1);
    }
}
