// Music track lists: the tracks of the configured folder (listed by the shell, which also allows the folder in the
// asset protocol scope so `convertFileSrc` URLs load) and the music bundled with the app (Open Lo-Fi, CC0).
import * as api from "$lib/api";
import type { Track } from "$lib/types";
import { toasts } from "./toasts.svelte";

const VIDEO_EXPANDED_KEY = "focusmode.player.videoExpanded";

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
  private request = 0;
  private libraryRequest: Promise<void> | null = null;

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
