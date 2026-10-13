//! Closing processes by executable file name (game launchers when focus starts).
//!
//! The shell passes the executable names of the launchers the user ticked (see `focus_core::launchers`), and
//! [`ProcessCloser::close`] terminates every matching process of the **current user session**.
//!
//! Windows implementation:
//! * `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` + `Process32FirstW` / `Process32NextW` list the processes,
//! * a process matches when its executable file name equals one of the names (ASCII case-insensitive, exact:
//!   `steam.exe` never matches `steamwebhelper.exe`),
//! * our own process is never touched, and neither is any process of another session (`ProcessIdToSessionId`),
//!   so other users signed in on the same machine keep their launchers,
//! * `OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION)`, then the image name is checked again
//!   through the handle (the pid may have been reused since the snapshot) and `TerminateProcess(handle, 0)`;
//!   the handle is always closed,
//! * processes are terminated in the order of the names given: pass the main launcher process before its
//!   helpers (e.g. `EADesktop.exe` before `EACefSubProcess.exe`) so a helper is not restarted by its parent,
//! * a process we may not terminate (elevated, protected) or that already exited is skipped with a debug log;
//!   one process never fails the whole call. Only a failed snapshot is an error.
//!
//! No admin rights are needed for processes that run as the user. Services (e.g. `steamservice.exe`) are not
//! closed: they run in session 0 and need elevation. Nothing is closed gently first: launchers minimise to the
//! tray on `WM_CLOSE`, so a window message would not end them anyway.

use crate::PlatformError;

/// A process that [`ProcessCloser::close`] terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedProcess {
    /// Executable file name as reported by the OS, e.g. `steam.exe`.
    pub name: String,
    /// Process id at the time it was terminated.
    pub pid: u32,
}

/// Terminates processes by executable file name. Call from a worker thread (it takes a process snapshot).
pub trait ProcessCloser: Send {
    /// Terminates every process of the current session whose executable file name equals one of `exe_names`
    /// (ASCII case-insensitive, exact match; blank names are ignored), in the order of `exe_names`.
    /// Returns the processes it terminated (empty when nothing was running). Processes that cannot be
    /// terminated are skipped; `Err` only when the process list cannot be read.
    fn close(&mut self, exe_names: &[&str]) -> Result<Vec<ClosedProcess>, PlatformError>;
}

/// Platform implementation: Windows process termination, or [`NoopProcessCloser`] elsewhere.
pub fn process_closer() -> Box<dyn ProcessCloser> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::WindowsProcessCloser)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopProcessCloser)
    }
}

/// Process closer for unsupported platforms: closing anything fails with [`PlatformError::Unsupported`];
/// an empty (or blank-only) list is a successful no-op.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProcessCloser;

impl ProcessCloser for NoopProcessCloser {
    fn close(&mut self, exe_names: &[&str]) -> Result<Vec<ClosedProcess>, PlatformError> {
        if has_names(exe_names) {
            Err(PlatformError::Unsupported)
        } else {
            Ok(Vec::new())
        }
    }
}

/// At least one non-blank name.
fn has_names(exe_names: &[&str]) -> bool {
    exe_names.iter().any(|name| !name.trim().is_empty())
}

/// Index of the first entry of `exe_names` that names `exe` (ASCII case-insensitive, exact; surrounding
/// whitespace of the entries is ignored and blank entries never match).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn match_index(exe: &str, exe_names: &[&str]) -> Option<usize> {
    let exe = exe.trim();
    if exe.is_empty() {
        return None;
    }
    exe_names.iter().position(|name| {
        let name = name.trim();
        !name.is_empty() && name.eq_ignore_ascii_case(exe)
    })
}

/// Picks the processes to terminate from a process list (`(pid, exe name)`): matching names only, never pid 0
/// or `own_pid`, ordered by the position of the matched name in `exe_names` (stable within one name).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn select_targets(
    processes: impl IntoIterator<Item = (u32, String)>,
    exe_names: &[&str],
    own_pid: u32,
) -> Vec<ClosedProcess> {
    let mut targets: Vec<(usize, ClosedProcess)> = processes
        .into_iter()
        .filter(|(pid, _)| *pid != 0 && *pid != own_pid)
        .filter_map(|(pid, name)| {
            match_index(&name, exe_names).map(|rank| (rank, ClosedProcess { name, pid }))
        })
        .collect();
    targets.sort_by_key(|(rank, _)| *rank);
    targets.into_iter().map(|(_, target)| target).collect()
}

/// Text before the first NUL of a fixed-size UTF-16 buffer (e.g. `PROCESSENTRY32W::szExeFile`).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn utf16_until_nul(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

/// File name part of a full image path (`C:\Games\Steam\steam.exe` → `steam.exe`).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn file_name_of(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path).trim()
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, ERROR_NO_MORE_FILES, HANDLE,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
        QueryFullProcessImageNameW, TerminateProcess,
    };
    use windows::core::PWSTR;

    use super::{
        ClosedProcess, ProcessCloser, file_name_of, has_names, select_targets, utf16_until_nul,
    };
    use crate::PlatformError;

    /// Exit code of terminated processes.
    const EXIT_CODE: u32 = 0;
    /// Image path buffer (UTF-16 units): long-path maximum.
    const IMAGE_PATH_CHARS: usize = 32_768;
    /// Safety bound for the snapshot walk (far above any real process count).
    const MAX_PROCESSES: usize = 65_536;

    pub(super) struct WindowsProcessCloser;

    impl ProcessCloser for WindowsProcessCloser {
        fn close(&mut self, exe_names: &[&str]) -> Result<Vec<ClosedProcess>, PlatformError> {
            if !has_names(exe_names) {
                return Ok(Vec::new());
            }
            let own_pid = std::process::id();
            let own_session = session_of(own_pid).ok_or_else(|| {
                PlatformError::Os("ProcessIdToSessionId failed for the current process".into())
            })?;

            let mut closed = Vec::new();
            for target in select_targets(list_processes()?, exe_names, own_pid) {
                if session_of(target.pid) != Some(own_session) {
                    log::debug!(
                        "processes: {} (pid {}) is not in our session, skipped",
                        target.name,
                        target.pid
                    );
                    continue;
                }
                match terminate(&target) {
                    Ok(true) => {
                        log::info!("processes: closed {} (pid {})", target.name, target.pid);
                        closed.push(target);
                    }
                    Ok(false) => log::debug!(
                        "processes: pid {} is no longer {}, skipped",
                        target.pid,
                        target.name
                    ),
                    Err(err) if is_benign(&err) => log::debug!(
                        "processes: {} (pid {}) not closed (exited or access denied): {err}",
                        target.name,
                        target.pid
                    ),
                    Err(err) => log::warn!(
                        "processes: closing {} (pid {}) failed: {err}",
                        target.name,
                        target.pid
                    ),
                }
            }
            Ok(closed)
        }
    }

    /// Closes a kernel handle on drop.
    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful Win32 call and is closed exactly once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    /// Remote Desktop session of a process; `None` when it cannot be read (e.g. the process exited).
    fn session_of(pid: u32) -> Option<u32> {
        let mut session = 0u32;
        // SAFETY: `session` is a valid out pointer for the duration of the call.
        unsafe { ProcessIdToSessionId(pid, &mut session) }
            .ok()
            .map(|()| session)
    }

    /// `(pid, exe file name)` of every running process.
    fn list_processes() -> Result<Vec<(u32, String)>, PlatformError> {
        // SAFETY: plain FFI call; an invalid handle is reported as Err.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|err| PlatformError::Os(format!("CreateToolhelp32Snapshot failed: {err}")))?;
        let snapshot = OwnedHandle(snapshot);

        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(u32::MAX),
            ..Default::default()
        };
        let mut processes = Vec::new();
        // SAFETY: `snapshot` is a live Toolhelp snapshot; `entry` is a valid PROCESSENTRY32W with dwSize set.
        let mut step = unsafe { Process32FirstW(snapshot.0, &mut entry) };
        while processes.len() < MAX_PROCESSES {
            match step {
                Ok(()) => processes.push((entry.th32ProcessID, utf16_until_nul(&entry.szExeFile))),
                Err(err) if err.code() == ERROR_NO_MORE_FILES.to_hresult() => break,
                Err(err) if processes.is_empty() => {
                    return Err(PlatformError::Os(format!(
                        "reading the process list failed: {err}"
                    )));
                }
                Err(err) => {
                    log::warn!("processes: process list cut short: {err}");
                    break;
                }
            }
            // SAFETY: as above; dwSize is preserved between calls.
            step = unsafe { Process32NextW(snapshot.0, &mut entry) };
        }
        Ok(processes)
    }

    /// Terminates `target` if its pid still runs the same executable. `Ok(false)`: the pid now belongs to
    /// another program (reused since the snapshot), nothing was done.
    fn terminate(target: &ClosedProcess) -> windows::core::Result<bool> {
        // SAFETY: plain FFI call; failure (access denied, exited process) is reported as Err.
        let handle = unsafe {
            OpenProcess(
                PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                target.pid,
            )
        }?;
        let handle = OwnedHandle(handle);
        if !image_name(&handle).is_some_and(|name| name.eq_ignore_ascii_case(&target.name)) {
            return Ok(false);
        }
        // SAFETY: `handle` is a live process handle opened with PROCESS_TERMINATE.
        unsafe { TerminateProcess(handle.0, EXIT_CODE) }?;
        Ok(true)
    }

    /// Executable file name of an open process.
    fn image_name(handle: &OwnedHandle) -> Option<String> {
        let mut buffer = vec![0u16; IMAGE_PATH_CHARS];
        let mut size = u32::try_from(buffer.len()).unwrap_or(u32::MAX);
        // SAFETY: `buffer` holds `size` UTF-16 units and outlives the call; `size` is a valid in/out pointer.
        unsafe {
            QueryFullProcessImageNameW(
                handle.0,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
        }
        .ok()?;
        let len = usize::try_from(size).unwrap_or(0).min(buffer.len());
        let path = String::from_utf16_lossy(&buffer[..len]);
        let name = file_name_of(&path);
        (!name.is_empty()).then(|| name.to_owned())
    }

    /// Errors that only mean "this process cannot / need not be closed": access denied (elevated, protected,
    /// or already exiting) and invalid parameter (the pid no longer exists).
    fn is_benign(err: &windows::core::Error) -> bool {
        err.code() == ERROR_ACCESS_DENIED.to_hresult()
            || err.code() == ERROR_INVALID_PARAMETER.to_hresult()
    }

    #[cfg(test)]
    mod tests {
        use std::path::PathBuf;
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        use super::super::ProcessCloser;
        use super::{WindowsProcessCloser, list_processes, session_of};

        /// Set in the environment of the child started by `closes_a_running_process`.
        const SLEEPER_ENV: &str = "FOCUSMODE_PROCESS_CLOSER_SLEEPER";

        #[test]
        fn lists_processes_including_ourselves() {
            let processes = list_processes().expect("process list");
            let own = std::process::id();
            assert!(
                processes
                    .iter()
                    .any(|(pid, name)| *pid == own && !name.is_empty())
            );
            assert!(session_of(own).is_some());
        }

        #[test]
        fn unknown_or_blank_names_close_nothing() {
            let mut closer = WindowsProcessCloser;
            assert_eq!(closer.close(&[]).expect("empty"), Vec::new());
            assert_eq!(closer.close(&["", "  "]).expect("blank"), Vec::new());
            assert_eq!(
                closer
                    .close(&["focusmode-no-such-process-7f3a9c.exe"])
                    .expect("unknown"),
                Vec::new()
            );
        }

        /// Child side of `closes_a_running_process`: sleeps until it is terminated (bounded).
        #[test]
        #[ignore = "helper process for closes_a_running_process"]
        fn sleeper_child() {
            if std::env::var_os(SLEEPER_ENV).is_some() {
                std::thread::sleep(Duration::from_secs(60));
            }
        }

        fn unique_exe_path() -> PathBuf {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            std::env::temp_dir().join(format!(
                "focusmode-closer-test-{}-{nanos}.exe",
                std::process::id()
            ))
        }

        #[test]
        fn closes_a_running_process() {
            // A copy of this test binary under a unique name, so nothing else can match it.
            let exe = unique_exe_path();
            std::fs::copy(std::env::current_exe().expect("current exe"), &exe).expect("copy");
            let name = exe
                .file_name()
                .and_then(|n| n.to_str())
                .expect("file name")
                .to_ascii_uppercase();
            let mut child = Command::new(&exe)
                .args([
                    "--ignored",
                    "--exact",
                    "processes::windows_impl::tests::sleeper_child",
                ])
                .env(SLEEPER_ENV, "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn");

            let mut closer = WindowsProcessCloser;
            let closed = closer
                .close(&["focusmode-no-such-process-7f3a9c.exe", &name])
                .expect("close");
            let found = closed.iter().any(|p| p.pid == child.id());
            if !found {
                let _ = child.kill();
            }
            let started = Instant::now();
            let status = child.wait().expect("wait");
            let _ = std::fs::remove_file(&exe);

            assert!(found, "child not closed: {closed:?}");
            assert_eq!(closed.len(), 1);
            assert!(closed[0].name.eq_ignore_ascii_case(&name));
            assert_eq!(status.code(), Some(0));
            assert!(started.elapsed() < Duration::from_secs(30));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(list: &[(u32, &str)]) -> Vec<(u32, String)> {
        list.iter()
            .map(|(pid, name)| (*pid, (*name).to_owned()))
            .collect()
    }

    #[test]
    fn names_match_exactly_ignoring_ascii_case() {
        let names = ["steam.exe", "steamwebhelper.exe", "Battle.net.exe"];
        assert_eq!(match_index("steam.exe", &names), Some(0));
        assert_eq!(match_index("STEAM.EXE", &names), Some(0));
        assert_eq!(match_index("SteamWebHelper.exe", &names), Some(1));
        assert_eq!(match_index("battle.NET.exe", &names), Some(2));
        assert_eq!(match_index("steam", &names), None, "no extension");
        assert_eq!(match_index("steam.exe.bak", &names), None);
        assert_eq!(match_index("mysteam.exe", &names), None, "no substring");
        assert_eq!(match_index("", &names), None);
        assert_eq!(match_index("steam.exe", &[]), None);
        assert_eq!(
            match_index("x.exe", &["", "  "]),
            None,
            "blank never matches"
        );
        assert_eq!(
            match_index("Amazon Games UI.exe", &[" amazon games ui.exe "]),
            Some(0)
        );
    }

    #[test]
    fn targets_are_filtered_and_ordered_by_name() {
        let processes = entries(&[
            (0, "steam.exe"),
            (10, "EACefSubProcess.exe"),
            (11, "explorer.exe"),
            (12, "EADesktop.exe"),
            (13, "eacefsubprocess.exe"),
            (99, "EADesktop.exe"),
        ]);
        let targets = select_targets(
            processes,
            &["EADesktop.exe", "EACefSubProcess.exe", "steam.exe"],
            99,
        );
        assert_eq!(
            targets,
            vec![
                ClosedProcess {
                    name: "EADesktop.exe".into(),
                    pid: 12
                },
                ClosedProcess {
                    name: "EACefSubProcess.exe".into(),
                    pid: 10
                },
                ClosedProcess {
                    name: "eacefsubprocess.exe".into(),
                    pid: 13
                },
            ],
            "own pid 99 and pid 0 skipped, main process first"
        );
        assert!(select_targets(entries(&[(5, "steam.exe")]), &[], 1).is_empty());
    }

    #[test]
    fn fixed_buffers_and_paths_are_decoded() {
        let mut buffer = [0u16; 16];
        for (slot, unit) in buffer.iter_mut().zip("steam.exe".encode_utf16()) {
            *slot = unit;
        }
        assert_eq!(utf16_until_nul(&buffer), "steam.exe");
        let full: Vec<u16> = "abc".encode_utf16().collect();
        assert_eq!(utf16_until_nul(&full), "abc", "no NUL: whole buffer");
        assert_eq!(utf16_until_nul(&[]), "");

        assert_eq!(
            file_name_of(r"C:\Program Files (x86)\Steam\steam.exe"),
            "steam.exe"
        );
        assert_eq!(file_name_of("itch.exe"), "itch.exe");
        assert_eq!(file_name_of(r"C:\dir\"), "");
    }

    #[test]
    fn noop_closer() {
        let mut closer = NoopProcessCloser;
        assert!(matches!(
            closer.close(&["steam.exe"]),
            Err(PlatformError::Unsupported)
        ));
        assert_eq!(closer.close(&[]).ok(), Some(Vec::new()));
        assert_eq!(closer.close(&[" "]).ok(), Some(Vec::new()));
    }

    #[test]
    fn platform_closer_ignores_unknown_names() {
        let mut closer = process_closer();
        let result = closer.close(&["focusmode-no-such-process-2b81d4.exe"]);
        #[cfg(target_os = "windows")]
        assert_eq!(result.ok(), Some(Vec::new()));
        #[cfg(not(target_os = "windows"))]
        assert!(matches!(result, Err(PlatformError::Unsupported)));
    }
}
