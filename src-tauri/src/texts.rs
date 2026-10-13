//! Polish texts shown by the shell itself (toasts, tray menu, window titles).
//!
//! The WebView UI keeps its texts in `src/lib/strings.ts`; the shell needs its own copy for toasts
//! and the tray because they must work even when the WebView is hidden or busy.
// TODO(hackathon): move to strings.ts if the UI ever drives the toasts itself.

use focus_core::api::{Phase, SessionSnapshot};

pub const TRAY_START: &str = "Start";
pub const TRAY_PAUSE: &str = "Pauza";
pub const TRAY_RESUME: &str = "Wznów";
pub const TRAY_SKIP: &str = "Pomiń fazę";
pub const TRAY_STOP: &str = "Stop";
pub const TRAY_SHOW: &str = "Pokaż okno";
pub const TRAY_QUIT: &str = "Zakończ";

/// Title of the separate YouTube window (links that refuse embedding).
pub const MEDIA_WINDOW_TITLE: &str = "FocusMode — muzyka (YouTube)";
/// Title of the phase popup window (bottom-right corner; seen only in Alt+Tab).
pub const POPUP_WINDOW_TITLE: &str = "FocusMode — powiadomienie";

/// A toast notification: title and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub title: String,
    pub body: String,
}

impl Toast {
    fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

/// Focus ended, break begins. `reminder` (the user's own text for this break, `Config::break_reminders`)
/// replaces the generic suggestion when present.
pub fn break_toast(break_secs: u32, reminder: Option<&str>) -> Toast {
    let back_in = duration_pl(break_secs);
    let body = match reminder.map(str::trim).filter(|text| !text.is_empty()) {
        Some(reminder) => format!("Przypomnienie: {reminder}\nWracamy za {back_in}."),
        None => format!("Wstań, rozciągnij się, napij się wody. Wracamy za {back_in}."),
    };
    Toast::new("Czas na przerwę ☕", body)
}

/// Break ended, next focus cycle begins.
pub fn focus_toast(focus_secs: u32, cycle: u32) -> Toast {
    Toast::new(
        "Wracamy do pracy 🎧",
        format!("Cykl {cycle}: {} skupienia.", duration_pl(focus_secs)),
    )
}

/// Another launch asked for the demo profile (`--demo`, "tryb prezentacji" in the UI) while a session runs.
pub fn demo_needs_stop_toast() -> Toast {
    Toast::new(
        "Tryb prezentacji",
        "Zatrzymaj sesję, aby włączyć tryb prezentacji.",
    )
}

/// The session was auto-paused after a sleep/clock gap.
pub fn interrupted_toast() -> Toast {
    Toast::new(
        "Sesja wstrzymana",
        "Wykryto uśpienie komputera — wznów, kiedy wrócisz.",
    )
}

/// The break started paused because a meeting runs (`MeetingChoice::PauseAtBreak`). `subject` is the
/// meeting subject (`None` when empty), `reminder` the user's own text for this break.
pub fn break_held_toast(subject: Option<&str>, break_secs: u32, reminder: Option<&str>) -> Toast {
    let meeting = match subject.map(short_subject).filter(|text| !text.is_empty()) {
        Some(subject) => format!("Przerwa wstrzymana — trwa spotkanie: {subject}."),
        None => "Przerwa wstrzymana — trwa spotkanie.".to_owned(),
    };
    let resume = if break_secs > 0 {
        format!(
            "Wznów, kiedy skończysz: przerwa potrwa {}.",
            duration_pl(break_secs)
        )
    } else {
        "Wznów, kiedy skończysz.".to_owned()
    };
    let body = match reminder.map(str::trim).filter(|text| !text.is_empty()) {
        Some(reminder) => format!("{meeting}\n{resume}\nPrzypomnienie: {reminder}"),
        None => format!("{meeting}\n{resume}"),
    };
    Toast::new("Przerwa wstrzymana 📅", body)
}

/// "Remind later" is due for a meeting that collides with a break (fallback when the popup window cannot
/// be shown). `start_local` is the meeting start as local `HH:MM`, `minutes_until` how soon it starts.
pub fn meeting_reminder_toast(
    subject: &str,
    start_local: Option<&str>,
    minutes_until: i64,
    break_number: u32,
) -> Toast {
    let subject = short_subject(subject);
    let subject = if subject.is_empty() {
        "Spotkanie".to_owned()
    } else {
        format!("„{subject}”")
    };
    let when = match (start_local, minutes_until) {
        (Some(clock), minutes) if minutes > 0 => format!("o {clock} (za {minutes} min)"),
        (Some(clock), _) => format!("o {clock}"),
        (None, minutes) if minutes > 0 => format!("za {minutes} min"),
        (None, _) => "teraz".to_owned(),
    };
    Toast::new(
        "Spotkanie w czasie przerwy 📅",
        format!(
            "{subject} {when} wypada w przerwie {break_number}. \
             Otwórz FocusMode i wybierz: przedłużyć skupienie, wstrzymać przerwę czy zignorować."
        ),
    )
}

/// A meeting subject for a toast: trimmed, whitespace collapsed, at most [`SUBJECT_MAX_CHARS`] characters.
fn short_subject(subject: &str) -> String {
    let collapsed = subject.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= SUBJECT_MAX_CHARS {
        return collapsed;
    }
    let mut short: String = collapsed.chars().take(SUBJECT_MAX_CHARS - 1).collect();
    short.push('…');
    short
}

/// Longest meeting subject shown in a toast.
const SUBJECT_MAX_CHARS: usize = 80;

/// `HH:MM` local time of `at_ms` (Unix ms), given the local time of day now (`local_minutes`, from
/// `focus_platform::clock::local_minutes_of_day`) and the wall clock now (`now_ms`). The UTC offset is
/// taken from the two and rounded to a quarter hour; `None` when the local time is unknown.
pub fn local_clock(at_ms: i64, now_ms: i64, local_minutes: Option<f64>) -> Option<String> {
    const MINUTES_PER_DAY: i64 = 24 * 60;
    const MS_PER_MINUTE: i64 = 60_000;
    let local_minutes = local_minutes.filter(|minutes| minutes.is_finite())?;
    let utc_minutes = now_ms.rem_euclid(MINUTES_PER_DAY * MS_PER_MINUTE) as f64 / 60_000.0;
    // Offsets are whole quarter hours (UTC-12 … UTC+14); wrap the difference into ±12 h first.
    let mut offset = local_minutes - utc_minutes;
    if offset > 720.0 {
        offset -= 1440.0;
    } else if offset < -720.0 {
        offset += 1440.0;
    }
    let offset = ((offset / 15.0).round() as i64) * 15;
    let minute_of_day = (at_ms.div_euclid(MS_PER_MINUTE) + offset).rem_euclid(MINUTES_PER_DAY);
    Some(format!(
        "{:02}:{:02}",
        minute_of_day / 60,
        minute_of_day % 60
    ))
}

/// Game launchers were closed when focus started (`names` from `focus_core::launchers::display_name`).
pub fn launchers_closed_toast(names: &[&str]) -> Toast {
    Toast::new(
        "Zamknięto launchery gier 🎮",
        format!(
            "{}. Uruchomisz je ponownie, kiedy zechcesz.",
            names.join(", ")
        ),
    )
}

/// New work items for "Co dalej?" (held back during focus, shown at the break or after the session).
/// Fallback when the popup window cannot be shown; never names an item (titles stay in the app).
pub fn work_digest_toast(count: u32) -> Toast {
    let (items, waits) = match plural_pl(count) {
        PluralPl::One => ("nowa pozycja", "czeka"),
        PluralPl::Few => ("nowe pozycje", "czekają"),
        PluralPl::Many => ("nowych pozycji", "czeka"),
    };
    Toast::new(
        "Co dalej? 📋",
        format!("{count} {items} z Twoich narzędzi {waits} w FocusMode."),
    )
}

/// Polish plural form of a count: 1 → one, 2–4 (not 12–14), 22–24, … → few, otherwise many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluralPl {
    One,
    Few,
    Many,
}

fn plural_pl(count: u32) -> PluralPl {
    if count == 1 {
        PluralPl::One
    } else if (2..=4).contains(&(count % 10)) && !(12..=14).contains(&(count % 100)) {
        PluralPl::Few
    } else {
        PluralPl::Many
    }
}

/// A blocked site was detected in the foreground browser window. `matched` is a domain, or a regex
/// source when `is_pattern` (not readable as a site name, so it is not shown).
pub fn nudge_toast(matched: &str, is_pattern: bool) -> Toast {
    let body = if is_pattern {
        "Ta strona jest na liście blokowanych i poczeka do przerwy.".to_owned()
    } else {
        format!("{matched} poczeka do przerwy.")
    };
    Toast::new("Wracaj do pracy", body)
}

pub fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "Gotowy",
        Phase::Focus => "Skupienie",
        Phase::Break => "Przerwa",
    }
}

/// Tray tooltip, e.g. `FocusMode — Skupienie 12:34`.
pub fn tray_tooltip(snapshot: &SessionSnapshot) -> String {
    match snapshot.phase {
        Phase::Idle => "FocusMode — gotowy do skupienia".to_owned(),
        phase => {
            let paused = if snapshot.paused { " (pauza)" } else { "" };
            format!(
                "FocusMode — {} {}{paused}",
                phase_label(phase),
                clock(snapshot.remaining_secs)
            )
        }
    }
}

/// First (disabled) line of the tray menu.
pub fn tray_status(snapshot: &SessionSnapshot) -> String {
    match snapshot.phase {
        Phase::Idle => "Gotowy do skupienia".to_owned(),
        phase => {
            let prefix = if snapshot.paused { "Pauza · " } else { "" };
            format!(
                "{prefix}{} · {} · cykl {}",
                phase_label(phase),
                clock(snapshot.remaining_secs),
                snapshot.cycle
            )
        }
    }
}

/// `mm:ss`, or `h:mm:ss` from one hour up.
pub fn clock(secs: u32) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Human duration in Polish: `20 s`, `15 min`, `1 h 30 min`.
pub fn duration_pl(secs: u32) -> String {
    if secs < 60 {
        return format!("{secs} s");
    }
    // Round to the nearest minute so 44:59 reads as 45 min.
    let minutes = secs.saturating_add(30) / 60;
    let hours = minutes / 60;
    let rest = minutes % 60;
    match (hours, rest) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration_pl(20), "20 s");
        assert_eq!(duration_pl(15 * 60), "15 min");
        assert_eq!(duration_pl(45 * 60 - 1), "45 min");
        assert_eq!(duration_pl(90 * 60), "1 h 30 min");
        assert_eq!(duration_pl(120 * 60), "2 h");
    }

    #[test]
    fn clock_formats() {
        assert_eq!(clock(0), "00:00");
        assert_eq!(clock(754), "12:34");
        assert_eq!(clock(3723), "1:02:03");
    }

    #[test]
    fn break_toast_shows_the_reminder() {
        let plain = break_toast(15 * 60, None);
        assert!(plain.body.contains("Wstań"));
        let reminder = break_toast(15 * 60, Some("  obiad  "));
        assert!(reminder.body.starts_with("Przypomnienie: obiad\n"));
        assert!(reminder.body.contains("15 min"));
        assert!(!reminder.body.contains("Wstań"));
        assert_eq!(break_toast(60, Some("   ")), break_toast(60, None));
    }

    #[test]
    fn held_break_toast_names_the_meeting() {
        let toast = break_held_toast(Some("  Daily   standup "), 15 * 60, None);
        assert!(
            toast
                .body
                .starts_with("Przerwa wstrzymana — trwa spotkanie: Daily standup.")
        );
        assert!(toast.body.contains("15 min"));
        assert!(!toast.body.contains("Przypomnienie"));
        let anonymous = break_held_toast(None, 0, Some(" obiad "));
        assert!(
            anonymous
                .body
                .starts_with("Przerwa wstrzymana — trwa spotkanie.\n")
        );
        assert!(anonymous.body.ends_with("Przypomnienie: obiad"));
        assert_eq!(
            break_held_toast(Some("   "), 60, None).body,
            break_held_toast(None, 60, None).body
        );
    }

    #[test]
    fn meeting_reminder_toast_says_what_and_when() {
        let toast = meeting_reminder_toast("Przegląd sprintu", Some("14:30"), 30, 2);
        assert!(
            toast
                .body
                .starts_with("„Przegląd sprintu” o 14:30 (za 30 min) wypada w przerwie 2.")
        );
        let long = "x".repeat(200);
        let toast = meeting_reminder_toast(&long, None, 0, 1);
        assert!(toast.body.contains('…'));
        assert!(toast.body.contains("teraz"));
        assert!(!toast.body.contains(&"x".repeat(81)));
        assert!(
            meeting_reminder_toast(" ", None, 5, 1)
                .body
                .starts_with("Spotkanie za 5 min")
        );
    }

    #[test]
    fn local_clock_uses_the_local_offset() {
        const HOUR: i64 = 3_600_000;
        // 2026-10-13 10:00 UTC; local time 12:00 (UTC+2).
        let now: i64 = 1_791_885_600_000;
        assert_eq!(now.rem_euclid(24 * HOUR), 10 * HOUR);
        let local_noon = Some(12.0 * 60.0 + 0.2);
        assert_eq!(local_clock(now, now, local_noon).as_deref(), Some("12:00"));
        assert_eq!(
            local_clock(now + 90 * 60_000, now, local_noon).as_deref(),
            Some("13:30")
        );
        // Across midnight and with a negative offset (UTC-5 → 05:00 local).
        assert_eq!(
            local_clock(now + 14 * HOUR, now, local_noon).as_deref(),
            Some("02:00")
        );
        assert_eq!(
            local_clock(now, now, Some(5.0 * 60.0)).as_deref(),
            Some("05:00")
        );
        // UTC+14 (local 00:00 at 10:00 UTC) wraps to UTC-10: the same time of day.
        assert_eq!(local_clock(now, now, Some(0.05)).as_deref(), Some("00:00"));
        assert_eq!(
            local_clock(now + HOUR, now, Some(0.05)).as_deref(),
            Some("01:00")
        );
        assert_eq!(local_clock(now, now, None), None);
        assert_eq!(local_clock(now, now, Some(f64::NAN)), None);
    }

    #[test]
    fn work_digest_toast_uses_polish_plurals() {
        assert_eq!(
            work_digest_toast(1).body,
            "1 nowa pozycja z Twoich narzędzi czeka w FocusMode."
        );
        assert_eq!(
            work_digest_toast(3).body,
            "3 nowe pozycje z Twoich narzędzi czekają w FocusMode."
        );
        assert_eq!(
            work_digest_toast(5).body,
            "5 nowych pozycji z Twoich narzędzi czeka w FocusMode."
        );
        assert!(work_digest_toast(12).body.starts_with("12 nowych pozycji"));
        assert!(work_digest_toast(22).body.starts_with("22 nowe pozycje"));
        assert!(work_digest_toast(0).body.starts_with("0 nowych pozycji"));
        assert_eq!(plural_pl(114), PluralPl::Many);
        assert_eq!(plural_pl(104), PluralPl::Few);
    }

    #[test]
    fn toasts_contain_numbers() {
        assert!(break_toast(15 * 60, None).body.contains("15 min"));
        assert!(focus_toast(45 * 60, 2).body.contains("Cykl 2"));
        assert!(nudge_toast("reddit.com", false).body.contains("reddit.com"));
        let launchers = launchers_closed_toast(&["Steam", "GOG Galaxy"]);
        assert!(launchers.body.starts_with("Steam, GOG Galaxy."));
        let pattern = nudge_toast(r"youtube\.com/shorts", true);
        assert!(!pattern.body.contains("youtube"));
        assert!(pattern.body.contains("na liście blokowanych"));
    }
}
