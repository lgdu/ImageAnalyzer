<script lang="ts">
  import DropZone from './components/DropZone.svelte';
  import FileList from './components/FileList.svelte';
  import MainPanel from './components/MainPanel.svelte';
  import { store } from './lib/store.svelte';
</script>

<div class="app">
  <aside class="sidebar">
    <div class="sidebar-header">
      <div class="logo-wrapper">
        <img class="logo" src="/logo.png" alt="Logo" />
      </div>
      <span class="title">ImageAnalyzer</span>
    </div>
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
    --bg-primary: #0b0f19;
    --bg-secondary: #111827;
    --bg-tertiary: #1a2235;
    --bg-elevated: #1e293b;
    --bg-hover: rgba(99, 102, 241, 0.08);
    --bg-active: rgba(99, 102, 241, 0.15);
    --border-subtle: rgba(148, 163, 184, 0.08);
    --border-default: rgba(148, 163, 184, 0.15);
    --border-strong: rgba(148, 163, 184, 0.25);
    --text-primary: #f1f5f9;
    --text-secondary: #94a3b8;
    --text-tertiary: #64748b;
    --accent: #818cf8;
    --accent-bright: #a5b4fc;
    --accent-dim: rgba(129, 140, 248, 0.12);
    --accent-glow: rgba(129, 140, 248, 0.25);
    --success: #34d399;
    --warning: #fbbf24;
    --error: #f87171;
    --error-dim: rgba(248, 113, 113, 0.12);
    --radius-sm: 6px;
    --radius-md: 10px;
    --radius-lg: 14px;
    --shadow-sm: 0 1px 2px rgba(0,0,0,0.3);
    --shadow-md: 0 4px 12px rgba(0,0,0,0.4);
    --duration-fast: 150ms;
    --duration-normal: 250ms;
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
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 13px;
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
  }

  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .sidebar {
    flex: 0 0 300px;
    min-width: 260px;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-subtle);
    position: relative;
  }

  .sidebar::after {
    content: '';
    position: absolute;
    top: 0;
    right: -1px;
    width: 1px;
    height: 100%;
    background: linear-gradient(
      to bottom,
      transparent,
      var(--border-default) 10%,
      var(--border-default) 90%,
      transparent
    );
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-subtle);
    background: linear-gradient(135deg, var(--bg-secondary) 0%, var(--bg-tertiary) 100%);
  }

  .logo-wrapper {
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .logo {
    width: 1.25rem;
    height: 1.25rem;
    object-fit: contain;
  }

  .title {
    font-size: 0.9375rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    background: linear-gradient(135deg, var(--text-primary) 0%, var(--accent-bright) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-primary);
    position: relative;
  }

  .content::before {
    content: '';
    position: absolute;
    top: -200px;
    left: -200px;
    width: 600px;
    height: 600px;
    background: radial-gradient(circle, rgba(99, 102, 241, 0.03) 0%, transparent 70%);
    pointer-events: none;
    z-index: 0;
  }

  .loading-bar {
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    animation: shimmer 1.5s ease-in-out infinite;
    position: relative;
    z-index: 1;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.625rem 1rem;
    background: var(--error-dim);
    border-bottom: 1px solid rgba(248, 113, 113, 0.2);
    font-size: 0.8125rem;
    color: var(--error);
    position: relative;
    z-index: 1;
  }

  .error-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.125rem;
    height: 1.125rem;
    border-radius: 50%;
    background: var(--error);
    color: var(--bg-primary);
    font-weight: 700;
    font-size: 0.6875rem;
    flex-shrink: 0;
  }
</style>
