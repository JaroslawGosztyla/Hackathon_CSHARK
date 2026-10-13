<script lang="ts">
  // Cozy autumn: leaves drift down slowly, swaying, and a few rest along the bottom edge. With reduced motion the
  // falling leaves stay still where they are.
  import { mulberry32 } from "./random";

  const FALLING = 16;
  const RESTING = 8;
  const COLORS = ["#c8641e", "#e08a2e", "#a8461b", "#d9a03b", "#b5532a", "#8f3b17"] as const;

  const rand = mulberry32(1023);
  const pick = () => COLORS[Math.floor(rand() * COLORS.length)] ?? COLORS[0];

  const falling = Array.from({ length: FALLING }, () => {
    const duration = 16 + rand() * 14;
    return {
      left: rand() * 100,
      size: 14 + rand() * 16,
      duration,
      delay: -rand() * duration,
      sway: 24 + rand() * 46,
      swayDuration: 3 + rand() * 3,
      spin: rand() < 0.5 ? -1 : 1,
      color: pick(),
      maple: rand() < 0.45,
      restTop: 8 + rand() * 84,
      opacity: 0.45 + rand() * 0.35,
    };
  });

  const resting = Array.from({ length: RESTING }, () => ({
    left: rand() * 100,
    size: 16 + rand() * 12,
    rotate: rand() * 360,
    color: pick(),
    maple: rand() < 0.5,
  }));

  /** Simple maple leaf and a plain oval leaf, both in a 24 × 24 box. */
  const MAPLE =
    "M12 1.5l1.7 3.9 2.7-1.3-.6 3.6 3.6-.9-1.5 2.8 3.6 1.3-3.2 1.9 1.1 1.7-3.8.2.4 3-3-1.5-.6 6.3h-.8l-.6-6.3-3 1.5.4-3-3.8-.2 1.1-1.7-3.2-1.9 3.6-1.3-1.5-2.8 3.6.9-.6-3.6 2.7 1.3z";
  const OVAL = "M12 2c5 4.5 7 8.4 7 12a7 7 0 0 1-14 0c0-3.6 2-7.5 7-12z";
</script>

{#each falling as l, i (i)}
  <span
    class="leaf"
    style:left="{l.left}%"
    style:--rest-top="{l.restTop}%"
    style:opacity={l.opacity}
    style:animation-duration="{l.duration}s"
    style:animation-delay="{l.delay}s"
  >
    <span
      class="sway"
      style:--sway="{l.sway}px"
      style:--spin={l.spin}
      style:animation-duration="{l.swayDuration}s"
      style:animation-delay="{l.delay}s"
    >
      <svg viewBox="0 0 24 24" width={l.size} height={l.size} focusable="false">
        <path d={l.maple ? MAPLE : OVAL} fill={l.color} />
        {#if !l.maple}<path d="M12 6v15" class="vein" />{/if}
      </svg>
    </span>
  </span>
{/each}

{#each resting as l, i (i)}
  <svg
    class="resting"
    viewBox="0 0 24 24"
    width={l.size}
    height={l.size}
    style:left="{l.left}%"
    style:transform="rotate({l.rotate}deg)"
    focusable="false"
  >
    <path d={l.maple ? MAPLE : OVAL} fill={l.color} />
  </svg>
{/each}

<div class="ground"></div>

<style>
  .leaf {
    position: absolute;
    top: -40px;
    animation-name: fall;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    will-change: transform;
  }

  .sway {
    display: block;
    animation-name: sway;
    animation-timing-function: ease-in-out;
    animation-iteration-count: infinite;
    animation-direction: alternate;
  }

  svg {
    display: block;
  }

  .vein {
    stroke: rgba(40, 18, 6, 0.35);
    stroke-width: 1;
    fill: none;
  }

  .resting {
    position: absolute;
    bottom: -6px;
    opacity: 0.5;
  }

  .ground {
    position: absolute;
    inset: auto 0 0 0;
    height: 26%;
    background: linear-gradient(180deg, transparent 0%, rgba(60, 30, 12, 0.28) 100%);
  }

  @keyframes fall {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(0, calc(100vh + 80px), 0);
    }
  }

  @keyframes sway {
    from {
      transform: translateX(calc(var(--sway) * -1)) rotate(calc(var(--spin) * -35deg));
    }
    to {
      transform: translateX(var(--sway)) rotate(calc(var(--spin) * 35deg));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .leaf {
      top: var(--rest-top);
      animation: none;
    }
    .sway {
      animation: none;
      transform: rotate(calc(var(--spin) * 20deg));
    }
  }
</style>
