//! System tray: status line, Start / Pauza|Wznów / Pomiń fazę / Stop, Pokaż okno, Zakończ.
//!
//! Every action goes straight to the session actor channel, so the tray works even when the
//! WebView is hidden, frozen or crashed. The tooltip is refreshed on every phase change and at
//! most every 5 s otherwise.
//!
//! The actor never waits for the main thread: menu and tooltip changes are posted to the main
//! thread fire-and-forget (there every menu call runs inline). The `TrayIcon` itself is never
//! held off the main thread (it wraps an `Rc`); it is looked up by id inside the posted task.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use focus_core::api::{Phase, SessionSnapshot};
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use crate::session_actor::{ActorCmd, ActorHandle};
use crate::{lock, show_main_window, texts};

const TRAY_ID: &str = "focusmode";
const ID_STATUS: &str = "status";
const ID_START: &str = "start";
const ID_PAUSE: &str = "pause";
const ID_SKIP: &str = "skip";
const ID_STOP: &str = "stop";
const ID_SHOW: &str = "show";
const ID_QUIT: &str = "quit";

const TEXT_REFRESH: Duration = Duration::from_secs(5);

pub struct Tray {
    app: AppHandle,
    items: TrayItems,
    view: Mutex<TrayView>,
}

/// Menu items that change with the session. `MenuItem` is `Arc`-based and safe to clone and drop
/// on any thread (its native part is always touched on the main thread).
#[derive(Clone)]
struct TrayItems {
    status: MenuItem<Wry>,
    start: MenuItem<Wry>,
    pause: MenuItem<Wry>,
    skip: MenuItem<Wry>,
    stop: MenuItem<Wry>,
}

struct TrayView {
    phase: Option<Phase>,
    paused: bool,
    last_text: Option<Instant>,
}

/// One batch of tray changes, applied on the main thread.
struct TrayUpdate {
    /// `Some((running, pause_text))` when the phase or the paused flag changed.
    controls: Option<(bool, &'static str)>,
    status: String,
    tooltip: String,
}

pub fn build(app: &AppHandle, actor: ActorHandle) -> tauri::Result<Tray> {
    let status = MenuItem::with_id(app, ID_STATUS, "Gotowy do skupienia", false, None::<&str>)?;
    let start = MenuItem::with_id(app, ID_START, texts::TRAY_START, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, ID_PAUSE, texts::TRAY_PAUSE, false, None::<&str>)?;
    let skip = MenuItem::with_id(app, ID_SKIP, texts::TRAY_SKIP, false, None::<&str>)?;
    let stop = MenuItem::with_id(app, ID_STOP, texts::TRAY_STOP, false, None::<&str>)?;
    let show = MenuItem::with_id(app, ID_SHOW, texts::TRAY_SHOW, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ID_QUIT, texts::TRAY_QUIT, true, None::<&str>)?;
    let separator_top = PredefinedMenuItem::separator(app)?;
    let separator_bottom = PredefinedMenuItem::separator(app)?;

    let items: [&dyn IsMenuItem<Wry>; 9] = [
        &status,
        &separator_top,
        &start,
        &pause,
        &skip,
        &stop,
        &separator_bottom,
        &show,
        &quit,
    ];
    let menu = Menu::with_items(app, &items)?;

    let mut builder = TrayIconBuilder::<Wry>::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("FocusMode")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| on_menu(app, &actor, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    // The app keeps the icon in its resource table (found again with `tray_by_id`); this handle is
    // dropped here, on the main thread (`build` runs in `setup`).
    builder.build(app)?;

    Ok(Tray {
        app: app.clone(),
        items: TrayItems {
            status,
            start,
            pause,
            skip,
            stop,
        },
        view: Mutex::new(TrayView {
            phase: None,
            paused: false,
            last_text: None,
        }),
    })
}

fn on_menu(app: &AppHandle, actor: &ActorHandle, id: &str) {
    let cmd = match id {
        ID_START => ActorCmd::Start,
        ID_PAUSE => ActorCmd::TogglePause,
        ID_SKIP => ActorCmd::Skip,
        ID_STOP => ActorCmd::Stop,
        ID_SHOW => {
            show_main_window(app);
            return;
        }
        ID_QUIT => {
            crate::shutdown(app);
            app.exit(0);
            return;
        }
        _ => return,
    };
    if let Err(err) = actor.send(cmd) {
        log::warn!("tray action {id} failed: {}", err.message);
    }
}

impl Tray {
    /// Called by the actor after every published snapshot. Never blocks: the changes are posted
    /// to the main thread.
    pub fn update(&self, snapshot: &SessionSnapshot) {
        let update = {
            let mut view = lock(&self.view);
            let changed = view.phase != Some(snapshot.phase) || view.paused != snapshot.paused;
            let text_due = view.last_text.is_none_or(|at| at.elapsed() >= TEXT_REFRESH);
            if !changed && !text_due {
                return;
            }
            view.phase = Some(snapshot.phase);
            view.paused = snapshot.paused;
            view.last_text = Some(Instant::now());
            let pause_text = if snapshot.paused {
                texts::TRAY_RESUME
            } else {
                texts::TRAY_PAUSE
            };
            TrayUpdate {
                controls: changed.then_some((snapshot.phase != Phase::Idle, pause_text)),
                status: texts::tray_status(snapshot),
                tooltip: texts::tray_tooltip(snapshot),
            }
        };
        let items = self.items.clone();
        let app = self.app.clone();
        let posted = self.app.run_on_main_thread(move || {
            items.apply(&update);
            if let Some(icon) = app.tray_by_id(TRAY_ID) {
                log_err(icon.set_tooltip(Some(&update.tooltip)));
            }
        });
        if let Err(err) = posted {
            log::debug!("cannot post tray update: {err}");
        }
    }
}

impl TrayItems {
    /// Runs on the main thread, where each menu call is executed inline.
    fn apply(&self, update: &TrayUpdate) {
        if let Some((running, pause_text)) = update.controls {
            log_err(self.start.set_enabled(!running));
            log_err(self.pause.set_enabled(running));
            log_err(self.skip.set_enabled(running));
            log_err(self.stop.set_enabled(running));
            log_err(self.pause.set_text(pause_text));
        }
        log_err(self.status.set_text(&update.status));
    }
}

fn log_err(result: tauri::Result<()>) {
    if let Err(err) = result {
        log::debug!("tray update failed: {err}");
    }
}
