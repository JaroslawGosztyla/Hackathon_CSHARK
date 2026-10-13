<script lang="ts">
  // Sakura night (ADR-024 collection): a big pale moon at the upper right with a wisp of cloud passing, cherry
  // branches in blossom reaching in from the top-left corner and the right edge, petals drifting slowly across on a
  // diagonal, and soft hills in mist below. Reduced motion: the petals rest mid-air, the branches and cloud hold still.
  import { mulberry32 } from "$lib/components/backgrounds/random";
  import Branch from "./Branch.svelte";
  import { PETAL } from "./branch";

  const uid = $props.id();
  const rand = mulberry32(3104);

  const stars = Array.from({ length: 40 }, () => ({
    x: rand() * 1000,
    y: rand() * 320,
    r: rand() < 0.15 ? 1.3 : 0.8,
    o: 0.2 + rand() * 0.4,
  }));

  const COLORS = ["#f9c6dc", "#f3a9c9", "#ffd9e8", "#eab0d6"] as const;

  /** Petals start above the window, spread across it, and fall on a diagonal toward the lower right. */
  const petals = Array.from({ length: 14 }, (_, i) => {
    const duration = 16 + rand() * 14;
    return {
      left: -12 + (i / 14) * 92 + rand() * 6,
      size: 9 + rand() * 7,
      duration,
      delay: -rand() * duration,
      drift: 22 + rand() * 20,
      spin: (rand() < 0.5 ? -1 : 1) * (360 + rand() * 360),
      color: COLORS[i % COLORS.length],
      opacity: 0.55 + rand() * 0.3,
      restTop: 14 + rand() * 70,
      restLeft: rand() * 10,
    };
  });

  /** Hills in a 2400 × 300 box. */
  const FAR_HILLS = "M0 300V200C200 170 380 150 560 170S900 220 1100 196 1460 140 1700 160 2100 210 2400 186V300Z";
  const NEAR_HILLS = "M0 300V250C260 236 480 214 700 230S1080 270 1320 256 1760 220 1980 236 2280 262 2400 254V300Z";
</script>

<svg class="stars" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMin slice" focusable="false">
  {#each stars as s, i (i)}
    <circle cx={s.x} cy={s.y} r={s.r} fill="#f6f0ff" opacity={s.o} />
  {/each}
</svg>

<svg class="moon" viewBox="0 0 200 200" focusable="false">
  <defs>
    <radialGradient id="{uid}-halo">
      <stop offset="35%" stop-color="#e8dcff" stop-opacity="0.22" />
      <stop offset="100%" stop-color="#e8dcff" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{uid}-face" cx="42%" cy="40%" r="62%">
      <stop offset="0%" stop-color="#fffaf4" />
      <stop offset="70%" stop-color="#f1e8f6" />
      <stop offset="100%" stop-color="#d9cdea" />
    </radialGradient>
  </defs>
  <circle cx="100" cy="100" r="100" fill="url(#{uid}-halo)" />
  <circle cx="100" cy="100" r="46" fill="url(#{uid}-face)" />
  <g fill="#cbbede" opacity="0.28">
    <ellipse cx="84" cy="86" rx="12" ry="9" />
    <ellipse cx="110" cy="80" rx="6" ry="4" />
    <ellipse cx="116" cy="108" rx="9" ry="7" />
    <ellipse cx="92" cy="118" rx="5" ry="3.5" />
    <circle cx="100" cy="98" r="2.4" />
    <circle cx="126" cy="92" r="1.8" />
  </g>
</svg>

<svg class="wisp" viewBox="0 0 300 60" focusable="false">
  <defs>
    <linearGradient id="{uid}-wisp" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#3a2f62" stop-opacity="0" />
      <stop offset="30%" stop-color="#3a2f62" stop-opacity="0.85" />
      <stop offset="70%" stop-color="#3a2f62" stop-opacity="0.85" />
      <stop offset="100%" stop-color="#3a2f62" stop-opacity="0" />
    </linearGradient>
  </defs>
  <path
    d="M0 38C30 30 60 24 96 25 112 14 140 12 158 20 182 12 214 14 230 26 254 24 280 28 300 34 270 42 230 44 190 42 150 46 100 46 60 44 30 44 10 42 0 38Z"
    fill="url(#{uid}-wisp)"
  />
</svg>

<svg class="hills" viewBox="0 0 2400 300" preserveAspectRatio="xMidYMax slice" focusable="false">
  <defs>
    <linearGradient id="{uid}-mist" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#b9a8e6" stop-opacity="0" />
      <stop offset="60%" stop-color="#b9a8e6" stop-opacity="0.1" />
      <stop offset="100%" stop-color="#b9a8e6" stop-opacity="0" />
    </linearGradient>
  </defs>
  <path d={FAR_HILLS} fill="#1d1738" />
  <path d="M0 260H2400V140H0Z" fill="url(#{uid}-mist)" />
  <path d={NEAR_HILLS} fill="#110d24" />
</svg>

<!-- Cherry branches: the same drawing twice, from the top-left corner and (mirrored, smaller) from the right edge. -->
<div class="branch corner"><Branch /></div>
<div class="branch edge"><Branch duration={13} delay={-5} /></div>

{#each petals as p, i (i)}
  <span
    class="petal"
    style:left="{p.left}%"
    style:--drift="{p.drift}vw"
    style:--spin="{p.spin}deg"
    style:--o={p.opacity}
    style:--rest-top="{p.restTop}%"
    style:--rest-left="{p.restLeft}vw"
    style:animation-duration="{p.duration}s"
    style:animation-delay="{p.delay}s"
  >
    <svg viewBox="0 0 20 20" width={p.size} height={p.size} focusable="false">
      <path d={PETAL} fill={p.color} />
    </svg>
  </span>
{/each}

<style>
  .stars {
    position: absolute;
    inset: 0 0 auto 0;
    width: 100%;
    height: 66%;
  }

  .moon {
    position: absolute;
    left: 82%;
    top: 21%;
    width: clamp(180px, 34vh, 400px);
    height: auto;
    transform: translate(-50%, -50%);
  }

  .wisp {
    position: absolute;
    left: 70%;
    top: 22%;
    width: clamp(160px, 26vh, 300px);
    height: auto;
    opacity: 0.75;
    animation: drift 46s ease-in-out infinite alternate;
    will-change: transform;
  }

  .hills {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: clamp(150px, 32vh, 360px);
    display: block;
  }

  .branch {
    position: absolute;
  }

  .corner {
    left: -2%;
    top: clamp(20px, 5vh, 56px);
    width: clamp(260px, 30vw, 560px);
  }

  /* Smaller, mirrored, reaching in from the right edge below the moon. */
  .edge {
    right: 3%;
    top: 44%;
    width: clamp(180px, 20vw, 380px);
    transform: scaleX(-1);
  }

  .petal {
    position: absolute;
    top: -4%;
    opacity: 0;
    animation: fall linear infinite;
    will-change: transform, opacity;
  }

  .petal svg {
    display: block;
  }

  @keyframes drift {
    from {
      transform: translate3d(-40px, 0, 0);
    }
    to {
      transform: translate3d(50px, 4px, 0);
    }
  }

  /* Down the window and to the right, turning over as it goes. */
  @keyframes fall {
    0% {
      opacity: 0;
      transform: translate3d(0, 0, 0) rotate(0deg);
    }
    8% {
      opacity: var(--o);
    }
    35% {
      transform: translate3d(calc(var(--drift) * 0.3), 38vh, 0) rotate(calc(var(--spin) * 0.35));
    }
    65% {
      transform: translate3d(calc(var(--drift) * 0.72), 70vh, 0) rotate(calc(var(--spin) * 0.65));
    }
    90% {
      opacity: var(--o);
    }
    100% {
      opacity: 0;
      transform: translate3d(var(--drift), 106vh, 0) rotate(var(--spin));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wisp {
      animation: none;
    }
    .petal {
      top: var(--rest-top);
      opacity: var(--o);
      animation: none;
      transform: translateX(var(--rest-left)) rotate(var(--spin));
    }
  }
</style>
