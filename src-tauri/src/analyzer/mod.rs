use crate::types::ImageFormat;

pub mod av1;
pub mod channel_split;
pub mod exif_reader;
pub mod gif_parser;
pub mod heif_parser;
pub mod hevc;
pub mod icc_parser;
pub mod iptc_reader;
pub mod jpeg_parser;
pub mod png_parser;
pub mod webp_parser;
pub mod xmp_reader;

pub fn detect_format(path: &str) -> Option<ImageFormat> {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        Some(ImageFormat::Png)
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some(ImageFormat::Jpeg)
    } else if lower.ends_with(".webp") {
        Some(ImageFormat::Webp)
    } else if lower.ends_with(".gif") {
        Some(ImageFormat::Gif)
    } else if lower.ends_with(".avif") {
        Some(ImageFormat::Avif)
    } else if lower.ends_with(".heic") || lower.ends_with(".heif") {
        Some(ImageFormat::Heic)
    } else {
        None
    }
}
