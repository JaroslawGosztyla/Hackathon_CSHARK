<script lang="ts">
  // "Teraz używasz": live top apps of the running session (polls `get_current_usage`).
  import * as api from "$lib/api";
  import { formatDuration, processLabel } from "$lib/format";
  import { config } from "$lib/state/config.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { strings } from "$lib/strings";
  import type { UsageReport } from "$lib/types";

  const POLL_MS = 5000;
  const TOP = 5;

  let usage = $state.raw<UsageReport | null>(null);

  $effect(() => {
    let alive = true;
    let reported = false;
    const poll = async () => {
      try {
        const report = await api.getCurrentUsage();
        if (alive) usage = report;
      } catch (e) {
        // Report once; a failing poll every 5 s would only spam the toasts.
        if (!reported && alive) {
          reported = true;
          const err = api.toAppError(e);
          if (err.code !== api.ERR_NO_SHELL) toasts.error(err);
        }
      }
    };
    void poll();
    const id = setInterval(() => void poll(), POLL_MS);
    return () => {
      alive = false;
      clearInterval(id);
    };
  });

  const tracking = $derived(config.value?.trackApps ?? true);
  const rows = $derived.by(() => {
    if (!usage) return [];
    const top = usage.apps.slice(0, TOP).map((a) => ({ name: processLabel(a.process), full: a.process, secs: a.secs, muted: false }));
    if (usage.idleSecs > 0) top.push({ name: strings.focusView.idle, full: strings.focusView.idle, secs: usage.idleSecs, muted: true });
    return top;
  });
  const maxSecs = $derived(Math.max(1, ...rows.map((r) => r.secs)));
</script>

<section class="card" aria-labelledby="usage-title">
  <div class="card-title">
    <h2 id="usage-title">{strings.focusView.nowUsing}</h2>
  </div>
  {#if !tracking}
    <p class="hint">{strings.focusView.trackingOff}</p>
  {:else if rows.length === 0}
    <p class="hint">{strings.focusView.nowUsingEmpty}</p>
  {:else}
    <ul class="usage">
      {#each rows as row (row.full)}
        <li class:muted-row={row.muted}>
          <span class="name" title={row.full}>{row.name}</span>
          <span class="bar-track" aria-hidden="true">
            <span class="bar" style:width="{(row.secs / maxSecs) * 100}%"></span>
          </span>
          <span class="secs tabular">{formatDuration(row.secs)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .usage {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  li {
    display: grid;
    grid-template-columns: minmax(0, 9rem) 1fr auto;
    align-items: center;
    gap: 10px;
    font-size: 0.92rem;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar-track {
    height: 10px;
    border-radius: 5px;
    background: rgba(255, 236, 214, 0.06);
    overflow: hidden;
  }

  .bar {
    display: block;
    height: 100%;
    border-radius: 0 5px 5px 0;
    background: var(--accent);
    transition: width 0.6s ease;
  }

  .muted-row .bar {
    background: var(--muted-mark);
  }

  .muted-row .name {
    color: var(--text-muted);
  }

  .secs {
    color: var(--text-2);
    font-size: 0.86rem;
  }
</style>
