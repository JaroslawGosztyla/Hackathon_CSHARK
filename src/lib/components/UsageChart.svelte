<script lang="ts">
  // Horizontal bar chart: focus time by process (top 8 + "Inne"), then idle and unknown as muted bars.
  // Single measure → single hue; values are printed at the bar tips, so the rows double as the table view.
  // Apps with recorded window titles (`trackTitles`) show their most-used windows in a tooltip on hover and on
  // keyboard focus (the row is focusable and describes itself with the list); Escape hides it.
  import { formatDuration, formatPercent, processLabel } from "$lib/format";
  import { strings } from "$lib/strings";
  import type { TitleUsage, UsageReport } from "$lib/types";

  interface Props {
    usage: UsageReport;
  }

  let { usage }: Props = $props();

  const TOP_APPS = 8;
  const uid = $props.id();

  interface Row {
    key: string;
    label: string;
    title: string;
    secs: number;
    muted: boolean;
    titles: TitleUsage[];
  }

  const rows = $derived.by((): Row[] => {
    const apps = usage.apps.filter((a) => a.secs > 0);
    const out: Row[] = apps.slice(0, TOP_APPS).map((a) => ({
      key: `app:${a.process}`,
      label: processLabel(a.process),
      title: a.process,
      secs: a.secs,
      muted: false,
      // Older history entries have no `titles`.
      titles: (a.titles ?? []).filter((t) => t.secs > 0 && t.title.trim().length > 0),
    }));
    const rest = apps.slice(TOP_APPS);
    if (rest.length > 0) {
      out.push({
        key: "other",
        label: strings.summary.other(rest.length),
        title: rest.map((a) => a.process).join(", "),
        secs: rest.reduce((sum, a) => sum + a.secs, 0),
        muted: true,
        titles: [],
      });
    }
    if (usage.idleSecs > 0) {
      out.push({
        key: "idle",
        label: strings.summary.idle,
        title: strings.summary.idle,
        secs: usage.idleSecs,
        muted: true,
        titles: [],
      });
    }
    if (usage.unknownSecs > 0) {
      out.push({
        key: "unknown",
        label: strings.summary.unknown,
        title: strings.summary.unknown,
        secs: usage.unknownSecs,
        muted: true,
        titles: [],
      });
    }
    return out;
  });

  const total = $derived(usage.trackedSecs > 0 ? usage.trackedSecs : rows.reduce((s, r) => s + r.secs, 0));
  const maxSecs = $derived(Math.max(1, ...rows.map((r) => r.secs)));
  const anyTitles = $derived(rows.some((r) => r.titles.length > 0));

  /** Row whose tooltip the user dismissed with Escape; it shows again after the pointer or focus leaves. */
  let dismissed = $state<string | null>(null);
  let hovered: string | null = null;
  let focused: string | null = null;

  /** Escape hides the open tooltip (WCAG 1.4.13), whether it opened on hover or on focus. */
  function onWindowKeydown(event: KeyboardEvent): void {
    if (event.key !== "Escape") return;
    const key = focused ?? hovered;
    if (key !== null && dismissed !== key) dismissed = key;
  }

  function leave(key: string, how: "pointer" | "focus"): void {
    if (how === "pointer" && hovered === key) hovered = null;
    if (how === "focus" && focused === key) focused = null;
    if (dismissed === key && hovered !== key && focused !== key) dismissed = null;
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if rows.length === 0}
  <p class="hint">{strings.summary.chartEmpty}</p>
{:else}
  <ul class="chart" aria-label={strings.summary.chartTitle}>
    {#each rows as row, i (row.key)}
      {@const pct = formatPercent(total > 0 ? row.secs / total : 0)}
      {@const time = formatDuration(row.secs)}
      {@const width = Math.max(0.5, (row.secs / maxSecs) * 100)}
      {#if row.titles.length > 0}
        <!-- The row is focusable so keyboard users get the hover tooltip too (and screen readers its list). -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <li
          class="has-titles"
          class:dismissed={dismissed === row.key}
          tabindex="0"
          aria-label={strings.summary.barAria(row.label, time, pct)}
          aria-describedby="{uid}-titles-{i}"
          onpointerenter={() => (hovered = row.key)}
          onpointerleave={() => leave(row.key, "pointer")}
          onfocusin={() => (focused = row.key)}
          onfocusout={() => leave(row.key, "focus")}
        >
          <span class="label">{row.label}</span>
          <span class="plot" aria-hidden="true">
            <span class="track">
              <span class="bar" style:width="{width}%"></span>
              <span class="value tabular" style:left="calc({width}% + 8px)">{time} · {pct}</span>
            </span>
          </span>
          <div class="tooltip" id="{uid}-titles-{i}" role="tooltip">
            <p class="tooltip-title">{row.title} · {strings.summary.titlesHeading}</p>
            <ul class="titles">
              {#each row.titles as t, j (j)}
                <li>
                  <span class="window">{t.title}</span>
                  <span class="secs tabular">{formatDuration(t.secs)}</span>
                </li>
              {/each}
            </ul>
          </div>
        </li>
      {:else}
        <li class:muted={row.muted} title={`${row.title}: ${time} (${pct})`} aria-label={strings.summary.barAria(row.label, time, pct)}>
          <span class="label">{row.label}</span>
          <span class="plot" aria-hidden="true">
            <span class="track">
              <span class="bar" style:width="{width}%"></span>
              <span class="value tabular" style:left="calc({width}% + 8px)">{time} · {pct}</span>
            </span>
          </span>
        </li>
      {/if}
    {/each}
  </ul>
  {#if anyTitles}
    <p class="hint titles-hint">{strings.summary.titlesHint}</p>
  {/if}
{/if}

<style>
  .chart {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .chart > li {
    position: relative;
    display: grid;
    grid-template-columns: minmax(84px, 11rem) 1fr;
    align-items: center;
    gap: 12px;
    border-radius: 8px;
  }

  .chart > li:hover .bar {
    filter: brightness(1.12);
  }

  .chart > li:hover .label {
    color: var(--text);
  }

  .has-titles {
    cursor: help;
  }

  .has-titles .label {
    text-decoration: underline dotted;
    text-decoration-color: var(--text-muted);
    text-underline-offset: 3px;
  }

  .has-titles:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-2);
    font-size: 0.92rem;
    text-align: right;
  }

  .plot {
    display: flex;
    align-items: center;
    min-width: 0;
    /* one hairline baseline the bars grow from */
    border-left: 1px solid color-mix(in srgb, var(--text) 18%, transparent);
    min-height: 26px;
  }

  /* The track leaves room on the right for the value label at the tip of the longest bar. */
  .track {
    position: relative;
    width: calc(100% - 8.5rem);
    height: 18px;
  }

  .bar {
    display: block;
    height: 100%;
    border-radius: 0 4px 4px 0;
    background: var(--accent);
    transition: width 0.5s ease;
  }

  .muted .bar {
    background: var(--muted-mark);
  }

  .value {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.85rem;
    color: var(--text-2);
    white-space: nowrap;
  }

  /* Tooltip: below the row, aligned with the bars; it is part of the row, so moving onto it keeps it open. */
  .tooltip {
    position: absolute;
    z-index: 5;
    top: calc(100% + 2px);
    left: calc(min(11rem, 40%) + 12px);
    width: max-content;
    max-width: min(440px, calc(100% - 12px));
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-raised);
    box-shadow: var(--shadow);
    color: var(--text);
    font-size: 0.85rem;
    visibility: hidden;
    opacity: 0;
    transform: translateY(-4px);
    transition:
      opacity 0.12s ease,
      transform 0.12s ease,
      visibility 0s linear 0.12s;
    pointer-events: none;
  }

  .has-titles:hover .tooltip,
  .has-titles:focus-visible .tooltip {
    visibility: visible;
    opacity: 1;
    transform: none;
    transition-delay: 0s;
    pointer-events: auto;
  }

  .has-titles.dismissed .tooltip {
    visibility: hidden;
    opacity: 0;
  }

  .tooltip-title {
    margin-bottom: 6px;
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .titles {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .titles li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
  }

  .window {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .secs {
    flex: none;
    color: var(--text-2);
  }

  .titles-hint {
    margin-top: 12px;
  }
</style>
