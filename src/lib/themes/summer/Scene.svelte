<script lang="ts">
  // Summer (ADR-024 collection): a low sun sinking into the sea at the right, a glitter path on the water, slow wave
  // crests, two palms that sway a little on dark dunes, a few thin lit clouds and two gulls gliding by. The sky
  // gradient (horizon at 66 %) is the theme's `--app-bg`. Reduced motion: everything holds still.
  import { PALM_FRONDS, PALM_TRUNK } from "./palm";

  const uid = $props.id();

  /** Glitter on the water under the sun: short bars, wider toward the viewer; two sets that shimmer in turn. */
  const GLINT_A = [
    { y: 3, w: 46, o: 0.8 },
    { y: 12, w: 70, o: 0.55 },
    { y: 24, w: 54, o: 0.6 },
    { y: 40, w: 96, o: 0.4 },
    { y: 60, w: 80, o: 0.35 },
  ];
  const GLINT_B = [
    { y: 7, w: 60, o: 0.6 },
    { y: 17, w: 40, o: 0.7 },
    { y: 31, w: 84, o: 0.45 },
    { y: 50, w: 64, o: 0.4 },
    { y: 72, w: 110, o: 0.3 },
  ];

  /**
   * Wave crests on the sea: short arcs scattered in a repeating tile, finer and fainter near the horizon. Each band
   * slides two tile widths per loop, so the loop is seamless.
   */
  const WAVES = [
    { top: 67.5, height: 16, period: 110, amp: 1.6, stroke: 1.1, opacity: 0.4, duration: 30, color: "#ffc59a" },
    { top: 72, height: 26, period: 180, amp: 2.6, stroke: 1.5, opacity: 0.32, duration: 40, color: "#e2b0d8" },
    { top: 79, height: 40, period: 280, amp: 4, stroke: 2, opacity: 0.26, duration: 52, color: "#ffb48a" },
  ].map((w) => ({ ...w, d: wavelets(w.period, w.height, w.amp) }));

  /** Four arcs per tile: [x, y, width] as fractions of the tile. */
  function wavelets(p: number, h: number, amp: number): string {
    const arcs = [
      [0.04, 0.32, 0.24],
      [0.42, 0.72, 0.2],
      [0.7, 0.22, 0.15],
      [0.86, 0.86, 0.1],
    ] as const;
    return arcs
      .map(([x, y, w]) => {
        const start = `M${(x * p).toFixed(1)} ${(y * h).toFixed(1)}`;
        return `${start}q${((w * p) / 2).toFixed(1)} ${-amp} ${(w * p).toFixed(1)} 0`;
      })
      .join("");
  }

  const gulls = [
    { top: 30, size: 22, duration: 70, delay: -8 },
    { top: 36, size: 15, duration: 86, delay: -40 },
  ];
</script>

<div class="sky">
  <div class="sun"></div>
  <svg class="clouds" viewBox="0 0 1000 200" preserveAspectRatio="none" focusable="false">
    <path d="M40 120h260c18 0 18 8 0 8H70c-20 0-20-8-30-8z" fill="#f29a72" opacity="0.28" />
    <path d="M120 150h330c16 0 16 7 0 7H140c-18 0-18-7-20-7z" fill="#f5a98a" opacity="0.22" />
    <path d="M610 96h220c14 0 14 6 0 6H630c-14 0-14-6-20-6z" fill="#d98aa2" opacity="0.22" />
    <path d="M700 168h240c16 0 16 7 0 7H720c-16 0-16-7-20-7z" fill="#ffb48a" opacity="0.3" />
  </svg>
</div>

{#each gulls as g, i (i)}
  <span class="gull" style:top="{g.top}%" style:animation-duration="{g.duration}s" style:animation-delay="{g.delay}s">
    <svg viewBox="0 0 24 10" width={g.size} height={g.size * 0.42} focusable="false">
      <path d="M1 7c4-5 7-5 11 0 4-5 7-5 11 0" fill="none" stroke="#2a1a33" stroke-width="1.6" stroke-linecap="round" />
    </svg>
  </span>
{/each}

<div class="glint">
  <div class="set a">
    {#each GLINT_A as g, i (i)}
      <span style:top="{g.y}%" style:width="{g.w}px" style:opacity={g.o}></span>
    {/each}
  </div>
  <div class="set b">
    {#each GLINT_B as g, i (i)}
      <span style:top="{g.y}%" style:width="{g.w}px" style:opacity={g.o}></span>
    {/each}
  </div>
</div>

{#each WAVES as w, i (i)}
  <svg
    class="waves"
    style:top="{w.top}%"
    style:height="{w.height}px"
    style:opacity={w.opacity}
    style:--period="{w.period * 2}px"
    style:animation-duration="{w.duration}s"
    focusable="false"
  >
    <defs>
      <pattern id="{uid}-wave-{i}" width={w.period} height={w.height} patternUnits="userSpaceOnUse">
        <path
          d={w.d}
          fill="none"
          stroke={w.color}
          stroke-width={w.stroke}
          stroke-linecap="round"
        />
      </pattern>
    </defs>
    <rect width="100%" height="100%" fill="url(#{uid}-wave-{i})" />
  </svg>
{/each}

<svg class="dunes" viewBox="0 0 1600 200" preserveAspectRatio="none" focusable="false">
  <path d="M0 200V96C120 70 260 90 400 132 480 156 520 180 560 200Z" fill="#1d1226" />
  <path d="M1600 200V110C1500 92 1380 104 1280 140 1220 162 1180 184 1150 200Z" fill="#1d1226" />
</svg>

<div class="palm left">
  <svg viewBox="0 0 240 420" focusable="false">
    <path d={PALM_TRUNK} />
    <path d={PALM_FRONDS} />
    <circle cx="146" cy="110" r="6" />
    <circle cx="157" cy="112" r="5.5" />
  </svg>
</div>
<div class="palm right">
  <svg viewBox="0 0 240 420" focusable="false">
    <path d={PALM_TRUNK} />
    <path d={PALM_FRONDS} />
    <circle cx="146" cy="110" r="6" />
  </svg>
</div>

<style>
  /* Everything above the horizon; the sun sinks below its bottom edge. */
  .sky {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 66%;
    overflow: hidden;
  }

  /* Right of the centred timer, so the digits never sit on the bright disc. */
  .sun {
    position: absolute;
    left: 76%;
    bottom: 0;
    width: clamp(150px, 28vh, 320px);
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, 42%);
    background: radial-gradient(circle at 50% 45%, #fff1c4 0%, #ffd27f 38%, #ff9f5a 68%, rgba(255, 140, 80, 0) 71%);
    box-shadow: 0 0 120px 40px rgba(255, 150, 90, 0.22);
  }

  .clouds {
    position: absolute;
    left: 0;
    width: 100%;
    bottom: 4%;
    height: 34%;
  }

  .gull {
    position: absolute;
    left: 0;
    opacity: 0;
    animation: glide linear infinite;
    will-change: transform, opacity;
  }

  .gull svg {
    display: block;
  }

  /* The glitter path: a narrow column under the sun, from the horizon down. */
  .glint {
    position: absolute;
    left: 76%;
    top: 66%;
    width: 0;
    height: 26%;
  }

  .set {
    position: absolute;
    inset: 0;
    animation: shimmer 5s ease-in-out infinite;
  }

  .set.b {
    animation-delay: -2.5s;
  }

  .set span {
    position: absolute;
    left: 0;
    height: 2px;
    border-radius: 2px;
    background: #ffd9a6;
    transform: translateX(-50%);
  }

  .waves {
    position: absolute;
    left: 0;
    width: calc(100% + var(--period));
    animation: slide linear infinite;
    will-change: transform;
  }

  .dunes {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: 22%;
  }

  .palm {
    position: absolute;
    bottom: 3%;
    transform-origin: 25% 100%;
    animation: sway ease-in-out infinite alternate;
    will-change: transform;
  }

  .palm svg {
    display: block;
    width: 100%;
    height: 100%;
    fill: #160d1f;
  }

  .palm.left {
    left: -3%;
    height: clamp(260px, 64vh, 680px);
    aspect-ratio: 240 / 420;
    animation-duration: 8s;
  }

  /* Mirrored and smaller, leaning in from the right. */
  .palm.right {
    right: 7%;
    height: clamp(200px, 46vh, 500px);
    aspect-ratio: 240 / 420;
    transform-origin: 75% 100%;
    animation-name: sway-mirrored;
    animation-duration: 9.5s;
    animation-delay: -3s;
  }

  @keyframes sway {
    from {
      transform: rotate(-1.2deg);
    }
    to {
      transform: rotate(1.4deg);
    }
  }

  @keyframes sway-mirrored {
    from {
      transform: scaleX(-1) rotate(-1.2deg);
    }
    to {
      transform: scaleX(-1) rotate(1.4deg);
    }
  }

  @keyframes shimmer {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 0.9;
    }
  }

  @keyframes slide {
    from {
      transform: translate3d(calc(var(--period) * -1), 0, 0);
    }
    to {
      transform: translate3d(0, 0, 0);
    }
  }

  @keyframes glide {
    0% {
      opacity: 0;
      transform: translate3d(14vw, 0, 0);
    }
    8% {
      opacity: 0.75;
    }
    50% {
      transform: translate3d(42vw, -14px, 0);
    }
    92% {
      opacity: 0.75;
    }
    100% {
      opacity: 0;
      transform: translate3d(70vw, 6px, 0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .waves,
    .set {
      animation: none;
    }
    .set.b {
      opacity: 0.5;
    }
    .palm.left {
      animation: none;
    }
    .palm.right {
      animation: none;
      transform: scaleX(-1);
    }
    .gull {
      animation: none;
      opacity: 0.7;
      transform: translate3d(18vw, 0, 0);
    }
    .gull + .gull {
      transform: translate3d(23vw, 18px, 0);
    }
  }
</style>
