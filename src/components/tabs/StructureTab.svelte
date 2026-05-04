<script lang="ts">
  import { store } from '../../lib/store.svelte';
  import TreeNode from './TreeNode.svelte';
</script>

<div class="structure-tree">
  {#if !store.currentImage}
    <p class="empty">Select an image to view structure</p>
  {:else}
    {@const structure = store.currentImage?.structure ?? []}
    {#if structure.length === 0}
      <p class="empty">No structure data available</p>
    {:else}
      <ul class="tree-list">
        {#each structure as rootNode}
          <TreeNode node={rootNode} />
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .structure-tree {
    padding: 0.25rem;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    font-size: 0.75rem;
    height: 100%;
    overflow-y: auto;
  }

  .empty {
    color: var(--text-secondary);
    text-align: center;
    padding: 2rem;
  }

  .tree-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
