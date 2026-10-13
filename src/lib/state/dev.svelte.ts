// Developer mode (ADR-019): the state of the Developer page and the actions of its buttons. Everything here lives in
// memory only: the "Clean screenshots" flag, the held mascot mood, the queue of main-window requests and the sample
// layers it puts into the stores (summary, "Co dalej?", calendar, goal, meeting hold). No sample is saved, sent to the
// shell as data or sent over the network; the shell only gets `dev_trigger` (sample popups, toasts, a shorter phase,
// presentation mode for this run, and the factory reset, which deletes the app's data and restarts it).
// The "Look & feel" and "Window & app" buttons are shortcuts to real settings: they save `config.json` like the
// Settings page, and the chime test switches `chimeSound` for one play and saves the user's choice back (ADR-019).
//
// Main-window effects open the tab where they are visible (Skupienie / Podsumowanie) and fire once that page has
// mounted: the button queues a request and navigates, the component that shows the effect takes it with
// `useDevRequests` and runs it after the tab slide.
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { untrack } from "svelte";
import * as api from "$lib/api";
import { DEV, DEV_REWARDS, POPUP_SAMPLES, type PopupSample } from "$lib/i18n/dev";
import type { MascotMood, MascotReaction } from "$lib/mascot/frames";
import { errorText, strings } from "$lib/strings";
import type { ChimeKind, ChimeSound, DevAction, Language, MeetingChoice, Phase, Theme } from "$lib/types";
import { calendar } from "./calendar.svelte";
import { config } from "./config.svelte";
import { nudge } from "./nudge.svelte";
import { rewards } from "./rewards.svelte";
import { sampleCalendar, sampleGoal, sampleHold, sampleNudge, sampleSummaries, sampleWork, type BreakPlan } from "./samples";
import { session } from "./session.svelte";
import { summary } from "./summary.svelte";
import { tint } from "./tint.svelte";
import { toasts } from "./toasts.svelte";
import { work } from "./work.svelte";

/** A main-window effect fires this long after its page mounted: the tab slide (app.css, ~460 ms) has ended. */
const FIRE_DELAY_MS = 500;
/** A request nobody took within this time is dropped (e.g. the mascot is hidden), so it never fires much later. */
const STALE_MS = 10_000;
/** "Play all popups": the next popup after this long. */
export const SEQUENCE_STEP_MS = 5000;
/** Used for the sample calendar plan when neither the session nor the config knows the lengths (focus-core defaults). */
const FALLBACK_FOCUS_MINUTES = 45;
const FALLBACK_BREAK_MINUTES = 15;
/**
 * `TimingProfile::demo()` (focus-core `timing.rs`), for the "Presentation mode" label only; once the snapshot is in
 * the demo profile the label shows the snapshot's own lengths.
 */
const DEMO_FOCUS_SECS = 40;
const DEMO_BREAK_SECS = 20;
const SECOND_MS = 1000;
const MINUTE_SECS = 60;

/** What the Skupienie page's axolotl is asked to do. */
export type MascotDevAction =
  | { type: "react"; reaction: MascotReaction }
  | { type: "tickle" }
  | { type: "say"; text: string; label: string | null };

/** A main-window effect waiting for its page: the axolotl, the close prompt, or something the layout shows. */
export type DevRequest =
  | { target: "mascot"; action: MascotDevAction }
  | { target: "closePrompt" }
  | { target: "layout"; run: () => void };

type DevTarget = DevRequest["target"];

interface QueuedRequest {
  id: number;
  /** `performance.now()` when the button was pressed. */
  at: number;
  request: DevRequest;
}

export interface DevStatus {
  tone: "ok" | "error";
  text: string;
}

/** Popup samples in the order of "Play all popups". */
const SEQUENCE: readonly PopupSample[] = POPUP_SAMPLES;

/** The planned breaks of the current session, or of one that would start now (for the sample calendar). */
function breakPlan(now: number): BreakPlan {
  const snap = session.snapshot;
  const cfg = config.value;
  const focusSecs = snap.focusSecs > 0 ? snap.focusSecs : (cfg?.focusMinutes ?? FALLBACK_FOCUS_MINUTES) * MINUTE_SECS;
  const breakSecs = snap.breakSecs > 0 ? snap.breakSecs : (cfg?.breakMinutes ?? FALLBACK_BREAK_MINUTES) * MINUTE_SECS;
  if (snap.phase === "focus") {
    return { firstBreakStartMs: now + snap.remainingSecs * SECOND_MS, focusSecs, breakSecs, planned: false };
  }
  if (snap.phase === "break") {
    return { firstBreakStartMs: now + (snap.remainingSecs + focusSecs) * SECOND_MS, focusSecs, breakSecs, planned: false };
  }
  return { firstBreakStartMs: now + focusSecs * SECOND_MS, focusSecs, breakSecs, planned: true };
}

class DevStore {
  /** "Clean screenshots": the Developer tab is hidden until Ctrl+Shift+D or Settings brings it back. */
  cleanScreenshots = $state(false);
  /** "Keep popup open": sample popups are sticky (no countdown, no auto-hide). */
  keepPopupOpen = $state(false);
  /** A mood held on the Skupienie page's axolotl; null = it follows the session. */
  mascotMood = $state<MascotMood | null>(null);
  /** Main-window requests waiting for their page. */
  queue = $state.raw<readonly QueuedRequest[]>([]);
  /** The last button's result (one line on the Developer page). */
  status = $state.raw<DevStatus | null>(null);
  /** "Play all popups" is running: the popup shown last (1-based) of all of them. */
  sequence = $state.raw<{ step: number; total: number } | null>(null);
  /** A chime test (which briefly switches the chime sound) is running. */
  chimeBusy = $state(false);
  /**
   * "Presentation mode" switched while a session ran (the shell applies it at the next Start): the wanted value and
   * the session it was switched in. Ignored once that session is over (the snapshot then shows the real state).
   */
  presentationPending = $state.raw<{ on: boolean; sessionId: string | null } | null>(null);

  /** Presentation mode as the next Start uses it: a switch made during the running session, else the snapshot's. */
  presentationOn = $derived.by((): boolean => {
    const snap = session.snapshot;
    const pending = this.presentationPending;
    if (!session.isIdle && pending !== null && pending.sessionId === snap.sessionId) return pending.on;
    return snap.demo;
  });
  /** The running session uses the other profile than the next Start will. */
  presentationDiffers = $derived(!session.isIdle && this.presentationOn !== session.snapshot.demo);
  /** Focus and break length of presentation mode (seconds), for its label. */
  presentationLengths = $derived.by((): { focusSecs: number; breakSecs: number } => {
    const snap = session.snapshot;
    return snap.demo && snap.focusSecs > 0 && snap.breakSecs > 0
      ? { focusSecs: snap.focusSecs, breakSecs: snap.breakSecs }
      : { focusSecs: DEMO_FOCUS_SECS, breakSecs: DEMO_BREAK_SECS };
  });

  /** `Config.developerMode` (off until the config arrives, and with a shell from before the setting). */
  enabled = $derived(config.value?.developerMode === true);
  /** The Developer tab is in the navigation. */
  tabVisible = $derived(this.enabled && !this.cleanScreenshots);

  /** Names of the sample layers shown now (for the Developer page). */
  activeSamples = $derived.by((): string[] => {
    const names = DEV.sampleNames;
    const out: string[] = [];
    if (summary.sample !== null) out.push(names.summary);
    if (work.sample !== null) out.push(names.work);
    if (calendar.sample !== null) out.push(names.calendar);
    if (session.sampleGoal !== null) out.push(names.goal);
    if (session.sampleHold !== null) out.push(names.hold);
    if (this.mascotMood !== null) out.push(names.mood);
    if (rewards.sampling) out.push(DEV_REWARDS.sampleName);
    return out;
  });

  #seq = 0;
  #sequenceTimer: ReturnType<typeof setTimeout> | null = null;

  // ---------------------------------------------------------------------------
  // Status line
  // ---------------------------------------------------------------------------

  ok(text: string): void {
    this.status = { tone: "ok", text };
  }

  fail(what: string, e: unknown): void {
    const err = api.toAppError(e);
    console.warn(`[FocusMode] dev: ${err.code}: ${err.message}`);
    this.status = { tone: "error", text: DEV.failed(what, err.code, err.message) };
  }

  // ---------------------------------------------------------------------------
  // Main-window requests
  // ---------------------------------------------------------------------------

  /** Opens `route` (unless it is open already). */
  async go(route: string): Promise<void> {
    if (page.url.pathname === route) return;
    try {
      await goto(route);
    } catch (e) {
      this.fail(route, e);
    }
  }

  /** Queues `request` for the component on `route` and opens it. */
  async request(route: string, request: DevRequest, what: string): Promise<void> {
    this.#seq++;
    this.queue = [...this.queue, { id: this.#seq, at: performance.now(), request }];
    this.ok(DEV.sent(what));
    await this.go(route);
  }

  /** Removes and returns the requests for `target` (fresh ones only). */
  take(target: DevTarget): QueuedRequest[] {
    const now = performance.now();
    const mine = this.queue.filter((q) => q.request.target === target);
    if (mine.length === 0) return [];
    this.queue = this.queue.filter((q) => q.request.target !== target && now - q.at <= STALE_MS);
    return mine.filter((q) => now - q.at <= STALE_MS);
  }

  // ---------------------------------------------------------------------------
  // Shell: popups, toasts, a shorter phase
  // ---------------------------------------------------------------------------

  async #trigger(what: string, action: DevAction): Promise<boolean> {
    try {
      // The shell checks the stored `developerMode`: a switch flipped a moment ago must be saved first.
      await config.flush();
      await api.devTrigger(action);
      this.ok(DEV.sent(what));
      return true;
    } catch (e) {
      this.fail(what, e);
      return false;
    }
  }

  popup(sample: PopupSample): Promise<boolean> {
    return this.#trigger(sample.label, {
      type: "popup",
      kind: sample.kind,
      leadMinutes: sample.leadMinutes,
      sticky: this.keepPopupOpen,
    });
  }

  /** Every popup in turn, one every `SEQUENCE_STEP_MS`; stops at the first failure. */
  playAllPopups(): void {
    this.stopSequence();
    const total = SEQUENCE.length;
    const step = (index: number) => {
      const sample = SEQUENCE[index];
      if (!sample) {
        this.stopSequence();
        return;
      }
      this.sequence = { step: index + 1, total };
      this.#sequenceTimer = setTimeout(() => step(index + 1), SEQUENCE_STEP_MS);
      void this.popup(sample).then((ok) => {
        if (!ok) this.stopSequence();
      });
    };
    step(0);
  }

  stopSequence(): void {
    if (this.#sequenceTimer !== null) clearTimeout(this.#sequenceTimer);
    this.#sequenceTimer = null;
    this.sequence = null;
  }

  async dismissPopup(): Promise<void> {
    try {
      await api.popupAction("dismiss");
      this.ok(DEV.sent(DEV.dismissPopup));
    } catch (e) {
      this.fail(DEV.dismissPopup, e);
    }
  }

  toast(phase: Exclude<Phase, "idle">, what: string): Promise<boolean> {
    return this.#trigger(what, { type: "toast", phase });
  }

  /** The running phase ends in `secs` (the shell clamps it to the phase length); then opens `route` when given. */
  async setRemaining(secs: number, what: string, route: string | null = null): Promise<void> {
    const ok = await this.#trigger(what, { type: "setRemaining", secs });
    if (ok && route !== null) await this.go(route);
  }

  /**
   * Presentation mode (the demo timing of `--demo`) on or off for this run. Idle: at once (the snapshot's `demo`
   * follows); a running session keeps its lengths and the next Start uses the new timing. Nothing is saved.
   */
  async setPresentation(on: boolean): Promise<void> {
    const running = !session.isIdle;
    const sessionId = session.snapshot.sessionId;
    const { focusSecs, breakSecs } = this.presentationLengths;
    const ok = await this.#trigger(DEV.presentationMode(focusSecs, breakSecs), { type: "presentationMode", on });
    if (!ok) return;
    this.presentationPending = running ? { on, sessionId } : null;
    this.ok(running ? DEV.presentationNextStart(on) : DEV.presentationNow(on));
  }

  /**
   * Factory reset: the shell deletes everything the app keeps (like "Delete everything", plus the settings file) and
   * restarts it. Throws, so the confirmation dialog shows the error and stays open. A failed delete (`privacy.delete`)
   * also gets a global toast: the defaults are stored by then (developer mode off), so this page is closing.
   */
  async factoryReset(): Promise<void> {
    try {
      // The shell checks the stored `developerMode`.
      await config.flush();
      await api.devTrigger({ type: "factoryReset" });
      this.ok(DEV.factoryResetRestarting);
    } catch (e) {
      this.fail(DEV.factoryResetName, e);
      if (api.toAppError(e).code === "privacy.delete") toasts.error(e);
      throw e;
    }
  }

  // ---------------------------------------------------------------------------
  // Mascot (Skupienie page)
  // ---------------------------------------------------------------------------

  mascot(action: MascotDevAction, what: string): Promise<void> {
    return this.request("/", { target: "mascot", action }, what);
  }

  /** Holds a mood on the axolotl (null: back to the session's mood) and opens Skupienie. */
  async holdMood(mood: MascotMood | null, what: string): Promise<void> {
    this.mascotMood = mood;
    this.ok(DEV.sent(what));
    await this.go("/");
  }

  // ---------------------------------------------------------------------------
  // Banners & dialogs (memory only)
  // ---------------------------------------------------------------------------

  showNudge(): Promise<void> {
    return this.request("/", { target: "layout", run: () => nudge.show(sampleNudge()) }, DEV.nudgeBanner);
  }

  showErrorToast(): Promise<void> {
    const text = errorText({ code: "shields.launchers", message: DEV.sampleErrorDetail, field: null });
    return this.request(
      "/",
      { target: "layout", run: () => toasts.push(text, "error", DEV.sampleErrorDetail) },
      DEV.errorToast,
    );
  }

  showInfoToast(): Promise<void> {
    return this.request("/", { target: "layout", run: () => toasts.push(strings.blocklist.reloaded, "info") }, DEV.infoToast);
  }

  showClosePrompt(): Promise<void> {
    return this.request("/", { target: "closePrompt" }, DEV.closePrompt);
  }

  async showHold(choice: Extract<MeetingChoice, "extend" | "pauseAtBreak">, what: string): Promise<void> {
    session.sampleHold = sampleHold(Date.now(), choice);
    this.ok(DEV.sent(what));
    await this.go("/");
  }

  async showGoal(): Promise<void> {
    session.sampleGoal = sampleGoal();
    this.ok(DEV.sent(DEV.goalLine));
    await this.go("/");
  }

  // ---------------------------------------------------------------------------
  // Sample data (memory only)
  // ---------------------------------------------------------------------------

  async showSummarySample(): Promise<void> {
    summary.showSample(sampleSummaries(Date.now()));
    this.ok(DEV.sent(DEV.sampleSummary));
    await this.go("/summary");
  }

  async showWorkSample(): Promise<void> {
    work.showSample(sampleWork(Date.now()));
    this.ok(DEV.sent(DEV.sampleWork));
    await this.go("/");
  }

  async showCalendarSample(): Promise<void> {
    const now = Date.now();
    calendar.showSample(sampleCalendar(now, breakPlan(now)));
    this.ok(DEV.sent(DEV.sampleCalendar));
    await this.go("/");
  }

  /** Drops every sample layer, the held mood, a sample banner and the waiting requests: the real data shows again. */
  clearSamples(report = true): void {
    summary.clearSample();
    work.clearSample();
    calendar.clearSample();
    session.sampleGoal = null;
    session.sampleHold = null;
    this.mascotMood = null;
    rewards.clearSamples();
    this.queue = [];
    if (nudge.current?.matched === sampleNudge().matched) nudge.dismiss();
    if (report) this.ok(DEV.samplesCleared);
  }

  // ---------------------------------------------------------------------------
  // Look & feel (real settings, like the Settings page)
  // ---------------------------------------------------------------------------

  setTheme(theme: Theme, what: string): void {
    if (config.update((c) => (c.theme = theme), true)) this.ok(DEV.sent(what));
  }

  /** The 8-bit theme (pixelated axolotl) and the Skupienie page, in one click. */
  async pixelLook(): Promise<void> {
    this.setTheme("pixel", DEV.pixelLook);
    await this.go("/");
  }

  setLanguage(language: Language, what: string): void {
    if (config.update((c) => (c.language = language), true)) this.ok(DEV.sent(what));
  }

  /** Warm screen preview at `strength` % for a few seconds (the shell restores the colours), or ends it (`null`). */
  async previewTint(strength: number | null, what: string): Promise<void> {
    const err = await tint.preview(strength);
    if (err === null) this.ok(DEV.sent(what));
    else this.fail(what, err);
  }

  /**
   * Plays `sound` for `kind` through the shell's `test_chime`, which plays the stored sound: the chime sound is
   * switched for the test and the user's choice is stored back right after.
   */
  async playChime(sound: ChimeSound, kind: ChimeKind): Promise<void> {
    const current = config.value;
    if (this.chimeBusy || !current) return;
    const what = `${DEV.chimeNames[sound]} · ${kind === "breakStart" ? DEV.chimeBreak : DEV.chimeFocus}`;
    const chosen = current.chimeSound;
    this.chimeBusy = true;
    try {
      if (chosen !== sound) {
        config.update((c) => (c.chimeSound = sound), true);
        if (!(await config.flush())) throw { code: "config.invalid", message: "fix the settings first", field: null };
      }
      await api.testChime(kind);
      this.ok(DEV.sent(what));
    } catch (e) {
      this.fail(what, e);
    } finally {
      if (chosen !== sound) {
        config.update((c) => (c.chimeSound = chosen), true);
        await config.flush();
      }
      this.chimeBusy = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Window & app
  // ---------------------------------------------------------------------------

  resetCloseChoice(): void {
    if (config.update((c) => (c.closeToTray = null), true)) this.ok(DEV.resetCloseDone);
  }

  reloadUi(): void {
    location.reload();
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  /** Ctrl+Shift+D: the Developer tab comes back (after "Clean screenshots") and opens. */
  async showTab(): Promise<void> {
    if (!this.enabled) return;
    this.cleanScreenshots = false;
    await this.go("/dev");
  }

  /** Developer mode was switched off: stop everything, drop the samples, show the tab again next time. */
  reset(): void {
    this.stopSequence();
    this.clearSamples(false);
    this.cleanScreenshots = false;
    this.keepPopupOpen = false;
    this.status = null;
  }
}

export const dev = new DevStore();

/**
 * Runs `handler` for the requests queued for `target`, once the calling component has mounted and the tab slide has
 * ended (`FIRE_DELAY_MS` after the later of the mount and the button press). Call while a component initialises.
 */
export function useDevRequests<T extends DevTarget>(
  target: T,
  handler: (request: Extract<DevRequest, { target: T }>) => void,
): void {
  const mountedAt = performance.now();
  const timers = new Set<ReturnType<typeof setTimeout>>();

  $effect(() => {
    if (!dev.queue.some((q) => q.request.target === target)) return;
    untrack(() => {
      for (const queued of dev.take(target)) {
        const wait = Math.max(mountedAt, queued.at) + FIRE_DELAY_MS - performance.now();
        const timer = setTimeout(
          () => {
            timers.delete(timer);
            handler(queued.request as Extract<DevRequest, { target: T }>);
          },
          Math.max(0, wait),
        );
        timers.add(timer);
      }
    });
  });

  $effect(() => () => {
    for (const timer of timers) clearTimeout(timer);
    timers.clear();
  });
}
