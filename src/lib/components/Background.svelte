<script lang="ts">
  // Decorative night scene: soft moon glow, a few stars and gentle rain. Purely visual (aria-hidden),
  // static when the user prefers reduced motion.
  const DROPS = 46;
  const STARS = 28;

  // Small deterministic PRNG so the scene does not reshuffle between reloads.
  function mulberry32(seed: number) {
    let a = seed;
    return () => {
      a = (a + 0x6d2b79f5) | 0;
      let t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const rand = mulberry32(20261012);
  const drops = Array.from({ length: DROPS }, () => ({
    left: rand() * 100,
    delay: -rand() * 6,
    duration: 2.6 + rand() * 2.4,
    length: 40 + rand() * 60,
    opacity: 0.08 + rand() * 0.16,
  }));
  const stars = Array.from({ length: STARS }, () => ({
    left: rand() * 100,
    top: rand() * 45,
    size: rand() < 0.2 ? 2 : 1,
    delay: -rand() * 8,
  }));
</script>

<div class="scene" aria-hidden="true">
  <div class="moon"></div>
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
  {#each drops as d, i (i)}
    <span
      class="drop"
      style:left="{d.left}%"
      style:height="{d.length}px"
      style:opacity={d.opacity}
      style:animation-delay="{d.delay}s"
      style:animation-duration="{d.duration}s"
    ></span>
  {/each}
  <div class="haze"></div>
</div>

<style>
  .scene {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
  }

  .moon {
    position: absolute;
    top: 7%;
    right: 9%;
    width: 110px;
    height: 110px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 38%, #ffe8c4 0%, #f6c98d 45%, rgba(246, 201, 141, 0) 72%);
    opacity: 0.22;
    filter: blur(1px);
    box-shadow: 0 0 120px 40px rgba(255, 196, 120, 0.08);
  }

  .star {
    position: absolute;
    border-radius: 50%;
    background: #fff3e0;
    opacity: 0.35;
    animation: twinkle 7s ease-in-out infinite;
  }

  .drop {
    position: absolute;
    top: -120px;
    width: 1px;
    background: linear-gradient(180deg, rgba(220, 210, 255, 0) 0%, rgba(220, 210, 255, 0.9) 100%);
    animation-name: fall;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    will-change: transform;
  }

  .haze {
    position: absolute;
    inset: auto 0 0 0;
    height: 40%;
    background: linear-gradient(180deg, rgba(17, 11, 25, 0) 0%, rgba(17, 11, 25, 0.55) 100%);
  }

  @keyframes fall {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(-40px, 115vh, 0);
    }
  }

  @keyframes twinkle {
    0%,
    100% {
      opacity: 0.15;
    }
    50% {
      opacity: 0.55;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .drop {
      display: none;
    }
    .star {
      animation: none;
    }
  }
</style>
