<script lang="ts">
  // First-run wizard (ADR-023, part 4): four steps over the whole main window on a fresh install (welcome + language,
  // music, shields, extras), each with one of the owner's axolotl poses and a line in its bubble. Every step changes
  // settings through the same stores and components as Settings (saved at once, like there), and every step can be
  // skipped (Skip / Esc, with a confirmation). Nothing starts by itself: "Start my first focus" only closes the wizard
  // on Skupienie with Start focused. The shell is never waited for (tray, popups and a session started from the tray
  // keep working; a running session shows here with its Stop, CLAUDE.md rule 7).
  // Keyboard: Enter = next (not on buttons, links, text areas and selects, which use Enter themselves), Esc = skip with
  // confirmation, Tab stays inside. The rest of the window is inert while the wizard is open (the app layout).
  import { untrack } from "svelte";
  import AppLogo from "$lib/components/AppLogo.svelte";
  import Background from "$lib/components/Background.svelte";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import MusicSourcePicker from "$lib/components/MusicSourcePicker.svelte";
  import { formatClock } from "$lib/format";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import type { PoseName } from "$lib/mascot/frames";
  import { config } from "$lib/state/config.svelte";
  import { ONBOARDING_STEPS, onboarding, type OnboardingStep } from "$lib/state/onboarding.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";
  import type { Theme } from "$lib/themes";
  import ExtrasStep from "./ExtrasStep.svelte";
  import ShieldsStep from "./ShieldsStep.svelte";
  import WelcomeStep from "./WelcomeStep.svelte";
  import WizardMascot from "./WizardMascot.svelte";

  interface Props {
    /** The theme in use (the window's background scene is drawn behind the wizard too). */
    theme: Theme;
  }

  let { theme }: Props = $props();

  /** The owner's pose for each step (static/mascot/poses-*; ADR-023). */
  const POSES: Record<OnboardingStep, PoseName> = {
    welcome: "02-clasped-hands-heart",
    music: "05-look-up",
    shields: "03-thinking",
    extras: "11-point-happy",
  };

  /** Elements that can take the focus (the Tab trap wraps between the first and the last visible one). */
  const FOCUSABLE =
    'a[href], button:not(:disabled), input:not(:disabled):not([type="hidden"]), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';
  /** Targets that use Enter themselves (Enter on them never means "next"). */
  const TAKES_ENTER = 'button, a[href], textarea, select, summary, [contenteditable="true"], [role="button"], [role="link"]';

  const uid = $props.id();
  const s = strings.onboarding;

  const step = $derived(onboarding.step);
  const texts = $derived(s[step]);
  const total = ONBOARDING_STEPS.length;
  const showMascot = $derived(mascotEnabled(config.value));

  let root = $state<HTMLElement | null>(null);
  let title = $state<HTMLElement | null>(null);
  let body = $state<HTMLElement | null>(null);
  let skipButton = $state<HTMLButtonElement | null>(null);
  let skipOpen = $state(false);
  /** The element of the wizard that took the focus last (`focusin`; not reactive). */
  let lastFocus: Element | null = null;

  // The header's Stop goes away with the session (pressed here or in the tray): when it had the focus, the focus moves
  // to Skip instead of being lost to the page.
  $effect(() => {
    if (!session.isIdle) return;
    untrack(() => {
      const active = document.activeElement;
      const lost = active === null || active === document.body;
      if (lost && lastFocus !== null && !lastFocus.isConnected) skipButton?.focus();
    });
  });

  // A new step: its content starts at the top and its title takes the focus (screen readers hear the new step, and
  // Enter goes on from there).
  $effect(() => {
    void onboarding.index;
    const heading = title;
    untrack(() => {
      body?.scrollTo({ top: 0 });
      heading?.focus({ preventScroll: true });
    });
  });

  function primary(): void {
    if (onboarding.last) void onboarding.finish();
    else onboarding.next();
  }

  function askSkip(): void {
    skipOpen = true;
  }

  /** The visible focusable elements of the wizard, in document order (a closed dialog's buttons are not visible). */
  function focusables(): HTMLElement[] {
    if (!root) return [];
    return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.getClientRects().length > 0);
  }

  /** Keeps Tab inside the wizard (WebView2 could otherwise move the focus out of the page). */
  function trapTab(event: KeyboardEvent): void {
    const items = focusables();
    const first = items[0];
    const last = items[items.length - 1];
    if (!first || !last) {
      event.preventDefault();
      return;
    }
    const active = document.activeElement;
    const inside = root?.contains(active) ?? false;
    if (event.shiftKey && (active === first || !inside)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (active === last || !inside)) {
      event.preventDefault();
      first.focus();
    }
  }

  function takesEnter(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest(TAKES_ENTER) !== null;
  }

  /**
   * On the window, so it also works while nothing has the focus. Dialogs (the skip confirmation, the close prompt) and
   * the toasts keep their own keys.
   */
  function onKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || skipOpen || document.querySelector("dialog[open]")) return;
    const target = event.target;
    const ours = target === document.body || (target instanceof Node && (root?.contains(target) ?? false));
    if (!ours) return;
    if (event.key === "Tab") {
      trapTab(event);
      return;
    }
    if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key === "Escape") {
      event.preventDefault();
      askSkip();
    } else if (event.key === "Enter" && !event.shiftKey && !event.repeat && !takesEnter(target)) {
      event.preventDefault();
      primary();
    }
  }

  /** An in-app link inside a step (e.g. "Calendar in Settings"): the wizard closes and the link opens. */
  function onClick(event: MouseEvent): void {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
      return;
    }
    const link = event.target instanceof Element ? event.target.closest("a[href]") : null;
    const href = link?.getAttribute("href") ?? "";
    if (!href.startsWith("/")) return;
    event.preventDefault();
    void onboarding.leave(href);
  }
</script>

<svelte:head>
  <title>{s.docTitle} · {strings.appName}</title>
</svelte:head>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events (links keep their own keyboard activation; this only routes them) -->
<div
  class="wizard"
  role="dialog"
  tabindex="-1"
  aria-modal="true"
  aria-labelledby="{uid}-title"
  aria-describedby="{uid}-lead"
  bind:this={root}
  onclick={onClick}
  onfocusin={(event) => (lastFocus = event.target instanceof Element ? event.target : null)}
>
  <Background {theme} />

  <header class="top">
    <div class="brand">
      <span class="logo" aria-hidden="true"><AppLogo size={28} /></span>
      <span class="brand-text">
        <span class="name">{strings.appName}</span>
        <span class="setup">{s.dialogLabel}</span>
      </span>
    </div>

    <ol class="progress" aria-label={s.progressLabel}>
      {#each ONBOARDING_STEPS as name, i (name)}
        <li>
          <button
            type="button"
            class="dot-step"
            class:done={i < onboarding.index}
            aria-current={i === onboarding.index ? "step" : undefined}
            onclick={() => onboarding.goTo(i)}
          >
            <span class="num tabular" aria-hidden="true">{i + 1}</span>
            <span class="label">{s.steps[name]}</span>
          </button>
        </li>
      {/each}
    </ol>

    <div class="actions">
      {#if !session.isIdle}
        <!-- A session started from the tray while the wizard is open: never trapped (rule 7). -->
        <span class="phase-pill tabular" class:break={session.phase === "break"}>
          <span class="dot" aria-hidden="true"></span>
          {strings.phase[session.phase]}
          {formatClock(session.snapshot.remainingSecs)}
          {#if session.paused}<span class="sr-only">({strings.timer.paused})</span>{/if}
        </span>
        <button type="button" class="btn btn-sm btn-danger" onclick={() => session.stop()}>
          {strings.controls.stop}
        </button>
      {/if}
      <button type="button" class="btn btn-sm btn-ghost" bind:this={skipButton} onclick={askSkip}>{s.skip}</button>
    </div>
  </header>

  <div class="main" class:with-mascot={showMascot}>
    {#if showMascot}
      <div class="mascot">
        <WizardMascot pose={POSES[step]} line={texts.line} />
      </div>
    {/if}

    <div class="body" bind:this={body}>
      {#key step}
        <section class="step" aria-labelledby="{uid}-title">
          <p class="eyebrow tabular">{s.stepCount(onboarding.index + 1, total)}</p>
          <h1 id="{uid}-title" tabindex="-1" bind:this={title}>{texts.title}</h1>
          <p class="lead secondary" id="{uid}-lead">{texts.lead}</p>

          {#if step === "welcome"}
            <WelcomeStep />
          {:else if step === "music"}
            <MusicSourcePicker />
          {:else if step === "shields"}
            <ShieldsStep />
          {:else}
            <ExtrasStep />
          {/if}
        </section>
      {/key}
    </div>
  </div>

  <footer class="bottom">
    <div class="side">
      {#if !onboarding.first}
        <button type="button" class="btn" onclick={() => onboarding.back()}>
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <path d="M15 5l-7 7 7 7" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          {s.back}
        </button>
      {/if}
    </div>
    <p class="keys small muted" aria-hidden="true">{onboarding.last ? s.keysFinish : s.keysNext}</p>
    <div class="side end">
      {#if onboarding.last}
        <button type="button" class="btn btn-primary btn-lg" onclick={primary}>
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <path d="M7 4.5v15l12.5-7.5z" fill="currentColor" />
          </svg>
          {s.finish}
        </button>
      {:else}
        <button type="button" class="btn btn-primary" onclick={primary}>
          {s.next}
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <path d="M9 5l7 7-7 7" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      {/if}
    </div>
  </footer>
</div>

<ConfirmDialog
  bind:open={skipOpen}
  title={s.skipTitle}
  body={s.skipBody}
  confirmLabel={s.skipConfirm}
  cancelLabel={s.skipCancel}
  onconfirm={() => onboarding.skip()}
/>

<style>
  /* Above the header (20) and the music bar (30), under the toasts (50); dialogs are in the top layer. */
  .wizard {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background-color: var(--bg-0);
    color: var(--text);
    animation: wizard-in 0.24s ease-out;
  }

  /* Everything but the background scene sits above it. */
  .wizard > :not(:global(.backdrop)) {
    position: relative;
    z-index: 1;
  }

  /* ---------- top bar: name, steps, skip ---------- */

  .top {
    display: flex;
    align-items: center;
    gap: 12px 20px;
    flex-wrap: wrap;
    padding: 12px var(--gutter);
    background: var(--header-bg);
    border-bottom: 1px solid var(--header-line);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .logo {
    display: grid;
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }

  .name {
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .setup {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .progress {
    display: flex;
    flex: 1 1 auto;
    justify-content: center;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .dot-step {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px 5px 6px;
    border-radius: var(--radius-pill);
    border: var(--border-w) solid transparent;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition:
      color 0.15s ease,
      background-color 0.15s ease,
      border-color 0.15s ease;
  }

  .dot-step:hover {
    color: var(--text);
    background: var(--surface-inset);
  }

  .num {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: var(--border-w) solid var(--border-strong);
    font-size: 0.8rem;
  }

  .dot-step.done {
    color: var(--text-2);
  }

  .dot-step.done .num {
    border-color: var(--accent-line);
    color: var(--accent-strong);
  }

  .dot-step[aria-current="step"] {
    color: var(--text);
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }

  .dot-step[aria-current="step"] .num {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }

  .phase-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 700;
    font-size: 0.86rem;
  }

  .phase-pill.break {
    background: var(--break-soft);
    color: var(--break);
  }

  .phase-pill .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  /* ---------- the step: the axolotl on the left, the content on the right ---------- */

  .main {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    width: 100%;
    max-width: 1040px;
    min-height: 0;
    margin: 0 auto;
    padding: 0 var(--gutter);
  }

  .main.with-mascot {
    grid-template-columns: 236px minmax(0, 1fr);
    gap: 28px;
  }

  .mascot {
    min-height: 0;
    padding: 20px 0 18px;
  }

  /* Only vertical scrolling: the step's slide-in would otherwise flash a horizontal scrollbar. */
  .body {
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 20px 6px 24px;
    scrollbar-width: thin;
  }

  .step {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 720px;
    animation: step-in 0.28s ease-out;
  }

  .main:not(.with-mascot) .step {
    margin: 0 auto;
  }

  .eyebrow {
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--accent-strong);
  }

  /* The heading takes the focus only so that a new step is announced and Enter goes on from there: it is not a control,
     so it shows no focus ring (the next Tab lands on the step's first control, which does). */
  h1:focus {
    outline: none;
  }

  .lead {
    max-width: 62ch;
    margin-bottom: 8px;
  }

  /* ---------- bottom bar: back, keys, next ---------- */

  .bottom {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 12px;
    padding: 12px var(--gutter);
    background: var(--header-bg);
    border-top: 1px solid var(--header-line);
  }

  .side {
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .side.end {
    justify-content: flex-end;
  }

  .keys {
    text-align: center;
  }

  /* ---------- 8-bit and motion ---------- */

  :global([data-theme="pixel"]) .dot-step,
  :global([data-theme="pixel"]) .num,
  :global([data-theme="pixel"]) .phase-pill {
    border-radius: 0;
    transition: none;
  }

  :global([data-theme="pixel"]) .wizard,
  :global([data-theme="pixel"]) .step {
    animation-timing-function: steps(4, end);
  }

  @keyframes wizard-in {
    from {
      opacity: 0;
    }
  }

  @keyframes step-in {
    from {
      opacity: 0;
      transform: translateX(12px);
    }
  }

  /* Narrower than the minimum window (should never happen on desktop): one column, the axolotl above. */
  @media (max-width: 760px) {
    .main.with-mascot {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto minmax(0, 1fr);
      gap: 0;
    }

    .mascot {
      display: none;
    }

    .progress .label {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wizard,
    .step {
      animation: none;
    }

    .dot-step {
      transition: none;
    }
  }
</style>
