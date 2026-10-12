//! Foreground app tracker (std thread, 1 s samples).
//!
//! - While focus runs (`Tracking(true)`) and `track_apps` is on, each sample adds the elapsed time to
//!   the foreground process (or idle / unknown) in a shared `UsageAccumulator`.
//! - While the sites shield is up and the browser extension is not connected, the foreground
//!   browser window title is matched against the blocklist → `focus://nudge`, the main window is
//!   brought to front and a "Wracaj do pracy" toast is shown (at most once per 30 s per domain).

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{Nudge, UsageReport, events};
use focus_core::blocklist::{is_browser_process, title_matches};
use focus_core::stats::UsageAccumulator;
use focus_core::timing::TimingProfile;
use focus_platform::foreground::ForegroundInfo;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::bridge::SharedBridge;
use crate::{SharedConfig, lock, read_config, show_main_window, texts};

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

struct Tracker {
    ctx: TrackerContext,
    probe: Box<dyn focus_platform::foreground::ForegroundProbe>,
    usage: Arc<Mutex<UsageAccumulator>>,
    enabled: bool,
    idle_after_secs: u32,
    /// Time of the previous counted sample while tracking.
    last_sample: Option<Instant>,
    /// What the previous sample saw: `(process, idle)`.
    last_seen: Option<(Option<String>, bool)>,
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
            // Attribute the partial second before the pause/break to what was last seen.
            if let (Some(prev), Some((process, idle))) = (self.last_sample, self.last_seen.clone())
            {
                let ms = elapsed_ms(prev);
                lock(&self.usage).add_sample(process.as_deref(), idle, ms);
            }
            self.last_sample = None;
        } else {
            self.last_sample = Some(Instant::now());
        }
        self.enabled = on;
    }

    fn tick(&mut self) {
        let track = self.enabled && read_config(&self.ctx.config).track_apps;
        let (watch_sites, blocklist) = {
            let bridge = lock(&self.ctx.bridge);
            let watch = bridge.block_active && !bridge.extension_connected();
            (
                watch,
                if watch {
                    bridge.blocklist.clone()
                } else {
                    Vec::new()
                },
            )
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
            let seen = match &sample {
                Ok(info) => (info.process.clone(), info.idle_secs >= self.idle_after_secs),
                Err(_) => (None, false),
            };
            lock(&self.usage).add_sample(seen.0.as_deref(), seen.1, ms);
            self.last_seen = Some(seen);
        }

        match sample {
            Ok(info) => {
                if self.probe_failing {
                    log::info!("foreground probe recovered");
                    self.probe_failing = false;
                }
                if watch_sites {
                    self.check_sites(&info, &blocklist);
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

    fn check_sites(&mut self, info: &ForegroundInfo, blocklist: &[String]) {
        let Some(process) = info.process.as_deref() else {
            return;
        };
        if !is_browser_process(process) {
            return;
        }
        let Some(domain) = title_matches(&info.title, blocklist) else {
            return;
        };
        let due = self
            .last_nudge
            .get(&domain)
            .is_none_or(|at| at.elapsed() >= NUDGE_EVERY);
        if !due {
            return;
        }
        self.last_nudge.insert(domain.clone(), Instant::now());
        // Debug only: browsing activity does not belong in the default (info) log file.
        log::debug!("blocked site in the foreground: {domain} ({process})");

        let nudge = Nudge {
            process: process.to_owned(),
            matched: domain.clone(),
            title: info.title.clone(),
        };
        if let Err(err) = self.ctx.app.emit(events::FOCUS_NUDGE, nudge) {
            log::warn!("cannot emit nudge: {err}");
        }
        show_main_window(&self.ctx.app);
        let toast = texts::nudge_toast(&domain);
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
