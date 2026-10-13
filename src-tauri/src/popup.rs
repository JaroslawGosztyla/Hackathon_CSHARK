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
//! A work digest (`PopupKind::WorkDigest` from `crate::work`: new work items held back during focus) never
//! replaces the popup on screen: it waits until that one goes away (at a break: the break popup first, then
//! the digest). A popup that arrives while the digest is on screen parks the digest the same way. Only the
//! newest digest is kept, and none outlives the start of a focus phase: a focus-started popup replaces it (it
//! is not parked) and [`PopupHandle::withdraw_work_digest`] (focus started with items held, also without phase
//! popups) removes it from the screen and from the parked ones.
//!
//! While the user presents (`focus_platform::notification_state::user_is_presenting`: a full-screen app or
//! game, Windows presentation settings), no popup window appears, whatever its kind (phase, launchers,
//! meeting reminder, work digest): the newest popup is held instead (it meets the one held before it as it
//! would meet one on screen, so a meeting question or a work digest it displaces waits among the parked ones)
//! and shown as soon as presenting ends, checked every [`PRESENTING_POLL`]. A popup still held
//! [`PRESENTING_HOLD_MAX`] after it arrived is dropped, without a fallback toast (Windows holds toasts back in
//! those modes too, and the main window keeps the state). A popup already on screen is left alone.
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
use focus_platform::notification_state;
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
/// While the user presents, the held popup is retried this often...
const PRESENTING_POLL: Duration = Duration::from_secs(5);
/// ...and dropped when the user still presents this long after it arrived.
const PRESENTING_HOLD_MAX: Duration = Duration::from_secs(10 * 60);

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
    /// Number of new work items of a `WorkDigest` popup (0 otherwise).
    pub work_count: u32,
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
            work_count: 0,
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
    /// Focus started while new work items are held: a work digest on screen or parked goes away.
    WithdrawWorkDigest,
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
        let info = Arc::new(Mutex::new(popup_info(0, None)));
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
                    held: None,
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

    /// Focus started and new work items are held (`crate::work`): a work digest still on screen or waiting
    /// behind another popup goes away, so it never shows during focus.
    pub fn withdraw_work_digest(&self) {
        self.send(PopupCmd::WithdrawWorkDigest);
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
    /// The newest popup that arrived while the user presents (shown when that ends).
    held: Option<Held>,
    /// Logical height of the popup window on screen.
    height: f64,
}

/// A popup held back while the user presents.
struct Held {
    request: PopupRequest,
    /// Dropped when the user still presents at this time.
    until: Instant,
    /// Next look at whether the user still presents.
    next_check: Instant,
}

impl Worker {
    fn run(mut self, rx: Receiver<PopupCmd>) {
        loop {
            let cmd = match self.next_wake() {
                Some(wake) => match rx.recv_timeout(wake.saturating_duration_since(Instant::now()))
                {
                    Ok(cmd) => Some(cmd),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                },
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
                Some(PopupCmd::WithdrawWorkDigest) => self.withdraw_work_digest(),
                None => self.on_timer(),
            }
        }
    }

    /// The earliest of the safety timeout of the popup on screen and the next look at a held popup.
    fn next_wake(&self) -> Option<Instant> {
        let check = self.held.as_ref().map(|held| held.next_check);
        match (self.deadline, check) {
            (Some(deadline), Some(check)) => Some(deadline.min(check)),
            (deadline, check) => deadline.or(check),
        }
    }

    fn on_timer(&mut self) {
        let now = Instant::now();
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.on_deadline(now);
        }
        if self
            .held
            .as_ref()
            .is_some_and(|held| now >= held.next_check)
        {
            self.check_held(now);
        }
    }

    /// The safety timeout of the popup on screen passed.
    fn on_deadline(&mut self, now: Instant) {
        if let Some(shown_at) = self.shown_at {
            let kind = self.current.as_ref().map(|current| current.kind);
            let due = hide_at(shown_at, self.page_loaded.load(Ordering::SeqCst), kind);
            if now < due {
                // The page runs (maybe held by the pointer): wait for it longer.
                self.deadline = Some(due);
                return;
            }
        }
        log::debug!("popup: not dismissed by the page, hiding it");
        self.hide();
    }

    /// A popup arrives: held while the user presents, shown otherwise.
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
        if matches!(
            request.kind,
            PopupKind::WorkDigest | PopupKind::FocusStarted
        ) {
            // Only the newest digest counts, and none survives into a focus phase.
            self.parked
                .retain(|parked| parked.kind != PopupKind::WorkDigest);
        }
        if notification_state::user_is_presenting() {
            self.hold(request, now);
            return;
        }
        if let Some(held) = self.held.take() {
            // Presenting ended before the next look: the held popup and this one meet as on screen, and
            // whichever goes first goes up now.
            request = merge_held(Some(held.request), request, &mut self.parked).0;
        }
        self.display(request);
    }

    /// Keeps `request` back while the user presents (see [`merge_held`]); a newly held popup gets
    /// [`PRESENTING_HOLD_MAX`] from now.
    fn hold(&mut self, request: PopupRequest, now: Instant) {
        log::info!(
            "popup held back while the user presents: {:?}",
            request.kind
        );
        let (previous, until) = match self.held.take() {
            Some(held) => (Some(held.request), Some(held.until)),
            None => (None, None),
        };
        let (request, newly_held) = merge_held(previous, request, &mut self.parked);
        let until = match until {
            Some(until) if !newly_held => until,
            _ => now + PRESENTING_HOLD_MAX,
        };
        self.held = Some(Held {
            request,
            until,
            next_check: now + PRESENTING_POLL,
        });
    }

    /// Shows the held popup once the user no longer presents; drops it when presenting outlasts
    /// [`PRESENTING_HOLD_MAX`].
    fn check_held(&mut self, now: Instant) {
        let presenting = notification_state::user_is_presenting();
        let Some(held) = self.held.take() else {
            return;
        };
        if !presenting {
            if meeting_over(&held.request, wall_ms()) {
                self.show_next_parked();
            } else {
                self.display(held.request);
            }
            return;
        }
        if now < held.until {
            self.held = Some(Held {
                next_check: now + PRESENTING_POLL,
                ..held
            });
            return;
        }
        log::info!(
            "popup dropped: the user still presents {} min after it arrived ({:?})",
            PRESENTING_HOLD_MAX.as_secs() / 60,
            held.request.kind
        );
        if self.current.is_none() {
            // Nothing on screen would bring back the ones waiting behind it.
            self.parked.clear();
        }
    }

    /// The held popup goes away (answered or withdrawn); the ones waiting behind it follow it unless a popup
    /// on screen brings them back when it goes away.
    fn drop_held(&mut self) {
        self.held = None;
        if self.current.is_none() {
            self.show_next_parked();
        }
    }

    /// Shows the popup on screen now (the user does not present).
    fn display(&mut self, request: PopupRequest) {
        let now = Instant::now();
        match arrival(self.current.as_ref(), &request) {
            Arrival::Wait => {
                self.parked.push(request);
                return;
            }
            Arrival::ParkCurrent => {
                if let Some(waiting) = self.current.take() {
                    self.parked.push(waiting);
                }
            }
            Arrival::Replace => {}
        }
        if let Some(id) = request.reminder_meeting_id() {
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
        self.show_next_parked();
    }

    /// The oldest parked popup whose meeting has not ended comes back. While a popup is held, they all keep
    /// waiting: they come after it, once it was shown and went away.
    fn show_next_parked(&mut self) {
        let now = wall_ms();
        self.parked.retain(|parked| !meeting_over(parked, now));
        if self.held.is_some() || self.parked.is_empty() {
            return;
        }
        let next = self.parked.remove(0);
        self.show(next);
    }

    /// A work digest on screen, held or parked goes away (focus started).
    fn withdraw_work_digest(&mut self) {
        self.parked
            .retain(|parked| parked.kind != PopupKind::WorkDigest);
        if self
            .held
            .as_ref()
            .is_some_and(|held| held.request.kind == PopupKind::WorkDigest)
        {
            self.drop_held();
        }
        if self
            .current
            .as_ref()
            .is_some_and(|current| current.kind == PopupKind::WorkDigest)
        {
            self.hide();
        }
    }

    /// The meeting `id` was answered: its reminder goes away (on screen, held or parked).
    fn resolve_meeting(&mut self, id: &str) {
        self.parked
            .retain(|parked| parked.reminder_meeting_id() != Some(id));
        if self
            .held
            .as_ref()
            .and_then(|held| held.request.reminder_meeting_id())
            == Some(id)
        {
            self.drop_held();
        }
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
            work_count: request.work_count,
        },
        None => PopupInfo {
            revision,
            kind: None,
            cycle: 0,
            phase_secs: 0,
            launchers: Vec::new(),
            reminder: None,
            meeting: None,
            work_count: 0,
        },
    }
}

/// How a new popup meets the one on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrival {
    /// Shown now in place of the popup on screen (if any).
    Replace,
    /// Shown now; the popup on screen (a meeting question or a work digest) is parked and comes back after it.
    ParkCurrent,
    /// Parked until the popup on screen goes away (a work digest never replaces another popup).
    Wait,
}

/// See [`Arrival`]. A newer digest updates a digest on screen; a newer reminder of the same meeting replaces
/// its older reminder. A focus start replaces a digest on screen for good (parked, it would come back during
/// focus).
fn arrival(current: Option<&PopupRequest>, request: &PopupRequest) -> Arrival {
    let Some(current) = current else {
        return Arrival::Replace;
    };
    let digest = request.kind == PopupKind::WorkDigest;
    if current.kind == PopupKind::WorkDigest {
        return if digest || request.kind == PopupKind::FocusStarted {
            Arrival::Replace
        } else {
            Arrival::ParkCurrent
        };
    }
    if digest {
        return Arrival::Wait;
    }
    match current.reminder_meeting_id() {
        Some(id) if request.reminder_meeting_id() != Some(id) => Arrival::ParkCurrent,
        _ => Arrival::Replace,
    }
}

/// While the user presents, `request` meets the popup held before it (if any) as it would meet one on
/// screen ([`arrival`]): it replaces it, parks it (a meeting question or a work digest), or waits behind it
/// (a work digest). Returns the popup to hold and whether that is `request`.
fn merge_held(
    held: Option<PopupRequest>,
    request: PopupRequest,
    parked: &mut Vec<PopupRequest>,
) -> (PopupRequest, bool) {
    let Some(held) = held else {
        return (request, true);
    };
    match arrival(Some(&held), &request) {
        Arrival::Replace => (request, true),
        Arrival::ParkCurrent => {
            parked.push(held);
            (request, true)
        }
        Arrival::Wait => {
            parked.push(request);
            (held, false)
        }
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

    fn meeting_reminder(id: &str) -> PopupRequest {
        let conflict = MeetingConflict {
            meeting: focus_core::api::Meeting {
                id: id.into(),
                source: focus_core::api::CalendarSource::Ics,
                subject: "Planowanie".into(),
                start_ms: 0,
                end_ms: 1,
                online: false,
            },
            break_number: 1,
            break_start_ms: 0,
            break_end_ms: 1,
            choice: None,
            next_reminder_ms: None,
            remind_later_allowed: true,
            too_long: false,
        };
        PopupRequest {
            meeting: Some(conflict),
            ..PopupRequest::new(
                PopupKind::MeetingReminder,
                1,
                900,
                texts::meeting_reminder_toast("Planowanie", None, 5, 1),
            )
        }
    }

    fn digest(count: u32) -> PopupRequest {
        PopupRequest {
            work_count: count,
            ..PopupRequest::new(PopupKind::WorkDigest, 0, 0, texts::work_digest_toast(count))
        }
    }

    #[test]
    fn a_work_digest_waits_for_the_popup_on_screen() {
        let break_popup = PopupRequest::new(
            PopupKind::BreakStarted,
            1,
            900,
            texts::break_toast(900, None),
        );
        // Nothing on screen: shown at once.
        assert_eq!(arrival(None, &digest(3)), Arrival::Replace);
        // The break popup stays; the digest comes after it.
        assert_eq!(arrival(Some(&break_popup), &digest(3)), Arrival::Wait);
        assert_eq!(
            arrival(Some(&meeting_reminder("m1")), &digest(3)),
            Arrival::Wait
        );
        // A newer digest updates the one on screen.
        assert_eq!(arrival(Some(&digest(2)), &digest(5)), Arrival::Replace);
        // Any other popup parks the digest on screen.
        assert_eq!(
            arrival(Some(&digest(2)), &break_popup),
            Arrival::ParkCurrent
        );
        assert_eq!(
            arrival(Some(&digest(2)), &meeting_reminder("m1")),
            Arrival::ParkCurrent
        );
        // Meeting questions as before.
        assert_eq!(
            arrival(Some(&meeting_reminder("m1")), &break_popup),
            Arrival::ParkCurrent
        );
        assert_eq!(
            arrival(Some(&meeting_reminder("m1")), &meeting_reminder("m1")),
            Arrival::Replace
        );
        assert_eq!(
            arrival(Some(&meeting_reminder("m1")), &meeting_reminder("m2")),
            Arrival::ParkCurrent
        );
        assert_eq!(arrival(Some(&break_popup), &break_popup), Arrival::Replace);
        // A focus start drops the digest on screen instead of parking it (it would come back during focus).
        let focus_popup = PopupRequest::new(
            PopupKind::FocusStarted,
            2,
            2700,
            texts::break_toast(900, None),
        );
        assert_eq!(arrival(Some(&digest(2)), &focus_popup), Arrival::Replace);

        let info = popup_info(4, Some(&digest(7)));
        assert_eq!(
            (info.kind, info.work_count),
            (Some(PopupKind::WorkDigest), 7)
        );
        assert_eq!(popup_info(5, None).work_count, 0);
    }

    #[test]
    fn the_newest_popup_is_held_while_the_user_presents() {
        let break_popup = PopupRequest::new(
            PopupKind::BreakStarted,
            1,
            900,
            texts::break_toast(900, None),
        );
        let focus_popup = PopupRequest::new(
            PopupKind::FocusStarted,
            2,
            2700,
            texts::focus_toast(2700, 2),
        );
        let mut parked = Vec::new();

        // Nothing held yet: held as it is.
        assert_eq!(
            merge_held(None, break_popup.clone(), &mut parked),
            (break_popup.clone(), true)
        );
        // A newer phase popup replaces the held one (the break is over anyway).
        assert_eq!(
            merge_held(Some(break_popup.clone()), focus_popup.clone(), &mut parked),
            (focus_popup.clone(), true)
        );
        assert!(parked.is_empty());

        // A digest waits behind the held popup; the held one stays (and keeps its time limit).
        assert_eq!(
            merge_held(Some(break_popup.clone()), digest(3), &mut parked),
            (break_popup.clone(), false)
        );
        assert_eq!(parked, [digest(3)]);
        parked.clear();

        // A held meeting question is parked, not lost, when a phase popup arrives.
        assert_eq!(
            merge_held(
                Some(meeting_reminder("m1")),
                break_popup.clone(),
                &mut parked
            ),
            (break_popup.clone(), true)
        );
        assert_eq!(parked, [meeting_reminder("m1")]);
        parked.clear();

        // A focus start drops a held digest for good.
        assert_eq!(
            merge_held(Some(digest(2)), focus_popup.clone(), &mut parked),
            (focus_popup, true)
        );
        assert!(parked.is_empty());
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
