<script lang="ts">
  // Transparent overlay windows (the phase popup). No app shell here: no navigation, no toasts,
  // no subscriptions of the main window and, above all, no music player (music must play in one window only).
  import { onMount, type Snippet } from "svelte";
  import { browser } from "$app/environment";
  import * as api from "$lib/api";
  import { EVENTS } from "$lib/bindings/names";
  import { applyTheme, DEFAULT_THEME, isTheme, readCachedTheme } from "$lib/themes";

  let { children }: { children: Snippet } = $props();

  // app.html marks /popup before the first paint; repeat it in case the window loads another path.
  if (browser) {
    document.documentElement.setAttribute("data-overlay", "");
    // The main window caches its theme (same origin), so the popup paints in the right colours at once.
    applyTheme(readCachedTheme() ?? DEFAULT_THEME);
  }

  /** The popup follows the app theme: read on load and again for every popup (the window can be reused). */
  async function refreshTheme(): Promise<void> {
    try {
      const cfg = await api.getConfig();
      if (isTheme(cfg.theme)) applyTheme(cfg.theme);
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
