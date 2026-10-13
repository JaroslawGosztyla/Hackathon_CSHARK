// Beta rewards (ADR-022): what a reward moment looks like over time. Pure functions and named timings only; the player
// (player.svelte.ts) runs them, the components (components/rewards) draw them. Rules: one celebration at a time, never
// modal, never over the session buttons; in focus only the user's own Start, tick and back-from-break move.
import type { RewardBeat } from "$lib/mascot/frames";
import type { Strings } from "$lib/i18n/pl";
import type { CelebrationLevel, MomentKind, Rarity, RewardLine, RewardMoment, RewardSound, StickerId } from "$lib/types";

// ---------------------------------------------------------------------------
// Timings (ms). Full and Calm use the same durations; Calm shows each beat's final state at once.
// ---------------------------------------------------------------------------

/** The burst card mounts at "+0"; the count-up starts this much later. */
export const COUNT_START = 200;
/** Count-up 0 → N (ease-out; the 8-bit theme counts in `PIXEL_COUNT_STEPS` steps). */
export const COUNT_MS = 800;
export const PIXEL_COUNT_STEPS = 8;
/** Line chips fade in one after another (at most `MAX_CHIPS`, the rest as "+N"). */
export const CHIP_STAGGER = 90;
export const MAX_CHIPS = 6;
/** Pause between two beats of a burst. */
export const BEAT_GAP = 150;
export const SURPRISE_MS = 1100;
/** The surprise card flips this long. */
export const SURPRISE_FLIP_MS = 300;
export const GOAL_MS = 1300;
/** The daily-goal ring closes this long. */
export const GOAL_RING_MS = 600;
export const COMEBACK_MS = 1300;
export const STREAK_MS = 1200;
export const MILESTONE_MS = 1500;
export const REST_MS = 1200;
export const LEVEL_MS = 1600;
/** Level up: the bar flashes, then "Poziom N!" pops (0.8 → 1.05 → 1). */
export const LEVEL_FLASH_MS = 150;
export const LEVEL_POP_MS = 300;
export const CONFETTI_MS = 1200;
/** At most this many confetti particles; the 8-bit theme draws them at `PIXEL_FPS`. */
export const CONFETTI_MAX = 60;
export const POND_MS = 1500;
export const STICKER_MS = 1200;
/** Stickers get their own beat up to this many, then one "+N w albumie" line. */
export const MAX_STICKER_BEATS = 2;
/** Beats that would start after this render in their final state at once. */
export const MAX_BURST_MS = 8000;

/** Start / back from break: the "+N ✦" float rises and fades (Calm: a still chip for the same time). */
export const FLOAT_MS = 900;
/** Float rise in px (the 8-bit theme moves in `PIXEL_FLOAT_STEP` px steps). */
export const FLOAT_RISE_PX = 24;
export const PIXEL_FLOAT_STEP = 4;
/** A ticked step: the checkbox fills, 6 dots burst, the "+3" float shows, the next step slides up. */
export const STEP_FILL_MS = 150;
export const STEP_BURST_MS = 400;
export const STEP_FLOAT_MS = 600;
export const STEP_SLIDE_MS = 250;
export const STEP_DOTS = 6;
export const STEP_MS = 1000;
/** "Wszystkie kroki! +5". */
export const ALL_STEPS_MS = 1600;
/** "Cel zrobiony! +10" and the goal box glow. */
export const GOAL_DONE_MS = 1200;
export const GOAL_GLOW_MS = 600;

/** The 8-bit theme animates at about this frame rate (`steps()` easing). */
export const PIXEL_FPS = 10;

/** A moment older than this shows static, without sound (and is acknowledged at once). */
export const FRESH_MS = 15 * 60 * 1000;
/**
 * The Start and back-from-break floats move in focus only right after the user's own action: older ones (the window
 * was hidden, or another page was open) are skipped, since nothing may move in focus after the fact.
 */
export const FOCUS_FLOAT_FRESH_MS = 10_000;

/** Mirrors focus-core `rewards.rs` (`REST_EARN_EVERY`, `MAX_STEPS`, `MAX_STEP_CHARS`) for the texts and the editor. */
export const REST_EARN_EVERY = 5;
export const MAX_STEPS = 5;
export const MAX_STEP_CHARS = 80;

// ---------------------------------------------------------------------------
// Moments
// ---------------------------------------------------------------------------

/** The page that shows a moment: Skupienie (`focus`) or Podsumowanie (`summary`). */
export type RewardScope = "focus" | "summary";

export function scopeOf(kind: MomentKind): RewardScope {
  return kind === "sessionEnd" || kind === "goalDone" ? "summary" : "focus";
}

/** A recap card with beats (break start, session end, an update outside focus). */
export function isBurst(kind: MomentKind): boolean {
  return kind === "breakStart" || kind === "sessionEnd" || kind === "update";
}

/**
 * A moment's identity in this window. Ids grow within an app run, but start again after "Zacznij od nowa" or a delete
 * in the privacy panel, and the demo preview numbers its own: the wall-clock time and the source keep keys unique.
 */
export function momentKey(m: RewardMoment): string {
  return `${m.sample ? "s" : m.preview ? "p" : "r"}:${m.id}:${m.atMs}`;
}

export function isFresh(m: RewardMoment, now = Date.now()): boolean {
  return now - m.atMs <= FRESH_MS;
}

/** A Start or back-from-break float that may still move in focus (see `FOCUS_FLOAT_FRESH_MS`). */
export function floatStillFresh(m: RewardMoment, now = Date.now()): boolean {
  if (m.sample || (m.kind !== "start" && m.kind !== "backFromBreak")) return true;
  return now - m.atMs <= FOCUS_FLOAT_FRESH_MS;
}

/** The sparks a recap counts up to: the session total at the end of a session, else the moment's own. */
export function burstSparks(m: RewardMoment): number {
  return m.kind === "sessionEnd" ? Math.max(m.sessionTotal, m.sparks) : m.sparks;
}

/** The moment earned something beyond its lines (a beat after the count-up). */
export function hasAchievements(m: RewardMoment): boolean {
  return (
    m.surprise !== null ||
    m.dailyGoal ||
    m.comeback ||
    m.newStreak ||
    m.streak !== null ||
    m.restDaysUsed > 0 ||
    m.restDayEarned ||
    m.levelUp !== null ||
    m.pondFull ||
    m.stickers.length > 0
  );
}

/** Worth showing at all (an update earned in focus carries nothing: its achievements wait for the break). */
export function hasContent(m: RewardMoment): boolean {
  return m.sparks > 0 || m.sessionTotal > 0 || m.lines.length > 0 || hasAchievements(m) || !isBurst(m.kind);
}

// ---------------------------------------------------------------------------
// Beats of a burst
// ---------------------------------------------------------------------------

export type BeatKind =
  | "count"
  | "surprise"
  | "dailyGoal"
  | "comeback"
  | "streak"
  | "restUsed"
  | "restEarned"
  | "levelUp"
  | "pondFull"
  | "sticker"
  | "stickersMore";

export interface Beat {
  kind: BeatKind;
  /** Start, ms after the moment started. */
  at: number;
  ms: number;
  /** Starts after `MAX_BURST_MS`: shown in its final state at once, without its pose. */
  late: boolean;
  /** Confetti on this beat (level up, a golden pearl, the very first daily goal). */
  confetti: boolean;
  /** The sticker of a `sticker` beat. */
  sticker: StickerId | null;
}

interface BeatSpec {
  kind: BeatKind;
  ms: number;
  confetti?: boolean;
  sticker?: StickerId | null;
}

/** Count-up time with its chips (they fade in while the number counts). */
function countMs(lines: number): number {
  return COUNT_START + Math.max(COUNT_MS, Math.min(lines, MAX_CHIPS) * CHIP_STAGGER);
}

/**
 * The beats of a burst, in the fixed order: count-up → surprise → daily goal → comeback → streak (+ milestone) →
 * rest days → level up → pond full → stickers. `firstGoal`: the very first daily goal (confetti). With "numbers only"
 * every beat starts at once (a static list). `showStreak` off ("Pokazuj serię"): no streak or rest-day beats, so no
 * pose, sound or announcement about a streak that is not shown.
 */
export function beatsFor(m: RewardMoment, level: CelebrationLevel, firstGoal = false, showStreak = true): Beat[] {
  const specs: BeatSpec[] = [{ kind: "count", ms: countMs(m.lines.length) }];
  if (m.surprise !== null) specs.push({ kind: "surprise", ms: SURPRISE_MS, confetti: m.surprise === "goldenPearl" });
  if (m.dailyGoal) specs.push({ kind: "dailyGoal", ms: GOAL_MS, confetti: firstGoal });
  if (m.comeback) specs.push({ kind: "comeback", ms: COMEBACK_MS });
  if (showStreak) {
    if (m.streak !== null || m.newStreak) specs.push({ kind: "streak", ms: m.milestone !== null ? MILESTONE_MS : STREAK_MS });
    if (m.restDaysUsed > 0) specs.push({ kind: "restUsed", ms: REST_MS });
    if (m.restDayEarned) specs.push({ kind: "restEarned", ms: REST_MS });
  }
  if (m.levelUp !== null) specs.push({ kind: "levelUp", ms: LEVEL_MS, confetti: true });
  if (m.pondFull) specs.push({ kind: "pondFull", ms: POND_MS });
  for (const sticker of m.stickers.slice(0, MAX_STICKER_BEATS)) specs.push({ kind: "sticker", ms: STICKER_MS, sticker });
  if (m.stickers.length > MAX_STICKER_BEATS) specs.push({ kind: "stickersMore", ms: STICKER_MS });

  const beats: Beat[] = [];
  let at = 0;
  for (const spec of specs) {
    const start = level === "numbersOnly" ? 0 : at;
    beats.push({
      kind: spec.kind,
      at: start,
      ms: level === "numbersOnly" ? 0 : spec.ms,
      late: level !== "numbersOnly" && start > MAX_BURST_MS,
      confetti: spec.confetti === true,
      sticker: spec.sticker ?? null,
    });
    at += spec.ms + BEAT_GAP;
  }
  return beats;
}

/** When the last beat that plays ends (late beats show at once at `MAX_BURST_MS`). */
export function burstEnd(beats: readonly Beat[]): number {
  let end = 0;
  for (const beat of beats) end = Math.max(end, beat.late ? MAX_BURST_MS : beat.at + beat.ms);
  return end;
}

/** The axolotl's pose for a beat (§9.5); null: no pose (it keeps its mood). */
export function poseFor(beat: Beat, m: RewardMoment): RewardBeat | null {
  switch (beat.kind) {
    case "count":
      return "recap";
    case "surprise":
      return m.surprise === "goldenPearl" ? "surpriseGolden" : m.surprise === "pearl" ? "surprisePearl" : "surpriseBubble";
    case "dailyGoal":
      return "dailyGoal";
    case "comeback":
      return "comeback";
    case "streak":
      return m.milestone !== null ? "milestone" : "streak";
    case "restUsed":
      return "restUsed";
    case "restEarned":
      return "restEarned";
    case "levelUp":
      return "levelUp";
    case "pondFull":
      return "pondFull";
    case "sticker":
    case "stickersMore":
      return null;
  }
}

/** The pose of a small moment (start, steps, back from break, goal done). */
export function smallPose(m: RewardMoment): RewardBeat | null {
  switch (m.kind) {
    case "start":
      return "start";
    case "step":
      return m.firstStep ? "firstStep" : "step";
    case "allSteps":
      return "allSteps";
    case "backFromBreak":
      return "back";
    case "goalDone":
      return "goalDone";
    default:
      return null;
  }
}

/** How long a small moment shows. */
export function smallMs(kind: MomentKind): number {
  switch (kind) {
    case "step":
      return STEP_MS;
    case "allSteps":
      return ALL_STEPS_MS;
    case "goalDone":
      return GOAL_DONE_MS;
    default:
      return FLOAT_MS;
  }
}

// ---------------------------------------------------------------------------
// Sound: at most one per moment, requested at the start of the beat it belongs to
// ---------------------------------------------------------------------------

function soundRank(sound: RewardSound | null): number {
  return sound === "fanfare" ? 3 : sound === "sparkle" ? 2 : sound === "pop" ? 1 : 0;
}

/** The sound of one beat (none for a plain recap: the phase chime already played). */
export function beatSound(beat: Beat, m: RewardMoment): RewardSound | null {
  switch (beat.kind) {
    case "levelUp":
      return "fanfare";
    case "surprise":
      return m.surprise === "goldenPearl" ? "fanfare" : m.surprise === "pearl" ? "sparkle" : null;
    case "dailyGoal":
    case "comeback":
      return "sparkle";
    case "streak":
      return m.milestone !== null ? "sparkle" : null;
    default:
      return null;
  }
}

/**
 * The one sound of a moment and when it is requested (index of its beat; 0 for a small moment), or null: fanfare for
 * a level up or a golden pearl, else sparkle for a daily goal, a milestone, a comeback or a pearl; pop for a ticked
 * step; nothing with "numbers only", for a quiet moment (held break, music muted) or a plain block.
 */
export function soundFor(
  m: RewardMoment,
  beats: readonly Beat[],
  level: CelebrationLevel,
): { sound: RewardSound; beat: number } | null {
  if (level === "numbersOnly" || m.quiet) return null;
  if (!isBurst(m.kind)) {
    if (m.kind === "step" || m.kind === "allSteps") return { sound: "pop", beat: 0 };
    if (m.kind === "goalDone") return { sound: "sparkle", beat: 0 };
    return null;
  }
  let best: { sound: RewardSound; beat: number } | null = null;
  for (const [index, beat] of beats.entries()) {
    if (beat.late) continue;
    const sound = beatSound(beat, m);
    if (sound !== null && soundRank(sound) > soundRank(best?.sound ?? null)) best = { sound, beat: index };
  }
  return best;
}

// ---------------------------------------------------------------------------
// Texts
// ---------------------------------------------------------------------------

type RewardStrings = Strings["rewards"];

/** One of the rotating variants (2–3 per moment), at random. */
export function pick(lines: readonly string[]): string {
  return lines[Math.floor(Math.random() * lines.length)] ?? lines[0] ?? "";
}

/** The variant number `n` (a play picks `n` at random once, so re-renders keep the same text). */
export function pickAt(lines: readonly string[], n: number): string {
  const count = lines.length;
  return count > 0 ? (lines[((Math.trunc(n) % count) + count) % count] ?? "") : "";
}

export function rarityName(r: Rarity, s: RewardStrings): string {
  return s.rarity[r];
}

/** The label of a line chip ("Pełny blok", "Skupienie", "Kroki ×3"). */
export function lineLabel(line: RewardLine, s: RewardStrings): string {
  const labels = s.line;
  switch (line.kind) {
    case "step":
      return labels.step(Math.max(1, line.count));
    default:
      return labels[line.kind];
  }
}

/** The surprise's sparks (from its line). */
export function surpriseSparks(m: RewardMoment): number {
  return m.lines.find((l) => l.kind === "surprise")?.sparks ?? 0;
}

/** The sparks of one kind in a moment. */
export function lineSparks(m: RewardMoment, kind: RewardLine["kind"]): number {
  return m.lines.find((l) => l.kind === kind)?.sparks ?? 0;
}

/**
 * One sentence for the polite live region, e.g. "Zdobyto 72 iskierki. Cel dnia. Seria: 5. Poziom 8." In focus only
 * Start and a ticked step are announced; back from a break is announced by the phase change already. The streak is
 * left out while it is not shown (`showStreak`).
 */
export function announce(m: RewardMoment, s: RewardStrings, showStreak = true): string {
  switch (m.kind) {
    case "start":
      return s.announceSparks(m.sparks);
    case "step":
      return m.sparks > 0 ? s.announceStep(m.sparks) : s.announceStepPlain;
    case "allSteps":
      return `${s.allDone} ${s.announceSparks(m.sparks)}`;
    case "backFromBreak":
      return "";
    case "goalDone":
      return s.goalDoneLine(m.sparks);
    default: {
      const parts = [s.announceSparks(burstSparks(m))];
      if (m.surprise !== null) parts.push(s.announceSurprise(rarityName(m.surprise, s)));
      if (m.dailyGoal) parts.push(s.announceDailyGoal);
      if (m.streak !== null && showStreak) parts.push(s.announceStreak(m.streak));
      if (m.levelUp !== null) parts.push(s.announceLevel(m.levelUp));
      return parts.join(" ");
    }
  }
}
