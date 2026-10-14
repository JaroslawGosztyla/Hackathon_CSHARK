// Presentation mode (Developer page switch, `--presentation`; ADR-019 + beta ADR-022): demo timings and the showcase in
// one switch, for a short live demo that must look like the real app on stage.
//
// On: the shell gets `dev_trigger { presentationMode, on: true }` (the demo profile from the next Start, a seeded preview
// ledger for the rewards and the beta rules, all in memory), then the UI shows its showcase layers (`./showcase`: the
// workday and history in Podsumowanie, the weekly stats, "Co dalej?", the session goal), counts the beta as on
// (`showcaseMode.active`), plans two small steps for the live session when none are planned, hides the Developer tab
// ("Clean screenshots"; Ctrl+Shift+D brings it back) and opens Skupienie. Off: every sample layer goes, the empty plan
// and the "Clean screenshots" state come back, and the shell drops its showcase. Nothing is saved or sent.
//
// A showcase the shell runs without this window asking (`--presentation`, another launch, a reload) arrives as the
// launch hint the shell evaluates in this page (`showcase.rs`): `window.__FOCUSMODE_LAUNCH__` + `focusmode:launch`.
import { untrack } from "svelte";
import * as api from "$lib/api";
import { DEV_PRESENTATION } from "$lib/i18n/dev";
import { language, type UiLanguage } from "$lib/i18n/index.svelte";
import { showcaseTexts } from "$lib/i18n/showcase";
import type { StepDraft } from "$lib/types";
import { addDays, localMidnightMs, mondayOf, todayDate } from "$lib/week";
import { config } from "./config.svelte";
import { dev } from "./dev.svelte";
import { onboarding } from "./onboarding.svelte";
import { sampleGoal, sampleWork } from "./samples";
import { session } from "./session.svelte";
import { buildShowcase, SHOWCASE_LABELS, showcaseDemoSteps, showcaseSeed, showcaseWeeklyStats } from "./showcase";
import { showcaseMode } from "./showcaseMode.svelte";
import { summary } from "./summary.svelte";
import { weekly } from "./weekly.svelte";
import { work } from "./work.svelte";

/** What the shell tells this page (`showcase.rs` `hint_script`): two booleans, nothing else. */
interface LaunchHint {
  presentation: boolean;
  developer: boolean;
}

declare global {
  interface Window {
    __FOCUSMODE_LAUNCH__?: LaunchHint;
  }
}

const LAUNCH_EVENT = "focusmode:launch";

function readHint(): LaunchHint | null {
  const hint = typeof window === "undefined" ? undefined : window.__FOCUSMODE_LAUNCH__;
  if (!hint || typeof hint !== "object") return null;
  return { presentation: hint.presentation === true, developer: hint.developer === true };
}

class PresentationStore {
  /** A switch is being applied (the Developer page disables the switch meanwhile). */
  busy = $state(false);
  /**
   * The switch was flipped while a session ran (the shell changes the timings at the next Start): the wanted state and
   * that session. Ignored once that session is over.
   */
  timingPending = $state.raw<{ on: boolean; sessionId: string | null } | null>(null);
  /** The running session uses other timings than the switch says (until the next Start). */
  timingDiffers = $derived.by((): boolean => {
    const pending = this.timingPending;
    const snap = session.snapshot;
    return !session.isIdle && pending !== null && pending.sessionId === snap.sessionId && pending.on !== snap.demo;
  });

  /** The showcase's time and seed: the numbers stay the same while it is on (also when the language changes). */
  #now = 0;
  #seed = 0;
  #language: UiLanguage | null = null;
  /** "Clean screenshots" before the showcase (restored when it ends). */
  #cleanBefore = false;
  /** The demo steps the showcase planned into an empty plan (cleared again when it ends); null = none. */
  #plannedSteps: readonly string[] | null = null;

  get on(): boolean {
    return showcaseMode.active;
  }

  /** The Developer page switch: asks the shell first, then shows or drops the showcase. Resolves false on a refusal. */
  async set(on: boolean): Promise<boolean> {
    if (this.busy) return false;
    this.busy = true;
    const running = !session.isIdle;
    const sessionId = session.snapshot.sessionId;
    try {
      // The shell checks the stored `developerMode`: a switch flipped a moment ago must be saved first.
      await config.flush();
      if (on) {
        await api.devTrigger({ type: "presentationMode", on: true });
        await this.#activate();
      } else {
        // The plan goes back while the shell still allows small steps (the beta may be off without the showcase).
        await this.#unplanSteps();
        await api.devTrigger({ type: "presentationMode", on: false });
        this.#deactivate();
      }
      this.timingPending = running ? { on, sessionId } : null;
      dev.ok(running ? DEV_PRESENTATION.nextStart(on) : DEV_PRESENTATION.now(on));
      return true;
    } catch (e) {
      dev.fail(DEV_PRESENTATION.name, e);
      return false;
    } finally {
      this.busy = false;
    }
  }

  /**
   * Follows the shell's launch hint and the language; ends the showcase when developer mode goes (the shell ends its
   * part in `store_config`). Call once from the app state; returns the cleanup.
   */
  listen(): () => void {
    const onHint = () => {
      const hint = readHint();
      if (hint) void this.#applyHint(hint);
    };
    onHint();
    window.addEventListener(LAUNCH_EVENT, onHint);
    const stopEffects = $effect.root(() => {
      // A new UI language: the showcase texts follow (same seed and time: the same numbers).
      $effect(() => {
        const lang = language();
        if (!showcaseMode.active || lang === this.#language) return;
        untrack(() => this.#applyLayers());
      });
      // A demo session on stage stays the showcase's (its summary is never added to the real layer, `summary.add`).
      $effect(() => {
        const snap = session.snapshot;
        if (showcaseMode.active && snap.demo && snap.sessionId !== null) showcaseMode.sessions.add(snap.sessionId);
      });
      // Developer mode switched off (Settings): the showcase goes with it.
      $effect(() => {
        if (!config.ready || dev.enabled || !showcaseMode.active) return;
        untrack(() => {
          void this.#unplanSteps();
          this.#deactivate();
        });
      });
    });
    return () => {
      window.removeEventListener(LAUNCH_EVENT, onHint);
      stopEffects();
    };
  }

  async #applyHint(hint: LaunchHint): Promise<void> {
    if (hint.developer) showcaseMode.launch = true;
    if (hint.presentation && !showcaseMode.active) await this.#activate();
  }

  /** Shows the showcase (the shell's part is on already). */
  async #activate(): Promise<void> {
    if (showcaseMode.active) return;
    this.#now = Date.now();
    this.#seed = showcaseSeed(this.#now);
    this.#cleanBefore = dev.cleanScreenshots;
    showcaseMode.active = true;
    // On stage the app shows its everyday state: a first-run wizard that just opened closes (nothing is stored).
    if (onboarding.open && !onboarding.preview) onboarding.open = false;
    this.#applyLayers();
    dev.cleanScreenshots = true;
    await this.#planSteps();
    await dev.go("/");
  }

  /** Drops every sample layer and the beta override; "Clean screenshots" goes back to what it was. */
  #deactivate(): void {
    if (!showcaseMode.active) return;
    showcaseMode.active = false;
    this.#language = null;
    dev.clearSamples(false);
    dev.cleanScreenshots = this.#cleanBefore;
  }

  /** The sample layers of the showcase, built from its seed in the current language. */
  #applyLayers(): void {
    const lang = language();
    this.#language = lang;
    const data = buildShowcase(this.#now, this.#seed, showcaseTexts(lang));
    summary.showSample({ last: data.last, history: data.history });
    weekly.showSample(SHOWCASE_LABELS, (date, now, labels) => showcaseWeeklyStats(date, now, labels, data.all));
    // Early in a week (a Monday morning) "this week" is still empty: open the last full one instead.
    const monday = mondayOf(todayDate(this.#now));
    if (!data.all.some((s) => s.startedAtMs >= localMidnightMs(monday))) weekly.show(addDays(monday, -7));
    work.showSample(sampleWork(this.#now));
    session.sampleGoal = sampleGoal();
  }

  /** Idle with no plan: the two demo steps (the Start then pays the plan bonus, a tick shows the step float). */
  async #planSteps(): Promise<void> {
    this.#plannedSteps = null;
    // The shell's plan as it is now (after a reload the snapshot may not have arrived yet).
    await session.reload();
    if (!session.isIdle || (session.snapshot.steps ?? []).length > 0) return;
    const steps = showcaseDemoSteps(showcaseTexts(this.#language ?? language()));
    if (await this.#setSteps(steps)) this.#plannedSteps = steps.map((s) => s.text);
  }

  /** Idle: the plan the showcase made goes again (a plan the user changed meanwhile is theirs and stays). */
  async #unplanSteps(): Promise<void> {
    if (this.#plannedSteps === null) return;
    const demo = new Set(this.#plannedSteps);
    this.#plannedSteps = null;
    const steps = session.snapshot.steps ?? [];
    if (!session.isIdle || steps.length === 0 || !steps.every((s) => demo.has(s.text))) return;
    await this.#setSteps([]);
  }

  async #setSteps(steps: StepDraft[]): Promise<boolean> {
    try {
      await api.setSessionSteps(steps);
      await session.reload();
      return true;
    } catch (e) {
      const err = api.toAppError(e);
      if (err.code !== api.ERR_NO_SHELL) console.warn(`[FocusMode] presentation: steps: ${err.code}`);
      return false;
    }
  }
}

export const presentation = new PresentationStore();
