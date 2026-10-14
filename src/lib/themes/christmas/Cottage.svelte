<script lang="ts">
  // A snowed-in cottage with a warm window, a wreath on the door and string lights under the roof (viewBox 0 0 240 210).
  // The bulbs twinkle slowly (opacity 0.6–1, staggered); still with reduced motion or `still`.
  import { BULBS } from "./art";

  let { still = false }: { still?: boolean } = $props();

  const uid = $props.id();

  /** Bulbs hanging under the roof edge: x, y, colour index, twinkle length (s), delay (s). */
  const LIGHTS: readonly (readonly [number, number, number, number, number])[] = [
    [36, 99, 0, 4.2, -0.6],
    [60, 82, 1, 3.4, -2.1],
    [84, 65, 2, 5.1, -1.2],
    [108, 49, 3, 3.8, -3.0],
    [134, 49, 4, 4.6, -0.2],
    [158, 65, 0, 3.2, -1.7],
    [182, 82, 2, 5.6, -2.6],
    [206, 99, 1, 4.0, -0.9],
  ];

  /** The wire sags between neighbouring bulbs. */
  const wire = LIGHTS.reduce((d, [x, y], i) => {
    if (i === 0) return `M22 106L${x} ${y - 4}`;
    const [px, py] = LIGHTS[i - 1] ?? [x, y];
    return `${d}Q${(px + x) / 2} ${(py + y) / 2 + 3} ${x} ${y - 4}`;
  }, "");
</script>

<svg viewBox="0 0 240 210" class:still focusable="false" aria-hidden="true">
  <defs>
    <linearGradient id="{uid}-glass" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#ffe2a0" />
      <stop offset="100%" stop-color="#ffb554" />
    </linearGradient>
    <radialGradient id="{uid}-spill">
      <stop offset="0%" stop-color="#ffbf6a" stop-opacity="0.42" />
      <stop offset="100%" stop-color="#ffbf6a" stop-opacity="0" />
    </radialGradient>
    {#each BULBS as color, i (i)}
      <radialGradient id="{uid}-bulb{i}">
        <stop offset="0%" stop-color={color} stop-opacity="0.85" />
        <stop offset="100%" stop-color={color} stop-opacity="0" />
      </radialGradient>
    {/each}
  </defs>

  <!-- Chimney (behind the roof), walls with planks, the roof. -->
  <path d="M164 26h26v52h-26z" fill="#2a2234" />
  <path d="M160 20h34q3 0 3 4v4q0 3-3 3h-34q-3 0-3-3v-4q0-4 3-4z" fill="#c9d6ea" />
  <path d="M28 96L120 32 212 96V206H28z" fill="#1c2135" />
  <g stroke="#11141f" stroke-opacity="0.6" stroke-width="1.4">
    <path d="M28 112h184M28 128h184M28 144h184M28 160h184M28 176h184M28 192h184" />
  </g>
  <path d="M-2 104L120 16 242 104 228 106 120 30 12 106z" fill="#141826" />

  <!-- Warm light from the window, then the window: frame, glass, bars, a snowy sill. -->
  <ellipse cx="77" cy="140" rx="74" ry="56" fill="url(#{uid}-spill)" />
  <path d="M46 116h62v52H46z" fill="#2c2f45" />
  <path d="M51 121h52v42H51z" fill="url(#{uid}-glass)" />
  <path d="M75.5 121h3v42h-3zM51 140.5h52v3H51z" fill="#2c2f45" />
  <path d="M42 168h70q4 0 4 3.5t-4 3.5H42q-4 0-4-3.5t4-3.5z" fill="#dbe5f3" />

  <!-- Door with a wreath. -->
  <path d="M138 132h42v74h-42z" fill="#3a2a36" />
  <path d="M141 135h36v68h-36z" fill="#45323f" />
  <circle cx="171" cy="172" r="2" fill="#e8c27a" />
  <circle cx="159" cy="152" r="9" fill="none" stroke="#2f6b4a" stroke-width="5" />
  <g fill="#ff8a8a">
    <circle cx="152" cy="147" r="1.6" />
    <circle cx="166" cy="149" r="1.6" />
    <circle cx="163" cy="160" r="1.6" />
  </g>
  <path d="M154 161l5 3 5-3-2 6-3-2-3 2z" fill="#d9606a" />

  <!-- Snow on the roof, thick at the eaves, and drifts against the walls. -->
  <path
    d="M-6 102L120 4 246 102q2 7-6 7-6 4-12-1L120 22 12 108q-6 5-12 1-8 0-6-7z"
    fill="#d3deee"
  />
  <path d="M120 4L246 102q2 7-6 7-6 4-12-1L120 22z" fill="#b6c5de" />
  <path d="M10 210c6-14 20-20 40-18 24 2 40-6 70-4 30 2 48 10 80 6 18-2 30 4 36 16z" fill="#c6d3e8" />

  <!-- String lights. -->
  <path d={wire} fill="none" stroke="#0e1220" stroke-width="1.2" />
  {#each LIGHTS as [x, y, c, dur, delay], i (i)}
    <circle
      class="halo"
      cx={x}
      cy={y + 3}
      r="9"
      fill="url(#{uid}-bulb{c})"
      style:animation-duration="{dur}s"
      style:animation-delay="{delay}s"
    />
    <ellipse cx={x} cy={y + 3} rx="2.6" ry="3.6" fill={BULBS[c]} />
    <rect x={x - 1.5} y={y - 2.4} width="3" height="2.6" rx="0.6" fill="#1b1e2c" />
  {/each}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .halo {
    animation: twinkle ease-in-out infinite alternate;
  }

  .still .halo {
    animation: none;
  }

  @keyframes twinkle {
    from {
      opacity: 0.6;
    }
    to {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .halo {
      animation: none;
    }
  }
</style>
