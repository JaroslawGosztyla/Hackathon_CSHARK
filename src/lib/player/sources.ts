// Turns the music config (+ folder tracks) into a playlist of playable items.
import { fileSrc } from "$lib/api";
import { strings } from "$lib/strings";
import type { MusicConfig, NoiseColor, Track } from "$lib/types";
import { isEmbedBlocked } from "./embedBlocks";
import { isYouTubeChannelPage, parseHttpUrl, parseYouTube, youTubeWindowUrl, type YouTubeTarget } from "./youtube";

/** Lofi Girl's current live stream (the channel /live page always points at it; stream ids change over time). */
export const LOFI_GIRL_URL = "https://www.youtube.com/@LofiGirl/live";

export interface StreamPreset {
  url: string;
  label: string;
  /** Help text shown when the preset is picked. */
  hint: string | null;
  /**
   * The owner refuses embedding (verified: YouTube error 150 from any origin), so it always plays in the
   * separate YouTube window, without trying the embedded player first.
   */
  windowOnly: boolean;
}

export const STREAM_PRESETS: StreamPreset[] = [
  { url: LOFI_GIRL_URL, label: strings.music.lofiPreset, hint: strings.music.lofiPresetHint, windowOnly: true },
];

export type PlayItem =
  | {
      kind: "youtube";
      /** What the embedded player loads; null = never embeddable, always the separate YouTube window. */
      target: YouTubeTarget | null;
      /** https page the separate YouTube window opens when the embed is refused (or impossible). */
      pageUrl: string;
      label: string;
    }
  | { kind: "audio"; url: string; label: string; live: boolean }
  | { kind: "noise"; color: NoiseColor; label: string };

export type LinkKind = "youtubeVideo" | "youtubePlaylist" | "youtubeWindow" | "audio" | "unsupported" | "invalid";

/** Services whose pages are not audio files and cannot be embedded here. */
const UNSUPPORTED_HOSTS = [
  "spotify.com",
  "soundcloud.com",
  "music.apple.com",
  "tidal.com",
  "deezer.com",
  "music.amazon.com",
  "bandcamp.com",
];

function hostMatches(host: string, domain: string): boolean {
  return host === domain || host.endsWith(`.${domain}`);
}

export function classifyLink(raw: string): LinkKind {
  const yt = parseYouTube(raw);
  if (yt) return yt.listId ? "youtubePlaylist" : "youtubeVideo";
  // Channel / handle pages (e.g. `/@handle/live`) cannot be embedded, but play in the separate YouTube window.
  if (isYouTubeChannelPage(raw)) return "youtubeWindow";
  const url = parseHttpUrl(raw);
  if (!url) return "invalid";
  const host = url.hostname.toLowerCase();
  if (UNSUPPORTED_HOSTS.some((d) => hostMatches(host, d))) return "unsupported";
  // Any other youtube.com URL (home page, search, …) has nothing to play.
  if (hostMatches(host, "youtube.com") || host === "youtu.be") return "unsupported";
  return "audio";
}

export function isPlayableKind(kind: LinkKind): boolean {
  return kind === "youtubeVideo" || kind === "youtubePlaylist" || kind === "youtubeWindow" || kind === "audio";
}

/** Splits textarea input into trimmed, non-empty lines. */
export function splitLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
}

function audioLabel(url: URL): string {
  const last = url.pathname.split("/").filter(Boolean).pop();
  if (last) {
    try {
      const decoded = decodeURIComponent(last).replace(/\.[a-z0-9]{2,5}$/i, "").trim();
      if (decoded) return decoded;
    } catch {
      // Malformed escape sequence: fall back to the host name.
    }
  }
  return url.hostname;
}

/** The preset a link points to (same URL or the same YouTube video/playlist), if any. */
export function presetFor(raw: string): StreamPreset | null {
  const target = parseYouTube(raw);
  for (const preset of STREAM_PRESETS) {
    if (raw.trim() === preset.url) return preset;
    const presetTarget = parseYouTube(preset.url);
    if (target && presetTarget && target.videoId === presetTarget.videoId && target.listId === presetTarget.listId) {
      return preset;
    }
  }
  return null;
}

/** The link plays in the separate YouTube window (cannot be embedded, or its embed was refused before). */
export function playsInWindow(raw: string): boolean {
  const kind = classifyLink(raw);
  if (kind === "youtubeWindow") return true;
  if (kind !== "youtubeVideo" && kind !== "youtubePlaylist") return false;
  if (presetFor(raw)?.windowOnly) return true;
  const target = parseYouTube(raw);
  return target !== null && isEmbedBlocked(target);
}

/** One link → one item; null for unsupported/invalid links. `live` marks radio streams. */
export function linkToItem(raw: string, live: boolean): PlayItem | null {
  const kind = classifyLink(raw);
  if (kind === "youtubeVideo" || kind === "youtubePlaylist" || kind === "youtubeWindow") {
    const pageUrl = youTubeWindowUrl(raw);
    if (!pageUrl) return null;
    const preset = presetFor(raw);
    const target = kind === "youtubeWindow" || preset?.windowOnly ? null : parseYouTube(raw);
    return { kind: "youtube", target, pageUrl, label: preset?.label ?? strings.player.youtubeFallbackTitle };
  }
  if (kind === "audio") {
    const url = parseHttpUrl(raw);
    if (!url) return null;
    return { kind: "audio", url: url.toString(), label: presetFor(raw)?.label ?? audioLabel(url), live };
  }
  return null;
}

function shuffled<T>(items: T[]): T[] {
  const out = items.slice();
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    const a = out[i];
    const b = out[j];
    if (a !== undefined && b !== undefined) {
      out[i] = b;
      out[j] = a;
    }
  }
  return out;
}

export function buildPlaylist(music: MusicConfig, tracks: Track[]): PlayItem[] {
  switch (music.source) {
    case "none":
      return [];
    case "noise":
      return [{ kind: "noise", color: music.noise, label: strings.player.noiseTitle(strings.music.noise[music.noise]) }];
    case "stream": {
      const item = linkToItem(music.streamUrl, true);
      return item ? [item] : [];
    }
    case "links": {
      const items = music.links.map((l) => linkToItem(l, false)).filter((i): i is PlayItem => i !== null);
      return music.shuffle ? shuffled(items) : items;
    }
    case "folder": {
      const items: PlayItem[] = tracks.map((t) => ({ kind: "audio", url: fileSrc(t.path), label: t.title, live: false }));
      return music.shuffle ? shuffled(items) : items;
    }
  }
}

/** Short description of the configured source for the idle player bar. */
export function describeSource(music: MusicConfig, trackCount: number | null): string {
  switch (music.source) {
    case "none":
      return strings.music.source.none;
    case "noise":
      return strings.player.noiseTitle(strings.music.noise[music.noise]);
    case "stream":
      return `${strings.music.source.stream}: ${presetFor(music.streamUrl)?.label ?? (music.streamUrl || "—")}`;
    case "links":
      return `${strings.music.source.links}: ${music.links.length}`;
    case "folder":
      return trackCount === null
        ? `${strings.music.source.folder}: ${music.folder ?? "—"}`
        : `${strings.music.source.folder}: ${strings.music.trackCount(trackCount)}`;
  }
}
