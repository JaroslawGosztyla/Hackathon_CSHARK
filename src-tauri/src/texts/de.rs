//! German shell texts (translated from `pl.rs` via `en.rs`; the UI's are in `src/lib/i18n/de.ts`).
//! The user is addressed with "du"; terms in docs/i18n-glossary.md (column DE).

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Start",
    tray_pause: "Anhalten",
    tray_resume: "Fortsetzen",
    tray_skip: "Phase überspringen",
    tray_stop: "Stopp",
    tray_show: "Fenster anzeigen",
    tray_quit: "Beenden",
    tray_status_idle: "Bereit für den Fokus",
    tray_tooltip_idle: "FocusMode — bereit für den Fokus",
    tray_status_paused: "Angehalten · ",
    tray_tooltip_paused: " (angehalten)",
    tray_cycle: |cycle| format!("Zyklus {cycle}"),
    phase_idle: "Bereit",
    phase_focus: "Fokus",
    phase_break: "Pause",

    media_window_title: "FocusMode — Musik (YouTube)",
    popup_window_title: "FocusMode — Benachrichtigung",
    youtube_sign_in_title: "FocusMode — bei YouTube anmelden",

    seconds: |secs| format!("{secs} s"),
    minutes: |minutes| format!("{minutes} min"),
    hours: |hours| format!("{hours} h"),
    hours_minutes: |hours, minutes| format!("{hours} h {minutes} min"),

    break_title: "Zeit für eine Pause ☕",
    break_body: |back_in| {
        format!("Steh auf, streck dich, trink einen Schluck Wasser. Weiter geht's in {back_in}.")
    },
    break_body_reminder: |reminder, back_in| {
        format!("Erinnerung: {reminder}\nWeiter geht's in {back_in}.")
    },
    focus_title: "Zeit für Fokus 🎧",
    focus_body: |cycle, focus| format!("Zyklus {cycle}: {focus} Fokus."),
    demo_title: "Demomodus",
    demo_body: "Beende die Sitzung mit Stopp, um den Demomodus einzuschalten.",
    interrupted_title: "Sitzung angehalten",
    interrupted_body: "Der Computer war im Energiesparmodus — setz die Sitzung fort, wenn du zurück bist.",

    held_title: "Pause angehalten 📅",
    held_meeting: |subject| format!("Pause angehalten — eine Besprechung läuft: {subject}."),
    held_meeting_untitled: "Pause angehalten — eine Besprechung läuft.",
    held_resume: |length| format!("Setz sie fort, wenn du fertig bist: Die Pause dauert {length}."),
    held_resume_untimed: "Setz sie fort, wenn du fertig bist.",
    held_reminder: |reminder| format!("Erinnerung: {reminder}"),

    meeting_title: "Besprechung während einer Pause 📅",
    meeting_subject: |subject| format!("„{subject}“"),
    meeting_untitled: "Eine Besprechung",
    meeting_at_in: |clock, minutes| format!("beginnt um {clock} (in {minutes} min)"),
    meeting_at: |clock| format!("beginnt um {clock}"),
    meeting_in: |minutes| format!("beginnt in {minutes} min"),
    meeting_now: "beginnt jetzt",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} und fällt in Pause {break_number}. \
             Öffne FocusMode und wähle: Fokus verlängern, Pause anhalten oder ignorieren."
        )
    },

    launchers_title: "Spiele-Launcher geschlossen 🎮",
    launchers_body: |names| format!("{names}. Du kannst sie jederzeit wieder starten."),
    ea_app: "EA app",
    xbox_app: "Xbox-App",

    work_digest_title: "Was steht an? 📋",
    work_digest_body: |count| match plural(Language::De, count) {
        Plural::One => format!("{count} neuer Eintrag aus deinen Tools wartet in FocusMode."),
        _ => format!("{count} neue Einträge aus deinen Tools warten in FocusMode."),
    },

    nudge_title: "Zurück an die Arbeit",
    nudge_body: |domain| format!("{domain} wartet bis zur Pause."),
    nudge_body_pattern: "Diese Seite steht auf der Sperrliste und wartet bis zur Pause.",

    blocklist_create: |detail| {
        format!("Die Datei mit der Standardliste kann nicht erstellt werden: {detail}")
    },
    blocklist_missing: "Die Datei fehlt (gelöscht oder verschoben). Öffne sie in den Einstellungen, dann wird sie mit der Standardliste neu angelegt.",
    blocklist_denied: "Kein Zugriff auf die Datei.",
    blocklist_io: |detail| format!("Fehler beim Lesen der Datei: {detail}"),
    blocklist_too_large: |megabytes| format!("Die Datei ist zu groß (Limit: {megabytes} MB)."),

    dev_sample_meeting: "Sprint-Planung",
    dev_sample_reminder: "Wasser und ein kurzer Spaziergang",
    dev_sample_next_step: "Die Einleitung schreiben",
};
