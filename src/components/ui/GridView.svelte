<script lang="ts">
  import type { GridInfo } from '../../lib/types';

  let { grid, imageSrc = null }: { grid: GridInfo; imageSrc?: string | null } = $props();
</script>

<div class="grid-view">
  <div class="grid-header">
    <span>Grid: {grid.cols}×{grid.rows}</span>
    <span>Output: {grid.output_width}×{grid.output_height}</span>
    <span>Tiles: {grid.tiles.length}</span>
  </div>
  <div class="grid-stage-card">
    <div
      class="grid-stage"
      style:aspect-ratio={`${grid.output_width} / ${grid.output_height}`}
    >
      {#if imageSrc}
        <img class="grid-background" src={imageSrc} alt="Grid preview" />
      {:else}
        <div class="grid-background grid-fallback"></div>
      {/if}

      <div class="grid-overlay">
        {#each grid.tiles as tile, i}
          <div
            class="grid-region"
            style:left={`${(tile.horizontal_offset / grid.output_width) * 100}%`}
            style:top={`${(tile.vertical_offset / grid.output_height) * 100}%`}
            style:width={`${(tile.width / grid.output_width) * 100}%`}
            style:height={`${(tile.height / grid.output_height) * 100}%`}
          >
            <div class="grid-label">
              <span class="tile-index">{i + 1}</span>
              <span class="tile-meta">{tile.width}×{tile.height}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>

  {#if grid.tiles.length === 0}
    <p class="no-tiles">Grid detected but tile details not yet parsed.</p>
  {/if}
</div>

<style>
  .grid-view {
    padding: 0.25rem;
  }
  .grid-header {
    display: flex;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
    padding: 0.625rem 0.875rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }
  .grid-stage-card {
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    background: linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01));
    padding: 1rem;
  }
  .grid-stage {
    position: relative;
    width: min(100%, 920px);
    margin: 0 auto;
    overflow: hidden;
    border-radius: calc(var(--radius-lg) - 4px);
    background: #0a1020;
    box-shadow: inset 0 0 0 1px rgba(255,255,255,0.06);
  }
  .grid-background {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    user-select: none;
    pointer-events: none;
  }
  .grid-fallback {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(circle at 50% 50%, rgba(255,255,255,0.12), transparent 35%),
      linear-gradient(135deg, rgba(97, 218, 251, 0.12), rgba(255, 184, 108, 0.12)),
      #0a1020;
  }
  .grid-overlay {
    position: absolute;
    inset: 0;
  }
  .grid-region {
    position: absolute;
    border: 1.5px solid rgba(134, 197, 255, 0.95);
    background:
      linear-gradient(135deg, rgba(73, 144, 226, 0.18), rgba(73, 144, 226, 0.05));
    box-shadow:
      inset 0 0 0 1px rgba(255,255,255,0.12),
      0 0 0 1px rgba(0,0,0,0.35);
  }
  .grid-label {
    position: absolute;
    left: 0.35rem;
    top: 0.35rem;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0.42rem;
    border-radius: 999px;
    background: rgba(7, 12, 24, 0.78);
    backdrop-filter: blur(6px);
    color: #eef6ff;
    font-size: 0.7rem;
    line-height: 1;
  }
  .tile-index {
    display: inline-grid;
    place-items: center;
    min-width: 1.15rem;
    height: 1.15rem;
    border-radius: 999px;
    background: rgba(134, 197, 255, 0.22);
    font-weight: 700;
  }
  .tile-meta {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    opacity: 0.92;
  }
  .no-tiles {
    color: var(--text-secondary);
    text-align: center;
    padding: 1.25rem;
    font-size: 0.8rem;
  }
  @media (max-width: 720px) {
    .grid-stage-card {
      padding: 0.625rem;
    }
    .grid-label {
      left: 0.2rem;
      top: 0.2rem;
      padding: 0.16rem 0.3rem;
      gap: 0.3rem;
      font-size: 0.62rem;
    }
  }
</style>
