<script lang="ts">
  // Halloween picker tile: the moon, two bats, the scarecrow's hill and a glowing jack-o'-lantern, kept to the right and
  // the top (the card's mini UI covers the lower left). Two slow loops (a wing beat, the lantern glow); still with
  // reduced motion.
  import { BAT_BODY, BAT_WINGS } from "./art";
  import Pumpkin from "./Pumpkin.svelte";

  const uid = $props.id();
</script>

<span class="tile">
  <svg class="sky" viewBox="0 0 160 100" preserveAspectRatio="xMaxYMid slice" focusable="false">
    <defs>
      <radialGradient id="{uid}-halo">
        <stop offset="0%" stop-color="#ffd696" stop-opacity="0.42" />
        <stop offset="45%" stop-color="#ffaa60" stop-opacity="0.12" />
        <stop offset="100%" stop-color="#ffaa60" stop-opacity="0" />
      </radialGradient>
      <radialGradient id="{uid}-moon" cx="42%" cy="40%" r="62%">
        <stop offset="0%" stop-color="#fff4da" />
        <stop offset="60%" stop-color="#f8dba6" />
        <stop offset="100%" stop-color="#eab774" />
      </radialGradient>
    </defs>
    <g fill="#fff0dc" opacity="0.6">
      <circle cx="62" cy="30" r="0.6" />
      <circle cx="88" cy="12" r="0.5" />
      <circle cx="152" cy="62" r="0.5" />
      <circle cx="104" cy="44" r="0.6" />
    </g>
    <circle cx="127" cy="27" r="34" fill="url(#{uid}-halo)" />
    <circle cx="127" cy="27" r="14.5" fill="url(#{uid}-moon)" />
    <g fill="#c88a52" opacity="0.18">
      <ellipse cx="122" cy="23" rx="4" ry="3" />
      <ellipse cx="131" cy="31" rx="4.6" ry="3.6" />
    </g>
    <!-- Hill with a tiny scarecrow. -->
    <path d="M86 100C100 92 116 78 132 74 144 71 154 76 160 80V100z" fill="#1d0e2e" />
    <g fill="#0b0512">
      <rect x="133.2" y="56" width="1.6" height="20" />
      <path d="M125 61.6l18-.8v1.4l-18 .8z" />
      <path d="M129.4 60h9.2l1.4 3.4-1.6.6.2 8-1.4-1-1 1.4-1.2-1.2-1.2 1.4-1.2-1.4-1.2 1.2-.2-8.4-1.6-.6z" />
      <circle cx="134" cy="56.6" r="2.6" />
      <ellipse cx="134" cy="54.2" rx="4.6" ry="0.9" />
      <path d="M131.4 54c.6-3 2-5 4.6-6-.6 1.8.4 2.8 1.2 3.2-.2 1-.2 1.8 0 2.8z" />
    </g>
    <path d="M0 100V94C30 90 60 92 90 95 120 98 140 94 160 92V100z" fill="#0e0717" />
  </svg>

  <span class="bat one">
    <svg class="wings" viewBox="0 0 100 50" focusable="false"><path d={BAT_WINGS} /></svg>
    <svg viewBox="0 0 100 50" focusable="false"><path d={BAT_BODY} /></svg>
  </span>
  <span class="bat two">
    <svg viewBox="0 0 100 50" focusable="false"><path d={BAT_WINGS} transform="matrix(1 0 0 .55 0 9)" /></svg>
    <svg viewBox="0 0 100 50" focusable="false"><path d={BAT_BODY} /></svg>
  </span>

  <span class="lantern">
    <span class="glow"></span>
    <span class="pumpkin"><Pumpkin face="classic" /></span>
  </span>
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

  .sky {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .bat {
    position: absolute;
    width: 12%;
    aspect-ratio: 2 / 1;
    color: #09040f;
  }

  .bat svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: currentColor;
    overflow: visible;
  }

  .bat.one {
    left: 58%;
    top: 14%;
  }

  .bat.two {
    left: 86%;
    top: 44%;
    width: 8%;
  }

  .wings {
    transform-origin: 50% 40%;
    animation: flap 0.5s ease-in-out infinite alternate;
  }

  .lantern {
    position: absolute;
    left: 68%;
    bottom: 3%;
    width: 13%;
    aspect-ratio: 120 / 100;
  }

  .pumpkin {
    position: absolute;
    inset: 0;
  }

  .glow {
    position: absolute;
    inset: -110%;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(255, 170, 70, 0.5) 0%, rgba(255, 128, 40, 0.18) 34%, transparent 66%);
    animation: flicker 3.2s ease-in-out infinite;
  }

  @keyframes flap {
    from {
      transform: scaleY(1);
    }
    to {
      transform: scaleY(-0.55);
    }
  }

  @keyframes flicker {
    0%,
    100% {
      opacity: 0.9;
    }
    30% {
      opacity: 1;
    }
    55% {
      opacity: 0.8;
    }
    80% {
      opacity: 0.96;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wings,
    .glow {
      animation: none;
    }
  }
</style>
