<script lang="ts">
  // First-run wizard, step 3 (ADR-023): the shields, with the same switches, texts and components as Settings →
  // Shields: Windows notifications, Teams (its account is connected later, in Settings), site blocking with the
  // extension's install guide (open the folder + three steps), keeping the screen awake, closing game launchers.
  import ExtensionGuide from "$lib/components/ExtensionGuide.svelte";
  import LauncherSettings from "$lib/components/LauncherSettings.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { config } from "$lib/state/config.svelte";
  import { strings } from "$lib/strings";
  import type { Config } from "$lib/types";

  const cfg = $derived(config.value);

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }
</script>

{#if cfg}
  <div class="card items">
    <div class="item">
      <Switch
        id="wizard-mute-notifications"
        label={strings.settings.notifications}
        hint={strings.settings.notificationsHint}
        checked={cfg.shields.muteNotifications}
        onchange={(on) => set((c) => (c.shields.muteNotifications = on))}
      />
    </div>

    <div class="item">
      <Switch
        id="wizard-teams-dnd"
        label={strings.settings.teams}
        hint={strings.settings.teamsHint}
        checked={cfg.shields.teamsDnd}
        onchange={(on) => set((c) => (c.shields.teamsDnd = on))}
      >
        {#if cfg.shields.teamsDnd}
          <p class="later">{strings.onboarding.shields.teamsLater}</p>
        {/if}
      </Switch>
    </div>

    <div class="item">
      <Switch
        id="wizard-block-sites"
        label={strings.settings.sites}
        hint={strings.settings.sitesHint}
        checked={cfg.shields.blockSites}
        onchange={(on) => set((c) => (c.shields.blockSites = on))}
      >
        {#if cfg.shields.blockSites}
          <ExtensionGuide />
        {/if}
      </Switch>
    </div>

    <div class="item">
      <Switch
        id="wizard-keep-awake"
        label={strings.settings.keepAwake}
        hint={strings.settings.keepAwakeHint}
        checked={cfg.shields.keepAwake}
        onchange={(on) => set((c) => (c.shields.keepAwake = on))}
      />
    </div>

    <div class="item">
      <LauncherSettings />
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

  .later {
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
    color: var(--text-2);
    font-size: 0.9rem;
  }
</style>
