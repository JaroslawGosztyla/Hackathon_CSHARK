// Developer-mode sample data (ADR-019): realistic content for screenshots, built at click time and kept only in memory
// (a sample layer in the stores, cleared by "Clear samples" and on restart). Nothing here is saved, sent or shown to
// the shell, and no link of a sample is ever opened. Ids start with `dev-sample-` (the shell treats meeting choices for
// such ids as no-ops); links start with `SAMPLE_URL_PREFIX`.
import type {
  AppUsage,
  DayCell,
  DayMark,
  MicroStep,
  MomentKind,
  RewardKind,
  RewardLine,
  RewardMoment,
  RewardsStatus,
  StickerId,
  CalendarStatus,
  Meeting,
  MeetingConflict,
  MeetingHoldView,
  Nudge,
  SessionGoal,
  SessionSummary,
  UsageReport,
  WorkItem,
  WorkSource,
  WorkSourceStatus,
  WorkStatus,
} from "$lib/types";
import { sampleTexts } from "$lib/i18n/dev";

/** Prefix of every sample id (meetings, sessions, work items); the shell's no-op rule for meetings matches it. */
export const SAMPLE_ID_PREFIX = "dev-sample-";
/** Prefix of every sample link (the reserved `.invalid` domain); `isSampleUrl` links are never opened. */
export const SAMPLE_URL_PREFIX = "https://dev-sample.invalid/";

export function isSampleId(id: string): boolean {
  return id.startsWith(SAMPLE_ID_PREFIX);
}

export function isSampleUrl(url: string): boolean {
  return url.trim().toLowerCase().startsWith(SAMPLE_URL_PREFIX);
}

const SECOND_MS = 1000;
const MINUTE_SECS = 60;
const MINUTE_MS = MINUTE_SECS * SECOND_MS;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

// ---------------------------------------------------------------------------
// Session goal, meeting hold, blocked-site nudge
// ---------------------------------------------------------------------------

const GOAL_ITEM = {
  source: "jira",
  key: "WEB-142",
  title: "Checkout: card form loses focus after a validation error",
} as const;

export function sampleGoal(): SessionGoal {
  return { ...GOAL_ITEM, url: `${SAMPLE_URL_PREFIX}jira/${GOAL_ITEM.key}` };
}

/** The meeting of the hold banner ends this long after the click. */
const HOLD_LEFT_MINUTES = 25;

export function sampleHold(now: number, choice: MeetingHoldView["choice"]): MeetingHoldView {
  return { subject: sampleTexts().meeting, untilMs: now + HOLD_LEFT_MINUTES * MINUTE_MS, choice };
}

export function sampleNudge(): Nudge {
  return { process: "chrome.exe", matched: "youtube.com", title: "" };
}

// ---------------------------------------------------------------------------
// Summary + history
// ---------------------------------------------------------------------------

/** One app of a sample session: process, focus minutes, and its windows (title, minutes). */
type AppSpec = readonly [process: string, minutes: number, titles?: readonly (readonly [title: string, minutes: number])[]];

interface SessionSpec {
  /** 0 = the last session (ended `LAST_ENDED_MINUTES_AGO` before the click). */
  daysAgo: number;
  /** Local start time of an older session. */
  startHour: number;
  startMinute: number;
  cycles: number;
  breakMinutes: number;
  idleMinutes: number;
  unknownMinutes: number;
  apps: readonly AppSpec[];
  goal: boolean;
  goalDone: boolean | null;
}

const LAST_ENDED_MINUTES_AGO = 5;

const SESSIONS: readonly SessionSpec[] = [
  {
    daysAgo: 0,
    startHour: 0,
    startMinute: 0,
    cycles: 3,
    breakMinutes: 30,
    idleMinutes: 4,
    unknownMinutes: 1,
    goal: true,
    goalDone: null,
    apps: [
      [
        "Code.exe",
        65,
        [
          ["session.rs — focusmode — Visual Studio Code", 28],
          ["+page.svelte — focusmode — Visual Studio Code", 21],
          ["api.ts — focusmode — Visual Studio Code", 12],
        ],
      ],
      [
        "chrome.exe",
        25,
        [
          ["Runes • Svelte docs", 9],
          ["Add retry with backoff to the sync worker · Pull Request #318", 8],
          ["WEB-142 Checkout: card form loses focus - Jira", 6],
        ],
      ],
      ["WindowsTerminal.exe", 15, [["cargo test --workspace", 11], ["npm run check", 4]]],
      ["Figma.exe", 10, [["Settings page – v3 – Figma", 10]]],
      ["ms-teams.exe", 7, [["Chat | Design team | Microsoft Teams", 7]]],
      ["Notion.exe", 5, [["Sprint 14 notes", 5]]],
      ["Slack.exe", 3],
      ["Spotify.exe", 2],
      ["OUTLOOK.EXE", 2],
      ["Obsidian.exe", 1],
    ],
  },
  {
    daysAgo: 1,
    startHour: 14,
    startMinute: 10,
    cycles: 2,
    breakMinutes: 15,
    idleMinutes: 6,
    unknownMinutes: 0,
    goal: true,
    goalDone: true,
    apps: [
      ["Code.exe", 48, [["NextUp.svelte — focusmode — Visual Studio Code", 30]]],
      ["chrome.exe", 22, [["GitLab · !57 Migrate CI cache", 12]]],
      ["Figma.exe", 9],
      ["Slack.exe", 5],
    ],
  },
  {
    daysAgo: 1,
    startHour: 9,
    startMinute: 30,
    cycles: 1,
    breakMinutes: 0,
    idleMinutes: 2,
    unknownMinutes: 1,
    goal: false,
    goalDone: null,
    apps: [
      ["OUTLOOK.EXE", 14],
      ["ms-teams.exe", 12],
      ["chrome.exe", 11],
      ["Code.exe", 5],
    ],
  },
  {
    daysAgo: 2,
    startHour: 13,
    startMinute: 0,
    cycles: 4,
    breakMinutes: 45,
    idleMinutes: 9,
    unknownMinutes: 2,
    goal: true,
    goalDone: false,
    apps: [
      ["Code.exe", 102, [["tracker.rs — focusmode — Visual Studio Code", 60]]],
      ["WindowsTerminal.exe", 31],
      ["chrome.exe", 26],
      ["Notion.exe", 8],
      ["Spotify.exe", 2],
    ],
  },
  {
    daysAgo: 3,
    startHour: 10,
    startMinute: 15,
    cycles: 2,
    breakMinutes: 15,
    idleMinutes: 3,
    unknownMinutes: 0,
    goal: false,
    goalDone: null,
    apps: [
      ["Figma.exe", 41],
      ["chrome.exe", 28],
      ["Slack.exe", 10],
      ["Code.exe", 8],
    ],
  },
  {
    daysAgo: 5,
    startHour: 16,
    startMinute: 40,
    cycles: 1,
    breakMinutes: 0,
    idleMinutes: 5,
    unknownMinutes: 3,
    goal: false,
    goalDone: null,
    apps: [
      ["chrome.exe", 21],
      ["Code.exe", 13],
      ["Obsidian.exe", 3],
    ],
  },
];

function usageOf(spec: SessionSpec): UsageReport {
  const apps: AppUsage[] = spec.apps.map(([process, minutes, titles]) => ({
    process,
    secs: minutes * MINUTE_SECS,
    titles: (titles ?? []).map(([title, m]) => ({ title, secs: m * MINUTE_SECS })),
  }));
  const idleSecs = spec.idleMinutes * MINUTE_SECS;
  const unknownSecs = spec.unknownMinutes * MINUTE_SECS;
  const trackedSecs = apps.reduce((sum, a) => sum + a.secs, 0) + idleSecs + unknownSecs;
  return { apps, idleSecs, unknownSecs, trackedSecs };
}

/** Local midnight `daysAgo` days before `now`. */
function midnight(now: number, daysAgo: number): number {
  const day = new Date(now - daysAgo * DAY_MS);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

function sessionOf(spec: SessionSpec, index: number, now: number): SessionSummary {
  const usage = usageOf(spec);
  const focusedSecs = usage.trackedSecs;
  const breakSecs = spec.breakMinutes * MINUTE_SECS;
  const lengthMs = (focusedSecs + breakSecs) * SECOND_MS;
  const startedAtMs =
    spec.daysAgo === 0
      ? now - LAST_ENDED_MINUTES_AGO * MINUTE_MS - lengthMs
      : midnight(now, spec.daysAgo) + spec.startHour * HOUR_MS + spec.startMinute * MINUTE_MS;
  return {
    sessionId: `${SAMPLE_ID_PREFIX}session-${index + 1}`,
    startedAtMs,
    endedAtMs: startedAtMs + lengthMs,
    focusedSecs,
    breakSecs,
    cyclesCompleted: spec.cycles,
    usage,
    demo: false,
    goal: spec.goal ? sampleGoal() : null,
    goalDone: spec.goal ? spec.goalDone : null,
  };
}

/** The last session and the history list (newest first, the last session included). */
export function sampleSummaries(now: number): { last: SessionSummary | null; history: SessionSummary[] } {
  const history = SESSIONS.map((spec, i) => sessionOf(spec, i, now)).sort((a, b) => b.endedAtMs - a.endedAtMs);
  return { last: history[0] ?? null, history };
}

// ---------------------------------------------------------------------------
// "Co dalej?" (work items)
// ---------------------------------------------------------------------------

const ACCOUNT = "you@example.com";
/** The sample tools and calendars "synced" this long before the click. */
const LAST_SYNC_MINUTES_AGO = 2;
const WORK_SOURCES: readonly WorkSource[] = ["github", "gitlab", "jira", "azureDevops", "linear"];

type ItemSpec = Omit<WorkItem, "id" | "url" | "dueMs" | "updatedMs"> & { dueToday: boolean; updatedMinutesAgo: number };

/** Ranked like focus-core: assigned first, then what waits on the user, then notifications. */
const ITEMS: readonly ItemSpec[] = [
  {
    source: "jira",
    kind: "assigned",
    key: GOAL_ITEM.key,
    title: GOAL_ITEM.title,
    project: "Web Shop",
    priorityLabel: "High",
    priorityRank: 2,
    status: "In Progress",
    inSprint: true,
    score: 96,
    reasons: ["assigned", "inSprint", "dueToday", "inProgress"],
    unseen: false,
    dueToday: true,
    updatedMinutesAgo: 40,
  },
  {
    source: "azureDevops",
    kind: "assigned",
    key: "AB#2041",
    title: "Update the onboarding checklist for new hires",
    project: "People Ops",
    priorityLabel: "Priority 1",
    priorityRank: 1,
    status: "Active",
    inSprint: true,
    score: 88,
    reasons: ["assigned", "highPriority", "inSprint"],
    unseen: true,
    dueToday: false,
    updatedMinutesAgo: 95,
  },
  {
    source: "github",
    kind: "reviewRequested",
    key: "#318",
    title: "Add retry with backoff to the sync worker",
    project: "focus-team/api",
    priorityLabel: null,
    priorityRank: null,
    status: null,
    inSprint: false,
    score: 74,
    reasons: ["waitingLong", "blockingOthers"],
    unseen: false,
    dueToday: false,
    updatedMinutesAgo: 26 * 60,
  },
  {
    source: "gitlab",
    kind: "changesRequested",
    key: "!57",
    title: "Migrate the CI cache to the new runners",
    project: "platform/ci",
    priorityLabel: null,
    priorityRank: null,
    status: null,
    inSprint: false,
    score: 61,
    reasons: ["mentioned"],
    unseen: false,
    dueToday: false,
    updatedMinutesAgo: 180,
  },
  {
    source: "linear",
    kind: "notification",
    key: "ENG-88",
    title: "Design review notes for the settings page",
    project: "Design System",
    priorityLabel: "Medium",
    priorityRank: 3,
    status: "In Review",
    inSprint: false,
    score: 32,
    reasons: ["mentioned"],
    unseen: true,
    dueToday: false,
    updatedMinutesAgo: 15,
  },
];

/** Evening of the day of `now` (a due date that reads "today"). */
const DUE_HOUR = 18;

export function sampleWork(now: number): WorkStatus {
  const due = new Date(now);
  due.setHours(DUE_HOUR, 0, 0, 0);
  const items: WorkItem[] = ITEMS.map(({ dueToday, updatedMinutesAgo, ...item }, i) => ({
    ...item,
    id: `${SAMPLE_ID_PREFIX}work-${i + 1}`,
    url: `${SAMPLE_URL_PREFIX}${item.source}/${encodeURIComponent(item.key)}`,
    dueMs: dueToday ? Math.max(due.getTime(), now + HOUR_MS) : null,
    updatedMs: now - updatedMinutesAgo * MINUTE_MS,
  }));
  return {
    revision: 0,
    sources: WORK_SOURCES.map((source): WorkSourceStatus => ({ source, state: "on", detail: null, account: ACCOUNT })),
    items,
    heldCount: 0,
    lastSyncMs: now - LAST_SYNC_MINUTES_AGO * MINUTE_MS,
  };
}

// ---------------------------------------------------------------------------
// Calendar conflicts
// ---------------------------------------------------------------------------

/** Where the breaks of the sample plan are: the first break's start and the phase lengths. */
export interface BreakPlan {
  firstBreakStartMs: number;
  focusSecs: number;
  breakSecs: number;
  /** Breaks of a session that would start now (idle), not of a running one. */
  planned: boolean;
}

/** Sample meetings, placed relative to the planned breaks (minutes after the break's start); subjects in the UI language. */
const MEETING_A = { afterBreakStartMinutes: 3, minutes: 30, online: true } as const;
const MEETING_B = { afterBreakStartMinutes: -5, minutes: 45, online: true } as const;
const MEETING_C = { afterBreakEndMinutes: 20, minutes: 30, online: false } as const;
/** The first reminder of a meeting comes this long before it (focus-core: 30 / 20 / 10). */
const FIRST_REMINDER_MINUTES = 20;

export function sampleCalendar(now: number, plan: BreakPlan): CalendarStatus {
  const cycleMs = (plan.focusSecs + plan.breakSecs) * SECOND_MS;
  const breakStart = (n: number) => plan.firstBreakStartMs + (n - 1) * cycleMs;
  const breakEnd = (n: number) => breakStart(n) + plan.breakSecs * SECOND_MS;

  const meeting = (id: string, subject: string, startMs: number, minutes: number, online: boolean): Meeting => ({
    id: `${SAMPLE_ID_PREFIX}meeting-${id}`,
    source: "microsoft",
    subject,
    startMs,
    endMs: startMs + minutes * MINUTE_MS,
    online,
  });

  const texts = sampleTexts();
  const a = meeting("a", texts.meeting, breakStart(1) + MEETING_A.afterBreakStartMinutes * MINUTE_MS, MEETING_A.minutes, MEETING_A.online);
  const b = meeting("b", texts.meetingOneOnOne, breakStart(2) + MEETING_B.afterBreakStartMinutes * MINUTE_MS, MEETING_B.minutes, MEETING_B.online);
  const c = meeting("c", texts.meetingDesign, breakEnd(2) + MEETING_C.afterBreakEndMinutes * MINUTE_MS, MEETING_C.minutes, MEETING_C.online);

  const reminderA = a.startMs - FIRST_REMINDER_MINUTES * MINUTE_MS;
  const conflicts: MeetingConflict[] = [
    {
      meeting: a,
      breakNumber: 1,
      breakStartMs: breakStart(1),
      breakEndMs: breakEnd(1),
      choice: null,
      nextReminderMs: reminderA > now ? reminderA : null,
      remindLaterAllowed: true,
      tooLong: false,
    },
    {
      meeting: b,
      breakNumber: 2,
      breakStartMs: breakStart(2),
      breakEndMs: breakEnd(2),
      choice: "extend",
      nextReminderMs: null,
      remindLaterAllowed: true,
      tooLong: false,
    },
  ];
  return {
    revision: 0,
    sources: [
      { source: "microsoft", state: "on", detail: null, account: ACCOUNT },
      { source: "google", state: "off", detail: null, account: null },
      { source: "ics", state: "off", detail: null, account: null },
    ],
    meetings: [a, b, c],
    conflicts,
    planned: plan.planned,
    lastSyncMs: now - LAST_SYNC_MINUTES_AGO * MINUTE_MS,
    googlePending: false,
  };
}

// ---------------------------------------------------------------------------
// Beta rewards (ADR-022): sample statuses and moments. They live in the rewards store's sample layer only (`sample:
// true`): never acked, never sent to the shell, never written.
// ---------------------------------------------------------------------------

/** Sample moment ids start here, far from the shell's (which grow from 1 within an app run). */
const SAMPLE_MOMENT_ID_BASE = 1_000_000;
let sampleMomentSeq = 0;

export type RewardsStatusSample = "today" | "goalMet" | "waiting" | "first" | "paused" | "allMarks" | "preview";

export type RewardMomentSample =
  | "start"
  | "step"
  | "firstStep"
  | "allSteps"
  | "back"
  | "recap"
  | "bubble"
  | "pearl"
  | "golden"
  | "dailyGoal"
  | "comeback"
  | "streak"
  | "milestone"
  | "restUsed"
  | "restEarned"
  | "levelUp"
  | "levelUp2"
  | "pondFull"
  | "full"
  | "sessionEnd"
  | "earlyStop"
  | "goalDone"
  | "sticker";

/** Mirrors focus-core `rewards.rs` (odds, pity, rest bank, level spans), for realistic samples only. */
const SAMPLE_ODDS = { nonePermille: 500, bubblePermille: 320, pearlPermille: 150, goldenPermille: 30, pityBlocks: 4 };
const SAMPLE_REST_BANK_MAX = 3;
const LEVEL_BASE = 60;
const LEVEL_STEP = 20;
const LEVEL_SPAN_MAX = 500;

function levelSpan(level: number): number {
  return Math.min(LEVEL_BASE + LEVEL_STEP * (level - 1), LEVEL_SPAN_MAX);
}

const pad2 = (n: number) => String(n).padStart(2, "0");

function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

/** Monday..Sunday of the local week of `now`: earlier days goal / rest / day off, then today, then the future. */
function sampleWeek(now: number, todayMark: DayMark, every: boolean): DayCell[] {
  const today = new Date(now);
  const weekday = (today.getDay() + 6) % 7;
  const monday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - weekday);
  const fixed: DayMark[] = ["goal", "rest", "neutral", "quiet", "today", "future", "future"];
  const past: DayMark[] = ["goal", "goal", "rest", "goal", "goal", "neutral", "neutral"];
  return Array.from({ length: 7 }, (_, i) => {
    const date = new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + i);
    let mark: DayMark;
    if (every) mark = fixed[i] ?? "future";
    else if (i < weekday) mark = past[i] ?? "goal";
    else if (i === weekday) mark = todayMark;
    else mark = "future";
    return { date: isoDate(date), mark };
  });
}

const rewardLine = (kind: RewardKind, sparks: number, count = 1): RewardLine => ({ kind, sparks, count });

/** A status for the Developer page's "Status samples" (also shown while a sample moment plays). */
export function sampleRewardsStatus(kind: RewardsStatusSample, now: number): RewardsStatus {
  const goalMet = kind === "goalMet";
  const first = kind === "first";
  const level = first ? 1 : 7;
  const sparks = goalMet ? 42 : first ? 12 : 18;
  const streakDays = goalMet ? 7 : 6;
  return {
    revision: 0,
    active: true,
    preview: kind === "preview",
    totalSparks: first ? 12 : 1180,
    level: { level, into: first ? 12 : 95, span: levelSpan(level) },
    today: { date: isoDate(new Date(now)), sparks, goal: 30, goalMet, pondFull: false },
    streak: {
      state: kind === "waiting" ? "waiting" : first ? "first" : kind === "paused" ? "paused" : "alive",
      days: kind === "waiting" || first ? 0 : streakDays,
      best: first ? 0 : 12,
      activeDays: first ? 0 : 41,
      todayCounted: goalMet,
      restBank: 2,
      restBankMax: SAMPLE_REST_BANK_MAX,
      restReserved: 0,
      nextRestIn: 3,
    },
    week: sampleWeek(now, goalMet ? "goal" : "today", kind === "allMarks"),
    session: {
      sessionId: `${SAMPLE_ID_PREFIX}rewards`,
      sparks: 134,
      lines: [
        rewardLine("start", 5),
        rewardLine("focusTime", 90),
        rewardLine("fullBlock", 20, 2),
        rewardLine("step", 9, 3),
        rewardLine("breakTaken", 5),
      ],
    },
    collection: first ? { bubbles: 0, pearls: 0, golden: 0 } : { bubbles: 12, pearls: 3, golden: 1 },
    odds: SAMPLE_ODDS,
    stickers: first ? [] : ["firstStep", "dailyGoal", "comeback"],
    pending: [],
  };
}

/** Small steps of the sample summary (plain sample texts). */
export function sampleSteps(): MicroStep[] {
  const ideas = ["Open the file", "Write the title", "Read the first paragraph", "List 3 things"];
  return ideas.map((text, i) => ({ id: i + 1, text, done: i < 3 }));
}

function sampleMoment(kind: MomentKind, now: number, patch: Partial<RewardMoment>): RewardMoment {
  sampleMomentSeq++;
  const lines = patch.lines ?? [];
  const sparks = lines.reduce((sum, l) => sum + l.sparks, 0);
  return {
    id: SAMPLE_MOMENT_ID_BASE + sampleMomentSeq,
    kind,
    atMs: now,
    sparks,
    sessionTotal: sparks,
    lines,
    surprise: null,
    dailyGoal: false,
    comeback: false,
    streak: null,
    newStreak: false,
    milestone: null,
    restDaysUsed: 0,
    restDayEarned: false,
    levelUp: null,
    levelsGained: 0,
    pondFull: false,
    stickers: [],
    firstStep: false,
    quiet: false,
    preview: false,
    ...patch,
    sample: true,
  };
}

const BLOCK_LINES: readonly RewardLine[] = [rewardLine("focusTime", 45), rewardLine("fullBlock", 10)];

/** One sample moment for the Developer page's "Small moments" and "Bursts" buttons. */
export function sampleRewardMoment(kind: RewardMomentSample, now: number): RewardMoment {
  const block = (extra: readonly RewardLine[], patch: Partial<RewardMoment> = {}) =>
    sampleMoment("breakStart", now, { lines: [...BLOCK_LINES, ...extra], ...patch });
  const goal = rewardLine("dailyGoal", 10);
  switch (kind) {
    case "start":
      return sampleMoment("start", now, {
        lines: [rewardLine("start", 5), rewardLine("firstOfDay", 5), rewardLine("plan", 2)],
      });
    case "step":
      return sampleMoment("step", now, { lines: [rewardLine("step", 3)] });
    case "firstStep":
      return sampleMoment("step", now, { lines: [rewardLine("step", 3)], firstStep: true });
    case "allSteps":
      return sampleMoment("allSteps", now, { lines: [rewardLine("step", 3), rewardLine("allSteps", 5)] });
    case "back":
      return sampleMoment("backFromBreak", now, { lines: [rewardLine("breakTaken", 5), rewardLine("backFromBreak", 5)] });
    case "recap":
      return block([]);
    case "bubble":
      return block([rewardLine("surprise", 5)], { surprise: "bubble" });
    case "pearl":
      return block([rewardLine("surprise", 15)], { surprise: "pearl" });
    case "golden":
      return block([rewardLine("surprise", 40)], { surprise: "goldenPearl" });
    case "dailyGoal":
      return block([goal], { dailyGoal: true });
    case "comeback":
      return block([goal, rewardLine("comeback", 10)], { dailyGoal: true, comeback: true, newStreak: true, streak: 1 });
    case "streak":
      return block([goal], { dailyGoal: true, streak: 6 });
    case "milestone":
      return block([goal], { dailyGoal: true, streak: 7, milestone: 7 });
    case "restUsed":
      return block([goal], { dailyGoal: true, streak: 8, restDaysUsed: 1 });
    case "restEarned":
      return block([goal], { dailyGoal: true, streak: 10, restDayEarned: true });
    case "levelUp":
      return block([], { levelUp: 8, levelsGained: 1 });
    case "levelUp2":
      return block([], { levelUp: 9, levelsGained: 2 });
    case "pondFull":
      return block([], { pondFull: true });
    case "full":
      return block([rewardLine("surprise", 40), goal], {
        surprise: "goldenPearl",
        dailyGoal: true,
        streak: 7,
        milestone: 7,
        restDayEarned: true,
        levelUp: 8,
        levelsGained: 1,
        pondFull: true,
        stickers: ["threeDays", "goldenPearl", "level5"] satisfies StickerId[],
      });
    case "sessionEnd": {
      const lines = [
        rewardLine("start", 5),
        rewardLine("focusTime", 90),
        rewardLine("fullBlock", 20, 2),
        rewardLine("step", 9, 3),
        rewardLine("breakTaken", 5),
        goal,
      ];
      const m = sampleMoment("sessionEnd", now, { lines, dailyGoal: true, streak: 6 });
      return { ...m, sessionTotal: m.sparks };
    }
    case "earlyStop": {
      const m = sampleMoment("sessionEnd", now, { lines: [rewardLine("start", 5), rewardLine("focusTime", 30)] });
      return { ...m, sessionTotal: m.sparks };
    }
    case "goalDone":
      return sampleMoment("goalDone", now, { lines: [rewardLine("goalDone", 10)] });
    case "sticker":
      return block([], { stickers: ["firstStep"] });
  }
}
