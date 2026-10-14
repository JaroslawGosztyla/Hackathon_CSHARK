<script lang="ts">
  // Skupienie (idle): today at a glance — the daily-goal ring, the streak (a bubble, never a flame; the UI never prints
  // 0 and never says a streak was lost), the rest-day bank, the level and the week. Wraps on a narrow window.
  import { REST_EARN_EVERY } from "$lib/rewards/choreography";
  import { config } from "$lib/state/config.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { showcaseMode } from "$lib/state/showcaseMode.svelte";
  import { strings } from "$lib/strings";
  import type { RewardsStatus } from "$lib/types";
  import BetaBadge from "./BetaBadge.svelte";
  import BubbleIcon from "./BubbleIcon.svelte";
  import GoalRing from "./GoalRing.svelte";
  import LevelBar from "./LevelBar.svelte";
  import SurpriseOdds from "./SurpriseOdds.svelte";
  import WeekStrip from "./WeekStrip.svelte";

  interface Props {
    status: RewardsStatus;
  }

  let { status }: Props = $props();

  const uid = $props.id();
  const s = strings.rewards;

  const today = $derived(status.today);
  const streak = $derived(status.streak);

  /** The streak line and its smaller second line, per state; a 0 is never printed. */
  const streakText = $derived.by((): { main: string; sub: string | null } => {
    switch (streak.state) {
      case "first":
        return { main: s.streakFirst, sub: null };
      case "alive": {
        const main = streak.days > 0 ? s.streakDays(streak.days) : s.streakFirst;
        if (streak.todayCounted) return { main, sub: s.todayCounts };
        return { main, sub: streak.restReserved > 0 ? s.restReserved(streak.restReserved) : null };
      }
      case "waiting": {
        const parts = [streak.best > 0 ? s.streakBest(streak.best) : null, streak.activeDays > 0 ? s.goalDays(streak.activeDays) : null];
        const sub = parts.filter((p) => p !== null).join(" · ");
        return { main: s.streakWaiting, sub: sub === "" ? null : sub };
      }
      case "paused":
        return { main: s.streakPaused, sub: streak.days > 0 ? s.streakDays(streak.days) : null };
    }
  });
</script>

<section class="card today-strip" aria-labelledby="{uid}-title">
  <div class="head">
    <h2 id="{uid}-title" class="title">{s.todayTitle}</h2>
    <!-- The presentation showcase must look like the real app on stage: no "demo" badge. -->
    {#if status.preview && !showcaseMode.active}<span class="badge demo">{s.demo}</span>{/if}
    <BetaBadge />
    <span class="spacer"></span>
    <!-- The odds only while surprises are on (otherwise they would promise something that never comes). -->
    {#if showcaseMode.active || config.value?.rewards?.surprises !== false}
      <SurpriseOdds odds={status.odds} collection={status.collection} />
    {/if}
  </div>

  <div class="items">
    <div class="item goal">
      <GoalRing sparks={today.sparks} goal={today.goal} met={today.goalMet} />
      <div class="text">
        <p class="label">{s.dailyGoal}</p>
        <p class="value tabular">{today.goalMet ? s.dailyGoalMet : s.dailyGoalOf(today.sparks, today.goal)}</p>
      </div>
    </div>

    {#if rewards.showStreak}
      <div class="item streak">
        <BubbleIcon size={22} filled={streak.state === "alive" && streak.todayCounted} />
        <div class="text">
          <p class="value">{streakText.main}</p>
          {#if streakText.sub}<p class="sub">{streakText.sub}</p>{/if}
        </div>
      </div>

      <p class="item rest" title={s.restBankHint(REST_EARN_EVERY)}>
        <span class="zz" aria-hidden="true">z</span>
        {s.restBank(streak.restBank, streak.restBankMax)}
        <span class="sr-only">. {s.restBankHint(REST_EARN_EVERY)}</span>
      </p>
    {/if}

    <div class="item level">
      <LevelBar level={status.level} />
    </div>

    {#if status.week.length > 0}
      <div class="item week">
        <WeekStrip week={status.week} />
      </div>
    {/if}
  </div>

  {#if today.pondFull}
    <p class="pond hint">{s.pondFull}</p>
  {/if}
</section>

<style>
  .today-strip {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title {
    font-size: 1rem;
  }

  .spacer {
    flex: 1;
  }

  .demo {
    background: var(--surface-inset);
    color: var(--text-2);
  }

  .items {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px 22px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .label {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .value {
    font-weight: 650;
    font-size: 0.92rem;
  }

  .goal .value {
    color: var(--reward-ink);
  }

  .streak .text {
    max-width: 26ch;
  }

  .sub {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .rest {
    gap: 6px;
    font-size: 0.84rem;
    color: var(--text-2);
    cursor: help;
  }

  .zz {
    color: var(--bubble);
    font-weight: 800;
    font-style: italic;
  }

  .level {
    flex: 0 1 auto;
  }

  .pond {
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  @media (max-width: 520px) {
    .items {
      gap: 10px 16px;
    }

    .level {
      flex-basis: 100%;
    }
  }
</style>
