//! Windows "Do not disturb" (Windows 11) / "Focus assist" (Windows 10) during focus.
//!
//! # Chosen approach (researched 2026-10)
//!
//! Windows has **no documented API** to switch Do not disturb. We use the same per-user, out-of-process COM
//! class that the Windows notification centre / quick settings use, reverse engineered by Rafael Rivera
//! (Windows 10 1903, <https://withinrafael.com/2019/09/19/determine-if-your-app-is-in-a-focus-assist-profiles-priority-list/>)
//! and still used on current Windows 11 builds (e.g. the Windhawk "mac-island-dock" mod, 2026):
//!
//! * `CoCreateInstance(CLSID_QuietHoursSettings {F53321FA-34F8-4B7F-B9A3-361877CB94CF}, CLSCTX_LOCAL_SERVER)`
//!   → `IQuietHoursSettings {6BFF4732-81EC-4FFB-AE67-B6C1BC29631F}`, vtable after `IUnknown`:
//!   `get_UserSelectedProfile(LPWSTR*)`, `put_UserSelectedProfile(LPCWSTR)`.
//! * Profiles: `Microsoft.QuietHoursProfile.Unrestricted` (off), `Microsoft.QuietHoursProfile.PriorityOnly`
//!   (Windows 11 "Do not disturb" / Windows 10 "Priority only"), `Microsoft.QuietHoursProfile.AlarmsOnly` (Windows 10).
//! * Mute, in two steps: [`NotificationMuter::prepare_mute`] reads the user-selected profile
//!   (→ [`MuteToken::previous`]) without changing anything; the shell persists the token (and registers the
//!   logon safety net, see [`crate::logon`]); then [`NotificationMuter::apply_mute`] selects `PriorityOnly` if it
//!   was off. Unmute: if we switched it on (previous = off), select `Unrestricted` again; otherwise we never touched
//!   the setting and leave it alone. Because the token is on disk before Windows is changed, a crash during focus
//!   is repaired on the next start (or at the next logon).
//! * Fallback for reading the previous state when `get_UserSelectedProfile` fails: the WNF state
//!   `WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED` (`0x0D83063EA3BF1C75`) through `ntdll!NtQueryWnfStateData`
//!   (one DWORD: 0 = off, 1 = priority only, 2 = alarms only) — see <https://gist.github.com/riverar/980120d7e3a13ed8b1d665cf974c8e31>.
//!
//! Properties: per user, **no admin rights**, no policy, no registry edits for the setting itself (the only registry
//! write is the shell's per-user `RunOnce` safety net in [`crate::logon`]), no service/Explorer restarts,
//! fully reversible (we only ever switch between the built-in profiles), works on Windows 10 1803+ and Windows 11.
//!
//! # Rejected alternatives
//!
//! * Writing the WNF state: it is the shell's *published* state (an output, not an input); updating it does not
//!   change the stored setting and only confuses listeners.
//! * Registry `HKCU\...\Notifications\Settings\NOC_GLOBAL_SETTING_TOASTS_ENABLED` or the CloudStore
//!   `quiethourssettings` blob: only applied after restarting Explorer / `WpnUserService` (not allowed), and the
//!   CloudStore format is opaque.
//! * Group Policy / Intune: needs admin and is not reversible per session.
//!
//! # Limits
//!
//! * Undocumented: a future Windows build may change it. We detect that at runtime: `CoCreateInstance` failing →
//!   [`NotificationMuter::supported`] is `false` and `mute` returns [`PlatformError::Os`]; nothing else breaks.
//! * Automatic rules (gaming, full screen, duplicated display, scheduled hours) stay under Windows' control; we only
//!   change the *user-selected* profile.
//! * If Do not disturb was off before focus and the user changes it by hand during focus, unmute still turns it
//!   off again (the pre-focus selection).
//! * When Do not disturb was already on before focus (or the previous state could only be read from WNF, which
//!   also reflects automatic rules), mute leaves it unchanged and unmute does nothing, so we never write a
//!   profile the user did not have selected.

use serde::{Deserialize, Serialize};

use crate::PlatformError;

/// Restore information returned by [`NotificationMuter::mute`]; persisted by the shell for crash-restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MuteToken {
    /// Implementation that produced the token (e.g. [`METHOD_QUIET_HOURS`]).
    pub method: String,
    /// State before muting: [`PROFILE_OFF`], [`PROFILE_PRIORITY_ONLY`] or [`PROFILE_ALARMS_ONLY`].
    pub previous: u32,
}

/// Mutes system notifications during focus and restores them afterwards.
///
/// Muting is two steps so the caller can persist the restore token (crash-restore) **before** anything changes:
/// [`prepare_mute`](Self::prepare_mute) only reads the current state, [`apply_mute`](Self::apply_mute) changes it.
pub trait NotificationMuter: Send {
    /// The mechanism is available on this machine (may probe the OS once; cheap afterwards).
    fn supported(&self) -> bool;
    /// Reads the current state and returns the token that [`apply_mute`](Self::apply_mute) and
    /// [`unmute`](Self::unmute) work with. Changes nothing.
    fn prepare_mute(&mut self) -> Result<MuteToken, PlatformError>;
    /// Turns notifications off as planned in `token` (a no-op when they were already muted before).
    fn apply_mute(&mut self, token: &MuteToken) -> Result<(), PlatformError>;
    /// Both steps at once: turns notifications off, remembering the previous state in the token.
    fn mute(&mut self) -> Result<MuteToken, PlatformError> {
        let token = self.prepare_mute()?;
        self.apply_mute(&token)?;
        Ok(token)
    }
    /// Restores the state recorded in `token` (undoes only what `apply_mute` changed). Idempotent.
    fn unmute(&mut self, token: &MuteToken) -> Result<(), PlatformError>;
}

/// Platform muter: Windows quiet hours, or [`NoopMuter`] elsewhere.
pub fn notification_muter() -> Box<dyn NotificationMuter> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::WindowsMuter::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopMuter)
    }
}

/// Muter for unsupported platforms: `supported() == false`, muting fails with
/// [`PlatformError::Unsupported`], `unmute` is a successful no-op (nothing was muted).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMuter;

impl NotificationMuter for NoopMuter {
    fn supported(&self) -> bool {
        false
    }

    fn prepare_mute(&mut self) -> Result<MuteToken, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn apply_mute(&mut self, _token: &MuteToken) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn unmute(&mut self, _token: &MuteToken) -> Result<(), PlatformError> {
        Ok(())
    }
}

/// [`MuteToken::method`] of the Windows quiet-hours implementation.
pub const METHOD_QUIET_HOURS: &str = "quietHours";

/// Do not disturb / Focus assist off.
pub const PROFILE_OFF: u32 = 0;
/// Windows 11 "Do not disturb" (priority notifications still allowed) / Windows 10 "Priority only".
pub const PROFILE_PRIORITY_ONLY: u32 = 1;
/// Windows 10 "Alarms only".
pub const PROFILE_ALARMS_ONLY: u32 = 2;

const PROFILE_ID_UNRESTRICTED: &str = "Microsoft.QuietHoursProfile.Unrestricted";
const PROFILE_ID_PRIORITY_ONLY: &str = "Microsoft.QuietHoursProfile.PriorityOnly";
const PROFILE_ID_ALARMS_ONLY: &str = "Microsoft.QuietHoursProfile.AlarmsOnly";

/// Profile id → state code; `None` for unknown profiles.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn profile_code(profile_id: &str) -> Option<u32> {
    let id = profile_id.trim();
    if id.eq_ignore_ascii_case(PROFILE_ID_UNRESTRICTED) {
        Some(PROFILE_OFF)
    } else if id.eq_ignore_ascii_case(PROFILE_ID_PRIORITY_ONLY) {
        Some(PROFILE_PRIORITY_ONLY)
    } else if id.eq_ignore_ascii_case(PROFILE_ID_ALARMS_ONLY) {
        Some(PROFILE_ALARMS_ONLY)
    } else {
        None
    }
}

/// State code → profile id; unknown codes map to "off".
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn profile_id(code: u32) -> &'static str {
    match normalize_profile(code) {
        PROFILE_PRIORITY_ONLY => PROFILE_ID_PRIORITY_ONLY,
        PROFILE_ALARMS_ONLY => PROFILE_ID_ALARMS_ONLY,
        _ => PROFILE_ID_UNRESTRICTED,
    }
}

/// Clamps a stored/queried code to a known state (unknown → off).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn normalize_profile(code: u32) -> u32 {
    match code {
        PROFILE_PRIORITY_ONLY | PROFILE_ALARMS_ONLY => code,
        _ => PROFILE_OFF,
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{
        CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
        CoUninitialize,
    };
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, Interface, PCWSTR, PWSTR, s, w};

    use super::{
        METHOD_QUIET_HOURS, MuteToken, NotificationMuter, PROFILE_OFF, PROFILE_PRIORITY_ONLY,
        normalize_profile, profile_code, profile_id,
    };
    use crate::PlatformError;

    const CLSID_QUIET_HOURS_SETTINGS: GUID =
        GUID::from_u128(0xf53321fa_34f8_4b7f_b9a3_361877cb94cf);
    const WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED: u64 = 0x0D83_063E_A3BF_1C75;
    /// A failed availability probe is retried after this long (the COM server may not be up yet at logon).
    const PROBE_RETRY: Duration = Duration::from_secs(300);

    /// `IQuietHoursSettings` (undocumented, see module docs). Only the first two methods are declared.
    #[repr(transparent)]
    #[derive(Clone)]
    #[allow(dead_code)] // constructed by COM (`CoCreateInstance` → `Interface::from_raw`), never by Rust code
    struct IQuietHoursSettings(IUnknown);

    #[repr(C)]
    #[allow(dead_code)] // laid out for COM, read through `Interface::vtable`
    struct QuietHoursSettingsVtbl {
        base: IUnknown_Vtbl,
        get_user_selected_profile:
            unsafe extern "system" fn(this: *mut c_void, profile_id: *mut PWSTR) -> HRESULT,
        put_user_selected_profile:
            unsafe extern "system" fn(this: *mut c_void, profile_id: PCWSTR) -> HRESULT,
    }

    // SAFETY: `IQuietHoursSettings` is a transparent wrapper of a COM interface pointer (via `IUnknown`) and its
    // vtable starts with `IUnknown` followed by the methods declared in `QuietHoursSettingsVtbl`, in this order.
    unsafe impl Interface for IQuietHoursSettings {
        type Vtable = QuietHoursSettingsVtbl;
        const IID: GUID = GUID::from_u128(0x6bff4732_81ec_4ffb_ae67_b6c1bc29631f);
    }

    impl IQuietHoursSettings {
        fn user_selected_profile(&self) -> Result<String, PlatformError> {
            let mut raw = PWSTR::null();
            // SAFETY: `self` is a live interface pointer with the declared vtable; `raw` is a valid out pointer.
            let hr = unsafe { (self.vtable().get_user_selected_profile)(self.as_raw(), &mut raw) };
            hr.ok()?;
            if raw.is_null() {
                return Err(PlatformError::Os("empty quiet hours profile id".into()));
            }
            // SAFETY: on success the callee returns a NUL-terminated, CoTaskMemAlloc'ed string we now own.
            let text = unsafe { raw.to_string() };
            // SAFETY: freed exactly once, after the last read.
            unsafe { CoTaskMemFree(Some(raw.0 as *const c_void)) };
            text.map_err(|err| PlatformError::Os(format!("invalid quiet hours profile id: {err}")))
        }

        fn set_user_selected_profile(&self, code: u32) -> Result<(), PlatformError> {
            let wide: Vec<u16> = profile_id(code)
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: `self` is a live interface pointer; `wide` is NUL-terminated and outlives the call.
            let hr = unsafe {
                (self.vtable().put_user_selected_profile)(self.as_raw(), PCWSTR(wide.as_ptr()))
            };
            hr.ok()?;
            Ok(())
        }
    }

    /// COM initialization for the current thread, undone on drop (if we did it).
    struct ComScope {
        initialized: bool,
    }

    impl ComScope {
        fn enter() -> Result<Self, PlatformError> {
            // SAFETY: plain FFI call; paired with CoUninitialize in Drop when it succeeded.
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if hr.is_ok() {
                Ok(Self { initialized: true })
            } else if hr == RPC_E_CHANGED_MODE {
                // The thread already uses an STA; COM is usable, just not ours to uninitialize.
                Ok(Self { initialized: false })
            } else {
                Err(PlatformError::Os(format!(
                    "CoInitializeEx failed: {}",
                    windows::core::Error::from_hresult(hr)
                )))
            }
        }
    }

    impl Drop for ComScope {
        fn drop(&mut self) {
            if self.initialized {
                // SAFETY: balances the successful CoInitializeEx in `enter`; all interfaces are released before
                // (declare the scope before any COM object so it is dropped last).
                unsafe { CoUninitialize() };
            }
        }
    }

    fn open_settings() -> Result<IQuietHoursSettings, PlatformError> {
        // SAFETY: COM is initialized on this thread (caller holds a `ComScope`).
        unsafe {
            CoCreateInstance::<_, IQuietHoursSettings>(
                &CLSID_QUIET_HOURS_SETTINGS,
                None::<&IUnknown>,
                CLSCTX_LOCAL_SERVER,
            )
        }
        .map_err(|err| PlatformError::Os(format!("QuietHoursSettings unavailable: {err}")))
    }

    /// Reads the shell's active quiet-hours profile from WNF (0 off, 1 priority only, 2 alarms only).
    fn read_active_profile() -> Result<u32, PlatformError> {
        type NtQueryWnfStateData = unsafe extern "system" fn(
            state_name: *const u64,
            type_id: *const c_void,
            explicit_scope: *const c_void,
            change_stamp: *mut u32,
            buffer: *mut c_void,
            buffer_size: *mut u32,
        ) -> i32;

        // SAFETY: ntdll is loaded in every process; the handle is not reference counted.
        let ntdll = unsafe { GetModuleHandleW(w!("ntdll.dll")) }?;
        // SAFETY: valid module handle and NUL-terminated name.
        let proc = unsafe { GetProcAddress(ntdll, s!("NtQueryWnfStateData")) }
            .ok_or_else(|| PlatformError::Os("NtQueryWnfStateData not found".into()))?;
        // SAFETY: NtQueryWnfStateData has this (stable since Windows 8) signature.
        let query = unsafe {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, NtQueryWnfStateData>(proc)
        };

        let state_name = WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED;
        let mut change_stamp = 0u32;
        let mut value = 0u32;
        let mut size = size_of::<u32>() as u32;
        // SAFETY: all pointers are valid for the call; the buffer holds `size` bytes.
        let status = unsafe {
            query(
                &state_name,
                std::ptr::null(),
                std::ptr::null(),
                &mut change_stamp,
                (&mut value as *mut u32).cast::<c_void>(),
                &mut size,
            )
        };
        if status < 0 {
            return Err(PlatformError::Os(format!(
                "NtQueryWnfStateData failed: NTSTATUS 0x{status:08X}"
            )));
        }
        if size as usize != size_of::<u32>() {
            return Err(PlatformError::Os(format!(
                "unexpected quiet hours state size {size}"
            )));
        }
        Ok(value)
    }

    /// The user-selected profile before muting; falls back to the active profile from WNF.
    fn previous_profile(settings: &IQuietHoursSettings) -> u32 {
        let selected = match settings.user_selected_profile() {
            Ok(id) => {
                let code = profile_code(&id);
                if code.is_none() {
                    log::warn!("unknown quiet hours profile {id:?}");
                }
                code
            }
            Err(err) => {
                log::warn!("reading the selected quiet hours profile failed: {err}");
                None
            }
        };
        selected.unwrap_or_else(|| match read_active_profile() {
            Ok(code) => normalize_profile(code),
            Err(err) => {
                log::warn!("reading the active quiet hours profile failed: {err}; assuming off");
                PROFILE_OFF
            }
        })
    }

    fn probe() -> bool {
        let available = ComScope::enter().and_then(|scope| {
            // Release the interface before COM is uninitialized.
            let opened = open_settings().map(drop);
            drop(scope);
            opened
        });
        match available {
            Ok(()) => true,
            Err(err) => {
                log::warn!("windows do-not-disturb control unavailable: {err}");
                false
            }
        }
    }

    pub(super) struct WindowsMuter {
        /// Cached availability probe: (available, when probed).
        probe: Cell<Option<(bool, Instant)>>,
    }

    impl WindowsMuter {
        pub(super) fn new() -> Self {
            Self {
                probe: Cell::new(None),
            }
        }
    }

    impl NotificationMuter for WindowsMuter {
        fn supported(&self) -> bool {
            if let Some((available, at)) = self.probe.get()
                && (available || at.elapsed() < PROBE_RETRY)
            {
                return available;
            }
            let available = probe();
            self.probe.set(Some((available, Instant::now())));
            available
        }

        fn prepare_mute(&mut self) -> Result<MuteToken, PlatformError> {
            let _com = ComScope::enter()?;
            let settings = open_settings()?;
            let previous = previous_profile(&settings);
            self.probe.set(Some((true, Instant::now())));
            Ok(MuteToken {
                method: METHOD_QUIET_HOURS.to_owned(),
                previous,
            })
        }

        fn apply_mute(&mut self, token: &MuteToken) -> Result<(), PlatformError> {
            if token.method != METHOD_QUIET_HOURS {
                return Err(PlatformError::Os(format!(
                    "notification token of an unknown method {:?}",
                    token.method
                )));
            }
            let previous = normalize_profile(token.previous);
            if previous != PROFILE_OFF {
                // Mirrors `unmute`: what was on before focus is neither changed now nor undone later.
                log::info!(
                    "windows do-not-disturb already active (profile {previous}), leaving it as is"
                );
                return Ok(());
            }
            let _com = ComScope::enter()?;
            let settings = open_settings()?;
            settings.set_user_selected_profile(PROFILE_PRIORITY_ONLY)?;
            log::info!("windows do-not-disturb turned on");
            Ok(())
        }

        fn unmute(&mut self, token: &MuteToken) -> Result<(), PlatformError> {
            if token.method != METHOD_QUIET_HOURS {
                // Not produced by this implementation (foreign or corrupted restore file): retrying can never
                // succeed, so report success and let the shell drop the stale token instead of failing forever.
                log::warn!(
                    "ignoring a notification restore token with unknown method {:?}",
                    token.method
                );
                return Ok(());
            }
            let previous = normalize_profile(token.previous);
            if previous != PROFILE_OFF {
                // `mute` did not change anything (Do not disturb was already on), so there is nothing to undo.
                // Writing `previous` back could turn on a profile that was only active through an automatic rule.
                log::info!(
                    "windows do-not-disturb was on before focus (profile {previous}), left as is"
                );
                return Ok(());
            }
            let _com = ComScope::enter()?;
            let settings = open_settings()?;
            settings.set_user_selected_profile(PROFILE_OFF)?;
            log::info!("windows do-not-disturb turned off again");
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::NotificationMuter;
        use super::{
            METHOD_QUIET_HOURS, MuteToken, PROFILE_OFF, PROFILE_PRIORITY_ONLY, WindowsMuter,
            read_active_profile,
        };

        #[test]
        fn unmute_only_undoes_our_own_change() {
            // Neither token needs a write, so the user's real setting is not touched.
            let mut muter = WindowsMuter::new();
            let already_on = MuteToken {
                method: METHOD_QUIET_HOURS.to_owned(),
                previous: PROFILE_PRIORITY_ONLY,
            };
            assert!(muter.unmute(&already_on).is_ok());
            let foreign = MuteToken {
                method: "somethingElse".to_owned(),
                previous: PROFILE_OFF,
            };
            assert!(muter.unmute(&foreign).is_ok());
        }

        #[test]
        fn apply_mute_leaves_an_active_profile_alone() {
            // No write happens for either token, so the user's real setting is not touched.
            let mut muter = WindowsMuter::new();
            let already_on = MuteToken {
                method: METHOD_QUIET_HOURS.to_owned(),
                previous: PROFILE_PRIORITY_ONLY,
            };
            assert!(muter.apply_mute(&already_on).is_ok());
            let foreign = MuteToken {
                method: "somethingElse".to_owned(),
                previous: PROFILE_OFF,
            };
            assert!(muter.apply_mute(&foreign).is_err());
        }

        #[test]
        fn supported_probe_does_not_panic() {
            let muter = WindowsMuter::new();
            let first = muter.supported();
            // Cached result is stable.
            assert_eq!(first, muter.supported());
        }

        #[test]
        fn wnf_state_is_readable_or_reports_error() {
            // Must never panic; the value (if any) is a small profile code.
            if let Ok(value) = read_active_profile() {
                assert!(value <= 16, "unexpected profile code {value}");
            }
        }

        /// Changes the real Do not disturb state for a moment — run manually:
        /// `cargo test -p focus-platform -- --ignored dnd_round_trip`
        #[test]
        #[ignore = "toggles the user's real Do not disturb setting"]
        fn dnd_round_trip() {
            let mut muter = WindowsMuter::new();
            assert!(muter.supported());
            let token = muter.mute().expect("mute");
            std::thread::sleep(std::time::Duration::from_secs(2));
            muter.unmute(&token).expect("unmute");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_json_round_trip() {
        let token = MuteToken {
            method: METHOD_QUIET_HOURS.to_owned(),
            previous: PROFILE_PRIORITY_ONLY,
        };
        let json = serde_json::to_string(&token).expect("serialize");
        assert_eq!(json, r#"{"method":"quietHours","previous":1}"#);
        let back: MuteToken = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, token);
    }

    #[test]
    fn profile_ids_round_trip() {
        for code in [PROFILE_OFF, PROFILE_PRIORITY_ONLY, PROFILE_ALARMS_ONLY] {
            assert_eq!(profile_code(profile_id(code)), Some(code));
        }
        assert_eq!(
            profile_code("microsoft.quiethoursprofile.priorityonly"),
            Some(PROFILE_PRIORITY_ONLY)
        );
        assert_eq!(profile_code("Microsoft.QuietHoursProfile.Custom"), None);
        assert_eq!(profile_code(""), None);
    }

    #[test]
    fn unknown_codes_mean_off() {
        assert_eq!(normalize_profile(7), PROFILE_OFF);
        assert_eq!(normalize_profile(PROFILE_ALARMS_ONLY), PROFILE_ALARMS_ONLY);
        assert_eq!(profile_id(99), "Microsoft.QuietHoursProfile.Unrestricted");
    }

    #[test]
    fn noop_muter() {
        let mut muter = NoopMuter;
        assert!(!muter.supported());
        assert!(matches!(muter.mute(), Err(PlatformError::Unsupported)));
        assert!(matches!(
            muter.prepare_mute(),
            Err(PlatformError::Unsupported)
        ));
        let token = MuteToken {
            method: METHOD_QUIET_HOURS.to_owned(),
            previous: PROFILE_OFF,
        };
        assert!(matches!(
            muter.apply_mute(&token),
            Err(PlatformError::Unsupported)
        ));
        assert!(muter.unmute(&token).is_ok());
    }

    #[test]
    fn platform_muter_is_constructible() {
        let muter = notification_muter();
        #[cfg(not(target_os = "windows"))]
        assert!(!muter.supported());
        #[cfg(target_os = "windows")]
        let _ = muter.supported();
    }
}
