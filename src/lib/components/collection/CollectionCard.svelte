<script lang="ts">
  // One collection theme in the picker (ADR-024). Unlocked: a radio in the same group as the base themes, with the
  // theme's live tile. Locked or available now: a blank card with "?", the name and how it is earned (never the art,
  // never darkened art, never a countdown); not a radio, so it is skipped by the arrow keys, but read in full.
  import { intlLocale } from "$lib/i18n/index.svelte";
  import { strings } from "$lib/strings";
  import type { RewardTheme, ThemeUnlock, UnlocksView } from "$lib/types";
  import ThemeTile from "./ThemeTile.svelte";
  import { conditionLines, fullDate } from "./unlockText";

  interface Props {
    item: ThemeUnlock;
    /** The collection view the card belongs to (for the texts of related items). */
    unlocks: UnlocksView | null;
    /** The radio group shared with the base themes. */
    group: string;
    selected: boolean;
    disabled?: boolean;
    onchoose: (theme: RewardTheme) => void;
  }

  let { item, unlocks, group, selected, disabled = false, onchoose }: Props = $props();

  const uid = $props.id();
  const c = strings.collection;

  const name = $derived(c.themeNames[item.theme]);
  const unlocked = $derived(item.state === "unlocked" || item.state === "free");
  const available = $derived(item.state === "availableNow");
  const lines = $derived(unlocked ? [] : conditionLines(item, c, intlLocale(), unlocks));
  const progress = $derived(available && item.progress ? Math.min(1, item.progress.have / Math.max(1, item.progress.need)) : null);
</script>

{#if unlocked}
  <label class="option" class:selected>
    <input
      class="sr-only"
      type="radio"
      name={group}
      value={item.theme}
      checked={selected}
      {disabled}
      aria-labelledby="{uid}-name"
      aria-describedby="{uid}-desc"
      onchange={() => onchoose(item.theme)}
    />
    <ThemeTile theme={item.theme} />
    <span class="text">
      <span class="name" id="{uid}-name">
        {name}
        {#if selected}
          <svg class="check" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" focusable="false">
            <path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        {/if}
      </span>
      <span class="desc" id="{uid}-desc">{c.themeDescriptions[item.theme]}</span>
      {#if item.unlockedOn}
        <span class="earned">{c.unlockedOn(fullDate(item.unlockedOn, intlLocale()))}</span>
      {/if}
    </span>
  </label>
{:else}
  <div class="option locked" class:available role="group" aria-label={c.lockedAria(name, lines.join(". "))}>
    <span class="blank" aria-hidden="true"><span class="mark">?</span></span>
    <span class="text" aria-hidden="true">
      <span class="name">{name}</span>
      <span class="state">{available ? c.availableNow : c.locked}</span>
      {#each lines as line, i (i)}
        <span class="desc">{line}</span>
      {/each}
      {#if progress !== null}
        <span class="bar"><span class="fill" style:transform="scaleX({progress})"></span></span>
      {/if}
    </span>
  </div>
{/if}

<style>
  .option {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 8px 8px 12px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--border);
    background: var(--surface-inset);
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      background-color 0.15s ease,
      box-shadow 0.15s ease;
  }

  label.option:hover {
    border-color: var(--border-hover);
    background: var(--surface-hover);
  }

  .option.selected {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .option:has(input:focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .option:has(input:disabled) {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .locked {
    cursor: default;
    border-style: dashed;
  }

  .locked.available {
    border-color: var(--reward-line);
    border-style: solid;
  }

  .blank {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 16 / 10;
    border-radius: max(0px, calc(var(--radius-sm) - 4px));
    border: 1px dashed var(--border-strong);
    background: var(--surface-solid);
  }

  .mark {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    color: var(--text-2);
    font-size: 1.4rem;
    font-weight: 750;
    line-height: 1;
  }

  .available .mark {
    border-color: var(--reward);
    color: var(--reward-ink);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 4px;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 650;
    color: var(--text);
  }

  .check {
    color: var(--accent-strong);
  }

  .state {
    font-size: 0.78rem;
    font-weight: 650;
    color: var(--text-2);
  }

  .available .state {
    color: var(--reward-ink);
  }

  .desc,
  .earned {
    font-size: 0.82rem;
    line-height: 1.4;
    color: var(--text-muted);
  }

  .bar {
    display: block;
    height: 6px;
    margin-top: 4px;
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--reward);
    transform-origin: left;
  }
</style>
