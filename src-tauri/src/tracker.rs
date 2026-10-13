//! Foreground app tracker (std thread, 1 s samples).
//!
//! - While focus runs (`Tracking(true)`) and `track_apps` is on, each sample adds the elapsed time to
//!   the foreground process (or idle / unknown) in a shared `UsageAccumulator`. Windows shell surfaces
//!   (taskbar, desktop, Start, search, Alt+Tab, ...) are not counted at all (`stats::is_shell_surface`).
//!   With `track_titles` on, the window title goes with the sample, so the chart can show each app's
//!   most-used titles; it stays in memory and in the local history file only, never in the log.
//! - While the sites shield is up and the browser extension is not connected, the foreground
//!   browser window title is matched against `websiteblacklist.txt` (domains, then regexes) →
//!   `focus://nudge`, the main window is brought to front and a "Wracaj do pracy" toast is shown
//!   (at most once per 30 s per matched entry). Titles are never logged.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use focus_core::api::{Nudge, UsageReport, events};
use focus_core::blocklist::is_browser_process;
use focus_core::stats::{UsageAccumulator, is_shell_surface};
use focus_core::timing::TimingProfile;
use focus_platform::PlatformError;
use focus_platform::foreground::ForegroundInfo;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::blocklist::SharedBlocklist;
use crate::bridge::SharedBridge;
use crate::{SharedConfig, lock, show_main_window, texts};

const SAMPLE_EVERY: Duration = Duration::from_millis(1000);
/// A longer gap between samples (thread starved, machine suspended) is not counted in full.
const MAX_SAMPLE_MS: u64 = 2000;
const NUDGE_EVERY: Duration = Duration::from_secs(30);

pub enum TrackerCmd {
    Enable(bool),
    /// New session: clear usage and use the profile's idle threshold.
    Reset {
        idle_after_secs: u32,
    },
    /// Reply with the usage once every earlier command (e.g. the final `Enable(false)`) is applied.
    Report(Sender<UsageReport>),
}

#[derive(Clone)]
pub struct TrackerHandle {
    tx: Sender<TrackerCmd>,
    usage: Arc<Mutex<UsageAccumulator>>,
}

impl TrackerHandle {
    pub fn set_enabled(&self, on: bool) {
        self.send(TrackerCmd::Enable(on));
    }

    pub fn reset(&self, idle_after_secs: u32) {
        self.send(TrackerCmd::Reset { idle_after_secs });
    }

    /// Usage collected since the last reset.
    pub fn report(&self) -> UsageReport {
        lock(&self.usage).report()
    }

    /// Usage after the tracker thread applied every command sent before this call, so the partial
    /// second counted by the final `Tracking(false)` is included. Waits at most `timeout` (the
    /// thread only samples the foreground window between commands), then falls back to [`Self::report`].
    pub fn settled_report(&self, timeout: Duration) -> UsageReport {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self.tx.send(TrackerCmd::Report(reply_tx)).is_ok() {
            match reply_rx.recv_timeout(timeout) {
                Ok(report) => return report,
                Err(_) => log::warn!("tracker did not report within {timeout:?}"),
            }
        }
        self.report()
    }

    fn send(&self, cmd: TrackerCmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("tracker is not running");
        }
    }
}

pub struct TrackerContext {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub bridge: SharedBridge,
    /// The parsed `websiteblacklist.txt` (window-title fallback).
    pub blocklist: SharedBlocklist,
}

pub fn spawn(ctx: TrackerContext) -> anyhow::Result<TrackerHandle> {
    let (tx, rx) = mpsc::channel();
    let usage = Arc::new(Mutex::new(UsageAccumulator::new()));
    let handle = TrackerHandle {
        tx,
        usage: Arc::clone(&usage),
    };
    std::thread::Builder::new()
        .name("focus-tracker".into())
        .spawn(move || {
            let mut tracker = Tracker {
                probe: focus_platform::foreground::probe(),
                ctx,
                usage,
                enabled: false,
                idle_after_secs: TimingProfile::normal().idle_after_secs,
                last_sample: None,
                last_seen: None,
                last_nudge: HashMap::new(),
                probe_failing: false,
            };
            tracker.run(rx);
        })?;
    Ok(handle)
}

/// What a tracking sample saw. No `Debug`: it may hold a window title, which is never logged.
#[derive(Clone, PartialEq, Eq)]
enum Seen {
    /// Counted: the process (`None` = unknown), the window title (only with `track_titles`) and idle.
    Counted {
        process: Option<String>,
        title: Option<String>,
        idle: bool,
    },
    /// A Windows shell surface (taskbar, desktop, Start, search, ...): not counted at all.
    ShellSurface,
}

impl Seen {
    /// Classifies a probe result. Idle time counts as idle whatever is in the foreground (the user is
    /// away); otherwise a shell surface is skipped. `titles`: keep the window title (`track_titles`).
    fn of(
        sample: &Result<ForegroundInfo, PlatformError>,
        idle_after_secs: u32,
        titles: bool,
    ) -> Self {
        let Ok(info) = sample else {
            return Self::Counted {
                process: None,
                title: None,
                idle: false,
            };
        };
        let idle = info.idle_secs >= idle_after_secs;
        if !idle && is_shell_surface(info.process.as_deref(), window_class(info)) {
            return Self::ShellSurface;
        }
        Self::Counted {
            process: info.process.clone(),
            title: (titles && !idle).then(|| info.title.clone()),
            idle,
        }
    }

    /// Adds `ms` of this sample to `usage` (nothing for a shell surface); the title only when `titles`
    /// (`track_titles` may have been switched off since the sample).
    fn add_to(&self, usage: &mut UsageAccumulator, ms: u64, titles: bool) {
        if let Self::Counted {
            process,
            title,
            idle,
        } = self
        {
            let title = title.as_deref().filter(|_| titles);
            usage.add_sample(process.as_deref(), title, *idle, ms);
        }
    }
}

/// `report` without window titles (used when `track_titles` is off, so titles collected before it was
/// switched off are neither shown nor saved).
pub fn without_titles(mut report: UsageReport) -> UsageReport {
    for app in &mut report.apps {
        app.titles.clear();
    }
    report
}

/// Class name of the foreground window, if the probe knows it.
fn window_class(info: &ForegroundInfo) -> Option<&str> {
    Some(info.class_name.as_str()).filter(|class| !class.is_empty())
}

struct Tracker {
    ctx: TrackerContext,
    probe: Box<dyn focus_platform::foreground::ForegroundProbe>,
    usage: Arc<Mutex<UsageAccumulator>>,
    enabled: bool,
    idle_after_secs: u32,
    /// Time of the previous counted sample while tracking.
    last_sample: Option<Instant>,
    /// What the previous sample saw.
    last_seen: Option<Seen>,
    last_nudge: HashMap<String, Instant>,
    probe_failing: bool,
}

impl Tracker {
    fn run(&mut self, rx: Receiver<TrackerCmd>) {
        let mut next = Instant::now() + SAMPLE_EVERY;
        loop {
            match rx.recv_timeout(next.saturating_duration_since(Instant::now())) {
                Ok(TrackerCmd::Enable(on)) => {
                    self.set_enabled(on);
                    continue;
                }
                Ok(TrackerCmd::Reset { idle_after_secs }) => {
                    self.idle_after_secs = idle_after_secs;
                    lock(&self.usage).clear();
                    self.last_seen = None;
                    self.last_sample = self.enabled.then(Instant::now);
                    self.last_nudge.clear();
                    continue;
                }
                Ok(TrackerCmd::Report(reply)) => {
                    let report = lock(&self.usage).report();
                    // The caller may have given up waiting; nothing to do then.
                    let _ = reply.send(report);
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            next = Instant::now() + SAMPLE_EVERY;
            self.tick();
        }
    }

    fn set_enabled(&mut self, on: bool) {
        if on == self.enabled {
            return;
        }
        if !on {
            // Attribute the partial second before the pause/break to what was last seen (nothing when
            // that was a shell surface).
            if let (Some(prev), Some(seen)) = (self.last_sample, self.last_seen.as_ref()) {
                let titles = self.tracking_switches().1;
                seen.add_to(&mut lock(&self.usage), elapsed_ms(prev), titles);
            }
            self.last_sample = None;
        } else {
            self.last_sample = Some(Instant::now());
        }
        self.enabled = on;
    }

    /// `(track_apps, track_titles)` from the current config (read without cloning it every second).
    fn tracking_switches(&self) -> (bool, bool) {
        let config = self
            .ctx
            .config
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        (config.track_apps, config.track_titles)
    }

    fn tick(&mut self) {
        let (track_apps, track_titles) = self.tracking_switches();
        let track = self.enabled && track_apps;
        let watch_sites = {
            let bridge = lock(&self.ctx.bridge);
            bridge.block_active && !bridge.extension_connected()
        };
        if !track {
            // Keep the clock fresh so re-enabling `track_apps` does not count the gap.
            if self.enabled {
                self.last_sample = Some(Instant::now());
            }
            if !watch_sites {
                return;
            }
        }

        let now = Instant::now();
        let sample = self.probe.sample();
        if track {
            let ms = self.last_sample.map_or(0, elapsed_ms);
            self.last_sample = Some(now);
            let seen = Seen::of(&sample, self.idle_after_secs, track_titles);
            seen.add_to(&mut lock(&self.usage), ms, track_titles);
            self.last_seen = Some(seen);
        }

        match sample {
            Ok(info) => {
                if self.probe_failing {
                    log::info!("foreground probe recovered");
                    self.probe_failing = false;
                }
                if watch_sites {
                    self.check_sites(&info);
                }
            }
            Err(err) => {
                if !self.probe_failing {
                    log::warn!("foreground probe failed: {err}");
                    self.probe_failing = true;
                }
            }
        }
    }

    fn check_sites(&mut self, info: &ForegroundInfo) {
        let Some(process) = info.process.as_deref() else {
            return;
        };
        if !is_browser_process(process) {
            return;
        }
        // `(entry, is_pattern)`: a domain, else the source of the first matching regex.
        let matched = {
            let list = self
                .ctx
                .blocklist
                .read()
                .unwrap_or_else(PoisonError::into_inner);
            list.match_title(&info.title).map(|entry| {
                let is_pattern = !list.domains().contains(&entry);
                (entry, is_pattern)
            })
        };
        let Some((entry, is_pattern)) = matched else {
            return;
        };
        let due = self
            .last_nudge
            .get(&entry)
            .is_none_or(|at| at.elapsed() >= NUDGE_EVERY);
        if !due {
            return;
        }
        self.last_nudge.insert(entry.clone(), Instant::now());
        // No title, URL or entry in the log (privacy rule 11): browsing activity stays private.
        log::debug!("blocked site in the foreground ({process})");

        let toast = texts::nudge_toast(&entry, is_pattern);
        let nudge = Nudge {
            process: process.to_owned(),
            matched: entry,
            title: info.title.clone(),
        };
        if let Err(err) = self.ctx.app.emit(events::FOCUS_NUDGE, nudge) {
            log::warn!("cannot emit nudge: {err}");
        }
        show_main_window(&self.ctx.app);
        if let Err(err) = self
            .ctx
            .app
            .notification()
            .builder()
            .title(toast.title)
            .body(toast.body)
            .show()
        {
            log::warn!("cannot show nudge toast: {err}");
        }
    }
}

fn elapsed_ms(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .min(MAX_SAMPLE_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE_AFTER: u32 = 120;

    fn info(process: &str, class: &str, title: &str, idle_secs: u32) -> ForegroundInfo {
        ForegroundInfo {
            process: Some(process.to_owned()),
            title: title.to_owned(),
            class_name: class.to_owned(),
            idle_secs,
        }
    }

    fn report_of(samples: &[(Seen, u64)], titles: bool) -> UsageReport {
        let mut usage = UsageAccumulator::new();
        for (seen, ms) in samples {
            seen.add_to(&mut usage, *ms, titles);
        }
        usage.report()
    }

    #[test]
    fn shell_surfaces_are_not_counted() {
        for (process, class) in [
            ("explorer.exe", "Shell_TrayWnd"),
            ("explorer.exe", "Progman"),
            ("explorer.exe", "WorkerW"),
            ("ShellExperienceHost.exe", "Windows.UI.Core.CoreWindow"),
            ("SearchHost.exe", "Windows.UI.Core.CoreWindow"),
            ("StartMenuExperienceHost.exe", ""),
        ] {
            let seen = Seen::of(&Ok(info(process, class, "x", 0)), IDLE_AFTER, true);
            assert!(seen == Seen::ShellSurface, "{process} / {class}");
        }
        // A File Explorer window counts, with its title.
        let folder = Seen::of(
            &Ok(info("explorer.exe", "CabinetWClass", "Pobrane", 0)),
            IDLE_AFTER,
            true,
        );
        let report = report_of(&[(folder, 5000)], true);
        assert_eq!(report.apps.len(), 1);
        assert_eq!(report.apps[0].process, "explorer.exe");
        assert_eq!(report.apps[0].titles[0].title, "Pobrane");
        // Nothing at all for the taskbar, not even unknown time.
        let taskbar = Seen::of(
            &Ok(info("explorer.exe", "Shell_TrayWnd", "", 0)),
            IDLE_AFTER,
            true,
        );
        assert_eq!(report_of(&[(taskbar, 5000)], true).tracked_secs, 0);
    }

    #[test]
    fn idle_time_counts_as_idle_even_on_the_desktop() {
        let seen = Seen::of(
            &Ok(info("explorer.exe", "Progman", "", IDLE_AFTER)),
            IDLE_AFTER,
            true,
        );
        let report = report_of(&[(seen, 3000)], true);
        assert_eq!(report.idle_secs, 3);
        assert!(report.apps.is_empty());
    }

    #[test]
    fn titles_only_with_track_titles() {
        let sample = Ok(info("chrome.exe", "Chrome_WidgetWin_1", "Docs - Chrome", 0));
        let with = Seen::of(&sample, IDLE_AFTER, true);
        let without = Seen::of(&sample, IDLE_AFTER, false);
        assert!(
            report_of(&[(without, 4000)], true).apps[0]
                .titles
                .is_empty()
        );
        // Switched off after the sample: the title is dropped when it is added.
        assert!(
            report_of(&[(with.clone(), 4000)], false).apps[0]
                .titles
                .is_empty()
        );
        let report = report_of(&[(with, 4000)], true);
        assert_eq!(report.apps[0].titles[0].title, "Docs - Chrome");
        assert!(without_titles(report).apps[0].titles.is_empty());
    }

    #[test]
    fn probe_failures_count_as_unknown() {
        let seen = Seen::of(&Err(PlatformError::Os("x".into())), IDLE_AFTER, true);
        assert_eq!(report_of(&[(seen, 2000)], true).unknown_secs, 2);
    }
}
