<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { ChannelData, GifFrame, ImageFormat, SingleChannel } from '../../lib/types';

  let { filePath, format }: { filePath: string; format: ImageFormat } = $props();
  let channels: ChannelData | null = $state(null);
  let loading = $state(true);

  // GIF frame support
  let isGif = $derived(format === 'gif');
  let gifFrames: GifFrame[] = $state([]);
  let framesLoading = $state(false);
  let selectedFrame: number = $state(0);
  let playing = $state(false);
  let playTimer: ReturnType<typeof setTimeout> | null = null;

  // Frame number input binding
  let frameInput = $state('1');

  // Update frameInput when selectedFrame changes (but not during typing)
  $effect(() => {
    if (!playing) {
      frameInput = String(selectedFrame + 1);
    }
  });

  let mode: 'rgb' | 'yuv' = $state('rgb');
  let rgbSelected: 'all' | 'r' | 'g' | 'b' | 'a' = $state('all');
  let yuvSelected: 'all' | 'y' | 'cb' | 'cr' = $state('all');

  let canvasRef: HTMLCanvasElement | null = $state(null);
  let alphaCanvasRef: HTMLCanvasElement | null = $state(null);
  let originalImageData: ImageData | null = $state(null);

  // For GIF: use selected frame's base64; for others: use channels data
  let imageSrc: string = $derived.by(() => {
    if (isGif && gifFrames.length > 0) {
      const frame = gifFrames[selectedFrame];
      return frame ? `data:image/png;base64,${frame.image_base64}` : '';
    }
    return channels?.image_base64 ? `data:image/png;base64,${channels.image_base64}` : '';
  });

  let currentChannel = $derived(mode === 'rgb' ? rgbSelected : yuvSelected);

  let activeStats: SingleChannel | null | undefined = $derived.by(() => {
    if (mode === 'rgb') {
      if (rgbSelected === 'r') return channels?.rgb?.r;
      if (rgbSelected === 'g') return channels?.rgb?.g;
      if (rgbSelected === 'b') return channels?.rgb?.b;
      if (rgbSelected === 'a') return channels?.rgb?.a;
      return null;
    }
    if (yuvSelected === 'y') return channels?.yuv?.y;
    if (yuvSelected === 'cb') return channels?.yuv?.cb;
    if (yuvSelected === 'cr') return channels?.yuv?.cr;
    return null;
  });

  // Load channels on mount
  $effect(() => {
    if (!filePath) return;
    loading = true;
    channels = null;
    invoke<ChannelData | null>('get_channels', { filePath })
      .then((result) => {
        channels = result;
        loading = false;
      })
      .catch(() => {
        loading = false;
      });
  });

  // Load GIF frames on mount for GIF files
  $effect(() => {
    if (!isGif || !filePath) return;
    stopPlay();
    framesLoading = true;
    gifFrames = [];
    invoke<GifFrame[]>('get_gif_frames', { filePath })
      .then((frames) => {
        gifFrames = frames;
        selectedFrame = 0;
        frameInput = '1';
        framesLoading = false;
      })
      .catch(() => {
        framesLoading = false;
      });

    return () => {
      stopPlay();
    };
  });

  // Load image once when source changes
  $effect(() => {
    if (!imageSrc || !canvasRef) return;

    const img = new Image();
    img.onload = () => {
      const canvas = canvasRef!;
      const ctx = canvas.getContext('2d')!;
      canvas.width = img.width;
      canvas.height = img.height;
      ctx.drawImage(img, 0, 0);
      originalImageData = ctx.getImageData(0, 0, img.width, img.height);
      applyChannelFilter();
      drawAlphaChannel();
    };
    img.src = imageSrc;
  });

  function drawAlphaChannel() {
    if (!alphaCanvasRef || !originalImageData) return;
    const ctx = alphaCanvasRef.getContext('2d')!;
    const w = originalImageData.width;
    const h = originalImageData.height;
    alphaCanvasRef.width = w;
    alphaCanvasRef.height = h;

    const size = 8;
    for (let y = 0; y < h; y += size) {
      for (let x = 0; x < w; x += size) {
        ctx.fillStyle = ((x / size + y / size) % 2 === 0) ? '#ccc' : '#fff';
        ctx.fillRect(x, y, size, size);
      }
    }

    const alphaData = new Uint8ClampedArray(originalImageData.data);
    for (let i = 0; i < alphaData.length; i += 4) {
      const a = alphaData[i + 3];
      alphaData[i] = alphaData[i + 1] = alphaData[i + 2] = a;
      alphaData[i + 3] = 255;
    }
    ctx.putImageData(new ImageData(alphaData, w, h), 0, 0);
  }

  $effect(() => {
    if (alphaCanvasRef && originalImageData && channels?.rgb?.a && mode === 'rgb') {
      drawAlphaChannel();
    }
  });

  function applyChannelFilter() {
    if (!canvasRef || !originalImageData) return;
    const ctx = canvasRef.getContext('2d')!;
    const w = originalImageData.width;
    const h = originalImageData.height;
    const data = new Uint8ClampedArray(originalImageData.data);

    const ch = mode === 'rgb' ? rgbSelected : yuvSelected;
    if (ch === 'all') {
      ctx.putImageData(originalImageData, 0, 0);
      return;
    }

    if (mode === 'rgb') {
      for (let i = 0; i < data.length; i += 4) {
        if (ch === 'r') {
          data[i + 1] = data[i + 2] = data[i];
        } else if (ch === 'g') {
          data[i] = data[i + 2] = data[i + 1];
        } else if (ch === 'b') {
          data[i] = data[i + 1] = data[i + 2];
        } else if (ch === 'a') {
          data[i] = data[i + 1] = data[i + 2] = data[i + 3];
        }
      }
    } else {
      for (let i = 0; i < data.length; i += 4) {
        const r = data[i];
        const g = data[i + 1];
        const b = data[i + 2];
        const y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        const cb = 128 - 0.1146 * r - 0.3854 * g + 0.5 * b + 128;
        const cr = 0.5 * r - 0.4542 * g - 0.0458 * b + 128;
        const val = ch === 'y' ? y : ch === 'cb' ? cb : cr;
        data[i] = data[i + 1] = data[i + 2] = val;
      }
    }

    ctx.putImageData(new ImageData(data, w, h), 0, 0);
  }

  function selectRgb(ch: 'all' | 'r' | 'g' | 'b' | 'a') {
    rgbSelected = ch;
    applyChannelFilter();
  }

  function selectYuv(ch: 'all' | 'y' | 'cb' | 'cr') {
    yuvSelected = ch;
    applyChannelFilter();
  }

  function switchMode(m: 'rgb' | 'yuv') {
    mode = m;
    if (m === 'rgb') rgbSelected = 'all';
    else yuvSelected = 'all';
    applyChannelFilter();
  }

  function selectFrame(index: number) {
    if (index < 0 || index >= gifFrames.length) return;
    if (playing) stopPlay();
    selectedFrame = index;
    originalImageData = null;
    frameInput = String(index + 1);
    // Scroll active thumbnail into view
    const activeThumb = document.querySelector('.frame-thumb.active');
    activeThumb?.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' });
  }

  function togglePlay() {
    if (playing) {
      stopPlay();
    } else {
      startPlay();
    }
  }

  function startPlay() {
    playing = true;
    playNextFrame();
  }

  function playNextFrame() {
    if (!playing || gifFrames.length === 0) return;

    const currentFrame = gifFrames[selectedFrame];
    const delay = currentFrame ? currentFrame.delay_ms : 100;
    const nextFrame = selectedFrame + 1;

    playTimer = setTimeout(() => {
      if (!playing) return;
      if (nextFrame >= gifFrames.length) {
        selectedFrame = 0;
        frameInput = '1';
        originalImageData = null;
      } else {
        selectedFrame = nextFrame;
        frameInput = String(nextFrame + 1);
        originalImageData = null;
      }
      playNextFrame();
    }, Math.max(delay, 50)); // minimum 50ms to avoid excessive CPU
  }

  function stopPlay() {
    playing = false;
    if (playTimer) {
      clearTimeout(playTimer);
      playTimer = null;
    }
  }

  function handleFrameInputChange(value: string) {
    frameInput = value;
    const num = parseInt(value, 10);
    if (!isNaN(num) && num >= 1 && num <= gifFrames.length) {
      selectFrame(num - 1);
    }
  }

  function handleSliderChange(e: Event) {
    const target = e.target as HTMLInputElement;
    const index = parseInt(target.value, 10);
    selectFrame(index);
  }
</script>

<div class="channels-tab">
  {#if loading || framesLoading}
    <div class="placeholder">Loading channel data...</div>
  {:else if isGif && gifFrames.length === 0 && !channels}
    <div class="placeholder">No channel data available for this format</div>
  {:else}
    {#if isGif && gifFrames.length > 0}
      <div class="frame-strip">
        <div class="frame-strip-header">
          <h3>GIF Frames ({gifFrames.length})</h3>
          <div class="playback-controls">
            <button
              class="play-btn"
              class:playing
              onclick={togglePlay}
              title={playing ? 'Pause' : 'Play'}
            >
              {#if playing}
                <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/></svg>
              {:else}
                <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
              {/if}
            </button>
            <div class="frame-nav">
              <input
                type="number"
                class="frame-input"
                value={frameInput}
                oninput={(e) => handleFrameInputChange(e.currentTarget.value)}
                min="1"
                max={gifFrames.length}
                title="Go to frame"
              />
              <span class="frame-total">/ {gifFrames.length}</span>
            </div>
          </div>
        </div>

        <div class="progress-bar-container">
          <input
            type="range"
            class="progress-slider"
            min="0"
            max={gifFrames.length - 1}
            value={selectedFrame}
            oninput={handleSliderChange}
          />
        </div>

        <div class="frame-thumbnails">
          {#each gifFrames as frame, i}
            <button
              class="frame-thumb"
              class:active={selectedFrame === i}
              onclick={() => selectFrame(i)}
              title="Frame {frame.index + 1} ({frame.delay_ms}ms)"
            >
              <img src={`data:image/png;base64,${frame.image_base64}`} alt="Frame {frame.index + 1}" />
              <span class="frame-label">#{frame.index + 1}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if channels}
  <div class="controls">
    <div class="mode-switcher">
      <button class:active={mode === 'rgb'} onclick={() => switchMode('rgb')}>RGB</button>
      <button class:active={mode === 'yuv'} onclick={() => switchMode('yuv')}>YUV</button>
    </div>
    {#if mode === 'rgb'}
      <div class="channel-buttons">
        <button class:active={rgbSelected === 'all'} onclick={() => selectRgb('all')}>All</button>
        <button class:active={rgbSelected === 'r'} onclick={() => selectRgb('r')}>R</button>
        <button class:active={rgbSelected === 'g'} onclick={() => selectRgb('g')}>G</button>
        <button class:active={rgbSelected === 'b'} onclick={() => selectRgb('b')}>B</button>
        {#if channels?.rgb?.a}
          <button class:active={rgbSelected === 'a'} onclick={() => selectRgb('a')}>A</button>
        {/if}
      </div>
    {:else}
      <div class="channel-buttons">
        <button class:active={yuvSelected === 'all'} onclick={() => selectYuv('all')}>All</button>
        <button class:active={yuvSelected === 'y'} onclick={() => selectYuv('y')}>Y</button>
        <button class:active={yuvSelected === 'cb'} onclick={() => selectYuv('cb')}>Cb</button>
        <button class:active={yuvSelected === 'cr'} onclick={() => selectYuv('cr')}>Cr</button>
      </div>
    {/if}
  </div>
    {/if}

  {#if imageSrc}
    <div class="image-container">
      <canvas bind:this={canvasRef}></canvas>
    </div>
  {:else}
    <div class="placeholder">No image data available</div>
  {/if}

  {#if channels?.rgb?.a && mode === 'rgb'}
    <div class="alpha-section">
      <h3>Alpha Channel</h3>
      <div class="alpha-container">
        <canvas bind:this={alphaCanvasRef} class="alpha-canvas"></canvas>
      </div>
    </div>
  {/if}

  {#if activeStats}
    <div class="stats-bar">
      <span class="stat-label">{currentChannel.toUpperCase()}:</span>
      <span>min {activeStats.min}</span>
      <span>max {activeStats.max}</span>
      <span>mean {activeStats.mean.toFixed(1)}</span>
      <span>median {activeStats.median}</span>
      <span>σ {activeStats.std_dev.toFixed(1)}</span>
    </div>
  {/if}
  {/if}
</div>

<style>
  .channels-tab {
    font-size: 0.875rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .frame-strip {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }

  .frame-strip-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .frame-strip-header h3 {
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
  }

  .playback-controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .play-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    color: white;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo),
                transform var(--duration-fast) var(--ease-out-expo);
  }

  .play-btn:hover {
    background: var(--accent-bright);
    transform: scale(1.05);
  }

  .play-btn.playing {
    background: var(--text-secondary);
  }

  .play-btn.playing:hover {
    background: var(--text-primary);
  }

  .frame-nav {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.75rem;
  }

  .frame-input {
    width: 3.5rem;
    padding: 0.2rem 0.4rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.75rem;
    text-align: center;
    outline: none;
    -moz-appearance: textfield;
  }

  .frame-input::-webkit-outer-spin-button,
  .frame-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .frame-input:focus {
    border-color: var(--accent);
  }

  .frame-total {
    color: var(--text-secondary);
  }

  .progress-bar-container {
    padding: 0 0.25rem;
  }

  .progress-slider {
    width: 100%;
    height: 6px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--bg-tertiary);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .progress-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    background: var(--accent);
    border-radius: 50%;
    cursor: pointer;
    transition: transform var(--duration-fast) var(--ease-out-expo);
  }

  .progress-slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }

  .progress-slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    background: var(--accent);
    border-radius: 50%;
    border: none;
    cursor: pointer;
  }

  .frame-thumbnails {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    max-height: 120px;
    overflow-y: auto;
  }

  .frame-thumb {
    position: relative;
    width: 64px;
    height: 64px;
    padding: 0;
    border: 2px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-tertiary);
    cursor: pointer;
    overflow: hidden;
    transition: border-color var(--duration-fast) var(--ease-out-expo);
    flex-shrink: 0;
  }

  .frame-thumb:hover {
    border-color: var(--accent);
  }

  .frame-thumb.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .frame-thumb img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .frame-label {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: rgba(0, 0, 0, 0.7);
    color: white;
    font-size: 0.625rem;
    text-align: center;
    padding: 1px 0;
    font-family: 'SF Mono', 'Cascadia Code', monospace;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
    padding: 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }

  .mode-switcher {
    display: flex;
    gap: 0.25rem;
  }

  .mode-switcher button {
    padding: 0.375rem 0.875rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 500;
    transition: background var(--duration-fast) var(--ease-out-expo),
                color var(--duration-fast) var(--ease-out-expo),
                border-color var(--duration-fast) var(--ease-out-expo);
  }

  .mode-switcher button:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .mode-switcher button.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .channel-buttons {
    display: flex;
    gap: 0.25rem;
  }

  .channel-buttons button {
    padding: 0.25rem 0.625rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 500;
    transition: background var(--duration-fast) var(--ease-out-expo),
                color var(--duration-fast) var(--ease-out-expo),
                border-color var(--duration-fast) var(--ease-out-expo);
  }

  .channel-buttons button:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .channel-buttons button.active {
    background: var(--accent-dim);
    color: var(--accent-bright);
    border-color: var(--accent);
  }

  .image-container {
    display: flex;
    justify-content: center;
    align-items: center;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    overflow: hidden;
    max-height: 60vh;
  }

  .image-container canvas {
    max-width: 100%;
    max-height: 60vh;
    image-rendering: pixelated;
  }

  .stats-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    padding: 0.625rem 0.875rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.75rem;
  }

  .stat-label {
    color: var(--accent-bright);
    font-weight: 600;
  }

  .stats-bar span:not(.stat-label) {
    color: var(--text-secondary);
  }

  .placeholder {
    text-align: center;
    color: var(--text-secondary);
    padding: 2rem;
  }

  .alpha-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .alpha-section h3 {
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .alpha-container {
    display: flex;
    justify-content: center;
    align-items: center;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    overflow: hidden;
    max-height: 40vh;
  }

  .alpha-canvas {
    max-width: 100%;
    max-height: 40vh;
    image-rendering: pixelated;
  }
</style>
