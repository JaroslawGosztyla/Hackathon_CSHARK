//! FocusMode desktop shell (Tauri 2).
//!
//! Wires the pure core (`focus-core`), chimes (`focus-audio`) and OS integrations
//! (`focus-platform`) to the WebView UI. Side effects run only here (ADR-012):
//! the session actor owns the state machine, workers own the slow integrations.

mod blocklist;
mod bridge;
mod calendar;
mod chimes;
mod close;
mod commands;
mod effects;
mod media_window;
mod music;
mod persistence;
mod popup;
mod safety_net;
mod session_actor;
mod shields;
mod teams;
mod texts;
mod tint;
mod tracker;
mod tray;
mod window_util;
mod work;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::Duration;

use anyhow::Context;
use focus_core::api::{AppError, Config, Language, SessionSummary, events};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

/// Current, validated config shared by the actor and the workers.
pub(crate) type SharedConfig = Arc<RwLock<Config>>;

pub(crate) const MAIN_WINDOW: &str = "main";
/// Short focus/break profile for presentations.
const DEMO_ARG: &str = "--demo";
/// Launched by the OS autostart entry: start in the tray, without showing the window.
const AUTOSTART_ARG: &str = "--autostart";
/// Launched by the logon safety net (`RunOnce`): restore muted notifications and the screen colours,
/// then exit.
pub(crate) const RESTORE_SHIELDS_ARG: &str = "--restore-shields";

/// `--restore-shields` launch state (see [`finish_restore_only`]).
static RESTORE_ONLY: AtomicU8 = AtomicU8::new(RESTORE_ONLY_OFF);
/// Workers that still run their startup crash-restore (shields, tint); see [`crash_restore_done`].
static CRASH_RESTORES_LEFT: AtomicU8 = AtomicU8::new(CRASH_RESTORES);
const CRASH_RESTORES: u8 = 2;
const RESTORE_ONLY_OFF: u8 = 0;
/// Started only to restore shields: exit once the crash-restore is done.
const RESTORE_ONLY_PENDING: u8 = 1;
/// The exit was requested.
const RESTORE_ONLY_EXITING: u8 = 2;
/// Startup errors are emitted with a delay so the WebView is already listening.
const STARTUP_ERROR_DELAY: Duration = Duration::from_secs(4);
const ACTOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const SHIELDS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const TINT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

/// Managed state used by the commands.
pub struct AppState {
    pub config: SharedConfig,
    /// Serializes `update_config` calls.
    pub config_update: tokio::sync::Mutex<()>,
    pub persistence: persistence::Persistence,
    pub actor: session_actor::ActorHandle,
    pub shields: shields::ShieldsHandle,
    pub tracker: tracker::TrackerHandle,
    pub teams: teams::TeamsHandle,
    pub bridge: bridge::SharedBridge,
    /// `websiteblacklist.txt` (the list of sites to block).
    pub blocklist: blocklist::BlocklistService,
    pub chime: effects::ChimeHandle,
    pub last_summary: Arc<Mutex<Option<SessionSummary>>>,
    pub extension_dir: Option<PathBuf>,
    /// Separate YouTube window for links that refuse embedding.
    pub media: media_window::MediaWindow,
    /// Phase popups (bottom-right corner).
    pub popup: popup::PopupHandle,
    /// Warm screen tint (Night light or display gamma).
    pub tint: tint::TintHandle,
    /// Calendar meetings against the planned breaks.
    pub calendar: calendar::CalendarHandle,
    /// "Co dalej?": work items from the developer tools.
    pub work: work::WorkHandle,
    /// The open "minimize to tray?" question of the first X.
    pub close: close::ClosePrompt,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch = Launch::from_env();
    if launch.restore_only {
        RESTORE_ONLY.store(RESTORE_ONLY_PENDING, Ordering::SeqCst);
    }
    let app = tauri::Builder::default()
        // Must be the first plugin: a second launch only reaches the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            on_second_instance(app, &args);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .setup(move |app| {
            setup(app, launch)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Only the main window decides about its close (tray, quit or ask); the other windows
            // handle their own close.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == MAIN_WINDOW
            {
                // Never close the WebView itself: hidden, it keeps the music playing.
                api.prevent_close();
                close::on_main_close_requested(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::start_session,
            commands::pause_session,
            commands::resume_session,
            commands::stop_session,
            commands::skip_phase,
            commands::get_config,
            commands::update_config,
            commands::list_folder_tracks,
            commands::get_shield_status,
            commands::get_current_usage,
            commands::get_last_summary,
            commands::list_history,
            commands::get_teams_status,
            commands::teams_connect,
            commands::teams_disconnect,
            commands::get_bridge_info,
            commands::open_extension_dir,
            commands::test_chime,
            commands::open_media_window,
            commands::control_media_window,
            commands::get_media_window,
            commands::get_blocklist,
            commands::open_blocklist_file,
            commands::reload_blocklist,
            commands::resolve_close,
            commands::get_system_language,
            commands::get_popup,
            commands::popup_action,
            commands::get_tint_status,
            commands::preview_tint,
            commands::import_chime,
            commands::remove_custom_chime,
            commands::list_library_tracks,
            commands::clear_history,
            commands::get_calendar,
            commands::refresh_calendar,
            commands::set_meeting_choice,
            commands::connect_google,
            commands::disconnect_calendar,
            commands::set_ics_urls,
            commands::get_ics_urls,
            commands::get_work,
            commands::refresh_work,
            commands::set_work_token,
            commands::set_session_goal,
            commands::mark_goal_done,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the FocusMode application");

    app.run(|app, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            shutdown(app);
        }
    });
}

fn setup(app: &mut tauri::App, launch: Launch) -> anyhow::Result<()> {
    let handle = app.handle().clone();
    let demo = launch.demo;
    log::info!(
        "FocusMode {} starting{}{}{}",
        env!("CARGO_PKG_VERSION"),
        if demo { " (demo profile)" } else { "" },
        if launch.autostart { " (autostart)" } else { "" },
        if launch.restore_only {
            " (restoring shields only)"
        } else {
            ""
        }
    );

    let persistence = persistence::Persistence::new(&handle)?;
    let (config, config_error) = persistence.load_config();
    if let Some(err) = config_error {
        emit_startup_error(&handle, err);
    }
    // Before the tray and any window or toast: they are all built in the UI language.
    texts::apply_config_language(config.language);
    log::info!("UI language: {}", texts::language().code());
    music::allow_configured_folder(&handle, config.music.folder.as_deref());
    // Not for a `--restore-shields` launch, which only repairs what a crash left behind.
    if config.launch_on_login && !launch.restore_only {
        refresh_autostart(&handle);
    }
    let config: SharedConfig = Arc::new(RwLock::new(config));

    let last_summary = match persistence.list_history(1) {
        Ok(history) => history.into_iter().next(),
        Err(err) => {
            log::warn!("cannot read session history: {err:#}");
            None
        }
    };
    let last_summary = Arc::new(Mutex::new(last_summary));

    let bridge = bridge::new_shared();
    // Creates websiteblacklist.txt on first run, loads it into the bridge and watches it.
    let blocklist = blocklist::BlocklistService::start(&handle, &persistence, Arc::clone(&bridge));
    let (chime_player, audio_events) = focus_audio::ChimePlayer::spawn();
    effects::forward_audio_errors(handle.clone(), audio_events)
        .context("spawn audio event thread")?;
    let chime = effects::ChimeHandle::new(chime_player, persistence.sounds_dir());

    // One logon restore entry for muted notifications and the screen colours; restores pending from a
    // previous run keep it until their worker has repaired them.
    let safety_net = safety_net::SafetyNet::new(
        persistence.has_shields_restore(),
        persistence.has_tint_restore(),
    );
    // Shields and Teams reference each other: create both channels before spawning.
    let (shields, shields_rx) = shields::ShieldsHandle::channel(Arc::clone(&bridge));
    let (teams, teams_rx) = teams::TeamsHandle::channel();
    // Popups fall back to toasts through the shields queue; shields report closed launchers as popups.
    let popup =
        popup::PopupHandle::spawn(handle.clone(), shields.clone()).context("spawn popup thread")?;
    let tint = tint::TintHandle::spawn(tint::TintContext {
        app: handle.clone(),
        config: Arc::clone(&config),
        persistence: persistence.clone(),
        safety_net: safety_net.clone(),
    })
    .context("spawn tint thread")?;
    teams::spawn(
        teams::TeamsContext {
            app: handle.clone(),
            config: Arc::clone(&config),
            token_path: persistence.teams_token_path(),
            shields: shields.clone(),
        },
        &teams,
        teams_rx,
    )
    .context("spawn Teams worker")?;
    shields::spawn(
        shields::ShieldsContext {
            app: handle.clone(),
            config: Arc::clone(&config),
            persistence: persistence.clone(),
            bridge: Arc::clone(&bridge),
            teams: teams.clone(),
            popup: popup.clone(),
            safety_net,
        },
        &shields,
        shields_rx,
    )
    .context("spawn shields worker")?;

    let tracker = tracker::spawn(tracker::TrackerContext {
        app: handle.clone(),
        config: Arc::clone(&config),
        bridge: Arc::clone(&bridge),
        blocklist: blocklist.shared(),
    })
    .context("spawn tracker")?;

    bridge::spawn(
        handle.clone(),
        Arc::clone(&bridge),
        shields.clone(),
        blocklist.clone(),
    )
    .context("spawn extension bridge")?;

    let (actor, actor_rx) = session_actor::ActorHandle::channel();
    let tray = match tray::build(&handle, actor.clone()) {
        Ok(tray) => Some(tray),
        Err(err) => {
            log::error!("cannot create the tray icon: {err}");
            None
        }
    };
    // The calendar and the actor talk both ways (holds one way, the session timing the other).
    let (calendar, calendar_rx) = calendar::CalendarHandle::channel();
    let idle_profile = if demo {
        focus_core::timing::TimingProfile::demo()
    } else {
        focus_core::timing::TimingProfile::from_config(&read_config(&config))
    };
    calendar::spawn(
        calendar::CalendarContext {
            app: handle.clone(),
            config: Arc::clone(&config),
            persistence: persistence.clone(),
            actor: actor.clone(),
            teams: teams.clone(),
            popup: popup.clone(),
            initial_plan: calendar::SessionPlan::idle(idle_profile),
        },
        &calendar,
        calendar_rx,
    )
    .context("spawn calendar thread")?;
    // Work items ("Co dalej?"): told about every phase change by the actor.
    let (work, work_rx) = work::WorkHandle::channel();
    work::spawn(
        work::WorkContext {
            app: handle.clone(),
            config: Arc::clone(&config),
            persistence: persistence.clone(),
            popup: popup.clone(),
        },
        &work,
        work_rx,
    )
    .context("spawn work items thread")?;
    session_actor::spawn(
        session_actor::ActorContext {
            app: handle.clone(),
            demo,
            config: Arc::clone(&config),
            effects: effects::Effects {
                app: handle.clone(),
                config: Arc::clone(&config),
                chime: chime.clone(),
                shields: shields.clone(),
                popup: popup.clone(),
                tracker: tracker.clone(),
                persistence: persistence.clone(),
                last_summary: Arc::clone(&last_summary),
                last_cycle: 0,
            },
            tracker: tracker.clone(),
            bridge: Arc::clone(&bridge),
            tray,
            calendar: calendar.clone(),
            work: work.clone(),
        },
        actor_rx,
    );

    let extension_dir = find_extension_dir(&handle);
    if extension_dir.is_none() {
        log::warn!("browser extension folder not found");
    }

    app.manage(AppState {
        config,
        config_update: tokio::sync::Mutex::new(()),
        persistence,
        actor,
        shields,
        tracker,
        teams,
        bridge,
        blocklist,
        chime,
        last_summary,
        extension_dir,
        media: media_window::MediaWindow::default(),
        popup,
        tint,
        calendar,
        work,
        close: close::ClosePrompt::default(),
    });
    chimes::clean_up_at_startup(&handle);

    // The window is created hidden (tauri.conf.json): an autostart launch stays in the tray, and a
    // `--restore-shields` launch exits right after the crash-restore without ever showing it.
    if !launch.autostart && !launch.restore_only {
        show_main_window(&handle);
    }
    Ok(())
}

/// Stops the session, drops every shield and restores the screen colours synchronously before the
/// process exits. Idempotent: runs once for "Zakończ", `ExitRequested` and `Exit`.
pub(crate) fn shutdown(app: &AppHandle) {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!("shutting down: stopping the session, lowering shields, restoring screen colours");
    if let Some(state) = app.try_state::<AppState>() {
        state.actor.shutdown_blocking(ACTOR_SHUTDOWN_TIMEOUT);
        state.shields.exit_blocking(SHIELDS_SHUTDOWN_TIMEOUT);
        state.tint.exit_blocking(TINT_SHUTDOWN_TIMEOUT);
    }
}

/// How this process was started.
#[derive(Debug, Clone, Copy)]
struct Launch {
    /// `--demo` argument or `FOCUSMODE_DEMO=1`.
    demo: bool,
    /// `--autostart` (the OS launch-on-login entry): stay in the tray.
    autostart: bool,
    /// `--restore-shields` (the logon safety net): crash-restore, then exit.
    restore_only: bool,
}

impl Launch {
    fn from_env() -> Self {
        // `args_os`: `args()` panics on arguments that are not valid Unicode.
        let has = |flag: &str| {
            std::env::args_os()
                .skip(1)
                .any(|arg| arg.to_str() == Some(flag))
        };
        Self {
            demo: has(DEMO_ARG)
                || std::env::var("FOCUSMODE_DEMO").is_ok_and(|value| {
                    value.trim() == "1" || value.trim().eq_ignore_ascii_case("true")
                }),
            autostart: has(AUTOSTART_ARG),
            restore_only: has(RESTORE_SHIELDS_ARG),
        }
    }
}

/// A second launch was redirected here by the single-instance plugin.
fn on_second_instance(app: &AppHandle, args: &[String]) {
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    if has(RESTORE_SHIELDS_ARG) {
        // This instance already ran the crash-restore when it started.
        return;
    }
    // A real launch arriving while this instance only restores shields at logon: keep running for it.
    if RESTORE_ONLY
        .compare_exchange(
            RESTORE_ONLY_PENDING,
            RESTORE_ONLY_OFF,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
    {
        log::info!("another launch arrived during the logon restore; FocusMode keeps running");
    }
    if has(AUTOSTART_ARG) {
        return;
    }
    if has(DEMO_ARG) {
        // The actor decides: idle → the demo profile from now on; a session runs → a toast asking to stop it.
        match app.try_state::<AppState>() {
            Some(state) => {
                if let Err(err) = state.actor.send(session_actor::ActorCmd::UseDemoProfile) {
                    log::warn!("--demo from another launch not applied: {}", err.message);
                }
            }
            None => log::info!("--demo from another launch arrived before the app was ready"),
        }
    }
    show_main_window(app);
}

/// `true` while this process was started only to restore shields and has not been kept running.
pub(crate) fn restore_only_pending() -> bool {
    RESTORE_ONLY.load(Ordering::SeqCst) == RESTORE_ONLY_PENDING
}

/// `true` for a `--restore-shields` launch that was not kept running (pending or already exiting):
/// nothing visible (screen tint) should appear.
pub(crate) fn restore_only_launch() -> bool {
    RESTORE_ONLY.load(Ordering::SeqCst) != RESTORE_ONLY_OFF
}

/// Called once by the shields worker and once by the tint worker when their startup crash-restore is done.
/// After the last one, a `--restore-shields` launch exits ([`finish_restore_only`]). Non-blocking.
pub(crate) fn crash_restore_done(app: &AppHandle) {
    let previous = CRASH_RESTORES_LEFT.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
        left.checked_sub(1)
    });
    if previous == Ok(1) {
        finish_restore_only(app);
    }
}

/// A `--restore-shields` launch exits now (through the normal exit path, which also lowers shields and
/// restores the screen colours). Non-blocking; no-op otherwise.
fn finish_restore_only(app: &AppHandle) {
    if RESTORE_ONLY
        .compare_exchange(
            RESTORE_ONLY_PENDING,
            RESTORE_ONLY_EXITING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
    {
        log::info!("notifications and screen colours restored at logon; exiting");
        app.exit(0);
    }
}

/// Bundled extension (`$RESOURCE/browser-extension`), or the repo folder in debug builds.
fn find_extension_dir(app: &AppHandle) -> Option<PathBuf> {
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("browser-extension"));
    // Release builds must not embed (or look into) the build machine's source path.
    #[cfg(debug_assertions)]
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|repo| repo.join("browser-extension"));
    #[cfg(not(debug_assertions))]
    let dev: Option<PathBuf> = None;
    [bundled, dev]
        .into_iter()
        .flatten()
        .find(|dir| dir.join("manifest.json").is_file())
        .map(without_verbatim_prefix)
}

/// `resource_dir` is a verbatim path (`\\?\C:\…`) on Windows; users see and copy this path, and
/// browsers' "Load unpacked" dialogs do not accept the prefix.
fn without_verbatim_prefix(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path,
    }
}

/// Keeps the OS autostart entry in line with `launch_on_login`.
pub(crate) fn sync_autostart(app: &AppHandle, enabled: bool) {
    let Some(manager) = app.try_state::<tauri_plugin_autostart::AutoLaunchManager>() else {
        return;
    };
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(err) = result {
        emit_error(
            app,
            AppError::new("autostart", format!("cannot change launch on login: {err}")),
        );
    }
}

/// `launch_on_login` is on: write the OS autostart entry again (disable + enable), so an entry written by an
/// older build or from another install path carries the current executable and [`AUTOSTART_ARG`]. An entry
/// that is missing or switched off at the OS level (e.g. disabled in Task Manager) is left alone: enabling
/// it would override the user's choice there. Errors are only logged.
fn refresh_autostart(app: &AppHandle) {
    let Some(manager) = app.try_state::<tauri_plugin_autostart::AutoLaunchManager>() else {
        return;
    };
    match manager.is_enabled() {
        Ok(true) => {}
        Ok(false) => {
            log::info!(
                "launch on login is on, but the OS entry is missing or disabled; leaving it as is"
            );
            return;
        }
        Err(err) => {
            log::warn!("cannot check the launch-on-login entry: {err}");
            return;
        }
    }
    if let Err(err) = manager.disable() {
        log::warn!("cannot remove the launch-on-login entry before writing it again: {err}");
    }
    match manager.enable() {
        Ok(()) => log::info!("launch-on-login entry written again"),
        Err(err) => log::warn!("cannot write the launch-on-login entry: {err}"),
    }
}

pub(crate) fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if let Err(err) = window.unminimize() {
        log::debug!("unminimize failed: {err}");
    }
    if let Err(err) = window.show() {
        log::debug!("show failed: {err}");
    }
    if let Err(err) = window.set_focus() {
        log::debug!("set_focus failed: {err}");
    }
}

/// Logs and emits `app://error`; the session keeps running.
pub(crate) fn emit_error(app: &AppHandle, error: AppError) {
    log::warn!("{}: {}", error.code, error.message);
    if let Err(err) = app.emit(events::APP_ERROR, error) {
        log::warn!("cannot emit app error: {err}");
    }
}

/// Like [`emit_error`], but delayed so a WebView that is still loading receives it.
pub(crate) fn emit_startup_error(app: &AppHandle, error: AppError) {
    log::warn!("{}: {}", error.code, error.message);
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("focus-startup-error".into())
        .spawn(move || {
            std::thread::sleep(STARTUP_ERROR_DELAY);
            if let Err(err) = app.emit(events::APP_ERROR, error) {
                log::warn!("cannot emit app error: {err}");
            }
        });
    if let Err(err) = spawned {
        log::warn!("cannot report startup error: {err}");
    }
}

/// Locks a mutex, recovering the data if another thread panicked while holding it.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Applies a new `Config.language` to the shell: toasts built from now on, the tray (relabelled with the
/// next snapshot, see `tray::Tray::update`) and the titles of the open YouTube and popup windows.
pub(crate) fn apply_language(app: &AppHandle, choice: Language) {
    if !texts::apply_config_language(choice) {
        return;
    }
    log::info!("UI language: {}", texts::language().code());
    let t = texts::current();
    for (label, title) in [
        (media_window::MEDIA_WINDOW, t.media_window_title),
        (popup::POPUP_WINDOW, t.popup_window_title),
    ] {
        if let Some(window) = app.get_webview_window(label)
            && let Err(err) = window.set_title(title)
        {
            log::debug!("cannot retitle window {label}: {err}");
        }
    }
}

pub(crate) fn read_config(config: &SharedConfig) -> Config {
    config
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub(crate) fn write_config(config: &SharedConfig, value: Config) {
    *config.write().unwrap_or_else(PoisonError::into_inner) = value;
}

#[cfg(test)]
mod tests {
    use super::without_verbatim_prefix;
    use std::path::PathBuf;

    #[test]
    fn strips_only_the_drive_verbatim_prefix() {
        assert_eq!(
            without_verbatim_prefix(PathBuf::from(r"\\?\C:\Apps\FocusMode\browser-extension")),
            PathBuf::from(r"C:\Apps\FocusMode\browser-extension")
        );
        assert_eq!(
            without_verbatim_prefix(PathBuf::from(r"\\?\UNC\server\share\x")),
            PathBuf::from(r"\\?\UNC\server\share\x")
        );
        assert_eq!(
            without_verbatim_prefix(PathBuf::from(r"C:\plain")),
            PathBuf::from(r"C:\plain")
        );
    }
}
