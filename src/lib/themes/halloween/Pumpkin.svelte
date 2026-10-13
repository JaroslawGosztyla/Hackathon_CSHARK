<script lang="ts">
  // A jack-o'-lantern (or a plain pumpkin without `face`), viewBox 0 0 120 100. Flat shapes: darker outer ribs, a warm
  // carved face lit from inside. The glow around it belongs to the caller (it flickers there).
  import { FACES, PUMPKIN_LEAF, PUMPKIN_RIBS, PUMPKIN_STEM, type Face } from "./art";

  let { face = null }: { face?: Face | null } = $props();

  const uid = $props.id();
  const SHADES = ["#6e290b", "#8a3a10", "#a14a16"] as const;
  const UNLIT = ["#5a2a12", "#6d3416", "#7d3e1a"] as const;
  const shades = $derived(face ? SHADES : UNLIT);
</script>

<svg viewBox="0 0 120 100" focusable="false" aria-hidden="true">
  <defs>
    <radialGradient id="{uid}-flame" cx="50%" cy="62%" r="60%">
      <stop offset="0%" stop-color="#fff3b8" />
      <stop offset="45%" stop-color="#ffc35a" />
      <stop offset="100%" stop-color="#ff8a24" />
    </radialGradient>
    <linearGradient id="{uid}-shine" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#ffb070" stop-opacity="0.32" />
      <stop offset="55%" stop-color="#ffb070" stop-opacity="0" />
    </linearGradient>
  </defs>
  {#each PUMPKIN_RIBS as [cx, cy, rx, ry, shade], i (i)}
    <ellipse {cx} {cy} {rx} {ry} fill={shades[shade]} stroke="#3a1505" stroke-opacity="0.55" stroke-width="1.4" />
  {/each}
  <!-- Rim light from the glowing face on the lower half, a soft sheen on top. -->
  <ellipse cx="60" cy="60" rx="52" ry="36" fill="url(#{uid}-shine)" />
  <path d={PUMPKIN_STEM} fill="#3f2a12" />
  <path d={PUMPKIN_LEAF} fill="#2f3a16" />
  {#if face}
    <path d={FACES[face].cut} fill="url(#{uid}-flame)" />
    {#if FACES[face].teeth}<path d={FACES[face].teeth} fill={shades[2]} />{/if}
  {/if}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
</style>
