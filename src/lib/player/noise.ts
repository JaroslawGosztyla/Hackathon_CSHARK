// Offline noise generator (WebAudio): looping stereo buffers of brown / pink / white noise.
import type { NoiseColor } from "$lib/types";

const LOOP_SECONDS = 6;
const CROSSFADE_SECONDS = 0.5;
const TARGET_RMS = 0.2;
const FADE_IN_SECONDS = 1.5;
const FADE_OUT_SECONDS = 0.4;
const VOLUME_SMOOTHING_SECONDS = 0.08;
/** Noise is dense; at 100 % user volume it should sit around music loudness, not above it. */
const NOISE_HEADROOM = 0.6;

let context: AudioContext | null = null;

/** Shared AudioContext (lazy). Null when WebAudio is unavailable. */
export function audioContext(): AudioContext | null {
  if (context) return context;
  if (typeof AudioContext === "undefined") return null;
  try {
    context = new AudioContext();
  } catch {
    context = null;
  }
  return context;
}

/** Call inside a user gesture (Start/Resume click) so later playback is not blocked by autoplay rules. */
export function unlockAudio(): void {
  const ctx = audioContext();
  if (ctx && ctx.state === "suspended") {
    ctx.resume().catch(() => {
      // Still locked; the WebView is started with --autoplay-policy=no-user-gesture-required anyway.
    });
  }
}

function generateChannel(color: NoiseColor, length: number): Float32Array<ArrayBuffer> {
  const raw = new Float32Array(length);
  let b0 = 0;
  let b1 = 0;
  let b2 = 0;
  let last = 0;
  for (let i = 0; i < length; i++) {
    const white = Math.random() * 2 - 1;
    if (color === "white") {
      raw[i] = white;
    } else if (color === "pink") {
      // Paul Kellet's economy pink filter.
      b0 = 0.99765 * b0 + white * 0.099046;
      b1 = 0.963 * b1 + white * 0.2965164;
      b2 = 0.57 * b2 + white * 1.0526913;
      raw[i] = b0 + b1 + b2 + white * 0.1848;
    } else {
      // Leaky integrator: brown noise without DC drift.
      last = (last + 0.02 * white) / 1.02;
      raw[i] = last;
    }
  }
  return raw;
}

function normalizeRms(samples: Float32Array<ArrayBuffer>): void {
  let sum = 0;
  for (const s of samples) sum += s * s;
  const rms = Math.sqrt(sum / Math.max(1, samples.length));
  const gain = rms > 0 ? TARGET_RMS / rms : 0;
  for (let i = 0; i < samples.length; i++) {
    samples[i] = Math.max(-1, Math.min(1, (samples[i] ?? 0) * gain));
  }
}

/** Equal-power crossfade of the tail into the head, so the loop point is seamless. */
function seamlessLoop(raw: Float32Array<ArrayBuffer>, loopLength: number, fadeLength: number): Float32Array<ArrayBuffer> {
  const out = raw.slice(0, loopLength);
  for (let i = 0; i < fadeLength; i++) {
    const t = i / fadeLength;
    out[i] = (raw[i] ?? 0) * Math.sqrt(t) + (raw[loopLength + i] ?? 0) * Math.sqrt(1 - t);
  }
  return out;
}

function createNoiseBuffer(ctx: AudioContext, color: NoiseColor): AudioBuffer {
  const loopLength = Math.floor(ctx.sampleRate * LOOP_SECONDS);
  const fadeLength = Math.floor(ctx.sampleRate * CROSSFADE_SECONDS);
  const buffer = ctx.createBuffer(2, loopLength, ctx.sampleRate);
  for (let channel = 0; channel < 2; channel++) {
    const raw = generateChannel(color, loopLength + fadeLength);
    normalizeRms(raw);
    buffer.copyToChannel(seamlessLoop(raw, loopLength, fadeLength), channel);
  }
  return buffer;
}

export interface NoiseEvents {
  onPlaying(): void;
  onError(): void;
}

interface Voice {
  source: AudioBufferSourceNode;
  fade: GainNode;
}

export class NoiseBackend {
  private master: GainNode | null = null;
  private voice: Voice | null = null;
  private buffers = new Map<NoiseColor, AudioBuffer>();
  private color: NoiseColor = "brown";
  private volume = 0.6;
  private readonly events: NoiseEvents;

  constructor(events: NoiseEvents) {
    this.events = events;
  }

  load(color: NoiseColor, play: boolean): void {
    if (this.voice && color !== this.color) this.release();
    this.color = color;
    if (play) this.play();
  }

  play(): void {
    if (this.voice) return;
    const ctx = audioContext();
    if (!ctx) {
      this.events.onError();
      return;
    }
    try {
      if (ctx.state === "suspended") void ctx.resume().catch(() => undefined);
      const master = this.ensureMaster(ctx);
      let buffer = this.buffers.get(this.color);
      if (!buffer) {
        buffer = createNoiseBuffer(ctx, this.color);
        this.buffers.set(this.color, buffer);
      }
      const source = ctx.createBufferSource();
      source.buffer = buffer;
      source.loop = true;
      const fade = ctx.createGain();
      const now = ctx.currentTime;
      fade.gain.setValueAtTime(0, now);
      fade.gain.linearRampToValueAtTime(1, now + FADE_IN_SECONDS);
      source.connect(fade);
      fade.connect(master);
      source.start(now);
      this.voice = { source, fade };
      this.events.onPlaying();
    } catch {
      this.events.onError();
    }
  }

  pause(): void {
    this.release();
  }

  unload(): void {
    this.release();
  }

  setVolume(volume: number): void {
    this.volume = Math.min(1, Math.max(0, volume));
    const ctx = context;
    if (ctx && this.master) {
      this.master.gain.setTargetAtTime(this.volume * NOISE_HEADROOM, ctx.currentTime, VOLUME_SMOOTHING_SECONDS);
    }
  }

  destroy(): void {
    this.release();
    this.master?.disconnect();
    this.master = null;
  }

  private ensureMaster(ctx: AudioContext): GainNode {
    if (!this.master) {
      this.master = ctx.createGain();
      this.master.gain.value = this.volume * NOISE_HEADROOM;
      this.master.connect(ctx.destination);
    }
    return this.master;
  }

  /** Fades the current voice out and stops it; a new voice can start immediately. */
  private release(): void {
    const voice = this.voice;
    const ctx = context;
    this.voice = null;
    if (!voice || !ctx) return;
    const now = ctx.currentTime;
    try {
      voice.fade.gain.cancelScheduledValues(now);
      voice.fade.gain.setValueAtTime(voice.fade.gain.value, now);
      voice.fade.gain.linearRampToValueAtTime(0, now + FADE_OUT_SECONDS);
      voice.source.stop(now + FADE_OUT_SECONDS + 0.05);
      voice.source.onended = () => {
        voice.source.disconnect();
        voice.fade.disconnect();
      };
    } catch {
      voice.source.disconnect();
      voice.fade.disconnect();
    }
  }
}
