<script lang="ts">
  // Focus: only the next small step is large — "Teraz: [☐] text · +2 kolejne". The other steps unfold on click,
  // "+ krok" adds one inline. A tick fills the box at once (the user's own action may move during focus), then the
  // next step slides up; the "+3" float and the pop come from the reward moment. Unticking keeps the sparks.
  import { untrack } from "svelte";
  import { fly } from "svelte/transition";
  import {
    MAX_STEP_CHARS,
    MAX_STEPS,
    STEP_BURST_MS,
    STEP_DOTS,
    STEP_FILL_MS,
    STEP_SLIDE_MS,
  } from "$lib/rewards/choreography";
  import type { RewardPlayer } from "$lib/rewards/player.svelte";
  import { nextOpenStep, rewards, sessionSteps } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { MicroStep } from "$lib/types";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    player: RewardPlayer;
  }

  let { player }: Props = $props();

  const uid = $props.id();
  const s = strings.rewards;

  const steps = $derived(sessionSteps());
  const next = $derived(nextOpenStep(steps));
  const others = $derived(steps.filter((step) => step.id !== next?.id));
  const openOthers = $derived(others.filter((step) => !step.done).length);
  const allDone = $derived(steps.length > 0 && next === null);
  const full = $derived(steps.length >= MAX_STEPS);
  const still = $derived(rewards.celebration !== "full");
  /** The step or all-steps moment playing now (this line shows it instead of the float over the ring). */
  const moment = $derived.by(() => {
    const info = player.current;
    const kind = info?.moment.kind;
    return info && (kind === "step" || kind === "allSteps") ? info : null;
  });

  let expanded = $state(false);
  let adding = $state(false);
  let text = $state("");
  let busy = $state(false);
  /** The step whose box fills right now (local, before the shell answers). */
  let ticking = $state<number | null>(null);
  let addInput = $state<HTMLInputElement | null>(null);

  // This line shows ticked steps itself: the float over the ring leaves them to it.
  $effect(() => {
    untrack(() => player.stepHosts++);
    return () => {
      player.stepHosts--;
    };
  });

  async function tick(step: MicroStep, done: boolean): Promise<void> {
    if (done) {
      ticking = step.id;
      setTimeout(() => {
        if (ticking === step.id) ticking = null;
      }, STEP_FILL_MS + STEP_BURST_MS);
    }
    await rewards.setStepDone(step.id, done);
  }

  /** Enter ticks too (Space does natively). */
  function onCheckKey(event: KeyboardEvent, step: MicroStep): void {
    if (event.key !== "Enter") return;
    event.preventDefault();
    void tick(step, !step.done);
  }

  async function add(): Promise<void> {
    const value = text.trim();
    if (value === "" || full || busy) return;
    busy = true;
    try {
      const drafts = steps.map((step) => ({ id: step.id, text: step.text }));
      if (await rewards.setSteps([...drafts, { id: null, text: value }])) {
        text = "";
        adding = false;
      }
    } finally {
      busy = false;
    }
  }

  function onAddKey(event: KeyboardEvent): void {
    if (event.key === "Enter" && !event.isComposing) {
      event.preventDefault();
      void add();
    } else if (event.key === "Escape") {
      event.stopPropagation();
      adding = false;
      text = "";
    }
  }

  function openAdd(): void {
    adding = true;
    queueMicrotask(() => addInput?.focus());
  }
</script>

<div class="next-step" class:still>
  <div class="line">
    {#if next}
      <span class="now">{s.now}</span>
      {#key next.id}
        <label class="current" in:fly={{ y: still ? 0 : 10, duration: still ? 0 : STEP_SLIDE_MS }}>
          <span class="box" class:ticking={ticking === next.id && !still}>
            <input
              type="checkbox"
              class="check"
              checked={ticking === next.id}
              onchange={(e) => tick(next, e.currentTarget.checked)}
              onkeydown={(e) => onCheckKey(e, next)}
            />
            {#if ticking === next.id && !still}
              <span class="dots" aria-hidden="true">
                {#each { length: STEP_DOTS } as _, i (i)}
                  <span class="dot" style:--angle="{(360 / STEP_DOTS) * i}deg" style:--ms="{STEP_BURST_MS}ms"></span>
                {/each}
              </span>
            {/if}
          </span>
          <span class="text">{next.text}</span>
        </label>
      {/key}
    {:else if allDone}
      <span class="all-done"><SparkIcon size={13} /> {s.allDone}</span>
      {#if !full}
        <button type="button" class="btn btn-sm" onclick={openAdd}>{s.addAnother}</button>
      {/if}
    {/if}

    {#if others.length > 0}
      <button
        type="button"
        class="more"
        aria-expanded={expanded}
        aria-controls="{uid}-list"
        onclick={() => (expanded = !expanded)}
      >
        {expanded ? s.moreHide : s.more(openOthers > 0 ? openOthers : others.length)}
      </button>
    {/if}
    {#if !full && !adding && !allDone}
      <button type="button" class="more" onclick={openAdd}>{s.stepAddMore}</button>
    {/if}

    {#if moment}
      {#key moment.moment.id}
        <span class="float kind-{moment.moment.kind}" class:still={moment.still} aria-hidden="true">
          <!-- A tick that pays nothing (a tick again, or past the daily cap) shows only its caption: never "+0". -->
          {#if moment.moment.sparks > 0}
            +{moment.moment.sparks}
            <SparkIcon size={12} />
          {/if}
          <span class="caption">{moment.caption}</span>
        </span>
      {/key}
    {/if}
  </div>

  {#if adding}
    <div class="add">
      <input
        class="input"
        type="text"
        maxlength={MAX_STEP_CHARS}
        placeholder={s.stepPlaceholder}
        aria-label={s.stepsPrompt}
        autocomplete="off"
        bind:value={text}
        bind:this={addInput}
        onkeydown={onAddKey}
      />
      <button type="button" class="btn btn-sm" disabled={busy || text.trim() === ""} onclick={add}>{s.stepAdd}</button>
    </div>
  {/if}

  {#if expanded && others.length > 0}
    <ul class="list" id="{uid}-list">
      {#each others as step (step.id)}
        <li>
          <label class="other" class:is-done={step.done}>
            <input
              type="checkbox"
              class="check"
              checked={step.done}
              onchange={(e) => tick(step, e.currentTarget.checked)}
              onkeydown={(e) => onCheckKey(e, step)}
            />
            <span class="text">{step.text}</span>
          </label>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .next-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    max-width: min(560px, 100%);
  }

  .line {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 6px 10px;
    min-width: 0;
  }

  .now {
    color: var(--text-muted);
    font-weight: 600;
  }

  .current {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 1.08rem;
    font-weight: 650;
    cursor: pointer;
  }

  .text {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .box {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
  }

  .check {
    width: 24px;
    height: 24px;
    margin: 0;
    accent-color: var(--reward);
    cursor: pointer;
  }

  .ticking .check {
    animation: fill var(--fill-ms, 150ms) ease-out;
  }

  @keyframes fill {
    from {
      transform: scale(0.8);
    }
  }

  .dots {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .dot {
    position: absolute;
    top: calc(50% - 2px);
    left: calc(50% - 2px);
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--reward);
    animation: burst var(--ms) ease-out forwards;
  }

  @keyframes burst {
    from {
      transform: rotate(var(--angle)) translateY(0);
      opacity: 1;
    }
    to {
      transform: rotate(var(--angle)) translateY(-16px);
      opacity: 0;
    }
  }

  .all-done {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--reward-ink);
    font-weight: 650;
  }

  .more {
    min-height: 24px;
    padding: 2px 8px;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-2);
    font: inherit;
    font-size: 0.86rem;
    cursor: pointer;
  }

  .more:hover {
    color: var(--text);
    background: var(--surface-inset);
  }

  .more:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  /* "+3 ✦": rises and fades next to the line (still: shows without movement). Never takes clicks. */
  .float {
    position: absolute;
    left: 100%;
    top: 50%;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: 8px;
    padding: 2px 10px;
    border-radius: var(--radius-pill);
    background: var(--reward-soft);
    color: var(--reward-ink);
    font-weight: 700;
    white-space: nowrap;
    pointer-events: none;
    transform: translateY(-50%);
    animation: step-float 600ms ease-out forwards;
  }

  .float.kind-allSteps {
    animation-duration: 1600ms;
  }

  .float.still {
    animation: none;
  }

  .caption {
    font-weight: 600;
  }

  @keyframes step-float {
    0% {
      opacity: 0;
      transform: translateY(-30%);
    }
    20% {
      opacity: 1;
      transform: translateY(-50%);
    }
    80% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateY(-90%);
    }
  }

  :global([data-theme="pixel"]) .float {
    animation-timing-function: steps(6, end);
  }

  @media (max-width: 720px) {
    .float {
      left: 50%;
      top: auto;
      bottom: 100%;
      margin-left: 0;
      transform: translateX(-50%);
      animation-name: step-float-up;
    }

    @keyframes step-float-up {
      0% {
        opacity: 0;
        transform: translate(-50%, 6px);
      }
      20%,
      80% {
        opacity: 1;
        transform: translate(-50%, 0);
      }
      100% {
        opacity: 0;
        transform: translate(-50%, -10px);
      }
    }
  }

  .add {
    display: flex;
    gap: 8px;
    width: min(420px, 100%);
  }

  .add .input {
    flex: 1;
    min-width: 0;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
    text-align: left;
  }

  .other {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .other.is-done .text {
    color: var(--text-muted);
  }
</style>
