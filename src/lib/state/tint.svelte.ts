// Warm screen colours (Windows Night light or display gamma, applied by the shell): status pushed by the shell
// plus the settings preview.
import * as api from "$lib/api";
import type { AppError, TintStatus } from "$lib/types";

class TintStore {
  status = $state<TintStatus | null>(null);

  /** Preview calls are sent one after another, so a late `previewTint(value)` never overtakes `previewTint(null)`. */
  private chain: Promise<void> = Promise.resolve();

  apply(status: TintStatus): void {
    this.status = status;
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
