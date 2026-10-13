<script lang="ts">
  import { onMount } from "svelte";
  import { getAutostart, setAutostart, toAppError, ERR_NO_SHELL } from "$lib/api";
  import BlocklistFile from "$lib/components/BlocklistFile.svelte";
  import BreakReminders from "$lib/components/BreakReminders.svelte";
  import CalendarSettings from "$lib/components/CalendarSettings.svelte";
  import ChimeSettings from "$lib/components/ChimeSettings.svelte";
  import ExtensionGuide from "$lib/components/ExtensionGuide.svelte";
  import HistorySettings from "$lib/components/HistorySettings.svelte";
  import LauncherSettings from "$lib/components/LauncherSettings.svelte";
  import PrivacySettings from "$lib/components/PrivacySettings.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import TeamsSettings from "$lib/components/TeamsSettings.svelte";
  import ThemeSettings from "$lib/components/ThemeSettings.svelte";
  import TintSettings from "$lib/components/TintSettings.svelte";
  import WorkSettings from "$lib/components/WorkSettings.svelte";
  import YoutubeAccountSettings from "$lib/components/YoutubeAccountSettings.svelte";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import {
    isUiLanguage,
    knownSystemLanguage,
    LANGUAGE_NAMES,
    LANGUAGES,
    loadSystemLanguage,
    strings,
  } from "$lib/i18n/index.svelte";
  import type { Config } from "$lib/types";

  /** `closeToTray` as a select value: null = ask at the first X. */
  type CloseChoice = "ask" | "tray" | "quit";
  const CLOSE_CHOICES: readonly CloseChoice[] = ["ask", "tray", "quit"];
  const closeLabels = $derived<Record<CloseChoice, string>>({
    ask: strings.settings.closeAsk,
    tray: strings.settings.closeTray,
    quit: strings.settings.closeQuit,
  });

  const cfg = $derived(config.value);

  let autostart = $state<boolean | null>(null);
  let autostartBusy = $state(false);

  const closeChoice = $derived.by((): CloseChoice => {
    const v = cfg?.closeToTray;
    return v === true ? "tray" : v === false ? "quit" : "ask";
  });
  /** OS UI language as resolved by the shell, for the "System (…)" option (native name). */
  const systemLanguageName = $derived.by(() => {
    const lang = knownSystemLanguage();
    return lang ? LANGUAGE_NAMES[lang] : null;
  });

  /** The OS autostart entry (also after "Delete everything" in the privacy panel removed it). */
  function loadAutostart() {
    getAutostart()
      .then((enabled) => (autostart = enabled))
      .catch((e: unknown) => {
        const err = toAppError(e);
        if (err.code !== ERR_NO_SHELL) toasts.error(err);
      });
  }

  onMount(() => {
    loadAutostart();
    // Only the label of the "system" option depends on it (failures are logged by `$lib/i18n`).
    void loadSystemLanguage();
  });

  function setCloseChoice(value: string) {
    if (!CLOSE_CHOICES.includes(value as CloseChoice)) return;
    const choice = value as CloseChoice;
    set((c) => (c.closeToTray = choice === "ask" ? null : choice === "tray"));
  }

  /** Saves `config.language`; the app layout follows it, so the whole UI switches at once. */
  function setLanguage(value: string) {
    if (value === "system" || isUiLanguage(value)) set((c) => (c.language = value));
  }

  function set(mutate: (c: Config) => void) {
    config.update(mutate, true);
  }

  function onRange(event: Event & { currentTarget: HTMLInputElement }, apply: (c: Config, v: number) => void) {
    const pct = event.currentTarget.valueAsNumber;
    if (!Number.isFinite(pct)) return;
    config.update((c) => apply(c, Math.min(1, Math.max(0, pct / 100))));
  }

  async function setLaunchOnLogin(enabled: boolean) {
    if (autostartBusy) return;
    autostartBusy = true;
    try {
      await setAutostart(enabled);
      autostart = enabled;
      set((c) => {
        c.launchOnLogin = enabled;
      });
    } catch (e) {
      toasts.error(e);
    } finally {
      autostartBusy = false;
    }
  }
</script>

<svelte:head>
  <title>{strings.settings.title} · {strings.appName}</title>
</svelte:head>

<div class="page">
  <div class="heading">
    <h1>{strings.settings.title}</h1>
    <p class="save-state small muted" aria-live="polite">
      {#if config.saving}
        {strings.settings.saving}
      {:else if config.savedCount > 0}
        {strings.settings.saved}
      {/if}
    </p>
  </div>

  {#if session.snapshot.demo}
    <p class="badge demo">{strings.settings.demoNote}</p>
  {/if}

  {#if !cfg}
    <p class="hint">{strings.player.loading}</p>
  {:else}
    <section class="card" aria-labelledby="appearance-settings">
      <div class="card-title">
        <h2 id="appearance-settings">{strings.settings.appearanceTitle}</h2>
      </div>
      <ThemeSettings />
      <div class="appearance-extra">
        <Switch
          id="show-mascot"
          label={strings.mascot.setting}
          hint={strings.mascot.settingHint}
          checked={mascotEnabled(cfg)}
          onchange={(on) => set((c) => (c.showMascot = on))}
        />
      </div>
    </section>

    <section class="card" aria-labelledby="shields-settings">
      <div class="card-title">
        <div>
          <h2 id="shields-settings">{strings.settings.shieldsTitle}</h2>
          <p class="hint">{strings.settings.shieldsLead}</p>
        </div>
      </div>

      <div class="items">
        <div class="item">
          <Switch
            id="mute-notifications"
            label={strings.settings.notifications}
            hint={strings.settings.notificationsHint}
            checked={cfg.shields.muteNotifications}
            onchange={(on) => set((c) => (c.shields.muteNotifications = on))}
          />
        </div>

        <div class="item">
          <Switch
            id="teams-dnd"
            label={strings.settings.teams}
            hint={strings.settings.teamsHint}
            checked={cfg.shields.teamsDnd}
            onchange={(on) => set((c) => (c.shields.teamsDnd = on))}
          >
            {#if cfg.shields.teamsDnd}
              <TeamsSettings />
            {/if}
          </Switch>
        </div>

        <div class="item">
          <Switch
            id="block-sites"
            label={strings.settings.sites}
            hint={strings.settings.sitesHint}
            checked={cfg.shields.blockSites}
            onchange={(on) => set((c) => (c.shields.blockSites = on))}
          >
            {#if cfg.shields.blockSites}
              <div class="stack">
                <BlocklistFile />
                <ExtensionGuide />
              </div>
            {/if}
          </Switch>
        </div>

        <div class="item">
          <Switch
            id="keep-awake"
            label={strings.settings.keepAwake}
            hint={strings.settings.keepAwakeHint}
            checked={cfg.shields.keepAwake}
            onchange={(on) => set((c) => (c.shields.keepAwake = on))}
          />
        </div>
      </div>
    </section>

    <section class="card" aria-labelledby="calendar-settings">
      <div class="card-title">
        <h2 id="calendar-settings">{strings.calendar.title}</h2>
      </div>
      <CalendarSettings />
    </section>

    <section class="card" aria-labelledby="work-settings">
      <div class="card-title">
        <h2 id="work-settings">{strings.work.title}</h2>
      </div>
      <WorkSettings />
    </section>

    <section class="card" aria-labelledby="reminders-settings">
      <div class="card-title">
        <h2 id="reminders-settings">{strings.settings.remindersTitle}</h2>
      </div>
      <BreakReminders />
    </section>

    <section class="card" aria-labelledby="sound-settings">
      <div class="card-title">
        <h2 id="sound-settings">{strings.settings.soundTitle}</h2>
      </div>
      <div class="items">
        <div class="item">
          <ChimeSettings />
        </div>

        <div class="item">
          <div class="field">
            <label for="music-volume">{strings.settings.musicVolume}</label>
            <div class="range-row">
              <input
                id="music-volume"
                type="range"
                min="0"
                max="100"
                step="1"
                value={Math.round(cfg.music.volume * 100)}
                aria-valuetext={`${Math.round(cfg.music.volume * 100)}%`}
                oninput={(e) => onRange(e, (c, v) => (c.music.volume = v))}
              />
              <span class="tabular value">{Math.round(cfg.music.volume * 100)}%</span>
            </div>
          </div>
        </div>

        <div class="item">
          <Switch
            id="pause-during-break"
            label={strings.music.pauseDuringBreak}
            hint={strings.music.pauseDuringBreakHint}
            checked={cfg.music.pauseDuringBreak}
            onchange={(on) => set((c) => (c.music.pauseDuringBreak = on))}
          />
        </div>

        <div class="item">
          <YoutubeAccountSettings />
        </div>
      </div>
    </section>

    <section class="card" aria-labelledby="launchers-settings">
      <div class="card-title">
        <h2 id="launchers-settings">{strings.settings.launchersTitle}</h2>
      </div>
      <LauncherSettings />
    </section>

    <section class="card" aria-labelledby="window-settings">
      <div class="card-title">
        <h2 id="window-settings">{strings.settings.windowTitle}</h2>
      </div>
      <div class="items">
        <div class="item">
          <div class="field">
            <label for="close-button">{strings.settings.closeButton}</label>
            <select
              id="close-button"
              class="input select narrow"
              value={closeChoice}
              aria-describedby="close-button-hint"
              onchange={(e) => setCloseChoice(e.currentTarget.value)}
            >
              {#each CLOSE_CHOICES as choice (choice)}
                <option value={choice}>{closeLabels[choice]}</option>
              {/each}
            </select>
            <p class="hint" id="close-button-hint">{strings.settings.closeHint}</p>
          </div>
        </div>

        <div class="item">
          <Switch
            id="phase-popups"
            label={strings.settings.phasePopups}
            hint={strings.settings.phasePopupsHint}
            checked={cfg.phasePopups}
            onchange={(on) => set((c) => (c.phasePopups = on))}
          />
        </div>
      </div>
    </section>

    <section class="card" aria-labelledby="tint-settings">
      <div class="card-title">
        <h2 id="tint-settings">{strings.settings.tintTitle}</h2>
      </div>
      <TintSettings />
    </section>

    <section class="card" aria-labelledby="history-settings">
      <div class="card-title">
        <h2 id="history-settings">{strings.settings.historyTitle}</h2>
      </div>
      <HistorySettings />
    </section>

    <section class="card" aria-labelledby="language-settings">
      <div class="card-title">
        <h2 id="language-settings">{strings.settings.languageTitle}</h2>
      </div>
      <div class="field">
        <label for="language">{strings.settings.language}</label>
        <select
          id="language"
          class="input select narrow"
          value={cfg.language}
          onchange={(e) => setLanguage(e.currentTarget.value)}
        >
          <option value="system">{strings.settings.languageSystem(systemLanguageName)}</option>
          {#each LANGUAGES as lang (lang)}
            <option value={lang} lang={lang === "zh" ? "zh-Hans" : lang}>{LANGUAGE_NAMES[lang]}</option>
          {/each}
        </select>
      </div>
    </section>

    <section class="card" aria-labelledby="system-settings">
      <div class="card-title">
        <h2 id="system-settings">{strings.settings.systemTitle}</h2>
      </div>
      <Switch
        id="launch-on-login"
        label={strings.settings.launchOnLogin}
        hint={strings.settings.launchOnLoginHint}
        checked={autostart ?? cfg.launchOnLogin}
        disabled={autostartBusy}
        onchange={(on) => void setLaunchOnLogin(on)}
      />
    </section>

    <section class="card" aria-labelledby="privacy-settings">
      <div class="card-title">
        <h2 id="privacy-settings">{strings.privacy.title}</h2>
      </div>
      <PrivacySettings onreset={loadAutostart} />
    </section>

    <!-- Last on purpose: a tool for tests and screenshots (ADR-019); its page itself is English only. -->
    <section class="card" aria-labelledby="developer-settings">
      <div class="card-title">
        <h2 id="developer-settings">{strings.settings.developerTitle}</h2>
      </div>
      <Switch
        id="developer-mode"
        label={strings.settings.developerMode}
        hint={strings.settings.developerModeHint}
        checked={cfg.developerMode === true}
        onchange={(on) => set((c) => (c.developerMode = on))}
      />
    </section>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }

  .demo {
    align-self: flex-start;
  }

  .items {
    display: flex;
    flex-direction: column;
  }

  .item {
    padding: 16px 0;
    border-top: 1px solid var(--border);
  }

  .item:first-child {
    padding-top: 0;
    border-top: 0;
  }

  .item:last-child {
    padding-bottom: 0;
  }

  .appearance-extra {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .range-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .range-row input {
    flex: 1;
    max-width: 320px;
  }

  .value {
    min-width: 3.5em;
    color: var(--text-2);
  }

  .narrow {
    max-width: 340px;
  }

  /* Linked from the Skupienie page (#calendar-settings, #work-settings): keep the heading clear of the sticky header. */
  .page :global(h2[id]) {
    scroll-margin-top: 110px;
  }
</style>
