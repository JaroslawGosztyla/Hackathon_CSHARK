<script lang="ts">
  // One cherry branch in blossom (600 × 420 box, see ./branch.ts), filling its parent's width. Used twice by the
  // Scene, each with its own slow sway (`duration`, `delay`); still under reduced motion.
  import { BLOSSOMS, BRANCH, FLOWER_PETAL, PUFFS } from "./branch";

  let { duration = 11, delay = 0 }: { duration?: number; delay?: number } = $props();

  const uid = $props.id();
</script>

<svg
  class="sway"
  viewBox="0 0 600 420"
  style:animation-duration="{duration}s"
  style:animation-delay="{delay}s"
  focusable="false"
>
  <defs>
    <path id="{uid}-petal" d={FLOWER_PETAL} />
    <g id="{uid}-flower">
      {#each [0, 72, 144, 216, 288] as a (a)}
        <use href="#{uid}-petal" transform="rotate({a})" />
      {/each}
      <circle r="1.7" fill="#ffe3a8" />
    </g>
    <radialGradient id="{uid}-puff">
      <stop offset="0%" stop-color="#f6b6d3" stop-opacity="1" />
      <stop offset="100%" stop-color="#f6b6d3" stop-opacity="0" />
    </radialGradient>
  </defs>
  <g fill="none" stroke="#3a2340" stroke-linecap="round">
    {#each BRANCH as b, i (i)}
      <path d={b.d} stroke-width={b.w} />
    {/each}
  </g>
  {#each PUFFS as p, i (i)}
    <circle cx={p.x} cy={p.y} r={p.r} fill="url(#{uid}-puff)" opacity={p.o * 2} />
  {/each}
  {#each BLOSSOMS as f, i (i)}
    <use href="#{uid}-flower" transform="translate({f.x} {f.y}) rotate({f.r}) scale({f.s})" fill={f.c} opacity={f.o} />
  {/each}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }

  /* The whole branch sways a little from where it enters the window (its top-left corner). */
  .sway {
    transform-origin: 0 0;
    animation: sway ease-in-out infinite alternate;
    will-change: transform;
  }

  @keyframes sway {
    from {
      transform: rotate(-0.7deg);
    }
    to {
      transform: rotate(0.6deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sway {
      animation: none;
    }
  }
</style>
