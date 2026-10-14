//! Privacy panel (ADR-020): the pure parts of the report, the export and the deletes.
//!
//! The shell (`src-tauri/src/privacy.rs`) reads the files and folders, runs the deletes through the existing paths
//! (`clear_history`, the disconnects, …) and writes the export; this module only decides: which kinds a delete
//! request covers and in which order, how a size is shown, what the exported config looks like (no secrets) and
//! the shape of the export document. No IO, no clock.

use serde::Serialize;

use crate::api::{AppError, Config, PrivacyItemKind, SessionSummary};

/// `export_privacy_data` failed (folder missing, no access, disk full).
pub const ERR_EXPORT: &str = "privacy.export";
/// A delete failed for at least one item (the others were deleted).
pub const ERR_DELETE: &str = "privacy.delete";
/// `delete_privacy_data` named a kind that is never deleted directly (`settings`, `shieldsRestore`, `tintRestore`).
pub const ERR_NOT_DELETABLE: &str = "privacy.notDeletable";
/// The data folder could not be opened in the file manager.
pub const ERR_OPEN_FOLDER: &str = "privacy.openFolder";
/// `AppError.field` of [`ERR_NOT_DELETABLE`].
pub const FIELD_KINDS: &str = "kinds";

/// `format` of the export document.
pub const EXPORT_FORMAT: &str = "focusmode-export";
/// `version` of the export document (ARCHITECTURE §6.12).
pub const EXPORT_VERSION: u32 = 1;

/// Size on disk in KiB, rounded up: a non-empty file is at least 1 KiB, an empty one 0. Saturates at `u32::MAX`.
pub fn size_kib(bytes: u64) -> u32 {
    u32::try_from(bytes.div_ceil(1024)).unwrap_or(u32::MAX)
}

/// The kinds a `delete_privacy_data` request covers: each once, in [`PrivacyItemKind::ALL`] order (so a request
/// always deletes in the same order, whatever order the UI sent). A kind that is never deleted directly rejects the
/// whole request with [`ERR_NOT_DELETABLE`] (field [`FIELD_KINDS`]): nothing is deleted then.
pub fn deletion_plan(kinds: &[PrivacyItemKind]) -> Result<Vec<PrivacyItemKind>, AppError> {
    if let Some(kind) = kinds.iter().find(|kind| !kind.is_deletable()) {
        return Err(AppError {
            code: ERR_NOT_DELETABLE.into(),
            message: format!("{kind:?} cannot be deleted directly"),
            field: Some(FIELD_KINDS.into()),
        });
    }
    Ok(PrivacyItemKind::ALL
        .into_iter()
        .filter(|kind| kinds.contains(kind))
        .collect())
}

/// Every kind "Delete everything" deletes (before the settings go back to the defaults), in report order.
pub fn everything() -> Vec<PrivacyItemKind> {
    PrivacyItemKind::ALL
        .into_iter()
        .filter(|kind| kind.is_deletable())
        .collect()
}

/// The config as it goes into the export: the same, with every secret emptied (`calendar.googleClientSecret`).
/// Tokens and ICS links are never in the config (they live in DPAPI files), so nothing else needs blanking.
pub fn exportable_config(config: &Config) -> Config {
    let mut exported = config.clone();
    exported.calendar.google_client_secret = String::new();
    exported
}

/// The stored session keeps window titles (`trackTitles` was on while it ran).
pub fn includes_titles(summary: &SessionSummary) -> bool {
    summary.usage.apps.iter().any(|app| !app.titles.is_empty())
}

/// The export document (one JSON file, ARCHITECTURE §6.12). `H` is one stored session, `C`, `S` and `R` the contents
/// of `meeting-choices.json`, `work-seen.json` and `rewards.json` as stored (`None` when there is no such file).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Export<H, C, S, R> {
    pub format: &'static str,
    pub version: u32,
    pub exported_at_ms: i64,
    pub app_version: String,
    /// [`exportable_config`]: no secrets.
    pub config: Config,
    /// Every stored session, oldest first (as in `history.jsonl`), window titles included when they were kept.
    pub history: Vec<H>,
    pub meeting_choices: Option<C>,
    pub work_seen: Option<S>,
    /// The rewards ledger (beta, ADR-022): numbers, day numbers and session ids only. Left out when there is none,
    /// so an export without the beta looks as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewards: Option<R>,
}

impl<H, C, S, R> Export<H, C, S, R> {
    /// A document with the current header fields; the config is made exportable here, so a caller cannot forget it.
    pub fn new(
        exported_at_ms: i64,
        app_version: &str,
        config: &Config,
        history: Vec<H>,
        meeting_choices: Option<C>,
        work_seen: Option<S>,
        rewards: Option<R>,
    ) -> Self {
        Self {
            format: EXPORT_FORMAT,
            version: EXPORT_VERSION,
            exported_at_ms,
            app_version: app_version.to_owned(),
            config: exportable_config(config),
            history,
            meeting_choices,
            work_seen,
            rewards,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AppUsage, TitleUsage, UsageReport};

    #[test]
    fn sizes_round_up_to_whole_kib() {
        assert_eq!(size_kib(0), 0);
        assert_eq!(size_kib(1), 1);
        assert_eq!(size_kib(1024), 1);
        assert_eq!(size_kib(1025), 2);
        assert_eq!(size_kib(u64::MAX), u32::MAX);
    }

    #[test]
    fn deletion_plan_is_ordered_unique_and_refuses_protected_kinds() {
        let plan = deletion_plan(&[
            PrivacyItemKind::Logs,
            PrivacyItemKind::History,
            PrivacyItemKind::Logs,
            PrivacyItemKind::GithubToken,
        ])
        .expect("deletable");
        assert_eq!(
            plan,
            [
                PrivacyItemKind::History,
                PrivacyItemKind::GithubToken,
                PrivacyItemKind::Logs
            ]
        );
        assert_eq!(deletion_plan(&[]).expect("empty"), []);
        for protected in [
            PrivacyItemKind::Settings,
            PrivacyItemKind::ShieldsRestore,
            PrivacyItemKind::TintRestore,
        ] {
            let err = deletion_plan(&[PrivacyItemKind::History, protected]).expect_err("refused");
            assert_eq!(err.code, ERR_NOT_DELETABLE);
            assert_eq!(err.field.as_deref(), Some(FIELD_KINDS));
        }
    }

    #[test]
    fn everything_skips_settings_and_restore_files() {
        let all = everything();
        assert_eq!(all.len(), PrivacyItemKind::ALL.len() - 3);
        assert!(all.iter().all(|kind| kind.is_deletable()));
        assert_eq!(deletion_plan(&all).expect("deletable"), all);
        assert!(all.contains(&PrivacyItemKind::BrowserData));
        assert!(!all.contains(&PrivacyItemKind::ShieldsRestore));
    }

    #[test]
    fn export_never_carries_the_google_client_secret() {
        let mut config = Config::default();
        config.calendar.google_client_id = "123-abc.apps.googleusercontent.com".into();
        config.calendar.google_client_secret = "GOCSPX-secret".into();
        config.focus_minutes = 30;
        let export: Export<u8, u8, u8, u8> =
            Export::new(5, "0.1.0", &config, vec![1], None, Some(2), None);
        assert_eq!(export.config.calendar.google_client_secret, "");
        assert_eq!(
            export.config.calendar.google_client_id,
            "123-abc.apps.googleusercontent.com"
        );
        assert_eq!(export.config.focus_minutes, 30);
        assert_eq!(
            (export.format, export.version, export.exported_at_ms),
            (EXPORT_FORMAT, EXPORT_VERSION, 5)
        );
        assert_eq!(config.calendar.google_client_secret, "GOCSPX-secret");

        let json = serde_json::to_value(&export).expect("serialize");
        assert_eq!(json["format"], "focusmode-export");
        assert_eq!(json["appVersion"], "0.1.0");
        assert_eq!(json["meetingChoices"], serde_json::Value::Null);
        assert_eq!(json["workSeen"], 2);
        assert!(json.get("rewards").is_none());
        let with_rewards: Export<u8, u8, u8, u8> =
            Export::new(5, "0.1.0", &config, Vec::new(), None, None, Some(3));
        assert_eq!(
            serde_json::to_value(&with_rewards).expect("serialize")["rewards"],
            3
        );
        assert_eq!(json["config"]["calendar"]["googleClientSecret"], "");
        assert!(!json.to_string().contains("GOCSPX-secret"));
    }

    #[test]
    fn titles_are_detected_per_session() {
        let mut summary = SessionSummary {
            session_id: "s".into(),
            started_at_ms: 0,
            ended_at_ms: 1,
            focused_secs: 1,
            break_secs: 0,
            cycles_completed: 0,
            usage: UsageReport {
                apps: vec![AppUsage {
                    process: "Code.exe".into(),
                    secs: 1,
                    titles: Vec::new(),
                }],
                ..UsageReport::default()
            },
            demo: false,
            goal: None,
            goal_done: None,
            steps: Vec::new(),
            sparks: None,
            focus_spans: Vec::new(),
        };
        assert!(!includes_titles(&summary));
        summary.usage.apps[0].titles.push(TitleUsage {
            title: "main.rs".into(),
            secs: 1,
        });
        assert!(includes_titles(&summary));
    }
}
