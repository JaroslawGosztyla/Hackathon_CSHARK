<script lang="ts">
  // Valentine's picker tile: dusk hills with the heart-crowned tree on the right, a thin moon and two hearts that
  // float up. The card's mini UI sits on the left, so the art stays right and low. Static under reduced motion.
  const HEART =
    "M12 20.6C12 20.6 3 15.2 3 9.1 3 6.1 5.2 4 7.8 4c1.9 0 3.4 1.1 4.2 2.6C12.8 5.1 14.3 4 16.2 4 18.8 4 21 6.1 21 9.1c0 6.1-9 11.5-9 11.5z";
</script>

<svg class="art" viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
  <path d="M140 12a8 8 0 1 0 0 16 9.6 9.6 0 0 1 0-16z" fill="#ffe6ee" opacity="0.85" />
  <path d="M0 100V84c30-4 60-2 90 0s50-6 70-4v20z" fill="#3b1840" />
  <path d="M60 100V92c22-4 40-14 62-16 14-1 26 4 38 8v16z" fill="#230f2b" />
  <g fill="#230f2b">
    <rect x="125.4" y="66" width="2.2" height="12" rx="1" />
    <path d="M126.5 68c0 0-11-6.4-11-14.2 0-3.6 2.7-6.4 6-6.4 2.3 0 4.1 1.4 5 3.2.9-1.8 2.7-3.2 5-3.2 3.3 0 6 2.8 6 6.4 0 7.8-11 14.2-11 14.2z" />
  </g>
  <g class="float one">
    <path d={HEART} transform="translate(100 70) scale(0.42)" fill="#ff97ba" />
  </g>
  <g class="float two">
    <path d={HEART} transform="translate(146 58) scale(0.32)" fill="#f7b4cc" />
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

  .float {
    opacity: 0;
    animation: float 7s ease-in-out infinite;
  }

  .two {
    animation-duration: 9s;
    animation-delay: -4s;
  }

  @keyframes float {
    0% {
      opacity: 0;
      transform: translate(0, 6px);
    }
    20% {
      opacity: 0.8;
    }
    70% {
      opacity: 0.7;
    }
    100% {
      opacity: 0;
      transform: translate(2px, -46px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .float {
      animation: none;
      opacity: 0.75;
      transform: translate(0, -18px);
    }
  }
</style>
