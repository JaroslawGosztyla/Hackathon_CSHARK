//! IPC contract between the Svelte UI and the Tauri shell (ADR-006, ADR-012, ADR-013).
//!
//! Every DTO here is exported to TypeScript by ts-rs (`cargo test -p focus-core` writes
//! `src/lib/bindings/`). Command and event names are constants exported to
//! `src/lib/bindings/names.ts` by the `export_names` test, so `src/lib/api.ts` never types them by hand.
//! Change this file only through `/sync-contract`.

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
    /// `{ action: DevAction } -> ()` developer page trigger (ADR-019): a sample popup, a phase toast or a shorter
    /// running phase. Rejected with `developerModeOff` unless `Config.developer_mode` is on. Nothing is persisted
    /// and nothing leaves the computer.
    pub const DEV_TRIGGER: &str = "dev_trigger";
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
    /// Windows toasts. Default false; turned on when the user picks the tray at the first X.
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

/// Warm overlay schedule. Times are minutes after local midnight; the schedule may cross midnight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
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
/// `history.jsonl` or `meeting-choices.json` and nothing is sent over the network.
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
    use super::{DevAction, Phase, PopupInfo, PopupKind, commands, events};
    use std::fmt::Write as _;

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

        assert!(serde_json::from_str::<DevAction>(r#"{"type":"wipeDisk"}"#).is_err());
    }

    #[test]
    fn popup_info_written_before_sticky_is_not_sticky() {
        let info: PopupInfo = serde_json::from_str(
            r#"{"revision":5,"kind":"breakStarted","cycle":2,"phaseSecs":900,"launchers":[],"reminder":null}"#,
        )
        .expect("old popup info");
        assert!(!info.sticky);
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
