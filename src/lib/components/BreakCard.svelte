<script lang="ts" module>
  import { SvelteMap } from "svelte/reactivity";

  /** "Inna propozycja" steps per break (session + cycle), so leaving the page and coming back keeps the tip. */
  const offsets = new SvelteMap<string, number>();
</script>

<script lang="ts">
  // Break screen: the user's own reminder for this break (if any), then one research-based tip picked
  // deterministically per (session, break) — the phase popup shows the same one.
  import { untrack } from "svelte";
  import SourceLink from "$lib/components/SourceLink.svelte";
  import Axolotl from "$lib/mascot/Axolotl.svelte";
  import { useMascot } from "$lib/mascot/controller.svelte";
  import { followCues, mascotEnabled } from "$lib/mascot/cues.svelte";
  import { config, reminderForBreak } from "$lib/state/config.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";
  import { breakTipText, CATEGORY_ICON, pickBreakTip } from "$lib/tips";

  const b = strings.breakView;
  /** Height of the axolotl in the card, in CSS pixels. */
  const MASCOT_SIZE = 104;

  const snap = $derived(session.snapshot);
  const breakKey = $derived(`${snap.sessionId ?? ""}:${snap.cycle}`);
  const offset = $derived(offsets.get(breakKey) ?? 0);

  const reminder = $derived.by(() => {
    const cfg = config.value;
    return cfg ? reminderForBreak(cfg.breakReminders, cfg.repeatReminders, snap.cycle) : null;
  });
  const tip = $derived(pickBreakTip(snap.sessionId, snap.cycle, snap.phaseTotalSecs, offset));

  // The axolotl sits in the card during a break (FocusMascot leaves the timer): it smiles when the break starts and
  // says the reminder and the tip (lip-sync), again after "Inna propozycja".
  const mascotOn = $derived(mascotEnabled(config.value));
  const mascot = useMascot("main");
  followCues(mascot, ["break"]);
  const speech = $derived([reminder, tip ? breakTipText(tip) : null].filter((part): part is string => part !== null).join(" "));

  $effect(() => {
    const text = mascotOn ? speech : "";
    untrack(() => (text ? mascot.talk(text) : mascot.hush()));
  });

  function another(): void {
    // Only the current break's entry matters; older ones are dropped so the map stays tiny.
    for (const key of offsets.keys()) if (key !== breakKey) offsets.delete(key);
    offsets.set(breakKey, offset + 1);
  }
</script>

<section class="card break-card" aria-labelledby="break-title">
  {#if mascotOn}
    <div class="mascot" aria-hidden="true">
      <Axolotl size={MASCOT_SIZE} frame={mascot.frame} />
    </div>
  {:else}
    <div class="illustration" aria-hidden="true">
      <svg viewBox="0 0 64 64" width="64" height="64">
        <path d="M14 26h30v14a12 12 0 0 1-12 12h-6a12 12 0 0 1-12-12z" style="fill: var(--break)" opacity="0.9" />
        <path d="M44 30h4a6 6 0 0 1 0 12h-4" fill="none" style="stroke: var(--break)" stroke-width="3" />
        <path
          class="steam"
          d="M24 20c-2-3 2-5 0-8M32 20c-2-3 2-5 0-8"
          fill="none"
          style="stroke: var(--text-2)"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>
    </div>
  {/if}
  <div class="content">
    <h2 id="break-title">{b.title}</h2>
    <p class="secondary">{b.lead}</p>

    {#if reminder}
      <div class="reminder">
        <p class="label">{b.reminderLabel(snap.cycle)}</p>
        <p class="reminder-text">{reminder}</p>
      </div>
    {/if}

    {#if tip}
      <div class="suggestion" aria-live="polite" aria-atomic="true">
        <p class="label">
          <span class="icon" aria-hidden="true">{CATEGORY_ICON[tip.category]}</span>
          <span>{b.categories[tip.category]}</span>
          <span class="sep" aria-hidden="true">·</span>
          <span class="muted-label">{b.suggestionLabel}</span>
        </p>
        <p class="text">{breakTipText(tip)}</p>
        {#if tip.category === "stretch"}
          <p class="footnote">{strings.stretchFootnote}</p>
        {/if}
      </div>
      <div class="row actions">
        <button type="button" class="btn btn-sm" onclick={another}>{b.another}</button>
        {#if tip.source}
          <SourceLink url={tip.source} label={b.source} aria={b.sourceAria} />
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .break-card {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    border-color: color-mix(in srgb, var(--break) 30%, transparent);
    background: linear-gradient(135deg, color-mix(in srgb, var(--break) 10%, transparent), var(--surface) 60%);
  }

  .illustration {
    flex: none;
    display: grid;
    place-items: center;
    width: 84px;
    height: 84px;
    border-radius: 24px;
    background: var(--break-soft);
  }

  .mascot {
    flex: none;
    display: grid;
    place-items: end center;
    width: 112px;
    padding-top: 4px;
  }

  .steam {
    animation: steam 3s ease-in-out infinite;
  }

  @keyframes steam {
    0%,
    100% {
      opacity: 0.3;
      transform: translateY(1px);
    }
    50% {
      opacity: 0.9;
      transform: translateY(-2px);
    }
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: stretch;
    flex: 1;
    min-width: 0;
  }

  .reminder {
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    background: var(--break-soft);
    border: 1px solid color-mix(in srgb, var(--break) 40%, transparent);
  }

  .reminder-text {
    font-size: 1.25rem;
    font-weight: 650;
    line-height: 1.35;
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .suggestion {
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
    border-left: 3px solid var(--break);
  }

  .label {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--break);
    margin-bottom: 4px;
  }

  .icon {
    font-size: 1rem;
    letter-spacing: 0;
  }

  .sep,
  .muted-label {
    color: var(--text-muted);
  }

  .text {
    font-size: 1.05rem;
  }

  .footnote {
    margin-top: 8px;
    color: var(--text-muted);
    font-size: 0.82rem;
  }

  .actions {
    gap: 14px;
  }

  @media (max-width: 560px) {
    .break-card {
      flex-direction: column;
    }
  }
</style>
