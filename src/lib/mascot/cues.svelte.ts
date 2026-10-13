// Session events for the mascots of the main window. `app.ts` emits a cue when a phase changes or a session ends;
// every mascot on screen that cares about it reacts. A mascot that appears right after the event (the summary page
// opens after Stop, the Skupienie page is opened again right after Start) still reacts while the cue is fresh.
import { untrack } from "svelte";
import type { Config, PhaseChange, SessionSnapshot, SessionSummary } from "$lib/types";
import type { MascotController } from "./controller.svelte";
import type { MascotMood, MascotReaction } from "./frames";

/** A cue older than this is not played by a mascot that appears later. */
const FRESH_MS = 4000;

interface MascotCue {
  id: number;
  reaction: MascotReaction;
  /** `performance.now()` when it was emitted. */
  at: number;
}

class MascotCues {
  last = $state.raw<MascotCue | null>(null);
  #seq = 0;

  emit(reaction: MascotReaction): void {
    this.#seq++;
    this.last = { id: this.#seq, reaction, at: performance.now() };
  }
}

export const mascotCues = new MascotCues();

/** Start (idle → focus), completed cycle (break → focus) or a break; Stop is decided by the summary. */
export function cueForPhaseChange(change: PhaseChange): MascotReaction | null {
  if (change.to === "break") return "break";
  if (change.to === "focus") return change.from === "idle" ? "start" : "cycle";
  return null;
}

/** Stop before one full focus phase (`focusSecs` = length of a focus phase in the profile in use). */
export function cueForSummary(summary: SessionSummary, focusSecs: number): MascotReaction | null {
  return focusSecs > 0 && summary.focusedSecs < focusSecs ? "stopEarly" : null;
}

/**
 * Makes `controller` react to fresh cues of the given kinds (and calls `onCue`, e.g. to say something about it).
 * Call while a component initialises.
 */
export function followCues(
  controller: MascotController,
  accept: readonly MascotReaction[],
  onCue?: (reaction: MascotReaction) => void,
): void {
  let seen = 0;
  $effect(() => {
    const cue = mascotCues.last;
    if (cue === null || cue.id === seen) return;
    seen = cue.id;
    if (!accept.includes(cue.reaction) || performance.now() - cue.at > FRESH_MS) return;
    untrack(() => {
      controller.react(cue.reaction);
      onCue?.(cue.reaction);
    });
  });
}

/**
 * The face the main window's mascot rests on, from the session alone (one source of truth): no session → neutral;
 * a meeting extends the focus or holds the break → neutral; paused → sad; break → smile; focus running → calm.
 */
export function sessionMood(snap: SessionSnapshot): MascotMood {
  if (snap.phase === "idle" || snap.meetingHold !== null) return "neutral";
  if (snap.paused) return "sad";
  return snap.phase === "break" ? "smile" : "calm";
}

/** `Config.showMascot`, on by default (also when talking to a shell build from before the setting). */
export function mascotEnabled(cfg: Config | null): boolean {
  return cfg !== null && (cfg.showMascot as boolean | undefined) !== false;
}
