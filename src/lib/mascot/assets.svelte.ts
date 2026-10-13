// Mascot art: the frame atlases and their manifests in static/mascot (art/mascot/README.md). Each variant is fetched
// once per window. The art is only displayed as it is: never redrawn, recoloured or traced.

export type MascotVariant = "main" | "peek";

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

const FILES: Record<MascotVariant, { manifest: string; atlas: string }> = {
  main: { manifest: "/mascot/axo-manifest.json", atlas: "/mascot/axo-atlas.webp" },
  peek: { manifest: "/mascot/peek-manifest.json", atlas: "/mascot/peek-atlas.webp" },
};

/**
 * Geometry known before the manifest arrives (mirrors static/mascot/*-manifest.json), so a mascot box has its final
 * size from the first paint and never shifts the layout when the atlas loads.
 */
export const FRAME_GEOMETRY: Record<MascotVariant, MascotGeometry> = {
  main: { frameWidth: 415, frameHeight: 420, anchorX: 211, anchorY: 405 },
  peek: { frameWidth: 495, frameHeight: 300, anchorX: 258, anchorY: 288 },
};

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

async function fetchSheet(variant: MascotVariant): Promise<MascotSheet | null> {
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
  #sheets = $state.raw<Partial<Record<MascotVariant, MascotSheet>>>({});
  #pending = new Map<MascotVariant, Promise<MascotSheet | null>>();

  /** The loaded sheet (reactive), or null while loading or when it failed. */
  sheet(variant: MascotVariant): MascotSheet | null {
    return this.#sheets[variant] ?? null;
  }

  /** Fetches the manifest and decodes the atlas once per window; later calls share the first result. */
  load(variant: MascotVariant): Promise<MascotSheet | null> {
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
