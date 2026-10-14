//! French shell texts (translated from `pl.rs` via `en.rs`; the UI's are in `src/lib/i18n/fr.ts`).
//! The user is addressed with "vous"; a no-break space goes before : ; ? ! and units, « » are the quotes.
//! Terms in docs/i18n-glossary.md (column FR).

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Démarrer",
    tray_pause: "Suspendre",
    tray_resume: "Reprendre",
    tray_skip: "Passer la phase",
    tray_stop: "Arrêter",
    tray_show: "Afficher la fenêtre",
    tray_quit: "Quitter",
    tray_status_idle: "Prêt pour la concentration",
    tray_tooltip_idle: "FocusMode — prêt pour la concentration",
    tray_status_paused: "Suspendu · ",
    tray_tooltip_paused: " (suspendu)",
    tray_cycle: |cycle| format!("cycle {cycle}"),
    phase_idle: "Prêt",
    phase_focus: "Concentration",
    phase_break: "Pause",

    media_window_title: "FocusMode — musique (YouTube)",
    popup_window_title: "FocusMode — notification",
    youtube_sign_in_title: "FocusMode — connexion à YouTube",

    seconds: |secs| format!("{secs} s"),
    minutes: |minutes| format!("{minutes} min"),
    hours: |hours| format!("{hours} h"),
    hours_minutes: |hours, minutes| format!("{hours} h {minutes} min"),

    break_title: "C'est l'heure de la pause ☕",
    break_body: |back_in| {
        format!("Levez-vous, étirez-vous, buvez un peu d'eau. Reprise dans {back_in}.")
    },
    break_body_reminder: |reminder, back_in| {
        format!("Rappel : {reminder}\nReprise dans {back_in}.")
    },
    focus_title: "Place à la concentration 🎧",
    focus_body: |cycle, focus| format!("Cycle {cycle} : {focus} de concentration."),
    demo_title: "Mode démo",
    demo_body: "Arrêtez la session pour activer le mode démo.",
    interrupted_title: "Session suspendue",
    interrupted_body: "L'ordinateur était en veille — reprenez quand vous serez de retour.",

    held_title: "Pause suspendue 📅",
    held_meeting: |subject| format!("Pause suspendue — réunion en cours : {subject}."),
    held_meeting_untitled: "Pause suspendue — une réunion est en cours.",
    held_resume: |length| format!("Reprenez quand vous aurez fini : la pause durera {length}."),
    held_resume_untimed: "Reprenez quand vous aurez fini.",
    held_reminder: |reminder| format!("Rappel : {reminder}"),

    meeting_title: "Réunion pendant une pause 📅",
    meeting_subject: |subject| format!("« {subject} »"),
    meeting_untitled: "Une réunion",
    meeting_at_in: |clock, minutes| format!("commence à {clock} (dans {minutes} min)"),
    meeting_at: |clock| format!("commence à {clock}"),
    meeting_in: |minutes| format!("commence dans {minutes} min"),
    meeting_now: "commence maintenant",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} et tombe pendant la pause {break_number}. \
             Ouvrez FocusMode et choisissez : prolonger la concentration, suspendre la pause ou ignorer."
        )
    },

    launchers_title: "Lanceurs de jeux fermés 🎮",
    launchers_body: |names| format!("{names}. Vous pouvez les relancer quand vous voulez."),
    ea_app: "Application EA",
    xbox_app: "Application Xbox",

    work_digest_title: "Et ensuite ? 📋",
    work_digest_body: |count| match plural(Language::Fr, count) {
        Plural::One => format!("{count} nouvel élément de vos outils vous attend dans FocusMode."),
        _ => format!("{count} nouveaux éléments de vos outils vous attendent dans FocusMode."),
    },

    nudge_title: "Retour au travail",
    nudge_body: |domain| format!("{domain} attendra la pause."),
    nudge_body_pattern: "Ce site est sur la liste de blocage et attendra la pause.",

    blocklist_create: |detail| {
        format!("Impossible de créer le fichier avec la liste par défaut : {detail}")
    },
    blocklist_missing: "Le fichier est introuvable (supprimé ou déplacé). Ouvrez-le depuis les Paramètres : il sera recréé avec la liste par défaut.",
    blocklist_denied: "Accès au fichier refusé.",
    blocklist_io: |detail| format!("Erreur de lecture du fichier : {detail}"),
    blocklist_too_large: |megabytes| {
        format!("Le fichier est trop volumineux (limite : {megabytes} Mo).")
    },

    dev_sample_meeting: "Planification du sprint",
    dev_sample_reminder: "De l'eau et une petite marche",
    dev_sample_next_step: "Écrire l'introduction",
};
