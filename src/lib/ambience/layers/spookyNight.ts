// Spooky night (Halloween): a howling wind (a narrow band of noise gliding 300–700 Hz), a faint low drone that beats
// slowly, and every 20–60 s one distant sound, never two at once: a soft thunder, an owl's two hoots or creaking
// wood. Everything is far away (low-passed, a short echo): calm and a little eerie, never a jump scare.
import { between, type Rng } from "../rng";
import type { Kit } from "../kit";
import { glide, swell } from "./shape";
import { RUMBLE_PEAK, rumble } from "./thunder";

export type SpookyOneShot = "owl" | "creak" | "thunder";

/** Level trim of the wind, the breeze and the drone (measured offline: peak ≈ −12 dBFS at volume 1). */
const BED_TRIM_DB = -6.5;

/** The echo every far sound goes through (and dry to the bus). */
function echo(kit: Kit): AudioNode {
  const input = kit.gain(1);
  const delay = kit.keep(kit.ctx.createDelay(1));
  delay.delayTime.value = 0.19;
  const feedback = kit.gain(0.32);
  const damp = kit.filter("lowpass", 1400, 0.5);
  const wet = kit.gain(0.45);
  input.connect(kit.bus);
  input.connect(delay);
  kit.chain(delay, damp, feedback, delay);
  kit.chain(damp, wet, kit.bus);
  return input;
}

/** One hoot: a soft sine with its octave, gliding down, with an 80 ms onset. */
function hoot(kit: Kit, into: AudioNode, at: number, from: number, to: number, seconds: number, level: number, pan: number): void {
  const env = kit.oneGain(0);
  env.gain.setValueAtTime(0, at);
  env.gain.linearRampToValueAtTime(level, at + 0.08);
  env.gain.setValueAtTime(level, at + seconds * 0.55);
  env.gain.exponentialRampToValueAtTime(level * 1e-3, at + seconds);
  const out = [env, kit.oneFilter("lowpass", 1100, 0.6), kit.onePan(pan)];
  const tone = kit.oneOsc("sine", from);
  glide(tone.frequency, at, from, to, seconds);
  kit.shot(tone, at, at + seconds + 0.05, out, into);
  const octave = kit.oneOsc("sine", from * 2);
  const octaveLevel = kit.oneGain(0.12);
  glide(octave.frequency, at, from * 2, to * 2, seconds);
  kit.shot(octave, at, at + seconds + 0.05, [octaveLevel], env);
}

/** An owl far away: "hoo … hoo-hooo". */
export function owl(kit: Kit, into: AudioNode, at: number): number {
  const { rng } = kit;
  const base = between(rng, 320, 360);
  const level = between(rng, 0.07, 0.1);
  const pan = between(rng, -0.7, 0.7);
  hoot(kit, into, at, base * 1.04, base * 0.97, 0.34, level, pan);
  const second = at + between(rng, 0.55, 0.75);
  hoot(kit, into, second, base * 1.02, base * 0.86, between(rng, 0.55, 0.7), level * 0.9, pan);
  if (rng() < 0.4) hoot(kit, into, second + 0.95, base, base * 0.9, 0.4, level * 0.6, pan);
  return second + 1.5;
}

/** Old wood creaking: a quiet buzz (sawtooth 80–140 Hz through a band-pass) with irregular pitch ramps. */
export function creak(kit: Kit, into: AudioNode, at: number): number {
  const { rng } = kit;
  const seconds = between(rng, 0.8, 1.8);
  const saw = kit.oneOsc("sawtooth", between(rng, 80, 140));
  let t = at;
  saw.frequency.setValueAtTime(saw.frequency.value, at);
  while (t < at + seconds) {
    t += between(rng, 0.12, 0.35);
    saw.frequency.exponentialRampToValueAtTime(between(rng, 70, 150), Math.min(t, at + seconds));
  }
  const env = kit.oneGain(0);
  const level = between(rng, 0.1, 0.15);
  env.gain.setValueAtTime(0, at);
  env.gain.linearRampToValueAtTime(level, at + 0.09);
  // Stick and slip: the level stutters a little while it creaks.
  for (let s = at + 0.15; s < at + seconds - 0.1; s += between(rng, 0.06, 0.14)) {
    env.gain.linearRampToValueAtTime(level * between(rng, 0.45, 1), s);
  }
  env.gain.linearRampToValueAtTime(0, at + seconds + 0.15);
  kit.shot(saw, at, at + seconds + 0.2, [
    kit.oneFilter("bandpass", between(rng, 700, 1100), 3.5),
    kit.oneFilter("lowpass", 2400, 0.6),
    env,
    kit.onePan(between(rng, -0.6, 0.6)),
  ], into);
  return at + seconds + 0.2;
}

function pickOneShot(rng: Rng): SpookyOneShot {
  const r = rng();
  return r < 0.4 ? "owl" : r < 0.75 ? "creak" : "thunder";
}

/** Plays `kind` now on this layer (the Developer page's buttons). */
export function playSpookyOneShot(kit: Kit, into: AudioNode, kind: SpookyOneShot, at: number): number {
  switch (kind) {
    case "owl":
      return owl(kit, into, at);
    case "creak":
      return creak(kit, into, at);
    case "thunder":
      return rumble(kit, at, RUMBLE_PEAK * 0.5);
  }
}

/** The far-away bus of the spooky layer (echo), kept on the kit for the Developer page's one-shots. */
const ECHO = new WeakMap<Kit, AudioNode>();

export function spookyEcho(kit: Kit): AudioNode {
  let node = ECHO.get(kit);
  if (!node) {
    node = echo(kit);
    ECHO.set(kit, node);
  }
  return node;
}

export function spookyNight(kit: Kit): void {
  const { buffers: b, rng } = kit;
  const out = kit.trim(BED_TRIM_DB);
  // Howling wind: a narrow band gliding between 300 and 700 Hz, with its fifth above, swelling slowly.
  const howl = kit.filter("bandpass", 450, 10);
  const howlHigh = kit.filter("bandpass", 675, 12);
  const howlLevel = kit.gain(0.5);
  const noise = kit.loop(b.pink);
  noise.connect(howl);
  noise.connect(howlHigh);
  // A narrow band keeps little of the noise's energy: made up here (the fifth sits lower).
  kit.chain(howlHigh, kit.gain(2.2), howlLevel);
  kit.chain(howl, kit.gain(5.5), howlLevel, out);
  const breeze = kit.gain(0.5);
  kit.chain(kit.loop(b.brown), kit.filter("bandpass", 380, 0.9), breeze, out);
  kit.lfo(0.06, 0.25, breeze.gain);
  kit.every({ min: 3, max: 7, first: 0.1 }, (at) => {
    const next = between(rng, 300, 700);
    const seconds = between(rng, 2.5, 6);
    howl.frequency.setTargetAtTime(next, at, seconds / 3);
    howlHigh.frequency.setTargetAtTime(next * 1.5, at, seconds / 3);
    howlLevel.gain.setTargetAtTime(rng() < 0.3 ? between(rng, 0.08, 0.25) : between(rng, 0.45, 1), at, seconds / 2.5);
  });

  // A faint drone (−30 dB): 55 / 55.6 Hz beat slowly, the octave and a soft tone above it for small speakers.
  const drone = kit.gain(0.6);
  kit.lfo(0.08, 0.2, drone.gain);
  kit.chain(drone, kit.filter("lowpass", 600, 0.5), out);
  for (const [freq, level] of [
    [55, 0.026],
    [55.6, 0.026],
    [110, 0.016],
    [220.3, 0.007],
  ] as const) {
    kit.chain(kit.osc(freq > 200 ? "triangle" : "sine", freq), kit.gain(level), drone);
  }

  // One far sound every 20–60 s (they never overlap: the longest lasts about 8 s).
  const far = spookyEcho(kit);
  kit.every({ min: 20, max: 60, first: between(rng, 6, 14) }, (at) => {
    playSpookyOneShot(kit, far, pickOneShot(rng), at);
  });
}
