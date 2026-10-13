<script lang="ts">
  // Warm screen colours by time of day: mechanism (Windows Night light or display gamma), schedule, strength with a
  // live preview, fade length and the status the shell reports (fallback mechanism, capped strength, errors).
  import { onDestroy } from "svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { minutesToTime, timeToMinutes } from "$lib/format";
  import { config, LIMITS, TINT_TRANSITION_PRESETS } from "$lib/state/config.svelte";
  import { tint } from "$lib/state/tint.svelte";
  import { strings } from "$lib/strings";
  import type { Config, TintMode } from "$lib/types";

  const MODES: readonly TintMode[] = ["nightLight", "gamma"];

  /** The preview follows the slider at most this often. */
  const PREVIEW_DEBOUNCE_MS = 150;
  /** A keyboard change keeps its preview this long after the last key, so it can be seen at all. */
  const KEYBOARD_PREVIEW_MS = 1500;

  const s = strings.settings;
  const cfg = $derived(config.value);
  const errors = $derived(config.fieldErrors);

  const transitionOptions = $derived.by((): number[] => {
    const presets: number[] = [...TINT_TRANSITION_PRESETS];
    const current = cfg?.tint.transitionMinutes;
    if (current !== undefined && !presets.includes(current)) presets.push(current);
    return presets.sort((a, b) => a - b);
  });

  const status = $derived.by((): { text: string; error: boolean; detail: string | null } | null => {
    const st = tint.status;
    if (!st) return null;
    // `error` with `active` is only a note (e.g. Night light cannot be controlled on this Windows, so the gamma
    // fallback warms the screen): the warmth works, the note below says which mechanism is used.
    if (st.error && !st.active) {
      return { text: st.level === 0 ? s.tintStatusRestoreError : s.tintStatusError, error: true, detail: st.error };
    }
    if (st.preview) return { text: s.tintStatusPreview(st.level), error: false, detail: st.error };
    if (!st.enabled || st.level === 0) return { text: s.tintStatusOff, error: false, detail: st.error };
    return { text: s.tintStatusLevel(st.level), error: false, detail: st.error };
  });

  /** Extra lines under the status: the shell fell back to the other mechanism, or Windows capped the strength. */
  const notes = $derived.by((): string[] => {
    const st = tint.status;
    const configured = cfg?.tint.mode;
    if (!st || !configured) return [];
    const out: string[] = [];
    if (st.mode !== configured) out.push(s.tintStatusFallback(s.tintModeShort[st.mode]));
    if (st.capped) out.push(s.tintStatusCapped);
    return out;
  });

  function setMode(value: string): void {
    const mode = MODES.find((m) => m === value);
    if (mode) set((c) => (c.tint.mode = mode));
  }

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }

  function onTime(event: Event & { currentTarget: HTMLInputElement }, apply: (c: Config, minutes: number) => void): void {
    // An incomplete time reads as "": keep the stored value until the input holds a whole time again.
    const minutes = timeToMinutes(event.currentTarget.value);
    if (minutes === null) return;
    config.update((c) => apply(c, minutes));
  }

  function onTransition(event: Event & { currentTarget: HTMLSelectElement }): void {
    const minutes = Number(event.currentTarget.value);
    if (Number.isInteger(minutes)) set((c) => (c.tint.transitionMinutes = minutes));
  }

  // ---------------------------------------------------------------------------
  // Strength slider with a live preview on the screen
  // ---------------------------------------------------------------------------

  let previewTimer: ReturnType<typeof setTimeout> | null = null;
  let endTimer: ReturnType<typeof setTimeout> | null = null;
  let previewing = false;
  let pointerDown = false;

  function clearTimers(): void {
    if (previewTimer !== null) clearTimeout(previewTimer);
    if (endTimer !== null) clearTimeout(endTimer);
    previewTimer = null;
    endTimer = null;
  }

  function onStrengthInput(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.valueAsNumber;
    if (!Number.isFinite(value)) return;
    const strength = Math.round(Math.min(LIMITS.tint.maxStrength, Math.max(0, value)));
    config.update((c) => (c.tint.strength = strength));
    clearTimers();
    previewTimer = setTimeout(() => {
      previewTimer = null;
      previewing = true;
      void tint.preview(strength);
    }, PREVIEW_DEBOUNCE_MS);
  }

  /** Released after dragging: end the preview now. Keyboard: end it shortly after the last key. */
  function onStrengthChange(): void {
    if (pointerDown) endPreview();
    else {
      if (endTimer !== null) clearTimeout(endTimer);
      endTimer = setTimeout(endPreview, KEYBOARD_PREVIEW_MS);
    }
  }

  function onPointerDown(): void {
    pointerDown = true;
  }

  function onPointerUp(): void {
    // `change` fires right after `pointerup` (same task) when the value moved; clear the flag after it.
    setTimeout(() => (pointerDown = false), 0);
  }

  function endPreview(): void {
    const pendingPreview = previewTimer !== null;
    clearTimers();
    if (previewing || pendingPreview) {
      previewing = false;
      void tint.preview(null);
    }
  }

  onDestroy(endPreview);
</script>

{#if cfg}
  <Switch
    id="tint-enabled"
    label={s.tint}
    hint={s.tintHint}
    checked={cfg.tint.enabled}
    onchange={(on) => set((c) => (c.tint.enabled = on))}
  >
    {#if cfg.tint.enabled}
      <div class="tint-fields">
        <fieldset class="modes">
          <legend class="field-label">{s.tintMode}</legend>
          {#each MODES as mode (mode)}
            <label class="mode" class:selected={cfg.tint.mode === mode}>
              <input
                type="radio"
                name="tint-mode"
                value={mode}
                checked={cfg.tint.mode === mode}
                aria-describedby="tint-mode-{mode}-hint"
                onchange={() => setMode(mode)}
              />
              <span class="mode-text">
                <span class="mode-name">{s.tintModes[mode]}</span>
                <span class="hint" id="tint-mode-{mode}-hint">{s.tintModeHints[mode]}</span>
              </span>
            </label>
          {/each}
        </fieldset>

        <div class="schedule">
          <div class="field">
            <label for="tint-start">{s.tintFrom}</label>
            <input
              id="tint-start"
              class="input time"
              type="time"
              step="60"
              value={minutesToTime(cfg.tint.startMinutes)}
              aria-invalid={errors["tint.startMinutes"] ? "true" : undefined}
              aria-describedby="tint-schedule-hint"
              oninput={(e) => onTime(e, (c, m) => (c.tint.startMinutes = m))}
            />
            {#if errors["tint.startMinutes"]}<p class="field-error">{errors["tint.startMinutes"]}</p>{/if}
          </div>
          <div class="field">
            <label for="tint-end">{s.tintTo}</label>
            <input
              id="tint-end"
              class="input time"
              type="time"
              step="60"
              value={minutesToTime(cfg.tint.endMinutes)}
              aria-invalid={errors["tint.endMinutes"] ? "true" : undefined}
              aria-describedby="tint-schedule-hint"
              oninput={(e) => onTime(e, (c, m) => (c.tint.endMinutes = m))}
            />
            {#if errors["tint.endMinutes"]}<p class="field-error">{errors["tint.endMinutes"]}</p>{/if}
          </div>
          <p class="hint schedule-hint" id="tint-schedule-hint">{s.tintScheduleHint}</p>
        </div>

        <div class="field">
          <label for="tint-strength">{s.tintStrength}</label>
          <div class="range-row">
            <input
              id="tint-strength"
              type="range"
              min="0"
              max={LIMITS.tint.maxStrength}
              step="1"
              value={cfg.tint.strength}
              aria-valuetext={`${cfg.tint.strength}%`}
              aria-describedby="tint-strength-hint"
              oninput={onStrengthInput}
              onchange={onStrengthChange}
              onpointerdown={onPointerDown}
              onpointerup={onPointerUp}
              onpointercancel={onPointerUp}
              onblur={endPreview}
            />
            <span class="tabular value">{cfg.tint.strength}%</span>
          </div>
          <p class="hint" id="tint-strength-hint">{s.tintStrengthHint}</p>
          {#if errors["tint.strength"]}<p class="field-error">{errors["tint.strength"]}</p>{/if}
        </div>

        <div class="field transition">
          <label for="tint-transition">{s.tintTransition}</label>
          <select
            id="tint-transition"
            class="input select"
            value={String(cfg.tint.transitionMinutes)}
            onchange={onTransition}
          >
            {#each transitionOptions as minutes (minutes)}
              <option value={String(minutes)}>
                {minutes === 0 ? s.tintTransitionNone : s.tintTransitionMinutes(minutes)}
              </option>
            {/each}
          </select>
          {#if errors["tint.transitionMinutes"]}<p class="field-error">{errors["tint.transitionMinutes"]}</p>{/if}
        </div>
      </div>
    {/if}

    {#if status && (cfg.tint.enabled || status.error || tint.status?.preview)}
      <div class="status-block" aria-live="polite">
        <p class="status" class:error={status.error} title={status.detail ?? undefined}>
          <span class="swatch" aria-hidden="true"></span>
          {status.text}
        </p>
        {#each notes as note (note)}
          <p class="hint note">{note}</p>
        {/each}
      </div>
    {/if}
  </Switch>
{/if}

<style>
  .tint-fields {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .schedule {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 160px));
    gap: 8px 16px;
  }

  .schedule-hint {
    grid-column: 1 / -1;
  }

  .time {
    max-width: 160px;
  }

  .range-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .range-row input {
    flex: 1;
    max-width: 320px;
  }

  .value {
    min-width: 3.5em;
    color: var(--text-2);
  }

  .transition select {
    max-width: 220px;
  }

  .modes {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  .modes legend {
    margin-bottom: 6px;
    padding: 0;
  }

  .mode {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-inset);
    cursor: pointer;
  }

  .mode:hover {
    border-color: var(--border-strong);
  }

  .mode.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .mode input {
    flex: none;
    width: 18px;
    height: 18px;
    margin: 2px 0 0;
  }

  .mode-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .mode-name {
    font-weight: 600;
  }

  .status-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 14px;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    font-size: 0.88rem;
    font-weight: 600;
  }

  .note {
    padding-left: 20px;
  }

  .status.error {
    color: var(--danger);
  }

  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: rgb(255, 130, 30);
    box-shadow: 0 0 0 2px rgba(255, 130, 30, 0.25);
  }

  .status.error .swatch {
    background: var(--danger);
    box-shadow: none;
  }
</style>
