// Wind (free in the beta): brown noise through a wide band-pass whose centre drifts 250–900 Hz (LFO 0.07 Hz), with
// gusts from a slow gain LFO (0.11 Hz) and a faint high whistle that follows them.
import type { Kit } from "../kit";

/** Level trim (measured offline: peak ≈ −12 dBFS at volume 1). */
const TRIM_DB = -11.5;

export function wind(kit: Kit): void {
  const { buffers: b } = kit;
  const out = kit.trim(TRIM_DB);
  const band = kit.filter("bandpass", 575, 0.8);
  kit.lfo(0.07, 325, band.frequency);
  const gust = kit.gain(1.9);
  kit.lfo(0.11, 0.75, gust.gain);
  kit.chain(kit.loop(b.brown), band, gust, out);

  const whistle = kit.filter("bandpass", 1150, 6);
  kit.lfo(0.05, 260, whistle.frequency);
  const whistleGain = kit.gain(0.35);
  kit.lfo(0.11, 0.3, whistleGain.gain);
  kit.chain(kit.loop(b.pink), whistle, whistleGain, out);
}
