// Café murmur: four band-passed pink-noise "voices" (300 Hz–2.5 kHz) whose levels wander like distant talk, a low
// room tone, and now and then (8–20 s) a quiet cup clink. No words, nothing close.
import { between } from "../rng";
import { kRate, type Kit } from "../kit";

/** Level trim of the murmur and the room (measured offline: peak ≈ −12 dBFS at volume 1). */
const TRIM_DB = -7.5;
const VOICE_CENTRES = [380, 720, 1250, 2150] as const;

function clink(kit: Kit, at: number): void {
  const { rng } = kit;
  const freq = between(rng, 2500, 4000);
  const decay = between(rng, 0.18, 0.3);
  const pan = between(rng, -0.7, 0.7);
  // Two inharmonic partials (a small cup): quiet, with a soft 12 ms onset so it never ticks.
  for (const [ratio, level] of [
    [1, 0.022],
    [2.76, 0.009],
  ] as const) {
    const osc = kit.oneOsc("sine", freq * ratio);
    const env = kit.oneGain(0);
    env.gain.setValueAtTime(0, at);
    env.gain.linearRampToValueAtTime(level, at + 0.012);
    env.gain.exponentialRampToValueAtTime(level * 1e-3, at + 0.012 + decay);
    kit.shot(osc, at, at + 0.05 + decay, [env, kit.onePan(pan)]);
  }
}

export function cafe(kit: Kit): void {
  const { buffers: b, rng } = kit;
  const room = kit.filter("lowpass", 3000, 0.6);
  const roomGain = kit.gain(1);
  const out = kit.trim(TRIM_DB);
  kit.chain(room, roomGain, out);

  const voices = VOICE_CENTRES.map((centre, i) => {
    const band = kit.filter("bandpass", centre * between(rng, 0.9, 1.1), 2.2);
    const level = kit.gain(0.5);
    kRate(level.gain);
    kit.chain(kit.loop(i % 2 === 0 ? b.pink : b.white), band, level, room);
    return level;
  });

  // Room tone: a soft low hum of the place.
  kit.chain(kit.loop(b.brown), kit.filter("lowpass", 260, 0.7), kit.gain(0.55), out);

  // Talk: every 0.25–0.6 s each voice glides to a new level (syllable-like, never in step).
  kit.every({ min: 0.25, max: 0.6, first: 0.05 }, (at) => {
    for (const level of voices) {
      const target = rng() < 0.18 ? between(rng, 0.05, 0.2) : between(rng, 0.45, 1.25);
      level.gain.setTargetAtTime(target, at + between(rng, 0, 0.2), between(rng, 0.06, 0.16));
    }
  });
  kit.every({ min: 8, max: 20, first: between(rng, 3, 8) }, (at) => clink(kit, at));
}
