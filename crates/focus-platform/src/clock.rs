//! Local wall-clock time of day (for the time-of-day screen tint).
//!
//! `focus-core` never reads the clock; the shell asks here and passes the value to `focus_core::tint::tint_level`.
//!
//! * Windows: `GetLocalTime` (local time zone and daylight saving time applied by the OS).
//! * Elsewhere: `None` (the standard library has no local time zone, and we do not add a time zone database
//!   just for the tint, which is a Windows feature anyway).

/// Minutes since local midnight, with the fraction from seconds and milliseconds (`18:15:30` → `1095.5`),
/// always in `0.0..1440.0`; `None` when the local time is unavailable.
pub fn local_minutes_of_day() -> Option<f64> {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: no preconditions; the function only fills the returned SYSTEMTIME.
        let now = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        minutes_of_day(now.wHour, now.wMinute, now.wSecond, now.wMilliseconds)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// `hour * 60 + minute + second / 60 + millis / 60000`; `None` for out-of-range fields.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn minutes_of_day(hour: u16, minute: u16, second: u16, millis: u16) -> Option<f64> {
    if hour >= 24 || minute >= 60 || second >= 60 || millis >= 1000 {
        return None;
    }
    Some(
        f64::from(hour) * 60.0
            + f64::from(minute)
            + f64::from(second) / 60.0
            + f64::from(millis) / 60_000.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_include_fractions() {
        assert_eq!(minutes_of_day(0, 0, 0, 0), Some(0.0));
        assert_eq!(minutes_of_day(18, 15, 30, 0), Some(1095.5));
        assert_eq!(minutes_of_day(9, 0, 0, 0), Some(540.0));
        assert_eq!(minutes_of_day(0, 0, 0, 600), Some(0.01));
        let last = minutes_of_day(23, 59, 59, 999).expect("valid");
        assert!(last < 1440.0 && last > 1439.99);
    }

    #[test]
    fn out_of_range_fields_are_rejected() {
        assert_eq!(minutes_of_day(24, 0, 0, 0), None);
        assert_eq!(minutes_of_day(12, 60, 0, 0), None);
        assert_eq!(minutes_of_day(12, 0, 60, 0), None);
        assert_eq!(minutes_of_day(12, 0, 0, 1000), None);
    }

    #[test]
    fn platform_clock() {
        let now = local_minutes_of_day();
        #[cfg(target_os = "windows")]
        assert!(now.is_some_and(|m| (0.0..1440.0).contains(&m)), "{now:?}");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(now, None);
    }
}
