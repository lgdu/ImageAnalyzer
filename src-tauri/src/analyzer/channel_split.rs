use image::{GenericImageView, Pixel};
use crate::types::{ChannelData, Histogram, SingleChannel, RgbChannels, YuvChannels};

pub fn compute_channels(bytes: &[u8]) -> Option<ChannelData> {
    let img = image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();

    let mut r_vals: Vec<u8> = Vec::new();
    let mut g_vals: Vec<u8> = Vec::new();
    let mut b_vals: Vec<u8> = Vec::new();
    let mut a_vals: Vec<u8> = Vec::new();
    let mut has_alpha = false;

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            let channels = pixel.channels();
            if channels.len() >= 3 {
                r_vals.push(channels[0]);
                g_vals.push(channels[1]);
                b_vals.push(channels[2]);
                if channels.len() >= 4 {
                    a_vals.push(channels[3]);
                    if channels[3] < 255 {
                        has_alpha = true;
                    }
                }
            }
        }
    }

    let rgb = RgbChannels {
        r: compute_stats("R", &r_vals),
        g: compute_stats("G", &g_vals),
        b: compute_stats("B", &b_vals),
        a: if has_alpha { Some(compute_stats("A", &a_vals)) } else { None },
    };

    // Convert RGB to YCbCr (BT.709)
    let y_vals: Vec<u8> = r_vals
        .iter()
        .zip(&g_vals)
        .zip(&b_vals)
        .map(|((r, g), b)| {
            (0.2126 * *r as f64 + 0.7152 * *g as f64 + 0.0722 * *b as f64) as u8
        })
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
            (0.5 * *r as f64 - 0.4542 * *g as f64 - 0.0458 * *b as f64 + 128.0)
                .clamp(0.0, 255.0) as u8
        })
        .collect();

    let yuv = YuvChannels {
        y: compute_stats("Y", &y_vals),
        cb: compute_stats("Cb", &cb_vals),
        cr: compute_stats("Cr", &cr_vals),
    };

    let histograms = vec![
        Histogram {
            channel: "R".to_string(),
            bins: compute_histogram(&r_vals),
        },
        Histogram {
            channel: "G".to_string(),
            bins: compute_histogram(&g_vals),
        },
        Histogram {
            channel: "B".to_string(),
            bins: compute_histogram(&b_vals),
        },
    ];

    Some(ChannelData {
        rgb: Some(rgb),
        yuv: Some(yuv),
        histograms,
        thumbnail_base64: None,
        ycbcr_subsampling: None,
        color_matrix: "BT.709".to_string(),
    })
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

fn compute_histogram(vals: &[u8]) -> Vec<u64> {
    let mut bins = vec![0u64; 256];
    for &v in vals {
        bins[v as usize] += 1;
    }
    bins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_histogram_all_same() {
        let vals = vec![128u8; 100];
        let hist = compute_histogram(&vals);
        assert_eq!(hist[128], 100);
        assert_eq!(hist.iter().sum::<u64>(), 100);
    }

    #[test]
    fn compute_histogram_distribution() {
        let vals = vec![0u8, 128, 255];
        let hist = compute_histogram(&vals);
        assert_eq!(hist[0], 1);
        assert_eq!(hist[128], 1);
        assert_eq!(hist[255], 1);
        assert_eq!(hist.iter().sum::<u64>(), 3);
    }

    #[test]
    fn compute_stats_empty() {
        let stats = compute_stats("Test", &[]);
        assert_eq!(stats.min, 0);
        assert_eq!(stats.max, 0);
        assert_eq!(stats.mean, 0.0);
        assert_eq!(stats.median, 0);
        assert_eq!(stats.std_dev, 0.0);
    }

    #[test]
    fn compute_stats_uniform() {
        let vals = vec![100u8; 50];
        let stats = compute_stats("Test", &vals);
        assert_eq!(stats.min, 100);
        assert_eq!(stats.max, 100);
        assert!((stats.mean - 100.0).abs() < f64::EPSILON);
        assert_eq!(stats.median, 100);
        assert!((stats.std_dev - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_stats_range() {
        let vals: Vec<u8> = (0..=255).collect();
        let stats = compute_stats("Full", &vals);
        assert_eq!(stats.min, 0);
        assert_eq!(stats.max, 255);
        assert!((stats.mean - 127.5).abs() < 1.0);
        assert!(stats.std_dev > 70.0);
    }

    #[test]
    fn compute_stats_median_odd() {
        let vals = vec![10u8, 20, 30, 40, 50];
        let stats = compute_stats("Median", &vals);
        assert_eq!(stats.median, 30);
    }

    #[test]
    fn compute_channels_returns_data_for_valid_image() {
        // Create a minimal 2x2 PNG in memory
        let mut buf = Vec::new();
        let mut encoder = png::Encoder::new(&mut buf, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let data = [255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 128, 128, 128, 255];
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();

        let result = compute_channels(&buf);
        assert!(result.is_some());
        let ch = result.unwrap();
        assert!(ch.rgb.is_some());
        assert!(ch.yuv.is_some());
        assert_eq!(ch.histograms.len(), 3);
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
