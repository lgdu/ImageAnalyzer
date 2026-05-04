# ImageAnalyzer

A desktop application for deep inspection of image files. View internal structure, metadata, codec syntax, channel statistics, and ICC profiles — no hex editors or external tools required.

## Features

- **File Structure** — Visualize the internal box/chunk/marker hierarchy of image files as an expandable tree
- **Metadata** — Extract and display EXIF, IPTC, and XMP metadata in a unified view
- **Codec Syntax** — Parse HEVC NAL units (VPS/SPS/PPS/slice headers) and AV1 OBUs (sequence headers, frame headers, tile info)
- **Channel Statistics** — On-demand RGB/YUV channel analysis with histograms
- **ICC Profile** — Detailed ICC profile parsing including primaries, TRCs, LUTs, and all tags
- **GIF Frames** — Extract all animation frames with timing information
- **HEIF Grid** — View HEIF image grid layout and tile composition
- **Thumbnail Preview** — Automatic thumbnail generation for all supported formats

## Supported Formats

| Format | Extensions | Notes |
|--------|-----------|-------|
| PNG | `.png` | Full chunk parsing, ancillary data, ICC |
| JPEG | `.jpg`, `.jpeg` | Marker parsing, EXIF, XMP, IPTC, ICC |
| WebP | `.webp` | VP8/VP8L/VP8X, animation, metadata |
| GIF | `.gif` | Full frame extraction with delays |
| HEIC/HEIF | `.heic`, `.heif` | HEVC codec parsing, box hierarchy, grid |
| AVIF | `.avif` | AV1 codec parsing, box hierarchy |

## Development

### Prerequisites

- Rust (edition 2021+)
- Node.js 18+
- macOS (HEIC/AVIF thumbnails use the built-in `sips` tool)

### Getting Started

```bash
# Install frontend dependencies
npm install

# Run the app in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

### Useful Commands

```bash
# Frontend only
npm run dev          # Start Vite dev server (port 1420)
npm run build        # Build frontend → dist/
npm run preview      # Preview built dist
npm run check        # Type-check with svelte-check

# Rust only
cargo check          # Fast compile check
cargo clippy         # Lint (treats warnings as errors)
cargo fmt            # Format code
cargo test           # Run tests
```

## Architecture

ImageAnalyzer is built with **Tauri v2** (desktop shell), **Rust** (backend parsers), and **Svelte 5** (frontend UI, runes mode).

The Rust backend detects image formats via magic bytes, then dispatches to format-specific parsers that build a hierarchical `FileBlock` tree. Metadata (EXIF, IPTC, XMP, ICC) is extracted and returned alongside codec-level details (HEVC NAL units, AV1 OBUs). Heavy operations like channel statistics and ICC parsing are loaded on-demand via separate Tauri commands.

The Svelte 5 frontend displays results in a tabbed panel with a sidebar file list and drop zone.

```
src/                          ← Frontend (Svelte 5 + TypeScript)
├── components/tabs/          ← Tab components (Metadata, Structure, Codec, etc.)
├── components/ui/            ← Shared UI components (GridView, Histogram)
└── lib/                      ← Types, store, utilities

src-tauri/src/                ← Backend (Rust)
├── commands.rs               ← Tauri command handlers
├── analyzer/                 ← Format parsers (PNG, JPEG, WebP, GIF, HEIF, HEVC, AV1)
└── types.rs                  ← Shared data types
```
