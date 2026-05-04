use crate::analyzer::{self, gif_parser, heif_parser, jpeg_parser, png_parser, webp_parser};
use crate::types::{ChannelData, GifFrame, IccInfo, ImageAnalysis, ImageFormat};
use crate::utils::read_file_bytes;
use base64::{engine::general_purpose::STANDARD as base64_standard, Engine as _};
use image::codecs::gif::GifDecoder;
use image::imageops::FilterType;
use image::AnimationDecoder;
use image::GenericImageView;
use image::ImageFormat as ImageCrateFormat;
use std::io::Cursor;
#[cfg(target_os = "macos")]
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::command;

const THUMB_MAX_SIZE: u32 = 320;

fn generate_thumbnail(path: &str, _format: &ImageFormat) -> Option<String> {
    // Use macOS sips for formats where system decoder is faster
    #[cfg(target_os = "macos")]
    if matches!(_format, ImageFormat::Heic | ImageFormat::Avif) {
        if let Some(b64) = thumbnail_via_sips(path) {
            return Some(b64);
        }
        // sips failed, try image crate (unlikely to work for HEIC/AVIF)
    }

    // Use load_from_memory to auto-detect format from content, not extension
    let bytes = std::fs::read(path).ok()?;
    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(e) => {
            let _ = e; // image crate doesn't support HEIC/AVIF, sips already failed
            return None;
        }
    };

    let (w, h) = img.dimensions();
    let scale = THUMB_MAX_SIZE as f64 / w.max(h) as f64;
    if scale >= 1.0 {
        let rgba = img.to_rgba8();
        return encode_png_base64(&rgba, w, h);
    }
    let new_w = (w as f64 * scale).round() as u32;
    let new_h = (h as f64 * scale).round() as u32;
    let resized = img.resize(new_w, new_h, FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
    encode_png_base64(&rgba, new_w, new_h)
}

#[cfg(target_os = "macos")]
fn thumbnail_via_sips(path: &str) -> Option<String> {
    let output_path = temp_png_path("thumb");
    let output = std::process::Command::new("sips")
        .args([
            "-s",
            "format",
            "png",
            "-Z",
            &THUMB_MAX_SIZE.to_string(),
            "--out",
            output_path.to_str()?,
            path,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&output_path);
        return None;
    }

    let bytes = std::fs::read(&output_path).ok()?;
    let _ = std::fs::remove_file(output_path);

    let img = image::load_from_memory(&bytes).ok()?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    encode_png_base64(&rgba, w, h)
}

#[cfg(target_os = "macos")]
fn temp_png_path(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("image-analyzer-{prefix}-{}-{nanos}.png", std::process::id()))
}

#[cfg(target_os = "macos")]
fn decode_via_sips(path: &str) -> Result<Vec<u8>, String> {
    let output_path = temp_png_path("decode");
    let output = std::process::Command::new("sips")
        .args(["-s", "format", "png", "--out", output_path.to_str().ok_or("Invalid temp path")?, path])
        .output()
        .map_err(|e| format!("Failed to run sips: {e}"))?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&output_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("sips failed to decode image: {stderr}"));
    }

    let bytes = std::fs::read(&output_path)
        .map_err(|e| format!("Failed to read decoded image: {e}"))?;
    let _ = std::fs::remove_file(output_path);
    Ok(bytes)
}

fn load_channel_source_bytes(path: &str, format: &ImageFormat) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "macos")]
    if matches!(format, ImageFormat::Heic | ImageFormat::Avif) {
        return decode_via_sips(path);
    }

    read_file_bytes(path)
}

fn encode_png_base64(rgba: &image::RgbaImage, w: u32, h: u32) -> Option<String> {
    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut buf, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba.as_raw()).ok()?;
    }
    Some(base64_standard.encode(&buf))
}

fn fill_thumbnail(analysis: &mut ImageAnalysis, path: &str) {
    analysis.thumbnail_base64 = generate_thumbnail(path, &analysis.format);
}

#[command]
pub async fn analyze_image(file_path: String) -> Result<ImageAnalysis, String> {
    let format = analyzer::detect_format(&file_path)
        .ok_or_else(|| format!("Unsupported format: {}", file_path))?;

    let mut analysis = match format {
        ImageFormat::Png => png_parser::analyze_png(&file_path),
        ImageFormat::Jpeg => jpeg_parser::analyze_jpeg(&file_path),
        ImageFormat::Webp => webp_parser::analyze_webp(&file_path),
        ImageFormat::Gif => gif_parser::analyze_gif(&file_path),
        ImageFormat::Avif | ImageFormat::Heic => heif_parser::analyze_heif(&file_path),
    }?;

    fill_thumbnail(&mut analysis, &file_path);
    Ok(analysis)
}

/// Get channel statistics on-demand (heavy: decodes full image)
#[command]
pub async fn get_channels(file_path: String) -> Result<Option<ChannelData>, String> {
    let format = analyzer::detect_format(&file_path).ok_or("Unsupported format")?;
    let bytes = load_channel_source_bytes(&file_path, &format)?;
    Ok(crate::analyzer::channel_split::compute_channels(&bytes))
}

/// Get ICC profile on-demand
#[command]
pub async fn get_icc_profile(file_path: String) -> Result<Option<IccInfo>, String> {
    let format = analyzer::detect_format(&file_path).ok_or("Unsupported format")?;
    let bytes = read_file_bytes(&file_path)?;

    let icc_data = match format {
        ImageFormat::Png => png_parser::extract_icc_data(&bytes),
        ImageFormat::Jpeg => jpeg_parser::extract_icc_data(&bytes),
        ImageFormat::Webp => webp_parser::extract_icc_data(&bytes),
        _ => None,
    };

    Ok(icc_data.and_then(|d| crate::analyzer::icc_parser::parse_icc(&d)))
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

/// Extract all frames from a GIF as base64-encoded PNG thumbnails
#[command]
pub async fn get_gif_frames(file_path: String) -> Result<Vec<GifFrame>, String> {
    let bytes = read_file_bytes(&file_path)?;
    let mut cursor = Cursor::new(bytes);

    let format = image::guess_format(&cursor.get_ref())
        .map_err(|e| format!("Failed to detect image format: {}", e))?;

    if format != ImageCrateFormat::Gif {
        return Err("Not a GIF file".to_string());
    }

    cursor.set_position(0);
    let decoder = GifDecoder::new(cursor).map_err(|e| format!("Failed to decode GIF: {}", e))?;

    let frames = decoder.into_frames();
    let mut result = Vec::new();

    for (idx, frame_result) in frames.enumerate() {
        let frame = frame_result.map_err(|e| format!("Failed to decode frame {}: {}", idx, e))?;

        let delay_ms: u16 = frame
            .delay()
            .numer_denom_ms()
            .0
            .try_into()
            .unwrap_or(u16::MAX);
        let (width, height) = frame.buffer().dimensions();

        // Encode frame as PNG then base64
        let mut png_bytes = Vec::new();
        let mut png_cursor = Cursor::new(&mut png_bytes);
        frame
            .buffer()
            .write_to(&mut png_cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode frame {} as PNG: {}", idx, e))?;

        let image_base64 = base64_standard.encode(&png_bytes);

        result.push(GifFrame {
            index: idx as u32,
            delay_ms,
            width,
            height,
            image_base64,
        });
    }

    Ok(result)
}
