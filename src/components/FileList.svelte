<script lang="ts">
  import { store } from '../lib/store';
  import ThumbnailCard from './ThumbnailCard.svelte';

  function selectImage(image: typeof store.currentImage) {
    store.currentImage = image;
  }
</script>

<div class="file-list">
  {#if store.fileList.length === 0}
    <div class="empty">No files loaded</div>
  {:else}
    {#each store.fileList as image}
      <ThumbnailCard
        {image}
        isSelected={store.currentImage === image}
        onclick={() => selectImage(image)}
      />
    {/each}
  {/if}
</div>

<style>
  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem 0;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 4rem;
    font-size: 0.8125rem;
    color: var(--color-text-muted);
  }
</style>
