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
  /** The cookies could not be read: treated as signed out, said in one line, with "Check again". */
  const unknown = $derived(!youtubeAccount.known && youtubeAccount.checkFailed);
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
      <!-- After Google's refusal the click starts over on Google's sign-in page in that window (youtube_sign_in). -->
      <button type="button" class="btn btn-sm" disabled={youtubeAccount.busy} onclick={() => youtubeAccount.signIn()}>
        {rejected ? s.restartSignIn : s.showSignIn}
      </button>
    {:else}
      <!-- Until the shell has read the cookies (normally a moment after start) signing in is already possible. -->
      <span class="state">
        <span class="dot" aria-hidden="true"></span>
        {youtubeAccount.known ? s.notSignedIn : unknown ? s.statusUnknown : s.checking}
      </span>
      <span class="actions">
        {#if unknown}
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            disabled={youtubeAccount.busy}
            onclick={() => youtubeAccount.recheck()}
          >
            {s.checkAgain}
          </button>
        {/if}
        <button
          type="button"
          class="btn btn-sm btn-primary"
          disabled={youtubeAccount.busy}
          onclick={() => youtubeAccount.signIn()}
        >
          {s.signIn}
        </button>
      </span>
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

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* Round in every theme but 8-bit, where --radius-pill is 0 and the dot is a square pixel. */
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: var(--radius-pill);
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
