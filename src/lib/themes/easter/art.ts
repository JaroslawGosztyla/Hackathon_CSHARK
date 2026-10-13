// Easter art shared by the scene and the picker tile: the project's own flat shapes.
import type { EggSpec } from "./Egg.svelte";

/** A five-petal blossom: [x, y, radius, rotation, white?]. */
export type Blossom = readonly [number, number, number, number, boolean];

/** Petal ellipses of a blossom of radius 1 (rotated by 72° each). */
export const PETAL_ANGLES = [0, 72, 144, 216, 288] as const;

export const PINK = { petal: "#f6b8cb", petalWhite: "#fff3f6", centre: "#e07f9f", stamen: "#f3c45a" } as const;

/** Eggs in the left clump (viewBox 0 0 240 120). */
export const EGGS_LEFT: readonly EggSpec[] = [
  { x: 92, y: 48, width: 36, tilt: -12, base: "#a9daf2", a: "#ffffff", b: "#f7c6d6", pattern: "wave" },
  { x: 52, y: 40, width: 40, tilt: 8, base: "#b8e6cf", a: "#ffffff", b: "#f6d36b", pattern: "flowers" },
  { x: 22, y: 66, width: 30, tilt: -6, base: "#f8c3d3", a: "#ffffff", b: "#c9b4ee", pattern: "dots" },
];

/** Eggs in the right clump, next to the bunny (viewBox 0 0 300 150). */
export const EGGS_RIGHT: readonly EggSpec[] = [
  { x: 92, y: 62, width: 42, tilt: -10, base: "#f6b9cb", a: "#ffffff", b: "#c6b0ef", pattern: "zigzag" },
  { x: 136, y: 80, width: 34, tilt: 12, base: "#cbb8f0", a: "#a8e0c8", b: "#ffffff", pattern: "stripes" },
  { x: 58, y: 92, width: 28, tilt: -4, base: "#f8e08c", a: "#f39ab4", b: "#ffffff", pattern: "dots" },
];

/** Grass blades along the bottom of a clump `width` wide (deterministic zigzag). */
export function grass(width: number, base: number, seed: number): string {
  let d = `M0 ${base}`;
  let x = 0;
  let i = seed;
  while (x < width) {
    i = (i * 9301 + 49297) % 233280;
    const h = 14 + (i / 233280) * 22;
    const lean = ((i % 7) - 3) * 1.6;
    d += `L${(x + 3 + lean).toFixed(1)} ${(base - h).toFixed(1)}L${(x + 6).toFixed(1)} ${base}`;
    x += 6;
  }
  return `${d}Z`;
}
