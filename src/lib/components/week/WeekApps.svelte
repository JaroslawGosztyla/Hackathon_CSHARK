<script lang="ts">
  // Weekly stats (ADR-023): the week's top apps with the user's own labels, and the share of the week's app time per
  // label. Only the user labels apps (a chip sets the label, the same chip again clears it; nothing is labelled by
  // default); the chips follow the config at once, the totals (which count every app of the week, not only the listed
  // ones) follow when the shell answers `history://changed`. Labels are words and colours, never colours alone.
  import { formatDuration, formatPercent, processLabel } from "$lib/format";
  import { weekly } from "$lib/state/weekly.svelte";
  import { strings } from "$lib/strings";
  import type { AppCategory, CategoryTotals, WeekApp } from "$lib/types";

  interface Props {
    apps: WeekApp[];
    totals: CategoryTotals;
  }

  let { apps, totals }: Props = $props();

  type Share = AppCategory | "unlabelled";
  const CATEGORIES: readonly AppCategory[] = ["productive", "neutral", "distracting"];
  const SHARES: readonly Share[] = ["productive", "neutral", "distracting", "unlabelled"];
  const s = strings.week;

  const maxSecs = $derived(Math.max(1, ...apps.map((a) => a.secs)));
  const shareSecs = $derived<Record<Share, number>>({
    productive: totals.productiveSecs,
    neutral: totals.neutralSecs,
    distracting: totals.distractingSecs,
    unlabelled: totals.unlabelledSecs,
  });
  const shareTotal = $derived(SHARES.reduce((sum, k) => sum + shareSecs[k], 0));
</script>

<div class="apps-block">
  <div class="block-title">
    <h3>{s.appsTitle}</h3>
    <p class="hint">{s.appsHint}</p>
  </div>

  {#if apps.length === 0}
    <p class="hint">{s.appsEmpty}</p>
  {:else}
    <ul class="apps">
      {#each apps as app (app.process)}
        {@const name = processLabel(app.process)}
        {@const label = weekly.labelOf(app.process)}
        {@const width = Math.max(0.5, (app.secs / maxSecs) * 100)}
        <li class="app">
          <span class="name" title={app.process}>{name}</span>
          <span class="track" aria-hidden="true">
            <span class="bar" data-cat={label ?? "unlabelled"} style:width="{width}%"></span>
          </span>
          <span class="time tabular">{formatDuration(app.secs)}</span>
          <span class="chips" role="group" aria-label={s.labelGroup(name)}>
            {#each CATEGORIES as cat (cat)}
              {@const on = label === cat}
              <button
                type="button"
                class="cat"
                data-cat={cat}
                aria-pressed={on}
                title={on ? s.clearLabel : undefined}
                onclick={() => weekly.toggleLabel(app.process, cat)}
              >
                {#if on}
                  <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
                    <path d="M2 6.5 5 9.2 10 3" fill="none" stroke="currentColor" stroke-width="2" />
                  </svg>
                {/if}
                {s.labels[cat]}
              </button>
            {/each}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if shareTotal > 0}
    <div class="block-title share-title">
      <h3>{s.shareTitle}</h3>
    </div>
    <div class="share-bar" aria-hidden="true">
      {#each SHARES as k (k)}
        {#if shareSecs[k] > 0}<span class="seg" data-cat={k} style:flex-grow={shareSecs[k]}></span>{/if}
      {/each}
    </div>
    <ul class="share-legend">
      {#each SHARES as k (k)}
        {@const secs = shareSecs[k]}
        {@const time = formatDuration(secs)}
        {@const pct = formatPercent(secs / shareTotal)}
        <li class:none={secs === 0}>
          <span class="swatch" data-cat={k} aria-hidden="true"></span>
          <span class="sr-only">{s.shareItem(s.shares[k], time, pct)}</span>
          <span aria-hidden="true">{s.shares[k]}</span>
          <span class="tabular muted" aria-hidden="true">{time} · {pct}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  [data-cat="productive"] {
    --cat: var(--chart-3);
  }

  [data-cat="neutral"] {
    --cat: var(--chart-1);
  }

  [data-cat="distracting"] {
    --cat: var(--chart-5);
  }

  [data-cat="unlabelled"] {
    --cat: var(--chart-other);
  }

  .block-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 10px;
  }

  .share-title {
    margin-top: 18px;
  }

  /* One grid for all rows (each row a subgrid), so the bars, times and chips line up whatever the texts. */
  .apps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: minmax(6rem, 10rem) minmax(40px, 1fr) auto auto;
    gap: 8px 12px;
  }

  .app {
    display: grid;
    grid-column: 1 / -1;
    grid-template-columns: subgrid;
    align-items: center;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-2);
  }

  .track {
    height: 12px;
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }

  .bar {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--cat);
    transition: width 0.3s var(--ease-slide);
  }

  .time {
    font-size: 0.86rem;
    color: var(--text-muted);
    white-space: nowrap;
    text-align: right;
  }

  /* Equal chips in every row: the label column is as wide as its longest chip, three times. */
  .chips {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }

  .cat {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 28px;
    padding: 2px 10px;
    border-radius: var(--radius-pill);
    border: var(--border-w) solid var(--border);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .cat:hover {
    border-color: var(--cat);
    color: var(--text);
  }

  .cat[aria-pressed="true"] {
    border-color: var(--cat);
    background: color-mix(in srgb, var(--cat) 24%, transparent);
    color: var(--text);
  }

  .cat:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .share-bar {
    display: flex;
    height: 16px;
    border-radius: var(--radius-pill);
    overflow: hidden;
    background: var(--track);
    gap: 2px;
  }

  .seg {
    flex: 1 1 0;
    min-width: 4px;
    background: var(--cat);
  }

  .share-legend {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    font-size: 0.88rem;
  }

  .share-legend li {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .share-legend li.none {
    opacity: 0.6;
  }

  .swatch {
    width: 12px;
    height: 12px;
    border-radius: min(3px, var(--radius-sm));
    background: var(--cat);
  }
</style>
