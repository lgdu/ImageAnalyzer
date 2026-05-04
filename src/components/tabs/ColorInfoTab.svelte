<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { IccInfo } from '../../lib/types';

  let { filePath }: { filePath: string } = $props();
  let iccProfile: IccInfo | null = $state(null);
  let loading = $state(true);

  // Load ICC on mount
  $effect(() => {
    if (!filePath) return;
    loading = true;
    iccProfile = null;
    invoke<IccInfo | null>('get_icc_profile', { filePath })
      .then((result) => {
        iccProfile = result;
        loading = false;
      })
      .catch(() => {
        loading = false;
      });
  });
</script>

<div class="color-info">
  {#if loading}
    <p class="no-icc">Loading ICC profile...</p>
  {:else if iccProfile}
    <section class="section">
      <h3>ICC Profile Header</h3>
      <table class="data-table">
        <tbody>
        <tr><td>CMM Type</td><td>{iccProfile.cmm_type}</td></tr>
        <tr><td>Version</td><td>{iccProfile.version}</td></tr>
        <tr><td>Profile Class</td><td>{iccProfile.profile_class}</td></tr>
        <tr><td>Color Space</td><td>{iccProfile.color_space}</td></tr>
        <tr><td>PCS</td><td>{iccProfile.pcs}</td></tr>
        {#if iccProfile.platform}
          <tr><td>Platform</td><td>{iccProfile.platform}</td></tr>
        {/if}
        <tr><td>Rendering Intent</td><td>{iccProfile.rendering_intent}</td></tr>
        <tr><td>Illuminant</td><td>X={iccProfile.illuminant[0].toFixed(4)} Y={iccProfile.illuminant[1].toFixed(4)} Z={iccProfile.illuminant[2].toFixed(4)}</td></tr>
        {#if iccProfile.description}
          <tr><td>Description</td><td>{iccProfile.description}</td></tr>
        {/if}
        {#if iccProfile.creator}
          <tr><td>Creator</td><td>{iccProfile.creator}</td></tr>
        {/if}
        </tbody>
      </table>
    </section>

    <section class="section">
      <h3>Transfer Curves (TRC)</h3>
      <table class="data-table">
        <tbody>
        {#if iccProfile.transfer_function}
          <tr><td>Global TRC</td><td>{iccProfile.transfer_function}</td></tr>
        {/if}
        {#if iccProfile.red_trc}
          <tr><td>Red TRC</td><td>{iccProfile.red_trc}</td></tr>
        {/if}
        {#if iccProfile.green_trc}
          <tr><td>Green TRC</td><td>{iccProfile.green_trc}</td></tr>
        {/if}
        {#if iccProfile.blue_trc}
          <tr><td>Blue TRC</td><td>{iccProfile.blue_trc}</td></tr>
        {/if}
        {#if iccProfile.luts.length > 0}
          {#each iccProfile.luts as lut}
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
      <h3>ICC Tags ({iccProfile.tag_count})</h3>
      <table class="data-table">
        <tbody>
        {#each iccProfile.tags as tag}
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
    padding: 1rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }
  .section h3 {
    margin: 0 0 0.75rem;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-weight: 600;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-subtle);
  }
  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .data-table td {
    padding: 0.375rem 0.625rem;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: top;
  }
  .data-table tbody tr:last-child td {
    border-bottom: none;
  }
  .data-table td:first-child {
    color: var(--text-secondary);
    width: 140px;
    white-space: nowrap;
  }
  .data-table td:last-child {
    color: var(--text-primary);
  }
  code {
    color: var(--accent-bright);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.75rem;
    background: var(--bg-elevated);
    padding: 0.125rem 0.25rem;
    border-radius: 3px;
  }
  .no-icc {
    color: var(--text-secondary);
    text-align: center;
    padding: 3rem 2rem;
    font-size: 0.875rem;
  }
</style>
