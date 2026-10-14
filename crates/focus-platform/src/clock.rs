//! Local wall-clock time of day (for the time-of-day screen tint) and the local date with the minute of the day (for
//! the reward day of the beta rewards, ADR-022).
//!
//! `focus-core` never reads the clock; the shell asks here and passes the value to `focus_core::tint::tint_level` and
//! `focus_core::rewards::reward_day`.
//!
//! * Windows: `GetLocalTime` (local time zone and daylight saving time applied by the OS).
//! * Elsewhere: `None` (the standard library has no local time zone, and we do not add a time zone database
//!   just for the tint, which is a Windows feature anyway).
//!
//! [`utc_offset_minutes_at`] gives the UTC offset of the user's time zone at any instant, past ones included, for the
//! weekly stats (ADR-023: `focus_core::stats::WeekAggregator` places stored sessions by local hour).

/// Milliseconds between 1601-01-01 (the `FILETIME` epoch) and 1970-01-01.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const FILETIME_EPOCH_OFFSET_MS: i64 = 11_644_473_600_000;
/// `FILETIME` ticks (100 ns) per millisecond.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const FILETIME_TICKS_PER_MS: i64 = 10_000;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const MS_PER_MINUTE: i64 = 60_000;

/// The UTC offset of the local time zone at the instant `utc_ms` (Unix epoch milliseconds), in minutes east of UTC
/// (Warsaw in summer → `120`, in winter → `60`), with the daylight-saving rule of that date; `None` when unknown
/// (outside Windows, or an instant before 1601 or after 30827).
pub fn utc_offset_minutes_at(utc_ms: i64) -> Option<i32> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
        use windows::Win32::System::Time::{
            FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime,
        };

        let ticks = utc_ms
            .checked_add(FILETIME_EPOCH_OFFSET_MS)?
            .checked_mul(FILETIME_TICKS_PER_MS)?;
        let ticks = u64::try_from(ticks).ok()?;
        let utc_file = FILETIME {
            // Low and high halves of the 64-bit tick count.
            dwLowDateTime: (ticks & u64::from(u32::MAX)) as u32,
            dwHighDateTime: (ticks >> 32) as u32,
        };
        let mut utc = SYSTEMTIME::default();
        let mut local = SYSTEMTIME::default();
        let mut local_file = FILETIME::default();
        // SAFETY: every pointer refers to a live local of the expected type; `None` = the current time zone.
        unsafe {
            FileTimeToSystemTime(&utc_file, &mut utc).ok()?;
            SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).ok()?;
            SystemTimeToFileTime(&local, &mut local_file).ok()?;
        }
        let local_ticks =
            (u64::from(local_file.dwHighDateTime) << 32) | u64::from(local_file.dwLowDateTime);
        let local_ticks = i64::try_from(local_ticks).ok()?;
        let delta_ms = (local_ticks / FILETIME_TICKS_PER_MS)
            .checked_sub(FILETIME_EPOCH_OFFSET_MS)?
            .checked_sub(utc_ms)?;
        i32::try_from(delta_ms.div_euclid(MS_PER_MINUTE)).ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = utc_ms;
        None
    }
}

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
    fn platform_utc_offset() {
        let max_minutes = 18 * 60;
        #[cfg(target_os = "windows")]
        for at in [0, 1_760_000_000_000, 1_790_000_000_000] {
            let offset = utc_offset_minutes_at(at);
            assert!(
                offset.is_some_and(|m| (-max_minutes..=max_minutes).contains(&m)),
                "{at}: {offset:?}"
            );
        }
        #[cfg(not(target_os = "windows"))]
        assert_eq!(utc_offset_minutes_at(0), None);
        // Out of the FILETIME range.
        assert_eq!(utc_offset_minutes_at(i64::MAX), None);
        assert_eq!(utc_offset_minutes_at(i64::MIN), None);
        assert_eq!(utc_offset_minutes_at(-FILETIME_EPOCH_OFFSET_MS - 1), None);
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
