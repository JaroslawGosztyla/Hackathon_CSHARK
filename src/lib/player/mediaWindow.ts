// Backend for the separate YouTube window: the shell opens a small Tauri window that loads the YouTube page
// top-level (like a normal browser tab), for links whose owner refuses embedding or that cannot be embedded.
// We only send commands; the window reports open/closed/title through `media://window` (applyStatus).
// The window sends no playback events, so "playing" is optimistic. Instead the page loops what it was opened with
// (media_window.rs) and the engine follows it by its title: signed in, a list of videos is one queue page that YouTube
// plays through by itself (engine.ts, ADR-021).
import { controlMediaWindow, openMediaWindow, toAppError } from "$lib/api";
import type { AppError, MediaAction, MediaWindowStatus } from "$lib/types";
import { samePage } from "./youtube";

export interface MediaWindowEvents {
  /** The window accepted our page (opened or navigated); there are no further playback events. */
  onOpened(): void;
  /** The user closed the window we were using. */
  onClosedByUser(): void;
  /** The shell refused to open the page (bad URL, window could not be created, no shell). */
  onOpenFailed(error: AppError): void;
  /** A play/pause/volume/show command failed. */
  onControlFailed(error: AppError): void;
}

export class MediaWindowBackend {
  /** Page the player keeps in the window; null when the window is not ours (never opened, released, closed). */
  private url: string | null = null;
  private wantPlay = false;
  private volume = 0.6;
  private sentVolume: number | null = null;
  private generation = 0;
  /** `open_media_window` is in flight. */
  private opening = false;
  /** The window reported itself open since our last open request, so a later "closed" can only be the user. */
  private seenOpen = false;
  /** We asked for the close ourselves: the next "closed" status is ours, not the user's. */
  private closing = false;
  private status: MediaWindowStatus | null = null;

  constructor(private readonly events: MediaWindowEvents) {}

  /** Page currently kept in the window for the player (null = the window is not used). */
  get page(): string | null {
    return this.url;
  }

  /** Opens the page in the window (or navigates it); the same page already open just gets play/pause. */
  load(url: string, play: boolean): void {
    const generation = ++this.generation;
    this.wantPlay = play;
    this.closing = false;
    const status = this.status;
    if (!this.opening && this.seenOpen && status?.open && samePage(this.url, url)) {
      // No reload: a live stream continues, a paused video resumes where it was.
      this.control(play ? "play" : "pause");
      this.pushVolume(true);
      queueMicrotask(() => {
        if (generation === this.generation) this.events.onOpened();
      });
      return;
    }
    this.url = url;
    this.opening = true;
    this.seenOpen = false;
    this.sentVolume = null;
    openMediaWindow(url, play).then(
      () => {
        if (generation !== this.generation) {
          // Released while this request was in flight: make sure the window does not stay open on its own.
          if (this.url === null && !this.opening) this.requestClose();
          return;
        }
        this.opening = false;
        if (this.wantPlay !== play) this.control(this.wantPlay ? "play" : "pause");
        this.pushVolume(true);
        this.events.onOpened();
      },
      (e: unknown) => {
        if (generation !== this.generation) return;
        this.opening = false;
        this.url = null;
        this.events.onOpenFailed(toAppError(e));
      },
    );
  }

  play(): void {
    this.wantPlay = true;
    if (this.url !== null && !this.opening) this.control("play");
  }

  pause(): void {
    this.wantPlay = false;
    if (this.url !== null && !this.opening) this.control("pause");
  }

  /** Brings the window to the front (e.g. to accept the YouTube consent page). */
  show(): void {
    if (this.url !== null) this.control("show");
  }

  setVolume(volume: number): void {
    this.volume = volume;
    this.pushVolume(false);
  }

  /** The player no longer uses the window (another item or source): close it. */
  release(): void {
    if (this.url === null && !this.opening) return;
    this.generation++;
    this.url = null;
    this.opening = false;
    this.seenOpen = false;
    this.requestClose();
  }

  /** New `MediaWindowStatus` from the shell (already revision-guarded by the store). */
  applyStatus(status: MediaWindowStatus): void {
    this.status = status;
    if (status.open) {
      // A status from before our open request may still be in flight: count it only for our page
      // (or once the request has completed).
      if (this.url !== null && !this.seenOpen && (!this.opening || samePage(status.url, this.url))) {
        this.seenOpen = true;
      }
      return;
    }
    if (this.closing) {
      this.closing = false;
      return;
    }
    if (this.url !== null && this.seenOpen) {
      this.generation++;
      this.url = null;
      this.opening = false;
      this.seenOpen = false;
      this.events.onClosedByUser();
    }
  }

  private pushVolume(force: boolean): void {
    if (this.url === null || this.opening) return;
    if (!force && this.sentVolume === this.volume) return;
    this.sentVolume = this.volume;
    this.control("setVolume", this.volume);
  }

  private requestClose(): void {
    this.closing = true;
    this.control("close");
  }

  private control(action: MediaAction, volume: number | null = null): void {
    controlMediaWindow(action, volume).catch((e: unknown) => {
      // Closing a window that is already gone is not worth a message.
      if (action !== "close") this.events.onControlFailed(toAppError(e));
    });
  }
}
