<script lang="ts">
  // The published surprise odds (permille from the shell), the pity rule and the collection, in a small non-modal
  // popover. Surprises are tied to completed blocks only: nothing to open, buy or re-roll.
  import { intlLocale } from "$lib/i18n/index.svelte";
  import { strings } from "$lib/strings";
  import type { Collection, Rarity, SurpriseOdds } from "$lib/types";

  interface Props {
    odds: SurpriseOdds;
    collection?: Collection | null;
    /** A text button ("Jak działają niespodzianki?") instead of the round "?". */
    labelled?: boolean;
    /** The popover opens towards this side of the button. */
    align?: "start" | "end";
  }

  let { odds, collection = null, labelled = false, align = "end" }: Props = $props();

  const uid = $props.id();
  let open = $state(false);
  let root = $state<HTMLElement | null>(null);
  let button = $state<HTMLButtonElement | null>(null);

  const s = strings.rewards;

  const percent = $derived(new Intl.NumberFormat(intlLocale(), { style: "percent", maximumFractionDigits: 1 }));

  const rows = $derived<{ key: Rarity | "none"; label: string; permille: number }[]>([
    { key: "none", label: s.oddsNone, permille: odds.nonePermille },
    { key: "bubble", label: s.rarity.bubble, permille: odds.bubblePermille },
    { key: "pearl", label: s.rarity.pearl, permille: odds.pearlPermille },
    { key: "goldenPearl", label: s.rarity.goldenPearl, permille: odds.goldenPermille },
  ]);

  function close(focusButton: boolean): void {
    open = false;
    if (focusButton) button?.focus();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "Escape" || !open) return;
    event.stopPropagation();
    close(true);
  }

  // A click or focus outside closes it (never a modal: the rest of the page stays usable).
  $effect(() => {
    if (!open) return;
    const outside = (event: Event) => {
      if (root && event.target instanceof Node && !root.contains(event.target)) close(false);
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("focusin", outside);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("focusin", outside);
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="odds" bind:this={root} onkeydown={onKeydown}>
  <button
    type="button"
    class={labelled ? "btn btn-sm btn-ghost" : "help"}
    aria-expanded={open}
    aria-controls="{uid}-pop"
    title={labelled ? undefined : s.oddsButton}
    bind:this={button}
    onclick={() => (open = !open)}
  >
    {#if labelled}
      {s.oddsButton}
    {:else}
      <span aria-hidden="true">?</span><span class="sr-only">{s.oddsButton}</span>
    {/if}
  </button>
  {#if open}
    <div class="pop align-{align}" id="{uid}-pop" role="region" aria-labelledby="{uid}-title">
      <p class="title" id="{uid}-title">{s.oddsTitle}</p>
      <p class="body">{s.oddsBody}</p>
      <ul class="rows">
        {#each rows as row (row.key)}
          <li class="row-{row.key}">
            <span class="swatch" aria-hidden="true"></span>
            {s.oddsRow(row.label, percent.format(row.permille / 1000))}
          </li>
        {/each}
      </ul>
      <p class="body">{s.oddsPity(odds.pityBlocks)}</p>
      {#if collection}
        <p class="collection">{s.collection(collection.bubbles, collection.pearls, collection.golden)}</p>
      {/if}
    </div>
  {/if}
</span>

<style>
  .odds {
    position: relative;
    display: inline-flex;
  }

  .help {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: 50%;
    border: var(--border-w) solid var(--border-strong);
    background: var(--surface-inset);
    color: var(--text-2);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
  }

  .help:hover {
    border-color: var(--border-hover);
    color: var(--text);
  }

  .help:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .pop {
    position: absolute;
    z-index: 30;
    top: calc(100% + 6px);
    width: min(300px, 80vw);
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--border-strong);
    background: var(--surface-raised);
    box-shadow: var(--shadow-sm);
    color: var(--text);
    text-align: left;
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .align-end {
    right: 0;
  }

  .align-start {
    left: 0;
  }

  .title {
    font-weight: 700;
  }

  .body {
    margin-top: 6px;
    color: var(--text-2);
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }

  .rows li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid var(--quiet-mark);
  }

  .row-bubble .swatch {
    border-color: var(--bubble);
    background: var(--bubble-soft);
  }

  .row-pearl .swatch {
    border-color: var(--text-2);
    background: color-mix(in srgb, var(--text) 30%, transparent);
  }

  .row-goldenPearl .swatch {
    border-color: var(--reward);
    background: var(--reward);
  }

  .collection {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    color: var(--text-2);
  }
</style>
