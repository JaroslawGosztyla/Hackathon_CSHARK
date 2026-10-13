<script lang="ts">
  // Shell of the main window. The overlay window (/popup) uses the (overlay) group instead, so the
  // music player, the subscriptions and the toasts run in exactly one window.
  import "../../app.css";
  import { onMount, type Snippet } from "svelte";
  import { MediaQuery } from "svelte/reactivity";
  import { onNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import AppLogo from "$lib/components/AppLogo.svelte";
  import Background from "$lib/components/Background.svelte";
  import ClosePrompt from "$lib/components/ClosePrompt.svelte";
  import NudgeBanner from "$lib/components/NudgeBanner.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import { formatClock } from "$lib/format";
  import { followLanguage } from "$lib/i18n/index.svelte";
  import MusicPlayer from "$lib/player/MusicPlayer.svelte";
  import { initAppState } from "$lib/state/app";
  import { config } from "$lib/state/config.svelte";
  import { music } from "$lib/state/music.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";
  import { applyTheme, DEFAULT_THEME, isTheme, readCachedTheme, type Theme } from "$lib/themes";

  let { children }: { children: Snippet } = $props();

  onMount(() => initAppState());

  /** In tab order: the highlight, the background pan and the page slide all follow this order. */
  const links = [
    {
      href: "/",
      get label() {
        return strings.nav.focus;
      },
    },
    {
      href: "/summary",
      get label() {
        return strings.nav.summary;
      },
    },
    {
      href: "/settings",
      get label() {
        return strings.nav.settings;
      },
    },
  ];

  const path = $derived(page.url.pathname);
  const reserveVideo = $derived(music.videoVisible && music.videoExpanded);
  const reducedMotion = new MediaQuery("(prefers-reduced-motion: reduce)");

  // ---------------------------------------------------------------------------
  // Theme: config.theme on <html data-theme>; until the config arrives, the theme of the last run.
  // ---------------------------------------------------------------------------

  const startTheme: Theme = readCachedTheme() ?? DEFAULT_THEME;
  applyTheme(startTheme);

  const theme = $derived.by((): Theme => {
    const configured = config.value?.theme;
    return isTheme(configured) ? configured : startTheme;
  });

  $effect.pre(() => applyTheme(theme));

  // ---------------------------------------------------------------------------
  // Language: config.language (`system` = the OS UI language); until the config arrives, the language of the last run
  // (`$lib/i18n` applies it before the first paint). A switch in Settings applies here at once.
  // ---------------------------------------------------------------------------

  const languageChoice = $derived(config.value?.language);
  $effect.pre(() => followLanguage(languageChoice));

  // ---------------------------------------------------------------------------
  // Navigation: one sliding highlight, a background pan by tab index, page slides (View Transitions).
  // ---------------------------------------------------------------------------

  function tabIndexOf(pathname: string | undefined): number {
    if (pathname === undefined) return -1;
    return links.findIndex((l) => (l.href === "/" ? pathname === "/" : pathname.startsWith(l.href)));
  }

  const activeIndex = $derived(tabIndexOf(path));
  const panIndex = $derived(Math.max(0, activeIndex));

  interface HighlightBox {
    x: number;
    y: number;
    w: number;
    h: number;
    /** Animate into place (the active tab changed); window resizes and font changes jump instead. */
    slide: boolean;
  }

  let highlight = $state<HighlightBox | null>(null);
  let navEl: HTMLElement | null = null;
  let placedIndex = -1;

  /** Moves the highlight under the active link (measured, so it follows any label width or wrapping). */
  function placeHighlight(index: number): void {
    const link = navEl?.querySelector<HTMLElement>('a[aria-current="page"]');
    if (!navEl || !link || index < 0) {
      highlight = null;
      placedIndex = -1;
      return;
    }
    const navBox = navEl.getBoundingClientRect();
    const box = link.getBoundingClientRect();
    // The highlight is positioned inside the nav's border; computed widths keep sub-pixel borders exact.
    const navStyle = getComputedStyle(navEl);
    const slide = placedIndex !== -1 && placedIndex !== index;
    placedIndex = index;
    highlight = {
      x: box.left - navBox.left - (parseFloat(navStyle.borderLeftWidth) || 0),
      y: box.top - navBox.top - (parseFloat(navStyle.borderTopWidth) || 0),
      w: box.width,
      h: box.height,
      slide,
    };
  }

  // After the DOM shows the new aria-current.
  $effect(() => placeHighlight(activeIndex));

  /** Re-measures when the nav or a link changes size (window width, theme font, wrapping on small screens). */
  function observeNav(nav: HTMLElement): () => void {
    navEl = nav;
    const observer = new ResizeObserver(() => placeHighlight(tabIndexOf(page.url.pathname)));
    observer.observe(nav);
    for (const link of nav.querySelectorAll("a")) observer.observe(link);
    return () => {
      observer.disconnect();
      navEl = null;
    };
  }

  /** Longest SvelteKit waits for the transition to take its "before" snapshot. */
  const VIEW_TRANSITION_START_TIMEOUT_MS = 1000;
  let activeTransition: ViewTransition | null = null;

  // The page content slides in the direction of the tab order; the header and the music bar stay in place
  // (app.css names them only while `data-nav-transition` is set).
  onNavigate((navigation) => {
    if (reducedMotion.current || typeof document.startViewTransition !== "function") return;
    // E.g. `goto("/summary")` while the window sits in the tray: nothing to animate, do not wait for a frame.
    if (document.visibilityState !== "visible") return;
    const from = tabIndexOf(navigation.from?.url.pathname);
    const to = tabIndexOf(navigation.to?.url.pathname);
    if (from < 0 || to < 0 || from === to) return;

    const root = document.documentElement;
    root.dataset.navTransition = to > from ? "forward" : "back";

    return new Promise<void>((resolve) => {
      // Never block the navigation, whatever happens to the transition.
      const fallback = setTimeout(resolve, VIEW_TRANSITION_START_TIMEOUT_MS);
      let transition: ViewTransition;
      try {
        transition = document.startViewTransition(async () => {
          clearTimeout(fallback);
          resolve();
          await navigation.complete;
        });
      } catch {
        clearTimeout(fallback);
        delete root.dataset.navTransition;
        resolve();
        return;
      }
      activeTransition = transition;
      const finish = () => {
        // A newer navigation replaced this transition and owns the attribute now.
        if (activeTransition !== transition) return;
        activeTransition = null;
        delete root.dataset.navTransition;
      };
      // A skipped transition (aborted navigation, hidden window) rejects these; nothing to report.
      transition.ready.catch(() => {});
      transition.updateCallbackDone.catch(() => {});
      transition.finished.then(finish, finish);
    });
  });
</script>

<Background {theme} pan={panIndex} />

<div class="app" class:with-video={reserveVideo}>
  <header class="top">
    <a class="brand" href="/">
      <span class="logo" aria-hidden="true"><AppLogo /></span>
      <span class="brand-text">
        <span class="name">{strings.appName}</span>
        <span class="tagline">{strings.tagline}</span>
      </span>
    </a>

    <nav aria-label={strings.nav.label} class:measured={highlight !== null} {@attach observeNav}>
      {#if highlight}
        <span
          class="highlight"
          class:slide={highlight.slide}
          style:width="{highlight.w}px"
          style:height="{highlight.h}px"
          style:transform="translate({highlight.x}px, {highlight.y}px)"
          aria-hidden="true"
        ></span>
      {/if}
      {#each links as link, i (link.href)}
        <a href={link.href} aria-current={i === activeIndex ? "page" : undefined}>{link.label}</a>
      {/each}
    </nav>

    {#if !session.isIdle && path !== "/"}
      <a class="phase-pill tabular" class:break={session.phase === "break"} href="/">
        <span class="dot" aria-hidden="true"></span>
        {strings.phase[session.phase]}
        {formatClock(session.snapshot.remainingSecs)}
        {#if session.paused}
          <span class="sr-only">({strings.timer.paused})</span>
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
            <rect x="3" y="2.5" width="3.5" height="11" rx="1" fill="currentColor" />
            <rect x="9.5" y="2.5" width="3.5" height="11" rx="1" fill="currentColor" />
          </svg>
        {/if}
      </a>
    {/if}
  </header>

  <main>
    <NudgeBanner />
    {@render children()}
  </main>
</div>

<MusicPlayer />
<Toasts />
<ClosePrompt />
<p class="sr-only" aria-live="polite" aria-atomic="true">{session.announcement}</p>

<style>
  .app {
    position: relative;
    z-index: 1;
    min-height: 100vh;
    padding: 0 var(--gutter) calc(var(--dock-h) + 32px);
  }

  .app.with-video {
    padding-bottom: calc(var(--dock-h) + 250px);
  }

  /* Pinned: logo + navigation stay visible while the page scrolls under them. */
  .top {
    position: sticky;
    top: 0;
    z-index: 20;
    max-width: 1040px;
    margin: 0 auto 4px;
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    padding: 16px 0;
  }

  /* Full-window-width backdrop behind the centred header, so scrolled content never shows through. */
  .top::before {
    content: "";
    position: absolute;
    inset: 0 calc(50% - 50vw);
    z-index: -1;
    background: var(--header-bg);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    border-bottom: 1px solid var(--header-line);
  }

  /* View Transitions (app.css): only while a tab change animates, so the header keeps its live backdrop blur. */
  :global(html[data-nav-transition]) .top {
    view-transition-name: app-header;
  }

  :global(html[data-nav-transition]) main {
    view-transition-name: page;
  }

  /* The music bar (MusicPlayer's root <section class="dock">) stays above the sliding page. */
  :global(html[data-nav-transition] section.dock) {
    view-transition-name: music-dock;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text);
    text-decoration: none;
    margin-right: auto;
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
    font-size: 1.05rem;
    letter-spacing: -0.01em;
  }

  .tagline {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  nav {
    position: relative;
    isolation: isolate;
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: var(--radius-pill);
    background: var(--surface);
    border: var(--border-w) solid var(--border);
  }

  /* One highlight for the whole nav: it slides to the active tab instead of each link lighting up. */
  .highlight {
    position: absolute;
    top: 0;
    left: 0;
    z-index: -1;
    border-radius: var(--radius-pill);
    background: var(--nav-highlight);
    pointer-events: none;
  }

  .highlight.slide {
    transition:
      transform var(--nav-slide-ms) var(--ease-slide),
      width var(--nav-slide-ms) var(--ease-slide),
      height var(--nav-slide-ms) var(--ease-slide);
  }

  :global([data-theme="pixel"]) .highlight {
    box-shadow: 2px 2px 0 0 var(--shadow-color);
  }

  nav a {
    padding: 7px 14px;
    border-radius: var(--radius-pill);
    color: var(--text-2);
    text-decoration: none;
    font-weight: 600;
    font-size: 0.92rem;
    transition:
      background-color 0.15s ease,
      color var(--nav-slide-ms) var(--ease-slide);
  }

  nav a:hover {
    color: var(--text);
    background: var(--surface-inset);
  }

  nav a[aria-current="page"] {
    color: var(--nav-highlight-ink);
  }

  /* The highlight sits behind the active link; until it is measured, the link lights up by itself. */
  nav.measured a[aria-current="page"],
  nav.measured a[aria-current="page"]:hover {
    background: transparent;
  }

  nav:not(.measured) a[aria-current="page"] {
    background: var(--nav-highlight);
  }

  .phase-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 700;
    font-size: 0.9rem;
    text-decoration: none;
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

  main {
    max-width: 1040px;
    margin: 0 auto;
  }

  @media (prefers-reduced-motion: reduce) {
    .highlight.slide,
    nav a {
      transition: none;
    }
  }
</style>
