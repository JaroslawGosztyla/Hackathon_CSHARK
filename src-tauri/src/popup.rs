//! Phase popups: a small, app-styled window in the bottom-right corner ("antivirus-style") that replaces
//! the Windows toast when focus or a break starts (`Config::phase_popups`), and tells which game launchers
//! were closed at focus start.
//!
//! One dedicated thread owns the window: the session actor and the shields worker only send requests,
//! so neither ever waits for window creation (which needs the main thread).
//!
//! The window ("popup", route `/popup`) is created for every popup that finds no popup on screen and
//! destroyed when the popup goes away, instead of being hidden and shown again: Tauri shows a window
//! without activating it only when it is created visible with `focused(false)` (later `show()` calls
//! activate it, which could take the keyboard focus from the user's app). A popup that arrives while
//! one is visible just updates it (`popup://show`). The page reads `get_popup` on load and applies only
//! newer revisions; it dismisses itself with `popup_action`, and the shell hides it after
//! [`SAFETY_TIMEOUT`] if the page never loaded (it never called `get_popup`), or after [`LOADED_TIMEOUT`]
//! if it runs but never dismissed it (its own countdown pauses while the pointer or the focus is on it).
//!
//! If the window cannot be created, the popup falls back to the toast it replaces (degradation, never both).
//!
//! The window stays focusable on purpose (no `WS_EX_NOACTIVATE`): its buttons must take clicks reliably.
//! It still appears without activation; only the user activates it (a click, Alt+Tab). It is listed in
//! Alt+Tab while visible (see the note in `crate::tint`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{AppError, GameLauncher, PopupInfo, PopupKind, events};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::shields::ShieldsHandle;
use crate::texts::{self, Toast};
use crate::window_util::{CornerPlacement, bottom_right, with_main_webview_options};
use crate::{emit_error, lock};

/// Label of the popup window (also listed in `capabilities/overlays.json`).
pub const POPUP_WINDOW: &str = "popup";
/// SvelteKit route rendered by the popup window.
const ROUTE: &str = "popup";
/// Logical size of the popup window.
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 150.0;
/// The shell hides a popup within this time when its page never loaded (an empty transparent window
/// would only catch clicks in the corner).
const SAFETY_TIMEOUT: Duration = Duration::from_secs(12);
/// Upper bound for a popup whose page runs: the page dismisses it itself, but holds it while hovered.
const LOADED_TIMEOUT: Duration = Duration::from_secs(120);
/// A `dismiss` this soon after a new popup belongs to the previous one (the page's own timer raced
/// the update) and is ignored. Buttons (`showMain`, `skipPhase`) always close the popup.
const DISMISS_GRACE: Duration = Duration::from_millis(500);
/// After `destroy` the label stays taken until the window is really gone: wait at most this long.
const CLOSE_WAIT: Duration = Duration::from_millis(1500);
const CLOSE_POLL: Duration = Duration::from_millis(20);
/// A "launchers closed" popup this soon after a "focus started" popup shows that focus phase's numbers.
const INHERIT_FOCUS_WITHIN: Duration = Duration::from_secs(30);

/// What to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupRequest {
    pub kind: PopupKind,
    pub cycle: u32,
    /// Length of the phase that just started, in seconds (0 when not applicable).
    pub phase_secs: u32,
    pub launchers: Vec<GameLauncher>,
    /// Shown instead (through the shields worker) when the popup window cannot be created.
    pub fallback: Toast,
}

enum PopupCmd {
    Show(PopupRequest),
    /// `forced`: a button was pressed (no [`DISMISS_GRACE`]).
    Dismiss {
        forced: bool,
    },
}

/// Cheap to clone; every method is non-blocking.
#[derive(Clone)]
pub struct PopupHandle {
    tx: Sender<PopupCmd>,
    info: Arc<Mutex<PopupInfo>>,
    /// The page of the current popup window called `get_popup` (it loaded and runs).
    page_loaded: Arc<AtomicBool>,
}

impl PopupHandle {
    /// Starts the popup thread. `shields` carries the fallback toasts.
    pub fn spawn(app: AppHandle, shields: ShieldsHandle) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let info = Arc::new(Mutex::new(PopupInfo {
            revision: 0,
            kind: None,
            cycle: 0,
            phase_secs: 0,
            launchers: Vec::new(),
        }));
        let shared = Arc::clone(&info);
        let page_loaded = Arc::new(AtomicBool::new(false));
        let loaded = Arc::clone(&page_loaded);
        std::thread::Builder::new()
            .name("focus-popup".into())
            .spawn(move || {
                Worker {
                    app,
                    shields,
                    info: shared,
                    page_loaded: loaded,
                    shown_at: None,
                    deadline: None,
                    last_focus: None,
                }
                .run(rx);
            })?;
        Ok(Self {
            tx,
            info,
            page_loaded,
        })
    }

    pub fn show(&self, request: PopupRequest) {
        self.send(PopupCmd::Show(request));
    }

    /// The page's close button or timeout.
    pub fn dismiss(&self) {
        self.send(PopupCmd::Dismiss { forced: false });
    }

    /// A popup button that acts (show the main window, skip the phase): always closes the popup.
    pub fn close(&self) {
        self.send(PopupCmd::Dismiss { forced: true });
    }

    /// What the popup shows now (`get_popup`). Only the popup page calls it, when it loads: from then on
    /// the shell gives the popup [`LOADED_TIMEOUT`] instead of [`SAFETY_TIMEOUT`].
    pub fn info(&self) -> PopupInfo {
        self.page_loaded.store(true, Ordering::SeqCst);
        lock(&self.info).clone()
    }

    fn send(&self, cmd: PopupCmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("popup thread is not running");
        }
    }
}

struct Worker {
    app: AppHandle,
    shields: ShieldsHandle,
    info: Arc<Mutex<PopupInfo>>,
    /// Set by `get_popup` (the page runs); cleared before a new window is created.
    page_loaded: Arc<AtomicBool>,
    /// When the current popup was shown (`None` = nothing on screen).
    shown_at: Option<Instant>,
    /// Safety timeout of the current popup.
    deadline: Option<Instant>,
    /// Last "focus started" popup: when, cycle, focus length.
    last_focus: Option<(Instant, u32, u32)>,
}

impl Worker {
    fn run(mut self, rx: Receiver<PopupCmd>) {
        loop {
            let cmd = match self.deadline {
                Some(deadline) => {
                    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(cmd) => Some(cmd),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                None => match rx.recv() {
                    Ok(cmd) => Some(cmd),
                    Err(_) => return,
                },
            };
            match cmd {
                Some(PopupCmd::Show(request)) => self.show(request),
                Some(PopupCmd::Dismiss { forced }) => {
                    let fresh = self.shown_at.is_some_and(|at| at.elapsed() < DISMISS_GRACE);
                    if forced || !fresh {
                        self.hide();
                    } else {
                        log::debug!("popup: ignoring a dismiss that raced a new popup");
                    }
                }
                None => {
                    if let Some(shown_at) = self.shown_at {
                        let due = hide_at(shown_at, self.page_loaded.load(Ordering::SeqCst));
                        if Instant::now() < due {
                            // The page runs (maybe held by the pointer): wait for it longer.
                            self.deadline = Some(due);
                            continue;
                        }
                    }
                    log::debug!("popup: not dismissed by the page, hiding it");
                    self.hide();
                }
            }
        }
    }

    fn show(&mut self, mut request: PopupRequest) {
        let now = Instant::now();
        match request.kind {
            PopupKind::FocusStarted => {
                self.last_focus = Some((now, request.cycle, request.phase_secs));
            }
            PopupKind::LaunchersClosed if request.cycle == 0 => {
                if let Some((at, cycle, secs)) = self.last_focus
                    && now.duration_since(at) < INHERIT_FOCUS_WITHIN
                {
                    request.cycle = cycle;
                    request.phase_secs = secs;
                }
            }
            _ => {}
        }
        // Published first: a window created below reads it with `get_popup` when its page loads.
        self.publish(
            Some(request.kind),
            request.cycle,
            request.phase_secs,
            request.launchers.clone(),
        );
        match self.ensure_window() {
            Ok(()) => {
                self.shown_at = Some(now);
                self.deadline = Some(now + SAFETY_TIMEOUT);
            }
            Err(err) => {
                emit_error(&self.app, err);
                self.publish(None, 0, 0, Vec::new());
                self.shown_at = None;
                self.deadline = None;
                self.shields.toast(request.fallback);
            }
        }
    }

    fn hide(&mut self) {
        self.shown_at = None;
        self.deadline = None;
        let showing = lock(&self.info).kind.is_some();
        if showing {
            self.publish(None, 0, 0, Vec::new());
        }
        if let Some(window) = self.app.get_webview_window(POPUP_WINDOW) {
            if let Err(err) = window.destroy() {
                log::warn!("cannot close the popup window: {err}");
                return;
            }
            self.wait_until_gone();
        }
    }

    /// A popup window is on screen: reuse it (the page follows `popup://show`), or create one.
    fn ensure_window(&self) -> Result<(), AppError> {
        if self.app.get_webview_window(POPUP_WINDOW).is_some() {
            return Ok(());
        }
        let placement = bottom_right(&self.app, WIDTH, HEIGHT);
        // The new page has not loaded yet; it reports itself through `get_popup`.
        self.page_loaded.store(false, Ordering::SeqCst);
        let window = self.build(placement).map_err(|err| {
            AppError::new(
                "popup.window",
                format!("cannot create the popup window: {err}"),
            )
        })?;
        if let Some(placement) = placement {
            place_exactly(&window, placement);
        }
        Ok(())
    }

    fn build(&self, placement: Option<CornerPlacement>) -> tauri::Result<WebviewWindow> {
        let mut builder =
            WebviewWindowBuilder::new(&self.app, POPUP_WINDOW, WebviewUrl::App(ROUTE.into()))
                .title(texts::POPUP_WINDOW_TITLE)
                .inner_size(WIDTH, HEIGHT)
                .resizable(false)
                .maximizable(false)
                .minimizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                // Created visible and unfocused: the window appears without activation
                // (`SW_SHOWNOACTIVATE`), so it never takes the keyboard focus from the user's app.
                .focused(false)
                .visible(true);
        builder = match placement {
            Some(placement) => builder.position(placement.logical.0, placement.logical.1),
            None => builder.center(),
        };
        with_main_webview_options(&self.app, builder).build()
    }

    /// `destroy` returns before the window is gone; the next popup needs the label free.
    fn wait_until_gone(&self) {
        let deadline = Instant::now() + CLOSE_WAIT;
        while self.app.get_webview_window(POPUP_WINDOW).is_some() {
            if Instant::now() >= deadline {
                log::warn!("popup window is still closing");
                return;
            }
            std::thread::sleep(CLOSE_POLL);
        }
    }

    /// Bumps the revision, stores the info for `get_popup` and emits `popup://show`.
    fn publish(
        &mut self,
        kind: Option<PopupKind>,
        cycle: u32,
        phase_secs: u32,
        launchers: Vec<GameLauncher>,
    ) {
        let info = {
            let mut info = lock(&self.info);
            info.revision = info.revision.wrapping_add(1);
            info.kind = kind;
            info.cycle = cycle;
            info.phase_secs = phase_secs;
            info.launchers = launchers;
            info.clone()
        };
        if let Err(err) = self.app.emit(events::POPUP_SHOW, info) {
            log::warn!("cannot emit popup: {err}");
        }
    }
}

/// When the shell hides a popup shown at `shown_at` that the page did not dismiss.
fn hide_at(shown_at: Instant, page_loaded: bool) -> Instant {
    shown_at
        + if page_loaded {
            LOADED_TIMEOUT
        } else {
            SAFETY_TIMEOUT
        }
}

/// The builder takes a logical position, which Tauri maps through the scale of whichever monitor it
/// matches first; with mixed DPI that can be off. Move and size it exactly in physical pixels.
fn place_exactly(window: &WebviewWindow, placement: CornerPlacement) {
    if let Err(err) = window.set_position(placement.physical) {
        log::debug!("cannot move the popup: {err}");
    }
    if let Err(err) = window.set_size(placement.size) {
        log::debug!("cannot size the popup: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_page_keeps_its_popup_longer() {
        let shown = Instant::now();
        assert_eq!(hide_at(shown, false), shown + SAFETY_TIMEOUT);
        // The page counts 8 s and pauses while hovered: the shell must not cut it short.
        assert_eq!(hide_at(shown, true), shown + LOADED_TIMEOUT);
    }
}
