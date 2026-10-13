//! All file IO of the app: config, session history, the crash-restore files of the shields (muted
//! notifications) and of the screen tint, the custom chime folder, the meeting choices and the paths of the
//! sealed secrets (Teams and Google tokens, ICS links; read and written through `focus_platform`).
//! `websiteblacklist.txt` lives next to `config.json`, but `crate::blocklist` reads and watches it.
//!
//! Every method is blocking. Callers on the async runtime use `spawn_blocking`; the shields
//! and Teams workers call it from their own threads. Writes are serialized by one lock.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use focus_core::api::{AppError, Config, SessionSummary};
use focus_core::blocklist::BLACKLIST_FILE_NAME;
use focus_platform::notifications::MuteToken;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tauri::{AppHandle, Manager};

use crate::lock;

const CONFIG_FILE: &str = "config.json";
const CONFIG_BACKUP_FILE: &str = "config.json.bak";
const HISTORY_FILE: &str = "history.jsonl";
const SHIELDS_RESTORE_FILE: &str = "shields-restore.json";
const TINT_RESTORE_FILE: &str = "tint-restore.json";
const TEAMS_TOKEN_FILE: &str = "teams-token.bin";
/// Google Calendar refresh token (DPAPI-sealed by `focus_platform::secrets`).
const GOOGLE_TOKEN_FILE: &str = "google-token.bin";
/// ICS subscription links (DPAPI-sealed; they often carry a private token).
const ICS_LINKS_FILE: &str = "ics-links.bin";
/// How the user answered meetings that collide with breaks (meeting ids only, no subjects).
const MEETING_CHOICES_FILE: &str = "meeting-choices.json";
/// Folder (in the app data directory) with the user's imported chime files.
const SOUNDS_DIR: &str = "sounds";

#[derive(Clone)]
pub struct Persistence {
    inner: Arc<Inner>,
}

struct Inner {
    config_path: PathBuf,
    config_backup_path: PathBuf,
    blocklist_path: PathBuf,
    history_path: PathBuf,
    restore_path: PathBuf,
    tint_restore_path: PathBuf,
    teams_token_path: PathBuf,
    google_token_path: PathBuf,
    ics_links_path: PathBuf,
    meeting_choices_path: PathBuf,
    sounds_dir: PathBuf,
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
                blocklist_path: config_dir.join(BLACKLIST_FILE_NAME),
                history_path: data_dir.join(HISTORY_FILE),
                restore_path: data_dir.join(SHIELDS_RESTORE_FILE),
                tint_restore_path: data_dir.join(TINT_RESTORE_FILE),
                teams_token_path: data_dir.join(TEAMS_TOKEN_FILE),
                google_token_path: data_dir.join(GOOGLE_TOKEN_FILE),
                ics_links_path: data_dir.join(ICS_LINKS_FILE),
                meeting_choices_path: data_dir.join(MEETING_CHOICES_FILE),
                sounds_dir: data_dir.join(SOUNDS_DIR),
                write_lock: Mutex::new(()),
            }),
        })
    }

    pub fn teams_token_path(&self) -> PathBuf {
        self.inner.teams_token_path.clone()
    }

    /// The sealed Google Calendar refresh token (read and written by `focus_platform`).
    pub fn google_token_path(&self) -> PathBuf {
        self.inner.google_token_path.clone()
    }

    /// The sealed ICS subscription links (read and written by `crate::calendar` via `focus_platform::secrets`).
    pub fn ics_links_path(&self) -> PathBuf {
        self.inner.ics_links_path.clone()
    }

    /// Stores the meeting choices (`crate::calendar`) atomically.
    pub fn save_meeting_choices<T: Serialize>(&self, choices: &T) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(choices).context("serialize meeting choices")?;
        let _guard = lock(&self.inner.write_lock);
        write_atomic(&self.inner.meeting_choices_path, &json)
    }

    /// The stored meeting choices; `None` when there are none. A corrupted file is deleted.
    pub fn load_meeting_choices<T: DeserializeOwned>(&self) -> Option<T> {
        let text = match fs::read_to_string(&self.inner.meeting_choices_path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return None,
            Err(err) => {
                log::warn!("cannot read meeting choices: {err}");
                return None;
            }
        };
        match serde_json::from_str::<T>(&text) {
            Ok(choices) => Some(choices),
            Err(err) => {
                log::warn!("corrupted meeting choices file, deleting it: {err}");
                let _guard = lock(&self.inner.write_lock);
                if let Err(err) = remove_if_present(&self.inner.meeting_choices_path) {
                    log::warn!("cannot delete meeting choices: {err:#}");
                }
                None
            }
        }
    }

    /// Folder of the imported chime files (`Config::custom_chimes` names files in it). Created on demand
    /// by [`Self::ensure_sounds_dir`].
    pub fn sounds_dir(&self) -> PathBuf {
        self.inner.sounds_dir.clone()
    }

    /// Creates [`Self::sounds_dir`] when it is missing and returns it.
    pub fn ensure_sounds_dir(&self) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(&self.inner.sounds_dir)
            .with_context(|| format!("create {}", self.inner.sounds_dir.display()))?;
        Ok(self.inner.sounds_dir.clone())
    }

    /// `websiteblacklist.txt` in the config directory (user-editable; see `crate::blocklist`).
    pub fn blocklist_path(&self) -> PathBuf {
        self.inner.blocklist_path.clone()
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

    /// Deletes the whole session history (`history.jsonl` and a temp file a crashed write may have left).
    /// A missing file is not an error.
    pub fn clear_history(&self) -> anyhow::Result<()> {
        let _guard = lock(&self.inner.write_lock);
        remove_if_present(&self.inner.history_path)?;
        remove_if_present(&tmp_path(&self.inner.history_path))?;
        Ok(())
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
        if let Err(err) = remove_if_present(&self.inner.restore_path) {
            log::warn!("cannot delete shields restore file: {err:#}");
        }
    }

    /// A notifications restore is pending (`shields-restore.json` exists).
    pub fn has_shields_restore(&self) -> bool {
        self.inner.restore_path.is_file()
    }

    /// Remembers the screen colours to restore if the app dies while the tint is applied.
    pub fn save_tint_restore<T: Serialize>(&self, snapshot: &T) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(snapshot).context("serialize tint restore")?;
        let _guard = lock(&self.inner.write_lock);
        write_atomic(&self.inner.tint_restore_path, &json)
    }

    /// The snapshot left behind by a run that did not restore the screen colours. A corrupted file is
    /// deleted (nothing sensible can be restored from it).
    pub fn load_tint_restore<T: DeserializeOwned>(&self) -> Option<T> {
        let text = match fs::read_to_string(&self.inner.tint_restore_path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return None,
            Err(err) => {
                log::warn!("cannot read tint restore file: {err}");
                return None;
            }
        };
        match serde_json::from_str::<T>(&text) {
            Ok(snapshot) => Some(snapshot),
            Err(err) => {
                log::warn!("corrupted tint restore file, deleting it: {err}");
                self.clear_tint_restore();
                None
            }
        }
    }

    pub fn clear_tint_restore(&self) {
        let _guard = lock(&self.inner.write_lock);
        if let Err(err) = remove_if_present(&self.inner.tint_restore_path) {
            log::warn!("cannot delete tint restore file: {err:#}");
        }
    }

    /// A screen colour restore is pending (`tint-restore.json` exists).
    pub fn has_tint_restore(&self) -> bool {
        self.inner.tint_restore_path.is_file()
    }
}

/// Deletes `path`; a missing file is fine.
fn remove_if_present(path: &Path) -> anyhow::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("delete {}", path.display())),
    }
}

/// `<path>.tmp`, the temp file of [`write_atomic`].
fn tmp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    path.with_file_name(format!("{file_name}.tmp"))
}

/// Writes `bytes` to `<path>.tmp`, syncs it and renames it over `path`.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = tmp_path(path);
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

        let history = root.join("data").join(HISTORY_FILE);
        fs::write(tmp_path(&history), b"x").expect("tmp");
        p.clear_history().expect("clear");
        assert!(p.list_history(10).expect("list").is_empty());
        assert!(!history.exists());
        assert!(!tmp_path(&history).exists());
        // Clearing an empty history is fine too.
        p.clear_history().expect("clear again");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tint_restore_round_trip_and_corruption() {
        let (root, p) = temp_dirs("tint");
        assert!(!p.has_tint_restore());
        assert_eq!(p.load_tint_restore::<Vec<u32>>(), None);
        p.save_tint_restore(&vec![1_u32, 2, 3]).expect("save");
        assert!(p.has_tint_restore());
        assert_eq!(p.load_tint_restore::<Vec<u32>>(), Some(vec![1, 2, 3]));
        fs::write(root.join("data").join(TINT_RESTORE_FILE), b"{ nope").expect("write");
        assert_eq!(p.load_tint_restore::<Vec<u32>>(), None);
        assert!(!p.has_tint_restore(), "a corrupted file is deleted");
        p.clear_tint_restore();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn meeting_choices_round_trip_and_corruption() {
        let (root, p) = temp_dirs("choices");
        assert_eq!(p.load_meeting_choices::<Vec<String>>(), None);
        p.save_meeting_choices(&vec!["a".to_owned()]).expect("save");
        assert_eq!(
            p.load_meeting_choices::<Vec<String>>(),
            Some(vec!["a".to_owned()])
        );
        fs::write(root.join("data").join(MEETING_CHOICES_FILE), b"[1,").expect("write");
        assert_eq!(p.load_meeting_choices::<Vec<String>>(), None);
        assert!(!root.join("data").join(MEETING_CHOICES_FILE).exists());
        assert_eq!(
            p.google_token_path(),
            root.join("data").join(GOOGLE_TOKEN_FILE)
        );
        assert_eq!(p.ics_links_path(), root.join("data").join(ICS_LINKS_FILE));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sounds_dir_is_created_on_demand() {
        let (root, p) = temp_dirs("sounds");
        assert!(!p.sounds_dir().exists());
        let dir = p.ensure_sounds_dir().expect("create");
        assert!(dir.is_dir());
        assert_eq!(dir, p.sounds_dir());
        let _ = fs::remove_dir_all(&root);
    }
}
