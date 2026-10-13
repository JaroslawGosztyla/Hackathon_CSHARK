<script lang="ts">
  // Persistent music dock (lives in the root layout, so music survives navigation).
  // Playing rule (who started the music, `MusicIntent` in state/music.svelte.ts): play/pause in the dock works with or
  // without a session; Start starts the music unless it already plays (kept as it is) or the user paused it; Stop stops
  // only music the session started; while a session runs, Pause, `music.pauseDuringBreak` in the break and the meeting
  // mute hold it silent. Links whose owner refuses embedding (and, when signed in to YouTube, every YouTube
  // item, ADR-021) play in a separate YouTube window (the engine routes play/pause/volume there); the dock then offers
  // "Pokaż okno" / "Otwórz ponownie".
  // Beta (ADR-024): a "Dźwięki tła" button opens the ambience mixer above the dock; the ambience follows the music
  // (`music.audible`, set here from the engine's status) or plays alone when the user starts it there.
  import { onMount, untrack } from "svelte";
  import { ambienceEngine } from "$lib/ambience/engine.svelte";
  import AmbienceHost from "$lib/components/ambience/AmbienceHost.svelte";
  import AmbienceMixer from "$lib/components/ambience/AmbienceMixer.svelte";
  import { ambience } from "$lib/state/ambience.svelte";
  import { config } from "$lib/state/config.svelte";
  import { mediaWindow } from "$lib/state/mediaWindow.svelte";
  import { music } from "$lib/state/music.svelte";
  import { session } from "$lib/state/session.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { youtubeAccount } from "$lib/state/youtubeAccount.svelte";
  import { strings } from "$lib/strings";
  import { EMPTY_VIEW, MusicEngine, type EngineView } from "./engine";
  import { unlockAudio } from "./noise";
  import { buildPlaylist, describeSource } from "./sources";

  /** A source edit while the music plays is applied once the edits pause this long (typing a link or an address). */
  const SOURCE_EDIT_DELAY_MS = 800;

  let ytHost: HTMLDivElement | undefined = $state();
  let engine = $state.raw<MusicEngine | null>(null);
  let view = $state.raw<EngineView>(EMPTY_VIEW);

  const musicCfg = $derived(config.value?.music ?? null);
  const pauseForBreak = $derived(session.phase === "break" && (musicCfg?.pauseDuringBreak ?? false));
  /** Focus extended through a meeting: silent until it ends (the engine pauses, then resumes in place). */
  const mutedForMeeting = $derived(session.snapshot.musicMuted);
  /** The running session holds the music silent now (Pauza, the break with `pauseDuringBreak`, the meeting mute). */
  const sessionHold = $derived(session.phase !== "idle" && (session.paused || pauseForBreak || mutedForMeeting));
  const shouldPlay = $derived(music.wanted && (!sessionHold || music.override));
  /** Idle and nobody started the music: the dock shows the idle hint. */
  const dormant = $derived(session.isIdle && music.intent === "off");

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

  // Rebuild the playlist when the source definition changes, or when the session starts the music (fresh shuffle) or
  // stops it (`music.playlistEpoch`). Music the user started is never rebuilt by Start or Stop.
  const playlistKey = $derived(
    musicCfg
      ? JSON.stringify([
          music.playlistEpoch,
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
  /** The current item plays in the separate YouTube window (with or without a session). */
  const inWindow = $derived(view.window);
  const canNext = $derived(view.count > 1 && !dormant);
  /** Play/pause shows Pause while the music is meant to play (Play while held silent or after every item failed). */
  const musicOn = $derived(shouldPlay && view.status !== "error");
  const canToggle = $derived(musicCfg !== null && musicCfg.source !== "none" && view.count > 0);
  const volumePct = $derived(Math.round((musicCfg?.volume ?? 0) * 100));

  const sourceText = $derived(musicCfg ? describeSource(musicCfg, knownTrackCount) : "");

  const title = $derived.by(() => {
    if (!musicCfg) return strings.player.loading;
    if (musicCfg.source === "none") return strings.player.silence;
    if (dormant) return strings.player.idle;
    if (view.count === 0) return strings.player.sourceNotReady;
    if (inWindow && !view.windowClosed && mediaWindow.title) return mediaWindow.title;
    return view.title ?? sourceText;
  });

  const subtitle = $derived.by(() => {
    if (!musicCfg) return "";
    if (musicCfg.source === "none") return strings.music.noneHint;
    if (dormant || view.count === 0) return sourceText;
    if (inWindow && view.windowClosed) return strings.player.windowClosed;
    switch (view.status) {
      case "loading":
        return strings.player.loading;
      case "paused":
      case "stopped":
        if (music.intent === "userPaused") return strings.player.pausedBySession;
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
      music.audible = false;
    };
  });

  // The music actually plays (the beta ambience follows it, ADR-024).
  $effect(() => {
    music.audible = view.status === "playing";
  });

  let mixerButton = $state<HTMLButtonElement | null>(null);
  /** Some of the user's ambience layers sound now (the button shows it). */
  const ambienceOn = $derived(ambienceEngine.sample === null && ambienceEngine.sounding.length > 0);

  // The beta went off: the mixer closes with it.
  $effect(() => {
    if (!ambience.betaOn && ambience.mixerOpen) untrack(() => (ambience.mixerOpen = false));
  });

  /** Session the dock saw last (undefined before the first run). */
  let seenSessionId: string | null | undefined;
  /** What the engine was told last (`setPlaying`): whether the music was meant to play right before a Stop. */
  let audible = false;
  /** `music.playlistEpoch` of the playlist the engine has (-1 before the first one). */
  let builtEpoch = -1;

  // Start / Stop → who wants the music on (`MusicIntent`). A pre-effect, so it runs before the effects below apply
  // `shouldPlay` and the playlist: a Stop never plays held music for a moment before the rule settles it. A new session
  // without an idle snapshot in between counts as a Stop and a Start.
  $effect.pre(() => {
    const id = session.snapshot.sessionId;
    untrack(() => {
      if (id === seenSessionId) return;
      const before = seenSessionId;
      seenSessionId = id;
      if (before) music.sessionEnded(audible);
      // The user's music that plays nothing (every item failed, or its YouTube window was closed) counts as stopped.
      if (id) music.sessionStarted(view.status === "error" || view.windowClosed);
    });
  });

  // A hold that ends also ends a play pressed during it (the next hold silences the music again).
  $effect.pre(() => {
    if (!sessionHold) untrack(() => music.holdEnded());
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
    const rebuild = () => {
      const m = config.value?.music;
      if (!m) return;
      builtEpoch = music.playlistEpoch;
      // One call: on Stop the new (idle) playlist must not load the first item before pausing it.
      eng.setItems(buildPlaylist(m, sourceTracks), shouldPlay);
    };
    // A source edit while the music plays (the picker is on the idle page, where music can play now) waits until the
    // edits stop: typing a link or a stream address would otherwise restart the music, or try half an address, on
    // every key. Start and Stop (a new epoch) and edits while silent apply at once.
    if (untrack(() => shouldPlay && music.playlistEpoch === builtEpoch)) {
      const timer = setTimeout(() => untrack(rebuild), SOURCE_EDIT_DELAY_MS);
      return () => clearTimeout(timer);
    }
    untrack(rebuild);
  });

  $effect(() => {
    const play = shouldPlay;
    const eng = engine;
    audible = play;
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

  /** Play/pause in the dock (a click, so WebAudio is unlocked inside the user gesture). */
  function togglePlay() {
    if (musicOn) {
      music.pause();
      return;
    }
    unlockAudio();
    // Already wanted and not held, so the engine halted on errors: try the current item again.
    const halted = shouldPlay;
    music.play(sessionHold);
    if (halted) engine?.retry();
  }

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
        onclick={togglePlay}
        disabled={!canToggle}
        aria-label={musicOn ? strings.player.pause : strings.player.play}
        title={musicOn ? strings.player.pause : strings.player.play}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
          {#if musicOn}
            <rect x="6" y="5" width="4" height="14" rx="1.2" fill="currentColor" />
            <rect x="14" y="5" width="4" height="14" rx="1.2" fill="currentColor" />
          {:else}
            <path d="M7 4.5v15l12.5-7.5z" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" />
          {/if}
        </svg>
      </button>
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
      {#if ambience.betaOn}
        <button
          class="btn btn-sm ambience-btn"
          class:on={ambienceOn}
          type="button"
          bind:this={mixerButton}
          aria-expanded={ambience.mixerOpen}
          aria-controls="ambience-mixer"
          title={strings.collection.mixer.open}
          onclick={() => (ambience.mixerOpen = !ambience.mixerOpen)}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <path
              d="M3 9c2-2.5 4-2.5 6 0s4 2.5 6 0 4-2.5 6 0M3 15c2-2.5 4-2.5 6 0s4 2.5 6 0 4-2.5 6 0"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          </svg>
          <span class="ambience-label">{strings.collection.mixer.open}</span>
        </button>
      {/if}
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

  {#if ambience.betaOn}
    <AmbienceMixer id="ambience-mixer" anchor={mixerButton} />
  {/if}
</section>
<AmbienceHost />

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

  /* The ambience mixer button (beta): the label hides on a narrow window (the icon and title stay). */
  .ambience-btn.on {
    border-color: var(--accent);
    color: var(--accent-strong);
  }

  @media (max-width: 760px) {
    .ambience-label {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }
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
