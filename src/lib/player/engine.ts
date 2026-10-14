// Music engine: one playlist, four backends (YouTube iframe, separate YouTube window, <audio>, WebAudio noise).
// The dock decides *whether* music should play (`setPlaying`: who started it and the session's holds, `MusicIntent` in
// state/music.svelte.ts); the engine decides *what* plays and where: YouTube goes
// to the embedded iframe, or to the separate window when embedding is refused or the user is signed in to YouTube (ADR-021).
import { errorText, strings } from "$lib/strings";
import type { MediaWindowStatus } from "$lib/types";
import { isEmbedBlocked, rememberEmbedBlocked } from "./embedBlocks";
import { HtmlAudioBackend, type AudioFailure } from "./htmlAudio";
import { MediaWindowBackend } from "./mediaWindow";
import { NoiseBackend } from "./noise";
import type { PlayItem } from "./sources";
import { EMBED_OWNER_REFUSED_CODES, EMBED_REFUSED_CODES, parseHttpUrl, samePage, YouTubeBackend } from "./youtube";

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
 * `/playlist?list=X` (YouTube or YouTube Music) only lists the videos; `www.youtube.com/playlist?list=X&playnext=1`
 * plays the playlist from its first video (as `/watch?v=…&list=X`). `/watch?list=X` without a video no longer does:
 * YouTube answers it with a "YouTube is not currently available on this device" clip. Any other page is unchanged.
 */
function playlistWatchPage(pageUrl: string): string {
  const url = parseHttpUrl(pageUrl);
  const list = url?.searchParams.get("list") ?? null;
  if (url?.pathname !== "/playlist" || list === null || !PLAYLIST_ID.test(list)) return pageUrl;
  return `https://www.youtube.com/playlist?${new URLSearchParams({ list, playnext: "1" }).toString()}`;
}

/** Only www.youtube.com pages carry the video in their title (YouTube Music always says "YouTube Music"). */
function titledPage(pageUrl: string): boolean {
  return parseHttpUrl(pageUrl)?.hostname === "www.youtube.com";
}

/**
 * Queue page of the YouTube window: YouTube answers `watch_videos?video_ids=A,B,…` with a temporary playlist of exactly
 * those videos in that order (`/watch?v=A&list=TLGG…`) and plays through it by itself.
 */
const QUEUE_PAGE = "https://www.youtube.com/watch_videos?video_ids=";
/** Most videos one queue page takes (YouTube keeps only the first 50 ids). */
const MAX_QUEUE = 50;
const VIDEO_ID = /^[A-Za-z0-9_-]{11}$/;
/**
 * A window page that has not reported a real title by then counts as unplayable, like a refused embed: YouTube's error
 * page ("Video unavailable", also when the first id of a queue page no longer exists) has no title of its own.
 */
const WINDOW_TITLE_TIMEOUT_MS = 20_000;

/** A queue page without the fragment that made the window load it again (`openWindow`); other pages unchanged. */
function unmarked(url: string | null): string | null {
  return url !== null && url.startsWith(QUEUE_PAGE) ? url.replace(/#.*$/, "") : url;
}

/** Video id of a YouTube item that is one video (not a playlist, mix or channel page), else null. */
function singleVideoId(item: PlayItem | undefined): string | null {
  if (item?.kind !== "youtube") return null;
  const id = item.target?.videoId ?? null;
  if (id === null || !VIDEO_ID.test(id)) return null;
  const page = parseHttpUrl(item.pageUrl);
  return page !== null && page.pathname === "/watch" && !page.searchParams.has("list") ? id : null;
}

/**
 * What the YouTube window plays, followed through its page title (the only thing the window reports besides open and
 * closed): each new title is the page's next video. The window loops its page by itself (media_window.rs).
 */
interface WindowFollow {
  /** Page the window was asked to open. */
  url: string;
  /** Index of the item the page starts with. */
  start: number;
  /** Videos of a queue page (the items from `start` on), 0 for any other page. */
  queue: number;
  /** First and latest title seen on the page. */
  first: string | null;
  last: string | null;
  /** Title changes since the first title. */
  steps: number;
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
  /** The YouTube window's page and how far it got (null when the window plays nothing for us). */
  private follow: WindowFollow | null = null;
  /** Waits for the first title of a newly opened window page (WINDOW_TITLE_TIMEOUT_MS). */
  private titleWatch: ReturnType<typeof setTimeout> | null = null;
  /** Latest `media://window` status. */
  private windowStatus: MediaWindowStatus | null = null;
  /** Fragments that made the window load a queue page it already showed (`openWindow`). */
  private reloads = 0;
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
    // so the next session continues without reloading it; otherwise it is closed. A queue page is always closed:
    // it has moved on through the list, and the next session starts the (maybe reshuffled) list from its start.
    const keep =
      windowPage !== null &&
      !windowPage.startsWith(QUEUE_PAGE) &&
      items.some((_, index) => this.windowPageAt(index) === windowPage);
    if (keep) this.win.pause();
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

  /** User pressed play while the music is wanted but halted (every item failed, autoplay refused): tries this item again. */
  retry(): void {
    if (!this.desired || this.items.length === 0) return;
    this.halted = false;
    this.failures = 0;
    this.loadCurrent();
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
    this.windowStatus = status;
    this.win.applyStatus(status);
    this.followWindow(status);
  }

  /** "Otwórz ponownie" after the user closed the YouTube window. Call from a click handler. */
  reopenWindow(): void {
    if (this.current()?.kind !== "youtube" || !this.isLive("window")) return;
    this.openWindow();
  }

  /** "Pokaż okno": brings the YouTube window to the front. */
  showWindow(): void {
    if (this.isLive("window") && !this.windowClosed) this.win.show();
  }

  destroy(): void {
    this.desired = false;
    this.clearTitleWatch();
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
   * Page for the separate YouTube window for the item at `index`: every YouTube item when signed in to YouTube,
   * otherwise only items that cannot (or may not) use the embedded player. Null = not a window item.
   */
  private windowPageAt(index: number): string | null {
    const item = this.items[index];
    if (item?.kind !== "youtube") return null;
    if (this.signedIn) return this.windowUrlAt(index, item);
    return item.target === null || isEmbedBlocked(item.target) ? item.pageUrl : null;
  }

  /**
   * Page the window opens for the YouTube item at `index`. Signed in (ADR-021), a video followed by more videos opens
   * as one queue page of them all, so the window plays on through the list by itself; playlist links (which the
   * embedded player would have played) open as a playing watch page instead of the playlist overview, which plays nothing.
   */
  private windowUrlAt(index: number, item: YouTubeItem): string {
    if (!this.signedIn) return item.pageUrl;
    const queue = this.queueAt(index);
    if (queue.length > 1) return QUEUE_PAGE + queue.join(",");
    return playlistWatchPage(item.pageUrl);
  }

  /**
   * Videos the window plays as one queue from `index` on: that item and the single videos after it (around the end of
   * the list, never twice), up to the first item of another kind and at most MAX_QUEUE. Empty when the item is not a
   * single video. A list of videos only is one queue that starts at `index`, so its loop is the list's loop.
   * The first video comes back later in the queue only in a row right at its start: `followWindow` reads its title
   * as "the queue started over", so a repeat further on (the same link twice in the list) ends the queue there.
   */
  private queueAt(index: number): string[] {
    const ids: string[] = [];
    const count = this.items.length;
    for (let step = 0; step < count && ids.length < MAX_QUEUE; step++) {
      const id = singleVideoId(this.items[(index + step) % count]);
      if (id === null) break;
      const first = ids[0];
      if (first !== undefined && id === first && ids[ids.length - 1] !== first) break;
      ids.push(id);
    }
    return ids;
  }

  private advance(): void {
    if (this.items.length === 0) return;
    this.advanceTo((this.index + 1) % this.items.length);
  }

  /** Moves on to the item at `target` (the next one, or the first one after a queue the window went through). */
  private advanceTo(target: number): void {
    const wasLoaded = this.loaded;
    this.index = target;
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
    if (this.windowPageAt(this.index) !== null && this.win.page !== null) {
      this.openWindow();
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
    if (this.windowPageAt(this.index) !== null) {
      // Signed in, or known to refuse embedding (or not embeddable at all): straight to the window, no failing embed.
      this.openWindow();
      return;
    }
    this.win.release();
    this.loaded = this.index;
    this.via = item.kind;
    this.emit({ status: "loading" });
    this.applyVolumeAll();
    switch (item.kind) {
      case "youtube":
        // `windowPageAt` sent every item without an embeddable target to the window above.
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

  /** Plays the current (YouTube) item in the separate YouTube window (opens it, navigates it, or just resumes it). */
  private openWindow(): void {
    const item = this.current();
    if (item?.kind !== "youtube") return;
    let url = this.windowUrlAt(this.index, item);
    // The same page in the same window continues where it is (no reload), so it keeps its progress. A page kept
    // paused over Stop may sit at another index of the next (reshuffled) list: it now stands for this item. A queue
    // page continues only while it is at this item (Stop closes queue pages anyway).
    const follow = this.follow;
    const inWindow = samePage(unmarked(this.win.page), url);
    const continues =
      follow !== null &&
      inWindow &&
      samePage(unmarked(follow.url), url) &&
      (follow.queue === 0 || this.queueIndex(follow) === this.index);
    if (continues) {
      if (follow.queue === 0) follow.start = this.index;
      url = follow.url;
    } else {
      // A queue page the window shows at another of its videos (Next from the last video wraps to the first): the
      // same address would only resume it, so a fragment (never sent to YouTube) makes the window load it again.
      const reload = inWindow && url.startsWith(QUEUE_PAGE);
      if (reload) url = `${url}#${++this.reloads}`;
      const queue = url.startsWith(QUEUE_PAGE) ? this.queueAt(this.index).length : 0;
      // A page the window already shows is not reloaded, so its title will not come again: start from the current one.
      const shown = this.windowStatus;
      const title = !reload && inWindow && shown?.open === true && samePage(shown.url, url) ? shown.title : null;
      const fresh: WindowFollow = { url, start: this.index, queue, first: title, last: title, steps: 0 };
      this.follow = fresh;
      if (title === null && titledPage(url)) this.watchTitle(fresh);
      else this.clearTitleWatch();
    }
    this.loaded = this.index;
    this.via = "window";
    this.windowClosed = false;
    this.emit({ status: "loading" });
    this.win.setVolume(this.volume);
    this.win.load(url, this.desired);
  }

  /**
   * Follows the YouTube window through its page by the page title (ADR-021): the window loops its page by itself and
   * reports nothing but open/closed and the title. In a queue page every new title is the next video, so the dock shows
   * the right "n of m" and Next continues from there; a queue that is only part of the list (videos mixed with other
   * links, or more than MAX_QUEUE videos) moves on to the next item once the window went around it, and so does a
   * playlist page or a lone video. "Went around" is the first title coming back, or the moment without a title the page
   * shows when it starts over (the only sign for a page whose title never changes).
   */
  private followWindow(status: MediaWindowStatus): void {
    const follow = this.follow;
    if (follow === null || !status.open || this.windowClosed || !this.isLive("window")) return;
    if (!samePage(status.url, follow.url)) return;
    const title = status.title;
    if (title === null) {
      // A page that had a title loses it only for a moment when it starts over (media_window.rs END_TITLE): the
      // only sign of the end of a page whose title never changes (a lone video, a playlist of one video).
      if (follow.last !== null) this.wentAround(follow);
      return;
    }
    if (follow.last === null) {
      // A real title: the page plays (an error page has none, see `watchTitle`).
      follow.first = title;
      follow.last = title;
      this.failures = 0;
      return;
    }
    if (title === follow.last) return;
    follow.last = title;
    follow.steps++;
    if (title === follow.first) {
      // The first video again: the page started over (also after it went back to its address), whatever came between.
      this.wentAround(follow);
      return;
    }
    if (follow.queue > 1) {
      if (follow.queue < this.items.length && follow.steps >= follow.queue) {
        this.advanceTo((follow.start + follow.queue) % this.items.length);
        return;
      }
      this.showQueueIndex(follow);
    }
  }

  /**
   * The window's page played through and starts over (both signs may come for the same end, so this is idempotent):
   * a page that is only part of the list moves on to the item after it, a queue of the whole list just starts over.
   */
  private wentAround(follow: WindowFollow): void {
    const count = this.items.length;
    if (follow.queue > 1) {
      follow.steps = Math.max(1, Math.ceil(follow.steps / follow.queue)) * follow.queue;
      if (follow.queue < count) this.advanceTo((follow.start + follow.queue) % count);
      else this.showQueueIndex(follow);
      return;
    }
    if (count > 1) this.advanceTo((follow.start + 1) % count);
  }

  /** Item a queue page is at (by the title changes since its first video). */
  private queueIndex(follow: WindowFollow): number {
    return (follow.start + (follow.steps % Math.max(1, follow.queue))) % Math.max(1, this.items.length);
  }

  private showQueueIndex(follow: WindowFollow): void {
    const index = this.queueIndex(follow);
    this.index = index;
    this.loaded = index;
    this.emit({});
  }

  /**
   * The window reports no playback events, but every playable YouTube page has a title. A page that shows none within
   * WINDOW_TITLE_TIMEOUT_MS (YouTube's "Video unavailable" page, e.g. a deleted video or the first id of a queue page
   * that no longer exists) counts as unplayable and moves on, as the embedded player does on an error. While the
   * music is paused it waits; a window the user closed is left alone.
   */
  private watchTitle(follow: WindowFollow): void {
    this.clearTitleWatch();
    this.titleWatch = setTimeout(() => {
      this.titleWatch = null;
      if (this.follow !== follow || follow.last !== null) return;
      if (!this.isLive("window") || this.windowClosed || this.win.page === null) return;
      if (!this.desired) {
        this.watchTitle(follow);
        return;
      }
      this.handleFailure("window", strings.errors.unplayable);
    }, WINDOW_TITLE_TIMEOUT_MS);
  }

  private clearTitleWatch(): void {
    if (this.titleWatch !== null) clearTimeout(this.titleWatch);
    this.titleWatch = null;
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
    if (this.via === "window" && item.kind === "youtube") this.openWindow();
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
      this.openWindow();
      return;
    }
    this.handleFailure("youtube", strings.errors.unplayable);
  }

  private handleWindowOpened(): void {
    if (!this.isLive("window") || this.windowClosed) return;
    // On www.youtube.com `failures` is reset by the page's first title (`followWindow`), not here: the window also
    // opens YouTube's "Video unavailable" page, and a list of those must end in "nothing playable", not skip forever.
    if (this.follow !== null && !titledPage(this.follow.url)) this.failures = 0;
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
