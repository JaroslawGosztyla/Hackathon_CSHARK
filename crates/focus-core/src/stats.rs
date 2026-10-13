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

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt;

use crate::api::{AppUsage, SessionSummary, TitleUsage, UsageReport};
use crate::session::SessionTotals;

const MS_PER_SEC: u64 = 1000;
/// Distinct window titles remembered per app; when full, the least-used title makes room.
pub const MAX_TITLES_PER_APP: usize = 50;
/// Titles are trimmed and shortened to this many characters (an ellipsis marks the cut).
pub const MAX_TITLE_CHARS: usize = 120;
/// Titles per app in [`UsageReport`] (most-used first).
pub const REPORTED_TITLES_PER_APP: usize = 5;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let totals = SessionTotals {
            session_id: "abc".to_owned(),
            started_at_ms: 1_000,
            ended_at_ms: 9_000,
            focused_secs: 5,
            break_secs: 2,
            cycles_completed: 1,
            demo: true,
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
            }
        );
    }
}
