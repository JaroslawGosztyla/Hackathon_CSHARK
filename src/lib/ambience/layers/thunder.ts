// Distant thunder: a faint low air bed and, every 25–70 s, a soft rumble (brown noise under 160 Hz) that rolls in
// over 0.6–1.5 s and fades over 3–6 s, panned a little to one side. No crack, never a sudden peak.
import { between } from "../rng";
import type { Kit } from "../kit";
import { swell } from "./shape";

/** Envelope peak of a rumble on the thunder layer (measured offline: peaks ≈ −12 dBFS at volume 1). */
export const RUMBLE_PEAK = 0.34;
/** The air between rumbles (≈ −30 dBFS RMS). */
const AIR_LEVEL = 0.18;

/** One rumble at `at` with envelope peak `peak` (also the spooky night's distant thunder). Returns when it ends. */
export function rumble(kit: Kit, at: number, peak: number, into: AudioNode = kit.bus): number {
  const { rng } = kit;
  const attack = between(rng, 0.6, 1.5);
  const decay = between(rng, 3, 6);
  const source = kit.oneNoise(kit.buffers.brown, between(rng, 0.55, 0.85));
  const low = kit.oneFilter("lowpass", 160, 0.8);
  const lower = kit.oneFilter("lowpass", 320, 0.5);
  const env = kit.oneGain(0);
  const end = swell(env.gain, at, attack, peak, decay);
  // Thunder rolls: now and then the level holds a moment longer inside the swell.
  if (rng() < 0.6) {
    const roll = at + attack + between(rng, 0.5, 1.6);
    env.gain.linearRampToValueAtTime(peak * between(rng, 0.65, 0.9), roll + 0.5);
  }
  kit.shot(source, at, end, [low, lower, env, kit.onePan(between(rng, -0.4, 0.4))], into);
  return end;
}

export function thunder(kit: Kit): void {
  const air = kit.gain(AIR_LEVEL);
  kit.chain(kit.loop(kit.buffers.brown), kit.filter("lowpass", 110, 0.7), air, kit.bus);
  kit.lfo(0.03, AIR_LEVEL * 0.35, air.gain);
  kit.every({ min: 25, max: 70, first: between(kit.rng, 4, 10) }, (at) => rumble(kit, at, RUMBLE_PEAK));
}
