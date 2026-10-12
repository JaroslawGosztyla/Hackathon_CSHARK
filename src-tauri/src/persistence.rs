//! All file IO of the app: config, session history, shields crash-restore token, Teams token path.
//!
//! Every method is blocking. Callers on the async runtime use `spawn_blocking`; the shields
//! and Teams workers call it from their own threads. Writes are serialized by one lock.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use focus_core::api::{AppError, Config, SessionSummary};
use focus_platform::notifications::MuteToken;
use tauri::{AppHandle, Manager};

use crate::lock;

const CONFIG_FILE: &str = "config.json";
const CONFIG_BACKUP_FILE: &str = "config.json.bak";
const HISTORY_FILE: &str = "history.jsonl";
const SHIELDS_RESTORE_FILE: &str = "shields-restore.json";
const TEAMS_TOKEN_FILE: &str = "teams-token.bin";

#[derive(Clone)]
pub struct Persistence {
    inner: Arc<Inner>,
}

struct Inner {
    config_path: PathBuf,
    config_backup_path: PathBuf,
    history_path: PathBuf,
    restore_path: PathBuf,
    teams_token_path: PathBuf,
    write_lock: Mutex<()>,
}

impl Persistence {
    /// Resolves and creates the app config and data directories.
    pub fn new(app: &AppHandle) -> anyhow::Result<Self> {
        let config_dir = app
            .path()
            .app_config_dir()
            .context("resolve app config dir")?;
        let data_dir = app.path().app_data_dir().context("resolve app data dir")?;
        Self::with_dirs(&config_dir, &data_dir)
    }

    fn with_dirs(config_dir: &Path, data_dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(config_dir)
            .with_context(|| format!("create config dir {}", config_dir.display()))?;
        fs::create_dir_all(data_dir)
            .with_context(|| format!("create data dir {}", data_dir.display()))?;
        Ok(Self {
            inner: Arc::new(Inner {
                config_path: config_dir.join(CONFIG_FILE),
                config_backup_path: config_dir.join(CONFIG_BACKUP_FILE),
                history_path: data_dir.join(HISTORY_FILE),
                restore_path: data_dir.join(SHIELDS_RESTORE_FILE),
                teams_token_path: data_dir.join(TEAMS_TOKEN_FILE),
                write_lock: Mutex::new(()),
            }),
        })
    }

    pub fn teams_token_path(&self) -> PathBuf {
        self.inner.teams_token_path.clone()
    }

    /// Loads `config.json`. A missing file gives defaults silently; a corrupted or invalid file is
    /// renamed to `config.json.bak` and defaults are used, with an error to report to the UI.
    pub fn load_config(&self) -> (Config, Option<AppError>) {
        let text = match fs::read_to_string(&self.inner.config_path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return (Config::default(), None),
            Err(err) => {
                return (
                    Config::default(),
                    Some(AppError::new(
                        "config.read",
                        format!("cannot read {}: {err}", self.inner.config_path.display()),
                    )),
                );
            }
        };

        let parsed = serde_json::from_str::<Config>(&text)
            .map_err(|err| err.to_string())
            .and_then(|config| config.normalized().map_err(|err| err.message));
        match parsed {
            Ok(config) => (config, None),
            Err(detail) => {
                let _guard = lock(&self.inner.write_lock);
                let backup = fs::rename(&self.inner.config_path, &self.inner.config_backup_path);
                if let Err(err) = backup {
                    log::warn!("cannot back up corrupted config: {err}");
                }
                (
                    Config::default(),
                    Some(AppError::new(
                        "config.corrupt",
                        format!(
                            "config.json was invalid ({detail}); defaults restored, old file kept as config.json.bak"
                        ),
                    )),
                )
            }
        }
    }

    /// Writes `config.json` atomically (temp file + rename).
    pub fn save_config(&self, config: &Config) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(config).context("serialize config")?;
        let _guard = lock(&self.inner.write_lock);
        write_atomic(&self.inner.config_path, &json)
    }

    /// Appends one summary as a JSON line to `history.jsonl`.
    pub fn append_history(&self, summary: &SessionSummary) -> anyhow::Result<()> {
        let mut line = serde_json::to_vec(summary).context("serialize summary")?;
        line.push(b'\n');
        let _guard = lock(&self.inner.write_lock);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.inner.history_path)
            .with_context(|| format!("open {}", self.inner.history_path.display()))?;
        file.write_all(&line).context("append history")?;
        file.flush().context("flush history")?;
        // TODO(hackathon): rotate history.jsonl when it grows large.
        Ok(())
    }

    /// Newest first, at most `limit` entries. Unparseable lines are skipped.
    pub fn list_history(&self, limit: usize) -> anyhow::Result<Vec<SessionSummary>> {
        let text = match fs::read_to_string(&self.inner.history_path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("read {}", self.inner.history_path.display()));
            }
        };
        Ok(text
            .lines()
            .rev()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| match serde_json::from_str::<SessionSummary>(line) {
                Ok(summary) => Some(summary),
                Err(err) => {
                    log::warn!("skipping invalid history line: {err}");
                    None
                }
            })
            .take(limit)
            .collect())
    }

    /// Remembers how to restore Windows notifications if the app dies while they are muted.
    pub fn save_shields_restore(&self, token: &MuteToken) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(token).context("serialize mute token")?;
        let _guard = lock(&self.inner.write_lock);
        write_atomic(&self.inner.restore_path, &json)
    }

    /// The token left behind by a previous run that did not restore notifications.
    /// A corrupted file is deleted (nothing sensible can be restored from it).
    pub fn load_shields_restore(&self) -> Option<MuteToken> {
        let text = match fs::read_to_string(&self.inner.restore_path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return None,
            Err(err) => {
                log::warn!("cannot read shields restore file: {err}");
                return None;
            }
        };
        match serde_json::from_str::<MuteToken>(&text) {
            Ok(token) => Some(token),
            Err(err) => {
                log::warn!("corrupted shields restore file, deleting it: {err}");
                self.clear_shields_restore();
                None
            }
        }
    }

    pub fn clear_shields_restore(&self) {
        let _guard = lock(&self.inner.write_lock);
        match fs::remove_file(&self.inner.restore_path) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => log::warn!("cannot delete shields restore file: {err}"),
        }
    }
}

/// Writes `bytes` to `<path>.tmp`, syncs it and renames it over `path`.
fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let tmp = path.with_file_name(format!("{file_name}.tmp"));
    {
        let mut file = File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("sync {}", tmp.display()))?;
    }
    fs::rename(&tmp, path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dirs(name: &str) -> (PathBuf, Persistence) {
        let root = std::env::temp_dir().join(format!(
            "focusmode-persistence-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let persistence =
            Persistence::with_dirs(&root.join("config"), &root.join("data")).expect("dirs");
        (root, persistence)
    }

    #[test]
    fn config_round_trip_and_corruption() {
        let (root, p) = temp_dirs("config");
        let (cfg, err) = p.load_config();
        assert!(err.is_none());
        assert_eq!(cfg, Config::default());

        let changed = Config {
            focus_minutes: 25,
            ..Config::default()
        };
        p.save_config(&changed).expect("save");
        let (loaded, err) = p.load_config();
        assert!(err.is_none());
        assert_eq!(loaded.focus_minutes, 25);

        fs::write(root.join("config").join(CONFIG_FILE), b"{ not json").expect("write");
        let (loaded, err) = p.load_config();
        assert_eq!(loaded, Config::default());
        assert_eq!(err.map(|e| e.code), Some("config.corrupt".to_owned()));
        assert!(root.join("config").join(CONFIG_BACKUP_FILE).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn history_is_newest_first() {
        let (root, p) = temp_dirs("history");
        for i in 0..3 {
            let summary = SessionSummary {
                session_id: format!("s{i}"),
                started_at_ms: i,
                ended_at_ms: i + 1,
                focused_secs: 1,
                break_secs: 0,
                cycles_completed: 0,
                usage: Default::default(),
                demo: true,
            };
            p.append_history(&summary).expect("append");
        }
        let list = p.list_history(2).expect("list");
        let ids: Vec<_> = list.iter().map(|s| s.session_id.as_str()).collect();
        assert_eq!(ids, ["s2", "s1"]);
        let _ = fs::remove_dir_all(&root);
    }
}
