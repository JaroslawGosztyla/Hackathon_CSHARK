<script lang="ts">
  // The ambience mixer (beta, ADR-024): a non-modal popover above the music dock. The master switch ("with the
  // music"), the master volume, "Graj same" / "Zatrzymaj" and one row per layer in catalogue order. Esc or a click
  // outside closes it (focus goes back to the dock button). Writes `Config.ambience` (sliders debounced); the sound
  // follows at once.
  import { tick } from "svelte";
  import { AMBIENCE_LAYERS } from "$lib/ambience/catalog";
  import { ambienceEngine } from "$lib/ambience/engine.svelte";
  import { ambience } from "$lib/state/ambience.svelte";
  import { strings } from "$lib/strings";
  import AmbienceRow from "./AmbienceRow.svelte";

  let { id, anchor }: { id: string; anchor: HTMLElement | null } = $props();

  const uid = $props.id();
  const c = strings.collection;

  let panel = $state<HTMLDivElement | null>(null);

  const cfg = $derived(ambience.cfg);
  const masterPct = $derived(Math.round(cfg.volume * 100));
  /** Something of the user's mix sounds now (not a Developer sample). */
  const sounding = $derived(ambienceEngine.sample === null && ambienceEngine.sounding.length > 0);
  const usableCount = $derived(AMBIENCE_LAYERS.filter((layer) => ambience.usable(layer)).length);

  function close(returnFocus = true): void {
    ambience.mixerOpen = false;
    if (returnFocus) anchor?.focus();
  }

  // Opened: focus the first control. Esc and outside clicks close it.
  $effect(() => {
    if (!ambience.mixerOpen) return;
    void tick().then(() => (panel?.querySelector<HTMLElement>("input") ?? panel?.querySelector<HTMLElement>("button"))?.focus());
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        event.preventDefault();
        close();
      }
    };
    const onPointer = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target || panel?.contains(target) || anchor?.contains(target)) return;
      close(false);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointer, true);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer, true);
    };
  });

  function onMaster(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.valueAsNumber;
    if (Number.isFinite(value)) ambience.setVolume(value / 100);
  }
</script>

{#if ambience.mixerOpen}
  <div class="mixer" {id} role="dialog" aria-labelledby="{uid}-title" bind:this={panel}>
    <div class="head">
      <h2 id="{uid}-title">{c.mixer.title}</h2>
      <button type="button" class="icon-btn close" aria-label={c.mixer.close} title={c.mixer.close} onclick={() => close()}>
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
          <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </button>
    </div>
    <p class="hint">{c.mixer.hint}</p>

    <div class="master">
      <input
        id="{uid}-master"
        type="checkbox"
        role="switch"
        class="switch"
        checked={cfg.enabled}
        onchange={(e) => {
          const input = e.currentTarget;
          ambience.setEnabled(input.checked);
          input.checked = ambience.cfg.enabled;
        }}
      />
      <label for="{uid}-master">{c.mixer.master}</label>
    </div>

    <div class="controls">
      {#if sounding}
        <button type="button" class="btn btn-sm" onclick={() => ambience.stop()}>
          <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
            <rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" />
          </svg>
          {c.mixer.stop}
        </button>
      {:else}
        <button type="button" class="btn btn-sm btn-primary" disabled={ambience.layers.length === 0} onclick={() => ambience.playAlone()}>
          <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
            <path d="M7 4.5v15l12.5-7.5z" fill="currentColor" />
          </svg>
          {c.mixer.playAlone}
        </button>
      {/if}
      <label class="volume">
        <span class="sr-only">{c.mixer.volume}</span>
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
          <path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" />
        </svg>
        <input type="range" min="0" max="100" step="1" value={masterPct} aria-valuetext="{masterPct}%" oninput={onMaster} />
      </label>
    </div>
    {#if sounding}<p class="status" role="status">{c.mixer.playing}</p>{/if}

    <ul class="layers">
      {#each AMBIENCE_LAYERS as layer (layer)}
        <AmbienceRow {layer} sounding={ambienceEngine.sample === null && ambienceEngine.sounding.includes(layer)} />
      {/each}
    </ul>
    {#if usableCount < AMBIENCE_LAYERS.length}
      <p class="hint more">{c.mixer.lockedHint}</p>
    {/if}
  </div>
{/if}

<style>
  .mixer {
    position: absolute;
    right: max(var(--gutter), calc((100vw - 1040px) / 2));
    bottom: calc(100% + 8px);
    z-index: 32;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(380px, calc(100vw - 2 * var(--gutter)));
    max-height: min(70vh, 600px);
    overflow-y: auto;
    padding: 14px 16px;
    border-radius: 18px;
    background: var(--surface-solid);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    pointer-events: auto;
    color: var(--text);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .close {
    width: 30px;
    height: 30px;
  }

  .master {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 600;
  }

  .master label {
    cursor: pointer;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .volume {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 140px;
    color: var(--text-2);
  }

  .volume input {
    flex: 1;
    min-width: 0;
  }

  .status {
    margin: 0;
    font-size: 0.82rem;
    color: var(--text-2);
  }

  .layers {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 6px 0 0;
    list-style: none;
    border-top: 1px solid var(--border);
  }

  .more {
    margin: 0;
  }

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
</style>
