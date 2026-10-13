// FocusMode extension service worker (Manifest V3).
//
// Polls the FocusMode desktop app on the loopback bridge and, while a focus phase is running, redirects
// navigations to blocked sites to the local "go back to work" page (blocked.html). Nothing leaves the
// computer: the extension only talks to the app on the loopback interface — `GET /v1/state` (the poll)
// and `POST /v1/report` (which of the app's own regex patterns the browser could not turn into rules).
//
// The blocklist (websiteblacklist.txt in the app) arrives as `blocklist` (domains, each with its
// subdomains) and `patterns` (regex sources, RE2 syntax, case-insensitive, searched in the full URL).
//
// Blocking layers:
//  1. declarativeNetRequest dynamic rules redirect main-frame requests: one rule for all domains
//     (`requestDomains`) and one rule per pattern (`regexFilter`). Preferred variant: `regexSubstitution`
//     that appends the original URL as the fragment of blocked.html, so the page can offer it again during
//     the break. Fallback variant: `extensionPath: "/blocked.html"`. Patterns are screened with
//     isRegexSupported first; refused ones (and ones beyond MAX_NUMBER_OF_REGEX_RULES) are skipped.
//  2. tabs.onUpdated / onActivated catch pages that did not go through the network (already open when
//     blocking started, back/forward cache, same-document navigations such as YouTube Shorts) and reload
//     them, so the rule applies and the history entry is replaced. If that does not help, or no rule
//     covers the entry, the tab is sent to blocked.html?u=<url> with tabs.update.
// App unreachable, `active: false` or a phase other than focus → all rules are removed on that very poll.
// The blocked page asks for an immediate poll (MSG_POLL_NOW) as soon as it sees the break, so the rules
// are gone before it sends the tab back to the original site.
//
// Skipped patterns: after the rules are installed for a focus phase, the list of patterns that did not get
// a rule (`skippedPatterns`) is posted to the app (debounced, at most once per distinct list per focus run,
// failures ignored), so Settings can say which lines Chrome cannot enforce on the network level.

import {
  BRIDGE_REPORT_URL,
  BRIDGE_STATE_URL,
  INITIAL_STATE,
  MSG_GET_STATE,
  MSG_POLL_NOW,
  formatClock,
  isBlocking,
  matchBlockedEntry,
  normalizePatterns,
  parseBridgeState,
  parseWebUrl,
  phaseLabel,
  ruleRegexFilter,
} from './common.js';

const ALARM_NAME = 'focusmode-poll';
const POLL_PERIOD_MINUTES = 0.5;
const FETCH_TIMEOUT_MS = 2000;
const UPDATED_THROTTLE_MS = 5000;
const ACTIVATED_THROTTLE_MS = 1000;
const PAGE_THROTTLE_MS = 1000;
/** The rule for all blocked domains. */
const DOMAIN_RULE_ID = 1;
/** Pattern rules get FIRST_PATTERN_RULE_ID, FIRST_PATTERN_RULE_ID + 1, … in list order. */
const FIRST_PATTERN_RULE_ID = 2;
/** chrome.declarativeNetRequest.MAX_NUMBER_OF_REGEX_RULES when the browser does not say. */
const DEFAULT_MAX_REGEX_RULES = 1000;
/** After reloading a tab, check that the rule really redirected it. */
const VERIFY_DELAY_MS = 4000;
/** The same URL showing up again this soon after a reload means the rule does not apply to it. */
const REPEAT_WINDOW_MS = 15000;
const MAX_REMEMBERED_TABS = 100;
const REMEMBERED_URL_TTL_MS = 60000;
const STORAGE_STATE_KEY = 'state';
const STORAGE_TAB_URLS_KEY = 'blockedTabUrls';
const STORAGE_REPORTED_KEY = 'reportedSkipped';
/** Quiet time before the skipped-pattern list is posted; a newer list within it replaces the pending one. */
const REPORT_DEBOUNCE_MS = 2000;
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
/**
 * JSON of the skipped-pattern list the app has answered for in this focus run, or null; mirrors
 * chrome.storage.session (STORAGE_REPORTED_KEY). `undefined` until read.
 * @type {string | null | undefined}
 */
let reportedSkipped;
/** @type {{ key: string, skipped: string[] } | null} list waiting for the debounce timer */
let pendingReport = null;
/** @type {ReturnType<typeof setTimeout> | null} */
let reportTimer = null;
/** JSON of the list being posted right now, or null. */
let sendingReportKey = null;
/** Bumped whenever the reported list is forgotten, so an answer to an older post cannot restore it. */
let reportEpoch = 0;

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
  return { ...INITIAL_STATE, blocklist: [], patterns: [], fetchedAt: Date.now(), error };
}

/**
 * @param {ReturnType<typeof disconnected>} fetched
 * @param {string} reason
 */
async function applyState(fetched, reason) {
  const previous = await loadState();
  const blocking = isBlocking(fetched);
  const domains = blocking ? fetched.blocklist : [];
  const patterns = blocking ? fetched.patterns : [];
  const key = JSON.stringify([domains, patterns]);
  const keyChanged = previous.rulesKey !== key;
  const nothingToBlock = domains.length === 0 && patterns.length === 0;

  let { rulesMode, rulesKey, patternRules, skippedPatterns } = previous;
  // Not blocking → make sure no rule is left, even when the cached key already says so (a rule that
  // survived a browser or extension restart, a cache out of sync). Costs one getDynamicRules per poll.
  if (keyChanged || nothingToBlock) {
    try {
      const installed = await installRules(domains, patterns);
      rulesMode = installed.mode;
      patternRules = installed.patternRules;
      skippedPatterns = installed.skippedPatterns;
      rulesKey = key;
      if (keyChanged) {
        console.info(
          LOG,
          `rules updated (${reason}): ${domains.length} domain(s), ${patterns.length} pattern(s) ` +
            `(${patternRules.length} as rules, ${skippedPatterns.length} skipped), mode ${rulesMode}`,
        );
      }
    } catch (err) {
      // Blocking still works through the tab listeners; the rules are retried on the next refresh.
      console.error(LOG, 'could not update blocking rules', err);
      rulesMode = nothingToBlock ? 'none' : 'tabs';
      patternRules = [];
      skippedPatterns = [...patterns];
      rulesKey = null;
    }
  }

  const next = { ...fetched, rulesMode, rulesKey, patternRules, skippedPatterns };
  await saveState(next);
  await updateBadge(next);
  await syncSkippedReport(next, key);

  const wasBlocking = isBlocking(previous);
  const oldDomains = new Set(wasBlocking ? previous.blocklist : []);
  const oldPatterns = new Set(wasBlocking ? previous.patterns : []);
  const newDomains = domains.filter((domain) => !oldDomains.has(domain));
  const newPatterns = patterns.filter((pattern) => !oldPatterns.has(pattern));
  let swept = false;
  if (newDomains.length > 0 || newPatterns.length > 0) {
    await sweepOpenTabs(newDomains, newPatterns, next);
    swept = true;
  }
  return { state: next, swept };
}

// ---------------------------------------------------------------------------
// declarativeNetRequest rules
// ---------------------------------------------------------------------------

/**
 * Atomically replaces all dynamic rules. `mode` is the variant in use: 'regex', 'path', 'none' (nothing
 * to block) or 'tabs' (nothing the browser accepted; the tab listeners block on their own).
 * @param {string[]} domains normalized ASCII domains
 * @param {string[]} patterns regex sources
 * @returns {Promise<{ mode: string, patternRules: string[], skippedPatterns: string[] }>}
 */
async function installRules(domains, patterns) {
  const dnr = chrome.declarativeNetRequest;
  const existing = await dnr.getDynamicRules();
  const removeRuleIds = existing.map((rule) => rule.id);
  if (domains.length === 0 && patterns.length === 0) {
    if (removeRuleIds.length > 0) {
      await dnr.updateDynamicRules({ removeRuleIds });
      console.info(LOG, `removed ${removeRuleIds.length} blocking rule(s)`);
    }
    return { mode: 'none', patternRules: [], skippedPatterns: [] };
  }

  // The domain rule's regexFilter (regex variant) counts towards the regex rule limit as well.
  const { accepted, skipped } = await screenPatterns(patterns, domains.length > 0 ? 1 : 0);
  if (domains.length === 0 && accepted.length === 0) {
    if (removeRuleIds.length > 0) await dnr.updateDynamicRules({ removeRuleIds });
    return { mode: 'tabs', patternRules: [], skippedPatterns: skipped };
  }

  // updateDynamicRules is atomic: a rejected attempt changes nothing. Prefer the variant that hands the
  // original URL to the page; if the browser still refuses some pattern rule, keep at least the domains.
  /** @type {Array<['regex' | 'path', string[]]>} */
  const attempts = [
    ['regex', accepted],
    ['path', accepted],
  ];
  if (domains.length > 0 && accepted.length > 0) attempts.push(['regex', []], ['path', []]);
  let lastError;
  for (const [mode, withPatterns] of attempts) {
    try {
      await dnr.updateDynamicRules({ removeRuleIds, addRules: buildRules(mode, domains, withPatterns) });
      const notInstalled = accepted.filter((pattern) => !withPatterns.includes(pattern));
      return { mode, patternRules: withPatterns, skippedPatterns: [...skipped, ...notInstalled] };
    } catch (err) {
      lastError = err;
      console.warn(LOG, `${mode} rules rejected (${withPatterns.length} pattern(s))`, err);
    }
  }
  throw lastError;
}

/**
 * Keeps the patterns the browser can turn into rules, in list order, up to the regex rule limit.
 * @param {string[]} patterns
 * @param {number} reserved regex rules already taken by the domain rule
 */
async function screenPatterns(patterns, reserved) {
  const max = chrome.declarativeNetRequest.MAX_NUMBER_OF_REGEX_RULES;
  const limit = Math.max(0, (Number.isInteger(max) && max > 0 ? max : DEFAULT_MAX_REGEX_RULES) - reserved);
  const supported = await Promise.all(patterns.map(isPatternSupported));
  /** @type {string[]} */
  const accepted = [];
  /** @type {string[]} */
  const skipped = [];
  patterns.forEach((pattern, index) => {
    if (supported[index] && accepted.length < limit) {
      accepted.push(pattern);
    } else {
      skipped.push(pattern);
    }
  });
  if (skipped.length > 0) console.warn(LOG, `${skipped.length} pattern(s) skipped`, skipped);
  return { accepted, skipped };
}

/**
 * Asks the browser whether the pattern is valid on its own and whether its rule regex compiles within the
 * limits (RE2 syntax, 2 KB memory). The first check keeps the wrapper honest: a malformed source such as
 * `a).*(b` would otherwise balance itself inside ruleRegexFilter and block something else.
 * When the API is missing or fails, the pattern is kept and updateDynamicRules decides.
 * @param {string} pattern
 */
async function isPatternSupported(pattern) {
  const dnr = chrome.declarativeNetRequest;
  if (typeof dnr.isRegexSupported !== 'function') return true;
  try {
    const [alone, asRule] = await Promise.all([
      dnr.isRegexSupported({ regex: pattern, isCaseSensitive: false }),
      dnr.isRegexSupported({ regex: ruleRegexFilter(pattern), isCaseSensitive: false, requireCapturing: true }),
    ]);
    const refused = [alone, asRule].find((result) => !result || result.isSupported !== true);
    if (refused === undefined) return true;
    console.warn(LOG, `pattern not supported by the browser (${refused?.reason ?? 'unknown'}):`, pattern);
    return false;
  } catch (err) {
    console.warn(LOG, 'isRegexSupported failed', err);
    return true;
  }
}

/**
 * @param {'regex' | 'path'} mode
 * @param {string[]} domains
 * @param {string[]} patterns
 */
function buildRules(mode, domains, patterns) {
  const rules = [];
  if (domains.length > 0) rules.push(domainRule(mode, domains));
  patterns.forEach((pattern, index) => rules.push(patternRule(mode, FIRST_PATTERN_RULE_ID + index, pattern)));
  return rules;
}

/** @param {'regex' | 'path'} mode */
function redirectAction(mode) {
  return mode === 'regex'
    ? // blocked.html#<original url>: `\1` is the whole URL in both rule kinds.
      { type: 'redirect', redirect: { regexSubstitution: `${chrome.runtime.getURL('blocked.html')}#\\1` } }
    : { type: 'redirect', redirect: { extensionPath: '/blocked.html' } };
}

/**
 * `requestDomains` also matches subdomains.
 * @param {'regex' | 'path'} mode
 * @param {string[]} domains
 */
function domainRule(mode, domains) {
  const condition =
    mode === 'regex'
      ? { regexFilter: '^(https?://.*)$', requestDomains: domains, resourceTypes: ['main_frame'] }
      : { requestDomains: domains, resourceTypes: ['main_frame'] };
  return { id: DOMAIN_RULE_ID, priority: 1, action: redirectAction(mode), condition };
}

/**
 * @param {'regex' | 'path'} mode
 * @param {number} id
 * @param {string} pattern
 */
function patternRule(mode, id, pattern) {
  return {
    id,
    priority: 1,
    action: redirectAction(mode),
    condition: {
      regexFilter: ruleRegexFilter(pattern),
      isUrlFilterCaseSensitive: false,
      resourceTypes: ['main_frame'],
    },
  };
}

// ---------------------------------------------------------------------------
// Skipped-pattern report (POST /v1/report)
// ---------------------------------------------------------------------------

/**
 * Tells the app which patterns got no declarativeNetRequest rule. Only a list screened for the lists being
 * blocked right now counts: a failed rule update leaves `rulesKey` null and is retried on the next refresh
 * instead of being reported as "unsupported". Outside focus the app sends no patterns, so nothing is
 * reported and the remembered list is dropped: the next focus run reports again (the app may have been
 * restarted meanwhile and lost it). Never throws.
 * @param {typeof INITIAL_STATE} state the state just applied
 * @param {string} key the rules key of the lists that state blocks
 */
async function syncSkippedReport(state, key) {
  try {
    if (!isBlocking(state)) {
      await forgetReportedSkipped();
      return;
    }
    if (state.rulesKey !== key) return;
    const skipped = [...state.skippedPatterns];
    const reportKey = JSON.stringify(skipped);
    if (reportKey === sendingReportKey || reportKey === (await loadReportedSkipped())) {
      cancelPendingReport();
      return;
    }
    if (pendingReport !== null && pendingReport.key === reportKey) return; // keep its timer running
    pendingReport = { key: reportKey, skipped };
    if (reportTimer !== null) clearTimeout(reportTimer);
    reportTimer = setTimeout(() => {
      sendPendingReport().catch(logError('report'));
    }, REPORT_DEBOUNCE_MS);
  } catch (err) {
    console.warn(LOG, 'could not schedule the skipped-pattern report', err);
  }
}

function cancelPendingReport() {
  pendingReport = null;
  if (reportTimer !== null) {
    clearTimeout(reportTimer);
    reportTimer = null;
  }
}

async function sendPendingReport() {
  reportTimer = null;
  const report = pendingReport;
  if (report === null) return;
  if (sendingReportKey !== null) {
    // One post at a time, so the app never receives an older list after a newer one.
    reportTimer = setTimeout(() => {
      sendPendingReport().catch(logError('report'));
    }, REPORT_DEBOUNCE_MS);
    return;
  }
  pendingReport = null;
  const epoch = reportEpoch;
  sendingReportKey = report.key;
  try {
    const answered = await postReport(report.skipped);
    // Not answered: nothing is remembered, so the next refresh in focus schedules the post again.
    if (answered && epoch === reportEpoch) await saveReportedSkipped(report.key);
  } finally {
    sendingReportKey = null;
  }
}

/**
 * Posts `{ skipped }` to the app. Returns true when the app answered at all: an older app without
 * `/v1/report` answers 404 and would refuse every retry as well. False when it could not be reached.
 * Never throws; a failed report does not affect blocking.
 * @param {string[]} skipped
 */
async function postReport(skipped) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const response = await fetch(BRIDGE_REPORT_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-FocusMode-Extension': '1' },
      body: JSON.stringify({ skipped }),
      cache: 'no-store',
      credentials: 'omit',
      redirect: 'error',
      signal: controller.signal,
    });
    if (response.ok) {
      console.info(LOG, `reported ${skipped.length} skipped pattern(s) to the app`);
    } else {
      console.info(LOG, `the app did not take the skipped-pattern report (http-${response.status})`);
    }
    return true;
  } catch (err) {
    const why = err && err.name === 'AbortError' ? 'timeout' : 'unreachable';
    console.info(LOG, `skipped-pattern report not delivered (${why})`);
    return false;
  } finally {
    clearTimeout(timer);
  }
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
  const url = rulesActive(state) ? tab.url || '' : tab.url || tab.pendingUrl || '';
  const match = matchBlockedEntry(url, state.blocklist, state.patterns);
  if (match !== null) {
    await redirectTab(tabId, url, ruleCovers(state, match));
  }
}

/** @param {typeof INITIAL_STATE} state */
function rulesActive(state) {
  return state.rulesMode === 'regex' || state.rulesMode === 'path';
}

/**
 * A declarativeNetRequest rule redirects this entry when the page is loaded from the network.
 * @param {typeof INITIAL_STATE} state
 * @param {{ kind: 'domain' | 'pattern', entry: string }} match
 */
function ruleCovers(state, match) {
  if (!rulesActive(state)) return false;
  return match.kind === 'domain' || state.patternRules.includes(match.entry);
}

/**
 * Sends already open tabs of newly blocked domains and patterns to the blocked page.
 * @param {string[]} domains
 * @param {string[]} patterns
 * @param {typeof INITIAL_STATE} state the state just applied
 */
async function sweepOpenTabs(domains, patterns, state) {
  const tabs = await chrome.tabs.query({ url: ['http://*/*', 'https://*/*'] });
  for (const tab of tabs) {
    // Discarded tabs are not loaded; the rule catches them when they are reloaded.
    if (tab.id === undefined || tab.discarded) continue;
    const url = tab.url || '';
    const match = matchBlockedEntry(url, domains, patterns);
    if (match === null) continue;
    try {
      await redirectTab(tab.id, url, ruleCovers(state, match));
    } catch (err) {
      console.warn(LOG, 'could not redirect an open tab', err);
    }
  }
}

/**
 * Reload lets the redirect rule replace the current history entry (so "back" does not loop); when no
 * rule covers the entry, or the rule demonstrably does not apply, navigate to blocked.html?u=<url> instead.
 * @param {number} tabId
 * @param {string} url
 * @param {boolean} ruleApplies a declarativeNetRequest rule should redirect this URL on reload
 */
async function redirectTab(tabId, url, ruleApplies) {
  const now = Date.now();
  const last = recentRedirects.get(tabId);
  const sameUrl = last !== undefined && last.url === url;
  if (sameUrl && now - last.at < VERIFY_DELAY_MS) return; // already being handled
  recentRedirects.set(tabId, { url, at: now });
  await rememberTabUrl(tabId, url);

  if (ruleApplies && !(sameUrl && now - last.at < REPEAT_WINDOW_MS)) {
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
  if (!isBlocking(state) || matchBlockedEntry(url, state.blocklist, state.patterns) === null) return;
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
      memoryState = withListDefaults(value);
      return memoryState;
    }
  } catch (err) {
    console.warn(LOG, 'could not read session storage', err);
  }
  return withListDefaults({ ...INITIAL_STATE });
}

/**
 * Fresh arrays, and the pattern fields a state stored by an older version of the extension lacks.
 * @param {typeof INITIAL_STATE} state
 */
function withListDefaults(state) {
  const strings = (value) => (Array.isArray(value) ? value.filter((item) => typeof item === 'string') : []);
  return {
    ...state,
    blocklist: strings(state.blocklist),
    patterns: normalizePatterns(state.patterns),
    patternRules: strings(state.patternRules),
    skippedPatterns: strings(state.skippedPatterns),
  };
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

/** @returns {Promise<string | null>} JSON of the skipped-pattern list the app has answered for, or null */
async function loadReportedSkipped() {
  if (reportedSkipped !== undefined) return reportedSkipped;
  let value = null;
  try {
    const stored = await chrome.storage.session.get(STORAGE_REPORTED_KEY);
    if (typeof stored[STORAGE_REPORTED_KEY] === 'string') value = stored[STORAGE_REPORTED_KEY];
  } catch (err) {
    console.warn(LOG, 'could not read session storage', err);
  }
  // A save that finished while this read was running wins.
  if (reportedSkipped === undefined) reportedSkipped = value;
  return reportedSkipped;
}

/** @param {string | null} reportKey */
async function saveReportedSkipped(reportKey) {
  reportedSkipped = reportKey;
  try {
    if (reportKey === null) {
      await chrome.storage.session.remove(STORAGE_REPORTED_KEY);
    } else {
      await chrome.storage.session.set({ [STORAGE_REPORTED_KEY]: reportKey });
    }
  } catch (err) {
    console.warn(LOG, 'could not write session storage', err);
  }
}

/** Outside focus: drop the pending post and the remembered list (an answer still on its way is ignored). */
async function forgetReportedSkipped() {
  cancelPendingReport();
  reportEpoch += 1;
  if ((await loadReportedSkipped()) !== null) await saveReportedSkipped(null);
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
