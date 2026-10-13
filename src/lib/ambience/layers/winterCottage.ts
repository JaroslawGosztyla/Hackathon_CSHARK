// Winter cottage (Christmas): the fire of the fireplace layer a little softer, a faint wind outside, and now and then
// (25–60 s) soft sleigh bells passing by: 6–10 small FM tinkles over about 1.5 s, quiet and high.
import { between } from "../rng";
import type { Kit } from "../kit";
import { fire } from "./fireplace";

/** One small bell: an FM tinkle (carrier 2.5–4.5 kHz, ratio 1.4–2.7) whose brightness and level fade together. */
function tinkle(kit: Kit, at: number, level: number, pan: number): void {
  const { rng } = kit;
  const freq = between(rng, 2500, 4500);
  const ratio = between(rng, 1.4, 2.7);
  const decay = between(rng, 0.12, 0.3);
  const carrier = kit.oneOsc("sine", freq);
  const env = kit.oneGain(0);
  env.gain.setValueAtTime(0, at);
  env.gain.linearRampToValueAtTime(level, at + 0.01);
  env.gain.exponentialRampToValueAtTime(level * 1e-3, at + 0.01 + decay);
  kit.shot(carrier, at, at + 0.05 + decay, [env, kit.oneFilter("highpass", 1800, 0.7), kit.onePan(pan)]);
  const modulator = kit.oneOsc("sine", freq * ratio);
  const depth = kit.oneGain(0);
  depth.gain.setValueAtTime(freq * 1.6, at);
  depth.gain.exponentialRampToValueAtTime(freq * 0.05, at + decay);
  kit.shot(modulator, at, at + 0.05 + decay, [depth], carrier.frequency);
}

/** Bells passing by: a few shakes, each 1–2 tinkles, louder in the middle. */
export function sleighBells(kit: Kit, at: number): number {
  const { rng } = kit;
  const shakes = 4 + Math.floor(rng() * 3);
  const step = between(rng, 0.22, 0.3);
  const startPan = between(rng, -0.6, 0.6);
  let t = at;
  for (let s = 0; s < shakes; s++) {
    const swell = 0.45 + 0.55 * Math.sin((Math.PI * (s + 0.5)) / shakes);
    const pan = startPan + (s / Math.max(1, shakes - 1)) * -startPan * 1.4;
    const tinkles = 1 + Math.floor(rng() * 2);
    for (let i = 0; i < tinkles; i++) tinkle(kit, t + i * between(rng, 0.02, 0.045), between(rng, 0.024, 0.04) * swell, pan);
    t += step + between(rng, -0.03, 0.03);
  }
  return t + 0.4;
}

export function winterCottage(kit: Kit): void {
  const { buffers: b, rng } = kit;
  fire(kit, 0.85);
  const outside = kit.gain(0.2);
  kit.chain(kit.loop(b.brown), kit.filter("bandpass", 420, 1.2), kit.filter("lowpass", 700, 0.5), outside, kit.bus);
  kit.lfo(0.05, 0.1, outside.gain);
  kit.every({ min: 25, max: 60, first: between(rng, 8, 15) }, (at) => sleighBells(kit, at));
}
