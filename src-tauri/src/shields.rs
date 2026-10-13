//! Shields worker: everything that is "up" during focus and "down" during breaks and when idle.
//!
//! One dedicated thread applies the desired state serially (ADR-012): Windows notifications
//! (Do not disturb), keep-awake, the sites blocker flag for the bridge, Teams presence
//! (delegated to the Teams worker so slow HTTP never delays the local shields) and, once at every
//! focus start, closing the selected game launchers (reported as a popup or a toast).
//! `Apply` carries a generation; an older generation is skipped because a newer one is queued.
//! Toasts of the session go through the same queue, so "shields down → toast" keeps its order, and
//! "toast → shields up" waits until the toast had time to appear before muting notifications.
//! No failure ever stops the session: it becomes `IntegrationState::Error` + `app://error`.
//!
//! Notifications crash-restore: the `MuteToken` is written to `shields-restore.json` and the logon
//! safety net (`RunOnce` → `FocusMode.exe --restore-shields`, shared with the screen tint, see
//! `crate::safety_net`) is armed **before** Windows is changed; both are cleared after notifications are
//! restored. Without a saved token we never mute.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{
    AppError, GameLauncher, IntegrationState, IntegrationStatus, PopupKind, ShieldStatus, events,
};
use focus_core::launchers;
use focus_platform::PlatformError;
use focus_platform::notifications::{MuteToken, NotificationMuter};
use focus_platform::power::KeepAwake;
use focus_platform::processes::{ClosedProcess, ProcessCloser};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::bridge::SharedBridge;
use crate::persistence::Persistence;
use crate::popup::{PopupHandle, PopupRequest};
use crate::safety_net::{Pending, SafetyNet};
use crate::teams::TeamsHandle;
use crate::texts::{self, Toast};
use crate::{
    SharedConfig, crash_restore_done, emit_error, emit_startup_error, lock, read_config,
    restore_only_pending,
};

/// How often the worker re-checks the extension connection when nothing else happens.
const IDLE_REFRESH: Duration = Duration::from_secs(5);
/// Teams gets this much more than the focus length, so DND outlives the phase a bit
/// and is still cleared explicitly at the break.
const TEAMS_EXTRA_MINUTES: u32 = 5;
/// How long the Teams worker may take to clear presence on exit.
const TEAMS_EXIT_TIMEOUT: Duration = Duration::from_secs(3);
/// Pause between restoring notifications and showing the next toast.
const DND_SETTLE: Duration = Duration::from_millis(400);
/// Toasts are shown asynchronously; muting notifications sooner than this after the last toast was
/// requested would swallow it (the "back to work" toast right before shields go up).
const TOAST_BEFORE_MUTE: Duration = Duration::from_millis(1500);
/// A `--restore-shields` launch runs early at logon, possibly before the Windows notification service is
/// up: it retries the restore this many times in total, `RESTORE_RETRY_DELAY` apart.
const RESTORE_ATTEMPTS_AT_LOGON: u32 = 4;
const RESTORE_RETRY_DELAY: Duration = Duration::from_secs(5);

pub enum ShieldsCmd {
    /// Desired state: `up` during focus (running or paused).
    Apply { generation: u64, up: bool },
    /// Config changed: re-apply with the new switches.
    ConfigChanged,
    /// Show a toast after everything queued before it was applied.
    Toast(Toast),
    /// Result of the Teams worker applying presence.
    TeamsApplied(IntegrationStatus),
    /// Re-publish the status (e.g. the extension connected).
    Refresh,
    /// Drop all shields synchronously and stop the worker; acks when done.
    Exit(Sender<()>),
}

#[derive(Clone)]
pub struct ShieldsHandle {
    tx: Sender<ShieldsCmd>,
    generation: Arc<AtomicU64>,
    status: Arc<Mutex<ShieldStatus>>,
    bridge: SharedBridge,
}

impl ShieldsHandle {
    pub fn channel(bridge: SharedBridge) -> (Self, Receiver<ShieldsCmd>) {
        let (tx, rx) = mpsc::channel();
        let handle = Self {
            tx,
            generation: Arc::new(AtomicU64::new(0)),
            status: Arc::new(Mutex::new(ShieldStatus {
                revision: 0,
                active: false,
                notifications: off(),
                teams: off(),
                sites: off(),
                keep_awake: off(),
                launchers: off(),
                closed_launchers: Vec::new(),
                extension_connected: false,
            })),
            bridge,
        };
        (handle, rx)
    }

    pub fn set_up(&self, up: bool) {
        let generation = self
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);
        self.send(ShieldsCmd::Apply { generation, up });
    }

    pub fn config_changed(&self) {
        self.send(ShieldsCmd::ConfigChanged);
    }

    pub fn toast(&self, toast: Toast) {
        self.send(ShieldsCmd::Toast(toast));
    }

    pub fn teams_applied(&self, status: IntegrationStatus) {
        self.send(ShieldsCmd::TeamsApplied(status));
    }

    pub fn refresh(&self) {
        self.send(ShieldsCmd::Refresh);
    }

    /// Latest published status, with a fresh `extension_connected`.
    pub fn status(&self) -> ShieldStatus {
        let mut status = lock(&self.status).clone();
        status.extension_connected = lock(&self.bridge).extension_connected();
        status
    }

    /// Drops every shield and waits (bounded) until the worker confirms. Safe to call twice.
    pub fn exit_blocking(&self, timeout: Duration) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self.tx.send(ShieldsCmd::Exit(ack_tx)).is_ok() && ack_rx.recv_timeout(timeout).is_err() {
            log::warn!("shields did not confirm shutdown within {timeout:?}");
        }
    }

    fn send(&self, cmd: ShieldsCmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("shields worker is not running");
        }
    }
}

pub struct ShieldsContext {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub persistence: Persistence,
    pub bridge: SharedBridge,
    pub teams: TeamsHandle,
    /// Reports closed launchers when `phase_popups` is on.
    pub popup: PopupHandle,
    /// Logon restore entry (shared with the screen tint).
    pub safety_net: SafetyNet,
}

pub fn spawn(
    ctx: ShieldsContext,
    handle: &ShieldsHandle,
    rx: Receiver<ShieldsCmd>,
) -> anyhow::Result<()> {
    let generation = Arc::clone(&handle.generation);
    let shared_status = Arc::clone(&handle.status);
    std::thread::Builder::new()
        .name("focus-shields".into())
        .spawn(move || {
            let mut worker = Worker {
                muter: focus_platform::notifications::notification_muter(),
                awake: focus_platform::power::keep_awake(),
                closer: focus_platform::processes::process_closer(),
                ctx,
                generation,
                applied_generation: 0,
                shared_status,
                up: false,
                mute_token: None,
                unmuted_at: None,
                last_toast_at: None,
                keep_awake_on: false,
                teams_requested: None,
                teams_reported: off(),
                notifications: off(),
                keep_awake: off(),
                sites: off(),
                launchers_handled: false,
                launchers: off(),
                closed_launchers: Vec::new(),
                revision: 0,
                last_published: None,
            };
            worker.restore_after_crash();
            // A `--restore-shields` launch exits once the tint is restored too (no-op for a normal
            // launch). The exit goes through the event loop and comes back here as `Exit`, so keep
            // serving the queue.
            crash_restore_done(&worker.ctx.app);
            worker.reconcile();
            worker.run(rx);
        })?;
    Ok(())
}

struct Worker {
    ctx: ShieldsContext,
    generation: Arc<AtomicU64>,
    /// Generation of the last `Apply` handled; differs from `generation` while a newer one is queued.
    applied_generation: u64,
    shared_status: Arc<Mutex<ShieldStatus>>,
    muter: Box<dyn NotificationMuter>,
    awake: Box<dyn KeepAwake>,
    closer: Box<dyn ProcessCloser>,
    /// Desired: shields up.
    up: bool,
    /// Present while notifications are muted by us (or a crash-restore still has to undo them).
    mute_token: Option<MuteToken>,
    /// When notifications were last restored (the next toast waits for `DND_SETTLE`).
    unmuted_at: Option<Instant>,
    /// When the last toast was requested (muting waits for `TOAST_BEFORE_MUTE`).
    last_toast_at: Option<Instant>,
    keep_awake_on: bool,
    /// Last `(on, minutes)` sent to the Teams worker.
    teams_requested: Option<(bool, u32)>,
    teams_reported: IntegrationStatus,
    notifications: IntegrationStatus,
    keep_awake: IntegrationStatus,
    sites: IntegrationStatus,
    /// The launchers were already handled in the current "shields up" period (they are closed once,
    /// when it starts; reset when the shields go down).
    launchers_handled: bool,
    launchers: IntegrationStatus,
    /// Launchers closed at the last focus start.
    closed_launchers: Vec<GameLauncher>,
    revision: u32,
    last_published: Option<ShieldStatus>,
}

impl Worker {
    fn run(&mut self, rx: Receiver<ShieldsCmd>) {
        loop {
            match rx.recv_timeout(IDLE_REFRESH) {
                Ok(ShieldsCmd::Apply { generation, up }) => {
                    if generation != self.generation.load(Ordering::SeqCst) {
                        // A newer desired state is already queued behind this one.
                        continue;
                    }
                    self.applied_generation = generation;
                    self.up = up;
                    self.reconcile();
                }
                Ok(ShieldsCmd::ConfigChanged) => self.reconcile(),
                Ok(ShieldsCmd::Toast(toast)) => self.show_toast(&toast),
                Ok(ShieldsCmd::TeamsApplied(status)) => {
                    self.teams_reported = status;
                    self.publish();
                }
                Ok(ShieldsCmd::Refresh) | Err(RecvTimeoutError::Timeout) => self.publish(),
                Ok(ShieldsCmd::Exit(ack)) => {
                    self.lower_for_exit();
                    let _ = ack.send(());
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.lower_for_exit();
                    return;
                }
            }
        }
    }

    /// A previous run died with notifications muted: restore them from the persisted token.
    /// Runs on this worker thread before the queue is served (it may sleep between retries).
    fn restore_after_crash(&mut self) {
        let Some(token) = self.ctx.persistence.load_shields_restore() else {
            // Nothing pending: drop a stale logon entry (e.g. the restore file was removed by hand).
            self.cancel_logon_restore();
            return;
        };
        log::info!("restoring notifications muted by a previous run");
        let attempts = if restore_only_pending() {
            RESTORE_ATTEMPTS_AT_LOGON
        } else {
            1
        };
        let mut attempt = 1;
        let result = loop {
            let result = self.muter.unmute(&token);
            if result.is_ok() || attempt >= attempts {
                break result;
            }
            if let Err(err) = &result {
                log::warn!("restoring notifications failed (attempt {attempt}/{attempts}): {err}");
            }
            attempt += 1;
            std::thread::sleep(RESTORE_RETRY_DELAY);
        };
        match result {
            Ok(()) => {
                self.ctx.persistence.clear_shields_restore();
                self.cancel_logon_restore();
            }
            Err(err) => {
                emit_startup_error(
                    &self.ctx.app,
                    AppError::new(
                        "shields.notifications",
                        format!("cannot restore notifications after a crash: {err}"),
                    ),
                );
                // Keep the old token (and the file): every lower retries it, and a focus started
                // meanwhile reuses it instead of recording "muted" as the state to restore.
                // Windows consumed the logon entry if it launched us: register it again.
                self.schedule_logon_restore();
                self.mute_token = Some(token);
            }
        }
    }

    /// Arms `FocusMode.exe --restore-shields` for the next logon (best effort: the restore file alone
    /// still repairs notifications on the next start of the app).
    fn schedule_logon_restore(&mut self) {
        self.ctx.safety_net.arm(Pending::Notifications);
    }

    /// Drops the notifications part of the logon entry (it stays while the tint still needs it).
    fn cancel_logon_restore(&mut self) {
        self.ctx.safety_net.disarm(Pending::Notifications);
    }

    fn reconcile(&mut self) {
        let config = read_config(&self.ctx.config);
        let shields = &config.shields;
        self.reconcile_keep_awake(shields.keep_awake);
        self.reconcile_sites(shields.block_sites);
        self.reconcile_teams(
            shields.teams_dnd,
            config.focus_minutes.saturating_add(TEAMS_EXTRA_MINUTES),
        );
        // Before muting: its "launchers closed" toast must be shown while notifications still work.
        self.reconcile_launchers(
            shields.close_launchers,
            &shields.launchers,
            config.phase_popups,
        );
        // Last: muting may first wait for a just-requested toast (`TOAST_BEFORE_MUTE`).
        self.reconcile_notifications(shields.mute_notifications);
        self.publish();
    }

    /// Closes the selected game launchers once per "shields up" period, when it starts (each focus
    /// phase start). Enabling the option during a focus phase takes effect at the next one.
    fn reconcile_launchers(&mut self, enabled: bool, selected: &[GameLauncher], popups: bool) {
        if !self.up {
            self.launchers_handled = false;
            self.launchers = off();
            if !enabled {
                self.closed_launchers.clear();
            }
            return;
        }
        if !enabled || selected.is_empty() {
            self.launchers_handled = true;
            self.launchers = off();
            self.closed_launchers.clear();
            return;
        }
        if self.launchers_handled {
            return;
        }
        if self.generation.load(Ordering::SeqCst) != self.applied_generation {
            // A newer desired state is queued (e.g. Stop right after Start): let it decide.
            return;
        }
        self.launchers_handled = true;
        let names: Vec<&str> = selected
            .iter()
            .flat_map(|launcher| launchers::process_names(*launcher).iter().copied())
            .collect();
        match self.closer.close(&names) {
            Ok(processes) => {
                let closed = closed_launchers(&processes);
                self.launchers = if closed.is_empty() {
                    on_with("no game launcher was running".into())
                } else {
                    log::info!(
                        "closed {} launcher process(es): {}",
                        processes.len(),
                        launcher_names(&closed).join(", ")
                    );
                    on_with(format!("closed {}", launcher_names(&closed).join(", ")))
                };
                if !closed.is_empty() {
                    self.report_closed_launchers(&closed, popups);
                }
                self.closed_launchers = closed;
            }
            Err(err) => {
                self.closed_launchers.clear();
                self.launchers = self.failure("shields.launchers", "close game launchers", &err);
            }
        }
    }

    /// The app's popup when `phase_popups` is on, otherwise a toast (shown before notifications are muted).
    fn report_closed_launchers(&mut self, closed: &[GameLauncher], popups: bool) {
        let toast = texts::launchers_closed_toast(&launcher_names(closed));
        if popups {
            self.ctx.popup.show(PopupRequest {
                kind: PopupKind::LaunchersClosed,
                cycle: 0,
                phase_secs: 0,
                launchers: closed.to_vec(),
                reminder: None,
                fallback: toast,
            });
        } else {
            self.show_toast(&toast);
        }
    }

    fn reconcile_notifications(&mut self, enabled: bool) {
        let want = self.up && enabled;
        if want {
            if self.mute_token.is_some() {
                self.notifications = on();
                return;
            }
            if !self.muter.supported() {
                self.notifications = unavailable("Do not disturb is not supported on this system");
                return;
            }
            self.notifications = match self.mute_notifications() {
                Ok(token) => {
                    self.mute_token = Some(token);
                    on()
                }
                Err(status) => status,
            };
            return;
        }

        if let Some(token) = self.mute_token.clone() {
            match self.muter.unmute(&token) {
                Ok(()) => {
                    self.mute_token = None;
                    self.unmuted_at = Some(Instant::now());
                    self.ctx.persistence.clear_shields_restore();
                    self.cancel_logon_restore();
                }
                Err(err) => {
                    // Keep the token (and the restore file): the next reconcile or the next start retries.
                    self.notifications =
                        self.failure("shields.notifications", "restore notifications", &err);
                    return;
                }
            }
        }
        self.notifications = if enabled && !self.muter.supported() {
            unavailable("Do not disturb is not supported on this system")
        } else {
            off()
        };
    }

    /// Persist, then mute: the restore token (and the logon safety net) exist before Windows is
    /// changed, so a crash at any point can be repaired. `Err` carries the status to publish.
    fn mute_notifications(&mut self) -> Result<MuteToken, IntegrationStatus> {
        self.wait_for_last_toast();
        if self.generation.load(Ordering::SeqCst) != self.applied_generation {
            // A newer desired state was queued while waiting (e.g. Stop right after the break ended).
            return Err(self.notifications.clone());
        }
        let token = match self.muter.prepare_mute() {
            Ok(token) => token,
            Err(err) => {
                return Err(self.failure("shields.notifications", "mute notifications", &err));
            }
        };
        if let Err(err) = self.ctx.persistence.save_shields_restore(&token) {
            // Without the token a crash would leave notifications muted for good: do not mute.
            return Err(self.error_status(
                "shields.notifications",
                format!("cannot mute notifications: the restore file cannot be saved: {err:#}"),
            ));
        }
        self.schedule_logon_restore();
        if let Err(err) = self.muter.apply_mute(&token) {
            self.ctx.persistence.clear_shields_restore();
            self.cancel_logon_restore();
            return Err(self.failure("shields.notifications", "mute notifications", &err));
        }
        Ok(token)
    }

    /// Windows shows toasts asynchronously: give the last one time to appear before muting
    /// (Break → Focus queues the "back to work" toast right before `Apply { up: true }`).
    /// Sleeps only this worker thread, never the session actor.
    fn wait_for_last_toast(&self) {
        let Some(at) = self.last_toast_at else {
            return;
        };
        let remaining = TOAST_BEFORE_MUTE.saturating_sub(at.elapsed());
        if !remaining.is_zero() {
            std::thread::sleep(remaining);
        }
    }

    fn reconcile_keep_awake(&mut self, enabled: bool) {
        let want = self.up && enabled;
        if want != self.keep_awake_on {
            match self.awake.set(want) {
                Ok(()) => self.keep_awake_on = want,
                Err(PlatformError::Unsupported) => {
                    self.keep_awake_on = false;
                    self.keep_awake = unavailable("keep awake is not supported on this system");
                    return;
                }
                Err(err) => {
                    let action = if want {
                        "keep the display awake"
                    } else {
                        "release keep awake"
                    };
                    self.keep_awake = self.failure("shields.keepAwake", action, &err);
                    return;
                }
            }
        }
        self.keep_awake = if want { on() } else { off() };
    }

    /// Only the switch: the list itself comes from `websiteblacklist.txt` (`crate::blocklist`).
    fn reconcile_sites(&mut self, enabled: bool) {
        let want = self.up && enabled;
        lock(&self.ctx.bridge).block_active = want;
        self.sites = if want { on() } else { off() };
    }

    fn reconcile_teams(&mut self, enabled: bool, minutes: u32) {
        let request = (self.up && enabled, minutes);
        if self.teams_requested != Some(request) {
            self.ctx.teams.set_dnd(request.0, request.1);
            self.teams_requested = Some(request);
        }
    }

    /// Exit path: restore notifications and power, unblock sites, clear Teams (bounded).
    fn lower_for_exit(&mut self) {
        self.up = false;
        let config = read_config(&self.ctx.config);
        self.reconcile_notifications(config.shields.mute_notifications);
        self.reconcile_keep_awake(config.shields.keep_awake);
        self.reconcile_sites(config.shields.block_sites);
        self.ctx.teams.exit_blocking(TEAMS_EXIT_TIMEOUT);
        self.publish();
    }

    fn show_toast(&mut self, toast: &Toast) {
        // Windows applies a Do-not-disturb change asynchronously; give it a moment so the break
        // toast queued right behind "shields down" is not swallowed.
        if let Some(at) = self.unmuted_at.take() {
            let waited = at.elapsed();
            if waited < DND_SETTLE {
                std::thread::sleep(DND_SETTLE - waited);
            }
        }
        let result = self
            .ctx
            .app
            .notification()
            .builder()
            .title(toast.title.clone())
            .body(toast.body.clone())
            .show();
        match result {
            Ok(()) => self.last_toast_at = Some(Instant::now()),
            Err(err) => log::warn!("cannot show toast: {err}"),
        }
    }

    fn failure(&self, code: &str, action: &str, err: &PlatformError) -> IntegrationStatus {
        if matches!(err, PlatformError::Unsupported) {
            return unavailable(&format!("cannot {action}: unsupported on this system"));
        }
        self.error_status(code, format!("cannot {action}: {err}"))
    }

    /// Emits `app://error` and returns the matching `Error` status.
    fn error_status(&self, code: &str, message: String) -> IntegrationStatus {
        emit_error(&self.ctx.app, AppError::new(code, message.clone()));
        IntegrationStatus {
            state: IntegrationState::Error,
            detail: Some(message),
        }
    }

    fn current(&self) -> ShieldStatus {
        let config = read_config(&self.ctx.config);
        let extension_connected = lock(&self.ctx.bridge).extension_connected();
        let mut sites = self.sites.clone();
        if sites.state == IntegrationState::On && !extension_connected {
            sites.detail =
                Some("browser extension not connected; using the window title fallback".into());
        }
        ShieldStatus {
            revision: self.revision,
            active: self.up,
            notifications: self.notifications.clone(),
            teams: if config.shields.teams_dnd {
                self.teams_reported.clone()
            } else {
                off()
            },
            sites,
            keep_awake: self.keep_awake.clone(),
            launchers: self.launchers.clone(),
            closed_launchers: self.closed_launchers.clone(),
            extension_connected,
        }
    }

    /// Emits `shields://status` when anything but the revision changed.
    fn publish(&mut self) {
        let mut status = self.current();
        let unchanged = self.last_published.as_ref().is_some_and(|last| {
            let mut last = last.clone();
            last.revision = status.revision;
            last == status
        });
        if unchanged {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
        status.revision = self.revision;
        *lock(&self.shared_status) = status.clone();
        if let Err(err) = self.ctx.app.emit(events::SHIELDS_STATUS, status.clone()) {
            log::warn!("cannot emit shields status: {err}");
        }
        self.last_published = Some(status);
    }
}

fn off() -> IntegrationStatus {
    IntegrationStatus {
        state: IntegrationState::Off,
        detail: None,
    }
}

fn on() -> IntegrationStatus {
    IntegrationStatus {
        state: IntegrationState::On,
        detail: None,
    }
}

fn on_with(detail: String) -> IntegrationStatus {
    IntegrationStatus {
        state: IntegrationState::On,
        detail: Some(detail),
    }
}

fn unavailable(detail: &str) -> IntegrationStatus {
    IntegrationStatus {
        state: IntegrationState::Unavailable,
        detail: Some(detail.to_owned()),
    }
}

/// Launchers whose processes were closed, in the canonical order, without duplicates.
fn closed_launchers(processes: &[ClosedProcess]) -> Vec<GameLauncher> {
    let found: Vec<GameLauncher> = processes
        .iter()
        .filter_map(|process| launchers::launcher_for_process(&process.name))
        .collect();
    launchers::normalized(&found)
}

fn launcher_names(closed: &[GameLauncher]) -> Vec<&'static str> {
    closed
        .iter()
        .map(|launcher| launchers::display_name(*launcher))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(name: &str, pid: u32) -> ClosedProcess {
        ClosedProcess {
            name: name.into(),
            pid,
        }
    }

    #[test]
    fn closed_processes_map_back_to_launchers() {
        let closed = closed_launchers(&[
            process("steamwebhelper.exe", 3),
            process("EpicGamesLauncher.exe", 4),
            process("steam.exe", 2),
            process("unrelated.exe", 5),
        ]);
        assert_eq!(closed, vec![GameLauncher::Steam, GameLauncher::Epic]);
        assert_eq!(
            launcher_names(&closed),
            vec!["Steam", "Epic Games Launcher"]
        );
        assert!(closed_launchers(&[]).is_empty());
    }
}
