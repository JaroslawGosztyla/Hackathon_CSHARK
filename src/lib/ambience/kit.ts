// The building kit of one ambience layer (ADR-024): it owns every node the layer creates, so a layer is a short
// recipe (noise loops, filters, a few oscillators, one-shots on the audio clock) and stopping it never leaks a node.
// Graph of a layer: [its sources → filters → gains] → `bus` (fade-in, the layer's volume, fade-out) → `out`.
// Persistent nodes (`gain`, `filter`, `loop`, `osc`, `lfo`…) live until the layer stops; one-shot nodes (`one*`) are
// disconnected when their `shot` ends, so a layer that runs for hours keeps a constant number of nodes.
import type { NoiseBuffers } from "./buffers";
import { every, type Every } from "./scheduler";
import type { Rng } from "./rng";

/** A playing layer, as the engine sees it. */
export interface LayerVoice {
  /** The layer's own volume 0..=1 (smoothed). */
  setVolume(volume: number, at?: number): void;
  /** Fades out over `fadeSec`, then stops every source and disconnects every node. Idempotent. */
  stop(fadeSec: number): void;
}

/** Builds and starts a layer's sound into `kit.bus` (the kit fades it in and owns every node). */
export type LayerFactory = (kit: Kit) => void;

/** Smoothing of volume changes (time constant, seconds). */
export const VOLUME_SMOOTHING_S = 0.08;
/** An LFO is drawn with this many linear ramps per period, each between these lengths (seconds). */
const LFO_STEPS_PER_PERIOD = 8;
const LFO_MIN_STEP_S = 0.25;
const LFO_MAX_STEP_S = 2.5;

type Source = AudioScheduledSourceNode;
type Target = AudioNode | AudioParam;

function connect(from: AudioNode, to: Target): void {
  if (to instanceof AudioParam) from.connect(to);
  else from.connect(to);
}

export class Kit {
  readonly ctx: BaseAudioContext;
  readonly rng: Rng;
  readonly buffers: NoiseBuffers;
  /** The layer's output: every recipe ends here. */
  readonly bus: GainNode;

  #nodes: AudioNode[] = [];
  #sources = new Set<Source>();
  #stops: (() => void)[] = [];
  #stopped = false;

  constructor(ctx: BaseAudioContext, out: AudioNode, rng: Rng, buffers: NoiseBuffers) {
    this.ctx = ctx;
    this.rng = rng;
    this.buffers = buffers;
    this.bus = ctx.createGain();
    this.bus.gain.value = 0;
    this.bus.connect(out);
  }

  get stopped(): boolean {
    return this.#stopped;
  }

  // -------------------------------------------------------------------------
  // Persistent nodes (until the layer stops)
  // -------------------------------------------------------------------------

  /** Keeps `node` until the layer stops (then it is disconnected). */
  keep<T extends AudioNode>(node: T): T {
    this.#nodes.push(node);
    return node;
  }

  gain(value: number): GainNode {
    return this.keep(this.oneGain(value));
  }

  filter(type: BiquadFilterType, frequency: number, q = 0.707): BiquadFilterNode {
    return this.keep(this.oneFilter(type, frequency, q));
  }

  pan(value: number): StereoPannerNode {
    return this.keep(this.onePan(value));
  }

  /** A looped buffer (shared, never copied), started now from a random point so two loops never line up. */
  loop(buffer: AudioBuffer, rate = 1): AudioBufferSourceNode {
    const source = this.keep(this.ctx.createBufferSource());
    source.buffer = buffer;
    source.loop = true;
    source.playbackRate.value = rate;
    source.start(this.ctx.currentTime, this.rng() * buffer.duration);
    this.#sources.add(source);
    return source;
  }

  /** A running oscillator. */
  osc(type: OscillatorType, frequency: number, detune = 0): OscillatorNode {
    const source = this.keep(this.oneOsc(type, frequency));
    source.detune.value = detune;
    source.start(this.ctx.currentTime);
    this.#sources.add(source);
    return source;
  }

  /**
   * A slow sine LFO adding ±`depth` to `param` (whose own value is the centre), from a random phase. Drawn as linear
   * ramps on the audio clock, eight per period (at most every 2.5 s): no oscillator runs, and the parameter is
   * evaluated once per block. For modulations under about 1 Hz.
   */
  lfo(frequency: number, depth: number, param: AudioParam): void {
    kRate(param);
    const centre = param.value;
    const f = Math.max(0.001, frequency);
    const phase = this.rng() * 2 * Math.PI;
    const t0 = this.ctx.currentTime;
    const valueAt = (t: number) => centre + depth * Math.sin(phase + 2 * Math.PI * f * (t - t0));
    const step = Math.min(LFO_MAX_STEP_S, Math.max(LFO_MIN_STEP_S, 1 / (LFO_STEPS_PER_PERIOD * f)));
    param.setValueAtTime(valueAt(t0), t0);
    this.every({ min: step, max: step, first: step }, (t) => param.linearRampToValueAtTime(valueAt(t), t));
  }

  /**
   * The recipe's output stage: a fixed level trim (dB) into the bus. The trims are measured offline so that every
   * layer at volume 1 peaks around −12 dBFS (RMS about −25 dBFS) before the master and the limiter.
   */
  trim(decibels: number): GainNode {
    const node = this.gain(db(decibels));
    node.connect(this.bus);
    return node;
  }

  /** Connects `nodes` in order; returns the last one. */
  chain(first: AudioNode, ...rest: AudioNode[]): AudioNode {
    let last = first;
    for (const node of rest) {
      last.connect(node);
      last = node;
    }
    return last;
  }

  /** Jittered events on the audio clock while the layer plays (see `scheduler.ts`). */
  every(timing: Every, fire: (at: number) => void): void {
    if (this.#stopped) return;
    this.#stops.push(
      every(this.ctx, this.rng, timing, (at) => {
        if (!this.#stopped) fire(at);
      }),
    );
  }

  // -------------------------------------------------------------------------
  // One-shot nodes (disconnected when their shot ends)
  // -------------------------------------------------------------------------

  oneGain(value: number): GainNode {
    const node = this.ctx.createGain();
    node.gain.value = value;
    return node;
  }

  oneFilter(type: BiquadFilterType, frequency: number, q = 0.707): BiquadFilterNode {
    const node = this.ctx.createBiquadFilter();
    node.type = type;
    node.frequency.value = frequency;
    node.Q.value = q;
    // Slow sweeps only: the coefficients are computed once per 128-frame block, not per sample (a modulated a-rate
    // filter costs about ten times as much).
    kRate(node.frequency, node.Q, node.detune, node.gain);
    return node;
  }

  onePan(value: number): StereoPannerNode {
    const node = this.ctx.createStereoPanner();
    node.pan.value = Math.max(-1, Math.min(1, value));
    kRate(node.pan);
    return node;
  }

  oneOsc(type: OscillatorType, frequency: number): OscillatorNode {
    const node = this.ctx.createOscillator();
    node.type = type;
    node.frequency.value = frequency;
    return node;
  }

  /** A noise buffer read once from a random point (for rumbles, pops, breaths). */
  oneNoise(buffer: AudioBuffer, rate = 1): AudioBufferSourceNode {
    const node = this.ctx.createBufferSource();
    node.buffer = buffer;
    node.loop = true;
    node.loopStart = 0;
    node.loopEnd = buffer.duration;
    node.playbackRate.value = rate;
    return node;
  }

  /**
   * A one-shot: `source` → `nodes` in order → `into` (default: the bus). The source starts at `at` and stops at
   * `until`; the whole path is disconnected when it ends. A noise source starts from a random point of its buffer.
   * Returns false (and connects nothing) once the layer stopped.
   */
  shot(source: Source, at: number, until: number, nodes: readonly AudioNode[], into: Target = this.bus): boolean {
    if (this.#stopped) return false;
    let last: AudioNode = source;
    for (const node of nodes) {
      last.connect(node);
      last = node;
    }
    connect(last, into);
    this.#sources.add(source);
    source.onended = () => {
      this.#sources.delete(source);
      source.disconnect();
      for (const node of nodes) node.disconnect();
    };
    if (source instanceof AudioBufferSourceNode && source.buffer) {
      source.start(at, this.rng() * source.buffer.duration);
    } else {
      source.start(at);
    }
    source.stop(Math.max(at + 0.01, until));
    return true;
  }

  // -------------------------------------------------------------------------
  // The engine's handle
  // -------------------------------------------------------------------------

  /** The handle the engine keeps: fades in to `volume` over `fadeIn` seconds. */
  voice(volume: number, fadeIn: number): LayerVoice {
    const now = this.ctx.currentTime;
    this.bus.gain.setValueAtTime(0, now);
    this.bus.gain.linearRampToValueAtTime(clampUnit(volume), now + Math.max(0.01, fadeIn));
    return {
      setVolume: (v, at) => {
        if (this.#stopped) return;
        const when = at ?? this.ctx.currentTime;
        this.bus.gain.cancelScheduledValues(when);
        this.bus.gain.setTargetAtTime(clampUnit(v), when, VOLUME_SMOOTHING_S);
      },
      stop: (fadeSec) => this.#stop(fadeSec),
    };
  }

  #stop(fadeSec: number): void {
    if (this.#stopped) return;
    this.#stopped = true;
    for (const stop of this.#stops) stop();
    this.#stops = [];
    const now = this.ctx.currentTime;
    const end = now + Math.max(0.01, fadeSec);
    try {
      const gain = this.bus.gain;
      gain.cancelScheduledValues(now);
      gain.setValueAtTime(gain.value, now);
      gain.linearRampToValueAtTime(0, end);
    } catch {
      // A closed context: nothing to fade.
    }
    for (const source of this.#sources) {
      try {
        source.stop(end + 0.05);
      } catch {
        // Not started or already stopped.
      }
    }
    const nodes = this.#nodes;
    const sources = this.#sources;
    const bus = this.bus;
    this.#nodes = [];
    this.#sources = new Set();
    setTimeout(
      () => {
        for (const source of sources) source.disconnect();
        for (const node of nodes) node.disconnect();
        bus.disconnect();
      },
      (Math.max(0.01, fadeSec) + 0.3) * 1000,
    );
  }
}

/** Block-rate automation where the engine supports it (older engines keep a-rate; only the cost differs). */
export function kRate(...params: AudioParam[]): void {
  for (const param of params) {
    try {
      param.automationRate = "k-rate";
    } catch {
      // Not supported for this parameter: it stays a-rate.
    }
  }
}

export function clampUnit(value: number): number {
  return Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 0;
}

/** Decibels → gain factor. */
export function db(decibels: number): number {
  return 10 ** (decibels / 20);
}

export type { Every };
