<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';

  let { data }: { data: ImageAnalysis } = $props();
</script>

<div class="color-info">
  {#if data.icc_profile}
    <section class="section">
      <h3>ICC Profile Header</h3>
      <table class="data-table">
        <tbody>
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
        {#if data.icc_profile.creator}
          <tr><td>Creator</td><td>{data.icc_profile.creator}</td></tr>
        {/if}
        </tbody>
      </table>
    </section>

    <section class="section">
      <h3>Transfer Curves (TRC)</h3>
      <table class="data-table">
        <tbody>
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
        {#if data.icc_profile.luts.length > 0}
          {#each data.icc_profile.luts as lut}
            <tr>
              <td>LUT: {lut.name}</td>
              <td>{lut.input_channels}→{lut.output_channels} channels{#if lut.clut_points !== null}, {lut.clut_points} CLUT points{/if}</td>
            </tr>
          {/each}
        {/if}
        </tbody>
      </table>
    </section>

    <section class="section">
      <h3>ICC Tags ({data.icc_profile.tag_count})</h3>
      <table class="data-table">
        <tbody>
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
        </tbody>
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
