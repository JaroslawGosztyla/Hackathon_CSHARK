//! Executes the effects returned by the core session (`Session::handle`).
//!
//! The actor never waits for slow work: chimes go to the audio thread, shields and toasts to the
//! shields worker, tracking to the tracker thread, history writes to the blocking pool.

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use focus_audio::{AudioEvent, Chime, ChimePlayer};
use focus_core::api::{AppError, ChimeKind, SessionSummary, events};
use focus_core::session::{Effect, NotifyKind, SessionTotals};
use tauri::{AppHandle, Emitter};

use crate::persistence::Persistence;
use crate::shields::ShieldsHandle;
use crate::tracker::TrackerHandle;
use crate::{SharedConfig, emit_error, lock, read_config, texts};

/// Longest wait for the tracker's final report when a session ends (normally a few ms).
const FINAL_REPORT_TIMEOUT: Duration = Duration::from_millis(200);

/// Handle to the chime player (audio thread), keyed by the IPC `ChimeKind`.
#[derive(Clone)]
pub struct ChimeHandle(ChimePlayer);

impl ChimeHandle {
    pub fn new(player: ChimePlayer) -> Self {
        Self(player)
    }

    /// Non-blocking.
    pub fn play(&self, kind: ChimeKind, volume: f32) {
        let chime = match kind {
            ChimeKind::BreakStart => Chime::BreakStart,
            ChimeKind::FocusStart => Chime::FocusStart,
        };
        self.0.play(chime, volume);
    }
}

/// Forwards audio thread errors to the UI as `app://error` (`audio.device`).
pub fn forward_audio_errors(app: AppHandle, events_rx: Receiver<AudioEvent>) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("focus-audio-events".into())
        .spawn(move || {
            while let Ok(event) = events_rx.recv() {
                match event {
                    AudioEvent::Error(message) => {
                        emit_error(&app, AppError::new("audio.device", message));
                    }
                }
            }
        })?;
    Ok(())
}

pub struct Effects {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub chime: ChimeHandle,
    pub shields: ShieldsHandle,
    pub tracker: TrackerHandle,
    pub persistence: Persistence,
    pub last_summary: Arc<Mutex<Option<SessionSummary>>>,
}

impl Effects {
    /// Dispatches effects in order. `shutting_down` writes the history synchronously so a session
    /// stopped by "Zakończ" is not lost.
    pub fn dispatch(&self, effects: Vec<Effect>, shutting_down: bool) {
        for effect in effects {
            match effect {
                Effect::Chime(kind) => {
                    let volume = read_config(&self.config).chime_volume;
                    self.chime.play(kind, volume);
                }
                Effect::Notify(kind) => self.shields.toast(match kind {
                    NotifyKind::BreakStarted { break_secs } => texts::break_toast(break_secs),
                    NotifyKind::FocusStarted { focus_secs, cycle } => {
                        texts::focus_toast(focus_secs, cycle)
                    }
                    NotifyKind::Interrupted => texts::interrupted_toast(),
                }),
                Effect::Shields(up) => self.shields.set_up(up),
                Effect::Tracking(on) => self.tracker.set_enabled(on),
                Effect::PhaseChanged(change) => {
                    log::info!(
                        "phase {:?} -> {:?} (cycle {})",
                        change.from,
                        change.to,
                        change.cycle
                    );
                    if let Err(err) = self.app.emit(events::SESSION_PHASE_CHANGED, change) {
                        log::warn!("cannot emit phase change: {err}");
                    }
                }
                Effect::SessionEnded(totals) => self.finish_session(&totals, shutting_down),
            }
        }
    }

    fn finish_session(&self, totals: &SessionTotals, shutting_down: bool) {
        // `Tracking(false)` was just queued to the tracker: wait (bounded) until it is applied so the
        // last partial sample is in the summary.
        let usage = self.tracker.settled_report(FINAL_REPORT_TIMEOUT);
        let summary = focus_core::stats::summarize(totals, usage);
        log::info!(
            "session {} ended: {} s focused, {} cycles",
            summary.session_id,
            summary.focused_secs,
            summary.cycles_completed
        );
        *lock(&self.last_summary) = Some(summary.clone());
        if let Err(err) = self.app.emit(events::SESSION_SUMMARY, summary.clone()) {
            log::warn!("cannot emit session summary: {err}");
        }

        if shutting_down {
            if let Err(err) = self.persistence.append_history(&summary) {
                log::error!("cannot save session history: {err:#}");
            }
            return;
        }
        let persistence = self.persistence.clone();
        let app = self.app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(err) = persistence.append_history(&summary) {
                emit_error(
                    &app,
                    AppError::new(
                        "persistence.history",
                        format!("cannot save session history: {err:#}"),
                    ),
                );
            }
        });
    }
}
