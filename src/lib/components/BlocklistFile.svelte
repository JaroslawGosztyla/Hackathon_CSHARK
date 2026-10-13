<script lang="ts">
  import { blocklist } from "$lib/state/blocklist.svelte";
  import { strings } from "$lib/strings";

  const s = strings.blocklist;
  const info = $derived(blocklist.info);
</script>

<section class="blocklist" aria-labelledby="blocklist-title">
  <div class="head">
    <h3 id="blocklist-title">{s.title}</h3>
    <p class="summary tabular" aria-live="polite">
      {info ? s.summary(info.domains.length, info.patterns.length) : s.loading}
    </p>
  </div>

  {#if info}
    <p class="path-row">
      <span class="sr-only">{s.pathLabel}:</span>
      <code class="path" title={info.path}>{info.path}</code>
    </p>
  {/if}

  <div class="row">
    <button type="button" class="btn btn-sm btn-primary" disabled={blocklist.opening} onclick={() => blocklist.open()}>
      {s.open}
    </button>
    <button type="button" class="btn btn-sm" disabled={blocklist.reloading} onclick={() => blocklist.reload()}>
      {s.reload}
    </button>
  </div>
  <p class="hint">{s.autoReload}</p>

  {#if info}
    {#if info.error}
      <!-- The shell sends a short Polish description of the read failure (BlocklistInfo.error). -->
      <p class="field-error">{s.readError} {info.error}</p>
    {/if}

    {#if info.issues.length > 0}
      <div class="issues">
        <p class="issues-title">{s.issuesTitle(info.issues.length)}</p>
        <ul>
          {#each info.issues as issue, i (i)}
            <li>
              <span class="line tabular">{s.issueLine(issue.line)}</span>
              <code class="text">{issue.text}</code>
              <span class="message">{issue.message}</span>
            </li>
          {/each}
        </ul>
      </div>
    {:else if !info.error}
      <p class="ok">
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path
            d="M3.5 8.5l3 3 6-7"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        {s.allValid}
      </p>
    {/if}

    {#if blocklist.count === 0 && !info.error}
      <p class="hint">{s.empty}</p>
    {/if}
  {/if}

  <details class="help">
    <summary>{s.helpTitle}</summary>
    <ul class="rules">
      <li>{s.helpLines}</li>
      <li>{s.helpDomain}</li>
      <li>{s.helpRegex}</li>
    </ul>
    <p class="examples-label">{s.examplesLabel}</p>
    <ul class="examples">
      {#each s.examples as example (example.code)}
        <li><code>{example.code}</code> — {example.note}</li>
      {/each}
    </ul>
  </details>
</section>

<style>
  .blocklist {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 4px 12px;
  }

  .summary {
    color: var(--text-2);
    font-size: 0.9rem;
    font-weight: 600;
  }

  code {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
  }

  .path {
    display: block;
    font-size: 0.8rem;
    color: var(--text-muted);
    overflow-wrap: anywhere;
    user-select: text;
  }

  .ok {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ok);
    font-size: 0.9rem;
    font-weight: 600;
  }

  .issues {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--danger-soft);
    border: 1px solid rgba(242, 148, 134, 0.4);
  }

  .issues-title {
    color: var(--text);
    font-size: 0.88rem;
    font-weight: 600;
  }

  .issues ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 260px;
    overflow-y: auto;
  }

  .issues li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 2px 10px;
    font-size: 0.85rem;
  }

  .line {
    grid-row: span 2;
    color: var(--danger);
    font-weight: 600;
    white-space: nowrap;
  }

  .text {
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .message {
    color: var(--text-2);
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

  .help ul {
    margin: 8px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .examples-label {
    margin-top: 10px;
    font-weight: 600;
  }

  .help code {
    color: var(--text);
    font-size: 0.85rem;
  }
</style>
