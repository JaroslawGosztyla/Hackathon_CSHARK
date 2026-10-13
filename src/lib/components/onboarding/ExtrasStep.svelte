<script lang="ts">
  // First-run wizard, step 4 (ADR-023): optional extras with the same settings as Settings: FocusMode pop-ups, the X
  // button (tray or quit, the same select), the warm screen colours (off by default; only the switch here, the hours
  // and strength stay in Settings), links to the calendar and work tool cards in Settings ("later"), and the beta
  // features switch. The links close the wizard (`FirstRunWizard` handles every in-app link inside it).
  import CloseButtonSetting from "$lib/components/CloseButtonSetting.svelte";
  import BetaBadge from "$lib/components/rewards/BetaBadge.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { config } from "$lib/state/config.svelte";
  import { strings } from "$lib/strings";
  import type { Config } from "$lib/types";

  const uid = $props.id();
  const cfg = $derived(config.value);
  const s = strings.onboarding.extras;

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }
</script>

{#if cfg}
  <div class="card items">
    <div class="item">
      <Switch
        id="wizard-phase-popups"
        label={strings.settings.phasePopups}
        hint={strings.settings.phasePopupsHint}
        checked={cfg.phasePopups}
        onchange={(on) => set((c) => (c.phasePopups = on))}
      />
    </div>

    <div class="item">
      <CloseButtonSetting id="wizard-close-button" />
    </div>

    <div class="item">
      <Switch
        id="wizard-tint"
        label={strings.settings.tint}
        hint={s.tintHint}
        checked={cfg.tint.enabled}
        onchange={(on) => set((c) => (c.tint.enabled = on))}
      />
    </div>

    <div class="item">
      <h3 id="{uid}-connect">{s.connectTitle}</h3>
      <p class="hint" id="{uid}-connect-hint">{s.connectText}</p>
      <div class="links" role="group" aria-labelledby="{uid}-connect" aria-describedby="{uid}-connect-hint">
        <a class="btn btn-sm" href="/settings#calendar-settings">{s.calendarLink}</a>
        <a class="btn btn-sm" href="/settings#work-settings">{s.workLink}</a>
      </div>
    </div>

    <div class="item beta">
      <BetaBadge />
      <Switch
        id="wizard-beta"
        label={strings.rewards.settings.master}
        hint={strings.rewards.settings.intro}
        checked={cfg.betaFeatures === true}
        onchange={(on) => set((c) => (c.betaFeatures = on))}
      />
    </div>
  </div>
{/if}

<style>
  .items {
    display: flex;
    flex-direction: column;
  }

  .item {
    padding: 14px 0;
    border-top: 1px solid var(--border);
  }

  .item:first-child {
    padding-top: 0;
    border-top: 0;
  }

  .item:last-child {
    padding-bottom: 0;
  }

  h3 {
    font-size: 1rem;
    font-weight: 600;
    letter-spacing: 0;
  }

  .hint {
    margin-top: 2px;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 10px;
  }

  .links .btn {
    text-decoration: none;
  }

  /* The beta pill above its switch, like the card title in Settings. */
  .beta {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .beta > :global(.switch-row) {
    align-self: stretch;
  }
</style>
