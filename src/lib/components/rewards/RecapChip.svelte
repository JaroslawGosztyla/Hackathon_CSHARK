<script lang="ts">
  // Skupienie, break: the block's recap as one static chip next to the "focused so far" line under the session buttons
  // ("+72 ✦ · Cel dnia ✓ · Seria 5 · Poz. 8", only the parts present; the streak only while it is shown), so it is in
  // view at any window size. The burst card below the hero has the details. Never animates.
  import { burstSparks } from "$lib/rewards/choreography";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { RewardMoment } from "$lib/types";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    moment: RewardMoment;
  }

  let { moment }: Props = $props();

  const s = strings.rewards;
  const total = $derived(burstSparks(moment));
  const parts = $derived.by(() => {
    const out: string[] = [];
    if (moment.dailyGoal) out.push(s.popupDailyGoal);
    if (moment.streak !== null && rewards.showStreak) out.push(s.popupStreak(moment.streak));
    if (moment.levelUp !== null) out.push(s.popupLevel(moment.levelUp));
    return out;
  });
</script>

{#if total > 0 || parts.length > 0}
  <span class="recap-chip tabular">
    {#if total > 0}<SparkIcon size={12} /><span>+{total}</span>{/if}
    {#each parts as part, i (i)}
      {#if total > 0 || i > 0}<span class="sep" aria-hidden="true">·</span>{/if}
      <span>{part}</span>
    {/each}
  </span>
{/if}

<style>
  .recap-chip {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 2px 6px;
    padding: 0 10px;
    border-radius: var(--radius-pill);
    background: var(--reward-soft);
    color: var(--reward-ink);
    font-size: 0.82rem;
    font-weight: 700;
    line-height: 1.6;
    vertical-align: 1px;
  }

  .sep {
    opacity: 0.7;
  }
</style>
