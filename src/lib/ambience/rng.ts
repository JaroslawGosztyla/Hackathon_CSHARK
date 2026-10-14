// A tiny seeded PRNG for the ambience layers (ADR-024): the variety of one start (when the thunder rolls, which owl
// hoots, how the crickets chirp) is deterministic for its seed, and nothing depends on `Math.random` in the timing.

export type Rng = () => number;

/** mulberry32: a fast 32-bit generator, uniform in [0, 1). Plenty for sound variety (not for anything secret). */
export function mulberry32(seed: number): Rng {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A value in [min, max). */
export function between(rng: Rng, min: number, max: number): number {
  return min + (max - min) * rng();
}

/** One of `items` (never empty). */
export function choose<T>(rng: Rng, items: readonly [T, ...T[]]): T {
  return items[Math.floor(rng() * items.length)] ?? items[0];
}

let seedCounter = 0;

/** A fresh seed per start (time + a counter, so two layers started in the same millisecond still differ). */
export function freshSeed(): number {
  seedCounter = (seedCounter + 0x9e3779b9) >>> 0;
  return (Date.now() ^ seedCounter) >>> 0;
}
