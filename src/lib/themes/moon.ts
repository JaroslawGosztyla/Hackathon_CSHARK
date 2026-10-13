// Moon phase for the "Lunar" theme: a simple mean synodic-month model (good to within about a day, which is
// plenty for a background picture). Pure functions, no clock reads: callers pass the date.

/** Mean length of a lunation (new moon to new moon), in days. */
export const SYNODIC_MONTH_DAYS = 29.530588853;

/** A known new moon used as the epoch: 2000-01-06 18:14 UTC. */
const REFERENCE_NEW_MOON_MS = Date.UTC(2000, 0, 6, 18, 14);
const DAY_MS = 86_400_000;

export type MoonPhaseName =
  | "new"
  | "waxingCrescent"
  | "firstQuarter"
  | "waxingGibbous"
  | "full"
  | "waningGibbous"
  | "lastQuarter"
  | "waningCrescent";

/** Days on either side of new moon, a quarter or full moon that still carry that phase's name. */
const PRINCIPAL_PHASE_DAYS = 1;

/** Name for a moon `ageDays` old: the four principal phases for ±1 day, crescents and gibbous moons in between. */
function phaseName(ageDays: number): MoonPhaseName {
  const quarter = SYNODIC_MONTH_DAYS / 4;
  const near = (day: number) => Math.abs(ageDays - day) <= PRINCIPAL_PHASE_DAYS;
  if (near(0) || near(SYNODIC_MONTH_DAYS)) return "new";
  if (near(quarter)) return "firstQuarter";
  if (near(2 * quarter)) return "full";
  if (near(3 * quarter)) return "lastQuarter";
  if (ageDays < quarter) return "waxingCrescent";
  if (ageDays < 2 * quarter) return "waxingGibbous";
  if (ageDays < 3 * quarter) return "waningGibbous";
  return "waningCrescent";
}

export interface MoonPhase {
  /** Position in the lunation, 0..1: 0 new moon, 0.25 first quarter, 0.5 full moon, 0.75 last quarter. */
  fraction: number;
  /** Days since the last new moon, 0..29.53. */
  ageDays: number;
  /** Lit part of the disk, 0..1. */
  illumination: number;
  name: MoonPhaseName;
}

export function moonPhase(date: Date): MoonPhase {
  const days = (date.getTime() - REFERENCE_NEW_MOON_MS) / DAY_MS;
  const ageDays = ((days % SYNODIC_MONTH_DAYS) + SYNODIC_MONTH_DAYS) % SYNODIC_MONTH_DAYS;
  const fraction = ageDays / SYNODIC_MONTH_DAYS;
  const illumination = (1 - Math.cos(2 * Math.PI * fraction)) / 2;
  return { fraction, ageDays, illumination, name: phaseName(ageDays) };
}

/**
 * SVG path of the lit part of a moon disk (centre `cx`,`cy`, radius `r`) at lunation `fraction`, as seen from the
 * northern hemisphere: waxing light on the right, waning light on the left. The terminator is half an ellipse
 * whose horizontal radius is `r·|cos(2π·fraction)|`.
 */
export function litPath(fraction: number, cx: number, cy: number, r: number): string {
  const f = ((fraction % 1) + 1) % 1;
  const waxing = f < 0.5;
  const cos = Math.cos(2 * Math.PI * f);
  // Horizontal offset of the terminator's middle from the centre (+ = right).
  const offset = waxing ? r * cos : -r * cos;
  const rx = Math.abs(offset);
  // SVG arcs, y pointing down: sweep 1 = clockwise on screen.
  const limbSweep = waxing ? 1 : 0;
  const terminatorSweep = offset > 0 ? 0 : 1;
  const n = (v: number) => Number(v.toFixed(3));
  return (
    `M ${n(cx)} ${n(cy - r)} ` +
    `A ${n(r)} ${n(r)} 0 0 ${limbSweep} ${n(cx)} ${n(cy + r)} ` +
    `A ${n(rx)} ${n(r)} 0 0 ${terminatorSweep} ${n(cx)} ${n(cy - r)} Z`
  );
}
