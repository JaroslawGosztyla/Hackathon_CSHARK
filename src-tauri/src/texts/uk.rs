//! Ukrainian shell texts (translated from `en.rs`, checked against the Polish source `pl.rs`; the UI's are in
//! `src/lib/i18n/uk.ts`). The user is addressed as "ви"; terms in docs/i18n-glossary.md.

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Старт",
    tray_pause: "Пауза",
    tray_resume: "Продовжити",
    tray_skip: "Пропустити фазу",
    tray_stop: "Стоп",
    tray_show: "Показати вікно",
    tray_quit: "Вийти",
    tray_status_idle: "Готово до фокусу",
    tray_tooltip_idle: "FocusMode — готово до фокусу",
    tray_status_paused: "Пауза · ",
    tray_tooltip_paused: " (пауза)",
    tray_cycle: |cycle| format!("цикл {cycle}"),
    phase_idle: "Готово",
    phase_focus: "Фокус",
    phase_break: "Перерва",

    media_window_title: "FocusMode — музика (YouTube)",
    popup_window_title: "FocusMode — сповіщення",

    seconds: |secs| format!("{secs} с"),
    minutes: |minutes| format!("{minutes} хв"),
    hours: |hours| format!("{hours} год"),
    hours_minutes: |hours, minutes| format!("{hours} год {minutes} хв"),

    break_title: "Час зробити перерву ☕",
    break_body: |back_in| {
        format!("Встаньте, потягніться, випийте води. Повертаємося через {back_in}.")
    },
    break_body_reminder: |reminder, back_in| {
        format!("Нагадування: {reminder}\nПовертаємося через {back_in}.")
    },
    focus_title: "Час для фокусу 🎧",
    focus_body: |cycle, focus| format!("Цикл {cycle}: {focus} фокусу."),
    demo_title: "Демо-режим",
    demo_body: "Зупиніть сесію, щоб увімкнути демо-режим.",
    interrupted_title: "Сесію призупинено",
    interrupted_body: "Комп’ютер був у режимі сну — продовжте, коли повернетеся.",

    held_title: "Перерву призупинено 📅",
    held_meeting: |subject| format!("Перерву призупинено — триває зустріч: {subject}."),
    held_meeting_untitled: "Перерву призупинено — триває зустріч.",
    held_resume: |length| format!("Продовжте, коли закінчите: перерва триватиме {length}."),
    held_resume_untimed: "Продовжте, коли закінчите.",
    held_reminder: |reminder| format!("Нагадування: {reminder}"),

    meeting_title: "Зустріч під час перерви 📅",
    meeting_subject: |subject| format!("«{subject}»"),
    meeting_untitled: "Зустріч",
    meeting_at_in: |clock, minutes| format!("о {clock} (через {minutes} хв)"),
    meeting_at: |clock| format!("о {clock}"),
    meeting_in: |minutes| format!("через {minutes} хв"),
    meeting_now: "зараз",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} припадає на перерву {break_number}. \
             Відкрийте FocusMode і виберіть: подовжити фокус, призупинити перерву або проігнорувати."
        )
    },

    launchers_title: "Лаунчери ігор закрито 🎮",
    launchers_body: |names| format!("{names}. Їх можна знову запустити будь-коли."),
    ea_app: "Застосунок EA",
    xbox_app: "Застосунок Xbox",

    work_digest_title: "Що далі? 📋",
    work_digest_body: |count| {
        let (items, waits) = match plural(Language::Uk, count) {
            Plural::One => ("новий елемент", "чекає"),
            Plural::Few => ("нові елементи", "чекають"),
            Plural::Many | Plural::Other => ("нових елементів", "чекають"),
        };
        format!("{count} {items} з ваших інструментів {waits} у FocusMode.")
    },

    nudge_title: "Повертаймося до роботи",
    nudge_body: |domain| format!("{domain} зачекає до перерви."),
    nudge_body_pattern: "Цей сайт є в списку блокування й зачекає до перерви.",

    blocklist_create: |detail| {
        format!("Не вдається створити файл зі стандартним списком: {detail}")
    },
    blocklist_missing: "Файлу немає (видалений або переміщений). Відкрийте його з налаштувань, і його буде створено знову зі стандартним списком.",
    blocklist_denied: "Немає доступу до файлу.",
    blocklist_io: |detail| format!("Помилка читання файлу: {detail}"),
    blocklist_too_large: |megabytes| format!("Файл завеликий (ліміт: {megabytes} МБ)."),
};
