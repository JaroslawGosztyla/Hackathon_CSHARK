// Beta ambience engine (ADR-024), main window only: mixes the synthesized layers over the music. One audio context
// (the shared one of `player/noise.ts`, never a second), one master gain and a limiter:
//   layer kit bus (fade-in 1.5 s, the layer's volume) → master (volume × AMBIENCE_HEADROOM) → limiter → speakers.
// `sync(target)` makes the sounding layers match the target (the follow rules live in `state/ambience.svelte.ts`);
// with nothing to play every layer fades out (0.6 s) and the master chain is disconnected: no node runs while idle,
// and the context is never created before something should play. The Developer page plays one layer alone as a sample
// (`startSample`, ignoring lock and beta, never writing anything) and the spooky one-shots on demand.
import { audioContext } from "$lib/player/noise";
import type { AmbienceLayer } from "$lib/types";
import { noiseBuffers } from "./buffers";
import { DEFAULT_AMBIENCE_VOLUME } from "./catalog";
import { Kit, clampUnit, type LayerVoice, VOLUME_SMOOTHING_S } from "./kit";
import { LAYERS } from "./layers";
import { playSpookyOneShot, spookyEcho, type SpookyOneShot } from "./layers/spookyNight";
import { freshSeed, mulberry32 } from "./rng";

/** Ambience sits under the music: at 100 % master volume the sum stays a step below it. */
export const AMBIENCE_HEADROOM = 0.7;
export const FADE_IN_S = 1.5;
export const FADE_OUT_S = 0.6;
/** The limiter on the master (a gentle brick wall for many layers at once). */
const LIMITER = { threshold: -14, knee: 6, ratio: 12, attack: 0.003, release: 0.25 } as const;
/** A Developer sample plays this long unless stopped. */
export const SAMPLE_MS = 20_000;
/** A Developer sample's layer volume. */
const SAMPLE_LAYER_VOLUME = 0.7;

export interface AmbienceTarget {
  /** Master volume 0..=1 (`Config.ambience.volume`). */
  master: number;
  /** The layers to play and their volumes, in catalogue order. */
  layers: readonly { layer: AmbienceLayer; volume: number }[];
}

interface Playing {
  kit: Kit;
  voice: LayerVoice;
}

interface Chain {
  ctx: AudioContext;
  master: GainNode;
  limiter: DynamicsCompressorNode;
}

class AmbienceEngine {
  /** Layers sounding now (fading in or playing), catalogue order. */
  sounding = $state.raw<readonly AmbienceLayer[]>([]);
  /** The Developer page's sample layer, while it plays. */
  sample = $state<AmbienceLayer | null>(null);

  #chain: Chain | null = null;
  #voices = new Map<AmbienceLayer, Playing>();
  #target: AmbienceTarget | null = null;
  #sampleTimer: ReturnType<typeof setTimeout> | null = null;
  #teardownTimer: ReturnType<typeof setTimeout> | null = null;
  /** One-shot kits of the Developer page (no layer playing to host them). */
  #extras = new Set<Kit>();

  /** The layers that should sound now (null or none = silence). */
  sync(target: AmbienceTarget | null): void {
    this.#target = target;
    this.#apply();
  }

  /** Developer page: plays `layer` alone for `ms` (lock, beta and the mixer are ignored; nothing is written). */
  startSample(layer: AmbienceLayer, ms = SAMPLE_MS): void {
    this.#clearSampleTimer();
    this.sample = layer;
    this.#sampleTimer = setTimeout(() => this.stopSample(), ms);
    this.#apply();
  }

  stopSample(): void {
    this.#clearSampleTimer();
    if (this.sample === null) return;
    this.sample = null;
    this.#apply();
  }

  /**
   * Developer page: one spooky far sound now. On the spooky layer when it plays (through its echo), else on a short
   * kit of its own that fades away when the sound is over.
   */
  oneShot(kind: SpookyOneShot): void {
    const chain = this.#ensureChain();
    if (!chain) return;
    this.#cancelTeardown();
    const host = this.#voices.get("spookyNight");
    const at = chain.ctx.currentTime + 0.05;
    if (host) {
      playSpookyOneShot(host.kit, spookyEcho(host.kit), kind, at);
      return;
    }
    const kit = new Kit(chain.ctx, chain.master, mulberry32(freshSeed()), noiseBuffers(chain.ctx));
    const voice = kit.voice(SAMPLE_LAYER_VOLUME, 0.01);
    const end = playSpookyOneShot(kit, spookyEcho(kit), kind, at);
    // Nothing else plays: the master is at the sample level (else it keeps the user's volume).
    if (this.#voices.size === 0) {
      chain.master.gain.setTargetAtTime(DEFAULT_AMBIENCE_VOLUME * AMBIENCE_HEADROOM, chain.ctx.currentTime, VOLUME_SMOOTHING_S);
    }
    this.#extras.add(kit);
    setTimeout(
      () => {
        voice.stop(FADE_OUT_S);
        this.#extras.delete(kit);
        if (this.#voices.size === 0 && this.#extras.size === 0) this.#scheduleTeardown();
      },
      (end - chain.ctx.currentTime + 1.5) * 1000,
    );
  }

  #clearSampleTimer(): void {
    if (this.#sampleTimer !== null) clearTimeout(this.#sampleTimer);
    this.#sampleTimer = null;
  }

  #effectiveTarget(): AmbienceTarget | null {
    if (this.sample !== null) {
      return { master: DEFAULT_AMBIENCE_VOLUME, layers: [{ layer: this.sample, volume: SAMPLE_LAYER_VOLUME }] };
    }
    return this.#target;
  }

  #apply(): void {
    const target = this.#effectiveTarget();
    const wanted = new Map((target?.layers ?? []).map((l) => [l.layer, clampUnit(l.volume)] as const));
    for (const [layer, playing] of this.#voices) {
      if (wanted.has(layer)) continue;
      playing.voice.stop(FADE_OUT_S);
      this.#voices.delete(layer);
    }
    if (wanted.size === 0 || !target) {
      this.sounding = [];
      if (this.#extras.size === 0) this.#scheduleTeardown();
      return;
    }
    const chain = this.#ensureChain();
    if (!chain) {
      this.sounding = [];
      return;
    }
    this.#cancelTeardown();
    const now = chain.ctx.currentTime;
    chain.master.gain.setTargetAtTime(clampUnit(target.master) * AMBIENCE_HEADROOM, now, VOLUME_SMOOTHING_S);
    for (const [layer, volume] of wanted) {
      const playing = this.#voices.get(layer);
      if (playing) {
        playing.voice.setVolume(volume);
        continue;
      }
      const kit = new Kit(chain.ctx, chain.master, mulberry32(freshSeed()), noiseBuffers(chain.ctx));
      try {
        LAYERS[layer](kit);
        this.#voices.set(layer, { kit, voice: kit.voice(volume, FADE_IN_S) });
      } catch (e) {
        // A layer that cannot build never stops the others (logged, no toast: decorative sound).
        kit.voice(0, 0.01).stop(0.01);
        console.warn(`[FocusMode] ambience: ${layer} could not start`, e);
      }
    }
    this.sounding = [...wanted.keys()].filter((layer) => this.#voices.has(layer));
  }

  #ensureChain(): Chain | null {
    if (this.#chain) return this.#chain;
    const ctx = audioContext();
    if (!ctx) return null;
    if (ctx.state === "suspended") void ctx.resume().catch(() => undefined);
    const master = ctx.createGain();
    master.gain.value = 0;
    const limiter = ctx.createDynamicsCompressor();
    limiter.threshold.value = LIMITER.threshold;
    limiter.knee.value = LIMITER.knee;
    limiter.ratio.value = LIMITER.ratio;
    limiter.attack.value = LIMITER.attack;
    limiter.release.value = LIMITER.release;
    master.connect(limiter).connect(ctx.destination);
    this.#chain = { ctx, master, limiter };
    return this.#chain;
  }

  #scheduleTeardown(): void {
    if (!this.#chain || this.#teardownTimer !== null) return;
    this.#teardownTimer = setTimeout(
      () => {
        this.#teardownTimer = null;
        if (this.#voices.size > 0 || this.#extras.size > 0) return;
        const chain = this.#chain;
        this.#chain = null;
        chain?.master.disconnect();
        chain?.limiter.disconnect();
      },
      (FADE_OUT_S + 0.5) * 1000,
    );
  }

  #cancelTeardown(): void {
    if (this.#teardownTimer !== null) clearTimeout(this.#teardownTimer);
    this.#teardownTimer = null;
  }
}

/** The one ambience engine of the main window. */
export const ambienceEngine = new AmbienceEngine();
