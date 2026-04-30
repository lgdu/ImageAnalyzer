<script lang="ts">
  import { store } from '../lib/store';

  const tabs = [
    { id: 'structure', label: 'Structure' },
    { id: 'metadata', label: 'Metadata' },
    { id: 'channels', label: 'Channels' },
    { id: 'color-info', label: 'Color Info' },
    { id: 'codec-syntax', label: 'Codec Syntax' },
    { id: 'grid', label: 'Grid' },
  ] as const;

  type TabId = (typeof tabs)[number]['id'];

  let activeTab: TabId = $state('structure');

  function selectTab(id: TabId) {
    activeTab = id;
  }
</script>

<div class="main-panel">
  <div class="tab-bar" role="tablist">
    {#each tabs as tab}
      <button
        role="tab"
        class="tab"
        class:active={activeTab === tab.id}
        aria-selected={activeTab === tab.id}
        onclick={() => selectTab(tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <div class="tab-content">
    {#if !store.currentImage}
      <div class="placeholder">Select an image to view details</div>
    {:else if activeTab === 'structure'}
      <div class="placeholder">Structure tree — coming next</div>
    {:else if activeTab === 'metadata'}
      <div class="placeholder">Metadata tab — coming next</div>
    {:else if activeTab === 'channels'}
      <div class="placeholder">Channels tab — coming next</div>
    {:else if activeTab === 'color-info'}
      <div class="placeholder">Color Info tab — coming next</div>
    {:else if activeTab === 'codec-syntax'}
      <div class="placeholder">Codec Syntax tab — coming next</div>
    {:else if activeTab === 'grid'}
      <div class="placeholder">Grid tab — coming next</div>
    {/if}
  </div>
</div>

<style>
  .main-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--color-border);
    padding: 0 0.75rem;
    overflow-x: auto;
  }

  .tab {
    padding: 0.625rem 1rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-muted);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    transition: color var(--duration-fast) var(--ease-out-expo),
                border-color var(--duration-fast) var(--ease-out-expo);
  }

  .tab:hover {
    color: var(--color-text);
  }

  .tab.active {
    color: var(--color-accent);
    border-bottom-color: var(--color-accent);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
  }

  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 12rem;
    font-size: 0.875rem;
    color: var(--color-text-muted);
  }
</style>
