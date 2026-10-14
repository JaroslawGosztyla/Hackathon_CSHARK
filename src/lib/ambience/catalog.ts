// The ambience layers (ADR-024) as the UI needs them: catalogue order, which ones are free in the beta, a layer's mix
// from the config and whether it may play (its lock state from `RewardsStatus.unlocks`).
import type { AmbienceConfig, AmbienceLayer, AmbienceMix, AmbienceUnlock, UnlocksView } from "$lib/types";

/** Catalogue order (focus-core `AmbienceLayer::ALL`). */
export const AMBIENCE_LAYERS: readonly AmbienceLayer[] = [
  "rain",
  "wind",
  "thunder",
  "cafe",
  "fireplace",
  "forest",
  "ocean",
  "spookyNight",
  "winterCottage",
];

/** Free from the start of the beta (focus-core `AMBIENCE_RULES`: `UnlockRule::Free`). */
export const FREE_AMBIENCE: readonly AmbienceLayer[] = ["rain", "wind"];

/** Mirrors focus-core `config::DEFAULT_AMBIENCE_VOLUME` / `DEFAULT_AMBIENCE_LAYER_VOLUME`. */
export const DEFAULT_AMBIENCE_VOLUME = 0.6;
export const DEFAULT_LAYER_VOLUME = 0.5;

/** The mixer as stored, or the defaults with a config from before ADR-024 (an older shell). */
export function ambienceConfig(cfg: { ambience?: AmbienceConfig | null } | null | undefined): AmbienceConfig {
  return cfg?.ambience ?? { enabled: false, volume: DEFAULT_AMBIENCE_VOLUME, layers: [{ layer: "rain", on: true, volume: DEFAULT_LAYER_VOLUME }] };
}

/** A layer's mix (mirrors `AmbienceConfig::mix`: a layer without an entry is off at the default volume). */
export function mixOf(cfg: AmbienceConfig, layer: AmbienceLayer): AmbienceMix {
  return cfg.layers.find((m) => m.layer === layer) ?? { layer, on: false, volume: DEFAULT_LAYER_VOLUME };
}

/** The layer's entry in the collection view, if the view lists it. */
export function ambienceUnlock(unlocks: UnlocksView | null | undefined, layer: AmbienceLayer): AmbienceUnlock | null {
  return unlocks?.ambiences.find((a) => a.layer === layer) ?? null;
}

/** The layer may play: free, or unlocked in `unlocks`. While the collection is unknown only the free layers may. */
export function isLayerUsable(unlocks: UnlocksView | null | undefined, layer: AmbienceLayer): boolean {
  const item = ambienceUnlock(unlocks, layer);
  if (item) return item.state === "free" || item.state === "unlocked";
  return FREE_AMBIENCE.includes(layer);
}
