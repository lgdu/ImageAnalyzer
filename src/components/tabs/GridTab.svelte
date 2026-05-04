<script lang="ts">
  import type { ImageAnalysis } from '../../lib/types';
  import GridView from '../ui/GridView.svelte';

  let { data }: { data: ImageAnalysis } = $props();

  let previewSrc = $derived(
    data.thumbnail_base64 ? `data:image/png;base64,${data.thumbnail_base64}` : null
  );
</script>

<div class="grid-tab">
  {#if data.grid}
    <GridView grid={data.grid} imageSrc={previewSrc} />
  {:else}
    <p class="no-grid">This image does not contain a grid structure.</p>
  {/if}
</div>

<style>
  .no-grid {
    color: var(--text-secondary);
    text-align: center;
    padding: 3rem 2rem;
    font-size: 0.875rem;
  }
</style>
