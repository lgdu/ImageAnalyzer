use crate::types::MetadataEntry;

pub fn read_iptc(_data: &[u8]) -> Result<Vec<MetadataEntry>, String> {
    Err("IPTC reader not yet implemented".to_string())
}
