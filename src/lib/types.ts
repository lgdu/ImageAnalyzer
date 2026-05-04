export type ImageFormat = 'png' | 'jpeg' | 'webp' | 'gif' | 'avif' | 'heic';

export interface GifFrame {
  index: number;
  delay_ms: number;
  width: number;
  height: number;
  image_base64: string;
}

export interface FileBlock {
  name: string;
  offset: number;
  length: number;
  data_preview: string | null;
  decoded_info: string | null;
  fields: [string, string][];
  children: FileBlock[];
}

export interface MetadataEntry {
  standard: string;
  tag_name: string;
  tag_value: string;
  raw_value: string | null;
}

export interface SingleChannel {
  name: string;
  min: number;
  max: number;
  mean: number;
  median: number;
  std_dev: number;
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

export interface ChannelData {
  rgb: RgbChannels | null;
  yuv: YuvChannels | null;
  image_base64: string | null;
  ycbcr_subsampling: string | null;
  color_matrix: string;
}

export interface IccTag {
  name: string;
  offset: number;
  size: number;
  tag_type: string;
  decoded_value: string | null;
}

export interface LutInfo {
  name: string;
  clut_points: number | null;
  input_channels: number;
  output_channels: number;
}

export interface PrimariesInfo {
  red_x: number;
  red_y: number;
  red_z: number;
  green_x: number;
  green_y: number;
  green_z: number;
  blue_x: number;
  blue_y: number;
  blue_z: number;
  white_x: number;
  white_y: number;
  white_z: number;
}

export interface Matrix3x3 {
  m: number[][];
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

export interface NalUnit {
  nal_unit_type: string;
  nuh_layer_id: number;
  nuh_temporal_id_plus1: number;
  size: number;
  offset: number;
}

export interface VideoParameterSet {
  vps_video_parameter_set_id: number;
  vps_base_layer_internal_flag: boolean;
  vps_base_layer_available_flag: boolean;
  vps_max_layers_minus1: number;
  vps_max_sub_layers_minus1: number;
  vps_temporal_id_nesting_flag: boolean;
}

export interface SequenceParameterSet {
  sps_seq_parameter_set_id: number;
  general_profile_idc: number;
  general_level_idc: number;
  sps_max_sub_layers_minus1: number;
  sps_temporal_id_nesting_flag: boolean;
  chroma_format_idc: number;
  separate_colour_plane_flag: boolean;
  pic_width_in_luma_samples: number;
  pic_height_in_luma_samples: number;
  conformance_window_flag: boolean;
  conf_win_left_offset: number;
  conf_win_right_offset: number;
  conf_win_top_offset: number;
  conf_win_bottom_offset: number;
  bit_depth_luma_minus8: number;
  bit_depth_chroma_minus8: number;
  log2_max_pic_order_cnt_lsb_minus4: number;
  log2_min_luma_coding_block_size_minus3: number;
  log2_diff_max_min_luma_coding_block_size: number;
  log2_min_transform_block_size_minus2: number;
  log2_diff_max_min_transform_block_size: number;
  max_transform_hierarchy_depth_inter: number;
  max_transform_hierarchy_depth_intra: number;
  amp_enabled_flag: boolean;
  sample_adaptive_offset_enabled_flag: boolean;
}

export interface PictureParameterSet {
  pps_pic_parameter_set_id: number;
  pps_seq_parameter_set_id: number;
  dependent_slice_segments_enabled_flag: boolean;
  output_flag_present_flag: boolean;
  num_extra_slice_header_bits: number;
  sign_data_hiding_enabled_flag: boolean;
  cabac_init_present_flag: boolean;
  num_ref_idx_l0_default_active_minus1: number;
  num_ref_idx_l1_default_active_minus1: number;
  init_qp_minus26: number;
  constrained_intra_pred_flag: boolean;
  transform_skip_enabled_flag: boolean;
  cu_qp_delta_enabled_flag: boolean;
  diff_cu_qp_delta_depth: number;
  pps_cb_qp_offset: number;
  pps_cr_qp_offset: number;
  pps_slice_chroma_qp_offsets_present_flag: boolean;
  weighted_pred_flag: boolean;
  weighted_bipred_flag: boolean;
  transquant_bypass_enabled_flag: boolean;
  tiles_enabled_flag: boolean;
  entropy_coding_sync_enabled_flag: boolean;
  num_tile_columns_minus1: number;
  num_tile_rows_minus1: number;
  uniform_spacing_flag: boolean;
  loop_filter_across_tiles_enabled_flag: boolean;
  pps_loop_filter_across_slices_enabled_flag: boolean;
  deblocking_filter_control_present_flag: boolean;
  deblocking_filter_override_enabled_flag: boolean;
  pps_deblocking_filter_disabled_flag: boolean;
  pps_beta_offset_div2: number;
  pps_tc_offset_div2: number;
  lists_modification_present_flag: boolean;
  log2_parallel_merge_level_minus2: number;
  slice_segment_header_extension_present_flag: boolean;
}

export interface HevcSliceHeader {
  nal_unit_type: string;
  slice_type: number;
  first_slice_segment_in_pic_flag: boolean;
  dependent_slice_segment_flag: boolean;
  no_output_of_prior_pics_flag: boolean;
  slice_segment_address: number;
  slice_pic_parameter_set_id: number;
  pic_output_flag: boolean | null;
  colour_plane_id: number | null;
  num_entry_point_offsets: number | null;
  offset_len_minus1: number | null;
  short_term_ref_pic_set_sps_flag: boolean | null;
  slice_sao_luma_flag: boolean | null;
  slice_sao_chroma_flag: boolean | null;
  num_ref_idx_active_override_flag: boolean | null;
  num_ref_idx_l0_active_minus1: number | null;
  num_ref_idx_l1_active_minus1: number | null;
  mvd_l1_zero_flag: boolean | null;
  cabac_init_flag: boolean | null;
  collocated_from_l0_flag: boolean | null;
  collocated_ref_idx: number | null;
  five_minus_max_num_merge_cand: number | null;
  slice_qp_delta: number | null;
  slice_cb_qp_offset: number | null;
  slice_cr_qp_offset: number | null;
  cu_chroma_qp_offset_enabled_flag: boolean | null;
  deblocking_filter_override_flag: boolean | null;
  slice_deblocking_filter_disabled_flag: boolean | null;
  beta_offset_div2: number | null;
  tc_offset_div2: number | null;
  slice_loop_filter_across_slices_enabled_flag: boolean | null;
  pic_width_in_luma_samples: number;
  pic_height_in_luma_samples: number;
  tiles_enabled_flag: boolean;
  entropy_coding_sync_enabled_flag: boolean;
}

export interface HevcSyntax {
  nal_units: NalUnit[];
  vps: VideoParameterSet | null;
  sps: SequenceParameterSet | null;
  pps: PictureParameterSet | null;
  slice_headers: HevcSliceHeader[];
}

export interface Obu {
  obu_type: string;
  obu_size: number;
  temporal_id: number;
  spatial_id: number;
  offset: number;
}

export interface ColorConfig {
  high_bitdepth: boolean;
  twelve_bit: boolean;
  mono_chrome: boolean;
  color_description_present_flag: boolean;
  color_primaries: number;
  transfer_characteristics: number;
  matrix_coefficients: number;
  color_range: boolean;
  subsampling_x: boolean;
  subsampling_y: boolean;
  chroma_sample_position: number | null;
  separate_uv_delta_q: boolean;
}

export interface SequenceHeader {
  seq_profile: number;
  reduced_still_picture_header: boolean;
  seq_level_idx_0: number;
  max_frame_width_minus1: number;
  max_frame_height_minus1: number;
  use_128x128_superblock: boolean;
  enable_superres: boolean;
  enable_cdef: boolean;
  enable_restoration: boolean;
  color_config: ColorConfig;
}

export interface QuantizerParams {
  base_q_idx: number;
  delta_q_present: boolean;
  delta_q_res: number;
}

export interface Av1FrameHeader {
  show_existing_frame: boolean;
  frame_to_show_map_idx: number | null;
  frame_type: string;
  show_frame: boolean;
  error_resilient_mode: boolean | null;
  disable_cdf_update: boolean | null;
  allow_screen_content_tools: boolean | null;
  force_integer_mv: boolean | null;
  frame_size_override_flag: boolean | null;
  render_and_frame_size_different: boolean | null;
  allow_intrabc: boolean | null;
  refresh_frame_flags: number | null;
  frame_size: [number, number] | null;
  order_hint: number;
  primary_ref_frame: number;
  quantizer_params: QuantizerParams | null;
  delta_q_y_dc_coded: boolean | null;
  delta_q_u_dc_coded: boolean | null;
  delta_q_u_ac_coded: boolean | null;
  using_qmatrix: boolean | null;
  segmentation_enabled: boolean | null;
  reduced_tx_set: boolean | null;
  use_128x128_superblock: boolean;
  tile_cols_log2: number;
  tile_rows_log2: number;
  uniform_tile_spacing_flag: boolean;
}

export interface Av1TileInfo {
  num_tiles: number;
  rows: number;
  cols: number;
  tile_width: number[];
  tile_height: number[];
  context_update_tile_id: number | null;
}

export interface Av1Syntax {
  obus: Obu[];
  sequence_header: SequenceHeader | null;
  frame_headers: Av1FrameHeader[];
  tile_info: Av1TileInfo | null;
}

export type CodecSyntax = { hevc: HevcSyntax } | { av1: Av1Syntax };

export interface GridTile {
  item_id: number;
  width: number;
  height: number;
  horizontal_offset: number;
  vertical_offset: number;
  codec: string;
}

export interface GridInfo {
  rows: number;
  cols: number;
  output_width: number;
  output_height: number;
  tiles: GridTile[];
}

export interface ImageAnalysis {
  file_name: string;
  file_path: string;
  file_size: number;
  format: ImageFormat;
  width: number;
  height: number;
  color_type: string;
  bit_depth: number;
  has_alpha: boolean;
  thumbnail_base64: string | null;
  structure: FileBlock[];
  metadata: MetadataEntry[];
  channels: ChannelData | null;
  icc_profile: IccInfo | null;
  codec_syntax: CodecSyntax | null;
  grid: GridInfo | null;
  analysis_errors: string[];
}
