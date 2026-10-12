<script lang="ts">
  import { formatDuration } from "$lib/format";
  import { shields } from "$lib/state/shields.svelte";
  import { strings } from "$lib/strings";

  const BRIDGE_REFRESH_MS = 5000;

  $effect(() => {
    void shields.refreshBridge();
    const id = setInterval(() => void shields.refreshBridge(), BRIDGE_REFRESH_MS);
    return () => clearInterval(id);
  });

  /** Same window as the shell's `extensionConnected` (the bridge treats 90 s of silence as disconnected). */
  const EXTENSION_TIMEOUT_SECS = 90;

  const seenAgo = $derived(shields.bridge?.extensionSeenSecsAgo ?? null);
  // `shields://status` is pushed only when a shield changes, so a vanished extension would keep showing as
  // connected; the bridge info polled above is fresher whenever it is available.
  const connected = $derived(
    shields.bridge
      ? seenAgo !== null && seenAgo < EXTENSION_TIMEOUT_SECS
      : (shields.status?.extensionConnected ?? false),
  );
</script>

<div class="guide">
  <h3>{strings.settings.extensionTitle}</h3>
  <p class="status" class:ok={connected} aria-live="polite">
    <span class="dot" aria-hidden="true"></span>
    {connected ? strings.settings.extensionConnected : strings.settings.extensionDisconnected}
    {#if seenAgo !== null}
      <span class="muted">({strings.settings.extensionSeen(formatDuration(seenAgo))})</span>
    {/if}
  </p>
  <ol>
    <li>
      {strings.settings.extensionStep1}
      <button type="button" class="btn btn-sm" onclick={() => shields.openExtensionDir()}>
        {strings.settings.extensionOpenDir}
      </button>
      {#if shields.bridge?.extensionDir}
        <code class="path" title={shields.bridge.extensionDir}>{shields.bridge.extensionDir}</code>
      {/if}
    </li>
    <li>{strings.settings.extensionStep2}</li>
    <li>{strings.settings.extensionStep3}</li>
  </ol>
</div>

<style>
  .guide {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
  }

  .status {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    color: var(--text-2);
    font-size: 0.9rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }

  .status.ok .dot {
    background: var(--ok);
  }

  ol {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--text-2);
    font-size: 0.9rem;
  }

  li :global(.btn) {
    margin-left: 6px;
  }

  .path {
    display: block;
    margin-top: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
