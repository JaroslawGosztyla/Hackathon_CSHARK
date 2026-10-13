<script lang="ts">
  // "+12 ✦ Start!" over the timer ring: rises 24 px and fades (Calm / reduced motion: a still chip for the same time;
  // 8-bit: 4 px steps). For Start and back from a break, and for a ticked step when no "Teraz:" line shows it.
  // Absolutely positioned, never takes clicks, never covers the session buttons.
  import { FLOAT_MS, FLOAT_RISE_PX, PIXEL_FLOAT_STEP } from "$lib/rewards/choreography";
  import type { RewardPlayer } from "$lib/rewards/player.svelte";
  import { strings } from "$lib/strings";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    player: RewardPlayer;
  }

  let { player }: Props = $props();

  const info = $derived.by(() => {
    const current = player.current;
    if (!current) return null;
    const kind = current.moment.kind;
    if (kind === "start" || kind === "backFromBreak") return current;
    if ((kind === "step" || kind === "allSteps") && player.stepHosts === 0) return current;
    return null;
  });
  const firstOfDay = $derived(info?.moment.lines.some((line) => line.kind === "firstOfDay") ?? false);
</script>

{#if info}
  {#key info.moment.id}
    <div
      class="float-spark"
      class:still={info.still}
      class:pixel={info.pixel}
      style:--float-ms="{FLOAT_MS}ms"
      style:--rise="{-FLOAT_RISE_PX}px"
      style:--pixel-steps={Math.round(FLOAT_RISE_PX / PIXEL_FLOAT_STEP)}
      aria-hidden="true"
    >
      {#if info.moment.sparks > 0}<span class="amount">+{info.moment.sparks} <SparkIcon size={16} /></span>{/if}
      <span class="caption">{info.caption}</span>
      {#if firstOfDay}<span class="day">{strings.rewards.dayOpened}</span>{/if}
    </div>
  {/key}
{/if}

<style>
  .float-spark {
    position: absolute;
    z-index: 2;
    left: 50%;
    top: 26%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    width: max-content;
    max-width: 240px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--surface-solid) 82%, transparent);
    color: var(--reward-ink);
    text-align: center;
    pointer-events: none;
    transform: translateX(-50%);
    animation: float-rise var(--float-ms) ease-out forwards;
  }

  .float-spark.pixel {
    animation-timing-function: steps(var(--pixel-steps), end);
  }

  /* Calm / reduced motion: the chip shows for the same time without moving. */
  .float-spark.still {
    animation: none;
  }

  .amount {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 1.35rem;
    font-weight: 750;
    line-height: 1.2;
  }

  .caption {
    font-size: 0.86rem;
    font-weight: 600;
    color: var(--text);
  }

  .day {
    padding: 0 8px;
    border-radius: var(--radius-pill);
    background: var(--reward-soft);
    font-size: 0.74rem;
    font-weight: 700;
  }

  @keyframes float-rise {
    0% {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
    15% {
      opacity: 1;
      transform: translate(-50%, 0);
    }
    70% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translate(-50%, var(--rise));
    }
  }
</style>
