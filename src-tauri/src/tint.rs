//! Warm screen tint (ADR-014): one transparent, click-through, always-on-top overlay window
//! per monitor (ADR-009: an overlay, never the gamma ramp, so nothing can stay tinted after a crash).
//!
//! A dedicated thread owns the overlays. Every [`SCHEDULE_TICK`], on a config change and on a settings
//! preview it computes the level (`focus_core::tint::tint_level` for the local time of day, or the preview
//! strength) and then:
//! * level > 0: makes sure there is one overlay per monitor (labels `tint-<generation>-<n>`, route `/tint`)
//!   covering the monitor's physical bounds; a changed monitor set or geometry rebuilds them;
//! * level 0: destroys them.
//!
//! The overlays read the level from `get_tint_status` / `tint://changed` and paint it themselves. They are
//! created visible but unfocused (shown without activation), never focusable, and made click-through
//! (`set_ignore_cursor_events`) right after creation; an overlay that cannot be made click-through is
//! destroyed at once. Failures go to `TintStatus::error` (+ one `app://error`) and are retried next tick.
//!
//! Verified on Windows 11 (tao 0.37 / wry 0.57, the versions Tauri 2.12 uses): a window made like this
//! stays translucent after `set_ignore_cursor_events` turns it into a layered window, clicks reach the
//! window below, and creating it does not change the foreground window.
// TODO(hackathon): the overlays are listed in Alt+Tab. tao gives every unowned window
// `WS_EX_APPWINDOW` and `skip_taskbar` only removes the taskbar button. Owning them by the main window
// would hide them whenever that window is minimized; the fix is `WS_EX_TOOLWINDOW` (a `focus-platform`
// helper applied after tao's last style change) or a hidden owner window that is never minimized.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, TintStatus, events};
use focus_core::tint::{MAX_STRENGTH, tint_level};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::window_util::{to_u32, with_main_webview_options};
use crate::{SharedConfig, emit_error, lock, read_config, restore_only_launch, texts};

/// Label prefix of the overlay windows (`capabilities/overlays.json` lists `tint-*`).
pub const TINT_WINDOW_PREFIX: &str = "tint-";
/// SvelteKit route rendered by the overlays.
const ROUTE: &str = "tint";
/// How often the schedule is re-evaluated while no overlay is shown.
const SCHEDULE_TICK: Duration = Duration::from_secs(30);
/// While overlays are shown, the monitor layout is checked this often (a new monitor or another
/// resolution rebuilds them).
const ACTIVE_TICK: Duration = Duration::from_secs(5);
/// A preview shows its strength this long after the last `preview_tint` call.
const PREVIEW_FOR: Duration = Duration::from_secs(6);
const SECS_PER_DAY: u64 = 86_400;

enum TintCmd {
    /// Config changed (or anything else that warrants a re-evaluation now).
    Refresh,
    /// `Some(strength)` starts or extends a preview; `None` ends it.
    Preview(Option<u32>),
}

/// Cheap to clone; every method is non-blocking.
#[derive(Clone)]
pub struct TintHandle {
    tx: Sender<TintCmd>,
    status: Arc<Mutex<TintStatus>>,
}

impl TintHandle {
    pub fn spawn(app: AppHandle, config: SharedConfig) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(TintStatus {
            revision: 0,
            enabled: read_config(&config).tint.enabled,
            level: 0,
            preview: false,
            active: false,
            error: None,
        }));
        let shared = Arc::clone(&status);
        std::thread::Builder::new()
            .name("focus-tint".into())
            .spawn(move || {
                Worker {
                    app,
                    config,
                    status: shared,
                    preview: None,
                    overlays: Vec::new(),
                    generation: 0,
                    error: None,
                    reported_error: None,
                    last_published: None,
                }
                .run(rx);
            })?;
        Ok(Self { tx, status })
    }

    pub fn status(&self) -> TintStatus {
        lock(&self.status).clone()
    }

    pub fn config_changed(&self) {
        self.send(TintCmd::Refresh);
    }

    /// Settings preview: show `strength` (clamped to 0..=100) for a few seconds; `None` ends it.
    pub fn preview(&self, strength: Option<u32>) {
        self.send(TintCmd::Preview(
            strength.map(|strength| strength.min(MAX_STRENGTH)),
        ));
    }

    fn send(&self, cmd: TintCmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("tint thread is not running");
        }
    }
}

/// Physical bounds of a monitor (and its scale, which sets the overlays' logical size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    /// Scale factor × 1000 (exact comparison of layouts).
    scale_permille: u32,
}

impl Bounds {
    fn of(monitor: &Monitor) -> Option<Self> {
        let position = monitor.position();
        let size = monitor.size();
        let scale = monitor.scale_factor();
        if size.width == 0 || size.height == 0 || !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        Some(Self {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            scale_permille: to_u32((scale * 1000.0).round()).max(1),
        })
    }

    fn scale(self) -> f64 {
        f64::from(self.scale_permille) / 1000.0
    }
}

struct Overlay {
    label: String,
    bounds: Bounds,
}

struct Worker {
    app: AppHandle,
    config: SharedConfig,
    status: Arc<Mutex<TintStatus>>,
    /// Preview strength and when it ends.
    preview: Option<(u32, Instant)>,
    overlays: Vec<Overlay>,
    /// Bumped for every rebuild, so new labels never collide with windows still being destroyed.
    generation: u64,
    error: Option<String>,
    /// Last error sent as `app://error` (each distinct error is reported once).
    reported_error: Option<String>,
    last_published: Option<TintStatus>,
}

impl Worker {
    fn run(mut self, rx: Receiver<TintCmd>) {
        self.apply();
        loop {
            match rx.recv_timeout(self.next_wait()) {
                Ok(cmd) => {
                    self.handle(cmd);
                    // A dragged strength slider sends many previews: take them all, apply once.
                    while let Ok(cmd) = rx.try_recv() {
                        self.handle(cmd);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.destroy_overlays();
                    return;
                }
            }
            self.apply();
        }
    }

    fn handle(&mut self, cmd: TintCmd) {
        match cmd {
            TintCmd::Refresh => {}
            TintCmd::Preview(Some(strength)) => {
                self.preview = Some((strength, Instant::now() + PREVIEW_FOR));
            }
            TintCmd::Preview(None) => self.preview = None,
        }
    }

    fn next_wait(&self) -> Duration {
        let tick = if self.overlays.is_empty() {
            SCHEDULE_TICK
        } else {
            ACTIVE_TICK
        };
        match self.preview {
            Some((_, until)) => tick.min(until.saturating_duration_since(Instant::now())),
            None => tick,
        }
    }

    /// Re-evaluates the level and brings the overlays in line with it.
    fn apply(&mut self) {
        let tint = read_config(&self.config).tint;
        if self
            .preview
            .is_some_and(|(_, until)| Instant::now() >= until)
        {
            self.preview = None;
        }
        let preview = self.preview.map(|(strength, _)| strength);
        let scheduled = if tint.enabled {
            tint_level(&tint, local_minutes_of_day())
        } else {
            0
        };
        // A `--restore-shields` launch only repairs notifications and exits: no overlays.
        let level = if restore_only_launch() {
            0
        } else {
            preview.unwrap_or(scheduled)
        };
        if level > 0 {
            self.ensure_overlays();
        } else {
            self.destroy_overlays();
            self.error = None;
        }
        self.publish(tint.enabled, level, preview.is_some());
    }

    /// One overlay per monitor, matching the current layout; rebuilt when it changed.
    fn ensure_overlays(&mut self) {
        let wanted: Vec<Bounds> = match self.app.available_monitors() {
            Ok(monitors) => monitors.iter().filter_map(Bounds::of).collect(),
            Err(err) => {
                self.fail(format!("cannot list monitors: {err}"));
                return;
            }
        };
        if wanted.is_empty() {
            self.fail("no monitor found".into());
            return;
        }
        let intact = self.overlays.len() == wanted.len()
            && self
                .overlays
                .iter()
                .zip(&wanted)
                .all(|(overlay, bounds)| overlay.bounds == *bounds)
            && self
                .overlays
                .iter()
                .all(|overlay| self.app.get_webview_window(&overlay.label).is_some());
        if intact {
            return;
        }

        self.destroy_overlays();
        self.generation = self.generation.wrapping_add(1);
        for (index, bounds) in wanted.into_iter().enumerate() {
            let label = format!("{TINT_WINDOW_PREFIX}{}-{index}", self.generation);
            match self.build(&label, bounds) {
                Ok(()) => self.overlays.push(Overlay { label, bounds }),
                Err(message) => {
                    self.destroy_overlays();
                    self.fail(message);
                    return;
                }
            }
        }
        log::info!("screen tint: {} overlay(s) shown", self.overlays.len());
        self.error = None;
    }

    fn build(&self, label: &str, bounds: Bounds) -> Result<(), String> {
        let scale = bounds.scale();
        let builder = WebviewWindowBuilder::new(&self.app, label, WebviewUrl::App(ROUTE.into()))
            .title(texts::TINT_WINDOW_TITLE)
            .position(f64::from(bounds.x) / scale, f64::from(bounds.y) / scale)
            .inner_size(
                f64::from(bounds.width) / scale,
                f64::from(bounds.height) / scale,
            )
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .closable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            // Created visible and unfocused, it appears without activation (`SW_SHOWNOACTIVATE`); not
            // focusable, it is never activated later either (e.g. when the window under it closes).
            .focused(false)
            .focusable(false)
            .visible(true);
        let window = with_main_webview_options(&self.app, builder)
            .build()
            .map_err(|err| format!("cannot create the tint overlay: {err}"))?;
        // Clicks must pass through before anything else; an overlay that would block them goes away.
        if let Err(err) = window.set_ignore_cursor_events(true) {
            destroy(&window);
            return Err(format!("cannot make the tint overlay click-through: {err}"));
        }
        // The builder maps logical units through whichever monitor matches first; with mixed DPI that can
        // be off, so set the exact physical bounds.
        if let Err(err) = window.set_position(PhysicalPosition::new(bounds.x, bounds.y)) {
            log::debug!("cannot move a tint overlay: {err}");
        }
        if let Err(err) = window.set_size(PhysicalSize::new(bounds.width, bounds.height)) {
            log::debug!("cannot size a tint overlay: {err}");
        }
        Ok(())
    }

    fn destroy_overlays(&mut self) {
        if self.overlays.is_empty() {
            return;
        }
        for overlay in self.overlays.drain(..) {
            if let Some(window) = self.app.get_webview_window(&overlay.label) {
                destroy(&window);
            }
        }
        log::info!("screen tint: overlays removed");
    }

    /// Records the error for `TintStatus` and reports a new one once as `app://error`.
    fn fail(&mut self, message: String) {
        if self.reported_error.as_deref() != Some(message.as_str()) {
            emit_error(&self.app, AppError::new("tint.overlay", message.clone()));
            self.reported_error = Some(message.clone());
        }
        self.error = Some(message);
    }

    /// Emits `tint://changed` when anything but the revision changed.
    fn publish(&mut self, enabled: bool, level: u32, preview: bool) {
        let mut status = TintStatus {
            revision: 0,
            enabled,
            level,
            preview,
            active: !self.overlays.is_empty(),
            error: self.error.clone(),
        };
        if self.last_published.as_ref().is_some_and(|last| {
            let mut last = last.clone();
            last.revision = 0;
            last == status
        }) {
            return;
        }
        {
            let mut shared = lock(&self.status);
            status.revision = shared.revision.wrapping_add(1);
            *shared = status.clone();
        }
        if let Err(err) = self.app.emit(events::TINT_CHANGED, status.clone()) {
            log::warn!("cannot emit tint status: {err}");
        }
        self.last_published = Some(status);
    }
}

fn destroy(window: &WebviewWindow) {
    if let Err(err) = window.destroy() {
        log::warn!("cannot remove a tint overlay: {err}");
    }
}

/// Local minutes after midnight; UTC when the local time is unavailable (non-Windows).
fn local_minutes_of_day() -> f64 {
    focus_platform::clock::local_minutes_of_day().unwrap_or_else(utc_minutes_of_day)
}

fn utc_minutes_of_day() -> f64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    // `secs % SECS_PER_DAY` < 86 400: exact in f64, and the result stays below 1440.
    (secs % SECS_PER_DAY) as f64 / 60.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::tint::MINUTES_PER_DAY;

    #[test]
    fn utc_fallback_is_within_a_day() {
        let minutes = utc_minutes_of_day();
        assert!((0.0..f64::from(MINUTES_PER_DAY)).contains(&minutes));
    }

    #[test]
    fn bounds_compare_layouts_exactly() {
        let a = Bounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale_permille: 1000,
        };
        let moved = Bounds { x: 1920, ..a };
        let rescaled = Bounds {
            scale_permille: 1250,
            ..a
        };
        assert_ne!(a, moved);
        assert_ne!(a, rescaled);
        assert!((rescaled.scale() - 1.25).abs() < 1e-12);
    }
}
