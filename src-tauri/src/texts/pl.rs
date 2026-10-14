//! Polish shell texts: the source language (the UI's are in `src/lib/i18n/pl.ts`).

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Start",
    tray_pause: "Pauza",
    tray_resume: "Wznów",
    tray_skip: "Pomiń fazę",
    tray_stop: "Stop",
    tray_show: "Pokaż okno",
    tray_quit: "Zakończ",
    tray_status_idle: "Gotowy do skupienia",
    tray_tooltip_idle: "FocusMode — gotowy do skupienia",
    tray_status_paused: "Pauza · ",
    tray_tooltip_paused: " (pauza)",
    tray_cycle: |cycle| format!("cykl {cycle}"),
    phase_idle: "Gotowy",
    phase_focus: "Skupienie",
    phase_break: "Przerwa",

    media_window_title: "FocusMode — muzyka (YouTube)",
    popup_window_title: "FocusMode — powiadomienie",
    youtube_sign_in_title: "FocusMode — logowanie do YouTube",

    seconds: |secs| format!("{secs} s"),
    minutes: |minutes| format!("{minutes} min"),
    hours: |hours| format!("{hours} h"),
    hours_minutes: |hours, minutes| format!("{hours} h {minutes} min"),

    break_title: "Czas na przerwę ☕",
    break_body: |back_in| format!("Wstań, rozciągnij się, napij się wody. Wracamy za {back_in}."),
    break_body_reminder: |reminder, back_in| {
        format!("Przypomnienie: {reminder}\nWracamy za {back_in}.")
    },
    focus_title: "Wracamy do pracy 🎧",
    focus_body: |cycle, focus| format!("Cykl {cycle}: {focus} skupienia."),
    demo_title: "Tryb prezentacji",
    demo_body: "Zatrzymaj sesję, aby włączyć tryb prezentacji.",
    interrupted_title: "Sesja wstrzymana",
    interrupted_body: "Wykryto uśpienie komputera — wznów, kiedy wrócisz.",

    held_title: "Przerwa wstrzymana 📅",
    held_meeting: |subject| format!("Przerwa wstrzymana — trwa spotkanie: {subject}."),
    held_meeting_untitled: "Przerwa wstrzymana — trwa spotkanie.",
    held_resume: |length| format!("Wznów, kiedy skończysz: przerwa potrwa {length}."),
    held_resume_untimed: "Wznów, kiedy skończysz.",
    held_reminder: |reminder| format!("Przypomnienie: {reminder}"),

    meeting_title: "Spotkanie w czasie przerwy 📅",
    meeting_subject: |subject| format!("„{subject}”"),
    meeting_untitled: "Spotkanie",
    meeting_at_in: |clock, minutes| format!("o {clock} (za {minutes} min)"),
    meeting_at: |clock| format!("o {clock}"),
    meeting_in: |minutes| format!("za {minutes} min"),
    meeting_now: "teraz",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} wypada w przerwie {break_number}. \
             Otwórz FocusMode i wybierz: przedłużyć skupienie, wstrzymać przerwę czy zignorować."
        )
    },

    launchers_title: "Zamknięto launchery gier 🎮",
    launchers_body: |names| format!("{names}. Uruchomisz je ponownie, kiedy zechcesz."),
    ea_app: "Aplikacja EA",
    xbox_app: "Aplikacja Xbox",

    work_digest_title: "Co dalej? 📋",
    work_digest_body: |count| {
        let (items, waits) = match plural(Language::Pl, count) {
            Plural::One => ("nowa pozycja", "czeka"),
            Plural::Few => ("nowe pozycje", "czekają"),
            Plural::Many | Plural::Other => ("nowych pozycji", "czeka"),
        };
        format!("{count} {items} z Twoich narzędzi {waits} w FocusMode.")
    },

    nudge_title: "Wracaj do pracy",
    nudge_body: |domain| format!("{domain} poczeka do przerwy."),
    nudge_body_pattern: "Ta strona jest na liście blokowanych i poczeka do przerwy.",

    blocklist_create: |detail| format!("Nie można utworzyć pliku z domyślną listą: {detail}"),
    blocklist_missing: "Pliku nie ma (usunięty albo przeniesiony). Otwórz go z Ustawień, a powstanie od nowa z domyślną listą.",
    blocklist_denied: "Brak dostępu do pliku.",
    blocklist_io: |detail| format!("Błąd odczytu pliku: {detail}"),
    blocklist_too_large: |megabytes| format!("Plik jest za duży (limit {megabytes} MB)."),

    dev_sample_meeting: "Planowanie sprintu",
    dev_sample_reminder: "Woda i krótki spacer",
    dev_sample_next_step: "Napisz wstęp",
};
