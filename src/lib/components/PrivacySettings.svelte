<script lang="ts">
  // Settings → Prywatność i dane (ADR-020): item by item what FocusMode stores on this computer (presence, size, count,
  // a plain explanation), with "Open data folder", "Export my data" (one JSON file, never secrets), a delete per item
  // where it makes sense and "Delete everything" behind a confirmation that lists what goes. Contents of tokens, ICS
  // links and cookies never reach this view: the shell reports only that they exist.
  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import { formatSize } from "$lib/format";
  import { config } from "$lib/state/config.svelte";
  import { privacy } from "$lib/state/privacy.svelte";
  import { session } from "$lib/state/session.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { youtubeAccount } from "$lib/state/youtubeAccount.svelte";
  import { strings } from "$lib/strings";
  import type { PrivacyItem, PrivacyItemKind } from "$lib/types";

  interface Props {
    /** Called after "Delete everything" (the settings page re-reads what lives outside the config, e.g. autostart). */
    onreset?: () => void;
  }

  let { onreset }: Props = $props();

  const p = strings.privacy;

  type GroupId = "app" | "connections" | "browser" | "restore";
  /** Display groups; together they cover every `PrivacyItemKind` (focus-core `PrivacyItemKind::ALL`). */
  const GROUPS: readonly { id: GroupId; kinds: readonly PrivacyItemKind[] }[] = [
    { id: "app", kinds: ["settings", "history", "rewards", "meetingChoices", "workSeen", "customChimes", "blocklist"] },
    {
      id: "connections",
      kinds: [
        "microsoftToken",
        "googleToken",
        "icsLinks",
        "githubToken",
        "gitlabToken",
        "jiraToken",
        "azureDevopsToken",
        "linearToken",
      ],
    },
    { id: "browser", kinds: ["browserData", "logs"] },
    { id: "restore", kinds: ["shieldsRestore", "tintRestore"] },
  ];

  /** What a count means per kind (`null`: no count shown). */
  const COUNT_UNIT: Partial<Record<PrivacyItemKind, keyof typeof p.counts>> = {
    history: "sessions",
    rewards: "days",
    meetingChoices: "answers",
    workSeen: "ids",
    icsLinks: "links",
    customChimes: "files",
    blocklist: "entries",
    logs: "files",
  };

  const report = $derived(privacy.report);
  const busy = $derived(privacy.busy);
  const byKind = $derived(new Map((report?.items ?? []).map((item) => [item.kind, item])));
  const sessionRuns = $derived(!session.isIdle);

  /** Deletable items that exist now, in report order (what "Delete everything" removes besides the settings). */
  const presentDeletable = $derived((report?.items ?? []).filter((item) => item.deletable && item.present));

  let pending = $state<PrivacyItemKind | null>(null);
  let itemOpen = $state(false);
  let allOpen = $state(false);
  let exporting = $state(false);

  onMount(() => privacy.watch());

  /**
   * The export also holds the rewards ledger (beta, ADR-022) while a readable `rewards.json` exists (its days are
   * counted then; an unreadable copy set aside at load is listed and deleted with the item, but never exported).
   */
  const rewardsListed = $derived.by(() => {
    const item = byKind.get("rewards");
    return item !== undefined && item.present && item.count !== null;
  });

  /**
   * The beta rewards ledger (ADR-022) is listed while it exists or while the beta features are on; with the beta off
   * and no `rewards.json` the panel looks as before.
   */
  function listed(item: PrivacyItem): boolean {
    return item.kind !== "rewards" || item.present || config.value?.betaFeatures === true;
  }

  function metaText(item: PrivacyItem): string {
    if (!item.present) return p.absent;
    const parts: string[] = [];
    const unit = COUNT_UNIT[item.kind];
    if (unit && item.count !== null) parts.push(p.counts[unit](item.count));
    if (item.sizeKib !== null) parts.push(formatSize(item.sizeKib));
    return parts.length > 0 ? parts.join(" · ") : p.present;
  }

  function actionLabel(kind: PrivacyItemKind): string {
    if (kind === "blocklist") return p.restoreDefault;
    if (kind === "browserData" || kind === "logs") return p.clear;
    return p.delete;
  }

  function ask(kind: PrivacyItemKind): void {
    if (busy) return;
    pending = kind;
    itemOpen = true;
  }

  const pendingTitle = $derived(
    pending === null ? "" : pending === "blocklist" ? p.resetTitle : p.deleteTitle(p.items[pending]),
  );
  const pendingBody = $derived.by(() => {
    if (pending === null) return "";
    const base = p.consequences[pending] ?? "";
    // The beta collection (ADR-024) lives in rewards.json too: its unlocks go with it.
    const consequence = pending === "rewards" ? `${base} ${strings.collection.resetNote}`.trim() : base;
    return sessionRuns ? `${consequence} ${p.sessionStops}`.trim() : consequence;
  });

  async function confirmItem(): Promise<void> {
    const kind = pending;
    if (kind === null) return;
    // Errors are shown inside the dialog (ConfirmDialog keeps it open).
    await privacy.remove([kind]);
    // The shell also drops the collection theme from config.json (ADR-024); the local config follows.
    if (kind === "rewards") config.applyStored((c) => (c.rewardTheme = null));
    toasts.push(kind === "blocklist" ? p.blocklistReset : p.deleted(p.items[kind]), "info");
  }

  const allBody = $derived(sessionRuns ? `${p.sessionStops} ${p.deleteAllBody}` : p.deleteAllBody);
  const allList = $derived([...presentDeletable.map((item) => p.items[item.kind]), p.deleteAllSettings]);

  async function confirmAll(): Promise<void> {
    await privacy.removeAll();
    toasts.push(p.deleteAllDone, "info");
    onreset?.();
  }

  async function exportData(): Promise<void> {
    if (exporting || busy) return;
    exporting = true;
    try {
      if (await privacy.exportData(p.exportDialogTitle, p.exportFilterName)) toasts.push(p.exported, "info");
    } catch (e) {
      toasts.error(e);
    } finally {
      exporting = false;
    }
  }

  function openFolder(): void {
    api.openDataFolder().catch((e: unknown) => toasts.error(e));
  }
</script>

<div class="privacy" aria-busy={busy}>
  <p class="hint">{p.lead}</p>

  <div class="actions">
    <button type="button" class="btn" onclick={openFolder}>
      <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
        <path
          d="M3.5 7.5V18a1.5 1.5 0 0 0 1.5 1.5h14a1.5 1.5 0 0 0 1.5-1.5V9.5A1.5 1.5 0 0 0 19 8h-7l-2-2.5H5A1.5 1.5 0 0 0 3.5 7v.5Z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linejoin="round"
        />
      </svg>
      {p.openFolder}
    </button>
    <button
      type="button"
      class="btn"
      disabled={busy || exporting}
      aria-describedby="privacy-export-hint"
      onclick={() => void exportData()}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
        <path
          d="M12 4v11m0 0-4-4m4 4 4-4M5 19.5h14"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      {p.export}
    </button>
    {#if busy}
      <span class="small muted" role="status">{p.busy}</span>
    {/if}
  </div>
  <p class="hint" id="privacy-export-hint">
    {p.exportHint}{#if rewardsListed}{" "}{strings.rewards.privacyExport}{/if}
  </p>

  {#if !report || report.items.length === 0}
    <p class="hint">{p.loading}</p>
  {:else}
    {#if report.folders.length > 0}
      <div class="folders small">
        <span class="muted">{p.folders}</span>
        {#each report.folders as folder (folder)}
          <code class="path">{folder}</code>
        {/each}
      </div>
    {/if}

    {#each GROUPS as group (group.id)}
      <section class="group" aria-labelledby="privacy-group-{group.id}">
        <h3 id="privacy-group-{group.id}">{p.groups[group.id]}</h3>
        {#if group.id === "restore"}
          <p class="hint">{p.restoreHint}</p>
        {/if}
        <ul class="items">
          {#each group.kinds as kind (kind)}
            {@const item = byKind.get(kind)}
            {#if item && listed(item)}
              <li class="item" class:absent={!item.present}>
                <div class="text">
                  <p class="name">
                    {p.items[kind]}
                    {#if item.includesTitles}
                      <span class="badge">{p.withTitles}</span>
                    {/if}
                    <!-- The YouTube sign-in lives in these cookies (ADR-021): clearing them signs out. -->
                    {#if kind === "browserData" && item.present && youtubeAccount.signedIn}
                      <span class="badge">{strings.youtube.signedIn}</span>
                    {/if}
                  </p>
                  <p class="hint">
                    {p.explain[kind]}{#if kind === "rewards"}{" "}{strings.collection.privacyNote}{/if}
                  </p>
                </div>
                <p class="meta small tabular">{metaText(item)}</p>
                <div class="action">
                  {#if item.deletable && item.present}
                    <button
                      type="button"
                      class="btn btn-sm"
                      disabled={busy}
                      aria-label={`${actionLabel(kind)}: ${p.items[kind]}`}
                      onclick={() => ask(kind)}
                    >
                      {actionLabel(kind)}
                    </button>
                  {/if}
                </div>
              </li>
            {/if}
          {/each}
        </ul>
      </section>
    {/each}

    <div class="delete-all">
      <div class="delete-all-text">
        <p class="name">{p.deleteAll}</p>
        <p class="hint" id="privacy-delete-all-hint">{p.deleteAllHint}</p>
      </div>
      <button
        type="button"
        class="btn btn-danger"
        disabled={busy}
        aria-describedby="privacy-delete-all-hint"
        onclick={() => (allOpen = true)}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
          <path
            d="M4 7h16M9 7V4.5h6V7M6.5 7l1 12.5h9L17.5 7M10 11v5M14 11v5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        {p.deleteAllButton}
      </button>
    </div>
  {/if}
</div>

<ConfirmDialog
  bind:open={itemOpen}
  title={pendingTitle}
  body={pendingBody}
  confirmLabel={pending === null ? p.delete : actionLabel(pending)}
  danger
  onconfirm={confirmItem}
/>

<ConfirmDialog
  bind:open={allOpen}
  title={p.deleteAllTitle}
  body={allBody}
  list={allList}
  confirmLabel={p.deleteAllConfirm}
  danger
  onconfirm={confirmAll}
/>

<style>
  .privacy {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .folders {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
  }

  .path {
    font-size: 0.8rem;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }

  .group {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .group h3 {
    margin: 0 0 6px;
    font-size: 0.95rem;
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(7rem, auto) minmax(9.5rem, auto);
    align-items: center;
    gap: 6px 16px;
    padding: 8px 0;
  }

  .item + .item {
    border-top: 1px dashed var(--border);
  }

  .item.absent .name {
    color: var(--text-2);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-weight: 600;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .meta {
    color: var(--text-2);
    text-align: right;
    white-space: nowrap;
  }

  .item.absent .meta {
    color: var(--text-muted);
  }

  .action {
    display: flex;
    justify-content: flex-end;
  }

  .delete-all {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .delete-all-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1 1 240px;
  }
</style>
