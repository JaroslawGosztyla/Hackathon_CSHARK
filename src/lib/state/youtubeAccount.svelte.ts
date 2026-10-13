// YouTube sign-in for Premium (ADR-021). The shell owns the sign-in window (Google's own page) and checks only the
// names of the sign-in cookies in the app's browser profile; this store mirrors its `YoutubeAccount`
// (revision-guarded in api.subscribe) and runs the sign-in / sign-out requests.
import * as api from "$lib/api";
import type { YoutubeAccount } from "$lib/types";
import { toasts } from "./toasts.svelte";

class YoutubeAccountStore {
  /** Replaced as a whole on every update; null until the shell answered. */
  status = $state.raw<YoutubeAccount | null>(null);
  /** A sign-in or sign-out request is running. */
  busy = $state(false);

  /** The shell has checked the cookies since it started. */
  known = $derived(this.status?.known === true);
  /**
   * Signed in (and checked): every YouTube item plays in the YouTube window, the only place where the youtube.com
   * cookies are first-party, so the account (and Premium) applies. Signed out: the player routes as before.
   */
  signedIn = $derived(this.status?.known === true && this.status.signedIn);

  apply(status: YoutubeAccount): void {
    this.status = status;
  }

  /** Opens the sign-in window, or brings the open one to the front. */
  signIn(): Promise<void> {
    return this.run(api.youtubeSignIn);
  }

  /** Deletes the Google / YouTube cookies of the app's browser (the cookie choice stays). */
  signOut(): Promise<void> {
    return this.run(api.youtubeSignOut);
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
