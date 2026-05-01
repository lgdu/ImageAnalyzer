<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';
  import GridView from '../ui/GridView.svelte';

  let { data }: { data: ImageAnalysis } = $props();
</script>

<div class="grid-tab">
  {#if data.grid}
    <GridView grid={data.grid} />
  {:else}
    <p class="no-grid">This image does not contain a grid structure.</p>
  {/if}
</div>

<style>
  .no-grid {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
  }
</style>
