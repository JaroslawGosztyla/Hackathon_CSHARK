// FocusMode extension service worker (Manifest V3).
//
// Polls the FocusMode desktop app on the loopback bridge and, while a focus phase is running, redirects
// navigations to blocked domains to the local "go back to work" page (blocked.html). Nothing is sent
// anywhere: the only request this extension makes is `GET http://127.0.0.1:47615/v1/state`.
//
// Blocking layers:
//  1. A declarativeNetRequest dynamic rule redirects main-frame requests to blocked domains. Preferred
//     variant: `regexSubstitution` that appends the original URL as the fragment of blocked.html, so the
//     page can offer it again during the break. Fallback variant: `extensionPath: "/blocked.html"`.
//  2. tabs.onUpdated / onActivated catch pages that did not go through the network (already open when
//     blocking started, back/forward cache, same-document navigations) and reload them, so the rule
//     applies and the history entry is replaced. If that does not help, the tab is sent to
//     blocked.html?u=<url> with tabs.update.
// App unreachable, `active: false` or a phase other than focus → all rules are removed on that very poll.
// The blocked page asks for an immediate poll (MSG_POLL_NOW) as soon as it sees the break, so the rules
// are gone before it sends the tab back to the original site.

import {
  BRIDGE_STATE_URL,
  INITIAL_STATE,
  MSG_GET_STATE,
  MSG_POLL_NOW,
  formatClock,
  isBlocking,
  matchBlockedDomain,
  parseBridgeState,
  parseWebUrl,
  phaseLabel,
} from './common.js';

const ALARM_NAME = 'focusmode-poll';
const POLL_PERIOD_MINUTES = 0.5;
const FETCH_TIMEOUT_MS = 2000;
const UPDATED_THROTTLE_MS = 5000;
const ACTIVATED_THROTTLE_MS = 1000;
const PAGE_THROTTLE_MS = 1000;
const RULE_ID = 1;
/** After reloading a tab, check that the rule really redirected it. */
const VERIFY_DELAY_MS = 4000;
/** The same URL showing up again this soon after a reload means the rule does not apply to it. */
const REPEAT_WINDOW_MS = 15000;
const MAX_REMEMBERED_TABS = 100;
const REMEMBERED_URL_TTL_MS = 60000;
const STORAGE_STATE_KEY = 'state';
const STORAGE_TAB_URLS_KEY = 'blockedTabUrls';
const BADGE_FOCUS_COLOR = '#d9893b';
const BADGE_BREAK_COLOR = '#3f9f84';
const BADGE_TEXT_COLOR = '#1b1208';
const LOG = '[FocusMode]';

/** Last applied state; mirrors chrome.storage.session so a restarted worker keeps it. */
let memoryState = null;
/** Running refresh, shared by concurrent callers. */
let inflight = null;
/** Epoch ms of the last completed bridge fetch (this worker instance). */
let lastFetchAt = 0;
/** tabId → { url, at } of tabs this worker recently reloaded or redirected. */
const recentRedirects = new Map();

// ---------------------------------------------------------------------------
// Event listeners (registered synchronously at top level, as MV3 requires)
// ---------------------------------------------------------------------------

chrome.runtime.onInstalled.addListener(() => {
  ensureAlarm(true).catch(logError('alarm'));
  refresh('installed').catch(logError('refresh'));
});

chrome.runtime.onStartup.addListener(() => {
  ensureAlarm(false).catch(logError('alarm'));
  refresh('startup').catch(logError('refresh'));
});

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === ALARM_NAME) refresh('alarm').catch(logError('refresh'));
});

chrome.tabs.onActivated.addListener(({ tabId }) => {
  checkTab(tabId, ACTIVATED_THROTTLE_MS).catch(logError('tab-activated'));
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (changeInfo.status !== 'loading' && changeInfo.url === undefined) return;
  checkTab(tabId, UPDATED_THROTTLE_MS).catch(logError('tab-updated'));
});

chrome.tabs.onRemoved.addListener((tabId) => {
  recentRedirects.delete(tabId);
  forgetTabUrl(tabId).catch(logError('tab-removed'));
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // Only our own pages (popup, blocked page) talk to the worker; there is no externally_connectable.
  if (sender.id !== chrome.runtime.id || !message || typeof message !== 'object') return false;
  let answer;
  if (message.type === MSG_GET_STATE) {
    answer = answerStateRequest(message, sender, false);
  } else if (message.type === MSG_POLL_NOW) {
    // The blocked page saw the break (or the app gone): fetch again right away so the rules are dropped
    // before it sends the tab back to the original site.
    answer = answerStateRequest(message, sender, true);
  } else {
    return false;
  }
  answer.then(sendResponse).catch((err) => {
    console.error(LOG, 'state request failed', err);
    sendResponse({ ok: false, error: String(err) });
  });
  return true; // keep the channel open for the async answer
});

// The worker may be woken by any event; make sure the periodic poll exists.
ensureAlarm(false).catch(logError('alarm'));

// ---------------------------------------------------------------------------
// Polling
// ---------------------------------------------------------------------------

/** @param {boolean} force recreate the alarm even when it exists */
async function ensureAlarm(force) {
  if (!force) {
    const existing = await chrome.alarms.get(ALARM_NAME);
    if (existing) return;
  }
  await chrome.alarms.create(ALARM_NAME, {
    delayInMinutes: POLL_PERIOD_MINUTES,
    periodInMinutes: POLL_PERIOD_MINUTES,
  });
}

/**
 * Fetches the bridge state and applies it (rules, open tabs, badge). Concurrent calls share one run;
 * with `minIntervalMs` a fetch younger than that is reused instead of asking the app again.
 * @param {string} reason for logs
 * @param {number} [minIntervalMs]
 * @returns {Promise<{ state: typeof INITIAL_STATE, swept: boolean }>}
 */
function refresh(reason, minIntervalMs = 0) {
  if (inflight !== null) return inflight;
  if (minIntervalMs > 0 && Date.now() - lastFetchAt < minIntervalMs) {
    return loadState().then((state) => ({ state, swept: false }));
  }
  inflight = (async () => {
    try {
      const fetched = await fetchBridgeState();
      lastFetchAt = Date.now();
      return await applyState(fetched, reason);
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

/**
 * Asks the app again even when a fetch is already running: that one may have started before the change
 * the caller has just seen, so it is awaited and a new one is made (or one started meanwhile is shared).
 * @param {string} reason for logs
 */
async function refreshNow(reason) {
  const running = inflight;
  if (running !== null) {
    try {
      await running;
    } catch {
      // That run reports its own failure; ours fetches again below.
    }
  }
  return refresh(reason);
}

/** Never throws: an unreachable or misbehaving app is reported as a disconnected state. */
async function fetchBridgeState() {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const response = await fetch(BRIDGE_STATE_URL, {
      method: 'GET',
      headers: { Accept: 'application/json', 'X-FocusMode-Extension': '1' },
      cache: 'no-store',
      credentials: 'omit',
      redirect: 'error',
      signal: controller.signal,
    });
    if (!response.ok) return disconnected(`http-${response.status}`);
    const body = await response.json();
    return parseBridgeState(body, Date.now()) ?? disconnected('invalid-response');
  } catch (err) {
    return disconnected(err && err.name === 'AbortError' ? 'timeout' : 'unreachable');
  } finally {
    clearTimeout(timer);
  }
}

/** @param {string} error */
function disconnected(error) {
  return { ...INITIAL_STATE, blocklist: [], fetchedAt: Date.now(), error };
}

/**
 * @param {ReturnType<typeof disconnected>} fetched
 * @param {string} reason
 */
async function applyState(fetched, reason) {
  const previous = await loadState();
  const domains = isBlocking(fetched) ? fetched.blocklist : [];
  const key = domains.join(',');
  const keyChanged = previous.rulesKey !== key;

  let rulesMode = previous.rulesMode;
  let rulesKey = previous.rulesKey;
  // Not blocking → make sure no rule is left, even when the cached key already says so (a rule that
  // survived a browser or extension restart, a cache out of sync). Costs one getDynamicRules per poll.
  if (keyChanged || domains.length === 0) {
    try {
      rulesMode = await installRules(domains);
      rulesKey = key;
      if (keyChanged) {
        console.info(LOG, `rules updated (${reason}): ${domains.length} domain(s), mode ${rulesMode}`);
      }
    } catch (err) {
      // Blocking still works through the tab listeners; the rules are retried on the next refresh.
      console.error(LOG, 'could not update blocking rules', err);
      rulesMode = domains.length > 0 ? 'tabs' : 'none';
      rulesKey = null;
    }
  }

  const next = { ...fetched, rulesMode, rulesKey };
  await saveState(next);
  await updateBadge(next);

  const wasBlocked = isBlocking(previous) ? new Set(previous.blocklist) : new Set();
  const newlyBlocked = domains.filter((domain) => !wasBlocked.has(domain));
  let swept = false;
  if (newlyBlocked.length > 0) {
    await sweepOpenTabs(newlyBlocked, rulesMode);
    swept = true;
  }
  return { state: next, swept };
}

// ---------------------------------------------------------------------------
// declarativeNetRequest rules
// ---------------------------------------------------------------------------

/**
 * Atomically replaces all dynamic rules. Returns the variant in use: 'regex', 'path' or 'none'.
 * @param {string[]} domains normalized ASCII domains
 */
async function installRules(domains) {
  const existing = await chrome.declarativeNetRequest.getDynamicRules();
  const removeRuleIds = existing.map((rule) => rule.id);
  if (domains.length === 0) {
    if (removeRuleIds.length > 0) {
      await chrome.declarativeNetRequest.updateDynamicRules({ removeRuleIds });
      console.info(LOG, `removed ${removeRuleIds.length} blocking rule(s)`);
    }
    return 'none';
  }
  try {
    await chrome.declarativeNetRequest.updateDynamicRules({
      removeRuleIds,
      addRules: [regexRedirectRule(domains)],
    });
    return 'regex';
  } catch (err) {
    // updateDynamicRules is atomic: nothing changed, try the simpler rule.
    console.warn(LOG, 'regexSubstitution redirect rejected, using extensionPath', err);
  }
  await chrome.declarativeNetRequest.updateDynamicRules({
    removeRuleIds,
    addRules: [pathRedirectRule(domains)],
  });
  return 'path';
}

/**
 * Redirects to blocked.html#<original url>. `requestDomains` also matches subdomains.
 * @param {string[]} domains
 */
function regexRedirectRule(domains) {
  return {
    id: RULE_ID,
    priority: 1,
    action: {
      type: 'redirect',
      redirect: { regexSubstitution: `${chrome.runtime.getURL('blocked.html')}#\\1` },
    },
    condition: {
      regexFilter: '^(https?://.*)$',
      requestDomains: domains,
      resourceTypes: ['main_frame'],
    },
  };
}

/** @param {string[]} domains */
function pathRedirectRule(domains) {
  return {
    id: RULE_ID,
    priority: 1,
    action: { type: 'redirect', redirect: { extensionPath: '/blocked.html' } },
    condition: { requestDomains: domains, resourceTypes: ['main_frame'] },
  };
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/**
 * Re-checks a tab that was activated or started loading.
 * @param {number} tabId
 * @param {number} throttleMs
 */
async function checkTab(tabId, throttleMs) {
  const { state, swept } = await refresh('tab', throttleMs);
  if (swept || !isBlocking(state)) return; // a sweep has just handled every open tab
  const tab = await getTab(tabId);
  if (tab === null) return;
  // With a rule installed the browser redirects a pending navigation by itself; reloading the tab now
  // would only start it a second time. A committed blocked page (back/forward cache, same-document
  // navigation, rule not applied) still needs help, and its commit reports the URL through onUpdated.
  const rulesActive = state.rulesMode === 'regex' || state.rulesMode === 'path';
  const url = rulesActive ? tab.url || '' : tab.url || tab.pendingUrl || '';
  if (matchBlockedDomain(url, state.blocklist) !== null) {
    await redirectTab(tabId, url, state.rulesMode);
  }
}

/**
 * Sends already open tabs of newly blocked domains to the blocked page.
 * @param {string[]} domains
 * @param {string} rulesMode
 */
async function sweepOpenTabs(domains, rulesMode) {
  const tabs = await chrome.tabs.query({ url: ['http://*/*', 'https://*/*'] });
  for (const tab of tabs) {
    // Discarded tabs are not loaded; the rule catches them when they are reloaded.
    if (tab.id === undefined || tab.discarded) continue;
    const url = tab.url || '';
    if (matchBlockedDomain(url, domains) === null) continue;
    try {
      await redirectTab(tab.id, url, rulesMode);
    } catch (err) {
      console.warn(LOG, 'could not redirect an open tab', err);
    }
  }
}

/**
 * Reload lets the redirect rule replace the current history entry (so "back" does not loop); when the
 * rule demonstrably does not apply, navigate to blocked.html?u=<url> instead.
 * @param {number} tabId
 * @param {string} url
 * @param {string} rulesMode
 */
async function redirectTab(tabId, url, rulesMode) {
  const now = Date.now();
  const last = recentRedirects.get(tabId);
  const sameUrl = last !== undefined && last.url === url;
  if (sameUrl && now - last.at < VERIFY_DELAY_MS) return; // already being handled
  recentRedirects.set(tabId, { url, at: now });
  await rememberTabUrl(tabId, url);

  const rulesActive = rulesMode === 'regex' || rulesMode === 'path';
  if (rulesActive && !(sameUrl && now - last.at < REPEAT_WINDOW_MS)) {
    await chrome.tabs.reload(tabId);
    setTimeout(() => {
      verifyRedirect(tabId, url).catch(logError('verify'));
    }, VERIFY_DELAY_MS);
    return;
  }
  await chrome.tabs.update(tabId, { url: blockedPageUrl(url) });
}

/**
 * The tab still shows the blocked URL after the reload → the rule did not apply (e.g. site access
 * withheld for this site); fall back to tabs.update.
 * @param {number} tabId
 * @param {string} url
 */
async function verifyRedirect(tabId, url) {
  const tab = await getTab(tabId);
  if (tab === null || tab.url !== url) return;
  const state = await loadState();
  if (!isBlocking(state) || matchBlockedDomain(url, state.blocklist) === null) return;
  recentRedirects.set(tabId, { url, at: Date.now() });
  await chrome.tabs.update(tabId, { url: blockedPageUrl(url) });
}

/** @param {string} originalUrl */
function blockedPageUrl(originalUrl) {
  const params = new URLSearchParams({ u: originalUrl, via: 'tab' });
  return `${chrome.runtime.getURL('blocked.html')}?${params.toString()}`;
}

/** @param {number} tabId */
async function getTab(tabId) {
  try {
    return await chrome.tabs.get(tabId);
  } catch {
    return null; // closed meanwhile
  }
}

// ---------------------------------------------------------------------------
// Messages from the popup and the blocked page
// ---------------------------------------------------------------------------

/**
 * @param {{ refresh?: boolean, tabId?: number }} message
 * @param {chrome.runtime.MessageSender} sender
 * @param {boolean} force fetch from the app now (MSG_POLL_NOW), never a cached or already running fetch
 */
async function answerStateRequest(message, sender, force) {
  const { state } = force
    ? await refreshNow('poll-now')
    : message.refresh
      ? await refresh('page', PAGE_THROTTLE_MS)
      : { state: await loadState() };
  const tabId = Number.isInteger(message.tabId) ? message.tabId : sender.tab?.id;
  const originalUrl = tabId === undefined ? null : await recallTabUrl(tabId);
  return { ok: true, state, originalUrl };
}

// ---------------------------------------------------------------------------
// Storage (chrome.storage.session: memory only, cleared when the browser closes)
// ---------------------------------------------------------------------------

/** @returns {Promise<typeof INITIAL_STATE>} */
async function loadState() {
  if (memoryState !== null) return memoryState;
  try {
    const stored = await chrome.storage.session.get(STORAGE_STATE_KEY);
    const value = stored[STORAGE_STATE_KEY];
    if (value && typeof value === 'object' && Array.isArray(value.blocklist)) {
      memoryState = value;
      return value;
    }
  } catch (err) {
    console.warn(LOG, 'could not read session storage', err);
  }
  return { ...INITIAL_STATE, blocklist: [] };
}

/** @param {typeof INITIAL_STATE} state */
async function saveState(state) {
  memoryState = state;
  try {
    await chrome.storage.session.set({ [STORAGE_STATE_KEY]: state });
  } catch (err) {
    console.warn(LOG, 'could not write session storage', err);
  }
}

/** @returns {Promise<Array<[number, string, number]>>} entries `[tabId, url, rememberedAtMs]` */
async function loadTabUrls() {
  try {
    const stored = await chrome.storage.session.get(STORAGE_TAB_URLS_KEY);
    const value = stored[STORAGE_TAB_URLS_KEY];
    return Array.isArray(value) ? value.filter((entry) => Array.isArray(entry) && entry.length === 3) : [];
  } catch (err) {
    console.warn(LOG, 'could not read session storage', err);
    return [];
  }
}

/** @param {Array<[number, string, number]>} entries */
async function saveTabUrls(entries) {
  try {
    await chrome.storage.session.set({ [STORAGE_TAB_URLS_KEY]: entries });
  } catch (err) {
    console.warn(LOG, 'could not write session storage', err);
  }
}

/**
 * Remembers the original URL of a tab this worker sends to the blocked page. Needed when the
 * extensionPath rule is in use (the page then gets no URL of its own).
 * @param {number} tabId
 * @param {string} url
 */
async function rememberTabUrl(tabId, url) {
  const entries = (await loadTabUrls()).filter(([id]) => id !== tabId);
  entries.push([tabId, url, Date.now()]);
  await saveTabUrls(entries.slice(-MAX_REMEMBERED_TABS));
}

/**
 * Only a fresh entry is trusted: the blocked page asks right after loading, and an old entry could
 * belong to an earlier site in the same tab.
 * @param {number} tabId
 */
async function recallTabUrl(tabId) {
  const entry = (await loadTabUrls()).find(([id]) => id === tabId);
  if (entry === undefined || Date.now() - entry[2] > REMEMBERED_URL_TTL_MS) return null;
  return parseWebUrl(entry[1]) !== null ? entry[1] : null;
}

async function forgetTabUrl(tabId) {
  const entries = await loadTabUrls();
  const kept = entries.filter(([id]) => id !== tabId);
  if (kept.length !== entries.length) await saveTabUrls(kept);
}

// ---------------------------------------------------------------------------
// Toolbar badge
// ---------------------------------------------------------------------------

/** @param {typeof INITIAL_STATE} state */
async function updateBadge(state) {
  try {
    let text = '';
    let color = BADGE_FOCUS_COLOR;
    let title = 'FocusMode — brak połączenia z aplikacją';
    if (state.connected) {
      title = `FocusMode — ${phaseLabel(state.phase)}`;
      if (state.phase === 'focus' || state.phase === 'break') {
        const minutes = Math.max(1, Math.ceil(state.remainingSecs / 60));
        text = `${minutes}m`;
        color = state.phase === 'focus' ? BADGE_FOCUS_COLOR : BADGE_BREAK_COLOR;
        title += `, zostało ${formatClock(state.remainingSecs)}`;
        if (isBlocking(state)) title += ' · blokada stron włączona';
      }
    }
    await chrome.action.setBadgeText({ text });
    if (text !== '') {
      await chrome.action.setBadgeBackgroundColor({ color });
      if (typeof chrome.action.setBadgeTextColor === 'function') {
        await chrome.action.setBadgeTextColor({ color: BADGE_TEXT_COLOR });
      }
    }
    await chrome.action.setTitle({ title });
  } catch (err) {
    console.warn(LOG, 'could not update the badge', err);
  }
}

// ---------------------------------------------------------------------------

/** @param {string} what */
function logError(what) {
  return (err) => console.error(LOG, `${what} failed`, err);
}
