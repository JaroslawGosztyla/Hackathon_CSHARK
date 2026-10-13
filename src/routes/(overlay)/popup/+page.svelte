<script lang="ts">
  // Phase popup: a small FocusMode card in the corner of the screen when focus or a break starts (instead of a
  // Windows toast). The shell owns the window (380 × 150, bottom-right, created and destroyed per popup); this
  // page renders `PopupInfo` and reports the buttons and its own timeout through `popup_action`.
  import "../../../app.css";
  import { onMount, untrack } from "svelte";
  import * as api from "$lib/api";
  import AppLogo from "$lib/components/AppLogo.svelte";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { useMascot } from "$lib/mascot/controller.svelte";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import type { MascotReaction } from "$lib/mascot/frames";
  import { formatDuration, formatTimeRange } from "$lib/format";
  import { launcherList, strings } from "$lib/strings";
  import { breakTipText, CATEGORY_ICON, pickBreakTip } from "$lib/tips";
  import type { Config, MeetingChoice, PopupInfo, PopupKind } from "$lib/types";

  /** How long the popup stays; the countdown pauses while the pointer or the keyboard focus is on it. */
  const VISIBLE_MS = 8000;
  /** Popups that explain something stay longer. */
  const VISIBLE_MS_BY_KIND: Partial<Record<PopupKind, number>> = {
    breakHeld: 15_000,
    workDigest: 12_000,
  };
  /** Buttons of a meeting reminder; "Przypomnij później" only while `remindLaterAllowed` (not on the last one). */
  const MEETING_CHOICES: readonly MeetingChoice[] = ["extend", "pauseAtBreak", "ignore", "remindLater"];
  /** Exit animation before the shell hides the window. */
  const LEAVE_MS = 180;
  /** The shell closes the popup after a button; if it does not, the popup closes itself after this delay. */
  const AFTER_ACTION_MS = 1000;

  const KIND_ICON: Record<PopupKind, string> = {
    breakStarted: "☕",
    focusStarted: "🎧",
    interrupted: "💤",
    launchersClosed: "🎮",
    meetingReminder: "📅",
    breakHeld: "⏸️",
    workDigest: "📋",
  };

  /** The peeking axolotl's reaction to each popup (a session start winks, a completed cycle cheers). */
  const KIND_REACTION: Record<Exclude<PopupKind, "focusStarted">, MascotReaction> = {
    breakStarted: "break",
    interrupted: "stopEarly",
    launchersClosed: "nudge",
    meetingReminder: "meeting",
    breakHeld: "meeting",
    workDigest: "digest",
  };
  /** Height of the peeking axolotl at the card's bottom-left edge, and of the small one in a meeting reminder's title. */
  const PEEK_SIZE = 50;
  const PEEK_SIZE_TITLE = 26;

  const p = strings.popup;
  const cal = strings.calendar;

  let info = $state<PopupInfo | null>(null);
  /** For the wording (music during the break, shields on) and the mascot switch; null → the default texts. */
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
  /** Saving the meeting choice failed (the popup then says to choose in the main window). */
  let choiceFailed = $state(false);
  /**
   * Session id for the break tip, fetched per popup (PopupInfo has none). The tip waits for it, so this popup and
   * the main window's break card pick the same tip.
   */
  let tipSession = $state<{ revision: number; sessionId: string | null } | null>(null);

  let held = false;
  /** Visible time of the current popup (depends on its kind). */
  let totalMs = VISIBLE_MS;
  let remainingMs = VISIBLE_MS;
  let deadline = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let frame = 0;
  let reducedMotion = false;

  const kind = $derived(info?.kind ?? null);
  const visible = $derived(kind !== null && !hidden);
  const canSkip = $derived(kind === "breakStarted" || kind === "focusStarted");
  /** The meeting of a reminder / held break (may be missing for a held break). */
  const meeting = $derived(kind === "meetingReminder" || kind === "breakHeld" ? (info?.meeting ?? null) : null);
  const meetingSubject = $derived(meeting ? meeting.meeting.subject.trim() || cal.untitled : "");
  /** A meeting longer than 4 h cannot extend the focus or hold the break: those buttons are left out (note instead). */
  const meetingChoices = $derived(
    meeting
      ? MEETING_CHOICES.filter(
          (choice) =>
            (choice !== "remindLater" || meeting.remindLaterAllowed) &&
            !(meeting.tooLong === true && (choice === "extend" || choice === "pauseAtBreak")),
        )
      : [],
  );
  /**
   * A meeting reminder asks a question: no countdown, it stays until a button or X (the shell, which sizes the
   * window taller for it, hides it after a long upper bound).
   */
  const waitsForAnswer = $derived(kind === "meetingReminder" && meeting !== null);

  const title = $derived.by((): string => {
    if (!info?.kind) return "";
    if (info.kind === "breakStarted") return p.breakTitle(formatDuration(info.phaseSecs));
    if (info.kind === "meetingReminder" && meeting) return p.meetingTitle(meeting.breakNumber);
    if (info.kind === "breakHeld" && info.phaseSecs > 0) return p.breakHeldTitle(formatDuration(info.phaseSecs));
    return p.title[info.kind];
  });

  /** The user's own reminder for this break: the main line of a break popup. */
  const reminder = $derived(info?.kind === "breakStarted" ? (info.reminder?.trim() ?? "") || null : null);
  /** A held break shows the meeting first and the user's reminder for that break on its own line. */
  const heldReminder = $derived(info?.kind === "breakHeld" ? (info.reminder?.trim() ?? "") || null : null);

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
      case "meetingReminder":
        if (choiceFailed) return p.choiceFailed;
        return meeting
          ? p.meetingMessage(formatTimeRange(meeting.meeting.startMs, meeting.meeting.endMs), meetingSubject)
          : "";
      case "breakHeld":
        return meeting ? p.breakHeld(meetingSubject) : p.breakHeldGeneric;
      case "workDigest":
        // Only a count: titles of work items never appear in the popup.
        return current.workCount > 0 ? p.workDigest(current.workCount) : p.workDigestGeneric;
    }
  });

  // ---------------------------------------------------------------------------
  // Mascot: the peeking axolotl reacts to the popup, then says its title and message (lip-sync).
  // ---------------------------------------------------------------------------

  const mascotOn = $derived(mascotEnabled(cfg));
  const peek = useMascot("peek");
  /** Revision whose popup the axolotl already reacted to. */
  let spokenRevision = -1;

  function reactionFor(popup: PopupInfo): MascotReaction | null {
    if (!popup.kind) return null;
    if (popup.kind === "focusStarted") return popup.cycle > 1 ? "cycle" : "start";
    return KIND_REACTION[popup.kind];
  }

  $effect(() => {
    const current = info;
    if (!current?.kind || !visible || !mascotOn) {
      untrack(() => peek.hush());
      return;
    }
    if (current.revision === spokenRevision) return;
    spokenRevision = current.revision;
    const text = [title, message].filter((part) => part.trim() !== "").join(". ");
    untrack(() => {
      const reaction = reactionFor(current);
      if (reaction) peek.react(reaction);
      peek.talk(text);
    });
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
    choiceFailed = false;
    if (next.kind === null) {
      hidden = true;
      return;
    }
    hidden = false;
    totalMs = VISIBLE_MS_BY_KIND[next.kind] ?? VISIBLE_MS;
    remainingMs = totalMs;
    progress = 1;
    if (!held) runCountdown();
    // Every popup reads the config again: the wording and the mascot switch follow the latest settings.
    void refreshConfig();
    if (next.kind === "breakStarted") void refreshTipSession(next.revision);
  }

  // ---------------------------------------------------------------------------
  // Countdown: a timeout decides, animation frames only draw the bar.
  // ---------------------------------------------------------------------------

  function runCountdown(): void {
    stopCountdown();
    if (!visible || busy || leaving || waitsForAnswer) return;
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
    progress = Math.min(1, shown / totalMs);
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

  /** A choice button of a meeting reminder: store it, then close the popup. */
  async function choose(choice: MeetingChoice): Promise<void> {
    const current = info;
    const target = meeting;
    if (!current || !target || busy || acted || leaving) return;
    busy = true;
    stopCountdown();
    try {
      await api.setMeetingChoice(target.meeting.id, choice);
    } catch (e) {
      warn(e);
      if (info?.revision === current.revision) {
        busy = false;
        choiceFailed = true;
        if (!held) runCountdown();
      }
      return;
    }
    if (info?.revision !== current.revision) return;
    busy = false;
    acted = true;
    await dismiss();
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
      {#if kind !== "meetingReminder"}
        {#if mascotOn}
          <div class="peek" aria-hidden="true">
            <Axolotl variant="peek" size={PEEK_SIZE} frame={peek.frame} sit />
          </div>
        {:else}
          <div class="icon" aria-hidden="true">
            <span class="emoji">{KIND_ICON[kind]}</span>
            <span class="badge-logo"><AppLogo size={20} /></span>
          </div>
        {/if}
      {/if}

      <div class="content">
        <p class="title">
          <span class="sr-only">{strings.appName}: </span>
          {#if kind === "meetingReminder" && mascotOn}<span class="title-peek" aria-hidden="true"
              ><Axolotl variant="peek" size={PEEK_SIZE_TITLE} frame={peek.frame} /></span
            >{/if}{#if kind === "meetingReminder" || mascotOn}<span class="title-icon" aria-hidden="true"
              >{KIND_ICON[kind]}</span
            >{/if}{title}
        </p>
        <p
          class="message"
          class:reminder
          class:single={kind === "breakStarted" || heldReminder !== null}
          class:failed={choiceFailed}
          title={message}
        >
          {#if reminder}<span class="sr-only">{p.reminderPrefix} </span>{/if}{message}
        </p>
        {#if kind === "breakStarted"}
          <p class="tip" title={tip ? breakTipText(tip) : undefined}>
            {#if tip}
              <span class="tip-icon" aria-hidden="true">{CATEGORY_ICON[tip.category]}</span>
              <span class="sr-only">{p.tipPrefix(strings.breakView.categories[tip.category])} </span>{breakTipText(tip)}
            {/if}
          </p>
        {/if}
        {#if heldReminder}
          <p class="tip" title={heldReminder}>
            <span class="sr-only">{p.reminderPrefix} </span>{heldReminder}
          </p>
        {/if}
        {#if waitsForAnswer && meeting}
          <p class="tip tabular">{p.meetingBreakAt(formatTimeRange(meeting.breakStartMs, meeting.breakEndMs))}</p>
          {#if meeting.tooLong}
            <p class="tip" title={cal.tooLong}>{cal.tooLong}</p>
          {/if}
        {/if}
        {#if waitsForAnswer}
          <div class="actions choices" role="group" aria-label={p.meetingChoices(meetingSubject)}>
            {#each meetingChoices as choice, i (choice)}
              <button
                type="button"
                class="btn btn-sm"
                class:btn-primary={i === 0}
                disabled={busy || acted}
                onclick={() => choose(choice)}
              >
                {cal.choice[choice]}
              </button>
            {/each}
          </div>
        {:else}
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
        {/if}
      </div>

      <button type="button" class="close" aria-label={p.dismiss} title={p.dismiss} onclick={() => dismiss()}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </button>

      {#if !waitsForAnswer}
        <div class="countdown" aria-hidden="true">
          <span style:transform="scaleX({progress})"></span>
        </div>
      {/if}
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

  .kind-breakStarted .icon,
  .kind-breakHeld .icon {
    background: var(--break-soft);
  }

  /* The axolotl peeks over the card's bottom-left edge (into the padding, so it touches the edge). */
  .peek {
    align-self: end;
    margin: 0 -4px -14px -12px;
  }

  /* A meeting reminder needs the whole width for its buttons: a small axolotl peeks over the title instead. */
  .title-peek {
    display: inline-block;
    margin-right: 6px;
    vertical-align: -7px;
  }

  /* A meeting reminder has four buttons: no icon column, so they get the whole width of the card. */
  .popup.kind-meetingReminder {
    grid-template-columns: minmax(0, 1fr);
    padding: 10px 14px 12px;
  }

  .title-icon {
    margin-right: 6px;
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

  .kind-breakStarted .title,
  .kind-breakHeld .title {
    color: var(--break);
  }

  .kind-focusStarted .title,
  .kind-meetingReminder .title,
  .kind-workDigest .title {
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

  .message.failed {
    color: var(--danger);
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

  /* Two columns of choices; a long label wraps to a second line (the window is taller for a reminder). */
  .actions.choices {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    margin-top: 8px;
  }

  .actions.choices .btn {
    min-height: 34px;
    padding: 4px 8px;
    font-size: 0.8rem;
    line-height: 1.25;
    white-space: normal;
    text-align: center;
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

  .kind-breakStarted .countdown span,
  .kind-breakHeld .countdown span {
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
