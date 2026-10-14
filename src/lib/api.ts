// The only module that talks to the Tauri shell (invoke / listen / plugins).
// Command and event names come from the generated `bindings/names.ts`, DTO types from ts-rs bindings.
import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { disable as autostartDisable, enable as autostartEnable, isEnabled as autostartIsEnabled } from "@tauri-apps/plugin-autostart";
import { open as dialogOpen, save as dialogSave } from "@tauri-apps/plugin-dialog";
import { openUrl as openerOpenUrl } from "@tauri-apps/plugin-opener";
import { COMMANDS, EVENTS } from "./bindings/names";
import type {
  AppError,
  BlocklistInfo,
  BridgeInfo,
  CalendarSource,
  CalendarStatus,
  ChimeKind,
  Config,
  DevAction,
  DeviceCodePrompt,
  HistoryChanged,
  Language,
  MediaAction,
  MediaWindowStatus,
  MeetingChoice,
  Nudge,
  PhaseChange,
  PopupAction,
  PopupInfo,
  PrivacyItemKind,
  PrivacyReport,
  RewardSound,
  RewardsStatus,
  SessionGoal,
  SessionSnapshot,
  SessionSummary,
  ShieldStatus,
  StepDraft,
  SunCity,
  TeamsStatus,
  TintStatus,
  Track,
  UsageReport,
  WeeklyStats,
  WorkSource,
  WorkStatus,
  YoutubeAccount,
} from "./types";

export type { UnlistenFn };

/** Error code used when the page runs outside the Tauri WebView (plain `vite dev` in a browser). */
export const ERR_NO_SHELL = "ipc.unavailable";
/** Error code used when the shell rejected a call without an `AppError` payload. */
export const ERR_IPC = "ipc.failed";

export function inShell(): boolean {
  try {
    return isTauri();
  } catch {
    return false;
  }
}

/** Normalizes anything thrown by `invoke` (an `AppError` object, a string, an `Error`) into an `AppError`. */
export function toAppError(e: unknown): AppError {
  if (e !== null && typeof e === "object" && "code" in e && "message" in e) {
    const field = "field" in e && typeof e.field === "string" ? e.field : null;
    return { code: String(e.code), message: String(e.message), field };
  }
  if (typeof e === "string") return { code: ERR_IPC, message: e, field: null };
  if (e instanceof Error) return { code: ERR_IPC, message: e.message, field: null };
  return { code: ERR_IPC, message: String(e), field: null };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inShell()) {
    throw { code: ERR_NO_SHELL, message: `command ${cmd} needs the FocusMode shell`, field: null } satisfies AppError;
  }
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

export const getSnapshot = () => call<SessionSnapshot>(COMMANDS.getSnapshot);
export const startSession = () => call<void>(COMMANDS.startSession);
export const pauseSession = () => call<void>(COMMANDS.pauseSession);
export const resumeSession = () => call<void>(COMMANDS.resumeSession);
export const stopSession = () => call<void>(COMMANDS.stopSession);
export const skipPhase = () => call<void>(COMMANDS.skipPhase);

export const getConfig = () => call<Config>(COMMANDS.getConfig);
export const updateConfig = (config: Config) => call<Config>(COMMANDS.updateConfig, { config });

export const listFolderTracks = (path: string) => call<Track[]>(COMMANDS.listFolderTracks, { path });

export const getShieldStatus = () => call<ShieldStatus>(COMMANDS.getShieldStatus);
export const getCurrentUsage = () => call<UsageReport>(COMMANDS.getCurrentUsage);
export const getLastSummary = () => call<SessionSummary | null>(COMMANDS.getLastSummary);
export const listHistory = (limit: number) => call<SessionSummary[]>(COMMANDS.listHistory, { limit });

export const getTeamsStatus = () => call<TeamsStatus>(COMMANDS.getTeamsStatus);
/**
 * Starts the Microsoft device code sign-in (also while connected: signs in again). The shell asks for `Calendars.Read`
 * only while `calendar.microsoft` is on in the stored config, so save pending edits first.
 */
export const teamsConnect = () => call<DeviceCodePrompt>(COMMANDS.teamsConnect);
export const teamsDisconnect = () => call<void>(COMMANDS.teamsDisconnect);

export const getBridgeInfo = () => call<BridgeInfo>(COMMANDS.getBridgeInfo);
export const openExtensionDir = () => call<void>(COMMANDS.openExtensionDir);
export const testChime = (chime: ChimeKind) => call<void>(COMMANDS.testChime, { chime });

/** Opens (or navigates) the separate YouTube window for a link that cannot play in the embedded player. */
export const openMediaWindow = (url: string, play: boolean) => call<void>(COMMANDS.openMediaWindow, { url, play });
/** Controls the YouTube window (no-op in the shell when it is closed). `volume` (0..1) is used only by `setVolume`. */
export const controlMediaWindow = (action: MediaAction, volume: number | null = null) =>
  call<void>(COMMANDS.controlMediaWindow, { action, volume });
export const getMediaWindow = () => call<MediaWindowStatus>(COMMANDS.getMediaWindow);

/** The parsed `websiteblacklist.txt` (the shell creates it from the bundled default on first run). */
export const getBlocklist = () => call<BlocklistInfo>(COMMANDS.getBlocklist);
/** Opens `websiteblacklist.txt` in the system's default text editor. */
export const openBlocklistFile = () => call<void>(COMMANDS.openBlocklistFile);
/** Re-reads `websiteblacklist.txt` now (the shell also reloads it by itself after every save). */
export const reloadBlocklist = () => call<BlocklistInfo>(COMMANDS.reloadBlocklist);

/**
 * Answer to `app://close-requested` (the first X on the main window): stores `closeToTray` (and turns on
 * `phasePopups` when `toTray`), then the shell hides the window or quits. Resolves with the stored config.
 */
export const resolveClose = (toTray: boolean) => call<Config>(COMMANDS.resolveClose, { toTray });
/** The supported language closest to the OS UI language (`en` when none matches). */
export const getSystemLanguage = () => call<Language>(COMMANDS.getSystemLanguage);

/** What the phase popup window shows now (popup window only). */
export const getPopup = () => call<PopupInfo>(COMMANDS.getPopup);
/** A button in the phase popup, or its timeout (`dismiss`). */
export const popupAction = (action: PopupAction) => call<void>(COMMANDS.popupAction, { action });

export const getTintStatus = () => call<TintStatus>(COMMANDS.getTintStatus);
/**
 * Shows the warm tint at `strength` (0..=100) for a few seconds, ignoring the schedule (settings preview);
 * `null` ends the preview at once.
 */
export const previewTint = (strength: number | null) => call<void>(COMMANDS.previewTint, { strength });

/**
 * Copies an audio file (mp3/wav/ogg/flac/m4a, at most 10 MB, must decode) into the app data folder as the custom
 * sound for `chime` and selects `chimeSound: custom`. Resolves with the stored config.
 */
export const importChime = (path: string, chime: ChimeKind) => call<Config>(COMMANDS.importChime, { path, chime });
/** Deletes the custom sound for `chime` (the other custom file, or the default sound, takes over). */
export const removeCustomChime = (chime: ChimeKind) => call<Config>(COMMANDS.removeCustomChime, { chime });
/** The music bundled with the app (Open Lo-Fi, CC0), playable via `fileSrc(path)`. */
export const listLibraryTracks = () => call<Track[]>(COMMANDS.listLibraryTracks);
/** Deletes the whole session history (history file and the last summary). Confirm with the user first. */
export const clearHistory = () => call<void>(COMMANDS.clearHistory);

/**
 * Meetings of the next hours from all connected calendars and their collisions with the planned breaks
 * (of the running session, or of a session that would start now).
 */
export const getCalendar = () => call<CalendarStatus>(COMMANDS.getCalendar);
/** Fetches all calendars now. */
export const refreshCalendar = () => call<CalendarStatus>(COMMANDS.refreshCalendar);
/** How a meeting that collides with a break is handled. */
export const setMeetingChoice = (meetingId: string, choice: MeetingChoice) =>
  call<CalendarStatus>(COMMANDS.setMeetingChoice, { meetingId, choice });
/** Starts the Google sign-in in the system browser (OAuth loopback + PKCE); progress arrives as `calendar://changed`. */
export const connectGoogle = () => call<void>(COMMANDS.connectGoogle);
/**
 * Forgets the stored credentials / links of that source. `microsoft` keeps the Teams sign-in: the shell only turns
 * `calendar.microsoft` off (and stores that config) and drops the Microsoft meetings.
 */
export const disconnectCalendar = (source: CalendarSource) =>
  call<CalendarStatus>(COMMANDS.disconnectCalendar, { source });
/** Replaces the ICS subscription links (the shell stores them encrypted, never in config.json). */
export const setIcsUrls = (urls: string[]) => call<CalendarStatus>(COMMANDS.setIcsUrls, { urls });
/** The stored ICS links (only to edit them in Settings; never show them in errors or logs). */
export const getIcsUrls = () => call<string[]>(COMMANDS.getIcsUrls);

/** Ranked work items ("Co dalej?") from the connected developer tools, plus the state of every tool. */
export const getWork = () => call<WorkStatus>(COMMANDS.getWork);
/** Fetches all connected tools now. */
export const refreshWork = () => call<WorkStatus>(COMMANDS.refreshWork);
/**
 * Stores (encrypted, never in config.json) or, with `null`, forgets the access token of a tool. The token is a
 * secret: never log it, never keep it in a store, never put it into an error text.
 */
export const setWorkToken = (source: WorkSource, token: string | null) =>
  call<WorkStatus>(COMMANDS.setWorkToken, { source, token });
/** The item the user focuses on (shown under the timer, kept in the summary); `null` clears it. Works in any phase. */
export const setSessionGoal = (goal: SessionGoal | null) => call<void>(COMMANDS.setSessionGoal, { goal });
/** Marks the goal of a finished session as done / not done (history). */
export const markGoalDone = (sessionId: string, done: boolean) =>
  call<void>(COMMANDS.markGoalDone, { sessionId, done });

/**
 * A button on the developer page (ADR-019): a sample popup, a phase toast, a shorter running phase, presentation mode,
 * a factory reset or a rewards cheat (`rewardsOff` while the beta rewards are off). The shell rejects
 * it with `developerModeOff` unless `config.developerMode` is on (and `setRemaining` with `notRunning` when idle).
 * Nothing leaves the computer; only the factory reset and a rewards cheat on the real ledger (`rewards.json`) write.
 */
export const devTrigger = (action: DevAction) => call<void>(COMMANDS.devTrigger, { action });

/**
 * What FocusMode stores on this computer, item by item (ADR-020): presence, size, count. Never the contents of a token,
 * an ICS link or a cookie.
 */
export const getPrivacyReport = () => call<PrivacyReport>(COMMANDS.getPrivacyReport);
/** Opens the app data folder (`PrivacyReport.folders[0]`) in the file manager. */
export const openDataFolder = () => call<void>(COMMANDS.openDataFolder);
/**
 * Writes one JSON file at `path` (from `pickSaveFile`): config without secrets, history, meeting choices, seen work items.
 * Never tokens, ICS links, cookies or other secrets.
 */
export const exportPrivacyData = (path: string) => call<void>(COMMANDS.exportPrivacyData, { path });
/**
 * Deletes those items (only `deletable` ones, else `privacy.notDeletable`). A running session is stopped first. Resolves
 * with the stored config (deleting the custom chimes changes it); the report follows as `privacy://changed`.
 * Confirm with the user first.
 */
export const deletePrivacyData = (kinds: PrivacyItemKind[]) => call<Config>(COMMANDS.deletePrivacyData, { kinds });
/**
 * "Delete everything": stops a running session, deletes every deletable item and resets the settings to the defaults
 * (resolves with them). The app stays installed. Confirm with the user first (list what goes).
 */
export const deleteAllData = () => call<Config>(COMMANDS.deleteAllData);

/** YouTube sign-in state of the app's embedded browser profile (ADR-021). */
export const getYoutubeAccount = () => call<YoutubeAccount>(COMMANDS.getYoutubeAccount);
/**
 * Opens (or focuses) the YouTube sign-in window: Google's own page, the user types there, the app never sees the
 * password. It closes by itself once signed in; progress via `youtube://account`.
 */
export const youtubeSignIn = () => call<void>(COMMANDS.youtubeSignIn);
/** Deletes the google.com / youtube.com cookies of the app's browser profile (the consent cookies stay). */
export const youtubeSignOut = () => call<void>(COMMANDS.youtubeSignOut);

// Beta: sparks, streak and small steps (ADR-022); the collection of themes and ambience layers (ADR-024).
/**
 * Sparks, daily goal, level, streak, week, the moments not acknowledged yet and the collection (`unlocks`, ADR-024).
 * The main window subscribes (`subscribeRewards`); the popup reads it only to resolve the collection theme on screen
 * (`effectiveTheme` in `$lib/themes`) and never acks.
 */
export const getRewards = () => call<RewardsStatus>(COMMANDS.getRewards);
/** Removes the pending moments with `id <= upTo` (played or shown). Main window only: the popup never acks. */
export const ackRewardMoments = (upTo: number) => call<void>(COMMANDS.ackRewardMoments, { upTo });
/** "Zacznij od nowa": a new ledger (session history stays; the collection unlocks go too). Confirm with the user first. */
export const resetRewards = () => call<RewardsStatus>(COMMANDS.resetRewards);
/**
 * A short reward sound (pop / sparkle / fanfare). The shell skips it while rewards or reward sounds are off,
 * celebrations are `numbersOnly` or the user is presenting, and waits for a phase chime to finish.
 */
export const playRewardSound = (sound: RewardSound) => call<void>(COMMANDS.playRewardSound, { sound });
/**
 * Replaces the small steps of the coming or running session (any phase): `id` keeps an existing step, `null` adds
 * one. Errors: `stepsOff`, `tooManySteps` (field `steps`, more than 5). The new plan arrives in `session://snapshot`.
 */
export const setSessionSteps = (steps: StepDraft[]) => call<void>(COMMANDS.setSessionSteps, { steps });
/** Ticks / unticks a step (Focus or Break, also paused; no-op when idle or for an unknown id). */
export const setStepDone = (id: number, done: boolean) => call<void>(COMMANDS.setStepDone, { id, done });

// ADR-023: tint exceptions, smart breaks, weekly stats (the first-run wizard needs only `Config.onboardingDone`).
/** The built-in cities for the sunset / sunrise tint schedule: static data in the app (no location service, no network). */
export const listSunCities = () => call<SunCity[]>(COMMANDS.listSunCities);
/**
 * "Ask me instead" for a meeting that smart breaks fitted (`optOut: true`): the session gets its unadjusted phase end
 * back and the meeting is asked about like any collision; `false` lets smart breaks fit it again.
 */
export const setSmartBreakOptOut = (meetingId: string, optOut: boolean) =>
  call<CalendarStatus>(COMMANDS.setSmartBreakOptOut, { meetingId, optOut });
/**
 * One ISO week (Monday first, local time) from the stored history. `weekStart` is any `YYYY-MM-DD` day of that week
 * (the answer's `weekStart` is its Monday). A malformed date → `stats.week` (field `weekStart`). Prefer
 * `watchWeeklyStats`, which also follows `history://changed`.
 */
export const getWeeklyStats = (weekStart: string) => call<WeeklyStats>(COMMANDS.getWeeklyStats, { weekStart });

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/** Listens to a shell event. Outside the shell it is a no-op. */
export async function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!inShell()) return () => {};
  return listen<T>(event, (e) => handler(e.payload));
}

/** `a` is newer than `b` in u32 serial-number arithmetic (survives wrap-around). */
export function isNewerRevision(a: number, b: number): boolean {
  const diff = (a - b) >>> 0;
  return diff !== 0 && diff < 0x8000_0000;
}

/**
 * Window state protocol (ADR-012): listen first, then read the getter, and apply only strictly newer revisions,
 * so neither an event emitted during the getter call nor a stale getter result can roll the state back.
 */
export async function subscribe<T extends { revision: number }>(
  event: string,
  getter: () => Promise<T>,
  apply: (value: T) => void,
  onError?: (error: AppError) => void,
): Promise<UnlistenFn> {
  let last: number | null = null;
  const offer = (value: T) => {
    if (last === null || isNewerRevision(value.revision, last)) {
      last = value.revision;
      apply(value);
    }
  };
  const unlisten = await on<T>(event, offer);
  try {
    offer(await getter());
  } catch (e) {
    onError?.(toAppError(e));
  }
  return unlisten;
}

export const subscribeSnapshot = (apply: (s: SessionSnapshot) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.sessionSnapshot, getSnapshot, apply, onError);
export const subscribeShields = (apply: (s: ShieldStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.shieldsStatus, getShieldStatus, apply, onError);
export const subscribeTeams = (apply: (s: TeamsStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.teamsStatus, getTeamsStatus, apply, onError);
export const subscribeMediaWindow = (apply: (s: MediaWindowStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.mediaWindow, getMediaWindow, apply, onError);
export const subscribeBlocklist = (apply: (b: BlocklistInfo) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.blocklistChanged, getBlocklist, apply, onError);
export const subscribePopup = (apply: (p: PopupInfo) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.popupShow, getPopup, apply, onError);
export const subscribeTint = (apply: (t: TintStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.tintChanged, getTintStatus, apply, onError);
export const subscribeCalendar = (apply: (c: CalendarStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.calendarChanged, getCalendar, apply, onError);
export const subscribeWork = (apply: (w: WorkStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.workChanged, getWork, apply, onError);
export const subscribePrivacy = (apply: (r: PrivacyReport) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.privacyChanged, getPrivacyReport, apply, onError);
export const subscribeYoutubeAccount = (apply: (a: YoutubeAccount) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.youtubeAccount, getYoutubeAccount, apply, onError);
/** Rewards view state (beta, ADR-022), revision-guarded like every other view state (ADR-012). */
export const subscribeRewards = (apply: (r: RewardsStatus) => void, onError?: (e: AppError) => void) =>
  subscribe(EVENTS.rewardsChanged, getRewards, apply, onError);

/** Raw `history://changed` (ADR-023): the stored history or the app labels changed (prefer `watchWeeklyStats`). */
export const onHistoryChanged = (handler: (c: HistoryChanged) => void) =>
  on<HistoryChanged>(EVENTS.historyChanged, handler);

/** Handle returned by `watchWeeklyStats`. */
export type WeeklyStatsWatch = {
  /** Reads `week()` again now (call it after the user picked another week). */
  refresh: () => void;
  stop: UnlistenFn;
};

/**
 * The ADR-012 protocol for a getter with a parameter (ADR-023): listens to `history://changed` first, then reads the
 * week `week()` names, and reads it again after every change. Only the answer to the latest request is applied, and
 * for the same week never one with an older `revision` than the last applied, so neither a slow answer for the
 * previous week nor a stale one can roll the view back.
 */
export async function watchWeeklyStats(
  week: () => string,
  apply: (stats: WeeklyStats) => void,
  onError?: (error: AppError) => void,
): Promise<WeeklyStatsWatch> {
  let latest = 0;
  let applied: { weekStart: string; revision: number } | null = null;
  const load = async () => {
    const request = ++latest;
    try {
      const stats = await getWeeklyStats(week());
      if (request !== latest) return;
      if (
        applied !== null &&
        applied.weekStart === stats.weekStart &&
        isNewerRevision(applied.revision, stats.revision)
      ) {
        return;
      }
      applied = { weekStart: stats.weekStart, revision: stats.revision };
      apply(stats);
    } catch (e) {
      if (request === latest) onError?.(toAppError(e));
    }
  };
  const stop = await onHistoryChanged(() => void load());
  await load();
  return { refresh: () => void load(), stop };
}

export const onPhaseChanged = (handler: (c: PhaseChange) => void) => on<PhaseChange>(EVENTS.sessionPhaseChanged, handler);
export const onSessionSummary = (handler: (s: SessionSummary) => void) => on<SessionSummary>(EVENTS.sessionSummary, handler);
export const onNudge = (handler: (n: Nudge) => void) => on<Nudge>(EVENTS.focusNudge, handler);
/** Raw `rewards://changed` (prefer `subscribeRewards`, which also reads the getter and drops stale revisions). */
export const onRewardsChanged = (handler: (r: RewardsStatus) => void) => on<RewardsStatus>(EVENTS.rewardsChanged, handler);
export const onAppError = (handler: (e: AppError) => void) =>
  on<unknown>(EVENTS.appError, (payload) => handler(toAppError(payload)));
/** The user pressed X on the main window while `closeToTray` is undecided: ask, then call `resolveClose`. */
export const listenCloseRequested = (handler: () => void) => on<null>(EVENTS.appCloseRequested, () => handler());

// ---------------------------------------------------------------------------
// Files, dialogs, opener, autostart
// ---------------------------------------------------------------------------

/** URL the WebView can load a local file from (asset protocol; the shell must allow the folder first). */
export function fileSrc(path: string): string {
  return inShell() ? convertFileSrc(path) : path;
}

/** Native folder picker. Resolves `null` when cancelled. */
export async function pickFolder(title: string, defaultPath?: string | null): Promise<string | null> {
  if (!inShell()) return null;
  try {
    const picked = await dialogOpen({ directory: true, multiple: false, title, defaultPath: defaultPath ?? undefined });
    return typeof picked === "string" ? picked : null;
  } catch (e) {
    throw toAppError(e);
  }
}

/** Extensions accepted by `importChime` (focus-core `CUSTOM_CHIME_EXTENSIONS`). */
export const CHIME_FILE_EXTENSIONS = ["mp3", "wav", "ogg", "flac", "m4a"] as const;

/** Native single-file picker limited to `extensions`. Resolves `null` when cancelled. */
export async function pickFile(title: string, filterName: string, extensions: readonly string[]): Promise<string | null> {
  if (!inShell()) return null;
  try {
    const picked = await dialogOpen({
      directory: false,
      multiple: false,
      title,
      filters: [{ name: filterName, extensions: [...extensions] }],
    });
    return typeof picked === "string" ? picked : null;
  } catch (e) {
    throw toAppError(e);
  }
}

/**
 * Native save dialog limited to `extensions`, starting with `defaultName` (e.g. `focusmode-export.json`).
 * Resolves `null` when cancelled.
 */
export async function pickSaveFile(
  title: string,
  defaultName: string,
  filterName: string,
  extensions: readonly string[],
): Promise<string | null> {
  if (!inShell()) return null;
  try {
    const picked = await dialogSave({
      title,
      defaultPath: defaultName,
      filters: [{ name: filterName, extensions: [...extensions] }],
    });
    return typeof picked === "string" ? picked : null;
  } catch (e) {
    throw toAppError(e);
  }
}

/** Opens an http(s) URL in the user's default browser. */
export async function openExternal(url: string): Promise<void> {
  if (!inShell()) {
    window.open(url, "_blank", "noopener");
    return;
  }
  try {
    await openerOpenUrl(url);
  } catch (e) {
    throw toAppError(e);
  }
}

export async function getAutostart(): Promise<boolean> {
  if (!inShell()) return false;
  try {
    return await autostartIsEnabled();
  } catch (e) {
    throw toAppError(e);
  }
}

export async function setAutostart(enabled: boolean): Promise<void> {
  if (!inShell()) return;
  try {
    if (enabled) await autostartEnable();
    else await autostartDisable();
  } catch (e) {
    throw toAppError(e);
  }
}
