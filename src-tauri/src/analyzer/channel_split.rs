use base64::{engine::general_purpose::STANDARD, Engine};
use image::{GenericImageView, RgbaImage};

use crate::types::{ChannelData, RgbChannels, SingleChannel, YuvChannels};

pub fn compute_channels(bytes: &[u8]) -> Option<ChannelData> {
    let img = image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();

    let rgba = img.to_rgba8();

    let mut r_vals: Vec<u8> = Vec::with_capacity((w * h) as usize);
    let mut g_vals: Vec<u8> = Vec::with_capacity((w * h) as usize);
    let mut b_vals: Vec<u8> = Vec::with_capacity((w * h) as usize);
    let mut a_vals: Vec<u8> = Vec::new();
    let mut has_alpha = false;

    for y in 0..h {
        for x in 0..w {
            let pixel = rgba.get_pixel(x, y);
            r_vals.push(pixel[0]);
            g_vals.push(pixel[1]);
            b_vals.push(pixel[2]);
            a_vals.push(pixel[3]);
            if pixel[3] < 255 {
                has_alpha = true;
            }
        }
    }

    let rgb = RgbChannels {
        r: compute_stats("R", &r_vals),
        g: compute_stats("G", &g_vals),
        b: compute_stats("B", &b_vals),
        a: if has_alpha {
            Some(compute_stats("A", &a_vals))
        } else {
            None
        },
    };

    // Convert RGB to YCbCr (BT.709)
    let y_vals: Vec<u8> = r_vals
        .iter()
        .zip(&g_vals)
        .zip(&b_vals)
        .map(|((r, g), b)| (0.2126 * *r as f64 + 0.7152 * *g as f64 + 0.0722 * *b as f64) as u8)
        .collect();

    let cb_vals: Vec<u8> = r_vals
        .iter()
        .zip(&g_vals)
        .zip(&b_vals)
        .map(|((r, g), b)| {
            (128.0 - 0.1146 * *r as f64 - 0.3854 * *g as f64 + 0.5 * *b as f64 + 128.0)
                .clamp(0.0, 255.0) as u8
        })
        .collect();

    let cr_vals: Vec<u8> = r_vals
        .iter()
        .zip(&g_vals)
        .zip(&b_vals)
        .map(|((r, g), b)| {
            (0.5 * *r as f64 - 0.4542 * *g as f64 - 0.0458 * *b as f64 + 128.0).clamp(0.0, 255.0)
                as u8
        })
        .collect();

    let yuv = YuvChannels {
        y: compute_stats("Y", &y_vals),
        cb: compute_stats("Cb", &cb_vals),
        cr: compute_stats("Cr", &cr_vals),
    };

    // Encode image as PNG then base64
    let image_base64 = encode_png_base64(&rgba, w, h);

    Some(ChannelData {
        rgb: Some(rgb),
        yuv: Some(yuv),
        image_base64,
        ycbcr_subsampling: None,
        color_matrix: "BT.709".to_string(),
    })
}

fn encode_png_base64(rgba: &RgbaImage, w: u32, h: u32) -> Option<String> {
    let mut png_buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_buf, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba.as_raw()).ok()?;
    }
    Some(STANDARD.encode(&png_buf))
}

fn compute_stats(name: &str, vals: &[u8]) -> SingleChannel {
    if vals.is_empty() {
        return SingleChannel {
            name: name.to_string(),
            min: 0,
            max: 0,
            mean: 0.0,
            median: 0,
            std_dev: 0.0,
        };
    }
    let mut sorted = vals.to_vec();
    sorted.sort();
    let sum: u64 = vals.iter().map(|&v| v as u64).sum();
    let mean = sum as f64 / vals.len() as f64;
    let variance: f64 =
        vals.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    SingleChannel {
        name: name.to_string(),
        min: sorted[0],
        max: sorted[sorted.len() - 1],
        median: sorted[sorted.len() / 2],
        mean,
        std_dev: variance.sqrt(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_channels_returns_data_for_valid_image() {
        // Create a minimal 2x2 PNG in memory
        let mut buf = Vec::new();
        let mut encoder = png::Encoder::new(&mut buf, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let data = [
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 128, 128, 128, 255,
        ];
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();

        let result = compute_channels(&buf);
        assert!(result.is_some());
        let ch = result.unwrap();
        assert!(ch.rgb.is_some());
        assert!(ch.yuv.is_some());
        assert!(ch.image_base64.is_some());
        assert_eq!(ch.color_matrix, "BT.709");
    }

    #[test]
    fn compute_channels_returns_none_for_invalid() {
        let result = compute_channels(b"not an image");
        assert!(result.is_none());
    }

    #[test]
    fn compute_channels_rgb_no_alpha() {
        // 2x2 RGB (no alpha)
        let mut buf = Vec::new();
        let mut encoder = png::Encoder::new(&mut buf, 2, 2);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let data = [255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();

        let result = compute_channels(&buf).unwrap();
        assert!(result.rgb.as_ref().unwrap().a.is_none());
    }
}
