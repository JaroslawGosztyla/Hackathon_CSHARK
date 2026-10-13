//! FocusMode platform integrations ("shields" and tracking) behind traits with `Noop` fallbacks.
//!
//! * [`foreground`]: which app is in the foreground (process name, window title and class, idle time).
//! * [`notifications`]: Windows "Do not disturb" (Windows 11) / "Focus assist" (Windows 10).
//! * [`notification_state`]: whether the user is presenting or running a full-screen app right now (read-only).
//! * [`power`]: keep the display and the system awake during focus.
//! * [`teams`]: Microsoft Teams presence "Do not disturb" through Microsoft Graph, plus the current presence so the
//!   shell can skip it while the user appears offline or already chose Do not disturb; the Microsoft 365 calendar
//!   (`calendarView`) through the same sign-in.
//! * [`calendar`]: meetings from Google Calendar (OAuth loopback + PKCE) and ICS subscription links (RFC 5545 with
//!   recurring events), read-only.
//! * [`secrets`]: DPAPI-encrypted files for refresh tokens and calendar links, secure random bytes.
//! * [`logon`]: one-shot command at the next logon (crash-restore of muted notifications and Night light).
//! * [`processes`]: close processes by executable name (game launchers when focus starts).
//! * [`locale`]: the system UI language tag.
//! * [`clock`]: local time of day (time-of-day screen tint).
//! * [`night_light`]: Windows Night light on/off and colour temperature (warm screen, Windows 10 – 11 23H2).
//! * [`gamma`]: per-display gamma ramps (warm screen, every Windows version; Windows may cap the warmth).
//! * [`work`]: "Co dalej?" work items, read-only, from GitHub, GitLab, Jira, Azure DevOps and Linear (personal
//!   tokens stored with [`secrets`]); ranking is `focus_core::work`'s job.
//!
//! All per-OS code lives in this crate, gated with `#[cfg(target_os = "...")]`. Every integration has a
//! `Noop` implementation used on other platforms, so the shell never needs `cfg` of its own.
//! Nothing here panics; every failure is a [`PlatformError`] and the caller degrades gracefully.

mod base64;
mod bond;
pub mod calendar;
pub mod clock;
pub mod foreground;
pub mod gamma;
pub mod locale;
pub mod logon;
mod net;
pub mod night_light;
pub mod notification_state;
pub mod notifications;
pub mod power;
pub mod processes;
pub mod secrets;
mod sha256;
pub mod teams;
pub mod work;

/// Error of a platform integration. The shell maps it to `AppError` / `IntegrationState::Error`
/// (or `IntegrationState::Unavailable` for [`PlatformError::Unsupported`]).
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("unsupported on this platform")]
    Unsupported,
    #[error("os error: {0}")]
    Os(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("io error: {0}")]
    Io(String),
    /// Input from the user that cannot be used (e.g. a calendar link that is not HTTPS). English detail that never
    /// repeats a secret.
    #[error("invalid input: {0}")]
    Invalid(String),
}

impl PlatformError {
    /// `true` for [`PlatformError::Unsupported`] (the integration is unavailable, not broken).
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported)
    }
}

impl From<std::io::Error> for PlatformError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for PlatformError {
    fn from(err: windows::core::Error) -> Self {
        Self::Os(format!(
            "{} (HRESULT 0x{:08X})",
            err.message(),
            err.code().0
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::PlatformError;

    #[test]
    fn error_messages_are_stable() {
        assert_eq!(
            PlatformError::Unsupported.to_string(),
            "unsupported on this platform"
        );
        assert_eq!(PlatformError::Os("x".into()).to_string(), "os error: x");
        assert_eq!(
            PlatformError::Network("x".into()).to_string(),
            "network error: x"
        );
        assert_eq!(PlatformError::Auth("x".into()).to_string(), "auth error: x");
        assert_eq!(PlatformError::Io("x".into()).to_string(), "io error: x");
        assert_eq!(
            PlatformError::Invalid("x".into()).to_string(),
            "invalid input: x"
        );
    }

    #[test]
    fn io_error_converts() {
        let err: PlatformError = std::io::Error::other("disk full").into();
        assert!(matches!(err, PlatformError::Io(ref m) if m.contains("disk full")));
        assert!(!err.is_unsupported());
        assert!(PlatformError::Unsupported.is_unsupported());
    }
}
