<script lang="ts">
  // Halloween (beta collection theme, ADR-024): a moonlit night. A big warm moon with soft clouds drifting past, a few
  // bats crossing the sky (some over the moon), a crooked tree with an owl on the left, a scarecrow on a hill with a
  // fence on the right, glowing jack-o'-lanterns and slow ground fog. Friendly, never scary. Every motion is a slow
  // transform or opacity loop; with reduced motion the bats rest around the moon and nothing moves.
  //
  // Layout: horizontal positions are % of this layer (the window + 5vw for the tab pan, so 4.8 %–95.2 % is always on
  // screen), sizes follow the window height (`--u`: 1vh, capped by the width) so the art keeps its proportions at
  // 960 × 640, 1920 × 1080 and on portrait screens. The busy art sits at the edges and low; the middle stays calm
  // behind the cards.
  import { BAT_BODY, BAT_WINGS, type Face } from "./art";
  import Pumpkin from "./Pumpkin.svelte";

  const uid = $props.id();

  interface Bat {
    /** Height of the flight line (vh) and the bob around it (vh). */
    top: number;
    amp: number;
    /** Wingspan (vh) and direction. */
    size: number;
    rtl: boolean;
    /** One cycle: the crossing takes 72 % of it, then the bat stays off screen. */
    duration: number;
    delay: number;
    flap: number;
    bob: number;
    /** Resting place with reduced motion (% of the layer, vh) and the wing pose (scaleY). */
    restX: number;
    restY: number;
    restWings: number;
  }

  // Fixed values (deterministic, like the other scenes): different heights, sizes and speeds; two fly as a pair.
  const BATS: readonly Bat[] = [
    { top: 21, amp: 2.4, size: 7, rtl: false, duration: 32, delay: -6, flap: 0.5, bob: 2.3, restX: 63, restY: 16, restWings: 1 },
    { top: 30, amp: 3.2, size: 4.8, rtl: false, duration: 40, delay: -24, flap: 0.42, bob: 1.9, restX: 68, restY: 33, restWings: 0.45 },
    { top: 33, amp: 2.8, size: 3.9, rtl: false, duration: 40, delay: -22.8, flap: 0.4, bob: 2.1, restX: 90, restY: 39, restWings: 0.8 },
    { top: 15, amp: 2, size: 5, rtl: true, duration: 36, delay: -14, flap: 0.44, bob: 2.6, restX: 86, restY: 13, restWings: -0.4 },
    { top: 40, amp: 3.6, size: 6, rtl: true, duration: 41, delay: -30, flap: 0.54, bob: 2.4, restX: 21, restY: 25, restWings: 0.9 },
  ];

  interface Lantern {
    face: Face | null;
    /** Centre (% of the layer), bottom (vh), width (vh). */
    x: number;
    bottom: number;
    size: number;
    flicker: number;
    delay: number;
  }

  const LANTERNS: readonly Lantern[] = [
    { face: "classic", x: 13, bottom: 3, size: 10, flicker: 3.4, delay: -1.2 },
    { face: "cute", x: 19.5, bottom: 2.2, size: 6.6, flicker: 2.7, delay: -0.4 },
    { face: null, x: 81.5, bottom: 2.6, size: 5.6, flicker: 0, delay: 0 },
    { face: "happy", x: 76, bottom: 4.4, size: 8.4, flicker: 3.9, delay: -2.1 },
    { face: "cute", x: 89.4, bottom: 15.6, size: 5.2, flicker: 2.4, delay: -0.9 },
  ];

  /** Fence pickets (viewBox 0 0 400 110): x, height, tilt (deg). One leans hard, one is missing. */
  const PICKETS: readonly (readonly [number, number, number])[] = [
    [14, 74, -4],
    [44, 80, 3],
    [74, 70, -2],
    [104, 78, 6],
    [166, 72, -3],
    [196, 64, 22],
    [228, 76, 2],
    [258, 70, -5],
    [290, 80, 3],
    [320, 68, -2],
    [352, 76, 4],
    [382, 70, -6],
  ];
</script>

<div class="scene">
  <div class="stars"></div>
  <div class="stars twinkle"></div>

  <div class="halo"></div>
  <svg class="moon" viewBox="0 0 100 100" focusable="false">
    <defs>
      <radialGradient id="{uid}-moon" cx="42%" cy="40%" r="62%">
        <stop offset="0%" stop-color="#fff4da" />
        <stop offset="60%" stop-color="#f8dba6" />
        <stop offset="100%" stop-color="#eab774" />
      </radialGradient>
      <radialGradient id="{uid}-mare">
        <stop offset="0%" stop-color="#c88a52" stop-opacity="0.34" />
        <stop offset="65%" stop-color="#c88a52" stop-opacity="0.2" />
        <stop offset="100%" stop-color="#c88a52" stop-opacity="0" />
      </radialGradient>
      <radialGradient id="{uid}-limb" cx="38%" cy="36%" r="70%">
        <stop offset="70%" stop-color="#b8743a" stop-opacity="0" />
        <stop offset="100%" stop-color="#b8743a" stop-opacity="0.32" />
      </radialGradient>
    </defs>
    <circle cx="50" cy="50" r="49" fill="url(#{uid}-moon)" />
    <!-- Soft maria (each fades out at its edge) and a few small craters. -->
    <g fill="url(#{uid}-mare)">
      <ellipse cx="36" cy="33" rx="13" ry="10" />
      <ellipse cx="60" cy="25" rx="8" ry="6" />
      <ellipse cx="63" cy="54" rx="15" ry="12" />
      <ellipse cx="38" cy="63" rx="10" ry="8" />
      <ellipse cx="52" cy="80" rx="6" ry="4.5" />
    </g>
    <g fill="#c98a4e" opacity="0.16">
      <circle cx="25" cy="48" r="2.2" />
      <circle cx="76" cy="72" r="2" />
      <circle cx="47" cy="44" r="1.5" />
      <circle cx="72" cy="36" r="1.4" />
    </g>
    <circle cx="50" cy="50" r="49" fill="url(#{uid}-limb)" />
  </svg>

  <div class="cloud near"></div>
  <div class="cloud far"></div>

  {#each BATS as b, i (i)}
    <div
      class="bat"
      class:rtl={b.rtl}
      style:top="calc({b.top} * var(--u))"
      style:--size="calc({b.size} * var(--u))"
      style:--amp="calc({b.amp} * var(--u))"
      style:--rest-x="{b.restX}%"
      style:--rest-y="calc({b.restY - b.top} * var(--u))"
      style:animation-duration="{b.duration}s"
      style:animation-delay="{b.delay}s"
    >
      <div class="bob" style:animation-duration="{b.bob}s" style:animation-delay="{b.delay}s">
        <svg
          class="wings"
          viewBox="0 0 100 50"
          focusable="false"
          style:--rest-wings={b.restWings}
          style:animation-duration="{b.flap}s"
        >
          <path d={BAT_WINGS} />
        </svg>
        <svg class="body" viewBox="0 0 100 50" focusable="false"><path d={BAT_BODY} /></svg>
      </div>
    </div>
  {/each}

  <!-- Far ridge, ground fog behind the hill, then the scarecrow's hill on the right. -->
  <svg class="ridge" viewBox="0 0 1000 300" preserveAspectRatio="none" focusable="false">
    <defs>
      <linearGradient id="{uid}-ridge" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#2d1744" />
        <stop offset="100%" stop-color="#1a0d29" />
      </linearGradient>
    </defs>
    <path
      d="M0 120C60 96 120 92 190 110S320 150 400 132 520 78 610 92 760 150 850 128 960 84 1000 92V300H0z"
      fill="url(#{uid}-ridge)"
    />
  </svg>
  <!-- On the far ridge: a crooked farmhouse with one warm window, and two bare trees. -->
  <svg class="dist house" viewBox="0 0 100 80" focusable="false">
    <defs>
      <radialGradient id="{uid}-window">
        <stop offset="0%" stop-color="#ffb35c" stop-opacity="0.45" />
        <stop offset="100%" stop-color="#ffb35c" stop-opacity="0" />
      </radialGradient>
    </defs>
    <circle cx="35.5" cy="60" r="22" fill="url(#{uid}-window)" />
    <path d="M16 46l68-5 3 39H13z" />
    <path d="M4 48L50 9l46 33-5 4-41-28L9 52z" />
    <path d="M66 26l1-14 8-1 1 21z" />
    <path d="M30 54h11v12H30z" fill="#ffb35c" opacity="0.9" />
    <path d="M35 54v12M30 60h11" stroke="currentColor" stroke-width="1.6" />
    <path d="M58 58h9v22h-9z" fill="#0b0512" />
  </svg>
  <svg class="dist bare one" viewBox="0 0 60 80" focusable="false">
    <path d="M30 80c-1-14 1-28-2-40M29 50C20 42 16 34 8 30M29 44c8-8 12-16 22-20M28 38c-2-10 2-18-1-28M18 36c-2-6 0-10-4-14M42 30c2-6 6-8 6-14" />
  </svg>
  <svg class="dist bare two" viewBox="0 0 60 80" focusable="false">
    <path d="M30 80c1-12-1-24 2-36M31 52c8-6 14-14 22-16M31 46C23 40 18 30 10 28M32 40c2-10-2-18 1-26M44 38c4-4 4-10 8-12M18 30c0-6-4-10-2-14" />
  </svg>
  <div class="fog fog-back"></div>
  <svg class="hill" viewBox="0 0 400 200" preserveAspectRatio="none" focusable="false">
    <defs>
      <linearGradient id="{uid}-hill" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#241237" />
        <stop offset="100%" stop-color="#160b24" />
      </linearGradient>
    </defs>
    <path d="M0 200C50 170 110 120 170 72 210 44 250 34 290 46 340 62 370 100 400 116V200z" fill="url(#{uid}-hill)" />
  </svg>

  <div class="tree">
    <svg viewBox="0 0 400 600" focusable="false">
      <!-- A crooked trunk (filled, tapering) with stroked limbs and twigs. -->
      <path
        d="M116 600C118 560 112 540 114 520 116 490 110 476 114 458 118 432 124 414 133 398 142 380 148 360 151 340 154 318 157 300 160 280 163 260 166 240 169 220 173 200 177 180 181 160L191 160C190 180 188 200 187 220 186 240 185 260 184 280 183 300 183 320 181 340 178 362 172 382 167 400 162 418 156 438 157 460 158 482 162 500 166 520 170 545 176 575 184 600z"
        fill="currentColor"
      />
      <path d="M86 600c22-8 30-24 32-44h50c4 22 20 38 52 44z" fill="currentColor" />
      <g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round">
        <path d="M130 410C104 382 84 360 54 342" stroke-width="15" />
        <path d="M54 342C34 330 26 312 14 288" stroke-width="7" />
        <path d="M14 288C10 278 12 268 6 258" stroke-width="3" />
        <path d="M80 360C76 334 86 316 76 292" stroke-width="6" />
        <path d="M76 292C72 280 78 270 72 256" stroke-width="3" />
        <path d="M40 334C28 340 16 336 4 344" stroke-width="3.5" />
        <path d="M178 330C212 316 242 326 282 306" stroke-width="12" />
        <path d="M282 306C312 290 334 296 362 272" stroke-width="6" />
        <path d="M362 272C372 264 378 254 390 250" stroke-width="3" />
        <path d="M252 318C264 336 284 340 300 356" stroke-width="5" />
        <path d="M300 356C306 364 316 366 322 376" stroke-width="2.5" />
        <path d="M322 290C324 272 338 264 340 248" stroke-width="3.5" />
        <path d="M172 250C150 240 140 222 120 210" stroke-width="7" />
        <path d="M120 210C110 204 106 192 96 186" stroke-width="3" />
        <path d="M184 200C206 184 222 164 246 156" stroke-width="7" />
        <path d="M246 156C260 150 266 138 280 134" stroke-width="3.5" />
        <path d="M186 164C184 132 194 112 188 82" stroke-width="5" />
        <path d="M188 82C186 68 192 58 188 44" stroke-width="2.5" />
        <path d="M190 112C204 104 206 92 218 86" stroke-width="2.5" />
      </g>
      <!-- An owl on the long branch: ear tufts, round amber eyes. -->
      <g transform="translate(232 276)">
        <path
          d="M2 34C-2 22 0 8 6 2l4 7c3-2 9-2 12 0l4-7c6 6 8 20 4 32-3 8-9 12-14 12S5 42 2 34z"
          fill="currentColor"
        />
        <circle cx="11" cy="15" r="3.6" fill="#ffbe55" />
        <circle cx="21" cy="15" r="3.6" fill="#ffbe55" />
        <circle cx="11.4" cy="15.4" r="1.6" fill="#1a0c06" />
        <circle cx="20.6" cy="15.4" r="1.6" fill="#1a0c06" />
        <path d="M16 18l-1.6 3h3.2z" fill="#c98a3a" />
      </g>
    </svg>
  </div>

  <div class="scarecrow">
    <svg viewBox="0 0 200 320" focusable="false">
      <g fill="currentColor">
        <!-- Pole and the cross stick. -->
        <rect x="96" y="70" width="8" height="250" rx="2" />
        <path d="M24 122l152-6 .4 7-152 6z" />
        <!-- Ragged shirt and sleeves. -->
        <path d="M64 104h72l10 22-12 4 2 70-10-8-8 12-10-10-10 12-10-13-10 10-8-9-2-74-12-4z" />
        <path d="M66 106l-34 8-6 14 8-3 4 8 4-8 30 3z" />
        <path d="M134 106l34 6 6 14-8-3-4 8-4-8-30 3z" />
        <!-- Sack head, a tie at the neck, and a floppy pointed hat. -->
        <circle cx="100" cy="84" r="19" />
        <path d="M92 100l8 6 8-6-3 8 3 6-8-4-8 4 3-6z" />
        <ellipse cx="100" cy="66" rx="36" ry="6.5" />
        <path d="M80 64c4-20 14-34 32-42-4 12 4 18 10 22-2 8-2 14 0 20z" />
        <path d="M112 22c6-2 12 0 16 6-6-1-10 1-13 4z" />
        <!-- A crow resting on the left arm. -->
        <path d="M30 116c-2-8 2-14 8-15 3-2 6-1 8 1l5 1-5 2c1 6-2 11-8 12l-4 6 1-6c-3 0-4-1-5-1z" />
      </g>
      <!-- Straw at the cuffs and the collar, in a dusty gold so the figure reads against the hill. -->
      <g fill="none" stroke="#5a4020" stroke-width="2.6" stroke-linecap="round">
        <path
          d="M26 126l-10-4M27 129l-12 2M28 132l-8 7M174 124l10-4M173 127l12 2M172 130l8 7M94 104l-4 6M100 105v7M106 104l4 6"
        />
      </g>
      <!-- Stitched smile and button eyes, barely there. -->
      <g fill="none" stroke="#4a2d4f" stroke-width="2" stroke-linecap="round">
        <path d="M90 90c6 4 14 4 20 0M93 89l-1 3M100 91v3M107 89l1 3" />
      </g>
      <circle cx="93" cy="80" r="2.6" fill="#4a2d4f" />
      <circle cx="107" cy="80" r="2.6" fill="#4a2d4f" />
    </svg>
  </div>

  <svg class="fence" viewBox="0 0 400 110" preserveAspectRatio="xMidYMax meet" focusable="false">
    <g fill="currentColor">
      <path d="M0 52c80-6 160 2 240-6s120 0 160-4l.4 7c-40 4-80-4-160 4S80 53 0 59z" />
      <path d="M0 82c90-2 170 4 260-2 60-4 100 0 140-2v7c-40 2-80-2-140 2-90 6-170 0-260 2z" />
      {#each PICKETS as [x, h, tilt], i (i)}
        <path d="M{x - 7} 110v-{h - 10}l7-10 7 10v{h - 10}z" transform="rotate({tilt} {x} 110)" />
      {/each}
    </g>
  </svg>

  <svg class="ground" viewBox="0 0 1000 120" preserveAspectRatio="none" focusable="false">
    <path d="M0 40C80 24 160 22 260 34S420 52 520 40 700 18 800 30 940 44 1000 36V120H0z" />
  </svg>

  {#each LANTERNS as l, i (i)}
    <div
      class="lantern"
      style:left="{l.x}%"
      style:bottom="calc({l.bottom} * var(--u))"
      style:--size="calc({l.size} * var(--u))"
    >
      {#if l.face}
        <span class="glow" style:animation-duration="{l.flicker}s" style:animation-delay="{l.delay}s"></span>
      {/if}
      <span class="pool" class:lit={l.face !== null}></span>
      <span class="pumpkin"><Pumpkin face={l.face} /></span>
    </div>
  {/each}

  <div class="fog fog-front"></div>
</div>

<style>
  .scene {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    /* Art unit: 1vh, but never more than 0.75vw, so tall (portrait) windows keep the art in proportion. */
    --u: min(1vh, 0.75vw);
    /* Silhouette shades, back to front; the moon's place (shared by the halo, the moon and the resting bats). */
    --hw-near: #0e0717;
    --hw-figure: #0b0512;
    --moon-top: clamp(76px, calc(12 * var(--u)), 150px);
    --moon-size: calc(24 * var(--u));
  }

  svg {
    display: block;
  }

  /* ---------- sky ---------- */

  .stars {
    position: absolute;
    inset: 0 0 45% 0;
    background-image:
      radial-gradient(circle at 30% 40%, rgba(255, 240, 220, 0.9) 0 1px, transparent 1.6px),
      radial-gradient(circle at 70% 20%, rgba(255, 240, 220, 0.7) 0 1px, transparent 1.6px),
      radial-gradient(circle at 50% 80%, rgba(220, 200, 255, 0.7) 0 1.2px, transparent 1.8px);
    background-size:
      173px 131px,
      227px 197px,
      311px 263px;
    mask-image: linear-gradient(180deg, #000 0%, #000 55%, transparent 100%);
    opacity: 0.5;
  }

  .stars.twinkle {
    background-image:
      radial-gradient(circle at 20% 60%, rgba(255, 244, 228, 0.95) 0 1.3px, transparent 2px),
      radial-gradient(circle at 80% 30%, rgba(230, 214, 255, 0.85) 0 1px, transparent 1.6px);
    background-size:
      389px 223px,
      281px 167px;
    opacity: 0.3;
    animation: twinkle 7s ease-in-out infinite alternate;
  }

  .halo {
    position: absolute;
    top: var(--moon-top);
    left: 73%;
    width: var(--moon-size);
    height: var(--moon-size);
    transform: scale(3.6);
    border-radius: 50%;
    background: radial-gradient(
      circle,
      rgba(255, 214, 150, 0.3) 0%,
      rgba(255, 170, 96, 0.13) 25%,
      rgba(200, 110, 160, 0.05) 46%,
      transparent 70%
    );
  }

  .moon {
    position: absolute;
    top: var(--moon-top);
    left: 73%;
    width: var(--moon-size);
    height: var(--moon-size);
  }

  /* Soft clouds (gradients only, no blur) gliding across the sky, one of them over the moon. */
  .cloud {
    position: absolute;
    left: 0;
    width: calc(76 * var(--u));
    height: calc(9 * var(--u));
    background:
      radial-gradient(28% 46% at 20% 60%, rgba(76, 50, 102, 0.8), transparent 72%),
      radial-gradient(26% 60% at 42% 48%, rgba(86, 58, 112, 0.85), transparent 72%),
      radial-gradient(24% 52% at 64% 56%, rgba(78, 52, 104, 0.8), transparent 72%),
      radial-gradient(20% 40% at 84% 62%, rgba(70, 46, 96, 0.7), transparent 72%);
    animation: drift-cloud linear infinite;
    will-change: transform;
  }

  .cloud.near {
    top: calc(var(--moon-top) + 14 * var(--u));
    opacity: 0.75;
    animation-duration: 120s;
    animation-delay: -46s;
  }

  .cloud.far {
    top: calc(24 * var(--u));
    height: calc(6 * var(--u));
    width: calc(52 * var(--u));
    opacity: 0.5;
    animation-duration: 170s;
    animation-delay: -120s;
  }

  /* ---------- bats ---------- */

  .bat {
    position: absolute;
    left: 0;
    width: var(--size);
    height: calc(var(--size) / 2);
    color: #09040f;
    animation-name: fly;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    will-change: transform;
  }

  .bat.rtl {
    animation-name: fly-back;
  }

  .bob {
    position: absolute;
    inset: 0;
    animation: bob ease-in-out infinite alternate;
  }

  .bat svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: currentColor;
    overflow: visible;
    /* A faint moonlit rim, so the silhouettes read against the dark sky. */
    stroke: rgba(170, 130, 220, 0.4);
    stroke-width: 1.4px;
    stroke-linejoin: round;
    paint-order: stroke;
  }

  .wings {
    transform-origin: 50% 40%;
    animation: flap ease-in-out infinite alternate;
    will-change: transform;
  }

  /* ---------- ground ---------- */

  .ridge {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: calc(34 * var(--u));
  }

  .hill {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 40%;
    height: calc(24 * var(--u));
  }

  .tree {
    position: absolute;
    left: 0;
    bottom: calc(4 * var(--u));
    height: calc(66 * var(--u));
    aspect-ratio: 400 / 600;
    color: var(--hw-figure);
  }

  .tree svg {
    width: 100%;
    height: 100%;
  }

  /* Far ridge details (x on the ridge, bottom where the ridge line is there). */
  .dist {
    position: absolute;
    overflow: visible;
  }

  .dist.house {
    left: 29%;
    bottom: calc(17.4 * var(--u));
    height: calc(8.5 * var(--u));
    width: calc(8.5 * var(--u) * 100 / 80);
    fill: #170b24;
    color: #170b24;
  }

  .dist.bare {
    height: calc(9 * var(--u));
    width: calc(9 * var(--u) * 60 / 80);
    fill: none;
    stroke: #1b0e2b;
    stroke-width: 2.6;
    stroke-linecap: round;
  }

  .dist.bare.one {
    left: 54%;
    bottom: calc(23.6 * var(--u));
  }

  .dist.bare.two {
    left: 63%;
    bottom: calc(22 * var(--u));
    height: calc(7 * var(--u));
    width: calc(7 * var(--u) * 60 / 80);
  }

  /* The scarecrow stands on the hill's top (84 % across, about 21 units up) and sways a little. */
  .scarecrow {
    position: absolute;
    left: 84%;
    bottom: calc(18.6 * var(--u));
    height: calc(34 * var(--u));
    aspect-ratio: 200 / 320;
    margin-left: calc(34 * var(--u) * 200 / 320 / -2);
    color: var(--hw-figure);
    transform-origin: 50% 100%;
    animation: sway 6.5s ease-in-out infinite alternate;
  }

  .scarecrow svg {
    width: 100%;
    height: 100%;
  }

  .fence {
    position: absolute;
    left: 70%;
    bottom: calc(3.2 * var(--u));
    width: 27%;
    height: calc(11 * var(--u));
    color: var(--hw-figure);
  }

  .ground {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: calc(12 * var(--u));
    fill: var(--hw-near);
  }

  /* ---------- jack-o'-lanterns ---------- */

  .lantern {
    position: absolute;
    width: var(--size);
    height: calc(var(--size) * 100 / 120);
    margin-left: calc(var(--size) / -2);
  }

  .pumpkin {
    position: absolute;
    inset: 0;
  }

  .glow {
    position: absolute;
    left: 50%;
    top: 55%;
    width: calc(var(--size) * 3.2);
    height: calc(var(--size) * 3.2);
    margin: calc(var(--size) * -1.6) 0 0 calc(var(--size) * -1.6);
    border-radius: 50%;
    background: radial-gradient(circle, rgba(255, 170, 70, 0.42) 0%, rgba(255, 128, 40, 0.16) 34%, transparent 68%);
    animation: flicker ease-in-out infinite;
  }

  .pool {
    position: absolute;
    left: -30%;
    right: -30%;
    bottom: -12%;
    height: 30%;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgba(0, 0, 0, 0.45), transparent);
  }

  .pool.lit {
    background: radial-gradient(closest-side, rgba(255, 150, 60, 0.28), transparent);
  }

  /* ---------- fog ---------- */

  /* Two tiles side by side; sliding by one tile (50 %) loops without a seam. Blobs stay inside their tile. */
  .fog {
    position: absolute;
    left: 0;
    width: 200%;
    background-image:
      radial-gradient(24% 46% at 25% 58%, rgba(196, 176, 230, 0.55), transparent 70%),
      radial-gradient(22% 40% at 55% 46%, rgba(196, 176, 230, 0.45), transparent 70%),
      radial-gradient(21% 50% at 82% 62%, rgba(196, 176, 230, 0.5), transparent 70%);
    background-size: 50% 100%;
    background-repeat: repeat-x;
    will-change: transform;
  }

  .fog-back {
    bottom: calc(10 * var(--u));
    height: calc(22 * var(--u));
    opacity: 0.32;
    animation: drift-left 84s linear infinite;
  }

  .fog-front {
    bottom: calc(-5 * var(--u));
    height: calc(18 * var(--u));
    opacity: 0.28;
    animation: drift-right 66s linear infinite;
    animation-delay: -20s;
  }

  /* ---------- motion ---------- */

  @keyframes twinkle {
    from {
      opacity: 0.15;
    }
    to {
      opacity: 0.55;
    }
  }

  @keyframes fly {
    0% {
      transform: translate3d(-10vw, 0, 0);
    }
    72%,
    100% {
      transform: translate3d(110vw, 0, 0);
    }
  }

  @keyframes fly-back {
    0% {
      transform: translate3d(110vw, 0, 0);
    }
    72%,
    100% {
      transform: translate3d(-10vw, 0, 0);
    }
  }

  @keyframes bob {
    from {
      transform: translate3d(0, calc(var(--amp) * -1), 0);
    }
    to {
      transform: translate3d(0, var(--amp), 0);
    }
  }

  @keyframes flap {
    from {
      transform: scaleY(1);
    }
    to {
      transform: scaleY(-0.55);
    }
  }

  @keyframes sway {
    from {
      transform: rotate(-1.8deg);
    }
    to {
      transform: rotate(1.8deg);
    }
  }

  /* A slow, irregular wobble between 0.78 and 1 (never a strobe). */
  @keyframes flicker {
    0%,
    100% {
      opacity: 0.9;
    }
    18% {
      opacity: 1;
    }
    34% {
      opacity: 0.8;
    }
    52% {
      opacity: 0.96;
    }
    70% {
      opacity: 0.78;
    }
    86% {
      opacity: 0.94;
    }
  }

  @keyframes drift-left {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(-50%, 0, 0);
    }
  }

  @keyframes drift-right {
    from {
      transform: translate3d(-50%, 0, 0);
    }
    to {
      transform: translate3d(0, 0, 0);
    }
  }

  @keyframes drift-cloud {
    from {
      transform: translate3d(110vw, 0, 0);
    }
    to {
      transform: translate3d(calc(-80 * var(--u)), 0, 0);
    }
  }

  /* Reduced motion: the same night, still. Bats rest around the moon, a cloud sits across it. */
  @media (prefers-reduced-motion: reduce) {
    .stars.twinkle,
    .cloud,
    .bat,
    .bob,
    .wings,
    .scarecrow,
    .glow,
    .fog {
      animation: none;
    }

    .bat {
      left: var(--rest-x);
      transform: translate3d(0, var(--rest-y), 0);
    }

    .wings {
      transform: scaleY(var(--rest-wings));
    }

    .cloud.near {
      transform: translate3d(62vw, 0, 0);
    }

    .cloud.far {
      transform: translate3d(8vw, 0, 0);
    }

    .fog-front {
      transform: translate3d(-18%, 0, 0);
    }
  }
</style>
