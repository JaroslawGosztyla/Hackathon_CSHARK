// Break tips and good working practices (research notes in the docs; sources per tip).
// Metadata plus a deterministic pick: the main window's break card and the phase popup must show the same tip.
// The texts live in the locale files (`strings.tips`, `strings.workingTips`, `strings.stretchFootnote` in `$lib/i18n`).
// Wording rule (CLAUDE.md, rule 10): suggestions only, never health promises.
import { language, strings } from "./i18n/index.svelte";
import type { Strings } from "./i18n/pl";

export type TipCategory = "eyes" | "stretch" | "move" | "hydrate" | "breathe" | "posture" | "outside" | "social" | "mind" | "tidy";

/** Break length a tip fits: `short` (< 10 min), `long` (10 min or more) or `any`. */
export type TipLength = "short" | "long" | "any";

/** Id of a break tip: a key of `strings.tips` in every locale. */
export type BreakTipId = keyof Strings["tips"];
/** Id of a good working practice: a key of `strings.workingTips` in every locale. */
export type WorkingTipId = keyof Strings["workingTips"];

export interface BreakTip {
  id: BreakTipId;
  category: TipCategory;
  length: TipLength;
  /** Where the suggestion comes from (opened in the browser); null = common practice without one source. */
  source: string | null;
}

export interface WorkingTip {
  id: WorkingTipId;
  source: string;
}

/** Icon per category (decorative; the category name is shown as text next to it). */
export const CATEGORY_ICON: Record<TipCategory, string> = {
  eyes: "👀",
  stretch: "🙆",
  move: "🚶",
  hydrate: "💧",
  breathe: "🌬️",
  posture: "🪑",
  outside: "🌳",
  social: "💬",
  mind: "🧘",
  tidy: "🧹",
};

export const BREAK_TIPS: readonly BreakTip[] = [
  {
    id: "eyes-20-20-20",
    category: "eyes",
    length: "any",
    source: "https://www.aao.org/eye-health/diseases/what-is-eye-strain",
  },
  {
    id: "eyes-green-view",
    category: "eyes",
    length: "any",
    source: "https://doi.org/10.1016/j.jenvp.2015.04.003",
  },
  {
    id: "eyes-slow-blinks",
    category: "eyes",
    length: "short",
    source: "https://www.aao.org/eye-health/diseases/computer-usage",
  },
  {
    id: "eyes-look-around",
    category: "eyes",
    length: "short",
    source: "https://www.ciop.pl/CIOPPortalWAR/file/91726/Ergonomia-pracy-z-komputerem-J-Kaminska.pdf",
  },
  {
    id: "eyes-near-far",
    category: "eyes",
    length: "short",
    source: "https://www.hse.gov.uk/contact/faqs/vdubreaks.htm",
  },
  {
    id: "eyes-screen-free",
    category: "eyes",
    length: "any",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7044622/",
  },
  {
    id: "stretch-shoulder-rolls",
    category: "stretch",
    length: "any",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-shoulder-shrug",
    category: "stretch",
    length: "short",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-neck-turn",
    category: "stretch",
    length: "any",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-neck-tilt",
    category: "stretch",
    length: "any",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-chin-glide",
    category: "stretch",
    length: "short",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-chest-open",
    category: "stretch",
    length: "any",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-upper-twist",
    category: "stretch",
    length: "any",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-side-reach",
    category: "stretch",
    length: "any",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-wrists",
    category: "stretch",
    length: "short",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-fingers",
    category: "stretch",
    length: "short",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-prayer",
    category: "stretch",
    length: "short",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-ankles",
    category: "stretch",
    length: "any",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-standing-back",
    category: "stretch",
    length: "any",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-routine",
    category: "stretch",
    length: "long",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "move-walk-5",
    category: "move",
    length: "any",
    source: "https://www.cuimc.columbia.edu/news/rx-prolonged-sitting-five-minute-stroll-every-half-hour",
  },
  {
    id: "move-stairs",
    category: "move",
    length: "any",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-march-seated",
    category: "move",
    length: "short",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "move-march-standing",
    category: "move",
    length: "short",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-calf-raises",
    category: "move",
    length: "short",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-sit-to-stand",
    category: "move",
    length: "short",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "move-one-song",
    category: "move",
    length: "any",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-chores",
    category: "move",
    length: "any",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "hydrate-glass",
    category: "hydrate",
    length: "any",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
  {
    id: "hydrate-refill",
    category: "hydrate",
    length: "short",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
  {
    id: "hydrate-in-reach",
    category: "hydrate",
    length: "short",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
  {
    id: "breathe-count-5",
    category: "breathe",
    length: "any",
    source: "https://www.nhs.uk/mental-health/self-help/guides-tools-and-activities/breathing-exercises-for-stress/",
  },
  {
    id: "breathe-belly-hand",
    category: "breathe",
    length: "short",
    source: "https://www.nhs.uk/mental-health/self-help/guides-tools-and-activities/breathing-exercises-for-stress/",
  },
  {
    id: "breathe-long-exhale",
    category: "breathe",
    length: "short",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC9873947/",
  },
  {
    id: "breathe-open-window",
    category: "breathe",
    length: "short",
    source: "https://www.nhs.uk/mental-health/self-help/guides-tools-and-activities/breathing-exercises-for-stress/",
  },
  {
    id: "breathe-muscle-release",
    category: "breathe",
    length: "long",
    source: "https://doi.org/10.1037/ocp0000083",
  },
  {
    id: "posture-reset",
    category: "posture",
    length: "any",
    source: "https://ergo.human.cornell.edu/DEA6510/dea6512k/ergo12tips.html",
  },
  {
    id: "posture-screen-check",
    category: "posture",
    length: "short",
    source: "https://www.osha.gov/etools/computer-workstations/components/monitors",
  },
  {
    id: "posture-glare-check",
    category: "posture",
    length: "short",
    source: "https://www.osha.gov/etools/computer-workstations/workstation-environment",
  },
  {
    id: "posture-shift",
    category: "posture",
    length: "short",
    source: "https://www.osha.gov/etools/computer-workstations/work-process",
  },
  {
    id: "outside-step-out",
    category: "outside",
    length: "short",
    source: "https://doi.org/10.1016/j.jenvp.2015.04.003",
  },
  {
    id: "outside-sky",
    category: "outside",
    length: "any",
    source: "https://doi.org/10.1016/j.jenvp.2015.04.003",
  },
  {
    id: "outside-walk-15",
    category: "outside",
    length: "long",
    source: "https://doi.org/10.1037/ocp0000083",
  },
  {
    id: "outside-walk-noticing",
    category: "outside",
    length: "long",
    source: "https://doi.org/10.1037/ocp0000083",
  },
  {
    id: "social-chat",
    category: "social",
    length: "any",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "social-message",
    category: "social",
    length: "short",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "social-walk-together",
    category: "social",
    length: "long",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "social-pet",
    category: "social",
    length: "any",
    source: null,
  },
  {
    id: "mind-do-nothing",
    category: "mind",
    length: "short",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "mind-one-song",
    category: "mind",
    length: "any",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "mind-5-4-3",
    category: "mind",
    length: "short",
    source: null,
  },
  {
    id: "mind-wander",
    category: "mind",
    length: "any",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "mind-park-the-task",
    category: "mind",
    length: "short",
    source: null,
  },
  {
    id: "mind-three-good",
    category: "mind",
    length: "any",
    source: null,
  },
  {
    id: "mind-meal-away",
    category: "mind",
    length: "long",
    source: "https://doi.org/10.5465/amj.2011.1072",
  },
  {
    id: "mind-your-choice",
    category: "mind",
    length: "long",
    source: "https://doi.org/10.5465/amj.2011.1072",
  },
  {
    id: "mind-lie-down",
    category: "mind",
    length: "long",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "tidy-desk",
    category: "tidy",
    length: "short",
    source: null,
  },
  {
    id: "tidy-air",
    category: "tidy",
    length: "any",
    source: null,
  },
  {
    id: "tidy-light",
    category: "tidy",
    length: "short",
    source: "https://www.aao.org/eye-health/tips-prevention/digital-devices-your-eyes",
  },
];

/** Good working practices for the idle Skupienie page (one per day). */
export const WORKING_TIPS: readonly WorkingTip[] = [
  {
    id: "gp-screen-position",
    source: "https://www.osha.gov/etools/computer-workstations/components/monitors",
  },
  {
    id: "gp-light-and-glare",
    source: "https://www.osha.gov/etools/computer-workstations/workstation-environment",
  },
  {
    id: "gp-sitting",
    source: "https://ergo.human.cornell.edu/DEA6510/dea6512k/ergo12tips.html",
  },
  {
    id: "gp-short-often",
    source: "https://www.hse.gov.uk/msd/dse/work-routine.htm",
  },
  {
    id: "gp-20-20-20",
    source: "https://www.aao.org/eye-health/diseases/what-is-eye-strain",
  },
  {
    id: "gp-change-position",
    source: "https://www.ciop.pl/CIOPPortalWAR/file/91726/Ergonomia-pracy-z-komputerem-J-Kaminska.pdf",
  },
  {
    id: "gp-batch-messages",
    source: "https://doi.org/10.1016/j.chb.2014.11.005",
  },
  {
    id: "gp-single-task",
    source: "https://www.apa.org/topics/research/multitasking",
  },
  {
    id: "gp-phone-away",
    source: "https://www.ics.uci.edu/~gmark/chi08-mark.pdf",
  },
  {
    id: "gp-water-in-reach",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
];

/** Source of the note shown under every stretch tip (`strings.stretchFootnote`). */
export const STRETCH_FOOTNOTE_SOURCE = "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html";

/** Polish labour law (screen work breaks): shown only in the Polish UI (`showsPolishLegalNote`), never translated. */
export const PL_LEGAL_NOTE = {
  id: "pl-law-note",
  text: "Pracując przy komputerze co najmniej połowę dnia, masz prawo do 5 minut przerwy po każdej godzinie, wliczanej do czasu pracy.",
  source: "https://isap.sejm.gov.pl/isap.nsf/DocDetails.xsp?id=WDU19981480973",
} as const;

/** The text of a break tip in the language in use. */
export function breakTipText(tip: BreakTip): string {
  return strings.tips[tip.id];
}

/** The text of a good working practice in the language in use. */
export function workingTipText(tip: WorkingTip): string {
  return strings.workingTips[tip.id];
}

/** The Polish labour-law note applies to Polish users only, so it is shown only in the Polish UI. Reactive. */
export function showsPolishLegalNote(): boolean {
  return language() === "pl";
}

// ---------------------------------------------------------------------------
// Deterministic pick
// ---------------------------------------------------------------------------

/**
 * Breaks shorter than this get `short` (and `any`) tips, longer ones `long` (and `any`) tips. This only sorts the
 * tip catalogue; the break length itself always comes from the session's timing profile (snapshot / popup).
 */
export const SHORT_BREAK_LIMIT_SECS = 10 * 60;

const MS_PER_DAY = 86_400_000;

/** Tips that fit a break of `breakSecs`, in catalogue order. */
export function tipsForBreak(breakSecs: number): BreakTip[] {
  const wanted: TipLength = breakSecs < SHORT_BREAK_LIMIT_SECS ? "short" : "long";
  return BREAK_TIPS.filter((t) => t.length === "any" || t.length === wanted);
}

/** FNV-1a, 32 bit. */
function hashString(text: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

/** mulberry32: small seeded PRNG, 0 <= x < 1. */
function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const orderCache = new Map<string, BreakTip[]>();

/** The session's own shuffled order of the fitting tips (same in every window for the same session). */
function sessionOrder(sessionId: string | null, breakSecs: number): BreakTip[] {
  const pool = tipsForBreak(breakSecs);
  const key = `${sessionId ?? ""}|${pool.length}|${breakSecs < SHORT_BREAK_LIMIT_SECS ? "s" : "l"}`;
  const cached = orderCache.get(key);
  if (cached) return cached;
  const random = seededRandom(hashString(sessionId ?? ""));
  const order = pool.slice();
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    const a = order[i];
    const b = order[j];
    if (a !== undefined && b !== undefined) {
      order[i] = b;
      order[j] = a;
    }
  }
  if (orderCache.size > 16) orderCache.clear();
  orderCache.set(key, order);
  return order;
}

/**
 * The tip for break number `cycle` (1-based, like `SessionSnapshot.cycle` during that break) of a session.
 * Consecutive breaks walk the session's shuffled order, so a break never repeats the previous break's tip and
 * no tip comes back before the whole pool was shown. `offset` steps through it ("Inna propozycja").
 */
export function pickBreakTip(sessionId: string | null, cycle: number, breakSecs: number, offset = 0): BreakTip | null {
  const order = sessionOrder(sessionId, breakSecs);
  const n = order.length;
  if (n === 0) return null;
  const position = Math.max(1, Math.trunc(Number.isFinite(cycle) ? cycle : 1)) - 1 + Math.trunc(offset);
  return order[((position % n) + n) % n] ?? null;
}

/** One working tip per calendar day (local date), the same all day. */
export function workingTipOfDay(date: Date = new Date()): WorkingTip | null {
  const day = workingTipDay(date);
  return WORKING_TIPS[((day % WORKING_TIPS.length) + WORKING_TIPS.length) % WORKING_TIPS.length] ?? null;
}

/** The local calendar day of `date` (a new day brings the tip of that day back). */
export function workingTipDay(date: Date = new Date()): number {
  return Math.floor(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) / MS_PER_DAY);
}

/**
 * "Inna porada": a random working tip that is neither `current` nor one of `seen` (the tips shown in this round).
 * When every tip has been shown, a new round starts (only `current` is left out), so a tip never comes twice in a row
 * and all of them come before one repeats. Returns the tip and the round after it; null when there is no other tip.
 */
export function nextWorkingTip(
  current: WorkingTipId | null,
  seen: ReadonlySet<WorkingTipId>,
  random: () => number = Math.random,
): { tip: WorkingTip; seen: Set<WorkingTipId> } | null {
  let round = new Set(seen);
  let pool = WORKING_TIPS.filter((t) => t.id !== current && !round.has(t.id));
  if (pool.length === 0) {
    round = new Set(current === null ? [] : [current]);
    pool = WORKING_TIPS.filter((t) => t.id !== current);
  }
  const tip = pool[Math.min(pool.length - 1, Math.floor(random() * pool.length))];
  if (!tip) return null;
  round.add(tip.id);
  return { tip, seen: round };
}

/** `https://www.nhs.uk/…` → `nhs.uk` (for the "Źródło" link). */
export function sourceHost(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
