<script lang="ts">
  // A snowy pine (viewBox 0 0 100 170). `far` draws it in the hazier colours of the back row.
  import { PINE } from "./art";

  let { far = false }: { far?: boolean } = $props();
</script>

<svg viewBox="0 0 100 170" class:far focusable="false" aria-hidden="true">
  <path d={PINE.trunk} class="trunk" />
  <path d={PINE.tiers} class="tiers" />
  <path d={PINE.snow} class="snow" />
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .trunk {
    fill: #1a1622;
  }

  .tiers {
    fill: #0d2633;
  }

  .snow {
    fill: #aebfdb;
  }

  .far .tiers {
    fill: #17304a;
  }

  .far .snow {
    fill: #8597b8;
  }
</style>
