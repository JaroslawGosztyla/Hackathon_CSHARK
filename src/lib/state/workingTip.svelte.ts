// The good working practice of the idle Skupienie page: the "Dobra praktyka" card shows it and the axolotl says it.
// The tip of the day comes first; "Inna porada" shows a different random one (never the same twice in a row, every
// tip once before one repeats, in this run of the app). A new day brings that day's tip back. Memory only.
import { nextWorkingTip, WORKING_TIPS, workingTipDay, workingTipOfDay, type WorkingTip, type WorkingTipId } from "$lib/tips";

class WorkingTipStore {
  /** The tip shown now: the tip of the day until "Inna porada". */
  current = $state.raw<WorkingTip | null>(workingTipOfDay(new Date()));
  /** Grows with every "Inna porada" (the axolotl says the new tip then). */
  revision = $state(0);
  /** There is another tip to show. */
  readonly canChange = WORKING_TIPS.length > 1;

  #day = workingTipDay(new Date());
  /** The tips shown in this round, the current one included. */
  #seen = new Set<WorkingTipId>(this.current === null ? [] : [this.current.id]);

  /** On a new day the day's tip comes back (the card and the axolotl call it when they mount). */
  refreshDay(date: Date = new Date()): void {
    const day = workingTipDay(date);
    if (day === this.#day) return;
    this.#day = day;
    const tip = workingTipOfDay(date);
    this.current = tip;
    this.#seen = new Set(tip === null ? [] : [tip.id]);
  }

  /** "Inna porada": a different random tip. */
  another(): void {
    const next = nextWorkingTip(this.current?.id ?? null, this.#seen);
    if (next === null) return;
    this.#seen = next.seen;
    this.current = next.tip;
    this.revision++;
  }
}

export const workingTip = new WorkingTipStore();
