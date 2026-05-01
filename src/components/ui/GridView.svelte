<script lang="ts">
  import type { GridInfo } from '../../lib/types';

  let { grid }: { grid: GridInfo } = $props();
</script>

<div class="grid-view">
  <div class="grid-header">
    <span>Grid: {grid.cols}×{grid.rows}</span>
    <span>Output: {grid.output_width}×{grid.output_height}</span>
    <span>Tiles: {grid.tiles.length}</span>
  </div>
  <div class="grid-container" style="grid-template-columns: repeat({grid.cols}, 1fr)">
    {#each grid.tiles as tile}
      <div class="grid-tile">
        <span class="tile-id">#{tile.item_id}</span>
        <span class="tile-size">{tile.width}×{tile.height}</span>
        <span class="tile-offset">({tile.horizontal_offset}, {tile.vertical_offset})</span>
        <span class="tile-codec">{tile.codec}</span>
      </div>
    {/each}
  </div>
  {#if grid.tiles.length === 0}
    <p class="no-tiles">Grid detected but tile details not yet parsed.</p>
  {/if}
</div>

<style>
  .grid-view {
    padding: 0.5rem;
  }
  .grid-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
    color: var(--color-muted, #64748b);
    margin-bottom: 0.75rem;
  }
  .grid-container {
    display: grid;
    gap: 4px;
  }
  .grid-tile {
    border: 1px solid var(--color-border, #334155);
    border-radius: 4px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background: rgba(255,255,255,0.02);
    min-height: 60px;
  }
  .tile-id {
    font-weight: 600;
    color: var(--color-accent, #818cf8);
    font-size: 0.85rem;
  }
  .tile-size {
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.75rem;
    color: var(--color-text, #e2e8f0);
  }
  .tile-offset {
    font-size: 0.7rem;
    color: var(--color-muted, #64748b);
  }
  .tile-codec {
    font-size: 0.7rem;
    color: var(--color-accent, #818cf8);
    font-family: monospace;
  }
  .no-tiles {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 1rem;
    font-size: 0.8rem;
  }
</style>
