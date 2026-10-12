// Shared helpers for the FocusMode browser extension (service worker, popup and blocked page).
//
// Nothing in this module talks to the network. The only network peer of the whole extension is the
// FocusMode desktop app on the loopback interface (see BRIDGE_STATE_URL), and only background.js calls it.

/** Loopback port of the FocusMode bridge (`BRIDGE_PORT` in crates/focus-core/src/api.rs). */
export const BRIDGE_PORT = 47615;

/** `GET` returns `BridgeState` JSON: `{ active, blocklist, phase, remainingSecs }`. */
export const BRIDGE_STATE_URL = `http://127.0.0.1:${BRIDGE_PORT}/v1/state`;

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

/** Upper bound of domains turned into a blocking rule (the app caps the list at 500). */
export const MAX_DOMAINS = 1000;

/** State used before the app has been contacted at least once. */
export const INITIAL_STATE = Object.freeze({
  connected: false,
  active: false,
  phase: null,
  remainingSecs: 0,
  blocklist: [],
  fetchedAt: 0,
  error: 'not-checked',
  rulesMode: 'none',
  rulesKey: null,
});

/** Polish UI texts shared by the popup and the blocked page. */
export const TEXT = Object.freeze({
  phase: Object.freeze({
    idle: 'Brak sesji',
    focus: 'Skupienie',
    break: 'Przerwa',
    unknown: 'Nieznana faza',
  }),
  connected: 'Połączono z aplikacją FocusMode',
  disconnected: 'Brak połączenia z aplikacją FocusMode',
  checking: 'Sprawdzam połączenie…',
  paused: 'pauza',
});

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
 * Validates the `/v1/state` response body; returns a connected state or null when the shape is wrong.
 * @param {unknown} body
 * @param {number} fetchedAt epoch ms
 */
export function parseBridgeState(body, fetchedAt) {
  if (body === null || typeof body !== 'object') return null;
  const { active, blocklist, phase, remainingSecs } = /** @type {Record<string, unknown>} */ (body);
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
    fetchedAt,
    error: null,
  };
}

/**
 * Sites are being blocked right now. Only the focus phase blocks: at break start the app switches the
 * phase first and lowers `active` a moment later (shields worker), so a lagging `active` must not keep
 * the rules alive.
 * @param {{ connected: boolean, active: boolean, phase: string | null, blocklist: string[] } | null | undefined} state
 */
export function isBlocking(state) {
  return Boolean(
    state && state.connected && state.active && state.phase === 'focus' && state.blocklist.length > 0,
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
 * Polish label of a phase.
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
