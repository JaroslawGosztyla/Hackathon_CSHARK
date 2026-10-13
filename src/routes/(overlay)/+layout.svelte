<script lang="ts">
  // Transparent overlay windows (phase popup, screen tint). No app shell here: no navigation, no toasts,
  // no subscriptions of the main window and, above all, no music player (music must play in one window only).
  import type { Snippet } from "svelte";
  import { browser } from "$app/environment";

  let { children }: { children: Snippet } = $props();

  // app.html marks /popup and /tint before the first paint; repeat it in case the window loads another path.
  if (browser) document.documentElement.setAttribute("data-overlay", "");
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
