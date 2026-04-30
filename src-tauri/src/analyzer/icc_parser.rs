use crate::types::IccInfo;

pub fn parse_icc(_data: &[u8]) -> Result<IccInfo, String> {
    Err("ICC parser not yet implemented".to_string())
}
