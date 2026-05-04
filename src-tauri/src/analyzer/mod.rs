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

/// PNG signature: \x89PNG\r\n\x1a\n
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// JPEG signature: \xff\xd8\xff
const JPEG_SIGNATURE: [u8; 3] = [0xFF, 0xD8, 0xFF];
/// WebP signature: RIFF....WEBP
const WEBP_SIGNATURE_RIFF: &[u8; 4] = b"RIFF";
const WEBP_SIGNATURE_WEBP: &[u8; 4] = b"WEBP";
/// GIF signature: GIF89a or GIF87a
const GIF_SIGNATURE_89A: &[u8; 6] = b"GIF89a";
const GIF_SIGNATURE_87A: &[u8; 6] = b"GIF87a";
/// HEIF signature: ftyp (at offset 4)
const HEIF_FTYP: &[u8; 4] = b"ftyp";

pub fn detect_format(path: &str) -> Option<ImageFormat> {
    // First try extension
    let lower = path.to_lowercase();

    // Read first 16 bytes to detect magic
    let mut header = [0u8; 16];
    if let Ok(mut file) = std::fs::File::open(path) {
        if let Ok(()) = std::io::Read::read_exact(&mut file, &mut header) {
            // Check magic bytes
            if header.len() >= 8 && header[..8] == PNG_SIGNATURE {
                return Some(ImageFormat::Png);
            }
            if header.len() >= 3 && header[..3] == JPEG_SIGNATURE {
                return Some(ImageFormat::Jpeg);
            }
            if header.len() >= 12
                && header[..4] == *WEBP_SIGNATURE_RIFF
                && header[8..12] == *WEBP_SIGNATURE_WEBP
            {
                return Some(ImageFormat::Webp);
            }
            if header.len() >= 6
                && (header[..6] == *GIF_SIGNATURE_89A || header[..6] == *GIF_SIGNATURE_87A)
            {
                return Some(ImageFormat::Gif);
            }
            // HEIF/HEIC/AVIF: check for ftyp at offset 4
            if header.len() >= 8 && header[4..8] == *HEIF_FTYP {
                // Check specific HEIF brand by reading more of the header
                // The brand string starts at offset 8
                if let Ok(mut file2) = std::fs::File::open(path) {
                    let mut brand_header = [0u8; 16];
                    if std::io::Read::read_exact(&mut file2, &mut brand_header).is_ok() {
                        // Brand is at offset 8, 4 bytes
                        let brand = &brand_header[8..12];
                        let brand_str = String::from_utf8_lossy(brand);
                        if brand_str.contains("heic") || brand_str.contains("heix") {
                            return Some(ImageFormat::Heic);
                        }
                        if brand_str.contains("avif") || brand_str.contains("avis") {
                            return Some(ImageFormat::Avif);
                        }
                        // Default to Heic for other ftyp
                        return Some(ImageFormat::Heic);
                    }
                }
            }
        }
    }

    // Fallback to extension
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
