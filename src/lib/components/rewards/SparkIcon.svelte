<script lang="ts">
  // The spark glyph (✦) of the beta rewards: a UI icon, not mascot art. 8-bit theme: an 8×8 pixel-art variant.
  import { isPixelTheme } from "$lib/mascot/theme.svelte";

  interface Props {
    /** Width and height in CSS pixels. */
    size?: number;
  }

  let { size = 14 }: Props = $props();

  const pixel = $derived(isPixelTheme());
</script>

{#if pixel}
  <svg class="spark" viewBox="0 0 8 8" width={size} height={size} aria-hidden="true" shape-rendering="crispEdges">
    <path d="M3 0h2v2H3zM0 3h2v2H0zM6 3h2v2H6zM3 6h2v2H3zM2 2h4v4H2z" />
  </svg>
{:else}
  <svg class="spark" viewBox="0 0 16 16" width={size} height={size} aria-hidden="true">
    <path d="M8 .8C8.6 5.2 10.8 7.4 15.2 8 10.8 8.6 8.6 10.8 8 15.2 7.4 10.8 5.2 8.6.8 8 5.2 7.4 7.4 5.2 8 .8z" />
  </svg>
{/if}

<style>
  .spark {
    flex: none;
    fill: var(--reward);
    vertical-align: -0.15em;
  }
</style>
