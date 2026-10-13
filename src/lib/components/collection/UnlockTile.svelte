<script lang="ts">
  // The unlock beat of a recap (ADR-024): the new theme's live tile, "Nowy motyw: Halloween" and a small "Wypróbuj"
  // (sets the collection theme; a sample previews it instead). Never modal, never in focus (the moment waits for the
  // break or Stop like every achievement). `still`: a static tile (calm celebrations, reduced motion).
  import { config } from "$lib/state/config.svelte";
  import { dev } from "$lib/state/dev.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import { effectiveTheme } from "$lib/themes";
  import type { RewardTheme } from "$lib/types";
  import ThemeTile from "./ThemeTile.svelte";

  interface Props {
    theme: RewardTheme;
    /** The moment is a Developer sample: "Wypróbuj" previews, never saves. */
    sample: boolean;
    still: boolean;
    /** The tile moves in now (the beat is playing). */
    entering?: boolean;
  }

  let { theme, sample, still, entering = false }: Props = $props();

  const c = strings.collection;

  const onScreen = $derived.by(() => {
    if (dev.themePreview !== null) return dev.themePreview === theme;
    const cfg = config.value;
    return cfg ? effectiveTheme(cfg, rewards.real?.unlocks) === theme : false;
  });

  function tryIt(): void {
    if (sample) {
      dev.previewRewardTheme(theme, c.themeNames[theme]);
      return;
    }
    config.update((cfg) => (cfg.rewardTheme = theme), true);
  }
</script>

<div class="unlock" class:entering={entering && !still}>
  <span class="tile"><ThemeTile {theme} {still} /></span>
  <span class="body">
    <span class="title">{c.newTheme(c.themeNames[theme])}</span>
    <span class="desc">{c.themeDescriptions[theme]}</span>
    {#if onScreen}
      <span class="badge">{c.inUse}</span>
    {:else}
      <button type="button" class="btn btn-sm" disabled={!config.ready} onclick={tryIt}>{c.tryIt}</button>
    {/if}
  </span>
</div>

<style>
  .unlock {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--reward-line);
    background: var(--reward-soft);
  }

  .entering {
    animation: tile-in 360ms ease-out both;
  }

  @keyframes tile-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.97);
    }
  }

  .tile {
    flex: none;
    width: min(168px, 40%);
  }

  .body {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
  }

  .title {
    font-weight: 700;
    color: var(--reward-ink);
  }

  .desc {
    font-size: 0.84rem;
    line-height: 1.4;
    color: var(--text-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .entering {
      animation: none;
    }
  }
</style>
