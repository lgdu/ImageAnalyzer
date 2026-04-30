use crate::types::Av1Syntax;

pub fn parse_av1(_data: &[u8]) -> Result<Av1Syntax, String> {
    Err("AV1 parser not yet implemented".to_string())
}
