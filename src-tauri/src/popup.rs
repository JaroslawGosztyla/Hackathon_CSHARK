//! Phase popups: a small, app-styled window in the bottom-right corner ("antivirus-style") that replaces
//! the Windows toast when focus or a break starts (`Config::phase_popups`), and tells which game launchers
//! were closed at focus start. Meeting reminders (`PopupKind::MeetingReminder`, from `crate::calendar`)
//! use the same window whatever `phase_popups` says.
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
//! A meeting reminder asks a question, so it is not lost to a phase popup that arrives while it is on
//! screen: it is parked and shown again when that popup goes away, unless the meeting was answered
//! meanwhile ([`PopupHandle::resolve_meeting`]). It gets a taller window and stays up to
//! [`MEETING_TIMEOUT`].
//!
//! The window stays focusable on purpose (no `WS_EX_NOACTIVATE`): its buttons must take clicks reliably.
//! It still appears without activation; only the user activates it (a click, Alt+Tab). It is listed in
//! Alt+Tab while visible.
// TODO(hackathon): tao gives every unowned window `WS_EX_APPWINDOW` and `skip_taskbar` only removes the
// taskbar button; hiding it from Alt+Tab needs `WS_EX_TOOLWINDOW` (a `focus-platform` helper).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, GameLauncher, MeetingConflict, PopupInfo, PopupKind, events};
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
/// A meeting reminder shows the meeting and four answers.
const MEETING_HEIGHT: f64 = 236.0;
/// The shell hides a popup within this time when its page never loaded (an empty transparent window
/// would only catch clicks in the corner).
const SAFETY_TIMEOUT: Duration = Duration::from_secs(12);
/// Upper bound for a popup whose page runs: the page dismisses it itself, but holds it while hovered.
const LOADED_TIMEOUT: Duration = Duration::from_secs(120);
/// Upper bound for a meeting reminder whose page runs (it waits for an answer).
const MEETING_TIMEOUT: Duration = Duration::from_secs(10 * 60);
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
    /// The user's reminder for the break that just started (`Config::break_reminders`).
    pub reminder: Option<String>,
    /// The meeting of a `MeetingReminder` / `BreakHeld` popup.
    pub meeting: Option<MeetingConflict>,
    /// Shown instead (through the shields worker) when the popup window cannot be created.
    pub fallback: Toast,
}

impl PopupRequest {
    /// A popup without launchers, reminder or meeting.
    pub fn new(kind: PopupKind, cycle: u32, phase_secs: u32, fallback: Toast) -> Self {
        Self {
            kind,
            cycle,
            phase_secs,
            launchers: Vec::new(),
            reminder: None,
            meeting: None,
            fallback,
        }
    }

    /// Id of the meeting a `MeetingReminder` asks about.
    fn reminder_meeting_id(&self) -> Option<&str> {
        match (self.kind, &self.meeting) {
            (PopupKind::MeetingReminder, Some(conflict)) => Some(conflict.meeting.id.as_str()),
            _ => None,
        }
    }
}

enum PopupCmd {
    Show(PopupRequest),
    /// `forced`: a button was pressed (no [`DISMISS_GRACE`]).
    Dismiss {
        forced: bool,
    },
    /// The meeting with this id was answered: close or forget its reminder.
    ResolveMeeting(String),
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
            reminder: None,
            meeting: None,
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
                    current: None,
                    parked: Vec::new(),
                    height: HEIGHT,
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

    /// The meeting `meeting_id` was answered: its reminder closes (or is not shown again).
    pub fn resolve_meeting(&self, meeting_id: &str) {
        self.send(PopupCmd::ResolveMeeting(meeting_id.to_owned()));
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
    /// What is on screen (`None` = nothing).
    current: Option<PopupRequest>,
    /// Meeting reminders replaced by another popup, oldest first; shown again when it goes away.
    parked: Vec<PopupRequest>,
    /// Logical height of the popup window on screen.
    height: f64,
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
                Some(PopupCmd::ResolveMeeting(id)) => self.resolve_meeting(&id),
                None => {
                    if let Some(shown_at) = self.shown_at {
                        let kind = self.current.as_ref().map(|current| current.kind);
                        let due = hide_at(shown_at, self.page_loaded.load(Ordering::SeqCst), kind);
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
        // A meeting question on screen gives way to the new popup and comes back after it; a newer
        // reminder of the same meeting replaces it.
        let new_id = request.reminder_meeting_id().map(str::to_owned);
        if let Some(question) = self.current.take_if(|current| {
            current.reminder_meeting_id().is_some()
                && current.reminder_meeting_id() != new_id.as_deref()
        }) {
            self.parked.push(question);
        }
        if let Some(id) = new_id.as_deref() {
            self.parked
                .retain(|parked| parked.reminder_meeting_id() != Some(id));
        }
        // Published first: a window created below reads it with `get_popup` when its page loads.
        self.publish(Some(&request));
        match self.ensure_window(height_for(request.kind)) {
            Ok(()) => {
                self.shown_at = Some(now);
                self.deadline = Some(now + SAFETY_TIMEOUT);
                self.current = Some(request);
            }
            Err(err) => {
                emit_error(&self.app, err);
                self.publish(None);
                self.shown_at = None;
                self.deadline = None;
                self.current = None;
                self.shields.toast(request.fallback);
                // No window now: the parked questions would not find one either.
                for parked in std::mem::take(&mut self.parked) {
                    if !meeting_over(&parked, wall_ms()) {
                        self.shields.toast(parked.fallback);
                    }
                }
            }
        }
    }

    fn hide(&mut self) {
        self.shown_at = None;
        self.deadline = None;
        self.current = None;
        let showing = lock(&self.info).kind.is_some();
        if showing {
            self.publish(None);
        }
        if let Some(window) = self.app.get_webview_window(POPUP_WINDOW) {
            if let Err(err) = window.destroy() {
                log::warn!("cannot close the popup window: {err}");
                return;
            }
            self.wait_until_gone();
        }
        let now = wall_ms();
        self.parked.retain(|parked| !meeting_over(parked, now));
        if !self.parked.is_empty() {
            let next = self.parked.remove(0);
            self.show(next);
        }
    }

    /// The meeting `id` was answered: its reminder goes away (on screen or parked).
    fn resolve_meeting(&mut self, id: &str) {
        self.parked
            .retain(|parked| parked.reminder_meeting_id() != Some(id));
        if self
            .current
            .as_ref()
            .and_then(PopupRequest::reminder_meeting_id)
            == Some(id)
        {
            self.hide();
        }
    }

    /// A popup window is on screen: reuse it (the page follows `popup://show`; moved and resized when
    /// the new popup needs another height), or create one `height` logical px tall.
    fn ensure_window(&mut self, height: f64) -> Result<(), AppError> {
        if let Some(window) = self.app.get_webview_window(POPUP_WINDOW) {
            if (self.height - height).abs() > f64::EPSILON {
                if let Some(placement) = bottom_right(&self.app, WIDTH, height) {
                    place_exactly(&window, placement);
                }
                self.height = height;
            }
            return Ok(());
        }
        let placement = bottom_right(&self.app, WIDTH, height);
        // The new page has not loaded yet; it reports itself through `get_popup`.
        self.page_loaded.store(false, Ordering::SeqCst);
        self.height = height;
        let window = self.build(placement, height).map_err(|err| {
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

    fn build(
        &self,
        placement: Option<CornerPlacement>,
        height: f64,
    ) -> tauri::Result<WebviewWindow> {
        let mut builder =
            WebviewWindowBuilder::new(&self.app, POPUP_WINDOW, WebviewUrl::App(ROUTE.into()))
                .title(texts::POPUP_WINDOW_TITLE)
                .inner_size(WIDTH, height)
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

    /// Bumps the revision, stores the info for `get_popup` and emits `popup://show`; `None` hides.
    fn publish(&mut self, request: Option<&PopupRequest>) {
        let info = {
            let mut info = lock(&self.info);
            let revision = info.revision.wrapping_add(1);
            *info = popup_info(revision, request);
            info.clone()
        };
        if let Err(err) = self.app.emit(events::POPUP_SHOW, info) {
            log::warn!("cannot emit popup: {err}");
        }
    }
}

/// What `get_popup` returns for `request` (`None` = nothing to show).
fn popup_info(revision: u32, request: Option<&PopupRequest>) -> PopupInfo {
    match request {
        Some(request) => PopupInfo {
            revision,
            kind: Some(request.kind),
            cycle: request.cycle,
            phase_secs: request.phase_secs,
            launchers: request.launchers.clone(),
            reminder: request.reminder.clone(),
            meeting: request.meeting.clone(),
        },
        None => PopupInfo {
            revision,
            kind: None,
            cycle: 0,
            phase_secs: 0,
            launchers: Vec::new(),
            reminder: None,
            meeting: None,
        },
    }
}

/// When the shell hides a popup of `kind` shown at `shown_at` that the page did not dismiss.
fn hide_at(shown_at: Instant, page_loaded: bool, kind: Option<PopupKind>) -> Instant {
    shown_at
        + match (page_loaded, kind) {
            (false, _) => SAFETY_TIMEOUT,
            (true, Some(PopupKind::MeetingReminder)) => MEETING_TIMEOUT,
            (true, _) => LOADED_TIMEOUT,
        }
}

/// Logical height of the popup window for `kind`.
fn height_for(kind: PopupKind) -> f64 {
    match kind {
        PopupKind::MeetingReminder => MEETING_HEIGHT,
        _ => HEIGHT,
    }
}

/// A meeting reminder whose meeting has ended (nothing left to decide).
fn meeting_over(request: &PopupRequest, now_ms: i64) -> bool {
    request
        .meeting
        .as_ref()
        .is_some_and(|conflict| conflict.meeting.end_ms <= now_ms)
}

/// Wall clock in Unix milliseconds.
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
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
        let breaks = Some(PopupKind::BreakStarted);
        assert_eq!(hide_at(shown, false, breaks), shown + SAFETY_TIMEOUT);
        // The page counts 8 s and pauses while hovered: the shell must not cut it short.
        assert_eq!(hide_at(shown, true, breaks), shown + LOADED_TIMEOUT);
        // A meeting question waits for an answer much longer, but not when its page never loaded.
        let meeting = Some(PopupKind::MeetingReminder);
        assert_eq!(hide_at(shown, true, meeting), shown + MEETING_TIMEOUT);
        assert_eq!(hide_at(shown, false, meeting), shown + SAFETY_TIMEOUT);
        assert!(height_for(PopupKind::MeetingReminder) > height_for(PopupKind::BreakHeld));
    }

    #[test]
    fn popup_info_carries_the_reminder_and_hides_clean() {
        let request = PopupRequest {
            reminder: Some("Woda i spacer z psem".into()),
            ..PopupRequest::new(
                PopupKind::BreakStarted,
                2,
                900,
                texts::break_toast(900, Some("Woda i spacer z psem")),
            )
        };
        let shown = popup_info(7, Some(&request));
        assert_eq!(shown.revision, 7);
        assert_eq!(shown.kind, Some(PopupKind::BreakStarted));
        assert_eq!((shown.cycle, shown.phase_secs), (2, 900));
        assert_eq!(shown.reminder.as_deref(), Some("Woda i spacer z psem"));

        assert_eq!(shown.meeting, None);
        assert_eq!(request.reminder_meeting_id(), None);

        let hidden = popup_info(8, None);
        assert_eq!(hidden.kind, None);
        assert_eq!(hidden.reminder, None);
        assert_eq!(hidden.meeting, None);
        assert!(hidden.launchers.is_empty());
    }
}
