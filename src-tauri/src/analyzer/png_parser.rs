use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{bytes_to_hex, read_file_bytes};

/// 8-byte PNG signature: \x89PNG\r\n\x1a\n
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Well-known PNG chunk types
const CHUNK_IHDR: &[u8; 4] = b"IHDR";
#[allow(dead_code)]
const CHUNK_PLTE: &[u8; 4] = b"PLTE";
const CHUNK_IDAT: &[u8; 4] = b"IDAT";
const CHUNK_IEND: &[u8; 4] = b"IEND";
const CHUNK_TEXP: &[u8; 4] = b"tEXt";
const CHUNK_ZTXT: &[u8; 4] = b"zTXt";
const CHUNK_ITXT: &[u8; 4] = b"iTXt";
const CHUNK_GAMA: &[u8; 4] = b"gAMA";
#[allow(dead_code)]
const CHUNK_CHRM: &[u8; 4] = b"cHRM";
#[allow(dead_code)]
const CHUNK_SRGB: &[u8; 4] = b"sRGB";
#[allow(dead_code)]
const CHUNK_ICCP: &[u8; 4] = b"iCCP";
#[allow(dead_code)]
const CHUNK_TIME: &[u8; 4] = b"tIME";
#[allow(dead_code)]
const CHUNK_PHYS: &[u8; 4] = b"pHYs";
#[allow(dead_code)]
const CHUNK_SBIT: &[u8; 4] = b"sBIT";
#[allow(dead_code)]
const CHUNK_BKGD: &[u8; 4] = b"bKGD";
#[allow(dead_code)]
const CHUNK_TRNS: &[u8; 4] = b"tRNS";

/// PNG color type names
fn color_type_name(ct: u8) -> &'static str {
    match ct {
        0 => "Grayscale",
        2 => "RGB",
        3 => "Indexed",
        4 => "Grayscale+Alpha",
        6 => "RGBA",
        _ => "Unknown",
    }
}

/// Check if a color type includes alpha
fn color_type_has_alpha(ct: u8) -> bool {
    ct == 4 || ct == 6
}

/// Parse a single chunk from raw bytes at the given position.
/// Returns (chunk_name_string, data_length, total_bytes_consumed).
/// total_bytes_consumed = 4 (length) + 4 (name) + data_length + 4 (CRC)
fn parse_chunk_header(data: &[u8], pos: usize) -> Result<(String, u32, usize), String> {
    if pos + 8 > data.len() {
        return Err(format!(
            "Not enough data for chunk header at offset {} (file len {})",
            pos,
            data.len()
        ));
    }

    let length = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    let name = String::from_utf8_lossy(&data[pos + 4..pos + 8]).to_string();

    // Validate chunk name is ASCII letters only
    if !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!("Invalid chunk name '{}' at offset {}", name, pos));
    }

    let total = 4 + 4 + length as usize + 4; // length + name + data + CRC
    Ok((name, length, total))
}

/// Decode IHDR chunk data (13 bytes)
fn decode_ihdr(data: &[u8]) -> Option<IhdrInfo> {
    if data.len() < 13 {
        return None;
    }
    Some(IhdrInfo {
        width: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        height: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        bit_depth: data[8],
        color_type: data[9],
        compression_method: data[10],
        filter_method: data[11],
        interlace_method: data[12],
    })
}

struct IhdrInfo {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    #[allow(dead_code)]
    compression_method: u8,
    #[allow(dead_code)]
    filter_method: u8,
    #[allow(dead_code)]
    interlace_method: u8,
}

/// Decode gAMA chunk (4 bytes, big-endian u32 representing gamma * 100000)
fn decode_gamma(data: &[u8]) -> Option<f64> {
    if data.len() < 4 {
        return None;
    }
    let gamma_int = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    Some(gamma_int as f64 / 100_000.0)
}

/// Decode sRGB rendering intent
fn decode_srgb_intent(data: &[u8]) -> Option<&'static str> {
    if data.is_empty() {
        return None;
    }
    match data[0] {
        0 => Some("Perceptual"),
        1 => Some("Relative Colorimetric"),
        2 => Some("Saturation"),
        3 => Some("Absolute Colorimetric"),
        _ => Some("Unknown"),
    }
}

/// Decode tIME chunk (7 bytes: year(2), month, day, hour, minute, second)
fn decode_time(data: &[u8]) -> Option<String> {
    if data.len() < 7 {
        return None;
    }
    let year = u16::from_be_bytes([data[0], data[1]]);
    let month = data[2];
    let day = data[3];
    let hour = data[4];
    let minute = data[5];
    let second = data[6];
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    ))
}

/// Decode pHYs chunk (9 bytes: pixels per unit X(4), Y(4), unit specifier(1))
fn decode_phys(data: &[u8]) -> Option<String> {
    if data.len() < 9 {
        return None;
    }
    let ppx = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let ppy = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let unit = data[8];
    let unit_str = if unit == 1 { "meter" } else { "unknown" };
    Some(format!("{} x {} pixels per {}", ppx, ppy, unit_str))
}

/// Decode a tEXt chunk: keyword\0text
fn decode_text_chunk(data: &[u8]) -> Option<(String, String)> {
    let null_pos = data.iter().position(|&b| b == 0)?;
    let keyword = String::from_utf8_lossy(&data[..null_pos]).to_string();
    let value = String::from_utf8_lossy(&data[null_pos + 1..]).to_string();
    Some((keyword, value))
}

/// Decode a zTXt chunk: keyword\0compression_method(1)\0compressed_text
fn decode_ztxt_chunk(data: &[u8]) -> Option<(String, String)> {
    let null_pos = data.iter().position(|&b| b == 0)?;
    let keyword = String::from_utf8_lossy(&data[..null_pos]).to_string();
    if null_pos + 2 > data.len() {
        return Some((keyword, "<zTXt decompression not implemented>".to_string()));
    }
    let _compression_method = data[null_pos + 1];
    // The compressed text starts after the second null byte
    let compressed_start = null_pos + 2;
    let compressed = &data[compressed_start..];
    Some((
        keyword,
        format!("<zTXt compressed, {} bytes, deflate>", compressed.len()),
    ))
}

/// Decode an iTXt chunk (more complex: keyword\0compression_flag(1)\0compression_method(1)\0language_tag\0translated_keyword\0text)
fn decode_itxt_chunk(data: &[u8]) -> Option<(String, String)> {
    let null_pos = data.iter().position(|&b| b == 0)?;
    let keyword = String::from_utf8_lossy(&data[..null_pos]).to_string();

    if null_pos + 3 > data.len() {
        return Some((keyword, "<malformed iTXt>".to_string()));
    }

    let compression_flag = data[null_pos + 1];
    let _compression_method = data[null_pos + 2];

    // Find language tag (null-terminated after compression method)
    let rest = &data[null_pos + 3..];
    let null2 = rest.iter().position(|&b| b == 0)?;
    let language_tag = String::from_utf8_lossy(&rest[..null2]).to_string();

    // Find translated keyword
    let rest2 = &rest[null2 + 1..];
    let null3 = rest2.iter().position(|&b| b == 0)?;
    let _translated_keyword = String::from_utf8_lossy(&rest2[..null3]).to_string();

    // The remaining data is the text
    let text_data = &rest2[null3 + 1..];

    if compression_flag == 1 {
        Some((
            keyword,
            format!(
                "<iTXt compressed, lang={}, {} bytes, deflate>",
                language_tag,
                text_data.len()
            ),
        ))
    } else {
        let text = String::from_utf8_lossy(text_data).to_string();
        Some((keyword, text))
    }
}

/// Build a data preview string for a chunk (hex of first N bytes of data)
fn build_data_preview(chunk_data: &[u8]) -> String {
    bytes_to_hex(chunk_data, 32)
}

/// Build a decoded info string based on chunk type
fn build_decoded_info(name: &str, chunk_data: &[u8]) -> Option<String> {
    match name {
        "IHDR" => {
            let info = decode_ihdr(chunk_data)?;
            Some(format!(
                "Width: {}, Height: {}, Bit Depth: {}, Color Type: {} ({})",
                info.width,
                info.height,
                info.bit_depth,
                info.color_type,
                color_type_name(info.color_type)
            ))
        }
        "gAMA" => {
            let gamma = decode_gamma(chunk_data)?;
            Some(format!("Gamma: {}", gamma))
        }
        "sRGB" => {
            let intent = decode_srgb_intent(chunk_data)?;
            Some(format!("Rendering intent: {}", intent))
        }
        "tIME" => {
            let time = decode_time(chunk_data)?;
            Some(format!("Last modified: {}", time))
        }
        "pHYs" => {
            let phys = decode_phys(chunk_data)?;
            Some(format!("Physical dimensions: {}", phys))
        }
        "tEXt" => {
            let (keyword, value) = decode_text_chunk(chunk_data)?;
            let preview = if value.len() > 100 {
                format!("{}...", &value[..100])
            } else {
                value
            };
            Some(format!("{}: {}", keyword, preview))
        }
        "zTXt" => {
            let (keyword, value) = decode_ztxt_chunk(chunk_data)?;
            Some(format!("{}: {}", keyword, value))
        }
        "iTXt" => {
            let (keyword, value) = decode_itxt_chunk(chunk_data)?;
            let preview = if value.len() > 100 {
                format!("{}...", &value[..100])
            } else {
                value
            };
            Some(format!("{}: {}", keyword, preview))
        }
        "PLTE" => {
            let count = chunk_data.len() / 3;
            Some(format!("{} palette entries", count))
        }
        "IDAT" => Some(format!(
            "{} bytes of compressed image data",
            chunk_data.len()
        )),
        "IEND" => Some("End of PNG image".to_string()),
        "tRNS" => Some(format!("Transparency data, {} bytes", chunk_data.len())),
        "iCCP" => {
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                let profile_name = String::from_utf8_lossy(&chunk_data[..null_pos]).to_string();
                Some(format!(
                    "ICC profile: {}, {} bytes compressed",
                    profile_name,
                    chunk_data.len() - null_pos - 2
                ))
            } else {
                Some(format!("ICC profile, {} bytes", chunk_data.len()))
            }
        }
        "cHRM" => {
            if chunk_data.len() >= 32 {
                // 4 bytes * 8 values (white point x,y + primary x,y for R,G,B)
                let decode_fixed = |data: &[u8], i: usize| -> f64 {
                    let val = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
                    val as f64 / 100_000.0
                };
                let wx = decode_fixed(chunk_data, 0);
                let wy = decode_fixed(chunk_data, 4);
                let rx = decode_fixed(chunk_data, 8);
                let ry = decode_fixed(chunk_data, 12);
                let gx = decode_fixed(chunk_data, 16);
                let gy = decode_fixed(chunk_data, 20);
                let bx = decode_fixed(chunk_data, 24);
                let by = decode_fixed(chunk_data, 28);
                Some(format!(
                    "White: ({:.5}, {:.5}), Red: ({:.5}, {:.5}), Green: ({:.5}, {:.5}), Blue: ({:.5}, {:.5})",
                    wx, wy, rx, ry, gx, gy, bx, by
                ))
            } else {
                Some(format!("Chromaticity data, {} bytes", chunk_data.len()))
            }
        }
        "sBIT" => Some(format!("Significant bits, {} bytes", chunk_data.len())),
        "bKGD" => Some(format!("Background color, {} bytes", chunk_data.len())),
        _ => None,
    }
}

pub fn analyze_png(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_size = bytes.len() as u64;

    // Extract file name from path
    let file_name = path.split('/').next_back().unwrap_or("unknown").to_string();

    // Verify PNG signature
    if bytes.len() < 8 {
        return Err("File too small to be a valid PNG".to_string());
    }

    if bytes[..8] != PNG_SIGNATURE {
        return Err("Invalid PNG signature".to_string());
    }

    let mut structure: Vec<FileBlock> = Vec::new();
    let mut metadata: Vec<MetadataEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut color_type: u8 = 0;
    let mut bit_depth: u8 = 0;
    let mut has_alpha: bool = false;

    // Parse chunks starting after the 8-byte signature
    let mut pos: usize = 8;

    while pos < bytes.len() {
        // Record chunk offset (relative to file start)
        let chunk_offset = pos as u64;

        // Parse chunk header
        match parse_chunk_header(&bytes, pos) {
            Ok((name, data_length, total_size)) => {
                // Validate we have enough data
                if pos + total_size > bytes.len() {
                    let err = format!(
                        "Chunk '{}' at offset {} extends beyond file end (needs {} bytes, have {})",
                        name,
                        pos,
                        total_size,
                        bytes.len() - pos
                    );
                    errors.push(err.clone());

                    // Still record what we can as a block
                    let available_data = bytes.len() - pos - 8; // subtract length + name fields
                    let chunk_data = &bytes[pos + 8..pos + 8 + available_data];
                    structure.push(FileBlock {
                        name: name.clone(),
                        offset: chunk_offset,
                        length: total_size as u64,
                        data_preview: Some(build_data_preview(chunk_data)),
                        decoded_info: Some(format!(
                            "{} (truncated, expected {} data bytes)",
                            name, data_length
                        )),
                        children: vec![],
                    });
                    break;
                }

                // Extract chunk data (between name and CRC)
                let data_start = pos + 8;
                let data_end = data_start + data_length as usize;
                let chunk_data = &bytes[data_start..data_end];

                // Build decoded info
                let decoded_info = build_decoded_info(&name, chunk_data);

                // Build data preview
                let data_preview = Some(build_data_preview(chunk_data));

                // Extract IHDR info (first chunk, mandatory)
                if name.as_bytes() == *CHUNK_IHDR {
                    if let Some(ihdr) = decode_ihdr(chunk_data) {
                        width = ihdr.width;
                        height = ihdr.height;
                        bit_depth = ihdr.bit_depth;
                        color_type = ihdr.color_type;
                        has_alpha = color_type_has_alpha(ihdr.color_type);
                    }
                }

                // Extract text metadata
                if name.as_bytes() == *CHUNK_TEXP {
                    if let Some((keyword, value)) = decode_text_chunk(chunk_data) {
                        metadata.push(MetadataEntry {
                            standard: "PNG".to_string(),
                            tag_name: format!("tEXt:{}", keyword),
                            tag_value: value.clone(),
                            raw_value: Some(value),
                        });
                    }
                }

                if name.as_bytes() == *CHUNK_ZTXT {
                    if let Some((keyword, value)) = decode_ztxt_chunk(chunk_data) {
                        metadata.push(MetadataEntry {
                            standard: "PNG".to_string(),
                            tag_name: format!("zTXt:{}", keyword),
                            tag_value: value.clone(),
                            raw_value: Some(value),
                        });
                    }
                }

                if name.as_bytes() == *CHUNK_ITXT {
                    if let Some((keyword, value)) = decode_itxt_chunk(chunk_data) {
                        metadata.push(MetadataEntry {
                            standard: "PNG".to_string(),
                            tag_name: format!("iTXt:{}", keyword),
                            tag_value: value.clone(),
                            raw_value: Some(value),
                        });
                    }
                }

                // Build the file block
                let block = FileBlock {
                    name: name.clone(),
                    offset: chunk_offset,
                    length: total_size as u64,
                    data_preview,
                    decoded_info,
                    children: vec![],
                };
                structure.push(block);

                // Advance past this chunk
                pos += total_size;

                // If we hit IEND, we can stop (but still validate we're at EOF)
                if name.as_bytes() == *CHUNK_IEND {
                    if pos != bytes.len() {
                        errors.push(format!(
                            "Trailing data after IEND: {} bytes",
                            bytes.len() - pos
                        ));
                    }
                    break;
                }
            }
            Err(e) => {
                errors.push(format!("Parse error at offset {}: {}", pos, e));
                break;
            }
        }
    }

    // Determine color type string
    let color_type_str = color_type_name(color_type);

    Ok(ImageAnalysis {
        file_name,
        file_path: path.to_string(),
        file_size,
        format: ImageFormat::Png,
        width,
        height,
        color_type: color_type_str.to_string(),
        bit_depth,
        has_alpha,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}
