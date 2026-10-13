<script lang="ts">
  import BreakCard from "$lib/components/BreakCard.svelte";
  import GoodPractice from "$lib/components/GoodPractice.svelte";
  import LiveUsage from "$lib/components/LiveUsage.svelte";
  import MeetingConflicts from "$lib/components/MeetingConflicts.svelte";
  import MeetingHoldBanner from "$lib/components/MeetingHoldBanner.svelte";
  import MusicSourcePicker from "$lib/components/MusicSourcePicker.svelte";
  import NextUp from "$lib/components/NextUp.svelte";
  import FloatSpark from "$lib/components/rewards/FloatSpark.svelte";
  import NextStep from "$lib/components/rewards/NextStep.svelte";
  import RecapChip from "$lib/components/rewards/RecapChip.svelte";
  import RewardBurst from "$lib/components/rewards/RewardBurst.svelte";
  import RewardsIntro from "$lib/components/rewards/RewardsIntro.svelte";
  import SparkChip from "$lib/components/rewards/SparkChip.svelte";
  import StepsCard from "$lib/components/rewards/StepsCard.svelte";
  import TodayStrip from "$lib/components/rewards/TodayStrip.svelte";
  import SessionGoalLine from "$lib/components/SessionGoalLine.svelte";
  import ShieldChips from "$lib/components/ShieldChips.svelte";
  import TimerRing from "$lib/components/TimerRing.svelte";
  import TimingEditor from "$lib/components/TimingEditor.svelte";
  import { formatDuration } from "$lib/format";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import FocusMascot, { MASCOT_WIDTH } from "$lib/mascot/FocusMascot.svelte";
  import { calendar, calendarEnabled } from "$lib/state/calendar.svelte";
  import { useRewardPlayer } from "$lib/rewards/player.svelte";
  import { config } from "$lib/state/config.svelte";
  import { rewards, sessionSteps } from "$lib/state/rewards.svelte";
  import { session } from "$lib/state/session.svelte";
  import { work, workEnabled } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";

  const snap = $derived(session.snapshot);

  // Idle: preview the configured focus length (or the demo profile reported by the shell).
  const idleSecs = $derived.by(() => {
    if (snap.demo && snap.focusSecs > 0) return snap.focusSecs;
    const minutes = config.value?.focusMinutes;
    if (minutes !== undefined && Number.isFinite(minutes) && minutes > 0) return minutes * 60;
    return snap.focusSecs;
  });
  const remaining = $derived(session.isIdle ? idleSecs : snap.remainingSecs);
  const total = $derived(session.isIdle ? idleSecs : snap.phaseTotalSecs);
  // A developer-mode sample (ADR-019) shows its card even without a connected calendar / tool.
  const calendarOn = $derived(calendarEnabled(config.value) || calendar.sample !== null);
  const workOn = $derived(workEnabled(config.value) || work.sample !== null);
  /**
   * "Co dalej?" stays out of the focus phase unless the user turned off holding notifications during focus; a
   * developer-mode sample (ADR-019) shows in every phase, so the button always has something to capture.
   */
  const workDuringFocus = $derived(
    work.sample !== null || (workOn && config.value?.work.holdDuringFocus === false),
  );
  /** The axolotl stands at the hero's bottom-left: the centred content under the ring keeps clear of it. */
  const mascotOn = $derived(mascotEnabled(config.value));

  // Beta rewards (ADR-022): this page plays the Start / step / back-from-break moments and the break recap. With the
  // beta features off none of it renders.
  const player = useRewardPlayer("focus");
  const steps = $derived(rewards.stepsActive ? sessionSteps() : []);
  const rewardsView = $derived(rewards.active ? rewards.view : null);
  const showIntro = $derived((rewards.configActive || rewards.sampling) && !rewards.introSeen);
  /** Focus with small steps: the "Teraz:" line comes first under the buttons (it must stay above the music dock). */
  const stepsInFocus = $derived(snap.phase === "focus" && steps.length > 0);
  /** Break: the recap in one line under the buttons, in view at any window size (the card below has the details). */
  const recapMoment = $derived.by(() => {
    if (!rewardsView || snap.phase !== "break") return null;
    const m = player.burst?.moment;
    return m && (m.kind === "breakStart" || m.kind === "update") ? m : null;
  });

  let hero = $state<HTMLElement | null>(null);
  let controls = $state<HTMLElement | null>(null);
  let lane = $state<HTMLElement | null>(null);
  /**
   * How far the top of the session buttons is above the lane's bottom (the hero's last line), when the buttons reach
   * into the lane (three wide buttons, or two rows of them, in a narrow window): the bubble stays above them.
   */
  let buttonsTop = $state(0);

  $effect(() => {
    const box = hero;
    const row = controls;
    const area = lane;
    if (!box || !row || !area) return;
    const measure = () => {
      const r = row.getBoundingClientRect();
      const a = area.getBoundingClientRect();
      buttonsTop = r.left < a.right && r.right > a.left ? Math.max(0, Math.ceil(a.bottom - r.top)) : 0;
    };
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    observer.observe(row);
    measure();
    return () => observer.disconnect();
  });
</script>

<svelte:head>
  <title>{session.isIdle ? strings.appName : `${strings.phase[snap.phase]} · ${strings.appName}`}</title>
</svelte:head>

<h1 class="sr-only">{strings.nav.focus}</h1>

<section
  class="hero"
  class:with-mascot={mascotOn}
  class:steps-focus={stepsInFocus}
  style:--mascot-w="{MASCOT_WIDTH}px"
  bind:this={hero}
>
  <div class="ring-wrap">
    <FloatSpark {player} />
    {#key `${snap.phase}-${snap.cycle}`}
      <TimerRing
        remainingSecs={remaining}
        totalSecs={total}
        phase={snap.phase}
        paused={snap.paused}
        cycle={snap.cycle}
        demo={snap.demo}
      />
    {/key}
  </div>

  <div class="controls" role="group" aria-label={strings.controls.sessionControls} bind:this={controls}>
    {#if session.isIdle}
      <button
        type="button"
        class="btn btn-primary btn-lg start"
        disabled={session.busy || !config.ready}
        onclick={() => session.start()}
      >
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
          <path d="M7 4.5v15l12.5-7.5z" fill="currentColor" />
        </svg>
        {strings.controls.start}
      </button>
    {:else}
      <button
        type="button"
        class="btn btn-lg"
        class:btn-primary={snap.paused}
        disabled={session.busy}
        onclick={() => session.togglePause()}
      >
        {#if snap.paused}
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <path d="M7 4.5v15l12.5-7.5z" fill="currentColor" />
          </svg>
          {strings.controls.resume}
        {:else}
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <rect x="6" y="5" width="4" height="14" rx="1.2" fill="currentColor" />
            <rect x="14" y="5" width="4" height="14" rx="1.2" fill="currentColor" />
          </svg>
          {strings.controls.pause}
        {/if}
      </button>
      <button
        type="button"
        class="btn btn-lg"
        class:btn-break={snap.phase === "focus"}
        disabled={session.busy}
        onclick={() => session.skip()}
        title={strings.controls.skip}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
          <path d="M5 5l9 7-9 7z" fill="currentColor" />
          <rect x="15.5" y="5" width="3" height="14" rx="1.2" fill="currentColor" />
        </svg>
        {snap.phase === "focus" ? strings.controls.skipToBreak : strings.controls.skipToFocus}
      </button>
      <button type="button" class="btn btn-lg btn-danger" onclick={() => session.stop()}>
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
          <rect x="5" y="5" width="14" height="14" rx="2.5" fill="currentColor" />
        </svg>
        {strings.controls.stop}
      </button>
    {/if}
  </div>

  {#if session.isIdle}
    <p class="hint">{strings.timer.idleHint}</p>
  {:else}
    <p class="secondary small tabular">
      {strings.timer.focusedSoFar(formatDuration(snap.focusedSecs))}
      {#if rewardsView && snap.phase === "focus"}<SparkChip sparks={rewardsView.today.sparks} />{/if}
      {#if recapMoment}<RecapChip moment={recapMoment} />{/if}
    </p>
  {/if}

  {#if stepsInFocus}
    <NextStep {player} />
  {/if}

  <SessionGoalLine />

  <!-- Last in the hero, so Tab reaches Start (and the session buttons) before the axolotl. -->
  <div class="mascot-lane" bind:this={lane}>
    <FocusMascot bubbleFloor={buttonsTop > 0 ? buttonsTop + 10 : 0} />
  </div>
</section>

<MeetingHoldBanner />

<!-- One sentence per reward moment (beta); empty while nothing plays. -->
<p class="sr-only" role="status" aria-live="polite">{player.announcement}</p>

{#if session.isIdle}
  {#if showIntro}
    <div class="above">
      <RewardsIntro />
    </div>
  {/if}
  <RewardBurst {player} />
  {#if rewardsView}
    <div class="above">
      <TodayStrip status={rewardsView} />
    </div>
  {/if}
  {#if rewards.stepsActive}
    <div class="above">
      <StepsCard mode="idle" />
    </div>
  {/if}
  {#if calendarOn}
    <div class="above">
      <MeetingConflicts />
    </div>
  {/if}
  {#if workOn}
    <div class="above">
      <NextUp />
    </div>
  {/if}
  <div class="grid">
    <TimingEditor />
    <MusicSourcePicker />
  </div>
  <div class="below">
    <GoodPractice />
    {#if config.ready && !calendarOn}
      <p class="hint calendar-hint">
        {strings.calendar.connectHint}
        <a href="/settings#calendar-settings">{strings.calendar.connectLink}</a>
      </p>
    {/if}
    {#if config.ready && !workOn}
      <!-- Without a connected tool, "Co dalej?" is just a hint with the link to Settings. -->
      <NextUp />
    {/if}
  </div>
{:else if snap.phase === "focus"}
  <RewardBurst {player} />
  <div class="grid">
    <LiveUsage />
    <ShieldChips />
  </div>
  {#if calendarOn}
    <div class="below">
      <MeetingConflicts />
    </div>
  {/if}
  {#if workDuringFocus}
    <div class="below">
      <NextUp />
    </div>
  {/if}
{:else}
  <RewardBurst {player} />
  <BreakCard />
  {#if steps.length > 0}
    <div class="below">
      <StepsCard mode="break" />
    </div>
  {/if}
  {#if workOn}
    <div class="below">
      <NextUp />
    </div>
  {/if}
  {#if calendarOn}
    <div class="below">
      <MeetingConflicts />
    </div>
  {/if}
{/if}

<style>
  .hero {
    /* Bottom padding; the axolotl stands on the line above it (the last line of the hero's content). */
    --hero-pad-bottom: 28px;
    /* Half the timer ring (TimerRing: 300 px wide from the minimum window width up). */
    --ring-half: 150px;
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    padding: 8px 0 var(--hero-pad-bottom);
    text-align: center;
  }

  /* Focus with small steps (beta): a little tighter, so the "Teraz:" line clears the music dock at 960 × 640. */
  .hero.steps-focus {
    gap: 12px;
  }

  /*
   * The axolotl's column (FocusMascot's width + a gap) on both sides, so the session buttons, the hint and the goal
   * line under the ring stay centred and never reach under the axolotl; a long row wraps instead.
   */
  .hero.with-mascot {
    padding-inline: calc(var(--mascot-w) + 14px);
  }

  /*
   * The axolotl's lane (FocusMascot fills it), outside the flow: at the page's left content edge (the cards' left
   * edge), from the hero's top to its last line, and narrower than the room left of the ring (a gap before the ring,
   * at most 300 px wide). The axolotl stands at its bottom-left, the bubble grows upwards above it (and above the
   * session buttons where they reach into the lane: `buttonsTop`).
   */
  .mascot-lane {
    position: absolute;
    z-index: 1;
    top: 0;
    bottom: var(--hero-pad-bottom);
    left: 0;
    width: min(300px, calc(50% - var(--ring-half) - 20px));
    pointer-events: none;
  }

  /* The "+12 ✦" float of a reward moment is placed over the ring. */
  .ring-wrap {
    position: relative;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px;
  }

  .start {
    min-width: 200px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 340px), 1fr));
    gap: 16px;
    align-items: start;
  }

  .above {
    margin-bottom: 16px;
  }

  .below {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 16px;
  }

  .calendar-hint {
    text-align: center;
  }

  .calendar-hint a {
    color: var(--accent-strong);
    font-weight: 600;
  }
</style>
