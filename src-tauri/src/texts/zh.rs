//! Simplified Chinese shell texts (translated from `en.rs`, checked against `pl.rs`; the UI's are in
//! `src/lib/i18n/zh.ts`). Chinese has no plural forms. Terms: docs/i18n-glossary.md (column ZH).

use super::Texts;

pub(super) static TEXTS: Texts = Texts {
    tray_start: "开始",
    tray_pause: "暂停",
    tray_resume: "继续",
    tray_skip: "跳过此阶段",
    tray_stop: "停止",
    tray_show: "显示窗口",
    tray_quit: "退出",
    tray_status_idle: "准备开始专注",
    tray_tooltip_idle: "FocusMode — 准备开始专注",
    tray_status_paused: "已暂停 · ",
    tray_tooltip_paused: "（已暂停）",
    tray_cycle: |cycle| format!("第 {cycle} 轮"),
    phase_idle: "就绪",
    phase_focus: "专注",
    phase_break: "休息",

    media_window_title: "FocusMode — 音乐（YouTube）",
    popup_window_title: "FocusMode — 通知",
    youtube_sign_in_title: "FocusMode — 登录 YouTube",

    seconds: |secs| format!("{secs} 秒"),
    minutes: |minutes| format!("{minutes} 分钟"),
    hours: |hours| format!("{hours} 小时"),
    hours_minutes: |hours, minutes| format!("{hours} 小时 {minutes} 分钟"),

    break_title: "休息一下吧 ☕",
    break_body: |back_in| format!("站起来，伸展一下，喝点水。{back_in}后回来。"),
    break_body_reminder: |reminder, back_in| format!("提醒：{reminder}\n{back_in}后回来。"),
    focus_title: "该专注了 🎧",
    focus_body: |cycle, focus| format!("第 {cycle} 轮：专注 {focus}。"),
    demo_title: "演示模式",
    demo_body: "请先停止当前时段，再开启演示模式。",
    interrupted_title: "时段已暂停",
    interrupted_body: "电脑刚才处于睡眠状态，回来后请点击继续。",

    held_title: "休息已暂停 📅",
    held_meeting: |subject| format!("休息已暂停，正在开会：{subject}。"),
    held_meeting_untitled: "休息已暂停，正在开会。",
    held_resume: |length| format!("结束后请点击继续：休息时长为 {length}。"),
    held_resume_untimed: "结束后请点击继续。",
    held_reminder: |reminder| format!("提醒：{reminder}"),

    meeting_title: "休息期间有会议 📅",
    meeting_subject: |subject| format!("会议“{subject}”"),
    meeting_untitled: "一个会议",
    meeting_at_in: |clock, minutes| format!("将于 {clock}（{minutes} 分钟后）"),
    meeting_at: |clock| format!("将于 {clock}"),
    meeting_in: |minutes| format!("将在 {minutes} 分钟后"),
    meeting_now: "现在就要",
    meeting_body: |subject, when, break_number| {
        format!(
            "{subject}{when}开始，正好在第 {break_number} 次休息期间。\
             打开 FocusMode 选择：延长专注、休息时暂停或忽略。"
        )
    },

    launchers_title: "已关闭游戏启动器 🎮",
    launchers_body: |names| format!("{names}。你可以随时重新启动它们。"),
    ea_app: "EA app",
    xbox_app: "Xbox 应用",

    work_digest_title: "接下来做什么？📋",
    work_digest_body: |count| format!("有 {count} 条来自你的工具的新内容在 FocusMode 中等你查看。"),

    nudge_title: "回到工作吧",
    nudge_body: |domain| format!("{domain} 可以等到休息时再看。"),
    nudge_body_pattern: "这个网站在屏蔽列表中，可以等到休息时再看。",

    blocklist_create: |detail| format!("无法创建包含默认列表的文件：{detail}"),
    blocklist_missing: "文件不存在（已被删除或移动）。从设置中打开它，就会用默认列表重新创建。",
    blocklist_denied: "没有访问该文件的权限。",
    blocklist_io: |detail| format!("读取文件时出错：{detail}"),
    blocklist_too_large: |megabytes| format!("文件太大（上限 {megabytes} MB）。"),

    dev_sample_meeting: "冲刺规划会",
    dev_sample_reminder: "喝水，短暂散步",
};
