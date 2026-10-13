// Last session summary + history (newest first).
import * as api from "$lib/api";
import type { SessionSummary } from "$lib/types";
import { config } from "./config.svelte";
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

  /** Bumped by `clear()`, so a load that started before it cannot bring the deleted sessions back. */
  private generation = 0;

  async load(): Promise<void> {
    const generation = this.generation;
    try {
      const [last, history] = await Promise.all([api.getLastSummary(), api.listHistory(HISTORY_LIMIT)]);
      if (generation !== this.generation) return;
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

  /** A session just ended (`session://summary`). With `keepHistory` off the shell did not store it: no history row. */
  add(summary: SessionSummary): void {
    this.last = summary;
    this.selectedId = null;
    if (config.value?.keepHistory !== false) {
      this.history = [summary, ...this.history.filter((s) => s.sessionId !== summary.sessionId)].slice(0, HISTORY_LIMIT);
    }
    this.loaded = true;
  }

  select(sessionId: string | null): void {
    this.selectedId = sessionId === this.last?.sessionId ? null : sessionId;
  }

  /** Deletes the whole history in the shell (the UI confirmed first), then empties the views. Throws on failure. */
  async clear(): Promise<void> {
    await api.clearHistory();
    this.generation++;
    this.last = null;
    this.history = [];
    this.selectedId = null;
    this.loaded = true;
  }
}

export const summary = new SummaryStore();
