// Last session summary + history (newest first).
import * as api from "$lib/api";
import type { SessionSummary } from "$lib/types";
import { toasts } from "./toasts.svelte";

export const HISTORY_LIMIT = 20;

class SummaryStore {
  last = $state<SessionSummary | null>(null);
  history = $state<SessionSummary[]>([]);
  loaded = $state(false);
  /** Session picked in the history list; null = show the last one. */
  selectedId = $state<string | null>(null);

  selected = $derived.by((): SessionSummary | null => {
    if (this.selectedId === null) return this.last;
    return this.history.find((s) => s.sessionId === this.selectedId) ?? this.last;
  });

  async load(): Promise<void> {
    try {
      const [last, history] = await Promise.all([api.getLastSummary(), api.listHistory(HISTORY_LIMIT)]);
      // A session that ended while the list was loading is already in `history` via `add()`; keep it,
      // and never show one session id twice (keyed each blocks would throw on duplicates).
      const seen = new Set<string>();
      const merged = [...$state.snapshot(this.history), ...history].filter((s) => {
        if (seen.has(s.sessionId)) return false;
        seen.add(s.sessionId);
        return true;
      });
      merged.sort((a, b) => b.endedAtMs - a.endedAtMs);
      this.history = merged.slice(0, HISTORY_LIMIT);
      this.last = this.last ?? last ?? this.history[0] ?? null;
      this.loaded = true;
    } catch (e) {
      const err = api.toAppError(e);
      if (err.code !== api.ERR_NO_SHELL) toasts.error(err);
      this.loaded = true;
    }
  }

  /** A session just ended (`session://summary`). */
  add(summary: SessionSummary): void {
    this.last = summary;
    this.selectedId = null;
    this.history = [summary, ...this.history.filter((s) => s.sessionId !== summary.sessionId)].slice(0, HISTORY_LIMIT);
    this.loaded = true;
  }

  select(sessionId: string | null): void {
    this.selectedId = sessionId === this.last?.sessionId ? null : sessionId;
  }
}

export const summary = new SummaryStore();
