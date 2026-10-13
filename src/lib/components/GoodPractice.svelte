<script lang="ts">
  // Idle Skupienie page: the good working practice of the day ("Inna porada" shows another one; the axolotl says it
  // too, `workingTip`), plus the Polish screen-work note in the Polish UI only.
  import SourceLink from "$lib/components/SourceLink.svelte";
  import { workingTip } from "$lib/state/workingTip.svelte";
  import { strings } from "$lib/strings";
  import { PL_LEGAL_NOTE, showsPolishLegalNote, workingTipText } from "$lib/tips";

  const t = strings.idleTips;

  // Checked once per mount: the page is shown again after every session, which is often enough to roll over at midnight.
  workingTip.refreshDay(new Date());
  const tip = $derived(workingTip.current);
  const showLegal = $derived(showsPolishLegalNote());
</script>

{#if tip}
  <section class="card practice" aria-labelledby="practice-title">
    <div class="block">
      <h2 id="practice-title" class="label">
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
          <path
            d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.6 10.8c.6.5 1.1 1.3 1.1 2.2h5c0-.9.5-1.7 1.1-2.2A6 6 0 0 0 12 3z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        {t.practiceTitle}
      </h2>
      <p class="text" aria-live="polite">{workingTipText(tip)}</p>
      <div class="row actions">
        {#if workingTip.canChange}
          <button type="button" class="btn btn-sm" onclick={() => workingTip.another()}>{t.anotherTip}</button>
        {/if}
        <SourceLink url={tip.source} label={t.source} aria={t.sourceAria} />
      </div>
    </div>

    {#if showLegal}
      <div class="block legal">
        <h3 class="label">{t.lawTitle}</h3>
        <p class="text small-text"><span lang="pl">{PL_LEGAL_NOTE.text}</span></p>
        <SourceLink url={PL_LEGAL_NOTE.source} label={t.source} aria={t.sourceAria} />
      </div>
    {/if}
  </section>
{/if}

<style>
  .practice {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
    min-width: 0;
  }

  .legal {
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--accent-strong);
  }

  .legal .label {
    color: var(--text-muted);
  }

  .text {
    color: var(--text-2);
  }

  .small-text {
    font-size: 0.9rem;
  }

  .actions {
    gap: 14px;
  }
</style>
