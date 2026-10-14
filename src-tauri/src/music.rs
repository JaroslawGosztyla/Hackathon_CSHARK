//! Local music: the user's folder and the library bundled with the app (Open Lo-Fi, CC0). Found audio
//! files are allowed in the asset protocol scope, so the WebView can play them with `convertFileSrc(path)`.
//!
//! Only the listed audio files are allowed (one scope entry per track, at most `MAX_FILES`), never
//! the folder itself: other files in it (documents, keys, ...) stay unreadable for the WebView.

use std::path::{Path, PathBuf};

use focus_core::api::{AppError, Track};
use serde::Deserialize;
use tauri::{AppHandle, Manager};

/// Bundled library folder inside the resource directory (`tauri.conf.json` → `bundle.resources`).
const LIBRARY_DIR: [&str; 2] = ["music", "open-lofi"];
/// Track list of the bundled library (file names and titles, see `CREDITS.md`).
const LIBRARY_INDEX: &str = "library.json";
/// Sanity limits for `library.json`.
const MAX_LIBRARY_INDEX_BYTES: u64 = 256 * 1024;
const MAX_LIBRARY_TRACKS: usize = 500;
const MAX_TITLE_CHARS: usize = 120;

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

/// `library.json`: only the fields the shell needs (the rest documents the source and licence).
#[derive(Debug, Deserialize)]
struct LibraryIndex {
    #[serde(default)]
    tracks: Vec<LibraryEntry>,
}

#[derive(Debug, Deserialize)]
struct LibraryEntry {
    file: String,
    #[serde(default)]
    title: String,
}

/// Blocking: the bundled tracks in `library.json` order, allowed for playback (`list_library_tracks`).
pub fn library_tracks(app: &AppHandle) -> Result<Vec<Track>, AppError> {
    let dir = find_library_dir(app).ok_or_else(|| {
        library_error("the bundled music library was not found (music/open-lofi/library.json)")
    })?;
    let tracks = read_library(&dir)?;
    allow_tracks(app, &tracks);
    Ok(tracks)
}

/// Bundled library (`$RESOURCE/music/open-lofi`), or the repo folder in debug builds.
fn find_library_dir(app: &AppHandle) -> Option<PathBuf> {
    let in_resources = |root: PathBuf| LIBRARY_DIR.iter().fold(root, |dir, part| dir.join(part));
    let bundled = app.path().resource_dir().ok().map(in_resources);
    // Release builds must not embed (or look into) the build machine's source path.
    #[cfg(debug_assertions)]
    let dev = Some(in_resources(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"),
    ));
    #[cfg(not(debug_assertions))]
    let dev: Option<PathBuf> = None;
    [bundled, dev]
        .into_iter()
        .flatten()
        .find(|dir| dir.join(LIBRARY_INDEX).is_file())
        .map(crate::without_verbatim_prefix)
}

/// Blocking: reads `library.json` in `dir`. Entries that are not a plain audio file name in `dir`, or
/// whose file is missing, are skipped (logged); the title falls back to the file name.
fn read_library(dir: &Path) -> Result<Vec<Track>, AppError> {
    let index = dir.join(LIBRARY_INDEX);
    let size = std::fs::metadata(&index)
        .map_err(|err| library_error(format!("cannot read {}: {err}", index.display())))?
        .len();
    if size > MAX_LIBRARY_INDEX_BYTES {
        return Err(library_error(format!("{} is too large", index.display())));
    }
    let text = std::fs::read_to_string(&index)
        .map_err(|err| library_error(format!("cannot read {}: {err}", index.display())))?;
    let parsed: LibraryIndex = serde_json::from_str(&text)
        .map_err(|err| library_error(format!("invalid {}: {err}", index.display())))?;

    let mut tracks = Vec::new();
    for entry in parsed.tracks.into_iter().take(MAX_LIBRARY_TRACKS) {
        let file = entry.file.trim();
        if !is_plain_file_name(file) || !is_audio(Path::new(file)) {
            log::warn!("music library: skipping invalid entry {file:?}");
            continue;
        }
        let path = dir.join(file);
        if !path.is_file() {
            log::warn!("music library: {file} is missing");
            continue;
        }
        let title = entry.title.trim();
        let title = if title.is_empty() {
            Path::new(file)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            title.chars().take(MAX_TITLE_CHARS).collect()
        };
        tracks.push(Track {
            path: path.to_string_lossy().into_owned(),
            title,
        });
    }
    Ok(tracks)
}

/// A bare file name: no separators, no `..`, no drive or stream colon, no control characters.
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && !name.contains("..")
        && !name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
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

fn library_error(message: impl Into<String>) -> AppError {
    AppError::new("music.library", message)
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
    fn reads_the_library_index_and_skips_bad_entries() {
        let root = std::env::temp_dir().join(format!("focusmode-library-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("dir");
        fs::write(root.join("one.mp3"), b"x").expect("file");
        fs::write(root.join("two.mp3"), b"x").expect("file");
        fs::write(
            root.join(LIBRARY_INDEX),
            r#"{
                "collection": "Test", "license": "CC0-1.0",
                "tracks": [
                    { "file": "two.mp3", "title": " Second Track " },
                    { "file": "one.mp3" },
                    { "file": "../outside.mp3", "title": "Escape" },
                    { "file": "notes.txt", "title": "Text" },
                    { "file": "missing.mp3", "title": "Missing" }
                ]
            }"#,
        )
        .expect("index");

        let tracks = read_library(&root).expect("library");
        let titles: Vec<_> = tracks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["Second Track", "one"]);
        assert!(Path::new(&tracks[0].path).ends_with("two.mp3"));

        fs::write(root.join(LIBRARY_INDEX), b"{ broken").expect("index");
        assert_eq!(
            read_library(&root).expect_err("broken").code,
            "music.library"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_bundled_library_lists_existing_tracks() {
        let dir = LIBRARY_DIR.iter().fold(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"),
            |dir, part| dir.join(part),
        );
        let tracks = read_library(&dir).expect("bundled library");
        assert!(!tracks.is_empty());
        assert!(tracks.iter().all(|track| Path::new(&track.path).is_file()));
        assert!(tracks.iter().all(|track| !track.title.is_empty()));
    }

    #[test]
    fn plain_file_names_only() {
        assert!(is_plain_file_name("terminal-rain.mp3"));
        for bad in [
            "", ".", "..", "a/b.mp3", r"a\b.mp3", "C:x.mp3", "a..mp3", "x\n.mp3",
        ] {
            assert!(!is_plain_file_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn rejects_missing_folder() {
        let missing = std::env::temp_dir().join("focusmode-definitely-missing-folder");
        let err = scan_folder(&missing).expect_err("missing");
        assert_eq!(err.code, "music.folder");
    }
}
