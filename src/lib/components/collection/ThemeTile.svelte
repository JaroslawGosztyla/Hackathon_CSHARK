<script lang="ts">
  // The live tile of a collection theme (ADR-024): the theme's own picker art (`src/lib/themes/<id>/Preview.svelte`)
  // drawn with the theme's tokens (`data-theme` on the tile), and a mini UI on top, like the base theme cards. Used
  // by an unlocked card in the picker and by the unlock beat of a recap. Never for a locked theme (rule 10 style: a
  // locked card is blank). `still`: no motion (calm celebrations, reduced motion).
  import { REWARD_PREVIEWS } from "$lib/themes/scenes";
  import type { RewardTheme } from "$lib/types";

  let { theme, still = false }: { theme: RewardTheme; still?: boolean } = $props();

  const Preview = $derived(REWARD_PREVIEWS[theme] ?? null);
</script>

<span class="preview" class:still data-theme={theme} aria-hidden="true">
  {#if Preview}
    <span class="art"><Preview /></span>
  {/if}
  <span class="mini-nav"><span class="mini-highlight"></span></span>
  <span class="mini-card">
    <span class="mini-title">Aa</span>
    <span class="mini-line"></span>
    <span class="mini-pills"><span class="pill accent"></span><span class="pill break"></span></span>
  </span>
</span>

<style>
  .preview {
    position: relative;
    display: block;
    aspect-ratio: 16 / 10;
    overflow: hidden;
    border-radius: max(0px, calc(var(--radius-sm) - 4px));
    border: 1px solid var(--border);
    background: var(--app-bg);
    background-color: var(--bg-0);
    color: var(--text);
    font-family: var(--font);
    isolation: isolate;
  }

  .art {
    position: absolute;
    inset: 0;
    z-index: 0;
  }

  .still :global(*) {
    animation: none !important;
  }

  .mini-nav {
    position: absolute;
    z-index: 1;
    top: 9%;
    left: 8%;
    width: 42%;
    height: 12%;
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .mini-highlight {
    display: block;
    width: 38%;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--nav-highlight);
  }

  .mini-card {
    position: absolute;
    z-index: 1;
    left: 8%;
    bottom: 10%;
    width: 56%;
    height: 54%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: max(0px, calc(var(--radius) / 3));
    background: var(--surface);
    border: var(--border-w) solid var(--border);
    box-shadow: var(--shadow-sm);
  }

  .mini-title {
    font-size: 0.8rem;
    font-weight: 700;
    line-height: 1;
    color: var(--text);
  }

  .mini-line {
    display: block;
    width: 80%;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--text-muted);
    opacity: 0.7;
  }

  .mini-pills {
    display: flex;
    gap: 4px;
  }

  .pill {
    display: block;
    width: 28%;
    height: 8px;
    border-radius: var(--radius-pill);
  }

  .pill.accent {
    background: var(--accent);
  }

  .pill.break {
    background: var(--break);
  }
</style>
