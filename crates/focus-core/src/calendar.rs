//! Calendar meetings against the planned breaks (pure).
//!
//! The shell fetches meetings (Microsoft Graph, Google Calendar, ICS links) and asks here which
//! breaks of the running session (or of a session that would start now) they collide with, and when
//! the next "remind later" popup for a meeting is due. All times are wall clock, Unix epoch
//! milliseconds, passed in by the caller; nothing here reads a clock.
//!
//! Smart breaks (ADR-023): [`fit_break`] moves one break around a meeting by shortening or lengthening the focus
//! block before it (within the bounds of [`TimingProfile::smart_min_focus_ms`] / [`TimingProfile::smart_max_focus_ms`]);
//! [`plan_smart_breaks`] plans a whole session that way. The session fits its own running block with the same
//! function, so the plan the calendar shows and the phase end the session uses agree.

use crate::api::{Meeting, SmartBreak, SmartBreakKind};
use crate::timing::{MAX_MEETING_HOLD_MS, MEETING_REMINDER_LEAD_MINUTES, TimingProfile};

/// [`plan_breaks`] never returns more breaks than this (e.g. 1-second phases over a 24-hour horizon).
pub const MAX_PLANNED_BREAKS: usize = 512;

const MS_PER_SEC: i64 = 1000;
const MS_PER_MINUTE: i64 = 60 * MS_PER_SEC;

/// One planned break, `[start_ms, end_ms)` on the wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakWindow {
    /// 1-based number of the break in the session (the break after focus cycle `n` is break `n`).
    pub number: u32,
    pub start_ms: i64,
    pub end_ms: i64,
}

impl BreakWindow {
    /// `true` when `[start_ms, end_ms)` overlaps this break; touching edges do not count.
    pub fn overlaps(&self, start_ms: i64, end_ms: i64) -> bool {
        start_ms < end_ms && start_ms < self.end_ms && self.start_ms < end_ms
    }
}

/// The breaks that start before `now_ms + horizon_ms`, in order (at most [`MAX_PLANNED_BREAKS`]).
///
/// - Idle (both `None`): a session that would start now; breaks are numbered from 1 (`cycle` is ignored).
/// - Focus (`focus_remaining_ms: Some(left)`, time until its break): the first break is break `cycle`
///   at `now_ms + left`.
/// - Break (`in_break_remaining_ms: Some(left)`): the running break is not listed; the next one is
///   break `cycle + 1`, after the rest of this break and a full focus.
///
/// A paused phase is planned as if it resumed now (pass its frozen remaining time). When both are
/// `Some`, the focus value wins. Negative remaining times count as 0, phases are at least 1 second
/// long (as in the session) and all arithmetic saturates.
pub fn plan_breaks(
    now_ms: i64,
    focus_remaining_ms: Option<i64>,
    in_break_remaining_ms: Option<i64>,
    cycle: u32,
    profile: &TimingProfile,
    horizon_ms: i64,
) -> Vec<BreakWindow> {
    let focus_ms = phase_ms(profile.focus_secs);
    let break_ms = phase_ms(profile.break_secs);
    let horizon_end = now_ms.saturating_add(horizon_ms.max(0));
    let (mut start, mut number) = match (focus_remaining_ms, in_break_remaining_ms) {
        (Some(left), _) => (now_ms.saturating_add(left.max(0)), cycle.max(1)),
        (None, Some(left)) => (
            now_ms.saturating_add(left.max(0)).saturating_add(focus_ms),
            cycle.saturating_add(1),
        ),
        (None, None) => (now_ms.saturating_add(focus_ms), 1),
    };

    let mut breaks = Vec::new();
    while start < horizon_end && breaks.len() < MAX_PLANNED_BREAKS {
        let end = start.saturating_add(break_ms);
        breaks.push(BreakWindow {
            number,
            start_ms: start,
            end_ms: end,
        });
        let next = end.saturating_add(focus_ms);
        if next <= start {
            // Saturated at the end of time.
            break;
        }
        start = next;
        number = number.saturating_add(1);
    }
    breaks
}

/// Meetings that overlap a break: `(index into meetings, the first break it overlaps)`, in meeting
/// order. A meeting overlaps a break when `[start, end)` intersects the break window (a meeting that
/// ends exactly when the break starts, or starts exactly when it ends, does not). A meeting spanning
/// several breaks is reported once, with the first one. Meetings with `end <= start` are skipped.
pub fn conflicts(breaks: &[BreakWindow], meetings: &[Meeting]) -> Vec<(usize, BreakWindow)> {
    meetings
        .iter()
        .enumerate()
        .filter_map(|(index, meeting)| {
            breaks
                .iter()
                .find(|window| window.overlaps(meeting.start_ms, meeting.end_ms))
                .map(|window| (index, *window))
        })
        .collect()
}

/// When the next "remind later" popup for a meeting starting at `meeting_start_ms` is due, and
/// whether that popup may offer "remind later" again.
///
/// `stage` is how many times "remind later" was already chosen for this meeting (0 for the first
/// choice, on the Focus page). Stage `n` is due [`MEETING_REMINDER_LEAD_MINUTES`]`[n]` minutes
/// before the meeting (30, 20, 10); a stage whose time is not after `now_ms` is skipped in favour
/// of the next one. The last stage (10 minutes before) returns `false`: that popup has no
/// "remind later". `None` when no stage is left (the meeting is 10 minutes away or closer, or all
/// stages were used): "remind later" should not be offered at all then.
pub fn reminder_schedule(meeting_start_ms: i64, now_ms: i64, stage: u32) -> Option<(i64, bool)> {
    let first = usize::try_from(stage).ok()?;
    let last = MEETING_REMINDER_LEAD_MINUTES.len().checked_sub(1)?;
    MEETING_REMINDER_LEAD_MINUTES
        .iter()
        .enumerate()
        .skip(first)
        .find_map(|(index, lead_minutes)| {
            let due = meeting_start_ms
                .saturating_sub(i64::from(*lead_minutes).saturating_mul(MS_PER_MINUTE));
            (due > now_ms).then_some((due, index < last))
        })
}

/// A meeting of `[start_ms, end_ms)` may extend a focus or hold a break (`Extend` / `PauseAtBreak`): it has a
/// length and lasts at most [`MAX_MEETING_HOLD_MS`] (4 hours; exactly 4 hours is still allowed). The session
/// ignores holds of longer meetings and the UI marks them (`MeetingConflict.too_long`).
pub fn meeting_hold_allowed(start_ms: i64, end_ms: i64) -> bool {
    end_ms > start_ms && end_ms.abs_diff(start_ms) <= MAX_MEETING_HOLD_MS
}

/// A meeting as smart breaks see it (ADR-023): wall-clock `[start_ms, end_ms)` and whether a break may be moved
/// around it. A meeting the user answered (`MeetingChoice`) or opted out of ("Ask me instead") is not `eligible`:
/// a break colliding with it is left to the user, and no moved break may land on it. No subject: the session keeps
/// these and they never need one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartMeeting {
    /// `Meeting.id`.
    pub id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub eligible: bool,
}

impl SmartMeeting {
    /// `[start, end)` intersects `[from_ms, to_ms)`; touching edges and meetings without length do not count.
    fn overlaps(&self, from_ms: i64, to_ms: i64) -> bool {
        self.start_ms < self.end_ms && self.start_ms < to_ms && from_ms < self.end_ms
    }
}

/// How smart breaks move one break (ADR-023).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakFit {
    /// Index of the meeting (in the slice given to [`fit_break`]) the break was moved for.
    pub meeting: usize,
    pub kind: SmartBreakKind,
    /// The moved break starts here (wall clock): the focus block before it now ends here.
    pub break_start_ms: i64,
    /// Change of that focus block in milliseconds: negative = shortened, positive = lengthened.
    pub delta_ms: i64,
}

/// Smart breaks (ADR-023): moves the break that would start at `break_start_ms` (wall clock, the nominal end of the
/// focus block before it; may lie before `now_ms` when a running block was already lengthened past it) around a
/// meeting, or `None` when it stays where it is.
///
/// - Only meetings that collide with the break (`[start, start + break)`, touching edges do not count) matter; with
///   none, nothing moves.
/// - When any of them is not `eligible` (the user decides) or is longer than [`MAX_MEETING_HOLD_MS`] (a day-long
///   block, an all-day event that slipped through), nothing moves either: the usual question stays.
/// - The break is moved for the colliding meeting that starts first: so that it ends when the meeting starts
///   ([`SmartBreakKind::EndsBeforeMeeting`]) or starts with it ([`SmartBreakKind::StartsAtMeeting`]).
/// - A variant is possible when the focus block keeps a length within
///   [`TimingProfile::smart_min_focus_ms`]..=[`TimingProfile::smart_max_focus_ms`] (a focus shorter than the 10-minute
///   floor is never shortened, only lengthened), the moved break does not start
///   before `now_ms`, it overlaps no other meeting (for `StartsAtMeeting` the target meeting itself is expected) and
///   the meeting ends by the time the full focus block after the moved break ends: a longer meeting would collide
///   with the next break, so it is left to the question (where "extend" fits it best).
/// - Of the possible variants the one with the smaller change wins; on a tie the break before the meeting.
///   No possible variant: `None` (the meeting gets the usual question).
///
/// All arithmetic saturates (wide integers inside); nothing panics.
pub fn fit_break(
    now_ms: i64,
    break_start_ms: i64,
    profile: &TimingProfile,
    meetings: &[SmartMeeting],
) -> Option<BreakFit> {
    let break_ms = phase_ms(profile.break_secs);
    let window_end = break_start_ms.saturating_add(break_ms);
    let colliding: Vec<(usize, &SmartMeeting)> = meetings
        .iter()
        .enumerate()
        .filter(|(_, meeting)| meeting.overlaps(break_start_ms, window_end))
        .collect();
    if colliding.iter().any(|(_, meeting)| {
        !meeting.eligible || !meeting_hold_allowed(meeting.start_ms, meeting.end_ms)
    }) {
        return None;
    }
    let (target, meeting) = colliding
        .into_iter()
        .min_by_key(|(index, meeting)| (meeting.start_ms, meeting.end_ms, *index))?;

    let focus_ms = i128::from(phase_ms(profile.focus_secs));
    // A focus shorter than the floor is never shortened, but it may still be lengthened.
    let min_ms = i128::from(profile.smart_min_focus_ms()).min(focus_ms);
    let max_ms = i128::from(profile.smart_max_focus_ms());
    let candidates = [
        (
            SmartBreakKind::EndsBeforeMeeting,
            meeting.start_ms.saturating_sub(break_ms),
        ),
        (SmartBreakKind::StartsAtMeeting, meeting.start_ms),
    ];
    candidates
        .into_iter()
        .filter_map(|(kind, start)| {
            let delta = i128::from(start) - i128::from(break_start_ms);
            let block = focus_ms + delta;
            let end = start.saturating_add(break_ms);
            // `EndsBeforeMeeting` only touches the target; `StartsAtMeeting` overlaps it on purpose.
            let lands_on_other = meetings
                .iter()
                .enumerate()
                .any(|(index, other)| index != target && other.overlaps(start, end));
            // The next full focus block absorbs the meeting, so no later break collides with it.
            let absorbed = i128::from(meeting.end_ms) <= i128::from(end) + focus_ms;
            let possible = (min_ms..=max_ms).contains(&block)
                && start >= now_ms
                && !lands_on_other
                && absorbed;
            possible.then(|| BreakFit {
                meeting: target,
                kind,
                break_start_ms: start,
                delta_ms: i64::try_from(delta).unwrap_or(if delta < 0 {
                    i64::MIN
                } else {
                    i64::MAX
                }),
            })
        })
        .min_by_key(|fit| {
            (
                fit.delta_ms.unsigned_abs(),
                fit.kind != SmartBreakKind::EndsBeforeMeeting,
            )
        })
}

/// The first break of a plan, before smart breaks: when it would start and its number (see [`plan_breaks`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstBreak {
    /// Nominal start, wall clock. Before `now_ms` when a running focus block was lengthened past its nominal end.
    pub start_ms: i64,
    /// 1-based number of the break.
    pub number: u32,
    /// The focus block before it runs already and its end is fixed (extended by a meeting hold or set on the
    /// Developer page): smart breaks leave it alone. Later blocks are always fitted.
    pub fixed: bool,
}

impl FirstBreak {
    /// As in [`plan_breaks`]: Idle (both `None`): a session that would start now, break 1 after a full focus;
    /// Focus: break `cycle` at `now_ms + focus_remaining_ms` (not clamped: a negative value is a block running past its
    /// nominal end); Break: break `cycle + 1` after the rest of this break (at least 0) and a full focus.
    pub fn new(
        now_ms: i64,
        focus_remaining_ms: Option<i64>,
        in_break_remaining_ms: Option<i64>,
        cycle: u32,
        profile: &TimingProfile,
    ) -> Self {
        let focus_ms = phase_ms(profile.focus_secs);
        let (start_ms, number) = match (focus_remaining_ms, in_break_remaining_ms) {
            (Some(left), _) => (now_ms.saturating_add(left), cycle.max(1)),
            (None, Some(left)) => (
                now_ms.saturating_add(left.max(0)).saturating_add(focus_ms),
                cycle.saturating_add(1),
            ),
            (None, None) => (now_ms.saturating_add(focus_ms), 1),
        };
        Self {
            start_ms,
            number,
            fixed: false,
        }
    }
}

/// A session plan with smart breaks applied (ADR-023).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartPlan {
    /// The breaks that start before the horizon, in order (moved ones where they were moved to), at most
    /// [`MAX_PLANNED_BREAKS`].
    pub breaks: Vec<BreakWindow>,
    /// Every moved break with its fit, in break order.
    pub moves: Vec<(BreakWindow, BreakFit)>,
}

/// Plans the breaks like [`plan_breaks`], but block by block with [`fit_break`]: a moved break moves every later
/// break with it (each focus block after a break is a full focus again). The first block is not fitted when
/// `first.fixed`; a first break due before `now_ms` without a fit starts at `now_ms`. With no meetings the result
/// equals [`plan_breaks`] for the same session.
pub fn plan_smart_breaks(
    now_ms: i64,
    first: FirstBreak,
    profile: &TimingProfile,
    horizon_ms: i64,
    meetings: &[SmartMeeting],
) -> SmartPlan {
    let focus_ms = phase_ms(profile.focus_secs);
    let break_ms = phase_ms(profile.break_secs);
    let horizon_end = now_ms.saturating_add(horizon_ms.max(0));
    let mut nominal = first.start_ms;
    let mut number = first.number;
    let mut fit_allowed = !first.fixed;
    let mut plan = SmartPlan::default();
    while plan.breaks.len() < MAX_PLANNED_BREAKS {
        let fit = if fit_allowed {
            fit_break(now_ms, nominal, profile, meetings)
        } else {
            None
        };
        let start = fit.map_or(nominal.max(now_ms), |fit| fit.break_start_ms);
        if start >= horizon_end {
            break;
        }
        let window = BreakWindow {
            number,
            start_ms: start,
            end_ms: start.saturating_add(break_ms),
        };
        plan.breaks.push(window);
        if let Some(fit) = fit {
            plan.moves.push((window, fit));
        }
        let next = window.end_ms.saturating_add(focus_ms);
        if next <= start {
            // Saturated at the end of time.
            break;
        }
        nominal = next;
        number = number.saturating_add(1);
        fit_allowed = true;
    }
    plan
}

/// The `SmartBreak` view of a moved break (`meeting` is the one the fit names). The change of the focus block is
/// rounded to whole seconds (saturating at the `i32` range).
pub fn smart_break_view(
    window: &BreakWindow,
    fit: &BreakFit,
    meeting: &SmartMeeting,
) -> SmartBreak {
    let half_up = fit.delta_ms.signum().saturating_mul(MS_PER_SEC / 2);
    let secs = fit.delta_ms.saturating_add(half_up) / MS_PER_SEC;
    SmartBreak {
        meeting_id: meeting.id.clone(),
        meeting_start_ms: meeting.start_ms,
        meeting_end_ms: meeting.end_ms,
        break_number: window.number,
        kind: fit.kind,
        focus_delta_secs: i32::try_from(secs).unwrap_or(if secs < 0 { i32::MIN } else { i32::MAX }),
        break_start_ms: window.start_ms,
        break_end_ms: window.end_ms,
    }
}

/// Phase length in milliseconds, at least one second (a zero-length phase would never advance).
fn phase_ms(secs: u32) -> i64 {
    i64::from(secs.max(1)).saturating_mul(MS_PER_SEC)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::CalendarSource;

    const T0: i64 = 1_760_000_000_000;
    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;

    fn profile(focus_minutes: u32, break_minutes: u32) -> TimingProfile {
        TimingProfile {
            focus_secs: focus_minutes * 60,
            break_secs: break_minutes * 60,
            ..TimingProfile::normal()
        }
    }

    fn window(number: u32, start_ms: i64, end_ms: i64) -> BreakWindow {
        BreakWindow {
            number,
            start_ms,
            end_ms,
        }
    }

    fn meeting(id: &str, start_ms: i64, end_ms: i64) -> Meeting {
        Meeting {
            id: id.to_owned(),
            source: CalendarSource::Ics,
            subject: format!("Meeting {id}"),
            start_ms,
            end_ms,
            online: false,
        }
    }

    #[test]
    fn idle_plans_a_session_that_would_start_now() {
        let breaks = plan_breaks(T0, None, None, 0, &profile(45, 15), 10 * HOUR);
        // Breaks start at 45, 105, ..., 585 minutes: ten of them before the 600-minute horizon.
        assert_eq!(breaks.len(), 10);
        assert_eq!(breaks[0], window(1, T0 + 45 * MIN, T0 + 60 * MIN));
        assert_eq!(breaks[1], window(2, T0 + 105 * MIN, T0 + 120 * MIN));
        assert_eq!(breaks[9], window(10, T0 + 585 * MIN, T0 + 600 * MIN));
        for (i, pair) in breaks.windows(2).enumerate() {
            assert_eq!(pair[1].start_ms - pair[0].start_ms, 60 * MIN, "pair {i}");
            assert_eq!(pair[1].number, pair[0].number + 1);
        }

        // The cycle of an idle session does not matter.
        assert_eq!(
            plan_breaks(T0, None, None, 7, &profile(45, 15), 10 * HOUR),
            breaks
        );
    }

    #[test]
    fn running_focus_plans_from_its_deadline() {
        let breaks = plan_breaks(T0, Some(10 * MIN), None, 3, &profile(45, 15), 3 * HOUR);
        assert_eq!(
            breaks,
            [
                window(3, T0 + 10 * MIN, T0 + 25 * MIN),
                window(4, T0 + 70 * MIN, T0 + 85 * MIN),
                window(5, T0 + 130 * MIN, T0 + 145 * MIN),
            ]
        );
        // A paused focus is planned the same way, with its frozen remaining time.
        assert_eq!(
            plan_breaks(
                T0,
                Some(10 * MIN),
                Some(99 * MIN),
                3,
                &profile(45, 15),
                3 * HOUR
            ),
            breaks,
            "the focus value wins"
        );
    }

    #[test]
    fn running_break_plans_the_next_one() {
        let breaks = plan_breaks(T0, None, Some(5 * MIN), 2, &profile(45, 15), 2 * HOUR);
        assert_eq!(
            breaks,
            [
                window(3, T0 + 50 * MIN, T0 + 65 * MIN),
                window(4, T0 + 110 * MIN, T0 + 125 * MIN),
            ]
        );
    }

    #[test]
    fn demo_profile_plans_in_seconds() {
        let breaks = plan_breaks(T0, None, None, 0, &TimingProfile::demo(), 2 * MIN);
        assert_eq!(
            breaks,
            [
                window(1, T0 + 40_000, T0 + 60_000),
                window(2, T0 + 100_000, T0 + 120_000),
            ]
        );
    }

    #[test]
    fn horizon_is_exclusive_and_never_negative() {
        let p = profile(45, 15);
        // The first break starts exactly at the horizon: not listed; one millisecond more and it is.
        assert!(plan_breaks(T0, None, None, 0, &p, 45 * MIN).is_empty());
        assert_eq!(plan_breaks(T0, None, None, 0, &p, 45 * MIN + 1).len(), 1);
        assert!(plan_breaks(T0, None, None, 0, &p, 0).is_empty());
        assert!(plan_breaks(T0, None, None, 0, &p, -HOUR).is_empty());
        assert!(plan_breaks(T0, Some(0), None, 1, &p, 0).is_empty());
    }

    #[test]
    fn negative_remaining_counts_as_zero() {
        let p = profile(45, 15);
        assert_eq!(
            plan_breaks(T0, Some(-5_000), None, 1, &p, HOUR)[0],
            window(1, T0, T0 + 15 * MIN)
        );
        assert_eq!(
            plan_breaks(T0, None, Some(-5_000), 1, &p, 2 * HOUR)[0],
            window(2, T0 + 45 * MIN, T0 + 60 * MIN)
        );
    }

    #[test]
    fn zero_length_phases_and_extreme_values_terminate() {
        let zero = TimingProfile {
            focus_secs: 0,
            break_secs: 0,
            ..TimingProfile::normal()
        };
        let breaks = plan_breaks(T0, None, None, 0, &zero, 24 * HOUR);
        assert_eq!(breaks.len(), MAX_PLANNED_BREAKS);
        assert_eq!(breaks[0], window(1, T0 + 1_000, T0 + 2_000));
        assert_eq!(breaks[1], window(2, T0 + 3_000, T0 + 4_000));

        let near_end = plan_breaks(i64::MAX - 10, Some(0), None, u32::MAX, &zero, i64::MAX);
        assert_eq!(near_end.len(), 1);
        assert_eq!(near_end[0].number, u32::MAX);
        assert_eq!(near_end[0].end_ms, i64::MAX);

        let huge = TimingProfile {
            focus_secs: u32::MAX,
            break_secs: u32::MAX,
            ..TimingProfile::normal()
        };
        assert_eq!(
            plan_breaks(i64::MIN, None, Some(i64::MAX), u32::MAX, &huge, i64::MAX).len(),
            0
        );
        assert_eq!(
            plan_breaks(T0, Some(i64::MAX), None, 1, &huge, i64::MAX).len(),
            0,
            "a break at the end of time never starts before the horizon"
        );
    }

    #[test]
    fn meetings_collide_only_when_they_overlap_a_break() {
        let breaks = [
            window(1, T0 + 45 * MIN, T0 + 60 * MIN),
            window(2, T0 + 105 * MIN, T0 + 120 * MIN),
        ];
        let meetings = [
            // 0: ends exactly when break 1 starts.
            meeting("ends-at-start", T0 + 15 * MIN, T0 + 45 * MIN),
            // 1: overlaps the start of break 1.
            meeting("into-break", T0 + 30 * MIN, T0 + 46 * MIN),
            // 2: inside break 1.
            meeting("inside", T0 + 50 * MIN, T0 + 55 * MIN),
            // 3: starts exactly when break 1 ends.
            meeting("starts-at-end", T0 + 60 * MIN, T0 + 90 * MIN),
            // 4: spans both breaks: reported once, with break 1.
            meeting("all-day", T0, T0 + 10 * HOUR),
            // 5: overlaps the end of break 2.
            meeting("late", T0 + 119 * MIN, T0 + 150 * MIN),
            // 6: zero length inside a break.
            meeting("empty", T0 + 50 * MIN, T0 + 50 * MIN),
            // 7: inverted.
            meeting("inverted", T0 + 59 * MIN, T0 + 46 * MIN),
            // 8: covers break 2 exactly.
            meeting("exact", T0 + 105 * MIN, T0 + 120 * MIN),
        ];
        let found = conflicts(&breaks, &meetings);
        assert_eq!(
            found,
            [
                (1, breaks[0]),
                (2, breaks[0]),
                (4, breaks[0]),
                (5, breaks[1]),
                (8, breaks[1]),
            ]
        );
        assert!(conflicts(&[], &meetings).is_empty());
        assert!(conflicts(&breaks, &[]).is_empty());
    }

    #[test]
    fn planned_breaks_and_conflicts_work_together() {
        // A 25/5 session that would start now; a stand-up at +27 min collides with break 1,
        // a meeting during focus 2 does not.
        let breaks = plan_breaks(T0, None, None, 0, &profile(25, 5), 2 * HOUR);
        let meetings = [
            meeting("standup", T0 + 27 * MIN, T0 + 42 * MIN),
            meeting("focus-time", T0 + 31 * MIN, T0 + 54 * MIN),
        ];
        let found = conflicts(&breaks, &meetings);
        assert_eq!(found, [(0, window(1, T0 + 25 * MIN, T0 + 30 * MIN))]);
    }

    #[test]
    fn reminders_follow_the_30_20_10_rule() {
        let start = T0 + 2 * HOUR;
        let early = start - HOUR;
        assert_eq!(
            reminder_schedule(start, early, 0),
            Some((start - 30 * MIN, true))
        );
        assert_eq!(
            reminder_schedule(start, early, 1),
            Some((start - 20 * MIN, true))
        );
        assert_eq!(
            reminder_schedule(start, early, 2),
            Some((start - 10 * MIN, false))
        );
        assert_eq!(reminder_schedule(start, early, 3), None);
        assert_eq!(reminder_schedule(start, early, u32::MAX), None);
    }

    #[test]
    fn owner_flow_remind_later_twice() {
        let start = T0 + 2 * HOUR;
        // "Remind later" on the Focus page an hour before.
        let (due, later) = reminder_schedule(start, start - HOUR, 0).expect("first reminder");
        assert_eq!((due, later), (start - 30 * MIN, true));
        // The popup at -30 min: "remind later" again → 10 minutes later.
        let (due, later) = reminder_schedule(start, due, 1).expect("second reminder");
        assert_eq!((due, later), (start - 20 * MIN, true));
        // The popup at -20 min: "remind later" again → the last popup at -10 min, without "later".
        let (due, later) = reminder_schedule(start, due, 2).expect("last reminder");
        assert_eq!((due, later), (start - 10 * MIN, false));
        assert_eq!(reminder_schedule(start, due, 3), None);
    }

    #[test]
    fn passed_reminder_stages_are_skipped() {
        let start = T0 + 2 * HOUR;
        // Exactly at a stage time it counts as passed (no popup "now").
        assert_eq!(
            reminder_schedule(start, start - 30 * MIN, 0),
            Some((start - 20 * MIN, true))
        );
        assert_eq!(
            reminder_schedule(start, start - 25 * MIN, 0),
            Some((start - 20 * MIN, true))
        );
        assert_eq!(
            reminder_schedule(start, start - 15 * MIN, 0),
            Some((start - 10 * MIN, false))
        );
        // A late "remind later" from the -30 popup (clicked at -21 min) still gets the -20 one.
        assert_eq!(
            reminder_schedule(start, start - 21 * MIN, 1),
            Some((start - 20 * MIN, true))
        );
        // Clicked at -19 min: straight to the last one.
        assert_eq!(
            reminder_schedule(start, start - 19 * MIN, 1),
            Some((start - 10 * MIN, false))
        );
        for now in [start - 10 * MIN, start - 5 * MIN, start, start + HOUR] {
            assert_eq!(reminder_schedule(start, now, 0), None, "{}", start - now);
        }
    }

    #[test]
    fn reminder_schedule_never_overflows() {
        assert_eq!(reminder_schedule(i64::MIN, i64::MIN, 0), None);
        assert_eq!(
            reminder_schedule(i64::MAX, i64::MIN, 0),
            Some((i64::MAX - 30 * MIN, true))
        );
        assert_eq!(reminder_schedule(i64::MIN + 5, i64::MIN, 0), None);
    }

    // -- Smart breaks (ADR-023) ----------------------------------------------------------------------------

    fn smart(id: &str, start_ms: i64, end_ms: i64) -> SmartMeeting {
        SmartMeeting {
            id: id.to_owned(),
            start_ms,
            end_ms,
            eligible: true,
        }
    }

    fn answered(id: &str, start_ms: i64, end_ms: i64) -> SmartMeeting {
        SmartMeeting {
            eligible: false,
            ..smart(id, start_ms, end_ms)
        }
    }

    fn fit(meeting: usize, kind: SmartBreakKind, break_start_ms: i64, delta_ms: i64) -> BreakFit {
        BreakFit {
            meeting,
            kind,
            break_start_ms,
            delta_ms,
        }
    }

    fn idle_first() -> FirstBreak {
        FirstBreak::new(T0, None, None, 0, &profile(45, 15))
    }

    #[test]
    fn without_a_colliding_meeting_nothing_moves() {
        let p = profile(45, 15);
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &[]), None);
        // Before the break, touching it on both sides, and after it.
        let around = [
            smart("before", T0 + 10 * MIN, T0 + 45 * MIN),
            smart("after", T0 + 60 * MIN, T0 + 90 * MIN),
            smart("empty", T0 + 50 * MIN, T0 + 50 * MIN),
            smart("inverted", T0 + 55 * MIN, T0 + 50 * MIN),
        ];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &around), None);

        // With no meetings, the smart plan is the plain plan.
        for (focus_left, break_left, cycle) in [
            (None, None, 0),
            (Some(10 * MIN), None, 3),
            (None, Some(5 * MIN), 2),
            (Some(-5_000), None, 1),
        ] {
            let plain = plan_breaks(T0, focus_left, break_left, cycle, &p, 10 * HOUR);
            let first = FirstBreak::new(T0, focus_left, break_left, cycle, &p);
            let planned = plan_smart_breaks(T0, first, &p, 10 * HOUR, &around[2..]);
            assert_eq!(planned.breaks, plain, "{focus_left:?} {break_left:?}");
            assert!(planned.moves.is_empty());
        }
    }

    #[test]
    fn meeting_in_the_middle_of_the_break() {
        let p = profile(45, 15);
        // Starts 12 min into the break (13:57 for a 13:45 break): the break ends right before it, 3 min earlier.
        let late = [smart("late", T0 + 57 * MIN, T0 + 90 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &late),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 42 * MIN,
                -3 * MIN
            ))
        );
        // Starts 5 min into the break: starting the break with it is the smaller change (+5 min, not −10 min).
        let early = [smart("early", T0 + 50 * MIN, T0 + 80 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &early),
            Some(fit(
                0,
                SmartBreakKind::StartsAtMeeting,
                T0 + 50 * MIN,
                5 * MIN
            ))
        );
        // Exactly in the middle (7.5 min either way): the break before the meeting wins the tie.
        let middle = [smart("middle", T0 + 52 * MIN + 30_000, T0 + 80 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &middle),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 37 * MIN + 30_000,
                -7 * MIN - 30_000
            ))
        );
    }

    #[test]
    fn meeting_in_the_middle_of_the_focus() {
        let p = profile(45, 15);
        // 13:40–14:10 with a 13:45 break: the break starts with the meeting, the block is 5 min shorter.
        let meetings = [smart("standup", T0 + 40 * MIN, T0 + 70 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &meetings),
            Some(fit(
                0,
                SmartBreakKind::StartsAtMeeting,
                T0 + 40 * MIN,
                -5 * MIN
            ))
        );
        // Starting 20 min before the break would leave a 25-minute block (< 27 min, 60 %): the usual question.
        let early = [smart("early", T0 + 25 * MIN, T0 + 50 * MIN)];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &early), None);
        // A meeting already running can never be fitted (the break would start in the past).
        let running = [smart("running", T0 - 5 * MIN, T0 + 50 * MIN)];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &running), None);
    }

    #[test]
    fn bounds_limit_the_change() {
        let p = profile(45, 15);
        // Lengthening: at most +25 % (45 min → 56 min 15 s). A focus with 1 min left cannot end its break before a
        // meeting 12 min away (it would start in the past), so only starting the break with it remains: +11 min fits,
        // +12 min does not.
        let ok = [smart("ok", T0 + 12 * MIN, T0 + 40 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + MIN, &p, &ok),
            Some(fit(
                0,
                SmartBreakKind::StartsAtMeeting,
                T0 + 12 * MIN,
                11 * MIN
            ))
        );
        let too_far = [smart("far", T0 + 13 * MIN, T0 + 40 * MIN)];
        assert_eq!(fit_break(T0, T0 + MIN, &p, &too_far), None);

        // Shortening: down to 27 min (60 %). A short call followed by a meeting rules out starting the break with
        // the call; ending it before the call leaves exactly 27 min: allowed, one minute earlier is not.
        let at_limit = [
            smart("call", T0 + 42 * MIN, T0 + 46 * MIN),
            smart("next", T0 + 46 * MIN, T0 + 80 * MIN),
        ];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &at_limit),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 27 * MIN,
                -18 * MIN
            ))
        );
        let below = [
            smart("call", T0 + 41 * MIN, T0 + 46 * MIN),
            smart("next", T0 + 46 * MIN, T0 + 80 * MIN),
        ];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &below), None);

        // A 5-minute focus can never be shortened (10-minute floor), only lengthened, by up to 1 min 15 s.
        let tiny = profile(5, 5);
        let soon = [smart("soon", T0 + 6 * MIN, T0 + 12 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + 5 * MIN, &tiny, &soon),
            Some(fit(0, SmartBreakKind::StartsAtMeeting, T0 + 6 * MIN, MIN))
        );
        let later = [smart("later", T0 + 7 * MIN, T0 + 12 * MIN)];
        assert_eq!(fit_break(T0, T0 + 5 * MIN, &tiny, &later), None);

        // A 15-minute focus: 60 % is 9 min, but never under 10 min (the call rules out starting with it again).
        let quarter = profile(15, 5);
        let nine = [
            smart("call", T0 + 14 * MIN + 30_000, T0 + 16 * MIN),
            smart("next", T0 + 16 * MIN, T0 + 30 * MIN),
        ];
        assert_eq!(
            fit_break(T0, T0 + 15 * MIN, &quarter, &nine),
            None,
            "9.5-min block"
        );
        let ten = [
            smart("call", T0 + 15 * MIN, T0 + 16 * MIN),
            smart("next", T0 + 16 * MIN, T0 + 30 * MIN),
        ];
        assert_eq!(
            fit_break(T0, T0 + 15 * MIN, &quarter, &ten),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 10 * MIN,
                -5 * MIN
            ))
        );
    }

    #[test]
    fn a_meeting_longer_than_the_next_block_is_asked_about() {
        let p = profile(45, 15);
        // 13:57–14:40 (T0 = 13:00): after a break ending at 13:57 the next block runs until 14:42 and absorbs it.
        let fits = [smart("fits", T0 + 57 * MIN, T0 + 100 * MIN)];
        assert!(fit_break(T0, T0 + 45 * MIN, &p, &fits).is_some());
        // 13:57–14:43: it would still run when the next break starts (13:57 + 45 min = 14:42): the question decides.
        let long = [smart("long", T0 + 57 * MIN, T0 + 103 * MIN)];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &long), None);
        // Starting the break with a meeting absorbs one up to break + focus (60 min) long.
        let hour = [smart("hour", T0 + 50 * MIN, T0 + 110 * MIN)];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &hour),
            Some(fit(
                0,
                SmartBreakKind::StartsAtMeeting,
                T0 + 50 * MIN,
                5 * MIN
            ))
        );
        let longer = [smart("longer", T0 + 50 * MIN, T0 + 111 * MIN)];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &longer), None);
    }

    #[test]
    fn back_to_back_meetings() {
        let p = profile(45, 15);
        // A short call, then a long meeting right after it: starting the break with the call would run into the
        // second meeting, so the break ends before the call.
        let pair = [
            smart("call", T0 + 50 * MIN, T0 + 55 * MIN),
            smart("review", T0 + 55 * MIN, T0 + 90 * MIN),
        ];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &pair),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 35 * MIN,
                -10 * MIN
            ))
        );
        // Two meetings from 13:40: before them the block would be too short, with the first one the break would run
        // into the second: no fit, both get the usual question.
        let chain = [
            smart("one", T0 + 40 * MIN, T0 + 50 * MIN),
            smart("two", T0 + 50 * MIN, T0 + 80 * MIN),
        ];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &chain), None);
        // A moved break never lands on a meeting the user answered, even one that does not collide with the plain
        // break: ending before 13:57 would overlap the answered 13:35–13:43, starting with it is +12 min (> 25 %).
        let late = smart("late", T0 + 57 * MIN, T0 + 90 * MIN);
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, std::slice::from_ref(&late)),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 42 * MIN,
                -3 * MIN
            ))
        );
        let guarded = [answered("answered", T0 + 35 * MIN, T0 + 43 * MIN), late];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &guarded), None);
        // With room to start the break with it instead (13:52, +7 min), that is what happens.
        let guarded_early = [
            answered("answered", T0 + 30 * MIN, T0 + 38 * MIN),
            smart("early", T0 + 52 * MIN, T0 + 90 * MIN),
        ];
        assert_eq!(
            fit_break(T0, T0 + 45 * MIN, &p, &guarded_early),
            Some(fit(
                1,
                SmartBreakKind::StartsAtMeeting,
                T0 + 52 * MIN,
                7 * MIN
            ))
        );

        // In a plan, each meeting moves its own break and the later breaks move along.
        let meetings = [
            smart("first", T0 + 57 * MIN, T0 + 70 * MIN),
            smart("second", T0 + 115 * MIN, T0 + 130 * MIN),
        ];
        let planned = plan_smart_breaks(T0, idle_first(), &p, 3 * HOUR, &meetings);
        assert_eq!(
            planned.breaks,
            [
                window(1, T0 + 42 * MIN, T0 + 57 * MIN),
                // Nominally at 57 + 45 = 102 min; the second meeting at 115 min is 13 min in: ends before it.
                window(2, T0 + 100 * MIN, T0 + 115 * MIN),
                window(3, T0 + 160 * MIN, T0 + 175 * MIN),
            ]
        );
        assert_eq!(
            planned.moves,
            [
                (
                    planned.breaks[0],
                    fit(
                        0,
                        SmartBreakKind::EndsBeforeMeeting,
                        T0 + 42 * MIN,
                        -3 * MIN
                    )
                ),
                (
                    planned.breaks[1],
                    fit(
                        1,
                        SmartBreakKind::EndsBeforeMeeting,
                        T0 + 100 * MIN,
                        -2 * MIN
                    )
                ),
            ]
        );
    }

    #[test]
    fn answered_long_and_all_day_meetings_are_left_alone() {
        let p = profile(45, 15);
        // Answered (or "Ask me instead"): the question decides.
        assert_eq!(
            fit_break(
                T0,
                T0 + 45 * MIN,
                &p,
                &[answered("a", T0 + 57 * MIN, T0 + 90 * MIN)]
            ),
            None
        );
        // An answered meeting next to an eligible one in the same break: the user decides for that break.
        let mixed = [
            smart("open", T0 + 57 * MIN, T0 + 90 * MIN),
            answered("answered", T0 + 50 * MIN, T0 + 52 * MIN),
        ];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &mixed), None);
        // A day-long block (an all-day event that slipped through as timed): never fitted, also when it starts
        // later today.
        let all_day = [smart("all-day", T0 - 10 * HOUR, T0 + 14 * HOUR)];
        assert_eq!(fit_break(T0, T0 + 45 * MIN, &p, &all_day), None);
        let afternoon = [smart(
            "afternoon",
            T0 + 50 * MIN,
            T0 + 50 * MIN + 4 * HOUR + 1,
        )];
        let long_focus = profile(240, 15);
        assert_eq!(
            fit_break(T0, T0 + 4 * HOUR - 5 * MIN, &long_focus, &afternoon),
            None
        );
        // Exactly 4 hours is still a meeting (with a focus long enough to absorb it).
        let workshop = [smart("workshop", T0 + 4 * HOUR, T0 + 8 * HOUR)];
        assert!(fit_break(T0, T0 + 4 * HOUR - 5 * MIN, &long_focus, &workshop).is_some());
        // The plan keeps such a break where it is (the conflict asks as before).
        let planned = plan_smart_breaks(T0, idle_first(), &p, 2 * HOUR, &all_day);
        assert_eq!(planned.breaks, plan_breaks(T0, None, None, 0, &p, 2 * HOUR));
        assert!(planned.moves.is_empty());
    }

    #[test]
    fn running_focus_and_running_break() {
        let p = profile(45, 15);
        // Focus with 3 min left, meeting in 10 min: ending the break before it would start it in the past, so the
        // break starts with the meeting (block +7 min).
        let soon = [smart("soon", T0 + 10 * MIN, T0 + 40 * MIN)];
        let first = FirstBreak::new(T0, Some(3 * MIN), None, 2, &p);
        assert_eq!(
            first,
            FirstBreak {
                start_ms: T0 + 3 * MIN,
                number: 2,
                fixed: false
            }
        );
        let planned = plan_smart_breaks(T0, first, &p, HOUR, &soon);
        assert_eq!(planned.breaks, [window(2, T0 + 10 * MIN, T0 + 25 * MIN)]);
        assert_eq!(
            planned.moves[0].1,
            fit(0, SmartBreakKind::StartsAtMeeting, T0 + 10 * MIN, 7 * MIN)
        );
        // A block already lengthened past its nominal end (nominal remaining −2 min) keeps its fit.
        let past = FirstBreak::new(T0, Some(-2 * MIN), None, 2, &p);
        let soon_after = [smart("soon", T0 + 3 * MIN, T0 + 40 * MIN)];
        assert_eq!(
            plan_smart_breaks(T0, past, &p, HOUR, &soon_after).moves[0].1,
            fit(0, SmartBreakKind::StartsAtMeeting, T0 + 3 * MIN, 5 * MIN)
        );
        // A fixed first block (extended by a meeting hold, set on the Developer page) is not fitted; a past nominal
        // end means "now", as in `plan_breaks`; the next block is fitted again.
        let fixed = FirstBreak {
            fixed: true,
            ..FirstBreak::new(T0, Some(-2 * MIN), None, 2, &p)
        };
        let later = [
            smart("soon", T0 + 3 * MIN, T0 + 10 * MIN),
            smart("next", T0 + 72 * MIN, T0 + 90 * MIN),
        ];
        let planned = plan_smart_breaks(T0, fixed, &p, 2 * HOUR, &later);
        assert_eq!(planned.breaks[0], window(2, T0, T0 + 15 * MIN));
        assert_eq!(planned.moves.len(), 1);
        assert_eq!(planned.moves[0].0.number, 3);
        assert_eq!(planned.moves[0].1.kind, SmartBreakKind::EndsBeforeMeeting);

        // In a break with 5 min left: the next block (break 3, nominally at +50 min) is fitted.
        let first = FirstBreak::new(T0, None, Some(5 * MIN), 2, &p);
        assert_eq!((first.start_ms, first.number), (T0 + 50 * MIN, 3));
        let meeting = [smart("m", T0 + 62 * MIN, T0 + 90 * MIN)];
        let planned = plan_smart_breaks(T0, first, &p, 2 * HOUR, &meeting);
        assert_eq!(planned.breaks[0], window(3, T0 + 47 * MIN, T0 + 62 * MIN));
    }

    #[test]
    fn daylight_saving_day_uses_absolute_time() {
        // 2026-10-25, Poland: 03:00 CEST becomes 02:00 CET (01:00 UTC). A session started at 02:20 CEST (00:20 UTC):
        // the plain break is at 01:05 UTC (02:05 CET); a meeting at 02:15 CET (01:15 UTC) moves it 5 min earlier.
        // Local wall hours repeat that night, the plan does not care: a 40-minute block, not "−65 minutes".
        const DST_START: i64 = 1_792_887_600_000; // 2026-10-25T00:20:00Z
        let p = profile(45, 15);
        let meetings = [smart("night", DST_START + 55 * MIN, DST_START + 85 * MIN)];
        let first = FirstBreak::new(DST_START, None, None, 0, &p);
        let planned = plan_smart_breaks(DST_START, first, &p, 3 * HOUR, &meetings);
        assert_eq!(
            planned.moves[0].1,
            fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                DST_START + 40 * MIN,
                -5 * MIN
            )
        );
        assert_eq!(planned.breaks[1].start_ms, DST_START + 100 * MIN);
    }

    #[test]
    fn demo_profile_fits_in_seconds() {
        // 40 s focus, 20 s break: 24 s .. 50 s blocks. A meeting 10 s into the break: −10 s and +10 s tie, so the
        // break ends before it.
        let demo = TimingProfile::demo();
        let meetings = [smart("demo", T0 + 50_000, T0 + 80_000)];
        assert_eq!(
            fit_break(T0, T0 + 40_000, &demo, &meetings),
            Some(fit(
                0,
                SmartBreakKind::EndsBeforeMeeting,
                T0 + 30_000,
                -10_000
            ))
        );
        // 17 s before the plain break: a 23-second block is too short, and starting with it as well.
        let early = [smart("early", T0 + 23_000, T0 + 70_000)];
        assert_eq!(fit_break(T0, T0 + 40_000, &demo, &early), None);
        let first = FirstBreak::new(T0, None, None, 0, &demo);
        let planned = plan_smart_breaks(T0, first, &demo, 2 * MIN, &meetings);
        assert_eq!(planned.breaks[0], window(1, T0 + 30_000, T0 + 50_000));
        assert_eq!(planned.breaks[1], window(2, T0 + 90_000, T0 + 110_000));
    }

    #[test]
    fn smart_break_view_describes_the_move() {
        let meeting = smart("ms-1", T0 + 60 * MIN, T0 + 90 * MIN);
        let w = window(2, T0 + 45 * MIN, T0 + 60 * MIN);
        let view = smart_break_view(
            &w,
            &fit(0, SmartBreakKind::EndsBeforeMeeting, w.start_ms, -400_499),
            &meeting,
        );
        assert_eq!(
            view,
            SmartBreak {
                meeting_id: "ms-1".into(),
                meeting_start_ms: T0 + 60 * MIN,
                meeting_end_ms: T0 + 90 * MIN,
                break_number: 2,
                kind: SmartBreakKind::EndsBeforeMeeting,
                focus_delta_secs: -400,
                break_start_ms: T0 + 45 * MIN,
                break_end_ms: T0 + 60 * MIN,
            }
        );
        let up = smart_break_view(
            &w,
            &fit(0, SmartBreakKind::StartsAtMeeting, 0, 1_500),
            &meeting,
        );
        assert_eq!(up.focus_delta_secs, 2);
        let huge = smart_break_view(
            &w,
            &fit(0, SmartBreakKind::StartsAtMeeting, 0, i64::MIN),
            &meeting,
        );
        assert_eq!(huge.focus_delta_secs, i32::MIN);
    }

    #[test]
    fn smart_planning_with_extreme_values_never_panics() {
        let zero = TimingProfile {
            focus_secs: 0,
            break_secs: 0,
            ..TimingProfile::normal()
        };
        let huge = TimingProfile {
            focus_secs: u32::MAX,
            break_secs: u32::MAX,
            ..TimingProfile::normal()
        };
        let meetings = [
            smart("min", i64::MIN, i64::MIN + 1),
            smart("max", i64::MAX - 1, i64::MAX),
            smart("all", i64::MIN, i64::MAX),
        ];
        for p in [zero, huge, TimingProfile::demo()] {
            for now in [i64::MIN, 0, T0, i64::MAX] {
                for start in [i64::MIN, 0, T0, i64::MAX] {
                    let _ = fit_break(now, start, &p, &meetings);
                    let first = FirstBreak {
                        start_ms: start,
                        number: u32::MAX,
                        fixed: false,
                    };
                    let planned = plan_smart_breaks(now, first, &p, i64::MAX, &meetings);
                    assert!(planned.breaks.len() <= MAX_PLANNED_BREAKS);
                }
            }
        }
        let first = FirstBreak::new(i64::MAX, Some(i64::MAX), None, u32::MAX, &huge);
        assert_eq!(first.start_ms, i64::MAX);
    }

    #[test]
    fn only_meetings_up_to_4_hours_may_hold_the_session() {
        assert!(meeting_hold_allowed(T0, T0 + 1));
        assert!(meeting_hold_allowed(T0, T0 + HOUR));
        assert!(meeting_hold_allowed(T0, T0 + 4 * HOUR), "exactly 4 h");
        assert!(!meeting_hold_allowed(T0, T0 + 4 * HOUR + 1));
        assert!(!meeting_hold_allowed(T0, T0 + 9 * HOUR), "a working day");
        assert!(!meeting_hold_allowed(T0, T0), "no length");
        assert!(!meeting_hold_allowed(T0, T0 - HOUR), "inverted");
        assert!(!meeting_hold_allowed(i64::MIN, i64::MAX));
        assert!(meeting_hold_allowed(i64::MAX - HOUR, i64::MAX));
        assert!(meeting_hold_allowed(i64::MIN, i64::MIN + HOUR));
    }
}
