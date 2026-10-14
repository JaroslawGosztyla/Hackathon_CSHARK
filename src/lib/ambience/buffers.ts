// Shared audio buffers of the ambience layers (ADR-024), rendered once per audio context at first use and reused by
// every layer and every start: the three noise colours (the generator of the music's noise source, `player/noise.ts`)
// and two textures drawn sample by sample (rain droplets, fire crackle). No audio files.
import { noiseBuffer } from "$lib/player/noise";
import { between, mulberry32, type Rng } from "./rng";

export interface NoiseBuffers {
  /** 6 s seamless stereo loops, RMS-normalised to about 0.2. */
  white: AudioBuffer;
  pink: AudioBuffer;
  brown: AudioBuffer;
  /** 4 s stereo loop of droplet ticks and tiny plinks on glass and leaves (peaks ≤ 0.4). */
  droplets: AudioBuffer;
  /** 8 s stereo loop of wood-fire crackle: clicks in irregular clusters (peaks ≤ 0.8). */
  crackle: AudioBuffer;
}

const DROPLETS_SECONDS = 4;
const DROPLETS_PER_SECOND = 38;
const CRACKLE_SECONDS = 8;
/** Fixed seeds: the textures sound the same in every run (the variety comes from where a loop starts). */
const DROPLETS_SEED = 0x5eed_d201;
const CRACKLE_SEED = 0x5eed_c4ac;
/** Nothing is drawn this close to the loop end, so no tick is cut at the seam. */
const SEAM_SECONDS = 0.05;

const cache = new WeakMap<BaseAudioContext, NoiseBuffers>();

/** The shared buffers of `ctx` (rendered on the first call; a few milliseconds of work). */
export function noiseBuffers(ctx: BaseAudioContext): NoiseBuffers {
  let buffers = cache.get(ctx);
  if (!buffers) {
    buffers = {
      white: noiseBuffer(ctx, "white"),
      pink: noiseBuffer(ctx, "pink"),
      brown: noiseBuffer(ctx, "brown"),
      droplets: renderDroplets(ctx),
      crackle: renderCrackle(ctx),
    };
    cache.set(ctx, buffers);
  }
  return buffers;
}

/** Adds a decaying noise click at `start` (seconds). */
function click(out: Float32Array, rate: number, rng: Rng, start: number, amp: number, decayMs: number): void {
  const first = Math.floor(start * rate);
  const length = Math.floor((decayMs * 5 * rate) / 1000);
  const k = 1000 / (decayMs * rate);
  for (let i = 0; i < length && first + i < out.length; i++) {
    const index = first + i;
    out[index] = (out[index] ?? 0) + (rng() * 2 - 1) * amp * Math.exp(-i * k);
  }
}

/** Adds a tiny decaying sine "plink" at `start` (seconds). */
function plink(out: Float32Array, rate: number, start: number, amp: number, freq: number, decayMs: number): void {
  const first = Math.floor(start * rate);
  const length = Math.floor((decayMs * 5 * rate) / 1000);
  const k = 1000 / (decayMs * rate);
  const w = (2 * Math.PI * freq) / rate;
  // A 1 ms fade-in keeps the plink from clicking.
  const attack = Math.max(1, Math.floor(rate / 1000));
  for (let i = 0; i < length && first + i < out.length; i++) {
    const index = first + i;
    const env = Math.min(1, i / attack) * Math.exp(-i * k);
    out[index] = (out[index] ?? 0) + Math.sin(i * w) * amp * env;
  }
}

function clamp(out: Float32Array, peak: number): void {
  for (let i = 0; i < out.length; i++) out[i] = Math.max(-peak, Math.min(peak, out[i] ?? 0));
}

function renderDroplets(ctx: BaseAudioContext): AudioBuffer {
  const rate = ctx.sampleRate;
  const buffer = ctx.createBuffer(2, Math.floor(rate * DROPLETS_SECONDS), rate);
  const rng = mulberry32(DROPLETS_SEED);
  for (let channel = 0; channel < 2; channel++) {
    const out = new Float32Array(buffer.length);
    const count = DROPLETS_SECONDS * DROPLETS_PER_SECOND;
    for (let n = 0; n < count; n++) {
      const at = between(rng, 0, DROPLETS_SECONDS - SEAM_SECONDS);
      // Many soft drops, a few closer ones (squared amplitude).
      const amp = 0.04 + 0.3 * rng() ** 2;
      if (rng() < 0.22) plink(out, rate, at, amp * 0.6, between(rng, 1800, 4600), between(rng, 6, 16));
      else click(out, rate, rng, at, amp, between(rng, 0.6, 2.2));
    }
    clamp(out, 0.4);
    buffer.copyToChannel(out, channel);
  }
  return buffer;
}

function renderCrackle(ctx: BaseAudioContext): AudioBuffer {
  const rate = ctx.sampleRate;
  const buffer = ctx.createBuffer(2, Math.floor(rate * CRACKLE_SECONDS), rate);
  const rng = mulberry32(CRACKLE_SEED);
  for (let channel = 0; channel < 2; channel++) {
    const out = new Float32Array(buffer.length);
    let t = between(rng, 0, 0.1);
    // A slow random walk of the crackle rate: quiet stretches and lively bursts.
    let activity = 0.5;
    while (t < CRACKLE_SECONDS - SEAM_SECONDS) {
      activity = Math.min(1, Math.max(0.08, activity + between(rng, -0.12, 0.12)));
      const amp = 0.06 + 0.7 * rng() ** 3;
      if (rng() < 0.12) {
        // A snap: a few clicks within 10–40 ms.
        const clicks = 3 + Math.floor(rng() * 4);
        let s = t;
        for (let c = 0; c < clicks; c++) {
          click(out, rate, rng, s, amp * between(rng, 0.4, 1), between(rng, 0.2, 0.9));
          s += between(rng, 0.002, 0.01);
        }
      } else {
        click(out, rate, rng, t, amp, between(rng, 0.15, 1.2));
      }
      // Mean gap 25–250 ms depending on the activity (exponential: irregular, like a real fire).
      const mean = 0.25 - 0.22 * activity;
      t += -Math.log(1 - rng() * 0.999) * mean;
    }
    clamp(out, 0.8);
    buffer.copyToChannel(out, channel);
  }
  return buffer;
}
