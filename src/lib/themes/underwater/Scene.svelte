<script lang="ts">
  // Underwater (beta collection theme, ADR-024): the axolotl's home. Light rays from the surface, slow fish shoals far
  // away, kelp swaying from a sandy floor with rocks, a little cave, a starfish and a shell, bubbles rising and a few
  // drifting specks. (The axolotl itself is the app's mascot; the scene never draws it.) Slow transform / opacity loops
  // only; with reduced motion everything rests.
  //
  // Layout: horizontal positions are % of this layer (the window + 5vw for the tab pan; 4.8 %–95.2 % is always on
  // screen), sizes follow the window height (`--u`: 1vh, capped by the width). Busy art at the edges and low; the
  // middle stays calm.
  import { FISH, STARFISH, kelp } from "./art";

  const uid = $props.id();

  interface Ray {
    x: number;
    width: number;
    tilt: number;
    duration: number;
    delay: number;
  }

  const RAYS: readonly Ray[] = [
    { x: 6, width: 24, tilt: 16, duration: 11, delay: -2 },
    { x: 55, width: 20, tilt: 14, duration: 12, delay: -4 },
    { x: 76, width: 27, tilt: 18, duration: 15, delay: -10 },
  ];

  interface Strand {
    x: number;
    height: number;
    seed: number;
    far: boolean;
    duration: number;
    delay: number;
  }

  const STRANDS: readonly Strand[] = [
    { x: 3, height: 58, seed: 3, far: true, duration: 9, delay: -2 },
    { x: 8, height: 70, seed: 1, far: false, duration: 7.5, delay: -5 },
    { x: 14, height: 44, seed: 6, far: false, duration: 8.6, delay: -1 },
    { x: 79, height: 40, seed: 4, far: false, duration: 8, delay: -3 },
    { x: 85, height: 66, seed: 2, far: true, duration: 10, delay: -6 },
    { x: 89.5, height: 74, seed: 5, far: false, duration: 7, delay: -4 },
    { x: 95, height: 52, seed: 7, far: false, duration: 9.4, delay: -8 },
  ];

  const KELP = STRANDS.map((s) => kelp(s.seed));

  interface Bubble {
    x: number;
    bottom: number;
    size: number;
    rise: number;
    duration: number;
    delay: number;
  }

  // From the kelp beds and the cave; small, slow, staggered.
  const BUBBLES: readonly Bubble[] = [
    { x: 9, bottom: 10, size: 1.6, rise: 82, duration: 13, delay: -1 },
    { x: 10.5, bottom: 12, size: 1, rise: 80, duration: 11, delay: -6 },
    { x: 15, bottom: 9, size: 1.3, rise: 78, duration: 15, delay: -9 },
    { x: 5, bottom: 11, size: 0.9, rise: 84, duration: 12, delay: -3.5 },
    { x: 87, bottom: 14, size: 1.5, rise: 76, duration: 14, delay: -2 },
    { x: 88.4, bottom: 15, size: 0.9, rise: 74, duration: 10, delay: -7.5 },
    { x: 92, bottom: 12, size: 1.2, rise: 80, duration: 16, delay: -12 },
    { x: 81, bottom: 10, size: 1.1, rise: 82, duration: 12.5, delay: -4.5 },
    { x: 70, bottom: 9, size: 0.8, rise: 84, duration: 13.5, delay: -10 },
    { x: 30, bottom: 8, size: 1, rise: 86, duration: 15.5, delay: -13 },
  ];

  /** A shoal: five fish at offsets inside a 200 × 70 box. */
  const SHOAL: readonly (readonly [number, number, number])[] = [
    [0, 30, 1],
    [34, 14, 0.85],
    [40, 44, 0.9],
    [74, 28, 1.05],
    [108, 40, 0.8],
  ];
</script>

<div class="scene">
  {#each RAYS as r, i (i)}
    <div class="ray-wrap" style:left="{r.x}%" style:width="calc({r.width} * var(--u))" style:transform="rotate({r.tilt}deg)">
      <div class="ray" style:animation-duration="{r.duration}s" style:animation-delay="{r.delay}s"></div>
    </div>
  {/each}

  <div class="specks"></div>

  <!-- Two shoals far away, one each way. -->
  <div class="shoal one">
    <svg viewBox="0 0 200 70" focusable="false">
      {#each SHOAL as [x, y, s], i (i)}
        <path d={FISH} transform="translate({x} {y}) scale({s})" />
      {/each}
    </svg>
  </div>
  <div class="shoal two">
    <svg viewBox="0 0 200 70" focusable="false">
      <g transform="translate(200 0) scale(-1 1)">
        {#each SHOAL as [x, y, s], i (i)}
          <path d={FISH} transform="translate({x} {y}) scale({s})" />
        {/each}
      </g>
    </svg>
  </div>

  <!-- Far rocks, then the sandy floor. -->
  <svg class="ridge" viewBox="0 0 1000 300" preserveAspectRatio="none" focusable="false">
    <path
      d="M0 150C40 120 90 130 130 112S220 120 260 140 360 170 420 160 520 120 600 130 720 172 800 150 900 96 1000 120V300H0z"
      fill="#0a2d45"
    />
  </svg>

  {#each STRANDS as s, i (i)}
    <svg
      class="kelp"
      class:far={s.far}
      viewBox="0 0 80 400"
      preserveAspectRatio="xMidYMax meet"
      focusable="false"
      style:left="{s.x}%"
      style:--h="calc({s.height} * var(--u))"
      style:animation-duration="{s.duration}s"
      style:animation-delay="{s.delay}s"
    >
      <path d={KELP[i]?.stalk} class="stalk" />
      <path d={KELP[i]?.blades} class="blade" />
      <path d={KELP[i]?.bladesDark} class="blade dark" />
    </svg>
  {/each}

  <svg class="floor" viewBox="0 0 1000 140" preserveAspectRatio="none" focusable="false">
    <defs>
      <linearGradient id="{uid}-sand" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#3f6a74" />
        <stop offset="100%" stop-color="#244452" />
      </linearGradient>
    </defs>
    <path d="M0 40C120 26 230 30 340 40S560 52 680 40 880 24 1000 34V140H0z" fill="url(#{uid}-sand)" />
    <g fill="none" stroke="#5a8790" stroke-opacity="0.35" stroke-width="1.6">
      <path d="M40 70c40-6 80-6 120 0M220 86c50-6 100-6 150 0M460 74c40-5 80-5 120 0M640 92c50-6 100-6 150 0M820 70c40-5 80-5 120 0" />
    </g>
  </svg>

  <!-- Left: rocks, pebbles, a shell and a starfish. -->
  <svg class="rocks left" viewBox="0 0 300 120" focusable="false">
    <path d="M0 120C4 80 30 58 70 56 96 40 140 44 158 70 180 72 196 92 198 120z" fill="#12394f" />
    <path d="M70 56c26-16 70-12 88 14-30-14-60-16-88-14z" fill="#1d5068" />
    <path d="M150 120c4-20 20-30 40-30s34 12 38 30z" fill="#164259" />
    <g fill="#2d5866">
      <ellipse cx="236" cy="114" rx="10" ry="5" />
      <ellipse cx="256" cy="116" rx="6" ry="3.4" />
      <ellipse cx="38" cy="116" rx="7" ry="3.6" />
    </g>
    <g transform="translate(270 104) rotate(-14)">
      <path d="M0 10L-11-2q11-12 22 0z" fill="#e7cfb4" />
      <path d="M0 10L-6-6M0 10V-8M0 10L6-6" stroke="#c7a98a" stroke-width="1.2" fill="none" />
    </g>
    <g transform="translate(212 106) rotate(12) scale(1.1)">
      <path d={STARFISH} fill="#ee9a7a" stroke="#ee9a7a" stroke-width="3" stroke-linejoin="round" />
      <g fill="#ffd0b8" opacity="0.8">
        <circle r="1.2" />
        <circle cx="0" cy="-5" r="0.8" />
        <circle cx="4.6" cy="-1.5" r="0.8" />
        <circle cx="2.9" cy="4" r="0.8" />
        <circle cx="-2.9" cy="4" r="0.8" />
        <circle cx="-4.6" cy="-1.5" r="0.8" />
      </g>
    </g>
  </svg>

  <!-- Right: a rock with a small cave, sea grass on top, pebbles. -->
  <svg class="rocks right" viewBox="0 0 300 160" focusable="false">
    <path d="M0 160C8 112 40 82 84 72 114 44 166 38 206 54 248 46 284 76 298 118L300 160z" fill="#12394f" />
    <path d="M84 72c30-28 82-34 122-18-44-6-86 0-122 18z" fill="#1d5068" />
    <path d="M118 160c2-30 20-48 42-48s40 18 42 48z" fill="#05141f" />
    <path d="M124 160c3-24 18-38 36-38" fill="none" stroke="#1d5068" stroke-width="3" stroke-opacity="0.6" />
    <g fill="none" stroke="#2f7a5c" stroke-width="3" stroke-linecap="round">
      <path d="M150 46c-2-14 4-22 0-34M160 44c2-10 8-14 6-26M140 50c-6-8-6-16-12-22" />
    </g>
    <g fill="#2d5866">
      <ellipse cx="24" cy="154" rx="9" ry="4.4" />
      <ellipse cx="226" cy="156" rx="12" ry="4.6" />
      <ellipse cx="250" cy="154" rx="6" ry="3.2" />
    </g>
  </svg>

  {#each BUBBLES as b, i (i)}
    <span
      class="bubble"
      style:left="{b.x}%"
      style:bottom="calc({b.bottom} * var(--u))"
      style:--size="calc({b.size} * var(--u))"
      style:--rise="calc({b.rise} * var(--u))"
      style:animation-duration="{b.duration}s"
      style:animation-delay="{b.delay}s"
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

  /* ---------- light from the surface ---------- */

  .ray-wrap {
    position: absolute;
    top: calc(-12 * var(--u));
    height: calc(100 * var(--u));
    transform-origin: 50% 0;
  }

  .ray {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      180deg,
      rgba(190, 245, 255, 0.24) 0%,
      rgba(190, 245, 255, 0.09) 45%,
      transparent 85%
    );
    /* Soft edges across the beam, fading out lower down. */
    mask-image: linear-gradient(90deg, transparent 0%, #000 35%, #000 65%, transparent 100%);
    animation: breathe ease-in-out infinite alternate;
  }

  /* Specks drifting slowly upwards (one tiled layer; loops by its tile height). */
  .specks {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: calc(100% + 360px);
    background-image:
      radial-gradient(circle at 30% 40%, rgba(200, 240, 250, 0.35) 0 1px, transparent 1.6px),
      radial-gradient(circle at 70% 70%, rgba(200, 240, 250, 0.25) 0 1.2px, transparent 1.8px),
      radial-gradient(circle at 50% 10%, rgba(200, 240, 250, 0.2) 0 0.8px, transparent 1.4px);
    background-size:
      157px 120px,
      229px 180px,
      191px 360px;
    animation: specks 60s linear infinite;
    will-change: transform;
  }

  .shoal {
    position: absolute;
    left: 0;
    width: calc(18 * var(--u));
    fill: #1a5470;
    opacity: 0.8;
    animation: swim linear infinite;
    will-change: transform;
  }

  .shoal svg {
    width: 100%;
    height: auto;
  }

  .shoal.one {
    top: calc(30 * var(--u));
    animation-duration: 95s;
    animation-delay: -30s;
  }

  .shoal.two {
    top: calc(44 * var(--u));
    width: calc(13 * var(--u));
    opacity: 0.6;
    animation-name: swim-back;
    animation-duration: 120s;
    animation-delay: -80s;
  }

  /* ---------- floor ---------- */

  .ridge {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: calc(28 * var(--u));
  }

  .kelp {
    position: absolute;
    bottom: calc(4 * var(--u));
    height: var(--h);
    width: calc(var(--h) * 80 / 400);
    margin-left: calc(var(--h) * 80 / 400 / -2);
    overflow: visible;
    transform-origin: 50% 100%;
    animation: sway ease-in-out infinite alternate;
    will-change: transform;
  }

  .stalk {
    fill: none;
    stroke: #1f5a4a;
    stroke-width: 4.4;
    stroke-linecap: round;
  }

  .blade {
    fill: #2c7258;
  }

  .blade.dark {
    fill: #215e4a;
  }

  .kelp.far .stalk {
    stroke: #194a52;
  }

  .kelp.far .blade {
    fill: #1f5458;
  }

  .kelp.far .blade.dark {
    fill: #1a4a50;
  }

  .floor {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: calc(13 * var(--u));
  }

  .rocks {
    position: absolute;
    bottom: 0;
  }

  .rocks.left {
    left: 0;
    height: calc(14 * var(--u));
    width: calc(14 * var(--u) * 300 / 120);
  }

  .rocks.right {
    right: 4%;
    height: calc(19 * var(--u));
    width: calc(19 * var(--u) * 300 / 160);
  }

  /* ---------- bubbles ---------- */

  .bubble {
    position: absolute;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    background:
      radial-gradient(circle at 34% 30%, rgba(255, 255, 255, 0.75) 0 12%, transparent 16%),
      radial-gradient(
        circle,
        rgba(190, 240, 255, 0.06) 0 52%,
        rgba(190, 240, 255, 0.55) 64%,
        rgba(190, 240, 255, 0.12) 71%,
        transparent 73%
      );
    opacity: 0;
    animation: rise ease-in infinite;
    will-change: transform, opacity;
  }

  /* ---------- motion ---------- */

  @keyframes breathe {
    from {
      opacity: 0.45;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes specks {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(0, -360px, 0);
    }
  }

  @keyframes swim {
    0% {
      transform: translate3d(calc(-16 * var(--u)), 0, 0);
    }
    50% {
      transform: translate3d(50vw, calc(-3 * var(--u)), 0);
    }
    100% {
      transform: translate3d(110vw, calc(1 * var(--u)), 0);
    }
  }

  @keyframes swim-back {
    0% {
      transform: translate3d(110vw, 0, 0);
    }
    50% {
      transform: translate3d(50vw, calc(2.5 * var(--u)), 0);
    }
    100% {
      transform: translate3d(calc(-16 * var(--u)), calc(-1 * var(--u)), 0);
    }
  }

  @keyframes sway {
    from {
      transform: rotate(-3.6deg);
    }
    to {
      transform: rotate(3.6deg);
    }
  }

  /* Up with a little side-to-side wobble, fading in at the floor and out below the surface. */
  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate3d(0, 0, 0);
    }
    8% {
      opacity: 0.9;
    }
    30% {
      transform: translate3d(calc(0.8 * var(--u)), calc(var(--rise) * -0.3), 0);
    }
    60% {
      transform: translate3d(calc(-0.8 * var(--u)), calc(var(--rise) * -0.6), 0);
    }
    85% {
      opacity: 0.7;
    }
    100% {
      opacity: 0;
      transform: translate3d(calc(0.4 * var(--u)), calc(var(--rise) * -1), 0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ray,
    .specks,
    .shoal,
    .kelp,
    .bubble {
      animation: none;
    }

    .ray {
      opacity: 0.75;
    }

    .shoal.one {
      transform: translate3d(66vw, 0, 0);
    }

    .shoal.two {
      transform: translate3d(18vw, 0, 0);
    }

    .bubble {
      opacity: 0.8;
      transform: translate3d(0, calc(var(--rise) * -0.25), 0);
    }

    .bubble:nth-of-type(2n) {
      transform: translate3d(calc(0.6 * var(--u)), calc(var(--rise) * -0.5), 0);
    }
  }
</style>
