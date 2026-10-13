<script lang="ts">
  // One frame of the axolotl, cut out of the atlas with background-position / background-size from the manifest.
  // In the 8-bit theme the frame is drawn into a small canvas and scaled up pixelated instead. The box has its final
  // size before the atlas loads (no layout shift). Decorative: hidden from assistive technology.
  // The art is shown as it is — never redrawn, recoloured or traced.
  import { FRAME_GEOMETRY, mascotAssets, type MascotCell, type MascotSheet, type MascotVariant } from "./assets.svelte";
  import { idleFrame } from "./frames";
  import { isPixelTheme } from "./theme.svelte";

  interface Props {
    variant?: MascotVariant;
    /** Height of the box in CSS pixels; the width follows the frame's aspect ratio. */
    size: number;
    /** Frame name (a key of the variant's manifest). */
    frame: string;
    /** Lowers the box so the frame's anchor line (feet / paws) sits exactly on the bottom edge of the box. */
    sit?: boolean;
  }

  let { variant = "main", size, frame, sit = false }: Props = $props();

  /** One art pixel of the 8-bit look, in CSS pixels (a few pixels at every size, so the face stays readable). */
  const PIXEL_MIN = 2;
  const PIXEL_MAX = 4;
  const PIXEL_PER_SIZE = 1 / 36;
  /** Pixels of the small canvas below this alpha become transparent, the rest opaque (crisp 8-bit edges). */
  const ALPHA_CUT = 110;

  $effect(() => {
    void mascotAssets.load(variant);
  });

  const sheet = $derived(mascotAssets.sheet(variant));
  const geometry = $derived(sheet?.manifest ?? FRAME_GEOMETRY[variant]);
  const height = $derived(Math.max(1, Math.round(size)));
  const width = $derived(Math.max(1, Math.round((size * geometry.frameWidth) / geometry.frameHeight)));
  const scaleX = $derived(width / geometry.frameWidth);
  const scaleY = $derived(height / geometry.frameHeight);
  const cell = $derived.by((): MascotCell | null => {
    const frames = sheet?.manifest.frames;
    return frames ? (frames[frame] ?? frames[idleFrame(variant)] ?? null) : null;
  });
  const sink = $derived(sit ? Math.round((geometry.frameHeight - geometry.anchorY) * scaleY) : 0);
  const pixel = $derived(isPixelTheme());

  const block = $derived(Math.min(PIXEL_MAX, Math.max(PIXEL_MIN, Math.round(height * PIXEL_PER_SIZE))));
  const canvasWidth = $derived(Math.max(8, Math.round(width / block)));
  const canvasHeight = $derived(Math.max(8, Math.round(height / block)));

  let canvas = $state<HTMLCanvasElement | null>(null);

  $effect(() => {
    if (!pixel || !canvas || !sheet || !cell) return;
    paintPixelated(canvas, sheet, cell, canvasWidth, canvasHeight);
  });

  /**
   * Draws the frame into the small canvas (the browser's area filter keeps thin outlines and the eyes), then makes
   * every pixel fully opaque or fully transparent. The canvas is scaled up with `image-rendering: pixelated`.
   */
  function paintPixelated(target: HTMLCanvasElement, art: MascotSheet, at: MascotCell, w: number, h: number): void {
    const ctx = target.getContext("2d", { willReadFrequently: true });
    if (!ctx) return;
    const { frameWidth, frameHeight } = art.manifest;
    ctx.clearRect(0, 0, w, h);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(art.image, at.x, at.y, frameWidth, frameHeight, 0, 0, w, h);
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
  {#if sheet && cell}
    {#if pixel}
      <canvas bind:this={canvas} width={canvasWidth} height={canvasHeight}></canvas>
    {:else}
      <span
        class="frame"
        style:background-image="url({sheet.url})"
        style:background-size="{sheet.width * scaleX}px {sheet.height * scaleY}px"
        style:background-position="{-cell.x * scaleX}px {-cell.y * scaleY}px"
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
</style>
