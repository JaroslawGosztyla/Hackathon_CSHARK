<script lang="ts">
  // Podsumowanie → "Ten tydzień" (ADR-023): one ISO week (Monday first, local time) from the stored history, picked
  // with ‹ ›: focus per day, totals, the hour heatmaps (this week / the last weeks added up) and the week's apps with
  // the user's labels. Numbers come from `weekly` (the shell, or a developer-mode sample); nothing is judged: an empty
  // week only says what will show up here.
  import { onMount } from "svelte";
  import { formatDayRange, formatDuration, formatShortDate, formatWeekday } from "$lib/format";
  import { weekly } from "$lib/state/weekly.svelte";
  import { errorText, strings } from "$lib/strings";
  import type { Weekday } from "$lib/types";
  import { addDays, dayOfDate, DAYS_PER_WEEK, localMidnightMs, todayDate, weekdayIndex } from "$lib/week";
  import WeekApps from "./WeekApps.svelte";
  import WeekHeatmap from "./WeekHeatmap.svelte";

  const WEEKDAYS: readonly Weekday[] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  const uid = $props.id();
  const s = strings.week;

  onMount(() => weekly.follow());

  type HeatMode = "week" | "recent";
  let heatMode = $state<HeatMode>("week");

  const stats = $derived(weekly.stats);
  const today = $derived(todayDate());
  const current = $derived(stats?.currentWeek ?? weekly.picked === null);
  const range = $derived.by(() => {
    if (!stats) return "";
    return formatDayRange(localMidnightMs(stats.weekStart), localMidnightMs(addDays(stats.weekStart, DAYS_PER_WEEK - 1)));
  });
  /** The first day of the "last weeks" heatmap (its weeks end with the shown one). */
  const recentRange = $derived.by(() => {
    if (!stats) return "";
    const first = addDays(stats.weekStart, -(stats.recentWeeks - 1) * DAYS_PER_WEEK);
    return formatDayRange(localMidnightMs(first), localMidnightMs(addDays(stats.weekStart, DAYS_PER_WEEK - 1)));
  });
  const canPrevious = $derived(
    stats !== null && stats.firstWeekStart !== null && stats.weekStart > stats.firstWeekStart,
  );
  const canNext = $derived(stats !== null && !stats.currentWeek);
  const weekEmpty = $derived(stats !== null && stats.focusSecs === 0 && stats.sessions === 0 && stats.topApps.length === 0);
  const recentAny = $derived(stats?.recentHeatmap.some((row) => row.some((secs) => secs > 0)) ?? false);
  const maxDay = $derived(Math.max(1, ...(stats?.days.map((d) => d.focusSecs) ?? [])));
  /** Row of today in this week's heatmap. */
  const todayRow = $derived.by(() => {
    if (!stats?.currentWeek) return null;
    const day = dayOfDate(today);
    return day === null ? null : weekdayIndex(day);
  });
  const bestDay = $derived(stats?.bestDay ? formatWeekday(WEEKDAYS.indexOf(stats.bestDay), "long") : "–");
  const emptyText = $derived.by(() => {
    if (!stats) return s.emptyCurrent;
    if (stats.historyOff && stats.firstWeekStart === null) return s.historyOffEmpty;
    // With history off a finished session never shows up here (the note below says why), so no promise that it will.
    return stats.currentWeek && !stats.historyOff ? s.emptyCurrent : s.emptyPast;
  });
  /** Older numbers stay on screen (dimmed) while the picked week loads. */
  const busy = $derived(weekly.pending && stats !== null);

  // The arrows stay focusable at the ends (aria-disabled instead of disabled), so the keyboard focus never drops to
  // the page when the oldest or the current week is reached.
  let previousButton = $state<HTMLButtonElement | null>(null);

  function backToThisWeek(): void {
    weekly.thisWeek();
    // This button goes away with the current week: keep the focus in the picker.
    previousButton?.focus();
  }
</script>

<section class="card week" aria-labelledby="{uid}-title">
  <div class="card-title head">
    <div>
      <h2 id="{uid}-title">{current || !stats ? s.title : s.weekNumber(stats.isoWeek)}</h2>
      {#if stats}
        <p class="muted small tabular">{current ? `${range} · ${s.weekNumber(stats.isoWeek)}` : range}</p>
      {/if}
    </div>
    <div class="picker" role="group" aria-label={s.pickerLabel}>
      {#if !current}
        <button type="button" class="btn btn-sm" onclick={backToThisWeek}>{s.backToThisWeek}</button>
      {/if}
      <button
        type="button"
        class="icon-btn"
        aria-label={s.previous}
        title={s.previous}
        aria-disabled={!canPrevious}
        bind:this={previousButton}
        onclick={() => canPrevious && weekly.previous()}
      >
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path d="M10 3 5 8l5 5" fill="none" stroke="currentColor" stroke-width="2" />
        </svg>
      </button>
      <button
        type="button"
        class="icon-btn"
        aria-label={s.next}
        title={s.next}
        aria-disabled={!canNext}
        onclick={() => canNext && weekly.next()}
      >
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path d="m6 3 5 5-5 5" fill="none" stroke="currentColor" stroke-width="2" />
        </svg>
      </button>
    </div>
  </div>

  {#if weekly.error}
    <div class="error" role="alert">
      <p>{s.error} <span class="muted">{errorText(weekly.error)}</span></p>
      <button type="button" class="btn btn-sm" onclick={() => weekly.retry()}>{s.retry}</button>
    </div>
  {/if}

  {#if !stats}
    {#if weekly.unavailable}
      <p class="secondary empty">{s.emptyCurrent}</p>
    {:else if !weekly.error}
      <p class="hint" role="status">{s.loading}</p>
    {/if}
  {:else}
    <div class="body" class:busy aria-busy={busy}>
      {#if weekEmpty}
        <div class="empty">
          <svg viewBox="0 0 64 40" width="64" height="40" aria-hidden="true">
            {#each [0, 1, 2, 3, 4, 5, 6] as i (i)}
              <rect x={i * 9 + 1} y="30" width="7" height="8" rx="2" style="fill: var(--track-strong)" />
            {/each}
          </svg>
          <p class="secondary">{emptyText}</p>
        </div>
      {:else}
        <dl class="stats">
          <div class="stat hero">
            <dt>{s.focusTotal}</dt>
            <dd class="tabular">{formatDuration(stats.focusSecs)}</dd>
          </div>
          <div class="stat">
            <dt>{s.breaksTaken}</dt>
            <dd class="tabular">{stats.breaksTaken}</dd>
          </div>
          <div class="stat">
            <dt>{s.sessions}</dt>
            <dd class="tabular">{stats.sessions}</dd>
          </div>
          <div class="stat">
            <dt>{s.bestDay}</dt>
            <dd class="best">{bestDay}</dd>
          </div>
        </dl>

        <div class="block">
          <h3 class="block-heading">{s.daysTitle}</h3>
          <ol class="days">
            {#each stats.days as day, i (day.date)}
              {@const ms = localMidnightMs(day.date)}
              {@const time = day.focusSecs > 0 ? formatDuration(day.focusSecs) : s.noFocus}
              {@const height = day.focusSecs > 0 ? Math.max(3, (day.focusSecs / maxDay) * 100) : 0}
              <li class:today={day.date === today}>
                <span class="sr-only">{s.dayLabel(`${formatWeekday(i, "long")}, ${formatShortDate(ms)}`, time)}</span>
                <span class="value tabular" aria-hidden="true">{day.focusSecs > 0 ? formatDuration(day.focusSecs) : ""}</span>
                <span class="column" aria-hidden="true"><span class="bar" style:height="{height}%"></span></span>
                <span class="name" aria-hidden="true">{formatWeekday(i, "short")}</span>
                <span class="date tabular" aria-hidden="true">{formatShortDate(ms)}</span>
              </li>
            {/each}
          </ol>
        </div>
      {/if}

      {#if !weekEmpty || recentAny}
        <div class="block">
          <div class="heat-head">
            <h3 class="block-heading">{s.heatTitle}</h3>
            <fieldset class="segmented small-seg">
              <legend>{s.heatModeLabel}</legend>
              <label>
                <input type="radio" name="{uid}-heat" value="week" bind:group={heatMode} />
                <span>{current ? s.title : s.weekNumber(stats.isoWeek)}</span>
              </label>
              <label>
                <input type="radio" name="{uid}-heat" value="recent" bind:group={heatMode} />
                <span>{s.heatRecent(stats.recentWeeks)}</span>
              </label>
            </fieldset>
          </div>
          {#if heatMode === "week"}
            <WeekHeatmap cells={stats.heatmap} caption={s.heatCaption(range)} todayIndex={todayRow} />
          {:else}
            <WeekHeatmap
              cells={stats.recentHeatmap}
              caption={s.heatRecentCaption(stats.recentWeeks, recentRange)}
              todayIndex={null}
            />
          {/if}
          {#if stats.estimated}<p class="hint estimated">{s.estimated}</p>{/if}
        </div>
      {/if}

      {#if !weekEmpty}
        <div class="block">
          <WeekApps apps={stats.topApps} totals={stats.categories} />
        </div>
      {/if}

      {#if stats.historyOff && !(weekEmpty && stats.firstWeekStart === null)}
        <p class="hint note">{s.historyOff}</p>
      {/if}
      <p class="hint note">{s.privacy}</p>
    </div>
  {/if}
</section>

<style>
  .head {
    align-items: flex-start;
  }

  .picker {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  /* Like `.icon-btn:disabled` in app.css, but still focusable. */
  .picker .icon-btn[aria-disabled="true"] {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .picker .icon-btn[aria-disabled="true"]:hover {
    background: var(--surface-inset);
  }

  :global([data-theme="pixel"]) .picker .icon-btn[aria-disabled="true"]:active {
    transform: none;
    box-shadow: 3px 3px 0 0 var(--shadow-color);
  }

  .error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--danger-line);
    background: var(--danger-soft);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 22px;
    transition: opacity 0.2s ease;
  }

  .body.busy {
    opacity: 0.6;
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    margin: 0;
  }

  .stat {
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
    min-width: 0;
  }

  dt {
    font-size: 0.84rem;
    color: var(--text-muted);
  }

  dd {
    margin: 2px 0 0;
    font-size: 1.4rem;
    font-weight: 650;
  }

  .stat.hero dd {
    color: var(--accent-strong);
  }

  dd.best {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-transform: capitalize;
  }

  .block-heading {
    margin-bottom: 10px;
  }

  .days {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 10px;
  }

  .days li {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .value {
    min-height: 1.3em;
    font-size: 0.78rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .column {
    display: flex;
    align-items: flex-end;
    width: min(44px, 100%);
    height: 110px;
    border-radius: var(--radius-sm);
    background: var(--track);
    overflow: hidden;
  }

  .bar {
    display: block;
    width: 100%;
    background: var(--accent);
    border-radius: inherit;
    transition: height 0.3s var(--ease-slide);
  }

  .name {
    font-weight: 600;
    font-size: 0.86rem;
    color: var(--text-2);
  }

  .date {
    font-size: 0.74rem;
    color: var(--text-muted);
  }

  .days li.today .name,
  .days li.today .date {
    color: var(--accent-strong);
  }

  .days li.today .column {
    box-shadow: inset 0 0 0 var(--border-w) var(--accent-line);
  }

  .heat-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }

  .heat-head .block-heading {
    margin-bottom: 0;
  }

  .small-seg span {
    padding: 4px 12px;
    font-size: 0.84rem;
  }

  .note {
    margin-top: -8px;
  }

  .estimated {
    margin-top: 6px;
  }
</style>
