<script lang="ts">
  // Warm screen tint (ADR-014: an overlay, never gamma). One click-through window per monitor shows this page;
  // the shell makes the window ignore the mouse, `pointer-events: none` keeps the page itself inert too.
  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import type { TintStatus } from "$lib/types";

  /** Overlay opacity at full strength (level 100): warms the picture without hiding it. */
  const MAX_OPACITY = 0.45;

  let status = $state<TintStatus | null>(null);

  const opacity = $derived.by(() => {
    const level = status?.level ?? 0;
    if (!Number.isFinite(level)) return 0;
    return (Math.min(100, Math.max(0, level)) / 100) * MAX_OPACITY;
  });

  onMount(() => {
    let disposed = false;
    let unlisten: api.UnlistenFn | null = null;
    api
      .subscribeTint(
        (s) => (status = s),
        (e) => console.warn(`[FocusMode] ${e.code}: ${e.message}`),
      )
      .then((u) => {
        if (disposed) u();
        else unlisten = u;
      })
      .catch((e: unknown) => console.warn("[FocusMode] tint subscription failed", e));
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<svelte:head>
  <title>FocusMode</title>
</svelte:head>

<!-- Schedule changes fade slowly; the settings preview follows the slider quickly. -->
<div class="tint" class:preview={status?.preview} style:opacity aria-hidden="true"></div>

<style>
  :global(html[data-overlay] body) {
    pointer-events: none;
    user-select: none;
  }

  .tint {
    position: fixed;
    inset: 0;
    background: rgb(255, 130, 30);
    opacity: 0;
    transition: opacity 2s ease;
    pointer-events: none;
  }

  .tint.preview {
    transition-duration: 0.3s;
  }
</style>
