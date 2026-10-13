//! Russian shell texts (translated from `en.rs`, checked against the Polish source `pl.rs`; the UI's are in
//! `src/lib/i18n/ru.ts`). The user is addressed as "вы", written without "ё" as Windows does; terms in
//! docs/i18n-glossary.md.

use focus_core::api::Language;

use super::{Plural, Texts, plural};

pub(super) static TEXTS: Texts = Texts {
    tray_start: "Старт",
    tray_pause: "Пауза",
    tray_resume: "Продолжить",
    tray_skip: "Пропустить фазу",
    tray_stop: "Стоп",
    tray_show: "Показать окно",
    tray_quit: "Выйти",
    tray_status_idle: "Готово к фокусу",
    tray_tooltip_idle: "FocusMode — готово к фокусу",
    tray_status_paused: "Пауза · ",
    tray_tooltip_paused: " (пауза)",
    tray_cycle: |cycle| format!("цикл {cycle}"),
    phase_idle: "Готово",
    phase_focus: "Фокус",
    phase_break: "Перерыв",

    media_window_title: "FocusMode — музыка (YouTube)",
    popup_window_title: "FocusMode — уведомление",
    youtube_sign_in_title: "FocusMode — вход в YouTube",

    seconds: |secs| format!("{secs} с"),
    minutes: |minutes| format!("{minutes} мин"),
    hours: |hours| format!("{hours} ч"),
    hours_minutes: |hours, minutes| format!("{hours} ч {minutes} мин"),

    break_title: "Пора сделать перерыв ☕",
    break_body: |back_in| {
        format!("Встаньте, потянитесь, выпейте воды. Возвращаемся через {back_in}.")
    },
    break_body_reminder: |reminder, back_in| {
        format!("Напоминание: {reminder}\nВозвращаемся через {back_in}.")
    },
    focus_title: "Время фокуса 🎧",
    focus_body: |cycle, focus| format!("Цикл {cycle}: {focus} фокуса."),
    demo_title: "Демо-режим",
    demo_body: "Остановите сессию, чтобы включить демо-режим.",
    interrupted_title: "Сессия приостановлена",
    interrupted_body: "Компьютер был в спящем режиме — продолжите, когда вернетесь.",

    held_title: "Перерыв приостановлен 📅",
    held_meeting: |subject| format!("Перерыв приостановлен — идет встреча: {subject}."),
    held_meeting_untitled: "Перерыв приостановлен — идет встреча.",
    held_resume: |length| format!("Продолжите, когда закончите: перерыв продлится {length}."),
    held_resume_untimed: "Продолжите, когда закончите.",
    held_reminder: |reminder| format!("Напоминание: {reminder}"),

    meeting_title: "Встреча во время перерыва 📅",
    meeting_subject: |subject| format!("«{subject}»"),
    meeting_untitled: "Встреча",
    meeting_at_in: |clock, minutes| format!("в {clock} (через {minutes} мин)"),
    meeting_at: |clock| format!("в {clock}"),
    meeting_in: |minutes| format!("через {minutes} мин"),
    meeting_now: "сейчас",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject} {when} выпадает на перерыв {break_number}. \
             Откройте FocusMode и выберите: продлить фокус, приостановить перерыв или проигнорировать."
        )
    },

    launchers_title: "Игровые лаунчеры закрыты 🎮",
    launchers_body: |names| format!("{names}. Их можно снова запустить в любой момент."),
    ea_app: "Приложение EA",
    xbox_app: "Приложение Xbox",

    work_digest_title: "Что дальше? 📋",
    work_digest_body: |count| {
        let (items, waits) = match plural(Language::Ru, count) {
            Plural::One => ("новый элемент", "ждет"),
            Plural::Few => ("новых элемента", "ждут"),
            Plural::Many | Plural::Other => ("новых элементов", "ждут"),
        };
        format!("{count} {items} из ваших инструментов {waits} в FocusMode.")
    },

    nudge_title: "Вернемся к работе",
    nudge_body: |domain| format!("{domain} подождет до перерыва."),
    nudge_body_pattern: "Этот сайт в списке блокировки и подождет до перерыва.",

    blocklist_create: |detail| format!("Не удается создать файл со стандартным списком: {detail}"),
    blocklist_missing: "Файла нет (удален или перемещен). Откройте его из настроек, и он будет создан заново со стандартным списком.",
    blocklist_denied: "Нет доступа к файлу.",
    blocklist_io: |detail| format!("Ошибка чтения файла: {detail}"),
    blocklist_too_large: |megabytes| format!("Файл слишком большой (лимит: {megabytes} МБ)."),

    dev_sample_meeting: "Планирование спринта",
    dev_sample_reminder: "Вода и короткая прогулка",
    dev_sample_next_step: "Написать вступление",
};
