<script lang="ts">
  import { openExternal } from "$lib/api";
  import { config } from "$lib/state/config.svelte";
  import { teams } from "$lib/state/teams.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";

  const shieldsCfg = $derived(config.value?.shields ?? null);
  const status = $derived(teams.status);
  const prompt = $derived(teams.prompt);
  const configured = $derived((shieldsCfg?.teamsClientId.trim() ?? "") !== "");
  const clientIdError = $derived(config.fieldErrors["shields.teamsClientId"] ?? null);
  const tenantError = $derived(config.fieldErrors["shields.teamsTenant"] ?? null);

  let copied = $state(false);

  function setClientId(event: Event & { currentTarget: HTMLInputElement }) {
    const value = event.currentTarget.value;
    config.update((c) => {
      c.shields.teamsClientId = value;
    });
  }

  function setTenant(event: Event & { currentTarget: HTMLInputElement }) {
    const value = event.currentTarget.value;
    config.update((c) => {
      c.shields.teamsTenant = value;
    });
  }

  async function openLogin(uri: string) {
    try {
      await openExternal(uri);
    } catch (e) {
      toasts.error(e);
    }
  }

  async function copyCode(code: string) {
    try {
      await navigator.clipboard.writeText(code);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      toasts.push(strings.errors.clipboard, "error");
    }
  }
</script>

{#if shieldsCfg}
  <div class="teams stack">
    <div class="grid2">
      <div class="field">
        <label for="teams-client-id">{strings.settings.teamsClientId}</label>
        <input
          id="teams-client-id"
          class="input mono"
          type="text"
          spellcheck="false"
          autocomplete="off"
          placeholder="00000000-0000-0000-0000-000000000000"
          value={shieldsCfg.teamsClientId}
          oninput={setClientId}
          aria-invalid={clientIdError ? "true" : undefined}
        />
        {#if clientIdError}<p class="field-error">{clientIdError}</p>{/if}
      </div>
      <div class="field">
        <label for="teams-tenant">{strings.settings.teamsTenant}</label>
        <input
          id="teams-tenant"
          class="input mono"
          type="text"
          spellcheck="false"
          autocomplete="off"
          placeholder="organizations"
          value={shieldsCfg.teamsTenant}
          oninput={setTenant}
          aria-invalid={tenantError ? "true" : undefined}
        />
        {#if tenantError}<p class="field-error">{tenantError}</p>{/if}
      </div>
    </div>

    <div class="status-row" aria-live="polite">
      {#if status?.connected}
        <span class="state ok">
          <span class="dot" aria-hidden="true"></span>
          {strings.settings.teamsConnected(status.account)}
        </span>
        <button type="button" class="btn btn-sm" onclick={() => teams.disconnect()}>
          {strings.settings.teamsDisconnect}
        </button>
      {:else}
        <span class="state">
          <span class="dot" aria-hidden="true"></span>
          {configured ? strings.settings.teamsNotConnected : strings.settings.teamsNotConfigured}
        </span>
        <button
          type="button"
          class="btn btn-sm btn-primary"
          disabled={!configured || teams.connecting || prompt !== null}
          onclick={() => teams.connect()}
        >
          {strings.settings.teamsConnect}
        </button>
      {/if}
    </div>

    {#if status?.error}
      <!-- The detail comes from Microsoft / the shell in English; the lead line is Polish. -->
      <p class="field-error">{strings.settings.teamsErrorLead} {status.error}</p>
    {/if}

    {#if prompt}
      <div class="device-code">
        <p>{strings.settings.teamsPendingLead}</p>
        <div class="code-row">
          <code class="code">{prompt.userCode}</code>
          <button type="button" class="btn btn-sm" onclick={() => copyCode(prompt.userCode)}>
            {copied ? strings.settings.teamsCopied : strings.settings.teamsCopyCode}
          </button>
          <button type="button" class="btn btn-sm btn-primary" onclick={() => openLogin(prompt.verificationUri)}>
            {strings.settings.teamsOpenLink}
          </button>
        </div>
        <p class="hint">
          {prompt.verificationUri} · {strings.settings.teamsCodeExpires(Math.max(1, Math.round(prompt.expiresInSecs / 60)))}
        </p>
      </div>
    {/if}

    <details class="help">
      <summary>{strings.settings.teamsHelpTitle}</summary>
      <ol>
        {#each strings.settings.teamsHelp as step, i (i)}
          <li>{step}</li>
        {/each}
      </ol>
    </details>
  </div>
{/if}

<style>
  .grid2 {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr));
    gap: 12px;
  }

  .mono {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.88rem;
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
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted-mark);
  }

  .state.ok .dot {
    background: var(--ok);
  }

  .device-code {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    border: 1px solid rgba(242, 166, 90, 0.4);
  }

  .code-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .code {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 1.5rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--accent-strong);
    user-select: all;
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
</style>
