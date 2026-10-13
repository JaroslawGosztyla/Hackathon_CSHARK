<script lang="ts">
  // Podsumowanie: the sparks of the shown session — "Ta sesja: +138", where they came from, the level and the week.
  // Right after Stop it plays the session-end recap (deferred achievements included) through RewardBurst.
  import { lineLabel } from "$lib/rewards/choreography";
  import type { RewardPlayer } from "$lib/rewards/player.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { SessionSummary } from "$lib/types";
  import BetaBadge from "./BetaBadge.svelte";
  import LevelBar from "./LevelBar.svelte";
  import RewardBurst from "./RewardBurst.svelte";
  import SparkIcon from "./SparkIcon.svelte";
  import WeekStrip from "./WeekStrip.svelte";

  interface Props {
    player: RewardPlayer;
    summary: SessionSummary;
  }

  let { player, summary }: Props = $props();

  const uid = $props.id();
  const s = strings.rewards;

  const status = $derived(rewards.view);
  /** The rewards' own record of this session (the running one, or the last one until the next Start). */
  const sessionSparks = $derived(status?.session?.sessionId === summary.sessionId ? status.session : null);
  const total = $derived(sessionSparks?.sparks ?? summary.sparks ?? 0);
  const lines = $derived(sessionSparks?.lines ?? []);
  /** The recap of this session plays (or shows its final state) in place of the static numbers. */
  const recap = $derived.by(() => {
    const burst = player.burst;
    if (!burst || burst.moment.kind !== "sessionEnd") return false;
    return burst.moment.sample || sessionSparks !== null;
  });
</script>

<section class="card sparks-card" aria-labelledby="{uid}-title">
  <div class="card-title">
    <h2 id="{uid}-title">{s.cardTitle}</h2>
    <BetaBadge />
  </div>

  {#if recap}
    <RewardBurst {player} embedded />
  {:else}
    {#if total > 0}
      <p class="total">
        <SparkIcon size={18} />
        <span class="tabular">{s.sessionTotal(total)}</span>
      </p>
    {/if}
    {#if lines.length > 0}
      <ul class="chips">
        {#each lines as line (line.kind)}
          <li class="chip line">{lineLabel(line, s)} +{line.sparks}</li>
        {/each}
      </ul>
    {/if}
    {#if status}
      <div class="progress">
        <LevelBar level={status.level} />
      </div>
    {/if}
  {/if}

  {#if status && status.week.length > 0}
    <div class="week">
      <WeekStrip week={status.week} />
    </div>
  {/if}
</section>

<style>
  .sparks-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .sparks-card .card-title {
    justify-content: flex-start;
    align-items: center;
    margin-bottom: 0;
  }

  .total {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--reward-ink);
    font-size: 1.3rem;
    font-weight: 700;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .line {
    background: var(--reward-soft);
    border-color: transparent;
    color: var(--reward-ink);
  }

  .week {
    padding-top: 4px;
  }
</style>
