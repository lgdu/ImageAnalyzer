use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{bytes_to_hex, read_file_bytes};

use super::{exif_reader, iptc_reader, xmp_reader};

// JPEG marker constants
const SOI: u8 = 0xD8;
const EOI: u8 = 0xD9;
const SOS: u8 = 0xDA;
const COM: u8 = 0xFE;

// SOF markers
const SOF0: u8 = 0xC0;
const SOF1: u8 = 0xC1;
const SOF2: u8 = 0xC2;
const SOF3: u8 = 0xC3;
const SOF5: u8 = 0xC5;
const SOF6: u8 = 0xC6;
const SOF7: u8 = 0xC7;
const SOF9: u8 = 0xC9;
const SOF10: u8 = 0xCA;
const SOF11: u8 = 0xCB;
const SOF13: u8 = 0xCD;
const SOF14: u8 = 0xCE;
const SOF15: u8 = 0xCF;

// APP markers
const APP0: u8 = 0xE0;
const APP1: u8 = 0xE1;
const APP2: u8 = 0xE2;
const APP13: u8 = 0xED;
const APP14: u8 = 0xEE;

// Other markers
const DQT: u8 = 0xDB;
const DHT: u8 = 0xC4;
const DRI: u8 = 0xDD;

/// Known marker names
fn marker_name(byte: u8) -> &'static str {
    match byte {
        SOI => "SOI",
        EOI => "EOI",
        SOS => "SOS",
        COM => "COM",
        DQT => "DQT",
        DHT => "DHT",
        DRI => "DRI",
        SOF0 => "SOF0 (Baseline DCT)",
        SOF1 => "SOF1 (Extended Sequential DCT)",
        SOF2 => "SOF2 (Progressive DCT)",
        SOF3 => "SOF3 (Lossless)",
        SOF5 => "SOF5 (Differential Sequential)",
        SOF6 => "SOF6 (Differential Progressive)",
        SOF7 => "SOF7 (Differential Lossless)",
        SOF9 => "SOF9 (Extended Sequential, Arithmetic)",
        SOF10 => "SOF10 (Progressive, Arithmetic)",
        SOF11 => "SOF11 (Lossless, Arithmetic)",
        SOF13 => "SOF13 (Differential Sequential, Arithmetic)",
        SOF14 => "SOF14 (Differential Progressive, Arithmetic)",
        SOF15 => "SOF15 (Differential Lossless, Arithmetic)",
        APP0 => "APP0 (JFIF)",
        APP1 => "APP1 (EXIF/XMP)",
        APP2 => "APP2 (ICC Profile)",
        0xE3..=0xEC => "APPn",
        APP13 => "APP13 (Photoshop/IPTC)",
        APP14 => "APP14 (Adobe)",
        0xEF..=0xFE => "APPn/RES",
        0x01 => "TEM",
        0xD0..=0xD7 => "RST",
        _ => "UNK",
    }
}

/// Markers that have no payload (standalone)
fn is_standalone_marker(marker: u8) -> bool {
    marker == SOI || marker == EOI || (0xD0..=0xD7).contains(&marker) || marker == 0x01
    // TEM
}

/// SOF marker: has payload with dimensions
fn is_sof_marker(marker: u8) -> bool {
    matches!(
        marker,
        SOF0 | SOF1
            | SOF2
            | SOF3
            | SOF5
            | SOF6
            | SOF7
            | SOF9
            | SOF10
            | SOF11
            | SOF13
            | SOF14
            | SOF15
    )
}

/// Parse SOF marker data to extract image info
fn parse_sof_data(data: &[u8]) -> Option<SofInfo> {
    if data.len() < 6 {
        return None;
    }
    let precision = data[0];
    let height = u16::from_be_bytes([data[1], data[2]]) as u32;
    let width = u16::from_be_bytes([data[3], data[4]]) as u32;
    let num_components = data[5];

    // Parse component info if available
    let mut components = Vec::new();
    let mut pos = 6;
    while pos + 3 <= data.len() && components.len() < num_components as usize {
        let id = data[pos];
        let sampling = data[pos + 1];
        let h = (sampling >> 4) & 0x0F;
        let v = sampling & 0x0F;
        let qt_table = data[pos + 2];
        components.push(ComponentInfo {
            id,
            h_sampling: h,
            v_sampling: v,
            qt_table,
        });
        pos += 3;
    }

    Some(SofInfo {
        precision,
        height,
        width,
        num_components,
        components,
    })
}

struct SofInfo {
    precision: u8,
    height: u32,
    width: u32,
    num_components: u8,
    components: Vec<ComponentInfo>,
}

struct ComponentInfo {
    id: u8,
    h_sampling: u8,
    v_sampling: u8,
    qt_table: u8,
}

fn color_type_from_components(n: u8, sof_name: &str) -> &'static str {
    match n {
        1 => "Grayscale",
        3 => {
            if sof_name.contains("Adobe") {
                "YCbCr (Adobe)"
            } else {
                "YCbCr"
            }
        }
        4 => "YCbCrK (CMYK)",
        _ => "Unknown",
    }
}

fn has_alpha_from_components(_n: u8) -> bool {
    // JPEG does not natively support alpha; always false from SOF
    false
}

/// Build decoded info string for a marker
fn build_marker_decoded_info(marker: u8, data: &[u8]) -> Option<String> {
    match marker {
        m if is_sof_marker(m) => {
            let info = parse_sof_data(data)?;
            let name = marker_name(m);
            Some(format!(
                "Width: {}, Height: {}, Bit Depth: {}, Components: {} ({})",
                info.width,
                info.height,
                info.precision,
                info.num_components,
                color_type_from_components(info.num_components, name).to_string()
            ))
        }
        APP0 => decode_app0(data),
        APP1 => decode_app1_type(data),
        APP2 => decode_app2(data),
        APP13 => decode_app13_type(data),
        APP14 => decode_app14(data),
        COM => {
            let text = String::from_utf8_lossy(data);
            let preview = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text.to_string()
            };
            Some(format!("Comment: {}", preview))
        }
        DQT => {
            let mut tables = 0;
            let mut pos = 0;
            while pos < data.len() {
                if pos >= data.len() {
                    break;
                }
                let tq = data[pos];
                let _precision = (tq >> 4) & 0x0F;
                let _id = tq & 0x0F;
                tables += 1;
                pos += 1;
                // 64 values for 8-bit, 128 for 16-bit
                let count = 64; // simplified
                pos += count * 2; // worst case
            }
            Some(format!("{} quantization table(s)", tables))
        }
        DHT => Some(format!("{} bytes of Huffman table data", data.len())),
        DRI => {
            if data.len() >= 2 {
                let restart_interval = u16::from_be_bytes([data[0], data[1]]);
                Some(format!("Restart interval: {} MCUs", restart_interval))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Decode APP0 (JFIF / JFXX)
fn decode_app0(data: &[u8]) -> Option<String> {
    if data.len() < 5 {
        return None;
    }
    let identifier = &data[0..5];
    if identifier == b"JFIF\0" {
        if data.len() >= 8 {
            let major = data[5];
            let minor = data[6];
            let units = match data[7] {
                0 => "no units",
                1 => "pixels per inch",
                2 => "pixels per cm",
                _ => "unknown units",
            };
            let mut info = format!("JFIF {}.{}", major, minor);
            if data.len() >= 12 {
                let x_density = u16::from_be_bytes([data[8], data[9]]);
                let y_density = u16::from_be_bytes([data[10], data[11]]);
                info.push_str(&format!(
                    ", Density: {} x {} ({})",
                    x_density, y_density, units
                ));
            }
            Some(info)
        } else {
            Some("JFIF".to_string())
        }
    } else if identifier == b"JFXX\0" {
        if data.len() >= 6 {
            let thumbnail_format = match data[5] {
                0x10 => "JPEG thumbnail",
                0x11 => "1bpp RGB thumbnail",
                0x13 => "1bpp YCbCr thumbnail",
                _ => "unknown thumbnail format",
            };
            Some(format!("JFXX: {}", thumbnail_format))
        } else {
            Some("JFXX".to_string())
        }
    } else {
        Some(format!("APP0: {} bytes (non-JFIF)", data.len()))
    }
}

/// Identify APP1 content type
fn decode_app1_type(data: &[u8]) -> Option<String> {
    if data.len() >= 6 && &data[0..6] == b"Exif\0\0" {
        Some("EXIF metadata".to_string())
    } else if data.len() >= 29 && &data[0..29] == b"http://ns.adobe.com/xap/1.0/\0" {
        let xml_len = data.len() - 29;
        Some(format!("XMP metadata ({} bytes XML)", xml_len))
    } else {
        Some(format!("APP1: {} bytes (unknown type)", data.len()))
    }
}

/// Identify APP2 content
fn decode_app2(data: &[u8]) -> Option<String> {
    if data.len() >= 14 && &data[0..14] == b"ICC_PROFILE\0\0\0" {
        if data.len() >= 16 {
            let seq = data[14];
            let total = data[15];
            Some(format!("ICC Profile: part {} of {}", seq, total))
        } else {
            Some("ICC_PROFILE".to_string())
        }
    } else {
        Some(format!("APP2: {} bytes", data.len()))
    }
}

/// Identify APP13 content type
fn decode_app13_type(data: &[u8]) -> Option<String> {
    if data.len() >= 14 && &data[0..14] == b"Photoshop 3.0\0" {
        Some("Photoshop Image Resources (IPTC)".to_string())
    } else {
        Some(format!("APP13: {} bytes", data.len()))
    }
}

/// Decode APP14 (Adobe)
fn decode_app14(data: &[u8]) -> Option<String> {
    if data.len() >= 5 && &data[0..5] == b"Adobe" {
        if data.len() >= 12 {
            let version = u16::from_be_bytes([data[5], data[6]]);
            let flags0 = u16::from_be_bytes([data[7], data[8]]);
            let flags1 = u16::from_be_bytes([data[9], data[10]]);
            let color_transform = data[11];
            let ct_str = match color_transform {
                0 => "Unknown (RGB or CMYK)",
                1 => "YCbCr",
                2 => "YCCK",
                _ => "Reserved",
            };
            Some(format!(
                "Adobe JPEG: v{}, transform={}, flags=0x{:04X}/0x{:04X} ({})",
                version, ct_str, flags0, flags1, ct_str
            ))
        } else {
            Some("Adobe JPEG".to_string())
        }
    } else {
        None
    }
}

/// Parse a single JPEG marker. Returns (marker_byte, data_length_excluding_length_field, total_bytes_consumed).
/// For standalone markers: total_bytes_consumed = 2 (0xFF + marker).
/// For markers with payload: total_bytes_consumed = 2 (0xFF + marker) + 2 (length) + data_length.
fn parse_marker(data: &[u8], pos: usize) -> Result<(u8, usize, usize), String> {
    if pos >= data.len() {
        return Err("Unexpected end of data".to_string());
    }
    if data[pos] != 0xFF {
        return Err(format!(
            "Expected 0xFF marker at offset {}, found 0x{:02X}",
            pos, data[pos]
        ));
    }

    let marker = data[pos + 1];

    if is_standalone_marker(marker) {
        return Ok((marker, 0, 2));
    }

    // Markers with payload: need at least 2 bytes for length
    if pos + 3 >= data.len() {
        return Err(format!(
            "Not enough data for marker length at offset {}",
            pos
        ));
    }

    let length = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
    // length includes the 2 length bytes themselves, so data = length - 2
    if length < 2 {
        return Err(format!(
            "Invalid marker length {} at offset {}",
            length, pos
        ));
    }

    let data_len = length - 2;
    let total = 2 + length; // 0xFF + marker + length field + payload

    Ok((marker, data_len, total))
}

pub fn analyze_jpeg(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_size = bytes.len() as u64;

    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    if bytes.len() < 2 {
        return Err("File too small to be a valid JPEG".to_string());
    }

    // Verify JPEG signature (SOI = 0xFFD8)
    if bytes[0] != 0xFF || bytes[1] != SOI {
        return Err("Invalid JPEG signature: expected 0xFFD8".to_string());
    }

    let mut structure: Vec<FileBlock> = Vec::new();
    let mut metadata: Vec<MetadataEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut bit_depth: u8 = 0;
    let mut color_type = String::from("YCbCr");
    let mut has_alpha: bool = false;

    // SOI block
    structure.push(FileBlock {
        name: "SOI".to_string(),
        offset: 0,
        length: 2,
        data_preview: Some("ff d8".to_string()),
        decoded_info: Some("Start of Image".to_string()),
        children: vec![],
    });

    let mut pos: usize = 2;

    while pos < bytes.len() {
        // Skip padding 0xFF bytes (fill bytes)
        while pos < bytes.len() && bytes[pos] == 0xFF {
            // Check if next byte is also 0xFF (fill) or a real marker
            if pos + 1 < bytes.len() && bytes[pos + 1] != 0xFF && bytes[pos + 1] != 0x00 {
                break;
            }
            // 0xFF 0x00 is a stuffed byte in entropy data, 0xFF 0xFF is fill
            if pos + 1 < bytes.len() && bytes[pos + 1] == 0x00 {
                pos += 2; // skip stuffed byte
                continue;
            }
            if pos + 1 < bytes.len() && bytes[pos + 1] == 0xFF {
                pos += 1; // skip fill byte
                continue;
            }
            break;
        }

        if pos >= bytes.len() {
            break;
        }

        if bytes[pos] != 0xFF {
            errors.push(format!(
                "Expected marker at offset {}, found 0x{:02X}",
                pos, bytes[pos]
            ));
            break;
        }

        let marker_offset = pos as u64;

        match parse_marker(&bytes, pos) {
            Ok((marker, data_len, total_consumed)) => {
                let name = marker_name(marker).to_string();

                // Handle SOS specially: parse header, then skip to next marker
                if marker == SOS {
                    // The SOS header has data_len bytes
                    let data_start = pos + 4; // after 0xFF + marker + 2-byte length
                    if data_start + data_len > bytes.len() {
                        errors.push(format!("SOS header extends beyond file at offset {}", pos));
                        break;
                    }
                    let sos_header = &bytes[data_start..data_start + data_len];

                    // Build SOS block
                    structure.push(FileBlock {
                        name: name.clone(),
                        offset: marker_offset,
                        length: total_consumed as u64,
                        data_preview: Some(bytes_to_hex(sos_header, 32)),
                        decoded_info: Some(format!("Start of Scan, header: {} bytes", data_len)),
                        children: vec![],
                    });

                    // Skip past SOS header to the entropy-coded data
                    pos = data_start + data_len;

                    // Scan for next marker (0xFF followed by non-0xFF, non-0x00)
                    let scan_start = pos;
                    while pos + 1 < bytes.len() {
                        if bytes[pos] == 0xFF && bytes[pos + 1] != 0x00 {
                            // Found next marker
                            break;
                        }
                        pos += 1;
                    }

                    let entropy_len = (pos - scan_start) as u64;
                    if entropy_len > 0 {
                        structure.push(FileBlock {
                            name: "Entropy Data".to_string(),
                            offset: scan_start as u64,
                            length: entropy_len,
                            data_preview: Some(format!(
                                "... {} bytes of entropy-coded data ...",
                                entropy_len
                            )),
                            decoded_info: None,
                            children: vec![],
                        });
                    }
                    continue;
                }

                // For markers with payload, extract the data
                let (data_preview, decoded_info) = if data_len > 0 {
                    let data_start = pos + 4;
                    if data_start + data_len > bytes.len() {
                        errors.push(format!(
                            "Marker {} data extends beyond file at offset {}",
                            name, pos
                        ));
                        break;
                    }
                    let marker_data = &bytes[data_start..data_start + data_len];

                    // Extract EXIF from APP1
                    if marker == APP1 && data_len >= 6 && &marker_data[0..6] == b"Exif\0\0" {
                        let exif_data = &marker_data[6..];
                        match exif_reader::read_exif(exif_data) {
                            Ok(entries) => metadata.extend(entries),
                            Err(e) => errors.push(format!("EXIF extraction error: {}", e)),
                        }
                    }

                    // Extract XMP from APP1
                    if marker == APP1
                        && data_len >= 29
                        && &marker_data[0..29] == b"http://ns.adobe.com/xap/1.0/\0"
                    {
                        let xmp_data = &marker_data[29..];
                        match xmp_reader::read_xmp(xmp_data) {
                            Ok(entries) => metadata.extend(entries),
                            Err(e) => errors.push(format!("XMP extraction error: {}", e)),
                        }
                    }

                    // Extract IPTC from APP13
                    if marker == APP13
                        && data_len >= 14
                        && &marker_data[0..14] == b"Photoshop 3.0\0"
                    {
                        let ps_data = &marker_data[14..];
                        match iptc_reader::read_iptc(ps_data) {
                            Ok(entries) => metadata.extend(entries),
                            Err(e) => errors.push(format!("IPTC extraction error: {}", e)),
                        }
                    }

                    // Extract SOF info
                    if is_sof_marker(marker) {
                        if let Some(info) = parse_sof_data(marker_data) {
                            width = info.width;
                            height = info.height;
                            bit_depth = info.precision;
                            color_type =
                                color_type_from_components(info.num_components, &name).to_string();
                            has_alpha = has_alpha_from_components(info.num_components);

                            // Add component details as children
                            let mut children = Vec::new();
                            for comp in &info.components {
                                let comp_name = match comp.id {
                                    1 => "Y (Luma)",
                                    2 => "Cb (Blue diff)",
                                    3 => "Cr (Red diff)",
                                    4 => "K (Black)",
                                    _ => "Unknown",
                                };
                                children.push(FileBlock {
                                    name: format!("Component {}", comp.id),
                                    offset: 0,
                                    length: 3,
                                    data_preview: None,
                                    decoded_info: Some(format!(
                                        "{}: sampling {}:{}, QT table {}",
                                        comp_name, comp.h_sampling, comp.v_sampling, comp.qt_table
                                    )),
                                    children: vec![],
                                });
                            }

                            structure.push(FileBlock {
                                name: name.clone(),
                                offset: marker_offset,
                                length: total_consumed as u64,
                                data_preview: Some(bytes_to_hex(marker_data, 32)),
                                decoded_info: build_marker_decoded_info(marker, marker_data),
                                children,
                            });

                            pos += total_consumed;
                            continue;
                        }
                    }

                    let preview = bytes_to_hex(marker_data, 32);
                    let decoded = build_marker_decoded_info(marker, marker_data);
                    (Some(preview), decoded)
                } else {
                    (None, Some("No payload".to_string()))
                };

                structure.push(FileBlock {
                    name: name.clone(),
                    offset: marker_offset,
                    length: total_consumed as u64,
                    data_preview,
                    decoded_info,
                    children: vec![],
                });

                pos += total_consumed;

                // Stop at EOI
                if marker == EOI {
                    break;
                }
            }
            Err(e) => {
                errors.push(format!("Parse error at offset {}: {}", pos, e));
                break;
            }
        }
    }

    // If no SOF was found, try to get dimensions from EXIF
    if width == 0 || height == 0 {
        for entry in &metadata {
            if entry.tag_name == "ImageWidth" {
                if let Ok(w) = entry.tag_value.parse::<u32>() {
                    width = w;
                }
            }
            if entry.tag_name == "ImageHeight" {
                if let Ok(h) = entry.tag_value.parse::<u32>() {
                    height = h;
                }
            }
        }
    }

    // Check for trailing data after EOI
    // (already handled by the EOI break, but report if we hit EOF without EOI)
    let last_block = structure.last().map(|b| b.name.clone());
    if last_block.as_deref() != Some("EOI") {
        // We did not find EOI; this might be a truncated JPEG
        errors.push("No EOI marker found (possibly truncated JPEG)".to_string());
    }

    // Build children for APP2 ICC_PROFILE blocks
    merge_icc_blocks(&mut structure);

    Ok(ImageAnalysis {
        file_name,
        file_path: path.to_string(),
        file_size,
        format: ImageFormat::Jpeg,
        width,
        height,
        color_type,
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

/// Merge consecutive APP2 ICC_PROFILE blocks under a single parent
fn merge_icc_blocks(structure: &mut Vec<FileBlock>) {
    let icc_indices: Vec<usize> = structure
        .iter()
        .enumerate()
        .filter(|(_, b)| b.name.contains("APP2") && b.name.contains("ICC"))
        .map(|(i, _)| i)
        .collect();

    if icc_indices.len() <= 1 {
        return;
    }

    // Group consecutive APP2 blocks
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group = vec![icc_indices[0]];
    for i in 1..icc_indices.len() {
        if icc_indices[i] == icc_indices[i - 1] + 1 {
            current_group.push(icc_indices[i]);
        } else {
            groups.push(std::mem::take(&mut current_group));
            current_group = vec![icc_indices[i]];
        }
    }
    if !current_group.is_empty() {
        groups.push(current_group);
    }

    // Merge each group (reverse order so indices stay valid)
    for group in groups.iter().rev() {
        if group.len() < 2 {
            continue;
        }
        let first = group[0];
        let total_length: u64 = group.iter().map(|&i| structure[i].length).sum();
        let first_offset = structure[first].offset;

        let children: Vec<FileBlock> = group
            .iter()
            .map(|&i| {
                let block = &structure[i];
                FileBlock {
                    name: block.name.clone(),
                    offset: block.offset,
                    length: block.length,
                    data_preview: block.data_preview.clone(),
                    decoded_info: block.decoded_info.clone(),
                    children: vec![],
                }
            })
            .collect();

        let merged = FileBlock {
            name: "ICC_PROFILE".to_string(),
            offset: first_offset,
            length: total_length,
            data_preview: Some(format!(
                "{} parts, {} bytes total",
                children.len(),
                total_length
            )),
            decoded_info: Some(format!("ICC Profile: {} parts", children.len())),
            children,
        };

        // Replace first with merged, mark rest for removal
        structure[first] = merged;
        // Remove the other blocks (in reverse to preserve indices)
        for &i in group.iter().skip(1).rev() {
            structure.remove(i);
        }
    }
}
