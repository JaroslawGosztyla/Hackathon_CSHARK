// Mascot art: the frame atlases and their manifests in static/mascot (art/mascot/README.md). Each atlas is fetched
// once per window. The art is only displayed as it is: never redrawn, recoloured or traced.
import type { Config, RewardsConfig } from "$lib/types";

/** The axolotl of a host: the main window's (`main`) or the popup's peeking one (`peek`), each with its own frames. */
export type MascotVariant = "main" | "peek";

/**
 * An atlas: a variant's own frames, or the owner's 11 poses (`poses`, beta rewards, ADR-022). The poses have no host
 * of their own: a frame `poses:<name>` is shown inside the `main` box (Axolotl.svelte), so this is not a variant.
 * Built by tools/mascot/poses-atlas.mjs at the main atlas's scale, with a shared feet line.
 */
export type MascotAtlas = MascotVariant | "poses";

/** Top-left corner of a frame in the atlas, in source pixels. */
export interface MascotCell {
  x: number;
  y: number;
}

/** Frame size and the anchor line (feet / paws) shared by every frame of a variant. */
export interface MascotGeometry {
  frameWidth: number;
  frameHeight: number;
  anchorX: number;
  anchorY: number;
}

export interface MascotManifest extends MascotGeometry {
  frames: Record<string, MascotCell>;
}

export interface MascotSheet {
  manifest: MascotManifest;
  /** Atlas image URL (same origin). */
  url: string;
  /** Size of the whole atlas in source pixels. */
  width: number;
  height: number;
  /** The decoded atlas, for the pixelated canvas of the 8-bit theme. */
  image: HTMLImageElement;
}

const FILES: Record<MascotAtlas, { manifest: string; atlas: string }> = {
  main: { manifest: "/mascot/axo-manifest.json", atlas: "/mascot/axo-atlas.webp" },
  peek: { manifest: "/mascot/peek-manifest.json", atlas: "/mascot/peek-atlas.webp" },
  poses: { manifest: "/mascot/poses-manifest.json", atlas: "/mascot/poses-atlas.webp" },
};

/**
 * Geometry known before the manifest arrives (mirrors static/mascot/*-manifest.json), so a mascot box has its final
 * size from the first paint and never shifts the layout when the atlas loads. A pose cell is larger than a main frame
 * (lying poses are wide) and is drawn around the main box, anchor on anchor, without changing the box.
 */
export const FRAME_GEOMETRY: Record<MascotAtlas, MascotGeometry> = {
  main: { frameWidth: 415, frameHeight: 420, anchorX: 211, anchorY: 405 },
  peek: { frameWidth: 495, frameHeight: 300, anchorX: 258, anchorY: 288 },
  poses: { frameWidth: 558, frameHeight: 466, anchorX: 296, anchorY: 460 },
};

/**
 * Pose frames may load the poses atlas: the beta features are on with the rewards or the small steps (the only parts
 * that show poses, ADR-022) and the mascot is shown. Otherwise the atlas is never requested.
 */
export function posesAllowed(cfg: Config | null): boolean {
  if (cfg === null || cfg.betaFeatures !== true || (cfg.showMascot as boolean | undefined) === false) return false;
  const rewards = cfg.rewards as RewardsConfig | undefined;
  return rewards !== undefined && (rewards.enabled || rewards.microSteps);
}

function isSize(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

/** Accepts only the shape this module uses; frames with a broken position are left out. */
function parseManifest(raw: unknown): MascotManifest | null {
  if (typeof raw !== "object" || raw === null) return null;
  const m = raw as Record<string, unknown>;
  const { frameWidth, frameHeight, anchorX, anchorY, frames } = m;
  if (!isSize(frameWidth) || !isSize(frameHeight) || frameWidth === 0 || frameHeight === 0) return null;
  if (!isSize(anchorX) || !isSize(anchorY)) return null;
  if (typeof frames !== "object" || frames === null) return null;
  const cells: Record<string, MascotCell> = {};
  for (const [name, cell] of Object.entries(frames as Record<string, unknown>)) {
    if (typeof cell !== "object" || cell === null) continue;
    const { x, y } = cell as Record<string, unknown>;
    if (isSize(x) && isSize(y)) cells[name] = { x, y };
  }
  if (Object.keys(cells).length === 0) return null;
  return { frameWidth, frameHeight, anchorX, anchorY, frames: cells };
}

async function fetchSheet(variant: MascotAtlas): Promise<MascotSheet | null> {
  const files = FILES[variant];
  try {
    const response = await fetch(files.manifest);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const manifest = parseManifest(await response.json());
    if (!manifest) throw new Error("invalid manifest");
    const image = new Image();
    image.decoding = "async";
    image.src = files.atlas;
    await image.decode();
    return { manifest, url: files.atlas, width: image.naturalWidth, height: image.naturalHeight, image };
  } catch (e) {
    // Decorative: without the art the mascot box simply stays empty.
    console.warn(`[FocusMode] mascot (${variant}): ${e instanceof Error ? e.message : String(e)}`);
    return null;
  }
}

class MascotAssets {
  #sheets = $state.raw<Partial<Record<MascotAtlas, MascotSheet>>>({});
  #pending = new Map<MascotAtlas, Promise<MascotSheet | null>>();

  /** The loaded sheet (reactive), or null while loading or when it failed. */
  sheet(variant: MascotAtlas): MascotSheet | null {
    return this.#sheets[variant] ?? null;
  }

  /**
   * Fetches the manifest and decodes the atlas once per window; later calls share the first result. Nothing is
   * fetched before the first call (the poses atlas: the first pose frame shown while `posesAllowed`).
   */
  load(variant: MascotAtlas): Promise<MascotSheet | null> {
    let pending = this.#pending.get(variant);
    if (!pending) {
      pending = fetchSheet(variant).then((sheet) => {
        if (sheet) this.#sheets = { ...this.#sheets, [variant]: sheet };
        return sheet;
      });
      this.#pending.set(variant, pending);
    }
    return pending;
  }
}

export const mascotAssets = new MascotAssets();
