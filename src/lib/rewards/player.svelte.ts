// Beta rewards (ADR-022): plays reward moments for one page, one at a time. Skupienie hosts the `focus` scope (Start,
// steps, back from a break, the break recap, an update), Podsumowanie the `summary` scope (session end, goal done).
// It plays only while the window is visible; a moment older than 15 min shows its final state at once, without sound.
// It drives the axolotl through `rewardCues` and asks the shell for at most one sound per moment. Esc or Skip shows
// the final state at once. Never modal: the components draw `current` / `last` in the content flow.
import { untrack } from "svelte";
import { createSubscriber } from "svelte/reactivity";
import { mascotEnabled, rewardCues } from "$lib/mascot/cues.svelte";
import { reactionSteps, rewardSteps, type RewardBeat } from "$lib/mascot/frames";
import { isPixelTheme } from "$lib/mascot/theme.svelte";
import { config } from "$lib/state/config.svelte";
import { rewards } from "$lib/state/rewards.svelte";
import { session } from "$lib/state/session.svelte";
import { strings } from "$lib/strings";
import type { CelebrationLevel, RewardMoment, RewardSound } from "$lib/types";
import {
  announce,
  BEAT_GAP,
  beatsFor,
  burstEnd,
  burstSparks,
  COUNT_MS,
  COUNT_START,
  floatStillFresh,
  hasAchievements,
  hasContent,
  isBurst,
  isFresh,
  MAX_BURST_MS,
  pick,
  PIXEL_COUNT_STEPS,
  poseFor,
  smallMs,
  smallPose,
  soundFor,
  type Beat,
  type RewardScope,
} from "./choreography";

const visibility = createSubscriber((update) => {
  document.addEventListener("visibilitychange", update);
  return () => document.removeEventListener("visibilitychange", update);
});

/** The window is shown (not minimised or hidden in the tray). Reactive. */
export function documentVisible(): boolean {
  visibility();
  return document.visibilityState === "visible";
}

/** One moment on stage. */
export interface PlayInfo {
  moment: RewardMoment;
  /** Beats of a burst (empty for a small moment). */
  beats: readonly Beat[];
  level: CelebrationLevel;
  /** No movement: calm, reduced motion, a quiet moment, a stale one or numbers only (final states, still poses). */
  still: boolean;
  /** Older than 15 min: final state, no sound, no pose. */
  stale: boolean;
  /** The 8-bit theme: stepped motion. */
  pixel: boolean;
  /** The rotating line picked for this play (a float caption, the goal-done chip). */
  caption: string;
  /** Picked at random once per play: chooses the rotating variants of the recap texts (stable while it shows). */
  variant: number;
  /** The number a recap counts up to. */
  total: number;
}

const ease = (t: number) => 1 - (1 - t) ** 3;
/** A beat's pose outlasts the start of the next beat by this much (see `#startBeat`). */
const POSE_OVERLAP_MS = 120;

function caption(m: RewardMoment): string {
  const s = strings.rewards;
  switch (m.kind) {
    case "start":
      return pick(s.startLines);
    case "step":
      return pick(s.stepLines);
    case "allSteps":
      return pick(s.allStepsLines);
    case "backFromBreak":
      return pick(s.backLines);
    case "goalDone":
      return s.goalDoneLine(m.sparks);
    case "sessionEnd":
      return burstSparks(m) > 0 ? s.sessionTotal(burstSparks(m)) : s.sessionPlain;
    default:
      // A recap that paid nothing: the caption only, never "+0".
      return burstSparks(m) > 0 ? pick(s.recapLines(burstSparks(m))) : m.kind === "update" ? s.todayTitle : s.recapPlain;
  }
}

export interface PlayerOptions {
  /** The pose of the recap count-up (default `recap`; the summary uses the early-stop pose after an early Stop). */
  countPose?: () => RewardBeat;
}

export class RewardPlayer {
  readonly scope: RewardScope;
  /** The moment playing now (null between moments). */
  current = $state.raw<PlayInfo | null>(null);
  /** The beat on stage (-1 before the first; `beats.length` once everything shows). */
  beat = $state(-1);
  /** The recap number shown now (counts up). */
  shown = $state(0);
  /** Every beat shows its final state. */
  done = $state(false);
  /** Bumps when confetti should start (a beat with confetti). */
  confetti = $state(0);
  /** The last burst of this page in its final state, until × or the end of its phase. */
  last = $state.raw<PlayInfo | null>(null);
  /** One sentence per moment for the page's polite live region. */
  announcement = $state("");
  /** Components that show a ticked step themselves (the "Teraz:" line); otherwise the float over the ring does. */
  stepHosts = $state(0);

  #timers = new Set<ReturnType<typeof setTimeout>>();
  #frame = 0;
  #countPose: () => RewardBeat;
  #disposed = false;

  constructor(scope: RewardScope, options: PlayerOptions = {}) {
    this.scope = scope;
    this.#countPose = options.countPose ?? (() => "recap");
  }

  /** The burst to draw: the one playing, else the last one in its final state. */
  get burst(): PlayInfo | null {
    const current = this.current;
    if (current && isBurst(current.moment.kind)) return current;
    return this.last;
  }

  play(m: RewardMoment): void {
    if (this.#disposed) return;
    this.#clear();
    rewards.markHandled(m);
    if (!hasContent(m) || !floatStillFresh(m)) {
      // Nothing to show (an update earned in focus: its achievements wait for the break), or a Start / back float the
      // user did not see happen (the window was hidden): nothing moves in focus after the fact.
      if (!m.sample) rewards.ack(session.phase);
      return;
    }
    const stale = !m.sample && !isFresh(m);
    const level = rewards.celebration;
    const still = stale || level !== "full" || m.quiet || rewards.reduced;
    const burst = isBurst(m.kind);
    const firstGoal = m.dailyGoal && (rewards.view?.streak.activeDays ?? 0) <= 1;
    const beats = burst ? beatsFor(m, level, firstGoal, rewards.showStreak) : [];
    const total = burst ? burstSparks(m) : m.sparks;
    const variant = Math.floor(Math.random() * 1000);
    const info: PlayInfo = { moment: m, beats, level, still, stale, pixel: isPixelTheme(), caption: caption(m), variant, total };

    if (burst) this.last = null;
    this.current = info;
    this.done = false;
    this.beat = -1;
    // `still` covers numbers only and calm: the final number at once.
    this.shown = still ? total : 0;
    this.announcement = announce(m, strings.rewards, rewards.showStreak);

    if (stale || level === "numbersOnly") {
      // Numbers only / stale: the final state at once, no pose, no sound (a burst stays as `last`).
      this.#complete(burst ? 0 : smallMs(m.kind));
      return;
    }

    const sound = soundFor(m, beats, level);
    if (!burst) {
      this.#pose(smallPose(m), smallMs(m.kind), still, m);
      if (sound) rewards.sound(sound.sound);
      this.#later(() => this.#complete(0), smallMs(m.kind));
      return;
    }

    for (const [index, beat] of beats.entries()) {
      if (beat.late) continue;
      this.#later(() => this.#startBeat(info, index, sound?.beat === index ? sound.sound : null), beat.at);
    }
    if (!still) this.#later(() => this.#countUp(total, info.pixel), COUNT_START);
    // A recap without a big beat ends with the happy loop (the poses never cover a quiet block).
    if (!hasAchievements(m)) {
      this.#later(() => this.#emit(reactionSteps("main", "cycle", still)), beats[0] ? beats[0].ms : 0);
    }
    this.#later(() => this.#complete(0), Math.min(burstEnd(beats), MAX_BURST_MS));
  }

  /** Esc / Skip: everything in its final state now (the pose already playing ends by itself). */
  finish(): void {
    if (this.current && !this.done) this.#complete(0);
  }

  /** × on the final state: hidden for this break. */
  dismiss(): void {
    this.last = null;
  }

  /** The final state of a burst belongs to its phase (a new break, the next focus). */
  dropLast(): void {
    this.last = null;
  }

  /** Rewards were switched off: a running celebration is discarded (nothing acknowledged, nothing shown). */
  stop(): void {
    this.#clear();
    this.current = null;
    this.last = null;
    this.done = false;
    this.beat = -1;
    this.announcement = "";
  }

  destroy(): void {
    this.#disposed = true;
    this.#clear();
  }

  #startBeat(info: PlayInfo, index: number, sound: RewardSound | null): void {
    if (this.current !== info) return;
    const beat = info.beats[index];
    if (!beat) return;
    this.beat = index;
    const pose = index === 0 ? this.#countPose() : poseFor(beat, info.moment);
    // Hold the pose through the gap and a little past the next beat's start: the next pose replaces it while it still
    // plays, so the axolotl never flicks back to its mood, and the break speech waits for the end of the whole recap.
    this.#pose(pose, beat.ms + BEAT_GAP + POSE_OVERLAP_MS, info.still, info.moment);
    if (sound) rewards.sound(sound);
    if (beat.confetti && !info.still) this.confetti++;
  }

  #countUp(total: number, pixel: boolean): void {
    const startedAt = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - startedAt) / COUNT_MS);
      const progress = pixel ? Math.floor(t * PIXEL_COUNT_STEPS) / PIXEL_COUNT_STEPS : ease(t);
      this.shown = Math.round(total * progress);
      this.#frame = t < 1 && !this.#disposed ? requestAnimationFrame(tick) : 0;
    };
    this.#frame = requestAnimationFrame(tick);
  }

  /**
   * The axolotl's pose for `ms` (a still pose when `still`). The page plays Start, back-from-break and "Tak" itself
   * (from the phase change or the click), so only samples bring their own pose for those.
   */
  #pose(pose: RewardBeat | null, ms: number, still: boolean, m: RewardMoment): void {
    if (pose === null) return;
    const pagePlays = m.kind === "start" || m.kind === "backFromBreak" || m.kind === "goalDone";
    if (pagePlays && !m.sample) return;
    // Exactly `ms` long: cut, or the last frame held (a beat may last longer than its pose).
    const steps = rewardSteps(pose, still, ms);
    const total = steps.reduce((sum, s) => sum + s.ms, 0);
    const last = steps.at(-1);
    if (last && total < ms) steps[steps.length - 1] = { ...last, ms: last.ms + ms - total };
    this.#emit(steps);
  }

  #emit(steps: ReturnType<typeof rewardSteps>): void {
    if (!mascotEnabled(config.value) || rewards.celebration === "numbersOnly") return;
    rewardCues.emit(steps);
  }

  #complete(holdMs: number): void {
    const info = this.current;
    this.#clear();
    if (!info) return;
    this.done = true;
    this.beat = info.beats.length;
    this.shown = info.total;
    const finish = () => {
      if (this.current !== info) return;
      if (isBurst(info.moment.kind)) this.last = info;
      this.current = null;
      if (!info.moment.sample) rewards.ack(session.phase);
    };
    // A small moment in its final state still shows for its time (numbers only: a still chip).
    if (holdMs > 0) this.#later(finish, holdMs);
    else finish();
  }

  #later(fn: () => void, ms: number): void {
    const timer = setTimeout(() => {
      this.#timers.delete(timer);
      if (!this.#disposed) fn();
    }, Math.max(0, ms));
    this.#timers.add(timer);
  }

  #clear(): void {
    for (const timer of this.#timers) clearTimeout(timer);
    this.#timers.clear();
    if (this.#frame) cancelAnimationFrame(this.#frame);
    this.#frame = 0;
  }
}

/**
 * A player for the calling page (call while a component initialises): it takes the next moment of its scope while
 * the window is visible and rewards are active, shows the final state at once when the window gets hidden, and drops
 * a burst's final state when the phase moves on.
 */
export function useRewardPlayer(scope: RewardScope, options: PlayerOptions = {}): RewardPlayer {
  const player = new RewardPlayer(scope, options);

  $effect(() => {
    if (!rewards.active) {
      untrack(() => player.stop());
      return;
    }
    const visible = documentVisible();
    const next = rewards.next(scope, session.phase);
    const playing = player.current !== null;
    if (!visible) {
      if (playing) untrack(() => player.finish());
      return;
    }
    if (next === null || playing) return;
    untrack(() => player.play(next));
  });

  // A burst's final state lasts for its own phase only.
  let phaseKey: string | null = null;
  $effect(() => {
    const key = `${session.phase}:${session.snapshot.cycle}`;
    if (phaseKey !== null && key !== phaseKey) untrack(() => player.dropLast());
    phaseKey = key;
  });

  $effect(() => () => player.destroy());
  return player;
}
