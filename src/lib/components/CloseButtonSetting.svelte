<script lang="ts">
  // What the X on the main window does (`Config.closeToTray`): ask the first time (null), hide in the tray, or quit.
  // Used in Settings → Window and in the first-run wizard (ADR-023).
  import { config } from "$lib/state/config.svelte";
  import { strings } from "$lib/strings";

  interface Props {
    /** Id of the select (its label and hint ids derive from it). */
    id?: string;
  }

  let { id = "close-button" }: Props = $props();

  /** `closeToTray` as a select value: null = ask at the first X. */
  type CloseChoice = "ask" | "tray" | "quit";
  const CLOSE_CHOICES: readonly CloseChoice[] = ["ask", "tray", "quit"];
  const closeLabels = $derived<Record<CloseChoice, string>>({
    ask: strings.settings.closeAsk,
    tray: strings.settings.closeTray,
    quit: strings.settings.closeQuit,
  });

  const closeChoice = $derived.by((): CloseChoice => {
    const v = config.value?.closeToTray;
    return v === true ? "tray" : v === false ? "quit" : "ask";
  });

  function setCloseChoice(value: string): void {
    if (!CLOSE_CHOICES.includes(value as CloseChoice)) return;
    const choice = value as CloseChoice;
    config.update((c) => (c.closeToTray = choice === "ask" ? null : choice === "tray"), true);
  }
</script>

<div class="field">
  <label for={id}>{strings.settings.closeButton}</label>
  <select
    {id}
    class="input select narrow"
    value={closeChoice}
    aria-describedby="{id}-hint"
    onchange={(e) => setCloseChoice(e.currentTarget.value)}
  >
    {#each CLOSE_CHOICES as choice (choice)}
      <option value={choice}>{closeLabels[choice]}</option>
    {/each}
  </select>
  <p class="hint" id="{id}-hint">{strings.settings.closeHint}</p>
</div>

<style>
  .narrow {
    max-width: 340px;
  }
</style>
