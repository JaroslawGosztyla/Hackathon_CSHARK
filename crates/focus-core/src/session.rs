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
//!
//! The session goal ([`Input::SetGoal`], picked in "Co dalej?") is shown by every snapshot, survives Start and
//! ends up in [`SessionTotals::goal`] on Stop (then the session has no goal until the next `SetGoal`). It
//! never changes phases or effects.
//!
//! Developer mode (ADR-019) can shorten the current phase ([`Input::DevSetRemaining`]): only its deadline
//! moves (monotonic clock, the profile stays), so the phase still ends on a Tick with the usual effects.
//!
//! Smart breaks (ADR-023, [`Input::SetSmartMeetings`]): each focus block is fitted around the calendar's meetings with
//! [`crate::calendar::fit_break`] (the same function the calendar plans with) when it starts, on Resume and when the
//! meetings change. A fit only moves the block's deadline (shorter or longer, within the ADR-023 bounds) and is shown by
//! the snapshot (`smart_break`); the phase still ends on a Tick with the usual effects in the ADR-013 order. A block
//! extended by an `Extend` hold or shortened on the Developer page keeps its end. Skip, Pause and Stop work as always.
//!
//! Beta (ADR-022): the session keeps a plan of small steps ([`Input::SetSteps`], [`Input::SetStepDone`]), shown
//! by every snapshot. It survives Start; at Stop all steps go into [`SessionTotals::steps`] and the open ones stay
//! as the next plan (same ids). Every phase start and end, and every tick of a step, is reported as an
//! [`Effect::Progress`] fact (numbers and ids only, never texts) inserted into the ADR-013 effect order; the
//! other effects keep their relative order. Steps never change phases or other effects.

use crate::api::{
    AppError, ChimeKind, Meeting, MeetingChoice, MeetingHoldView, MicroStep, Phase, PhaseChange,
    SessionGoal, SessionSnapshot, SmartBreak, StepDraft, TimeSpan,
};
use crate::calendar::{
    BreakWindow, SmartMeeting, fit_break, meeting_hold_allowed, smart_break_view,
};
use crate::stats::MAX_FOCUS_SPANS;
use crate::timing::{MAX_MEETING_HOLD_MS, TimingProfile};
use crate::work::normalize_goal;

const MS_PER_SEC: u64 = 1000;

/// `AppError.code` of [`Session::check_dev_set_remaining`]: no session runs (ADR-019).
pub const ERR_NOT_RUNNING: &str = "notRunning";
/// Most small steps in one session plan (beta, ADR-022).
pub const MAX_STEPS: usize = 5;
/// Longest step text in characters; a longer one is cut with `…` as its last character.
pub const MAX_STEP_CHARS: usize = 80;
/// `AppError.code` of [`normalize_steps`]: more than [`MAX_STEPS`] steps (field `steps`).
pub const ERR_TOO_MANY_STEPS: &str = "tooManySteps";

/// Cleans the steps the UI sends (`set_session_steps`, ADR-022): every text is trimmed, control characters become
/// spaces, empty texts are dropped and a text longer than [`MAX_STEP_CHARS`] is cut with `…` as its last character.
/// More than [`MAX_STEPS`] non-empty steps return [`ERR_TOO_MANY_STEPS`] with field `steps`. The texts are the
/// user's own: the error never echoes them.
pub fn normalize_steps(drafts: Vec<StepDraft>) -> Result<Vec<StepDraft>, AppError> {
    let steps: Vec<StepDraft> = drafts
        .into_iter()
        .filter_map(|draft| {
            let text = clean_step_text(&draft.text);
            (!text.is_empty()).then_some(StepDraft { id: draft.id, text })
        })
        .collect();
    if steps.len() > MAX_STEPS {
        return Err(AppError {
            code: ERR_TOO_MANY_STEPS.to_owned(),
            message: format!("at most {MAX_STEPS} steps, got {}", steps.len()),
            field: Some("steps".to_owned()),
        });
    }
    Ok(steps)
}

/// One step text: control characters → spaces, trimmed, cut at [`MAX_STEP_CHARS`] with `…`.
fn clean_step_text(raw: &str) -> String {
    let spaced: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = spaced.trim();
    if trimmed.chars().count() <= MAX_STEP_CHARS {
        return trimmed.to_owned();
    }
    let mut cut: String = trimmed.chars().take(MAX_STEP_CHARS - 1).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    cut
}

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
    /// Smart breaks (ADR-023): replaces the meetings focus blocks are fitted around (wall clock, every meeting of the
    /// calendar with whether it may be fitted; empty while `Config.smart_breaks` is off). Idle or running; they survive
    /// Start and Stop. When the list changed, the current focus block (running or paused) is fitted again at once,
    /// unless its end is fixed: only its deadline moves. Returns no effects; only the snapshot changes.
    SetSmartMeetings(Vec<SmartMeeting>),
    /// Replaces the goal of the coming or running session (`None` clears it), cleaned by
    /// [`normalize_goal`] (a goal with neither key nor title counts as `None`). Allowed in every phase;
    /// returns no effects, only the snapshot changes.
    SetGoal(Option<SessionGoal>),
    /// Developer mode (ADR-019): the current phase ends `secs` from now, clamped to 1 s..=its length
    /// (a meeting extension included). A paused phase keeps that time frozen until Resume. Returns no
    /// effects: the phase ends on a later Tick exactly as at its own deadline. No-op while idle (the
    /// shell asks [`Session::check_dev_set_remaining`] first).
    DevSetRemaining { secs: u32 },
    /// Beta (ADR-022): replaces the small steps of the coming or running session (any phase) with drafts cleaned by
    /// [`normalize_steps`] (at most [`MAX_STEPS`] are kept, empty texts are dropped). A draft with the id of an
    /// existing step keeps that step's id and tick; any other draft becomes a new step with a fresh id (one above
    /// every id given out before). An empty list clears the plan. Returns no effects; only the snapshot changes.
    SetSteps(Vec<StepDraft>),
    /// Beta (ADR-022): ticks (`done`) or unticks a step in Focus or Break (also paused). Idle, an unknown id or no
    /// change: no-op. A tick returns `[Progress(StepDone)]` (`first_time` = never ticked before in this session);
    /// an untick returns nothing.
    SetStepDone { id: u32, done: bool },
}

/// How a phase ended (beta rewards, ADR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseEnd {
    /// Its deadline was reached on a Tick.
    Deadline,
    /// Skipped (`SkipPhase`).
    Skipped,
    /// The session was stopped.
    Stopped,
}

/// What happened in the session, for the beta rewards (ADR-022, `Effect::Progress`). Numbers and ids only, never
/// texts. `ran_secs` is the running time of that phase alone (no pauses or sleep gaps; a meeting extension counts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressFact {
    /// Idle → Focus; `steps` = the steps planned at Start.
    Started {
        session_id: String,
        steps: u32,
        profile: TimingProfile,
    },
    /// A focus phase ended; `held` = the break that follows starts paused for a meeting (`PauseAtBreak`).
    FocusEnded {
        session_id: String,
        cycle: u32,
        ran_secs: u32,
        how: PhaseEnd,
        held: bool,
        profile: TimingProfile,
    },
    /// A break ended (into the next focus, or by Stop); `cycle` = the cycle of the focus before it.
    BreakEnded {
        session_id: String,
        cycle: u32,
        ran_secs: u32,
        how: PhaseEnd,
        profile: TimingProfile,
    },
    /// A step was ticked: `done` of `total` steps are ticked now; `phase` = Focus or Break.
    StepDone {
        session_id: String,
        step_id: u32,
        first_time: bool,
        done: u32,
        total: u32,
        phase: Phase,
        profile: TimingProfile,
    },
}

impl ProgressFact {
    /// The session the fact belongs to.
    pub fn session_id(&self) -> &str {
        match self {
            ProgressFact::Started { session_id, .. }
            | ProgressFact::FocusEnded { session_id, .. }
            | ProgressFact::BreakEnded { session_id, .. }
            | ProgressFact::StepDone { session_id, .. } => session_id,
        }
    }

    /// The profile of that session (`demo` tells a demo session).
    pub fn profile(&self) -> &TimingProfile {
        match self {
            ProgressFact::Started { profile, .. }
            | ProgressFact::FocusEnded { profile, .. }
            | ProgressFact::BreakEnded { profile, .. }
            | ProgressFact::StepDone { profile, .. } => profile,
        }
    }
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
    /// The goal the session had when it ended ([`Input::SetGoal`]).
    pub goal: Option<SessionGoal>,
    /// Every small step of the session with its tick (beta, ADR-022); empty without a plan.
    pub steps: Vec<MicroStep>,
    /// Wall-clock intervals of running focus, oldest first (the weekly heatmaps, ADR-023): no pauses, no gaps; at
    /// most [`MAX_FOCUS_SPANS`] (beyond that the last one is stretched).
    pub focus_spans: Vec<TimeSpan>,
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
    /// Beta rewards (ADR-022): a fact for the rewards service. Emitted whatever the config says (the shell drops it
    /// while the rewards are off); it never changes the other effects or their order.
    Progress(ProgressFact),
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
    /// `focused_ms` when the current focus phase began (its own running time = the difference).
    phase_focused_start_ms: u64,
    /// `break_ms` when the current break began.
    phase_break_start_ms: u64,
    /// Running focus on the wall clock (ADR-023), see [`SessionTotals::focus_spans`].
    focus_spans: Vec<TimeSpan>,
    /// Smart breaks (ADR-023): how much the fit changed the current focus block, in ms (negative = shorter; 0 = not
    /// fitted). Part of [`Active::phase_total_ms`] in Focus; reset when a phase begins.
    smart_delta_ms: i64,
    /// The moved break of the current focus block, or the running break when it is the moved one (the snapshot's
    /// `smart_break`).
    smart_view: Option<SmartBreak>,
    /// The current focus block keeps its end (an `Extend` hold extended it, or the Developer page set its time left):
    /// it is not fitted again.
    smart_fixed: bool,
}

impl Active {
    fn is_running(&self) -> bool {
        matches!(self.clock, Clock::Running { .. })
    }

    /// Length of the current phase in milliseconds, including a smart-break fit and a meeting extension.
    fn phase_total_ms(&self) -> u64 {
        let base_ms = match self.stage {
            Stage::Focus => self
                .profile
                .focus_ms()
                .saturating_add_signed(self.smart_delta_ms),
            Stage::Break => self.profile.break_ms(),
        };
        base_ms.saturating_add(self.extra_ms)
    }

    /// Smart breaks (ADR-023): fits the current focus block around `meetings` again ([`fit_break`], the nominal end being
    /// the deadline without the current fit). Only the deadline (or a paused block's time left) moves, by the change of
    /// the fit; the time so far counts as before and there are no effects. A paused block is fitted as if it resumed
    /// now. Nothing happens for a fixed block, during a meeting hold or after an unnoticed gap (the next Tick pauses,
    /// Resume fits again). In a break the moved break keeps its view only while its meeting is still listed.
    fn refit_smart(&mut self, now: Now, meetings: &[SmartMeeting]) {
        if self.stage == Stage::Break {
            if self
                .smart_view
                .as_ref()
                .is_some_and(|view| !meetings.iter().any(|m| m.id == view.meeting_id))
            {
                self.smart_view = None;
            }
            return;
        }
        if self.smart_fixed || self.hold.is_some() || (self.is_running() && self.is_gap(now)) {
            return;
        }
        // Time left until the end without the current fit (negative: the block already runs past that end).
        let left_ms = match self.clock {
            Clock::Running { deadline_ms } => {
                signed_ms(deadline_ms) - signed_ms(now.mono_ms.max(self.last_seen.mono_ms))
            }
            Clock::Paused { remaining_ms } => signed_ms(remaining_ms),
        } - i128::from(self.smart_delta_ms);
        let nominal_ms = clamp_i64(i128::from(now.wall_ms) + left_ms);
        let fit = fit_break(now.wall_ms, nominal_ms, &self.profile, meetings);
        let delta_ms = fit.map_or(0, |fit| fit.delta_ms);
        let shift = delta_ms.saturating_sub(self.smart_delta_ms);
        self.clock = match self.clock {
            Clock::Running { deadline_ms } => Clock::Running {
                deadline_ms: deadline_ms.saturating_add_signed(shift),
            },
            Clock::Paused { remaining_ms } => Clock::Paused {
                remaining_ms: remaining_ms.saturating_add_signed(shift),
            },
        };
        self.smart_delta_ms = delta_ms;
        let break_ms = i64::try_from(self.profile.break_ms()).unwrap_or(i64::MAX);
        self.smart_view = fit.and_then(|fit| {
            let meeting = meetings.get(fit.meeting)?;
            let window = BreakWindow {
                number: self.cycle,
                start_ms: fit.break_start_ms,
                end_ms: fit.break_start_ms.saturating_add(break_ms),
            };
            Some(smart_break_view(&window, &fit, meeting))
        });
    }

    /// A focus block or break begins: no fit yet, nothing fixed (the moved break's view is handled by the caller).
    fn reset_smart(&mut self) {
        self.smart_delta_ms = 0;
        self.smart_fixed = false;
    }

    /// Length of the current phase in seconds, including a meeting extension.
    fn phase_total_secs(&self) -> u32 {
        ms_to_secs_ceil(self.phase_total_ms())
    }

    /// [`Input::DevSetRemaining`]: `secs` clamped to 1 s..=the phase length becomes the time left.
    fn set_remaining(&mut self, now: Now, secs: u32) {
        let remaining_ms = u64::from(secs)
            .saturating_mul(MS_PER_SEC)
            .clamp(MS_PER_SEC, self.phase_total_ms().max(MS_PER_SEC));
        // Smart breaks (ADR-023): the phase no longer ends at a moved time, and a focus block keeps this end.
        self.smart_view = None;
        self.smart_fixed = self.stage == Stage::Focus;
        self.clock = match self.clock {
            Clock::Paused { .. } => Clock::Paused { remaining_ms },
            Clock::Running { .. } => {
                // The time so far counts as usual. After an unnoticed gap nothing is counted and `last_seen`
                // stays: the next Tick still auto-pauses, with `remaining_ms` left.
                self.advance(now);
                Clock::Running {
                    deadline_ms: self.last_seen.mono_ms.saturating_add(remaining_ms),
                }
            }
        };
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
            Stage::Focus => {
                self.focused_ms = self.focused_ms.saturating_add(elapsed);
                self.record_focus_span(self.last_seen.wall_ms, now.wall_ms);
            }
            Stage::Break => self.break_ms = self.break_ms.saturating_add(elapsed),
        }
        // A monotonic clock never goes back; if it did, keep the later point so nothing is counted twice.
        if now.mono_ms >= self.last_seen.mono_ms {
            self.last_seen = now;
        }
        Advance::Counted
    }

    /// Adds `[from_ms, to_ms)` of running focus (ADR-023): it extends the last span when it starts where that one ended
    /// (every tick), otherwise it starts a new one (after a pause, a gap or a break). At [`MAX_FOCUS_SPANS`] spans the
    /// last one is stretched instead. A wall clock that went back adds nothing.
    fn record_focus_span(&mut self, from_ms: i64, to_ms: i64) {
        if to_ms <= from_ms {
            return;
        }
        let full = self.focus_spans.len() >= MAX_FOCUS_SPANS;
        match self.focus_spans.last_mut() {
            Some(last) if full || last.end_ms == from_ms => last.end_ms = last.end_ms.max(to_ms),
            _ => self.focus_spans.push(TimeSpan {
                start_ms: from_ms,
                end_ms: to_ms,
            }),
        }
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

    /// Running time of the current focus phase alone, in whole seconds (call [`Active::advance`] first).
    fn focus_ran_secs(&self) -> u32 {
        ms_to_secs_floor(self.focused_ms.saturating_sub(self.phase_focused_start_ms))
    }

    /// Running time of the current break alone, in whole seconds (call [`Active::advance`] first).
    fn break_ran_secs(&self) -> u32 {
        ms_to_secs_floor(self.break_ms.saturating_sub(self.phase_break_start_ms))
    }

    /// The `FocusEnded` fact of the focus phase that ends now.
    fn focus_ended(&self, how: PhaseEnd, held: bool) -> Effect {
        Effect::Progress(ProgressFact::FocusEnded {
            session_id: self.session_id.clone(),
            cycle: self.cycle,
            ran_secs: self.focus_ran_secs(),
            how,
            held,
            profile: self.profile,
        })
    }

    /// The `BreakEnded` fact of the break that ends now.
    fn break_ended(&self, how: PhaseEnd) -> Effect {
        Effect::Progress(ProgressFact::BreakEnded {
            session_id: self.session_id.clone(),
            cycle: self.cycle,
            ran_secs: self.break_ran_secs(),
            how,
            profile: self.profile,
        })
    }

    /// Focus → Break (deadline reached or skipped). Shields go down before the toast.
    fn enter_break(&mut self, now: Now, how: PhaseEnd) -> Vec<Effect> {
        let progress = self.focus_ended(how, false);
        let clock = Clock::Running {
            deadline_ms: now.mono_ms.saturating_add(self.profile.break_ms()),
        };
        self.begin_break(now, clock);
        if how != PhaseEnd::Deadline {
            // Skipped: this break starts now, not at the moved time.
            self.smart_view = None;
        }
        self.break_effects(
            progress,
            NotifyKind::BreakStarted {
                break_secs: self.profile.break_secs,
            },
        )
    }

    /// Focus → Break during a `PauseAtBreak` meeting: the break starts paused with its full length.
    /// Same effects as [`Active::enter_break`], with `BreakHeld` instead of `BreakStarted`.
    fn enter_held_break(&mut self, now: Now, hold: MeetingHold) -> Vec<Effect> {
        let progress = self.focus_ended(PhaseEnd::Deadline, true);
        let clock = Clock::Paused {
            remaining_ms: self.profile.break_ms(),
        };
        self.begin_break(now, clock);
        self.smart_view = None;
        let subject = Some(hold.subject.trim())
            .filter(|subject| !subject.is_empty())
            .map(str::to_owned);
        self.used_holds.push(hold.id.clone());
        self.hold = Some(hold);
        self.break_effects(progress, NotifyKind::BreakHeld { subject })
    }

    fn begin_break(&mut self, now: Now, clock: Clock) {
        self.cycles_completed = self.cycles_completed.saturating_add(1);
        self.stage = Stage::Break;
        self.clock = clock;
        self.last_seen = now;
        self.extra_ms = 0;
        self.hold = None;
        self.phase_break_start_ms = self.break_ms;
        self.reset_smart();
    }

    /// ADR-013 order with the `FocusEnded` fact (ADR-022) right before the toast.
    fn break_effects(&self, progress: Effect, notify: NotifyKind) -> Vec<Effect> {
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Focus,
                to: Phase::Break,
                cycle: self.cycle,
            }),
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::Chime(ChimeKind::BreakStart),
            progress,
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
        // Smart breaks (ADR-023): the meeting decides the end of this block now.
        self.smart_fixed = true;
        self.smart_view = None;
    }

    /// Break → next Focus (deadline reached or skipped). The toast comes before shields go up; the `BreakEnded`
    /// fact (ADR-022) right before the toast.
    fn enter_focus(&mut self, now: Now, how: PhaseEnd) -> Vec<Effect> {
        let progress = self.break_ended(how);
        self.cycle = self.cycle.saturating_add(1);
        self.stage = Stage::Focus;
        self.clock = Clock::Running {
            deadline_ms: now.mono_ms.saturating_add(self.profile.focus_ms()),
        };
        self.last_seen = now;
        self.extra_ms = 0;
        self.hold = None;
        self.phase_focused_start_ms = self.focused_ms;
        // The moved break (if any) is over; the session fits this block next (`Session::refit_smart`).
        self.reset_smart();
        self.smart_view = None;
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Break,
                to: Phase::Focus,
                cycle: self.cycle,
            }),
            Effect::Chime(ChimeKind::FocusStart),
            progress,
            Effect::Notify(NotifyKind::FocusStarted {
                focus_secs: self.profile.focus_secs,
                cycle: self.cycle,
            }),
            Effect::Shields(true),
            Effect::Tracking(true),
        ]
    }

    fn next_phase(&mut self, now: Now, how: PhaseEnd) -> Vec<Effect> {
        match self.stage {
            Stage::Focus => self.enter_break(now, how),
            Stage::Break => self.enter_focus(now, how),
        }
    }

    fn into_totals(
        self,
        now: Now,
        goal: Option<SessionGoal>,
        steps: Vec<MicroStep>,
    ) -> SessionTotals {
        SessionTotals {
            session_id: self.session_id,
            started_at_ms: self.started_at_ms,
            ended_at_ms: now.wall_ms,
            focused_secs: ms_to_secs_floor(self.focused_ms),
            break_secs: ms_to_secs_floor(self.break_ms),
            cycles_completed: self.cycles_completed,
            demo: self.profile.demo,
            goal,
            steps,
            focus_spans: self.focus_spans,
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
    /// Smart breaks (ADR-023): the meetings focus blocks are fitted around ([`Input::SetSmartMeetings`]).
    smart_meetings: Vec<SmartMeeting>,
    /// Goal of the coming (idle) or running session; moved into the totals on Stop.
    goal: Option<SessionGoal>,
    /// Small steps of the coming (idle) or running session (beta, ADR-022).
    steps: Vec<MicroStep>,
    /// Ids ticked at least once in the running session (a second tick is not a first one).
    ever_done: Vec<u32>,
    /// The highest step id given out so far (new ids are always above it).
    last_step_id: u32,
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
            smart_meetings: Vec::new(),
            goal: None,
            steps: Vec::new(),
            ever_done: Vec::new(),
            last_step_id: 0,
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
            Input::SetSmartMeetings(meetings) => {
                // The actor re-sends an unchanged list now and then: only a change fits the block again.
                if meetings != self.smart_meetings {
                    self.smart_meetings = meetings;
                    self.refit_smart(now);
                }
                Vec::new()
            }
            Input::SetGoal(goal) => {
                self.goal = goal.and_then(normalize_goal);
                Vec::new()
            }
            Input::DevSetRemaining { secs } => {
                if let Some(active) = self.active.as_mut() {
                    active.set_remaining(now, secs);
                }
                Vec::new()
            }
            Input::SetSteps(drafts) => {
                self.set_steps(drafts);
                Vec::new()
            }
            Input::SetStepDone { id, done } => self.set_step_done(id, done),
        }
    }

    /// Smart breaks (ADR-023): fits the current focus block around the meetings again (see [`Active::refit_smart`]).
    fn refit_smart(&mut self, now: Now) {
        if let Some(active) = self.active.as_mut() {
            active.refit_smart(now, &self.smart_meetings);
        }
    }

    /// The meetings smart breaks fit focus blocks around ([`Input::SetSmartMeetings`]).
    pub fn smart_meetings(&self) -> &[SmartMeeting] {
        &self.smart_meetings
    }

    /// [`Input::SetSteps`]: kept ids keep their tick (and "ticked before"), the others get fresh ids.
    fn set_steps(&mut self, drafts: Vec<StepDraft>) {
        let mut next_id = self
            .steps
            .iter()
            .map(|step| step.id)
            .fold(self.last_step_id, u32::max);
        let mut plan: Vec<MicroStep> = Vec::new();
        for draft in drafts {
            if plan.len() >= MAX_STEPS {
                break;
            }
            // Cleaned again: the input is meant to be clean already, but the core never trusts it.
            let text = clean_step_text(&draft.text);
            if text.is_empty() {
                continue;
            }
            let existing = draft
                .id
                .filter(|id| !plan.iter().any(|step| step.id == *id))
                .and_then(|id| self.steps.iter().find(|step| step.id == id));
            let step = match existing {
                Some(step) => MicroStep {
                    id: step.id,
                    text,
                    done: step.done,
                },
                None => {
                    next_id = next_id.saturating_add(1);
                    MicroStep {
                        id: next_id,
                        text,
                        done: false,
                    }
                }
            };
            plan.push(step);
        }
        self.last_step_id = next_id;
        self.ever_done
            .retain(|id| plan.iter().any(|step| step.id == *id));
        self.steps = plan;
    }

    /// [`Input::SetStepDone`]: a tick in a running (or paused) session reports `StepDone`.
    fn set_step_done(&mut self, id: u32, done: bool) -> Vec<Effect> {
        let Some(active) = self.active.as_ref() else {
            return Vec::new();
        };
        let Some(step) = self.steps.iter_mut().find(|step| step.id == id) else {
            return Vec::new();
        };
        if step.done == done {
            return Vec::new();
        }
        step.done = done;
        if !done {
            return Vec::new();
        }
        let first_time = !self.ever_done.contains(&id);
        if first_time {
            self.ever_done.push(id);
        }
        let ticked = self.steps.iter().filter(|step| step.done).count();
        vec![Effect::Progress(ProgressFact::StepDone {
            session_id: active.session_id.clone(),
            step_id: id,
            first_time,
            done: u32::try_from(ticked).unwrap_or(u32::MAX),
            total: u32::try_from(self.steps.len()).unwrap_or(u32::MAX),
            phase: active.stage.phase(),
            profile: active.profile,
        })]
    }

    /// Developer mode (ADR-019): `Ok` when [`Input::DevSetRemaining`] applies now (a session runs, paused or
    /// not), `Err` with [`ERR_NOT_RUNNING`] while idle.
    pub fn check_dev_set_remaining(&self) -> Result<(), AppError> {
        if self.active.is_some() {
            Ok(())
        } else {
            Err(AppError::new(ERR_NOT_RUNNING, "no session is running"))
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
                goal: self.goal.clone(),
                steps: self.steps.clone(),
                smart_break: None,
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
                goal: self.goal.clone(),
                steps: self.steps.clone(),
                // A meeting hold decides this phase now: no "moved break" line next to its banner.
                smart_break: active
                    .hold
                    .is_none()
                    .then(|| active.smart_view.clone())
                    .flatten(),
            },
        }
    }

    /// The goal of the coming or running session ([`Input::SetGoal`]).
    pub fn goal(&self) -> Option<&SessionGoal> {
        self.goal.as_ref()
    }

    /// The small steps of the coming or running session (beta, ADR-022).
    pub fn steps(&self) -> &[MicroStep] {
        &self.steps
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
        let started = ProgressFact::Started {
            session_id: session_id.clone(),
            steps: u32::try_from(self.steps.len()).unwrap_or(u32::MAX),
            profile,
        };
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
            phase_focused_start_ms: 0,
            phase_break_start_ms: 0,
            focus_spans: Vec::new(),
            smart_delta_ms: 0,
            smart_view: None,
            smart_fixed: false,
        });
        self.ever_done.clear();
        self.refit_smart(now);
        vec![
            Effect::PhaseChanged(PhaseChange {
                from: Phase::Idle,
                to: Phase::Focus,
                cycle: 1,
            }),
            Effect::Shields(true),
            Effect::Tracking(true),
            Effect::Chime(ChimeKind::FocusStart),
            Effect::Progress(started),
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
        // Smart breaks (ADR-023): the block may now fit the meetings differently (it was paused a while).
        active.refit_smart(now, &self.smart_meetings);
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
        let progress = match active.stage {
            Stage::Focus => active.focus_ended(PhaseEnd::Stopped, false),
            Stage::Break => active.break_ended(PhaseEnd::Stopped),
        };
        // Every step goes into the totals; the open ones stay as the next plan (same ids).
        let steps = self.steps.clone();
        self.steps.retain(|step| !step.done);
        self.ever_done.clear();
        vec![
            Effect::Tracking(false),
            Effect::Shields(false),
            Effect::PhaseChanged(change),
            progress,
            Effect::SessionEnded(active.into_totals(now, self.goal.take(), steps)),
        ]
    }

    fn skip(&mut self, now: Now) -> Vec<Effect> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        active.advance(now);
        let effects = active.next_phase(now, PhaseEnd::Skipped);
        // A new focus block is fitted (no effects; the phase change above keeps its ADR-013 order).
        active.refit_smart(now, &self.smart_meetings);
        effects
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
            let effects = active.enter_focus(now, PhaseEnd::Deadline);
            active.refit_smart(now, &self.smart_meetings);
            return effects;
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
        active.enter_break(now, PhaseEnd::Deadline)
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

/// Milliseconds as a wide signed number (differences never overflow).
fn signed_ms(ms: u64) -> i128 {
    i128::from(ms)
}

/// A wide number back to `i64`, saturating.
fn clamp_i64(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
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

    /// The effects without the ADR-022 `Progress` facts: what the session emitted before ADR-022. The helpers
    /// below build those vectors, so every test comparing `legacy(..)` with them checks that the old effects keep
    /// their exact order; the facts themselves have their own exact-order tests (ADR-022 section).
    fn legacy(effects: &[Effect]) -> Vec<Effect> {
        effects
            .iter()
            .filter(|effect| !matches!(effect, Effect::Progress(_)))
            .cloned()
            .collect()
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
                goal: None,
                steps: Vec::new(),
                smart_break: None,
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
    fn idle_profile_switched_during_a_session_waits_for_the_next_start() {
        // "Presentation mode" switched on the Developer page while a normal session runs (ADR-019).
        let mut session = started(TimingProfile::normal());
        session.set_idle_profile(TimingProfile::demo());
        let snap = session.snapshot(at(10));
        assert_eq!(
            (snap.focus_secs, snap.break_secs, snap.demo),
            (2700, 900, false)
        );
        assert_eq!(snap.remaining_secs, 2690);
        assert_eq!(session.profile(), Some(TimingProfile::normal()));

        session.handle(at(20), Input::Stop);
        let idle = session.snapshot(at(21));
        assert_eq!(
            (idle.focus_secs, idle.break_secs, idle.demo),
            (40, 20, true)
        );
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
                Effect::Progress(ProgressFact::Started {
                    session_id: "s1".to_owned(),
                    steps: 0,
                    profile: TimingProfile::demo(),
                }),
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

        assert_eq!(
            legacy(&session.handle(at(40), Input::Tick)),
            break_transition(1, 20)
        );
        let snap = session.snapshot(at(40));
        assert_eq!(snap.phase, Phase::Break);
        assert_eq!(snap.cycle, 1);
        assert_eq!(snap.remaining_secs, 20);
        assert_eq!(snap.phase_total_secs, 20);
        assert_eq!(snap.focused_secs, 40);
        assert!(!session.wants_shields());

        assert!(tick_through(&mut session, 40, 59).is_empty());
        assert_eq!(session.snapshot(at(59)).remaining_secs, 1);
        assert_eq!(
            legacy(&session.handle(at(60), Input::Tick)),
            focus_transition(2, 40)
        );
        let snap = session.snapshot(at(60));
        assert_eq!(
            (snap.phase, snap.cycle, snap.remaining_secs),
            (Phase::Focus, 2, 40)
        );

        assert!(tick_through(&mut session, 60, 99).is_empty());
        assert_eq!(
            legacy(&session.handle(at(100), Input::Tick)),
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
            goal: None,
            steps: Vec::new(),
            // The two focus phases on the wall clock; the breaks in between are not focus.
            focus_spans: vec![
                TimeSpan {
                    start_ms: WALL0,
                    end_ms: WALL0 + 40_000,
                },
                TimeSpan {
                    start_ms: WALL0 + 60_000,
                    end_ms: WALL0 + 100_000,
                },
            ],
        };
        assert_eq!(legacy(&effects), stop_effects(Phase::Break, 2, expected));
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn late_tick_still_transitions_once() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 38);
        // The tick for second 39 and 40 got merged into one late tick.
        assert_eq!(
            legacy(&session.handle(at_ms(41_500), Input::Tick)),
            break_transition(1, 20)
        );
        // The break is measured from the transition.
        assert_eq!(session.snapshot(at_ms(41_500)).remaining_secs, 20);
        assert!(session.handle(at_ms(61_499), Input::Tick).is_empty());
        assert_eq!(
            legacy(&session.handle(at_ms(61_500), Input::Tick)),
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
            legacy(&session.handle(at_ms(229_500), Input::Tick)),
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
        assert_eq!(
            legacy(&session.handle(at(76), Input::Tick)),
            focus_transition(2, 40)
        );
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
            goal: None,
            steps: Vec::new(),
            focus_spans: vec![TimeSpan {
                start_ms: WALL0,
                end_ms: WALL0 + 12_700,
            }],
        };
        assert_eq!(legacy(&effects), stop_effects(Phase::Focus, 1, expected));
        assert_eq!(session.phase(), Phase::Idle);
        assert!(!session.wants_shields());
        assert_eq!(session.snapshot(at(13)).phase, Phase::Idle);
    }

    fn span(start_ms: i64, end_ms: i64) -> TimeSpan {
        TimeSpan {
            start_ms: WALL0 + start_ms,
            end_ms: WALL0 + end_ms,
        }
    }

    #[test]
    fn focus_spans_leave_out_pauses_gaps_and_a_clock_that_went_back() {
        let mut session = started(TimingProfile::normal());
        tick_through(&mut session, 0, 10);
        session.handle(at(10), Input::Pause);
        // Paused time is not focus.
        session.handle(at(25), Input::Resume);
        tick_through(&mut session, 25, 30);
        // The wall clock went back by 5 s (e.g. a time sync): that second adds nothing, the next ones go on from there.
        session.handle(
            Now {
                mono_ms: 31_000,
                wall_ms: WALL0 + 26_000,
            },
            Input::Tick,
        );
        session.handle(
            Now {
                mono_ms: 32_000,
                wall_ms: WALL0 + 27_000,
            },
            Input::Tick,
        );
        // A sleep gap: nothing is counted and the session pauses itself.
        session.handle(
            Now {
                mono_ms: 32_000 + 3_600_000,
                wall_ms: WALL0 + 27_000 + 3_600_000,
            },
            Input::Tick,
        );
        let totals = only_totals(&session.handle(
            Now {
                mono_ms: 33_000 + 3_600_000,
                wall_ms: WALL0 + 28_000 + 3_600_000,
            },
            Input::Stop,
        ));
        assert_eq!(
            totals.focus_spans,
            vec![span(0, 10_000), span(25_000, 30_000), span(26_000, 27_000)]
        );
        assert_eq!(totals.focused_secs, 10 + 5 + 1 + 1);
    }

    #[test]
    fn focus_spans_are_capped_by_stretching_the_last_one() {
        let mut session = started(TimingProfile::normal());
        let mut t = 0;
        for _ in 0..300 {
            session.handle(at(t + 1), Input::Tick);
            session.handle(at(t + 1), Input::Pause);
            session.handle(at(t + 2), Input::Resume);
            t += 2;
        }
        let totals = only_totals(&session.handle(at(t), Input::Stop));
        assert_eq!(totals.focus_spans.len(), MAX_FOCUS_SPANS);
        assert_eq!(totals.focus_spans[0], span(0, 1_000));
        assert_eq!(totals.focus_spans[1], span(2_000, 3_000));
        let last = totals.focus_spans.last().expect("spans");
        assert_eq!(last.end_ms, WALL0 + 599_000);
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
        assert_eq!(legacy(&effects), stop_effects(Phase::Focus, 1, totals));
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
        assert_eq!(legacy(&effects), stop_effects(Phase::Break, 1, totals));

        let mut paused = started(TimingProfile::demo());
        tick_through(&mut paused, 0, 45);
        paused.handle(at(45), Input::Pause);
        let effects = paused.handle(at(70), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!((totals.focused_secs, totals.break_secs), (40, 5));
        assert_eq!(legacy(&effects), stop_effects(Phase::Break, 1, totals));
    }

    #[test]
    fn double_stop_ends_the_session_once() {
        let mut session = started(TimingProfile::demo());
        let mut effects = session.handle(at(3), Input::Stop);
        effects.extend(session.handle(at(4), Input::Stop));
        effects.extend(session.handle(at(5), Input::Stop));
        assert_eq!(ended_totals(&effects).len(), 1);
        // Tracking, Shields, PhaseChanged, Progress(FocusEnded), SessionEnded: the later Stops emit nothing.
        assert_eq!(effects.len(), 5);
        assert_eq!(progress_of(&effects).len(), 1);
    }

    #[test]
    fn skip_from_focus_counts_a_completed_cycle() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 5);
        assert_eq!(
            legacy(&session.handle(at(5), Input::SkipPhase)),
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
            legacy(&session.handle(at(4), Input::SkipPhase)),
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
            legacy(&session.handle(at(30), Input::SkipPhase)),
            break_transition(1, 20)
        );
        assert!(!session.is_paused());
        assert_eq!(session.snapshot(at(30)).remaining_secs, 20);
        // Paused time is not focus time.
        assert_eq!(session.snapshot(at(30)).focused_secs, 3);

        session.handle(at(31), Input::Pause);
        assert_eq!(
            legacy(&session.handle(at(90), Input::SkipPhase)),
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
            legacy(&session.handle(at(180), Input::Tick)),
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
        assert_eq!(
            legacy(&session.handle(at(1), Input::Tick)),
            break_transition(1, 1)
        );
        assert_eq!(
            legacy(&session.handle(at(2), Input::Tick)),
            focus_transition(2, 1)
        );
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
        session.handle(far, Input::DevSetRemaining { secs: u32::MAX });
        session.handle(at(1), Input::DevSetRemaining { secs: 0 });
        session.handle(far, Input::Tick);
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

        assert_eq!(
            legacy(&session.handle(at(70), Input::Tick)),
            break_transition(1, 20)
        );
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
        assert_eq!(
            legacy(&session.handle(at(40), Input::Tick)),
            break_transition(1, 20)
        );
        assert!(!session.snapshot(at(40)).music_muted);

        // Music is muted only in Focus: not in the break, again in the next focus.
        assert!(tick_through(&mut session, 40, 59).is_empty());
        assert_eq!(
            legacy(&session.handle(at(60), Input::Tick)),
            focus_transition(2, 40)
        );
        assert!(session.snapshot(at(65)).music_muted);
        assert!(!session.snapshot(at(80)).music_muted);
        // The break due at 100 s is [100, 120): after both meetings.
        assert!(tick_through(&mut session, 60, 99).is_empty());
        assert_eq!(
            legacy(&session.handle(at(100), Input::Tick)),
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
            legacy(&session.handle(at(100), Input::SkipPhase)),
            break_transition(1, 20)
        );
        let snap = session.snapshot(at(100));
        assert_eq!(snap.meeting_hold, None);
        assert!(!snap.music_muted, "no muting in a break");

        assert!(tick_through(&mut session, 100, 119).is_empty());
        assert_eq!(
            legacy(&session.handle(at(120), Input::Tick)),
            focus_transition(2, 40)
        );
        assert!(
            session.snapshot(at(120)).music_muted,
            "the meeting still runs"
        );

        // The break due at 160 s overlaps the meeting again, but the hold was used.
        assert!(tick_through(&mut session, 120, 159).is_empty());
        assert_eq!(
            legacy(&session.handle(at(160), Input::Tick)),
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
            legacy(&session.handle(at(10), Input::SkipPhase)),
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
        assert_eq!(
            legacy(&session.handle(at(90), Input::Tick)),
            break_transition(1, 20)
        );
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
            legacy(&session.handle(at(100), Input::Tick)),
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
        assert_eq!(legacy(&effects), stop_effects(Phase::Focus, 1, totals));
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
            legacy(&session.handle(at(170), Input::Tick)),
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
            legacy(&session.handle(at(135), Input::Tick)),
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
            legacy(&session.handle(at(155), Input::Tick)),
            held_break_transition(1, Some("Meeting m"))
        );
    }

    #[test]
    fn pause_at_break_starts_the_break_paused() {
        let mut session = with_holds(vec![hold("standup", PauseAtBreak, 45, 90)]);
        assert!(tick_through(&mut session, 0, 39).is_empty());
        assert!(!session.snapshot(at(39)).music_muted);

        assert_eq!(
            legacy(&session.handle(at(40), Input::Tick)),
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
            legacy(&session.handle(at(220), Input::Tick)),
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
            legacy(&skipped.handle(at(60), Input::SkipPhase)),
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
        assert_eq!(legacy(&effects), stop_effects(Phase::Break, 1, totals));
        assert_eq!(stopped.snapshot(at(60)).meeting_hold, None);
    }

    #[test]
    fn held_break_without_subject_and_tracking_stays_off() {
        let mut blank = hold("m", PauseAtBreak, 45, 90);
        blank.subject = "   ".to_owned();
        let mut session = with_holds(vec![blank]);
        let effects = tick_through(&mut session, 0, 40);
        assert_eq!(legacy(&effects), held_break_transition(1, None));
        // Tracking stays off through the held break and its Resume; on again with the next focus.
        assert!(session.handle(at(50), Input::Resume).is_empty());
        assert_eq!(
            legacy(&session.handle(at(70), Input::Tick)),
            focus_transition(2, 40)
        );
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
            legacy(&session.handle(at(60), Input::Tick)),
            held_break_transition(1, Some("Meeting b"))
        );
        assert_eq!(
            session.snapshot(at(60)).meeting_hold,
            view("b", PauseAtBreak, 300)
        );

        assert!(session.handle(at(70), Input::Resume).is_empty());
        assert!(tick_through(&mut session, 70, 89).is_empty());
        assert_eq!(
            legacy(&session.handle(at(90), Input::Tick)),
            focus_transition(2, 40)
        );
        assert!(
            !session.snapshot(at(100)).music_muted,
            "PauseAtBreak never mutes"
        );
        // The break due at 130 s overlaps "b" again, but the hold was used.
        assert!(tick_through(&mut session, 90, 129).is_empty());
        assert_eq!(
            legacy(&session.handle(at(130), Input::Tick)),
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
        assert_eq!(
            legacy(&session.handle(at(40), Input::Tick)),
            break_transition(1, 20)
        );

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
        assert_eq!(
            legacy(&session.handle(at(70), Input::Tick)),
            break_transition(1, 20)
        );
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
        assert_eq!(
            legacy(&session.handle(at(40), Input::Tick)),
            break_transition(1, 20)
        );
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
        assert_eq!(
            legacy(&tick_through(&mut session, 0, 40)),
            break_transition(1, 20)
        );

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

    fn goal(key: &str) -> SessionGoal {
        SessionGoal {
            source: crate::api::WorkSource::Jira,
            key: key.to_owned(),
            title: format!("Title of {key}"),
            url: format!("https://firma.atlassian.net/browse/{key}"),
        }
    }

    #[test]
    fn goal_picked_before_start_is_shown_and_ends_in_the_totals() {
        let mut session = Session::new();
        assert!(
            session
                .handle(at(0), Input::SetGoal(Some(goal("PROJ-1"))))
                .is_empty()
        );
        assert_eq!(session.snapshot(at(0)).goal, Some(goal("PROJ-1")));
        assert_eq!(session.goal(), Some(&goal("PROJ-1")));

        // Start keeps it; it is in every snapshot of the session, in Focus and in Break.
        session.handle(at(0), start_input(TimingProfile::demo()));
        assert_eq!(session.snapshot(at(5)).goal, Some(goal("PROJ-1")));
        assert_eq!(
            legacy(&tick_through(&mut session, 0, 40)),
            break_transition(1, 20)
        );
        assert_eq!(session.snapshot(at(41)).goal, Some(goal("PROJ-1")));

        // Stop moves it into the totals and leaves the next session without a goal.
        let totals = only_totals(&session.handle(at(45), Input::Stop));
        assert_eq!(totals.goal, Some(goal("PROJ-1")));
        assert_eq!(session.snapshot(at(46)).goal, None);
        assert_eq!(session.goal(), None);
        session.handle(at(50), start_input(TimingProfile::demo()));
        let totals = only_totals(&session.handle(at(60), Input::Stop));
        assert_eq!(totals.goal, None);
    }

    #[test]
    fn goal_changes_never_touch_phases_or_effects() {
        let mut session = started(TimingProfile::demo());
        let before = session.snapshot(at(10));
        for input in [
            Input::SetGoal(Some(goal("A-1"))),
            Input::SetGoal(Some(goal("B-2"))),
            Input::SetGoal(None),
            Input::SetGoal(Some(goal("C-3"))),
        ] {
            assert!(session.handle(at(10), input).is_empty());
        }
        let after = session.snapshot(at(10));
        assert_eq!(
            after,
            SessionSnapshot {
                goal: Some(goal("C-3")),
                ..before
            }
        );
        session.handle(at(12), Input::Pause);
        session.handle(at(13), Input::SetGoal(None));
        assert!(session.is_paused());
        assert_eq!(session.snapshot(at(14)).goal, None);
        let totals = only_totals(&session.handle(at(15), Input::Stop));
        assert_eq!(totals.goal, None);
    }

    #[test]
    fn goal_is_cleaned() {
        let mut session = Session::new();
        let messy = SessionGoal {
            key: " PROJ-9\n".to_owned(),
            title: "  Ship\tit  ".to_owned(),
            url: "javascript:alert(1)".to_owned(),
            ..goal("PROJ-9")
        };
        session.handle(at(0), Input::SetGoal(Some(messy)));
        assert_eq!(
            session.goal(),
            Some(&SessionGoal {
                key: "PROJ-9".to_owned(),
                title: "Ship it".to_owned(),
                url: String::new(),
                ..goal("PROJ-9")
            })
        );
        let empty = SessionGoal {
            key: "  ".to_owned(),
            title: String::new(),
            ..goal("X")
        };
        session.handle(at(1), Input::SetGoal(Some(empty)));
        assert_eq!(session.goal(), None);
    }

    // -----------------------------------------------------------------------
    // Developer mode: Input::DevSetRemaining (ADR-019)
    // -----------------------------------------------------------------------

    fn set_remaining(secs: u32) -> Input {
        Input::DevSetRemaining { secs }
    }

    #[test]
    fn dev_set_remaining_ends_a_running_focus_with_the_usual_effects() {
        let mut session = started(TimingProfile::normal());
        assert!(tick_through(&mut session, 0, 10).is_empty());
        assert_eq!(session.check_dev_set_remaining(), Ok(()));

        assert!(session.handle(at(10), set_remaining(5)).is_empty());
        let snap = session.snapshot(at(10));
        assert_eq!(
            (
                snap.phase,
                snap.paused,
                snap.remaining_secs,
                snap.phase_total_secs
            ),
            (Phase::Focus, false, 5, 2700),
            "only the deadline moves, the phase length stays the profile's"
        );
        assert_eq!(session.remaining_ms(at(10)), Some(5_000));

        assert!(tick_through(&mut session, 10, 14).is_empty());
        assert_eq!(
            legacy(&session.handle(at(15), Input::Tick)),
            break_transition(1, 900)
        );
        // The break that follows has its full length; the time really spent counts.
        let snap = session.snapshot(at(15));
        assert_eq!(
            (snap.phase, snap.remaining_secs, snap.focused_secs),
            (Phase::Break, 900, 15)
        );
        let totals = only_totals(&session.handle(at(20), Input::Stop));
        assert_eq!((totals.focused_secs, totals.break_secs), (15, 5));
        assert_eq!(totals.cycles_completed, 1);
    }

    #[test]
    fn dev_set_remaining_ends_a_running_break_into_the_next_focus() {
        let mut session = started(TimingProfile::normal());
        session.handle(at(5), Input::SkipPhase);
        assert!(session.handle(at(6), set_remaining(3)).is_empty());
        assert_eq!(session.snapshot(at(6)).remaining_secs, 3);
        assert!(tick_through(&mut session, 6, 8).is_empty());
        assert_eq!(
            legacy(&session.handle(at(9), Input::Tick)),
            focus_transition(2, 2700)
        );
        let snap = session.snapshot(at(9));
        assert_eq!(
            (snap.phase, snap.cycle, snap.remaining_secs),
            (Phase::Focus, 2, 2700)
        );
    }

    #[test]
    fn dev_set_remaining_on_a_paused_phase_waits_for_resume() {
        let mut session = started(TimingProfile::normal());
        session.handle(at(10), Input::Pause);
        assert_eq!(session.check_dev_set_remaining(), Ok(()));
        assert!(session.handle(at(20), set_remaining(7)).is_empty());
        let snap = session.snapshot(at(20));
        assert_eq!((snap.paused, snap.remaining_secs), (true, 7));
        // Frozen: nothing happens while paused, and paused time is not focus time.
        assert!(tick_through(&mut session, 20, 60).is_empty());
        assert_eq!(session.snapshot(at(60)).remaining_secs, 7);

        assert_eq!(
            session.handle(at(100), Input::Resume),
            vec![Effect::Tracking(true)]
        );
        assert!(tick_through(&mut session, 100, 106).is_empty());
        assert_eq!(
            legacy(&session.handle(at(107), Input::Tick)),
            break_transition(1, 900)
        );
        assert_eq!(session.snapshot(at(107)).focused_secs, 17);
    }

    #[test]
    fn dev_set_remaining_while_idle_is_refused_and_changes_nothing() {
        let mut session = Session::new();
        let error = session
            .check_dev_set_remaining()
            .expect_err("no session runs");
        assert_eq!(error.code, ERR_NOT_RUNNING);
        assert_eq!(error.code, "notRunning");
        let before = session.snapshot(at(3));
        assert!(session.handle(at(3), set_remaining(5)).is_empty());
        assert_eq!(session.snapshot(at(3)), before);
        assert!(tick_through(&mut session, 3, 10).is_empty());

        // After Stop it is refused again.
        let mut session = started(TimingProfile::demo());
        session.handle(at(2), Input::Stop);
        assert_eq!(
            session.check_dev_set_remaining().map_err(|e| e.code),
            Err(ERR_NOT_RUNNING.to_owned())
        );
    }

    #[test]
    fn dev_set_remaining_is_clamped_to_the_phase_length() {
        let mut session = started(TimingProfile::normal());
        // Zero would end the phase on the next tick without warning: at least one second.
        session.handle(at(10), set_remaining(0));
        assert_eq!(session.remaining_ms(at(10)), Some(1_000));
        assert!(session.handle(at_ms(10_999), Input::Tick).is_empty());
        assert_eq!(
            legacy(&session.handle(at(11), Input::Tick)),
            break_transition(1, 900)
        );

        // Never longer than the phase (the break: 900 s), even long after it started.
        session.handle(at(80), set_remaining(u32::MAX));
        assert_eq!(session.remaining_ms(at(80)), Some(900_000));
        let snap = session.snapshot(at(80));
        assert_eq!((snap.remaining_secs, snap.phase_total_secs), (900, 900));

        // Demo profile: 40 s focus.
        let mut demo = started(TimingProfile::demo());
        demo.handle(at(1), set_remaining(3600));
        assert_eq!(demo.snapshot(at(1)).remaining_secs, 40);
        demo.handle(at(2), Input::Pause);
        demo.handle(at(3), set_remaining(0));
        assert_eq!(demo.remaining_ms(at(3)), Some(1_000));
    }

    #[test]
    fn dev_set_remaining_counts_a_meeting_extension_as_phase_length() {
        let mut session = with_holds(vec![hold("a", Extend, 50, 70)]);
        tick_through(&mut session, 0, 40);
        assert_eq!(session.snapshot(at(40)).phase_total_secs, 70);
        session.handle(at(45), set_remaining(u32::MAX));
        assert_eq!(session.snapshot(at(45)).remaining_secs, 70);
        session.handle(at(46), set_remaining(2));
        assert!(tick_through(&mut session, 46, 47).is_empty());
        // The extension was used: the focus now ends into a normal break.
        assert_eq!(
            legacy(&session.handle(at(48), Input::Tick)),
            break_transition(1, 20)
        );
    }

    #[test]
    fn dev_set_remaining_after_an_unnoticed_gap_still_auto_pauses() {
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        // The computer slept 91 s; the input arrives before the next tick.
        assert!(session.handle(at(101), set_remaining(5)).is_empty());
        assert_eq!(
            session.handle(at(102), Input::Tick),
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        let snap = session.snapshot(at(102));
        assert_eq!((snap.paused, snap.remaining_secs), (true, 5));
        assert_eq!(snap.focused_secs, 10, "gap time is not focus time");
    }

    // -----------------------------------------------------------------------
    // Beta (ADR-022): Progress facts and small steps (demo profile: 40 s focus, 20 s break)
    // -----------------------------------------------------------------------

    fn progress_of(effects: &[Effect]) -> Vec<ProgressFact> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Progress(fact) => Some(fact.clone()),
                _ => None,
            })
            .collect()
    }

    fn focus_ended(cycle: u32, ran_secs: u32, how: PhaseEnd, held: bool) -> Effect {
        Effect::Progress(ProgressFact::FocusEnded {
            session_id: "s1".to_owned(),
            cycle,
            ran_secs,
            how,
            held,
            profile: TimingProfile::demo(),
        })
    }

    fn break_ended(cycle: u32, ran_secs: u32, how: PhaseEnd) -> Effect {
        Effect::Progress(ProgressFact::BreakEnded {
            session_id: "s1".to_owned(),
            cycle,
            ran_secs,
            how,
            profile: TimingProfile::demo(),
        })
    }

    fn draft(id: Option<u32>, text: &str) -> StepDraft {
        StepDraft {
            id,
            text: text.to_owned(),
        }
    }

    fn step(id: u32, text: &str, done: bool) -> MicroStep {
        MicroStep {
            id,
            text: text.to_owned(),
            done,
        }
    }

    /// An idle session with the plan `texts` (ids 1, 2, ...).
    fn planned(texts: &[&str]) -> Session {
        let mut session = Session::new();
        let drafts = texts.iter().map(|text| draft(None, text)).collect();
        assert!(session.handle(at(0), Input::SetSteps(drafts)).is_empty());
        session
    }

    #[test]
    fn old_effects_keep_their_order() {
        // Every ADR-013 row with the Progress facts filtered out is exactly the pre-ADR-022 vector.
        let mut session = Session::new();
        assert_eq!(
            legacy(&session.handle(at(0), start_input(TimingProfile::demo()))),
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
        assert_eq!(
            legacy(&tick_through(&mut session, 0, 40)),
            break_transition(1, 20)
        );
        assert_eq!(
            legacy(&tick_through(&mut session, 40, 60)),
            focus_transition(2, 40)
        );
        assert_eq!(
            legacy(&session.handle(at(61), Input::SkipPhase)),
            break_transition(2, 20)
        );
        assert_eq!(
            legacy(&session.handle(at(62), Input::SkipPhase)),
            focus_transition(3, 40)
        );
        let effects = session.handle(at(63), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(legacy(&effects), stop_effects(Phase::Focus, 3, totals));

        let mut held = with_holds(vec![hold("m", PauseAtBreak, 45, 90)]);
        assert_eq!(
            legacy(&tick_through(&mut held, 0, 40)),
            held_break_transition(1, Some("Meeting m"))
        );
        let effects = held.handle(at(50), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(legacy(&effects), stop_effects(Phase::Break, 1, totals));
    }

    #[test]
    fn every_transition_reports_its_fact_in_place() {
        let mut session = started(TimingProfile::demo());
        // Focus → Break at the deadline: the fact right before the toast.
        assert_eq!(
            tick_through(&mut session, 0, 40),
            vec![
                Effect::PhaseChanged(PhaseChange {
                    from: Phase::Focus,
                    to: Phase::Break,
                    cycle: 1,
                }),
                Effect::Tracking(false),
                Effect::Shields(false),
                Effect::Chime(ChimeKind::BreakStart),
                focus_ended(1, 40, PhaseEnd::Deadline, false),
                Effect::Notify(NotifyKind::BreakStarted { break_secs: 20 }),
            ]
        );
        // Break → Focus at the deadline: after the chime, before the toast and the shields.
        assert_eq!(
            tick_through(&mut session, 40, 60),
            vec![
                Effect::PhaseChanged(PhaseChange {
                    from: Phase::Break,
                    to: Phase::Focus,
                    cycle: 2,
                }),
                Effect::Chime(ChimeKind::FocusStart),
                break_ended(1, 20, PhaseEnd::Deadline),
                Effect::Notify(NotifyKind::FocusStarted {
                    focus_secs: 40,
                    cycle: 2,
                }),
                Effect::Shields(true),
                Effect::Tracking(true),
            ]
        );
        // Skips report `Skipped` with the time that phase ran.
        let skipped = session.handle(at(65), Input::SkipPhase);
        assert_eq!(skipped[4], focus_ended(2, 5, PhaseEnd::Skipped, false));
        assert_eq!(legacy(&skipped), break_transition(2, 20));
        let skipped = session.handle(at(68), Input::SkipPhase);
        assert_eq!(skipped[2], break_ended(2, 3, PhaseEnd::Skipped));
        assert_eq!(legacy(&skipped), focus_transition(3, 40));

        // Stop in a running focus: the fact after the phase change, before the summary.
        let effects = session.handle(at(75), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(
            effects,
            vec![
                Effect::Tracking(false),
                Effect::Shields(false),
                Effect::PhaseChanged(PhaseChange {
                    from: Phase::Focus,
                    to: Phase::Idle,
                    cycle: 3,
                }),
                focus_ended(3, 7, PhaseEnd::Stopped, false),
                Effect::SessionEnded(totals),
            ]
        );
    }

    #[test]
    fn stop_in_a_paused_focus_and_in_a_break_reports_the_phase_time() {
        let mut paused = started(TimingProfile::demo());
        tick_through(&mut paused, 0, 12);
        paused.handle(at(12), Input::Pause);
        let effects = paused.handle(at(90), Input::Stop);
        assert_eq!(effects[3], focus_ended(1, 12, PhaseEnd::Stopped, false));
        assert!(matches!(effects[4], Effect::SessionEnded(_)));

        let mut on_break = started(TimingProfile::demo());
        tick_through(&mut on_break, 0, 47);
        let effects = on_break.handle(at(47), Input::Stop);
        let totals = only_totals(&effects);
        assert_eq!(
            effects,
            vec![
                Effect::Tracking(false),
                Effect::Shields(false),
                Effect::PhaseChanged(PhaseChange {
                    from: Phase::Break,
                    to: Phase::Idle,
                    cycle: 1,
                }),
                break_ended(1, 7, PhaseEnd::Stopped),
                Effect::SessionEnded(totals),
            ]
        );
    }

    #[test]
    fn a_held_break_reports_held_before_its_toast() {
        let mut session = with_holds(vec![hold("m", PauseAtBreak, 45, 90)]);
        assert_eq!(
            tick_through(&mut session, 0, 40),
            vec![
                Effect::PhaseChanged(PhaseChange {
                    from: Phase::Focus,
                    to: Phase::Break,
                    cycle: 1,
                }),
                Effect::Tracking(false),
                Effect::Shields(false),
                Effect::Chime(ChimeKind::BreakStart),
                focus_ended(1, 40, PhaseEnd::Deadline, true),
                Effect::Notify(NotifyKind::BreakHeld {
                    subject: Some("Meeting m".to_owned()),
                }),
            ]
        );
        // Stopped while still held: the break never ran.
        let effects = session.handle(at(70), Input::Stop);
        assert_eq!(effects[3], break_ended(1, 0, PhaseEnd::Stopped));
        assert_eq!(progress_of(&effects).len(), 1);
    }

    #[test]
    fn ran_secs_counts_running_time_only_and_includes_an_extension() {
        // Paused 10 s → 50 s: the block ran 40 s.
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        session.handle(at(10), Input::Pause);
        session.handle(at(50), Input::Resume);
        assert_eq!(
            progress_of(&tick_through(&mut session, 50, 80)),
            [ProgressFact::FocusEnded {
                session_id: "s1".to_owned(),
                cycle: 1,
                ran_secs: 40,
                how: PhaseEnd::Deadline,
                held: false,
                profile: TimingProfile::demo(),
            }]
        );

        // A sleep gap is not counted and its auto-pause reports nothing.
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 10);
        let interrupted = session.handle(at(101), Input::Tick);
        assert!(progress_of(&interrupted).is_empty());
        session.handle(at(110), Input::Resume);
        let effects = tick_through(&mut session, 110, 140);
        assert_eq!(effects[4], focus_ended(1, 40, PhaseEnd::Deadline, false));

        // A meeting extension counts as focus.
        let mut session = with_holds(vec![hold("a", Extend, 50, 70)]);
        assert!(progress_of(&tick_through(&mut session, 0, 69)).is_empty());
        let effects = session.handle(at(70), Input::Tick);
        assert_eq!(effects[4], focus_ended(1, 70, PhaseEnd::Deadline, false));

        // Developer mode's shorter phase reports the time really spent.
        let mut session = started(TimingProfile::demo());
        tick_through(&mut session, 0, 5);
        session.handle(at(5), Input::DevSetRemaining { secs: 2 });
        let effects = tick_through(&mut session, 5, 7);
        assert_eq!(effects[4], focus_ended(1, 7, PhaseEnd::Deadline, false));
    }

    #[test]
    fn inputs_without_a_phase_change_report_nothing() {
        let mut session = started(TimingProfile::demo());
        for (secs, input) in [
            (1, Input::Pause),
            (2, Input::Resume),
            (3, Input::SetGoal(Some(goal("A-1")))),
            (4, Input::SetMeetingHolds(Vec::new())),
            (5, Input::DevSetRemaining { secs: 30 }),
            (6, Input::SetSteps(vec![draft(None, "a")])),
            (7, Input::SetStepDone { id: 1, done: false }),
            (8, Input::Tick),
        ] {
            assert!(
                progress_of(&session.handle(at(secs), input)).is_empty(),
                "{secs}"
            );
        }
        // Inputs while idle report nothing either (also a double Stop).
        session.handle(at(9), Input::Stop);
        assert!(session.handle(at(10), Input::Stop).is_empty());
        assert!(
            session
                .handle(at(11), Input::SetStepDone { id: 1, done: true })
                .is_empty()
        );
    }

    #[test]
    fn steps_are_cleaned_and_limited() {
        let long = "x".repeat(85);
        let cleaned = normalize_steps(vec![
            draft(None, "  Napisz tytuł  "),
            draft(Some(3), ""),
            draft(None, " \t\n "),
            draft(Some(7), "a\tb\nc"),
            draft(None, &long),
        ])
        .expect("three steps");
        assert_eq!(cleaned.len(), 3);
        assert_eq!(cleaned[0], draft(None, "Napisz tytuł"));
        assert_eq!(cleaned[1], draft(Some(7), "a b c"));
        assert_eq!(cleaned[2].text.chars().count(), MAX_STEP_CHARS);
        assert!(cleaned[2].text.ends_with('…'));
        assert_eq!(
            cleaned[2].text,
            format!("{}…", "x".repeat(MAX_STEP_CHARS - 1))
        );
        // Exactly 80 characters stay as they are; a space before the cut is not kept.
        let exact = "y".repeat(MAX_STEP_CHARS);
        assert_eq!(
            normalize_steps(vec![draft(None, &exact)]).expect("ok")[0].text,
            exact
        );
        let spaced = format!("{} {}", "z".repeat(78), "w".repeat(10));
        assert_eq!(
            normalize_steps(vec![draft(None, &spaced)]).expect("ok")[0].text,
            format!("{}…", "z".repeat(78))
        );

        // Five steps (plus empty ones) are fine, six are not.
        let mut five: Vec<StepDraft> = (0..5).map(|n| draft(None, &format!("s{n}"))).collect();
        five.push(draft(None, "   "));
        assert_eq!(normalize_steps(five.clone()).expect("five").len(), 5);
        five.push(draft(None, "s5"));
        let error = normalize_steps(five).expect_err("six");
        assert_eq!(error.code, ERR_TOO_MANY_STEPS);
        assert_eq!(error.code, "tooManySteps");
        assert_eq!(error.field.as_deref(), Some("steps"));
        assert!(!error.message.contains("s5"), "never echoes a text");
    }

    #[test]
    fn step_ids_are_stable_and_new_ids_go_above_every_issued_one() {
        let mut session = planned(&["a", "b"]);
        assert_eq!(session.steps(), [step(1, "a", false), step(2, "b", false)]);

        session.handle(
            at(1),
            Input::SetSteps(vec![draft(Some(2), "b2"), draft(None, "c")]),
        );
        assert_eq!(session.steps(), [step(2, "b2", false), step(3, "c", false)]);
        // A removed id is never reused: it comes back as a new step.
        session.handle(at(2), Input::SetSteps(vec![draft(Some(1), "a again")]));
        assert_eq!(session.steps(), [step(4, "a again", false)]);
        session.handle(at(3), Input::SetSteps(Vec::new()));
        assert!(session.steps().is_empty());
        session.handle(at(4), Input::SetSteps(vec![draft(None, "z")]));
        assert_eq!(session.steps(), [step(5, "z", false)]);
        // A repeated id counts once; anything above the limit is dropped; texts are cleaned again.
        let drafts = vec![
            draft(Some(5), "z"),
            draft(Some(5), "dup"),
            draft(None, "  "),
            draft(None, "  three\t"),
            draft(None, "four"),
            draft(None, "five"),
            draft(None, "six"),
        ];
        session.handle(at(5), Input::SetSteps(drafts));
        let ids: Vec<u32> = session.steps().iter().map(|s| s.id).collect();
        assert_eq!(ids, [5, 6, 7, 8, 9]);
        assert_eq!(session.steps()[2].text, "three");
    }

    #[test]
    fn ticking_is_for_a_running_session_and_reports_the_first_tick_only_once() {
        let mut session = planned(&["a", "b"]);
        // Idle: no ticks.
        assert!(
            session
                .handle(at(0), Input::SetStepDone { id: 1, done: true })
                .is_empty()
        );
        assert!(!session.steps()[0].done);

        let effects = session.handle(at(0), start_input(TimingProfile::demo()));
        assert_eq!(
            progress_of(&effects),
            [ProgressFact::Started {
                session_id: "s1".to_owned(),
                steps: 2,
                profile: TimingProfile::demo(),
            }]
        );
        let tick = |session: &mut Session, secs, id, done| {
            session.handle(at(secs), Input::SetStepDone { id, done })
        };
        assert_eq!(
            tick(&mut session, 5, 1, true),
            [Effect::Progress(ProgressFact::StepDone {
                session_id: "s1".to_owned(),
                step_id: 1,
                first_time: true,
                done: 1,
                total: 2,
                phase: Phase::Focus,
                profile: TimingProfile::demo(),
            })]
        );
        assert!(session.steps()[0].done);
        // No change, an untick and an unknown id report nothing.
        assert!(tick(&mut session, 6, 1, true).is_empty());
        assert!(tick(&mut session, 7, 1, false).is_empty());
        assert!(!session.steps()[0].done);
        assert!(tick(&mut session, 8, 99, true).is_empty());
        // Ticked again: not a first tick any more.
        let again = progress_of(&tick(&mut session, 9, 1, true));
        assert!(matches!(
            again[..],
            [ProgressFact::StepDone {
                first_time: false,
                done: 1,
                ..
            }]
        ));
        // In a paused break too, with the phase it happened in.
        session.handle(at(10), Input::SkipPhase);
        session.handle(at(11), Input::Pause);
        let in_break = progress_of(&tick(&mut session, 12, 2, true));
        assert!(matches!(
            in_break[..],
            [ProgressFact::StepDone {
                step_id: 2,
                first_time: true,
                done: 2,
                total: 2,
                phase: Phase::Break,
                ..
            }]
        ));
    }

    #[test]
    fn steps_survive_start_and_open_ones_carry_over_at_stop() {
        let mut session = planned(&["a", "b", "c"]);
        let idle = session.snapshot(at(0));
        assert_eq!(idle.steps.len(), 3);
        assert_eq!(idle.phase, Phase::Idle);

        session.handle(at(0), start_input(TimingProfile::demo()));
        assert_eq!(session.snapshot(at(1)).steps, idle.steps);
        session.handle(at(2), Input::SetStepDone { id: 2, done: true });
        assert!(session.snapshot(at(3)).steps[1].done);

        let totals = only_totals(&session.handle(at(5), Input::Stop));
        assert_eq!(
            totals.steps,
            [step(1, "a", false), step(2, "b", true), step(3, "c", false)]
        );
        // The open steps are the next plan, with their ids; nothing counts as ticked before.
        assert_eq!(session.steps(), [step(1, "a", false), step(3, "c", false)]);
        assert_eq!(session.snapshot(at(6)).steps, session.steps());
        session.handle(at(7), start_input(TimingProfile::demo()));
        let first = progress_of(&session.handle(at(8), Input::SetStepDone { id: 1, done: true }));
        assert!(matches!(
            first[..],
            [ProgressFact::StepDone {
                first_time: true,
                ..
            }]
        ));
        // A new step after the carry-over gets an id above every one given out.
        session.handle(
            at(9),
            Input::SetSteps(vec![draft(Some(1), "a"), draft(None, "d")]),
        );
        assert_eq!(session.steps(), [step(1, "a", true), step(4, "d", false)]);
        // A session without steps ends with none.
        let mut plain = started(TimingProfile::demo());
        assert!(
            only_totals(&plain.handle(at(1), Input::Stop))
                .steps
                .is_empty()
        );
    }

    #[test]
    fn steps_never_change_phases_or_other_effects() {
        let mut with_steps = planned(&["a", "b"]);
        let mut without = Session::new();
        let inputs = |secs: u64| match secs {
            0 => start_input(TimingProfile::demo()),
            10 => Input::Pause,
            20 => Input::Resume,
            70 => Input::SkipPhase,
            95 => Input::Stop,
            _ => Input::Tick,
        };
        for secs in 0..=96 {
            let mut a = with_steps.handle(at(secs), inputs(secs));
            if secs == 5 || secs == 75 {
                a.extend(with_steps.handle(
                    at(secs),
                    Input::SetStepDone {
                        id: 1,
                        done: secs == 5,
                    },
                ));
                a.extend(with_steps.handle(at(secs), Input::SetSteps(vec![draft(Some(1), "a")])));
            }
            let b = without.handle(at(secs), inputs(secs));
            // The summary lists the steps; everything else is the same.
            let no_steps = |effects: Vec<Effect>| -> Vec<Effect> {
                legacy(&effects)
                    .into_iter()
                    .map(|effect| match effect {
                        Effect::SessionEnded(totals) => Effect::SessionEnded(SessionTotals {
                            steps: Vec::new(),
                            ..totals
                        }),
                        other => other,
                    })
                    .collect()
            };
            assert_eq!(no_steps(a), no_steps(b), "at {secs} s");
            assert_eq!(
                with_steps.snapshot(at(secs)).remaining_secs,
                without.snapshot(at(secs)).remaining_secs
            );
            assert_eq!(with_steps.phase(), without.phase());
        }
    }

    // -----------------------------------------------------------------------
    // Smart breaks (ADR-023): normal profile 45/15 unless said otherwise
    // -----------------------------------------------------------------------

    use crate::api::SmartBreakKind;

    const MINUTE: u64 = 60;

    fn smart_meeting(id: &str, start_secs: u64, end_secs: u64, eligible: bool) -> SmartMeeting {
        SmartMeeting {
            id: id.to_owned(),
            start_ms: at(start_secs).wall_ms,
            end_ms: at(end_secs).wall_ms,
            eligible,
        }
    }

    /// A session started at 0 s with `meetings` set before the start.
    fn smart_session(profile: TimingProfile, meetings: Vec<SmartMeeting>) -> Session {
        let mut session = Session::new();
        assert!(
            session
                .handle(at(0), Input::SetSmartMeetings(meetings))
                .is_empty()
        );
        session.handle(at(0), start_input(profile));
        session
    }

    /// The expected `SmartBreak`: meeting `(start, end)` and moved break `(start, end)` in seconds after the start.
    fn moved(
        id: &str,
        meeting: (u64, u64),
        number: u32,
        kind: SmartBreakKind,
        delta_secs: i32,
        window: (u64, u64),
    ) -> Option<SmartBreak> {
        Some(SmartBreak {
            meeting_id: id.to_owned(),
            meeting_start_ms: at(meeting.0).wall_ms,
            meeting_end_ms: at(meeting.1).wall_ms,
            break_number: number,
            kind,
            focus_delta_secs: delta_secs,
            break_start_ms: at(window.0).wall_ms,
            break_end_ms: at(window.1).wall_ms,
        })
    }

    #[test]
    fn smart_breaks_without_a_colliding_meeting_change_nothing() {
        // Demo: breaks at 40–60 s and 100–120 s; the meeting at 62–70 s lies inside the second focus block.
        let mut plain = started(TimingProfile::demo());
        let mut smart = smart_session(
            TimingProfile::demo(),
            vec![smart_meeting("inside", 62, 70, true)],
        );
        for secs in 1..=130 {
            let a = plain.handle(at(secs), Input::Tick);
            let b = smart.handle(at(secs), Input::Tick);
            assert_eq!(a, b, "at {secs} s");
            assert_eq!(plain.snapshot(at(secs)), smart.snapshot(at(secs)));
            assert_eq!(smart.snapshot(at(secs)).smart_break, None);
        }
        // Idle never shows a moved break, whatever the meetings.
        let mut idle = Session::new();
        idle.handle(
            at(0),
            Input::SetSmartMeetings(vec![smart_meeting("m", 50 * MINUTE, 90 * MINUTE, true)]),
        );
        assert_eq!(idle.snapshot(at(0)).smart_break, None);
        assert_eq!(idle.smart_meetings().len(), 1);
    }

    #[test]
    fn a_meeting_in_the_middle_of_the_break_ends_the_break_before_it() {
        // Meeting 0:57–1:30 after a start at 0:00: the break moves from 0:45 to 0:42 and ends when it starts.
        let meeting = (57 * MINUTE, 90 * MINUTE);
        let mut session = smart_session(
            TimingProfile::normal(),
            vec![smart_meeting("m", meeting.0, meeting.1, true)],
        );
        let expected = moved(
            "m",
            meeting,
            1,
            SmartBreakKind::EndsBeforeMeeting,
            -180,
            (42 * MINUTE, 57 * MINUTE),
        );
        let snap = session.snapshot(at(0));
        assert_eq!((snap.phase_total_secs, snap.remaining_secs), (2520, 2520));
        assert_eq!(snap.smart_break, expected);
        assert!(tick_through(&mut session, 0, 42 * MINUTE - 1).is_empty());
        // At the moved end: exactly the usual Focus → Break effects (ADR-013 order); the fact counts 42 minutes.
        let effects = session.handle(at(42 * MINUTE), Input::Tick);
        assert_eq!(legacy(&effects), break_transition(1, 900));
        assert!(
            effects.contains(&Effect::Progress(ProgressFact::FocusEnded {
                session_id: "s1".into(),
                cycle: 1,
                ran_secs: 2520,
                how: PhaseEnd::Deadline,
                held: false,
                profile: TimingProfile::normal(),
            }))
        );
        // The running break is the moved one, a normal 15-minute break.
        let snap = session.snapshot(at(42 * MINUTE));
        assert_eq!((snap.phase, snap.phase_total_secs), (Phase::Break, 900));
        assert_eq!(snap.smart_break, expected);
        // The next block starts with the meeting and holds all of it: a full focus, nothing moved.
        let effects = tick_through(&mut session, 42 * MINUTE, 57 * MINUTE);
        assert_eq!(legacy(&effects), focus_transition(2, 2700));
        let snap = session.snapshot(at(57 * MINUTE));
        assert_eq!((snap.phase_total_secs, snap.smart_break), (2700, None));
    }

    #[test]
    fn a_meeting_in_the_middle_of_the_focus_starts_the_break_with_it() {
        // 0:40–1:10: ending the break before it would leave a 25-minute block (< 60 %), so the break starts with it.
        let mut session = smart_session(
            TimingProfile::normal(),
            vec![smart_meeting("standup", 40 * MINUTE, 70 * MINUTE, true)],
        );
        assert_eq!(
            session.snapshot(at(0)).smart_break,
            moved(
                "standup",
                (40 * MINUTE, 70 * MINUTE),
                1,
                SmartBreakKind::StartsAtMeeting,
                -300,
                (40 * MINUTE, 55 * MINUTE)
            )
        );
        assert!(tick_through(&mut session, 0, 40 * MINUTE - 1).is_empty());
        let effects = session.handle(at(40 * MINUTE), Input::Tick);
        assert_eq!(legacy(&effects), break_transition(1, 900));
    }

    #[test]
    fn a_block_may_be_lengthened_by_a_quarter_at_most() {
        // 0:50–1:20: the break starts with it, the block is 5 minutes longer (smaller than ending it 10 min early).
        let mut session = smart_session(
            TimingProfile::normal(),
            vec![smart_meeting("m", 50 * MINUTE, 80 * MINUTE, true)],
        );
        let snap = session.snapshot(at(0));
        assert_eq!(snap.phase_total_secs, 3000);
        assert_eq!(
            snap.smart_break
                .as_ref()
                .map(|moved| moved.focus_delta_secs),
            Some(300)
        );
        // Nothing happens at the usual end.
        assert!(tick_through(&mut session, 0, 50 * MINUTE - 1).is_empty());
        assert_eq!(
            legacy(&session.handle(at(50 * MINUTE), Input::Tick)),
            break_transition(1, 900)
        );

        // 0:58: starting the break with it would be +13 min (more than 25 %, 11 min 15 s): it ends 2 min early instead.
        let session = smart_session(
            TimingProfile::normal(),
            vec![smart_meeting("late", 58 * MINUTE, 80 * MINUTE, true)],
        );
        assert_eq!(
            session
                .snapshot(at(0))
                .smart_break
                .map(|moved| (moved.kind, moved.focus_delta_secs)),
            Some((SmartBreakKind::EndsBeforeMeeting, -120))
        );
    }

    #[test]
    fn ask_me_instead_gives_back_the_unfitted_end() {
        let meeting = |eligible| vec![smart_meeting("m", 57 * MINUTE, 90 * MINUTE, eligible)];
        let mut session = smart_session(TimingProfile::normal(), meeting(true));
        tick_through(&mut session, 0, 10 * MINUTE);
        assert_eq!(session.snapshot(at(10 * MINUTE)).remaining_secs, 32 * 60);
        // "Ask me instead": the meeting may no longer be fitted. No effects; the block is 45 minutes again.
        assert!(
            session
                .handle(at(10 * MINUTE), Input::SetSmartMeetings(meeting(false)))
                .is_empty()
        );
        let snap = session.snapshot(at(10 * MINUTE));
        assert_eq!(
            (snap.remaining_secs, snap.phase_total_secs, snap.smart_break),
            (35 * 60, 2700, None)
        );
        // The same list again changes nothing; "fit it again" brings the fit back.
        assert!(
            session
                .handle(at(10 * MINUTE), Input::SetSmartMeetings(meeting(false)))
                .is_empty()
        );
        session.handle(at(10 * MINUTE), Input::SetSmartMeetings(meeting(true)));
        assert_eq!(session.snapshot(at(10 * MINUTE)).remaining_secs, 32 * 60);
        // Smart breaks switched off (an empty list): the plain end again.
        session.handle(at(10 * MINUTE), Input::SetSmartMeetings(Vec::new()));
        assert_eq!(session.snapshot(at(10 * MINUTE)).remaining_secs, 35 * 60);
    }

    #[test]
    fn undoing_a_lengthened_block_after_its_usual_end_starts_the_break_at_once() {
        let meeting = |eligible| vec![smart_meeting("m", 50 * MINUTE, 80 * MINUTE, eligible)];
        let mut session = smart_session(TimingProfile::normal(), meeting(true));
        assert!(tick_through(&mut session, 0, 47 * MINUTE).is_empty());
        session.handle(at(47 * MINUTE), Input::SetSmartMeetings(meeting(false)));
        assert_eq!(session.snapshot(at(47 * MINUTE)).remaining_secs, 0);
        let effects = session.handle(at(47 * MINUTE + 1), Input::Tick);
        assert_eq!(legacy(&effects), break_transition(1, 900));
        assert_eq!(session.snapshot(at(47 * MINUTE + 1)).smart_break, None);
    }

    #[test]
    fn smart_breaks_never_block_pause_skip_or_stop() {
        let meeting = (57 * MINUTE, 90 * MINUTE);
        let mut session = smart_session(
            TimingProfile::normal(),
            vec![smart_meeting("m", meeting.0, meeting.1, true)],
        );
        tick_through(&mut session, 0, 10 * MINUTE);
        assert_eq!(
            session.handle(at(10 * MINUTE), Input::Pause),
            vec![Effect::Tracking(false)]
        );
        // Paused two minutes: Resume fits the block again, so the break still ends when the meeting starts (0:57),
        // now 5 minutes shorter instead of 3.
        assert_eq!(
            session.handle(at(12 * MINUTE), Input::Resume),
            vec![Effect::Tracking(true)]
        );
        let snap = session.snapshot(at(12 * MINUTE));
        assert_eq!(snap.remaining_secs, 30 * 60);
        assert_eq!(
            snap.smart_break,
            moved(
                "m",
                meeting,
                1,
                SmartBreakKind::EndsBeforeMeeting,
                -300,
                (42 * MINUTE, 57 * MINUTE)
            )
        );
        // Skip: the break starts now (the usual effects) and is not the moved one.
        let effects = session.handle(at(13 * MINUTE), Input::SkipPhase);
        assert_eq!(legacy(&effects), break_transition(1, 900));
        assert_eq!(session.snapshot(at(13 * MINUTE)).smart_break, None);
        // Stop works as always.
        let effects = session.handle(at(14 * MINUTE), Input::Stop);
        assert_eq!(
            legacy(&effects),
            stop_effects(Phase::Break, 1, only_totals(&effects))
        );
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn a_paused_block_is_fitted_as_if_it_resumed_now() {
        let mut session = started(TimingProfile::normal());
        tick_through(&mut session, 0, 10 * MINUTE);
        session.handle(at(10 * MINUTE), Input::Pause);
        // The calendar reports a meeting while paused: planned as if resumed now, the break is due at 0:50; the
        // meeting at 1:02 is 12 min into it, so the frozen time left shrinks by 3 minutes.
        session.handle(
            at(15 * MINUTE),
            Input::SetSmartMeetings(vec![smart_meeting("m", 62 * MINUTE, 90 * MINUTE, true)]),
        );
        let snap = session.snapshot(at(20 * MINUTE));
        assert!(snap.paused);
        assert_eq!(snap.remaining_secs, 32 * 60);
        assert_eq!(snap.phase_total_secs, 42 * 60);
    }

    #[test]
    fn a_fixed_block_keeps_its_end() {
        let fitted = |eligible| vec![smart_meeting("m", 57 * MINUTE, 90 * MINUTE, eligible)];
        // Developer page: the time left was set; a later change of the meetings does not move it.
        let mut session = smart_session(TimingProfile::normal(), fitted(true));
        session.handle(at(60), Input::DevSetRemaining { secs: 120 });
        session.handle(at(60), Input::SetSmartMeetings(fitted(false)));
        let snap = session.snapshot(at(60));
        assert_eq!((snap.remaining_secs, snap.smart_break), (120, None));

        // An `Extend` hold reaching the moved break (only with inconsistent inputs) extends the block as usual and
        // decides its end from then on.
        let mut session = smart_session(TimingProfile::normal(), fitted(true));
        session.handle(
            at(0),
            Input::SetMeetingHolds(vec![hold("h", Extend, 42 * MINUTE, 50 * MINUTE)]),
        );
        assert!(tick_through(&mut session, 0, 42 * MINUTE).is_empty());
        let snap = session.snapshot(at(42 * MINUTE));
        assert_eq!(snap.phase, Phase::Focus);
        assert_eq!(snap.smart_break, None);
        assert!(snap.meeting_hold.is_some());
        session.handle(at(43 * MINUTE), Input::SetSmartMeetings(fitted(false)));
        assert_eq!(session.snapshot(at(43 * MINUTE)).remaining_secs, 7 * 60);
        let effects = tick_through(&mut session, 43 * MINUTE, 50 * MINUTE);
        assert_eq!(legacy(&effects), break_transition(1, 900));
    }

    #[test]
    fn back_to_back_blocks_are_fitted_one_by_one_in_the_demo_profile() {
        // Demo 40/20: blocks of 24..=50 s. "a" at 50–60 s moves break 1 to 30–50 s; "b" at 100–130 s moves break 2
        // (nominally 90–110 s) to 80–100 s. Both times −10 s and +10 s tie: the break before the meeting wins.
        let mut session = smart_session(
            TimingProfile::demo(),
            vec![
                smart_meeting("a", 50, 60, true),
                smart_meeting("b", 100, 130, true),
            ],
        );
        assert_eq!(
            session.snapshot(at(0)).smart_break,
            moved(
                "a",
                (50, 60),
                1,
                SmartBreakKind::EndsBeforeMeeting,
                -10,
                (30, 50)
            )
        );
        let effects = tick_through(&mut session, 0, 30);
        assert_eq!(legacy(&effects), break_transition(1, 20));
        let effects = tick_through(&mut session, 30, 50);
        assert_eq!(legacy(&effects), focus_transition(2, 40));
        let snap = session.snapshot(at(50));
        assert_eq!(snap.phase_total_secs, 30);
        assert_eq!(
            snap.smart_break,
            moved(
                "b",
                (100, 130),
                2,
                SmartBreakKind::EndsBeforeMeeting,
                -10,
                (80, 100)
            )
        );
        let effects = tick_through(&mut session, 50, 80);
        assert_eq!(legacy(&effects), break_transition(2, 20));
    }

    #[test]
    fn an_answered_or_day_long_meeting_is_left_to_the_question() {
        for meetings in [
            vec![smart_meeting("answered", 57 * MINUTE, 90 * MINUTE, false)],
            // A day-long block (an all-day event that slipped through as timed).
            vec![smart_meeting("all-day", 0, 24 * 60 * MINUTE, true)],
        ] {
            let session = smart_session(TimingProfile::normal(), meetings);
            let snap = session.snapshot(at(0));
            assert_eq!((snap.phase_total_secs, snap.smart_break), (2700, None));
        }
    }

    #[test]
    fn after_a_sleep_gap_the_block_is_fitted_on_resume() {
        let meeting = vec![smart_meeting("m", 57 * MINUTE, 90 * MINUTE, true)];
        let mut session = smart_session(TimingProfile::normal(), meeting);
        tick_through(&mut session, 0, 5 * MINUTE);
        // Asleep 10 minutes: the Tick pauses with the time left at 0:05 (no time counted).
        let after = at(15 * MINUTE);
        assert_eq!(
            session.handle(after, Input::Tick),
            vec![
                Effect::Tracking(false),
                Effect::Notify(NotifyKind::Interrupted)
            ]
        );
        assert_eq!(session.snapshot(after).remaining_secs, 37 * 60);
        // Resume: the unfitted break is due at 0:55 now and the meeting at 0:57 is 2 minutes into it; starting the
        // break with it (+2 min) is a smaller change than ending it before (−13 min).
        session.handle(after, Input::Resume);
        let snap = session.snapshot(after);
        assert_eq!(
            snap.smart_break
                .map(|moved| (moved.kind, moved.focus_delta_secs)),
            Some((SmartBreakKind::StartsAtMeeting, 120))
        );
        assert_eq!(snap.remaining_secs, 42 * 60);
    }
}
