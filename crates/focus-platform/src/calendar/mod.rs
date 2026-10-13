//! Calendar meetings, read-only, from three kinds of sources:
//!
//! * Microsoft 365 / Outlook: [`crate::teams::TeamsClient::calendar_view`] (Graph, the Microsoft sign-in used for
//!   Teams, `Calendars.Read`).
//! * Google Calendar: [`google`] (OAuth desktop flow with PKCE and a loopback redirect, `calendar.readonly`).
//! * Any other calendar through an ICS / iCal subscription link: [`ics`] (Exchange on-premises publishing, Zoho,
//!   Zimbra, Nextcloud, iCloud, Proton, …), with recurring events expanded here.
//!
//! Every source returns [`CalEvent`]s that overlap the requested range, already passed through
//! [`meetings_only`]: no cancelled or declined events, no all-day events, no events shown as free.
//! All times are UTC Unix milliseconds. Calls are **blocking** (10 s timeouts): use them on a worker thread,
//! never inside an async runtime. Tokens, codes and ICS links (they are secrets) are never logged.

pub mod google;
pub mod ics;
mod rrule;
pub(crate) mod time;
mod tz;

use focus_core::api::{CalendarSource, Meeting};

pub use time::format_utc;
pub use tz::TimeZone;

/// Most events one source returns (also the cap of expanded ICS occurrences).
pub const MAX_EVENTS: usize = 2000;
/// Subjects are cut to this many characters.
pub const MAX_SUBJECT_CHARS: usize = 200;

/// One calendar event (one occurrence of a recurring event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalEvent {
    /// The provider's id. Graph and Google: one per occurrence. ICS: the UID, shared by the occurrences of a
    /// recurring event (combine it with `start_ms`, as [`meeting_id`] does).
    pub id: String,
    /// Single line, at most [`MAX_SUBJECT_CHARS`] characters; may be empty.
    pub subject: String,
    pub start_ms: i64,
    pub end_ms: i64,
    /// A Teams / Meet / Zoom / Webex (…) link or online meeting flag is present.
    pub online: bool,
    pub all_day: bool,
    /// Blocks time (busy, tentative, out of office); `false` for events shown as free / working elsewhere.
    pub busy: bool,
}

impl CalEvent {
    /// The contract DTO, with the stable id from [`meeting_id`].
    pub fn to_meeting(&self, source: CalendarSource) -> Meeting {
        Meeting {
            id: meeting_id(source, &self.id, self.start_ms),
            source,
            subject: self.subject.clone(),
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            online: self.online,
        }
    }
}

/// Stable meeting id: source + hash of the provider's event id + start (`ms-0123456789abcdef-1760000000000`).
/// A moved meeting gets a new id (a choice made for the old time does not silently apply to the new one).
pub fn meeting_id(source: CalendarSource, event_id: &str, start_ms: i64) -> String {
    let prefix = match source {
        CalendarSource::Microsoft => "ms",
        CalendarSource::Google => "google",
        CalendarSource::Ics => "ics",
    };
    format!("{prefix}-{:016x}-{start_ms}", fnv1a(event_id.as_bytes()))
}

/// Keeps the events that are meetings worth planning around and overlap `[start_ms, end_ms)`: drops all-day,
/// free and zero-length events; sorts by start (then end, id); removes exact duplicates; caps at [`MAX_EVENTS`].
pub fn meetings_only(mut events: Vec<CalEvent>, start_ms: i64, end_ms: i64) -> Vec<CalEvent> {
    events.retain(|e| {
        !e.all_day && e.busy && e.end_ms > e.start_ms && e.start_ms < end_ms && e.end_ms > start_ms
    });
    events.sort_by(|a, b| {
        (a.start_ms, a.end_ms, &a.id, &a.subject).cmp(&(b.start_ms, b.end_ms, &b.id, &b.subject))
    });
    events.dedup_by(|a, b| a.id == b.id && a.start_ms == b.start_ms && a.end_ms == b.end_ms);
    events.truncate(MAX_EVENTS);
    events
}

/// One line, collapsed whitespace, control characters removed, at most [`MAX_SUBJECT_CHARS`] characters.
pub(crate) fn clean_subject(text: &str) -> String {
    let mut out = String::new();
    for word in text.split(|c: char| c.is_whitespace() || c.is_control()) {
        if word.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    match out.char_indices().nth(MAX_SUBJECT_CHARS) {
        Some((cut, _)) => out[..cut].to_owned(),
        None => out,
    }
}

/// Hosts of online meeting links.
const ONLINE_MEETING_HOSTS: &[&str] = &[
    "teams.microsoft.com",
    "teams.live.com",
    "meet.google.com",
    "zoom.us/",
    "zoom.com/",
    "zoomgov.com/",
    "webex.com",
    "gotomeeting.com",
    "meet.goto.com",
    "whereby.com",
    "bluejeans.com",
    "chime.aws",
    "meet.jit.si",
    "join.skype.com",
    "meet.lync.com",
    "lifesize.com",
    "ringcentral.com",
];

/// `text` contains a link to an online meeting.
pub(crate) fn looks_online(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    ONLINE_MEETING_HOSTS.iter().any(|host| lower.contains(host))
}

/// FNV-1a 64-bit (stable across Rust versions, unlike `DefaultHasher`).
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, start: i64, end: i64) -> CalEvent {
        CalEvent {
            id: id.into(),
            subject: format!("Meeting {id}"),
            start_ms: start,
            end_ms: end,
            online: false,
            all_day: false,
            busy: true,
        }
    }

    #[test]
    fn filters_and_sorts() {
        let mut all_day = event("all-day", 0, 1000);
        all_day.all_day = true;
        let mut free = event("free", 100, 200);
        free.busy = false;
        let events = vec![
            event("b", 500, 600),
            event("a", 100, 200),
            event("a", 100, 200),
            event("zero", 300, 300),
            event("before", -100, 0),
            event("after", 1000, 1100),
            event("spanning", -100, 2000),
            all_day,
            free,
        ];
        let ids: Vec<String> = meetings_only(events, 0, 1000)
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, vec!["spanning", "a", "b"]);
    }

    #[test]
    fn caps_the_number_of_events() {
        let events = (0..MAX_EVENTS as i64 + 10)
            .map(|i| event(&i.to_string(), i, i + 1))
            .collect();
        assert_eq!(meetings_only(events, 0, i64::MAX).len(), MAX_EVENTS);
    }

    #[test]
    fn subjects_are_cleaned() {
        assert_eq!(
            clean_subject("  Daily\r\n  standup\t(team) "),
            "Daily standup (team)"
        );
        assert_eq!(clean_subject(""), "");
        let long = "ą".repeat(MAX_SUBJECT_CHARS + 5);
        assert_eq!(clean_subject(&long).chars().count(), MAX_SUBJECT_CHARS);
    }

    #[test]
    fn online_links_are_detected() {
        assert!(looks_online(
            "Microsoft Teams Meeting https://teams.microsoft.com/l/meetup-join/19%3a"
        ));
        assert!(looks_online("https://meet.google.com/abc-defg-hij"));
        assert!(looks_online("Join: https://us02web.ZOOM.us/j/123"));
        assert!(!looks_online("Room 4.12, building B"));
        assert!(!looks_online(""));
    }

    #[test]
    fn meeting_ids_are_stable_and_distinct() {
        let a = meeting_id(CalendarSource::Ics, "uid-1@example.com", 1_760_000_000_000);
        assert_eq!(
            a,
            meeting_id(CalendarSource::Ics, "uid-1@example.com", 1_760_000_000_000)
        );
        assert!(
            a.starts_with("ics-") && a.ends_with("-1760000000000"),
            "{a}"
        );
        assert_ne!(
            a,
            meeting_id(
                CalendarSource::Google,
                "uid-1@example.com",
                1_760_000_000_000
            )
        );
        assert_ne!(
            a,
            meeting_id(CalendarSource::Ics, "uid-1@example.com", 1_760_000_600_000)
        );
        assert_ne!(
            a,
            meeting_id(CalendarSource::Ics, "uid-2@example.com", 1_760_000_000_000)
        );
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);

        let meeting = event("x", 10, 20).to_meeting(CalendarSource::Microsoft);
        assert_eq!(meeting.id, meeting_id(CalendarSource::Microsoft, "x", 10));
        assert_eq!((meeting.start_ms, meeting.end_ms), (10, 20));
        assert_eq!(meeting.subject, "Meeting x");
        assert_eq!(meeting.source, CalendarSource::Microsoft);
    }
}
