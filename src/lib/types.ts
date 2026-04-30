export type ImageFormat = 'png' | 'jpeg' | 'webp' | 'gif' | 'avif' | 'heic';

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

export interface Histogram {
  channel: string;
  bins: number[];
}

export interface ChannelData {
  rgb: RgbChannels | null;
  yuv: YuvChannels | null;
  histograms: Histogram[];
  thumbnail_base64: string | null;
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
  nal_type: string;
  nuh_layer_id: number;
  nuh_temporal_id: number;
  size: number;
  offset: number;
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
  matrix_coefficients: string;
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

export interface QuantizerParams {
  base_q_idx: number;
  delta_q_present: boolean;
  delta_q_res: number;
}

export interface Av1FrameHeader {
  frame_type: string;
  show_frame: boolean;
  frame_size: [number, number] | null;
  order_hint: number;
  primary_ref_frame: number;
  quantizer_params: QuantizerParams | null;
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
  structure: FileBlock[];
  metadata: MetadataEntry[];
  channels: ChannelData | null;
  icc_profile: IccInfo | null;
  codec_syntax: CodecSyntax | null;
  grid: GridInfo | null;
  analysis_errors: string[];
}
