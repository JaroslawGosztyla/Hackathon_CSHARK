<script lang="ts">
  // Small "Źródło: host" link that opens a tip's source in the default browser (opener plugin).
  import { openExternal } from "$lib/api";
  import { toasts } from "$lib/state/toasts.svelte";
  import { sourceHost } from "$lib/tips";

  interface Props {
    url: string;
    /** Visible text before the host, with its punctuation and spacing (fr "Source : ", zh "来源："). */
    label: string;
    /** Accessible name built from the host. */
    aria: (host: string) => string;
  }

  let { url, label, aria }: Props = $props();

  const host = $derived(sourceHost(url));

  async function open(): Promise<void> {
    try {
      await openExternal(url);
    } catch (e) {
      toasts.error(e);
    }
  }
</script>

<button type="button" class="source" aria-label={aria(host)} title={url} onclick={open}>
  {label}<span class="host">{host}</span>
  <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true">
    <path
      d="M6 3H3.5A1.5 1.5 0 0 0 2 4.5v8A1.5 1.5 0 0 0 3.5 14h8a1.5 1.5 0 0 0 1.5-1.5V10M9 2h5v5M14 2L7.5 8.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</button>

<style>
  .source {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 2px 0;
    border: 0;
    background: none;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .host {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .source:hover {
    color: var(--text);
  }
</style>
