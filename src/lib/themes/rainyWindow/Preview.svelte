<script lang="ts">
  // Rainy window picker tile: warm and lilac city lights out of focus on the right, a few beads on the glass, rain
  // that falls slowly and one light that breathes. The card's mini UI sits on the left. Static under reduced motion.
  const uid = $props.id();

  const LIGHTS = [
    { x: 122, y: 30, r: 13, c: "#f0a86e", o: 0.4 },
    { x: 146, y: 52, r: 16, c: "#b9a2f0", o: 0.32 },
    { x: 104, y: 70, r: 10, c: "#ffd9a0", o: 0.36 },
    { x: 138, y: 86, r: 12, c: "#7fcfd0", o: 0.3 },
    { x: 84, y: 92, r: 9, c: "#e88fb4", o: 0.3 },
  ];
  const BEADS = [
    [96, 22, 1.1],
    [112, 48, 0.8],
    [130, 64, 1.2],
    [152, 18, 0.9],
    [150, 76, 1],
    [88, 60, 0.7],
  ] as const;
</script>

<svg class="art" viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
  <defs>
    {#each LIGHTS as l, i (i)}
      <radialGradient id="{uid}-b{i}">
        <stop offset="0%" stop-color={l.c} stop-opacity="0.6" />
        <stop offset="78%" stop-color={l.c} stop-opacity="0.5" />
        <stop offset="90%" stop-color={l.c} stop-opacity="0.7" />
        <stop offset="100%" stop-color={l.c} stop-opacity="0" />
      </radialGradient>
    {/each}
    <pattern id="{uid}-rain" width="24" height="40" patternUnits="userSpaceOnUse">
      <line x1="4" y1="2" x2="4" y2="12" stroke="#d4e2ff" stroke-width="0.5" />
      <line x1="15" y1="20" x2="15" y2="28" stroke="#d4e2ff" stroke-width="0.5" />
      <line x1="20" y1="6" x2="20" y2="13" stroke="#d4e2ff" stroke-width="0.5" />
    </pattern>
  </defs>
  {#each LIGHTS as l, i (i)}
    <circle class:breathe={i === 1} cx={l.x} cy={l.y} r={l.r} fill="url(#{uid}-b{i})" opacity={l.o} />
  {/each}
  <g transform="rotate(7 80 50)">
    <g class="rain">
      <rect x="-20" y="-60" width="200" height="180" fill="url(#{uid}-rain)" opacity="0.28" />
    </g>
  </g>
  {#each BEADS as [x, y, r], i (i)}
    <circle cx={x} cy={y} r={r} fill="#dbe6fb" opacity="0.5" />
  {/each}
  <rect x="0" y="96" width="160" height="4" fill="#06080e" />
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

  .rain {
    animation: fall 1.4s linear infinite;
  }

  .breathe {
    animation: breathe 7s ease-in-out infinite alternate;
  }

  @keyframes fall {
    from {
      transform: translateY(0);
    }
    to {
      transform: translateY(40px);
    }
  }

  @keyframes breathe {
    from {
      opacity: 0.18;
    }
    to {
      opacity: 0.36;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .rain {
      animation: none;
      display: none;
    }
    .breathe {
      animation: none;
    }
  }
</style>
