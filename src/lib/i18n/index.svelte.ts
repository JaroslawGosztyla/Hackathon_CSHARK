// UI language at runtime: the texts of the language in use (`strings`), <html lang> and the Intl locale of every
// date, time and number formatter. Every locale file is imported statically, so a switch never waits for a download
// and the first paint is already in the right language (the last resolved language is cached for the next window).
//
// `strings` keeps the shape of the Polish source (`Strings`) but reads the current locale on every property access,
// so `strings.x.y` (and aliases such as `const s = strings.settings`) re-render on a switch wherever Svelte tracks
// them: templates, `$derived` and effects. A value copied into plain state keeps the language it was read in.
import { untrack } from "svelte";
import { ERR_NO_SHELL, getSystemLanguage, toAppError } from "$lib/api";
import type { AppError, GameLauncher, Language } from "$lib/types";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { it } from "./it";
import { ja } from "./ja";
import { pl, type Strings } from "./pl";
import { ru } from "./ru";
import { uk } from "./uk";
import { zh } from "./zh";

export type { Strings };
export { pluralizer, type PluralForms } from "./plural";

/** A concrete UI language: everything `Config.language` allows except `system`. */
export type UiLanguage = Exclude<Language, "system">;

const LOCALES: Record<UiLanguage, Strings> = { en, pl, de, fr, it, uk, ru, es, zh, ja };

/** Every UI language, in the order of the language picker (focus-core `Language::SUPPORTED`). */
export const LANGUAGES: readonly UiLanguage[] = ["en", "pl", "de", "fr", "it", "uk", "ru", "es", "zh", "ja"];

/** Native language names (never translated), for the language picker. */
export const LANGUAGE_NAMES: Record<UiLanguage, string> = {
  en: "English",
  pl: "Polski",
  de: "Deutsch",
  fr: "Français",
  it: "Italiano",
  uk: "Українська",
  ru: "Русский",
  es: "Español",
  zh: "中文",
  ja: "日本語",
};

/** `<html lang>` per language (the script matters for CJK glyphs: Simplified Chinese, not Japanese forms). */
const HTML_LANG: Record<UiLanguage, string> = {
  en: "en",
  pl: "pl",
  de: "de",
  fr: "fr",
  it: "it",
  uk: "uk",
  ru: "ru",
  es: "es",
  zh: "zh-Hans",
  ja: "ja",
};

/** Intl locale per language when the WebView reports no regional variant of it (24-hour clock in Europe). */
const DEFAULT_INTL: Record<UiLanguage, string> = {
  en: "en-GB",
  pl: "pl-PL",
  de: "de-DE",
  fr: "fr-FR",
  it: "it-IT",
  uk: "uk-UA",
  ru: "ru-RU",
  es: "es-ES",
  zh: "zh-CN",
  ja: "ja-JP",
};

/**
 * Per-viewer cache of the last applied language (shared by the main window and the popup window, same origin), so
 * a window paints in the right language before `get_config` answers. `config.json` stays the only source of truth.
 */
const CACHE_KEY = "focusmode.language";

export function isUiLanguage(value: unknown): value is UiLanguage {
  return typeof value === "string" && (LANGUAGES as readonly string[]).includes(value);
}

/**
 * The UI language of a BCP 47 tag or locale name (`pl-PL`, `en_US`, `zh-Hans-CN`), by its primary subtag; null for
 * unsupported tags. Mirrors focus-core `Language::from_tag` for two-letter codes.
 */
export function languageFromTag(tag: string | null | undefined): UiLanguage | null {
  const primary = (tag ?? "").trim().split(/[-_.@]/)[0]?.toLowerCase() ?? "";
  return isUiLanguage(primary) ? primary : null;
}

function readCachedLanguage(): UiLanguage | null {
  try {
    const value = localStorage.getItem(CACHE_KEY);
    return isUiLanguage(value) ? value : null;
  } catch {
    return null;
  }
}

/** The WebView's preferred languages (the OS UI language in the shell). */
function browserTags(): readonly string[] {
  try {
    return navigator.languages.length > 0 ? navigator.languages : [navigator.language];
  } catch {
    return [];
  }
}

/** First supported language among the WebView's preferred ones (a guess until the shell answers). */
function browserLanguage(): UiLanguage | null {
  for (const tag of browserTags()) {
    const lang = languageFromTag(tag);
    if (lang) return lang;
  }
  return null;
}

let current = $state.raw<UiLanguage>(readCachedLanguage() ?? browserLanguage() ?? "en");
/** The OS UI language as resolved by the shell; null until `loadSystemLanguage` answers. */
let systemLanguage = $state.raw<UiLanguage | null>(null);
let systemPending: Promise<UiLanguage> | null = null;
/** Bumps with every `followLanguage`, so a late `system` lookup never overrides a newer choice. */
let followGeneration = 0;

applyDocumentLanguage(current);

/** The language in use. Reactive: templates, `$derived` and effects that read it re-run on a switch. */
export function language(): UiLanguage {
  return current;
}

/** The OS UI language for the "System (…)" option; null until known. Reactive. */
export function knownSystemLanguage(): UiLanguage | null {
  return systemLanguage;
}

const intlCache = new Map<UiLanguage, string>();

/**
 * The Intl locale of every formatter in the app (`$lib/format`): the WebView's own regional variant of the language
 * in use when it has one (`en-US` keeps a 12-hour clock), otherwise a default one. Reactive.
 */
export function intlLocale(): string {
  const lang = current;
  let tag = intlCache.get(lang);
  if (tag === undefined) {
    tag = DEFAULT_INTL[lang];
    for (const candidate of browserTags()) {
      if (languageFromTag(candidate) !== lang) continue;
      try {
        const [supported] = Intl.DateTimeFormat.supportedLocalesOf(candidate);
        if (supported) tag = supported;
      } catch {
        // Malformed tag: keep the default.
      }
      break;
    }
    intlCache.set(lang, tag);
  }
  return tag;
}

function applyDocumentLanguage(lang: UiLanguage): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (root.lang !== HTML_LANG[lang]) root.lang = HTML_LANG[lang];
}

/**
 * Switches every text, `<html lang>` and the Intl locale at once, and remembers the language for the first paint of
 * the next window. Does not save the config (the settings page does that).
 */
export function setLanguage(lang: UiLanguage): void {
  if (!isUiLanguage(lang)) return;
  if (untrack(() => current) !== lang) current = lang;
  applyDocumentLanguage(lang);
  try {
    if (localStorage.getItem(CACHE_KEY) !== lang) localStorage.setItem(CACHE_KEY, lang);
  } catch {
    // Storage unavailable: the next window starts in the WebView's language until the config arrives.
  }
}

/** Asks the shell for the OS UI language once per window (`en` when it is not supported). */
export function loadSystemLanguage(): Promise<UiLanguage> {
  systemPending ??= getSystemLanguage()
    .then((lang): UiLanguage => (isUiLanguage(lang) ? lang : "en"))
    .catch((e: unknown): UiLanguage => {
      const err = toAppError(e);
      if (err.code !== ERR_NO_SHELL) console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      return browserLanguage() ?? "en";
    })
    .then((lang) => {
      systemLanguage = lang;
      return lang;
    });
  return systemPending;
}

/**
 * Applies `Config.language`: a concrete language at once, `system` as soon as the OS language is known (at once when
 * it already is). Safe to call from effects: it never subscribes the caller to the language state.
 */
export function followLanguage(choice: Language | null | undefined): void {
  if (!choice) return;
  const generation = ++followGeneration;
  untrack(() => {
    if (choice !== "system") {
      setLanguage(choice);
      return;
    }
    if (systemLanguage) {
      setLanguage(systemLanguage);
      return;
    }
    void loadSystemLanguage().then((lang) => {
      if (generation === followGeneration) setLanguage(lang);
    });
  });
}

// ---------------------------------------------------------------------------
// `strings`: a read-only view of the current locale
// ---------------------------------------------------------------------------

type Branch = Record<string, unknown>;

function isBranch(value: unknown): value is Branch {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** The node at `path` in the current locale (reading `current` makes the caller depend on the language). */
function nodeAt(path: readonly string[]): unknown {
  let node: unknown = LOCALES[current];
  for (const key of path) {
    if (!isBranch(node)) return undefined;
    node = node[key];
  }
  return node;
}

const views = new Map<string, object>();

/** A stable proxy for the object at `path`; nested objects are views too, arrays and functions are returned as is. */
function view(path: readonly string[]): object {
  const id = path.join("\u0000");
  const cached = views.get(id);
  if (cached) return cached;
  const target = {};
  const read = (key: string): unknown => {
    const node = nodeAt(path);
    if (!isBranch(node) || !Object.hasOwn(node, key)) return undefined;
    const value = node[key];
    return isBranch(value) ? view([...path, key]) : value;
  };
  const proxy = new Proxy(target, {
    get(t, key) {
      if (typeof key === "symbol") return Reflect.get(t, key);
      const value = read(key);
      // Object.prototype members (`toString`, `hasOwnProperty`…) still work on a view.
      return value === undefined ? Reflect.get(t, key) : value;
    },
    has(_t, key) {
      const node = nodeAt(path);
      return typeof key === "string" && isBranch(node) && Object.hasOwn(node, key);
    },
    ownKeys() {
      const node = nodeAt(path);
      return isBranch(node) ? Object.keys(node) : [];
    },
    getOwnPropertyDescriptor(_t, key) {
      if (typeof key === "symbol") return undefined;
      const node = nodeAt(path);
      if (!isBranch(node) || !Object.hasOwn(node, key)) return undefined;
      return { value: read(key), enumerable: true, configurable: true, writable: false };
    },
    set: () => false,
    defineProperty: () => false,
    deleteProperty: () => false,
  });
  views.set(id, proxy);
  return proxy;
}

/** Texts of the language in use (shape of `pl.ts`); see the header comment. */
export const strings = view([]) as Strings;

/** Text for an `AppError` in the language in use (exact code, then its area prefix, then a generic line). */
export function errorText(error: AppError): string {
  if (error.code === "config.invalid" && error.field) {
    return strings.errors.fieldInvalid(strings.fields[error.field] ?? error.field);
  }
  const exact = strings.errors.codes[error.code];
  if (exact) return exact;
  const area = error.code.split(".")[0] ?? "";
  return strings.errors.areas[area] ?? strings.errors.generic;
}

/** Every game launcher, in the order of the Settings checkboxes (focus-core `launchers::ALL`). */
export const GAME_LAUNCHERS = Object.keys(pl.launchers) as GameLauncher[];

/** `Steam, Epic Games Launcher` */
export function launcherList(launchers: readonly GameLauncher[]): string {
  return launchers.map((l) => strings.launchers[l] ?? l).join(", ");
}
