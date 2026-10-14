<script lang="ts">
  // Vaporwave: a striped sunset sun on the horizon and a neon grid floor in perspective that slowly rolls toward
  // the viewer. The sky gradient is the theme's `--app-bg` (horizon at 62 %).
  import { mulberry32 } from "./random";

  const STARS = 26;
  const rand = mulberry32(1984);
  const stars = Array.from({ length: STARS }, () => ({
    left: rand() * 100,
    top: rand() * 40,
    size: rand() < 0.25 ? 2 : 1,
    delay: -rand() * 6,
  }));
</script>

{#each stars as s, i (i)}
  <span
    class="star"
    style:left="{s.left}%"
    style:top="{s.top}%"
    style:width="{s.size}px"
    style:height="{s.size}px"
    style:animation-delay="{s.delay}s"
  ></span>
{/each}

<div class="sky">
  <div class="sun-glow"></div>
  <div class="sun"></div>
</div>
<div class="horizon"></div>
<div class="floor">
  <div class="plane">
    <div class="grid"></div>
  </div>
</div>

<style>
  .star {
    position: absolute;
    background: #ffe6fb;
    border-radius: 50%;
    opacity: 0.4;
    animation: twinkle 6s ease-in-out infinite;
  }

  /* Everything above the horizon; the sun sinks below its bottom edge. */
  .sky {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 62%;
    overflow: hidden;
  }

  /* Off to the right of the centred timer, so the digits never sit on the bright sun. */
  .sun,
  .sun-glow {
    position: absolute;
    left: 80%;
    bottom: 0;
    width: clamp(160px, 34vh, 340px);
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, 24%);
  }

  .sun-glow {
    box-shadow: 0 0 90px 30px rgba(255, 113, 206, 0.3);
  }

  .sun {
    background: linear-gradient(180deg, #fffb96 0%, #ffd27f 28%, #ff9eb5 55%, #ff71ce 74%, #b967ff 100%);
    opacity: 0.75;
    /* The classic sunset stripes: gaps grow toward the bottom. */
    mask-image: linear-gradient(
      180deg,
      #000 0 50%,
      transparent 50% 53%,
      #000 53% 61%,
      transparent 61% 65%,
      #000 65% 72%,
      transparent 72% 77%,
      #000 77% 83%,
      transparent 83% 89%,
      #000 89%
    );
  }

  .horizon {
    position: absolute;
    left: 0;
    right: 0;
    top: 62%;
    height: 2px;
    background: #ff71ce;
    box-shadow: 0 0 24px 6px rgba(255, 113, 206, 0.45);
    opacity: 0.85;
  }

  .floor {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(62% + 2px);
    bottom: 0;
    overflow: hidden;
    perspective: 260px;
    perspective-origin: 50% 0;
    /* The grid fades into the haze at the horizon. */
    mask-image: linear-gradient(180deg, transparent 0%, #000 35%);
  }

  .plane {
    position: absolute;
    left: -100%;
    right: -100%;
    top: 0;
    height: 400%;
    overflow: hidden;
    transform-origin: 50% 0;
    transform: rotateX(74deg);
  }

  .grid {
    --cell: 64px;
    position: absolute;
    left: 0;
    right: 0;
    top: calc(var(--cell) * -1);
    bottom: 0;
    background-image:
      linear-gradient(90deg, rgba(255, 113, 206, 0.45) 2px, transparent 2px),
      linear-gradient(180deg, rgba(1, 205, 254, 0.42) 2px, transparent 2px);
    background-size: var(--cell) var(--cell);
    background-position: center top;
    animation: roll 3.6s linear infinite;
    will-change: transform;
  }

  @keyframes roll {
    to {
      transform: translate3d(0, var(--cell), 0);
    }
  }

  @keyframes twinkle {
    0%,
    100% {
      opacity: 0.18;
    }
    50% {
      opacity: 0.6;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .grid,
    .star {
      animation: none;
    }
  }
</style>
