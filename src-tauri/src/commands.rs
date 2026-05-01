use crate::analyzer::{self, gif_parser, jpeg_parser, png_parser, webp_parser};
use crate::types::{ImageAnalysis, ImageFormat};
use tauri::command;

#[command]
pub async fn analyze_image(file_path: String) -> Result<ImageAnalysis, String> {
    let format = analyzer::detect_format(&file_path)
        .ok_or_else(|| format!("Unsupported format: {}", file_path))?;

    match format {
        ImageFormat::Png => png_parser::analyze_png(&file_path),
        ImageFormat::Jpeg => jpeg_parser::analyze_jpeg(&file_path),
        ImageFormat::Webp => webp_parser::analyze_webp(&file_path),
        ImageFormat::Gif => gif_parser::analyze_gif(&file_path),
        _ => Err(format!("Parser not implemented for {:?}", format)),
    }
}

#[command]
pub async fn analyze_batch(file_paths: Vec<String>) -> Result<Vec<ImageAnalysis>, String> {
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for path in file_paths {
        match analyze_image(path).await {
            Ok(analysis) => results.push(analysis),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(results)
    } else {
        Err(errors.join("; "))
    }
}
