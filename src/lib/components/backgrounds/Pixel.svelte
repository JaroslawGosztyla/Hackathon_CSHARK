<script lang="ts">
  // 8-bit: square stars blinking in hard steps, a pixel-art moon, clouds that hop across the sky one "pixel" at a
  // time and stepped hills. Drawn with crisp edges (no anti-aliasing), so it looks like scaled-up pixel art.
  import { mulberry32 } from "./random";

  const rand = mulberry32(8);

  const STAR_COLORS = ["#ffffff", "#fcfc54", "#7ce8ff", "#ffb8f8"] as const;
  const stars = Array.from({ length: 44 }, () => ({
    left: rand() * 100,
    top: rand() * 62,
    size: [2, 3, 4][Math.floor(rand() * 3)] ?? 2,
    color: STAR_COLORS[Math.floor(rand() * STAR_COLORS.length)] ?? STAR_COLORS[0],
    duration: 1.6 + rand() * 2.8,
    delay: -rand() * 4,
  }));

  /** One character per pixel: `#` lit surface, `+` crater, anything else transparent. */
  const MOON = [
    "....######....",
    "..##########..",
    ".############.",
    ".####++######.",
    "##############",
    "######++######",
    "##############",
    "###++#########",
    "##############",
    "##########++##",
    ".############.",
    ".#####++#####.",
    "..##########..",
    "....######....",
  ];
  const CLOUD = ["....####....", "..########..", ".##########.", "############"];

  function pixels(rows: readonly string[]): { x: number; y: number; kind: string }[] {
    const out: { x: number; y: number; kind: string }[] = [];
    rows.forEach((row, y) => {
      [...row].forEach((kind, x) => {
        if (kind !== ".") out.push({ x, y, kind });
      });
    });
    return out;
  }

  const moon = pixels(MOON);
  const cloud = pixels(CLOUD);

  /** Stepped skyline: `columns` blocks of `width` units, heights from a bounded random walk. */
  function hills(columns: number, width: number, base: number, min: number, max: number): string {
    let h = base;
    let d = `M0 ${40}`;
    for (let i = 0; i < columns; i++) {
      h = Math.min(max, Math.max(min, h + Math.round((rand() - 0.5) * 6)));
      d += ` V${40 - h} H${(i + 1) * width}`;
    }
    return `${d} V40 Z`;
  }

  const farHills = hills(40, 4, 18, 10, 28);
  const nearHills = hills(32, 5, 9, 4, 15);
</script>

{#each stars as s, i (i)}
  <span
    class="star"
    style:left="{s.left}%"
    style:top="{s.top}%"
    style:width="{s.size}px"
    style:height="{s.size}px"
    style:background={s.color}
    style:animation-duration="{s.duration}s"
    style:animation-delay="{s.delay}s"
  ></span>
{/each}

<svg class="moon" viewBox="0 0 14 14" shape-rendering="crispEdges" focusable="false">
  {#each moon as p, i (i)}
    <rect x={p.x} y={p.y} width="1" height="1" class:crater={p.kind === "+"} />
  {/each}
</svg>

{#each [0, 1] as c (c)}
  <svg class="cloud cloud-{c}" viewBox="0 0 12 4" shape-rendering="crispEdges" focusable="false">
    {#each cloud as p, i (i)}
      <rect x={p.x} y={p.y} width="1" height="1" />
    {/each}
  </svg>
{/each}

<svg class="hills" viewBox="0 0 160 40" preserveAspectRatio="xMidYMax slice" shape-rendering="crispEdges" focusable="false">
  <path d={farHills} class="far" />
  <path d={nearHills} class="near" />
</svg>

<style>
  svg {
    position: absolute;
    display: block;
    image-rendering: pixelated;
  }

  .star {
    position: absolute;
    opacity: 0.85;
    animation-name: blink;
    animation-timing-function: steps(1, end);
    animation-iteration-count: infinite;
  }

  .moon {
    top: 8%;
    right: 11%;
    width: 84px;
    height: 84px;
    fill: #f8f0c8;
    opacity: 0.9;
  }

  .moon .crater {
    fill: #c8b878;
  }

  .cloud {
    width: 120px;
    height: 40px;
    fill: #2c2670;
    opacity: 0.9;
    animation: drift 120s steps(120, end) infinite;
  }

  .cloud-0 {
    top: 14%;
    animation-delay: -30s;
  }

  .cloud-1 {
    top: 30%;
    width: 84px;
    height: 28px;
    animation-duration: 160s;
    animation-timing-function: steps(160, end);
    animation-delay: -110s;
  }

  .hills {
    left: 0;
    bottom: 0;
    width: 100%;
    height: 24vh;
  }

  .far {
    fill: #1f1a5c;
  }

  .near {
    fill: #120f33;
  }

  @keyframes blink {
    0%,
    100% {
      opacity: 0.9;
    }
    50% {
      opacity: 0.25;
    }
  }

  @keyframes drift {
    from {
      transform: translateX(-140px);
    }
    to {
      transform: translateX(110vw);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .star,
    .cloud {
      animation: none;
    }
    .cloud-0 {
      transform: translateX(18vw);
    }
    .cloud-1 {
      transform: translateX(64vw);
    }
  }
</style>
