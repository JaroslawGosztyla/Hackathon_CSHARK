<script lang="ts">
  // New Year picker tile: a skyline on the right, one firework that opens now and then, a still one, a few stars.
  // The card's mini UI sits on the left, so the art stays right and low. Static under reduced motion.
  const SPARKS = Array.from({ length: 14 }, (_, i) => {
    const a = (i / 14) * Math.PI * 2;
    return { x: Math.cos(a), y: Math.sin(a) };
  });
  const STARS = [
    [96, 10],
    [118, 22],
    [140, 8],
    [152, 30],
    [78, 6],
    [108, 40],
  ] as const;
</script>

<svg class="art" viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
  {#each STARS as [x, y], i (i)}
    <circle cx={x} cy={y} r="0.7" fill="#fff6e2" opacity="0.6" />
  {/each}
  <g transform="translate(118 30)">
    <g class="burst">
      {#each SPARKS as p, i (i)}
        <circle cx={p.x * 17} cy={p.y * 17} r="1.1" fill="#fbe7b0" />
        <circle cx={p.x * 11} cy={p.y * 11} r="0.7" fill="#f0c870" opacity="0.7" />
      {/each}
    </g>
  </g>
  <g transform="translate(146 52)" opacity="0.7">
    {#each SPARKS as p, i (i)}
      <circle cx={p.x * 9} cy={p.y * 9} r="0.8" fill="#ef9dbd" />
    {/each}
  </g>
  <path
    d="M80 100V84h8v-6h7v10h6V72h4v-4h2v4h4v28zM112 100V80h9v-8h8v12h6v-6h7v10h5V76h13v24z"
    fill="#10153a"
  />
  <path d="M86 100v-9h10v-5h9v8h7v-6h10v12z M126 100v-8h8v-4h9v6h6v-4h11v10z" fill="#05071a" />
  <g fill="#f3cf8a" opacity="0.6">
    <rect x="90" y="94" width="1.4" height="1.6" />
    <rect x="108" y="96" width="1.4" height="1.6" />
    <rect x="137" y="91" width="1.4" height="1.6" />
    <rect x="152" y="95" width="1.4" height="1.6" />
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

  .burst {
    transform-box: fill-box;
    transform-origin: center;
    animation: burst 6s cubic-bezier(0.2, 0.7, 0.3, 1) infinite;
  }

  @keyframes burst {
    0% {
      opacity: 0;
      transform: scale(0.2);
    }
    8% {
      opacity: 1;
      transform: scale(0.5);
    }
    40% {
      opacity: 0.75;
      transform: scale(1);
    }
    65%,
    100% {
      opacity: 0;
      transform: scale(1.08);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .burst {
      animation: none;
      opacity: 0.85;
    }
  }
</style>
