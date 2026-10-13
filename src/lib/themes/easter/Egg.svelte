<script lang="ts" module>
  export type EggPattern = "zigzag" | "stripes" | "dots" | "wave" | "flowers";

  export interface EggSpec {
    /** Place in the parent SVG (its units). */
    x: number;
    y: number;
    width: number;
    tilt: number;
    base: string;
    /** Pattern colours. */
    a: string;
    b: string;
    pattern: EggPattern;
  }
</script>

<script lang="ts">
  // A painted Easter egg (viewBox 0 0 60 80) placed inside a parent SVG: pastel shell, one simple pattern, a soft light
  // from the upper left and a shadow on the lower right.
  let { egg }: { egg: EggSpec } = $props();

  const uid = $props.id();
  const SHELL = "M30 2C45 2 57 28 57 50 57 67 45 78 30 78 15 78 3 67 3 50 3 28 15 2 30 2Z";
  const ZIGZAG = Array.from({ length: 11 }, (_, i) => `${i === 0 ? "M" : "L"}${i * 6} ${i % 2 === 0 ? 46 : 38}`).join("");
</script>

<svg x={egg.x} y={egg.y} width={egg.width} height={(egg.width * 80) / 60} viewBox="0 0 60 80" overflow="visible">
  <defs>
    <clipPath id="{uid}-shell"><path d={SHELL} /></clipPath>
    <radialGradient id="{uid}-light" cx="34%" cy="26%" r="60%">
      <stop offset="0%" stop-color="#ffffff" stop-opacity="0.55" />
      <stop offset="100%" stop-color="#ffffff" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{uid}-shade" cx="78%" cy="86%" r="70%">
      <stop offset="0%" stop-color="#4a3a5e" stop-opacity="0.26" />
      <stop offset="100%" stop-color="#4a3a5e" stop-opacity="0" />
    </radialGradient>
  </defs>
  <g transform="rotate({egg.tilt} 30 40)">
    <path d={SHELL} fill={egg.base} />
    <g clip-path="url(#{uid}-shell)">
      {#if egg.pattern === "zigzag"}
        <path d={ZIGZAG} fill="none" stroke={egg.a} stroke-width="4.5" stroke-linejoin="round" />
        <g fill={egg.b}>
          <circle cx="16" cy="24" r="2.6" /><circle cx="30" cy="20" r="2.6" /><circle cx="44" cy="24" r="2.6" />
          <circle cx="14" cy="62" r="2.6" /><circle cx="30" cy="66" r="2.6" /><circle cx="46" cy="62" r="2.6" />
        </g>
      {:else if egg.pattern === "stripes"}
        <rect x="0" y="22" width="60" height="6" fill={egg.a} />
        <rect x="0" y="40" width="60" height="9" fill={egg.b} />
        <rect x="0" y="61" width="60" height="5" fill={egg.a} />
      {:else if egg.pattern === "dots"}
        <g fill={egg.a}>
          <circle cx="20" cy="18" r="3.4" /><circle cx="38" cy="26" r="4" /><circle cx="16" cy="40" r="4.4" />
          <circle cx="34" cy="48" r="3.6" /><circle cx="50" cy="44" r="3.2" /><circle cx="22" cy="64" r="3.8" />
          <circle cx="42" cy="66" r="3.4" />
        </g>
        <g fill={egg.b}><circle cx="28" cy="32" r="1.8" /><circle cx="46" cy="56" r="1.8" /><circle cx="12" cy="54" r="1.8" /></g>
      {:else if egg.pattern === "wave"}
        <path d="M0 40q7.5-9 15 0t15 0 15 0 15 0" fill="none" stroke={egg.a} stroke-width="6" />
        <path d="M0 26q7.5-6 15 0t15 0 15 0 15 0M0 56q7.5-6 15 0t15 0 15 0 15 0" fill="none" stroke={egg.b} stroke-width="2.6" />
      {:else}
        {#each [[18, 26], [40, 22], [28, 44], [14, 60], [44, 58]] as [cx, cy], i (i)}
          <g fill={egg.a}>
            <circle cx={cx} cy={cy - 3} r="2.4" /><circle cx={cx + 3} cy={cy} r="2.4" /><circle cx={cx} cy={cy + 3} r="2.4" />
            <circle cx={cx - 3} cy={cy} r="2.4" />
          </g>
          <circle cx={cx} cy={cy} r="1.6" fill={egg.b} />
        {/each}
      {/if}
      <rect width="60" height="80" fill="url(#{uid}-shade)" />
      <rect width="60" height="80" fill="url(#{uid}-light)" />
    </g>
  </g>
</svg>
