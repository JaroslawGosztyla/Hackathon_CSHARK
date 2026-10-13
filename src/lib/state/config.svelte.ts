// Config store: loaded once, edited in place, saved (debounced) through `update_config`.
// The shell normalizes + validates; its validation errors land in `fieldErrors` next to the field.
import * as api from "$lib/api";
import { errorText, strings } from "$lib/strings";
import type { Config } from "$lib/types";
import { toasts } from "./toasts.svelte";

const SAVE_DEBOUNCE_MS = 600;
const FOCUS_MINUTES = { min: 1, max: 240 } as const;
const BREAK_MINUTES = { min: 1, max: 120 } as const;
/** Mirrors focus-core `tint.rs`: times are minutes after midnight, strength 0..=100, fade 0..=180 min. */
const TINT = { minutesPerDay: 1440, maxStrength: 100, maxTransitionMinutes: 180 } as const;
/** Choices offered for the tint fade (minutes); a different stored value is shown as an extra option. */
export const TINT_TRANSITION_PRESETS = [0, 15, 30, 60, 90] as const;

function inRange(n: number, min: number, max: number): boolean {
  return Number.isInteger(n) && n >= min && n <= max;
}

function isUnit(n: number): boolean {
  return Number.isFinite(n) && n >= 0 && n <= 1;
}

/** Client-side mirror of the ranges in `api.rs`, so obviously invalid values are never sent. */
export function validateConfig(c: Config): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!inRange(c.focusMinutes, FOCUS_MINUTES.min, FOCUS_MINUTES.max)) errors.focusMinutes = strings.timing.rangeFocus;
  if (!inRange(c.breakMinutes, BREAK_MINUTES.min, BREAK_MINUTES.max)) errors.breakMinutes = strings.timing.rangeBreak;
  if (!isUnit(c.music.volume)) errors["music.volume"] = strings.errors.fieldInvalid(strings.fields["music.volume"] ?? "");
  if (!isUnit(c.chimeVolume)) errors.chimeVolume = strings.errors.fieldInvalid(strings.fields.chimeVolume ?? "");
  // Validated even when the tint is off, like in focus-core (the shell would reject the whole config).
  const t = c.tint;
  if (!inRange(t.startMinutes, 0, TINT.minutesPerDay - 1)) errors["tint.startMinutes"] = strings.settings.tintTimeInvalid;
  if (!inRange(t.endMinutes, 0, TINT.minutesPerDay - 1)) errors["tint.endMinutes"] = strings.settings.tintTimeInvalid;
  else if (t.startMinutes === t.endMinutes) errors["tint.endMinutes"] = strings.settings.tintSameTimes;
  if (!inRange(t.strength, 0, TINT.maxStrength)) errors["tint.strength"] = strings.settings.tintStrengthRange;
  if (!inRange(t.transitionMinutes, 0, TINT.maxTransitionMinutes)) {
    errors["tint.transitionMinutes"] = strings.settings.tintTransitionRange;
  }
  return errors;
}

export const LIMITS = { focusMinutes: FOCUS_MINUTES, breakMinutes: BREAK_MINUTES, tint: TINT } as const;

function copyField<K extends keyof Config>(target: Config, source: Config, key: K): void {
  target[key] = source[key];
}

class ConfigStore {
  value = $state<Config | null>(null);
  fieldErrors = $state<Record<string, string>>({});
  saving = $state(false);
  /** Bumps after every successful save (for a small "Zapisano" hint). */
  savedCount = $state(0);

  private timer: ReturnType<typeof setTimeout> | null = null;
  private editGeneration = 0;
  private chain: Promise<void> = Promise.resolve();
  private loading: Promise<void> | null = null;

  get ready(): boolean {
    return this.value !== null;
  }

  load(): Promise<void> {
    this.loading ??= api
      .getConfig()
      .then((cfg) => {
        // Edits made before the load finished are impossible (UI waits for `ready`), so just take it.
        this.value = cfg;
      })
      .catch((e: unknown) => {
        this.loading = null;
        toasts.error(e);
      });
    return this.loading;
  }

  /**
   * Mutates the local config and schedules a save. Returns false when the config is not loaded yet.
   * `immediate` skips the debounce (toggles, pickers); typing in inputs should use the default.
   */
  update(mutate: (c: Config) => void, immediate = false): boolean {
    const current = this.value;
    if (!current) return false;
    mutate(current);
    this.editGeneration++;
    const local = validateConfig(current);
    this.fieldErrors = local;
    this.cancelTimer();
    if (Object.keys(local).length > 0) return true;
    if (immediate) void this.saveNow();
    else this.timer = setTimeout(() => void this.saveNow(), SAVE_DEBOUNCE_MS);
    return true;
  }

  /**
   * Takes fields of a config the shell changed and stored by itself (e.g. `resolve_close`). Only `keys` are copied,
   * so local edits that are not saved yet stay; nothing is saved again (the shell already stored it).
   */
  adopt(saved: Config, keys: readonly (keyof Config)[]): void {
    const current = this.value;
    if (!current) {
      this.value = saved;
      return;
    }
    for (const key of keys) copyField(current, saved, key);
  }

  /** Saves pending edits right away. Resolves true when the config is valid and stored. */
  async flush(): Promise<boolean> {
    if (!this.value) {
      await this.load();
      return this.value !== null;
    }
    if (Object.keys(validateConfig(this.value)).length > 0) {
      this.fieldErrors = validateConfig(this.value);
      return false;
    }
    if (this.timer !== null) {
      this.cancelTimer();
      await this.saveNow();
    } else {
      await this.chain;
    }
    return Object.keys(this.fieldErrors).length === 0;
  }

  private cancelTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }

  /** Saves are serialized; a response is applied only if no newer edit happened meanwhile. */
  private saveNow(): Promise<void> {
    this.chain = this.chain.then(async () => {
      const current = this.value;
      if (!current) return;
      const generation = this.editGeneration;
      const payload = $state.snapshot(current) as Config;
      this.saving = true;
      try {
        const saved = await api.updateConfig(payload);
        if (generation === this.editGeneration) {
          this.value = saved;
          this.fieldErrors = {};
        }
        this.savedCount++;
      } catch (e) {
        const err = api.toAppError(e);
        if (err.field) this.fieldErrors = { ...this.fieldErrors, [err.field]: errorText(err) };
        // Also as a toast: not every field has an inline error slot on the current page.
        toasts.error(err);
      } finally {
        this.saving = false;
      }
    });
    return this.chain;
  }
}

export const config = new ConfigStore();
