<script lang="ts">
  // Under the timer: the item the user focuses on in this session (picked in "Co dalej?"). The name opens the item in
  // the browser; "×" clears the goal. Shown in every phase, also while idle (the goal survives Start).
  import { session } from "$lib/state/session.svelte";
  import { isOpenableUrl, work } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";

  const w = strings.work;

  const goal = $derived(session.snapshot.goal ?? null);
  const name = $derived(goal ? w.itemName(goal.key, goal.title) : "");
  const openable = $derived(goal !== null && isOpenableUrl(goal.url));
</script>

{#if goal}
  <div class="goal" aria-live="polite">
    <span class="label">{w.goalLabel}</span>
    <span class="source badge">{w.sourceShort[goal.source]}</span>
    {#if openable}
      <button type="button" class="name link" title={name} aria-label={w.goalOpenAria(name)} onclick={() => work.open(goal.url)}>
        {#if goal.key}<span class="key">{goal.key}</span>{/if}
        <span class="title">{goal.title}</span>
      </button>
    {:else}
      <span class="name" title={name}>
        {#if goal.key}<span class="key">{goal.key}</span>{/if}
        <span class="title">{goal.title}</span>
      </span>
    {/if}
    <button
      type="button"
      class="clear"
      aria-label={w.goalClear}
      title={w.goalClear}
      disabled={work.goalBusy}
      onclick={() => work.setGoal(null)}
    >
      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
        <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
    </button>
  </div>
{/if}

<style>
  .goal {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: min(100%, 640px);
    padding: 4px 6px 4px 14px;
    border-radius: var(--radius-pill);
    border: var(--border-w) solid var(--border-strong);
    background: var(--surface-inset);
    color: var(--text-2);
    font-size: 0.9rem;
  }

  .label {
    flex: none;
    color: var(--text-muted);
  }

  .source {
    flex: none;
  }

  .name {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    color: var(--text);
    font-weight: 600;
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

  .link:hover .title,
  .link:focus-visible .title {
    text-decoration: underline;
  }

  .link:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: 4px;
  }

  .key {
    flex: none;
    font-family: var(--font-mono);
    font-size: 0.84rem;
    color: var(--accent-strong);
  }

  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .clear {
    flex: none;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .clear:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface-hover);
  }

  .clear:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .clear:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
</style>
