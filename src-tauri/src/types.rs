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
    pub thumbnail_base64: Option<String>,
    pub structure: Vec<FileBlock>,
    pub metadata: Vec<MetadataEntry>,
    pub channels: Option<ChannelData>,
    pub icc_profile: Option<IccInfo>,
    pub codec_syntax: Option<CodecSyntax>,
    pub grid: Option<GridInfo>,
    pub analysis_errors: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct FileBlock {
    pub name: String,
    pub offset: u64,
    pub length: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_info: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_preview: Option<String>,
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
pub struct GifFrame {
    pub index: u32,
    pub delay_ms: u16,
    pub width: u32,
    pub height: u32,
    pub image_base64: String,
}

#[derive(Debug, Serialize)]
pub struct ChannelData {
    pub rgb: Option<RgbChannels>,
    pub yuv: Option<YuvChannels>,
    pub image_base64: Option<String>,
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
    pub nal_unit_type: String,
    pub nuh_layer_id: u8,
    pub nuh_temporal_id_plus1: u8,
    pub size: usize,
    pub offset: u64,
}

#[derive(Debug, Serialize)]
pub struct HevcSliceHeader {
    pub nal_unit_type: String,
    pub slice_type: u8,
    pub first_slice_segment_in_pic_flag: bool,
    pub dependent_slice_segment_flag: bool,
    pub no_output_of_prior_pics_flag: bool,
    pub slice_segment_address: u32,
    pub slice_pic_parameter_set_id: u8,
    pub pic_output_flag: Option<bool>,
    pub colour_plane_id: Option<u8>,
    pub num_entry_point_offsets: Option<u32>,
    pub offset_len_minus1: Option<u8>,
    pub short_term_ref_pic_set_sps_flag: Option<bool>,
    pub slice_sao_luma_flag: Option<bool>,
    pub slice_sao_chroma_flag: Option<bool>,
    pub num_ref_idx_active_override_flag: Option<bool>,
    pub num_ref_idx_l0_active_minus1: Option<u8>,
    pub num_ref_idx_l1_active_minus1: Option<u8>,
    pub mvd_l1_zero_flag: Option<bool>,
    pub cabac_init_flag: Option<bool>,
    pub collocated_from_l0_flag: Option<bool>,
    pub collocated_ref_idx: Option<u8>,
    pub five_minus_max_num_merge_cand: Option<u8>,
    pub slice_qp_delta: Option<i32>,
    pub slice_cb_qp_offset: Option<i8>,
    pub slice_cr_qp_offset: Option<i8>,
    pub cu_chroma_qp_offset_enabled_flag: Option<bool>,
    pub deblocking_filter_override_flag: Option<bool>,
    pub slice_deblocking_filter_disabled_flag: Option<bool>,
    pub beta_offset_div2: Option<i8>,
    pub tc_offset_div2: Option<i8>,
    pub slice_loop_filter_across_slices_enabled_flag: Option<bool>,
    pub pic_width_in_luma_samples: u32,
    pub pic_height_in_luma_samples: u32,
    pub tiles_enabled_flag: bool,
    pub entropy_coding_sync_enabled_flag: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct VideoParameterSet {
    pub vps_video_parameter_set_id: u8,
    pub vps_base_layer_internal_flag: bool,
    pub vps_base_layer_available_flag: bool,
    pub vps_max_layers_minus1: u8,
    pub vps_max_sub_layers_minus1: u8,
    pub vps_temporal_id_nesting_flag: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SequenceParameterSet {
    pub sps_seq_parameter_set_id: u8,
    pub general_profile_idc: u8,
    pub general_level_idc: u8,
    pub sps_max_sub_layers_minus1: u8,
    pub sps_temporal_id_nesting_flag: bool,
    pub chroma_format_idc: u8,
    pub separate_colour_plane_flag: bool,
    pub pic_width_in_luma_samples: u32,
    pub pic_height_in_luma_samples: u32,
    pub conformance_window_flag: bool,
    pub conf_win_left_offset: u32,
    pub conf_win_right_offset: u32,
    pub conf_win_top_offset: u32,
    pub conf_win_bottom_offset: u32,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub log2_max_pic_order_cnt_lsb_minus4: u8,
    pub log2_min_luma_coding_block_size_minus3: u8,
    pub log2_diff_max_min_luma_coding_block_size: u8,
    pub log2_min_transform_block_size_minus2: u8,
    pub log2_diff_max_min_transform_block_size: u8,
    pub max_transform_hierarchy_depth_inter: u8,
    pub max_transform_hierarchy_depth_intra: u8,
    pub amp_enabled_flag: bool,
    pub sample_adaptive_offset_enabled_flag: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PictureParameterSet {
    pub pps_pic_parameter_set_id: u8,
    pub pps_seq_parameter_set_id: u8,
    pub dependent_slice_segments_enabled_flag: bool,
    pub output_flag_present_flag: bool,
    pub num_extra_slice_header_bits: u8,
    pub sign_data_hiding_enabled_flag: bool,
    pub cabac_init_present_flag: bool,
    pub num_ref_idx_l0_default_active_minus1: u8,
    pub num_ref_idx_l1_default_active_minus1: u8,
    pub init_qp_minus26: i8,
    pub constrained_intra_pred_flag: bool,
    pub transform_skip_enabled_flag: bool,
    pub cu_qp_delta_enabled_flag: bool,
    pub diff_cu_qp_delta_depth: u8,
    pub pps_cb_qp_offset: i8,
    pub pps_cr_qp_offset: i8,
    pub pps_slice_chroma_qp_offsets_present_flag: bool,
    pub weighted_pred_flag: bool,
    pub weighted_bipred_flag: bool,
    pub transquant_bypass_enabled_flag: bool,
    pub tiles_enabled_flag: bool,
    pub entropy_coding_sync_enabled_flag: bool,
    pub num_tile_columns_minus1: u8,
    pub num_tile_rows_minus1: u8,
    pub uniform_spacing_flag: bool,
    pub loop_filter_across_tiles_enabled_flag: bool,
    pub pps_loop_filter_across_slices_enabled_flag: bool,
    pub deblocking_filter_control_present_flag: bool,
    pub deblocking_filter_override_enabled_flag: bool,
    pub pps_deblocking_filter_disabled_flag: bool,
    pub pps_beta_offset_div2: i8,
    pub pps_tc_offset_div2: i8,
    pub lists_modification_present_flag: bool,
    pub log2_parallel_merge_level_minus2: u8,
    pub slice_segment_header_extension_present_flag: bool,
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
    pub seq_profile: u8,
    pub reduced_still_picture_header: bool,
    pub seq_level_idx_0: u8,
    pub max_frame_width_minus1: u32,
    pub max_frame_height_minus1: u32,
    pub use_128x128_superblock: bool,
    pub enable_superres: bool,
    pub enable_cdef: bool,
    pub enable_restoration: bool,
    pub color_config: ColorConfig,
}

#[derive(Debug, Serialize)]
pub struct ColorConfig {
    pub high_bitdepth: bool,
    pub twelve_bit: bool,
    pub mono_chrome: bool,
    pub color_description_present_flag: bool,
    pub color_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
    pub color_range: bool,
    pub subsampling_x: bool,
    pub subsampling_y: bool,
    pub chroma_sample_position: Option<u8>,
    pub separate_uv_delta_q: bool,
}

#[derive(Debug, Serialize)]
pub struct Av1FrameHeader {
    pub show_existing_frame: bool,
    pub frame_to_show_map_idx: Option<u8>,
    pub frame_type: String,
    pub show_frame: bool,
    pub error_resilient_mode: Option<bool>,
    pub disable_cdf_update: Option<bool>,
    pub allow_screen_content_tools: Option<bool>,
    pub force_integer_mv: Option<bool>,
    pub frame_size_override_flag: Option<bool>,
    pub render_and_frame_size_different: Option<bool>,
    pub allow_intrabc: Option<bool>,
    pub refresh_frame_flags: Option<u8>,
    pub frame_size: Option<(u32, u32)>,
    pub order_hint: u8,
    pub primary_ref_frame: u8,
    pub quantizer_params: Option<QuantizerParams>,
    pub delta_q_y_dc_coded: Option<bool>,
    pub delta_q_u_dc_coded: Option<bool>,
    pub delta_q_u_ac_coded: Option<bool>,
    pub using_qmatrix: Option<bool>,
    pub segmentation_enabled: Option<bool>,
    pub reduced_tx_set: Option<bool>,
    pub use_128x128_superblock: bool,
    pub tile_cols_log2: u8,
    pub tile_rows_log2: u8,
    pub uniform_tile_spacing_flag: bool,
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
