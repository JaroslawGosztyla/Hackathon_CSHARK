<script lang="ts">
  // Underwater picker tile: a light ray, kelp on the right, the little cave rock on the sand and a bubble rising, kept
  // to the right (the card's mini UI covers the lower left). Two slow loops (kelp sway, the bubble); still with reduced
  // motion.
  import { STARFISH, kelp } from "./art";

  const BIG = kelp(5);
  const SMALL = kelp(2);
</script>

<span class="tile">
  <span class="ray"></span>
  <svg class="floor" viewBox="0 0 160 100" preserveAspectRatio="xMaxYMid slice" focusable="false">
    <path d="M0 100V90C40 86 80 88 120 90S150 88 160 87V100z" fill="#2f5764" />
    <svg x="122" y="28" width="16" height="66" viewBox="0 0 80 400" preserveAspectRatio="none">
      <path d={SMALL.stalk} fill="none" stroke="#194a52" stroke-width="5" />
      <path d={SMALL.blades + SMALL.bladesDark} fill="#1f5458" />
    </svg>
    <path d="M98 100c2-12 10-20 22-22 8-8 22-8 28 2 6 2 10 10 12 20z" fill="#12394f" />
    <path d="M121 100c1-7 5-11 9-11s8 4 9 11z" fill="#05141f" />
    <path d={STARFISH} transform="translate(108 95) rotate(14) scale(0.32)" fill="#ee9a7a" stroke="#ee9a7a" stroke-width="3" stroke-linejoin="round" />
    <g fill="none" stroke="#bdeefa" stroke-opacity="0.55" stroke-width="0.6">
      <circle cx="140" cy="40" r="1.6" />
      <circle cx="146" cy="30" r="1.1" />
      <circle cx="112" cy="54" r="1.3" />
    </g>
  </svg>
  <span class="kelp">
    <svg viewBox="0 0 80 400" preserveAspectRatio="xMidYMax meet" focusable="false">
      <path d={BIG.stalk} fill="none" stroke="#1f5a4a" stroke-width="5" stroke-linecap="round" />
      <path d={BIG.blades} fill="#2c7258" />
      <path d={BIG.bladesDark} fill="#215e4a" />
    </svg>
  </span>
  <span class="bubble"></span>
</span>

<style>
  .tile {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  svg {
    display: block;
  }

  .floor {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .ray {
    position: absolute;
    top: -20%;
    left: 66%;
    width: 22%;
    height: 130%;
    transform: rotate(18deg);
    transform-origin: 50% 0;
    background: linear-gradient(180deg, rgba(190, 245, 255, 0.3) 0%, rgba(190, 245, 255, 0.08) 55%, transparent 90%);
    mask-image: linear-gradient(90deg, transparent 0%, #000 35%, #000 65%, transparent 100%);
  }

  .kelp {
    position: absolute;
    right: 3%;
    bottom: 4%;
    height: 86%;
    aspect-ratio: 80 / 400;
    transform-origin: 50% 100%;
    animation: sway 7s ease-in-out infinite alternate;
  }

  .kelp svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .bubble {
    position: absolute;
    left: 80%;
    bottom: 22%;
    width: 4%;
    aspect-ratio: 1;
    border-radius: 50%;
    background:
      radial-gradient(circle at 34% 30%, rgba(255, 255, 255, 0.8) 0 14%, transparent 18%),
      radial-gradient(circle, transparent 0 52%, rgba(190, 240, 255, 0.6) 64%, transparent 74%);
    animation: rise 6s ease-in infinite;
  }

  @keyframes sway {
    from {
      transform: rotate(-4deg);
    }
    to {
      transform: rotate(4deg);
    }
  }

  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate(0, 0);
    }
    15% {
      opacity: 0.9;
    }
    100% {
      opacity: 0;
      transform: translate(30%, -1500%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .kelp,
    .bubble {
      animation: none;
    }

    .bubble {
      opacity: 0.85;
      transform: translate(0, -600%);
    }
  }
</style>
