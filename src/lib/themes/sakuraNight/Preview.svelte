<script lang="ts">
  // Sakura night picker tile: the big moon at the upper right, a blossoming branch reaching in from the right edge
  // and two petals drifting down. The card's mini UI sits on the left, so the art stays right. Static under reduced
  // motion.
  import { FLOWER_PETAL, PETAL } from "./branch";

  const uid = $props.id();

  const FLOWERS = [
    [150, 58, 1.2, "#f9c6dc"],
    [140, 64, 1, "#f3a9c9"],
    [128, 62, 1.15, "#ffd9e8"],
    [118, 70, 0.95, "#f9c6dc"],
    [136, 74, 0.9, "#eab0d6"],
    [108, 64, 0.85, "#f3a9c9"],
    [124, 80, 0.8, "#ffd9e8"],
    [156, 70, 1, "#eab0d6"],
  ] as const;
</script>

<svg class="art" viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
  <defs>
    <radialGradient id="{uid}-halo">
      <stop offset="35%" stop-color="#e8dcff" stop-opacity="0.3" />
      <stop offset="100%" stop-color="#e8dcff" stop-opacity="0" />
    </radialGradient>
    <path id="{uid}-petal" d={FLOWER_PETAL} />
    <g id="{uid}-flower">
      {#each [0, 72, 144, 216, 288] as a (a)}
        <use href="#{uid}-petal" transform="rotate({a})" />
      {/each}
      <circle r="1.7" fill="#ffe3a8" />
    </g>
  </defs>
  <circle cx="128" cy="28" r="26" fill="url(#{uid}-halo)" />
  <circle cx="128" cy="28" r="12" fill="#f6eef8" />
  <path d="M0 100V88C40 80 80 84 110 90S150 92 160 88V100Z" fill="#1d1738" />
  <g fill="none" stroke="#3a2340" stroke-linecap="round">
    <path d="M166 60C150 62 132 66 104 66" stroke-width="2.6" />
    <path d="M140 64C134 70 128 76 122 82" stroke-width="1.4" />
  </g>
  {#each FLOWERS as [x, y, s, c], i (i)}
    <use href="#{uid}-flower" transform="translate({x} {y}) scale({s * 0.42})" fill={c} />
  {/each}
  <g class="petal one">
    <path d={PETAL} transform="translate(98 40) scale(0.3)" fill="#f9c6dc" />
  </g>
  <g class="petal two">
    <path d={PETAL} transform="translate(146 82) scale(0.26)" fill="#f3a9c9" />
  </g>
</svg>

<style>
  .art {
    position: absolute;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
    display: block;
  }

  .petal {
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
    animation: fall 8s linear infinite;
  }

  .petal.two {
    animation-duration: 10s;
    animation-delay: -5s;
  }

  @keyframes fall {
    0% {
      opacity: 0;
      transform: translate(-10px, -16px) rotate(0deg);
    }
    15% {
      opacity: 0.9;
    }
    85% {
      opacity: 0.8;
    }
    100% {
      opacity: 0;
      transform: translate(18px, 26px) rotate(160deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .petal {
      animation: none;
      opacity: 0.85;
    }
  }
</style>
