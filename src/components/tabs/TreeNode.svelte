<script lang="ts">
  import type { FileBlock } from '../../lib/types';
  import TreeNode from './TreeNode.svelte';

  let { node }: { node: FileBlock } = $props();

  let expanded: boolean = $state(false);
  let canExpand = $derived.by(() => {
    const f = node.fields;
    const c = node.children;
    return c.length > 0 || !!node.decoded_info || (f !== undefined && f.length > 0) || !!node.data_preview;
  });

  function toggle() {
    expanded = !expanded;
  }

  function formatBytes(len: number): string {
    if (len < 1024) return `${len} B`;
    if (len < 1024 * 1024) return `${(len / 1024).toFixed(1)} KB`;
    return `${(len / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<li class="tree-node">
  <button
    class="block-title-row"
    class:clickable={canExpand}
    onclick={toggle}
  >
    <span class="toggle-icon">
      {canExpand ? (expanded ? '−' : '+') : '·'}
    </span>
    <span class="node-name">{node.name}</span>
    <span class="node-size">{formatBytes(node.length)}</span>
    <span class="node-offset">0x{node.offset.toString(16).toUpperCase()}</span>
  </button>

  {#if expanded && canExpand}
    <div class="block-details">
      <dl class="field-list">
        <div class="field-row">
          <dt>Offset</dt>
          <dd>0x{node.offset.toString(16).toUpperCase()} ({node.offset})</dd>
        </div>
        <div class="field-row">
          <dt>Size</dt>
          <dd>{node.length} bytes ({formatBytes(node.length)})</dd>
        </div>
        {#if node.decoded_info}
          <div class="field-row">
            <dt>Decoded</dt>
            <dd class="decoded">{node.decoded_info}</dd>
          </div>
        {/if}
        {#each node.fields as field}
          <div class="field-row">
            <dt>{field[0]}</dt>
            <dd class="field-value">{field[1]}</dd>
          </div>
        {/each}
        {#if node.data_preview}
          <div class="field-row">
            <dt>Preview</dt>
            <dd class="hex-preview">{node.data_preview}</dd>
          </div>
        {/if}
      </dl>

      {#if node.children.length > 0}
        <ul class="child-list">
          {#each node.children as child}
            <TreeNode node={child} />
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</li>

<style>
  .tree-node {
    padding: 0;
    position: relative;
  }

  .block-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-sm);
    transition: background var(--duration-fast) var(--ease-out-expo);
    cursor: default;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    width: 100%;
    text-align: left;
    line-height: 1.8;
    min-height: 1.8rem;
  }

  .clickable {
    cursor: pointer;
  }

  .clickable:hover {
    background: var(--bg-hover);
  }

  .toggle-icon {
    width: 1.2em;
    text-align: center;
    color: var(--text-tertiary);
    flex-shrink: 0;
    font-size: 0.7rem;
  }

  .node-name {
    color: var(--accent-bright);
    font-weight: 600;
    min-width: 4ch;
    flex-shrink: 0;
  }

  .node-size {
    color: var(--text-secondary);
    min-width: 5ch;
    text-align: right;
    flex-shrink: 0;
  }

  .node-offset {
    color: var(--text-tertiary);
    margin-left: auto;
    flex-shrink: 0;
  }

  .block-details {
    padding-left: 1.5rem;
    margin-top: 0.25rem;
    margin-bottom: 0.25rem;
  }

  .field-list {
    margin: 0 0 0.5rem 0;
    padding: 0;
    border: none;
  }

  .field-row {
    display: flex;
    gap: 0.75rem;
    line-height: 1.6;
    margin-bottom: 0.15rem;
    flex-wrap: wrap;
  }

  .field-row dt {
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.6rem;
    font-weight: 600;
    padding-top: 0.1rem;
    flex-shrink: 0;
    min-width: 12ch;
  }

  .field-row dd {
    color: var(--text-secondary);
    margin: 0;
    word-break: break-all;
    overflow-wrap: break-word;
    padding-top: 0.1rem;
    flex: 1;
    min-width: 0;
    max-width: 100%;
  }

  .field-row dd.decoded {
    color: var(--text-secondary);
    font-style: italic;
  }

  .field-row dd.hex-preview {
    color: var(--text-tertiary);
    font-size: 0.6rem;
    letter-spacing: 0.02em;
    max-height: 4rem;
    overflow-y: auto;
  }

  .field-row dd.field-value {
    color: var(--text-primary);
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    font-size: 0.7rem;
  }

  .child-list {
    list-style: none;
    margin: 0;
    padding: 0 0 0 0.5rem;
    border-left: 1px solid var(--border-subtle);
  }
</style>
