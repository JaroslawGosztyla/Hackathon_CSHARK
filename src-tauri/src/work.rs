//! "Co dalej?": work items from the developer tools (GitHub, GitLab, Jira, Azure DevOps, Linear), read-only.
//!
//! One thread ("focus-work") owns the state; every fetch runs on its own short-lived thread
//! ([`fetch`], blocking HTTP in `focus_platform::work`), so commands are answered at once and the session
//! actor is never blocked:
//! * every enabled source is fetched every [`model::POLL_EVERY`] (5 min); a source that sent a poll hint
//!   (GitHub `X-Poll-Interval`) is never fetched more often than that, also on a manual refresh (GitHub's
//!   `Last-Modified` / `If-Modified-Since` is kept by its client between fetches). A failed fetch keeps the
//!   previous items and is retried with an exponential backoff (1, 2, 4 … up to 30 min);
//! * fetches start at once on `refresh_work`, a token change, a change of the source's settings and when a
//!   session starts;
//! * the items of all sources are merged and ranked by `focus_core::work::rank` (assigned to the user first,
//!   then waiting on the user, then other notifications) and published as `work://changed` (own revision)
//!   whenever anything but the revision changed;
//! * `unseen` marks the items that arrived since the user last looked at the list (`get_work` /
//!   `refresh_work`: the items returned then count as seen from the next status on). `work-seen.json` keeps
//!   their ids only;
//! * new items are announced with a `WorkDigest` popup (toast fallback). While focus runs and
//!   `holdDuringFocus` is on they are held instead (`held_count`) and announced together when the break
//!   starts or the session stops, never during focus ([`model::Announcer`]). The first list of a source after
//!   the start or a (re)configuration is its baseline and announces nothing.
//!
//! Tokens are sealed with DPAPI (`work-<tool>-token.bin`), never in `config.json`, never logged, and never
//! leave the fetch threads. Titles, keys and links of items are never logged either (only counts). Nothing is
//! sent anywhere but to the tools the user connected.

mod fetch;
mod model;

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{
    AppError, Phase, PopupKind, WorkConfig, WorkItem, WorkSource, WorkStatus, events,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

use crate::persistence::Persistence;
use crate::popup::{PopupHandle, PopupRequest};
use crate::{SharedConfig, emit_error, lock, read_config, texts};

pub use fetch::validate_token;
use model::{Announcer, IdSet, Poll, Reported, SOURCES, SourceSettings, StoredSeen, source_name};

/// Items are re-ranked this often (age and due dates move with the clock).
const RECOMPUTE_EVERY: Duration = Duration::from_secs(5 * 60);
/// A fetch that did not answer within this time is given up (its late result is dropped).
const FETCH_GIVE_UP: Duration = Duration::from_secs(3 * 60);
/// `refresh_work` waits at most this long for the fetches, then returns the current status.
const REFRESH_WAIT: Duration = Duration::from_secs(60);

pub enum WorkCmd {
    /// The status for `get_work`; its items count as seen afterwards.
    Get(oneshot::Sender<WorkStatus>),
    /// Fetch now (`refresh_work`); the reply gets the status once the fetches ended.
    Refresh(oneshot::Sender<WorkStatus>),
    /// An already validated token ([`validate_token`]) to store, or `None` to forget it.
    SetToken {
        source: WorkSource,
        token: Option<String>,
        reply: oneshot::Sender<Result<WorkStatus, AppError>>,
    },
    ConfigChanged,
    /// The session phase changed (from the actor).
    Session(Phase),
    /// A fetch finished (from its fetch thread).
    Fetched(fetch::Fetched),
}

/// Cheap to clone. The async methods wait for the work thread, never block a thread.
#[derive(Clone)]
pub struct WorkHandle {
    tx: Sender<WorkCmd>,
    status: Arc<Mutex<WorkStatus>>,
}

impl WorkHandle {
    pub fn channel() -> (Self, Receiver<WorkCmd>) {
        let (tx, rx) = mpsc::channel();
        let status = WorkStatus {
            revision: 0,
            sources: Vec::new(),
            items: Vec::new(),
            held_count: 0,
            last_sync_ms: None,
        };
        (
            Self {
                tx,
                status: Arc::new(Mutex::new(status)),
            },
            rx,
        )
    }

    /// The last published status (`get_work`); the user has now seen its items.
    pub async fn get(&self) -> Result<WorkStatus, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(WorkCmd::Get(reply))?;
        response.await.map_err(|_| unavailable_error())
    }

    /// Fetches every connected tool now and returns the status after the fetches (or the current one after
    /// [`REFRESH_WAIT`]).
    pub async fn refresh(&self) -> Result<WorkStatus, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(WorkCmd::Refresh(reply))?;
        match tokio::time::timeout(REFRESH_WAIT, response).await {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(_)) => Err(unavailable_error()),
            Err(_) => Ok(lock(&self.status).clone()),
        }
    }

    /// Stores (sealed) or forgets the token of `source`; the source is fetched again at once.
    pub async fn set_token(
        &self,
        source: WorkSource,
        token: Option<String>,
    ) -> Result<WorkStatus, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(WorkCmd::SetToken {
            source,
            token,
            reply,
        })?;
        response.await.map_err(|_| unavailable_error())?
    }

    /// The config was saved (non-blocking).
    pub fn config_changed(&self) {
        let _ = self.send(WorkCmd::ConfigChanged);
    }

    /// The session phase changed (non-blocking; from the actor).
    pub fn session_changed(&self, phase: Phase) {
        let _ = self.send(WorkCmd::Session(phase));
    }

    fn send(&self, cmd: WorkCmd) -> Result<(), AppError> {
        self.tx.send(cmd).map_err(|_| {
            log::warn!("work thread is not running");
            unavailable_error()
        })
    }
}

fn unavailable_error() -> AppError {
    AppError::new("work.unavailable", "work items thread is not running")
}

pub struct WorkContext {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub persistence: Persistence,
    pub popup: PopupHandle,
}

/// Starts the work thread.
pub fn spawn(ctx: WorkContext, handle: &WorkHandle, rx: Receiver<WorkCmd>) -> anyhow::Result<()> {
    let shared = Arc::clone(&handle.status);
    let tx = handle.tx.clone();
    std::thread::Builder::new()
        .name("focus-work".into())
        .spawn(move || {
            // Reads the seen ids and looks for the sealed tokens here, off the main thread.
            let mut service = Service::new(ctx, shared, tx);
            service.rebuild();
            service.run(rx);
        })?;
    Ok(())
}

/// What one source contributed and when it is fetched next.
struct SourceState {
    settings: SourceSettings,
    /// A sealed token file exists.
    has_token: bool,
    /// Items of the last successful fetch.
    items: Vec<WorkItem>,
    account: Option<String>,
    /// Redacted English error of the last fetch.
    error: Option<String>,
    /// Redacted warning of the last successful fetch (an optional part failed).
    warning: Option<String>,
    /// A fetch succeeded since the source was (re)configured (its first list is the baseline).
    synced: bool,
    /// Bumped on every (re)configuration: older results are dropped and the client is rebuilt.
    epoch: u64,
    poll: Poll,
    /// Id of the fetch in flight and when it started.
    in_flight: Option<(u64, Instant)>,
}

impl SourceState {
    fn new(settings: SourceSettings, has_token: bool, now: Instant) -> Self {
        Self {
            settings,
            has_token,
            items: Vec::new(),
            account: None,
            error: None,
            warning: None,
            synced: false,
            epoch: 0,
            poll: Poll::new(now),
            in_flight: None,
        }
    }

    /// Starts over: no items, no error, a new baseline, due now (the client is rebuilt).
    fn reset(&mut self, now: Instant) {
        self.items.clear();
        self.account = None;
        self.error = None;
        self.warning = None;
        self.synced = false;
        self.epoch = self.epoch.wrapping_add(1);
        self.poll = Poll::new(now);
        self.in_flight = None;
    }
}

struct Service {
    ctx: WorkContext,
    shared: Arc<Mutex<WorkStatus>>,
    tx: Sender<WorkCmd>,
    clients: fetch::Clients,
    /// The work config the sources were set up with.
    config: WorkConfig,
    sources: [SourceState; SOURCES.len()],
    /// Ids the user has seen in the list (`work-seen.json`).
    seen: IdSet,
    announcer: Announcer,
    phase: Phase,
    waiters: Vec<oneshot::Sender<WorkStatus>>,
    next_fetch_id: u64,
    last_sync_ms: Option<i64>,
    next_recompute: Instant,
    revision: u32,
    last_published: Option<WorkStatus>,
}

impl Service {
    fn new(ctx: WorkContext, shared: Arc<Mutex<WorkStatus>>, tx: Sender<WorkCmd>) -> Self {
        let now = Instant::now();
        let config = read_config(&ctx.config).work;
        let sources = SOURCES.map(|source| {
            let has_token = fetch::has_token(&ctx.persistence.work_token_path(source));
            SourceState::new(model::settings(&config, source), has_token, now)
        });
        let seen = ctx
            .persistence
            .load_work_seen::<StoredSeen>()
            .map(|stored| IdSet::from_ids(stored.seen, model::MAX_SEEN))
            .unwrap_or_else(|| IdSet::new(model::MAX_SEEN));
        let tokens = sources.iter().filter(|state| state.has_token).count();
        log::info!("work: {tokens} stored tokens, {} seen items", seen.len());
        Self {
            ctx,
            shared,
            tx,
            clients: fetch::Clients::default(),
            config: config.clone(),
            sources,
            seen,
            announcer: Announcer::new(config.hold_during_focus),
            phase: Phase::Idle,
            waiters: Vec::new(),
            next_fetch_id: 0,
            last_sync_ms: None,
            next_recompute: now + RECOMPUTE_EVERY,
            revision: 0,
            last_published: None,
        }
    }

    fn run(&mut self, rx: Receiver<WorkCmd>) {
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
        let mut next = self.next_recompute;
        for (index, state) in self.sources.iter().enumerate() {
            match state.in_flight {
                Some((_, started)) => next = next.min(started + FETCH_GIVE_UP),
                None if self.usable(index) => next = next.min(state.poll.next_due()),
                None => {}
            }
        }
        next
    }

    fn on_wake(&mut self) {
        let now = Instant::now();
        let mut gave_up = false;
        for (index, &source) in SOURCES.iter().enumerate() {
            if let Some((_, started)) = self.sources[index].in_flight
                && now.duration_since(started) >= FETCH_GIVE_UP
            {
                log::warn!(
                    "work: {}: the fetch did not finish, giving up on it",
                    source_name(source)
                );
                self.failed(index, "the request did not finish in time".to_owned(), now);
                gave_up = true;
            }
            self.start_fetch(index, false);
        }
        if gave_up || now >= self.next_recompute {
            self.rebuild();
        }
        if gave_up {
            self.answer_waiters_when_idle();
        }
    }

    fn handle(&mut self, cmd: WorkCmd) {
        match cmd {
            WorkCmd::Get(reply) => {
                let _ = reply.send(lock(&self.shared).clone());
                self.mark_seen();
            }
            WorkCmd::Refresh(reply) => {
                self.mark_seen();
                self.waiters.push(reply);
                self.fetch_all(true);
                if !self.fetching() {
                    // Nothing to fetch (or every source waits for its poll hint): answer at once.
                    self.rebuild();
                    self.answer_waiters_when_idle();
                }
            }
            WorkCmd::SetToken {
                source,
                token,
                reply,
            } => {
                let result = self.set_token(source, token);
                let _ = reply.send(result);
            }
            WorkCmd::ConfigChanged => self.config_changed(),
            WorkCmd::Session(phase) => self.session_changed(phase),
            WorkCmd::Fetched(fetched) => self.fetched(fetched),
        }
    }

    // -- fetching ------------------------------------------------------------------------------------

    /// The source is enabled and fully set up.
    fn usable(&self, index: usize) -> bool {
        self.sources[index].settings.enabled && self.missing(index).is_none()
    }

    fn missing(&self, index: usize) -> model::Missing {
        let state = &self.sources[index];
        model::missing(
            SOURCES[index],
            &state.settings,
            state.has_token,
            fetch::SECRETS_SUPPORTED,
        )
    }

    fn fetching(&self) -> bool {
        self.sources.iter().any(|state| state.in_flight.is_some())
    }

    /// Starts every usable source that is not in flight (`forced`: see [`Poll::may_start`]).
    fn fetch_all(&mut self, forced: bool) {
        for index in 0..SOURCES.len() {
            self.start_fetch(index, forced);
        }
    }

    fn start_fetch(&mut self, index: usize, forced: bool) {
        if !self.usable(index) || self.sources[index].in_flight.is_some() {
            return;
        }
        let now = Instant::now();
        let source = SOURCES[index];
        let id = self.next_fetch_id;
        let token_path = self.ctx.persistence.work_token_path(source);
        let state = &mut self.sources[index];
        if !state.poll.may_start(now, forced) {
            return;
        }
        self.next_fetch_id = self.next_fetch_id.wrapping_add(1);
        let job = fetch::Job {
            id,
            source,
            epoch: state.epoch,
            settings: state.settings.clone(),
            config: self.config.clone(),
            token_path,
            now_ms: wall_ms(),
        };
        if self.clients.fetch(job, self.tx.clone()) {
            state.in_flight = Some((id, now));
            state.poll.started(now);
        } else {
            log::warn!(
                "work: {}: cannot start the fetch thread",
                source_name(source)
            );
            state.poll.not_started(now);
        }
    }

    fn fetched(&mut self, fetched: fetch::Fetched) {
        let index = model::source_index(fetched.source);
        let state = &self.sources[index];
        if state.epoch != fetched.epoch || state.in_flight.map(|(id, _)| id) != Some(fetched.id) {
            log::debug!("work: dropping the result of an abandoned fetch");
            return;
        }
        let now = Instant::now();
        let name = source_name(fetched.source);
        match fetched.outcome {
            Ok(outcome) => {
                let state = &mut self.sources[index];
                state.in_flight = None;
                state.error = None;
                if outcome.warning != state.warning
                    && let Some(warning) = &outcome.warning
                {
                    log::info!("work: {name}: {warning}");
                }
                state.warning = outcome.warning;
                state.poll.succeeded(now, outcome.poll_hint_secs);
                if outcome.account.is_some() {
                    state.account = outcome.account;
                }
                let baseline = !std::mem::replace(&mut state.synced, true);
                let arrivals = self.announcer.list(&outcome.items, baseline, &self.seen);
                log::info!(
                    "work: {name}: {} items, {} new{}",
                    outcome.items.len(),
                    arrivals.new,
                    if baseline { " (first list)" } else { "" }
                );
                state.items = outcome.items;
                self.last_sync_ms = Some(fetched.at_ms);
                self.show_digest(arrivals.announce);
            }
            Err(error) => {
                log::warn!("work: {name}: fetch failed: {error}");
                self.failed(index, error, now);
            }
        }
        self.rebuild();
        self.answer_waiters_when_idle();
    }

    /// Records a failed fetch (`error` already redacted) and schedules the retry.
    fn failed(&mut self, index: usize, error: String, now: Instant) {
        let state = &mut self.sources[index];
        state.in_flight = None;
        state.poll.failed(now);
        let first = state.error.is_none();
        state.error = Some(error.clone());
        // During a held focus the error only shows in the source status (the "Co dalej?" card at the break).
        if first && !self.announcer.holding() {
            emit_error(
                &self.ctx.app,
                AppError::new(
                    "work.sync",
                    format!("cannot read {}: {error}", source_name(SOURCES[index])),
                ),
            );
        }
    }

    fn answer_waiters_when_idle(&mut self) {
        if self.waiters.is_empty() || self.fetching() {
            return;
        }
        let status = lock(&self.shared).clone();
        for waiter in self.waiters.drain(..) {
            let _ = waiter.send(status.clone());
        }
    }

    // -- announcing ----------------------------------------------------------------------------------

    /// A `WorkDigest` popup (toast fallback) for `count` new items; nothing for 0.
    fn show_digest(&self, count: u32) {
        if count == 0 {
            return;
        }
        log::info!("work: announcing {count} new items");
        self.ctx.popup.show(PopupRequest {
            work_count: count,
            ..PopupRequest::new(PopupKind::WorkDigest, 0, 0, texts::work_digest_toast(count))
        });
    }

    /// The items of the last status count as seen from the next status on.
    fn mark_seen(&mut self) {
        let ids: Vec<String> = lock(&self.shared)
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect();
        if ids.is_empty() {
            return;
        }
        for id in &ids {
            self.seen.touch(id);
        }
        let stored = StoredSeen {
            seen: self.seen.ids(),
        };
        if let Err(err) = self.ctx.persistence.save_work_seen(&stored) {
            log::warn!("work: cannot save the seen items: {err:#}");
        }
    }

    // -- commands ------------------------------------------------------------------------------------

    fn session_changed(&mut self, phase: Phase) {
        let previous = std::mem::replace(&mut self.phase, phase);
        if previous == phase {
            return;
        }
        if previous == Phase::Idle && phase == Phase::Focus {
            // A session starts: the freshest list for its goal; what arrives from now on is held.
            self.fetch_all(true);
        }
        let digest = self.announcer.phase(phase, &self.seen);
        self.show_digest(digest);
        if self.announcer.holding() {
            // A digest from before (still on screen, or parked behind another popup) must not show during focus.
            self.ctx.popup.withdraw_work_digest();
        }
        self.rebuild();
    }

    fn config_changed(&mut self) {
        let config: WorkConfig = read_config(&self.ctx.config).work;
        // Before the fetches below: their providers are built from it.
        self.config = config.clone();
        let now = Instant::now();
        for (index, &source) in SOURCES.iter().enumerate() {
            let settings = model::settings(&config, source);
            if settings == self.sources[index].settings {
                continue;
            }
            let state = &mut self.sources[index];
            log::info!(
                "work: {} settings changed ({})",
                source_name(source),
                if settings.enabled { "on" } else { "off" }
            );
            state.settings = settings;
            state.reset(now);
            self.start_fetch(index, true);
        }
        let digest = self
            .announcer
            .set_hold(config.hold_during_focus, &self.seen);
        self.show_digest(digest);
        if self.announcer.holding() {
            // Holding turned on during focus: a digest still waiting for the screen goes away too.
            self.ctx.popup.withdraw_work_digest();
        }
        self.rebuild();
    }

    fn set_token(
        &mut self,
        source: WorkSource,
        token: Option<String>,
    ) -> Result<WorkStatus, AppError> {
        let path = self.ctx.persistence.work_token_path(source);
        let name = source_name(source);
        match token {
            Some(token) => {
                fetch::store_token(&path, &token)?;
                log::info!("work: {name} token stored");
            }
            None => {
                fetch::delete_token(&path)?;
                log::info!("work: {name} token deleted");
            }
        }
        let index = model::source_index(source);
        let state = &mut self.sources[index];
        state.has_token = fetch::has_token(&path);
        state.reset(Instant::now());
        self.start_fetch(index, true);
        self.rebuild();
        Ok(lock(&self.shared).clone())
    }

    // -- status --------------------------------------------------------------------------------------

    /// Merges and ranks the items of the usable sources and publishes the status.
    fn rebuild(&mut self) {
        let now_ms = wall_ms();
        self.next_recompute = Instant::now() + RECOMPUTE_EVERY;
        let mut items: Vec<WorkItem> = Vec::new();
        for index in 0..SOURCES.len() {
            if self.usable(index) {
                items.extend(self.sources[index].items.iter().cloned());
            }
        }
        for item in &mut items {
            item.unseen = !self.seen.contains(&item.id);
        }
        let items = focus_core::work::rank(items, now_ms);
        let sources = SOURCES
            .iter()
            .enumerate()
            .map(|(index, &source)| {
                let state = &self.sources[index];
                model::source_status(
                    source,
                    state.settings.enabled,
                    self.missing(index),
                    Reported {
                        error: state.error.as_deref(),
                        warning: state.warning.as_deref(),
                        synced: state.synced,
                    },
                    state.account.clone(),
                )
            })
            .collect();
        let status = WorkStatus {
            revision: self.revision,
            sources,
            items,
            held_count: self.announcer.held_count(&self.seen),
            last_sync_ms: self.last_sync_ms,
        };
        self.publish(status);
    }

    /// Emits `work://changed` when anything but the revision changed.
    fn publish(&mut self, mut status: WorkStatus) {
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
        if let Err(err) = self.ctx.app.emit(events::WORK_CHANGED, status.clone()) {
            log::warn!("cannot emit work status: {err}");
        }
        self.last_published = Some(status);
    }
}

/// Wall clock in Unix milliseconds.
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
