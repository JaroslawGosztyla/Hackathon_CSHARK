<script lang="ts">
  import { config, LIMITS } from "$lib/state/config.svelte";
  import { strings } from "$lib/strings";

  /** Focus / break presets in minutes (product spec: 25/5, 45/15 default, 50/10, 90/20). */
  const PRESETS: ReadonlyArray<readonly [number, number]> = [
    [25, 5],
    [45, 15],
    [50, 10],
    [90, 20],
  ];

  const cfg = $derived(config.value);
  const focusError = $derived(config.fieldErrors.focusMinutes ?? null);
  const breakError = $derived(config.fieldErrors.breakMinutes ?? null);

  const shown = (n: number | undefined) => (n !== undefined && Number.isFinite(n) ? n : null);

  function setFocus(event: Event & { currentTarget: HTMLInputElement }) {
    const n = event.currentTarget.valueAsNumber;
    config.update((c) => {
      c.focusMinutes = n;
    });
  }

  function setBreak(event: Event & { currentTarget: HTMLInputElement }) {
    const n = event.currentTarget.valueAsNumber;
    config.update((c) => {
      c.breakMinutes = n;
    });
  }

  function applyPreset(focus: number, brk: number) {
    config.update((c) => {
      c.focusMinutes = focus;
      c.breakMinutes = brk;
    }, true);
  }
</script>

<section class="card" aria-labelledby="timing-title">
  <div class="card-title">
    <h2 id="timing-title">{strings.timing.title}</h2>
  </div>

  <div class="inputs">
    <div class="field">
      <label for="focus-minutes">{strings.timing.focus}</label>
      <input
        id="focus-minutes"
        class="input tabular"
        type="number"
        inputmode="numeric"
        min={LIMITS.focusMinutes.min}
        max={LIMITS.focusMinutes.max}
        step="1"
        value={shown(cfg?.focusMinutes)}
        disabled={!cfg}
        aria-invalid={focusError ? "true" : undefined}
        aria-describedby={focusError ? "focus-minutes-error" : undefined}
        oninput={setFocus}
      />
      {#if focusError}<p class="field-error" id="focus-minutes-error">{focusError}</p>{/if}
    </div>
    <div class="field">
      <label for="break-minutes">{strings.timing.break}</label>
      <input
        id="break-minutes"
        class="input tabular"
        type="number"
        inputmode="numeric"
        min={LIMITS.breakMinutes.min}
        max={LIMITS.breakMinutes.max}
        step="1"
        value={shown(cfg?.breakMinutes)}
        disabled={!cfg}
        aria-invalid={breakError ? "true" : undefined}
        aria-describedby={breakError ? "break-minutes-error" : undefined}
        oninput={setBreak}
      />
      {#if breakError}<p class="field-error" id="break-minutes-error">{breakError}</p>{/if}
    </div>
  </div>

  <div class="presets" role="group" aria-label={strings.timing.presets}>
    {#each PRESETS as [focus, brk] (`${focus}-${brk}`)}
      <button
        type="button"
        class="preset tabular"
        aria-pressed={cfg?.focusMinutes === focus && cfg?.breakMinutes === brk}
        aria-label={strings.timing.presetAria(focus, brk)}
        disabled={!cfg}
        onclick={() => applyPreset(focus, brk)}
      >
        {strings.timing.preset(focus, brk)}
      </button>
    {/each}
  </div>
</section>

<style>
  .inputs {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .inputs .input {
    font-size: 1.3rem;
    font-weight: 600;
    padding: 8px 12px;
  }

  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 16px;
  }

  .preset {
    flex: 1 1 70px;
    min-height: 38px;
    padding: 6px 12px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border-strong);
    background: transparent;
    color: var(--text-2);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      color 0.15s ease;
  }

  .preset:hover:not(:disabled) {
    background: var(--surface-inset);
    color: var(--text);
  }

  .preset[aria-pressed="true"] {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent-strong);
  }

  .preset:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
