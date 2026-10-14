<script lang="ts" module>
  /** Height of the wizard's axolotl box in CSS pixels (a pose may reach a little past it, see Axolotl.svelte). */
  export const WIZARD_MASCOT_SIZE = 150;
</script>

<script lang="ts">
  // The first-run wizard's axolotl (ADR-023): one of the owner's poses per step, a boing when the step opens (the
  // tickle's `mascot-boing`, app.css) and the step's line in a bubble above it, revealed letter by letter like the main
  // mascot's (a click shows it all; with reduced motion it appears at once and nothing moves). A pose is one drawing
  // without mouth frames, so the speech reveals the text only. The art is only displayed (CLAUDE.md rule 10).
  import { untrack } from "svelte";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { useMascot } from "$lib/mascot/controller.svelte";
  import { poseFrame, type PoseName } from "$lib/mascot/frames";
  import { useSpeech } from "$lib/mascot/speech.svelte";
  import SpeechBubble from "$lib/mascot/SpeechBubble.svelte";

  interface Props {
    pose: PoseName;
    /**
     * What Axi says on this step, one bubble after another (the next one shortly after the previous one is fully
     * shown); said again from the first when they change (e.g. a new language on the first step).
     */
    lines: readonly string[];
    /** Axi's recording for each bubble (same order as `lines`; empty = text only, see `voice.ts`). */
    voices?: readonly string[];
    /** Voice volume 0..1 (the app's sound volume). */
    volume?: number;
  }

  let { pose, lines, voices = [], volume = 1 }: Props = $props();

  /** Pause between a fully shown (and fully spoken) bubble and the next one of the same step. */
  const NEXT_LINE_DELAY_MS = 900;

  // The controller only drives the speech's timing here (its mouth frames are never shown over a pose).
  const mascot = useMascot("main");
  const speech = useSpeech(mascot, { persist: true });

  /** The bubble of `lines` shown now. */
  let index = $state(0);
  /** The recording of the current bubble has ended (or there is none, or it could not play). */
  let voiceDone = $state(true);
  let audio: HTMLAudioElement | null = null;

  function stopVoice(): void {
    if (audio) {
      audio.onended = null;
      audio.onerror = null;
      audio.pause();
    }
    voiceDone = true;
  }

  function playVoice(url: string | undefined): void {
    stopVoice();
    if (!url || volume <= 0) return;
    audio ??= new Audio();
    const el = audio;
    el.src = url;
    el.volume = Math.min(1, Math.max(0, volume));
    el.onended = () => (voiceDone = true);
    el.onerror = () => (voiceDone = true);
    voiceDone = false;
    el.play().catch(() => (voiceDone = true));
  }

  /** Shows bubble `i` and plays its recording. */
  function speak(i: number): void {
    const text = lines[i];
    if (text === undefined) return;
    index = i;
    speech.say({ text, urgent: true, tag: "onboarding" });
    playVoice(voices[i]);
  }

  $effect(() => {
    void lines;
    void voices;
    untrack(() => speak(0));
  });

  // The next bubble follows once the current one is fully shown and spoken (a click on the bubble shows its text at
  // once; the recording still plays to its end).
  $effect(() => {
    if (!speech.complete || !voiceDone) return;
    if (lines[index + 1] === undefined) return;
    const next = index + 1;
    const timer = setTimeout(() => speak(next), NEXT_LINE_DELAY_MS);
    return () => clearTimeout(timer);
  });

  // A new step, Skip or closing the wizard silences Axi at once.
  $effect(() => () => {
    stopVoice();
    audio = null;
  });
</script>

<div class="wizard-mascot">
  <div class="bubble-spot">
    <SpeechBubble {speech} />
  </div>
  <!-- A new pose mounts a new wrapper: its boing plays once, on entry. -->
  {#key pose}
    <span class="hop">
      <Axolotl size={WIZARD_MASCOT_SIZE} frame={poseFrame(pose)} sit poses />
    </span>
  {/key}
</div>

<style>
  /* The bubble fills the room above the axolotl (its text scrolls inside when long); the axolotl stands at the bottom. */
  .wizard-mascot {
    /* The bubble's tail points at the axolotl's head: the middle of the column. */
    --bubble-tail-x: calc(50% - 5px);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 26px;
    height: 100%;
    min-height: 0;
  }

  .bubble-spot {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    width: 100%;
    min-height: 0;
  }

  /* The boing transforms this wrapper, never the art (in the 8-bit theme the art is a canvas). */
  .hop {
    display: block;
    flex: none;
    transform-origin: 50% 100%;
    animation: mascot-boing var(--mascot-boing-ms) both;
  }

  @media (prefers-reduced-motion: reduce) {
    .hop {
      animation: none;
    }
  }
</style>
