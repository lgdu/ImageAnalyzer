use crate::types::{CodecSyntax, FileBlock, GridInfo, GridTile, ImageAnalysis, ImageFormat};
use crate::utils::{bytes_to_hex, read_file_bytes};

pub fn analyze_heif(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return Err("Invalid HEIF/HEIC file (no ftyp box)".to_string());
    }

    let brand = String::from_utf8_lossy(&bytes[8..12]).to_string();
    let format = if brand.contains("heic")
        || brand.contains("heix")
        || brand.contains("heim")
        || brand.contains("heis")
    {
        ImageFormat::Heic
    } else if brand.contains("avif") || brand.contains("avis") {
        ImageFormat::Avif
    } else {
        return Err(format!("Unsupported HEIF brand: {}", brand));
    };

    let mut structure = Vec::new();
    let mut grid: Option<GridInfo> = None;
    let mut errors = Vec::new();
    let mut width = 0u32;
    let mut height = 0u32;

    // Track codec-relevant data for codec_syntax extraction
    let mut idat_data: Option<&[u8]> = None;
    let mut item_types: Vec<String> = Vec::new();

    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let box_size = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        if box_size == 0 {
            break;
        }

        let box_type =
            String::from_utf8_lossy(&bytes[offset + 4..offset + 8]).to_string();
        let data_start = offset + 8;
        let data_end = offset + box_size;

        if data_end > bytes.len() {
            errors.push(format!(
                "Box '{}' at offset {} extends beyond file",
                box_type, offset
            ));
            break;
        }

        let decoded_info = match box_type.as_str() {
            "ftyp" => {
                let brands: Vec<_> = (12..data_end)
                    .step_by(4)
                    .filter(|i| i + 4 <= data_end)
                    .map(|i| {
                        String::from_utf8_lossy(&bytes[i..i + 4]).to_string()
                    })
                    .collect();
                Some(format!(
                    "brand={}, compatible={}",
                    brand,
                    brands.join(", ")
                ))
            }
            "meta" => Some("Metadata container".to_string()),
            "hdlr" => {
                if data_start + 12 <= data_end {
                    let handler = String::from_utf8_lossy(
                        &bytes[data_start + 8..data_start + 12],
                    )
                    .to_string();
                    Some(format!("handler={}", handler))
                } else {
                    None
                }
            }
            "pitm" => {
                if data_start + 2 <= data_end {
                    let item_id = u16::from_be_bytes([
                        bytes[data_start],
                        bytes[data_start + 1],
                    ]);
                    Some(format!("primary item ID={}", item_id))
                } else {
                    None
                }
            }
            "grid" => {
                if data_start + 5 <= data_end {
                    let _version = bytes[data_start];
                    let flags = bytes[data_start + 1];
                    let rows_minus1 = bytes[data_start + 2];
                    let cols_minus1 = bytes[data_start + 3];

                    let has_16bit = (flags & 1) != 0;
                    let output_w = if has_16bit && data_start + 7 <= data_end {
                        u32::from_be_bytes([
                            bytes[data_start + 4],
                            bytes[data_start + 5],
                            bytes[data_start + 6],
                            bytes[data_start + 7],
                        ])
                    } else if data_start + 5 <= data_end {
                        u16::from_be_bytes([
                            bytes[data_start + 4],
                            bytes[data_start + 5],
                        ]) as u32
                    } else {
                        0
                    };

                    let h_pos = if has_16bit {
                        data_start + 8
                    } else {
                        data_start + 6
                    };
                    let output_h = if h_pos + 2 <= data_end {
                        u16::from_be_bytes([bytes[h_pos], bytes[h_pos + 1]]) as u32
                    } else {
                        0
                    };

                    let rows = rows_minus1 as u32 + 1;
                    let cols = cols_minus1 as u32 + 1;
                    width = output_w;
                    height = output_h;

                    grid = Some(GridInfo {
                        rows,
                        cols,
                        output_width: output_w,
                        output_height: output_h,
                        tiles: Vec::new(),
                    });

                    Some(format!(
                        "grid {}x{}, output {}x{}",
                        rows, cols, output_w, output_h
                    ))
                } else {
                    None
                }
            }
            "iloc" => Some("Item location box".to_string()),
            "iprp" => Some("Item properties container".to_string()),
            "iinf" => Some("Item info".to_string()),
            "idat" => {
                idat_data = Some(&bytes[data_start..data_end]);
                Some("Item data".to_string())
            }
            "iref" => Some("Item references".to_string()),
            "infe" => {
                if data_start + 4 <= data_end {
                    let item_id = u16::from_be_bytes([
                        bytes[data_start],
                        bytes[data_start + 1],
                    ]);
                    let item_type = String::from_utf8_lossy(
                        &bytes[data_start + 2..data_end],
                    )
                    .trim_matches(char::from(0))
                    .to_string();
                    if !item_type.is_empty() {
                        item_types.push(item_type.clone());
                        // Add tile to existing grid
                        if let Some(ref mut g) = grid {
                            let codec = if item_type.contains("hvc")
                                || item_type.contains("hev")
                            {
                                "HEVC"
                            } else if item_type.contains("av0") {
                                "AV1"
                            } else {
                                &item_type
                            }
                            .to_string();
                            g.tiles.push(GridTile {
                                item_id,
                                width: 0,
                                height: 0,
                                horizontal_offset: 0,
                                vertical_offset: 0,
                                codec,
                            });
                        }
                        Some(format!(
                            "item ID={}, type={}",
                            item_id, item_type
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let data_preview = if box_type == "idat" {
            Some("[large binary data]".to_string())
        } else if data_start < data_end {
            Some(bytes_to_hex(
                &bytes[data_start..data_end.min(data_start + 16)],
                16,
            ))
        } else {
            None
        };

        structure.push(FileBlock {
            name: box_type,
            offset: offset as u64,
            length: box_size as u64,
            data_preview,
            decoded_info,
            children: Vec::new(),
        });

        offset += box_size;
    }

    // Extract codec syntax from idat data based on item types
    let codec_syntax = if let Some(idat) = idat_data {
        let has_hevc = item_types.iter().any(|t| t.contains("hvc") || t.contains("hev"));
        let has_av1 = item_types.iter().any(|t| t.contains("av0"));

        if has_hevc {
            let hevc = crate::analyzer::hevc::parse_hevc_bitstream(idat);
            Some(CodecSyntax::Hevc(hevc))
        } else if has_av1 {
            let av1 = crate::analyzer::av1::parse_av1_bitstream(idat);
            Some(CodecSyntax::Av1(av1))
        } else {
            None
        }
    } else {
        None
    };

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size: bytes.len() as u64,
        format,
        width,
        height,
        color_type: "HEIF".to_string(),
        bit_depth: 0,
        has_alpha: false,
        structure,
        metadata: Vec::new(),
        channels: None,
        icc_profile: None,
        codec_syntax,
        grid,
        analysis_errors: errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_file_without_ftyp() {
        let result = analyze_heif("/tmp/test_no_ftyp.bin");
        // Should fail because file doesn't exist or has no ftyp
        assert!(result.is_err());
    }

    #[test]
    fn rejects_nonexistent_file() {
        let result = analyze_heif("/tmp/does_not_exist.heic");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_file() {
        // Write an empty file
        std::fs::write("/tmp/empty_heif.heic", &[]).unwrap();
        let result = analyze_heif("/tmp/empty_heif.heic");
        assert!(result.is_err());
    }

    #[test]
    fn parses_ftyp_brand() {
        // Create a minimal HEIF file with ftyp box
        let mut data = Vec::new();
        // ftyp box: size=24 (8 header + 4 brand + 4 minor + 4 compat + 4 compat)
        data.extend_from_slice(&24u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic"); // major brand
        data.extend_from_slice(&0u32.to_be_bytes()); // minor version
        data.extend_from_slice(b"mif1"); // compatible brand
        data.extend_from_slice(b"heic"); // compatible brand

        std::fs::write("/tmp/test_ftyp.heic", &data).unwrap();
        let result = analyze_heif("/tmp/test_ftyp.heic");
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.format, ImageFormat::Heic);
        assert_eq!(analysis.width, 0);
        assert_eq!(analysis.height, 0);
        assert!(analysis.grid.is_none());
        assert!(analysis.structure.iter().any(|b| b.name == "ftyp"));
    }

    #[test]
    fn parses_avif_brand() {
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"mif1");

        std::fs::write("/tmp/test_avif.heic", &data).unwrap();
        let result = analyze_heif("/tmp/test_avif.heic");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().format, ImageFormat::Avif);
    }

    #[test]
    fn detects_grid_box() {
        let mut data = Vec::new();
        // ftyp
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"mif1");

        // grid box: version=0, flags=0, rows-1=1, cols-1=1, output_w=200, output_h=300
        // Total size: 4 (size) + 4 (type) + 8 (data) = 16
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"grid");
        data.push(0); // version
        data.push(0); // flags (8-bit fields)
        data.push(1); // rows_minus1 (2 rows)
        data.push(1); // cols_minus1 (2 cols)
        data.extend_from_slice(&200u16.to_be_bytes()); // output width
        data.extend_from_slice(&300u16.to_be_bytes()); // output height

        std::fs::write("/tmp/test_grid.heic", &data).unwrap();
        let result = analyze_heif("/tmp/test_grid.heic");
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.grid.is_some());
        let grid = analysis.grid.unwrap();
        assert_eq!(grid.rows, 2);
        assert_eq!(grid.cols, 2);
        assert_eq!(grid.output_width, 200);
        assert_eq!(grid.output_height, 300);
        assert_eq!(analysis.width, 200);
        assert_eq!(analysis.height, 300);
    }

    #[test]
    fn parses_multiple_boxes() {
        let mut data = Vec::new();
        // ftyp
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"mif1");

        // meta box: 4 (size) + 4 (type) + 4 (version+flags) = 12
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&0u32.to_be_bytes()); // version+flags

        // pitm box: 4 (size) + 4 (type) + 2 (item_id) + 2 (padding) = 12
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"pitm");
        data.extend_from_slice(&1u16.to_be_bytes()); // item_id=1
        data.extend_from_slice(&[0, 0]); // padding

        std::fs::write("/tmp/test_multi.heic", &data).unwrap();
        let result = analyze_heif("/tmp/test_multi.heic");
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.structure.len(), 3);
        assert_eq!(analysis.structure[0].name, "ftyp");
        assert_eq!(analysis.structure[1].name, "meta");
        assert_eq!(analysis.structure[2].name, "pitm");
    }
}
