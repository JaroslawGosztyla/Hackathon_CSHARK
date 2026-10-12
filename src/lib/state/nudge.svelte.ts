// "Wracaj do pracy" banner, shown when the shell detects a blocked site in the foreground browser window.
import type { Nudge } from "$lib/types";

const NUDGE_VISIBLE_MS = 20_000;

class NudgeStore {
  current = $state<Nudge | null>(null);
  private timer: ReturnType<typeof setTimeout> | null = null;

  show(nudge: Nudge): void {
    this.current = nudge;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.dismiss(), NUDGE_VISIBLE_MS);
  }

  dismiss(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.current = null;
  }
}

export const nudge = new NudgeStore();
