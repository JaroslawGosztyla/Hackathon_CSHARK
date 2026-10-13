// Drives the frame of one mascot: rests on its mood frame (with a random blink, and a short smile now and then when
// calm), plays reactions, and shows the mouth for the letter its speech bubble just revealed (speech.svelte.ts).
// One controller per mascot on screen; `useMascot` ties it to the component that shows it.
import type { MascotVariant } from "./assets.svelte";
import {
  BLINK_DELAY_MS,
  blinkSteps,
  idleFrame,
  moodFrame,
  MOUTH_HOLD_MS,
  reactionSteps,
  SMILE_DELAY_MS,
  type MascotMood,
  type MascotReaction,
  type MascotStep,
} from "./frames";

export function prefersReducedMotion(): boolean {
  try {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return false;
  }
}

function randomDelay(range: { min: number; max: number }): number {
  return range.min + Math.random() * (range.max - range.min);
}

/** A sequence of frames playing now. */
interface Track {
  steps: MascotStep[];
  index: number;
  timer: ReturnType<typeof setTimeout> | null;
}

export class MascotController {
  /** Frame shown now (a key of the variant's manifest). */
  frame = $state("");
  readonly variant: MascotVariant;

  #mood: MascotMood;
  /** A reaction (start, nudge, …): wins over everything else, and speech waits for it. */
  #reaction: Track | null = null;
  /** A blink or a calm smile: only while resting, dropped as soon as anything else happens. */
  #ambient: Track | null = null;
  /** The speech bubble is revealing text: the frame is the mouth of the last revealed letter. */
  #talking = false;
  #mouth: string | null = null;
  /** The mouth for the latest letter; it shows once the current one has been held `MOUTH_HOLD_MS`. */
  #mouthWanted: string | null = null;
  #mouthShownAt = Number.NEGATIVE_INFINITY;
  #mouthTimer: ReturnType<typeof setTimeout> | null = null;
  /** Reduced motion: smiling while the bubble is shown, instead of moving the mouth. */
  #still = false;
  #blinkTimer: ReturnType<typeof setTimeout> | null = null;
  #smileTimer: ReturnType<typeof setTimeout> | null = null;
  /** Called once when the running reaction ends (speech waiting to start). */
  #onFree = new Set<() => void>();
  #disposed = false;

  constructor(variant: MascotVariant, mood: MascotMood = "neutral") {
    this.variant = variant;
    this.#mood = mood;
    this.frame = moodFrame(variant, mood);
    this.#scheduleAmbient();
  }

  /** A reaction is playing (speech waits for it to end). */
  get reacting(): boolean {
    return this.#reaction !== null;
  }

  /** The face to rest on (`sessionMood` on the Skupienie page, happy when the goal was done, …). */
  setMood(mood: MascotMood): void {
    if (this.#disposed || mood === this.#mood) return;
    this.#mood = mood;
    this.#stopAmbient();
    this.#render();
    this.#scheduleAmbient();
  }

  /** Plays a reaction now; it replaces a running one. Speech that starts meanwhile waits for it (`whenFree`). */
  react(reaction: MascotReaction): void {
    if (this.#disposed) return;
    this.#stopAmbient();
    this.#stopTrack(this.#reaction);
    const steps = reactionSteps(this.variant, reaction, prefersReducedMotion());
    if (steps.length === 0) {
      this.#reaction = null;
      this.#reactionEnded();
      return;
    }
    this.#reaction = { steps, index: 0, timer: null };
    this.#advance(this.#reaction);
  }

  /** Calls `fn` once no reaction plays (at once when none does). Returns a function that cancels the wait. */
  whenFree(fn: () => void): () => void {
    if (this.#disposed) return () => {};
    if (this.#reaction === null) {
      fn();
      return () => {};
    }
    this.#onFree.add(fn);
    return () => this.#onFree.delete(fn);
  }

  /** The speech bubble starts revealing text. */
  startTalking(): void {
    if (this.#disposed) return;
    this.#talking = true;
    this.#resetMouth();
    this.#stopAmbient();
    this.#render();
  }

  /**
   * The frame for the letter just revealed (a mouth frame, or the idle frame for a closed mouth). A new mouth shows only
   * after the current one was held `MOUTH_HOLD_MS`; letters revealed meanwhile only update which mouth comes next.
   */
  setMouth(frame: string): void {
    if (this.#disposed || !this.#talking) return;
    this.#mouthWanted = frame;
    if (this.#mouthTimer !== null) return;
    const wait = this.#mouthShownAt + MOUTH_HOLD_MS - performance.now();
    if (wait <= 0) {
      this.#showMouth();
      return;
    }
    this.#mouthTimer = setTimeout(() => {
      this.#mouthTimer = null;
      this.#showMouth();
    }, wait);
  }

  /** All the text is shown: back to the mood frame. */
  stopTalking(): void {
    if (this.#disposed || !this.#talking) return;
    this.#talking = false;
    this.#resetMouth();
    this.#render();
    this.#scheduleAmbient();
  }

  /** Reduced motion: smile while the bubble is shown (no mouth movement). */
  setStill(still: boolean): void {
    if (this.#disposed || still === this.#still) return;
    this.#still = still;
    this.#stopAmbient();
    this.#render();
    this.#scheduleAmbient();
  }

  destroy(): void {
    this.#disposed = true;
    this.#stopTrack(this.#reaction);
    this.#stopTrack(this.#ambient);
    this.#reaction = null;
    this.#ambient = null;
    this.#onFree.clear();
    this.#resetMouth();
    for (const timer of [this.#blinkTimer, this.#smileTimer]) if (timer !== null) clearTimeout(timer);
    this.#blinkTimer = null;
    this.#smileTimer = null;
  }

  /** The frame when nothing plays: the mouth while talking, else the mood (a smile while the still bubble shows). */
  #render(): void {
    if (this.#reaction !== null || this.#ambient !== null) return;
    if (this.#talking) this.frame = this.#mouth ?? idleFrame(this.variant);
    else this.frame = this.#still ? moodFrame(this.variant, "smile") : moodFrame(this.variant, this.#mood);
  }

  #showMouth(): void {
    // A closed mouth (null) shows the idle frame: no change, no hold.
    if (!this.#talking || this.#mouthWanted === (this.#mouth ?? idleFrame(this.variant))) return;
    this.#mouth = this.#mouthWanted;
    this.#mouthShownAt = performance.now();
    this.#render();
  }

  #resetMouth(): void {
    if (this.#mouthTimer !== null) clearTimeout(this.#mouthTimer);
    this.#mouthTimer = null;
    this.#mouth = null;
    this.#mouthWanted = null;
    this.#mouthShownAt = Number.NEGATIVE_INFINITY;
  }

  #advance(track: Track): void {
    const current = track.steps[track.index];
    if (!current) {
      if (track === this.#reaction) {
        this.#reaction = null;
        this.#reactionEnded();
      } else if (track === this.#ambient) {
        this.#ambient = null;
        this.#render();
        this.#scheduleAmbient();
      }
      return;
    }
    this.frame = current.frame;
    track.timer = setTimeout(() => {
      track.timer = null;
      track.index++;
      this.#advance(track);
    }, current.ms);
  }

  #reactionEnded(): void {
    const waiting = [...this.#onFree];
    this.#onFree.clear();
    for (const fn of waiting) fn();
    this.#render();
    this.#scheduleAmbient();
  }

  #stopTrack(track: Track | null): void {
    if (track?.timer != null) clearTimeout(track.timer);
    if (track) track.timer = null;
  }

  #stopAmbient(): void {
    if (this.#ambient === null) return;
    this.#stopTrack(this.#ambient);
    this.#ambient = null;
    this.#render();
  }

  /** Resting with the eyes open (blinks and calm smiles play only then). */
  #resting(): boolean {
    return this.#reaction === null && this.#ambient === null && !this.#talking && !this.#still;
  }

  #scheduleAmbient(): void {
    if (this.#disposed) return;
    if (this.#blinkTimer === null) {
      this.#blinkTimer = setTimeout(() => {
        this.#blinkTimer = null;
        // Blink frames have a neutral face: only from rest with a neutral or calm mood, never with reduced motion.
        if (this.#mood === "neutral" || this.#mood === "calm") this.#playAmbient(blinkSteps(this.variant));
        this.#scheduleAmbient();
      }, randomDelay(BLINK_DELAY_MS));
    }
    if (this.#smileTimer === null && this.#mood === "calm") {
      this.#smileTimer = setTimeout(() => {
        this.#smileTimer = null;
        if (this.#mood === "calm") this.#playAmbient(reactionSteps(this.variant, "smile", false));
        this.#scheduleAmbient();
      }, randomDelay(SMILE_DELAY_MS));
    }
  }

  #playAmbient(steps: MascotStep[]): void {
    if (!this.#resting() || steps.length === 0 || prefersReducedMotion()) return;
    if (document.visibilityState !== "visible") return;
    this.#ambient = { steps, index: 0, timer: null };
    this.#advance(this.#ambient);
  }
}

/** A controller for the calling component; it stops its timers when the component goes away. */
export function useMascot(variant: MascotVariant, mood: MascotMood = "neutral"): MascotController {
  const controller = new MascotController(variant, mood);
  $effect(() => () => controller.destroy());
  return controller;
}
