<script lang="ts">
  import HistoryList from "$lib/components/HistoryList.svelte";
  import UsageChart from "$lib/components/UsageChart.svelte";
  import { formatDuration, formatSpan } from "$lib/format";
  import { summary } from "$lib/state/summary.svelte";
  import { strings } from "$lib/strings";

  const selected = $derived(summary.selected);
  const isLast = $derived(selected !== null && selected.sessionId === summary.last?.sessionId);
</script>

<svelte:head>
  <title>{strings.summary.title} · {strings.appName}</title>
</svelte:head>

<div class="page">
  <h1>{strings.summary.title}</h1>

  {#if !summary.loaded}
    <p class="hint">{strings.player.loading}</p>
  {:else if !selected}
    <section class="card empty">
      <svg viewBox="0 0 64 64" width="56" height="56" aria-hidden="true">
        <rect x="8" y="34" width="10" height="20" rx="3" style="fill: var(--muted-mark)" />
        <rect x="27" y="22" width="10" height="32" rx="3" style="fill: var(--accent)" opacity="0.7" />
        <rect x="46" y="12" width="10" height="42" rx="3" style="fill: var(--muted-mark)" />
      </svg>
      <p class="secondary">{strings.summary.empty}</p>
    </section>
  {:else}
    <section class="card session" aria-labelledby="session-title">
      <div class="card-title">
        <div>
          <h2 id="session-title">{isLast ? strings.summary.lastSession : strings.summary.selectedSession}</h2>
          <p class="muted small tabular">{formatSpan(selected.startedAtMs, selected.endedAtMs)}</p>
        </div>
        <div class="row">
          {#if selected.demo}<span class="badge">{strings.summary.demo}</span>{/if}
          {#if !isLast}
            <button type="button" class="btn btn-sm" onclick={() => summary.select(null)}>{strings.summary.backToLast}</button>
          {/if}
        </div>
      </div>

      <dl class="stats">
        <div class="stat hero">
          <dt>{strings.summary.focusTime}</dt>
          <dd>{formatDuration(selected.focusedSecs)}</dd>
        </div>
        <div class="stat">
          <dt>{strings.summary.cycles}</dt>
          <dd>{selected.cyclesCompleted}</dd>
        </div>
        <div class="stat">
          <dt>{strings.summary.breaks}</dt>
          <dd>{formatDuration(selected.breakSecs)}</dd>
        </div>
      </dl>
    </section>

    <section class="card" aria-labelledby="chart-title">
      <div class="card-title">
        <div>
          <h2 id="chart-title">{strings.summary.chartTitle}</h2>
          <p class="muted small">{strings.summary.chartSubtitle(formatDuration(selected.usage.trackedSecs))}</p>
        </div>
      </div>
      <UsageChart usage={selected.usage} />
      <p class="hint privacy">{strings.summary.privacy}</p>
    </section>
  {/if}

  {#if summary.loaded && summary.history.length > 0}
    <section class="card" aria-labelledby="history-title">
      <div class="card-title">
        <h2 id="history-title">{strings.summary.history}</h2>
      </div>
      <HistoryList
        history={summary.history}
        selectedId={selected?.sessionId ?? null}
        onselect={(id) => summary.select(id)}
      />
    </section>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 18px;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 12px;
    margin: 0;
  }

  .stat {
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
  }

  dt {
    font-size: 0.84rem;
    color: var(--text-muted);
  }

  dd {
    margin: 2px 0 0;
    font-size: 1.5rem;
    font-weight: 650;
  }

  .stat.hero dd {
    font-size: clamp(2.2rem, 5vw, 3rem);
    line-height: 1.1;
    color: var(--accent-strong);
  }

  .privacy {
    margin-top: 16px;
  }
</style>
