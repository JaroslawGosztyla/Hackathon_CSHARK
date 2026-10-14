// Separate YouTube window (links that refuse embedding play there as a normal top-level page).
// The shell owns the window; this store mirrors its `MediaWindowStatus` (revision-guarded in api.subscribe).
import type { MediaWindowStatus } from "$lib/types";

const CLOSED: MediaWindowStatus = { revision: 0, open: false, url: null, title: null };

class MediaWindowStore {
  /** Replaced as a whole on every update, so no deep proxy is needed. */
  status = $state.raw<MediaWindowStatus>(CLOSED);

  open = $derived(this.status.open);
  /** Page title without the " - YouTube" suffix, if the shell knows it. */
  title = $derived(this.status.open ? this.status.title : null);

  apply(status: MediaWindowStatus): void {
    this.status = status;
  }
}

export const mediaWindow = new MediaWindowStore();
