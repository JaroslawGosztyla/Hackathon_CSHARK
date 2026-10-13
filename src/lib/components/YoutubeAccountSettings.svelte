<script lang="ts">
  // Settings → Dźwięk → "Konto YouTube" (ADR-021). The user signs in on Google's own page in a window of the app;
  // FocusMode never sees the password and only learns whether sign-in cookies exist. Signed in, every YouTube item
  // plays in the YouTube window, where Premium applies.
  import { youtubeAccount } from "$lib/state/youtubeAccount.svelte";
  import { strings } from "$lib/strings";

  const s = $derived(strings.youtube);
  const status = $derived(youtubeAccount.status);
  const signInOpen = $derived(status?.signInOpen === true);
  /** Google's refusal stays explained until the next sign-in attempt (or a successful one). */
  const rejected = $derived(status?.browserRejected === true && !youtubeAccount.signedIn);
</script>

<div class="youtube stack" role="group" aria-labelledby="youtube-account-title">
  <p class="field-label" id="youtube-account-title">{s.title}</p>

  <div class="status-row" aria-live="polite">
    {#if youtubeAccount.signedIn}
      <span class="state ok">
        <span class="dot" aria-hidden="true"></span>
        {s.signedIn}
      </span>
      <button type="button" class="btn btn-sm" disabled={youtubeAccount.busy} onclick={() => youtubeAccount.signOut()}>
        {s.signOut}
      </button>
    {:else if signInOpen}
      <span class="state pending">
        <span class="dot" aria-hidden="true"></span>
        {s.signInOpen}
      </span>
      <button type="button" class="btn btn-sm" disabled={youtubeAccount.busy} onclick={() => youtubeAccount.signIn()}>
        {s.showSignIn}
      </button>
    {:else}
      <!-- Until the shell has read the cookies (normally a moment after start) signing in is already possible. -->
      <span class="state">
        <span class="dot" aria-hidden="true"></span>
        {youtubeAccount.known ? s.notSignedIn : s.checking}
      </span>
      <button
        type="button"
        class="btn btn-sm btn-primary"
        disabled={youtubeAccount.busy}
        onclick={() => youtubeAccount.signIn()}
      >
        {s.signIn}
      </button>
    {/if}
  </div>

  <p class="secondary">{youtubeAccount.signedIn ? s.signedInHint : s.notSignedInHint}</p>
  {#if rejected}
    <p class="field-error" role="status">{s.rejected}</p>
  {/if}
  <p class="hint">{s.privacy}</p>
</div>

<style>
  .youtube {
    gap: 8px;
  }

  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--text-2);
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted-mark);
  }

  .state.ok .dot {
    background: var(--ok);
  }

  .state.pending .dot {
    background: var(--accent);
  }

  .secondary {
    font-size: 0.9rem;
  }
</style>
