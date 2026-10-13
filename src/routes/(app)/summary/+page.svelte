<script lang="ts">
  import { untrack } from "svelte";
  import HistoryList from "$lib/components/HistoryList.svelte";
  import SessionSparksCard from "$lib/components/rewards/SessionSparksCard.svelte";
  import SparkIcon from "$lib/components/rewards/SparkIcon.svelte";
  import StepsCard from "$lib/components/rewards/StepsCard.svelte";
  import StickerAlbum from "$lib/components/rewards/StickerAlbum.svelte";
  import UsageChart from "$lib/components/UsageChart.svelte";
  import { formatDuration, formatSpan } from "$lib/format";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { useMascot } from "$lib/mascot/controller.svelte";
  import { cueForSummary, followCues, followRewardCues, mascotEnabled } from "$lib/mascot/cues.svelte";
  import { moodFrame, reactionSteps, rewardSteps, type MascotStep } from "$lib/mascot/frames";
  import { GOAL_GLOW_MS, pickAt } from "$lib/rewards/choreography";
  import { useRewardPlayer } from "$lib/rewards/player.svelte";
  import { config } from "$lib/state/config.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { session } from "$lib/state/session.svelte";
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

  // The axolotl: sad over an empty summary; in the session card it is sad after a Stop before one full focus phase
  // (cue from app.ts) and happy when the goal was done (a short cheer when the answer changes to "Tak").
  const MASCOT_SIZE_EMPTY = 96;
  const MASCOT_SIZE_CARD = 66;
  const mascotOn = $derived(mascotEnabled(config.value));
  const mascot = useMascot("main");

  // ---------------------------------------------------------------------------
  // Beta rewards (ADR-022): the session-end recap, the goal-done sparks, and no sad face after an early Stop.
  // ---------------------------------------------------------------------------

  /** Poses replace the old reactions while the rewards are active (numbers only: no pose changes). */
  const posesOn = $derived(rewards.configActive && rewards.celebration !== "numbersOnly");
  /** Length of one focus phase in the profile in use (as in app.ts, for the early-Stop check). */
  const focusSecs = $derived(session.snapshot.focusSecs > 0 ? session.snapshot.focusSecs : (config.value?.focusMinutes ?? 0) * 60);
  /** The shown session is the last one and it ended before one full focus phase. */
  const earlyStop = $derived(isLast && selected !== null && cueForSummary(selected, focusSecs) === "stopEarly");
  /** Calm (and reduced motion, which caps it at calm): one still pose per beat. */
  const stillPoses = $derived(rewards.celebration !== "full");
  /** The early Stop is never sad while the beta is on: holding a heart (a smile with numbers only or without sparks). */
  function stopEarlySteps(): readonly MascotStep[] | null {
    if (!rewards.betaOn) return null;
    return posesOn ? rewardSteps("earlyStop", stillPoses) : reactionSteps("main", "smile", stillPoses);
  }

  followCues(mascot, ["stopEarly"], undefined, stopEarlySteps);
  followRewardCues(mascot, () => mascotOn);
  const player = useRewardPlayer("summary", { countPose: () => (earlyStop ? "earlyStop" : "recap") });

  /** The sparks of the shown session, when the rewards know them. */
  const sessionSparks = $derived.by((): number | null => {
    if (!selected) return null;
    const own = rewards.view?.session;
    if (own && own.sessionId === selected.sessionId) return own.sparks;
    return selected.sparks ?? null;
  });
  /** The session-end recap on stage belongs to the shown session (a sample always shows). */
  const ownBurst = $derived.by(() => {
    const burst = player.burst;
    if (!burst || burst.moment.kind !== "sessionEnd" || !selected) return false;
    return burst.moment.sample || rewards.view?.session?.sessionId === selected.sessionId;
  });
  /** Only for a session with sparks ("+0" is never shown) or while its recap plays. */
  const showSparksCard = $derived(rewards.active && selected !== null && ((sessionSparks ?? 0) > 0 || ownBurst));
  /** A calm caption under an early Stop (picked once per shown session). */
  const earlyCaption = $derived.by(() => {
    if (!rewards.configActive || !earlyStop || !sessionSparks || !selected) return null;
    return pickAt(strings.rewards.earlyStopLines(sessionSparks), selected.startedAtMs);
  });
  const stepsShown = $derived(rewards.betaOn ? (selected?.steps ?? []) : []);
  /** "Cel zrobiony! +10" while the goal-done moment plays (the goal box glows unless the celebration is still). */
  const goalChip = $derived(player.current?.moment.kind === "goalDone" ? player.current : null);

  let notDoneSeenFor: string | null = null;

  $effect(() => {
    const id = selected?.sessionId ?? null;
    const done = goalDone === true;
    untrack(() => {
      mascot.setMood(done ? "happy" : "neutral");
      // Cheer only for a change seen here ("Tak" clicked), not for a session that was already done.
      if (done && notDoneSeenFor === id) {
        if (posesOn) mascot.play(rewardSteps("goalDone", stillPoses));
        else mascot.react("cycle");
      }
      notDoneSeenFor = done ? null : id;
    });
  });
</script>

<svelte:head>
  <title>{strings.summary.title} · {strings.appName}</title>
</svelte:head>

<div class="page">
  <h1>{strings.summary.title}</h1>
  <!-- One sentence per reward moment (beta); empty while nothing plays. -->
  <p class="sr-only" role="status" aria-live="polite">{player.announcement}</p>

  {#if !summary.loaded}
    <p class="hint">{strings.player.loading}</p>
  {:else if !selected}
    <section class="card empty">
      {#if mascotOn}
        <Axolotl size={MASCOT_SIZE_EMPTY} frame={moodFrame("main", "sad")} />
      {:else}
        <svg viewBox="0 0 64 64" width="56" height="56" aria-hidden="true">
          <rect x="8" y="34" width="10" height="20" rx="3" style="fill: var(--muted-mark)" />
          <rect x="27" y="22" width="10" height="32" rx="3" style="fill: var(--accent)" opacity="0.7" />
          <rect x="46" y="12" width="10" height="42" rx="3" style="fill: var(--muted-mark)" />
        </svg>
      {/if}
      <p class="secondary">{strings.summary.empty}</p>
    </section>
  {:else}
    <section class="card session" aria-labelledby="session-title">
      {#if mascotOn}
        <span class="card-mascot"><Axolotl size={MASCOT_SIZE_CARD} frame={mascot.frame} /></span>
      {/if}
      <div class="card-title" class:with-mascot={mascotOn}>
        <div>
          <h2 id="session-title">{isLast ? strings.summary.lastSession : strings.summary.selectedSession}</h2>
          <p class="muted small tabular">{formatSpan(selected.startedAtMs, selected.endedAtMs)}</p>
          {#if notSaved}<p class="hint not-saved">{strings.summary.notSaved}</p>{/if}
          {#if earlyCaption}<p class="early-caption">{earlyCaption}</p>{/if}
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
        <div
          class="goal"
          class:done={goalDone === true}
          class:glow={goalChip !== null && !goalChip.still}
          style:--glow-ms="{GOAL_GLOW_MS}ms"
        >
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
                {rewards.configActive ? strings.rewards.goalNotYet : strings.summary.goalNo}
              </button>
            </div>
            {#if goalChip}
              <span class="goal-chip" aria-hidden="true"><SparkIcon size={12} /> {goalChip.caption}</span>
            {/if}
            {#if goalDone !== null}
              <p class="small secondary" aria-live="polite">
                {goalDone
                  ? strings.summary.goalDone
                  : rewards.configActive
                    ? strings.rewards.goalNotYetNote
                    : strings.summary.goalNotDone}
              </p>
            {/if}
          </div>
        </div>
      {/if}

      {#if stepsShown.length > 0}
        <StepsCard mode="summary" steps={stepsShown} latest={isLast} />
      {/if}
    </section>

    {#if showSparksCard}
      <SessionSparksCard {player} summary={selected} />
    {/if}

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

  {#if rewards.active && rewards.view}
    <!-- Beta rewards (P1, ADR-022): the sticker album, at the bottom, opened on purpose. -->
    <StickerAlbum unlocked={rewards.view.stickers} />
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

  /* In the session card's top-right corner; the title row leaves room for it. */
  .session {
    position: relative;
  }

  .card-mascot {
    position: absolute;
    top: 6px;
    right: 12px;
  }

  .card-title.with-mascot {
    padding-right: 64px;
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

  .early-caption {
    margin-top: 4px;
    color: var(--reward-ink);
    font-size: 0.9rem;
    font-weight: 600;
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

  /* Beta rewards: the goal-done chip and a short glow of the goal box. */
  .goal-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 10px;
    border-radius: var(--radius-pill);
    background: var(--reward-soft);
    color: var(--reward-ink);
    font-size: 0.86rem;
    font-weight: 700;
  }

  .goal.glow {
    animation: goal-glow var(--glow-ms) ease-out;
  }

  @keyframes goal-glow {
    50% {
      box-shadow: 0 0 0 3px var(--reward-line);
    }
  }

  .chosen-no {
    border-color: var(--text-2);
    background: var(--surface-hover);
  }
</style>
