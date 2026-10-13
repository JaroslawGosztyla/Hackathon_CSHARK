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
//! meeting extension, a smart-break fit) the actor tells `crate::calendar`, which plans the breaks from it.
//! Smart breaks (ADR-023): the calendar also sends every meeting with whether a break may be moved for it
//! (`ActorCmd::SetSmartMeetings`); the core fits the running focus block itself (`Input::SetSmartMeetings`).
//!
//! The session goal ("Co dalej?", `set_session_goal`) lives in the core session: it is part of every
//! snapshot and moves into the summary on Stop. Every phase change is reported to `crate::work`, which holds
//! new work items back during focus and shows them at the break or after the session.
//!
//! Beta (ADR-022): the session's small steps live in the core session too (`set_session_steps`, `set_step_done`);
//! a focus popup after a break gets the next open step, and switching small steps off clears the plan. Every
//! [`DAY_CHECK`] (also while idle) the rewards service checks for a new reward day; at exit its last write is flushed.

use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{
    AppError, Meeting, MeetingChoice, MeetingConflict, Phase, SessionGoal, SessionSnapshot,
    SmartBreak, StepDraft, events,
};
use focus_core::calendar::SmartMeeting;
use focus_core::session::{Effect, Input, MeetingHold, NotifyKind, Now, Session};
use focus_core::timing::TimingProfile;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};
use tokio::time::MissedTickBehavior;

use crate::bridge::SharedBridge;
use crate::calendar::{CalendarHandle, SessionPlan};
use crate::effects::{Effects, HeldBreak, NotifyContext};
use crate::texts;
use crate::tracker::TrackerHandle;
use crate::tray::Tray;
use crate::work::WorkHandle;
use crate::{SharedConfig, lock, read_config};

/// While a session runs, the meeting holds are rebuilt from the wall clock this often.
const HOLDS_RESYNC: Duration = Duration::from_secs(60);
/// The rewards service looks for a new reward day this often (beta, ADR-022).
const DAY_CHECK: Duration = Duration::from_secs(60);

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
    /// Developer page "Presentation mode" (`dev_trigger`, ADR-019): `true` → the demo profile, `false` → the
    /// profile from the config, for the rest of this process. Idle: at once; a running or paused session keeps its
    /// profile and the next Start uses the new one. Replies once applied.
    SetPresentationMode {
        on: bool,
        reply: oneshot::Sender<()>,
    },
    GetSnapshot(oneshot::Sender<SessionSnapshot>),
    /// Every meeting answered with `Extend` / `PauseAtBreak` (from `crate::calendar`): replaces the
    /// session's meeting holds.
    SetMeetings(Vec<(Meeting, MeetingChoice)>),
    /// Smart breaks (ADR-023, from `crate::calendar`): every meeting with whether a break may be moved for it (empty
    /// while smart breaks are off). The session fits its focus blocks around them itself.
    SetSmartMeetings(Vec<SmartMeeting>),
    /// The goal of the coming or running session (`set_session_goal`); `None` clears it.
    SetGoal(Option<SessionGoal>),
    /// Developer mode (`dev_trigger`, ADR-019): the current phase ends in `secs` (`Input::DevSetRemaining`);
    /// `notRunning` while idle.
    DevSetRemaining {
        secs: u32,
        reply: oneshot::Sender<Result<(), AppError>>,
    },
    /// Beta (ADR-022): the small steps of the coming or running session (`set_session_steps`, already cleaned by
    /// `normalize_steps`). The texts are never logged.
    SetSteps(Vec<StepDraft>),
    /// Beta (ADR-022): ticks or unticks a step (`set_step_done`); Idle or an unknown id: no-op.
    SetStepDone {
        id: u32,
        done: bool,
    },
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

    /// Developer mode (ADR-019): the current phase ends in `secs` (clamped by the core); `notRunning` while idle.
    pub async fn dev_set_remaining(&self, secs: u32) -> Result<(), AppError> {
        let (reply, response) = oneshot::channel();
        self.send(ActorCmd::DevSetRemaining { secs, reply })?;
        response
            .await
            .map_err(|_| AppError::new("session.unavailable", "session actor is not running"))?
    }

    /// Developer mode (ADR-019): presentation mode (the demo profile) on or off; see [`ActorCmd::SetPresentationMode`].
    pub async fn set_presentation_mode(&self, on: bool) -> Result<(), AppError> {
        let (reply, response) = oneshot::channel();
        self.send(ActorCmd::SetPresentationMode { on, reply })?;
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
    /// Demo profile (`--demo` at start, later from another launch while idle, or the Developer page's
    /// "Presentation mode", which also switches it off). Used by the next Start and the idle snapshot.
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
        day_checked_at: Instant::now(),
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
    /// When the rewards service last looked for a new reward day.
    day_checked_at: Instant,
}

/// What the calendar's break plan depends on; a new plan is sent when it changes. A running phase keeps
/// its deadline, so its remaining time is not part of the key (a paused one's is frozen and is). A smart-break fit
/// (ADR-023) changes the phase length and the moved break.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanKey {
    phase: Phase,
    paused: bool,
    cycle: u32,
    profile: TimingProfile,
    phase_total_secs: u32,
    frozen_ms: u64,
    smart: Option<SmartBreak>,
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
            // Wall-clock meetings: the core fits the running block with the clock of this input (no effects).
            ActorCmd::SetSmartMeetings(meetings) => self.apply(Input::SetSmartMeetings(meetings)),
            // Cleaned by the core; shown by every snapshot, copied into the summary on Stop.
            ActorCmd::SetGoal(goal) => self.apply(Input::SetGoal(goal)),
            ActorCmd::DevSetRemaining { secs, reply } => {
                let _ = reply.send(self.dev_set_remaining(secs));
            }
            // Shown by every snapshot; the core never trusts the texts and keeps at most 5.
            ActorCmd::SetSteps(steps) => self.apply(Input::SetSteps(steps)),
            ActorCmd::SetStepDone { id, done } => self.apply(Input::SetStepDone { id, done }),
            ActorCmd::Stop => self.apply(Input::Stop),
            ActorCmd::Skip => self.apply(Input::SkipPhase),
            ActorCmd::ConfigChanged => {
                let profile = self.profile();
                self.session.set_idle_profile(profile);
                if !read_config(&self.ctx.config).steps_active() && !self.session.steps().is_empty()
                {
                    // Small steps were switched off: no hidden plan lingers (nor reaches the summary).
                    log::info!("small steps are off: the session plan is cleared");
                    self.apply(Input::SetSteps(Vec::new()));
                }
                self.publish(false);
                // A new UI language relabels the tray even when the snapshot did not change.
                if !self.shutting_down
                    && let (Some(tray), Some(snapshot)) = (&self.ctx.tray, &self.last_published)
                {
                    tray.update(snapshot);
                }
            }
            ActorCmd::UseDemoProfile => self.use_demo_profile(),
            ActorCmd::SetPresentationMode { on, reply } => {
                self.set_presentation_mode(on);
                let _ = reply.send(());
            }
            ActorCmd::GetSnapshot(reply) => {
                let mut snapshot = self.session.snapshot(self.clock.now());
                snapshot.revision = self.revision;
                let _ = reply.send(snapshot);
            }
            ActorCmd::Shutdown(ack) => {
                self.shutting_down = true;
                self.apply(Input::Stop);
                // The facts of the stopped session are in the ledger: write it before the process ends.
                self.ctx.effects.rewards.flush_blocking();
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
        // Already on: the next Start uses it (a running session started with it, or the Developer page switched it
        // on while that session ran).
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
        log::info!("--demo from another launch: demo profile from now on");
        self.switch_profile(true);
    }

    /// Developer page "Presentation mode" (ADR-019): the demo profile on or off for the rest of this process. Idle:
    /// the snapshot shows it at once. A running or paused session keeps its profile and phase lengths (only the idle
    /// profile changes, which the core never applies to a running session); the next Start uses the new one.
    fn set_presentation_mode(&mut self, on: bool) {
        if self.shutting_down {
            return;
        }
        if self.ctx.demo == on {
            self.publish(true);
            return;
        }
        let when = if self.session.phase() == Phase::Idle {
            "now"
        } else {
            "from the next Start (the running session keeps its profile)"
        };
        log::info!(
            "developer mode: presentation mode {} {when}",
            if on { "on" } else { "off" }
        );
        self.switch_profile(on);
    }

    /// Demo profile on or off for the next Start and the idle snapshot (a running session is not touched).
    fn switch_profile(&mut self, demo: bool) {
        self.ctx.demo = demo;
        let profile = self.profile();
        self.session.set_idle_profile(profile);
        // Idle, the calendar gets the new profile with this snapshot (it is part of the plan key).
        self.publish(true);
    }

    /// Developer mode (ADR-019): the current phase ends in `secs`. Checked here, where the session lives, so no
    /// Stop can slip in between the check and the input.
    fn dev_set_remaining(&mut self, secs: u32) -> Result<(), AppError> {
        self.session.check_dev_set_remaining()?;
        log::info!(
            "developer mode: the current phase ends in {secs} s (clamped to the phase length)"
        );
        // A running phase keeps its deadline out of the plan key: make the calendar plan from the new one.
        self.last_plan = None;
        self.apply(Input::DevSetRemaining { secs });
        Ok(())
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

    /// Executes the effects; a held break gets its meeting and length for the popup, a focus start after a break the
    /// next open step (beta, while small steps are on).
    fn dispatch(&mut self, effects: Vec<Effect>, now: Now) {
        let held = effects
            .iter()
            .any(|effect| matches!(effect, Effect::Notify(NotifyKind::BreakHeld { .. })))
            .then(|| self.held_break(now));
        let focus_started = effects
            .iter()
            .any(|effect| matches!(effect, Effect::Notify(NotifyKind::FocusStarted { .. })));
        let next_step = (focus_started && read_config(&self.ctx.config).steps_active())
            .then(|| {
                self.session
                    .steps()
                    .iter()
                    .find(|step| !step.done)
                    .map(|step| step.text.clone())
            })
            .flatten();
        let context = NotifyContext { held, next_step };
        self.ctx
            .effects
            .dispatch(effects, self.shutting_down, &context);
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
        if self.day_checked_at.elapsed() >= DAY_CHECK {
            // Also while idle: the week strip and "today" follow the reward day.
            self.day_checked_at = Instant::now();
            self.ctx
                .effects
                .rewards
                .tick_day(&read_config(&self.ctx.config));
        }
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
            smart: snapshot.smart_break.clone(),
        };
        if self.last_plan.as_ref() == Some(&key) {
            return;
        }
        self.last_plan = Some(key);
        self.ctx.calendar.session_changed(SessionPlan {
            phase: snapshot.phase,
            paused: snapshot.paused,
            cycle: snapshot.cycle,
            remaining_ms,
            profile,
            smart: snapshot.smart_break.clone(),
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
