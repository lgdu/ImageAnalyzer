use crate::types::HevcSyntax;

pub fn parse_hevc(_data: &[u8]) -> Result<HevcSyntax, String> {
    Err("HEVC parser not yet implemented".to_string())
}
