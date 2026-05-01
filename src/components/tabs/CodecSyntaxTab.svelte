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
</script>

<div class="codec-syntax-tab">
  {#if mode === 'hevc' && hevc}
    <section class="codec-section">
      <h2 class="section-title">HEVC / H.265 Bitstream</h2>

      {#if hevc.vps}
        <div class="info-card">
          <h3 class="card-title">Video Parameter Set (VPS)</h3>
          <table class="info-table">
            <tbody>
              <tr><td>VPS ID</td><td>{hevc.vps.vps_id}</td></tr>
              <tr><td>Max Layers</td><td>{hevc.vps.max_layers}</td></tr>
              <tr><td>Max Sub-layers</td><td>{hevc.vps.max_sub_layers}</td></tr>
            </tbody>
          </table>
        </div>
      {/if}

      {#if hevc.sps}
        <div class="info-card">
          <h3 class="card-title">Sequence Parameter Set (SPS)</h3>
          <table class="info-table">
            <tbody>
              <tr><td>Profile</td><td>{hevc.sps.profile}</td></tr>
              <tr><td>Level</td><td>{hevc.sps.level}</td></tr>
              <tr><td>Chroma Format</td><td>{hevc.sps.chroma_format}</td></tr>
              <tr><td>Resolution</td><td>{hevc.sps.pic_width}×{hevc.sps.pic_height}</td></tr>
              <tr><td>Bit Depth</td><td>{hevc.sps.bit_depth}-bit</td></tr>
            </tbody>
          </table>
        </div>
      {/if}

      {#if hevc.pps}
        <div class="info-card">
          <h3 class="card-title">Picture Parameter Set (PPS)</h3>
          <table class="info-table">
            <tbody>
              <tr><td>PPS ID</td><td>{hevc.pps.pps_id}</td></tr>
              <tr><td>SPS ID</td><td>{hevc.pps.sps_id}</td></tr>
            </tbody>
          </table>
        </div>
      {/if}

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
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              {#each hevc.nal_units as nal, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{nal.nal_type}</td>
                  <td>{nal.nuh_layer_id}</td>
                  <td>{nal.nuh_temporal_id}</td>
                  <td>{nal.size} B</td>
                </tr>
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
                <th>Type</th>
                <th>First Slice</th>
                <th>Dependent</th>
                <th>Address</th>
                <th>PPS ID</th>
              </tr>
            </thead>
            <tbody>
              {#each hevc.slice_headers as sh, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{sliceTypeNames[sh.slice_type] ?? sh.slice_type}</td>
                  <td>{sh.first_slice_segment_in_pic_flag ? 'Yes' : 'No'}</td>
                  <td>{sh.dependent_slice_segment_flag ? 'Yes' : 'No'}</td>
                  <td>{sh.slice_segment_address}</td>
                  <td>{sh.pps_id}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {:else if mode === 'av1' && av1}
    <section class="codec-section">
      <h2 class="section-title">AV1 Bitstream</h2>

      {#if av1.sequence_header}
        <div class="info-card">
          <h3 class="card-title">Sequence Header</h3>
          <table class="info-table">
            <tbody>
              <tr><td>Profile</td><td>{av1.sequence_header.profile}</td></tr>
              <tr><td>Level</td><td>{av1.sequence_header.level}</td></tr>
              <tr><td>Bit Depth</td><td>{av1.sequence_header.bit_depth}-bit</td></tr>
              <tr><td>Chroma Format</td><td>{av1.sequence_header.chroma_format}</td></tr>
              <tr><td>Frame Size</td><td>{av1.sequence_header.frame_width}×{av1.sequence_header.frame_height}</td></tr>
            </tbody>
          </table>
        </div>
      {/if}

      {#if av1.obus.length > 0}
        <div class="info-card">
          <h3 class="card-title">OBUs ({av1.obus.length})</h3>
          <table class="info-table mono-table">
            <thead>
              <tr>
                <th>#</th>
                <th>Type</th>
                <th>Temporal</th>
                <th>Spatial</th>
                <th>Payload Size</th>
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

      {#if av1.frame_headers.length > 0}
        <div class="info-card">
          <h3 class="card-title">Frame Headers ({av1.frame_headers.length})</h3>
          <table class="info-table mono-table">
            <thead>
              <tr>
                <th>#</th>
                <th>Type</th>
                <th>Show Frame</th>
                <th>Size</th>
                <th>Order Hint</th>
                <th>Primary Ref</th>
                <th>Base QIdx</th>
              </tr>
            </thead>
            <tbody>
              {#each av1.frame_headers as fh, i}
                <tr>
                  <td>{i + 1}</td>
                  <td>{frameTypeLabels[fh.frame_type] ?? fh.frame_type}</td>
                  <td>{fh.show_frame ? 'Yes' : 'No'}</td>
                  <td>{fh.frame_size ? `${fh.frame_size[0]}×${fh.frame_size[1]}` : '—'}</td>
                  <td>{fh.order_hint}</td>
                  <td>{fh.primary_ref_frame}</td>
                  <td>{fh.quantizer_params?.base_q_idx ?? '—'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if av1.tile_info}
        <div class="info-card">
          <h3 class="card-title">Tile Info</h3>
          <table class="info-table">
            <tbody>
              <tr><td>Total Tiles</td><td>{av1.tile_info.num_tiles}</td></tr>
              <tr><td>Tile Rows</td><td>{av1.tile_info.rows}</td></tr>
              <tr><td>Tile Cols</td><td>{av1.tile_info.cols}</td></tr>
              {#if av1.tile_info.tile_width.length > 0}
                <tr><td>Tile Widths</td><td>{av1.tile_info.tile_width.join(', ')}</td></tr>
              {/if}
              {#if av1.tile_info.tile_height.length > 0}
                <tr><td>Tile Heights</td><td>{av1.tile_info.tile_height.join(', ')}</td></tr>
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
    padding: 0.5rem;
  }

  .codec-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--color-accent, #818cf8);
    margin: 0;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--color-border, #334155);
  }

  .info-card {
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    overflow: hidden;
  }

  .card-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-muted, #64748b);
    margin: 0;
    padding: 0.5rem 0.75rem;
    background: rgba(255, 255, 255, 0.02);
    border-bottom: 1px solid var(--color-border, #334155);
  }

  .info-table {
    width: 100%;
    font-size: 0.75rem;
    border-collapse: collapse;
  }

  .info-table thead {
    background: rgba(255, 255, 255, 0.03);
  }

  .info-table th {
    text-align: left;
    font-weight: 500;
    color: var(--color-muted, #64748b);
    padding: 0.375rem 0.5rem;
    border-bottom: 1px solid var(--color-border, #334155);
  }

  .info-table td {
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .info-table tbody tr:last-child td {
    border-bottom: none;
  }

  .mono-table {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.7rem;
  }

  .no-data {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
    font-size: 0.8rem;
  }
</style>
