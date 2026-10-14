<script lang="ts">
  // One canvas clipped to the recap card: at most 60 particles for 1.2 s, drawn with requestAnimationFrame only while
  // they fly and only while the window is visible. 8-bit theme: 4×4 squares at 10 fps. Colours: --confetti-1..4.
  // Never takes clicks. Not drawn for Calm, Numbers only or reduced motion (the caller passes `enabled`).
  import { CONFETTI_MAX, CONFETTI_MS, PIXEL_FPS } from "$lib/rewards/choreography";

  interface Props {
    /** Bumps to start a burst of confetti (0: none yet). */
    trigger: number;
    enabled: boolean;
    pixel?: boolean;
  }

  let { trigger, enabled, pixel = false }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);

  interface Particle {
    x: number;
    y: number;
    vx: number;
    vy: number;
    spin: number;
    angle: number;
    size: number;
    color: string;
  }

  const GRAVITY = 900;
  const PIXEL_SIZE = 4;

  function colors(el: HTMLElement): string[] {
    const style = getComputedStyle(el);
    const out = [1, 2, 3, 4].map((n) => style.getPropertyValue(`--confetti-${n}`).trim()).filter((c) => c !== "");
    return out.length > 0 ? out : ["#ffd27a"];
  }

  $effect(() => {
    const el = canvas;
    if (trigger <= 0 || !enabled || !el) return;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    const ratio = window.devicePixelRatio || 1;
    const width = el.clientWidth;
    const height = el.clientHeight;
    el.width = Math.max(1, Math.round(width * ratio));
    el.height = Math.max(1, Math.round(height * ratio));
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    const palette = colors(el);
    const particles: Particle[] = Array.from({ length: CONFETTI_MAX }, (_, i) => ({
      x: width * (0.3 + Math.random() * 0.4),
      y: height * 0.35,
      vx: (Math.random() - 0.5) * 520,
      vy: -220 - Math.random() * 360,
      spin: (Math.random() - 0.5) * 12,
      angle: Math.random() * Math.PI,
      size: pixel ? PIXEL_SIZE : 4 + Math.random() * 4,
      color: palette[i % palette.length] ?? "#ffd27a",
    }));

    const startedAt = performance.now();
    let last = startedAt;
    let lastDrawn = 0;
    let frame = 0;
    const stepMs = 1000 / PIXEL_FPS;

    const draw = (now: number) => {
      if (document.visibilityState !== "visible" || now - startedAt >= CONFETTI_MS) {
        ctx.clearRect(0, 0, width, height);
        frame = 0;
        return;
      }
      frame = requestAnimationFrame(draw);
      if (pixel && now - lastDrawn < stepMs) return;
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      lastDrawn = now;
      const fade = 1 - (now - startedAt) / CONFETTI_MS;
      ctx.clearRect(0, 0, width, height);
      ctx.globalAlpha = Math.max(0, Math.min(1, fade * 1.6));
      for (const p of particles) {
        p.vy += GRAVITY * dt;
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.angle += p.spin * dt;
        ctx.fillStyle = p.color;
        if (pixel) {
          ctx.fillRect(Math.round(p.x / PIXEL_SIZE) * PIXEL_SIZE, Math.round(p.y / PIXEL_SIZE) * PIXEL_SIZE, PIXEL_SIZE, PIXEL_SIZE);
        } else {
          ctx.save();
          ctx.translate(p.x, p.y);
          ctx.rotate(p.angle);
          ctx.fillRect(-p.size / 2, -p.size / 4, p.size, p.size / 2);
          ctx.restore();
        }
      }
    };
    frame = requestAnimationFrame(draw);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      ctx.clearRect(0, 0, width, height);
    };
  });
</script>

<canvas class="confetti" bind:this={canvas} aria-hidden="true"></canvas>

<style>
  .confetti {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    border-radius: inherit;
  }
</style>
