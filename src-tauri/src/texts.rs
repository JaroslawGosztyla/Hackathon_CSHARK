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

/// Focus ended, break begins.
pub fn break_toast(break_secs: u32) -> Toast {
    Toast::new(
        "Czas na przerwę ☕",
        format!(
            "Wstań, rozciągnij się, napij się wody. Wracamy za {}.",
            duration_pl(break_secs)
        ),
    )
}

/// Break ended, next focus cycle begins.
pub fn focus_toast(focus_secs: u32, cycle: u32) -> Toast {
    Toast::new(
        "Wracamy do pracy 🎧",
        format!("Cykl {cycle}: {} skupienia.", duration_pl(focus_secs)),
    )
}

/// The session was auto-paused after a sleep/clock gap.
pub fn interrupted_toast() -> Toast {
    Toast::new(
        "Sesja wstrzymana",
        "Wykryto uśpienie komputera — wznów, kiedy wrócisz.",
    )
}

/// A blocked site was detected in the foreground browser window.
pub fn nudge_toast(domain: &str) -> Toast {
    Toast::new("Wracaj do pracy", format!("{domain} poczeka do przerwy."))
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
    fn toasts_contain_numbers() {
        assert!(break_toast(15 * 60).body.contains("15 min"));
        assert!(focus_toast(45 * 60, 2).body.contains("Cykl 2"));
        assert!(nudge_toast("reddit.com").body.contains("reddit.com"));
    }
}
