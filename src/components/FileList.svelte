<script lang="ts">
  import { store } from '../lib/store.svelte';
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
    scrollbar-width: thin;
    scrollbar-color: var(--border-default) transparent;
  }

  .file-list::-webkit-scrollbar {
    width: 5px;
  }

  .file-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .file-list::-webkit-scrollbar-thumb {
    background: var(--border-default);
    border-radius: 10px;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 4rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }
</style>
