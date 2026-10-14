<script lang="ts">
  // Easter (beta collection theme, ADR-024; the light one): a spring morning. Soft sky with two slow clouds, rolling
  // pastel hills, a blossom branch reaching in from the upper left, a blossoming tree on the right (away from the
  // mascot, whose pink would blend into it on narrow windows), painted eggs in the grass with a bunny beside them, and a
  // few petals drifting down. Slow transform / opacity loops only; with reduced
  // motion the petals rest mid-air and the clouds stay put.
  //
  // Layout: horizontal positions are % of this layer (the window + 5vw for the tab pan; 4.8 %–95.2 % is always on
  // screen), sizes follow the window height (`--u`: 1vh, capped by the width). Busy art at the edges and low; the
  // middle stays calm.
  import { EGGS_LEFT, EGGS_RIGHT, PETAL_ANGLES, PINK, grass, type Blossom } from "./art";
  import Egg from "./Egg.svelte";

  const uid = $props.id();

  /** Blossoms on the branch (viewBox 0 0 400 240; drawn entering from the right, shown mirrored on the left). */
  const BRANCH_BLOSSOMS: readonly Blossom[] = [
    [262, 66, 13, 10, false],
    [236, 84, 11, 40, true],
    [300, 50, 10, -20, false],
    [208, 80, 12, 0, false],
    [196, 30, 10, 25, true],
    [204, 52, 9, 50, false],
    [168, 98, 11, -15, true],
    [140, 104, 12, 20, false],
    [134, 156, 10, 5, false],
    [146, 128, 9, 33, true],
    [96, 108, 11, -8, false],
    [84, 74, 9, 18, true],
    [60, 122, 10, 42, false],
    [36, 138, 9, 0, true],
    [288, 112, 10, 14, true],
    [330, 44, 9, 30, true],
    [356, 36, 10, -6, false],
  ];

  /** Canopy of the blossom tree (viewBox 0 0 300 400): circles [x, y, r, shade]. */
  const CANOPY: readonly (readonly [number, number, number, number])[] = [
    [70, 170, 44, 1],
    [44, 214, 34, 1],
    [196, 172, 40, 1],
    [128, 186, 46, 1],
    [234, 132, 30, 0],
    [104, 120, 54, 0],
    [166, 112, 50, 0],
    [56, 116, 36, 2],
    [124, 66, 40, 2],
    [190, 70, 32, 2],
    [74, 74, 28, 2],
  ];
  const CANOPY_SHADES = ["#f5bccd", "#f0a9bf", "#f9d0dc"] as const;

  /** Blossoms dotted over the canopy. */
  const TREE_BLOSSOMS: readonly Blossom[] = [
    [40, 150, 9, 0, true],
    [86, 96, 8, 30, true],
    [150, 64, 9, 12, true],
    [210, 104, 8, 40, true],
    [236, 160, 8, 8, true],
    [118, 150, 9, 20, false],
    [176, 150, 8, 50, true],
    [60, 196, 8, 15, true],
    [104, 210, 8, 0, false],
    [166, 206, 8, 30, true],
    [196, 52, 7, 18, false],
    [120, 34, 7, 0, true],
  ];

  interface Petal {
    /** Start (% of the layer, vh), drift (vh), spin (deg), size (vh), timing (s). */
    x: number;
    y: number;
    dx: number;
    dy: number;
    spin: number;
    size: number;
    duration: number;
    delay: number;
    white: boolean;
  }

  // From the branch (upper left) and from the tree (right); long, staggered, gentle.
  const PETALS: readonly Petal[] = [
    { x: 14, y: 22, dx: 26, dy: 58, spin: 320, size: 1.5, duration: 24, delay: -3, white: false },
    { x: 20, y: 28, dx: 30, dy: 52, spin: -260, size: 1.2, duration: 28, delay: -15, white: true },
    { x: 8, y: 18, dx: 22, dy: 64, spin: 400, size: 1.3, duration: 26, delay: -9, white: false },
    { x: 26, y: 32, dx: 34, dy: 50, spin: -300, size: 1.1, duration: 30, delay: -22, white: false },
    { x: 5, y: 30, dx: 18, dy: 56, spin: 280, size: 1.4, duration: 22, delay: -18, white: true },
    { x: 86, y: 44, dx: -18, dy: 40, spin: 300, size: 1.3, duration: 22, delay: -6, white: false },
    { x: 79, y: 40, dx: -24, dy: 44, spin: -340, size: 1.1, duration: 26, delay: -17, white: true },
    { x: 92, y: 50, dx: -16, dy: 34, spin: 260, size: 1.2, duration: 20, delay: -11, white: false },
  ];

  const GRASS_LEFT = grass(240, 120, 7);
  const GRASS_RIGHT = grass(300, 150, 31);
</script>

<div class="scene">
  <svg class="cloud one" viewBox="0 0 160 60" focusable="false">
    <path d="M14 56c-10 0-12-14-2-16 0-12 14-18 24-10 4-14 24-20 36-8 8-10 28-8 32 6 12-2 22 6 20 16 10 2 10 12 0 12z" />
  </svg>
  <svg class="cloud two" viewBox="0 0 160 60" focusable="false">
    <path d="M18 56c-12 0-12-14 0-14 2-10 14-14 22-8 6-12 22-14 32-4 10-8 26-4 28 8 14-2 22 8 14 18z" />
  </svg>

  <!-- Hills: far (hazy), mid, near meadow. -->
  <svg class="hills far" viewBox="0 0 1000 300" preserveAspectRatio="none" focusable="false">
    <path d="M0 110C80 80 170 74 260 96S420 140 520 118 690 60 800 76 950 120 1000 104V300H0z" fill="#d2e7c1" />
  </svg>
  <svg class="hills mid" viewBox="0 0 1000 200" preserveAspectRatio="none" focusable="false">
    <path d="M0 70C110 40 220 46 330 70S540 96 650 70 860 34 1000 60V200H0z" fill="#bcdc9e" />
  </svg>

  <!-- The blossom tree on the right (drawn leaning left, mirrored). -->
  <svg class="tree" viewBox="0 0 300 400" focusable="false">
    <path
      d="M92 400C98 360 100 330 96 300 92 270 100 244 114 220L132 226C120 250 116 274 120 300 124 330 124 364 134 400Z"
      fill="#7d5d57"
    />
    <g fill="none" stroke="#7d5d57" stroke-linecap="round">
      <path d="M118 226C100 200 84 184 64 168" stroke-width="11" />
      <path d="M124 222C140 196 160 180 190 166" stroke-width="10" />
      <path d="M122 222C124 190 120 160 126 126" stroke-width="8" />
    </g>
    <!-- Blossom clusters, each with light catching its upper left once mirrored. -->
    {#each CANOPY as [cx, cy, rr, shade], i (i)}
      <circle {cx} {cy} r={rr} fill={CANOPY_SHADES[shade]} />
      <circle cx={cx + rr * 0.28} cy={cy - rr * 0.3} r={rr * 0.5} fill="#fde3ea" opacity="0.5" />
    {/each}
    {#snippet flower([x, y, r, rot, white]: Blossom)}
      <g transform="translate({x} {y}) rotate({rot}) scale({r})">
        {#each PETAL_ANGLES as a (a)}
          <ellipse cx="0" cy="-0.52" rx="0.4" ry="0.56" transform="rotate({a})" fill={white ? PINK.petalWhite : PINK.petal} />
        {/each}
        <circle r="0.24" fill={PINK.centre} />
        <circle r="0.1" cx="0.12" cy="-0.06" fill={PINK.stamen} />
      </g>
    {/snippet}
    {#each TREE_BLOSSOMS as b, i (i)}
      {@render flower(b)}
    {/each}
    <!-- Petals already on the grass. -->
    <g fill="#f6b8cb" opacity="0.85">
      <ellipse cx="70" cy="392" rx="4" ry="2" />
      <ellipse cx="160" cy="396" rx="3.4" ry="1.8" />
      <ellipse cx="196" cy="390" rx="4" ry="2" />
      <ellipse cx="40" cy="398" rx="3" ry="1.6" />
    </g>
  </svg>

  <svg class="hills near" viewBox="0 0 1000 120" preserveAspectRatio="none" focusable="false">
    <path d="M0 40C90 26 180 24 280 36S460 56 560 44 760 22 860 32 960 44 1000 40V120H0z" fill="#a3cf82" />
  </svg>

  <!-- Blossom branch from the upper left (drawn from the right, mirrored). -->
  <svg class="branch" viewBox="0 0 400 240" focusable="false">
    <g fill="none" stroke="#7d5d57" stroke-linecap="round" stroke-linejoin="round">
      <path d="M410 32C350 38 300 58 250 70" stroke-width="10" />
      <path d="M250 70C200 82 160 100 120 104" stroke-width="7" />
      <path d="M120 104C90 108 60 118 30 140" stroke-width="4" />
      <path d="M300 56C290 80 296 100 284 124" stroke-width="4" />
      <path d="M210 80C206 58 212 42 202 22" stroke-width="3.5" />
      <path d="M152 98C142 120 148 140 134 162" stroke-width="3" />
      <path d="M96 108C90 92 94 80 84 66" stroke-width="2.5" />
    </g>
    <g fill="#9ccf7e">
      <ellipse cx="320" cy="48" rx="9" ry="4" transform="rotate(-24 320 48)" />
      <ellipse cx="226" cy="96" rx="8" ry="3.6" transform="rotate(28 226 96)" />
      <ellipse cx="118" cy="118" rx="7" ry="3.2" transform="rotate(36 118 118)" />
      <ellipse cx="276" cy="86" rx="7" ry="3.2" transform="rotate(-50 276 86)" />
    </g>
    <g fill="#f29bb6">
      <ellipse cx="284" cy="128" rx="3.4" ry="4.6" />
      <ellipse cx="202" cy="18" rx="3" ry="4.2" />
      <ellipse cx="132" cy="166" rx="3" ry="4.2" />
      <ellipse cx="84" cy="62" rx="2.8" ry="3.8" />
      <ellipse cx="28" cy="142" rx="3" ry="4" />
    </g>
    {#snippet flower([x, y, r, rot, white]: Blossom)}
      <g transform="translate({x} {y}) rotate({rot}) scale({r})">
        {#each PETAL_ANGLES as a (a)}
          <ellipse cx="0" cy="-0.52" rx="0.4" ry="0.56" transform="rotate({a})" fill={white ? PINK.petalWhite : PINK.petal} />
        {/each}
        <circle r="0.24" fill={PINK.centre} />
        <circle r="0.1" cx="0.12" cy="-0.06" fill={PINK.stamen} />
      </g>
    {/snippet}
    {#each BRANCH_BLOSSOMS as b, i (i)}
      {@render flower(b)}
    {/each}
  </svg>

  <!-- Painted eggs in the grass: a clump on the left, another on the right with a bunny. -->
  <svg class="clump left" viewBox="0 0 240 120" focusable="false">
    <ellipse cx="96" cy="114" rx="88" ry="7" fill="#6f9f55" opacity="0.25" />
    {#each EGGS_LEFT as egg, i (i)}
      <Egg {egg} />
    {/each}
    <path d={GRASS_LEFT} fill="#86bf67" />
    <g>
      <path d="M176 120c0-14 2-26 6-36" stroke="#6a9f4e" stroke-width="2.4" fill="none" />
      <path d="M174 86c0-8 4-12 8-12s8 4 8 12c-2 3-5 5-8 5s-6-2-8-5z" fill="#f5a3bd" />
      <path d="M178 76l4 6 4-6" stroke="#e57f9f" stroke-width="1.6" fill="none" />
      <path d="M200 120c0-10-1-20-4-28" stroke="#6a9f4e" stroke-width="2.2" fill="none" />
      <path d="M190 94c0-7 3-10 6-10s7 3 7 10c-2 3-4 4-7 4s-5-1-6-4z" fill="#f6d36b" />
    </g>
  </svg>

  <svg class="clump right" viewBox="0 0 300 150" focusable="false">
    <ellipse cx="170" cy="144" rx="120" ry="8" fill="#6f9f55" opacity="0.25" />
    {#each EGGS_RIGHT as egg, i (i)}
      <Egg {egg} />
    {/each}
    <!-- A bunny sitting by the eggs, facing them. -->
    <g transform="translate(-8 0)">
      <ellipse cx="248" cy="52" rx="7" ry="22" transform="rotate(14 248 52)" fill="#f1e8de" />
      <ellipse cx="248" cy="54" rx="3.4" ry="15" transform="rotate(14 248 54)" fill="#f6c3d1" />
      <ellipse cx="262" cy="114" rx="34" ry="28" fill="#fbf6ef" />
      <circle cx="280" cy="122" r="22" fill="#efe5da" />
      <circle cx="298" cy="116" r="8.5" fill="#ffffff" />
      <circle cx="232" cy="84" r="19" fill="#fbf6ef" />
      <ellipse cx="236" cy="50" rx="7.4" ry="24" transform="rotate(-10 236 50)" fill="#fbf6ef" />
      <ellipse cx="236" cy="52" rx="3.8" ry="17" transform="rotate(-10 236 52)" fill="#f6c3d1" />
      <circle cx="223" cy="80" r="2.8" fill="#3d3240" />
      <circle cx="222" cy="79" r="0.9" fill="#ffffff" />
      <path d="M214 88q2-2 4 0-1 2-2 2t-2-2z" fill="#e88aa6" />
      <circle cx="226" cy="92" r="4" fill="#f7c1cf" opacity="0.6" />
      <ellipse cx="238" cy="138" rx="10" ry="5.4" fill="#f4ece2" />
    </g>
    <path d={GRASS_RIGHT} fill="#86bf67" />
    <g>
      <path d="M30 150c0-12 1-22 4-30" stroke="#6a9f4e" stroke-width="2.2" fill="none" />
      <g transform="translate(34 116)">
        {#each PETAL_ANGLES as a (a)}
          <ellipse cx="0" cy="-5" rx="2.6" ry="4.6" transform="rotate({a})" fill="#ffffff" />
        {/each}
        <circle r="2.6" fill="#f3c45a" />
      </g>
    </g>
  </svg>

  {#each PETALS as p, i (i)}
    <span
      class="petal"
      class:white={p.white}
      style:left="{p.x}%"
      style:top="calc({p.y} * var(--u))"
      style:--size="calc({p.size} * var(--u))"
      style:--dx="calc({p.dx} * var(--u))"
      style:--dy="calc({p.dy} * var(--u))"
      style:--spin="{p.spin}deg"
      style:animation-duration="{p.duration}s"
      style:animation-delay="{p.delay}s"
    ></span>
  {/each}
</div>

<style>
  .scene {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    /* Art unit: 1vh, but never more than 0.75vw, so tall (portrait) windows keep the art in proportion. */
    --u: min(1vh, 0.75vw);
  }

  svg {
    display: block;
  }

  /* ---------- sky ---------- */

  .cloud {
    position: absolute;
    left: 0;
    fill: #ffffff;
    animation: drift linear infinite;
    will-change: transform;
  }

  .cloud.one {
    top: calc(18 * var(--u));
    width: calc(26 * var(--u));
    opacity: 0.85;
    animation-duration: 150s;
    animation-delay: -40s;
  }

  .cloud.two {
    top: calc(30 * var(--u));
    width: calc(18 * var(--u));
    opacity: 0.7;
    animation-duration: 190s;
    animation-delay: -130s;
  }

  /* ---------- hills ---------- */

  .hills {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
  }

  .hills.far {
    height: calc(36 * var(--u));
  }

  .hills.mid {
    height: calc(24 * var(--u));
  }

  .hills.near {
    height: calc(11 * var(--u));
  }

  .tree {
    position: absolute;
    right: -1%;
    transform: scaleX(-1);
    bottom: calc(7 * var(--u));
    height: calc(52 * var(--u));
    width: calc(52 * var(--u) * 300 / 400);
  }

  .branch {
    position: absolute;
    left: 0;
    transform: scaleX(-1);
    top: clamp(70px, calc(9 * var(--u)), 110px);
    height: calc(32 * var(--u));
    width: calc(32 * var(--u) * 400 / 240);
    overflow: visible;
  }

  .clump {
    position: absolute;
    bottom: calc(0.6 * var(--u));
  }

  .clump.left {
    left: 8%;
    height: calc(14 * var(--u));
    width: calc(14 * var(--u) * 240 / 120);
  }

  .clump.right {
    left: 74%;
    height: calc(17 * var(--u));
    width: calc(17 * var(--u) * 300 / 150);
  }

  /* ---------- petals ---------- */

  .petal {
    position: absolute;
    width: var(--size);
    height: calc(var(--size) * 0.7);
    border-radius: 80% 0 80% 0;
    background: #f4adc3;
    opacity: 0;
    animation: petal ease-in-out infinite;
    will-change: transform, opacity;
  }

  .petal.white {
    background: #fff1f5;
    box-shadow: inset 0 0 0 1px rgba(232, 150, 180, 0.35);
  }

  /* ---------- motion ---------- */

  @keyframes drift {
    from {
      transform: translate3d(calc(-30 * var(--u)), 0, 0);
    }
    to {
      transform: translate3d(110vw, 0, 0);
    }
  }

  /* A slow fall with a lazy side-to-side, fading in and out. */
  @keyframes petal {
    0% {
      opacity: 0;
      transform: translate3d(0, 0, 0) rotate(0deg);
    }
    10% {
      opacity: 0.9;
    }
    33% {
      transform: translate3d(calc(var(--dx) * 0.33 + 2.5 * var(--u)), calc(var(--dy) * 0.33), 0)
        rotate(calc(var(--spin) * 0.33));
    }
    66% {
      transform: translate3d(calc(var(--dx) * 0.66 - 2.5 * var(--u)), calc(var(--dy) * 0.66), 0)
        rotate(calc(var(--spin) * 0.66));
    }
    88% {
      opacity: 0.9;
    }
    100% {
      opacity: 0;
      transform: translate3d(var(--dx), var(--dy), 0) rotate(var(--spin));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .cloud,
    .petal {
      animation: none;
    }

    .cloud.one {
      transform: translate3d(22vw, 0, 0);
    }

    .cloud.two {
      transform: translate3d(58vw, 0, 0);
    }

    .petal {
      opacity: 0.85;
      transform: translate3d(calc(var(--dx) * 0.4), calc(var(--dy) * 0.4), 0) rotate(calc(var(--spin) * 0.4));
    }
  }
</style>
