<script lang="ts">
  // Christmas (beta collection theme, ADR-024): a snowy night. A thin moon, a row of distant pines, big snowy pines and a
  // snowman on the left (where the mascot sits on narrow windows), a cottage with a warm window, chimney smoke and
  // twinkling string lights on the right, and gentle snowfall in two layers. Motion is slow transform / opacity only;
  // with reduced motion the snow hangs still.
  //
  // Layout: horizontal positions are % of this layer (the window + 5vw for the tab pan; 4.8 %–95.2 % is always on
  // screen), sizes follow the window height (`--u`: 1vh, capped by the width). The busy art sits at the edges and
  // low; the middle stays calm.
  import Cottage from "./Cottage.svelte";
  import Pine from "./Pine.svelte";

  const uid = $props.id();

  /** Big pines: centre (% of the layer), bottom (vh), height (vh). */
  const PINES: readonly (readonly [number, number, number])[] = [
    [3, 5, 62],
    [12.5, 7, 34],
    [22, 7, 19],
    [93, 5, 58],
    [70.5, 7, 22],
  ];

  /** A strip of distant pines (300 × 90), repeated along the far hills; bottoms are hidden by the snow in front. */
  function forestStrip(): string {
    const trees: [number, number][] = [
      [18, 58],
      [44, 76],
      [70, 50],
      [98, 66],
      [130, 84],
      [158, 54],
      [186, 70],
      [214, 60],
      [244, 80],
      [276, 56],
    ];
    const paths = trees
      .map(([x, h]) => {
        const top = 90 - h;
        const w = h * 0.3;
        return (
          `M${x} ${top}l${w * 0.5} ${h * 0.3}h${-w * 0.2}l${w * 0.45} ${h * 0.3}h${-w * 0.2}` +
          `l${w * 0.45} ${h * 0.4}h${-w * 2}l${w * 0.45} ${-h * 0.4}h${-w * 0.2}l${w * 0.45} ${-h * 0.3}h${-w * 0.2}z`
        );
      })
      .join("");
    const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 300 90'><path d='${paths}' fill='#18304f'/></svg>`;
    return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
  }

  const FOREST = forestStrip();
</script>

<div class="scene">
  <div class="stars"></div>
  <div class="stars twinkle"></div>

  <div class="moon-halo"></div>
  <svg class="moon" viewBox="0 0 100 100" focusable="false">
    <defs>
      <mask id="{uid}-crescent">
        <circle cx="50" cy="50" r="40" fill="#fff" />
        <circle cx="66" cy="40" r="36" fill="#000" />
      </mask>
    </defs>
    <circle cx="50" cy="50" r="40" fill="#f3f1e4" mask="url(#{uid}-crescent)" />
  </svg>

  <div class="snow far"><div class="flakes"></div></div>

  <div class="forest" style:background-image={FOREST}></div>
  <svg class="drift mid" viewBox="0 0 1000 200" preserveAspectRatio="none" focusable="false">
    <defs>
      <linearGradient id="{uid}-mid" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#4a6696" />
        <stop offset="100%" stop-color="#2c4470" />
      </linearGradient>
    </defs>
    <path d="M0 40C90 20 170 18 260 34S420 56 520 42 700 12 800 24 940 50 1000 40V200H0z" fill="url(#{uid}-mid)" />
  </svg>

  {#each PINES as [x, bottom, height], i (i)}
    <div
      class="pine"
      style:left="{x}%"
      style:bottom="calc({bottom} * var(--u))"
      style:--h="calc({height} * var(--u))"
    >
      <Pine />
    </div>
  {/each}

  <div class="cottage">
    <Cottage />
    {#each [0, 1, 2] as i (i)}
      <span class="puff" style:animation-delay="{-i * 3.4}s"></span>
    {/each}
  </div>

  <svg class="snowman" viewBox="0 0 100 140" focusable="false">
    <defs>
      <radialGradient id="{uid}-ball" cx="38%" cy="32%" r="72%">
        <stop offset="0%" stop-color="#f2f6fd" />
        <stop offset="70%" stop-color="#cdd9ec" />
        <stop offset="100%" stop-color="#9fb2d2" />
      </radialGradient>
    </defs>
    <g fill="none" stroke="#3d2e26" stroke-width="2.6" stroke-linecap="round">
      <path d="M32 66L12 54M18 58l-6-8M18 58l-9 2M68 66l20-14M82 56l4-8M82 56l8 2" />
    </g>
    <circle cx="50" cy="110" r="28" fill="url(#{uid}-ball)" />
    <circle cx="50" cy="68" r="20" fill="url(#{uid}-ball)" />
    <circle cx="50" cy="38" r="14" fill="url(#{uid}-ball)" />
    <path d="M37 46c6 5 20 5 26 0l1 6c-4 3-8 3-12 3l2 14-6 1-1-15c-4 0-8-1-11-3z" fill="#c95f6a" />
    <path d="M35 26h30l-2 4H37z" fill="#1c2135" />
    <path d="M39 26V10q0-3 3-3h16q3 0 3 3v16z" fill="#1c2135" />
    <path d="M39 20h22v3H39z" fill="#c95f6a" />
    <g fill="#1c2135">
      <circle cx="45" cy="35" r="1.8" />
      <circle cx="55" cy="35" r="1.8" />
      <circle cx="50" cy="64" r="1.8" />
      <circle cx="50" cy="73" r="1.8" />
    </g>
    <path d="M50 39l11 2.4-11 1.6z" fill="#f29a4a" />
    <ellipse cx="50" cy="138" rx="34" ry="4" fill="#5f739a" opacity="0.5" />
  </svg>

  <svg class="drift near" viewBox="0 0 1000 120" preserveAspectRatio="none" focusable="false">
    <defs>
      <linearGradient id="{uid}-near" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#9db1d2" />
        <stop offset="100%" stop-color="#5f739a" />
      </linearGradient>
    </defs>
    <path d="M0 46C70 30 150 26 250 38S420 60 540 48 720 22 820 32 950 52 1000 44V120H0z" fill="url(#{uid}-near)" />
  </svg>

  <div class="snow near"><div class="flakes"></div></div>
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

  .stars {
    position: absolute;
    inset: 0 0 40% 0;
    background-image:
      radial-gradient(circle at 30% 40%, rgba(240, 246, 255, 0.9) 0 1px, transparent 1.6px),
      radial-gradient(circle at 70% 20%, rgba(240, 246, 255, 0.7) 0 1px, transparent 1.6px),
      radial-gradient(circle at 50% 80%, rgba(255, 236, 200, 0.7) 0 1.2px, transparent 1.8px);
    background-size:
      181px 137px,
      233px 191px,
      317px 271px;
    mask-image: linear-gradient(180deg, #000 0%, #000 50%, transparent 100%);
    opacity: 0.55;
  }

  .stars.twinkle {
    background-image:
      radial-gradient(circle at 20% 60%, rgba(255, 248, 230, 0.95) 0 1.3px, transparent 2px),
      radial-gradient(circle at 80% 30%, rgba(214, 230, 255, 0.85) 0 1px, transparent 1.6px);
    background-size:
      383px 229px,
      277px 173px;
    opacity: 0.3;
    animation: twinkle 8s ease-in-out infinite alternate;
  }

  .moon,
  .moon-halo {
    position: absolute;
    top: clamp(80px, calc(13 * var(--u)), 160px);
    left: 77%;
    width: calc(11 * var(--u));
    height: calc(11 * var(--u));
  }

  .moon-halo {
    transform: scale(3.2);
    border-radius: 50%;
    background: radial-gradient(circle, rgba(220, 230, 255, 0.16) 0%, rgba(160, 190, 240, 0.06) 40%, transparent 70%);
  }

  /* ---------- hills, trees, cottage ---------- */

  .forest {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(15 * var(--u));
    height: calc(11 * var(--u));
    background-repeat: repeat-x;
    background-size: auto 100%;
    background-position: 0 100%;
  }

  .drift {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
  }

  .drift.mid {
    height: calc(19 * var(--u));
  }

  .drift.near {
    height: calc(11 * var(--u));
  }

  .pine {
    position: absolute;
    height: var(--h);
    aspect-ratio: 100 / 170;
    margin-left: calc(var(--h) * 100 / 170 / -2);
  }

  .cottage {
    position: absolute;
    left: 73.5%;
    bottom: calc(8 * var(--u));
    height: calc(27 * var(--u));
    aspect-ratio: 240 / 210;
  }

  /* Smoke from the chimney (its top sits at 74 % across, 10 % down the cottage). */
  .puff {
    position: absolute;
    left: 74%;
    top: 9%;
    width: calc(9 * var(--u));
    height: calc(9 * var(--u));
    margin: calc(-4.5 * var(--u)) 0 0 calc(-4.5 * var(--u));
    border-radius: 50%;
    background: radial-gradient(circle, rgba(206, 220, 244, 0.22) 0%, rgba(206, 220, 244, 0.09) 45%, transparent 70%);
    opacity: 0;
    animation: rise 10.2s linear infinite;
    will-change: transform, opacity;
  }

  .snowman {
    position: absolute;
    left: 18%;
    bottom: calc(6.5 * var(--u));
    height: calc(17 * var(--u));
    width: calc(17 * var(--u) * 100 / 140);
    margin-left: calc(17 * var(--u) * 100 / 140 / -2);
  }

  /* ---------- snowfall ---------- */

  /* Tiles of 100–600 px; the layer falls by 600 px per loop (a multiple of every tile height), so it never jumps. */
  .snow {
    position: absolute;
    inset: 0;
  }

  .flakes {
    position: absolute;
    left: -40px;
    right: -40px;
    top: -600px;
    height: calc(100% + 600px);
    animation: fall linear infinite;
    will-change: transform;
  }

  .snow.far {
    opacity: 0.55;
    animation: sway 9s ease-in-out infinite alternate;
  }

  .snow.far .flakes {
    background-image:
      radial-gradient(circle at 20% 30%, #fff 0 1.1px, transparent 1.8px),
      radial-gradient(circle at 70% 60%, #fff 0 1px, transparent 1.7px),
      radial-gradient(circle at 45% 80%, #e8f0ff 0 1.3px, transparent 2px),
      radial-gradient(circle at 85% 15%, #fff 0 0.9px, transparent 1.6px);
    background-size:
      137px 100px,
      211px 150px,
      173px 120px,
      251px 200px;
    animation-duration: 30s;
  }

  .snow.near {
    opacity: 0.8;
    animation: sway 6s ease-in-out infinite alternate-reverse;
  }

  .snow.near .flakes {
    background-image:
      radial-gradient(circle at 30% 20%, #fff 0 2px, transparent 3px),
      radial-gradient(circle at 75% 55%, #f2f6ff 0 2.4px, transparent 3.4px),
      radial-gradient(circle at 15% 75%, #fff 0 1.7px, transparent 2.6px);
    background-size:
      311px 300px,
      433px 600px,
      389px 200px;
    animation-duration: 19s;
  }

  /* ---------- motion ---------- */

  @keyframes twinkle {
    from {
      opacity: 0.15;
    }
    to {
      opacity: 0.5;
    }
  }

  @keyframes fall {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(0, 600px, 0);
    }
  }

  @keyframes sway {
    from {
      transform: translate3d(-18px, 0, 0);
    }
    to {
      transform: translate3d(18px, 0, 0);
    }
  }

  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate3d(0, 0, 0) scale(0.4);
    }
    15% {
      opacity: 0.7;
    }
    100% {
      opacity: 0;
      transform: translate3d(calc(5 * var(--u)), calc(-17 * var(--u)), 0) scale(1.7);
    }
  }

  /* Reduced motion: the snow hangs in the air, two soft puffs above the chimney. */
  @media (prefers-reduced-motion: reduce) {
    .stars.twinkle,
    .snow,
    .flakes,
    .puff {
      animation: none;
    }

    .puff {
      opacity: 0.55;
      transform: translate3d(calc(1.5 * var(--u)), calc(-5 * var(--u)), 0) scale(0.9);
    }

    .puff:nth-of-type(2) {
      opacity: 0.35;
      transform: translate3d(calc(3.5 * var(--u)), calc(-11 * var(--u)), 0) scale(1.3);
    }

    .puff:nth-of-type(3) {
      opacity: 0;
    }
  }
</style>
