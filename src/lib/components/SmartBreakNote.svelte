<script lang="ts">
  // Skupienie page, under the timer (ADR-023): the break after this focus block was moved around a meeting, with
  // "Ask me instead". Idle, the first break of a session that would start now (the calendar's plan); during the moved
  // break itself a line without the undo. Times only: the meeting's subject stays in the meetings card.
  import { formatTimeNear } from "$lib/format";
  import { calendar, calendarEnabled } from "$lib/state/calendar.svelte";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";
  import type { SmartBreak } from "$lib/types";

  const s = $derived(strings.calendar.smart);
  const phase = $derived(session.snapshot.phase);

  const moved = $derived.by((): SmartBreak | null => {
    if (session.smartBreak) return session.smartBreak;
    const status = calendar.status;
    if (phase !== "idle" || !status?.planned) return null;
    // The switch was just turned off (or the calendars): hide the plan's line at once, before the shell re-plans.
    const on = config.value?.smartBreaks === true && calendarEnabled(config.value);
    if (!on && calendar.sample === null) return null;
    return status.smartBreaks.find((b) => b.breakNumber === 1) ?? null;
  });
  /** The running break is the moved one (a sample always shows the focus line). */
  const running = $derived(phase === "break" && session.sampleSmartBreak === null);
  const meetingTime = $derived(moved ? formatTimeNear(moved.meetingStartMs) : "");
  const breakTime = $derived(moved ? formatTimeNear(moved.breakStartMs) : "");
  const busy = $derived(moved !== null && moved.meetingId in calendar.smartPending);

  /** "Ask me instead": this meeting gets the usual question, the break goes back where it was. */
  function askMe(): void {
    if (moved) void calendar.setSmartOptOut(moved.meetingId, true);
  }
</script>

{#if moved}
  <div class="smart-note small">
    <svg class="icon" viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
      <rect x="3.5" y="5" width="17" height="15" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.8" />
      <path d="M3.5 9.5h17M8 3v4M16 3v4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      <path d="M9 15h6m-2-2 2 2-2 2" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    </svg>
    <p class="text tabular" role="status">
      {running ? s.runningNote(meetingTime) : s.movedNote(breakTime, meetingTime)}
    </p>
    {#if !running}
      <button
        type="button"
        class="ask"
        disabled={busy}
        aria-label={s.askMeAria(meetingTime)}
        onclick={askMe}
      >
        {s.askMe}
      </button>
    {/if}
  </div>
{/if}

<style>
  .smart-note {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 4px 10px;
    max-width: 100%;
    color: var(--text-2);
  }

  .icon {
    flex: none;
    color: var(--accent-strong);
  }

  .text {
    margin: 0;
  }

  .ask {
    /* WCAG 2.2 target size: at least 24 px tall, though it reads as a link. */
    min-height: 24px;
    padding: 2px 4px;
    border: 0;
    border-radius: var(--radius-sm, 6px);
    background: none;
    color: var(--accent-strong);
    font: inherit;
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }

  .ask:hover:not(:disabled) {
    color: var(--text);
  }

  .ask:disabled {
    opacity: 0.6;
    cursor: progress;
  }
</style>
