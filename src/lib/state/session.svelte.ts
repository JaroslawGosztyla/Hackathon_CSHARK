// Session store: the latest `SessionSnapshot` (revision-guarded in api.subscribe) plus user actions.
import * as api from "$lib/api";
import { unlockAudio } from "$lib/player/noise";
import { strings } from "$lib/strings";
import type { PhaseChange, SessionSnapshot } from "$lib/types";
import { config } from "./config.svelte";
import { toasts } from "./toasts.svelte";

const IDLE_SNAPSHOT: SessionSnapshot = {
  revision: 0,
  sessionId: null,
  phase: "idle",
  paused: false,
  cycle: 0,
  remainingSecs: 0,
  phaseTotalSecs: 0,
  focusSecs: 0,
  breakSecs: 0,
  focusedSecs: 0,
  demo: false,
  musicMuted: false,
  meetingHold: null,
  goal: null,
};

class SessionStore {
  /** Replaced as a whole on every update, so no deep proxy is needed. */
  snapshot = $state.raw<SessionSnapshot>(IDLE_SNAPSHOT);
  /** At least one snapshot arrived from the shell. */
  connected = $state(false);
  busy = $state(false);
  /** Text for the polite aria-live region (phase changes). */
  announcement = $state("");

  phase = $derived(this.snapshot.phase);
  paused = $derived(this.snapshot.paused);
  isIdle = $derived(this.snapshot.phase === "idle");
  /** Focus or Break and not paused. */
  running = $derived(this.snapshot.phase !== "idle" && !this.snapshot.paused);

  /** Takes a snapshot unless it is older than the one shown (it may come from the event or from `reload`). */
  apply(snapshot: SessionSnapshot): void {
    if (this.connected && !api.isNewerRevision(snapshot.revision, this.snapshot.revision)) return;
    this.snapshot = snapshot;
    this.connected = true;
  }

  /** Reads the snapshot now (e.g. right after a command that changed it). Failures stay quiet: the next event fixes it. */
  async reload(): Promise<void> {
    try {
      this.apply(await api.getSnapshot());
    } catch {
      // `session://snapshot` brings it anyway.
    }
  }

  announce(change: PhaseChange): void {
    if (change.to === "focus") this.announcement = strings.phaseAnnouncement.focusStarted(change.cycle);
    else if (change.to === "break") this.announcement = strings.phaseAnnouncement.breakStarted;
    else this.announcement = strings.phaseAnnouncement.stopped;
  }

  /** Must be called from a click handler: unlocks WebAudio inside the user gesture. */
  start(): Promise<void> {
    return this.run(async () => {
      unlockAudio();
      const ok = await config.flush();
      if (!ok) {
        toasts.push(strings.errors.fixConfigFirst, "error");
        return;
      }
      await api.startSession();
    });
  }

  pause(): Promise<void> {
    return this.run(() => api.pauseSession());
  }

  resume(): Promise<void> {
    return this.run(async () => {
      unlockAudio();
      await api.resumeSession();
    });
  }

  togglePause(): Promise<void> {
    return this.snapshot.paused ? this.resume() : this.pause();
  }

  skip(): Promise<void> {
    return this.run(() => api.skipPhase());
  }

  /** Stop never waits for other actions: it must work in every state. */
  async stop(): Promise<void> {
    try {
      await api.stopSession();
    } catch (e) {
      toasts.error(e);
    }
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

export const session = new SessionStore();
