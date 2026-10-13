<script lang="ts">
  // Small steps (beta, ADR-022), at most 5 per session of at most 80 characters.
  //   idle    — the plan of the coming session: add (Enter), remove (×), idea chips; ticking starts after Start.
  //   break   — dimmed under the break card, "Po przerwie: <next step>", still editable and tickable.
  //   summary — the steps of a finished session with their ticks; the open ones still in the coming plan "wait for the
  //             next session" (they carry over by themselves; "Wyczyść" drops the carried plan while idle). Open steps
  //             that no longer carry over (cleared, an older entry, after a restart) are only listed, with no promise.
  // Step texts are the user's own: shown here, never logged.
  import { untrack } from "svelte";
  import { MAX_STEP_CHARS, MAX_STEPS } from "$lib/rewards/choreography";
  import { nextOpenStep, rewards, sessionSteps } from "$lib/state/rewards.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";
  import type { MicroStep, StepDraft } from "$lib/types";
  import BetaBadge from "./BetaBadge.svelte";

  interface Props {
    mode: "idle" | "break" | "summary";
    /** Summary mode: the steps of the shown session. */
    steps?: readonly MicroStep[];
    /** Summary mode: the shown session is the last one (the carried plan belongs to it). */
    latest?: boolean;
  }

  let { mode, steps: summarySteps = [], latest = false }: Props = $props();

  const uid = $props.id();
  const s = strings.rewards;

  const live = $derived(sessionSteps());
  const steps = $derived(mode === "summary" ? summarySteps : live);
  const next = $derived(nextOpenStep(live));
  const full = $derived(live.length >= MAX_STEPS);
  const goal = $derived(session.goal);
  const goalName = $derived(goal ? strings.work.itemName(goal.key, goal.title) : null);

  let text = $state("");
  let busy = $state(false);
  let focused = $state(false);
  let input = $state<HTMLInputElement | null>(null);

  // The axolotl thinks along while the first step is being planned (idle, editor focused, nothing written yet).
  $effect(() => {
    const planning = mode === "idle" && focused && live.length === 0 && text.trim() === "";
    untrack(() => (rewards.planning = planning));
  });
  $effect(() => () => {
    if (mode === "idle") rewards.planning = false;
  });

  function drafts(list: readonly MicroStep[]): StepDraft[] {
    return list.map((step) => ({ id: step.id, text: step.text }));
  }

  async function save(next: StepDraft[]): Promise<boolean> {
    if (busy) return false;
    busy = true;
    try {
      return await rewards.setSteps(next);
    } finally {
      busy = false;
    }
  }

  async function add(): Promise<void> {
    const value = text.trim();
    if (value === "" || full) return;
    if (await save([...drafts(live), { id: null, text: value }])) text = "";
    input?.focus();
  }

  function remove(id: number): void {
    void save(drafts(live.filter((step) => step.id !== id)));
  }

  function clearCarried(): void {
    void save([]);
  }

  function useIdea(idea: string): void {
    text = idea;
    input?.focus();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "Enter" || event.isComposing) return;
    event.preventDefault();
    void add();
  }

  async function tick(step: MicroStep, done: boolean): Promise<void> {
    await rewards.setStepDone(step.id, done);
  }

  const title = $derived.by(() => {
    if (mode === "break") return next ? `${s.afterBreak} ${next.text}` : s.stepsTitle;
    if (mode === "summary") return s.stepsHeading;
    return goalName ? s.stepsFor(goalName) : s.stepsTitle;
  });

  const done = $derived(steps.filter((step) => step.done));
  const open = $derived(steps.filter((step) => !step.done));
  /** Summary: the open steps are still the plan of the coming session. */
  const carried = $derived(mode === "summary" && latest && session.isIdle && live.length > 0);
  /** Summary: the open steps that really wait in the coming plan (the same step, still open), and the others. */
  const waitingSteps = $derived(
    carried ? open.filter((step) => live.some((l) => l.id === step.id && l.text === step.text && !l.done)) : [],
  );
  const otherOpen = $derived(open.filter((step) => !waitingSteps.includes(step)));
</script>

{#if mode === "summary"}
  <div class="steps summary" aria-labelledby="{uid}-title">
    <p class="label" id="{uid}-title">
      {title}
      <span class="tabular count">{s.progress(done.length, steps.length)}</span>
    </p>
    <ul class="list">
      {#each done as step (step.id)}
        <li class="item is-done">
          <span class="mark" aria-hidden="true">✓</span>
          <span class="text"><span class="sr-only">{s.stepDoneAria("")}</span>{step.text}</span>
        </li>
      {/each}
    </ul>
    {#if otherOpen.length > 0}
      <ul class="list">
        {#each otherOpen as step (step.id)}
          <li class="item">
            <span class="mark open" aria-hidden="true"></span>
            <span class="text">{step.text}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if waitingSteps.length > 0}
      <p class="waiting">
        {s.waiting}
        <button type="button" class="btn btn-sm btn-ghost" disabled={busy} onclick={clearCarried}>{s.waitingClear}</button>
      </p>
      <ul class="list">
        {#each waitingSteps as step (step.id)}
          <li class="item">
            <span class="mark open" aria-hidden="true"></span>
            <span class="text">{step.text}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{:else}
  <section class="card steps mode-{mode}" aria-labelledby="{uid}-title">
    <div class="head">
      <h2 class="title" id="{uid}-title" title={title}>{title}</h2>
      <BetaBadge />
    </div>

    {#if steps.length > 0}
      <ul class="list">
        {#each steps as step (step.id)}
          <li class="item" class:is-done={step.done}>
            {#if mode === "break"}
              <input
                type="checkbox"
                class="check"
                checked={step.done}
                aria-label={step.text}
                onchange={(e) => tick(step, e.currentTarget.checked)}
              />
            {:else}
              <span class="mark open" aria-hidden="true"></span>
            {/if}
            <span class="text">{step.text}</span>
            <button
              type="button"
              class="icon-btn remove"
              aria-label={s.stepRemove(step.text)}
              title={s.stepRemove(step.text)}
              disabled={busy}
              onclick={() => remove(step.id)}
            >
              <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
                <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {:else if mode === "idle" && goalName}
      <button type="button" class="btn btn-sm" onclick={() => input?.focus()}>{s.breakItDown}</button>
    {/if}

    {#if full}
      <p class="hint">{s.stepsLimit(MAX_STEPS)}</p>
    {:else}
      <div class="editor">
        <label class:sr-only={live.length > 0} for="{uid}-input">{s.stepsPrompt}</label>
        <div class="row">
          <input
            id="{uid}-input"
            class="input"
            type="text"
            maxlength={MAX_STEP_CHARS}
            placeholder={s.stepPlaceholder}
            autocomplete="off"
            bind:value={text}
            bind:this={input}
            onkeydown={onKeydown}
            onfocus={() => (focused = true)}
            onblur={() => (focused = false)}
          />
          <button type="button" class="btn" disabled={busy || text.trim() === ""} onclick={add}>{s.stepAdd}</button>
        </div>
        {#if live.length === 0 && mode === "idle"}
          <div class="ideas" role="group" aria-label={s.stepsIdeasLabel}>
            {#each s.stepsIdeas as idea (idea)}
              <button type="button" class="chip idea" onclick={() => useIdea(idea)}>{idea}</button>
            {/each}
          </div>
          <p class="hint">{s.stepsHint}</p>
        {/if}
      </div>
    {/if}

    {#if mode === "idle"}
      <p class="hint note">{s.stepsIdleNote}</p>
    {/if}
  </section>
{/if}

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .mode-break {
    opacity: 0.86;
  }

  .mode-break:focus-within,
  .mode-break:hover {
    opacity: 1;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .title {
    min-width: 0;
    font-size: 1rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    padding: 2px 4px 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .is-done .text {
    color: var(--text-muted);
  }

  .mark {
    display: grid;
    place-items: center;
    flex: none;
    width: 16px;
    height: 16px;
    color: var(--reward-ink);
    font-weight: 800;
  }

  .mark.open::before {
    content: "";
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid var(--quiet-mark);
  }

  /* A real checkbox, at least 24 px (WCAG target size). */
  .check {
    flex: none;
    width: 24px;
    height: 24px;
    margin: 0;
    accent-color: var(--reward);
    cursor: pointer;
  }

  .remove {
    flex: none;
    width: 28px;
    height: 28px;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .editor label {
    font-weight: 600;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row .input {
    flex: 1;
    min-width: 0;
  }

  .ideas {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .idea {
    min-height: 28px;
    cursor: pointer;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 600;
  }

  .idea:hover {
    border-color: var(--border-hover);
    color: var(--text);
  }

  .idea:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .note {
    margin-top: 2px;
  }

  /* Summary: inside the session card, under the goal. */
  .summary {
    margin-top: 14px;
    gap: 6px;
  }

  .label {
    display: flex;
    align-items: baseline;
    gap: 8px;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .count {
    color: var(--reward-ink);
    font-weight: 650;
  }

  .waiting {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    color: var(--text-2);
    font-size: 0.86rem;
    font-weight: 600;
  }
</style>
