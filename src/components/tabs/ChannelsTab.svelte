<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';
  import Histogram from '../ui/Histogram.svelte';

  let { data }: { data: ImageAnalysis } = $props();
  let mode = $state<'rgb' | 'yuv'>('rgb');
</script>

<div class="channels-tab">
  <div class="mode-switcher">
    <button class:active={mode === 'rgb'} onclick={() => mode = 'rgb'}>RGB</button>
    <button class:active={mode === 'yuv'} onclick={() => mode = 'yuv'}>YUV</button>
    {#if data.channels?.ycbcr_subsampling}
      <span class="subsampling">{data.channels.ycbcr_subsampling}</span>
    {/if}
  </div>

  <div class="color-matrix">Matrix: {data.channels?.color_matrix || 'N/A'}</div>

  {#if mode === 'rgb' && data.channels?.rgb}
    <div class="channels">
      <div class="channel-card" style="--ch-color: #ef4444">
        <h4>R</h4>
        <p>min: {data.channels.rgb.r.min} max: {data.channels.rgb.r.max} mean: {data.channels.rgb.r.mean.toFixed(1)} median: {data.channels.rgb.r.median} std: {data.channels.rgb.r.std_dev.toFixed(1)}</p>
        <Histogram bins={data.channels.histograms[0]?.bins || []} color="#ef4444" label="Red" />
      </div>
      <div class="channel-card" style="--ch-color: #22c55e">
        <h4>G</h4>
        <p>min: {data.channels.rgb.g.min} max: {data.channels.rgb.g.max} mean: {data.channels.rgb.g.mean.toFixed(1)} median: {data.channels.rgb.g.median} std: {data.channels.rgb.g.std_dev.toFixed(1)}</p>
        <Histogram bins={data.channels.histograms[1]?.bins || []} color="#22c55e" label="Green" />
      </div>
      <div class="channel-card" style="--ch-color: #3b82f6">
        <h4>B</h4>
        <p>min: {data.channels.rgb.b.min} max: {data.channels.rgb.b.max} mean: {data.channels.rgb.b.mean.toFixed(1)} median: {data.channels.rgb.b.median} std: {data.channels.rgb.b.std_dev.toFixed(1)}</p>
        <Histogram bins={data.channels.histograms[2]?.bins || []} color="#3b82f6" label="Blue" />
      </div>
      {#if data.channels.rgb.a}
        <div class="channel-card" style="--ch-color: #888">
          <h4>A</h4>
          <p>min: {data.channels.rgb.a.min} max: {data.channels.rgb.a.max} mean: {data.channels.rgb.a.mean.toFixed(1)} median: {data.channels.rgb.a.median} std: {data.channels.rgb.a.std_dev.toFixed(1)}</p>
        </div>
      {/if}
    </div>
  {/if}

  {#if mode === 'yuv' && data.channels?.yuv}
    <div class="channels">
      <div class="channel-card" style="--ch-color: #fff">
        <h4>Y (Luma)</h4>
        <p>min: {data.channels.yuv.y.min} max: {data.channels.yuv.y.max} mean: {data.channels.yuv.y.mean.toFixed(1)} median: {data.channels.yuv.y.median} std: {data.channels.yuv.y.std_dev.toFixed(1)}</p>
      </div>
      <div class="channel-card" style="--ch-color: #3b82f6">
        <h4>Cb (Chroma Blue)</h4>
        <p>min: {data.channels.yuv.cb.min} max: {data.channels.yuv.cb.max} mean: {data.channels.yuv.cb.mean.toFixed(1)} median: {data.channels.yuv.cb.median} std: {data.channels.yuv.cb.std_dev.toFixed(1)}</p>
      </div>
      <div class="channel-card" style="--ch-color: #ef4444">
        <h4>Cr (Chroma Red)</h4>
        <p>min: {data.channels.yuv.cr.min} max: {data.channels.yuv.cr.max} mean: {data.channels.yuv.cr.mean.toFixed(1)} median: {data.channels.yuv.cr.median} std: {data.channels.yuv.cr.std_dev.toFixed(1)}</p>
      </div>
    </div>
  {/if}

  {#if !data.channels}
    <p class="placeholder">Channel data not available for this format</p>
  {/if}
</div>

<style>
  .channels-tab {
    font-size: 0.875rem;
  }
  .mode-switcher {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
  .mode-switcher button {
    padding: 0.4rem 1rem;
    background: rgba(255,255,255,0.05);
    border: 1px solid var(--color-border, #334155);
    color: var(--color-muted, #64748b);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    transition: background 0.15s, color 0.15s;
  }
  .mode-switcher button:hover {
    background: rgba(255,255,255,0.1);
  }
  .mode-switcher button.active {
    background: var(--color-accent, #6366f1);
    color: white;
    border-color: var(--color-accent, #6366f1);
  }
  .subsampling {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
  }
  .color-matrix {
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
    margin-bottom: 1rem;
  }
  .channels {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .channel-card {
    padding: 0.75rem;
    border: 1px solid var(--color-border, #1e293b);
    border-radius: 6px;
    border-left: 3px solid var(--ch-color);
  }
  .channel-card h4 {
    margin: 0 0 0.25rem;
    color: var(--ch-color);
    font-size: 0.85rem;
  }
  .channel-card p {
    margin: 0;
    font-size: 0.75rem;
    color: var(--color-muted, #64748b);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
  }
  .placeholder {
    text-align: center;
    color: var(--color-muted, #64748b);
    padding: 2rem;
  }
</style>
