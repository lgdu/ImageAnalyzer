<script lang="ts">
  import type { ImageAnalysis } from '../lib/types';
  import { formatBytes } from '../lib/utils';

  interface Props {
    image: ImageAnalysis;
    isSelected: boolean;
    onclick?: () => void;
  }

  let { image, isSelected, onclick }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    if ((e.key === 'Enter' || e.key === ' ') && onclick) {
      e.preventDefault();
      onclick();
    }
  }

  const formatLabel: Record<string, string> = {
    png: 'PNG',
    jpeg: 'JPEG',
    webp: 'WebP',
    gif: 'GIF',
    avif: 'AVIF',
    heic: 'HEIC',
  };

  let thumbSrc = $derived(
    image.thumbnail_base64 ? `data:image/png;base64,${image.thumbnail_base64}` : ''
  );
</script>

<div class="thumbnail-card" class:active={isSelected} onclick={onclick} onkeydown={onKeydown} role="button" tabindex="0">
  {#if thumbSrc}
    <div class="thumb-wrapper">
      <img class="thumb" src={thumbSrc} alt="" loading="lazy" />
    </div>
  {/if}
  <div class="info">
    <span class="filename" title={image.file_name}>{image.file_name}</span>
    <div class="meta">
      <span class="badge">{formatLabel[image.format] ?? image.format}</span>
      <span class="dim">{image.width} × {image.height}</span>
      <span class="size">{formatBytes(image.file_size)}</span>
    </div>
  </div>
</div>

<style>
  .thumbnail-card {
    display: flex;
    flex-direction: column;
    margin: 0.25rem 0.625rem;
    border-radius: var(--radius-md);
    cursor: pointer;
    position: relative;
    overflow: hidden;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .thumbnail-card::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    width: 3px;
    height: 0;
    background: var(--accent);
    border-radius: 2px;
    transition: height var(--duration-normal) var(--ease-out-expo);
    z-index: 2;
  }

  .thumbnail-card:hover {
    background: var(--bg-hover);
  }

  .thumbnail-card:hover::before {
    height: 40%;
  }

  .thumbnail-card.active {
    background: var(--bg-active);
  }

  .thumbnail-card.active::before {
    height: 100%;
  }

  .thumbnail-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .thumb-wrapper {
    width: 100%;
    aspect-ratio: 1;
    background: var(--bg-tertiary);
    border-radius: var(--radius-md) var(--radius-md) 0 0;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .thumb {
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: auto;
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem 0.625rem 0.625rem;
    min-width: 0;
  }

  .filename {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.6875rem;
    color: var(--text-secondary);
  }

  .badge {
    display: inline-block;
    padding: 0.125rem 0.375rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 3px;
    font-weight: 600;
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent-bright);
  }

  .size {
    margin-left: auto;
  }
</style>
