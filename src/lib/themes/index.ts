// App themes (`Config.theme`). Colours live in app.css as custom properties per `[data-theme="…"]`; this module
// only applies the chosen theme to <html> and remembers it for the first paint of the next window.
import type { Theme } from "$lib/bindings/Theme";

export type { Theme };

export type ColorScheme = "dark" | "light";

/** Every theme in the order of the settings grid, with the colour scheme of its form controls and scrollbars. */
const SCHEMES: Record<Theme, ColorScheme> = {
  midnight: "dark",
  vaporwave: "dark",
  lunar: "dark",
  autumn: "dark",
  dark: "dark",
  light: "light",
  pixel: "dark",
};

export const THEMES = Object.keys(SCHEMES) as Theme[];

/** Mirrors `Config::default()` in focus-core. */
export const DEFAULT_THEME: Theme = "midnight";

/**
 * Per-viewer cache of the last applied theme (shared by the main window and the popup window, same origin), so a
 * window paints in the right colours before `get_config` answers. `config.json` stays the only source of truth.
 */
const CACHE_KEY = "focusmode.theme";

export function isTheme(value: unknown): value is Theme {
  return typeof value === "string" && (THEMES as readonly string[]).includes(value);
}

export function colorScheme(theme: Theme): ColorScheme {
  return SCHEMES[theme];
}

/** The theme applied last time, if storage is available and holds a known theme. */
export function readCachedTheme(): Theme | null {
  try {
    const value = localStorage.getItem(CACHE_KEY);
    return isTheme(value) ? value : null;
  } catch {
    return null;
  }
}

/** Sets `<html data-theme>` (all tokens in app.css follow it) and caches the choice. Idempotent. */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (root.dataset.theme !== theme) root.dataset.theme = theme;
  // Overlay windows force a light scheme to keep their canvas transparent (app.html); leave their meta alone.
  if (!root.hasAttribute("data-overlay")) {
    document.querySelector('meta[name="color-scheme"]')?.setAttribute("content", SCHEMES[theme]);
  }
  try {
    if (localStorage.getItem(CACHE_KEY) !== theme) localStorage.setItem(CACHE_KEY, theme);
  } catch {
    // Storage unavailable: the next window just starts in the default colours until the config arrives.
  }
}
