// The cherry branch of the Sakura night theme (Scene and Preview), in a 600 × 420 box: the limb enters at the top-left
// corner and reaches right and down. Strokes taper by segment; blossoms are five-petal flowers in clusters at the
// twig ends and along the limb, placed with a seeded PRNG so they never reshuffle.
import { mulberry32 } from "$lib/components/backgrounds/random";

export interface BranchStroke {
  d: string;
  w: number;
}

/** A soft disc behind a cluster, so the blossoms read as a mass rather than single flowers. */
export interface Puff {
  x: number;
  y: number;
  r: number;
  o: number;
}

export interface Blossom {
  x: number;
  y: number;
  /** Rotation (degrees). */
  r: number;
  /** Scale of the 18-unit flower. */
  s: number;
  c: string;
  o: number;
}

export const BRANCH: BranchStroke[] = [
  { d: "M-30 6C40 30 110 60 170 92", w: 15 },
  { d: "M170 92C230 124 290 148 360 168", w: 10 },
  { d: "M360 168C410 182 460 190 520 186", w: 6 },
  { d: "M120 66C150 40 190 22 240 16", w: 7 },
  { d: "M240 16C270 12 300 18 330 30", w: 4 },
  { d: "M200 108C214 150 236 190 266 222", w: 6 },
  { d: "M266 222C280 238 300 252 318 258", w: 3.5 },
  { d: "M300 150C330 120 360 104 400 98", w: 4 },
  { d: "M440 182C470 210 490 236 500 262", w: 3 },
  { d: "M60 34C64 70 74 100 70 130", w: 4 },
  { d: "M520 186C545 180 565 168 585 150", w: 2.5 },
];

const CLUSTERS: [number, number, number][] = [
  // x, y, flowers
  [240, 18, 7],
  [330, 30, 6],
  [266, 222, 7],
  [318, 258, 5],
  [400, 98, 7],
  [500, 262, 5],
  [520, 186, 7],
  [585, 150, 4],
  [70, 130, 6],
  [170, 92, 4],
  [360, 168, 5],
  [300, 150, 4],
  [120, 66, 4],
  [440, 182, 6],
  [210, 140, 3],
  [190, 40, 3],
];

const COLORS = ["#f9c6dc", "#f3a9c9", "#ffd9e8", "#eab0d6", "#ffe6ef"] as const;

const rand = mulberry32(4141);

export const PUFFS: Puff[] = CLUSTERS.flatMap(([cx, cy, n]) =>
  Array.from({ length: n > 4 ? 2 : 1 }, () => ({
    x: cx + (rand() - 0.5) * 18,
    y: cy + (rand() - 0.5) * 12,
    r: 12 + n * 2.6 + rand() * 6,
    o: 0.12 + rand() * 0.08,
  })),
);

export const BLOSSOMS: Blossom[] = CLUSTERS.flatMap(([cx, cy, n]) =>
  Array.from({ length: n }, () => {
    const a = rand() * Math.PI * 2;
    const dist = 4 + rand() * (10 + n * 2.4);
    return {
      x: cx + Math.cos(a) * dist,
      y: cy + Math.sin(a) * dist * 0.8,
      r: rand() * 72,
      s: 1.25 + rand() * 0.85,
      c: COLORS[Math.floor(rand() * COLORS.length)] ?? COLORS[0],
      o: 0.85 + rand() * 0.15,
    };
  }),
);

/** One petal of a blossom, pointing up from the flower's centre (0, 0), with a small notch at its tip. */
export const FLOWER_PETAL = "M0 -1.2C2.6 -2.6 3.4 -6.4 1.6 -8.4L0 -7.4L-1.6 -8.4C-3.4 -6.4 -2.6 -2.6 0 -1.2Z";

/** A falling petal in a 20 × 20 box. */
export const PETAL = "M10 2.5C14.5 5.5 16 10.5 13 15.5c-.8 1.4-1.8 1.8-3 .9-1.2.9-2.2.5-3-.9C4 10.5 5.5 5.5 10 2.5z";
