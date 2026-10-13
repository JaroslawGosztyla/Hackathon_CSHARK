<script lang="ts">
  import HistoryList from "$lib/components/HistoryList.svelte";
  import UsageChart from "$lib/components/UsageChart.svelte";
  import { formatDuration, formatSpan } from "$lib/format";
  import { config } from "$lib/state/config.svelte";
  import { summary } from "$lib/state/summary.svelte";
  import { isOpenableUrl, work } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";

  const selected = $derived(summary.selected);
  const isLast = $derived(selected !== null && selected.sessionId === summary.last?.sessionId);
  /** History saving is off and the last session is not in the stored history. */
  const notSaved = $derived(
    isLast &&
      config.value?.keepHistory === false &&
      !summary.history.some((s) => s.sessionId === selected?.sessionId),
  );
  const goal = $derived(selected?.goal ?? null);
  const goalName = $derived(goal ? strings.work.itemName(goal.key, goal.title) : "");
  const goalDone = $derived(selected?.goalDone ?? null);
  const goalBusy = $derived(summary.goalBusyId !== null);
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
          {#if notSaved}<p class="hint not-saved">{strings.summary.notSaved}</p>{/if}
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

      {#if goal}
        {@const sessionId = selected.sessionId}
        <div class="goal" class:done={goalDone === true}>
          <p class="goal-line">
            <span class="goal-label">{strings.summary.goalLabel}:</span>
            <span class="badge goal-source">{strings.work.sourceShort[goal.source]}</span>
            {#if isOpenableUrl(goal.url)}
              <button
                type="button"
                class="goal-name link"
                title={goalName}
                aria-label={strings.summary.goalOpenAria(goalName)}
                onclick={() => work.open(goal.url)}
              >
                {#if goal.key}<span class="goal-key">{goal.key}</span>{/if}
                {goal.title}
              </button>
            {:else}
              <span class="goal-name" title={goalName}>
                {#if goal.key}<span class="goal-key">{goal.key}</span>{/if}
                {goal.title}
              </span>
            {/if}
          </p>
          <div class="goal-answer">
            <span class="goal-question" aria-hidden="true">{strings.summary.goalQuestion}</span>
            <div class="row" role="group" aria-label={strings.summary.goalGroup}>
              <button
                type="button"
                class="btn btn-sm"
                class:btn-primary={goalDone === true}
                aria-pressed={goalDone === true}
                disabled={goalBusy}
                onclick={() => summary.markGoalDone(sessionId, true)}
              >
                {strings.summary.goalYes}
              </button>
              <button
                type="button"
                class="btn btn-sm"
                class:chosen-no={goalDone === false}
                aria-pressed={goalDone === false}
                disabled={goalBusy}
                onclick={() => summary.markGoalDone(sessionId, false)}
              >
                {strings.summary.goalNo}
              </button>
            </div>
            {#if goalDone !== null}
              <p class="small secondary" aria-live="polite">
                {goalDone ? strings.summary.goalDone : strings.summary.goalNotDone}
              </p>
            {/if}
          </div>
        </div>
      {/if}
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

  .not-saved {
    margin-top: 4px;
  }

  .goal {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px 16px;
    margin-top: 14px;
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-inset);
  }

  .goal.done {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .goal-line {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 8px;
    min-width: 0;
  }

  .goal-label {
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .goal-source {
    align-self: center;
  }

  .goal-name {
    min-width: 0;
    font-weight: 600;
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-weight: 600;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  .link:hover,
  .link:focus-visible {
    text-decoration: underline;
  }

  .link:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: 4px;
  }

  .goal-key {
    margin-right: 6px;
    font-family: var(--font-mono);
    font-size: 0.88rem;
    color: var(--accent-strong);
  }

  .goal-answer {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
  }

  .goal-question {
    font-weight: 600;
  }

  .goal-answer p {
    flex-basis: 100%;
  }

  .chosen-no {
    border-color: var(--text-2);
    background: var(--surface-hover);
  }
</style>
