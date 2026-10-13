<script lang="ts">
  // Text the mascot says, appearing letter by letter: every letter is laid out from the start (the box never grows
  // while it fills) and fades in when the speech reaches it. The letters are visual only (aria-hidden); the full
  // text is there for screen readers at once. Without an active speech for this text, it is plain text.
  import { graphemes, type MascotSpeech } from "./speech.svelte";

  interface Props {
    text: string;
    speech: MascotSpeech | null;
    /** The mascot will say this text but has not started yet: keep the letters hidden (no flash of the full text). */
    waiting?: boolean;
  }

  let { text, speech, waiting = false }: Props = $props();

  const units = $derived(graphemes(text));
  const active = $derived(speech !== null && speech.line !== null && speech.line.text === text);
  const shown = $derived(active && speech ? speech.revealed : waiting ? 0 : units.length);
  const animated = $derived(active || waiting);
</script>

{#if animated}<span class="sr-only">{text}</span><span class="reveal" aria-hidden="true"
    >{#each units as unit, i (i)}<span class="ch" class:on={i < shown}>{unit}</span>{/each}</span
  >{:else}{text}{/if}

<style>
  .ch {
    opacity: 0;
    transition: opacity 150ms ease-out;
  }

  .ch.on {
    opacity: 1;
  }
</style>
