<script lang="ts">
  import { strings } from "$lib/strings";
  import { toasts } from "$lib/state/toasts.svelte";
</script>

<section class="toasts" aria-label={strings.toasts.region} aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <div class="toast" class:error={toast.kind === "error"} role={toast.kind === "error" ? "alert" : "status"}>
      <span class="dot" aria-hidden="true"></span>
      <p title={toast.detail ?? undefined}>{toast.text}</p>
      <button class="close" type="button" aria-label={strings.toasts.dismiss} onclick={() => toasts.dismiss(toast.id)}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </button>
    </div>
  {/each}
</section>

<style>
  .toasts {
    position: fixed;
    top: 72px;
    right: var(--gutter);
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(380px, calc(100vw - 2 * var(--gutter)));
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: start;
    gap: 10px;
    padding: 12px 10px 12px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    animation: slide-in 0.2s ease-out;
  }

  .toast p {
    font-size: 0.9rem;
    color: var(--text);
  }

  .dot {
    width: 8px;
    height: 8px;
    margin-top: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .toast.error {
    border-color: color-mix(in srgb, var(--danger) 45%, transparent);
  }

  .toast.error .dot {
    background: var(--danger);
  }

  .close {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
  }

  .close:hover {
    background: var(--surface-inset);
    color: var(--text);
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
