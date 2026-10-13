<script lang="ts">
  // Developer page → Rewards (beta) → collection samples (ADR-019, ADR-024). English only. Ambience: one layer alone
  // for a while (lock state, beta switch and mixer ignored; the config is never written) and the spooky one-shots.
  // Sample month: the Kolekcja grid as in that month (a sample status in the rewards store: memory only, nothing
  // unlocks). Both are listed in "samples shown" and cleared with the other samples.
  import { AMBIENCE_LAYERS } from "$lib/ambience/catalog";
  import { ambienceEngine, SAMPLE_MS } from "$lib/ambience/engine.svelte";
  import type { SpookyOneShot } from "$lib/ambience/layers/spookyNight";
  import { DEV, DEV_COLLECTION } from "$lib/i18n/dev";
  import { unlockAudio } from "$lib/player/noise";
  import { dev } from "$lib/state/dev.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { sampleRewardsStatus, sampleUnlocks } from "$lib/state/samples";
  import type { AmbienceLayer } from "$lib/types";

  const d = DEV_COLLECTION;
  const ONE_SHOTS: readonly SpookyOneShot[] = ["owl", "creak", "thunder"];
  /** The sample month's user: level 4, best streak 5 (Underwater unlocked, the rest still to earn). */
  const SAMPLE_LEVEL = 4;
  const SAMPLE_BEST = 5;

  const uid = $props.id();
  const today = new Date();
  let month = $state(today.getMonth() + 1);

  function sample(layer: AmbienceLayer): void {
    unlockAudio();
    ambienceEngine.startSample(layer, SAMPLE_MS);
    dev.ok(DEV.sent(d.layerNames[layer]));
  }

  function stop(): void {
    ambienceEngine.stopSample();
    dev.ok(DEV.sent(d.stop));
  }

  function oneShot(kind: SpookyOneShot): void {
    unlockAudio();
    ambienceEngine.oneShot(kind);
    dev.ok(DEV.sent(d.oneShots[kind]));
  }

  function showMonth(): void {
    const now = Date.now();
    const year = today.getFullYear();
    const label = d.monthSent(d.months[month - 1] ?? String(month), year);
    rewards.sampleStatus = {
      ...sampleRewardsStatus("today", now),
      unlocks: sampleUnlocks(month, year, SAMPLE_LEVEL, SAMPLE_BEST, now),
    };
    void dev.request(
      "/settings",
      {
        target: "layout",
        run: () => document.getElementById("appearance-settings")?.scrollIntoView({ block: "start" }),
      },
      label,
    );
  }
</script>

<fieldset class="set">
  <legend>{d.ambienceLabel}</legend>
  <p class="hint">{d.ambienceHint(Math.round(SAMPLE_MS / 1000))}</p>
  <div class="buttons">
    {#each AMBIENCE_LAYERS as layer (layer)}
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={ambienceEngine.sample === layer}
        aria-pressed={ambienceEngine.sample === layer}
        onclick={() => sample(layer)}
      >
        {d.layerNames[layer]}
      </button>
    {/each}
    <button type="button" class="btn btn-sm" disabled={ambienceEngine.sample === null} onclick={stop}>{d.stop}</button>
  </div>
</fieldset>

<fieldset class="set">
  <legend>{d.oneShotsLabel}</legend>
  <div class="buttons">
    {#each ONE_SHOTS as kind (kind)}
      <button type="button" class="btn btn-sm" onclick={() => oneShot(kind)}>{d.oneShots[kind]}</button>
    {/each}
  </div>
</fieldset>

<fieldset class="set">
  <legend>{d.monthLabel}</legend>
  <p class="hint">{d.monthHint}</p>
  <div class="buttons">
    <label class="month" for="{uid}-month">{d.monthPick}</label>
    <select id="{uid}-month" class="select" bind:value={month}>
      {#each d.months as name, i (name)}
        <option value={i + 1}>{name}</option>
      {/each}
    </select>
    <button type="button" class="btn btn-sm" onclick={showMonth}>{d.monthShow}</button>
  </div>
</fieldset>

<style>
  .buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .buttons .btn {
    white-space: normal;
    text-align: center;
  }

  .set {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    margin: 0;
    padding: 12px 0 0;
    border: 0;
    border-top: 1px solid var(--border);
  }

  .set legend {
    float: left;
    width: 100%;
    padding: 0;
    margin-bottom: 4px;
    font-weight: 650;
    font-size: 0.9rem;
  }

  .month {
    font-weight: 600;
  }

  .select {
    width: auto;
  }
</style>
