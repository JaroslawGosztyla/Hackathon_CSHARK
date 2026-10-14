// <audio> backend: local folder files (asset protocol URLs), direct audio URLs and internet radio streams.

export type AudioFailure = "media" | "autoplay";

export interface HtmlAudioEvents {
  onPlaying(): void;
  onEnded(): void;
  onError(kind: AudioFailure): void;
}

/** After this long in pause a live stream is reloaded, so it resumes at "now" instead of stale buffer. */
const LIVE_RELOAD_AFTER_MS = 10_000;

export class HtmlAudioBackend {
  private readonly el: HTMLAudioElement;
  private readonly events: HtmlAudioEvents;
  private url: string | null = null;
  private live = false;
  private pausedAt: number | null = null;

  constructor(events: HtmlAudioEvents) {
    this.events = events;
    this.el = new Audio();
    this.el.preload = "auto";
    this.el.addEventListener("playing", () => {
      if (this.url) this.events.onPlaying();
    });
    this.el.addEventListener("ended", () => {
      if (this.url) this.events.onEnded();
    });
    this.el.addEventListener("error", () => {
      if (this.url) this.events.onError("media");
    });
  }

  load(url: string, play: boolean, live: boolean): void {
    this.url = url;
    this.live = live;
    this.pausedAt = null;
    this.el.src = url;
    this.el.load();
    if (play) this.tryPlay();
  }

  play(): void {
    if (!this.url) return;
    if (this.live && this.pausedAt !== null && Date.now() - this.pausedAt > LIVE_RELOAD_AFTER_MS) {
      this.el.src = this.url;
      this.el.load();
    }
    this.pausedAt = null;
    this.tryPlay();
  }

  pause(): void {
    if (!this.url) return;
    this.el.pause();
    this.pausedAt = Date.now();
  }

  /** Restarts the current file from the beginning. */
  restart(): void {
    if (!this.url) return;
    try {
      this.el.currentTime = 0;
    } catch {
      // Live streams are not seekable; just keep playing.
    }
    this.tryPlay();
  }

  unload(): void {
    this.url = null;
    this.pausedAt = null;
    this.el.pause();
    this.el.removeAttribute("src");
    this.el.load();
  }

  setVolume(volume: number): void {
    this.el.volume = Math.min(1, Math.max(0, Number.isFinite(volume) ? volume : 0));
  }

  destroy(): void {
    this.unload();
  }

  private tryPlay(): void {
    this.el.play().catch((e: unknown) => {
      if (!this.url) return;
      const name = e instanceof DOMException ? e.name : "";
      // AbortError: superseded by pause()/load(). NotSupportedError: reported by the `error` event.
      if (name === "NotAllowedError") this.events.onError("autoplay");
    });
  }
}
