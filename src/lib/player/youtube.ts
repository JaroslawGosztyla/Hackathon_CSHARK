// YouTube support: URL parsing and a small backend around an embedded player <iframe>.
//
// Security: no third-party script ever runs in the app document (it has access to the Tauri IPC).
// We do NOT load https://www.youtube.com/iframe_api; instead we create the cross-origin <iframe> ourselves
// and speak the embed player's postMessage protocol directly (the same JSON messages the official API sends).
// Incoming messages are accepted only from the YouTube origin AND from our own iframe's window.
//
// The iframe is created by us so `allow` and `referrerpolicy` are set before its first navigation
// (embeds without a referrer fail with error 153).

export interface YouTubeTarget {
  videoId: string | null;
  listId: string | null;
}

const VIDEO_ID = /^[A-Za-z0-9_-]{11}$/;
const LIST_ID = /^[A-Za-z0-9_-]{2,80}$/;
const YOUTUBE_HOSTS = new Set([
  "youtube.com",
  "m.youtube.com",
  "music.youtube.com",
  "youtube-nocookie.com",
  "youtu.be",
]);
const VIDEO_PATH_PREFIXES = new Set(["live", "shorts", "embed", "v", "e"]);
/** `/embed/live_stream?channel=<id>`: the channel's current live stream in the embedded player. */
const LIVE_STREAM = "live_stream";
const CHANNEL_ID = /^UC[A-Za-z0-9_-]{22}$/;

/** Parses an http(s) URL; a missing scheme is treated as https. */
export function parseHttpUrl(raw: string): URL | null {
  const text = raw.trim();
  if (!text || /\s/.test(text)) return null;
  try {
    const url = new URL(/^[a-z][a-z0-9+.-]*:/i.test(text) ? text : `https://${text}`);
    return url.protocol === "http:" || url.protocol === "https:" ? url : null;
  } catch {
    return null;
  }
}

/**
 * youtube.com/watch?v=, youtu.be/ID, /live/ID, /shorts/ID, /embed/ID, music.youtube.com/watch, `list=` playlists.
 * Returns null for anything else.
 */
export function parseYouTube(raw: string): YouTubeTarget | null {
  const url = parseHttpUrl(raw);
  if (!url) return null;
  const host = url.hostname.toLowerCase().replace(/^www\./, "");
  if (!YOUTUBE_HOSTS.has(host)) return null;

  const segments = url.pathname.split("/").filter(Boolean);
  let videoId: string | null = null;
  if (host === "youtu.be") {
    videoId = segments[0] ?? null;
  } else {
    const [first, second] = segments;
    if (first === "watch") videoId = url.searchParams.get("v");
    // `/embed/live_stream?channel=…` is a channel's live embed, not a video id (it plays in the YouTube window).
    else if (first && second && VIDEO_PATH_PREFIXES.has(first) && second !== "videoseries" && second !== LIVE_STREAM) {
      videoId = second;
    }
  }
  if (videoId !== null && !VIDEO_ID.test(videoId)) videoId = null;

  let listId = url.searchParams.get("list");
  if (listId !== null && !LIST_ID.test(listId)) listId = null;
  // Auto-generated mixes ("RD…") do not load as playlists in the embedded player; play the video alone.
  if (listId?.startsWith("RD") && videoId) listId = null;

  if (!videoId && !listId) return null;
  return { videoId, listId };
}

export function sameTarget(a: YouTubeTarget | null, b: YouTubeTarget | null): boolean {
  return a !== null && b !== null && a.videoId === b.videoId && a.listId === b.listId;
}

// ---------------------------------------------------------------------------
// Separate YouTube window (a top-level page in its own Tauri window, for links the embed cannot play).
// ---------------------------------------------------------------------------

/** YouTube error codes meaning "this video may not play in an embedded player": 101/150 owner, 153 referrer. */
export const EMBED_REFUSED_CODES: ReadonlySet<number> = new Set([101, 150, 153]);
/**
 * The subset that is the owner's decision about the video, so it is worth remembering. 153 (missing referrer)
 * is a problem of our embed, not of the video: it falls back to the window for that play only.
 */
export const EMBED_OWNER_REFUSED_CODES: ReadonlySet<number> = new Set([101, 150]);

/** Path prefixes that only exist for the embedded player (opened top-level they just say "unavailable"). */
const EMBED_ONLY_PREFIXES = new Set(["embed", "v", "e"]);
/** Channel pages: `/channel/<id>`, `/c/<name>`, `/user/<name>` (+ `/live`, `/streams`, …). */
const CHANNEL_PREFIXES = new Set(["channel", "c", "user"]);

function bareHost(url: URL): string {
  return url.hostname.toLowerCase().replace(/^www\./, "");
}

/** Channel id of an `/embed/live_stream?channel=<id>` link, else null. */
function liveStreamChannel(url: URL): string | null {
  const [first, second] = url.pathname.split("/").filter(Boolean);
  if (first !== "embed" || second !== LIVE_STREAM) return null;
  const channel = url.searchParams.get("channel");
  return channel !== null && CHANNEL_ID.test(channel) ? channel : null;
}

/**
 * Channel / handle pages the embedded player cannot load at all, but a normal YouTube tab can:
 * `/@handle`, `/@handle/live`, `/channel/<id>`, `/channel/<id>/live`, `/c/<name>`, `/user/<name>` on youtube.com
 * or music.youtube.com (`/@handle/live` and `/channel/<id>/live` jump to the channel's current live stream),
 * plus the channel live embed `/embed/live_stream?channel=<id>` (opened as `/channel/<id>/live`).
 */
export function isYouTubeChannelPage(raw: string): boolean {
  const url = parseHttpUrl(raw);
  if (!url) return false;
  const host = bareHost(url);
  if (host === "youtube.com" || host === "youtube-nocookie.com") {
    if (liveStreamChannel(url) !== null) return true;
  }
  if (host !== "youtube.com" && host !== "m.youtube.com" && host !== "music.youtube.com") return false;
  const [first, second] = url.pathname.split("/").filter(Boolean);
  if (!first) return false;
  if (first.startsWith("@")) return first.length > 1;
  return CHANNEL_PREFIXES.has(first) && second !== undefined;
}

/**
 * Normal page for an embed-only link: the watch page (keeping the original `list`, so mixes still work,
 * and the `t` start time) or the playlist page.
 */
function pageUrlFor(target: YouTubeTarget, original: URL): string {
  const rawList = original.searchParams.get("list");
  const listId = rawList !== null && LIST_ID.test(rawList) ? rawList : target.listId;
  if (target.videoId) {
    const params = new URLSearchParams({ v: target.videoId });
    if (listId) params.set("list", listId);
    const start = original.searchParams.get("t") ?? original.searchParams.get("start");
    if (start && /^\d{1,6}s?$/.test(start)) params.set("t", start);
    return `${YOUTUBE_ORIGIN}/watch?${params.toString()}`;
  }
  return `${YOUTUBE_ORIGIN}/playlist?${new URLSearchParams({ list: listId ?? "" }).toString()}`;
}

/**
 * The https page the separate YouTube window opens for a link: the link itself on www.youtube.com or
 * music.youtube.com (so mixes, timestamps and YouTube Music keep working), except embed-only forms
 * (`/embed/ID`, youtube-nocookie.com, youtu.be) which become the normal watch/playlist/channel-live page.
 * Null for anything that is not a YouTube page.
 */
export function youTubeWindowUrl(raw: string): string | null {
  const url = parseHttpUrl(raw);
  if (!url) return null;
  const host = bareHost(url);
  const first = url.pathname.split("/").filter(Boolean)[0] ?? "";
  if (host === "youtu.be" || host === "youtube-nocookie.com" || EMBED_ONLY_PREFIXES.has(first)) {
    const channel = liveStreamChannel(url);
    if (channel !== null) return `${YOUTUBE_ORIGIN}/channel/${channel}/live`;
    const target = parseYouTube(raw);
    return target ? pageUrlFor(target, url) : null;
  }
  if (host === "music.youtube.com") url.hostname = "music.youtube.com";
  else if (host === "youtube.com" || host === "m.youtube.com") url.hostname = "www.youtube.com";
  else return null;
  url.protocol = "https:";
  url.port = "";
  url.username = "";
  url.password = "";
  return url.toString();
}

/** Same page, ignoring trivial spelling differences (the shell may normalize the URL it reports). */
export function samePage(a: string | null, b: string | null): boolean {
  if (a === null || b === null) return false;
  if (a === b) return true;
  const ua = parseHttpUrl(a);
  const ub = parseHttpUrl(b);
  return ua !== null && ub !== null && ua.toString() === ub.toString();
}

// ---------------------------------------------------------------------------
// Embed player postMessage protocol (what the official widget API does under the hood).
//   iframe → parent: {"event":"readyToListen"} (optional hint that the embed's handler is installed)
//   parent → iframe: {"event":"listening","id":N,"channel":"widget"} until the player answers,
//                    {"event":"command","func":"playVideo","args":[],"id":N,"channel":"widget"}, …
//   iframe → parent: {"event":"onReady"}, {"event":"infoDelivery","info":{playerState, videoData, playlist, …}},
//                    {"event":"onStateChange","info":1}, {"event":"onError","info":150}, …
// ---------------------------------------------------------------------------

/** The only origin we embed from, post to, and accept messages from. */
const YOUTUBE_ORIGIN = "https://www.youtube.com";

/** Player states as reported by the embed (`playerState` / `onStateChange`). */
const STATE_UNSTARTED = -1;
const STATE_ENDED = 0;
const STATE_PLAYING = 1;

const LISTEN_INTERVAL_MS = 250;
/** No `onReady` from the iframe in this time → the player is considered unavailable (offline, blocked…). */
const READY_TIMEOUT_MS = 15_000;

type PlayerCommand =
  | "playVideo"
  | "pauseVideo"
  | "stopVideo"
  | "seekTo"
  | "setVolume"
  | "mute"
  | "unMute"
  | "nextVideo"
  | "playVideoAt"
  | "loadVideoById"
  | "cueVideoById"
  | "loadPlaylist"
  | "cuePlaylist"
  | "addEventListener";

interface PlayerMessage {
  event: string;
  info: unknown;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

/** Parses one message from the embed. Anything that is not a JSON object with a string `event` is ignored. */
function parsePlayerMessage(data: unknown): PlayerMessage | null {
  let value: unknown = data;
  if (typeof data === "string") {
    // Cheap pre-check: the player always sends a JSON object.
    if (data.length === 0 || data[0] !== "{") return null;
    try {
      value = JSON.parse(data);
    } catch {
      return null;
    }
  }
  const record = asRecord(value);
  const event = record?.["event"];
  if (!record || typeof event !== "string") return null;
  return { event, info: record["info"] };
}

function asFiniteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

let nextWidgetId = 1;

function embedUrl(target: YouTubeTarget, autoplay: boolean, widgetId: number): string {
  const params = new URLSearchParams({
    enablejsapi: "1",
    autoplay: autoplay ? "1" : "0",
    playsinline: "1",
    rel: "0",
    // URLSearchParams percent-encodes the value (same result as encodeURIComponent for an origin).
    origin: window.location.origin,
    widgetid: String(widgetId),
  });
  if (target.listId) {
    params.set("listType", "playlist");
    params.set("list", target.listId);
  }
  const path = target.videoId ?? "videoseries";
  return `${YOUTUBE_ORIGIN}/embed/${encodeURIComponent(path)}?${params.toString()}`;
}

export interface YouTubeEvents {
  onPlaying(title: string | null): void;
  onEnded(): void;
  onError(code: number): void;
}

interface ReadyWaiter {
  generation: number;
  resolve: () => void;
  reject: (error: Error) => void;
}

/** One embedded player reused for every YouTube item (cue/load commands instead of re-creating the iframe). */
export class YouTubeBackend {
  private iframe: HTMLIFrameElement | null = null;
  private widgetId = 0;
  /** The embed answered our `listening` message (stop repeating it). */
  private heard = false;
  /** The embed sent `onReady`: commands are accepted from now on. */
  private ready = false;
  private listenTimer: ReturnType<typeof setInterval> | null = null;
  private readyTimer: ReturnType<typeof setTimeout> | null = null;
  private waiters: ReadyWaiter[] = [];
  private windowListener = false;

  /** Target the iframe has loaded or cued. */
  private applied: YouTubeTarget | null = null;
  /** Target the engine wants (may differ while the player is still booting). */
  private wanted: YouTubeTarget | null = null;
  private wantPlay = false;
  private volume = 0.6;
  private generation = 0;

  // Last known player info (from `infoDelivery` / `onStateChange`).
  private playerState = STATE_UNSTARTED;
  private title: string | null = null;
  private playlist: string[] | null = null;
  private playlistIndex = -1;
  /** PLAYING was seen since the current attempt began; a stale ENDED cannot finish a fresh item. */
  private seenPlaying = false;
  /** An error was already reported for the current attempt (duplicates must not skip two items). */
  private errorReported = false;

  private readonly onWindowMessage = (event: MessageEvent): void => this.handleMessage(event);

  constructor(
    private readonly host: HTMLElement,
    private readonly events: YouTubeEvents,
  ) {}

  /**
   * Loads (and optionally plays) a video/playlist. Resolves once the player accepts commands (or the request was
   * superseded); rejects only when the embedded player does not come up at all (offline, blocked).
   */
  load(target: YouTubeTarget, play: boolean): Promise<void> {
    const generation = ++this.generation;
    this.wanted = target;
    this.wantPlay = play;
    this.seenPlaying = false;
    this.errorReported = false;
    if (!this.iframe) this.create(target, play);
    if (this.ready) {
      this.apply();
      return Promise.resolve();
    }
    return new Promise<void>((resolve, reject) => {
      this.waiters.push({ generation, resolve, reject });
    });
  }

  play(): void {
    this.wantPlay = true;
    this.apply();
  }

  pause(): void {
    this.wantPlay = false;
    this.apply();
  }

  /** Restarts the current item (single video or playlist) from the beginning. */
  restart(): void {
    if (!this.ready || !this.applied) return;
    if (this.applied.listId) this.command("playVideoAt", [0]);
    else {
      this.command("seekTo", [0, true]);
      this.command("playVideo");
    }
  }

  unload(): void {
    this.generation++;
    this.wanted = null;
    this.wantPlay = false;
    if (this.ready) this.command("stopVideo");
  }

  setVolume(volume: number): void {
    this.volume = volume;
    this.applyVolume();
  }

  destroy(): void {
    this.generation++;
    this.wanted = null;
    this.wantPlay = false;
    this.teardown();
    // Pending loads are stale now: settle them quietly.
    this.settleWaiters(null);
  }

  // -------------------------------------------------------------------------
  // iframe lifecycle
  // -------------------------------------------------------------------------

  private create(target: YouTubeTarget, play: boolean): void {
    this.teardown();
    this.widgetId = nextWidgetId++;
    const iframe = document.createElement("iframe");
    // Attributes first: they must be in place before the first navigation.
    iframe.title = "YouTube";
    iframe.allow = "autoplay; encrypted-media";
    iframe.referrerPolicy = "strict-origin-when-cross-origin";
    iframe.className = "yt-frame";
    iframe.addEventListener("load", () => {
      if (this.iframe === iframe && !this.heard) this.sendListening();
    });
    iframe.src = embedUrl(target, play, this.widgetId);

    if (!this.windowListener) {
      window.addEventListener("message", this.onWindowMessage);
      this.windowListener = true;
    }
    this.iframe = iframe;
    this.applied = target;
    this.resetPlayerInfo();
    this.host.replaceChildren(iframe);

    // Like the official API: repeat `listening` until the embed answers (its handler may not be installed yet).
    this.listenTimer = setInterval(() => this.sendListening(), LISTEN_INTERVAL_MS);
    this.readyTimer = setTimeout(() => this.handleReadyTimeout(), READY_TIMEOUT_MS);
  }

  /** Removes the iframe and every timer/listener; the next `load` creates a fresh player. */
  private teardown(): void {
    this.stopListening();
    if (this.readyTimer !== null) {
      clearTimeout(this.readyTimer);
      this.readyTimer = null;
    }
    if (this.windowListener) {
      window.removeEventListener("message", this.onWindowMessage);
      this.windowListener = false;
    }
    this.iframe = null;
    this.ready = false;
    this.heard = false;
    this.applied = null;
    this.resetPlayerInfo();
    this.host.replaceChildren();
  }

  private stopListening(): void {
    if (this.listenTimer !== null) {
      clearInterval(this.listenTimer);
      this.listenTimer = null;
    }
  }

  private handleReadyTimeout(): void {
    this.readyTimer = null;
    if (this.ready) return;
    this.teardown();
    this.settleWaiters(new Error("YouTube player did not respond"));
  }

  /** Resolves stale waiters; rejects the current one only when `error` is given. */
  private settleWaiters(error: Error | null): void {
    const waiters = this.waiters;
    this.waiters = [];
    for (const waiter of waiters) {
      if (error && waiter.generation === this.generation) waiter.reject(error);
      else waiter.resolve();
    }
  }

  private resetPlayerInfo(): void {
    this.playerState = STATE_UNSTARTED;
    this.title = null;
    this.playlist = null;
    this.playlistIndex = -1;
  }

  // -------------------------------------------------------------------------
  // Outgoing messages
  // -------------------------------------------------------------------------

  private post(message: Record<string, unknown>): void {
    const target = this.iframe?.contentWindow;
    if (!target) return;
    try {
      // Explicit target origin: if the frame is anywhere else (about:blank, an error page) nothing is delivered.
      target.postMessage(JSON.stringify({ ...message, id: this.widgetId, channel: "widget" }), YOUTUBE_ORIGIN);
    } catch {
      // A detached/closing frame: nothing to control any more.
    }
  }

  private sendListening(): void {
    this.post({ event: "listening" });
  }

  private command(func: PlayerCommand, args: unknown[] = []): void {
    if (!this.ready) return;
    this.post({ event: "command", func, args });
  }

  // -------------------------------------------------------------------------
  // Incoming messages
  // -------------------------------------------------------------------------

  private handleMessage(event: MessageEvent): void {
    const iframe = this.iframe;
    // Both checks: the right origin AND our own frame (another YouTube frame must not drive this player).
    if (!iframe || event.origin !== YOUTUBE_ORIGIN) return;
    if (event.source === null || event.source !== iframe.contentWindow) return;
    const message = parsePlayerMessage(event.data);
    if (!message) return;

    // The embed announces itself before it has seen our `listening`: answer at once (the interval keeps going).
    if (message.event === "readyToListen") {
      if (!this.heard) this.sendListening();
      return;
    }
    if (!this.heard) {
      this.heard = true;
      this.stopListening();
    }

    switch (message.event) {
      case "onReady":
        this.handleReady();
        break;
      case "initialDelivery":
      case "infoDelivery":
        this.handleInfo(message.info);
        break;
      case "onStateChange": {
        const state = asFiniteNumber(message.info);
        if (state !== null) this.handleState(state);
        break;
      }
      case "onError":
        this.handleError(message.info);
        break;
      default:
        break;
    }
  }

  private handleReady(): void {
    if (this.ready) return;
    this.ready = true;
    if (this.readyTimer !== null) {
      clearTimeout(this.readyTimer);
      this.readyTimer = null;
    }
    // Subscribe once (a second subscription would deliver every event twice).
    this.command("addEventListener", ["onStateChange"]);
    this.command("addEventListener", ["onError"]);
    this.applyVolume();
    this.apply();
    this.settleWaiters(null);
  }

  private handleInfo(info: unknown): void {
    const record = asRecord(info);
    if (!record) return;

    if ("playlist" in record) {
      const list = record["playlist"];
      this.playlist = Array.isArray(list) ? list.filter((v): v is string => typeof v === "string") : null;
    }
    const index = asFiniteNumber(record["playlistIndex"]);
    if (index !== null) this.playlistIndex = index;

    const videoData = asRecord(record["videoData"]);
    if (videoData) {
      const raw = videoData["title"];
      const title = typeof raw === "string" ? raw.trim() || null : null;
      if (title !== this.title) {
        this.title = title;
        // The title often arrives after PLAYING; refresh the now-playing line.
        if (title && this.playerState === STATE_PLAYING && this.wanted) this.events.onPlaying(title);
      }
    }

    const state = asFiniteNumber(record["playerState"]);
    if (state !== null) this.handleState(state);
  }

  private handleState(state: number): void {
    // `infoDelivery` and `onStateChange` both report states: react to transitions only.
    if (state === this.playerState) return;
    this.playerState = state;
    if (!this.wanted) return;
    if (state === STATE_PLAYING) {
      this.seenPlaying = true;
      this.events.onPlaying(this.title);
    } else if (state === STATE_ENDED) {
      if (!this.seenPlaying) return;
      // In a playlist ENDED fires after every video; only the end of the last one finishes the item.
      if (this.applied?.listId && this.playlist) {
        const index = this.playlistIndex;
        if (index >= 0 && index < this.playlist.length - 1) return;
      }
      this.events.onEnded();
    }
  }

  private handleError(info: unknown): void {
    if (!this.wanted || this.errorReported) return;
    this.errorReported = true;
    // Force a real reload if the same target is tried again (playVideo on a failed video does nothing).
    this.applied = null;
    // 2 invalid id, 5 HTML5 player error, 100 not found, 101/150 embedding disabled, 153 missing referrer.
    const code = asFiniteNumber(info) ?? (typeof info === "string" ? Number.parseInt(info, 10) : Number.NaN);
    this.events.onError(Number.isFinite(code) ? code : -1);
  }

  // -------------------------------------------------------------------------
  // Desired state → commands
  // -------------------------------------------------------------------------

  /** Brings the ready player to the wanted target and play state. */
  private apply(): void {
    if (!this.ready) return;
    const target = this.wanted;
    if (!target) {
      this.command("stopVideo");
      return;
    }
    if (!sameTarget(target, this.applied)) {
      this.applied = target;
      this.resetPlayerInfo();
      if (target.listId) {
        const options = { list: target.listId, listType: "playlist", index: 0 };
        this.command(this.wantPlay ? "loadPlaylist" : "cuePlaylist", [options]);
      } else if (target.videoId) {
        this.command(this.wantPlay ? "loadVideoById" : "cueVideoById", [target.videoId]);
      }
      return;
    }
    this.command(this.wantPlay ? "playVideo" : "pauseVideo");
  }

  private applyVolume(): void {
    if (!this.ready) return;
    const v = Math.round(Math.min(1, Math.max(0, Number.isFinite(this.volume) ? this.volume : 0)) * 100);
    this.command("setVolume", [v]);
    if (v > 0) this.command("unMute");
  }
}
