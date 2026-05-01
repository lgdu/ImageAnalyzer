use crate::types::MetadataEntry;

/// Photoshop Image Resource Block header:
/// - Signature: "8BIM" (4 bytes)
/// - Resource ID: 2 bytes (big-endian)
/// - Name: Pascal string (length byte + name + optional padding byte)
/// - Data size: 4 bytes (big-endian)
/// - Data: data_size bytes (padded to even)

const PHOTOSHOP_SIG: &[u8] = b"8BIM";
const IPTC_NAA_ID: u16 = 1028;

/// IPTC dataset record:
/// - Record number: 2 bytes (big-endian, 0x1C)
/// - Dataset number: 1 byte
/// - Data size: 1 or 2 bytes (depends on dataset)
/// - Data: variable

const IPTC_RECORD_MARKER: u8 = 0x1C;

/// Map IPTC dataset numbers to tag names
fn iptc_dataset_name(record: u8, dataset: u8) -> &'static str {
    match (record, dataset) {
        // IIM Envelope (record 1)
        (1, 0) => "Envelope Record Version",
        (1, 5) => "Destination",
        (1, 20) => "File Format",
        (1, 22) => "File Format Version",
        (1, 30) => "Service Identifier",
        (1, 40) => "Envelope Number",
        (1, 50) => "Product ID",
        (1, 60) => "Envelope Priority",
        (1, 70) => "Date Sent",
        (1, 80) => "Time Sent",
        (1, 90) => "Character Set",
        (1, 100) => "EOB",

        // Application Record (record 2)
        (2, 0) => "Application Record Version",
        (2, 3) => "Object Type Reference",
        (2, 5) => "ObjectName",
        (2, 7) => "Edit Status",
        (2, 10) => "Editorial Update",
        (2, 12) => "Urgency",
        (2, 15) => "Subject Reference",
        (2, 20) => "Category",
        (2, 22) => "Supplemental Category",
        (2, 25) => "Fixture Identifier",
        (2, 30) => "Keywords",
        (2, 35) => "Release Date",
        (2, 40) => "Release Time",
        (2, 45) => "Expiration Date",
        (2, 47) => "Expiration Time",
        (2, 50) => "Special Instructions",
        (2, 55) => "Recommended Service",
        (2, 60) => "Date Created",
        (2, 62) => "Digital Creation Date",
        (2, 63) => "Digital Creation Time",
        (2, 65) => "Originating Program",
        (2, 70) => "Program Version",
        (2, 75) => "Object Cycle",
        (2, 80) => "By-line",
        (2, 85) => "By-line Title",
        (2, 90) => "City",
        (2, 95) => "Sub-location",
        (2, 100) => "Province/State",
        (2, 101) => "Country Code",
        (2, 103) => "Country",
        (2, 105) => "Original Transmission Reference",
        (2, 110) => "Credit",
        (2, 115) => "Source",
        (2, 116) => "Copyright Notice",
        (2, 118) => "Contact",
        (2, 120) => "Caption",
        (2, 122) => "Writer/Editor",
        (2, 125) => "Rasterized Caption",
        (2, 130) => "Image Type",
        (2, 131) => "Image Orientation",
        (2, 135) => "Language Identifier",
        (2, 150) => "Audio Type",
        (2, 151) => "Audio Sampling Rate",
        (2, 152) => "Audio Sampling Resolution",
        (2, 153) => "Audio Duration",
        (2, 154) => "Audio Outcue",
        (2, 200) => "Preview File Format",
        (2, 201) => "Preview File Format Version",
        (2, 202) => "Preview Data",
        _ => "Unknown",
    }
}

pub fn read_iptc(data: &[u8]) -> Result<Vec<MetadataEntry>, String> {
    // Parse Photoshop Image Resource blocks
    let resources = parse_photoshop_resources(data)?;

    // Find IPTC-NAA record (ID 1028)
    let iptc_data = resources
        .iter()
        .find(|r| r.resource_id == IPTC_NAA_ID)
        .map(|r| r.data.as_slice())
        .ok_or_else(|| "No IPTC-NAA resource (ID 1028) found in Photoshop data".to_string())?;

    // Parse IPTC datasets
    let entries = parse_iptc_datasets(iptc_data);

    if entries.is_empty() {
        return Err("No IPTC datasets found".to_string());
    }

    Ok(entries)
}

struct PhotoshopResourceBlock {
    #[allow(dead_code)]
    signature: String,
    resource_id: u16,
    #[allow(dead_code)]
    name: String,
    data: Vec<u8>,
}

/// Parse Photoshop 8BIM resource blocks
fn parse_photoshop_resources(data: &[u8]) -> Result<Vec<PhotoshopResourceBlock>, String> {
    let mut resources = Vec::new();
    let mut pos = 0;

    while pos + 12 <= data.len() {
        // Check signature
        if pos + 4 > data.len() {
            break;
        }
        let sig = &data[pos..pos + 4];
        if sig != PHOTOSHOP_SIG {
            // Not a resource block, skip
            break;
        }
        pos += 4;

        // Resource ID (2 bytes)
        let resource_id = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;

        // Resource name: Pascal string (1 byte length, then name, then pad to even)
        if pos >= data.len() {
            return Err("Truncated resource name".to_string());
        }
        let name_len = data[pos] as usize;
        pos += 1;

        let name_end = pos + name_len;
        if name_end > data.len() {
            return Err("Truncated resource name data".to_string());
        }
        let name = String::from_utf8_lossy(&data[pos..name_end]).to_string();
        pos = name_end;

        // Pad name to even length
        if name_len % 2 == 0 {
            pos += 1;
        }

        // Data size (4 bytes)
        if pos + 4 > data.len() {
            return Err("Truncated resource data size".to_string());
        }
        let data_size =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // Data
        let data_end = pos + data_size;
        if data_end > data.len() {
            return Err(format!(
                "Truncated resource data: expected {} bytes, have {}",
                data_size,
                data.len() - pos
            ));
        }
        let block_data = data[pos..data_end].to_vec();
        pos = data_end;

        // Pad data to even length
        if data_size % 2 == 1 {
            pos += 1;
        }

        resources.push(PhotoshopResourceBlock {
            signature: "8BIM".to_string(),
            resource_id,
            name,
            data: block_data,
        });
    }

    if resources.is_empty() {
        return Err("No Photoshop resource blocks found".to_string());
    }

    Ok(resources)
}

/// Parse IPTC IIM datasets from raw IPTC data
fn parse_iptc_datasets(data: &[u8]) -> Vec<MetadataEntry> {
    let mut entries = Vec::new();
    let mut pos = 0;

    while pos + 4 < data.len() {
        // Check for record marker (should be 0x1C)
        if data[pos] != IPTC_RECORD_MARKER {
            // Not a valid dataset, skip
            pos += 1;
            continue;
        }

        pos += 1; // skip marker byte (0x1C)

        // Record number
        if pos >= data.len() {
            break;
        }
        let record = data[pos];
        pos += 1;

        // Dataset number
        if pos >= data.len() {
            break;
        }
        let dataset = data[pos];
        pos += 1;

        // Data size: 1 byte for sizes < 128, or 2 bytes (big-endian) if high bit set
        let data_size = if pos < data.len() {
            let size_byte = data[pos];
            pos += 1;
            if size_byte < 0x80 {
                size_byte as usize
            } else {
                // Extended size: 2-byte big-endian
                if pos >= data.len() {
                    break;
                }
                let size = ((size_byte as u16 & 0x7F) << 8) | data[pos] as u16;
                pos += 1;
                size as usize
            }
        } else {
            break;
        };

        if data_size == 0 {
            // Empty dataset, skip
            continue;
        }

        if pos + data_size > data.len() {
            break;
        }

        let dataset_data = &data[pos..pos + data_size];
        pos += data_size;

        // Try to decode as UTF-8/Latin-1
        let value = String::from_utf8_lossy(dataset_data).to_string();
        let tag_name = iptc_dataset_name(record, dataset);

        // For Keywords (dataset 30), there can be multiple values
        if tag_name == "Keywords" {
            // Each keyword in the dataset might be separated
            // IPTC allows multiple keyword records with same dataset number
            entries.push(MetadataEntry {
                standard: "IPTC".to_string(),
                tag_name: tag_name.to_string(),
                tag_value: value.clone(),
                raw_value: Some(value),
            });
        } else {
            entries.push(MetadataEntry {
                standard: "IPTC".to_string(),
                tag_name: tag_name.to_string(),
                tag_value: value.clone(),
                raw_value: Some(value),
            });
        }
    }

    entries
}
