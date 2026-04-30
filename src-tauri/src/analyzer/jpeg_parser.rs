use crate::types::ImageAnalysis;

pub fn analyze_jpeg(_path: &str) -> Result<ImageAnalysis, String> {
    Err("JPEG parser not yet implemented".to_string())
}
