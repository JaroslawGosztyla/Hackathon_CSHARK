// Videos/playlists whose owner refused embedding (YouTube errors 101/150/153). Remembered in memory and,
// per viewer, in localStorage, so the next play goes straight to the separate YouTube window instead of
// showing the failing embed first. Storage may be missing or throw: then it is just not remembered.
import type { YouTubeTarget } from "./youtube";

const STORAGE_KEY = "focusmode.player.embedBlocked";
/** Oldest entries are dropped beyond this (keeps localStorage small). */
const MAX_REMEMBERED = 200;

let blocked: Set<string> | null = null;

function keyOf(target: YouTubeTarget): string {
  return `${target.videoId ?? ""}|${target.listId ?? ""}`;
}

function loadBlocked(): Set<string> {
  if (blocked) return blocked;
  const set = new Set<string>();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    if (Array.isArray(parsed)) {
      for (const value of parsed) if (typeof value === "string") set.add(value);
    }
  } catch {
    // Unreadable or corrupt storage: start empty.
  }
  blocked = set;
  return set;
}

export function isEmbedBlocked(target: YouTubeTarget): boolean {
  return loadBlocked().has(keyOf(target));
}

export function rememberEmbedBlocked(target: YouTubeTarget): void {
  const set = loadBlocked();
  const key = keyOf(target);
  // Re-insert so the most recent entry is last (Set keeps insertion order).
  set.delete(key);
  set.add(key);
  for (const oldest of set) {
    if (set.size <= MAX_REMEMBERED) break;
    set.delete(oldest);
  }
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify([...set]));
  } catch {
    // Storage unavailable: remembered for this run only.
  }
}
