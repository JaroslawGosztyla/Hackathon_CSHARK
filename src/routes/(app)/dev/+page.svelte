<script lang="ts">
  // Developer page (ADR-019): only in developer mode (the layout leaves it when the mode is switched off). Buttons that
  // trigger popups, toasts, mascot reactions, banners and samples for screenshots. English only (`$lib/i18n/dev`);
  // every button does one thing and reports in the status line. Samples stay in memory (`$lib/state/dev.svelte`).
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { formatClock } from "$lib/format";
  import {
    DEV,
    DEV_PRESENTATION,
    MASCOT_MOODS,
    MASCOT_REACTIONS,
    POPUP_SAMPLES,
    POSE_GALLERY,
    REMAINING_SAMPLES,
    RING_SAMPLE_SECS,
    sampleTexts,
    speechSample,
    TINT_PREVIEW_LEVELS,
    TOAST_SAMPLES,
  } from "$lib/i18n/dev";
  import { LANGUAGE_NAMES, LANGUAGES, strings } from "$lib/i18n/index.svelte";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import PoseGallery from "$lib/mascot/PoseGallery.svelte";
  import DevRewards from "$lib/components/rewards/DevRewards.svelte";
  import { config, reminderForBreak } from "$lib/state/config.svelte";
  import { dev, SEQUENCE_STEP_MS } from "$lib/state/dev.svelte";
  import { presentation } from "$lib/state/presentation.svelte";
  import { session } from "$lib/state/session.svelte";
  import { showcaseMode } from "$lib/state/showcaseMode.svelte";
  import { REWARD_THEMES, THEMES } from "$lib/themes";
  import { breakTipText, pickBreakTip, WORKING_TIPS, workingTipText } from "$lib/tips";
  import type { ChimeKind, ChimeSound, Language } from "$lib/types";

  const SECOND_MS = 1000;
  const MINUTE_SECS = 60;
  /** Working tips in the "Long scrolling text" sample. */
  const LONG_TEXT_TIPS = 4;
  /** The break the "Break reminder + tip" sample talks about. */
  const SAMPLE_BREAK_NUMBER = 2;
  const CHIME_SOUNDS: readonly ChimeSound[] = ["midnight", "vaporwave", "lunar", "autumn", "classic", "soft", "chiptune"];
  const CHIME_KINDS: readonly { kind: ChimeKind; label: string }[] = [
    { kind: "breakStart", label: DEV.chimeBreak },
    { kind: "focusStart", label: DEV.chimeFocus },
  ];
  const LANGUAGE_CHOICES: readonly { value: Language; label: string }[] = [
    { value: "system", label: DEV.systemLanguage },
    ...LANGUAGES.map((lang) => ({ value: lang, label: LANGUAGE_NAMES[lang] })),
  ];

  const cfg = $derived(config.value);
  const snap = $derived(session.snapshot);
  const mascotOn = $derived(mascotEnabled(cfg));
  const running = $derived(!session.isIdle);
  /** Custom chime files are offered only when the user imported one. */
  const chimeSounds = $derived<readonly ChimeSound[]>(
    cfg && (cfg.customChimes.breakFile !== null || cfg.customChimes.focusFile !== null)
      ? [...CHIME_SOUNDS, "custom"]
      : CHIME_SOUNDS,
  );
  const sessionLine = $derived(
    session.isIdle
      ? DEV.idleState
      : DEV.sessionState(DEV.phaseNames[snap.phase], snap.cycle, formatClock(snap.remainingSecs), snap.paused),
  );

  function pick(lines: readonly string[]): string {
    return lines[Math.floor(Math.random() * lines.length)] ?? lines[0] ?? "";
  }

  function sayShort(): void {
    void dev.mascot({ type: "say", text: pick(strings.mascot.cycleLines), label: null }, DEV.speakShort);
  }

  function sayLong(): void {
    const text = WORKING_TIPS.slice(0, LONG_TEXT_TIPS).map(workingTipText).join("\n");
    void dev.mascot({ type: "say", text, label: strings.mascot.practiceLabel }, DEV.speakLong);
  }

  function sayBreak(): void {
    const reminder =
      (cfg ? reminderForBreak(cfg.breakReminders, cfg.repeatReminders, SAMPLE_BREAK_NUMBER) : null) ?? sampleTexts().reminder;
    const breakSecs = snap.breakSecs > 0 ? snap.breakSecs : (cfg?.breakMinutes ?? 0) * MINUTE_SECS;
    const tip = pickBreakTip(snap.sessionId, SAMPLE_BREAK_NUMBER, breakSecs);
    const text = [reminder, tip ? breakTipText(tip) : null].filter((part) => part !== null).join("\n");
    void dev.mascot(
      { type: "say", text, label: strings.breakView.reminderLabel(SAMPLE_BREAK_NUMBER) },
      DEV.speakBreak,
    );
  }

  function sayLanguage(lang: (typeof LANGUAGES)[number]): void {
    void dev.mascot({ type: "say", text: speechSample(lang), label: LANGUAGE_NAMES[lang] }, LANGUAGE_NAMES[lang]);
  }

  function showMascot(): void {
    config.update((c) => (c.showMascot = true), true);
  }

  /** The pose gallery (beta poses, atlas QA): mounted only on request, so the poses atlas loads only then. */
  let poseGalleryOpen = $state(false);

  /** The factory reset confirmation (lists what goes; the shell restarts the app when it is done). */
  let resetOpen = $state(false);
  const resetBody = $derived(
    [running ? DEV.factoryResetSessionStops : null, DEV.factoryResetBody, import.meta.env.DEV ? DEV.factoryResetDevNote : null]
      .filter((part) => part !== null)
      .join(" "),
  );

  // "Play all popups" belongs to this page: leaving it stops the sequence (its Stop button lives here).
  $effect(() => () => dev.stopSequence());
</script>

<svelte:head>
  <title>{DEV.title} · {strings.appName}</title>
</svelte:head>

<div class="page" lang="en">
  <div class="heading">
    <h1>{DEV.title}</h1>
    <span class="badge">ADR-019</span>
  </div>
  <p class="hint lead">{DEV.lead}</p>

  {#if !dev.enabled}
    <section class="card">
      <p>
        {DEV.off}
        <a href="/settings#developer-settings">{DEV.offLink}</a>
      </p>
    </section>
  {:else}
    <section class="card toolbar" aria-label={DEV.title}>
      <Switch
        id="dev-clean-screenshots"
        label={DEV.cleanScreenshots}
        hint={DEV.cleanScreenshotsHint}
        checked={dev.cleanScreenshots}
        onchange={(on) => (dev.cleanScreenshots = on)}
      />
      <p class="status small" class:error={dev.status?.tone === "error"} role="status" aria-live="polite">
        <span class="dot" aria-hidden="true"></span>
        {dev.status?.text ?? DEV.statusIdle}
      </p>
    </section>

    <div class="columns">
      <div class="column">
        <!-- Popups -->
        <section class="card group" aria-labelledby="dev-popups">
          <div class="card-title">
            <div>
              <h2 id="dev-popups">{DEV.popupsTitle}</h2>
              <p class="hint">{DEV.popupsHint}</p>
            </div>
          </div>
          <div class="buttons">
            {#each POPUP_SAMPLES as sample (sample.label)}
              <button type="button" class="btn btn-sm" onclick={() => dev.popup(sample)}>{sample.label}</button>
            {/each}
          </div>
          <div class="buttons split">
            {#if dev.sequence}
              <button type="button" class="btn btn-sm btn-danger" onclick={() => dev.stopSequence()}>
                {DEV.stop}
              </button>
              <span class="small secondary tabular" aria-live="polite">
                {DEV.playing(dev.sequence.step, dev.sequence.total)}
              </span>
            {:else}
              <button type="button" class="btn btn-sm btn-primary" onclick={() => dev.playAllPopups()}>
                {DEV.playAll(SEQUENCE_STEP_MS / SECOND_MS)}
              </button>
            {/if}
            <button type="button" class="btn btn-sm" onclick={() => dev.dismissPopup()}>{DEV.dismissPopup}</button>
          </div>
          <div class="sub-switch">
            <Switch
              id="dev-keep-popup"
              label={DEV.keepPopupOpen}
              hint={DEV.keepPopupOpenHint}
              checked={dev.keepPopupOpen}
              onchange={(on) => (dev.keepPopupOpen = on)}
            />
          </div>
        </section>

        <!-- Session -->
        <section class="card group" aria-labelledby="dev-session">
          <div class="card-title">
            <div>
              <h2 id="dev-session">{DEV.sessionTitle}</h2>
              <p class="hint">{DEV.sessionHint}</p>
            </div>
          </div>
          <!-- Presentation mode = demo timings + the showcase (`$lib/state/presentation.svelte`). -->
          <Switch
            id="dev-presentation"
            label={DEV_PRESENTATION.label}
            hint={presentation.timingDiffers
              ? DEV_PRESENTATION.pending
              : DEV_PRESENTATION.hint(dev.presentationLengths.focusSecs, dev.presentationLengths.breakSecs)}
            checked={showcaseMode.active}
            disabled={presentation.busy}
            onchange={(on) => presentation.set(on)}
          />
          {#if showcaseMode.launch}<p class="hint">{DEV_PRESENTATION.launched}</p>{/if}
          <p class="readout tabular" class:break={snap.phase === "break"} aria-live="polite">{sessionLine}</p>
          <div class="buttons">
            <button
              type="button"
              class="btn btn-sm btn-primary"
              disabled={running || session.busy || !config.ready}
              onclick={() => session.start()}
            >
              {DEV.start}
            </button>
            <button
              type="button"
              class="btn btn-sm"
              disabled={!running || snap.paused || session.busy}
              onclick={() => session.pause()}
            >
              {DEV.pause}
            </button>
            <button
              type="button"
              class="btn btn-sm"
              disabled={!running || !snap.paused || session.busy}
              onclick={() => session.resume()}
            >
              {DEV.resume}
            </button>
            <button type="button" class="btn btn-sm" disabled={!running || session.busy} onclick={() => session.skip()}>
              {DEV.skip}
            </button>
            <button type="button" class="btn btn-sm btn-danger" disabled={!running} onclick={() => session.stop()}>
              {DEV.stopSession}
            </button>
          </div>
          <div class="buttons">
            {#each REMAINING_SAMPLES as sample (sample.secs)}
              <button
                type="button"
                class="btn btn-sm"
                disabled={!running}
                title={running ? undefined : DEV.needsSession}
                onclick={() => dev.setRemaining(sample.secs, sample.label)}
              >
                {sample.label}
              </button>
            {/each}
            <button
              type="button"
              class="btn btn-sm"
              disabled={!running}
              title={running ? undefined : DEV.needsSession}
              onclick={() => dev.setRemaining(RING_SAMPLE_SECS, DEV.ringSample, "/")}
            >
              {DEV.ringSample}
            </button>
          </div>
        </section>

        <!-- Windows toasts -->
        <section class="card group" aria-labelledby="dev-toasts">
          <div class="card-title">
            <div>
              <h2 id="dev-toasts">{DEV.toastsTitle}</h2>
              <p class="hint">{DEV.toastsHint}</p>
            </div>
          </div>
          <div class="buttons">
            {#each TOAST_SAMPLES as sample (sample.phase)}
              <button type="button" class="btn btn-sm" onclick={() => dev.toast(sample.phase, sample.label)}>
                {sample.label}
              </button>
            {/each}
          </div>
        </section>

        <!-- Banners & dialogs -->
        <section class="card group" aria-labelledby="dev-banners">
          <div class="card-title">
            <div>
              <h2 id="dev-banners">{DEV.bannersTitle}</h2>
              <p class="hint">{DEV.bannersHint}</p>
            </div>
          </div>
          <div class="buttons">
            <button type="button" class="btn btn-sm" onclick={() => dev.showNudge()}>{DEV.nudgeBanner}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showErrorToast()}>{DEV.errorToast}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showInfoToast()}>{DEV.infoToast}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showClosePrompt()}>{DEV.closePrompt}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showHold("extend", DEV.holdExtend)}>
              {DEV.holdExtend}
            </button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showHold("pauseAtBreak", DEV.holdPause)}>
              {DEV.holdPause}
            </button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showGoal()}>{DEV.goalLine}</button>
          </div>
        </section>

        <!-- Sample data -->
        <section class="card group" aria-labelledby="dev-samples">
          <div class="card-title">
            <div>
              <h2 id="dev-samples">{DEV.samplesTitle}</h2>
              <p class="hint">{DEV.samplesHint}</p>
            </div>
          </div>
          <div class="buttons">
            <button type="button" class="btn btn-sm" onclick={() => dev.showSummarySample()}>{DEV.sampleSummary}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showWeekSample()}>{DEV.sampleWeek}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showWorkSample()}>{DEV.sampleWork}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showCalendarSample()}>{DEV.sampleCalendar}</button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showSmartBreakSample()}>
              {DEV.sampleSmartBreak}
            </button>
            <button type="button" class="btn btn-sm" onclick={() => dev.showTintPauseSample()}>
              {DEV.sampleTintPause}
            </button>
            <button
              type="button"
              class="btn btn-sm btn-danger"
              disabled={dev.activeSamples.length === 0}
              onclick={() => dev.clearSamples()}
            >
              {DEV.clearSamples}
            </button>
          </div>
          <p class="small secondary active">
            {#if dev.activeSamples.length === 0}
              {DEV.noSamples}
            {:else}
              {DEV.activeSamples}
              {#each dev.activeSamples as name (name)}<span class="chip">{name}</span>{/each}
            {/if}
          </p>
        </section>

        <!-- Window & app -->
        <section class="card group" aria-labelledby="dev-app">
          <div class="card-title">
            <div>
              <h2 id="dev-app">{DEV.appTitle}</h2>
              <p class="hint">{DEV.appHint}</p>
            </div>
          </div>
          <div class="buttons">
            <button type="button" class="btn btn-sm" disabled={!config.ready} onclick={() => dev.resetCloseChoice()}>
              {DEV.resetClose}
            </button>
            <button
              type="button"
              class="btn btn-sm"
              disabled={!config.ready}
              aria-describedby="dev-wizard-hint"
              onclick={() => dev.showFirstRunWizard()}
            >
              {DEV.showWizard}
            </button>
            <button type="button" class="btn btn-sm" onclick={() => dev.reloadUi()}>{DEV.reloadUi}</button>
          </div>
          <p class="hint" id="dev-wizard-hint">{DEV.showWizardHint}</p>
          <div class="danger-row">
            <p class="hint" id="dev-factory-reset-hint">{DEV.factoryResetHint}</p>
            <button
              type="button"
              class="btn btn-sm btn-danger"
              aria-describedby="dev-factory-reset-hint"
              onclick={() => (resetOpen = true)}
            >
              {DEV.factoryReset}
            </button>
          </div>
        </section>
      </div>

      <div class="column">
        <!-- Mascot -->
        <section class="card group" aria-labelledby="dev-mascot">
          <div class="card-title">
            <div>
              <h2 id="dev-mascot">{DEV.mascotTitle}</h2>
              <p class="hint">{DEV.mascotHint}</p>
            </div>
          </div>
          {#if !mascotOn && config.ready}
            <div class="notice">
              <span class="small">{DEV.mascotOff}</span>
              <button type="button" class="btn btn-sm btn-primary" onclick={showMascot}>{DEV.mascotOn}</button>
            </div>
          {/if}
          <fieldset class="set" disabled={!mascotOn}>
            <legend>{DEV.reactionsLabel}</legend>
            <div class="buttons">
              {#each MASCOT_REACTIONS as item (item.reaction)}
                <button
                  type="button"
                  class="btn btn-sm"
                  onclick={() => dev.mascot({ type: "react", reaction: item.reaction }, item.label)}
                >
                  {item.label}
                </button>
              {/each}
              <button type="button" class="btn btn-sm" onclick={() => dev.mascot({ type: "tickle" }, DEV.tickle)}>
                {DEV.tickle}
              </button>
            </div>
          </fieldset>
          <fieldset class="set" disabled={!mascotOn}>
            <legend>{DEV.moodsLabel}</legend>
            <div class="buttons">
              {#each MASCOT_MOODS as item (item.mood)}
                <button
                  type="button"
                  class="btn btn-sm"
                  class:btn-primary={dev.mascotMood === item.mood}
                  aria-pressed={dev.mascotMood === item.mood}
                  onclick={() => dev.holdMood(item.mood, item.label)}
                >
                  {item.label}
                </button>
              {/each}
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
          <fieldset class="set" disabled={!mascotOn}>
            <legend>{DEV.speakLabel}</legend>
            <div class="buttons">
              <button type="button" class="btn btn-sm" onclick={sayShort}>{DEV.speakShort}</button>
              <button type="button" class="btn btn-sm" onclick={sayLong}>{DEV.speakLong}</button>
              <button type="button" class="btn btn-sm" onclick={sayBreak}>{DEV.speakBreak}</button>
            </div>
          </fieldset>
          <fieldset class="set" disabled={!mascotOn}>
            <legend>{DEV.languagesLabel}</legend>
            <div class="buttons">
              {#each LANGUAGES as lang (lang)}
                <button
                  type="button"
                  class="btn btn-sm"
                  lang={lang === "zh" ? "zh-Hans" : lang}
                  onclick={() => sayLanguage(lang)}
                >
                  {LANGUAGE_NAMES[lang]}
                </button>
              {/each}
            </div>
          </fieldset>
          <fieldset class="set" disabled={!config.ready}>
            <legend>{DEV.shortcutLabel}</legend>
            <div class="buttons">
              <button type="button" class="btn btn-sm" onclick={() => dev.pixelLook()}>{DEV.pixelLook}</button>
            </div>
          </fieldset>
          <fieldset class="set" disabled={!mascotOn}>
            <legend>{POSE_GALLERY.title}</legend>
            <p class="hint">{POSE_GALLERY.hint}</p>
            <div class="buttons">
              <button
                type="button"
                class="btn btn-sm"
                class:btn-primary={poseGalleryOpen}
                aria-pressed={poseGalleryOpen}
                onclick={() => (poseGalleryOpen = !poseGalleryOpen)}
              >
                {poseGalleryOpen ? POSE_GALLERY.close : POSE_GALLERY.open}
              </button>
            </div>
            {#if poseGalleryOpen && mascotOn}
              <PoseGallery />
            {/if}
          </fieldset>
        </section>

        <!-- Look & feel -->
        <section class="card group" aria-labelledby="dev-look">
          <div class="card-title">
            <div>
              <h2 id="dev-look">{DEV.lookTitle}</h2>
              <p class="hint">{DEV.lookHint}</p>
            </div>
          </div>
          <fieldset class="set" disabled={!config.ready}>
            <legend>{DEV.themesLabel}</legend>
            <div class="buttons">
              {#each THEMES as theme (theme)}
                <button
                  type="button"
                  class="btn btn-sm"
                  class:btn-primary={cfg?.theme === theme}
                  aria-pressed={cfg?.theme === theme}
                  onclick={() => dev.setTheme(theme, DEV.themeNames[theme])}
                >
                  {DEV.themeNames[theme]}
                </button>
              {/each}
            </div>
          </fieldset>
          <fieldset class="set">
            <legend>{DEV.rewardThemesLabel}</legend>
            <p class="hint">{DEV.rewardThemesHint}</p>
            <div class="buttons">
              {#each REWARD_THEMES as theme (theme)}
                <button
                  type="button"
                  class="btn btn-sm"
                  class:btn-primary={dev.themePreview === theme}
                  aria-pressed={dev.themePreview === theme}
                  onclick={() => dev.previewRewardTheme(theme, DEV.rewardThemeNames[theme])}
                >
                  {DEV.rewardThemeNames[theme]}
                </button>
              {/each}
              <button
                type="button"
                class="btn btn-sm"
                disabled={dev.themePreview === null}
                onclick={() => dev.previewRewardTheme(null, DEV.rewardThemeStop)}
              >
                {DEV.rewardThemeStop}
              </button>
            </div>
          </fieldset>
          <fieldset class="set" disabled={!config.ready}>
            <legend>{DEV.languagesPick}</legend>
            <div class="buttons">
              {#each LANGUAGE_CHOICES as choice (choice.value)}
                <button
                  type="button"
                  class="btn btn-sm"
                  class:btn-primary={cfg?.language === choice.value}
                  aria-pressed={cfg?.language === choice.value}
                  lang={choice.value === "system" ? undefined : choice.value === "zh" ? "zh-Hans" : choice.value}
                  onclick={() => dev.setLanguage(choice.value, choice.label)}
                >
                  {choice.label}
                </button>
              {/each}
            </div>
          </fieldset>
          <fieldset class="set">
            <legend>{DEV.tintLabel}</legend>
            <div class="buttons">
              {#each TINT_PREVIEW_LEVELS as level (level)}
                <button
                  type="button"
                  class="btn btn-sm"
                  onclick={() => dev.previewTint(level, `${DEV.tintLabel} ${DEV.tintLevel(level)}`)}
                >
                  {DEV.tintLevel(level)}
                </button>
              {/each}
              <button type="button" class="btn btn-sm btn-ghost" onclick={() => dev.previewTint(null, DEV.tintStop)}>
                {DEV.tintStop}
              </button>
            </div>
          </fieldset>
          <fieldset class="set" disabled={!config.ready || dev.chimeBusy}>
            <legend>{DEV.chimesLabel}</legend>
            <p class="hint">{DEV.chimesHint}</p>
            <ul class="chimes">
              {#each chimeSounds as sound (sound)}
                <li>
                  <span class="chime-name">{DEV.chimeNames[sound]}</span>
                  <span class="row">
                    {#each CHIME_KINDS as item (item.kind)}
                      <button type="button" class="btn btn-sm" onclick={() => dev.playChime(sound, item.kind)}>
                        {item.label}
                      </button>
                    {/each}
                  </span>
                </li>
              {/each}
            </ul>
          </fieldset>
        </section>

        <!-- Rewards (beta, ADR-022) -->
        <DevRewards />
      </div>
    </div>

    <ConfirmDialog
      bind:open={resetOpen}
      title={DEV.factoryResetTitle}
      body={resetBody}
      list={DEV.factoryResetList}
      confirmLabel={DEV.factoryResetConfirm}
      cancelLabel={DEV.cancel}
      danger
      onconfirm={() => dev.factoryReset()}
    />
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
    align-items: center;
    gap: 12px;
  }

  .lead {
    margin-top: -8px;
    max-width: 72ch;
  }

  .toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    align-items: center;
    gap: 12px 24px;
  }

  .status {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
    color: var(--text-2);
    overflow-wrap: anywhere;
  }

  .status .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    transform: translateY(-1px);
  }

  .status.error {
    color: var(--danger);
    background: var(--danger-soft);
  }

  .status.error .dot {
    background: var(--danger);
  }

  /* Two independent columns (cards of very different heights), one under the other in a narrow window. */
  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 420px), 1fr));
    gap: 16px;
    align-items: start;
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }

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

  /* Long labels wrap inside the button instead of overflowing the card. */
  .buttons .btn {
    white-space: normal;
    text-align: center;
  }

  .split {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .sub-switch {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  /* Factory reset: apart from the everyday buttons, with what it does next to it. */
  .danger-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .danger-row .hint {
    flex: 1 1 220px;
    min-width: 0;
  }

  .readout {
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 650;
  }

  .readout.break {
    background: var(--break-soft);
    color: var(--break);
  }

  .notice {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px dashed var(--border-strong);
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
    margin-bottom: 8px;
    padding: 0;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .set legend + * {
    clear: both;
  }

  .set:disabled {
    opacity: 0.6;
  }

  .active {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .chimes {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chimes li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 0;
  }

  .chime-name {
    font-weight: 600;
    color: var(--text-2);
  }

  @media (max-width: 720px) {
    .toolbar {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
