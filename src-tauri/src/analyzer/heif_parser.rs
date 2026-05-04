use crate::types::{
    CodecSyntax, FileBlock, GridInfo, GridTile, ImageAnalysis, ImageFormat, MetadataEntry,
};
use crate::utils::read_file_bytes;
use std::collections::HashMap;

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

/// A property found in ipco, indexed by 1-based index.
#[derive(Debug, Clone)]
enum HeifProperty {
    Ispe {
        width: u32,
        height: u32,
    },
    Pixi {
        channels: Vec<u8>,
    },
    ColrNclx {
        color_primaries: u16,
        transfer_characteristics: u16,
        matrix_coefficients: u16,
        full_range: bool,
    },
    ColrIcc {
        profile_data: Vec<u8>,
    },
    HvcC {
        bit_depth_luma: u8,
        bit_depth_chroma: u8,
        chroma_format: u8,
        profile_idc: u8,
        level_idc: u8,
        raw_data: Vec<u8>,
    },
    Av1C {
        raw_data: Vec<u8>,
    },
    Irot {
        anticlockwise_rotation: u8,
    },
    Clap {
        width_n: u32,
        width_d: u32,
        height_n: u32,
        height_d: u32,
        horiz_offset_n: i32,
        horiz_offset_d: u32,
        vert_offset_n: i32,
        vert_offset_d: u32,
    },
    AuxC {
        aux_type: String,
    },
    Other {
        box_type: String,
        size: usize,
    },
}

/// An item from iinf/infe.
#[derive(Debug, Clone)]
struct HeifItem {
    item_id: u32,
    item_type: String,
    item_name: String,
    content_type: String,
    hidden: bool,
}

/// Reference from iref.
#[derive(Debug, Clone)]
struct HeifRef {
    from_item_id: u32,
    ref_type: String,
    to_item_ids: Vec<u32>,
}

/// Parsed HEIF/HEIC data using libheif-style item/property model.
struct HeifParseResult {
    items: Vec<HeifItem>,
    primary_item_id: u32,
    /// ipco properties by 1-based index
    properties: Vec<HeifProperty>,
    /// ipma: item_id -> list of (property_index, essential)
    item_props: HashMap<u32, Vec<(usize, bool)>>,
    /// iref references
    refs: Vec<HeifRef>,
    /// Grid info if present
    grid: Option<GridInfo>,
    /// Structure tree nodes
    structure: Vec<FileBlock>,
    /// ICC profile data (from primary image's colr)
    icc_data: Option<Vec<u8>>,
    errors: Vec<String>,
}

fn grid_tile_codec(items: &[HeifItem], item_id: u32) -> String {
    items.iter()
        .find(|it| it.item_id == item_id)
        .map(|it| {
            if it.item_type.contains("hvc") || it.item_type.contains("hev") {
                "HEVC".to_string()
            } else if it.item_type.contains("av0") {
                "AV1".to_string()
            } else {
                it.item_type.clone()
            }
        })
        .unwrap_or_default()
}

fn build_grid_tiles(
    grid_tile_ids: &[u32],
    items: &[HeifItem],
    rows: u32,
    cols: u32,
    output_width: u32,
    output_height: u32,
) -> Vec<GridTile> {
    grid_tile_ids
        .iter()
        .enumerate()
        .map(|(index, &id)| {
            let row = (index as u32) / cols.max(1);
            let col = (index as u32) % cols.max(1);

            let x0 = output_width.saturating_mul(col) / cols.max(1);
            let x1 = output_width.saturating_mul(col + 1) / cols.max(1);
            let y0 = output_height.saturating_mul(row) / rows.max(1);
            let y1 = output_height.saturating_mul(row + 1) / rows.max(1);

            GridTile {
                item_id: id as u16,
                width: x1.saturating_sub(x0),
                height: y1.saturating_sub(y0),
                horizontal_offset: x0,
                vertical_offset: y0,
                codec: grid_tile_codec(items, id),
            }
        })
        .collect()
}

fn read_box_header(bytes: &[u8], offset: usize) -> Option<(usize, String, usize)> {
    if offset + 8 > bytes.len() {
        return None;
    }
    let size_raw = u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]);
    let (size, hdr_extra) = if size_raw == 1 {
        if offset + 16 > bytes.len() {
            return None;
        }
        let ext = u64::from_be_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
            bytes[offset + 12],
            bytes[offset + 13],
            bytes[offset + 14],
            bytes[offset + 15],
        ]);
        (ext as usize, 8)
    } else if size_raw == 0 {
        return None; // extends to EOF - handle if needed
    } else {
        (size_raw as usize, 0)
    };
    if size < 8 + hdr_extra {
        return None;
    }
    let box_type = String::from_utf8_lossy(&bytes[offset + 4..offset + 8]).to_string();
    Some((size, box_type, hdr_extra))
}

fn parse_heif_properties(ipco_data: &[u8]) -> Vec<HeifProperty> {
    let mut props = Vec::new();
    let mut offset = 0;
    while offset + 8 <= ipco_data.len() {
        let Some((size, box_type, _)) = read_box_header(ipco_data, offset) else {
            break;
        };
        let data_start = offset + 8;
        let data_end = offset + size;
        if data_end > ipco_data.len() || size < 8 {
            break;
        }

        let prop = match box_type.as_str() {
            "ispe" => {
                // FullBox: version(1)+flags(3) + width(4) + height(4)
                if data_start + 12 <= data_end {
                    let w = u32::from_be_bytes([
                        ipco_data[data_start + 4],
                        ipco_data[data_start + 5],
                        ipco_data[data_start + 6],
                        ipco_data[data_start + 7],
                    ]);
                    let h = u32::from_be_bytes([
                        ipco_data[data_start + 8],
                        ipco_data[data_start + 9],
                        ipco_data[data_start + 10],
                        ipco_data[data_start + 11],
                    ]);
                    HeifProperty::Ispe {
                        width: w,
                        height: h,
                    }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            "pixi" => {
                // FullBox + num_channels(1) + channel_depths
                if data_start + 5 <= data_end {
                    let nc = ipco_data[data_start + 4];
                    let channels: Vec<u8> =
                        ipco_data[data_start + 5..data_start + 5 + nc as usize].to_vec();
                    HeifProperty::Pixi { channels }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            "colr" => {
                if data_start + 4 <= data_end {
                    let ctype = &ipco_data[data_start..data_start + 4];
                    if ctype == b"nclx" {
                        // nclx: color_primaries(2) + transfer_characteristics(2)
                        //   + matrix_coefficients(2) + full_range_flag(1 bit)
                        if data_start + 10 <= data_end {
                            let cp = u16::from_be_bytes([
                                ipco_data[data_start + 4],
                                ipco_data[data_start + 5],
                            ]);
                            let tc = u16::from_be_bytes([
                                ipco_data[data_start + 6],
                                ipco_data[data_start + 7],
                            ]);
                            let mc = u16::from_be_bytes([
                                ipco_data[data_start + 8],
                                ipco_data[data_start + 9],
                            ]);
                            let fr = if data_start + 10 <= data_end {
                                (ipco_data[data_start + 10] & 0x80) != 0
                            } else {
                                false
                            };
                            HeifProperty::ColrNclx {
                                color_primaries: cp,
                                transfer_characteristics: tc,
                                matrix_coefficients: mc,
                                full_range: fr,
                            }
                        } else {
                            HeifProperty::Other {
                                box_type,
                                size: size - 8,
                            }
                        }
                    } else if ctype == b"prof" || ctype == b"rICC" {
                        let profile_data = ipco_data[data_start + 4..data_end].to_vec();
                        HeifProperty::ColrIcc { profile_data }
                    } else {
                        HeifProperty::Other {
                            box_type,
                            size: size - 8,
                        }
                    }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            "hvcC" => {
                if data_start + 23 <= data_end {
                    let config = &ipco_data[data_start..data_end];
                    let _config_version = config[0];
                    let profile_byte = config[1];
                    let _general_profile_space = (profile_byte >> 6) & 0x03;
                    let _general_tier_flag = (profile_byte >> 5) & 0x01;
                    let general_profile_idc = profile_byte & 0x1F;
                    let general_level_idc = config[12];
                    let chroma_format = config[16] & 0x03;
                    let bit_depth_luma = (config[17] & 0x07) + 8;
                    let bit_depth_chroma = (config[18] & 0x07) + 8;

                    HeifProperty::HvcC {
                        bit_depth_luma,
                        bit_depth_chroma,
                        chroma_format,
                        profile_idc: general_profile_idc,
                        level_idc: general_level_idc,
                        raw_data: ipco_data[offset..data_end].to_vec(),
                    }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            "av1C" => {
                HeifProperty::Av1C {
                    raw_data: ipco_data[offset..data_end].to_vec(),
                }
            }
            "irot" => {
                if data_start < data_end {
                    HeifProperty::Irot {
                        anticlockwise_rotation: ipco_data[data_start] & 0x03,
                    }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            "clap" => {
                if data_start + 32 <= data_end {
                    let width_n = u32::from_be_bytes([
                        ipco_data[data_start],
                        ipco_data[data_start + 1],
                        ipco_data[data_start + 2],
                        ipco_data[data_start + 3],
                    ]);
                    let width_d = u32::from_be_bytes([
                        ipco_data[data_start + 4],
                        ipco_data[data_start + 5],
                        ipco_data[data_start + 6],
                        ipco_data[data_start + 7],
                    ]);
                    let height_n = u32::from_be_bytes([
                        ipco_data[data_start + 8],
                        ipco_data[data_start + 9],
                        ipco_data[data_start + 10],
                        ipco_data[data_start + 11],
                    ]);
                    let height_d = u32::from_be_bytes([
                        ipco_data[data_start + 12],
                        ipco_data[data_start + 13],
                        ipco_data[data_start + 14],
                        ipco_data[data_start + 15],
                    ]);
                    let horiz_offset_n = i32::from_be_bytes([
                        ipco_data[data_start + 16],
                        ipco_data[data_start + 17],
                        ipco_data[data_start + 18],
                        ipco_data[data_start + 19],
                    ]);
                    let horiz_offset_d = u32::from_be_bytes([
                        ipco_data[data_start + 20],
                        ipco_data[data_start + 21],
                        ipco_data[data_start + 22],
                        ipco_data[data_start + 23],
                    ]);
                    let vert_offset_n = i32::from_be_bytes([
                        ipco_data[data_start + 24],
                        ipco_data[data_start + 25],
                        ipco_data[data_start + 26],
                        ipco_data[data_start + 27],
                    ]);
                    let vert_offset_d = u32::from_be_bytes([
                        ipco_data[data_start + 28],
                        ipco_data[data_start + 29],
                        ipco_data[data_start + 30],
                        ipco_data[data_start + 31],
                    ]);
                    HeifProperty::Clap {
                        width_n,
                        width_d,
                        height_n,
                        height_d,
                        horiz_offset_n,
                        horiz_offset_d,
                        vert_offset_n,
                        vert_offset_d,
                    }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            "auxC" => {
                // FullBox + aux_type (null-terminated URI)
                if data_start + 5 <= data_end {
                    let type_start = data_start + 4;
                    let mut end = type_start;
                    while end < data_end && ipco_data[end] != 0 {
                        end += 1;
                    }
                    let aux_type = String::from_utf8_lossy(&ipco_data[type_start..end]).to_string();
                    HeifProperty::AuxC { aux_type }
                } else {
                    HeifProperty::Other {
                        box_type,
                        size: size - 8,
                    }
                }
            }
            _ => HeifProperty::Other {
                box_type,
                size: size - 8,
            },
        };
        props.push(prop);
        offset += size;
    }
    props
}

fn parse_ipma(bytes: &[u8]) -> HashMap<u32, Vec<(usize, bool)>> {
    let mut map = HashMap::new();
    if bytes.len() < 8 {
        return map;
    }
    let version = bytes[0];
    // entry_count can be u8 (standard) or u32 (some implementations)
    // Try u32 first; if it's 0, fall back to u8
    let entry_count_u32 = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let entry_count = if entry_count_u32 == 0 {
        bytes[4] as usize
    } else {
        entry_count_u32
    };
    let mut pos = if entry_count_u32 != 0 { 8 } else { 5 };

    for _ in 0..entry_count {
        if pos + 3 > bytes.len() {
            break;
        }
        let item_id = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
        pos += 2;
        let assoc_count = bytes[pos] as usize;
        pos += 1;
        let mut props = Vec::new();
        for _ in 0..assoc_count {
            if version >= 1 {
                if pos + 2 > bytes.len() {
                    break;
                }
                let entry = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
                pos += 2;
                let essential = (entry & 0x8000) != 0;
                let prop_index = (entry & 0x7FFF) as usize;
                props.push((prop_index, essential));
            } else {
                if pos + 1 > bytes.len() {
                    break;
                }
                let entry = bytes[pos];
                pos += 1;
                let essential = (entry & 0x80) != 0;
                let prop_index = (entry & 0x7F) as usize;
                props.push((prop_index, essential));
            }
        }
        if !props.is_empty() {
            map.insert(item_id as u32, props);
        }
    }
    map
}

fn parse_iprp(bytes: &[u8]) -> (Vec<HeifProperty>, HashMap<u32, Vec<(usize, bool)>>) {
    // iprp contains ipco (properties) and ipma (associations)
    let mut ipco_data: Option<&[u8]> = None;
    let mut ipma_data: Option<&[u8]> = None;
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let Some((size, box_type, _)) = read_box_header(bytes, offset) else {
            break;
        };
        let data_start = offset + 8;
        match box_type.as_str() {
            "ipco" => {
                ipco_data = Some(&bytes[data_start..offset + size]);
            }
            "ipma" => {
                // ipma is a FullBox, data starts at offset + 8 + 4
                ipma_data = Some(&bytes[offset + 8..offset + size]);
            }
            _ => {}
        }
        offset += size;
    }

    let properties = ipco_data
        .map(|d| parse_heif_properties(d))
        .unwrap_or_default();
    let item_props = ipma_data.map(|d| parse_ipma(d)).unwrap_or_default();
    (properties, item_props)
}

#[derive(Debug)]
struct IlocEntry {
    item_id: u32,
    offset: u64,
    length: u64,
}

fn extract_item_payload<'a>(file_bytes: &'a [u8], iloc_entries: &[IlocEntry], item_id: u32) -> Option<&'a [u8]> {
    let entry = iloc_entries.iter().find(|e| e.item_id == item_id)?;
    let start = entry.offset as usize;
    let end = start.checked_add(entry.length as usize)?;
    if start >= end || end > file_bytes.len() {
        return None;
    }
    Some(&file_bytes[start..end])
}

fn parse_iloc_entries(bytes: &[u8]) -> Vec<IlocEntry> {
    let mut entries = Vec::new();
    if bytes.len() < 8 {
        return entries;
    }
    let version = bytes[0];
    let offset_size_field = (bytes[4] >> 4) & 0x0F;
    let length_size_field = bytes[4] & 0x0F;
    let base_offset_size = (bytes[5] >> 4) & 0x0F;
    let index_size = bytes[5] & 0x0F;
    let item_count = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;

    let offset_size = offset_size_field as usize;
    let length_size = length_size_field as usize;

    let mut ip = 8usize;
    for _ in 0..item_count {
        if ip + 2 > bytes.len() {
            break;
        }
        let item_id = if version < 2 {
            let id = u16::from_be_bytes([bytes[ip], bytes[ip + 1]]);
            ip += 2;
            id as u32
        } else {
            let id = u32::from_be_bytes([bytes[ip], bytes[ip + 1], bytes[ip + 2], bytes[ip + 3]]);
            ip += 4;
            id
        };

        if version == 1 || version == 2 {
            if ip + 2 > bytes.len() {
                break;
            }
            ip += 2; // construction_method with reserved bits
        }
        if ip + 2 > bytes.len() {
            break;
        }
        ip += 2; // data_reference_index

        let mut base_offset: u64 = 0;
        if base_offset_size > 0 {
            let size = base_offset_size as usize;
            if ip + size > bytes.len() {
                break;
            }
            match base_offset_size {
                1 => base_offset = bytes[ip] as u64,
                2 => base_offset = u16::from_be_bytes([bytes[ip], bytes[ip + 1]]) as u64,
                4 => {
                    base_offset =
                        u32::from_be_bytes([bytes[ip], bytes[ip + 1], bytes[ip + 2], bytes[ip + 3]])
                            as u64
                }
                8 => {
                    base_offset = u64::from_be_bytes([
                        bytes[ip],
                        bytes[ip + 1],
                        bytes[ip + 2],
                        bytes[ip + 3],
                        bytes[ip + 4],
                        bytes[ip + 5],
                        bytes[ip + 6],
                        bytes[ip + 7],
                    ])
                }
                _ => {}
            }
            ip += size;
        }

        if ip + 2 > bytes.len() {
            break;
        }
        let extent_count = u16::from_be_bytes([bytes[ip], bytes[ip + 1]]) as usize;
        ip += 2;

        let mut total_length: u64 = 0;
        let mut first_extent_offset: u64 = 0;
        for e in 0..extent_count {
            if (version == 1 || version == 2) && index_size > 0 {
                let size = index_size as usize;
                if ip + size > bytes.len() {
                    break;
                }
                ip += size;
            }
            let extent_offset = if offset_size > 0 {
                if ip + offset_size > bytes.len() {
                    break;
                }
                let offset = match offset_size {
                    1 => bytes[ip] as u64,
                    2 => u16::from_be_bytes([bytes[ip], bytes[ip + 1]]) as u64,
                    4 => {
                        u32::from_be_bytes([bytes[ip], bytes[ip + 1], bytes[ip + 2], bytes[ip + 3]])
                            as u64
                    }
                    8 => u64::from_be_bytes([
                        bytes[ip],
                        bytes[ip + 1],
                        bytes[ip + 2],
                        bytes[ip + 3],
                        bytes[ip + 4],
                        bytes[ip + 5],
                        bytes[ip + 6],
                        bytes[ip + 7],
                    ]),
                    _ => 0,
                };
                ip += offset_size;
                offset
            } else {
                0
            };
            if e == 0 {
                first_extent_offset = extent_offset;
            }
            let extent_length = if length_size > 0 {
                if ip + length_size > bytes.len() {
                    break;
                }
                let len = match length_size {
                    1 => bytes[ip] as u64,
                    2 => u16::from_be_bytes([bytes[ip], bytes[ip + 1]]) as u64,
                    4 => {
                        u32::from_be_bytes([bytes[ip], bytes[ip + 1], bytes[ip + 2], bytes[ip + 3]])
                            as u64
                    }
                    8 => u64::from_be_bytes([
                        bytes[ip],
                        bytes[ip + 1],
                        bytes[ip + 2],
                        bytes[ip + 3],
                        bytes[ip + 4],
                        bytes[ip + 5],
                        bytes[ip + 6],
                        bytes[ip + 7],
                    ]),
                    _ => 0,
                };
                ip += length_size;
                len
            } else {
                0
            };
            total_length += extent_length;
        }

        let file_offset = base_offset + first_extent_offset;
        entries.push(IlocEntry {
            item_id,
            offset: file_offset,
            length: if total_length > 0 { total_length } else { 0 },
        });
    }
    entries
}

fn parse_iinf(bytes: &[u8], iloc_ids: &[u32]) -> Vec<HeifItem> {
    let mut items = Vec::new();
    if bytes.len() < 8 {
        return items;
    }
    let version = bytes[0];
    let (entry_count, mut pos) = if version == 0 {
        if bytes.len() < 6 {
            return items;
        }
        (u16::from_be_bytes([bytes[4], bytes[5]]) as usize, 6usize)
    } else {
        if bytes.len() < 8 {
            return items;
        }
        (
            u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize,
            8usize,
        )
    };
    let mut entry_idx = 0;

    for _ in 0..entry_count {
        let Some((infe_size, infe_type, _)) = read_box_header(bytes, pos) else {
            break;
        };
        if infe_type != "infe" {
            break;
        }
        let infe_data_start = pos + 8;
        let infe_end = pos + infe_size;
        if infe_end > bytes.len() || infe_data_start + 4 > infe_end {
            break;
        }

        // Get item_id from iloc (matching by entry index)
        let item_id = iloc_ids.get(entry_idx).copied().unwrap_or(0);
        entry_idx += 1;

        let infe_ver = bytes[infe_data_start];
        let p = infe_data_start + 4; // payload start (after FullBox header)

        let (item_type, item_name, content_type, hidden) = if infe_ver == 0 {
            let pi = u16::from_be_bytes([bytes[p], bytes[p + 1]]);
            let name_start = p + 2; // no item_ID in v0
            let mut ne = name_start;
            while ne < infe_end && bytes[ne] != 0 {
                ne += 1;
            }
            let item_name = String::from_utf8_lossy(&bytes[name_start..ne]).to_string();
            let ct_start = ne + 1;
            let mut cte = ct_start;
            while cte < infe_end && bytes[cte] != 0 {
                cte += 1;
            }
            let content_type = String::from_utf8_lossy(&bytes[ct_start..cte]).to_string();
            let hidden = (pi & 0x01) != 0;
            (String::new(), item_name, content_type, hidden)
        } else if infe_ver == 2 {
            // version 2: protection_index(2) + item_ID(2) + item_type(4) + item_name(Z) + content_type(Z)
            // We get item_id from iloc, skip item_ID field in payload
            let _protection_index = u16::from_be_bytes([bytes[p], bytes[p + 1]]);
            let item_type = String::from_utf8_lossy(&bytes[p + 4..p + 8]).to_string();
            let name_start = p + 8;
            let mut ne = name_start;
            while ne < infe_end && bytes[ne] != 0 {
                ne += 1;
            }
            let item_name = String::from_utf8_lossy(&bytes[name_start..ne]).to_string();
            let ct_start = ne + 1;
            let mut cte = ct_start;
            while cte < infe_end && bytes[cte] != 0 {
                cte += 1;
            }
            let content_type = String::from_utf8_lossy(&bytes[ct_start..cte]).to_string();
            let hidden = (_protection_index & 0x01) != 0;
            (item_type, item_name, content_type, hidden)
        } else {
            ("".to_string(), String::new(), String::new(), false)
        };

        items.push(HeifItem {
            item_id,
            item_type,
            item_name,
            content_type,
            hidden,
        });
        pos += infe_size;
    }
    items
}

fn parse_iref(bytes: &[u8]) -> Vec<HeifRef> {
    let mut refs = Vec::new();
    // Input: iref content with iref FullBox header stripped (starts at first child box).
    // Each child is a SingleItemTypeReferenceBox FullBox:
    //   size(4) + type(4) + version(1) + flags(3) + from_item_id(2) + reference_count(2) + to_item_ids
    // In practice, many implementations treat the version/flags as part of the payload area.
    // The actual payload (from_id, ref_count, to_ids) starts at offset+8+4 = offset+12.
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let Some((size, box_type, _hdr_extra)) = read_box_header(bytes, offset) else {
            break;
        };
        if size < 16 {
            offset += size.max(8);
            continue;
        }
        let payload = offset + 8; // box header (8) = start of FullBox payload (version/flags + from_id + ref_count + to_ids)
        let data_end = offset + size;

        if payload + 4 <= data_end {}

        let from_id = u16::from_be_bytes([bytes[payload], bytes[payload + 1]]) as u32;
        let ref_count = u16::from_be_bytes([bytes[payload + 2], bytes[payload + 3]]) as usize;
        let mut to_ids = Vec::new();
        let mut pos = payload + 4;
        for _ in 0..ref_count {
            if pos + 2 > data_end {
                break;
            }
            let to_id = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as u32;
            to_ids.push(to_id);
            pos += 2;
        }
        refs.push(HeifRef {
            from_item_id: from_id,
            ref_type: box_type,
            to_item_ids: to_ids,
        });
        offset += size;
    }
    refs
}

fn item_type_is_image(item_type: &str, content_type: &str) -> bool {
    matches!(
        item_type,
        "hvc1" | "grid" | "iden" | "iovl" | "av01" | "unci" | "vvc1" | "jpeg" | "j2k1" | "mski"
    ) || (item_type == "mime" && content_type == "image/jpeg")
}

fn parse_heif_structure(bytes: &[u8]) -> (Vec<FileBlock>, Vec<String>) {
    let mut structure = Vec::new();
    let mut errors = Vec::new();
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let Some((size, box_type, _hdr_extra)) = read_box_header(bytes, offset) else {
            break;
        };
        if size < 8 {
            break;
        }
        let data_end = offset + size;
        if data_end > bytes.len() {
            errors.push(format!(
                "Box '{}' at offset {} extends beyond file",
                box_type, offset
            ));
            break;
        }

        let mut fields = Vec::new();
        let children = parse_heif_box(&box_type, &bytes[offset..data_end], offset, &mut fields);

        if box_type == "meta" {
            // Keep meta as a wrapper node with its children nested inside
            structure.push(FileBlock {
                name: box_type.to_string(),
                offset: offset as u64,
                length: size as u64,
                fields,
                decoded_info: None,
                data_preview: None,
                children,
            });
        } else {
            structure.push(FileBlock {
                name: box_type,
                offset: offset as u64,
                length: size as u64,
                fields,
                decoded_info: None,
                data_preview: None,
                children,
            });
        }

        offset += size;
    }
    (structure, errors)
}

/// Recursively parse a box and its children, populating `fields` with semantic data.
/// Returns the list of child FileBlocks.
fn parse_heif_box(
    box_type: &str,
    box_bytes: &[u8],
    global_offset: usize,
    fields: &mut Vec<(String, String)>,
) -> Vec<FileBlock> {
    let size = box_bytes.len();
    if size < 8 {
        return Vec::new();
    }

    // Read local box header
    let size_raw = u32::from_be_bytes([box_bytes[0], box_bytes[1], box_bytes[2], box_bytes[3]]);
    let (payload_start, _hdr_size, _box_size) = if size_raw == 1 && size >= 16 {
        let ext = u64::from_be_bytes([
            box_bytes[8],
            box_bytes[9],
            box_bytes[10],
            box_bytes[11],
            box_bytes[12],
            box_bytes[13],
            box_bytes[14],
            box_bytes[15],
        ]);
        (16, 16, ext as usize)
    } else {
        (8, 8, size_raw as usize)
    };

    // Decode box-specific fields
    decode_box_fields(box_type, &box_bytes[payload_start..], fields);

    // Determine child offset and whether this box is a container
    let (child_start, is_container) = match box_type {
        "meta" => (payload_start + 4, true), // FullBox: skip version+flags
        "iinf" => (payload_start + 4, true), // FullBox: skip version+flags (entry_count handled in special case)
        "ipco" => (payload_start, true),     // plain container
        "ipma" => (payload_start + 4, true), // FullBox
        "iref" => (payload_start + 4, true), // FullBox
        "iprp" => (payload_start, true),     // plain container
        "dinf" => (payload_start, true),     // plain container
        "dref" => (payload_start + 4, true), // FullBox
        "iloc" => (payload_start + 4, true), // FullBox
        "hvcC" => (payload_start + 4, true), // FullBox (arrays inside)
        "av1C" => (payload_start, true),     // plain container
        _ => (payload_start, false),
    };

    if !is_container {
        return Vec::new();
    }

    let child_end = size;
    let mut children = Vec::new();
    let mut co = child_start;

    // Special case: iinf children are infe boxes
    if box_type == "iinf" {
        // entry_count is at payload_start+4..+6, infe boxes start after it
        let iinf_data_start = payload_start + 6;
        let mut co = iinf_data_start;
        while co + 8 <= child_end {
            if let Some((csize, ctype, _)) = read_box_header(box_bytes, co) {
                if csize < 8 || co + csize > child_end {
                    break;
                }
                let cbox_data = &box_bytes[co..co + csize];
                let mut cfields = Vec::new();
                let cchildren = parse_heif_box(&ctype, cbox_data, global_offset + co, &mut cfields);
                children.push(FileBlock {
                    name: ctype,
                    offset: (global_offset + co) as u64,
                    length: csize as u64,
                    fields: cfields,
                    decoded_info: None,
                    data_preview: None,
                    children: cchildren,
                });
                co += csize;
            } else {
                break;
            }
        }
        return children;
    }

    // Special case: iloc - parse entries (not child boxes)
    if box_type == "iloc" {
        parse_iloc_entries_to_fields(box_bytes, payload_start, global_offset, fields);
        return Vec::new();
    }

    // Special case: ipma - parse associations
    if box_type == "ipma" {
        parse_ipma_entries(box_bytes, payload_start, global_offset, fields);
        return Vec::new();
    }

    // Special case: iref - parse reference boxes
    if box_type == "iref" {
        return parse_iref_children(box_bytes, child_start, global_offset, child_end);
    }

    // Special case: ipco - assign 1-based property indexes
    if box_type == "ipco" {
        let mut children = Vec::new();
        let mut prop_index = 1u32;
        let mut co = child_start;
        while co + 8 <= child_end {
            let Some((csize, ctype, _)) = read_box_header(box_bytes, co) else {
                break;
            };
            if csize < 8 || co + csize > child_end {
                break;
            }
            let cbox_data = &box_bytes[co..co + csize];
            let mut cfields = Vec::new();
            // Add property index label like heif-info
            cfields.push(("index".into(), prop_index.to_string()));
            let cchildren = parse_heif_box(&ctype, cbox_data, global_offset + co, &mut cfields);
            children.push(FileBlock {
                name: ctype,
                offset: (global_offset + co) as u64,
                length: csize as u64,
                fields: cfields,
                decoded_info: None,
                data_preview: None,
                children: cchildren,
            });
            prop_index += 1;
            co += csize;
        }
        return children;
    }

    // General case: walk child boxes
    while co + 8 <= child_end {
        let Some((csize, ctype, _)) = read_box_header(box_bytes, co) else {
            break;
        };
        if csize < 8 || co + csize > child_end {
            break;
        }
        let cbox_data = &box_bytes[co..co + csize];
        let mut cfields = Vec::new();
        let cchildren = parse_heif_box(&ctype, cbox_data, global_offset + co, &mut cfields);
        children.push(FileBlock {
            name: ctype,
            offset: (global_offset + co) as u64,
            length: csize as u64,
            fields: cfields,
            decoded_info: None,
            data_preview: None,
            children: cchildren,
        });
        co += csize;
    }
    children
}

/// Decode semantic fields for a specific box type.
fn decode_box_fields(box_type: &str, payload: &[u8], fields: &mut Vec<(String, String)>) {
    match box_type {
        "ftyp" => {
            if payload.len() >= 8 {
                let major = String::from_utf8_lossy(&payload[0..4]).to_string();
                let minor = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                fields.push(("major brand".into(), major));
                fields.push(("minor version".into(), minor.to_string()));
                if payload.len() > 8 {
                    let compat: Vec<_> = payload[8..]
                        .chunks(4)
                        .map(|c| String::from_utf8_lossy(c).to_string())
                        .collect();
                    fields.push(("compatible brands".into(), compat.join(", ")));
                }
            }
        }
        "meta" => {}
        "hdlr" => {
            if payload.len() >= 12 {
                // FullBox version+flags at 0..4, pre_defined at 4..8, handler_type at 8..12
                let pre_defined =
                    u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let handler = String::from_utf8_lossy(&payload[8..12]).to_string();
                fields.push(("pre_defined".into(), pre_defined.to_string()));
                fields.push(("handler_type".into(), handler.trim().to_string()));
                if payload.len() > 12 {
                    let name = String::from_utf8_lossy(&payload[12..]).trim().to_string();
                    fields.push(("name".into(), name));
                }
            }
        }
        "dinf" => {}
        "dref" => {
            if payload.len() >= 4 {
                let entry_count =
                    u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                fields.push(("entry count".into(), entry_count.to_string()));
            }
        }
        "url " => {
            if payload.len() >= 1 {
                let version = payload[0]; // FullBox version byte
                if version == 0 && payload.len() > 4 {
                    let location = String::from_utf8_lossy(&payload[4..]).trim().to_string();
                    fields.push(("location".into(), location));
                } else {
                    fields.push(("version".into(), version.to_string()));
                }
            }
        }
        "pitm" => {
            if payload.len() >= 2 {
                let item_id = u16::from_be_bytes([payload[0], payload[1]]);
                fields.push(("item_ID".into(), item_id.to_string()));
            }
        }
        "iinf" => {
            // entry_count decoded by parent
        }
        "infe" => {
            if payload.len() < 3 {
                return;
            }
            let version = payload[0];
            if version == 0 || version == 1 {
                // v0/v1: version(1) + flags(3) + protection_index(2) + item_name(Z) [+ content_type(Z) for v1]
                let protection_index = u16::from_be_bytes([payload[1], payload[2]]);
                fields.push(("item_protection_index".into(), protection_index.to_string()));
                let hidden = (protection_index & 0x01) != 0;
                fields.push(("hidden item".into(), hidden.to_string()));
                let mut pos = 3;
                if pos < payload.len() {
                    let name_end = payload[pos..]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(payload.len() - pos);
                    let item_name =
                        String::from_utf8_lossy(&payload[pos..pos + name_end]).to_string();
                    fields.push(("item_name".into(), item_name));
                    pos += name_end + 1;
                }
                if version == 1 && pos < payload.len() {
                    let ct_end = payload[pos..]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(payload.len() - pos);
                    let content_type =
                        String::from_utf8_lossy(&payload[pos..pos + ct_end]).to_string();
                    fields.push(("content_type".into(), content_type));
                }
            } else if version == 2 || version == 3 {
                // ISO 23008-12 infe v2/v3: FullBox header(4) + item_ID(2) + protection_index(2) + item_type(4) + item_name(Z) [+ content_type(Z) for v3]
                if payload.len() < 12 {
                    return;
                }
                let item_id = u16::from_be_bytes([payload[4], payload[5]]);
                fields.push(("item_ID".into(), item_id.to_string()));
                let protection_index = u16::from_be_bytes([payload[6], payload[7]]);
                fields.push(("item_protection_index".into(), protection_index.to_string()));
                let hidden = (protection_index & 0x01) != 0;
                fields.push(("hidden item".into(), hidden.to_string()));
                let item_type = String::from_utf8_lossy(&payload[8..12]).to_string();
                fields.push(("item_type".into(), item_type));
                let mut pos = 12;
                if pos < payload.len() {
                    let name_end = payload[pos..]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(payload.len() - pos);
                    let item_name =
                        String::from_utf8_lossy(&payload[pos..pos + name_end]).to_string();
                    fields.push(("item_name".into(), item_name));
                    pos += name_end + 1;
                }
                if version == 3 && pos < payload.len() {
                    let ct_end = payload[pos..]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(payload.len() - pos);
                    let content_type =
                        String::from_utf8_lossy(&payload[pos..pos + ct_end]).to_string();
                    fields.push(("content_type".into(), content_type));
                }
            }
        }
        "iref" => {
            // parsed by parse_iref_children
        }
        "iprp" => {}
        "ipco" => {}
        "ipma" => {
            // parsed by parse_ipma_entries
        }
        "colr" => {
            if payload.len() >= 4 {
                let ctype = String::from_utf8_lossy(&payload[0..4]).to_string();
                fields.push(("colour_type".into(), ctype.clone()));
                if ctype == "nclx" && payload.len() >= 11 {
                    let cp = u16::from_be_bytes([payload[4], payload[5]]);
                    let tc = u16::from_be_bytes([payload[6], payload[7]]);
                    let mc = u16::from_be_bytes([payload[8], payload[9]]);
                    let full_range = (payload[10] >> 7) != 0;
                    fields.push(("colour_primaries".into(), cp.to_string()));
                    fields.push(("transfer_characteristics".into(), tc.to_string()));
                    fields.push(("matrix_coefficients".into(), mc.to_string()));
                    fields.push(("full_range".into(), full_range.to_string()));
                } else if ctype == "prof" || ctype == "rICC" {
                    let prof_size = payload.len() - 4;
                    fields.push(("profile size".into(), prof_size.to_string()));
                }
            }
        }
        "hvcC" => {
            if payload.len() < 23 {
                return;
            }
            let p = payload;
            fields.push(("configuration_version".into(), p[0].to_string()));
            let byte1 = p[1];
            let profile_space = (byte1 >> 6) & 0x03;
            let tier = (byte1 >> 5) & 0x01;
            let profile_idc = byte1 & 0x1F;
            fields.push(("general_profile_space".into(), profile_space.to_string()));
            fields.push(("general_tier_flag".into(), tier.to_string()));
            fields.push(("general_profile_idc".into(), profile_idc.to_string()));
            let compat = &p[2..6];
            let compat_hex: Vec<_> = compat
                .chunks(2)
                .map(|b| format!("{:02x}{:02x}", b[0], b[1]))
                .collect();
            fields.push((
                "general_profile_compatibility_flags".into(),
                compat_hex.join(". "),
            ));
            let constraint = &p[6..12];
            let constraint_hex: Vec<_> = constraint
                .chunks(2)
                .map(|b| format!("{:08b}", b[0]))
                .collect();
            fields.push((
                "general_constraint_indicator_flags".into(),
                constraint_hex.join(" "),
            ));
            fields.push(("general_level_idc".into(), p[12].to_string()));
            let min_ssm = u16::from_be_bytes([p[14] & 0x0F, p[15]]);
            fields.push(("min_spatial_segmentation_idc".into(), min_ssm.to_string()));
            fields.push(("parallelism_type".into(), (p[16] & 0x03).to_string()));
            fields.push(("chroma_format".into(), (p[16] & 0x03).to_string()));
            fields.push(("bit_depth_luma".into(), ((p[17] & 0x07) + 8).to_string()));
            fields.push(("bit_depth_chroma".into(), ((p[18] & 0x07) + 8).to_string()));
            let avg_fr = u16::from_be_bytes([p[21], p[22]]);
            fields.push(("avg_frame_rate".into(), avg_fr.to_string()));
            let byte23 = p[23];
            let constant_frame_rate = (byte23 >> 6) & 0x03;
            let num_temporal_layers = (byte23 >> 3) & 0x07;
            let temporal_id_nested = (byte23 >> 2) & 0x01;
            let length_size_minus_one = byte23 & 0x03;
            fields.push((
                "constant_frame_rate".into(),
                constant_frame_rate.to_string(),
            ));
            fields.push((
                "num_temporal_layers".into(),
                (num_temporal_layers + 1).to_string(),
            ));
            fields.push(("temporal_id_nested".into(), temporal_id_nested.to_string()));
            fields.push((
                "length_size".into(),
                (length_size_minus_one + 1).to_string(),
            ));
            // NAL arrays follow
            if p.len() > 24 {
                let num_arrays = p[24];
                let mut arr_offset = 25;
                for _ in 0..num_arrays {
                    if arr_offset + 3 > p.len() {
                        break;
                    }
                    let ac = (p[arr_offset] >> 7) & 0x01;
                    let nut = p[arr_offset] & 0x3F;
                    let num_nals = u16::from_be_bytes([p[arr_offset + 1], p[arr_offset + 2]]);
                    fields.push(("NAL_unit_type".into(), nut.to_string()));
                    fields.push(("array_completeness".into(), ac.to_string()));
                    arr_offset += 3;
                    for _ in 0..num_nals {
                        if arr_offset + 2 > p.len() {
                            break;
                        }
                        let nal_len =
                            u16::from_be_bytes([p[arr_offset], p[arr_offset + 1]]) as usize;
                        arr_offset += 2;
                        if arr_offset + nal_len > p.len() {
                            break;
                        }
                        let nal_data = &p[arr_offset..arr_offset + nal_len];
                        let hex: Vec<_> = nal_data.iter().map(|b| format!("{:02x}", b)).collect();
                        fields.push(("NAL data".into(), hex.join(" ")));
                        arr_offset += nal_len;
                    }
                }
            }
        }
        "ispe" => {
            if payload.len() >= 12 {
                let w = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let h = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                fields.push(("image width".into(), w.to_string()));
                fields.push(("image height".into(), h.to_string()));
            }
        }
        "pixi" => {
            if payload.len() >= 5 {
                let num_channels = payload[4];
                if payload.len() >= 5 + num_channels as usize {
                    let channels: Vec<_> = payload[5..5 + num_channels as usize]
                        .iter()
                        .map(|b| b.to_string())
                        .collect();
                    fields.push(("bits_per_channel".into(), channels.join(",")));
                }
            }
        }
        "idat" => {
            fields.push(("number of data bytes".into(), payload.len().to_string()));
        }
        "iloc" => {
            // parsed by parse_iloc_entries_to_fields
        }
        "mdat" => {
            fields.push(("size".into(), payload.len().to_string()));
        }
        "auxC" => {
            if payload.len() >= 2 {
                let aux_type = String::from_utf8_lossy(&payload[1..]).trim().to_string();
                fields.push(("aux_type".into(), aux_type));
            }
        }
        "grid" => {
            if payload.len() >= 8 {
                let rows = payload[1] as u32 + 1;
                let cols = payload[0] as u32 + 1;
                let out_w =
                    u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]) + 1;
                let out_h =
                    u32::from_be_bytes([payload[6], payload[7], payload[8], payload[9]]) + 1;
                fields.push(("rows".into(), rows.to_string()));
                fields.push(("columns".into(), cols.to_string()));
                fields.push(("output_width".into(), out_w.to_string()));
                fields.push(("output_height".into(), out_h.to_string()));
            }
        }
        "clap" => {
            if payload.len() >= 32 {
                let cw_n = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let cw_d = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let ch_n = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                let ch_d = u32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]);
                let ho_n = i32::from_be_bytes([payload[16], payload[17], payload[18], payload[19]]);
                let ho_d = u32::from_be_bytes([payload[20], payload[21], payload[22], payload[23]]);
                let vo_n = i32::from_be_bytes([payload[24], payload[25], payload[26], payload[27]]);
                let vo_d = u32::from_be_bytes([payload[28], payload[29], payload[30], payload[31]]);
                fields.push(("cleanApertureWidthN".into(), cw_n.to_string()));
                fields.push(("cleanApertureWidthD".into(), cw_d.to_string()));
                fields.push(("cleanApertureHeightN".into(), ch_n.to_string()));
                fields.push(("cleanApertureHeightD".into(), ch_d.to_string()));
                fields.push(("horizOffN".into(), ho_n.to_string()));
                fields.push(("horizOffD".into(), ho_d.to_string()));
                fields.push(("vertOffN".into(), vo_n.to_string()));
                fields.push(("vertOffD".into(), vo_d.to_string()));
            }
        }
        _ => {}
    }
}

fn parse_iref_children(
    box_bytes: &[u8],
    payload_start: usize,
    global_offset: usize,
    child_end: usize,
) -> Vec<FileBlock> {
    let mut children = Vec::new();
    let mut co = payload_start;
    while co + 8 <= child_end {
        let Some((csize, ctype, _)) = read_box_header(box_bytes, co) else {
            break;
        };
        if csize < 12 || co + csize > child_end {
            break;
        }
        // SingleReferenceBox: from_item_id (u16 at offset 4), ref_count (u16 at offset 6), to_item_ids...
        let cdata = &box_bytes[co..co + csize];
        let mut cfields = Vec::new();
        if cdata.len() >= 8 {
            let from_id = u16::from_be_bytes([cdata[4], cdata[5]]) as u32;
            let ref_count = u16::from_be_bytes([cdata[6], cdata[7]]) as usize;
            cfields.push(("from ID".into(), from_id.to_string()));
            let mut to_ids = Vec::new();
            let mut pos = 8;
            for _ in 0..ref_count {
                if pos + 2 > cdata.len() {
                    break;
                }
                to_ids.push(u16::from_be_bytes([cdata[pos], cdata[pos + 1]]) as u32);
                pos += 2;
            }
            let to_str: Vec<_> = to_ids.iter().map(|id| id.to_string()).collect();
            cfields.push(("type".into(), ctype.clone()));
            cfields.push(("to IDs".into(), to_str.join(" ")));
        }
        children.push(FileBlock {
            name: ctype,
            offset: (global_offset + co) as u64,
            length: csize as u64,
            fields: cfields,
            decoded_info: None,
            data_preview: None,
            children: Vec::new(),
        });
        co += csize;
    }
    children
}

fn parse_iloc_entries_to_fields(
    box_bytes: &[u8],
    payload_start: usize,
    _global_offset: usize,
    fields: &mut Vec<(String, String)>,
) {
    let data = &box_bytes[payload_start..];
    // iloc is a FullBox: version(1) + flags(3) + header fields
    if data.len() < 12 {
        return;
    }
    let version = data[0];
    let offset_size_field = (data[4] >> 4) & 0x0F;
    let length_size_field = data[4] & 0x0F;
    let base_offset_size_field = (data[5] >> 4) & 0x0F;

    let offset_size = if offset_size_field == 0 {
        4
    } else {
        offset_size_field as usize
    };
    let length_size = if length_size_field == 0 {
        4
    } else {
        length_size_field as usize
    };
    let base_offset_size = base_offset_size_field as usize;

    let header_size = if version == 2 { 9 } else { 8 };
    let item_count = u16::from_be_bytes([data[6], data[7]]) as usize;

    fn read_field(data: &[u8], offset: usize, size: usize) -> u64 {
        if size == 0 || offset + size > data.len() {
            return 0;
        }
        let mut val = 0u64;
        for i in 0..size {
            val = (val << 8) | data[offset + i] as u64;
        }
        val
    }

    let mut pos = header_size;
    for _ in 0..item_count {
        if pos + 2 > data.len() {
            break;
        }
        let item_id = if version < 2 {
            let id = u16::from_be_bytes([data[pos], data[pos + 1]]);
            pos += 2;
            id as u32
        } else {
            if pos + 4 > data.len() {
                break;
            }
            let id = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;
            id
        };

        if pos + 2 > data.len() {
            break;
        }
        let dr_and_cm = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        let construction_method = if version >= 1 { dr_and_cm >> 12 } else { 0 };
        let data_ref_idx = dr_and_cm & 0x0FFF;

        // base_offset: v0 and v1 always 2 bytes; v2 uses header size field
        let base_offset = if version == 0 || version == 1 {
            let val = read_field(data, pos, 2);
            pos += 2;
            val
        } else if base_offset_size > 0 {
            let val = read_field(data, pos, base_offset_size);
            pos += base_offset_size;
            val
        } else {
            0
        };

        if pos + 2 > data.len() {
            break;
        }
        let extent_count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;

        let mut extents = Vec::new();
        for _ in 0..extent_count {
            if pos + offset_size > data.len() {
                break;
            }
            let extent_offset = read_field(data, pos, offset_size);
            pos += offset_size;
            if pos + length_size > data.len() {
                break;
            }
            let extent_length = read_field(data, pos, length_size);
            pos += length_size;
            extents.push(format!("{},{}", extent_offset, extent_length));
        }

        fields.push((
            format!("item ID: {}", item_id),
            format!(
                "construction method: {}, data_reference_index: {}, base_offset: {}, extents=[{}]",
                construction_method,
                data_ref_idx,
                base_offset,
                extents.join("; ")
            ),
        ));
    }
}

fn parse_ipma_entries(
    box_bytes: &[u8],
    payload_start: usize,
    _global_offset: usize,
    fields: &mut Vec<(String, String)>,
) {
    let data = &box_bytes[payload_start..];
    // ipma is a FullBox: version(1) + flags(3) + entry_count(4)
    // flags bit 0: 1=32-bit item_ID, 0=16-bit item_ID
    if data.len() < 8 {
        return;
    }
    let flags = u32::from_be_bytes([0, data[1], data[2], data[3]]);
    let large_item_id = (flags & 0x01) != 0;
    let entry_count = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut pos = 8;

    for _ in 0..entry_count {
        if pos + 2 > data.len() {
            break;
        }
        let item_id = if large_item_id {
            if pos + 4 > data.len() {
                break;
            }
            let id = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;
            id
        } else {
            let id = u16::from_be_bytes([data[pos], data[pos + 1]]) as u32;
            pos += 2;
            id
        };

        if pos >= data.len() {
            break;
        }
        let assoc_count = data[pos] as usize;
        pos += 1;

        let mut assocs = Vec::new();
        for _ in 0..assoc_count {
            if pos + 2 > data.len() {
                break;
            }
            let b0 = data[pos];
            let b1 = data[pos + 1];
            pos += 2;
            let essential = (b0 & 0x80) != 0;
            let prop_idx = if essential {
                (((b0 & 0x7F) as u16) << 8) | (b1 as u16)
            } else {
                b1 as u16
            };
            assocs.push(format!(
                "property index: {} (essential: {})",
                prop_idx, essential
            ));
        }
        fields.push((
            format!("associations for item ID: {}", item_id),
            assocs.join(", "),
        ));
    }
}

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

    // Find and parse the meta box
    let mut offset = 0;
    let mut meta_data: Option<&[u8]> = None;
    let mut _meta_offset = 0;
    let mut _meta_size = 0;
    while offset + 8 <= bytes.len() {
        let Some((size, box_type, _)) = read_box_header(&bytes, offset) else {
            break;
        };
        if box_type == "meta" {
            meta_data = Some(&bytes[offset..offset + size]);
            _meta_offset = offset;
            _meta_size = size;
            break;
        }
        offset += size;
    }

    let (structure, errors) = parse_heif_structure(&bytes);

    if meta_data.is_none() {
        return Err("No meta box found in HEIF file".to_string());
    }

    let meta_bytes = meta_data.unwrap();
    let meta_content_start = 12; // skip box header (8) + FullBox version+flags (4)

    // Parse pitm (primary item)
    let mut primary_item_id: Option<u32> = None;
    // Parse iinf (items)
    let mut items: Vec<HeifItem> = Vec::new();
    // Parse iprp (properties)
    let mut properties: Vec<HeifProperty> = Vec::new();
    let mut item_props: HashMap<u32, Vec<(usize, bool)>> = HashMap::new();
    // Parse iref (references)
    let mut refs: Vec<HeifRef> = Vec::new();
    // Grid info
    let mut grid: Option<GridInfo> = None;
    // ICC data
    let mut _icc_data: Option<Vec<u8>> = None;

    // Walk through meta children
    let mut co = meta_content_start;
    let meta_end = meta_bytes.len();
    let mut grid_box_data: Option<&[u8]> = None;

    // First pass: find iloc to get item IDs and locations (needed before parsing iinf)
    let mut iloc_co = meta_content_start;
    let mut iloc_entries: Vec<IlocEntry> = Vec::new();
    while iloc_co + 8 <= meta_end {
        let Some((iloc_size, iloc_type, _)) = read_box_header(meta_bytes, iloc_co) else {
            break;
        };
        if iloc_type == "iloc" {
            iloc_entries = parse_iloc_entries(&meta_bytes[iloc_co + 8..iloc_co + iloc_size]);
            break;
        }
        iloc_co += iloc_size;
    }
    let iloc_ids: Vec<u32> = iloc_entries.iter().map(|e| e.item_id).collect();

    while co + 8 <= meta_end {
        let Some((size, box_type, hdr_extra)) = read_box_header(meta_bytes, co) else {
            break;
        };
        let data_start = co + 8 + hdr_extra;
        let data_end = co + size;
        if data_end > meta_end || size < 8 {
            break;
        }

        match box_type.as_str() {
            "pitm" => {
                if data_start + 6 <= data_end {
                    let pitm_payload = data_start + 4;
                    let ver = meta_bytes[data_start];
                    if ver == 0 {
                        primary_item_id = Some(u16::from_be_bytes([
                            meta_bytes[pitm_payload],
                            meta_bytes[pitm_payload + 1],
                        ]) as u32);
                    } else {
                        primary_item_id = Some(u32::from_be_bytes([
                            meta_bytes[pitm_payload],
                            meta_bytes[pitm_payload + 1],
                            meta_bytes[pitm_payload + 2],
                            meta_bytes[pitm_payload + 3],
                        ]));
                    }
                }
            }
            "iinf" => {
                items = parse_iinf(&meta_bytes[co + 8..co + size], &iloc_ids);
            }
            "iprp" => {
                (properties, item_props) = parse_iprp(&meta_bytes[co + 8..co + size]);
            }
            "iref" => {
                // iref is a FullBox container. Its children (dimg, auxl, etc.)
                // are SingleItemTypeReferenceBox FullBoxes.
                // data_start points to iref version byte. Strip 4 bytes to skip
                // iref FullBox header so parse_iref gets data starting at first child box.
                let iref_content = &meta_bytes[data_start..data_end];
                if iref_content.len() > 4 {
                    refs = parse_iref(&iref_content[4..]);
                    for _r in &refs {}
                }
            }
            "grid" => {
                let gslice = &meta_bytes[co + 8..co + size];
                if gslice.len() >= 11 {}
                grid_box_data = Some(gslice);
            }
            _ => {}
        }

        co += size;
    }

    // Parse grid box if found from meta children, or from grid item data via iloc
    let grid_tile_ids: Vec<u32> = refs
        .iter()
        .filter(|r| r.ref_type == "dimg")
        .flat_map(|r| r.to_item_ids.clone())
        .collect();
    for _e in iloc_entries.iter().take(5) {}
    // Show grid item location
    if let Some(g) = items.iter().find(|it| it.item_type == "grid") {
        if let Some(_ie) = iloc_entries.iter().find(|e| e.item_id == g.item_id) {}
    }

    // If no grid box found in meta children, try reading from grid item's data via iloc
    if grid_box_data.is_none() {
        if let Some(grid_item) = items.iter().find(|it| it.item_type == "grid") {
            if let Some(iloc_entry) = iloc_entries.iter().find(|e| e.item_id == grid_item.item_id) {
                let start = iloc_entry.offset as usize;
                let end = start + iloc_entry.length as usize;
                if end <= bytes.len() && start < end {
                    grid_box_data = Some(&bytes[start..end]);
                }
            }
        }
    }

    if let Some(gdata) = grid_box_data {
        if gdata.len() >= 9 {
            // FullBox: version(1)+flags(3) + flags_byte + rows-1 + cols-1
            let _version = gdata[0];
            let flags = gdata[4];
            let cols_minus1 = gdata[5];
            let rows_minus1 = gdata[6];
            let has_16bit = (flags & 1) != 0;

            let pos = 7;
            let output_w = if has_16bit && pos + 8 <= gdata.len() {
                u32::from_be_bytes([gdata[pos], gdata[pos + 1], gdata[pos + 2], gdata[pos + 3]])
            } else if pos + 4 <= gdata.len() {
                u16::from_be_bytes([gdata[pos], gdata[pos + 1]]) as u32
            } else {
                0
            };

            let h_pos = if has_16bit { pos + 4 } else { pos + 2 };
            let output_h = if h_pos + 2 <= gdata.len() {
                u16::from_be_bytes([gdata[h_pos], gdata[h_pos + 1]]) as u32
            } else {
                0
            };

            let cols = cols_minus1 as u32 + 1;
            let rows = rows_minus1 as u32 + 1;

            let tiles = build_grid_tiles(&grid_tile_ids, &items, rows, cols, output_w, output_h);

            grid = Some(GridInfo {
                rows,
                cols,
                output_width: output_w,
                output_height: output_h,
                tiles,
            });
        }
    }

    // Find the primary image and resolve its dimensions
    let pid = primary_item_id.unwrap_or(0);

    // Get properties for the primary item via ipma
    let props_for_item = item_props.get(&pid).cloned().unwrap_or_default();

    // Check if primary item is a grid
    let is_grid = items
        .iter()
        .find(|it| it.item_id == pid)
        .map(|it| it.item_type == "grid")
        .unwrap_or(false);

    let (width, height, bit_depth, color_type, has_alpha, codec_raw_data) = if is_grid {
        // For grid images, get dimensions from the grid's own ispe property
        let mut w = 0u32;
        let mut h = 0u32;
        let mut bd = 0u8;
        let mut ct = "HEVC".to_string();
        let mut raw: Option<Vec<u8>> = None;

        for (prop_idx, _essential) in &props_for_item {
            if *prop_idx > 0 && *prop_idx <= properties.len() {
                match &properties[prop_idx - 1] {
                    HeifProperty::Ispe {
                        width: sw,
                        height: sh,
                    } => {
                        w = *sw;
                        h = *sh;
                    }
                    HeifProperty::Pixi { channels } => {
                        if !channels.is_empty() {
                            bd = channels[0];
                        }
                    }
                    HeifProperty::HvcC {
                        bit_depth_luma,
                        bit_depth_chroma: _,
                        raw_data,
                        ..
                    } => {
                        bd = *bit_depth_luma;
                        ct = "HEVC".to_string();
                        raw = Some(raw_data.clone());
                    }
                    _ => {}
                }
            }
        }

        (w, h, bd, ct, false, raw)
    } else {
        // Regular image: find ispe and codec config
        let mut w = 0u32;
        let mut h = 0u32;
        let mut bd = 0u8;
        let mut ct = String::new();
        let mut ha = false;
        let mut raw: Option<Vec<u8>> = None;

        // Check for alpha channel via iref
        for r in &refs {
            if r.from_item_id == pid && r.ref_type == "auxl" {
                ha = true;
            }
        }

        for (prop_idx, _essential) in &props_for_item {
            if *prop_idx > 0 && *prop_idx <= properties.len() {
                match &properties[prop_idx - 1] {
                    HeifProperty::Ispe {
                        width: sw,
                        height: sh,
                    } => {
                        w = *sw;
                        h = *sh;
                    }
                    HeifProperty::Pixi { channels } => {
                        if !channels.is_empty() {
                            bd = channels[0];
                        }
                    }
                    HeifProperty::HvcC {
                        bit_depth_luma,
                        chroma_format,
                        raw_data,
                        ..
                    } => {
                        bd = *bit_depth_luma;
                        ct = match chroma_format {
                            0 => "YUV 4:0:0".to_string(),
                            1 => "YUV 4:2:0".to_string(),
                            2 => "YUV 4:2:2".to_string(),
                            3 => "YUV 4:4:4".to_string(),
                            _ => "HEVC".to_string(),
                        };
                        raw = Some(raw_data.clone());
                    }
                    HeifProperty::ColrIcc { profile_data } => {
                        _icc_data = Some(profile_data.clone());
                    }
                    _ => {}
                }
            }
        }

        if ct.is_empty() {
            let item = items.iter().find(|it| it.item_id == pid);
            ct = item.map(|it| it.item_type.clone()).unwrap_or_default();
        }

        (w, h, bd, ct, ha, raw)
    };

    // Apply rotation (irot) to dimensions
    let (final_width, final_height) = {
        let mut fw = width;
        let mut fh = height;
        for (prop_idx, _) in &props_for_item {
            if *prop_idx > 0 && *prop_idx <= properties.len() {
                if let HeifProperty::Irot {
                    anticlockwise_rotation,
                } = &properties[prop_idx - 1]
                {
                    if *anticlockwise_rotation == 1 || *anticlockwise_rotation == 3 {
                        std::mem::swap(&mut fw, &mut fh);
                    }
                }
            }
        }
        (fw, fh)
    };

    // If grid was detected via iref but grid_box_data wasn't found, infer dimensions
    if grid.is_none() && !grid_tile_ids.is_empty() && is_grid {
        let tile_count = grid_tile_ids.len() as u32;
        let output_w = final_width;
        let output_h = final_height;

        // Find best rows/cols split based on aspect ratio
        let mut best_rows = 1u32;
        let mut best_cols = tile_count;
        let mut best_diff = f64::MAX;
        for r in 1..=tile_count {
            if tile_count % r == 0 {
                let c = tile_count / r;
                let tile_aspect = (output_w as f64 / c as f64) / (output_h as f64 / r as f64);
                let diff = (tile_aspect - 1.0).abs();
                if diff < best_diff {
                    best_diff = diff;
                    best_rows = r;
                    best_cols = c;
                }
            }
        }

        let tiles = build_grid_tiles(
            &grid_tile_ids,
            &items,
            best_rows,
            best_cols,
            output_w,
            output_h,
        );

        grid = Some(GridInfo {
            rows: best_rows,
            cols: best_cols,
            output_width: output_w,
            output_height: output_h,
            tiles,
        });
    }

    // Extract codec syntax from codec config and merge with actual tile/item sample data.
    let codec_syntax = {
        let raw_from_tile = item_props.iter().find_map(|(_item_id, prop_list)| {
            for (pi, _) in prop_list {
                if *pi > 0 && *pi <= properties.len() {
                    if let HeifProperty::HvcC { raw_data, .. } = &properties[*pi - 1] {
                        return Some(raw_data.clone());
                    }
                }
            }
            None
        });
        let config_raw = codec_raw_data.clone().or(raw_from_tile);
        let sample_item_ids: Vec<u32> = if is_grid && !grid_tile_ids.is_empty() {
            grid_tile_ids.clone()
        } else {
            vec![pid]
        };

        if let Some(raw) = config_raw {
            let hvcc = crate::analyzer::hevc::parse_hevc_bitstream(&raw);
            let merged = sample_item_ids.iter().fold(hvcc, |acc, item_id| {
                let Some(sample_bytes) = extract_item_payload(&bytes, &iloc_entries, *item_id) else {
                    return acc;
                };
                let sample = crate::analyzer::hevc::parse_hevc_bitstream_with_seed(
                    sample_bytes,
                    acc.vps.clone(),
                    acc.sps.clone(),
                    acc.pps.clone(),
                );
                crate::analyzer::hevc::merge_hevc_syntax(acc, sample)
            });
            Some(CodecSyntax::Hevc(merged))
        } else {
            let has_av1 = items.iter().any(|it| it.item_type == "av01");
            if has_av1 {
                let merged = sample_item_ids.iter().fold(
                    crate::analyzer::av1::parse_av1_bitstream(&[]),
                    |acc, item_id| {
                    let Some(sample_bytes) = extract_item_payload(&bytes, &iloc_entries, *item_id) else {
                        return acc;
                    };
                    let sample = crate::analyzer::av1::parse_av1_bitstream(sample_bytes);
                    crate::analyzer::av1::merge_av1_syntax(acc, sample)
                    }
                );
                if merged.sequence_header.is_some() || !merged.obus.is_empty() {
                    Some(CodecSyntax::Av1(merged))
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    let file_size = bytes.len() as u64;
    let format_label = match format {
        ImageFormat::Heic => "HEIF/HEIC",
        ImageFormat::Avif => "AVIF",
        _ => "HEIF",
    };

    // Build metadata
    let mut metadata = Vec::new();
    metadata.push(MetadataEntry {
        standard: "File".to_string(),
        tag_name: "Format".to_string(),
        tag_value: format_label.to_string(),
        raw_value: None,
    });
    metadata.push(MetadataEntry {
        standard: "File".to_string(),
        tag_name: "File Size".to_string(),
        tag_value: format_bytes(file_size),
        raw_value: None,
    });
    if final_width > 0 && final_height > 0 {
        metadata.push(MetadataEntry {
            standard: "Image".to_string(),
            tag_name: "Dimensions".to_string(),
            tag_value: format!("{final_width} × {final_height}"),
            raw_value: None,
        });
    }
    if bit_depth > 0 {
        metadata.push(MetadataEntry {
            standard: "Image".to_string(),
            tag_name: "Bit Depth".to_string(),
            tag_value: format!("{bit_depth} bits"),
            raw_value: None,
        });
    }
    if !color_type.is_empty() {
        metadata.push(MetadataEntry {
            standard: "Image".to_string(),
            tag_name: "Color Type".to_string(),
            tag_value: color_type.clone(),
            raw_value: None,
        });
    }
    if has_alpha {
        metadata.push(MetadataEntry {
            standard: "Image".to_string(),
            tag_name: "Alpha".to_string(),
            tag_value: "yes".to_string(),
            raw_value: None,
        });
    }
    if is_grid {
        if let Some(ref g) = grid {
            metadata.push(MetadataEntry {
                standard: "Grid".to_string(),
                tag_name: "Layout".to_string(),
                tag_value: format!("{}x{} ({} tiles)", g.rows, g.cols, g.rows * g.cols),
                raw_value: None,
            });
        }
    }

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size,
        format,
        width: final_width,
        height: final_height,
        color_type,
        bit_depth,
        has_alpha,
        thumbnail_base64: None,
        structure,
        metadata,
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
    use std::fs;

    #[test]
    fn test_parse_real_heic_file() {
        let path = "/Users/liguodu/Downloads/image1.heic";
        // Skip if file doesn't exist (CI environment)
        if !std::path::Path::new(path).exists() {
            return;
        }
        let bytes = fs::read(path).unwrap();
        let result = analyze_heif(path);
        assert!(result.is_ok(), "Failed to parse HEIC: {:?}", result);
        let analysis = result.unwrap();
        if let Some(ref g) = analysis.grid {}
        assert_eq!(analysis.format, ImageFormat::Heic);
        // The file is a 3992x2992 grid image
        assert!(analysis.width > 0, "Width should be resolved from ispe");
        assert!(analysis.height > 0, "Height should be resolved from ispe");
        assert_eq!(analysis.width, 3992);
        assert_eq!(analysis.height, 2992);
        assert!(
            analysis.bit_depth > 0,
            "Bit depth should be resolved from hvcC"
        );
        let codec = analysis
            .codec_syntax
            .as_ref()
            .expect("Should extract codec syntax from tile hvcC");
        match codec {
            CodecSyntax::Hevc(hevc) => {
                assert!(
                    hevc.nal_units.len() >= 3,
                    "Expected VPS/SPS/PPS to be extracted from hvcC"
                );
                assert!(
                    hevc.nal_units.len() > 3,
                    "Expected tile sample data to contribute slice NAL units"
                );
                assert!(
                    !hevc.slice_headers.is_empty(),
                    "Expected at least one HEVC slice header from tile sample data"
                );
                let sps = hevc.sps.as_ref().expect("Should parse SPS");
                assert_eq!(sps.general_profile_idc, 1);
                assert_eq!(sps.chroma_format_idc, 1);
                assert!(
                    sps.bit_depth_luma_minus8 + 8 >= 8,
                    "Unexpected HEVC bit depth: {}",
                    sps.bit_depth_luma_minus8 + 8
                );
            }
            _ => panic!("HEIC sample should produce HEVC codec syntax"),
        }
        // Meta is a wrapper box containing children
        let meta = analysis
            .structure
            .iter()
            .find(|b| b.name == "meta")
            .expect("Should have meta block");
        let meta_child_names: Vec<_> = meta.children.iter().map(|b| &b.name).collect();
        assert!(
            meta_child_names.contains(&&"hdlr".to_string()),
            "meta should have hdlr child"
        );
        assert!(
            meta_child_names.contains(&&"pitm".to_string()),
            "meta should have pitm child"
        );
        assert!(
            meta_child_names.contains(&&"iinf".to_string()),
            "meta should have iinf child"
        );
        assert!(
            meta_child_names.contains(&&"iprp".to_string()),
            "meta should have iprp child"
        );
        assert!(
            meta_child_names.contains(&&"iloc".to_string()),
            "meta should have iloc child"
        );
        // Top-level boxes
        let top_names: Vec<_> = analysis.structure.iter().map(|b| &b.name).collect();
        assert!(top_names.contains(&&"ftyp".to_string()), "Should have ftyp");
        assert!(top_names.contains(&&"mdat".to_string()), "Should have mdat");
        // Check infe children exist inside iinf
        let iinf = meta
            .children
            .iter()
            .find(|b| b.name == "iinf")
            .expect("Should have iinf block inside meta");
        assert!(
            iinf.children.len() >= 3,
            "iinf should have at least 3 infe children, got {}",
            iinf.children.len()
        );
        let infe_types: Vec<_> = iinf.children.iter().map(|c| &c.name).collect();
        assert!(
            infe_types.contains(&&"infe".to_string()),
            "iinf should have infe children"
        );
        assert!(analysis.grid.is_some(), "Should detect grid layout");
        let grid = analysis.grid.as_ref().unwrap();
        assert_eq!(grid.rows, 6, "image1.heic should be a 6-row grid");
        assert_eq!(grid.cols, 8, "image1.heic should be an 8-column grid");
        assert_eq!(grid.rows * grid.cols, grid.tiles.len() as u32);
        assert!(grid.tiles.iter().all(|t| t.width > 0 && t.height > 0));
        assert_eq!(grid.tiles.first().map(|t| (t.horizontal_offset, t.vertical_offset)), Some((0, 0)));
        assert!(grid.tiles.last().map(|t| t.horizontal_offset).unwrap_or(0) > 0);
        assert!(grid.tiles.last().map(|t| t.vertical_offset).unwrap_or(0) > 0);
    }

    #[test]
    fn test_parse_real_avif_file() {
        let path = "/Users/liguodu/projects/avif-sample-images/red-at-12-oclock-with-color-profile-8bpc.avif";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let analysis = analyze_heif(path).expect("Failed to parse AVIF");
        assert_eq!(analysis.format, ImageFormat::Avif);
        assert!(analysis.width > 0);
        assert!(analysis.height > 0);
        let codec = analysis.codec_syntax.as_ref().expect("AVIF should have codec syntax");
        match codec {
            CodecSyntax::Av1(av1) => {
                assert!(!av1.obus.is_empty(), "Expected AV1 OBUs from AVIF sample");
                let sequence_header = av1
                    .sequence_header
                    .as_ref()
                    .expect("Expected AV1 sequence header");
                assert_eq!(sequence_header.seq_profile, 0);
                assert!(!sequence_header.color_config.high_bitdepth);
                assert!(sequence_header.color_config.subsampling_x);
                assert!(sequence_header.color_config.subsampling_y);
                assert_eq!(sequence_header.max_frame_width_minus1 + 1, 800);
                assert_eq!(sequence_header.max_frame_height_minus1 + 1, 800);
            }
            _ => panic!("AVIF sample should produce AV1 codec syntax"),
        }
    }
}
