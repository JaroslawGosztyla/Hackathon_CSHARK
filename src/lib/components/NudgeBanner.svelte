<script lang="ts">
  import { strings } from "$lib/strings";
  import { nudge } from "$lib/state/nudge.svelte";

  /** A plain domain from websiteblacklist.txt reads well in the banner; a regex source does not. */
  const DOMAIN = /^[a-z0-9-]+(\.[a-z0-9-]+)+$/i;

  const body = $derived.by(() => {
    const matched = nudge.current?.matched ?? "";
    return DOMAIN.test(matched) ? strings.nudge.body(matched) : strings.nudge.bodyPattern;
  });
</script>

{#if nudge.current}
  <div class="nudge" role="alert">
    <span class="icon" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="22" height="22">
        <path
          d="M4 14v-2a8 8 0 0 1 16 0v2"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
        />
        <rect x="3" y="13" width="4.5" height="7" rx="1.8" fill="currentColor" />
        <rect x="16.5" y="13" width="4.5" height="7" rx="1.8" fill="currentColor" />
      </svg>
    </span>
    <p>
      <strong>{strings.nudge.title}</strong> — {body}
    </p>
    <button class="btn btn-sm btn-ghost" type="button" onclick={() => nudge.dismiss()}>{strings.nudge.dismiss}</button>
  </div>
{/if}

<style>
  .nudge {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 auto 16px;
    padding: 12px 14px 12px 16px;
    border-radius: var(--radius);
    background: linear-gradient(90deg, rgba(242, 166, 90, 0.22), rgba(242, 166, 90, 0.08));
    border: 1px solid rgba(242, 166, 90, 0.45);
    animation: pop 0.25s ease-out;
  }

  .icon {
    color: var(--accent-strong);
    display: grid;
  }

  p {
    flex: 1;
    color: var(--text);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
