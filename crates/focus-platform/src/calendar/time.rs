//! Civil calendar arithmetic (proleptic Gregorian, no time zones) and the date-time formats of the calendar APIs.
//!
//! "Wall" milliseconds are a local date and time encoded as if it were UTC (`days * MS_PER_DAY + time of day`);
//! a [`super::TimeZone`] turns them into real instants.

pub(crate) const MS_PER_SECOND: i64 = 1000;
pub(crate) const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;
pub(crate) const MS_PER_HOUR: i64 = 60 * MS_PER_MINUTE;
pub(crate) const MS_PER_DAY: i64 = 24 * MS_PER_HOUR;

/// Years accepted from calendar data.
const MIN_YEAR: i64 = 1;
const MAX_YEAR: i64 = 9999;

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's `days_from_civil`).
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `(year, month, day)` of a day number (inverse of [`days_from_civil`]).
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Day of the week: 0 = Monday … 6 = Sunday (1970-01-01 was a Thursday).
pub(crate) fn weekday(days: i64) -> u32 {
    (days + 3).rem_euclid(7) as u32
}

pub(crate) fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn valid_date(year: i64, month: u32, day: u32) -> bool {
    (MIN_YEAR..=MAX_YEAR).contains(&year)
        && (1..=12).contains(&month)
        && day >= 1
        && day <= days_in_month(year, month)
}

fn valid_time(hour: u32, minute: u32, second: u32) -> bool {
    // Second 60 = leap second; it is folded into 59 by the callers.
    hour < 24 && minute < 60 && second <= 60
}

fn wall_ms(days: i64, hour: u32, minute: u32, second: u32) -> i64 {
    days * MS_PER_DAY
        + i64::from(hour) * MS_PER_HOUR
        + i64::from(minute) * MS_PER_MINUTE
        + i64::from(second.min(59)) * MS_PER_SECOND
}

/// Parses an all-digit field.
fn digits(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// iCalendar DATE `YYYYMMDD` → day number.
pub(crate) fn parse_ical_date(text: &str) -> Option<i64> {
    if text.len() != 8 || !text.is_ascii() {
        return None;
    }
    let year = i64::from(digits(&text[0..4])?);
    let month = digits(&text[4..6])?;
    let day = digits(&text[6..8])?;
    valid_date(year, month, day).then(|| days_from_civil(year, month, day))
}

/// iCalendar DATE-TIME `YYYYMMDDTHHMMSS[Z]` → (wall ms, is UTC).
pub(crate) fn parse_ical_date_time(text: &str) -> Option<(i64, bool)> {
    if !text.is_ascii() {
        return None;
    }
    let (body, utc) = match text.strip_suffix(['Z', 'z']) {
        Some(body) => (body, true),
        None => (text, false),
    };
    if body.len() != 15 || !matches!(body.as_bytes()[8], b'T' | b't') {
        return None;
    }
    let days = parse_ical_date(&body[0..8])?;
    let hour = digits(&body[9..11])?;
    let minute = digits(&body[11..13])?;
    let second = digits(&body[13..15])?;
    valid_time(hour, minute, second).then(|| (wall_ms(days, hour, minute, second), utc))
}

/// ISO 8601 date `YYYY-MM-DD` → day number.
pub(crate) fn parse_iso_date(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if text.len() != 10 || !text.is_ascii() || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = i64::from(digits(&text[0..4])?);
    let month = digits(&text[5..7])?;
    let day = digits(&text[8..10])?;
    valid_date(year, month, day).then(|| days_from_civil(year, month, day))
}

/// ISO 8601 local date-time `YYYY-MM-DDTHH:MM[:SS[.fraction]]` (no offset) → wall ms. Graph sends
/// `2026-10-13T08:00:00.0000000`.
pub(crate) fn parse_iso_naive(text: &str) -> Option<i64> {
    if !text.is_ascii() || text.len() < 16 || !matches!(text.as_bytes()[10], b'T' | b't' | b' ') {
        return None;
    }
    let days = parse_iso_date(&text[0..10])?;
    let time = &text[11..];
    let (hm, rest) = time.split_at(5);
    let (hour, minute) = hm.split_once(':')?;
    let (hour, minute) = (digits(hour)?, digits(minute)?);
    let mut second = 0;
    let mut millis = 0;
    if !rest.is_empty() {
        let rest = rest.strip_prefix(':')?;
        let (sec, fraction) = match rest.split_once(['.', ',']) {
            Some((sec, fraction)) => (sec, Some(fraction)),
            None => (rest, None),
        };
        if sec.len() != 2 {
            return None;
        }
        second = digits(sec)?;
        if let Some(fraction) = fraction {
            digits(fraction)?;
            let padded = format!("{fraction:0<3}");
            millis = i64::from(digits(&padded[..3])?);
        }
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(wall_ms(days, hour, minute, second) + millis)
}

/// RFC 3339 date-time with offset (`2026-10-13T10:00:00+02:00`, `…Z`) → UTC ms.
pub(crate) fn parse_rfc3339(text: &str) -> Option<i64> {
    let text = text.trim();
    if !text.is_ascii() {
        return None;
    }
    if let Some(naive) = text.strip_suffix(['Z', 'z']) {
        return parse_iso_naive(naive);
    }
    // The offset is the last `+hh:mm` / `-hh:mm` (the date part also contains '-').
    if text.len() < 6 {
        return None;
    }
    let (naive, offset) = text.split_at(text.len() - 6);
    let sign = match offset.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (hours, minutes) = offset[1..].split_once(':')?;
    let (hours, minutes) = (digits(hours)?, digits(minutes)?);
    if hours > 23 || minutes > 59 {
        return None;
    }
    let offset_ms = sign * (i64::from(hours) * MS_PER_HOUR + i64::from(minutes) * MS_PER_MINUTE);
    Some(parse_iso_naive(naive)? - offset_ms)
}

/// UTC ms → `YYYY-MM-DDTHH:MM:SSZ` (what Graph and Google expect for time ranges).
pub fn format_utc(utc_ms: i64) -> String {
    let days = utc_ms.div_euclid(MS_PER_DAY);
    let time = utc_ms.rem_euclid(MS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let year = year.clamp(0, MAX_YEAR);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / MS_PER_HOUR,
        time % MS_PER_HOUR / MS_PER_MINUTE,
        time % MS_PER_MINUTE / MS_PER_SECOND
    )
}

/// iCalendar / ISO 8601 duration (`PT1H30M`, `P1D`, `-PT15M`, `P1W`, `P1DT2H`) → ms (days count as 24 h).
pub(crate) fn parse_duration(text: &str) -> Option<i64> {
    let text = text.trim();
    let (sign, rest) = match text.as_bytes().first()? {
        b'-' => (-1, &text[1..]),
        b'+' => (1, &text[1..]),
        _ => (1, text),
    };
    let rest = rest.strip_prefix(['P', 'p'])?;
    let mut total: i64 = 0;
    let mut number = String::new();
    let mut in_time = false;
    let mut any = false;
    for c in rest.chars() {
        match c.to_ascii_uppercase() {
            '0'..='9' => number.push(c),
            'T' if number.is_empty() && !in_time => in_time = true,
            unit => {
                let value: i64 = number.parse().ok()?;
                number.clear();
                let scale = match (unit, in_time) {
                    ('W', false) => 7 * MS_PER_DAY,
                    ('D', false) => MS_PER_DAY,
                    ('H', true) => MS_PER_HOUR,
                    ('M', true) => MS_PER_MINUTE,
                    ('S', true) => MS_PER_SECOND,
                    _ => return None,
                };
                total = total.checked_add(value.checked_mul(scale)?)?;
                any = true;
            }
        }
    }
    (any && number.is_empty()).then_some(sign * total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        for days in (-800_000..800_000).step_by(37) {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        }
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(civil_from_days(days_from_civil(1601, 1, 1)), (1601, 1, 1));
    }

    #[test]
    fn weekdays_and_months() {
        assert_eq!(weekday(0), 3, "1970-01-01 was a Thursday");
        assert_eq!(weekday(days_from_civil(2026, 10, 13)), 1, "Tuesday");
        assert_eq!(weekday(days_from_civil(2026, 10, 25)), 6, "Sunday");
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2026, 13), 0);
    }

    #[test]
    fn ical_values() {
        let day = days_from_civil(2026, 10, 13);
        assert_eq!(parse_ical_date("20261013"), Some(day));
        assert_eq!(parse_ical_date("20261332"), None);
        assert_eq!(parse_ical_date("2026101"), None);
        assert_eq!(
            parse_ical_date_time("20261013T083000Z"),
            Some((
                day * MS_PER_DAY + 8 * MS_PER_HOUR + 30 * MS_PER_MINUTE,
                true
            ))
        );
        assert_eq!(
            parse_ical_date_time("20261013T083005"),
            Some((
                day * MS_PER_DAY + 8 * MS_PER_HOUR + 30 * MS_PER_MINUTE + 5000,
                false
            ))
        );
        assert_eq!(parse_ical_date_time("20261013T246000"), None);
        assert_eq!(parse_ical_date_time("20261013"), None);
        assert_eq!(parse_ical_date_time("20261013T0830Z"), None);
        assert_eq!(parse_ical_date_time("2026101ąT083000"), None);
    }

    #[test]
    fn iso_values() {
        let day = days_from_civil(2026, 10, 13);
        let eight = day * MS_PER_DAY + 8 * MS_PER_HOUR;
        assert_eq!(parse_iso_naive("2026-10-13T08:00:00.0000000"), Some(eight));
        assert_eq!(parse_iso_naive("2026-10-13T08:00:00"), Some(eight));
        assert_eq!(parse_iso_naive("2026-10-13T08:00"), Some(eight));
        assert_eq!(parse_iso_naive("2026-10-13T08:00:00.5"), Some(eight + 500));
        assert_eq!(parse_iso_naive("2026-10-13T08:00:0"), None);
        assert_eq!(parse_iso_naive("2026-10-13"), None);
        assert_eq!(parse_rfc3339("2026-10-13T10:00:00+02:00"), Some(eight));
        assert_eq!(parse_rfc3339("2026-10-13T03:00:00-05:00"), Some(eight));
        assert_eq!(parse_rfc3339("2026-10-13T08:00:00Z"), Some(eight));
        assert_eq!(parse_rfc3339("2026-10-13T08:00:00.000Z"), Some(eight));
        assert_eq!(parse_rfc3339("2026-10-13T08:00:00"), None);
        assert_eq!(parse_iso_date("2026-10-13"), Some(day));
        assert_eq!(parse_iso_date("2026-02-30"), None);
    }

    #[test]
    fn formats_utc() {
        let day = days_from_civil(2026, 10, 13);
        assert_eq!(
            format_utc(day * MS_PER_DAY + 8 * MS_PER_HOUR + 5 * MS_PER_MINUTE + 7_999),
            "2026-10-13T08:05:07Z"
        );
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(-1), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("PT1H30M"), Some(90 * MS_PER_MINUTE));
        assert_eq!(parse_duration("P1D"), Some(MS_PER_DAY));
        assert_eq!(parse_duration("P1W"), Some(7 * MS_PER_DAY));
        assert_eq!(parse_duration("P1DT2H"), Some(MS_PER_DAY + 2 * MS_PER_HOUR));
        assert_eq!(parse_duration("-PT15M"), Some(-15 * MS_PER_MINUTE));
        assert_eq!(parse_duration("PT45S"), Some(45 * MS_PER_SECOND));
        assert_eq!(parse_duration("P"), None);
        assert_eq!(parse_duration("PT"), None);
        assert_eq!(parse_duration("P1H"), None);
        assert_eq!(parse_duration("PT1"), None);
        assert_eq!(parse_duration("1H"), None);
    }
}
