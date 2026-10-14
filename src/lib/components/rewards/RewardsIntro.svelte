<script lang="ts">
  // The one-time intro card on Skupienie, the first time the rewards are active in this WebView (not a modal). Its
  // axolotl looks up, then holds its hands to its heart. "Zaczynamy" and "Ustawienia" both dismiss it; the Developer
  // page can show it again. A future first-run wizard can reuse it.
  import { onMount } from "svelte";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { prefersReducedMotion } from "$lib/mascot/controller.svelte";
  import { mascotEnabled } from "$lib/mascot/cues.svelte";
  import { poseFrame, REWARD_STEPS } from "$lib/mascot/frames";
  import { config } from "$lib/state/config.svelte";
  import { rewards } from "$lib/state/rewards.svelte";
  import { strings } from "$lib/strings";
  import BetaBadge from "./BetaBadge.svelte";

  const MASCOT_SIZE = 84;
  const uid = $props.id();
  const s = strings.rewards;

  const [lookUp, heart] = REWARD_STEPS.intro;
  const mascotOn = $derived(mascotEnabled(config.value) && rewards.celebration !== "numbersOnly");

  // Still with reduced motion: the final pose at once.
  let frame = $state(prefersReducedMotion() ? (heart?.frame ?? poseFrame("02-clasped-hands-heart")) : (lookUp?.frame ?? ""));

  onMount(() => {
    if (!heart || frame === heart.frame) return;
    const timer = setTimeout(() => (frame = heart.frame), lookUp?.ms ?? 0);
    return () => clearTimeout(timer);
  });
</script>

<section class="card intro" aria-labelledby="{uid}-title">
  {#if mascotOn}
    <span class="mascot"><Axolotl size={MASCOT_SIZE} {frame} /></span>
  {/if}
  <div class="content">
    <div class="head">
      <h2 id="{uid}-title">{s.introTitle}</h2>
      <BetaBadge />
    </div>
    <p class="secondary">{s.introBody}</p>
    <div class="row actions">
      <button type="button" class="btn btn-primary btn-sm" onclick={() => rewards.dismissIntro()}>{s.introGo}</button>
      <a class="btn btn-sm" href="/settings#beta-settings" onclick={() => rewards.dismissIntro()}>{s.introSettings}</a>
    </div>
  </div>
</section>

<style>
  .intro {
    display: flex;
    align-items: center;
    gap: 16px;
    border-color: var(--reward-line);
  }

  .mascot {
    flex: none;
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  h2 {
    font-size: 1.05rem;
  }

  .actions {
    gap: 8px;
  }

  @media (max-width: 520px) {
    .intro {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
