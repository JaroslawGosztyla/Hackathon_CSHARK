// Music track lists: the tracks of the configured folder (listed by the shell, which also allows the folder in the
// asset protocol scope so `convertFileSrc` URLs load) and the music bundled with the app (Open Lo-Fi, CC0).
// Also who wants the music on (`intent`), which the dock (MusicPlayer.svelte) turns into play/pause.
import * as api from "$lib/api";
import type { Track } from "$lib/types";
import { toasts } from "./toasts.svelte";

const VIDEO_EXPANDED_KEY = "focusmode.player.videoExpanded";

/**
 * Who wants the music on. The one rule of the dock (ARCHITECTURE §6.3 „Kto włączył muzykę”):
 * - "off": silent. Start → "session" (the session starts the music on a fresh playlist).
 * - "session": the session started it. Stop → "off" (the music stops).
 * - "user": the user pressed play (with or without a session). Start keeps it playing as it is (no restart, no new
 *   track); only when it plays nothing (every item failed, its YouTube window closed) Start takes it as from "off".
 *   Stop keeps it playing, unless the session held it silent at that moment (see below): then "off", so Stop never
 *   starts music.
 * - "userPaused": the user pressed pause. Stays silent across phase changes, Start and Stop until the user presses play:
 *   the pause in the dock always wins.
 * While a session runs it may hold the music silent whoever started it (Pauza, `pauseDuringBreak` in the break, the
 * meeting mute); the session lifts its own hold (Wznów, end of the break, end of the meeting), and Start lifts "off".
 * A play pressed during a hold plays through it (`override`) until that hold ends. In memory only: the app never
 * starts music by itself when it starts.
 */
export type MusicIntent = "off" | "session" | "user" | "userPaused";

function readExpanded(): boolean {
  try {
    return localStorage.getItem(VIDEO_EXPANDED_KEY) !== "0";
  } catch {
    return true;
  }
}

class MusicStore {
  /** Folder the `tracks` belong to. */
  folder = $state<string | null>(null);
  tracks = $state.raw<Track[]>([]);
  loading = $state(false);
  /** The YouTube video panel is on screen (the layout reserves room for it). */
  videoVisible = $state(false);
  /** Large video vs. a small bubble; remembered per viewer. */
  videoExpanded = $state(readExpanded());
  /** Bundled tracks; listed once (null until the shell answered). */
  library = $state.raw<Track[] | null>(null);
  libraryLoading = $state(false);
  /** Who wants the music on (`MusicIntent`). */
  intent = $state<MusicIntent>("off");
  /** The user pressed play while the session held the music silent: it plays through that hold until the hold ends. */
  override = $state(false);
  /** Bumped when the session starts the music (fresh shuffle) or stops it: the dock rebuilds the playlist. */
  playlistEpoch = $state(0);
  /** Someone wants the music on (it plays unless the session holds it silent). */
  wanted = $derived(this.intent === "session" || this.intent === "user");
  private request = 0;
  private libraryRequest: Promise<void> | null = null;

  /** Play in the dock. `held`: the session holds the music silent right now (the play overrides that hold). */
  play(held: boolean): void {
    this.intent = "user";
    this.override = held;
  }

  /** Pause in the dock: silent until the user presses play again. */
  pause(): void {
    this.intent = "userPaused";
    this.override = false;
  }

  /**
   * A session started (Start, or the first snapshot of a running session after the window loaded). `silent`: the music
   * plays nothing although it is wanted (every item failed, its YouTube window was closed): a "user" intent then counts
   * as stopped, and the session starts the music as from "off".
   */
  sessionStarted(silent: boolean): void {
    if (this.intent !== "off" && !(this.intent === "user" && silent)) return;
    this.intent = "session";
    this.playlistEpoch++;
  }

  /** The session ended (Stop). `audible`: the music was meant to play right before it (not held silent). */
  sessionEnded(audible: boolean): void {
    this.override = false;
    if (this.intent === "session" || (this.intent === "user" && !audible)) {
      this.intent = "off";
      this.playlistEpoch++;
    }
  }

  /** The session's hold ended (Wznów, end of the break or of the meeting mute): a play pressed during it is spent. */
  holdEnded(): void {
    this.override = false;
  }

  /** Lists the bundled tracks once; a failed attempt can be retried by the next call. */
  ensureLibrary(): Promise<void> {
    this.libraryRequest ??= (async () => {
      this.libraryLoading = true;
      try {
        this.library = await api.listLibraryTracks();
      } catch (e) {
        this.libraryRequest = null;
        const err = api.toAppError(e);
        if (err.code !== api.ERR_NO_SHELL) toasts.error(err);
      } finally {
        this.libraryLoading = false;
      }
    })();
    return this.libraryRequest;
  }

  toggleVideo(): void {
    this.videoExpanded = !this.videoExpanded;
    try {
      localStorage.setItem(VIDEO_EXPANDED_KEY, this.videoExpanded ? "1" : "0");
    } catch {
      // Storage unavailable: the preference just is not remembered.
    }
  }

  /** Loads the folder's tracks unless they are already loaded (or loading) for that folder. */
  async ensureTracks(folder: string | null, force = false): Promise<void> {
    if (!folder) {
      this.folder = null;
      this.tracks = [];
      return;
    }
    if (!force && this.folder === folder) return;
    const request = ++this.request;
    this.folder = folder;
    this.tracks = [];
    this.loading = true;
    try {
      const tracks = await api.listFolderTracks(folder);
      if (request === this.request) this.tracks = tracks;
    } catch (e) {
      if (request === this.request) {
        // Allow a retry later (e.g. the drive was not mounted yet).
        this.folder = null;
        const err = api.toAppError(e);
        if (err.code !== api.ERR_NO_SHELL) toasts.error(err);
      }
    } finally {
      if (request === this.request) this.loading = false;
    }
  }
}

export const music = new MusicStore();
