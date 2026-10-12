//! Local music folder: scanning for audio files and allowing them in the asset protocol scope,
//! so the WebView can play them with `convertFileSrc(path)`.
//!
//! Only the scanned audio files are allowed (one scope entry per track, at most `MAX_FILES`), never
//! the folder itself: other files in it (documents, keys, ...) stay unreadable for the WebView.

use std::path::{Path, PathBuf};

use focus_core::api::{AppError, Track};
use tauri::{AppHandle, Manager};

const EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "aac", "flac", "wav", "ogg", "oga", "opus", "webm",
];
/// Subdirectory levels below the chosen folder that are still scanned.
const MAX_DEPTH: usize = 3;
const MAX_FILES: usize = 2000;

/// Blocking: walks `dir` (max depth 3, max 2000 tracks), sorted by path.
pub fn scan_folder(dir: &Path) -> Result<Vec<Track>, AppError> {
    if !dir.is_absolute() {
        return Err(folder_error(format!(
            "music folder must be an absolute path: {}",
            dir.display()
        )));
    }
    if !dir.is_dir() {
        return Err(folder_error(format!(
            "music folder does not exist: {}",
            dir.display()
        )));
    }

    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(dir.to_path_buf(), 0)];
    'walk: while let Some((current, depth)) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(err) if depth == 0 => {
                return Err(folder_error(format!(
                    "cannot read music folder {}: {err}",
                    current.display()
                )));
            }
            Err(err) => {
                log::warn!("skipping unreadable folder {}: {err}", current.display());
                continue;
            }
        };
        for entry in entries.flatten() {
            // `file_type` does not follow symlinks, so symlinked folders cannot loop the walk.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                if depth < MAX_DEPTH {
                    stack.push((path, depth + 1));
                }
            } else if is_audio(&path) {
                files.push(path);
                if files.len() >= MAX_FILES {
                    log::warn!(
                        "music folder has more than {MAX_FILES} tracks; the rest is ignored"
                    );
                    break 'walk;
                }
            }
        }
    }

    let mut tracks: Vec<Track> = files
        .into_iter()
        .map(|path| Track {
            title: path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
        })
        .collect();
    tracks.sort_by_cached_key(|track| track.path.to_lowercase());
    Ok(tracks)
}

/// Blocking: lets the WebView load exactly these files through the asset protocol. Tauri also adds
/// the canonical spelling of each path (junctions, `\\?\` prefixes), which the scope check compares.
pub fn allow_tracks(app: &AppHandle, tracks: &[Track]) {
    let scope = app.asset_protocol_scope();
    let mut failed = 0usize;
    for track in tracks {
        if let Err(err) = scope.allow_file(Path::new(&track.path)) {
            failed += 1;
            log::debug!("cannot allow music file {}: {err}", track.path);
        }
    }
    if failed > 0 {
        log::warn!(
            "{failed} of {} music files could not be allowed for playback",
            tracks.len()
        );
    }
}

/// Blocking: scans `dir` and allows the found tracks (`list_folder_tracks`).
pub fn scan_and_allow(app: &AppHandle, dir: &Path) -> Result<Vec<Track>, AppError> {
    let tracks = scan_folder(dir)?;
    allow_tracks(app, &tracks);
    Ok(tracks)
}

/// Startup / config change: scans the configured folder on a background thread and allows its
/// tracks, so a saved folder plays before the UI lists it again. Never blocks the caller.
pub fn allow_configured_folder(app: &AppHandle, folder: Option<&str>) {
    let Some(folder) = folder.map(str::trim).filter(|folder| !folder.is_empty()) else {
        return;
    };
    let dir = PathBuf::from(folder);
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("focus-music-scan".into())
        .spawn(move || match scan_and_allow(&app, &dir) {
            Ok(tracks) => log::info!("music folder ready: {} tracks", tracks.len()),
            Err(err) => log::warn!("{}", err.message),
        });
    if let Err(err) = spawned {
        log::warn!("cannot scan the music folder: {err}");
    }
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
        })
}

fn folder_error(message: String) -> AppError {
    AppError {
        code: "music.folder".into(),
        message,
        field: Some("music.folder".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_audio_files_recursively() {
        let root = std::env::temp_dir().join(format!("focusmode-music-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a").join("b")).expect("dirs");
        fs::write(root.join("Zeta.MP3"), b"x").expect("file");
        fs::write(root.join("notes.txt"), b"x").expect("file");
        fs::write(root.join("a").join("alpha.ogg"), b"x").expect("file");
        fs::write(root.join("a").join("b").join("beta.flac"), b"x").expect("file");

        let tracks = scan_folder(&root).expect("scan");
        let titles: Vec<_> = tracks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["alpha", "beta", "Zeta"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_missing_folder() {
        let missing = std::env::temp_dir().join("focusmode-definitely-missing-folder");
        let err = scan_folder(&missing).expect_err("missing");
        assert_eq!(err.code, "music.folder");
    }
}
