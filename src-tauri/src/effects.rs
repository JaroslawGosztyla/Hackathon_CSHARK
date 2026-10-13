//! Executes the effects returned by the core session (`Session::handle`).
//!
//! The actor never waits for slow work: chimes go to the audio thread, shields and toasts to the
//! shields worker, phase popups to the popup thread, tracking to the tracker thread, history writes to
//! the blocking pool.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use focus_audio::{AudioEvent, Chime, ChimePlayer, ChimeSource, SoundPack};
use focus_core::api::{
    AppError, ChimeKind, ChimeSound, Config, MeetingConflict, PopupKind, SessionSummary, events,
};
use focus_core::config::reminder_for_break;
use focus_core::session::{Effect, NotifyKind, SessionTotals};
use tauri::{AppHandle, Emitter};

use crate::persistence::Persistence;
use crate::popup::{PopupHandle, PopupRequest};
use crate::shields::ShieldsHandle;
use crate::tracker::{TrackerHandle, without_titles};
use crate::{SharedConfig, chimes, emit_error, lock, read_config, texts};

/// Longest wait for the tracker's final report when a session ends (normally a few ms).
const FINAL_REPORT_TIMEOUT: Duration = Duration::from_millis(200);

/// Handle to the chime player (audio thread), keyed by the IPC `ChimeKind`.
#[derive(Clone)]
pub struct ChimeHandle {
    player: ChimePlayer,
    /// Folder of the imported chime files (`Persistence::sounds_dir`).
    sounds_dir: PathBuf,
}

impl ChimeHandle {
    pub fn new(player: ChimePlayer, sounds_dir: PathBuf) -> Self {
        Self { player, sounds_dir }
    }

    /// Plays `kind` with the configured sound (`chime_sound`, `custom_chimes`) and volume. Non-blocking:
    /// a custom file is read and decoded on the audio thread.
    pub fn play(&self, kind: ChimeKind, config: &Config) {
        let chime = match kind {
            ChimeKind::BreakStart => Chime::BreakStart,
            ChimeKind::FocusStart => Chime::FocusStart,
        };
        let source = chime_source(config, kind, &self.sounds_dir);
        self.player.play(chime, source, config.chime_volume);
    }
}

/// The sound for `kind`: the configured pack, or the user's file for `custom` (its own file, else the other
/// one; the default pack when none is set).
fn chime_source(config: &Config, kind: ChimeKind, sounds_dir: &Path) -> ChimeSource {
    let pack = match config.chime_sound {
        ChimeSound::Midnight => SoundPack::Midnight,
        ChimeSound::Vaporwave => SoundPack::Vaporwave,
        ChimeSound::Lunar => SoundPack::Lunar,
        ChimeSound::Autumn => SoundPack::Autumn,
        ChimeSound::Classic => SoundPack::Classic,
        ChimeSound::Soft => SoundPack::Soft,
        ChimeSound::Chiptune => SoundPack::Chiptune,
        ChimeSound::Custom => {
            return config
                .custom_chimes
                .file_for(kind)
                .and_then(|name| chimes::path_of(sounds_dir, name))
                .map_or(ChimeSource::Pack(SoundPack::Midnight), ChimeSource::File);
        }
    };
    ChimeSource::Pack(pack)
}

/// Blocking: `Ok` when `path` is an audio file the chime player can decode (`import_chime`).
pub fn probe_chime_file(path: &Path) -> Result<(), String> {
    focus_audio::probe_file(path)
        .map(|_| ())
        .map_err(|err| err.to_string())
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

/// Details of a break that started paused because of a meeting (`NotifyKind::BreakHeld`), known only to
/// the session actor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldBreak {
    /// Break length of the running session.
    pub break_secs: u32,
    /// The meeting holding the break, when the actor still knows it.
    pub meeting: Option<MeetingConflict>,
}

pub struct Effects {
    pub app: AppHandle,
    pub config: SharedConfig,
    pub chime: ChimeHandle,
    pub shields: ShieldsHandle,
    pub popup: PopupHandle,
    pub tracker: TrackerHandle,
    pub persistence: Persistence,
    pub last_summary: Arc<Mutex<Option<SessionSummary>>>,
    /// Cycle of the last phase change (a break popup shows which focus cycle just ended).
    pub last_cycle: u32,
}

impl Effects {
    /// Dispatches effects in order. `shutting_down` writes the history synchronously so a session
    /// stopped by "Zakończ" is not lost. `held` describes a `Notify(BreakHeld)` among the effects.
    pub fn dispatch(
        &mut self,
        effects: Vec<Effect>,
        shutting_down: bool,
        held: Option<&HeldBreak>,
    ) {
        for effect in effects {
            match effect {
                Effect::Chime(kind) => self.chime.play(kind, &read_config(&self.config)),
                Effect::Notify(kind) => self.notify(kind, held),
                Effect::Shields(up) => self.shields.set_up(up),
                Effect::Tracking(on) => self.tracker.set_enabled(on),
                Effect::PhaseChanged(change) => {
                    self.last_cycle = change.cycle;
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

    /// The app's own popup when `phase_popups` is on, otherwise the Windows toast (never both).
    fn notify(&self, kind: NotifyKind, held: Option<&HeldBreak>) {
        let config = read_config(&self.config);
        let request = notification(kind, self.last_cycle, &config, held);
        if config.phase_popups {
            self.popup.show(request);
        } else {
            self.shields.toast(request.fallback);
        }
    }

    fn finish_session(&self, totals: &SessionTotals, shutting_down: bool) {
        // `Tracking(false)` was just queued to the tracker: wait (bounded) until it is applied so the
        // last partial sample is in the summary.
        let config = read_config(&self.config);
        let mut usage = self.tracker.settled_report(FINAL_REPORT_TIMEOUT);
        if !config.track_titles {
            usage = without_titles(usage);
        }
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

        if !config.keep_history {
            // The summary is shown (and kept in memory until the app quits), but not stored.
            log::info!("session history is off; the summary is not saved");
            return;
        }
        if shutting_down {
            if let Err(err) = self
                .persistence
                .append_history(&summary, &self.last_summary)
            {
                log::error!("cannot save session history: {err:#}");
            }
            return;
        }
        let persistence = self.persistence.clone();
        let latest = Arc::clone(&self.last_summary);
        let app = self.app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(err) = persistence.append_history(&summary, &latest) {
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

/// Popup (and its toast fallback) of a session notification. `last_cycle` is the cycle of the latest
/// phase change: for a break, the focus cycle that just ended, which is also the break's number
/// (the break after cycle 1 is break 1) for the user's per-break reminder. `held` details a `BreakHeld`.
fn notification(
    kind: NotifyKind,
    last_cycle: u32,
    config: &Config,
    held: Option<&HeldBreak>,
) -> PopupRequest {
    let request = |kind, cycle, phase_secs, reminder, fallback| PopupRequest {
        reminder,
        ..PopupRequest::new(kind, cycle, phase_secs, fallback)
    };
    match kind {
        NotifyKind::BreakHeld { subject } => {
            let held = held.cloned().unwrap_or_default();
            let reminder = reminder_for_break(config, last_cycle);
            let toast =
                texts::break_held_toast(subject.as_deref(), held.break_secs, reminder.as_deref());
            PopupRequest {
                meeting: held.meeting,
                ..request(
                    PopupKind::BreakHeld,
                    last_cycle,
                    held.break_secs,
                    reminder,
                    toast,
                )
            }
        }
        NotifyKind::BreakStarted { break_secs } => {
            let reminder = reminder_for_break(config, last_cycle);
            let toast = texts::break_toast(break_secs, reminder.as_deref());
            request(
                PopupKind::BreakStarted,
                last_cycle,
                break_secs,
                reminder,
                toast,
            )
        }
        NotifyKind::FocusStarted { focus_secs, cycle } => request(
            PopupKind::FocusStarted,
            cycle,
            focus_secs,
            None,
            texts::focus_toast(focus_secs, cycle),
        ),
        NotifyKind::Interrupted => request(
            PopupKind::Interrupted,
            last_cycle,
            0,
            None,
            texts::interrupted_toast(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifications_map_to_popups_and_toasts() {
        let config = Config::default();
        let popup = notification(
            NotifyKind::BreakStarted { break_secs: 900 },
            3,
            &config,
            None,
        );
        assert_eq!(
            (popup.kind, popup.cycle, popup.phase_secs),
            (PopupKind::BreakStarted, 3, 900)
        );
        assert_eq!(popup.reminder, None);
        assert_eq!(popup.fallback, texts::break_toast(900, None));

        let popup = notification(
            NotifyKind::FocusStarted {
                focus_secs: 2700,
                cycle: 4,
            },
            3,
            &config,
            None,
        );
        assert_eq!(
            (popup.kind, popup.cycle, popup.phase_secs),
            (PopupKind::FocusStarted, 4, 2700)
        );
        assert_eq!(popup.fallback, texts::focus_toast(2700, 4));

        let popup = notification(NotifyKind::Interrupted, 2, &config, None);
        assert_eq!(
            (popup.kind, popup.cycle, popup.phase_secs),
            (PopupKind::Interrupted, 2, 0)
        );
        assert_eq!(popup.meeting, None);
    }

    #[test]
    fn a_held_break_names_its_meeting() {
        let config = Config {
            break_reminders: vec!["woda".into()],
            ..Config::default()
        };
        let conflict = MeetingConflict {
            meeting: focus_core::api::Meeting {
                id: "ics:1:0".into(),
                source: focus_core::api::CalendarSource::Ics,
                subject: "Planowanie".into(),
                start_ms: 0,
                end_ms: 1_800_000,
                online: true,
            },
            break_number: 1,
            break_start_ms: 600_000,
            break_end_ms: 1_500_000,
            choice: Some(focus_core::api::MeetingChoice::PauseAtBreak),
            next_reminder_ms: None,
            remind_later_allowed: false,
            too_long: false,
        };
        let held = HeldBreak {
            break_secs: 900,
            meeting: Some(conflict.clone()),
        };
        let popup = notification(
            NotifyKind::BreakHeld {
                subject: Some("Planowanie".into()),
            },
            1,
            &config,
            Some(&held),
        );
        assert_eq!(
            (popup.kind, popup.cycle, popup.phase_secs),
            (PopupKind::BreakHeld, 1, 900)
        );
        assert_eq!(popup.meeting, Some(conflict));
        assert_eq!(popup.reminder.as_deref(), Some("woda"));
        assert_eq!(
            popup.fallback,
            texts::break_held_toast(Some("Planowanie"), 900, Some("woda"))
        );

        // Without details (the actor lost the meeting) the popup still says the break is held.
        let popup = notification(NotifyKind::BreakHeld { subject: None }, 2, &config, None);
        assert_eq!((popup.kind, popup.phase_secs), (PopupKind::BreakHeld, 0));
        assert_eq!(popup.meeting, None);
        assert!(
            popup
                .fallback
                .body
                .starts_with("Przerwa wstrzymana — trwa spotkanie.")
        );
    }

    #[test]
    fn chime_sound_picks_the_pack_or_the_custom_file() {
        let sounds = Path::new("sounds");
        let mut config = Config::default();
        assert_eq!(
            chime_source(&config, ChimeKind::BreakStart, sounds),
            ChimeSource::Pack(SoundPack::Midnight)
        );
        config.chime_sound = ChimeSound::Chiptune;
        assert_eq!(
            chime_source(&config, ChimeKind::FocusStart, sounds),
            ChimeSource::Pack(SoundPack::Chiptune)
        );

        // Custom without files: the default sound.
        config.chime_sound = ChimeSound::Custom;
        assert_eq!(
            chime_source(&config, ChimeKind::FocusStart, sounds),
            ChimeSource::Pack(SoundPack::Midnight)
        );
        // Only a break file: used for both chimes.
        config.custom_chimes.break_file = Some("custom-break-1.mp3".into());
        let break_file = ChimeSource::File(sounds.join("custom-break-1.mp3"));
        assert_eq!(
            chime_source(&config, ChimeKind::BreakStart, sounds),
            break_file
        );
        assert_eq!(
            chime_source(&config, ChimeKind::FocusStart, sounds),
            break_file
        );
        config.custom_chimes.focus_file = Some("custom-focus-2.wav".into());
        assert_eq!(
            chime_source(&config, ChimeKind::FocusStart, sounds),
            ChimeSource::File(sounds.join("custom-focus-2.wav"))
        );
        // A name that would leave the folder is never used.
        config.custom_chimes.focus_file = Some("../x.mp3".into());
        config.custom_chimes.break_file = None;
        assert_eq!(
            chime_source(&config, ChimeKind::FocusStart, sounds),
            ChimeSource::Pack(SoundPack::Midnight)
        );
    }

    #[test]
    fn each_break_gets_its_own_reminder() {
        let config = Config {
            break_reminders: vec!["rozciąganie".into(), String::new(), "obiad".into()],
            ..Config::default()
        };
        let reminder = |cycle| {
            notification(
                NotifyKind::BreakStarted { break_secs: 600 },
                cycle,
                &config,
                None,
            )
            .reminder
        };
        assert_eq!(reminder(1).as_deref(), Some("rozciąganie"));
        assert_eq!(reminder(2), None);
        assert_eq!(reminder(3).as_deref(), Some("obiad"));
        assert_eq!(reminder(4), None);

        let popup = notification(
            NotifyKind::BreakStarted { break_secs: 600 },
            3,
            &config,
            None,
        );
        assert_eq!(popup.fallback, texts::break_toast(600, Some("obiad")));
        // Focus popups never carry a break reminder.
        let focus = notification(
            NotifyKind::FocusStarted {
                focus_secs: 60,
                cycle: 1,
            },
            0,
            &config,
            None,
        );
        assert_eq!(focus.reminder, None);
    }
}
