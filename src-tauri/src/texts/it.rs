//! Italian shell texts (translated from `en.rs`, checked against `pl.rs`; the UI's are in `src/lib/i18n/it.ts`).
//! "Pausa" is the break phase, so pausing the timer is "Sospendi" / "Sospeso".

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Avvia",
    tray_pause: "Sospendi",
    tray_resume: "Riprendi",
    tray_skip: "Salta fase",
    tray_stop: "Stop",
    tray_show: "Mostra finestra",
    tray_quit: "Esci",
    tray_status_idle: "Pronto per la concentrazione",
    tray_tooltip_idle: "FocusMode — pronto per la concentrazione",
    tray_status_paused: "Sospeso · ",
    tray_tooltip_paused: " (sospeso)",
    tray_cycle: |cycle| format!("ciclo {cycle}"),
    phase_idle: "Pronto",
    phase_focus: "Concentrazione",
    phase_break: "Pausa",

    media_window_title: "FocusMode — musica (YouTube)",
    popup_window_title: "FocusMode — notifica",

    seconds: |secs| format!("{secs} s"),
    minutes: |minutes| format!("{minutes} min"),
    hours: |hours| format!("{hours} h"),
    hours_minutes: |hours, minutes| format!("{hours} h {minutes} min"),

    break_title: "È ora di una pausa ☕",
    break_body: |back_in| {
        format!("Alzati, sgranchisciti, bevi un po' d'acqua. Si riparte tra {back_in}.")
    },
    break_body_reminder: |reminder, back_in| {
        format!("Promemoria: {reminder}\nSi riparte tra {back_in}.")
    },
    focus_title: "È ora di concentrarsi 🎧",
    focus_body: |cycle, focus| format!("Ciclo {cycle}: {focus} di concentrazione."),
    demo_title: "Modalità demo",
    demo_body: "Ferma la sessione per attivare la modalità demo.",
    interrupted_title: "Sessione sospesa",
    interrupted_body: "Il computer era in sospensione — riprendi quando torni.",

    held_title: "Pausa in attesa 📅",
    held_meeting: |subject| format!("Pausa in attesa — riunione in corso: {subject}."),
    held_meeting_untitled: "Pausa in attesa — c'è una riunione in corso.",
    held_resume: |length| format!("Riprendi quando hai finito: la pausa durerà {length}."),
    held_resume_untimed: "Riprendi quando hai finito.",
    held_reminder: |reminder| format!("Promemoria: {reminder}"),

    meeting_title: "Riunione durante una pausa 📅",
    meeting_subject: |subject| format!("“{subject}”"),
    meeting_untitled: "Una riunione",
    meeting_at_in: |clock, minutes| format!("alle {clock} (tra {minutes} min)"),
    meeting_at: |clock| format!("alle {clock}"),
    meeting_in: |minutes| format!("tra {minutes} min"),
    meeting_now: "adesso",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} cade durante la pausa {break_number}. \
             Apri FocusMode e scegli: prolungare la concentrazione, mettere in attesa la pausa o ignorarla."
        )
    },

    launchers_title: "Launcher di gioco chiusi 🎮",
    launchers_body: |names| format!("{names}. Puoi riavviarli quando vuoi."),
    ea_app: "App EA",
    xbox_app: "App Xbox",

    work_digest_title: "Cosa fare dopo? 📋",
    work_digest_body: |count| match plural(Language::It, count) {
        Plural::One => {
            format!("{count} nuovo elemento dai tuoi strumenti ti aspetta in FocusMode.")
        }
        _ => format!("{count} nuovi elementi dai tuoi strumenti ti aspettano in FocusMode."),
    },

    nudge_title: "Torna al lavoro",
    nudge_body: |domain| format!("{domain} può aspettare fino alla pausa."),
    nudge_body_pattern: "Questo sito è nell'elenco dei siti bloccati e può aspettare fino alla pausa.",

    blocklist_create: |detail| {
        format!("Impossibile creare il file con l'elenco predefinito: {detail}")
    },
    blocklist_missing: "Il file non c'è (eliminato o spostato). Aprilo dalle Impostazioni e verrà ricreato con l'elenco predefinito.",
    blocklist_denied: "Nessun accesso al file.",
    blocklist_io: |detail| format!("Errore durante la lettura del file: {detail}"),
    blocklist_too_large: |megabytes| format!("Il file è troppo grande (limite: {megabytes} MB)."),
};
