<script lang="ts">
  // Skupienie page: meetings that fall into a (planned) break, each with a choice, plus the next few meetings.
  // Idle, the breaks are those of a session that would start now, so the user decides before pressing Start.
  import { formatTimeNear, formatTimeOfDay, formatTimeRange } from "$lib/format";
  import { calendar } from "$lib/state/calendar.svelte";
  import { config } from "$lib/state/config.svelte";
  import { strings } from "$lib/strings";
  import type { MeetingChoice, MeetingConflict } from "$lib/types";

  /** Order of the buttons. "Przypomnij później" is offered only while a later reminder is still possible. */
  const CHOICES: readonly MeetingChoice[] = ["extend", "pauseAtBreak", "ignore", "remindLater"];
  const UPCOMING_COUNT = 3;
  /** The list of upcoming meetings drops finished ones on this beat. */
  const CLOCK_MS = 30_000;
  /** Planned breaks of an idle session move with the clock: read the status again now and then. */
  const RELOAD_MS = 60_000;

  const c = strings.calendar;

  let now = $state(Date.now());

  $effect(() => {
    const clock = setInterval(() => (now = Date.now()), CLOCK_MS);
    const reload = setInterval(() => void calendar.reload(), RELOAD_MS);
    return () => {
      clearInterval(clock);
      clearInterval(reload);
    };
  });

  const status = $derived(calendar.status);
  const conflicts = $derived(status?.conflicts ?? []);
  const planned = $derived(status?.planned ?? true);
  const horizonHours = $derived(config.value?.calendar.horizonHours ?? 0);

  const upcoming = $derived(
    (status?.meetings ?? [])
      .filter((m) => m.endMs > now)
      .sort((a, b) => a.startMs - b.startMs)
      .slice(0, UPCOMING_COUNT),
  );
  const conflictIds = $derived(new Set(conflicts.map((x) => x.meeting.id)));

  /** Enabled sources that report an error, by name. */
  const failing = $derived(
    (status?.sources ?? [])
      .filter((s) => s.state === "error")
      .map((s) => c.sourceName[s.source])
      .join(", "),
  );

  function subjectOf(subject: string): string {
    return subject.trim() || c.untitled;
  }

  function shownChoice(conflict: MeetingConflict): MeetingChoice | null {
    return calendar.pending[conflict.meeting.id] ?? conflict.choice;
  }

  function choiceText(conflict: MeetingConflict, choice: MeetingChoice | null): string {
    // An earlier "extend" / "pause" for a meeting that is now longer than 4 h is ignored by the session.
    if (choice !== null && holdsSession(choice) && conflict.tooLong === true) return c.choiceIgnore;
    switch (choice) {
      case "extend":
        return c.choiceExtend(formatTimeNear(conflict.meeting.endMs, now));
      case "pauseAtBreak":
        return c.choicePauseAtBreak;
      case "ignore":
        return c.choiceIgnore;
      case "remindLater":
        return c.choiceRemindLater(conflict.nextReminderMs !== null ? formatTimeNear(conflict.nextReminderMs, now) : null);
      case null:
        return c.choiceNone;
    }
  }

  function offered(conflict: MeetingConflict, choice: MeetingChoice): boolean {
    return choice !== "remindLater" || conflict.remindLaterAllowed || conflict.choice === "remindLater";
  }

  /** "Extend" and "pause at break" need a meeting of at most 4 h (`tooLong`: the session would ignore them). */
  function holdsSession(choice: MeetingChoice): boolean {
    return choice === "extend" || choice === "pauseAtBreak";
  }

  function choiceDisabled(conflict: MeetingConflict, choice: MeetingChoice): boolean {
    return (
      (choice === "remindLater" && !conflict.remindLaterAllowed) || (holdsSession(choice) && conflict.tooLong === true)
    );
  }
</script>

<section class="card meetings" aria-labelledby="meetings-title">
  <div class="card-title">
    <h2 id="meetings-title">{c.cardTitle}</h2>
    <div class="sync">
      {#if status?.lastSyncMs != null}
        <span class="hint tabular">{c.lastSync(formatTimeOfDay(status.lastSyncMs))}</span>
      {/if}
      <button
        type="button"
        class="icon-btn small-btn"
        disabled={calendar.refreshing}
        aria-label={calendar.refreshing ? c.refreshing : c.refresh}
        title={calendar.refreshing ? c.refreshing : c.refresh}
        onclick={() => {
          // A developer-mode sample is shown: no calendar is contacted for it (ADR-019).
          if (calendar.sample === null) void calendar.refresh();
        }}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" class:spin={calendar.refreshing}>
          <path
            d="M20 12a8 8 0 1 1-2.34-5.66M20 4v4.5h-4.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.9"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>
  </div>

  {#if !status}
    <p class="hint">{c.loading}</p>
  {:else}
    {#if failing}
      <p class="warn small">
        {c.sourceErrors(failing)}
        <a href="/settings#calendar-settings">{c.sourceErrorsLink}</a>
      </p>
    {/if}

    {#if conflicts.length === 0}
      <p class="secondary">{planned ? c.noConflictsPlanned : c.noConflictsRunning}</p>
    {:else}
      <p class="secondary">{planned ? c.leadPlanned : c.leadRunning}</p>
      <ul class="conflicts">
        {#each conflicts as conflict, i (conflict.meeting.id)}
          {@const subject = subjectOf(conflict.meeting.subject)}
          {@const current = shownChoice(conflict)}
          {@const busy = conflict.meeting.id in calendar.pending}
          <li class="conflict" class:undecided={current === null}>
            <div class="head">
              <span class="time tabular">{formatTimeRange(conflict.meeting.startMs, conflict.meeting.endMs, now)}</span>
              <span class="subject" title={subject}>{subject}</span>
              {#if conflict.meeting.online}
                <span class="badge" title={c.onlineAria}>{c.online}</span>
              {/if}
              <span class="source muted small">{c.sourceShort[conflict.meeting.source]}</span>
            </div>
            <p class="hint tabular">
              {c.breakInfo(conflict.breakNumber, formatTimeRange(conflict.breakStartMs, conflict.breakEndMs, now))}
            </p>
            {#if conflict.tooLong}
              <p class="small too-long" id="meeting-too-long-{i}">{c.tooLong}</p>
            {/if}
            <div class="choices" role="group" aria-label={c.choiceGroup(subject)}>
              {#each CHOICES as choice (choice)}
                {#if offered(conflict, choice)}
                  <button
                    type="button"
                    class="btn btn-sm choice"
                    aria-pressed={current === choice}
                    disabled={busy || choiceDisabled(conflict, choice)}
                    aria-describedby={conflict.tooLong && holdsSession(choice)
                      ? `meeting-too-long-${i}`
                      : undefined}
                    onclick={() => calendar.choose(conflict.meeting.id, choice)}
                  >
                    {c.choice[choice]}
                  </button>
                {/if}
              {/each}
            </div>
            <p class="small choice-text" class:muted={current === null} aria-live="polite">
              {choiceText(conflict, current)}
            </p>
          </li>
        {/each}
      </ul>
    {/if}

    <div class="upcoming">
      <h3 class="label">{c.upcomingTitle}</h3>
      {#if upcoming.length === 0}
        <p class="hint">{c.upcomingEmpty(horizonHours)}</p>
      {:else}
        <ul>
          {#each upcoming as meeting (meeting.id)}
            {@const live = meeting.startMs <= now}
            <li class:collides={conflictIds.has(meeting.id)}>
              <span class="time tabular">
                {#if live}<span class="now">{c.now}</span>{/if}
                {formatTimeRange(meeting.startMs, meeting.endMs, now)}
              </span>
              <span class="subject" title={subjectOf(meeting.subject)}>{subjectOf(meeting.subject)}</span>
              {#if meeting.online}<span class="badge" title={c.onlineAria}>{c.online}</span>{/if}
              <span class="source muted small">{c.sourceShort[meeting.source]}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</section>

<style>
  .meetings {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card-title {
    margin-bottom: 0;
    align-items: center;
  }

  .sync {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .small-btn {
    width: 32px;
    height: 32px;
  }

  .spin {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .warn {
    color: var(--danger);
  }

  .warn a {
    color: inherit;
    font-weight: 600;
  }

  .conflicts {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .conflict {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
    border: var(--border-w) solid var(--border);
  }

  .conflict.undecided {
    border-color: var(--accent-line);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
    flex-wrap: wrap;
  }

  .time {
    font-weight: 700;
    color: var(--text);
    white-space: nowrap;
  }

  .subject {
    flex: 1 1 160px;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source {
    white-space: nowrap;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }

  .choice {
    white-space: normal;
    text-align: center;
  }

  .choice[aria-pressed="true"] {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }

  .choice[aria-pressed="true"]:hover:not(:disabled) {
    background: var(--accent-strong);
    border-color: var(--accent-strong);
  }

  .choice-text {
    color: var(--text-2);
  }

  .too-long {
    color: var(--text-2);
    font-weight: 600;
  }

  .upcoming {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 12px;
    border-top: var(--border-w) solid var(--border);
  }

  .label {
    font-size: 0.86rem;
    font-weight: 600;
    color: var(--text-2);
  }

  .upcoming ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .upcoming li {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
    padding: 4px 0 4px 10px;
    border-left: 3px solid transparent;
  }

  .upcoming li.collides {
    border-left-color: var(--accent);
  }

  .upcoming .time {
    font-weight: 600;
    color: var(--text-2);
  }

  .upcoming .subject {
    font-weight: 500;
  }

  .now {
    margin-right: 4px;
    color: var(--break);
    font-weight: 700;
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
