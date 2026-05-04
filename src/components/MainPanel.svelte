<script lang="ts">
  import { store } from '../lib/store.svelte';
  import ChannelsTab from './tabs/ChannelsTab.svelte';
  import CodecSyntaxTab from './tabs/CodecSyntaxTab.svelte';
  import ColorInfoTab from './tabs/ColorInfoTab.svelte';
  import GridTab from './tabs/GridTab.svelte';
  import MetadataTab from './tabs/MetadataTab.svelte';
  import StructureTab from './tabs/StructureTab.svelte';

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
      <StructureTab />
    {:else if activeTab === 'metadata'}
      <MetadataTab entries={store.currentImage.metadata} />
    {:else if activeTab === 'channels'}
      <ChannelsTab filePath={store.currentImage.file_path} format={store.currentImage.format} />
    {:else if activeTab === 'color-info'}
      <ColorInfoTab filePath={store.currentImage.file_path} />
    {:else if activeTab === 'codec-syntax'}
      {#if store.currentImage?.codec_syntax}
        <CodecSyntaxTab codec={store.currentImage.codec_syntax} />
      {:else}
        <div class="placeholder">No codec syntax data for this image</div>
      {/if}
    {:else if activeTab === 'grid'}
      <GridTab data={store.currentImage} />
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
    border-bottom: 1px solid var(--border-subtle);
    padding: 0 0.5rem;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--bg-secondary);
  }

  .tab-bar::-webkit-scrollbar {
    display: none;
  }

  .tab {
    padding: 0.75rem 1rem;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    position: relative;
    transition: color var(--duration-fast) var(--ease-out-expo),
                border-color var(--duration-fast) var(--ease-out-expo);
  }

  .tab::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 50%;
    width: 0;
    height: 2px;
    background: var(--accent);
    transform: translateX(-50%);
    transition: width var(--duration-normal) var(--ease-out-expo);
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab:hover::after {
    width: 60%;
  }

  .tab.active {
    color: var(--accent-bright);
  }

  .tab.active::after {
    width: 100%;
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem;
    background: var(--bg-primary);
    scrollbar-width: thin;
    scrollbar-color: var(--border-default) transparent;
  }

  .tab-content::-webkit-scrollbar {
    width: 6px;
  }

  .tab-content::-webkit-scrollbar-track {
    background: transparent;
  }

  .tab-content::-webkit-scrollbar-thumb {
    background: var(--border-default);
    border-radius: 10px;
  }

  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 12rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }
</style>
