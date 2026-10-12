<script lang="ts">
  import { pickFolder, toAppError } from "$lib/api";
  import {
    classifyLink,
    isPlayableKind,
    playsInWindow,
    presetFor,
    splitLines,
    STREAM_PRESETS,
    type LinkKind,
  } from "$lib/player/sources";
  import { config } from "$lib/state/config.svelte";
  import { music } from "$lib/state/music.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";
  import type { MusicSourceKind, NoiseColor } from "$lib/types";

  const SOURCES: MusicSourceKind[] = ["stream", "links", "folder", "noise", "none"];
  const NOISES: NoiseColor[] = ["brown", "pink", "white"];
  const MAX_BAD_LINKS_SHOWN = 3;

  const m = $derived(config.value?.music ?? null);

  // Links textarea keeps its own text while focused, so normalization from the shell never fights the cursor.
  let linksText = $state("");
  let linksFocused = $state(false);
  $effect(() => {
    const links = config.value?.music.links;
    if (links && !linksFocused) linksText = links.join("\n");
  });

  // Same for the stream URL: the shell adds `https://` to bare links, which must not jump the cursor mid-typing.
  let streamText = $state("");
  let streamFocused = $state(false);
  $effect(() => {
    const url = config.value?.music.streamUrl;
    if (url !== undefined && !streamFocused) streamText = url;
  });

  const linkRows = $derived(splitLines(linksText).map((url) => ({ url, kind: classifyLink(url) })));
  const playableLinks = $derived(linkRows.filter((r) => isPlayableKind(r.kind)).length);
  const badLinks = $derived(linkRows.filter((r) => !isPlayableKind(r.kind)));

  const streamKind = $derived(streamText.trim() ? classifyLink(streamText) : null);
  const streamHint = $derived.by(() => {
    if (!streamKind) return "";
    const preset = presetFor(streamText);
    if (preset?.hint) return preset.hint;
    return playsInWindow(streamText) ? strings.music.detected.youtubeWindow : detectedText(streamKind);
  });
  const folderReady = $derived(m?.folder !== null && m?.folder !== undefined && music.folder === m.folder);

  function detectedText(kind: LinkKind): string {
    return strings.music.detected[kind];
  }

  function setSource(source: MusicSourceKind) {
    config.update((c) => {
      c.music.source = source;
    }, true);
  }

  function setStreamUrl(event: Event & { currentTarget: HTMLInputElement }) {
    const value = event.currentTarget.value;
    streamText = value;
    config.update((c) => {
      c.music.streamUrl = value;
    });
  }

  function onStreamBlur() {
    streamFocused = false;
    streamText = config.value?.music.streamUrl ?? streamText;
  }

  function usePreset(url: string) {
    streamText = url;
    config.update((c) => {
      c.music.streamUrl = url;
    }, true);
  }

  function onLinksInput(event: Event & { currentTarget: HTMLTextAreaElement }) {
    linksText = event.currentTarget.value;
    const links = splitLines(linksText);
    config.update((c) => {
      c.music.links = links;
    });
  }

  function onLinksBlur() {
    linksFocused = false;
    linksText = (config.value?.music.links ?? []).join("\n");
  }

  function setShuffle(event: Event & { currentTarget: HTMLInputElement }) {
    const checked = event.currentTarget.checked;
    config.update((c) => {
      c.music.shuffle = checked;
    }, true);
  }

  function setNoise(color: NoiseColor) {
    config.update((c) => {
      c.music.noise = color;
    }, true);
  }

  async function chooseFolder() {
    try {
      const folder = await pickFolder(strings.music.pickFolderTitle, m?.folder ?? null);
      if (!folder) return;
      config.update((c) => {
        c.music.folder = folder;
      }, true);
      await music.ensureTracks(folder, true);
    } catch (e) {
      toasts.error(toAppError(e));
    }
  }
</script>

<section class="card" aria-labelledby="music-title">
  <div class="card-title">
    <h2 id="music-title">{strings.music.title}</h2>
  </div>

  {#if m}
    <fieldset class="segmented">
      <legend>{strings.music.sourceLabel}</legend>
      {#each SOURCES as source (source)}
        <label>
          <input
            type="radio"
            name="music-source"
            value={source}
            checked={m.source === source}
            onchange={() => setSource(source)}
          />
          <span>{strings.music.source[source]}</span>
        </label>
      {/each}
    </fieldset>

    <div class="panel">
      {#if m.source === "stream"}
        <div class="field">
          <label for="stream-url">{strings.music.streamUrl}</label>
          <input
            id="stream-url"
            class="input"
            type="url"
            spellcheck="false"
            autocomplete="off"
            placeholder={strings.music.streamPlaceholder}
            value={streamText}
            onfocus={() => (streamFocused = true)}
            onblur={onStreamBlur}
            oninput={setStreamUrl}
            aria-invalid={config.fieldErrors["music.streamUrl"] ? "true" : undefined}
            aria-describedby="stream-hint"
          />
          <p class="hint" id="stream-hint" class:warn={streamKind === "unsupported" || streamKind === "invalid"}>
            {streamHint}
          </p>
          {#if config.fieldErrors["music.streamUrl"]}
            <p class="field-error">{config.fieldErrors["music.streamUrl"]}</p>
          {/if}
        </div>
        <div class="presets" role="group" aria-label={strings.music.presetsLabel}>
          {#each STREAM_PRESETS as preset (preset.url)}
            <button
              type="button"
              class="preset-btn"
              aria-pressed={streamText.trim() === preset.url}
              title={preset.hint ?? undefined}
              onclick={() => usePreset(preset.url)}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                <path d="M8 5v14l11-7z" fill="currentColor" />
              </svg>
              {preset.label}
            </button>
          {/each}
        </div>
      {:else if m.source === "links"}
        <div class="field">
          <label for="links">{strings.music.linksLabel}</label>
          <textarea
            id="links"
            class="textarea"
            rows="5"
            spellcheck="false"
            placeholder={strings.music.linksPlaceholder}
            value={linksText}
            onfocus={() => (linksFocused = true)}
            onblur={onLinksBlur}
            oninput={onLinksInput}
            aria-invalid={config.fieldErrors["music.links"] ? "true" : undefined}
            aria-describedby="links-hint"
          ></textarea>
          <div class="hint" id="links-hint">
            {#if linkRows.length > 0}
              <p>{strings.music.linksSummary(playableLinks, badLinks.length)}</p>
              {#each badLinks.slice(0, MAX_BAD_LINKS_SHOWN) as bad, i (i)}
                <p class="warn bad-link" title={bad.url}>{bad.url} — {detectedText(bad.kind)}</p>
              {/each}
            {/if}
            {#if config.fieldErrors["music.links"]}
              <p class="field-error">{config.fieldErrors["music.links"]}</p>
            {/if}
          </div>
        </div>
        <label class="check">
          <input type="checkbox" checked={m.shuffle} onchange={setShuffle} />
          {strings.music.shuffle}
        </label>
      {:else if m.source === "folder"}
        <div class="folder-row">
          <button type="button" class="btn" onclick={chooseFolder}>
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
              <path
                d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
                fill="none"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linejoin="round"
              />
            </svg>
            {m.folder ? strings.music.changeFolder : strings.music.pickFolder}
          </button>
          <div class="folder-info" aria-live="polite">
            {#if m.folder}
              <p class="folder-path" title={m.folder}>{m.folder}</p>
              <p class="hint">
                {#if music.loading}
                  {strings.music.loadingTracks}
                {:else if folderReady && music.tracks.length === 0}
                  <span class="warn">{strings.music.noTracks}</span>
                {:else if folderReady}
                  {strings.music.trackCount(music.tracks.length)}
                {/if}
              </p>
            {:else}
              <p class="hint">{strings.music.noFolder}</p>
            {/if}
          </div>
        </div>
        <label class="check">
          <input type="checkbox" checked={m.shuffle} onchange={setShuffle} />
          {strings.music.shuffle}
        </label>
      {:else if m.source === "noise"}
        <fieldset class="segmented small-seg">
          <legend>{strings.music.noiseLabel}</legend>
          {#each NOISES as color (color)}
            <label>
              <input
                type="radio"
                name="noise-color"
                value={color}
                checked={m.noise === color}
                onchange={() => setNoise(color)}
              />
              <span>{strings.music.noise[color]}</span>
            </label>
          {/each}
        </fieldset>
        <p class="hint">{strings.music.noiseHint}</p>
      {:else}
        <p class="hint">{strings.music.noneHint}</p>
      {/if}
    </div>
  {:else}
    <p class="hint">{strings.player.loading}</p>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 16px;
  }

  .hint {
    min-height: 1.2em;
  }

  .warn {
    color: var(--danger);
  }

  .bad-link {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .preset-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border-strong);
    background: transparent;
    color: var(--text-2);
    font: inherit;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    max-width: 100%;
  }

  .preset-btn:hover {
    background: var(--surface-inset);
    color: var(--text);
  }

  .preset-btn[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    cursor: pointer;
    width: fit-content;
  }

  .check input {
    width: 18px;
    height: 18px;
  }

  .folder-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .folder-info {
    flex: 1;
    min-width: 0;
  }

  .folder-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.86rem;
  }

  .small-seg {
    max-width: 360px;
  }
</style>
