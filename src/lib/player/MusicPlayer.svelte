<script lang="ts">
  // Persistent music dock (lives in the root layout, so music survives navigation).
  // Playing rule: play while a phase runs; pause on Pause; stop on Stop; keep playing in the break
  // unless `music.pauseDuringBreak`. Links whose owner refuses embedding (and, when signed in to YouTube, every YouTube
  // item, ADR-021) play in a separate YouTube window (the engine routes play/pause/volume there); the dock then offers
  // "Pokaż okno" / "Otwórz ponownie".
  import { onMount, untrack } from "svelte";
  import { config } from "$lib/state/config.svelte";
  import { mediaWindow } from "$lib/state/mediaWindow.svelte";
  import { music } from "$lib/state/music.svelte";
  import { session } from "$lib/state/session.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { youtubeAccount } from "$lib/state/youtubeAccount.svelte";
  import { strings } from "$lib/strings";
  import { EMPTY_VIEW, MusicEngine, type EngineView } from "./engine";
  import { buildPlaylist, describeSource } from "./sources";

  let ytHost: HTMLDivElement | undefined = $state();
  let engine = $state.raw<MusicEngine | null>(null);
  let view = $state.raw<EngineView>(EMPTY_VIEW);

  const musicCfg = $derived(config.value?.music ?? null);
  const pauseForBreak = $derived(session.phase === "break" && (musicCfg?.pauseDuringBreak ?? false));
  /** Focus extended through a meeting: silent until it ends (the engine pauses, then resumes in place). */
  const mutedForMeeting = $derived(session.snapshot.musicMuted);
  const shouldPlay = $derived(session.phase !== "idle" && !session.paused && !pauseForBreak && !mutedForMeeting);

  /** Local tracks of the current source: the folder's (once listed for that folder) or the bundled library. */
  const sourceTracks = $derived.by(() => {
    if (musicCfg?.source === "folder") {
      return musicCfg.folder !== null && music.folder === musicCfg.folder ? music.tracks : [];
    }
    if (musicCfg?.source === "library") return music.library ?? [];
    return [];
  });
  /** Track count for the idle description; null while it is not known yet. */
  const knownTrackCount = $derived.by((): number | null => {
    if (musicCfg?.source === "folder") return music.folder === musicCfg.folder ? music.tracks.length : null;
    if (musicCfg?.source === "library") return music.library?.length ?? null;
    return null;
  });

  // Rebuild the playlist when the source definition changes or a new session starts (fresh shuffle).
  const playlistKey = $derived(
    musicCfg
      ? JSON.stringify([
          session.snapshot.sessionId,
          musicCfg.source,
          musicCfg.streamUrl,
          musicCfg.links,
          musicCfg.folder,
          musicCfg.noise,
          musicCfg.shuffle,
          sourceTracks.length,
          sourceTracks[0]?.path ?? "",
        ])
      : "",
  );

  const showVideo = $derived(
    view.kind === "youtube" &&
      !view.window &&
      (view.status === "playing" || view.status === "loading" || view.status === "paused"),
  );
  /** The current item plays in the separate YouTube window (and the session is running). */
  const inWindow = $derived(view.window && !session.isIdle);
  const canNext = $derived(view.count > 1 && !session.isIdle);
  const volumePct = $derived(Math.round((musicCfg?.volume ?? 0) * 100));

  const sourceText = $derived(musicCfg ? describeSource(musicCfg, knownTrackCount) : "");

  const title = $derived.by(() => {
    if (!musicCfg) return strings.player.loading;
    if (musicCfg.source === "none") return strings.player.silence;
    if (session.isIdle) return strings.player.idle;
    if (view.count === 0) return strings.player.sourceNotReady;
    if (inWindow && !view.windowClosed && mediaWindow.title) return mediaWindow.title;
    return view.title ?? sourceText;
  });

  const subtitle = $derived.by(() => {
    if (!musicCfg) return "";
    if (musicCfg.source === "none") return strings.music.noneHint;
    if (session.isIdle || view.count === 0) return sourceText;
    if (inWindow && view.windowClosed) return strings.player.windowClosed;
    switch (view.status) {
      case "loading":
        return strings.player.loading;
      case "paused":
      case "stopped":
        if (mutedForMeeting && !session.paused) return strings.player.pausedForMeeting;
        if (pauseForBreak) return strings.player.pausedForBreak;
        return strings.player.pausedBySession;
      case "error":
        return strings.errors.unplayable;
      default:
        if (inWindow) {
          return view.count > 1
            ? `${strings.player.windowModeShort} · ${strings.player.trackOf(view.index + 1, view.count)}`
            : strings.player.windowModeShort;
        }
        return view.count > 1 ? strings.player.trackOf(view.index + 1, view.count) : sourceText;
    }
  });
  /** Full explanation on hover while the window plays (signed in to YouTube: it plays there on the user's account). */
  const subtitleHint = $derived(
    inWindow && !view.windowClosed && view.status === "playing"
      ? youtubeAccount.signedIn
        ? strings.youtube.playerWindowHint
        : strings.player.windowMode
      : subtitle,
  );

  onMount(() => {
    if (!ytHost) return;
    const created = new MusicEngine(ytHost, {
      onChange: (v) => (view = v),
      onError: (text) => toasts.push(text, "error"),
    });
    engine = created;
    return () => {
      created.destroy();
      engine = null;
      music.videoVisible = false;
    };
  });

  // Folder → track list (the shell also allows the folder in the asset protocol scope).
  $effect(() => {
    const m = musicCfg;
    const folder = m?.source === "folder" ? m.folder : null;
    if (folder) untrack(() => void music.ensureTracks(folder));
  });

  // Bundled music → track list (listed once; the shell allows its folder in the asset protocol scope).
  $effect(() => {
    if (musicCfg?.source === "library") untrack(() => void music.ensureLibrary());
  });

  // YouTube sign-in → where YouTube plays (ADR-021). Declared before the playlist effect, so a known sign-in already
  // routes the first item of a new playlist to the YouTube window.
  $effect(() => {
    const signedIn = youtubeAccount.signedIn;
    const eng = engine;
    if (eng) untrack(() => eng.setSignedIn(signedIn));
  });

  $effect(() => {
    const key = playlistKey;
    const eng = engine;
    if (!eng || !key) return;
    untrack(() => {
      const m = config.value?.music;
      if (!m) return;
      // One call: on Stop the new (idle) playlist must not load the first item before pausing it.
      eng.setItems(buildPlaylist(m, sourceTracks), shouldPlay);
    });
  });

  $effect(() => {
    const play = shouldPlay;
    const eng = engine;
    if (eng) untrack(() => eng.setPlaying(play));
  });

  $effect(() => {
    const volume = musicCfg?.volume;
    const eng = engine;
    if (eng && volume !== undefined) untrack(() => eng.setVolume(volume));
  });

  $effect(() => {
    music.videoVisible = showVideo;
  });

  // Open/closed/title of the separate YouTube window → engine (detects a window closed by the user).
  $effect(() => {
    const status = mediaWindow.status;
    const eng = engine;
    if (eng) untrack(() => eng.setWindowStatus(status));
  });

  function onVolume(event: Event & { currentTarget: HTMLInputElement }) {
    const pct = event.currentTarget.valueAsNumber;
    if (!Number.isFinite(pct)) return;
    config.update((c) => {
      c.music.volume = Math.min(1, Math.max(0, pct / 100));
    });
  }
</script>

<section class="dock" aria-label={strings.player.region}>
  <div class="video" class:visible={showVideo} class:compact={!music.videoExpanded}>
    <div class="yt-host" bind:this={ytHost}></div>
    {#if !music.videoExpanded}
      <button class="video-expand" type="button" aria-label={strings.player.expand} onclick={() => music.toggleVideo()}
      ></button>
    {/if}
  </div>

  <div class="bar">
    <div class="art" class:playing={view.status === "playing"} aria-hidden="true">
      {#if view.status === "playing"}
        <span class="eq"><i></i><i></i><i></i><i></i></span>
      {:else if musicCfg?.source === "noise"}
        <svg viewBox="0 0 24 24" width="22" height="22">
          <path
            d="M2 12c2-5 4-5 6 0s4 5 6 0 4-5 6 0 2 3 2 3"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
          />
        </svg>
      {:else if musicCfg?.source === "none"}
        <svg viewBox="0 0 24 24" width="22" height="22">
          <path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" fill="currentColor" />
        </svg>
      {:else}
        <svg viewBox="0 0 24 24" width="22" height="22">
          <path d="M9 18V5l11-2v13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" />
          <circle cx="6.5" cy="18" r="2.6" fill="currentColor" />
          <circle cx="17.5" cy="16" r="2.6" fill="currentColor" />
        </svg>
      {/if}
    </div>

    <div class="meta">
      <p class="title" title={title}>{title}</p>
      <p class="sub" title={subtitleHint}>{subtitle}</p>
    </div>

    <div class="controls">
      {#if inWindow && view.windowClosed}
        <button
          class="btn btn-sm"
          type="button"
          onclick={() => engine?.reopenWindow()}
          aria-label={strings.player.reopenWindowAria}
        >
          {strings.player.reopenWindow}
        </button>
      {:else if inWindow}
        <button
          class="btn btn-sm"
          type="button"
          onclick={() => engine?.showWindow()}
          aria-label={strings.player.showWindowAria}
          title={strings.player.showWindowAria}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <rect x="3" y="5" width="18" height="14" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.8" />
            <path d="M3 9h18" stroke="currentColor" stroke-width="1.8" />
          </svg>
          <span class="btn-label">{strings.player.showWindow}</span>
        </button>
      {/if}
      {#if showVideo}
        <button
          class="icon-btn"
          type="button"
          onclick={() => music.toggleVideo()}
          aria-label={music.videoExpanded ? strings.player.collapse : strings.player.expand}
          title={music.videoExpanded ? strings.player.collapse : strings.player.expand}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            {#if music.videoExpanded}
              <path d="M6 9l6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
            {:else}
              <path d="M6 15l6-6 6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
            {/if}
          </svg>
        </button>
      {/if}
      <button
        class="icon-btn"
        type="button"
        onclick={() => engine?.next()}
        disabled={!canNext}
        aria-label={strings.player.next}
        title={strings.player.next}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
          <path d="M5 5l10 7-10 7z" fill="currentColor" />
          <rect x="16.5" y="5" width="2.5" height="14" rx="1" fill="currentColor" />
        </svg>
      </button>
      <label class="volume">
        <span class="sr-only">{strings.player.volume}</span>
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
          <path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" />
          {#if volumePct > 0}
            <path
              d="M16 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          {/if}
        </svg>
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          value={volumePct}
          disabled={!musicCfg}
          oninput={onVolume}
          aria-valuetext={`${volumePct}%`}
        />
      </label>
    </div>
  </div>
</section>

<style>
  .dock {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 30;
    padding: 0 var(--gutter) 12px;
    pointer-events: none;
  }

  .bar {
    pointer-events: auto;
    max-width: 1040px;
    margin: 0 auto;
    min-height: 64px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--surface-solid) 94%, transparent);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    backdrop-filter: blur(14px);
  }

  .art {
    flex: none;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .eq {
    display: flex;
    align-items: flex-end;
    gap: 3px;
    height: 20px;
  }

  .eq i {
    display: block;
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: currentColor;
    transform-origin: bottom;
    animation: eq 1.1s ease-in-out infinite;
  }

  .eq i:nth-child(2) {
    animation-delay: -0.4s;
  }

  .eq i:nth-child(3) {
    animation-delay: -0.75s;
  }

  .eq i:nth-child(4) {
    animation-delay: -0.2s;
  }

  @keyframes eq {
    0%,
    100% {
      transform: scaleY(0.3);
    }
    50% {
      transform: scaleY(1);
    }
  }

  .meta {
    flex: 1;
    min-width: 0;
  }

  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-weight: 650;
  }

  .sub {
    font-size: 0.84rem;
    color: var(--text-muted);
  }

  .controls {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .volume {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
  }

  .volume input {
    width: 110px;
  }

  .video {
    position: fixed;
    right: var(--gutter);
    bottom: calc(var(--dock-h) + 10px);
    z-index: 31;
    display: none;
    width: min(356px, calc(100vw - 2 * var(--gutter)));
    height: 200px;
    border-radius: 16px;
    overflow: hidden;
    background: #000;
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    pointer-events: auto;
    transform-origin: bottom right;
    transition: transform 0.2s ease;
  }

  .video.visible {
    display: block;
  }

  /* Compact: the iframe keeps the 200×200 minimum YouTube requires, shown as a small bubble at the side. */
  .video.compact {
    width: 200px;
    height: 200px;
    border-radius: 40px;
    transform: scale(0.36);
  }

  .yt-host {
    width: 100%;
    height: 100%;
  }

  .yt-host :global(iframe) {
    display: block;
    width: 100%;
    height: 100%;
    border: 0;
  }

  .video-expand {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  @media (prefers-reduced-motion: reduce) {
    .eq i {
      animation: none;
      transform: scaleY(0.7);
    }
  }
</style>
