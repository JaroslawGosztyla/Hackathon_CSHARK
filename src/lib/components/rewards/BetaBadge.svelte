<script lang="ts">
  // The "beta" pill of every reward surface (ADR-022): once per surface, with what it means as a tooltip.
  import { strings } from "$lib/strings";
</script>

<span class="beta-badge" title={strings.rewards.betaTitle}>
  {strings.rewards.beta}<span class="sr-only">: {strings.rewards.betaTitle}</span>
</span>

<style>
  .beta-badge {
    display: inline-flex;
    align-items: center;
    flex: none;
    padding: 0 8px;
    border-radius: var(--radius-pill);
    border: var(--border-w) solid var(--reward-line);
    color: var(--reward-ink);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    line-height: 1.6;
    cursor: help;
  }
</style>
