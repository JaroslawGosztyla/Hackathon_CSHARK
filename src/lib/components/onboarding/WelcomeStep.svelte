<script lang="ts">
  // First-run wizard, step 1 (ADR-023): the app language. "System" (the default) is preselected; a choice is saved like
  // the Settings language select (`config.language`) and the app layout switches every text at once.
  import { onMount } from "svelte";
  import {
    isUiLanguage,
    knownSystemLanguage,
    LANGUAGE_NAMES,
    LANGUAGES,
    loadSystemLanguage,
    strings,
  } from "$lib/i18n/index.svelte";
  import { config } from "$lib/state/config.svelte";
  import type { Language } from "$lib/types";

  const uid = $props.id();
  const CHOICES: readonly Language[] = ["system", ...LANGUAGES];

  const current = $derived<Language>(config.value?.language ?? "system");
  /** The OS UI language for the "System (…)" choice (native name). */
  const systemName = $derived.by(() => {
    const lang = knownSystemLanguage();
    return lang ? LANGUAGE_NAMES[lang] : null;
  });

  onMount(() => {
    void loadSystemLanguage();
  });

  function label(choice: Language): string {
    return choice === "system" ? strings.settings.languageSystem(systemName) : LANGUAGE_NAMES[choice];
  }

  /** `lang` of a native language name (CJK glyphs need the right script). */
  function langOf(choice: Language): string | undefined {
    if (!isUiLanguage(choice)) return undefined;
    return choice === "zh" ? "zh-Hans" : choice;
  }

  function choose(choice: Language): void {
    if (choice === "system" || isUiLanguage(choice)) config.update((c) => (c.language = choice), true);
  }
</script>

<fieldset class="card languages">
  <legend class="field-label">{strings.onboarding.welcome.languageLabel}</legend>
  <div class="grid">
    {#each CHOICES as choice (choice)}
      <label class="choice" class:on={current === choice} class:system={choice === "system"} lang={langOf(choice)}>
        <input
          type="radio"
          name="{uid}-language"
          value={choice}
          checked={current === choice}
          onchange={() => choose(choice)}
        />
        <span>{label(choice)}</span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
  .languages {
    margin: 0;
    min-width: 0;
  }

  /* A real <legend> inside the card: drawn as the card's first line. */
  .languages legend {
    float: left;
    width: 100%;
    margin-bottom: 12px;
    padding: 0;
  }

  .grid {
    clear: both;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }

  /* "System (…)" first, across the row: it is the default and usually the right one. */
  .choice.system {
    grid-column: 1 / -1;
  }

  .choice {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 9px 12px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--border);
    background: var(--surface-inset);
    color: var(--text-2);
    font-weight: 600;
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      color 0.15s ease,
      background-color 0.15s ease;
  }

  .choice:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .choice.on {
    border-color: var(--accent-line);
    background: var(--accent-soft);
    color: var(--text);
  }

  .choice input {
    width: 16px;
    height: 16px;
    margin: 0;
    flex: none;
    cursor: pointer;
  }

  .choice span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .choice:has(input:focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .choice input:focus-visible {
    outline: none;
  }

  :global([data-theme="pixel"]) .choice:has(input:focus-visible) {
    outline-style: dashed;
    outline-width: 3px;
  }

  @media (prefers-reduced-motion: reduce) {
    .choice {
      transition: none;
    }
  }
</style>
