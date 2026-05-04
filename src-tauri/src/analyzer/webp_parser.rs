use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{bytes_to_hex, read_file_bytes};

use super::{exif_reader, xmp_reader};

/// Decompress zlib-compressed WebP ICCP data
fn decompress_iccp_chunk(data: &[u8]) -> Option<Vec<u8>> {
    // WebP ICCP chunk: null-terminated profile name, then zlib-compressed ICC profile
    let name_end = data.iter().position(|&b| b == 0)? + 1;
    // Pad to even byte boundary if needed
    let icc_start = if name_end % 2 != 0 {
        name_end + 1
    } else {
        name_end
    };
    if icc_start >= data.len() {
        return None;
    }
    let compressed = &data[icc_start..];
    miniz_oxide::inflate::decompress_to_vec(compressed).ok()
}

/// Extract raw ICC profile bytes from WebP ICCP chunk
pub fn extract_icc_data(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 20 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }

    let file_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if bytes.len() < file_size + 8 {
        return None;
    }

    let mut pos: usize = 12;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;

        if fourcc == CHUNK_ICCP {
            let data_start = pos + 8;
            let data_end = data_start + chunk_size;
            if data_end <= bytes.len() {
                return decompress_iccp_chunk(&bytes[data_start..data_end]);
            }
        }

        pos += 8 + chunk_size;
        // Chunks are padded to even byte boundary
        if chunk_size % 2 != 0 {
            pos += 1;
        }
    }
    None
}

fn format_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1024 * 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", n as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// WebP RIFF constants
const RIFF_SIGNATURE: &[u8] = b"RIFF";
const WEBP_SIGNATURE: &[u8] = b"WEBP";

// Chunk fourccs
const CHUNK_VP8: [u8; 4] = *b"VP8 ";
const CHUNK_VP8L: [u8; 4] = *b"VP8L";
const CHUNK_VP8X: [u8; 4] = *b"VP8X";
const CHUNK_ANIM: [u8; 4] = *b"ANIM";
const CHUNK_ANMF: [u8; 4] = *b"ANMF";
const CHUNK_ICCP: [u8; 4] = *b"ICCP";
const CHUNK_EXIF: [u8; 4] = *b"EXIF";
const CHUNK_XMP: [u8; 4] = *b"XMP ";
const CHUNK_ALPH: [u8; 4] = *b"ALPH";

/// VP8X flag bits
const FLAG_ICCP: u8 = 0x20;
const FLAG_ALPHA: u8 = 0x10;
const FLAG_EXIF: u8 = 0x08;
const FLAG_XMP: u8 = 0x04;
const FLAG_ANIMATION: u8 = 0x02;

/// Result of parsing a VP8 bitstream
struct Vp8Info {
    width: u32,
    height: u32,
}

/// Parse the VP8 lossy bitstream header to extract dimensions.
/// VP8 bitstream: frame tag (1-3 bytes), then sync code (0x9D 0x01 0x2A) for keyframes,
/// then width (2 bytes LE) and height (2 bytes LE) masked with 0x3FFF.
fn parse_vp8_header(data: &[u8]) -> Option<Vp8Info> {
    if data.len() < 10 {
        return None;
    }

    // Check if it's a keyframe (bit 0 = 0)
    let frame_tag = data[0];
    let is_keyframe = (frame_tag & 0x01) == 0;
    if !is_keyframe {
        return None;
    }

    // Find the keyframe sync code 0x9D 0x01 0x2A within the first few bytes.
    // Frame tag can be 1-3 bytes depending on show_frame and partition flags.
    let sync_start = (1..=3).find(|&i| {
        data.len() >= i + 3 && data[i] == 0x9D && data[i + 1] == 0x01 && data[i + 2] == 0x2A
    })?;

    // After sync code: 3 bytes (width as 16-bit LE & 0x3FFF, height as 16-bit LE & 0x3FFF)
    let base = sync_start + 3;
    if data.len() < base + 6 {
        return None;
    }

    let width = ((data[base] as u16) | ((data[base + 1] as u16) << 8)) & 0x3FFF;
    let height = ((data[base + 2] as u16) | ((data[base + 3] as u16) << 8)) & 0x3FFF;

    Some(Vp8Info {
        width: width as u32,
        height: height as u32,
    })
}

/// Parse the VP8L lossless bitstream header to extract dimensions.
/// VP8L starts with a 1-byte signature (0x2F), then a 4-byte header
/// containing width-1 (14 bits) and height-1 (14 bits).
fn parse_vp8l_header(data: &[u8]) -> Option<Vp8Info> {
    if data.len() < 5 {
        return None;
    }

    // First byte must be 0x2F (VP8L signature)
    if data[0] != 0x2F {
        return None;
    }

    // Next 4 bytes: bitstream header
    let b0 = data[1] as u32;
    let b1 = data[2] as u32;
    let b2 = data[3] as u32;
    let b3 = data[4] as u32;

    // Width: bits 0-13 of the 32-bit value (after removing signature)
    // Height: bits 14-27
    let val = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    let width = (val & 0x3FFF) + 1;
    let height = ((val >> 14) & 0x3FFF) + 1;

    Some(Vp8Info { width, height })
}

/// Parse VP8X extended header (10 bytes)
fn parse_vp8x_header(data: &[u8]) -> Option<Vp8xInfo> {
    if data.len() < 10 {
        return None;
    }

    let flags = data[0];
    // bytes 1-3 are reserved (should be 0)
    // Canvas width: bytes 3-6 (24-bit LE), stored as width - 1
    let canvas_width_minus_1 =
        (data[3] as u32) | ((data[4] as u32) << 8) | ((data[5] as u32) << 16);
    // Canvas height: bytes 6-10 (24-bit LE), stored as height - 1
    let canvas_height_minus_1 =
        (data[6] as u32) | ((data[7] as u32) << 8) | ((data[8] as u32) << 16);

    Some(Vp8xInfo {
        flags,
        width: canvas_width_minus_1 + 1,
        height: canvas_height_minus_1 + 1,
        has_icc: (flags & FLAG_ICCP) != 0,
        has_alpha: (flags & FLAG_ALPHA) != 0,
        has_exif: (flags & FLAG_EXIF) != 0,
        has_xmp: (flags & FLAG_XMP) != 0,
        is_animated: (flags & FLAG_ANIMATION) != 0,
    })
}

struct Vp8xInfo {
    #[allow(dead_code)]
    flags: u8,
    width: u32,
    height: u32,
    has_icc: bool,
    has_alpha: bool,
    has_exif: bool,
    has_xmp: bool,
    is_animated: bool,
}

/// Parse ANIM chunk (6 bytes):
/// - background_color (4 bytes, ARGB)
/// - loop_count (2 bytes LE)
fn parse_anim_chunk(data: &[u8]) -> Option<String> {
    if data.len() < 6 {
        return None;
    }
    let bg_r = data[1];
    let bg_g = data[2];
    let bg_b = data[3];
    let loop_count = u16::from_le_bytes([data[4], data[5]]);
    Some(format!(
        "Background: #{:02X}{:02X}{:02X}, Loop count: {}",
        bg_r, bg_g, bg_b, loop_count
    ))
}

/// Parse ANMF chunk header (12+ bytes):
/// - X position (24-bit LE)
/// - Y position (24-bit LE)
/// - Width (24-bit LE), stored as width - 1
/// - Height (24-bit LE), stored as height - 1
/// - Duration (24-bit LE, ms)
/// - Reserved (1 byte)
/// - Blending/Disposal flags (1 byte)
/// - Then VP8/VP8L/ALPH data
fn parse_anmf_header(data: &[u8]) -> Option<AnmfInfo> {
    if data.len() < 16 {
        return None;
    }

    let x = (data[0] as u32) | ((data[1] as u32) << 8) | ((data[2] as u32) << 16);
    let y = (data[3] as u32) | ((data[4] as u32) << 8) | ((data[5] as u32) << 16);
    let width = ((data[6] as u32) | ((data[7] as u32) << 8) | ((data[8] as u32) << 16)) + 1;
    let height = ((data[9] as u32) | ((data[10] as u32) << 8) | ((data[11] as u32) << 16)) + 1;
    let duration_ms = (data[12] as u32) | ((data[13] as u32) << 8) | ((data[14] as u32) << 16);

    Some(AnmfInfo {
        x,
        y,
        width,
        height,
        duration_ms,
    })
}

struct AnmfInfo {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    duration_ms: u32,
}

/// Read a 4-byte chunk fourcc from the data at pos
fn read_fourcc(data: &[u8], pos: usize) -> Option<[u8; 4]> {
    if pos + 4 > data.len() {
        return None;
    }
    Some([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
}

/// Read a 4-byte little-endian u32 from the data at pos
fn read_u32_le(data: &[u8], pos: usize) -> Option<u32> {
    if pos + 4 > data.len() {
        return None;
    }
    Some(
        (data[pos] as u32)
            | ((data[pos + 1] as u32) << 8)
            | ((data[pos + 2] as u32) << 16)
            | ((data[pos + 3] as u32) << 24),
    )
}

/// WebP chunks are padded to even byte alignment.
/// If chunk data size is odd, there is a padding byte.
fn chunk_payload_size_with_padding(data_size: u32) -> usize {
    if data_size % 2 == 1 {
        data_size as usize + 1
    } else {
        data_size as usize
    }
}

pub fn analyze_webp(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_size = bytes.len() as u64;

    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Verify RIFF signature (bytes 0-3: "RIFF")
    if bytes.len() < 12 {
        return Err("File too small to be a valid WebP".to_string());
    }

    if &bytes[0..4] != RIFF_SIGNATURE {
        return Err(format!(
            "Invalid WebP signature: expected 'RIFF', found '{}'",
            String::from_utf8_lossy(&bytes[0..4])
        ));
    }

    // Read file size (bytes 4-7, little-endian u32, total file size - 8)
    let riff_size = read_u32_le(&bytes, 4).ok_or("Failed to read RIFF size")?;

    // Verify WEBP signature (bytes 8-11: "WEBP")
    if &bytes[8..12] != WEBP_SIGNATURE {
        return Err(format!(
            "Invalid WebP container: expected 'WEBP', found '{}'",
            String::from_utf8_lossy(&bytes[8..12])
        ));
    }

    let mut structure: Vec<FileBlock> = Vec::new();
    let mut metadata: Vec<MetadataEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Root RIFF block
    structure.push(FileBlock {
        name: "RIFF".to_string(),
        offset: 0,
        length: file_size,
        data_preview: Some(format!(
            "WEBP container, declared size: {} bytes",
            riff_size
        )),
        decoded_info: Some(format!(
            "RIFF size field: {} (total file: {})",
            riff_size, file_size
        )),
        fields: Vec::new(),
        children: vec![],
    });

    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut has_alpha: bool = false;
    let mut color_type = String::from("RGB");
    let bit_depth: u8 = 8; // WebP is always 8-bit per channel
    let mut is_animated = false;
    let mut loop_count: u16 = 0;
    let mut frame_count: u32 = 0;

    // Parse chunks starting at offset 12 (after RIFF header + WEBP signature)
    let mut pos: usize = 12;

    while pos + 8 <= bytes.len() {
        let chunk_offset = pos as u64;

        // Read chunk fourcc
        let fourcc = match read_fourcc(&bytes, pos) {
            Some(f) => f,
            None => {
                errors.push(format!("Incomplete chunk fourcc at offset {}", pos));
                break;
            }
        };

        // Read chunk data size (4 bytes LE)
        let data_size = match read_u32_le(&bytes, pos + 4) {
            Some(s) => s,
            None => {
                errors.push(format!("Incomplete chunk size at offset {}", pos));
                break;
            }
        };

        let chunk_name = String::from_utf8_lossy(&fourcc).to_string();

        // Calculate total bytes for this chunk: 4 (fourcc) + 4 (size) + data (+ padding)
        let payload_with_padding = chunk_payload_size_with_padding(data_size);
        let total_chunk_size = 8 + payload_with_padding;

        if pos + total_chunk_size > bytes.len() {
            errors.push(format!(
                "Chunk '{}' at offset {} extends beyond file (needs {} bytes, have {})",
                chunk_name,
                pos,
                total_chunk_size,
                bytes.len() - pos
            ));
            // Still record what we can
            let _available = bytes.len() - pos - 8;
            structure.push(FileBlock {
                name: chunk_name.clone(),
                offset: chunk_offset,
                length: total_chunk_size as u64,
                data_preview: Some(bytes_to_hex(&bytes[pos + 8..], 32)),
                decoded_info: Some(format!(
                    "{} (truncated, expected {} data bytes)",
                    chunk_name, data_size
                )),
                fields: Vec::new(),
                children: vec![],
            });
            break;
        }

        let data_start = pos + 8;
        let data_end = data_start + data_size as usize;
        let chunk_data = &bytes[data_start..data_end];

        let mut decoded_info: Option<String> = None;
        let mut children: Vec<FileBlock> = Vec::new();

        if fourcc == CHUNK_VP8 {
            color_type = "YCbCr (VP8 lossy)".to_string();
            has_alpha = false;
            if let Some(info) = parse_vp8_header(chunk_data) {
                width = info.width;
                height = info.height;
                decoded_info = Some(format!("VP8 lossy: {}x{} pixels", info.width, info.height));
            } else {
                decoded_info = Some(format!(
                    "VP8 lossy: {} bytes (header parse failed)",
                    data_size
                ));
                errors.push(format!(
                    "Failed to parse VP8 header at offset {}",
                    chunk_offset
                ));
            }
        } else if fourcc == CHUNK_VP8L {
            color_type = "RGBA (VP8L lossless)".to_string();
            if let Some(info) = parse_vp8l_header(chunk_data) {
                width = info.width;
                height = info.height;
                has_alpha = true; // VP8L always supports alpha bit
                decoded_info = Some(format!(
                    "VP8L lossless: {}x{} pixels",
                    info.width, info.height
                ));
            } else {
                decoded_info = Some(format!(
                    "VP8L lossless: {} bytes (header parse failed)",
                    data_size
                ));
                errors.push(format!(
                    "Failed to parse VP8L header at offset {}",
                    chunk_offset
                ));
            }
        } else if fourcc == CHUNK_VP8X {
            if let Some(vp8x) = parse_vp8x_header(chunk_data) {
                width = vp8x.width;
                height = vp8x.height;
                has_alpha = vp8x.has_alpha;
                is_animated = vp8x.is_animated;

                let mut flags_str = Vec::new();
                if vp8x.has_icc {
                    flags_str.push("ICCP");
                }
                if vp8x.has_alpha {
                    flags_str.push("ALPHA");
                }
                if vp8x.has_exif {
                    flags_str.push("EXIF");
                }
                if vp8x.has_xmp {
                    flags_str.push("XMP");
                }
                if vp8x.is_animated {
                    flags_str.push("ANIMATION");
                }

                decoded_info = Some(format!(
                    "VP8X: {}x{}, flags: {}",
                    vp8x.width,
                    vp8x.height,
                    if flags_str.is_empty() {
                        "none".to_string()
                    } else {
                        flags_str.join(", ")
                    }
                ));
            } else {
                errors.push(format!(
                    "Failed to parse VP8X header at offset {}",
                    chunk_offset
                ));
            }
        } else if fourcc == CHUNK_ANIM {
            decoded_info = parse_anim_chunk(chunk_data);
            if let Some(ref info) = decoded_info {
                if let Some(loop_val) = info.split("Loop count: ").nth(1) {
                    if let Ok(lc) = loop_val.parse::<u16>() {
                        loop_count = lc;
                    }
                }
            }
        } else if fourcc == CHUNK_ANMF {
            frame_count += 1;
            // Parse the frame header (first 16 bytes)
            if let Some(anmf) = parse_anmf_header(chunk_data) {
                decoded_info = Some(format!(
                    "Frame {}: {}x{} at ({},{}), duration {}ms",
                    frame_count, anmf.width, anmf.height, anmf.x, anmf.y, anmf.duration_ms
                ));

                // Parse sub-chunks within the ANMF payload (after the 16-byte header)
                let mut sub_pos: usize = 16;
                while sub_pos < chunk_data.len() {
                    if sub_pos + 8 > chunk_data.len() {
                        break;
                    }
                    let sub_fourcc = read_fourcc(chunk_data, sub_pos).unwrap();
                    let sub_data_size = read_u32_le(chunk_data, sub_pos + 4).unwrap();
                    let sub_name = String::from_utf8_lossy(&sub_fourcc).to_string();
                    let sub_payload = chunk_payload_size_with_padding(sub_data_size);
                    let sub_total = 8 + sub_payload;

                    if sub_pos + sub_total > chunk_data.len() {
                        break;
                    }

                    let sub_data_start = sub_pos + 8;
                    let sub_data_end = sub_data_start + sub_data_size as usize;
                    let sub_data = &chunk_data[sub_data_start..sub_data_end];

                    let sub_decoded = if sub_fourcc == CHUNK_ALPH {
                        Some(format!("Alpha channel data, {} bytes", sub_data_size))
                    } else if sub_fourcc == CHUNK_VP8 {
                        Some(format!("VP8 compressed data, {} bytes", sub_data_size))
                    } else if sub_fourcc == CHUNK_VP8L {
                        has_alpha = true;
                        Some(format!("VP8L compressed data, {} bytes", sub_data_size))
                    } else {
                        Some(format!("{} data, {} bytes", sub_name, sub_data_size))
                    };

                    children.push(FileBlock {
                        name: sub_name,
                        offset: chunk_offset + 16 + sub_pos as u64,
                        length: sub_total as u64,
                        data_preview: Some(bytes_to_hex(sub_data, 32)),
                        decoded_info: sub_decoded,
                        fields: Vec::new(),
                        children: vec![],
                    });

                    sub_pos += sub_total;
                }
            } else {
                decoded_info = Some(format!("ANMF frame {}, {} bytes", frame_count, data_size));
            }
        } else if fourcc == CHUNK_ICCP {
            decoded_info = Some(format!("ICC profile (compressed, {} bytes)", data_size));
        } else if fourcc == CHUNK_EXIF {
            decoded_info = Some(format!("EXIF metadata, {} bytes", data_size));
            // EXIF chunk starts with TIFF header (8 bytes):
            // "II" (0x4949) or "MM" (0x4D4D) for byte order, then 0x002A
            if chunk_data.len() > 8 {
                match exif_reader::read_exif(chunk_data) {
                    Ok(entries) => metadata.extend(entries),
                    Err(e) => errors.push(format!("EXIF extraction error: {}", e)),
                }
            }
        } else if fourcc == CHUNK_XMP {
            decoded_info = Some(format!("XMP metadata, {} bytes", data_size));
            match xmp_reader::read_xmp(chunk_data) {
                Ok(entries) => metadata.extend(entries),
                Err(e) => errors.push(format!("XMP extraction error: {}", e)),
            }
        } else if fourcc == CHUNK_ALPH {
            has_alpha = true;
            decoded_info = Some(format!("Alpha chunk, {} bytes", data_size));
        } else {
            decoded_info = Some(format!(
                "Unknown chunk '{}', {} bytes",
                chunk_name, data_size
            ));
        }

        let block = FileBlock {
            name: chunk_name.clone(),
            offset: chunk_offset,
            length: total_chunk_size as u64,
            data_preview: Some(bytes_to_hex(chunk_data, 32)),
            decoded_info,
            fields: Vec::new(),
            children,
        };

        structure.push(block);
        pos += total_chunk_size;
    }

    // Validate we found at least one image-bearing chunk
    if width == 0 || height == 0 {
        errors.push("No VP8/VP8L/VP8X chunk found with dimensions".to_string());
    }

    // Verify RIFF size consistency
    if riff_size as u64 + 8 != file_size {
        errors.push(format!(
            "RIFF size mismatch: declared {} (+8 = {}), actual file size {}",
            riff_size,
            riff_size as u64 + 8,
            file_size
        ));
    }

    // Add animation info to root block's decoded info
    if is_animated {
        if let Some(root) = structure.first_mut() {
            root.decoded_info = Some(format!(
                "Animated WebP: {}x{}, {} frames, {} loops",
                width, height, frame_count, loop_count
            ));
        }
    }

    // Update color_type for animated WebP
    if is_animated {
        color_type = format!("Animated WebP ({})", color_type);
    }

    // Add structural metadata
    metadata.push(MetadataEntry {
        standard: "File".to_string(),
        tag_name: "Format".to_string(),
        tag_value: "WebP".to_string(),
        raw_value: None,
    });
    metadata.push(MetadataEntry {
        standard: "File".to_string(),
        tag_name: "File Size".to_string(),
        tag_value: format_bytes(file_size),
        raw_value: None,
    });
    if width > 0 {
        metadata.push(MetadataEntry {
            standard: "Image".to_string(),
            tag_name: "Dimensions".to_string(),
            tag_value: format!("{width} × {height}"),
            raw_value: None,
        });
        if is_animated {
            metadata.push(MetadataEntry {
                standard: "Animation".to_string(),
                tag_name: "Frames".to_string(),
                tag_value: format!("{frame_count}"),
                raw_value: None,
            });
            metadata.push(MetadataEntry {
                standard: "Animation".to_string(),
                tag_name: "Loop Count".to_string(),
                tag_value: format!("{loop_count}"),
                raw_value: None,
            });
        }
    }
    metadata.push(MetadataEntry {
        standard: "Image".to_string(),
        tag_name: "Color Type".to_string(),
        tag_value: color_type.clone(),
        raw_value: None,
    });
    metadata.push(MetadataEntry {
        standard: "Image".to_string(),
        tag_name: "Bit Depth".to_string(),
        tag_value: format!("{bit_depth}"),
        raw_value: None,
    });
    metadata.push(MetadataEntry {
        standard: "Image".to_string(),
        tag_name: "Has Alpha".to_string(),
        tag_value: if has_alpha { "Yes" } else { "No" }.to_string(),
        raw_value: None,
    });

    Ok(ImageAnalysis {
        file_name,
        file_path: path.to_string(),
        file_size,
        format: ImageFormat::Webp,
        width,
        height,
        color_type,
        bit_depth,
        has_alpha,
        thumbnail_base64: None,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vp8_with_3byte_frame_tag() {
        // Simulates the user's file: 3-byte frame tag (50 ed 0e) then sync code
        // data[0] = 0x50 -> keyframe (bit 0 = 0)
        // frame tag = 3 bytes, sync code at offset 3
        let mut vp8_data = vec![0u8; 20];
        vp8_data[0] = 0x50; // keyframe, 3-byte tag
        vp8_data[1] = 0xed;
        vp8_data[2] = 0x0e;
        vp8_data[3] = 0x9D; // sync code
        vp8_data[4] = 0x01;
        vp8_data[5] = 0x2A;
        // width at offset 6-7: 0x800b -> 0b80 07 -> actually bytes 6,7 after sync
        // In real file: data[base] = 0x40, data[base+1] = 0x0b
        // width = (0x40 | (0x0b << 8)) & 0x3FFF = 0x0B40 = 2880
        vp8_data[6] = 0x40;
        vp8_data[7] = 0x0b;
        vp8_data[8] = 0x80;
        vp8_data[9] = 0x07;
        // height = (0x80 | (0x07 << 8)) & 0x3FFF = 0x0780 = 1920
        let info = parse_vp8_header(&vp8_data).expect("should parse 3-byte tag VP8");
        assert_eq!(info.width, 2880);
        assert_eq!(info.height, 1920);
    }

    #[test]
    fn analyzes_lossy_webp() {
        let result = analyze_webp("/tmp/image-analyzer-tests/test_lossy.webp");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let analysis = result.unwrap();
        assert!(matches!(analysis.format, ImageFormat::Webp));
        assert_eq!(analysis.width, 100);
        assert_eq!(analysis.height, 80);
        // image crate may encode as VP8L even for "lossy" requests on small images
        assert!(analysis
            .structure
            .iter()
            .any(|b| b.name == "VP8 " || b.name == "VP8L"));
    }

    #[test]
    fn analyzes_lossless_webp() {
        let result = analyze_webp("/tmp/image-analyzer-tests/test_lossless.webp");
        assert!(result.is_ok(), "Failed: {:?}", result);
        let analysis = result.unwrap();
        assert!(matches!(analysis.format, ImageFormat::Webp));
        assert_eq!(analysis.width, 50);
        assert_eq!(analysis.height, 50);
        assert!(analysis.has_alpha);
        assert!(analysis.structure.iter().any(|b| b.name == "VP8L"));
    }

    #[test]
    fn rejects_invalid_webp() {
        let result = analyze_webp("/tmp/image-analyzer-tests/test_static.gif");
        assert!(result.is_err(), "Should have failed for non-WebP file");
    }

    #[test]
    fn rejects_missing_file() {
        let result = analyze_webp("/tmp/image-analyzer-tests/nonexistent.webp");
        assert!(result.is_err());
    }
}
