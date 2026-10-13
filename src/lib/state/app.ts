// Wires shell events and initial reads into the stores. Called once by the root layout.
import { goto } from "$app/navigation";
import * as api from "$lib/api";
import type { AppError } from "$lib/types";
import { blocklist } from "./blocklist.svelte";
import { config } from "./config.svelte";
import { mediaWindow } from "./mediaWindow.svelte";
import { nudge } from "./nudge.svelte";
import { session } from "./session.svelte";
import { shields } from "./shields.svelte";
import { summary } from "./summary.svelte";
import { teams } from "./teams.svelte";
import { tint } from "./tint.svelte";
import { toasts } from "./toasts.svelte";

/** Starts all subscriptions; returns a cleanup function. */
export function initAppState(): () => void {
  const unlisteners: api.UnlistenFn[] = [];
  let disposed = false;
  let reportedNoShell = false;

  const reportError = (error: AppError) => {
    // Outside the shell (plain `vite dev`) every getter fails the same way: say it once.
    if (error.code === api.ERR_NO_SHELL) {
      if (reportedNoShell) return;
      reportedNoShell = true;
    }
    toasts.error(error);
  };

  const keep = (pending: Promise<api.UnlistenFn>) => {
    pending
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      })
      .catch((e: unknown) => reportError(api.toAppError(e)));
  };

  keep(api.onAppError((error) => toasts.error(error)));
  keep(api.subscribeSnapshot((s) => session.apply(s), reportError));
  keep(api.subscribeShields((s) => shields.apply(s), reportError));
  keep(api.subscribeTeams((s) => teams.apply(s), reportError));
  keep(api.subscribeMediaWindow((s) => mediaWindow.apply(s), reportError));
  keep(api.subscribeBlocklist((b) => blocklist.apply(b), reportError));
  keep(api.subscribeTint((t) => tint.apply(t), reportError));
  keep(api.onPhaseChanged((change) => session.announce(change)));
  keep(api.onNudge((n) => nudge.show(n)));
  keep(
    api.onSessionSummary((s) => {
      summary.add(s);
      void goto("/summary");
    }),
  );

  void config.load();
  void summary.load();

  return () => {
    disposed = true;
    for (const unlisten of unlisteners.splice(0)) unlisten();
  };
}
