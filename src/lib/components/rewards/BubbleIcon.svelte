<script lang="ts">
  // The streak glyph of the beta rewards: a water bubble (never a flame). A UI icon, not mascot art.
  // 8-bit theme: an 8×8 pixel-art ring.
  import { isPixelTheme } from "$lib/mascot/theme.svelte";

  interface Props {
    size?: number;
    /** Filled (the streak counts today), else a light fill. */
    filled?: boolean;
  }

  let { size = 16, filled = false }: Props = $props();

  const pixel = $derived(isPixelTheme());
</script>

{#if pixel}
  <svg class="bubble" class:filled viewBox="0 0 8 8" width={size} height={size} aria-hidden="true" shape-rendering="crispEdges">
    <path class="body" d="M2 1h4v1H2zM1 2h6v4H1zM2 6h4v1H2z" />
    <path class="px" d="M2 0h4v1H2zM1 1h1v1H1zM6 1h1v1H6zM0 2h1v4H0zM7 2h1v4H7zM1 6h1v1H1zM6 6h1v1H6zM2 7h4v1H2zM2 2h1v1H2z" />
  </svg>
{:else}
  <svg class="bubble" class:filled viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <circle class="body" cx="8" cy="8" r="6.4" />
    <circle class="line" cx="8" cy="8" r="6.4" stroke-width="1.6" />
    <path class="line" d="M5 6.2a3.4 3.4 0 0 1 2.4-2.1" stroke-width="1.4" stroke-linecap="round" />
  </svg>
{/if}

<style>
  .bubble {
    flex: none;
    vertical-align: -0.2em;
  }

  .body {
    fill: var(--bubble-soft);
  }

  .filled .body {
    fill: color-mix(in srgb, var(--bubble) 55%, transparent);
  }

  .line {
    fill: none;
    stroke: var(--bubble);
  }

  .px {
    fill: var(--bubble);
  }
</style>
