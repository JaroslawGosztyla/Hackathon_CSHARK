// Underwater art shared by the scene and the picker tile: the project's own flat shapes (generated SVG paths).

/** A kelp strand in an 80 × 400 box (base at the bottom centre): a wavy stalk and blades along it. */
export interface Kelp {
  stalk: string;
  blades: string;
  bladesDark: string;
}

const f = (n: number): string => n.toFixed(1);

/** `seed` varies the wiggle and the blades; `top` (0..1) is where the strand ends (0 = full height). */
export function kelp(seed: number, top = 0.02): Kelp {
  const end = 400 * top;
  const span = 400 - end;
  const sway = (i: number) => Math.sin(seed * 1.7 + i * 1.3) * 9;
  const points: [number, number][] = [];
  for (let i = 0; i <= 10; i++) points.push([40 + sway(i), 400 - (span * i) / 10]);
  let stalk = `M40 400`;
  for (let i = 1; i < points.length; i++) {
    const [px, py] = points[i - 1] ?? [40, 400];
    const [x, y] = points[i] ?? [40, end];
    stalk += `C${f(px)} ${f(py - span / 24)} ${f(x)} ${f(y + span / 24)} ${f(x)} ${f(y)}`;
  }
  let blades = "";
  let bladesDark = "";
  for (let i = 1; i < points.length; i++) {
    const [x, y] = points[i] ?? [40, end];
    const side = (i + seed) % 2 === 0 ? 1 : -1;
    const len = 30 + ((seed * 7 + i * 5) % 16);
    const lift = 26 + ((seed * 3 + i * 11) % 18);
    const blade =
      `M${f(x)} ${f(y)}` +
      `c${f(side * len * 0.4)} ${f(-lift * 0.1)} ${f(side * len * 0.85)} ${f(-lift * 0.45)} ${f(side * len)} ${f(-lift)}` +
      `c${f(-side * len * 0.3)} ${f(lift * 0.25)} ${f(-side * len * 0.7)} ${f(lift * 0.7)} ${f(-side * len)} ${f(lift)}z`;
    if (i % 3 === 0) bladesDark += blade;
    else blades += blade;
  }
  return { stalk, blades, bladesDark };
}

/** A small fish (viewBox 0 0 40 20), facing right. */
export const FISH = "M32 10c-4-6-13-8-21-4L3 1l2 9-2 9 8-5c8 4 17 2 21-4z";

/** A five-armed starfish with round tips (viewBox -10 -10 20 20). */
export const STARFISH = (() => {
  let d = "";
  for (let i = 0; i < 5; i++) {
    const a = (-90 + i * 72) * (Math.PI / 180);
    const b = (-90 + i * 72 + 36) * (Math.PI / 180);
    const tip = [Math.cos(a) * 9.5, Math.sin(a) * 9.5];
    const valley = [Math.cos(b) * 3.6, Math.sin(b) * 3.6];
    d += `${i === 0 ? "M" : "L"}${f(tip[0] ?? 0)} ${f(tip[1] ?? 0)}L${f(valley[0] ?? 0)} ${f(valley[1] ?? 0)}`;
  }
  return `${d}Z`;
})();
