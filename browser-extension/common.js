// Shared helpers for the FocusMode browser extension (service worker, popup and blocked page).
//
// Nothing in this module talks to the network. The only network peer of the whole extension is the
// FocusMode desktop app on the loopback interface (BRIDGE_STATE_URL, BRIDGE_REPORT_URL), and only
// background.js calls it.
//
// Texts come from _locales/<language>/messages.json through chrome.i18n (the browser's UI language,
// English when it is not translated): t() in scripts, data-i18n attributes in the HTML pages.

/** Loopback port of the FocusMode bridge (`BRIDGE_PORT` in crates/focus-core/src/api.rs). */
export const BRIDGE_PORT = 47615;

/**
 * `GET` returns `BridgeState` JSON: `{ active, blocklist, patterns, phase, remainingSecs }`. `blocklist` holds
 * domains, `patterns` regex sources (RE2 syntax, case-insensitive, matched against the full URL); both come
 * from `websiteblacklist.txt` in the app and are empty when nothing is blocked.
 */
export const BRIDGE_STATE_URL = `http://127.0.0.1:${BRIDGE_PORT}/v1/state`;

/**
 * `POST` with `{ "skipped": string[] }`: the pattern sources (exactly as received in `patterns`) that could
 * not become declarativeNetRequest rules (refused by the browser's RE2 checks, or beyond the regex rule
 * limit). The app shows them next to the blocklist (`BlocklistInfo.extensionSkipped`). Only patterns the
 * app itself sent are reported; never a URL, title or anything else about browsing.
 */
export const BRIDGE_REPORT_URL = `http://127.0.0.1:${BRIDGE_PORT}/v1/report`;

/** Runtime message: ask the service worker for the cached (or a refreshed) state. */
export const MSG_GET_STATE = 'focusmode:get-state';

/**
 * Runtime message: make the service worker ask the app right now (no throttle, no reuse of a fetch that
 * was already running) and apply the answer — e.g. drop the blocking rules — before replying.
 * Answered with the same shape as MSG_GET_STATE.
 */
export const MSG_POLL_NOW = 'focusmode:poll-now';

/** Phases as serialized by the app (`Phase` enum, camelCase). */
export const PHASES = Object.freeze(['idle', 'focus', 'break']);

/** Upper bound of domains turned into a blocking rule (same cap as `MAX_DOMAINS` in the app's blocklist parser). */
export const MAX_DOMAINS = 2000;

/** Upper bound of regex patterns taken from the app (each one becomes its own blocking rule). */
export const MAX_PATTERNS = 500;

/** Longer regex sources are ignored (Chrome limits a compiled rule regex to 2 KB anyway). */
export const MAX_PATTERN_LENGTH = 1000;

/**
 * State used before the app has been contacted at least once. The worker adds what it did with it:
 * `rulesMode` ('none' | 'regex' | 'path' | 'tabs'), `rulesKey`, `patternRules` (patterns that got a
 * declarativeNetRequest rule) and `skippedPatterns` (patterns the browser refused or that did not fit).
 */
export const INITIAL_STATE = Object.freeze({
  connected: false,
  active: false,
  phase: null,
  remainingSecs: 0,
  blocklist: [],
  patterns: [],
  fetchedAt: 0,
  error: 'not-checked',
  rulesMode: 'none',
  rulesKey: null,
  patternRules: [],
  skippedPatterns: [],
});

/**
 * A text from _locales in the browser's language; `substitutions` fill the message's placeholders in order.
 * Falls back to the key itself, so a missing message is visible instead of an empty element.
 * @param {string} key
 * @param {...(string | number)} substitutions
 * @returns {string}
 */
export function t(key, ...substitutions) {
  try {
    const text = chrome.i18n.getMessage(key, substitutions.map(String));
    return text !== '' ? text : key;
  } catch {
    return key;
  }
}

/** Texts shared by the popup, the blocked page and the toolbar button (read on use, in the browser's language). */
export const TEXT = Object.freeze({
  phase: Object.freeze({
    get idle() {
      return t('phaseIdle');
    },
    get focus() {
      return t('phaseFocus');
    },
    get break() {
      return t('phaseBreak');
    },
    get unknown() {
      return t('phaseUnknown');
    },
  }),
  get connected() {
    return t('connected');
  },
  get disconnected() {
    return t('disconnected');
  },
  get checking() {
    return t('checking');
  },
  get paused() {
    return t('paused');
  },
});

/** Marker in a message (data-i18n-code) replaced by a <code> element with the attribute's value. */
const CODE_MARKER = '{{file}}';

/**
 * Fills a page from _locales: `data-i18n="key"` sets an element's text, `data-i18n-title` its tooltip,
 * `data-i18n-code="text"` puts <code>text</code> where the message has {{file}} (never as HTML), and
 * `data-i18n-doc-title` on <html> the document title. Sets <html lang> to the language of the texts.
 * @param {Document} doc
 */
export function localizePage(doc) {
  const root = doc.documentElement;
  root.lang = t('lang');
  const docTitle = root.getAttribute('data-i18n-doc-title');
  if (docTitle) doc.title = t(docTitle);
  for (const element of doc.querySelectorAll('[data-i18n]')) {
    const key = element.getAttribute('data-i18n');
    if (!key) continue;
    const text = t(key);
    const code = element.getAttribute('data-i18n-code');
    const at = code === null ? -1 : text.indexOf(CODE_MARKER);
    if (code === null || at === -1) {
      element.textContent = text;
      continue;
    }
    const codeElement = doc.createElement('code');
    codeElement.textContent = code;
    element.replaceChildren(text.slice(0, at), codeElement, text.slice(at + CODE_MARKER.length));
  }
  for (const element of doc.querySelectorAll('[data-i18n-title]')) {
    const key = element.getAttribute('data-i18n-title');
    if (key) element.setAttribute('title', t(key));
  }
}

/**
 * Normalizes a blocklist entry to a lowercase ASCII (punycode) host name, or returns null.
 * Accepts bare domains ("reddit.com"), "*.reddit.com" and "www.reddit.com"; rejects anything with a
 * scheme, path, port, credentials, spaces or without a dot. Mirrors `blocklist::normalize_domain` in the app.
 * @param {unknown} input
 * @returns {string | null}
 */
export function normalizeDomain(input) {
  if (typeof input !== 'string') return null;
  let value = input.trim().toLowerCase();
  if (value.startsWith('*.')) value = value.slice(2);
  while (value.endsWith('.')) value = value.slice(0, -1);
  if (value.startsWith('www.') && value.indexOf('.', 4) !== -1) value = value.slice(4);
  if (value.length === 0 || value.length > 253 || !value.includes('.')) return null;
  if (/[\s\/\\?#@:%[\]]/.test(value)) return null;
  let host;
  try {
    // The URL parser converts internationalized names to punycode, as requestDomains requires.
    host = new URL(`http://${value}/`).hostname;
  } catch {
    return null;
  }
  if (!/^[a-z0-9.-]+$/.test(host) || host.startsWith('.') || host.includes('..')) return null;
  return host;
}

/**
 * Normalizes, deduplicates and sorts a blocklist received from the app.
 * @param {unknown} list
 * @returns {string[]}
 */
export function normalizeBlocklist(list) {
  if (!Array.isArray(list)) return [];
  const domains = new Set();
  for (const entry of list) {
    const domain = normalizeDomain(entry);
    if (domain !== null) domains.add(domain);
    if (domains.size >= MAX_DOMAINS) break;
  }
  return [...domains].sort();
}

/**
 * Validates the regex sources received from the app: strings, trimmed, non-empty, not too long, without
 * line breaks; duplicates dropped, order kept (it is the file order), at most MAX_PATTERNS. Whether the
 * browser can run them is checked later (isRegexSupported in the worker, compilePattern here).
 * @param {unknown} list
 * @returns {string[]}
 */
export function normalizePatterns(list) {
  if (!Array.isArray(list)) return [];
  const seen = new Set();
  const patterns = [];
  for (const entry of list) {
    if (typeof entry !== 'string') continue;
    const source = entry.trim();
    if (source.length === 0 || source.length > MAX_PATTERN_LENGTH || /[\r\n]/.test(source)) continue;
    if (seen.has(source)) continue;
    seen.add(source);
    patterns.push(source);
    if (patterns.length >= MAX_PATTERNS) break;
  }
  return patterns;
}

/**
 * `regexFilter` of the declarativeNetRequest rule for a pattern. The user regex is an unanchored search in
 * the URL; the rule must match the whole URL so that `\1` (the outer group, always group 1) carries the
 * complete original address to blocked.html. `.*` around a search does not change what matches, and a
 * `^` / `$` inside the pattern still means the start / end of the URL (the `.*` next to it then matches
 * nothing). The non-capturing group keeps a top-level `a|b` together.
 * @param {string} source
 */
export function ruleRegexFilter(source) {
  return `^(.*(?:${source}).*)$`;
}

/**
 * Translates an RE2 / Rust regex source into a JavaScript one with the same meaning for URL matching, or
 * returns null when it uses syntax that JavaScript would silently read differently (`\pL`, `\Q…\E`,
 * `\x{…}`, `[[:alpha:]]`). Handles `\A`, `\z`, `(?P<name>` and a leading `(?i)`; other inline flags make
 * `new RegExp` throw, which compilePattern also reports as null.
 * @param {string} source
 * @returns {string | null}
 */
function toJsRegexSource(source) {
  // A leading (?i) repeats what matching does anyway (always case-insensitive).
  const src = source.startsWith('(?i)') ? source.slice(4) : source;
  let out = '';
  for (let i = 0; i < src.length; i += 1) {
    const c = src[i];
    if (c === '\\') {
      const next = src[i + 1];
      if (next === undefined) return null;
      i += 1;
      if (next === 'A') {
        out += '^';
      } else if (next === 'z') {
        out += '$';
      } else if ('pPQECZ'.includes(next) || (next === 'x' && src[i + 1] === '{')) {
        return null;
      } else {
        out += c + next;
      }
    } else if (c === '[' && src[i + 1] === ':') {
      return null;
    } else if (c === '(' && src.startsWith('(?P<', i)) {
      out += '(?<';
      i += 3;
    } else {
      out += c;
    }
  }
  return out;
}

/** source → compiled RegExp, or null when JavaScript cannot evaluate it faithfully. */
const compiledPatterns = new Map();

/**
 * Case-insensitive JavaScript RegExp for a pattern (for checks outside the declarativeNetRequest rules:
 * already open tabs, same-document navigations, the blocked page). Null when JavaScript cannot run it the
 * way the browser rule (RE2) does; such a pattern is still enforced by its rule.
 * @param {string} source
 * @returns {RegExp | null}
 */
export function compilePattern(source) {
  const cached = compiledPatterns.get(source);
  if (cached !== undefined) return cached;
  let regex = null;
  const translated = typeof source === 'string' ? toJsRegexSource(source) : null;
  if (translated !== null) {
    try {
      regex = new RegExp(translated, 'i');
    } catch {
      regex = null;
    }
  }
  if (compiledPatterns.size >= 4 * MAX_PATTERNS) compiledPatterns.clear();
  compiledPatterns.set(source, regex);
  return regex;
}

/**
 * Some pattern cannot be checked in JavaScript, so "no pattern matched" is not a reliable answer.
 * @param {readonly string[] | undefined} patterns
 */
export function hasUncheckablePatterns(patterns) {
  return Array.isArray(patterns) && patterns.some((source) => compilePattern(source) === null);
}

/**
 * Returns the parsed URL when it is an http(s) URL, otherwise null.
 * @param {unknown} raw
 * @returns {URL | null}
 */
export function parseWebUrl(raw) {
  if (typeof raw !== 'string' || raw.length === 0) return null;
  let url;
  try {
    url = new URL(raw);
  } catch {
    return null;
  }
  return url.protocol === 'http:' || url.protocol === 'https:' ? url : null;
}

/**
 * Lowercase host of an http(s) URL without a trailing dot, or null.
 * @param {unknown} raw
 * @returns {string | null}
 */
export function hostOf(raw) {
  const url = parseWebUrl(raw);
  if (url === null) return null;
  let host = url.hostname.toLowerCase();
  while (host.endsWith('.')) host = host.slice(0, -1);
  return host.length > 0 ? host : null;
}

/**
 * Host equals the domain or is one of its subdomains.
 * @param {string} host
 * @param {string} domain
 */
export function hostMatches(host, domain) {
  return host === domain || host.endsWith(`.${domain}`);
}

/**
 * Returns the blocklist entry that matches the URL's host, or null.
 * @param {unknown} url
 * @param {readonly string[]} domains normalized domains
 * @returns {string | null}
 */
export function matchBlockedDomain(url, domains) {
  const host = hostOf(url);
  if (host === null) return null;
  for (const domain of domains) {
    if (hostMatches(host, domain)) return domain;
  }
  return null;
}

/**
 * Returns the first pattern that matches the full http(s) URL (case-insensitive search), or null.
 * Other schemes never match: the blocked page itself (chrome-extension://…#<url>) must not.
 * @param {unknown} url
 * @param {readonly string[] | undefined} patterns
 * @returns {string | null}
 */
export function matchBlockedPattern(url, patterns) {
  if (!Array.isArray(patterns) || patterns.length === 0) return null;
  const parsed = parseWebUrl(url);
  if (parsed === null) return null;
  for (const source of patterns) {
    const regex = compilePattern(source);
    if (regex !== null && regex.test(parsed.href)) return source;
  }
  return null;
}

/**
 * The blocklist entry that blocks this URL: a domain (host or its subdomain) or a regex pattern.
 * @param {unknown} url
 * @param {readonly string[]} domains normalized domains
 * @param {readonly string[] | undefined} patterns regex sources
 * @returns {{ kind: 'domain' | 'pattern', entry: string } | null}
 */
export function matchBlockedEntry(url, domains, patterns) {
  const domain = matchBlockedDomain(url, domains);
  if (domain !== null) return { kind: 'domain', entry: domain };
  const pattern = matchBlockedPattern(url, patterns);
  return pattern !== null ? { kind: 'pattern', entry: pattern } : null;
}

/**
 * Validates the `/v1/state` response body; returns a connected state or null when the shape is wrong.
 * @param {unknown} body
 * @param {number} fetchedAt epoch ms
 */
export function parseBridgeState(body, fetchedAt) {
  if (body === null || typeof body !== 'object') return null;
  const { active, blocklist, patterns, phase, remainingSecs } = /** @type {Record<string, unknown>} */ (body);
  if (typeof active !== 'boolean' || !Array.isArray(blocklist)) return null;
  if (typeof phase !== 'string' || !PHASES.includes(phase)) return null;
  const remaining =
    typeof remainingSecs === 'number' && Number.isFinite(remainingSecs) && remainingSecs > 0
      ? Math.floor(remainingSecs)
      : 0;
  return {
    connected: true,
    active,
    phase,
    remainingSecs: remaining,
    blocklist: normalizeBlocklist(blocklist),
    // Optional (`#[serde(default)]`): an older app sends no patterns.
    patterns: normalizePatterns(patterns),
    fetchedAt,
    error: null,
  };
}

/**
 * Sites are being blocked right now. Only the focus phase blocks: at break start the app switches the
 * phase first and lowers `active` a moment later (shields worker), so a lagging `active` must not keep
 * the rules alive.
 * @param {{ connected: boolean, active: boolean, phase: string | null, blocklist: string[], patterns?: string[] } | null | undefined} state
 */
export function isBlocking(state) {
  return Boolean(
    state &&
      state.connected &&
      state.active &&
      state.phase === 'focus' &&
      (state.blocklist.length > 0 || (Array.isArray(state.patterns) && state.patterns.length > 0)),
  );
}

/**
 * "mm:ss", or "h:mm:ss" for an hour and more.
 * @param {number} totalSecs
 */
export function formatClock(totalSecs) {
  const secs = Number.isFinite(totalSecs) && totalSecs > 0 ? Math.floor(totalSecs) : 0;
  const hours = Math.floor(secs / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  const seconds = secs % 60;
  const mm = String(minutes).padStart(2, '0');
  const ss = String(seconds).padStart(2, '0');
  return hours > 0 ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
}

/**
 * Label of a phase in the browser's language.
 * @param {unknown} phase
 */
export function phaseLabel(phase) {
  if (phase === 'idle' || phase === 'focus' || phase === 'break') return TEXT.phase[phase];
  return TEXT.phase.unknown;
}

/**
 * Asks the service worker for the state. Never throws; returns null when the worker cannot answer.
 * `force` sends MSG_POLL_NOW: a fresh fetch from the app, applied (rules updated) before the answer.
 * @param {{ refresh?: boolean, force?: boolean, tabId?: number }} [options]
 * @returns {Promise<{ state: typeof INITIAL_STATE, originalUrl: string | null } | null>}
 */
export async function requestState(options = {}) {
  try {
    const response = await chrome.runtime.sendMessage({
      type: options.force ? MSG_POLL_NOW : MSG_GET_STATE,
      refresh: Boolean(options.refresh),
      tabId: Number.isInteger(options.tabId) ? options.tabId : undefined,
    });
    if (response && response.ok === true && response.state && typeof response.state === 'object') {
      return {
        state: response.state,
        originalUrl: typeof response.originalUrl === 'string' ? response.originalUrl : null,
      };
    }
    console.warn('[FocusMode] unexpected state response', response);
  } catch (err) {
    console.warn('[FocusMode] could not reach the service worker', err);
  }
  return null;
}

/**
 * Polls the service worker every `pollMs` (each poll refreshes the bridge state) and calls `onView`
 * after every poll and every second with an interpolated countdown. `pollSeq` grows by one with every
 * completed poll (it stays the same for the countdown ticks in between). A phase whose remaining time did
 * not move between two fetches at least 2 s apart is reported as paused (the bridge has no pause flag).
 *
 * @param {{
 *   tabId?: number,
 *   pollMs?: number,
 *   onView: (view: { state: typeof INITIAL_STATE | null, originalUrl: string | null, remainingSecs: number, paused: boolean, pollSeq: number }) => void,
 * }} options
 * @returns {{ refresh: () => void, pollNow: () => void, stop: () => void }}
 */
export function watchState({ tabId, pollMs = 5000, onView }) {
  /** @type {typeof INITIAL_STATE | null} */
  let current = null;
  /** @type {string | null} */
  let originalUrl = null;
  let paused = false;
  let polling = false;
  /** A forced poll asked for while another poll was running; it runs right after that one. */
  let forceQueued = false;
  let lastPollAt = 0;
  let pollSeq = 0;
  let stopped = false;

  const remainingNow = () => {
    if (current === null || !current.connected || current.phase === 'idle') return 0;
    if (paused) return current.remainingSecs;
    const elapsed = Math.floor(Math.max(0, Date.now() - current.fetchedAt) / 1000);
    return Math.max(0, current.remainingSecs - elapsed);
  };

  const emit = () => {
    if (pollSeq === 0 || stopped) return;
    try {
      onView({ state: current, originalUrl, remainingSecs: remainingNow(), paused, pollSeq });
    } catch (err) {
      console.error('[FocusMode] render failed', err);
    }
  };

  /** @param {typeof INITIAL_STATE} next */
  const updatePaused = (next) => {
    if (current === null || !current.connected || !next.connected || current.phase !== next.phase) {
      paused = false;
    } else if (next.phase === 'idle') {
      paused = false;
    } else if (next.fetchedAt - current.fetchedAt >= 2000) {
      paused = current.remainingSecs === next.remainingSecs;
    }
    // Same fetch, or fetches too close together to tell: keep the previous answer.
  };

  /** @param {boolean} force */
  const poll = async (force) => {
    if (stopped) return;
    if (polling) {
      if (force) forceQueued = true;
      return;
    }
    polling = true;
    lastPollAt = Date.now();
    try {
      const response = await requestState({ refresh: true, force, tabId });
      if (response === null) {
        current = null;
        paused = false;
      } else {
        updatePaused(response.state);
        current = response.state;
        originalUrl = response.originalUrl;
      }
      pollSeq += 1;
    } finally {
      polling = false;
    }
    emit();
    if (forceQueued) {
      forceQueued = false;
      void poll(true);
    }
  };

  void poll(false);
  const pollTimer = setInterval(() => void poll(false), pollMs);
  const tickTimer = setInterval(() => {
    // Ask again right away when the local countdown reaches the end of the phase.
    if (
      current !== null &&
      current.connected &&
      current.phase !== 'idle' &&
      !paused &&
      remainingNow() === 0 &&
      Date.now() - lastPollAt > 1500
    ) {
      void poll(false);
    }
    emit();
  }, 1000);

  return {
    refresh: () => void poll(false),
    pollNow: () => void poll(true),
    stop: () => {
      stopped = true;
      clearInterval(pollTimer);
      clearInterval(tickTimer);
    },
  };
}
