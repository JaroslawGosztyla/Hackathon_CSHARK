<script lang="ts">
  // Settings → Kalendarze: Microsoft 365 (the Teams sign-in), Google (OAuth desktop flow with the user's own client
  // id and secret) and any other calendar through ICS subscription links. Read-only everywhere. ICS links are secrets: they are
  // loaded only while the editor is open, kept in this component only, and never shown in errors.
  import Switch from "$lib/components/Switch.svelte";
  import TeamsSettings from "$lib/components/TeamsSettings.svelte";
  import { formatTimeOfDay } from "$lib/format";
  import { calendar, needsReconnect, parseIcsLines, redactLinks } from "$lib/state/calendar.svelte";
  import { CALENDAR_HORIZON_PRESETS, config } from "$lib/state/config.svelte";
  import { teams } from "$lib/state/teams.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";
  import type { CalendarSourceStatus, Config } from "$lib/types";

  const c = strings.calendar;

  const cfg = $derived(config.value);
  const cal = $derived(cfg?.calendar ?? null);
  const status = $derived(calendar.status);
  const anyOn = $derived(cal !== null && (cal.microsoft || cal.google || cal.ics));

  const microsoft = $derived(calendar.source("microsoft"));
  const google = $derived(calendar.source("google"));
  const ics = $derived(calendar.source("ics"));

  const horizonOptions = $derived.by((): number[] => {
    const list: number[] = [...CALENDAR_HORIZON_PRESETS];
    const current = cal?.horizonHours;
    if (current !== undefined && Number.isInteger(current) && !list.includes(current)) {
      list.push(current);
      list.sort((a, b) => a - b);
    }
    return list;
  });
  const horizonError = $derived(config.fieldErrors["calendar.horizonHours"] ?? null);

  // --- Microsoft ---
  /**
   * The Teams sign-in works but has no calendar permission (made for Teams only): signing in again with the calendar
   * switched on asks for it. The old sign-in (and Teams presence) keeps working until the new one succeeds.
   */
  const microsoftReconnect = $derived(needsReconnect(microsoft) && teams.status?.connected === true);

  /** Signs in to Microsoft again; the shell asks for `Calendars.Read` because the calendar is on. */
  function reconnectMicrosoft(): void {
    void teams.connect();
  }

  // --- Google ---
  const clientIdError = $derived(config.fieldErrors["calendar.googleClientId"] ?? null);
  const clientSecretError = $derived(config.fieldErrors["calendar.googleClientSecret"] ?? null);
  const googleConfigured = $derived((cal?.googleClientId.trim() ?? "") !== "");
  const googlePending = $derived(status?.googlePending ?? false);
  const googleConnected = $derived(google?.state === "on");
  /** A stored sign-in can be forgotten even when the last read failed. */
  const googleCanDisconnect = $derived(googleConnected || (google?.account ?? null) !== null);

  // --- ICS ---
  let icsEditing = $state(false);
  let icsText = $state("");
  let icsLoading = $state(false);
  let icsSaving = $state(false);
  let icsInvalid = $state<number[]>([]);
  /** Number of stored links, when the shell reports it as the source's `account`. */
  const icsCount = $derived.by((): number | null => {
    const account = ics?.account?.trim() ?? "";
    return /^\d+$/.test(account) ? Number(account) : null;
  });

  function set(mutate: (x: Config) => void, immediate = true): void {
    config.update(mutate, immediate);
  }

  function setHorizon(value: string): void {
    const hours = Number(value);
    if (Number.isInteger(hours)) set((x) => (x.calendar.horizonHours = hours));
  }

  function setClientId(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.value;
    set((x) => (x.calendar.googleClientId = value), false);
  }

  function setClientSecret(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.value;
    set((x) => (x.calendar.googleClientSecret = value), false);
  }

  /** `połączony · jan@firma.pl`; the detail of an error is shown separately. */
  function stateLine(source: CalendarSourceStatus | null): string | null {
    if (!source) return null;
    return c.statusLine(c.state[source.state], source.account);
  }

  /** English detail of a failing source, links removed. */
  function errorDetail(source: CalendarSourceStatus | null): string | null {
    if (source?.state !== "error") return null;
    const detail = redactLinks(source.detail ?? "").trim();
    return detail || null;
  }

  async function editIcs(): Promise<void> {
    if (icsLoading) return;
    icsLoading = true;
    try {
      const urls = await calendar.loadIcsUrls();
      if (urls === null) return;
      icsText = urls.join("\n");
      icsInvalid = [];
      icsEditing = true;
    } finally {
      icsLoading = false;
    }
  }

  function cancelIcs(): void {
    icsEditing = false;
    icsText = "";
    icsInvalid = [];
  }

  async function saveIcs(): Promise<void> {
    if (icsSaving) return;
    const { urls, invalidLines } = parseIcsLines(icsText);
    icsInvalid = invalidLines;
    if (invalidLines.length > 0) return;
    icsSaving = true;
    try {
      if (await calendar.saveIcsUrls(urls)) {
        cancelIcs();
        toasts.push(c.icsSaved, "info");
      }
    } finally {
      icsSaving = false;
    }
  }

  async function removeIcs(): Promise<void> {
    if (await calendar.disconnect("ics")) {
      cancelIcs();
      toasts.push(c.icsRemoved, "info");
    }
  }
</script>

{#if cfg && cal}
  <div class="calendars">
    <p class="hint lead">{c.lead}</p>

    <div class="items">
      <!-- Microsoft 365 / Outlook -->
      <div class="item">
        <Switch
          id="calendar-microsoft"
          label={c.microsoft}
          hint={c.microsoftHint}
          checked={cal.microsoft}
          onchange={(on) => set((x) => (x.calendar.microsoft = on))}
        >
          {#if cal.microsoft}
            <div class="stack">
              {#if microsoft}
                <p class="state" class:ok={microsoft.state === "on"} class:bad={microsoft.state === "error"}>
                  <span class="dot" aria-hidden="true"></span>
                  {stateLine(microsoft)}
                </p>
              {/if}
              {#if errorDetail(microsoft)}
                <p class="field-error">{c.errorLead} {errorDetail(microsoft)}</p>
              {/if}
              {#if microsoftReconnect}
                <div class="status-row" aria-live="polite">
                  <span class="hint">{c.microsoftNeedsReconnect}</span>
                  <button
                    type="button"
                    class="btn btn-sm btn-primary"
                    disabled={teams.connecting || teams.prompt !== null}
                    onclick={reconnectMicrosoft}
                  >
                    {teams.connecting ? c.microsoftReconnecting : c.microsoftReconnectButton}
                  </button>
                </div>
                {#if teams.prompt && cfg.shields.teamsDnd}
                  <!-- The full sign-in box is in the Teams shield above; the code is repeated here. -->
                  <p class="hint">{c.microsoftReconnectCode(teams.prompt.userCode)}</p>
                {/if}
              {/if}
              <p class="hint">{c.microsoftReconnect}</p>
              {#if cfg.shields.teamsDnd}
                <p class="hint">{c.microsoftTeamsAbove}</p>
              {:else}
                <TeamsSettings connectLabel={c.microsoftConnect} />
              {/if}
            </div>
          {/if}
        </Switch>
      </div>

      <!-- Google Calendar -->
      <div class="item">
        <Switch
          id="calendar-google"
          label={c.google}
          hint={c.googleHint}
          checked={cal.google}
          onchange={(on) => set((x) => (x.calendar.google = on))}
        >
          {#if cal.google}
            <div class="stack">
              <div class="field">
                <label for="google-client-id">{c.googleClientId}</label>
                <input
                  id="google-client-id"
                  class="input mono"
                  type="text"
                  spellcheck="false"
                  autocomplete="off"
                  placeholder={c.googleClientIdPlaceholder}
                  value={cal.googleClientId}
                  oninput={setClientId}
                  aria-invalid={clientIdError ? "true" : undefined}
                  aria-describedby={clientIdError ? "google-client-id-error" : undefined}
                />
                {#if clientIdError}<p class="field-error" id="google-client-id-error">{clientIdError}</p>{/if}
              </div>

              <div class="field">
                <label for="google-client-secret">{c.googleClientSecret}</label>
                <input
                  id="google-client-secret"
                  class="input mono"
                  type="text"
                  spellcheck="false"
                  autocomplete="off"
                  placeholder={c.googleClientSecretPlaceholder}
                  value={cal.googleClientSecret}
                  oninput={setClientSecret}
                  aria-invalid={clientSecretError ? "true" : undefined}
                  aria-describedby="google-client-secret-hint{clientSecretError ? ' google-client-secret-error' : ''}"
                />
                {#if clientSecretError}
                  <p class="field-error" id="google-client-secret-error">{clientSecretError}</p>
                {/if}
                <p class="hint" id="google-client-secret-hint">{c.googleClientSecretHint}</p>
              </div>

              <div class="status-row" aria-live="polite">
                {#if googleConnected}
                  <span class="state ok">
                    <span class="dot" aria-hidden="true"></span>
                    {c.googleConnected(google?.account ?? null)}
                  </span>
                {:else if googlePending}
                  <span class="state pending">
                    <span class="dot" aria-hidden="true"></span>
                    {c.googlePending}
                  </span>
                {:else}
                  <span class="state" class:bad={google?.state === "error"}>
                    <span class="dot" aria-hidden="true"></span>
                    {googleConfigured ? c.googleNotConnected : c.googleNeedsClientId}
                  </span>
                {/if}
                <span class="row">
                  {#if !googleConnected}
                    <button
                      type="button"
                      class="btn btn-sm btn-primary"
                      disabled={!googleConfigured ||
                        clientIdError !== null ||
                        clientSecretError !== null ||
                        googlePending ||
                        calendar.googleStarting}
                      onclick={() => calendar.connectGoogle()}
                    >
                      {c.googleConnect}
                    </button>
                  {/if}
                  {#if googleCanDisconnect}
                    <button type="button" class="btn btn-sm" onclick={() => calendar.disconnect("google")}>
                      {c.googleDisconnect}
                    </button>
                  {/if}
                </span>
              </div>

              {#if errorDetail(google)}
                <!-- The detail comes from Google / the shell in English; the lead line is Polish. -->
                <p class="field-error">{c.googleErrorLead} {errorDetail(google)}</p>
              {/if}

              <details class="help">
                <summary>{c.googleHelpTitle}</summary>
                <ol>
                  {#each c.googleHelp as step, i (i)}
                    <li>{step}</li>
                  {/each}
                </ol>
              </details>
            </div>
          {/if}
        </Switch>
      </div>

      <!-- ICS / iCal subscription links -->
      <div class="item">
        <Switch
          id="calendar-ics"
          label={c.ics}
          hint={c.icsHint}
          checked={cal.ics}
          onchange={(on) => set((x) => (x.calendar.ics = on))}
        >
          {#if cal.ics}
            <div class="stack">
              {#if ics}
                <p class="state" class:ok={ics.state === "on"} class:bad={ics.state === "error"}>
                  <span class="dot" aria-hidden="true"></span>
                  {icsCount !== null ? c.statusLine(c.state[ics.state], c.icsCount(icsCount)) : stateLine(ics)}
                </p>
              {/if}
              {#if errorDetail(ics)}
                <p class="field-error">{c.errorLead} {errorDetail(ics)}</p>
              {/if}

              {#if icsEditing}
                <div class="field">
                  <label for="ics-links">{c.icsLinks}</label>
                  <textarea
                    id="ics-links"
                    class="textarea"
                    rows="4"
                    spellcheck="false"
                    autocomplete="off"
                    placeholder={c.icsPlaceholder}
                    bind:value={icsText}
                    aria-invalid={icsInvalid.length > 0 ? "true" : undefined}
                    aria-describedby="ics-links-note{icsInvalid.length > 0 ? ' ics-links-error' : ''}"
                  ></textarea>
                  {#if icsInvalid.length > 0}
                    <p class="field-error" id="ics-links-error">{c.icsInvalidLines(icsInvalid)}</p>
                  {/if}
                  <p class="hint" id="ics-links-note">{c.icsSecret}</p>
                </div>
                <div class="row">
                  <button type="button" class="btn btn-sm btn-primary" disabled={icsSaving} onclick={saveIcs}>
                    {c.icsSave}
                  </button>
                  <button type="button" class="btn btn-sm" disabled={icsSaving} onclick={cancelIcs}>
                    {c.icsCancel}
                  </button>
                </div>
              {:else}
                <div class="row">
                  <button type="button" class="btn btn-sm btn-primary" disabled={icsLoading} onclick={editIcs}>
                    {icsLoading ? c.icsLoading : icsCount === 0 ? c.icsAdd : c.icsEdit}
                  </button>
                  {#if icsCount !== 0}
                    <button type="button" class="btn btn-sm btn-danger" onclick={removeIcs}>{c.icsRemoveAll}</button>
                  {/if}
                </div>
              {/if}

              <details class="help">
                <summary>{c.icsHelpTitle}</summary>
                <ul>
                  {#each c.icsHelp as entry (entry.name)}
                    <li><strong>{entry.name}:</strong> {entry.text}</li>
                  {/each}
                </ul>
              </details>
            </div>
          {/if}
        </Switch>
      </div>

      <!-- Look-ahead and sync -->
      <div class="item">
        <div class="field">
          <label for="calendar-horizon">{c.horizon}</label>
          <select
            id="calendar-horizon"
            class="input select narrow"
            value={cal.horizonHours}
            aria-describedby="calendar-horizon-hint"
            aria-invalid={horizonError ? "true" : undefined}
            onchange={(e) => setHorizon(e.currentTarget.value)}
          >
            {#each horizonOptions as hours (hours)}
              <option value={hours}>{c.horizonOption(hours)}</option>
            {/each}
          </select>
          {#if horizonError}<p class="field-error">{horizonError}</p>{/if}
          <p class="hint" id="calendar-horizon-hint">{c.horizonHint}</p>
        </div>

        {#if anyOn}
          <div class="sync row">
            <span class="hint tabular">
              {status?.lastSyncMs != null ? c.lastSync(formatTimeOfDay(status.lastSyncMs)) : c.neverSynced}
            </span>
            <button type="button" class="btn btn-sm" disabled={calendar.refreshing} onclick={() => calendar.refresh()}>
              {calendar.refreshing ? c.refreshing : c.refresh}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <p class="hint privacy">{c.privacy}</p>
  </div>
{/if}

<style>
  .calendars {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .items {
    display: flex;
    flex-direction: column;
  }

  .item {
    padding: 16px 0;
    border-top: 1px solid var(--border);
  }

  .item:first-child {
    padding-top: 0;
    border-top: 0;
  }

  .item:last-child {
    padding-bottom: 0;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 0.88rem;
  }

  .narrow {
    max-width: 340px;
  }

  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted-mark);
  }

  .state.ok .dot {
    background: var(--ok);
  }

  .state.pending .dot {
    background: var(--accent);
  }

  .state.bad .dot {
    background: var(--danger);
  }

  .sync {
    margin-top: 12px;
  }

  .help {
    color: var(--text-2);
    font-size: 0.9rem;
  }

  .help summary {
    cursor: pointer;
    font-weight: 600;
    color: var(--accent-strong);
    width: fit-content;
  }

  .help ol,
  .help ul {
    margin: 10px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .help strong {
    color: var(--text);
  }

  .privacy {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
