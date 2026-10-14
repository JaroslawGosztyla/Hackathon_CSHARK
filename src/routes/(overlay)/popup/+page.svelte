<script lang="ts">
  // Phase popup: a small FocusMode card in the corner of the screen when focus or a break starts (instead of a
  // Windows toast). The shell owns the window (380 × 150, bottom-right, created and destroyed per popup); this
  // page renders `PopupInfo` and reports the buttons and its own timeout through `popup_action`.
  import "../../../app.css";
  import { onMount, untrack } from "svelte";
  import * as api from "$lib/api";
  import AppLogo from "$lib/components/AppLogo.svelte";
  import SparkIcon from "$lib/components/rewards/SparkIcon.svelte";
  import { FRAME_GEOMETRY } from "$lib/mascot/assets.svelte";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { useMascot } from "$lib/mascot/controller.svelte";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import type { MascotReaction } from "$lib/mascot/frames";
  import RevealText from "$lib/mascot/RevealText.svelte";
  import { useSpeech } from "$lib/mascot/speech.svelte";
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
  /** Exit animation before the shell hides the window (card only / the peeking axolotl dips first). */
  const LEAVE_MS = 180;
  const LEAVE_MS_PEEK = 420;
  /**
   * The peeking axolotl rises from below the card's edge and lands with a squash-and-stretch "boing" (CSS
   * `peek-enter`, 900 ms); with reduced motion it only fades in (200 ms). It speaks once it has settled.
   */
  const ENTER_MS = 900;
  const ENTER_MS_REDUCED = 200;
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
  /**
   * The peeking axolotl's box as Axolotl lays it out (the frame's aspect ratio; `sit` sinks the part below its paws
   * under the edge): the buttons start right of it and their row is at least as tall as the part that shows.
   */
  const PEEK_WIDTH = Math.round((PEEK_SIZE * FRAME_GEOMETRY.peek.frameWidth) / FRAME_GEOMETRY.peek.frameHeight);
  const PEEK_HEIGHT =
    PEEK_SIZE -
    Math.round(((FRAME_GEOMETRY.peek.frameHeight - FRAME_GEOMETRY.peek.anchorY) * PEEK_SIZE) / FRAME_GEOMETRY.peek.frameHeight);

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
  let reducedMotion = $state(false);

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
  /**
   * A developer-mode sample with "Keep popup open" (ADR-019): it stays until X, and its countdown bar stays full
   * (frozen) so a screenshot shows it like a fresh popup. After a button it still closes like any popup.
   */
  const sticky = $derived(info?.sticky === true);

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
  /** Persistent: the message is the popup's content, it stays after the last letter. */
  const speech = useSpeech(peek, { persist: true });
  /** The peeking axolotl (not the small one in a meeting reminder's title) rises into the card. */
  const peeking = $derived(mascotOn && kind !== null && kind !== "meetingReminder");
  /** Revision whose popup the axolotl already reacted to / was asked to say (and the text it was asked to say). */
  let reactedRevision = -1;
  let spokenRevision = $state(-1);
  let spokenText = "";
  /** Revision whose entrance has finished (the axolotl stands still, ready to speak). */
  let settledRevision = $state(-1);
  /**
   * Revision whose message no longer waits for the axolotl: its first letter showed (after the entrance and the
   * reaction), or the text changed meanwhile and is shown plainly.
   */
  let revealedRevision = $state(-1);
  /** The axolotl will say the message of this popup but has not started: the letters stay hidden until then. */
  const speechPending = $derived(
    mascotOn && info !== null && message.trim() !== "" && revealedRevision !== info.revision,
  );
  /** The message is still appearing: the line clamp's "…" stays hidden until the last letter is shown. */
  const messageRevealing = $derived(mascotOn && (speechPending || (speech.line !== null && !speech.complete)));

  /**
   * Beta rewards (ADR-022): the reward line at the end of the title row ("+72 ✦ · Cel dnia ✓ · Seria 5 · Poz. 8", only
   * the parts present; the streak only while it is shown). The shell fills it only while the rewards are active.
   */
  const rewardParts = $derived.by((): { sparks: number; rest: string } | null => {
    const r = info?.rewards;
    if (!r) return null;
    const rs = strings.rewards;
    const parts: string[] = [];
    if (r.dailyGoal) parts.push(rs.popupDailyGoal);
    if (r.streak !== null && cfg?.rewards?.showStreak !== false) parts.push(rs.popupStreak(r.streak));
    if (r.levelUp !== null) parts.push(rs.popupLevel(r.levelUp));
    return { sparks: r.sparks, rest: parts.join(" · ") };
  });
  /** Focus after a break with small steps on: the next step replaces the tip line ("Teraz: …"). */
  const nextStep = $derived(info?.kind === "focusStarted" ? (info.nextStep?.trim() ?? "") || null : null);
  /** A focus popup gets one extra line (reward line and / or next step); its message then keeps to one line. */
  const focusExtra = $derived(info?.kind === "focusStarted" && (nextStep !== null || rewardParts !== null));
  /** The reward line as plain text (tooltip). */
  const rewardText = $derived(
    rewardParts ? [`+${rewardParts.sparks} ✦`, rewardParts.rest].filter((part) => part !== "").join(" · ") : "",
  );

  /** A big reward moment (daily goal, level, streak, a pearl) makes the peeking axolotl happy instead of smiling. */
  function bigReward(popup: PopupInfo): boolean {
    const r = popup.rewards;
    if (!r) return false;
    return r.dailyGoal || r.levelUp !== null || r.streak !== null || r.surprise === "pearl" || r.surprise === "goldenPearl";
  }

  function reactionFor(popup: PopupInfo): MascotReaction | null {
    if (!popup.kind) return null;
    if (popup.kind === "breakStarted" && bigReward(popup)) return "cycle";
    if (popup.kind === "focusStarted") return popup.cycle > 1 ? "cycle" : "start";
    return KIND_REACTION[popup.kind];
  }

  // A new popup: the reaction starts while the axolotl rises (speech waits for both).
  $effect(() => {
    const current = info;
    if (!current?.kind || !visible || !mascotOn) {
      untrack(() => speech.clear());
      return;
    }
    if (current.revision === reactedRevision) return;
    reactedRevision = current.revision;
    untrack(() => {
      speech.clear();
      const reaction = reactionFor(current);
      if (reaction) peek.react(reaction);
    });
  });

  // The entrance of this popup's axolotl ends (a meeting reminder has none).
  $effect(() => {
    const current = info;
    if (!current?.kind || !visible) return;
    const revision = current.revision;
    const delay = peeking ? (reducedMotion ? ENTER_MS_REDUCED : ENTER_MS) : 0;
    const timer = setTimeout(() => (settledRevision = revision), delay);
    return () => clearTimeout(timer);
  });

  // Once settled, it says the message letter by letter (the title is shown at once).
  $effect(() => {
    const current = info;
    const text = message;
    if (!current?.kind || !visible || !mascotOn || settledRevision !== current.revision) return;
    if (spokenRevision === current.revision) {
      // The text changed afterwards (e.g. saving a choice failed): show it plainly.
      if (text !== spokenText) {
        untrack(() => speech.clear());
        revealedRevision = current.revision;
      }
      return;
    }
    spokenRevision = current.revision;
    spokenText = text;
    if (text.trim() !== "") untrack(() => speech.say({ text, urgent: true }));
  });

  // The speech waits for the reaction to end: the letters stay hidden until the first one shows.
  $effect(() => {
    const current = info;
    if (current !== null && speech.line !== null && spokenRevision === current.revision) {
      revealedRevision = current.revision;
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
    if (!visible || busy || leaving || waitsForAnswer || (sticky && !acted)) return;
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
    await sleep(peeking ? (reducedMotion ? ENTER_MS_REDUCED : LEAVE_MS_PEEK) : LEAVE_MS);
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

{#snippet rewardChip(parts: { sparks: number; rest: string })}
  <span class="reward-sparks">+{parts.sparks}<SparkIcon size={11} /></span>{#if parts.rest}<span class="reward-rest"
      >{" · "}{parts.rest}</span
    >{/if}
{/snippet}

{#if visible && info && kind}
  {#key info.revision}
    <div
      class="popup kind-{kind}"
      class:leaving
      class:with-peek={peeking}
      style:--peek-w="{PEEK_WIDTH}px"
      style:--peek-h="{PEEK_HEIGHT}px"
      role="status"
      aria-live="polite"
      aria-atomic="true"
      {@attach holdWhileInside}
    >
      {#if kind !== "meetingReminder" && !mascotOn}
        <div class="icon" aria-hidden="true">
          <span class="emoji">{KIND_ICON[kind]}</span>
          <span class="badge-logo"><AppLogo size={20} /></span>
        </div>
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
          class:single={kind === "breakStarted" || heldReminder !== null || focusExtra}
          class:failed={choiceFailed}
          class:revealing={messageRevealing}
          title={message}
        >
          {#if reminder}<span class="sr-only">{p.reminderPrefix} </span>{/if}<RevealText
            text={message}
            speech={mascotOn ? speech : null}
            waiting={speechPending}
          />
        </p>
        {#if kind === "breakStarted" && rewardParts}
          <!-- Beta rewards: the reward line takes the tip line's place (same window size; the tip is on the break card). -->
          <p class="tip reward-line" title={rewardText}>{@render rewardChip(rewardParts)}</p>
        {:else if kind === "breakStarted"}
          <p class="tip" title={tip ? breakTipText(tip) : undefined}>
            {#if tip}
              <span class="tip-icon" aria-hidden="true">{CATEGORY_ICON[tip.category]}</span>
              <span class="sr-only">{p.tipPrefix(strings.breakView.categories[tip.category])} </span>{breakTipText(tip)}
            {/if}
          </p>
        {/if}
        {#if focusExtra}
          <!-- Beta: back from a break, the reward line and the next small step ("Teraz: …") share one line. -->
          <p class="tip next-step" title={[rewardText, nextStep ? strings.rewards.popupNext(nextStep) : null].filter(Boolean).join(" · ")}>
            {#if rewardParts}<span class="reward-line">{@render rewardChip(rewardParts)}</span>{#if nextStep}{" · "}{/if}{/if}{#if nextStep}{strings.rewards.popupNext(nextStep)}{/if}
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
            {#if peeking}
              <!-- In the card's bottom-left corner, left of the buttons. Inert: a click on it does nothing. -->
              <div class="peek" aria-hidden="true">
                <Axolotl variant="peek" size={PEEK_SIZE} frame={peek.frame} sit />
              </div>
            {/if}
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
    --pad-left: 12px;
    --pad-bottom: 14px;
    position: fixed;
    inset: 8px;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 12px;
    padding: 12px 14px var(--pad-bottom) var(--pad-left);
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

  /* The axolotl dips below the edge first (peek-leave), then the card slides away. */
  .popup.leaving.with-peek {
    animation: slide-out 0.18s ease-in 0.22s forwards;
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

  /*
   * With the peeking axolotl the text spans the whole card, like a popup without it: only the buttons make room for
   * the axolotl. Their row fills the card down to its bottom padding and is at least as tall as the axolotl shows
   * above it, and the axolotl sits in that row's bottom-left corner, so it never reaches a text line.
   */
  .popup.with-peek {
    grid-template-columns: minmax(0, 1fr);
  }

  .with-peek .content {
    align-self: stretch;
  }

  /* The buttons stay where the text column used to start: right of the axolotl (which overlapped the gap by 4 px). */
  .with-peek .actions {
    position: relative;
    flex: 1 0 auto;
    align-content: flex-start;
    align-items: flex-start;
    min-height: calc(var(--peek-h) - var(--pad-bottom));
    padding-left: calc(var(--peek-w) - 4px);
  }

  /* The axolotl peeks over the card's bottom-left edge (into the padding, so it touches the edge). */
  /*
   * Enters from below the card's bottom edge (clipped by the card): a smooth ease-out rise that ends stretched a few
   * pixels above its place, then squashes, rebounds a little and rests — scaled from its bottom centre, like a spring.
   * Inert: a click on it does nothing (it falls through to the card, which has no click action).
   */
  .peek {
    position: absolute;
    left: calc(-1 * var(--pad-left));
    bottom: calc(-1 * var(--pad-bottom));
    pointer-events: none;
    transform-origin: 50% 100%;
    animation: peek-enter 900ms both;
  }

  .popup.leaving .peek {
    animation: peek-leave 250ms cubic-bezier(0.5, 0, 0.75, 0) forwards;
  }

  @keyframes peek-enter {
    0% {
      transform: translateY(calc(100% + 18px)) scale(1, 1);
      animation-timing-function: cubic-bezier(0.2, 0.75, 0.3, 1);
    }
    61% {
      transform: translateY(-5px) scale(0.88, 1.14);
      animation-timing-function: cubic-bezier(0.4, 0, 0.5, 1);
    }
    77% {
      transform: translateY(0) scale(1.06, 0.94);
      animation-timing-function: cubic-bezier(0.4, 0, 0.5, 1);
    }
    90% {
      transform: translateY(-1px) scale(0.98, 1.02);
      animation-timing-function: cubic-bezier(0.4, 0, 0.5, 1);
    }
    100% {
      transform: none;
    }
  }

  @keyframes peek-leave {
    from {
      transform: none;
    }
    to {
      transform: translateY(calc(100% + 18px)) scale(0.96, 1.04);
    }
  }

  @keyframes peek-fade-in {
    from {
      opacity: 0;
    }
  }

  @keyframes peek-fade-out {
    to {
      opacity: 0;
    }
  }

  /* Reduced motion: a plain 200 ms fade, no movement (wins over app.css's global "no animation" rule on purpose). */
  @media (prefers-reduced-motion: reduce) {
    .peek {
      animation-name: peek-fade-in !important;
      animation-duration: 200ms !important;
      animation-timing-function: ease-out !important;
    }

    .popup.leaving .peek {
      animation-name: peek-fade-out !important;
      animation-duration: 200ms !important;
      animation-timing-function: ease-in !important;
    }
  }

  /* A meeting reminder needs the whole width for its buttons: a small axolotl peeks over the title instead. */
  .title-peek {
    display: inline-block;
    margin-right: 6px;
    vertical-align: -7px;
    pointer-events: none;
  }

  /* A meeting reminder has four buttons: no icon column, so they get the whole width of the card. */
  .popup.kind-meetingReminder {
    grid-template-columns: minmax(0, 1fr);
    padding: 10px 14px 12px;
  }

  .title-icon {
    margin-right: 6px;
  }

  /* Beta rewards: the reward line ("+72 ✦ · Cel dnia ✓ · Seria 5 · Poz. 8"), one line like the tip it replaces. */
  .reward-line,
  .tip.reward-line {
    color: var(--reward-ink);
    font-weight: 700;
  }

  .reward-sparks {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    vertical-align: -1px;
  }

  .tip.next-step {
    color: var(--text-2);
    font-weight: 600;
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
    --message-color: var(--text-2);
    color: var(--message-color);
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
    --message-color: var(--text);
    font-weight: 650;
  }

  .message.failed {
    --message-color: var(--danger);
  }

  /* The clamp's "…" is drawn in the paragraph's colour: hidden while the letters still appear. */
  .message.revealing {
    color: transparent;
  }

  .message.revealing :global(.reveal) {
    color: var(--message-color);
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
