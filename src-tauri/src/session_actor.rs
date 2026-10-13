//! Session actor: the only owner of the core `Session` (ADR-012).
//!
//! A tokio task selects over the command channel and a 1 s ticker, feeds `Input`s with `Now`
//! (monotonic `Instant` since process start + wall clock) to `Session::handle`, dispatches the
//! returned effects without waiting for slow work, and publishes `session://snapshot` with a
//! growing `revision`. It also mirrors the state into the extension bridge and the tray.
//!
//! Calendar meetings the user answered with `Extend` / `PauseAtBreak` arrive from `crate::calendar` as
//! wall-clock meetings; the actor turns them into meeting holds with its own clock when they arrive, at
//! every Start and Resume and every [`HOLDS_RESYNC`] while a session runs (so the monotonic times follow
//! the wall clock after a sleep). Whenever the session timing changes (phase, pause, cycle, profile, a
//! meeting extension) the actor tells `crate::calendar`, which plans the breaks from it.
//!
//! The session goal ("Co dalej?", `set_session_goal`) lives in the core session: it is part of every
//! snapshot and moves into the summary on Stop. Every phase change is reported to `crate::work`, which holds
//! new work items back during focus and shows them at the break or after the session.

use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{
    AppError, Meeting, MeetingChoice, MeetingConflict, Phase, SessionGoal, SessionSnapshot, events,
};
use focus_core::session::{Effect, Input, MeetingHold, NotifyKind, Now, Session};
use focus_core::timing::TimingProfile;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};
use tokio::time::MissedTickBehavior;

use crate::bridge::SharedBridge;
use crate::calendar::{CalendarHandle, SessionPlan};
use crate::effects::{Effects, HeldBreak};
use crate::texts;
use crate::tracker::TrackerHandle;
use crate::tray::Tray;
use crate::work::WorkHandle;
use crate::{SharedConfig, lock, read_config};

/// While a session runs, the meeting holds are rebuilt from the wall clock this often.
const HOLDS_RESYNC: Duration = Duration::from_secs(60);

pub enum ActorCmd {
    Start,
    Pause,
    Resume,
    /// Tray convenience: pause when running, resume when paused.
    TogglePause,
    Stop,
    Skip,
    /// Config saved: refresh the profile shown while idle.
    ConfigChanged,
    /// Another launch asked for `--demo`: idle → the demo profile for the rest of this process;
    /// a session runs → a toast asking to stop it first.
    UseDemoProfile,
    GetSnapshot(oneshot::Sender<SessionSnapshot>),
    /// Every meeting answered with `Extend` / `PauseAtBreak` (from `crate::calendar`): replaces the
    /// session's meeting holds.
    SetMeetings(Vec<(Meeting, MeetingChoice)>),
    /// The goal of the coming or running session (`set_session_goal`); `None` clears it.
    SetGoal(Option<SessionGoal>),
    /// App exit: stop the session (history written synchronously), then ack.
    Shutdown(std_mpsc::Sender<()>),
}

#[derive(Clone)]
pub struct ActorHandle {
    tx: mpsc::UnboundedSender<ActorCmd>,
}

impl ActorHandle {
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<ActorCmd>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    pub fn send(&self, cmd: ActorCmd) -> Result<(), AppError> {
        self.tx
            .send(cmd)
            .map_err(|_| AppError::new("session.unavailable", "session actor is not running"))
    }

    pub async fn snapshot(&self) -> Result<SessionSnapshot, AppError> {
        let (reply, response) = oneshot::channel();
        self.send(ActorCmd::GetSnapshot(reply))?;
        response
            .await
            .map_err(|_| AppError::new("session.unavailable", "session actor is not running"))
    }

    /// Stops the running session and waits (bounded) until its summary is saved.
    pub fn shutdown_blocking(&self, timeout: Duration) {
        let (ack_tx, ack_rx) = std_mpsc::channel();
        if self.tx.send(ActorCmd::Shutdown(ack_tx)).is_ok() && ack_rx.recv_timeout(timeout).is_err()
        {
            log::warn!("session did not stop within {timeout:?}");
        }
    }
}

pub struct ActorContext {
    pub app: AppHandle,
    /// Demo profile (`--demo` at start, or later from another launch while idle).
    pub demo: bool,
    pub config: SharedConfig,
    pub effects: Effects,
    pub tracker: TrackerHandle,
    pub bridge: SharedBridge,
    pub tray: Option<Tray>,
    /// Plans the breaks against the calendars; told about every timing change.
    pub calendar: CalendarHandle,
    /// "Co dalej?": told about every phase change (holds new items during focus).
    pub work: WorkHandle,
}

pub fn spawn(ctx: ActorContext, rx: mpsc::UnboundedReceiver<ActorCmd>) {
    tauri::async_runtime::spawn(run(ctx, rx));
}

async fn run(ctx: ActorContext, mut rx: mpsc::UnboundedReceiver<ActorCmd>) {
    let mut actor = Actor {
        ctx,
        session: Session::new(),
        clock: Clock {
            origin: Instant::now(),
        },
        revision: 0,
        last_published: None,
        started: 0,
        shutting_down: false,
        meetings: Vec::new(),
        holds_synced_at: Instant::now(),
        last_plan: None,
        last_work_phase: None,
    };
    let profile = actor.profile();
    actor.session.set_idle_profile(profile);
    actor.publish(true);

    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            cmd = rx.recv() => match cmd {
                Some(cmd) => actor.handle(cmd),
                None => break,
            },
            _ = ticker.tick() => actor.tick(),
        }
    }
    log::info!("session actor stopped");
}

struct Clock {
    origin: Instant,
}

impl Clock {
    fn now(&self) -> Now {
        Now {
            mono_ms: u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX),
            wall_ms: wall_ms(),
        }
    }
}

fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

struct Actor {
    ctx: ActorContext,
    session: Session,
    clock: Clock,
    revision: u32,
    /// Last emitted snapshot (with its revision).
    last_published: Option<SessionSnapshot>,
    /// Sessions started by this process (part of the session id).
    started: u32,
    shutting_down: bool,
    /// The meetings behind the session's meeting holds (`ActorCmd::SetMeetings`).
    meetings: Vec<(Meeting, MeetingChoice)>,
    /// When the holds were last rebuilt from `meetings`.
    holds_synced_at: Instant,
    /// Timing last reported to the calendar.
    last_plan: Option<PlanKey>,
    /// Phase last reported to the work service.
    last_work_phase: Option<Phase>,
}

/// What the calendar's break plan depends on; a new plan is sent when it changes. A running phase keeps
/// its deadline, so its remaining time is not part of the key (a paused one's is frozen and is).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlanKey {
    phase: Phase,
    paused: bool,
    cycle: u32,
    profile: TimingProfile,
    phase_total_secs: u32,
    frozen_ms: u64,
}

impl Actor {
    fn handle(&mut self, cmd: ActorCmd) {
        match cmd {
            ActorCmd::Start => self.start(),
            ActorCmd::Pause => self.apply(Input::Pause),
            ActorCmd::Resume => self.resume(),
            ActorCmd::TogglePause => {
                if self.session.phase() == Phase::Idle {
                    return;
                }
                if self.session.is_paused() {
                    self.resume();
                } else {
                    self.apply(Input::Pause);
                }
            }
            ActorCmd::SetMeetings(meetings) => {
                self.meetings = meetings;
                self.sync_holds();
                self.publish(false);
            }
            // Cleaned by the core; shown by every snapshot, copied into the summary on Stop.
            ActorCmd::SetGoal(goal) => self.apply(Input::SetGoal(goal)),
            ActorCmd::Stop => self.apply(Input::Stop),
            ActorCmd::Skip => self.apply(Input::SkipPhase),
            ActorCmd::ConfigChanged => {
                let profile = self.profile();
                self.session.set_idle_profile(profile);
                self.publish(false);
            }
            ActorCmd::UseDemoProfile => self.use_demo_profile(),
            ActorCmd::GetSnapshot(reply) => {
                let mut snapshot = self.session.snapshot(self.clock.now());
                snapshot.revision = self.revision;
                let _ = reply.send(snapshot);
            }
            ActorCmd::Shutdown(ack) => {
                self.shutting_down = true;
                self.apply(Input::Stop);
                let _ = ack.send(());
            }
        }
    }

    fn start(&mut self) {
        if self.shutting_down {
            return;
        }
        if self.session.phase() != Phase::Idle {
            // Already running: keep the input idempotent, just re-publish.
            self.publish(true);
            return;
        }
        let profile = self.profile();
        self.started = self.started.wrapping_add(1);
        let session_id = format!("{}-{}-{}", wall_ms(), std::process::id(), self.started);
        log::info!(
            "starting session {session_id}: focus {} s, break {} s, demo {}",
            profile.focus_secs,
            profile.break_secs,
            profile.demo
        );
        // Reset before the core emits `Tracking(true)`; the tracker channel keeps the order.
        self.ctx.tracker.reset(profile.idle_after_secs);
        self.sync_holds();
        self.apply(Input::Start {
            session_id,
            profile,
        });
    }

    /// Switches to the demo profile while idle (the snapshot shows `demo`, the next Start uses it); a running
    /// or paused session keeps its profile and the user is asked to stop it first.
    fn use_demo_profile(&mut self) {
        // Already on: a running session started after it was switched on, so it uses the demo profile too.
        if self.shutting_down || self.ctx.demo {
            return;
        }
        if self.session.phase() != Phase::Idle {
            log::info!("--demo from another launch: a session runs, asking to stop it first");
            self.ctx
                .effects
                .shields
                .toast(texts::demo_needs_stop_toast());
            return;
        }
        self.ctx.demo = true;
        log::info!("--demo from another launch: demo profile from now on");
        let profile = self.profile();
        self.session.set_idle_profile(profile);
        self.publish(true);
    }

    /// Resume, with the meeting holds rebuilt first (the monotonic clock may have drifted during a sleep).
    fn resume(&mut self) {
        self.sync_holds();
        self.apply(Input::Resume);
    }

    /// Rebuilds the session's meeting holds from `meetings` with the clock now. No effects.
    fn sync_holds(&mut self) {
        let now = self.clock.now();
        let holds = self
            .meetings
            .iter()
            .filter_map(|(meeting, choice)| MeetingHold::from_meeting(meeting, *choice, now))
            .collect();
        let effects = self.session.handle(now, Input::SetMeetingHolds(holds));
        self.dispatch(effects, now);
        self.holds_synced_at = Instant::now();
    }

    /// Executes the effects; a held break gets its meeting and length for the popup.
    fn dispatch(&mut self, effects: Vec<Effect>, now: Now) {
        let held = effects
            .iter()
            .any(|effect| matches!(effect, Effect::Notify(NotifyKind::BreakHeld { .. })))
            .then(|| self.held_break(now));
        self.ctx
            .effects
            .dispatch(effects, self.shutting_down, held.as_ref());
    }

    /// The break that just started paused: its length and the meeting holding it.
    fn held_break(&self, now: Now) -> HeldBreak {
        let break_secs = self
            .session
            .profile()
            .map_or(0, |profile| profile.break_secs);
        let snapshot = self.session.snapshot(now);
        let meeting = snapshot.meeting_hold.as_ref().and_then(|view| {
            self.meetings
                .iter()
                .find(|(meeting, choice)| {
                    *choice == view.choice
                        && meeting.end_ms == view.until_ms
                        && meeting.subject == view.subject
                })
                .map(|(meeting, choice)| MeetingConflict {
                    meeting: meeting.clone(),
                    break_number: snapshot.cycle,
                    break_start_ms: now.wall_ms,
                    break_end_ms: now
                        .wall_ms
                        .saturating_add(i64::from(break_secs).saturating_mul(1000)),
                    choice: Some(*choice),
                    next_reminder_ms: None,
                    remind_later_allowed: false,
                    // The session held the break, so the meeting is short enough.
                    too_long: false,
                })
        });
        HeldBreak {
            break_secs,
            meeting,
        }
    }

    /// Demo profile, or the profile from the current config.
    fn profile(&self) -> TimingProfile {
        if self.ctx.demo {
            TimingProfile::demo()
        } else {
            TimingProfile::from_config(&read_config(&self.ctx.config))
        }
    }

    fn apply(&mut self, input: Input) {
        let now = self.clock.now();
        let effects = self.session.handle(now, input);
        self.dispatch(effects, now);
        self.publish(true);
    }

    fn tick(&mut self) {
        if self.session.phase() == Phase::Idle {
            return;
        }
        if !self.meetings.is_empty() && self.holds_synced_at.elapsed() >= HOLDS_RESYNC {
            self.sync_holds();
        }
        let now = self.clock.now();
        let effects = self.session.handle(now, Input::Tick);
        let changed = !effects.is_empty();
        self.dispatch(effects, now);
        self.publish(changed);
    }

    /// Tells the calendar about a new session timing (only when it changed).
    fn report_plan(&mut self, snapshot: &SessionSnapshot, now: Now) {
        let remaining_ms = self.session.remaining_ms(now).unwrap_or(0);
        let profile = self.session.profile().unwrap_or_else(|| self.profile());
        let key = PlanKey {
            phase: snapshot.phase,
            paused: snapshot.paused,
            cycle: snapshot.cycle,
            profile,
            phase_total_secs: snapshot.phase_total_secs,
            frozen_ms: if snapshot.paused { remaining_ms } else { 0 },
        };
        if self.last_plan == Some(key) {
            return;
        }
        self.last_plan = Some(key);
        self.ctx.calendar.session_changed(SessionPlan {
            phase: snapshot.phase,
            paused: snapshot.paused,
            cycle: snapshot.cycle,
            remaining_ms,
            profile,
        });
    }

    /// Emits `session://snapshot` when forced (after an input) or when the view changed.
    fn publish(&mut self, force: bool) {
        let now = self.clock.now();
        let mut snapshot = self.session.snapshot(now);
        if !self.shutting_down {
            self.report_plan(&snapshot, now);
            if self.last_work_phase != Some(snapshot.phase) {
                self.last_work_phase = Some(snapshot.phase);
                self.ctx.work.session_changed(snapshot.phase);
            }
        }
        if !force {
            let unchanged = self.last_published.as_ref().is_some_and(|last| {
                let mut last = last.clone();
                last.revision = snapshot.revision;
                last == snapshot
            });
            if unchanged {
                return;
            }
        }
        self.revision = self.revision.wrapping_add(1);
        snapshot.revision = self.revision;
        if let Err(err) = self
            .ctx
            .app
            .emit(events::SESSION_SNAPSHOT, snapshot.clone())
        {
            log::warn!("cannot emit snapshot: {err}");
        }
        {
            let mut bridge = lock(&self.ctx.bridge);
            bridge.phase = snapshot.phase;
            bridge.remaining_secs = snapshot.remaining_secs;
        }
        // Non-blocking: the tray changes are posted to the main thread. Skipped while quitting,
        // when the main thread is busy shutting down and the menu is about to disappear.
        if !self.shutting_down
            && let Some(tray) = &self.ctx.tray
        {
            tray.update(&snapshot);
        }
        self.last_published = Some(snapshot);
    }
}
