<script lang="ts">
  // Settings → Dźwięk: chime sound (named like the themes, or the user's own files), chime volume and test buttons.
  // Own files are copied by the shell into the app data folder (`import_chime`) and stored as `break.<ext>` /
  // `focus.<ext>`; the original file name is remembered in this browser profile only, for display.
  import * as api from "$lib/api";
  import { config } from "$lib/state/config.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";
  import type { ChimeKind, ChimeSound, Config } from "$lib/types";

  const s = strings.settings;

  const SOUNDS: readonly ChimeSound[] = ["midnight", "vaporwave", "lunar", "autumn", "classic", "soft", "chiptune", "custom"];
  const KINDS: readonly ChimeKind[] = ["breakStart", "focusStart"];
  const LABELS = $derived<Record<ChimeKind, string>>({ breakStart: s.customBreak, focusStart: s.customFocus });
  const NAME_KEY = "focusmode.customChimeNames";

  /** Stored file name (in the app data folder) → the name of the file the user picked. */
  type NameMap = Record<string, string>;

  const cfg = $derived(config.value);
  const volumeError = $derived(config.fieldErrors.chimeVolume ?? null);
  const fileError = $derived(
    config.fieldErrors["customChimes.breakFile"] ?? config.fieldErrors["customChimes.focusFile"] ?? null,
  );

  /** Import or removal in flight. */
  let busy = $state(false);
  let testing = $state(false);
  let names = $state<NameMap>(readNames());

  function readNames(): NameMap {
    try {
      const raw = localStorage.getItem(NAME_KEY);
      const parsed: unknown = raw ? JSON.parse(raw) : null;
      if (parsed === null || typeof parsed !== "object") return {};
      const out: NameMap = {};
      for (const [key, value] of Object.entries(parsed)) if (typeof value === "string") out[key] = value;
      return out;
    } catch {
      return {};
    }
  }

  function rememberName(file: string | null, original: string): void {
    if (!file) return;
    names = { ...names, [file]: original };
    try {
      localStorage.setItem(NAME_KEY, JSON.stringify(names));
    } catch {
      // Storage unavailable: the stored file name is shown instead.
    }
  }

  function storedFile(c: Config, kind: ChimeKind): string | null {
    return kind === "breakStart" ? c.customChimes.breakFile : c.customChimes.focusFile;
  }

  /** Display name for a stored file: the picked file's name when known. */
  function displayName(file: string): string {
    return names[file] ?? file;
  }

  /** What plays for `kind` without its own file (focus-core `CustomChimes::file_for`). */
  function fallbackText(c: Config, kind: ChimeKind): string {
    const other = storedFile(c, kind === "breakStart" ? "focusStart" : "breakStart");
    if (!other) return s.customDefaultFallback;
    return kind === "breakStart" ? s.customBreakUsesFocus : s.customFocusUsesBreak;
  }

  /** Last path segment of a picked file, for display only. */
  function baseName(path: string): string {
    return path.split(/[\\/]/).pop() || path;
  }

  function setSound(value: string): void {
    const sound = SOUNDS.find((x) => x === value);
    if (sound) config.update((c) => (c.chimeSound = sound), true);
  }

  async function pick(kind: ChimeKind): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const path = await api.pickFile(s.customPickTitle(LABELS[kind]), s.customFilter, api.CHIME_FILE_EXTENSIONS);
      if (!path) return;
      // Pending edits go first, so a late save of the old config cannot overwrite the imported file.
      await config.flush();
      const saved = await api.importChime(path, kind);
      config.adopt(saved, ["chimeSound", "customChimes"]);
      rememberName(storedFile(saved, kind), baseName(path));
      toasts.push(s.customImported, "info");
    } catch (e) {
      toasts.error(e);
    } finally {
      busy = false;
    }
  }

  async function remove(kind: ChimeKind): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await config.flush();
      const saved = await api.removeCustomChime(kind);
      config.adopt(saved, ["chimeSound", "customChimes"]);
    } catch (e) {
      toasts.error(e);
    } finally {
      busy = false;
    }
  }

  async function playChime(kind: ChimeKind): Promise<void> {
    if (testing) return;
    testing = true;
    try {
      // The shell plays the stored sound at the stored volume: save the dropdown and the slider first.
      await config.flush();
      await api.testChime(kind);
    } catch (e) {
      toasts.error(e);
    } finally {
      testing = false;
    }
  }

  function onVolume(event: Event & { currentTarget: HTMLInputElement }): void {
    const pct = event.currentTarget.valueAsNumber;
    if (!Number.isFinite(pct)) return;
    config.update((c) => (c.chimeVolume = Math.min(1, Math.max(0, pct / 100))));
  }
</script>

{#if cfg}
  <div class="item-block">
    <div class="field">
      <label for="chime-sound">{s.chimeSound}</label>
      <select
        id="chime-sound"
        class="input select narrow"
        value={cfg.chimeSound}
        aria-describedby="chime-sound-hint"
        onchange={(e) => setSound(e.currentTarget.value)}
      >
        {#each SOUNDS as sound (sound)}
          <option value={sound}>{s.chimeSounds[sound]}</option>
        {/each}
      </select>
      <p class="hint" id="chime-sound-hint">{s.chimeSoundHint}</p>
    </div>

    {#if cfg.chimeSound === "custom"}
      <div class="custom" role="group" aria-labelledby="custom-chimes-title">
        <p class="field-label" id="custom-chimes-title">{s.customChimes}</p>
        {#each KINDS as kind (kind)}
          {@const file = storedFile(cfg, kind)}
          <div class="file-row">
            <div class="file-info">
              <p class="file-label" id="chime-file-{kind}">{LABELS[kind]}</p>
              {#if file}
                <p class="file-name" title={displayName(file)}>{displayName(file)}</p>
              {:else}
                <p class="file-name none">{s.customNone}</p>
                <p class="hint">{fallbackText(cfg, kind)}</p>
              {/if}
            </div>
            <div class="row file-actions">
              <button
                type="button"
                class="btn btn-sm"
                disabled={busy}
                aria-describedby="chime-file-{kind}"
                onclick={() => pick(kind)}
              >
                {file ? s.customChange : s.customPick}
              </button>
              {#if file}
                <button
                  type="button"
                  class="btn btn-sm btn-ghost"
                  disabled={busy}
                  aria-label={s.customRemoveAria(LABELS[kind])}
                  onclick={() => remove(kind)}
                >
                  {s.customRemove}
                </button>
              {/if}
            </div>
          </div>
        {/each}
        <p class="hint">{s.customHint}</p>
        {#if fileError}<p class="field-error">{fileError}</p>{/if}
      </div>
    {/if}

    <div class="field">
      <label for="chime-volume">{s.chimeVolume}</label>
      <div class="range-row">
        <input
          id="chime-volume"
          type="range"
          min="0"
          max="100"
          step="1"
          value={Math.round(cfg.chimeVolume * 100)}
          aria-valuetext={`${Math.round(cfg.chimeVolume * 100)}%`}
          oninput={onVolume}
        />
        <span class="tabular value">{Math.round(cfg.chimeVolume * 100)}%</span>
      </div>
      {#if volumeError}<p class="field-error">{volumeError}</p>{/if}
    </div>

    <div class="row" role="group" aria-labelledby="test-chime-label">
      <span class="secondary small" id="test-chime-label">{s.testChime}</span>
      <button type="button" class="btn btn-sm" disabled={testing} onclick={() => playChime("breakStart")}>
        {s.testBreak}
      </button>
      <button type="button" class="btn btn-sm" disabled={testing} onclick={() => playChime("focusStart")}>
        {s.testFocus}
      </button>
    </div>
  </div>
{/if}

<style>
  .item-block {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .narrow {
    max-width: 340px;
  }

  .custom {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-inset);
  }

  .file-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px 16px;
  }

  .file-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 200px;
    min-width: 0;
  }

  .file-label {
    font-weight: 600;
    font-size: 0.92rem;
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.84rem;
    color: var(--text-2);
  }

  .file-name.none {
    font-family: inherit;
    color: var(--text-muted);
  }

  .file-actions {
    flex: none;
    gap: 6px;
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
</style>
