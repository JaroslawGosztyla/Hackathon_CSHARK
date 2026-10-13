<script lang="ts">
  // The recap of a block (break start), a session (Stop) or an update outside focus: "Ten blok: +72" counts up, the
  // line chips fade in, then the beats in order — surprise → daily goal → comeback → streak → rest days → level up →
  // pond full → stickers — and finally a compact state (the header, then ring, streak, level) with × to hide it for this
  // break, so the Break card below is not pushed down for the whole break. A stale moment and "numbers only" keep the
  // full list (it never played). Inside the summary card everything stays.
  // In the content flow (never over the session buttons), never modal, never takes focus. Skip, Esc or a click on the
  // card shows the final state at once. Calm: every beat in its final state at once (same timing, no particles);
  // numbers only: everything listed at once.
  import {
    CHIP_STAGGER,
    COUNT_START,
    MAX_CHIPS,
    MAX_STICKER_BEATS,
    lineLabel,
    lineSparks,
    pickAt,
    surpriseSparks,
    type Beat,
  } from "$lib/rewards/choreography";
  import type { RewardPlayer } from "$lib/rewards/player.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { RewardMoment } from "$lib/types";
  import BetaBadge from "./BetaBadge.svelte";
  import BubbleIcon from "./BubbleIcon.svelte";
  import Confetti from "./Confetti.svelte";
  import GoalRing from "./GoalRing.svelte";
  import LevelBar from "./LevelBar.svelte";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    player: RewardPlayer;
    /** Inside another card (the summary): no card of its own. */
    embedded?: boolean;
  }

  let { player, embedded = false }: Props = $props();

  const uid = $props.id();
  const s = strings.rewards;
  /** The ring of the daily-goal beat starts here and closes. */
  const GOAL_RING_FROM = 0.72;

  const info = $derived(player.burst);
  const moment = $derived(info?.moment ?? null);
  const isCurrent = $derived(info !== null && player.current === info);
  const playing = $derived(isCurrent && !player.done);
  /** The beat reached so far (everything once it is not the one playing any more). */
  const reached = $derived(info ? (isCurrent ? player.beat : info.beats.length) : -1);
  const shownNumber = $derived(info ? (isCurrent ? player.shown : info.total) : 0);
  const view = $derived(rewards.view);
  const still = $derived(info?.still ?? true);
  /** The final state after a play: header and the state row only (see above). */
  const compact = $derived(!embedded && info !== null && !playing && !info.stale && info.level !== "numbersOnly");

  const variant = $derived(info?.variant ?? 0);

  /** The header line (the recap variant is picked once per play). */
  function header(m: RewardMoment, n: number): string {
    if (m.kind === "sessionEnd") return s.sessionTotal(n);
    if (m.kind === "update") return `${s.todayTitle}: +${n}`;
    return pickAt(s.recapLines(n), variant);
  }

  const chips = $derived.by(() => {
    const lines = moment?.lines ?? [];
    const shown = lines.slice(0, MAX_CHIPS).map((line) => ({ key: line.kind, text: `${lineLabel(line, s)} +${line.sparks}` }));
    const rest = lines.slice(MAX_CHIPS).reduce((sum, line) => sum + line.sparks, 0);
    return { shown, rest };
  });

  /** One row per beat after the count-up (the texts are picked once per moment). */
  const rows = $derived.by(() => {
    const m = moment;
    if (!m || !info) return [];
    const showStreak = rewards.showStreak;
    const out: { index: number; beat: Beat; text: string }[] = [];
    info.beats.forEach((beat, index) => {
      if (index === 0) return;
      const streakBeat = beat.kind === "streak" || beat.kind === "restUsed" || beat.kind === "restEarned";
      if (streakBeat && !showStreak) return;
      const text = beatText(beat, m, variant + index);
      if (text) out.push({ index, beat, text });
    });
    return out;
  });

  function beatText(beat: Beat, m: RewardMoment, n: number): string | null {
    switch (beat.kind) {
      case "count":
        return null;
      case "surprise":
        return m.surprise ? s.surpriseFound(s.rarity[m.surprise], surpriseSparks(m)) : null;
      case "dailyGoal": {
        const bonus = lineSparks(m, "dailyGoal");
        return bonus > 0 ? `${pickAt(s.dailyGoalLines, n)} +${bonus}` : pickAt(s.dailyGoalLines, n);
      }
      case "comeback": {
        const bonus = lineSparks(m, "comeback");
        return bonus > 0 ? `${pickAt(s.comebackLines, n)} +${bonus}` : pickAt(s.comebackLines, n);
      }
      case "streak":
        if (m.newStreak) return s.newStreak;
        if (m.milestone !== null) return s.milestone(m.milestone);
        return m.streak !== null ? s.streakUp(m.streak) : null;
      case "restUsed":
        return s.restUsed(m.streak ?? 1);
      case "restEarned": {
        const streak = rewards.view?.streak;
        return streak ? s.restEarned(streak.restBank, streak.restBankMax) : null;
      }
      case "levelUp":
        if (m.levelUp === null) return null;
        return m.levelsGained > 1 ? s.levelUpMany(m.levelUp, m.levelsGained) : s.levelUp(m.levelUp);
      case "pondFull":
        return s.pondFull;
      case "sticker":
        return beat.sticker ? s.album.unlocked(s.album.sticker[beat.sticker].name) : null;
      case "stickersMore":
        return s.inAlbum(m.stickers.length - MAX_STICKER_BEATS);
    }
  }

  const golden = $derived.by(() => {
    if (!info || moment?.surprise !== "goldenPearl") return false;
    const index = info.beats.findIndex((b) => b.kind === "surprise");
    return index >= 0 && reached >= index;
  });

  /** Esc ends the celebration (final state). */
  $effect(() => {
    if (!playing) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) player.finish();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  /** A click on the card (not on a button inside) shows the final state. */
  function onCardClick(event: MouseEvent): void {
    if (!playing) return;
    if (event.target instanceof Element && event.target.closest("button, a, input")) return;
    player.finish();
  }
</script>

{#if info && moment}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <section
    class="burst"
    class:card={!embedded}
    class:embedded
    class:golden
    class:still
    class:pixel={info.pixel}
    aria-labelledby="{uid}-title"
    onclick={onCardClick}
  >
    <Confetti trigger={isCurrent ? player.confetti : 0} enabled={!still} pixel={info.pixel} />

    <div class="head">
      <h2 class="total" id="{uid}-title">
        <SparkIcon size={20} />
        <span aria-hidden="true" class="tabular">{header(moment, shownNumber)}</span>
        <span class="sr-only">{header(moment, info.total)}</span>
      </h2>
      {#if moment.preview || moment.sample}<span class="badge demo">{s.demo}</span>{/if}
      {#if !embedded}<BetaBadge />{/if}
      <span class="spacer"></span>
      {#if playing}
        <button
          type="button"
          class="btn btn-sm skip"
          aria-label={s.skipCelebration}
          title={s.skipCelebration}
          onclick={() => player.finish()}>{s.skip}</button
        >
      {:else if !embedded}
        <button type="button" class="icon-btn close" aria-label={s.close} title={s.close} onclick={() => player.dismiss()}>
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          </svg>
        </button>
      {/if}
    </div>

    {#if chips.shown.length > 0 && !compact}
      <ul class="chips">
        {#each chips.shown as chip, i (chip.key)}
          <li class="chip line" class:enter={playing && !still} style:--delay="{COUNT_START + i * CHIP_STAGGER}ms">{chip.text}</li>
        {/each}
        {#if chips.rest > 0}
          <li class="chip line">+{chips.rest}</li>
        {/if}
      </ul>
    {/if}

    {#if rows.length > 0 && !compact}
      <ul class="beats">
        {#each rows as row (row.index)}
          {#if row.index <= reached}
            {@const now = playing && row.index === reached}
            <li class="beat kind-{row.beat.kind}" class:now class:moving={now && !still}>
              <span class="icon" aria-hidden="true">
                {#if row.beat.kind === "dailyGoal" && view}
                  <GoalRing
                    sparks={view.today.sparks}
                    goal={view.today.goal}
                    met={true}
                    size={26}
                    from={now ? GOAL_RING_FROM : null}
                    still={still || !now}
                  />
                {:else if row.beat.kind === "streak" || row.beat.kind === "comeback"}
                  <BubbleIcon size={20} filled />
                {:else if row.beat.kind === "restUsed" || row.beat.kind === "restEarned"}
                  <span class="zz">z</span>
                {:else if row.beat.kind === "surprise"}
                  <span class="pearl rarity-{moment.surprise}" class:flip={now && !still}></span>
                {:else}
                  <SparkIcon size={18} />
                {/if}
              </span>
              <span class="text">{row.text}</span>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}

    {#if !playing && view}
      <div class="final">
        <GoalRing sparks={view.today.sparks} goal={view.today.goal} met={view.today.goalMet} size={30} still />
        <span class="today tabular">{view.today.goalMet ? s.dailyGoalMet : s.dailyGoalOf(view.today.sparks, view.today.goal)}</span>
        {#if rewards.showStreak && view.streak.state === "alive" && view.streak.days > 0}
          <span class="streak"><BubbleIcon size={16} filled={view.streak.todayCounted} /> {s.streakDays(view.streak.days)}</span>
        {/if}
        <LevelBar level={view.level} flash={false} />
      </div>
    {:else if playing && view && moment.levelUp !== null}
      {@const levelBeat = info.beats.findIndex((b) => b.kind === "levelUp")}
      {#if levelBeat >= 0 && reached === levelBeat}
        <div class="final">
          <LevelBar level={view.level} flash={!still} />
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .burst {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
  }

  .card.burst {
    margin-bottom: 16px;
    border-color: var(--reward-line);
  }

  /* A golden pearl: the card gets a gold border (rarity is also said in text). */
  .card.golden {
    border-color: var(--reward);
    box-shadow:
      0 0 0 1px var(--reward),
      var(--shadow);
  }

  .embedded {
    padding: 0;
  }

  .head {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .total {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--reward-ink);
    font-size: 1.3rem;
  }

  .spacer {
    flex: 1;
  }

  .demo {
    background: var(--surface-inset);
    color: var(--text-2);
  }

  .close {
    width: 30px;
    height: 30px;
  }

  .chips,
  .beats {
    position: relative;
    display: flex;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chips {
    flex-wrap: wrap;
    gap: 6px;
  }

  .line {
    background: var(--reward-soft);
    border-color: transparent;
    color: var(--reward-ink);
  }

  .line.enter {
    animation: chip-in 300ms ease-out both;
    animation-delay: var(--delay);
  }

  .pixel .line.enter {
    animation-timing-function: steps(3, end);
  }

  @keyframes chip-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }

  .beats {
    flex-direction: column;
    gap: 6px;
  }

  .beat {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    font-weight: 600;
  }

  .beat.now .text {
    color: var(--reward-ink);
  }

  .beat.moving {
    animation: beat-pop 300ms ease-out both;
  }

  .pixel .beat.moving {
    animation-timing-function: steps(3, end);
  }

  .kind-levelUp.moving .text {
    display: inline-block;
    animation: level-pop 300ms ease-out both;
  }

  @keyframes beat-pop {
    from {
      opacity: 0;
      transform: translateX(-6px);
    }
  }

  @keyframes level-pop {
    0% {
      transform: scale(0.8);
    }
    60% {
      transform: scale(1.05);
    }
    100% {
      transform: scale(1);
    }
  }

  .icon {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 28px;
  }

  /* The daily goal: once the ring has closed, a bubble ripples out of it (and a bubble pops on a streak +1). */
  .kind-dailyGoal.moving .icon::after,
  .kind-streak.moving .icon::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 2px solid var(--bubble);
    pointer-events: none;
    animation: ripple 500ms ease-out both;
  }

  .kind-dailyGoal.moving .icon::after {
    animation-delay: 600ms;
  }

  .pixel .beat.moving .icon::after {
    border-radius: 0;
    animation-timing-function: steps(4, end);
  }

  @keyframes ripple {
    from {
      opacity: 1;
      transform: scale(0.8);
    }
    to {
      opacity: 0;
      transform: scale(1.8);
    }
  }

  .zz {
    color: var(--bubble);
    font-weight: 800;
    font-style: italic;
  }

  /* The surprise: a small round "pearl" (the rarity is in the text, not only in the colour). */
  .pearl {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--text-2);
    background: color-mix(in srgb, var(--text) 30%, transparent);
  }

  .pearl.rarity-bubble {
    border-color: var(--bubble);
    background: var(--bubble-soft);
  }

  .pearl.rarity-goldenPearl {
    border-color: var(--reward);
    background: var(--reward);
  }

  .pearl.flip {
    animation: flip 300ms ease-out both;
  }

  @keyframes flip {
    from {
      transform: rotateY(90deg);
    }
  }

  .final {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 16px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .today {
    color: var(--reward-ink);
    font-weight: 650;
  }

  .streak {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
    font-weight: 600;
  }
</style>
