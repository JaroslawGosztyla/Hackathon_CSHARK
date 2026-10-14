<script lang="ts">
  // P1: a small pill in the header — the level as a ring with its number, and the streak bubble with its days (only
  // while the streak shows and counts something; never a 0). Links to Skupienie. Static: it never animates.
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { RewardsStatus } from "$lib/types";
  import BubbleIcon from "./BubbleIcon.svelte";

  interface Props {
    status: RewardsStatus;
  }

  let { status }: Props = $props();

  const RADIUS = 9;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
  const s = strings.rewards;

  const level = $derived(status.level);
  const fraction = $derived(level.span > 0 ? Math.min(1, Math.max(0, level.into / level.span)) : 0);
  const streakDays = $derived(
    rewards.showStreak && (status.streak.state === "alive" || status.streak.state === "paused") ? status.streak.days : 0,
  );
  const label = $derived(
    [s.levelAria(level.level, level.into, level.span), streakDays > 0 ? s.streakDays(streakDays) : null]
      .filter((part) => part !== null)
      .join(". "),
  );
</script>

<a class="rewards-pill" href="/" aria-label={label} title={label}>
  <span class="ring" aria-hidden="true">
    <svg viewBox="0 0 22 22" width="22" height="22">
      <circle class="track" cx="11" cy="11" r={RADIUS} />
      <circle
        class="value"
        cx="11"
        cy="11"
        r={RADIUS}
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={CIRCUMFERENCE * (1 - fraction)}
        transform="rotate(-90 11 11)"
      />
    </svg>
    <span class="level tabular">{level.level}</span>
  </span>
  {#if streakDays > 0}
    <span class="streak tabular" aria-hidden="true"><BubbleIcon size={14} filled={status.streak.todayCounted} />{streakDays}</span>
  {/if}
</a>

<style>
  .rewards-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 3px 10px 3px 4px;
    border-radius: var(--radius-pill);
    border: var(--border-w) solid var(--reward-line);
    background: var(--surface-inset);
    color: var(--text);
    text-decoration: none;
    font-size: 0.82rem;
    font-weight: 700;
  }

  .rewards-pill:hover {
    background: var(--surface-hover);
  }

  .rewards-pill:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .ring {
    position: relative;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
  }

  svg {
    position: absolute;
    inset: 0;
  }

  .track {
    fill: none;
    stroke: var(--track-strong);
    stroke-width: 2.5;
  }

  .value {
    fill: none;
    stroke: var(--reward);
    stroke-width: 2.5;
    stroke-linecap: round;
  }

  .level {
    position: relative;
    color: var(--reward-ink);
    font-size: 0.68rem;
    line-height: 1;
  }

  .streak {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
  }
</style>
