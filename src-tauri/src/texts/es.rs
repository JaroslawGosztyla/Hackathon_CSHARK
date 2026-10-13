//! Spanish shell texts (translated from `en.rs`, checked against `pl.rs`; the UI's are in `src/lib/i18n/es.ts`).
//! "Descanso" is the break phase, "Pausar" / "En pausa" pauses the timer.

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Iniciar",
    tray_pause: "Pausar",
    tray_resume: "Reanudar",
    tray_skip: "Saltar fase",
    tray_stop: "Detener",
    tray_show: "Mostrar ventana",
    tray_quit: "Salir",
    tray_status_idle: "Listo para concentrarte",
    tray_tooltip_idle: "FocusMode — listo para concentrarte",
    tray_status_paused: "En pausa · ",
    tray_tooltip_paused: " (en pausa)",
    tray_cycle: |cycle| format!("ciclo {cycle}"),
    phase_idle: "Listo",
    phase_focus: "Concentración",
    phase_break: "Descanso",

    media_window_title: "FocusMode — música (YouTube)",
    popup_window_title: "FocusMode — notificación",

    seconds: |secs| format!("{secs} s"),
    minutes: |minutes| format!("{minutes} min"),
    hours: |hours| format!("{hours} h"),
    hours_minutes: |hours, minutes| format!("{hours} h {minutes} min"),

    break_title: "Hora de descansar ☕",
    break_body: |back_in| {
        format!("Levántate, estírate, bebe un poco de agua. Volvemos en {back_in}.")
    },
    break_body_reminder: |reminder, back_in| {
        format!("Recordatorio: {reminder}\nVolvemos en {back_in}.")
    },
    focus_title: "Hora de concentrarse 🎧",
    focus_body: |cycle, focus| format!("Ciclo {cycle}: {focus} de concentración."),
    demo_title: "Modo demo",
    demo_body: "Detén la sesión para activar el modo demo.",
    interrupted_title: "Sesión en pausa",
    interrupted_body: "El equipo estaba en suspensión — reanuda cuando vuelvas.",

    held_title: "Descanso en espera 📅",
    held_meeting: |subject| format!("Descanso en espera — reunión en curso: {subject}."),
    held_meeting_untitled: "Descanso en espera — hay una reunión en curso.",
    held_resume: |length| format!("Reanuda cuando termines: el descanso durará {length}."),
    held_resume_untimed: "Reanuda cuando termines.",
    held_reminder: |reminder| format!("Recordatorio: {reminder}"),

    meeting_title: "Reunión durante un descanso 📅",
    meeting_subject: |subject| format!("«{subject}»"),
    meeting_untitled: "Una reunión",
    meeting_at_in: |clock, minutes| format!("a las {clock} (dentro de {minutes} min)"),
    meeting_at: |clock| format!("a las {clock}"),
    meeting_in: |minutes| format!("dentro de {minutes} min"),
    meeting_now: "ahora",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} cae durante el descanso {break_number}. \
             Abre FocusMode y elige: alargar la concentración, dejar el descanso en espera o ignorarla."
        )
    },

    launchers_title: "Lanzadores de juegos cerrados 🎮",
    launchers_body: |names| format!("{names}. Puedes volver a abrirlos cuando quieras."),
    ea_app: "Aplicación EA",
    xbox_app: "Aplicación Xbox",

    work_digest_title: "¿Qué sigue? 📋",
    work_digest_body: |count| match plural(Language::Es, count) {
        Plural::One => {
            format!("{count} elemento nuevo de tus herramientas te espera en FocusMode.")
        }
        _ => format!("{count} elementos nuevos de tus herramientas te esperan en FocusMode."),
    },

    nudge_title: "Vuelve al trabajo",
    nudge_body: |domain| format!("{domain} puede esperar hasta el descanso."),
    nudge_body_pattern: "Este sitio está en la lista de bloqueo y puede esperar hasta el descanso.",

    blocklist_create: |detail| {
        format!("No se puede crear el archivo con la lista predeterminada: {detail}")
    },
    blocklist_missing: "Falta el archivo (se borró o se movió). Ábrelo desde Configuración y se volverá a crear con la lista predeterminada.",
    blocklist_denied: "Sin acceso al archivo.",
    blocklist_io: |detail| format!("Error al leer el archivo: {detail}"),
    blocklist_too_large: |megabytes| {
        format!("El archivo es demasiado grande (límite: {megabytes} MB).")
    },

    dev_sample_meeting: "Planificación del sprint",
    dev_sample_reminder: "Agua y un paseo corto",
};
