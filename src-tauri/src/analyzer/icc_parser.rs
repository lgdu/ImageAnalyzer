use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use crate::types::{IccInfo, IccTag, LutInfo};

/// Parse ICC profile raw data into structured IccInfo
pub fn parse_icc(data: &[u8]) -> Option<IccInfo> {
    if data.len() < 128 {
        return None;
    }

    let mut cursor = Cursor::new(data);
    let size = cursor.read_u32::<BigEndian>().ok()?;

    let cmm_type = read_tag4(data, 12)?;
    let version = decode_version(data);
    let profile_class = read_tag4(data, 20)?;
    let color_space = read_tag4(data, 28)?;
    let pcs = read_tag4(data, 32)?;
    let platform = read_tag4(data, 48);

    cursor.set_position(64);
    let intent_num = cursor.read_u32::<BigEndian>().ok()?;
    let rendering_intent = match intent_num {
        0 => "Perceptual",
        1 => "Relative Colorimetric",
        2 => "Saturation",
        3 => "Absolute Colorimetric",
        _ => "Unknown",
    }
    .to_string();

    let illuminant_x = read_s15fixed16(data, 68);
    let illuminant_y = read_s15fixed16(data, 72);
    let illuminant_z = read_s15fixed16(data, 76);
    let creator = read_tag4(data, 80);

    cursor.set_position(128);
    let tag_count = cursor.read_u32::<BigEndian>().ok()?;
    let mut tags = Vec::new();
    let mut luts = Vec::new();
    let mut description: Option<String> = None;
    let mut transfer_function: Option<String> = None;
    let mut red_trc: Option<String> = None;
    let mut green_trc: Option<String> = None;
    let mut blue_trc: Option<String> = None;

    for _ in 0..tag_count {
        let tag_sig = read_tag4(data, cursor.position() as usize)?;
        let tag_offset = cursor.read_u32::<BigEndian>().ok()? as usize;
        let tag_size = cursor.read_u32::<BigEndian>().ok()? as usize;

        if tag_offset + 4 > data.len() {
            continue;
        }
        let tag_type = read_tag4(data, tag_offset).unwrap_or_default();

        let decoded_value = decode_tag(data, tag_offset, tag_size, &tag_type);

        if tag_sig == "desc" {
            description = extract_desc(data, tag_offset, tag_size);
        }

        if tag_sig == "rTRC" {
            red_trc = describe_trc(data, tag_offset, tag_size);
            if transfer_function.is_none() {
                transfer_function = red_trc.clone();
            }
        }
        if tag_sig == "gTRC" {
            green_trc = describe_trc(data, tag_offset, tag_size);
        }
        if tag_sig == "bTRC" {
            blue_trc = describe_trc(data, tag_offset, tag_size);
        }

        if tag_sig.starts_with("A2B") || tag_sig.starts_with("B2A") {
            let input_channels = if tag_offset + 8 <= data.len() {
                data[tag_offset + 4]
            } else {
                0
            };
            let output_channels = if tag_offset + 9 <= data.len() {
                data[tag_offset + 5]
            } else {
                0
            };
            let clut_points = if tag_offset + 6 <= data.len() {
                Some(data[tag_offset + 6])
            } else {
                None
            };
            luts.push(LutInfo {
                name: tag_sig.clone(),
                clut_points,
                input_channels,
                output_channels,
            });
        }

        tags.push(IccTag {
            name: tag_sig,
            offset: tag_offset as u32,
            size: tag_size as u32,
            tag_type,
            decoded_value,
        });
    }

    Some(IccInfo {
        size,
        cmm_type,
        version,
        profile_class,
        color_space,
        pcs,
        platform,
        rendering_intent,
        illuminant: (illuminant_x, illuminant_y, illuminant_z),
        creator,
        description,
        transfer_function,
        red_trc,
        green_trc,
        blue_trc,
        primaries: None,
        matrix: None,
        luts,
        tag_count,
        tags,
        raw_base64: None,
    })
}

fn decode_version(data: &[u8]) -> String {
    format!(
        "{}.{}.{}",
        data[44],
        (data[45] >> 4) & 0x0F,
        data[45] & 0x0F
    )
}

fn read_tag4(data: &[u8], offset: usize) -> Option<String> {
    if offset + 4 > data.len() {
        return None;
    }
    String::from_utf8(data[offset..offset + 4].to_vec()).ok()
}

fn read_s15fixed16(data: &[u8], offset: usize) -> f64 {
    if offset + 4 > data.len() {
        return 0.0;
    }
    let val = ((data[offset] as i32) << 24
        | (data[offset + 1] as i32) << 16
        | (data[offset + 2] as i32) << 8
        | data[offset + 3] as i32) as f64
        / 65536.0;
    val
}

fn extract_desc(data: &[u8], offset: usize, _size: usize) -> Option<String> {
    if offset + 12 > data.len() {
        return None;
    }
    let count = u32::from_be_bytes([
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]) as usize;
    if count == 0 {
        return Some(String::new());
    }
    let end = std::cmp::min(offset + 8 + count - 1, data.len());
    String::from_utf8(data[offset + 8..end].to_vec()).ok()
}

fn describe_trc(data: &[u8], offset: usize, _size: usize) -> Option<String> {
    if offset + 4 > data.len() {
        return None;
    }
    let sig = read_tag4(data, offset)?;
    match sig.as_str() {
        "curv" => {
            if offset + 8 <= data.len() {
                let count = u32::from_be_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);
                if count == 0 {
                    Some("Linear".to_string())
                } else if count == 1 {
                    if offset + 10 <= data.len() {
                        let gamma =
                            u16::from_be_bytes([data[offset + 8], data[offset + 9]]);
                        Some(format!("Gamma={:.2}", gamma as f64 / 256.0))
                    } else {
                        None
                    }
                } else {
                    Some(format!("LUT ({}) entries", count))
                }
            } else {
                None
            }
        }
        "para" => {
            if offset + 8 <= data.len() {
                let gct = u16::from_be_bytes([data[offset + 4], data[offset + 5]]);
                Some(format!("Parametric type {}", gct))
            } else {
                None
            }
        }
        "sf32" => Some("sRGB".to_string()),
        _ => Some(format!("Unknown TRC type: {}", sig)),
    }
}

fn decode_tag(data: &[u8], offset: usize, size: usize, tag_type: &str) -> Option<String> {
    match tag_type {
        "text" => {
            let end = std::cmp::min(offset + size, data.len());
            let s = String::from_utf8_lossy(&data[offset..end]);
            Some(s.trim_matches(char::from(0)).to_string())
        }
        "desc" => extract_desc(data, offset, size),
        "XYZ " => {
            if offset + 20 <= data.len() {
                let x = read_s15fixed16(data, offset + 4);
                let y = read_s15fixed16(data, offset + 8);
                let z = read_s15fixed16(data, offset + 12);
                Some(format!("X={:.4} Y={:.4} Z={:.4}", x, y, z))
            } else {
                None
            }
        }
        "sig " => read_tag4(data, offset),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_data_too_short() {
        let result = parse_icc(&[0u8; 100]);
        assert!(result.is_none());
    }

    #[test]
    fn rejects_empty_data() {
        let result = parse_icc(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn parses_minimal_icc_header() {
        let mut data = vec![0u8; 132];
        data[0..4].copy_from_slice(&132u32.to_be_bytes());
        data[12..16].copy_from_slice(b"abc ");
        data[20..24].copy_from_slice(b"scnr");
        data[28..32].copy_from_slice(b"RGB ");
        data[32..36].copy_from_slice(b"XYZ ");
        data[64..68].copy_from_slice(&0u32.to_be_bytes());
        data[68..72].copy_from_slice(&63193i32.to_be_bytes());
        data[72..76].copy_from_slice(&65536i32.to_be_bytes());
        data[76..80].copy_from_slice(&54033i32.to_be_bytes());
        data[128..132].copy_from_slice(&0u32.to_be_bytes());

        let result = parse_icc(&data);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.size, 132);
        assert_eq!(info.cmm_type, "abc ");
        assert_eq!(info.profile_class, "scnr");
        assert_eq!(info.color_space, "RGB ");
        assert_eq!(info.pcs, "XYZ ");
        assert_eq!(info.rendering_intent, "Perceptual");
        assert_eq!(info.tag_count, 0);
        assert!(info.tags.is_empty());
    }

    #[test]
    fn decode_version_format() {
        let mut data = vec![0u8; 48];
        data[44] = 4;
        data[45] = 0x20;
        assert_eq!(decode_version(&data), "4.2.0");
    }

    #[test]
    fn read_s15fixed16_values() {
        let one = [0x00, 0x01, 0x00, 0x00];
        assert!((read_s15fixed16(&one, 0) - 1.0).abs() < 0.0001);

        let half = [0x00, 0x00, 0x80, 0x00];
        assert!((read_s15fixed16(&half, 0) - 0.5).abs() < 0.0001);

        let zero = [0x00, 0x00, 0x00, 0x00];
        assert!((read_s15fixed16(&zero, 0) - 0.0).abs() < 0.0001);
    }

    #[test]
    fn read_tag4_success() {
        let data = b"abcd";
        assert_eq!(read_tag4(data, 0), Some("abcd".to_string()));
    }

    #[test]
    fn read_tag4_out_of_bounds() {
        let data = b"ab";
        assert!(read_tag4(data, 0).is_none());
    }

    #[test]
    fn describe_trc_linear() {
        let data = [b'c', b'u', b'r', b'v', 0, 0, 0, 0];
        assert_eq!(
            describe_trc(&data, 0, data.len()),
            Some("Linear".to_string())
        );
    }

    #[test]
    fn describe_trc_gamma() {
        let data = [b'c', b'u', b'r', b'v', 0, 0, 0, 1, 0x02, 0x40];
        assert_eq!(
            describe_trc(&data, 0, data.len()),
            Some("Gamma=2.25".to_string())
        );
    }

    #[test]
    fn describe_trc_srgb() {
        let data: &[u8] = b"sf32";
        assert_eq!(describe_trc(data, 0, data.len()), Some("sRGB".to_string()));
    }

    #[test]
    fn describe_trc_lut() {
        let data = [b'c', b'u', b'r', b'v', 0, 0, 1, 0];
        assert_eq!(
            describe_trc(&data, 0, data.len()),
            Some("LUT (256) entries".to_string())
        );
    }

    #[test]
    fn decode_tag_xyz() {
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(b"XYZ ");
        data[4..8].copy_from_slice(&65536i32.to_be_bytes());
        data[8..12].copy_from_slice(&32768i32.to_be_bytes());
        data[12..16].copy_from_slice(&16384i32.to_be_bytes());

        assert_eq!(
            decode_tag(&data, 0, 20, "XYZ "),
            Some("X=1.0000 Y=0.5000 Z=0.2500".to_string())
        );
    }

    #[test]
    fn decode_tag_text() {
        let data = b"hello world\x00\x00\x00";
        let result = decode_tag(data, 0, data.len(), "text");
        assert_eq!(result, Some("hello world".to_string()));
    }
}
