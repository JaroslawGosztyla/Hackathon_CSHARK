//! `websiteblacklist.txt`: the user-editable list of distracting sites (ADR-013 pt 4).
//!
//! The file sits in the app config directory next to `config.json`. When it is missing it is created
//! from the bundled default ([`DEFAULT_BLACKLIST`]): at startup and before `open_blocklist_file`
//! opens it. It is loaded at startup, by a watcher thread whenever its modification time or size
//! changes (polled every 2 s), and on demand (`reload_blocklist`); `Blocklist::parse` (focus-core)
//! does the parsing. A different result bumps the `BlocklistInfo` revision, goes to the extension
//! bridge and to the tracker's window-title fallback, and is emitted as `blocklist://changed`.
//! A file that cannot be read keeps the list in use (at startup: the bundled default) and sets
//! `BlocklistInfo.error`, plus `app://error` `blocklist.read`.
//!
//! The browser extension reports which regex sources it could not turn into network rules
//! (`POST /v1/report`, see `crate::bridge`); [`BlocklistService::set_extension_skipped`] keeps the sources of
//! the latest report that are in the list in use as `BlocklistInfo.extension_skipped` (a new revision and
//! `blocklist://changed` when that changes). A reload keeps only those that are still in the list.
//!
//! Privacy (rule 11): only counts are logged, never entries, titles or URLs.

use std::collections::HashSet;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{Duration, SystemTime};

use anyhow::Context;
use focus_core::api::{AppError, BlocklistInfo, BlocklistIssue, events};
use focus_core::blocklist::{BLACKLIST_FILE_NAME, Blocklist, DEFAULT_BLACKLIST};
use tauri::{AppHandle, Emitter};

use crate::bridge::SharedBridge;
use crate::persistence::{Persistence, write_atomic};
use crate::texts::{self, Texts};
use crate::{emit_error, emit_startup_error, lock};

/// The parsed list in use, shared with the tracker (window-title fallback).
pub type SharedBlocklist = Arc<RwLock<Blocklist>>;

/// How often the watcher looks at the file's modification time and size.
const POLL_EVERY: Duration = Duration::from_secs(2);
/// A change must hold still this long before it is read (an editor may still be writing).
const SETTLE: Duration = Duration::from_millis(300);
/// Larger files are not read (the parser keeps at most 2000 domains and 500 regexes anyway).
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const READ_ERROR_CODE: &str = "blocklist.read";

/// Owner of `websiteblacklist.txt`: loads, watches and publishes it. Cheap to clone.
#[derive(Clone)]
pub struct BlocklistService {
    inner: Arc<Inner>,
}

struct Inner {
    app: AppHandle,
    path: PathBuf,
    list: SharedBlocklist,
    bridge: SharedBridge,
    /// Serializes loads (watcher thread and commands), so emitted revisions keep their order.
    reload: Mutex<()>,
    state: Mutex<State>,
}

struct State {
    info: BlocklistInfo,
    /// Modification time and size seen right before the last read.
    stamp: Option<FileStamp>,
    /// The last read failed for a reason that may pass without the file changing (locked, no access).
    retry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

type Loaded = Result<(Blocklist, Vec<BlocklistIssue>), LoadError>;

impl BlocklistService {
    /// Creates the file if it is missing, loads it (the bundled default when it cannot be read),
    /// publishes it to the bridge and starts the watcher thread. Blocking, like loading the config.
    pub fn start(app: &AppHandle, persistence: &Persistence, bridge: SharedBridge) -> Self {
        let path = persistence.blocklist_path();
        let created = create_if_missing(&path);
        let stamp = file_stamp(&path);
        let loaded = match (load(&path), created) {
            (Err(LoadError::Io(err)), Err(create)) if err.kind() == ErrorKind::NotFound => {
                Err(LoadError::Create(create))
            }
            (loaded, created) => {
                if let Err(err) = created {
                    log::warn!("cannot create {BLACKLIST_FILE_NAME}: {err:#}");
                }
                loaded
            }
        };
        let retry = matches!(&loaded, Err(err) if err.is_transient());
        let (list, issues, error) = match loaded {
            Ok((list, issues)) => (list, issues, None),
            Err(err) => {
                emit_startup_error(app, AppError::new(READ_ERROR_CODE, err.english()));
                let (list, issues) = Blocklist::parse(DEFAULT_BLACKLIST);
                (list, issues, Some(err.message(texts::current())))
            }
        };
        let info = BlocklistInfo {
            revision: 1,
            path: path.to_string_lossy().into_owned(),
            domains: list.domains().to_vec(),
            patterns: list.pattern_sources(),
            issues,
            error,
            extension_skipped: Vec::new(),
        };
        log_loaded(&info);
        lock(&bridge).set_blocklist(&list);

        let service = Self {
            inner: Arc::new(Inner {
                app: app.clone(),
                path,
                list: Arc::new(RwLock::new(list)),
                bridge,
                reload: Mutex::new(()),
                state: Mutex::new(State { info, stamp, retry }),
            }),
        };
        service.spawn_watcher();
        service
    }

    /// Current info (for `get_blocklist`).
    pub fn info(&self) -> BlocklistInfo {
        lock(&self.inner.state).info.clone()
    }

    /// The parsed list in use.
    pub fn shared(&self) -> SharedBlocklist {
        Arc::clone(&self.inner.list)
    }

    /// Reads the file now and applies the result; returns the current info (a new revision only
    /// when anything changed). Blocking.
    pub fn reload(&self) -> BlocklistInfo {
        let _serial = lock(&self.inner.reload);
        let stamp = file_stamp(&self.inner.path);
        let loaded = load(&self.inner.path);
        self.apply(stamp, loaded)
    }

    /// Recreates a missing file from the bundled default (and loads it); returns its path. Blocking.
    pub fn ensure_file(&self) -> anyhow::Result<PathBuf> {
        if create_if_missing(&self.inner.path)? {
            self.reload();
        }
        Ok(self.inner.path.clone())
    }

    /// Privacy panel (ADR-020): replaces the file with the bundled default list and loads it (a new revision and
    /// `blocklist://changed` when the list changed). Blocking.
    pub fn restore_default(&self) -> anyhow::Result<BlocklistInfo> {
        {
            // Under the reload lock, so the watcher never reads a half-replaced file between the write and the load.
            let _serial = lock(&self.inner.reload);
            write_atomic(&self.inner.path, DEFAULT_BLACKLIST.as_bytes())?;
        }
        log::info!("{BLACKLIST_FILE_NAME} replaced by the bundled default at the user's request");
        Ok(self.reload())
    }

    /// The latest report of the browser extension: the regex sources it could not turn into network rules
    /// (`POST /v1/report`). Only sources of the list in use are kept, in the list's order and without
    /// repeats. Publishes a new revision (`blocklist://changed`) and returns `true` only when that changed.
    /// Blocking only for as long as a reload runs.
    pub fn set_extension_skipped(&self, reported: &[String]) -> bool {
        let _serial = lock(&self.inner.reload);
        let mut state = lock(&self.inner.state);
        let skipped = kept_skipped(&state.info.patterns, reported);
        if skipped == state.info.extension_skipped {
            return false;
        }
        let next = BlocklistInfo {
            revision: state.info.revision.wrapping_add(1),
            extension_skipped: skipped,
            ..state.info.clone()
        };
        state.info = next.clone();
        drop(state);

        log::info!(
            "browser extension cannot enforce {} of {} patterns as network rules (checked when a tab loads)",
            next.extension_skipped.len(),
            next.patterns.len()
        );
        if let Err(err) = self.inner.app.emit(events::BLOCKLIST_CHANGED, next) {
            log::warn!("cannot emit blocklist info: {err}");
        }
        true
    }

    fn apply(&self, stamp: Option<FileStamp>, loaded: Loaded) -> BlocklistInfo {
        let mut state = lock(&self.inner.state);
        state.stamp = stamp;
        state.retry = matches!(&loaded, Err(err) if err.is_transient());
        let Some(next) = next_info(&state.info, &loaded) else {
            return state.info.clone();
        };
        let new_error = match &loaded {
            Err(err) if next.error != state.info.error => Some(err.english()),
            _ => None,
        };
        if let Ok((list, _issues)) = loaded {
            let mut in_use = self
                .inner
                .list
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            if *in_use != list {
                lock(&self.inner.bridge).set_blocklist(&list);
                *in_use = list;
            }
        }
        state.info = next.clone();
        drop(state);

        log_loaded(&next);
        if let Some(message) = new_error {
            emit_error(&self.inner.app, AppError::new(READ_ERROR_CODE, message));
        }
        if let Err(err) = self.inner.app.emit(events::BLOCKLIST_CHANGED, next.clone()) {
            log::warn!("cannot emit blocklist info: {err}");
        }
        next
    }

    /// Best effort: without the watcher the list still reloads from the settings.
    fn spawn_watcher(&self) {
        let service = self.clone();
        let spawned = std::thread::Builder::new()
            .name("focus-blocklist".into())
            .spawn(move || service.watch());
        if let Err(err) = spawned {
            log::error!("cannot watch {BLACKLIST_FILE_NAME} for changes: {err}");
        }
    }

    /// Reloads whenever the file's modification time or size changes (also when it is deleted or
    /// recreated); a failed read that may pass on its own is retried on every poll.
    fn watch(&self) {
        loop {
            std::thread::sleep(POLL_EVERY);
            let seen = file_stamp(&self.inner.path);
            let (known, retry) = {
                let state = lock(&self.inner.state);
                (state.stamp, state.retry)
            };
            if seen == known && !retry {
                continue;
            }
            // A vanished file gets a full poll: editors that save by delete + rename bring it back.
            std::thread::sleep(if seen.is_none() { POLL_EVERY } else { SETTLE });
            if file_stamp(&self.inner.path) != seen {
                // Still being written: look again at the next poll.
                continue;
            }
            self.reload();
        }
    }
}

/// The info after a load, with the next revision; `None` when nothing changed. Comparing the
/// domains and patterns is the same as comparing the `Blocklist`s (they are its contents). The
/// extension's skipped sources that are no longer in the list are dropped.
fn next_info(current: &BlocklistInfo, loaded: &Loaded) -> Option<BlocklistInfo> {
    let next = match loaded {
        Ok((list, issues)) => {
            let patterns = list.pattern_sources();
            BlocklistInfo {
                revision: current.revision,
                path: current.path.clone(),
                domains: list.domains().to_vec(),
                extension_skipped: kept_skipped(&patterns, &current.extension_skipped),
                patterns,
                issues: issues.clone(),
                error: None,
            }
        }
        // The list in use and its issues stay; only the error is new.
        Err(err) => BlocklistInfo {
            error: Some(err.message(texts::current())),
            ..current.clone()
        },
    };
    (next != *current).then(|| BlocklistInfo {
        revision: current.revision.wrapping_add(1),
        ..next
    })
}

/// The sources of `patterns` that `reported` names, in the order of `patterns`, each once.
fn kept_skipped(patterns: &[String], reported: &[String]) -> Vec<String> {
    let reported: HashSet<&str> = reported.iter().map(String::as_str).collect();
    let mut kept = HashSet::new();
    patterns
        .iter()
        .filter(|source| reported.contains(source.as_str()) && kept.insert(source.as_str()))
        .cloned()
        .collect()
}

#[derive(Debug)]
enum LoadError {
    /// The bundled default could not be written (first run) and there is no file.
    Create(anyhow::Error),
    Io(std::io::Error),
    TooLarge,
}

impl LoadError {
    /// Text for `BlocklistInfo.error` in the UI language (the UI adds that the previous list stays in use).
    fn message(&self, t: &Texts) -> String {
        match self {
            Self::Create(err) => (t.blocklist_create)(&format!("{err:#}")),
            Self::Io(err) if err.kind() == ErrorKind::NotFound => t.blocklist_missing.to_owned(),
            Self::Io(err) if err.kind() == ErrorKind::PermissionDenied => {
                t.blocklist_denied.to_owned()
            }
            Self::Io(err) => (t.blocklist_io)(&err.to_string()),
            Self::TooLarge => (t.blocklist_too_large)(MAX_FILE_BYTES / (1024 * 1024)),
        }
    }

    /// English detail for `app://error` and the log.
    fn english(&self) -> String {
        match self {
            Self::Create(err) => format!("cannot create {BLACKLIST_FILE_NAME}: {err:#}"),
            Self::Io(err) => format!("cannot read {BLACKLIST_FILE_NAME}: {err}"),
            Self::TooLarge => {
                format!("{BLACKLIST_FILE_NAME} is larger than {MAX_FILE_BYTES} bytes")
            }
        }
    }

    /// May pass without the file changing (locked by another program, no access).
    fn is_transient(&self) -> bool {
        matches!(self, Self::Io(err) if err.kind() != ErrorKind::NotFound)
    }
}

/// Writes the bundled default unless the file exists; `Ok(true)` when it was created.
fn create_if_missing(path: &Path) -> anyhow::Result<bool> {
    let exists = path
        .try_exists()
        .with_context(|| format!("check {}", path.display()))?;
    if exists {
        return Ok(false);
    }
    write_atomic(path, DEFAULT_BLACKLIST.as_bytes())?;
    log::info!("{BLACKLIST_FILE_NAME} created from the bundled default");
    Ok(true)
}

fn load(path: &Path) -> Loaded {
    read_text(path).map(|text| Blocklist::parse(&text))
}

fn read_text(path: &Path) -> Result<String, LoadError> {
    let file = File::open(path).map_err(LoadError::Io)?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(LoadError::Io)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(LoadError::TooLarge);
    }
    Ok(decode_text(bytes))
}

/// UTF-8 (the parser strips a BOM) or UTF-16 with a BOM (Notepad's "Unicode" encodings).
/// Invalid bytes become U+FFFD, which only spoils the line they are in.
fn decode_text(bytes: Vec<u8>) -> String {
    fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
        let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().copied().map(unit).collect();
        String::from_utf16_lossy(&units)
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(rest, u16::from_le_bytes);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(rest, u16::from_be_bytes);
    }
    String::from_utf8(bytes)
        .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
}

/// `None` when the file is missing or its metadata cannot be read.
fn file_stamp(path: &Path) -> Option<FileStamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        modified: meta.modified().ok(),
        len: meta.len(),
    })
}

fn log_loaded(info: &BlocklistInfo) {
    log::info!(
        "{BLACKLIST_FILE_NAME} loaded (revision {}): {} domains, {} patterns, {} skipped lines{}",
        info.revision,
        info.domains.len(),
        info.patterns.len(),
        info.issues.len(),
        if info.error.is_some() {
            ", read failed (previous list kept)"
        } else {
            ""
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::api::Language;

    fn pl() -> &'static Texts {
        texts::for_language(Language::Pl)
    }

    fn en() -> &'static Texts {
        texts::for_language(Language::En)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("focusmode-blocklist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn info(revision: u32, text: &str) -> BlocklistInfo {
        let (list, issues) = Blocklist::parse(text);
        BlocklistInfo {
            revision,
            path: "websiteblacklist.txt".into(),
            domains: list.domains().to_vec(),
            patterns: list.pattern_sources(),
            issues,
            error: None,
            extension_skipped: Vec::new(),
        }
    }

    fn not_found() -> LoadError {
        LoadError::Io(std::io::Error::from(ErrorKind::NotFound))
    }

    #[test]
    fn creates_the_default_once_and_reads_it() {
        let dir = temp_dir("create");
        let path = dir.join(BLACKLIST_FILE_NAME);
        assert!(create_if_missing(&path).expect("create"));
        assert_eq!(read_text(&path).expect("read"), DEFAULT_BLACKLIST);
        assert!(!dir.join(format!("{BLACKLIST_FILE_NAME}.tmp")).exists());

        std::fs::write(&path, "kwejk.pl\n").expect("edit");
        assert!(!create_if_missing(&path).expect("exists"));
        let (list, issues) = load(&path).expect("load");
        assert_eq!(list.domains(), ["kwejk.pl"]);
        assert!(issues.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_and_oversized_files_are_errors() {
        let dir = temp_dir("errors");
        let path = dir.join(BLACKLIST_FILE_NAME);
        assert!(file_stamp(&path).is_none());
        let missing = read_text(&path).expect_err("missing");
        assert!(matches!(&missing, LoadError::Io(err) if err.kind() == ErrorKind::NotFound));
        assert!(!missing.is_transient());
        assert!(missing.message(pl()).contains("Pliku nie ma"));

        let size = usize::try_from(MAX_FILE_BYTES).expect("fits") + 1;
        std::fs::write(&path, vec![b'#'; size]).expect("write");
        assert!(matches!(read_text(&path), Err(LoadError::TooLarge)));
        assert!(file_stamp(&path).is_some_and(|stamp| stamp.len == MAX_FILE_BYTES + 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decodes_utf8_and_utf16() {
        assert_eq!(decode_text(b"kwejk.pl\n".to_vec()), "kwejk.pl\n");
        let text = "# Memy żółte\nkwejk.pl\n";
        let le: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode_text(le), text);
        let be: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
            .collect();
        assert_eq!(decode_text(be), text);
        // Windows-1250 "ż" in a comment does not break the domains.
        let ansi = b"# \xBF\nreddit.com\n".to_vec();
        let (list, issues) = Blocklist::parse(&decode_text(ansi));
        assert_eq!(list.domains(), ["reddit.com"]);
        assert!(issues.is_empty());
    }

    #[test]
    fn next_info_bumps_the_revision_only_on_change() {
        let current = info(7, "kwejk.pl\n");
        assert_eq!(
            next_info(&current, &Ok(Blocklist::parse("# x\nKWEJK.pl\n"))),
            None
        );

        let next = next_info(
            &current,
            &Ok(Blocklist::parse("kwejk.pl\nyoutube\\.com/shorts\n")),
        )
        .expect("changed");
        assert_eq!(next.revision, 8);
        assert_eq!(next.patterns, [r"youtube\.com/shorts"]);
        assert_eq!(next.path, current.path);

        let with_issue =
            next_info(&current, &Ok(Blocklist::parse("kwejk.pl\nreddit\n"))).expect("issue added");
        assert_eq!(with_issue.domains, current.domains);
        assert_eq!(with_issue.issues.len(), 1);

        let wrapped = next_info(&info(u32::MAX, "a.com"), &Ok(Blocklist::parse("b.com")));
        assert_eq!(wrapped.map(|i| i.revision), Some(0));
    }

    #[test]
    fn read_errors_keep_the_list_in_use() {
        let current = info(3, "kwejk.pl\nreddit\n");
        let failed = next_info(&current, &Err(not_found())).expect("error is new");
        assert_eq!(failed.revision, 4);
        assert_eq!(failed.domains, current.domains);
        assert_eq!(failed.issues, current.issues);
        assert!(failed.error.is_some());
        // The same error again changes nothing; a successful read clears it.
        assert_eq!(next_info(&failed, &Err(not_found())), None);
        let fixed =
            next_info(&failed, &Ok(Blocklist::parse("kwejk.pl\nreddit\n"))).expect("error cleared");
        assert_eq!(fixed.error, None);
        assert_eq!(fixed.revision, 5);
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|&item| item.to_owned()).collect()
    }

    #[test]
    fn only_skipped_sources_of_the_list_in_use_are_kept() {
        let patterns = strings(&[r"a\d+", r"youtube\.com/shorts", r"c\d+"]);
        // List order, no repeats, unknown sources dropped (a report for an older list, or a forged one).
        assert_eq!(
            kept_skipped(&patterns, &strings(&[r"c\d+", "zzz", r"a\d+", r"c\d+"])),
            strings(&[r"a\d+", r"c\d+"])
        );
        assert!(kept_skipped(&patterns, &[]).is_empty());
        assert!(kept_skipped(&[], &strings(&[r"a\d+"])).is_empty());
        // Exact match only: the extension reports the sources exactly as the bridge sent them.
        assert!(kept_skipped(&patterns, &strings(&[r"A\d+", r" a\d+"])).is_empty());
    }

    #[test]
    fn a_reload_keeps_the_skipped_sources_still_in_the_list() {
        let text = "/a\\d+/\n/c\\d+/\nkwejk.pl\n";
        let base = info(2, text);
        assert_eq!(base.patterns.len(), 2);
        let current = BlocklistInfo {
            extension_skipped: base.patterns.clone(),
            ..base
        };
        // Unchanged file: nothing new.
        assert_eq!(next_info(&current, &Ok(Blocklist::parse(text))), None);
        // The second pattern left the file: it leaves the report too.
        let next =
            next_info(&current, &Ok(Blocklist::parse("/a\\d+/\nkwejk.pl\n"))).expect("changed");
        assert_eq!(next.extension_skipped, current.patterns[..1]);
        assert_eq!(next.revision, 3);
        // A read error keeps everything in use, the report included.
        let failed = next_info(&current, &Err(not_found())).expect("error is new");
        assert_eq!(failed.extension_skipped, current.extension_skipped);
    }

    #[test]
    fn error_texts() {
        let denied = LoadError::Io(std::io::Error::from(ErrorKind::PermissionDenied));
        assert!(denied.is_transient());
        assert_eq!(denied.message(pl()), "Brak dostępu do pliku.");
        assert_eq!(denied.message(en()), "No access to the file.");
        assert!(
            denied
                .english()
                .starts_with("cannot read websiteblacklist.txt")
        );
        assert_eq!(
            LoadError::TooLarge.message(pl()),
            "Plik jest za duży (limit 1 MB)."
        );
        assert!(!LoadError::TooLarge.is_transient());
        let create = LoadError::Create(anyhow::anyhow!("disk full"));
        assert!(create.message(pl()).contains("disk full"));
        assert!(create.message(en()).contains("disk full"));
        assert!(!create.is_transient());
    }
}
