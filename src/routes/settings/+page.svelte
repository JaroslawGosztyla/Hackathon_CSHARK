<script lang="ts">
  import { onMount } from "svelte";
  import { getAutostart, setAutostart, testChime, toAppError, ERR_NO_SHELL } from "$lib/api";
  import ExtensionGuide from "$lib/components/ExtensionGuide.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import TeamsSettings from "$lib/components/TeamsSettings.svelte";
  import { splitLines } from "$lib/player/sources";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";
  import type { ChimeKind, Config } from "$lib/types";

  const cfg = $derived(config.value);

  // Blocklist textarea: local text while focused; the shell normalizes domains on save.
  let blocklistText = $state("");
  let blocklistFocused = $state(false);
  $effect(() => {
    const list = config.value?.shields.blocklist;
    if (list && !blocklistFocused) blocklistText = list.join("\n");
  });
  const blocklistCount = $derived(splitLines(blocklistText).length);
  const blocklistError = $derived(config.fieldErrors["shields.blocklist"] ?? null);
  const chimeError = $derived(config.fieldErrors.chimeVolume ?? null);

  let autostart = $state<boolean | null>(null);
  let autostartBusy = $state(false);
  let chimeBusy = $state(false);

  onMount(() => {
    getAutostart()
      .then((enabled) => (autostart = enabled))
      .catch((e: unknown) => {
        const err = toAppError(e);
        if (err.code !== ERR_NO_SHELL) toasts.error(err);
      });
  });

  function set(mutate: (c: Config) => void) {
    config.update(mutate, true);
  }

  function onBlocklistInput(event: Event & { currentTarget: HTMLTextAreaElement }) {
    blocklistText = event.currentTarget.value;
    const domains = splitLines(blocklistText);
    config.update((c) => {
      c.shields.blocklist = domains;
    });
  }

  async function onBlocklistBlur() {
    blocklistFocused = false;
    // Save now so the normalized list (lowercase, no www./paths, deduped) comes back right away.
    await config.flush();
    // The user may have clicked back into the textarea while the save was running.
    if (!blocklistFocused) blocklistText = (config.value?.shields.blocklist ?? []).join("\n");
  }

  function onRange(event: Event & { currentTarget: HTMLInputElement }, apply: (c: Config, v: number) => void) {
    const pct = event.currentTarget.valueAsNumber;
    if (!Number.isFinite(pct)) return;
    config.update((c) => apply(c, Math.min(1, Math.max(0, pct / 100))));
  }

  async function playChime(kind: ChimeKind) {
    if (chimeBusy) return;
    chimeBusy = true;
    try {
      // The shell plays at the stored chime volume: save the slider first.
      await config.flush();
      await testChime(kind);
    } catch (e) {
      toasts.error(e);
    } finally {
      chimeBusy = false;
    }
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
                <div class="field">
                  <label for="blocklist">{strings.settings.blocklist}</label>
                  <textarea
                    id="blocklist"
                    class="textarea"
                    rows="7"
                    spellcheck="false"
                    value={blocklistText}
                    onfocus={() => (blocklistFocused = true)}
                    onblur={onBlocklistBlur}
                    oninput={onBlocklistInput}
                    aria-invalid={blocklistError ? "true" : undefined}
                    aria-describedby="blocklist-count"
                  ></textarea>
                  <p class="hint" id="blocklist-count">{strings.settings.blocklistCount(blocklistCount)}</p>
                  {#if blocklistError}<p class="field-error">{blocklistError}</p>{/if}
                </div>
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

    <section class="card" aria-labelledby="tracking-settings">
      <div class="card-title">
        <h2 id="tracking-settings">{strings.settings.trackingTitle}</h2>
      </div>
      <Switch
        id="track-apps"
        label={strings.settings.trackApps}
        hint={strings.settings.trackAppsHint}
        checked={cfg.trackApps}
        onchange={(on) => set((c) => (c.trackApps = on))}
      />
    </section>

    <section class="card" aria-labelledby="sound-settings">
      <div class="card-title">
        <h2 id="sound-settings">{strings.settings.soundTitle}</h2>
      </div>
      <div class="items">
        <div class="item">
          <div class="field">
            <label for="chime-volume">{strings.settings.chimeVolume}</label>
            <div class="range-row">
              <input
                id="chime-volume"
                type="range"
                min="0"
                max="100"
                step="1"
                value={Math.round(cfg.chimeVolume * 100)}
                aria-valuetext={`${Math.round(cfg.chimeVolume * 100)}%`}
                oninput={(e) => onRange(e, (c, v) => (c.chimeVolume = v))}
              />
              <span class="tabular value">{Math.round(cfg.chimeVolume * 100)}%</span>
            </div>
            {#if chimeError}<p class="field-error">{chimeError}</p>{/if}
          </div>
          <div class="row test-row">
            <span class="secondary small">{strings.settings.testChime}</span>
            <button type="button" class="btn btn-sm" disabled={chimeBusy} onclick={() => playChime("breakStart")}>
              {strings.settings.testBreak}
            </button>
            <button type="button" class="btn btn-sm" disabled={chimeBusy} onclick={() => playChime("focusStart")}>
              {strings.settings.testFocus}
            </button>
          </div>
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

  .test-row {
    margin-top: 10px;
  }
</style>
