<script lang="ts">
  // Break screen: the user's own reminder for this break (if any), then one research-based tip picked
  // deterministically per (session, break) — the phase popup shows the same one (`breakTip` store). The axolotl
  // at the bottom-left of the timer section says them in its bubble (FocusMascot), so the card shows the cup.
  import SourceLink from "$lib/components/SourceLink.svelte";
  import { breakTip } from "$lib/state/breakTip.svelte";
  import { session } from "$lib/state/session.svelte";
  import { strings } from "$lib/strings";
  import { breakTipText, CATEGORY_ICON } from "$lib/tips";

  const b = strings.breakView;

  const snap = $derived(session.snapshot);
  const reminder = $derived(breakTip.reminder);
  const tip = $derived(breakTip.tip);
</script>

<section class="card break-card" aria-labelledby="break-title">
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
        <button type="button" class="btn btn-sm" onclick={() => breakTip.another()}>{b.another}</button>
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
</style>
