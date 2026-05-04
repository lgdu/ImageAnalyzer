<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { store } from '../lib/store.svelte';
  import type { ImageAnalysis } from '../lib/types';

  let dragOver = $state(false);

  const ACCEPTED_EXTENSIONS = ['.png', '.jpg', '.jpeg', '.webp', '.gif', '.avif', '.heic', '.heif'];

  async function handlePaths(paths: string[]) {
    store.isAnalyzing = true;
    const errors: string[] = [];

    // Close old files: clear file list and current image
    store.fileList = [];
    store.currentImage = null;

    for (const filePath of paths) {
      try {
        const result: ImageAnalysis = await invoke('analyze_image', { filePath });
        store.fileList.push(result);
        if (store.currentImage === null) {
          store.currentImage = result;
        }
        store.error = null;
      } catch (e: unknown) {
        errors.push(e instanceof Error ? e.message : String(e));
      }
    }

    if (errors.length > 0) {
      store.error = errors.join('; ');
    }
    store.isAnalyzing = false;
  }

  async function openFileDialog() {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif', 'heic', 'heif'],
        },
      ],
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];
    await handlePaths(paths);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openFileDialog();
    }
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    dragOver = true;
  }

  function onDragLeave() {
    dragOver = false;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;

    // Extract file paths from dropped files using Tauri-compatible method
    const paths: string[] = [];

    // Try URI list first (Tauri provides this)
    const uriList = e.dataTransfer?.getData('text/uri-list');
    if (uriList) {
      const uris = uriList.split('\n').filter(Boolean);
      for (const uri of uris) {
        // Convert file:// URIs to paths
        const path = uri.replace(/^file:\/\//, '').replace(/\?.*$/, '');
        const decoded = decodeURIComponent(path);
        const ext = '.' + decoded.split('.').pop()?.toLowerCase();
        if (ACCEPTED_EXTENSIONS.includes(ext)) {
          paths.push(decoded);
        }
      }
    }

    // Fallback: use File objects from dataTransfer
    if (paths.length === 0 && e.dataTransfer?.files) {
      const files = Array.from(e.dataTransfer.files);
      for (const file of files) {
        const ext = '.' + file.name.split('.').pop()?.toLowerCase();
        if (ACCEPTED_EXTENSIONS.includes(ext)) {
          // Use file.path if available (Tauri provides it), otherwise name
          interface TauriFile extends File {
      path?: string;
    }
    const filePath = (file as TauriFile).path || file.name;
          paths.push(filePath);
        }
      }
    }

    if (paths.length > 0) {
      handlePaths(paths);
    } else {
      // If we couldn't get paths, open the file dialog
      openFileDialog();
    }
  }
</script>

<div
  class="dropzone"
  class:drag-over={dragOver}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
  onclick={openFileDialog}
  onkeydown={onKeydown}
  role="button"
  tabindex="0"
  aria-label="Drop images here or click to select"
>
  {#if store.isAnalyzing}
    <div class="spinner"></div>
    <span class="label">Analyzing...</span>
  {:else}
    <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
      <path d="M12 16V4m0 0l-4 4m4-4l4 4" />
      <path d="M2 17l.621 2.485A2 2 0 004.561 21h14.878a2 2 0 001.94-1.515L22 17" />
    </svg>
    <span class="label">Drop images here or click to browse</span>
    <span class="hint">PNG, JPG, WebP, GIF, AVIF, HEIC</span>
  {/if}
</div>

<style>
  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1.75rem 1.5rem;
    margin: 0.75rem;
    border: 2px dashed var(--border-default);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: border-color var(--duration-fast) var(--ease-out-expo),
                background var(--duration-fast) var(--ease-out-expo),
                box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .dropzone:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .drag-over {
    border-color: var(--accent-bright);
    background: var(--bg-active);
    box-shadow: 0 0 20px var(--accent-glow);
  }

  .icon {
    width: 2rem;
    height: 2rem;
    color: var(--text-secondary);
    transition: color var(--duration-fast) var(--ease-out-expo);
  }

  .dropzone:hover .icon {
    color: var(--accent);
  }

  .label {
    font-size: 0.8125rem;
    color: var(--text-primary);
    font-weight: 500;
  }

  .hint {
    font-size: 0.6875rem;
    color: var(--text-tertiary);
    letter-spacing: 0.02em;
  }

  .spinner {
    width: 1.5rem;
    height: 1.5rem;
    border: 2px solid var(--border-default);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
