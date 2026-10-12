//! FocusMode platform integrations ("shields" and tracking) behind traits with `Noop` fallbacks.
//!
//! * [`foreground`]: which app is in the foreground (process name, window title, idle time).
//! * [`notifications`]: Windows "Do not disturb" (Windows 11) / "Focus assist" (Windows 10).
//! * [`power`]: keep the display and the system awake during focus.
//! * [`teams`]: Microsoft Teams presence "Do not disturb" through Microsoft Graph.
//! * [`logon`]: one-shot command at the next logon (crash-restore of muted notifications).
//!
//! All per-OS code lives in this crate, gated with `#[cfg(target_os = "...")]`. Every integration has a
//! `Noop` implementation used on other platforms, so the shell never needs `cfg` of its own.
//! Nothing here panics; every failure is a [`PlatformError`] and the caller degrades gracefully.

pub mod foreground;
pub mod logon;
pub mod notifications;
pub mod power;
pub mod teams;

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
    }

    #[test]
    fn io_error_converts() {
        let err: PlatformError = std::io::Error::other("disk full").into();
        assert!(matches!(err, PlatformError::Io(ref m) if m.contains("disk full")));
        assert!(!err.is_unsupported());
        assert!(PlatformError::Unsupported.is_unsupported());
    }
}
