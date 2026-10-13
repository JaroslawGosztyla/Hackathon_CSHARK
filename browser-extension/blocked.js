// "Wracaj do pracy" page shown instead of a blocked site during focus.
//
// The original address arrives as the fragment (declarativeNetRequest regexSubstitution rule), as `?u=`
// (tab fallback, together with `via=tab`) or from the service worker's per-tab memory. During the break,
// when no session runs or when the app is not running, the page offers the original address again and,
// after two consecutive polls that say "not blocking", sends the tab back there by itself: the first such
// poll shows the "Przerwa!" card and asks the worker for an immediate second poll (MSG_POLL_NOW, which
// also drops the blocking rules), and the tab leaves once that one agrees and ~1.5 s have passed.

import {
  formatClock,
  hasUncheckablePatterns,
  hostOf,
  isBlocking,
  localizePage,
  matchBlockedEntry,
  parseWebUrl,
  t,
  watchState,
} from './common.js';

localizePage(document);

/** One of these encouraging lines (_locales: focusLine1…6) is picked per page. */
const FOCUS_LINES = [1, 2, 3, 4, 5, 6].map((n) => t(`focusLine${n}`));
const BREAK_LINE = t('lineBreak');
const IDLE_LINE = t('lineIdle');
const OFFLINE_LINE = t('lineOffline');
const UNBLOCKED_LINE = t('lineUnblocked');

/** @param {string} id */
function byId(id) {
  const element = document.getElementById(id);
  if (element === null) throw new Error(`blocked.html is missing #${id}`);
  return element;
}

const ui = {
  card: byId('card'),
  headline: byId('headline'),
  emoji: byId('headline-emoji'),
  site: byId('site'),
  match: byId('match'),
  matchLabel: byId('match-label'),
  matchEntry: byId('match-entry'),
  timer: byId('timer'),
  timerLabel: byId('timer-label'),
  timerClock: byId('timer-clock'),
  line: byId('line'),
  open: /** @type {HTMLAnchorElement} */ (byId('open')),
  back: /** @type {HTMLButtonElement} */ (byId('back')),
  status: byId('status'),
};

/**
 * After "Go back" (`back`), close the tab only if no navigation started by then (history.go() silently does nothing
 * when there is no entry that far back).
 */
const LEAVE_FALLBACK_MS = 1200;

/** Longer blocklist entries (regex patterns) are shortened on the card; the full text is in the tooltip. */
const MAX_ENTRY_CHARS = 120;

/** How long the "you may open the page" card stays before the tab goes back to the site by itself. */
const AUTO_RETURN_DELAY_MS = 1500;
/** Consecutive polls that must report "not blocking" (break, idle, app not running) before that. */
const AUTO_RETURN_POLLS = 2;
/** sessionStorage (this tab, extension origin): the last automatic return, `{ url, at }`. */
const AUTO_RETURN_STORAGE_KEY = 'focusmode:auto-return';
/**
 * The same address landing back here this soon after an automatic return means it is still being
 * redirected (a rule the worker could not remove); stop and leave the "Open the site" link (`openSite`) instead.
 */
const AUTO_RETURN_LOOP_MS = 20000;

const params = new URLSearchParams(location.search);
/** Reached through tabs.update (new history entry) rather than a rule redirect (entry replaced). */
const viaTab = params.get('via') === 'tab';
/**
 * The rules only ever send a top-level tab here. The page is web accessible, so a site could frame it;
 * a framed copy must not drive the tab (history, closing it).
 */
const framed = window.top !== window;
const focusLine = FOCUS_LINES[Math.floor(Math.random() * FOCUS_LINES.length)] ?? FOCUS_LINES[0];

/** @type {string | null} */
let originalUrl = urlFromLocation();
let currentMode = 'loading';
/** @type {ReturnType<typeof setTimeout> | undefined} */
let leaveFallbackTimer;
/** The document started navigating away (beforeunload also fires for history traversals). */
let leaving = false;
/** @type {ReturnType<typeof import('./common.js').watchState> | null} */
let watcher = null;
/** `pollSeq` of the last poll answer taken into account (countdown ticks repeat it). */
let lastPollSeq = 0;
/** Consecutive poll answers in which this page should not block. */
let freePolls = 0;
/** The card has been shown for AUTO_RETURN_DELAY_MS since the first such answer. */
let autoReturnDelayDone = false;
/** @type {ReturnType<typeof setTimeout> | undefined} */
let autoReturnTimer;
/** The tab is already going back to the site. */
let autoReturnStarted = false;
/** The loop guard said no; stays until focus comes back. */
let autoReturnRefused = false;
/** The user chose where to go ("Go back" or "Open the site"); never navigate on top of that. */
let userLeft = false;

/** @returns {string | null} */
function urlFromLocation() {
  const fromHash = location.hash.length > 1 ? parseWebUrl(location.hash.slice(1)) : null;
  if (fromHash !== null) return fromHash.href;
  const fromQuery = parseWebUrl(params.get('u'));
  return fromQuery !== null ? fromQuery.href : null;
}

/** "www.reddit.com" → "reddit.com" for display. */
function displayHost() {
  const host = hostOf(originalUrl);
  if (host === null) return null;
  return host.startsWith('www.') && host.indexOf('.', 4) !== -1 ? host.slice(4) : host;
}

/** @param {HTMLElement} element @param {string} text */
function setText(element, text) {
  if (element.textContent !== text) element.textContent = text;
}

/**
 * @param {typeof import('./common.js').INITIAL_STATE | null} state
 * @returns {'focus' | 'break' | 'idle' | 'offline' | 'unblocked'}
 */
function modeFor(state) {
  if (state === null || !state.connected) return 'offline';
  if (state.phase === 'focus') {
    // A pattern JavaScript cannot evaluate may still be what the browser rule matched: keep blocking
    // rather than sending the tab back into the rule.
    const siteBlocked =
      originalUrl === null ||
      matchBlockedEntry(originalUrl, state.blocklist, state.patterns) !== null ||
      hasUncheckablePatterns(state.patterns);
    return isBlocking(state) && siteBlocked ? 'focus' : 'unblocked';
  }
  return state.phase === 'break' ? 'break' : 'idle';
}

/**
 * Shows which line of websiteblacklist.txt blocks the page (focus only; the app sends no list otherwise).
 * @param {typeof import('./common.js').INITIAL_STATE | null} state
 * @param {string} mode
 */
function renderMatch(state, mode) {
  const match =
    mode === 'focus' && state !== null && originalUrl !== null
      ? matchBlockedEntry(originalUrl, state.blocklist, state.patterns)
      : null;
  ui.match.hidden = match === null;
  if (match === null) return;
  setText(ui.matchLabel, match.kind === 'domain' ? t('matchDomain') : t('matchPattern'));
  const shown =
    match.entry.length > MAX_ENTRY_CHARS ? `${match.entry.slice(0, MAX_ENTRY_CHARS - 1)}…` : match.entry;
  setText(ui.matchEntry, shown);
  ui.matchEntry.title = shown === match.entry ? '' : match.entry;
}

/**
 * @param {{ state: typeof import('./common.js').INITIAL_STATE | null, originalUrl: string | null, remainingSecs: number, paused: boolean, pollSeq: number }} view
 */
function render(view) {
  if (originalUrl === null && view.originalUrl !== null) {
    const remembered = parseWebUrl(view.originalUrl);
    if (remembered !== null) originalUrl = remembered.href;
  }

  const mode = modeFor(view.state);
  const host = displayHost();
  const clock = formatClock(view.remainingSecs);
  ui.card.dataset.mode = mode;

  switch (mode) {
    case 'focus':
      setText(ui.headline, t('headlineFocus'));
      setText(ui.emoji, '🎧');
      setText(ui.site, host !== null ? t('siteHostWaits', host) : t('siteWaits'));
      setText(ui.timerLabel, view.paused ? t('timerFocusPaused') : t('timerFocus'));
      setText(ui.line, focusLine);
      document.title = t('titleFocus', clock);
      break;
    case 'break':
      setText(ui.headline, t('headlineBreak'));
      setText(ui.emoji, '☕');
      setText(ui.site, host !== null ? t('siteHostBreak', host) : t('siteBreak'));
      setText(ui.timerLabel, view.paused ? t('timerBreakPaused') : t('timerBreak'));
      setText(ui.line, BREAK_LINE);
      document.title = t('titleBreak', clock);
      break;
    case 'idle':
      setText(ui.headline, t('headlineIdle'));
      setText(ui.emoji, '🌙');
      setText(ui.site, host !== null ? t('siteHostNotBlocked', host) : t('sitesNotBlocked'));
      setText(ui.line, IDLE_LINE);
      document.title = t('titleUnblocked');
      break;
    case 'unblocked':
      setText(ui.headline, t('headlineUnblocked'));
      setText(ui.emoji, '🌙');
      setText(ui.site, host !== null ? t('siteHostNotBlocked', host) : t('thisSiteNotBlocked'));
      setText(ui.line, UNBLOCKED_LINE);
      document.title = t('titleUnblocked');
      break;
    default:
      setText(ui.headline, t('blockingOff'));
      setText(ui.emoji, '🌙');
      setText(ui.site, host !== null ? t('siteHostNotBlocked', host) : t('sitesNotBlocked'));
      setText(ui.line, OFFLINE_LINE);
      document.title = t('titleOffline');
      break;
  }

  ui.timer.hidden = mode !== 'focus' && mode !== 'break';
  setText(ui.timerClock, clock);
  renderMatch(view.state, mode);

  const canOpen = mode !== 'focus' && originalUrl !== null;
  ui.open.hidden = !canOpen;
  if (canOpen && ui.open.getAttribute('href') !== originalUrl) ui.open.href = originalUrl;
  ui.back.classList.toggle('btn--primary', !canOpen);
  ui.back.classList.toggle('btn--ghost', canOpen);

  if (mode !== currentMode) {
    const previous = currentMode;
    currentMode = mode;
    announce(mode, previous);
  }

  trackAutoReturn(view.pollSeq, mode);
}

/**
 * Screen-reader friendly notice about phase changes (the status line is aria-live).
 * @param {string} mode
 * @param {string} previous
 */
function announce(mode, previous) {
  if (mode === 'focus') {
    setText(ui.status, previous === 'loading' ? '' : t('statusBackToFocus'));
    return;
  }
  if (originalUrl === null) {
    setText(ui.status, t('statusNoAddress'));
    return;
  }
  if (autoReturnStarted || autoReturnRefused) return; // keeps "Wracasz na stronę…" or the loop-guard notice
  const host = displayHost();
  setText(ui.status, host !== null ? t('statusReturnHost', host) : t('statusReturn'));
}

// ---------------------------------------------------------------------------
// Going back to the site automatically when blocking ends
// ---------------------------------------------------------------------------

/**
 * Counts consecutive poll answers in which this page should not block and sends the tab back to the
 * original address after AUTO_RETURN_POLLS of them, once the card has been visible for AUTO_RETURN_DELAY_MS.
 * @param {number} pollSeq
 * @param {string} mode
 */
function trackAutoReturn(pollSeq, mode) {
  if (pollSeq === lastPollSeq) return; // a countdown tick, not a new answer
  lastPollSeq = pollSeq;
  if (mode === 'focus' || mode === 'loading') {
    resetAutoReturn();
    return;
  }
  freePolls += 1;
  if (freePolls === 1) {
    // Second opinion right away: the worker asks the app again and drops its rules before answering,
    // so the site is reachable by the time the tab goes back there.
    watcher?.pollNow();
    autoReturnDelayDone = false;
    clearAutoReturnTimer();
    autoReturnTimer = setTimeout(() => {
      autoReturnTimer = undefined;
      autoReturnDelayDone = true;
      maybeAutoReturn();
    }, AUTO_RETURN_DELAY_MS);
  }
  maybeAutoReturn();
}

function maybeAutoReturn() {
  if (autoReturnStarted || autoReturnRefused || userLeft || leaving || framed) return;
  if (freePolls < AUTO_RETURN_POLLS || !autoReturnDelayDone) return;
  if (currentMode === 'focus' || currentMode === 'loading') return;
  // Only ever an http(s) address (originalUrl is always parsed that way; checked again on purpose).
  const target = parseWebUrl(originalUrl);
  if (target === null) return;
  if (!claimAutoReturn(target.href)) {
    autoReturnRefused = true;
    setText(ui.status, t('statusLoop'));
    return;
  }
  autoReturnStarted = true;
  setText(ui.status, t('statusReturning'));
  watcher?.stop();
  // replace(): the blocked page leaves the history, so "back" from the site does not land here again.
  location.replace(target.href);
}

/** Back in focus (or the page came back from the back/forward cache): start counting from zero. */
function resetAutoReturn() {
  freePolls = 0;
  autoReturnDelayDone = false;
  autoReturnRefused = false;
  clearAutoReturnTimer();
}

function clearAutoReturnTimer() {
  if (autoReturnTimer !== undefined) {
    clearTimeout(autoReturnTimer);
    autoReturnTimer = undefined;
  }
}

/**
 * Loop guard: records this automatic return in the tab's sessionStorage and refuses a second one to the
 * same address within AUTO_RETURN_LOOP_MS. Storage errors never block the return.
 * @param {string} url
 * @returns {boolean} the return may go ahead
 */
function claimAutoReturn(url) {
  const now = Date.now();
  try {
    const raw = sessionStorage.getItem(AUTO_RETURN_STORAGE_KEY);
    const last = raw === null ? null : JSON.parse(raw);
    if (
      last !== null &&
      typeof last === 'object' &&
      last.url === url &&
      typeof last.at === 'number' &&
      now - last.at >= 0 &&
      now - last.at < AUTO_RETURN_LOOP_MS
    ) {
      return false;
    }
  } catch (err) {
    console.warn('[FocusMode] could not read the auto-return marker', err);
  }
  try {
    sessionStorage.setItem(AUTO_RETURN_STORAGE_KEY, JSON.stringify({ url, at: now }));
  } catch (err) {
    console.warn('[FocusMode] could not store the auto-return marker', err);
  }
  return true;
}

/** Leaves the blocked page: back in history, or close the tab / open a new tab page when there is no history. */
function goBack() {
  userLeft = true;
  clearAutoReturnTimer();
  // A rule redirect replaced the blocked entry, so one step back is the page before it. The tab fallback
  // added a new entry after the blocked site, so skip that one as well.
  const steps = viaTab ? -2 : -1;
  if (history.length > Math.abs(steps)) {
    leaving = false;
    clearLeaveFallback();
    // A slow back navigation keeps this document alive until the previous page commits; closing the
    // tab then would throw the user out instead of taking them back.
    leaveFallbackTimer = setTimeout(() => {
      leaveFallbackTimer = undefined;
      if (!leaving) void leaveTab();
    }, LEAVE_FALLBACK_MS);
    history.go(steps);
    return;
  }
  void leaveTab();
}

function clearLeaveFallback() {
  if (leaveFallbackTimer !== undefined) {
    clearTimeout(leaveFallbackTimer);
    leaveFallbackTimer = undefined;
  }
}

async function leaveTab() {
  try {
    const tab = await chrome.tabs.getCurrent();
    if (tab === undefined || tab.id === undefined) {
      window.close();
      return;
    }
    const siblings = await chrome.tabs.query({ windowId: tab.windowId });
    if (siblings.length > 1) {
      await chrome.tabs.remove(tab.id);
    } else {
      await chrome.tabs.update(tab.id, { url: 'chrome://newtab/' });
    }
  } catch (err) {
    console.warn('[FocusMode] could not leave the blocked page', err);
    location.replace('about:blank');
  }
}

async function start() {
  /** @type {number | undefined} */
  let tabId;
  try {
    const tab = await chrome.tabs.getCurrent();
    tabId = tab?.id;
  } catch (err) {
    console.warn('[FocusMode] could not identify the tab', err);
  }
  watcher = watchState({ tabId, pollMs: 5000, onView: render });
}

if (framed) {
  ui.card.hidden = true;
} else {
  ui.back.addEventListener('click', goBack);
  ui.open.addEventListener('click', (event) => {
    // A plain click navigates this tab; a modified click opens another tab and this page stays.
    if (event.button === 0 && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.altKey) {
      userLeft = true;
      clearAutoReturnTimer();
    }
  });
  // No preventDefault: this only notes that a navigation started, it never shows a "leave site?" prompt.
  window.addEventListener('beforeunload', () => {
    leaving = true;
  });
  window.addEventListener('pagehide', () => {
    clearLeaveFallback();
    clearAutoReturnTimer();
  });
  window.addEventListener('pageshow', (event) => {
    // Restored from the back/forward cache: a pending fallback must not close the tab now, and the
    // automatic return starts counting again from fresh answers.
    if (event.persisted) {
      leaving = false;
      userLeft = false;
      clearLeaveFallback();
      resetAutoReturn();
      watcher?.refresh();
    }
  });
  void start();
}
