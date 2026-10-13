<script lang="ts">
  // Christmas picker tile: the cottage with its lit window and string lights, a snowy pine, a thin moon and light
  // snowfall, kept to the right (the card's mini UI covers the lower left). One slow loop (the snow); still with
  // reduced motion.
  import Cottage from "./Cottage.svelte";
  import Pine from "./Pine.svelte";

  const uid = $props.id();
</script>

<span class="tile">
  <svg class="sky" viewBox="0 0 160 100" preserveAspectRatio="xMaxYMid slice" focusable="false">
    <defs>
      <mask id="{uid}-crescent">
        <circle cx="134" cy="20" r="9" fill="#fff" />
        <circle cx="137.6" cy="17.8" r="8" fill="#000" />
      </mask>
      <radialGradient id="{uid}-halo">
        <stop offset="0%" stop-color="#dce6ff" stop-opacity="0.22" />
        <stop offset="100%" stop-color="#dce6ff" stop-opacity="0" />
      </radialGradient>
    </defs>
    <circle cx="134" cy="20" r="24" fill="url(#{uid}-halo)" />
    <circle cx="134" cy="20" r="9" fill="#f3f1e4" mask="url(#{uid}-crescent)" />
    <path d="M0 100V86C40 80 80 82 112 86S150 84 160 82V100z" fill="#2c4470" />
    <path d="M0 100V94C40 90 90 92 120 94S150 92 160 91V100z" fill="#8ea3c6" />
  </svg>
  <span class="pine"><Pine /></span>
  <span class="cottage"><Cottage still /></span>
  <span class="snow"></span>
</span>

<style>
  .tile {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .sky {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
  }

  .pine {
    position: absolute;
    right: -3%;
    bottom: 4%;
    height: 72%;
    aspect-ratio: 100 / 170;
  }

  .cottage {
    position: absolute;
    left: 63%;
    bottom: 5%;
    height: 40%;
    aspect-ratio: 240 / 210;
  }

  .snow {
    position: absolute;
    left: 0;
    right: 0;
    top: -120px;
    height: calc(100% + 120px);
    background-image:
      radial-gradient(circle at 30% 30%, rgba(255, 255, 255, 0.85) 0 1px, transparent 1.6px),
      radial-gradient(circle at 70% 70%, rgba(255, 255, 255, 0.7) 0 0.8px, transparent 1.4px);
    background-size:
      37px 40px,
      53px 60px;
    animation: fall 14s linear infinite;
  }

  @keyframes fall {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(0, 120px, 0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .snow {
      animation: none;
    }
  }
</style>
