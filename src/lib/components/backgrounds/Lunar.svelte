<script lang="ts">
  // Lunar phases: a silver moon showing today's real phase (mean synodic month, see themes/moon.ts), a calm star
  // field, now and then a shooting star, and a low indigo mist.
  import { litPath, moonPhase } from "$lib/themes/moon";
  import { mulberry32 } from "./random";

  /** The phase changes slowly; refreshing twice an hour keeps a window left open overnight right. */
  const REFRESH_MS = 30 * 60_000;
  const R = 48;

  let now = $state(new Date());
  $effect(() => {
    const id = setInterval(() => (now = new Date()), REFRESH_MS);
    return () => clearInterval(id);
  });

  const phase = $derived(moonPhase(now));
  const lit = $derived(litPath(phase.fraction, 50, 50, R));

  const STARS = 70;
  const rand = mulberry32(29530589);
  const stars = Array.from({ length: STARS }, () => ({
    left: rand() * 100,
    top: rand() * 70,
    size: rand() < 0.15 ? 2 : 1,
    delay: -rand() * 10,
    duration: 7 + rand() * 6,
  }));
</script>

{#each stars as s, i (i)}
  <span
    class="star"
    class:bright={s.size > 1}
    style:left="{s.left}%"
    style:top="{s.top}%"
    style:width="{s.size}px"
    style:height="{s.size}px"
    style:animation-delay="{s.delay}s"
    style:animation-duration="{s.duration}s"
  ></span>
{/each}

<span class="shooting-star"></span>

<div class="moon-wrap">
  <div class="halo" style:opacity={0.12 + 0.4 * phase.illumination}></div>
  <svg class="moon" viewBox="0 0 100 100" focusable="false">
    <defs>
      <radialGradient id="lunar-surface" cx="42%" cy="38%" r="68%">
        <stop offset="0%" stop-color="#fbfcff" />
        <stop offset="55%" stop-color="#dfe3f6" />
        <stop offset="100%" stop-color="#aeb6d9" />
      </radialGradient>
      <clipPath id="lunar-lit">
        <path d={lit} />
      </clipPath>
    </defs>
    <!-- Earthshine: the dark part of the disk stays faintly visible. -->
    <circle cx="50" cy="50" r={R} class="earthshine" />
    <g clip-path="url(#lunar-lit)">
      <circle cx="50" cy="50" r={R} fill="url(#lunar-surface)" />
      <g class="maria">
        <ellipse cx="38" cy="34" rx="11" ry="8" />
        <ellipse cx="58" cy="30" rx="8" ry="6" />
        <ellipse cx="62" cy="52" rx="12" ry="9" />
        <ellipse cx="40" cy="60" rx="9" ry="7" />
        <ellipse cx="52" cy="74" rx="6" ry="4" />
        <circle cx="30" cy="48" r="3" />
        <circle cx="70" cy="70" r="2.4" />
        <circle cx="47" cy="45" r="1.8" />
      </g>
    </g>
  </svg>
</div>

<div class="mist"></div>

<style>
  .star {
    position: absolute;
    border-radius: 50%;
    background: #eef1ff;
    opacity: 0.3;
    animation-name: twinkle;
    animation-timing-function: ease-in-out;
    animation-iteration-count: infinite;
  }

  .star.bright {
    box-shadow: 0 0 4px 1px rgba(220, 228, 255, 0.35);
  }

  .moon-wrap {
    position: absolute;
    top: 7%;
    right: 10%;
    width: clamp(90px, 16vh, 150px);
    aspect-ratio: 1;
  }

  .halo {
    position: absolute;
    inset: -60%;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(200, 212, 255, 0.28) 0%, rgba(200, 212, 255, 0.08) 38%, transparent 68%);
  }

  .moon {
    position: relative;
    display: block;
    width: 100%;
    height: 100%;
    opacity: 0.88;
    filter: drop-shadow(0 0 18px rgba(214, 222, 255, 0.35));
  }

  .earthshine {
    fill: rgba(170, 182, 230, 0.1);
  }

  .maria ellipse,
  .maria circle {
    fill: rgba(112, 122, 168, 0.22);
  }

  .shooting-star {
    position: absolute;
    top: 12%;
    left: 18%;
    width: 120px;
    height: 1px;
    background: linear-gradient(90deg, rgba(238, 241, 255, 0) 0%, rgba(238, 241, 255, 0.85) 100%);
    opacity: 0;
    transform: rotate(18deg);
    transform-origin: right center;
    animation: shoot 17s linear infinite 4s;
  }

  .mist {
    position: absolute;
    inset: auto 0 0 0;
    height: 34%;
    background: linear-gradient(180deg, transparent 0%, rgba(40, 52, 110, 0.22) 60%, rgba(30, 38, 84, 0.35) 100%);
  }

  @keyframes twinkle {
    0%,
    100% {
      opacity: 0.15;
    }
    50% {
      opacity: 0.6;
    }
  }

  @keyframes shoot {
    0% {
      opacity: 0;
      transform: rotate(18deg) translateX(0);
    }
    1% {
      opacity: 0.9;
    }
    5% {
      opacity: 0;
      transform: rotate(18deg) translateX(320px);
    }
    100% {
      opacity: 0;
      transform: rotate(18deg) translateX(320px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .star {
      animation: none;
    }
    .shooting-star {
      display: none;
    }
  }
</style>
