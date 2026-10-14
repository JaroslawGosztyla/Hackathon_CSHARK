<script lang="ts">
  // Settings → Historia: what is recorded (apps, window titles), whether sessions are saved, and deleting it all.
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { config } from "$lib/state/config.svelte";
  import { summary } from "$lib/state/summary.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";
  import type { Config } from "$lib/types";

  const s = strings.settings;
  const cfg = $derived(config.value);

  let confirmOpen = $state(false);

  function set(mutate: (c: Config) => void): void {
    config.update(mutate, true);
  }

  async function clearAll(): Promise<void> {
    // Errors are shown inside the dialog (ConfirmDialog keeps it open).
    await summary.clear();
    toasts.push(s.historyCleared, "info");
  }
</script>

{#if cfg}
  <div class="items">
    <div class="item">
      <Switch
        id="track-apps"
        label={s.trackApps}
        hint={s.trackAppsHint}
        checked={cfg.trackApps}
        onchange={(on) => set((c) => (c.trackApps = on))}
      />
    </div>

    <div class="item">
      <Switch
        id="track-titles"
        label={s.trackTitles}
        hint={cfg.trackApps ? s.trackTitlesHint : `${s.trackTitlesHint} ${s.trackTitlesNeedsApps}`}
        checked={cfg.trackTitles}
        disabled={!cfg.trackApps}
        onchange={(on) => set((c) => (c.trackTitles = on))}
      />
    </div>

    <div class="item">
      <Switch
        id="keep-history"
        label={s.keepHistory}
        hint={s.keepHistoryHint}
        checked={cfg.keepHistory}
        onchange={(on) => set((c) => (c.keepHistory = on))}
      />
    </div>

    <div class="item clear">
      <div class="clear-text">
        <p class="clear-title">{s.clearHistory}</p>
        <p class="hint" id="clear-history-hint">{s.clearHistoryHint}</p>
      </div>
      <button
        type="button"
        class="btn btn-danger"
        aria-describedby="clear-history-hint"
        onclick={() => (confirmOpen = true)}
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
        {s.clearHistory}
      </button>
    </div>
  </div>

  <ConfirmDialog
    bind:open={confirmOpen}
    title={s.clearHistoryTitle}
    body={s.clearHistoryBody}
    confirmLabel={s.clearHistoryConfirm}
    danger
    onconfirm={clearAll}
  />
{/if}

<style>
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

  .clear {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px 16px;
  }

  .clear-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1 1 240px;
  }

  .clear-title {
    font-weight: 600;
  }
</style>
