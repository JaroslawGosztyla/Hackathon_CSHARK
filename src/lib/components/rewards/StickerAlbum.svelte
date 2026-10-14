<script lang="ts">
  // P1: the sticker album (ADR-022) — one sticker per pose of the owner's art, unlocked deterministically (no luck).
  // An unlocked sticker shows its pose (only displayed, never redrawn); a locked one is a plain card back with "?" and
  // the hint, never a darkened or silhouetted axolotl. A collection the user opens on purpose: it shows the poses even
  // while the mascot is hidden elsewhere, and only once it is open (the poses atlas is requested on the first expand).
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { poseFrame, type PoseName } from "$lib/mascot/frames";
  import { strings } from "$lib/strings";
  import type { StickerId } from "$lib/types";

  interface Props {
    unlocked: readonly StickerId[];
  }

  let { unlocked }: Props = $props();

  /** Sticker → pose (spec ADR-022 §3.9), in album order. */
  const STICKERS: readonly { id: StickerId; pose: PoseName }[] = [
    { id: "firstStep", pose: "01-point-excited" },
    { id: "dailyGoal", pose: "02-clasped-hands-heart" },
    { id: "planner", pose: "03-thinking" },
    { id: "goodBreak", pose: "04-lying-awake" },
    { id: "comeback", pose: "05-look-up" },
    { id: "amazed", pose: "06-amazed-sparkles" },
    { id: "restDay", pose: "07-sleeping" },
    { id: "together", pose: "08-back-view" },
    { id: "threeDays", pose: "09-lying-look-up" },
    { id: "goldenPearl", pose: "10-holding-heart" },
    { id: "level5", pose: "11-point-happy" },
  ];
  const POSE_SIZE = 72;

  const s = strings.rewards.album;
  const owned = $derived(new Set(unlocked));
  const count = $derived(STICKERS.filter((sticker) => owned.has(sticker.id)).length);
  /** Opened at least once: the poses render from then on (closing it again keeps them, no reload). */
  let open = $state(false);
  let opened = $state(false);
  $effect(() => {
    if (open) opened = true;
  });
</script>

<details class="card album" bind:open>
  <summary>
    <span class="title">{s.title}</span>
    <span class="count tabular">{s.count(count, STICKERS.length)}</span>
  </summary>
  <ul class="grid">
    {#each STICKERS as sticker (sticker.id)}
      {@const name = s.sticker[sticker.id].name}
      {@const hint = s.sticker[sticker.id].hint}
      {#if owned.has(sticker.id)}
        <li class="sticker">
          <span class="art">
            {#if opened}<Axolotl size={POSE_SIZE} frame={poseFrame(sticker.pose)} poses />{/if}
          </span>
          <span class="name">{name}</span>
        </li>
      {:else}
        <li class="sticker locked" aria-label={s.lockedAria(hint)}>
          <span class="back" aria-hidden="true">?</span>
          <span class="hint" aria-hidden="true">{hint}</span>
        </li>
      {/if}
    {/each}
  </ul>
</details>

<style>
  .album summary {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-weight: 650;
  }

  .album summary:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
    border-radius: 4px;
  }

  .count {
    color: var(--reward-ink);
    font-size: 0.86rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 10px;
    margin: 14px 0 0;
    padding: 0;
    list-style: none;
  }

  .sticker {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    min-height: 132px;
    padding: 10px 8px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--reward-line);
    background: var(--reward-soft);
    text-align: center;
  }

  .art {
    display: grid;
    place-items: end center;
    height: 80px;
  }

  .name {
    font-size: 0.84rem;
    font-weight: 650;
  }

  .locked {
    justify-content: center;
    border-style: dashed;
    border-color: var(--border-strong);
    background: var(--surface-inset);
  }

  .back {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: var(--border-w) solid var(--border-strong);
    color: var(--text-muted);
    font-size: 1.2rem;
    font-weight: 700;
  }

  .hint {
    color: var(--text-muted);
    font-size: 0.78rem;
    line-height: 1.35;
  }
</style>
