use crate::types::MetadataEntry;

pub fn read_exif(_data: &[u8]) -> Result<Vec<MetadataEntry>, String> {
    Err("EXIF reader not yet implemented".to_string())
}
