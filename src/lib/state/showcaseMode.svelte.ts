// Presentation showcase flags (Developer page "Presentation mode", `--presentation`; ADR-019). A leaf module: the stores
// and components that look different on stage read it, `presentation.svelte.ts` sets it. Memory only, never saved.
class ShowcaseMode {
  /**
   * The showcase is on: the sample layers show (workday, history, weekly stats, "Co dalej?", goal), the beta features
   * count as on for the rewards and small steps, and no "demo" badge shows (it must look like the real app on stage).
   */
  active = $state(false);
  /** This run was started with `--presentation`: developer mode is on for this run only (the setting is not changed). */
  launch = $state(false);
  /**
   * Ids of the demo sessions that ran while the showcase was on (not reactive). Like the shell
   * (`RewardsHandle::showcase_session`), their summaries are never stored or remembered, also when such a session ends
   * after the showcase was switched off.
   */
  readonly sessions = new Set<string>();

  /** A finished session's summary is only shown, never added to the real layer (see `sessions`). */
  shownOnly(sessionId: string, demo: boolean): boolean {
    return demo && (this.active || this.sessions.has(sessionId));
  }
}

export const showcaseMode = new ShowcaseMode();
