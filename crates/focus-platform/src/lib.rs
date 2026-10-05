//! FocusMode platform integrations behind traits with `Noop` fallbacks (stretch goals):
//! Teams "Do not disturb" via Microsoft Graph and experimental gamma ramp tint.
//!
//! All per-OS code lives here, gated with `#[cfg(target_os = "...")]`.
