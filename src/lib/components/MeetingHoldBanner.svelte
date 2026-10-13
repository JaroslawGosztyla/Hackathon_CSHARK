<script lang="ts">
  // Skupienie page, running session: a meeting extends the focus (music muted) or holds the break until Resume.
  import { formatTimeNear } from "$lib/format";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";

  const c = strings.calendar;

  const hold = $derived(session.isIdle ? null : session.snapshot.meetingHold);
  const subject = $derived(hold ? hold.subject.trim() || c.untitled : "");
  const until = $derived(hold ? formatTimeNear(hold.untilMs) : "");
  const extended = $derived(hold?.choice === "extend");
</script>

{#if hold}
  <div class="hold" class:held-break={!extended} role="status">
    <span class="icon" aria-hidden="true">
      {#if extended}
        <svg viewBox="0 0 24 24" width="22" height="22">
          <rect x="3.5" y="5" width="17" height="15" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.8" />
          <path d="M3.5 9.5h17M8 3v4M16 3v4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          <path d="M12 12.5v3l2 1.2" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
        </svg>
      {:else}
        <svg viewBox="0 0 24 24" width="22" height="22">
          <rect x="6" y="5" width="4" height="14" rx="1.2" fill="currentColor" />
          <rect x="14" y="5" width="4" height="14" rx="1.2" fill="currentColor" />
        </svg>
      {/if}
    </span>
    <div class="text">
      <p class="title">{extended ? c.holdExtend(until, subject) : c.holdPause(subject)}</p>
      <p class="hint">{extended ? c.holdExtendHint : c.holdPauseHint(until)}</p>
    </div>
  </div>
{/if}

<style>
  .hold {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 auto 16px;
    padding: 12px 16px;
    border-radius: var(--radius);
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--accent) 20%, transparent),
      color-mix(in srgb, var(--accent) 6%, transparent)
    );
    border: var(--border-w) solid var(--accent-line);
  }

  .hold.held-break {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--break) 20%, transparent),
      color-mix(in srgb, var(--break) 6%, transparent)
    );
    border-color: var(--break-line);
  }

  .icon {
    display: grid;
    flex: none;
    color: var(--accent-strong);
  }

  .held-break .icon {
    color: var(--break);
  }

  .text {
    min-width: 0;
  }

  .title {
    font-weight: 650;
    color: var(--text);
    overflow-wrap: anywhere;
  }
</style>
