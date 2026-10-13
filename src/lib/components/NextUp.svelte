<script lang="ts">
  // Skupienie page: "Co dalej?" — the top work items from the connected tools, ranked in focus-core (assigned to the
  // user first, then what waits on the user, then other notifications), each with the reasons of its place.
  // "Otwórz" opens the item in the browser, "Ustaw jako cel" makes it the session goal (shown under the timer).
  import { formatDayNear, formatTimeOfDay } from "$lib/format";
  import { config } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { goalOf, isGoalItem, isOpenableUrl, sourceEnabled, work, workEnabled } from "$lib/state/work.svelte";
  import { strings } from "$lib/strings";
  import type { WorkItem, WorkItemKind, WorkReason } from "$lib/types";

  /** Items shown at first, and after "Pokaż więcej". */
  const TOP_COUNT = 5;
  const MORE_COUNT = 15;
  /** Kinds whose meaning no reason names; they get a chip of their own. */
  const KIND_CHIP: ReadonlySet<WorkItemKind> = new Set(["changesRequested", "ciFailed", "notification"]);
  /** Due dates and "today" move with the clock. */
  const CLOCK_MS = 60_000;

  const w = strings.work;

  let expanded = $state(false);
  let now = $state(Date.now());

  $effect(() => {
    const clock = setInterval(() => (now = Date.now()), CLOCK_MS);
    return () => clearInterval(clock);
  });

  const cfg = $derived(config.value);
  const realEnabled = $derived(workEnabled(cfg));
  /** A developer-mode sample (ADR-019) shows the card even without a connected tool. */
  const enabled = $derived(realEnabled || work.sample !== null);

  // The list is on screen: read it through `get_work`, so the shell counts its items as seen ("nowe" and the
  // held count clear with the next status). Runs again when the tools are switched on.
  $effect(() => {
    if (realEnabled) void work.reload();
  });
  const status = $derived(work.status);
  const items = $derived(status?.items ?? []);
  const shown = $derived(items.slice(0, expanded ? MORE_COUNT : TOP_COUNT));
  const moreCount = $derived(Math.max(0, Math.min(items.length, MORE_COUNT) - TOP_COUNT));
  const goal = $derived(session.goal);
  /**
   * New items. While focus runs they are held (`heldCount`); when the break starts the shell announces them and
   * empties the count, so from then on they are the items still marked `unseen`.
   */
  const unseenCount = $derived(items.filter((item) => item.unseen).length);
  const newBadge = $derived.by((): string | null => {
    const held = status?.heldCount ?? 0;
    if (held > 0) return w.held(held);
    if (unseenCount === 0) return null;
    return session.snapshot.phase === "break" ? w.held(unseenCount) : w.newCount(unseenCount);
  });

  /** Switched-on tools that report an error, by name. */
  const failing = $derived(
    (status?.sources ?? [])
      .filter((s) => s.state === "error" && sourceEnabled(cfg, s.source))
      .map((s) => w.sourceName[s.source])
      .join(", "),
  );
  const anyConnected = $derived((status?.sources ?? []).some((s) => s.state === "on"));

  function nameOf(item: WorkItem): string {
    return w.itemName(item.key, item.title);
  }

  /** `In Progress · High · web-app`; the due date unless a reason already says "today". */
  function meta(item: WorkItem): string {
    const parts = [item.status, item.priorityLabel, item.project].filter(
      (part): part is string => typeof part === "string" && part.trim() !== "",
    );
    if (item.dueMs !== null && !item.reasons.includes("dueToday")) parts.push(w.due(formatDayNear(item.dueMs, now)));
    return parts.join(" · ");
  }

  function reasonClass(reason: WorkReason): string {
    if (reason === "overdue") return "tag danger";
    if (reason === "assigned" || reason === "blockingOthers") return "tag accent";
    if (reason === "highPriority" || reason === "dueToday") return "tag warm";
    return "tag";
  }
</script>

{#if enabled}
  <section class="card next-up" aria-labelledby="next-up-title">
    <div class="card-title">
      <div class="heading">
        <h2 id="next-up-title">{w.cardTitle}</h2>
        {#if newBadge}<span class="badge held">{newBadge}</span>{/if}
      </div>
      <div class="sync">
        {#if status?.lastSyncMs != null}
          <span class="hint tabular">{w.lastSync(formatTimeOfDay(status.lastSyncMs))}</span>
        {/if}
        <button
          type="button"
          class="icon-btn small-btn"
          disabled={work.refreshing}
          aria-label={work.refreshing ? w.refreshing : w.refresh}
          title={work.refreshing ? w.refreshing : w.refresh}
          onclick={() => {
            // A developer-mode sample is shown: no tool is contacted for it (ADR-019).
            if (work.sample === null) void work.refresh();
          }}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" class:spin={work.refreshing}>
            <path
              d="M20 12a8 8 0 1 1-2.34-5.66M20 4v4.5h-4.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.9"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
      </div>
    </div>

    {#if !status}
      <p class="hint">{w.loading}</p>
    {:else}
      {#if failing}
        <p class="warn small">
          {w.sourceErrors(failing)}
          <a href="/settings#work-settings">{w.sourceErrorsLink}</a>
        </p>
      {/if}

      {#if items.length === 0}
        {#if !anyConnected && !failing}
          <p class="secondary">
            {w.noTokenHint}
            <a class="settings-link" href="/settings#work-settings">{w.connectLink}</a>
          </p>
        {:else}
          <p class="secondary">{w.empty}</p>
        {/if}
      {:else}
        <p class="hint lead">{w.cardLead}</p>
        <ol class="items">
          {#each shown as item (item.id)}
            {@const name = nameOf(item)}
            {@const details = meta(item)}
            <li class="item" class:goal={isGoalItem(goal, item)}>
              <div class="body">
                <p class="head">
                  <span class="source badge">{w.sourceShort[item.source]}</span>
                  {#if item.key}<span class="key">{item.key}</span>{/if}
                  {#if item.unseen}<span class="badge unseen">{w.unseen}</span>{/if}
                </p>
                <p class="title" title={item.title}>{item.title}</p>
                {#if details}<p class="meta small">{details}</p>{/if}
                {#if item.reasons.length > 0 || KIND_CHIP.has(item.kind)}
                  <ul class="tags" aria-label={w.reasonsLabel}>
                    {#if KIND_CHIP.has(item.kind)}
                      <li class={item.kind === "ciFailed" ? "tag danger" : "tag"}>{w.kind[item.kind]}</li>
                    {/if}
                    {#each item.reasons as reason (reason)}
                      <li class={reasonClass(reason)}>{w.reason[reason]}</li>
                    {/each}
                  </ul>
                {/if}
              </div>
              <div class="actions">
                {#if isOpenableUrl(item.url)}
                  <button type="button" class="btn btn-sm" aria-label={w.openAria(name)} onclick={() => work.open(item.url)}>
                    {w.open}
                  </button>
                {/if}
                {#if isGoalItem(goal, item)}
                  <span class="badge is-goal">{w.isGoal}</span>
                {:else}
                  <button
                    type="button"
                    class="btn btn-sm"
                    aria-label={w.setGoalAria(name)}
                    disabled={work.goalBusy}
                    onclick={() => work.setGoal(goalOf(item))}
                  >
                    {w.setGoal}
                  </button>
                {/if}
              </div>
            </li>
          {/each}
        </ol>
        {#if moreCount > 0}
          <button type="button" class="btn btn-sm btn-ghost more" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
            {expanded ? w.showLess : w.showMore(moreCount)}
          </button>
        {/if}
      {/if}
    {/if}
  </section>
{:else if cfg}
  <p class="hint connect">
    {w.connectHint}
    <a class="settings-link" href="/settings#work-settings">{w.connectLink}</a>
  </p>
{/if}

<style>
  .next-up {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card-title {
    margin-bottom: 0;
    align-items: center;
    flex-wrap: wrap;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .held {
    background: var(--break-soft);
    color: var(--break);
  }

  .sync {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .small-btn {
    width: 32px;
    height: 32px;
  }

  .spin {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }

  .warn {
    color: var(--danger);
  }

  .warn a {
    color: inherit;
    font-weight: 600;
  }

  .settings-link {
    color: var(--accent-strong);
    font-weight: 600;
  }

  .connect {
    text-align: center;
  }

  .lead {
    margin-top: -4px;
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--surface-inset);
  }

  .item.goal {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    min-width: 0;
  }

  .source {
    flex: none;
    background: var(--surface-hover);
    color: var(--text-2);
  }

  .key {
    font-family: var(--font-mono);
    font-size: 0.84rem;
    color: var(--accent-strong);
    overflow-wrap: anywhere;
  }

  .unseen {
    background: var(--break-soft);
    color: var(--break);
  }

  .title {
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }

  .meta {
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .tags {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    padding: 1px 9px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    font-size: 0.78rem;
    font-weight: 600;
    line-height: 1.5;
  }

  .tag.accent {
    border-color: transparent;
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .tag.warm {
    border-color: transparent;
    background: var(--break-soft);
    color: var(--break);
  }

  .tag.danger {
    border-color: var(--danger-line);
    background: var(--danger-soft);
    color: var(--danger);
  }

  .actions {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }

  .is-goal {
    justify-content: center;
    min-height: 32px;
  }

  .more {
    align-self: flex-start;
    color: var(--accent-strong);
  }
</style>
