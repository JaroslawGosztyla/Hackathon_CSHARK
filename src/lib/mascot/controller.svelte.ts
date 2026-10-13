// Drives the frame of one mascot: rests (mood frame) with a random blink, plays reactions and vowel lip-sync.
// One controller per mascot on screen; `useMascot` ties it to the component that shows it.
import type { MascotVariant } from "./assets.svelte";
import {
  BLINK_DELAY_MS,
  blinkSteps,
  lipSyncSteps,
  moodFrame,
  reactionSteps,
  type MascotMood,
  type MascotReaction,
  type MascotStep,
} from "./frames";

type TrackKind = "reaction" | "talk" | "blink";
/** What `talk` plays: lip-sync steps, or (reduced motion) a smile held until `hush`. */
type Speech = MascotStep[] | "still";

function prefersReducedMotion(): boolean {
  try {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return false;
  }
}

export class MascotController {
  /** Frame shown now (a key of the variant's manifest). */
  frame = $state("");
  readonly variant: MascotVariant;

  #mood: MascotMood;
  #kind: TrackKind | null = null;
  #track: MascotStep[] = [];
  #index = 0;
  /** Speech waiting for the running reaction to end. */
  #queued: Speech | null = null;
  /** Reduced motion: smiling while the bubble talks, instead of lip-sync. */
  #still = false;
  #stepTimer: ReturnType<typeof setTimeout> | null = null;
  #blinkTimer: ReturnType<typeof setTimeout> | null = null;
  #disposed = false;

  constructor(variant: MascotVariant, mood: MascotMood = "neutral") {
    this.variant = variant;
    this.#mood = mood;
    this.frame = moodFrame(variant, mood);
    this.#scheduleBlink();
  }

  /** The face to rest on (e.g. sad in an empty summary, happy when the goal was done). */
  setMood(mood: MascotMood): void {
    if (this.#disposed || mood === this.#mood) return;
    this.#mood = mood;
    if (this.#kind === null) this.frame = this.#rest();
    this.#scheduleBlink();
  }

  /** Plays a reaction now; it replaces whatever runs (speech waits for it only when `talk` comes after). */
  react(reaction: MascotReaction): void {
    if (this.#disposed) return;
    this.#queued = null;
    this.#still = false;
    this.#play(reactionSteps(this.variant, reaction, prefersReducedMotion()), "reaction");
  }

  /**
   * Says `text`: vowel lip-sync for at most a few seconds, then rest. A running reaction finishes first. With reduced
   * motion there is no lip-sync: the mascot smiles until `hush` (call it when the bubble goes away).
   */
  talk(text: string): void {
    if (this.#disposed || text.trim() === "") return;
    const speech: Speech = prefersReducedMotion() ? "still" : lipSyncSteps(text, this.variant);
    if (this.#kind === "reaction") {
      this.#queued = speech;
      return;
    }
    this.#speak(speech);
  }

  /** Stops talking (the bubble went away); a running reaction keeps playing. */
  hush(): void {
    if (this.#disposed) return;
    this.#queued = null;
    const wasTalking = this.#kind === "talk" || this.#still;
    this.#still = false;
    if (this.#kind === "talk") this.#stopTrack();
    if (wasTalking && this.#kind === null) {
      this.frame = this.#rest();
      this.#scheduleBlink();
    }
  }

  destroy(): void {
    this.#disposed = true;
    this.#stopTrack();
    if (this.#blinkTimer !== null) clearTimeout(this.#blinkTimer);
    this.#blinkTimer = null;
  }

  #rest(): string {
    return this.#still ? moodFrame(this.variant, "smile") : moodFrame(this.variant, this.#mood);
  }

  #speak(speech: Speech): void {
    if (speech === "still") {
      this.#stopTrack();
      this.#still = true;
      this.frame = this.#rest();
      return;
    }
    this.#still = false;
    this.#play(speech, "talk");
  }

  #play(steps: MascotStep[], kind: TrackKind): void {
    this.#stopTrack();
    if (steps.length === 0) {
      this.#finish();
      return;
    }
    this.#kind = kind;
    this.#track = steps;
    this.#index = 0;
    this.#step();
  }

  #step(): void {
    const current = this.#track[this.#index];
    if (!current) {
      this.#finish();
      return;
    }
    this.frame = current.frame;
    this.#stepTimer = setTimeout(() => {
      this.#stepTimer = null;
      this.#index++;
      this.#step();
    }, current.ms);
  }

  #stopTrack(): void {
    if (this.#stepTimer !== null) clearTimeout(this.#stepTimer);
    this.#stepTimer = null;
    this.#kind = null;
    this.#track = [];
    this.#index = 0;
  }

  #finish(): void {
    this.#kind = null;
    this.#track = [];
    const next = this.#queued;
    this.#queued = null;
    if (next !== null) {
      this.#speak(next);
      if (this.#kind !== null) return;
    }
    this.frame = this.#rest();
    this.#scheduleBlink();
  }

  #scheduleBlink(): void {
    if (this.#blinkTimer !== null) clearTimeout(this.#blinkTimer);
    this.#blinkTimer = null;
    if (this.#disposed) return;
    const delay = BLINK_DELAY_MS.min + Math.random() * (BLINK_DELAY_MS.max - BLINK_DELAY_MS.min);
    this.#blinkTimer = setTimeout(() => {
      this.#blinkTimer = null;
      // Blink frames have a neutral face: only from rest with a neutral mood, and never with reduced motion.
      const resting = this.#kind === null && !this.#still && this.#mood === "neutral";
      if (resting && !prefersReducedMotion() && document.visibilityState === "visible") {
        this.#play(blinkSteps(this.variant), "blink");
      } else {
        this.#scheduleBlink();
      }
    }, delay);
  }
}

/** A controller for the calling component; it stops its timers when the component goes away. */
export function useMascot(variant: MascotVariant, mood: MascotMood = "neutral"): MascotController {
  const controller = new MascotController(variant, mood);
  $effect(() => () => controller.destroy());
  return controller;
}
