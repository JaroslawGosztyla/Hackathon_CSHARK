<script lang="ts">
  // One layer of the ambience mixer (ADR-024): a switch and its volume while the layer may play (free or unlocked),
  // else "?" with how it is earned (the same texts as the theme cards; a seasonal sound says "Razem z motywem …").
  import { ambienceUnlock, mixOf } from "$lib/ambience/catalog";
  import { conditionLines } from "$lib/components/collection/unlockText";
  import { intlLocale } from "$lib/i18n/index.svelte";
  import { ambience } from "$lib/state/ambience.svelte";
  import { strings } from "$lib/strings";
  import type { AmbienceLayer } from "$lib/types";

  let { layer, sounding }: { layer: AmbienceLayer; sounding: boolean } = $props();

  const uid = $props.id();
  const c = strings.collection;

  const name = $derived(c.ambienceNames[layer]);
  const usable = $derived(ambience.usable(layer));
  const mix = $derived(mixOf(ambience.cfg, layer));
  const pct = $derived(Math.round(mix.volume * 100));
  const lines = $derived.by(() => {
    if (usable) return [];
    const item = ambienceUnlock(ambience.shownUnlocks, layer);
    return item ? conditionLines(item, c, intlLocale(), ambience.shownUnlocks) : [];
  });

  function onVolume(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.valueAsNumber;
    if (Number.isFinite(value)) ambience.setLayer(layer, { volume: value / 100 });
  }
</script>

{#if usable}
  <li class="row" class:on={mix.on}>
    <input
      id="{uid}-on"
      type="checkbox"
      role="switch"
      class="switch"
      checked={mix.on}
      onchange={(e) => {
        const input = e.currentTarget;
        ambience.setLayer(layer, { on: input.checked }, true);
        input.checked = mixOf(ambience.cfg, layer).on;
      }}
    />
    <label class="name" for="{uid}-on">
      {name}
      {#if sounding}<span class="dot" aria-hidden="true"></span>{/if}
    </label>
    <input
      class="volume"
      type="range"
      min="0"
      max="100"
      step="1"
      value={pct}
      aria-label={c.mixer.layerVolume(name)}
      aria-valuetext="{pct}%"
      oninput={onVolume}
    />
  </li>
{:else}
  <li class="row locked" aria-label={c.lockedAria(name, lines.join(". "))}>
    <span class="mark" aria-hidden="true">?</span>
    <span class="text" aria-hidden="true">
      <span class="name">{name}</span>
      <span class="cond">{lines.length > 0 ? lines.join(" · ") : c.mixer.locked}</span>
    </span>
  </li>
{/if}

<style>
  .row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) minmax(80px, 120px);
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 4px 0;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
  }

  .row:not(.on) .name {
    color: var(--text-2);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .volume {
    width: 100%;
  }

  /* A compact switch (the settings Switch, smaller for the mixer). */
  .switch {
    flex: none;
    appearance: none;
    position: relative;
    width: 38px;
    height: 22px;
    margin: 0;
    border-radius: 11px;
    background: color-mix(in srgb, var(--text) 14%, transparent);
    border: 1px solid var(--border-strong);
    cursor: pointer;
    transition: background-color 0.18s ease;
  }

  .switch::before {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-2);
    transition:
      transform 0.18s ease,
      background-color 0.18s ease;
  }

  .switch:checked {
    background: var(--accent);
    border-color: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(16px);
    background: var(--accent-ink);
  }

  .switch:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .switch,
    .switch::before {
      transition: none;
    }
  }

  .locked {
    grid-template-columns: auto minmax(0, 1fr);
  }

  .locked .name {
    cursor: default;
    color: var(--text-2);
  }

  .mark {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: 2px dashed var(--border-strong);
    color: var(--text-2);
    font-weight: 750;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .cond {
    font-size: 0.8rem;
    line-height: 1.35;
    color: var(--text-muted);
  }
</style>
