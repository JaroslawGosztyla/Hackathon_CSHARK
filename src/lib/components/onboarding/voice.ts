// Axi's recorded voice for the first-run wizard: the owner's Polish recordings, one file per bubble (CREDITS.md).
// Other languages show the text only. Files live in static/voice/ (Git LFS) and are served by the app itself.
import type { UiLanguage } from "$lib/i18n/index.svelte";
import type { OnboardingStep } from "$lib/state/onboarding.svelte";

const VOICE_PL: Record<OnboardingStep, readonly string[]> = {
  welcome: ["/voice/pl/axi-setup-01.mp3", "/voice/pl/axi-setup-02.mp3"],
  music: ["/voice/pl/axi-setup-03.mp3"],
  shields: ["/voice/pl/axi-setup-04.mp3"],
  extras: ["/voice/pl/axi-setup-05.mp3"],
};

/** The recordings for a step's bubbles (same order as its `lines`), or none when there is no voice in this language. */
export function wizardVoices(lang: UiLanguage, step: OnboardingStep): readonly string[] {
  return lang === "pl" ? VOICE_PL[step] : [];
}
