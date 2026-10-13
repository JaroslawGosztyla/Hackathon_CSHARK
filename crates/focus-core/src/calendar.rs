//! Calendar meetings against the planned breaks (pure).
//!
//! The shell fetches meetings (Microsoft Graph, Google Calendar, ICS links) and asks here which
//! breaks of the running session (or of a session that would start now) they collide with, and when
//! the next "remind later" popup for a meeting is due. All times are wall clock, Unix epoch
//! milliseconds, passed in by the caller; nothing here reads a clock.

use crate::api::Meeting;
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
