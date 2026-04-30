# ImageAnalyzer 设计文档

## 概述

一款桌面图片分析软件，用于深入分析图像文件的语法结构、元数据和通道信息。支持 PNG, JPEG, WebP, GIF, AVIF, HEIC 格式，提供文件结构树形查看、元数据提取（EXIF/XMP/IPTC/ICC 等）、分通道可视化和 HEVC/AV1 编解码语法展示。

## 技术选型

- **框架**: Tauri 2.0
- **前端**: Svelte 5 + TypeScript
- **后端**: Rust
- **构建**: Tauri CLI
- **目标平台**: macOS + Windows

## 架构

```
┌─────────────────────────────────────────────────────┐
│                    Tauri Window (Svelte)              │
│                                                      │
│  ┌──────────────────┬──────────────────────────────┐ │
│  │   左栏 (30%)      │         右栏 (70%)            │ │
│  │                  │                              │ │
│  │  · DropZone       │  Tab 1: Structure            │ │
│  │  · FileList       │  Tab 2: Metadata             │ │
│  │  · ThumbnailCard  │  Tab 3: Channels             │ │
│  │                  │  Tab 4: Color Info           │ │
│  │                  │  Tab 5: Codec Syntax         │ │
│  │                  │  Tab 6: Grid (HEIC/AVIF)     │ │
│  └──────────────────┴──────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
         │                              ▲
         ▼                              │
    文件拖入/选择              Tauri IPC Commands
    ──────────────────────────────►  Rust Backend
                                     └── analyzer/
```

**数据流**: 用户拖入图片 → Tauri 捕获文件路径 → Rust 读取字节并解析 → 返回 JSON → Svelte 渲染面板。

## 数据模型

### ImageAnalysis（主分析结果）

```rust
struct ImageAnalysis {
    file_name: String,
    file_path: String,
    file_size: u64,
    format: ImageFormat,       // PNG, JPEG, WebP, GIF, AVIF, HEIC
    width: u32,
    height: u32,
    color_type: String,        // RGB, RGBA, Grayscale, YCbCr, etc.
    bit_depth: u8,
    has_alpha: bool,
    structure: Vec<FileBlock>, // PNG chunks / JPEG markers / GIF blocks / HEIF boxes
    metadata: Vec<MetadataEntry>,
    channels: Option<ChannelData>,
    icc_profile: Option<IccInfo>,
    codec_syntax: Option<CodecSyntax>, // HEIC→HEVC, AVIF→AV1
    grid: Option<GridInfo>,            // HEIC/AVIF image grid
    analysis_errors: Vec<String>,
}
```

### FileBlock（文件结构单元）

```rust
struct FileBlock {
    name: String,               // "IHDR", "IDAT", "SOF0", "GCE", etc.
    offset: u64,
    length: u64,
    data_preview: Option<String>, // hex string (首 16 字节)
    decoded_info: Option<String>, // human-readable info
    children: Vec<FileBlock>,    // nested blocks
}
```

### MetadataEntry

```rust
struct MetadataEntry {
    standard: String,           // EXIF, XMP, IPTC, tEXt, zTXt, iTXt, ICC, COM
    tag_name: String,
    tag_value: String,
    raw_value: Option<String>,
}
```

### ChannelData

```rust
enum ColorMode {
    Rgb(RgbChannels),
    Yuv(YuvChannels),
}

struct RgbChannels {
    r: SingleChannel,
    g: SingleChannel,
    b: SingleChannel,
    a: Option<SingleChannel>,
}

struct YuvChannels {
    y: SingleChannel,   // Luma (brightness)
    cb: SingleChannel,  // Chroma blue
    cr: SingleChannel,  // Chroma red
}

struct ChannelData {
    rgb: Option<RgbChannels>,       // 原始 RGB 通道（sRGB）
    yuv: Option<YuvChannels>,       // RGB→YCbCr 转换（BT.709/BT.601/BT.2020）
    histograms: Vec<Histogram>,     // 当前选中模式的直方图
    thumbnail_base64: Option<String>, // 单通道灰度缩略图
    ycbcr_subsampling: Option<String>, // "4:2:0", "4:2:2", "4:4:4"
    color_matrix: String,           // 推断的转换矩阵: BT.601/BT.709/BT.2020
}

struct SingleChannel {
    name: String,
    min: u8,
    max: u8,
    mean: f64,
    median: u8,
    std_dev: f64,
}
```

### CodecSyntax（HEVC / AV1）

```rust
enum CodecSyntax {
    Hevc(HevcSyntax),
    Av1(Av1Syntax),
}

struct HevcSyntax {
    nal_units: Vec<NalUnit>,
    vps: Option<VideoParameterSet>,
    sps: Option<SequenceParameterSet>,
    pps: Option<PictureParameterSet>,
    slice_headers: Vec<HevcSliceHeader>,
}

struct NalUnit {
    nal_type: String,
    nuh_layer_id: u8,
    nuh_temporal_id: u8,
    size: usize,
    offset: u64,
}

struct HevcSliceHeader {
    slice_type: u8,                        // 0=I, 1=P, 2=B
    first_slice_segment_in_pic_flag: bool,
    dependent_slice_segment_flag: bool,
    slice_segment_address: u32,
    pps_id: u8,
    num_entry_point_offsets: Option<u32>,
    offset_len_minus1: Option<u8>,
    // 从关联 PPS/SPS 展开的关键信息（便于展示）
    pic_width: u32,
    pic_height: u32,
    tile_enabled: bool,
}

struct VideoParameterSet {
    vps_id: u8,
    max_layers: u8,
    max_sub_layers: u8,
    // ... key fields
}

struct SequenceParameterSet {
    profile: String,
    level: String,
    chroma_format: String,
    pic_width: u32,
    pic_height: u32,
    bit_depth: u8,
    // ... key fields
}

struct PictureParameterSet {
    pps_id: u8,
    sps_id: u8,
    // ... key fields
}

struct Av1Syntax {
    obus: Vec<Obu>,
    sequence_header: Option<SequenceHeader>,
    frame_headers: Vec<Av1FrameHeader>,
    tile_info: Option<Av1TileInfo>,
}

struct Obu {
    obu_type: String,        // OBU_SEQUENCE_HEADER, OBU_FRAME_HEADER, etc.
    obu_size: usize,
    temporal_id: u8,
    spatial_id: u8,
    offset: u64,
}

struct SequenceHeader {
    profile: u8,
    level: String,
    bit_depth: u8,
    chroma_format: String,
    frame_width: u32,
    frame_height: u32,
    color_config: Option<ColorConfig>,
}

struct Av1FrameHeader {
    frame_type: String,                   // key/inter/switch/intra-only
    show_frame: bool,
    frame_size: Option<(u32, u32)>,
    order_hint: u8,
    primary_ref_frame: u8,
    quantizer_params: Option<QuantizerParams>,
}

struct QuantizerParams {
    base_q_idx: u8,
    delta_q_present: bool,
    delta_q_res: u8,
}

struct Av1TileInfo {
    num_tiles: u32,
    rows: u32,
    cols: u32,
    tile_width: Vec<u32>,
    tile_height: Vec<u32>,
    context_update_tile_id: Option<u32>,
}

struct GridInfo {
    rows: u32,
    cols: u32,
    output_width: u32,
    output_height: u32,
    tiles: Vec<GridTile>,
}

struct GridTile {
    item_id: u16,
    width: u32,
    height: u32,
    horizontal_offset: u32,
    vertical_offset: u32,
    codec: String,  // "hvc1" / "av01"
}

struct IccInfo {
    // Header
    size: u32,
    cmm_type: String,              // "appl", "msft", "adbe", etc.
    version: String,               // "4.3.0.0"
    profile_class: String,         // input/display/output/link/devicelink/abstract/namedColor
    color_space: String,           // "RGB ", "CMYK", "GRAY", "XYZ ", etc.
    pcs: String,                   // "XYZ " or "Lab "
    platform: Option<String>,      // "APPL", "MSFT", etc.
    rendering_intent: String,      // perceptual/relative/saturation/absolute
    illuminant: (f64, f64, f64),   // XYZ values (D50 default)
    creator: Option<String>,
    description: Option<String>,

    // Transfer curves (TRC / gamma)
    transfer_function: Option<TransferFunction>, // sRGB / linear / gamma / parametric / LUT
    red_trc: Option<TransferFunction>,           // 单独 R 通道 TRC (ICC v4)
    green_trc: Option<TransferFunction>,         // 单独 G 通道 TRC
    blue_trc: Option<TransferFunction>,          // 单独 B 通道 TRC

    // Colorimetric data
    primaries: Option<PrimariesInfo>,            // chromaticities (r,g,b white point)
    matrix: Option<Matrix3x3>,                   // RGB→XYZ conversion matrix
    luts: Vec<LutInfo>,                          // AToB / BToA LUT tables

    // Tags summary
    tag_count: u32,
    tags: Vec<IccTag>,           // name, offset, size, type
    raw_base64: Option<String>,
}

enum TransferFunction {
    Srgb,
    Linear,
    Gamma(f64),
    Parametric(ParametricCurve),
    Lut16(Vec<u16>),
    LutFloat(Vec<f32>),
}

struct ParametricCurve {
    function_type: u16,
    parameters: Vec<f64>,        // 0-7 params depending on type
}

struct PrimariesInfo {
    red_x: f64, red_y: f64, red_z: f64,
    green_x: f64, green_y: f64, green_z: f64,
    blue_x: f64, blue_y: f64, blue_z: f64,
    white_x: f64, white_y: f64, white_z: f64,
}

struct Matrix3x3 {
    m: [[f64; 3]; 3],
}

struct LutInfo {
    name: String,                // "A2B0", "B2A0", etc.
    clut_points: Option<u8>,     // CLUT grid size
    input_channels: u8,
    output_channels: u8,
}

struct IccTag {
    name: String,                // "desc", "cprt", "rXYZ", "bkpt", "wtpt", etc.
    offset: u32,
    size: u32,
    tag_type: String,            // "text", "XYZ ", "curv", "para", "sf32", "mluc", etc.
    decoded_value: Option<String>,
}
```

## Rust 后端模块

```
src-tauri/src/
├── main.rs                  // Tauri 入口
├── commands.rs              // Tauri commands
├── analyzer/
│   ├── mod.rs               // 分析器分发入口
│   ├── png_parser.rs        // PNG chunks (IHDR/IDAT/tEXt/zTXt/iTXt/iCCP)
│   ├── jpeg_parser.rs       // JPEG markers (SOF/SOS/DQT) + EXIF + XMP + IPTC + COM
│   ├── webp_parser.rs       // WebP RIFF chunks (VP8/VP8L/VP8X/EXIF/XMP)
│   ├── gif_parser.rs        // GIF blocks (LSD/GCE/CE/PlainText/AppExt/COM)
│   ├── heif_parser.rs       // HEIF boxes + HEVC NAL / AV1 OBU 解析 + Grid 解析
│   ├── exif_reader.rs       // EXIF 通用提取
│   ├── xmp_reader.rs        // XMP 解析
│   ├── iptc_reader.rs       // IPTC 解析
│   ├── channel_split.rs     // 通道分离 + 直方图计算
│   ├── hevc/                // HEVC NAL + Slice Header 解析
│   │   ├── mod.rs
│   │   ├── nalu.rs
│   │   ├── vps.rs
│   │   ├── sps.rs
│   │   ├── pps.rs
│   │   └── slice_header.rs
│   └── av1/                 // AV1 OBU + Frame Header + Tile 解析
│       ├── mod.rs
│       ├── obu.rs
│       ├── sequence_header.rs
│       ├── frame_header.rs
│       └── tile_info.rs
├── types.rs                 // 所有数据结构
└── utils.rs                 // hex dump, byte helpers
```

## 关键 Rust 依赖

| Crate | 用途 |
|-------|------|
| `image` | 基础图像加载、通道分离、直方图采样 |
| `kamadak-exif` | EXIF 解析 |
| `png` | PNG chunk 解析 |
| `byteorder` | 字节序处理 |
| `serde` + `serde_json` | JSON 序列化 |
| `mp4parse` | MP4/HEIF 容器 box 解析 |
| `quick-xml` | XMP XML 解析 |
| `tauri` | 桌面框架 |

HEVC NAL/Slice Header 和 AV1 OBU/Frame Header 解析需要自定义实现（Rust 生态缺少成熟库）。

## Tauri Commands

```rust
#[tauri::command]
async fn analyze_image(file_path: String) -> Result<ImageAnalysis, String>;

#[tauri::command]
async fn analyze_batch(file_paths: Vec<String>) -> Result<Vec<ImageAnalysis>, String>;
```

## 前端组件

```
src/
├── lib/
│   ├── types.ts               // TS 类型镜像 Rust 结构
│   ├── store.ts               // Svelte store: currentImage, fileList
│   └── utils.ts               // formatBytes, hexToRgb 等
├── components/
│   ├── DropZone.svelte        // 拖入区域
│   ├── FileList.svelte        // 文件列表
│   ├── ThumbnailCard.svelte   // 单张缩略卡片
│   ├── MainPanel.svelte       // 右栏主面板容器
│   ├── tabs/
│   │   ├── StructureTab.svelte  // 文件结构树
│   │   ├── MetadataTab.svelte   // 元数据表格/列表
│   │   ├── ChannelsTab.svelte   // 分通道预览 + 直方图 (RGB/YUV 模式切换)
│   │   ├── ColorInfoTab.svelte  // ICC 深度展示 (matrix/TRC/primaries/LUT/tags)
│   │   ├── CodecSyntaxTab.svelte// HEVC/AV1 语法树 (VPS/SPS/PPS/OBU/Slice)
│   │   └── GridTab.svelte       // HEIC/AVIF Grid 可视化
│   └── ui/
│       ├── TreeView.svelte      // 可折叠树形控件
│       ├── TreeItem.svelte      // 单个树节点
│       ├── DataTable.svelte     // 元数据表格
│       ├── Histogram.svelte     // Canvas 直方图
│       ├── HexDump.svelte       // 十六进制预览
│       └── GridView.svelte      // Grid 网格可视化
└── App.svelte
```

## 各格式解析内容

### PNG

- IHDR (width, height, bit_depth, color_type, compression, filter, interlace)
- IDAT (数量 + 累计大小)
- tEXt / zTXt / iTXt (文本注释)
- iCCP (ICC profile)
- PLTE (调色板)
- gAMA, cHRM, sRGB, pHYs, tRNS (辅助 chunks)
- bKGD, hIST, sBIT (其他)

### JPEG

- SOF0-SOF15 (Start of Frame — 编码参数)
- SOS (Start of Scan)
- DQT (量化表)
- DHT (Huffman 表)
- APP0 (JFIF)
- APP1 (EXIF)
- APP2 (ICC Profile / FPX)
- APP13 (IPTC / Photoshop)
- APP14 (Adobe)
- COM (注释)
- EOI (End of Image)

### WebP

- VP8 (lossy) / VP8L (lossless) / VP8X (extended)
- RIFF chunks: EXIF, XMP, ANIM, ANMF
- VP8X 标志位 (Alpha, ICC, Animation, Exif, XMP)
- 画布尺寸

### GIF

- Logical Screen Descriptor (width, height, GCT flag, color resolution, sort flag, pixel aspect)
- Global/Local Color Table
- Graphic Control Extension (disposal method, transparency flag, delay, transparent color index)
- Comment Extension
- Plain Text Extension
- Application Extension (NETSCAPE2.0 loop count + 尝试提取嵌入的 XMP/EXIF)
- Image Descriptor (left, top, width, height, LCT flag, interlace, sort flag)
- 注释字段

### AVIF

- ftyp, meta, hdlr, pitm, iprp, ipco, ipma, iloc, iinf
- **Grid 解析**: `grid` property — rows, cols, output_width, output_height, 每个 tile 的 item_id/尺寸/偏移
- AV1 OBU 解析:
  - OBU_SEQUENCE_HEADER (profile, level, bit_depth, chroma_subsampling, color_config, matrix_coefficients)
  - OBU_FRAME (frame_type, show_frame, order_hint, quantizer_params)
  - OBU_FRAME_HEADER
  - OBU_TILE_GROUP (tile rows/cols, tile sizes, context_update_tile_id)
  - OBU_METADATA
- 主图像项 + 所有辅助图像项的 AV1 bitstream
- YCbCr 转换矩阵: 从 color_config.matrix_coefficients 推断 (BT.601/BT.709/BT.2020)

### HEIC

- ftyp, meta, hdlr, pitm, iprp, ipco, ipma, iloc, iinf
- **Grid 解析**: `grid` property — rows, cols, output_width, output_height, 每个 tile 的 item_id/尺寸/偏移
- HEVC NAL 解析:
  - VPS (Video Parameter Set — vps_id, max_layers, max_sub_layers)
  - SPS (Sequence Parameter Set — profile, level, chroma_format, pic_width, pic_height, bit_depth)
  - PPS (Picture Parameter Set — pps_id, sps_id, tile info)
  - NAL 单元列表 (type, layer_id, temporal_id, size, offset)
  - Slice Headers (slice_type, first_slice_segment_in_pic_flag, dependent_slice_segment_flag, slice_segment_address, pps_id)
- 主图像项 + 所有辅助图像项的 HEVC bitstream
- YCbCr 转换矩阵: 从 SPS chroma_format / VUI 推断 (BT.601/BT.709/BT.2020)

## 错误处理

- 不支持的格式: 提示用户，不崩溃
- 损坏文件: 继续解析成功部分，记录 `analysis_errors` 数组
- 大文件 (100MB+): 直方图采样而非全量计算，限制 hex dump 长度
- 元数据解析失败: 标记该标准不可用，不阻塞其他分析
- HEVC/AV1 bitstream 解析失败: 记录失败的 NAL/OBU 位置，展示已解析部分

## 性能考虑

- 文件读取使用 Rust 异步 IO
- 通道分离和直方图计算使用 `image` crate，单线程已足够（单张图片分析场景）
- 批量分析时并行处理
- 前端缩略图使用 base64 编码，限制最大 200px
- Grid 图像多个 tile 逐个解析，避免一次性加载全部

## 安全考虑

- 不执行文件内容中的任何代码或脚本
- 限制单次分析文件大小（默认 500MB）
- 沙箱化文件访问，仅读取用户明确拖入的文件
- 不将任何数据发送到外部服务

## 视觉设计

- 深色主题为主（类似 hex editor / 分析工具风格）
- 等宽字体用于 hex dump 和结构名称
- 树形控件支持折叠/展开
- 直方图使用 Canvas 绘制
- 色彩通道使用对应颜色高亮（R=红, G=绿, B=蓝, A=灰）
- Grid 视图使用网格线分隔各 tile，标注 tile ID 和尺寸
