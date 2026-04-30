<script lang="ts">
  import DropZone from './components/DropZone.svelte';
  import FileList from './components/FileList.svelte';
  import MainPanel from './components/MainPanel.svelte';
  import { store } from './lib/store';
</script>

<div class="app">
  <aside class="sidebar">
    <h1 class="title">ImageAnalyzer</h1>
    <DropZone />
    <FileList />
  </aside>

  <main class="content">
    {#if store.isAnalyzing}
      <div class="loading-bar"></div>
    {/if}

    {#if store.error}
      <div class="error-banner">
        <span class="error-icon">!</span>
        <span class="error-text">{store.error}</span>
      </div>
    {/if}

    <MainPanel />
  </main>
</div>

<style>
  :global(:root) {
    --color-bg: #0d1117;
    --color-surface: #161b22;
    --color-surface-raised: #21262d;
    --color-border: #30363d;
    --color-text: #e6edf3;
    --color-text-muted: #8b949e;
    --color-accent: #58a6ff;
    --color-accent-dim: rgba(88, 166, 255, 0.1);
    --color-error: #f85149;
    --color-error-dim: rgba(248, 81, 73, 0.1);
    --color-hover: rgba(255, 255, 255, 0.05);
    --duration-fast: 150ms;
    --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
  }

  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(html, body, #app) {
    height: 100%;
    width: 100%;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background: var(--color-bg);
    color: var(--color-text);
    font-size: var(--text-base, 14px);
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
  }

  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .sidebar {
    flex: 0 0 30%;
    min-width: 240px;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
  }

  .title {
    font-size: 1rem;
    font-weight: 600;
    padding: 0.75rem 1rem;
    color: var(--color-text);
    letter-spacing: -0.01em;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-bg);
  }

  .loading-bar {
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--color-accent), transparent);
    animation: shimmer 1.5s ease-in-out infinite;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: var(--color-error-dim);
    border-bottom: 1px solid var(--color-error);
    font-size: 0.8125rem;
    color: var(--color-error);
  }

  .error-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 50%;
    background: var(--color-error);
    color: var(--color-bg);
    font-weight: 700;
    font-size: 0.75rem;
  }
</style>
