<script lang="ts">
  // App-styled modal confirmation (native <dialog>, focus kept inside, Escape = cancel). Focus starts on Cancel,
  // because the confirm button usually does something that cannot be undone; it returns where it was on close.
  import { errorText, strings } from "$lib/strings";
  import { toAppError } from "$lib/api";

  interface Props {
    /** Opens the dialog when it turns true; closing (any way) sets it back to false. */
    open: boolean;
    title: string;
    body: string;
    /** Optional bullet list under the body (e.g. everything a delete removes). */
    list?: readonly string[];
    confirmLabel: string;
    cancelLabel?: string;
    /** Red confirm button for destructive actions. */
    danger?: boolean;
    /** Runs on confirm; a thrown error is shown inside the dialog and the dialog stays open. */
    onconfirm: () => Promise<void> | void;
  }

  let {
    open = $bindable(false),
    title,
    body,
    list = [],
    confirmLabel,
    cancelLabel = strings.dialog.cancel,
    danger = false,
    onconfirm,
  }: Props = $props();

  const uid = $props.id();

  let dialog = $state<HTMLDialogElement | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);
  let busy = $state(false);
  /** Shown inside the dialog: toasts would sit under the modal backdrop. */
  let error = $state<string | null>(null);
  let restoreFocus: HTMLElement | null = null;

  $effect(() => {
    const el = dialog;
    if (!el) return;
    if (open && !el.open) {
      restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      error = null;
      el.showModal();
      cancelButton?.focus();
    } else if (!open && el.open) {
      el.close();
    }
  });

  /** `close` fires for the buttons and for Escape (the dialog's `cancel`). */
  function onClosed(): void {
    open = false;
    busy = false;
    const target = restoreFocus;
    restoreFocus = null;
    if (target?.isConnected) target.focus();
  }

  function onCancel(event: Event): void {
    // Escape while the action runs: let it finish (its result decides what happens).
    if (busy) event.preventDefault();
  }

  async function confirm(): Promise<void> {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await onconfirm();
      dialog?.close();
    } catch (e) {
      const err = toAppError(e);
      console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      error = errorText(err);
      busy = false;
    }
  }

  /** Keeps Tab inside the dialog (WebView2 could otherwise move focus out of the page). */
  function trapTab(event: KeyboardEvent): void {
    if (event.key !== "Tab" || !dialog) return;
    const focusable = Array.from(dialog.querySelectorAll<HTMLElement>("button:not(:disabled)"));
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (!first || !last) {
      event.preventDefault();
      return;
    }
    const active = document.activeElement;
    if (event.shiftKey && (active === first || !dialog.contains(active))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (active === last || !dialog.contains(active))) {
      event.preventDefault();
      first.focus();
    }
  }

  $effect(() => {
    const el = dialog;
    if (!el) return;
    el.addEventListener("keydown", trapTab);
    return () => el.removeEventListener("keydown", trapTab);
  });
</script>

<dialog
  bind:this={dialog}
  class="confirm"
  aria-labelledby="{uid}-title"
  aria-describedby="{uid}-body"
  onclose={onClosed}
  oncancel={onCancel}
>
  <h2 id="{uid}-title">{title}</h2>
  <p id="{uid}-body" class="body">{body}</p>
  {#if list.length > 0}
    <ul class="list">
      {#each list as entry, i (i)}
        <li>{entry}</li>
      {/each}
    </ul>
  {/if}
  {#if error}
    <p class="field-error error" role="alert">{error}</p>
  {/if}
  <div class="actions">
    <button bind:this={cancelButton} type="button" class="btn" disabled={busy} onclick={() => dialog?.close()}>
      {cancelLabel}
    </button>
    <button type="button" class="btn" class:btn-danger={danger} class:btn-primary={!danger} disabled={busy} onclick={confirm}>
      {confirmLabel}
    </button>
  </div>
</dialog>

<style>
  .confirm {
    width: min(440px, calc(100vw - 32px));
    padding: 24px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: var(--shadow);
  }

  .confirm[open] {
    animation: pop-in 0.18s ease-out;
  }

  .confirm::backdrop {
    background: color-mix(in srgb, var(--bg-0) 70%, transparent);
    backdrop-filter: blur(3px);
  }

  h2 {
    margin: 0 0 10px;
  }

  .body {
    color: var(--text-2);
  }

  .list {
    margin: 10px 0 0;
    padding-left: 20px;
    max-height: min(40vh, 320px);
    overflow-y: auto;
    color: var(--text-2);
  }

  .list li + li {
    margin-top: 4px;
  }

  .error {
    margin-top: 12px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }

  @keyframes pop-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
</style>
