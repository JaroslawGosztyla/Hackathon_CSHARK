//! Calendars: meetings that collide with the breaks (read-only).
//!
//! Sources:
//! * Microsoft 365 / Outlook through Microsoft Graph with the Microsoft sign-in of Teams (`Calendars.Read`,
//!   requested only by a sign-in made while `calendar.microsoft` is on; a sign-in without it shows "reconnect
//!   needed" and the UI signs in again); the Teams worker owns that client, so the fetch goes through
//!   `TeamsHandle::calendar_view`. Disconnecting this source turns it off but keeps the Teams sign-in;
//! * Google Calendar (OAuth desktop flow: loopback redirect + PKCE, the user's own client id); the refresh
//!   token is a sealed file (`google-token.bin`);
//! * any calendar published as an ICS / iCal link (Exchange on-premises publishing, Zoho, Zimbra, Nextcloud,
//!   iCloud, Proton, …, recurring meetings included); the links are a sealed file (`ics-links.bin`).
//!
//! One thread ("focus-calendar") owns the state; the fetches run on their own thread ([`fetch`]) and the
//! Google sign-in on another, so commands are answered at once:
//! * every [`SYNC_EVERY`], on `refresh_calendar`, on a calendar config change and (at most every
//!   [`MIN_AUTO_SYNC_GAP`]) on a session phase change it fetches the meetings of the next `horizon_hours`;
//!   a source whose fetch fails keeps its previous meetings and reports `Error`;
//! * it merges the sources (the same subject starting within a minute is one meeting), checks them against
//!   the breaks of the running session, or of a session that would start now (`focus_core::calendar`), and
//!   publishes `calendar://changed` (own revision) when anything but the revision changed;
//! * it keeps the user's answer per meeting (`meeting-choices.json`: meeting ids only, forgotten after the
//!   meeting) and sends the `Extend` / `PauseAtBreak` ones to the session actor, which turns them into
//!   meeting holds with its own clock;
//! * "remind later" shows a `MeetingReminder` popup 30 / 20 / 10 minutes before the meeting
//!   (`focus_core::calendar::reminder_schedule`), whatever `phase_popups` says (a toast with the subject and
//!   time when the popup window cannot be shown). A reminder is skipped when the meeting no longer
//!   collides with a break.
//!
//! Secrets (the Google refresh token, the ICS links) never reach `config.json` or the log; meeting subjects,
//! links and tokens are never logged either (only counts and redacted errors).

mod fetch;
mod model;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{
    AppError, CalendarConfig, CalendarSource, CalendarSourceStatus, CalendarStatus, Meeting,
    MeetingChoice, MeetingConflict, PopupKind, events,
};
use tauri::{AppHandle, Emitter, Listener};
use tokio::sync::oneshot;

use crate::persistence::Persistence;
use crate::popup::{PopupHandle, PopupRequest};
use crate::session_actor::{ActorCmd, ActorHandle};
use crate::teams::TeamsHandle;
use crate::{SharedConfig, emit_error, lock, read_config, texts};

use model::{Choices, StoredChoices};
pub use model::{SessionPlan, normalize_ics_urls};

/// Calendars are fetched this often.
const SYNC_EVERY: Duration = Duration::from_secs(5 * 60);
/// A session phase change fetches again only when the last fetch started at least this long ago.
const MIN_AUTO_SYNC_GAP: Duration = Duration::from_secs(60);
/// Conflicts are recomputed this often (an idle plan moves with the clock; reminders become due).
const RECOMPUTE_EVERY: Duration = Duration::from_secs(30);
/// A due reminder whose meeting was not read yet (a source has not answered since the start) is tried again
/// after this many milliseconds.
const REMINDER_RETRY_MS: i64 = 30_000;
/// A fetch that did not answer within this time is given up (its late result is dropped).
const SYNC_GIVE_UP: Duration = Duration::from_secs(3 * 60);
/// `refresh_calendar` waits at most this long for the fetch, then returns the current status.
const REFRESH_WAIT: Duration = Duration::from_secs(60);
const SOURCES: [CalendarSource; 3] = [
    CalendarSource::Microsoft,
    CalendarSource::Google,
    CalendarSource::Ics,
];

pub enum CalendarCmd {
    /// Fetch now; the reply (if any) gets the status after the fetch.
    Refresh(Option<oneshot::Sender<CalendarStatus>>),
    ConfigChanged,
    /// The session timing changed (from the actor), taken at the given instant.
    Session(SessionPlan, Instant),
    /// `teams://status` was emitted (the Microsoft sign-in may have changed).
    TeamsChanged,
    SetChoice {
        meeting_id: String,
        choice: MeetingChoice,
        reply: oneshot::Sender<Result<CalendarStatus, AppError>>,
    },
    ConnectGoogle(oneshot::Sender<Result<(), AppError>>),
    Disconnect {
        source: CalendarSource,
        reply: oneshot::Sender<Result<CalendarStatus, AppError>>,
    },
    /// Already validated links (`normalize_ics_urls`).
    SetIcsUrls {
        urls: Vec<String>,
        reply: oneshot::Sender<Result<CalendarStatus, AppError>>,
    },
    GetIcsUrls(oneshot::Sender<Vec<String>>),
    /// A fetch finished (from the fetch thread).
    Fetched(fetch::Fetched),
    /// A Google sign-in ended (from its thread): the account, if known, or an English error.
    GoogleDone {
        attempt: u64,
        result: Result<Option<String>, String>,
    },
}

/// Cheap to clone. The async methods wait for the calendar thread, never block a thread.
#[derive(Clone)]
pub struct CalendarHandle {
    tx: Sender<CalendarCmd>,
    status: Arc<Mutex<CalendarStatus>>,
}

impl CalendarHandle {
    pub fn channel() -> (Self, Receiver<CalendarCmd>) {
        let (tx, rx) = mpsc::channel();
        let status = CalendarStatus {
            revision: 0,
            sources: Vec::new(),
            meetings: Vec::new(),
            conflicts: Vec::new(),
            planned: true,
            last_sync_ms: None,
            google_pending: false,
        };
        (
            Self {
                tx,
                status: Arc::new(Mutex::new(status)),
            },
            rx,
        )
    }

    /// The last published status (`get_calendar`).
    pub fn status(&self) -> CalendarStatus {
        lock(&self.status).clone()
    }

    /// The session timing changed (non-blocking; from the actor).
    pub fn session_changed(&self, plan: SessionPlan) {
        let _ = self.send(CalendarCmd::Session(plan, Instant::now()));
    }

    /// The config was saved (non-blocking).
    pub fn config_changed(&self) {
        let _ = self.send(CalendarCmd::ConfigChanged);
    }

    /// Fetches now and returns the status after the fetch (or the current one after [`REFRESH_WAIT`]).
    pub async fn refresh(&self) -> Result<CalendarStatus, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(CalendarCmd::Refresh(Some(reply)))?;
        match tokio::time::timeout(REFRESH_WAIT, response).await {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(_)) => Err(unavailable_error()),
            Err(_) => Ok(self.status()),
        }
    }

    pub async fn set_choice(
        &self,
        meeting_id: String,
        choice: MeetingChoice,
    ) -> Result<CalendarStatus, AppError> {
        self.request(|reply| CalendarCmd::SetChoice {
            meeting_id,
            choice,
            reply,
        })
        .await?
    }

    /// Opens the Google sign-in in the browser; the result arrives as `calendar://changed`.
    pub async fn connect_google(&self) -> Result<(), AppError> {
        self.request(CalendarCmd::ConnectGoogle).await?
    }

    pub async fn disconnect(&self, source: CalendarSource) -> Result<CalendarStatus, AppError> {
        self.request(|reply| CalendarCmd::Disconnect { source, reply })
            .await?
    }

    /// Stores already validated links (see [`normalize_ics_urls`]).
    pub async fn set_ics_urls(&self, urls: Vec<String>) -> Result<CalendarStatus, AppError> {
        self.request(|reply| CalendarCmd::SetIcsUrls { urls, reply })
            .await?
    }

    pub async fn ics_urls(&self) -> Result<Vec<String>, AppError> {
        self.request(CalendarCmd::GetIcsUrls).await
    }

    async fn request<T>(
        &self,
        make: impl FnOnce(oneshot::Sender<T>) -> CalendarCmd,
    ) -> Result<T, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(make(reply))?;
        response.await.map_err(|_| unavailable_error())
    }

    fn send(&self, cmd: CalendarCmd) -> Result<(), AppError> {
        self.tx.send(cmd).map_err(|_| {
            log::warn!("calendar thread is not running");
            unavailable_error()
        })
    }
}

fn unavailable_error() -> AppError {
    AppError::new("calendar.unavailable", "calendar thread is not running")
}

pub struct CalendarContext {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub persistence: Persistence,
    pub actor: ActorHandle,
    pub teams: TeamsHandle,
    pub popup: PopupHandle,
    /// The plan until the actor reports the first one.
    pub initial_plan: SessionPlan,
}

/// Starts the calendar thread and its fetch thread.
pub fn spawn(
    ctx: CalendarContext,
    handle: &CalendarHandle,
    rx: Receiver<CalendarCmd>,
) -> anyhow::Result<()> {
    let fetcher = fetch::Fetcher::spawn(ctx.teams.clone(), handle.tx.clone())?;
    // The Microsoft source follows the Teams sign-in.
    let teams_tx = handle.tx.clone();
    ctx.app.listen_any(events::TEAMS_STATUS, move |_| {
        let _ = teams_tx.send(CalendarCmd::TeamsChanged);
    });
    let shared = Arc::clone(&handle.status);
    let tx = handle.tx.clone();
    std::thread::Builder::new()
        .name("focus-calendar".into())
        .spawn(move || {
            let mut service = Service::new(ctx, shared, tx, fetcher);
            service.run(rx);
        })?;
    Ok(())
}

/// What one source contributed.
#[derive(Debug, Default)]
struct SourceState {
    /// Meetings of the last successful fetch.
    meetings: Vec<Meeting>,
    /// Redacted English error of the last fetch.
    error: Option<String>,
    /// At least one fetch succeeded since the source was (re)configured.
    synced: bool,
    /// Bumped when the source is switched, reconfigured or disconnected: older results are dropped.
    epoch: u64,
}

impl SourceState {
    fn reset(&mut self) {
        self.meetings.clear();
        self.error = None;
        self.synced = false;
        self.epoch = self.epoch.wrapping_add(1);
    }
}

#[derive(Debug, Default)]
struct GoogleState {
    /// Number of the latest sign-in, shared with the sign-in threads: an older sign-in gives up and its
    /// result is ignored.
    attempt: Arc<AtomicU64>,
    pending: bool,
    has_token: bool,
    account: Option<String>,
}

impl GoogleState {
    /// Starts a new sign-in number (cancels the one in progress); returns it.
    fn next_attempt(&self) -> u64 {
        self.attempt.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
    }

    fn is_current(&self, attempt: u64) -> bool {
        self.attempt.load(Ordering::SeqCst) == attempt
    }
}

struct Service {
    ctx: CalendarContext,
    shared: Arc<Mutex<CalendarStatus>>,
    tx: Sender<CalendarCmd>,
    fetcher: fetch::Fetcher,
    config: CalendarConfig,
    plan: SessionPlan,
    plan_at: Instant,
    teams_connected: bool,
    /// A Teams device code sign-in is in progress (`TeamsStatus.pending`).
    teams_signing_in: bool,
    sources: [SourceState; 3],
    meetings: Vec<Meeting>,
    choices: Choices,
    choices_dirty: bool,
    ics_urls: Vec<String>,
    google: GoogleState,
    /// Id of the fetch in flight and when it started.
    syncing: Option<(u64, Instant)>,
    next_sync_id: u64,
    /// Another fetch was asked for while one was running.
    sync_again: bool,
    waiters: Vec<oneshot::Sender<CalendarStatus>>,
    next_sync: Instant,
    last_sync_started: Option<Instant>,
    last_sync_ms: Option<i64>,
    next_recompute: Instant,
    /// Holds last sent to the actor.
    sent_holds: Option<Vec<(Meeting, MeetingChoice)>>,
    revision: u32,
    last_published: Option<CalendarStatus>,
}

impl Service {
    fn new(
        ctx: CalendarContext,
        shared: Arc<Mutex<CalendarStatus>>,
        tx: Sender<CalendarCmd>,
        fetcher: fetch::Fetcher,
    ) -> Self {
        let now = Instant::now();
        let now_ms = wall_ms();
        let config = read_config(&ctx.config).calendar;
        let choices = ctx
            .persistence
            .load_meeting_choices::<StoredChoices>()
            .map(|stored| Choices::from_stored(stored, now_ms))
            .unwrap_or_default();
        let ics_urls = fetch::load_ics_urls(&ctx.persistence.ics_links_path());
        let (has_token, account) = fetch::google_token(&ctx.persistence.google_token_path());
        let google = GoogleState {
            has_token,
            account,
            ..GoogleState::default()
        };
        let teams = ctx.teams.status();
        let (teams_connected, teams_signing_in) = (teams.connected, teams.pending.is_some());
        let plan = ctx.initial_plan;
        log::info!(
            "calendar: {} stored answers, {} ICS links, Google {}connected",
            choices.to_stored().choices.len(),
            ics_urls.len(),
            if google.has_token { "" } else { "not " }
        );
        Self {
            ctx,
            shared,
            tx,
            fetcher,
            config,
            plan,
            plan_at: now,
            teams_connected,
            teams_signing_in,
            sources: Default::default(),
            meetings: Vec::new(),
            choices,
            choices_dirty: false,
            ics_urls,
            google,
            syncing: None,
            next_sync_id: 0,
            sync_again: false,
            waiters: Vec::new(),
            next_sync: now,
            last_sync_started: None,
            last_sync_ms: None,
            next_recompute: now,
            sent_holds: None,
            revision: 0,
            last_published: None,
        }
    }

    fn run(&mut self, rx: Receiver<CalendarCmd>) {
        loop {
            let timeout = self
                .next_wakeup()
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(50));
            match rx.recv_timeout(timeout) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.on_wake();
        }
    }

    fn next_wakeup(&self) -> Instant {
        let now = Instant::now();
        let mut next = self.next_sync.min(self.next_recompute);
        if let Some((_, started)) = self.syncing {
            next = next.min(started + SYNC_GIVE_UP);
        }
        if let Some(due_ms) = self.choices.next_due() {
            let wait = u64::try_from(due_ms.saturating_sub(wall_ms())).unwrap_or(0);
            next = next.min(now + Duration::from_millis(wait));
        }
        next
    }

    /// The Microsoft source follows the Teams sign-in: a new connection syncs it, a lost one empties it, and
    /// so does the end of a sign-in made while already connected ("Połącz ponownie" to add `Calendars.Read`).
    /// Also checked on every wake-up (the first Teams status may predate the event listener).
    /// `true` when the connection changed.
    fn follow_teams(&mut self) -> bool {
        let status = self.ctx.teams.status();
        let connected = status.connected;
        let signing_in = status.pending.is_some();
        let was_signing_in = std::mem::replace(&mut self.teams_signing_in, signing_in);
        if !microsoft_signin_changed(
            (self.teams_connected, was_signing_in),
            (connected, signing_in),
        ) {
            return false;
        }
        self.teams_connected = connected;
        self.source_mut(CalendarSource::Microsoft).reset();
        if connected && self.config.microsoft {
            self.start_sync();
        }
        true
    }

    fn on_wake(&mut self) {
        let teams_changed = self.follow_teams();
        let now = Instant::now();
        if let Some((_, started)) = self.syncing
            && now.duration_since(started) >= SYNC_GIVE_UP
        {
            log::warn!("calendar: the fetch did not finish, giving up on it");
            self.syncing = None;
            self.answer_waiters();
        }
        if now >= self.next_sync {
            self.start_sync();
        }
        let reminder_due = self.choices.next_due().is_some_and(|due| due <= wall_ms());
        if teams_changed || reminder_due || now >= self.next_recompute {
            self.rebuild();
        }
    }

    fn handle(&mut self, cmd: CalendarCmd) {
        match cmd {
            CalendarCmd::Refresh(reply) => {
                if let Some(reply) = reply {
                    self.waiters.push(reply);
                }
                self.start_sync();
                if self.syncing.is_none() {
                    // Nothing to fetch: answer at once.
                    self.rebuild();
                    self.answer_waiters();
                }
            }
            CalendarCmd::ConfigChanged => self.config_changed(),
            CalendarCmd::Session(plan, at) => {
                let phase_changed = plan.phase != self.plan.phase || plan.cycle != self.plan.cycle;
                self.plan = plan;
                self.plan_at = at;
                if phase_changed
                    && self
                        .last_sync_started
                        .is_none_or(|started| started.elapsed() >= MIN_AUTO_SYNC_GAP)
                {
                    self.start_sync();
                }
                self.rebuild();
            }
            CalendarCmd::TeamsChanged => {
                self.follow_teams();
                self.rebuild();
            }
            CalendarCmd::SetChoice {
                meeting_id,
                choice,
                reply,
            } => {
                let result = self.set_choice(&meeting_id, choice);
                let _ = reply.send(result);
            }
            CalendarCmd::ConnectGoogle(reply) => self.connect_google(reply),
            CalendarCmd::Disconnect { source, reply } => {
                let result = self.disconnect(source);
                let _ = reply.send(result);
            }
            CalendarCmd::SetIcsUrls { urls, reply } => {
                let result = self.set_ics_urls(urls);
                let _ = reply.send(result);
            }
            CalendarCmd::GetIcsUrls(reply) => {
                let _ = reply.send(self.ics_urls.clone());
            }
            CalendarCmd::Fetched(fetched) => self.fetched(fetched),
            CalendarCmd::GoogleDone { attempt, result } => self.google_done(attempt, result),
        }
    }

    // -- fetching ------------------------------------------------------------------------------------

    fn start_sync(&mut self) {
        let now = Instant::now();
        self.next_sync = now + SYNC_EVERY;
        if self.syncing.is_some() {
            self.sync_again = true;
            return;
        }
        let now_ms = wall_ms();
        let job = fetch::Job {
            id: self.next_sync_id,
            from_ms: now_ms,
            to_ms: now_ms.saturating_add(self.config.horizon_ms()),
            microsoft: (self.config.microsoft && self.microsoft_missing().is_none())
                .then(|| self.source(CalendarSource::Microsoft).epoch),
            google: (self.config.google && self.google_missing().is_none()).then(|| {
                fetch::GoogleJob {
                    epoch: self.source(CalendarSource::Google).epoch,
                    client_id: self.config.google_client_id.clone(),
                    client_secret: self.config.google_client_secret.clone(),
                    token_path: self.ctx.persistence.google_token_path(),
                }
            }),
            ics: (self.config.ics && !self.ics_urls.is_empty()).then(|| fetch::IcsJob {
                epoch: self.source(CalendarSource::Ics).epoch,
                urls: self.ics_urls.clone(),
            }),
        };
        if job.is_empty() {
            return;
        }
        self.next_sync_id = self.next_sync_id.wrapping_add(1);
        let id = job.id;
        if self.fetcher.fetch(job) {
            self.syncing = Some((id, now));
            self.last_sync_started = Some(now);
        } else {
            log::warn!("calendar: the fetch thread is not running");
        }
    }

    fn fetched(&mut self, fetched: fetch::Fetched) {
        if self.syncing.map(|(id, _)| id) != Some(fetched.id) {
            log::debug!("calendar: dropping the result of an abandoned fetch");
            return;
        }
        self.syncing = None;
        self.last_sync_ms = Some(fetched.at_ms);
        let mut summary = Vec::new();
        for result in fetched.results {
            let source = result.source;
            let state = &mut self.sources[source_index(source)];
            if state.epoch != result.epoch {
                continue;
            }
            match result.outcome {
                Ok(meetings) => {
                    summary.push(format!("{source:?}: {} meetings", meetings.len()));
                    state.meetings = meetings;
                    state.error = None;
                    state.synced = true;
                }
                Err(error) => {
                    summary.push(format!("{source:?}: error"));
                    let first = state.error.is_none();
                    state.error = Some(error.clone());
                    if first {
                        emit_error(
                            &self.ctx.app,
                            AppError::new(
                                "calendar.sync",
                                format!("cannot read the {source:?} calendar: {error}"),
                            ),
                        );
                    }
                }
            }
            if let Some(account) = result.account {
                self.google.account = Some(account);
            }
        }
        log::info!("calendar: synced ({})", summary.join(", "));
        self.rebuild();
        self.answer_waiters();
        if std::mem::take(&mut self.sync_again) {
            self.start_sync();
        }
    }

    fn answer_waiters(&mut self) {
        if self.waiters.is_empty() {
            return;
        }
        let status = lock(&self.shared).clone();
        for waiter in self.waiters.drain(..) {
            let _ = waiter.send(status.clone());
        }
    }

    // -- commands ------------------------------------------------------------------------------------

    fn config_changed(&mut self) {
        let config = read_config(&self.ctx.config).calendar;
        if config == self.config {
            // Focus / break minutes are reported by the actor (the idle plan).
            return;
        }
        let old = std::mem::replace(&mut self.config, config);
        if old.microsoft != self.config.microsoft {
            self.source_mut(CalendarSource::Microsoft).reset();
        }
        if old.google != self.config.google
            || old.google_client_id != self.config.google_client_id
            || old.google_client_secret != self.config.google_client_secret
        {
            self.source_mut(CalendarSource::Google).reset();
        }
        if old.ics != self.config.ics {
            self.source_mut(CalendarSource::Ics).reset();
        }
        self.start_sync();
        self.rebuild();
    }

    fn set_choice(
        &mut self,
        meeting_id: &str,
        choice: MeetingChoice,
    ) -> Result<CalendarStatus, AppError> {
        let now_ms = wall_ms();
        let meeting = self
            .meetings
            .iter()
            .find(|meeting| meeting.id == meeting_id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "calendar.unknownMeeting",
                    "the meeting is no longer in the calendar",
                )
            })?;
        self.choices.set(&meeting, choice, now_ms)?;
        self.choices_dirty = true;
        log::info!("calendar: a meeting was answered with {choice:?}");
        // The reminder popup of that meeting (if any) has its answer.
        self.ctx.popup.resolve_meeting(&meeting.id);
        self.rebuild();
        Ok(lock(&self.shared).clone())
    }

    fn connect_google(&mut self, reply: oneshot::Sender<Result<(), AppError>>) {
        let client_id = self.config.google_client_id.trim().to_owned();
        if client_id.is_empty() {
            let _ = reply.send(Err(AppError {
                code: "calendar.googleNotConfigured".into(),
                message: "the Google OAuth client id is not set".into(),
                field: Some("calendar.googleClientId".into()),
            }));
            return;
        }
        // A sign-in still waiting for its browser redirect gives up.
        let attempt = self.google.next_attempt();
        let started = fetch::spawn_google_login(fetch::GoogleLogin {
            app: self.ctx.app.clone(),
            client_id,
            client_secret: self.config.google_client_secret.clone(),
            token_path: self.ctx.persistence.google_token_path(),
            attempt,
            current: Arc::clone(&self.google.attempt),
            done: self.tx.clone(),
            reply,
        });
        self.google.pending = started;
        if started {
            log::info!("calendar: Google sign-in started");
            self.source_mut(CalendarSource::Google).error = None;
        }
        self.rebuild();
    }

    fn google_done(&mut self, attempt: u64, result: Result<Option<String>, String>) {
        if !self.google.is_current(attempt) {
            return;
        }
        self.google.pending = false;
        match result {
            Ok(account) => {
                log::info!("calendar: Google connected");
                self.google.has_token = true;
                self.google.account = account;
                let state = self.source_mut(CalendarSource::Google);
                state.reset();
                if self.config.google {
                    self.start_sync();
                }
            }
            Err(error) => {
                let error = model::redact(&error);
                log::warn!("calendar: Google sign-in failed: {error}");
                self.source_mut(CalendarSource::Google).error =
                    Some(format!("sign-in failed: {error}"));
            }
        }
        self.rebuild();
    }

    fn disconnect(&mut self, source: CalendarSource) -> Result<CalendarStatus, AppError> {
        match source {
            // The Microsoft sign-in belongs to Teams and stays: `disconnect_calendar` has already turned
            // `calendar.microsoft` off (config saved); only the Microsoft meetings go (`reset` below).
            // Takes that config now in case its `ConfigChanged` was not sent (it was off already).
            CalendarSource::Microsoft => self.config_changed(),
            CalendarSource::Google => {
                // Also cancels a sign-in in progress.
                self.google.next_attempt();
                self.google.pending = false;
                // Revokes the token at Google (best effort, bounded by the platform's HTTP timeout) and
                // deletes it.
                fetch::disconnect_google(&self.ctx.persistence.google_token_path()).map_err(
                    |err| {
                        AppError::new(
                            "calendar.disconnect",
                            format!("cannot delete the Google token: {err}"),
                        )
                    },
                )?;
                self.google.has_token = false;
                self.google.account = None;
            }
            CalendarSource::Ics => {
                fetch::delete_ics_urls(&self.ctx.persistence.ics_links_path()).map_err(|err| {
                    AppError::new(
                        "calendar.disconnect",
                        format!("cannot delete the ICS links: {err}"),
                    )
                })?;
                self.ics_urls.clear();
            }
        }
        log::info!("calendar: {source:?} disconnected");
        self.source_mut(source).reset();
        self.rebuild();
        Ok(lock(&self.shared).clone())
    }

    fn set_ics_urls(&mut self, urls: Vec<String>) -> Result<CalendarStatus, AppError> {
        // Sealed; an empty list deletes the file. `webcal://` links are stored as `https://`.
        let stored = fetch::save_ics_urls(&self.ctx.persistence.ics_links_path(), &urls)?;
        log::info!("calendar: {} ICS links stored", stored.len());
        self.ics_urls = stored;
        self.source_mut(CalendarSource::Ics).reset();
        self.start_sync();
        self.rebuild();
        Ok(lock(&self.shared).clone())
    }

    // -- state ---------------------------------------------------------------------------------------

    fn source(&self, source: CalendarSource) -> &SourceState {
        &self.sources[source_index(source)]
    }

    fn source_mut(&mut self, source: CalendarSource) -> &mut SourceState {
        &mut self.sources[source_index(source)]
    }

    fn enabled(&self, source: CalendarSource) -> bool {
        match source {
            CalendarSource::Microsoft => self.config.microsoft,
            CalendarSource::Google => self.config.google,
            CalendarSource::Ics => self.config.ics,
        }
    }

    fn microsoft_missing(&self) -> model::Missing {
        let teams = self.ctx.teams.status();
        if !teams.configured {
            Some("set up the Microsoft app registration (client id) in the Teams settings")
        } else if !teams.connected {
            Some("connect the Microsoft account in the Teams settings")
        } else {
            None
        }
    }

    fn google_missing(&self) -> model::Missing {
        if self.config.google_client_id.trim().is_empty() {
            Some("enter the Google OAuth client id")
        } else if self.google.pending {
            Some("finish the sign-in in the browser")
        } else if !self.google.has_token {
            Some("connect the Google account")
        } else {
            None
        }
    }

    fn missing(&self, source: CalendarSource) -> model::Missing {
        match source {
            CalendarSource::Microsoft => self.microsoft_missing(),
            CalendarSource::Google => self.google_missing(),
            CalendarSource::Ics if self.ics_urls.is_empty() => Some("add an ICS link"),
            CalendarSource::Ics => None,
        }
    }

    /// Every source that can deliver meetings has delivered them at least once (since it was (re)configured).
    fn all_sources_read(&self) -> bool {
        SOURCES.iter().all(|&source| {
            !self.enabled(source) || self.missing(source).is_some() || self.source(source).synced
        })
    }

    fn account(&self, source: CalendarSource) -> Option<String> {
        match source {
            CalendarSource::Microsoft => self.ctx.teams.status().account,
            CalendarSource::Google => self.google.account.clone(),
            CalendarSource::Ics => {
                (!self.ics_urls.is_empty()).then(|| self.ics_urls.len().to_string())
            }
        }
    }

    fn source_statuses(&self) -> Vec<CalendarSourceStatus> {
        SOURCES
            .iter()
            .map(|&source| {
                let state = self.source(source);
                model::source_status(
                    source,
                    self.enabled(source),
                    self.missing(source),
                    state.error.as_deref(),
                    state.synced,
                    self.account(source),
                )
            })
            .collect()
    }

    /// Merges the sources, follows the answers, delivers due reminders, updates the session's holds and
    /// publishes the status.
    fn rebuild(&mut self) {
        let now_ms = wall_ms();
        self.next_recompute = Instant::now() + RECOMPUTE_EVERY;
        let lists: Vec<&[Meeting]> = SOURCES
            .iter()
            .filter(|&&source| self.enabled(source))
            .map(|&source| self.source(source).meetings.as_slice())
            .collect();
        let merged = model::merge(
            &lists,
            now_ms,
            now_ms.saturating_add(self.config.horizon_ms()),
        );
        if self.choices.follow(&merged) {
            self.choices_dirty = true;
        }
        if self.choices.prune(now_ms) {
            self.choices_dirty = true;
        }
        self.meetings = merged.meetings;
        self.deliver_reminders(now_ms);
        self.save_choices();
        self.send_holds();
        self.publish(now_ms);
    }

    /// Shows the reminders that are due (for meetings that still collide with a break).
    fn deliver_reminders(&mut self, now_ms: i64) {
        let due = self.choices.take_due(now_ms);
        if due.is_empty() {
            return;
        }
        self.choices_dirty = true;
        let breaks = self.breaks(now_ms);
        let all_read = self.all_sources_read();
        for id in due {
            let Some(meeting) = self.meetings.iter().find(|meeting| meeting.id == id) else {
                if all_read {
                    log::info!(
                        "calendar: reminder skipped, the meeting is no longer in the calendar"
                    );
                } else {
                    // Right after a start (or while a source has not answered yet) the meeting may simply not be
                    // read yet: ask again a little later instead of dropping the reminder.
                    self.choices
                        .retry_reminder(&id, now_ms.saturating_add(REMINDER_RETRY_MS));
                }
                continue;
            };
            let Some(window) = breaks
                .iter()
                .find(|window| window.overlaps(meeting.start_ms, meeting.end_ms))
            else {
                log::info!(
                    "calendar: reminder skipped, the meeting no longer collides with a break"
                );
                continue;
            };
            let conflict = model::conflict_view(meeting, *window, &self.choices, now_ms);
            log::info!("calendar: showing a meeting reminder");
            self.ctx.popup.show(reminder_popup(
                conflict,
                self.plan.profile.break_secs,
                now_ms,
            ));
        }
    }

    fn breaks(&self, now_ms: i64) -> Vec<focus_core::calendar::BreakWindow> {
        let elapsed = u64::try_from(self.plan_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.plan.breaks(now_ms, elapsed, self.config.horizon_ms())
    }

    fn save_choices(&mut self) {
        if !std::mem::take(&mut self.choices_dirty) {
            return;
        }
        if let Err(err) = self
            .ctx
            .persistence
            .save_meeting_choices(&self.choices.to_stored())
        {
            log::warn!("calendar: cannot save the meeting answers: {err:#}");
        }
    }

    /// Sends the `Extend` / `PauseAtBreak` meetings to the session actor when they changed.
    fn send_holds(&mut self) {
        let holds = self.choices.holds(&self.meetings);
        if self.sent_holds.as_ref() == Some(&holds) {
            return;
        }
        match self.ctx.actor.send(ActorCmd::SetMeetings(holds.clone())) {
            Ok(()) => self.sent_holds = Some(holds),
            Err(err) => log::warn!("calendar: cannot update the meeting holds: {}", err.message),
        }
    }

    /// Emits `calendar://changed` when anything but the revision changed.
    fn publish(&mut self, now_ms: i64) {
        let breaks = self.breaks(now_ms);
        let mut status = CalendarStatus {
            revision: self.revision,
            sources: self.source_statuses(),
            meetings: self.meetings.clone(),
            conflicts: model::conflict_views(&breaks, &self.meetings, &self.choices, now_ms),
            planned: self.plan.planned(),
            last_sync_ms: self.last_sync_ms,
            google_pending: self.google.pending,
        };
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
        *lock(&self.shared) = status.clone();
        if let Err(err) = self.ctx.app.emit(events::CALENDAR_CHANGED, status.clone()) {
            log::warn!("cannot emit calendar status: {err}");
        }
        self.last_published = Some(status);
    }
}

/// The Microsoft source must start over: the Teams connection changed, or a sign-in ended while connected
/// (a new sign-in, maybe with `Calendars.Read` now). Arguments: `(connected, signing in)` before and now.
fn microsoft_signin_changed(before: (bool, bool), now: (bool, bool)) -> bool {
    let ((was_connected, was_signing_in), (connected, signing_in)) = (before, now);
    connected != was_connected || (connected && was_signing_in && !signing_in)
}

fn source_index(source: CalendarSource) -> usize {
    match source {
        CalendarSource::Microsoft => 0,
        CalendarSource::Google => 1,
        CalendarSource::Ics => 2,
    }
}

/// The popup (and its toast fallback) for a due "remind later".
fn reminder_popup(conflict: MeetingConflict, break_secs: u32, now_ms: i64) -> PopupRequest {
    let meeting = &conflict.meeting;
    let start_local = texts::local_clock(
        meeting.start_ms,
        now_ms,
        focus_platform::clock::local_minutes_of_day(),
    );
    // Rounded up: a meeting 29 min 30 s away starts "in 30 min".
    let minutes_until = meeting
        .start_ms
        .saturating_sub(now_ms)
        .max(0)
        .saturating_add(59_999)
        / 60_000;
    let fallback = texts::meeting_reminder_toast(
        &meeting.subject,
        start_local.as_deref(),
        minutes_until,
        conflict.break_number,
    );
    PopupRequest {
        meeting: Some(conflict.clone()),
        ..PopupRequest::new(
            PopupKind::MeetingReminder,
            conflict.break_number,
            break_secs,
            fallback,
        )
    }
}

/// Wall clock in Unix milliseconds.
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminder_popup_describes_the_meeting() {
        let now = 1_791_885_600_000;
        let conflict = MeetingConflict {
            meeting: Meeting {
                id: "ms:x:1".into(),
                source: CalendarSource::Microsoft,
                subject: "Przegląd".into(),
                start_ms: now + 30 * 60_000,
                end_ms: now + 90 * 60_000,
                online: true,
            },
            break_number: 2,
            break_start_ms: now + 45 * 60_000,
            break_end_ms: now + 60 * 60_000,
            choice: Some(MeetingChoice::RemindLater),
            next_reminder_ms: None,
            remind_later_allowed: true,
            too_long: false,
        };
        let popup = reminder_popup(conflict.clone(), 900, now);
        assert_eq!(popup.kind, PopupKind::MeetingReminder);
        assert_eq!((popup.cycle, popup.phase_secs), (2, 900));
        assert_eq!(popup.meeting, Some(conflict));
        // In the UI language in use (English in tests): subject, how soon and which break.
        assert!(popup.fallback.body.contains("Przegląd"));
        assert!(popup.fallback.body.contains("30 min"));
        assert!(popup.fallback.body.contains(" 2."));
    }

    #[test]
    fn microsoft_follows_connections_and_finished_sign_ins() {
        // (connected, signing in) before → now
        assert!(
            microsoft_signin_changed((false, true), (true, false)),
            "first sign-in"
        );
        assert!(
            microsoft_signin_changed((true, false), (false, false)),
            "signed out"
        );
        assert!(
            microsoft_signin_changed((true, true), (true, false)),
            "signed in again while connected (\"Połącz ponownie\")"
        );
        assert!(
            !microsoft_signin_changed((true, false), (true, true)),
            "sign-in started"
        );
        assert!(!microsoft_signin_changed((true, false), (true, false)));
        assert!(
            !microsoft_signin_changed((false, true), (false, false)),
            "sign-in failed"
        );
        assert!(!microsoft_signin_changed((false, false), (false, false)));
    }

    #[test]
    fn sources_have_fixed_slots() {
        let indexes: Vec<usize> = SOURCES.iter().map(|&source| source_index(source)).collect();
        assert_eq!(indexes, [0, 1, 2]);
        let mut state = SourceState {
            meetings: Vec::new(),
            error: Some("x".into()),
            synced: true,
            epoch: 7,
        };
        state.reset();
        assert_eq!((state.error, state.synced, state.epoch), (None, false, 8));
    }
}
