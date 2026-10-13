// Which atlas frame the mascot shows: rest (mood), blinks, short reactions and the mouth for a spoken letter.
// Pure functions; the controller (controller.svelte.ts) plays the steps and the speech (speech.svelte.ts) drives the
// mouth. Frame names are the keys of static/mascot/*-manifest.json.
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
  /** A focus cycle was completed (the next focus starts), or the session goal was done. */
  | "cycle"
  /** Stop before one full focus phase. */
  | "stopEarly"
  /** A blocked site in the foreground browser. */
  | "nudge"
  /** A meeting reminder. */
  | "meeting"
  /** New work items announced. */
  | "digest"
  /** A short smile now and then while the user focuses (mood `calm`). */
  | "smile";

/**
 * The face the mascot rests on between animations: `neutral` (no session, a meeting hold) and `calm` (focus running)
 * both rest on the idle frame with random blinks; `calm` also smiles now and then.
 */
export type MascotMood = "neutral" | "calm" | "smile" | "sad" | "angry" | "happy";

export type Mouth = "a" | "e" | "i" | "o" | "u" | "y";

/** A random blink every 3–7 s while the mascot rests with a neutral or calm face. */
export const BLINK_DELAY_MS = { min: 3000, max: 7000 } as const;
/** A short smile every 20–40 s while the mascot rests with a calm face. */
export const SMILE_DELAY_MS = { min: 20_000, max: 40_000 } as const;
/**
 * While talking, a mouth frame stays at least this long before the next one shows (the text keeps its pace). The frames
 * are cut from separate drawings and differ slightly beyond the mouth, so switching them per letter looks jittery.
 */
export const MOUTH_HOLD_MS = 200;
/** "Cycle completed": happy frames loop this long. */
const HAPPY_LOOP_MS = 2500;
/** A blocked site: one angry frame, held this long. */
const ANGRY_MS = 3000;

interface Reaction {
  steps: readonly MascotStep[];
  /** Repeats `steps` until this much time passed. */
  loopMs?: number;
  /** The only frame shown when the user prefers reduced motion (for the same total time). */
  still: string;
  /** Instead of `steps`: one of these frames, picked at random and held (never animated), for `ms`. */
  pick?: { frames: readonly string[]; ms: number };
}

interface FrameSet {
  /** Eyes open, neutral face, mouth closed. */
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
    mood: { neutral: "blink-1", calm: "blink-1", smile: "smile-1", sad: "sad-1", angry: "angry-1", happy: "happy-1" },
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
      stopEarly: { steps: [step("sad-1", 200), step("sad-2", 220), step("sad-3", 2580)], still: "sad-1" },
      // angry-1 and angry-2 are the same pose (owner): one of the two distinct ones, never animated.
      nudge: { steps: [], pick: { frames: ["angry-1", "angry-3"], ms: ANGRY_MS }, still: "angry-1" },
      meeting: MAIN_SMILE,
      digest: { steps: [step("happy-1", 160), step("happy-2", 160), step("happy-3", 900)], still: "happy-1" },
      smile: { steps: [step("smile-1", 120), step("smile-2", 120), step("smile-3", 900)], still: "smile-1" },
    },
  },
  // The peek frames have no O mouth (A instead) and no wink (a blink into a smile instead).
  peek: {
    idle: "idle",
    blink: [step("blink-2", 60), step("blink-3", 90), step("blink-2", 60)],
    mouth: { a: "mouth-a", e: "mouth-e", i: "mouth-i", o: "mouth-a", u: "mouth-u", y: "mouth-y" },
    mood: { neutral: "idle", calm: "idle", smile: "smile", sad: "sad", angry: "angry", happy: "happy" },
    reactions: {
      start: { steps: [step("blink-2", 80), step("blink-3", 140), step("blink-2", 80), step("smile", 1200)], still: "smile" },
      break: PEEK_SMILE,
      cycle: { steps: [step("happy", HAPPY_LOOP_MS)], still: "happy" },
      stopEarly: { steps: [step("sad", 1600)], still: "sad" },
      nudge: { steps: [step("angry", 1400)], still: "angry" },
      meeting: PEEK_SMILE,
      digest: { steps: [step("happy", 1600)], still: "happy" },
      smile: { steps: [step("smile", 1100)], still: "smile" },
    },
  },
};

/**
 * Vowels of the UI languages and the mouth they show: Polish and the other Latin languages with their accents, and
 * Cyrillic (я and ю sound like "ja" / "ju", so they open the A / U mouth).
 */
const VOWELS: Record<string, Mouth> = Object.fromEntries(
  (
    [
      ["a", "aąàáâäãåая"],
      ["e", "eęèéêëěеэєё"],
      ["i", "iìíîïıиіїй"],
      ["o", "oóòôöõøо"],
      ["u", "uùúûüůую"],
      ["y", "yýÿы"],
    ] as const
  ).flatMap(([mouth, letters]) => Array.from(letters, (letter) => [letter, mouth] as const)),
);

/** A Chinese or Japanese character is a syllable of its own: the mouth opens A, O, U in turn. */
const CJK_SYLLABLE = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/u;
const CJK_MOUTHS: readonly Mouth[] = ["a", "o", "u"];

/** Counts as a CJK syllable for `mouthFor`. */
export function isSyllable(grapheme: string): boolean {
  return CJK_SYLLABLE.test(grapheme);
}

/**
 * The mouth for one spoken character (a grapheme), or null for a consonant, a space or punctuation (mouth closed).
 * `syllable` counts the CJK characters spoken before this one in the message (it picks the next of A / O / U).
 */
export function mouthFor(grapheme: string, syllable: number): Mouth | null {
  const lower = grapheme.normalize("NFC").toLowerCase();
  const vowel = VOWELS[lower] ?? VOWELS[lower.normalize("NFD").charAt(0)];
  if (vowel !== undefined) return vowel;
  if (!isSyllable(grapheme)) return null;
  return CJK_MOUTHS[syllable % CJK_MOUTHS.length] ?? "a";
}

/** The frame for a mouth; null (closed mouth) is the idle frame. */
export function mouthFrame(variant: MascotVariant, mouth: Mouth | null): string {
  const set = FRAME_SETS[variant];
  return mouth === null ? set.idle : set.mouth[mouth];
}

/** The resting frame for a mood (neutral and calm = the idle frame). */
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
  if (r.pick) {
    const frame = r.pick.frames[Math.floor(Math.random() * r.pick.frames.length)] ?? r.still;
    return [step(frame, r.pick.ms)];
  }
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
