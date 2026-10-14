//! Productivity statistics: foreground-app usage during focus and the session summary.
//!
//! Windows shell surfaces (Start, search, notification centre, quick settings, the taskbar and the
//! desktop, Alt+Tab, the lock screen, ...) are not apps the user works in, so they never show up in
//! the chart: [`is_ignored_process`] and [`is_shell_surface_class`] / [`is_shell_surface`] say which
//! foreground windows those are. File Explorer windows (`explorer.exe`, class `CabinetWClass`)
//! still count.
//!
//! Window titles (optional, `Config.track_titles`) are kept per app only to show the most-used ones
//! as a tooltip in the chart. They stay on this computer and must never be logged; the [`Debug`]
//! output of [`UsageAccumulator`] leaves them out.
//!
//! Weekly stats and heatmaps (ADR-023): [`WeekAggregator`] sums the stored sessions of one ISO week (Monday first,
//! local time) and of the [`RECENT_WEEKS`] weeks that end with it. Pure: the shell reads `history.jsonl` line by line
//! and passes every session, the wall clock (`now_ms`) and the UTC offset of the user's time zone at any instant (a
//! function, so daylight saving time is right for every hour of the past weeks).

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::api::{
    AppCategory, AppUsage, CategoryTotals, DayStats, SessionSummary, TitleUsage, UsageReport,
    WeekApp, Weekday, WeeklyStats,
};
use crate::rewards::{civil_from_days, date_string, days_from_civil, days_in_month};
use crate::session::SessionTotals;

const MS_PER_SEC: u64 = 1000;
/// Distinct window titles remembered per app; when full, the least-used title makes room.
pub const MAX_TITLES_PER_APP: usize = 50;
/// Titles are trimmed and shortened to this many characters (an ellipsis marks the cut).
pub const MAX_TITLE_CHARS: usize = 120;
/// Titles per app in [`UsageReport`] (most-used first).
pub const REPORTED_TITLES_PER_APP: usize = 5;
/// Weeks summed in `WeeklyStats.recent_heatmap` (ADR-023), the selected week included.
pub const RECENT_WEEKS: u32 = 8;
/// Apps in `WeeklyStats.top_apps` (ADR-023), most focus time first.
pub const TOP_APPS_PER_WEEK: usize = 10;

/// Executables (lowercase, without `.exe`) of Windows shell surfaces and system UI hosts. Their
/// windows are parts of Windows, not apps, so their foreground time is not counted.
const IGNORED_PROCESSES: [&str; 23] = [
    // Notification centre, toasts and tray flyouts (Windows 10/11).
    "shellexperiencehost",
    // Start menu (Windows 10 1903+, Windows 11).
    "startmenuexperiencehost",
    // Quick settings / control centre (Windows 11 24H2+).
    "shellhost",
    // Search: Windows 11, Windows 11 taskbar search box, Windows 10 2004+, older Windows 10 (Cortana era).
    "searchhost",
    "minisearchhost",
    "searchapp",
    "searchui",
    // Touch keyboard, emoji panel, clipboard history, IME UI (Windows 11 / Windows 10).
    "textinputhost",
    "windowsinternal.composableshell.experiences.textinput.inputapp",
    // Text services framework (language bar, IME candidates).
    "ctfmon",
    // Lock screen, sign-in and Ctrl+Alt+Del screens.
    "lockapp",
    "logonui",
    "winlogon",
    // Widgets board (Windows 11).
    "widgets",
    "widgetboard",
    // People bar (Windows 10).
    "peopleexperiencehost",
    // Win+Shift+S capture overlay (Windows 10).
    "screenclippinghost",
    // UWP frame host: the probe reports the hosted app instead; this name is left only when that failed.
    "applicationframehost",
    // Shell infrastructure, desktop window manager, client/server runtime.
    "sihost",
    "dwm",
    "csrss",
    // UAC prompt (when not on the secure desktop) and Windows credential prompts.
    "consent",
    "credentialuibroker",
];

/// Top-level window classes of Windows shell surfaces (compared case-insensitively, like Windows does).
/// All belong to `explorer.exe` or to a replacement shell, never to a document window.
const SHELL_SURFACE_CLASSES: [&str; 12] = [
    // Taskbar on the primary monitor and on the others.
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    // Desktop (icons, wallpaper, after Win+D).
    "Progman",
    "WorkerW",
    // Hidden tray icons: Windows 10, Windows 11.
    "NotifyIconOverflowWindow",
    "TopLevelWindowForOverflowXamlIsland",
    // Taskbar thumbnail previews.
    "TaskListThumbnailWnd",
    // Alt+Tab / Task View: Windows 10, Windows 11 (also Snap Assist).
    "MultitaskingViewFrame",
    "XamlExplorerHostIslandWindow",
    // Transient windows while switching apps or closing a shell flyout.
    "ForegroundStaging",
    "Shell_LightDismissOverlay",
    // Input language switcher (Win+Space).
    "Shell_InputSwitchTopLevelWindow",
];

/// Class of UWP / immersive windows. Real Store apps use it too (in their own process), so it
/// marks a shell surface only when `explorer.exe` owns it (Windows 10 Task View) or the process is
/// ignored anyway.
const CORE_WINDOW_CLASS: &str = "Windows.UI.Core.CoreWindow";
const EXPLORER: &str = "explorer";

/// `true` for an executable of a Windows shell surface or system UI host (`ShellExperienceHost.exe`,
/// `StartMenuExperienceHost.exe`, `SearchHost.exe`, `LockApp.exe`, `dwm.exe`, ...): its foreground
/// time is not counted. Accepts a bare name or a full path, with or without `.exe`, any case.
/// `explorer.exe` is not ignored here (File Explorer windows count); see [`is_shell_surface_class`].
pub fn is_ignored_process(name: &str) -> bool {
    let stem = process_stem(name);
    IGNORED_PROCESSES.contains(&stem.as_str())
}

/// `true` for the window class of a shell surface that is not an app window: the taskbars
/// (`Shell_TrayWnd`, `Shell_SecondaryTrayWnd`), the desktop (`Progman`, `WorkerW`), the hidden-icons
/// flyouts, taskbar thumbnails, Alt+Tab / Task View and the input switcher. Case-insensitive.
/// `Windows.UI.Core.CoreWindow` is not included: Store apps use it too (see [`is_shell_surface`]).
pub fn is_shell_surface_class(class: &str) -> bool {
    let class = class.trim();
    SHELL_SURFACE_CLASSES
        .iter()
        .any(|known| class.eq_ignore_ascii_case(known))
}

/// `true` when the foreground window is part of the Windows shell and its time should not be
/// counted: an [`is_ignored_process`] executable, a [`is_shell_surface_class`] window (e.g.
/// `explorer.exe` showing the taskbar or the desktop), or a `Windows.UI.Core.CoreWindow` owned by
/// `explorer.exe` (Windows 10 Task View). An `explorer.exe` folder window (`CabinetWClass`) is not.
pub fn is_shell_surface(process: Option<&str>, class: Option<&str>) -> bool {
    if process.is_some_and(is_ignored_process) {
        return true;
    }
    let Some(class) = class.map(str::trim) else {
        return false;
    };
    is_shell_surface_class(class)
        || (class.eq_ignore_ascii_case(CORE_WINDOW_CLASS)
            && process.is_some_and(|p| process_stem(p) == EXPLORER))
}

/// Lowercase file name without a trailing `.exe` (`C:\Windows\Explorer.EXE` → `explorer`).
fn process_stem(name: &str) -> String {
    let trimmed = name.trim();
    let file_name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    let lower = file_name.to_lowercase();
    match lower.strip_suffix(".exe") {
        Some(stem) => stem.to_owned(),
        None => lower,
    }
}

/// Accumulates foreground samples of one session. Process names are grouped
/// case-insensitively; the first spelling seen is the one reported.
#[derive(Clone, Default)]
pub struct UsageAccumulator {
    /// Lowercase process name → reported name, time and title times.
    apps: HashMap<String, AppTime>,
    idle_ms: u64,
    unknown_ms: u64,
}

#[derive(Clone)]
struct AppTime {
    name: String,
    ms: u64,
    /// Normalized title → milliseconds; at most [`MAX_TITLES_PER_APP`] entries.
    titles: HashMap<String, u64>,
}

impl AppTime {
    fn add_title(&mut self, title: String, ms: u64) {
        if let Some(time) = self.titles.get_mut(&title) {
            *time = time.saturating_add(ms);
            return;
        }
        if self.titles.len() >= MAX_TITLES_PER_APP {
            // Make room by forgetting the least-used title (ties: the alphabetically last one), so a
            // title used for long stretches later in the session still makes it into the report.
            let least = self
                .titles
                .iter()
                .min_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
                .map(|(title, _)| title.clone());
            if let Some(least) = least {
                self.titles.remove(&least);
            }
        }
        self.titles.insert(title, ms);
    }

    fn top_titles(&self) -> Vec<TitleUsage> {
        let mut titles: Vec<TitleUsage> = self
            .titles
            .iter()
            .map(|(title, &ms)| TitleUsage {
                title: title.clone(),
                secs: ms_to_rounded_secs(ms),
            })
            .filter(|title| title.secs > 0)
            .collect();
        titles.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.title.cmp(&b.title)));
        titles.truncate(REPORTED_TITLES_PER_APP);
        titles
    }
}

/// Titles are private (privacy rule 11): only counts and names of processes are printed.
impl fmt::Debug for UsageAccumulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut apps: Vec<(&str, u64, usize)> = self
            .apps
            .values()
            .map(|app| (app.name.as_str(), app.ms, app.titles.len()))
            .collect();
        apps.sort_unstable();
        f.debug_struct("UsageAccumulator")
            // (process, ms, number of titles)
            .field("apps", &apps)
            .field("idle_ms", &self.idle_ms)
            .field("unknown_ms", &self.unknown_ms)
            .finish()
    }
}

impl UsageAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `ms` of focus time.
    ///
    /// - `idle` counts as idle regardless of the process,
    /// - `process` `None` (or blank) counts as unknown,
    /// - an [`is_ignored_process`] (Start, search, lock screen, ...) is not counted at all. The shell
    ///   also skips samples whose window is a shell surface by class ([`is_shell_surface`]), e.g. the
    ///   taskbar or the desktop of `explorer.exe`,
    /// - `title` (pass `None` when `track_titles` is off) is added to that app's title times when it
    ///   is not blank: trimmed, control characters replaced by spaces, shortened to
    ///   [`MAX_TITLE_CHARS`]. Titles of idle or unknown time are dropped.
    pub fn add_sample(&mut self, process: Option<&str>, title: Option<&str>, idle: bool, ms: u64) {
        if idle {
            self.idle_ms = self.idle_ms.saturating_add(ms);
            return;
        }
        let Some(name) = process.map(str::trim).filter(|name| !name.is_empty()) else {
            self.unknown_ms = self.unknown_ms.saturating_add(ms);
            return;
        };
        if is_ignored_process(name) {
            return;
        }
        let entry = self
            .apps
            .entry(name.to_lowercase())
            .or_insert_with(|| AppTime {
                name: name.to_owned(),
                ms: 0,
                titles: HashMap::new(),
            });
        entry.ms = entry.ms.saturating_add(ms);
        if let Some(title) = title.and_then(normalize_title) {
            entry.add_title(title, ms);
        }
    }

    /// Usage so far: apps sorted by seconds (descending) then by name, each value rounded to
    /// whole seconds (apps that round to 0 s are left out); `tracked_secs` is the sum of all parts.
    /// Each app lists its [`REPORTED_TITLES_PER_APP`] most-used titles (seconds descending, then
    /// by title; titles that round to 0 s are left out; empty when no titles were given).
    pub fn report(&self) -> UsageReport {
        let mut apps: Vec<AppUsage> = self
            .apps
            .values()
            .filter_map(|app| {
                let secs = ms_to_rounded_secs(app.ms);
                (secs > 0).then(|| AppUsage {
                    process: app.name.clone(),
                    secs,
                    titles: app.top_titles(),
                })
            })
            .collect();
        apps.sort_by_cached_key(|app| (Reverse(app.secs), app.process.to_lowercase()));

        let idle_secs = ms_to_rounded_secs(self.idle_ms);
        let unknown_secs = ms_to_rounded_secs(self.unknown_ms);
        let tracked_secs = apps
            .iter()
            .fold(idle_secs.saturating_add(unknown_secs), |sum, app| {
                sum.saturating_add(app.secs)
            });

        UsageReport {
            apps,
            idle_secs,
            unknown_secs,
            tracked_secs,
        }
    }

    /// Forgets all samples (new session).
    pub fn clear(&mut self) {
        self.apps.clear();
        self.idle_ms = 0;
        self.unknown_ms = 0;
    }

    /// No time recorded yet.
    pub fn is_empty(&self) -> bool {
        self.idle_ms == 0 && self.unknown_ms == 0 && self.apps.values().all(|app| app.ms == 0)
    }
}

/// Merges session totals and usage into the summary DTO kept in history.
pub fn summarize(totals: &SessionTotals, usage: UsageReport) -> SessionSummary {
    SessionSummary {
        session_id: totals.session_id.clone(),
        started_at_ms: totals.started_at_ms,
        ended_at_ms: totals.ended_at_ms,
        focused_secs: totals.focused_secs,
        break_secs: totals.break_secs,
        cycles_completed: totals.cycles_completed,
        usage,
        demo: totals.demo,
        goal: totals.goal.clone(),
        goal_done: None,
        // The small steps with their ticks (beta, ADR-022); the shell fills `sparks` while the rewards are on.
        steps: totals.steps.clone(),
        sparks: None,
        // Timestamps only (ADR-023): the weekly heatmaps place the focus by them.
        focus_spans: totals.focus_spans.clone(),
    }
}

/// Title as stored: control characters become spaces, trimmed, at most [`MAX_TITLE_CHARS`]
/// characters (a longer one keeps its first `MAX_TITLE_CHARS - 1` characters plus `…`).
/// `None` when nothing is left.
fn normalize_title(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() <= MAX_TITLE_CHARS {
        return Some(trimmed.to_owned());
    }
    let mut short: String = trimmed.chars().take(MAX_TITLE_CHARS - 1).collect();
    short.truncate(short.trim_end().len());
    short.push('…');
    Some(short)
}

fn ms_to_rounded_secs(ms: u64) -> u32 {
    u32::try_from(ms.saturating_add(MS_PER_SEC / 2) / MS_PER_SEC).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------------------------------------------
// Weekly stats and heatmaps (ADR-023)
// ---------------------------------------------------------------------------------------------------------------

/// A local calendar day: days since 1970-01-01 in the user's time zone (the numbering of `rewards::RewardDay`,
/// without its 04:00 shift).
pub type LocalDay = i32;

/// Entries of `WeeklyStats.days` and rows of the heatmaps (Monday..Sunday).
pub const DAYS_PER_WEEK: usize = 7;
/// Columns of the heatmaps: local hours 00..23.
pub const HOURS_PER_DAY: usize = 24;
/// Most focus spans a session records (`SessionTotals::focus_spans`; beyond it the last one is stretched) and the
/// weekly stats read of one stored session.
pub const MAX_FOCUS_SPANS: usize = 256;
/// UTC offsets from the shell are clamped to ±18 h (real time zones are within −12 h..+14 h).
pub const MAX_UTC_OFFSET_MINUTES: i32 = 18 * 60;

const MS_PER_MINUTE: i64 = 60_000;
const MS_PER_HOUR: i64 = 60 * MS_PER_MINUTE;
const MS_PER_DAY: i64 = 24 * MS_PER_HOUR;
/// The offset from the shell is remembered per this much UTC time: every UTC offset and every daylight-saving change
/// in use falls on a quarter hour (Newfoundland changes at 05:30 UTC), so one slot never holds two offsets.
const OFFSET_SLOT_MS: i64 = 15 * MS_PER_MINUTE;
const WEEK_DAYS: LocalDay = 7;
/// 1970-01-01 (day 0) was a Thursday: weekday index 3 with Monday = 0.
const EPOCH_WEEKDAY: i64 = 3;
/// The Thursday of a week decides its ISO week-numbering year (ISO 8601).
const ISO_THURSDAY: i64 = 3;
const WEEKDAYS: [Weekday; DAYS_PER_WEEK] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];
/// Years `parse_local_date` accepts.
const MIN_YEAR: u32 = 1;
const MAX_YEAR: u32 = 9999;

/// `YYYY-MM-DD` (exactly: four-digit year 0001..=9999, two-digit month and day) → its local day; `None` for anything
/// else or a date that does not exist (`2026-02-30`).
pub fn parse_local_date(text: &str) -> Option<LocalDay> {
    let number = |from: usize, to: usize| -> Option<u32> {
        text.get(from..to)?
            .chars()
            .try_fold(0_u32, |acc, c| Some(acc * 10 + c.to_digit(10)?))
    };
    if text.len() != 10 || text.get(4..5) != Some("-") || text.get(7..8) != Some("-") {
        return None;
    }
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let year_i32 = i32::try_from(year).ok()?;
    if !(MIN_YEAR..=MAX_YEAR).contains(&year)
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year_i32, month)
    {
        return None;
    }
    LocalDay::try_from(days_from_civil(i64::from(year), month, day)).ok()
}

/// `YYYY-MM-DD` of a local day.
pub fn local_date_string(day: LocalDay) -> String {
    date_string(day)
}

/// 0 = Monday ... 6 = Sunday.
fn weekday_index(day: LocalDay) -> usize {
    usize::try_from((i64::from(day) + EPOCH_WEEKDAY).rem_euclid(7)).unwrap_or(0)
}

/// The weekday of a local day.
pub fn weekday_of(day: LocalDay) -> Weekday {
    WEEKDAYS
        .get(weekday_index(day))
        .copied()
        .unwrap_or(Weekday::Mon)
}

/// The Monday of the week (ISO 8601: Monday first) that contains `day`.
pub fn week_monday(day: LocalDay) -> LocalDay {
    let back = LocalDay::try_from(weekday_index(day)).unwrap_or(0);
    day.saturating_sub(back)
}

/// ISO 8601 week-numbering year and week (1..=53) of the week that contains `day` (`2026-10-13` → `(2026, 42)`;
/// `2027-01-01`, a Friday, → `(2026, 53)`).
pub fn iso_week(day: LocalDay) -> (u32, u32) {
    let thursday = i64::from(week_monday(day)) + ISO_THURSDAY;
    let (year, _, _) = civil_from_days(thursday);
    let first = days_from_civil(year, 1, 1);
    let week = (thursday - first).div_euclid(i64::from(WEEK_DAYS)) + 1;
    (
        u32::try_from(year).unwrap_or(0),
        u32::try_from(week).unwrap_or(1),
    )
}

/// The request of `get_weekly_stats` as the shell resolved it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekQuery {
    /// Any day of the week to show (`weekStart` through [`parse_local_date`]).
    pub day: LocalDay,
    /// Wall clock now (Unix epoch milliseconds): which week is the current one.
    pub now_ms: i64,
    /// `HistoryChanged.revision` the history was read at (`WeeklyStats.revision`).
    pub revision: u32,
    /// `Config.keep_history` is off (`WeeklyStats.history_off`).
    pub history_off: bool,
}

/// One app of the selected week while summing (first spelling seen, seconds).
struct AppSum {
    name: String,
    secs: u64,
}

/// Sums the stored sessions into one [`WeeklyStats`]: feed every session with [`WeekAggregator::add`] (any order,
/// streaming), then [`WeekAggregator::finish`]. Demo sessions are left out everywhere.
///
/// - Focus by local day and hour comes from `SessionSummary.focus_spans`, split at every local hour (and so at
///   midnight, also across a daylight-saving change). A session stored without spans (older history) spreads its
///   `focused_secs` evenly between its start and end, and the stats say `estimated` when one of those lands in the
///   [`RECENT_WEEKS`] window.
/// - Sessions, breaks, break time and apps count for the local day the session started on.
/// - `offset_minutes(utc_ms)` is the user's UTC offset at that instant in minutes east of UTC (CET = 60, CEST = 120);
///   it is asked once per UTC quarter hour of the window and clamped to ±[`MAX_UTC_OFFSET_MINUTES`].
pub struct WeekAggregator<'a, F> {
    query: WeekQuery,
    week_start: LocalDay,
    week_end: LocalDay,
    recent_start: LocalDay,
    /// UTC instants outside which no focus can land in the window, whatever the offset.
    window_from_ms: i64,
    window_to_ms: i64,
    categories: &'a BTreeMap<String, AppCategory>,
    offset_minutes: F,
    /// UTC quarter hour ([`OFFSET_SLOT_MS`]) → offset in milliseconds, inside the window only.
    offsets: HashMap<i64, i64>,
    week_ms: [[u64; HOURS_PER_DAY]; DAYS_PER_WEEK],
    recent_ms: [[u64; HOURS_PER_DAY]; DAYS_PER_WEEK],
    day_break_secs: [u32; DAYS_PER_WEEK],
    day_sessions: [u32; DAYS_PER_WEEK],
    breaks_taken: u32,
    /// Lowercase process name → reported name and seconds.
    apps: HashMap<String, AppSum>,
    first_day: Option<LocalDay>,
    estimated: bool,
}

impl<'a, F: FnMut(i64) -> i32> WeekAggregator<'a, F> {
    /// `categories` = `Config.app_categories` (lowercase exe name → label).
    pub fn new(
        query: WeekQuery,
        categories: &'a BTreeMap<String, AppCategory>,
        offset_minutes: F,
    ) -> Self {
        let week_start = week_monday(query.day);
        let week_end = week_start.saturating_add(WEEK_DAYS);
        let recent_weeks = LocalDay::try_from(RECENT_WEEKS).unwrap_or(1);
        let recent_start = week_end.saturating_sub(WEEK_DAYS.saturating_mul(recent_weeks));
        let max_offset_ms = i64::from(MAX_UTC_OFFSET_MINUTES) * MS_PER_MINUTE;
        Self {
            query,
            week_start,
            week_end,
            recent_start,
            window_from_ms: (i64::from(recent_start) * MS_PER_DAY).saturating_sub(max_offset_ms),
            window_to_ms: (i64::from(week_end) * MS_PER_DAY).saturating_add(max_offset_ms),
            categories,
            offset_minutes,
            offsets: HashMap::new(),
            week_ms: [[0; HOURS_PER_DAY]; DAYS_PER_WEEK],
            recent_ms: [[0; HOURS_PER_DAY]; DAYS_PER_WEEK],
            day_break_secs: [0; DAYS_PER_WEEK],
            day_sessions: [0; DAYS_PER_WEEK],
            breaks_taken: 0,
            apps: HashMap::new(),
            first_day: None,
            estimated: false,
        }
    }

    /// Adds one stored session (demo sessions are skipped).
    pub fn add(&mut self, summary: &SessionSummary) {
        if summary.demo {
            return;
        }
        if let Some(start_day) = self.local_day(summary.started_at_ms) {
            self.first_day = Some(
                self.first_day
                    .map_or(start_day, |first| first.min(start_day)),
            );
            if let Some(index) = self.index_in_week(start_day) {
                self.add_session_counts(index, summary);
            }
        }
        if summary.focus_spans.is_empty() {
            let spread_ms = u64::from(summary.focused_secs).saturating_mul(MS_PER_SEC);
            if spread_ms > 0
                && self.add_interval(summary.started_at_ms, summary.ended_at_ms, Some(spread_ms))
            {
                self.estimated = true;
            }
        } else {
            for span in summary.focus_spans.iter().take(MAX_FOCUS_SPANS) {
                self.add_interval(span.start_ms, span.end_ms, None);
            }
        }
    }

    /// The stats of the week.
    pub fn finish(mut self) -> WeeklyStats {
        let today = self.local_day(self.query.now_ms);
        let days: Vec<DayStats> = (0..DAYS_PER_WEEK)
            .map(|index| {
                let offset = LocalDay::try_from(index).unwrap_or(0);
                let day = self.week_start.saturating_add(offset);
                let focus_ms = self.week_ms.get(index).map_or(0, |hours| {
                    hours.iter().fold(0_u64, |sum, ms| sum.saturating_add(*ms))
                });
                DayStats {
                    date: local_date_string(day),
                    weekday: weekday_of(day),
                    focus_secs: ms_to_rounded_secs(focus_ms),
                    break_secs: self.day_break_secs.get(index).copied().unwrap_or(0),
                    sessions: self.day_sessions.get(index).copied().unwrap_or(0),
                }
            })
            .collect();
        let sum = |pick: fn(&DayStats) -> u32| {
            days.iter()
                .fold(0_u32, |total, day| total.saturating_add(pick(day)))
        };
        let (focus_secs, break_secs, sessions) = (
            sum(|day| day.focus_secs),
            sum(|day| day.break_secs),
            sum(|day| day.sessions),
        );
        // The first day with the most focus (`max_by_key` would keep the last one on a tie).
        let best_day = days
            .iter()
            .filter(|day| day.focus_secs > 0)
            .fold(None::<&DayStats>, |best, day| match best {
                Some(best) if best.focus_secs >= day.focus_secs => Some(best),
                _ => Some(day),
            })
            .map(|day| day.weekday);
        let (top_apps, categories) = self.apps_of_week();
        let (iso_year, iso_week) = iso_week(self.week_start);
        WeeklyStats {
            revision: self.query.revision,
            week_start: local_date_string(self.week_start),
            iso_year,
            iso_week,
            current_week: today.is_some_and(|today| week_monday(today) == self.week_start),
            first_week_start: self
                .first_day
                .map(|day| local_date_string(week_monday(day))),
            history_off: self.query.history_off,
            days,
            focus_secs,
            break_secs,
            breaks_taken: self.breaks_taken,
            sessions,
            best_day,
            heatmap: rounded_heatmap(&self.week_ms),
            recent_heatmap: rounded_heatmap(&self.recent_ms),
            recent_weeks: RECENT_WEEKS,
            top_apps,
            categories,
            estimated: self.estimated,
        }
    }

    /// Sessions, breaks, break time and apps of a session that started on day `index` of the week.
    fn add_session_counts(&mut self, index: usize, summary: &SessionSummary) {
        if let Some(count) = self.day_sessions.get_mut(index) {
            *count = count.saturating_add(1);
        }
        if let Some(secs) = self.day_break_secs.get_mut(index) {
            *secs = secs.saturating_add(summary.break_secs);
        }
        self.breaks_taken = self.breaks_taken.saturating_add(summary.cycles_completed);
        for app in &summary.usage.apps {
            let name = app.process.trim();
            if name.is_empty() || app.secs == 0 {
                continue;
            }
            let entry = self
                .apps
                .entry(name.to_lowercase())
                .or_insert_with(|| AppSum {
                    name: name.to_owned(),
                    secs: 0,
                });
            entry.secs = entry.secs.saturating_add(u64::from(app.secs));
        }
    }

    /// The [`TOP_APPS_PER_WEEK`] apps with the most time (then by name) and the time of every app per label.
    fn apps_of_week(&self) -> (Vec<WeekApp>, CategoryTotals) {
        let mut totals = CategoryTotals::default();
        let mut apps: Vec<WeekApp> = self
            .apps
            .iter()
            .map(|(key, app)| {
                let category = self.categories.get(key).copied();
                let secs = u32::try_from(app.secs).unwrap_or(u32::MAX);
                let slot = match category {
                    Some(AppCategory::Productive) => &mut totals.productive_secs,
                    Some(AppCategory::Neutral) => &mut totals.neutral_secs,
                    Some(AppCategory::Distracting) => &mut totals.distracting_secs,
                    None => &mut totals.unlabelled_secs,
                };
                *slot = slot.saturating_add(secs);
                WeekApp {
                    process: app.name.clone(),
                    secs,
                    category,
                }
            })
            .collect();
        apps.sort_by_cached_key(|app| (Reverse(app.secs), app.process.to_lowercase()));
        apps.truncate(TOP_APPS_PER_WEEK);
        (apps, totals)
    }

    /// Index 0..7 of `day` in the selected week.
    fn index_in_week(&self, day: LocalDay) -> Option<usize> {
        (self.week_start..self.week_end)
            .contains(&day)
            .then(|| usize::try_from(day - self.week_start).ok())
            .flatten()
    }

    /// The user's UTC offset at `utc_ms`, in milliseconds (remembered per UTC quarter hour inside the window).
    fn offset_ms(&mut self, utc_ms: i64) -> i64 {
        let slot = utc_ms.div_euclid(OFFSET_SLOT_MS);
        if let Some(offset) = self.offsets.get(&slot) {
            return *offset;
        }
        let minutes =
            (self.offset_minutes)(utc_ms).clamp(-MAX_UTC_OFFSET_MINUTES, MAX_UTC_OFFSET_MINUTES);
        let offset = i64::from(minutes) * MS_PER_MINUTE;
        if (self.window_from_ms..self.window_to_ms).contains(&utc_ms) {
            self.offsets.insert(slot, offset);
        }
        offset
    }

    /// The local day of a UTC instant; `None` for instants out of any calendar.
    fn local_day(&mut self, utc_ms: i64) -> Option<LocalDay> {
        let local = utc_ms.checked_add(self.offset_ms(utc_ms))?;
        LocalDay::try_from(local.div_euclid(MS_PER_DAY)).ok()
    }

    /// Adds `[start_ms, end_ms)` of focus to the heatmaps, cut to the window and split at every local hour. With
    /// `spread_ms` the interval holds that much focus spread evenly (at most its length); without, all of it is focus.
    /// `true` when some focus landed in the window.
    fn add_interval(&mut self, start_ms: i64, end_ms: i64, spread_ms: Option<u64>) -> bool {
        let Some(length) = end_ms
            .checked_sub(start_ms)
            .and_then(|length| u64::try_from(length).ok())
            .filter(|length| *length > 0)
        else {
            return false;
        };
        let from = start_ms.max(self.window_from_ms);
        let to = end_ms.min(self.window_to_ms);
        let mut landed = false;
        let mut at = from;
        while at < to {
            let local = at.saturating_add(self.offset_ms(at));
            let to_next_hour = MS_PER_HOUR - local.rem_euclid(MS_PER_HOUR);
            let chunk_end = to.min(at.saturating_add(to_next_hour));
            let chunk = u64::try_from(chunk_end - at).unwrap_or(0);
            let ms = match spread_ms {
                None => chunk,
                Some(spread) => scale(chunk, spread.min(length), length),
            };
            landed |= self.add_focus(local, ms);
            at = chunk_end;
        }
        landed
    }

    /// Adds `ms` of focus at local instant `local_ms` (Unix ms shifted by the offset) when its day is in the window.
    fn add_focus(&mut self, local_ms: i64, ms: u64) -> bool {
        let Ok(day) = LocalDay::try_from(local_ms.div_euclid(MS_PER_DAY)) else {
            return false;
        };
        if !(self.recent_start..self.week_end).contains(&day) || ms == 0 {
            return false;
        }
        let weekday = weekday_index(day);
        let hour = usize::try_from(local_ms.rem_euclid(MS_PER_DAY) / MS_PER_HOUR).unwrap_or(0);
        if let Some(cell) = self
            .recent_ms
            .get_mut(weekday)
            .and_then(|row| row.get_mut(hour))
        {
            *cell = cell.saturating_add(ms);
        }
        if day >= self.week_start
            && let Some(cell) = self
                .week_ms
                .get_mut(weekday)
                .and_then(|row| row.get_mut(hour))
        {
            *cell = cell.saturating_add(ms);
        }
        true
    }
}

/// `part * total / whole` without overflow (`whole > 0`).
fn scale(part: u64, total: u64, whole: u64) -> u64 {
    let scaled = u128::from(part) * u128::from(total) / u128::from(whole.max(1));
    u64::try_from(scaled).unwrap_or(u64::MAX)
}

fn rounded_heatmap(ms: &[[u64; HOURS_PER_DAY]; DAYS_PER_WEEK]) -> Vec<Vec<u32>> {
    ms.iter()
        .map(|hours| hours.iter().map(|ms| ms_to_rounded_secs(*ms)).collect())
        .collect()
}

/// [`WeekAggregator`] over a list of sessions (tests and small callers).
pub fn weekly_stats<'s, F: FnMut(i64) -> i32>(
    query: WeekQuery,
    categories: &BTreeMap<String, AppCategory>,
    offset_minutes: F,
    sessions: impl IntoIterator<Item = &'s SessionSummary>,
) -> WeeklyStats {
    let mut aggregator = WeekAggregator::new(query, categories, offset_minutes);
    for session in sessions {
        aggregator.add(session);
    }
    aggregator.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{MicroStep, SessionGoal, TimeSpan, WorkSource};

    fn usage(process: &str, secs: u32) -> AppUsage {
        AppUsage {
            process: process.to_owned(),
            secs,
            titles: Vec::new(),
        }
    }

    fn title(title: &str, secs: u32) -> TitleUsage {
        TitleUsage {
            title: title.to_owned(),
            secs,
        }
    }

    #[test]
    fn empty_accumulator_reports_nothing() {
        let acc = UsageAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.report(), UsageReport::default());
    }

    #[test]
    fn groups_process_names_case_insensitively_keeping_first_spelling() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("Code.exe"), None, false, 1000);
        acc.add_sample(Some("code.EXE"), None, false, 2000);
        acc.add_sample(Some("  CODE.exe "), None, false, 1000);
        let report = acc.report();
        assert_eq!(report.apps, vec![usage("Code.exe", 4)]);
        assert_eq!(report.tracked_secs, 4);
    }

    #[test]
    fn idle_wins_over_process_and_missing_process_is_unknown() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("chrome.exe"), Some("Inbox"), true, 3000);
        acc.add_sample(None, None, true, 1000);
        acc.add_sample(None, Some("Something"), false, 2000);
        acc.add_sample(Some("   "), None, false, 1000);
        acc.add_sample(Some("chrome.exe"), None, false, 5000);
        let report = acc.report();
        assert_eq!(report.idle_secs, 4);
        assert_eq!(report.unknown_secs, 3);
        // Titles of idle and unknown time are dropped.
        assert_eq!(report.apps, vec![usage("chrome.exe", 5)]);
        assert_eq!(report.tracked_secs, 12);
    }

    #[test]
    fn sorts_by_seconds_then_name() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("slack.exe"), None, false, 10_000);
        acc.add_sample(Some("Code.exe"), None, false, 60_000);
        acc.add_sample(Some("chrome.exe"), None, false, 10_000);
        acc.add_sample(Some("Teams.exe"), None, false, 30_000);
        acc.add_sample(Some("brave.exe"), None, false, 10_000);
        let report = acc.report();
        assert_eq!(
            report.apps,
            vec![
                usage("Code.exe", 60),
                usage("Teams.exe", 30),
                usage("brave.exe", 10),
                usage("chrome.exe", 10),
                usage("slack.exe", 10),
            ]
        );
        assert_eq!(report.tracked_secs, 120);
    }

    #[test]
    fn rounds_to_nearest_second_and_drops_zero_second_apps() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("a.exe"), None, false, 1499);
        acc.add_sample(Some("b.exe"), None, false, 1500);
        acc.add_sample(Some("tiny.exe"), Some("Tiny"), false, 499);
        acc.add_sample(None, None, false, 500);
        acc.add_sample(None, None, true, 2499);
        let report = acc.report();
        assert_eq!(report.apps, vec![usage("b.exe", 2), usage("a.exe", 1)]);
        assert_eq!(report.unknown_secs, 1);
        assert_eq!(report.idle_secs, 2);
        assert_eq!(report.tracked_secs, 2 + 1 + 1 + 2);
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("a.exe"), Some("Doc"), false, u64::MAX);
        acc.add_sample(Some("a.exe"), Some("Doc"), false, u64::MAX);
        acc.add_sample(None, None, true, u64::MAX);
        let report = acc.report();
        assert_eq!(
            report.apps,
            vec![AppUsage {
                titles: vec![title("Doc", u32::MAX)],
                ..usage("a.exe", u32::MAX)
            }]
        );
        assert_eq!(report.idle_secs, u32::MAX);
        assert_eq!(report.tracked_secs, u32::MAX);
    }

    #[test]
    fn clear_forgets_everything() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("Code.exe"), Some("main.rs"), false, 5000);
        acc.add_sample(None, None, true, 5000);
        assert!(!acc.is_empty());
        acc.clear();
        assert!(acc.is_empty());
        assert_eq!(acc.report(), UsageReport::default());
        // A new spelling wins after clearing; old titles are gone.
        acc.add_sample(Some("CODE.EXE"), None, false, 1000);
        assert_eq!(acc.report().apps, vec![usage("CODE.EXE", 1)]);
    }

    #[test]
    fn keeps_top_titles_per_app() {
        let mut acc = UsageAccumulator::new();
        for (name, secs) in [
            ("Inbox - Outlook", 3),
            ("Pull request #42 - GitHub", 40),
            ("YouTube", 2),
            ("Docs", 10),
            ("Jira board", 10),
            ("Calendar", 5),
            ("Wikipedia", 1),
        ] {
            for _ in 0..secs {
                acc.add_sample(Some("chrome.exe"), Some(name), false, 1000);
            }
        }
        // Time without a title still counts for the app.
        acc.add_sample(Some("chrome.exe"), None, false, 4000);
        acc.add_sample(Some("chrome.exe"), Some("   "), false, 1000);
        acc.add_sample(Some("Code.exe"), Some("stats.rs - FocusMode"), false, 2000);

        let report = acc.report();
        assert_eq!(
            report.apps,
            vec![
                AppUsage {
                    process: "chrome.exe".to_owned(),
                    secs: 76,
                    titles: vec![
                        title("Pull request #42 - GitHub", 40),
                        title("Docs", 10),
                        title("Jira board", 10),
                        title("Calendar", 5),
                        title("Inbox - Outlook", 3),
                    ],
                },
                AppUsage {
                    process: "Code.exe".to_owned(),
                    secs: 2,
                    titles: vec![title("stats.rs - FocusMode", 2)],
                },
            ]
        );
        assert_eq!(report.tracked_secs, 78);
    }

    #[test]
    fn titles_are_trimmed_cleaned_and_shortened() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("app.exe"), Some("  Report\r\nQ3\t "), false, 1000);
        acc.add_sample(Some("app.exe"), Some("Report  Q3"), false, 1000);
        let long = "ż".repeat(MAX_TITLE_CHARS + 30);
        acc.add_sample(Some("app.exe"), Some(&long), false, 3000);
        let exact = "a".repeat(MAX_TITLE_CHARS);
        acc.add_sample(Some("app.exe"), Some(&exact), false, 500);
        // The cut does not leave a space before the ellipsis.
        let spaced = format!("{} tail", "b".repeat(MAX_TITLE_CHARS - 2));
        acc.add_sample(Some("app.exe"), Some(&spaced), false, 400);

        let titles = acc.report().apps[0].titles.clone();
        let shortened = format!("{}…", "ż".repeat(MAX_TITLE_CHARS - 1));
        assert_eq!(
            titles,
            vec![
                title(&shortened, 3),
                title("Report  Q3", 2),
                title(&exact, 1),
            ]
        );
        assert!(
            titles
                .iter()
                .all(|t| t.title.chars().count() <= MAX_TITLE_CHARS)
        );
        assert_eq!(
            normalize_title(&spaced),
            Some(format!("{}…", "b".repeat(MAX_TITLE_CHARS - 2)))
        );
        assert_eq!(normalize_title("\n\t "), None);
        assert_eq!(normalize_title(""), None);
    }

    #[test]
    fn caps_distinct_titles_per_app_dropping_the_least_used() {
        let mut acc = UsageAccumulator::new();
        // One long-running title, then many short-lived ones (e.g. browser tabs).
        acc.add_sample(Some("firefox.exe"), Some("Focus doc"), false, 30_000);
        for i in 0..200 {
            acc.add_sample(Some("firefox.exe"), Some(&format!("Tab {i}")), false, 1000);
        }
        // A title used for a long time late in the session still gets in.
        for _ in 0..20 {
            acc.add_sample(Some("firefox.exe"), Some("Late but long"), false, 1000);
        }
        let app = acc.apps.get("firefox.exe").expect("app");
        assert_eq!(app.titles.len(), MAX_TITLES_PER_APP);
        let report = acc.report();
        assert_eq!(report.apps[0].secs, 30 + 200 + 20);
        let top = &report.apps[0].titles;
        assert_eq!(top.len(), REPORTED_TITLES_PER_APP);
        assert_eq!(top[0], title("Focus doc", 30));
        assert_eq!(top[1], title("Late but long", 20));
        assert!(
            top[2..]
                .iter()
                .all(|t| t.secs == 1 && t.title.starts_with("Tab "))
        );
    }

    #[test]
    fn ignored_shell_processes_are_not_counted() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(
            Some("ShellExperienceHost.exe"),
            Some("New notification"),
            false,
            3000,
        );
        acc.add_sample(
            Some("StartMenuExperienceHost.exe"),
            Some("Start"),
            false,
            2000,
        );
        acc.add_sample(Some("SearchHost.exe"), Some("Search"), false, 2000);
        acc.add_sample(Some("LockApp.exe"), None, false, 2000);
        // Idle still wins (screen locked and the user away).
        acc.add_sample(Some("LockApp.exe"), None, true, 5000);
        acc.add_sample(Some("explorer.exe"), Some("Downloads"), false, 4000);
        let report = acc.report();
        assert_eq!(
            report.apps,
            vec![AppUsage {
                titles: vec![title("Downloads", 4)],
                ..usage("explorer.exe", 4)
            }]
        );
        assert_eq!(report.idle_secs, 5);
        assert_eq!(report.unknown_secs, 0);
        assert_eq!(report.tracked_secs, 9);
    }

    #[test]
    fn recognizes_shell_processes() {
        for name in [
            "ShellExperienceHost.exe",
            "StartMenuExperienceHost.exe",
            "ShellHost.exe",
            "SearchHost.exe",
            "MiniSearchHost.exe",
            "SearchApp.exe",
            "SearchUI.exe",
            "TextInputHost.exe",
            "WindowsInternal.ComposableShell.Experiences.TextInput.InputApp.exe",
            "ctfmon.exe",
            "LockApp.exe",
            "LogonUI.exe",
            "Widgets.exe",
            "WidgetBoard.exe",
            "PeopleExperienceHost.exe",
            "ScreenClippingHost.exe",
            "ApplicationFrameHost.exe",
            "sihost.exe",
            "dwm.exe",
            "csrss.exe",
            "consent.exe",
            "CredentialUIBroker.exe",
            "shellexperiencehost",
            "  SEARCHHOST.EXE ",
            "C:\\Windows\\SystemApps\\ShellExperienceHost_cw5n1h2txyewy\\ShellExperienceHost.exe",
        ] {
            assert!(is_ignored_process(name), "{name}");
        }
        for name in [
            "explorer.exe",
            "Code.exe",
            "chrome.exe",
            "SystemSettings.exe",
            "Taskmgr.exe",
            "SnippingTool.exe",
            "WindowsTerminal.exe",
            "focusmode.exe",
            "ShellExperienceHostHelper.exe",
            "",
        ] {
            assert!(!is_ignored_process(name), "{name}");
        }
    }

    #[test]
    fn recognizes_shell_surface_classes() {
        for class in [
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
            "Progman",
            "WorkerW",
            "NotifyIconOverflowWindow",
            "TopLevelWindowForOverflowXamlIsland",
            "TaskListThumbnailWnd",
            "MultitaskingViewFrame",
            "XamlExplorerHostIslandWindow",
            "ForegroundStaging",
            "Shell_LightDismissOverlay",
            "Shell_InputSwitchTopLevelWindow",
            "shell_traywnd",
            " PROGMAN ",
        ] {
            assert!(is_shell_surface_class(class), "{class}");
        }
        for class in [
            "CabinetWClass",
            "Chrome_WidgetWin_1",
            "Windows.UI.Core.CoreWindow",
            "ApplicationFrameWindow",
            "",
        ] {
            assert!(!is_shell_surface_class(class), "{class}");
        }
    }

    #[test]
    fn explorer_counts_only_for_folder_windows() {
        let explorer = Some("explorer.exe");
        assert!(!is_shell_surface(explorer, Some("CabinetWClass")));
        assert!(!is_shell_surface(explorer, None));
        assert!(is_shell_surface(explorer, Some("Shell_TrayWnd")));
        assert!(is_shell_surface(explorer, Some("Shell_SecondaryTrayWnd")));
        assert!(is_shell_surface(explorer, Some("Progman")));
        assert!(is_shell_surface(explorer, Some("WorkerW")));
        assert!(is_shell_surface(
            explorer,
            Some("TopLevelWindowForOverflowXamlIsland")
        ));
        assert!(is_shell_surface(
            Some("C:\\Windows\\Explorer.EXE"),
            Some("XamlExplorerHostIslandWindow")
        ));
        // CoreWindow: shell when explorer or an ignored host owns it, an app otherwise.
        assert!(is_shell_surface(
            explorer,
            Some("Windows.UI.Core.CoreWindow")
        ));
        assert!(is_shell_surface(
            Some("StartMenuExperienceHost.exe"),
            Some("Windows.UI.Core.CoreWindow")
        ));
        assert!(!is_shell_surface(
            Some("Calculator.exe"),
            Some("Windows.UI.Core.CoreWindow")
        ));
        assert!(!is_shell_surface(None, Some("Windows.UI.Core.CoreWindow")));
        // Ignored processes are ignored whatever the class; unknown processes only by class.
        assert!(is_shell_surface(Some("SearchHost.exe"), None));
        assert!(is_shell_surface(None, Some("Shell_TrayWnd")));
        assert!(!is_shell_surface(None, None));
        assert!(!is_shell_surface(
            Some("Code.exe"),
            Some("Chrome_WidgetWin_1")
        ));
    }

    #[test]
    fn debug_output_never_contains_titles() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(
            Some("chrome.exe"),
            Some("Secret project - Docs"),
            false,
            2000,
        );
        let debug = format!("{acc:?}");
        assert!(debug.contains("chrome.exe"), "{debug}");
        assert!(!debug.contains("Secret"), "{debug}");
    }

    #[test]
    fn old_history_without_titles_still_loads() {
        let app: AppUsage =
            serde_json::from_str(r#"{"process":"Code.exe","secs":12}"#).expect("old entry");
        assert_eq!(app, usage("Code.exe", 12));
    }

    #[test]
    fn summarize_merges_totals_and_usage() {
        let goal = SessionGoal {
            source: WorkSource::Github,
            key: "o/r#7".to_owned(),
            title: "Fix login".to_owned(),
            url: "https://github.com/o/r/issues/7".to_owned(),
        };
        let totals = SessionTotals {
            session_id: "abc".to_owned(),
            started_at_ms: 1_000,
            ended_at_ms: 9_000,
            focused_secs: 5,
            break_secs: 2,
            cycles_completed: 1,
            demo: true,
            goal: Some(goal.clone()),
            steps: vec![MicroStep {
                id: 2,
                text: "Napisz test".to_owned(),
                done: true,
            }],
            focus_spans: vec![TimeSpan {
                start_ms: 1_000,
                end_ms: 6_000,
            }],
        };
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("Code.exe"), None, false, 5000);
        let report = acc.report();
        let summary = summarize(&totals, report.clone());
        assert_eq!(
            summary,
            SessionSummary {
                session_id: "abc".to_owned(),
                started_at_ms: 1_000,
                ended_at_ms: 9_000,
                focused_secs: 5,
                break_secs: 2,
                cycles_completed: 1,
                usage: report,
                demo: true,
                goal: Some(goal),
                goal_done: None,
                steps: totals.steps.clone(),
                sparks: None,
                focus_spans: totals.focus_spans.clone(),
            }
        );
    }

    #[test]
    fn history_written_before_goals_still_loads() {
        let old: SessionSummary = serde_json::from_str(
            r#"{"sessionId":"s","startedAtMs":1,"endedAtMs":2,"focusedSecs":3,"breakSecs":4,
                "cyclesCompleted":1,"usage":{"apps":[],"idleSecs":0,"unknownSecs":0,"trackedSecs":0},
                "demo":false}"#,
        )
        .expect("old history entry");
        assert_eq!(old.goal, None);
        assert_eq!(old.goal_done, None);
        // Weekly stats (ADR-023): no spans → estimated from start and end.
        assert!(old.focus_spans.is_empty());
    }

    // --- Weekly stats (ADR-023) -------------------------------------------------------------------------------

    const CEST: i32 = 120;

    /// Unix ms of a UTC date and time.
    fn utc(year: i64, month: u32, day: u32, hour: i64, minute: i64) -> i64 {
        (days_from_civil(year, month, day) * 24 * 60 + hour * 60 + minute) * 60_000
    }

    fn date(text: &str) -> LocalDay {
        parse_local_date(text).expect("valid date")
    }

    fn query(day: &str, now_ms: i64) -> WeekQuery {
        WeekQuery {
            day: date(day),
            now_ms,
            revision: 7,
            history_off: false,
        }
    }

    /// A stored session whose focus is exactly `spans`.
    fn stored(id: &str, start_ms: i64, end_ms: i64, spans: &[(i64, i64)]) -> SessionSummary {
        let focus_ms = spans
            .iter()
            .map(|(start, end)| end.saturating_sub(*start))
            .fold(0_i64, i64::saturating_add);
        SessionSummary {
            session_id: id.to_owned(),
            started_at_ms: start_ms,
            ended_at_ms: end_ms,
            focused_secs: u32::try_from(focus_ms / 1000).unwrap_or(0),
            break_secs: 0,
            cycles_completed: 0,
            usage: UsageReport::default(),
            demo: false,
            goal: None,
            goal_done: None,
            steps: Vec::new(),
            sparks: None,
            focus_spans: spans
                .iter()
                .map(|&(start_ms, end_ms)| TimeSpan { start_ms, end_ms })
                .collect(),
        }
    }

    fn with_apps(mut session: SessionSummary, apps: &[(&str, u32)]) -> SessionSummary {
        session.usage.apps = apps.iter().map(|&(p, secs)| usage(p, secs)).collect();
        session
    }

    fn cell_sum(heatmap: &[Vec<u32>]) -> u32 {
        heatmap.iter().flatten().sum()
    }

    #[test]
    fn local_dates_parse_strictly() {
        assert_eq!(local_date_string(date("2026-10-13")), "2026-10-13");
        assert_eq!(local_date_string(date("2024-02-29")), "2024-02-29");
        assert_eq!(date("1970-01-01"), 0);
        assert_eq!(local_date_string(date("0001-01-01")), "0001-01-01");
        for bad in [
            "",
            "2026-2-03",
            "2026-02-3",
            "2026-02-30",
            "2026-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-10-00",
            "0000-01-01",
            "2026/10/13",
            " 2026-10-13",
            "2026-10-13 ",
            "+026-10-13",
            "2026-1a-13",
            "2026-10-13T00:00",
            "２０２６-10-13",
        ] {
            assert_eq!(parse_local_date(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn iso_weeks_start_on_monday() {
        assert_eq!(iso_week(date("2026-10-13")), (2026, 42));
        assert_eq!(
            local_date_string(week_monday(date("2026-10-18"))),
            "2026-10-12"
        );
        assert_eq!(
            local_date_string(week_monday(date("2026-10-12"))),
            "2026-10-12"
        );
        assert_eq!(weekday_of(date("2026-10-13")), Weekday::Tue);
        assert_eq!(weekday_of(date("2026-10-18")), Weekday::Sun);
        // Year boundaries: the Thursday decides.
        assert_eq!(iso_week(date("2027-01-01")), (2026, 53));
        assert_eq!(iso_week(date("2024-12-30")), (2025, 1));
        assert_eq!(iso_week(date("2021-01-03")), (2020, 53));
        assert_eq!(iso_week(date("2021-01-04")), (2021, 1));
        // Before 1970 too.
        assert_eq!(weekday_of(date("1969-12-31")), Weekday::Wed);
        assert_eq!(
            local_date_string(week_monday(date("1970-01-01"))),
            "1969-12-29"
        );
    }

    #[test]
    fn a_week_sums_days_hours_sessions_and_apps() {
        let categories = BTreeMap::from([
            ("code.exe".to_owned(), AppCategory::Productive),
            ("chrome.exe".to_owned(), AppCategory::Distracting),
            ("figma.exe".to_owned(), AppCategory::Neutral),
        ]);
        // Monday 09:30–11:00 local (CEST, UTC+2).
        let mut monday = with_apps(
            stored(
                "mon",
                utc(2026, 10, 12, 7, 30),
                utc(2026, 10, 12, 9, 10),
                &[(utc(2026, 10, 12, 7, 30), utc(2026, 10, 12, 9, 0))],
            ),
            &[("Code.exe", 3000), ("chrome.exe", 1200)],
        );
        monday.break_secs = 600;
        monday.cycles_completed = 1;
        // Wednesday 23:30 → Thursday 00:30 local: split at midnight, counted as a Wednesday session.
        let wednesday = with_apps(
            stored(
                "wed",
                utc(2026, 10, 14, 21, 30),
                utc(2026, 10, 14, 22, 30),
                &[(utc(2026, 10, 14, 21, 30), utc(2026, 10, 14, 22, 30))],
            ),
            &[("code.exe", 600), ("Slack.exe", 300)],
        );
        let mut demo = stored(
            "demo",
            utc(2026, 10, 13, 10, 0),
            utc(2026, 10, 13, 11, 0),
            &[(utc(2026, 10, 13, 10, 0), utc(2026, 10, 13, 11, 0))],
        );
        demo.demo = true;
        // The Sunday before (10:00–10:45 local): only in the recent heatmap.
        let sunday_before = with_apps(
            stored(
                "sun",
                utc(2026, 10, 11, 8, 0),
                utc(2026, 10, 11, 8, 45),
                &[(utc(2026, 10, 11, 8, 0), utc(2026, 10, 11, 8, 45))],
            ),
            &[("Figma.exe", 2000)],
        );

        let stats = weekly_stats(
            query("2026-10-15", utc(2026, 10, 13, 12, 0)),
            &categories,
            |_| CEST,
            [&monday, &wednesday, &demo, &sunday_before],
        );

        assert_eq!(stats.revision, 7);
        assert_eq!(stats.week_start, "2026-10-12");
        assert_eq!((stats.iso_year, stats.iso_week), (2026, 42));
        assert!(stats.current_week);
        assert!(!stats.history_off);
        assert_eq!(stats.first_week_start.as_deref(), Some("2026-10-05"));
        let days: Vec<(&str, Weekday, u32, u32, u32)> = stats
            .days
            .iter()
            .map(|d| {
                (
                    d.date.as_str(),
                    d.weekday,
                    d.focus_secs,
                    d.break_secs,
                    d.sessions,
                )
            })
            .collect();
        assert_eq!(
            days,
            vec![
                ("2026-10-12", Weekday::Mon, 5400, 600, 1),
                ("2026-10-13", Weekday::Tue, 0, 0, 0),
                ("2026-10-14", Weekday::Wed, 1800, 0, 1),
                ("2026-10-15", Weekday::Thu, 1800, 0, 0),
                ("2026-10-16", Weekday::Fri, 0, 0, 0),
                ("2026-10-17", Weekday::Sat, 0, 0, 0),
                ("2026-10-18", Weekday::Sun, 0, 0, 0),
            ]
        );
        assert_eq!(
            (
                stats.focus_secs,
                stats.break_secs,
                stats.breaks_taken,
                stats.sessions
            ),
            (9000, 600, 1, 2)
        );
        assert_eq!(stats.best_day, Some(Weekday::Mon));

        assert_eq!(stats.heatmap.len(), DAYS_PER_WEEK);
        assert!(stats.heatmap.iter().all(|row| row.len() == HOURS_PER_DAY));
        assert_eq!(stats.heatmap[0][9], 1800);
        assert_eq!(stats.heatmap[0][10], 3600);
        assert_eq!(stats.heatmap[2][23], 1800);
        assert_eq!(stats.heatmap[3][0], 1800);
        assert_eq!(cell_sum(&stats.heatmap), 9000);
        assert_eq!(stats.recent_weeks, RECENT_WEEKS);
        assert_eq!(stats.recent_heatmap[6][10], 2700);
        assert_eq!(cell_sum(&stats.recent_heatmap), 9000 + 2700);
        assert!(!stats.estimated);

        assert_eq!(
            stats.top_apps,
            vec![
                WeekApp {
                    process: "Code.exe".to_owned(),
                    secs: 3600,
                    category: Some(AppCategory::Productive),
                },
                WeekApp {
                    process: "chrome.exe".to_owned(),
                    secs: 1200,
                    category: Some(AppCategory::Distracting),
                },
                WeekApp {
                    process: "Slack.exe".to_owned(),
                    secs: 300,
                    category: None,
                },
            ]
        );
        assert_eq!(
            stats.categories,
            CategoryTotals {
                productive_secs: 3600,
                neutral_secs: 0,
                distracting_secs: 1200,
                unlabelled_secs: 300,
            }
        );

        // The same history seen from the next week: nothing in it, the recent heatmap still has everything.
        let next = weekly_stats(
            query("2026-10-19", utc(2026, 10, 13, 12, 0)),
            &categories,
            |_| CEST,
            [&monday, &wednesday, &demo, &sunday_before],
        );
        assert!(!next.current_week);
        assert_eq!(next.focus_secs, 0);
        assert_eq!(next.best_day, None);
        assert!(next.top_apps.is_empty());
        assert_eq!(next.categories, CategoryTotals::default());
        assert_eq!(cell_sum(&next.recent_heatmap), 9000 + 2700);
    }

    #[test]
    fn daylight_saving_changes_are_placed_by_the_offset_of_each_instant() {
        // Europe/Warsaw: CEST (UTC+2) until 2026-10-25 01:00 UTC, then CET (UTC+1).
        let switch = utc(2026, 10, 25, 1, 0);
        let warsaw = |ms: i64| if ms < switch { 120 } else { 60 };
        // 02:30 CEST → 02:30 CET: one real hour, all of it in the local hour 02 of that Sunday.
        let night = stored(
            "night",
            utc(2026, 10, 25, 0, 30),
            utc(2026, 10, 25, 1, 30),
            &[(utc(2026, 10, 25, 0, 30), utc(2026, 10, 25, 1, 30))],
        );
        // Monday 2026-09-07 09:00–10:00 CEST, seven weeks before.
        let summer = stored(
            "summer",
            utc(2026, 9, 7, 7, 0),
            utc(2026, 9, 7, 8, 0),
            &[(utc(2026, 9, 7, 7, 0), utc(2026, 9, 7, 8, 0))],
        );
        let stats = weekly_stats(
            query("2026-10-25", utc(2026, 10, 26, 9, 0)),
            &BTreeMap::new(),
            warsaw,
            [&night, &summer],
        );
        assert_eq!(stats.week_start, "2026-10-19");
        assert_eq!(stats.heatmap[6][2], 3600);
        assert_eq!(cell_sum(&stats.heatmap), 3600);
        assert_eq!(stats.days[6].focus_secs, 3600);
        assert_eq!(stats.days[6].sessions, 1);
        assert_eq!(
            stats.recent_heatmap[0][9], 3600,
            "summer time stays at 09:00"
        );
        assert_eq!(stats.heatmap[0][9], 0);
        assert_eq!(stats.first_week_start.as_deref(), Some("2026-09-07"));
        assert!(!stats.current_week);
    }

    #[test]
    fn sessions_without_spans_are_spread_evenly_and_marked_estimated() {
        // Tuesday 08:00–10:00 UTC with 1 h of focus: half an hour in each hour.
        let mut old = stored(
            "old",
            utc(2026, 10, 13, 8, 0),
            utc(2026, 10, 13, 10, 0),
            &[],
        );
        old.focused_secs = 3600;
        // More focus than the session lasted: capped at its length.
        let mut short = stored(
            "short",
            utc(2026, 10, 14, 8, 0),
            utc(2026, 10, 14, 8, 30),
            &[],
        );
        short.focused_secs = 3600;
        let stats = weekly_stats(
            query("2026-10-13", utc(2026, 10, 13, 12, 0)),
            &BTreeMap::new(),
            |_| 0,
            [&old, &short],
        );
        assert_eq!(stats.heatmap[1][8], 1800);
        assert_eq!(stats.heatmap[1][9], 1800);
        assert_eq!(stats.heatmap[2][8], 1800);
        assert_eq!(stats.focus_secs, 5400);
        assert!(stats.estimated);

        // An old session outside the recent weeks does not make this week estimated.
        let mut ancient = stored("ancient", utc(2025, 1, 7, 8, 0), utc(2025, 1, 7, 9, 0), &[]);
        ancient.focused_secs = 3600;
        let exact = stored(
            "exact",
            utc(2026, 10, 13, 8, 0),
            utc(2026, 10, 13, 9, 0),
            &[(utc(2026, 10, 13, 8, 0), utc(2026, 10, 13, 9, 0))],
        );
        let stats = weekly_stats(
            query("2026-10-13", utc(2026, 10, 13, 12, 0)),
            &BTreeMap::new(),
            |_| 0,
            [&ancient, &exact],
        );
        assert!(!stats.estimated);
        assert_eq!(stats.first_week_start.as_deref(), Some("2025-01-06"));
    }

    #[test]
    fn an_empty_history_gives_an_empty_week() {
        let stats = weekly_stats(
            WeekQuery {
                history_off: true,
                ..query("2026-10-13", utc(2026, 10, 13, 12, 0))
            },
            &BTreeMap::new(),
            |_| CEST,
            [],
        );
        assert!(stats.history_off);
        assert!(stats.current_week);
        assert_eq!(stats.days.len(), DAYS_PER_WEEK);
        assert!(
            stats
                .days
                .iter()
                .all(|d| d.focus_secs == 0 && d.sessions == 0)
        );
        assert_eq!(stats.best_day, None);
        assert_eq!(stats.first_week_start, None);
        assert_eq!(
            cell_sum(&stats.heatmap) + cell_sum(&stats.recent_heatmap),
            0
        );
        assert!(stats.top_apps.is_empty());
        assert!(!stats.estimated);
    }

    #[test]
    fn corrupt_spans_are_cut_to_the_window_and_never_panic() {
        let monday = utc(2026, 10, 12, 0, 0);
        let decade_ms = 10 * 365 * 24 * 3_600_000;
        let mut wild = stored(
            "wild",
            i64::MAX,
            i64::MIN,
            &[
                (i64::MIN, i64::MAX),
                (5, 1),
                (i64::MAX - 1, i64::MAX),
                (monday - decade_ms, monday + decade_ms),
            ],
        );
        wild.focused_secs = u32::MAX;
        let mut no_spans = stored("no-spans", i64::MIN, i64::MAX, &[]);
        no_spans.focused_secs = u32::MAX;
        let stats = weekly_stats(
            query("2026-10-12", 0),
            &BTreeMap::new(),
            |_| i32::MAX,
            [&wild, &no_spans],
        );
        // The ten-year span fills every hour of the window once (the offset is clamped, the window shifts with it).
        assert!(stats.heatmap.iter().flatten().all(|secs| *secs == 3600));
        assert!(
            stats
                .recent_heatmap
                .iter()
                .flatten()
                .all(|secs| *secs == 3600 * RECENT_WEEKS)
        );
        assert_eq!(stats.focus_secs, 7 * 24 * 3600);
        assert_eq!(stats.sessions, 0);
    }

    #[test]
    fn top_apps_are_capped_but_categories_count_every_app() {
        let apps: Vec<(String, u32)> = (1..=12)
            .map(|i| (format!("app{i:02}.exe"), i * 100))
            .collect();
        let apps: Vec<(&str, u32)> = apps.iter().map(|(p, s)| (p.as_str(), *s)).collect();
        let mut session = with_apps(
            stored("s", utc(2026, 10, 13, 8, 0), utc(2026, 10, 13, 9, 0), &[]),
            &apps,
        );
        // A tie with app12 is ordered by name.
        session.usage.apps.push(usage("aaa.exe", 1200));
        let categories = BTreeMap::from([("app01.exe".to_owned(), AppCategory::Neutral)]);
        let stats = weekly_stats(query("2026-10-13", 0), &categories, |_| 0, [&session]);
        assert_eq!(stats.top_apps.len(), TOP_APPS_PER_WEEK);
        let names: Vec<&str> = stats.top_apps.iter().map(|a| a.process.as_str()).collect();
        assert_eq!(&names[..3], ["aaa.exe", "app12.exe", "app11.exe"]);
        assert_eq!(stats.categories.neutral_secs, 100);
        assert_eq!(
            stats.categories.unlabelled_secs,
            (2..=12).map(|i| i * 100).sum::<u32>() + 1200
        );
    }

    #[test]
    fn the_best_day_is_the_first_one_on_a_tie() {
        let tuesday = stored(
            "tue",
            utc(2026, 10, 13, 8, 0),
            utc(2026, 10, 13, 9, 0),
            &[(utc(2026, 10, 13, 8, 0), utc(2026, 10, 13, 9, 0))],
        );
        let thursday = stored(
            "thu",
            utc(2026, 10, 15, 8, 0),
            utc(2026, 10, 15, 9, 0),
            &[(utc(2026, 10, 15, 8, 0), utc(2026, 10, 15, 9, 0))],
        );
        let stats = weekly_stats(
            query("2026-10-13", 0),
            &BTreeMap::new(),
            |_| 0,
            [&thursday, &tuesday],
        );
        assert_eq!(stats.best_day, Some(Weekday::Tue));
    }

    #[test]
    fn a_session_across_sunday_midnight_is_split_between_the_weeks() {
        // Sunday 2026-10-18 23:30 → Monday 2026-10-19 00:30 local (CEST): counted as a session of the week it started in.
        let night = with_apps(
            stored(
                "night",
                utc(2026, 10, 18, 21, 30),
                utc(2026, 10, 18, 22, 30),
                &[(utc(2026, 10, 18, 21, 30), utc(2026, 10, 18, 22, 30))],
            ),
            &[("Code.exe", 3600)],
        );
        let now = utc(2026, 10, 20, 10, 0);
        let first = weekly_stats(
            query("2026-10-12", now),
            &BTreeMap::new(),
            |_| CEST,
            [&night],
        );
        assert_eq!(first.days[6].focus_secs, 1800);
        assert_eq!(first.heatmap[6][23], 1800);
        assert_eq!((first.sessions, first.focus_secs), (1, 1800));
        assert_eq!(first.top_apps.len(), 1);
        assert!(!first.current_week);

        let second = weekly_stats(
            query("2026-10-19", now),
            &BTreeMap::new(),
            |_| CEST,
            [&night],
        );
        assert!(second.current_week);
        assert_eq!(second.days[0].focus_secs, 1800);
        assert_eq!(second.heatmap[0][0], 1800);
        // The session (and its apps) belong to the week it started in.
        assert_eq!((second.sessions, second.focus_secs), (0, 1800));
        assert!(second.top_apps.is_empty());
        // The recent heatmap of the later week holds both halves.
        assert_eq!(cell_sum(&second.recent_heatmap), 3600);
        assert_eq!(second.first_week_start.as_deref(), Some("2026-10-12"));
    }

    #[test]
    fn west_of_utc_the_local_day_comes_from_the_offset() {
        // New York in October (EDT, UTC−4): Tuesday 01:00 UTC is Monday 21:00 local.
        let late = stored(
            "late",
            utc(2026, 10, 13, 1, 0),
            utc(2026, 10, 13, 2, 0),
            &[(utc(2026, 10, 13, 1, 0), utc(2026, 10, 13, 2, 0))],
        );
        let stats = weekly_stats(
            query("2026-10-12", utc(2026, 10, 13, 12, 0)),
            &BTreeMap::new(),
            |_| -240,
            [&late],
        );
        assert_eq!(stats.heatmap[0][21], 3600);
        assert_eq!(stats.days[0].sessions, 1);
        assert_eq!(stats.best_day, Some(Weekday::Mon));
    }

    #[test]
    fn a_daylight_saving_change_on_the_half_hour_is_placed_exactly() {
        // St. John's: NST (UTC−3:30) until 2026-03-08 05:30 UTC (02:00 local), then NDT (UTC−2:30).
        let switch = utc(2026, 3, 8, 5, 30);
        let st_johns = |ms: i64| if ms < switch { -210 } else { -150 };
        // 01:30 NST → 04:00 NDT: half an hour in the local hour 01, the hour 02 is skipped, a full hour in 03.
        let night = stored(
            "night",
            utc(2026, 3, 8, 5, 0),
            utc(2026, 3, 8, 6, 30),
            &[(utc(2026, 3, 8, 5, 0), utc(2026, 3, 8, 6, 30))],
        );
        let stats = weekly_stats(
            query("2026-03-08", utc(2026, 3, 9, 12, 0)),
            &BTreeMap::new(),
            st_johns,
            [&night],
        );
        assert_eq!(stats.week_start, "2026-03-02");
        assert_eq!(stats.heatmap[6][1], 1800);
        assert_eq!(stats.heatmap[6][2], 0);
        assert_eq!(stats.heatmap[6][3], 3600);
        assert_eq!(stats.days[6].focus_secs, 5400);
    }
}
