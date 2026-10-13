//! Privacy panel (ADR-020, ARCHITECTURE §6.12): what FocusMode stores on this computer, the export without secrets
//! and the deletes.
//!
//! * **Report** (`get_privacy_report` + `privacy://changed`, own revision): one `PrivacyItem` per `PrivacyItemKind`,
//!   from file metadata only (presence, size, a count where it makes sense). DPAPI files (tokens, ICS links) and the
//!   browser profile are reported by presence and size, never opened; the history, the rewards ledger (beta,
//!   ADR-022), the meeting answers and the seen work items are parsed for their counts only. Built on a blocking thread and published only when it differs.
//! * **Refresh**: the "focus-privacy" thread rescans every [`POLL_EVERY`] (metadata only: the JSON files are parsed
//!   again only when their size or modification time changed, the WebView2 profile is walked at most every
//!   [`BROWSER_EVERY`] and with limits), and [`DEBOUNCE`] after [`PrivacyHandle::changed`]: the events of the modules
//!   that store something (calendar, work, Teams, blocklist, tint, shields, summary, YouTube account) and every
//!   delete or export. So the panel also follows files the user removed by hand in the folder.
//! * **Deletes** (`delete_privacy_data`, `delete_all_data`): one at a time, `busy` while they run. A running session
//!   is stopped first and the shields are awaited (rule 7: Windows settings come back before anything goes); then
//!   every item goes through the module that owns it (history like `clear_history`, tokens like the disconnects,
//!   meeting answers and seen items through their threads, the blocklist back to the bundled default, the browser
//!   profile through WebView2). An item that fails does not stop the others; the call then fails with
//!   `privacy.delete`. The restore files of the shields and the tint are never deleted here.
//! * **Export** (`export_privacy_data`): one JSON file (`focus_core::privacy::Export`): the config without secrets,
//!   the history, the meeting answers, the seen items and the rewards ledger (numbers only, when it exists). Never tokens, ICS links, cookies, sounds or logs.
//!
//! Privacy (rule 11): only kinds and counts are logged; never file contents, titles, tokens or links.

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use focus_core::api::{
    AppError, CalendarSource, ChimeSound, Config, CustomChimes, MediaAction, Phase, PrivacyItem,
    PrivacyItemKind, PrivacyReport, SessionSummary, events,
};
use focus_core::config::is_chime_file_name;
use focus_core::privacy::{
    ERR_DELETE, ERR_EXPORT, ERR_OPEN_FOLDER, Export, deletion_plan, everything, includes_titles,
    size_kib,
};
use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::blocklist::BlocklistService;
use crate::calendar::CalendarHandle;
use crate::persistence::{DataFiles, Persistence, write_atomic};
use crate::session_actor::ActorCmd;
use crate::{AppState, MAIN_WINDOW, chimes, lock, read_config};

/// The report is rebuilt this often without any trigger (metadata only).
const POLL_EVERY: Duration = Duration::from_secs(3);
/// A trigger waits this long, so a burst of changes is one rescan.
const DEBOUNCE: Duration = Duration::from_millis(400);
/// The WebView2 profile is walked at most this often (unless it was just cleared).
const BROWSER_EVERY: Duration = Duration::from_secs(30);
/// WebView2 clears its data asynchronously: after a clear, every scan walks the profile for this long.
const BROWSER_SETTLE: Duration = Duration::from_secs(15);
/// The walk of the WebView2 profile gives up (size unknown) after this many entries…
const BROWSER_MAX_ENTRIES: usize = 20_000;
/// …or after this long.
const BROWSER_MAX_TIME: Duration = Duration::from_secs(2);
/// WebView2's folder in the app local data directory (`EBWebView`, created by WebView2 itself).
const WEBVIEW_DIR: &str = "EBWebView";
/// Prefix of imported chime files (`crate::chimes`).
const CUSTOM_CHIME_PREFIX: &str = "custom-";
/// A delete waits at most this long for the stopped session to lower its shields.
const STOP_TIMEOUT: Duration = Duration::from_secs(8);
const STOP_POLL: Duration = Duration::from_millis(100);
/// After Stop the session's summary is appended to `history.jsonl` on the blocking pool: give it this long, so a
/// history delete that follows also covers that session.
const HISTORY_SETTLE: Duration = Duration::from_millis(500);
/// Extension of the export file when the save dialog returned a name without one.
const EXPORT_EXTENSION: &str = "json";
/// The report could not be built (the scan task failed).
const ERR_REPORT: &str = "privacy.report";

pub struct PrivacyContext {
    pub app: AppHandle,
    pub persistence: Persistence,
    /// Number of ICS links (its status shows it), without opening the sealed file here.
    pub calendar: CalendarHandle,
    /// Number of blocklist entries.
    pub blocklist: BlocklistService,
}

/// Cheap to clone.
#[derive(Clone)]
pub struct PrivacyHandle {
    inner: Arc<Inner>,
}

struct Inner {
    app: AppHandle,
    folders: Folders,
    calendar: CalendarHandle,
    blocklist: BlocklistService,
    /// Serializes scans (the thread and the getter) and keeps their caches.
    scanner: Mutex<Scanner>,
    published: Mutex<Published>,
    /// Deletes / exports running now (`PrivacyReport.busy`).
    busy: AtomicU32,
    /// One delete or export at a time.
    ops: Arc<tokio::sync::Mutex<()>>,
    /// The browser profile was cleared: every scan walks it again until then (WebView2 clears asynchronously), and
    /// once more after.
    browser_recheck_until: Mutex<Option<Instant>>,
    wake: Sender<()>,
}

#[derive(Default)]
struct Published {
    report: Option<PrivacyReport>,
    revision: u32,
}

/// Where the data lives.
struct Folders {
    files: DataFiles,
    /// `app_local_data_dir/EBWebView`.
    webview: Option<PathBuf>,
    /// `app_log_dir`.
    logs: Option<PathBuf>,
    /// The log file the logger writes to now (truncated, never deleted).
    active_log: Option<PathBuf>,
    /// `PrivacyReport.folders`: the first one is opened by `open_data_folder`.
    list: Vec<PathBuf>,
}

impl PrivacyHandle {
    /// Resolves the folders, starts the "focus-privacy" thread and listens to the events of the modules that store
    /// something.
    pub fn spawn(ctx: PrivacyContext) -> anyhow::Result<Self> {
        let paths = ctx.app.path();
        let files = ctx.persistence.files();
        let local = paths.app_local_data_dir().ok();
        let logs = paths.app_log_dir().ok();
        let active_log = logs
            .as_ref()
            .map(|dir| dir.join(format!("{}.log", ctx.app.package_info().name)));
        let list = distinct_folders(
            [
                Some(files.data_dir.clone()),
                Some(files.config_dir.clone()),
                local.clone(),
                logs.clone(),
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let folders = Folders {
            files,
            webview: local.map(|dir| dir.join(WEBVIEW_DIR)),
            logs,
            active_log,
            list,
        };
        let (wake, rx) = mpsc::channel();
        let handle = Self {
            inner: Arc::new(Inner {
                app: ctx.app.clone(),
                folders,
                calendar: ctx.calendar,
                blocklist: ctx.blocklist,
                scanner: Mutex::new(Scanner::default()),
                published: Mutex::new(Published::default()),
                busy: AtomicU32::new(0),
                ops: Arc::new(tokio::sync::Mutex::new(())),
                browser_recheck_until: Mutex::new(None),
                wake,
            }),
        };
        for event in [
            events::CALENDAR_CHANGED,
            events::WORK_CHANGED,
            events::TEAMS_STATUS,
            events::BLOCKLIST_CHANGED,
            events::TINT_CHANGED,
            events::SHIELDS_STATUS,
            events::SESSION_SUMMARY,
            events::YOUTUBE_ACCOUNT,
        ] {
            let this = handle.clone();
            ctx.app.listen_any(event, move |_| this.changed());
        }
        let this = handle.clone();
        std::thread::Builder::new()
            .name("focus-privacy".into())
            .spawn(move || this.run(&rx))?;
        Ok(handle)
    }

    /// Something stored may have changed: rescan soon (non-blocking).
    pub fn changed(&self) {
        let _ = self.inner.wake.send(());
    }

    /// A fresh report (`get_privacy_report`): scanned now on the blocking pool, published when it changed.
    pub async fn report(&self) -> Result<PrivacyReport, AppError> {
        let this = self.clone();
        tauri::async_runtime::spawn_blocking(move || this.rescan(false))
            .await
            .map_err(|err| AppError::new(ERR_REPORT, format!("scan failed: {err}")))
    }

    /// The folder `open_data_folder` opens (settings, history, tokens, sounds).
    fn primary_folder(&self) -> PathBuf {
        self.inner
            .folders
            .list
            .first()
            .cloned()
            .unwrap_or_else(|| self.inner.folders.files.data_dir.clone())
    }

    fn run(&self, rx: &Receiver<()>) {
        loop {
            match rx.recv_timeout(POLL_EVERY) {
                Ok(()) => {
                    std::thread::sleep(DEBOUNCE);
                    while rx.try_recv().is_ok() {}
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.rescan(false);
        }
    }

    /// Scans (blocking) and publishes the result when it differs; returns the current report.
    fn rescan(&self, force_browser: bool) -> PrivacyReport {
        let live = self.live();
        let force_browser = self.browser_recheck_due() || force_browser;
        // Held until published, so a slower scan never overwrites a newer one (lock order: scanner, published).
        let mut scanner = lock(&self.inner.scanner);
        let items = scanner.scan(&self.inner.folders, &live, force_browser);
        let report = self.publish(Some(items));
        drop(scanner);
        report
    }

    /// The browser profile must be walked now (it was cleared lately): `true` until [`BROWSER_SETTLE`] after the
    /// clear, and once more after it, so the settled size is shown.
    fn browser_recheck_due(&self) -> bool {
        let mut until = lock(&self.inner.browser_recheck_until);
        match *until {
            Some(at) if Instant::now() < at => true,
            Some(_) => {
                *until = None;
                true
            }
            None => false,
        }
    }

    /// Counts the other modules keep in memory.
    fn live(&self) -> Live {
        let ics_links = self
            .inner
            .calendar
            .status()
            .sources
            .iter()
            .find(|source| source.source == CalendarSource::Ics)
            .and_then(|source| source.account.as_deref())
            .and_then(|count| count.parse().ok());
        let info = self.inner.blocklist.info();
        let blocklist_entries =
            u32::try_from(info.domains.len().saturating_add(info.patterns.len()))
                .unwrap_or(u32::MAX);
        Live {
            ics_links,
            blocklist_entries,
        }
    }

    /// Emits a new report when the items (`None`: the last ones) or `busy` changed; returns the current one.
    fn publish(&self, items: Option<Vec<PrivacyItem>>) -> PrivacyReport {
        let busy = self.inner.busy.load(Ordering::SeqCst) > 0;
        let mut published = lock(&self.inner.published);
        let Some(items) = items.or_else(|| published.report.as_ref().map(|r| r.items.clone()))
        else {
            // Nothing scanned yet: the first scan publishes (with `busy`).
            return PrivacyReport {
                revision: published.revision,
                folders: self.folder_strings(),
                items: Vec::new(),
                busy,
            };
        };
        let mut next = PrivacyReport {
            revision: published.revision,
            folders: self.folder_strings(),
            items,
            busy,
        };
        if published.report.as_ref() == Some(&next) {
            return next;
        }
        published.revision = published.revision.wrapping_add(1);
        next.revision = published.revision;
        published.report = Some(next.clone());
        // Under the lock: revisions go out in order.
        if let Err(err) = self.inner.app.emit(events::PRIVACY_CHANGED, next.clone()) {
            log::warn!("cannot emit the privacy report: {err}");
        }
        next
    }

    fn folder_strings(&self) -> Vec<String> {
        self.inner
            .folders
            .list
            .iter()
            .map(|dir| dir.to_string_lossy().into_owned())
            .collect()
    }

    /// Starts a delete or an export: waits for the one running, then `busy: true` is published.
    async fn begin(&self) -> Operation {
        let guard = Arc::clone(&self.inner.ops).lock_owned().await;
        self.inner.busy.fetch_add(1, Ordering::SeqCst);
        self.publish(None);
        Operation {
            handle: self.clone(),
            _guard: guard,
            ended: false,
        }
    }
}

/// A running delete / export (`busy`). [`Operation::end`] publishes the fresh report; dropped without it (an early
/// error), it still clears `busy`.
struct Operation {
    handle: PrivacyHandle,
    _guard: tokio::sync::OwnedMutexGuard<()>,
    ended: bool,
}

impl Operation {
    /// Rescans (with the browser profile when `browser`) and publishes the report with `busy: false`.
    async fn end(mut self, browser: bool) {
        self.ended = true;
        let handle = self.handle.clone();
        handle.inner.busy.fetch_sub(1, Ordering::SeqCst);
        let scanned = tauri::async_runtime::spawn_blocking(move || handle.rescan(browser)).await;
        if let Err(err) = scanned {
            log::warn!("privacy: cannot rescan after the operation: {err}");
            self.handle.publish(None);
        }
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        if !self.ended {
            self.handle.inner.busy.fetch_sub(1, Ordering::SeqCst);
            self.handle.publish(None);
            self.handle.changed();
        }
    }
}

// -- commands -------------------------------------------------------------------------------------------

/// `open_data_folder`.
pub async fn open_data_folder(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    let folder = state.privacy.primary_folder();
    let app = app.clone();
    let open_error = |detail: String| AppError::new(ERR_OPEN_FOLDER, detail);
    tauri::async_runtime::spawn_blocking(move || {
        fs::create_dir_all(&folder)
            .map_err(|err| open_error(format!("cannot create the folder: {err}")))?;
        app.opener()
            .open_path(folder.to_string_lossy(), None::<&str>)
            .map_err(|err| open_error(format!("cannot open the folder: {err}")))
    })
    .await
    .map_err(|err| open_error(err.to_string()))?
}

/// `export_privacy_data { path }`.
pub async fn export(state: &AppState, path: &str) -> Result<(), AppError> {
    let target = export_target(path, &state.privacy.inner.folders.list)?;
    let op = state.privacy.begin().await;
    let config = read_config(&state.config);
    let files = state.persistence.files();
    let written =
        tauri::async_runtime::spawn_blocking(move || write_export(&files, &config, &target))
            .await
            .map_err(|err| anyhow::anyhow!("export task failed: {err}"))
            .and_then(|result| result);
    op.end(false).await;
    match written {
        Ok(stats) => {
            log::info!(
                "privacy: data exported ({} sessions, meeting answers {}, seen items {}, rewards {})",
                stats.sessions,
                if stats.meeting_choices { "yes" } else { "no" },
                if stats.work_seen { "yes" } else { "no" },
                if stats.rewards { "yes" } else { "no" }
            );
            Ok(())
        }
        Err(err) => Err(AppError::new(
            ERR_EXPORT,
            format!("cannot write the export: {err:#}"),
        )),
    }
}

/// `delete_privacy_data { kinds }`: returns the stored config.
pub async fn delete(
    app: &AppHandle,
    state: &AppState,
    kinds: &[PrivacyItemKind],
) -> Result<Config, AppError> {
    let plan = deletion_plan(kinds)?;
    if plan.is_empty() {
        return Ok(read_config(&state.config));
    }
    let op = state.privacy.begin().await;
    log::info!("privacy: deleting {plan:?} at the user's request");
    stop_session(state).await;
    let failures = run_plan(app, state, &plan).await;
    op.end(plan.contains(&PrivacyItemKind::BrowserData)).await;
    outcome(failures, read_config(&state.config))
}

/// `delete_all_data`: every deletable item, then the default settings (returned).
pub async fn delete_all(app: &AppHandle, state: &AppState) -> Result<Config, AppError> {
    let op = state.privacy.begin().await;
    log::info!("privacy: deleting everything at the user's request");
    stop_session(state).await;
    let mut failures = run_plan(app, state, &everything()).await;
    // Saved like `update_config`: the tint goes off (colours restored), the autostart entry goes with
    // `launchOnLogin`, the first X asks again, Teams / calendars / tools follow their (empty) settings.
    let stored = {
        let _serial = state.config_update.lock().await;
        crate::commands::store_config(app, state, Config::default()).await
    };
    if let Err(err) = stored {
        failures.push(format!("{:?}: {}", PrivacyItemKind::Settings, err.message));
    }
    // `store_config` removes the autostart entry only when `launchOnLogin` changed; an entry left on while the
    // config said off (the UI writes the entry first, the config later) goes too: the defaults have it off.
    remove_autostart_entry(app);
    let persistence = state.persistence.clone();
    let leftovers =
        tauri::async_runtime::spawn_blocking(move || persistence.remove_config_leftovers())
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
            .and_then(|result| result);
    if let Err(err) = leftovers {
        failures.push(format!("{:?}: {err:#}", PrivacyItemKind::Settings));
    }
    op.end(true).await;
    outcome(failures, read_config(&state.config))
}

/// Developer page "Factory reset" (`dev_trigger`, ADR-019): [`delete_all`] (the same code path: the session stops and
/// its shields come down first, the tint goes off with the default settings), then what a first start does not have
/// yet ([`Persistence::remove_for_first_start`]: `config.json` and an empty `sounds` folder), then a restart through
/// the normal exit path (session stopped, shields and screen colours restored, `crate::shutdown`) that comes back with
/// the main window like a first launch (`crate::request_fresh_restart`: without `--autostart` / `--restore-shields`).
///
/// `websiteblacklist.txt` was just reset to the bundled default, which is what a first start writes. The restore files
/// of the shields and the tint are left to their workers, which delete them once Windows is restored (rule 7): one
/// still pending at the restart is completed by the next start. Nothing outside the app's own folders is touched (the
/// installed app, the browser extension folder and the music folders stay). A failed delete rejects with
/// `privacy.delete` and the app does not restart, so the error can be read.
pub async fn factory_reset(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    log::info!("developer mode: factory reset requested");
    delete_all(app, state).await?;
    let persistence = state.persistence.clone();
    blocking(move || persistence.remove_for_first_start()).await?;
    log::info!("developer mode: factory reset done, restarting");
    crate::request_fresh_restart(app);
    Ok(())
}

/// The app's own autostart entry (Windows `Run` value) is removed when it is still there.
fn remove_autostart_entry(app: &AppHandle) {
    let enabled = app
        .try_state::<tauri_plugin_autostart::AutoLaunchManager>()
        .is_some_and(|manager| manager.is_enabled().unwrap_or(false));
    if enabled && !Config::default().launch_on_login {
        crate::sync_autostart(app, false);
    }
}

/// The config, or `privacy.delete` naming every item that failed.
fn outcome(failures: Vec<String>, config: Config) -> Result<Config, AppError> {
    if failures.is_empty() {
        return Ok(config);
    }
    Err(AppError::new(
        ERR_DELETE,
        format!("some data could not be deleted: {}", failures.join("; ")),
    ))
}

/// Stops a running session (like Stop in the UI) and waits, bounded, until its shields are down (rule 7) and its
/// summary had time to reach the history.
async fn stop_session(state: &AppState) {
    let running = match state.actor.snapshot().await {
        Ok(snapshot) => snapshot.phase != Phase::Idle,
        Err(err) => {
            log::warn!("privacy: cannot read the session: {}", err.message);
            false
        }
    };
    if !running {
        return;
    }
    log::info!("privacy: stopping the running session first");
    if let Err(err) = state.actor.send(ActorCmd::Stop) {
        log::warn!("privacy: cannot stop the session: {}", err.message);
        return;
    }
    let deadline = Instant::now() + STOP_TIMEOUT;
    loop {
        let idle = state
            .actor
            .snapshot()
            .await
            .is_ok_and(|snapshot| snapshot.phase == Phase::Idle);
        if idle && !state.shields.status().active {
            break;
        }
        if Instant::now() >= deadline {
            log::warn!(
                "privacy: the shields did not come down within {STOP_TIMEOUT:?}; deleting anyway"
            );
            break;
        }
        tokio::time::sleep(STOP_POLL).await;
    }
    tokio::time::sleep(HISTORY_SETTLE).await;
}

/// Deletes every kind of `plan` (in its order); returns the failures (English, for the error and the log).
async fn run_plan(app: &AppHandle, state: &AppState, plan: &[PrivacyItemKind]) -> Vec<String> {
    if plan.contains(&PrivacyItemKind::BrowserData) {
        close_browser_windows(app, state).await;
    }
    let mut failures = Vec::new();
    for &kind in plan {
        match delete_one(app, state, kind).await {
            Ok(()) => log::info!("privacy: {kind:?} deleted"),
            Err(err) => {
                log::warn!("privacy: cannot delete {kind:?}: {}", err.message);
                failures.push(format!("{kind:?}: {}", err.message));
            }
        }
    }
    failures
}

async fn delete_one(
    app: &AppHandle,
    state: &AppState,
    kind: PrivacyItemKind,
) -> Result<(), AppError> {
    if let Some(source) = kind.work_source() {
        return state.work.set_token(source, None).await.map(drop);
    }
    match kind {
        PrivacyItemKind::History => {
            let persistence = state.persistence.clone();
            blocking(move || persistence.clear_history()).await?;
            *lock(&state.last_summary) = None;
            // The weekly stats ask again (ADR-023).
            crate::weekly::changed(app);
            Ok(())
        }
        PrivacyItemKind::Rewards => {
            // Like "Zacznij od nowa" (`reset_rewards`), through the rewards writer so no queued write brings the file
            // back; no fresh file is written (the item stays gone until the next spark).
            let rewards = state.rewards.clone();
            let config = read_config(&state.config);
            blocking(move || rewards.delete_files(&config)).await
        }
        PrivacyItemKind::MeetingChoices => state.calendar.clear_choices().await,
        PrivacyItemKind::WorkSeen => state.work.clear_seen().await,
        // Teams presence was restored by the Stop before; the worker clears it first anyway.
        PrivacyItemKind::MicrosoftToken => state.teams.disconnect().await,
        PrivacyItemKind::GoogleToken => state
            .calendar
            .disconnect(CalendarSource::Google)
            .await
            .map(drop),
        PrivacyItemKind::IcsLinks => state
            .calendar
            .disconnect(CalendarSource::Ics)
            .await
            .map(drop),
        PrivacyItemKind::CustomChimes => delete_custom_chimes(app, state).await,
        PrivacyItemKind::Blocklist => {
            let blocklist = state.blocklist.clone();
            blocking(move || blocklist.restore_default().map(drop)).await
        }
        PrivacyItemKind::BrowserData => clear_browser_data(app, state),
        PrivacyItemKind::Logs => {
            let logs = state.privacy.inner.folders.logs.clone();
            let active = state.privacy.inner.folders.active_log.clone();
            match logs {
                Some(dir) => blocking(move || clear_logs(&dir, active.as_deref())).await,
                None => Ok(()),
            }
        }
        PrivacyItemKind::Settings
        | PrivacyItemKind::ShieldsRestore
        | PrivacyItemKind::TintRestore => deletion_plan(&[kind]).map(drop),
        // Work tokens are handled above.
        PrivacyItemKind::GithubToken
        | PrivacyItemKind::GitlabToken
        | PrivacyItemKind::JiraToken
        | PrivacyItemKind::AzureDevopsToken
        | PrivacyItemKind::LinearToken => Ok(()),
    }
}

/// Runs blocking file IO on the blocking pool; errors become `privacy.delete`.
async fn blocking(
    work: impl FnOnce() -> anyhow::Result<()> + Send + 'static,
) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| AppError::new(ERR_DELETE, err.to_string()))?
        .map_err(|err| AppError::new(ERR_DELETE, format!("{err:#}")))
}

/// No custom chime in the config (a `custom` sound falls back to the default sound), and the files go.
async fn delete_custom_chimes(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    let _serial = state.config_update.lock().await;
    let mut config = read_config(&state.config);
    config.custom_chimes = CustomChimes::default();
    if config.chime_sound == ChimeSound::Custom {
        config.chime_sound = Config::default().chime_sound;
    }
    let saved = crate::commands::store_config(app, state, config).await?;
    chimes::remove_unused_later(state.persistence.sounds_dir(), saved.custom_chimes).await;
    Ok(())
}

/// The YouTube windows close before the profile is cleared (they would keep using the old cookies).
async fn close_browser_windows(app: &AppHandle, state: &AppState) {
    if let Err(err) = state.media.control(app, MediaAction::Close, None).await {
        log::warn!("privacy: cannot close the YouTube window: {}", err.message);
    }
    state.youtube.close_sign_in_window(app);
}

/// Clears cookies (YouTube sign-in and consent too), cache and local storage of the shared WebView2 profile.
/// WebView2 does it asynchronously: the YouTube account reads its cookies again (expecting a sign-out, ADR-021) and
/// the next scan walks the profile again.
fn clear_browser_data(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    let main = app
        .get_webview_window(MAIN_WINDOW)
        .ok_or_else(|| AppError::new(ERR_DELETE, "the main window is gone"))?;
    main.clear_all_browsing_data().map_err(|err| {
        AppError::new(ERR_DELETE, format!("cannot clear the browser data: {err}"))
    })?;
    // Only queued here: the scans walk the profile again until it has settled (the poll runs every 3 s).
    *lock(&state.privacy.inner.browser_recheck_until) = Some(Instant::now() + BROWSER_SETTLE);
    state.youtube.refresh(app, true);
    Ok(())
}

/// Deletes the log files; the one the logger keeps open is emptied instead (deleting it would lose the log until
/// the next start). A file that cannot be deleted is emptied too.
fn clear_logs(dir: &Path, active: Option<&Path>) -> anyhow::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err).with_context(|| format!("list {}", dir.display())),
    };
    let mut first_error = None;
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        let result = if Some(path.as_path()) == active {
            truncate(&path)
        } else {
            fs::remove_file(&path).or_else(|_| truncate(&path))
        };
        if let Err(err) = result {
            first_error
                .get_or_insert_with(|| anyhow::anyhow!("cannot delete {}: {err}", path.display()));
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn truncate(path: &Path) -> std::io::Result<()> {
    OpenOptions::new().write(true).open(path)?.set_len(0)
}

// -- export -------------------------------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy)]
struct ExportStats {
    sessions: usize,
    meeting_choices: bool,
    work_seen: bool,
    rewards: bool,
}

/// The file the export goes to: an absolute path (`.json` added when it has no extension), outside the app's own
/// folders (it must never overwrite the app's data).
fn export_target(raw: &str, app_folders: &[PathBuf]) -> Result<PathBuf, AppError> {
    let invalid = |detail: &str| AppError {
        code: ERR_EXPORT.into(),
        message: detail.to_owned(),
        field: Some("path".into()),
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(invalid("no file was chosen"));
    }
    let mut path = PathBuf::from(trimmed);
    // `..` would let a path slip past the folder check below (the save dialog never returns one).
    if !path.is_absolute()
        || path.file_name().is_none()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(invalid("the export needs a full file path"));
    }
    if path.extension().is_none() {
        path.set_extension(EXPORT_EXTENSION);
    }
    if app_folders.iter().any(|folder| is_inside(&path, folder)) {
        return Err(invalid("choose a folder outside FocusMode's data folder"));
    }
    Ok(path)
}

/// Builds the export document and writes it atomically.
fn write_export(files: &DataFiles, config: &Config, target: &Path) -> anyhow::Result<ExportStats> {
    let history = read_history_lines(&files.history)?;
    let meeting_choices = read_json(&files.meeting_choices);
    let work_seen = read_json(&files.work_seen);
    let rewards = read_json(&files.rewards);
    let stats = ExportStats {
        sessions: history.len(),
        meeting_choices: meeting_choices.is_some(),
        work_seen: work_seen.is_some(),
        rewards: rewards.is_some(),
    };
    let document = Export::new(
        wall_ms(),
        env!("CARGO_PKG_VERSION"),
        config,
        history,
        meeting_choices,
        work_seen,
        rewards,
    );
    let json = serde_json::to_vec_pretty(&document).context("serialize the export")?;
    write_atomic(target, &json)?;
    Ok(stats)
}

/// The stored sessions as they are (oldest first); lines that are not JSON are skipped.
fn read_history_lines(path: &Path) -> anyhow::Result<Vec<serde_json::Value>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
    };
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

/// A JSON file as stored; `None` when it is missing or not JSON.
fn read_json(path: &Path) -> Option<serde_json::Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// -- scanning -----------------------------------------------------------------------------------------------

/// Counts other modules keep in memory.
struct Live {
    ics_links: Option<u32>,
    blocklist_entries: u32,
}

/// Size and modification time of a file: its parsed counts are reused while it stays the same.
type Stamp = (Option<SystemTime>, u64);

#[derive(Default)]
struct Scanner {
    history: Option<(Stamp, HistoryCount)>,
    meeting_choices: Option<(Stamp, Option<u32>)>,
    work_seen: Option<(Stamp, Option<u32>)>,
    rewards: Option<(Stamp, Option<u32>)>,
    browser: Option<(Instant, Walked)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HistoryCount {
    sessions: u32,
    includes_titles: bool,
}

/// What a walk of a folder found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Walked {
    files: u32,
    /// `None`: the walk hit its limits.
    bytes: Option<u64>,
}

impl Scanner {
    fn scan(&mut self, folders: &Folders, live: &Live, force_browser: bool) -> Vec<PrivacyItem> {
        let files = &folders.files;
        PrivacyItemKind::ALL
            .into_iter()
            .map(|kind| match kind {
                PrivacyItemKind::Settings => {
                    let config = file_size(&files.config);
                    let backup = file_size(&files.config_backup);
                    let bytes = config.map(|size| size.saturating_add(backup.unwrap_or(0)));
                    item(
                        kind,
                        config.is_some() || backup.is_some(),
                        bytes.or(backup),
                        None,
                    )
                }
                PrivacyItemKind::History => self.history_item(&files.history),
                PrivacyItemKind::Rewards => {
                    // The ledger, its backup and unreadable copies set aside at load together (what the delete
                    // takes); `count` = stored days of `rewards.json`.
                    let count = cached_count(&mut self.rewards, &files.rewards, &["days"]);
                    let sizes: Vec<u64> = [files.rewards.clone(), files.rewards_backup.clone()]
                        .into_iter()
                        .chain(crate::persistence::unreadable_rewards_in(&files.data_dir))
                        .filter_map(|path| file_size(&path))
                        .collect();
                    let bytes = (!sizes.is_empty()).then(|| {
                        sizes
                            .iter()
                            .fold(0_u64, |sum, size| sum.saturating_add(*size))
                    });
                    item(kind, bytes.is_some(), bytes, count)
                }
                PrivacyItemKind::MeetingChoices => {
                    // Answers plus the "Ask me instead" entries of smart breaks (ADR-023), which can exist alone.
                    let count = cached_count(
                        &mut self.meeting_choices,
                        &files.meeting_choices,
                        &["choices", "optOuts"],
                    );
                    file_item(kind, &files.meeting_choices, count)
                }
                PrivacyItemKind::WorkSeen => {
                    let count = cached_count(&mut self.work_seen, &files.work_seen, &["seen"]);
                    file_item(kind, &files.work_seen, count)
                }
                PrivacyItemKind::MicrosoftToken => file_item(kind, &files.teams_token, None),
                PrivacyItemKind::GoogleToken => file_item(kind, &files.google_token, None),
                PrivacyItemKind::IcsLinks => file_item(kind, &files.ics_links, live.ics_links),
                PrivacyItemKind::GithubToken
                | PrivacyItemKind::GitlabToken
                | PrivacyItemKind::JiraToken
                | PrivacyItemKind::AzureDevopsToken
                | PrivacyItemKind::LinearToken => match kind.work_source() {
                    Some(source) => file_item(
                        kind,
                        &focus_platform::work::token_path_for(&files.data_dir, source),
                        None,
                    ),
                    None => item(kind, false, None, None),
                },
                PrivacyItemKind::CustomChimes => {
                    let walked = custom_chimes(&files.sounds);
                    item(kind, walked.files > 0, walked.bytes, Some(walked.files))
                }
                PrivacyItemKind::Blocklist => {
                    file_item(kind, &files.blocklist, Some(live.blocklist_entries))
                }
                PrivacyItemKind::ShieldsRestore => file_item(kind, &files.shields_restore, None),
                PrivacyItemKind::TintRestore => file_item(kind, &files.tint_restore, None),
                PrivacyItemKind::BrowserData => {
                    let walked = self.browser(folders.webview.as_deref(), force_browser);
                    item(kind, walked.files > 0, walked.bytes, None)
                }
                PrivacyItemKind::Logs => {
                    let walked = folders
                        .logs
                        .as_deref()
                        .map(|dir| walk(dir, BROWSER_MAX_ENTRIES, BROWSER_MAX_TIME))
                        .unwrap_or_default();
                    item(kind, walked.files > 0, walked.bytes, Some(walked.files))
                }
            })
            .collect()
    }

    fn history_item(&mut self, path: &Path) -> PrivacyItem {
        let kind = PrivacyItemKind::History;
        let Some(stamp) = stamp_of(path) else {
            self.history = None;
            return item(kind, false, None, None);
        };
        let count = match self.history {
            Some((known, count)) if known == stamp => count,
            _ => {
                let count = count_history(path);
                self.history = Some((stamp, count));
                count
            }
        };
        PrivacyItem {
            includes_titles: count.includes_titles,
            ..item(kind, true, Some(stamp.1), Some(count.sessions))
        }
    }

    fn browser(&mut self, dir: Option<&Path>, force: bool) -> Walked {
        let fresh = self
            .browser
            .filter(|(at, _)| !force && at.elapsed() < BROWSER_EVERY)
            .map(|(_, walked)| walked);
        if let Some(walked) = fresh {
            return walked;
        }
        let walked = dir
            .map(|dir| walk(dir, BROWSER_MAX_ENTRIES, BROWSER_MAX_TIME))
            .unwrap_or_default();
        self.browser = Some((Instant::now(), walked));
        walked
    }
}

fn item(
    kind: PrivacyItemKind,
    present: bool,
    bytes: Option<u64>,
    count: Option<u32>,
) -> PrivacyItem {
    PrivacyItem {
        kind,
        present,
        size_kib: bytes.filter(|_| present).map(size_kib),
        count: count.filter(|_| present),
        deletable: kind.is_deletable(),
        includes_titles: false,
    }
}

/// A single file: present when it is a file; `count` only then.
fn file_item(kind: PrivacyItemKind, path: &Path, count: Option<u32>) -> PrivacyItem {
    let size = file_size(path);
    item(kind, size.is_some(), size, count)
}

fn file_size(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(fs::Metadata::is_file)
        .map(|meta| meta.len())
}

fn stamp_of(path: &Path) -> Option<Stamp> {
    let meta = fs::metadata(path).ok().filter(fs::Metadata::is_file)?;
    Some((meta.modified().ok(), meta.len()))
}

/// The total length of the arrays `keys` of a JSON file (re-parsed only when the file changed); `None` when none of
/// them is there.
fn cached_count(
    cache: &mut Option<(Stamp, Option<u32>)>,
    path: &Path,
    keys: &[&str],
) -> Option<u32> {
    let Some(stamp) = stamp_of(path) else {
        *cache = None;
        return None;
    };
    if let Some((known, count)) = *cache
        && known == stamp
    {
        return count;
    }
    let count = read_json(path).and_then(|value| {
        keys.iter()
            .filter_map(|key| array_len(&value, key))
            .reduce(u32::saturating_add)
    });
    *cache = Some((stamp, count));
    count
}

fn array_len(value: &serde_json::Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .and_then(|array| u32::try_from(array.len()).ok())
}

/// Stored sessions and whether any keeps window titles (lines this version cannot read are not counted).
fn count_history(path: &Path) -> HistoryCount {
    let Ok(text) = fs::read_to_string(path) else {
        return HistoryCount::default();
    };
    history_count(&text)
}

fn history_count(text: &str) -> HistoryCount {
    let mut count = HistoryCount::default();
    for summary in text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<SessionSummary>(line).ok())
    {
        count.sessions = count.sessions.saturating_add(1);
        count.includes_titles |= includes_titles(&summary);
    }
    count
}

/// Imported chime files (`custom-…`) in the sounds folder.
fn custom_chimes(dir: &Path) -> Walked {
    let Ok(entries) = fs::read_dir(dir) else {
        return Walked::default();
    };
    let mut walked = Walked {
        files: 0,
        bytes: Some(0),
    };
    for entry in entries.flatten() {
        let ours = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(CUSTOM_CHIME_PREFIX) && is_chime_file_name(name));
        let Ok(meta) = entry.metadata() else { continue };
        if ours && meta.is_file() {
            walked.files = walked.files.saturating_add(1);
            walked.bytes = walked.bytes.map(|bytes| bytes.saturating_add(meta.len()));
        }
    }
    walked
}

/// Files and bytes under `dir` (symbolic links are not followed); the size is unknown (`None`) once the walk passes
/// `max_entries` or `max_time`.
fn walk(dir: &Path, max_entries: usize, max_time: Duration) -> Walked {
    let started = Instant::now();
    let mut walked = Walked {
        files: 0,
        bytes: Some(0),
    };
    let mut pending = vec![dir.to_path_buf()];
    let mut entries_seen = 0_usize;
    while let Some(next) = pending.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            entries_seen += 1;
            if entries_seen > max_entries || started.elapsed() > max_time {
                walked.bytes = None;
                return walked;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                walked.files = walked.files.saturating_add(1);
                let len = entry.metadata().map_or(0, |meta| meta.len());
                walked.bytes = walked.bytes.map(|bytes| bytes.saturating_add(len));
            }
        }
    }
    walked
}

/// The folders without repeats and without one inside another (the earlier one wins).
fn distinct_folders(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut kept: Vec<PathBuf> = Vec::new();
    for candidate in candidates {
        if kept.iter().any(|folder| is_inside(&candidate, folder)) {
            continue;
        }
        kept.retain(|folder| !is_inside(folder, &candidate));
        kept.push(candidate);
    }
    kept
}

/// `path` is `dir` or below it (compared by components, ignoring ASCII case: Windows paths are case-insensitive).
fn is_inside(path: &Path, dir: &Path) -> bool {
    let normalize = |path: &Path| -> Vec<String> {
        path.components()
            .filter(|part| !matches!(part, Component::CurDir))
            .map(|part| part.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect()
    };
    let path = normalize(path);
    let dir = normalize(dir);
    !dir.is_empty() && path.len() >= dir.len() && path[..dir.len()] == dir[..]
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

    fn temp_dir(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("focusmode-privacy-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create");
        root
    }

    #[test]
    fn folders_drop_repeats_and_nested_ones() {
        let data = PathBuf::from(r"C:\Users\u\AppData\Roaming\com.focusmode.app");
        let local = PathBuf::from(r"C:\Users\u\AppData\Local\com.focusmode.app");
        let logs = local.join("logs");
        assert_eq!(
            distinct_folders(vec![
                data.clone(),
                data.clone(),
                local.clone(),
                logs.clone()
            ]),
            [data.clone(), local.clone()]
        );
        // A parent listed later replaces its child.
        assert_eq!(distinct_folders(vec![logs, local.clone()]), [local]);
        assert!(is_inside(
            Path::new(r"c:\users\U\appdata\roaming\COM.FOCUSMODE.APP\config.json"),
            &data
        ));
        assert!(!is_inside(
            Path::new(r"C:\Users\u\AppData\Roaming\com.focusmode.app2\x.json"),
            &data
        ));
    }

    #[test]
    fn export_target_needs_a_full_path_outside_the_app_folders() {
        let root = temp_dir("target");
        let app = root.join("app");
        let folders = [app.clone()];
        let plain = root.join("out").join("my-data");
        let target = export_target(&format!("  {}  ", plain.display()), &folders).expect("valid");
        assert_eq!(target, root.join("out").join("my-data.json"));
        let named = root.join("export.txt");
        assert_eq!(
            export_target(&named.to_string_lossy(), &folders).expect("valid"),
            named
        );
        for bad in ["", "   ", "relative.json"] {
            let err = export_target(bad, &folders).expect_err("invalid");
            assert_eq!(
                (err.code.as_str(), err.field.as_deref()),
                (ERR_EXPORT, Some("path"))
            );
        }
        let inside = app.join("config.json");
        assert!(export_target(&inside.to_string_lossy(), &folders).is_err());
        // A detour through `..` that lands in the app folder is refused too.
        let detour = root.join("out").join("..").join("app").join("config.json");
        assert!(export_target(&detour.to_string_lossy(), &folders).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn history_counts_sessions_and_titles() {
        let session = |id: &str, title: Option<&str>| {
            let titles = title.map_or_else(String::new, |title| {
                format!(r#"{{"title":"{title}","secs":5}}"#)
            });
            format!(
                r#"{{"sessionId":"{id}","startedAtMs":1,"endedAtMs":2,"focusedSecs":1,"breakSecs":0,"cyclesCompleted":0,"usage":{{"apps":[{{"process":"Code.exe","secs":5,"titles":[{titles}]}}],"idleSecs":0,"unknownSecs":0,"trackedSecs":5}},"demo":false}}"#
            )
        };
        let text = format!(
            "{}\nnot json\n\n{}\n",
            session("a", None),
            session("b", None)
        );
        assert_eq!(
            history_count(&text),
            HistoryCount {
                sessions: 2,
                includes_titles: false
            }
        );
        let text = format!(
            "{}\n{}\n",
            session("a", None),
            session("b", Some("main.rs"))
        );
        assert_eq!(
            history_count(&text),
            HistoryCount {
                sessions: 2,
                includes_titles: true
            }
        );
    }

    #[test]
    fn scan_reports_presence_sizes_and_counts() {
        let root = temp_dir("scan");
        let persistence =
            Persistence::with_test_dirs(&root.join("config"), &root.join("data")).expect("dirs");
        let files = persistence.files();
        let folders = Folders {
            files: files.clone(),
            webview: Some(root.join("local").join(WEBVIEW_DIR)),
            logs: Some(root.join("local").join("logs")),
            active_log: None,
            list: vec![files.data_dir.clone()],
        };
        let live = Live {
            ics_links: Some(2),
            blocklist_entries: 7,
        };
        let mut scanner = Scanner::default();
        let find = |items: &[PrivacyItem], kind| {
            items
                .iter()
                .find(|item| item.kind == kind)
                .cloned()
                .expect("every kind is listed")
        };

        let empty = scanner.scan(&folders, &live, true);
        assert_eq!(empty.len(), PrivacyItemKind::ALL.len());
        assert!(
            empty
                .iter()
                .all(|item| !item.present && item.size_kib.is_none() && item.count.is_none())
        );
        assert!(!find(&empty, PrivacyItemKind::Settings).deletable);
        assert!(find(&empty, PrivacyItemKind::History).deletable);
        assert!(find(&empty, PrivacyItemKind::Rewards).deletable);

        persistence.save_config(&Config::default()).expect("config");
        persistence
            .save_meeting_choices(&serde_json::json!({ "version": 1, "choices": [{}, {}, {}] }))
            .expect("choices");
        fs::write(&files.ics_links, vec![0_u8; 1500]).expect("ics");
        fs::write(
            &files.rewards,
            r#"{"version":1,"totalSparks":62,"days":[{"day":20739},{"day":20740}]}"#,
        )
        .expect("rewards");
        fs::write(&files.rewards_backup, vec![b' '; 1100]).expect("rewards backup");
        fs::create_dir_all(&files.sounds).expect("sounds");
        fs::write(files.sounds.join("custom-break-1.mp3"), b"x").expect("chime");
        fs::write(files.sounds.join("readme.txt"), b"not ours").expect("other");
        let webview = root.join("local").join(WEBVIEW_DIR).join("Default");
        fs::create_dir_all(&webview).expect("webview");
        fs::write(webview.join("Cookies"), vec![0_u8; 3000]).expect("cookies");

        let items = scanner.scan(&folders, &live, true);
        let settings = find(&items, PrivacyItemKind::Settings);
        assert!(settings.present && settings.size_kib.is_some_and(|kib| kib >= 1));
        assert_eq!(find(&items, PrivacyItemKind::MeetingChoices).count, Some(3));
        // "Ask me instead" entries of smart breaks (ADR-023) count too, also without any answer.
        persistence
            .save_meeting_choices(
                &serde_json::json!({ "version": 1, "choices": [], "optOuts": [{}, {}] }),
            )
            .expect("opt-outs");
        let opt_outs = scanner.scan(&folders, &live, true);
        assert_eq!(
            find(&opt_outs, PrivacyItemKind::MeetingChoices).count,
            Some(2)
        );
        let rewards = find(&items, PrivacyItemKind::Rewards);
        assert_eq!(
            (rewards.present, rewards.size_kib, rewards.count),
            (true, Some(2), Some(2))
        );
        persistence.clear_rewards().expect("clear rewards");
        assert!(!files.rewards.exists() && !files.rewards_backup.exists());
        // Only an unreadable copy set aside at load: still listed (it is deleted with the item).
        let aside = files.data_dir.join("rewards.unreadable-1760342400000.json");
        fs::write(&aside, vec![b'x'; 2000]).expect("aside");
        let only_aside = scanner.scan(&folders, &live, true);
        let rewards = find(&only_aside, PrivacyItemKind::Rewards);
        assert_eq!(
            (rewards.present, rewards.size_kib, rewards.count),
            (true, Some(2), None)
        );
        persistence.clear_rewards().expect("clear rewards");
        assert!(!aside.exists());
        let ics = find(&items, PrivacyItemKind::IcsLinks);
        assert_eq!(
            (ics.present, ics.size_kib, ics.count),
            (true, Some(2), Some(2))
        );
        let chimes = find(&items, PrivacyItemKind::CustomChimes);
        assert_eq!((chimes.present, chimes.count), (true, Some(1)));
        let browser = find(&items, PrivacyItemKind::BrowserData);
        assert_eq!((browser.present, browser.size_kib), (true, Some(3)));
        // The blocklist file does not exist here: no count without the file.
        assert_eq!(find(&items, PrivacyItemKind::Blocklist).count, None);
        assert!(!find(&items, PrivacyItemKind::ShieldsRestore).present);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn logs_are_deleted_and_the_active_one_emptied() {
        let root = temp_dir("logs");
        let active = root.join("FocusMode.log");
        fs::write(&active, b"current").expect("active");
        fs::write(root.join("FocusMode_2026-10-01.log"), b"old").expect("old");
        clear_logs(&root, Some(&active)).expect("clear");
        assert_eq!(fs::read(&active).expect("kept").len(), 0);
        assert!(!root.join("FocusMode_2026-10-01.log").exists());
        clear_logs(&root.join("missing"), None).expect("a missing folder is fine");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn walk_stops_at_its_limits() {
        let root = temp_dir("walk");
        fs::create_dir_all(root.join("a")).expect("dir");
        for name in ["1", "2", "3"] {
            fs::write(root.join("a").join(name), b"12").expect("file");
        }
        assert_eq!(
            walk(&root, 100, Duration::from_secs(5)),
            Walked {
                files: 3,
                bytes: Some(6)
            }
        );
        assert_eq!(walk(&root, 2, Duration::from_secs(5)).bytes, None);
        assert_eq!(
            walk(&root.join("missing"), 100, Duration::from_secs(5)),
            Walked {
                files: 0,
                bytes: Some(0)
            }
        );
        let _ = fs::remove_dir_all(&root);
    }
}
