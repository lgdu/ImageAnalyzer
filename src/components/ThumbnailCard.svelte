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
</script>

<div class="thumbnail-card" class:active={isSelected} onclick={onclick} onkeydown={onKeydown} role="button" tabindex="0">
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
    align-items: center;
    padding: 0.5rem 0.75rem;
    margin: 0 0.5rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .thumbnail-card:hover {
    background: var(--color-hover);
  }

  .thumbnail-card.active {
    background: var(--color-accent-dim);
    outline: 1px solid var(--color-accent);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }

  .filename {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.6875rem;
    color: var(--color-text-muted);
  }

  .badge {
    display: inline-block;
    padding: 0.125rem 0.375rem;
    background: var(--color-surface-raised);
    border-radius: 3px;
    font-weight: 600;
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
