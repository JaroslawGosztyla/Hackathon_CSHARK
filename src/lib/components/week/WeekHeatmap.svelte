<script lang="ts">
  // Weekly stats (ADR-023): focus by weekday × local hour. A real table, so it is its own text alternative: every cell
  // is a <td> with its time for screen readers (row = weekday, column = hour) and a hover title; the colour is one of
  // five steps of the theme's accent (every theme, 8-bit included), relative to the busiest cell of this map.
  import { formatDuration, formatHour, formatWeekday } from "$lib/format";
  import { strings } from "$lib/strings";
  import { HOURS_PER_DAY } from "$lib/week";

  interface Props {
    /** 7 rows (Monday..Sunday) of 24 cells (00..23): focus seconds. */
    cells: number[][];
    /** Read by screen readers before the table (what the numbers are and for which weeks). */
    caption: string;
    /** Row of today (0 = Monday), highlighted; null when today is not in this map. */
    todayIndex: number | null;
  }

  let { cells, caption, todayIndex }: Props = $props();

  /** Coloured steps above "no focus". */
  const LEVELS = 4;
  /** A visible hour label every this many columns (screen readers get every hour). */
  const HOUR_LABEL_EVERY = 3;
  const HOURS = Array.from({ length: HOURS_PER_DAY }, (_, h) => h);
  const LEGEND = Array.from({ length: LEVELS + 1 }, (_, l) => l);

  const max = $derived(cells.reduce((top, row) => Math.max(top, ...row), 0));

  function level(secs: number): number {
    if (secs <= 0 || max <= 0) return 0;
    return Math.min(LEVELS, Math.max(1, Math.ceil((secs / max) * LEVELS)));
  }

  const pad2 = (n: number) => String(n).padStart(2, "0");
</script>

<div class="heat-wrap">
  <table class="heat">
    <caption class="sr-only">{caption}</caption>
    <thead>
      <tr>
        <th scope="col" class="corner"><span class="sr-only">{strings.week.heatDay}</span></th>
        {#each HOURS as h (h)}
          <th scope="col" class="hour tabular">
            <span aria-hidden="true">{h % HOUR_LABEL_EVERY === 0 ? pad2(h) : ""}</span>
            <span class="sr-only">{formatHour(h)}</span>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each cells as row, d (d)}
        {@const dayLong = formatWeekday(d, "long")}
        <tr class:today={todayIndex === d}>
          <th scope="row" class="day">
            <span aria-hidden="true">{formatWeekday(d, "short")}</span>
            <span class="sr-only">{dayLong}</span>
          </th>
          {#each row as secs, h (h)}
            {@const time = secs > 0 ? formatDuration(secs) : strings.week.noFocus}
            <td class="cell" data-level={level(secs)} title={strings.week.heatCell(dayLong, formatHour(h), time)}>
              <span class="sr-only">{time}</span>
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="legend" aria-hidden="true">
    <span>{strings.week.heatLess}</span>
    {#each LEGEND as l (l)}<span class="cell swatch" data-level={l}></span>{/each}
    <span>{strings.week.heatMore}</span>
  </p>
</div>

<style>
  .heat-wrap {
    min-width: 0;
  }

  .heat {
    width: 100%;
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 3px;
  }

  .corner,
  .day {
    width: 3.4rem;
  }

  .hour {
    padding: 0 0 2px;
    font-size: 0.72rem;
    font-weight: 500;
    color: var(--text-muted);
    text-align: left;
    white-space: nowrap;
    overflow: visible;
  }

  .day {
    padding: 0 6px 0 0;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-2);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr.today .day {
    color: var(--accent-strong);
  }

  .cell {
    height: 22px;
    padding: 0;
    border-radius: min(4px, var(--radius-sm));
    background: var(--track);
  }

  .cell[data-level="1"] {
    background: color-mix(in srgb, var(--accent) 30%, var(--track));
  }

  .cell[data-level="2"] {
    background: color-mix(in srgb, var(--accent) 55%, var(--track));
  }

  .cell[data-level="3"] {
    background: color-mix(in srgb, var(--accent) 78%, var(--track));
  }

  .cell[data-level="4"] {
    background: var(--accent);
  }

  tr.today .cell {
    box-shadow: inset 0 0 0 1px var(--accent-line);
  }

  .legend {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    margin-top: 8px;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .legend span:first-child {
    margin-right: 4px;
  }

  .legend span:last-child {
    margin-left: 4px;
  }

  .swatch {
    display: inline-block;
    width: 14px;
    height: 14px;
  }
</style>
