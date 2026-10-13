<script lang="ts">
  // Settings → Przypomnienia w przerwach: an ordered list (1st break, 2nd break, …) of the user's own reminders,
  // shown on the break card and in the break popup. Saved through the config store (typing debounced).
  import { tick, untrack } from "svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { charCount, config, LIMITS } from "$lib/state/config.svelte";
  import { strings } from "$lib/strings";

  const s = strings.settings;
  const MAX_COUNT = LIMITS.reminders.maxCount;
  const MAX_CHARS = LIMITS.reminders.maxChars;
  /** The character counter appears only near the limit. */
  const COUNTER_FROM = MAX_CHARS - 30;

  const uid = $props.id();

  const cfg = $derived(config.value);
  const error = $derived(config.fieldErrors.breakReminders ?? null);

  // Drafts keep the text of the input being edited: the shell trims entries when it saves, which must not eat a
  // space typed right before a pause. Every other entry follows the config.
  let drafts = $state<string[]>([]);
  let editing = $state<number | null>(null);
  /** Polite announcement after moving or removing an entry (the list itself changes silently). */
  let announcement = $state("");
  let addButton = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    const list = [...(config.value?.breakReminders ?? [])];
    const focused = editing;
    untrack(() => {
      drafts = list.map((text, i) => (i === focused ? (drafts[i] ?? text) : text));
    });
  });

  const full = $derived(drafts.length >= MAX_COUNT);

  function placeholder(index: number): string {
    return s.reminderPlaceholders[index] ?? s.reminderPlaceholderOther;
  }

  function inputId(index: number): string {
    return `${uid}-reminder-${index}`;
  }

  function onInput(index: number, event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.value;
    drafts[index] = value;
    config.update((c) => {
      if (index < c.breakReminders.length) c.breakReminders[index] = value;
    });
  }

  async function add(): Promise<void> {
    if (!cfg || cfg.breakReminders.length >= MAX_COUNT) return;
    const index = cfg.breakReminders.length;
    config.update((c) => {
      c.breakReminders.push("");
    }, true);
    await tick();
    document.getElementById(inputId(index))?.focus();
  }

  async function removeAt(index: number): Promise<void> {
    config.update((c) => {
      c.breakReminders.splice(index, 1);
    }, true);
    announcement = s.reminderRemoved;
    await tick();
    const remaining = config.value?.breakReminders.length ?? 0;
    const next = remaining > 0 ? document.getElementById(inputId(Math.min(index, remaining - 1))) : null;
    (next ?? addButton)?.focus();
  }

  async function move(index: number, delta: -1 | 1): Promise<void> {
    const target = index + delta;
    const length = config.value?.breakReminders.length ?? 0;
    if (target < 0 || target >= length) return;
    config.update((c) => {
      const [item] = c.breakReminders.splice(index, 1);
      c.breakReminders.splice(target, 0, item ?? "");
    }, true);
    announcement = s.reminderMoved(target + 1);
    await tick();
    // Keep the focus on the same arrow at the entry's new place (or the other arrow at the end of the list).
    const same = document.getElementById(`${uid}-${delta < 0 ? "up" : "down"}-${target}`);
    const other = document.getElementById(`${uid}-${delta < 0 ? "down" : "up"}-${target}`);
    (same instanceof HTMLButtonElement && !same.disabled ? same : other)?.focus();
  }
</script>

{#if cfg}
  <div class="reminders">
    <p class="hint">{s.remindersLead}</p>

    {#if drafts.length === 0}
      <p class="empty">{s.remindersEmpty}</p>
    {:else}
      <ol class="list">
        {#each drafts as draft, i (i)}
          {@const used = charCount(draft)}
          <li class="entry">
            <label class="num" for={inputId(i)}>{s.reminderLabel(i + 1)}</label>
            <div class="input-wrap">
              <input
                id={inputId(i)}
                class="input"
                type="text"
                autocomplete="off"
                maxlength={MAX_CHARS}
                placeholder={placeholder(i)}
                value={draft}
                aria-describedby={used > COUNTER_FROM ? `${uid}-count-${i}` : undefined}
                onfocus={() => (editing = i)}
                onblur={() => (editing = null)}
                oninput={(e) => onInput(i, e)}
              />
              {#if used > COUNTER_FROM}
                <span class="count tabular" id="{uid}-count-{i}">
                  <span aria-hidden="true">{s.reminderChars(used, MAX_CHARS)}</span>
                  <span class="sr-only">{s.reminderCharsAria(used, MAX_CHARS)}</span>
                </span>
              {/if}
            </div>
            <div class="tools">
              <button
                type="button"
                class="tool"
                id="{uid}-up-{i}"
                disabled={i === 0}
                aria-label={s.reminderUp(i + 1)}
                title={s.reminderUp(i + 1)}
                onclick={() => move(i, -1)}
              >
                <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
                  <path d="M4 10l4-4 4 4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
                </svg>
              </button>
              <button
                type="button"
                class="tool"
                id="{uid}-down-{i}"
                disabled={i === drafts.length - 1}
                aria-label={s.reminderDown(i + 1)}
                title={s.reminderDown(i + 1)}
                onclick={() => move(i, 1)}
              >
                <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
                  <path d="M4 6l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
                </svg>
              </button>
              <button
                type="button"
                class="tool remove"
                aria-label={s.reminderRemove(i + 1)}
                title={s.reminderRemove(i + 1)}
                onclick={() => removeAt(i)}
              >
                <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
                  <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
                </svg>
              </button>
            </div>
          </li>
        {/each}
      </ol>
    {/if}

    <div class="row">
      <button bind:this={addButton} type="button" class="btn btn-sm" disabled={full} onclick={add}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
        {s.reminderAdd}
      </button>
      {#if full}<span class="hint">{s.remindersFull(MAX_COUNT)}</span>{/if}
    </div>
    {#if error}<p class="field-error">{error}</p>{/if}

    {#if drafts.length > 0}
      <div class="repeat">
        <Switch
          id="{uid}-repeat"
          label={s.remindersRepeat}
          hint={s.remindersRepeatHint}
          checked={cfg.repeatReminders}
          onchange={(on) => config.update((c) => (c.repeatReminders = on), true)}
        />
      </div>
    {/if}

    <p class="sr-only" aria-live="polite">{announcement}</p>
  </div>
{/if}

<style>
  .reminders {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty {
    color: var(--text-2);
    font-size: 0.92rem;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .entry {
    display: grid;
    grid-template-columns: 6.5rem minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
  }

  .num {
    font-size: 0.86rem;
    font-weight: 600;
    color: var(--text-2);
    white-space: nowrap;
  }

  .input-wrap {
    position: relative;
    min-width: 0;
  }

  .count {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.75rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  .input-wrap:has(.count) .input {
    padding-right: 64px;
  }

  .tools {
    display: flex;
    gap: 4px;
  }

  .tool {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--surface-inset);
    color: var(--text-2);
    cursor: pointer;
  }

  .tool:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--border-strong);
  }

  .tool.remove:hover:not(:disabled) {
    color: var(--danger);
    border-color: var(--danger);
  }

  .tool:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .repeat {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  @media (max-width: 520px) {
    .entry {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .num {
      grid-column: 1 / -1;
    }
  }
</style>
