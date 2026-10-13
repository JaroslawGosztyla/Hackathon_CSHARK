//! Local wall-clock time of day (for the time-of-day screen tint) and the local date with the minute of the day (for
//! the reward day of the beta rewards, ADR-022).
//!
//! `focus-core` never reads the clock; the shell asks here and passes the value to `focus_core::tint::tint_level` and
//! `focus_core::rewards::reward_day`.
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

/// The local date and the minute of the day: `(year, month 1..=12, day 1..=31, minute 0..1440)` (`2026-10-13 18:15` →
/// `(2026, 10, 13, 1095)`); `None` when the local time is unavailable (the shell then falls back to the UTC date, best
/// effort, ADR-008).
pub fn local_date_time() -> Option<(i32, u32, u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: no preconditions; the function only fills the returned SYSTEMTIME.
        let now = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        date_time(now.wYear, now.wMonth, now.wDay, now.wHour, now.wMinute)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// `(year, month, day, hour * 60 + minute)`; `None` for out-of-range fields (the day of the month is checked
/// against the month later, by `focus_core::rewards::reward_day`).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn date_time(
    year: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
) -> Option<(i32, u32, u32, u32)> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour >= 24 || minute >= 60 {
        return None;
    }
    Some((
        i32::from(year),
        u32::from(month),
        u32::from(day),
        u32::from(hour) * 60 + u32::from(minute),
    ))
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
    fn date_fields_are_validated() {
        assert_eq!(date_time(2026, 10, 13, 18, 15), Some((2026, 10, 13, 1095)));
        assert_eq!(date_time(2026, 1, 1, 0, 0), Some((2026, 1, 1, 0)));
        assert_eq!(date_time(2026, 12, 31, 23, 59), Some((2026, 12, 31, 1439)));
        for (month, day, hour, minute) in [
            (0, 1, 0, 0),
            (13, 1, 0, 0),
            (1, 0, 0, 0),
            (1, 32, 0, 0),
            (1, 1, 24, 0),
            (1, 1, 0, 60),
        ] {
            assert_eq!(
                date_time(2026, month, day, hour, minute),
                None,
                "{month}-{day} {hour}:{minute}"
            );
        }
    }

    #[test]
    fn platform_date() {
        let now = local_date_time();
        #[cfg(target_os = "windows")]
        assert!(
            now.is_some_and(|(year, month, day, minute)| year >= 2020
                && (1..=12).contains(&month)
                && (1..=31).contains(&day)
                && minute < 1440),
            "{now:?}"
        );
        #[cfg(not(target_os = "windows"))]
        assert_eq!(now, None);
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
