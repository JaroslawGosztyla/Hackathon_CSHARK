//! Session state machine: Idle → Focus → Break → Focus → ... until Stop.
//!
//! Pure: the caller passes the current time with every input ([`Now`]) and executes the
//! returned [`Effect`]s in order. Every input is idempotent, unknown combinations are no-ops,
//! all arithmetic saturates and nothing panics.
//!
//! Phases are timed with the monotonic clock; the wall clock is only used for statistics and to
//! detect sleep/hibernate gaps (a running phase is auto-paused instead of silently consuming the
//! time the computer was asleep).
//!
//! Meeting holds ([`MeetingHold`], set with [`Input::SetMeetingHolds`]) change only what happens
//! when a focus deadline is reached on a [`Input::Tick`] (never on Skip, Pause, Resume or Stop),
//! and each hold applies at most once per session:
//! - `Extend`: a hold overlapping the break that would start now (`[now, now + break)`) keeps the
//!   focus running until the meeting ends (no effects; the snapshot shows `meeting_hold` and a longer
//!   `phase_total_secs`). Music is muted (`music_muted`) during an `Extend` meeting in Focus.
//! - `PauseAtBreak`: such a hold starts the break paused with its full length frozen (effects of a
//!   normal Focus → Break, with `Notify(BreakHeld)` instead of `Notify(BreakStarted)`); the snapshot
//!   shows `meeting_hold` until Resume (or until the break ends by Skip/Stop).
//! - Meetings longer than [`MAX_MEETING_HOLD_MS`] (4 h) are ignored: they never extend the focus, hold the
//!   break or mute the music.

use crate::api::{
    ChimeKind, Meeting, MeetingChoice, MeetingHoldView, Phase, PhaseChange, SessionSnapshot,
};
use crate::calendar::meeting_hold_allowed;
use crate::timing::{MAX_MEETING_HOLD_MS, TimingProfile};

const MS_PER_SEC: u64 = 1000;

/// A point in time supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    /// Monotonic milliseconds (any origin); phase deadlines use this clock.
    pub mono_ms: u64,
    /// Wall clock, Unix epoch milliseconds; statistics and gap detection.
    pub wall_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Starts focus cycle 1 (no-op unless idle).
    Start {
        session_id: String,
        profile: TimingProfile,
    },
    /// Freezes the remaining time of the running phase.
    Pause,
    /// Continues a paused phase.
    Resume,
    /// Ends the session from any phase; idempotent.
    Stop,
    /// Focus → Break now, Break → next Focus now (also from a paused phase).
    SkipPhase,
    /// Periodic clock tick (the shell sends one per second).
    Tick,
    /// Replaces all meeting holds (idle or running; they survive Start and Stop). Only `Extend` and
    /// `PauseAtBreak` holds with `end > start` and a meeting of at most [`MAX_MEETING_HOLD_MS`] are kept.
    /// Returns no effects; only the snapshot changes.
    /// A focus already extended by a hold stays extended even when that hold is removed.
    SetMeetingHolds(Vec<MeetingHold>),
}

/// A meeting the user chose to handle with `Extend` or `PauseAtBreak` (see the module docs).
///
/// The shell sends every meeting that has one of these choices (not only the ones colliding right
/// now) and re-sends them after every calendar sync, choice change and Resume, so the monotonic
/// times stay close to the wall clock ([`MeetingHold::from_meeting`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingHold {
    /// `Meeting.id`.
    pub id: String,
    pub subject: String,
    /// Meeting start / end on the monotonic clock of [`Now::mono_ms`].
    pub start_mono_ms: u64,
    pub end_mono_ms: u64,
    /// Meeting start / end, Unix epoch milliseconds (shown in the snapshot).
    pub start_wall_ms: i64,
    pub end_wall_ms: i64,
    /// `Extend` or `PauseAtBreak`; other choices are dropped by [`Input::SetMeetingHolds`].
    pub choice: MeetingChoice,
}

impl MeetingHold {
    /// The hold for `meeting` with `choice`, its monotonic times derived from `now`
    /// (`mono = now.mono_ms + (wall - now.wall_ms)`, saturating). `None` for `Ignore` /
    /// `RemindLater`, for a meeting without length (also after saturation) and for a meeting longer
    /// than [`MAX_MEETING_HOLD_MS`].
    pub fn from_meeting(meeting: &Meeting, choice: MeetingChoice, now: Now) -> Option<Self> {
        let hold = Self {
            id: meeting.id.clone(),
            subject: meeting.subject.clone(),
            start_mono_ms: wall_to_mono(meeting.start_ms, now),
            end_mono_ms: wall_to_mono(meeting.end_ms, now),
            start_wall_ms: meeting.start_ms,
            end_wall_ms: meeting.end_ms,
            choice,
        };
        hold.is_usable().then_some(hold)
    }

    /// `Extend` / `PauseAtBreak` for a meeting with a length of at most [`MAX_MEETING_HOLD_MS`] on both clocks
    /// ([`meeting_hold_allowed`]): a longer meeting never extends the focus or holds the break.
    fn is_usable(&self) -> bool {
        matches!(
            self.choice,
            MeetingChoice::Extend | MeetingChoice::PauseAtBreak
        ) && self.start_mono_ms < self.end_mono_ms
            && self.end_mono_ms - self.start_mono_ms <= MAX_MEETING_HOLD_MS
            && meeting_hold_allowed(self.start_wall_ms, self.end_wall_ms)
    }

    /// `[start, end)` of the meeting overlaps `[from_ms, to_ms)` (monotonic); touching edges do not count.
    fn overlaps(&self, from_ms: u64, to_ms: u64) -> bool {
        self.start_mono_ms < to_ms && from_ms < self.end_mono_ms
    }

    /// The meeting runs at `at_ms` (monotonic).
    fn covers(&self, at_ms: u64) -> bool {
        self.start_mono_ms <= at_ms && at_ms < self.end_mono_ms
    }

    fn view(&self) -> MeetingHoldView {
        MeetingHoldView {
            subject: self.subject.clone(),
            until_ms: self.end_wall_ms,
            choice: self.choice,
        }
    }
}

/// Toast notifications requested by the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyKind {
    /// Focus ended; the break lasts `break_secs`.
    BreakStarted { break_secs: u32 },
    /// Break ended; focus cycle `cycle` lasts `focus_secs`.
    FocusStarted { focus_secs: u32, cycle: u32 },
    /// A sleep/clock gap was detected and the session was paused automatically.
    Interrupted,
    /// Focus ended during a `PauseAtBreak` meeting: the break started paused (full length) and waits
    /// for Resume. `subject` is the meeting subject (`None` when it is empty).
    BreakHeld { subject: Option<String> },
}

/// Totals of an ended session (input for `stats::summarize`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTotals {
    pub session_id: String,
    /// Wall clock at Start, Unix epoch milliseconds.
    pub started_at_ms: i64,
    /// Wall clock at Stop, Unix epoch milliseconds.
    pub ended_at_ms: i64,
    /// Running focus time (no pauses, no gaps).
    pub focused_secs: u32,
    /// Running break time (no pauses, no gaps).
    pub break_secs: u32,
    /// Focus phases that reached their end or were skipped into a break.
    pub cycles_completed: u32,
    pub demo: bool,
}

/// Side effects the shell executes, in the order given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Play a chime.
    Chime(ChimeKind),
    /// Show a toast notification.
    Notify(NotifyKind),
    /// `true` = raise the focus shields, `false` = lower them. Idempotent for the executor.
    Shields(bool),
    /// Foreground-app sampling on/off (on only while Focus is running, not paused).
    Tracking(bool),
    /// Emit `session://phase-changed`. For transitions to Idle `cycle` is the last cycle of the
    /// ended session; otherwise it is the cycle the new phase belongs to.
    PhaseChanged(PhaseChange),
    /// The session ended (exactly once per session).
    SessionEnded(SessionTotals),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Focus,
    Break,
}

impl Stage {
    fn phase(self) -> Phase {
        match self {
            Stage::Focus => Phase::Focus,
            Stage::Break => Phase::Break,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clock {
    Running { deadline_ms: u64 },
    Paused { remaining_ms: u64 },
}

/// Result of accounting running time up to an observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Advance {
    /// Time was accounted (or there was nothing to account).
    Counted,
    /// A sleep/clock gap was detected; nothing was accounted and `last_seen` was kept.
    Gap,
}

#[derive(Debug, Clone)]
struct Active {
    session_id: String,
    profile: TimingProfile,
    stage: Stage,
    /// 1-based focus cycle; a break belongs to the cycle of the focus before it.
    cycle: u32,
    clock: Clock,
    /// Last trusted observation of a running phase; elapsed time is measured from here.
    last_seen: Now,
    started_at_ms: i64,
    focused_ms: u64,
    break_ms: u64,
    cycles_completed: u32,
    /// Time added to the current focus phase by `Extend` holds (0 otherwise).
    extra_ms: u64,
    /// The hold that extends the current focus or holds the current break. Cleared with the
    /// phase, and for a held break also on Resume.
    hold: Option<MeetingHold>,
    /// Ids of the holds already applied in this session (each applies at most once).
    used_holds: Vec<String>,
}

impl Active {
    fn is_running(&self) -> bool {
        matches!(self.clock, Clock::Running { .. })
    }

    /// Length of the current phase in seconds, including a meeting extension.
    fn phase_total_secs(&self) -> u32 {
        let base_ms = match self.stage {
            Stage::Focus => self.profile.focus_ms(),
            Stage::Break => self.profile.break_ms(),
        };
        ms_to_secs_ceil(base_ms.saturating_add(self.extra_ms))
    }

    /// Music must be silent: Focus (running or paused) during an `Extend` meeting, either one of
    /// `holds` or the one extending this focus.
    fn music_muted(&self, holds: &[MeetingHold], now: Now) -> bool {
        self.stage == Stage::Focus
            && holds
                .iter()
                .chain(self.hold.iter())
                .any(|hold| hold.choice == MeetingChoice::Extend && hold.covers(now.mono_ms))
    }

    fn is_gap(&self, now: Now) -> bool {
        is_gap(self.last_seen, now, self.profile.interruption_gap_ms())
    }

    /// Accounts running time since `last_seen` to the current stage. Does nothing while paused.
    fn advance(&mut self, now: Now) -> Advance {
        if !self.is_running() {
            return Advance::Counted;
        }
        if self.is_gap(now) {
            return Advance::Gap;
        }
        let elapsed = now.mono_ms.saturating_sub(self.last_seen.mono_ms);
        match self.stage {
            Stage::Focus => self.focused_ms = self.focused_ms.saturating_add(elapsed),
            Stage::Break => self.break_ms = self.break_ms.saturating_add(elapsed),
        }
        // A monotonic clock never goes back; if it did, keep the later point so nothing is counted twice.
        if now.mono_ms >= self.last_seen.mono_ms {
            self.last_seen = now;
        }
        Advance::Counted
    }

    /// Pauses a running phase with the time left at `last_seen` (call [`Active::advance`] first).
    fn freeze(&mut self) {
        if let Clock::Running { deadline_ms } = self.clock {
            self.clock = Clock::Paused {
                remaining_ms: deadline_ms.saturating_sub(self.last_seen.mono_ms),
            };
        }
    }

    fn remaining_ms(&self, now: Now) -> u64 {
        match self.clock {
            Clock::Paused { remaining_ms } => remaining_ms,
            Clock::Running { deadline_ms } => {
                let reference = if self.is_gap(now) {
                    self.last_seen.mono_ms
                } else {
                    now.mono_ms.max(self.last_seen.mono_ms)
                };
                deadline_ms.saturating_sub(reference)
            }
        }
    }

    /// Focus time including the not yet accounted part of a running focus phase.
    fn focused_ms_at(&self, now: Now) -> u64 {
        let pending = if self.stage == Stage::Focus && self.is_running() && !self.is_gap(now) {
            now.mono_ms.saturating_sub(self.last_seen.mono_ms)
        } else {
            0
        };
        self.focused_ms.saturating_add(pending)
    }

    fn tracking_off_if_focus(&self) -> Vec<Effect> {
        if self.stage == Stage::Focus {
            vec![Effect::Tracking(false)]
        } else {
            Vec::new()
        }
    }

    /// Focus → Break (deadline reached or skipped). Shields go down before the toast.
    fn enter_break(&mut self, now: Now) -> Vec<Effect> {
        let clock = Clock::Running {
            deadline_ms: now.mono_ms.saturating_add(self.profile.break_ms()),
        };
        self.begin_break(now, clock);
        self.break_effects(NotifyKind::BreakStarted {
            break_secs: self.profile.break_secs,
        })
    }

    /// Focus → Break during a `PauseAtBreak` meeting: the break starts paused with its full length.
    /// Same effects as [`Active::enter_break`], with `BreakHeld` instead of `BreakStarted`.
    fn enter_held_break(&mut self, now: Now, hold: MeetingHold) -> Vec<Effect> {
        let clock = Clock::Paused {
            remaining_ms: self.profile.break_ms(),
        };
        self.begin_break(now, clock);
        let subject = Some(hold.subject.trim())
            .filter(|subject| !subject.is_empty())
            .map(str::to_owned);
        self.used_holds.push(hold.id.clone());
        self.hold = Some(hold);
        self.break_effects(NotifyKind::BreakHeld { subject })
    }

    fn begin_break(&mut self, now: Now, clock: Clock) {
        self.cycles_completed = self.cycles_completed.saturating_add(1);
        self.stage = Stage::Break;
        self.clock = clock;
        self.last_seen = now;
        self.extra_ms = 0;
        self.hold = None;
    }

    fn break_effects(&self, notify: NotifyKind) -> Vec<Effect> {
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Focus,
                to: Phase::Break,
                cycle: self.cycle,
            }),
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::Chime(ChimeKind::BreakStart),
            Effect::Notify(notify),
        ]
    }

    /// Keeps the running focus going until the end of an `Extend` meeting (`deadline_ms` is the
    /// deadline that was just reached). No effects.
    fn extend_focus(&mut self, deadline_ms: u64, hold: MeetingHold) {
        let new_deadline = hold.end_mono_ms.max(deadline_ms);
        self.extra_ms = self
            .extra_ms
            .saturating_add(new_deadline.saturating_sub(deadline_ms));
        self.clock = Clock::Running {
            deadline_ms: new_deadline,
        };
        self.used_holds.push(hold.id.clone());
        self.hold = Some(hold);
    }

    /// Break → next Focus (deadline reached or skipped). The toast comes before shields go up.
    fn enter_focus(&mut self, now: Now) -> Vec<Effect> {
        self.cycle = self.cycle.saturating_add(1);
        self.stage = Stage::Focus;
        self.clock = Clock::Running {
            deadline_ms: now.mono_ms.saturating_add(self.profile.focus_ms()),
        };
        self.last_seen = now;
        self.extra_ms = 0;
        self.hold = None;
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Break,
                to: Phase::Focus,
                cycle: self.cycle,
            }),
            Effect::Chime(ChimeKind::FocusStart),
            Effect::Notify(NotifyKind::FocusStarted {
                focus_secs: self.profile.focus_secs,
                cycle: self.cycle,
            }),
            Effect::Shields(true),
            Effect::Tracking(true),
        ]
    }

    fn next_phase(&mut self, now: Now) -> Vec<Effect> {
        match self.stage {
            Stage::Focus => self.enter_break(now),
            Stage::Break => self.enter_focus(now),
        }
    }

    fn into_totals(self, now: Now) -> SessionTotals {
        SessionTotals {
            session_id: self.session_id,
            started_at_ms: self.started_at_ms,
            ended_at_ms: now.wall_ms,
            focused_secs: ms_to_secs_floor(self.focused_ms),
            break_secs: ms_to_secs_floor(self.break_ms),
            cycles_completed: self.cycles_completed,
            demo: self.profile.demo,
        }
    }
}

/// The focus session state machine. See the module docs and SPEC (session.rs table).
#[derive(Debug, Clone)]
pub struct Session {
    active: Option<Active>,
    /// Profile reported by idle snapshots (`focus_secs`, `break_secs`, `demo`).
    idle_profile: TimingProfile,
    /// Meeting holds (only usable `Extend` / `PauseAtBreak` ones), independent of the running session.
    holds: Vec<MeetingHold>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// An idle session.
    pub fn new() -> Self {
        Self {
            active: None,
            idle_profile: TimingProfile::normal(),
            holds: Vec::new(),
        }
    }

    /// Sets the profile shown by idle snapshots (e.g. the demo profile, or the profile derived
    /// from a changed config). Does not affect a running session.
    pub fn set_idle_profile(&mut self, profile: TimingProfile) {
        self.idle_profile = sanitize(profile);
    }

    /// Applies one input and returns the effects to execute, in order.
    pub fn handle(&mut self, now: Now, input: Input) -> Vec<Effect> {
        match input {
            Input::Start {
                session_id,
                profile,
            } => self.start(now, session_id, profile),
            Input::Pause => self.pause(now),
            Input::Resume => self.resume(now),
            Input::Stop => self.stop(now),
            Input::SkipPhase => self.skip(now),
            Input::Tick => self.tick(now),
            Input::SetMeetingHolds(holds) => {
                self.holds = holds.into_iter().filter(MeetingHold::is_usable).collect();
                Vec::new()
            }
        }
    }

    /// View state at `now`. `revision` is 0; the shell sets it.
    pub fn snapshot(&self, now: Now) -> SessionSnapshot {
        match &self.active {
            None => SessionSnapshot {
                revision: 0,
                session_id: None,
                phase: Phase::Idle,
                paused: false,
                cycle: 0,
                remaining_secs: 0,
                phase_total_secs: 0,
                focus_secs: self.idle_profile.focus_secs,
                break_secs: self.idle_profile.break_secs,
                focused_secs: 0,
                demo: self.idle_profile.demo,
                music_muted: false,
                meeting_hold: None,
            },
            Some(active) => SessionSnapshot {
                revision: 0,
                session_id: Some(active.session_id.clone()),
                phase: active.stage.phase(),
                paused: !active.is_running(),
                cycle: active.cycle,
                remaining_secs: ms_to_secs_ceil(active.remaining_ms(now)),
                phase_total_secs: active.phase_total_secs(),
                focus_secs: active.profile.focus_secs,
                break_secs: active.profile.break_secs,
                focused_secs: ms_to_secs_floor(active.focused_ms_at(now)),
                demo: active.profile.demo,
                music_muted: active.music_muted(&self.holds, now),
                meeting_hold: active.hold.as_ref().map(MeetingHold::view),
            },
        }
    }

    /// Exact time left in the current phase at `now` (frozen while paused); `None` when idle.
    /// Input for [`crate::calendar::plan_breaks`].
    pub fn remaining_ms(&self, now: Now) -> Option<u64> {
        self.active.as_ref().map(|active| active.remaining_ms(now))
    }

    /// The meeting holds in use (after filtering by [`Input::SetMeetingHolds`]).
    pub fn meeting_holds(&self) -> &[MeetingHold] {
        &self.holds
    }

    /// Current phase (`Idle` when no session runs).
    pub fn phase(&self) -> Phase {
        self.active
            .as_ref()
            .map_or(Phase::Idle, |active| active.stage.phase())
    }

    /// Paused by the user or by a detected gap (always `false` when idle).
    pub fn is_paused(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| !active.is_running())
    }

    /// Id of the running session.
    pub fn session_id(&self) -> Option<&str> {
        self.active
            .as_ref()
            .map(|active| active.session_id.as_str())
    }

    /// Profile of the running session.
    pub fn profile(&self) -> Option<TimingProfile> {
        self.active.as_ref().map(|active| active.profile)
    }

    /// The shields should be up: Focus, running or paused.
    pub fn wants_shields(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.stage == Stage::Focus)
    }

    fn start(&mut self, now: Now, session_id: String, profile: TimingProfile) -> Vec<Effect> {
        if self.active.is_some() {
            return Vec::new();
        }
        let profile = sanitize(profile);
        self.active = Some(Active {
            session_id,
            profile,
            stage: Stage::Focus,
            cycle: 1,
            clock: Clock::Running {
                deadline_ms: now.mono_ms.saturating_add(profile.focus_ms()),
            },
            last_seen: now,
            started_at_ms: now.wall_ms,
            focused_ms: 0,
            break_ms: 0,
            cycles_completed: 0,
            extra_ms: 0,
            hold: None,
            used_holds: Vec::new(),
        });
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Idle,
                to: Phase::Focus,
                cycle: 1,
            }),
            Effect::Shields(true),
            Effect::Tracking(true),
            Effect::Chime(ChimeKind::FocusStart),
        ]
    }

    fn pause(&mut self, now: Now) -> Vec<Effect> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        if !active.is_running() {
            return Vec::new();
        }
        // A gap before an explicit pause is simply not counted; no "interrupted" toast needed.
        active.advance(now);
        active.freeze();
        active.tracking_off_if_focus()
    }

    fn resume(&mut self, now: Now) -> Vec<Effect> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        let Clock::Paused { remaining_ms } = active.clock else {
            return Vec::new();
        };
        active.clock = Clock::Running {
            deadline_ms: now.mono_ms.saturating_add(remaining_ms),
        };
        active.last_seen = now;
        if active
            .hold
            .as_ref()
            .is_some_and(|hold| hold.choice == MeetingChoice::PauseAtBreak)
        {
            // The held break now runs; an extended focus keeps its hold until it ends.
            active.hold = None;
        }
        if active.stage == Stage::Focus {
            vec![Effect::Tracking(true)]
        } else {
            Vec::new()
        }
    }

    fn stop(&mut self, now: Now) -> Vec<Effect> {
        let Some(mut active) = self.active.take() else {
            return Vec::new();
        };
        active.advance(now);
        let change = PhaseChange {
            from: active.stage.phase(),
            to: Phase::Idle,
            cycle: active.cycle,
        };
        vec![
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::PhaseChanged(change),
            Effect::SessionEnded(active.into_totals(now)),
        ]
    }

    fn skip(&mut self, now: Now) -> Vec<Effect> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        active.advance(now);
        active.next_phase(now)
    }

    fn tick(&mut self, now: Now) -> Vec<Effect> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        let Clock::Running { deadline_ms } = active.clock else {
            return Vec::new();
        };
        if active.advance(now) == Advance::Gap {
            active.freeze();
            let mut effects = active.tracking_off_if_focus();
            effects.push(Effect::Notify(NotifyKind::Interrupted));
            return effects;
        }
        if now.mono_ms < deadline_ms {
            return Vec::new();
        }
        if active.stage == Stage::Break {
            return active.enter_focus(now);
        }
        // The break that would start now: [now, now + break). `Extend` wins over `PauseAtBreak`.
        let window_end = now.mono_ms.saturating_add(active.profile.break_ms());
        let pick = |choice| {
            pick_hold(
                &self.holds,
                &active.used_holds,
                choice,
                now.mono_ms,
                window_end,
            )
        };
        if let Some(hold) = pick(MeetingChoice::Extend).cloned() {
            active.extend_focus(deadline_ms, hold);
            return Vec::new();
        }
        if let Some(hold) = pick(MeetingChoice::PauseAtBreak).cloned() {
            return active.enter_held_break(now, hold);
        }
        active.enter_break(now)
    }
}

/// The unused hold with `choice` that overlaps `[from_ms, to_ms)`; the one that ends last when
/// several do.
fn pick_hold<'a>(
    holds: &'a [MeetingHold],
    used: &[String],
    choice: MeetingChoice,
    from_ms: u64,
    to_ms: u64,
) -> Option<&'a MeetingHold> {
    holds
        .iter()
        .filter(|hold| {
            hold.choice == choice && hold.overlaps(from_ms, to_ms) && !used.contains(&hold.id)
        })
        .max_by_key(|hold| hold.end_mono_ms)
}

/// `wall_ms` on the monotonic clock of `now` (saturating at 0 and `u64::MAX`).
fn wall_to_mono(wall_ms: i64, now: Now) -> u64 {
    let delta = wall_ms.saturating_sub(now.wall_ms);
    match u64::try_from(delta) {
        Ok(ahead) => now.mono_ms.saturating_add(ahead),
        Err(_) => now.mono_ms.saturating_sub(delta.unsigned_abs()),
    }
}

/// A phase never has zero length (it would flip on every tick).
fn sanitize(profile: TimingProfile) -> TimingProfile {
    TimingProfile {
        focus_secs: profile.focus_secs.max(1),
        break_secs: profile.break_secs.max(1),
        ..profile
    }
}

/// `true` when the time between two observations cannot be trusted: the monotonic clock jumped
/// by more than `gap_ms`, or the wall clock advanced more than `gap_ms` beyond the monotonic one
/// (monotonic clocks that stop during sleep). `gap_ms == 0` disables detection.
fn is_gap(last: Now, now: Now, gap_ms: u64) -> bool {
    if gap_ms == 0 {
        return false;
    }
    let mono_delta = now.mono_ms.saturating_sub(last.mono_ms);
    if mono_delta > gap_ms {
        return true;
    }
    let wall_delta = now.wall_ms.saturating_sub(last.wall_ms);
    let mono_delta = i64::try_from(mono_delta).unwrap_or(i64::MAX);
    let gap_ms = i64::try_from(gap_ms).unwrap_or(i64::MAX);
    wall_delta.saturating_sub(mono_delta) > gap_ms
}

fn ms_to_secs_floor(ms: u64) -> u32 {
    u32::try_from(ms / MS_PER_SEC).unwrap_or(u32::MAX)
}

fn ms_to_secs_ceil(ms: u64) -> u32 {
    u32::try_from(ms.div_ceil(MS_PER_SEC)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL0: i64 = 1_760_000_000_000;

    fn at_ms(ms: u64) -> Now {
        Now {
            mono_ms: ms,
            wall_ms: WALL0 + i64::try_from(ms).expect("test time fits in i64"),
        }
    }

    fn at(secs: u64) -> Now {
        at_ms(secs * 1000)
    }

    fn start_input(profile: TimingProfile) -> Input {
        Input::Start {
            session_id: "s1".to_owned(),
            profile,
        }
    }

    fn started(profile: TimingProfile) -> Session {
        let mut session = Session::new();
        session.handle(at(0), start_input(profile));
        session
    }

    /// Ticks once per second for every second in `(from, to]` and collects the effects.
    fn tick_through(session: &mut Session, from: u64, to: u64) -> Vec<Effect> {
        (from + 1..=to)
            .flat_map(|t| session.handle(at(t), Input::Tick))
            .collect()
    }

    fn ended_totals(effects: &[Effect]) -> Vec<SessionTotals> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::SessionEnded(totals) => Some(totals.clone()),
                _ => None,
            })
            .collect()
    }

    fn only_totals(effects: &[Effect]) -> SessionTotals {
        let mut all = ended_totals(effects);
        assert_eq!(all.len(), 1, "exactly one SessionEnded in {effects:?}");
        all.remove(0)
    }

    fn break_transition(cycle: u32, break_secs: u32) -> Vec<Effect> {
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Focus,
                to: Phase::Break,
                cycle,
            }),
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::Chime(ChimeKind::BreakStart),
            Effect::Notify(NotifyKind::BreakStarted { break_secs }),
        ]
    }

    fn focus_transition(cycle: u32, focus_secs: u32) -> Vec<Effect> {
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Break,
                to: Phase::Focus,
                cycle,
            }),
            Effect::Chime(ChimeKind::FocusStart),
            Effect::Notify(NotifyKind::FocusStarted { focus_secs, cycle }),
            Effect::Shields(true),
            Effect::Tracking(true),
        ]
    }

    fn stop_effects(from: Phase, cycle: u32, totals: SessionTotals) -> Vec<Effect> {
        vec![
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::PhaseChanged(PhaseChange {
                from,
                to: Phase::Idle,
                cycle,
            }),
            Effect::SessionEnded(totals),
        ]
    }

    #[test]
    fn idle_snapshot_reports_the_idle_profile() {
        let mut session = Session::new();
        let snap = session.snapshot(at(5));
        assert_eq!(
            snap,
            SessionSnapshot {
                revision: 0,
                session_id: None,
                phase: Phase::Idle,
                paused: false,
                cycle: 0,
                remaining_secs: 0,
                phase_total_secs: 0,
                focus_secs: 2700,
                break_secs: 900,
                focused_secs: 0,
                demo: false,
                music_muted: false,
                meeting_hold: None,
            }
        );
        assert_eq!(session.phase(), Phase::Idle);
        assert!(!session.is_paused());
        assert!(!session.wants_shields());
        assert_eq!(session.session_id(), None);
        assert_eq!(session.profile(), None);

        session.set_idle_profile(TimingProfile::demo());
        let snap = session.snapshot(at(5));
        assert_eq!(
            (snap.focus_secs, snap.break_secs, snap.demo),
            (40, 20, true)
        );
        assert_eq!(Session::default().snapshot(at(0)).focus_secs, 2700);
    }

    #[test]
    fn inputs_while_idle_are_no_ops() {
        let mut session = Session::new();
        for input in [
            Input::Pause,
            Input::Resume,
            Input::Stop,
            Input::SkipPhase,
            Input::Tick,
        ] {
            assert!(session.handle(at(1), input).is_empty());
        }
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn start_enters_focus_with_effects_in_order() {
        let mut session = Session::new();
        let effects = session.handle(at(0), start_input(TimingProfile::demo()));
        assert_eq!(
            effects,
            vec![
                Effect::PhaseChanged(PhaseChange {
                    from: Phase::Idle,
                    to: Phase::Focus,
                    cycle: 1,
                }),
                Effect::Shields(true),
                Effect::Tracking(true),
                Effect::Chime(ChimeKind::FocusStart),
            ]
        );
        let snap = session.snapshot(at(0));
        assert_eq!(snap.session_id.as_deref(), Some("s1"));
        assert_eq!(snap.phase, Phase::Focus);
        assert_eq!(snap.cycle, 1);
        assert_eq!(snap.remaining_secs, 40);
        assert_eq!(snap.phase_total_secs, 40);
        assert_eq!((snap.focus_secs, snap.break_secs), (40, 20));
        assert!(snap.demo);
        assert!(!snap.paused);
        assert!(session.wants_shields());
        assert_eq!(session.profile(), Some(TimingProfile::demo()));
    }

    #[test]
    fn start_is_idempotent() {
        let mut session = started(TimingProfile::demo());
        let again = session.handle(
            at(3),
            Input::Start {
                session_id: "other".to_owned(),
                profile: TimingProfile::normal(),
            },
        );
        assert!(again.is_empty());
        assert_eq!(session.session_id(), Some("s1"));
        assert_eq!(session.snapshot(at(3)).focus_secs, 40);
    }

    #[test]
    fn demo_profile_loops_at_exact_boundaries() {
        let mut session = started(TimingProfile::demo());

        assert!(tick_through(&mut session, 0, 39).is_empty());
        assert_eq!(session.phase(), Phase::Focus);
        assert_eq!(session.snapshot(at(39)).remaining_secs, 1);
        assert_eq!(session.snapshot(at(39)).focused_secs, 39);

        assert_eq!(session.handle(at(40), Input::Tick), break_transition(1, 20));
        let snap = session.snapshot(at(40));
        assert_eq!(snap.phase, Phase::Break);
        assert_eq!(snap.cycle, 1);
        assert_eq!(snap.remaining_secs, 20);
        assert_eq!(snap.phase_total_secs, 20);
        assert_eq!(snap.focused_secs, 40);
        assert!(!session.wants_shields());

        assert!(tick_through(&mut session, 40, 59).is_empty());
        assert_eq!(session.snapshot(at(59)).remaining_secs, 1);
        assert_eq!(session.handle(at(60), Input::Tick), focus_transition(2, 40));
        let snap = session.snapshot(at(60));
        assert_eq!(
            (snap.phase, snap.cycle, snap.remaining_secs),
            (Phase::Focus, 2, 40)
        );

        assert!(tick_through(&mut session, 60, 99).is_empty());
        assert_eq!(
            session.handle(at(100), Input::Tick),
            break_transition(2, 20)
        );
        assert!(tick_through(&mut session, 100, 110).is_empty());

        let effects = session.handle(at(110), Input::Stop);
        let expected = SessionTotals {
            session_id: "s1".to_owned(),
            started_at_ms: WALL0,
            ended_at_ms: WALL0 + 110_000,
            focused_secs: 80,
            break_secs: 30,
            cycles_completed: 2,
            demo: true,
        };
        assert_eq!(effects, stop_effects(Phase::Break, 2, expected));
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn late_tick_still_transitions_once() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 38);
        // The tick for second 39 and 40 got merged into one late tick.
        assert_eq!(
            session.handle(at_ms(41_500), Input::Tick),
            break_transition(1, 20)
        );
        // The break is measured from the transition.
        assert_eq!(session.snapshot(at_ms(41_500)).remaining_secs, 20);
        assert!(session.handle(at_ms(61_499), Input::Tick).is_empty());
        assert_eq!(
            session.handle(at_ms(61_500), Input::Tick),
            focus_transition(2, 40)
        );
    }

    #[test]
    fn pause_freezes_remaining_time() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);

        assert_eq!(
            session.handle(at_ms(10_500), Input::Pause),
            vec![Effect::Tracking(false)]
        );
        assert!(session.is_paused());
        assert!(
            session.wants_shields(),
            "shields stay up while focus is paused"
        );
        assert_eq!(session.snapshot(at_ms(10_500)).remaining_secs, 30);

        assert!(session.handle(at(11), Input::Pause).is_empty());
        assert!(tick_through(&mut session, 11, 300).is_empty());

        let snap = session.snapshot(at(300));
        assert!(snap.paused);
        assert_eq!(snap.phase, Phase::Focus);
        assert_eq!(snap.remaining_secs, 30);
        assert_eq!(snap.focused_secs, 10);
    }

    #[test]
    fn resume_shifts_the_deadline() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        session.handle(at_ms(10_500), Input::Pause);

        assert_eq!(
            session.handle(at(200), Input::Resume),
            vec![Effect::Tracking(true)]
        );
        assert!(session.handle(at(201), Input::Resume).is_empty());
        assert!(!session.is_paused());

        // 29.5 s were left, so the new deadline is 229.5 s.
        assert!(tick_through(&mut session, 200, 229).is_empty());
        assert_eq!(session.snapshot(at(229)).remaining_secs, 1);
        assert!(session.handle(at_ms(229_499), Input::Tick).is_empty());
        assert_eq!(
            session.handle(at_ms(229_500), Input::Tick),
            break_transition(1, 20)
        );
        assert_eq!(session.snapshot(at_ms(229_500)).focused_secs, 40);
    }

    #[test]
    fn pause_and_resume_in_break_do_not_touch_tracking() {
        let mut session = started(TimingProfile::demo());
        session.handle(at(1), Input::SkipPhase);
        assert_eq!(session.phase(), Phase::Break);

        assert!(session.handle(at(5), Input::Pause).is_empty());
        assert!(session.is_paused());
        assert_eq!(session.snapshot(at(50)).remaining_secs, 16);
        assert!(tick_through(&mut session, 5, 60).is_empty());

        assert!(session.handle(at(60), Input::Resume).is_empty());
        assert!(!session.is_paused());
        assert!(tick_through(&mut session, 60, 75).is_empty());
        assert_eq!(session.handle(at(76), Input::Tick), focus_transition(2, 40));
    }

    #[test]
    fn focused_time_excludes_pauses() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        session.handle(at(10), Input::Pause);
        session.handle(at(50), Input::Resume);
        tick_through(&mut session, 50, 60);
        assert_eq!(session.snapshot(at(60)).focused_secs, 20);

        let totals = only_totals(&session.handle(at(60), Input::Stop));
        assert_eq!(totals.focused_secs, 20);
        assert_eq!(totals.break_secs, 0);
        assert_eq!(totals.cycles_completed, 0);
    }

    #[test]
    fn stop_in_running_focus() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 12);
        let effects = session.handle(at_ms(12_700), Input::Stop);
        let expected = SessionTotals {
            session_id: "s1".to_owned(),
            started_at_ms: WALL0,
            ended_at_ms: WALL0 + 12_700,
            focused_secs: 12,
            break_secs: 0,
            cycles_completed: 0,
            demo: true,
        };
        assert_eq!(effects, stop_effects(Phase::Focus, 1, expected));
        assert_eq!(session.phase(), Phase::Idle);
        assert!(!session.wants_shields());
        assert_eq!(session.snapshot(at(13)).phase, Phase::Idle);
    }

    #[test]
    fn stop_in_paused_focus() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 5);
        session.handle(at(5), Input::Pause);
        let effects = session.handle(at(80), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(totals.focused_secs, 5);
        assert_eq!(totals.ended_at_ms, WALL0 + 80_000);
        assert_eq!(effects, stop_effects(Phase::Focus, 1, totals));
    }

    #[test]
    fn stop_in_running_and_paused_break() {
        let mut running = started(TimingProfile::demo());
        tick_through(&mut running, 0, 45);
        let effects = running.handle(at(45), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(
            (
                totals.focused_secs,
                totals.break_secs,
                totals.cycles_completed
            ),
            (40, 5, 1)
        );
        assert_eq!(effects, stop_effects(Phase::Break, 1, totals));

        let mut paused = started(TimingProfile::demo());
        tick_through(&mut paused, 0, 45);
        paused.handle(at(45), Input::Pause);
        let effects = paused.handle(at(70), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!((totals.focused_secs, totals.break_secs), (40, 5));
        assert_eq!(effects, stop_effects(Phase::Break, 1, totals));
    }

    #[test]
    fn double_stop_ends_the_session_once() {
        let mut session = started(TimingProfile::demo());
        let mut effects = session.handle(at(3), Input::Stop);
        effects.extend(session.handle(at(4), Input::Stop));
        effects.extend(session.handle(at(5), Input::Stop));
        assert_eq!(ended_totals(&effects).len(), 1);
        assert_eq!(effects.len(), 4);
    }

    #[test]
    fn skip_from_focus_counts_a_completed_cycle() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 5);
        assert_eq!(
            session.handle(at(5), Input::SkipPhase),
            break_transition(1, 20)
        );
        let snap = session.snapshot(at(5));
        assert_eq!(
            (snap.phase, snap.cycle, snap.remaining_secs),
            (Phase::Break, 1, 20)
        );

        let totals = only_totals(&session.handle(at(6), Input::Stop));
        assert_eq!(totals.cycles_completed, 1);
        assert_eq!(totals.focused_secs, 5);
        assert_eq!(totals.break_secs, 1);
    }

    #[test]
    fn skip_from_break_starts_the_next_cycle() {
        let mut session = started(TimingProfile::demo());
        session.handle(at(2), Input::SkipPhase);
        assert_eq!(
            session.handle(at(4), Input::SkipPhase),
            focus_transition(2, 40)
        );
        let snap = session.snapshot(at(4));
        assert_eq!(
            (snap.phase, snap.cycle, snap.remaining_secs),
            (Phase::Focus, 2, 40)
        );
        assert!(session.wants_shields());
    }

    #[test]
    fn skip_from_paused_phase_runs_the_next_phase() {
        let mut session = started(TimingProfile::demo());
        session.handle(at(3), Input::Pause);
        assert_eq!(
            session.handle(at(30), Input::SkipPhase),
            break_transition(1, 20)
        );
        assert!(!session.is_paused());
        assert_eq!(session.snapshot(at(30)).remaining_secs, 20);
        // Paused time is not focus time.
        assert_eq!(session.snapshot(at(30)).focused_secs, 3);

        session.handle(at(31), Input::Pause);
        assert_eq!(
            session.handle(at(90), Input::SkipPhase),
            focus_transition(2, 40)
        );
        assert!(!session.is_paused());
    }

    #[test]
    fn monotonic_gap_auto_pauses_focus() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);

        let effects = session.handle(at(10 + 91), Input::Tick);
        assert_eq!(
            effects,
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        assert!(session.is_paused());
        let snap = session.snapshot(at(101));
        assert_eq!(snap.remaining_secs, 30, "gap time is not consumed");
        assert_eq!(snap.focused_secs, 10, "gap time is not focus time");
        assert!(tick_through(&mut session, 101, 120).is_empty());

        assert_eq!(
            session.handle(at(150), Input::Resume),
            vec![Effect::Tracking(true)]
        );
        assert!(tick_through(&mut session, 150, 179).is_empty());
        assert_eq!(
            session.handle(at(180), Input::Tick),
            break_transition(1, 20)
        );
    }

    #[test]
    fn gap_threshold_is_exclusive() {
        let mut session = started(TimingProfile::normal());
        assert!(session.handle(at(90), Input::Tick).is_empty());
        assert!(!session.is_paused());
        assert_eq!(session.snapshot(at(90)).focused_secs, 90);

        let effects = session.handle(at_ms(180_001), Input::Tick);
        assert_eq!(
            effects,
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        assert_eq!(session.snapshot(at(200)).remaining_secs, 2700 - 90);
    }

    #[test]
    fn wall_clock_jump_auto_pauses() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 5);
        // Monotonic clock stopped during sleep, the wall clock moved one hour.
        let woke = Now {
            mono_ms: 6_000,
            wall_ms: WALL0 + 6_000 + 3_600_000,
        };
        assert_eq!(
            session.handle(woke, Input::Tick),
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        assert_eq!(session.snapshot(woke).remaining_secs, 35);
    }

    #[test]
    fn wall_clock_going_back_is_not_a_gap() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 5);
        let corrected = Now {
            mono_ms: 6_000,
            wall_ms: WALL0 - 3_600_000,
        };
        assert!(session.handle(corrected, Input::Tick).is_empty());
        assert!(!session.is_paused());
    }

    #[test]
    fn gap_in_break_only_notifies() {
        let mut session = started(TimingProfile::demo());
        session.handle(at(1), Input::SkipPhase);
        let effects = session.handle(at(200), Input::Tick);
        assert_eq!(effects, vec![Effect::Notify(NotifyKind::Interrupted)]);
        assert!(session.is_paused());
        assert_eq!(session.snapshot(at(200)).remaining_secs, 20);
    }

    #[test]
    fn stop_after_unnoticed_gap_does_not_count_the_gap() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        let totals = only_totals(&session.handle(at(500), Input::Stop));
        assert_eq!(totals.focused_secs, 10);
        assert_eq!(totals.ended_at_ms, WALL0 + 500_000);
    }

    #[test]
    fn pause_after_unnoticed_gap_keeps_pre_gap_remaining() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        assert_eq!(
            session.handle(at(500), Input::Pause),
            vec![Effect::Tracking(false)]
        );
        assert_eq!(session.snapshot(at(500)).remaining_secs, 30);
        assert_eq!(session.snapshot(at(500)).focused_secs, 10);
    }

    #[test]
    fn zero_gap_disables_detection() {
        let profile = TimingProfile {
            interruption_gap_secs: 0,
            ..TimingProfile::normal()
        };
        let mut session = started(profile);
        assert!(session.handle(at(1000), Input::Tick).is_empty());
        assert!(!session.is_paused());
        assert_eq!(session.snapshot(at(1000)).focused_secs, 1000);
    }

    #[test]
    fn zero_length_phases_are_treated_as_one_second() {
        let profile = TimingProfile {
            focus_secs: 0,
            break_secs: 0,
            ..TimingProfile::demo()
        };
        let mut session = started(profile);
        assert_eq!(session.snapshot(at(0)).phase_total_secs, 1);
        assert_eq!(session.handle(at(1), Input::Tick), break_transition(1, 1));
        assert_eq!(session.handle(at(2), Input::Tick), focus_transition(2, 1));
    }

    #[test]
    fn snapshot_rounds_remaining_up_and_focused_down() {
        let session = started(TimingProfile::demo());
        let snap = session.snapshot(at_ms(1));
        assert_eq!(snap.remaining_secs, 40);
        assert_eq!(snap.focused_secs, 0);
        let snap = session.snapshot(at_ms(39_001));
        assert_eq!(snap.remaining_secs, 1);
        assert_eq!(snap.focused_secs, 39);
        assert_eq!(session.snapshot(at(45)).remaining_secs, 0);
    }

    #[test]
    fn odd_clock_values_never_panic() {
        let mut session = Session::new();
        session.handle(at(100), start_input(TimingProfile::demo()));
        // Monotonic clock going back: nothing counted, nothing fires.
        assert!(session.handle(at(50), Input::Tick).is_empty());
        assert_eq!(session.snapshot(at(50)).focused_secs, 0);
        assert_eq!(session.snapshot(at(50)).remaining_secs, 40);

        let far = Now {
            mono_ms: u64::MAX,
            wall_ms: i64::MAX,
        };
        session.handle(far, Input::Tick);
        assert!(session.is_paused());
        session.handle(far, Input::Resume);
        session.handle(far, Input::Tick);
        session.handle(far, Input::SkipPhase);
        let _ = session.snapshot(far);
        let ended = Now {
            mono_ms: 0,
            wall_ms: i64::MIN,
        };
        let totals = only_totals(&session.handle(ended, Input::Stop));
        assert_eq!(totals.ended_at_ms, i64::MIN);
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn tracking_is_on_only_while_focus_runs() {
        let mut session = Session::new();
        let mut tracking = false;
        let steps = [
            (0, start_input(TimingProfile::demo()), true),
            (5, Input::Pause, false),
            (6, Input::Resume, true),
            (7, Input::SkipPhase, false),
            (8, Input::Pause, false),
            (9, Input::Resume, false),
            (10, Input::SkipPhase, true),
            (11, Input::Stop, false),
        ];
        for (secs, input, expected) in steps {
            for effect in session.handle(at(secs), input) {
                if let Effect::Tracking(on) = effect {
                    tracking = on;
                }
            }
            assert_eq!(tracking, expected, "tracking after the input at {secs} s");
        }
    }

    // -----------------------------------------------------------------------
    // Meeting holds (demo profile: 40 s focus, 20 s break)
    // -----------------------------------------------------------------------

    use crate::api::{CalendarSource, MeetingChoice::*};

    fn hold(id: &str, choice: MeetingChoice, start_secs: u64, end_secs: u64) -> MeetingHold {
        let (start, end) = (at(start_secs), at(end_secs));
        MeetingHold {
            id: id.to_owned(),
            subject: format!("Meeting {id}"),
            start_mono_ms: start.mono_ms,
            end_mono_ms: end.mono_ms,
            start_wall_ms: start.wall_ms,
            end_wall_ms: end.wall_ms,
            choice,
        }
    }

    fn view(id: &str, choice: MeetingChoice, end_secs: u64) -> Option<MeetingHoldView> {
        Some(MeetingHoldView {
            subject: format!("Meeting {id}"),
            until_ms: at(end_secs).wall_ms,
            choice,
        })
    }

    /// A demo session started at 0 s with `holds` set before the start.
    fn with_holds(holds: Vec<MeetingHold>) -> Session {
        let mut session = Session::new();
        assert!(
            session
                .handle(at(0), Input::SetMeetingHolds(holds))
                .is_empty()
        );
        session.handle(at(0), start_input(TimingProfile::demo()));
        session
    }

    fn held_break_transition(cycle: u32, subject: Option<&str>) -> Vec<Effect> {
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Focus,
                to: Phase::Break,
                cycle,
            }),
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::Chime(ChimeKind::BreakStart),
            Effect::Notify(NotifyKind::BreakHeld {
                subject: subject.map(str::to_owned),
            }),
        ]
    }

    #[test]
    fn extend_keeps_focus_running_until_the_meeting_ends() {
        let mut session = with_holds(vec![hold("a", Extend, 50, 70)]);
        assert!(tick_through(&mut session, 0, 39).is_empty());
        assert!(!session.snapshot(at(39)).music_muted);

        // The break due at 40 s would be [40, 60) and overlaps the meeting [50, 70).
        assert!(session.handle(at(40), Input::Tick).is_empty());
        let snap = session.snapshot(at(40));
        assert_eq!(
            (
                snap.phase,
                snap.paused,
                snap.remaining_secs,
                snap.phase_total_secs
            ),
            (Phase::Focus, false, 30, 70)
        );
        assert_eq!(snap.meeting_hold, view("a", Extend, 70));
        assert!(!snap.music_muted, "the meeting starts at 50 s");
        assert_eq!(session.remaining_ms(at(40)), Some(30_000));
        assert!(session.wants_shields());

        assert!(tick_through(&mut session, 40, 69).is_empty());
        for t in [50, 60, 69] {
            assert!(session.snapshot(at(t)).music_muted, "muted at {t} s");
        }
        assert_eq!(session.snapshot(at(69)).remaining_secs, 1);

        assert_eq!(session.handle(at(70), Input::Tick), break_transition(1, 20));
        let snap = session.snapshot(at(70));
        assert_eq!(snap.meeting_hold, None);
        assert!(!snap.music_muted);
        assert_eq!(snap.phase_total_secs, 20);
        assert_eq!(snap.focused_secs, 70);

        let totals = only_totals(&session.handle(at(75), Input::Stop));
        assert_eq!(
            (
                totals.focused_secs,
                totals.break_secs,
                totals.cycles_completed
            ),
            (70, 5, 1)
        );
        assert_eq!(session.remaining_ms(at(76)), None);
    }

    #[test]
    fn meetings_that_only_touch_the_break_change_nothing() {
        // The break due at 40 s is [40, 60): "before" ends at 40 s, "after" starts at 60 s.
        let mut session = with_holds(vec![
            hold("before", Extend, 10, 40),
            hold("after", Extend, 60, 80),
            hold("held", PauseAtBreak, 60, 90),
        ]);
        assert!(!session.snapshot(at(5)).music_muted);
        assert!(session.snapshot(at(10)).music_muted);
        assert!(session.snapshot(at(39)).music_muted);
        assert!(tick_through(&mut session, 0, 39).is_empty());
        assert_eq!(session.handle(at(40), Input::Tick), break_transition(1, 20));
        assert!(!session.snapshot(at(40)).music_muted);

        // Music is muted only in Focus: not in the break, again in the next focus.
        assert!(tick_through(&mut session, 40, 59).is_empty());
        assert_eq!(session.handle(at(60), Input::Tick), focus_transition(2, 40));
        assert!(session.snapshot(at(65)).music_muted);
        assert!(!session.snapshot(at(80)).music_muted);
        // The break due at 100 s is [100, 120): after both meetings.
        assert!(tick_through(&mut session, 60, 99).is_empty());
        assert_eq!(
            session.handle(at(100), Input::Tick),
            break_transition(2, 20)
        );
    }

    #[test]
    fn extend_applies_once_per_meeting_and_skip_ends_it() {
        let mut session = with_holds(vec![hold("long", Extend, 30, 200)]);
        assert!(session.snapshot(at(30)).music_muted);
        assert!(tick_through(&mut session, 0, 40).is_empty());
        assert_eq!(session.snapshot(at(40)).phase_total_secs, 200);

        // Skip during the extension starts the break at once.
        assert_eq!(
            session.handle(at(100), Input::SkipPhase),
            break_transition(1, 20)
        );
        let snap = session.snapshot(at(100));
        assert_eq!(snap.meeting_hold, None);
        assert!(!snap.music_muted, "no muting in a break");

        assert!(tick_through(&mut session, 100, 119).is_empty());
        assert_eq!(
            session.handle(at(120), Input::Tick),
            focus_transition(2, 40)
        );
        assert!(
            session.snapshot(at(120)).music_muted,
            "the meeting still runs"
        );

        // The break due at 160 s overlaps the meeting again, but the hold was used.
        assert!(tick_through(&mut session, 120, 159).is_empty());
        assert_eq!(
            session.handle(at(160), Input::Tick),
            break_transition(2, 20)
        );
    }

    #[test]
    fn skip_ignores_meeting_holds() {
        let mut session = with_holds(vec![
            hold("held", PauseAtBreak, 0, 100),
            hold("ext", Extend, 0, 100),
        ]);
        assert_eq!(
            session.handle(at(10), Input::SkipPhase),
            break_transition(1, 20)
        );
        assert!(!session.is_paused());
        assert_eq!(session.snapshot(at(10)).meeting_hold, None);
    }

    #[test]
    fn extensions_chain_and_take_the_latest_meeting_end() {
        let mut session = with_holds(vec![
            hold("a", Extend, 45, 55),
            hold("b", Extend, 50, 70),
            hold("c", Extend, 75, 90),
        ]);
        assert!(tick_through(&mut session, 0, 40).is_empty());
        let snap = session.snapshot(at(40));
        assert_eq!((snap.remaining_secs, snap.phase_total_secs), (30, 70));
        assert_eq!(snap.meeting_hold, view("b", Extend, 70));

        // At 70 s the break would be [70, 90) and overlaps "c".
        assert!(tick_through(&mut session, 40, 70).is_empty());
        let snap = session.snapshot(at(70));
        assert_eq!((snap.remaining_secs, snap.phase_total_secs), (20, 90));
        assert_eq!(snap.meeting_hold, view("c", Extend, 90));
        assert!(!snap.music_muted, "between the meetings");
        assert!(session.snapshot(at(80)).music_muted);

        assert!(tick_through(&mut session, 70, 89).is_empty());
        assert_eq!(session.handle(at(90), Input::Tick), break_transition(1, 20));
    }

    #[test]
    fn extended_focus_pauses_and_resumes() {
        let mut session = with_holds(vec![hold("a", Extend, 45, 70)]);
        assert!(tick_through(&mut session, 0, 50).is_empty());
        assert_eq!(
            session.handle(at(50), Input::Pause),
            vec![Effect::Tracking(false)]
        );
        let snap = session.snapshot(at(55));
        assert!(snap.paused);
        assert_eq!(snap.remaining_secs, 20);
        assert_eq!(snap.meeting_hold, view("a", Extend, 70));
        assert!(snap.music_muted, "paused focus during the meeting");
        assert!(tick_through(&mut session, 50, 80).is_empty());

        assert_eq!(
            session.handle(at(80), Input::Resume),
            vec![Effect::Tracking(true)]
        );
        let snap = session.snapshot(at(80));
        assert_eq!(snap.meeting_hold, view("a", Extend, 70), "still extended");
        assert!(!snap.music_muted, "the meeting is over");
        assert_eq!(snap.remaining_secs, 20);

        assert!(tick_through(&mut session, 80, 99).is_empty());
        assert_eq!(
            session.handle(at(100), Input::Tick),
            break_transition(1, 20)
        );
        assert_eq!(session.snapshot(at(100)).focused_secs, 70);
    }

    #[test]
    fn stop_during_an_extended_focus() {
        let mut session = with_holds(vec![hold("a", Extend, 45, 70)]);
        assert!(tick_through(&mut session, 0, 40).is_empty());
        let effects = session.handle(at(55), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!((totals.focused_secs, totals.cycles_completed), (55, 0));
        assert_eq!(effects, stop_effects(Phase::Focus, 1, totals));
        let snap = session.snapshot(at(55));
        assert_eq!(snap.phase, Phase::Idle);
        assert_eq!(snap.meeting_hold, None);
        assert!(!snap.music_muted, "nothing is muted while idle");
        assert!(session.handle(at(56), Input::Stop).is_empty());
    }

    #[test]
    fn gap_during_an_extended_focus_auto_pauses_it() {
        let mut session = with_holds(vec![hold("a", Extend, 45, 70)]);
        assert!(tick_through(&mut session, 0, 50).is_empty());
        // Asleep from 50 s to 141 s.
        assert_eq!(
            session.handle(at(141), Input::Tick),
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        let snap = session.snapshot(at(141));
        assert!(snap.paused);
        assert_eq!(snap.remaining_secs, 20);
        assert_eq!(snap.meeting_hold, view("a", Extend, 70));

        assert_eq!(
            session.handle(at(150), Input::Resume),
            vec![Effect::Tracking(true)]
        );
        assert!(tick_through(&mut session, 150, 169).is_empty());
        assert_eq!(
            session.handle(at(170), Input::Tick),
            break_transition(1, 20)
        );
        assert_eq!(session.snapshot(at(170)).focused_secs, 70);
    }

    #[test]
    fn holds_are_checked_when_the_deadline_is_really_reached() {
        // A gap shifts the deadline past the meeting: the break starts normally.
        let mut session = with_holds(vec![hold("m", PauseAtBreak, 50, 100)]);
        assert!(tick_through(&mut session, 0, 30).is_empty());
        assert_eq!(
            session.handle(at(121), Input::Tick),
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        session.handle(at(125), Input::Resume);
        assert!(tick_through(&mut session, 125, 134).is_empty());
        assert_eq!(
            session.handle(at(135), Input::Tick),
            break_transition(1, 20)
        );

        // A gap shifts the deadline into the meeting: the break is held.
        let mut session = with_holds(vec![hold("m", PauseAtBreak, 150, 300)]);
        assert!(tick_through(&mut session, 0, 30).is_empty());
        session.handle(at(121), Input::Tick);
        assert!(session.is_paused());
        session.handle(at(145), Input::Resume);
        assert!(tick_through(&mut session, 145, 154).is_empty());
        assert_eq!(
            session.handle(at(155), Input::Tick),
            held_break_transition(1, Some("Meeting m"))
        );
    }

    #[test]
    fn pause_at_break_starts_the_break_paused() {
        let mut session = with_holds(vec![hold("standup", PauseAtBreak, 45, 90)]);
        assert!(tick_through(&mut session, 0, 39).is_empty());
        assert!(!session.snapshot(at(39)).music_muted);

        assert_eq!(
            session.handle(at(40), Input::Tick),
            held_break_transition(1, Some("Meeting standup"))
        );
        let snap = session.snapshot(at(40));
        assert_eq!(
            (
                snap.phase,
                snap.paused,
                snap.remaining_secs,
                snap.phase_total_secs
            ),
            (Phase::Break, true, 20, 20)
        );
        assert_eq!(snap.meeting_hold, view("standup", PauseAtBreak, 90));
        assert!(!snap.music_muted);
        assert!(session.is_paused());
        assert!(!session.wants_shields());

        // Frozen until Resume, however long the meeting runs.
        assert!(tick_through(&mut session, 40, 200).is_empty());
        assert_eq!(session.snapshot(at(200)).remaining_secs, 20);
        assert!(session.handle(at(200), Input::Pause).is_empty());

        assert!(session.handle(at(200), Input::Resume).is_empty());
        let snap = session.snapshot(at(200));
        assert!(!snap.paused);
        assert_eq!(snap.meeting_hold, None);
        assert_eq!(snap.remaining_secs, 20);

        assert!(tick_through(&mut session, 200, 219).is_empty());
        assert_eq!(
            session.handle(at(220), Input::Tick),
            focus_transition(2, 40)
        );
        let totals = only_totals(&session.handle(at(225), Input::Stop));
        assert_eq!(
            (
                totals.focused_secs,
                totals.break_secs,
                totals.cycles_completed
            ),
            (45, 20, 1)
        );
    }

    #[test]
    fn held_break_can_be_skipped_or_stopped() {
        let mut skipped = with_holds(vec![hold("m", PauseAtBreak, 45, 90)]);
        tick_through(&mut skipped, 0, 40);
        assert!(skipped.is_paused());
        assert_eq!(
            skipped.handle(at(60), Input::SkipPhase),
            focus_transition(2, 40)
        );
        let snap = skipped.snapshot(at(60));
        assert_eq!((snap.phase, snap.paused), (Phase::Focus, false));
        assert_eq!(snap.meeting_hold, None);

        let mut stopped = with_holds(vec![hold("m", PauseAtBreak, 45, 90)]);
        tick_through(&mut stopped, 0, 40);
        let effects = stopped.handle(at(60), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(
            (
                totals.focused_secs,
                totals.break_secs,
                totals.cycles_completed
            ),
            (40, 0, 1)
        );
        assert_eq!(effects, stop_effects(Phase::Break, 1, totals));
        assert_eq!(stopped.snapshot(at(60)).meeting_hold, None);
    }

    #[test]
    fn held_break_without_subject_and_tracking_stays_off() {
        let mut blank = hold("m", PauseAtBreak, 45, 90);
        blank.subject = "   ".to_owned();
        let mut session = with_holds(vec![blank]);
        let effects = tick_through(&mut session, 0, 40);
        assert_eq!(effects, held_break_transition(1, None));
        // Tracking stays off through the held break and its Resume; on again with the next focus.
        assert!(session.handle(at(50), Input::Resume).is_empty());
        assert_eq!(session.handle(at(70), Input::Tick), focus_transition(2, 40));
    }

    #[test]
    fn extend_wins_and_pause_at_break_applies_once() {
        // Both overlap the break due at 40 s: the focus is extended first, then the break is held.
        let mut session = with_holds(vec![
            hold("a", Extend, 45, 60),
            hold("b", PauseAtBreak, 45, 300),
        ]);
        assert!(tick_through(&mut session, 0, 40).is_empty());
        assert_eq!(session.snapshot(at(40)).meeting_hold, view("a", Extend, 60));
        assert!(tick_through(&mut session, 40, 59).is_empty());
        assert_eq!(
            session.handle(at(60), Input::Tick),
            held_break_transition(1, Some("Meeting b"))
        );
        assert_eq!(
            session.snapshot(at(60)).meeting_hold,
            view("b", PauseAtBreak, 300)
        );

        assert!(session.handle(at(70), Input::Resume).is_empty());
        assert!(tick_through(&mut session, 70, 89).is_empty());
        assert_eq!(session.handle(at(90), Input::Tick), focus_transition(2, 40));
        assert!(
            !session.snapshot(at(100)).music_muted,
            "PauseAtBreak never mutes"
        );
        // The break due at 130 s overlaps "b" again, but the hold was used.
        assert!(tick_through(&mut session, 90, 129).is_empty());
        assert_eq!(
            session.handle(at(130), Input::Tick),
            break_transition(2, 20)
        );
    }

    #[test]
    fn set_meeting_holds_keeps_usable_holds_across_sessions() {
        let mut session = Session::new();
        let holds = vec![
            hold("ignore", Ignore, 10, 20),
            hold("later", RemindLater, 10, 20),
            hold("empty", Extend, 30, 30),
            hold("inverted", PauseAtBreak, 30, 20),
            hold("ok", Extend, 10, 50),
        ];
        assert!(
            session
                .handle(at(0), Input::SetMeetingHolds(holds))
                .is_empty()
        );
        let ids: Vec<_> = session
            .meeting_holds()
            .iter()
            .map(|h| h.id.as_str())
            .collect();
        assert_eq!(ids, ["ok"]);

        let snap = session.snapshot(at(20));
        assert!(!snap.music_muted, "idle");
        assert_eq!(snap.meeting_hold, None);

        session.handle(at(20), start_input(TimingProfile::demo()));
        assert!(session.snapshot(at(20)).music_muted);
        session.handle(at(25), Input::Stop);
        assert_eq!(session.meeting_holds().len(), 1, "holds survive Stop");
        session.handle(at(30), start_input(TimingProfile::demo()));
        assert!(session.snapshot(at(30)).music_muted);

        assert!(
            session
                .handle(at(31), Input::SetMeetingHolds(Vec::new()))
                .is_empty()
        );
        assert!(session.meeting_holds().is_empty());
        assert!(!session.snapshot(at(31)).music_muted);
    }

    #[test]
    fn removing_a_hold_cancels_it_until_it_is_applied() {
        // Removed before the deadline: the break starts normally.
        let mut session = with_holds(vec![hold("m", PauseAtBreak, 45, 90)]);
        assert!(tick_through(&mut session, 0, 20).is_empty());
        assert!(
            session
                .handle(at(20), Input::SetMeetingHolds(Vec::new()))
                .is_empty()
        );
        assert!(tick_through(&mut session, 20, 39).is_empty());
        assert_eq!(session.handle(at(40), Input::Tick), break_transition(1, 20));

        // Removed during an extension: the focus stays extended (and muted) until the meeting ends.
        let mut session = with_holds(vec![hold("a", Extend, 45, 70)]);
        assert!(tick_through(&mut session, 0, 40).is_empty());
        assert!(
            session
                .handle(at(50), Input::SetMeetingHolds(Vec::new()))
                .is_empty()
        );
        let snap = session.snapshot(at(55));
        assert_eq!(snap.meeting_hold, view("a", Extend, 70));
        assert!(snap.music_muted);
        assert!(tick_through(&mut session, 50, 69).is_empty());
        assert_eq!(session.handle(at(70), Input::Tick), break_transition(1, 20));
    }

    #[test]
    fn hold_from_meeting_maps_wall_clock_to_monotonic() {
        let now = Now {
            mono_ms: 1_000_000,
            wall_ms: WALL0,
        };
        let meeting = Meeting {
            id: "ics:abc:1760000600000".to_owned(),
            source: CalendarSource::Ics,
            subject: "Review".to_owned(),
            start_ms: WALL0 + 600_000,
            end_ms: WALL0 + 1_200_000,
            online: true,
        };
        assert_eq!(
            MeetingHold::from_meeting(&meeting, Extend, now),
            Some(MeetingHold {
                id: meeting.id.clone(),
                subject: "Review".to_owned(),
                start_mono_ms: 1_600_000,
                end_mono_ms: 2_200_000,
                start_wall_ms: WALL0 + 600_000,
                end_wall_ms: WALL0 + 1_200_000,
                choice: Extend,
            })
        );

        let running = Meeting {
            start_ms: WALL0 - 300_000,
            ..meeting.clone()
        };
        let hold = MeetingHold::from_meeting(&running, PauseAtBreak, now).expect("running");
        assert_eq!((hold.start_mono_ms, hold.end_mono_ms), (700_000, 2_200_000));

        for choice in [Ignore, RemindLater] {
            assert_eq!(MeetingHold::from_meeting(&meeting, choice, now), None);
        }
        let empty = Meeting {
            end_ms: meeting.start_ms,
            ..meeting.clone()
        };
        assert_eq!(MeetingHold::from_meeting(&empty, Extend, now), None);

        // Saturates at the start of the monotonic clock: an old meeting has no length left.
        let early = Now {
            mono_ms: 10,
            wall_ms: WALL0,
        };
        let past = Meeting {
            start_ms: WALL0 - 50_000,
            end_ms: WALL0 - 20_000,
            ..meeting.clone()
        };
        assert_eq!(MeetingHold::from_meeting(&past, Extend, early), None);

        // Longer than 4 hours: no hold at all.
        let max = i64::try_from(MAX_MEETING_HOLD_MS).expect("4 h fits in i64");
        let four_hours = Meeting {
            end_ms: meeting.start_ms + max,
            ..meeting.clone()
        };
        let hold = MeetingHold::from_meeting(&four_hours, PauseAtBreak, now).expect("exactly 4 h");
        assert_eq!(hold.end_mono_ms - hold.start_mono_ms, MAX_MEETING_HOLD_MS);
        let too_long = Meeting {
            end_ms: meeting.start_ms + max + 1,
            ..meeting.clone()
        };
        for choice in [Extend, PauseAtBreak] {
            assert_eq!(MeetingHold::from_meeting(&too_long, choice, now), None);
        }
        let endless = Meeting {
            start_ms: i64::MIN,
            end_ms: i64::MAX,
            ..meeting
        };
        assert_eq!(MeetingHold::from_meeting(&endless, Extend, now), None);
    }

    #[test]
    fn meetings_longer_than_4_hours_never_hold_the_session() {
        let day = MAX_MEETING_HOLD_MS / 1000 + 1;
        // Both overlap the break due at 40 s ([40, 60)) and run for 4 h + 1 s.
        let mut session = with_holds(vec![
            hold("conference", Extend, 30, 30 + day),
            hold("offsite", PauseAtBreak, 45, 45 + day),
        ]);
        assert!(session.meeting_holds().is_empty());
        assert!(
            !session.snapshot(at(35)).music_muted,
            "no muting for an ignored meeting"
        );
        assert!(tick_through(&mut session, 0, 39).is_empty());
        assert_eq!(session.handle(at(40), Input::Tick), break_transition(1, 20));
        let snap = session.snapshot(at(40));
        assert_eq!((snap.paused, snap.phase_total_secs), (false, 20));
        assert_eq!(snap.meeting_hold, None);

        // Exactly 4 h still extends the focus to the meeting end.
        let four_hours = MAX_MEETING_HOLD_MS / 1000;
        let mut session = with_holds(vec![hold("workshop", Extend, 30, 30 + four_hours)]);
        assert_eq!(session.meeting_holds().len(), 1);
        assert!(tick_through(&mut session, 0, 40).is_empty());
        let snap = session.snapshot(at(40));
        assert_eq!(snap.meeting_hold, view("workshop", Extend, 30 + four_hours));
        assert_eq!(
            snap.phase_total_secs,
            u32::try_from(30 + four_hours).expect("fits")
        );
        assert!(snap.music_muted);
    }

    #[test]
    fn meeting_holds_with_extreme_values_never_panic() {
        // Endless on both clocks, or only on one of them: longer than 4 h, dropped.
        let endless = MeetingHold {
            start_mono_ms: 0,
            end_mono_ms: u64::MAX,
            start_wall_ms: i64::MIN,
            end_wall_ms: i64::MAX,
            ..hold("endless", Extend, 0, 1)
        };
        let endless_mono = MeetingHold {
            start_mono_ms: 0,
            end_mono_ms: u64::MAX,
            ..hold("endless-mono", Extend, 0, 1)
        };
        let endless_wall = MeetingHold {
            start_wall_ms: i64::MIN,
            end_wall_ms: i64::MAX,
            ..hold("endless-wall", Extend, 0, 1)
        };
        // At the very end of both clocks, 1 s long: kept.
        let late = MeetingHold {
            start_mono_ms: u64::MAX - 1000,
            end_mono_ms: u64::MAX,
            start_wall_ms: i64::MAX - 1000,
            end_wall_ms: i64::MAX,
            ..hold("late", Extend, 0, 1)
        };
        let last = MeetingHold {
            start_mono_ms: u64::MAX - 1,
            end_mono_ms: u64::MAX,
            ..hold("last", PauseAtBreak, 0, 1)
        };
        let mut session = with_holds(vec![endless, endless_mono, endless_wall, late, last]);
        let ids: Vec<_> = session
            .meeting_holds()
            .iter()
            .map(|h| h.id.as_str())
            .collect();
        assert_eq!(ids, ["late", "last"]);
        assert!(!session.snapshot(at(5)).music_muted);
        assert_eq!(tick_through(&mut session, 0, 40), break_transition(1, 20));

        let far = Now {
            mono_ms: u64::MAX,
            wall_ms: i64::MAX,
        };
        session.handle(far, Input::Tick);
        assert!(session.is_paused());
        session.handle(far, Input::Resume);
        session.handle(far, Input::Tick);
        session.handle(far, Input::SkipPhase);
        let _ = session.snapshot(far);
        let totals = only_totals(&session.handle(far, Input::Stop));
        assert_eq!(totals.ended_at_ms, i64::MAX);
    }
}
