// Beta: sparks, streak and small steps (ADR-022). The rewards view state from the shell (`rewards://changed`,
// revision-guarded, ADR-012), the Developer-page sample layers (memory only, ADR-019), the effective celebration level
// and the queue of moments the players (`$lib/rewards/player.svelte`) show. With `Config.betaFeatures` off nothing here
// renders: every surface checks `active` / `stepsActive` first.
import { MediaQuery, SvelteSet } from "svelte/reactivity";
import * as api from "$lib/api";
import { isFresh, momentKey, scopeOf, type RewardScope } from "$lib/rewards/choreography";
import type {
  AppError,
  CelebrationLevel,
  MicroStep,
  Phase,
  RewardMoment,
  RewardSound,
  RewardsStatus,
  StepDraft,
} from "$lib/types";
import { config } from "./config.svelte";
import { session } from "./session.svelte";
import { showcaseMode } from "./showcaseMode.svelte";
import { toasts } from "./toasts.svelte";

/** Per-viewer convenience (not data): the one-time intro card was dismissed in this WebView. */
const INTRO_SEEN_KEY = "focusmode.rewardsIntroSeen";
/** A moment younger than this is never dropped for its phase being over (events of one change may arrive apart). */
const PHASE_GRACE_MS = 10_000;

function readIntroSeen(): boolean {
  try {
    return localStorage.getItem(INTRO_SEEN_KEY) === "1";
  } catch {
    return false;
  }
}

function writeIntroSeen(seen: boolean): void {
  try {
    if (seen) localStorage.setItem(INTRO_SEEN_KEY, "1");
    else localStorage.removeItem(INTRO_SEEN_KEY);
  } catch {
    // Storage unavailable: the card may show once more in a later run, nothing else.
  }
}

function warn(error: AppError): void {
  console.warn(`[FocusMode] rewards: ${error.code}: ${error.message}`);
}

/**
 * A moment can still show in `phase`: the small focus moments only while their phase lasts, a break recap only in its
 * break, an update only when it carries something (achievements earned in focus wait for the break anyway).
 */
function fitsPhase(m: RewardMoment, phase: Phase): boolean {
  switch (m.kind) {
    case "start":
    case "backFromBreak":
      return phase === "focus";
    case "step":
    case "allSteps":
      return phase !== "idle";
    case "breakStart":
      return phase === "break";
    case "update":
      return phase !== "focus";
    case "sessionEnd":
    case "goalDone":
      return true;
  }
}

class RewardsStore {
  /** From the shell (`get_rewards`, `rewards://changed`); null until the first status (or with an older shell). */
  real = $state.raw<RewardsStatus | null>(null);
  /** Developer page (ADR-019): a sample status shown instead of the real one until cleared. Memory only. */
  sampleStatus = $state.raw<RewardsStatus | null>(null);
  /** Developer page: sample moments (`sample: true`), played like real ones but never acked or sent. */
  sampleMoments = $state.raw<readonly RewardMoment[]>([]);
  /** Developer page: celebration level override (memory only). */
  levelOverride = $state<CelebrationLevel | null>(null);
  /** Developer page: behave as if the OS asked for reduced motion (memory only). */
  simulateReduced = $state(false);
  /** Developer page: a pose sample is held on the Skupienie axolotl, so poses show even with the beta off (memory only). */
  posePreview = $state(false);
  /** The steps editor is focused and empty (the axolotl thinks along: mood `thinking`). */
  planning = $state(false);
  /** The one-time intro card was dismissed (localStorage, per viewer). */
  introSeen = $state(readIntroSeen());

  /** Moments shown, playing or skipped (keys: `momentKey`), so none plays twice. */
  #handled = new SvelteSet<string>();
  #reducedMotion = new MediaQuery("(prefers-reduced-motion: reduce)");
  /** The last `upTo` sent to the shell, until the next status (no duplicate calls in between). */
  #ackSent = 0;

  /** Developer samples are shown. */
  sampling = $derived(this.sampleStatus !== null || this.sampleMoments.length > 0);
  /**
   * `Config.betaFeatures` (off until the config arrives, and with a config from before the switch). The presentation
   * showcase (ADR-019) counts it as on, with the beta defaults below, in memory only (the shell does the same).
   */
  betaOn = $derived(config.value?.betaFeatures === true || showcaseMode.active);
  /** Sparks, levels and streak are on (`Config::rewards_active`), from the local config so a switch shows at once. */
  configActive = $derived(showcaseMode.active || (this.betaOn && config.value?.rewards?.enabled === true));
  /** Every reward surface shows (also for a Developer sample). */
  active = $derived(this.sampling || this.configActive);
  /** Small steps are on (`Config::steps_active`). */
  stepsActive = $derived(showcaseMode.active || (this.betaOn && config.value?.rewards?.microSteps === true));
  /** The status to show: a sample wins over the shell's. */
  view = $derived(this.sampleStatus ?? (this.configActive ? this.real : null));
  /** "Pokazuj serię" (the streak is still computed when it is hidden). */
  showStreak = $derived(showcaseMode.active || config.value?.rewards?.showStreak !== false);
  /** OS reduced motion, or its simulation on the Developer page. */
  reduced = $derived(this.simulateReduced || this.#reducedMotion.current);

  /** The effective level: numbers only stays; reduced motion caps the rest at calm; a Developer override wins. */
  celebration = $derived.by((): CelebrationLevel => {
    const configured = this.levelOverride ?? (showcaseMode.active ? "full" : (config.value?.rewards?.celebrations ?? "full"));
    if (configured === "numbersOnly") return "numbersOnly";
    return this.reduced ? "calm" : configured;
  });

  /** Reward sounds may be requested (the shell checks again: presenting, phase chime, rewards active). */
  soundsOn = $derived(config.value?.rewards?.sounds !== false && this.celebration !== "numbersOnly");

  /** The real pending moments (while active) and the sample ones, oldest first. */
  queue = $derived.by((): RewardMoment[] => {
    const real = this.configActive ? (this.real?.pending ?? []) : [];
    return [...this.sampleMoments, ...real];
  });

  apply(status: RewardsStatus): void {
    this.real = status;
    this.#ackSent = 0;
  }

  /** `get_rewards` / `rewards://changed` errors: decorative state, so only logged (never a toast). */
  reportError(error: AppError): void {
    if (error.code === api.ERR_NO_SHELL) return;
    warn(error);
  }

  /**
   * The oldest moment for `scope` that has not been shown yet and still fits the session (`phase`); stale real ones
   * (older than 15 min) are taken too: the player shows them static and acks them at once. Reactive.
   */
  next(scope: RewardScope, phase: Phase): RewardMoment | null {
    if (!this.active) return null;
    for (const m of this.queue) {
      if (this.#handled.has(momentKey(m)) || scopeOf(m.kind) !== scope) continue;
      if (m.sample || fitsPhase(m, phase)) return m;
    }
    return null;
  }

  /** The moment will not play again (shown, playing or skipped). */
  markHandled(m: RewardMoment): void {
    this.#handled.add(momentKey(m));
  }

  /**
   * Tells the shell which moments are done: up to the newest id below which every pending moment was handled, stale,
   * or can no longer show (its phase is over), so a moment waiting for the other page is never dropped. Samples never
   * reach the shell.
   */
  ack(phase: Phase = session.phase): void {
    const pending = this.real?.pending ?? [];
    const now = Date.now();
    const done = (m: RewardMoment): boolean => {
      // A focus-page moment whose phase is over can no longer show (an update can, later). The grace keeps a moment
      // that arrived a moment before its phase change reached this window.
      const over =
        scopeOf(m.kind) === "focus" && m.kind !== "update" && !fitsPhase(m, phase) && now - m.atMs > PHASE_GRACE_MS;
      return this.#handled.has(momentKey(m)) || !isFresh(m, now) || over;
    };
    // Ids are not always in order (the demo preview numbers its own): never acknowledge past a moment still to show.
    let firstOpen = Number.POSITIVE_INFINITY;
    for (const m of pending) if (!done(m)) firstOpen = Math.min(firstOpen, m.id);
    let upTo = 0;
    for (const m of pending) if (m.id < firstOpen && done(m)) upTo = Math.max(upTo, m.id);
    if (upTo === 0 || upTo === this.#ackSent) return;
    this.#ackSent = upTo;
    api.ackRewardMoments(upTo).catch((e: unknown) => this.reportError(api.toAppError(e)));
  }

  /** A reward sound (one per celebration at most); the shell may still skip it. */
  sound(sound: RewardSound): void {
    if (!this.soundsOn) return;
    api.playRewardSound(sound).catch((e: unknown) => this.reportError(api.toAppError(e)));
  }

  /** Replaces the session's small steps (Idle: the coming session's plan). Resolves false when the shell refused. */
  async setSteps(drafts: StepDraft[]): Promise<boolean> {
    try {
      await api.setSessionSteps(drafts);
      await session.reload();
      return true;
    } catch (e) {
      toasts.error(e);
      return false;
    }
  }

  /** Ticks / unticks a step (Focus or Break). Resolves false when the shell refused. */
  async setStepDone(id: number, done: boolean): Promise<boolean> {
    try {
      await api.setStepDone(id, done);
      await session.reload();
      return true;
    } catch (e) {
      toasts.error(e);
      return false;
    }
  }

  /** "Zacznij od nowa" (the user confirmed): a new ledger. Throws, so the dialog shows the error and stays open. */
  async reset(): Promise<void> {
    const status = await api.resetRewards();
    this.real = status;
    this.#handled.clear();
    this.#ackSent = 0;
  }

  dismissIntro(): void {
    this.introSeen = true;
    writeIntroSeen(true);
  }

  /** Developer page: the intro card shows again. */
  showIntroAgain(): void {
    this.introSeen = false;
    writeIntroSeen(false);
  }

  /** Developer page: plays `moment` (a sample) on its page, and shows a sample status when none is shown yet. */
  addSample(moment: RewardMoment, status: () => RewardsStatus): void {
    this.sampleStatus ??= status();
    this.sampleMoments = [...this.sampleMoments, moment];
  }

  /** Drops every reward sample layer and the Developer overrides: the real state shows again. */
  clearSamples(): void {
    for (const m of this.sampleMoments) this.#handled.delete(momentKey(m));
    this.sampleMoments = [];
    this.sampleStatus = null;
    this.levelOverride = null;
    this.simulateReduced = false;
    this.posePreview = false;
  }
}

export const rewards = new RewardsStore();

/** The steps of the coming or running session (an older shell sends none). */
export function sessionSteps(): MicroStep[] {
  return session.snapshot.steps ?? [];
}

/** The first open step, or null. */
export function nextOpenStep(steps: readonly MicroStep[]): MicroStep | null {
  return steps.find((s) => !s.done) ?? null;
}
