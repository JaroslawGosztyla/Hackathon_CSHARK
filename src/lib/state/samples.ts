// Developer-mode sample data (ADR-019): realistic content for screenshots, built at click time and kept only in memory
// (a sample layer in the stores, cleared by "Clear samples" and on restart). Nothing here is saved, sent or shown to
// the shell, and no link of a sample is ever opened. Ids start with `dev-sample-` (the shell treats meeting choices for
// such ids as no-ops); links start with `SAMPLE_URL_PREFIX`.
import type {
  AppUsage,
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
