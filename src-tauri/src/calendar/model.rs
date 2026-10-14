//! Pure calendar logic of the shell: merging the sources, the user's answer per meeting ("choices"),
//! "remind later" scheduling, smart breaks (ADR-023) and the parts of `CalendarStatus`. No IO and no clock: the
//! wall clock (`now_ms`, Unix milliseconds) and the session plan come in as arguments.

use std::collections::HashMap;

use focus_core::api::{
    AppError, CalendarSource, CalendarSourceStatus, IntegrationState, Meeting, MeetingChoice,
    MeetingConflict, Phase, SmartBreak,
};
use focus_core::calendar::{
    BreakWindow, FirstBreak, SmartMeeting, SmartPlan, conflicts, meeting_hold_allowed,
    plan_smart_breaks, reminder_schedule, smart_break_view,
};
use focus_core::timing::TimingProfile;
use serde::{Deserialize, Serialize};

/// At most this many meetings are kept (the earliest ones).
pub const MAX_MEETINGS: usize = 200;
/// At most this many ICS links.
pub const MAX_ICS_URLS: usize = 10;
/// Longest ICS link accepted.
pub const MAX_ICS_URL_LEN: usize = 2048;
/// Meetings from different sources with the same subject starting this close are one meeting.
pub const SAME_MEETING_WITHIN_MS: i64 = 60_000;
/// Version of `meeting-choices.json`.
pub const CHOICES_VERSION: u32 = 1;
/// Longest error detail shown in the settings.
const MAX_DETAIL_CHARS: usize = 300;

const HTTPS: &str = "https://";
/// Calendar subscription schemes: the same feed over HTTPS (the platform fetches them as `https://`).
const WEBCAL: &str = "webcal://";
const WEBCALS: &str = "webcals://";

// ---------------------------------------------------------------------------------------------------
// Session plan
// ---------------------------------------------------------------------------------------------------

/// The session as the calendar sees it; the actor sends a new one whenever its timing changes
/// (phase, pause, cycle, profile, a meeting extension, a smart-break fit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPlan {
    pub phase: Phase,
    pub paused: bool,
    /// Cycle of the running session (0 when idle).
    pub cycle: u32,
    /// Exact time left in the phase when the plan was taken (0 when idle); a smart-break fit included.
    pub remaining_ms: u64,
    /// Profile of the running session, or the one a session started now would use.
    pub profile: TimingProfile,
    /// Smart breaks (ADR-023): the session's own moved break (`SessionSnapshot.smart_break`): in Focus the break
    /// after the running block, in a break the running one. The calendar never fits a running block itself.
    pub smart: Option<SmartBreak>,
}

impl SessionPlan {
    /// No session: breaks of a session that would start now with `profile`.
    pub fn idle(profile: TimingProfile) -> Self {
        Self {
            phase: Phase::Idle,
            paused: false,
            cycle: 0,
            remaining_ms: 0,
            profile,
            smart: None,
        }
    }

    /// `true` while no session runs (the breaks are those of a session that would start now).
    pub fn planned(&self) -> bool {
        self.phase == Phase::Idle
    }

    /// Time left in the phase `elapsed_ms` after the plan was taken (a paused phase has used none of it).
    fn left_ms(&self, elapsed_ms: u64) -> i64 {
        let elapsed = if self.paused { 0 } else { elapsed_ms };
        i64::try_from(self.remaining_ms.saturating_sub(elapsed)).unwrap_or(i64::MAX)
    }

    /// Smart breaks (ADR-023): the breaks of [`SessionPlan::breaks`] with every block the calendar plans fitted around
    /// `meetings` (`focus_core::calendar::plan_smart_breaks`). A running focus block belongs to the session: its time
    /// left already has the session's own fit, so it is never fitted here. With no meetings (smart breaks off) the
    /// breaks are exactly those of [`SessionPlan::breaks`].
    pub fn smart_plan(
        &self,
        now_ms: i64,
        elapsed_ms: u64,
        horizon_ms: i64,
        meetings: &[SmartMeeting],
    ) -> SmartPlan {
        let left = self.left_ms(elapsed_ms);
        let first = match self.phase {
            Phase::Idle => FirstBreak::new(now_ms, None, None, 0, &self.profile),
            Phase::Focus => FirstBreak {
                fixed: true,
                ..FirstBreak::new(now_ms, Some(left), None, self.cycle, &self.profile)
            },
            Phase::Break => FirstBreak::new(now_ms, None, Some(left), self.cycle, &self.profile),
        };
        plan_smart_breaks(now_ms, first, &self.profile, horizon_ms, meetings)
    }

    /// The breaks that start within `horizon_ms` of `now_ms`, `elapsed_ms` after the plan was taken
    /// (a running phase has used that much of its remaining time; a paused one has not).
    #[cfg(test)]
    pub fn breaks(&self, now_ms: i64, elapsed_ms: u64, horizon_ms: i64) -> Vec<BreakWindow> {
        let left = self.left_ms(elapsed_ms);
        match self.phase {
            Phase::Idle => {
                focus_core::calendar::plan_breaks(now_ms, None, None, 0, &self.profile, horizon_ms)
            }
            Phase::Focus => focus_core::calendar::plan_breaks(
                now_ms,
                Some(left),
                None,
                self.cycle,
                &self.profile,
                horizon_ms,
            ),
            Phase::Break => focus_core::calendar::plan_breaks(
                now_ms,
                None,
                Some(left),
                self.cycle,
                &self.profile,
                horizon_ms,
            ),
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Meetings
// ---------------------------------------------------------------------------------------------------

/// Meetings of all sources merged into one list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merged {
    /// Sorted by start (then end, then id), at most [`MAX_MEETINGS`].
    pub meetings: Vec<Meeting>,
    /// `(dropped id, kept id)` for every duplicate that was dropped.
    pub aliases: Vec<(String, String)>,
}

/// Merges the meetings of the sources (given in priority order: the first copy of a meeting wins),
/// keeping those that overlap `[from_ms, to_ms)` and have a length. A meeting with the id of a kept one,
/// or with the same subject (ignoring case and spacing) starting within [`SAME_MEETING_WITHIN_MS`] of it,
/// is a duplicate.
pub fn merge(sources: &[&[Meeting]], from_ms: i64, to_ms: i64) -> Merged {
    let mut merged = Merged::default();
    let mut keys: Vec<String> = Vec::new();
    for meetings in sources {
        for meeting in *meetings {
            if meeting.end_ms <= meeting.start_ms
                || meeting.end_ms <= from_ms
                || meeting.start_ms >= to_ms
            {
                continue;
            }
            let key = subject_key(&meeting.subject);
            let duplicate = merged
                .meetings
                .iter()
                .zip(&keys)
                .find(|(kept, kept_key)| {
                    kept.id == meeting.id
                        || (**kept_key == key
                            && kept.start_ms.abs_diff(meeting.start_ms)
                                <= SAME_MEETING_WITHIN_MS.unsigned_abs())
                })
                .map(|(kept, _)| kept.id.clone());
            match duplicate {
                Some(kept) => {
                    if kept != meeting.id {
                        merged.aliases.push((meeting.id.clone(), kept));
                    }
                }
                None => {
                    merged.meetings.push(meeting.clone());
                    keys.push(key);
                }
            }
        }
    }
    merged.meetings.sort_by(|a, b| {
        a.start_ms
            .cmp(&b.start_ms)
            .then(a.end_ms.cmp(&b.end_ms))
            .then_with(|| a.id.cmp(&b.id))
    });
    merged.meetings.truncate(MAX_MEETINGS);
    merged
}

fn subject_key(subject: &str) -> String {
    subject
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

// ---------------------------------------------------------------------------------------------------
// Choices
// ---------------------------------------------------------------------------------------------------

/// The user's answer for one meeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceEntry {
    pub choice: MeetingChoice,
    /// How many times "remind later" was chosen (the stage for `reminder_schedule`).
    pub stage: u32,
    /// When the next "remind later" popup is due (wall clock).
    pub next_reminder_ms: Option<i64>,
    /// Meeting end; the entry is forgotten after it.
    pub end_ms: i64,
}

/// One line of `meeting-choices.json` (meeting ids only, never subjects).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredChoice {
    pub meeting_id: String,
    #[serde(flatten)]
    pub entry: ChoiceEntry,
}

/// One "Ask me instead" of `meeting-choices.json` (ADR-023): the meeting id and its end, after which it is forgotten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredOptOut {
    pub meeting_id: String,
    pub end_ms: i64,
}

/// `meeting-choices.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredChoices {
    pub version: u32,
    pub choices: Vec<StoredChoice>,
    /// Smart breaks (ADR-023): meetings the user wants to be asked about ("Ask me instead"). Missing in files written
    /// before ADR-023 and left out while empty, so the file stays the same for users without smart breaks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opt_outs: Vec<StoredOptOut>,
}

/// The answers per meeting id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choices {
    entries: HashMap<String, ChoiceEntry>,
    /// Smart breaks (ADR-023): "Ask me instead", meeting id → meeting end.
    opt_outs: HashMap<String, i64>,
}

impl Choices {
    /// The stored answers, without the ones whose meeting has ended.
    pub fn from_stored(stored: StoredChoices, now_ms: i64) -> Self {
        let mut choices = Self::default();
        if stored.version != CHOICES_VERSION {
            return choices;
        }
        for line in stored.choices {
            if line.entry.end_ms > now_ms && !line.meeting_id.is_empty() {
                choices.entries.insert(line.meeting_id, line.entry);
            }
        }
        for line in stored.opt_outs {
            if line.end_ms > now_ms && !line.meeting_id.is_empty() {
                choices.opt_outs.insert(line.meeting_id, line.end_ms);
            }
        }
        choices
    }

    /// For `meeting-choices.json`, sorted by id (a stable file).
    pub fn to_stored(&self) -> StoredChoices {
        let mut choices: Vec<StoredChoice> = self
            .entries
            .iter()
            .map(|(id, entry)| StoredChoice {
                meeting_id: id.clone(),
                entry: *entry,
            })
            .collect();
        choices.sort_by(|a, b| a.meeting_id.cmp(&b.meeting_id));
        let mut opt_outs: Vec<StoredOptOut> = self
            .opt_outs
            .iter()
            .map(|(id, end_ms)| StoredOptOut {
                meeting_id: id.clone(),
                end_ms: *end_ms,
            })
            .collect();
        opt_outs.sort_by(|a, b| a.meeting_id.cmp(&b.meeting_id));
        StoredChoices {
            version: CHOICES_VERSION,
            choices,
            opt_outs,
        }
    }

    /// Smart breaks (ADR-023): "Ask me instead" for `meeting` (`true`): it is no longer fitted and gets the usual
    /// question. `false` lets smart breaks fit it again: the opt-out goes and so does an answer given meanwhile (an
    /// answer also keeps a meeting from being fitted). `true` when something changed.
    pub fn set_opt_out(&mut self, meeting_id: &str, end_ms: i64, opt_out: bool) -> bool {
        if opt_out {
            self.opt_outs.insert(meeting_id.to_owned(), end_ms) != Some(end_ms)
        } else {
            let removed = self.opt_outs.remove(meeting_id).is_some();
            self.entries.remove(meeting_id).is_some() || removed
        }
    }

    /// Smart breaks (ADR-023): the user asked to decide about this meeting by hand.
    pub fn opted_out(&self, meeting_id: &str) -> bool {
        self.opt_outs.contains_key(meeting_id)
    }

    /// The opted-out meeting ids, sorted (`CalendarStatus.smart_opt_outs`).
    pub fn opt_out_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.opt_outs.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn get(&self, meeting_id: &str) -> Option<&ChoiceEntry> {
        self.entries.get(meeting_id)
    }

    /// Records `choice` for `meeting` at `now_ms`. "Remind later" schedules the next reminder (30 / 20 / 10
    /// minutes before the meeting) and fails when none is left (`calendar.remindLater`); while one is
    /// already pending it changes nothing (a second click never skips a reminder). Any other answer
    /// cancels a pending reminder. `Extend` / `PauseAtBreak` fail for a meeting longer than 4 hours
    /// (`calendar.meetingTooLong`; the session would ignore them, `meeting_hold_allowed`).
    pub fn set(
        &mut self,
        meeting: &Meeting,
        choice: MeetingChoice,
        now_ms: i64,
    ) -> Result<(), AppError> {
        if matches!(choice, MeetingChoice::Extend | MeetingChoice::PauseAtBreak)
            && !meeting_hold_allowed(meeting.start_ms, meeting.end_ms)
        {
            return Err(AppError::new(
                "calendar.meetingTooLong",
                "the meeting is longer than 4 hours: it cannot extend the focus or hold a break",
            ));
        }
        let previous = self.entries.get(&meeting.id).copied();
        if choice == MeetingChoice::RemindLater
            && let Some(entry) = self.entries.get_mut(&meeting.id)
            && entry.choice == MeetingChoice::RemindLater
            && entry.next_reminder_ms.is_some_and(|due| due > now_ms)
        {
            entry.end_ms = meeting.end_ms;
            return Ok(());
        }
        let stage = previous.map_or(0, |entry| entry.stage);
        let (stage, next_reminder_ms) = match choice {
            MeetingChoice::RemindLater => {
                let (due, _) =
                    reminder_schedule(meeting.start_ms, now_ms, stage).ok_or_else(|| {
                        AppError::new(
                            "calendar.remindLater",
                            "no reminder is left: the meeting starts in 10 minutes or less",
                        )
                    })?;
                (stage.saturating_add(1), Some(due))
            }
            _ => (stage, None),
        };
        self.entries.insert(
            meeting.id.clone(),
            ChoiceEntry {
                choice,
                stage,
                next_reminder_ms,
                end_ms: meeting.end_ms,
            },
        );
        Ok(())
    }

    /// "Remind later" may still be offered for `meeting` at `now_ms`.
    pub fn remind_later_allowed(&self, meeting: &Meeting, now_ms: i64) -> bool {
        let stage = self.entries.get(&meeting.id).map_or(0, |entry| entry.stage);
        reminder_schedule(meeting.start_ms, now_ms, stage).is_some()
    }

    /// Ids of the meetings whose reminder is due at `now_ms`; their reminder is marked as delivered.
    pub fn take_due(&mut self, now_ms: i64) -> Vec<String> {
        let mut due: Vec<(i64, String)> = Vec::new();
        for (id, entry) in &mut self.entries {
            if let Some(at) = entry.next_reminder_ms
                && at <= now_ms
            {
                entry.next_reminder_ms = None;
                due.push((at, id.clone()));
            }
        }
        due.sort();
        due.into_iter().map(|(_, id)| id).collect()
    }

    /// Puts a delivered reminder back for `at_ms` (its meeting was not read yet, e.g. right after a restart). Only a
    /// "remind later" answer is re-armed.
    pub fn retry_reminder(&mut self, meeting_id: &str, at_ms: i64) {
        if let Some(entry) = self.entries.get_mut(meeting_id)
            && entry.choice == MeetingChoice::RemindLater
        {
            entry.next_reminder_ms = Some(at_ms);
        }
    }

    /// The earliest pending reminder.
    pub fn next_due(&self) -> Option<i64> {
        self.entries
            .values()
            .filter_map(|entry| entry.next_reminder_ms)
            .min()
    }

    /// Forgets the answers (and "Ask me instead") for meetings that ended by `now_ms`; `true` when something was
    /// removed.
    pub fn prune(&mut self, now_ms: i64) -> bool {
        let before = (self.entries.len(), self.opt_outs.len());
        self.entries.retain(|_, entry| entry.end_ms > now_ms);
        self.opt_outs.retain(|_, end_ms| *end_ms > now_ms);
        (self.entries.len(), self.opt_outs.len()) != before
    }

    /// Follows the meetings: an answer (or "Ask me instead") given for a dropped duplicate moves to the copy that was
    /// kept, and a meeting that moved keeps it with the new end. `true` when something changed.
    pub fn follow(&mut self, merged: &Merged) -> bool {
        let mut changed = false;
        for (dropped, kept) in &merged.aliases {
            if !self.entries.contains_key(kept)
                && let Some(entry) = self.entries.remove(dropped)
            {
                self.entries.insert(kept.clone(), entry);
                changed = true;
            }
            if !self.opt_outs.contains_key(kept)
                && let Some(end_ms) = self.opt_outs.remove(dropped)
            {
                self.opt_outs.insert(kept.clone(), end_ms);
                changed = true;
            }
        }
        for meeting in &merged.meetings {
            if let Some(entry) = self.entries.get_mut(&meeting.id)
                && entry.end_ms != meeting.end_ms
            {
                entry.end_ms = meeting.end_ms;
                changed = true;
            }
            if let Some(end_ms) = self.opt_outs.get_mut(&meeting.id)
                && *end_ms != meeting.end_ms
            {
                *end_ms = meeting.end_ms;
                changed = true;
            }
        }
        changed
    }

    /// The meetings answered with `Extend` / `PauseAtBreak`, for the session's meeting holds.
    pub fn holds(&self, meetings: &[Meeting]) -> Vec<(Meeting, MeetingChoice)> {
        meetings
            .iter()
            .filter_map(|meeting| {
                let choice = self.entries.get(&meeting.id)?.choice;
                matches!(choice, MeetingChoice::Extend | MeetingChoice::PauseAtBreak)
                    .then(|| (meeting.clone(), choice))
            })
            .collect()
    }
}

/// The meetings that overlap a break, with the user's answers.
pub fn conflict_views(
    breaks: &[BreakWindow],
    meetings: &[Meeting],
    choices: &Choices,
    now_ms: i64,
) -> Vec<MeetingConflict> {
    conflicts(breaks, meetings)
        .into_iter()
        .filter_map(|(index, window)| {
            let meeting = meetings.get(index)?;
            Some(conflict_view(meeting, window, choices, now_ms))
        })
        .collect()
}

/// One meeting that overlaps `window`, with the user's answer.
pub fn conflict_view(
    meeting: &Meeting,
    window: BreakWindow,
    choices: &Choices,
    now_ms: i64,
) -> MeetingConflict {
    let entry = choices.get(&meeting.id);
    MeetingConflict {
        meeting: meeting.clone(),
        break_number: window.number,
        break_start_ms: window.start_ms,
        break_end_ms: window.end_ms,
        choice: entry.map(|entry| entry.choice),
        next_reminder_ms: entry.and_then(|entry| entry.next_reminder_ms),
        remind_later_allowed: choices.remind_later_allowed(meeting, now_ms),
        too_long: !meeting_hold_allowed(meeting.start_ms, meeting.end_ms),
    }
}

// ---------------------------------------------------------------------------------------------------
// Smart breaks (ADR-023)
// ---------------------------------------------------------------------------------------------------

/// The meetings smart breaks plan around (for the session and the plan): every meeting, with whether a break may be
/// moved for it: no answer from the user and no "Ask me instead". Empty while smart breaks are off. No subjects.
/// All-day and free events never get here (the platform drops them); tentative ones count, as for the question.
pub fn smart_meetings(meetings: &[Meeting], choices: &Choices, on: bool) -> Vec<SmartMeeting> {
    if !on {
        return Vec::new();
    }
    meetings
        .iter()
        .map(|meeting| SmartMeeting {
            id: meeting.id.clone(),
            start_ms: meeting.start_ms,
            end_ms: meeting.end_ms,
            eligible: choices.get(&meeting.id).is_none() && !choices.opted_out(&meeting.id),
        })
        .collect()
}

/// `CalendarStatus.smart_breaks`, in break order: the session's own moved break while a focus block runs (the
/// snapshot's), then the breaks the plan moved.
pub fn smart_break_views(
    plan: &SessionPlan,
    smart_plan: &SmartPlan,
    meetings: &[SmartMeeting],
) -> Vec<SmartBreak> {
    let own = plan.smart.clone().filter(|_| plan.phase == Phase::Focus);
    own.into_iter()
        .chain(smart_plan.moves.iter().filter_map(|(window, fit)| {
            let meeting = meetings.get(fit.meeting)?;
            Some(smart_break_view(window, fit, meeting))
        }))
        .collect()
}

/// The conflicts without the meetings a break was moved for (a break that starts with its meeting overlaps it on
/// purpose; the others no longer collide).
pub fn without_moved(
    conflicts: Vec<MeetingConflict>,
    moved: &[SmartBreak],
) -> Vec<MeetingConflict> {
    conflicts
        .into_iter()
        .filter(|conflict| !moved.iter().any(|m| m.meeting_id == conflict.meeting.id))
        .collect()
}

// ---------------------------------------------------------------------------------------------------
// ICS links
// ---------------------------------------------------------------------------------------------------

/// Validates the ICS links typed by the user: blank lines are skipped, each link must be `https://`,
/// `webcal://` or `webcals://` with a host, without spaces, at most [`MAX_ICS_URL_LEN`] characters; duplicates are
/// dropped and at most [`MAX_ICS_URLS`] links remain. Errors name the link by its number, never its text
/// (links often carry a private token).
pub fn normalize_ics_urls(urls: &[String]) -> Result<Vec<String>, AppError> {
    let mut valid: Vec<String> = Vec::new();
    for (index, url) in urls.iter().enumerate() {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let number = index + 1;
        if url.chars().count() > MAX_ICS_URL_LEN {
            return Err(ics_error(format!(
                "link {number} is longer than {MAX_ICS_URL_LEN} characters"
            )));
        }
        if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(ics_error(format!("link {number} contains spaces")));
        }
        let rest = strip_scheme(url).ok_or_else(|| {
            ics_error(format!(
                "link {number} must start with https:// or webcal://"
            ))
        })?;
        let host = rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .rsplit('@')
            .next()
            .unwrap_or_default();
        if host.is_empty() || host.starts_with(':') {
            return Err(ics_error(format!("link {number} has no server name")));
        }
        if !valid.iter().any(|kept| kept == url) {
            valid.push(url.to_owned());
        }
    }
    if valid.len() > MAX_ICS_URLS {
        return Err(ics_error(format!(
            "at most {MAX_ICS_URLS} links are allowed ({} given)",
            valid.len()
        )));
    }
    Ok(valid)
}

fn strip_scheme(url: &str) -> Option<&str> {
    [HTTPS, WEBCAL, WEBCALS]
        .iter()
        .find_map(|scheme| strip_prefix_ignore_case(url, scheme))
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| text.get(prefix.len()..))
        .flatten()
}

fn ics_error(message: String) -> AppError {
    AppError {
        code: "calendar.icsUrl".into(),
        message,
        field: Some("urls".into()),
    }
}

// ---------------------------------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------------------------------

/// What a source needs before it can sync (`None` = ready).
pub type Missing = Option<&'static str>;

/// Status of one source for the settings screen.
pub fn source_status(
    source: CalendarSource,
    enabled: bool,
    missing: Missing,
    error: Option<&str>,
    synced: bool,
    account: Option<String>,
) -> CalendarSourceStatus {
    let (state, detail) = if !enabled {
        (IntegrationState::Off, None)
    } else if let Some(missing) = missing {
        (IntegrationState::Unavailable, Some(missing.to_owned()))
    } else if let Some(error) = error {
        (IntegrationState::Error, Some(error.to_owned()))
    } else if synced {
        (IntegrationState::On, None)
    } else {
        (IntegrationState::On, Some("not synced yet".to_owned()))
    };
    CalendarSourceStatus {
        source,
        state,
        detail,
        account,
    }
}

/// An error detail safe to show and log: every word with `://` (a link, maybe with a private token)
/// becomes `<link>`, and it is at most [`MAX_DETAIL_CHARS`] characters.
pub fn redact(detail: &str) -> String {
    let redacted = detail
        .split(' ')
        .map(|word| if word.contains("://") { "<link>" } else { word })
        .collect::<Vec<_>>()
        .join(" ");
    if redacted.chars().count() <= MAX_DETAIL_CHARS {
        redacted
    } else {
        let mut short: String = redacted.chars().take(MAX_DETAIL_CHARS - 1).collect();
        short.push('…');
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_760_000_000_000;
    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;

    /// Test ids look like the platform's: `ms-a-<start>`.
    fn meeting_id(source: CalendarSource, id: &str, start: i64) -> String {
        let prefix = match source {
            CalendarSource::Microsoft => "ms",
            CalendarSource::Google => "google",
            CalendarSource::Ics => "ics",
        };
        format!("{prefix}-{id}-{start}")
    }

    fn meeting(source: CalendarSource, id: &str, subject: &str, start: i64, end: i64) -> Meeting {
        Meeting {
            id: meeting_id(source, id, start),
            source,
            subject: subject.to_owned(),
            start_ms: start,
            end_ms: end,
            online: false,
        }
    }

    fn profile(focus_minutes: u32, break_minutes: u32) -> TimingProfile {
        TimingProfile {
            focus_secs: focus_minutes * 60,
            break_secs: break_minutes * 60,
            ..TimingProfile::normal()
        }
    }

    #[test]
    fn plan_follows_the_session() {
        let p = profile(45, 15);
        let idle = SessionPlan::idle(p);
        assert!(idle.planned());
        let breaks = idle.breaks(T0, 999_999, 2 * HOUR);
        assert_eq!(breaks[0].number, 1);
        assert_eq!(breaks[0].start_ms, T0 + 45 * MIN);

        // A focus with 10 minutes left when the plan was taken, 4 minutes ago.
        let focus = SessionPlan {
            phase: Phase::Focus,
            paused: false,
            cycle: 2,
            remaining_ms: 10 * MIN as u64,
            profile: p,
            smart: None,
        };
        let breaks = focus.breaks(T0, 4 * MIN as u64, 2 * HOUR);
        assert_eq!((breaks[0].number, breaks[0].start_ms), (2, T0 + 6 * MIN));
        assert!(!focus.planned());
        // Paused: the remaining time is frozen.
        let paused = SessionPlan {
            paused: true,
            ..focus.clone()
        };
        assert_eq!(
            paused.breaks(T0, 4 * MIN as u64, 2 * HOUR)[0].start_ms,
            T0 + 10 * MIN
        );
        // Overdue: the break is now.
        assert_eq!(focus.breaks(T0, 99 * MIN as u64, 2 * HOUR)[0].start_ms, T0);
        // In a break: the next one comes after the rest of it and a full focus.
        let in_break = SessionPlan {
            phase: Phase::Break,
            remaining_ms: 5 * MIN as u64,
            ..focus.clone()
        };
        let breaks = in_break.breaks(T0, 0, 2 * HOUR);
        assert_eq!((breaks[0].number, breaks[0].start_ms), (3, T0 + 50 * MIN));
    }

    #[test]
    fn merge_drops_duplicates_across_sources() {
        use CalendarSource::{Google, Ics, Microsoft};
        let ms = [
            meeting(
                Microsoft,
                "a",
                "Planowanie sprintu",
                T0 + HOUR,
                T0 + 2 * HOUR,
            ),
            meeting(
                Microsoft,
                "b",
                "1:1",
                T0 + 3 * HOUR,
                T0 + 3 * HOUR + 30 * MIN,
            ),
        ];
        let google = [
            // Same meeting (other case and spacing, 30 s later): dropped, alias to the Microsoft copy.
            meeting(
                Google,
                "g",
                "planowanie  SPRINTU",
                T0 + HOUR + 30_000,
                T0 + 2 * HOUR,
            ),
            // Same subject, 2 minutes later: another meeting.
            meeting(Google, "h", "1:1", T0 + 3 * HOUR + 2 * MIN, T0 + 4 * HOUR),
        ];
        let ics = [
            // Already over, zero length, beyond the window: skipped.
            meeting(Ics, "old", "Old", T0 - 2 * HOUR, T0),
            meeting(Ics, "empty", "Empty", T0 + HOUR, T0 + HOUR),
            meeting(Ics, "late", "Late", T0 + 10 * HOUR, T0 + 11 * HOUR),
            // In progress: kept.
            meeting(Ics, "now", "Now", T0 - MIN, T0 + MIN),
            // The same event twice (two links): one copy.
            meeting(Ics, "dup", "Dup", T0 + 5 * HOUR, T0 + 6 * HOUR),
            meeting(Ics, "dup", "Dup", T0 + 5 * HOUR, T0 + 6 * HOUR),
        ];
        let merged = merge(&[&ms, &google, &ics], T0, T0 + 10 * HOUR);
        let ids: Vec<&str> = merged.meetings.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                meeting_id(CalendarSource::Ics, "now", T0 - MIN),
                meeting_id(CalendarSource::Microsoft, "a", T0 + HOUR),
                meeting_id(CalendarSource::Microsoft, "b", T0 + 3 * HOUR),
                meeting_id(CalendarSource::Google, "h", T0 + 3 * HOUR + 2 * MIN),
                meeting_id(CalendarSource::Ics, "dup", T0 + 5 * HOUR),
            ]
        );
        assert_eq!(
            merged.aliases,
            [(
                meeting_id(CalendarSource::Google, "g", T0 + HOUR + 30_000),
                meeting_id(CalendarSource::Microsoft, "a", T0 + HOUR)
            )]
        );
    }

    #[test]
    fn merge_keeps_the_earliest_meetings() {
        let many: Vec<Meeting> = (0..(MAX_MEETINGS as i64 + 20))
            .rev()
            .map(|i| {
                meeting(
                    CalendarSource::Ics,
                    &i.to_string(),
                    &format!("m{i}"),
                    T0 + i * MIN,
                    T0 + i * MIN + 1,
                )
            })
            .collect();
        let merged = merge(&[&many], T0, T0 + 24 * HOUR);
        assert_eq!(merged.meetings.len(), MAX_MEETINGS);
        assert_eq!(merged.meetings[0].start_ms, T0);
        assert!(
            merged
                .meetings
                .windows(2)
                .all(|w| w[0].start_ms <= w[1].start_ms)
        );
    }

    #[test]
    fn remind_later_follows_30_20_10_and_then_stops() {
        let start = T0 + 2 * HOUR;
        let m = meeting(CalendarSource::Ics, "r", "Review", start, start + HOUR);
        let mut choices = Choices::default();
        assert!(choices.remind_later_allowed(&m, T0));

        // On the Focus page, two hours before: the first reminder 30 minutes before.
        choices
            .set(&m, MeetingChoice::RemindLater, T0)
            .expect("first");
        let entry = *choices.get(&m.id).expect("entry");
        assert_eq!(
            (entry.stage, entry.next_reminder_ms),
            (1, Some(start - 30 * MIN))
        );
        // A second click while it is pending changes nothing.
        choices
            .set(&m, MeetingChoice::RemindLater, T0 + MIN)
            .expect("again");
        assert_eq!(choices.get(&m.id), Some(&entry));
        assert_eq!(choices.next_due(), Some(start - 30 * MIN));
        assert!(choices.take_due(start - 31 * MIN).is_empty());
        assert_eq!(choices.take_due(start - 30 * MIN), vec![m.id.clone()]);
        assert_eq!(choices.get(&m.id).and_then(|e| e.next_reminder_ms), None);
        assert!(
            choices.take_due(start - 30 * MIN).is_empty(),
            "delivered once"
        );

        // From the popup at -30: again in 10 minutes; at -20: the last one at -10, without "later".
        assert!(choices.remind_later_allowed(&m, start - 30 * MIN));
        choices
            .set(&m, MeetingChoice::RemindLater, start - 30 * MIN)
            .expect("second");
        assert_eq!(choices.next_due(), Some(start - 20 * MIN));
        choices.take_due(start - 20 * MIN);
        choices
            .set(&m, MeetingChoice::RemindLater, start - 20 * MIN)
            .expect("third");
        assert_eq!(choices.next_due(), Some(start - 10 * MIN));
        choices.take_due(start - 10 * MIN);
        assert!(!choices.remind_later_allowed(&m, start - 10 * MIN));
        let err = choices
            .set(&m, MeetingChoice::RemindLater, start - 10 * MIN)
            .expect_err("no reminder left");
        assert_eq!(err.code, "calendar.remindLater");
        // The answer stays "remind later" with nothing pending; another answer is still possible.
        assert_eq!(
            choices.get(&m.id).map(|e| e.choice),
            Some(MeetingChoice::RemindLater)
        );
        choices
            .set(&m, MeetingChoice::Extend, start - 5 * MIN)
            .expect("extend");
        assert_eq!(choices.next_due(), None);
    }

    #[test]
    fn remind_later_chosen_late_skips_to_the_next_stage() {
        // 25 minutes before: the -30 stage is gone, the reminder comes at -20 and still offers "later" (-10).
        let start = T0 + 25 * MIN;
        let m = meeting(CalendarSource::Ics, "late", "Late", start, start + HOUR);
        let mut choices = Choices::default();
        choices
            .set(&m, MeetingChoice::RemindLater, T0)
            .expect("at -25");
        assert_eq!(choices.next_due(), Some(start - 20 * MIN));
        assert_eq!(choices.take_due(start - 20 * MIN), vec![m.id.clone()]);
        assert!(choices.remind_later_allowed(&m, start - 20 * MIN));
        choices
            .set(&m, MeetingChoice::RemindLater, start - 20 * MIN)
            .expect("at -20");
        assert_eq!(choices.next_due(), Some(start - 10 * MIN));
        choices.take_due(start - 10 * MIN);
        assert!(
            !choices.remind_later_allowed(&m, start - 10 * MIN),
            "the -10 popup is the last one"
        );

        // 15 minutes before: straight to the last reminder (-10), which offers no "later".
        let start = T0 + 15 * MIN;
        let m = meeting(CalendarSource::Ics, "later", "Later", start, start + HOUR);
        let mut choices = Choices::default();
        assert!(choices.remind_later_allowed(&m, T0));
        choices
            .set(&m, MeetingChoice::RemindLater, T0)
            .expect("at -15");
        assert_eq!(choices.next_due(), Some(start - 10 * MIN));
        choices.take_due(start - 10 * MIN);
        assert!(!choices.remind_later_allowed(&m, start - 10 * MIN));
        // A popup answered a few seconds late still has no "later".
        assert!(!choices.remind_later_allowed(&m, start - 10 * MIN + 5_000));
    }

    #[test]
    fn a_reminder_can_be_retried_but_only_for_remind_later() {
        let start = T0 + HOUR;
        let m = meeting(CalendarSource::Ics, "r", "Review", start, start + HOUR);
        let mut choices = Choices::default();
        choices
            .set(&m, MeetingChoice::RemindLater, T0)
            .expect("set");
        assert_eq!(choices.take_due(start - 30 * MIN), vec![m.id.clone()]);
        choices.retry_reminder(&m.id, start - 29 * MIN);
        assert_eq!(choices.next_due(), Some(start - 29 * MIN));
        assert_eq!(
            choices.get(&m.id).map(|entry| entry.stage),
            Some(1),
            "a retry is not a new stage"
        );

        choices
            .set(&m, MeetingChoice::Ignore, start - 28 * MIN)
            .expect("ignore");
        choices.retry_reminder(&m.id, start - 27 * MIN);
        choices.retry_reminder("unknown", start - 27 * MIN);
        assert_eq!(choices.next_due(), None);
    }

    #[test]
    fn remind_later_is_refused_close_to_the_meeting_without_a_trace() {
        let start = T0 + 9 * MIN;
        let m = meeting(CalendarSource::Ics, "soon", "Soon", start, start + HOUR);
        let mut choices = Choices::default();
        assert!(!choices.remind_later_allowed(&m, T0));
        assert!(choices.set(&m, MeetingChoice::RemindLater, T0).is_err());
        assert_eq!(choices.get(&m.id), None);
        // Another answer cancels a pending reminder.
        let later = meeting(
            CalendarSource::Ics,
            "later",
            "Later",
            T0 + HOUR,
            T0 + 2 * HOUR,
        );
        choices
            .set(&later, MeetingChoice::RemindLater, T0)
            .expect("later");
        choices
            .set(&later, MeetingChoice::Ignore, T0)
            .expect("ignore");
        let entry = choices.get(&later.id).expect("entry");
        assert_eq!(
            (entry.choice, entry.next_reminder_ms, entry.stage),
            (MeetingChoice::Ignore, None, 1)
        );
    }

    #[test]
    fn choices_are_stored_pruned_and_follow_the_meetings() {
        let a = meeting(
            CalendarSource::Microsoft,
            "a",
            "A",
            T0 + HOUR,
            T0 + 2 * HOUR,
        );
        let b = meeting(
            CalendarSource::Google,
            "b",
            "B",
            T0 + 3 * HOUR,
            T0 + 4 * HOUR,
        );
        let mut choices = Choices::default();
        choices.set(&a, MeetingChoice::Extend, T0).expect("a");
        choices.set(&b, MeetingChoice::RemindLater, T0).expect("b");

        let stored = choices.to_stored();
        assert_eq!(stored.version, CHOICES_VERSION);
        let json = serde_json::to_value(&stored).expect("json");
        // Sorted by id: "google-b-…" before "ms-a-…".
        assert_eq!(json["choices"][0]["meetingId"], b.id.as_str());
        assert_eq!(json["choices"][0]["choice"], "remindLater");
        assert_eq!(
            json["choices"][0]["nextReminderMs"],
            T0 + 3 * HOUR - 30 * MIN
        );
        assert_eq!(json["choices"][1]["meetingId"], a.id.as_str());
        assert_eq!(json["choices"][1]["choice"], "extend");
        assert_eq!(json["choices"][1]["stage"], 0);
        let text = json.to_string();
        assert!(
            !text.contains("\"A\"") && !text.contains("subject"),
            "no subjects: {text}"
        );
        let back: StoredChoices = serde_json::from_value(json).expect("parse");
        assert_eq!(Choices::from_stored(back.clone(), T0), choices);
        // Loaded after meeting A ended: only B is left.
        let later = Choices::from_stored(back.clone(), T0 + 2 * HOUR);
        assert_eq!(later.get(&a.id), None);
        assert!(later.get(&b.id).is_some());
        // Another file version is ignored.
        let other = StoredChoices {
            version: 99,
            ..back
        };
        assert_eq!(Choices::from_stored(other, T0), Choices::default());

        // Holds: only Extend / PauseAtBreak of the listed meetings.
        assert_eq!(
            choices.holds(&[a.clone(), b.clone()]),
            [(a.clone(), MeetingChoice::Extend)]
        );
        choices.set(&b, MeetingChoice::PauseAtBreak, T0).expect("b");
        assert_eq!(
            choices.holds(std::slice::from_ref(&b)),
            [(b.clone(), MeetingChoice::PauseAtBreak)]
        );

        // A duplicate answer moves to the kept copy; a moved meeting keeps its answer.
        let ics_copy = meeting(CalendarSource::Ics, "a", "A", T0 + HOUR, T0 + 2 * HOUR);
        let mut by_ics = Choices::default();
        by_ics
            .set(&ics_copy, MeetingChoice::Ignore, T0)
            .expect("ics");
        let moved_a = Meeting {
            end_ms: T0 + 2 * HOUR + 15 * MIN,
            ..a.clone()
        };
        let merged = Merged {
            meetings: vec![moved_a.clone()],
            aliases: vec![(ics_copy.id.clone(), a.id.clone())],
        };
        assert!(by_ics.follow(&merged));
        let entry = by_ics.get(&a.id).expect("moved");
        assert_eq!(
            (entry.choice, entry.end_ms),
            (MeetingChoice::Ignore, moved_a.end_ms)
        );
        assert!(!by_ics.follow(&merged), "nothing left to change");

        assert!(choices.prune(T0 + 2 * HOUR));
        assert_eq!(choices.get(&a.id), None);
        assert!(!choices.prune(T0 + 2 * HOUR));
    }

    #[test]
    fn conflicts_carry_the_answers() {
        let p = profile(25, 5);
        let breaks = SessionPlan::idle(p).breaks(T0, 0, 3 * HOUR);
        let standup = meeting(
            CalendarSource::Ics,
            "s",
            "Standup",
            T0 + 27 * MIN,
            T0 + 42 * MIN,
        );
        let free = meeting(
            CalendarSource::Ics,
            "f",
            "Free",
            T0 + 31 * MIN,
            T0 + 54 * MIN,
        );
        let soon = meeting(
            CalendarSource::Ics,
            "n",
            "Soon",
            T0 + 5 * MIN,
            T0 + 26 * MIN,
        );
        let mut choices = Choices::default();
        choices
            .set(&standup, MeetingChoice::RemindLater, T0)
            .expect("later");
        let views = conflict_views(
            &breaks,
            &[soon.clone(), standup.clone(), free],
            &choices,
            T0,
        );
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].meeting, soon);
        assert_eq!((views[0].break_number, views[0].choice), (1, None));
        // "Soon" starts in 5 minutes: no reminder can be offered.
        assert!(!views[0].remind_later_allowed);
        assert_eq!(views[1].meeting, standup);
        assert_eq!(views[1].choice, Some(MeetingChoice::RemindLater));
        // 27 minutes away: the -30 stage has passed, the -20 one is pending.
        assert_eq!(views[1].next_reminder_ms, Some(T0 + 7 * MIN));
        assert!(views[1].remind_later_allowed);
        assert_eq!(
            (views[1].break_start_ms, views[1].break_end_ms),
            (T0 + 25 * MIN, T0 + 30 * MIN)
        );
        assert!(!views[0].too_long && !views[1].too_long);
    }

    #[test]
    fn meetings_longer_than_4_hours_cannot_hold_the_session() {
        let p = profile(25, 5);
        let breaks = SessionPlan::idle(p).breaks(T0, 0, 10 * HOUR);
        let offsite = meeting(
            CalendarSource::Microsoft,
            "o",
            "Offsite",
            T0 + 27 * MIN,
            T0 + 27 * MIN + 4 * HOUR + 1,
        );
        let workshop = meeting(
            CalendarSource::Google,
            "w",
            "Workshop",
            T0 + 3 * HOUR,
            T0 + 7 * HOUR,
        );
        let views = conflict_views(
            &breaks,
            &[offsite.clone(), workshop.clone()],
            &Choices::default(),
            T0,
        );
        assert_eq!(views.len(), 2);
        assert!(views[0].too_long, "4 h + 1 ms");
        assert!(!views[1].too_long, "exactly 4 h");

        let mut choices = Choices::default();
        for choice in [MeetingChoice::Extend, MeetingChoice::PauseAtBreak] {
            let err = choices
                .set(&offsite, choice, T0)
                .expect_err("too long for a hold");
            assert_eq!(err.code, "calendar.meetingTooLong");
            choices.set(&workshop, choice, T0).expect("4 h is fine");
        }
        assert_eq!(choices.get(&offsite.id), None);
        // Ignore and "remind later" stay possible.
        choices
            .set(&offsite, MeetingChoice::Ignore, T0)
            .expect("ignore");
        choices
            .set(&offsite, MeetingChoice::RemindLater, T0)
            .expect("later");
        assert_eq!(
            choices.holds(&[offsite, workshop.clone()]),
            vec![(workshop, MeetingChoice::PauseAtBreak)]
        );
    }

    #[test]
    fn ics_links_are_validated() {
        let links =
            |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_owned()).collect() };
        assert_eq!(
            normalize_ics_urls(&links(&[
                "  https://outlook.office365.com/owa/calendar/x/reachcalendar.ics ",
                "",
                "webcal://p01-caldav.icloud.com/published/2/abc",
                "HTTPS://Calendar.Zoho.eu/ical/x",
                "https://outlook.office365.com/owa/calendar/x/reachcalendar.ics",
            ]))
            .expect("valid"),
            links(&[
                "https://outlook.office365.com/owa/calendar/x/reachcalendar.ics",
                "webcal://p01-caldav.icloud.com/published/2/abc",
                "HTTPS://Calendar.Zoho.eu/ical/x",
            ])
        );
        assert_eq!(
            normalize_ics_urls(&[]).expect("empty"),
            Vec::<String>::new()
        );
        for bad in [
            "http://example.com/cal.ics",
            "ftp://example.com/cal.ics",
            "example.com/cal.ics",
            "https://",
            "https:///cal.ics",
            "https://:443/cal.ics",
            "https://example.com/my cal.ics",
            "webcal://",
        ] {
            let err =
                normalize_ics_urls(&links(&["https://ok.example/a.ics", bad])).expect_err(bad);
            assert_eq!(err.code, "calendar.icsUrl", "{bad}");
            assert_eq!(err.field.as_deref(), Some("urls"));
            assert!(err.message.starts_with("link 2"), "{}", err.message);
            assert!(
                !err.message.contains("example.com"),
                "the link is never echoed"
            );
        }
        let too_long = format!("https://example.com/{}", "a".repeat(MAX_ICS_URL_LEN));
        assert!(normalize_ics_urls(&[too_long]).is_err());
        let eleven: Vec<String> = (0..=MAX_ICS_URLS)
            .map(|i| format!("https://example.com/{i}.ics"))
            .collect();
        assert!(normalize_ics_urls(&eleven).is_err());
        assert_eq!(
            normalize_ics_urls(&eleven[..MAX_ICS_URLS])
                .expect("ten")
                .len(),
            10
        );
        assert_eq!(
            normalize_ics_urls(&links(&["webcals://example.com/a.ics"])).expect("webcals"),
            links(&["webcals://example.com/a.ics"])
        );
    }

    #[test]
    fn statuses_and_details() {
        let off = source_status(
            CalendarSource::Google,
            false,
            Some("x"),
            Some("y"),
            true,
            None,
        );
        assert_eq!((off.state, off.detail), (IntegrationState::Off, None));
        let missing = source_status(
            CalendarSource::Google,
            true,
            Some("connect"),
            Some("y"),
            true,
            None,
        );
        assert_eq!(missing.state, IntegrationState::Unavailable);
        assert_eq!(missing.detail.as_deref(), Some("connect"));
        let error = source_status(
            CalendarSource::Ics,
            true,
            None,
            Some("boom"),
            true,
            Some("2".into()),
        );
        assert_eq!(
            (error.state, error.detail.as_deref()),
            (IntegrationState::Error, Some("boom"))
        );
        assert_eq!(error.account.as_deref(), Some("2"));
        let on = source_status(CalendarSource::Microsoft, true, None, None, true, None);
        assert_eq!((on.state, on.detail), (IntegrationState::On, None));
        let waiting = source_status(CalendarSource::Microsoft, true, None, None, false, None);
        assert_eq!(waiting.state, IntegrationState::On);
        assert!(waiting.detail.is_some());

        assert_eq!(
            redact("error sending request for url (https://cal.example/x?token=s3cret): timed out"),
            "error sending request for url <link> timed out"
        );
        assert_eq!(redact("HTTP 404"), "HTTP 404");
        assert!(redact(&"x ".repeat(400)).ends_with('…'));
    }

    // -- Smart breaks (ADR-023) -----------------------------------------------------------------------------

    fn smart_on(meetings: &[Meeting], choices: &Choices) -> Vec<SmartMeeting> {
        smart_meetings(meetings, choices, true)
    }

    #[test]
    fn smart_breaks_off_change_nothing() {
        let p = profile(45, 15);
        let planning = meeting(
            CalendarSource::Ics,
            "p",
            "Planning",
            T0 + 57 * MIN,
            T0 + 90 * MIN,
        );
        let choices = Choices::default();
        assert!(smart_meetings(std::slice::from_ref(&planning), &choices, false).is_empty());
        let idle = SessionPlan::idle(p);
        let focus = SessionPlan {
            phase: Phase::Focus,
            cycle: 2,
            remaining_ms: 10 * MIN as u64,
            ..idle.clone()
        };
        let in_break = SessionPlan {
            phase: Phase::Break,
            ..focus.clone()
        };
        for plan in [&idle, &focus, &in_break] {
            let smart = plan.smart_plan(T0, 60_000, 4 * HOUR, &[]);
            assert_eq!(smart.breaks, plan.breaks(T0, 60_000, 4 * HOUR), "{plan:?}");
            assert!(smart.moves.is_empty());
            assert!(smart_break_views(plan, &smart, &[]).is_empty());
        }
        // Without smart breaks the meeting is asked about as before.
        let breaks = idle.smart_plan(T0, 0, 4 * HOUR, &[]).breaks;
        let conflicts = conflict_views(&breaks, std::slice::from_ref(&planning), &choices, T0);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(without_moved(conflicts.clone(), &[]), conflicts);
    }

    #[test]
    fn smart_breaks_move_planned_breaks_and_drop_their_conflicts() {
        let p = profile(45, 15);
        // 0:57–1:30 collides with break 1 (0:45–1:00); "Review" 2:40–2:50 with break 3, but it was answered.
        let planning = meeting(
            CalendarSource::Ics,
            "p",
            "Planning",
            T0 + 57 * MIN,
            T0 + 90 * MIN,
        );
        let review = meeting(
            CalendarSource::Ics,
            "r",
            "Review",
            T0 + 160 * MIN,
            T0 + 170 * MIN,
        );
        let meetings = [planning.clone(), review.clone()];
        let mut choices = Choices::default();
        choices
            .set(&review, MeetingChoice::Ignore, T0)
            .expect("ignore");
        let smart = smart_on(&meetings, &choices);
        assert_eq!(
            smart.iter().map(|m| m.eligible).collect::<Vec<_>>(),
            [true, false]
        );
        let idle = SessionPlan::idle(p);
        let planned = idle.smart_plan(T0, 0, 4 * HOUR, &smart);
        // Break 1 ends when the planning starts (0:42–0:57); break 2 at 1:42, break 3 at 2:42 collides with the
        // answered review and stays.
        assert_eq!(
            planned
                .breaks
                .iter()
                .map(|w| (w.number, (w.start_ms - T0) / MIN))
                .collect::<Vec<_>>(),
            [(1, 42), (2, 102), (3, 162), (4, 222)]
        );
        let views = smart_break_views(&idle, &planned, &smart);
        assert_eq!(views.len(), 1);
        assert_eq!(
            (
                views[0].meeting_id.as_str(),
                views[0].break_number,
                views[0].focus_delta_secs
            ),
            (planning.id.as_str(), 1, -180)
        );
        let conflicts = without_moved(
            conflict_views(&planned.breaks, &meetings, &choices, T0),
            &views,
        );
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            (&conflicts[0].meeting, conflicts[0].break_number),
            (&review, 3)
        );

        // "Ask me instead": the planning is asked about again, its break back at 0:45.
        assert!(choices.set_opt_out(&planning.id, planning.end_ms, true));
        assert!(!choices.set_opt_out(&planning.id, planning.end_ms, true));
        assert_eq!(choices.opt_out_ids(), std::slice::from_ref(&planning.id));
        let smart = smart_on(&meetings, &choices);
        assert!(!smart[0].eligible);
        let planned = idle.smart_plan(T0, 0, 4 * HOUR, &smart);
        assert_eq!(planned.breaks[0].start_ms, T0 + 45 * MIN);
        assert!(smart_break_views(&idle, &planned, &smart).is_empty());
        let conflicts = conflict_views(&planned.breaks, &meetings, &choices, T0);
        assert_eq!(conflicts[0].meeting, planning);

        // Stored with the answers (ids only) and forgotten after the meeting.
        let json = serde_json::to_value(choices.to_stored()).expect("json");
        assert_eq!(json["optOuts"][0]["meetingId"], planning.id.as_str());
        assert_eq!(json["optOuts"][0]["endMs"], planning.end_ms);
        assert!(!json.to_string().contains("Planning"));
        let back: StoredChoices = serde_json::from_value(json).expect("parse");
        assert_eq!(Choices::from_stored(back.clone(), T0), choices);
        assert!(!Choices::from_stored(back, T0 + 2 * HOUR).opted_out(&planning.id));
        let mut pruned = choices.clone();
        assert!(pruned.prune(T0 + 2 * HOUR));
        assert!(pruned.opt_out_ids().is_empty());

        // A file from before ADR-023 loads; an empty list is left out of the file.
        let old: StoredChoices =
            serde_json::from_str(r#"{"version":1,"choices":[]}"#).expect("old file");
        assert!(old.opt_outs.is_empty());
        let empty = serde_json::to_value(Choices::default().to_stored()).expect("json");
        assert!(empty.get("optOuts").is_none());

        // "Fit it again" also forgets an answer given meanwhile, so the meeting may be fitted.
        choices
            .set(&planning, MeetingChoice::Ignore, T0)
            .expect("ignore");
        assert!(choices.set_opt_out(&planning.id, planning.end_ms, false));
        assert!(choices.get(&planning.id).is_none() && !choices.opted_out(&planning.id));
        assert!(smart_on(&meetings, &choices)[0].eligible);
        assert!(!choices.set_opt_out(&planning.id, planning.end_ms, false));
    }

    #[test]
    fn opt_outs_follow_merged_duplicates_and_moved_ends() {
        let kept = meeting(
            CalendarSource::Microsoft,
            "k",
            "Sync",
            T0 + HOUR,
            T0 + 2 * HOUR,
        );
        let dropped = meeting(CalendarSource::Ics, "d", "Sync", T0 + HOUR, T0 + 2 * HOUR);
        let mut choices = Choices::default();
        choices.set_opt_out(&dropped.id, dropped.end_ms, true);
        let longer = Meeting {
            end_ms: T0 + 3 * HOUR,
            ..kept.clone()
        };
        let merged = merge(
            &[
                std::slice::from_ref(&longer),
                std::slice::from_ref(&dropped),
            ],
            T0,
            T0 + 4 * HOUR,
        );
        assert!(choices.follow(&merged));
        assert!(choices.opted_out(&kept.id) && !choices.opted_out(&dropped.id));
        // The kept copy's new end keeps it until then.
        assert!(!choices.prune(T0 + 2 * HOUR + 1));
        assert!(choices.prune(T0 + 3 * HOUR));
    }

    #[test]
    fn a_running_focus_block_belongs_to_the_session() {
        let p = profile(45, 15);
        let planning = meeting(
            CalendarSource::Ics,
            "p",
            "Planning",
            T0 + 12 * MIN,
            T0 + 40 * MIN,
        );
        let smart = smart_on(std::slice::from_ref(&planning), &Choices::default());
        // The session fitted its block (break at 0:12 with the meeting instead of 0:09) and reports it.
        let own = SmartBreak {
            meeting_id: planning.id.clone(),
            meeting_start_ms: planning.start_ms,
            meeting_end_ms: planning.end_ms,
            break_number: 2,
            kind: focus_core::api::SmartBreakKind::StartsAtMeeting,
            focus_delta_secs: 180,
            break_start_ms: T0 + 12 * MIN,
            break_end_ms: T0 + 27 * MIN,
        };
        let focus = SessionPlan {
            phase: Phase::Focus,
            paused: false,
            cycle: 2,
            remaining_ms: 12 * MIN as u64,
            profile: p,
            smart: Some(own.clone()),
        };
        let planned = focus.smart_plan(T0, 0, 2 * HOUR, &smart);
        // Not fitted again here: the break stays where the session put it.
        assert_eq!(
            planned.breaks[0],
            BreakWindow {
                number: 2,
                start_ms: T0 + 12 * MIN,
                end_ms: T0 + 27 * MIN,
            }
        );
        assert!(planned.moves.is_empty());
        let views = smart_break_views(&focus, &planned, &smart);
        assert_eq!(views, std::slice::from_ref(&own));
        let conflicts = conflict_views(
            &planned.breaks,
            std::slice::from_ref(&planning),
            &Choices::default(),
            T0,
        );
        assert_eq!(
            conflicts.len(),
            1,
            "the break overlaps the meeting on purpose"
        );
        assert!(without_moved(conflicts, &views).is_empty());
        // During the moved break the session's view is not an upcoming break.
        let in_break = SessionPlan {
            phase: Phase::Break,
            remaining_ms: 10 * MIN as u64,
            ..focus.clone()
        };
        let planned = in_break.smart_plan(T0, 0, 2 * HOUR, &smart);
        assert!(smart_break_views(&in_break, &planned, &smart).is_empty());
    }
}
