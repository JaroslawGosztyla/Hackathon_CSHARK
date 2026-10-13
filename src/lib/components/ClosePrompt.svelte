<script lang="ts">
  // First X on the main window while `closeToTray` is undecided: the shell keeps the window open and sends
  // `app://close-requested`; the answer goes back through `resolve_close` (stored, then hide or quit).
  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import AppLogo from "$lib/components/AppLogo.svelte";
  import { config } from "$lib/state/config.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { errorText, strings } from "$lib/strings";

  const s = strings.closePrompt;

  let dialog = $state<HTMLDialogElement | null>(null);
  let primary = $state<HTMLButtonElement | null>(null);
  let busy = $state(false);
  /** Shown inside the dialog: toasts would sit under the modal backdrop. */
  let error = $state<string | null>(null);
  let restoreFocus: HTMLElement | null = null;

  onMount(() => {
    let disposed = false;
    let unlisten: api.UnlistenFn | null = null;
    api
      .listenCloseRequested(show)
      .then((u) => {
        if (disposed) u();
        else unlisten = u;
      })
      .catch((e: unknown) => toasts.error(e));
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function show(): void {
    if (!dialog || dialog.open) return;
    restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    error = null;
    dialog.showModal();
    primary?.focus();
  }

  /** `close` fires for the buttons and for Escape (the dialog's `cancel`), which just keeps the app open. */
  function onClosed(): void {
    const target = restoreFocus;
    restoreFocus = null;
    if (target?.isConnected) target.focus();
  }

  async function answer(toTray: boolean): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const saved = await api.resolveClose(toTray);
      // The shell stored the choice (and turned on popups for the tray); keep the settings page in sync.
      config.adopt(saved, ["closeToTray", "phasePopups"]);
      dialog?.close();
    } catch (e) {
      const err = api.toAppError(e);
      console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      error = errorText(err);
    } finally {
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
  class="close-prompt"
  aria-labelledby="close-prompt-title"
  aria-describedby="close-prompt-body"
  onclose={onClosed}
>
  <div class="head">
    <span class="logo"><AppLogo size={40} /></span>
    <h2 id="close-prompt-title">{s.title}</h2>
  </div>
  <div id="close-prompt-body" class="body">
    <p>{s.body}</p>
    <p class="hint">{s.note}</p>
  </div>
  {#if error}
    <p class="field-error error" role="alert">{error}</p>
  {/if}
  <div class="actions">
    <button bind:this={primary} type="button" class="btn btn-primary" disabled={busy} onclick={() => answer(true)}>
      {s.toTray}
    </button>
    <button type="button" class="btn" disabled={busy} onclick={() => answer(false)}>
      {s.quit}
    </button>
  </div>
</dialog>

<style>
  .close-prompt {
    width: min(460px, calc(100vw - 32px));
    padding: 24px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background:
      radial-gradient(420px 220px at 100% 0%, rgba(242, 166, 90, 0.12) 0%, transparent 70%),
      var(--surface-raised);
    color: var(--text);
    box-shadow: var(--shadow);
  }

  .close-prompt[open] {
    animation: pop-in 0.18s ease-out;
  }

  .close-prompt::backdrop {
    background: rgba(8, 4, 14, 0.62);
    backdrop-filter: blur(3px);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .logo {
    display: grid;
    flex: none;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--text-2);
  }

  .error {
    margin-top: 12px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 20px;
  }

  .actions .btn {
    flex: 1 1 auto;
  }

  @keyframes pop-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
</style>
