// Microsoft Teams connection (device code login runs in the shell; progress arrives as `teams://status`).
import * as api from "$lib/api";
import type { DeviceCodePrompt, TeamsStatus } from "$lib/types";
import { config } from "./config.svelte";
import { toasts } from "./toasts.svelte";

class TeamsStore {
  status = $state<TeamsStatus | null>(null);
  /** Prompt returned by `teams_connect`, used until the status carries its own `pending`. */
  private localPrompt = $state<DeviceCodePrompt | null>(null);
  connecting = $state(false);

  /** The device code of a sign-in in progress, also while an older sign-in is still connected ("Połącz ponownie"). */
  prompt = $derived.by((): DeviceCodePrompt | null => this.status?.pending ?? this.localPrompt);

  apply(status: TeamsStatus): void {
    this.status = status;
    // Once the shell reports its own login state, the local copy is superseded: a later status without
    // `pending` (expired, cancelled by a config change, finished) must hide the code, not fall back to it.
    if (status.pending || status.connected || status.error) this.localPrompt = null;
  }

  async connect(): Promise<void> {
    if (this.connecting) return;
    this.connecting = true;
    try {
      // The shell reads the client id / tenant from the stored config.
      if (!(await config.flush())) return;
      const prompt = await api.teamsConnect();
      // The `teams://status` event with the same prompt usually arrives first; keep a copy only until it does.
      if (!this.status?.pending) this.localPrompt = prompt;
    } catch (e) {
      toasts.error(e);
    } finally {
      this.connecting = false;
    }
  }

  async disconnect(): Promise<void> {
    try {
      await api.teamsDisconnect();
      this.localPrompt = null;
    } catch (e) {
      toasts.error(e);
    }
  }
}

export const teams = new TeamsStore();
