<script lang="ts">
  // Settings → Wygląd → "Kolekcja" (beta, ADR-024): the collection themes under the base ones, in catalogue order with
  // the seasonal theme whose window is open first. Rendered only while the beta is on (the parent checks). Unlocked
  // cards are radios of the same group as the base themes; the rest explain how they are earned. With the rewards off
  // the unlocked themes stay usable and a note says where new unlocks come from. A Developer-page sample status shows
  // its own collection (picking from it previews the theme instead of saving it).
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { ThemeId } from "$lib/themes";
  import type { RewardTheme, ThemeUnlock } from "$lib/types";
  import CollectionCard from "./CollectionCard.svelte";

  interface Props {
    /** The radio group shared with the base themes. */
    group: string;
    /** The theme on screen. */
    current: ThemeId;
    disabled?: boolean;
    onchoose: (theme: RewardTheme) => void;
  }

  let { group, current, disabled = false, onchoose }: Props = $props();

  const uid = $props.id();
  const c = strings.collection;

  const unlocks = $derived(rewards.sampleStatus?.unlocks ?? rewards.real?.unlocks ?? null);

  /** Catalogue order, the open seasonal window first (stable otherwise). */
  const items = $derived.by((): ThemeUnlock[] => {
    const themes = unlocks?.themes ?? [];
    const open = (t: ThemeUnlock) => (t.state !== "unlocked" && t.window?.open === true ? 0 : 1);
    return [...themes].sort((a, b) => open(a) - open(b));
  });

  /** Some card still has something to earn. */
  const earnable = $derived(items.some((t) => t.state === "locked" || t.state === "availableNow"));

  function goToBeta(): void {
    const target = document.getElementById("beta-settings");
    if (!target) return;
    target.scrollIntoView({ block: "start", behavior: rewards.reduced ? "auto" : "smooth" });
  }
</script>

<section class="collection" aria-labelledby="{uid}-title">
  <div class="head">
    <h3 id="{uid}-title">{c.title}</h3>
    <p class="hint">{c.hint}</p>
    {#if !rewards.configActive && earnable && !rewards.sampleStatus}
      <p class="hint note">
        {c.rewardsOffHint}
        <button type="button" class="link" onclick={goToBeta}>{c.rewardsOffLink}</button>
      </p>
    {/if}
  </div>

  {#if items.length > 0}
    <div class="grid">
      {#each items as item (item.theme)}
        <CollectionCard
          {item}
          {unlocks}
          {group}
          {disabled}
          selected={current === item.theme}
          {onchoose}
        />
      {/each}
    </div>
  {/if}
</section>

<style>
  .collection {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  h3 {
    margin: 0;
    font-size: 1rem;
  }

  .note {
    color: var(--text-2);
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-strong);
    font: inherit;
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }

  .link:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: 2px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }

  @container (max-width: 440px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }
  }
</style>
