<script lang="ts">
  import { blocklist } from "$lib/state/blocklist.svelte";
  import { config } from "$lib/state/config.svelte";
  import { shields } from "$lib/state/shields.svelte";
  import { launcherList, strings } from "$lib/strings";
  import type { IntegrationStatus } from "$lib/types";

  interface Chip {
    key: string;
    name: string;
    status: IntegrationStatus;
    /** Tooltip; defaults to the status detail. */
    title?: string;
  }

  /** "N domen · M wyrażeń regularnych" from websiteblacklist.txt, shown on hover over the sites chip. */
  const listSummary = $derived(
    blocklist.info ? strings.blocklist.summary(blocklist.info.domains.length, blocklist.info.patterns.length) : null,
  );

  const chips = $derived.by((): Chip[] => {
    const s = shields.status;
    if (!s) return [];
    const all: Chip[] = [
      { key: "notifications", name: strings.shieldChip.notifications, status: s.notifications },
      { key: "teams", name: strings.shieldChip.teams, status: s.teams },
      {
        key: "sites",
        name: s.extensionConnected ? strings.shieldChip.sites : strings.shieldChip.sitesFallback,
        status: s.sites,
        title: [listSummary, s.sites.detail].filter(Boolean).join(" — ") || undefined,
      },
      { key: "keepAwake", name: strings.shieldChip.keepAwake, status: s.keepAwake },
    ];
    const visible = all.filter((c) => c.status.state !== "off");
    // Launchers are closed once, when focus starts: with the option on, the chip stays for the whole focus.
    if (config.value?.shields.closeLaunchers || s.launchers.state !== "off") {
      const status: IntegrationStatus = s.launchers.state === "off" ? { ...s.launchers, state: "on" } : s.launchers;
      const closed = s.closedLaunchers.length > 0 ? strings.shieldChip.launchersClosed(launcherList(s.closedLaunchers)) : null;
      const result = closed ?? (status.state === "on" ? strings.shieldChip.launchersNothing : null);
      visible.push({
        key: "launchers",
        name: strings.shieldChip.launchers,
        status,
        title: [result, s.launchers.detail].filter(Boolean).join(" — ") || undefined,
      });
    }
    return visible;
  });
</script>

<section class="card" aria-labelledby="shields-title">
  <div class="card-title">
    <h2 id="shields-title">{strings.focusView.shields}</h2>
  </div>
  {#if chips.length === 0}
    <p class="hint">{strings.focusView.shieldsNone}</p>
  {:else}
    <ul class="chips">
      {#each chips as chip (chip.key)}
        <li
          class="chip state-{chip.status.state}"
          title={chip.title ?? chip.status.detail ?? undefined}
          aria-label={strings.shieldChip.chipAria(chip.name, strings.shieldChip.state[chip.status.state])}
        >
          <span class="indicator" aria-hidden="true">
            {#if chip.status.state === "on"}
              <svg viewBox="0 0 16 16" width="14" height="14">
                <path
                  d="M3.5 8.5l3 3 6-7"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            {:else if chip.status.state === "error"}
              <svg viewBox="0 0 16 16" width="14" height="14">
                <path d="M8 3.5v5.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
                <circle cx="8" cy="12.2" r="1.2" fill="currentColor" />
              </svg>
            {:else}
              <svg viewBox="0 0 16 16" width="14" height="14">
                <path d="M4 8h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
              </svg>
            {/if}
          </span>
          <span>{chip.name}</span>
          {#if chip.status.state !== "on"}
            <span class="state">· {strings.shieldChip.state[chip.status.state]}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .indicator {
    display: grid;
  }

  .state-on {
    border-color: color-mix(in srgb, var(--break) 45%, transparent);
    color: var(--text);
  }

  .state-on .indicator {
    color: var(--ok);
  }

  .state-error {
    border-color: color-mix(in srgb, var(--danger) 50%, transparent);
    background: var(--danger-soft);
  }

  .state-error .indicator {
    color: var(--danger);
  }

  .state-unavailable {
    opacity: 0.75;
  }

  .state {
    color: var(--text-muted);
    font-weight: 500;
  }
</style>
