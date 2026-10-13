//! Developer mode (ADR-019): what the developer page's buttons show (`dev_trigger`).
//!
//! The samples are built here, without side effects, from the session snapshot (cycle and phase lengths of the
//! current `TimingProfile`), the config and the clocks: a phase popup with a sample meeting, reminder, launchers
//! or work count, or the Windows toast of a phase change. The command shows them through the usual popup thread
//! and shields worker. Nothing is persisted and nothing leaves the computer: a sample meeting exists only in its
//! popup, and its id starts with [`SAMPLE_ID_PREFIX`], so answering it changes nothing ([`is_sample_meeting`]).

use focus_core::api::{
    AppError, CalendarSource, Config, GameLauncher, Meeting, MeetingChoice, MeetingConflict, Phase,
    PopupKind, SessionSnapshot,
};
use focus_core::config::reminder_for_break;
use focus_core::timing::MEETING_REMINDER_LEAD_MINUTES;

use crate::popup::PopupRequest;
use crate::texts::{self, Toast};

/// Ids of sample meetings (also the developer page's own samples on the Focus page). Real ids start with their
/// calendar source, so they never match.
pub const SAMPLE_ID_PREFIX: &str = "dev-sample-";
/// Id of the meeting in sample popups: a newer sample reminder replaces the older one instead of parking it.
const SAMPLE_MEETING_ID: &str = "dev-sample-meeting";
/// `AppError.code` of `toast { phase: "idle" }`: only focus and break changes show a toast.
pub const ERR_INVALID_PHASE: &str = "dev.invalidPhase";
/// Length of the sample meeting.
const SAMPLE_MEETING_MINUTES: i64 = 30;
/// The meeting of a held-break sample started this long ago (it runs now).
const HELD_MEETING_STARTED_MINUTES_AGO: i64 = 10;
/// `leadMinutes: null` = the first reminder (30 minutes before the meeting).
const DEFAULT_LEAD_MINUTES: u32 = MEETING_REMINDER_LEAD_MINUTES[0];
/// The last reminder (10 minutes before) offers no "remind later".
const LAST_LEAD_MINUTES: u32 =
    MEETING_REMINDER_LEAD_MINUTES[MEETING_REMINDER_LEAD_MINUTES.len() - 1];
/// Launchers of the "launchers closed" sample.
const SAMPLE_LAUNCHERS: [GameLauncher; 2] = [GameLauncher::Steam, GameLauncher::Epic];
/// New work items of the work digest sample.
const SAMPLE_WORK_COUNT: u32 = 3;
const MS_PER_SEC: i64 = 1000;
const MS_PER_MINUTE: i64 = 60 * MS_PER_SEC;

/// `id` belongs to a sample meeting: `set_meeting_choice` accepts it without storing anything.
pub fn is_sample_meeting(id: &str) -> bool {
    id.starts_with(SAMPLE_ID_PREFIX)
}

/// What the samples are built from.
pub struct Samples<'a> {
    /// The session now: cycle, phase, focus and break length of the current profile (the idle profile, demo
    /// included, while no session runs).
    pub snapshot: &'a SessionSnapshot,
    /// The user's break reminders.
    pub config: &'a Config,
    /// Wall clock, Unix milliseconds.
    pub now_ms: i64,
    /// Local time of day (`focus_platform::clock::local_minutes_of_day`) for the `HH:MM` of a fallback toast.
    pub local_minutes: Option<f64>,
}

impl Samples<'_> {
    /// A sample popup of `kind` (`PopupRequest::sample`; `sticky` = stays until closed). `lead_minutes` is how
    /// long before the sample meeting a `MeetingReminder` comes (`None` = 30); the last reminder (10) offers no
    /// "remind later".
    pub fn popup(&self, kind: PopupKind, lead_minutes: Option<u32>, sticky: bool) -> PopupRequest {
        let cycle = self.cycle(kind == PopupKind::FocusStarted);
        let (focus_secs, break_secs) = (self.snapshot.focus_secs, self.snapshot.break_secs);
        let sample = |phase_secs, fallback| PopupRequest {
            sample: true,
            sticky,
            ..PopupRequest::new(kind, cycle, phase_secs, fallback)
        };
        match kind {
            PopupKind::BreakStarted => {
                let reminder = self.reminder(cycle);
                PopupRequest {
                    reminder: Some(reminder.clone()),
                    ..sample(break_secs, texts::break_toast(break_secs, Some(&reminder)))
                }
            }
            PopupKind::FocusStarted => sample(focus_secs, texts::focus_toast(focus_secs, cycle)),
            PopupKind::Interrupted => sample(0, texts::interrupted_toast()),
            PopupKind::LaunchersClosed => PopupRequest {
                launchers: SAMPLE_LAUNCHERS.to_vec(),
                ..sample(focus_secs, texts::launchers_closed_toast(&SAMPLE_LAUNCHERS))
            },
            PopupKind::MeetingReminder => {
                let lead = lead_minutes.unwrap_or(DEFAULT_LEAD_MINUTES);
                let conflict = self.upcoming_meeting(cycle, lead);
                let meeting = &conflict.meeting;
                let start_local =
                    texts::local_clock(meeting.start_ms, self.now_ms, self.local_minutes);
                let fallback = texts::meeting_reminder_toast(
                    &meeting.subject,
                    start_local.as_deref(),
                    i64::from(lead),
                    cycle,
                );
                PopupRequest {
                    meeting: Some(conflict),
                    ..sample(break_secs, fallback)
                }
            }
            PopupKind::BreakHeld => {
                let reminder = self.reminder(cycle);
                let conflict = self.running_meeting(cycle);
                let fallback = texts::break_held_toast(
                    Some(&conflict.meeting.subject),
                    break_secs,
                    Some(&reminder),
                );
                PopupRequest {
                    reminder: Some(reminder),
                    meeting: Some(conflict),
                    ..sample(break_secs, fallback)
                }
            }
            PopupKind::WorkDigest => PopupRequest {
                work_count: SAMPLE_WORK_COUNT,
                ..sample(0, texts::work_digest_toast(SAMPLE_WORK_COUNT))
            },
        }
    }

    /// The Windows toast a real change to `phase` shows (the user's reminder for that break included).
    pub fn toast(&self, phase: Phase) -> Result<Toast, AppError> {
        match phase {
            Phase::Focus => Ok(texts::focus_toast(
                self.snapshot.focus_secs,
                self.cycle(true),
            )),
            Phase::Break => {
                let break_number = self.cycle(false);
                let reminder = reminder_for_break(self.config, break_number);
                Ok(texts::break_toast(
                    self.snapshot.break_secs,
                    reminder.as_deref(),
                ))
            }
            Phase::Idle => Err(AppError {
                code: ERR_INVALID_PHASE.to_owned(),
                message: "only a change to focus or break shows a toast".to_owned(),
                field: Some("phase".to_owned()),
            }),
        }
    }

    /// The cycle a sample belongs to: the current one (1 while idle), or the next focus cycle for a focus start
    /// during a break.
    fn cycle(&self, next_focus: bool) -> u32 {
        let current = self.snapshot.cycle.max(1);
        if next_focus && self.snapshot.phase == Phase::Break {
            current.saturating_add(1)
        } else {
            current
        }
    }

    /// The user's own reminder for that break, else the sample one.
    fn reminder(&self, break_number: u32) -> String {
        reminder_for_break(self.config, break_number)
            .unwrap_or_else(|| texts::current().dev_sample_reminder.to_owned())
    }

    fn meeting(&self, start_ms: i64) -> Meeting {
        Meeting {
            id: SAMPLE_MEETING_ID.to_owned(),
            source: CalendarSource::Microsoft,
            subject: texts::current().dev_sample_meeting.to_owned(),
            start_ms,
            end_ms: start_ms.saturating_add(SAMPLE_MEETING_MINUTES * MS_PER_MINUTE),
            online: true,
        }
    }

    /// A meeting `lead` minutes from now that collides with break `break_number` (starting with the meeting).
    fn upcoming_meeting(&self, break_number: u32, lead: u32) -> MeetingConflict {
        let start_ms = self
            .now_ms
            .saturating_add(i64::from(lead).saturating_mul(MS_PER_MINUTE));
        MeetingConflict {
            meeting: self.meeting(start_ms),
            break_number,
            break_start_ms: start_ms,
            break_end_ms: start_ms.saturating_add(self.break_ms()),
            choice: Some(MeetingChoice::RemindLater),
            next_reminder_ms: None,
            remind_later_allowed: lead > LAST_LEAD_MINUTES,
            too_long: false,
        }
    }

    /// A meeting running now that holds break `break_number`, which starts now.
    fn running_meeting(&self, break_number: u32) -> MeetingConflict {
        let start_ms = self
            .now_ms
            .saturating_sub(HELD_MEETING_STARTED_MINUTES_AGO * MS_PER_MINUTE);
        MeetingConflict {
            meeting: self.meeting(start_ms),
            break_number,
            break_start_ms: self.now_ms,
            break_end_ms: self.now_ms.saturating_add(self.break_ms()),
            choice: Some(MeetingChoice::PauseAtBreak),
            next_reminder_ms: None,
            remind_later_allowed: false,
            too_long: false,
        }
    }

    fn break_ms(&self) -> i64 {
        i64::from(self.snapshot.break_secs).saturating_mul(MS_PER_SEC)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-13 10:00 UTC.
    const NOW: i64 = 1_791_885_600_000;

    fn snapshot(phase: Phase, cycle: u32) -> SessionSnapshot {
        SessionSnapshot {
            revision: 0,
            session_id: None,
            phase,
            paused: false,
            cycle,
            remaining_secs: 0,
            phase_total_secs: 0,
            focus_secs: 2700,
            break_secs: 900,
            focused_secs: 0,
            demo: false,
            music_muted: false,
            meeting_hold: None,
            goal: None,
        }
    }

    fn samples<'a>(snapshot: &'a SessionSnapshot, config: &'a Config) -> Samples<'a> {
        Samples {
            snapshot,
            config,
            now_ms: NOW,
            local_minutes: Some(12.0 * 60.0),
        }
    }

    #[test]
    fn sample_meetings_are_recognised_by_their_id() {
        assert!(is_sample_meeting(SAMPLE_MEETING_ID));
        assert!(is_sample_meeting("dev-sample-42"));
        assert!(!is_sample_meeting("ics:1:0"));
        assert!(!is_sample_meeting("ms:dev-sample-1"));
        assert!(!is_sample_meeting(""));
        assert_eq!((DEFAULT_LEAD_MINUTES, LAST_LEAD_MINUTES), (30, 10));
    }

    #[test]
    fn every_popup_kind_has_a_sample_with_the_current_profile() {
        let config = Config::default();
        let snap = snapshot(Phase::Focus, 2);
        let samples = samples(&snap, &config);
        for (kind, phase_secs) in [
            (PopupKind::BreakStarted, 900),
            (PopupKind::FocusStarted, 2700),
            (PopupKind::Interrupted, 0),
            (PopupKind::LaunchersClosed, 2700),
            (PopupKind::MeetingReminder, 900),
            (PopupKind::BreakHeld, 900),
            (PopupKind::WorkDigest, 0),
        ] {
            for sticky in [false, true] {
                let popup = samples.popup(kind, None, sticky);
                assert!(popup.sample, "{kind:?}");
                assert_eq!(popup.sticky, sticky, "{kind:?}");
                assert_eq!(
                    (popup.kind, popup.cycle, popup.phase_secs),
                    (kind, 2, phase_secs)
                );
                assert!(!popup.fallback.title.is_empty() && !popup.fallback.body.is_empty());
            }
        }

        let launchers = samples.popup(PopupKind::LaunchersClosed, None, false);
        assert_eq!(
            launchers.launchers,
            [GameLauncher::Steam, GameLauncher::Epic]
        );
        assert_eq!(
            launchers.fallback,
            texts::launchers_closed_toast(&SAMPLE_LAUNCHERS)
        );
        let digest = samples.popup(PopupKind::WorkDigest, None, false);
        assert_eq!(digest.work_count, 3);
        assert_eq!(digest.meeting, None);
        let focus = samples.popup(PopupKind::FocusStarted, None, false);
        assert_eq!(focus.fallback, texts::focus_toast(2700, 2));
        assert_eq!(focus.reminder, None);
    }

    #[test]
    fn break_samples_carry_the_users_reminder_or_a_sample_one() {
        let snap = snapshot(Phase::Focus, 2);
        let config = Config::default();
        let popup = samples(&snap, &config).popup(PopupKind::BreakStarted, None, false);
        let sample_reminder = texts::current().dev_sample_reminder;
        assert_eq!(popup.reminder.as_deref(), Some(sample_reminder));
        assert_eq!(
            popup.fallback,
            texts::break_toast(900, Some(sample_reminder))
        );

        let config = Config {
            break_reminders: vec!["stretch".into(), "lunch".into()],
            ..Config::default()
        };
        let samples = samples(&snap, &config);
        let popup = samples.popup(PopupKind::BreakStarted, None, false);
        assert_eq!(popup.reminder.as_deref(), Some("lunch"));
        let held = samples.popup(PopupKind::BreakHeld, None, false);
        assert_eq!(held.reminder.as_deref(), Some("lunch"));
    }

    #[test]
    fn meeting_reminder_sample_follows_the_lead_time() {
        let config = Config::default();
        let snap = snapshot(Phase::Focus, 1);
        let samples = samples(&snap, &config);
        for (lead, minutes, later) in [
            (None, 30, true),
            (Some(30), 30, true),
            (Some(20), 20, true),
            (Some(10), 10, false),
        ] {
            let popup = samples.popup(PopupKind::MeetingReminder, lead, true);
            let conflict = popup.meeting.expect("a sample meeting");
            assert!(is_sample_meeting(&conflict.meeting.id));
            assert_eq!(
                conflict.meeting.start_ms,
                NOW + minutes * 60_000,
                "{lead:?}"
            );
            assert_eq!(
                conflict.meeting.end_ms - conflict.meeting.start_ms,
                SAMPLE_MEETING_MINUTES * 60_000
            );
            assert_eq!(conflict.remind_later_allowed, later, "{lead:?}");
            assert_eq!(
                conflict.meeting.subject,
                texts::current().dev_sample_meeting
            );
            assert_eq!(conflict.break_number, 1);
            assert!(conflict.break_start_ms <= conflict.meeting.start_ms);
            assert!(conflict.break_end_ms > conflict.meeting.start_ms);
            assert!(!conflict.too_long);
            assert_eq!(
                popup.fallback,
                texts::meeting_reminder_toast(
                    texts::current().dev_sample_meeting,
                    texts::local_clock(conflict.meeting.start_ms, NOW, Some(720.0)).as_deref(),
                    minutes,
                    1
                )
            );
        }
        // Any other lead still works: "remind later" only before the last reminder.
        let close = samples.popup(PopupKind::MeetingReminder, Some(5), false);
        assert_eq!(close.meeting.map(|c| c.remind_later_allowed), Some(false));
        let far = samples.popup(PopupKind::MeetingReminder, Some(u32::MAX), false);
        assert_eq!(far.meeting.map(|c| c.remind_later_allowed), Some(true));
    }

    #[test]
    fn held_break_sample_has_a_meeting_running_now() {
        let config = Config::default();
        let snap = snapshot(Phase::Break, 3);
        let popup = samples(&snap, &config).popup(PopupKind::BreakHeld, Some(10), false);
        let conflict = popup.meeting.expect("a sample meeting");
        assert!(is_sample_meeting(&conflict.meeting.id));
        assert!(conflict.meeting.start_ms < NOW && NOW < conflict.meeting.end_ms);
        assert_eq!(
            (conflict.break_start_ms, conflict.break_end_ms),
            (NOW, NOW + 900_000)
        );
        assert_eq!(conflict.choice, Some(MeetingChoice::PauseAtBreak));
        assert_eq!(popup.cycle, 3);
    }

    #[test]
    fn sample_cycles_follow_the_session() {
        let config = Config::default();
        // Idle: cycle 1 of the idle profile.
        let idle = SessionSnapshot {
            focus_secs: 40,
            break_secs: 20,
            demo: true,
            ..snapshot(Phase::Idle, 0)
        };
        let popup = samples(&idle, &config).popup(PopupKind::BreakStarted, None, false);
        assert_eq!((popup.cycle, popup.phase_secs), (1, 20));
        // During break 2 the next focus is cycle 3.
        let on_break = snapshot(Phase::Break, 2);
        let samples = samples(&on_break, &config);
        assert_eq!(samples.popup(PopupKind::FocusStarted, None, false).cycle, 3);
        assert_eq!(samples.popup(PopupKind::BreakStarted, None, false).cycle, 2);
    }

    #[test]
    fn toasts_are_the_real_phase_toasts() {
        let config = Config {
            break_reminders: vec!["water".into()],
            ..Config::default()
        };
        let snap = snapshot(Phase::Focus, 1);
        let samples = samples(&snap, &config);
        assert_eq!(
            samples.toast(Phase::Break),
            Ok(texts::break_toast(900, Some("water")))
        );
        assert_eq!(samples.toast(Phase::Focus), Ok(texts::focus_toast(2700, 1)));
        let error = samples.toast(Phase::Idle).expect_err("idle has no toast");
        assert_eq!(error.code, ERR_INVALID_PHASE);
        assert_eq!(error.field.as_deref(), Some("phase"));
    }
}
