// Jittered event scheduler of the ambience layers (ADR-024): one timer per use, events placed on the audio clock
// (`ctx.currentTime`) up to `LOOKAHEAD_S` ahead, so they land exactly on time even when the timer is late. The timer
// sleeps until the next event is near: a thunder every 25–70 s costs one wake-up per thunder.
import { between, type Rng } from "./rng";

/** Events are placed this far ahead of the audio clock. */
export const LOOKAHEAD_S = 2;
/** The timer never wakes more often than this. */
const MIN_WAKE_MS = 250;

export interface Every {
  /** Smallest and largest gap between two events, in seconds. */
  min: number;
  max: number;
  /** Delay of the first event (default: a gap like the others). */
  first?: number;
}

/**
 * Calls `fire(at)` for every event, `at` on the audio clock (never in the past), until the returned stop function is
 * called. A suspended context (its clock stands still) simply delays the events.
 */
export function every(ctx: BaseAudioContext, rng: Rng, timing: Every, fire: (at: number) => void): () => void {
  let next = ctx.currentTime + (timing.first ?? between(rng, timing.min, timing.max));
  let timer: ReturnType<typeof setTimeout> | null = null;
  let stopped = false;
  const gap = () => Math.max(0.01, between(rng, timing.min, timing.max));
  const tick = () => {
    timer = null;
    if (stopped) return;
    const now = ctx.currentTime;
    while (next <= now + LOOKAHEAD_S) {
      try {
        fire(Math.max(next, now + 0.02));
      } catch {
        // One failed event (a node refused a value) never stops the layer.
      }
      next += gap();
    }
    const sleep = (next - LOOKAHEAD_S - ctx.currentTime) * 1000;
    timer = setTimeout(tick, Math.max(MIN_WAKE_MS, sleep));
  };
  tick();
  return () => {
    stopped = true;
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
}
