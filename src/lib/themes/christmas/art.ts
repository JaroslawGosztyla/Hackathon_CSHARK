// Christmas art shared by the scene and the picker tile: the project's own flat shapes (hand-written SVG paths).

/** A snowy pine in a 100 × 170 box (trunk at the bottom centre): tiers, the snow on them, the trunk. */
export interface Pine {
  tiers: string;
  snow: string;
  trunk: string;
}

/** Tiers from the top: [apex y, base y, half width]. */
const TIERS: readonly (readonly [number, number, number])[] = [
  [4, 52, 17],
  [26, 86, 26],
  [52, 120, 35],
  [82, 154, 44],
];

const r = (n: number): string => n.toFixed(1);

function tier([apex, base, w]: readonly [number, number, number]): string {
  const h = base - apex;
  return (
    `M50 ${r(apex)}` +
    `Q${r(50 + w * 0.42)} ${r(apex + h * 0.62)} ${r(50 + w)} ${r(base)}` +
    `L${r(50 + w * 0.66)} ${r(base - 5)}L${r(50 + w * 0.36)} ${r(base + 1)}L50 ${r(base - 4)}` +
    `L${r(50 - w * 0.36)} ${r(base + 1)}L${r(50 - w * 0.66)} ${r(base - 5)}L${r(50 - w)} ${r(base)}` +
    `Q${r(50 - w * 0.42)} ${r(apex + h * 0.62)} 50 ${r(apex)}Z`
  );
}

/** The snow on a tier: its upper part, with a soft scalloped lower edge (lit from the upper left). */
function snowCap([apex, base, w]: readonly [number, number, number]): string {
  const h = base - apex;
  const y = apex + h * 0.36;
  const x = w * 0.38;
  return (
    `M50 ${r(apex)}` +
    `Q${r(50 + x * 0.7)} ${r(apex + h * 0.3)} ${r(50 + x)} ${r(y)}` +
    `Q${r(50 + x * 0.6)} ${r(y + 5)} ${r(50 + x * 0.3)} ${r(y - 1)}` +
    `Q50 ${r(y + 6)} ${r(50 - x * 0.35)} ${r(y)}` +
    `Q${r(50 - x * 0.7)} ${r(y + 6)} ${r(50 - x)} ${r(y + 2)}` +
    `Q${r(50 - x * 0.7)} ${r(apex + h * 0.3)} 50 ${r(apex)}Z`
  );
}

export const PINE: Pine = {
  tiers: TIERS.map(tier).join(""),
  snow: TIERS.map(snowCap).join(""),
  trunk: "M45 146h10v24H45z",
};

/** Bulb colours of the string lights (soft, never pure red or white). */
export const BULBS = ["#ffd36b", "#ff9a8a", "#8fd4a8", "#9fd8ff", "#ffb86b"] as const;
