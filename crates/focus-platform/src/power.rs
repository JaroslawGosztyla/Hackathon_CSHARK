//! Keep the display and the system awake during focus.
//!
//! Windows: a power request object (`PowerCreateRequest`) with `PowerRequestDisplayRequired` and
//! `PowerRequestSystemRequired` set/cleared through `PowerSetRequest` / `PowerClearRequest`. Unlike
//! `SetThreadExecutionState`, the request is handle based (not bound to the calling thread), shows up with a
//! reason in `powercfg /requests`, and is released automatically when the handle is closed (also on crash).

use crate::PlatformError;

/// Keeps the machine awake while on. `set` is idempotent.
pub trait KeepAwake: Send {
    fn set(&mut self, on: bool) -> Result<(), PlatformError>;
}

/// Platform implementation: Windows power request, or [`NoopKeepAwake`] elsewhere.
pub fn keep_awake() -> Box<dyn KeepAwake> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::WindowsKeepAwake::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopKeepAwake)
    }
}

/// Keep-awake for unsupported platforms: turning it on fails with [`PlatformError::Unsupported`],
/// turning it off is a successful no-op.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopKeepAwake;

impl KeepAwake for NoopKeepAwake {
    fn set(&mut self, on: bool) -> Result<(), PlatformError> {
        if on {
            Err(PlatformError::Unsupported)
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Power::{
        POWER_REQUEST_TYPE, PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired,
        PowerRequestSystemRequired, PowerSetRequest,
    };
    use windows::Win32::System::Threading::{
        POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
    };
    use windows::core::PWSTR;

    use super::KeepAwake;
    use crate::PlatformError;

    /// `POWER_REQUEST_CONTEXT_VERSION` (Win32_System_SystemServices, not enabled in our `windows` features).
    const POWER_REQUEST_CONTEXT_VERSION: u32 = 0;
    /// Shown by `powercfg /requests`.
    const REASON: &str = "FocusMode: focus session in progress";

    /// Owned power request handle.
    struct PowerRequest(HANDLE);

    // SAFETY: a power request is a kernel object referenced by a handle; handles may be used and closed from any
    // thread of the process (the request is not bound to the creating thread).
    unsafe impl Send for PowerRequest {}

    impl Drop for PowerRequest {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful PowerCreateRequest and is closed exactly once.
            // Closing it releases every request still set on it.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) struct WindowsKeepAwake {
        request: Option<PowerRequest>,
        display: bool,
        system: bool,
    }

    impl WindowsKeepAwake {
        pub(super) fn new() -> Self {
            Self {
                request: None,
                display: false,
                system: false,
            }
        }

        /// Creates the power request object on first use.
        fn handle(&mut self) -> Result<HANDLE, PlatformError> {
            if let Some(request) = &self.request {
                return Ok(request.0);
            }
            let mut reason: Vec<u16> = REASON.encode_utf16().chain(std::iter::once(0)).collect();
            let context = REASON_CONTEXT {
                Version: POWER_REQUEST_CONTEXT_VERSION,
                Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
                Reason: REASON_CONTEXT_0 {
                    SimpleReasonString: PWSTR(reason.as_mut_ptr()),
                },
            };
            // SAFETY: `context` and the NUL-terminated `reason` it points to are valid for the call; the kernel
            // copies the string.
            let handle = unsafe { PowerCreateRequest(&context) }?;
            self.request = Some(PowerRequest(handle));
            Ok(handle)
        }

        fn turn_on(&mut self) -> Result<(), PlatformError> {
            let handle = self.handle()?;
            if !self.system {
                set_request(handle, PowerRequestSystemRequired)?;
                self.system = true;
            }
            if !self.display {
                set_request(handle, PowerRequestDisplayRequired)?;
                self.display = true;
            }
            Ok(())
        }

        fn turn_off(&mut self) -> Result<(), PlatformError> {
            let Some(handle) = self.request.as_ref().map(|request| request.0) else {
                self.display = false;
                self.system = false;
                return Ok(());
            };
            let mut first_error = None;
            if self.display {
                match clear_request(handle, PowerRequestDisplayRequired) {
                    Ok(()) => self.display = false,
                    Err(err) => first_error = Some(err),
                }
            }
            if self.system {
                match clear_request(handle, PowerRequestSystemRequired) {
                    Ok(()) => self.system = false,
                    Err(err) => {
                        first_error.get_or_insert(err);
                    }
                }
            }
            match first_error {
                None => Ok(()),
                Some(err) => {
                    // Last resort: closing the handle drops every request on it.
                    self.request = None;
                    self.display = false;
                    self.system = false;
                    log::warn!("clearing the power request failed ({err}); handle closed instead");
                    Ok(())
                }
            }
        }
    }

    fn set_request(handle: HANDLE, kind: POWER_REQUEST_TYPE) -> Result<(), PlatformError> {
        // SAFETY: `handle` is a live power request handle owned by `WindowsKeepAwake`.
        unsafe { PowerSetRequest(handle, kind) }
            .map_err(|err| PlatformError::Os(format!("PowerSetRequest({}) failed: {err}", kind.0)))
    }

    fn clear_request(handle: HANDLE, kind: POWER_REQUEST_TYPE) -> Result<(), PlatformError> {
        // SAFETY: `handle` is a live power request handle owned by `WindowsKeepAwake`.
        unsafe { PowerClearRequest(handle, kind) }.map_err(|err| {
            PlatformError::Os(format!("PowerClearRequest({}) failed: {err}", kind.0))
        })
    }

    impl KeepAwake for WindowsKeepAwake {
        fn set(&mut self, on: bool) -> Result<(), PlatformError> {
            if on { self.turn_on() } else { self.turn_off() }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::KeepAwake;
        use super::WindowsKeepAwake;

        #[test]
        fn toggles_idempotently() {
            let mut awake = WindowsKeepAwake::new();
            assert!(awake.set(false).is_ok(), "off before on is a no-op");
            assert!(awake.set(true).is_ok());
            assert!(awake.set(true).is_ok());
            assert!(awake.display && awake.system);
            assert!(awake.set(false).is_ok());
            assert!(awake.set(false).is_ok());
            assert!(!awake.display && !awake.system);
            assert!(awake.set(true).is_ok(), "the request object is reused");
            drop(awake);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_keep_awake() {
        let mut awake = NoopKeepAwake;
        assert!(matches!(awake.set(true), Err(PlatformError::Unsupported)));
        assert!(awake.set(false).is_ok());
    }

    #[test]
    fn platform_keep_awake_turns_off_cleanly() {
        let mut awake = keep_awake();
        assert!(awake.set(false).is_ok());
    }
}
