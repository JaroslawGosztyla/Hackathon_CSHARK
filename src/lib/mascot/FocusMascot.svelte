<script lang="ts">
  // Skupienie page: the axolotl beside the timer. Idle, it says the top "Co dalej?" suggestion (when a tool is
  // connected and has one) or the good practice of the day; it winks at Start and cheers a completed cycle. During a
  // break it moves into the break card (BreakCard), so one axolotl is on screen at a time.
  // Absolutely positioned next to the timer ring: showing, hiding or talking never moves the page.
  import { MediaQuery } from "svelte/reactivity";
  import { untrack } from "svelte";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { work, workEnabled } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";
  import { workingTipOfDay } from "$lib/tips";
  import Axolotl from "./Axolotl.svelte";
  import { useMascot } from "./controller.svelte";
  import { followCues, mascotEnabled } from "./cues.svelte";
  import SpeechBubble from "./SpeechBubble.svelte";

  /** Beside the ring on a wide window; smaller, in the ring's corner and without the bubble on a narrow one. */
  const SIZE_WIDE = 140;
  const SIZE_NARROW = 84;

  const narrow = new MediaQuery("(max-width: 719px)");
  const mascot = useMascot("main");
  followCues(mascot, ["start", "cycle"]);

  // Read once per mount, like the "Dobra praktyka" card below.
  const tip = workingTipOfDay(new Date());

  const show = $derived(mascotEnabled(config.value) && session.phase !== "break");
  const topItem = $derived(workEnabled(config.value) ? (work.status?.items[0] ?? null) : null);
  const speech = $derived.by((): { label: string; text: string } | null => {
    if (!session.isIdle) return null;
    if (topItem) return { label: strings.mascot.nextUpLabel, text: strings.work.itemName(topItem.key, topItem.title) };
    if (tip) return { label: strings.mascot.practiceLabel, text: tip.pl };
    return null;
  });

  /** The bubble is shown (it does not fit next to the ring on a narrow window). */
  const bubble = $derived(show && !narrow.current ? speech : null);

  // A string, so a refresh with the same text (e.g. the same top work item after a sync) does not talk again.
  const bubbleText = $derived(bubble?.text ?? null);

  // Says the text when the bubble appears or changes; stops when it goes away (Start, hidden mascot, narrow window).
  $effect(() => {
    const text = bubbleText;
    untrack(() => (text ? mascot.talk(text) : mascot.hush()));
  });
</script>

{#if show}
  <div class="focus-mascot">
    {#if bubble}
      <div class="bubble-spot">
        <SpeechBubble label={bubble.label} text={bubble.text} live />
      </div>
    {/if}
    <Axolotl size={narrow.current ? SIZE_NARROW : SIZE_WIDE} frame={mascot.frame} sit />
  </div>
{/if}

<style>
  /* Next to the ring (its wrapper is the positioning box), feet on the ring's bottom line. */
  .focus-mascot {
    position: absolute;
    left: calc(100% + 6px);
    bottom: 0;
  }

  /* Above the axolotl's head, growing upwards; never wider than the space right of the ring. */
  .bubble-spot {
    position: absolute;
    left: 6px;
    bottom: calc(100% + 10px);
    width: max-content;
    max-width: min(270px, calc(50vw - 150px - var(--gutter) - 20px));
  }

  @media (max-width: 719px) {
    .focus-mascot {
      left: auto;
      right: -22px;
    }
  }
</style>
