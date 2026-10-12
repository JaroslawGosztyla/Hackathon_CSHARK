//! Thin `#[tauri::command]` adapters (names = `focus_core::api::commands`).
//!
//! All commands are `async` so they run on the async runtime, never on the main thread
//! (several of them wait for a worker reply; the main thread must stay free for the event loop).

use std::path::PathBuf;

use focus_core::api::{
    AppError, BRIDGE_PORT, BridgeInfo, ChimeKind, Config, DeviceCodePrompt, MediaAction,
    MediaWindowStatus, Phase, SessionSnapshot, SessionSummary, ShieldStatus, TeamsStatus, Track,
    UsageReport,
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::session_actor::ActorCmd;
use crate::{AppState, emit_error, lock, music, read_config, write_config};

/// Upper bound for `list_history(limit)`.
const MAX_HISTORY: u32 = 500;

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
            &app,
            AppError::new("config.save", format!("cannot save config: {err:#}")),
        ),
        Err(err) => emit_error(
            &app,
            AppError::new("config.save", format!("cannot save config: {err}")),
        ),
    }

    write_config(&state.config, normalized.clone());
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
        music::allow_configured_folder(&app, normalized.music.folder.as_deref());
    }
    if normalized.launch_on_login != previous.launch_on_login {
        crate::sync_autostart(&app, normalized.launch_on_login);
    }
    Ok(normalized)
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
pub async fn get_shield_status(state: State<'_, AppState>) -> Result<ShieldStatus, AppError> {
    Ok(state.shields.status())
}

#[tauri::command]
pub async fn get_current_usage(state: State<'_, AppState>) -> Result<UsageReport, AppError> {
    let snapshot = state.actor.snapshot().await?;
    if snapshot.phase == Phase::Idle {
        return Ok(UsageReport::default());
    }
    Ok(state.tracker.report())
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
    let volume = read_config(&state.config).chime_volume;
    state.chime.play(chime, volume);
    Ok(())
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
