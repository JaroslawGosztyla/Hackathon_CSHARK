//! Foreground window sampling: process name of the active window, its title, its window class and the user's
//! idle time.
//!
//! Used by the shell's tracker (productivity chart by process name, with Windows shell surfaces such as the
//! taskbar or the desktop recognised by class) and by the blocked-site fallback (window title of the foreground
//! browser). Nothing leaves the computer.
//!
//! Windows implementation:
//! * `GetForegroundWindow` → `InternalGetWindowText` (title), `GetClassNameW` (class) and
//!   `GetWindowThreadProcessId` (pid). Unlike `GetWindowTextW`, `InternalGetWindowText` never sends `WM_GETTEXT`,
//!   so sampling our own window (FocusMode in the foreground) never waits for the UI thread; `GetClassNameW`
//!   reads the class atom and sends no message either,
//! * `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` → executable file name
//!   (works for elevated processes too; protected processes and pid 0/4 report `None` = unknown),
//! * UWP apps are hosted by `ApplicationFrameHost.exe`; best effort we report the process of the first child
//!   window that belongs to another process (the app's `CoreWindow`),
//! * `GetLastInputInfo` + `GetTickCount` → seconds since the last keyboard/mouse input (session-wide).
//!
//! pid → name lookups are cached for [`CACHE_TTL`] (pids can be reused, so entries expire).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::PlatformError;

/// One sample of the foreground state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundInfo {
    /// Executable file name, e.g. `Code.exe`; `None` when it cannot be determined.
    pub process: Option<String>,
    /// Title of the foreground window (may be empty). The shell decides whether it is kept; never log it.
    pub title: String,
    /// Window class of the foreground window, e.g. `Chrome_WidgetWin_1`, `Shell_TrayWnd` (taskbar),
    /// `ApplicationFrameWindow` (UWP frame); empty when there is no foreground window or it cannot be read.
    pub class_name: String,
    /// Seconds since the last keyboard/mouse input anywhere in the session.
    pub idle_secs: u32,
}

/// Samples the foreground window. Call from a worker thread (one sample per second is cheap).
pub trait ForegroundProbe: Send {
    fn sample(&mut self) -> Result<ForegroundInfo, PlatformError>;
}

/// Platform probe: Windows implementation, or [`NoopProbe`] elsewhere.
pub fn probe() -> Box<dyn ForegroundProbe> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::WindowsProbe::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopProbe)
    }
}

/// Probe for unsupported platforms: every sample is [`PlatformError::Unsupported`].
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProbe;

impl ForegroundProbe for NoopProbe {
    fn sample(&mut self) -> Result<ForegroundInfo, PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

/// How long a pid → process name entry stays valid.
pub const CACHE_TTL: Duration = Duration::from_secs(30);
/// Upper bound of cached pids; the cache is pruned when it grows beyond this.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const CACHE_MAX_ENTRIES: usize = 256;
/// Host process of UWP app windows.
const FRAME_HOST: &str = "ApplicationFrameHost.exe";

/// File name part of a full image path (`C:\Program Files\App\app.exe` → `app.exe`).
/// Accepts both separators; returns `None` for an empty name.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn image_file_name(path: &str) -> Option<String> {
    let name = path.trim().rsplit(['\\', '/']).next()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// `true` for the UWP frame host (`ApplicationFrameHost.exe`, any case).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_frame_host(process: &str) -> bool {
    process.eq_ignore_ascii_case(FRAME_HOST)
}

/// Idle seconds from 32-bit millisecond tick counts (handles the 49.7-day wrap-around).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn idle_secs_from_ticks(now_ms: u32, last_input_ms: u32) -> u32 {
    now_ms.wrapping_sub(last_input_ms) / 1000
}

/// pid → process name cache with expiry (pids are reused by the OS).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug)]
struct ProcessNameCache {
    entries: HashMap<u32, (String, Instant)>,
    ttl: Duration,
    max_entries: usize,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
impl ProcessNameCache {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
            max_entries: max_entries.max(1),
        }
    }

    fn get(&self, pid: u32, now: Instant) -> Option<String> {
        self.entries
            .get(&pid)
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.ttl)
            .map(|(name, _)| name.clone())
    }

    fn insert(&mut self, pid: u32, name: String, now: Instant) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&pid) {
            let ttl = self.ttl;
            self.entries
                .retain(|_, (_, at)| now.saturating_duration_since(*at) < ttl);
            if self.entries.len() >= self.max_entries {
                self.entries.clear();
            }
        }
        self.entries.insert(pid, (name, now));
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::time::Instant;

    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM};
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId,
        InternalGetWindowText,
    };
    use windows::core::{BOOL, PWSTR};

    use super::{
        CACHE_MAX_ENTRIES, CACHE_TTL, ForegroundInfo, ForegroundProbe, ProcessNameCache,
        idle_secs_from_ticks, image_file_name, is_frame_host,
    };
    use crate::PlatformError;

    /// Longest window title we read (in UTF-16 units); longer titles are truncated.
    const MAX_TITLE_CHARS: usize = 2048;
    /// Window class names are at most 256 characters (including the terminating NUL).
    const MAX_CLASS_CHARS: usize = 256;
    /// Image path buffer sizes (UTF-16 units): typical paths first, then the long-path maximum.
    const IMAGE_PATH_BUFFERS: [usize; 2] = [1024, 32_768];

    pub(super) struct WindowsProbe {
        cache: ProcessNameCache,
    }

    impl WindowsProbe {
        pub(super) fn new() -> Self {
            Self {
                cache: ProcessNameCache::new(CACHE_TTL, CACHE_MAX_ENTRIES),
            }
        }

        fn process_name(&mut self, pid: u32, now: Instant) -> Option<String> {
            if let Some(name) = self.cache.get(pid, now) {
                return Some(name);
            }
            let name = query_process_name(pid)?;
            self.cache.insert(pid, name.clone(), now);
            Some(name)
        }
    }

    impl ForegroundProbe for WindowsProbe {
        fn sample(&mut self) -> Result<ForegroundInfo, PlatformError> {
            let idle_secs = idle_secs();
            let now = Instant::now();

            // SAFETY: no preconditions; returns a null HWND when no window is active (e.g. during a desktop switch).
            let hwnd = unsafe { GetForegroundWindow() };
            if hwnd.is_invalid() {
                return Ok(ForegroundInfo {
                    process: None,
                    title: String::new(),
                    class_name: String::new(),
                    idle_secs,
                });
            }

            let title = window_title(hwnd);
            let class_name = window_class(hwnd);
            let pid = window_pid(hwnd);
            let mut process = pid.and_then(|pid| self.process_name(pid, now));

            // UWP apps: the frame belongs to ApplicationFrameHost.exe, the content to the app process.
            if let Some(host_pid) = pid
                && process.as_deref().is_some_and(is_frame_host)
                && let Some(child_pid) = foreign_child_pid(hwnd, host_pid)
                && let Some(name) = self.process_name(child_pid, now)
            {
                process = Some(name);
            }

            Ok(ForegroundInfo {
                process,
                title,
                class_name,
                idle_secs,
            })
        }
    }

    /// Closes a process handle on drop.
    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful OpenProcess and is closed exactly once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    fn idle_secs() -> u32 {
        let mut info = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: `info` is a valid, initialized LASTINPUTINFO with cbSize set.
        let ok = unsafe { GetLastInputInfo(&mut info) };
        if !ok.as_bool() {
            return 0;
        }
        // SAFETY: no preconditions. LASTINPUTINFO.dwTime uses the same 32-bit tick clock.
        let now = unsafe { GetTickCount() };
        idle_secs_from_ticks(now, info.dwTime)
    }

    fn window_title(hwnd: HWND) -> String {
        // Room for MAX_TITLE_CHARS characters plus the terminating NUL; longer titles are truncated.
        let mut buffer = vec![0u16; MAX_TITLE_CHARS + 1];
        // SAFETY: any HWND value is accepted (an invalid window yields 0); the buffer is valid for its whole
        // length and the API writes at most that many UTF-16 units, including the NUL. It reads the caption
        // from the window structure and never sends a message, so it cannot block on a busy UI thread.
        let copied = unsafe { InternalGetWindowText(hwnd, &mut buffer) };
        let copied = usize::try_from(copied).unwrap_or(0).min(MAX_TITLE_CHARS);
        String::from_utf16_lossy(&buffer[..copied])
    }

    fn window_class(hwnd: HWND) -> String {
        let mut buffer = [0u16; MAX_CLASS_CHARS];
        // SAFETY: any HWND value is accepted (an invalid window yields 0); the wrapper passes the buffer length and
        // the API writes at most that many UTF-16 units, including the NUL. No message is sent.
        let copied = unsafe { GetClassNameW(hwnd, &mut buffer) };
        let copied = usize::try_from(copied).unwrap_or(0).min(MAX_CLASS_CHARS);
        String::from_utf16_lossy(&buffer[..copied])
    }

    fn window_pid(hwnd: HWND) -> Option<u32> {
        let mut pid = 0u32;
        // SAFETY: `pid` is a valid out pointer for the duration of the call.
        let _thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32)) };
        (pid != 0).then_some(pid)
    }

    fn query_process_name(pid: u32) -> Option<String> {
        // SAFETY: plain FFI call; failure (access denied, exited process) is reported as Err.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let handle = OwnedHandle(handle);

        for capacity in IMAGE_PATH_BUFFERS {
            let mut buffer = vec![0u16; capacity];
            let mut size = u32::try_from(buffer.len()).unwrap_or(u32::MAX);
            // SAFETY: `buffer` holds `size` UTF-16 units and outlives the call; `size` is a valid in/out pointer.
            let result = unsafe {
                QueryFullProcessImageNameW(
                    handle.0,
                    PROCESS_NAME_WIN32,
                    PWSTR(buffer.as_mut_ptr()),
                    &mut size,
                )
            };
            if result.is_ok() {
                let len = usize::try_from(size).unwrap_or(0).min(buffer.len());
                return image_file_name(&String::from_utf16_lossy(&buffer[..len]));
            }
        }
        None
    }

    /// State shared with [`find_foreign_child`] through the `LPARAM`.
    struct ChildSearch {
        host_pid: u32,
        found: Option<u32>,
    }

    /// `EnumChildWindows` callback: stops at the first child window owned by another process.
    unsafe extern "system" fn find_foreign_child(child: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` carries a `*mut ChildSearch` created in `foreign_child_pid`, which keeps the value
        // alive and otherwise untouched for the whole (synchronous) EnumChildWindows call.
        let search = unsafe { &mut *(lparam.0 as *mut ChildSearch) };
        let mut pid = 0u32;
        // SAFETY: `pid` is a valid out pointer for the duration of the call.
        let _thread = unsafe { GetWindowThreadProcessId(child, Some(&mut pid as *mut u32)) };
        if pid != 0 && pid != search.host_pid {
            search.found = Some(pid);
            BOOL(0)
        } else {
            BOOL(1)
        }
    }

    fn foreign_child_pid(hwnd: HWND, host_pid: u32) -> Option<u32> {
        let mut search = ChildSearch {
            host_pid,
            found: None,
        };
        // SAFETY: the callback only dereferences the pointer to `search`, which outlives this synchronous call.
        // The return value only says whether enumeration was stopped early, so it is ignored.
        let _ = unsafe {
            EnumChildWindows(
                Some(hwnd),
                Some(find_foreign_child),
                LPARAM(&mut search as *mut ChildSearch as isize),
            )
        };
        search.found
    }

    #[cfg(test)]
    mod tests {
        use super::super::ForegroundProbe;
        use super::WindowsProbe;

        #[test]
        fn sampling_does_not_fail() {
            let mut probe = WindowsProbe::new();
            // A headless CI session may have no foreground window; the call must still succeed.
            let first = probe.sample();
            assert!(first.is_ok());
            let second = probe.sample();
            assert!(second.is_ok());
        }

        #[test]
        fn window_class_resolves() {
            use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
            // SAFETY: no preconditions; the desktop window always exists.
            let desktop = unsafe { GetDesktopWindow() };
            assert_eq!(super::window_class(desktop), "#32769");
            assert_eq!(
                super::window_class(windows::Win32::Foundation::HWND::default()),
                ""
            );
        }

        #[test]
        fn own_process_name_resolves() {
            let pid = std::process::id();
            let name = super::query_process_name(pid);
            assert!(
                name.as_deref()
                    .is_some_and(|n| n.to_ascii_lowercase().ends_with(".exe"))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_file_name_takes_last_component() {
        assert_eq!(
            image_file_name(r"C:\Program Files\Microsoft VS Code\Code.exe").as_deref(),
            Some("Code.exe")
        );
        assert_eq!(
            image_file_name("/usr/bin/firefox").as_deref(),
            Some("firefox")
        );
        assert_eq!(image_file_name("chrome.exe").as_deref(), Some("chrome.exe"));
        assert_eq!(image_file_name(r"C:\dir\"), None);
        assert_eq!(image_file_name(""), None);
        assert_eq!(image_file_name("   "), None);
    }

    #[test]
    fn frame_host_is_case_insensitive() {
        assert!(is_frame_host("ApplicationFrameHost.exe"));
        assert!(is_frame_host("applicationframehost.EXE"));
        assert!(!is_frame_host("explorer.exe"));
    }

    #[test]
    fn idle_handles_tick_wraparound() {
        assert_eq!(idle_secs_from_ticks(10_000, 4_000), 6);
        assert_eq!(idle_secs_from_ticks(999, 0), 0);
        // Tick counter wrapped between the last input and now.
        assert_eq!(idle_secs_from_ticks(1_500, u32::MAX - 499), 2);
        assert_eq!(idle_secs_from_ticks(5, 5), 0);
    }

    #[test]
    fn cache_returns_fresh_entries_only() {
        let start = Instant::now();
        let mut cache = ProcessNameCache::new(Duration::from_secs(30), 8);
        cache.insert(42, "Code.exe".into(), start);
        assert_eq!(
            cache.get(42, start + Duration::from_secs(29)).as_deref(),
            Some("Code.exe")
        );
        assert_eq!(cache.get(42, start + Duration::from_secs(30)), None);
        assert_eq!(cache.get(7, start), None);
    }

    #[test]
    fn cache_is_bounded() {
        let start = Instant::now();
        let mut cache = ProcessNameCache::new(Duration::from_secs(30), 4);
        for pid in 0..4 {
            cache.insert(pid, format!("p{pid}.exe"), start);
        }
        assert_eq!(cache.len(), 4);
        // Full of fresh entries → cleared, then the new one is stored.
        cache.insert(100, "new.exe".into(), start);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(100, start).as_deref(), Some("new.exe"));

        // Expired entries are pruned first.
        let mut cache = ProcessNameCache::new(Duration::from_secs(30), 2);
        cache.insert(1, "old.exe".into(), start);
        cache.insert(2, "fresh.exe".into(), start + Duration::from_secs(40));
        cache.insert(3, "newer.exe".into(), start + Duration::from_secs(45));
        assert_eq!(cache.get(1, start + Duration::from_secs(45)), None);
        assert_eq!(
            cache.get(2, start + Duration::from_secs(45)).as_deref(),
            Some("fresh.exe")
        );
        assert_eq!(
            cache.get(3, start + Duration::from_secs(45)).as_deref(),
            Some("newer.exe")
        );
    }

    #[test]
    fn noop_probe_is_unsupported() {
        let mut probe = NoopProbe;
        assert!(matches!(probe.sample(), Err(PlatformError::Unsupported)));
    }

    #[test]
    fn platform_probe_is_constructible() {
        let mut probe = probe();
        let result = probe.sample();
        #[cfg(target_os = "windows")]
        assert!(result.is_ok());
        #[cfg(not(target_os = "windows"))]
        assert!(matches!(result, Err(PlatformError::Unsupported)));
    }
}
