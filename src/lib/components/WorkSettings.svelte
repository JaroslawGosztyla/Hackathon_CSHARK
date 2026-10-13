<script lang="ts">
  // Settings → Narzędzia pracy (Co dalej?): GitHub, GitLab, Jira, Azure DevOps and Linear, each with a switch, the
  // fields it needs, its read-only access token and a short how-to with the minimal scopes. Tokens are secrets: they
  // live only in this component's inputs until "Zapisz token" sends them (the shell stores them encrypted, outside
  // config.json), are cleared right after, and are never shown, logged or put into error texts.
  import Switch from "$lib/components/Switch.svelte";
  import { formatTimeOfDay } from "$lib/format";
  import { config } from "$lib/state/config.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import {
    hasToken,
    isValidToken,
    noSecretStore,
    notSyncedYet,
    redactDetail,
    sourceEnabled,
    WORK_SOURCES,
    WORK_SWITCH,
    work,
  } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";
  import type { Config, WorkSource, WorkSourceStatus } from "$lib/types";

  const w = strings.work;

  type TextField = "gitlabUrl" | "jiraUrl" | "jiraEmail" | "azureOrg";

  const cfg = $derived(config.value);
  const wcfg = $derived((cfg?.work as Config["work"] | undefined) ?? null);
  const status = $derived(work.status);
  const anyOn = $derived(WORK_SOURCES.some((source) => sourceEnabled(cfg, source)));

  /** Token inputs (component state only; emptied after a save and when a tool is switched off). */
  let tokens = $state<Record<WorkSource, string>>({ github: "", gitlab: "", jira: "", azureDevops: "", linear: "" });

  function set(mutate: (x: Config) => void, immediate = true): void {
    config.update(mutate, immediate);
  }

  function toggle(source: WorkSource, on: boolean): void {
    if (!on) tokens[source] = "";
    set((x) => (x.work[WORK_SWITCH[source]] = on));
  }

  function setText(field: TextField, event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.value;
    set((x) => (x.work[field] = value), false);
  }

  function fieldError(field: TextField): string | null {
    return config.fieldErrors[`work.${field}`] ?? null;
  }

  /** The address / organisation the tool needs is still empty. */
  function missingSetting(source: WorkSource): string | null {
    if (!wcfg) return null;
    if (source === "gitlab" && wcfg.gitlabUrl.trim() === "") return w.needsUrl.gitlab;
    if (source === "jira" && wcfg.jiraUrl.trim() === "") return w.needsUrl.jira;
    if (source === "azureDevops" && wcfg.azureOrg.trim() === "") return w.needsUrl.azureDevops;
    return null;
  }

  type StateKind = "ok" | "bad" | "none";

  function stateOf(source: WorkSource, s: WorkSourceStatus | null): { text: string; kind: StateKind } {
    if (noSecretStore(s)) return { text: w.noSecretStore, kind: "bad" };
    const missing = missingSetting(source);
    if (missing) return { text: missing, kind: "none" };
    switch (s?.state) {
      case "on":
        return notSyncedYet(s) && !s.account ? { text: w.syncing, kind: "none" } : { text: w.connectedAs(s.account), kind: "ok" };
      case "error":
        return { text: s.account ? `${w.state.error} · ${s.account}` : w.state.error, kind: "bad" };
      case "off":
        return { text: w.state.off, kind: "none" };
      default:
        return { text: w.notConnected, kind: "none" };
    }
  }

  /** English detail of a failing tool, links and token-like strings removed. */
  function errorDetail(s: WorkSourceStatus | null): string | null {
    if (s?.state !== "error") return null;
    return redactDetail(s.detail ?? "") || null;
  }

  /** A pasted token that the shell would refuse (shown inline; the token itself is never echoed). */
  function tokenError(source: WorkSource): string | null {
    const token = tokens[source].trim();
    return token !== "" && !isValidToken(token) ? w.tokenInvalid : null;
  }

  async function saveToken(source: WorkSource): Promise<void> {
    const token = tokens[source].trim();
    if (!isValidToken(token)) return;
    if (await work.setToken(source, token)) {
      tokens[source] = "";
      toasts.push(w.tokenSaved(w.sourceName[source]), "info");
    }
  }

  async function removeToken(source: WorkSource): Promise<void> {
    if (await work.setToken(source, null)) {
      tokens[source] = "";
      toasts.push(w.tokenRemoved(w.sourceName[source]), "info");
    }
  }
</script>

{#snippet textField(field: TextField, label: string, placeholder: string, hint: string, inputmode: "url" | "email" | "text")}
  {@const error = fieldError(field)}
  <div class="field">
    <label for="work-{field}">{label}</label>
    <input
      id="work-{field}"
      class="input mono"
      type="text"
      {inputmode}
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      {placeholder}
      value={wcfg?.[field] ?? ""}
      oninput={(e) => setText(field, e)}
      aria-invalid={error ? "true" : undefined}
      aria-describedby="work-{field}-hint{error ? ` work-${field}-error` : ''}"
    />
    {#if error}<p class="field-error" id="work-{field}-error">{error}</p>{/if}
    <p class="hint" id="work-{field}-hint">{hint}</p>
  </div>
{/snippet}

{#if cfg && wcfg}
  <div class="work">
    <p class="hint lead">{w.lead}</p>

    <div class="items">
      {#each WORK_SOURCES as source (source)}
        {@const on = sourceEnabled(cfg, source)}
        {@const name = w.sourceName[source]}
        {@const s = work.source(source)}
        {@const shown = stateOf(source, s)}
        {@const detail = errorDetail(s)}
        {@const stored = hasToken(s)}
        {@const busy = work.tokenBusy[source] === true}
        {@const badToken = tokenError(source)}
        {@const noStore = noSecretStore(s)}
        <div class="item">
          <Switch
            id="work-{source}"
            label={name}
            hint={w.sourceHint[source]}
            checked={on}
            onchange={(value) => toggle(source, value)}
          >
            {#if on}
              <div class="stack">
                {#if source === "gitlab"}
                  {@render textField("gitlabUrl", w.gitlabUrl, w.gitlabUrlPlaceholder, w.gitlabUrlHint, "url")}
                {:else if source === "jira"}
                  {@render textField("jiraUrl", w.jiraUrl, w.jiraUrlPlaceholder, w.jiraUrlHint, "url")}
                  {@render textField("jiraEmail", w.jiraEmail, w.jiraEmailPlaceholder, w.jiraEmailHint, "email")}
                {:else if source === "azureDevops"}
                  {@render textField("azureOrg", w.azureOrg, w.azureOrgPlaceholder, w.azureOrgHint, "text")}
                {/if}

                <p class="state" class:ok={shown.kind === "ok"} class:bad={shown.kind === "bad"} aria-live="polite">
                  <span class="dot" aria-hidden="true"></span>
                  {shown.text}
                </p>
                {#if detail}
                  <!-- The detail comes from the tool / the shell in English; the lead line is Polish. -->
                  <p class="field-error">{w.errorLead} {detail}</p>
                {/if}

                <form
                  class="field"
                  onsubmit={(e) => {
                    e.preventDefault();
                    void saveToken(source);
                  }}
                >
                  <label for="work-{source}-token">{w.token(name)}</label>
                  <div class="token-row">
                    <input
                      id="work-{source}-token"
                      class="input mono token"
                      type="password"
                      spellcheck="false"
                      autocomplete="off"
                      autocapitalize="off"
                      placeholder={stored ? w.tokenPlaceholderReplace : w.tokenPlaceholder}
                      disabled={noStore}
                      bind:value={tokens[source]}
                      aria-invalid={badToken ? "true" : undefined}
                      aria-describedby="work-{source}-token-hint{badToken ? ` work-${source}-token-error` : ''}"
                    />
                    <button
                      type="submit"
                      class="btn btn-sm btn-primary"
                      disabled={busy || noStore || badToken !== null || tokens[source].trim() === ""}
                    >
                      {busy ? w.tokenSaving : w.tokenSave}
                    </button>
                    {#if stored}
                      <button type="button" class="btn btn-sm btn-danger" disabled={busy} onclick={() => removeToken(source)}>
                        {w.tokenRemove}
                      </button>
                    {/if}
                  </div>
                  {#if badToken}<p class="field-error" id="work-{source}-token-error">{badToken}</p>{/if}
                  <p class="hint" id="work-{source}-token-hint">{w.tokenHint}</p>
                </form>

                <details class="help">
                  <summary>{w.helpTitle}</summary>
                  <ol>
                    {#each w.help[source] as step, i (i)}
                      <li>{step}</li>
                    {/each}
                  </ol>
                  {#if source === "github"}
                    <p class="note">{w.help.githubNotifications}</p>
                  {/if}
                </details>
              </div>
            {/if}
          </Switch>
        </div>
      {/each}

      <div class="item">
        <Switch
          id="work-hold-during-focus"
          label={w.holdDuringFocus}
          hint={w.holdDuringFocusHint}
          checked={wcfg.holdDuringFocus}
          onchange={(value) => set((x) => (x.work.holdDuringFocus = value))}
        />
      </div>

      {#if anyOn}
        <div class="item">
          <div class="row sync">
            <span class="hint tabular">
              {status?.lastSyncMs != null ? w.lastSync(formatTimeOfDay(status.lastSyncMs)) : w.neverSynced}
            </span>
            <button type="button" class="btn btn-sm" disabled={work.refreshing} onclick={() => work.refresh()}>
              {work.refreshing ? w.refreshing : w.refresh}
            </button>
          </div>
        </div>
      {/if}
    </div>

    <p class="hint privacy">{w.privacy}</p>
  </div>
{/if}

<style>
  .work {
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

  .token-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .token {
    flex: 1 1 240px;
    min-width: 0;
  }

  .token::placeholder {
    font-family: var(--font);
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

  .state.bad .dot {
    background: var(--danger);
  }

  .sync {
    justify-content: space-between;
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

  .help ol {
    margin: 10px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .note {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
  }

  .privacy {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
