<script lang="ts">
  // Phase popup: a small FocusMode card in the corner of the screen when focus or a break starts (instead of a
  // Windows toast). The shell owns the window (380 × 150, bottom-right, created and destroyed per popup); this
  // page renders `PopupInfo` and reports the buttons and its own timeout through `popup_action`.
  import "../../../app.css";
  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import AppLogo from "$lib/components/AppLogo.svelte";
  import { formatDuration } from "$lib/format";
  import { launcherList, strings } from "$lib/strings";
  import { CATEGORY_ICON, pickBreakTip } from "$lib/tips";
  import type { Config, PopupInfo, PopupKind } from "$lib/types";

  /** How long the popup stays; the countdown pauses while the pointer or the keyboard focus is on it. */
  const VISIBLE_MS = 8000;
  /** Exit animation before the shell hides the window. */
  const LEAVE_MS = 180;
  /** The shell closes the popup after a button; if it does not, the popup closes itself after this delay. */
  const AFTER_ACTION_MS = 1000;

  const KIND_ICON: Record<PopupKind, string> = {
    breakStarted: "☕",
    focusStarted: "🎧",
    interrupted: "💤",
    launchersClosed: "🎮",
  };

  const p = strings.popup;

  let info = $state<PopupInfo | null>(null);
  /** For the wording only (music during the break, shields on); null → the default texts. */
  let cfg = $state<Config | null>(null);
  /** Dismissed by this page (the shell hides the window) until the next popup arrives. */
  let hidden = $state(false);
  let leaving = $state(false);
  /** A button's call is in flight. */
  let busy = $state(false);
  /** A button worked: the buttons stay disabled until the popup goes away or a newer one replaces it. */
  let acted = $state(false);
  /** Countdown bar, 1 → 0. */
  let progress = $state(1);
  /**
   * Session id for the break tip, fetched per popup (PopupInfo has none). The tip waits for it, so this popup and
   * the main window's break card pick the same tip.
   */
  let tipSession = $state<{ revision: number; sessionId: string | null } | null>(null);

  let held = false;
  let remainingMs = VISIBLE_MS;
  let deadline = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let frame = 0;
  let reducedMotion = false;

  const kind = $derived(info?.kind ?? null);
  const visible = $derived(kind !== null && !hidden);
  const canSkip = $derived(kind === "breakStarted" || kind === "focusStarted");

  const title = $derived.by((): string => {
    if (!info?.kind) return "";
    return info.kind === "breakStarted" ? p.breakTitle(formatDuration(info.phaseSecs)) : p.title[info.kind];
  });

  /** The user's own reminder for this break: the main line of a break popup. */
  const reminder = $derived(info?.kind === "breakStarted" ? (info.reminder?.trim() ?? "") || null : null);

  const tip = $derived.by(() => {
    const current = info;
    if (current?.kind !== "breakStarted" || tipSession?.revision !== current.revision) return null;
    return pickBreakTip(tipSession.sessionId, current.cycle, current.phaseSecs);
  });

  const message = $derived.by((): string => {
    const current = info;
    if (!current?.kind) return "";
    const time = formatDuration(current.phaseSecs);
    switch (current.kind) {
      case "breakStarted":
        if (reminder) return reminder;
        if (cfg?.music.source === "none") return p.breakNoMusic;
        if (cfg?.music.pauseDuringBreak) return p.breakMusicPaused;
        return p.breakMusicOn;
      case "focusStarted":
        return cfg && !anyShield(cfg) ? p.focusMessageNoShields(current.cycle, time) : p.focusMessage(current.cycle, time);
      case "interrupted":
        return p.interrupted;
      case "launchersClosed":
        return p.launchersClosed(launcherList(current.launchers));
    }
  });

  function anyShield(c: Config): boolean {
    const s = c.shields;
    return s.muteNotifications || s.teamsDnd || s.blockSites || s.keepAwake || s.closeLaunchers;
  }

  function warn(e: unknown): void {
    const err = api.toAppError(e);
    console.warn(`[FocusMode] ${err.code}: ${err.message}`);
  }

  async function refreshConfig(): Promise<void> {
    try {
      cfg = await api.getConfig();
    } catch (e) {
      warn(e);
    }
  }

  async function refreshTipSession(revision: number): Promise<void> {
    let sessionId: string | null = null;
    try {
      sessionId = (await api.getSnapshot()).sessionId;
    } catch (e) {
      // Without it the tip is still picked (from the break number alone); it may differ from the main window's.
      warn(e);
    }
    if (info?.revision === revision) tipSession = { revision, sessionId };
  }

  function apply(next: PopupInfo): void {
    stopCountdown();
    info = next;
    leaving = false;
    busy = false;
    acted = false;
    if (next.kind === null) {
      hidden = true;
      return;
    }
    hidden = false;
    remainingMs = VISIBLE_MS;
    progress = 1;
    if (!held) runCountdown();
    if (next.kind === "breakStarted" || next.kind === "focusStarted") void refreshConfig();
    if (next.kind === "breakStarted") void refreshTipSession(next.revision);
  }

  // ---------------------------------------------------------------------------
  // Countdown: a timeout decides, animation frames only draw the bar.
  // ---------------------------------------------------------------------------

  function runCountdown(): void {
    stopCountdown();
    if (!visible || busy || leaving) return;
    deadline = performance.now() + remainingMs;
    timer = setTimeout(() => {
      timer = null;
      remainingMs = 0;
      progress = 0;
      void dismiss();
    }, remainingMs);
    frame = requestAnimationFrame(draw);
  }

  /** Freezes the countdown where it is. */
  function stopCountdown(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
      remainingMs = Math.max(0, deadline - performance.now());
    }
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }

  function draw(now: number): void {
    if (timer === null) {
      frame = 0;
      return;
    }
    const left = Math.max(0, deadline - now);
    // Reduced motion: the bar steps once a second instead of sliding.
    const shown = reducedMotion ? Math.ceil(left / 1000) * 1000 : left;
    progress = Math.min(1, shown / VISIBLE_MS);
    frame = requestAnimationFrame(draw);
  }

  function setHeld(next: boolean): void {
    if (held === next) return;
    held = next;
    if (!held) runCountdown();
    else if (!acted) stopCountdown();
  }

  /** Pauses the countdown while the pointer is over the card or focus is inside it. */
  function holdWhileInside(node: HTMLElement): () => void {
    let hover = false;
    let focus = false;
    const update = () => setHeld(hover || focus);
    const enter = () => {
      hover = true;
      update();
    };
    const leave = () => {
      hover = false;
      update();
    };
    const focusIn = () => {
      focus = true;
      update();
    };
    const focusOut = (e: FocusEvent) => {
      focus = e.relatedTarget instanceof Node && node.contains(e.relatedTarget);
      update();
    };
    node.addEventListener("pointerenter", enter);
    node.addEventListener("pointerleave", leave);
    node.addEventListener("focusin", focusIn);
    node.addEventListener("focusout", focusOut);
    return () => {
      node.removeEventListener("pointerenter", enter);
      node.removeEventListener("pointerleave", leave);
      node.removeEventListener("focusin", focusIn);
      node.removeEventListener("focusout", focusOut);
      setHeld(false);
    };
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

  /** Plays the exit animation, then asks the shell to hide the window. A newer popup cancels it. */
  async function dismiss(): Promise<void> {
    const current = info;
    if (!current || !current.kind || hidden || leaving) return;
    stopCountdown();
    leaving = true;
    await sleep(LEAVE_MS);
    if (info?.revision !== current.revision) return;
    try {
      await api.popupAction("dismiss");
      if (info?.revision === current.revision) hidden = true;
    } catch (e) {
      warn(e);
      // The window is still there: keep the card visible so it can be closed again.
      if (info?.revision === current.revision) leaving = false;
    }
  }

  async function act(action: "showMain" | "skipPhase"): Promise<void> {
    const current = info;
    if (!current || busy || acted || leaving) return;
    busy = true;
    stopCountdown();
    try {
      await api.popupAction(action);
    } catch (e) {
      warn(e);
      if (info?.revision === current.revision) {
        busy = false;
        if (!held) runCountdown();
      }
      return;
    }
    // A newer popup (e.g. the next phase after "Pomiń") already replaced this one.
    if (info?.revision !== current.revision) return;
    busy = false;
    acted = true;
    remainingMs = Math.min(remainingMs, AFTER_ACTION_MS);
    runCountdown();
  }

  onMount(() => {
    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    reducedMotion = motion.matches;
    const onMotion = (e: MediaQueryListEvent) => (reducedMotion = e.matches);
    motion.addEventListener("change", onMotion);

    let disposed = false;
    let unlisten: api.UnlistenFn | null = null;
    // Config first, so the first popup already has the right wording; then listen → getter (ADR-012).
    void refreshConfig()
      .then(() => (disposed ? null : api.subscribePopup(apply, warn)))
      .then((u) => {
        if (!u) return;
        if (disposed) u();
        else unlisten = u;
      })
      .catch(warn);

    return () => {
      disposed = true;
      unlisten?.();
      stopCountdown();
      motion.removeEventListener("change", onMotion);
    };
  });
</script>

<svelte:head>
  <title>{strings.appName}</title>
</svelte:head>

{#if visible && info && kind}
  {#key info.revision}
    <div class="popup kind-{kind}" class:leaving role="status" aria-live="polite" aria-atomic="true" {@attach holdWhileInside}>
      <div class="icon" aria-hidden="true">
        <span class="emoji">{KIND_ICON[kind]}</span>
        <span class="badge-logo"><AppLogo size={20} /></span>
      </div>

      <div class="content">
        <p class="title"><span class="sr-only">{strings.appName}: </span>{title}</p>
        <p class="message" class:reminder class:single={kind === "breakStarted"} title={message}>
          {#if reminder}<span class="sr-only">{p.reminderPrefix} </span>{/if}{message}
        </p>
        {#if kind === "breakStarted"}
          <p class="tip" title={tip?.pl}>
            {#if tip}
              <span class="tip-icon" aria-hidden="true">{CATEGORY_ICON[tip.category]}</span>
              <span class="sr-only">{p.tipPrefix(strings.breakView.categories[tip.category])} </span>{tip.pl}
            {/if}
          </p>
        {/if}
        <div class="actions">
          <button type="button" class="btn btn-sm btn-primary" disabled={busy || acted} onclick={() => act("showMain")}>
            {p.showMain}
          </button>
          {#if canSkip}
            <button
              type="button"
              class="btn btn-sm"
              disabled={busy || acted}
              aria-label={kind === "breakStarted" ? p.skipBreak : p.skipFocus}
              onclick={() => act("skipPhase")}
            >
              {p.skip}
            </button>
          {/if}
        </div>
      </div>

      <button type="button" class="close" aria-label={p.dismiss} title={p.dismiss} onclick={() => dismiss()}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </button>

      <div class="countdown" aria-hidden="true">
        <span style:transform="scaleX({progress})"></span>
      </div>
    </div>
  {/key}
{/if}

<style>
  :global(html[data-overlay] body) {
    user-select: none;
    cursor: default;
  }

  /* The card fills the transparent window, leaving room for its shadow inside the window bounds. */
  .popup {
    position: fixed;
    inset: 8px;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 12px;
    padding: 12px 14px 14px 12px;
    overflow: hidden;
    border-radius: var(--radius);
    border: var(--border-w) solid var(--border-strong);
    background:
      radial-gradient(260px 150px at 100% 0%, color-mix(in srgb, var(--glow-warm) 95%, transparent) 0%, transparent 72%),
      radial-gradient(220px 140px at 0% 100%, color-mix(in srgb, var(--glow-cool) 90%, transparent) 0%, transparent 70%),
      linear-gradient(180deg, var(--surface-raised), var(--surface-solid));
    color: var(--text);
    /* Small enough to fit in the 8 px around the card (the window itself is transparent). */
    box-shadow: var(--shadow-sm);
    animation: slide-in 0.32s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .popup.leaving {
    animation: slide-out 0.18s ease-in forwards;
  }

  .icon {
    position: relative;
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    margin-top: 2px;
    border-radius: calc(var(--radius-sm) * 7 / 6);
    background: var(--accent-soft);
    font-size: 1.5rem;
    line-height: 1;
  }

  /* The FocusMode moon on the kind icon, like an app badge. */
  .badge-logo {
    position: absolute;
    right: -6px;
    bottom: -6px;
    display: grid;
    padding: 2px;
    border-radius: 50%;
    background: var(--surface-solid);
  }

  .kind-breakStarted .icon {
    background: var(--break-soft);
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    padding-right: 26px;
    font-weight: 700;
    font-size: 0.98rem;
    line-height: 1.3;
  }

  .kind-breakStarted .title {
    color: var(--break);
  }

  .kind-focusStarted .title {
    color: var(--accent-strong);
  }

  .message {
    color: var(--text-2);
    font-size: 0.84rem;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }

  /* A break popup has a tip line too: one line each keeps the card inside its fixed-size window. */
  .message.single {
    -webkit-line-clamp: 1;
    line-clamp: 1;
  }

  .message.reminder {
    color: var(--text);
    font-weight: 650;
  }

  .tip {
    min-height: 1.35em;
    color: var(--text-muted);
    font-size: 0.8rem;
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tip-icon {
    margin-right: 4px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }

  .actions .btn {
    min-height: 30px;
    padding: 3px 12px;
    font-size: 0.84rem;
  }

  .close {
    position: absolute;
    top: 8px;
    right: 8px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .close:hover {
    color: var(--text);
    background: var(--surface-inset);
  }

  .countdown {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: var(--track);
  }

  .countdown span {
    display: block;
    height: 100%;
    background: var(--accent);
    transform-origin: left center;
  }

  .kind-breakStarted .countdown span {
    background: var(--break);
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateX(36px);
    }
  }

  @keyframes slide-out {
    to {
      opacity: 0;
      transform: translateX(36px);
    }
  }
</style>
