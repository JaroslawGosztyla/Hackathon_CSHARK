<script lang="ts">
  // What the mascot says, in a bubble with a tail toward the mascot. The text appears letter by letter (RevealText);
  // a click shows it all at once. Screen readers get the full text once, from a polite live region that stays in the
  // page (the bubble itself is visual only). A long text scrolls inside the bubble, following the newest letter, so
  // the bubble never grows past the room its parent gives it.
  import RevealText from "./RevealText.svelte";
  import type { MascotSpeech } from "./speech.svelte";

  interface Props {
    speech: MascotSpeech;
    /** Announce each new message politely. */
    live?: boolean;
    /** Where the tail points: down to a mascot below the bubble, or left to a mascot beside it. */
    tail?: "bottom" | "left";
  }

  let { speech, live = true, tail = "bottom" }: Props = $props();

  const line = $derived(speech.line);
  const announced = $derived(line ? (line.label ? `${line.label}: ${line.text}` : line.text) : "");

  let textEl = $state<HTMLElement | null>(null);

  // Keeps the newest letter in view while a long text fills the bubble.
  $effect(() => {
    const n = speech.revealed;
    const box = textEl;
    if (!box || n === 0 || box.scrollHeight <= box.clientHeight) return;
    const letter = box.querySelectorAll<HTMLElement>(".ch")[n - 1];
    if (!letter) return;
    const bottom = letter.offsetTop + letter.offsetHeight;
    if (bottom > box.scrollTop + box.clientHeight) box.scrollTop = bottom - box.clientHeight;
  });
</script>

{#if live}
  <p class="sr-only" aria-live="polite" aria-atomic="true">{announced}</p>
{/if}

{#if line}
  {#key line.id}
    <!-- Visual only (the live region above has the text): a click shows the rest of the text at once. -->
    <div
      class="bubble"
      class:tail-left={tail === "left"}
      class:leaving={speech.leaving}
      class:revealing={!speech.complete}
      aria-hidden="true"
      onclick={() => speech.skip()}
    >
      {#if line.label}<p class="label">{line.label}</p>{/if}
      <p class="text" bind:this={textEl}><RevealText text={line.text} {speech} /></p>
    </div>
  {/key}
{/if}

<style>
  .bubble {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
    max-height: 100%;
    padding: 9px 12px 10px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text-2);
    box-shadow: var(--shadow-sm);
    font-size: 0.86rem;
    line-height: 1.4;
    text-align: left;
    pointer-events: auto;
    transition: opacity 0.3s ease;
    animation: bubble-in 0.22s ease-out;
  }

  .bubble.revealing {
    cursor: pointer;
  }

  .bubble.leaving {
    opacity: 0;
  }

  /* The tail: a small square turned 45°, showing only its two outer borders. */
  .bubble::after {
    content: "";
    position: absolute;
    left: 28px;
    bottom: -6px;
    width: 10px;
    height: 10px;
    background: var(--surface-raised);
    border-right: var(--border-w) solid var(--border-strong);
    border-bottom: var(--border-w) solid var(--border-strong);
    transform: rotate(45deg);
  }

  .bubble.tail-left::after {
    left: -6px;
    bottom: auto;
    top: 18px;
    border-right: 0;
    border-bottom: var(--border-w) solid var(--border-strong);
    border-left: var(--border-w) solid var(--border-strong);
  }

  .label {
    flex: none;
    margin-bottom: 2px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--accent-strong);
  }

  .text {
    position: relative;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    overflow-wrap: anywhere;
    /* A message can have several lines (a break: the reminder, then the tip). */
    white-space: pre-line;
    scrollbar-width: thin;
    user-select: none;
  }

  @keyframes bubble-in {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.98);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bubble {
      animation: none;
    }
  }
</style>
