use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
    Avif,
    Heic,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct FileBlock {
    pub name: String,
    pub offset: u64,
    pub length: u64,
    pub data_preview: Option<String>,
    pub decoded_info: Option<String>,
    pub children: Vec<FileBlock>,
}

#[derive(Debug, Serialize)]
pub struct MetadataEntry {
    pub standard: String,
    pub tag_name: String,
    pub tag_value: String,
    pub raw_value: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Rgb,
    Yuv,
}

#[derive(Debug, Serialize)]
pub struct ChannelData {
    pub rgb: Option<RgbChannels>,
    pub yuv: Option<YuvChannels>,
    pub histograms: Vec<Histogram>,
    pub thumbnail_base64: Option<String>,
    pub ycbcr_subsampling: Option<String>,
    pub color_matrix: String,
}

#[derive(Debug, Serialize)]
pub struct RgbChannels {
    pub r: SingleChannel,
    pub g: SingleChannel,
    pub b: SingleChannel,
    pub a: Option<SingleChannel>,
}

#[derive(Debug, Serialize)]
pub struct YuvChannels {
    pub y: SingleChannel,
    pub cb: SingleChannel,
    pub cr: SingleChannel,
}

#[derive(Debug, Serialize)]
pub struct SingleChannel {
    pub name: String,
    pub min: u8,
    pub max: u8,
    pub mean: f64,
    pub median: u8,
    pub std_dev: f64,
}

#[derive(Debug, Serialize)]
pub struct Histogram {
    pub channel: String,
    pub bins: Vec<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecSyntax {
    Hevc(HevcSyntax),
    Av1(Av1Syntax),
}

#[derive(Debug, Serialize)]
pub struct HevcSyntax {
    pub nal_units: Vec<NalUnit>,
    pub vps: Option<VideoParameterSet>,
    pub sps: Option<SequenceParameterSet>,
    pub pps: Option<PictureParameterSet>,
    pub slice_headers: Vec<HevcSliceHeader>,
}

#[derive(Debug, Serialize)]
pub struct NalUnit {
    pub nal_type: String,
    pub nuh_layer_id: u8,
    pub nuh_temporal_id: u8,
    pub size: usize,
    pub offset: u64,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct VideoParameterSet {
    pub vps_id: u8,
    pub max_layers: u8,
    pub max_sub_layers: u8,
}

#[derive(Debug, Serialize)]
pub struct SequenceParameterSet {
    pub profile: String,
    pub level: String,
    pub chroma_format: String,
    pub pic_width: u32,
    pub pic_height: u32,
    pub bit_depth: u8,
}

#[derive(Debug, Serialize)]
pub struct PictureParameterSet {
    pub pps_id: u8,
    pub sps_id: u8,
}

#[derive(Debug, Serialize)]
pub struct Av1Syntax {
    pub obus: Vec<Obu>,
    pub sequence_header: Option<SequenceHeader>,
    pub frame_headers: Vec<Av1FrameHeader>,
    pub tile_info: Option<Av1TileInfo>,
}

#[derive(Debug, Serialize)]
pub struct Obu {
    pub obu_type: String,
    pub obu_size: usize,
    pub temporal_id: u8,
    pub spatial_id: u8,
    pub offset: u64,
}

#[derive(Debug, Serialize)]
pub struct SequenceHeader {
    pub profile: u8,
    pub level: String,
    pub bit_depth: u8,
    pub chroma_format: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub color_config: Option<ColorConfig>,
}

#[derive(Debug, Serialize)]
pub struct ColorConfig {
    pub matrix_coefficients: String,
}

#[derive(Debug, Serialize)]
pub struct Av1FrameHeader {
    pub frame_type: String,
    pub show_frame: bool,
    pub frame_size: Option<(u32, u32)>,
    pub order_hint: u8,
    pub primary_ref_frame: u8,
    pub quantizer_params: Option<QuantizerParams>,
}

#[derive(Debug, Serialize)]
pub struct QuantizerParams {
    pub base_q_idx: u8,
    pub delta_q_present: bool,
    pub delta_q_res: u8,
}

#[derive(Debug, Serialize)]
pub struct Av1TileInfo {
    pub num_tiles: u32,
    pub rows: u32,
    pub cols: u32,
    pub tile_width: Vec<u32>,
    pub tile_height: Vec<u32>,
    pub context_update_tile_id: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GridInfo {
    pub rows: u32,
    pub cols: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub tiles: Vec<GridTile>,
}

#[derive(Debug, Serialize)]
pub struct GridTile {
    pub item_id: u16,
    pub width: u32,
    pub height: u32,
    pub horizontal_offset: u32,
    pub vertical_offset: u32,
    pub codec: String,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct Matrix3x3 {
    pub m: [[f64; 3]; 3],
}

#[derive(Debug, Serialize)]
pub struct LutInfo {
    pub name: String,
    pub clut_points: Option<u8>,
    pub input_channels: u8,
    pub output_channels: u8,
}

#[derive(Debug, Serialize)]
pub struct IccTag {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub tag_type: String,
    pub decoded_value: Option<String>,
}
