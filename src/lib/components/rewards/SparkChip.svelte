<script lang="ts">
  // Focus: today's sparks as a static "✦ 37" next to the "focused so far" line. It never animates during focus.
  import { strings } from "$lib/strings";
  import SparkIcon from "./SparkIcon.svelte";

  interface Props {
    sparks: number;
  }

  let { sparks }: Props = $props();
</script>

<span class="spark-chip tabular" role="img" aria-label={strings.rewards.todayChipAria(sparks)}>
  <SparkIcon size={12} />
  <span aria-hidden="true">{sparks}</span>
</span>

<style>
  .spark-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    border-radius: var(--radius-pill);
    background: var(--reward-soft);
    color: var(--reward-ink);
    font-size: 0.82rem;
    font-weight: 700;
    line-height: 1.6;
    vertical-align: 1px;
  }
</style>
