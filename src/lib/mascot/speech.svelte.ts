// What a mascot says: one message at a time, revealed letter by letter (each letter fades in, see RevealText) while
// the mouth shows the letter just revealed. A message waits for a running reaction (wink, cheer, …) to end, stays a
// while after its last letter and then fades out, unless a newer one replaces it. Messages that are not urgent keep
// a minimum gap after the previous one (the newest waiting one wins), so the mascot never chatters.
// With reduced motion the whole text appears at once and the mascot smiles while it is shown.
import type { MascotController } from "./controller.svelte";
import { prefersReducedMotion } from "./controller.svelte";
import { isSyllable, mouthFor, mouthFrame } from "./frames";

/** Time between two letters (random within the range, so it sounds less mechanical). */
export const LETTER_MS = { min: 45, max: 60 } as const;
/** Extra pause after a comma-like and after a sentence-ending mark. */
const PAUSE_SHORT_MS = 170;
const PAUSE_LONG_MS = 320;
const SHORT_PAUSE = new Set([",", ";", ":", "，", "、", "；", "："]);
const LONG_PAUSE = new Set([".", "!", "?", "…", "—", "–", "。", "！", "？", "\n"]);
/** After the last letter the bubble stays max(HOLD_MIN_MS, HOLD_PER_LETTER_MS × letters), then fades out. */
const HOLD_MIN_MS = 4000;
const HOLD_PER_LETTER_MS = 60;
/** Fade-out of the bubble (SpeechBubble's CSS transition). */
export const FADE_MS = 300;
/** Minimum time between the starts of two messages that are not urgent. */
export const MIN_GAP_MS = 8000;

export interface MascotLine {
  text: string;
  /** Small heading above the text (e.g. "Dobra praktyka"). */
  label?: string | null;
  /** Answers something that just happened (Start, a break, a cycle): shown at once, never held back by the gap. */
  urgent?: boolean;
  /** Lets `withdraw` take the message back when what it says no longer applies. */
  tag?: string;
}

/** The message shown now. */
export interface SpokenLine {
  id: number;
  text: string;
  label: string | null;
  tag: string | null;
  /** The text split into graphemes (what is revealed one by one). */
  units: string[];
}

const segmenter: Intl.Segmenter | null = (() => {
  try {
    return new Intl.Segmenter(undefined, { granularity: "grapheme" });
  } catch {
    return null;
  }
})();

/** The text as user-perceived characters (an emoji or a letter with a combining accent stays one unit). */
export function graphemes(text: string): string[] {
  return segmenter ? Array.from(segmenter.segment(text), (s) => s.segment) : Array.from(text);
}

function pauseAfter(unit: string): number {
  const base = LETTER_MS.min + Math.random() * (LETTER_MS.max - LETTER_MS.min);
  if (LONG_PAUSE.has(unit)) return base + PAUSE_LONG_MS;
  if (SHORT_PAUSE.has(unit)) return base + PAUSE_SHORT_MS;
  return base;
}

export interface SpeechOptions {
  /** Keep the text after it is revealed (the popup: the message is the popup's content). */
  persist?: boolean;
}

export class MascotSpeech {
  /** The message shown now (null: no bubble). */
  line = $state.raw<SpokenLine | null>(null);
  /** Letters (graphemes) of `line` shown so far. */
  revealed = $state(0);
  /** The bubble is fading out. */
  leaving = $state(false);

  readonly #controller: MascotController;
  readonly #persist: boolean;
  #seq = 0;
  /** `performance.now()` when the last message started (for the gap). */
  #lastAt = Number.NEGATIVE_INFINITY;
  /** The message shown now, or waiting for a reaction to end before it shows. */
  #current: MascotLine | null = null;
  #pending: MascotLine | null = null;
  #pendingTimer: ReturnType<typeof setTimeout> | null = null;
  #timer: ReturnType<typeof setTimeout> | null = null;
  #cancelWait: (() => void) | null = null;
  /** CJK characters revealed so far in this message (they alternate the mouth). */
  #syllables = 0;
  #disposed = false;

  constructor(controller: MascotController, options: SpeechOptions = {}) {
    this.#controller = controller;
    this.#persist = options.persist === true;
  }

  /** All the text of the current message is shown. */
  get complete(): boolean {
    return this.line !== null && this.revealed >= this.line.units.length;
  }

  /** Says `line` now (urgent, or the gap since the last message passed) or as soon as the gap allows. */
  say(line: MascotLine): void {
    if (this.#disposed || line.text.trim() === "") return;
    if (!line.urgent) {
      const wait = this.#lastAt + MIN_GAP_MS - performance.now();
      if (wait > 0) {
        this.#pending = line;
        this.#schedulePending(wait);
        return;
      }
    }
    if (!line.urgent || (this.#pending !== null && this.#pending.tag === line.tag)) this.#dropPending();
    // An urgent line interrupts a message about the current state: that one comes back after the gap.
    const interrupted = this.#current;
    if (line.urgent && this.#pending === null && interrupted && !interrupted.urgent && !this.leaving) {
      this.#pending = interrupted;
    }
    this.#start(line);
    // A message still waiting keeps the gap after this one.
    if (this.#pending !== null) this.#schedulePending(MIN_GAP_MS);
  }

  /** Takes back the messages with this tag: a waiting one is dropped, a shown one fades out. */
  withdraw(tag: string): void {
    if (this.#pending?.tag === tag) this.#dropPending();
    if (this.#current?.tag !== tag) return;
    // Still waiting for a reaction to end: never shown, just forget it.
    if (this.line === null) this.#stop();
    else if (!this.leaving) this.#fadeOut();
  }

  /** Shows the whole text at once (a click on the bubble). */
  skip(): void {
    const line = this.line;
    if (!line || this.complete) return;
    this.#clearTimer();
    this.#cancelWait?.();
    this.#cancelWait = null;
    this.revealed = line.units.length;
    this.#finished();
  }

  /** Hides the bubble now and forgets a waiting message. */
  clear(): void {
    this.#dropPending();
    this.#stop();
  }

  destroy(): void {
    this.clear();
    this.#disposed = true;
  }

  #start(line: MascotLine): void {
    this.#stop();
    this.#current = line;
    this.#lastAt = performance.now();
    const reduced = prefersReducedMotion();
    const show = () => {
      this.#cancelWait = null;
      this.#show(line, reduced);
    };
    // The mouth would fight a running reaction: wait for it (with reduced motion nothing moves, show at once).
    if (reduced) show();
    else this.#cancelWait = this.#controller.whenFree(show);
  }

  #show(line: MascotLine, reduced: boolean): void {
    const units = graphemes(line.text);
    this.#seq++;
    this.line = { id: this.#seq, text: line.text, label: line.label ?? null, tag: line.tag ?? null, units };
    this.leaving = false;
    this.#syllables = 0;
    if (reduced) {
      this.revealed = units.length;
      this.#controller.setStill(true);
      this.#finished();
      return;
    }
    this.revealed = 0;
    this.#controller.startTalking();
    this.#next();
  }

  /** Reveals the next letter and moves the mouth for it. */
  #next(): void {
    const line = this.line;
    if (!line) return;
    const unit = line.units[this.revealed];
    if (unit === undefined) {
      this.#finished();
      return;
    }
    this.revealed++;
    const syllable = isSyllable(unit);
    this.#controller.setMouth(mouthFrame(this.#controller.variant, mouthFor(unit, this.#syllables)));
    if (syllable) this.#syllables++;
    this.#timer = setTimeout(() => {
      this.#timer = null;
      this.#next();
    }, pauseAfter(unit));
  }

  #finished(): void {
    this.#controller.stopTalking();
    if (this.#persist || !this.line) return;
    const hold = Math.max(HOLD_MIN_MS, HOLD_PER_LETTER_MS * this.line.units.length);
    this.#clearTimer();
    this.#timer = setTimeout(() => {
      this.#timer = null;
      this.#fadeOut();
    }, hold);
  }

  #fadeOut(): void {
    this.#clearTimer();
    this.#cancelWait?.();
    this.#cancelWait = null;
    this.#controller.stopTalking();
    this.leaving = true;
    this.#timer = setTimeout(() => {
      this.#timer = null;
      this.#stop();
    }, FADE_MS);
  }

  /** Removes the bubble at once. */
  #stop(): void {
    this.#clearTimer();
    this.#cancelWait?.();
    this.#cancelWait = null;
    this.#controller.stopTalking();
    this.#controller.setStill(false);
    this.#current = null;
    this.line = null;
    this.revealed = 0;
    this.leaving = false;
  }

  #clearTimer(): void {
    if (this.#timer !== null) clearTimeout(this.#timer);
    this.#timer = null;
  }

  #schedulePending(wait: number): void {
    if (this.#pendingTimer !== null) clearTimeout(this.#pendingTimer);
    this.#pendingTimer = setTimeout(() => {
      this.#pendingTimer = null;
      const next = this.#pending;
      this.#pending = null;
      if (next) this.#start(next);
    }, wait);
  }

  #dropPending(): void {
    this.#pending = null;
    if (this.#pendingTimer !== null) clearTimeout(this.#pendingTimer);
    this.#pendingTimer = null;
  }
}

/** Speech for the calling component's mascot; it stops its timers when the component goes away. */
export function useSpeech(controller: MascotController, options: SpeechOptions = {}): MascotSpeech {
  const speech = new MascotSpeech(controller, options);
  $effect(() => () => speech.destroy());
  return speech;
}
