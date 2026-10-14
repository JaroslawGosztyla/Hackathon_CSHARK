// Presentation showcase data (Developer page "Presentation mode", `--presentation`; ADR-019, beta ADR-022): a believable
// history for a short live demo, built in memory from one seed per activation (random-looking, stable while presenting)
// and never saved, sent or shown to the shell.
//
// - `buildShowcase`: the last session (an ~8-hour workday on the latest work day that has already ended, 45 / 15
//   cycles with a lunch pause), the history list (about three weeks, newest first) and every generated session
//   (ten weeks: the weekly stats look back eight weeks from the current one).
// - `showcaseWeeklyStats`: the weekly stats and heatmaps of any week, aggregated from those sessions the way focus-core
//   `stats::WeekAggregator` does (sessions, breaks and apps on the day a session started; focus spans split by local
//   hour), so the week picker browses the same history the list shows.
// - `SHOWCASE_LABELS`: the app labels of the sample (a memory-only copy, never `Config.appCategories`).
//
// Ids start with `dev-sample-` and links with `SAMPLE_URL_PREFIX` like every developer sample (never opened). App names
// and window titles are generic: no real people, no real accounts.
import { showcaseTexts, type ShowcaseTexts } from "$lib/i18n/showcase";
import type {
  AppCategory,
  AppUsage,
  CategoryTotals,
  DayStats,
  MicroStep,
  SessionGoal,
  SessionSummary,
  TimeSpan,
  TitleUsage,
  WeekApp,
  Weekday,
  WeeklyStats,
} from "$lib/types";
import { dateOfDay, dayOfDate, DAYS_PER_WEEK, HOURS_PER_DAY, isoWeekOfDay, localDayOf, mondayOfDay, weekdayIndex } from "$lib/week";
import { SAMPLE_ID_PREFIX, SAMPLE_URL_PREFIX, sampleWork } from "./samples";

const SECOND_MS = 1000;
const MINUTE_SECS = 60;
const MINUTE_MS = MINUTE_SECS * SECOND_MS;
const HOUR_MS = 60 * MINUTE_MS;

/** Focus block and break of the sample sessions (the app's defaults). */
const FOCUS_MINUTES = 45;
const BREAK_MINUTES = 15;
/** Weeks of generated sessions, the current one included (the weekly stats sum 8 weeks; one more for the picker). */
const HISTORY_WEEKS = 10;
/** The history list shows the sessions of this many days (about three weeks). */
const LIST_DAYS = 21;
/** The beta (sparks, small steps) has been on this many days: older sessions have no sparks and no steps. */
const BETA_DAYS = 30;
/** Chance of a light session on a weekend day. */
const WEEKEND_SESSION = 0.3;
/** No day off in this many days before the last workday: the rewards streak (12 work days) covers them. */
const NO_DAY_OFF_DAYS = 21;
/** Mirrors focus-core `stats::RECENT_WEEKS` and `stats::TOP_APPS_PER_WEEK`. */
const RECENT_WEEKS = 8;
const TOP_APPS = 10;
/** The last session: 09:02 → 17:11 (shown once that end is this long in the past; otherwise the work day before). */
const LAST_START = { hour: 9, minute: 2 } as const;
const LAST_ENDED_MARGIN_MINUTES = 10;
/** Its plan: 7 full blocks with breaks, a 30-minute lunch pause after the 4th break and a last block of 39 minutes. */
const LAST_FULL_BLOCKS = 7;
const LAST_LUNCH_AFTER_BLOCK = 4;
const LAST_LUNCH_MINUTES = 30;
const LAST_TAIL_MINUTES = 39;
const LAST_SPARKS = 418;
/** Sparks of a sample session: focus minutes (at most a day's cap) + full blocks + breaks + the start. */
const SPARK_MINUTE_CAP = 300;
const SPARKS_PER_BLOCK = 10;
const SPARKS_PER_BREAK = 10;
const SPARKS_START = 12;
/** Share of the focus without a foreground app: idle and unknown (small). */
const IDLE_SHARE = [0.012, 0.028] as const;
const UNKNOWN_SHARE = [0.003, 0.009] as const;
/** Saturday (`weekdayIndex`: 0 = Monday). */
const SATURDAY = 5;
const WEEKDAY_NAMES: readonly Weekday[] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

// ---------------------------------------------------------------------------
// Apps of the sample: process (as the tracker reports it), weight of the app time, label, window titles
// ---------------------------------------------------------------------------

interface AppSpec {
  process: string;
  /** Share of the app time in an average session (before the per-session variation). */
  weight: number;
  /** Chance that a session used the app at all. */
  presence: number;
  category: AppCategory | null;
  /** Window titles and their share of the app's time. */
  titles: readonly (readonly [title: string, weight: number])[];
}

const APPS: readonly AppSpec[] = [
  {
    process: "Code.exe",
    weight: 0.39,
    presence: 1,
    category: "productive",
    titles: [
      ["rewards.rs — FocusMode — Visual Studio Code", 0.3],
      ["showcase.ts — FocusMode — Visual Studio Code", 0.22],
      ["+page.svelte — FocusMode — Visual Studio Code", 0.18],
      ["session.rs — FocusMode — Visual Studio Code", 0.14],
      ["README.md — FocusMode — Visual Studio Code", 0.08],
    ],
  },
  {
    process: "chrome.exe",
    weight: 0.13,
    presence: 1,
    category: "neutral",
    titles: [
      ["Pull request #42 · GitHub — Google Chrome", 0.34],
      ["Svelte 5 runes · Docs — Google Chrome", 0.24],
      ["Tauri 2 guides — Google Chrome", 0.2],
      ["Issues · focusmode — Google Chrome", 0.12],
    ],
  },
  {
    process: "ms-teams.exe",
    weight: 0.11,
    presence: 0.95,
    category: "neutral",
    titles: [
      ["Daily stand-up | Microsoft Teams", 0.46],
      ["Chat | Hackathon team | Microsoft Teams", 0.34],
      ["Calendar | Microsoft Teams", 0.12],
    ],
  },
  {
    process: "Claude.exe",
    weight: 0.09,
    presence: 0.9,
    category: "productive",
    titles: [["Claude", 1]],
  },
  {
    process: "WindowsTerminal.exe",
    weight: 0.08,
    presence: 0.9,
    category: "productive",
    titles: [
      ["cargo test --workspace", 0.48],
      ["npm run check", 0.3],
      ["PowerShell", 0.16],
    ],
  },
  {
    process: "Figma.exe",
    weight: 0.06,
    presence: 0.55,
    category: "productive",
    titles: [
      ["Rewards screens – Figma", 0.6],
      ["Pitch deck – Figma", 0.32],
    ],
  },
  {
    process: "opera.exe",
    weight: 0.04,
    presence: 0.6,
    category: "distracting",
    titles: [
      ["YouTube — Opera GX", 0.52],
      ["r/rust — Reddit — Opera GX", 0.3],
      ["Twitch — Opera GX", 0.14],
    ],
  },
  {
    process: "OUTLOOK.EXE",
    weight: 0.03,
    presence: 0.85,
    category: "neutral",
    titles: [
      ["Inbox — Outlook", 0.7],
      ["Calendar — Outlook", 0.24],
    ],
  },
  {
    process: "Discord.exe",
    weight: 0.025,
    presence: 0.7,
    category: "distracting",
    titles: [
      ["#hackathon — Discord", 0.66],
      ["#general — Discord", 0.28],
    ],
  },
  {
    process: "explorer.exe",
    weight: 0.015,
    presence: 0.6,
    category: null,
    titles: [
      ["Downloads — File Explorer", 0.55],
      ["FocusMode — File Explorer", 0.4],
    ],
  },
  {
    process: "Spotify.exe",
    weight: 0.01,
    presence: 0.5,
    category: "distracting",
    titles: [["Spotify", 1]],
  },
];

/** The labels the sample starts with (lowercase exe → label); the chips change only this copy in memory. */
export const SHOWCASE_LABELS: Readonly<Record<string, AppCategory>> = Object.fromEntries(
  APPS.filter((app) => app.category !== null).map((app) => [app.process.toLowerCase(), app.category as AppCategory]),
);

// ---------------------------------------------------------------------------
// Seeded randomness (mulberry32): one stream per seed and day, so every day is stable on its own
// ---------------------------------------------------------------------------

type Rng = () => number;

function rngOf(seed: number): Rng {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/** A stable seed for one day of one showcase (any odd multipliers). */
function daySeed(seed: number, day: number, salt = 0): number {
  return (Math.imul(seed | 0, 0x9e3779b1) ^ Math.imul(day + salt * 7919, 0x85ebca6b)) >>> 0;
}

const between = (rng: Rng, [low, high]: readonly [number, number]) => low + (high - low) * rng();
const intBetween = (rng: Rng, low: number, high: number) => low + Math.floor(rng() * (high - low + 1));

/** The seed of an activation: its time, so every presentation looks a little different but stays stable while on. */
export function showcaseSeed(now: number): number {
  return Math.floor(now / SECOND_MS) >>> 0;
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

/** One part of a session plan, in minutes: running focus, a break, or a pause (neither). */
type Part = { focus: number } | { break: number } | { pause: number };

interface SessionDraft {
  startMs: number;
  plan: readonly Part[];
  goal: SessionGoal | null;
  goalDone: boolean | null;
  steps: MicroStep[];
}

/** Local time `hour:minute` (fractional minutes allowed) of local day number `day`, as Unix ms. */
function localTime(day: number, hour: number, minute: number): number {
  const date = new Date(day * 86_400_000);
  return new Date(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate(), hour, 0, 0, 0).getTime() + minute * MINUTE_MS;
}

/** A session of `blocks` focus blocks (the last one `tail` minutes) with breaks, and a pause after break `pauseAfter`. */
function cyclePlan(blocks: number, tail: number, pauseAfter: number | null, pauseMinutes: number): Part[] {
  const plan: Part[] = [];
  for (let i = 1; i <= blocks; i++) {
    plan.push({ focus: FOCUS_MINUTES });
    plan.push({ break: BREAK_MINUTES });
    if (pauseAfter === i) plan.push({ pause: pauseMinutes });
  }
  if (tail > 0) plan.push({ focus: tail });
  else plan.pop();
  return plan;
}

function usageOf(
  focusSecs: number,
  rng: Rng,
  flavor: Partial<Record<string, number>>,
  everyApp: boolean,
): {
  apps: AppUsage[];
  idleSecs: number;
  unknownSecs: number;
} {
  const idleSecs = Math.round(focusSecs * between(rng, IDLE_SHARE));
  const unknownSecs = Math.round(focusSecs * between(rng, UNKNOWN_SHARE));
  const appSecs = focusSecs - idleSecs - unknownSecs;
  const picked = APPS.filter((app) => everyApp || app.presence >= 1 || rng() < app.presence).map((app) => ({
    app,
    weight: app.weight * (0.75 + 0.5 * rng()) * (flavor[app.process] ?? 1),
  }));
  const total = picked.reduce((sum, p) => sum + p.weight, 0);
  const apps: AppUsage[] = picked
    .map(({ app, weight }) => {
      const secs = Math.round((appSecs * weight) / total);
      return { process: app.process, secs, titles: titlesOf(app, secs, rng) };
    })
    .filter((app) => app.secs > 0);
  // Rounding leftovers go to the biggest app, so apps + idle + unknown = the focus exactly.
  const drift = appSecs - apps.reduce((sum, a) => sum + a.secs, 0);
  apps.sort((a, b) => b.secs - a.secs || a.process.toLowerCase().localeCompare(b.process.toLowerCase()));
  if (apps[0]) apps[0].secs += drift;
  return { apps, idleSecs, unknownSecs };
}

/** The app's window titles (longest first, at most 5) covering most of its time. */
function titlesOf(app: AppSpec, secs: number, rng: Rng): TitleUsage[] {
  const covered = secs * (0.86 + 0.12 * rng());
  const weights = app.titles.map(([title, weight]) => ({ title, weight: weight * (0.7 + 0.6 * rng()) }));
  const total = weights.reduce((sum, w) => sum + w.weight, 0);
  return weights
    .map(({ title, weight }) => ({ title, secs: Math.round((covered * weight) / total) }))
    .filter((t) => t.secs > 0)
    .sort((a, b) => b.secs - a.secs)
    .slice(0, 5);
}

/** `beta`: the session ran with the beta on (sparks, small steps); `everyApp`: every sample app was used. */
function summaryOf(
  draft: SessionDraft,
  index: number,
  rng: Rng,
  flavor: Partial<Record<string, number>>,
  { beta, everyApp }: { beta: boolean; everyApp: boolean },
): SessionSummary {
  const focusSpans: TimeSpan[] = [];
  let at = draft.startMs;
  let focusSecs = 0;
  let breakSecs = 0;
  let cycles = 0;
  let fullBlocks = 0;
  for (const part of draft.plan) {
    if ("focus" in part) {
      const end = at + part.focus * MINUTE_MS;
      focusSpans.push({ startMs: at, endMs: end });
      focusSecs += part.focus * MINUTE_SECS;
      if (part.focus >= FOCUS_MINUTES) fullBlocks++;
      at = end;
    } else if ("break" in part) {
      breakSecs += part.break * MINUTE_SECS;
      cycles++;
      at += part.break * MINUTE_MS;
    } else {
      at += part.pause * MINUTE_MS;
    }
  }
  const { apps, idleSecs, unknownSecs } = usageOf(focusSecs, rng, flavor, everyApp);
  const sparks =
    Math.min(Math.round(focusSecs / MINUTE_SECS), SPARK_MINUTE_CAP) +
    SPARKS_PER_BLOCK * fullBlocks +
    SPARKS_PER_BREAK * cycles +
    SPARKS_START;
  return {
    sessionId: `${SAMPLE_ID_PREFIX}showcase-${index}`,
    startedAtMs: draft.startMs,
    endedAtMs: at,
    focusedSecs: focusSecs,
    breakSecs,
    cyclesCompleted: cycles,
    usage: { apps, idleSecs, unknownSecs, trackedSecs: focusSecs },
    demo: false,
    goal: draft.goal,
    goalDone: draft.goal ? draft.goalDone : null,
    steps: beta ? draft.steps : [],
    sparks: beta ? sparks : null,
    focusSpans,
  };
}

/** The local day and start of the last sample workday: today once 17:11 (+10 min) is past, else the work day before. */
function lastWorkday(now: number): number {
  const minutes = LAST_FULL_BLOCKS * (FOCUS_MINUTES + BREAK_MINUTES) + LAST_LUNCH_MINUTES + LAST_TAIL_MINUTES;
  let day = localDayOf(now);
  for (;;) {
    const ends = localTime(day, LAST_START.hour, LAST_START.minute + minutes + LAST_ENDED_MARGIN_MINUTES);
    if (weekdayIndex(day) < SATURDAY && ends <= now) return day;
    day--;
  }
}

/** The goals of older sessions: the "Co dalej?" sample items. */
function goalPool(now: number): SessionGoal[] {
  return sampleWork(now).items.map((item) => ({ source: item.source, key: item.key, title: item.title, url: item.url }));
}

/**
 * One generated day (not the last workday): a full day, a split day, a short day, a light weekend or nothing (a work
 * day off only when `dayOffAllowed`).
 */
function dayDrafts(
  day: number,
  seed: number,
  texts: ShowcaseTexts,
  goals: readonly SessionGoal[],
  dayOffAllowed: boolean,
): SessionDraft[] {
  const rng = rngOf(daySeed(seed, day));
  const weekend = weekdayIndex(day) >= SATURDAY;
  const extras = (): Pick<SessionDraft, "goal" | "goalDone" | "steps"> => {
    const withGoal = rng() < 0.45;
    const done = rng();
    const stepCount = rng() < 0.4 ? intBetween(rng, 2, 3) : 0;
    const first = intBetween(rng, 0, texts.moreSteps.length - 1);
    return {
      goal: withGoal ? (goals[intBetween(rng, 0, goals.length - 1)] ?? null) : null,
      goalDone: done < 0.7 ? true : done < 0.9 ? false : null,
      steps: Array.from({ length: stepCount }, (_, i) => ({
        id: i + 1,
        text: texts.moreSteps[(first + i) % texts.moreSteps.length] ?? texts.moreSteps[0],
        done: i < stepCount - 1 || rng() < 0.6,
      })),
    };
  };
  if (weekend) {
    if (rng() >= WEEKEND_SESSION) return [];
    const start = localTime(day, intBetween(rng, 10, 16), intBetween(rng, 0, 50));
    return [{ startMs: start, plan: cyclePlan(intBetween(rng, 1, 2), intBetween(rng, 0, 30), null, 0), ...extras() }];
  }
  const kind = rng();
  if (kind < 0.04 && dayOffAllowed) return []; // a day off
  const morning = localTime(day, 8, intBetween(rng, 30, 105));
  if (kind < 0.14) {
    // A short day (meetings, a doctor, a Friday): one shorter session.
    return [{ startMs: morning, plan: cyclePlan(intBetween(rng, 2, 4), intBetween(rng, 0, 35), null, 0), ...extras() }];
  }
  if (kind < 0.42) {
    // Two sessions around lunch.
    const amBlocks = intBetween(rng, 3, 4);
    const am = cyclePlan(amBlocks, intBetween(rng, 15, 40), null, 0);
    const amMinutes = am.reduce((sum, p) => sum + ("focus" in p ? p.focus : "break" in p ? p.break : p.pause), 0);
    const pm = morning + (amMinutes + intBetween(rng, 40, 70)) * MINUTE_MS;
    return [
      { startMs: morning, plan: am, ...extras() },
      { startMs: pm, plan: cyclePlan(intBetween(rng, 3, 4), intBetween(rng, 0, 40), null, 0), ...extras() },
    ];
  }
  // One long session with a lunch pause.
  const blocks = intBetween(rng, 6, 8);
  return [
    {
      startMs: morning,
      plan: cyclePlan(blocks, intBetween(rng, 0, 40), intBetween(rng, 3, 4), intBetween(rng, 25, 50)),
      ...extras(),
    },
  ];
}

/** App time mix of one day (meeting days, design days…), so the sessions do not all look alike. */
function flavorOf(day: number, seed: number): Partial<Record<string, number>> {
  const rng = rngOf(daySeed(seed, day, 1));
  const kind = rng();
  if (kind < 0.2) return { "ms-teams.exe": 2.2, "OUTLOOK.EXE": 1.6, "Code.exe": 0.7 };
  if (kind < 0.32) return { "Figma.exe": 3, "Code.exe": 0.8 };
  if (kind < 0.45) return { "Claude.exe": 1.6, "WindowsTerminal.exe": 1.3 };
  return {};
}

export interface Showcase {
  /** The last session (the sample workday). */
  last: SessionSummary;
  /** The history list: the sessions of the last `LIST_DAYS` days, newest first. */
  history: SessionSummary[];
  /** Every generated session (ten weeks), newest first: the weekly stats are built from these. */
  all: SessionSummary[];
}

/** The whole showcase history for `now` (the activation time) and `seed`, with texts in the given UI language. */
export function buildShowcase(now: number, seed: number, texts: ShowcaseTexts = showcaseTexts()): Showcase {
  const lastDay = lastWorkday(now);
  const firstDay = mondayOfDay(localDayOf(now)) - (HISTORY_WEEKS - 1) * DAYS_PER_WEEK;
  const goals = goalPool(now);
  const drafts: { draft: SessionDraft; day: number }[] = [];
  for (let day = firstDay; day < lastDay; day++) {
    for (const draft of dayDrafts(day, seed, texts, goals, day < lastDay - NO_DAY_OFF_DAYS)) drafts.push({ draft, day });
  }
  const betaFrom = localDayOf(now) - BETA_DAYS;
  let index = 0;
  const older = drafts.map(({ draft, day }) =>
    summaryOf(draft, ++index, rngOf(daySeed(seed, day, 2 + index)), flavorOf(day, seed), {
      beta: day >= betaFrom,
      everyApp: false,
    }),
  );

  const lastDraft: SessionDraft = {
    startMs: localTime(lastDay, LAST_START.hour, LAST_START.minute),
    plan: cyclePlan(LAST_FULL_BLOCKS, LAST_TAIL_MINUTES, LAST_LUNCH_AFTER_BLOCK, LAST_LUNCH_MINUTES),
    goal: { source: "github", key: "#42", title: texts.goal, url: `${SAMPLE_URL_PREFIX}github/42` },
    goalDone: true,
    steps: texts.daySteps.map((text, i) => ({ id: i + 1, text, done: true })),
  };
  const last: SessionSummary = {
    ...summaryOf(lastDraft, ++index, rngOf(daySeed(seed, lastDay, 3)), {}, { beta: true, everyApp: true }),
    sessionId: `${SAMPLE_ID_PREFIX}showcase-last`,
    sparks: LAST_SPARKS,
  };

  const all = [last, ...older].sort((a, b) => b.endedAtMs - a.endedAtMs);
  const listFrom = localTime(localDayOf(now) - LIST_DAYS, 0, 0);
  return { last, history: all.filter((s) => s.startedAtMs >= listFrom), all };
}

// ---------------------------------------------------------------------------
// Weekly stats (mirrors focus-core `stats::WeekAggregator`)
// ---------------------------------------------------------------------------

/** The weekly stats of the week that contains `date` (`YYYY-MM-DD`), from `sessions`, with `labels` as app labels. */
export function showcaseWeeklyStats(
  date: string,
  now: number,
  labels: Readonly<Record<string, AppCategory>>,
  sessions: readonly SessionSummary[],
): WeeklyStats {
  const today = localDayOf(now);
  const weekStart = mondayOfDay(dayOfDate(date) ?? today);
  const weekEnd = weekStart + DAYS_PER_WEEK;
  const recentStart = weekEnd - DAYS_PER_WEEK * RECENT_WEEKS;
  const grid = () => Array.from({ length: DAYS_PER_WEEK }, () => new Array<number>(HOURS_PER_DAY).fill(0));
  const weekMs = grid();
  const recentMs = grid();
  const dayBreakSecs = new Array<number>(DAYS_PER_WEEK).fill(0);
  const daySessions = new Array<number>(DAYS_PER_WEEK).fill(0);
  let breaksTaken = 0;
  let firstDay: number | null = null;
  const apps = new Map<string, { name: string; secs: number }>();

  for (const s of sessions) {
    if (s.demo) continue;
    const startDay = localDayOf(s.startedAtMs);
    firstDay = firstDay === null ? startDay : Math.min(firstDay, startDay);
    if (startDay >= weekStart && startDay < weekEnd) {
      const i = startDay - weekStart;
      daySessions[i] += 1;
      dayBreakSecs[i] += s.breakSecs;
      breaksTaken += s.cyclesCompleted;
      for (const app of s.usage.apps) {
        const name = app.process.trim();
        if (name.length === 0 || app.secs === 0) continue;
        const entry = apps.get(name.toLowerCase()) ?? { name, secs: 0 };
        entry.secs += app.secs;
        apps.set(name.toLowerCase(), entry);
      }
    }
    for (const span of s.focusSpans ?? []) {
      let at = span.startMs;
      while (at < span.endMs) {
        const local = new Date(at);
        const nextHour = new Date(local.getFullYear(), local.getMonth(), local.getDate(), local.getHours() + 1).getTime();
        const chunkEnd = Math.min(span.endMs, nextHour);
        const day = localDayOf(at);
        if (day >= recentStart && day < weekEnd) {
          const weekday = weekdayIndex(day);
          const hour = local.getHours();
          recentMs[weekday][hour] += chunkEnd - at;
          if (day >= weekStart) weekMs[weekday][hour] += chunkEnd - at;
        }
        at = chunkEnd;
      }
    }
  }

  const secs = (ms: number) => Math.round(ms / SECOND_MS);
  const days: DayStats[] = weekMs.map((hours, i) => ({
    date: dateOfDay(weekStart + i),
    weekday: WEEKDAY_NAMES[i] ?? "mon",
    focusSecs: secs(hours.reduce((sum, ms) => sum + ms, 0)),
    breakSecs: dayBreakSecs[i] ?? 0,
    sessions: daySessions[i] ?? 0,
  }));
  const sum = (pick: (d: DayStats) => number) => days.reduce((total, d) => total + pick(d), 0);
  const best = days.reduce<DayStats | null>((top, d) => (d.focusSecs > 0 && d.focusSecs > (top?.focusSecs ?? 0) ? d : top), null);

  const categories: CategoryTotals = { productiveSecs: 0, neutralSecs: 0, distractingSecs: 0, unlabelledSecs: 0 };
  const weekApps: WeekApp[] = [...apps.entries()].map(([key, app]) => {
    const category = labels[key] ?? null;
    if (category === "productive") categories.productiveSecs += app.secs;
    else if (category === "neutral") categories.neutralSecs += app.secs;
    else if (category === "distracting") categories.distractingSecs += app.secs;
    else categories.unlabelledSecs += app.secs;
    return { process: app.name, secs: app.secs, category };
  });
  weekApps.sort((a, b) => b.secs - a.secs || a.process.toLowerCase().localeCompare(b.process.toLowerCase()));

  const iso = isoWeekOfDay(weekStart);
  return {
    revision: 0,
    weekStart: dateOfDay(weekStart),
    isoYear: iso.year,
    isoWeek: iso.week,
    currentWeek: weekStart === mondayOfDay(today),
    firstWeekStart: firstDay === null ? null : dateOfDay(mondayOfDay(firstDay)),
    historyOff: false,
    days,
    focusSecs: sum((d) => d.focusSecs),
    breakSecs: sum((d) => d.breakSecs),
    breaksTaken,
    sessions: sum((d) => d.sessions),
    bestDay: best?.weekday ?? null,
    heatmap: weekMs.map((row) => row.map(secs)),
    recentHeatmap: recentMs.map((row) => row.map(secs)),
    recentWeeks: RECENT_WEEKS,
    topApps: weekApps.slice(0, TOP_APPS),
    categories,
    estimated: false,
  };
}

/** The two steps planned for the live demo session (Start then pays the plan bonus; one tick shows the step float). */
export function showcaseDemoSteps(texts: ShowcaseTexts = showcaseTexts()): { id: null; text: string }[] {
  return texts.demoSteps.map((text) => ({ id: null, text }));
}
