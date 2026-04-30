use crate::types::ImageAnalysis;

pub fn analyze_gif(_path: &str) -> Result<ImageAnalysis, String> {
    Err("GIF parser not yet implemented".to_string())
}
