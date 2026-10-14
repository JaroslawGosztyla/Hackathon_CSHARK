//! The blocking side of "Co dalej?", over `focus_platform::work`: one short-lived thread per fetch, the
//! providers kept between fetches (GitHub keeps `Last-Modified` for `If-Modified-Since` and its last
//! notifications, Jira its sprint field) and the sealed token files. Never runs on the async runtime or
//! the main thread (the platform uses blocking HTTP).
//!
//! A provider is built from the sealed token when its source's epoch or settings change. The token never
//! leaves this module: results carry items, the account name, the poll hint and warnings, or an error text
//! redacted here ([`super::model::redact`]: links and the token itself replaced).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, WorkConfig, WorkItem, WorkSource};
use focus_platform::PlatformError;
use focus_platform::work::{self as platform, WorkProvider};

use super::WorkCmd;
use super::model::{SOURCES, SourceSettings, redact, source_index};
use crate::lock;

/// Tokens can be stored (sealed) on this system.
pub const SECRETS_SUPPORTED: bool = focus_platform::secrets::SUPPORTED;

/// One fetch of one source.
pub struct Job {
    pub id: u64,
    pub source: WorkSource,
    pub epoch: u64,
    /// The source's part of `config` (a change rebuilds the provider).
    pub settings: SourceSettings,
    /// The work config the provider is built from.
    pub config: WorkConfig,
    pub token_path: PathBuf,
    /// Wall clock when the fetch was asked for.
    pub now_ms: i64,
}

/// The answer to a [`Job`].
pub struct Fetched {
    pub id: u64,
    pub source: WorkSource,
    pub epoch: u64,
    /// Wall clock when the fetch finished.
    pub at_ms: i64,
    /// The items, or a redacted English error.
    pub outcome: Result<Outcome, String>,
}

pub struct Outcome {
    /// Unranked items (`focus_core::work::rank` scores, dedupes and cleans them).
    pub items: Vec<WorkItem>,
    /// Signed-in user name, when the tool reports it.
    pub account: Option<String>,
    /// How often the tool allows polling (GitHub `X-Poll-Interval`), seconds.
    pub poll_hint_secs: Option<u32>,
    /// Optional parts that failed while the main list worked (redacted, joined), e.g. GitHub notifications
    /// with a fine-grained token.
    pub warning: Option<String>,
}

/// The provider of one source, kept while its epoch and settings stay the same.
struct Slot {
    epoch: u64,
    settings: SourceSettings,
    provider: Box<dyn WorkProvider>,
    /// Only to redact errors.
    token: String,
}

/// The providers of all sources; a fetch locks its source's slot, so fetches of one source never overlap.
#[derive(Default)]
pub struct Clients {
    slots: [Arc<Mutex<Option<Slot>>>; SOURCES.len()],
}

impl Clients {
    /// Runs `job` on its own thread; the result arrives as `WorkCmd::Fetched`. `false` when the thread
    /// could not start.
    pub fn fetch(&self, job: Job, done: Sender<WorkCmd>) -> bool {
        let slot = Arc::clone(&self.slots[source_index(job.source)]);
        std::thread::Builder::new()
            .name("focus-work-fetch".into())
            .spawn(move || {
                let outcome = catch_unwind(AssertUnwindSafe(|| run(&slot, &job)))
                    .unwrap_or_else(|_| Err("the fetch failed unexpectedly".to_owned()));
                let _ = done.send(WorkCmd::Fetched(Fetched {
                    id: job.id,
                    source: job.source,
                    epoch: job.epoch,
                    at_ms: wall_ms(),
                    outcome,
                }));
            })
            .is_ok()
    }
}

fn run(slot: &Mutex<Option<Slot>>, job: &Job) -> Result<Outcome, String> {
    let mut slot = lock(slot);
    let fresh = slot
        .as_ref()
        .is_none_or(|kept| kept.epoch != job.epoch || kept.settings != job.settings);
    if fresh {
        *slot = None;
        let token = match platform::load_token(&job.token_path) {
            Ok(Some(token)) => token,
            Ok(None) => return Err("no access token is stored".to_owned()),
            Err(err) => {
                return Err(format!(
                    "cannot read the stored access token, enter it again: {}",
                    redact(&err.to_string(), None)
                ));
            }
        };
        let provider = platform::provider(job.source, &job.config, &token)
            .map_err(|err| redact(&err.to_string(), Some(&token)))?;
        *slot = Some(Slot {
            epoch: job.epoch,
            settings: job.settings.clone(),
            provider,
            token,
        });
    }
    let Some(kept) = slot.as_mut() else {
        return Err("the client is not available".to_owned());
    };
    let token = Some(kept.token.as_str());
    let fetched = kept
        .provider
        .fetch(job.now_ms)
        .map_err(|err| redact(&err.to_string(), token))?;
    let warnings: Vec<String> = fetched
        .warnings
        .iter()
        .map(|warning| redact(warning, token))
        .filter(|warning| !warning.is_empty())
        .collect();
    Ok(Outcome {
        items: fetched.items,
        account: fetched.account,
        poll_hint_secs: fetched.poll_interval_secs,
        warning: (!warnings.is_empty()).then(|| redact(&warnings.join("; "), token)),
    })
}

// -- sealed tokens ---------------------------------------------------------------------------------------

/// The token as stored (`set_work_token`): trimmed, 1..=4096 visible ASCII characters
/// (`focus_platform::work::normalize_token`). The error never repeats the token.
pub fn validate_token(raw: &str) -> Result<String, AppError> {
    platform::normalize_token(raw).map_err(|err| AppError {
        code: "work.token".into(),
        message: match err {
            PlatformError::Invalid(detail) => detail,
            _ => "the access token cannot be used".to_owned(),
        },
        field: Some("token".into()),
    })
}

/// A sealed token file exists.
pub fn has_token(path: &Path) -> bool {
    path.is_file()
}

/// Stores a validated token sealed (DPAPI, atomic write; refused where encryption is unavailable).
pub fn store_token(path: &Path, token: &str) -> Result<(), AppError> {
    platform::save_token(path, token).map_err(|err| {
        AppError::new(
            "work.tokenSave",
            format!(
                "cannot store the access token: {}",
                redact(&err.to_string(), Some(token))
            ),
        )
    })
}

/// Forgets the stored token (a missing file is fine).
pub fn delete_token(path: &Path) -> Result<(), AppError> {
    platform::delete_token(path).map_err(|err| {
        AppError::new(
            "work.tokenSave",
            format!(
                "cannot delete the access token: {}",
                redact(&err.to_string(), None)
            ),
        )
    })
}

fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_tokens_map_to_a_field_error_without_the_value() {
        assert_eq!(
            validate_token("  glpat-abc_123 ").as_deref(),
            Ok("glpat-abc_123")
        );
        for bad in ["", "  ", "lin_api secret", "zażółć", &"a".repeat(4097)] {
            let err = validate_token(bad).expect_err("rejected");
            assert_eq!(err.code, "work.token");
            assert_eq!(err.field.as_deref(), Some("token"));
            assert!(!err.message.contains("secret"), "{}", err.message);
            assert!(!err.message.is_empty());
        }
        assert!(validate_token(&"a".repeat(4096)).is_ok());
    }

    #[test]
    fn missing_tokens_are_not_found() {
        let path = std::env::temp_dir().join(format!(
            "focusmode-work-token-missing-{}.bin",
            std::process::id()
        ));
        assert!(!has_token(&path));
        delete_token(&path).expect("deleting a missing token is fine");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tokens_round_trip_sealed() {
        let path =
            std::env::temp_dir().join(format!("focusmode-work-token-{}.bin", std::process::id()));
        store_token(&path, "ghp_Sealed123").expect("store");
        assert!(has_token(&path));
        let raw = std::fs::read(&path).expect("raw");
        assert!(!String::from_utf8_lossy(&raw).contains("ghp_Sealed123"));
        assert_eq!(
            platform::load_token(&path).expect("load").as_deref(),
            Some("ghp_Sealed123")
        );
        delete_token(&path).expect("delete");
        assert!(!has_token(&path));
    }
}
