<script lang="ts">
  // Skupienie page: the axolotl beside the timer, in every phase. Its face follows the session (`sessionMood`) and
  // it reacts to what happens: a wink at Start, a smile when a break starts, a cheer for a completed cycle, an angry
  // look at a blocked site. What it says appears in its bubble letter by letter (MascotSpeech):
  //   idle   — a meeting that would fall into a break, else the top "Co dalej?" item, else the good practice of the day;
  //   events — a short line at Start, when a break starts and when a cycle is completed (urgent, at once);
  //   break  — the user's reminder for this break and the break tip (again after "Inna propozycja").
  // Absolutely positioned in its own lane right of the ring, from the ring's top to its bottom line: the bubble grows
  // upwards above the axolotl and scrolls inside when the text is long, so it never covers the ring or the buttons.
  import { untrack } from "svelte";
  import { breakTip } from "$lib/state/breakTip.svelte";
  import { calendar, calendarEnabled } from "$lib/state/calendar.svelte";
  import { config } from "$lib/state/config.svelte";
  import { nudge } from "$lib/state/nudge.svelte";
  import { session } from "$lib/state/session.svelte";
  import { work, workEnabled } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";
  import { breakTipText, workingTipOfDay, workingTipText } from "$lib/tips";
  import Axolotl from "./Axolotl.svelte";
  import { useMascot } from "./controller.svelte";
  import { followCues, mascotEnabled, sessionMood } from "./cues.svelte";
  import type { MascotReaction } from "./frames";
  import { useSpeech, type MascotLine } from "./speech.svelte";
  import SpeechBubble from "./SpeechBubble.svelte";

  /** Height of the axolotl in CSS pixels. */
  const SIZE = 140;

  const mascot = useMascot("main");
  const speech = useSpeech(mascot);

  // Read once per mount, like the "Dobra praktyka" card below.
  const tip = workingTipOfDay(new Date());

  const show = $derived(mascotEnabled(config.value));

  function pick(lines: readonly string[]): string {
    return lines[Math.floor(Math.random() * lines.length)] ?? lines[0] ?? "";
  }

  // ---------------------------------------------------------------------------
  // Face: the session decides the mood; events play a reaction on top of it.
  // ---------------------------------------------------------------------------

  const mood = $derived(sessionMood(session.snapshot));
  $effect(() => {
    const next = mood;
    untrack(() => mascot.setMood(next));
  });

  const EVENT_LINES: Partial<Record<MascotReaction, () => readonly string[]>> = {
    start: () => strings.mascot.startLines,
    break: () => strings.mascot.breakLines,
    cycle: () => strings.mascot.cycleLines,
  };

  followCues(mascot, ["start", "break", "cycle"], (reaction) => {
    const lines = EVENT_LINES[reaction]?.();
    if (show && lines) speech.say({ text: pick(lines), urgent: true, tag: "event" });
  });

  // Every nudge (a new object, also while the banner is already shown) gets an angry look.
  $effect(() => {
    if (nudge.current && show) untrack(() => mascot.react("nudge"));
  });

  // ---------------------------------------------------------------------------
  // What it says about the current state (not urgent: at most one message per gap, the newest wins).
  // ---------------------------------------------------------------------------

  /** First meeting of an idle session's plan that falls into a break and has no choice yet. */
  const undecided = $derived.by(() => {
    if (!session.isIdle || !calendarEnabled(config.value)) return null;
    return calendar.status?.conflicts.find((c) => c.choice === null) ?? null;
  });
  const topItem = $derived(workEnabled(config.value) ? (work.status?.items[0] ?? null) : null);

  const idleLine = $derived.by((): MascotLine | null => {
    if (!session.isIdle) return null;
    if (undecided) {
      const subject = undecided.meeting.subject.trim() || strings.calendar.untitled;
      return { label: strings.calendar.cardTitle, text: strings.mascot.conflictHint(subject), tag: "idle" };
    }
    if (topItem) {
      return { label: strings.mascot.nextUpLabel, text: strings.work.itemName(topItem.key, topItem.title), tag: "idle" };
    }
    if (tip) return { label: strings.mascot.practiceLabel, text: workingTipText(tip), tag: "idle" };
    return null;
  });

  const breakLine = $derived.by((): MascotLine | null => {
    if (session.phase !== "break") return null;
    const reminder = breakTip.reminder;
    const current = breakTip.tip;
    // The user's reminder on its own line (it often has no full stop), then the tip.
    const text = [reminder, current ? breakTipText(current) : null].filter((part) => part !== null).join("\n");
    if (text === "") return null;
    const label = reminder ? strings.breakView.reminderLabel(session.snapshot.cycle) : strings.breakView.suggestionLabel;
    return { label, text, tag: "break" };
  });

  /**
   * The calendar and the work items answer right after the page opens: wait a moment, so the first message is the
   * right one (not the good practice replaced by a meeting hint a gap later).
   */
  const SETTLE_MS = 1200;
  let settled = $state(false);
  $effect(() => {
    const timer = setTimeout(() => (settled = true), SETTLE_MS);
    return () => clearTimeout(timer);
  });

  /** Says `line` when its text changes; takes the tag's message back when there is nothing to say. */
  function sayWhenChanged(tag: string, line: () => MascotLine | null): void {
    let said: string | null = null;
    $effect(() => {
      const next = show && settled ? line() : null;
      const text = next?.text ?? null;
      if (text === said) return;
      said = text;
      untrack(() => (next ? speech.say(next) : speech.withdraw(tag)));
    });
  }

  sayWhenChanged("idle", () => idleLine);
  sayWhenChanged("break", () => breakLine);

  // Hidden in Settings: no bubble either.
  $effect(() => {
    if (!show) untrack(() => speech.clear());
  });
</script>

{#if show}
  <div class="focus-mascot">
    <div class="bubble-spot">
      <SpeechBubble {speech} />
    </div>
    <Axolotl size={SIZE} frame={mascot.frame} sit />
  </div>
{/if}

<style>
  /*
   * The lane right of the ring (its wrapper is the positioning box), as tall as the ring: the axolotl stands on the
   * ring's bottom line and the bubble fills the room above it. 150px is half the ring (TimerRing: 300px wide); the
   * lane ends a little before the page gutter, so it fits from the minimum window width (960px) up.
   */
  .focus-mascot {
    position: absolute;
    top: 0;
    bottom: 0;
    left: calc(100% + 6px);
    width: min(282px, calc(50vw - 150px - var(--gutter) - 36px));
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-end;
    gap: 10px;
    pointer-events: none;
  }

  /* Above the axolotl's head; shrinks (and the text scrolls inside the bubble) when the text is long. */
  .bubble-spot {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    min-height: 0;
    max-width: calc(100% - 6px);
    margin-left: 6px;
  }
</style>
