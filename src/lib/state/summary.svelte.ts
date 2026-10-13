// Last session summary + history (newest first). Two layers: `real` (from the shell) and `sample` (developer mode,
// ADR-019: memory only, wins until cleared); the views read `last` / `history`, which pick the layer.
import * as api from "$lib/api";
import type { SessionSummary } from "$lib/types";
import { config } from "./config.svelte";
import { isSampleId } from "./samples";
import { toasts } from "./toasts.svelte";

export const HISTORY_LIMIT = 20;

interface SummaryLayer {
  last: SessionSummary | null;
  history: SessionSummary[];
}

class SummaryStore {
  /** From the shell (`get_last_summary`, `list_history`, `session://summary`). */
  real = $state<SummaryLayer>({ last: null, history: [] });
  /** Developer-mode sample (ADR-019): never saved, never sent; shown instead of `real` until `clearSample`. */
  sample = $state<SummaryLayer | null>(null);
  /** The real history finished loading (or failed). */
  realLoaded = $state(false);
  /** Session picked in the history list; null = show the last one. */
  selectedId = $state<string | null>(null);

  get last(): SessionSummary | null {
    return this.sample ? this.sample.last : this.real.last;
  }

  get history(): SessionSummary[] {
    return this.sample ? this.sample.history : this.real.history;
  }

  get loaded(): boolean {
    return this.sample !== null || this.realLoaded;
  }

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
      const merged = [...$state.snapshot(this.real.history), ...history].filter((s) => {
        if (seen.has(s.sessionId)) return false;
        seen.add(s.sessionId);
        return true;
      });
      merged.sort((a, b) => b.endedAtMs - a.endedAtMs);
      this.real.history = merged.slice(0, HISTORY_LIMIT);
      this.real.last = this.real.last ?? last ?? this.real.history[0] ?? null;
      this.realLoaded = true;
    } catch (e) {
      const err = api.toAppError(e);
      if (err.code !== api.ERR_NO_SHELL) toasts.error(err);
      this.realLoaded = true;
    }
  }

  /**
   * A session just ended (`session://summary`). With `keepHistory` off the shell did not store it: no history row.
   * A developer-mode sample goes away: the app opens Podsumowanie for the session that just ended, not the sample.
   */
  add(summary: SessionSummary): void {
    this.real.last = summary;
    this.sample = null;
    this.selectedId = null;
    if (config.value?.keepHistory !== false) {
      this.real.history = [summary, ...this.real.history.filter((s) => s.sessionId !== summary.sessionId)].slice(
        0,
        HISTORY_LIMIT,
      );
    }
    this.realLoaded = true;
  }

  select(sessionId: string | null): void {
    this.selectedId = sessionId === this.last?.sessionId ? null : sessionId;
  }

  /** Shows a developer-mode sample instead of the real data (ADR-019). */
  showSample(sample: SummaryLayer): void {
    this.sample = sample;
    this.selectedId = null;
  }

  /** Back to the real data. */
  clearSample(): void {
    if (this.sample === null) return;
    this.sample = null;
    this.selectedId = null;
  }

  /** A `mark_goal_done` call is in flight for this session. */
  goalBusyId = $state<string | null>(null);

  /**
   * "Zrobione?" for the goal of a finished session. Stored in the history by the shell; a session that was not saved
   * (history off) or a developer-mode sample is only marked here. Resolves true when the answer is shown.
   */
  async markGoalDone(sessionId: string, done: boolean): Promise<boolean> {
    if (this.goalBusyId !== null) return false;
    if (isSampleId(sessionId)) {
      for (const s of this.sample?.history ?? []) if (s.sessionId === sessionId) s.goalDone = done;
      const last = this.sample?.last;
      if (last?.sessionId === sessionId) last.goalDone = done;
      return true;
    }
    const stored = config.value?.keepHistory !== false || this.real.history.some((s) => s.sessionId === sessionId);
    this.goalBusyId = sessionId;
    try {
      if (stored) await api.markGoalDone(sessionId, done);
    } catch (e) {
      toasts.error(e);
      return false;
    } finally {
      this.goalBusyId = null;
    }
    if (this.real.last?.sessionId === sessionId) this.real.last.goalDone = done;
    for (const s of this.real.history) if (s.sessionId === sessionId) s.goalDone = done;
    return true;
  }

  /** Deletes the whole history in the shell (the UI confirmed first), then empties the views. Throws on failure. */
  async clear(): Promise<void> {
    await api.clearHistory();
    this.forget();
  }

  /** The shell has deleted the history (here or from the privacy panel, ADR-020): empties the views. */
  forget(): void {
    this.generation++;
    this.real = { last: null, history: [] };
    this.selectedId = null;
    this.realLoaded = true;
  }
}

export const summary = new SummaryStore();
