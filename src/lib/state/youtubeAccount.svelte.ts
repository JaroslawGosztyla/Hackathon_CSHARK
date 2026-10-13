// YouTube sign-in for Premium (ADR-021). The shell owns the sign-in window (Google's own page) and checks only the
// names of the sign-in cookies in the app's browser profile; this store mirrors its `YoutubeAccount`
// (revision-guarded in api.subscribe) and runs the sign-in / sign-out requests.
import * as api from "$lib/api";
import type { YoutubeAccount } from "$lib/types";
import { toasts } from "./toasts.svelte";

/**
 * How long the row may say "Checking…" after the app started before it says the state is unknown: the shell tries
 * the cookies 5 times, 3 s apart, at start (youtube_account.rs), so this is well past its last try.
 */
const STARTUP_CHECK_WAIT_MS = 20_000;
/** After "Check again": the shell tries once (at most 10 s). */
const RECHECK_WAIT_MS = 12_000;

class YoutubeAccountStore {
  /** Replaced as a whole on every update; null until the shell answered. */
  status = $state.raw<YoutubeAccount | null>(null);
  /** A sign-in or sign-out request is running. */
  busy = $state(false);
  /**
   * The shell could not read the sign-in cookies in time (startup checks or "Check again" failed). The row then says
   * the state is unknown, offers "Check again", and YouTube plays as when signed out.
   */
  checkFailed = $state(false);

  /** The shell has checked the cookies since it started. */
  known = $derived(this.status?.known === true);
  /**
   * Signed in (and checked): every YouTube item plays in the YouTube window, the only place where the youtube.com
   * cookies are first-party, so the account (and Premium) applies. Signed out: the player routes as before.
   */
  signedIn = $derived(this.status?.known === true && this.status.signedIn);

  private checkTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    if (typeof window !== "undefined") this.waitForCheck(STARTUP_CHECK_WAIT_MS);
  }

  apply(status: YoutubeAccount): void {
    this.status = status;
    if (status.known) {
      this.clearCheckTimer();
      this.checkFailed = false;
    }
  }

  /** Opens the sign-in window, or brings the open one to the front. */
  signIn(): Promise<void> {
    return this.run(api.youtubeSignIn);
  }

  /** Deletes the Google / YouTube cookies of the app's browser (the cookie choice stays). */
  signOut(): Promise<void> {
    return this.run(api.youtubeSignOut);
  }

  /**
   * "Check again" while the state is unknown: asking the shell for the state starts one more cookie check there; the
   * answer arrives as `youtube://account` (the getter's own reply is not applied, the subscription guards revisions).
   */
  async recheck(): Promise<void> {
    if (this.known) return;
    this.checkFailed = false;
    this.waitForCheck(RECHECK_WAIT_MS);
    try {
      await api.getYoutubeAccount();
    } catch (e) {
      this.clearCheckTimer();
      this.checkFailed = true;
      toasts.error(e);
    }
  }

  /** Says "unknown" after `ms` unless a known state arrived by then. */
  private waitForCheck(ms: number): void {
    this.clearCheckTimer();
    this.checkTimer = setTimeout(() => {
      this.checkTimer = null;
      if (!this.known) this.checkFailed = true;
    }, ms);
  }

  private clearCheckTimer(): void {
    if (this.checkTimer !== null) clearTimeout(this.checkTimer);
    this.checkTimer = null;
  }

  private async run(action: () => Promise<void>): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      await action();
    } catch (e) {
      toasts.error(e);
    } finally {
      this.busy = false;
    }
  }
}

export const youtubeAccount = new YoutubeAccountStore();
