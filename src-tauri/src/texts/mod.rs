//! Texts shown by the shell itself (toasts, tray menu, window titles), in every UI language.
//!
//! The WebView UI keeps its texts in `src/lib/i18n/`; the shell needs its own copy for toasts and the tray
//! because they must work even when the WebView is hidden or busy. Polish (`pl.rs`) is the source language and
//! every language file fills the same [`Texts`]. The language in use is process-wide: `Config.language` resolved
//! by [`apply_config_language`] at startup and after every config change (`System` = the OS UI language).
//! A toast is built in the language in use when it is created.

mod de;
mod en;
mod es;
mod fr;
mod it;
mod ja;
mod pl;
mod ru;
mod uk;
mod zh;

use std::sync::atomic::{AtomicU8, Ordering};

use focus_core::api::{GameLauncher, Language, Phase, SessionSnapshot};
use focus_core::launchers;

/// Every text of one language: plain strings for fixed texts, functions for texts with values.
/// "FocusMode" is a name and stays as it is in every language.
pub struct Texts {
    // --- Tray menu and tooltip ---
    pub tray_start: &'static str,
    pub tray_pause: &'static str,
    pub tray_resume: &'static str,
    pub tray_skip: &'static str,
    pub tray_stop: &'static str,
    pub tray_show: &'static str,
    pub tray_quit: &'static str,
    /// First (disabled) menu line while no session runs.
    pub tray_status_idle: &'static str,
    /// Tooltip while no session runs, with the app name.
    pub tray_tooltip_idle: &'static str,
    /// Menu status line prefix while paused (`Pauza · `).
    pub tray_status_paused: &'static str,
    /// Tooltip suffix while paused (` (pauza)`).
    pub tray_tooltip_paused: &'static str,
    /// `cykl 2` at the end of the menu status line.
    pub tray_cycle: fn(u32) -> String,
    pub phase_idle: &'static str,
    pub phase_focus: &'static str,
    pub phase_break: &'static str,

    // --- Window titles (seen in Alt+Tab and the taskbar) ---
    /// The separate YouTube window (links that refuse embedding).
    pub media_window_title: &'static str,
    /// The phase popup window (bottom-right corner).
    pub popup_window_title: &'static str,
    /// The YouTube sign-in window (Google's sign-in page, ADR-021).
    pub youtube_sign_in_title: &'static str,

    // --- Durations: `20 s`, `15 min`, `2 h`, `1 h 30 min` ---
    pub seconds: fn(u32) -> String,
    pub minutes: fn(u32) -> String,
    pub hours: fn(u32) -> String,
    pub hours_minutes: fn(u32, u32) -> String,

    // --- Phase toasts ---
    pub break_title: &'static str,
    /// Break toast body without a reminder: (break length).
    pub break_body: fn(&str) -> String,
    /// Break toast body with the user's own reminder: (reminder, break length).
    pub break_body_reminder: fn(&str, &str) -> String,
    pub focus_title: &'static str,
    /// (cycle, focus length)
    pub focus_body: fn(u32, &str) -> String,
    /// Another launch asked for the demo profile ("tryb prezentacji") while a session runs.
    pub demo_title: &'static str,
    pub demo_body: &'static str,
    /// The session was paused after the computer slept.
    pub interrupted_title: &'static str,
    pub interrupted_body: &'static str,

    // --- Break held because a meeting runs ---
    pub held_title: &'static str,
    /// (meeting subject)
    pub held_meeting: fn(&str) -> String,
    pub held_meeting_untitled: &'static str,
    /// (break length)
    pub held_resume: fn(&str) -> String,
    pub held_resume_untimed: &'static str,
    /// The user's own reminder for that break: (reminder).
    pub held_reminder: fn(&str) -> String,

    // --- "Remind me later" for a meeting during a break ---
    pub meeting_title: &'static str,
    /// The subject in the language's quotation marks.
    pub meeting_subject: fn(&str) -> String,
    /// Used instead of the subject when the meeting has none (starts the sentence).
    pub meeting_untitled: &'static str,
    /// (local start `HH:MM`, minutes until it starts)
    pub meeting_at_in: fn(&str, i64) -> String,
    pub meeting_at: fn(&str) -> String,
    pub meeting_in: fn(i64) -> String,
    pub meeting_now: &'static str,
    /// (subject, when, break number)
    pub meeting_body: fn(&str, &str, u32) -> String,

    // --- Game launchers closed when focus started ---
    pub launchers_title: &'static str,
    /// (comma-separated launcher names)
    pub launchers_body: fn(&str) -> String,
    /// Launcher names that are translated (the others are brand names).
    pub ea_app: &'static str,
    pub xbox_app: &'static str,

    // --- New work items for "Co dalej?" ---
    pub work_digest_title: &'static str,
    /// (count) Never names an item: titles stay in the app.
    pub work_digest_body: fn(u32) -> String,

    // --- A blocked site in the foreground browser window ---
    pub nudge_title: &'static str,
    /// (domain)
    pub nudge_body: fn(&str) -> String,
    /// The match was a regex from websiteblacklist.txt, not a readable domain.
    pub nudge_body_pattern: &'static str,

    // --- websiteblacklist.txt cannot be read (Settings, after the UI's "the previous list stays") ---
    /// The bundled default could not be written: (English error detail).
    pub blocklist_create: fn(&str) -> String,
    pub blocklist_missing: &'static str,
    pub blocklist_denied: &'static str,
    /// (English error detail)
    pub blocklist_io: fn(&str) -> String,
    /// (size limit in MB)
    pub blocklist_too_large: fn(u64) -> String,

    // --- Developer mode samples (ADR-019): sample popups and their fallback toasts ---
    /// Subject of the sample meeting (a meeting reminder or a held break).
    pub dev_sample_meeting: &'static str,
    /// The sample break reminder, used when the user has none for that break.
    pub dev_sample_reminder: &'static str,
    /// The sample next step ("Teraz: …") of a focus popup sample while small steps are on (beta, ADR-022).
    pub dev_sample_next_step: &'static str,
}

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

// ---------------------------------------------------------------------------
// Language in use
// ---------------------------------------------------------------------------

/// Index in [`Language::SUPPORTED`] of the language in use (0 = English until the config is read).
static LANGUAGE: AtomicU8 = AtomicU8::new(0);

/// The texts of a concrete language (`System` gets English; resolve it first with [`resolve`]).
pub fn for_language(language: Language) -> &'static Texts {
    match language {
        Language::Pl => &pl::TEXTS,
        Language::En | Language::System => &en::TEXTS,
        Language::De => &de::TEXTS,
        Language::Fr => &fr::TEXTS,
        Language::It => &it::TEXTS,
        Language::Uk => &uk::TEXTS,
        Language::Ru => &ru::TEXTS,
        Language::Es => &es::TEXTS,
        Language::Zh => &zh::TEXTS,
        Language::Ja => &ja::TEXTS,
    }
}

/// The language in use (never `System`).
pub fn language() -> Language {
    let index = usize::from(LANGUAGE.load(Ordering::Relaxed));
    Language::SUPPORTED
        .get(index)
        .copied()
        .unwrap_or(Language::En)
}

/// The texts of the language in use.
pub fn current() -> &'static Texts {
    for_language(language())
}

/// Sets the language in use (`System` counts as English). Returns true when it changed.
pub fn set_language(language: Language) -> bool {
    let index = Language::SUPPORTED
        .iter()
        .position(|supported| *supported == language)
        .unwrap_or(0);
    let index = u8::try_from(index).unwrap_or(0);
    LANGUAGE.swap(index, Ordering::Relaxed) != index
}

/// The supported language closest to the OS UI language; `System` when it is not supported.
pub fn system_language() -> Language {
    focus_platform::locale::system_language_tag()
        .and_then(|tag| Language::from_tag(&tag))
        .unwrap_or(Language::System)
}

/// `Config.language` as shown: `System` becomes the OS UI language (English when it is not supported).
pub fn resolve(choice: Language) -> Language {
    match choice {
        Language::System => Language::System.resolve(system_language()),
        chosen => chosen,
    }
}

/// Resolves and applies `Config.language`. Returns true when the language in use changed.
pub fn apply_config_language(choice: Language) -> bool {
    set_language(resolve(choice))
}

// ---------------------------------------------------------------------------
// Toasts and tray texts (built from the language's parts)
// ---------------------------------------------------------------------------

impl Texts {
    /// Human duration: `20 s`, `15 min`, `1 h 30 min` (rounded to the nearest minute from one minute up).
    pub fn duration(&self, secs: u32) -> String {
        if secs < 60 {
            return (self.seconds)(secs);
        }
        // Round to the nearest minute so 44:59 reads as 45 min.
        let minutes = secs.saturating_add(30) / 60;
        match (minutes / 60, minutes % 60) {
            (0, m) => (self.minutes)(m),
            (h, 0) => (self.hours)(h),
            (h, m) => (self.hours_minutes)(h, m),
        }
    }

    pub fn phase_label(&self, phase: Phase) -> &'static str {
        match phase {
            Phase::Idle => self.phase_idle,
            Phase::Focus => self.phase_focus,
            Phase::Break => self.phase_break,
        }
    }

    /// Tray tooltip, e.g. `FocusMode — Skupienie 12:34`.
    pub fn tray_tooltip(&self, snapshot: &SessionSnapshot) -> String {
        match snapshot.phase {
            Phase::Idle => self.tray_tooltip_idle.to_owned(),
            phase => {
                let paused = if snapshot.paused {
                    self.tray_tooltip_paused
                } else {
                    ""
                };
                format!(
                    "FocusMode — {} {}{paused}",
                    self.phase_label(phase),
                    clock(snapshot.remaining_secs)
                )
            }
        }
    }

    /// First (disabled) line of the tray menu, e.g. `Skupienie · 12:34 · cykl 2`.
    pub fn tray_status(&self, snapshot: &SessionSnapshot) -> String {
        match snapshot.phase {
            Phase::Idle => self.tray_status_idle.to_owned(),
            phase => {
                let prefix = if snapshot.paused {
                    self.tray_status_paused
                } else {
                    ""
                };
                format!(
                    "{prefix}{} · {} · {}",
                    self.phase_label(phase),
                    clock(snapshot.remaining_secs),
                    (self.tray_cycle)(snapshot.cycle)
                )
            }
        }
    }

    /// Focus ended, break begins. `reminder` (the user's own text for this break, `Config::break_reminders`)
    /// replaces the generic suggestion when present.
    pub fn break_toast(&self, break_secs: u32, reminder: Option<&str>) -> Toast {
        let back_in = self.duration(break_secs);
        let body = match non_empty(reminder) {
            Some(reminder) => (self.break_body_reminder)(reminder, &back_in),
            None => (self.break_body)(&back_in),
        };
        Toast::new(self.break_title, body)
    }

    /// Break ended, next focus cycle begins.
    pub fn focus_toast(&self, focus_secs: u32, cycle: u32) -> Toast {
        Toast::new(
            self.focus_title,
            (self.focus_body)(cycle, &self.duration(focus_secs)),
        )
    }

    /// Another launch asked for the demo profile (`--demo`) while a session runs.
    pub fn demo_needs_stop_toast(&self) -> Toast {
        Toast::new(self.demo_title, self.demo_body)
    }

    /// The session was auto-paused after a sleep/clock gap.
    pub fn interrupted_toast(&self) -> Toast {
        Toast::new(self.interrupted_title, self.interrupted_body)
    }

    /// The break started paused because a meeting runs (`MeetingChoice::PauseAtBreak`). `subject` is the
    /// meeting subject (`None` when empty), `reminder` the user's own text for this break.
    pub fn break_held_toast(
        &self,
        subject: Option<&str>,
        break_secs: u32,
        reminder: Option<&str>,
    ) -> Toast {
        let meeting = match subject.map(short_subject).filter(|text| !text.is_empty()) {
            Some(subject) => (self.held_meeting)(&subject),
            None => self.held_meeting_untitled.to_owned(),
        };
        let resume = if break_secs > 0 {
            (self.held_resume)(&self.duration(break_secs))
        } else {
            self.held_resume_untimed.to_owned()
        };
        let body = match non_empty(reminder) {
            Some(reminder) => format!("{meeting}\n{resume}\n{}", (self.held_reminder)(reminder)),
            None => format!("{meeting}\n{resume}"),
        };
        Toast::new(self.held_title, body)
    }

    /// "Remind later" is due for a meeting that collides with a break (fallback when the popup window cannot
    /// be shown). `start_local` is the meeting start as local `HH:MM`, `minutes_until` how soon it starts.
    pub fn meeting_reminder_toast(
        &self,
        subject: &str,
        start_local: Option<&str>,
        minutes_until: i64,
        break_number: u32,
    ) -> Toast {
        let subject = short_subject(subject);
        let subject = if subject.is_empty() {
            self.meeting_untitled.to_owned()
        } else {
            (self.meeting_subject)(&subject)
        };
        let when = match (start_local, minutes_until) {
            (Some(clock), minutes) if minutes > 0 => (self.meeting_at_in)(clock, minutes),
            (Some(clock), _) => (self.meeting_at)(clock),
            (None, minutes) if minutes > 0 => (self.meeting_in)(minutes),
            (None, _) => self.meeting_now.to_owned(),
        };
        Toast::new(
            self.meeting_title,
            (self.meeting_body)(&subject, &when, break_number),
        )
    }

    /// The name of a game launcher in this language.
    pub fn launcher_name(&self, launcher: GameLauncher) -> &'static str {
        match launcher {
            GameLauncher::Ea => self.ea_app,
            GameLauncher::Xbox => self.xbox_app,
            other => launchers::display_name(other),
        }
    }

    /// Game launchers were closed when focus started.
    pub fn launchers_closed_toast(&self, closed: &[GameLauncher]) -> Toast {
        let names: Vec<&str> = closed
            .iter()
            .map(|launcher| self.launcher_name(*launcher))
            .collect();
        Toast::new(
            self.launchers_title,
            (self.launchers_body)(&names.join(", ")),
        )
    }

    /// New work items for "Co dalej?" (held back during focus, shown at the break or after the session).
    /// Fallback when the popup window cannot be shown; never names an item (titles stay in the app).
    pub fn work_digest_toast(&self, count: u32) -> Toast {
        Toast::new(self.work_digest_title, (self.work_digest_body)(count))
    }

    /// A blocked site was detected in the foreground browser window. `matched` is a domain, or a regex
    /// source when `is_pattern` (not readable as a site name, so it is not shown).
    pub fn nudge_toast(&self, matched: &str, is_pattern: bool) -> Toast {
        let body = if is_pattern {
            self.nudge_body_pattern.to_owned()
        } else {
            (self.nudge_body)(matched)
        };
        Toast::new(self.nudge_title, body)
    }
}

// Shortcuts in the language in use, for the call sites that build one toast.

pub fn break_toast(break_secs: u32, reminder: Option<&str>) -> Toast {
    current().break_toast(break_secs, reminder)
}

pub fn focus_toast(focus_secs: u32, cycle: u32) -> Toast {
    current().focus_toast(focus_secs, cycle)
}

pub fn demo_needs_stop_toast() -> Toast {
    current().demo_needs_stop_toast()
}

pub fn interrupted_toast() -> Toast {
    current().interrupted_toast()
}

/// The sample next step of the developer page's focus popup sample (beta small steps, ADR-022).
pub fn sample_next_step() -> &'static str {
    current().dev_sample_next_step
}

pub fn break_held_toast(subject: Option<&str>, break_secs: u32, reminder: Option<&str>) -> Toast {
    current().break_held_toast(subject, break_secs, reminder)
}

pub fn meeting_reminder_toast(
    subject: &str,
    start_local: Option<&str>,
    minutes_until: i64,
    break_number: u32,
) -> Toast {
    current().meeting_reminder_toast(subject, start_local, minutes_until, break_number)
}

pub fn launchers_closed_toast(closed: &[GameLauncher]) -> Toast {
    current().launchers_closed_toast(closed)
}

pub fn work_digest_toast(count: u32) -> Toast {
    current().work_digest_toast(count)
}

pub fn nudge_toast(matched: &str, is_pattern: bool) -> Toast {
    current().nudge_toast(matched, is_pattern)
}

// ---------------------------------------------------------------------------
// Plural forms (for the language files)
// ---------------------------------------------------------------------------

/// CLDR plural category of a whole number (the categories `Intl.PluralRules` gives the UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Plural {
    One,
    Few,
    Many,
    Other,
}

/// The plural category of `n` in `language`, as `Intl.PluralRules` decides it for whole numbers:
/// pl: one (1), few (2–4, 22–24…, not 12–14), many · uk/ru: one (1, 21, 31…, not 11), few, many ·
/// fr: one (0, 1), many (millions), other · it/es: one (1), many (millions), other · en/de: one, other ·
/// zh/ja: other.
pub(super) fn plural(language: Language, n: u32) -> Plural {
    let (last, last_two) = (n % 10, n % 100);
    let few = (2..=4).contains(&last) && !(12..=14).contains(&last_two);
    let millions = n != 0 && n.is_multiple_of(1_000_000);
    match language {
        Language::Pl if n == 1 => Plural::One,
        Language::Uk | Language::Ru if last == 1 && last_two != 11 => Plural::One,
        Language::Pl | Language::Uk | Language::Ru if few => Plural::Few,
        Language::Pl | Language::Uk | Language::Ru => Plural::Many,
        Language::Fr if n <= 1 => Plural::One,
        Language::Fr | Language::It | Language::Es if millions => Plural::Many,
        Language::Zh | Language::Ja => Plural::Other,
        _ if n == 1 => Plural::One,
        _ => Plural::Other,
    }
}

// ---------------------------------------------------------------------------
// Language-independent helpers
// ---------------------------------------------------------------------------

/// The user's text trimmed, or `None` when empty.
fn non_empty(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    const PL: &Texts = &pl::TEXTS;
    const EN: &Texts = &en::TEXTS;

    fn snapshot(phase: Phase, remaining_secs: u32, cycle: u32, paused: bool) -> SessionSnapshot {
        SessionSnapshot {
            revision: 0,
            session_id: None,
            phase,
            paused,
            cycle,
            remaining_secs,
            phase_total_secs: 0,
            focus_secs: 0,
            break_secs: 0,
            focused_secs: 0,
            demo: false,
            music_muted: false,
            meeting_hold: None,
            goal: None,
            steps: Vec::new(),
            smart_break: None,
        }
    }

    fn all() -> impl Iterator<Item = (Language, &'static Texts)> {
        Language::SUPPORTED
            .into_iter()
            .map(|language| (language, for_language(language)))
    }

    #[test]
    fn durations_read_naturally() {
        assert_eq!(PL.duration(20), "20 s");
        assert_eq!(PL.duration(15 * 60), "15 min");
        assert_eq!(PL.duration(45 * 60 - 1), "45 min");
        assert_eq!(PL.duration(90 * 60), "1 h 30 min");
        assert_eq!(PL.duration(120 * 60), "2 h");
        assert_eq!(EN.duration(90 * 60), "1 h 30 min");
    }

    #[test]
    fn clock_formats() {
        assert_eq!(clock(0), "00:00");
        assert_eq!(clock(754), "12:34");
        assert_eq!(clock(3723), "1:02:03");
    }

    #[test]
    fn break_toast_shows_the_reminder() {
        let plain = PL.break_toast(15 * 60, None);
        assert!(plain.body.contains("Wstań"));
        let reminder = PL.break_toast(15 * 60, Some("  obiad  "));
        assert!(reminder.body.starts_with("Przypomnienie: obiad\n"));
        assert!(reminder.body.contains("15 min"));
        assert!(!reminder.body.contains("Wstań"));
        assert_eq!(PL.break_toast(60, Some("   ")), PL.break_toast(60, None));
        let english = EN.break_toast(15 * 60, Some(" lunch "));
        assert!(english.body.starts_with("Reminder: lunch\n"), "{english:?}");
        assert!(english.body.contains("15 min"));
    }

    #[test]
    fn held_break_toast_names_the_meeting() {
        let toast = PL.break_held_toast(Some("  Daily   standup "), 15 * 60, None);
        assert!(
            toast
                .body
                .starts_with("Przerwa wstrzymana — trwa spotkanie: Daily standup.")
        );
        assert!(toast.body.contains("15 min"));
        assert!(!toast.body.contains("Przypomnienie"));
        let anonymous = PL.break_held_toast(None, 0, Some(" obiad "));
        assert!(
            anonymous
                .body
                .starts_with("Przerwa wstrzymana — trwa spotkanie.\n")
        );
        assert!(anonymous.body.ends_with("Przypomnienie: obiad"));
        assert_eq!(
            PL.break_held_toast(Some("   "), 60, None).body,
            PL.break_held_toast(None, 60, None).body
        );
    }

    #[test]
    fn meeting_reminder_toast_says_what_and_when() {
        let toast = PL.meeting_reminder_toast("Przegląd sprintu", Some("14:30"), 30, 2);
        assert!(
            toast
                .body
                .starts_with("„Przegląd sprintu” o 14:30 (za 30 min) wypada w przerwie 2.")
        );
        let long = "x".repeat(200);
        let toast = PL.meeting_reminder_toast(&long, None, 0, 1);
        assert!(toast.body.contains('…'));
        assert!(toast.body.contains("teraz"));
        assert!(!toast.body.contains(&"x".repeat(81)));
        assert!(
            PL.meeting_reminder_toast(" ", None, 5, 1)
                .body
                .starts_with("Spotkanie za 5 min")
        );
        let english = EN.meeting_reminder_toast("Sprint review", Some("14:30"), 30, 2);
        assert!(
            english
                .body
                .starts_with("“Sprint review” at 14:30 (in 30 min) falls during break 2."),
            "{english:?}"
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
    fn work_digest_toast_uses_plurals() {
        assert_eq!(
            PL.work_digest_toast(1).body,
            "1 nowa pozycja z Twoich narzędzi czeka w FocusMode."
        );
        assert_eq!(
            PL.work_digest_toast(3).body,
            "3 nowe pozycje z Twoich narzędzi czekają w FocusMode."
        );
        assert_eq!(
            PL.work_digest_toast(5).body,
            "5 nowych pozycji z Twoich narzędzi czeka w FocusMode."
        );
        assert!(
            PL.work_digest_toast(12)
                .body
                .starts_with("12 nowych pozycji")
        );
        assert!(PL.work_digest_toast(22).body.starts_with("22 nowe pozycje"));
        assert!(PL.work_digest_toast(0).body.starts_with("0 nowych pozycji"));
        assert!(PL.work_digest_toast(114).body.starts_with("114 nowych"));
        assert!(PL.work_digest_toast(104).body.starts_with("104 nowe"));
        assert!(EN.work_digest_toast(1).body.starts_with("1 new item "));
        assert!(EN.work_digest_toast(2).body.starts_with("2 new items "));
    }

    #[test]
    fn toasts_contain_numbers() {
        for (language, texts) in all() {
            assert!(
                texts.break_toast(15 * 60, None).body.contains("15"),
                "{language:?}"
            );
            assert!(
                texts.focus_toast(45 * 60, 2).body.contains('2'),
                "{language:?}"
            );
            assert!(
                texts
                    .nudge_toast("reddit.com", false)
                    .body
                    .contains("reddit.com"),
                "{language:?}"
            );
            let launchers = texts.launchers_closed_toast(&[GameLauncher::Steam, GameLauncher::Gog]);
            assert!(launchers.body.contains("Steam, GOG Galaxy"), "{language:?}");
            let pattern = texts.nudge_toast(r"youtube\.com/shorts", true);
            assert!(!pattern.body.contains("youtube"), "{language:?}");
            assert!(
                texts.work_digest_toast(7).body.contains('7'),
                "{language:?}"
            );
        }
        assert!(PL.focus_toast(45 * 60, 2).body.contains("Cykl 2"));
        assert!(
            PL.nudge_toast("x", true)
                .body
                .contains("na liście blokowanych")
        );
    }

    #[test]
    fn every_language_fills_every_text() {
        let snapshot = snapshot(Phase::Focus, 754, 3, true);
        for (language, texts) in all() {
            for text in [
                texts.tray_start,
                texts.tray_pause,
                texts.tray_resume,
                texts.tray_skip,
                texts.tray_stop,
                texts.tray_show,
                texts.tray_quit,
                texts.tray_status_idle,
                texts.tray_tooltip_idle,
                texts.phase_idle,
                texts.phase_focus,
                texts.phase_break,
                texts.media_window_title,
                texts.popup_window_title,
                texts.youtube_sign_in_title,
                texts.ea_app,
                texts.xbox_app,
                texts.blocklist_missing,
                texts.blocklist_denied,
                texts.dev_sample_meeting,
                texts.dev_sample_reminder,
                texts.dev_sample_next_step,
            ] {
                assert!(!text.trim().is_empty(), "{language:?}");
            }
            assert!(
                texts.tray_tooltip_idle.contains("FocusMode"),
                "{language:?}"
            );
            assert!(
                texts.media_window_title.contains("FocusMode"),
                "{language:?}"
            );
            assert!(
                texts.popup_window_title.contains("FocusMode"),
                "{language:?}"
            );
            assert!(
                texts.youtube_sign_in_title.contains("FocusMode")
                    && texts.youtube_sign_in_title.contains("YouTube"),
                "{language:?}"
            );
            let status = texts.tray_status(&snapshot);
            assert!(status.contains("12:34") && status.contains('3'), "{status}");
            let tooltip = texts.tray_tooltip(&snapshot);
            assert!(tooltip.starts_with("FocusMode — ") && tooltip.contains("12:34"));
            for toast in [
                texts.demo_needs_stop_toast(),
                texts.interrupted_toast(),
                texts.break_held_toast(Some("Planning"), 900, Some("water")),
                texts.meeting_reminder_toast("Planning", Some("09:00"), 5, 1),
            ] {
                assert!(!toast.title.trim().is_empty() && !toast.body.trim().is_empty());
            }
        }
    }

    #[test]
    fn tray_texts_follow_the_language() {
        let snapshot = snapshot(Phase::Break, 60, 2, false);
        assert_eq!(PL.tray_status(&snapshot), "Przerwa · 01:00 · cykl 2");
        assert_eq!(EN.tray_status(&snapshot), "Break · 01:00 · cycle 2");
        assert_eq!(EN.tray_tooltip(&snapshot), "FocusMode — Break 01:00");
        assert_eq!(PL.launcher_name(GameLauncher::Ea), "Aplikacja EA");
        assert_eq!(EN.launcher_name(GameLauncher::Ea), "EA app");
        assert_eq!(EN.launcher_name(GameLauncher::Steam), "Steam");
    }

    #[test]
    fn plural_categories_match_cldr() {
        use Plural::{Few, Many, One, Other};
        for (n, expected) in [
            (0, Many),
            (1, One),
            (2, Few),
            (4, Few),
            (5, Many),
            (12, Many),
            (22, Few),
            (104, Few),
            (114, Many),
        ] {
            assert_eq!(plural(Language::Pl, n), expected, "pl {n}");
        }
        for (n, expected) in [
            (0, Many),
            (1, One),
            (2, Few),
            (11, Many),
            (21, One),
            (22, Few),
            (25, Many),
            (111, Many),
        ] {
            assert_eq!(plural(Language::Ru, n), expected, "ru {n}");
            assert_eq!(plural(Language::Uk, n), expected, "uk {n}");
        }
        for (n, expected) in [(0, One), (1, One), (2, Other), (1_000_000, Many)] {
            assert_eq!(plural(Language::Fr, n), expected, "fr {n}");
        }
        for language in [Language::En, Language::De, Language::It, Language::Es] {
            assert_eq!(plural(language, 1), One);
            assert_eq!(plural(language, 0), Other);
            assert_eq!(plural(language, 21), Other);
        }
        assert_eq!(plural(Language::Es, 2_000_000), Many);
        assert_eq!(plural(Language::Ja, 1), Other);
        assert_eq!(plural(Language::Zh, 3), Other);
    }

    #[test]
    fn languages_map_to_their_texts() {
        assert!(std::ptr::eq(for_language(Language::Pl), PL));
        assert!(std::ptr::eq(for_language(Language::En), EN));
        assert!(std::ptr::eq(for_language(Language::System), EN));
        assert_eq!(resolve(Language::De), Language::De);
        assert_ne!(resolve(Language::System), Language::System);
    }
}
