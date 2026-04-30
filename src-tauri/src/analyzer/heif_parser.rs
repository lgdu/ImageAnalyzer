use crate::types::ImageAnalysis;

pub fn analyze_heif(_path: &str) -> Result<ImageAnalysis, String> {
    Err("HEIF parser not yet implemented".to_string())
}
