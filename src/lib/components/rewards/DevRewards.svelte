<script lang="ts">
  // Developer page → "Rewards (beta)" (ADR-019, ADR-022). English only. Sample moments and statuses live in the
  // rewards store's sample layer (memory only, `sample: true`): never acknowledged, sent to the shell or written. The
  // popups use the existing `dev_trigger` popup; the sounds ask the shell (`play_reward_sound`).
  import * as api from "$lib/api";
  import DevCollection from "$lib/components/collection/DevCollection.svelte";
  import { DEV, DEV_REWARDS } from "$lib/i18n/dev";
  import { rewardCues } from "$lib/mascot/cues.svelte";
  import { POSE_NAMES, poseFrame, reactionSteps, rewardSteps } from "$lib/mascot/frames";
  import { scopeOf } from "$lib/rewards/choreography";
  import { config } from "$lib/state/config.svelte";
  import { dev } from "$lib/state/dev.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import {
    sampleRewardMoment,
    sampleRewardsStatus,
    sampleSteps,
    sampleSummaries,
    type RewardMomentSample,
    type RewardsStatusSample,
  } from "$lib/state/samples";
  import { summary } from "$lib/state/summary.svelte";
  import type { CelebrationLevel, RewardSound } from "$lib/types";

  /** A pose button holds the pose this long on the Skupienie axolotl. */
  const POSE_HOLD_MS = 5000;
  const LEVELS: readonly CelebrationLevel[] = ["full", "calm", "numbersOnly"];
  const SOUNDS: readonly RewardSound[] = ["pop", "sparkle", "fanfare"];

  const d = DEV_REWARDS;

  /** A sample summary with steps and sparks, so Podsumowanie has a session card to show the recap in. */
  function ensureSummary(now: number): void {
    if (summary.selected !== null && summary.sample === null) return;
    const samples = sampleSummaries(now);
    const last = samples.last ? { ...samples.last, steps: sampleSteps(), sparks: 134 } : null;
    summary.showSample({
      last,
      history: samples.history.map((s) => (last && s.sessionId === last.sessionId ? last : s)),
    });
  }

  /**
   * Plays a sample moment on its page: opens it, then adds the moment once the page has mounted (the layout runs the
   * request after the tab slide), so the celebration starts with the page (the break recap also shows as a chip under
   * the session buttons; a small window may need a scroll for the card itself).
   */
  function moment(kind: RewardMomentSample, label: string): void {
    const now = Date.now();
    const sample = sampleRewardMoment(kind, now);
    const scope = scopeOf(sample.kind);
    if (scope === "summary") ensureSummary(now);
    void dev.request(
      scope === "summary" ? "/summary" : "/",
      {
        target: "layout",
        run: () => {
          rewards.addSample({ ...sample, atMs: Date.now() }, () => sampleRewardsStatus("today", Date.now()));
          // The early Stop shows the axolotl holding a heart, or a smile with numbers only (as the summary does for real
          // sessions); calm and reduced motion: one still pose.
          if (kind === "earlyStop") {
            rewardCues.emit(
              rewards.celebration === "numbersOnly"
                ? reactionSteps("main", "smile", true)
                : rewardSteps("earlyStop", rewards.celebration !== "full"),
            );
          }
        },
      },
      label,
    );
  }

  function status(kind: RewardsStatusSample, label: string): void {
    rewards.sampleStatus = sampleRewardsStatus(kind, Date.now());
    dev.ok(DEV.sent(label));
    void dev.go("/");
  }

  function clear(): void {
    rewards.clearSamples();
    dev.ok(d.cleared);
  }

  let poseTimer: ReturnType<typeof setTimeout> | null = null;

  /** Holds a pose on the Skupienie axolotl; it shows even with the beta off while it is held. */
  function pose(name: (typeof POSE_NAMES)[number]): void {
    const label = d.pose(name);
    void dev.request(
      "/",
      {
        target: "layout",
        run: () => {
          rewards.posePreview = true;
          if (poseTimer !== null) clearTimeout(poseTimer);
          poseTimer = setTimeout(() => {
            poseTimer = null;
            rewards.posePreview = false;
          }, POSE_HOLD_MS);
          rewardCues.emit([{ frame: poseFrame(name), ms: POSE_HOLD_MS }]);
        },
      },
      label,
    );
  }

  async function sound(s: RewardSound): Promise<void> {
    const label = d.sounds[s];
    try {
      await api.playRewardSound(s);
      dev.ok(DEV.sent(label));
    } catch (e) {
      dev.fail(label, e);
    }
  }

  function setLevel(level: CelebrationLevel | null, label: string): void {
    rewards.levelOverride = level;
    dev.ok(DEV.sent(label));
  }

  function toggleReduced(): void {
    rewards.simulateReduced = !rewards.simulateReduced;
    dev.ok(DEV.sent(d.reduced));
  }

  function intro(): void {
    rewards.showIntroAgain();
    dev.ok(DEV.sent(d.intro));
    void dev.go("/");
  }
</script>

<section class="card group" aria-labelledby="dev-rewards">
  <div class="card-title">
    <div>
      <h2 id="dev-rewards">{d.title}</h2>
      <p class="hint">{d.hint}</p>
    </div>
  </div>

  <fieldset class="set">
    <legend>{d.smallLabel}</legend>
    <div class="buttons">
      {#each d.small as item (item.kind)}
        <button type="button" class="btn btn-sm" onclick={() => moment(item.kind, item.label)}>{item.label}</button>
      {/each}
    </div>
  </fieldset>

  <fieldset class="set">
    <legend>{d.burstsLabel}</legend>
    <div class="buttons">
      {#each d.bursts as item (item.kind)}
        <button type="button" class="btn btn-sm" onclick={() => moment(item.kind, item.label)}>{item.label}</button>
      {/each}
    </div>
  </fieldset>

  <fieldset class="set">
    <legend>{d.statusLabel}</legend>
    <div class="buttons">
      {#each d.statuses as item (item.kind)}
        <button type="button" class="btn btn-sm" onclick={() => status(item.kind, item.label)}>{item.label}</button>
      {/each}
      <button type="button" class="btn btn-sm btn-danger" disabled={!rewards.sampling} onclick={clear}>{d.clear}</button>
    </div>
  </fieldset>

  <fieldset class="set">
    <legend>{d.popupsLabel}</legend>
    <p class="hint">{d.popupsHint}</p>
    <div class="buttons">
      <button
        type="button"
        class="btn btn-sm"
        onclick={() => dev.popup({ kind: "breakStarted", leadMinutes: null, label: d.popupBreak })}
      >
        {d.popupBreak}
      </button>
      <button
        type="button"
        class="btn btn-sm"
        onclick={() => dev.popup({ kind: "focusStarted", leadMinutes: null, label: d.popupFocus })}
      >
        {d.popupFocus}
      </button>
    </div>
  </fieldset>

  <fieldset class="set">
    <legend>{d.posesLabel}</legend>
    <p class="hint">{d.posesHint}</p>
    <div class="buttons">
      {#each POSE_NAMES as name (name)}
        <button type="button" class="btn btn-sm" onclick={() => pose(name)}>{d.pose(name)}</button>
      {/each}
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={dev.mascotMood === "thinking"}
        aria-pressed={dev.mascotMood === "thinking"}
        onclick={() => dev.holdMood("thinking", d.moodThinking)}
      >
        {d.moodThinking}
      </button>
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={dev.mascotMood === "working"}
        aria-pressed={dev.mascotMood === "working"}
        onclick={() => dev.holdMood("working", d.moodWorking)}
      >
        {d.moodWorking}
      </button>
      <button
        type="button"
        class="btn btn-sm btn-ghost"
        disabled={dev.mascotMood === null}
        onclick={() => dev.holdMood(null, DEV.autoMood)}
      >
        {DEV.autoMood}
      </button>
    </div>
  </fieldset>

  <fieldset class="set" disabled={!config.ready}>
    <legend>{d.soundsLabel}</legend>
    <p class="hint">{d.soundsHint}</p>
    <div class="buttons">
      {#each SOUNDS as s (s)}
        <button type="button" class="btn btn-sm" onclick={() => sound(s)}>{d.sounds[s]}</button>
      {/each}
    </div>
  </fieldset>

  <fieldset class="set">
    <legend>{d.lookLabel}</legend>
    <p class="hint">{d.levelsHint}</p>
    <div class="buttons">
      {#each LEVELS as level (level)}
        <button
          type="button"
          class="btn btn-sm"
          class:btn-primary={rewards.levelOverride === level}
          aria-pressed={rewards.levelOverride === level}
          onclick={() => setLevel(level, d.levels[level])}
        >
          {d.levels[level]}
        </button>
      {/each}
      <button
        type="button"
        class="btn btn-sm btn-ghost"
        disabled={rewards.levelOverride === null}
        onclick={() => setLevel(null, d.levelAuto)}
      >
        {d.levelAuto}
      </button>
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={rewards.simulateReduced}
        aria-pressed={rewards.simulateReduced}
        onclick={toggleReduced}
      >
        {d.reduced}
      </button>
      <button type="button" class="btn btn-sm" onclick={intro}>{d.intro}</button>
    </div>
  </fieldset>

  <!-- Beta collection (ADR-024): ambience samples, spooky one-shots, the sample month. -->
  <DevCollection />
</section>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group .card-title {
    margin-bottom: 0;
  }

  .group .card-title .hint {
    margin-top: 4px;
  }

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
</style>
