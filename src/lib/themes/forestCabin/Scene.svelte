<script lang="ts">
  // Forest cabin (ADR-024 collection): nightfall over a pine forest in three depths, a log cabin on the right with a
  // warm window and a chimney whose smoke drifts up and away, and fireflies wandering low between the trees. The
  // forest, cabin and smoke share one SVG so they stay aligned at any window size. Reduced motion: everything holds
  // still (smoke shows one soft puff, fireflies glow where they are).
  import { mulberry32 } from "$lib/components/backgrounds/random";

  const uid = $props.id();
  const rand = mulberry32(6116);

  const stars = Array.from({ length: 46 }, () => ({
    x: rand() * 1000,
    y: rand() * 300,
    r: rand() < 0.15 ? 1.4 : 0.8,
    o: 0.2 + rand() * 0.45,
  }));

  /** A pine: stacked jagged tiers on a short trunk, `h` tall from `base`, centred on `cx`. */
  function pine(cx: number, base: number, h: number, w: number): string {
    const tiers = 5;
    const trunk = w * 0.05;
    let d = `M${(cx - trunk).toFixed(1)} ${base}V${(base - h * 0.12).toFixed(1)}H${(cx + trunk).toFixed(1)}V${base}Z`;
    for (let i = 0; i < tiers; i++) {
      const top = base - h + i * h * 0.17;
      const bottom = top + h * 0.3;
      const half = (w / 2) * (0.32 + (0.68 * (i + 1)) / tiers);
      // A slightly ragged hem: two notches per side.
      d +=
        `M${cx.toFixed(1)} ${top.toFixed(1)}` +
        `L${(cx + half * 0.55).toFixed(1)} ${(bottom - h * 0.06).toFixed(1)}` +
        `L${(cx + half * 0.48).toFixed(1)} ${(bottom - h * 0.05).toFixed(1)}` +
        `L${(cx + half).toFixed(1)} ${bottom.toFixed(1)}` +
        `L${(cx - half).toFixed(1)} ${bottom.toFixed(1)}` +
        `L${(cx - half * 0.48).toFixed(1)} ${(bottom - h * 0.05).toFixed(1)}` +
        `L${(cx - half * 0.55).toFixed(1)} ${(bottom - h * 0.06).toFixed(1)}Z`;
    }
    return d;
  }

  /** A row of pines across [from, to] in the 2400 × 400 box. */
  function row(
    seed: number,
    from: number,
    to: number,
    base: number,
    minH: number,
    maxH: number,
    gap: [number, number],
  ): string {
    const r = mulberry32(seed);
    let d = "";
    let x = from;
    while (x < to) {
      const h = minH + r() * (maxH - minH);
      d += pine(x, base + r() * 8, h, h * (0.42 + r() * 0.1));
      x += gap[0] + r() * (gap[1] - gap[0]);
    }
    return d;
  }

  /** Mountains far behind the forest. */
  const RIDGE =
    "M0 400V230Q90 196 170 168T330 132Q390 150 450 176T600 158Q690 128 760 150T900 196Q1000 176 1080 150T1240 118Q1320 140 1400 170T1560 176Q1660 140 1740 150T1900 190Q1990 168 2080 140T2240 150Q2320 172 2400 186V400Z";
  const FAR = row(11, -20, 2420, 330, 40, 92, [22, 46]);
  const MID = row(23, -20, 2420, 372, 70, 168, [34, 78]);
  const NEAR = row(37, 380, 780, 430, 190, 330, [70, 120]) + row(41, 1720, 1960, 430, 200, 320, [70, 110]);
  const GROUND = "M0 400V374C300 368 700 380 1100 376S1700 366 2000 372 2300 380 2400 376V400Z";
  /** The rise the cabin stands on (its floor is at y 306). */
  const HILL = "M1180 400C1260 380 1340 330 1440 310 1540 300 1660 300 1740 314 1840 332 1920 372 2000 400Z";

  /** Smoke puffs leaving the chimney top, each on its own delay. */
  const puffs = [0, 2.4, 4.8, 7.2];

  /** Fireflies: low, between the trees and around the cabin (percent of the scene; seconds). */
  const fireflies = Array.from({ length: 11 }, (_, i) => {
    const left = i < 4 ? 2 + rand() * 22 : i < 8 ? 74 + rand() * 22 : 30 + rand() * 40;
    const middle = i >= 8;
    return {
      left,
      top: middle ? 86 + rand() * 8 : 58 + rand() * 34,
      dx: 12 + rand() * 26,
      dy: 8 + rand() * 18,
      duration: 6 + rand() * 8,
      delay: -rand() * 10,
      size: 2.5 + rand() * 1.5,
    };
  });
</script>

<svg class="stars" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMin slice" focusable="false">
  {#each stars as s, i (i)}
    <circle cx={s.x} cy={s.y} r={s.r} fill="#f2f7e8" opacity={s.o} />
  {/each}
</svg>

<svg class="forest" viewBox="0 0 2400 400" preserveAspectRatio="xMidYMax slice" focusable="false">
  <defs>
    <radialGradient id="{uid}-glow">
      <stop offset="0%" stop-color="#ffb862" stop-opacity="0.5" />
      <stop offset="100%" stop-color="#ffb862" stop-opacity="0" />
    </radialGradient>
    <linearGradient id="{uid}-mist" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#9fc4b8" stop-opacity="0" />
      <stop offset="70%" stop-color="#9fc4b8" stop-opacity="0.07" />
      <stop offset="100%" stop-color="#9fc4b8" stop-opacity="0" />
    </linearGradient>
    <radialGradient id="{uid}-smoke">
      <stop offset="0%" stop-color="#c9d6cf" stop-opacity="0.5" />
      <stop offset="100%" stop-color="#c9d6cf" stop-opacity="0" />
    </radialGradient>
  </defs>

  <path d={RIDGE} fill="#10292b" />
  <path d={FAR} fill="#0d2324" />
  <path d={MID} fill="#091817" />

  <path d="M0 330H2400V200H0Z" fill="url(#{uid}-mist)" />
  <path d={HILL} fill="#061010" />

  <!-- The cabin on its rise (drawn at floor y 376, moved up onto the hill). -->
  <g transform="translate(-160 -70)">
    <ellipse cx="1756" cy="382" rx="120" ry="12" fill="#ffb862" opacity="0.12" />
    <rect x="1780" y="244" width="17" height="46" fill="#060d0d" />
    <rect x="1776" y="240" width="25" height="7" fill="#060d0d" />
    <rect x="1662" y="300" width="150" height="76" fill="#0c1716" />
    <g stroke="#13211f" stroke-width="2">
      <line x1="1662" y1="316" x2="1812" y2="316" />
      <line x1="1662" y1="332" x2="1812" y2="332" />
      <line x1="1662" y1="348" x2="1812" y2="348" />
      <line x1="1662" y1="364" x2="1812" y2="364" />
    </g>
    <path d="M1644 306L1737 246L1830 306L1822 310L1737 256L1652 310Z" fill="#060d0d" />
    <path d="M1652 310L1737 256L1822 310Z" fill="#0a1413" />
    <circle class="window-glow" cx="1771" cy="331" r="78" fill="url(#{uid}-glow)" />
    <rect x="1754" y="318" width="34" height="26" rx="1.5" fill="#ffc777" />
    <rect x="1757" y="321" width="28" height="20" fill="#ffdd9e" opacity="0.55" />
    <path d="M1771 318V344M1754 331H1788" stroke="#0c1716" stroke-width="2.4" />
    <rect x="1694" y="330" width="24" height="46" fill="#071010" />
    <rect x="1712" y="352" width="2.5" height="2.5" fill="#ffc777" opacity="0.6" />
    <!-- A little stack of logs by the wall. -->
    <g fill="#071010">
      <circle cx="1826" cy="370" r="6" />
      <circle cx="1838" cy="370" r="6" />
      <circle cx="1832" cy="360" r="6" />
    </g>

    {#each puffs as delay, i (i)}
      <circle
        class="puff"
        class:first={i === 0}
        cx="1788"
        cy="236"
        r="12"
        fill="url(#{uid}-smoke)"
        style:animation-delay="{-delay}s"
      />
    {/each}
  </g>

  <path d={GROUND} fill="#050c0c" />
  <path d={NEAR} fill="#030808" />
</svg>

{#each fireflies as f, i (i)}
  <span
    class="firefly"
    style:left="{f.left}%"
    style:top="{f.top}%"
    style:width="{f.size}px"
    style:height="{f.size}px"
    style:--dx="{f.dx}px"
    style:--dy="{f.dy}px"
    style:animation-duration="{f.duration}s"
    style:animation-delay="{f.delay}s"
  ></span>
{/each}

<style>
  .stars {
    position: absolute;
    inset: 0 0 auto 0;
    width: 100%;
    height: 62%;
  }

  .forest {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: clamp(220px, 60vh, 560px);
    display: block;
  }

  .window-glow {
    animation: flicker 5.5s ease-in-out infinite alternate;
  }

  .puff {
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
    animation: smoke 9.6s ease-out infinite;
    will-change: transform, opacity;
  }

  .firefly {
    position: absolute;
    border-radius: 50%;
    background: #e9f7a0;
    box-shadow:
      0 0 6px 2px rgba(220, 240, 130, 0.55),
      0 0 16px 6px rgba(220, 240, 130, 0.18);
    opacity: 0;
    animation: wander ease-in-out infinite;
    will-change: transform, opacity;
  }

  @keyframes flicker {
    0% {
      opacity: 0.85;
    }
    40% {
      opacity: 1;
    }
    70% {
      opacity: 0.9;
    }
    100% {
      opacity: 0.97;
    }
  }

  @keyframes smoke {
    0% {
      opacity: 0;
      transform: translate(0, 0) scale(0.6);
    }
    12% {
      opacity: 0.55;
    }
    60% {
      opacity: 0.3;
      transform: translate(26px, -88px) scale(1.9);
    }
    100% {
      opacity: 0;
      transform: translate(60px, -160px) scale(2.8);
    }
  }

  /* A small loop with a glow that comes and goes. */
  @keyframes wander {
    0% {
      opacity: 0;
      transform: translate3d(0, 0, 0);
    }
    20% {
      opacity: 0.9;
      transform: translate3d(var(--dx), calc(var(--dy) * -1), 0);
    }
    45% {
      opacity: 0.25;
      transform: translate3d(calc(var(--dx) * 1.6), calc(var(--dy) * 0.4), 0);
    }
    70% {
      opacity: 0.85;
      transform: translate3d(calc(var(--dx) * 0.5), var(--dy), 0);
    }
    100% {
      opacity: 0;
      transform: translate3d(0, 0, 0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .window-glow {
      animation: none;
    }
    .puff {
      animation: none;
      opacity: 0;
    }
    .puff.first {
      opacity: 0.35;
      transform: translate(18px, -60px) scale(1.6);
    }
    .firefly {
      animation: none;
      opacity: 0.7;
    }
  }
</style>
