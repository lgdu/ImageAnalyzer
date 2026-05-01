use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{bytes_to_hex, read_file_bytes};

// GIF constants
const GIF87A_SIGNATURE: &[u8] = b"GIF87a";
const GIF89A_SIGNATURE: &[u8] = b"GIF89a";

// Block introducer bytes
const BLOCK_IMAGE_DESCRIPTOR: u8 = 0x2C;
const BLOCK_EXTENSION: u8 = 0x21;
const BLOCK_TRAILER: u8 = 0x3B;

// Extension labels
const EXT_GRAPHIC_CONTROL: u8 = 0xF9;
const EXT_COMMENT: u8 = 0xFE;
const EXT_PLAIN_TEXT: u8 = 0x01;
const EXT_APPLICATION: u8 = 0xFF;

/// Packed byte helpers
fn gct_flag(packed: u8) -> bool {
    (packed & 0x80) != 0
}

fn gct_size(packed: u8) -> usize {
    let n = (packed & 0x07) as usize;
    3 * (1 << (n + 1))
}

fn lct_flag(packed: u8) -> bool {
    (packed & 0x80) != 0
}

fn interlace_flag(packed: u8) -> bool {
    (packed & 0x40) != 0
}

fn lct_size(packed: u8) -> usize {
    let n = (packed & 0x07) as usize;
    3 * (1 << (n + 1))
}

fn color_resolution(packed: u8) -> u8 {
    ((packed >> 4) & 0x07) + 1
}

fn disposal_method(packed: u8) -> u8 {
    (packed >> 2) & 0x07
}

fn disposal_method_name(dm: u8) -> &'static str {
    match dm {
        0 => "No disposal specified",
        1 => "Do not dispose",
        2 => "Restore to background color",
        3 => "Restore to previous",
        4..=7 => "Reserved",
        _ => "Unknown",
    }
}

/// Parse a sub-block sequence. Sub-blocks start with a size byte (0-255),
/// followed by that many bytes. A size of 0 terminates the sequence.
fn parse_sub_blocks(data: &[u8], pos: usize) -> (usize, usize) {
    // Returns (bytes_consumed, total_data_bytes)
    let start = pos;
    let mut total_data = 0;
    let mut p = pos;

    while p < data.len() {
        let block_size = data[p] as usize;
        p += 1;
        if block_size == 0 {
            break;
        }
        if p + block_size > data.len() {
            break;
        }
        total_data += block_size;
        p += block_size;
    }

    (p - start, total_data)
}

pub fn analyze_gif(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_size = bytes.len() as u64;

    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Verify GIF signature (6 bytes: "GIF87a" or "GIF89a")
    if bytes.len() < 13 {
        return Err("File too small to be a valid GIF".to_string());
    }

    let version = if &bytes[0..6] == GIF87A_SIGNATURE {
        "GIF87a".to_string()
    } else if &bytes[0..6] == GIF89A_SIGNATURE {
        "GIF89a".to_string()
    } else {
        return Err(format!(
            "Invalid GIF signature: expected 'GIF87a' or 'GIF89a', found '{}'",
            String::from_utf8_lossy(&bytes[0..6])
        ));
    };

    // Logical Screen Descriptor (7 bytes starting at offset 6)
    let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
    let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
    let packed = bytes[10];
    let bg_color_index = bytes[11];
    let pixel_aspect_ratio = bytes[12];

    let has_gct = gct_flag(packed);
    let gct_size = gct_size(packed);
    let cr = color_resolution(packed);

    let mut structure: Vec<FileBlock> = Vec::new();
    let mut metadata: Vec<MetadataEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Header block
    structure.push(FileBlock {
        name: "Header".to_string(),
        offset: 0,
        length: 6,
        data_preview: Some(version.clone()),
        decoded_info: Some(format!("GIF version: {}", version)),
        children: vec![],
    });

    // Logical Screen Descriptor block
    let lsd_info = format!(
        "Width: {}, Height: {}, GCT: {} ({} bytes), Color Resolution: {} bpp, BG index: {}, Pixel aspect: {}",
        width,
        height,
        if has_gct { "yes" } else { "no" },
        gct_size,
        cr,
        bg_color_index,
        if pixel_aspect_ratio == 0 {
            "none".to_string()
        } else {
            format!("{}:1", (pixel_aspect_ratio as f64 + 15.0) / 64.0)
        }
    );

    let mut lsd_children: Vec<FileBlock> = Vec::new();
    if has_gct {
        lsd_children.push(FileBlock {
            name: "Global Color Table".to_string(),
            offset: 13,
            length: gct_size as u64,
            data_preview: Some(bytes_to_hex(&bytes[13..13 + gct_size.min(32)], 32)),
            decoded_info: Some(format!("{} palette entries", gct_size / 3)),
            children: vec![],
        });
    }

    structure.push(FileBlock {
        name: "Logical Screen Descriptor".to_string(),
        offset: 6,
        length: 7 + if has_gct { gct_size as u64 } else { 0 },
        data_preview: None,
        decoded_info: Some(lsd_info),
        children: lsd_children,
    });

    let mut pos: usize = 13 + if has_gct { gct_size } else { 0 };

    let mut frame_count: u32 = 0;
    let mut has_animation = false;
    let mut loop_count: u16 = 0;
    let mut comments: Vec<String> = Vec::new();
    let mut last_gce_delay: Option<u16> = None;

    while pos < bytes.len() {
        let block_offset = pos as u64;
        let block_type = bytes[pos];

        match block_type {
            BLOCK_TRAILER => {
                // 0x3B: GIF trailer
                structure.push(FileBlock {
                    name: "Trailer".to_string(),
                    offset: block_offset,
                    length: 1,
                    data_preview: Some("3b".to_string()),
                    decoded_info: Some("End of GIF".to_string()),
                    children: vec![],
                });
                break;
            }
            BLOCK_IMAGE_DESCRIPTOR => {
                // 0x2C: Image Descriptor
                frame_count += 1;
                if pos + 10 > bytes.len() {
                    errors.push(format!("Incomplete Image Descriptor at offset {}", pos));
                    break;
                }

                let left = u16::from_le_bytes([bytes[pos + 1], bytes[pos + 2]]) as u32;
                let top = u16::from_le_bytes([bytes[pos + 3], bytes[pos + 4]]) as u32;
                let img_width = u16::from_le_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
                let img_height = u16::from_le_bytes([bytes[pos + 7], bytes[pos + 8]]) as u32;
                let img_packed = bytes[pos + 9];

                let has_lct = lct_flag(img_packed);
                let is_interlaced = interlace_flag(img_packed);
                let lct_size = if has_lct { lct_size(img_packed) } else { 0 };

                let mut img_info = format!(
                    "Frame {}: {}x{} at ({},{}), {}{}",
                    frame_count,
                    img_width,
                    img_height,
                    left,
                    top,
                    if has_lct {
                        format!("LCT ({} bytes)", lct_size)
                    } else {
                        "uses GCT".to_string()
                    },
                    if is_interlaced { ", interlaced" } else { "" },
                );

                // Add delay info if we have a preceding GCE
                if let Some(delay) = last_gce_delay {
                    img_info.push_str(&format!(", delay: {}ms", delay * 10));
                }

                let mut img_children: Vec<FileBlock> = Vec::new();

                // Local color table
                if has_lct {
                    let lct_start = pos + 10;
                    img_children.push(FileBlock {
                        name: "Local Color Table".to_string(),
                        offset: lct_start as u64,
                        length: lct_size as u64,
                        data_preview: Some(bytes_to_hex(
                            &bytes[lct_start..lct_start + lct_size.min(32)],
                            32,
                        )),
                        decoded_info: Some(format!("{} palette entries", lct_size / 3)),
                        children: vec![],
                    });
                }

                // Image data: LZW minimum code size (1 byte) + sub-blocks
                let mut data_pos = pos + 10 + lct_size;
                if data_pos < bytes.len() {
                    let lzw_min_code_size = bytes[data_pos];
                    data_pos += 1;

                    let (sub_blocks_len, total_sub_data) = parse_sub_blocks(&bytes, data_pos);

                    img_children.push(FileBlock {
                        name: "Image Data".to_string(),
                        offset: (data_pos - 1) as u64,
                        length: (1 + sub_blocks_len) as u64,
                        data_preview: Some(format!(
                            "LZW min code size: {}, {} bytes of compressed data",
                            lzw_min_code_size, total_sub_data
                        )),
                        decoded_info: Some(format!(
                            "LZW compressed, {} sub-blocks, {} data bytes",
                            count_sub_blocks(&bytes, data_pos),
                            total_sub_data
                        )),
                        children: vec![],
                    });

                    pos = data_pos + sub_blocks_len;
                } else {
                    errors.push(format!(
                        "Image data extends beyond file at offset {}",
                        data_pos
                    ));
                    break;
                }

                if frame_count > 1 {
                    has_animation = true;
                }

                structure.push(FileBlock {
                    name: "Image Descriptor".to_string(),
                    offset: block_offset,
                    length: (pos as u64) - block_offset,
                    data_preview: None,
                    decoded_info: Some(img_info),
                    children: img_children,
                });

                // Reset GCE after image descriptor consumes it
                last_gce_delay = None;
            }
            BLOCK_EXTENSION => {
                // 0x21: Extension block
                if pos + 1 >= bytes.len() {
                    errors.push(format!("Incomplete extension at offset {}", pos));
                    break;
                }

                let ext_label = bytes[pos + 1];

                match ext_label {
                    EXT_GRAPHIC_CONTROL => {
                        // Graphic Control Extension (0xF9)
                        if pos + 8 > bytes.len() {
                            errors.push(format!("Incomplete GCE at offset {}", pos));
                            break;
                        }

                        let block_size = bytes[pos + 2]; // should be 4
                        if block_size != 4 {
                            errors.push(format!(
                                "GCE block size should be 4, found {} at offset {}",
                                block_size, pos
                            ));
                        }

                        let gce_packed = bytes[pos + 3];
                        let dm = disposal_method(gce_packed);
                        let delay = u16::from_le_bytes([bytes[pos + 4], bytes[pos + 5]]);
                        let transparent_index = bytes[pos + 6];
                        let _terminator = bytes[pos + 7]; // should be 0

                        last_gce_delay = Some(delay);

                        let gce_info = format!(
                            "Disposal: {} ({}), Delay: {} ({}ms), Transparent index: {}",
                            dm,
                            disposal_method_name(dm),
                            delay,
                            delay * 10,
                            transparent_index
                        );

                        structure.push(FileBlock {
                            name: "Graphic Control Extension".to_string(),
                            offset: block_offset,
                            length: 8,
                            data_preview: None,
                            decoded_info: Some(gce_info),
                            children: vec![],
                        });

                        pos += 8;
                    }
                    EXT_COMMENT => {
                        // Comment Extension (0xFE)
                        // Starts with block size, then sub-blocks
                        let comment_start = pos + 2;
                        let (sub_blocks_len, _) = parse_sub_blocks(&bytes, comment_start);

                        // Extract comment text
                        let mut comment_data = Vec::new();
                        let mut cp = comment_start;
                        while cp < comment_start + sub_blocks_len && cp < bytes.len() {
                            let bs = bytes[cp] as usize;
                            cp += 1;
                            if bs == 0 {
                                break;
                            }
                            if cp + bs <= bytes.len() {
                                comment_data.extend_from_slice(&bytes[cp..cp + bs]);
                            }
                            cp += bs;
                        }

                        let comment_text = String::from_utf8_lossy(&comment_data).to_string();
                        if !comment_text.is_empty() {
                            comments.push(comment_text.clone());
                        }

                        structure.push(FileBlock {
                            name: "Comment Extension".to_string(),
                            offset: block_offset,
                            length: (2 + sub_blocks_len) as u64,
                            data_preview: Some(bytes_to_hex(
                                &bytes[comment_start..comment_start + sub_blocks_len.min(32)],
                                32,
                            )),
                            decoded_info: if comment_text.is_empty() {
                                Some("Empty comment".to_string())
                            } else {
                                let preview = if comment_text.len() > 100 {
                                    format!("{}...", &comment_text[..100])
                                } else {
                                    comment_text.clone()
                                };
                                Some(format!("Comment: {}", preview))
                            },
                            children: vec![],
                        });

                        pos = comment_start + sub_blocks_len;
                    }
                    EXT_PLAIN_TEXT => {
                        // Plain Text Extension (0x01)
                        let pt_start = pos + 2;
                        if pt_start >= bytes.len() {
                            errors
                                .push(format!("Incomplete Plain Text Extension at offset {}", pos));
                            break;
                        }

                        let block_size = bytes[pt_start - 1]; // should be 12
                        let (sub_blocks_len, _) =
                            parse_sub_blocks(&bytes, pt_start + block_size as usize);

                        let total_len = 1 + block_size as usize + sub_blocks_len;

                        structure.push(FileBlock {
                            name: "Plain Text Extension".to_string(),
                            offset: block_offset,
                            length: (1 + total_len) as u64,
                            data_preview: Some(bytes_to_hex(&bytes[pt_start..], 32)),
                            decoded_info: Some(format!(
                                "Plain text, {} bytes of data",
                                sub_blocks_len
                            )),
                            children: vec![],
                        });

                        pos = pt_start + block_size as usize + sub_blocks_len;
                    }
                    EXT_APPLICATION => {
                        // Application Extension (0xFF)
                        // Used for NETSCAPE2.0 loop count
                        // Layout: 0x21 (introducer), 0xFF (label), block_size(1), app_id(block_size), sub-blocks, terminator
                        if pos + 3 >= bytes.len() {
                            errors.push(format!(
                                "Incomplete Application Extension at offset {}",
                                pos
                            ));
                            break;
                        }

                        let block_size = bytes[pos + 2] as usize;
                        let app_id_start = pos + 3;
                        let app_id_data = if app_id_start + block_size <= bytes.len() {
                            &bytes[app_id_start..app_id_start + block_size]
                        } else {
                            &bytes[app_id_start..]
                        };

                        let app_identifier = String::from_utf8_lossy(app_id_data).to_string();

                        let sub_start = app_id_start + block_size;
                        let (sub_blocks_len, _) = parse_sub_blocks(&bytes, sub_start);

                        // Check for NETSCAPE2.0
                        if app_identifier.starts_with("NETSCAPE2.0")
                            || app_identifier.starts_with("ANIMEXTS1.0")
                        {
                            // Parse loop count from first sub-block
                            if sub_start < bytes.len() {
                                let first_sub_size = bytes[sub_start] as usize;
                                if first_sub_size >= 3 && sub_start + 3 <= bytes.len() {
                                    let sub_label = bytes[sub_start + 1]; // 0x01 for loop count
                                    if sub_label == 0x01 {
                                        loop_count = u16::from_le_bytes([
                                            bytes[sub_start + 2],
                                            bytes[sub_start + 3],
                                        ]);
                                    }
                                }
                            }

                            let loop_str = if loop_count == 0 {
                                "infinite"
                            } else {
                                "finite"
                            };

                            structure.push(FileBlock {
                                name: "Application Extension".to_string(),
                                offset: block_offset,
                                length: (2 + block_size + sub_blocks_len) as u64,
                                data_preview: Some(bytes_to_hex(
                                    &bytes[app_id_start
                                        ..app_id_start + (block_size + sub_blocks_len).min(32)],
                                    32,
                                )),
                                decoded_info: Some(format!(
                                    "NETSCAPE2.0: loop count = {} ({})",
                                    loop_count, loop_str
                                )),
                                children: vec![],
                            });
                        } else {
                            structure.push(FileBlock {
                                name: "Application Extension".to_string(),
                                offset: block_offset,
                                length: (2 + block_size + sub_blocks_len) as u64,
                                data_preview: Some(bytes_to_hex(
                                    &bytes[app_id_start
                                        ..app_id_start + (block_size + sub_blocks_len).min(32)],
                                    32,
                                )),
                                decoded_info: Some(format!(
                                    "Application: {}, {} bytes of data",
                                    app_identifier, sub_blocks_len
                                )),
                                children: vec![],
                            });
                        }

                        pos = sub_start + sub_blocks_len;
                    }
                    _ => {
                        // Unknown extension label
                        let ext_start = pos + 2;
                        let (sub_blocks_len, _) = parse_sub_blocks(&bytes, ext_start);

                        structure.push(FileBlock {
                            name: format!("Extension (0x{:02X})", ext_label),
                            offset: block_offset,
                            length: (2 + sub_blocks_len) as u64,
                            data_preview: Some(bytes_to_hex(
                                &bytes[pos..pos + 16.min(bytes.len() - pos)],
                                16,
                            )),
                            decoded_info: Some(format!(
                                "Unknown extension label 0x{:02X}, {} bytes",
                                ext_label, sub_blocks_len
                            )),
                            children: vec![],
                        });

                        pos = ext_start + sub_blocks_len;
                    }
                }
            }
            _ => {
                // Unknown block type — this is likely a data corruption or parsing error
                errors.push(format!(
                    "Unknown block type 0x{:02X} at offset {}",
                    block_type, pos
                ));
                break;
            }
        }
    }

    // Check if we found a trailer
    let has_trailer = structure.iter().any(|b| b.name == "Trailer");
    if !has_trailer && errors.is_empty() {
        // Non-fatal: many GIFs are missing the trailer
        errors.push("No GIF trailer (0x3B) found (possibly truncated)".to_string());
    }

    // Determine animation status
    if frame_count > 1 {
        has_animation = true;
    }

    // Add comment metadata
    for (i, comment) in comments.iter().enumerate() {
        metadata.push(MetadataEntry {
            standard: "GIF".to_string(),
            tag_name: format!("Comment {}", i + 1),
            tag_value: comment.clone(),
            raw_value: Some(comment.clone()),
        });
    }

    // Build color_type and bit_depth
    let color_type = if has_animation {
        format!("Indexed (animated, {} frames)", frame_count)
    } else {
        "Indexed".to_string()
    };
    let bit_depth: u8 = 8; // GIF is always 8-bit indexed

    // Add animation info to the header block
    if has_animation {
        if let Some(header) = structure.first_mut() {
            header.decoded_info = Some(format!(
                "GIF {} animated: {}x{}, {} frames, loop count: {}",
                version,
                width,
                height,
                frame_count,
                if loop_count == 0 {
                    "infinite".to_string()
                } else {
                    loop_count.to_string()
                }
            ));
        }
    }

    Ok(ImageAnalysis {
        file_name,
        file_path: path.to_string(),
        file_size,
        format: ImageFormat::Gif,
        width,
        height,
        color_type,
        bit_depth,
        has_alpha: false, // GIF89a supports transparency via transparent index, but not true alpha channel
        structure,
        metadata,
        channels: crate::analyzer::channel_split::compute_channels(&bytes),
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}

/// Count the number of sub-blocks (for display purposes)
fn count_sub_blocks(data: &[u8], start_pos: usize) -> usize {
    let mut count = 0;
    let mut p = start_pos;
    while p < data.len() {
        let bs = data[p] as usize;
        p += 1;
        if bs == 0 {
            break;
        }
        count += 1;
        p += bs;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_static_gif() {
        let result = analyze_gif("/tmp/image-analyzer-tests/test_static.gif");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let analysis = result.unwrap();
        assert!(matches!(analysis.format, ImageFormat::Gif));
        assert_eq!(analysis.width, 80);
        assert_eq!(analysis.height, 60);
        assert!(!analysis.has_alpha);
        assert_eq!(analysis.color_type, "Indexed");
        assert_eq!(analysis.bit_depth, 8);
        assert!(analysis.structure.iter().any(|b| b.name == "Header"));
        assert!(analysis
            .structure
            .iter()
            .any(|b| b.name == "Logical Screen Descriptor"));
    }

    #[test]
    fn analyzes_animated_gif() {
        let result = analyze_gif("/tmp/image-analyzer-tests/test_animated.gif");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let analysis = result.unwrap();
        assert!(matches!(analysis.format, ImageFormat::Gif));
        assert_eq!(analysis.width, 4);
        assert_eq!(analysis.height, 4);
        assert!(analysis.color_type.contains("animated"));
        assert!(analysis.color_type.contains("2 frames"));
        // Check NETSCAPE2.0 extension was detected
        assert!(analysis
            .structure
            .iter()
            .any(|b| b.name == "Application Extension"));
        // Check multiple image descriptors (frames)
        let img_desc_count = analysis
            .structure
            .iter()
            .filter(|b| b.name == "Image Descriptor")
            .count();
        assert!(
            img_desc_count >= 2,
            "Expected at least 2 frames, found {}",
            img_desc_count
        );
    }

    #[test]
    fn rejects_invalid_gif() {
        let result = analyze_gif("/tmp/image-analyzer-tests/test_lossy.webp");
        assert!(result.is_err(), "Should have failed for non-GIF file");
    }

    #[test]
    fn rejects_missing_file() {
        let result = analyze_gif("/tmp/image-analyzer-tests/nonexistent.gif");
        assert!(result.is_err());
    }

    #[test]
    fn gct_size_calculation() {
        // GCT flag set, size field = 2 => 3 * 2^(2+1) = 24 bytes
        assert_eq!(gct_size(0x82), 24);
        // GCT flag set, size field = 0 => 3 * 2^(0+1) = 6 bytes
        assert_eq!(gct_size(0x80), 6);
        // GCT flag set, size field = 7 => 3 * 2^(7+1) = 768 bytes
        assert_eq!(gct_size(0x87), 768);
    }

    #[test]
    fn sub_block_parsing() {
        // Build a simple sub-block sequence: size=3, data, size=2, data, terminator=0
        let data = vec![3, 1, 2, 3, 2, 4, 5, 0];
        let (consumed, total_data) = parse_sub_blocks(&data, 0);
        assert_eq!(consumed, 8); // 1+3 + 1+2 + 1(terminator) = 8
        assert_eq!(total_data, 5); // 3 + 2 = 5
    }
}
