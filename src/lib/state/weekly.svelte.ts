// Weekly stats (ADR-023): the week shown in Podsumowanie → "Ten tydzień". Real numbers come from `get_weekly_stats`
// through `api.watchWeeklyStats` (listen to `history://changed`, then the getter, never an older revision), only while
// the section is on screen. The app labels live in `Config.appCategories` (lowercase exe → label), set by the user
// with the chips. In developer mode a sample (ADR-019) replaces the numbers: built in memory per week, its labels are
// a memory-only copy that never reaches the config.
import * as api from "$lib/api";
import type { AppCategory, AppError, WeeklyStats } from "$lib/types";
import { addDays, DAYS_PER_WEEK, mondayOf, todayDate } from "$lib/week";
import { config } from "./config.svelte";
import { SAMPLE_WEEK_LABELS, sampleWeeklyStats } from "./samples";

/** The key of an app in `Config.appCategories` (focus-core `normalize_app_categories`: trimmed, lowercase). */
export function labelKey(process: string): string {
  return process.trim().toLowerCase();
}

class WeeklyStore {
  /** Monday (`YYYY-MM-DD`) of the week the user picked; null = the current week (follows the date). */
  picked = $state<string | null>(null);
  /** The shell's latest answer (possibly for the previously picked week while the next one loads). */
  real = $state<WeeklyStats | null>(null);
  /** The latest request failed (shown with a retry; the last numbers stay). */
  error = $state<AppError | null>(null);
  /** Not running in the shell (a plain browser): there is no history to read. */
  unavailable = $state(false);
  /** Developer-mode sample (ADR-019): its labels, in memory only; null = real data. */
  sampleLabels = $state<Record<string, AppCategory> | null>(null);
  /** Bumped by `requestedChanged` so `stats` rebuilds a sample for the newly picked week. */
  private sampleTick = $state(0);

  private watch: api.WeeklyStatsWatch | null = null;
  private watchToken = 0;
  private users = 0;

  get sampling(): boolean {
    return this.sampleLabels !== null;
  }

  /** Monday of the week asked for. */
  get requested(): string {
    return this.picked ?? mondayOf(todayDate());
  }

  /** What the section shows: the sample, or the shell's answer (the previous week's while the picked one loads). */
  stats = $derived.by((): WeeklyStats | null => {
    void this.sampleTick;
    const labels = this.sampleLabels;
    if (labels !== null) return sampleWeeklyStats(this.picked ?? todayDate(), Date.now(), labels);
    return this.real;
  });

  /** The numbers on screen are not (yet) the requested week's. */
  pending = $derived.by((): boolean => {
    if (this.sampleLabels !== null) return false;
    return this.real === null || this.real.weekStart !== mondayOf(this.picked ?? todayDate());
  });

  /** The section is on screen: follow the history while it is (counted, so two mounts share one watch). */
  follow(): () => void {
    this.users++;
    if (this.users === 1) void this.startWatch();
    else this.watch?.refresh();
    let stopped = false;
    return () => {
      if (stopped) return;
      stopped = true;
      this.users = Math.max(0, this.users - 1);
      if (this.users === 0) this.stopWatch();
    };
  }

  /** Shows the week of `date` (any day of it); the current week goes back to following the date. */
  show(date: string): void {
    const monday = mondayOf(date);
    this.picked = monday === mondayOf(todayDate()) ? null : monday;
    this.requestedChanged();
  }

  previous(): void {
    this.show(addDays(this.requested, -DAYS_PER_WEEK));
  }

  next(): void {
    this.show(addDays(this.requested, DAYS_PER_WEEK));
  }

  thisWeek(): void {
    this.show(todayDate());
  }

  /** Reads the week again (the retry after an error). */
  retry(): void {
    this.requestedChanged();
  }

  /** The user's label of an app (from the config, or the sample's copy). */
  labelOf(process: string): AppCategory | null {
    const labels = this.sampleLabels ?? config.value?.appCategories ?? {};
    return labels[labelKey(process)] ?? null;
  }

  /**
   * A chip was clicked: sets the label of the app, or clears it when it already had this one. Saved right away in
   * `config.json` (the shell then announces `history://changed` and the totals follow); a sample keeps it in memory.
   */
  toggleLabel(process: string, category: AppCategory): void {
    const key = labelKey(process);
    if (key.length === 0) return;
    const toggle = (labels: Record<string, AppCategory>): Record<string, AppCategory> => {
      const next = { ...labels };
      if (next[key] === category) delete next[key];
      else next[key] = category;
      return next;
    };
    if (this.sampleLabels !== null) {
      this.sampleLabels = toggle(this.sampleLabels);
      return;
    }
    config.update((c) => (c.appCategories = toggle(c.appCategories)), true);
  }

  /** Shows the developer-mode sample (ADR-019), from the current week. */
  showSample(): void {
    this.sampleLabels = { ...SAMPLE_WEEK_LABELS };
    this.picked = null;
    this.error = null;
  }

  /** Back to the real data (of the current week). */
  clearSample(): void {
    if (this.sampleLabels === null) return;
    this.sampleLabels = null;
    this.picked = null;
    this.requestedChanged();
  }

  private requestedChanged(): void {
    this.error = null;
    this.sampleTick++;
    if (this.sampleLabels === null) this.watch?.refresh();
  }

  private async startWatch(): Promise<void> {
    const token = ++this.watchToken;
    try {
      const watch = await api.watchWeeklyStats(
        () => this.requested,
        (stats) => {
          this.real = stats;
          this.error = null;
        },
        (error) => this.fail(error),
      );
      if (token !== this.watchToken) watch.stop();
      else this.watch = watch;
    } catch (e) {
      if (token === this.watchToken) this.fail(api.toAppError(e));
    }
  }

  private stopWatch(): void {
    this.watchToken++;
    this.watch?.stop();
    this.watch = null;
  }

  private fail(error: AppError): void {
    // Outside the shell (a plain browser) there is nothing to read: the section shows its empty state.
    if (error.code === api.ERR_NO_SHELL) {
      this.unavailable = true;
      return;
    }
    this.error = error;
  }
}

export const weekly = new WeeklyStore();
