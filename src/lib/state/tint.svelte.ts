// Warm screen colours (Windows Night light or display gamma, applied by the shell): status pushed by the shell
// plus the settings preview, the built-in cities of the sunset / sunrise schedule and a Developer-page sample.
import * as api from "$lib/api";
import type { AppError, SunCity, TintStatus } from "$lib/types";

/** The app of the Developer-page sample (one of focus-core `tint::DEFAULT_PAUSE_APPS`). */
const SAMPLE_PAUSE_APP = "Photoshop.exe";

class TintStore {
  status = $state<TintStatus | null>(null);

  /** Developer page: a status shown instead of the shell's one until cleared (memory only, never sent anywhere). */
  sample = $state<TintStatus | null>(null);

  /** Built-in cities for the sunset / sunrise schedule (`list_sun_cities`, ADR-023); empty until loaded. */
  cities = $state<SunCity[]>([]);

  /** Preview calls are sent one after another, so a late `previewTint(value)` never overtakes `previewTint(null)`. */
  private chain: Promise<void> = Promise.resolve();
  private citiesRequest: Promise<void> | null = null;

  /** What the UI shows: the Developer-page sample, else the shell's status. */
  get shown(): TintStatus | null {
    return this.sample ?? this.status;
  }

  apply(status: TintStatus): void {
    this.status = status;
  }

  /** Developer page: the warmth suspended by a colour-critical app in front (ADR-023), on top of the real status. */
  showSample(): void {
    const base = this.status;
    this.sample = {
      revision: base?.revision ?? 0,
      enabled: true,
      level: 0,
      preview: false,
      active: false,
      mode: base?.mode ?? "nightLight",
      capped: false,
      error: null,
      pausedBy: { reason: "app", app: SAMPLE_PAUSE_APP },
      schedule: base?.schedule ?? "fixed",
      windowStartMinutes: base?.windowStartMinutes ?? 0,
      windowEndMinutes: base?.windowEndMinutes ?? 0,
    };
  }

  clearSample(): void {
    this.sample = null;
  }

  /** Loads the city list once (a failure is logged and retried on the next call). */
  loadCities(): Promise<void> {
    this.citiesRequest ??= api.listSunCities().then(
      (cities) => {
        this.cities = cities;
      },
      (e: unknown) => {
        this.citiesRequest = null;
        const err = api.toAppError(e);
        if (err.code !== api.ERR_NO_SHELL) console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      },
    );
    return this.citiesRequest;
  }

  /**
   * Shows the tint at `strength` (0..100) for a few seconds, or ends the preview (`null`). Failures are only logged;
   * resolves with the failure (null when the shell took it), for callers that report it.
   */
  preview(strength: number | null): Promise<AppError | null> {
    const result = this.chain.then(async (): Promise<AppError | null> => {
      try {
        await api.previewTint(strength);
        return null;
      } catch (e) {
        const err = api.toAppError(e);
        if (err.code !== api.ERR_NO_SHELL) console.warn(`[FocusMode] ${err.code}: ${err.message}`);
        return err;
      }
    });
    this.chain = result.then(() => undefined);
    return result;
  }
}

export const tint = new TintStore();
