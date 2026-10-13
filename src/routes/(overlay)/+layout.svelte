<script lang="ts">
  // Transparent overlay windows (the phase popup). No app shell here: no navigation, no toasts,
  // no subscriptions of the main window and, above all, no music player (music must play in one window only).
  import { onMount, type Snippet } from "svelte";
  import { browser } from "$app/environment";
  import * as api from "$lib/api";
  import { EVENTS } from "$lib/bindings/names";
  import { followLanguage } from "$lib/i18n/index.svelte";
  import { applyTheme, DEFAULT_THEME, effectiveTheme, isTheme, isThemeId, readCachedTheme } from "$lib/themes";
  import type { UnlocksView } from "$lib/types";

  let { children }: { children: Snippet } = $props();

  // app.html marks /popup before the first paint; repeat it in case the window loads another path.
  if (browser) {
    document.documentElement.setAttribute("data-overlay", "");
    // The main window caches its theme (same origin), so the popup paints in the right colours at once.
    applyTheme(readCachedTheme() ?? DEFAULT_THEME);
  }

  /**
   * The popup follows the app theme and language: read on load and again for every popup (the window can be reused).
   * Until then it uses the ones the main window cached (`$lib/themes`, `$lib/i18n`).
   */
  async function refreshTheme(): Promise<void> {
    try {
      // A beta collection theme (ADR-024) shows only while it is unlocked: the collection comes with the rewards status
      // (read only; the popup never acks). Without it (older shell) the config is trusted.
      const [cfg, unlocks] = await Promise.all([
        api.getConfig(),
        api
          .getRewards()
          .then((status): UnlocksView | null => status.unlocks ?? null)
          .catch(() => null),
      ]);
      const shown = effectiveTheme(cfg, unlocks);
      if (isThemeId(shown)) applyTheme(shown);
      else if (isTheme(cfg.theme)) applyTheme(cfg.theme);
      followLanguage(cfg.language);
    } catch (e) {
      const err = api.toAppError(e);
      if (err.code !== api.ERR_NO_SHELL) console.warn(`[FocusMode] ${err.code}: ${err.message}`);
    }
  }

  onMount(() => {
    let disposed = false;
    let unlisten: api.UnlistenFn | null = null;
    void refreshTheme();
    api
      .on(EVENTS.popupShow, () => void refreshTheme())
      .then((u) => {
        if (disposed) u();
        else unlisten = u;
      })
      .catch((e: unknown) => {
        const err = api.toAppError(e);
        console.warn(`[FocusMode] ${err.code}: ${err.message}`);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

{@render children()}

<style>
  :global(html[data-overlay]),
  :global(html[data-overlay] body) {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: transparent;
  }
</style>
