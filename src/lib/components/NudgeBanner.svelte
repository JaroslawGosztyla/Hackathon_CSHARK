<script lang="ts">
  import { untrack } from "svelte";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { useMascot } from "$lib/mascot/controller.svelte";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import { config } from "$lib/state/config.svelte";
  import { nudge } from "$lib/state/nudge.svelte";
  import { strings } from "$lib/strings";

  /** A plain domain from websiteblacklist.txt reads well in the banner; a regex source does not. */
  const DOMAIN = /^[a-z0-9-]+(\.[a-z0-9-]+)+$/i;
  /** Height of the small axolotl in the banner, in CSS pixels. */
  const MASCOT_SIZE = 46;

  const body = $derived.by(() => {
    const matched = nudge.current?.matched ?? "";
    return DOMAIN.test(matched) ? strings.nudge.body(matched) : strings.nudge.bodyPattern;
  });

  const mascotOn = $derived(mascotEnabled(config.value));
  const mascot = useMascot("main");

  // Every nudge (a new object, also while the banner is already shown) gets a frown.
  $effect(() => {
    if (nudge.current && mascotOn) untrack(() => mascot.react("nudge"));
  });
</script>

{#if nudge.current}
  <div class="nudge" role="alert">
    {#if mascotOn}
      <span class="mascot" aria-hidden="true">
        <Axolotl size={MASCOT_SIZE} frame={mascot.frame} />
      </span>
    {:else}
      <span class="icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="22" height="22">
          <path
            d="M4 14v-2a8 8 0 0 1 16 0v2"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
          />
          <rect x="3" y="13" width="4.5" height="7" rx="1.8" fill="currentColor" />
          <rect x="16.5" y="13" width="4.5" height="7" rx="1.8" fill="currentColor" />
        </svg>
      </span>
    {/if}
    <p>
      <strong>{strings.nudge.title}</strong> — {body}
    </p>
    <button class="btn btn-sm btn-ghost" type="button" onclick={() => nudge.dismiss()}>{strings.nudge.dismiss}</button>
  </div>
{/if}

<style>
  .nudge {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 auto 16px;
    padding: 12px 14px 12px 16px;
    border-radius: var(--radius);
    background: linear-gradient(90deg, color-mix(in srgb, var(--accent) 22%, transparent), color-mix(in srgb, var(--accent) 8%, transparent));
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    animation: pop 0.25s ease-out;
  }

  .icon {
    color: var(--accent-strong);
    display: grid;
  }

  /* The banner keeps its height: the axolotl overlaps the padding a little instead of growing the row. */
  .mascot {
    display: grid;
    margin: -8px 0 -8px -6px;
  }

  p {
    flex: 1;
    color: var(--text);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
