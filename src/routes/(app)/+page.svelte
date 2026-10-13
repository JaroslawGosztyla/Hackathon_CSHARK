<script lang="ts">
  import BreakCard from "$lib/components/BreakCard.svelte";
  import GoodPractice from "$lib/components/GoodPractice.svelte";
  import LiveUsage from "$lib/components/LiveUsage.svelte";
  import MeetingConflicts from "$lib/components/MeetingConflicts.svelte";
  import MeetingHoldBanner from "$lib/components/MeetingHoldBanner.svelte";
  import MusicSourcePicker from "$lib/components/MusicSourcePicker.svelte";
  import NextUp from "$lib/components/NextUp.svelte";
  import SessionGoalLine from "$lib/components/SessionGoalLine.svelte";
  import ShieldChips from "$lib/components/ShieldChips.svelte";
  import TimerRing from "$lib/components/TimerRing.svelte";
  import TimingEditor from "$lib/components/TimingEditor.svelte";
  import { formatDuration } from "$lib/format";
  import FocusMascot from "$lib/mascot/FocusMascot.svelte";
  import { calendarEnabled } from "$lib/state/calendar.svelte";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { workEnabled } from "$lib/state/work.svelte";
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
  const calendarOn = $derived(calendarEnabled(config.value));
  const workOn = $derived(workEnabled(config.value));
  /** "Co dalej?" stays out of the focus phase unless the user turned off holding notifications during focus. */
  const workDuringFocus = $derived(workOn && config.value?.work.holdDuringFocus === false);
</script>

<svelte:head>
  <title>{session.isIdle ? strings.appName : `${strings.phase[snap.phase]} · ${strings.appName}`}</title>
</svelte:head>

<h1 class="sr-only">{strings.nav.focus}</h1>

<section class="hero">
  <div class="ring-wrap">
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
    <FocusMascot />
  </div>

  <div class="controls" role="group" aria-label={strings.controls.sessionControls}>
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
    <p class="secondary small tabular">{strings.timer.focusedSoFar(formatDuration(snap.focusedSecs))}</p>
  {/if}

  <SessionGoalLine />
</section>

<MeetingHoldBanner />

{#if session.isIdle}
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
  <BreakCard />
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
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    padding: 8px 0 28px;
    text-align: center;
  }

  /* The axolotl (FocusMascot) is placed against the ring's box, outside the flow. */
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

  @media (max-width: 560px) {
    .controls :global(.btn-lg) {
      min-height: 46px;
      padding: 10px 18px;
      font-size: 1rem;
    }
  }
</style>
