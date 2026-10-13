<script lang="ts">
  // Developer page → "Rewards cheats (dev)" (ADR-019, ADR-024). English only. Each button is one `dev_trigger`
  // `rewardsCheat`: the shell changes the real ledger (`rewards.json`), or the showcase ledger in presentation mode, and
  // runs the normal unlock rules, so the usual moments follow. The status line reads the rewards status the shell
  // publishes after the cheat (`rewards://changed`).
  import * as api from "$lib/api";
  import { DEV_REWARDS_CHEATS } from "$lib/i18n/dev";
  import { config } from "$lib/state/config.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { showcaseMode } from "$lib/state/showcaseMode.svelte";
  import type { RewardsCheat, RewardsStatus, StickerId } from "$lib/types";

  const d = DEV_REWARDS_CHEATS;

  /** Every sticker (type-checked against `StickerId`), for the "x / y" count. */
  const ALL_STICKERS = {
    firstStep: true,
    dailyGoal: true,
    planner: true,
    goodBreak: true,
    comeback: true,
    amazed: true,
    restDay: true,
    together: true,
    threeDays: true,
    goldenPearl: true,
    level5: true,
  } as const satisfies Record<StickerId, true>;
  const STICKERS_MAX = Object.keys(ALL_STICKERS).length;

  let busy = $state(false);
  let error = $state<string | null>(null);
  /** A cheat was sent this visit: the status line shows from then on. */
  let used = $state(false);

  function summary(s: RewardsStatus): string {
    const lockable = <T extends { state: string }>(items: readonly T[]) => items.filter((i) => i.state !== "free");
    const themes = lockable(s.unlocks.themes);
    const ambiences = lockable(s.unlocks.ambiences);
    return d.status({
      level: s.level.level,
      streak: s.streak.days,
      best: s.streak.best,
      themes: themes.filter((i) => i.state === "unlocked").length,
      themesMax: themes.length,
      ambiences: ambiences.filter((i) => i.state === "unlocked").length,
      ambiencesMax: ambiences.length,
      stickers: s.stickers.length,
      stickersMax: STICKERS_MAX,
    });
  }

  const statusLine = $derived.by((): string | null => {
    if (error !== null) return error;
    if (!used) return null;
    if (!rewards.configActive) return d.off;
    return rewards.real ? summary(rewards.real) : null;
  });

  async function run(cheat: RewardsCheat, label: string): Promise<void> {
    busy = true;
    error = null;
    try {
      // The shell checks the stored `developerMode` and beta switches: a switch flipped a moment ago is saved first.
      await config.flush();
      // The new status arrives with `rewards://changed` (the app-wide subscription), and the line follows it.
      await api.devTrigger({ type: "rewardsCheat", cheat });
    } catch (e) {
      const err = api.toAppError(e);
      console.warn(`[FocusMode] dev: ${err.code}: ${err.message}`);
      error = d.failed(label, err.code);
    } finally {
      used = true;
      busy = false;
    }
  }
</script>

<section class="card group" aria-labelledby="dev-rewards-cheats">
  <div class="card-title">
    <div>
      <h2 id="dev-rewards-cheats">{d.title}</h2>
      <p class="hint">{d.hint}</p>
      {#if showcaseMode.active}
        <p class="hint">{d.showcaseNote}</p>
      {/if}
    </div>
  </div>

  <div class="buttons">
    {#each d.cheats as item (item.cheat)}
      <button type="button" class="btn btn-sm" disabled={busy} onclick={() => run(item.cheat, item.label)}>
        {item.label}
      </button>
    {/each}
    <button
      type="button"
      class="btn btn-sm btn-danger"
      title={d.lockAllTitle}
      disabled={busy}
      onclick={() => run("lockAll", d.lockAll)}
    >
      {d.lockAll}
    </button>
  </div>

  {#if statusLine !== null}
    <p class="status" class:error={error !== null} role="status">{statusLine}</p>
  {/if}
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

  .status {
    margin: 0;
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }

  .status.error {
    color: var(--danger);
  }
</style>
