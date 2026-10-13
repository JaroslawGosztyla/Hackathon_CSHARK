// Registries of the collection themes' art (ADR-024): `src/lib/themes/<id>/Scene.svelte` (the window background) and
// `src/lib/themes/<id>/Preview.svelte` (the picker card), found by folder name. Only the main window imports this (the
// popup needs the tokens only, `./catalog.ts`). A theme without one of the files has none (`undefined`).
import type { Component } from "svelte";
import type { RewardTheme } from "$lib/types";
import { isRewardTheme } from "./catalog";

/** `./halloween/Scene.svelte` → `halloween` (a folder that is not a `RewardTheme` is ignored). */
function byTheme(modules: Record<string, Component>): Partial<Record<RewardTheme, Component>> {
  const out: Partial<Record<RewardTheme, Component>> = {};
  for (const [path, component] of Object.entries(modules)) {
    const id = path.split("/")[1];
    if (isRewardTheme(id)) out[id] = component;
  }
  return out;
}

/**
 * Window backgrounds. Contract for every Scene: purely visual (Background puts it in an `aria-hidden` layer), fills its
 * parent (`position: absolute; inset: 0`), draws with tokens / its own palette, animates only `transform` and `opacity`,
 * and is static under `prefers-reduced-motion: reduce`. Polished at 960 × 640 and 1920 × 1080.
 */
export const REWARD_SCENES = byTheme(
  import.meta.glob<Component>("./*/Scene.svelte", { eager: true, import: "default" }),
);

/** Picker cards: the same art reduced to a small tile (about 168 × 100), light enough for ten at once. */
export const REWARD_PREVIEWS = byTheme(
  import.meta.glob<Component>("./*/Preview.svelte", { eager: true, import: "default" }),
);
