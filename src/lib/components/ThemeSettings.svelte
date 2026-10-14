<script lang="ts">
  // Theme picker: a radio group of live previews. Each preview carries its own `data-theme`, so it is drawn with
  // the real tokens of the theme it shows (app.css). Picking one saves `config.theme` at once; the switch
  // crossfades (View Transitions) unless the user prefers reduced motion.
  // Beta (ADR-024): below the base themes the "Kolekcja" grid (CollectionGrid) adds the collection themes to the same
  // radio group. Picking an unlocked one saves `config.rewardTheme` (the base theme stays as the fallback, marked
  // "Motyw podstawowy"); picking a base theme clears it. Without the beta the grid is not rendered at all.
  import { flushSync } from "svelte";
  import { MediaQuery } from "svelte/reactivity";
  import CollectionGrid from "$lib/components/collection/CollectionGrid.svelte";
  import { config } from "$lib/state/config.svelte";
  import { dev } from "$lib/state/dev.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { showcaseMode } from "$lib/state/showcaseMode.svelte";
  import { strings } from "$lib/strings";
  import {
    applyTheme,
    DEFAULT_THEME,
    effectiveTheme,
    isRewardTheme,
    isTheme,
    isThemeId,
    THEMES,
    type Theme,
    type ThemeId,
  } from "$lib/themes";
  import { litPath, moonPhase } from "$lib/themes/moon";
  import { themeStrings as t } from "$lib/themes/strings";

  const uid = $props.id();
  const reducedMotion = new MediaQuery("(prefers-reduced-motion: reduce)");

  /** The user's base theme (the fallback of a collection theme). */
  const base = $derived.by((): Theme => {
    const configured = config.value?.theme;
    return isTheme(configured) ? configured : DEFAULT_THEME;
  });

  /** The theme on screen: a Developer collection sample, else the config's (`effectiveTheme`). */
  const current = $derived.by((): ThemeId => {
    if (dev.themePreview !== null) return dev.themePreview;
    const cfg = config.value;
    if (!cfg) return base;
    const shown = effectiveTheme(cfg, rewards.real?.unlocks);
    return isThemeId(shown) ? shown : base;
  });
  /** A collection theme is on screen: the base card shows the "Motyw podstawowy" badge instead of being selected. */
  const collectionShown = $derived(isRewardTheme(current));

  const moon = moonPhase(new Date());
  const moonLit = litPath(moon.fraction, 12, 12, 9);

  function description(theme: Theme): string {
    if (theme !== "lunar") return t.descriptions[theme];
    const lunarDescription = t.descriptions.lunar;
    return `${lunarDescription} ${t.moonToday(t.moonPhases[moon.name], Math.round(moon.illumination * 100))}`;
  }

  /** The crossfade waits for a frame; the choice is saved after this delay even if no frame comes. */
  const COMMIT_TIMEOUT_MS = 300;

  function choose(theme: ThemeId): void {
    if (theme === current || !config.value) return;
    let committed = false;
    const commit = () => {
      if (committed) return;
      committed = true;
      if (isRewardTheme(theme)) {
        // A Developer sample collection never saves anything: the pick previews the theme instead. So does the
        // presentation showcase (ADR-019): its collection is the seeded ledger's, not the real one.
        if (rewards.sampleStatus !== null || showcaseMode.active) {
          dev.previewRewardTheme(theme, strings.collection.themeNames[theme]);
        } else {
          config.update((c) => (c.rewardTheme = theme), true);
        }
      } else {
        if (dev.themePreview !== null) dev.themePreview = null;
        config.update((c) => {
          c.theme = theme;
          c.rewardTheme = null;
        }, true);
      }
      applyTheme(theme);
    };
    const animate =
      !reducedMotion.current &&
      typeof document.startViewTransition === "function" &&
      document.visibilityState === "visible";
    if (!animate) {
      commit();
      return;
    }
    try {
      const transition = document.startViewTransition(() => flushSync(commit));
      transition.ready.catch(() => {});
      transition.updateCallbackDone.catch(() => {});
      transition.finished.catch(() => {});
      setTimeout(commit, COMMIT_TIMEOUT_MS);
    } catch {
      commit();
    }
  }
</script>

<fieldset class="themes" disabled={!config.ready}>
  <legend class="sr-only">{t.legend}</legend>
  <p class="hint">{t.hint}</p>

  {#if rewards.betaOn}<h3 class="group-title">{strings.collection.baseTitle}</h3>{/if}
  <div class="grid">
    {#each THEMES as theme (theme)}
      <label class="option" class:selected={theme === current} class:fallback={collectionShown && theme === base}>
        <input
          class="sr-only"
          type="radio"
          name="{uid}-theme"
          value={theme}
          checked={theme === current}
          aria-labelledby="{uid}-{theme}-name"
          aria-describedby="{uid}-{theme}-desc"
          onchange={() => choose(theme)}
        />

        <span class="preview" data-theme={theme} aria-hidden="true">
          {#if theme === "midnight"}
            <span class="mn-moon"></span>
            <span class="mn-rain"></span>
          {:else if theme === "vaporwave"}
            <span class="vw-sun"></span>
            <span class="vw-grid"></span>
          {:else if theme === "lunar"}
            <svg class="ln-moon" viewBox="0 0 24 24" focusable="false">
              <circle cx="12" cy="12" r="9" class="ln-dark" />
              <path d={moonLit} class="ln-lit" />
            </svg>
            <span class="ln-stars"></span>
          {:else if theme === "autumn"}
            <svg class="au-leaves" viewBox="0 0 48 32" focusable="false">
              <path d="M8 4c3 2.7 4.2 5 4.2 7.2a4.2 4.2 0 0 1-8.4 0C3.8 9 5 6.7 8 4z" fill="#e08a2e" />
              <path d="M30 12c3 2.7 4.2 5 4.2 7.2a4.2 4.2 0 0 1-8.4 0c0-2.2 1.2-4.5 4.2-7.2z" fill="#c8641e" transform="rotate(35 30 18)" />
              <path d="M42 2c2.4 2.2 3.4 4 3.4 5.8a3.4 3.4 0 0 1-6.8 0c0-1.8 1-3.6 3.4-5.8z" fill="#d9a03b" transform="rotate(-25 42 7)" />
            </svg>
          {:else if theme === "pixel"}
            <span class="px-stars"></span>
            <span class="px-moon"></span>
            <span class="px-hills"></span>
          {/if}

          <span class="mini-nav"><span class="mini-highlight"></span></span>
          <span class="mini-card">
            <span class="mini-title">Aa</span>
            <span class="mini-line"></span>
            <span class="mini-pills"><span class="pill accent"></span><span class="pill break"></span></span>
          </span>
        </span>

        <span class="text">
          <span class="name" id="{uid}-{theme}-name">
            {t.names[theme]}
            {#if theme === current}
              <svg class="check" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" focusable="false">
                <path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            {/if}
          </span>
          <span class="desc" id="{uid}-{theme}-desc">{description(theme)}</span>
          {#if collectionShown && theme === base}
            <span class="badge fallback-badge" title={strings.collection.baseBadgeHint}>{strings.collection.baseBadge}</span>
          {/if}
        </span>
      </label>
    {/each}
  </div>

  {#if rewards.betaOn}
    <CollectionGrid group="{uid}-theme" {current} disabled={!config.ready} onchoose={(theme) => choose(theme)} />
  {/if}
</fieldset>

<style>
  .themes {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    container-type: inline-size;
  }

  /* Four per row on a wide card (4 + 3), two per row on a phone. */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }

  @container (max-width: 440px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }
  }

  .option {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 8px 8px 12px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--border);
    background: var(--surface-inset);
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      background-color 0.15s ease,
      box-shadow 0.15s ease;
  }

  .option:hover {
    border-color: var(--border-hover);
    background: var(--surface-hover);
  }

  .option.selected {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  /* The base theme behind a collection theme: marked, not selected. */
  .option.fallback {
    border-style: dashed;
    border-color: var(--accent-line);
  }

  .group-title {
    margin: 0;
    font-size: 1rem;
  }

  .fallback-badge {
    align-self: flex-start;
    margin-top: 4px;
  }

  .option:has(input:focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .themes:disabled .option {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 4px;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 650;
    color: var(--text);
  }

  .check {
    color: var(--accent-strong);
  }

  .desc {
    font-size: 0.82rem;
    line-height: 1.4;
    color: var(--text-muted);
  }

  /* ---------- the preview: drawn with the tokens of its own data-theme ---------- */

  .preview {
    position: relative;
    display: block;
    aspect-ratio: 16 / 10;
    overflow: hidden;
    border-radius: max(0px, calc(var(--radius-sm) - 4px));
    border: 1px solid var(--border);
    background: var(--app-bg);
    background-color: var(--bg-0);
    color: var(--text);
    font-family: var(--font);
  }

  .mini-nav {
    position: absolute;
    top: 9%;
    left: 8%;
    width: 42%;
    height: 12%;
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .mini-highlight {
    display: block;
    width: 38%;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--nav-highlight);
  }

  .mini-card {
    position: absolute;
    left: 8%;
    bottom: 10%;
    width: 56%;
    height: 54%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: max(0px, calc(var(--radius) / 3));
    background: var(--surface);
    border: var(--border-w) solid var(--border);
    box-shadow: var(--shadow-sm);
  }

  .mini-title {
    font-size: 0.8rem;
    font-weight: 700;
    line-height: 1;
    color: var(--text);
  }

  .mini-line {
    display: block;
    width: 80%;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--text-muted);
    opacity: 0.7;
  }

  .mini-pills {
    display: flex;
    gap: 4px;
  }

  .pill {
    display: block;
    width: 28%;
    height: 8px;
    border-radius: var(--radius-pill);
  }

  .pill.accent {
    background: var(--accent);
  }

  .pill.break {
    background: var(--break);
  }

  /* Midnight: warm moon glow and a little rain. */
  .mn-moon {
    position: absolute;
    top: 10%;
    right: 10%;
    width: 22%;
    aspect-ratio: 1;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 38%, #ffe8c4 0%, #f6c98d 45%, rgba(246, 201, 141, 0) 72%);
    opacity: 0.4;
  }

  .mn-rain {
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(
      100deg,
      transparent 0 14px,
      rgba(220, 210, 255, 0.16) 14px 15px
    );
  }

  /* Vaporwave: striped sun on the horizon and the neon floor. */
  .vw-sun {
    position: absolute;
    right: 8%;
    bottom: 38%;
    width: 32%;
    aspect-ratio: 2 / 1;
    border-radius: 999px 999px 0 0;
    background: linear-gradient(180deg, #fffb96 0%, #ffd27f 35%, #ff9eb5 70%, #ff71ce 100%);
    mask-image: linear-gradient(180deg, #000 0 55%, transparent 55% 63%, #000 63% 76%, transparent 76% 86%, #000 86%);
  }

  .vw-grid {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 38%;
    border-top: 1px solid #ff71ce;
    background-image:
      linear-gradient(90deg, rgba(255, 113, 206, 0.55) 1px, transparent 1px),
      linear-gradient(180deg, rgba(1, 205, 254, 0.5) 1px, transparent 1px);
    background-size: 12px 8px;
    opacity: 0.7;
  }

  /* Lunar: today's phase, a few stars. */
  .ln-moon {
    position: absolute;
    top: 8%;
    right: 8%;
    width: 26%;
    height: auto;
    aspect-ratio: 1;
    filter: drop-shadow(0 0 6px rgba(214, 222, 255, 0.45));
  }

  .ln-dark {
    fill: rgba(170, 182, 230, 0.16);
  }

  .ln-lit {
    fill: #e9ecfb;
  }

  .ln-stars,
  .px-stars {
    position: absolute;
    inset: 0;
    background-image:
      radial-gradient(circle, rgba(238, 241, 255, 0.8) 0 1px, transparent 1.5px),
      radial-gradient(circle, rgba(238, 241, 255, 0.6) 0 1px, transparent 1.5px);
    background-size:
      37px 29px,
      53px 41px;
    background-position:
      5px 7px,
      21px 17px;
  }

  /* Autumn: a few falling leaves. */
  .au-leaves {
    position: absolute;
    top: 6%;
    right: 6%;
    width: 40%;
    height: auto;
    opacity: 0.85;
  }

  /* 8-bit: square stars, a block moon, stepped hills. */
  .px-stars {
    background-image:
      linear-gradient(#ffffff, #ffffff),
      linear-gradient(#fcfc54, #fcfc54),
      linear-gradient(#7ce8ff, #7ce8ff);
    background-size: 3px 3px;
    background-repeat: no-repeat;
    background-position:
      62% 14%,
      82% 40%,
      48% 30%;
  }

  .px-moon {
    position: absolute;
    top: 10%;
    right: 9%;
    width: 16%;
    aspect-ratio: 1;
    background: #f8f0c8;
    box-shadow: inset -4px -4px 0 0 #c8b878;
  }

  .px-hills {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 26%;
    background: #120f33;
    clip-path: polygon(
      0 40%,
      12% 40%,
      12% 20%,
      26% 20%,
      26% 50%,
      40% 50%,
      40% 10%,
      56% 10%,
      56% 35%,
      72% 35%,
      72% 0,
      86% 0,
      86% 30%,
      100% 30%,
      100% 100%,
      0 100%
    );
  }
</style>
