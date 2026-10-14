<script lang="ts">
  // Drives the beta ambience engine (ADR-024) from the follow rule in `state/ambience.svelte.ts`. No markup; lives in
  // the main window's dock (MusicPlayer), so the phase popup never loads the engine.
  import { untrack } from "svelte";
  import { ambienceEngine } from "$lib/ambience/engine.svelte";
  import { ambience } from "$lib/state/ambience.svelte";
  import { music } from "$lib/state/music.svelte";

  $effect(() => {
    const target = ambience.target;
    untrack(() => ambienceEngine.sync(target));
  });

  // The beta or the master switch went off: what the user started alone never resumes by itself later.
  $effect(() => {
    if (!ambience.betaOn || !ambience.cfg.enabled) untrack(() => ambience.reset());
  });

  // "Zatrzymaj" while it went with the music lasts until the music stops.
  $effect(() => {
    if (!music.audible) untrack(() => (ambience.held = false));
  });

  $effect(() => () => {
    ambienceEngine.sync(null);
    ambienceEngine.stopSample();
  });
</script>
