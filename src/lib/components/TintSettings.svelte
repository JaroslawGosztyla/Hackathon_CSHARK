<script lang="ts">
  // Warm screen colours by time of day: mechanism (Windows Night light or display gamma), schedule (fixed hours or
  // sunset → sunrise at a place the user picks or types; computed offline by the shell, ADR-023), strength with a
  // live preview, fade length, the exceptions (colour-critical apps, full screen) and the status the shell reports
  // (fallback mechanism, capped strength, errors, what suspends the warmth).
  import { onDestroy, onMount, tick } from "svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { formatTimeOfDay, minutesToTime, processLabel, timeToMinutes } from "$lib/format";
  import { intlLocale } from "$lib/i18n/index.svelte";
  import {
    config,
    DEFAULT_TINT_PAUSE_APPS,
    isProcessName,
    LIMITS,
    TINT_TRANSITION_PRESETS,
  } from "$lib/state/config.svelte";
  import { tint } from "$lib/state/tint.svelte";
  import { strings } from "$lib/strings";
  import type { Config, SunCity, TintMode, TintPause, TintSchedule } from "$lib/types";

  const MODES: readonly TintMode[] = ["nightLight", "gamma"];
  const SCHEDULES: readonly TintSchedule[] = ["fixed", "sun"];
  /** `<select>` value of "custom coordinates" (city ids are `xx-name`, never this). */
  const CUSTOM_PLACE = "custom";
  /** A country with at least this many built-in cities gets its own group in the city list. */
  const GROUP_MIN_CITIES = 3;
  /** Appended to a typed app name without it: the foreground reports exe names. */
  const EXE_SUFFIX = ".exe";

  /** The preview follows the slider at most this often. */
  const PREVIEW_DEBOUNCE_MS = 150;
  /** A keyboard change keeps its preview this long after the last key, so it can be seen at all. */
  const KEYBOARD_PREVIEW_MS = 1500;

  const s = strings.settings;
  const cfg = $derived(config.value);
  const errors = $derived(config.fieldErrors);

  onMount(() => void tint.loadCities());

  const transitionOptions = $derived.by((): number[] => {
    const presets: number[] = [...TINT_TRANSITION_PRESETS];
    const current = cfg?.tint.transitionMinutes;
    if (current !== undefined && !presets.includes(current)) presets.push(current);
    return presets.sort((a, b) => a - b);
  });

  const status = $derived.by(
    (): { text: string; error: boolean; paused: boolean; detail: string | null } | null => {
      const st = tint.shown;
      if (!st) return null;
      // `error` with `active` is only a note (e.g. Night light cannot be controlled on this Windows, so the gamma
      // fallback warms the screen): the warmth works, the note below says which mechanism is used.
      if (st.error && !st.active) {
        const text = st.level === 0 ? s.tintStatusRestoreError : s.tintStatusError;
        return { text, error: true, paused: false, detail: st.error };
      }
      if (st.preview) return { text: s.tintStatusPreview(st.level), error: false, paused: false, detail: st.error };
      if (st.enabled && st.pausedBy) {
        return { text: pausedText(st.pausedBy), error: false, paused: true, detail: st.error };
      }
      if (!st.enabled || st.level === 0) return { text: s.tintStatusOff, error: false, paused: false, detail: st.error };
      return { text: s.tintStatusLevel(st.level), error: false, paused: false, detail: st.error };
    },
  );

  /** "Paused: Photoshop in front" (process name without `.exe`; never a window title). */
  function pausedText(pause: TintPause): string {
    const app = pause.app ? processLabel(pause.app) : null;
    if (pause.reason === "app" && app) return s.tintStatusPausedApp(app);
    return app ? s.tintStatusPausedFullscreenApp(app) : s.tintStatusPausedFullscreen;
  }

  /** Extra lines under the status: the shell fell back to the other mechanism, or Windows capped the strength. */
  const notes = $derived.by((): string[] => {
    const st = tint.shown;
    const configured = cfg?.tint.mode;
    if (!st || !configured) return [];
    const out: string[] = [];
    if (st.mode !== configured) out.push(s.tintStatusFallback(s.tintModeShort[st.mode]));
    if (st.capped) out.push(s.tintStatusCapped);
    return out;
  });

  function setMode(value: string): void {
    const mode = MODES.find((m) => m === value);
    if (mode) set((c) => (c.tint.mode = mode));
  }

  function setSchedule(value: string): void {
    const schedule = SCHEDULES.find((v) => v === value);
    if (schedule) set((c) => (c.tint.schedule = schedule));
  }

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }

  function onTime(event: Event & { currentTarget: HTMLInputElement }, apply: (c: Config, minutes: number) => void): void {
    // An incomplete time reads as "": keep the stored value until the input holds a whole time again.
    const minutes = timeToMinutes(event.currentTarget.value);
    if (minutes === null) return;
    config.update((c) => apply(c, minutes));
  }

  function onTransition(event: Event & { currentTarget: HTMLSelectElement }): void {
    const minutes = Number(event.currentTarget.value);
    if (Number.isInteger(minutes)) set((c) => (c.tint.transitionMinutes = minutes));
  }

  // ---------------------------------------------------------------------------
  // Sunset → sunrise: the place (a built-in city or typed coordinates)
  // ---------------------------------------------------------------------------

  /** "Custom coordinates" was picked before any were typed (nothing stored yet). */
  let customPicked = $state(false);
  /** What the coordinate inputs hold while typed (`null`: show the stored value). */
  let latDraft = $state<string | null>(null);
  let lonDraft = $state<string | null>(null);

  const regionNames = $derived.by((): Intl.DisplayNames | null => {
    try {
      return new Intl.DisplayNames([intlLocale()], { type: "region" });
    } catch {
      return null;
    }
  });

  function countryName(code: string): string {
    try {
      return regionNames?.of(code) ?? code;
    } catch {
      return code;
    }
  }

  /** The city list in groups: countries with several cities on their own (in list order), the rest together. */
  const cityGroups = $derived.by((): { key: string; label: string; own: boolean; cities: SunCity[] }[] => {
    const counts = new Map<string, number>();
    for (const city of tint.cities) counts.set(city.country, (counts.get(city.country) ?? 0) + 1);
    const groups = new Map<string, SunCity[]>();
    const rest: SunCity[] = [];
    for (const city of tint.cities) {
      if ((counts.get(city.country) ?? 0) < GROUP_MIN_CITIES) {
        rest.push(city);
        continue;
      }
      const group = groups.get(city.country) ?? [];
      group.push(city);
      groups.set(city.country, group);
    }
    const out = [...groups].map(([country, cities]) => ({ key: country, label: countryName(country), own: true, cities }));
    if (rest.length > 0) out.push({ key: "", label: s.tintPlaceOther, own: false, cities: rest });
    return out;
  });

  const placeValue = $derived.by((): string => {
    const location = cfg?.tint.location;
    if (location?.city && tint.cities.some((c) => c.id === location.city)) return location.city;
    return location || customPicked ? CUSTOM_PLACE : "";
  });

  const degrees = $derived(new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: 4 }));

  /** Coordinates of the chosen city, for the hint under the list. */
  const cityCoords = $derived.by((): string | null => {
    const location = cfg?.tint.location;
    if (!location || placeValue === CUSTOM_PLACE || placeValue === "") return null;
    return s.tintCoordsShown(`${degrees.format(location.latitude)}°`, `${degrees.format(location.longitude)}°`);
  });

  /** Today's sunset and sunrise as the shell computed them, or why the backup hours apply. */
  const scheduleNote = $derived.by((): string | null => {
    const st = tint.shown;
    if (!st || cfg?.tint.schedule !== "sun") return null;
    switch (st.schedule) {
      case "sun":
        return s.tintSunToday(timeOfDay(st.windowStartMinutes), timeOfDay(st.windowEndMinutes));
      case "polarDay":
      case "polarNight":
      case "noLocation":
        return s.tintScheduleFallback[st.schedule];
      default:
        // "fixed": the shell has not applied the change yet.
        return null;
    }
  });

  /** Minutes after midnight as a time of day in the locale. */
  function timeOfDay(minutes: number): string {
    return formatTimeOfDay(new Date(2000, 0, 1, Math.floor(minutes / 60), minutes % 60).getTime());
  }

  function onPlace(event: Event & { currentTarget: HTMLSelectElement }): void {
    const value = event.currentTarget.value;
    if (value === CUSTOM_PLACE) {
      customPicked = true;
      latDraft = null;
      lonDraft = null;
      // From a city to custom: start from its coordinates.
      const location = cfg?.tint.location;
      if (location?.city) set((c) => (c.tint.location = { ...location, city: null }));
      return;
    }
    const city = tint.cities.find((c) => c.id === value);
    if (!city) return;
    customPicked = false;
    latDraft = null;
    lonDraft = null;
    set((c) => (c.tint.location = { latitude: city.latitude, longitude: city.longitude, city: city.id }));
  }

  const latShown = $derived(latDraft ?? (cfg?.tint.location ? String(cfg.tint.location.latitude) : ""));
  const lonShown = $derived(lonDraft ?? (cfg?.tint.location ? String(cfg.tint.location.longitude) : ""));
  const latParsed = $derived(parseDegrees(latShown, LIMITS.tint.maxLatitude));
  const lonParsed = $derived(parseDegrees(lonShown, LIMITS.tint.maxLongitude));
  const latError = $derived(
    latShown.trim() !== "" && latParsed === null ? s.tintLatitudeRange : (errors["tint.location.latitude"] ?? null),
  );
  const lonError = $derived(
    lonShown.trim() !== "" && lonParsed === null ? s.tintLongitudeRange : (errors["tint.location.longitude"] ?? null),
  );

  /** Decimal degrees within ±`max` (a comma or a typographic minus accepted), else null. */
  function parseDegrees(text: string, max: number): number | null {
    const normalized = text.trim().replace(",", ".").replace(/^[−–]/, "-");
    if (!/^[+-]?(\d+(\.\d*)?|\.\d+)$/.test(normalized)) return null;
    const value = Number(normalized);
    return Number.isFinite(value) && Math.abs(value) <= max ? value : null;
  }

  function onCoordinate(which: "lat" | "lon", text: string): void {
    if (which === "lat") latDraft = text;
    else lonDraft = text;
    const latitude = latParsed;
    const longitude = lonParsed;
    if (latitude === null || longitude === null) return;
    config.update((c) => (c.tint.location = { latitude, longitude, city: null }));
  }

  /** Leaving a coordinate field: show the stored values again once both are valid. */
  function onCoordinateBlur(): void {
    if (latParsed !== null && lonParsed !== null) {
      latDraft = null;
      lonDraft = null;
    }
  }

  // ---------------------------------------------------------------------------
  // Exceptions: colour-critical apps
  // ---------------------------------------------------------------------------

  let newApp = $state("");
  let appError = $state<string | null>(null);
  let appList = $state<HTMLUListElement | null>(null);
  let newAppInput = $state<HTMLInputElement | null>(null);

  const isDefaultList = $derived.by((): boolean => {
    const apps = cfg?.tint.pauseApps ?? [];
    return apps.length === DEFAULT_TINT_PAUSE_APPS.length && apps.every((app, i) => app === DEFAULT_TINT_PAUSE_APPS[i]);
  });

  function addApp(): void {
    const current = cfg?.tint.pauseApps;
    if (!current) return;
    let name = newApp.trim();
    if (name === "") return;
    if (!name.toLowerCase().endsWith(EXE_SUFFIX)) name += EXE_SUFFIX;
    if (!isProcessName(name)) {
      appError = s.tintPauseAppInvalid(LIMITS.tint.maxProcessNameChars);
      return;
    }
    const lower = name.toLowerCase();
    if (current.some((app) => app.toLowerCase() === lower)) {
      appError = s.tintPauseAppDuplicate;
      return;
    }
    if (current.length >= LIMITS.tint.maxPauseApps) {
      appError = s.tintPauseAppsFull(LIMITS.tint.maxPauseApps);
      return;
    }
    set((c) => (c.tint.pauseApps = [...c.tint.pauseApps, name]));
    newApp = "";
    appError = null;
  }

  function onNewAppKey(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      addApp();
    }
  }

  async function removeApp(index: number): Promise<void> {
    set((c) => (c.tint.pauseApps = c.tint.pauseApps.filter((_, i) => i !== index)));
    appError = null;
    await tick();
    // Keep the keyboard where it was: the next remove button, else the previous one, else the input.
    const buttons = appList?.querySelectorAll<HTMLButtonElement>("button") ?? [];
    const next = buttons[Math.min(index, buttons.length - 1)];
    (next ?? newAppInput)?.focus();
  }

  async function resetApps(): Promise<void> {
    set((c) => (c.tint.pauseApps = [...DEFAULT_TINT_PAUSE_APPS]));
    appError = null;
    await tick();
    // The button is gone with the default list: keep the keyboard in the card.
    newAppInput?.focus();
  }

  // ---------------------------------------------------------------------------
  // Strength slider with a live preview on the screen
  // ---------------------------------------------------------------------------

  let previewTimer: ReturnType<typeof setTimeout> | null = null;
  let endTimer: ReturnType<typeof setTimeout> | null = null;
  let previewing = false;
  let pointerDown = false;

  function clearTimers(): void {
    if (previewTimer !== null) clearTimeout(previewTimer);
    if (endTimer !== null) clearTimeout(endTimer);
    previewTimer = null;
    endTimer = null;
  }

  function onStrengthInput(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.valueAsNumber;
    if (!Number.isFinite(value)) return;
    const strength = Math.round(Math.min(LIMITS.tint.maxStrength, Math.max(0, value)));
    config.update((c) => (c.tint.strength = strength));
    clearTimers();
    previewTimer = setTimeout(() => {
      previewTimer = null;
      previewing = true;
      void tint.preview(strength);
    }, PREVIEW_DEBOUNCE_MS);
  }

  /** Released after dragging: end the preview now. Keyboard: end it shortly after the last key. */
  function onStrengthChange(): void {
    if (pointerDown) endPreview();
    else {
      if (endTimer !== null) clearTimeout(endTimer);
      endTimer = setTimeout(endPreview, KEYBOARD_PREVIEW_MS);
    }
  }

  function onPointerDown(): void {
    pointerDown = true;
  }

  function onPointerUp(): void {
    // `change` fires right after `pointerup` (same task) when the value moved; clear the flag after it.
    setTimeout(() => (pointerDown = false), 0);
  }

  function endPreview(): void {
    const pendingPreview = previewTimer !== null;
    clearTimers();
    if (previewing || pendingPreview) {
      previewing = false;
      void tint.preview(null);
    }
  }

  onDestroy(endPreview);
</script>

{#if cfg}
  <Switch
    id="tint-enabled"
    label={s.tint}
    hint={s.tintHint}
    checked={cfg.tint.enabled}
    onchange={(on) => set((c) => (c.tint.enabled = on))}
  >
    {#if cfg.tint.enabled}
      <div class="tint-fields">
        <fieldset class="modes">
          <legend class="field-label">{s.tintMode}</legend>
          {#each MODES as mode (mode)}
            <label class="mode" class:selected={cfg.tint.mode === mode}>
              <input
                type="radio"
                name="tint-mode"
                value={mode}
                checked={cfg.tint.mode === mode}
                aria-describedby="tint-mode-{mode}-hint"
                onchange={() => setMode(mode)}
              />
              <span class="mode-text">
                <span class="mode-name">{s.tintModes[mode]}</span>
                <span class="hint" id="tint-mode-{mode}-hint">{s.tintModeHints[mode]}</span>
              </span>
            </label>
          {/each}
        </fieldset>

        <fieldset class="modes">
          <legend class="field-label">{s.tintWhen}</legend>
          {#each SCHEDULES as schedule (schedule)}
            <label class="mode" class:selected={cfg.tint.schedule === schedule}>
              <input
                type="radio"
                name="tint-schedule"
                value={schedule}
                checked={cfg.tint.schedule === schedule}
                aria-describedby={schedule === "sun" ? "tint-schedule-sun-hint" : undefined}
                onchange={() => setSchedule(schedule)}
              />
              <span class="mode-text">
                <span class="mode-name">{s.tintSchedules[schedule]}</span>
                {#if schedule === "sun"}
                  <span class="hint" id="tint-schedule-sun-hint">{s.tintScheduleSunHint}</span>
                {/if}
              </span>
            </label>
          {/each}
        </fieldset>

        {#if cfg.tint.schedule === "sun"}
          <div class="place">
            <div class="field">
              <label for="tint-place">{s.tintPlace}</label>
              <!-- Re-created when the list arrives, so the stored city is selected among the new options. -->
              {#key tint.cities.length}
                <select
                  id="tint-place"
                  class="input select place-select"
                  value={placeValue}
                  aria-invalid={errors["tint.location.city"] ? "true" : undefined}
                  aria-describedby="tint-place-note"
                  onchange={onPlace}
                >
                  <option value="" disabled>{s.tintPlacePick}</option>
                  {#each cityGroups as group (group.key)}
                    <optgroup label={group.label}>
                      {#each group.cities as city (city.id)}
                        <option value={city.id}>
                          {group.own ? city.name : `${city.name} (${countryName(city.country)})`}
                        </option>
                      {/each}
                    </optgroup>
                  {/each}
                  <option value={CUSTOM_PLACE}>{s.tintPlaceCustom}</option>
                </select>
              {/key}
              {#if errors["tint.location.city"]}<p class="field-error">{errors["tint.location.city"]}</p>{/if}
            </div>

            {#if placeValue === CUSTOM_PLACE}
              <div class="coords">
                <div class="field">
                  <label for="tint-latitude">{s.tintLatitude}</label>
                  <input
                    id="tint-latitude"
                    class="input coord"
                    type="text"
                    inputmode="decimal"
                    autocomplete="off"
                    spellcheck="false"
                    value={latShown}
                    aria-invalid={latError ? "true" : undefined}
                    aria-describedby="tint-coords-hint"
                    oninput={(e) => onCoordinate("lat", e.currentTarget.value)}
                    onblur={onCoordinateBlur}
                  />
                  {#if latError}<p class="field-error">{latError}</p>{/if}
                </div>
                <div class="field">
                  <label for="tint-longitude">{s.tintLongitude}</label>
                  <input
                    id="tint-longitude"
                    class="input coord"
                    type="text"
                    inputmode="decimal"
                    autocomplete="off"
                    spellcheck="false"
                    value={lonShown}
                    aria-invalid={lonError ? "true" : undefined}
                    aria-describedby="tint-coords-hint"
                    oninput={(e) => onCoordinate("lon", e.currentTarget.value)}
                    onblur={onCoordinateBlur}
                  />
                  {#if lonError}<p class="field-error">{lonError}</p>{/if}
                </div>
                <p class="hint wide" id="tint-coords-hint">{s.tintCoordsHint}</p>
              </div>
            {/if}

            <div id="tint-place-note" class="place-note" aria-live="polite">
              {#if cityCoords}<p class="hint">{cityCoords}</p>{/if}
              {#if scheduleNote}<p class="sun-today" class:fallback={tint.shown?.schedule !== "sun"}>{scheduleNote}</p>{/if}
            </div>
          </div>
        {/if}

        <div class="schedule">
          <div class="field">
            <label for="tint-start">{s.tintFrom}</label>
            <input
              id="tint-start"
              class="input time"
              type="time"
              step="60"
              value={minutesToTime(cfg.tint.startMinutes)}
              aria-invalid={errors["tint.startMinutes"] ? "true" : undefined}
              aria-describedby="tint-schedule-hint"
              oninput={(e) => onTime(e, (c, m) => (c.tint.startMinutes = m))}
            />
            {#if errors["tint.startMinutes"]}<p class="field-error">{errors["tint.startMinutes"]}</p>{/if}
          </div>
          <div class="field">
            <label for="tint-end">{s.tintTo}</label>
            <input
              id="tint-end"
              class="input time"
              type="time"
              step="60"
              value={minutesToTime(cfg.tint.endMinutes)}
              aria-invalid={errors["tint.endMinutes"] ? "true" : undefined}
              aria-describedby="tint-schedule-hint"
              oninput={(e) => onTime(e, (c, m) => (c.tint.endMinutes = m))}
            />
            {#if errors["tint.endMinutes"]}<p class="field-error">{errors["tint.endMinutes"]}</p>{/if}
          </div>
          <p class="hint schedule-hint" id="tint-schedule-hint">
            {cfg.tint.schedule === "sun" ? s.tintFallbackHint : s.tintScheduleHint}
          </p>
        </div>

        <div class="field">
          <label for="tint-strength">{s.tintStrength}</label>
          <div class="range-row">
            <input
              id="tint-strength"
              type="range"
              min="0"
              max={LIMITS.tint.maxStrength}
              step="1"
              value={cfg.tint.strength}
              aria-valuetext={`${cfg.tint.strength}%`}
              aria-describedby="tint-strength-hint"
              oninput={onStrengthInput}
              onchange={onStrengthChange}
              onpointerdown={onPointerDown}
              onpointerup={onPointerUp}
              onpointercancel={onPointerUp}
              onblur={endPreview}
            />
            <span class="tabular value">{cfg.tint.strength}%</span>
          </div>
          <p class="hint" id="tint-strength-hint">{s.tintStrengthHint}</p>
          {#if errors["tint.strength"]}<p class="field-error">{errors["tint.strength"]}</p>{/if}
        </div>

        <div class="field transition">
          <label for="tint-transition">{s.tintTransition}</label>
          <select
            id="tint-transition"
            class="input select"
            value={String(cfg.tint.transitionMinutes)}
            onchange={onTransition}
          >
            {#each transitionOptions as minutes (minutes)}
              <option value={String(minutes)}>
                {minutes === 0 ? s.tintTransitionNone : s.tintTransitionMinutes(minutes)}
              </option>
            {/each}
          </select>
          {#if errors["tint.transitionMinutes"]}<p class="field-error">{errors["tint.transitionMinutes"]}</p>{/if}
        </div>

        <fieldset class="exceptions">
          <legend class="field-label">{s.tintExceptions}</legend>
          <Switch
            id="tint-pause-apps"
            label={s.tintPauseApps}
            hint={s.tintPauseAppsHint}
            checked={cfg.tint.pauseForApps}
            onchange={(on) => set((c) => (c.tint.pauseForApps = on))}
          >
            {#if cfg.tint.pauseForApps}
              <div class="apps">
                <p class="field-label" id="tint-apps-label">{s.tintPauseAppsList}</p>
                {#if cfg.tint.pauseApps.length > 0}
                  <ul class="app-list" bind:this={appList} aria-labelledby="tint-apps-label">
                    {#each cfg.tint.pauseApps as app, i}
                      <li class="chip app">
                        <span class="app-name">{app}</span>
                        <button
                          type="button"
                          class="app-remove"
                          aria-label={s.tintPauseAppRemove(app)}
                          title={s.tintPauseAppRemove(app)}
                          onclick={() => removeApp(i)}
                        >
                          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
                            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
                          </svg>
                        </button>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="hint">{s.tintPauseAppsEmpty}</p>
                {/if}
                <div class="add-row">
                  <input
                    bind:this={newAppInput}
                    bind:value={newApp}
                    class="input add-input"
                    type="text"
                    autocomplete="off"
                    spellcheck="false"
                    maxlength={LIMITS.tint.maxProcessNameChars}
                    placeholder={s.tintPauseAppPlaceholder}
                    aria-labelledby="tint-apps-label"
                    aria-invalid={appError ? "true" : undefined}
                    aria-describedby={appError ? "tint-app-error" : undefined}
                    oninput={() => (appError = null)}
                    onkeydown={onNewAppKey}
                  />
                  <button type="button" class="btn btn-sm" disabled={newApp.trim() === ""} onclick={addApp}>
                    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
                      <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
                    </svg>
                    {s.tintPauseAppAdd}
                  </button>
                </div>
                {#if appError}<p class="field-error" id="tint-app-error">{appError}</p>{/if}
                {#if errors["tint.pauseApps"]}<p class="field-error">{errors["tint.pauseApps"]}</p>{/if}
                {#if !isDefaultList}
                  <button type="button" class="btn btn-sm btn-ghost reset" onclick={resetApps}>
                    {s.tintPauseAppsReset}
                  </button>
                {/if}
              </div>
            {/if}
          </Switch>
          <Switch
            id="tint-pause-fullscreen"
            label={s.tintPauseFullscreen}
            hint={s.tintPauseFullscreenHint}
            checked={cfg.tint.pauseForFullscreen}
            onchange={(on) => set((c) => (c.tint.pauseForFullscreen = on))}
          />
        </fieldset>
      </div>
    {/if}

    {#if status && (cfg.tint.enabled || status.error || tint.shown?.preview || tint.sample !== null)}
      <div class="status-block" aria-live="polite">
        <p class="status" class:error={status.error} class:paused={status.paused} title={status.detail ?? undefined}>
          <span class="swatch" aria-hidden="true"></span>
          {status.text}
        </p>
        {#each notes as note (note)}
          <p class="hint note">{note}</p>
        {/each}
      </div>
    {/if}
  </Switch>
{/if}

<style>
  .tint-fields {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Two fields side by side; the hint under them spans the whole card. */
  .schedule,
  .coords {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 160px)) minmax(0, 1fr);
    gap: 8px 16px;
  }

  /* "Szerokość geograficzna" and friends: room for the label on one line. */
  .coords {
    grid-template-columns: repeat(2, minmax(0, 200px)) minmax(0, 1fr);
  }

  .schedule-hint,
  .coords .wide {
    grid-column: 1 / -1;
  }

  .time,
  .coord {
    max-width: 160px;
  }

  .place {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .place-select {
    max-width: 336px;
  }

  .place-note {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .place-note:empty {
    display: none;
  }

  .sun-today {
    color: var(--text-2);
    font-size: 0.88rem;
    font-weight: 600;
  }

  .sun-today.fallback {
    font-weight: 400;
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

  .transition select {
    max-width: 220px;
  }

  .modes,
  .exceptions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  .modes legend,
  .exceptions legend {
    margin-bottom: 6px;
    padding: 0;
  }

  .exceptions {
    gap: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .mode {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-inset);
    cursor: pointer;
  }

  .mode:hover {
    border-color: var(--border-strong);
  }

  .mode.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .mode input {
    flex: none;
    width: 18px;
    height: 18px;
    margin: 2px 0 0;
  }

  .mode-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .mode-name {
    font-weight: 600;
  }

  .apps {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    margin-top: 10px;
  }

  .app-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
    max-width: 560px;
  }

  .app {
    padding: 3px 4px 3px 10px;
    gap: 4px;
    max-width: 100%;
  }

  .app-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .app-remove {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
  }

  .app-remove:hover {
    color: var(--danger);
    background: var(--danger-soft);
  }

  .app-remove:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .add-input {
    width: 240px;
    max-width: 100%;
  }

  .reset {
    margin-top: 2px;
  }

  .status-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 14px;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    font-size: 0.88rem;
    font-weight: 600;
  }

  .note {
    padding-left: 20px;
  }

  .status.error {
    color: var(--danger);
  }

  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: rgb(255, 130, 30);
    box-shadow: 0 0 0 2px rgba(255, 130, 30, 0.25);
  }

  /* Suspended: the warm dot is hollow, the warmth is off for now. */
  .status.paused .swatch {
    background: transparent;
    box-shadow: inset 0 0 0 2px rgb(255, 130, 30);
  }

  .status.error .swatch {
    background: var(--danger);
    box-shadow: none;
  }
</style>
