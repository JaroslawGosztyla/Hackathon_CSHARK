//! Japanese shell texts (translated from `en.rs`, checked against `pl.rs`; the UI's are in `src/lib/i18n/ja.ts`).
//! Japanese has no plural forms. Terms: docs/i18n-glossary.md (column JA).

use super::Texts;

pub(super) static TEXTS: Texts = Texts {
    tray_start: "開始",
    tray_pause: "一時停止",
    tray_resume: "再開",
    tray_skip: "フェーズをスキップ",
    tray_stop: "停止",
    tray_show: "ウィンドウを表示",
    tray_quit: "終了",
    tray_status_idle: "集中の準備ができました",
    tray_tooltip_idle: "FocusMode — 集中の準備ができました",
    tray_status_paused: "一時停止中 · ",
    tray_tooltip_paused: "（一時停止中）",
    tray_cycle: |cycle| format!("サイクル {cycle}"),
    phase_idle: "待機中",
    phase_focus: "集中",
    phase_break: "休憩",

    media_window_title: "FocusMode — 音楽（YouTube）",
    popup_window_title: "FocusMode — 通知",

    seconds: |secs| format!("{secs}秒"),
    minutes: |minutes| format!("{minutes}分"),
    hours: |hours| format!("{hours}時間"),
    hours_minutes: |hours, minutes| format!("{hours}時間{minutes}分"),

    break_title: "休憩の時間です ☕",
    break_body: |back_in| {
        format!("立ち上がって体を伸ばし、水を飲みましょう。{back_in}後に戻ります。")
    },
    break_body_reminder: |reminder, back_in| {
        format!("リマインダー：{reminder}\n{back_in}後に戻ります。")
    },
    focus_title: "集中の時間です 🎧",
    focus_body: |cycle, focus| format!("サイクル {cycle}：{focus}の集中。"),
    demo_title: "デモモード",
    demo_body: "デモモードにするには、セッションを停止してください。",
    interrupted_title: "セッションを一時停止しました",
    interrupted_body: "パソコンがスリープしていました。戻ったら再開してください。",

    held_title: "休憩を保留中 📅",
    held_meeting: |subject| format!("休憩を保留中です。会議中：{subject}。"),
    held_meeting_untitled: "休憩を保留中です。会議が行われています。",
    held_resume: |length| format!("終わったら再開してください。休憩は{length}です。"),
    held_resume_untimed: "終わったら再開してください。",
    held_reminder: |reminder| format!("リマインダー：{reminder}"),

    meeting_title: "休憩中に会議があります 📅",
    meeting_subject: |subject| format!("会議「{subject}」"),
    meeting_untitled: "会議",
    meeting_at_in: |clock, minutes| format!("{clock}（{minutes}分後）に"),
    meeting_at: |clock| format!("{clock}に"),
    meeting_in: |minutes| format!("{minutes}分後に"),
    meeting_now: "今から",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject}は{when}始まり、{break_number}回目の休憩と重なります。\
             FocusMode を開いて、集中を延長、休憩で一時停止、無視のどれかを選んでください。"
        )
    },

    launchers_title: "ゲームランチャーを終了しました 🎮",
    launchers_body: |names| format!("{names}。いつでもまた起動できます。"),
    ea_app: "EA app",
    xbox_app: "Xbox アプリ",

    work_digest_title: "次にやること 📋",
    work_digest_body: |count| {
        format!("ツールからの新しい項目が{count}件、FocusMode で待っています。")
    },

    nudge_title: "作業に戻りましょう",
    nudge_body: |domain| format!("{domain} は休憩まで待ってくれます。"),
    nudge_body_pattern: "このサイトはブロックリストに入っています。休憩まで待ちましょう。",

    blocklist_create: |detail| format!("既定のリストでファイルを作成できません：{detail}"),
    blocklist_missing: "ファイルがありません（削除または移動されました）。設定から開くと、既定のリストで作り直されます。",
    blocklist_denied: "ファイルにアクセスできません。",
    blocklist_io: |detail| format!("ファイルの読み込みエラー：{detail}"),
    blocklist_too_large: |megabytes| format!("ファイルが大きすぎます（上限：{megabytes} MB）。"),
};
