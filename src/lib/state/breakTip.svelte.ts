// The break on the Skupienie page: the user's own reminder for this break and one research-based tip, picked
// deterministically per (session, break) so the phase popup shows the same one. Shared by the break card and the
// axolotl at the bottom-left of the timer section, which says them. "Inna propozycja" steps are kept per break, so
// leaving the page and coming back keeps the tip.
import { SvelteMap } from "svelte/reactivity";
import { pickBreakTip, type BreakTip } from "$lib/tips";
import { config, reminderForBreak } from "./config.svelte";
import { session } from "./session.svelte";

class BreakTipStore {
  /** "Inna propozycja" steps per break key. */
  #offsets = new SvelteMap<string, number>();

  /** One key per break: session + cycle. */
  readonly key = $derived(`${session.snapshot.sessionId ?? ""}:${session.snapshot.cycle}`);

  readonly reminder = $derived.by((): string | null => {
    const cfg = config.value;
    return cfg ? reminderForBreak(cfg.breakReminders, cfg.repeatReminders, session.snapshot.cycle) : null;
  });

  readonly tip = $derived.by((): BreakTip | null => {
    const snap = session.snapshot;
    return pickBreakTip(snap.sessionId, snap.cycle, snap.phaseTotalSecs, this.#offsets.get(this.key) ?? 0);
  });

  /** "Inna propozycja": the next tip for this break. */
  another(): void {
    const key = this.key;
    const offset = this.#offsets.get(key) ?? 0;
    // Only the current break's entry matters; older ones are dropped so the map stays tiny.
    for (const k of this.#offsets.keys()) if (k !== key) this.#offsets.delete(k);
    this.#offsets.set(key, offset + 1);
  }
}

export const breakTip = new BreakTipStore();
