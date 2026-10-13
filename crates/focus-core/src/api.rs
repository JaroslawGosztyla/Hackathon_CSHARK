//! IPC contract between the Svelte UI and the Tauri shell (ADR-006, ADR-012, ADR-013).
//!
//! Every DTO here is exported to TypeScript by ts-rs (`cargo test -p focus-core` writes
//! `src/lib/bindings/`). Command and event names are constants exported to
//! `src/lib/bindings/names.ts` by the `export_names` test, so `src/lib/api.ts` never types them by hand.
//! Change this file only through `/sync-contract`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tauri command names (`#[tauri::command]` function names in `src-tauri/src/commands.rs`).
pub mod commands {
    /// `() -> SessionSnapshot`
    pub const GET_SNAPSHOT: &str = "get_snapshot";
    /// `() -> ()` starts a session with the current config (or the demo profile).
    pub const START_SESSION: &str = "start_session";
    /// `() -> ()` freezes the remaining time of the current phase (Focus or Break).
    pub const PAUSE_SESSION: &str = "pause_session";
    /// `() -> ()`
    pub const RESUME_SESSION: &str = "resume_session";
    /// `() -> ()` idempotent, works in every phase.
    pub const STOP_SESSION: &str = "stop_session";
    /// `() -> ()` Focus → Break now, Break → next Focus now.
    pub const SKIP_PHASE: &str = "skip_phase";
    /// `() -> Config`
    pub const GET_CONFIG: &str = "get_config";
    /// `{ config: Config } -> Config` (normalized + validated + persisted)
    pub const UPDATE_CONFIG: &str = "update_config";
    /// `{ path: string } -> Track[]` also allows the folder in the asset protocol scope.
    pub const LIST_FOLDER_TRACKS: &str = "list_folder_tracks";
    /// `() -> ShieldStatus`
    pub const GET_SHIELD_STATUS: &str = "get_shield_status";
    /// `() -> UsageReport` usage of the running session (empty when idle).
    pub const GET_CURRENT_USAGE: &str = "get_current_usage";
    /// `() -> SessionSummary | null`
    pub const GET_LAST_SUMMARY: &str = "get_last_summary";
    /// `{ limit: number } -> SessionSummary[]` newest first.
    pub const LIST_HISTORY: &str = "list_history";
    /// `() -> TeamsStatus`
    pub const GET_TEAMS_STATUS: &str = "get_teams_status";
    /// `() -> DeviceCodePrompt` starts the device code login; progress arrives as `teams://status`.
    /// Asks for `Calendars.Read` too only while `calendar.microsoft` is on (also to reconnect a sign-in without it).
    pub const TEAMS_CONNECT: &str = "teams_connect";
    /// `() -> ()` forgets the stored token.
    pub const TEAMS_DISCONNECT: &str = "teams_disconnect";
    /// `() -> BridgeInfo`
    pub const GET_BRIDGE_INFO: &str = "get_bridge_info";
    /// `() -> ()` opens the bundled browser extension folder in the file manager.
    pub const OPEN_EXTENSION_DIR: &str = "open_extension_dir";
    /// `{ chime: ChimeKind } -> ()` plays a chime at the configured chime volume.
    pub const TEST_CHIME: &str = "test_chime";
    /// `{ url: string, play: boolean } -> ()` opens (or navigates) the separate YouTube window that plays
    /// the link as a top-level page. Used when a link refuses embedding (YouTube errors 101/150/153) or cannot be embedded.
    /// Only https YouTube / YouTube Music URLs are accepted (`media.url` error otherwise).
    pub const OPEN_MEDIA_WINDOW: &str = "open_media_window";
    /// `{ action: MediaAction, volume: number | null } -> ()` controls the YouTube window; no-op when it is closed.
    /// `volume` (0..=1) is used only by `setVolume`.
    pub const CONTROL_MEDIA_WINDOW: &str = "control_media_window";
    /// `() -> MediaWindowStatus`
    pub const GET_MEDIA_WINDOW: &str = "get_media_window";
    /// `() -> BlocklistInfo` the parsed `websiteblacklist.txt` (created from the bundled default on first run).
    pub const GET_BLOCKLIST: &str = "get_blocklist";
    /// `() -> ()` opens `websiteblacklist.txt` in the system's default text editor.
    pub const OPEN_BLOCKLIST_FILE: &str = "open_blocklist_file";
    /// `() -> BlocklistInfo` re-reads the file now (it is also reloaded automatically when it changes).
    pub const RELOAD_BLOCKLIST: &str = "reload_blocklist";
    /// `{ toTray: boolean } -> Config` answer to `app://close-requested` (the first X on the main window):
    /// stores `closeToTray` (and turns on `phasePopups` when `toTray`), then hides the window or quits.
    pub const RESOLVE_CLOSE: &str = "resolve_close";
    /// `() -> Language` the supported language closest to the OS UI language (`en` when none matches).
    pub const GET_SYSTEM_LANGUAGE: &str = "get_system_language";
    /// `() -> PopupInfo` what the phase popup window shows now (popup window only).
    pub const GET_POPUP: &str = "get_popup";
    /// `{ action: PopupAction } -> ()` a button in the phase popup (or its timeout: `dismiss`).
    pub const POPUP_ACTION: &str = "popup_action";
    /// `() -> TintStatus`
    pub const GET_TINT_STATUS: &str = "get_tint_status";
    /// `{ strength: number | null } -> ()` shows the warm tint at `strength` (0..=100) for a few seconds,
    /// ignoring the schedule (settings preview); `null` ends the preview at once.
    pub const PREVIEW_TINT: &str = "preview_tint";
    /// `{ path: string, chime: ChimeKind } -> Config` copies an audio file (mp3/wav/ogg/flac/m4a, at most 10 MB,
    /// must decode) into the app data folder as the custom sound for that chime and selects `chimeSound: custom`.
    pub const IMPORT_CHIME: &str = "import_chime";
    /// `{ chime: ChimeKind } -> Config` deletes the custom sound for that chime (falls back to the other custom file,
    /// or to the default sound when none is left).
    pub const REMOVE_CUSTOM_CHIME: &str = "remove_custom_chime";
    /// `() -> Track[]` the music bundled with the app (Open Lo-Fi, CC0), playable via `convertFileSrc(path)`.
    pub const LIST_LIBRARY_TRACKS: &str = "list_library_tracks";
    /// `() -> ()` deletes the whole session history (history file and the last summary). The UI confirms first.
    pub const CLEAR_HISTORY: &str = "clear_history";
    /// `() -> CalendarStatus` meetings of the next hours from all connected calendars and their collisions with
    /// the planned breaks (of the running session, or of a session that would start now).
    pub const GET_CALENDAR: &str = "get_calendar";
    /// `() -> CalendarStatus` fetches all calendars now.
    pub const REFRESH_CALENDAR: &str = "refresh_calendar";
    /// `{ meetingId: string, choice: MeetingChoice } -> CalendarStatus` how a meeting that collides with a break is handled.
    pub const SET_MEETING_CHOICE: &str = "set_meeting_choice";
    /// `() -> ()` starts the Google sign-in in the system browser (OAuth loopback + PKCE); progress via `calendar://changed`.
    pub const CONNECT_GOOGLE: &str = "connect_google";
    /// `{ source: CalendarSource } -> CalendarStatus` forgets the stored credentials / links of that source.
    /// `microsoft` keeps the Teams sign-in (Teams presence goes on): it only turns `calendar.microsoft` off
    /// (config saved as by `update_config`) and drops the Microsoft meetings.
    pub const DISCONNECT_CALENDAR: &str = "disconnect_calendar";
    /// `{ urls: string[] } -> CalendarStatus` replaces the ICS subscription links (stored encrypted, never in config.json).
    pub const SET_ICS_URLS: &str = "set_ics_urls";
    /// `() -> string[]` the stored ICS links (to edit them in Settings).
    pub const GET_ICS_URLS: &str = "get_ics_urls";
    /// `() -> WorkStatus` ranked work items ("Co dalej?") from the connected tools.
    pub const GET_WORK: &str = "get_work";
    /// `() -> WorkStatus` fetches all connected tools now.
    pub const REFRESH_WORK: &str = "refresh_work";
    /// `{ source: WorkSource, token: string | null } -> WorkStatus` stores (encrypted) or forgets the access token.
    pub const SET_WORK_TOKEN: &str = "set_work_token";
    /// `{ goal: SessionGoal | null } -> ()` the item the user focuses on (shown under the timer, kept in the summary).
    pub const SET_SESSION_GOAL: &str = "set_session_goal";
    /// `{ sessionId: string, done: boolean } -> ()` marks the goal of a finished session as done / not done (history).
    pub const MARK_GOAL_DONE: &str = "mark_goal_done";
    /// `{ action: DevAction } -> ()` developer page trigger (ADR-019): a sample popup, a phase toast, a shorter
    /// running phase, presentation mode on / off or a factory reset (then the app restarts). Rejected with
    /// `developerModeOff` unless `Config.developer_mode` is on. Nothing leaves the computer; only the factory reset
    /// changes files (it deletes the app's own data).
    pub const DEV_TRIGGER: &str = "dev_trigger";
    /// `() -> PrivacyReport` what FocusMode stores on this computer, item by item (ADR-020). Presence, sizes and
    /// counts only: never the contents of a token, an ICS link or a cookie.
    pub const GET_PRIVACY_REPORT: &str = "get_privacy_report";
    /// `() -> ()` opens the app data folder (`PrivacyReport.folders[0]`) in the file manager.
    pub const OPEN_DATA_FOLDER: &str = "open_data_folder";
    /// `{ path: string } -> ()` writes one JSON file (path from the save dialog): config without secrets, history,
    /// meeting choices and seen work items. Never tokens, ICS links, cookies or other secrets.
    pub const EXPORT_PRIVACY_DATA: &str = "export_privacy_data";
    /// `{ kinds: PrivacyItemKind[] } -> Config` deletes those items (only `deletable` ones, else `privacy.notDeletable`).
    /// A running session is stopped first (shields restored, rule 7). Returns the stored config (deleting the custom
    /// chimes changes it); the new report arrives as `privacy://changed`.
    pub const DELETE_PRIVACY_DATA: &str = "delete_privacy_data";
    /// `() -> Config` "Delete everything": stops a running session, deletes every deletable item and resets the
    /// settings to the defaults (returned). Nothing outside the app's own folders is touched; the app stays installed.
    pub const DELETE_ALL_DATA: &str = "delete_all_data";
    /// `() -> YoutubeAccount` (ADR-021).
    pub const GET_YOUTUBE_ACCOUNT: &str = "get_youtube_account";
    /// `() -> ()` opens (or focuses) the visible YouTube sign-in window (Google's own page, shared WebView2 profile);
    /// it closes by itself once youtube.com sign-in cookies appear. Progress via `youtube://account`.
    pub const YOUTUBE_SIGN_IN: &str = "youtube_sign_in";
    /// `() -> ()` deletes the google.com / youtube.com cookies of the WebView2 profile (the consent cookies stay).
    pub const YOUTUBE_SIGN_OUT: &str = "youtube_sign_out";
    /// `() -> RewardsStatus` sparks, daily goal, level, streak and pending moments (beta, ADR-022). Main window.
    pub const GET_REWARDS: &str = "get_rewards";
    /// `{ upTo: number } -> ()` removes the pending moments with `id <= upTo` (they were played or shown).
    /// Main window only: the popup never acks.
    pub const ACK_REWARD_MOMENTS: &str = "ack_reward_moments";
    /// `() -> RewardsStatus` "Zacznij od nowa": a new ledger with a new seed; the demo preview is dropped.
    /// The UI confirms first. Session history is not touched.
    pub const RESET_REWARDS: &str = "reset_rewards";
    /// `{ sound: RewardSound } -> ()` plays a reward sound. No-op while rewards are inactive, reward sounds are off,
    /// celebrations are `numbersOnly` or the user is presenting; waits until a phase chime has finished.
    pub const PLAY_REWARD_SOUND: &str = "play_reward_sound";
    /// `{ steps: StepDraft[] } -> ()` replaces the small steps of the coming or running session (any phase).
    /// Errors: `stepsOff` while small steps are off, `tooManySteps` (field `steps`) above 5. The new plan arrives in
    /// `session://snapshot`. Step texts are never logged.
    pub const SET_SESSION_STEPS: &str = "set_session_steps";
    /// `{ id: number, done: boolean } -> ()` ticks / unticks a step (Focus or Break, also paused). Idle or an
    /// unknown id: no-op. `stepsOff` while small steps are off.
    pub const SET_STEP_DONE: &str = "set_step_done";
    /// `() -> SunCity[]` the built-in cities for the sunset / sunrise tint schedule (ADR-023), in list order.
    /// Static data compiled into the app: no location service, no network.
    pub const LIST_SUN_CITIES: &str = "list_sun_cities";
    /// `{ meetingId: string, optOut: boolean } -> CalendarStatus` "Ask me instead" for one meeting (ADR-023):
    /// `true` undoes the automatic smart-break fit for it (the session gets its unadjusted phase end back through
    /// the actor, and the meeting is asked about like any collision); `false` lets smart breaks fit it again.
    /// Stored with the meeting choices (ids only). Never blocks Stop / Skip / Pause.
    pub const SET_SMART_BREAK_OPT_OUT: &str = "set_smart_break_opt_out";
    /// `{ weekStart: string } -> WeeklyStats` the ISO week (Monday first, local time) containing the date
    /// `weekStart` (`YYYY-MM-DD`, any day of that week), aggregated from `history.jsonl` (ADR-023).
    /// A malformed date → `stats.week` (field `weekStart`).
    pub const GET_WEEKLY_STATS: &str = "get_weekly_stats";
}

/// Event names (`emit` from the shell, `listen` in `api.ts`).
pub mod events {
    /// `SessionSnapshot`, every tick (1 s) and after every change.
    pub const SESSION_SNAPSHOT: &str = "session://snapshot";
    /// `PhaseChange`, on every phase transition.
    pub const SESSION_PHASE_CHANGED: &str = "session://phase-changed";
    /// `SessionSummary`, once when a session ends (Stop).
    pub const SESSION_SUMMARY: &str = "session://summary";
    /// `ShieldStatus`, whenever an integration changes state.
    pub const SHIELDS_STATUS: &str = "shields://status";
    /// `TeamsStatus`, whenever the Teams connection changes.
    pub const TEAMS_STATUS: &str = "teams://status";
    /// `Nudge`, a blocked site was detected in the foreground browser window (fallback without the extension).
    pub const FOCUS_NUDGE: &str = "focus://nudge";
    /// `AppError`, an integration failed; the session keeps running (degradation).
    pub const APP_ERROR: &str = "app://error";
    /// `MediaWindowStatus`, whenever the YouTube window opens, closes, navigates or changes its title.
    pub const MEDIA_WINDOW: &str = "media://window";
    /// `BlocklistInfo`, whenever `websiteblacklist.txt` was (re)loaded with a different result.
    pub const BLOCKLIST_CHANGED: &str = "blocklist://changed";
    /// `null`, sent to the main window when the user pressed X and `closeToTray` is still undecided:
    /// the UI asks and answers with `resolve_close`.
    pub const APP_CLOSE_REQUESTED: &str = "app://close-requested";
    /// `PopupInfo`, the phase popup window should show (or hide, `kind: null`) a message.
    pub const POPUP_SHOW: &str = "popup://show";
    /// `TintStatus`, whenever the tint level, preview or overlay state changes.
    pub const TINT_CHANGED: &str = "tint://changed";
    /// `CalendarStatus`, after every sync, choice or connection change.
    pub const CALENDAR_CHANGED: &str = "calendar://changed";
    /// `WorkStatus`, after every sync or token change.
    pub const WORK_CHANGED: &str = "work://changed";
    /// `PrivacyReport`, whenever something stored changed (debounced rescan) or a delete / export starts or ends.
    pub const PRIVACY_CHANGED: &str = "privacy://changed";
    /// `YoutubeAccount`, whenever the sign-in state or the sign-in window changes.
    pub const YOUTUBE_ACCOUNT: &str = "youtube://account";
    /// `RewardsStatus` (beta, ADR-022), after every applied fact, ack, reset, config change, reward-day rollover and
    /// load. Never emitted with `active: true` while the rewards are off.
    pub const REWARDS_CHANGED: &str = "rewards://changed";
    /// `HistoryChanged` (ADR-023), whenever the stored history changed (a session was saved, a goal marked, the
    /// history cleared or deleted) or `Config.app_categories` changed: the UI asks `get_weekly_stats` again for the
    /// week it shows and keeps answers whose `revision` is not older.
    pub const HISTORY_CHANGED: &str = "history://changed";
}

/// Loopback port of the browser-extension bridge (`GET http://127.0.0.1:{BRIDGE_PORT}/v1/state` → `BridgeState`).
pub const BRIDGE_PORT: u16 = 47615;

/// Error returned by every command and sent as `app://error`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppError {
    /// Stable machine code, e.g. `config.invalid`, `audio.device`, `shields.notifications`, `teams.auth`.
    pub code: String,
    /// Human readable detail (English, for logs); the UI maps `code` to Polish text.
    pub message: String,
    /// Config field that failed validation (`focusMinutes`, `music.volume`, ...), if any.
    pub field: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Phase {
    Idle,
    Focus,
    Break,
}

/// Full view state of the session. Windows apply it only when `revision` is newer than what they have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionSnapshot {
    /// Set by the shell; grows with every emitted snapshot.
    pub revision: u32,
    pub session_id: Option<String>,
    pub phase: Phase,
    /// Paused by the user or by a detected sleep/clock gap. Only meaningful in Focus/Break.
    pub paused: bool,
    /// 1-based number of the current focus cycle (0 when idle).
    pub cycle: u32,
    /// Seconds left in the current phase (frozen while paused).
    pub remaining_secs: u32,
    /// Length of the current phase in seconds (0 when idle).
    pub phase_total_secs: u32,
    /// Profile in use (from config, or the demo profile).
    pub focus_secs: u32,
    pub break_secs: u32,
    /// Focus time actually spent in this session (no pauses, no gaps).
    pub focused_secs: u32,
    /// Demo profile active (`--demo` / `FOCUSMODE_DEMO=1`).
    pub demo: bool,
    /// Music must be silent now (focus extended through a meeting).
    #[serde(default)]
    pub music_muted: bool,
    /// A meeting currently extends the focus or holds the break.
    #[serde(default)]
    pub meeting_hold: Option<MeetingHoldView>,
    /// What the user focuses on in this session (from "Co dalej?").
    #[serde(default)]
    pub goal: Option<SessionGoal>,
    /// Small steps of the coming (Idle) or running session (beta, ADR-022). Empty while small steps are off.
    #[serde(default)]
    pub steps: Vec<MicroStep>,
    /// Smart breaks (ADR-023): the break after the current focus block (or the running break) was moved around a
    /// meeting; the one-line note under the timer ("Break moved to 13:50 · meeting at 14:00") and its undo. No
    /// meeting subject. `None` in Idle, while `Config.smart_breaks` is off and when nothing was moved.
    #[serde(default)]
    pub smart_break: Option<SmartBreak>,
}

/// A small step of the session plan (beta, ADR-022). `text` is the user's own text: it stays local (memory, the IPC
/// snapshot, the popup, `history.jsonl` only with `keepHistory`) and is never logged: `Debug` prints its length only.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MicroStep {
    /// Stable within the session plan (kept across Start and the carry-over at Stop).
    pub id: u32,
    /// Trimmed, at most 80 characters.
    pub text: String,
    pub done: bool,
}

impl std::fmt::Debug for MicroStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicroStep")
            .field("id", &self.id)
            .field("text", &RedactedText(&self.text))
            .field("done", &self.done)
            .finish()
    }
}

/// One step as the UI edits it (`set_session_steps`): `id` = an existing step (it keeps its `done`), `None` = a new one.
/// `Debug` prints the text length only.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StepDraft {
    pub id: Option<u32>,
    pub text: String,
}

impl std::fmt::Debug for StepDraft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepDraft")
            .field("id", &self.id)
            .field("text", &RedactedText(&self.text))
            .finish()
    }
}

/// Prints `"<n chars>"` instead of the user's text (rule 11: step texts never reach the log).
struct RedactedText<'a>(&'a str);

impl std::fmt::Debug for RedactedText<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"<{} chars>\"", self.0.chars().count())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PhaseChange {
    pub from: Phase,
    pub to: Phase,
    pub cycle: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChimeKind {
    /// Focus ended, break begins.
    BreakStart,
    /// Break ended (or session started), focus begins.
    FocusStart,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Config {
    /// Focus length in minutes, 1..=240. Default 45.
    pub focus_minutes: u32,
    /// Break length in minutes, 1..=120. Default 15.
    pub break_minutes: u32,
    pub music: MusicConfig,
    pub shields: ShieldsConfig,
    /// Chime volume 0..=1. Default 0.7.
    pub chime_volume: f32,
    /// Track the foreground app during focus for the productivity chart. Default true.
    pub track_apps: bool,
    /// Start FocusMode when the user logs in (autostart plugin). Default false.
    pub launch_on_login: bool,
    /// UI, tray, popup and notification language. Default `System` (the OS UI language).
    pub language: Language,
    /// X on the main window: `Some(true)` hides it to the tray, `Some(false)` quits the app,
    /// `None` (default) asks once, at the first X (`app://close-requested`).
    pub close_to_tray: Option<bool>,
    /// Small FocusMode popups (bottom-right, app-styled) when focus or a break starts, instead of
    /// Windows toasts (never both; a toast only when this is off or the popup window cannot be shown). Default true
    /// (a missing field too, like every field); also turned on when the user picks the tray at the first X.
    pub phase_popups: bool,
    /// Warm screen tint by time of day. Off by default.
    pub tint: TintConfig,
    /// App look. Default `Midnight`.
    pub theme: Theme,
    /// Sound of the break / focus chimes. Default `Midnight`.
    pub chime_sound: ChimeSound,
    /// Custom chime files (names inside the app data `sounds` folder) used when `chime_sound` is `Custom`.
    pub custom_chimes: CustomChimes,
    /// Optional reminder per break: index 0 = first break of a session, 1 = second, …; "" = no reminder for
    /// that break. At most 24 entries of at most 160 characters each.
    pub break_reminders: Vec<String>,
    /// After the last reminder, start the list again from the first one. Default false.
    pub repeat_reminders: bool,
    /// Also keep the most-used window titles per app for the productivity chart (local only). Default true.
    pub track_titles: bool,
    /// Save finished sessions to the history file. Default true; when off, the summary is shown but not stored.
    pub keep_history: bool,
    /// Calendar integrations (meeting / break collisions). All off by default.
    pub calendar: CalendarConfig,
    /// Developer tools for "Co dalej?". All off by default; tokens are stored separately (encrypted).
    pub work: WorkConfig,
    /// Show the axolotl mascot (main window, break card, popups). Default true.
    pub show_mascot: bool,
    /// Developer page with test triggers (sample popups, toasts, mascot, a shorter phase) for screenshots (ADR-019).
    /// Default false; an older `config.json` without it loads with it off.
    pub developer_mode: bool,
    /// Master switch for beta features (ADR-022). Default false; an older `config.json` loads with it off.
    pub beta_features: bool,
    /// Sparks, streak and small steps (ADR-022). Effective only while `beta_features` is on.
    pub rewards: RewardsConfig,
    /// Smart breaks (ADR-023): while a calendar is connected, a meeting that collides with a break moves that break
    /// by shortening or lengthening the focus block before it (within the bounds of ADR-023) instead of asking; when
    /// no fit is possible the meeting is asked about as before. Default false.
    pub smart_breaks: bool,
    /// The user's labels for apps in the weekly stats (ADR-023): exe name → category. Keys are stored trimmed and
    /// lowercased (`str::to_lowercase`, JS `toLowerCase()`); an app without a key is unlabelled. Default empty;
    /// at most `config::MAX_APP_CATEGORIES` entries.
    pub app_categories: BTreeMap<String, AppCategory>,
    /// The first-run wizard was finished or skipped (ADR-023). Default true, so a `config.json` without the field
    /// (every install before the wizard) never shows it; only the config the shell creates on a fresh install
    /// (`Config::first_run`) starts with false. "Delete everything" resets to the defaults, so it stays true there.
    pub onboarding_done: bool,
    /// Beta collection theme (ADR-024), picked in Settings → "Kolekcja". It replaces `theme` on screen only while
    /// `beta_features` is on and the theme is unlocked in `rewards.json`; `theme` always stays the base theme (the one
    /// shown without the beta, and the "last base theme" a locked or hidden collection theme falls back to). Default
    /// `None`. `update_config` refuses to set a theme that is not unlocked (`config.invalid`, field `rewardTheme`);
    /// "Zacznij od nowa" and deleting the rewards in the privacy panel set it back to `None`.
    pub reward_theme: Option<RewardTheme>,
    /// Beta ambience mixer (ADR-024): synthesized background layers over the music. Effective only while
    /// `beta_features` is on; a layer plays only when it is free or unlocked.
    pub ambience: AmbienceConfig,
}

/// Beta: sparks, levels, daily goal, streak, celebrations and small steps (ADR-022). Every switch is effective only
/// while `Config.beta_features` is on; turning anything off hides its UI and keeps the data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct RewardsConfig {
    /// Sparks, levels, daily goal, streak and celebrations. Default true.
    pub enabled: bool,
    /// Small steps in a session (works without `enabled` too). Default true.
    pub micro_steps: bool,
    /// Surprise rolls after full blocks. Default true.
    pub surprises: bool,
    /// Reward sounds (pop, sparkle, fanfare). Default true.
    pub sounds: bool,
    /// Default `Full`; the OS reduced-motion setting caps the visuals at `Calm`.
    pub celebrations: CelebrationLevel,
    /// P1: the axolotl's back-view pose while focus runs. Default false (owner decides before it is on by default).
    pub focus_companion: bool,
    /// UI only: hide the streak (it keeps being computed). Default true.
    pub show_streak: bool,
    /// Holiday / rough week: days while paused are neutral for the streak. Default false.
    pub streak_paused: bool,
    /// Default `Light` (30 sparks).
    pub daily_goal: DailyGoal,
    /// Days off (neutral for the streak). Default Sat + Sun; deduplicated and sorted Mon..Sun; at most 6.
    pub rest_weekdays: Vec<Weekday>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

/// Daily goal preset in sparks (`focus_core::rewards::DAILY_GOAL_SPARKS`): 30 / 60 / 120.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DailyGoal {
    Light,
    Regular,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CelebrationLevel {
    /// Count-up, poses, confetti, one sound per celebration.
    Full,
    /// The same moments as still frames: no particles, floats, count-up or flips.
    Calm,
    /// Numbers and lists only: no pose changes, no sounds.
    NumbersOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct WorkConfig {
    pub github: bool,
    pub gitlab: bool,
    /// GitLab instance. Default `https://gitlab.com`.
    pub gitlab_url: String,
    pub jira: bool,
    /// Jira site, e.g. `https://firma.atlassian.net` or a Data Center URL.
    pub jira_url: String,
    /// Account e-mail for Jira Cloud API tokens (empty = Data Center personal access token).
    pub jira_email: String,
    pub azure_devops: bool,
    /// Azure DevOps organisation, e.g. `firma` (https://dev.azure.com/firma).
    pub azure_org: String,
    pub linear: bool,
    /// Hold new notifications during focus and show them at the break. Default true.
    pub hold_during_focus: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CalendarConfig {
    /// Microsoft 365 / Outlook calendar through the Microsoft account used for Teams (Graph, Calendars.Read;
    /// the Teams sign-in asks for that scope only while this is on).
    pub microsoft: bool,
    /// Google Calendar (OAuth desktop flow); needs `google_client_id`.
    pub google: bool,
    /// OAuth client id of a Google Cloud "Desktop app" client.
    pub google_client_id: String,
    /// Client secret of the same "Desktop app" client, or empty. Google's token endpoint requires it for such
    /// clients; Google documents it as non-confidential for installed apps, so it lives in `config.json`
    /// (never in the log). Default "".
    pub google_client_secret: String,
    /// ICS / iCal subscription links (stored separately, encrypted).
    pub ics: bool,
    /// How far ahead meetings are read, 1..=24 hours. Default 10.
    pub horizon_hours: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Theme {
    Midnight,
    Vaporwave,
    Lunar,
    Autumn,
    Dark,
    Light,
    Pixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChimeSound {
    /// Soft bell (the original chime).
    Midnight,
    /// Detuned synth chord.
    Vaporwave,
    /// Glassy crystal tones.
    Lunar,
    /// Warm wooden marimba.
    Autumn,
    /// Two-tone ding-dong.
    Classic,
    /// Gentle sine tone.
    Soft,
    /// 8-bit square-wave arpeggio.
    Chiptune,
    /// The user's own files (`custom_chimes`).
    Custom,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CustomChimes {
    /// File name in the app data `sounds` folder used when a break starts.
    pub break_file: Option<String>,
    /// File name used when focus starts; `None` = use `break_file` for both.
    pub focus_file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Language {
    /// Follow the OS UI language (`en` when it is not one of the supported languages).
    System,
    En,
    Pl,
    De,
    Fr,
    It,
    Uk,
    Ru,
    Es,
    Zh,
    Ja,
}

/// Warm screen schedule. Times are minutes after local midnight; the schedule may cross midnight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TintConfig {
    /// Default false.
    pub enabled: bool,
    /// The tint starts fading in here, 0..1440. Default 1080 (18:00).
    pub start_minutes: u32,
    /// The screen is back to normal here, 0..1440, different from `start_minutes`. Default 540 (09:00).
    pub end_minutes: u32,
    /// Full tint strength 0..=100. Default 45.
    pub strength: u32,
    /// Fade length in minutes after `start_minutes` and before `end_minutes`, 0..=180. Default 30.
    pub transition_minutes: u32,
    /// How the warmth is applied. Default `NightLight`.
    pub mode: TintMode,
    /// What opens and closes the warm window (ADR-023): the fixed hours above or local sunset → sunrise. Default
    /// `Fixed`. Without a `location`, and on days without a sunset or sunrise (polar day / night), the fixed hours
    /// are used (`TintStatus.schedule` says which).
    pub schedule: TintSchedule,
    /// Where sunset and sunrise are computed for `Sun`: typed by the user or picked from `list_sun_cities`. Never
    /// located automatically (no location service, no network). Default `None`.
    pub location: Option<TintLocation>,
    /// Suspend the warmth while one of `pause_apps` is the foreground window (ADR-023). Default true.
    pub pause_for_apps: bool,
    /// Colour-critical apps: exact exe names (`Photoshop.exe`), matched case-insensitively. Stored trimmed, without
    /// empty entries and case-insensitive duplicates (the first spelling stays), at most `config::MAX_PAUSE_APPS`
    /// names of at most `config::MAX_PROCESS_NAME_CHARS` characters, no path separators. Default
    /// `tint::DEFAULT_PAUSE_APPS`.
    pub pause_apps: Vec<String>,
    /// Suspend the warmth while a full-screen app or game is in front (ADR-023). Default false.
    pub pause_for_fullscreen: bool,
}

/// What opens and closes the warm window (ADR-023).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TintSchedule {
    /// `TintConfig.start_minutes` → `end_minutes`.
    Fixed,
    /// Local sunset → sunrise at `TintConfig.location`, computed offline (NOAA solar algorithm in `focus-core`).
    Sun,
}

/// A place on Earth for the sunset / sunrise schedule (ADR-023). Stays in `config.json`; never sent anywhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TintLocation {
    /// Degrees, -90..=90 (north positive).
    pub latitude: f64,
    /// Degrees, -180..=180 (east positive).
    pub longitude: f64,
    /// `SunCity.id` when the place was picked from the built-in list (the UI shows that city); `None` for typed
    /// coordinates. Stored trimmed (empty → `None`), at most `config::MAX_CITY_ID_CHARS` characters; not checked
    /// against the list (an id the list no longer has shows as typed coordinates).
    pub city: Option<String>,
}

/// One built-in city for the sunset / sunrise schedule (`list_sun_cities`, ADR-023).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SunCity {
    /// Stable id, e.g. `pl-warszawa`.
    pub id: String,
    /// The city's own name (`Warszawa`, `München`, `Tokyo`); the UI may add the localized country.
    pub name: String,
    /// ISO 3166-1 alpha-2 country code (`PL`), for `Intl.DisplayNames` in the UI.
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TintMode {
    /// Windows Night light, switched and set by FocusMode on its schedule (per-user setting, no admin).
    NightLight,
    /// Colour temperature through each display's gamma ramp (whole screen incl. taskbar and fullscreen
    /// games; not on HDR displays; Windows may cap very warm values).
    Gamma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GameLauncher {
    Steam,
    Epic,
    Gog,
    Ubisoft,
    Ea,
    BattleNet,
    Riot,
    Xbox,
    Amazon,
    Itch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MusicSourceKind {
    None,
    /// Generated noise (WebAudio), works offline.
    Noise,
    /// Local folder with audio files.
    Folder,
    /// Playlist of links: YouTube / YouTube Music videos and playlists, direct audio URLs.
    Links,
    /// One live stream: YouTube live (e.g. Lofi Girl) or an internet radio URL.
    Stream,
    /// Music bundled with the app (Open Lo-Fi, CC0), works offline. Default source.
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NoiseColor {
    Brown,
    Pink,
    White,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MusicConfig {
    pub source: MusicSourceKind,
    pub folder: Option<String>,
    pub links: Vec<String>,
    pub stream_url: String,
    pub noise: NoiseColor,
    pub shuffle: bool,
    /// User volume 0..=1.
    pub volume: f32,
    /// Default false: music keeps playing during the break.
    pub pause_during_break: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ShieldsConfig {
    /// Turn on Windows "Do not disturb" / Focus Assist during focus. Default false.
    pub mute_notifications: bool,
    /// Set Microsoft Teams presence to Do not disturb during focus (Microsoft Graph). Default false.
    pub teams_dnd: bool,
    /// Azure app registration (public client, `Presence.ReadWrite`) used for Teams.
    pub teams_client_id: String,
    /// Tenant id or `organizations` / `common`. Default `organizations`.
    pub teams_tenant: String,
    /// Block distracting sites during focus (browser extension + window-title fallback). Default true.
    /// The list itself lives in `websiteblacklist.txt` (see `BlocklistInfo`), not in the config.
    pub block_sites: bool,
    /// Keep the display and system awake during focus. Default true.
    pub keep_awake: bool,
    /// Close the selected game launchers whenever a focus phase starts. Default false.
    pub close_launchers: bool,
    /// Launchers to close. Default: Steam, Epic, GOG, Ubisoft, EA, Battle.net, Riot.
    pub launchers: Vec<GameLauncher>,
}

// ---------------------------------------------------------------------------
// Music
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Track {
    /// Absolute path; the UI plays it via `convertFileSrc(path)`.
    pub path: String,
    /// File name without extension.
    pub title: String,
}

// ---------------------------------------------------------------------------
// Shields (notifications, Teams, sites, keep awake)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum IntegrationState {
    /// Disabled in config or not wanted right now.
    Off,
    /// Active.
    On,
    /// Not supported on this OS / not configured.
    Unavailable,
    /// Last attempt failed; see `detail`.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IntegrationStatus {
    pub state: IntegrationState,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShieldStatus {
    pub revision: u32,
    /// The session currently wants shields up (Focus, including paused Focus).
    pub active: bool,
    pub notifications: IntegrationStatus,
    pub teams: IntegrationStatus,
    pub sites: IntegrationStatus,
    pub keep_awake: IntegrationStatus,
    /// Closing game launchers at focus start.
    pub launchers: IntegrationStatus,
    /// Launchers closed at the last focus start (empty when none were running).
    pub closed_launchers: Vec<GameLauncher>,
    /// The browser extension polled the bridge in the last 90 s.
    pub extension_connected: bool,
}

// ---------------------------------------------------------------------------
// Productivity tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppUsage {
    /// Process executable name, e.g. `Code.exe`, `chrome.exe`.
    pub process: String,
    pub secs: u32,
    /// Most-used window titles of this app (at most 5, longest first), only when `track_titles` is on.
    /// Shown as a tooltip in the chart; stays on this computer, never logged.
    #[serde(default)]
    pub titles: Vec<TitleUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TitleUsage {
    /// Window title, trimmed and shortened to 120 characters.
    pub title: String,
    pub secs: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UsageReport {
    /// Sorted by `secs` descending, then by name.
    pub apps: Vec<AppUsage>,
    /// Focus time with no keyboard/mouse input for longer than the idle threshold.
    pub idle_secs: u32,
    /// Focus time where the foreground process could not be determined.
    pub unknown_secs: u32,
    /// Sum of all apps + idle + unknown.
    pub tracked_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionSummary {
    pub session_id: String,
    /// Unix epoch milliseconds (wall clock).
    #[ts(type = "number")]
    pub started_at_ms: i64,
    #[ts(type = "number")]
    pub ended_at_ms: i64,
    pub focused_secs: u32,
    pub break_secs: u32,
    /// Focus phases that ran to their end (or were skipped into a break).
    pub cycles_completed: u32,
    pub usage: UsageReport,
    pub demo: bool,
    #[serde(default)]
    pub goal: Option<SessionGoal>,
    /// Set by the user in the summary ("Zrobione?").
    #[serde(default)]
    pub goal_done: Option<bool>,
    /// Small steps of this session with their ticks (beta, ADR-022). Left out of the JSON when empty, so a
    /// `history.jsonl` line without steps stays as before (the TS field is optional: `undefined` = none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<MicroStep>,
    /// Sparks earned by this session (beta; `None` while rewards were off). Numbers only. Left out of the JSON when
    /// `None` (the TS field is optional: `undefined` or `null` = none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sparks: Option<u32>,
    /// Wall-clock intervals of running focus in this session (no pauses, no sleep gaps), oldest first, for the
    /// weekly heatmaps (ADR-023). Timestamps only. Left out of the JSON when empty, so lines written before stay
    /// as they were (TS field optional); for those the weekly stats spread `focused_secs` evenly between
    /// `started_at_ms` and `ended_at_ms` (`WeeklyStats.estimated`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_spans: Vec<TimeSpan>,
}

/// `[start_ms, end_ms)` on the wall clock (Unix epoch milliseconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TimeSpan {
    #[ts(type = "number")]
    pub start_ms: i64,
    #[ts(type = "number")]
    pub end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Nudge {
    /// Browser process, e.g. `chrome.exe`.
    pub process: String,
    /// Blocklist entry that matched: a domain (`reddit.com`) or a regex source.
    pub matched: String,
    /// Window title that matched.
    pub title: String,
}

// ---------------------------------------------------------------------------
// Teams
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceCodePrompt {
    pub user_code: String,
    pub verification_uri: String,
    /// Message from Microsoft identity platform (English).
    pub message: String,
    pub expires_in_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TeamsStatus {
    pub revision: u32,
    /// `teams_client_id` is set in config.
    pub configured: bool,
    /// A refresh token is stored and still works.
    pub connected: bool,
    /// Signed-in account (UPN), if known.
    pub account: Option<String>,
    /// Device code login in progress.
    pub pending: Option<DeviceCodePrompt>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// YouTube media window (fallback for links that refuse embedding)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MediaAction {
    Play,
    Pause,
    SetVolume,
    /// Shows, unminimizes and focuses the window (e.g. to accept the YouTube consent page).
    Show,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MediaWindowStatus {
    /// Set by the shell; grows with every emitted status.
    pub revision: u32,
    pub open: bool,
    /// URL the window was last asked to open (not the page it navigated to).
    pub url: Option<String>,
    /// Page title without the " - YouTube" / " - YouTube Music" suffix, if known.
    pub title: Option<String>,
}

// ---------------------------------------------------------------------------
// Site blocklist (websiteblacklist.txt)
// ---------------------------------------------------------------------------

/// A line of `websiteblacklist.txt` that was skipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlocklistIssue {
    /// 1-based line number.
    pub line: u32,
    /// The line as written (trimmed, shortened to 200 characters).
    pub text: String,
    /// What is wrong; the UI shows a localized explanation for it.
    pub code: BlocklistIssueCode,
    /// Extra detail (English), e.g. the regex parser error or the suggested domain; may be empty.
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BlocklistIssueCode {
    InvalidDomain,
    InvalidRegex,
    RegexTooComplex,
    MatchesEverything,
    /// A pasted address with `?` that, read as a regex, does not match itself; `message` holds the domain.
    PastedUrl,
    LineTooLong,
    TooManyDomains,
    TooManyPatterns,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlocklistInfo {
    /// Set by the shell; grows with every emitted info.
    pub revision: u32,
    /// Absolute path of `websiteblacklist.txt`.
    pub path: String,
    /// Normalized domains (plain lines), in file order without duplicates.
    pub domains: Vec<String>,
    /// Regex sources (lines in `/…/` or with regex characters), in file order without duplicates.
    pub patterns: Vec<String>,
    /// Skipped lines (invalid domain, invalid or too complex regex, limits exceeded).
    pub issues: Vec<BlocklistIssue>,
    /// The file could not be read; the previous list (or the bundled default) stays in use.
    pub error: Option<String>,
    /// Regex sources the browser extension reported it could not enforce as network rules (Chrome's RE2
    /// limits); they are still checked when a tab loads. Empty when no extension reported.
    #[serde(default)]
    pub extension_skipped: Vec<String>,
}

// ---------------------------------------------------------------------------
// Phase popups and screen tint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PopupKind {
    BreakStarted,
    FocusStarted,
    /// The session was paused because the computer slept.
    Interrupted,
    /// Game launchers were closed at focus start (`launchers`).
    LaunchersClosed,
    /// A meeting collides with a break: ask how to handle it (`meeting`).
    MeetingReminder,
    /// The break started paused because of a meeting (`meeting`).
    BreakHeld,
    /// New work items arrived during focus (`work_count`).
    WorkDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PopupInfo {
    /// Set by the shell; grows with every popup.
    pub revision: u32,
    /// `None` = nothing to show (hide).
    pub kind: Option<PopupKind>,
    pub cycle: u32,
    /// Length of the phase that just started, in seconds.
    pub phase_secs: u32,
    pub launchers: Vec<GameLauncher>,
    /// The user's own reminder for the break that just started (`break_reminders`), if any.
    pub reminder: Option<String>,
    /// The meeting for `MeetingReminder` / `BreakHeld`.
    #[serde(default)]
    pub meeting: Option<MeetingConflict>,
    /// Number of new work items for `WorkDigest`.
    #[serde(default)]
    pub work_count: u32,
    /// Developer-mode sample that stays until closed (ADR-019): no auto-hide countdown in the popup page and no
    /// timeout in the shell. Always `false` for real popups.
    #[serde(default)]
    pub sticky: bool,
    /// `BreakStarted` / `BreakHeld` / `FocusStarted` while rewards are active: the reward line (beta, ADR-022).
    #[serde(default)]
    pub rewards: Option<PopupRewards>,
    /// `FocusStarted` after a break while small steps are active: the next open step ("Teraz: …"). The user's own
    /// text: local only, never logged.
    #[serde(default)]
    pub next_step: Option<String>,
}

/// The reward line of a phase popup (beta, ADR-022): `+72 ✦ · Cel dnia ✓ · Seria 5 · Poz. 8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PopupRewards {
    /// Sparks of the moment (e.g. the block that just ended).
    pub sparks: u32,
    pub daily_goal: bool,
    /// The streak after a +1 (shown only while `rewards.showStreak`).
    pub streak: Option<u32>,
    /// The new level after a level up.
    pub level_up: Option<u32>,
    pub surprise: Option<Rarity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PopupAction {
    /// Close the popup (also its timeout).
    Dismiss,
    /// Show and focus the main window.
    ShowMain,
    /// Same as the tray's "Pomiń fazę".
    SkipPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TintStatus {
    /// Set by the shell; grows with every emitted status.
    pub revision: u32,
    pub enabled: bool,
    /// Current strength 0..=100 (schedule or preview); 0 = screen colours untouched.
    pub level: u32,
    pub preview: bool,
    /// The warmth is applied right now.
    pub active: bool,
    /// Mechanism in use (the configured one, or the fallback when it is unavailable).
    pub mode: TintMode,
    /// Windows refused the full strength (gamma limits) and a weaker value is applied.
    pub capped: bool,
    /// The mechanism failed or is unavailable (English detail); the schedule keeps trying.
    pub error: Option<String>,
    /// The warmth is suspended by an exception (ADR-023): `level` fades to 0 and comes back when it ends. `None`
    /// when nothing suspends it (also during a settings preview, which ignores the exceptions).
    #[serde(default)]
    pub paused_by: Option<TintPause>,
    /// Which schedule is in effect today (ADR-023).
    #[serde(default)]
    pub schedule: TintScheduleState,
    /// The warm window in effect today, minutes after local midnight (`0..1440`): sunset and sunrise for `sun`,
    /// otherwise the fixed hours (`TintConfig.start_minutes` / `end_minutes`).
    #[serde(default)]
    pub window_start_minutes: u32,
    #[serde(default)]
    pub window_end_minutes: u32,
}

/// Why the warmth is suspended (ADR-023).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TintPause {
    pub reason: TintPauseReason,
    /// Exe name of the foreground app (`Photoshop.exe`), when known; the UI shows it without `.exe`
    /// ("Paused: Photoshop in front"). Process name only, never a window title; never logged.
    pub app: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TintPauseReason {
    /// One of `TintConfig.pause_apps` is in front (`pause_for_apps`).
    App,
    /// A full-screen app or game is in front (`pause_for_fullscreen`).
    Fullscreen,
}

/// The schedule in effect today (`TintStatus.schedule`, ADR-023).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TintScheduleState {
    /// The fixed hours were chosen.
    #[default]
    Fixed,
    /// Sunset → sunrise at the configured location.
    Sun,
    /// `sun` chosen, but the sun does not set today at that place: the fixed hours are used.
    PolarDay,
    /// `sun` chosen, but the sun does not rise today at that place: the fixed hours are used.
    PolarNight,
    /// `sun` chosen without a location: the fixed hours are used.
    NoLocation,
}

// ---------------------------------------------------------------------------
// Calendars: meetings that collide with planned breaks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CalendarSource {
    Microsoft,
    Google,
    Ics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MeetingChoice {
    /// Keep focusing through the meeting with the music muted; the break starts when the meeting ends.
    Extend,
    /// When the break is due during the meeting, it starts paused (timer frozen, music silent) until Resume.
    PauseAtBreak,
    /// Change nothing.
    Ignore,
    /// Ask again 30 / 20 / 10 minutes before the meeting (the last reminder offers no "later").
    RemindLater,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Meeting {
    /// Stable id: source + event id + start.
    pub id: String,
    pub source: CalendarSource,
    pub subject: String,
    #[ts(type = "number")]
    pub start_ms: i64,
    #[ts(type = "number")]
    pub end_ms: i64,
    /// Teams / Meet / Zoom / Webex link present.
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MeetingConflict {
    pub meeting: Meeting,
    /// 1-based number of the colliding break in the (planned) session.
    pub break_number: u32,
    #[ts(type = "number")]
    pub break_start_ms: i64,
    #[ts(type = "number")]
    pub break_end_ms: i64,
    pub choice: Option<MeetingChoice>,
    /// When the next "remind later" popup is due (wall clock), if any.
    #[ts(type = "number | null")]
    pub next_reminder_ms: Option<i64>,
    /// `false` for the last reminder (10 minutes before).
    pub remind_later_allowed: bool,
    /// The meeting is longer than `timing::MAX_MEETING_HOLD_MS` (4 h): `Extend` / `PauseAtBreak` are not
    /// offered and the session ignores them.
    #[serde(default)]
    pub too_long: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalendarSourceStatus {
    pub source: CalendarSource,
    pub state: IntegrationState,
    /// English detail for the settings screen (error or state).
    pub detail: Option<String>,
    /// Signed-in account or number of ICS links.
    pub account: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalendarStatus {
    /// Set by the shell; grows with every emitted status.
    pub revision: u32,
    pub sources: Vec<CalendarSourceStatus>,
    /// Meetings of the next `horizon_hours` (sorted by start), declined / cancelled / "free" ones excluded.
    pub meetings: Vec<Meeting>,
    /// Meetings that overlap a planned break.
    pub conflicts: Vec<MeetingConflict>,
    /// `true` = breaks of a session that would start now (idle); `false` = breaks of the running session.
    pub planned: bool,
    #[ts(type = "number | null")]
    pub last_sync_ms: Option<i64>,
    /// Google sign-in in progress (browser opened).
    pub google_pending: bool,
    /// Breaks moved by smart breaks (ADR-023), in break order. Empty while `Config.smart_breaks` is off. A meeting
    /// listed here is not in `conflicts`: it no longer collides with the adjusted plan.
    #[serde(default)]
    pub smart_breaks: Vec<SmartBreak>,
    /// Meeting ids the user asked to decide about by hand ("Ask me instead", `set_smart_break_opt_out`); their
    /// collisions stay in `conflicts` with the usual question.
    #[serde(default)]
    pub smart_opt_outs: Vec<String>,
}

/// One break moved around a meeting by smart breaks (ADR-023). Wall-clock times; no meeting subject (the UI may
/// look it up in `CalendarStatus.meetings` by `meeting_id` where ADR-016 allows showing it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SmartBreak {
    /// `Meeting.id`.
    pub meeting_id: String,
    #[ts(type = "number")]
    pub meeting_start_ms: i64,
    #[ts(type = "number")]
    pub meeting_end_ms: i64,
    /// 1-based number of the moved break; the focus block before it changes length.
    pub break_number: u32,
    pub kind: SmartBreakKind,
    /// Change of that focus block in seconds: negative = shortened (to at least 60 % of the focus length and never
    /// under 10 minutes, demo profile scaled), positive = lengthened (by at most 25 %).
    pub focus_delta_secs: i32,
    /// The moved break, `[start, end)`.
    #[ts(type = "number")]
    pub break_start_ms: i64,
    #[ts(type = "number")]
    pub break_end_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SmartBreakKind {
    /// The break ends right when the meeting starts.
    EndsBeforeMeeting,
    /// The break starts when the meeting starts.
    StartsAtMeeting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MeetingHoldView {
    pub subject: String,
    /// Focus extended / break held until this moment (meeting end), wall clock.
    #[ts(type = "number")]
    pub until_ms: i64,
    pub choice: MeetingChoice,
}

// ---------------------------------------------------------------------------
// Work items ("Co dalej?") and the session goal
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WorkSource {
    Github,
    Gitlab,
    Jira,
    AzureDevops,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WorkItemKind {
    /// Issue / ticket / work item assigned to the user.
    Assigned,
    /// A pull / merge request waits for the user's review.
    ReviewRequested,
    /// Changes were requested on the user's own pull / merge request.
    ChangesRequested,
    /// Checks failed on the user's own pull / merge request.
    CiFailed,
    /// The user was mentioned.
    Mention,
    /// Any other notification.
    Notification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WorkReason {
    Assigned,
    InSprint,
    HighPriority,
    DueToday,
    Overdue,
    InProgress,
    BlockingOthers,
    WaitingLong,
    Mentioned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkItem {
    /// Stable id: source + key.
    pub id: String,
    pub source: WorkSource,
    pub kind: WorkItemKind,
    /// Human key, e.g. `PROJ-123`, `owner/repo#45`, `ENG-7`.
    pub key: String,
    pub title: String,
    pub url: String,
    pub project: Option<String>,
    /// Priority as the tool names it (e.g. "High", "P1").
    pub priority_label: Option<String>,
    /// Normalised priority 0 (highest) ..= 4 (lowest).
    pub priority_rank: Option<u32>,
    #[ts(type = "number | null")]
    pub due_ms: Option<i64>,
    /// Status as the tool names it (e.g. "In Progress").
    pub status: Option<String>,
    /// In the active sprint / cycle / iteration.
    pub in_sprint: bool,
    #[ts(type = "number")]
    pub updated_ms: i64,
    /// Ranking score (higher first); tier 1 = assigned to the user always ranks above tier 2 and 3.
    pub score: u32,
    pub reasons: Vec<WorkReason>,
    /// Arrived since the user last looked (held during focus).
    pub unseen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkSourceStatus {
    pub source: WorkSource,
    pub state: IntegrationState,
    pub detail: Option<String>,
    /// Signed-in user name, if known.
    pub account: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkStatus {
    /// Set by the shell; grows with every emitted status.
    pub revision: u32,
    pub sources: Vec<WorkSourceStatus>,
    /// Ranked, best first (at most 50).
    pub items: Vec<WorkItem>,
    /// New items held back during focus (shown at the break).
    pub held_count: u32,
    #[ts(type = "number | null")]
    pub last_sync_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionGoal {
    pub source: WorkSource,
    pub key: String,
    pub title: String,
    pub url: String,
}

// ---------------------------------------------------------------------------
// Developer mode (ADR-019)
// ---------------------------------------------------------------------------

/// A button on the developer page (`dev_trigger`). Sample data lives only in memory: nothing is written to
/// `history.jsonl` or `meeting-choices.json` and nothing is sent over the network. The one exception is
/// [`DevAction::FactoryReset`], which deletes the app's own data on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum DevAction {
    /// Shows the phase popup window with a sample `PopupInfo` filled by the shell: cycle, phase length from the
    /// current `TimingProfile`, a sample reminder, a sample meeting (id prefixed `dev-sample-`) for
    /// `MeetingReminder` / `BreakHeld`, Steam + Epic for `LaunchersClosed`, `work_count` 3 for `WorkDigest`.
    /// Shown at once, also while the user is presenting.
    #[serde(rename_all = "camelCase")]
    Popup {
        kind: PopupKind,
        /// Minutes before the sample meeting (30 / 20 / 10); decides which buttons a `MeetingReminder` offers
        /// (10 = the last reminder, without "later"). `None` = 30.
        lead_minutes: Option<u32>,
        /// Keep the popup open until it is closed (`PopupInfo.sticky`).
        sticky: bool,
    },
    /// The same Windows toast a real change to `phase` shows (`focus` or `break`; `idle` is rejected).
    #[serde(rename_all = "camelCase")]
    Toast { phase: Phase },
    /// Running session only (else `notRunning`): the current phase ends in `secs`, clamped to 1..=phase length.
    #[serde(rename_all = "camelCase")]
    SetRemaining { secs: u32 },
    /// Presentation mode on / off: the demo `TimingProfile` (the one of `--demo` / `FOCUSMODE_DEMO=1`) or the profile
    /// from the config, for the rest of this process (nothing is saved: a restart is normal again unless `--demo`).
    /// Idle: at once (`SessionSnapshot.demo` follows). A running or paused session keeps its profile and phase
    /// lengths; the next Start uses the new one.
    #[serde(rename_all = "camelCase")]
    PresentationMode { on: bool },
    /// Factory reset: everything `delete_all_data` deletes (a running session is stopped first, shields and screen
    /// colours come back), then `config.json` and the app's other files in its own folders (restore files only once
    /// their restore is done), then the app restarts like a fresh install. Resolves just before the restart; when a
    /// delete fails it rejects with `privacy.delete` and the app does not restart. Never touches anything outside the
    /// app's folders (the installed app, the extension folder and music folders stay).
    FactoryReset,
}

// ---------------------------------------------------------------------------
// Privacy panel (ADR-020)
// ---------------------------------------------------------------------------

/// One kind of data FocusMode keeps on this computer. `PrivacyReport.items` lists them in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PrivacyItemKind {
    /// `config.json` (+ `config.json.bak`). Not deletable alone: `delete_all_data` resets it to the defaults.
    Settings,
    /// `history.jsonl`; `count` = stored sessions, `includesTitles` = some session keeps window titles.
    History,
    /// `rewards.json` (+ `rewards.json.bak`), beta (ADR-022): sparks, streak and surprises as numbers, day numbers and
    /// session ids only (no step texts, titles or app names), plus the collection unlocks and seasonal goal days
    /// (ADR-024: fixed ids, day numbers and years); `count` = stored days. Deleting it equals `reset_rewards` (the
    /// unlocks go too, and `Config.reward_theme` goes back to `None`).
    Rewards,
    /// `meeting-choices.json` (meeting ids and choices, no subjects); `count` = stored choices.
    MeetingChoices,
    /// `work-seen.json` (work item ids, no titles); `count` = stored ids.
    WorkSeen,
    /// `teams-token.bin` (DPAPI): the Microsoft sign-in used by Teams and the Microsoft calendar.
    MicrosoftToken,
    /// `google-token.bin` (DPAPI): the Google Calendar sign-in.
    GoogleToken,
    /// `ics-links.bin` (DPAPI): ICS subscription links; `count` = number of links when known.
    IcsLinks,
    /// `work-github-token.bin` (DPAPI).
    GithubToken,
    /// `work-gitlab-token.bin` (DPAPI).
    GitlabToken,
    /// `work-jira-token.bin` (DPAPI).
    JiraToken,
    /// `work-azure-devops-token.bin` (DPAPI).
    AzureDevopsToken,
    /// `work-linear-token.bin` (DPAPI).
    LinearToken,
    /// Imported chime files in the `sounds` folder; `count` = files.
    CustomChimes,
    /// `websiteblacklist.txt`; `count` = domains + patterns. Deleting it restores the bundled default list.
    Blocklist,
    /// `shields-restore.json`: exists only while shields are up (or after a crash). Never deletable (rule 7):
    /// it goes away by itself once Windows settings are restored.
    ShieldsRestore,
    /// `tint-restore.json`: exists only while the screen tint is applied (or after a crash). Never deletable (rule 7).
    TintRestore,
    /// The embedded browser profile (WebView2 data folder): cookies incl. the YouTube sign-in and consent, cache,
    /// local storage. Deleting it clears all browsing data (signs out of YouTube).
    BrowserData,
    /// Log files (no tokens, titles or meeting subjects by rule 11).
    Logs,
}

impl PrivacyItemKind {
    /// Every kind in report order.
    pub const ALL: [Self; 19] = [
        Self::Settings,
        Self::History,
        Self::Rewards,
        Self::MeetingChoices,
        Self::WorkSeen,
        Self::MicrosoftToken,
        Self::GoogleToken,
        Self::IcsLinks,
        Self::GithubToken,
        Self::GitlabToken,
        Self::JiraToken,
        Self::AzureDevopsToken,
        Self::LinearToken,
        Self::CustomChimes,
        Self::Blocklist,
        Self::ShieldsRestore,
        Self::TintRestore,
        Self::BrowserData,
        Self::Logs,
    ];

    /// `delete_privacy_data` may delete it. Settings are only reset by `delete_all_data`; the restore files only
    /// go away once the shell has restored Windows settings (CLAUDE.md, rule 7).
    pub const fn is_deletable(self) -> bool {
        !matches!(
            self,
            Self::Settings | Self::ShieldsRestore | Self::TintRestore
        )
    }

    /// The work tool whose token this kind is.
    pub const fn work_source(self) -> Option<WorkSource> {
        match self {
            Self::GithubToken => Some(WorkSource::Github),
            Self::GitlabToken => Some(WorkSource::Gitlab),
            Self::JiraToken => Some(WorkSource::Jira),
            Self::AzureDevopsToken => Some(WorkSource::AzureDevops),
            Self::LinearToken => Some(WorkSource::Linear),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PrivacyItem {
    pub kind: PrivacyItemKind,
    /// Something of this kind is stored now.
    pub present: bool,
    /// Size on disk in KiB, rounded up (a non-empty file is at least 1); `None` when absent, unknown or too costly
    /// to compute.
    pub size_kib: Option<u32>,
    /// Number of entries where it makes sense (sessions, choices, ids, links, files, blocklist entries).
    pub count: Option<u32>,
    /// `delete_privacy_data` accepts this kind (`settings`, `shieldsRestore`, `tintRestore` never).
    pub deletable: bool,
    /// `history` only: at least one stored session keeps window titles (`trackTitles`). `false` for other kinds.
    pub includes_titles: bool,
}

/// What FocusMode stores on this computer (Settings → "Privacy & data").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PrivacyReport {
    /// Set by the shell; grows with every emitted report.
    pub revision: u32,
    /// Every folder where FocusMode keeps data, without duplicates; the first one is what `open_data_folder` opens
    /// (settings, history, tokens, sounds), the others hold the browser profile and the logs.
    pub folders: Vec<String>,
    /// One entry per `PrivacyItemKind`, in its order (absent items too, with `present: false`).
    pub items: Vec<PrivacyItem>,
    /// A delete or an export is running (buttons stay disabled until the next report).
    pub busy: bool,
}

// ---------------------------------------------------------------------------
// YouTube sign-in for Premium (ADR-021)
// ---------------------------------------------------------------------------

/// YouTube sign-in state of the shared WebView2 profile. Only cookie names are checked (`LOGIN_INFO`, `SAPISID`,
/// `__Secure-3PAPISID` on youtube.com); cookie values are never read into the app's state or logged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct YoutubeAccount {
    /// Set by the shell; grows with every emitted state.
    pub revision: u32,
    /// The shell has checked the cookies at least once since start (until then `signed_in` is only a guess:
    /// the player keeps today's routing).
    pub known: bool,
    /// youtube.com sign-in cookies are present: YouTube plays in the media window (first-party cookies, Premium applies).
    pub signed_in: bool,
    /// The sign-in window is open.
    pub sign_in_open: bool,
    /// Google refused the embedded browser in the sign-in window ("This browser or app may not be secure",
    /// detected by its `/signin/rejected` URL). Cleared when a new sign-in starts.
    pub browser_rejected: bool,
}

// ---------------------------------------------------------------------------
// Beta: sparks, streak and small steps (ADR-022)
// ---------------------------------------------------------------------------

/// Rewards view state (getter `get_rewards` + event `rewards://changed` with revision, ADR-012). Numbers, dates and
/// ids only: never step texts, goal titles or app names. Everything stays on this computer (`rewards.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RewardsStatus {
    /// Set by the shell; grows with every emitted status.
    pub revision: u32,
    /// `betaFeatures && rewards.enabled`. When false the UI hides every reward surface.
    pub active: bool,
    /// The demo preview ledger is shown (nothing is saved); the UI shows a "demo" badge.
    pub preview: bool,
    pub total_sparks: u32,
    /// Computed from `total_sparks` (never stored, never goes down).
    pub level: LevelView,
    pub today: TodayView,
    pub streak: StreakView,
    /// 7 cells, Monday..Sunday of the current reward week.
    pub week: Vec<DayCell>,
    /// The running session, or the last one until the next Start.
    pub session: Option<SessionSparks>,
    pub collection: Collection,
    /// Published in the UI ("?" popover).
    pub odds: SurpriseOdds,
    /// P1: unlocked stickers (empty until implemented).
    pub stickers: Vec<StickerId>,
    /// Moments the main window has not acknowledged yet (oldest first, at most 8).
    pub pending: Vec<RewardMoment>,
    /// The beta collection (ADR-024): every collection theme and ambience layer with its state, rule and progress.
    /// Filled also while the rewards are off (unlocked items stay usable while the beta is on); the UI hides it
    /// without the beta. While achievements wait in focus it shows the unlocks as they were before them.
    #[serde(default)]
    pub unlocks: UnlocksView,
}

/// `level` from the total; `into` sparks of `span` towards the next level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LevelView {
    /// 1-based, no cap.
    pub level: u32,
    pub into: u32,
    pub span: u32,
}

/// The current reward day (it starts at 04:00 local time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TodayView {
    /// `YYYY-MM-DD` of the reward day.
    pub date: String,
    /// Sparks of the day (the daily-goal bonus included).
    pub sparks: u32,
    /// The daily goal in sparks (30 / 60 / 120).
    pub goal: u32,
    pub goal_met: bool,
    /// The daily cap of focus-minute sparks is reached (shown once a day).
    pub pond_full: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreakView {
    pub state: StreakState,
    /// The streak length; 0 for `first` / `waiting` (the UI never prints 0).
    pub days: u32,
    /// The longest streak so far (only grows).
    pub best: u32,
    /// Days with the daily goal met (only grows).
    pub active_days: u32,
    /// Today already counts for the streak.
    pub today_counted: bool,
    /// Rest days in the bank (they cover a work day without a goal by themselves).
    pub rest_bank: u32,
    pub rest_bank_max: u32,
    /// Rest days the bank will use for the current gap.
    pub rest_reserved: u32,
    /// Goal days until the next rest day (1..=5).
    pub next_rest_in: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StreakState {
    /// No goal day yet: "the streak starts with the first daily goal".
    First,
    /// Counting (`days` is the streak).
    Alive,
    /// A new streak starts with the next daily goal (never "lost").
    Waiting,
    /// Paused by the user or while the rewards are off (`days` is kept).
    Paused,
}

/// One day of the week strip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayCell {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub mark: DayMark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DayMark {
    /// The daily goal was met.
    Goal,
    /// A rest day from the bank covered it.
    Rest,
    /// A day off (rest weekday) or a paused day.
    Neutral,
    /// A work day without the goal (a grey empty dot, never red).
    Quiet,
    /// Today, goal not met yet.
    Today,
    Future,
}

/// Sparks of one session and where they came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionSparks {
    pub session_id: String,
    pub sparks: u32,
    pub lines: Vec<RewardLine>,
}

/// `count` credits of one kind worth `sparks` together (e.g. 3 steps = `{ step, 9, 3 }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RewardLine {
    pub kind: RewardKind,
    pub sparks: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RewardKind {
    Start,
    FirstOfDay,
    Plan,
    FocusTime,
    FullBlock,
    BreakTaken,
    BackFromBreak,
    Step,
    AllSteps,
    GoalDone,
    DailyGoal,
    Comeback,
    Surprise,
}

/// Surprises found so far (only grow).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Collection {
    pub bubbles: u32,
    pub pearls: u32,
    pub golden: u32,
}

/// The published surprise odds (permille, they add up to 1000) and the pity rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SurpriseOdds {
    pub none_permille: u16,
    pub bubble_permille: u16,
    pub pearl_permille: u16,
    pub golden_permille: u16,
    /// After this many full blocks without a pearl, the next one brings at least a pearl.
    pub pity_blocks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Rarity {
    Bubble,
    Pearl,
    GoldenPearl,
}

/// P1 sticker album: one sticker per mascot pose, unlocked deterministically (no luck required).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StickerId {
    FirstStep,
    DailyGoal,
    Planner,
    GoodBreak,
    Comeback,
    Amazed,
    RestDay,
    Together,
    ThreeDays,
    GoldenPearl,
    Level5,
}

/// Something to celebrate (or just to count). Achievements earned during focus wait for the break or for Stop
/// ("focus is sacred"): a moment returned while focus goes on carries only its lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RewardMoment {
    /// Grows within an app run (`ack_reward_moments { upTo }`).
    pub id: u32,
    pub kind: MomentKind,
    /// Wall clock when it happened (older than 15 min: shown static, without sound).
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Sum of `lines`.
    pub sparks: u32,
    /// For `breakStart` / `sessionEnd`: the session total so far.
    pub session_total: u32,
    pub lines: Vec<RewardLine>,
    pub surprise: Option<Rarity>,
    pub daily_goal: bool,
    pub comeback: bool,
    /// The streak after a +1.
    pub streak: Option<u32>,
    /// The streak started again at 1 ("Nowy początek!").
    pub new_streak: bool,
    /// The streak reached 3 / 7 / 14 / 30 / 50 / 100 / 200 / 365.
    pub milestone: Option<u32>,
    /// Rest days the bank used to cover a gap.
    pub rest_days_used: u32,
    pub rest_day_earned: bool,
    /// The new (highest) level.
    pub level_up: Option<u32>,
    pub levels_gained: u32,
    /// The daily cap of focus-minute sparks was reached.
    pub pond_full: bool,
    /// P1: stickers unlocked by this moment.
    pub stickers: Vec<StickerId>,
    /// The first step ticked in this session.
    pub first_step: bool,
    /// Break held by a meeting / music muted: static, no sound, no confetti.
    pub quiet: bool,
    /// From the demo preview ledger (badge; nothing was saved).
    pub preview: bool,
    /// A Developer-page sample built in the frontend (never from the shell, never acked).
    #[serde(default)]
    pub sample: bool,
    /// Collection themes unlocked by this moment (ADR-024), in catalogue order. Like every achievement they wait for
    /// the break or Stop ("focus is sacred"); never in the demo preview.
    #[serde(default)]
    pub unlocked_themes: Vec<RewardTheme>,
    /// Ambience layers unlocked by this moment (ADR-024), in catalogue order.
    #[serde(default)]
    pub unlocked_ambiences: Vec<AmbienceLayer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MomentKind {
    /// Idle → Focus (user's Start).
    Start,
    /// A step ticked.
    Step,
    /// The last open step ticked (at least 2 steps).
    AllSteps,
    /// Break → Focus.
    BackFromBreak,
    /// Focus → Break: the recap of the block (with the deferred achievements).
    BreakStart,
    /// Stop: the recap of the session (with the deferred achievements).
    SessionEnd,
    /// "Zrobione? → Tak" in the summary.
    GoalDone,
    /// A config change met today's goal (e.g. a lower daily goal).
    Update,
}

/// Short synthetic reward sounds (`play_reward_sound`), per chime pack; no licensed audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RewardSound {
    /// ~0.15 s: a step ticked.
    Pop,
    /// ~0.7 s: daily goal, milestone, comeback, pearl.
    Sparkle,
    /// ~1.2 s: level up, golden pearl.
    Fanfare,
}

// ---------------------------------------------------------------------------
// Beta: collection themes and ambience layers (ADR-024)
// ---------------------------------------------------------------------------

/// A beta collection theme (ADR-024): earned with the rewards, never bought; the base themes (`Theme`) stay free for
/// everyone. Visible and applied only while `Config.beta_features` is on. Rules: `focus_core::unlocks::THEME_RULES`.
/// The serialized name is also the `data-theme` value and the folder `src/lib/themes/<name>/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RewardTheme {
    /// The axolotl's home: kelp, bubbles, light rays. Level 3.
    Underwater,
    /// Rain on glass, city bokeh. Best streak 7.
    RainyWindow,
    /// Pines, fireflies, a warm window. Level 6.
    ForestCabin,
    /// Blossoms drifting under the moon. Best streak 14.
    SakuraNight,
    /// Gentle fireworks over a night skyline. Seasonal: January.
    NewYear,
    /// Soft hearts drifting, pink dusk. Seasonal: February.
    Valentines,
    /// Spring meadow, pastel eggs, blossoms. Seasonal: around Western Easter (`unlocks::season_window`).
    Easter,
    /// Sunset beach, palm silhouettes, gentle waves. Seasonal: July.
    Summer,
    /// Moon, fog, bats, a scarecrow, glowing jack-o'-lanterns. Seasonal: October.
    Halloween,
    /// Snowfall, a cozy lit window, pines, twinkling string lights. Seasonal: December.
    Christmas,
}

impl RewardTheme {
    /// Catalogue order (the order of `UnlocksView.themes` and of the "Kolekcja" grid).
    pub const ALL: [Self; 10] = [
        Self::Underwater,
        Self::RainyWindow,
        Self::ForestCabin,
        Self::SakuraNight,
        Self::NewYear,
        Self::Valentines,
        Self::Easter,
        Self::Summer,
        Self::Halloween,
        Self::Christmas,
    ];
}

/// A beta ambience layer (ADR-024), synthesized at runtime in the main window (WebAudio; no audio files). Rules:
/// `focus_core::unlocks::AMBIENCE_RULES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AmbienceLayer {
    /// Free in the beta.
    Rain,
    /// Free in the beta.
    Wind,
    /// Distant thunder. Level 2.
    Thunder,
    /// Café murmur. Level 4.
    Cafe,
    /// Crackling fire. Best streak 5.
    Fireplace,
    /// Forest and crickets. Level 5.
    Forest,
    /// Ocean waves. Level 7.
    Ocean,
    /// Howling wind, distant thunder, owl hoots, creaking wood, a faint drone. With the Halloween theme.
    SpookyNight,
    /// Crackling fire and soft sleigh bells now and then. With the Christmas theme.
    WinterCottage,
}

impl AmbienceLayer {
    /// Catalogue order (the order of `UnlocksView.ambiences` and of the mixer).
    pub const ALL: [Self; 9] = [
        Self::Rain,
        Self::Wind,
        Self::Thunder,
        Self::Cafe,
        Self::Fireplace,
        Self::Forest,
        Self::Ocean,
        Self::SpookyNight,
        Self::WinterCottage,
    ];
}

/// A seasonal unlock window (ADR-024, `focus_core::unlocks::season_window`). Windows never overlap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Season {
    /// January.
    NewYear,
    /// February.
    Valentines,
    /// The calendar month of Western Easter Sunday, stretched to at least 7 days on both sides of it.
    Easter,
    /// July.
    Summer,
    /// October.
    Halloween,
    /// December.
    Christmas,
}

impl Season {
    pub const ALL: [Self; 6] = [
        Self::NewYear,
        Self::Valentines,
        Self::Easter,
        Self::Summer,
        Self::Halloween,
        Self::Christmas,
    ];
}

/// The beta ambience mixer (`Config.ambience`, ADR-024). Remembered per user; nothing ever plays at app start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AmbienceConfig {
    /// Master switch: the layers that are `on` play while the music plays (and alone when the user starts them in the
    /// mixer). Default false.
    pub enabled: bool,
    /// Master volume 0..=1, on top of every layer's own volume (the music volume is separate). Default 0.6.
    pub volume: f32,
    /// One entry per layer the user touched, in catalogue order (duplicates dropped, the first wins); a layer without
    /// an entry is off at `config::DEFAULT_AMBIENCE_LAYER_VOLUME`. Default: rain on.
    pub layers: Vec<AmbienceMix>,
}

/// One layer of the ambience mixer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AmbienceMix {
    pub layer: AmbienceLayer,
    /// The user wants it in the mix (a locked layer never plays, whatever this says).
    pub on: bool,
    /// 0..=1.
    pub volume: f32,
}

/// How a collection item is earned (ADR-024). Internally tagged: `{ "type": "level", "level": 3 }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum UnlockRule {
    /// In the beta from the start (rain, wind).
    Free,
    /// The level (`RewardsStatus.level.level`) reaches `level`.
    #[serde(rename_all = "camelCase")]
    Level { level: u32 },
    /// The best streak ever (`StreakView.best`) reaches `days` (a current streak of `days` counts the same).
    #[serde(rename_all = "camelCase")]
    BestStreak { days: u32 },
    /// The daily goal met on `goal_days` different reward days inside one window of `season` (only then can it unlock).
    #[serde(rename_all = "camelCase")]
    Seasonal { season: Season, goal_days: u32 },
    /// Unlocks together with that collection theme (seasonal ambiences).
    #[serde(rename_all = "camelCase")]
    WithTheme { theme: RewardTheme },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UnlockState {
    /// Usable from the start of the beta (no rule to meet).
    Free,
    /// Earned (kept forever; only "Zacznij od nowa" / the privacy delete clear it).
    Unlocked,
    /// Seasonal and its window is open now: it can be earned today (`progress` shows how far).
    AvailableNow,
    /// Not earned yet (a seasonal one outside its window says when it opens: `window`).
    Locked,
}

/// `have` of `need` towards an unlock (levels, best-streak days, goal days in the open seasonal window).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UnlockProgress {
    /// Capped at `need`.
    pub have: u32,
    pub need: u32,
}

/// A seasonal window: the open one, or else the next one (this year or the next). Reward days (they start at 04:00).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SeasonWindow {
    pub season: Season,
    /// First and last day, `YYYY-MM-DD`, inclusive.
    pub start: String,
    pub end: String,
    /// Today is inside the window.
    pub open: bool,
    /// 1..=12 when the window is exactly one calendar month (the UI says "in October"); `None` = show the dates.
    pub month: Option<u32>,
}

/// One collection theme in `UnlocksView.themes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThemeUnlock {
    pub theme: RewardTheme,
    pub state: UnlockState,
    pub rule: UnlockRule,
    /// `level` / `bestStreak` while locked, `seasonal` while its window is open (`availableNow`); else `None`.
    pub progress: Option<UnlockProgress>,
    /// `seasonal` rules only.
    pub window: Option<SeasonWindow>,
    /// Reward day it was unlocked (`YYYY-MM-DD`); `None` unless `unlocked`.
    pub unlocked_on: Option<String>,
}

/// One ambience layer in `UnlocksView.ambiences`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AmbienceUnlock {
    pub layer: AmbienceLayer,
    pub state: UnlockState,
    pub rule: UnlockRule,
    /// `level` / `bestStreak` while locked; else `None` (`withTheme`: see that theme).
    pub progress: Option<UnlockProgress>,
    /// Reward day it was unlocked (`YYYY-MM-DD`); `None` unless `unlocked`.
    pub unlocked_on: Option<String>,
}

/// The beta collection (`RewardsStatus.unlocks`, ADR-024), in catalogue order (`RewardTheme::ALL`, `AmbienceLayer::ALL`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UnlocksView {
    pub themes: Vec<ThemeUnlock>,
    pub ambiences: Vec<AmbienceUnlock>,
}

// ---------------------------------------------------------------------------
// Weekly stats and heatmaps (ADR-023)
// ---------------------------------------------------------------------------

/// The user's label for an app (`Config.app_categories`). Only the user labels apps; nothing is labelled by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AppCategory {
    Productive,
    Neutral,
    Distracting,
}

/// Payload of `history://changed`: the stored history (or the app labels) changed; ask `get_weekly_stats` again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryChanged {
    /// Set by the shell; grows with every change. `WeeklyStats.revision` is the revision it was computed from.
    pub revision: u32,
}

/// One ISO week (Monday first, local time) from `history.jsonl` (`get_weekly_stats`, ADR-023). Aggregated by
/// `focus_core::stats` from the stored sessions; demo sessions are left out. Seconds everywhere (the UI rounds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WeeklyStats {
    /// The history revision (`HistoryChanged.revision`) these numbers come from.
    pub revision: u32,
    /// Monday of the week, `YYYY-MM-DD` (the requested date moved back to its Monday).
    pub week_start: String,
    /// ISO 8601 week-numbering year and week (1..=53): `2026-W42`.
    pub iso_year: u32,
    pub iso_week: u32,
    /// The week contains today (the UI disables "next week").
    pub current_week: bool,
    /// Monday of the week of the oldest stored session (the UI disables "previous week" there); `None` without history.
    pub first_week_start: Option<String>,
    /// `Config.keep_history` is off: new sessions are not stored (the numbers still cover what is stored, usually
    /// nothing; the UI explains).
    pub history_off: bool,
    /// Always 7 entries, Monday..Sunday.
    pub days: Vec<DayStats>,
    pub focus_secs: u32,
    pub break_secs: u32,
    /// Breaks started (sum of `SessionSummary.cycles_completed`).
    pub breaks_taken: u32,
    /// Sessions that started in this week.
    pub sessions: u32,
    /// The day with the most focus (the earlier one on a tie); `None` for a week without focus.
    pub best_day: Option<Weekday>,
    /// Focus seconds by weekday × local hour of this week: 7 rows (Monday..Sunday) of 24 cells (00..23).
    pub heatmap: Vec<Vec<u32>>,
    /// The same shape summed over the `recent_weeks` weeks that end with this one.
    pub recent_heatmap: Vec<Vec<u32>>,
    /// Number of weeks in `recent_heatmap` (`focus_core::stats::RECENT_WEEKS`, 8).
    pub recent_weeks: u32,
    /// Apps of this week by focus time, most first (at most `focus_core::stats::TOP_APPS_PER_WEEK`).
    pub top_apps: Vec<WeekApp>,
    /// App time of this week per category (all apps, not only `top_apps`; idle and unknown time left out).
    pub categories: CategoryTotals,
    /// Some sessions of `recent_heatmap` were stored without `focus_spans` (older history): their focus is spread
    /// evenly between start and end, so the hours are approximate.
    pub estimated: bool,
}

/// One day of `WeeklyStats.days`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayStats {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub weekday: Weekday,
    pub focus_secs: u32,
    pub break_secs: u32,
    /// Sessions that started on this day.
    pub sessions: u32,
}

/// One app in `WeeklyStats.top_apps`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WeekApp {
    /// Process executable name as tracked (`Code.exe`); `Config.app_categories` uses it lowercased.
    pub process: String,
    pub secs: u32,
    /// `None` = unlabelled.
    pub category: Option<AppCategory>,
}

/// App time of a week per category (`WeeklyStats.categories`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryTotals {
    pub productive_secs: u32,
    pub neutral_secs: u32,
    pub distracting_secs: u32,
    pub unlabelled_secs: u32,
}

// ---------------------------------------------------------------------------
// Browser extension bridge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BridgeInfo {
    pub port: u16,
    /// Absolute path of the bundled unpacked extension, if found.
    pub extension_dir: Option<String>,
    /// Seconds since the extension last polled the bridge.
    pub extension_seen_secs_ago: Option<u32>,
}

/// JSON served to the browser extension at `GET /v1/state`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BridgeState {
    /// Block now (Focus running or paused, `block_sites` on).
    pub active: bool,
    /// Normalized domains; each also blocks its subdomains. Empty when not blocking.
    pub blocklist: Vec<String>,
    /// Regex sources (RE2-compatible, matched case-insensitively against the full URL). Empty when not blocking.
    #[serde(default)]
    pub patterns: Vec<String>,
    pub phase: Phase,
    pub remaining_secs: u32,
}

#[cfg(test)]
mod tests {
    use super::{
        AmbienceLayer, AmbienceMix, AmbienceUnlock, AppCategory, CalendarStatus, CategoryTotals,
        CelebrationLevel, Config, DailyGoal, DayStats, DevAction, HistoryChanged, MicroStep,
        MomentKind, Phase, PopupInfo, PopupKind, PopupRewards, PrivacyItemKind, Rarity, RewardKind,
        RewardLine, RewardMoment, RewardSound, RewardTheme, Season, SeasonWindow, SessionSnapshot,
        SessionSummary, SmartBreak, SmartBreakKind, StepDraft, StickerId, ThemeUnlock, TimeSpan,
        TintPause, TintPauseReason, TintSchedule, TintScheduleState, TintStatus, UnlockProgress,
        UnlockRule, UnlockState, WeekApp, Weekday, WeeklyStats, WorkSource, YoutubeAccount,
        commands, events,
    };
    use std::fmt::Write as _;

    /// A `SessionSummary` line as `history.jsonl` stored it before ADR-022.
    const OLD_SUMMARY: &str = r#"{"sessionId":"1760342400000-4242-3","startedAtMs":1,"endedAtMs":2,"focusedSecs":60,"breakSecs":0,"cyclesCompleted":0,"usage":{"apps":[],"idleSecs":0,"unknownSecs":0,"trackedSecs":0},"demo":false,"goal":null,"goalDone":null}"#;

    #[test]
    fn config_written_before_beta_loads_with_beta_off() {
        let old: Config = serde_json::from_str(r#"{"focusMinutes":30,"developerMode":true}"#)
            .expect("old config");
        assert!(!old.beta_features);
        assert_eq!(old.rewards, Config::default().rewards);
        assert!(old.rewards.enabled && old.rewards.micro_steps && old.rewards.surprises);
        assert!(old.rewards.sounds && !old.rewards.focus_companion && old.rewards.show_streak);
        assert!(!old.rewards.streak_paused);
        assert_eq!(old.rewards.celebrations, CelebrationLevel::Full);
        assert_eq!(old.rewards.daily_goal, DailyGoal::Light);
        assert_eq!(old.rewards.rest_weekdays, [Weekday::Sat, Weekday::Sun]);

        let json = serde_json::to_value(Config::default()).expect("serialize");
        assert_eq!(json["betaFeatures"], false);
        assert_eq!(
            json["rewards"],
            serde_json::json!({
                "enabled": true, "microSteps": true, "surprises": true, "sounds": true,
                "celebrations": "full", "focusCompanion": false, "showStreak": true, "streakPaused": false,
                "dailyGoal": "light", "restWeekdays": ["sat", "sun"]
            })
        );
        let partial: Config = serde_json::from_str(
            r#"{"betaFeatures":true,"rewards":{"dailyGoal":"strong","celebrations":"numbersOnly"}}"#,
        )
        .expect("partial rewards");
        assert!(partial.beta_features);
        assert_eq!(partial.rewards.daily_goal, DailyGoal::Strong);
        assert_eq!(partial.rewards.celebrations, CelebrationLevel::NumbersOnly);
        assert!(partial.rewards.enabled);
    }

    #[test]
    fn summary_written_before_steps_loads_without_steps_or_sparks() {
        let old: SessionSummary = serde_json::from_str(OLD_SUMMARY).expect("old summary");
        assert!(old.steps.is_empty());
        assert_eq!(old.sparks, None);
        // Without steps and sparks the stored line is exactly what older versions wrote.
        assert_eq!(serde_json::to_string(&old).expect("serialize"), OLD_SUMMARY);

        let with = SessionSummary {
            steps: vec![MicroStep {
                id: 1,
                text: "Napisz tytuł".into(),
                done: true,
            }],
            sparks: Some(72),
            ..old
        };
        let json = serde_json::to_value(&with).expect("serialize");
        assert_eq!(
            json["steps"],
            serde_json::json!([{ "id": 1, "text": "Napisz tytuł", "done": true }])
        );
        assert_eq!(json["sparks"], 72);
    }

    #[test]
    fn snapshot_written_before_steps_has_no_steps() {
        let snapshot: SessionSnapshot = serde_json::from_str(
            r#"{"revision":1,"sessionId":null,"phase":"idle","paused":false,"cycle":0,"remainingSecs":0,"phaseTotalSecs":0,"focusSecs":2700,"breakSecs":900,"focusedSecs":0,"demo":false}"#,
        )
        .expect("old snapshot");
        assert!(snapshot.steps.is_empty());
    }

    #[test]
    fn step_texts_never_reach_debug_output() {
        let step = MicroStep {
            id: 7,
            text: "Zadzwoń do Ani".into(),
            done: false,
        };
        let draft = StepDraft {
            id: None,
            text: "tajny plan".into(),
        };
        let printed = format!("{step:?} {draft:?} {:?}", vec![step.clone()]);
        assert!(!printed.contains("Ani") && !printed.contains("tajny"));
        assert!(printed.contains("\"<14 chars>\"") && printed.contains("\"<10 chars>\""));
        assert!(printed.contains("id: 7"));
    }

    #[test]
    fn step_draft_and_reward_sound_wire_format() {
        let draft = StepDraft {
            id: Some(3),
            text: "Otwórz plik".into(),
        };
        let json = serde_json::to_value(&draft).expect("serialize");
        assert_eq!(json, serde_json::json!({ "id": 3, "text": "Otwórz plik" }));
        assert_eq!(
            serde_json::from_value::<StepDraft>(json).expect("deserialize"),
            draft
        );
        let new: StepDraft =
            serde_json::from_str(r#"{"id":null,"text":"Nowy"}"#).expect("new step");
        assert_eq!(new.id, None);

        for (sound, name) in [
            (RewardSound::Pop, "pop"),
            (RewardSound::Sparkle, "sparkle"),
            (RewardSound::Fanfare, "fanfare"),
        ] {
            let json = serde_json::to_value(sound).expect("serialize");
            assert_eq!(json, serde_json::json!(name));
            assert_eq!(
                serde_json::from_value::<RewardSound>(json).expect("deserialize"),
                sound
            );
        }
    }

    #[test]
    fn reward_moment_round_trips_in_camel_case() {
        let moment = RewardMoment {
            id: 4,
            kind: MomentKind::BreakStart,
            at_ms: 1_760_342_400_000,
            sparks: 72,
            session_total: 84,
            lines: vec![RewardLine {
                kind: RewardKind::FocusTime,
                sparks: 45,
                count: 45,
            }],
            surprise: Some(Rarity::GoldenPearl),
            daily_goal: true,
            comeback: false,
            streak: Some(5),
            new_streak: false,
            milestone: None,
            rest_days_used: 1,
            rest_day_earned: false,
            level_up: Some(8),
            levels_gained: 1,
            pond_full: false,
            stickers: vec![StickerId::Level5],
            first_step: false,
            quiet: true,
            preview: false,
            sample: false,
            unlocked_themes: vec![RewardTheme::Halloween],
            unlocked_ambiences: vec![AmbienceLayer::SpookyNight],
        };
        let json = serde_json::to_value(&moment).expect("serialize");
        assert_eq!(json["kind"], "breakStart");
        assert_eq!(json["atMs"], 1_760_342_400_000_i64);
        assert_eq!(json["sessionTotal"], 84);
        assert_eq!(
            json["lines"],
            serde_json::json!([{ "kind": "focusTime", "sparks": 45, "count": 45 }])
        );
        assert_eq!(json["surprise"], "goldenPearl");
        assert_eq!(json["dailyGoal"], true);
        assert_eq!(json["restDaysUsed"], 1);
        assert_eq!(json["levelUp"], 8);
        assert_eq!(json["stickers"], serde_json::json!(["level5"]));
        assert_eq!(json["firstStep"], false);
        assert_eq!(
            serde_json::from_value::<RewardMoment>(json.clone()).expect("deserialize"),
            moment
        );
        assert_eq!(json["unlockedThemes"], serde_json::json!(["halloween"]));
        assert_eq!(
            json["unlockedAmbiences"],
            serde_json::json!(["spookyNight"])
        );
        let mut without_sample = json;
        if let Some(object) = without_sample.as_object_mut() {
            object.remove("sample");
            object.remove("unlockedThemes");
            object.remove("unlockedAmbiences");
        }
        let old = serde_json::from_value::<RewardMoment>(without_sample)
            .expect("sample and unlocks may be left out");
        assert!(!old.sample);
        assert!(old.unlocked_themes.is_empty() && old.unlocked_ambiences.is_empty());
    }

    #[test]
    fn config_written_before_adr_024_loads_without_collection_theme_or_ambience() {
        let old: Config =
            serde_json::from_str(r#"{"theme":"lunar","betaFeatures":true}"#).expect("old config");
        assert_eq!(old.theme, super::Theme::Lunar);
        assert_eq!(old.reward_theme, None);
        assert!(!old.ambience.enabled);
        assert_eq!(old.ambience, Config::default().ambience);

        let json = serde_json::to_value(Config::default()).expect("serialize");
        assert!(json["rewardTheme"].is_null());
        assert_eq!(json["ambience"]["enabled"], false);
        assert!(
            json["ambience"]["volume"]
                .as_f64()
                .is_some_and(|volume| (volume - 0.6).abs() < 1e-6)
        );
        assert_eq!(
            json["ambience"]["layers"],
            serde_json::json!([ { "layer": "rain", "on": true, "volume": 0.5 } ])
        );
        let chosen: Config = serde_json::from_str(
            r#"{"rewardTheme":"halloween","ambience":{"enabled":true,"layers":[{"layer":"spookyNight","on":true,"volume":0.3}]}}"#,
        )
        .expect("collection theme and ambience");
        assert_eq!(chosen.reward_theme, Some(RewardTheme::Halloween));
        assert!(chosen.ambience.enabled);
        assert_eq!(
            chosen.ambience.layers,
            [AmbienceMix {
                layer: AmbienceLayer::SpookyNight,
                on: true,
                volume: 0.3
            }]
        );
        assert!(
            (chosen.ambience.volume - 0.6).abs() < 1e-6,
            "missing volume → default"
        );
    }

    #[test]
    fn collection_wire_format() {
        let names: Vec<_> = RewardTheme::ALL
            .iter()
            .map(|theme| serde_json::to_value(theme).expect("serialize"))
            .collect();
        assert_eq!(
            names,
            [
                "underwater",
                "rainyWindow",
                "forestCabin",
                "sakuraNight",
                "newYear",
                "valentines",
                "easter",
                "summer",
                "halloween",
                "christmas"
            ]
        );
        let layers: Vec<_> = AmbienceLayer::ALL
            .iter()
            .map(|layer| serde_json::to_value(layer).expect("serialize"))
            .collect();
        assert_eq!(
            layers,
            [
                "rain",
                "wind",
                "thunder",
                "cafe",
                "fireplace",
                "forest",
                "ocean",
                "spookyNight",
                "winterCottage"
            ]
        );
        assert_eq!(Season::ALL.len(), 6);
        for (rule, json) in [
            (UnlockRule::Free, serde_json::json!({ "type": "free" })),
            (
                UnlockRule::Level { level: 3 },
                serde_json::json!({ "type": "level", "level": 3 }),
            ),
            (
                UnlockRule::BestStreak { days: 7 },
                serde_json::json!({ "type": "bestStreak", "days": 7 }),
            ),
            (
                UnlockRule::Seasonal {
                    season: Season::Halloween,
                    goal_days: 3,
                },
                serde_json::json!({ "type": "seasonal", "season": "halloween", "goalDays": 3 }),
            ),
            (
                UnlockRule::WithTheme {
                    theme: RewardTheme::Christmas,
                },
                serde_json::json!({ "type": "withTheme", "theme": "christmas" }),
            ),
        ] {
            assert_eq!(serde_json::to_value(rule).expect("serialize"), json);
            assert_eq!(
                serde_json::from_value::<UnlockRule>(json).expect("deserialize"),
                rule
            );
        }
        let halloween = ThemeUnlock {
            theme: RewardTheme::Halloween,
            state: UnlockState::AvailableNow,
            rule: UnlockRule::Seasonal {
                season: Season::Halloween,
                goal_days: 3,
            },
            progress: Some(UnlockProgress { have: 2, need: 3 }),
            window: Some(SeasonWindow {
                season: Season::Halloween,
                start: "2026-10-01".into(),
                end: "2026-10-31".into(),
                open: true,
                month: Some(10),
            }),
            unlocked_on: None,
        };
        let json = serde_json::to_value(&halloween).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "theme": "halloween", "state": "availableNow",
                "rule": { "type": "seasonal", "season": "halloween", "goalDays": 3 },
                "progress": { "have": 2, "need": 3 },
                "window": { "season": "halloween", "start": "2026-10-01", "end": "2026-10-31", "open": true, "month": 10 },
                "unlockedOn": null
            })
        );
        let rain = AmbienceUnlock {
            layer: AmbienceLayer::Rain,
            state: UnlockState::Free,
            rule: UnlockRule::Free,
            progress: None,
            unlocked_on: None,
        };
        assert_eq!(
            serde_json::to_value(&rain).expect("serialize"),
            serde_json::json!({ "layer": "rain", "state": "free", "rule": { "type": "free" }, "progress": null, "unlockedOn": null })
        );
    }

    #[test]
    fn dev_action_wire_format_is_tagged_camel_case() {
        let popup = DevAction::Popup {
            kind: PopupKind::MeetingReminder,
            lead_minutes: Some(10),
            sticky: true,
        };
        let json = serde_json::to_value(popup).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({ "type": "popup", "kind": "meetingReminder", "leadMinutes": 10, "sticky": true })
        );
        assert_eq!(
            serde_json::from_value::<DevAction>(json).expect("deserialize"),
            popup
        );

        let no_lead: DevAction =
            serde_json::from_str(r#"{"type":"popup","kind":"breakStarted","sticky":false}"#)
                .expect("lead minutes may be left out");
        assert_eq!(
            no_lead,
            DevAction::Popup {
                kind: PopupKind::BreakStarted,
                lead_minutes: None,
                sticky: false,
            }
        );

        let toast: DevAction =
            serde_json::from_str(r#"{"type":"toast","phase":"break"}"#).expect("toast");
        assert_eq!(
            toast,
            DevAction::Toast {
                phase: Phase::Break
            }
        );
        let remaining: DevAction =
            serde_json::from_str(r#"{"type":"setRemaining","secs":5}"#).expect("set remaining");
        assert_eq!(remaining, DevAction::SetRemaining { secs: 5 });

        let presentation = DevAction::PresentationMode { on: true };
        assert_eq!(
            serde_json::to_value(presentation).expect("serialize"),
            serde_json::json!({ "type": "presentationMode", "on": true })
        );
        let off: DevAction =
            serde_json::from_str(r#"{"type":"presentationMode","on":false}"#).expect("off");
        assert_eq!(off, DevAction::PresentationMode { on: false });
        assert!(serde_json::from_str::<DevAction>(r#"{"type":"presentationMode"}"#).is_err());

        assert_eq!(
            serde_json::to_value(DevAction::FactoryReset).expect("serialize"),
            serde_json::json!({ "type": "factoryReset" })
        );
        assert_eq!(
            serde_json::from_str::<DevAction>(r#"{"type":"factoryReset"}"#).expect("reset"),
            DevAction::FactoryReset
        );

        assert!(serde_json::from_str::<DevAction>(r#"{"type":"wipeDisk"}"#).is_err());
    }

    #[test]
    fn privacy_kinds_are_complete_and_camel_case() {
        let mut seen = std::collections::HashSet::new();
        for kind in PrivacyItemKind::ALL {
            assert!(seen.insert(kind), "{kind:?} listed twice");
        }
        assert_eq!(
            serde_json::to_value(PrivacyItemKind::AzureDevopsToken).expect("serialize"),
            serde_json::json!("azureDevopsToken")
        );
        assert_eq!(
            serde_json::from_str::<PrivacyItemKind>(r#""meetingChoices""#).expect("deserialize"),
            PrivacyItemKind::MeetingChoices
        );
        let not_deletable: Vec<_> = PrivacyItemKind::ALL
            .into_iter()
            .filter(|kind| !kind.is_deletable())
            .collect();
        assert_eq!(
            not_deletable,
            [
                PrivacyItemKind::Settings,
                PrivacyItemKind::ShieldsRestore,
                PrivacyItemKind::TintRestore
            ]
        );
        let tools: Vec<_> = PrivacyItemKind::ALL
            .into_iter()
            .filter_map(PrivacyItemKind::work_source)
            .collect();
        assert_eq!(
            tools,
            [
                WorkSource::Github,
                WorkSource::Gitlab,
                WorkSource::Jira,
                WorkSource::AzureDevops,
                WorkSource::Linear
            ]
        );
    }

    #[test]
    fn youtube_account_wire_format() {
        let account = YoutubeAccount {
            revision: 3,
            known: true,
            signed_in: true,
            sign_in_open: false,
            browser_rejected: false,
        };
        assert_eq!(
            serde_json::to_value(&account).expect("serialize"),
            serde_json::json!({ "revision": 3, "known": true, "signedIn": true, "signInOpen": false, "browserRejected": false })
        );
    }

    #[test]
    fn popup_info_written_before_sticky_is_not_sticky() {
        let info: PopupInfo = serde_json::from_str(
            r#"{"revision":5,"kind":"breakStarted","cycle":2,"phaseSecs":900,"launchers":[],"reminder":null}"#,
        )
        .expect("old popup info");
        assert!(!info.sticky);
        assert_eq!(info.rewards, None);
        assert_eq!(info.next_step, None);
    }

    #[test]
    fn popup_rewards_wire_format() {
        let rewards = PopupRewards {
            sparks: 72,
            daily_goal: true,
            streak: Some(5),
            level_up: None,
            surprise: Some(Rarity::Pearl),
        };
        assert_eq!(
            serde_json::to_value(rewards).expect("serialize"),
            serde_json::json!({ "sparks": 72, "dailyGoal": true, "streak": 5, "levelUp": null, "surprise": "pearl" })
        );
    }

    #[test]
    fn views_written_before_adr_023_load_with_its_fields_empty() {
        let snapshot: SessionSnapshot = serde_json::from_str(
            r#"{"revision":1,"sessionId":null,"phase":"idle","paused":false,"cycle":0,"remainingSecs":0,"phaseTotalSecs":0,"focusSecs":2700,"breakSecs":900,"focusedSecs":0,"demo":false}"#,
        )
        .expect("old snapshot");
        assert_eq!(snapshot.smart_break, None);

        let tint: TintStatus = serde_json::from_str(
            r#"{"revision":3,"enabled":true,"level":23,"preview":false,"active":true,"mode":"nightLight","capped":false,"error":null}"#,
        )
        .expect("old tint status");
        assert_eq!(tint.paused_by, None);
        assert_eq!(tint.schedule, TintScheduleState::Fixed);

        let calendar: CalendarStatus = serde_json::from_str(
            r#"{"revision":9,"sources":[],"meetings":[],"conflicts":[],"planned":true,"lastSyncMs":null,"googlePending":false}"#,
        )
        .expect("old calendar status");
        assert!(calendar.smart_breaks.is_empty() && calendar.smart_opt_outs.is_empty());

        let summary: SessionSummary = serde_json::from_str(OLD_SUMMARY).expect("old summary");
        assert!(summary.focus_spans.is_empty());
        let with_spans = SessionSummary {
            focus_spans: vec![TimeSpan {
                start_ms: 1_760_342_400_000,
                end_ms: 1_760_345_100_000,
            }],
            ..summary
        };
        assert_eq!(
            serde_json::to_value(&with_spans).expect("serialize")["focusSpans"],
            serde_json::json!([{ "startMs": 1_760_342_400_000_i64, "endMs": 1_760_345_100_000_i64 }])
        );
    }

    #[test]
    fn tint_pause_and_smart_break_wire_format() {
        let pause = TintPause {
            reason: TintPauseReason::App,
            app: Some("Photoshop.exe".into()),
        };
        assert_eq!(
            serde_json::to_value(&pause).expect("serialize"),
            serde_json::json!({ "reason": "app", "app": "Photoshop.exe" })
        );
        assert_eq!(
            serde_json::to_value(TintScheduleState::PolarNight).expect("serialize"),
            serde_json::json!("polarNight")
        );
        assert_eq!(
            serde_json::to_value(TintSchedule::Sun).expect("serialize"),
            serde_json::json!("sun")
        );

        let moved = SmartBreak {
            meeting_id: "microsoft:AAMk-1760356800000".into(),
            meeting_start_ms: 1_760_356_800_000,
            meeting_end_ms: 1_760_360_400_000,
            break_number: 2,
            kind: SmartBreakKind::EndsBeforeMeeting,
            focus_delta_secs: -480,
            break_start_ms: 1_760_355_900_000,
            break_end_ms: 1_760_356_800_000,
        };
        let json = serde_json::to_value(&moved).expect("serialize");
        assert_eq!(json["kind"], "endsBeforeMeeting");
        assert_eq!(json["focusDeltaSecs"], -480);
        assert_eq!(json["breakNumber"], 2);
        assert_eq!(
            serde_json::from_value::<SmartBreak>(json).expect("deserialize"),
            moved
        );
    }

    #[test]
    fn weekly_stats_wire_format() {
        let mut heatmap = vec![vec![0_u32; 24]; 7];
        heatmap[2][9] = 1800;
        let stats = WeeklyStats {
            revision: 4,
            week_start: "2026-10-12".into(),
            iso_year: 2026,
            iso_week: 42,
            current_week: true,
            first_week_start: Some("2026-09-07".into()),
            history_off: false,
            days: vec![DayStats {
                date: "2026-10-14".into(),
                weekday: Weekday::Wed,
                focus_secs: 1800,
                break_secs: 300,
                sessions: 1,
            }],
            focus_secs: 1800,
            break_secs: 300,
            breaks_taken: 1,
            sessions: 1,
            best_day: Some(Weekday::Wed),
            heatmap: heatmap.clone(),
            recent_heatmap: heatmap,
            recent_weeks: 8,
            top_apps: vec![WeekApp {
                process: "Code.exe".into(),
                secs: 1500,
                category: Some(AppCategory::Productive),
            }],
            categories: CategoryTotals {
                productive_secs: 1500,
                unlabelled_secs: 300,
                ..CategoryTotals::default()
            },
            estimated: false,
        };
        let json = serde_json::to_value(&stats).expect("serialize");
        assert_eq!(json["weekStart"], "2026-10-12");
        assert_eq!(json["isoWeek"], 42);
        assert_eq!(json["bestDay"], "wed");
        assert_eq!(json["heatmap"][2][9], 1800);
        assert_eq!(json["recentHeatmap"].as_array().map(Vec::len), Some(7));
        assert_eq!(
            json["topApps"],
            serde_json::json!([{ "process": "Code.exe", "secs": 1500, "category": "productive" }])
        );
        assert_eq!(
            json["categories"],
            serde_json::json!({ "productiveSecs": 1500, "neutralSecs": 0, "distractingSecs": 0, "unlabelledSecs": 300 })
        );
        assert_eq!(
            serde_json::from_value::<WeeklyStats>(json).expect("deserialize"),
            stats
        );
        assert_eq!(
            serde_json::to_value(HistoryChanged { revision: 5 }).expect("serialize"),
            serde_json::json!({ "revision": 5 })
        );
    }

    /// Writes `names.ts` with command and event names next to the ts-rs bindings.
    #[test]
    fn export_names() {
        let cmds: &[(&str, &str)] = &[
            ("getSnapshot", commands::GET_SNAPSHOT),
            ("startSession", commands::START_SESSION),
            ("pauseSession", commands::PAUSE_SESSION),
            ("resumeSession", commands::RESUME_SESSION),
            ("stopSession", commands::STOP_SESSION),
            ("skipPhase", commands::SKIP_PHASE),
            ("getConfig", commands::GET_CONFIG),
            ("updateConfig", commands::UPDATE_CONFIG),
            ("listFolderTracks", commands::LIST_FOLDER_TRACKS),
            ("getShieldStatus", commands::GET_SHIELD_STATUS),
            ("getCurrentUsage", commands::GET_CURRENT_USAGE),
            ("getLastSummary", commands::GET_LAST_SUMMARY),
            ("listHistory", commands::LIST_HISTORY),
            ("getTeamsStatus", commands::GET_TEAMS_STATUS),
            ("teamsConnect", commands::TEAMS_CONNECT),
            ("teamsDisconnect", commands::TEAMS_DISCONNECT),
            ("getBridgeInfo", commands::GET_BRIDGE_INFO),
            ("openExtensionDir", commands::OPEN_EXTENSION_DIR),
            ("testChime", commands::TEST_CHIME),
            ("openMediaWindow", commands::OPEN_MEDIA_WINDOW),
            ("controlMediaWindow", commands::CONTROL_MEDIA_WINDOW),
            ("getMediaWindow", commands::GET_MEDIA_WINDOW),
            ("getBlocklist", commands::GET_BLOCKLIST),
            ("openBlocklistFile", commands::OPEN_BLOCKLIST_FILE),
            ("reloadBlocklist", commands::RELOAD_BLOCKLIST),
            ("resolveClose", commands::RESOLVE_CLOSE),
            ("getSystemLanguage", commands::GET_SYSTEM_LANGUAGE),
            ("getPopup", commands::GET_POPUP),
            ("popupAction", commands::POPUP_ACTION),
            ("getTintStatus", commands::GET_TINT_STATUS),
            ("previewTint", commands::PREVIEW_TINT),
            ("importChime", commands::IMPORT_CHIME),
            ("removeCustomChime", commands::REMOVE_CUSTOM_CHIME),
            ("listLibraryTracks", commands::LIST_LIBRARY_TRACKS),
            ("clearHistory", commands::CLEAR_HISTORY),
            ("getCalendar", commands::GET_CALENDAR),
            ("refreshCalendar", commands::REFRESH_CALENDAR),
            ("setMeetingChoice", commands::SET_MEETING_CHOICE),
            ("connectGoogle", commands::CONNECT_GOOGLE),
            ("disconnectCalendar", commands::DISCONNECT_CALENDAR),
            ("setIcsUrls", commands::SET_ICS_URLS),
            ("getIcsUrls", commands::GET_ICS_URLS),
            ("getWork", commands::GET_WORK),
            ("refreshWork", commands::REFRESH_WORK),
            ("setWorkToken", commands::SET_WORK_TOKEN),
            ("setSessionGoal", commands::SET_SESSION_GOAL),
            ("markGoalDone", commands::MARK_GOAL_DONE),
            ("devTrigger", commands::DEV_TRIGGER),
            ("getPrivacyReport", commands::GET_PRIVACY_REPORT),
            ("openDataFolder", commands::OPEN_DATA_FOLDER),
            ("exportPrivacyData", commands::EXPORT_PRIVACY_DATA),
            ("deletePrivacyData", commands::DELETE_PRIVACY_DATA),
            ("deleteAllData", commands::DELETE_ALL_DATA),
            ("getYoutubeAccount", commands::GET_YOUTUBE_ACCOUNT),
            ("youtubeSignIn", commands::YOUTUBE_SIGN_IN),
            ("youtubeSignOut", commands::YOUTUBE_SIGN_OUT),
            ("getRewards", commands::GET_REWARDS),
            ("ackRewardMoments", commands::ACK_REWARD_MOMENTS),
            ("resetRewards", commands::RESET_REWARDS),
            ("playRewardSound", commands::PLAY_REWARD_SOUND),
            ("setSessionSteps", commands::SET_SESSION_STEPS),
            ("setStepDone", commands::SET_STEP_DONE),
            ("listSunCities", commands::LIST_SUN_CITIES),
            ("setSmartBreakOptOut", commands::SET_SMART_BREAK_OPT_OUT),
            ("getWeeklyStats", commands::GET_WEEKLY_STATS),
        ];
        let evts: &[(&str, &str)] = &[
            ("sessionSnapshot", events::SESSION_SNAPSHOT),
            ("sessionPhaseChanged", events::SESSION_PHASE_CHANGED),
            ("sessionSummary", events::SESSION_SUMMARY),
            ("shieldsStatus", events::SHIELDS_STATUS),
            ("teamsStatus", events::TEAMS_STATUS),
            ("focusNudge", events::FOCUS_NUDGE),
            ("appError", events::APP_ERROR),
            ("mediaWindow", events::MEDIA_WINDOW),
            ("blocklistChanged", events::BLOCKLIST_CHANGED),
            ("appCloseRequested", events::APP_CLOSE_REQUESTED),
            ("popupShow", events::POPUP_SHOW),
            ("tintChanged", events::TINT_CHANGED),
            ("calendarChanged", events::CALENDAR_CHANGED),
            ("workChanged", events::WORK_CHANGED),
            ("privacyChanged", events::PRIVACY_CHANGED),
            ("youtubeAccount", events::YOUTUBE_ACCOUNT),
            ("rewardsChanged", events::REWARDS_CHANGED),
            ("historyChanged", events::HISTORY_CHANGED),
        ];
        let mut out = String::from(
            "// This file was generated by `cargo test -p focus-core` (api.rs). Do not edit this file manually.\n\n",
        );
        out.push_str("export const COMMANDS = {\n");
        for (k, v) in cmds {
            let _ = writeln!(out, "  {k}: \"{v}\",");
        }
        out.push_str("} as const;\n\nexport const EVENTS = {\n");
        for (k, v) in evts {
            let _ = writeln!(out, "  {k}: \"{v}\",");
        }
        let _ = writeln!(
            out,
            "}} as const;\n\nexport const BRIDGE_PORT = {};",
            super::BRIDGE_PORT
        );

        let dir = std::env::var("TS_RS_EXPORT_DIR").unwrap_or_else(|_| "bindings".into());
        std::fs::create_dir_all(&dir).expect("create bindings dir");
        std::fs::write(std::path::Path::new(&dir).join("names.ts"), out).expect("write names.ts");
    }
}
