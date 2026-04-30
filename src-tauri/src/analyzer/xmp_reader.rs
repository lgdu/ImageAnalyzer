use crate::types::MetadataEntry;

pub fn read_xmp(_data: &[u8]) -> Result<Vec<MetadataEntry>, String> {
    Err("XMP reader not yet implemented".to_string())
}
