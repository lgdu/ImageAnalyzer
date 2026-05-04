<script lang="ts">
  import type { CodecSyntax } from '../../lib/types';

  let { codec }: { codec: CodecSyntax } = $props();

  let mode: 'hevc' | 'av1' = $derived(
    'hevc' in codec ? 'hevc' : 'av1'
  );

  let hevc = $derived(mode === 'hevc' ? (codec as { hevc: any }).hevc : null);
  let av1 = $derived(mode === 'av1' ? (codec as { av1: any }).av1 : null);

  const sliceTypeNames: Record<number, string> = {
    0: 'B',
    1: 'P',
    2: 'I',
  };

  const frameTypeLabels: Record<string, string> = {
    KEY: 'Key Frame',
    INTER: 'Inter Frame',
    INTRA_ONLY: 'Intra Only',
    SWITCH: 'Switch Frame',
    UNKNOWN: 'Unknown',
  };

  function boolLabel(value: boolean): string {
    return value ? 'Yes' : 'No';
  }

  function optionalLabel(value: boolean | number | string | null | undefined): string {
    return value === null || value === undefined ? '—' : String(value);
  }

  function hevcProfileName(generalProfileIdc: number): string {
    switch (generalProfileIdc) {
      case 1: return 'Main';
      case 2: return 'Main 10';
      case 3: return 'Main Still Picture';
      default: return `profile_idc=${generalProfileIdc}`;
    }
  }

  function hevcChromaFormatName(chromaFormatIdc: number): string {
    switch (chromaFormatIdc) {
      case 0: return '4:0:0';
      case 1: return '4:2:0';
      case 2: return '4:2:2';
      case 3: return '4:4:4';
      default: return `chroma_format_idc=${chromaFormatIdc}`;
    }
  }

  function av1LevelName(seqLevelIdx0: number): string {
    const levels = [
      '2.0', '2.1', '2.2', '2.3', '3.0', '3.1', '3.2', '3.3',
      '4.0', '4.1', '4.2', '4.3', '5.0', '5.1', '5.2', '5.3',
      '6.0', '6.1', '6.2', '6.3', '7.0', '7.1', '7.2', '7.3',
    ];
    return levels[seqLevelIdx0] ? `Level ${levels[seqLevelIdx0]}` : `seq_level_idx_0=${seqLevelIdx0}`;
  }

  function av1BitDepth(colorConfig: NonNullable<typeof av1>['sequence_header']['color_config']): string {
    if (!colorConfig.high_bitdepth) return '8-bit';
    return colorConfig.twelve_bit ? '12-bit' : '10-bit';
  }

  function av1ChromaFormat(colorConfig: NonNullable<typeof av1>['sequence_header']['color_config']): string {
    if (colorConfig.mono_chrome) return '4:0:0';
    if (!colorConfig.subsampling_x && !colorConfig.subsampling_y) return '4:4:4';
    if (colorConfig.subsampling_x && !colorConfig.subsampling_y) return '4:2:2';
    return '4:2:0';
  }

  let vpsRows = $derived.by(() => {
    if (!hevc?.vps) return [];
    return [
      ['vps_video_parameter_set_id', hevc.vps.vps_video_parameter_set_id],
      ['vps_base_layer_internal_flag', boolLabel(hevc.vps.vps_base_layer_internal_flag)],
      ['vps_base_layer_available_flag', boolLabel(hevc.vps.vps_base_layer_available_flag)],
      ['vps_max_layers_minus1', hevc.vps.vps_max_layers_minus1],
      ['vps_max_sub_layers_minus1', hevc.vps.vps_max_sub_layers_minus1],
      ['vps_temporal_id_nesting_flag', boolLabel(hevc.vps.vps_temporal_id_nesting_flag)],
    ];
  });

  let spsRows = $derived.by(() => {
    if (!hevc?.sps) return [];
    return [
      ['sps_seq_parameter_set_id', hevc.sps.sps_seq_parameter_set_id],
      ['general_profile_idc', `${hevc.sps.general_profile_idc} (${hevcProfileName(hevc.sps.general_profile_idc)})`],
      ['general_level_idc', `${hevc.sps.general_level_idc}`],
      ['sps_max_sub_layers_minus1', hevc.sps.sps_max_sub_layers_minus1],
      ['sps_temporal_id_nesting_flag', boolLabel(hevc.sps.sps_temporal_id_nesting_flag)],
      ['chroma_format_idc', hevc.sps.chroma_format_idc],
      ['chroma_format_idc (derived)', hevcChromaFormatName(hevc.sps.chroma_format_idc)],
      ['separate_colour_plane_flag', boolLabel(hevc.sps.separate_colour_plane_flag)],
      ['pic_width_in_luma_samples', hevc.sps.pic_width_in_luma_samples],
      ['pic_height_in_luma_samples', hevc.sps.pic_height_in_luma_samples],
      ['conformance_window_flag', boolLabel(hevc.sps.conformance_window_flag)],
      ['conf_win_left_offset', hevc.sps.conf_win_left_offset],
      ['conf_win_right_offset', hevc.sps.conf_win_right_offset],
      ['conf_win_top_offset', hevc.sps.conf_win_top_offset],
      ['conf_win_bottom_offset', hevc.sps.conf_win_bottom_offset],
      ['bit_depth_luma_minus8', hevc.sps.bit_depth_luma_minus8],
      ['bit_depth_chroma_minus8', hevc.sps.bit_depth_chroma_minus8],
      ['bit_depth_luma_minus8 + 8 (derived)', `${hevc.sps.bit_depth_luma_minus8 + 8}-bit`],
      ['bit_depth_chroma_minus8 + 8 (derived)', `${hevc.sps.bit_depth_chroma_minus8 + 8}-bit`],
      ['log2_max_pic_order_cnt_lsb_minus4', hevc.sps.log2_max_pic_order_cnt_lsb_minus4],
      ['log2_min_luma_coding_block_size_minus3', hevc.sps.log2_min_luma_coding_block_size_minus3],
      ['log2_diff_max_min_luma_coding_block_size', hevc.sps.log2_diff_max_min_luma_coding_block_size],
      ['log2_min_transform_block_size_minus2', hevc.sps.log2_min_transform_block_size_minus2],
      ['log2_diff_max_min_transform_block_size', hevc.sps.log2_diff_max_min_transform_block_size],
      ['max_transform_hierarchy_depth_inter', hevc.sps.max_transform_hierarchy_depth_inter],
      ['max_transform_hierarchy_depth_intra', hevc.sps.max_transform_hierarchy_depth_intra],
      ['amp_enabled_flag', boolLabel(hevc.sps.amp_enabled_flag)],
      ['sample_adaptive_offset_enabled_flag', boolLabel(hevc.sps.sample_adaptive_offset_enabled_flag)],
    ];
  });

  let ppsRows = $derived.by(() => {
    if (!hevc?.pps) return [];
    return [
      ['pps_pic_parameter_set_id', hevc.pps.pps_pic_parameter_set_id],
      ['pps_seq_parameter_set_id', hevc.pps.pps_seq_parameter_set_id],
      ['dependent_slice_segments_enabled_flag', boolLabel(hevc.pps.dependent_slice_segments_enabled_flag)],
      ['output_flag_present_flag', boolLabel(hevc.pps.output_flag_present_flag)],
      ['num_extra_slice_header_bits', hevc.pps.num_extra_slice_header_bits],
      ['sign_data_hiding_enabled_flag', boolLabel(hevc.pps.sign_data_hiding_enabled_flag)],
      ['cabac_init_present_flag', boolLabel(hevc.pps.cabac_init_present_flag)],
      ['num_ref_idx_l0_default_active_minus1', hevc.pps.num_ref_idx_l0_default_active_minus1],
      ['num_ref_idx_l1_default_active_minus1', hevc.pps.num_ref_idx_l1_default_active_minus1],
      ['init_qp_minus26', hevc.pps.init_qp_minus26],
      ['constrained_intra_pred_flag', boolLabel(hevc.pps.constrained_intra_pred_flag)],
      ['transform_skip_enabled_flag', boolLabel(hevc.pps.transform_skip_enabled_flag)],
      ['cu_qp_delta_enabled_flag', boolLabel(hevc.pps.cu_qp_delta_enabled_flag)],
      ['diff_cu_qp_delta_depth', hevc.pps.diff_cu_qp_delta_depth],
      ['pps_cb_qp_offset', hevc.pps.pps_cb_qp_offset],
      ['pps_cr_qp_offset', hevc.pps.pps_cr_qp_offset],
      ['pps_slice_chroma_qp_offsets_present_flag', boolLabel(hevc.pps.pps_slice_chroma_qp_offsets_present_flag)],
      ['weighted_pred_flag', boolLabel(hevc.pps.weighted_pred_flag)],
      ['weighted_bipred_flag', boolLabel(hevc.pps.weighted_bipred_flag)],
      ['transquant_bypass_enabled_flag', boolLabel(hevc.pps.transquant_bypass_enabled_flag)],
      ['tiles_enabled_flag', boolLabel(hevc.pps.tiles_enabled_flag)],
      ['entropy_coding_sync_enabled_flag', boolLabel(hevc.pps.entropy_coding_sync_enabled_flag)],
      ['num_tile_columns_minus1', hevc.pps.num_tile_columns_minus1],
      ['num_tile_rows_minus1', hevc.pps.num_tile_rows_minus1],
      ['uniform_spacing_flag', boolLabel(hevc.pps.uniform_spacing_flag)],
      ['loop_filter_across_tiles_enabled_flag', boolLabel(hevc.pps.loop_filter_across_tiles_enabled_flag)],
      ['pps_loop_filter_across_slices_enabled_flag', boolLabel(hevc.pps.pps_loop_filter_across_slices_enabled_flag)],
      ['deblocking_filter_control_present_flag', boolLabel(hevc.pps.deblocking_filter_control_present_flag)],
      ['deblocking_filter_override_enabled_flag', boolLabel(hevc.pps.deblocking_filter_override_enabled_flag)],
      ['pps_deblocking_filter_disabled_flag', boolLabel(hevc.pps.pps_deblocking_filter_disabled_flag)],
      ['pps_beta_offset_div2', hevc.pps.pps_beta_offset_div2],
      ['pps_tc_offset_div2', hevc.pps.pps_tc_offset_div2],
      ['lists_modification_present_flag', boolLabel(hevc.pps.lists_modification_present_flag)],
      ['log2_parallel_merge_level_minus2', hevc.pps.log2_parallel_merge_level_minus2],
      ['slice_segment_header_extension_present_flag', boolLabel(hevc.pps.slice_segment_header_extension_present_flag)],
    ];
  });

  let av1SequenceHeaderRows = $derived.by(() => {
    if (!av1?.sequence_header) return [];
    return [
      ['seq_profile', av1.sequence_header.seq_profile],
      ['reduced_still_picture_header', boolLabel(av1.sequence_header.reduced_still_picture_header)],
      ['seq_level_idx_0', `${av1.sequence_header.seq_level_idx_0} (${av1LevelName(av1.sequence_header.seq_level_idx_0)})`],
      ['max_frame_width_minus1', av1.sequence_header.max_frame_width_minus1],
      ['max_frame_height_minus1', av1.sequence_header.max_frame_height_minus1],
      ['max_frame_width_minus1 + 1 (derived)', av1.sequence_header.max_frame_width_minus1 + 1],
      ['max_frame_height_minus1 + 1 (derived)', av1.sequence_header.max_frame_height_minus1 + 1],
      ['color_config.high_bitdepth', boolLabel(av1.sequence_header.color_config.high_bitdepth)],
      ['color_config.twelve_bit', boolLabel(av1.sequence_header.color_config.twelve_bit)],
      ['color_config.mono_chrome', boolLabel(av1.sequence_header.color_config.mono_chrome)],
      ['color_config.color_description_present_flag', boolLabel(av1.sequence_header.color_config.color_description_present_flag)],
      ['color_config.color_primaries', av1.sequence_header.color_config.color_primaries],
      ['color_config.transfer_characteristics', av1.sequence_header.color_config.transfer_characteristics],
      ['color_config.matrix_coefficients', av1.sequence_header.color_config.matrix_coefficients],
      ['color_config.color_range', boolLabel(av1.sequence_header.color_config.color_range)],
      ['color_config.subsampling_x', boolLabel(av1.sequence_header.color_config.subsampling_x)],
      ['color_config.subsampling_y', boolLabel(av1.sequence_header.color_config.subsampling_y)],
      ['color_config.chroma_sample_position', optionalLabel(av1.sequence_header.color_config.chroma_sample_position)],
      ['color_config.separate_uv_delta_q', boolLabel(av1.sequence_header.color_config.separate_uv_delta_q)],
      ['color_config.bit_depth (derived)', av1BitDepth(av1.sequence_header.color_config)],
      ['color_config.chroma_sampling (derived)', av1ChromaFormat(av1.sequence_header.color_config)],
    ];
  });
</script>

<div class="codec-syntax-tab">
  {#if mode === 'hevc' && hevc}
    <section class="codec-section">
      <h2 class="section-title">HEVC / H.265 Bitstream</h2>

      {#if hevc.nal_units.length > 0}
        <div class="info-card">
          <h3 class="card-title">NAL Units ({hevc.nal_units.length})</h3>
          <table class="info-table mono-table">
            <thead>
              <tr>
                <th>#</th>
                <th>Type</th>
                <th>Layer</th>
                <th>Temporal</th>
                <th>Offset</th>
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              {#each hevc.nal_units as nal, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{nal.nal_unit_type}</td>
                  <td>{nal.nuh_layer_id}</td>
                  <td>{nal.nuh_temporal_id_plus1}</td>
                  <td>{nal.offset}</td>
                  <td>{nal.size} B</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if hevc.vps}
        <div class="info-card">
          <h3 class="card-title">Video Parameter Set (VPS)</h3>
          <table class="info-table">
            <tbody>
              {#each vpsRows as [label, value]}
                <tr><td>{label}</td><td>{value}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if hevc.sps}
        <div class="info-card">
          <h3 class="card-title">Sequence Parameter Set (SPS)</h3>
          <table class="info-table">
            <tbody>
              {#each spsRows as [label, value]}
                <tr><td>{label}</td><td>{value}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if hevc.pps}
        <div class="info-card">
          <h3 class="card-title">Picture Parameter Set (PPS)</h3>
          <table class="info-table">
            <tbody>
              {#each ppsRows as [label, value]}
                <tr><td>{label}</td><td>{value}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if hevc.slice_headers.length > 0}
        <div class="info-card">
          <h3 class="card-title">Slice Headers ({hevc.slice_headers.length})</h3>
          <table class="info-table mono-table">
            <thead>
              <tr>
                <th>#</th>
                <th>NAL</th>
                <th>Type</th>
                <th>First Slice</th>
                <th>Dependent</th>
                <th>No Output</th>
                <th>Address</th>
                <th>PPS ID</th>
                <th>QP Δ</th>
                <th>Entry Points</th>
                <th>Tiles</th>
              </tr>
            </thead>
            <tbody>
              {#each hevc.slice_headers as sh, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{sh.nal_unit_type}</td>
                  <td>{sliceTypeNames[sh.slice_type] ?? sh.slice_type}</td>
                  <td>{sh.first_slice_segment_in_pic_flag ? 'Yes' : 'No'}</td>
                  <td>{sh.dependent_slice_segment_flag ? 'Yes' : 'No'}</td>
                  <td>{sh.no_output_of_prior_pics_flag ? 'Yes' : 'No'}</td>
                  <td>{sh.slice_segment_address}</td>
                  <td>{sh.slice_pic_parameter_set_id}</td>
                  <td>{optionalLabel(sh.slice_qp_delta)}</td>
                  <td>{optionalLabel(sh.num_entry_point_offsets)}</td>
                  <td>{sh.tiles_enabled_flag ? 'Yes' : 'No'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <div class="info-card">
          <h3 class="card-title">Slice Header Details</h3>
          <div class="slice-detail-list">
            {#each hevc.slice_headers as sh, i}
              <section class="slice-detail">
                <h4>Slice {i + 1} · {sh.nal_unit_type}</h4>
                <table class="info-table">
                  <tbody>
                    <tr><td>slice_type</td><td>{sliceTypeNames[sh.slice_type] ?? sh.slice_type}</td></tr>
                    <tr><td>first_slice_segment_in_pic_flag</td><td>{boolLabel(sh.first_slice_segment_in_pic_flag)}</td></tr>
                    <tr><td>dependent_slice_segment_flag</td><td>{boolLabel(sh.dependent_slice_segment_flag)}</td></tr>
                    <tr><td>no_output_of_prior_pics_flag</td><td>{boolLabel(sh.no_output_of_prior_pics_flag)}</td></tr>
                    <tr><td>slice_segment_address</td><td>{sh.slice_segment_address}</td></tr>
                    <tr><td>slice_pic_parameter_set_id</td><td>{sh.slice_pic_parameter_set_id}</td></tr>
                    <tr><td>pic_output_flag</td><td>{optionalLabel(sh.pic_output_flag)}</td></tr>
                    <tr><td>colour_plane_id</td><td>{optionalLabel(sh.colour_plane_id)}</td></tr>
                    <tr><td>short_term_ref_pic_set_sps_flag</td><td>{optionalLabel(sh.short_term_ref_pic_set_sps_flag)}</td></tr>
                    <tr><td>slice_sao_luma_flag</td><td>{optionalLabel(sh.slice_sao_luma_flag)}</td></tr>
                    <tr><td>slice_sao_chroma_flag</td><td>{optionalLabel(sh.slice_sao_chroma_flag)}</td></tr>
                    <tr><td>num_ref_idx_active_override_flag</td><td>{optionalLabel(sh.num_ref_idx_active_override_flag)}</td></tr>
                    <tr><td>num_ref_idx_l0_active_minus1</td><td>{optionalLabel(sh.num_ref_idx_l0_active_minus1)}</td></tr>
                    <tr><td>num_ref_idx_l1_active_minus1</td><td>{optionalLabel(sh.num_ref_idx_l1_active_minus1)}</td></tr>
                    <tr><td>mvd_l1_zero_flag</td><td>{optionalLabel(sh.mvd_l1_zero_flag)}</td></tr>
                    <tr><td>cabac_init_flag</td><td>{optionalLabel(sh.cabac_init_flag)}</td></tr>
                    <tr><td>collocated_from_l0_flag</td><td>{optionalLabel(sh.collocated_from_l0_flag)}</td></tr>
                    <tr><td>collocated_ref_idx</td><td>{optionalLabel(sh.collocated_ref_idx)}</td></tr>
                    <tr><td>num_entry_point_offsets</td><td>{optionalLabel(sh.num_entry_point_offsets)}</td></tr>
                    <tr><td>offset_len_minus1</td><td>{optionalLabel(sh.offset_len_minus1)}</td></tr>
                    <tr><td>five_minus_max_num_merge_cand</td><td>{optionalLabel(sh.five_minus_max_num_merge_cand)}</td></tr>
                    <tr><td>slice_qp_delta</td><td>{optionalLabel(sh.slice_qp_delta)}</td></tr>
                    <tr><td>slice_cb_qp_offset</td><td>{optionalLabel(sh.slice_cb_qp_offset)}</td></tr>
                    <tr><td>slice_cr_qp_offset</td><td>{optionalLabel(sh.slice_cr_qp_offset)}</td></tr>
                    <tr><td>cu_chroma_qp_offset_enabled_flag</td><td>{optionalLabel(sh.cu_chroma_qp_offset_enabled_flag)}</td></tr>
                    <tr><td>deblocking_filter_override_flag</td><td>{optionalLabel(sh.deblocking_filter_override_flag)}</td></tr>
                    <tr><td>slice_deblocking_filter_disabled_flag</td><td>{optionalLabel(sh.slice_deblocking_filter_disabled_flag)}</td></tr>
                    <tr><td>beta_offset_div2</td><td>{optionalLabel(sh.beta_offset_div2)}</td></tr>
                    <tr><td>tc_offset_div2</td><td>{optionalLabel(sh.tc_offset_div2)}</td></tr>
                    <tr><td>slice_loop_filter_across_slices_enabled_flag</td><td>{optionalLabel(sh.slice_loop_filter_across_slices_enabled_flag)}</td></tr>
                    <tr><td>tiles_enabled_flag</td><td>{boolLabel(sh.tiles_enabled_flag)}</td></tr>
                    <tr><td>entropy_coding_sync_enabled_flag</td><td>{boolLabel(sh.entropy_coding_sync_enabled_flag)}</td></tr>
                    <tr><td>pic_width_in_luma_samples / pic_height_in_luma_samples</td><td>{sh.pic_width_in_luma_samples}×{sh.pic_height_in_luma_samples}</td></tr>
                  </tbody>
                </table>
              </section>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  {:else if mode === 'av1' && av1}
    <section class="codec-section">
      <h2 class="section-title">AV1 Bitstream</h2>

      {#if av1.obus.length > 0}
        <div class="info-card">
          <h3 class="card-title">OBUs ({av1.obus.length})</h3>
          <table class="info-table mono-table">
            <thead>
              <tr>
                <th>#</th>
                <th>obu_type</th>
                <th>temporal_id</th>
                <th>spatial_id</th>
                <th>obu_size</th>
              </tr>
            </thead>
            <tbody>
              {#each av1.obus as obu, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{obu.obu_type}</td>
                  <td>{obu.temporal_id}</td>
                  <td>{obu.spatial_id}</td>
                  <td>{obu.obu_size} B</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if av1.sequence_header}
        <div class="info-card">
          <h3 class="card-title">sequence_header_obu()</h3>
          <table class="info-table">
            <tbody>
              {#each av1SequenceHeaderRows as [label, value]}
                <tr><td>{label}</td><td>{value}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if av1.frame_headers.length > 0}
        <div class="info-card">
          <h3 class="card-title">frame_header_obu() ({av1.frame_headers.length})</h3>
          <table class="info-table mono-table">
            <thead>
              <tr>
                <th>#</th>
                <th>show_existing_frame</th>
                <th>frame_type</th>
                <th>show_frame</th>
                <th>frame_size_override_flag</th>
                <th>frame_size</th>
                <th>order_hint</th>
                <th>primary_ref_frame</th>
                <th>base_q_idx</th>
              </tr>
            </thead>
            <tbody>
              {#each av1.frame_headers as fh, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{fh.show_existing_frame ? 'Yes' : 'No'}</td>
                  <td>{frameTypeLabels[fh.frame_type] ?? fh.frame_type}</td>
                  <td>{fh.show_frame ? 'Yes' : 'No'}</td>
                  <td>{optionalLabel(fh.frame_size_override_flag)}</td>
                  <td>{fh.frame_size ? `${fh.frame_size[0]}×${fh.frame_size[1]}` : '—'}</td>
                  <td>{fh.order_hint}</td>
                  <td>{fh.primary_ref_frame}</td>
                  <td>{fh.quantizer_params?.base_q_idx ?? '—'}</td>
                </tr>
              {/each}
            </tbody>
          </table>

          <div class="detail-list">
            {#each av1.frame_headers as fh, i}
              <section class="detail-card">
                <h4>frame_header_obu #{i + 1}</h4>
                <table class="info-table">
                  <tbody>
                    <tr><td>show_existing_frame</td><td>{boolLabel(fh.show_existing_frame)}</td></tr>
                    <tr><td>frame_to_show_map_idx</td><td>{optionalLabel(fh.frame_to_show_map_idx)}</td></tr>
                    <tr><td>frame_type</td><td>{fh.frame_type}</td></tr>
                    <tr><td>show_frame</td><td>{boolLabel(fh.show_frame)}</td></tr>
                    <tr><td>error_resilient_mode</td><td>{optionalLabel(fh.error_resilient_mode)}</td></tr>
                    <tr><td>disable_cdf_update</td><td>{optionalLabel(fh.disable_cdf_update)}</td></tr>
                    <tr><td>allow_screen_content_tools</td><td>{optionalLabel(fh.allow_screen_content_tools)}</td></tr>
                    <tr><td>force_integer_mv</td><td>{optionalLabel(fh.force_integer_mv)}</td></tr>
                    <tr><td>frame_size_override_flag</td><td>{optionalLabel(fh.frame_size_override_flag)}</td></tr>
                    <tr><td>render_and_frame_size_different</td><td>{optionalLabel(fh.render_and_frame_size_different)}</td></tr>
                    <tr><td>allow_intrabc</td><td>{optionalLabel(fh.allow_intrabc)}</td></tr>
                    <tr><td>refresh_frame_flags</td><td>{optionalLabel(fh.refresh_frame_flags)}</td></tr>
                    <tr><td>frame_size</td><td>{fh.frame_size ? `${fh.frame_size[0]}×${fh.frame_size[1]}` : '—'}</td></tr>
                    <tr><td>order_hint</td><td>{fh.order_hint}</td></tr>
                    <tr><td>primary_ref_frame</td><td>{fh.primary_ref_frame}</td></tr>
                    <tr><td>base_q_idx</td><td>{optionalLabel(fh.quantizer_params?.base_q_idx)}</td></tr>
                    <tr><td>delta_q_present</td><td>{optionalLabel(fh.quantizer_params?.delta_q_present)}</td></tr>
                    <tr><td>delta_q_res</td><td>{optionalLabel(fh.quantizer_params?.delta_q_res)}</td></tr>
                    <tr><td>delta_q_y_dc.delta_coded</td><td>{optionalLabel(fh.delta_q_y_dc_coded)}</td></tr>
                    <tr><td>delta_q_u_dc.delta_coded</td><td>{optionalLabel(fh.delta_q_u_dc_coded)}</td></tr>
                    <tr><td>delta_q_u_ac.delta_coded</td><td>{optionalLabel(fh.delta_q_u_ac_coded)}</td></tr>
                    <tr><td>using_qmatrix</td><td>{optionalLabel(fh.using_qmatrix)}</td></tr>
                    <tr><td>segmentation_enabled</td><td>{optionalLabel(fh.segmentation_enabled)}</td></tr>
                    <tr><td>reduced_tx_set</td><td>{optionalLabel(fh.reduced_tx_set)}</td></tr>
                    <tr><td>use_128x128_superblock</td><td>{boolLabel(fh.use_128x128_superblock)}</td></tr>
                    <tr><td>tile_cols_log2</td><td>{fh.tile_cols_log2}</td></tr>
                    <tr><td>tile_rows_log2</td><td>{fh.tile_rows_log2}</td></tr>
                    <tr><td>uniform_tile_spacing_flag</td><td>{boolLabel(fh.uniform_tile_spacing_flag)}</td></tr>
                  </tbody>
                </table>
              </section>
            {/each}
          </div>
        </div>
      {/if}

      {#if av1.tile_info}
        <div class="info-card">
          <h3 class="card-title">tile_info()</h3>
          <table class="info-table">
            <tbody>
              <tr><td>TileCols * TileRows</td><td>{av1.tile_info.num_tiles}</td></tr>
              <tr><td>TileRows</td><td>{av1.tile_info.rows}</td></tr>
              <tr><td>TileCols</td><td>{av1.tile_info.cols}</td></tr>
              {#if av1.tile_info.tile_width.length > 0}
                <tr><td>tile_width</td><td>{av1.tile_info.tile_width.join(', ')}</td></tr>
              {/if}
              {#if av1.tile_info.tile_height.length > 0}
                <tr><td>tile_height</td><td>{av1.tile_info.tile_height.join(', ')}</td></tr>
              {/if}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {:else}
    <p class="no-data">No codec syntax data available.</p>
  {/if}
</div>

<style>
  .codec-syntax-tab {
    padding: 0.25rem;
  }

  .codec-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--accent-bright);
    margin: 0;
    padding-bottom: 0.625rem;
    border-bottom: 1px solid var(--border-default);
  }

  .info-card {
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-secondary);
    transition: border-color var(--duration-fast) var(--ease-out-expo);
  }

  .slice-detail-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.875rem;
  }

  .slice-detail {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .slice-detail h4 {
    margin: 0;
    padding: 0.625rem 0.875rem;
    font-size: 0.75rem;
    color: var(--text-primary);
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .info-card:hover {
    border-color: var(--border-strong);
  }

  .card-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
    padding: 0.5rem 0.875rem;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-subtle);
    letter-spacing: 0.01em;
  }

  .info-table {
    width: 100%;
    font-size: 0.75rem;
    border-collapse: collapse;
  }

  .info-table thead {
    background: var(--bg-tertiary);
  }

  .info-table th {
    text-align: left;
    font-weight: 500;
    color: var(--text-tertiary);
    padding: 0.375rem 0.625rem;
    border-bottom: 1px solid var(--border-default);
    text-transform: uppercase;
    font-size: 0.6875rem;
    letter-spacing: 0.04em;
  }

  .info-table td {
    padding: 0.3rem 0.625rem;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
  }

  .info-table tbody tr:last-child td {
    border-bottom: none;
  }

  .info-table tbody tr:hover {
    background: var(--bg-hover);
  }

  .mono-table {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.7rem;
  }

  .no-data {
    color: var(--text-secondary);
    text-align: center;
    padding: 3rem 2rem;
    font-size: 0.875rem;
  }
</style>
