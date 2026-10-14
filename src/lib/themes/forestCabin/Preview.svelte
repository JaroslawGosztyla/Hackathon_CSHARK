<script lang="ts">
  // Forest cabin picker tile: pines and the cabin with its warm window on the right, a puff of chimney smoke and two
  // fireflies. The card's mini UI sits on the left, so the art stays right and low. Static under reduced motion.
  const uid = $props.id();

  /** A small pine: three tiers on a trunk, `h` tall from `base`. */
  function pine(cx: number, base: number, h: number): string {
    const w = h * 0.46;
    let d = `M${cx - 0.6} ${base}V${base - h * 0.12}H${cx + 0.6}V${base}Z`;
    for (let i = 0; i < 3; i++) {
      const top = base - h + i * h * 0.24;
      const bottom = top + h * 0.42;
      const half = (w / 2) * (0.45 + (0.55 * (i + 1)) / 3);
      d += `M${cx} ${top}L${cx + half} ${bottom}L${cx - half} ${bottom}Z`;
    }
    return d;
  }
  const BACK = [72, 82, 92, 104, 150, 158].map((x, i) => pine(x, 90, 18 + ((i * 7) % 10))).join("");
  const FRONT = [141, 152].map((x, i) => pine(x, 102, 34 + i * 8)).join("");
</script>

<svg class="art" viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
  <defs>
    <radialGradient id="{uid}-glow">
      <stop offset="0%" stop-color="#ffb862" stop-opacity="0.55" />
      <stop offset="100%" stop-color="#ffb862" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{uid}-smoke">
      <stop offset="0%" stop-color="#c9d6cf" stop-opacity="0.9" />
      <stop offset="100%" stop-color="#c9d6cf" stop-opacity="0" />
    </radialGradient>
  </defs>
  <path d="M0 100V70Q30 58 60 66T120 60Q140 56 160 62V100Z" fill="#10292b" />
  <path d={BACK} fill="#091817" />
  <path d="M60 100C80 92 100 84 118 84S150 88 160 92V100Z" fill="#061010" />
  <!-- Cabin -->
  <rect x="122" y="66" width="5" height="12" fill="#060d0d" />
  <circle class="puff" cx="124.5" cy="62" r="4" fill="url(#{uid}-smoke)" />
  <rect x="104" y="74" width="30" height="13" fill="#0c1716" />
  <path d="M100 75L119 62L138 75Z" fill="#060d0d" />
  <circle cx="125" cy="80" r="11" fill="url(#{uid}-glow)" />
  <rect x="121" y="77" width="8" height="6" fill="#ffc777" />
  <path d="M125 77V83M121 80H129" stroke="#0c1716" stroke-width="0.7" />
  <rect x="109" y="79" width="5" height="8" fill="#071010" />
  <path d={FRONT} fill="#030808" />
  <circle class="fly one" cx="96" cy="78" r="1" fill="#e9f7a0" />
  <circle class="fly two" cx="146" cy="56" r="0.9" fill="#e9f7a0" />
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

  .puff {
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
    animation: smoke 6s ease-out infinite;
  }

  .fly {
    animation: blink 4s ease-in-out infinite;
  }

  .fly.two {
    animation-delay: -2s;
  }

  @keyframes smoke {
    0% {
      opacity: 0;
      transform: translate(0, 0) scale(0.6);
    }
    15% {
      opacity: 0.4;
    }
    100% {
      opacity: 0;
      transform: translate(8px, -22px) scale(2.2);
    }
  }

  @keyframes blink {
    0%,
    100% {
      opacity: 0.15;
    }
    50% {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .puff {
      animation: none;
      opacity: 0.3;
      transform: translate(3px, -8px) scale(1.5);
    }
    .fly {
      animation: none;
      opacity: 0.8;
    }
  }
</style>
