// Settings → Prywatność i dane (ADR-020): what FocusMode stores on this computer (`PrivacyReport`, pushed by the shell
// with a revision while the panel is open), the export and the deletes. A delete returns the stored config, which this
// store hands to the config store (there is no config event); a history delete also empties the summary views.
import * as api from "$lib/api";
import type { Config, PrivacyItemKind, PrivacyReport } from "$lib/types";
import { config } from "./config.svelte";
import { summary } from "./summary.svelte";
import { toasts } from "./toasts.svelte";

/** Default file name of the export (language-neutral, dated). */
function exportFileName(now: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `focusmode-export-${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.json`;
}

class PrivacyStore {
  report = $state<PrivacyReport | null>(null);
  /**
   * A delete started here is running. The session it stops must not open Podsumowanie (the user stays in the panel);
   * read by the app state when a summary arrives.
   */
  deleting = $state(false);

  private watchers = 0;
  private unlisten: Promise<api.UnlistenFn> | null = null;

  /** Follows the report while a view shows it; returns the cleanup (the last one stops listening). */
  watch(): () => void {
    this.watchers++;
    if (this.watchers === 1) {
      this.unlisten = api.subscribePrivacy(
        (report) => (this.report = report),
        (e) => {
          if (e.code !== api.ERR_NO_SHELL) toasts.error(e);
        },
      );
      this.unlisten.catch((e: unknown) => toasts.error(e));
    }
    let done = false;
    return () => {
      if (done) return;
      done = true;
      this.watchers--;
      if (this.watchers > 0) return;
      const pending = this.unlisten;
      this.unlisten = null;
      void pending?.then((stop) => stop()).catch(() => undefined);
    };
  }

  /** The shell runs a delete or an export (from any window). */
  get busy(): boolean {
    return this.deleting || this.report?.busy === true;
  }

  /** Save dialog → one JSON file without secrets. Resolves false when the dialog was cancelled; throws on failure. */
  async exportData(dialogTitle: string, filterName: string): Promise<boolean> {
    const path = await api.pickSaveFile(dialogTitle, exportFileName(), filterName, ["json"]);
    if (!path) return false;
    await api.exportPrivacyData(path);
    return true;
  }

  /** Deletes those items (the UI confirmed first). Throws on failure (the config is reloaded then). */
  remove(kinds: PrivacyItemKind[]): Promise<void> {
    return this.run(() => api.deletePrivacyData(kinds), kinds.includes("history"));
  }

  /** "Usuń wszystko": every deletable item and the settings back to the defaults (the UI confirmed first). */
  removeAll(): Promise<void> {
    return this.run(() => api.deleteAllData(), true);
  }

  private async run(action: () => Promise<Config>, historyGoes: boolean): Promise<void> {
    if (this.deleting) return;
    this.deleting = true;
    try {
      // Pending edits are stored first, so the config the shell returns is the whole truth.
      await config.flush();
      let saved: Config;
      try {
        saved = await action();
      } catch (e) {
        // A partial failure may still have changed the config (chimes, defaults) and the history.
        await this.reloadAfterFailure(historyGoes);
        throw e;
      }
      adoptAll(saved);
      if (historyGoes) summary.forget();
    } finally {
      this.deleting = false;
    }
  }

  private async reloadAfterFailure(historyGoes: boolean): Promise<void> {
    try {
      adoptAll(await api.getConfig());
    } catch (e) {
      console.warn("[FocusMode] cannot reload the config after a failed delete", api.toAppError(e).code);
    }
    if (historyGoes) {
      summary.forget();
      await summary.load();
    }
  }
}

/** Takes the whole config the shell stored (nothing is saved again). */
function adoptAll(saved: Config): void {
  config.adopt(saved, Object.keys(saved) as (keyof Config)[]);
}

export const privacy = new PrivacyStore();
