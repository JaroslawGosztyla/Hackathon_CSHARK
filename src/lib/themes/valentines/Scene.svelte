<script lang="ts">
  // Valentine's (ADR-024 collection): a pink-violet dusk with a thin moon, soft clouds that barely move, rolling hills
  // with a heart-crowned tree and a far village, and a few hearts drifting slowly upward at the sides. Reduced
  // motion: the hearts rest where they are and the clouds stay still.
  import { mulberry32 } from "$lib/components/backgrounds/random";

  const uid = $props.id();
  const rand = mulberry32(214);

  const stars = Array.from({ length: 26 }, () => ({
    x: rand() * 1000,
    y: rand() * 230,
    r: rand() < 0.2 ? 1.3 : 0.8,
    o: 0.2 + rand() * 0.4,
  }));

  const COLORS = ["#ff97ba", "#f7b4cc", "#ff8fa8", "#dca8e6", "#ffc2d4"] as const;

  /** Hearts keep to the sides (the timer and the cards sit in the middle); one small one crosses the middle. */
  const LANES = [4, 10, 17, 24, 47, 76, 83, 90, 96] as const;
  const hearts = LANES.map((left, i) => {
    const duration = 22 + rand() * 14;
    const middle = left > 30 && left < 70;
    return {
      left: left + (rand() - 0.5) * 3,
      size: middle ? 10 : 12 + rand() * 18,
      duration,
      delay: -rand() * duration,
      sway: 10 + rand() * 18,
      tilt: (rand() < 0.5 ? -1 : 1) * (6 + rand() * 8),
      color: COLORS[i % COLORS.length],
      opacity: middle ? 0.28 : 0.38 + rand() * 0.24,
      rest: 22 + rand() * 56,
    };
  });

  const clouds = [
    { left: 6, top: 20, width: 260, opacity: 0.42, duration: 70, delay: -12 },
    { left: 64, top: 13, width: 340, opacity: 0.36, duration: 90, delay: -40 },
    { left: 30, top: 42, width: 220, opacity: 0.26, duration: 80, delay: -25 },
  ];

  /** A heart in a 24 × 24 box. */
  const HEART =
    "M12 20.6C12 20.6 3 15.2 3 9.1 3 6.1 5.2 4 7.8 4c1.9 0 3.4 1.1 4.2 2.6C12.8 5.1 14.3 4 16.2 4 18.8 4 21 6.1 21 9.1c0 6.1-9 11.5-9 11.5z";

  /** Far village lights on the back hill (3200 × 300 box). */
  const lights = Array.from({ length: 14 }, (_, i) => ({
    x: 560 + i * 34 + rand() * 18 + (i > 6 ? 1500 : 0),
    y: 0,
    o: 0.35 + rand() * 0.4,
  })).map((l) => ({ ...l, y: farHillY(l.x) + 6 + rand() * 10 }));

  /** The back hill's top edge (matches the path below closely enough for the lights to sit on it). */
  function farHillY(x: number): number {
    return 150 + Math.sin((x / 3200) * Math.PI * 5 + 0.6) * 22 + Math.sin((x / 3200) * Math.PI * 11) * 8;
  }

  const FAR_HILL = (() => {
    let d = "M0 300";
    for (let x = 0; x <= 3200; x += 40) d += `L${x} ${farHillY(x).toFixed(1)}`;
    return d + "L3200 300Z";
  })();
</script>

<svg class="stars" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMin slice" focusable="false">
  {#each stars as s, i (i)}
    <circle cx={s.x} cy={s.y} r={s.r} fill="#ffeef4" opacity={s.o} />
  {/each}
</svg>

<!-- A thin waxing moon, low and soft. -->
<svg class="moon" viewBox="0 0 100 100" focusable="false">
  <defs>
    <radialGradient id="{uid}-halo">
      <stop offset="30%" stop-color="#ffd7e2" stop-opacity="0.22" />
      <stop offset="100%" stop-color="#ffd7e2" stop-opacity="0" />
    </radialGradient>
  </defs>
  <circle cx="44" cy="50" r="50" fill="url(#{uid}-halo)" />
  <path d="M60 14a36 36 0 1 0 0 72 44 44 0 0 1 0-72z" fill="#ffe6ee" opacity="0.85" />
</svg>

{#each clouds as c, i (i)}
  <svg
    class="cloud"
    viewBox="0 0 200 60"
    style:left="{c.left}%"
    style:top="{c.top}%"
    style:width="{c.width}px"
    style:opacity={c.opacity}
    style:animation-duration="{c.duration}s"
    style:animation-delay="{c.delay}s"
    focusable="false"
  >
    <defs>
      <linearGradient id="{uid}-cloud-{i}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#6a3062" />
        <stop offset="100%" stop-color="#b4618a" />
      </linearGradient>
    </defs>
    <path
      d="M14 52c-9 0-12-10-4-14 0-10 12-15 20-9 4-12 22-15 30-5 6-10 26-12 32 2 9-6 24-2 24 9 9-1 15 9 8 14-4 3-6 3-10 3z"
      fill="url(#{uid}-cloud-{i})"
    />
    <path d="M30 52c-6 0-6-6 0-7 3-6 12-7 16-2 6-5 18-4 20 3 6-2 12 2 10 6z" fill="#d77fa0" opacity="0.35" />
  </svg>
{/each}

<svg class="hills" viewBox="0 0 3200 300" preserveAspectRatio="xMidYMax slice" focusable="false">
  <path d={FAR_HILL} fill="#3b1840" opacity="0.9" />
  {#each lights as l, i (i)}
    <rect x={l.x} y={l.y} width="4" height="4" rx="1" fill="#ffd2a1" opacity={l.o} />
  {/each}
  <path
    d="M0 300V236C260 214 520 222 760 238S1240 262 1520 246 1900 214 2080 186C2160 174 2240 172 2320 182 2560 212 2900 236 3200 226V300Z"
    fill="#230f2b"
  />
  <!-- A tree with a heart-shaped crown on the right hilltop, and a small round one beside it. -->
  <g fill="#230f2b">
    <rect x="2178" y="118" width="9" height="62" rx="3" />
    <path d="M2182 126c0 0-48-28-48-62 0-16 12-28 26-28 10 0 18 6 22 14 4-8 12-14 22-14 14 0 26 12 26 28 0 34-48 62-48 62z" />
    <rect x="2262" y="150" width="6" height="34" rx="2" />
    <circle cx="2265" cy="140" r="20" />
  </g>
</svg>

{#each hearts as h, i (i)}
  <span
    class="heart"
    style:left="{h.left}%"
    style:--o={h.opacity}
    style:--sway="{h.sway}px"
    style:--tilt="{h.tilt}deg"
    style:--rest="{h.rest}%"
    style:animation-duration="{h.duration}s"
    style:animation-delay="{h.delay}s"
  >
    <svg viewBox="0 0 24 24" width={h.size} height={h.size} focusable="false">
      <path d={HEART} fill={h.color} />
    </svg>
  </span>
{/each}

<div class="haze"></div>

<style>
  .stars {
    position: absolute;
    inset: 0 0 auto 0;
    width: 100%;
    height: 60%;
  }

  .moon {
    position: absolute;
    left: 13%;
    top: 15%;
    width: clamp(54px, 9vh, 96px);
    height: auto;
    overflow: visible;
    transform: rotate(-24deg);
  }

  .cloud {
    position: absolute;
    height: auto;
    animation: drift ease-in-out infinite alternate;
    will-change: transform;
  }

  .hills {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: clamp(150px, 30vh, 340px);
    display: block;
  }

  .heart {
    position: absolute;
    top: 100%;
    opacity: 0;
    animation: rise linear infinite;
    will-change: transform, opacity;
  }

  .heart svg {
    display: block;
  }

  .haze {
    position: absolute;
    inset: auto 0 0 0;
    height: 18%;
    background: linear-gradient(180deg, transparent 0%, color-mix(in srgb, var(--bg-0) 55%, transparent) 100%);
  }

  @keyframes drift {
    from {
      transform: translate3d(-30px, 0, 0);
    }
    to {
      transform: translate3d(30px, 0, 0);
    }
  }

  /* Up the whole window with a gentle side-to-side sway and tilt; fades in at the bottom and out near the top. */
  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate3d(0, 20px, 0) rotate(calc(var(--tilt) * -1));
    }
    10% {
      opacity: var(--o);
    }
    25% {
      transform: translate3d(var(--sway), -28vh, 0) rotate(var(--tilt));
    }
    50% {
      transform: translate3d(calc(var(--sway) * -1), -55vh, 0) rotate(calc(var(--tilt) * -1));
    }
    75% {
      opacity: var(--o);
      transform: translate3d(var(--sway), -82vh, 0) rotate(var(--tilt));
    }
    92%,
    100% {
      opacity: 0;
      transform: translate3d(0, -100vh, 0) rotate(0deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .cloud {
      animation: none;
    }
    .heart {
      top: var(--rest);
      opacity: var(--o);
      animation: none;
      transform: rotate(var(--tilt));
    }
  }
</style>
