# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ImageAnalyzer is a **Tauri v2 desktop application** (macOS, Apple Silicon) built with **Svelte 5** (runes mode) and **Rust**. It analyzes image files (PNG, JPEG, WebP, GIF, HEIC, AVIF) and displays detailed structural, metadata, and codec-level information in a dark-themed UI.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri 2.10 |
| Frontend | Svelte 5 (runes), TypeScript 5 |
| Build/dev | Vite 6 |
| Backend | Rust (edition 2021) |
| Image decoding | `image` crate + macOS `sips` for HEIC/AVIF |
| Metadata | `kamadak-exif` (EXIF), custom parsers (ICC, IPTC, XMP) |
| Serialization | `serde` + `serde_json` |

## Key Commands

```bash
# Development (starts Tauri + Vite dev server on port 1420)
cargo tauri dev

# Production build
cargo tauri build

# Frontend-only dev (no Tauri shell)
npm run dev          # Vite dev server
npm run build        # Vite build → dist/
npm run preview      # Preview built dist

# Type checking
npm run check        # svelte-check

# Rust checks
cargo check          # Fast compile check
cargo clippy         # Lint (treat warnings as errors)
cargo fmt            # Format Rust code
cargo test           # Run tests
```

## Architecture

### Backend (Rust) — `src-tauri/src/`

```
src-tauri/src/
├── main.rs              # Tauri entry point
├── lib.rs               # Module declarations
├── commands.rs          # Tauri #[command] handlers
├── types.rs             # Shared data types (ImageAnalysis, FileBlock, etc.)
├── utils.rs             # File I/O utilities
└── analyzer/
    ├── mod.rs           # Format detection via magic bytes
    ├── png_parser.rs    # PNG chunk parsing
    ├── jpeg_parser.rs   # JPEG marker parsing
    ├── webp_parser.rs   # WebP chunk parsing
    ├── gif_parser.rs    # GIF frame extraction
    ├── heif_parser.rs   # HEIF/HEIC/AVIF box parsing
    ├── hevc.rs          # HEVC NAL unit parsing (VPS/SPS/PPS/slice)
    ├── av1.rs           # AV1 OBU parsing (sequence header/frame header/tiles)
    ├── icc_parser.rs    # ICC profile tag parsing
    ├── exif_reader.rs   # EXIF metadata extraction
    ├── iptc_reader.rs   # IPTC metadata extraction
    ├── xmp_reader.rs    # XMP metadata extraction
    └── channel_split.rs # Per-channel RGB/YUV statistics
```

**Tauri commands** exposed to the frontend (`commands.rs`):
- `analyze_image(path)` → full `ImageAnalysis` (structure, metadata, codec syntax)
- `get_channels(path)` → heavy channel statistics (decodes full image)
- `get_icc_profile(path)` → detailed ICC profile info
- `analyze_batch(paths)` → batch analysis
- `get_gif_frames(path)` → extract all GIF frames as base64 PNGs

### Frontend (Svelte 5) — `src/`

```
src/
├── App.svelte                  # Main layout: sidebar + content area
├── main.ts                     # App bootstrap
├── app.d.ts                    # Global type declarations
├── app.html                    # HTML shell
├── lib/
│   ├── types.ts                # Frontend type mirrors of Rust types
│   ├── store.svelte.ts         # Svelte 5 rune-based global store
│   └── utils.ts                # Utility functions
└── components/
    ├── DropZone.svelte          # File drop zone for loading images
    ├── FileList.svelte          # Sidebar file list
    ├── MainPanel.svelte         # Main content panel with tabs
    ├── ThumbnailCard.svelte     # Image thumbnail display
    ├── tabs/
    │   ├── MetadataTab.svelte   # EXIF/IPTC/XMP metadata
    │   ├── StructureTab.svelte  # File structure tree
    │   ├── CodecSyntaxTab.svelte# HEVC/AV1 codec details
    │   ├── ChannelsTab.svelte   # Channel statistics
    │   ├── ColorInfoTab.svelte  # ICC/color profile info
    │   ├── GridTab.svelte       # HEIF grid info
    │   └── TreeNode.svelte      # Recursive tree node component
    └── ui/
        ├── GridView.svelte      # Grid layout component
        └── Histogram.svelte     # Channel histogram visualization
```

### State Management

Uses a **Svelte 5 class-based store** with `$state` runes (`src/lib/store.svelte.ts`):
- `fileList: ImageAnalysis[]` — all analyzed images
- `currentImage: ImageAnalysis | null` — currently selected image
- `isAnalyzing: boolean` — loading state
- `error: string | null` — error message

### Data Flow

1. User drops/selects files → `DropZone.svelte` calls `analyze_image()` Tauri command
2. Rust backend detects format via magic bytes, dispatches to format-specific parser
3. Parser builds `ImageAnalysis` with file structure tree, metadata, and codec syntax
4. Thumbnail generated via `image` crate (or macOS `sips` for HEIC/AVIF)
5. Frontend receives `ImageAnalysis`, stores in global store, renders tab views
6. Heavy operations (channel stats, ICC parsing) are on-demand via separate Tauri commands

### Format-Specific Parsers

Each format parser extracts a hierarchical `FileBlock` tree showing the file's internal structure:
- **PNG**: IHDR, IDAT, ancillary chunks (tEXt, iTXt, iCCP, etc.)
- **JPEG**: SOI, APPn markers (EXIF, XMP, ICC), SOS, DQT, DHT, frame/scan headers
- **WebP**: VP8/VP8L/VP8X chunks, animation data, EXIF/XMP/ICCP
- **GIF**: Header, LCGE, image descriptors, extension blocks, all frames
- **HEIF/HEIC/AVIF**: Box hierarchy (ftyp, moov, meta, iloc, iref, iprp, ipco, ipma, idat, mdat)
  - HEVC: NAL units with VPS, SPS, PPS, slice header parsing
  - AV1: OBUs with sequence header, frame header, tile info

## Supported Formats

PNG, JPEG, WebP, GIF, HEIC/HEIF, AVIF

## Notes

- macOS-only for now (HEIC/AVIF thumbnail decoding uses `sips`)
- Svelte 5 uses runes mode (`compilerOptions: { runes: true }` in svelte.config.js)
- Vite dev server pinned to port 1420 (strict)
- All large image data (base64 thumbnails, channel images) passed as base64 strings over Tauri IPC
