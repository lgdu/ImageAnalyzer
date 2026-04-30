use crate::types::ImageAnalysis;

pub fn analyze_webp(_path: &str) -> Result<ImageAnalysis, String> {
    Err("WebP parser not yet implemented".to_string())
}
