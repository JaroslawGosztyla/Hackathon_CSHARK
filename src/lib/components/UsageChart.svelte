<script lang="ts">
  // Horizontal bar chart: focus time by process (top 8 + "Inne"), then idle and unknown as muted bars.
  // Single measure → single hue; values are printed at the bar tips, so the rows double as the table view.
  import { formatDuration, formatPercent, processLabel } from "$lib/format";
  import { strings } from "$lib/strings";
  import type { UsageReport } from "$lib/types";

  interface Props {
    usage: UsageReport;
  }

  let { usage }: Props = $props();

  const TOP_APPS = 8;

  interface Row {
    key: string;
    label: string;
    title: string;
    secs: number;
    muted: boolean;
  }

  const rows = $derived.by((): Row[] => {
    const apps = usage.apps.filter((a) => a.secs > 0);
    const out: Row[] = apps.slice(0, TOP_APPS).map((a) => ({
      key: `app:${a.process}`,
      label: processLabel(a.process),
      title: a.process,
      secs: a.secs,
      muted: false,
    }));
    const rest = apps.slice(TOP_APPS);
    if (rest.length > 0) {
      out.push({
        key: "other",
        label: strings.summary.other(rest.length),
        title: rest.map((a) => a.process).join(", "),
        secs: rest.reduce((sum, a) => sum + a.secs, 0),
        muted: true,
      });
    }
    if (usage.idleSecs > 0) {
      out.push({ key: "idle", label: strings.summary.idle, title: strings.summary.idle, secs: usage.idleSecs, muted: true });
    }
    if (usage.unknownSecs > 0) {
      out.push({
        key: "unknown",
        label: strings.summary.unknown,
        title: strings.summary.unknown,
        secs: usage.unknownSecs,
        muted: true,
      });
    }
    return out;
  });

  const total = $derived(usage.trackedSecs > 0 ? usage.trackedSecs : rows.reduce((s, r) => s + r.secs, 0));
  const maxSecs = $derived(Math.max(1, ...rows.map((r) => r.secs)));
</script>

{#if rows.length === 0}
  <p class="hint">{strings.summary.chartEmpty}</p>
{:else}
  <ul class="chart" aria-label={strings.summary.chartTitle}>
    {#each rows as row (row.key)}
      {@const pct = formatPercent(total > 0 ? row.secs / total : 0)}
      {@const time = formatDuration(row.secs)}
      {@const width = Math.max(0.5, (row.secs / maxSecs) * 100)}
      <li
        class:muted={row.muted}
        title={`${row.title}: ${time} (${pct})`}
        aria-label={strings.summary.barAria(row.label, time, pct)}
      >
        <span class="label">{row.label}</span>
        <span class="plot" aria-hidden="true">
          <span class="track">
            <span class="bar" style:width="{width}%"></span>
            <span class="value tabular" style:left="calc({width}% + 8px)">{time} · {pct}</span>
          </span>
        </span>
      </li>
    {/each}
  </ul>
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

  li {
    display: grid;
    grid-template-columns: minmax(84px, 11rem) 1fr;
    align-items: center;
    gap: 12px;
    border-radius: 8px;
  }

  li:hover .bar {
    filter: brightness(1.12);
  }

  li:hover .label {
    color: var(--text);
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
    border-left: 1px solid rgba(255, 236, 214, 0.18);
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

  @media (max-width: 560px) {
    li {
      grid-template-columns: 1fr;
      gap: 4px;
    }
    .label {
      text-align: left;
    }
  }
</style>
