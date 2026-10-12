//! One-shot command at the next logon of the current user.
//!
//! This is the safety net for Windows notifications muted during focus: if FocusMode dies while they are muted
//! (crash, power loss, killed at logoff), the next logon starts `FocusMode.exe --restore-shields`, which restores
//! them from the persisted [`MuteToken`](crate::notifications::MuteToken) and exits.
//!
//! Windows: the per-user `RunOnce` key (`HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce`), value
//! [`RUN_ONCE_VALUE`]. Windows deletes the value before it runs the command, so it runs at most once. Per user,
//! **no admin rights**, no scheduled task, no service. The shell writes the value before muting and deletes it
//! after restoring, so it only exists while a restore is pending.

use std::path::Path;

use crate::PlatformError;

/// Name of the `RunOnce` value.
pub const RUN_ONCE_VALUE: &str = "FocusModeRestore";

/// Runs a command once at the next logon of the current user.
pub trait LogonTask: Send {
    /// Registers `"<exe>" <args>`, replacing an earlier registration.
    fn schedule(&mut self, exe: &Path, args: &str) -> Result<(), PlatformError>;
    /// Removes the registration. Succeeds when nothing is registered.
    fn cancel(&mut self) -> Result<(), PlatformError>;
}

/// Platform implementation: the `RunOnce` value on Windows, or [`NoopLogonTask`] elsewhere.
pub fn restore_at_logon() -> Box<dyn LogonTask> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::RunOnceTask)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopLogonTask)
    }
}

/// Logon task for unsupported platforms: `schedule` fails with [`PlatformError::Unsupported`],
/// `cancel` is a successful no-op (nothing can be registered).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopLogonTask;

impl LogonTask for NoopLogonTask {
    fn schedule(&mut self, _exe: &Path, _args: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn cancel(&mut self) -> Result<(), PlatformError> {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW,
    };
    use windows::core::{PCWSTR, w};

    use super::{LogonTask, RUN_ONCE_VALUE};
    use crate::PlatformError;

    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce`, value [`RUN_ONCE_VALUE`].
    pub(super) struct RunOnceTask;

    impl LogonTask for RunOnceTask {
        fn schedule(&mut self, exe: &Path, args: &str) -> Result<(), PlatformError> {
            let name = wide(RUN_ONCE_VALUE);
            let data = command_line(exe, args);
            let bytes = u32::try_from(data.len() * size_of::<u16>())
                .map_err(|_| PlatformError::Os("logon command line is too long".into()))?;
            // SAFETY: `name` and `data` are NUL-terminated UTF-16 buffers that outlive the call; `bytes` is the
            // size of `data` (including the terminator), as REG_SZ requires. RegSetKeyValueW creates the
            // per-user subkey if it is missing; HKCU needs no elevation.
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    w!("Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce"),
                    PCWSTR(name.as_ptr()),
                    REG_SZ.0,
                    Some(data.as_ptr().cast::<c_void>()),
                    bytes,
                )
            }
            .ok()?;
            Ok(())
        }

        fn cancel(&mut self) -> Result<(), PlatformError> {
            let name = wide(RUN_ONCE_VALUE);
            // SAFETY: `name` is a NUL-terminated UTF-16 buffer that outlives the call.
            let status = unsafe {
                RegDeleteKeyValueW(
                    HKEY_CURRENT_USER,
                    w!("Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce"),
                    PCWSTR(name.as_ptr()),
                )
            };
            if status == ERROR_FILE_NOT_FOUND {
                // Nothing registered (or Windows already consumed it at logon).
                return Ok(());
            }
            status.ok()?;
            Ok(())
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// `"<exe>" <args>` as NUL-terminated UTF-16. The path is quoted (it may contain spaces); Windows paths cannot
    /// contain `"`, so no escaping is needed.
    pub(super) fn command_line(exe: &Path, args: &str) -> Vec<u16> {
        let quote = u16::from(b'"');
        let mut line = vec![quote];
        line.extend(exe.as_os_str().encode_wide());
        line.push(quote);
        if !args.is_empty() {
            line.push(u16::from(b' '));
            line.extend(args.encode_utf16());
        }
        line.push(0);
        line
    }

    #[cfg(test)]
    mod tests {
        use super::super::LogonTask;
        use super::{RunOnceTask, command_line};
        use std::path::Path;

        #[test]
        fn command_line_quotes_the_exe() {
            let line = command_line(
                Path::new(r"C:\Users\Jan Kowalski\AppData\Local\FocusMode\FocusMode.exe"),
                "--restore-shields",
            );
            assert_eq!(line.last(), Some(&0));
            let text = String::from_utf16(&line[..line.len() - 1]).expect("utf-16");
            assert_eq!(
                text,
                r#""C:\Users\Jan Kowalski\AppData\Local\FocusMode\FocusMode.exe" --restore-shields"#
            );
        }

        /// Writes and deletes the real per-user `RunOnce` value — run manually:
        /// `cargo test -p focus-platform -- --ignored run_once_round_trip`
        #[test]
        #[ignore = "writes the user's real RunOnce registry key"]
        fn run_once_round_trip() {
            let mut task = RunOnceTask;
            task.schedule(Path::new(r"C:\Windows\System32\cmd.exe"), "/c exit")
                .expect("schedule");
            task.cancel().expect("cancel");
            // Cancelling again (nothing registered) still succeeds.
            task.cancel().expect("cancel twice");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_logon_task() {
        let mut task = NoopLogonTask;
        assert!(matches!(
            task.schedule(Path::new("focusmode"), "--restore-shields"),
            Err(PlatformError::Unsupported)
        ));
        assert!(task.cancel().is_ok());
    }

    #[test]
    fn platform_logon_task_is_constructible() {
        let _task = restore_at_logon();
    }
}
