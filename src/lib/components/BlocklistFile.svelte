<script lang="ts">
  import { blocklist } from "$lib/state/blocklist.svelte";
  import { strings } from "$lib/strings";
  import type { BlocklistIssue } from "$lib/types";

  const s = strings.blocklist;
  const info = $derived(blocklist.info);
  /** Regexes the browser extension could not turn into network rules (missing with an older shell build). */
  const extensionSkipped = $derived((info?.extensionSkipped as string[] | undefined) ?? []);

  /** Polish explanation for the issue code, plus the shell's English detail when it adds something. */
  function describe(issue: BlocklistIssue): { text: string; detail: string | null } {
    const detail = issue.message.trim();
    if (issue.code === "pastedUrl") {
      // The detail is the domain to suggest.
      return { text: detail ? s.pastedUrl(detail) : s.issueText.pastedUrl, detail: null };
    }
    return { text: s.issueText[issue.code] ?? strings.errors.generic, detail: detail || null };
  }
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
            {@const why = describe(issue)}
            <li>
              <span class="line tabular">{s.issueLine(issue.line)}</span>
              <code class="text">{issue.text}</code>
              <span class="message">
                {why.text}
                {#if why.detail}
                  <span class="detail" lang="en" title={s.issueDetailLabel}>({why.detail})</span>
                {/if}
              </span>
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

    {#if extensionSkipped.length > 0}
      <div class="skipped">
        <p class="skipped-title">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
            <path d="M8 1.8l6.6 11.7H1.4z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
            <path d="M8 6.2v3.4M8 11.4v.4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          </svg>
          {s.extensionSkippedTitle}
        </p>
        <ul>
          {#each extensionSkipped as pattern, i (i)}
            <li><code>{pattern}</code></li>
          {/each}
        </ul>
      </div>
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
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
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

  .detail {
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  .skipped {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    border: 1px solid var(--accent-line);
  }

  .skipped-title {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--text);
    font-size: 0.88rem;
    font-weight: 600;
  }

  .skipped-title svg {
    flex: none;
    margin-top: 2px;
    color: var(--accent-strong);
  }

  .skipped ul {
    list-style: none;
    margin: 0;
    padding: 0 0 0 22px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 180px;
    overflow-y: auto;
  }

  .skipped code {
    color: var(--text);
    font-size: 0.85rem;
    overflow-wrap: anywhere;
    user-select: text;
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
