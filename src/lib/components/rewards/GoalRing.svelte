<script lang="ts">
  // The daily goal as a small ring (`today.sparks / goal`). The daily-goal beat of a recap closes it (`from`: the
  // fraction it starts at). 8-bit theme: 12 square segments.
  import { isPixelTheme } from "$lib/mascot/theme.svelte";
  import { GOAL_RING_MS } from "$lib/rewards/choreography";
  import { strings } from "$lib/strings";

  interface Props {
    sparks: number;
    goal: number;
    met: boolean;
    /** Width and height in CSS pixels. */
    size?: number;
    /** Fraction shown first; the ring then moves to the real one (the closing ring of the daily-goal beat). */
    from?: number | null;
    /** No movement (calm, reduced motion, numbers only). */
    still?: boolean;
  }

  let { sparks, goal, met, size = 36, from = null, still = false }: Props = $props();

  const SEGMENTS = 12;
  const RADIUS = 15;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
  /** Pixel segments: 6×6 squares on a circle of radius 13 around the centre of the 36×36 box. */
  const CELLS = Array.from({ length: SEGMENTS }, (_, i) => {
    const angle = (i / SEGMENTS) * 2 * Math.PI;
    return { x: Math.round(18 + 13 * Math.sin(angle) - 3), y: Math.round(18 - 13 * Math.cos(angle) - 3) };
  });

  const fraction = $derived(met ? 1 : goal > 0 ? Math.min(1, Math.max(0, sparks / goal)) : 0);
  const pixel = $derived(isPixelTheme());

  // svelte-ignore state_referenced_locally
  let shown = $state(from ?? fraction);
  let animate = $state(false);

  $effect(() => {
    const target = fraction;
    if (still || pixel) {
      animate = false;
      shown = target;
      return;
    }
    // One frame at the start fraction, then the transition to the real one.
    const frame = requestAnimationFrame(() => {
      animate = true;
      shown = target;
    });
    return () => cancelAnimationFrame(frame);
  });

  const lit = $derived(Math.floor(shown * SEGMENTS + 1e-9));
</script>

<span
  class="goal-ring"
  class:met
  role="img"
  aria-label={strings.rewards.dailyGoalAria(sparks, goal)}
  style:width="{size}px"
  style:height="{size}px"
>
  {#if pixel}
    <svg viewBox="0 0 36 36" shape-rendering="crispEdges" aria-hidden="true">
      {#each CELLS as cell, i (i)}
        <rect class="cell" class:on={i < lit} x={cell.x} y={cell.y} width="6" height="6" />
      {/each}
    </svg>
  {:else}
    <svg viewBox="0 0 36 36" aria-hidden="true">
      <circle class="track" cx="18" cy="18" r={RADIUS} />
      <circle
        class="value"
        cx="18"
        cy="18"
        r={RADIUS}
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={CIRCUMFERENCE * (1 - shown)}
        transform="rotate(-90 18 18)"
        style:transition-duration={animate ? `${GOAL_RING_MS}ms` : "0ms"}
      />
    </svg>
  {/if}
  {#if met}
    <span class="check" aria-hidden="true">✓</span>
  {/if}
</span>

<style>
  .goal-ring {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
  }

  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .track {
    fill: none;
    stroke: var(--track-strong);
    stroke-width: 4;
  }

  .value {
    fill: none;
    stroke: var(--reward);
    stroke-width: 4;
    stroke-linecap: round;
    transition-property: stroke-dashoffset;
    transition-timing-function: cubic-bezier(0.22, 0.61, 0.36, 1);
  }

  .cell {
    fill: var(--track-strong);
  }

  .cell.on {
    fill: var(--reward);
  }

  .check {
    position: relative;
    color: var(--reward-ink);
    font-size: 0.85rem;
    font-weight: 800;
    line-height: 1;
  }
</style>
