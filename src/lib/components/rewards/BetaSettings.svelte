<script lang="ts">
  // Settings → "Funkcje beta" (ADR-022): the master switch (off by default) and, while it is on, sparks / levels /
  // streak with their options, and small steps. Turning anything off hides its UI and keeps the data (days while the
  // rewards are off count as paused). "Zacznij od nowa" is the only reset, never suggested anywhere else.
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { intlLocale } from "$lib/i18n/index.svelte";
  import { config } from "$lib/state/config.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import type { CelebrationLevel, Config, DailyGoal, Weekday } from "$lib/types";
  import SurpriseOdds from "./SurpriseOdds.svelte";

  const DAILY_GOALS: readonly DailyGoal[] = ["light", "regular", "strong"];
  const LEVELS: readonly CelebrationLevel[] = ["full", "calm", "numbersOnly"];
  const WEEKDAYS: readonly Weekday[] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  /** Mirrors focus-core `RewardsConfig::normalized` (at most 6 days off: one work day stays). */
  const MAX_REST_WEEKDAYS = 6;
  /** 2024-01-01 was a Monday: the reference week for the weekday names. */
  const MONDAY_UTC = Date.UTC(2024, 0, 1, 12);
  const DAY_MS = 86_400_000;

  const uid = $props.id();
  const s = strings.rewards;

  const cfg = $derived(config.value);
  const r = $derived(cfg?.rewards ?? null);

  const dayNames = $derived.by(() => {
    const format = new Intl.DateTimeFormat(intlLocale(), { weekday: "short", timeZone: "UTC" });
    return WEEKDAYS.map((day, i) => ({ day, name: format.format(new Date(MONDAY_UTC + i * DAY_MS)) }));
  });

  /** The 7th day off was refused: the inline note shows until the next change. */
  let limitShown = $state(false);
  let resetOpen = $state(false);

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }

  function toggleDay(day: Weekday, on: boolean): void {
    const current = r?.restWeekdays ?? [];
    if (on && !current.includes(day) && current.length >= MAX_REST_WEEKDAYS) {
      limitShown = true;
      return;
    }
    limitShown = false;
    const next = on ? [...current, day] : current.filter((d) => d !== day);
    set((c) => (c.rewards.restWeekdays = WEEKDAYS.filter((d) => next.includes(d))));
  }
</script>

{#if cfg && r}
  <Switch
    id="beta-master"
    label={s.settings.master}
    hint={s.settings.intro}
    checked={cfg.betaFeatures === true}
    onchange={(on) => set((c) => (c.betaFeatures = on))}
  >
    {#if cfg.betaFeatures}
      <div class="items">
        <div class="item">
          <Switch
            id="beta-rewards"
            label={s.settings.rewards}
            hint={s.settings.rewardsHint}
            checked={r.enabled}
            onchange={(on) => set((c) => (c.rewards.enabled = on))}
          >
            {#if r.enabled}
              <div class="options">
                <div class="field">
                  <span class="field-label" id="{uid}-goal">{s.settings.dailyGoal}</span>
                  <fieldset class="segmented" aria-describedby="{uid}-goal-hint">
                    <legend>{s.settings.dailyGoal}</legend>
                    {#each DAILY_GOALS as goal (goal)}
                      <label>
                        <input
                          type="radio"
                          name="{uid}-daily-goal"
                          value={goal}
                          checked={r.dailyGoal === goal}
                          onchange={() => set((c) => (c.rewards.dailyGoal = goal))}
                        />
                        <span>{s.settings.dailyGoalPreset[goal]}</span>
                      </label>
                    {/each}
                  </fieldset>
                  <p class="hint" id="{uid}-goal-hint">{s.settings.dailyGoalHint}</p>
                </div>

                <div class="field">
                  <fieldset class="days" aria-describedby="{uid}-days-hint">
                    <legend class="field-label">{s.settings.restWeekdays}</legend>
                    <div class="chips">
                      {#each dayNames as { day, name } (day)}
                        <label class="day" class:on={r.restWeekdays.includes(day)}>
                          <input
                            type="checkbox"
                            checked={r.restWeekdays.includes(day)}
                            onchange={(e) => {
                              const input = e.currentTarget;
                              toggleDay(day, input.checked);
                              input.checked = r.restWeekdays.includes(day);
                            }}
                          />
                          <span>{name}</span>
                        </label>
                      {/each}
                    </div>
                  </fieldset>
                  <p class="hint" id="{uid}-days-hint">{s.settings.restWeekdaysHint}</p>
                  {#if limitShown}
                    <p class="hint limit" role="status">{s.settings.restWeekdaysLimit}</p>
                  {/if}
                </div>

                <Switch
                  id="beta-show-streak"
                  label={s.settings.showStreak}
                  hint={s.settings.showStreakHint}
                  checked={r.showStreak}
                  onchange={(on) => set((c) => (c.rewards.showStreak = on))}
                />
                <Switch
                  id="beta-pause-streak"
                  label={s.settings.pauseStreak}
                  hint={s.settings.pauseStreakHint}
                  checked={r.streakPaused}
                  onchange={(on) => set((c) => (c.rewards.streakPaused = on))}
                />
                <Switch
                  id="beta-surprises"
                  label={s.settings.surprises}
                  checked={r.surprises}
                  onchange={(on) => set((c) => (c.rewards.surprises = on))}
                >
                  {#if rewards.real}
                    <SurpriseOdds odds={rewards.real.odds} collection={rewards.real.collection} labelled align="start" />
                  {/if}
                </Switch>

                <div class="field">
                  <span class="field-label">{s.settings.celebrations}</span>
                  <fieldset class="segmented" aria-describedby="{uid}-celebrations-hint">
                    <legend>{s.settings.celebrations}</legend>
                    {#each LEVELS as level (level)}
                      <label>
                        <input
                          type="radio"
                          name="{uid}-celebrations"
                          value={level}
                          checked={r.celebrations === level}
                          onchange={() => set((c) => (c.rewards.celebrations = level))}
                        />
                        <span>{s.settings.celebrationLevel[level]}</span>
                      </label>
                    {/each}
                  </fieldset>
                  <p class="hint" id="{uid}-celebrations-hint">{s.settings.celebrationsHint}</p>
                </div>

                <Switch
                  id="beta-sounds"
                  label={s.settings.sounds}
                  checked={r.sounds}
                  onchange={(on) => set((c) => (c.rewards.sounds = on))}
                />
                <Switch
                  id="beta-companion"
                  label={s.settings.companion}
                  hint={s.settings.companionHint}
                  checked={r.focusCompanion}
                  onchange={(on) => set((c) => (c.rewards.focusCompanion = on))}
                />

                <p class="hint">{s.settings.storage}</p>
                <div>
                  <button type="button" class="btn btn-sm" onclick={() => (resetOpen = true)}>{s.settings.reset}</button>
                </div>
              </div>
            {/if}
          </Switch>
        </div>

        <div class="item">
          <Switch
            id="beta-steps"
            label={s.settings.steps}
            hint={s.settings.stepsHint}
            checked={r.microSteps}
            onchange={(on) => set((c) => (c.rewards.microSteps = on))}
          />
        </div>
      </div>
    {/if}
  </Switch>

  <ConfirmDialog
    bind:open={resetOpen}
    title={s.settings.resetTitle}
    body={s.settings.resetBody}
    confirmLabel={s.settings.resetConfirm}
    danger
    onconfirm={() => rewards.reset()}
  />
{/if}

<style>
  .items {
    display: flex;
    flex-direction: column;
    margin-top: 4px;
  }

  .item {
    padding: 14px 0;
    border-top: 1px solid var(--border);
  }

  .item:last-child {
    padding-bottom: 0;
  }

  /* The options of "Iskierki, poziomy i seria", indented under their switch. */
  .options {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-left: 16px;
    border-left: 2px solid var(--border);
  }

  .days {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  .days legend {
    padding: 0;
    margin-bottom: 6px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .day {
    position: relative;
    display: inline-flex;
  }

  .day input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .day span {
    display: grid;
    place-items: center;
    min-width: 44px;
    min-height: 32px;
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    border: var(--border-w) solid var(--border-strong);
    background: var(--surface-inset);
    color: var(--text-2);
    font-size: 0.86rem;
    font-weight: 600;
  }

  .day.on span {
    border-color: var(--bubble);
    background: var(--bubble-soft);
    color: var(--text);
  }

  .day input:focus-visible + span {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .limit {
    color: var(--text-2);
    font-weight: 600;
  }
</style>
