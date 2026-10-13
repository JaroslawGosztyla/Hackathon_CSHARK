// First-run wizard (ADR-023, part 4): shown once on a fresh install, i.e. while `Config.onboardingDone` is false (only
// the config the shell creates when `config.json` does not exist has that: `Config::first_run()`; a config without the
// field counts as done, so existing users never see it). Finishing or skipping stores `onboardingDone: true` through
// the config store, like any setting. The Developer page can show it again (`preview`): then only finishing stores the
// flag, skipping leaves it as it is. The wizard changes settings through the existing stores and starts nothing.
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { tick } from "svelte";
import type { Config } from "$lib/types";
import { config } from "./config.svelte";

/** The steps, in order (their texts: `strings.onboarding.steps`). */
export const ONBOARDING_STEPS = ["welcome", "music", "shields", "extras"] as const;
export type OnboardingStep = (typeof ONBOARDING_STEPS)[number];

/** The Skupienie page (where finishing leaves the user, with Start focused). */
const FOCUS_ROUTE = "/";
/** Frames to wait for the Skupienie page to mount before giving up on focusing Start. */
const FOCUS_START_FRAMES = 30;

/**
 * Focuses the first enabled session button of the Skupienie page (Start while idle; Pause / Resume when a session
 * already runs), once the page has mounted (`data-session-controls` on its button group).
 */
function focusSessionControls(framesLeft = FOCUS_START_FRAMES): void {
  const button = document.querySelector<HTMLButtonElement>("[data-session-controls] button:not(:disabled)");
  if (button) {
    button.focus();
    return;
  }
  if (framesLeft > 0) requestAnimationFrame(() => focusSessionControls(framesLeft - 1));
}

class OnboardingStore {
  /** The wizard covers the main window. */
  open = $state(false);
  /** Shown from the Developer page: skipping (or leaving through a link) keeps `onboardingDone` as it is. */
  preview = $state(false);
  /** Index into `ONBOARDING_STEPS`. */
  index = $state(0);

  step = $derived<OnboardingStep>(ONBOARDING_STEPS[this.index] ?? "welcome");
  first = $derived(this.index === 0);
  last = $derived(this.index === ONBOARDING_STEPS.length - 1);

  /** The loaded config was looked at once in this window: a later reload never opens the wizard by itself. */
  #decided = false;

  /** Opens the wizard for a fresh install: the first loaded config with `onboardingDone: false`, once per window. */
  follow(cfg: Config | null): void {
    if (this.#decided || cfg === null) return;
    this.#decided = true;
    // A config from an older shell build has no field: an existing user.
    if ((cfg.onboardingDone as boolean | undefined) === false) this.#show(false);
  }

  /** Developer page: shows the wizard from its first step without changing `onboardingDone`. */
  showPreview(): void {
    this.#decided = true;
    this.#show(true);
  }

  next(): void {
    if (!this.last) this.index++;
  }

  back(): void {
    if (!this.first) this.index--;
  }

  /** A step of the progress list (any step, also ahead: every step is optional). */
  goTo(index: number): void {
    if (Number.isInteger(index) && index >= 0 && index < ONBOARDING_STEPS.length) this.index = index;
  }

  /** "Start my first focus": stores the flag (also in a preview), closes and focuses Start on Skupienie. */
  async finish(): Promise<void> {
    if (!this.open) return;
    this.#markDone();
    this.open = false;
    await this.#goTo(FOCUS_ROUTE);
    focusSessionControls();
  }

  /** Skip (with confirmation): stores the flag unless previewing, closes; Start gets the focus when on Skupienie. */
  async skip(): Promise<void> {
    if (!this.open) return;
    if (!this.preview) this.#markDone();
    this.open = false;
    await tick();
    if (page.url.pathname === FOCUS_ROUTE) focusSessionControls();
  }

  /** A link inside the wizard ("later in Settings"): leaves like a skip, then opens the link. */
  async leave(href: string): Promise<void> {
    if (!this.open) return;
    if (!this.preview) this.#markDone();
    this.open = false;
    await this.#goTo(href);
  }

  #show(preview: boolean): void {
    this.preview = preview;
    this.index = 0;
    this.open = true;
  }

  #markDone(): void {
    config.update((c) => {
      c.onboardingDone = true;
    }, true);
  }

  async #goTo(href: string): Promise<void> {
    const current = `${page.url.pathname}${page.url.hash}`;
    try {
      if (current !== href) await goto(href);
      else await tick();
    } catch (e) {
      console.warn(`[FocusMode] onboarding: cannot open ${href}: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
}

export const onboarding = new OnboardingStore();
