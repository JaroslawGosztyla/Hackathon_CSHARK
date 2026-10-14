// Beta ambience (ADR-024): who wants the ambience on and what the mixer writes. Memory only, except the mixer itself
// (`Config.ambience`, saved like every setting). The follow rule (spec §6.3):
//   it plays iff the beta is on, the master switch is on, the music is audible or the user started it alone
//   ("Graj same"), the session does not mute the music for a meeting, the user did not stop it ("Zatrzymaj"), and at
//   least one usable layer (free or unlocked) is on.
// Nothing ever starts by itself when the app starts: `alone` starts false and the music never autoplays. The engine
// (`$lib/ambience/engine.svelte`) is driven by AmbienceHost.svelte in the main window's dock; the popup never loads it.
import { AMBIENCE_LAYERS, ambienceConfig, isLayerUsable, mixOf } from "$lib/ambience/catalog";
import type { AmbienceTarget } from "$lib/ambience/engine.svelte";
import { unlockAudio } from "$lib/player/noise";
import type { AmbienceLayer, AmbienceMix, Config } from "$lib/types";
import { config } from "./config.svelte";
import { music } from "./music.svelte";
import { rewards } from "./rewards.svelte";
import { session } from "./session.svelte";

class AmbienceStore {
  /** The user started the ambience on its own ("Graj same"): it plays without music. */
  alone = $state(false);
  /** The user stopped it ("Zatrzymaj") while it went with the music: silent until the music stops or "Graj same". */
  held = $state(false);
  /** The mixer popover above the dock is open. */
  mixerOpen = $state(false);

  /** `Config.betaFeatures`: without it the ambience is not listed and never plays. */
  betaOn = $derived(config.value?.betaFeatures === true);
  /** The stored mixer (defaults with an older shell). */
  cfg = $derived(ambienceConfig(config.value));
  /** The collection the mixer shows: a Developer sample wins over the shell's status. */
  shownUnlocks = $derived(rewards.sampleStatus?.unlocks ?? rewards.real?.unlocks ?? null);

  /** Layers that are on and may play (the real lock states, never a sample), with their volumes. */
  layers = $derived.by(() => {
    const cfg = this.cfg;
    const unlocks = rewards.real?.unlocks;
    return AMBIENCE_LAYERS.flatMap((layer) => {
      const mix = mixOf(cfg, layer);
      return mix.on && isLayerUsable(unlocks, layer) ? [{ layer, volume: mix.volume }] : [];
    });
  });

  /** The follow rule (see the header). */
  wanted = $derived(
    this.betaOn &&
      this.cfg.enabled &&
      !this.held &&
      (music.audible || this.alone) &&
      session.snapshot.musicMuted !== true,
  );

  /** What the engine should play now (null: silence). */
  target = $derived.by((): AmbienceTarget | null => {
    if (!this.wanted || this.layers.length === 0) return null;
    return { master: this.cfg.volume, layers: this.layers };
  });

  /** The layer may play (free or unlocked; for the mixer rows). */
  usable(layer: AmbienceLayer): boolean {
    return isLayerUsable(this.shownUnlocks, layer);
  }

  /** "Graj same": a user gesture, so the audio context may start; turns the master switch on when it is off. */
  playAlone(): void {
    unlockAudio();
    this.held = false;
    this.alone = true;
    if (!this.cfg.enabled) this.setEnabled(true);
  }

  /** "Zatrzymaj": stops what plays now (alone, or with the music until the music stops). */
  stop(): void {
    this.alone = false;
    if (music.audible) this.held = true;
  }

  /** The master switch. Off stops everything at once. */
  setEnabled(on: boolean): void {
    if (!on) {
      this.alone = false;
      this.held = false;
    } else {
      unlockAudio();
    }
    config.update((c) => {
      ensureAmbience(c).enabled = on;
    }, true);
  }

  /** Master volume 0..=1 (debounced save; the sound follows at once). */
  setVolume(volume: number): void {
    config.update((c) => {
      ensureAmbience(c).volume = unit(volume);
    });
  }

  /** One layer's switch or volume. The entry is written the first time the user touches the layer. */
  setLayer(layer: AmbienceLayer, patch: Partial<Pick<AmbienceMix, "on" | "volume">>, immediate = false): void {
    if (patch.on) unlockAudio();
    config.update((c) => {
      const amb = ensureAmbience(c);
      let entry = amb.layers.find((m) => m.layer === layer);
      if (!entry) {
        entry = { ...mixOf(amb, layer) };
        amb.layers.push(entry);
        amb.layers.sort((a, b) => AMBIENCE_LAYERS.indexOf(a.layer) - AMBIENCE_LAYERS.indexOf(b.layer));
        entry = amb.layers.find((m) => m.layer === layer) ?? entry;
      }
      if (patch.on !== undefined) entry.on = patch.on;
      if (patch.volume !== undefined) entry.volume = unit(patch.volume);
    }, immediate);
  }

  /** The beta or the master switch went off: nothing may resume by itself later. */
  reset(): void {
    this.alone = false;
    this.held = false;
  }
}

function unit(value: number): number {
  return Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 0;
}

/** The config's mixer (created with the defaults when an older shell sent none). */
function ensureAmbience(c: Config): Config["ambience"] {
  const current = c.ambience as Config["ambience"] | undefined;
  if (current) return current;
  c.ambience = ambienceConfig(null);
  return c.ambience;
}

export const ambience = new AmbienceStore();
