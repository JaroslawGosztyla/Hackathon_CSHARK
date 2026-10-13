// Which atlas frame the mascot shows: idle, blinking, lip-sync and short reactions, as timed steps. Pure functions;
// the controller (controller.svelte.ts) plays the steps. Frame names are the keys of static/mascot/*-manifest.json.
import type { MascotVariant } from "./assets.svelte";

export interface MascotStep {
  frame: string;
  ms: number;
}

/** Short reactions to what happens in the app. */
export type MascotReaction =
  /** A session started. */
  | "start"
  /** A break started. */
  | "break"
  /** A focus cycle was completed (the next focus starts). */
  | "cycle"
  /** Stop before one full focus phase. */
  | "stopEarly"
  /** A blocked site in the foreground browser. */
  | "nudge"
  /** A meeting reminder. */
  | "meeting"
  /** New work items announced. */
  | "digest";

/** The face the mascot rests on between animations. */
export type MascotMood = "neutral" | "smile" | "sad" | "angry" | "happy";

type Mouth = "a" | "e" | "i" | "o" | "u" | "y";

/** Lip-sync: one character per step, at most this long in total (then the mascot rests, the text stays). */
export const CHAR_MS = 70;
export const TALK_MAX_MS = 6000;
/** A random blink every 3–7 s while the mascot rests with a neutral face. */
export const BLINK_DELAY_MS = { min: 3000, max: 7000 } as const;
/** "Cycle completed": happy frames loop this long. */
const HAPPY_LOOP_MS = 2500;

interface Reaction {
  steps: readonly MascotStep[];
  /** Repeats `steps` until this much time passed. */
  loopMs?: number;
  /** The only frame shown when the user prefers reduced motion (for the same total time). */
  still: string;
}

interface FrameSet {
  /** Eyes open, neutral face. */
  idle: string;
  blink: readonly MascotStep[];
  mouth: Record<Mouth, string>;
  mood: Record<MascotMood, string>;
  reactions: Record<MascotReaction, Reaction>;
}

const step = (frame: string, ms: number): MascotStep => ({ frame, ms });

const MAIN_SMILE: Reaction = {
  steps: [step("smile-1", 160), step("smile-2", 160), step("smile-3", 1000)],
  still: "smile-1",
};

const PEEK_SMILE: Reaction = { steps: [step("smile", 1400)], still: "smile" };

const FRAME_SETS: Record<MascotVariant, FrameSet> = {
  main: {
    idle: "blink-1",
    blink: [step("blink-3", 120)],
    mouth: { a: "mouth-a", e: "mouth-e", i: "mouth-i", o: "mouth-o", u: "mouth-u", y: "mouth-y" },
    mood: { neutral: "blink-1", smile: "smile-1", sad: "sad-1", angry: "angry-1", happy: "happy-1" },
    reactions: {
      // wink-1 and wink-3 have both eyes open; wink-2 is the wink itself, so that one is held.
      start: {
        steps: [step("wink-1", 100), step("wink-2", 700), step("wink-3", 300)],
        still: "wink-2",
      },
      break: MAIN_SMILE,
      cycle: {
        steps: [step("happy-1", 160), step("happy-2", 160), step("happy-3", 160)],
        loopMs: HAPPY_LOOP_MS,
        still: "happy-1",
      },
      stopEarly: { steps: [step("sad-1", 200), step("sad-2", 220), step("sad-3", 1400)], still: "sad-1" },
      nudge: { steps: [step("angry-1", 160), step("angry-2", 160), step("angry-3", 1200)], still: "angry-3" },
      meeting: MAIN_SMILE,
      digest: { steps: [step("happy-1", 160), step("happy-2", 160), step("happy-3", 900)], still: "happy-1" },
    },
  },
  // The peek frames have no O mouth (A instead) and no wink (a blink into a smile instead).
  peek: {
    idle: "idle",
    blink: [step("blink-2", 60), step("blink-3", 90), step("blink-2", 60)],
    mouth: { a: "mouth-a", e: "mouth-e", i: "mouth-i", o: "mouth-a", u: "mouth-u", y: "mouth-y" },
    mood: { neutral: "idle", smile: "smile", sad: "sad", angry: "angry", happy: "happy" },
    reactions: {
      start: { steps: [step("blink-2", 80), step("blink-3", 140), step("blink-2", 80), step("smile", 1200)], still: "smile" },
      break: PEEK_SMILE,
      cycle: { steps: [step("happy", HAPPY_LOOP_MS)], still: "happy" },
      stopEarly: { steps: [step("sad", 1600)], still: "sad" },
      nudge: { steps: [step("angry", 1400)], still: "angry" },
      meeting: PEEK_SMILE,
      digest: { steps: [step("happy", 1600)], still: "happy" },
    },
  },
};

/** Vowels of the UI languages (Latin with diacritics, Cyrillic) and the mouth they show. */
const VOWELS: Record<string, Mouth> = Object.fromEntries(
  (
    [
      ["a", "aąàáâäãая"],
      ["e", "eęèéêëеєэ"],
      ["i", "iìíîïиії"],
      ["y", "yы"],
      ["o", "oóòôöõоё"],
      ["u", "uùúûüуюů"],
    ] as const
  ).flatMap(([mouth, letters]) => Array.from(letters, (letter) => [letter, mouth] as const)),
);

/** A Chinese or Japanese character is a syllable on its own: each one opens the mouth, in turn. */
const CJK_SYLLABLE = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}]/u;
const CJK_MOUTHS: readonly Mouth[] = ["a", "o", "e", "u", "i"];

function mouthFor(ch: string): Mouth | undefined {
  const vowel = VOWELS[ch.toLowerCase()];
  if (vowel !== undefined || !CJK_SYLLABLE.test(ch)) return vowel;
  return CJK_MOUTHS[(ch.codePointAt(0) ?? 0) % CJK_MOUTHS.length];
}

const LETTER_OR_DIGIT = /[\p{L}\p{N}]/u;

/** The resting frame for a mood (neutral = the idle frame). */
export function moodFrame(variant: MascotVariant, mood: MascotMood): string {
  return FRAME_SETS[variant].mood[mood];
}

export function idleFrame(variant: MascotVariant): string {
  return FRAME_SETS[variant].idle;
}

export function blinkSteps(variant: MascotVariant): MascotStep[] {
  return [...FRAME_SETS[variant].blink];
}

/** Steps of a reaction; with reduced motion one still frame for the same time. */
export function reactionSteps(variant: MascotVariant, reaction: MascotReaction, reducedMotion: boolean): MascotStep[] {
  const r = FRAME_SETS[variant].reactions[reaction];
  const once = r.steps.reduce((sum, s) => sum + s.ms, 0);
  const total = r.loopMs ?? once;
  if (reducedMotion) return [step(r.still, total)];
  if (r.loopMs === undefined || once <= 0) return [...r.steps];
  const out: MascotStep[] = [];
  let elapsed = 0;
  while (elapsed < total) {
    for (const s of r.steps) {
      const ms = Math.min(s.ms, total - elapsed);
      if (ms <= 0) break;
      out.push(step(s.frame, ms));
      elapsed += ms;
    }
  }
  return out;
}

/**
 * Vowel lip-sync, one step per character (`CHAR_MS`), at most `TALK_MAX_MS`: a vowel shows its mouth, the first
 * consonant after it keeps that mouth, further consonants show the neutral face, and spaces or punctuation close the
 * mouth (idle frame). Repeated frames are merged into one longer step.
 */
export function lipSyncSteps(text: string, variant: MascotVariant): MascotStep[] {
  const set = FRAME_SETS[variant];
  const out: MascotStep[] = [];
  let elapsed = 0;
  let lastMouth: string | null = null;
  let consonants = 0;
  for (const ch of text) {
    if (elapsed + CHAR_MS > TALK_MAX_MS) break;
    const vowel = mouthFor(ch);
    let frame: string;
    if (vowel !== undefined) {
      frame = set.mouth[vowel];
      lastMouth = frame;
      consonants = 0;
    } else if (LETTER_OR_DIGIT.test(ch)) {
      frame = consonants === 0 && lastMouth !== null ? lastMouth : set.idle;
      consonants++;
    } else {
      frame = set.idle;
      lastMouth = null;
      consonants = 0;
    }
    const previous = out[out.length - 1];
    if (previous && previous.frame === frame) previous.ms += CHAR_MS;
    else out.push(step(frame, CHAR_MS));
    elapsed += CHAR_MS;
  }
  return out;
}
