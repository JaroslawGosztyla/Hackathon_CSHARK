<script lang="ts">
  // What the mascot says: real text (selectable, read by screen readers), in a bubble with a tail toward the mascot.
  interface Props {
    text: string;
    /** Small heading above the text (e.g. "Dobra praktyka"). */
    label?: string | null;
    /** Announce a changed text politely (new tips only; the text shown at first is not announced). */
    live?: boolean;
    /** Where the tail points: down to a mascot below the bubble, or left to a mascot beside it. */
    tail?: "bottom" | "left";
  }

  let { text, label = null, live = false, tail = "bottom" }: Props = $props();
</script>

<div
  class="bubble"
  class:tail-left={tail === "left"}
  aria-live={live ? "polite" : undefined}
  aria-atomic={live ? "true" : undefined}
>
  {#if label}<p class="label">{label}</p>{/if}
  <p class="text">{text}</p>
</div>

<style>
  .bubble {
    position: relative;
    padding: 9px 12px 10px;
    border-radius: var(--radius-sm);
    border: var(--border-w) solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text-2);
    box-shadow: var(--shadow-sm);
    font-size: 0.86rem;
    line-height: 1.4;
    text-align: left;
    animation: bubble-in 0.22s ease-out;
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
    margin-bottom: 2px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--accent-strong);
  }

  .text {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 7;
    line-clamp: 7;
    overflow: hidden;
    overflow-wrap: anywhere;
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
