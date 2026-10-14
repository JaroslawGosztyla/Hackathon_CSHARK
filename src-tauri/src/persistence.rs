//! All file IO of the app: config, session history, the crash-restore files of the shields (muted
//! notifications) and of the screen tint, the custom chime folder, the meeting choices, the seen work items
//! and the paths of the sealed secrets (Teams and Google tokens, ICS links, work tool tokens; read and
//! written through `focus_platform`).
//! `websiteblacklist.txt` lives next to `config.json`, but `crate::blocklist` reads and watches it.
//!
//! Every method is blocking. Callers on the async runtime use `spawn_blocking`; the shields
//! and Teams workers call it from their own threads. Writes are serialized by one lock; every change of
//! `history.jsonl` additionally holds the history lock for the whole read-modify-write.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use focus_core::api::{AppError, Config, SessionSummary, WorkSource};
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
/// Work items the user has seen in "Co dalej?" (item ids only, no titles).
const WORK_SEEN_FILE: &str = "work-seen.json";
/// The rewards ledger (beta, ADR-022): numbers, day numbers and session ids only.
const REWARDS_FILE: &str = "rewards.json";
/// The previous `rewards.json` (copied before a write, at most once per minute).
const REWARDS_BACKUP_FILE: &str = "rewards.json.bak";
/// A corrupted `rewards.json` is renamed to `rewards.unreadable-<ms>.json`.
const REWARDS_UNREADABLE_PREFIX: &str = "rewards.unreadable-";
/// Folder (in the app data directory) with the user's imported chime files.
const SOUNDS_DIR: &str = "sounds";
/// The weekly stats read at most this much of the end of `history.jsonl` (ADR-023; roughly 30 000 sessions).
const HISTORY_SCAN_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// A `history.jsonl` line longer than this is skipped by the weekly stats (a session line is a few kB).
const HISTORY_SCAN_MAX_LINE_BYTES: usize = 1024 * 1024;

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
    work_seen_path: PathBuf,
    rewards_path: PathBuf,
    rewards_backup_path: PathBuf,
    data_dir: PathBuf,
    sounds_dir: PathBuf,
    write_lock: Mutex<()>,
    /// Held for every change of `history.jsonl` (taken before `write_lock`).
    history_lock: Mutex<()>,
}

/// The files [`Persistence`] keeps, for the privacy report (ADR-020; read-only use there).
#[derive(Debug, Clone)]
pub struct DataFiles {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config: PathBuf,
    pub config_backup: PathBuf,
    pub history: PathBuf,
    pub meeting_choices: PathBuf,
    pub work_seen: PathBuf,
    /// `rewards.json` (beta, ADR-022).
    pub rewards: PathBuf,
    /// `rewards.json.bak`.
    pub rewards_backup: PathBuf,
    pub shields_restore: PathBuf,
    pub tint_restore: PathBuf,
    pub teams_token: PathBuf,
    pub google_token: PathBuf,
    pub ics_links: PathBuf,
    pub sounds: PathBuf,
    pub blocklist: PathBuf,
}

/// Where [`Persistence::mark_goal_done`] found the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalMarked {
    /// The last summary (in memory) is that session.
    pub in_memory: bool,
    /// `history.jsonl` has that session.
    pub in_file: bool,
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

    /// For tests of other modules (e.g. `crate::privacy`): the files in these folders.
    #[cfg(test)]
    pub fn with_test_dirs(config_dir: &Path, data_dir: &Path) -> anyhow::Result<Self> {
        Self::with_dirs(config_dir, data_dir)
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
                work_seen_path: data_dir.join(WORK_SEEN_FILE),
                rewards_path: data_dir.join(REWARDS_FILE),
                rewards_backup_path: data_dir.join(REWARDS_BACKUP_FILE),
                data_dir: data_dir.to_path_buf(),
                sounds_dir: data_dir.join(SOUNDS_DIR),
                write_lock: Mutex::new(()),
                history_lock: Mutex::new(()),
            }),
        })
    }

    /// Every file and folder of this module (the privacy report reads their metadata, ADR-020).
    pub fn files(&self) -> DataFiles {
        let inner = &self.inner;
        DataFiles {
            config_dir: inner
                .config_path
                .parent()
                .map_or_else(|| inner.data_dir.clone(), Path::to_path_buf),
            data_dir: inner.data_dir.clone(),
            config: inner.config_path.clone(),
            config_backup: inner.config_backup_path.clone(),
            history: inner.history_path.clone(),
            meeting_choices: inner.meeting_choices_path.clone(),
            work_seen: inner.work_seen_path.clone(),
            rewards: inner.rewards_path.clone(),
            rewards_backup: inner.rewards_backup_path.clone(),
            shields_restore: inner.restore_path.clone(),
            tint_restore: inner.tint_restore_path.clone(),
            teams_token: inner.teams_token_path.clone(),
            google_token: inner.google_token_path.clone(),
            ics_links: inner.ics_links_path.clone(),
            sounds: inner.sounds_dir.clone(),
            blocklist: inner.blocklist_path.clone(),
        }
    }

    /// Deletes `meeting-choices.json` (privacy panel; the calendar thread forgets its answers at the same time).
    /// A missing file is fine.
    pub fn clear_meeting_choices(&self) -> anyhow::Result<()> {
        let _guard = lock(&self.inner.write_lock);
        remove_if_present(&self.inner.meeting_choices_path)?;
        remove_if_present(&tmp_path(&self.inner.meeting_choices_path))
    }

    /// Deletes `work-seen.json` (privacy panel; the work thread forgets the ids at the same time). A missing file is
    /// fine.
    pub fn clear_work_seen(&self) -> anyhow::Result<()> {
        let _guard = lock(&self.inner.write_lock);
        remove_if_present(&self.inner.work_seen_path)?;
        remove_if_present(&tmp_path(&self.inner.work_seen_path))
    }

    /// Deletes `rewards.json`, its backup, a temp file a crashed write left and the `rewards.unreadable-<ms>.json` copies
    /// of a corrupted ledger. A missing file is fine. Called only by the rewards writer (`crate::rewards`), so a delete
    /// never races a pending write (the privacy panel and "Zacznij od nowa" go through `RewardsHandle`).
    pub fn clear_rewards(&self) -> anyhow::Result<()> {
        let _guard = lock(&self.inner.write_lock);
        remove_if_present(&self.inner.rewards_path)?;
        remove_if_present(&self.inner.rewards_backup_path)?;
        remove_if_present(&tmp_path(&self.inner.rewards_path))?;
        for path in self.unreadable_rewards() {
            remove_if_present(&path)?;
        }
        Ok(())
    }

    /// `rewards.unreadable-<ms>.json` files in the data folder (corrupted ledgers set aside at load).
    fn unreadable_rewards(&self) -> Vec<PathBuf> {
        unreadable_rewards_in(&self.inner.data_dir)
    }

    /// The text of `rewards.json` (beta, ADR-022); `Ok(None)` when there is none.
    pub fn read_rewards(&self) -> std::io::Result<Option<String>> {
        read_optional(&self.inner.rewards_path)
    }

    /// The text of `rewards.json.bak`; `Ok(None)` when there is none.
    pub fn read_rewards_backup(&self) -> std::io::Result<Option<String>> {
        read_optional(&self.inner.rewards_backup_path)
    }

    /// Writes the rewards ledger atomically. With `backup` the previous `rewards.json` is copied to `rewards.json.bak`
    /// first (the rewards writer does that at most once per minute); a failed copy is only logged.
    pub fn save_rewards<T: Serialize>(&self, ledger: &T, backup: bool) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(ledger).context("serialize rewards")?;
        let _guard = lock(&self.inner.write_lock);
        if backup
            && self.inner.rewards_path.is_file()
            && let Err(err) = fs::copy(&self.inner.rewards_path, &self.inner.rewards_backup_path)
        {
            log::warn!("cannot back up rewards.json: {err}");
        }
        write_atomic(&self.inner.rewards_path, &json)
    }

    /// Sets an unreadable `rewards.json` aside as `rewards.unreadable-<ms>.json` (kept for the user, never read again).
    pub fn set_aside_rewards(&self, now_ms: i64) -> anyhow::Result<PathBuf> {
        let _guard = lock(&self.inner.write_lock);
        let target = self
            .inner
            .data_dir
            .join(format!("{REWARDS_UNREADABLE_PREFIX}{now_ms}.json"));
        fs::rename(&self.inner.rewards_path, &target)
            .with_context(|| format!("rename {}", self.inner.rewards_path.display()))?;
        Ok(target)
    }

    /// Deletes `config.json.bak` (the copy of a corrupted config) and a temp file a crashed config write left
    /// ("Delete everything"). A missing file is fine.
    pub fn remove_config_leftovers(&self) -> anyhow::Result<()> {
        let _guard = lock(&self.inner.write_lock);
        remove_if_present(&self.inner.config_backup_path)?;
        remove_if_present(&tmp_path(&self.inner.config_path))
    }

    /// Developer page "Factory reset" (ADR-019), after "Delete everything" stored the defaults: deletes `config.json`
    /// (and its leftovers), so the next start is a first start, and the `sounds` folder when it is empty (it is created
    /// on demand). Files someone else put into `sounds` stay, and so does the folder then. A missing file is fine.
    pub fn remove_for_first_start(&self) -> anyhow::Result<()> {
        let _guard = lock(&self.inner.write_lock);
        remove_if_present(&self.inner.config_path)?;
        remove_if_present(&self.inner.config_backup_path)?;
        remove_if_present(&tmp_path(&self.inner.config_path))?;
        // `remove_dir` only removes an empty folder.
        match fs::remove_dir(&self.inner.sounds_dir) {
            Err(err) if err.kind() != ErrorKind::NotFound => {
                log::info!("factory reset: the sounds folder stays ({err})");
            }
            _ => {}
        }
        Ok(())
    }

    /// The sealed access token of a work tool (read and written by `crate::work` via `focus_platform`).
    pub fn work_token_path(&self, source: WorkSource) -> PathBuf {
        focus_platform::work::token_path_for(&self.inner.data_dir, source)
    }

    /// Stores the ids of the seen work items (`crate::work`) atomically.
    pub fn save_work_seen<T: Serialize>(&self, seen: &T) -> anyhow::Result<()> {
        self.save_json(&self.inner.work_seen_path, seen, "seen work items")
    }

    /// The stored seen work items; `None` when there are none. A corrupted file is deleted.
    pub fn load_work_seen<T: DeserializeOwned>(&self) -> Option<T> {
        self.load_json(&self.inner.work_seen_path, "seen work items")
    }

    /// Writes `value` as pretty JSON to `path` atomically.
    fn save_json<T: Serialize>(&self, path: &Path, value: &T, what: &str) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(value).with_context(|| format!("serialize {what}"))?;
        let _guard = lock(&self.inner.write_lock);
        write_atomic(path, &json)
    }

    /// Reads JSON from `path`; `None` when it is missing or unreadable. A corrupted file is deleted.
    fn load_json<T: DeserializeOwned>(&self, path: &Path, what: &str) -> Option<T> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return None,
            Err(err) => {
                log::warn!("cannot read {what}: {err}");
                return None;
            }
        };
        match serde_json::from_str::<T>(&text) {
            Ok(value) => Some(value),
            Err(err) => {
                log::warn!("corrupted {what} file, deleting it: {err}");
                let _guard = lock(&self.inner.write_lock);
                if let Err(err) = remove_if_present(path) {
                    log::warn!("cannot delete {what}: {err:#}");
                }
                None
            }
        }
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
        self.save_json(&self.inner.meeting_choices_path, choices, "meeting choices")
    }

    /// The stored meeting choices; `None` when there are none. A corrupted file is deleted.
    pub fn load_meeting_choices<T: DeserializeOwned>(&self) -> Option<T> {
        self.load_json(&self.inner.meeting_choices_path, "meeting choices")
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

    /// Loads `config.json`. A missing file is a first start (a fresh install, or the next start after a factory
    /// reset): the defaults with the first-run wizard still to show (`Config::first_run`, ADR-023), silently. A
    /// corrupted or invalid file is renamed to `config.json.bak` and defaults are used (no wizard: the user had a
    /// config), with an error to report to the UI.
    pub fn load_config(&self) -> (Config, Option<AppError>) {
        let text = match fs::read_to_string(&self.inner.config_path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return (Config::first_run(), None),
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

    /// Appends one summary as a JSON line to `history.jsonl`, with the goal answer ("Zrobione?") the user may
    /// already have given for it in `latest` (the last summary in memory, see [`Self::mark_goal_done`]) and the
    /// sparks that answer paid (beta): the append runs on the blocking pool and can come after that answer.
    pub fn append_history(
        &self,
        summary: &SessionSummary,
        latest: &Mutex<Option<SessionSummary>>,
    ) -> anyhow::Result<()> {
        let _history = lock(&self.inner.history_lock);
        let answered = lock(latest)
            .as_ref()
            .filter(|last| last.session_id == summary.session_id)
            .map(|last| (last.goal_done, last.sparks))
            .filter(|answer| *answer != (summary.goal_done, summary.sparks));
        let mut line = match answered {
            Some((goal_done, sparks)) => serde_json::to_vec(&SessionSummary {
                goal_done,
                sparks,
                ..summary.clone()
            }),
            None => serde_json::to_vec(summary),
        }
        .context("serialize summary")?;
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

    /// Streams the stored sessions, oldest first, into `each` (the weekly stats, ADR-023) without holding the file in
    /// memory: at most the newest [`HISTORY_SCAN_MAX_BYTES`] (an older part of a huge file is left out), lines longer
    /// than [`HISTORY_SCAN_MAX_LINE_BYTES`] and unparseable lines are skipped. A missing file is an empty history.
    /// Reads without the history lock: writers append or replace the file whole, so a scan sees the old or the new one.
    pub fn scan_history(&self, mut each: impl FnMut(SessionSummary)) -> anyhow::Result<()> {
        let path = &self.inner.history_path;
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err).with_context(|| format!("open {}", path.display())),
        };
        let len = file
            .metadata()
            .with_context(|| format!("stat {}", path.display()))?
            .len();
        let mut reader = BufReader::new(file);
        // Starting inside a line: its rest is not a whole session.
        let mut skip_first = false;
        if len > HISTORY_SCAN_MAX_BYTES {
            reader
                .seek(SeekFrom::Start(len - HISTORY_SCAN_MAX_BYTES))
                .with_context(|| format!("seek {}", path.display()))?;
            skip_first = true;
        }
        let mut reader = reader.take(HISTORY_SCAN_MAX_BYTES);
        let mut line = Vec::new();
        let (mut skipped, mut too_long) = (0_u32, 0_u32);
        while let Some(fits) =
            read_bounded_line(&mut reader, &mut line, HISTORY_SCAN_MAX_LINE_BYTES)
                .with_context(|| format!("read {}", path.display()))?
        {
            if std::mem::take(&mut skip_first) {
                continue;
            }
            if !fits {
                too_long += 1;
                continue;
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match serde_json::from_slice::<SessionSummary>(&line) {
                Ok(summary) => each(summary),
                Err(_) => skipped += 1,
            }
        }
        if skipped > 0 || too_long > 0 {
            // Counts only: a history line may hold window titles (privacy rule 11).
            log::warn!(
                "weekly stats skipped {skipped} unreadable and {too_long} oversized history lines"
            );
        }
        Ok(())
    }

    /// Marks the goal of session `session_id` as done / not done ("Zrobione?"): in `latest` (the last summary,
    /// in memory) and in `history.jsonl`, whose matching line is rewritten (temp file + rename; the other lines
    /// stay as they are). Serialized with [`Self::append_history`], so an answer given while the session is
    /// still being appended is kept either way. `bonus` = sparks the answer paid (beta, ADR-022; 0 otherwise): added
    /// to the session's `sparks` in both places.
    pub fn mark_goal_done(
        &self,
        session_id: &str,
        done: bool,
        bonus: u32,
        latest: &Mutex<Option<SessionSummary>>,
    ) -> anyhow::Result<GoalMarked> {
        let _history = lock(&self.inner.history_lock);
        let in_memory = match lock(latest)
            .as_mut()
            .filter(|last| last.session_id == session_id)
        {
            Some(last) => {
                last.goal_done = Some(done);
                last.sparks = with_bonus(last.sparks, bonus);
                true
            }
            None => false,
        };
        let path = &self.inner.history_path;
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                return Ok(GoalMarked {
                    in_memory,
                    in_file: false,
                });
            }
            Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
        };
        let (rewritten, in_file) = with_goal_done(&text, session_id, done, bonus);
        if in_file {
            let _guard = lock(&self.inner.write_lock);
            write_atomic(path, rewritten.as_bytes())?;
        }
        Ok(GoalMarked { in_memory, in_file })
    }

    /// Deletes the whole session history (`history.jsonl` and a temp file a crashed write may have left).
    /// A missing file is not an error.
    pub fn clear_history(&self) -> anyhow::Result<()> {
        let _history = lock(&self.inner.history_lock);
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

/// `history.jsonl` text with `goalDone` set on the summary lines of `session_id` (every other line kept as
/// it is, one per line), and whether there was such a line.
/// `sparks` plus `bonus` sparks (unchanged without a bonus).
fn with_bonus(sparks: Option<u32>, bonus: u32) -> Option<u32> {
    if bonus == 0 {
        sparks
    } else {
        Some(sparks.unwrap_or(0).saturating_add(bonus))
    }
}

fn with_goal_done(text: &str, session_id: &str, done: bool, bonus: u32) -> (String, bool) {
    let mut found = false;
    let mut out = String::with_capacity(text.len() + 32);
    for line in text.lines() {
        let updated = Some(line)
            .filter(|line| line.contains(session_id))
            .and_then(|line| serde_json::from_str::<SessionSummary>(line).ok())
            .filter(|summary| summary.session_id == session_id)
            .and_then(|summary| {
                serde_json::to_string(&SessionSummary {
                    goal_done: Some(done),
                    sparks: with_bonus(summary.sparks, bonus),
                    ..summary
                })
                .ok()
            });
        match updated {
            Some(updated) => {
                found = true;
                out.push_str(&updated);
            }
            None => out.push_str(line),
        }
        out.push('\n');
    }
    (out, found)
}

/// `rewards.unreadable-<ms>.json` files in `data_dir`: corrupted ledgers set aside at load (beta, ADR-022). The
/// privacy panel lists them with the rewards item, which deletes them too ([`Persistence::clear_rewards`]).
pub fn unreadable_rewards_in(data_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(data_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(REWARDS_UNREADABLE_PREFIX) && name.ends_with(".json")
                })
        })
        .collect()
}

/// Reads one `\n`-terminated line (without the `\n`) into `line`, keeping at most `max` bytes of it. `Ok(None)` at the
/// end of the input; `Ok(Some(false))` for a longer line, which is read to its end but not kept (`line` is empty).
fn read_bounded_line(
    reader: &mut impl BufRead,
    line: &mut Vec<u8>,
    max: usize,
) -> std::io::Result<Option<bool>> {
    line.clear();
    let mut fits = true;
    let mut read_any = false;
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            return Ok(read_any.then_some(fits));
        }
        read_any = true;
        let newline = buf.iter().position(|byte| *byte == b'\n');
        let part = newline.map_or(buf, |at| &buf[..at]);
        if fits && line.len() + part.len() <= max {
            line.extend_from_slice(part);
        } else {
            fits = false;
            line.clear();
        }
        let used = newline.map_or(buf.len(), |at| at + 1);
        reader.consume(used);
        if newline.is_some() {
            return Ok(Some(fits));
        }
    }
}

/// The text of `path`; `Ok(None)` when it does not exist.
fn read_optional(path: &Path) -> std::io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
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
        // No config.json yet: a first start, with the first-run wizard to show (ADR-023).
        let (cfg, err) = p.load_config();
        assert!(err.is_none());
        assert_eq!(cfg, Config::first_run());
        assert!(!cfg.onboarding_done);

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
    fn first_run_wizard_stays_until_finished_and_never_returns() {
        let (root, p) = temp_dirs("onboarding");
        // A fresh install changes a setting in the wizard before finishing it: the wizard is still due.
        let (fresh, _) = p.load_config();
        let edited = Config {
            focus_minutes: 30,
            ..fresh
        };
        p.save_config(&edited).expect("save");
        let (loaded, err) = p.load_config();
        assert!(err.is_none());
        assert!(!loaded.onboarding_done);
        assert_eq!(loaded.focus_minutes, 30);

        // Finished (or skipped): stored, and never shown again.
        let done = Config {
            onboarding_done: true,
            ..loaded
        };
        p.save_config(&done).expect("save");
        assert!(p.load_config().0.onboarding_done);

        // A config written before ADR-023 has no field: an existing user, no wizard.
        fs::write(
            root.join("config").join(CONFIG_FILE),
            br#"{"focusMinutes":25}"#,
        )
        .expect("write");
        assert!(p.load_config().0.onboarding_done);
        let _ = fs::remove_dir_all(&root);
    }

    fn summary(id: &str, at: i64) -> SessionSummary {
        SessionSummary {
            session_id: id.to_owned(),
            started_at_ms: at,
            ended_at_ms: at + 1,
            focused_secs: 1,
            break_secs: 0,
            cycles_completed: 0,
            usage: Default::default(),
            demo: true,
            goal: Some(focus_core::api::SessionGoal {
                source: WorkSource::Jira,
                key: format!("PROJ-{at}"),
                title: "Goal".to_owned(),
                url: String::new(),
            }),
            goal_done: None,
            steps: Vec::new(),
            sparks: None,
            focus_spans: Vec::new(),
        }
    }

    #[test]
    fn history_is_newest_first() {
        let (root, p) = temp_dirs("history");
        let none = Mutex::new(None);
        for i in 0..3 {
            p.append_history(&summary(&format!("s{i}"), i), &none)
                .expect("append");
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
    fn scan_streams_history_oldest_first_and_skips_bad_lines() {
        let (root, p) = temp_dirs("history-scan");
        let scanned = |p: &Persistence| {
            let mut ids = Vec::new();
            p.scan_history(|s| ids.push(s.session_id)).expect("scan");
            ids
        };
        // No file yet: an empty history.
        assert!(scanned(&p).is_empty());
        let none = Mutex::new(None);
        p.append_history(&summary("s0", 0), &none).expect("append");
        let history = root.join("data").join(HISTORY_FILE);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&history)
            .expect("open");
        file.write_all(b"not json\n\n  \r\n").expect("garbage");
        let long = format!(
            "{{\"x\":\"{}\"}}\n",
            "y".repeat(HISTORY_SCAN_MAX_LINE_BYTES)
        );
        file.write_all(long.as_bytes()).expect("long line");
        drop(file);
        p.append_history(&summary("s1", 1), &none).expect("append");
        assert_eq!(scanned(&p), ["s0", "s1"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bounded_lines_keep_short_lines_and_drop_long_ones() {
        let mut input = std::io::BufReader::with_capacity(4, &b"ab\nabcdef\n\nxyz"[..]);
        let mut line = Vec::new();
        assert_eq!(
            read_bounded_line(&mut input, &mut line, 3).expect("read"),
            Some(true)
        );
        assert_eq!(line, b"ab");
        assert_eq!(
            read_bounded_line(&mut input, &mut line, 3).expect("read"),
            Some(false)
        );
        assert!(line.is_empty());
        assert_eq!(
            read_bounded_line(&mut input, &mut line, 3).expect("read"),
            Some(true)
        );
        assert!(line.is_empty());
        // The last line has no newline.
        assert_eq!(
            read_bounded_line(&mut input, &mut line, 3).expect("read"),
            Some(true)
        );
        assert_eq!(line, b"xyz");
        assert_eq!(
            read_bounded_line(&mut input, &mut line, 3).expect("read"),
            None
        );
    }

    #[test]
    fn goal_done_rewrites_only_its_line() {
        let (root, p) = temp_dirs("goal-done");
        let latest = Mutex::new(None);
        // No history yet: only the summary in memory can be marked.
        let marked = p.mark_goal_done("s1", true, 0, &latest).expect("mark");
        assert_eq!(
            marked,
            GoalMarked {
                in_memory: false,
                in_file: false
            }
        );

        let history = root.join("data").join(HISTORY_FILE);
        p.append_history(&summary("s1", 1), &latest)
            .expect("append");
        // A line this version cannot read stays as it is.
        let mut file = OpenOptions::new()
            .append(true)
            .open(&history)
            .expect("open");
        file.write_all(b"{\"future\":true}\n").expect("write");
        drop(file);
        p.append_history(&summary("s2", 2), &latest)
            .expect("append");

        let marked = p.mark_goal_done("s1", true, 0, &latest).expect("mark");
        assert_eq!(
            marked,
            GoalMarked {
                in_memory: false,
                in_file: true
            }
        );
        let text = fs::read_to_string(&history).expect("read");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "{\"future\":true}");
        let list = p.list_history(10).expect("list");
        let done: Vec<_> = list
            .iter()
            .map(|s| (s.session_id.as_str(), s.goal_done))
            .collect();
        assert_eq!(done, [("s2", None), ("s1", Some(true))]);
        assert_eq!(list[1].goal, summary("s1", 1).goal, "the goal is kept");
        assert!(!tmp_path(&history).exists());

        // Answered while the append is still pending: the append takes the answer from memory.
        *latest.lock().expect("lock") = Some(summary("s3", 3));
        let marked = p.mark_goal_done("s3", false, 0, &latest).expect("mark");
        assert_eq!(
            marked,
            GoalMarked {
                in_memory: true,
                in_file: false
            }
        );
        p.append_history(&summary("s3", 3), &latest)
            .expect("append");
        let newest = p.list_history(1).expect("list");
        assert_eq!(newest[0].goal_done, Some(false));
        // Unknown sessions change nothing.
        let before = fs::read_to_string(&history).expect("read");
        let marked = p.mark_goal_done("nope", true, 0, &latest).expect("mark");
        assert!(!marked.in_memory && !marked.in_file);
        assert_eq!(fs::read_to_string(&history).expect("read"), before);

        // Beta: the sparks the answer paid go into the stored line and the summary in memory.
        *latest.lock().expect("lock") = Some(SessionSummary {
            sparks: Some(40),
            ..summary("s4", 4)
        });
        p.mark_goal_done("s4", true, 10, &latest).expect("mark");
        assert_eq!(
            latest.lock().expect("lock").as_ref().and_then(|s| s.sparks),
            Some(50)
        );
        // The append comes after the answer: it takes both from memory.
        p.append_history(
            &SessionSummary {
                sparks: Some(40),
                ..summary("s4", 4)
            },
            &latest,
        )
        .expect("append");
        let newest = p.list_history(1).expect("list");
        assert_eq!(
            (newest[0].goal_done, newest[0].sparks),
            (Some(true), Some(50))
        );
        // An entry already in the file gets the bonus in its line; without a bonus nothing about sparks changes.
        p.mark_goal_done("s2", true, 10, &latest).expect("mark");
        p.mark_goal_done("s1", false, 0, &latest).expect("mark");
        let sparks: Vec<_> = p
            .list_history(10)
            .expect("list")
            .iter()
            .map(|s| (s.session_id.clone(), s.sparks))
            .collect();
        assert_eq!(
            sparks,
            [
                ("s4".to_owned(), Some(50)),
                ("s3".to_owned(), None),
                ("s2".to_owned(), Some(10)),
                ("s1".to_owned(), None)
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn work_seen_round_trip_and_token_paths() {
        let (root, p) = temp_dirs("work");
        assert_eq!(p.load_work_seen::<Vec<String>>(), None);
        p.save_work_seen(&vec!["github:o/r#1".to_owned()])
            .expect("save");
        assert_eq!(
            p.load_work_seen::<Vec<String>>(),
            Some(vec!["github:o/r#1".to_owned()])
        );
        fs::write(root.join("data").join(WORK_SEEN_FILE), b"{").expect("write");
        assert_eq!(p.load_work_seen::<Vec<String>>(), None);
        assert!(!root.join("data").join(WORK_SEEN_FILE).exists());
        let paths: std::collections::HashSet<_> = [
            WorkSource::Github,
            WorkSource::Gitlab,
            WorkSource::Jira,
            WorkSource::AzureDevops,
            WorkSource::Linear,
        ]
        .into_iter()
        .map(|source| p.work_token_path(source))
        .collect();
        assert_eq!(paths.len(), 5);
        assert!(paths.iter().all(|path| path.starts_with(root.join("data"))));
        assert_eq!(
            p.work_token_path(WorkSource::AzureDevops),
            root.join("data").join("work-azure-devops-token.bin")
        );
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
    fn privacy_clears_only_their_own_files() {
        let (root, p) = temp_dirs("privacy");
        let files = p.files();
        assert_eq!(files.config_dir, root.join("config"));
        assert_eq!(files.data_dir, root.join("data"));
        assert_eq!(files.history, root.join("data").join(HISTORY_FILE));
        p.save_meeting_choices(&vec!["a".to_owned()]).expect("save");
        p.save_work_seen(&vec!["b".to_owned()]).expect("save");
        p.save_config(&Config::default()).expect("save");
        fs::write(&files.config_backup, b"{").expect("write");
        p.clear_meeting_choices().expect("clear");
        assert!(!files.meeting_choices.exists());
        assert!(files.work_seen.exists(), "the other files stay");
        p.clear_work_seen().expect("clear");
        assert!(!files.work_seen.exists());
        p.remove_config_leftovers().expect("remove");
        assert!(!files.config_backup.exists());
        assert!(files.config.exists(), "config.json itself stays");
        // Nothing left to delete is fine.
        p.clear_meeting_choices().expect("again");
        p.clear_work_seen().expect("again");
        p.remove_config_leftovers().expect("again");
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

    #[test]
    fn first_start_removes_the_config_and_an_empty_sounds_folder_only() {
        let (root, p) = temp_dirs("first-start");
        let files = p.files();
        p.save_config(&Config::default()).expect("save");
        fs::write(&files.config_backup, b"{").expect("backup");
        p.append_history(&summary("kept", 1), &Mutex::new(None))
            .expect("history");
        p.ensure_sounds_dir().expect("sounds");
        p.remove_for_first_start().expect("remove");
        assert!(!files.config.exists());
        assert!(!files.config_backup.exists());
        assert!(!files.sounds.exists(), "an empty sounds folder goes");
        assert!(files.history.exists(), "other files are not this helper's");
        // The next start is a first start: the first-run wizard shows again (ADR-023).
        assert!(!p.load_config().0.onboarding_done);

        // A file that is not ours keeps the sounds folder (and the file).
        p.ensure_sounds_dir().expect("sounds");
        fs::write(files.sounds.join("readme.txt"), b"not ours").expect("other");
        p.remove_for_first_start().expect("again");
        assert!(files.sounds.join("readme.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
