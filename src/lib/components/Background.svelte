<script lang="ts">
  // Window background: the theme's sky (`--app-bg`) plus its decorations. Purely visual (aria-hidden). The layer is
  // a little wider than the window and pans left as the user moves right through the tabs (`pan` = tab index),
  // so switching tabs feels like moving along one scene. The whole pan range stays the same with 3 or 4 tabs (the
  // Developer tab, ADR-019): `steps` (tabs − 1) only divides it, so the scene never shows its edge or resizes.
  // Static when the user prefers reduced motion.
  import type { Component } from "svelte";
  import { isTheme, type Theme, type ThemeId } from "$lib/themes";
  import { REWARD_SCENES } from "$lib/themes/scenes";
  import Autumn from "./backgrounds/Autumn.svelte";
  import Lunar from "./backgrounds/Lunar.svelte";
  import Midnight from "./backgrounds/Midnight.svelte";
  import Pixel from "./backgrounds/Pixel.svelte";
  import Vaporwave from "./backgrounds/Vaporwave.svelte";

  let { theme, pan = 0, steps = 2 }: { theme: ThemeId; pan?: number; steps?: number } = $props();

  /** At least one step (a single tab never pans), and the pan never runs past the last step. */
  const stepCount = $derived(Math.max(1, Math.trunc(steps)));
  const panSteps = $derived(Math.min(Math.max(0, pan), stepCount));

  /** Basic dark and basic light are plain on purpose: no decorations. */
  const SCENES: Record<Theme, Component | null> = {
    midnight: Midnight,
    vaporwave: Vaporwave,
    lunar: Lunar,
    autumn: Autumn,
    dark: null,
    light: null,
    pixel: Pixel,
  };

  /** A beta collection theme (ADR-024) brings its own scene from its folder (`$lib/themes/scenes`). */
  const Scene = $derived(isTheme(theme) ? SCENES[theme] : (REWARD_SCENES[theme] ?? null));
</script>

<div class="backdrop" aria-hidden="true" style:--pan={panSteps} style:--pan-steps={stepCount}>
  {#if Scene}
    <Scene />
  {/if}
</div>

<style>
  .backdrop {
    /* The whole pan (first tab → last tab); the layer is that much wider than the window so its edge never shows. */
    --pan-range: 5vw;
    /* One tab step of the pan: 2.5vw with 3 tabs, a third of the range with 4. */
    --pan-step: calc(var(--pan-range) / var(--pan-steps, 2));
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    width: calc(100% + var(--pan-range) + 2px);
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
    background: var(--app-bg);
    background-color: var(--bg-0);
    transform: translate3d(calc(var(--pan, 0) * var(--pan-step) * -1), 0, 0);
    transition: transform var(--pan-ms) var(--ease-slide);
    will-change: transform;
  }

  @media (prefers-reduced-motion: reduce) {
    .backdrop {
      transform: none;
      transition: none;
    }
  }
</style>
