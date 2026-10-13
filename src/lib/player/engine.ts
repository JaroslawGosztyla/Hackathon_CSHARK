// Music engine: one playlist, four backends (YouTube iframe, separate YouTube window, <audio>, WebAudio noise).
// The session decides *whether* music should play (`setPlaying`); the engine decides *what* plays and where: YouTube goes
// to the embedded iframe, or to the separate window when embedding is refused or the user is signed in to YouTube (ADR-021).
import { errorText, strings } from "$lib/strings";
import type { MediaWindowStatus } from "$lib/types";
import { isEmbedBlocked, rememberEmbedBlocked } from "./embedBlocks";
import { HtmlAudioBackend, type AudioFailure } from "./htmlAudio";
import { MediaWindowBackend } from "./mediaWindow";
import { NoiseBackend } from "./noise";
import type { PlayItem } from "./sources";
import { EMBED_OWNER_REFUSED_CODES, EMBED_REFUSED_CODES, parseHttpUrl, YouTubeBackend } from "./youtube";

export type EngineStatus = "stopped" | "loading" | "playing" | "paused" | "error";

/** Where the current item plays. "window" = the separate YouTube window (embedding refused or impossible). */
type Backend = "youtube" | "window" | "audio" | "noise";

type YouTubeItem = Extract<PlayItem, { kind: "youtube" }>;

export interface EngineView {
  status: EngineStatus;
  kind: PlayItem["kind"] | null;
  title: string | null;
  /** 0-based index of the current item. */
  index: number;
  count: number;
  /** The current item plays in the separate YouTube window. */
  window: boolean;
  /** Page the window was asked to open (window mode only). */
  windowUrl: string | null;
  /** The user closed that window; music waits for "Otwórz ponownie" (or the next item / session). */
  windowClosed: boolean;
}

export interface EngineCallbacks {
  onChange(view: EngineView): void;
  onError(text: string): void;
}

export const EMPTY_VIEW: EngineView = {
  status: "stopped",
  kind: null,
  title: null,
  index: 0,
  count: 0,
  window: false,
  windowUrl: null,
  windowClosed: false,
};

/** Short, readable form of a page URL for the now-playing line. */
function displayUrl(url: string): string {
  return url.replace(/^https:\/\/(www\.)?/, "");
}

/** Playlist ids as `youtube.ts` accepts them. */
const PLAYLIST_ID = /^[A-Za-z0-9_-]{2,80}$/;

/**
 * `/playlist?list=X` (YouTube or YouTube Music) only lists the videos; `www.youtube.com/watch?list=X` plays the playlist
 * from its first video. Any other page is returned unchanged.
 */
function playlistWatchPage(pageUrl: string): string {
  const url = parseHttpUrl(pageUrl);
  const list = url?.searchParams.get("list") ?? null;
  if (url?.pathname !== "/playlist" || list === null || !PLAYLIST_ID.test(list)) return pageUrl;
  return `https://www.youtube.com/watch?${new URLSearchParams({ list }).toString()}`;
}

export class MusicEngine {
  private items: PlayItem[] = [];
  private index = 0;
  /** What the session wants. */
  private desired = false;
  /** Every item failed in a row: wait for the user (Next) or the next pause/resume. */
  private halted = false;
  /** Index of the item loaded into a backend, null when nothing is loaded. */
  private loaded: number | null = null;
  /** Backend holding the loaded item. */
  private via: Backend | null = null;
  /** The user closed the YouTube window of the loaded item. */
  private windowClosed = false;
  /**
   * Signed in to YouTube (ADR-021): every YouTube item plays in the YouTube window, the only place where the account
   * (and Premium, no ads) applies. The embedded player runs on the app's origin, where youtube.com cookies are third-party.
   */
  private signedIn = false;
  private failures = 0;
  private volume = 0.6;
  private view: EngineView = { ...EMPTY_VIEW };

  private readonly yt: YouTubeBackend;
  private readonly win: MediaWindowBackend;
  private readonly audio: HtmlAudioBackend;
  private readonly noise: NoiseBackend;
  private readonly callbacks: EngineCallbacks;

  constructor(youtubeHost: HTMLElement, callbacks: EngineCallbacks) {
    this.callbacks = callbacks;
    this.yt = new YouTubeBackend(youtubeHost, {
      onPlaying: (title) => this.handlePlaying("youtube", title),
      onEnded: () => this.handleEnded("youtube"),
      onError: (code) => this.handleYouTubeError(code),
    });
    this.win = new MediaWindowBackend({
      onOpened: () => this.handleWindowOpened(),
      onClosedByUser: () => this.handleWindowClosed(),
      onOpenFailed: (error) => this.handleFailure("window", errorText(error)),
      onControlFailed: (error) => this.callbacks.onError(errorText(error)),
    });
    this.audio = new HtmlAudioBackend({
      onPlaying: () => this.handlePlaying("audio", null),
      onEnded: () => this.handleEnded("audio"),
      onError: (kind: AudioFailure) =>
        kind === "autoplay" ? this.haltWith(strings.errors.autoplayBlocked) : this.handleFailure("audio", strings.errors.unplayable),
    });
    this.noise = new NoiseBackend({
      onPlaying: () => this.handlePlaying("noise", null),
      onError: () => this.handleFailure("noise", strings.errors.allUnplayable),
    });
  }

  /**
   * Replaces the playlist (source changed, a new session started, or the session stopped) and sets the
   * wanted play state in the same step, so a Stop never reloads the first item just to pause it again.
   */
  setItems(items: PlayItem[], play: boolean): void {
    const windowPage = this.win.page;
    this.unloadEmbedded();
    this.items = items;
    this.index = 0;
    this.failures = 0;
    this.halted = false;
    this.desired = play;
    this.emit({ status: "stopped" });
    if (play) {
      // Loading decides about the window: same page → play, another page → navigate, no window item → close.
      this.loadCurrent();
      return;
    }
    // Stop (or a source change while idle): the window is only paused while the playlist still uses that page,
    // so the next session continues without reloading it; otherwise it is closed.
    if (windowPage !== null && items.some((item) => this.windowPageOf(item) === windowPage)) this.win.pause();
    else this.win.release();
  }

  setPlaying(play: boolean): void {
    if (play === this.desired) return;
    this.desired = play;
    if (play) {
      this.halted = false;
      this.failures = 0;
      const item = this.current();
      if (!item) return;
      if (this.loaded === this.index) {
        this.resumeCurrent(item);
      } else {
        this.loadCurrent();
      }
    } else if (this.loaded !== null) {
      this.pauseCurrent();
      this.emit({ status: this.windowClosed ? "stopped" : "paused" });
    }
  }

  /** User pressed "Następny": always tries again, even after every item failed. */
  next(): void {
    if (this.items.length === 0) return;
    this.halted = false;
    this.failures = 0;
    this.advance();
  }

  setVolume(volume: number): void {
    this.volume = Math.min(1, Math.max(0, Number.isFinite(volume) ? volume : 0));
    this.yt.setVolume(this.volume);
    this.win.setVolume(this.volume);
    this.audio.setVolume(this.volume);
    this.noise.setVolume(this.volume);
  }

  /**
   * YouTube sign-in state (`youtube://account`, known and signed in). Signing in moves a YouTube item that the embedded
   * player holds to the YouTube window right away, so Premium applies without waiting for the next item (the shell reloads
   * an open window by itself). Signing out changes only where the next item loads: the current one keeps playing.
   */
  setSignedIn(signedIn: boolean): void {
    if (signedIn === this.signedIn) return;
    this.signedIn = signedIn;
    if (!signedIn || this.current()?.kind !== "youtube" || !this.isLive("youtube")) return;
    if (this.desired) {
      this.loadCurrent();
      return;
    }
    // Paused: drop the embed now; resuming loads the item into the window.
    this.unloadEmbedded();
    this.emit({ status: "stopped" });
  }

  /** New status of the separate YouTube window (from `media://window`). */
  setWindowStatus(status: MediaWindowStatus): void {
    this.win.applyStatus(status);
  }

  /** "Otwórz ponownie" after the user closed the YouTube window. Call from a click handler. */
  reopenWindow(): void {
    const item = this.current();
    if (item?.kind !== "youtube" || !this.isLive("window")) return;
    this.openWindow(item);
  }

  /** "Pokaż okno": brings the YouTube window to the front. */
  showWindow(): void {
    if (this.isLive("window") && !this.windowClosed) this.win.show();
  }

  destroy(): void {
    this.desired = false;
    this.yt.destroy();
    this.win.release();
    this.audio.destroy();
    this.noise.destroy();
    this.loaded = null;
    this.via = null;
  }

  private current(): PlayItem | undefined {
    return this.items[this.index];
  }

  /**
   * Page for the separate YouTube window: every YouTube item when signed in to YouTube, otherwise only items that cannot
   * (or may not) use the embedded player.
   */
  private windowPageOf(item: PlayItem): string | null {
    if (item.kind !== "youtube") return null;
    if (this.signedIn) return this.windowUrlOf(item);
    return item.target === null || isEmbedBlocked(item.target) ? item.pageUrl : null;
  }

  /**
   * Page the window opens for a YouTube item. Signed in, playlist links (which the embedded player would have played)
   * open as a playing watch page instead of the playlist overview, which plays nothing (ADR-021).
   */
  private windowUrlOf(item: YouTubeItem): string {
    return this.signedIn ? playlistWatchPage(item.pageUrl) : item.pageUrl;
  }

  private advance(): void {
    if (this.items.length === 0) return;
    const wasLoaded = this.loaded;
    this.index = (this.index + 1) % this.items.length;
    const item = this.current();
    // A single-item playlist (or a looping stream) restarts in place without reloading the backend.
    if (wasLoaded === this.index && item && this.desired) {
      this.restartCurrent(item);
      return;
    }
    this.unloadEmbedded();
    if (this.desired) {
      this.loadCurrent();
      return;
    }
    // Paused: a window page replaces the window's page (still paused) only if the window is ours already,
    // so nothing new pops up while paused. Anything else loads on resume.
    if (item?.kind === "youtube" && this.windowPageOf(item) !== null && this.win.page !== null) {
      this.openWindow(item);
      return;
    }
    this.win.release();
    this.emit({ status: "stopped" });
  }

  private loadCurrent(): void {
    const item = this.current();
    if (!item || this.halted) {
      this.emit({ status: "stopped" });
      return;
    }
    this.unloadEmbedded();
    if (item.kind === "youtube" && this.windowPageOf(item) !== null) {
      // Known to refuse embedding (or not embeddable at all): straight to the window, no failing embed first.
      this.openWindow(item);
      return;
    }
    this.win.release();
    this.loaded = this.index;
    this.via = item.kind;
    this.emit({ status: "loading" });
    this.applyVolumeAll();
    switch (item.kind) {
      case "youtube":
        // `windowPageOf` sent every item without an embeddable target to the window above.
        if (!item.target) return;
        this.yt.load(item.target, true).catch(() => {
          if (this.isLive("youtube")) this.haltWith(strings.errors.youtubeApi);
        });
        break;
      case "audio":
        this.audio.load(item.url, true, item.live);
        break;
      case "noise":
        this.noise.load(item.color, true);
        break;
    }
  }

  /** Plays the item in the separate YouTube window (opens it, navigates it, or just resumes it). */
  private openWindow(item: YouTubeItem): void {
    this.loaded = this.index;
    this.via = "window";
    this.windowClosed = false;
    this.emit({ status: "loading" });
    this.win.setVolume(this.volume);
    this.win.load(this.windowUrlOf(item), this.desired);
  }

  private resumeCurrent(item: PlayItem): void {
    if (this.via === "window") {
      // A window the user closed stays closed until "Otwórz ponownie" (no surprise pop-ups on resume).
      if (this.windowClosed) {
        this.emit({ status: "stopped" });
        return;
      }
      this.win.play();
      // No playback events from the window: optimistic, unless the open request is still running.
      if (this.view.status !== "loading") this.emit({ status: "playing" });
      return;
    }
    this.emit({ status: "loading", title: this.view.title });
    if (item.kind === "youtube") this.yt.play();
    else if (item.kind === "audio") this.audio.play();
    else this.noise.play();
  }

  private pauseCurrent(): void {
    switch (this.via) {
      case "youtube":
        this.yt.pause();
        break;
      case "window":
        this.win.pause();
        break;
      case "audio":
        this.audio.pause();
        break;
      case "noise":
        this.noise.pause();
        break;
      case null:
        break;
    }
  }

  private restartCurrent(item: PlayItem): void {
    if (this.via === "window" && item.kind === "youtube") this.openWindow(item);
    else if (item.kind === "youtube") this.yt.restart();
    else if (item.kind === "audio") this.audio.restart();
    else this.noise.play();
  }

  /** Unloads the in-app backends (embed, <audio>, noise). The window is closed explicitly (`win.release`). */
  private unloadEmbedded(): void {
    this.yt.unload();
    this.audio.unload();
    this.noise.unload();
    this.loaded = null;
    this.via = null;
    this.windowClosed = false;
  }

  private applyVolumeAll(): void {
    this.setVolume(this.volume);
  }

  /** Events from a backend count only while that backend holds the current, loaded item. */
  private isLive(backend: Backend): boolean {
    return this.loaded === this.index && this.via === backend;
  }

  private handlePlaying(backend: Backend, title: string | null): void {
    if (!this.isLive(backend)) return;
    this.failures = 0;
    this.emit({ status: "playing", title: title ?? this.current()?.label ?? null });
  }

  private handleEnded(backend: Backend): void {
    if (!this.isLive(backend)) return;
    this.advance();
  }

  private handleYouTubeError(code: number): void {
    if (!this.isLive("youtube")) return;
    const item = this.current();
    if (item?.kind === "youtube" && item.target && EMBED_REFUSED_CODES.has(code)) {
      // The embed was refused: no skip, no toast. The same link plays in the YouTube window, and when it was
      // the owner's decision (not our embed's referrer, 153), next time it goes there directly.
      if (EMBED_OWNER_REFUSED_CODES.has(code)) rememberEmbedBlocked(item.target);
      this.yt.unload();
      this.openWindow(item);
      return;
    }
    this.handleFailure("youtube", strings.errors.unplayable);
  }

  private handleWindowOpened(): void {
    if (!this.isLive("window") || this.windowClosed) return;
    this.failures = 0;
    this.emit({ status: this.desired ? "playing" : "paused" });
  }

  private handleWindowClosed(): void {
    if (!this.isLive("window")) return;
    this.windowClosed = true;
    this.emit({ status: "stopped" });
  }

  private handleFailure(backend: Backend, text: string): void {
    if (!this.isLive(backend)) return;
    this.failures++;
    if (this.failures >= this.items.length) {
      this.haltWith(this.items.length > 1 ? strings.errors.allUnplayable : text);
      return;
    }
    this.callbacks.onError(text);
    this.advance();
  }

  private haltWith(text: string): void {
    this.halted = true;
    this.unloadEmbedded();
    this.win.release();
    this.callbacks.onError(text);
    this.emit({ status: "error" });
  }

  private emit(patch: Partial<EngineView>): void {
    const item = this.current();
    const windowItem = this.via === "window" && this.loaded === this.index && item?.kind === "youtube" ? item : null;
    this.view = {
      ...this.view,
      kind: item?.kind ?? null,
      index: this.index,
      count: this.items.length,
      window: windowItem !== null,
      windowUrl: windowItem?.pageUrl ?? null,
      windowClosed: windowItem !== null && this.windowClosed,
      ...patch,
    };
    if (!("title" in patch) && (patch.status === "loading" || patch.status === "stopped")) {
      // In window mode the dock prefers the window's own page title; until it is known, a named preset or the link.
      const generic = windowItem !== null && windowItem.label === strings.player.youtubeFallbackTitle;
      this.view.title = generic ? displayUrl(windowItem.pageUrl) : (item?.label ?? null);
    }
    this.callbacks.onChange({ ...this.view });
  }
}
