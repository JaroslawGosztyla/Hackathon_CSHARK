//! English shell texts (translated from `pl.rs`; the UI's are in `src/lib/i18n/en.ts`).

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Start",
    tray_pause: "Pause",
    tray_resume: "Resume",
    tray_skip: "Skip phase",
    tray_stop: "Stop",
    tray_show: "Show window",
    tray_quit: "Quit",
    tray_status_idle: "Ready to focus",
    tray_tooltip_idle: "FocusMode — ready to focus",
    tray_status_paused: "Paused · ",
    tray_tooltip_paused: " (paused)",
    tray_cycle: |cycle| format!("cycle {cycle}"),
    phase_idle: "Ready",
    phase_focus: "Focus",
    phase_break: "Break",

    media_window_title: "FocusMode — music (YouTube)",
    popup_window_title: "FocusMode — notification",

    seconds: |secs| format!("{secs} s"),
    minutes: |minutes| format!("{minutes} min"),
    hours: |hours| format!("{hours} h"),
    hours_minutes: |hours, minutes| format!("{hours} h {minutes} min"),

    break_title: "Time for a break ☕",
    break_body: |back_in| format!("Stand up, stretch, have some water. Back in {back_in}."),
    break_body_reminder: |reminder, back_in| format!("Reminder: {reminder}\nBack in {back_in}."),
    focus_title: "Time to focus 🎧",
    focus_body: |cycle, focus| format!("Cycle {cycle}: {focus} of focus."),
    demo_title: "Demo mode",
    demo_body: "Stop the session to turn on demo mode.",
    interrupted_title: "Session paused",
    interrupted_body: "The computer was asleep — resume when you're back.",

    held_title: "Break on hold 📅",
    held_meeting: |subject| format!("Break on hold — meeting in progress: {subject}."),
    held_meeting_untitled: "Break on hold — a meeting is in progress.",
    held_resume: |length| format!("Resume when you're done: the break will last {length}."),
    held_resume_untimed: "Resume when you're done.",
    held_reminder: |reminder| format!("Reminder: {reminder}"),

    meeting_title: "Meeting during a break 📅",
    meeting_subject: |subject| format!("“{subject}”"),
    meeting_untitled: "A meeting",
    meeting_at_in: |clock, minutes| format!("at {clock} (in {minutes} min)"),
    meeting_at: |clock| format!("at {clock}"),
    meeting_in: |minutes| format!("in {minutes} min"),
    meeting_now: "now",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} falls during break {break_number}. \
             Open FocusMode and choose: extend focus, hold the break or ignore it."
        )
    },

    launchers_title: "Game launchers closed 🎮",
    launchers_body: |names| format!("{names}. You can start them again whenever you like."),
    ea_app: "EA app",
    xbox_app: "Xbox app",

    work_digest_title: "What's next? 📋",
    work_digest_body: |count| match plural(Language::En, count) {
        Plural::One => format!("{count} new item from your tools is waiting in FocusMode."),
        _ => format!("{count} new items from your tools are waiting in FocusMode."),
    },

    nudge_title: "Back to work",
    nudge_body: |domain| format!("{domain} will wait until the break."),
    nudge_body_pattern: "This site is on the block list and will wait until the break.",

    blocklist_create: |detail| format!("Can't create the file with the default list: {detail}"),
    blocklist_missing: "The file is missing (deleted or moved). Open it from Settings and it will be created again with the default list.",
    blocklist_denied: "No access to the file.",
    blocklist_io: |detail| format!("Error reading the file: {detail}"),
    blocklist_too_large: |megabytes| format!("The file is too large (limit: {megabytes} MB)."),
};
