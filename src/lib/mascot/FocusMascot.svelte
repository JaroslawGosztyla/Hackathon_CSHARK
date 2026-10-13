<script lang="ts" module>
  import { FRAME_GEOMETRY } from "./assets.svelte";

  /** Height of the axolotl in CSS pixels. */
  export const MASCOT_SIZE = 140;
  /** Width of its box (the frame's aspect ratio): the page keeps its centred content clear of it. */
  export const MASCOT_WIDTH = Math.round((MASCOT_SIZE * FRAME_GEOMETRY.main.frameWidth) / FRAME_GEOMETRY.main.frameHeight);
</script>

<script lang="ts">
  // Skupienie page: the axolotl at the bottom-left of the hero, in every phase. Its face follows the session
  // (`sessionMood`) and it reacts to what happens: a wink at Start, a smile when a break starts, a cheer for a
  // completed cycle, an angry look at a blocked site. What it says appears in its bubble letter by letter (MascotSpeech):
  //   idle   — a meeting that would fall into a break, else the top "Co dalej?" item, else the good practice of the day;
  //   events — a short line at Start, when a break starts and when a cycle is completed (urgent, at once);
  //   break  — the user's reminder for this break and the break tip (again after "Inna propozycja");
  //   tickle — a click on the axolotl while it says nothing: a boing, a giggle and one funny line.
  // The page gives it a lane at its left content edge (left of the timer ring, from the hero's top to its last line):
  // the axolotl stands at the bottom, the bubble grows upwards above its head (or above the session buttons, when
  // they reach into the lane: `bubbleFloor`) and scrolls inside when the text is long, so it never covers the ring or
  // the buttons. Absolutely positioned: talking never moves the page.
  import { untrack } from "svelte";
  import { breakTip } from "$lib/state/breakTip.svelte";
  import { calendar, calendarEnabled } from "$lib/state/calendar.svelte";
  import { config } from "$lib/state/config.svelte";
  import { dev, useDevRequests } from "$lib/state/dev.svelte";
  import { nudge } from "$lib/state/nudge.svelte";
  import { session } from "$lib/state/session.svelte";
  import { work, workEnabled } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";
  import { breakTipText, workingTipOfDay, workingTipText } from "$lib/tips";
  import Axolotl from "./Axolotl.svelte";
  import { prefersReducedMotion, useMascot } from "./controller.svelte";
  import { followCues, mascotEnabled, sessionMood } from "./cues.svelte";
  import type { MascotReaction } from "./frames";
  import { useSpeech, type MascotLine } from "./speech.svelte";
  import SpeechBubble from "./SpeechBubble.svelte";

  interface Props {
    /**
     * The bubble's bottom stays at least this many pixels above the lane's bottom (the page keeps it above session
     * buttons that reach into the lane); 0: right above the axolotl's head.
     */
    bubbleFloor?: number;
  }

  let { bubbleFloor = 0 }: Props = $props();

  /** The bubble's tail points at the axolotl's head (the frame's anchor column); this is the tail's left edge. */
  const TAIL_X = Math.round((MASCOT_SIZE * FRAME_GEOMETRY.main.anchorX) / FRAME_GEOMETRY.main.frameHeight) - 5;
  /** The axolotl's height in the lane (Axolotl `sit` sinks the part below its feet) and the gap above its head. */
  const STAND_HEIGHT =
    MASCOT_SIZE -
    Math.round(((FRAME_GEOMETRY.main.frameHeight - FRAME_GEOMETRY.main.anchorY) * MASCOT_SIZE) / FRAME_GEOMETRY.main.frameHeight);
  const BUBBLE_GAP = 10;
  /** Extra room under the bubble when it has to stay above the session buttons. */
  const lift = $derived(Math.max(0, Math.ceil(bubbleFloor - STAND_HEIGHT - BUBBLE_GAP)));
  /** The squash-and-stretch after a click (CSS `boing`), then a pause before a click counts again (no boing spam). */
  const BOING_MS = 700;
  const TICKLE_COOLDOWN_MS = 1000;

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

  /** A mood held from the Developer page (ADR-019) wins over the session's. */
  const mood = $derived(dev.mascotMood ?? sessionMood(session.snapshot));
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

  /** First meeting of an idle session's plan that falls into a break and has no choice yet (or of a dev sample). */
  const undecided = $derived.by(() => {
    if (!session.isIdle || !(calendarEnabled(config.value) || calendar.sample !== null)) return null;
    return calendar.status?.conflicts.find((c) => c.choice === null) ?? null;
  });
  const topItem = $derived(
    workEnabled(config.value) || work.sample !== null ? (work.status?.items[0] ?? null) : null,
  );

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

  // ---------------------------------------------------------------------------
  // Tickle: a click (or Enter / Space) on the axolotl.
  // ---------------------------------------------------------------------------

  /** The boing plays now (CSS class; removed when the animation ends or is cancelled). */
  let boinging = $state(false);
  /** The wrapper the boing transforms. */
  let hop = $state<HTMLElement | null>(null);
  /** `performance.now()` from which a click counts again. */
  let readyAt = 0;
  /** Index of the last funny line, so the next one is always a different one. */
  let lastTickle = -1;

  function tickleLine(): string {
    const lines = strings.mascot.tickleLines;
    const skip = lastTickle >= 0 && lastTickle < lines.length && lines.length > 1 ? 1 : 0;
    let i = Math.floor(Math.random() * (lines.length - skip));
    if (skip === 1 && i >= lastTickle) i++;
    lastTickle = i;
    return lines[i] ?? "";
  }

  /**
   * Starts the squash-and-stretch. If the class is still on (its end was never reported), the same animation is
   * replayed from its first frame: taking the class off and back on within one frame would not restart it.
   */
  function boing(): void {
    if (!boinging) {
      boinging = true;
      return;
    }
    for (const animation of hop?.getAnimations() ?? []) {
      animation.currentTime = 0;
      animation.play();
    }
  }

  /**
   * Always a boing (none with reduced motion). Only when nothing is said (no bubble, none waiting for a reaction) it
   * also giggles and says one funny line, at once (urgent: a state message waiting for the gap still comes after it).
   * A message on screen is never skipped, replaced or delayed by a click: a click on the bubble shows its whole text.
   */
  function tickle(): void {
    const now = performance.now();
    if (!show || now < readyAt) return;
    const reduced = prefersReducedMotion();
    readyAt = now + (reduced ? 0 : BOING_MS) + TICKLE_COOLDOWN_MS;
    if (!reduced) boing();
    if (speech.active) return;
    mascot.react("tickle");
    speech.say({ text: tickleLine(), urgent: true, tag: "tickle" });
  }

  function boingEnded(e: AnimationEvent): void {
    if (e.target === e.currentTarget) boinging = false;
  }

  // ---------------------------------------------------------------------------
  // Developer page (ADR-019): reactions, the tickle and sample lines, once this page has mounted.
  // ---------------------------------------------------------------------------

  /** Tag of the Developer page's lines (an urgent message, like an event's). */
  const DEV_TAG = "dev";

  useDevRequests("mascot", ({ action }) => {
    if (!show) return;
    speech.clear();
    switch (action.type) {
      case "react": {
        mascot.react(action.reaction);
        const lines = EVENT_LINES[action.reaction]?.();
        if (lines) speech.say({ text: pick(lines), urgent: true, tag: "event" });
        break;
      }
      case "tickle":
        readyAt = 0;
        tickle();
        break;
      case "say":
        speech.say({ text: action.text, label: action.label, urgent: true, tag: DEV_TAG });
        break;
    }
  });

  // A boing cut short (e.g. reduced motion switched on meanwhile) ends too, so it never replays by itself later.
  $effect(() => {
    const el = hop;
    if (!el) return;
    el.addEventListener("animationcancel", boingEnded);
    return () => el.removeEventListener("animationcancel", boingEnded);
  });
</script>

{#if show}
  <div class="focus-mascot" style:--bubble-tail-x="{TAIL_X}px" style:gap="{BUBBLE_GAP}px">
    <div class="bubble-spot" style:margin-bottom={lift > 0 ? `${lift}px` : undefined}>
      <SpeechBubble {speech} />
    </div>
    <!-- The axolotl is the button (its picture is decorative: the label says what a click does). -->
    <button type="button" class="tickle" aria-label={strings.mascot.tickle} onclick={tickle}>
      <span class="lift">
        <span
          class="hop"
          class:boing={boinging}
          bind:this={hop}
          onanimationend={boingEnded}
        >
          <Axolotl size={MASCOT_SIZE} frame={mascot.frame} sit />
        </span>
      </span>
    </button>
  </div>
{/if}

<style>
  /*
   * Fills the lane the page gives it (left of the ring, as tall as the hero's content): the axolotl stands on the
   * lane's bottom line, the bubble fills the room above it. Only the bubble and the axolotl take clicks.
   */
  .focus-mascot {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-end;
    height: 100%;
    pointer-events: none;
  }

  /* Above the axolotl's head; shrinks (and the text scrolls inside the bubble) when the text is long. */
  .bubble-spot {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    min-height: 0;
    min-width: calc(var(--bubble-tail-x) + 34px);
    max-width: 100%;
  }

  /* No button chrome: just the axolotl, with a hand cursor and a focus ring in the theme's accent. */
  .tickle {
    display: block;
    flex: none;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    pointer-events: auto;
    -webkit-tap-highlight-color: transparent;
  }

  /* The app's focus ring (app.css: 2 px, dashed 3 px in the 8-bit theme) in the theme's accent. */
  .tickle:focus-visible {
    outline-color: var(--accent);
    outline-offset: 4px;
  }

  /* Hover: a tiny lift (its own wrapper, so it never fights the boing). */
  .lift {
    display: block;
    transition: transform 150ms ease-out;
  }

  .tickle:hover .lift {
    transform: translateY(-3px);
  }

  /* The boing transforms this wrapper, never the art (in the 8-bit theme the art is a canvas). */
  .hop {
    display: block;
    transform-origin: 50% 100%;
  }

  .hop.boing {
    animation: boing 700ms both;
  }

  /*
   * Squash-and-stretch from the feet, the same shape as the popup's peeking axolotl landing (`peek-enter` from its
   * stretch on): a quick squash, a stretch lifted a few pixels, a settling squash, a small rebound, rest.
   */
  @keyframes boing {
    0% {
      transform: none;
      animation-timing-function: cubic-bezier(0.3, 0, 0.5, 1);
    }
    14% {
      transform: scale(1.08, 0.9);
      animation-timing-function: cubic-bezier(0.2, 0.75, 0.3, 1);
    }
    44% {
      transform: translateY(-6px) scale(0.9, 1.12);
      animation-timing-function: cubic-bezier(0.4, 0, 0.5, 1);
    }
    66% {
      transform: translateY(0) scale(1.06, 0.94);
      animation-timing-function: cubic-bezier(0.4, 0, 0.5, 1);
    }
    84% {
      transform: translateY(-1px) scale(0.98, 1.02);
      animation-timing-function: cubic-bezier(0.4, 0, 0.5, 1);
    }
    100% {
      transform: none;
    }
  }

  /* Reduced motion: no boing and no lift (the smile and the whole text at once still answer the click). */
  @media (prefers-reduced-motion: reduce) {
    .lift {
      transition: none;
    }

    .tickle:hover .lift {
      transform: none;
    }

    .hop.boing {
      animation: none;
    }
  }
</style>
