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
  | "smile"
  /** The user tickled the axolotl on the Skupienie page (a click on it while it says nothing). */
  | "tickle";

/**
 * The face the mascot rests on between animations: `neutral` (no session, a meeting hold) and `calm` (focus running)
 * both rest on the idle frame with random blinks; `calm` also smiles now and then. Beta rewards (ADR-022) add
 * `thinking` (planning the small steps) and `working` (P1 focus companion): held poses, never blinked.
 */
export type MascotMood = "neutral" | "calm" | "smile" | "sad" | "angry" | "happy" | "thinking" | "working";

// ---------------------------------------------------------------------------
// Poses (beta rewards, ADR-022): the owner's 11 poses in their own atlas (`poses-atlas.webp`). A frame name
// `poses:<name>` selects that atlas (Axolotl.svelte); every other name is a frame of the variant's own atlas.
// ---------------------------------------------------------------------------

/** The 11 pose drawings of the owner (art/mascot/poses), by their file names. Only displayed, never redrawn. */
export const POSE_NAMES = [
  "01-point-excited",
  "02-clasped-hands-heart",
  "03-thinking",
  "04-lying-awake",
  "05-look-up",
  "06-amazed-sparkles",
  "07-sleeping",
  "08-back-view",
  "09-lying-look-up",
  "10-holding-heart",
  "11-point-happy",
] as const;

export type PoseName = (typeof POSE_NAMES)[number];

/** Prefix of a frame name that lives in the poses atlas. */
export const POSE_PREFIX = "poses:";

/** The frame name of a pose (`poses:05-look-up`). */
export function poseFrame(name: PoseName): string {
  return `${POSE_PREFIX}${name}`;
}

/** The pose of a `poses:<name>` frame, or null for a frame of the main / peek atlas. */
export function poseOf(frame: string): PoseName | null {
  if (!frame.startsWith(POSE_PREFIX)) return null;
  const name = frame.slice(POSE_PREFIX.length);
  return (POSE_NAMES as readonly string[]).includes(name) ? (name as PoseName) : null;
}

/**
 * The main-atlas frame shown for a pose while the poses atlas loads (or when it failed): a face of the same mood, so a
 * celebration never shows a sad or angry frame in between.
 */
export const POSE_FALLBACK: Record<PoseName, string> = {
  "01-point-excited": "happy-1",
  "02-clasped-hands-heart": "happy-1",
  "03-thinking": "blink-1",
  "04-lying-awake": "smile-1",
  "05-look-up": "blink-1",
  "06-amazed-sparkles": "happy-1",
  "07-sleeping": "smile-1",
  "08-back-view": "blink-1",
  "09-lying-look-up": "smile-1",
  "10-holding-heart": "smile-1",
  "11-point-happy": "happy-1",
};

/** The beats of a reward moment the axolotl plays a pose for (spec ADR-022 §9.5). */
export type RewardBeat =
  | "start"
  | "step"
  | "firstStep"
  | "allSteps"
  | "back"
  | "recap"
  | "surpriseBubble"
  | "surprisePearl"
  | "surpriseGolden"
  | "dailyGoal"
  | "comeback"
  | "streak"
  | "milestone"
  | "restUsed"
  | "restEarned"
  | "levelUp"
  | "pondFull"
  | "earlyStop"
  | "goalDone"
  | "intro";

const pose = (name: PoseName, ms: number): MascotStep => ({ frame: poseFrame(name), ms });

/** Steps per reward beat (main atlas size; the poses are anchored on the main frame's box by Axolotl). */
export const REWARD_STEPS: Record<RewardBeat, readonly MascotStep[]> = {
  start: [pose("11-point-happy", 1200)],
  step: [pose("11-point-happy", 1000)],
  firstStep: [pose("01-point-excited", 1000)],
  allSteps: [pose("02-clasped-hands-heart", 1600)],
  back: [pose("05-look-up", 600), pose("01-point-excited", 1000)],
  recap: [pose("05-look-up", 1000)],
  surpriseBubble: [pose("11-point-happy", 1100)],
  surprisePearl: [pose("06-amazed-sparkles", 1100)],
  surpriseGolden: [pose("10-holding-heart", 1100)],
  dailyGoal: [pose("02-clasped-hands-heart", 1300)],
  comeback: [pose("05-look-up", 500), pose("02-clasped-hands-heart", 800)],
  streak: [pose("10-holding-heart", 1200)],
  milestone: [pose("10-holding-heart", 1500)],
  restUsed: [pose("07-sleeping", 1200)],
  restEarned: [pose("04-lying-awake", 1200)],
  levelUp: [pose("06-amazed-sparkles", 1600)],
  pondFull: [pose("09-lying-look-up", 1500)],
  earlyStop: [pose("10-holding-heart", 2000)],
  goalDone: [pose("02-clasped-hands-heart", 1600)],
  intro: [pose("05-look-up", 800), pose("02-clasped-hands-heart", 4000)],
};

/**
 * Steps of a reward beat, at most `maxMs` long (a beat of a celebration holds its pose for the beat's time). With
 * reduced motion the first frame is held for the whole time (one still pose per beat, like `reactionSteps`).
 */
export function rewardSteps(beat: RewardBeat, reducedMotion: boolean, maxMs = Number.POSITIVE_INFINITY): MascotStep[] {
  const steps = REWARD_STEPS[beat];
  const total = Math.min(
    maxMs,
    steps.reduce((sum, s) => sum + s.ms, 0),
  );
  const first = steps[0];
  if (!first || total <= 0) return [];
  if (reducedMotion) return [step(first.frame, total)];
  const out: MascotStep[] = [];
  let left = total;
  for (const s of steps) {
    const ms = Math.min(s.ms, left);
    if (ms <= 0) break;
    out.push(step(s.frame, ms));
    left -= ms;
  }
  return out;
}

export type Mouth = "a" | "e" | "i" | "o" | "u" | "y";

/** A random blink every 3–7 s while the mascot rests with a neutral or calm face. */
export const BLINK_DELAY_MS = { min: 3000, max: 7000 } as const;
/** A short smile every 20–40 s while the mascot rests with a calm face. */
export const SMILE_DELAY_MS = { min: 20_000, max: 40_000 } as const;
/** Focus companion (beta, P1): every 3–5 min it turns round for a blink and a smile, then sits back. */
export const COMPANION_DELAY_MS = { min: 180_000, max: 300_000 } as const;
export const COMPANION_GLANCE: readonly MascotStep[] = [
  { frame: "blink-1", ms: 400 },
  { frame: "smile-1", ms: 300 },
  { frame: "smile-2", ms: 300 },
  { frame: "smile-3", ms: 300 },
];
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
    mood: {
      neutral: "blink-1",
      calm: "blink-1",
      smile: "smile-1",
      sad: "sad-1",
      angry: "angry-1",
      happy: "happy-1",
      // Beta rewards (ADR-022): held poses from the poses atlas.
      thinking: "poses:03-thinking",
      working: "poses:08-back-view",
    },
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
      // A giggle: the smile opens quickly and is held a moment before the funny line starts.
      tickle: { steps: [step("smile-1", 110), step("smile-2", 110), step("smile-3", 600)], still: "smile-1" },
    },
  },
  // The peek frames have no O mouth (A instead) and no wink (a blink into a smile instead).
  peek: {
    idle: "idle",
    blink: [step("blink-2", 60), step("blink-3", 90), step("blink-2", 60)],
    mouth: { a: "mouth-a", e: "mouth-e", i: "mouth-i", o: "mouth-a", u: "mouth-u", y: "mouth-y" },
    mood: {
      neutral: "idle",
      calm: "idle",
      smile: "smile",
      sad: "sad",
      angry: "angry",
      happy: "happy",
      // The peeking axolotl has no poses (the popup shows only its own frames).
      thinking: "idle",
      working: "idle",
    },
    reactions: {
      start: { steps: [step("blink-2", 80), step("blink-3", 140), step("blink-2", 80), step("smile", 1200)], still: "smile" },
      break: PEEK_SMILE,
      cycle: { steps: [step("happy", HAPPY_LOOP_MS)], still: "happy" },
      stopEarly: { steps: [step("sad", 1600)], still: "sad" },
      nudge: { steps: [step("angry", 1400)], still: "angry" },
      meeting: PEEK_SMILE,
      digest: { steps: [step("happy", 1600)], still: "happy" },
      smile: { steps: [step("smile", 1100)], still: "smile" },
      // Only the main window's axolotl can be tickled; the popup's peeking one ignores clicks (kept for completeness).
      tickle: { steps: [step("smile", 800)], still: "smile" },
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
