// Toolbar popup: connection with the app, current phase with a live countdown, blocked domain count.

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
  domains: /** @type {HTMLDetailsElement} */ (byId('domains')),
  domainList: byId('domain-list'),
  refresh: /** @type {HTMLButtonElement} */ (byId('refresh')),
};

let renderedDomainsKey = '';

/** @param {HTMLElement} element @param {string} text */
function setText(element, text) {
  if (element.textContent !== text) element.textContent = text;
}

/** @param {string[]} domains */
function renderDomains(domains) {
  const key = domains.join(',');
  if (key === renderedDomainsKey) return;
  renderedDomainsKey = key;
  ui.domainList.replaceChildren(
    ...domains.map((domain) => {
      const item = document.createElement('li');
      item.textContent = domain;
      return item;
    }),
  );
  ui.domains.hidden = domains.length === 0;
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
    renderDomains([]);
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
  setText(ui.domainCount, String(state.blocklist.length));
  renderDomains(state.blocklist);
}

const watcher = watchState({ pollMs: 5000, onView: render });

ui.refresh.addEventListener('click', () => {
  setText(ui.connectionText, TEXT.checking);
  watcher.refresh();
});

window.addEventListener('pagehide', () => watcher.stop());
