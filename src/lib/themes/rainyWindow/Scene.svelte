<script lang="ts">
  // Rainy window (ADR-024 collection): looking out through wet glass at a night city. Soft out-of-focus city lights
  // (bokeh) glow and breathe slowly, rain falls in two faint layers behind the glass, beads of water sit on it and a
  // few drops slide down now and then; the window frame and a small plant on the sill frame the view. Reduced
  // motion: the rain layers go, the lights and drops hold still.
  import { mulberry32 } from "$lib/components/backgrounds/random";

  const uid = $props.id();
  const rand = mulberry32(1107);

  const LIGHTS = ["#f0a86e", "#ffd9a0", "#e88fb4", "#7fcfd0", "#b9a2f0", "#f6c27e"] as const;

  /**
   * Bokeh discs: the city below the window, mostly low and at the sides. Nothing bright sits in the middle column
   * above the cards (the timer, the Start button and the hint), and the ones low in the middle stay faint.
   */
  const bokeh = Array.from({ length: 20 }, (_, i) => {
    const color = LIGHTS[Math.floor(rand() * LIGHTS.length)] ?? LIGHTS[0];
    const pulse = i % 3 === 1;
    const duration = 8 + rand() * 8;
    const delay = -rand() * 12;
    if (i < 7) {
      // High, at the edges.
      const left = rand() < 0.5 ? 1 + rand() * 20 : 80 + rand() * 18;
      return {
        left,
        top: 16 + rand() * 40,
        size: 34 + rand() * 70,
        color,
        opacity: 0.15 + rand() * 0.12,
        pulse,
        duration,
        delay,
      };
    }
    const left = rand() * 100;
    const middle = left > 28 && left < 72;
    return {
      left,
      top: middle ? 78 + rand() * 18 : 58 + rand() * 38,
      size: 40 + rand() * (middle ? 70 : 120),
      color,
      opacity: middle ? 0.1 + rand() * 0.06 : 0.22 + rand() * 0.16,
      pulse,
      duration,
      delay,
    };
  });

  /** Rain streak tile (120 × 220): thin lines of different lengths. */
  const streaks = (seed: number) => {
    const r = mulberry32(seed);
    return Array.from({ length: 7 }, () => ({ x: r() * 120, y: r() * 220, len: 14 + r() * 30, o: 0.35 + r() * 0.5 }));
  };
  const RAIN_FAR = streaks(3);
  const RAIN_NEAR = streaks(9);

  /** Beads of water on the glass (1000 × 600 box), denser toward the bottom. */
  const beads = Array.from({ length: 70 }, () => {
    const y = 600 * Math.sqrt(rand());
    return { x: rand() * 1000, y, r: 0.8 + rand() * 2.2, o: 0.25 + rand() * 0.35 };
  });

  /** Drops that run down the glass with pauses (percent of the scene; seconds). */
  const drops = [
    { left: 8, top: 18, size: 9, duration: 19, delay: -3 },
    { left: 21, top: 30, size: 7, duration: 24, delay: -14 },
    { left: 71, top: 22, size: 10, duration: 21, delay: -8 },
    { left: 86, top: 12, size: 8, duration: 26, delay: -19 },
    { left: 93, top: 40, size: 6, duration: 17, delay: -5 },
  ];

  /** Far city blocks (2400 × 300 box): a soft, low-contrast skyline seen through the rain. */
  const CITY = (() => {
    const r = mulberry32(77);
    let x = -10;
    let d = "";
    while (x < 2410) {
      const w = 50 + r() * 110;
      const h = 60 + r() * 170;
      d += `M${x.toFixed(0)} 300V${(300 - h).toFixed(0)}H${(x + w).toFixed(0)}V300Z`;
      x += w + 4 + r() * 20;
    }
    return d;
  })();
</script>

<svg class="city" viewBox="0 0 2400 300" preserveAspectRatio="xMidYMax slice" focusable="false">
  <path d={CITY} fill="#1a2335" />
</svg>

{#each bokeh as b, i (i)}
  <span
    class="bokeh"
    class:pulse={b.pulse}
    style:left="{b.left}%"
    style:top="{b.top}%"
    style:width="{b.size}px"
    style:--c={b.color}
    style:--o={b.opacity}
    style:animation-duration="{b.duration}s"
    style:animation-delay="{b.delay}s"
  ></span>
{/each}

<div class="rain far">
  <svg class="sheet" style:animation-duration="1.15s" focusable="false">
    <defs>
      <pattern id="{uid}-rain-far" width="120" height="220" patternUnits="userSpaceOnUse">
        {#each RAIN_FAR as s, i (i)}
          <line x1={s.x} y1={s.y} x2={s.x} y2={s.y + s.len} stroke="#d4e2ff" stroke-width="1" opacity={s.o} />
          <line
            x1={s.x}
            y1={s.y - 220}
            x2={s.x}
            y2={s.y - 220 + s.len}
            stroke="#d4e2ff"
            stroke-width="1"
            opacity={s.o}
          />
        {/each}
      </pattern>
    </defs>
    <rect width="100%" height="100%" fill="url(#{uid}-rain-far)" />
  </svg>
</div>
<div class="rain near">
  <svg class="sheet" style:animation-duration="0.9s" focusable="false">
    <defs>
      <pattern id="{uid}-rain-near" width="170" height="220" patternUnits="userSpaceOnUse">
        {#each RAIN_NEAR as s, i (i)}
          <line
            x1={s.x * 1.4}
            y1={s.y}
            x2={s.x * 1.4}
            y2={s.y + s.len * 1.3}
            stroke="#e2ebff"
            stroke-width="1.2"
            opacity={s.o}
          />
          <line
            x1={s.x * 1.4}
            y1={s.y - 220}
            x2={s.x * 1.4}
            y2={s.y - 220 + s.len * 1.3}
            stroke="#e2ebff"
            stroke-width="1.2"
            opacity={s.o}
          />
        {/each}
      </pattern>
    </defs>
    <rect width="100%" height="100%" fill="url(#{uid}-rain-near)" />
  </svg>
</div>

<!-- The glass: a faint fog low down, and beads of water. -->
<div class="fog"></div>
<svg class="beads" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMid slice" focusable="false">
  {#each beads as b, i (i)}
    <circle cx={b.x} cy={b.y} r={b.r} fill="#cfdcf5" opacity={b.o * 0.5} />
    <circle cx={b.x - b.r * 0.3} cy={b.y - b.r * 0.35} r={b.r * 0.35} fill="#ffffff" opacity={b.o} />
  {/each}
</svg>

{#each drops as d, i (i)}
  <span
    class="drop"
    style:left="{d.left}%"
    style:top="{d.top}%"
    style:--size="{d.size}px"
    style:animation-duration="{d.duration}s"
    style:animation-delay="{d.delay}s"
  >
    <span class="trail"></span>
    <span class="bead"></span>
  </span>
{/each}

<!-- Window frame (left edge and sill) and a small plant on the sill. -->
<div class="frame-left"></div>
<div class="sill"></div>
<svg class="plant" viewBox="0 0 120 160" focusable="false">
  <g fill="#070a12">
    <path d="M38 118h44l-6 40H44z" />
    <rect x="34" y="112" width="52" height="9" rx="3" />
    <path d="M60 114C58 90 46 70 22 58c18 2 34 14 40 34 2-26 10-52 30-70-10 22-14 48-14 74 6-16 18-26 34-30-14 10-24 26-28 48z" />
    <path d="M58 112C52 96 38 88 18 88c16-6 34 0 44 16z" />
  </g>
</svg>

<style>
  .city {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: clamp(160px, 36vh, 420px);
    opacity: 0.55;
  }

  /* A soft disc with a slightly brighter rim, like an out-of-focus light. */
  .bokeh {
    position: absolute;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(
      circle,
      color-mix(in srgb, var(--c) 62%, transparent) 0%,
      color-mix(in srgb, var(--c) 52%, transparent) 54%,
      color-mix(in srgb, var(--c) 66%, transparent) 64%,
      transparent 72%
    );
    opacity: var(--o);
  }

  .bokeh.pulse {
    animation: breathe ease-in-out infinite alternate;
  }

  /* Slanted a little, wider than the window so the tilt never shows an edge. */
  .rain {
    position: absolute;
    left: -10%;
    width: 120%;
    top: 0;
    bottom: 0;
    overflow: hidden;
    transform: rotate(7deg);
  }

  .rain.far {
    opacity: 0.16;
  }

  .rain.near {
    opacity: 0.2;
  }

  .sheet {
    position: absolute;
    left: 0;
    top: -220px;
    width: 100%;
    height: calc(100% + 440px);
    animation: fall linear infinite;
    will-change: transform;
  }

  .fog {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, transparent 55%, rgba(170, 190, 225, 0.07) 100%),
      radial-gradient(70% 40% at 50% 100%, rgba(190, 205, 235, 0.06) 0%, transparent 70%);
  }

  .beads {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .drop {
    position: absolute;
    width: var(--size);
    animation: run ease-in infinite;
    opacity: 0;
    will-change: transform, opacity;
  }

  .trail {
    position: absolute;
    left: 50%;
    bottom: 50%;
    width: 2px;
    height: calc(var(--size) * 7);
    transform: translateX(-50%);
    border-radius: 2px;
    background: linear-gradient(180deg, transparent 0%, rgba(214, 228, 255, 0.28) 100%);
  }

  .bead {
    position: relative;
    display: block;
    width: var(--size);
    height: calc(var(--size) * 1.15);
    border-radius: 50% 50% 50% 50% / 45% 45% 55% 55%;
    background: radial-gradient(
      circle at 35% 65%,
      rgba(255, 255, 255, 0.75) 0 18%,
      rgba(190, 210, 245, 0.35) 40%,
      rgba(150, 175, 220, 0.25) 70%
    );
    box-shadow: inset 0 -1px 2px rgba(10, 14, 24, 0.5);
  }

  .frame-left {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: clamp(20px, 2.6vw, 44px);
    background: linear-gradient(90deg, #06080e 0%, #0b0f19 70%, #1a2232 100%);
    box-shadow: 6px 0 18px rgba(2, 4, 10, 0.45);
  }

  .sill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: clamp(16px, 3.2vh, 36px);
    background: linear-gradient(180deg, #1a2232 0%, #0b0f19 30%, #06080e 100%);
    box-shadow: 0 -6px 18px rgba(2, 4, 10, 0.4);
  }

  .plant {
    position: absolute;
    left: 85%;
    bottom: calc(clamp(16px, 3.2vh, 36px) - 2px);
    height: clamp(90px, 20vh, 220px);
    width: auto;
    aspect-ratio: 120 / 160;
  }

  @keyframes breathe {
    from {
      opacity: calc(var(--o) * 0.55);
    }
    to {
      opacity: var(--o);
    }
  }

  @keyframes fall {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(0, 220px, 0);
    }
  }

  /* Stop-and-go down the glass: runs, waits, runs again, then fades out low on the pane. */
  @keyframes run {
    0% {
      opacity: 0;
      transform: translate3d(0, 0, 0);
    }
    6% {
      opacity: 0.9;
    }
    18% {
      transform: translate3d(0, 8vh, 0);
    }
    36% {
      transform: translate3d(1px, 10vh, 0);
    }
    52% {
      transform: translate3d(-1px, 26vh, 0);
    }
    70% {
      opacity: 0.9;
      transform: translate3d(0, 30vh, 0);
    }
    88%,
    100% {
      opacity: 0;
      transform: translate3d(0, 46vh, 0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .rain {
      display: none;
    }
    .bokeh.pulse {
      animation: none;
    }
    .drop {
      animation: none;
      opacity: 0.85;
      transform: translate3d(0, 12vh, 0);
    }
  }
</style>
