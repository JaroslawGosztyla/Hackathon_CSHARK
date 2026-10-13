<script lang="ts">
  // Monday..Sunday of the current reward week: a goal day is a spark, a rest day a small "z", a day off a dash, a quiet
  // day a grey empty dot (never red), today an outline, the coming days faint.
  import { intlLocale } from "$lib/i18n/index.svelte";
  import { strings } from "$lib/strings";
  import type { DayCell } from "$lib/types";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    week: readonly DayCell[];
  }

  let { week }: Props = $props();

  /** `YYYY-MM-DD` at noon UTC, formatted in UTC: the weekday of that civil date in every time zone. */
  function civil(date: string): Date | null {
    const [y, m, d] = date.split("-").map(Number);
    if (!y || !m || !d) return null;
    return new Date(Date.UTC(y, m - 1, d, 12));
  }

  const formats = $derived.by(() => {
    const locale = intlLocale();
    return {
      narrow: new Intl.DateTimeFormat(locale, { weekday: "narrow", timeZone: "UTC" }),
      long: new Intl.DateTimeFormat(locale, { weekday: "long", timeZone: "UTC" }),
    };
  });

  const cells = $derived(
    week.map((cell) => {
      const date = civil(cell.date);
      return {
        ...cell,
        letter: date ? formats.narrow.format(date) : "",
        name: date ? formats.long.format(date) : cell.date,
      };
    }),
  );
</script>

<ol class="week" aria-label={strings.rewards.weekAria}>
  {#each cells as cell (cell.date)}
    <li class="day mark-{cell.mark}" aria-label="{cell.name}: {strings.rewards.dayMark[cell.mark]}">
      <span class="letter" aria-hidden="true">{cell.letter}</span>
      <span class="dot" aria-hidden="true">
        {#if cell.mark === "goal"}
          <SparkIcon size={12} />
        {:else if cell.mark === "rest"}
          <span class="zz">z</span>
        {:else if cell.mark === "neutral"}
          <span class="dash"></span>
        {:else}
          <span class="circle"></span>
        {/if}
      </span>
    </li>
  {/each}
</ol>

<style>
  .week {
    display: inline-flex;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .day {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    width: 20px;
  }

  .letter {
    font-size: 0.68rem;
    font-weight: 600;
    line-height: 1.2;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .mark-today .letter {
    color: var(--reward-ink);
  }

  .dot {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
  }

  .circle {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid var(--quiet-mark);
  }

  .mark-today .circle {
    width: 12px;
    height: 12px;
    border: 2px solid var(--reward);
  }

  /* Coming days: dotted (the same ≥ 3:1 grey; no extra transparency). */
  .mark-future .circle {
    border-style: dotted;
  }

  .dash {
    width: 10px;
    height: 2px;
    border-radius: 1px;
    background: var(--quiet-mark);
  }

  .zz {
    color: var(--bubble);
    font-size: 0.78rem;
    font-weight: 800;
    font-style: italic;
    line-height: 1;
  }
</style>
