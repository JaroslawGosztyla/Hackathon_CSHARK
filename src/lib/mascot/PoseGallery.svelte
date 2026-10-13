<script lang="ts">
  // Developer page (ADR-019): atlas QA for the beta poses (ADR-022). The main idle frame and the 11 poses, each in an
  // Axolotl box of the same size standing on one dashed feet line, optionally with the idle frame laid over every pose
  // (body size and place). The poses show whatever the beta switches say (`poses`); the poses atlas is requested only
  // while the gallery is mounted (the page mounts it on request, with the mascot shown). English only, like the page.
  import { POSE_GALLERY } from "$lib/i18n/dev";
  import { FRAME_GEOMETRY, mascotAssets } from "./assets.svelte";
  import Axolotl from "./Axolotl.svelte";
  import { MASCOT_SIZE } from "./FocusMascot.svelte";
  import { idleFrame, POSE_NAMES, poseFrame } from "./frames";

  /** Box heights to check: the Skupienie axolotl, then two small sizes (the summary page's empty state and card). */
  const SIZES: readonly number[] = [MASCOT_SIZE, 96, 66];
  const IDLE = idleFrame("main");

  const tiles = [
    { frame: IDLE, label: POSE_GALLERY.idle, pose: false },
    ...POSE_NAMES.map((name) => ({ frame: poseFrame(name), label: name, pose: true })),
  ];

  let size = $state(MASCOT_SIZE);
  let overlay = $state(false);
  let status = $state<"loading" | "ready" | "failed">("loading");

  $effect(() => {
    let live = true;
    void mascotAssets.load("poses").then((sheet) => {
      if (live) status = sheet ? "ready" : "failed";
    });
    return () => {
      live = false;
    };
  });

  const main = FRAME_GEOMETRY.main;
  const posesGeometry = $derived(mascotAssets.sheet("poses")?.manifest ?? FRAME_GEOMETRY.poses);
  /** Room around a box for a pose reaching past it, and below the feet line (CSS pixels at `size`). */
  const room = $derived.by(() => {
    const k = size / main.frameHeight;
    const p = posesGeometry;
    return {
      left: Math.ceil(Math.max(0, p.anchorX - main.anchorX) * k),
      right: Math.ceil(Math.max(0, p.frameWidth - p.anchorX - (main.frameWidth - main.anchorX)) * k),
      top: Math.ceil(Math.max(0, p.anchorY - main.anchorY) * k),
      bottom: Math.ceil(Math.max(main.frameHeight - main.anchorY, p.frameHeight - p.anchorY) * k),
    };
  });
</script>

<div class="pose-gallery">
  <div class="controls">
    <div class="sizes" role="group" aria-label={POSE_GALLERY.sizeLabel}>
      {#each SIZES as px (px)}
        <button
          type="button"
          class="btn btn-sm"
          class:btn-primary={size === px}
          aria-pressed={size === px}
          onclick={() => (size = px)}
        >
          {POSE_GALLERY.size(px)}
        </button>
      {/each}
    </div>
    <label class="overlay">
      <input type="checkbox" bind:checked={overlay} />
      <span>{POSE_GALLERY.overlay}</span>
    </label>
  </div>
  {#if status !== "ready"}
    <p class="hint" role="status">{status === "loading" ? POSE_GALLERY.loading : POSE_GALLERY.failed}</p>
  {/if}
  <ul class="tiles">
    {#each tiles as tile (tile.frame)}
      <li class="tile">
        <div class="stage" style:padding="{room.top}px {room.right}px {room.bottom}px {room.left}px">
          <span class="feet" style:bottom="{room.bottom}px"></span>
          <span class="spot">
            <Axolotl {size} frame={tile.frame} sit poses />
            {#if overlay && tile.pose}
              <span class="ghost"><Axolotl {size} frame={IDLE} sit /></span>
            {/if}
          </span>
        </div>
        <span class="name">{tile.label}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .pose-gallery {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 16px;
  }

  .sizes {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .overlay {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
    font-size: 0.86rem;
    color: var(--text-2);
  }

  .overlay input {
    width: 18px;
    height: 18px;
    margin: 0;
  }

  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
  }

  .stage {
    position: relative;
    display: flex;
    align-items: flex-end;
  }

  /* The shared feet line, behind the axolotl. */
  .feet {
    position: absolute;
    left: 4px;
    right: 4px;
    height: 0;
    border-top: 1px dashed var(--accent);
    pointer-events: none;
  }

  /* Its bottom edge is the feet line (Axolotl `sit`); flow-root keeps the sunk margin inside. */
  .spot {
    position: relative;
    display: flow-root;
  }

  .ghost {
    position: absolute;
    left: 0;
    bottom: 0;
    opacity: 0.45;
  }

  .name {
    padding: 0 6px 6px;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.72rem;
    color: var(--text-muted);
  }
</style>
