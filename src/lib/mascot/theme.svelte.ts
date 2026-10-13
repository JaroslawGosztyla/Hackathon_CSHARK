// The mascot is drawn pixelated in the 8-bit theme. The theme lives on `<html data-theme>` (themes/index.ts) in both
// the main window and the popup window, so this follows the attribute instead of a store.
import { createSubscriber } from "svelte/reactivity";

const subscribe = createSubscriber((update) => {
  const observer = new MutationObserver(update);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observer.disconnect();
});

/** The 8-bit theme is applied; reactive inside effects, deriveds and templates. */
export function isPixelTheme(): boolean {
  subscribe();
  return document.documentElement.dataset.theme === "pixel";
}
