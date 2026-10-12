<script lang="ts">
  import "../app.css";
  import { onMount, type Snippet } from "svelte";
  import { page } from "$app/state";
  import Background from "$lib/components/Background.svelte";
  import NudgeBanner from "$lib/components/NudgeBanner.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import { formatClock } from "$lib/format";
  import MusicPlayer from "$lib/player/MusicPlayer.svelte";
  import { initAppState } from "$lib/state/app";
  import { music } from "$lib/state/music.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";

  let { children }: { children: Snippet } = $props();

  onMount(() => initAppState());

  const links = [
    { href: "/", label: strings.nav.focus },
    { href: "/summary", label: strings.nav.summary },
    { href: "/settings", label: strings.nav.settings },
  ];

  const path = $derived(page.url.pathname);
  const isActive = (href: string) => (href === "/" ? path === "/" : path.startsWith(href));
  const reserveVideo = $derived(music.videoVisible && music.videoExpanded);
</script>

<Background />

<div class="app" class:with-video={reserveVideo}>
  <header class="top">
    <a class="brand" href="/">
      <span class="logo" aria-hidden="true">
        <svg viewBox="0 0 32 32" width="30" height="30">
          <circle cx="16" cy="16" r="15" fill="rgba(242,166,90,0.16)" />
          <path d="M21.5 19.5A7.5 7.5 0 1 1 13 9a6 6 0 0 0 8.5 10.5z" fill="#f2a65a" />
          <circle cx="22.5" cy="10" r="1.2" fill="#ffe2bd" />
        </svg>
      </span>
      <span class="brand-text">
        <span class="name">{strings.appName}</span>
        <span class="tagline">{strings.tagline}</span>
      </span>
    </a>

    <nav aria-label={strings.nav.label}>
      {#each links as link (link.href)}
        <a href={link.href} aria-current={isActive(link.href) ? "page" : undefined}>{link.label}</a>
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

  .top {
    max-width: 1040px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    padding: 16px 0 20px;
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
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: var(--radius-pill);
    background: var(--surface);
    border: 1px solid var(--border);
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
      color 0.15s ease;
  }

  nav a:hover {
    color: var(--text);
    background: var(--surface-inset);
  }

  nav a[aria-current="page"] {
    color: var(--accent-ink);
    background: var(--accent);
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

  @media (max-width: 560px) {
    .tagline {
      display: none;
    }
    nav {
      order: 3;
      width: 100%;
      justify-content: space-between;
    }
    nav a {
      flex: 1;
      text-align: center;
      padding: 7px 8px;
    }
  }
</style>
