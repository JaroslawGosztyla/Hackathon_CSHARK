// Toolbar popup: connection with the app, current phase with a live countdown, how many domains and regex
// patterns are blocked (and the list itself).

import { TEXT, formatClock, isBlocking, phaseLabel, watchState } from './common.js';

/** @param {string} id */
function byId(id) {
  const element = document.getElementById(id);
  if (element === null) throw new Error(`popup.html is missing #${id}`);
  return element;
}

const ui = {
  main: /** @type {HTMLElement} */ (document.querySelector('.popup__main')),
  connectionText: byId('connection-text'),
  phase: byId('phase'),
  clock: byId('clock'),
  hint: byId('phase-hint'),
  blocking: byId('blocking'),
  domainCount: byId('domain-count'),
  patternCount: byId('pattern-count'),
  domains: /** @type {HTMLDetailsElement} */ (byId('domains')),
  domainList: byId('domain-list'),
  refresh: /** @type {HTMLButtonElement} */ (byId('refresh')),
};

const SKIPPED_HINT = 'Przeglądarka nie obsługuje tego wzorca albo przekroczono limit reguł — nie jest blokowany.';

let renderedListKey = '';

/** @param {HTMLElement} element @param {string} text */
function setText(element, text) {
  if (element.textContent !== text) element.textContent = text;
}

/**
 * @param {string[]} domains
 * @param {string[]} patterns
 * @param {string[]} skipped patterns the browser did not accept as rules
 */
function renderList(domains, patterns, skipped) {
  const key = JSON.stringify([domains, patterns, skipped]);
  if (key === renderedListKey) return;
  renderedListKey = key;
  const skippedSet = new Set(skipped);
  ui.domainList.replaceChildren(
    ...domains.map((domain) => {
      const item = document.createElement('li');
      item.textContent = domain;
      return item;
    }),
    ...patterns.map((pattern) => {
      const item = document.createElement('li');
      item.className = 'domains__pattern';
      const code = document.createElement('code');
      code.textContent = pattern;
      item.append(code);
      if (skippedSet.has(pattern)) {
        item.classList.add('domains__pattern--skipped');
        item.title = SKIPPED_HINT;
      }
      return item;
    }),
  );
  ui.domains.hidden = domains.length === 0 && patterns.length === 0;
}

/** @param {number} total @param {number} skipped */
function patternCountText(total, skipped) {
  return skipped > 0 ? `${total} (pominięte: ${skipped})` : String(total);
}

/**
 * @param {{ state: typeof import('./common.js').INITIAL_STATE | null, remainingSecs: number, paused: boolean }} view
 */
function render({ state, remainingSecs, paused }) {
  if (state === null || !state.connected) {
    ui.main.dataset.mode = 'offline';
    setText(ui.connectionText, TEXT.disconnected);
    setText(ui.phase, 'Blokada wyłączona');
    setText(ui.clock, '--:--');
    setText(
      ui.hint,
      state === null
        ? 'Nie udało się odczytać stanu rozszerzenia. Spróbuj odświeżyć.'
        : 'Uruchom aplikację FocusMode. Bez niej żadna strona nie jest blokowana.',
    );
    setText(ui.blocking, 'wyłączone');
    setText(ui.domainCount, '—');
    setText(ui.patternCount, '—');
    ui.patternCount.title = '';
    renderList([], [], []);
    return;
  }

  const phase = state.phase ?? 'idle';
  ui.main.dataset.mode = phase;
  setText(ui.connectionText, TEXT.connected);
  setText(ui.phase, paused ? `${phaseLabel(phase)} · ${TEXT.paused}` : phaseLabel(phase));
  setText(ui.clock, phase === 'idle' ? '--:--' : formatClock(remainingSecs));

  const blocking = isBlocking(state);
  if (phase === 'focus') {
    setText(
      ui.hint,
      blocking ? 'pozostało do przerwy' : 'pozostało do przerwy · blokowanie stron wyłączone w aplikacji',
    );
  } else if (phase === 'break') {
    setText(ui.hint, 'pozostało do końca przerwy · strony są odblokowane');
  } else {
    setText(ui.hint, 'Rozpocznij sesję skupienia w aplikacji FocusMode.');
  }

  setText(ui.blocking, blocking ? 'aktywne' : phase === 'break' ? 'wyłączone (przerwa)' : 'wyłączone');
  // The app sends the list only while it blocks, so outside focus both counts are 0.
  const patterns = Array.isArray(state.patterns) ? state.patterns : [];
  const skipped = Array.isArray(state.skippedPatterns) ? state.skippedPatterns : [];
  setText(ui.domainCount, String(state.blocklist.length));
  setText(ui.patternCount, patternCountText(patterns.length, skipped.length));
  ui.patternCount.title = skipped.length > 0 ? SKIPPED_HINT : '';
  renderList(state.blocklist, patterns, skipped);
}

const watcher = watchState({ pollMs: 5000, onView: render });

ui.refresh.addEventListener('click', () => {
  setText(ui.connectionText, TEXT.checking);
  watcher.refresh();
});

window.addEventListener('pagehide', () => watcher.stop());
