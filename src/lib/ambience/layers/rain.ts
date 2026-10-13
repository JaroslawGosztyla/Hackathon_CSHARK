// Rain (free in the beta): a steady pink-noise bed (400 Hz–7 kHz) and two droplet loops from different points,
// filtered apart so the 4 s texture never sounds like a loop; the intensity wanders slowly.
import type { Kit } from "../kit";

/** Level trim (measured offline: peak ≈ −12 dBFS at volume 1). */
const TRIM_DB = -4;

export function rain(kit: Kit): void {
  const { buffers: b } = kit;
  const out = kit.trim(TRIM_DB);
  const bed = kit.gain(0.62);
  kit.chain(kit.loop(b.pink), kit.filter("highpass", 400, 0.5), kit.filter("lowpass", 7000, 0.5), bed, out);
  kit.lfo(0.045, 0.1, bed.gain);

  const drops = kit.gain(0.5);
  kit.chain(kit.loop(b.droplets), kit.filter("highpass", 1400, 0.6), drops, out);
  const farDrops = kit.gain(0.3);
  kit.chain(kit.loop(b.droplets), kit.filter("bandpass", 2600, 0.8), farDrops, out);
  kit.lfo(0.07, 0.12, drops.gain);
}
