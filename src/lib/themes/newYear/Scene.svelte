<script lang="ts">
  // New Year (ADR-024 collection): a starry midnight over a city skyline and a few gentle fireworks. Each firework is
  // a faint trail that rises and opens into a soft ring of sparks, then drifts down and fades; they take turns on both
  // sides of the timer, never all at once, in muted colours (no white flashes). Reduced motion: two bursts stay still.
  import { mulberry32 } from "$lib/components/backgrounds/random";

  const uid = $props.id();
  const rand = mulberry32(20270101);

  /** Static star field in a 1000 × 600 box (upper sky only), plus a few that twinkle. */
  const stars = Array.from({ length: 64 }, () => ({
    x: rand() * 1000,
    y: rand() * 330,
    r: rand() < 0.18 ? 1.5 : 0.9,
    o: 0.25 + rand() * 0.45,
  }));
  const twinkles = [
    { left: 18, top: 9, delay: -1.2, duration: 6.5 },
    { left: 41, top: 6, delay: -4.1, duration: 8 },
    { left: 63, top: 12, delay: -2.6, duration: 7.2 },
    { left: 92, top: 22, delay: -5.4, duration: 9 },
  ];

  interface Spark {
    x: number;
    y: number;
    r: number;
    o: number;
    tip: boolean;
  }

  /**
   * Sparks of one burst in a 200 × 200 box centred on 0,0: rays of three dots that grow toward a brighter tip (a
   * trail look without strokes), slightly uneven lengths, and an inner ring of small dots.
   */
  function sparks(seed: number): Spark[] {
    const r = mulberry32(seed);
    const out: Spark[] = [];
    const rays = 20;
    for (let i = 0; i < rays; i++) {
      const a = (i / rays) * Math.PI * 2 + r() * 0.12;
      const len = 80 + r() * 16;
      for (const [f, size, o] of [
        [0.6, 1.1, 0.35],
        [0.78, 1.6, 0.6],
        [0.94, 2.3, 1],
      ] as const) {
        out.push({ x: Math.cos(a) * len * f, y: Math.sin(a) * len * f, r: size, o, tip: f > 0.9 });
      }
    }
    for (let i = 0; i < 10; i++) {
      const a = (i / 10) * Math.PI * 2 + 0.3;
      out.push({ x: Math.cos(a) * 42, y: Math.sin(a) * 42, r: 1.6, o: 0.7, tip: false });
    }
    return out;
  }

  /** Where and when each firework goes off (percent of the scene; seconds). Off the timer column, below the header. */
  const bursts = [
    { left: 12, top: 30, size: 1, color: "#f0c870", accent: "#fbe7b0", duration: 11, delay: -1, sparks: sparks(11) },
    { left: 84, top: 24, size: 1.15, color: "#ef9dbd", accent: "#fbd6e4", duration: 13, delay: -5.5, sparks: sparks(23) },
    { left: 27, top: 18, size: 0.8, color: "#8fd8cc", accent: "#d2f1eb", duration: 12, delay: -9, sparks: sparks(37) },
    { left: 71, top: 38, size: 0.75, color: "#b3a6f2", accent: "#e2dcfb", duration: 14, delay: -3, sparks: sparks(41) },
  ];

  /** Skyline in a 2400 × 300 box: two depths of plain blocks, a few setbacks, spires and a dome. */
  function skyline(seed: number, minH: number, maxH: number, tall: number): string {
    const r = mulberry32(seed);
    let x = -20;
    let d = "";
    while (x < 2420) {
      const w = 34 + r() * 76;
      // Taller blocks bunch up in two "downtowns" near the sides, where the timer and cards leave the sky open.
      const centre = (x + w / 2) / 2400;
      const downtown = Math.max(Math.exp(-(((centre - 0.18) / 0.09) ** 2)), Math.exp(-(((centre - 0.82) / 0.1) ** 2)));
      const h = minH + r() * (maxH - minH) + downtown * tall * r();
      const top = 300 - h;
      d += `M${x.toFixed(0)} 300V${top.toFixed(0)}H${(x + w).toFixed(0)}V300Z`;
      const extra = r();
      if (extra < 0.18) {
        // A setback: a narrower block on top.
        const inset = w * (0.18 + r() * 0.14);
        const up = 14 + r() * 26;
        d += `M${(x + inset).toFixed(0)} ${top.toFixed(0)}V${(top - up).toFixed(0)}H${(x + w - inset).toFixed(0)}V${top.toFixed(0)}Z`;
      } else if (extra < 0.28 && h > 120) {
        // A spire.
        const cx = x + w / 2;
        d += `M${(cx - 2).toFixed(0)} ${top.toFixed(0)}V${(top - 28 - r() * 30).toFixed(0)}H${(cx + 2).toFixed(0)}V${top.toFixed(0)}Z`;
      } else if (extra < 0.32 && w > 70) {
        // A dome.
        const rr = w * 0.3;
        const cx = x + w / 2;
        d += `M${(cx - rr).toFixed(0)} ${top.toFixed(0)}A${rr.toFixed(0)} ${rr.toFixed(0)} 0 0 1 ${(cx + rr).toFixed(0)} ${top.toFixed(0)}Z`;
      }
      x += w + (r() < 0.25 ? r() * 14 : 0);
    }
    return d;
  }
  const FAR = skyline(7, 40, 150, 70);
  const NEAR = skyline(31, 20, 95, 120);
</script>

<svg class="stars" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMin slice" focusable="false">
  {#each stars as s, i (i)}
    <circle cx={s.x} cy={s.y} r={s.r} fill="#fff6e2" opacity={s.o} />
  {/each}
</svg>

{#each twinkles as s, i (i)}
  <span
    class="twinkle"
    style:left="{s.left}%"
    style:top="{s.top}%"
    style:animation-delay="{s.delay}s"
    style:animation-duration="{s.duration}s"
  ></span>
{/each}

{#each bursts as b, i (i)}
  <div class="firework" style:left="{b.left}%" style:top="{b.top}%" style:--size={b.size}>
    <span
      class="trail"
      style:--trail={b.accent}
      style:animation-duration="{b.duration}s"
      style:animation-delay="{b.delay}s"
    ></span>
    <span class="burst" style:animation-duration="{b.duration}s" style:animation-delay="{b.delay}s">
      <svg viewBox="-100 -100 200 200" focusable="false">
        <circle r="70" fill="url(#{uid}-glow-{i})" />
        <defs>
          <radialGradient id="{uid}-glow-{i}">
            <stop offset="0%" stop-color={b.color} stop-opacity="0.28" />
            <stop offset="100%" stop-color={b.color} stop-opacity="0" />
          </radialGradient>
        </defs>
        {#each b.sparks as p, j (j)}
          <circle cx={p.x} cy={p.y} r={p.r} fill={p.tip ? b.accent : b.color} opacity={p.o} />
        {/each}
      </svg>
    </span>
  </div>
{/each}

<div class="city-glow"></div>

<svg class="skyline far" viewBox="0 0 2400 300" preserveAspectRatio="xMidYMax slice" focusable="false">
  <path d={FAR} />
</svg>

<svg class="skyline near" viewBox="0 0 2400 300" preserveAspectRatio="xMidYMax slice" focusable="false">
  <defs>
    <!-- Lit windows: a small irregular tile, so the lights look scattered rather than gridded. -->
    <pattern id="{uid}-windows" width="96" height="54" patternUnits="userSpaceOnUse">
      <rect x="8" y="10" width="6" height="8" fill="#f3cf8a" opacity="0.55" />
      <rect x="44" y="10" width="6" height="8" fill="#f3cf8a" opacity="0.3" />
      <rect x="62" y="30" width="6" height="8" fill="#f6dca6" opacity="0.5" />
      <rect x="26" y="38" width="6" height="8" fill="#e9b97a" opacity="0.4" />
      <rect x="80" y="20" width="6" height="8" fill="#f3cf8a" opacity="0.25" />
    </pattern>
  </defs>
  <path d={NEAR} class="block" />
  <path d={NEAR} fill="url(#{uid}-windows)" />
</svg>

<div class="haze"></div>

<style>
  .stars {
    position: absolute;
    inset: 0 0 auto 0;
    width: 100%;
    height: 70%;
  }

  .twinkle {
    position: absolute;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: #fff3d6;
    box-shadow: 0 0 6px 1px rgba(255, 236, 190, 0.5);
    opacity: 0.3;
    animation: twinkle ease-in-out infinite;
  }

  .firework {
    position: absolute;
    width: calc(clamp(130px, 24vh, 260px) * var(--size));
    aspect-ratio: 1;
    transform: translate(-50%, -50%);
  }

  .burst {
    position: absolute;
    inset: 0;
    opacity: 0;
    animation-name: burst;
    animation-timing-function: cubic-bezier(0.2, 0.7, 0.3, 1);
    animation-iteration-count: infinite;
    will-change: transform, opacity;
  }

  .burst svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
    opacity: 0.85;
  }

  /* The rising trail: a short soft line below the burst centre that climbs to it just before it opens. */
  .trail {
    position: absolute;
    left: calc(50% - 1px);
    top: 50%;
    width: 2px;
    height: 34px;
    border-radius: 2px;
    background: linear-gradient(180deg, var(--trail) 0%, transparent 100%);
    opacity: 0;
    animation-name: rise;
    animation-timing-function: cubic-bezier(0.3, 0.6, 0.4, 1);
    animation-iteration-count: infinite;
    will-change: transform, opacity;
  }

  .city-glow {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 42%;
    background:
      radial-gradient(40% 70% at 18% 100%, rgba(239, 170, 100, 0.16) 0%, transparent 70%),
      radial-gradient(40% 70% at 82% 100%, rgba(214, 140, 170, 0.14) 0%, transparent 70%);
  }

  .skyline {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    display: block;
  }

  .far {
    height: clamp(150px, 34vh, 380px);
    fill: #10153a;
    opacity: 0.95;
  }

  .near {
    height: clamp(110px, 24vh, 270px);
  }

  .block {
    fill: #05071a;
  }

  .haze {
    position: absolute;
    inset: auto 0 0 0;
    height: 22%;
    background: linear-gradient(180deg, transparent 0%, color-mix(in srgb, var(--bg-0) 70%, transparent) 100%);
  }

  @keyframes twinkle {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 0.8;
    }
  }

  /* One cycle: the trail climbs (0–12 %), the burst opens (12–30 %), drifts down and fades (30–55 %), rests. */
  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate3d(0, 26vh, 0);
    }
    2% {
      opacity: 0.7;
    }
    11% {
      opacity: 0.55;
      transform: translate3d(0, 0, 0);
    }
    13%,
    100% {
      opacity: 0;
      transform: translate3d(0, 0, 0);
    }
  }

  @keyframes burst {
    0%,
    11% {
      opacity: 0;
      transform: translate3d(0, 0, 0) scale(0.12);
    }
    14% {
      opacity: 0.95;
      transform: translate3d(0, 0, 0) scale(0.45);
    }
    30% {
      opacity: 0.8;
      transform: translate3d(0, 4px, 0) scale(0.96);
    }
    46% {
      opacity: 0.3;
      transform: translate3d(0, 14px, 0) scale(1.05);
    }
    56%,
    100% {
      opacity: 0;
      transform: translate3d(0, 22px, 0) scale(1.08);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .twinkle,
    .trail {
      animation: none;
    }
    .burst {
      animation: none;
      transform: none;
      opacity: 0;
    }
    /* Two fireworks stay open, one on each side. */
    .firework:nth-of-type(1) .burst,
    .firework:nth-of-type(2) .burst {
      opacity: 0.55;
    }
  }
</style>
