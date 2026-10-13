//! The user's own chime sounds (`ChimeSound::Custom`): files copied into the app data `sounds` folder.
//!
//! `import_chime` copies the chosen file there under a new name (`custom-<break|focus>-<ms>.<ext>`, so a file
//! the audio thread may still be playing is never overwritten), checks that it decodes, and stores the name in
//! `Config::custom_chimes`. Files of ours that the config no longer names are deleted afterwards
//! ([`remove_unused`]), also at startup. Only names passing `focus_core::config::is_chime_file_name` are ever
//! joined to the folder, so a config entry can never point outside it.
//!
//! The plain functions are blocking file IO: callers use `spawn_blocking` (or the async helpers below).

use std::fs;
use std::path::{Path, PathBuf};

use focus_core::api::{AppError, ChimeKind, CustomChimes};
use focus_core::config::{MAX_CUSTOM_CHIME_BYTES, custom_chime_file_name, is_chime_file_name};
use tauri::{AppHandle, Manager};

use crate::{AppState, read_config};

/// Prefix of the files this module creates (only those are ever deleted by [`remove_unused`]).
const IMPORTED_PREFIX: &str = "custom-";

/// The config slot of `chime`.
pub fn slot(custom: &mut CustomChimes, chime: ChimeKind) -> &mut Option<String> {
    match chime {
        ChimeKind::BreakStart => &mut custom.break_file,
        ChimeKind::FocusStart => &mut custom.focus_file,
    }
}

/// Path of the configured file `name` in `sounds_dir`, or `None` for a name that is not a plain chime file
/// name (it is never joined then).
pub fn path_of(sounds_dir: &Path, name: &str) -> Option<PathBuf> {
    is_chime_file_name(name).then(|| sounds_dir.join(name))
}

/// Copies `source` into `sounds_dir` as the new custom sound for `chime` and returns the new file name.
/// `decodes` checks the copy (the audio crate's probe); a copy that fails it is deleted again.
/// `now_ms` makes the name unique.
pub fn import(
    sounds_dir: &Path,
    source: &Path,
    chime: ChimeKind,
    now_ms: i64,
    decodes: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<String, AppError> {
    let source_text = source.to_string_lossy();
    let extension = custom_chime_file_name(chime, &source_text)
        .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext.to_owned()))
        .ok_or_else(|| {
            chime_error(
                "chime.format",
                format!(
                    "{} is not an mp3, wav, ogg, flac or m4a file",
                    display_name(source)
                ),
            )
        })?;
    let metadata = fs::metadata(source).map_err(|err| {
        chime_error(
            "chime.read",
            format!("cannot read {}: {err}", display_name(source)),
        )
    })?;
    if !metadata.is_file() {
        return Err(chime_error(
            "chime.read",
            format!("{} is not a file", display_name(source)),
        ));
    }
    if metadata.len() > MAX_CUSTOM_CHIME_BYTES {
        return Err(chime_error(
            "chime.tooLarge",
            format!(
                "{} is larger than {} MB",
                display_name(source),
                MAX_CUSTOM_CHIME_BYTES / (1024 * 1024)
            ),
        ));
    }

    fs::create_dir_all(sounds_dir).map_err(|err| {
        chime_error(
            "chime.save",
            format!("cannot create {}: {err}", sounds_dir.display()),
        )
    })?;
    let prefix = match chime {
        ChimeKind::BreakStart => "break",
        ChimeKind::FocusStart => "focus",
    };
    let (name, target) = (0..100)
        .map(|n| {
            let suffix = if n == 0 {
                String::new()
            } else {
                format!("-{n}")
            };
            format!("{IMPORTED_PREFIX}{prefix}-{now_ms}{suffix}.{extension}")
        })
        .filter(|name| is_chime_file_name(name))
        .map(|name| {
            let target = sounds_dir.join(&name);
            (name, target)
        })
        .find(|(_, target)| !target.exists())
        .ok_or_else(|| chime_error("chime.save", "no free file name for the chime"))?;

    match fs::copy(source, &target) {
        Err(err) => {
            remove_quietly(&target);
            return Err(chime_error(
                "chime.save",
                format!("cannot copy {}: {err}", display_name(source)),
            ));
        }
        // The file grew after the size check: the copy is what counts.
        Ok(copied) if copied > MAX_CUSTOM_CHIME_BYTES => {
            remove_quietly(&target);
            return Err(chime_error(
                "chime.tooLarge",
                format!(
                    "{} is larger than {} MB",
                    display_name(source),
                    MAX_CUSTOM_CHIME_BYTES / (1024 * 1024)
                ),
            ));
        }
        Ok(_) => {}
    }
    // The copy is checked (not the source): it is what will be played, and it cannot change under us.
    if let Err(detail) = decodes(&target) {
        remove_quietly(&target);
        return Err(chime_error(
            "chime.decode",
            format!("{} cannot be played: {detail}", display_name(source)),
        ));
    }
    log::info!("custom {prefix} chime imported as {name}");
    Ok(name)
}

/// Deletes the files of ours (`custom-…`) in `sounds_dir` that `custom` does not name. Best effort.
pub fn remove_unused(sounds_dir: &Path, custom: &CustomChimes) {
    let entries = match fs::read_dir(sounds_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => {
            log::warn!("cannot list {}: {err}", sounds_dir.display());
            return;
        }
    };
    let used = |name: &str| {
        custom.break_file.as_deref() == Some(name) || custom.focus_file.as_deref() == Some(name)
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let is_ours = name.starts_with(IMPORTED_PREFIX) && is_chime_file_name(&name);
        if is_ours && !used(&name) && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            match fs::remove_file(entry.path()) {
                Ok(()) => log::info!("removed unused chime file {name}"),
                Err(err) => log::warn!("cannot remove unused chime file {name}: {err}"),
            }
        }
    }
}

/// [`remove_unused`] on the blocking pool. The caller holds `AppState::config_update`, so no import is
/// between copying a file and storing its name.
pub async fn remove_unused_later(sounds_dir: PathBuf, custom: CustomChimes) {
    let removed =
        tauri::async_runtime::spawn_blocking(move || remove_unused(&sounds_dir, &custom)).await;
    if let Err(err) = removed {
        log::warn!("cannot clean up chime files: {err}");
    }
}

/// Startup (after `AppState` is managed): deletes imported chime files the saved config no longer names
/// (left by a crash in the middle of an import, or by a config edited elsewhere). Non-blocking.
pub fn clean_up_at_startup(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let _serial = state.config_update.lock().await;
        let custom = read_config(&state.config).custom_chimes;
        remove_unused_later(state.persistence.sounds_dir(), custom).await;
    });
}

fn remove_quietly(path: &Path) {
    if let Err(err) = fs::remove_file(path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("cannot remove {}: {err}", path.display());
    }
}

/// Only the file name: the full path of the user's file is not needed in errors.
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn chime_error(code: &str, message: impl Into<String>) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        field: Some("customChimes".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("focusmode-chimes-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root");
        root
    }

    fn ok(_: &Path) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn imports_a_copy_under_a_new_name() {
        let root = temp("import");
        let source = root.join("My Bell.MP3");
        fs::write(&source, b"sound").expect("source");
        let sounds = root.join("sounds");

        let name = import(&sounds, &source, ChimeKind::BreakStart, 42, ok).expect("import");
        assert_eq!(name, "custom-break-42.mp3");
        assert_eq!(fs::read(sounds.join(&name)).expect("copy"), b"sound");
        // The same millisecond again gets another name.
        let second = import(&sounds, &source, ChimeKind::BreakStart, 42, ok).expect("import");
        assert_eq!(second, "custom-break-42-1.mp3");
        let focus = import(&sounds, &source, ChimeKind::FocusStart, 43, ok).expect("import");
        assert_eq!(focus, "custom-focus-43.mp3");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_bad_files_and_leaves_nothing_behind() {
        let root = temp("reject");
        let sounds = root.join("sounds");
        let text = root.join("notes.txt");
        fs::write(&text, b"x").expect("text");
        let err = import(&sounds, &text, ChimeKind::BreakStart, 1, ok).expect_err("txt");
        assert_eq!(err.code, "chime.format");

        let missing = root.join("missing.wav");
        let err = import(&sounds, &missing, ChimeKind::BreakStart, 1, ok).expect_err("missing");
        assert_eq!(err.code, "chime.read");

        let big = root.join("big.wav");
        let file = fs::File::create(&big).expect("big");
        file.set_len(MAX_CUSTOM_CHIME_BYTES + 1).expect("size");
        drop(file);
        let err = import(&sounds, &big, ChimeKind::BreakStart, 1, ok).expect_err("big");
        assert_eq!(err.code, "chime.tooLarge");

        let broken = root.join("broken.ogg");
        fs::write(&broken, b"not audio").expect("broken");
        let err = import(&sounds, &broken, ChimeKind::FocusStart, 1, |_| {
            Err("no decoder".into())
        })
        .expect_err("decode");
        assert_eq!(err.code, "chime.decode");
        assert!(!err.message.contains(&root.to_string_lossy().into_owned()));
        let left: Vec<_> = fs::read_dir(&sounds).expect("dir").flatten().collect();
        assert!(left.is_empty(), "the copy that does not decode is deleted");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn removes_only_our_unused_files() {
        let root = temp("unused");
        for name in [
            "custom-break-1.mp3",
            "custom-focus-2.wav",
            "custom-break-3.mp3",
            "mine.mp3",
            "custom-notes.txt",
        ] {
            fs::write(root.join(name), b"x").expect("file");
        }
        let custom = CustomChimes {
            break_file: Some("custom-break-3.mp3".into()),
            focus_file: Some("mine.mp3".into()),
        };
        remove_unused(&root, &custom);
        let mut left: Vec<String> = fs::read_dir(&root)
            .expect("dir")
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .collect();
        left.sort();
        assert_eq!(left, ["custom-break-3.mp3", "custom-notes.txt", "mine.mp3"]);
        // A missing folder is fine.
        remove_unused(&root.join("nope"), &custom);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn configured_names_never_leave_the_folder() {
        let dir = Path::new("sounds");
        assert_eq!(
            path_of(dir, "custom-break-1.mp3"),
            Some(dir.join("custom-break-1.mp3"))
        );
        assert_eq!(path_of(dir, "../evil.mp3"), None);
        assert_eq!(path_of(dir, r"C:\evil.mp3"), None);
        let mut custom = CustomChimes::default();
        *slot(&mut custom, ChimeKind::FocusStart) = Some("f.mp3".into());
        assert_eq!(custom.focus_file.as_deref(), Some("f.mp3"));
        assert_eq!(custom.break_file, None);
    }
}
