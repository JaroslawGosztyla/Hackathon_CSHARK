//! Beta rewards service (ADR-022): the shell side of `focus_core::rewards`.
//!
//! * One [`RewardsHandle`] (a mutex around the service) is shared by the effects dispatcher (session facts), the commands
//!   (getter, ack, reset, goal answers, config changes) and the actor (reward-day rollover, shutdown flush). Facts are
//!   applied synchronously under the mutex (pure, microseconds), so the actor never waits on slow work (rule 3); only the
//!   file write is queued, to the coalescing writer thread ("focus-rewards").
//! * While the rewards are off (`!Config::rewards_active()`) facts are dropped and nothing is emitted; config changes still
//!   record pause ranges (for an existing streak only), which is the one write that can happen then.
//! * Demo sessions (`TimingProfile.demo`) go to a preview copy of the ledger (status `preview: true`): created at the demo
//!   Start, dropped at the next real Start, on reset and at exit, never written.
//! * Presentation showcase (Developer page "Presentation mode" / `--presentation`, ADR-019): the preview is a seeded
//!   showcase ledger (`crate::showcase::seeded_ledger`) kept across Starts until the showcase ends, and the view and
//!   the demo facts use the beta rules of `crate::showcase::beta_config` (in memory, whatever `betaFeatures` says).
//!   Real facts keep the real rules and the real ledger; nothing of the showcase is ever written.
//! * `rewards.json` (numbers, day numbers and session ids only) is written after every change of the real ledger: temp
//!   file + rename, the previous file copied to `rewards.json.bak` at most once per minute, flushed at shutdown. A
//!   corrupted file is replaced by its backup, or set aside as `rewards.unreadable-<ms>.json` (`rewards.load`, once);
//!   while the rewards are off that repair waits until they come on. A file that cannot be read at all (locked) is
//!   never replaced: nothing is written that run. A failed write reports `rewards.save` once per run (rule 8: the
//!   session goes on).
//! * Beta collection (ADR-024): the unlock records and seasonal goal days live in the same ledger (fixed ids, day
//!   numbers and years). The core unlocks on real facts only; at load with the rewards on, items whose rules already
//!   hold are recorded silently and saved. Switching the beta or the rewards off never removes them; "Zacznij od nowa"
//!   and the privacy delete start a fresh ledger (and the commands clear `Config.reward_theme`).
//! * Moments wait in `pending` (at most [`PENDING_MAX`], memory only) until the main window acknowledges them.
//! * Logs carry kinds and counts only, never texts (rule 11).

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::ErrorKind;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{
    AppError, Config, Phase, RewardMoment, RewardTheme, RewardsCheat, RewardsStatus, events,
};
use focus_core::rewards::{
    self, Ledger, LedgerMode, RewardDay, RewardRules, RewardsState, SessionTally,
};
use focus_core::session::ProgressFact;
use focus_core::timing::REWARD_DAY_START_MINUTE;
use focus_core::unlocks::{self, Unlockable};
use tauri::{AppHandle, Emitter};

use crate::persistence::Persistence;
use crate::{emit_error, emit_startup_error, lock};

/// Moments kept for the main window (the oldest go first).
pub const PENDING_MAX: usize = 8;
/// `AppError.code`: `rewards.json` could not be read; it was set aside and the ledger starts fresh.
pub const ERR_LOAD: &str = "rewards.load";
/// `AppError.code`: `rewards.json` could not be written (reported once per run).
pub const ERR_SAVE: &str = "rewards.save";
/// A developer cheat while the beta rewards are off.
pub const ERR_REWARDS_OFF: &str = "rewardsOff";
/// The previous `rewards.json` is copied to `rewards.json.bak` at most this often.
const BACKUP_EVERY: Duration = Duration::from_secs(60);
/// A `rewards.json` that cannot be read (locked, no access) is read again this many times at load, after this delay.
const READ_RETRIES: u32 = 2;
const READ_RETRY_DELAY: Duration = Duration::from_millis(200);
/// Longest wait for the last write at shutdown.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(2);
/// Longest wait for the files to go (privacy panel).
const RESET_TIMEOUT: Duration = Duration::from_secs(5);
const MS_PER_DAY: i64 = 86_400_000;
const MS_PER_MINUTE: i64 = 60_000;

/// Where the service reports its status: the WebView (`rewards://changed`) or a test recorder.
trait Outlet: Send {
    fn status(&self, status: &RewardsStatus);
}

struct AppOutlet(AppHandle);

impl Outlet for AppOutlet {
    fn status(&self, status: &RewardsStatus) {
        if let Err(err) = self.0.emit(events::REWARDS_CHANGED, status.clone()) {
            log::warn!("cannot emit the rewards status: {err}");
        }
    }
}

/// The reward day and the wall clock (Unix ms) now.
type Clock = Box<dyn Fn() -> (RewardDay, i64) + Send>;

/// Reports an error to the user (`app://error`), or to a test sink.
type Report = Arc<dyn Fn(AppError) + Send + Sync>;

struct Service {
    real: RewardsState,
    /// The demo preview (see the module docs).
    preview: Option<RewardsState>,
    /// Moments the main window has not acknowledged (oldest first).
    pending: Vec<RewardMoment>,
    revision: u32,
    /// Ids of the moments of this run (the real ledger and the preview share them).
    next_id: u32,
    /// The reward day of the last status (rollover check).
    last_day: RewardDay,
    /// A broken `rewards.json` found at load while the rewards were off: repaired when they come on (nothing is
    /// written while they are off).
    repair: Option<Repair>,
    /// The load problem not reported yet (the rewards were off then).
    unreported: Option<AppError>,
    /// `rewards.json` could not be read at all (locked, no access): it is never overwritten this run.
    read_only: bool,
    /// The presentation showcase is on (see the module docs): `preview` holds the seeded showcase ledger.
    showcase: bool,
    /// The latest demo session that ran while the showcase was on: shown only, never stored, also when it ends after
    /// the showcase was switched off ([`RewardsHandle::showcase_session`]). An id only, memory only.
    showcase_session: Option<String>,
    writer: Writer,
    outlet: Box<dyn Outlet>,
    clock: Clock,
    report: Report,
}

impl Service {
    /// Demo session `session_id` ran while the showcase was on (see `showcase_session`).
    fn mark_showcase_session(&mut self, session_id: &str) {
        if self.showcase_session.as_deref() != Some(session_id) {
            self.showcase_session = Some(session_id.to_owned());
        }
    }

    /// The rules of what the view shows (and of the demo facts): the showcase's beta rules while the presentation
    /// showcase is on, else the config's.
    fn shown_rules(&self, cfg: &Config) -> RewardRules {
        if self.showcase {
            RewardRules::from_config(&crate::showcase::beta_config(cfg))
        } else {
            RewardRules::from_config(cfg)
        }
    }

    fn status(&self, rules: &RewardRules, today: RewardDay) -> RewardsStatus {
        let shown = self.preview.as_ref().unwrap_or(&self.real);
        let mut status = rewards::view(shown, today, rules);
        status.revision = self.revision;
        status.preview = self.preview.is_some();
        status.pending = if rules.active {
            self.pending.clone()
        } else {
            Vec::new()
        };
        status
    }

    /// Emits `rewards://changed` with a new revision.
    fn publish(&mut self, rules: &RewardRules, today: RewardDay) {
        self.revision = self.revision.wrapping_add(1);
        self.last_day = today;
        let status = self.status(rules, today);
        self.outlet.status(&status);
    }

    /// Numbers the moment for this run and keeps it for the main window.
    fn push(&mut self, mut moment: RewardMoment) -> RewardMoment {
        moment.id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.pending.push(moment.clone());
        if self.pending.len() > PENDING_MAX {
            let extra = self.pending.len() - PENDING_MAX;
            self.pending.drain(..extra);
        }
        moment
    }

    fn save_if_changed(&self, before: &Ledger) {
        if self.real.ledger != *before && !self.read_only {
            self.writer.save(self.real.ledger.clone());
        }
    }

    /// The rewards are on: a repair left from the load goes to the writer (before any save), and a load problem not
    /// reported yet is reported now.
    fn repair_now(&mut self) {
        match self.repair.take() {
            Some(Repair::Restore) => self.writer.restore(self.real.ledger.clone()),
            Some(Repair::SetAside) => {
                let (_, now_ms) = (self.clock)();
                self.writer.set_aside(now_ms);
            }
            None => {}
        }
        if let Some(error) = self.unreported.take() {
            (self.report)(error);
        }
    }

    /// ADR-024, at load with the rewards on: collection items whose rules already hold (an update from a version
    /// without the collection) are recorded silently (no moment) and saved. Logs the count only.
    fn catch_up_unlocks(&mut self, today: RewardDay) {
        let before = self.real.ledger.clone();
        let count = rewards::catch_up(&mut self.real, today);
        if count > 0 {
            log::info!("rewards: {count} collection items unlocked at load");
            self.save_if_changed(&before);
        }
    }
}

/// What to do with a broken `rewards.json` found at load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repair {
    /// The backup was good: write the restored ledger over the broken file (no backup copy of the broken one).
    Restore,
    /// The backup was missing or broken too: set the file aside as `rewards.unreadable-<ms>.json`.
    SetAside,
}

/// The outcome of reading `rewards.json` (nothing is written by the read itself).
struct Loaded {
    ledger: Ledger,
    repair: Option<Repair>,
    /// `rewards.load`, to report while the rewards are on.
    problem: Option<AppError>,
    read_only: bool,
}

impl Loaded {
    fn fine(ledger: Ledger) -> Self {
        Self {
            ledger,
            repair: None,
            problem: None,
            read_only: false,
        }
    }
}

/// The beta rewards service (see the module docs). Cheap to clone.
#[derive(Clone)]
pub struct RewardsHandle {
    inner: Arc<Mutex<Service>>,
}

impl RewardsHandle {
    /// Loads `rewards.json` (recovering from `.bak` or setting a corrupted file aside) and starts the writer thread.
    /// While the rewards are off nothing is written or renamed and a load problem is only logged: the repair and the
    /// report (`rewards.load`) wait until the rewards come on. The session is never affected.
    pub fn load(app: &AppHandle, persistence: &Persistence, config: &Config) -> Self {
        let error_app = app.clone();
        let report: Report = Arc::new(move |error| emit_error(&error_app, error));
        let writer = Writer::spawn(persistence.clone(), Arc::clone(&report));
        let mut loaded = load_ledger(persistence);
        let active = config.rewards_active();
        if active && let Some(error) = loaded.problem.take() {
            emit_startup_error(app, error);
        }
        Self::with_parts(
            loaded,
            active,
            writer,
            Box::new(AppOutlet(app.clone())),
            Box::new(system_clock),
            report,
        )
    }

    fn with_parts(
        loaded: Loaded,
        active: bool,
        writer: Writer,
        outlet: Box<dyn Outlet>,
        clock: Clock,
        report: Report,
    ) -> Self {
        let (today, _) = clock();
        if let Some(error) = &loaded.problem {
            log::warn!("{}: {}", error.code, error.message);
        }
        let mut service = Service {
            real: RewardsState::new(loaded.ledger),
            preview: None,
            pending: Vec::new(),
            revision: 0,
            next_id: 1,
            last_day: today,
            repair: loaded.repair,
            unreported: loaded.problem,
            read_only: loaded.read_only,
            showcase: false,
            showcase_session: None,
            writer,
            outlet,
            clock,
            report,
        };
        if active {
            service.repair_now();
            service.catch_up_unlocks(today);
        }
        Self {
            inner: Arc::new(Mutex::new(service)),
        }
    }

    /// ADR-024: `theme` is unlocked in the real ledger (never the demo preview or the showcase). `update_config`
    /// accepts a new `rewardTheme` only then.
    pub fn is_theme_unlocked(&self, theme: RewardTheme) -> bool {
        unlocks::is_unlocked(&lock(&self.inner).real.ledger, Unlockable::Theme(theme))
    }

    /// Applies a session fact (`Effect::Progress`) and returns its moment, for the popup line. Dropped while the
    /// rewards are off. A demo fact goes to the preview (created at the demo Start); a real Start drops the preview.
    /// While the presentation showcase is on, the preview is the showcase ledger (kept at every Start) and demo facts
    /// use the showcase rules; real facts always use the real rules.
    pub fn apply_fact(&self, fact: &ProgressFact, cfg: &Config) -> Option<RewardMoment> {
        let mut guard = lock(&self.inner);
        let svc = &mut *guard;
        let demo = fact.profile().demo;
        if demo && svc.showcase {
            svc.mark_showcase_session(fact.session_id());
        }
        let rules = if demo {
            svc.shown_rules(cfg)
        } else {
            RewardRules::from_config(cfg)
        };
        if !rules.active {
            return None;
        }
        let (today, now_ms) = (svc.clock)();
        if matches!(fact, ProgressFact::Started { .. }) && !svc.showcase {
            svc.preview = demo.then(|| svc.real.clone());
        }
        let before = svc.real.ledger.clone();
        let moment = if demo {
            let showcase = svc.showcase;
            let preview = svc.preview.get_or_insert_with(|| svc.real.clone());
            let mut moment =
                rewards::apply(preview, today, &rules, LedgerMode::Preview, fact, now_ms);
            if showcase && let Some(moment) = moment.as_mut() {
                // The showcase ledger's own collection unlocks (the core never unlocks in a preview).
                crate::showcase::apply_showcase_unlocks(preview, today, moment);
            }
            moment
        } else {
            rewards::apply(&mut svc.real, today, &rules, LedgerMode::Real, fact, now_ms)
        };
        let moment = moment.map(|moment| svc.push(moment));
        if let Some(moment) = &moment {
            log::info!(
                "rewards: {:?} +{} ({}){}",
                moment.kind,
                moment.sparks,
                kinds(moment),
                if demo { ", demo preview" } else { "" }
            );
        }
        svc.save_if_changed(&before);
        let shown = svc.shown_rules(cfg);
        svc.publish(&shown, today);
        moment
    }

    /// "Zrobione? → Tak" for the latest session of this run: +10 once per session. `phase` = now. A demo session's
    /// answer goes to the demo preview while it exists (never written). Returns the sparks the real ledger credited
    /// (0 for a demo session or an answer already paid), for the stored summary.
    pub fn goal_done(&self, session_id: &str, cfg: &Config, phase: Phase, demo: bool) -> u32 {
        let mut guard = lock(&self.inner);
        let svc = &mut *guard;
        let rules = if demo {
            svc.shown_rules(cfg)
        } else {
            RewardRules::from_config(cfg)
        };
        if !rules.active {
            return 0;
        }
        let (today, now_ms) = (svc.clock)();
        let before = svc.real.ledger.clone();
        let moment = if demo {
            let Some(preview) = svc.preview.as_mut() else {
                return 0;
            };
            rewards::goal_done(
                preview,
                today,
                &rules,
                LedgerMode::Preview,
                session_id,
                phase,
                now_ms,
            )
        } else {
            rewards::goal_done(
                &mut svc.real,
                today,
                &rules,
                LedgerMode::Real,
                session_id,
                phase,
                now_ms,
            )
        };
        let paid = match moment {
            Some(moment) => {
                log::info!(
                    "rewards: session goal done +{}{}",
                    moment.sparks,
                    if demo { ", demo preview" } else { "" }
                );
                let sparks = moment.sparks;
                svc.push(moment);
                if demo { 0 } else { sparks }
            }
            None => 0,
        };
        svc.save_if_changed(&before);
        let shown = svc.shown_rules(cfg);
        svc.publish(&shown, today);
        paid
    }

    /// The config was saved (`store_config`): pause bookkeeping, a lower daily goal, the UI hides or shows the rewards.
    pub fn config_changed(&self, old: &Config, new: &Config, phase: Phase) {
        let (old_rules, new_rules) = (RewardRules::from_config(old), RewardRules::from_config(new));
        let mut guard = lock(&self.inner);
        let svc = &mut *guard;
        if new_rules.active {
            svc.repair_now();
        }
        let (today, now_ms) = (svc.clock)();
        let before = svc.real.ledger.clone();
        let moment =
            rewards::config_changed(&mut svc.real, today, &old_rules, &new_rules, phase, now_ms);
        if let Some(moment) = moment {
            log::info!(
                "rewards: config change met the daily goal +{}",
                moment.sparks
            );
            svc.push(moment);
        }
        // The presentation showcase keeps its beta rules whatever the real config says.
        let shown = svc.shown_rules(new);
        if !shown.active {
            // A celebration of a switched-off feature is never shown later.
            svc.pending.clear();
        }
        let changed = svc.real.ledger != before;
        svc.save_if_changed(&before);
        if (old_rules.active || new_rules.active) && (old_rules != new_rules || changed) {
            svc.publish(&shown, today);
        }
    }

    /// The view state (`get_rewards`), with the current revision.
    pub fn status(&self, cfg: &Config) -> RewardsStatus {
        let svc = lock(&self.inner);
        let rules = svc.shown_rules(cfg);
        let (today, _) = (svc.clock)();
        svc.status(&rules, today)
    }

    /// The main window played or showed the moments up to `up_to`.
    pub fn ack(&self, up_to: u32, cfg: &Config) {
        let mut svc = lock(&self.inner);
        let count = svc.pending.len();
        svc.pending.retain(|moment| moment.id > up_to);
        if svc.pending.len() != count {
            let rules = svc.shown_rules(cfg);
            let (today, _) = (svc.clock)();
            svc.publish(&rules, today);
        }
    }

    /// "Zacznij od nowa" (`reset_rewards`): a fresh ledger with a new seed, no preview, no pending moments; the files go
    /// and, while the rewards are on, a fresh `rewards.json` is written.
    pub fn reset(&self, cfg: &Config) -> RewardsStatus {
        let rules = RewardRules::from_config(cfg);
        let mut svc = lock(&self.inner);
        let fresh = Self::clear(&mut svc);
        svc.writer.reset(rules.active.then_some(fresh), None);
        let shown = svc.shown_rules(cfg);
        let (today, _) = (svc.clock)();
        svc.publish(&shown, today);
        log::info!("rewards: started again at the user's request");
        svc.status(&shown, today)
    }

    /// Privacy panel (ADR-020): like [`Self::reset`], but no fresh file is written (the item disappears until the next
    /// spark), and the call waits until the files are gone. Blocking.
    pub fn delete_files(&self, cfg: &Config) -> anyhow::Result<()> {
        let rules = RewardRules::from_config(cfg);
        let (done_tx, done_rx) = mpsc::channel();
        {
            let mut svc = lock(&self.inner);
            Self::clear(&mut svc);
            svc.writer.reset(None, Some(done_tx));
            let shown = svc.shown_rules(cfg);
            if rules.active || shown.active {
                let (today, _) = (svc.clock)();
                svc.publish(&shown, today);
            }
        }
        match done_rx.recv_timeout(RESET_TIMEOUT) {
            Ok(result) => result.map_err(anyhow::Error::msg),
            Err(_) => anyhow::bail!("the rewards files were not deleted in time"),
        }
    }

    /// Resets the state in memory; returns the fresh ledger. A session running now counts from now on (an adoptable
    /// tally; the next Start replaces it). The files are replaced, so a pending repair or a read-only load is over.
    fn clear(svc: &mut Service) -> Ledger {
        let ledger = Ledger::new(new_seed());
        let (_, now_ms) = (svc.clock)();
        svc.real = RewardsState::new(ledger.clone());
        svc.real.session = Some(SessionTally::joined(now_ms));
        // The presentation showcase is memory only and not part of the real ledger: it stays until it ends.
        if !svc.showcase {
            svc.preview = None;
        }
        svc.pending.clear();
        svc.repair = None;
        svc.unreported = None;
        svc.read_only = false;
        ledger
    }

    /// Sparks of real session `session_id` (`SessionSummary.sparks`, stored in the history). Never the demo preview:
    /// nothing from a demo session is stored (its summary card reads the preview from the status).
    pub fn session_sparks(&self, session_id: &str) -> Option<u32> {
        let svc = lock(&self.inner);
        svc.real
            .session
            .as_ref()
            .filter(|tally| tally.session_id == session_id && !tally.demo)
            .map(|tally| tally.sparks)
    }

    /// Every minute from the actor: a new reward day refreshes the view (week strip, today) while the rewards are on.
    pub fn tick_day(&self, cfg: &Config) {
        let mut svc = lock(&self.inner);
        let rules = svc.shown_rules(cfg);
        let (today, _) = (svc.clock)();
        if today == svc.last_day {
            return;
        }
        svc.last_day = today;
        if rules.active {
            log::info!("rewards: a new reward day");
            svc.publish(&rules, today);
        }
    }

    /// Developer page "Presentation mode" / `--presentation` (ADR-019): `on` → the view shows a seeded showcase
    /// ledger for today (`crate::showcase::seeded_ledger`) as the demo preview, kept across Starts, with the showcase's
    /// beta rules; `off` → the preview and its moments go and the real ledger shows again. Already in that state: no-op
    /// (a running showcase keeps its progress). Nothing is written; `rewards.json` and the real ledger are untouched.
    pub fn set_showcase(&self, on: bool, cfg: &Config) {
        let mut guard = lock(&self.inner);
        let svc = &mut *guard;
        if svc.showcase == on {
            return;
        }
        let (today, now_ms) = (svc.clock)();
        svc.showcase = on;
        svc.preview = on.then(|| {
            RewardsState::new(crate::showcase::seeded_ledger(
                today,
                crate::showcase::activation_seed(now_ms),
            ))
        });
        // Moments of a preview never outlive it (moments of real sessions stay).
        svc.pending.retain(|moment| !moment.preview);
        log::info!(
            "rewards: presentation showcase {}",
            if on {
                "on (seeded preview ledger, memory only)"
            } else {
                "off (the real ledger shows again)"
            }
        );
        let rules = svc.shown_rules(cfg);
        svc.publish(&rules, today);
    }

    /// Developer page rewards cheat (`DevAction::RewardsCheat`, ADR-019): applied to the showcase ledger while the
    /// presentation showcase is on (memory only), else to the real ledger and saved like any fact (`rewards.json`; a
    /// demo preview goes, so the view shows the result, and the next demo fact starts a new one from the real ledger).
    /// Its moment is kept for the main window like any other. `phase` = now. `Ok(true)` when the real ledger took the
    /// cheat (`false`: the showcase); `rewardsOff` while the shown rules are inactive.
    pub fn cheat(&self, cheat: RewardsCheat, cfg: &Config, phase: Phase) -> Result<bool, AppError> {
        let mut guard = lock(&self.inner);
        let svc = &mut *guard;
        let rules = svc.shown_rules(cfg);
        if !rules.active {
            return Err(AppError::new(ERR_REWARDS_OFF, "the beta rewards are off"));
        }
        let (today, now_ms) = (svc.clock)();
        let before = svc.real.ledger.clone();
        let real = !svc.showcase;
        let moment = if real {
            svc.preview = None;
            rewards::cheat(
                &mut svc.real,
                today,
                &rules,
                LedgerMode::Real,
                cheat,
                phase,
                now_ms,
            )
        } else {
            let preview = svc.preview.get_or_insert_with(|| svc.real.clone());
            rewards::cheat(
                preview,
                today,
                &rules,
                LedgerMode::Preview,
                cheat,
                phase,
                now_ms,
            )
        };
        log::info!(
            "rewards: developer cheat {cheat:?}{}",
            if real { "" } else { ", showcase" }
        );
        if let Some(moment) = moment {
            svc.push(moment);
        }
        svc.save_if_changed(&before);
        svc.publish(&rules, today);
        Ok(real)
    }

    /// The presentation showcase is on ([`Self::set_showcase`]).
    pub fn showcase_on(&self) -> bool {
        lock(&self.inner).showcase
    }

    /// Session `session_id` belongs to the showcase: it ends while the showcase is on, or it is the demo session that
    /// ran while it was on (switched off before that session ended). Its summary is only shown
    /// (`crate::showcase::keeps_summary`).
    pub fn showcase_session(&self, session_id: &str) -> bool {
        let svc = lock(&self.inner);
        svc.showcase || svc.showcase_session.as_deref() == Some(session_id)
    }

    /// The showcase went on while demo session `session_id` runs: it belongs to the showcase too
    /// ([`Self::showcase_session`]). Ignored while the showcase is off.
    pub fn adopt_showcase_session(&self, session_id: &str) {
        let mut svc = lock(&self.inner);
        if svc.showcase {
            svc.mark_showcase_session(session_id);
        }
    }

    /// App exit: the preview goes and the last write finishes (bounded wait). Blocking.
    pub fn flush_blocking(&self) {
        let (ack_tx, ack_rx) = mpsc::channel();
        {
            let mut svc = lock(&self.inner);
            svc.preview = None;
            svc.writer.flush(ack_tx);
        }
        if ack_rx.recv_timeout(FLUSH_TIMEOUT).is_err() {
            log::warn!("rewards: the last write did not finish in time");
        }
    }
}

/// `focusTime+fullBlock`: the kinds of a moment's lines, for the log.
fn kinds(moment: &RewardMoment) -> String {
    let kinds: Vec<String> = moment
        .lines
        .iter()
        .map(|line| format!("{:?}", line.kind))
        .collect();
    kinds.join("+")
}

/// The reward day from the local date and time; the UTC date when the local time is unavailable (best effort, ADR-008).
fn system_clock() -> (RewardDay, i64) {
    let now = wall_ms();
    let today = focus_platform::clock::local_date_time()
        .and_then(|(year, month, day, minute)| rewards::reward_day(year, month, day, minute))
        .unwrap_or_else(|| utc_reward_day(now));
    (today, now)
}

fn utc_reward_day(wall_ms: i64) -> RewardDay {
    let day = wall_ms.div_euclid(MS_PER_DAY);
    let minute = wall_ms.rem_euclid(MS_PER_DAY) / MS_PER_MINUTE;
    let day = if minute < i64::from(REWARD_DAY_START_MINUTE) {
        day - 1
    } else {
        day
    };
    RewardDay::try_from(day).unwrap_or(0)
}

fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// A seed for the surprise rolls of a new ledger (no new crate: the standard library's random hasher keys).
fn new_seed() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_i64(wall_ms());
    hasher.write_u32(std::process::id());
    hasher.finish()
}

/// Reads the stored ledger (normalized); a fresh one when there is none. Writes nothing: a broken file (bad JSON or
/// not text) gives the backup's ledger with [`Repair::Restore`], or a fresh one with [`Repair::SetAside`] and the
/// `rewards.load` error. A file that cannot be read at all (locked, no access) is read again a few times; if it
/// still cannot be, it may well be fine, so it is never replaced or set aside: the backup (or a fresh ledger) shows
/// and nothing is written this run.
fn load_ledger(persistence: &Persistence) -> Loaded {
    let parse = |text: &str| {
        serde_json::from_str::<Ledger>(text).map(|mut ledger| {
            ledger.normalize();
            ledger
        })
    };
    let backup = || {
        persistence
            .read_rewards_backup()
            .ok()
            .flatten()
            .and_then(|text| parse(&text).ok())
    };
    let mut retries = 0;
    let read = loop {
        match persistence.read_rewards() {
            Err(err) if err.kind() != ErrorKind::InvalidData && retries < READ_RETRIES => {
                retries += 1;
                log::warn!("rewards: cannot read rewards.json ({err}); trying again");
                std::thread::sleep(READ_RETRY_DELAY);
            }
            read => break read,
        }
    };
    let detail = match read {
        Ok(None) => return Loaded::fine(Ledger::new(new_seed())),
        Ok(Some(text)) => match parse(&text) {
            Ok(ledger) => return Loaded::fine(ledger),
            Err(err) => err.to_string(),
        },
        // Not UTF-8: as broken as bad JSON.
        Err(err) if err.kind() == ErrorKind::InvalidData => err.to_string(),
        Err(err) => {
            let restored = backup();
            log::warn!(
                "rewards: rewards.json cannot be read ({err}); {}, nothing is saved this run",
                if restored.is_some() {
                    "the backup shows"
                } else {
                    "a fresh ledger shows"
                }
            );
            return Loaded {
                ledger: restored.unwrap_or_else(|| Ledger::new(new_seed())),
                repair: None,
                problem: Some(AppError::new(
                    ERR_LOAD,
                    format!("rewards.json could not be read ({err}); it is kept as it is"),
                )),
                read_only: true,
            };
        }
    };
    log::warn!("rewards: rewards.json is not valid ({detail}); trying the backup");
    match backup() {
        Some(ledger) => {
            log::info!("rewards: restored from rewards.json.bak");
            Loaded {
                ledger,
                repair: Some(Repair::Restore),
                problem: None,
                read_only: false,
            }
        }
        None => Loaded {
            ledger: Ledger::new(new_seed()),
            repair: Some(Repair::SetAside),
            problem: Some(AppError::new(
                ERR_LOAD,
                format!(
                    "rewards.json could not be read ({detail}); it is kept aside and the sparks start again"
                ),
            )),
            read_only: false,
        },
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Writer thread
// ---------------------------------------------------------------------------------------------------------------

enum WriteCmd {
    /// The latest real ledger (a newer one in the queue replaces it).
    Save(Ledger),
    /// Writes the ledger restored from the backup over a broken `rewards.json`, without a backup copy of the broken
    /// file (done before any save of the same batch).
    Restore(Ledger),
    /// Sets a broken `rewards.json` aside as `rewards.unreadable-<ms>.json` (before any save of the same batch).
    SetAside(i64),
    /// Delete the files, then write `fresh` (if any); `done` gets the result of the delete.
    Reset {
        fresh: Option<Ledger>,
        done: Option<Sender<Result<(), String>>>,
    },
    /// Answered once everything queued before it is written.
    Flush(Sender<()>),
}

/// Handle to the coalescing writer ("focus-rewards"): writes in order, only the latest ledger of a burst.
struct Writer {
    tx: Sender<WriteCmd>,
}

impl Writer {
    fn spawn(persistence: Persistence, report: Arc<dyn Fn(AppError) + Send + Sync>) -> Self {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("focus-rewards".into())
            .spawn(move || run_writer(&rx, &persistence, report.as_ref()));
        if let Err(err) = spawned {
            // Degraded (rule 8): the sparks count in memory, nothing is written this run.
            log::error!("rewards: cannot start the writer thread: {err}");
        }
        Self { tx }
    }

    fn send(&self, cmd: WriteCmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("rewards: the writer is not running; rewards.json is not updated");
        }
    }

    fn save(&self, ledger: Ledger) {
        self.send(WriteCmd::Save(ledger));
    }

    fn restore(&self, ledger: Ledger) {
        self.send(WriteCmd::Restore(ledger));
    }

    fn set_aside(&self, now_ms: i64) {
        self.send(WriteCmd::SetAside(now_ms));
    }

    fn reset(&self, fresh: Option<Ledger>, done: Option<Sender<Result<(), String>>>) {
        self.send(WriteCmd::Reset { fresh, done });
    }

    fn flush(&self, ack: Sender<()>) {
        self.send(WriteCmd::Flush(ack));
    }
}

fn run_writer(
    rx: &Receiver<WriteCmd>,
    persistence: &Persistence,
    report: &(dyn Fn(AppError) + Send + Sync),
) {
    let mut last_backup: Option<Instant> = None;
    // The broken file could not be replaced by the restored ledger: never copy it over the good backup.
    let mut keep_backup = false;
    let mut save_reported = false;
    while let Ok(first) = rx.recv() {
        let mut latest = None;
        let mut reset = false;
        let mut repair = None;
        let mut dones = Vec::new();
        let mut flushes = Vec::new();
        for cmd in std::iter::once(first).chain(rx.try_iter()) {
            match cmd {
                WriteCmd::Save(ledger) => latest = Some(ledger),
                WriteCmd::Restore(ledger) => repair = Some(Ok(ledger)),
                WriteCmd::SetAside(now_ms) => repair = Some(Err(now_ms)),
                WriteCmd::Reset { fresh, done } => {
                    reset = true;
                    latest = fresh;
                    dones.extend(done);
                }
                WriteCmd::Flush(ack) => flushes.push(ack),
            }
        }
        // A reset deletes the broken file anyway.
        match repair.filter(|_| !reset) {
            Some(Ok(ledger)) => match persistence.save_rewards(&ledger, false) {
                Ok(()) => log::info!("rewards: the broken rewards.json was replaced by its backup"),
                Err(err) => {
                    keep_backup = true;
                    log::warn!("rewards: cannot write the restored ledger: {err:#}");
                }
            },
            Some(Err(now_ms)) => match persistence.set_aside_rewards(now_ms) {
                Ok(path) => log::warn!(
                    "rewards: unreadable rewards.json kept as {}",
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default()
                ),
                Err(err) => {
                    log::warn!("rewards: cannot set the unreadable rewards.json aside: {err:#}");
                }
            },
            None => {}
        }
        if reset {
            let deleted = persistence.clear_rewards();
            match &deleted {
                Ok(()) => log::info!("rewards: rewards.json deleted"),
                Err(err) => log::warn!("rewards: cannot delete rewards.json: {err:#}"),
            }
            last_backup = None;
            keep_backup = false;
            let outcome = deleted.map_err(|err| format!("{err:#}"));
            for done in dones {
                let _ = done.send(outcome.clone());
            }
        }
        if let Some(ledger) = latest {
            let backup =
                !reset && !keep_backup && last_backup.is_none_or(|at| at.elapsed() >= BACKUP_EVERY);
            match persistence.save_rewards(&ledger, backup) {
                Ok(()) if backup => last_backup = Some(Instant::now()),
                // The restored ledger is in place now: the backup may follow it again.
                Ok(()) if keep_backup => keep_backup = false,
                Ok(()) => {}
                Err(err) => {
                    log::warn!("rewards: cannot write rewards.json: {err:#}");
                    if !save_reported {
                        save_reported = true;
                        report(AppError::new(
                            ERR_SAVE,
                            format!("cannot save rewards.json: {err:#}"),
                        ));
                    }
                }
            }
        }
        for ack in flushes {
            let _ = ack.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::api::{MomentKind, StepDraft};
    use focus_core::session::{Input, Now, Session, normalize_steps};
    use focus_core::timing::TimingProfile;
    use std::fs;
    use std::path::PathBuf;

    /// Monday 2026-10-12.
    const MON: RewardDay = 20_738;

    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<RewardsStatus>>>);

    impl Outlet for Recorder {
        fn status(&self, status: &RewardsStatus) {
            lock(&self.0).push(status.clone());
        }
    }

    /// Deletes the test folder when the last fixture over it goes.
    struct TempRoot(PathBuf);

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct Fixture {
        root: Arc<TempRoot>,
        persistence: Persistence,
        handle: RewardsHandle,
        statuses: Recorder,
        errors: Arc<Mutex<Vec<AppError>>>,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("focusmode-rewards-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let persistence = Persistence::with_test_dirs(&root.join("config"), &root.join("data"))
                .expect("dirs");
            Self::open(Arc::new(TempRoot(root)), persistence)
        }

        /// The service as the app builds it at start (the rewards on), over the files in `root`.
        fn open(root: Arc<TempRoot>, persistence: Persistence) -> Self {
            Self::open_with(root, persistence, true)
        }

        /// Like [`Self::open`], with the rewards on or off at start.
        fn open_with(root: Arc<TempRoot>, persistence: Persistence, active: bool) -> Self {
            let errors = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&errors);
            let report: Report = Arc::new(move |error| lock(&sink).push(error));
            let writer = Writer::spawn(persistence.clone(), Arc::clone(&report));
            let mut loaded = load_ledger(&persistence);
            if active {
                lock(&errors).extend(loaded.problem.take());
            }
            let statuses = Recorder::default();
            let handle = RewardsHandle::with_parts(
                loaded,
                active,
                writer,
                Box::new(statuses.clone()),
                Box::new(|| (MON, 1_791_885_600_000)),
                report,
            );
            Self {
                root,
                persistence,
                handle,
                statuses,
                errors,
            }
        }

        fn file(&self) -> PathBuf {
            self.persistence.files().rewards
        }

        fn reopen(self) -> Self {
            self.reopen_with(true)
        }

        fn reopen_with(self, active: bool) -> Self {
            let Self {
                root, persistence, ..
            } = self;
            Self::open_with(root, persistence, active)
        }
    }

    fn beta() -> Config {
        Config {
            beta_features: true,
            ..Config::default()
        }
    }

    /// Runs a session through the core and feeds its facts to the service, like `Effects::dispatch` does.
    fn run_session(
        handle: &RewardsHandle,
        cfg: &Config,
        profile: TimingProfile,
        steps: &[&str],
    ) -> Vec<RewardMoment> {
        // No sleep-gap detection: the test jumps straight to the deadlines.
        let profile = TimingProfile {
            interruption_gap_secs: 0,
            ..profile
        };
        let mut session = Session::new();
        let at = |secs: u64| Now {
            mono_ms: secs * 1000,
            wall_ms: 1_791_885_600_000 + i64::try_from(secs).expect("fits") * 1000,
        };
        let drafts: Vec<StepDraft> = steps
            .iter()
            .map(|text| StepDraft {
                id: None,
                text: (*text).to_owned(),
            })
            .collect();
        session.handle(
            at(0),
            Input::SetSteps(normalize_steps(drafts).expect("steps")),
        );
        let mut effects = session.handle(
            at(0),
            Input::Start {
                session_id: "1760342400000-4242-1".into(),
                profile,
            },
        );
        effects.extend(session.handle(at(1), Input::SetStepDone { id: 1, done: true }));
        let focus = u64::from(profile.focus_secs);
        effects.extend(session.handle(at(focus), Input::Tick));
        // Stopped in the break after half of it (the break counts).
        effects.extend(session.handle(at(focus + u64::from(profile.break_secs) / 2), Input::Stop));
        effects
            .into_iter()
            .filter_map(|effect| match effect {
                focus_core::session::Effect::Progress(fact) => handle.apply_fact(&fact, cfg),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_real_session_writes_numbers_only() {
        let fixture = Fixture::new("real");
        let cfg = beta();
        let moments = run_session(
            &fixture.handle,
            &cfg,
            TimingProfile::normal(),
            &["Zadzwoń do Ani w sprawie umowy"],
        );
        let kinds: Vec<MomentKind> = moments.iter().map(|moment| moment.kind).collect();
        assert_eq!(
            kinds,
            [
                MomentKind::Start,
                MomentKind::Step,
                MomentKind::BreakStart,
                MomentKind::SessionEnd
            ]
        );
        assert_eq!(
            moments.iter().map(|m| m.id).collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
        fixture.handle.flush_blocking();
        let text = fs::read_to_string(fixture.file()).expect("rewards.json");
        assert!(!text.contains("Ani") && !text.contains("umowy"), "{text}");
        let ledger: Ledger = serde_json::from_str(&text).expect("ledger");
        assert!(ledger.total_sparks > 0);
        assert_eq!(
            fixture.handle.session_sparks("1760342400000-4242-1"),
            Some(moments.iter().map(|m| m.sparks).sum())
        );
        let status = fixture.handle.status(&cfg);
        assert!(status.active && !status.preview);
        assert_eq!(status.pending.len(), 4);
        assert_eq!(status.total_sparks, ledger.total_sparks);
        // The UI acknowledged the first three: one is left, and a new status went out.
        let before = lock(&fixture.statuses.0).len();
        fixture.handle.ack(3, &cfg);
        assert_eq!(fixture.handle.status(&cfg).pending.len(), 1);
        assert_eq!(lock(&fixture.statuses.0).len(), before + 1);
        let revisions: Vec<u32> = lock(&fixture.statuses.0)
            .iter()
            .map(|s| s.revision)
            .collect();
        assert!(revisions.windows(2).all(|pair| pair[1] > pair[0]));
        assert!(lock(&fixture.errors).is_empty());
    }

    #[test]
    fn a_demo_session_never_touches_the_file() {
        let fixture = Fixture::new("demo");
        let cfg = beta();
        // No file yet: a demo session does not create one.
        let moments = run_session(&fixture.handle, &cfg, TimingProfile::demo(), &["a", "b"]);
        assert!(moments.iter().all(|moment| moment.preview));
        fixture.handle.flush_blocking();
        assert!(!fixture.file().exists());
        // With a file: byte-identical after a demo session; the preview shows meanwhile.
        run_session(&fixture.handle, &cfg, TimingProfile::normal(), &[]);
        fixture.handle.flush_blocking();
        let before = fs::read(fixture.file()).expect("file");
        run_session(&fixture.handle, &cfg, TimingProfile::demo(), &["a"]);
        assert!(fixture.handle.status(&cfg).preview);
        fixture.handle.flush_blocking();
        assert_eq!(fs::read(fixture.file()).expect("file"), before);
        assert!(!fixture.handle.status(&cfg).preview, "dropped at exit");
    }

    #[test]
    fn with_the_beta_off_nothing_happens() {
        let fixture = Fixture::new("off");
        let off = Config::default();
        assert!(run_session(&fixture.handle, &off, TimingProfile::normal(), &[]).is_empty());
        fixture
            .handle
            .goal_done("1760342400000-4242-1", &off, Phase::Idle, false);
        fixture.handle.config_changed(
            &off,
            &Config {
                theme: focus_core::api::Theme::Lunar,
                ..off.clone()
            },
            Phase::Idle,
        );
        fixture.handle.tick_day(&off);
        fixture.handle.flush_blocking();
        assert!(lock(&fixture.statuses.0).is_empty(), "no rewards://changed");
        assert!(!fixture.file().exists());
        assert!(!fixture.handle.status(&off).active);
        assert_eq!(fixture.handle.session_sparks("1760342400000-4242-1"), None);
        // Turning the beta on and off without a streak writes nothing either.
        fixture.handle.config_changed(&off, &beta(), Phase::Idle);
        fixture.handle.config_changed(&beta(), &off, Phase::Idle);
        fixture.handle.flush_blocking();
        assert!(!fixture.file().exists());
        assert!(
            lock(&fixture.statuses.0)
                .iter()
                .all(|status| !status.active || status.pending.is_empty())
        );
        assert_eq!(
            lock(&fixture.statuses.0).last().map(|status| status.active),
            Some(false)
        );
    }

    #[test]
    fn a_broken_file_is_restored_from_the_backup_or_set_aside() {
        let fixture = Fixture::new("broken");
        let cfg = beta();
        run_session(&fixture.handle, &cfg, TimingProfile::normal(), &[]);
        fixture.handle.flush_blocking();
        let good = fs::read_to_string(fixture.file()).expect("file");
        let total = serde_json::from_str::<Ledger>(&good)
            .expect("ledger")
            .total_sparks;
        let files = fixture.persistence.files();
        fs::write(&files.rewards_backup, &good).expect("backup");
        fs::write(&files.rewards, b"{ not json").expect("break");

        let fixture = fixture.reopen();
        assert!(lock(&fixture.errors).is_empty(), "recovered quietly");
        assert_eq!(fixture.handle.status(&cfg).total_sparks, total);
        fixture.handle.flush_blocking();
        assert_eq!(
            serde_json::from_str::<Ledger>(&fs::read_to_string(fixture.file()).expect("rewritten"))
                .expect("valid again")
                .total_sparks,
            total
        );

        // Both broken: set aside, a fresh ledger, one calm error; the session still works.
        fs::write(&files.rewards, b"[1, 2").expect("break");
        fs::write(&files.rewards_backup, b"nope").expect("break backup");
        let fixture = fixture.reopen();
        let errors: Vec<String> = lock(&fixture.errors)
            .iter()
            .map(|e| e.code.clone())
            .collect();
        assert_eq!(errors, [ERR_LOAD]);
        assert_eq!(fixture.handle.status(&cfg).total_sparks, 0);
        fixture.handle.flush_blocking();
        assert!(!fixture.file().exists());
        assert_eq!(set_aside_count(&files.data_dir), 1);
        assert!(!run_session(&fixture.handle, &cfg, TimingProfile::normal(), &[]).is_empty());
        fixture.handle.flush_blocking();
        assert!(fixture.file().exists());
        // The privacy delete takes every rewards file, the set-aside one included.
        fixture.handle.delete_files(&cfg).expect("deleted");
        assert!(!fixture.file().exists() && !files.rewards_backup.exists());
        assert_eq!(
            fs::read_dir(&files.data_dir)
                .expect("dir")
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("rewards"))
                .count(),
            0
        );
        assert_eq!(fixture.handle.status(&cfg).total_sparks, 0);
    }

    fn set_aside_count(dir: &std::path::Path) -> usize {
        fs::read_dir(dir)
            .expect("dir")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("rewards.unreadable-")
            })
            .count()
    }

    /// A real session written to the file, which is then copied to the backup; returns the total.
    fn good_file_and_backup(fixture: &Fixture, cfg: &Config) -> u32 {
        run_session(&fixture.handle, cfg, TimingProfile::normal(), &[]);
        fixture.handle.flush_blocking();
        let good = fs::read_to_string(fixture.file()).expect("file");
        fs::write(fixture.persistence.files().rewards_backup, &good).expect("backup");
        serde_json::from_str::<Ledger>(&good)
            .expect("ledger")
            .total_sparks
    }

    #[test]
    fn a_file_that_is_not_text_is_restored_from_the_backup() {
        let fixture = Fixture::new("not-text");
        let cfg = beta();
        let total = good_file_and_backup(&fixture, &cfg);
        fs::write(fixture.file(), [0xFF_u8, 0xFE, 0x00, 0x7B, 0xC3]).expect("break");
        let fixture = fixture.reopen();
        assert!(lock(&fixture.errors).is_empty(), "recovered quietly");
        assert_eq!(fixture.handle.status(&cfg).total_sparks, total);
        fixture.handle.flush_blocking();
        let restored: Ledger =
            serde_json::from_str(&fs::read_to_string(fixture.file()).expect("text again"))
                .expect("valid again");
        assert_eq!(restored.total_sparks, total);
        assert_eq!(set_aside_count(&fixture.persistence.files().data_dir), 0);
    }

    #[test]
    fn with_the_rewards_off_a_broken_file_waits_for_them() {
        let fixture = Fixture::new("off-broken");
        let cfg = beta();
        let off = Config::default();
        let total = good_file_and_backup(&fixture, &cfg);
        let files = fixture.persistence.files();
        // Broken file, good backup: nothing is written while the rewards are off.
        fs::write(&files.rewards, b"{ not json").expect("break");
        let fixture = fixture.reopen_with(false);
        fixture.handle.flush_blocking();
        assert_eq!(fs::read(&files.rewards).expect("untouched"), b"{ not json");
        assert!(lock(&fixture.errors).is_empty());
        // On again: the backup replaces the broken file, quietly.
        fixture.handle.config_changed(&off, &cfg, Phase::Idle);
        fixture.handle.flush_blocking();
        let restored: Ledger =
            serde_json::from_str(&fs::read_to_string(&files.rewards).expect("file"))
                .expect("valid again");
        assert_eq!(restored.total_sparks, total);
        assert!(lock(&fixture.errors).is_empty());

        // Both broken: not renamed while off; set aside (and reported once) when the rewards come on.
        fs::write(&files.rewards, b"[1, 2").expect("break");
        fs::write(&files.rewards_backup, b"nope").expect("break backup");
        let fixture = fixture.reopen_with(false);
        fixture.handle.flush_blocking();
        assert_eq!(fs::read(&files.rewards).expect("untouched"), b"[1, 2");
        assert_eq!(set_aside_count(&files.data_dir), 0);
        assert!(lock(&fixture.errors).is_empty(), "only logged while off");
        fixture.handle.config_changed(&off, &cfg, Phase::Idle);
        fixture.handle.config_changed(&cfg, &cfg, Phase::Idle);
        fixture.handle.flush_blocking();
        assert!(!files.rewards.exists());
        assert_eq!(set_aside_count(&files.data_dir), 1);
        let codes: Vec<String> = lock(&fixture.errors)
            .iter()
            .map(|e| e.code.clone())
            .collect();
        assert_eq!(codes, [ERR_LOAD]);
    }

    #[cfg(windows)]
    #[test]
    fn a_locked_file_is_never_replaced() {
        use std::os::windows::fs::OpenOptionsExt;
        let fixture = Fixture::new("locked");
        let cfg = beta();
        let total = good_file_and_backup(&fixture, &cfg);
        // One more session after the backup: the locked file is newer than the backup.
        run_session(&fixture.handle, &cfg, TimingProfile::normal(), &[]);
        fixture.handle.flush_blocking();
        let files = fixture.persistence.files();
        let before = fs::read(&files.rewards).expect("file");
        // No sharing at all: reading fails with a sharing violation, not with bad data.
        let lock_handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&files.rewards)
            .expect("lock");
        let fixture = fixture.reopen();
        let codes: Vec<String> = lock(&fixture.errors)
            .iter()
            .map(|e| e.code.clone())
            .collect();
        assert_eq!(codes, [ERR_LOAD]);
        assert_eq!(
            fixture.handle.status(&cfg).total_sparks,
            total,
            "the backup shows"
        );
        drop(lock_handle);
        // Sparks still count in memory, but the (good) file is never overwritten or set aside this run.
        assert!(!run_session(&fixture.handle, &cfg, TimingProfile::normal(), &[]).is_empty());
        fixture.handle.flush_blocking();
        assert_eq!(fs::read(&files.rewards).expect("file"), before);
        assert_eq!(set_aside_count(&files.data_dir), 0);
    }

    #[test]
    fn reset_starts_again_and_keeps_at_most_eight_moments() {
        let fixture = Fixture::new("reset");
        let cfg = beta();
        for _ in 0..3 {
            run_session(&fixture.handle, &cfg, TimingProfile::normal(), &["a"]);
        }
        let status = fixture.handle.status(&cfg);
        assert_eq!(status.pending.len(), PENDING_MAX);
        assert!(
            status
                .pending
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
        );
        fixture.handle.flush_blocking();
        let seed =
            serde_json::from_str::<Ledger>(&fs::read_to_string(fixture.file()).expect("file"))
                .expect("ledger")
                .seed;
        let fresh = fixture.handle.reset(&cfg);
        assert_eq!((fresh.total_sparks, fresh.pending.len()), (0, 0));
        fixture.handle.flush_blocking();
        let after = serde_json::from_str::<Ledger>(
            &fs::read_to_string(fixture.file()).expect("fresh file"),
        )
        .expect("ledger");
        assert_eq!(after.total_sparks, 0);
        assert_ne!(after.seed, seed, "a new seed");
        assert!(!fixture.persistence.files().rewards_backup.exists());
        // Turning the rewards off drops the pending moments.
        run_session(&fixture.handle, &cfg, TimingProfile::normal(), &[]);
        fixture
            .handle
            .config_changed(&cfg, &Config::default(), Phase::Idle);
        assert!(fixture.handle.status(&cfg).pending.is_empty());
    }

    #[test]
    fn a_new_reward_day_refreshes_the_view_once() {
        let fixture = Fixture::new("day");
        let cfg = beta();
        fixture.handle.tick_day(&cfg);
        assert!(lock(&fixture.statuses.0).is_empty(), "same day");
        lock(&fixture.handle.inner).clock = Box::new(|| (MON + 1, 1_791_972_000_000));
        fixture.handle.tick_day(&cfg);
        fixture.handle.tick_day(&cfg);
        let statuses = lock(&fixture.statuses.0);
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].today.date, "2026-10-13");
    }

    #[test]
    fn utc_fallback_follows_the_reward_day_rule() {
        // 2026-10-13 03:59 UTC is still Monday's reward day; 04:00 is Tuesday's.
        let day_ms = i64::from(MON + 1) * MS_PER_DAY;
        assert_eq!(utc_reward_day(day_ms + 3 * 3_600_000 + 59 * 60_000), MON);
        assert_eq!(utc_reward_day(day_ms + 4 * 3_600_000), MON + 1);
        let (today, now) = system_clock();
        assert!(today > MON - 1 && now > 0);
        assert_ne!(new_seed(), 0);
    }

    #[test]
    fn steps_in_a_session_never_reach_the_status() {
        let fixture = Fixture::new("texts");
        let cfg = beta();
        run_session(
            &fixture.handle,
            &cfg,
            TimingProfile::normal(),
            &["Tajny plan Ani"],
        );
        let json = serde_json::to_string(&fixture.handle.status(&cfg)).expect("status");
        assert!(!json.contains("Tajny") && !json.contains("Ani"));
    }

    /// A `rewards.json` of a level-8 user written before the collection existed (ADR-024).
    const LEVEL_8_LEDGER: &str = r#"{"version":1,"seed":7,"totalSparks":840}"#;

    fn unlock_ids(fixture: &Fixture) -> Vec<String> {
        let ledger: Ledger =
            serde_json::from_str(&fs::read_to_string(fixture.file()).expect("rewards.json"))
                .expect("ledger");
        ledger.unlocks.into_iter().map(|record| record.id).collect()
    }

    fn unlocked_themes(status: &RewardsStatus) -> Vec<RewardTheme> {
        status
            .unlocks
            .themes
            .iter()
            .filter(|item| item.state == focus_core::api::UnlockState::Unlocked)
            .map(|item| item.theme)
            .collect()
    }

    #[test]
    fn the_load_catches_up_with_the_collection_silently() {
        let fixture = Fixture::new("catch-up");
        let cfg = beta();
        fs::write(fixture.file(), LEVEL_8_LEDGER).expect("old file");
        // The beta off: nothing is recorded or written.
        let fixture = fixture.reopen_with(false);
        fixture.handle.flush_blocking();
        assert_eq!(
            fs::read_to_string(fixture.file()).expect("file"),
            LEVEL_8_LEDGER
        );
        assert!(!fixture.handle.is_theme_unlocked(RewardTheme::Underwater));
        // The rewards on: recorded at once, saved, without a moment.
        let fixture = fixture.reopen();
        let status = fixture.handle.status(&cfg);
        assert!(status.pending.is_empty(), "no celebration");
        assert!(lock(&fixture.statuses.0).is_empty(), "no event at load");
        assert_eq!(
            unlocked_themes(&status),
            [RewardTheme::Underwater, RewardTheme::ForestCabin]
        );
        fixture.handle.flush_blocking();
        assert_eq!(
            unlock_ids(&fixture),
            [
                "theme:underwater",
                "theme:forestCabin",
                "ambience:thunder",
                "ambience:cafe",
                "ambience:forest",
                "ambience:ocean"
            ]
        );
        // The next load finds nothing new and writes nothing.
        let before = fs::read(fixture.file()).expect("file");
        let fixture = fixture.reopen();
        fixture.handle.flush_blocking();
        assert_eq!(fs::read(fixture.file()).expect("file"), before);
        // Switching the beta off and on again keeps every unlock.
        fixture
            .handle
            .config_changed(&cfg, &Config::default(), Phase::Idle);
        fixture
            .handle
            .config_changed(&Config::default(), &cfg, Phase::Idle);
        fixture.handle.flush_blocking();
        assert_eq!(unlock_ids(&fixture).len(), 6);
        assert!(fixture.handle.is_theme_unlocked(RewardTheme::ForestCabin));
        assert!(lock(&fixture.errors).is_empty());
    }

    #[test]
    fn a_collection_theme_can_be_picked_once_it_is_unlocked() {
        let fixture = Fixture::new("pick-theme");
        let handle = &fixture.handle;
        let pick = |previous: Option<RewardTheme>, next: Option<RewardTheme>| {
            unlocks::check_reward_theme(previous, next, |theme| handle.is_theme_unlocked(theme))
        };
        let refused = pick(None, Some(RewardTheme::Underwater)).expect_err("locked");
        assert_eq!(refused.code, focus_core::config::CONFIG_INVALID);
        assert_eq!(refused.field.as_deref(), Some("rewardTheme"));
        assert_eq!(pick(Some(RewardTheme::Underwater), None), Ok(()));
        // A demo session never unlocks anything in the real ledger.
        run_session(handle, &beta(), TimingProfile::demo(), &["a"]);
        assert!(!handle.is_theme_unlocked(RewardTheme::Underwater));
        fs::write(fixture.file(), LEVEL_8_LEDGER).expect("level 8");
        let fixture = fixture.reopen();
        assert!(fixture.handle.is_theme_unlocked(RewardTheme::Underwater));
        assert!(
            unlocks::check_reward_theme(None, Some(RewardTheme::Underwater), |theme| fixture
                .handle
                .is_theme_unlocked(theme))
            .is_ok()
        );
        assert!(!fixture.handle.is_theme_unlocked(RewardTheme::Halloween));
    }

    #[test]
    fn reset_and_the_privacy_delete_empty_the_collection() {
        let fixture = Fixture::new("reset-collection");
        let cfg = beta();
        fs::write(fixture.file(), LEVEL_8_LEDGER).expect("level 8");
        let fixture = fixture.reopen();
        assert_eq!(unlocked_themes(&fixture.handle.status(&cfg)).len(), 2);
        let fresh = fixture.handle.reset(&cfg);
        assert!(unlocked_themes(&fresh).is_empty());
        let free: Vec<_> = fresh
            .unlocks
            .ambiences
            .iter()
            .filter(|item| item.state == focus_core::api::UnlockState::Free)
            .map(|item| item.layer)
            .collect();
        assert_eq!(
            free,
            [
                focus_core::api::AmbienceLayer::Rain,
                focus_core::api::AmbienceLayer::Wind
            ]
        );
        assert!(!fixture.handle.is_theme_unlocked(RewardTheme::Underwater));
        fixture.handle.flush_blocking();
        assert!(unlock_ids(&fixture).is_empty());
        // The privacy delete: the same, and no file is left.
        fs::write(fixture.file(), LEVEL_8_LEDGER).expect("level 8 again");
        let fixture = fixture.reopen();
        assert!(fixture.handle.is_theme_unlocked(RewardTheme::ForestCabin));
        fixture.handle.delete_files(&cfg).expect("deleted");
        assert!(!fixture.file().exists());
        assert!(!fixture.handle.is_theme_unlocked(RewardTheme::ForestCabin));
        assert!(unlocked_themes(&fixture.handle.status(&cfg)).is_empty());
    }

    /// Presentation mode (ADR-019, `crate::showcase`) with the beta off in the user's config: the seeded ledger shows
    /// and a demo session plays its live moments, but `config.json`, `history.jsonl` and `rewards.json` stay byte for
    /// byte as they were, real facts keep the real rules, and switching it off shows the real state again.
    #[test]
    fn a_presentation_showcase_writes_nothing_and_leaves_no_trace() {
        let fixture = Fixture::new("showcase");
        let files = fixture.persistence.files();
        // The user's data: one earlier real session with the beta on (so rewards.json exists), then the beta off.
        run_session(&fixture.handle, &beta(), TimingProfile::normal(), &[]);
        fixture.handle.flush_blocking();
        let off = Config::default();
        fixture.persistence.save_config(&off).expect("config.json");
        fs::write(
            &files.history,
            b"{\"sessionId\":\"earlier\"}
",
        )
        .expect("history.jsonl");
        let read = |path: &PathBuf| fs::read(path).ok();
        let stored = |f: &crate::persistence::DataFiles| {
            [&f.config, &f.history, &f.rewards, &f.rewards_backup].map(read)
        };
        let before = stored(&files);
        assert!(before[..3].iter().all(Option::is_some));
        let real_before = fixture.handle.status(&off);
        assert!(!real_before.active);

        fixture.handle.set_showcase(true, &off);
        assert!(fixture.handle.showcase_on());
        let shown = fixture.handle.status(&off);
        assert!(shown.active && shown.preview);
        assert_eq!((shown.level.level, shown.level.into), (8, 170));
        assert_eq!((shown.streak.days, shown.streak.best), (12, 21));

        let moments = run_session(&fixture.handle, &off, TimingProfile::demo(), &["a", "b"]);
        let kinds: Vec<MomentKind> = moments.iter().map(|moment| moment.kind).collect();
        assert_eq!(
            kinds,
            [
                MomentKind::Start,
                MomentKind::Step,
                MomentKind::BreakStart,
                MomentKind::SessionEnd
            ]
        );
        assert!(moments.iter().all(|moment| moment.preview));
        assert_eq!((moments[0].sparks, moments[1].sparks), (12, 3));
        let burst = &moments[2];
        assert!(burst.daily_goal && burst.surprise.is_some());
        assert_eq!((burst.streak, burst.level_up), (Some(13), Some(9)));
        // A demo session's goal answer goes to the showcase ledger; a real session's facts keep the real rules (off).
        assert_eq!(
            fixture
                .handle
                .goal_done("1760342400000-4242-1", &off, Phase::Idle, true),
            0
        );
        assert!(run_session(&fixture.handle, &off, TimingProfile::normal(), &[]).is_empty());
        // The demo session's summary is shown only (`Effects::finish_session`).
        assert!(!crate::showcase::keeps_summary(
            fixture.handle.showcase_on(),
            true
        ));
        // A second activation keeps the progress of the running showcase.
        let progressed = fixture.handle.status(&off);
        assert!(progressed.level.level >= 9 && progressed.today.goal_met);
        fixture.handle.set_showcase(true, &off);
        assert_eq!(
            fixture.handle.status(&off).total_sparks,
            progressed.total_sparks
        );
        fixture.handle.ack(u32::MAX, &off);

        fixture.handle.set_showcase(false, &off);
        assert!(!fixture.handle.showcase_on());
        let real_after = fixture.handle.status(&off);
        assert_eq!(
            RewardsStatus {
                revision: real_before.revision,
                ..real_after
            },
            real_before
        );
        fixture.handle.flush_blocking();
        assert_eq!(
            stored(&files),
            before,
            "nothing of the showcase was written"
        );
        assert!(lock(&fixture.errors).is_empty());
    }

    /// The demo session started during the showcase stays the showcase's when the showcase is switched off before
    /// it ends (its summary is only shown, `Effects::finish_session`); a demo session started later is an ordinary one.
    #[test]
    fn a_showcase_demo_session_is_never_stored_even_when_it_ends_later() {
        let fixture = Fixture::new("showcase-session");
        let off = Config::default();
        let started = |session_id: &str| ProgressFact::Started {
            session_id: session_id.into(),
            steps: 0,
            profile: TimingProfile::demo(),
        };
        let keeps = |session_id: &str| {
            crate::showcase::keeps_summary(fixture.handle.showcase_session(session_id), true)
        };
        assert!(keeps("before"), "no showcase: an ordinary demo session");
        fixture.handle.adopt_showcase_session("before");
        assert!(keeps("before"), "adopted only while the showcase is on");
        fixture.handle.set_showcase(true, &off);
        assert!(!keeps("before"), "it ends during the showcase");
        fixture.handle.apply_fact(&started("on-stage"), &off);
        fixture.handle.set_showcase(false, &off);
        assert!(
            !keeps("on-stage"),
            "started on stage, ends after the showcase"
        );
        assert!(keeps("before"));
        fixture.handle.apply_fact(&started("later"), &off);
        assert!(keeps("later") && !keeps("on-stage"));
        // Running when the showcase went on (`showcase::set`), ending after it went off: on stage too.
        fixture.handle.set_showcase(true, &off);
        fixture.handle.adopt_showcase_session("later");
        fixture.handle.set_showcase(false, &off);
        assert!(!keeps("later"));
        // A real session is always kept.
        assert!(crate::showcase::keeps_summary(
            fixture.handle.showcase_session("real"),
            false
        ));
    }

    #[test]
    fn a_cheat_goes_to_the_real_ledger_or_to_the_showcase() {
        let fixture = Fixture::new("cheat");
        let off = Config::default();
        let refused = fixture
            .handle
            .cheat(RewardsCheat::UnlockAll, &off, Phase::Idle)
            .expect_err("rewards off");
        assert_eq!(refused.code, ERR_REWARDS_OFF);

        let cfg = beta();
        assert!(
            fixture
                .handle
                .cheat(RewardsCheat::UnlockAll, &cfg, Phase::Idle)
                .expect("real")
        );
        let status = fixture.handle.status(&cfg);
        assert_eq!(unlocked_themes(&status), RewardTheme::ALL);
        assert!(
            status
                .pending
                .last()
                .is_some_and(|moment| moment.unlocked_themes.len() == RewardTheme::ALL.len())
        );
        fixture.handle.flush_blocking();
        assert!(unlock_ids(&fixture).contains(&"theme:halloween".to_owned()));

        // Presentation mode: the showcase ledger takes the cheat, the file stays as it was.
        let stored = fs::read(fixture.file()).expect("rewards.json");
        fixture.handle.set_showcase(true, &cfg);
        assert!(
            !fixture
                .handle
                .cheat(RewardsCheat::LockAll, &cfg, Phase::Idle)
                .expect("showcase")
        );
        assert!(unlocked_themes(&fixture.handle.status(&cfg)).is_empty());
        fixture.handle.flush_blocking();
        assert_eq!(fs::read(fixture.file()).expect("rewards.json"), stored);
        fixture.handle.set_showcase(false, &cfg);
        assert_eq!(
            unlocked_themes(&fixture.handle.status(&cfg)),
            RewardTheme::ALL
        );
    }
}
