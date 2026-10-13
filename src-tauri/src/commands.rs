//! Thin `#[tauri::command]` adapters (names = `focus_core::api::commands`).
//!
//! All commands are `async` so they run on the async runtime, never on the main thread
//! (several of them wait for a worker reply; the main thread must stay free for the event loop).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use focus_core::api::{
    AppError, BRIDGE_PORT, BlocklistInfo, BridgeInfo, CalendarSource, CalendarStatus,
    CelebrationLevel, ChimeKind, ChimeSound, Config, DevAction, DeviceCodePrompt, Language,
    MediaAction, MediaWindowStatus, MeetingChoice, Phase, PopupAction, PopupInfo, PrivacyItemKind,
    PrivacyReport, RewardSound, RewardsStatus, SessionGoal, SessionSnapshot, SessionSummary,
    ShieldStatus, StepDraft, TeamsStatus, TintStatus, Track, UsageReport, WorkSource, WorkStatus,
    YoutubeAccount,
};
use focus_core::blocklist::BLACKLIST_FILE_NAME;
use focus_core::session::normalize_steps;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::session_actor::ActorCmd;
use crate::{
    AppState, calendar, chimes, close, dev, effects, emit_error, lock, music, privacy, read_config,
    tracker, work, write_config,
};

/// Upper bound for `list_history(limit)`.
const MAX_HISTORY: u32 = 500;
/// Longest session id accepted by `mark_goal_done` (ids are `<ms>-<pid>-<n>`).
const MAX_SESSION_ID_LEN: usize = 128;
/// `dev_trigger` while `Config::developer_mode` is off (ADR-019).
const ERR_DEVELOPER_MODE_OFF: &str = "developerModeOff";
/// `set_session_steps` / `set_step_done` while small steps are off (beta, ADR-022).
const ERR_STEPS_OFF: &str = "stepsOff";

#[tauri::command]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<SessionSnapshot, AppError> {
    state.actor.snapshot().await
}

#[tauri::command]
pub async fn start_session(state: State<'_, AppState>) -> Result<(), AppError> {
    state.actor.send(ActorCmd::Start)
}

#[tauri::command]
pub async fn pause_session(state: State<'_, AppState>) -> Result<(), AppError> {
    state.actor.send(ActorCmd::Pause)
}

#[tauri::command]
pub async fn resume_session(state: State<'_, AppState>) -> Result<(), AppError> {
    state.actor.send(ActorCmd::Resume)
}

#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>) -> Result<(), AppError> {
    state.actor.send(ActorCmd::Stop)
}

#[tauri::command]
pub async fn skip_phase(state: State<'_, AppState>) -> Result<(), AppError> {
    state.actor.send(ActorCmd::Skip)
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, AppError> {
    Ok(read_config(&state.config))
}

#[tauri::command]
pub async fn update_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Config,
) -> Result<Config, AppError> {
    // Serializes concurrent updates so the file and memory always end with the last call.
    let _serial = state.config_update.lock().await;
    store_config(&app, &state, config).await
}

/// Validates, persists and applies a new config (shared by `update_config`, `resolve_close` and the privacy
/// deletes). The caller holds `state.config_update`.
pub(crate) async fn store_config(
    app: &AppHandle,
    state: &AppState,
    config: Config,
) -> Result<Config, AppError> {
    let normalized = config.normalized()?;
    let previous = read_config(&state.config);
    if normalized == previous {
        return Ok(normalized);
    }

    let persistence = state.persistence.clone();
    let to_save = normalized.clone();
    let saved =
        tauri::async_runtime::spawn_blocking(move || persistence.save_config(&to_save)).await;
    match saved {
        Ok(Ok(())) => {}
        Ok(Err(err)) => emit_error(
            app,
            AppError::new("config.save", format!("cannot save config: {err:#}")),
        ),
        Err(err) => emit_error(
            app,
            AppError::new("config.save", format!("cannot save config: {err}")),
        ),
    }

    write_config(&state.config, normalized.clone());
    if normalized.language != previous.language {
        // Before `ConfigChanged`, so the actor's tray refresh already uses the new labels.
        crate::apply_language(app, normalized.language);
    }
    state.shields.config_changed();
    if let Err(err) = state.actor.send(ActorCmd::ConfigChanged) {
        log::warn!("cannot refresh the idle profile: {}", err.message);
    }
    if normalized.shields.teams_client_id != previous.shields.teams_client_id
        || normalized.shields.teams_tenant != previous.shields.teams_tenant
    {
        state.teams.config_changed();
    }
    if normalized.music.folder != previous.music.folder {
        music::allow_configured_folder(app, normalized.music.folder.as_deref());
    }
    if normalized.launch_on_login != previous.launch_on_login {
        crate::sync_autostart(app, normalized.launch_on_login);
    }
    if normalized.tint != previous.tint {
        state.tint.config_changed();
    }
    if normalized.calendar != previous.calendar {
        state.calendar.config_changed();
    }
    if normalized.work != previous.work {
        state.work.config_changed();
    }
    if previous.developer_mode && !normalized.developer_mode {
        // A sticky sample has no timeout: it goes with developer mode (ADR-019).
        state.popup.withdraw_samples();
    }
    // Beta rewards (ADR-022): pause bookkeeping, a lower daily goal, the UI shows or hides the rewards.
    let phase = current_phase(state).await;
    state.rewards.config_changed(&previous, &normalized, phase);
    Ok(normalized)
}

#[tauri::command]
pub async fn resolve_close(
    app: AppHandle,
    state: State<'_, AppState>,
    to_tray: bool,
) -> Result<Config, AppError> {
    let saved = {
        let _serial = state.config_update.lock().await;
        let mut config = read_config(&state.config);
        config.close_to_tray = Some(to_tray);
        if to_tray {
            // Hidden in the tray, the app tells about phase changes with its own popups.
            config.phase_popups = true;
        }
        store_config(&app, &state, config).await?
    };
    state.close.resolved();
    if to_tray {
        close::hide_main_window(&app);
    } else {
        log::info!("quitting at the user's request (closeToTray = false)");
        // The normal exit path stops the session and lowers every shield (`RunEvent::ExitRequested`).
        app.exit(0);
    }
    Ok(saved)
}

#[tauri::command]
pub async fn get_system_language() -> Result<Language, AppError> {
    Ok(focus_platform::locale::system_language_tag()
        .and_then(|tag| Language::from_tag(&tag))
        .unwrap_or(Language::En))
}

#[tauri::command]
pub async fn get_popup(state: State<'_, AppState>) -> Result<PopupInfo, AppError> {
    Ok(state.popup.info())
}

#[tauri::command]
pub async fn popup_action(
    app: AppHandle,
    state: State<'_, AppState>,
    action: PopupAction,
) -> Result<(), AppError> {
    match action {
        PopupAction::Dismiss => state.popup.dismiss(),
        PopupAction::ShowMain => {
            state.popup.close();
            crate::show_main_window(&app);
        }
        PopupAction::SkipPhase => {
            // The popup thread closes the popup and decides whether the button was on a developer-mode sample
            // (ADR-019), which only closes, from the popup the button belonged to (not from a newer one).
            let actor = state.actor.clone();
            state.popup.skip_phase(move || {
                if let Err(err) = actor.send(ActorCmd::Skip) {
                    log::warn!("cannot skip the phase from the popup: {}", err.message);
                }
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_tint_status(state: State<'_, AppState>) -> Result<TintStatus, AppError> {
    Ok(state.tint.status())
}

#[tauri::command]
pub async fn preview_tint(
    state: State<'_, AppState>,
    strength: Option<u32>,
) -> Result<(), AppError> {
    state.tint.preview(strength);
    Ok(())
}

#[tauri::command]
pub async fn list_folder_tracks(app: AppHandle, path: String) -> Result<Vec<Track>, AppError> {
    let dir = PathBuf::from(path.trim());
    // Scanning and allowing up to 2000 files touches the disk: keep it off the async workers.
    tauri::async_runtime::spawn_blocking(move || music::scan_and_allow(&app, &dir))
        .await
        .map_err(|err| AppError::new("music.folder", format!("folder scan failed: {err}")))?
}

#[tauri::command]
pub async fn list_library_tracks(app: AppHandle) -> Result<Vec<Track>, AppError> {
    // Reads library.json and allows each bundled file in the asset scope: keep it off the async workers.
    tauri::async_runtime::spawn_blocking(move || music::library_tracks(&app))
        .await
        .map_err(|err| AppError::new("music.library", format!("library scan failed: {err}")))?
}

#[tauri::command]
pub async fn get_shield_status(state: State<'_, AppState>) -> Result<ShieldStatus, AppError> {
    Ok(state.shields.status())
}

#[tauri::command]
pub async fn get_current_usage(state: State<'_, AppState>) -> Result<UsageReport, AppError> {
    let snapshot = state.actor.snapshot().await?;
    if snapshot.phase == Phase::Idle {
        return Ok(UsageReport::default());
    }
    let report = state.tracker.report();
    Ok(if read_config(&state.config).track_titles {
        report
    } else {
        tracker::without_titles(report)
    })
}

#[tauri::command]
pub async fn get_last_summary(
    state: State<'_, AppState>,
) -> Result<Option<SessionSummary>, AppError> {
    Ok(lock(&state.last_summary).clone())
}

#[tauri::command]
pub async fn list_history(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<SessionSummary>, AppError> {
    let limit = usize::try_from(limit.clamp(1, MAX_HISTORY)).unwrap_or(20);
    let persistence = state.persistence.clone();
    tauri::async_runtime::spawn_blocking(move || persistence.list_history(limit))
        .await
        .map_err(|err| AppError::new("persistence.history", err.to_string()))?
        .map_err(|err| AppError::new("persistence.history", format!("{err:#}")))
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), AppError> {
    let persistence = state.persistence.clone();
    tauri::async_runtime::spawn_blocking(move || persistence.clear_history())
        .await
        .map_err(|err| AppError::new("persistence.history", err.to_string()))?
        .map_err(|err| {
            AppError::new(
                "persistence.history",
                format!("cannot delete the session history: {err:#}"),
            )
        })?;
    *lock(&state.last_summary) = None;
    log::info!("session history deleted at the user's request");
    Ok(())
}

#[tauri::command]
pub async fn get_teams_status(state: State<'_, AppState>) -> Result<TeamsStatus, AppError> {
    Ok(state.teams.status())
}

#[tauri::command]
pub async fn teams_connect(state: State<'_, AppState>) -> Result<DeviceCodePrompt, AppError> {
    state.teams.connect().await
}

#[tauri::command]
pub async fn teams_disconnect(state: State<'_, AppState>) -> Result<(), AppError> {
    state.teams.disconnect().await
}

#[tauri::command]
pub async fn get_calendar(state: State<'_, AppState>) -> Result<CalendarStatus, AppError> {
    Ok(state.calendar.status())
}

#[tauri::command]
pub async fn refresh_calendar(state: State<'_, AppState>) -> Result<CalendarStatus, AppError> {
    state.calendar.refresh().await
}

#[tauri::command]
pub async fn set_meeting_choice(
    state: State<'_, AppState>,
    meeting_id: String,
    choice: MeetingChoice,
) -> Result<CalendarStatus, AppError> {
    if dev::is_sample_meeting(&meeting_id) {
        // A developer-mode sample (ADR-019): its popup closes like an answered one, nothing is stored
        // (`meeting-choices.json`), sent to the session or synced.
        state.popup.resolve_meeting(&meeting_id);
        return Ok(state.calendar.status());
    }
    state.calendar.set_choice(meeting_id, choice).await
}

#[tauri::command]
pub async fn connect_google(state: State<'_, AppState>) -> Result<(), AppError> {
    state.calendar.connect_google().await
}

#[tauri::command]
pub async fn disconnect_calendar(
    app: AppHandle,
    state: State<'_, AppState>,
    source: CalendarSource,
) -> Result<CalendarStatus, AppError> {
    if source == CalendarSource::Microsoft {
        // The Microsoft calendar shares the Teams sign-in, which stays (Teams presence keeps working):
        // only the calendar is switched off, saved like an `update_config`.
        let _serial = state.config_update.lock().await;
        let mut config = read_config(&state.config);
        if config.calendar.microsoft {
            config.calendar.microsoft = false;
            store_config(&app, &state, config).await?;
        }
    }
    // Drops the source's meetings (and for Google / ICS the stored token / links).
    state.calendar.disconnect(source).await
}

#[tauri::command]
pub async fn set_ics_urls(
    state: State<'_, AppState>,
    urls: Vec<String>,
) -> Result<CalendarStatus, AppError> {
    let urls = calendar::normalize_ics_urls(&urls)?;
    state.calendar.set_ics_urls(urls).await
}

#[tauri::command]
pub async fn get_ics_urls(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    state.calendar.ics_urls().await
}

#[tauri::command]
pub async fn get_work(state: State<'_, AppState>) -> Result<WorkStatus, AppError> {
    state.work.get().await
}

#[tauri::command]
pub async fn refresh_work(state: State<'_, AppState>) -> Result<WorkStatus, AppError> {
    state.work.refresh().await
}

#[tauri::command]
pub async fn set_work_token(
    state: State<'_, AppState>,
    source: WorkSource,
    token: Option<String>,
) -> Result<WorkStatus, AppError> {
    // Checked here, sealed on the work thread; never logged.
    let token = token.as_deref().map(work::validate_token).transpose()?;
    state.work.set_token(source, token).await
}

#[tauri::command]
pub async fn set_session_goal(
    state: State<'_, AppState>,
    goal: Option<SessionGoal>,
) -> Result<(), AppError> {
    state.actor.send(ActorCmd::SetGoal(goal))
}

#[tauri::command]
pub async fn mark_goal_done(
    state: State<'_, AppState>,
    session_id: String,
    done: bool,
) -> Result<(), AppError> {
    let unknown = || {
        AppError::new(
            "history.unknownSession",
            "the session is not in the history",
        )
    };
    if session_id.is_empty() || session_id.len() > MAX_SESSION_ID_LEN {
        return Err(unknown());
    }
    let config = read_config(&state.config);
    let keep_history = config.keep_history;
    // Beta rewards (ADR-022): "Tak" for the latest session of this run pays once (a demo session's answer pays into
    // the demo preview, never stored); older history entries never do. The sparks paid go into the stored summary
    // too, so the history row and the summary card agree.
    let latest_demo = lock(&state.last_summary)
        .as_ref()
        .filter(|summary| summary.session_id == session_id)
        .map(|summary| summary.demo);
    let mut bonus = 0;
    if done && let Some(demo) = latest_demo {
        let phase = current_phase(&state).await;
        bonus = state.rewards.goal_done(&session_id, &config, phase, demo);
    }
    let persistence = state.persistence.clone();
    let latest = Arc::clone(&state.last_summary);
    let marked_id = session_id.clone();
    // Rewrites history.jsonl: keep it off the async workers.
    let marked = tauri::async_runtime::spawn_blocking(move || {
        persistence.mark_goal_done(&marked_id, done, bonus, &latest)
    })
    .await
    .map_err(|err| AppError::new("persistence.history", err.to_string()))?
    .map_err(|err| {
        AppError::new(
            "persistence.history",
            format!("cannot update the session history: {err:#}"),
        )
    })?;
    // With the history off a session that is not in the file only lives in memory (or is gone): no-op.
    if keep_history && !marked.in_memory && !marked.in_file {
        return Err(unknown());
    }
    Ok(())
}

/// A button on the developer page (ADR-019). Only while `developerMode` is on; sample data stays in memory
/// and nothing leaves the computer. Presentation mode is kept in memory only; the factory reset deletes the app's
/// own data (`privacy::factory_reset`) and restarts the app.
#[tauri::command]
pub async fn dev_trigger(
    app: AppHandle,
    state: State<'_, AppState>,
    action: DevAction,
) -> Result<(), AppError> {
    let config = read_config(&state.config);
    if !config.developer_mode {
        return Err(AppError::new(
            ERR_DEVELOPER_MODE_OFF,
            "developer mode is off",
        ));
    }
    log::info!("developer mode: {action:?}");
    match action {
        DevAction::Popup {
            kind,
            lead_minutes,
            sticky,
        } => {
            let snapshot = state.actor.snapshot().await?;
            let samples = dev::Samples {
                snapshot: &snapshot,
                config: &config,
                now_ms: wall_ms(),
                local_minutes: focus_platform::clock::local_minutes_of_day(),
            };
            // Shown whatever `phasePopups` says, and at once (also while the user presents).
            state.popup.show(samples.popup(kind, lead_minutes, sticky));
        }
        DevAction::Toast { phase } => {
            let snapshot = state.actor.snapshot().await?;
            let samples = dev::Samples {
                snapshot: &snapshot,
                config: &config,
                now_ms: wall_ms(),
                local_minutes: None,
            };
            state.shields.toast(samples.toast(phase)?);
        }
        DevAction::SetRemaining { secs } => state.actor.dev_set_remaining(secs).await?,
        DevAction::PresentationMode { on } => state.actor.set_presentation_mode(on).await?,
        DevAction::FactoryReset => privacy::factory_reset(&app, &state).await?,
    }
    Ok(())
}

// --- Privacy panel (ADR-020, `privacy.rs`) --------------------------------------------------------

#[tauri::command]
pub async fn get_privacy_report(state: State<'_, AppState>) -> Result<PrivacyReport, AppError> {
    state.privacy.report().await
}

#[tauri::command]
pub async fn open_data_folder(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    privacy::open_data_folder(&app, &state).await
}

#[tauri::command]
pub async fn export_privacy_data(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    privacy::export(&state, &path).await
}

#[tauri::command]
pub async fn delete_privacy_data(
    app: AppHandle,
    state: State<'_, AppState>,
    kinds: Vec<PrivacyItemKind>,
) -> Result<Config, AppError> {
    privacy::delete(&app, &state, &kinds).await
}

#[tauri::command]
pub async fn delete_all_data(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Config, AppError> {
    privacy::delete_all(&app, &state).await
}

// --- YouTube sign-in (ADR-021, `youtube_account.rs`) --------------------------------------------
// Async on purpose: the sign-in window is created here, and WebView2 cookie calls deadlock on the main thread.

/// While the state is still unknown (every startup cookie read failed), this also starts one more check: the
/// "Check again" action in Settings is this getter (the result arrives as `youtube://account`).
#[tauri::command]
pub async fn get_youtube_account(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<YoutubeAccount, AppError> {
    Ok(state.youtube.status_or_recheck(&app))
}

#[tauri::command]
pub async fn youtube_sign_in(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    state.youtube.sign_in(&app).await
}

#[tauri::command]
pub async fn youtube_sign_out(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    state.youtube.sign_out(&app).await
}

// --- Sparks, streak and small steps (beta, ADR-022, `rewards.rs`) -----------------------------------------
// Step texts are never logged; the rewards log carries kinds and counts only.

#[tauri::command]
pub async fn get_rewards(state: State<'_, AppState>) -> Result<RewardsStatus, AppError> {
    Ok(state.rewards.status(&read_config(&state.config)))
}

#[tauri::command]
pub async fn ack_reward_moments(state: State<'_, AppState>, up_to: u32) -> Result<(), AppError> {
    state.rewards.ack(up_to, &read_config(&state.config));
    Ok(())
}

#[tauri::command]
pub async fn reset_rewards(state: State<'_, AppState>) -> Result<RewardsStatus, AppError> {
    Ok(state.rewards.reset(&read_config(&state.config)))
}

/// A reward sound, unless the rewards or their sounds are off, celebrations are numbers only or the user presents;
/// it waits (here, never in the actor) until a phase chime that plays now has finished.
#[tauri::command]
pub async fn play_reward_sound(
    state: State<'_, AppState>,
    sound: RewardSound,
) -> Result<(), AppError> {
    let config = read_config(&state.config);
    if !config.rewards_active()
        || !config.rewards.sounds
        || config.rewards.celebrations == CelebrationLevel::NumbersOnly
        || focus_platform::notification_state::user_is_presenting()
    {
        return Ok(());
    }
    let Some(wait) = state.chime.reward_sound_wait() else {
        log::debug!("reward sound {sound:?} dropped: a long phase chime plays");
        return Ok(());
    };
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
    // The settings may have changed meanwhile.
    let config = read_config(&state.config);
    if config.rewards_active() && config.rewards.sounds {
        state.chime.play_reward(sound, &config);
    }
    Ok(())
}

#[tauri::command]
pub async fn set_session_steps(
    state: State<'_, AppState>,
    steps: Vec<StepDraft>,
) -> Result<(), AppError> {
    if !read_config(&state.config).steps_active() {
        return Err(steps_off());
    }
    let steps = normalize_steps(steps)?;
    state.actor.send(ActorCmd::SetSteps(steps))
}

#[tauri::command]
pub async fn set_step_done(
    state: State<'_, AppState>,
    id: u32,
    done: bool,
) -> Result<(), AppError> {
    if !read_config(&state.config).steps_active() {
        return Err(steps_off());
    }
    state.actor.send(ActorCmd::SetStepDone { id, done })
}

fn steps_off() -> AppError {
    AppError::new(ERR_STEPS_OFF, "small steps are switched off")
}

/// The phase now (for the "focus is sacred" deferral of reward moments); `Idle` when the actor cannot answer.
async fn current_phase(state: &AppState) -> Phase {
    state
        .actor
        .snapshot()
        .await
        .map_or(Phase::Idle, |snapshot| snapshot.phase)
}

#[tauri::command]
pub async fn get_bridge_info(state: State<'_, AppState>) -> Result<BridgeInfo, AppError> {
    let extension_seen_secs_ago = lock(&state.bridge).seen_secs_ago();
    Ok(BridgeInfo {
        port: BRIDGE_PORT,
        extension_dir: state
            .extension_dir
            .as_ref()
            .map(|dir| dir.to_string_lossy().into_owned()),
        extension_seen_secs_ago,
    })
}

#[tauri::command]
pub async fn open_extension_dir(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let Some(dir) = state.extension_dir.as_ref() else {
        return Err(AppError::new(
            "extension.missing",
            "the browser extension folder was not found",
        ));
    };
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|err| {
            AppError::new(
                "extension.open",
                format!("cannot open {}: {err}", dir.display()),
            )
        })
}

#[tauri::command]
pub async fn test_chime(state: State<'_, AppState>, chime: ChimeKind) -> Result<(), AppError> {
    state.chime.play(chime, &read_config(&state.config));
    Ok(())
}

#[tauri::command]
pub async fn import_chime(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    chime: ChimeKind,
) -> Result<Config, AppError> {
    // Held for the whole import: `remove_unused` below must never see a copy whose name is not stored yet.
    let _serial = state.config_update.lock().await;
    let sounds_dir = state.persistence.sounds_dir();
    let source = PathBuf::from(path.trim());
    let dir = sounds_dir.clone();
    let name = tauri::async_runtime::spawn_blocking(move || {
        chimes::import(&dir, &source, chime, wall_ms(), effects::probe_chime_file)
    })
    .await
    .map_err(|err| AppError::new("chime.save", format!("chime import failed: {err}")))??;

    let mut config = read_config(&state.config);
    *chimes::slot(&mut config.custom_chimes, chime) = Some(name.clone());
    config.chime_sound = ChimeSound::Custom;
    let stored = store_config(&app, &state, config).await;
    let custom = match &stored {
        Ok(saved) => saved.custom_chimes.clone(),
        // Not stored: the new copy is unused and goes away with the other leftovers.
        Err(_) => read_config(&state.config).custom_chimes,
    };
    chimes::remove_unused_later(sounds_dir, custom).await;
    stored
}

#[tauri::command]
pub async fn remove_custom_chime(
    app: AppHandle,
    state: State<'_, AppState>,
    chime: ChimeKind,
) -> Result<Config, AppError> {
    let _serial = state.config_update.lock().await;
    let mut config = read_config(&state.config);
    // `chimeSound` stays `custom`: the other file (or, with none left, the default sound) plays instead.
    *chimes::slot(&mut config.custom_chimes, chime) = None;
    let saved = store_config(&app, &state, config).await?;
    chimes::remove_unused_later(state.persistence.sounds_dir(), saved.custom_chimes.clone()).await;
    Ok(saved)
}

/// Wall clock in Unix milliseconds (unique imported chime names).
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[tauri::command]
pub async fn open_media_window(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    play: bool,
) -> Result<(), AppError> {
    let default_volume = read_config(&state.config).music.volume;
    state.media.open(&app, &url, play, default_volume).await
}

#[tauri::command]
pub async fn control_media_window(
    app: AppHandle,
    state: State<'_, AppState>,
    action: MediaAction,
    volume: Option<f64>,
) -> Result<(), AppError> {
    state.media.control(&app, action, volume).await
}

#[tauri::command]
pub async fn get_media_window(state: State<'_, AppState>) -> Result<MediaWindowStatus, AppError> {
    Ok(state.media.status())
}

#[tauri::command]
pub async fn get_blocklist(state: State<'_, AppState>) -> Result<BlocklistInfo, AppError> {
    Ok(state.blocklist.info())
}

#[tauri::command]
pub async fn open_blocklist_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let blocklist = state.blocklist.clone();
    let open_error = |detail: String| {
        AppError::new(
            "blocklist.open",
            format!("cannot open {BLACKLIST_FILE_NAME}: {detail}"),
        )
    };
    // Recreating a deleted file and starting the editor touch the disk: keep it off the async workers.
    tauri::async_runtime::spawn_blocking(move || {
        let path = blocklist
            .ensure_file()
            .map_err(|err| open_error(format!("{err:#}")))?;
        app.opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|err| open_error(err.to_string()))
    })
    .await
    .map_err(|err| open_error(err.to_string()))?
}

#[tauri::command]
pub async fn reload_blocklist(state: State<'_, AppState>) -> Result<BlocklistInfo, AppError> {
    let blocklist = state.blocklist.clone();
    tauri::async_runtime::spawn_blocking(move || blocklist.reload())
        .await
        .map_err(|err| {
            AppError::new(
                "blocklist.read",
                format!("cannot reload {BLACKLIST_FILE_NAME}: {err}"),
            )
        })
}
