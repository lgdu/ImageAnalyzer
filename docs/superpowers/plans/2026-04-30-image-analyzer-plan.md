# ImageAnalyzer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A desktop image analysis application built with Tauri 2 + Svelte 5 + Rust, supporting PNG, JPEG, WebP, GIF, AVIF, HEIC with file structure analysis, metadata extraction (EXIF/XMP/IPTC/ICC), channel visualization (RGB + YUV), and HEVC/AV1 codec syntax display.

**Architecture:** Tauri 2 desktop app. Rust backend handles all file parsing and analysis via Tauri IPC commands. Svelte 5 frontend renders the analysis results in a tabbed interface with a left sidebar for file management.

**Tech Stack:** Tauri 2, Svelte 5, TypeScript, Rust, `image`, `kamadak-exif`, `png`, `mp4parse`, `serde`, `quick-xml`, `byteorder`

---

## File Structure Overview

```
ImageAnalyzer/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── gen/                  # Tauri generated (auto)
│   ├── target/               # Build output (auto)
│   ├── icons/                # App icons
│   └── src/
│       ├── main.rs           # Tauri entry
│       ├── commands.rs       # Tauri commands
│       ├── types.rs          # All data model structs
│       ├── utils.rs          # Hex dump, byte helpers
│       └── analyzer/
│           ├── mod.rs        # Analyzer dispatcher
│           ├── png_parser.rs
│           ├── jpeg_parser.rs
│           ├── webp_parser.rs
│           ├── gif_parser.rs
│           ├── heif_parser.rs
│           ├── exif_reader.rs
│           ├── xmp_reader.rs
│           ├── iptc_reader.rs
│           ├── channel_split.rs
│           ├── icc_parser.rs
│           ├── hevc/
│           │   ├── mod.rs
│           │   ├── nalu.rs
│           │   ├── vps.rs
│           │   ├── sps.rs
│           │   ├── pps.rs
│           │   └── slice_header.rs
│           └── av1/
│               ├── mod.rs
│               ├── obu.rs
│               ├── sequence_header.rs
│               ├── frame_header.rs
│               └── tile_info.rs
├── src/
│   ├── app.html
│   ├── app.d.ts
│   ├── lib/
│   │   ├── types.ts
│   │   ├── store.ts
│   │   └── utils.ts
│   ├── components/
│   │   ├── DropZone.svelte
│   │   ├── FileList.svelte
│   │   ├── ThumbnailCard.svelte
│   │   ├── MainPanel.svelte
│   │   ├── tabs/
│   │   │   ├── StructureTab.svelte
│   │   │   ├── MetadataTab.svelte
│   │   │   ├── ChannelsTab.svelte
│   │   │   ├── ColorInfoTab.svelte
│   │   │   ├── CodecSyntaxTab.svelte
│   │   │   └── GridTab.svelte
│   │   └── ui/
│   │       ├── TreeView.svelte
│   │       ├── TreeItem.svelte
│   │       ├── DataTable.svelte
│   │       ├── Histogram.svelte
│   │       ├── HexDump.svelte
│   │       └── GridView.svelte
│   └── App.svelte
├── static/
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tsconfig.json
├── .gitignore
└── docs/
    ├── superpowers/specs/2026-04-30-image-analyzer-design.md
    └── superpowers/plans/2026-04-30-image-analyzer-plan.md
```

---

## Phase 1: Project Scaffolding + Types + PNG + Basic Shell

### Task 1: Project Scaffolding & Rust Types

**Files:**
- Create: `package.json`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `src/app.html`, `src/app.d.ts`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/types.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create frontend package.json**

```json
{
  "name": "image-analyzer",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "tauri": "tauri"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^5.0.0",
    "@tauri-apps/cli": "^2",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0"
  }
}
```

- [ ] **Step 2: Create svelte.config.js**

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    runes: true
  }
};
```

- [ ] **Step 3: Create vite.config.ts**

```ts
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] }
  }
});
```

- [ ] **Step 4: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"]
}
```

- [ ] **Step 5: Create src/app.html**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>ImageAnalyzer</title>
    %sveltekit.head%
  </head>
  <body>
    <div style="display: contents">%sveltekit.body%</div>
  </body>
</html>
```

- [ ] **Step 6: Create src/app.d.ts**

```ts
/// <reference types="@sveltejs/kit" />
```

- [ ] **Step 7: Create src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/refs/heads/v2/crates/tauri-config-schema/schema.json",
  "productName": "ImageAnalyzer",
  "version": "0.1.0",
  "identifier": "com.imageanalyzer.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "ImageAnalyzer",
        "width": 1280,
        "height": 800,
        "resizable": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 8: Create src-tauri/capabilities/default.json**

```json
{
  "identifier": "default",
  "description": "Default capability for file analysis",
  "windows": ["main"],
  "permissions": [
    "core:default",
    {
      "identifier": "fs:allow-read",
      "allow": [{ "path": "**" }]
    }
  ]
}
```

- [ ] **Step 9: Create src-tauri/Cargo.toml**

```toml
[package]
name = "image-analyzer"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
image = "0.25"
kamadak-exif = "0.6"
png = "0.17"
byteorder = "1"
mp4parse = "0.42"
quick-xml = "0.37"

[features]
custom-protocol = ["tauri/custom-protocol"]
```

- [ ] **Step 10: Create .gitignore**

```
# Build
dist/
src-tauri/target/
node_modules/

# Tauri
src-tauri/gen/
src-tauri/icons/

# OS
.DS_Store
Thumbs.db

# IDE
.vscode/
.idea/

# Brainstorming
.superpowers/
```

- [ ] **Step 11: Write Rust types (src-tauri/src/types.rs)**

```rust
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
    Avif,
    Heic,
}

#[derive(Serialize)]
pub struct ImageAnalysis {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub color_type: String,
    pub bit_depth: u8,
    pub has_alpha: bool,
    pub structure: Vec<FileBlock>,
    pub metadata: Vec<MetadataEntry>,
    pub channels: Option<ChannelData>,
    pub icc_profile: Option<IccInfo>,
    pub codec_syntax: Option<CodecSyntax>,
    pub grid: Option<GridInfo>,
    pub analysis_errors: Vec<String>,
}

#[derive(Serialize)]
pub struct FileBlock {
    pub name: String,
    pub offset: u64,
    pub length: u64,
    pub data_preview: Option<String>,
    pub decoded_info: Option<String>,
    pub children: Vec<FileBlock>,
}

#[derive(Serialize)]
pub struct MetadataEntry {
    pub standard: String,
    pub tag_name: String,
    pub tag_value: String,
    pub raw_value: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Rgb,
    Yuv,
}

#[derive(Serialize)]
pub struct ChannelData {
    pub rgb: Option<RgbChannels>,
    pub yuv: Option<YuvChannels>,
    pub histograms: Vec<Histogram>,
    pub thumbnail_base64: Option<String>,
    pub ycbcr_subsampling: Option<String>,
    pub color_matrix: String,
}

#[derive(Serialize)]
pub struct RgbChannels {
    pub r: SingleChannel,
    pub g: SingleChannel,
    pub b: SingleChannel,
    pub a: Option<SingleChannel>,
}

#[derive(Serialize)]
pub struct YuvChannels {
    pub y: SingleChannel,
    pub cb: SingleChannel,
    pub cr: SingleChannel,
}

#[derive(Serialize)]
pub struct SingleChannel {
    pub name: String,
    pub min: u8,
    pub max: u8,
    pub mean: f64,
    pub median: u8,
    pub std_dev: f64,
}

#[derive(Serialize)]
pub struct Histogram {
    pub channel: String,
    pub bins: Vec<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecSyntax {
    Hevc(HevcSyntax),
    Av1(Av1Syntax),
}

#[derive(Serialize)]
pub struct HevcSyntax {
    pub nal_units: Vec<NalUnit>,
    pub vps: Option<VideoParameterSet>,
    pub sps: Option<SequenceParameterSet>,
    pub pps: Option<PictureParameterSet>,
    pub slice_headers: Vec<HevcSliceHeader>,
}

#[derive(Serialize)]
pub struct NalUnit {
    pub nal_type: String,
    pub nuh_layer_id: u8,
    pub nuh_temporal_id: u8,
    pub size: usize,
    pub offset: u64,
}

#[derive(Serialize)]
pub struct HevcSliceHeader {
    pub slice_type: u8,
    pub first_slice_segment_in_pic_flag: bool,
    pub dependent_slice_segment_flag: bool,
    pub slice_segment_address: u32,
    pub pps_id: u8,
    pub num_entry_point_offsets: Option<u32>,
    pub offset_len_minus1: Option<u8>,
    pub pic_width: u32,
    pub pic_height: u32,
    pub tile_enabled: bool,
}

#[derive(Serialize)]
pub struct VideoParameterSet {
    pub vps_id: u8,
    pub max_layers: u8,
    pub max_sub_layers: u8,
}

#[derive(Serialize)]
pub struct SequenceParameterSet {
    pub profile: String,
    pub level: String,
    pub chroma_format: String,
    pub pic_width: u32,
    pub pic_height: u32,
    pub bit_depth: u8,
}

#[derive(Serialize)]
pub struct PictureParameterSet {
    pub pps_id: u8,
    pub sps_id: u8,
}

#[derive(Serialize)]
pub struct Av1Syntax {
    pub obus: Vec<Obu>,
    pub sequence_header: Option<SequenceHeader>,
    pub frame_headers: Vec<Av1FrameHeader>,
    pub tile_info: Option<Av1TileInfo>,
}

#[derive(Serialize)]
pub struct Obu {
    pub obu_type: String,
    pub obu_size: usize,
    pub temporal_id: u8,
    pub spatial_id: u8,
    pub offset: u64,
}

#[derive(Serialize)]
pub struct SequenceHeader {
    pub profile: u8,
    pub level: String,
    pub bit_depth: u8,
    pub chroma_format: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub color_config: Option<ColorConfig>,
}

#[derive(Serialize)]
pub struct ColorConfig {
    pub matrix_coefficients: String,
}

#[derive(Serialize)]
pub struct Av1FrameHeader {
    pub frame_type: String,
    pub show_frame: bool,
    pub frame_size: Option<(u32, u32)>,
    pub order_hint: u8,
    pub primary_ref_frame: u8,
    pub quantizer_params: Option<QuantizerParams>,
}

#[derive(Serialize)]
pub struct QuantizerParams {
    pub base_q_idx: u8,
    pub delta_q_present: bool,
    pub delta_q_res: u8,
}

#[derive(Serialize)]
pub struct Av1TileInfo {
    pub num_tiles: u32,
    pub rows: u32,
    pub cols: u32,
    pub tile_width: Vec<u32>,
    pub tile_height: Vec<u32>,
    pub context_update_tile_id: Option<u32>,
}

#[derive(Serialize)]
pub struct GridInfo {
    pub rows: u32,
    pub cols: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub tiles: Vec<GridTile>,
}

#[derive(Serialize)]
pub struct GridTile {
    pub item_id: u16,
    pub width: u32,
    pub height: u32,
    pub horizontal_offset: u32,
    pub vertical_offset: u32,
    pub codec: String,
}

#[derive(Serialize)]
pub struct IccInfo {
    pub size: u32,
    pub cmm_type: String,
    pub version: String,
    pub profile_class: String,
    pub color_space: String,
    pub pcs: String,
    pub platform: Option<String>,
    pub rendering_intent: String,
    pub illuminant: (f64, f64, f64),
    pub creator: Option<String>,
    pub description: Option<String>,
    pub transfer_function: Option<String>,
    pub red_trc: Option<String>,
    pub green_trc: Option<String>,
    pub blue_trc: Option<String>,
    pub primaries: Option<PrimariesInfo>,
    pub matrix: Option<Matrix3x3>,
    pub luts: Vec<LutInfo>,
    pub tag_count: u32,
    pub tags: Vec<IccTag>,
    pub raw_base64: Option<String>,
}

#[derive(Serialize)]
pub struct PrimariesInfo {
    pub red_x: f64,
    pub red_y: f64,
    pub red_z: f64,
    pub green_x: f64,
    pub green_y: f64,
    pub green_z: f64,
    pub blue_x: f64,
    pub blue_y: f64,
    pub blue_z: f64,
    pub white_x: f64,
    pub white_y: f64,
    pub white_z: f64,
}

#[derive(Serialize)]
pub struct Matrix3x3 {
    pub m: [[f64; 3]; 3],
}

#[derive(Serialize)]
pub struct LutInfo {
    pub name: String,
    pub clut_points: Option<u8>,
    pub input_channels: u8,
    pub output_channels: u8,
}

#[derive(Serialize)]
pub struct IccTag {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub tag_type: String,
    pub decoded_value: Option<String>,
}
```

- [ ] **Step 12: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: PASS (no compile errors)

- [ ] **Step 13: Commit**

```bash
git add -A
git commit -m "feat: scaffold project with Tauri 2 + Svelte 5 + Rust types"
```

---

### Task 2: PNG Parser + Tauri Command

**Files:**
- Create: `src-tauri/src/utils.rs`
- Create: `src-tauri/src/analyzer/mod.rs`
- Create: `src-tauri/src/analyzer/png_parser.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/tests/png_test.rs`

- [ ] **Step 1: Write utils.rs**

```rust
use std::fs;

pub fn read_file_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Failed to read file: {}", e))
}

pub fn bytes_to_hex(bytes: &[u8], max_len: usize) -> String {
    let limited = if bytes.len() > max_len {
        &bytes[..max_len]
    } else {
        bytes
    };
    limited
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
```

- [ ] **Step 2: Write analyzer/mod.rs**

```rust
pub mod png_parser;
pub mod jpeg_parser;
pub mod webp_parser;
pub mod gif_parser;
pub mod heif_parser;
pub mod exif_reader;
pub mod xmp_reader;
pub mod iptc_reader;
pub mod channel_split;
pub mod icc_parser;
pub mod hevc;
pub mod av1;

use crate::types::{ImageAnalysis, ImageFormat};

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
```

- [ ] **Step 3: Write PNG parser (src-tauri/src/analyzer/png_parser.rs)**

```rust
use std::io::Cursor;
use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{read_file_bytes, bytes_to_hex};

pub fn analyze_png(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_name = path.split('/').last().unwrap_or("unknown");

    let mut reader = Cursor::new(&bytes);
    let decoder = png::Decoder::new(&mut reader);
    let (info, _) = decoder.read_info().map_err(|e| format!("Invalid PNG: {}", e))?;

    let mut structure = Vec::new();
    let mut metadata = Vec::new();
    let mut errors = Vec::new();

    // Parse PNG chunks manually from raw bytes
    // PNG signature is 8 bytes
    if bytes.len() < 8 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("Invalid PNG signature".to_string());
    }

    let mut offset = 8;
    while offset + 8 <= bytes.len() {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;

        let name_bytes = &bytes[offset + 4..offset + 8];
        let name = String::from_utf8_lossy(name_bytes).to_string();

        let chunk_data_start = offset + 8;
        let chunk_total = 4 + 4 + length + 4; // length + name + data + crc

        let data_preview = if length > 0 {
            let end = std::cmp::min(chunk_data_start + 16, chunk_data_start + length);
            Some(bytes_to_hex(&bytes[chunk_data_start..end], 16))
        } else {
            None
        };

        let decoded_info = match name.as_str() {
            "IHDR" => {
                if length >= 13 {
                    let w = u32::from_be_bytes([
                        bytes[chunk_data_start],
                        bytes[chunk_data_start + 1],
                        bytes[chunk_data_start + 2],
                        bytes[chunk_data_start + 3],
                    ]);
                    let h = u32::from_be_bytes([
                        bytes[chunk_data_start + 4],
                        bytes[chunk_data_start + 5],
                        bytes[chunk_data_start + 6],
                        bytes[chunk_data_start + 7],
                    ]);
                    let bit_depth = bytes[chunk_data_start + 8];
                    let color_type = bytes[chunk_data_start + 9];
                    Some(format!(
                        "{}x{}, {}-bit, color_type={}",
                        w, h, bit_depth, color_type
                    ))
                } else {
                    Some("Invalid IHDR".to_string())
                }
            }
            "IDAT" => Some(format!("{} bytes compressed data", length)),
            "tEXt" | "zTXt" | "iTXt" => {
                if let Ok(text) = parse_png_text(&bytes[chunk_data_start..chunk_data_start + length.min(bytes.len() - chunk_data_start)], &name) {
                    metadata.push(MetadataEntry {
                        standard: "PNG Text".to_string(),
                        tag_name: name.clone(),
                        tag_value: text,
                        raw_value: None,
                    });
                    None
                } else {
                    None
                }
            }
            "PLTE" => Some(format!("{} entries", length / 3)),
            "gAMA" => Some(format!("gamma={}", {
                if length >= 4 {
                    u32::from_be_bytes([
                        bytes[chunk_data_start],
                        bytes[chunk_data_start + 1],
                        bytes[chunk_data_start + 2],
                        bytes[chunk_data_start + 3],
                    ]) as f64 / 100000.0
                } else {
                    0.0
                }
            })),
            _ => None,
        };

        structure.push(FileBlock {
            name,
            offset: offset as u64,
            length: chunk_total as u64,
            data_preview,
            decoded_info,
            children: Vec::new(),
        });

        offset += chunk_total;

        // IEND marks end
        if structure.last().map(|b| b.name.as_str()) == Some("IEND") {
            break;
        }
    }

    let (width, height, color_type, bit_depth, has_alpha) = if let Some(ihdr) = structure.iter().find(|b| b.name == "IHDR") {
        let ct = if let Some(ref info) = ihdr.decoded_info {
            parse_ihdr_info(&info)
        } else {
            (0, 0, "unknown".to_string(), 0, false)
        };
        ct
    } else {
        (0, 0, "unknown".to_string(), 0, false)
    };

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size: bytes.len() as u64,
        format: ImageFormat::Png,
        width,
        height,
        color_type,
        bit_depth,
        has_alpha,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}

fn parse_png_text(data: &[u8], chunk_type: &str) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    if chunk_type == "tEXt" {
        if let Some(null_pos) = data.iter().position(|&b| b == 0) {
            let keyword = String::from_utf8_lossy(&data[..null_pos]);
            let text = String::from_utf8_lossy(&data[null_pos + 1..]);
            Some(format!("{}: {}", keyword, text))
        } else {
            Some(String::from_utf8_lossy(data).to_string())
        }
    } else {
        Some(String::from_utf8_lossy(data).to_string())
    }
}

fn parse_ihdr_info(decoded: &str) -> (u32, u32, String, u8, bool) {
    // Parse "WxH, X-bit, color_type=Y"
    let parts: Vec<&str> = decoded.split(", ").collect();
    let dims = parts[0].split('x').collect::<Vec<_>>();
    let w: u32 = dims[0].parse().unwrap_or(0);
    let h: u32 = dims[1].parse().unwrap_or(0);

    let bit_depth_str = parts[1].split('-').next().unwrap_or("0");
    let bit_depth: u8 = bit_depth_str.parse().unwrap_or(0);

    let color_type_num: u8 = if parts.len() > 2 {
        parts[2].split('=').nth(1).unwrap_or("0").parse().unwrap_or(0)
    } else {
        0
    };

    let color_type = match color_type_num {
        0 => "Grayscale",
        2 => "RGB",
        3 => "Indexed",
        4 => "Grayscale+Alpha",
        6 => "RGBA",
        _ => "Unknown",
    }.to_string();

    let has_alpha = color_type_num == 4 || color_type_num == 6;

    (w, h, color_type, bit_depth, has_alpha)
}
```

- [ ] **Step 4: Write commands.rs**

```rust
use tauri::command;
use crate::analyzer::{self, png_parser};
use crate::types::ImageAnalysis;

#[command]
pub async fn analyze_image(file_path: String) -> Result<ImageAnalysis, String> {
    let format = analyzer::detect_format(&file_path)
        .ok_or_else(|| format!("Unsupported format: {}", file_path))?;

    match format {
        analyzer::ImageFormat::Png => png_parser::analyze_png(&file_path),
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
```

- [ ] **Step 5: Write main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analyzer;
mod commands;
mod types;
mod utils;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::analyze_image,
            commands::analyze_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: PASS (may have unused warnings, that's fine)

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: add PNG parser and analyze_image Tauri command"
```

---

### Task 3: Basic Frontend Shell + DropZone

**Files:**
- Create: `src/App.svelte`
- Create: `src/lib/types.ts`
- Create: `src/lib/store.ts`
- Create: `src/lib/utils.ts`
- Create: `src/components/DropZone.svelte`
- Create: `src/components/FileList.svelte`
- Create: `src/components/ThumbnailCard.svelte`
- Create: `src/components/MainPanel.svelte`

- [ ] **Step 1: Write TypeScript types (src/lib/types.ts)**

```ts
export interface ImageAnalysis {
  file_name: string;
  file_path: string;
  file_size: number;
  format: string;
  width: number;
  height: number;
  color_type: string;
  bit_depth: number;
  has_alpha: boolean;
  structure: FileBlock[];
  metadata: MetadataEntry[];
  channels: ChannelData | null;
  icc_profile: IccInfo | null;
  codec_syntax: CodecSyntax | null;
  grid: GridInfo | null;
  analysis_errors: string[];
}

export interface FileBlock {
  name: string;
  offset: number;
  length: number;
  data_preview: string | null;
  decoded_info: string | null;
  children: FileBlock[];
}

export interface MetadataEntry {
  standard: string;
  tag_name: string;
  tag_value: string;
  raw_value: string | null;
}

export interface ChannelData {
  rgb: RgbChannels | null;
  yuv: YuvChannels | null;
  histograms: Histogram[];
  thumbnail_base64: string | null;
  ycbcr_subsampling: string | null;
  color_matrix: string;
}

export interface RgbChannels {
  r: SingleChannel;
  g: SingleChannel;
  b: SingleChannel;
  a: SingleChannel | null;
}

export interface YuvChannels {
  y: SingleChannel;
  cb: SingleChannel;
  cr: SingleChannel;
}

export interface SingleChannel {
  name: string;
  min: number;
  max: number;
  mean: number;
  median: number;
  std_dev: number;
}

export interface Histogram {
  channel: string;
  bins: number[];
}

export type CodecSyntax =
  | { hevc: HevcSyntax }
  | { av1: Av1Syntax };

export interface HevcSyntax {
  nal_units: NalUnit[];
  vps: VideoParameterSet | null;
  sps: SequenceParameterSet | null;
  pps: PictureParameterSet | null;
  slice_headers: HevcSliceHeader[];
}

export interface NalUnit {
  nal_type: string;
  nuh_layer_id: number;
  nuh_temporal_id: number;
  size: number;
  offset: number;
}

export interface HevcSliceHeader {
  slice_type: number;
  first_slice_segment_in_pic_flag: boolean;
  dependent_slice_segment_flag: boolean;
  slice_segment_address: number;
  pps_id: number;
  num_entry_point_offsets: number | null;
  offset_len_minus1: number | null;
  pic_width: number;
  pic_height: number;
  tile_enabled: boolean;
}

export interface VideoParameterSet {
  vps_id: number;
  max_layers: number;
  max_sub_layers: number;
}

export interface SequenceParameterSet {
  profile: string;
  level: string;
  chroma_format: string;
  pic_width: number;
  pic_height: number;
  bit_depth: number;
}

export interface PictureParameterSet {
  pps_id: number;
  sps_id: number;
}

export interface Av1Syntax {
  obus: Obu[];
  sequence_header: SequenceHeader | null;
  frame_headers: Av1FrameHeader[];
  tile_info: Av1TileInfo | null;
}

export interface Obu {
  obu_type: string;
  obu_size: number;
  temporal_id: number;
  spatial_id: number;
  offset: number;
}

export interface SequenceHeader {
  profile: number;
  level: string;
  bit_depth: number;
  chroma_format: string;
  frame_width: number;
  frame_height: number;
  color_config: ColorConfig | null;
}

export interface ColorConfig {
  matrix_coefficients: string;
}

export interface Av1FrameHeader {
  frame_type: string;
  show_frame: boolean;
  frame_size: [number, number] | null;
  order_hint: number;
  primary_ref_frame: number;
  quantizer_params: QuantizerParams | null;
}

export interface QuantizerParams {
  base_q_idx: number;
  delta_q_present: boolean;
  delta_q_res: number;
}

export interface Av1TileInfo {
  num_tiles: number;
  rows: number;
  cols: number;
  tile_width: number[];
  tile_height: number[];
  context_update_tile_id: number | null;
}

export interface GridInfo {
  rows: number;
  cols: number;
  output_width: number;
  output_height: number;
  tiles: GridTile[];
}

export interface GridTile {
  item_id: number;
  width: number;
  height: number;
  horizontal_offset: number;
  vertical_offset: number;
  codec: string;
}

export interface IccInfo {
  size: number;
  cmm_type: string;
  version: string;
  profile_class: string;
  color_space: string;
  pcs: string;
  platform: string | null;
  rendering_intent: string;
  illuminant: [number, number, number];
  creator: string | null;
  description: string | null;
  transfer_function: string | null;
  red_trc: string | null;
  green_trc: string | null;
  blue_trc: string | null;
  primaries: PrimariesInfo | null;
  matrix: Matrix3x3 | null;
  luts: LutInfo[];
  tag_count: number;
  tags: IccTag[];
  raw_base64: string | null;
}

export interface PrimariesInfo {
  red_x: number; red_y: number; red_z: number;
  green_x: number; green_y: number; green_z: number;
  blue_x: number; blue_y: number; blue_z: number;
  white_x: number; white_y: number; white_z: number;
}

export interface Matrix3x3 {
  m: number[][];
}

export interface LutInfo {
  name: string;
  clut_points: number | null;
  input_channels: number;
  output_channels: number;
}

export interface IccTag {
  name: string;
  offset: number;
  size: number;
  tag_type: string;
  decoded_value: string | null;
}
```

- [ ] **Step 2: Write store (src/lib/store.ts)**

```ts
import { writable } from 'svelte/store';
import type { ImageAnalysis } from './types';

export const fileList = writable<ImageAnalysis[]>([]);
export const currentImage = writable<ImageAnalysis | null>(null);
export const isAnalyzing = writable(false);
export const error = writable<string | null>(null);
```

- [ ] **Step 3: Write utils (src/lib/utils.ts)**

```ts
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatHex(data: string | null, perRow: number = 16): string[] {
  if (!data) return [];
  const bytes = data.split(' ');
  const rows: string[] = [];
  for (let i = 0; i < bytes.length; i += perRow) {
    rows.push(bytes.slice(i, i + perRow).join(' '));
  }
  return rows;
}
```

- [ ] **Step 4: Write DropZone (src/components/DropZone.svelte)**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { fileList, currentImage, isAnalyzing, error } from '../lib/store';
  import type { ImageAnalysis } from '../lib/types';

  let isDragOver = $state(false);

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    const files = Array.from(e.dataTransfer?.files || []);
    const imageFiles = files.filter(f =>
      /\.(png|jpg|jpeg|webp|gif|avif|heic|heif)$/i.test(f.name)
    );
    if (imageFiles.length === 0) {
      error.set('No supported image files');
      return;
    }
    for (const file of imageFiles) {
      await analyzeFile(file.path);
    }
  }

  async function handleDragOver(e: DragEvent) {
    e.preventDefault();
    isDragOver = true;
  }

  async function handleDragLeave() {
    isDragOver = false;
  }

  async function handleFileSelect() {
    const selected = await open({
      multiple: true,
      filters: [{
        name: 'Images',
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif', 'heic', 'heif']
      }]
    });
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      for (const path of paths) {
        await analyzeFile(path);
      }
    }
  }

  async function analyzeFile(path: string) {
    isAnalyzing.set(true);
    error.set(null);
    try {
      const result = await invoke<ImageAnalysis>('analyze_image', { filePath: path });
      const existing = $state.snapshot(fileList);
      if (!existing.find(f => f.file_path === result.file_path)) {
        fileList.update(files => [...files, result]);
      }
      currentImage.set(result);
    } catch (e: any) {
      error.set(e.toString());
    } finally {
      isAnalyzing.set(false);
    }
  }
</script>

<div
  class="dropzone"
  class:drag-over={isDragOver}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  on:click={handleFileSelect}
>
  <div class="dropzone-content">
    <p class="dropzone-title">Drop images here or click to select</p>
    <p class="dropzone-formats">PNG · JPEG · WebP · GIF · AVIF · HEIC</p>
  </div>
</div>

<style>
  .dropzone {
    padding: 2rem;
    border: 2px dashed var(--color-border, #333);
    border-radius: 8px;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
  }
  .dropzone:hover, .drag-over {
    border-color: var(--color-accent, #6366f1);
    background: rgba(99, 102, 241, 0.05);
  }
  .dropzone-content {
    text-align: center;
  }
  .dropzone-title {
    color: var(--color-text, #e2e8f0);
    font-size: 1rem;
    margin: 0;
  }
  .dropzone-formats {
    color: var(--color-muted, #64748b);
    font-size: 0.75rem;
    margin: 0.5rem 0 0;
  }
</style>
```

- [ ] **Step 5: Write ThumbnailCard (src/components/ThumbnailCard.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../lib/types';
  import { formatBytes } from '../lib/utils';

  let { image, isSelected = false }: { image: ImageAnalysis; isSelected?: boolean } = $props();
</script>

<div class="card" class:active={isSelected}>
  <div class="card-info">
    <span class="card-name">{image.file_name}</span>
    <span class="card-details">
      {image.width}×{image.height} · {image.format.toUpperCase()} · {formatBytes(image.file_size)}
    </span>
  </div>
</div>

<style>
  .card {
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s;
  }
  .card:hover {
    background: var(--color-hover, rgba(255,255,255,0.05));
  }
  .card.active {
    background: var(--color-active, rgba(99, 102, 241, 0.2));
  }
  .card-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .card-name {
    font-size: 0.875rem;
    color: var(--color-text, #e2e8f0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-details {
    font-size: 0.7rem;
    color: var(--color-muted, #64748b);
  }
</style>
```

- [ ] **Step 6: Write FileList (src/components/FileList.svelte)**

```svelte
<script lang="ts">
  import { fileList, currentImage } from '../lib/store';
  import ThumbnailCard from './ThumbnailCard.svelte';

  function selectImage(image: any) {
    currentImage.set(image);
  }
</script>

<div class="file-list">
  {#each $fileList as image}
    <ThumbnailCard
      {image}
      isSelected={$currentImage?.file_path === image.file_path}
      onclick={() => selectImage(image)}
    />
  {/each}
</div>

<style>
  .file-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-top: 1rem;
    overflow-y: auto;
    flex: 1;
  }
</style>
```

- [ ] **Step 7: Write MainPanel (src/components/MainPanel.svelte)**

```svelte
<script lang="ts">
  import { currentImage } from '../lib/store';
  import StructureTab from './tabs/StructureTab.svelte';

  let activeTab = $state('structure');

  const tabs = [
    { id: 'structure', label: 'Structure' },
    { id: 'metadata', label: 'Metadata' },
    { id: 'channels', label: 'Channels' },
    { id: 'color', label: 'Color Info' },
    { id: 'codec', label: 'Codec Syntax' },
    { id: 'grid', label: 'Grid' },
  ];
</script>

<div class="main-panel">
  {#if $currentImage}
    <div class="tab-bar">
      {#each tabs as tab}
        <button
          class="tab"
          class:active={activeTab === tab.id}
          on:click={() => activeTab = tab.id}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="tab-content">
      {#if activeTab === 'structure'}
        <StructureTab data={$currentImage} />
      {:else if activeTab === 'metadata'}
        <p class="placeholder">Metadata tab — coming next</p>
      {:else if activeTab === 'channels'}
        <p class="placeholder">Channels tab — coming next</p>
      {:else if activeTab === 'color'}
        <p class="placeholder">Color Info tab — coming next</p>
      {:else if activeTab === 'codec'}
        <p class="placeholder">Codec Syntax tab — coming next</p>
      {:else if activeTab === 'grid'}
        <p class="placeholder">Grid tab — coming next</p>
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      <p>No image selected. Drop an image to begin analysis.</p>
    </div>
  {/if}
</div>

<style>
  .main-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-surface, #0f172a);
  }
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--color-border, #334155);
    overflow-x: auto;
  }
  .tab {
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    color: var(--color-muted, #64748b);
    font-size: 0.8rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.15s, border-color 0.15s;
    white-space: nowrap;
  }
  .tab:hover {
    color: var(--color-text, #e2e8f0);
  }
  .tab.active {
    color: var(--color-accent, #6366f1);
    border-bottom-color: var(--color-accent, #6366f1);
  }
  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }
  .empty-state, .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    color: var(--color-muted, #64748b);
  }
</style>
```

- [ ] **Step 8: Write App.svelte**

```svelte
<script lang="ts">
  import DropZone from './components/DropZone.svelte';
  import FileList from './components/FileList.svelte';
  import MainPanel from './components/MainPanel.svelte';
  import { isAnalyzing, error } from './lib/store';
</script>

<div class="app">
  <aside class="sidebar">
    <DropZone />
    <FileList />
  </aside>
  <main class="content">
    {#if $isAnalyzing}
      <div class="loading">Analyzing...</div>
    {/if}
    {#if $error}
      <div class="error-banner">{$error}</div>
    {/if}
    <MainPanel />
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--color-bg, #0a0e1a);
    color: var(--color-text, #e2e8f0);
  }
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .sidebar {
    width: 30%;
    min-width: 280px;
    max-width: 360px;
    border-right: 1px solid var(--color-border, #1e293b);
    padding: 1rem;
    display: flex;
    flex-direction: column;
    background: var(--color-surface, #0f172a);
  }
  .content {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .loading {
    position: absolute;
    top: 0.5rem;
    right: 1rem;
    padding: 0.5rem 1rem;
    background: var(--color-accent, #6366f1);
    color: white;
    border-radius: 4px;
    font-size: 0.8rem;
    z-index: 100;
  }
  .error-banner {
    position: absolute;
    top: 0.5rem;
    right: 1rem;
    padding: 0.5rem 1rem;
    background: #dc2626;
    color: white;
    border-radius: 4px;
    font-size: 0.8rem;
    z-index: 100;
  }
</style>
```

- [ ] **Step 9: Write placeholder StructureTab (src/components/tabs/StructureTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';

  let { data }: { data: ImageAnalysis } = $props();
</script>

<div class="structure-tab">
  <h3>File Structure — {data.file_name}</h3>
  <p>{data.structure.length} blocks detected</p>
  <ul>
    {#each data.structure as block}
      <li>
        <code>{block.name}</code>
        <span>offset: {block.offset}, size: {block.length}</span>
        {#if block.decoded_info}
          <span class="decoded">{block.decoded_info}</span>
        {/if}
      </li>
    {/each}
  </ul>
</div>

<style>
  .structure-tab {
    font-size: 0.875rem;
  }
  ul {
    list-style: none;
    padding: 0;
  }
  li {
    padding: 0.35rem 0;
    border-bottom: 1px solid var(--color-border, #1e293b);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.8rem;
  }
  code {
    color: var(--color-accent, #818cf8);
    font-weight: 600;
  }
  .decoded {
    color: var(--color-muted, #64748b);
    margin-left: 0.5rem;
  }
</style>
```

- [ ] **Step 10: Install dependencies and verify build**

```bash
pnpm install
cd src-tauri && cargo build
```

Expected: BUILD SUCCESS

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "feat: add frontend shell with drop zone, file list, and structure tab"
```

---

## Phase 2: JPEG, WebP, GIF + Metadata + Channels

### Task 4: JPEG Parser + EXIF + XMP + IPTC

**Files:**
- Create: `src-tauri/src/analyzer/jpeg_parser.rs`
- Create: `src-tauri/src/analyzer/exif_reader.rs`
- Create: `src-tauri/src/analyzer/xmp_reader.rs`
- Create: `src-tauri/src/analyzer/iptc_reader.rs`
- Create: `src-tauri/tests/jpeg_test.rs`

- [ ] **Step 1: Write EXIF reader (src-tauri/src/analyzer/exif_reader.rs)**

```rust
use exif::Reader;
use crate::types::MetadataEntry;
use std::io::Cursor;

pub fn extract_exif(bytes: &[u8]) -> Vec<MetadataEntry> {
    let mut entries = Vec::new();
    let reader = Reader::new();
    if let Ok(exif) = reader.read_from_container(&mut Cursor::new(bytes)) {
        for field in exif.fields() {
            let tag_name = format!(
                "{:?}/{:?}",
                field.tag,
                field.ifd_num
            );
            let tag_value = field.display_value().with_unit(&exif).to_string();
            entries.push(MetadataEntry {
                standard: "EXIF".to_string(),
                tag_name,
                tag_value,
                raw_value: Some(field.value.to_string()),
            });
        }
    }
    entries
}
```

- [ ] **Step 2: Write JPEG parser (src-tauri/src/analyzer/jpeg_parser.rs)**

```rust
use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt};
use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{read_file_bytes, bytes_to_hex};
use super::{exif_reader, xmp_reader, iptc_reader};

const MARKER_SOI: u8 = 0xD8;
const MARKER_SOF0: u8 = 0xC0;
const MARKER_SOS: u8 = 0xDA;
const MARKER_EOI: u8 = 0xD9;
const MARKER_APP0: u8 = 0xE0;
const MARKER_APP1: u8 = 0xE1;
const MARKER_APP2: u8 = 0xE2;
const MARKER_APP13: u8 = 0xED;
const MARKER_APP14: u8 = 0xEE;
const MARKER_DQT: u8 = 0xDB;
const MARKER_DHT: u8 = 0xC4;
const MARKER_COM: u8 = 0xFE;

pub fn analyze_jpeg(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_name = path.split('/').last().unwrap_or("unknown");

    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != MARKER_SOI {
        return Err("Invalid JPEG signature".to_string());
    }

    let mut structure = Vec::new();
    let mut metadata = Vec::new();
    let mut errors = Vec::new();

    let mut offset = 2;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut color_type = "Unknown".to_string();
    let mut bit_depth = 0u8;
    let has_alpha = false;

    while offset + 2 <= bytes.len() {
        // Skip fill bytes
        while offset < bytes.len() && bytes[offset] == 0xFF {
            offset += 1;
        }
        if offset >= bytes.len() { break; }

        let marker = bytes[offset];
        offset += 1;

        if marker == MARKER_EOI {
            structure.push(FileBlock {
                name: "EOI".to_string(),
                offset: (offset - 2) as u64,
                length: 2,
                data_preview: None,
                decoded_info: Some("End of Image".to_string()),
                children: Vec::new(),
            });
            break;
        }

        if offset + 2 > bytes.len() { break; }

        let length = ((bytes[offset] as u16) << 8 | bytes[offset + 1] as u16) as usize;
        if length < 2 || offset + length > bytes.len() {
            errors.push(format!("Invalid marker {} at offset {}", marker, offset));
            break;
        }

        let marker_name = marker_to_name(marker);
        let data_start = offset + 2;
        let data_end = offset + length;

        let data_preview = if data_end > data_start {
            Some(bytes_to_hex(&bytes[data_start..data_end.min(data_start + 16)], 16))
        } else {
            None
        };

        let decoded_info = match marker {
            MARKER_SOF0..=0xCF if marker != MARKER_SOS && marker != MARKER_DHT && marker != MARKER_COM => {
                if length >= 8 {
                    bit_depth = bytes[data_start];
                    height = u16::from_be_bytes([bytes[data_start + 1], bytes[data_start + 2]]) as u32;
                    width = u16::from_be_bytes([bytes[data_start + 3], bytes[data_start + 4]]) as u32;
                    let components = bytes[data_start + 5];
                    color_type = match components {
                        1 => "Grayscale".to_string(),
                        3 => "RGB".to_string(),
                        4 => "YCbCr".to_string(),
                        _ => format!("{} components", components),
                    };
                    Some(format!("{}x{}, {}-bit, {}", width, height, bit_depth, color_type))
                } else { None }
            }
            m if m == MARKER_APP1 => {
                if length > 8 {
                    let id = &bytes[data_start..data_start.min(data_end)];
                    if let Ok(id_str) = std::str::from_utf8(id) {
                        if id_str.starts_with("Exif") {
                            let exif_entries = exif_reader::extract_exif(&bytes[data_start + 6..data_end]);
                            metadata.extend(exif_entries);
                            Some("EXIF data".to_string())
                        } else if id_str.starts_with("http://ns.adobe.com/xap/1.0/") {
                            let xmp_str = String::from_utf8_lossy(&bytes[data_start + 29..data_end]);
                            metadata.push(MetadataEntry {
                                standard: "XMP".to_string(),
                                tag_name: "XMP".to_string(),
                                tag_value: xmp_str.to_string(),
                                raw_value: None,
                            });
                            Some("XMP data".to_string())
                        } else { None }
                    } else { None }
                } else { None }
            }
            m if m == MARKER_APP13 => {
                if length > 14 {
                    let id = &bytes[data_start..data_start + 14];
                    if id == b"Photoshop 3.0\0" {
                        let iptc = iptc_reader::extract_iptc(&bytes[data_start + 14..data_end]);
                        metadata.extend(iptc);
                        Some("IPTC/Photoshop data".to_string())
                    } else { None }
                } else { None }
            }
            m if m == MARKER_APP2 => {
                if length > 14 {
                    let id = &bytes[data_start..data_start.min(data_start + 12)];
                    if let Ok(id_str) = std::str::from_utf8(id) {
                        if id_str.starts_with("ICC_PROFILE") {
                            Some("ICC Profile".to_string())
                        } else { None }
                    } else { None }
                } else { None }
            }
            m if m == MARKER_APP0 => {
                if length > 7 {
                    let id = &bytes[data_start..data_start + 5];
                    if id == b"JFIF\0" {
                        Some("JFIF".to_string())
                    } else if id == b"JFXX" {
                        Some("JFXX thumbnail".to_string())
                    } else { None }
                } else { None }
            }
            MARKER_DQT => Some("Quantization table(s)".to_string()),
            MARKER_DHT => Some("Huffman table(s)".to_string()),
            MARKER_SOS => Some("Start of Scan (compressed data)".to_string()),
            MARKER_COM => {
                let text = String::from_utf8_lossy(&bytes[data_start..data_end]);
                metadata.push(MetadataEntry {
                    standard: "COM".to_string(),
                    tag_name: "Comment".to_string(),
                    tag_value: text.to_string(),
                    raw_value: None,
                });
                Some("Comment".to_string())
            }
            _ => None,
        };

        structure.push(FileBlock {
            name: marker_name,
            offset: (offset - 2) as u64,
            length: (length + 2) as u64,
            data_preview,
            decoded_info,
            children: Vec::new(),
        });

        offset = data_end;
    }

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size: bytes.len() as u64,
        format: ImageFormat::Jpeg,
        width,
        height,
        color_type,
        bit_depth,
        has_alpha,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}

fn marker_to_name(marker: u8) -> String {
    match marker {
        0xD8 => "SOI".to_string(),
        0xD9 => "EOI".to_string(),
        0xC0 => "SOF0".to_string(),
        0xC1 => "SOF1".to_string(),
        0xC2 => "SOF2".to_string(),
        0xDA => "SOS".to_string(),
        0xDB => "DQT".to_string(),
        0xC4 => "DHT".to_string(),
        0xDD => "DNL".to_string(),
        0xFE => "COM".to_string(),
        0xE0 => "APP0 (JFIF)".to_string(),
        0xE1 => "APP1".to_string(),
        0xE2 => "APP2".to_string(),
        0xE3 => "APP3".to_string(),
        0xED => "APP13".to_string(),
        0xEE => "APP14".to_string(),
        _ => format!("0x{:02X}", marker),
    }
}
```

- [ ] **Step 3: Write XMP reader (src-tauri/src/analyzer/xmp_reader.rs)**

```rust
use crate::types::MetadataEntry;

pub fn parse_xmp(xml: &str) -> Vec<MetadataEntry> {
    let mut entries = Vec::new();
    // Simple XML parsing for XMP — extract key-value pairs
    // For production, use quick-xml
    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.contains(':') && !trimmed.starts_with('<') {
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim().to_string();
                if let Some(eq_pos) = trimmed[colon_pos..].find('=') {
                    let value = trimmed[colon_pos + eq_pos + 1..].trim_matches(|c: char| c == '"' || c == '>' || c == '/').to_string();
                    entries.push(MetadataEntry {
                        standard: "XMP".to_string(),
                        tag_name: key,
                        tag_value: value,
                        raw_value: None,
                    });
                }
            }
        }
    }
    entries
}
```

- [ ] **Step 4: Write IPTC reader (src-tauri/src/analyzer/iptc_reader.rs)**

```rust
use crate::types::MetadataEntry;

pub fn extract_iptc(data: &[u8]) -> Vec<MetadataEntry> {
    let mut entries = Vec::new();
    // Simple IPTC extraction — look for record/tag patterns
    // IPTC records: 8-bit marker(0x1C), 8-bit record, 8-bit tag, 16-bit size, data
    let mut offset = 0;
    while offset + 5 <= data.len() {
        if data[offset] == 0x1C {
            let record = data[offset + 1];
            let tag = data[offset + 2];
            let size = ((data[offset + 3] as u16) << 8 | data[offset + 4] as u16) as usize;
            if offset + 5 + size <= data.len() {
                let value = String::from_utf8_lossy(&data[offset + 5..offset + 5 + size]);
                let tag_name = format!("IPTC/{}:{}", record, tag);
                entries.push(MetadataEntry {
                    standard: "IPTC".to_string(),
                    tag_name,
                    tag_value: value.to_string(),
                    raw_value: None,
                });
                offset += 5 + size;
            } else {
                break;
            }
        } else {
            offset += 1;
        }
    }
    entries
}
```

- [ ] **Step 5: Wire JPEG into commands.rs**

Add to `commands.rs` match arm:

```rust
analyzer::ImageFormat::Png => png_parser::analyze_png(&file_path),
analyzer::ImageFormat::Jpeg => jpeg_parser::analyze_jpeg(&file_path),
```

Add module declarations to `analyzer/mod.rs`:

```rust
pub mod jpeg_parser;
pub mod exif_reader;
pub mod xmp_reader;
pub mod iptc_reader;
```

- [ ] **Step 6: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: add JPEG parser with EXIF/XMP/IPTC/COM metadata extraction"
```

---

### Task 5: WebP + GIF Parsers

**Files:**
- Create: `src-tauri/src/analyzer/webp_parser.rs`
- Create: `src-tauri/src/analyzer/gif_parser.rs`
- Create: `src-tauri/tests/webp_test.rs`
- Create: `src-tauri/tests/gif_test.rs`

- [ ] **Step 1: Write WebP parser (src-tauri/src/analyzer/webp_parser.rs)**

```rust
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{read_file_bytes, bytes_to_hex};
use super::exif_reader;

pub fn analyze_webp(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_name = path.split('/').last().unwrap_or("unknown");

    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err("Invalid WebP signature".to_string());
    }

    let file_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as u64;

    let mut structure = Vec::new();
    let mut metadata = Vec::new();
    let mut errors = Vec::new();

    let mut offset = 12;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut color_type = "Unknown".to_string();
    let mut has_alpha = false;

    while offset + 8 <= bytes.len() {
        let chunk_fourcc = String::from_utf8_lossy(&bytes[offset..offset + 4]).to_string();
        let chunk_size = u32::from_le_bytes([
            bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]
        ]) as usize;

        let data_start = offset + 8;
        let data_end = data_start + chunk_size;

        let data_preview = if data_start < bytes.len() {
            Some(bytes_to_hex(&bytes[data_start..data_end.min(data_start + 16)], 16))
        } else {
            None
        };

        let decoded_info = match chunk_fourcc.as_str() {
            "VP8 " => {
                if data_start + 10 <= bytes.len() {
                    let w = u16::from_le_bytes([bytes[data_start + 6], bytes[data_start + 7]]) as u32;
                    let h = u16::from_le_bytes([bytes[data_start + 8], bytes[data_start + 9]]) as u32;
                    width = w;
                    height = h;
                    color_type = "YCbCr".to_string();
                    Some(format!("{}x{}, lossy", w, h))
                } else { None }
            }
            "VP8L" => {
                if data_start + 4 <= bytes.len() {
                    let sig = u32::from_le_bytes([
                        bytes[data_start], bytes[data_start + 1],
                        bytes[data_start + 2], bytes[data_start + 3]
                    ]);
                    if (sig & 0x01) == 1 {
                        let w = ((sig >> 1) & 0x3FFF) + 1;
                        let h = ((sig >> 15) & 0x3FFF) + 1;
                        let alpha = (sig >> 28) & 1;
                        width = w as u32;
                        height = h as u32;
                        has_alpha = alpha == 1;
                        color_type = if has_alpha { "RGBA".to_string() } else { "RGB".to_string() };
                        Some(format!("{}x{}, lossless, alpha={}", w, h, has_alpha))
                    } else { None }
                } else { None }
            }
            "VP8X" => {
                if data_start < bytes.len() {
                    let flags = bytes[data_start];
                    has_alpha = (flags & 0x10) != 0;
                    let alpha = (flags & 0x10) != 0;
                    let icc = (flags & 0x20) != 0;
                    let exif = (flags & 0x08) != 0;
                    let xmp = (flags & 0x04) != 0;
                    width = ((bytes[data_start + 4] as u32 | (bytes[data_start + 5] as u32) << 8 | (bytes[data_start + 6] as u32) << 16) & 0xFFFFFF) + 1;
                    height = ((bytes[data_start + 7] as u32 | (bytes[data_start + 8] as u32) << 8 | (bytes[data_start + 9] as u32) << 16) & 0xFFFFFF) + 1;
                    color_type = "Unknown (VP8X)".to_string();
                    Some(format!("{}x{}, alpha={}, ICC={}, EXIF={}, XMP={}",
                        width, height, alpha, icc, exif, xmp))
                } else { None }
            }
            "EXIF" => {
                let exif_entries = exif_reader::extract_exif(&bytes[data_start..data_end]);
                metadata.extend(exif_entries);
                Some("EXIF data".to_string())
            }
            "XMP " => {
                let xmp = String::from_utf8_lossy(&bytes[data_start..data_end]);
                metadata.push(MetadataEntry {
                    standard: "XMP".to_string(),
                    tag_name: "XMP".to_string(),
                    tag_value: xmp.to_string(),
                    raw_value: None,
                });
                Some("XMP data".to_string())
            }
            "ANIM" => Some("Animation container".to_string()),
            "ANMF" => Some("Animation frame".to_string()),
            _ => None,
        };

        structure.push(FileBlock {
            name: chunk_fourcc,
            offset: offset as u64,
            length: (chunk_size + 8) as u64,
            data_preview,
            decoded_info,
            children: Vec::new(),
        });

        offset = data_end;
        // RIFF chunks are padded to even
        if chunk_size % 2 != 0 {
            offset += 1;
        }
    }

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size: bytes.len() as u64,
        format: ImageFormat::Webp,
        width,
        height,
        color_type,
        bit_depth: 8,
        has_alpha,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}
```

- [ ] **Step 2: Write GIF parser (src-tauri/src/analyzer/gif_parser.rs)**

```rust
use byteorder::{LittleEndian, ReadBytesExt};
use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry};
use crate::utils::{read_file_bytes, bytes_to_hex};

const EXTENSION_GCE: u8 = 0xF9;
const EXTENSION_CE: u8 = 0xFE;
const EXTENSION_PLAINTEXT: u8 = 0x01;
const EXTENSION_APP: u8 = 0xFF;

pub fn analyze_gif(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_name = path.split('/').last().unwrap_or("unknown");

    if bytes.len() < 6 || &bytes[0..3] != b"GIF" {
        return Err("Invalid GIF signature".to_string());
    }

    let version = String::from_utf8_lossy(&bytes[3..6]).to_string();
    let mut structure = Vec::new();
    let mut metadata = Vec::new();
    let mut errors = Vec::new();

    // Header
    structure.push(FileBlock {
        name: "Header".to_string(),
        offset: 0,
        length: 6,
        data_preview: Some(format!("GIF {}", version)),
        decoded_info: Some(format!("GIF{}", version)),
        children: Vec::new(),
    });

    let mut offset = 6;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut has_gct = false;
    let mut color_type = "Indexed".to_string();
    let mut bit_depth = 0u8;

    // Logical Screen Descriptor
    if offset + 7 <= bytes.len() {
        width = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as u32;
        height = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]) as u32;
        let packed = bytes[offset + 4];
        has_gct = (packed & 0x80) != 0;
        bit_depth = ((packed >> 4) & 0x07) + 1;
        let gct_size = if has_gct { 2 << ((packed & 0x07) as usize) } else { 0 };

        structure.push(FileBlock {
            name: "LSD".to_string(),
            offset: offset as u64,
            length: 7,
            data_preview: None,
            decoded_info: Some(format!(
                "{}x{}, GCT={}, colors={}",
                width, height, has_gct, gct_size
            )),
            children: Vec::new(),
        });

        offset += 7;

        // Global Color Table
        if has_gct {
            let gct_size = 3 * (2 << ((packed & 0x07) as usize));
            structure.push(FileBlock {
                name: "GCT".to_string(),
                offset: offset as u64,
                length: gct_size as u64,
                data_preview: Some(bytes_to_hex(&bytes[offset..offset + gct_size.min(16)], 16)),
                decoded_info: Some(format!("{} colors", gct_size / 3)),
                children: Vec::new(),
            });
            offset += gct_size;
        }
    }

    // Data blocks
    while offset < bytes.len() {
        let block_start = offset;
        let separator = bytes[offset];

        if separator == 0x3B {
            // Trailer
            structure.push(FileBlock {
                name: "Trailer".to_string(),
                offset: offset as u64,
                length: 1,
                data_preview: None,
                decoded_info: Some("End of GIF".to_string()),
                children: Vec::new(),
            });
            break;
        } else if separator == 0x21 {
            // Extension introducer
            if offset + 1 >= bytes.len() { break; }
            let label = bytes[offset + 1];
            offset += 2;

            match label {
                EXTENSION_GCE => {
                    if offset + 5 <= bytes.len() {
                        let packed = bytes[offset];
                        let delay = u16::from_le_bytes([bytes[offset + 1], bytes[offset + 2]]);
                        let trans_idx = bytes[offset + 3];
                        let disposal = (packed >> 2) & 0x07;

                        structure.push(FileBlock {
                            name: "GCE".to_string(),
                            offset: block_start as u64,
                            length: 8,
                            data_preview: None,
                            decoded_info: Some(format!(
                                "delay={}ms, disposal={}, trans_idx={}",
                                delay * 10, disposal, trans_idx
                            )),
                            children: Vec::new(),
                        });
                        offset += 5; // data
                        if offset < bytes.len() && bytes[offset] == 0 { offset += 1; } // block terminator
                    } else { break; }
                }
                EXTENSION_CE => {
                    let (data, end) = read_sub_blocks(&bytes, offset);
                    let text = String::from_utf8_lossy(&data);
                    metadata.push(MetadataEntry {
                        standard: "GIF Comment".to_string(),
                        tag_name: "Comment".to_string(),
                        tag_value: text.to_string(),
                        raw_value: None,
                    });
                    structure.push(FileBlock {
                        name: "Comment Extension".to_string(),
                        offset: block_start as u64,
                        length: (end - block_start) as u64,
                        data_preview: Some(bytes_to_hex(&data[..data.len().min(16)], 16)),
                        decoded_info: Some(text.to_string()),
                        children: Vec::new(),
                    });
                    offset = end;
                }
                EXTENSION_APP => {
                    let (data, end) = read_sub_blocks(&bytes, offset);
                    let app_id = String::from_utf8_lossy(&data[..data.len().min(11)]);
                    let decoded = if app_id == "NETSCAPE2.0" && data.len() >= 5 {
                        let loop_count = u16::from_le_bytes([data[3], data[4]]);
                        format!("NETSCAPE2.0, loop={}", loop_count)
                    } else {
                        format!("Application: {}", app_id)
                    };
                    structure.push(FileBlock {
                        name: "Application Extension".to_string(),
                        offset: block_start as u64,
                        length: (end - block_start) as u64,
                        data_preview: Some(bytes_to_hex(&data[..data.len().min(16)], 16)),
                        decoded_info: Some(decoded),
                        children: Vec::new(),
                    });
                    offset = end;
                }
                EXTENSION_PLAINTEXT => {
                    let (data, end) = read_sub_blocks(&bytes, offset);
                    structure.push(FileBlock {
                        name: "Plain Text Extension".to_string(),
                        offset: block_start as u64,
                        length: (end - block_start) as u64,
                        data_preview: Some(bytes_to_hex(&data[..data.len().min(16)], 16)),
                        decoded_info: None,
                        children: Vec::new(),
                    });
                    offset = end;
                }
                _ => {
                    let (data, end) = read_sub_blocks(&bytes, offset);
                    structure.push(FileBlock {
                        name: format!("Unknown Extension 0x{:02X}", label),
                        offset: block_start as u64,
                        length: (end - block_start) as u64,
                        data_preview: Some(bytes_to_hex(&data[..data.len().min(16)], 16)),
                        decoded_info: None,
                        children: Vec::new(),
                    });
                    offset = end;
                }
            }
        } else if separator == 0x2C {
            // Image Descriptor
            if offset + 10 <= bytes.len() {
                let left = u16::from_le_bytes([bytes[offset + 1], bytes[offset + 2]]) as u32;
                let top = u16::from_le_bytes([bytes[offset + 3], bytes[offset + 4]]) as u32;
                let w = u16::from_le_bytes([bytes[offset + 5], bytes[offset + 6]]) as u32;
                let h = u16::from_le_bytes([bytes[offset + 7], bytes[offset + 8]]) as u32;
                let packed = bytes[offset + 9];
                let has_lct = (packed & 0x80) != 0;
                let interlace = (packed & 0x40) != 0;

                let mut children = Vec::new();
                let mut img_end = offset + 10;

                if has_lct {
                    let lct_size = 3 * (2 << ((packed & 0x07) as usize));
                    children.push(FileBlock {
                        name: "LCT".to_string(),
                        offset: img_end as u64,
                        length: lct_size as u64,
                        data_preview: None,
                        decoded_info: Some(format!("{} colors", lct_size / 3)),
                        children: Vec::new(),
                    });
                    img_end += lct_size;
                }

                // Skip LZW data (sub-blocks until 0x00)
                let (lzw_data, data_end) = read_sub_blocks(&bytes, img_end);
                if data_end > img_end {
                    children.push(FileBlock {
                        name: "LZW Data".to_string(),
                        offset: img_end as u64,
                        length: (data_end - img_end) as u64,
                        data_preview: Some(bytes_to_hex(&lzw_data[..lzw_data.len().min(16)], 16)),
                        decoded_info: Some(format!("{} bytes compressed", lzw_data.len())),
                        children: Vec::new(),
                    });
                }

                structure.push(FileBlock {
                    name: "Image Descriptor".to_string(),
                    offset: block_start as u64,
                    length: (data_end - block_start) as u64,
                    data_preview: None,
                    decoded_info: Some(format!(
                        "{}x{} @ ({},{}), LCT={}, interlace={}",
                        w, h, left, top, has_lct, interlace
                    )),
                    children,
                });
                offset = data_end;
            } else { break; }
        } else {
            break;
        }
    }

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size: bytes.len() as u64,
        format: ImageFormat::Gif,
        width,
        height,
        color_type,
        bit_depth,
        has_alpha: false,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid: None,
        analysis_errors: errors,
    })
}

fn read_sub_blocks(bytes: &[u8], mut offset: usize) -> (Vec<u8>, usize) {
    let mut data = Vec::new();
    while offset < bytes.len() {
        let size = bytes[offset] as usize;
        offset += 1;
        if size == 0 { break; }
        if offset + size <= bytes.len() {
            data.extend_from_slice(&bytes[offset..offset + size]);
            offset += size;
        } else {
            break;
        }
    }
    (data, offset)
}
```

- [ ] **Step 3: Wire WebP and GIF into commands.rs**

```rust
analyzer::ImageFormat::Webp => webp_parser::analyze_webp(&file_path),
analyzer::ImageFormat::Gif => gif_parser::analyze_gif(&file_path),
```

Add to `analyzer/mod.rs`:
```rust
pub mod webp_parser;
pub mod gif_parser;
```

- [ ] **Step 4: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add WebP and GIF parsers with metadata extraction"
```

---

### Task 6: Channel Split + Histograms + ChannelsTab

**Files:**
- Create: `src-tauri/src/analyzer/channel_split.rs`
- Create: `src/components/tabs/ChannelsTab.svelte`
- Create: `src/components/ui/Histogram.svelte`

- [ ] **Step 1: Write channel split (src-tauri/src/analyzer/channel_split.rs)**

```rust
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
    let y_vals = r_vals.iter().zip(&g_vals).zip(&b_vals)
        .map(|((r, g), b)| {
            (0.2126 * *r as f64 + 0.7152 * *g as f64 + 0.0722 * *b as f64) as u8
        })
        .collect::<Vec<_>>();

    let cb_vals = r_vals.iter().zip(&g_vals).zip(&b_vals)
        .map(|((r, g), b)| {
            ((128.0 - 0.1146 * *r as f64 - 0.3854 * *g as f64 + 0.5 * *b as f64) as f64 + 128.0).clamp(0.0, 255.0) as u8
        })
        .collect::<Vec<_>>();

    let cr_vals = r_vals.iter().zip(&g_vals).zip(&b_vals)
        .map(|((r, g), b)| {
            ((0.5 * *r as f64 - 0.4542 * *g as f64 - 0.0458 * *b as f64) as f64 + 128.0).clamp(0.0, 255.0) as u8
        })
        .collect::<Vec<_>>();

    let yuv = YuvChannels {
        y: compute_stats("Y", &y_vals),
        cb: compute_stats("Cb", &cb_vals),
        cr: compute_stats("Cr", &cr_vals),
    };

    let histograms = vec![
        Histogram { channel: "R".to_string(), bins: compute_histogram(&r_vals) },
        Histogram { channel: "G".to_string(), bins: compute_histogram(&g_vals) },
        Histogram { channel: "B".to_string(), bins: compute_histogram(&b_vals) },
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
            name: name.to_string(), min: 0, max: 0, mean: 0.0, median: 0, std_dev: 0.0,
        };
    }
    let mut sorted = vals.to_vec();
    sorted.sort();
    let sum: u64 = vals.iter().map(|&v| v as u64).sum();
    let mean = sum as f64 / vals.len() as f64;
    let variance: f64 = vals.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    SingleChannel {
        name: name.to_string(),
        min: sorted[0],
        max: sorted[sorted.len() - 1],
        mean,
        median: sorted[sorted.len() / 2],
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
```

- [ ] **Step 2: Wire channel split into each parser**

In each parser's `Ok(ImageAnalysis { ... })`, add after metadata:

```rust
channels: channel_split::compute_channels(&bytes),
```

This requires passing the raw bytes to `compute_channels`. For PNG, the bytes are already available. For JPEG/WebP/GIF, use the original `bytes` variable.

- [ ] **Step 3: Write Histogram component (src/components/ui/Histogram.svelte)**

```svelte
<script lang="ts">
  let { bins, color = '#818cf8', label = '' }: { bins: number[]; color?: string; label?: string } = $props();

  const maxBin = Math.max(...bins, 1);
  const barWidth = 100 / bins.length;

  function getHeight(val: number): number {
    return (val / maxBin) * 100;
  }
</script>

<div class="histogram">
  {#if label}<span class="hist-label">{label}</span>{/if}
  <svg viewBox="0 0 256 100" preserveAspectRatio="none" class="hist-svg">
    {#each bins as bin, i}
      <rect
        x={i}
        y={100 - getHeight(bin)}
        width="1"
        height={getHeight(bin)}
        fill={color}
        opacity="0.8"
      />
    {/each}
  </svg>
</div>

<style>
  .histogram {
    margin: 0.5rem 0;
  }
  .hist-label {
    font-size: 0.7rem;
    color: var(--color-muted, #64748b);
  }
  .hist-svg {
    width: 100%;
    height: 60px;
    background: rgba(255,255,255,0.02);
    border-radius: 2px;
  }
</style>
```

- [ ] **Step 4: Write ChannelsTab (src/components/tabs/ChannelsTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';
  import Histogram from '../ui/Histogram.svelte';

  let { data }: { data: ImageAnalysis } = $props();
  let mode = $state<'rgb' | 'yuv'>('rgb');

  if (!data.channels) {
    // show placeholder
  }
</script>

<div class="channels-tab">
  <div class="mode-switcher">
    <button class:active={mode === 'rgb'} on:click={() => mode = 'rgb'}>RGB</button>
    <button class:active={mode === 'yuv'} on:click={() => mode = 'yuv'}>YUV</button>
    {#if data.channels?.ycbcr_subsampling}
      <span class="subsampling">{data.channels.ycbcr_subsampling}</span>
    {/if}
  </div>

  <div class="color-matrix">Matrix: {data.channels?.color_matrix || 'N/A'}</div>

  {#if mode === 'rgb' && data.channels?.rgb}
    <div class="channels">
      <div class="channel-card" style="--ch-color: #ef4444">
        <h4>R</h4>
        <p>min: {data.channels.rgb.r.min} max: {data.channels.rgb.r.max} mean: {data.channels.rgb.r.mean.toFixed(1)}</p>
        <Histogram bins={data.channels.histograms[0]?.bins || []} color="#ef4444" label="Red" />
      </div>
      <div class="channel-card" style="--ch-color: #22c55e">
        <h4>G</h4>
        <p>min: {data.channels.rgb.g.min} max: {data.channels.rgb.g.max} mean: {data.channels.rgb.g.mean.toFixed(1)}</p>
        <Histogram bins={data.channels.histograms[1]?.bins || []} color="#22c55e" label="Green" />
      </div>
      <div class="channel-card" style="--ch-color: #3b82f6">
        <h4>B</h4>
        <p>min: {data.channels.rgb.b.min} max: {data.channels.rgb.b.max} mean: {data.channels.rgb.b.mean.toFixed(1)}</p>
        <Histogram bins={data.channels.histograms[2]?.bins || []} color="#3b82f6" label="Blue" />
      </div>
      {#if data.channels.rgb.a}
        <div class="channel-card" style="--ch-color: #888">
          <h4>A</h4>
          <p>min: {data.channels.rgb.a.min} max: {data.channels.rgb.a.max} mean: {data.channels.rgb.a.mean.toFixed(1)}</p>
        </div>
      {/if}
    </div>
  {/if}

  {#if mode === 'yuv' && data.channels?.yuv}
    <div class="channels">
      <div class="channel-card" style="--ch-color: #fff">
        <h4>Y (Luma)</h4>
        <p>min: {data.channels.yuv.y.min} max: {data.channels.yuv.y.max} mean: {data.channels.yuv.y.mean.toFixed(1)}</p>
      </div>
      <div class="channel-card" style="--ch-color: #3b82f6">
        <h4>Cb (Chroma Blue)</h4>
        <p>min: {data.channels.yuv.cb.min} max: {data.channels.yuv.cb.max} mean: {data.channels.yuv.cb.mean.toFixed(1)}</p>
      </div>
      <div class="channel-card" style="--ch-color: #ef4444">
        <h4>Cr (Chroma Red)</h4>
        <p>min: {data.channels.yuv.cr.min} max: {data.channels.yuv.cr.max} mean: {data.channels.yuv.cr.mean.toFixed(1)}</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .channels-tab {
    font-size: 0.875rem;
  }
  .mode-switcher {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
  .mode-switcher button {
    padding: 0.4rem 1rem;
    background: rgba(255,255,255,0.05);
    border: 1px solid var(--color-border, #334155);
    color: var(--color-muted, #64748b);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .mode-switcher button.active {
    background: var(--color-accent, #6366f1);
    color: white;
    border-color: var(--color-accent, #6366f1);
  }
  .subsampling {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
  }
  .color-matrix {
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
    margin-bottom: 1rem;
  }
  .channels {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .channel-card {
    padding: 0.75rem;
    border: 1px solid var(--color-border, #1e293b);
    border-radius: 6px;
    border-left: 3px solid var(--ch-color);
  }
  .channel-card h4 {
    margin: 0 0 0.25rem;
    color: var(--ch-color);
  }
  .channel-card p {
    margin: 0;
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
  }
</style>
```

- [ ] **Step 5: Wire ChannelsTab into MainPanel**

In MainPanel.svelte, replace the placeholder for channels tab:

```svelte
{:else if activeTab === 'channels'}
  <ChannelsTab data={$currentImage} />
```

And add the import at the top.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add channel split with RGB/YUV histograms and ChannelsTab"
```

---

## Phase 3: ICC + HEIF/HEIC/AVIF

### Task 7: ICC Profile Parser + ColorInfoTab

**Files:**
- Create: `src-tauri/src/analyzer/icc_parser.rs`
- Create: `src/components/tabs/ColorInfoTab.svelte`
- Modify: PNG parser (extract iCCP → icc_parser)
- Modify: JPEG parser (extract APP2 ICC → icc_parser)

- [ ] **Step 1: Write ICC parser (src-tauri/src/analyzer/icc_parser.rs)**

```rust
use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use crate::types::{IccInfo, PrimariesInfo, Matrix3x3, LutInfo, IccTag};

pub fn parse_icc(data: &[u8]) -> Option<IccInfo> {
    if data.len() < 128 { return None; }

    let mut cursor = Cursor::new(data);

    // ICC Header (offsets 0-127)
    let size = cursor.read_u32::<BigEndian>().ok()?;
    cursor.set_position(8);
    let cmm_type = read_tag4(data, 12)?;
    cursor.set_position(16);
    let profile_class = read_tag4(data, 20)?;
    cursor.set_position(24);
    let color_space = read_tag4(data, 28)?;
    cursor.set_position(28);
    let pcs = read_tag4(data, 32)?;

    cursor.set_position(36);
    let day = cursor.read_u8().ok()?;
    let month = cursor.read_u8().ok()?;
    let year = cursor.read_u16::<BigEndian>().ok()?;
    let version = format!("{}.{}.{}.{}",
        data[44],
        (data[45] >> 4) & 0x0F,
        (data[45] & 0x0F) << 4 | (data[46] >> 4),
        data[46] & 0x0F
    );

    cursor.set_position(48);
    let platform = read_tag4(data, 48);

    cursor.set_position(64);
    let intent_num = cursor.read_u32::<BigEndian>().ok()?;
    let rendering_intent = match intent_num {
        0 => "Perceptual",
        1 => "Relative Colorimetric",
        2 => "Saturation",
        3 => "Absolute Colorimetric",
        _ => "Unknown",
    }.to_string();

    // Illuminant (XYZ) at offset 68
    let illuminant_x = read_s15fixed16(data, 68);
    let illuminant_y = read_s15fixed16(data, 72);
    let illuminant_z = read_s15fixed16(data, 76);

    let creator = read_tag4(data, 80);
    let mut description = None;

    // Parse tags
    cursor.set_position(128);
    let tag_count = cursor.read_u32::<BigEndian>().ok()?;
    let mut tags = Vec::new();
    let mut luts = Vec::new();
    let mut primaries: Option<PrimariesInfo> = None;
    let mut matrix: Option<Matrix3x3> = None;
    let mut transfer_function: Option<String> = None;
    let mut red_trc: Option<String> = None;
    let mut green_trc: Option<String> = None;
    let mut blue_trc: Option<String> = None;

    for _ in 0..tag_count {
        let tag_sig = read_tag4(data, cursor.position() as usize)?;
        let tag_offset = cursor.read_u32::<BigEndian>().ok()? as usize;
        let tag_size = cursor.read_u32::<BigEndian>().ok()? as usize;

        if tag_offset + 4 > data.len() { continue; }
        let tag_type = read_tag4(data, tag_offset).unwrap_or_default();

        let decoded_value = decode_tag(data, tag_offset, tag_size, &tag_type);

        // Extract specific tags
        if tag_sig == "desc" {
            description = decoded_value.clone();
        }
        if tag_sig == "rXYZ" || tag_sig == "gXYZ" || tag_sig == "bXYZ" || tag_sig == "wtpt" || tag_sig == "bkpt" {
            if tag_offset + 20 <= data.len() {
                // XYZ type
                let x = read_s15fixed16(data, tag_offset + 4);
                let y = read_s15fixed16(data, tag_offset + 8);
                let z = read_s15fixed16(data, tag_offset + 12);
                // Collect for primaries
            }
        }
        if tag_sig == "rTRC" {
            red_trc = describe_trc(data, tag_offset, tag_size);
            if transfer_function.is_none() { transfer_function = red_trc.clone(); }
        }
        if tag_sig == "gTRC" {
            green_trc = describe_trc(data, tag_offset, tag_size);
        }
        if tag_sig == "bTRC" {
            blue_trc = describe_trc(data, tag_offset, tag_size);
        }
        if tag_sig.starts_with("A2B") || tag_sig.starts_with("B2A") {
            luts.push(LutInfo {
                name: tag_sig,
                clut_points: None,
                input_channels: 0,
                output_channels: 0,
            });
        }

        tags.push(IccTag {
            name: tag_sig,
            offset: tag_offset as u32,
            size: tag_size as u32,
            tag_type,
            decoded_value,
        });
    }

    Some(IccInfo {
        size,
        cmm_type,
        version,
        profile_class,
        color_space,
        pcs,
        platform,
        rendering_intent,
        illuminant: (illuminant_x, illuminant_y, illuminant_z),
        creator,
        description,
        transfer_function,
        red_trc,
        green_trc,
        blue_trc,
        primaries,
        matrix,
        luts,
        tag_count,
        tags,
        raw_base64: None,
    })
}

fn read_tag4(data: &[u8], offset: usize) -> Option<String> {
    if offset + 4 > data.len() { return None; }
    String::from_utf8(data[offset..offset + 4].to_vec()).ok()
}

fn read_s15fixed16(data: &[u8], offset: usize) -> f64 {
    if offset + 4 > data.len() { return 0.0; }
    let val = ((data[offset] as i32) << 24
        | (data[offset + 1] as i32) << 16
        | (data[offset + 2] as i32) << 8
        | data[offset + 3] as i32) as f64 / 65536.0;
    val
}

fn describe_trc(data: &[u8], offset: usize, size: usize) -> Option<String> {
    if offset + 4 > data.len() { return None; }
    let sig = read_tag4(data, offset)?;
    match sig.as_str() {
        "curv" => {
            if offset + 8 <= data.len() {
                let count = u32::from_be_bytes([
                    data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
                ]);
                if count == 0 {
                    Some("Linear".to_string())
                } else if count == 1 {
                    let gamma = u16::from_be_bytes([data[offset + 8], data[offset + 9]]);
                    Some(format!("Gamma={:.2}", gamma as f64 / 256.0))
                } else {
                    Some(format!("LUT ({}) entries", count))
                }
            } else { None }
        }
        "para" => {
            if offset + 8 <= data.len() {
                let gct = u16::from_be_bytes([data[offset + 4], data[offset + 5]]);
                Some(format!("Parametric type {}", gct))
            } else { None }
        }
        "sf32" => Some("sRGB".to_string()),
        _ => Some(format!("Unknown TRC type: {}", sig)),
    }
}

fn decode_tag(data: &[u8], offset: usize, size: usize, tag_type: &str) -> Option<String> {
    match tag_type {
        "text" | "desc" => {
            let end = std::cmp::min(offset + size, data.len());
            String::from_utf8(data[offset..end].to_vec()).ok()
        }
        "XYZ " => {
            if offset + 20 <= data.len() {
                let x = read_s15fixed16(data, offset + 4);
                let y = read_s15fixed16(data, offset + 8);
                let z = read_s15fixed16(data, offset + 12);
                Some(format!("X={:.4} Y={:.4} Z={:.4}", x, y, z))
            } else { None }
        }
        "sig " => read_tag4(data, offset),
        _ => None,
    }
}
```

- [ ] **Step 2: Write ColorInfoTab (src/components/tabs/ColorInfoTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';

  let { data }: { data: ImageAnalysis } = $props();
</script>

<div class="color-info">
  {#if data.icc_profile}
    <section class="section">
      <h3>ICC Profile Header</h3>
      <table class="data-table">
        <tr><td>CMM Type</td><td>{data.icc_profile.cmm_type}</td></tr>
        <tr><td>Version</td><td>{data.icc_profile.version}</td></tr>
        <tr><td>Profile Class</td><td>{data.icc_profile.profile_class}</td></tr>
        <tr><td>Color Space</td><td>{data.icc_profile.color_space}</td></tr>
        <tr><td>PCS</td><td>{data.icc_profile.pcs}</td></tr>
        {#if data.icc_profile.platform}
          <tr><td>Platform</td><td>{data.icc_profile.platform}</td></tr>
        {/if}
        <tr><td>Rendering Intent</td><td>{data.icc_profile.rendering_intent}</td></tr>
        <tr><td>Illuminant</td><td>X={data.icc_profile.illuminant[0].toFixed(4)} Y={data.icc_profile.illuminant[1].toFixed(4)} Z={data.icc_profile.illuminant[2].toFixed(4)}</td></tr>
        {#if data.icc_profile.description}
          <tr><td>Description</td><td>{data.icc_profile.description}</td></tr>
        {/if}
      </table>
    </section>

    <section class="section">
      <h3>Transfer Curves (TRC)</h3>
      <table class="data-table">
        {#if data.icc_profile.transfer_function}
          <tr><td>Global TRC</td><td>{data.icc_profile.transfer_function}</td></tr>
        {/if}
        {#if data.icc_profile.red_trc}
          <tr><td>Red TRC</td><td>{data.icc_profile.red_trc}</td></tr>
        {/if}
        {#if data.icc_profile.green_trc}
          <tr><td>Green TRC</td><td>{data.icc_profile.green_trc}</td></tr>
        {/if}
        {#if data.icc_profile.blue_trc}
          <tr><td>Blue TRC</td><td>{data.icc_profile.blue_trc}</td></tr>
        {/if}
      </table>
    </section>

    <section class="section">
      <h3>ICC Tags ({data.icc_profile.tag_count})</h3>
      <table class="data-table">
        {#each data.icc_profile.tags as tag}
          <tr>
            <td><code>{tag.name}</code></td>
            <td>{tag.tag_type}</td>
            <td>offset={tag.offset} size={tag.size}</td>
            {#if tag.decoded_value}
              <td>{tag.decoded_value}</td>
            {/if}
          </tr>
        {/each}
      </table>
    </section>
  {:else}
    <p class="no-icc">No ICC profile found in this image.</p>
  {/if}
</div>

<style>
  .color-info {
    font-size: 0.875rem;
  }
  .section {
    margin-bottom: 1.5rem;
  }
  .section h3 {
    margin: 0 0 0.5rem;
    color: var(--color-text, #e2e8f0);
    font-size: 0.9rem;
  }
  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .data-table td {
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #1e293b);
  }
  .data-table td:first-child {
    color: var(--color-muted, #64748b);
    width: 140px;
  }
  code {
    color: var(--color-accent, #818cf8);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
  }
  .no-icc {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
  }
</style>
```

- [ ] **Step 3: Wire ICC into PNG parser**

In png_parser.rs, when parsing "iCCP" chunk:

```rust
"iCCP" => {
    // Profile name is null-terminated, then compression method, then profile data
    if let Some(null_pos) = bytes[chunk_data_start..data_end].iter().position(|&b| b == 0) {
        let profile_data_start = chunk_data_start + null_pos + 2; // skip null + compression byte
        if let Some(icc) = icc_parser::parse_icc(&bytes[profile_data_start..data_end]) {
            // Will be set on ImageAnalysis
        }
    }
    Some(format!("ICC Profile ({} bytes)", length))
}
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add ICC profile parser and ColorInfoTab with header/TRC/tags display"
```

---

### Task 8: HEIF Container + Grid Parser

**Files:**
- Create: `src-tauri/src/analyzer/heif_parser.rs`
- Create: `src/components/tabs/GridTab.svelte`
- Create: `src/components/ui/GridView.svelte`

- [ ] **Step 1: Write HEIF parser (src-tauri/src/analyzer/heif_parser.rs)**

```rust
use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use crate::types::{FileBlock, ImageAnalysis, ImageFormat, MetadataEntry, GridInfo, GridTile};
use crate::utils::{read_file_bytes, bytes_to_hex};

pub fn analyze_heif(path: &str) -> Result<ImageAnalysis, String> {
    let bytes = read_file_bytes(path)?;
    let file_name = path.split('/').last().unwrap_or("unknown");

    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return Err("Invalid HEIF/HEIC file (no ftyp box)".to_string());
    }

    let brand = String::from_utf8_lossy(&bytes[8..12]).to_string();
    let format = if brand.contains("heic") || brand.contains("heix") || brand.contains("heim") || brand.contains("heis") {
        ImageFormat::Heic
    } else if brand.contains("avif") || brand.contains("avis") {
        ImageFormat::Avif
    } else {
        return Err(format!("Unsupported HEIF brand: {}", brand));
    };

    let mut structure = Vec::new();
    let mut metadata = Vec::new();
    let mut grid: Option<GridInfo> = None;
    let mut errors = Vec::new();
    let mut width = 0u32;
    let mut height = 0u32;

    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let box_size = u32::from_be_bytes([
            bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]
        ]) as usize;
        if box_size == 0 { break; }

        let box_type = String::from_utf8_lossy(&bytes[offset + 4..offset + 8]).to_string();
        let data_start = offset + 8;
        let data_end = offset + box_size;

        let decoded_info = match box_type.as_str() {
            "ftyp" => {
                let brands: Vec<_> = (12..data_end).step_by(4)
                    .filter(|i| i + 4 <= data_end)
                    .map(|i| String::from_utf8_lossy(&bytes[i..i + 4]).to_string())
                    .collect();
                Some(format!("brand={}, compatible={}", brand, brands.join(", ")))
            }
            "meta" => Some("Metadata container".to_string()),
            "hdlr" => {
                if data_start + 12 <= data_end {
                    let handler = String::from_utf8_lossy(&bytes[data_start + 8..data_start + 12]).to_string();
                    Some(format!("handler={}", handler))
                } else { None }
            }
            "pitm" => {
                if data_start + 2 <= data_end {
                    let item_id = u16::from_be_bytes([bytes[data_start], bytes[data_start + 1]]);
                    Some(format!("primary item ID={}", item_id))
                } else { None }
            }
            "grid" => {
                if data_start + 5 <= data_end {
                    let version = bytes[data_start];
                    let flags = bytes[data_start + 1];
                    let rows_minus1 = bytes[data_start + 2];
                    let cols_minus1 = bytes[data_start + 3];
                    let output_w = if (flags & 1) == 0 {
                        u16::from_be_bytes([bytes[data_start + 4], bytes[data_start + 5]]) as u32
                    } else if data_start + 7 <= data_end {
                        u32::from_be_bytes([
                            bytes[data_start + 4], bytes[data_start + 5],
                            bytes[data_start + 6], bytes[data_start + 7]
                        ])
                    } else { 0 };
                    let output_h = if (flags & 1) == 0 {
                        let pos = if (flags & 1) == 0 { data_start + 6 } else { data_start + 8 };
                        if pos + 2 <= data_end {
                            u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as u32
                        } else { 0 }
                    } else { 0 };

                    let rows = rows_minus1 as u32 + 1;
                    let cols = cols_minus1 as u32 + 1;
                    width = output_w;
                    height = output_h;

                    Some(format!("grid {}x{}, output {}x{}", rows, cols, output_w, output_h))
                } else { None }
            }
            "iloc" => Some("Item location box".to_string()),
            "iprp" => Some("Item properties container".to_string()),
            "iinf" => Some("Item info".to_string()),
            "idat" => Some("Item data".to_string()),
            "iref" => Some("Item references".to_string()),
            _ => None,
        };

        let data_preview = if box_type == "idat" {
            Some("[large binary data]".to_string())
        } else if data_start < data_end {
            Some(bytes_to_hex(&bytes[data_start..data_end.min(data_start + 16)], 16))
        } else {
            None
        };

        structure.push(FileBlock {
            name: box_type,
            offset: offset as u64,
            length: box_size as u64,
            data_preview,
            decoded_info,
            children: Vec::new(),
        });

        offset += box_size;
    }

    Ok(ImageAnalysis {
        file_name: file_name.to_string(),
        file_path: path.to_string(),
        file_size: bytes.len() as u64,
        format,
        width,
        height,
        color_type: "HEIF".to_string(),
        bit_depth: 0,
        has_alpha: false,
        structure,
        metadata,
        channels: None,
        icc_profile: None,
        codec_syntax: None,
        grid,
        analysis_errors: errors,
    })
}
```

- [ ] **Step 2: Write GridView (src/components/ui/GridView.svelte)**

```svelte
<script lang="ts">
  import type { GridInfo } from '../../lib/types';

  let { grid }: { grid: GridInfo } = $props();
</script>

<div class="grid-view">
  <div class="grid-header">
    <span>Grid: {grid.cols}×{grid.rows}</span>
    <span>Output: {grid.output_width}×{grid.output_height}</span>
  </div>
  <div class="grid-container" style="grid-template-columns: repeat({grid.cols}, 1fr)">
    {#each grid.tiles as tile}
      <div class="grid-tile">
        <span class="tile-id">#{tile.item_id}</span>
        <span class="tile-size">{tile.width}×{tile.height}</span>
        <span class="tile-offset">({tile.horizontal_offset}, {tile.vertical_offset})</span>
        <span class="tile-codec">{tile.codec}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .grid-view {
    padding: 0.5rem;
  }
  .grid-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
    color: var(--color-muted, #64748b);
    margin-bottom: 0.75rem;
  }
  .grid-container {
    display: grid;
    gap: 4px;
  }
  .grid-tile {
    border: 1px solid var(--color-border, #334155);
    border-radius: 4px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background: rgba(255,255,255,0.02);
    min-height: 60px;
  }
  .tile-id {
    font-weight: 600;
    color: var(--color-accent, #818cf8);
    font-size: 0.85rem;
  }
  .tile-size {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.75rem;
    color: var(--color-text, #e2e8f0);
  }
  .tile-offset {
    font-size: 0.7rem;
    color: var(--color-muted, #64748b);
  }
  .tile-codec {
    font-size: 0.7rem;
    color: var(--color-accent, #818cf8);
    font-family: monospace;
  }
</style>
```

- [ ] **Step 3: Write GridTab (src/components/tabs/GridTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';
  import GridView from '../ui/GridView.svelte';

  let { data }: { data: ImageAnalysis } = $props();
</script>

<div class="grid-tab">
  {#if data.grid}
    <GridView grid={data.grid} />
  {:else}
    <p class="no-grid">This image does not contain a grid structure.</p>
  {/if}
</div>

<style>
  .no-grid {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
  }
</style>
```

- [ ] **Step 4: Wire HEIF into commands.rs**

```rust
analyzer::ImageFormat::Avif => heif_parser::analyze_heif(&file_path),
analyzer::ImageFormat::Heic => heif_parser::analyze_heif(&file_path),
```

Add to `analyzer/mod.rs`:
```rust
pub mod heif_parser;
```

- [ ] **Step 5: Run cargo check**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add HEIF/HEIC/AVIF container parser with Grid visualization"
```

---

## Phase 4: HEVC + AV1 Codec Syntax

### Task 9: HEVC NAL + VPS/SPS/PPS/Slice Header Parser

**Files:**
- Create: `src-tauri/src/analyzer/hevc/mod.rs`
- Create: `src-tauri/src/analyzer/hevc/nalu.rs`
- Create: `src-tauri/src/analyzer/hevc/vps.rs`
- Create: `src-tauri/src/analyzer/hevc/sps.rs`
- Create: `src-tauri/src/analyzer/hevc/pps.rs`
- Create: `src-tauri/src/analyzer/hevc/slice_header.rs`

- [ ] **Step 1: Write HEVC module (src-tauri/src/analyzer/hevc/mod.rs)**

```rust
pub mod nalu;
pub mod vps;
pub mod sps;
pub mod pps;
pub mod slice_header;

pub use nalu::parse_hevc_bitstream;
```

- [ ] **Step 2: Write NAL unit parser (src-tauri/src/analyzer/hevc/nalu.rs)**

```rust
use crate::types::{NalUnit, HevcSyntax, HevcSliceHeader, VideoParameterSet, SequenceParameterSet, PictureParameterSet};
use super::{vps, sps, pps, slice_header};

pub fn parse_hevc_bitstream(data: &[u8]) -> HevcSyntax {
    let mut nal_units = Vec::new();
    let mut vps_parsed: Option<VideoParameterSet> = None;
    let mut sps_parsed: Option<SequenceParameterSet> = None;
    let mut pps_parsed: Option<PictureParameterSet> = None;
    let mut slice_headers = Vec::new();

    let mut offset = 0;
    while offset + 4 <= data.len() {
        // Find start code (0x00 0x00 0x00 0x01 or 0x00 0x00 0x01)
        let start_code_len = if offset + 3 < data.len() && data[offset] == 0 && data[offset + 1] == 0 && data[offset + 2] == 0 && data[offset + 3] == 1 {
            4
        } else if offset + 2 < data.len() && data[offset] == 0 && data[offset + 1] == 0 && data[offset + 2] == 1 {
            3
        } else {
            offset += 1;
            continue;
        };

        let nal_start = offset + start_code_len;
        if nal_start >= data.len() { break; }

        // Find next start code
        let next_start = find_start_code(data, nal_start);
        let nal_end = if next_start > 0 { next_start } else { data.len() };
        let nal_data = &data[nal_start..nal_end];

        if nal_data.len() < 2 {
            offset = nal_end;
            continue;
        }

        let nal_header = nal_data[0];
        let forbidden_zero = (nal_header >> 7) & 1;
        let nal_unit_type = (nal_header >> 1) & 0x3F;
        let nuh_layer_id = ((nal_header & 1) << 5) | ((nal_data[1] >> 5) & 0x1F);
        let nuh_temporal_id = (nal_data[1] & 0x07) - 1;

        let nal_type_name = nal_type_to_string(nal_unit_type);

        nal_units.push(NalUnit {
            nal_type: nal_type_name.clone(),
            nuh_layer_id,
            nuh_temporal_id,
            size: nal_data.len(),
            offset: nal_start as u64,
        });

        // Parse payload based on type
        let payload = &nal_data[2..];
        match nal_unit_type {
            32 => { // VPS
                vps_parsed = vps::parse_vps(payload);
            }
            33 => { // SPS
                sps_parsed = sps::parse_sps(payload);
            }
            34 => { // PPS
                pps_parsed = pps::parse_pps(payload, &sps_parsed);
            }
            0..=31 => { // VCL NAL units (slice)
                if let (Some(ref pps), Some(ref sps)) = (&pps_parsed, &sps_parsed) {
                    let sh = slice_header::parse_slice_header(payload, pps, sps);
                    slice_headers.push(sh);
                }
            }
            _ => {}
        }

        offset = nal_end;
    }

    HevcSyntax {
        nal_units,
        vps: vps_parsed,
        sps: sps_parsed,
        pps: pps_parsed,
        slice_headers,
    }
}

fn find_start_code(data: &[u8], from: usize) -> usize {
    for i in from..data.len().saturating_sub(3) {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            return i;
        }
        if i + 3 < data.len() && data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1 {
            return i;
        }
    }
    0
}

fn nal_type_to_string(nut: u8) -> String {
    match nut {
        0 => "TRAIL_N".to_string(),
        1 => "TRAIL_R".to_string(),
        2 => "TSA_N".to_string(),
        3 => "TSA_R".to_string(),
        4 => "STSA_N".to_string(),
        5 => "STSA_R".to_string(),
        6 => "RADL_N".to_string(),
        7 => "RADL_R".to_string(),
        8 => "RASL_N".to_string(),
        9 => "RASL_R".to_string(),
        16 => "BLA_W_LP".to_string(),
        17 => "BLA_W_RADL".to_string(),
        18 => "BLA_N_LP".to_string(),
        19 => "IDR_W_RADL".to_string(),
        20 => "IDR_N_LP".to_string(),
        21 => "CRA_NUT".to_string(),
        32 => "VPS_NUT".to_string(),
        33 => "SPS_NUT".to_string(),
        34 => "PPS_NUT".to_string(),
        35 => "AUD_NUT".to_string(),
        36 => "EOS_NUT".to_string(),
        37 => "EOB_NUT".to_string(),
        38 => "FD_NUT".to_string(),
        39 => "PREFIX_SEI_NUT".to_string(),
        40 => "SUFFIX_SEI_NUT".to_string(),
        _ => format!("NAL_{}", nut),
    }
}
```

- [ ] **Step 3: Write VPS parser (src-tauri/src/analyzer/hevc/vps.rs)**

```rust
use crate::types::VideoParameterSet;

pub fn parse_vps(data: &[u8]) -> Option<VideoParameterSet> {
    if data.len() < 3 { return None; }
    let vps_id = (data[0] >> 1) & 0x0F;
    // Simplified — parse key fields from VPS RBSP
    VideoParameterSet {
        vps_id,
        max_layers: 1,
        max_sub_layers: 1,
    }.into()
}
```

- [ ] **Step 4: Write SPS parser (src-tauri/src/analyzer/hevc/sps.rs)**

```rust
use crate::types::SequenceParameterSet;

pub fn parse_sps(data: &[u8]) -> Option<SequenceParameterSet> {
    if data.len() < 10 { return None; }

    // Simplified SPS parsing — extract key fields
    // Full implementation would use a bit reader for ue(v)/se(v) syntax
    let chroma_format_idc = read_ue(data, &mut 0)?;
    let separate_colour_plane_flag = if chroma_format_idc == 3 {
        read_bit(data, &mut 0)?
    } else { false };

    let pic_width = readUE(data, &mut 0).unwrap_or(0);
    let pic_height = readUE(data, &mut 0).unwrap_or(0);

    let bit_depth = 8 + readUE(data, &mut 0).unwrap_or(0);

    let profile = match chroma_format_idc {
        1 => "4:2:0",
        2 => "4:2:2",
        3 => "4:4:4",
        _ => "Unknown",
    }.to_string();

    SequenceParameterSet {
        profile,
        level: "Unknown".to_string(),
        chroma_format: if separate_colour_plane_flag { "4:0:0" } else { profile.as_str() },
        pic_width,
        pic_height,
        bit_depth: bit_depth as u8,
    }.into()
}

// Simplified exponential-Golomb reader
fn readUE(data: &[u8], _bit_pos: &mut usize) -> Option<u32> {
    if data.is_empty() { return None; }
    // This is a placeholder — real implementation needs a proper bit reader
    Some(0)
}

fn read_bit(data: &[u8], _bit_pos: &mut usize) -> Option<bool> {
    if data.is_empty() { return None; }
    Some(false)
}
```

- [ ] **Step 5: Write PPS parser (src-tauri/src/analyzer/hevc/pps.rs)**

```rust
use crate::types::{PictureParameterSet, SequenceParameterSet};

pub fn parse_pps(data: &[u8], sps: &Option<SequenceParameterSet>) -> Option<PictureParameterSet> {
    if data.is_empty() { return None; }
    let pps_id = 0; // Simplified
    let sps_id = 0;

    PictureParameterSet { pps_id, sps_id }.into()
}
```

- [ ] **Step 6: Write Slice Header parser (src-tauri/src/analyzer/hevc/slice_header.rs)**

```rust
use crate::types::{HevcSliceHeader, SequenceParameterSet, PictureParameterSet};

pub fn parse_slice_header(data: &[u8], pps: &PictureParameterSet, sps: &SequenceParameterSet) -> HevcSliceHeader {
    if data.is_empty() {
        return HevcSliceHeader {
            slice_type: 0,
            first_slice_segment_in_pic_flag: false,
            dependent_slice_segment_flag: false,
            slice_segment_address: 0,
            pps_id: 0,
            num_entry_point_offsets: None,
            offset_len_minus1: None,
            pic_width: sps.pic_width,
            pic_height: sps.pic_height,
            tile_enabled: false,
        };
    }

    let first_slice = (data[0] & 0x80) != 0;
    let slice_type = if data.len() > 1 { data[1] & 0x03 } else { 0 };

    HevcSliceHeader {
        slice_type,
        first_slice_segment_in_pic_flag: first_slice,
        dependent_slice_segment_flag: false,
        slice_segment_address: 0,
        pps_id: pps.pps_id,
        num_entry_point_offsets: None,
        offset_len_minus1: None,
        pic_width: sps.pic_width,
        pic_height: sps.pic_height,
        tile_enabled: false,
    }
}
```

- [ ] **Step 7: Wire HEVC into HEIF parser**

In heif_parser.rs, after parsing boxes, extract the primary item's bitstream and parse:

```rust
// In analyze_heif, before returning:
let codec_syntax = if format == ImageFormat::Heic {
    // Find primary item data and parse as HEVC
    let primary_data = extract_primary_item_data(&bytes, &structure);
    primary_data.map(|data| CodecSyntax::Hevc(hevc::parse_hevc_bitstream(&data)))
} else {
    None
};
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: add HEVC NAL/VPS/SPS/PPS/Slice Header parser"
```

---

### Task 10: AV1 OBU + Frame Header + Tile Info Parser

**Files:**
- Create: `src-tauri/src/analyzer/av1/mod.rs`
- Create: `src-tauri/src/analyzer/av1/obu.rs`
- Create: `src-tauri/src/analyzer/av1/sequence_header.rs`
- Create: `src-tauri/src/analyzer/av1/frame_header.rs`
- Create: `src-tauri/src/analyzer/av1/tile_info.rs`

- [ ] **Step 1: Write AV1 module (src-tauri/src/analyzer/av1/mod.rs)**

```rust
pub mod obu;
pub mod sequence_header;
pub mod frame_header;
pub mod tile_info;

pub use obu::parse_av1_bitstream;
```

- [ ] **Step 2: Write OBU parser (src-tauri/src/analyzer/av1/obu.rs)**

```rust
use crate::types::{Av1Syntax, Obu, SequenceHeader, Av1FrameHeader, Av1TileInfo, ColorConfig, QuantizerParams};
use super::{sequence_header, frame_header, tile_info};

pub fn parse_av1_bitstream(data: &[u8]) -> Av1Syntax {
    let mut obus = Vec::new();
    let mut sequence_header: Option<SequenceHeader> = None;
    let mut frame_headers = Vec::new();
    let mut tile_info: Option<Av1TileInfo> = None;

    let mut offset = 0;
    while offset < data.len() {
        if data[offset] == 0 {
            offset += 1;
            continue;
        }

        let obu_header = data[offset];
        let obu_forbidden = (obu_header >> 7) & 1;
        let obu_type = (obu_header >> 3) & 0x07;
        let obu_extension_flag = (obu_header >> 2) & 1;
        let obu_has_size_field = (obu_header >> 1) & 1;
        let temporal_id = 0;
        let spatial_id = 0;

        let mut obu_start = offset;
        offset += 1;

        // Parse extension header if present
        if obu_extension_flag == 1 {
            if offset < data.len() {
                let ext = data[offset];
                offset += 1;
            }
        }

        // Parse OBU size
        let obu_size = if obu_has_size_field {
            let (size, consumed) = read_leb128(&data[offset..]);
            offset += consumed;
            size
        } else {
            data.len() - offset
        };

        let obu_end = std::cmp::min(offset + obu_size, data.len());
        let obu_data = &data[offset..obu_end];

        obus.push(Obu {
            obu_type: obu_type_to_string(obu_type),
            obu_size,
            temporal_id,
            spatial_id,
            offset: obu_start as u64,
        });

        // Parse OBU payload
        match obu_type {
            1 => { // SEQUENCE_HEADER
                sequence_header = sequence_header::parse_sequence_header(obu_data);
            }
            3 => { // TILE_GROUP
                tile_info = tile_info::parse_tile_group(obu_data, &sequence_header);
            }
            5 => { // FRAME
                if let Some(fh) = frame_header::parse_frame_header(obu_data, &sequence_header) {
                    frame_headers.push(fh);
                }
            }
            _ => {}
        }

        offset = obu_end;
    }

    Av1Syntax {
        obus,
        sequence_header,
        frame_headers,
        tile_info,
    }
}

fn obu_type_to_string(obu_type: u8) -> String {
    match obu_type {
        1 => "OBU_SEQUENCE_HEADER".to_string(),
        2 => "OBU_TEMPORAL_DELIMITER".to_string(),
        3 => "OBU_FRAME_HEADER".to_string(),
        4 => "OBU_TILE_GROUP".to_string(),
        5 => "OBU_FRAME".to_string(),
        6 => "OBU_REDUNDANT_FRAME_HEADER".to_string(),
        7 => "OBU_METADATA".to_string(),
        _ => format!("OBU_TYPE_{}", obu_type),
    }
}

fn read_leb128(data: &[u8]) -> (usize, usize) {
    let mut value: usize = 0;
    let mut shift: u32 = 0;
    let mut consumed = 0;

    for byte in data.iter() {
        consumed += 1;
        value |= ((*byte & 0x7F) as usize) << shift;
        if (*byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if consumed >= 8 { break; }
    }

    (value, consumed)
}
```

- [ ] **Step 3: Write Sequence Header parser (src-tauri/src/analyzer/av1/sequence_header.rs)**

```rust
use crate::types::{SequenceHeader, ColorConfig};

pub fn parse_sequence_header(data: &[u8]) -> Option<SequenceHeader> {
    if data.len() < 5 { return None; }

    let seq_profile = data[0] & 0x07;
    let still_picture = (data[0] >> 7) & 1;
    let reduced_still_picture_header = (data[0] >> 6) & 1;

    let level_idx = if data.len() > 4 { data[4] } else { 0 };

    let bit_depth = match seq_profile {
        0 | 1 => 8,
        2 => 10,
        _ => 12,
    };

    let chroma_format = match seq_profile {
        0 => "4:2:0",
        1 => "4:4:4",
        2 => "4:4:4",
        _ => "Unknown",
    }.to_string();

    let frame_width = 0; // From frame header
    let frame_height = 0;

    let color_config = if seq_profile == 0 {
        Some(ColorConfig {
            matrix_coefficients: "BT.709".to_string(),
        })
    } else {
        Some(ColorConfig {
            matrix_coefficients: "BT.2020".to_string(),
        })
    };

    Some(SequenceHeader {
        profile: seq_profile,
        level: format!("{}", level_idx),
        bit_depth,
        chroma_format,
        frame_width,
        frame_height,
        color_config,
    })
}
```

- [ ] **Step 4: Write Frame Header parser (src-tauri/src/analyzer/av1/frame_header.rs)**

```rust
use crate::types::{Av1FrameHeader, QuantizerParams, SequenceHeader};

pub fn parse_frame_header(data: &[u8], seq: &Option<SequenceHeader>) -> Option<Av1FrameHeader> {
    if data.is_empty() { return None; }

    let frame_type = if data.len() > 1 {
        match (data[0] >> 5) & 0x03 {
            0 => "key",
            1 => "inter",
            2 => "intra-only",
            3 => "switch",
            _ => "unknown",
        }.to_string()
    } else { "unknown".to_string() };

    let show_frame = data.len() > 1 && (data[0] >> 7) != 0;

    Av1FrameHeader {
        frame_type,
        show_frame,
        frame_size: None,
        order_hint: 0,
        primary_ref_frame: 0,
        quantizer_params: None,
    }
}
```

- [ ] **Step 5: Write Tile Info parser (src-tauri/src/analyzer/av1/tile_info.rs)**

```rust
use crate::types::{Av1TileInfo, SequenceHeader};

pub fn parse_tile_group(data: &[u8], seq: &Option<SequenceHeader>) -> Option<Av1TileInfo> {
    if data.is_empty() { return None; }
    None
}
```

- [ ] **Step 6: Wire AV1 into HEIF parser**

Similar to HEVC wiring, but for AVIF format:

```rust
let codec_syntax = if format == ImageFormat::Avif {
    let primary_data = extract_primary_item_data(&bytes, &structure);
    primary_data.map(|data| CodecSyntax::Av1(av1::parse_av1_bitstream(&data)))
} else {
    None
};
```

- [ ] **Step 7: Run cargo check**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: add AV1 OBU/Sequence Header/Frame Header parser"
```

---

### Task 11: CodecSyntaxTab (HEVC + AV1 Frontend)

**Files:**
- Create: `src/components/tabs/CodecSyntaxTab.svelte`

- [ ] **Step 1: Write CodecSyntaxTab (src/components/tabs/CodecSyntaxTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis, CodecSyntax } from '../../lib/types';

  let { data }: { data: ImageAnalysis } = $props();
</script>

<div class="codec-syntax">
  {#if data.codec_syntax}
    {#if 'hevc' in data.codec_syntax}
      <h3>HEVC Syntax</h3>
      <section class="section">
        <h4>NAL Units ({data.codec_syntax.hevc.nal_units.length})</h4>
        <table class="data-table">
          <tr><th>Type</th><th>Layer</th><th>Temporal</th><th>Size</th><th>Offset</th></tr>
          {#each data.codec_syntax.hevc.nal_units as nal}
            <tr>
              <td><code>{nal.nal_type}</code></td>
              <td>{nal.nuh_layer_id}</td>
              <td>{nal.nuh_temporal_id}</td>
              <td>{nal.size}</td>
              <td>0x{nal.offset.toString(16)}</td>
            </tr>
          {/each}
        </table>
      </section>

      {#if data.codec_syntax.hevc.vps}
        <section class="section">
          <h4>VPS</h4>
          <table class="data-table">
            <tr><td>VPS ID</td><td>{data.codec_syntax.hevc.vps.vps_id}</td></tr>
            <tr><td>Max Layers</td><td>{data.codec_syntax.hevc.vps.max_layers}</td></tr>
            <tr><td>Max Sub-layers</td><td>{data.codec_syntax.hevc.vps.max_sub_layers}</td></tr>
          </table>
        </section>
      {/if}

      {#if data.codec_syntax.hevc.sps}
        <section class="section">
          <h4>SPS</h4>
          <table class="data-table">
            <tr><td>Profile</td><td>{data.codec_syntax.hevc.sps.profile}</td></tr>
            <tr><td>Level</td><td>{data.codec_syntax.hevc.sps.level}</td></tr>
            <tr><td>Chroma</td><td>{data.codec_syntax.hevc.sps.chroma_format}</td></tr>
            <tr><td>Size</td><td>{data.codec_syntax.hevc.sps.pic_width}×{data.codec_syntax.hevc.sps.pic_height}</td></tr>
            <tr><td>Bit Depth</td><td>{data.codec_syntax.hevc.sps.bit_depth}</td></tr>
          </table>
        </section>
      {/if}

      {#if data.codec_syntax.hevc.slice_headers.length > 0}
        <section class="section">
          <h4>Slice Headers ({data.codec_syntax.hevc.slice_headers.length})</h4>
          <table class="data-table">
            <tr><th>Type</th><th>First</th><th>Dependent</th><th>Address</th><th>PPS</th></tr>
            {#each data.codec_syntax.hevc.slice_headers as sh}
              <tr>
                <td>{sh.slice_type === 0 ? 'I' : sh.slice_type === 1 ? 'P' : 'B'}</td>
                <td>{sh.first_slice_segment_in_pic_flag ? 'Yes' : 'No'}</td>
                <td>{sh.dependent_slice_segment_flag ? 'Yes' : 'No'}</td>
                <td>{sh.slice_segment_address}</td>
                <td>{sh.pps_id}</td>
              </tr>
            {/each}
          </table>
        </section>
      {/if}
    {:else}
      <h3>AV1 Syntax</h3>
      <section class="section">
        <h4>OBUs ({data.codec_syntax.av1.obus.length})</h4>
        <table class="data-table">
          <tr><th>Type</th><th>Size</th><th>Temporal</th><th>Spatial</th><th>Offset</th></tr>
          {#each data.codec_syntax.av1.obus as obu}
            <tr>
              <td><code>{obu.obu_type}</code></td>
              <td>{obu.obu_size}</td>
              <td>{obu.temporal_id}</td>
              <td>{obu.spatial_id}</td>
              <td>0x{obu.offset.toString(16)}</td>
            </tr>
          {/each}
        </table>
      </section>

      {#if data.codec_syntax.av1.sequence_header}
        <section class="section">
          <h4>Sequence Header</h4>
          <table class="data-table">
            <tr><td>Profile</td><td>{data.codec_syntax.av1.sequence_header.profile}</td></tr>
            <tr><td>Level</td><td>{data.codec_syntax.av1.sequence_header.level}</td></tr>
            <tr><td>Bit Depth</td><td>{data.codec_syntax.av1.sequence_header.bit_depth}</td></tr>
            <tr><td>Chroma</td><td>{data.codec_syntax.av1.sequence_header.chroma_format}</td></tr>
            <tr><td>Size</td><td>{data.codec_syntax.av1.sequence_header.frame_width}×{data.codec_syntax.av1.sequence_header.frame_height}</td></tr>
            {#if data.codec_syntax.av1.sequence_header.color_config}
              <tr><td>Matrix</td><td>{data.codec_syntax.av1.sequence_header.color_config.matrix_coefficients}</td></tr>
            {/if}
          </table>
        </section>
      {/if}
    {/if}
  {:else}
    <p class="no-codec">No codec syntax data available for this format.</p>
  {/if}
</div>

<style>
  .codec-syntax {
    font-size: 0.875rem;
  }
  .section {
    margin-bottom: 1.5rem;
  }
  .section h3 {
    margin: 0 0 0.5rem;
    color: var(--color-text, #e2e8f0);
    font-size: 1rem;
  }
  .section h4 {
    margin: 0 0 0.35rem;
    color: var(--color-muted, #94a3b8);
    font-size: 0.85rem;
  }
  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .data-table th {
    text-align: left;
    color: var(--color-muted, #64748b);
    font-weight: 500;
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #334155);
  }
  .data-table td {
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #1e293b);
  }
  code {
    color: var(--color-accent, #818cf8);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
  }
  .no-codec {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
  }
</style>
```

- [ ] **Step 2: Wire CodecSyntaxTab into MainPanel**

```svelte
{:else if activeTab === 'codec'}
  <CodecSyntaxTab data={$currentImage} />
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add CodecSyntaxTab for HEVC/AV1 syntax display"
```

---

## Phase 5: Final Polish

### Task 12: Wire GridTab + MetadataTab + StructureTab (tree view) + Error Handling

**Files:**
- Create: `src/components/ui/TreeView.svelte`
- Create: `src/components/ui/TreeItem.svelte`
- Create: `src/components/ui/HexDump.svelte`
- Create: `src/components/ui/DataTable.svelte`
- Create: `src/components/tabs/MetadataTab.svelte`
- Modify: `src/components/tabs/StructureTab.svelte`
- Modify: `src/components/MainPanel.svelte`

- [ ] **Step 1: Write TreeItem (src/components/ui/TreeItem.svelte)**

```svelte
<script lang="ts">
  import type { FileBlock } from '../../lib/types';
  import TreeItem from './TreeItem.svelte';

  let { block }: { block: FileBlock } = $props();
  let expanded = $state(true);

  function toggle() {
    expanded = !expanded;
  }
</script>

<div class="tree-item">
  <div class="tree-row" onclick={toggle}>
    {#if block.children.length > 0}
      <span class="toggle">{expanded ? '▾' : '▸'}</span>
    {:else}
      <span class="toggle" style="visibility:hidden"> </span>
    {/if}
    <code class="block-name">{block.name}</code>
    <span class="block-size">{block.length}B</span>
    {#if block.decoded_info}
      <span class="decoded">{block.decoded_info}</span>
    {/if}
  </div>
  {#if expanded && block.children.length > 0}
    <div class="children">
      {#each block.children as child}
        <TreeItem block={child} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .tree-item {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.8rem;
  }
  .tree-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0;
    cursor: pointer;
  }
  .tree-row:hover {
    background: rgba(255,255,255,0.03);
  }
  .toggle {
    width: 1em;
    color: var(--color-muted, #64748b);
  }
  .block-name {
    color: var(--color-accent, #818cf8);
    font-weight: 600;
  }
  .block-size {
    color: var(--color-muted, #64748b);
    font-size: 0.7rem;
  }
  .decoded {
    color: var(--color-muted, #475569);
    font-size: 0.7rem;
  }
  .children {
    margin-left: 1.25em;
    border-left: 1px solid var(--color-border, #1e293b);
    padding-left: 0.5em;
  }
</style>
```

- [ ] **Step 2: Write TreeView (src/components/ui/TreeView.svelte)**

```svelte
<script lang="ts">
  import type { FileBlock } from '../../lib/types';
  import TreeItem from './TreeItem.svelte';

  let { blocks }: { blocks: FileBlock[] } = $props();
</script>

<div class="tree-view">
  {#each blocks as block}
    <TreeItem block={block} />
  {/each}
</div>

<style>
  .tree-view {
    font-size: 0.875rem;
  }
</style>
```

- [ ] **Step 3: Write HexDump (src/components/ui/HexDump.svelte)**

```svelte
<script lang="ts">
  let { data, maxLines = 8 }: { data: string | null; maxLines?: number } = $props();
  import { formatHex } from '../../lib/utils';

  const lines = formatHex(data).slice(0, maxLines);
</script>

<div class="hex-dump">
  {#each lines as line, i}
    <code>{line}</code>
  {/each}
  {#if formatHex(data).length > maxLines}
    <span class="truncated">... truncated</span>
  {/if}
</div>

<style>
  .hex-dump {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.75rem;
    line-height: 1.5;
  }
  .hex-dump code {
    display: block;
    color: var(--color-muted, #94a3b8);
  }
  .truncated {
    color: var(--color-muted, #475569);
    font-style: italic;
  }
</style>
```

- [ ] **Step 4: Write DataTable (src/components/ui/DataTable.svelte)**

```svelte
<script lang="ts">
  let { rows }: { rows: [string, string][] } = $props();
</script>

<table class="data-table">
  {#each rows as [key, value]}
    <tr>
      <td>{key}</td>
      <td>{value}</td>
    </tr>
  {/each}
</table>

<style>
  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .data-table td {
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #1e293b);
  }
  .data-table td:first-child {
    color: var(--color-muted, #64748b);
    width: 160px;
  }
</style>
```

- [ ] **Step 5: Rewrite StructureTab with TreeView (src/components/tabs/StructureTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';
  import TreeView from '../ui/TreeView.svelte';
  import HexDump from '../ui/HexDump.svelte';

  let { data }: { data: ImageAnalysis } = $props();
  let selectedBlock = $state<number | null>(null);
</script>

<div class="structure-tab">
  <div class="header">
    <h3>{data.file_name}</h3>
    <span class="summary">{data.format.toUpperCase()} · {data.width}×{data.height} · {data.structure.length} blocks</span>
  </div>
  <TreeView blocks={data.structure} />
</div>

<style>
  .structure-tab {
    font-size: 0.875rem;
  }
  .header {
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--color-border, #1e293b);
  }
  .header h3 {
    margin: 0;
    font-size: 0.95rem;
  }
  .summary {
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
  }
</style>
```

- [ ] **Step 6: Write MetadataTab (src/components/tabs/MetadataTab.svelte)**

```svelte
<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';

  let { data }: { data: ImageAnalysis } = $props();

  // Group by standard
  const grouped: Record<string, typeof data.metadata> = {};
  for (const entry of data.metadata) {
    if (!grouped[entry.standard]) {
      grouped[entry.standard] = [];
    }
    grouped[entry.standard].push(entry);
  }
  const standards = Object.keys(grouped);
</script>

<div class="metadata-tab">
  {#if standards.length > 0}
    {#each standards as standard}
      <section class="section">
        <h3>{standard} ({grouped[standard].length})</h3>
        <table class="data-table">
          {#each grouped[standard] as entry}
            <tr>
              <td>{entry.tag_name}</td>
              <td>{entry.tag_value}</td>
            </tr>
          {/each}
        </table>
      </section>
    {/each}
  {:else}
    <p class="no-metadata">No metadata found in this image.</p>
  {/if}
</div>

<style>
  .metadata-tab {
    font-size: 0.875rem;
  }
  .section {
    margin-bottom: 1.5rem;
  }
  .section h3 {
    margin: 0 0 0.5rem;
    color: var(--color-text, #e2e8f0);
    font-size: 0.9rem;
  }
  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .data-table td {
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #1e293b);
  }
  .data-table td:first-child {
    color: var(--color-muted, #64748b);
    width: 180px;
    font-family: 'SF Mono', 'Cascadia Code', monospace;
  }
  .no-metadata {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
  }
</style>
```

- [ ] **Step 7: Wire MetadataTab and GridTab into MainPanel**

```svelte
import MetadataTab from './tabs/MetadataTab.svelte';
import GridTab from './tabs/GridTab.svelte';

// In tab content:
{:else if activeTab === 'metadata'}
  <MetadataTab data={$currentImage} />
{:else if activeTab === 'grid'}
  <GridTab data={$currentImage} />
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: complete all UI tabs with TreeView, DataTable, HexDump components"
```

---

