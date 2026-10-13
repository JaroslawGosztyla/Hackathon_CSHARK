// Warm screen tint (overlay, ADR-014): status pushed by the shell plus the settings preview.
import * as api from "$lib/api";
import type { TintStatus } from "$lib/types";

class TintStore {
  status = $state<TintStatus | null>(null);

  /** Preview calls are sent one after another, so a late `previewTint(value)` never overtakes `previewTint(null)`. */
  private chain: Promise<void> = Promise.resolve();

  apply(status: TintStatus): void {
    this.status = status;
  }

  /** Shows the tint at `strength` (0..100) for a few seconds, or ends the preview (`null`). Failures are only logged. */
  preview(strength: number | null): Promise<void> {
    this.chain = this.chain.then(async () => {
      try {
        await api.previewTint(strength);
      } catch (e) {
        const err = api.toAppError(e);
        if (err.code !== api.ERR_NO_SHELL) console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      }
    });
    return this.chain;
  }
}

export const tint = new TintStore();
