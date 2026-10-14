<script lang="ts">
  // One frame of the axolotl, cut out of the atlas with background-position / background-size from the manifest.
  // In the 8-bit theme the frame is drawn into a small canvas and scaled up pixelated instead. The box has its final
  // size before the atlas loads (no layout shift). Decorative: hidden from assistive technology.
  // A pose frame (`poses:<name>`, beta rewards, ADR-022) comes from the poses atlas and is drawn around the same main
  // box: the pose's anchor (feet line, body) on the main frame's anchor, at the main frame's scale, so the box, the
  // feet line and the page layout never move. A pose may reach past the box (lying poses are wide); it never takes
  // clicks. While the poses atlas loads, when it failed or while poses are not allowed, the pose's fallback frame from
  // the main atlas is shown instead.
  // The art is shown as it is — never redrawn, recoloured or traced.
  import { config } from "$lib/state/config.svelte";
  import { FRAME_GEOMETRY, mascotAssets, posesAllowed, type MascotSheet, type MascotVariant } from "./assets.svelte";
  import { idleFrame, POSE_FALLBACK, poseOf } from "./frames";
  import { isPixelTheme } from "./theme.svelte";

  interface Props {
    variant?: MascotVariant;
    /** Height of the box in CSS pixels; the width follows the frame's aspect ratio. */
    size: number;
    /** Frame name (a key of the variant's manifest, or `poses:<name>` for a pose of the main variant). */
    frame: string;
    /** Lowers the box so the frame's anchor line (feet / paws) sits exactly on the bottom edge of the box. */
    sit?: boolean;
    /**
     * Pose frames may load and show the poses atlas (main variant only). Default: `posesAllowed` (beta rewards or small
     * steps on, mascot shown). The Developer pose gallery passes true; false always shows the fallback frames.
     */
    poses?: boolean;
  }

  let { variant = "main", size, frame, sit = false, poses }: Props = $props();

  /** One art pixel of the 8-bit look, in CSS pixels (a few pixels at every size, so the face stays readable). */
  const PIXEL_MIN = 2;
  const PIXEL_MAX = 4;
  const PIXEL_PER_SIZE = 1 / 36;
  /** Pixels of the small canvas below this alpha become transparent, the rest opaque (crisp 8-bit edges). */
  const ALPHA_CUT = 110;

  /** Where a frame is drawn: a source rectangle of an atlas and its place relative to the box (CSS pixels). */
  interface Layer {
    sheet: MascotSheet;
    sx: number;
    sy: number;
    sw: number;
    sh: number;
    left: number;
    top: number;
    width: number;
    height: number;
  }

  const pose = $derived(variant === "main" ? poseOf(frame) : null);
  const posesOn = $derived(pose !== null && (poses ?? posesAllowed(config.value)));

  $effect(() => {
    void mascotAssets.load(variant);
  });

  // Lazy: the poses atlas is requested by the first pose frame shown while poses are allowed, never before.
  $effect(() => {
    if (posesOn) void mascotAssets.load("poses");
  });

  const sheet = $derived(mascotAssets.sheet(variant));
  const poseSheet = $derived(posesOn ? mascotAssets.sheet("poses") : null);
  const geometry = $derived(sheet?.manifest ?? FRAME_GEOMETRY[variant]);
  const height = $derived(Math.max(1, Math.round(size)));
  const width = $derived(Math.max(1, Math.round((size * geometry.frameWidth) / geometry.frameHeight)));
  const scaleX = $derived(width / geometry.frameWidth);
  const scaleY = $derived(height / geometry.frameHeight);
  const sink = $derived(sit ? Math.round((geometry.frameHeight - geometry.anchorY) * scaleY) : 0);
  const pixel = $derived(isPixelTheme());

  /** The pose, drawn from the poses atlas (null: the variant's own atlas). */
  const poseLayer = $derived.by((): Layer | null => {
    const cell = pose !== null && poseSheet ? poseSheet.manifest.frames[pose] : undefined;
    if (!poseSheet || !cell) return null;
    const pg = poseSheet.manifest;
    // The poses atlas has the main art's scale: one atlas pixel is as large as one main-frame pixel.
    const k = scaleY;
    return {
      sheet: poseSheet,
      sx: cell.x,
      sy: cell.y,
      sw: pg.frameWidth,
      sh: pg.frameHeight,
      left: Math.round(geometry.anchorX * scaleX - pg.anchorX * k),
      top: Math.round(geometry.anchorY * scaleY - pg.anchorY * k),
      width: pg.frameWidth * k,
      height: pg.frameHeight * k,
    };
  });

  /** The frame of the variant's own atlas: the frame itself, or the fallback of a pose that is not shown (yet). */
  const mainLayer = $derived.by((): Layer | null => {
    if (!sheet) return null;
    const frames = sheet.manifest.frames;
    const name = pose === null ? frame : POSE_FALLBACK[pose];
    const cell = frames[name] ?? frames[idleFrame(variant)];
    if (!cell) return null;
    const { frameWidth, frameHeight } = sheet.manifest;
    return { sheet, sx: cell.x, sy: cell.y, sw: frameWidth, sh: frameHeight, left: 0, top: 0, width, height };
  });

  const layer = $derived(poseLayer ?? mainLayer);

  const block = $derived(Math.min(PIXEL_MAX, Math.max(PIXEL_MIN, Math.round(height * PIXEL_PER_SIZE))));
  const canvasWidth = $derived(Math.max(8, Math.round(width / block)));
  const canvasHeight = $derived(Math.max(8, Math.round(height / block)));

  /**
   * The 8-bit canvas of a pose: whole art pixels of `block` CSS pixels on the box's pixel grid (so the pose's pixels
   * line up with the main frame's), large enough for the pose cell; the pose is drawn at its place inside.
   */
  const posePixels = $derived.by(() => {
    if (!poseLayer) return null;
    const left = Math.floor(poseLayer.left / block) * block;
    const top = Math.floor(poseLayer.top / block) * block;
    return {
      left,
      top,
      columns: Math.max(1, Math.ceil((poseLayer.left + poseLayer.width - left) / block)),
      rows: Math.max(1, Math.ceil((poseLayer.top + poseLayer.height - top) / block)),
      dx: (poseLayer.left - left) / block,
      dy: (poseLayer.top - top) / block,
      dw: poseLayer.width / block,
      dh: poseLayer.height / block,
    };
  });

  let canvas = $state<HTMLCanvasElement | null>(null);

  $effect(() => {
    if (!pixel || !canvas || !layer) return;
    if (posePixels && layer === poseLayer) {
      const p = posePixels;
      paintPixelated(canvas, layer, p.columns, p.rows, p.dx, p.dy, p.dw, p.dh);
    } else {
      paintPixelated(canvas, layer, canvasWidth, canvasHeight, 0, 0, canvasWidth, canvasHeight);
    }
  });

  /**
   * Draws the frame into the small canvas (the browser's area filter keeps thin outlines and the eyes), then makes
   * every pixel fully opaque or fully transparent. The canvas is scaled up with `image-rendering: pixelated`.
   */
  function paintPixelated(
    target: HTMLCanvasElement,
    art: Layer,
    w: number,
    h: number,
    dx: number,
    dy: number,
    dw: number,
    dh: number,
  ): void {
    const ctx = target.getContext("2d", { willReadFrequently: true });
    if (!ctx) return;
    ctx.clearRect(0, 0, w, h);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(art.sheet.image, art.sx, art.sy, art.sw, art.sh, dx, dy, dw, dh);
    try {
      const data = ctx.getImageData(0, 0, w, h);
      const px = data.data;
      for (let i = 3; i < px.length; i += 4) px[i] = (px[i] ?? 0) < ALPHA_CUT ? 0 : 255;
      ctx.putImageData(data, 0, 0);
    } catch {
      // Pixels not readable: the soft edges stay, the frame is still shown.
    }
  }
</script>

<span
  class="axolotl"
  aria-hidden="true"
  style:width="{width}px"
  style:height="{height}px"
  style:margin-bottom={sink > 0 ? `${-sink}px` : undefined}
>
  {#if poseLayer}
    {#if pixel && posePixels}
      <canvas
        class="pose"
        bind:this={canvas}
        width={posePixels.columns}
        height={posePixels.rows}
        style:left="{posePixels.left}px"
        style:top="{posePixels.top}px"
        style:width="{posePixels.columns * block}px"
        style:height="{posePixels.rows * block}px"
      ></canvas>
    {:else}
      {@const k = poseLayer.width / poseLayer.sw}
      <span
        class="pose"
        style:left="{poseLayer.left}px"
        style:top="{poseLayer.top}px"
        style:width="{poseLayer.width}px"
        style:height="{poseLayer.height}px"
        style:background-image="url({poseLayer.sheet.url})"
        style:background-size="{poseLayer.sheet.width * k}px {poseLayer.sheet.height * k}px"
        style:background-position="{-poseLayer.sx * k}px {-poseLayer.sy * k}px"
      ></span>
    {/if}
  {:else if mainLayer}
    {#if pixel}
      <canvas bind:this={canvas} width={canvasWidth} height={canvasHeight}></canvas>
    {:else}
      <span
        class="frame"
        style:background-image="url({mainLayer.sheet.url})"
        style:background-size="{mainLayer.sheet.width * scaleX}px {mainLayer.sheet.height * scaleY}px"
        style:background-position="{-mainLayer.sx * scaleX}px {-mainLayer.sy * scaleY}px"
      ></span>
    {/if}
  {/if}
</span>

<style>
  .axolotl {
    position: relative;
    display: block;
    flex: none;
    pointer-events: none;
    user-select: none;
  }

  .frame {
    position: absolute;
    inset: 0;
    background-repeat: no-repeat;
  }

  canvas {
    display: block;
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
  }

  /* A pose cell around the box (may reach past it; the box never changes size). */
  .pose {
    position: absolute;
    display: block;
    max-width: none;
    background-repeat: no-repeat;
  }
</style>
