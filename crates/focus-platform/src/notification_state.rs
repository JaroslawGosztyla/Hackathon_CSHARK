//! Whether the user is presenting or running something full screen right now, so the shell can hold back what
//! would pop up over it (e.g. the mascot) without the user turning anything off.
//!
//! * Windows: `SHQueryUserNotificationState` (shell32, the same check Windows uses before showing balloon
//!   notifications). A full-screen app or game, exclusive-mode Direct3D, Windows presentation settings and a
//!   Windows Store app in the foreground count as presenting. Accepting notifications, the first-logon quiet time,
//!   and the user being away (locked, screen saver, another fast-user-switching session) do not. Any error counts
//!   as not presenting.
//! * Elsewhere: never presenting (`Noop`).
//!
//! The call is cheap and needs no COM, so the shell may poll it every tick.

/// `true` while the user is presenting, playing a full-screen game or otherwise running a full-screen app;
/// `false` when notifications are welcome, the user is away, the state is unknown or the platform has no such
/// state.
pub fn user_is_presenting() -> bool {
    #[cfg(target_os = "windows")]
    {
        match windows_impl::query_state() {
            Ok(state) => is_presenting_state(state),
            Err(err) => {
                log::debug!("notification state: query failed, assuming not presenting: {err}");
                false
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// `QUERY_USER_NOTIFICATION_STATE` values from `shellapi.h` (unchanged since Windows Vista; `APP` since Windows 8).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
mod quns {
    /// Screen saver, locked workstation or an inactive fast-user-switching session.
    pub const NOT_PRESENT: i32 = 1;
    /// A full-screen application is running or presentation settings are applied.
    pub const BUSY: i32 = 2;
    /// A full-screen, exclusive-mode Direct3D application is running.
    pub const RUNNING_D3D_FULL_SCREEN: i32 = 3;
    /// The user turned on Windows presentation settings.
    pub const PRESENTATION_MODE: i32 = 4;
    /// None of the other states: notifications are welcome.
    pub const ACCEPTS_NOTIFICATIONS: i32 = 5;
    /// The first hour after a new user's first logon (Windows 8+).
    pub const QUIET_TIME: i32 = 6;
    /// A Windows Store app is running in the foreground (Windows 8+).
    pub const APP: i32 = 7;
}

/// Maps a raw `QUERY_USER_NOTIFICATION_STATE` to "presenting"; values this code does not know count as not
/// presenting (nothing is held back on a guess).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_presenting_state(state: i32) -> bool {
    match state {
        quns::BUSY | quns::RUNNING_D3D_FULL_SCREEN | quns::PRESENTATION_MODE | quns::APP => true,
        quns::NOT_PRESENT | quns::ACCEPTS_NOTIFICATIONS | quns::QUIET_TIME => false,
        _ => false,
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows::core::HRESULT;

    use crate::PlatformError;

    // TODO(hackathon): declared by hand because the `windows` dependency does not enable the `Win32_UI_Shell`
    // feature; with it, this is `windows::Win32::UI::Shell::SHQueryUserNotificationState`. Same raw-dylib import
    // the `windows` crate generates for it.
    windows::core::link!("shell32.dll" "system" fn SHQueryUserNotificationState(pquns: *mut i32) -> HRESULT);

    /// The raw `QUERY_USER_NOTIFICATION_STATE` of the current session.
    pub(super) fn query_state() -> Result<i32, PlatformError> {
        let mut state = 0i32;
        // SAFETY: `state` is a valid, writable out pointer for the duration of the call; the function has no other
        // preconditions (no COM, any thread).
        unsafe { SHQueryUserNotificationState(&mut state) }.ok()?;
        Ok(state)
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn query_reports_a_known_state() {
            // A test runner without an interactive desktop may fail the query; that only means "not presenting".
            match super::query_state() {
                Ok(state) => assert!((1..=7).contains(&state), "unexpected state {state}"),
                Err(err) => eprintln!("SHQueryUserNotificationState failed: {err}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_screen_and_presentation_states_count_as_presenting() {
        assert!(is_presenting_state(quns::BUSY));
        assert!(is_presenting_state(quns::RUNNING_D3D_FULL_SCREEN));
        assert!(is_presenting_state(quns::PRESENTATION_MODE));
        assert!(is_presenting_state(quns::APP));
    }

    #[test]
    fn welcome_away_and_quiet_states_do_not() {
        assert!(!is_presenting_state(quns::ACCEPTS_NOTIFICATIONS));
        assert!(!is_presenting_state(quns::QUIET_TIME));
        assert!(!is_presenting_state(quns::NOT_PRESENT));
    }

    #[test]
    fn unknown_states_do_not() {
        for state in [0, 8, 42, -1, i32::MIN, i32::MAX] {
            assert!(!is_presenting_state(state), "{state}");
        }
    }

    #[test]
    fn platform_query_never_panics() {
        let presenting = user_is_presenting();
        #[cfg(not(target_os = "windows"))]
        assert!(!presenting);
        #[cfg(target_os = "windows")]
        let _ = presenting;
    }
}
