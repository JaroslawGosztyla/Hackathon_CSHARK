<script lang="ts">
  // "Poziom N" pill and a thin bar towards the next level (levels never go down). The level-up beat flashes it.
  // 8-bit theme: a segmented bar.
  import { isPixelTheme } from "$lib/mascot/theme.svelte";
  import { LEVEL_FLASH_MS } from "$lib/rewards/choreography";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { LevelView } from "$lib/types";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    level: LevelView;
    /** The level-up beat: a short flash of the bar. */
    flash?: boolean;
    /** Narrow: the pill shows "Poz. N". */
    compact?: boolean;
  }

  let { level, flash = false, compact = false }: Props = $props();

  const SEGMENTS = 10;
  const fraction = $derived(level.span > 0 ? Math.min(1, Math.max(0, level.into / level.span)) : 0);
  const pixel = $derived(isPixelTheme());
  const lit = $derived(Math.floor(fraction * SEGMENTS + 1e-9));
  /** Calm, numbers only or reduced motion: the bar jumps to its value. */
  const still = $derived(rewards.celebration !== "full");
</script>

<span
  class="level-bar"
  class:flash
  class:still
  role="img"
  aria-label={strings.rewards.levelAria(level.level, level.into, level.span)}
  style:--flash-ms="{LEVEL_FLASH_MS}ms"
>
  <span class="pill" aria-hidden="true">
    <SparkIcon size={11} />
    {compact ? strings.rewards.levelShort(level.level) : strings.rewards.level(level.level)}
  </span>
  {#if pixel}
    <span class="segments" aria-hidden="true">
      {#each { length: SEGMENTS } as _, i (i)}
        <span class="segment" class:on={i < lit}></span>
      {/each}
    </span>
  {:else}
    <span class="bar" aria-hidden="true"><span class="fill" style:transform="scaleX({fraction})"></span></span>
  {/if}
</span>

<style>
  .level-bar {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
    padding: 1px 9px;
    border-radius: var(--radius-pill);
    background: var(--reward-soft);
    color: var(--reward-ink);
    font-size: 0.8rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .bar {
    position: relative;
    display: block;
    flex: 1 1 64px;
    min-width: 48px;
    max-width: 120px;
    height: 6px;
    border-radius: var(--radius-pill);
    background: var(--track-strong);
    overflow: hidden;
  }

  .fill {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--reward);
    transform-origin: left center;
    transition: transform 0.6s cubic-bezier(0.22, 0.61, 0.36, 1);
  }

  .still .fill {
    transition: none;
  }

  .segments {
    display: inline-flex;
    gap: 2px;
  }

  .segment {
    width: 6px;
    height: 8px;
    background: var(--track-strong);
  }

  .segment.on {
    background: var(--reward);
  }

  .flash .bar,
  .flash .segments {
    animation: level-flash var(--flash-ms) steps(2, jump-none) 2;
  }

  @keyframes level-flash {
    50% {
      filter: brightness(1.8);
    }
  }
</style>
