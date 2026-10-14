// Fireplace: a warm low bed (brown noise under 500 Hz) that breathes with the flames, the crackle texture read twice
// from different points (bright and mid), and a rare soft pop of a log (4–12 s). Also the base of the winter cottage.
import { between } from "../rng";
import type { Kit } from "../kit";
import { swell } from "./shape";

/** Level trim (measured offline: peak ≈ −12 dBFS at volume 1). */
const TRIM_DB = -13.5;

/** The fire at `level` (1 = the fireplace layer on its own). */
export function fire(kit: Kit, level = 1): void {
  const { buffers: b, rng } = kit;
  const out = kit.trim(TRIM_DB);
  const bed = kit.gain(1.25 * level);
  kit.chain(kit.loop(b.brown), kit.filter("lowpass", 500, 0.6), bed, out);
  kit.lfo(0.13, 0.3 * level, bed.gain);

  const crackle = kit.gain(0.55 * level);
  kit.chain(kit.loop(b.crackle), kit.filter("highpass", 900, 0.6), crackle, out);
  const embers = kit.gain(0.3 * level);
  kit.chain(kit.loop(b.crackle), kit.filter("bandpass", 2200, 0.7), embers, out);
  kit.lfo(0.21, 0.18 * level, crackle.gain);

  kit.every({ min: 4, max: 12, first: between(rng, 2, 6) }, (at) => {
    const env = kit.oneGain(0);
    const end = swell(env.gain, at, 0.02, 0.5 * level, between(rng, 0.12, 0.28));
    kit.shot(
      kit.oneNoise(b.white),
      at,
      end,
      [kit.oneFilter("bandpass", between(rng, 220, 600), 1.4), env, kit.onePan(between(rng, -0.5, 0.5))],
      out,
    );
  });
}

export function fireplace(kit: Kit): void {
  fire(kit);
}
