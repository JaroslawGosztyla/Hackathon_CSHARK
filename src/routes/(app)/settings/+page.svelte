<script lang="ts">
  import { onMount } from "svelte";
  import { getAutostart, getSystemLanguage, setAutostart, toAppError, ERR_NO_SHELL } from "$lib/api";
  import BlocklistFile from "$lib/components/BlocklistFile.svelte";
  import BreakReminders from "$lib/components/BreakReminders.svelte";
  import CalendarSettings from "$lib/components/CalendarSettings.svelte";
  import ChimeSettings from "$lib/components/ChimeSettings.svelte";
  import ExtensionGuide from "$lib/components/ExtensionGuide.svelte";
  import HistorySettings from "$lib/components/HistorySettings.svelte";
  import LauncherSettings from "$lib/components/LauncherSettings.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import TeamsSettings from "$lib/components/TeamsSettings.svelte";
  import ThemeSettings from "$lib/components/ThemeSettings.svelte";
  import TintSettings from "$lib/components/TintSettings.svelte";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { LANGUAGES, strings } from "$lib/strings";
  import type { Config, Language } from "$lib/types";

  /** `closeToTray` as a select value: null = ask at the first X. */
  type CloseChoice = "ask" | "tray" | "quit";
  const CLOSE_CHOICES: readonly CloseChoice[] = ["ask", "tray", "quit"];
  const closeLabels: Record<CloseChoice, string> = {
    ask: strings.settings.closeAsk,
    tray: strings.settings.closeTray,
    quit: strings.settings.closeQuit,
  };

  const cfg = $derived(config.value);

  let autostart = $state<boolean | null>(null);
  let autostartBusy = $state(false);
  /** OS UI language as resolved by the shell, for the "Systemowy (…)" option. */
  let systemLanguage = $state<Language | null>(null);

  const closeChoice = $derived.by((): CloseChoice => {
    const v = cfg?.closeToTray;
    return v === true ? "tray" : v === false ? "quit" : "ask";
  });
  const systemLanguageName = $derived(
    systemLanguage && systemLanguage !== "system" ? strings.languageNames[systemLanguage] : null,
  );

  onMount(() => {
    getAutostart()
      .then((enabled) => (autostart = enabled))
      .catch((e: unknown) => {
        const err = toAppError(e);
        if (err.code !== ERR_NO_SHELL) toasts.error(err);
      });
    getSystemLanguage()
      .then((lang) => (systemLanguage = lang))
      .catch((e: unknown) => {
        // Only the label of the "system" option depends on it.
        const err = toAppError(e);
        if (err.code !== ERR_NO_SHELL) console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      });
  });

  function setCloseChoice(value: string) {
    if (!CLOSE_CHOICES.includes(value as CloseChoice)) return;
    const choice = value as CloseChoice;
    set((c) => (c.closeToTray = choice === "ask" ? null : choice === "tray"));
  }

  function setLanguage(value: string) {
    const lang = value === "system" ? "system" : LANGUAGES.find((l) => l === value);
    if (lang) set((c) => (c.language = lang));
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
            <option value={lang} {lang}>{strings.languageNames[lang]}</option>
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

  /* Linked from the Skupienie page (#calendar-settings): keep the heading clear of the sticky header. */
  .page :global(h2[id]) {
    scroll-margin-top: 110px;
  }
</style>
