//! Warm screen tint (ADR-014): a real colour temperature applied by Windows after composition, so it
//! covers the whole screen (taskbar, fullscreen apps) and stays out of screenshots.
//!
//! Two mechanisms (`TintConfig::mode`):
//! * `NightLight` (default): FocusMode switches Windows Night light on and sets its strength on its own
//!   schedule (per-user setting, no admin). When it is unavailable, the tint falls back to `Gamma` and
//!   says why in `TintStatus::error`; Night light is tried again at the next warm period or config change.
//! * `Gamma`: each display's gamma ramp scaled by `focus_core::tint::rgb_gain_for_kelvin`. Windows may refuse
//!   very warm ramps; the platform then applies the warmest accepted value (`TintStatus::capped`).
//!
//! A dedicated thread owns both. Every [`SCHEDULE_TICK`], on a config change and on a settings preview it
//! computes the level (`focus_core::tint::tint_level` for the local time of day, or the preview strength)
//! and the colour temperature (`kelvin_for_level`), then applies it, or restores the screen at level 0.
//!
//! Crash safety, like muted notifications (`crate::shields`): **before the first change** the previous state
//! of the mechanism is written to `tint-restore.json` and the logon safety net is armed
//! (`crate::safety_net`). The screen is restored when the level drops to 0, when the tint is turned off,
//! when the mode changes, when the app quits (`exit_blocking` from `crate::shutdown`) and at the next start
//! (or logon, `--restore-shields`) if a restore file was left behind. The file and the logon entry are
//! removed only after a successful restore.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::api::{AppError, TintMode, TintStatus, events};
use focus_core::tint::{MAX_STRENGTH, kelvin_for_level, tint_level};
use focus_platform::PlatformError;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::persistence::Persistence;
use crate::safety_net::{Pending, SafetyNet};
use crate::{
    SharedConfig, crash_restore_done, emit_error, emit_startup_error, lock, read_config,
    restore_only_launch, restore_only_pending,
};

/// How often the schedule is re-evaluated (a fade of 30 min moves in steps of ~1/60).
const SCHEDULE_TICK: Duration = Duration::from_secs(30);
/// A preview shows its strength this long after the last `preview_tint` call.
const PREVIEW_FOR: Duration = Duration::from_secs(6);
/// A `--restore-shields` launch runs early at logon: the crash-restore is tried this many times in total,
/// [`RESTORE_RETRY_DELAY`] apart.
const RESTORE_ATTEMPTS_AT_LOGON: u32 = 4;
const RESTORE_RETRY_DELAY: Duration = Duration::from_secs(5);
const SECS_PER_DAY: u64 = 86_400;

/// Opaque state of one mechanism before FocusMode changed it (what the platform needs to put it back).
type Snapshot = serde_json::Value;

/// What [`Warmth::apply`] achieved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Applied {
    /// Windows refused the full strength and a weaker value is shown.
    capped: bool,
    /// Partly applied (e.g. a display that refused its gamma ramp): English detail for `TintStatus::error`.
    detail: Option<String>,
}

/// One way of warming the screen (an adapter over `focus_platform`; a fake in the tests).
trait Warmth: Send {
    /// The mechanism can be used on this machine at all (Night light: not on Windows 11 24H2 and later).
    fn available(&self) -> bool;
    /// Captures what [`Warmth::restore`] puts back. Called before the first change.
    fn snapshot(&mut self) -> Result<Snapshot, PlatformError>;
    /// Shows `kelvin` (an error: nothing could be applied).
    fn apply(&mut self, kelvin: u32) -> Result<Applied, PlatformError>;
    /// Puts `snapshot` back.
    fn restore(&mut self, snapshot: &Snapshot) -> Result<(), PlatformError>;
    /// `true`: applied again every tick even when the value did not change (a display change resets a
    /// gamma ramp). `false` (Night light): written only when it changes, so the user can still switch it
    /// off by hand in between.
    fn reapply_unchanged(&self) -> bool;
    /// `snapshot` extended with what the mechanism recorded since (gamma: the original ramp of a display
    /// connected later); `None` when `snapshot` still covers everything it changed.
    fn extend_snapshot(&self, _snapshot: &Snapshot) -> Option<Snapshot> {
        None
    }
}

/// Content of `tint-restore.json`: the snapshot of every mechanism FocusMode has changed and not restored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TintRestore {
    night_light: Option<Snapshot>,
    gamma: Option<Snapshot>,
}

impl TintRestore {
    fn slot(&mut self, mode: TintMode) -> &mut Option<Snapshot> {
        match mode {
            TintMode::NightLight => &mut self.night_light,
            TintMode::Gamma => &mut self.gamma,
        }
    }

    fn get(&self, mode: TintMode) -> Option<&Snapshot> {
        match mode {
            TintMode::NightLight => self.night_light.as_ref(),
            TintMode::Gamma => self.gamma.as_ref(),
        }
    }

    fn is_empty(&self) -> bool {
        self.night_light.is_none() && self.gamma.is_none()
    }
}

/// Where the restore snapshot is kept: the file and the logon safety net (a fake in the tests).
trait RestoreStore: Send {
    /// Persists `restore` and arms the logon restore. Called before Windows is changed.
    fn save(&mut self, restore: &TintRestore) -> anyhow::Result<()>;
    /// Nothing is left to restore: deletes the file and drops the tint part of the logon entry.
    fn clear(&mut self);
}

struct PersistedRestore {
    persistence: Persistence,
    safety_net: SafetyNet,
}

impl RestoreStore for PersistedRestore {
    fn save(&mut self, restore: &TintRestore) -> anyhow::Result<()> {
        self.persistence.save_tint_restore(restore)?;
        self.safety_net.arm(Pending::Tint);
        Ok(())
    }

    fn clear(&mut self) {
        self.persistence.clear_tint_restore();
        self.safety_net.disarm(Pending::Tint);
    }
}

/// Result of bringing the screen to a level.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Outcome {
    /// Mechanism in use (the preferred one, or `Gamma` as the Night light fallback).
    mode: TintMode,
    active: bool,
    capped: bool,
    error: Option<String>,
    /// `error` is worth an `app://error`; `false` when it only says that Night light cannot be used on this
    /// Windows version (a permanent fact, shown in the settings through `TintStatus::error`).
    report: bool,
}

/// Why Night light is skipped for now (the gamma fallback is used).
#[derive(Debug, Clone, PartialEq, Eq)]
struct NightLightFallback {
    reason: String,
    /// Night light cannot be used here at all (not a failure).
    expected: bool,
}

/// Status detail when Night light cannot be used on this machine.
const NIGHT_LIGHT_UNAVAILABLE: &str = "Night light cannot be controlled on this version of Windows; \
                                       the display gamma is used instead";

/// The tint state machine, free of Tauri (unit-tested with fakes).
struct Engine {
    night_light: Box<dyn Warmth>,
    gamma: Box<dyn Warmth>,
    store: Box<dyn RestoreStore>,
    /// Mirrors the persisted restore file.
    restore: TintRestore,
    /// Mechanism and temperature shown last (`None`: nothing applied right now) and how it went.
    applied: Option<(TintMode, u32)>,
    last: Applied,
    /// Why Night light is skipped (falling back to gamma) until the next warm period or config change.
    night_light_failed: Option<NightLightFallback>,
}

impl Engine {
    fn new(
        night_light: Box<dyn Warmth>,
        gamma: Box<dyn Warmth>,
        store: Box<dyn RestoreStore>,
    ) -> Self {
        Self {
            night_light,
            gamma,
            store,
            restore: TintRestore::default(),
            applied: None,
            last: Applied::default(),
            night_light_failed: None,
        }
    }

    fn backend(&mut self, mode: TintMode) -> &mut dyn Warmth {
        match mode {
            TintMode::NightLight => self.night_light.as_mut(),
            TintMode::Gamma => self.gamma.as_mut(),
        }
    }

    /// A config change: try Night light again.
    fn retry_night_light(&mut self) {
        self.night_light_failed = None;
    }

    /// Brings the screen to `level` (0 = untouched) with the `preferred` mechanism.
    fn set(&mut self, level: u32, preferred: TintMode) -> Outcome {
        if level == 0 {
            self.night_light_failed = None;
            let error = self.restore_all().err();
            return Outcome {
                mode: preferred,
                active: false,
                capped: false,
                report: error.is_some(),
                error,
            };
        }
        let kelvin = kelvin_for_level(level);
        if preferred == TintMode::NightLight && self.night_light_failed.is_none() {
            if !self.night_light.available() {
                log::info!("Night light cannot be controlled here; using the display gamma");
                self.night_light_failed = Some(NightLightFallback {
                    reason: NIGHT_LIGHT_UNAVAILABLE.to_owned(),
                    expected: true,
                });
            } else {
                match self.show(TintMode::NightLight, kelvin) {
                    Ok(applied) => {
                        return Outcome {
                            mode: TintMode::NightLight,
                            active: true,
                            capped: applied.capped,
                            report: applied.detail.is_some(),
                            error: applied.detail,
                        };
                    }
                    Err(err) => {
                        log::warn!("Night light failed, falling back to the display gamma: {err}");
                        // Never leave a half-applied Night light under the gamma fallback.
                        let restored = self.restore_one(TintMode::NightLight).err();
                        let reason = join_details(
                            Some(format!(
                                "Night light failed ({err}); the display gamma is used instead"
                            )),
                            restored,
                        )
                        .unwrap_or_default();
                        self.night_light_failed = Some(NightLightFallback {
                            reason,
                            expected: false,
                        });
                    }
                }
            }
        }
        let fallback = self
            .night_light_failed
            .clone()
            .filter(|_| preferred == TintMode::NightLight);
        let fallback_failed = fallback.as_ref().is_some_and(|fallback| !fallback.expected);
        let reason = fallback.map(|fallback| fallback.reason);
        match self.show(TintMode::Gamma, kelvin) {
            Ok(applied) => Outcome {
                mode: TintMode::Gamma,
                active: true,
                capped: applied.capped,
                report: fallback_failed || applied.detail.is_some(),
                error: join_details(reason, applied.detail),
            },
            Err(err) => Outcome {
                mode: TintMode::Gamma,
                active: false,
                capped: false,
                report: true,
                error: join_details(reason, Some(err)),
            },
        }
    }

    /// Shows `kelvin` with `mode`: leaves the other mechanism first, persists the snapshot before the first
    /// change.
    fn show(&mut self, mode: TintMode, kelvin: u32) -> Result<Applied, String> {
        let other = match mode {
            TintMode::NightLight => TintMode::Gamma,
            TintMode::Gamma => TintMode::NightLight,
        };
        // Read first (it only reads): when this mechanism is unavailable, the other one stays as it is.
        let snapshot = match self.restore.get(mode) {
            Some(_) => None,
            None => Some(
                self.backend(mode)
                    .snapshot()
                    .map_err(|err| format!("cannot read the current {}: {err}", name(mode)))?,
            ),
        };
        // Both at once would double the warmth: the other one must be restored first.
        self.restore_one(other)?;

        if let Some(snapshot) = snapshot {
            *self.restore.slot(mode) = Some(snapshot);
            if let Err(err) = self.store.save(&self.restore) {
                // Without the snapshot on disk a crash could leave the screen warm: do not change it.
                *self.restore.slot(mode) = None;
                return Err(format!(
                    "cannot change the {}: the restore file cannot be saved: {err:#}",
                    name(mode)
                ));
            }
        }

        if self.applied == Some((mode, kelvin)) && !self.backend(mode).reapply_unchanged() {
            return Ok(self.last.clone());
        }
        match self.backend(mode).apply(kelvin) {
            Ok(applied) => {
                if self.applied != Some((mode, kelvin)) {
                    log::info!("screen tint: {kelvin} K via {}", name(mode));
                }
                self.applied = Some((mode, kelvin));
                self.last = applied.clone();
                self.persist_recorded(mode);
                Ok(applied)
            }
            Err(err) => {
                self.applied = None;
                self.last = Applied::default();
                Err(format!("cannot change the {}: {err}", name(mode)))
            }
        }
    }

    /// Keeps the restore file in step when `mode` recorded more to restore than its snapshot (a display
    /// connected after it was warmed), so a crash-restore covers that display too.
    fn persist_recorded(&mut self, mode: TintMode) {
        let Some(snapshot) = self.restore.get(mode).cloned() else {
            return;
        };
        let Some(extended) = self.backend(mode).extend_snapshot(&snapshot) else {
            return;
        };
        *self.restore.slot(mode) = Some(extended);
        if let Err(err) = self.store.save(&self.restore) {
            log::warn!("cannot update the tint restore file: {err:#}");
        }
    }

    /// Puts back what FocusMode changed with `mode` (no-op when it changed nothing).
    fn restore_one(&mut self, mode: TintMode) -> Result<(), String> {
        let Some(snapshot) = self.restore.get(mode).cloned() else {
            return Ok(());
        };
        self.backend(mode)
            .restore(&snapshot)
            .map_err(|err| format!("cannot restore the {}: {err}", name(mode)))?;
        log::info!("screen tint: {} restored", name(mode));
        *self.restore.slot(mode) = None;
        if self.applied.is_some_and(|(applied, _)| applied == mode) {
            self.applied = None;
            self.last = Applied::default();
        }
        if self.restore.is_empty() {
            self.store.clear();
        } else if let Err(err) = self.store.save(&self.restore) {
            log::warn!("cannot update the tint restore file: {err:#}");
        }
        Ok(())
    }

    /// Restores both mechanisms; the error lists every failure (what failed stays pending).
    fn restore_all(&mut self) -> Result<(), String> {
        let errors: Vec<String> = [TintMode::NightLight, TintMode::Gamma]
            .into_iter()
            .filter_map(|mode| self.restore_one(mode).err())
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Startup: a previous run left `restore` behind (it crashed with the screen warm). `attempts` tries,
    /// [`RESTORE_RETRY_DELAY`] apart. On failure the snapshot stays (in memory and on disk, and the logon
    /// entry is armed again: Windows deleted it if it launched us), so every later restore retries it.
    fn restore_after_crash(&mut self, restore: TintRestore, attempts: u32) -> Result<(), String> {
        self.restore = restore;
        let mut attempt = 1;
        loop {
            match self.restore_all() {
                Ok(()) => return Ok(()),
                Err(err) if attempt >= attempts => {
                    if let Err(save) = self.store.save(&self.restore) {
                        log::warn!("cannot re-arm the tint restore: {save:#}");
                    }
                    return Err(err);
                }
                Err(err) => {
                    log::warn!(
                        "restoring the screen colours failed (attempt {attempt}/{attempts}): {err}"
                    );
                    attempt += 1;
                    std::thread::sleep(RESTORE_RETRY_DELAY);
                }
            }
        }
    }
}

/// `a; b`, either, or `None`.
fn join_details(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(format!("{a}; {b}")),
        (a, b) => a.or(b),
    }
}

fn name(mode: TintMode) -> &'static str {
    match mode {
        TintMode::NightLight => "Night light",
        TintMode::Gamma => "display gamma",
    }
}

enum TintCmd {
    /// Config changed (or anything else that warrants a re-evaluation now).
    Refresh,
    /// `Some(strength)` starts or extends a preview; `None` ends it.
    Preview(Option<u32>),
    /// Restore the screen and stop the thread; acks when done.
    Exit(Sender<()>),
}

/// Cheap to clone; every method but [`TintHandle::exit_blocking`] is non-blocking.
#[derive(Clone)]
pub struct TintHandle {
    tx: Sender<TintCmd>,
    status: Arc<Mutex<TintStatus>>,
}

pub struct TintContext {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub persistence: Persistence,
    pub safety_net: SafetyNet,
}

impl TintHandle {
    pub fn spawn(ctx: TintContext) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let tint = read_config(&ctx.config).tint;
        let status = Arc::new(Mutex::new(TintStatus {
            revision: 0,
            enabled: tint.enabled,
            level: 0,
            preview: false,
            active: false,
            mode: tint.mode,
            capped: false,
            error: None,
        }));
        let shared = Arc::clone(&status);
        std::thread::Builder::new()
            .name("focus-tint".into())
            .spawn(move || {
                let engine = Engine::new(
                    backends::night_light(),
                    backends::gamma(),
                    Box::new(PersistedRestore {
                        persistence: ctx.persistence.clone(),
                        safety_net: ctx.safety_net,
                    }),
                );
                Worker {
                    app: ctx.app,
                    config: ctx.config,
                    persistence: ctx.persistence,
                    status: shared,
                    engine,
                    preview: None,
                    reported_error: None,
                    last_published: None,
                }
                .run(rx);
            })?;
        Ok(Self { tx, status })
    }

    pub fn status(&self) -> TintStatus {
        lock(&self.status).clone()
    }

    pub fn config_changed(&self) {
        self.send(TintCmd::Refresh);
    }

    /// Settings preview: show `strength` (clamped to 0..=100) for a few seconds; `None` ends it.
    pub fn preview(&self, strength: Option<u32>) {
        self.send(TintCmd::Preview(
            strength.map(|strength| strength.min(MAX_STRENGTH)),
        ));
    }

    /// Restores the screen colours and waits (bounded) until the thread confirms. Safe to call twice.
    pub fn exit_blocking(&self, timeout: Duration) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self.tx.send(TintCmd::Exit(ack_tx)).is_ok() && ack_rx.recv_timeout(timeout).is_err() {
            log::warn!("screen tint did not confirm shutdown within {timeout:?}");
        }
    }

    fn send(&self, cmd: TintCmd) {
        if self.tx.send(cmd).is_err() {
            log::warn!("tint thread is not running");
        }
    }
}

struct Worker {
    app: AppHandle,
    config: SharedConfig,
    persistence: Persistence,
    status: Arc<Mutex<TintStatus>>,
    engine: Engine,
    /// Preview strength and when it ends.
    preview: Option<(u32, Instant)>,
    /// Last error sent as `app://error` (each distinct error is reported once).
    reported_error: Option<String>,
    last_published: Option<TintStatus>,
}

impl Worker {
    fn run(mut self, rx: Receiver<TintCmd>) {
        self.restore_after_crash();
        // A `--restore-shields` launch exits once the notifications are restored too.
        crash_restore_done(&self.app);
        self.apply();
        loop {
            match rx.recv_timeout(self.next_wait()) {
                Ok(TintCmd::Exit(ack)) => {
                    self.exit();
                    let _ = ack.send(());
                    return;
                }
                Ok(cmd) => {
                    self.handle(cmd);
                    // A dragged strength slider sends many previews: take them all, apply once.
                    while let Ok(cmd) = rx.try_recv() {
                        if let TintCmd::Exit(ack) = cmd {
                            self.exit();
                            let _ = ack.send(());
                            return;
                        }
                        self.handle(cmd);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.exit();
                    return;
                }
            }
            self.apply();
        }
    }

    fn handle(&mut self, cmd: TintCmd) {
        match cmd {
            TintCmd::Refresh => self.engine.retry_night_light(),
            TintCmd::Preview(Some(strength)) => {
                self.preview = Some((strength, Instant::now() + PREVIEW_FOR));
            }
            TintCmd::Preview(None) => self.preview = None,
            // Handled by `run`.
            TintCmd::Exit(_) => {}
        }
    }

    fn next_wait(&self) -> Duration {
        match self.preview {
            Some((_, until)) => SCHEDULE_TICK.min(until.saturating_duration_since(Instant::now())),
            None => SCHEDULE_TICK,
        }
    }

    /// A previous run left the screen warm (crash, power loss): put it back before anything else.
    fn restore_after_crash(&mut self) {
        let Some(restore) = self.persistence.load_tint_restore::<TintRestore>() else {
            // Nothing pending: drop a stale tint part of the logon entry.
            self.engine.store.clear();
            return;
        };
        log::info!("restoring screen colours changed by a previous run");
        let attempts = if restore_only_pending() {
            RESTORE_ATTEMPTS_AT_LOGON
        } else {
            1
        };
        if let Err(err) = self.engine.restore_after_crash(restore, attempts) {
            emit_startup_error(
                &self.app,
                AppError::new(
                    "tint.restore",
                    format!("cannot restore the screen colours after a crash: {err}"),
                ),
            );
        }
    }

    /// Re-evaluates the level and brings the screen in line with it.
    fn apply(&mut self) {
        let tint = read_config(&self.config).tint;
        if self
            .preview
            .is_some_and(|(_, until)| Instant::now() >= until)
        {
            self.preview = None;
        }
        let preview = self.preview.map(|(strength, _)| strength);
        let scheduled = if tint.enabled {
            tint_level(&tint, local_minutes_of_day())
        } else {
            0
        };
        // A `--restore-shields` launch only repairs what a crash left behind and exits: no warmth.
        let level = if restore_only_launch() {
            0
        } else {
            preview.unwrap_or(scheduled)
        };
        let outcome = self.engine.set(level, tint.mode);
        if let Some(error) = outcome.error.as_deref().filter(|_| outcome.report) {
            self.report(error);
        }
        self.publish(TintStatus {
            revision: 0,
            enabled: tint.enabled,
            level,
            preview: preview.is_some(),
            active: outcome.active,
            mode: outcome.mode,
            capped: outcome.capped,
            error: outcome.error,
        });
    }

    /// Quit: restore the screen. A failure leaves the restore file and the logon entry in place.
    fn exit(&mut self) {
        match self.engine.restore_all() {
            Ok(()) => log::info!("screen tint: screen colours untouched at exit"),
            Err(err) => log::error!("cannot restore the screen colours at exit: {err}"),
        }
    }

    /// Sends a new error once as `app://error`.
    fn report(&mut self, message: &str) {
        if self.reported_error.as_deref() != Some(message) {
            emit_error(&self.app, AppError::new("tint.apply", message.to_owned()));
            self.reported_error = Some(message.to_owned());
        }
    }

    /// Emits `tint://changed` when anything but the revision changed.
    fn publish(&mut self, mut status: TintStatus) {
        if status.error.is_none() {
            self.reported_error = None;
        }
        if self.last_published.as_ref().is_some_and(|last| {
            let mut last = last.clone();
            last.revision = 0;
            last == status
        }) {
            return;
        }
        {
            let mut shared = lock(&self.status);
            status.revision = shared.revision.wrapping_add(1);
            *shared = status.clone();
        }
        if let Err(err) = self.app.emit(events::TINT_CHANGED, status.clone()) {
            log::warn!("cannot emit tint status: {err}");
        }
        self.last_published = Some(status);
    }
}

/// Local minutes after midnight; UTC when the local time is unavailable (non-Windows).
fn local_minutes_of_day() -> f64 {
    focus_platform::clock::local_minutes_of_day().unwrap_or_else(utc_minutes_of_day)
}

fn utc_minutes_of_day() -> f64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    // `secs % SECS_PER_DAY` < 86 400: exact in f64, and the result stays below 1440.
    (secs % SECS_PER_DAY) as f64 / 60.0
}

/// Adapters from the `focus_platform` colour APIs to [`Warmth`].
mod backends {
    use super::{Applied, Snapshot, Warmth};
    use focus_core::tint::rgb_gain_for_kelvin;
    use focus_platform::PlatformError;
    use focus_platform::gamma::{GammaRamp, GammaSnapshot};
    use focus_platform::night_light::{NightLight, NightLightSnapshot};
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    pub(super) fn night_light() -> Box<dyn Warmth> {
        Box::new(NightLightWarmth(focus_platform::night_light::night_light()))
    }

    pub(super) fn gamma() -> Box<dyn Warmth> {
        Box::new(GammaWarmth(focus_platform::gamma::gamma()))
    }

    /// Windows Night light: switched on at the level's temperature; written only when it changes.
    struct NightLightWarmth(Box<dyn NightLight>);

    impl Warmth for NightLightWarmth {
        fn available(&self) -> bool {
            self.0.available()
        }

        fn snapshot(&mut self) -> Result<Snapshot, PlatformError> {
            to_snapshot(&self.0.snapshot()?)
        }

        fn apply(&mut self, kelvin: u32) -> Result<Applied, PlatformError> {
            // The levels map to 6500..=1900 K, inside the Night light range: never capped.
            self.0.apply(true, kelvin)?;
            Ok(Applied::default())
        }

        fn restore(&mut self, snapshot: &Snapshot) -> Result<(), PlatformError> {
            match from_snapshot::<NightLightSnapshot>(snapshot) {
                Some(snapshot) => self.0.restore(&snapshot),
                None => Ok(()),
            }
        }

        fn reapply_unchanged(&self) -> bool {
            false
        }
    }

    /// Display gamma ramps: the original ramps scaled by the black-body gains; re-applied every tick
    /// (a display mode change or a new display resets or lacks the ramp).
    struct GammaWarmth(Box<dyn GammaRamp>);

    impl Warmth for GammaWarmth {
        fn available(&self) -> bool {
            true
        }

        fn snapshot(&mut self) -> Result<Snapshot, PlatformError> {
            to_snapshot(&self.0.snapshot()?)
        }

        fn apply(&mut self, kelvin: u32) -> Result<Applied, PlatformError> {
            let result = self.0.apply(rgb_gain_for_kelvin(kelvin))?;
            Ok(Applied {
                capped: result.capped,
                detail: None,
            })
        }

        fn restore(&mut self, snapshot: &Snapshot) -> Result<(), PlatformError> {
            match from_snapshot::<GammaSnapshot>(snapshot) {
                Some(snapshot) => self.0.restore(&snapshot),
                None => Ok(()),
            }
        }

        fn reapply_unchanged(&self) -> bool {
            true
        }

        fn extend_snapshot(&self, snapshot: &Snapshot) -> Option<Snapshot> {
            let saved = from_snapshot::<GammaSnapshot>(snapshot)?;
            // The saved ramps win (read before any change); only displays it lacks are added.
            let extended = saved.merged_with(&self.0.originals().displays);
            (extended != saved)
                .then(|| to_snapshot(&extended).ok())
                .flatten()
        }
    }

    fn to_snapshot(value: &impl Serialize) -> Result<Snapshot, PlatformError> {
        serde_json::to_value(value)
            .map_err(|err| PlatformError::Os(format!("cannot record the screen colours: {err}")))
    }

    /// The platform snapshot in `value`; `None` (logged) when it cannot be read: nothing sensible can be
    /// restored from it, so it is dropped like the platform drops snapshots it cannot use.
    fn from_snapshot<T: DeserializeOwned>(value: &Snapshot) -> Option<T> {
        match serde_json::from_value(value.clone()) {
            Ok(snapshot) => Some(snapshot),
            Err(err) => {
                log::warn!("dropping an unreadable screen colour snapshot: {err}");
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use focus_platform::gamma::{DisplayGamma, GammaResult, RAMP_LEN};

        #[test]
        fn platform_snapshots_round_trip_through_the_restore_file() {
            let gamma = GammaSnapshot::default();
            let value = to_snapshot(&gamma).expect("gamma");
            assert_eq!(from_snapshot::<GammaSnapshot>(&value), Some(gamma));

            let night = NightLightSnapshot {
                method: "cloudStore".into(),
                state: "AAA=".into(),
                settings: "AAA=".into(),
            };
            let value = to_snapshot(&night).expect("night");
            assert_eq!(from_snapshot::<NightLightSnapshot>(&value), Some(night));
            assert_eq!(
                from_snapshot::<NightLightSnapshot>(&serde_json::json!({ "x": 1 })),
                None
            );
        }

        /// Platform gamma that has recorded `originals`.
        struct Recorded(GammaSnapshot);

        impl GammaRamp for Recorded {
            fn snapshot(&mut self) -> Result<GammaSnapshot, PlatformError> {
                Ok(self.0.clone())
            }

            fn apply(&mut self, _gain: [f32; 3]) -> Result<GammaResult, PlatformError> {
                Ok(GammaResult::default())
            }

            fn restore(&mut self, _snap: &GammaSnapshot) -> Result<(), PlatformError> {
                Ok(())
            }

            fn originals(&self) -> GammaSnapshot {
                self.0.clone()
            }
        }

        #[test]
        fn the_gamma_snapshot_grows_with_displays_connected_later() {
            let display = |device: &str, level: u16| DisplayGamma {
                device: device.into(),
                red: vec![level; RAMP_LEN],
                green: vec![level; RAMP_LEN],
                blue: vec![level; RAMP_LEN],
            };
            let saved = GammaSnapshot {
                displays: vec![display("D1", 1)],
            };
            let saved_value = to_snapshot(&saved).expect("saved");
            // The platform knows D1 with another ramp (it must not replace the saved original) and a new D2.
            let warmth = GammaWarmth(Box::new(Recorded(GammaSnapshot {
                displays: vec![display("D1", 7), display("D2", 2)],
            })));
            let extended = warmth.extend_snapshot(&saved_value).expect("grows");
            assert_eq!(
                from_snapshot::<GammaSnapshot>(&extended),
                Some(GammaSnapshot {
                    displays: vec![display("D1", 1), display("D2", 2)]
                })
            );
            assert_eq!(warmth.extend_snapshot(&extended), None, "nothing new");
            assert_eq!(night_light().extend_snapshot(&saved_value), None);
        }

        #[test]
        fn platform_backends_are_constructible() {
            let night = night_light();
            let gamma = gamma();
            assert!(!night.reapply_unchanged());
            assert!(gamma.reapply_unchanged());
            assert!(gamma.available());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::tint::MINUTES_PER_DAY;

    /// What happened to the fakes, in order.
    type Log = Arc<Mutex<Vec<String>>>;

    struct FakeWarmth {
        name: &'static str,
        log: Log,
        fail_snapshot: bool,
        fail_apply: bool,
        fail_restore: Arc<Mutex<bool>>,
        capped: bool,
        reapply: bool,
        available: bool,
        /// Recorded more to restore since the snapshot (a display connected later).
        grown: Arc<Mutex<bool>>,
    }

    impl FakeWarmth {
        fn new(name: &'static str, log: &Log) -> Self {
            Self {
                name,
                log: Arc::clone(log),
                fail_snapshot: false,
                fail_apply: false,
                fail_restore: Arc::new(Mutex::new(false)),
                capped: false,
                reapply: name == "gamma",
                available: true,
                grown: Arc::new(Mutex::new(false)),
            }
        }

        fn note(&self, what: String) {
            lock(&self.log).push(format!("{} {what}", self.name));
        }
    }

    impl Warmth for FakeWarmth {
        fn available(&self) -> bool {
            self.available
        }

        fn snapshot(&mut self) -> Result<Snapshot, PlatformError> {
            self.note("snapshot".into());
            if self.fail_snapshot {
                return Err(PlatformError::Os("no such key".into()));
            }
            Ok(serde_json::json!({ "was": self.name }))
        }

        fn apply(&mut self, kelvin: u32) -> Result<Applied, PlatformError> {
            self.note(format!("apply {kelvin}"));
            if self.fail_apply {
                return Err(PlatformError::Os("refused".into()));
            }
            Ok(Applied {
                capped: self.capped,
                detail: self.capped.then(|| "display 2 refused the ramp".to_owned()),
            })
        }

        fn restore(&mut self, snapshot: &Snapshot) -> Result<(), PlatformError> {
            assert_eq!(snapshot["was"], self.name);
            let grown = snapshot.get("grown").is_some();
            self.note(if grown { "restore grown" } else { "restore" }.into());
            if *lock(&self.fail_restore) {
                return Err(PlatformError::Os("busy".into()));
            }
            Ok(())
        }

        fn reapply_unchanged(&self) -> bool {
            self.reapply
        }

        fn extend_snapshot(&self, snapshot: &Snapshot) -> Option<Snapshot> {
            (*lock(&self.grown) && snapshot.get("grown").is_none())
                .then(|| serde_json::json!({ "was": self.name, "grown": true }))
        }
    }

    struct FakeStore {
        log: Log,
        saved: Arc<Mutex<Option<TintRestore>>>,
        fail: bool,
    }

    impl RestoreStore for FakeStore {
        fn save(&mut self, restore: &TintRestore) -> anyhow::Result<()> {
            lock(&self.log).push("save".into());
            if self.fail {
                anyhow::bail!("disk full");
            }
            *lock(&self.saved) = Some(restore.clone());
            Ok(())
        }

        fn clear(&mut self) {
            lock(&self.log).push("clear".into());
            *lock(&self.saved) = None;
        }
    }

    struct Rig {
        engine: Engine,
        log: Log,
        saved: Arc<Mutex<Option<TintRestore>>>,
    }

    impl Rig {
        fn take_log(&self) -> Vec<String> {
            std::mem::take(&mut *lock(&self.log))
        }
    }

    fn rig(tweak: impl FnOnce(&mut FakeWarmth, &mut FakeWarmth, &mut FakeStore)) -> Rig {
        let log: Log = Arc::default();
        let saved = Arc::new(Mutex::new(None));
        let mut night = FakeWarmth::new("night", &log);
        let mut gamma = FakeWarmth::new("gamma", &log);
        let mut store = FakeStore {
            log: Arc::clone(&log),
            saved: Arc::clone(&saved),
            fail: false,
        };
        tweak(&mut night, &mut gamma, &mut store);
        Rig {
            engine: Engine::new(Box::new(night), Box::new(gamma), Box::new(store)),
            log,
            saved,
        }
    }

    #[test]
    fn night_light_is_saved_before_the_first_change_and_restored_at_level_zero() {
        let mut rig = rig(|_, _, _| {});
        let kelvin = kelvin_for_level(45);
        let outcome = rig.engine.set(45, TintMode::NightLight);
        assert_eq!(
            outcome,
            Outcome {
                mode: TintMode::NightLight,
                active: true,
                capped: false,
                error: None,
                report: false
            }
        );
        assert_eq!(
            rig.take_log(),
            ["night snapshot", "save", &format!("night apply {kelvin}")]
        );
        assert!(
            lock(&rig.saved)
                .as_ref()
                .is_some_and(|r| r.night_light.is_some())
        );

        // Same value again: Night light is not written again.
        rig.engine.set(45, TintMode::NightLight);
        assert!(rig.take_log().is_empty());
        // A new value is written, without a new snapshot.
        rig.engine.set(50, TintMode::NightLight);
        assert_eq!(
            rig.take_log(),
            [format!("night apply {}", kelvin_for_level(50))]
        );

        let outcome = rig.engine.set(0, TintMode::NightLight);
        assert!(!outcome.active);
        assert_eq!(outcome.error, None);
        assert_eq!(rig.take_log(), ["night restore", "clear"]);
        assert_eq!(*lock(&rig.saved), None);
        // Nothing to restore any more.
        rig.engine.set(0, TintMode::NightLight);
        assert!(rig.take_log().is_empty());
    }

    #[test]
    fn gamma_is_reapplied_every_tick_and_reports_capping() {
        let mut rig = rig(|_, gamma, _| gamma.capped = true);
        let outcome = rig.engine.set(100, TintMode::Gamma);
        assert_eq!(
            (outcome.mode, outcome.active, outcome.capped),
            (TintMode::Gamma, true, true)
        );
        assert_eq!(outcome.error.as_deref(), Some("display 2 refused the ramp"));
        rig.engine.set(100, TintMode::Gamma);
        let kelvin = kelvin_for_level(100);
        assert_eq!(
            rig.take_log(),
            [
                "gamma snapshot".to_owned(),
                "save".to_owned(),
                format!("gamma apply {kelvin}"),
                format!("gamma apply {kelvin}"),
            ]
        );
    }

    #[test]
    fn a_display_connected_later_is_added_to_the_restore_file() {
        let grown = Arc::new(Mutex::new(false));
        let flag = Arc::clone(&grown);
        let mut rig = rig(move |_, gamma, _| gamma.grown = flag);
        let apply = format!("gamma apply {}", kelvin_for_level(45));
        rig.engine.set(45, TintMode::Gamma);
        assert_eq!(rig.take_log(), ["gamma snapshot", "save", &apply]);

        // The next apply warmed a display that was not in the snapshot: the file is updated right after.
        *lock(&grown) = true;
        rig.engine.set(45, TintMode::Gamma);
        assert_eq!(rig.take_log(), [apply.as_str(), "save"]);
        assert_eq!(
            lock(&rig.saved).as_ref().and_then(|r| r.gamma.clone()),
            Some(serde_json::json!({ "was": "gamma", "grown": true }))
        );
        // Nothing new on the next tick: no write.
        rig.engine.set(45, TintMode::Gamma);
        assert_eq!(rig.take_log(), [apply.as_str()]);
        // The restore gets the extended snapshot.
        rig.engine.set(0, TintMode::Gamma);
        assert_eq!(rig.take_log(), ["gamma restore grown", "clear"]);
    }

    #[test]
    fn a_mode_change_restores_the_previous_mechanism_first() {
        let mut rig = rig(|_, _, _| {});
        rig.engine.set(45, TintMode::NightLight);
        rig.take_log();
        let outcome = rig.engine.set(45, TintMode::Gamma);
        assert_eq!((outcome.mode, outcome.active), (TintMode::Gamma, true));
        assert_eq!(
            rig.take_log(),
            [
                "gamma snapshot".to_owned(),
                "night restore".to_owned(),
                "clear".to_owned(),
                "save".to_owned(),
                format!("gamma apply {}", kelvin_for_level(45)),
            ]
        );
    }

    #[test]
    fn failing_night_light_falls_back_to_gamma_until_retried() {
        let mut rig = rig(|night, _, _| night.fail_snapshot = true);
        let outcome = rig.engine.set(45, TintMode::NightLight);
        assert_eq!((outcome.mode, outcome.active), (TintMode::Gamma, true));
        assert!(
            outcome
                .error
                .as_deref()
                .is_some_and(|e| e.contains("Night light failed"))
        );
        assert!(outcome.report, "a real failure is reported");
        rig.take_log();
        // Next tick: straight to gamma, the reason is still reported.
        let outcome = rig.engine.set(45, TintMode::NightLight);
        assert_eq!(outcome.mode, TintMode::Gamma);
        assert!(outcome.error.is_some());
        assert_eq!(
            rig.take_log(),
            [format!("gamma apply {}", kelvin_for_level(45))]
        );
        // A config change tries Night light again (still failing: gamma stays).
        rig.engine.retry_night_light();
        rig.engine.set(45, TintMode::NightLight);
        assert_eq!(rig.take_log()[0], "night snapshot");
    }

    #[test]
    fn night_light_unavailable_on_this_windows_uses_gamma_quietly() {
        let mut rig = rig(|night, _, _| night.available = false);
        let outcome = rig.engine.set(45, TintMode::NightLight);
        assert_eq!((outcome.mode, outcome.active), (TintMode::Gamma, true));
        assert_eq!(outcome.error.as_deref(), Some(NIGHT_LIGHT_UNAVAILABLE));
        assert!(
            !outcome.report,
            "an expected limitation is not an error toast"
        );
        // Night light itself is never touched.
        assert!(
            rig.take_log()
                .iter()
                .all(|entry| entry.starts_with("gamma") || entry == "save")
        );
        // Gamma chosen explicitly: no Night light note at all.
        let outcome = rig.engine.set(45, TintMode::Gamma);
        assert_eq!(outcome.error, None);
        // A failing gamma is reported even with the expected Night light note.
        let mut rig = rig_with_failing_gamma();
        let outcome = rig.engine.set(45, TintMode::NightLight);
        assert!(!outcome.active);
        assert!(outcome.report);
        rig.take_log();
    }

    fn rig_with_failing_gamma() -> Rig {
        rig(|night, gamma, _| {
            night.available = false;
            gamma.fail_apply = true;
        })
    }

    #[test]
    fn a_failed_night_light_apply_is_undone_before_the_fallback() {
        let mut rig = rig(|night, _, _| night.fail_apply = true);
        let outcome = rig.engine.set(45, TintMode::NightLight);
        assert_eq!((outcome.mode, outcome.active), (TintMode::Gamma, true));
        let log = rig.take_log();
        let kelvin = kelvin_for_level(45);
        assert_eq!(
            log,
            [
                "night snapshot".to_owned(),
                "save".to_owned(),
                format!("night apply {kelvin}"),
                "night restore".to_owned(),
                "clear".to_owned(),
                "gamma snapshot".to_owned(),
                "save".to_owned(),
                format!("gamma apply {kelvin}"),
            ]
        );
    }

    #[test]
    fn nothing_changes_when_the_restore_file_cannot_be_saved() {
        let mut rig = rig(|_, _, store| store.fail = true);
        let outcome = rig.engine.set(45, TintMode::Gamma);
        assert!(!outcome.active);
        assert!(
            outcome
                .error
                .as_deref()
                .is_some_and(|e| e.contains("restore file"))
        );
        assert_eq!(rig.take_log(), ["gamma snapshot", "save"]);
        assert!(rig.engine.restore.is_empty());
    }

    #[test]
    fn a_failed_restore_stays_pending_and_is_retried() {
        let fail = Arc::new(Mutex::new(true));
        let flag = Arc::clone(&fail);
        let mut rig = rig(move |_, gamma, _| gamma.fail_restore = flag);
        rig.engine.set(45, TintMode::Gamma);
        rig.take_log();
        let outcome = rig.engine.set(0, TintMode::Gamma);
        assert!(outcome.error.is_some());
        assert_eq!(rig.take_log(), ["gamma restore"]);
        assert!(lock(&rig.saved).is_some(), "the restore file stays");

        *lock(&fail) = false;
        let outcome = rig.engine.set(0, TintMode::Gamma);
        assert_eq!(outcome.error, None);
        assert_eq!(rig.take_log(), ["gamma restore", "clear"]);
    }

    #[test]
    fn a_crash_left_behind_is_restored_at_startup() {
        let mut rig = rig(|_, _, _| {});
        let left = TintRestore {
            night_light: Some(serde_json::json!({ "was": "night" })),
            gamma: Some(serde_json::json!({ "was": "gamma" })),
        };
        assert_eq!(rig.engine.restore_after_crash(left, 1), Ok(()));
        assert_eq!(
            rig.take_log(),
            ["night restore", "save", "gamma restore", "clear"]
        );
        assert!(rig.engine.restore.is_empty());
    }

    #[test]
    fn a_failed_crash_restore_re_arms_the_logon_entry() {
        let fail = Arc::new(Mutex::new(true));
        let flag = Arc::clone(&fail);
        let mut rig = rig(move |night, _, _| night.fail_restore = flag);
        let left = TintRestore {
            night_light: Some(serde_json::json!({ "was": "night" })),
            gamma: None,
        };
        assert!(rig.engine.restore_after_crash(left.clone(), 1).is_err());
        assert_eq!(rig.take_log(), ["night restore", "save"]);
        assert_eq!(*lock(&rig.saved), Some(left));
        // The schedule warming the screen later keeps the original snapshot (no new one).
        *lock(&fail) = false;
        rig.engine.set(45, TintMode::NightLight);
        assert_eq!(
            rig.take_log(),
            [format!("night apply {}", kelvin_for_level(45))]
        );
    }

    #[test]
    fn restore_file_uses_camel_case_and_tolerates_missing_keys() {
        let restore = TintRestore {
            night_light: Some(serde_json::json!([1, 2])),
            gamma: None,
        };
        let json = serde_json::to_string(&restore).expect("json");
        assert!(json.contains("\"nightLight\""));
        let empty: TintRestore = serde_json::from_str("{}").expect("empty");
        assert!(empty.is_empty());
    }

    #[test]
    fn utc_fallback_is_within_a_day() {
        let minutes = utc_minutes_of_day();
        assert!((0.0..f64::from(MINUTES_PER_DAY)).contains(&minutes));
    }
}
