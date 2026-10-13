//! Teams worker: owns the `TeamsClient` (blocking HTTP) on its own thread.
//!
//! - device code login (`teams_connect`): returns the code at once, then polls every `interval_secs`
//!   until the user signs in or the code expires; progress is published as `teams://status`;
//! - presence: the shields worker sends the desired DND state; this worker reconciles it, refreshes
//!   the expiration during long focus phases and reports the result back to the shields worker.
//!   It reads the current presence first: a user who appears offline stays invisible, a DND the user
//!   set themselves is left alone, and only a DND we set (and that is still there) is cleared.
//!
//! The client is recreated when the client id or tenant changes in the config.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{
    AppError, DeviceCodePrompt, IntegrationState, IntegrationStatus, TeamsStatus, events,
};
use focus_platform::PlatformError;
use focus_platform::teams::{
    DeviceLogin, LoginPoll, Presence, TeamsClient, TeamsSettings, presence_blocks_dnd,
    presence_is_user_dnd,
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;

use crate::shields::ShieldsHandle;
use crate::{SharedConfig, emit_error, lock, read_config};

/// Wake up at least this often (login expiry, DND refresh).
const MAX_SLEEP: Duration = Duration::from_secs(30);
/// After a failed `set_dnd`, retry after this long.
const RETRY_AFTER: Duration = Duration::from_secs(60);
/// Graph may report the old presence for a short while after `setUserPreferredPresence`: a DND set
/// more recently than this is cleared without reading the presence first.
const PRESENCE_SETTLE: Duration = Duration::from_secs(60);

pub enum TeamsCmd {
    Connect(oneshot::Sender<Result<DeviceCodePrompt, AppError>>),
    Disconnect(oneshot::Sender<Result<(), AppError>>),
    ConfigChanged,
    /// Desired presence: DND for `minutes` when `on`.
    SetDnd {
        on: bool,
        minutes: u32,
    },
    /// Clear presence if we set it, then ack and stop.
    Exit(Sender<()>),
}

#[derive(Clone)]
pub struct TeamsHandle {
    tx: Sender<TeamsCmd>,
    status: Arc<Mutex<TeamsStatus>>,
}

impl TeamsHandle {
    pub fn channel() -> (Self, Receiver<TeamsCmd>) {
        let (tx, rx) = mpsc::channel();
        let status = TeamsStatus {
            revision: 0,
            configured: false,
            connected: false,
            account: None,
            pending: None,
            error: None,
        };
        (
            Self {
                tx,
                status: Arc::new(Mutex::new(status)),
            },
            rx,
        )
    }

    pub fn status(&self) -> TeamsStatus {
        lock(&self.status).clone()
    }

    pub async fn connect(&self) -> Result<DeviceCodePrompt, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(TeamsCmd::Connect(reply))?;
        response.await.map_err(|_| unavailable_error())?
    }

    pub async fn disconnect(&self) -> Result<(), AppError> {
        let (reply, response) = oneshot::channel();
        self.send(TeamsCmd::Disconnect(reply))?;
        response.await.map_err(|_| unavailable_error())?
    }

    pub fn config_changed(&self) {
        let _ = self.send(TeamsCmd::ConfigChanged);
    }

    pub fn set_dnd(&self, on: bool, minutes: u32) {
        let _ = self.send(TeamsCmd::SetDnd { on, minutes });
    }

    /// Clears presence (if set) and stops the worker, waiting at most `timeout`.
    pub fn exit_blocking(&self, timeout: Duration) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self.tx.send(TeamsCmd::Exit(ack_tx)).is_ok() && ack_rx.recv_timeout(timeout).is_err() {
            log::warn!("Teams presence was not cleared within {timeout:?}");
        }
    }

    fn send(&self, cmd: TeamsCmd) -> Result<(), AppError> {
        self.tx.send(cmd).map_err(|_| {
            log::warn!("Teams worker is not running");
            unavailable_error()
        })
    }
}

fn unavailable_error() -> AppError {
    AppError::new("teams.unavailable", "Teams worker is not running")
}

pub struct TeamsContext {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub token_path: PathBuf,
    pub shields: ShieldsHandle,
}

pub fn spawn(
    ctx: TeamsContext,
    handle: &TeamsHandle,
    rx: Receiver<TeamsCmd>,
) -> anyhow::Result<()> {
    let shared = Arc::clone(&handle.status);
    std::thread::Builder::new()
        .name("focus-teams".into())
        .spawn(move || {
            let key = client_key(&ctx.config);
            let client = TeamsClient::new(TeamsSettings {
                client_id: key.0.clone(),
                tenant: key.1.clone(),
                token_path: ctx.token_path.clone(),
            });
            let mut worker = Worker {
                connected: client.is_configured() && client.has_token(),
                account: client.account(),
                client,
                client_key: key,
                ctx,
                shared,
                login: None,
                error: None,
                revision: 0,
                last_published: None,
                desired_on: false,
                desired_minutes: 0,
                applied_on: false,
                applied_at: None,
                retry_at: None,
                force_refresh: false,
                cleared_at: None,
                skip: None,
                reported: None,
            };
            worker.publish();
            worker.run(rx);
        })?;
    Ok(())
}

struct PendingLogin {
    login: DeviceLogin,
    expires_at: Instant,
    next_poll: Instant,
    interval: Duration,
}

struct Worker {
    ctx: TeamsContext,
    shared: Arc<Mutex<TeamsStatus>>,
    client: TeamsClient,
    client_key: (String, String),
    connected: bool,
    account: Option<String>,
    login: Option<PendingLogin>,
    error: Option<String>,
    revision: u32,
    last_published: Option<TeamsStatus>,
    desired_on: bool,
    desired_minutes: u32,
    applied_on: bool,
    applied_at: Option<Instant>,
    retry_at: Option<Instant>,
    /// Our DND must be sent again now (the focus length changed).
    force_refresh: bool,
    /// When we last cleared our DND (the presence may still show it for a short while).
    cleared_at: Option<Instant>,
    /// Set when DND is left to the user for the current focus phase (reset when focus ends).
    skip: Option<Skip>,
    /// Last presence status reported to the shields worker.
    reported: Option<IntegrationStatus>,
}

enum Flow {
    Continue,
    Stop,
}

impl Worker {
    fn run(&mut self, rx: Receiver<TeamsCmd>) {
        loop {
            let timeout = self
                .next_wakeup()
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(50));
            match rx.recv_timeout(timeout) {
                Ok(cmd) => {
                    if let Flow::Stop = self.handle(cmd) {
                        return;
                    }
                    // Coalesce whatever else is queued (e.g. rapid SetDnd on/off) before any HTTP call.
                    loop {
                        match rx.try_recv() {
                            Ok(cmd) => {
                                if let Flow::Stop = self.handle(cmd) {
                                    return;
                                }
                            }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => {
                                self.desired_on = false;
                                self.reconcile_dnd(false);
                                return;
                            }
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.desired_on = false;
                    self.reconcile_dnd(false);
                    return;
                }
            }
            self.poll_login_if_due();
            self.reconcile_dnd(true);
        }
    }

    fn handle(&mut self, cmd: TeamsCmd) -> Flow {
        match cmd {
            TeamsCmd::Connect(reply) => {
                let result = self.begin_login();
                let _ = reply.send(result);
            }
            TeamsCmd::Disconnect(reply) => {
                let result = self.disconnect();
                let _ = reply.send(result);
            }
            TeamsCmd::ConfigChanged => self.refresh_client(),
            TeamsCmd::SetDnd { on, minutes } => {
                if !on {
                    // The next focus phase reads the user's status again.
                    self.skip = None;
                }
                self.desired_on = on;
                if on && minutes != self.desired_minutes && self.applied_on {
                    // New length: re-apply with the new expiration (still our DND).
                    self.force_refresh = true;
                }
                self.desired_minutes = minutes;
            }
            TeamsCmd::Exit(ack) => {
                self.desired_on = false;
                self.reconcile_dnd(false);
                let _ = ack.send(());
                return Flow::Stop;
            }
        }
        Flow::Continue
    }

    fn next_wakeup(&self) -> Instant {
        let now = Instant::now();
        let mut next = now + MAX_SLEEP;
        if let Some(login) = &self.login {
            next = next.min(login.next_poll).min(login.expires_at);
        }
        if let Some(retry) = self.retry_at {
            next = next.min(retry);
        }
        if self.applied_on
            && let Some(applied) = self.applied_at
        {
            next = next.min(applied + refresh_after(self.desired_minutes));
        }
        next
    }

    /// Recreates the client when the client id or tenant changed.
    fn refresh_client(&mut self) {
        let key = client_key(&self.ctx.config);
        if key == self.client_key {
            self.publish();
            return;
        }
        if self.applied_on {
            if let Err(err) = self.client.clear_dnd() {
                log::warn!("cannot clear Teams presence before switching app registration: {err}");
            }
            self.release();
        }
        // TODO(hackathon): the stored refresh token belongs to the previous app registration;
        // a mismatch surfaces as an auth error and the user reconnects.
        self.client = TeamsClient::new(TeamsSettings {
            client_id: key.0.clone(),
            tenant: key.1.clone(),
            token_path: self.ctx.token_path.clone(),
        });
        self.client_key = key;
        self.login = None;
        self.error = None;
        self.retry_at = None;
        self.connected = self.client.is_configured() && self.client.has_token();
        self.account = self.client.account();
        self.publish();
    }

    fn begin_login(&mut self) -> Result<DeviceCodePrompt, AppError> {
        self.refresh_client();
        if !self.client.is_configured() {
            return Err(AppError {
                code: "teams.notConfigured".into(),
                message: "Teams client id is not set".into(),
                field: Some("shields.teamsClientId".into()),
            });
        }
        match self.client.begin_login() {
            Ok(login) => {
                let now = Instant::now();
                let prompt = DeviceCodePrompt {
                    user_code: login.user_code.clone(),
                    verification_uri: login.verification_uri.clone(),
                    message: login.message.clone(),
                    expires_in_secs: login.expires_in_secs,
                };
                let interval = Duration::from_secs(u64::from(login.interval_secs.max(1)));
                self.login = Some(PendingLogin {
                    expires_at: now + Duration::from_secs(u64::from(login.expires_in_secs.max(1))),
                    next_poll: now + interval,
                    interval,
                    login,
                });
                self.error = None;
                self.publish();
                if let Err(err) = self
                    .ctx
                    .app
                    .opener()
                    .open_url(prompt.verification_uri.clone(), None::<&str>)
                {
                    log::warn!("cannot open the Teams sign-in page: {err}");
                }
                Ok(prompt)
            }
            Err(err) => {
                let message = err.to_string();
                self.error = Some(message.clone());
                self.publish();
                Err(AppError::new(error_code(&err), message))
            }
        }
    }

    fn poll_login_if_due(&mut self) {
        let now = Instant::now();
        let (login, expired, due) = match &self.login {
            Some(pending) => (
                pending.login.clone(),
                now >= pending.expires_at,
                now >= pending.next_poll,
            ),
            None => return,
        };
        if expired {
            self.login = None;
            self.error = Some("The sign-in code expired. Try again.".into());
            self.publish();
            return;
        }
        if !due {
            return;
        }
        match self.client.poll_login(&login) {
            Ok(LoginPoll::Pending) => self.schedule_next_poll(now),
            Ok(LoginPoll::Done { account }) => {
                self.login = None;
                self.connected = true;
                self.account = account.or_else(|| self.client.account());
                self.error = None;
                self.retry_at = None;
                log::info!("Teams connected");
                self.publish();
            }
            Ok(LoginPoll::Failed(message)) => {
                self.login = None;
                self.error = Some(message);
                self.publish();
            }
            Err(err) => {
                // Network hiccup: keep polling until the code expires.
                log::warn!("Teams login poll failed: {err}");
                self.schedule_next_poll(now);
            }
        }
    }

    fn schedule_next_poll(&mut self, now: Instant) {
        if let Some(pending) = self.login.as_mut() {
            pending.next_poll = now + pending.interval;
        }
    }

    fn disconnect(&mut self) -> Result<(), AppError> {
        if self.applied_on {
            if let Err(err) = self.client.clear_dnd() {
                log::warn!("cannot clear Teams presence before disconnecting: {err}");
            }
            self.release();
        }
        self.login = None;
        let result = self.client.disconnect();
        self.connected = false;
        self.account = None;
        self.retry_at = None;
        match result {
            Ok(()) => {
                self.error = None;
                self.publish();
                Ok(())
            }
            Err(err) => {
                let message = err.to_string();
                self.error = Some(message.clone());
                self.publish();
                Err(AppError::new("teams.disconnect", message))
            }
        }
    }

    /// Brings the real presence in line with `desired_on`, respecting the user's own Teams status:
    /// * focus start: "Appear offline" is never turned into a visible DND, and a DND the user already
    ///   has is left alone (not set, and so never cleared) — decided once per focus phase;
    /// * refresh during a long focus: if the user changed the status meanwhile, it is theirs now;
    /// * break / stop: only a DND we set and that is still in place is cleared.
    ///
    /// `check_presence: false` (app exit) clears without reading the presence first (bounded shutdown).
    fn reconcile_dnd(&mut self, check_presence: bool) {
        let now = Instant::now();
        if self.desired_on {
            if !self.client.is_configured() {
                self.report(
                    IntegrationState::Unavailable,
                    Some("Teams client id is not set".into()),
                );
                return;
            }
            if !self.connected {
                self.report(
                    IntegrationState::Unavailable,
                    Some("Teams is not connected".into()),
                );
                return;
            }
            if let Some(skip) = self.skip {
                self.report_skip(skip);
                return;
            }
            let refresh_due = self.force_refresh
                || self.applied_at.is_some_and(|at| {
                    now.duration_since(at) >= refresh_after(self.desired_minutes)
                });
            if self.applied_on && !refresh_due {
                return;
            }
            if self.retry_at.is_some_and(|retry| now < retry) {
                return;
            }
            // Right after our own change Graph may still report the old status: no read then.
            let settled = self
                .applied_at
                .is_none_or(|at| now.duration_since(at) >= PRESENCE_SETTLE);
            if !self.applied_on || settled {
                match self.client.get_presence() {
                    Ok(presence) => {
                        let mut skip = skip_for(&presence, self.applied_on);
                        if skip == Some(Skip::UserDnd)
                            && self
                                .cleared_at
                                .is_some_and(|at| now.duration_since(at) < PRESENCE_SETTLE)
                        {
                            // A short break: Graph still shows the DND we have just cleared.
                            skip = None;
                        }
                        if let Some(skip) = skip {
                            log::info!("Teams: Do not disturb left to the user ({skip:?})");
                            // Never cleared later: the status is the user's.
                            self.release();
                            self.skip = Some(skip);
                            self.report_skip(skip);
                            return;
                        }
                    }
                    Err(err) if !self.applied_on => {
                        self.presence_failed("read Teams presence", err, now);
                        return;
                    }
                    // Our DND is in place: refresh it even without knowing the current status.
                    Err(err) => log::warn!("cannot read Teams presence before refreshing: {err}"),
                }
            }
            match self.client.set_dnd(self.desired_minutes.max(1)) {
                Ok(()) => {
                    self.applied_on = true;
                    self.applied_at = Some(now);
                    self.retry_at = None;
                    self.force_refresh = false;
                    self.report(IntegrationState::On, None);
                }
                Err(err) => self.presence_failed("set Teams presence", err, now),
            }
        } else if self.applied_on {
            if check_presence && !self.ours_to_clear(now) {
                self.release();
                self.report_skip(Skip::UserChanged);
                return;
            }
            self.release();
            match self.client.clear_dnd() {
                Ok(()) => {
                    self.cleared_at = Some(now);
                    self.report(IntegrationState::Off, None);
                }
                Err(err) => {
                    // The presence expires on its own (focus length + a few minutes).
                    let message = format!("cannot clear Teams presence: {err}");
                    emit_error(
                        &self.ctx.app,
                        AppError::new("shields.teams", message.clone()),
                    );
                    self.report(IntegrationState::Error, Some(message));
                }
            }
        } else {
            self.retry_at = None;
            self.report(IntegrationState::Off, None);
        }
    }

    /// Our DND may be cleared: set too recently for the presence to show it yet, still `DoNotDisturb`,
    /// or the presence cannot be read (shields always come down). Not when the user appears offline or
    /// picked another status meanwhile: clearing the preferred presence would drop their choice.
    fn ours_to_clear(&mut self, now: Instant) -> bool {
        if self
            .applied_at
            .is_some_and(|at| now.duration_since(at) < PRESENCE_SETTLE)
        {
            return true;
        }
        match self.client.get_presence() {
            Ok(presence) => !presence_blocks_dnd(&presence) && presence_is_user_dnd(&presence),
            Err(err) => {
                log::warn!("cannot read Teams presence before clearing it: {err}");
                true
            }
        }
    }

    /// Forgets that we set DND (it is not cleared later).
    fn release(&mut self) {
        self.applied_on = false;
        self.applied_at = None;
        self.retry_at = None;
        self.force_refresh = false;
    }

    fn presence_failed(&mut self, action: &str, err: PlatformError, now: Instant) {
        self.retry_at = Some(now + RETRY_AFTER);
        if matches!(err, PlatformError::Auth(_)) {
            self.connected = self.client.has_token();
            self.error = Some(err.to_string());
            self.publish();
        }
        let message = format!("cannot {action}: {err}");
        emit_error(
            &self.ctx.app,
            AppError::new("shields.teams", message.clone()),
        );
        self.report(IntegrationState::Error, Some(message));
    }

    fn report_skip(&mut self, skip: Skip) {
        let (state, detail) = match skip {
            Skip::AppearOffline => (
                IntegrationState::Off,
                "you appear offline in Teams; Do not disturb was not set, so you stay invisible",
            ),
            Skip::UserDnd => (
                IntegrationState::On,
                "you already set Do not disturb in Teams; FocusMode leaves it as it is",
            ),
            Skip::UserChanged => (
                IntegrationState::Off,
                "your Teams status changed during focus; FocusMode leaves it as it is",
            ),
        };
        self.report(state, Some(detail.into()));
    }

    fn report(&mut self, state: IntegrationState, detail: Option<String>) {
        let status = IntegrationStatus { state, detail };
        if self.reported.as_ref() == Some(&status) {
            return;
        }
        self.reported = Some(status.clone());
        self.ctx.shields.teams_applied(status);
    }

    /// Emits `teams://status` when anything but the revision changed.
    fn publish(&mut self) {
        let mut status = TeamsStatus {
            revision: self.revision,
            configured: self.client.is_configured(),
            connected: self.connected,
            account: self.account.clone(),
            pending: self.login.as_ref().map(|pending| DeviceCodePrompt {
                user_code: pending.login.user_code.clone(),
                verification_uri: pending.login.verification_uri.clone(),
                message: pending.login.message.clone(),
                expires_in_secs: u32::try_from(
                    pending
                        .expires_at
                        .saturating_duration_since(Instant::now())
                        .as_secs(),
                )
                .unwrap_or(u32::MAX),
            }),
            error: self.error.clone(),
        };
        let unchanged = self.last_published.as_ref().is_some_and(|last| {
            let mut last = last.clone();
            last.revision = status.revision;
            // The countdown alone is not worth an event.
            if let (Some(a), Some(b)) = (last.pending.as_mut(), status.pending.as_ref()) {
                a.expires_in_secs = b.expires_in_secs;
            }
            last == status
        });
        if unchanged {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
        status.revision = self.revision;
        *lock(&self.shared) = status.clone();
        if let Err(err) = self.ctx.app.emit(events::TEAMS_STATUS, status.clone()) {
            log::warn!("cannot emit Teams status: {err}");
        }
        self.last_published = Some(status);
    }
}

/// Why Do not disturb is left to the user for the rest of a focus phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Skip {
    /// "Appear offline" (or not signed in anywhere): DND would make the user visible.
    AppearOffline,
    /// The user was already in Do not disturb when focus started.
    UserDnd,
    /// The user picked another status after we set DND.
    UserChanged,
}

/// Decision from the presence read before setting (`applied_on: false`) or refreshing (`true`) DND.
fn skip_for(presence: &Presence, applied_on: bool) -> Option<Skip> {
    if presence_blocks_dnd(presence) {
        Some(Skip::AppearOffline)
    } else if applied_on && !presence_is_user_dnd(presence) {
        Some(Skip::UserChanged)
    } else if !applied_on && presence_is_user_dnd(presence) {
        Some(Skip::UserDnd)
    } else {
        None
    }
}

fn client_key(config: &SharedConfig) -> (String, String) {
    let config = read_config(config);
    (
        config.shields.teams_client_id.trim().to_owned(),
        config.shields.teams_tenant.trim().to_owned(),
    )
}

/// Re-send DND a bit before it expires (`minutes` is the requested expiration).
fn refresh_after(minutes: u32) -> Duration {
    Duration::from_secs(u64::from(minutes.saturating_sub(2).max(1)) * 60)
}

fn error_code(err: &PlatformError) -> &'static str {
    match err {
        PlatformError::Network(_) => "teams.network",
        PlatformError::Unsupported => "teams.unsupported",
        _ => "teams.auth",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presence(availability: &str, activity: &str) -> Presence {
        Presence {
            availability: availability.into(),
            activity: activity.into(),
        }
    }

    #[test]
    fn focus_start_respects_the_users_status() {
        assert_eq!(
            skip_for(&presence("Offline", "OffWork"), false),
            Some(Skip::AppearOffline)
        );
        assert_eq!(
            skip_for(&presence("DoNotDisturb", "Presenting"), false),
            Some(Skip::UserDnd)
        );
        for availability in [
            "Available",
            "Busy",
            "Away",
            "BeRightBack",
            "PresenceUnknown",
        ] {
            assert_eq!(
                skip_for(&presence(availability, "Available"), false),
                None,
                "{availability}"
            );
        }
    }

    #[test]
    fn refresh_notices_a_status_the_user_picked() {
        assert_eq!(
            skip_for(&presence("DoNotDisturb", "DoNotDisturb"), true),
            None
        );
        assert_eq!(
            skip_for(&presence("Offline", "OffWork"), true),
            Some(Skip::AppearOffline)
        );
        assert_eq!(
            skip_for(&presence("Available", "Available"), true),
            Some(Skip::UserChanged)
        );
    }
}
