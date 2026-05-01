<script lang="ts">
  let { bins, color = '#818cf8', label = '' }: { bins: number[]; color?: string; label?: string } = $props();

  const maxBin = $derived(Math.max(...bins, 1));

  function getHeight(val: number): number {
    return (val / maxBin) * 100;
  }
</script>

<div class="histogram">
  {#if label}<span class="hist-label">{label}</span>{/if}
  <svg viewBox="0 0 256 100" preserveAspectRatio="none" class="hist-svg">
    {#each bins as bin, i}
      <rect
        x={i}
        y={100 - getHeight(bin)}
        width="1"
        height={getHeight(bin)}
        fill={color}
        opacity="0.8"
      />
    {/each}
  </svg>
</div>

<style>
  .histogram {
    margin: 0.5rem 0;
  }
  .hist-label {
    font-size: 0.7rem;
    color: var(--color-muted, #64748b);
  }
  .hist-svg {
    width: 100%;
    height: 60px;
    background: rgba(255,255,255,0.02);
    border-radius: 2px;
  }
</style>
