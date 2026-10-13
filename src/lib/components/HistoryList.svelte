<script lang="ts">
  // Session history with mini stacked bars. Colors follow the app (entity), not its rank in a row:
  // the 7 apps with the most focus time across the listed history get fixed categorical slots,
  // everything else folds into "Inne"; idle/unknown share one muted gray. A legend names every color.
  import { formatDate, formatDuration, formatShortDate, formatTimeOfDay, processLabel } from "$lib/format";
  import { strings } from "$lib/strings";
  import type { SessionSummary } from "$lib/types";

  interface Props {
    history: SessionSummary[];
    selectedId: string | null;
    onselect: (sessionId: string) => void;
  }

  let { history, selectedId, onselect }: Props = $props();

  /** Categorical steps of the current theme (`--chart-1..7` in app.css), in fixed order. */
  const SLOTS = [1, 2, 3, 4, 5, 6, 7].map((n) => `var(--chart-${n})`);
  const OTHER_COLOR = "var(--chart-other)";
  const IDLE_COLOR = "var(--chart-idle)";

  const normalize = (process: string) => process.toLowerCase();

  const colorMap = $derived.by(() => {
    const totals = new Map<string, { label: string; secs: number }>();
    for (const s of history) {
      for (const app of s.usage.apps) {
        const key = normalize(app.process);
        const entry = totals.get(key);
        if (entry) entry.secs += app.secs;
        else totals.set(key, { label: processLabel(app.process), secs: app.secs });
      }
    }
    const ranked = [...totals.entries()].sort((a, b) => b[1].secs - a[1].secs || a[0].localeCompare(b[0]));
    const map = new Map<string, { label: string; color: string }>();
    ranked.slice(0, SLOTS.length).forEach(([key, v], i) => map.set(key, { label: v.label, color: SLOTS[i] ?? OTHER_COLOR }));
    return { map, hasOther: ranked.length > SLOTS.length };
  });

  interface Segment {
    key: string;
    color: string;
    secs: number;
    title: string;
  }

  function segments(s: SessionSummary): Segment[] {
    const out: Segment[] = [];
    let other = 0;
    for (const app of s.usage.apps) {
      const slot = colorMap.map.get(normalize(app.process));
      if (slot) {
        out.push({ key: `app:${app.process}`, color: slot.color, secs: app.secs, title: `${slot.label}: ${formatDuration(app.secs)}` });
      }
      else other += app.secs;
    }
    if (other > 0) out.push({ key: "other", color: OTHER_COLOR, secs: other, title: `${strings.summary.otherPlain}: ${formatDuration(other)}` });
    const idle = s.usage.idleSecs + s.usage.unknownSecs;
    if (idle > 0) out.push({ key: "idle", color: IDLE_COLOR, secs: idle, title: `${strings.summary.idle}: ${formatDuration(idle)}` });
    return out.filter((seg) => seg.secs > 0);
  }

  const hasIdle = $derived(history.some((s) => s.usage.idleSecs + s.usage.unknownSecs > 0));
</script>

{#if history.length === 0}
  <p class="hint">{strings.summary.historyEmpty}</p>
{:else}
  <ul class="legend" aria-label={strings.summary.historyLegend}>
    {#each [...colorMap.map.values()] as entry (entry.label + entry.color)}
      <li><span class="swatch" style:background={entry.color}></span>{entry.label}</li>
    {/each}
    {#if colorMap.hasOther}
      <li><span class="swatch" style:background={OTHER_COLOR}></span>{strings.summary.otherPlain}</li>
    {/if}
    {#if hasIdle}
      <li><span class="swatch" style:background={IDLE_COLOR}></span>{strings.summary.idle}</li>
    {/if}
  </ul>

  <ul class="history">
    {#each history as s (s.sessionId)}
      {@const segs = segments(s)}
      {@const total = segs.reduce((sum, seg) => sum + seg.secs, 0)}
      <li>
        <button
          type="button"
          class="entry"
          aria-current={s.sessionId === selectedId ? "true" : undefined}
          aria-label={strings.summary.showSession(`${formatDate(s.startedAtMs)}, ${formatTimeOfDay(s.startedAtMs)}`)}
          onclick={() => onselect(s.sessionId)}
        >
          <span class="when">
            <span class="date">{formatShortDate(s.startedAtMs)}</span>
            <span class="time tabular">{formatTimeOfDay(s.startedAtMs)}</span>
          </span>
          <span class="stats">
            <span class="focus tabular">{formatDuration(s.focusedSecs)}</span>
            <span class="cycles">{strings.summary.cyclesCount(s.cyclesCompleted)}</span>
            {#if s.demo}<span class="badge">{strings.summary.demo}</span>{/if}
          </span>
          <span class="mini" aria-hidden="true">
            {#if total > 0}
              {#each segs as seg (seg.key)}
                <span class="seg" style:flex-grow={seg.secs / total} style:background={seg.color} title={seg.title}></span>
              {/each}
            {:else}
              <span class="seg empty"></span>
            {/if}
          </span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .legend {
    list-style: none;
    margin: 0 0 14px;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    font-size: 0.84rem;
    color: var(--text-2);
  }

  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }

  .history {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .entry {
    width: 100%;
    display: grid;
    grid-template-columns: 7.5rem 10rem 1fr;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--surface-inset);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .entry:hover {
    border-color: var(--border-strong);
  }

  .entry[aria-current="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .when,
  .stats {
    display: flex;
    align-items: baseline;
    gap: 8px;
    white-space: nowrap;
  }

  .date {
    font-weight: 600;
  }

  .time,
  .cycles {
    color: var(--text-muted);
    font-size: 0.86rem;
  }

  .focus {
    font-weight: 600;
  }

  .mini {
    display: flex;
    gap: 2px;
    height: 10px;
    min-width: 0;
    border-radius: 4px;
    overflow: hidden;
  }

  .seg {
    flex-basis: 0;
    min-width: 2px;
  }

  .seg.empty {
    flex-grow: 1;
    background: color-mix(in srgb, var(--text) 6%, transparent);
  }

  @media (max-width: 640px) {
    .entry {
      grid-template-columns: 1fr auto;
    }
    .mini {
      grid-column: 1 / -1;
    }
  }
</style>
