<script lang="ts">
  // Summer picker tile: the sun half in the sea at the right, a shimmering glitter path and a palm leaning in from
  // the right edge. The card's mini UI sits on the left, so the art stays right. Static under reduced motion.
  import { PALM_FRONDS, PALM_TRUNK } from "./palm";

  const uid = $props.id();
</script>

<svg class="art" viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
  <defs>
    <radialGradient id="{uid}-sun" cx="50%" cy="45%" r="50%">
      <stop offset="0%" stop-color="#fff1c4" />
      <stop offset="55%" stop-color="#ffd27f" />
      <stop offset="100%" stop-color="#ff9f5a" />
    </radialGradient>
    <clipPath id="{uid}-sky">
      <rect width="160" height="66" />
    </clipPath>
  </defs>
  <circle cx="116" cy="70" r="17" fill="url(#{uid}-sun)" clip-path="url(#{uid}-sky)" />
  <rect x="70" y="58" width="44" height="1.4" rx="0.7" fill="#f29a72" opacity="0.35" />
  <g class="glint">
    <rect x="109" y="68" width="14" height="1" rx="0.5" fill="#ffd9a6" />
    <rect x="106" y="72" width="20" height="1" rx="0.5" fill="#ffd9a6" opacity="0.7" />
    <rect x="110" y="77" width="12" height="1" rx="0.5" fill="#ffd9a6" opacity="0.6" />
    <rect x="104" y="83" width="24" height="1" rx="0.5" fill="#ffd9a6" opacity="0.45" />
  </g>
  <g fill="none" stroke="#e2b0d8" stroke-width="0.8" stroke-linecap="round" opacity="0.45">
    <path d="M78 74q4-1.2 8 0M136 70q3-1 6 0M92 88q5-1.4 10 0M140 92q4-1.2 8 0" />
  </g>
  <g transform="translate(171 100) scale(-0.23 0.23) translate(-60 -420)">
    <path d={PALM_TRUNK} fill="#160d1f" />
    <path d={PALM_FRONDS} fill="#160d1f" />
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

  .glint {
    animation: shimmer 4s ease-in-out infinite;
  }

  @keyframes shimmer {
    0%,
    100% {
      opacity: 0.45;
    }
    50% {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .glint {
      animation: none;
      opacity: 0.8;
    }
  }
</style>
