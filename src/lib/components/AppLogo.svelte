<script lang="ts">
  /** The FocusMode moon mark (decorative). */
  let { size = 30 }: { size?: number } = $props();
</script>

<svg viewBox="0 0 32 32" width={size} height={size} aria-hidden="true" focusable="false">
  <circle cx="16" cy="16" r="15" style="fill: var(--logo-bg)" />
  <path d="M21.5 19.5A7.5 7.5 0 1 1 13 9a6 6 0 0 0 8.5 10.5z" style="fill: var(--logo-moon)" />
  <circle cx="22.5" cy="10" r="1.2" style="fill: var(--logo-star)" />
</svg>
