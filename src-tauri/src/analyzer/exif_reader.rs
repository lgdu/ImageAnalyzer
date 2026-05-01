use crate::types::MetadataEntry;
use exif::Reader;

pub fn read_exif(data: &[u8]) -> Result<Vec<MetadataEntry>, String> {
    // kamadak-exif's read_raw expects a Vec<u8>.
    // The data passed here is the APP1 payload after "Exif\0\0",
    // which is raw TIFF-formatted EXIF data.
    let reader = Reader::new();

    let exif = reader
        .read_raw(data.to_vec())
        .map_err(|e| format!("Failed to parse EXIF data: {}", e))?;

    let mut entries = Vec::new();

    for field in exif.fields() {
        let tag_name = field.tag.to_string();
        let tag_value = field.display_value().with_unit(&exif).to_string();

        let raw_value = match field.value {
            exif::Value::Ascii(ref vals) => {
                vals.first().map(|v| String::from_utf8_lossy(v).to_string())
            }
            _ => Some(tag_value.clone()),
        };

        entries.push(MetadataEntry {
            standard: "EXIF".to_string(),
            tag_name,
            tag_value,
            raw_value,
        });
    }

    Ok(entries)
}
