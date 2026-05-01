<script lang="ts">
  import type { FileBlock } from '../../lib/types';

  let { blocks }: { blocks: FileBlock[] } = $props();

  let expandedNodes: Set<number> = $state(new Set());

  function toggleNode(offset: number) {
    if (expandedNodes.has(offset)) {
      expandedNodes.delete(offset);
    } else {
      expandedNodes.add(offset);
    }
  }

  function formatBytes(len: number): string {
    if (len < 1024) return `${len} B`;
    if (len < 1024 * 1024) return `${(len / 1024).toFixed(1)} KB`;
    return `${(len / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="structure-tree">
  {#if blocks.length === 0}
    <p class="empty">No structure data available</p>
  {:else}
    <ul class="tree-list">
      {#each blocks as block}
        <li class="tree-node">
          {#if block.children.length > 0}
            <button
              class="tree-row has-children"
              onclick={() => toggleNode(block.offset)}
            >
              <span class="toggle-icon">{expandedNodes.has(block.offset) ? '▾' : '▸'}</span>
              <span class="node-name">{block.name}</span>
              <span class="node-size">{formatBytes(block.length)}</span>
              <span class="node-offset">0x{block.offset.toString(16).toUpperCase()}</span>
            </button>
          {:else}
            <div class="tree-row">
              <span class="toggle-icon">·</span>
              <span class="node-name">{block.name}</span>
              <span class="node-size">{formatBytes(block.length)}</span>
              <span class="node-offset">0x{block.offset.toString(16).toUpperCase()}</span>
            </div>
          {/if}
          {#if block.children.length > 0 && expandedNodes.has(block.offset)}
            <ul class="child-list">
              {#each block.children as child}
                <li class="tree-node">
                  <div class="tree-row">
                    <span class="toggle-icon">·</span>
                    <span class="node-name">{child.name}</span>
                    <span class="node-size">{formatBytes(child.length)}</span>
                    <span class="node-offset">0x{child.offset.toString(16).toUpperCase()}</span>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
          {#if block.decoded_info}
            <div class="node-info">{block.decoded_info}</div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .structure-tree {
    padding: 0.5rem;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    font-size: 0.75rem;
  }

  .empty {
    color: var(--color-muted, #64748b);
    text-align: center;
    padding: 2rem;
  }

  .tree-list,
  .child-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .child-list {
    margin-left: 1.25rem;
    border-left: 1px solid var(--color-border, #334155);
  }

  .tree-node {
    padding: 0;
  }

  .tree-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.2rem 0.375rem;
    border-radius: 3px;
    transition: background var(--duration-fast, 150ms) var(--ease-out-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .tree-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .has-children {
    cursor: pointer;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    width: 100%;
    text-align: left;
    padding: 0.2rem 0.375rem;
  }

  .toggle-icon {
    width: 1em;
    text-align: center;
    color: var(--color-muted, #64748b);
    flex-shrink: 0;
  }

  .node-name {
    color: var(--color-accent, #818cf8);
    font-weight: 600;
    min-width: 4ch;
  }

  .node-size {
    color: var(--color-muted, #64748b);
    min-width: 5ch;
    text-align: right;
  }

  .node-offset {
    color: var(--color-muted, #64748b);
    margin-left: auto;
  }

  .node-info {
    color: var(--color-muted, #64748b);
    padding: 0.1rem 0 0.1rem 1.625rem;
    font-size: 0.7rem;
    font-style: italic;
  }
</style>
