<script lang="ts">
  import { formatClock } from "$lib/format";
  import { strings } from "$lib/strings";
  import type { Phase } from "$lib/types";

  interface Props {
    remainingSecs: number;
    totalSecs: number;
    phase: Phase;
    paused: boolean;
    cycle: number;
    demo: boolean;
  }

  let { remainingSecs, totalSecs, phase, paused, cycle, demo }: Props = $props();

  const SIZE = 300;
  const STROKE = 12;
  const CENTER = SIZE / 2;
  const RADIUS = CENTER - STROKE - 8;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
  const TICKS = 60;

  const progress = $derived(totalSecs > 0 ? Math.min(1, Math.max(0, remainingSecs / totalSecs)) : 1);
  const dashOffset = $derived(CIRCUMFERENCE * (1 - progress));
  const clock = $derived(formatClock(remainingSecs));
  const phaseLabel = $derived(strings.phase[phase]);

  const ticks = Array.from({ length: TICKS }, (_, i) => {
    const angle = (i / TICKS) * 2 * Math.PI;
    const major = i % 5 === 0;
    const outer = RADIUS + STROKE / 2 + 6;
    const inner = outer - (major ? 7 : 4);
    return {
      x1: CENTER + inner * Math.sin(angle),
      y1: CENTER - inner * Math.cos(angle),
      x2: CENTER + outer * Math.sin(angle),
      y2: CENTER - outer * Math.cos(angle),
      major,
    };
  });
</script>

<div
  class="ring"
  class:focus={phase === "focus"}
  class:break={phase === "break"}
  class:paused
  role="timer"
  aria-label={strings.timer.remainingLabel(phaseLabel, clock)}
>
  <svg viewBox="0 0 {SIZE} {SIZE}" aria-hidden="true">
    <defs>
      <radialGradient id="ring-glow" cx="50%" cy="50%" r="50%">
        <stop offset="60%" style="stop-color: var(--ring-color); stop-opacity: 0.1" />
        <stop offset="100%" style="stop-color: var(--ring-color); stop-opacity: 0" />
      </radialGradient>
    </defs>
    <circle cx={CENTER} cy={CENTER} r={CENTER - 2} fill="url(#ring-glow)" />
    {#each ticks as t, i (i)}
      <line class="tick" class:major={t.major} x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2} />
    {/each}
    <circle class="track" cx={CENTER} cy={CENTER} r={RADIUS} stroke-width={STROKE} fill="none" />
    <circle
      class="progress"
      cx={CENTER}
      cy={CENTER}
      r={RADIUS}
      stroke-width={STROKE}
      fill="none"
      stroke-dasharray={CIRCUMFERENCE}
      stroke-dashoffset={dashOffset}
      transform="rotate(-90 {CENTER} {CENTER})"
    />
    {#if progress > 0}
      <g class="knob" style:transform="rotate({progress * 360}deg)">
        <circle cx={CENTER} cy={CENTER - RADIUS} r={STROKE / 2 + 3} />
      </g>
    {/if}
  </svg>

  <div class="center">
    <span class="phase">{phaseLabel}</span>
    <span class="time">{clock}</span>
    <span class="meta">
      {#if cycle > 0}<span>{strings.timer.cycle(cycle)}</span>{/if}
      {#if paused}<span class="badge pause-badge">{strings.timer.paused}</span>{/if}
    </span>
    {#if demo}<span class="badge demo-badge">{strings.timer.demo}</span>{/if}
  </div>
</div>

<style>
  .ring {
    --ring-color: var(--text-muted);
    position: relative;
    width: min(300px, 78vw);
    aspect-ratio: 1;
    margin: 0 auto;
  }

  .ring.focus {
    --ring-color: var(--accent);
  }

  .ring.break {
    --ring-color: var(--break);
  }

  svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .tick {
    stroke: var(--text-muted);
    stroke-width: 1;
    opacity: 0.35;
  }

  .tick.major {
    stroke-width: 1.6;
    opacity: 0.6;
  }

  .track {
    stroke: color-mix(in srgb, var(--text) 8%, transparent);
  }

  .progress {
    stroke: var(--ring-color);
    stroke-linecap: round;
    transition:
      stroke-dashoffset 1s linear,
      stroke 0.4s ease;
    filter: drop-shadow(0 0 8px color-mix(in srgb, var(--ring-color) 45%, transparent));
  }

  .knob {
    transform-origin: 50% 50%;
    transform-box: view-box;
    transition: transform 1s linear;
  }

  .knob circle {
    fill: var(--knob-fill);
    stroke: var(--ring-color);
    stroke-width: 3;
  }

  .paused .progress,
  .paused .knob circle {
    opacity: 0.55;
  }

  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    text-align: center;
  }

  .phase {
    font-size: 0.95rem;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ring-color);
  }

  .ring:not(.focus):not(.break) .phase {
    color: var(--text-2);
  }

  .time {
    font-size: clamp(2.8rem, 12vw, 4.2rem);
    font-weight: 300;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    line-height: 1.05;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
    color: var(--text-2);
    font-size: 0.9rem;
  }

  .pause-badge {
    background: color-mix(in srgb, var(--text) 12%, transparent);
    color: var(--text);
    animation: blink 1.6s ease-in-out infinite;
  }

  .demo-badge {
    margin-top: 2px;
  }

  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
    }
  }
</style>
