<script lang="ts">
  import type { MetadataEntry } from '../../lib/types';

  let { entries }: { entries: MetadataEntry[] } = $props();

  // Auto-expand all standards on mount
  let expandedStandards: Set<string> = $state(new Set());

  let grouped: Map<string, MetadataEntry[]> = $derived(
    entries.reduce((map, e) => {
      if (!map.has(e.standard)) {
        map.set(e.standard, []);
      }
      map.get(e.standard)!.push(e);
      return map;
    }, new Map<string, MetadataEntry[]>())
  );

  // Auto-expand when entries change
  $effect(() => {
    if (entries.length > 0) {
      expandedStandards = new Set(grouped.keys());
    } else {
      expandedStandards.clear();
    }
  });

  function toggleStandard(standard: string) {
    if (expandedStandards.has(standard)) {
      expandedStandards.delete(standard);
    } else {
      expandedStandards.add(standard);
    }
  }

  function formatValue(value: string): [string, string][] {
    // Attempt to parse structured values like "key1=value1; key2=value2"
    if (!value.includes('=') && !value.includes(':')) return [['', value]];
    const pairs = value.split(/;\s*/);
    return pairs
      .map((p): [string, string] => {
        const sep = p.includes('=') ? '=' : ':';
        const idx = p.indexOf(sep);
        if (idx < 0) return ['', p];
        return [p.slice(0, idx).trim(), p.slice(idx + 1).trim()];
      })
      .filter(([k, v]) => k || v);
  }
</script>

<div class="metadata-panel">
  {#if entries.length === 0}
    <p class="empty">No metadata found</p>
  {:else}
    <div class="metadata-groups">
      {#each Array.from(grouped.entries()) as [standard, items]}
        <div class="metadata-group">
          <button
            class="group-header"
            onclick={() => toggleStandard(standard)}
          >
            <span class="group-toggle">
              {expandedStandards.has(standard) ? '▾' : '▸'}
            </span>
            <span class="group-name">{standard}</span>
            <span class="group-count">{items.length}</span>
          </button>
          {#if expandedStandards.has(standard)}
            <table class="metadata-table">
              <thead>
                <tr>
                  <th>Tag</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                {#each items as entry}
                  <tr>
                    <td class="tag-name">{entry.tag_name}</td>
                    <td class="tag-value">
                      {#if formatValue(entry.tag_value).length > 1}
                        <dl class="structured-value">
                          {#each formatValue(entry.tag_value) as [key, val]}
                            <div class="value-row">
                              {#if key}<dt>{key}</dt>{/if}
                              <dd>{val}</dd>
                            </div>
                          {/each}
                        </dl>
                      {:else}
                        {entry.tag_value}
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .metadata-panel {
    padding: 0.25rem;
  }

  .empty {
    color: var(--text-secondary);
    text-align: center;
    padding: 2rem;
  }

  .metadata-groups {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .metadata-group {
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-secondary);
    transition: border-color var(--duration-fast) var(--ease-out-expo);
  }

  .metadata-group:hover {
    border-color: var(--border-strong);
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.625rem 0.875rem;
    background: var(--bg-tertiary);
    border: none;
    cursor: pointer;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .group-header:hover {
    background: var(--bg-elevated);
  }

  .group-toggle {
    width: 1em;
    text-align: center;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .group-name {
    flex: 1;
  }

  .group-count {
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 400;
    background: var(--bg-elevated);
    padding: 0.125rem 0.375rem;
    border-radius: 10px;
    min-width: 1.25rem;
    text-align: center;
  }

  .metadata-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
  }

  .metadata-table thead {
    border-bottom: 1px solid var(--border-default);
  }

  .metadata-table th {
    text-align: left;
    padding: 0.375rem 0.875rem;
    color: var(--text-tertiary);
    font-weight: 500;
    text-transform: uppercase;
    font-size: 0.6875rem;
    letter-spacing: 0.05em;
    background: var(--bg-tertiary);
  }

  .metadata-table td {
    padding: 0.375rem 0.875rem;
    vertical-align: top;
    border-top: 1px solid var(--border-subtle);
  }

  .tag-name {
    color: var(--accent-bright);
    font-weight: 500;
    width: 12rem;
    min-width: 8rem;
  }

  .tag-value {
    color: var(--text-primary);
    word-break: break-word;
  }

  .structured-value {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .value-row {
    display: flex;
    gap: 0.5rem;
  }

  .value-row dt {
    color: var(--text-secondary);
    min-width: 8ch;
    flex-shrink: 0;
  }

  .value-row dd {
    margin: 0;
  }
</style>
