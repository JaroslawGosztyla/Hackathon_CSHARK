<script lang="ts">
  // Close game launchers (Steam, Epic, GOG, …) when focus starts: on/off, which ones, and the last result.
  import Switch from "$lib/components/Switch.svelte";
  import { config } from "$lib/state/config.svelte";
  import { shields } from "$lib/state/shields.svelte";
  import { GAME_LAUNCHERS, launcherList, strings } from "$lib/strings";
  import type { Config, GameLauncher } from "$lib/types";

  const s = strings.settings;
  const cfg = $derived(config.value);
  const selected = $derived(new Set<GameLauncher>(cfg?.shields.launchers ?? []));

  /** What happened at the last focus start, from `ShieldStatus`. */
  const lastResult = $derived.by((): { text: string; error: boolean; detail: string | null } | null => {
    const st = shields.status;
    if (!st) return null;
    const detail = st.launchers.detail ?? null;
    switch (st.launchers.state) {
      case "unavailable":
        return { text: s.launchersUnavailable, error: false, detail };
      case "error":
        return { text: s.launchersError, error: true, detail };
      default:
        if (st.closedLaunchers.length > 0) {
          return { text: s.launchersLastClosed(launcherList(st.closedLaunchers)), error: false, detail };
        }
        return st.launchers.state === "on" ? { text: s.launchersLastNone, error: false, detail } : null;
    }
  });

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }

  function toggle(launcher: GameLauncher, on: boolean): void {
    set((c) => {
      const next = new Set(c.shields.launchers);
      if (on) next.add(launcher);
      else next.delete(launcher);
      c.shields.launchers = GAME_LAUNCHERS.filter((l) => next.has(l));
    });
  }
</script>

{#if cfg}
  <Switch
    id="close-launchers"
    label={s.launchers}
    hint={s.launchersHint}
    checked={cfg.shields.closeLaunchers}
    onchange={(on) => set((c) => (c.shields.closeLaunchers = on))}
  >
    {#if cfg.shields.closeLaunchers}
      <fieldset class="launchers">
        <legend class="field-label">{s.launchersPick}</legend>
        <div class="grid">
          {#each GAME_LAUNCHERS as launcher (launcher)}
            <label class="check" class:on={selected.has(launcher)}>
              <input
                type="checkbox"
                checked={selected.has(launcher)}
                onchange={(e) => toggle(launcher, e.currentTarget.checked)}
              />
              <span>{strings.launchers[launcher]}</span>
            </label>
          {/each}
        </div>
      </fieldset>
      {#if selected.size === 0}
        <p class="field-error">{s.launchersNone}</p>
      {/if}
      {#if lastResult}
        <p class="result" class:error={lastResult.error} title={lastResult.detail ?? undefined}>{lastResult.text}</p>
      {/if}
    {/if}
  </Switch>
{/if}

<style>
  .launchers {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  .launchers legend {
    padding: 0;
    margin-bottom: 8px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 8px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-inset);
    color: var(--text-2);
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      color 0.15s ease;
  }

  .check:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .check.on {
    border-color: rgba(242, 166, 90, 0.45);
    color: var(--text);
  }

  .check input {
    width: 16px;
    height: 16px;
    margin: 0;
    flex: none;
    cursor: pointer;
  }

  .field-error,
  .result {
    margin-top: 10px;
  }

  .result {
    color: var(--text-2);
    font-size: 0.88rem;
  }

  .result.error {
    color: var(--danger);
  }
</style>
