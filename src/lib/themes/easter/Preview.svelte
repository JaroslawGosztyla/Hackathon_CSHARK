<script lang="ts">
  // Easter picker tile: a blossom twig in the upper right, soft hills and two painted eggs in the grass, kept to the
  // right (the card's mini UI covers the lower left). One slow loop (a falling petal); still with reduced motion.
  import { PETAL_ANGLES, PINK, grass, type Blossom } from "./art";
  import Egg from "./Egg.svelte";

  const BLOSSOMS: readonly Blossom[] = [
    [146, 10, 6, 10, false],
    [132, 16, 5, 40, true],
    [118, 20, 5.4, 0, false],
    [104, 26, 4.6, 20, true],
    [126, 30, 4.2, 12, false],
  ];
  const GRASS = grass(160, 100, 11);
</script>

<span class="tile">
  <svg viewBox="0 0 160 100" preserveAspectRatio="xMaxYMid slice" focusable="false">
    <path d="M0 100V70C30 62 60 64 90 72S140 70 160 64V100z" fill="#d2e7c1" />
    <path d="M0 100V82C40 78 80 80 110 84S146 82 160 80V100z" fill="#bcdc9e" />
    <g fill="none" stroke="#7d5d57" stroke-linecap="round">
      <path d="M164 6C150 10 136 14 120 20 110 24 100 28 92 34" stroke-width="2.6" />
      <path d="M128 18c-2 6 0 10-4 14" stroke-width="1.4" />
    </g>
    {#each BLOSSOMS as [x, y, r, rot, white], i (i)}
      <g transform="translate({x} {y}) rotate({rot}) scale({r})">
        {#each PETAL_ANGLES as a (a)}
          <ellipse cx="0" cy="-0.52" rx="0.4" ry="0.56" transform="rotate({a})" fill={white ? PINK.petalWhite : PINK.petal} />
        {/each}
        <circle r="0.24" fill={PINK.centre} />
      </g>
    {/each}
    <Egg egg={{ x: 116, y: 72, width: 14, tilt: -10, base: "#f6b9cb", a: "#ffffff", b: "#c6b0ef", pattern: "zigzag" }} />
    <Egg egg={{ x: 132, y: 78, width: 12, tilt: 12, base: "#a9daf2", a: "#ffffff", b: "#f7c6d6", pattern: "wave" }} />
    <path d={GRASS} fill="#86bf67" transform="matrix(1 0 0 .5 0 50)" />
  </svg>
  <span class="petal"></span>
</span>

<style>
  .tile {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  svg {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
  }

  .petal {
    position: absolute;
    left: 74%;
    top: 30%;
    width: 4%;
    aspect-ratio: 10 / 7;
    border-radius: 80% 0 80% 0;
    background: #f4adc3;
    animation: petal 9s ease-in-out infinite;
  }

  @keyframes petal {
    0% {
      opacity: 0;
      transform: translate(0, 0) rotate(0deg);
    }
    15% {
      opacity: 0.9;
    }
    85% {
      opacity: 0.9;
    }
    100% {
      opacity: 0;
      transform: translate(-160%, 600%) rotate(300deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .petal {
      animation: none;
      opacity: 0.85;
      transform: translate(-60%, 220%) rotate(120deg);
    }
  }
</style>
