//! Time zones for calendar times.
//!
//! * iCalendar `VTIMEZONE` definitions (STANDARD / DAYLIGHT observances with their own yearly RRULEs or RDATEs),
//!   built by the ICS parser through [`TimeZone::from_observances`].
//! * Zone names ([`TimeZone::by_name`]): on Windows the zone's rules from the registry (`Time Zones\<name>\TZI`,
//!   every Windows zone name that Outlook / Exchange write), then a small built-in table of common Windows and
//!   IANA names (EU, US, AU and NZ daylight saving rules), `Etc/GMT±N` and `UTC±hh:mm`.
//! * The system zone ([`TimeZone::local`]) for floating times: Windows registry
//!   (`TimeZoneInformation\TimeZoneKeyName`), elsewhere `TZ` / `/etc/localtime`; UTC when unknown.

use super::rrule::{Freq, Rule};
use super::time::{MS_PER_DAY, MS_PER_HOUR, MS_PER_MINUTE, civil_from_days, days_from_civil};

/// Longest zone name looked up.
const MAX_NAME_LEN: usize = 128;
/// Largest UTC offset accepted, in minutes (UTC+14 / UTC-12 exist; allow some slack).
const MAX_OFFSET_MINUTES: i32 = 18 * 60;

/// UTC offset as a function of the instant. Offsets are minutes east of UTC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeZone {
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Fixed(i32),
    Rules(Vec<Observance>),
}

/// One STANDARD / DAYLIGHT block: from each of its onsets on, the offset is `offset_to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Observance {
    pub(crate) offset_from: i32,
    pub(crate) offset_to: i32,
    /// DTSTART: the first onset, local wall time in `offset_from`.
    pub(crate) start_wall_ms: i64,
    /// Yearly rule of later onsets (time of day from `start_wall_ms`).
    pub(crate) rule: Option<Rule>,
    /// Extra onsets (RDATE), local wall time in `offset_from`.
    pub(crate) rdates_wall_ms: Vec<i64>,
}

impl Observance {
    /// The latest onset at or before `utc_ms`.
    fn latest_onset(&self, utc_ms: i64) -> Option<i64> {
        let from = i64::from(self.offset_from) * MS_PER_MINUTE;
        let first = self.start_wall_ms - from;
        let mut best = None;
        let mut consider = |onset: i64| {
            if onset <= utc_ms && best.is_none_or(|b| onset > b) {
                best = Some(onset);
            }
        };
        consider(first);
        for &wall in &self.rdates_wall_ms {
            consider(wall - from);
        }
        if let Some(rule) = self.rule.as_ref().filter(|r| r.freq == Freq::Yearly)
            && first <= utc_ms
        {
            let (_, start_month, start_day) =
                civil_from_days(self.start_wall_ms.div_euclid(MS_PER_DAY));
            let time_of_day = self.start_wall_ms.rem_euclid(MS_PER_DAY);
            let (year, _, _) = civil_from_days(utc_ms.div_euclid(MS_PER_DAY));
            for y in year - 1..=year + 1 {
                for day in rule.yearly_days(y, start_month, start_day) {
                    let wall = day * MS_PER_DAY + time_of_day;
                    let onset = wall - from;
                    if onset >= first && !rule.is_after_until(wall, onset) {
                        consider(onset);
                    }
                }
            }
        }
        best
    }

    /// A yearly observance on the `ordinal` `weekday` of `month` at `time_of_day_ms` local time.
    fn yearly(
        from: i32,
        to: i32,
        month: u32,
        ordinal: i32,
        weekday: u32,
        time_of_day_ms: i64,
    ) -> Self {
        Self {
            offset_from: from,
            offset_to: to,
            start_wall_ms: days_from_civil(1601, 1, 1) * MS_PER_DAY + time_of_day_ms,
            rule: Some(Rule::yearly_nth(month, ordinal, weekday)),
            rdates_wall_ms: Vec::new(),
        }
    }
}

impl TimeZone {
    pub fn utc() -> Self {
        Self::fixed(0)
    }

    /// A constant offset (minutes east of UTC).
    pub fn fixed(minutes_east: i32) -> Self {
        Self {
            kind: Kind::Fixed(minutes_east.clamp(-MAX_OFFSET_MINUTES, MAX_OFFSET_MINUTES)),
        }
    }

    /// The zone of a `VTIMEZONE` (`None` without observances).
    pub(crate) fn from_observances(observances: Vec<Observance>) -> Option<Self> {
        if observances.is_empty() {
            return None;
        }
        let fixed = observances[0].offset_to;
        if observances
            .iter()
            .all(|o| o.offset_to == fixed && o.offset_from == fixed)
        {
            return Some(Self::fixed(fixed));
        }
        Some(Self {
            kind: Kind::Rules(observances),
        })
    }

    /// The system time zone (for floating times); UTC when it cannot be determined.
    pub fn local() -> Self {
        platform::local_zone().unwrap_or_else(Self::utc)
    }

    /// A zone by name: Windows zone names (`W. Europe Standard Time`), IANA names (`Europe/Warsaw`), `Etc/GMT-1`,
    /// `UTC+01:00`. `None` when unknown.
    pub fn by_name(name: &str) -> Option<Self> {
        let name = name.trim().trim_matches('"').trim();
        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return None;
        }
        let bare = name.trim_start_matches('/');
        if let Some(zone) = platform::zone_by_windows_name(bare) {
            return Some(zone);
        }
        if let Some(zone) = table_zone(bare).or_else(|| fixed_offset_name(bare)) {
            return Some(zone);
        }
        // Prefixed IANA names: `/mozilla.org/20050126_1/Europe/Warsaw`, `/citadel.org/…/Europe/Warsaw`.
        let parts: Vec<&str> = bare.split('/').filter(|p| !p.is_empty()).collect();
        for keep in [3, 2] {
            if parts.len() > keep {
                let suffix = parts[parts.len() - keep..].join("/");
                if let Some(zone) = table_zone(&suffix) {
                    return Some(zone);
                }
            }
        }
        None
    }

    /// Offset at the instant, in minutes east of UTC.
    pub fn offset_minutes_at(&self, utc_ms: i64) -> i32 {
        match &self.kind {
            Kind::Fixed(minutes) => *minutes,
            Kind::Rules(observances) => {
                let mut best: Option<(i64, i32)> = None;
                for observance in observances {
                    if let Some(onset) = observance.latest_onset(utc_ms)
                        && best.is_none_or(|(b, _)| onset > b)
                    {
                        best = Some((onset, observance.offset_to));
                    }
                }
                match best {
                    Some((_, offset)) => offset,
                    // Before the first onset the zone was in the earliest observance's "from" offset.
                    None => observances
                        .iter()
                        .min_by_key(|o| o.start_wall_ms - i64::from(o.offset_from) * MS_PER_MINUTE)
                        .map_or(0, |o| o.offset_from),
                }
            }
        }
    }

    fn offset_ms_at(&self, utc_ms: i64) -> i64 {
        i64::from(self.offset_minutes_at(utc_ms)) * MS_PER_MINUTE
    }

    /// Local wall time → instant. A time that occurs twice (end of daylight saving time) is the first one; a time
    /// that does not exist (start of daylight saving time) uses the offset before the gap (RFC 5545).
    pub fn wall_to_utc(&self, wall_ms: i64) -> i64 {
        if let Kind::Fixed(minutes) = self.kind {
            return wall_ms - i64::from(minutes) * MS_PER_MINUTE;
        }
        // Offsets just before and after any transition near `wall_ms` (transitions are months apart).
        let before = self.offset_ms_at(wall_ms - MS_PER_DAY);
        let after = self.offset_ms_at(wall_ms + MS_PER_DAY);
        let fits = |offset: i64| {
            let utc = wall_ms - offset;
            (self.offset_ms_at(utc) == offset).then_some(utc)
        };
        match (fits(before), fits(after)) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => wall_ms - before,
        }
    }

    /// Instant → local wall time.
    pub fn utc_to_wall(&self, utc_ms: i64) -> i64 {
        utc_ms + self.offset_ms_at(utc_ms)
    }
}

/// Daylight saving time rules of the built-in table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dst {
    None,
    /// EU: last Sunday of March 01:00 UTC → last Sunday of October 01:00 UTC.
    Eu,
    /// US / Canada: second Sunday of March 02:00 → first Sunday of November 02:00 (local).
    Us,
    /// South-eastern Australia: first Sunday of October 02:00 → first Sunday of April 03:00 (local).
    AuSouth,
    /// New Zealand: last Sunday of September 02:00 → first Sunday of April 03:00 (local).
    Nz,
}

struct Entry {
    names: &'static [&'static str],
    /// Standard offset, minutes east of UTC.
    std: i32,
    dst: Dst,
}

const SUNDAY: u32 = 6;

/// Common zones (Windows names first, then IANA names). On Windows the registry wins for Windows names.
const TABLE: &[Entry] = &[
    Entry {
        names: &[
            "UTC",
            "Coordinated Universal Time",
            "Etc/UTC",
            "Etc/GMT",
            "Etc/Universal",
            "Etc/Zulu",
            "GMT",
            "Z",
            "Universal",
            "Zulu",
        ],
        std: 0,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "GMT Standard Time",
            "Europe/London",
            "Europe/Dublin",
            "Europe/Lisbon",
            "Atlantic/Canary",
            "Atlantic/Madeira",
            "Atlantic/Faroe",
        ],
        std: 0,
        dst: Dst::Eu,
    },
    Entry {
        names: &[
            "Greenwich Standard Time",
            "Atlantic/Reykjavik",
            "Africa/Abidjan",
            "Africa/Accra",
            "Africa/Dakar",
        ],
        std: 0,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "W. Europe Standard Time",
            "Central Europe Standard Time",
            "Central European Standard Time",
            "Romance Standard Time",
            "Europe/Warsaw",
            "Europe/Berlin",
            "Europe/Amsterdam",
            "Europe/Brussels",
            "Europe/Paris",
            "Europe/Madrid",
            "Europe/Rome",
            "Europe/Vienna",
            "Europe/Zurich",
            "Europe/Stockholm",
            "Europe/Oslo",
            "Europe/Copenhagen",
            "Europe/Prague",
            "Europe/Bratislava",
            "Europe/Budapest",
            "Europe/Ljubljana",
            "Europe/Zagreb",
            "Europe/Sarajevo",
            "Europe/Skopje",
            "Europe/Belgrade",
            "Europe/Podgorica",
            "Europe/Tirane",
            "Europe/Luxembourg",
            "Europe/Monaco",
            "Europe/Malta",
            "Europe/Andorra",
            "Europe/Vaduz",
            "Europe/San_Marino",
            "Europe/Vatican",
            "Europe/Gibraltar",
            "Europe/Busingen",
            "Africa/Ceuta",
            "Arctic/Longyearbyen",
            "CET",
        ],
        std: 60,
        dst: Dst::Eu,
    },
    Entry {
        names: &[
            "W. Central Africa Standard Time",
            "Africa/Lagos",
            "Africa/Algiers",
            "Africa/Tunis",
        ],
        std: 60,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "GTB Standard Time",
            "FLE Standard Time",
            "E. Europe Standard Time",
            "Europe/Athens",
            "Europe/Bucharest",
            "Europe/Helsinki",
            "Europe/Kiev",
            "Europe/Kyiv",
            "Europe/Riga",
            "Europe/Sofia",
            "Europe/Tallinn",
            "Europe/Vilnius",
            "Europe/Chisinau",
            "Europe/Mariehamn",
            "Asia/Nicosia",
            "Europe/Nicosia",
            "EET",
        ],
        std: 120,
        dst: Dst::Eu,
    },
    Entry {
        names: &[
            "South Africa Standard Time",
            "Kaliningrad Standard Time",
            "Africa/Johannesburg",
            "Africa/Harare",
            "Africa/Maputo",
            "Europe/Kaliningrad",
        ],
        std: 120,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Russian Standard Time",
            "Belarus Standard Time",
            "Turkey Standard Time",
            "Arab Standard Time",
            "Arabic Standard Time",
            "E. Africa Standard Time",
            "Europe/Moscow",
            "Europe/Minsk",
            "Europe/Istanbul",
            "Europe/Simferopol",
            "Asia/Riyadh",
            "Asia/Qatar",
            "Asia/Kuwait",
            "Asia/Baghdad",
            "Africa/Nairobi",
        ],
        std: 180,
        dst: Dst::None,
    },
    Entry {
        names: &["Arabian Standard Time", "Asia/Dubai", "Asia/Muscat"],
        std: 240,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Pakistan Standard Time",
            "West Asia Standard Time",
            "Asia/Karachi",
            "Asia/Tashkent",
        ],
        std: 300,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "India Standard Time",
            "Sri Lanka Standard Time",
            "Asia/Kolkata",
            "Asia/Calcutta",
            "Asia/Colombo",
        ],
        std: 330,
        dst: Dst::None,
    },
    Entry {
        names: &["Nepal Standard Time", "Asia/Kathmandu", "Asia/Katmandu"],
        std: 345,
        dst: Dst::None,
    },
    Entry {
        names: &["Bangladesh Standard Time", "Asia/Dhaka"],
        std: 360,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "SE Asia Standard Time",
            "Asia/Bangkok",
            "Asia/Jakarta",
            "Asia/Ho_Chi_Minh",
            "Asia/Saigon",
        ],
        std: 420,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "China Standard Time",
            "Singapore Standard Time",
            "Taipei Standard Time",
            "W. Australia Standard Time",
            "Asia/Shanghai",
            "Asia/Hong_Kong",
            "Asia/Singapore",
            "Asia/Kuala_Lumpur",
            "Asia/Manila",
            "Asia/Taipei",
            "Australia/Perth",
        ],
        std: 480,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Tokyo Standard Time",
            "Korea Standard Time",
            "Asia/Tokyo",
            "Asia/Seoul",
        ],
        std: 540,
        dst: Dst::None,
    },
    Entry {
        names: &["AUS Central Standard Time", "Australia/Darwin"],
        std: 570,
        dst: Dst::None,
    },
    Entry {
        names: &["Cen. Australia Standard Time", "Australia/Adelaide"],
        std: 570,
        dst: Dst::AuSouth,
    },
    Entry {
        names: &["E. Australia Standard Time", "Australia/Brisbane"],
        std: 600,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "AUS Eastern Standard Time",
            "Tasmania Standard Time",
            "Australia/Sydney",
            "Australia/Melbourne",
            "Australia/Canberra",
            "Australia/Hobart",
        ],
        std: 600,
        dst: Dst::AuSouth,
    },
    Entry {
        names: &["New Zealand Standard Time", "Pacific/Auckland"],
        std: 720,
        dst: Dst::Nz,
    },
    Entry {
        names: &["Newfoundland Standard Time", "America/St_Johns"],
        std: -210,
        dst: Dst::Us,
    },
    Entry {
        names: &["Atlantic Standard Time", "America/Halifax"],
        std: -240,
        dst: Dst::Us,
    },
    Entry {
        names: &[
            "SA Western Standard Time",
            "Venezuela Standard Time",
            "America/La_Paz",
            "America/Caracas",
            "America/Puerto_Rico",
        ],
        std: -240,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "E. South America Standard Time",
            "Argentina Standard Time",
            "America/Sao_Paulo",
            "America/Buenos_Aires",
            "America/Argentina/Buenos_Aires",
            "America/Montevideo",
        ],
        std: -180,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Eastern Standard Time",
            "US Eastern Standard Time",
            "America/New_York",
            "America/Toronto",
            "America/Detroit",
            "America/Indiana/Indianapolis",
            "America/Indianapolis",
            "America/Nassau",
            "US/Eastern",
            "EST5EDT",
        ],
        std: -300,
        dst: Dst::Us,
    },
    Entry {
        names: &[
            "SA Pacific Standard Time",
            "America/Bogota",
            "America/Lima",
            "America/Panama",
        ],
        std: -300,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Central Standard Time",
            "America/Chicago",
            "America/Winnipeg",
            "US/Central",
            "CST6CDT",
        ],
        std: -360,
        dst: Dst::Us,
    },
    Entry {
        names: &[
            "Central Standard Time (Mexico)",
            "Canada Central Standard Time",
            "Central America Standard Time",
            "America/Mexico_City",
            "America/Regina",
            "America/Guatemala",
            "America/Costa_Rica",
        ],
        std: -360,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Mountain Standard Time",
            "America/Denver",
            "America/Edmonton",
            "America/Boise",
            "US/Mountain",
            "MST7MDT",
        ],
        std: -420,
        dst: Dst::Us,
    },
    Entry {
        names: &["US Mountain Standard Time", "America/Phoenix"],
        std: -420,
        dst: Dst::None,
    },
    Entry {
        names: &[
            "Pacific Standard Time",
            "America/Los_Angeles",
            "America/Vancouver",
            "America/Tijuana",
            "US/Pacific",
            "PST8PDT",
        ],
        std: -480,
        dst: Dst::Us,
    },
    Entry {
        names: &["Alaskan Standard Time", "America/Anchorage"],
        std: -540,
        dst: Dst::Us,
    },
    Entry {
        names: &["Hawaiian Standard Time", "Pacific/Honolulu"],
        std: -600,
        dst: Dst::None,
    },
];

fn table_zone(name: &str) -> Option<TimeZone> {
    TABLE
        .iter()
        .find(|entry| entry.names.iter().any(|n| n.eq_ignore_ascii_case(name)))
        .map(|entry| rule_zone(entry.std, entry.dst))
}

fn rule_zone(std: i32, dst: Dst) -> TimeZone {
    // (month, ordinal Sunday, local minutes) of the switch to daylight time and back to standard time.
    let (to_daylight, to_standard) = match dst {
        Dst::None => return TimeZone::fixed(std),
        Dst::Eu => ((3, -1, 60 + std), (10, -1, 120 + std)),
        Dst::Us => ((3, 2, 120), (11, 1, 120)),
        Dst::AuSouth => ((10, 1, 120), (4, 1, 180)),
        Dst::Nz => ((9, -1, 120), (4, 1, 180)),
    };
    let daylight = std + 60;
    let observance = |from: i32, to: i32, (month, ordinal, minutes): (u32, i32, i32)| {
        Observance::yearly(
            from,
            to,
            month,
            ordinal,
            SUNDAY,
            i64::from(minutes) * MS_PER_MINUTE,
        )
    };
    TimeZone {
        kind: Kind::Rules(vec![
            observance(std, daylight, to_daylight),
            observance(daylight, std, to_standard),
        ]),
    }
}

/// `Etc/GMT+5` (= UTC-5, POSIX sign), `UTC+01:00`, `GMT-0300`, `(UTC+01:00) Warsaw…` → fixed offset.
fn fixed_offset_name(name: &str) -> Option<TimeZone> {
    if let Some(rest) = strip_prefix_ci(name, "Etc/GMT") {
        let hours: i32 = rest.trim_start_matches('+').parse().ok()?;
        return (-14..=12)
            .contains(&hours)
            .then(|| TimeZone::fixed(-hours * 60));
    }
    let text = name.strip_prefix('(').unwrap_or(name);
    let rest = strip_prefix_ci(text, "UTC").or_else(|| strip_prefix_ci(text, "GMT"))?;
    let rest = rest.split([')', ' ']).next().unwrap_or("");
    if rest.is_empty() {
        return None;
    }
    let sign = match rest.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits = rest[1..].replace(':', "");
    if digits.is_empty() || digits.len() > 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (hours, minutes) = if digits.len() <= 2 {
        (digits.parse::<i32>().ok()?, 0)
    } else {
        let split = digits.len() - 2;
        (
            digits[..split].parse::<i32>().ok()?,
            digits[split..].parse::<i32>().ok()?,
        )
    };
    (hours <= 14 && minutes < 60).then(|| TimeZone::fixed(sign * (hours * 60 + minutes)))
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// Windows `REG_TZI_FORMAT` (`Bias`, `StandardBias`, `DaylightBias`, `StandardDate`, `DaylightDate`; biases in
/// minutes west of UTC, dates as `SYSTEMTIME` "n-th weekday of month" rules) → zone.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn zone_from_tzi(bytes: &[u8], daylight_disabled: bool) -> Option<TimeZone> {
    if bytes.len() < 44 {
        return None;
    }
    let long =
        |at: usize| i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let word = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let bias = long(0);
    let standard_bias = long(4);
    let daylight_bias = long(8);
    let std = -(bias.checked_add(standard_bias)?);
    let daylight = -(bias.checked_add(daylight_bias)?);
    if std.abs() > MAX_OFFSET_MINUTES || daylight.abs() > MAX_OFFSET_MINUTES {
        return None;
    }
    // SYSTEMTIME: wYear, wMonth, wDayOfWeek (0 = Sunday), wDay (1-5, 5 = last), wHour, wMinute, wSecond, wMs.
    let transition = |at: usize| -> Option<(u32, i32, u32, i64)> {
        let (year, month, day_of_week, day) = (word(at), word(at + 2), word(at + 4), word(at + 6));
        if year != 0 || !(1..=12).contains(&month) || day_of_week > 6 || !(1..=5).contains(&day) {
            return None;
        }
        let ordinal = if day == 5 { -1 } else { i32::from(day) };
        let weekday = (u32::from(day_of_week) + 6) % 7;
        let time = i64::from(word(at + 8)) * MS_PER_HOUR
            + i64::from(word(at + 10)) * MS_PER_MINUTE
            + i64::from(word(at + 12)) * 1000
            + i64::from(word(at + 14));
        Some((u32::from(month), ordinal, weekday, time.min(MS_PER_DAY - 1)))
    };
    if daylight_disabled || std == daylight {
        return Some(TimeZone::fixed(std));
    }
    match (transition(12), transition(28)) {
        (Some(to_standard), Some(to_daylight)) => {
            let (m, o, w, t) = to_daylight;
            let (m2, o2, w2, t2) = to_standard;
            Some(TimeZone {
                kind: Kind::Rules(vec![
                    Observance::yearly(std, daylight, m, o, w, t),
                    Observance::yearly(daylight, std, m2, o2, w2, t2),
                ]),
            })
        }
        // No (or a one-off) daylight saving rule: standard time all year.
        _ => Some(TimeZone::fixed(std)),
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::c_void;

    use windows::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
    };
    use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};
    use windows::core::{PCWSTR, w};

    use super::{MAX_NAME_LEN, TimeZone, zone_from_tzi};
    use crate::calendar::time::{MS_PER_DAY, MS_PER_MINUTE, days_from_civil};

    const TIME_ZONES: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Time Zones";
    const TIME_ZONE_INFORMATION: &str = r"SYSTEM\CurrentControlSet\Control\TimeZoneInformation";

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn read_binary(subkey: &str, value: PCWSTR) -> Option<Vec<u8>> {
        let subkey = wide(subkey);
        let mut buffer = vec![0u8; 256];
        let mut size = buffer.len() as u32;
        // SAFETY: `subkey` is NUL-terminated and outlives the call; `buffer` holds `size` bytes; `size` is a valid
        // in/out pointer. RRF_RT_REG_BINARY makes the call fail for any other value type.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey.as_ptr()),
                value,
                RRF_RT_REG_BINARY,
                None,
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                Some(&mut size),
            )
        };
        status.ok().ok()?;
        buffer.truncate(usize::try_from(size).unwrap_or(0).min(256));
        Some(buffer)
    }

    fn read_string(subkey: &str, value: PCWSTR) -> Option<String> {
        let subkey = wide(subkey);
        let mut buffer = [0u16; 256];
        let mut size = size_of_val(&buffer) as u32;
        // SAFETY: `subkey` is NUL-terminated; `buffer` holds `size` bytes; RRF_RT_REG_SZ NUL-terminates the result.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey.as_ptr()),
                value,
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                Some(&mut size),
            )
        };
        status.ok().ok()?;
        let len = buffer.iter().position(|&unit| unit == 0)?;
        Some(String::from_utf16_lossy(&buffer[..len]))
    }

    fn read_dword(subkey: &str, value: PCWSTR) -> Option<u32> {
        let subkey = wide(subkey);
        let mut data = 0u32;
        let mut size = size_of::<u32>() as u32;
        // SAFETY: `subkey` is NUL-terminated; `data` is a valid 4-byte out buffer; RRF_RT_REG_DWORD checks the type.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey.as_ptr()),
                value,
                RRF_RT_REG_DWORD,
                None,
                Some((&mut data as *mut u32).cast::<c_void>()),
                Some(&mut size),
            )
        };
        status.ok().ok()?;
        Some(data)
    }

    /// Rules of a Windows zone name from the registry.
    pub(super) fn zone_by_windows_name(name: &str) -> Option<TimeZone> {
        if name.is_empty()
            || name.len() > MAX_NAME_LEN
            || name.contains(['\\', '/'])
            || name.chars().any(char::is_control)
        {
            return None;
        }
        let bytes = read_binary(&format!(r"{TIME_ZONES}\{name}"), w!("TZI"))?;
        zone_from_tzi(&bytes, false)
    }

    pub(super) fn local_zone() -> Option<TimeZone> {
        let disabled = read_dword(TIME_ZONE_INFORMATION, w!("DynamicDaylightTimeDisabled"))
            .is_some_and(|value| value != 0);
        if let Some(name) = read_string(TIME_ZONE_INFORMATION, w!("TimeZoneKeyName"))
            .filter(|n| !n.trim().is_empty())
            && let Some(bytes) = read_binary(&format!(r"{TIME_ZONES}\{}", name.trim()), w!("TZI"))
            && let Some(zone) = zone_from_tzi(&bytes, disabled)
        {
            return Some(zone);
        }
        // Fallback: the current offset (correct until the next daylight saving time change).
        // SAFETY: no preconditions; the functions only fill the returned SYSTEMTIME.
        let (local, utc) = unsafe { (GetLocalTime(), GetSystemTime()) };
        let to_ms = |t: windows::Win32::Foundation::SYSTEMTIME| {
            days_from_civil(i64::from(t.wYear), u32::from(t.wMonth), u32::from(t.wDay)) * MS_PER_DAY
                + i64::from(t.wHour) * 3_600_000
                + i64::from(t.wMinute) * MS_PER_MINUTE
        };
        let minutes = (to_ms(local) - to_ms(utc)) / MS_PER_MINUTE;
        // Offsets are whole quarter hours.
        let rounded = ((minutes as f64 / 15.0).round() as i32) * 15;
        Some(TimeZone::fixed(rounded))
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::TimeZone;

    pub(super) fn zone_by_windows_name(_name: &str) -> Option<TimeZone> {
        None
    }

    /// `TZ`, or the zone name `/etc/localtime` links to (`…/zoneinfo/Europe/Warsaw`).
    pub(super) fn local_zone() -> Option<TimeZone> {
        if let Ok(tz) = std::env::var("TZ")
            && let Some(zone) = TimeZone::by_name(tz.trim_start_matches(':'))
        {
            return Some(zone);
        }
        let target = std::fs::read_link("/etc/localtime").ok()?;
        let text = target.to_string_lossy();
        let name = text.split("zoneinfo/").nth(1)?;
        TimeZone::by_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i64, m: u32, d: u32, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * MS_PER_DAY + h * MS_PER_HOUR + min * MS_PER_MINUTE
    }

    fn eu_zone() -> TimeZone {
        table_zone("Europe/Warsaw").expect("table zone")
    }

    #[test]
    fn eu_rules() {
        let zone = eu_zone();
        assert_eq!(zone.offset_minutes_at(utc(2026, 1, 15, 12, 0)), 60);
        assert_eq!(zone.offset_minutes_at(utc(2026, 7, 15, 12, 0)), 120);
        // 2026: daylight time from 29 March 01:00 UTC to 25 October 01:00 UTC.
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 29, 0, 59)), 60);
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 29, 1, 0)), 120);
        assert_eq!(zone.offset_minutes_at(utc(2026, 10, 25, 0, 59)), 120);
        assert_eq!(zone.offset_minutes_at(utc(2026, 10, 25, 1, 0)), 60);
        assert_eq!(zone.offset_minutes_at(utc(2031, 12, 31, 23, 30)), 60);

        let london = table_zone("europe/london").expect("case-insensitive");
        assert_eq!(london.offset_minutes_at(utc(2026, 3, 29, 1, 0)), 60);
        assert_eq!(london.offset_minutes_at(utc(2026, 3, 29, 0, 59)), 0);
        let helsinki = table_zone("FLE Standard Time").expect("windows name");
        assert_eq!(helsinki.offset_minutes_at(utc(2026, 10, 25, 0, 59)), 180);
        assert_eq!(helsinki.offset_minutes_at(utc(2026, 10, 25, 1, 0)), 120);
    }

    #[test]
    fn other_hemispheres_and_us() {
        let ny = table_zone("Eastern Standard Time").expect("zone");
        // 2026: 8 March 02:00 EST (07:00 UTC) → 1 November 02:00 EDT (06:00 UTC).
        assert_eq!(ny.offset_minutes_at(utc(2026, 3, 8, 6, 59)), -300);
        assert_eq!(ny.offset_minutes_at(utc(2026, 3, 8, 7, 0)), -240);
        assert_eq!(ny.offset_minutes_at(utc(2026, 11, 1, 5, 59)), -240);
        assert_eq!(ny.offset_minutes_at(utc(2026, 11, 1, 6, 0)), -300);
        let sydney = table_zone("Australia/Sydney").expect("zone");
        assert_eq!(sydney.offset_minutes_at(utc(2026, 1, 15, 0, 0)), 660);
        assert_eq!(sydney.offset_minutes_at(utc(2026, 7, 15, 0, 0)), 600);
        let auckland = table_zone("Pacific/Auckland").expect("zone");
        assert_eq!(auckland.offset_minutes_at(utc(2026, 1, 15, 0, 0)), 780);
        assert_eq!(auckland.offset_minutes_at(utc(2026, 7, 15, 0, 0)), 720);
        let india = table_zone("India Standard Time").expect("zone");
        assert_eq!(india.offset_minutes_at(utc(2026, 7, 15, 0, 0)), 330);
    }

    #[test]
    fn wall_time_conversion_handles_gaps_and_overlaps() {
        let zone = eu_zone();
        assert_eq!(
            zone.wall_to_utc(utc(2026, 7, 1, 10, 0)),
            utc(2026, 7, 1, 8, 0)
        );
        assert_eq!(
            zone.wall_to_utc(utc(2026, 12, 1, 10, 0)),
            utc(2026, 12, 1, 9, 0)
        );
        // 02:30 does not exist on 29 March: the offset before the gap (+1) applies.
        assert_eq!(
            zone.wall_to_utc(utc(2026, 3, 29, 2, 30)),
            utc(2026, 3, 29, 1, 30)
        );
        // 02:30 happens twice on 25 October: the first one (+2).
        assert_eq!(
            zone.wall_to_utc(utc(2026, 10, 25, 2, 30)),
            utc(2026, 10, 25, 0, 30)
        );
        assert_eq!(
            zone.wall_to_utc(utc(2026, 10, 25, 3, 30)),
            utc(2026, 10, 25, 2, 30)
        );
        assert_eq!(
            zone.utc_to_wall(utc(2026, 7, 1, 8, 0)),
            utc(2026, 7, 1, 10, 0)
        );
        let fixed = TimeZone::fixed(-300);
        assert_eq!(
            fixed.wall_to_utc(utc(2026, 7, 1, 10, 0)),
            utc(2026, 7, 1, 15, 0)
        );
    }

    #[test]
    fn names_resolve() {
        let july = utc(2026, 7, 1, 12, 0);
        let offset = |name: &str| TimeZone::by_name(name).map(|z| z.offset_minutes_at(july));
        assert_eq!(offset("Europe/Warsaw"), Some(120));
        assert_eq!(offset("\"Europe/Warsaw\""), Some(120));
        assert_eq!(offset("/mozilla.org/20050126_1/Europe/Warsaw"), Some(120));
        assert_eq!(
            offset("/citadel.org/20190914_1/America/New_York"),
            Some(-240)
        );
        assert_eq!(offset("America/Argentina/Buenos_Aires"), Some(-180));
        assert_eq!(offset("Central European Standard Time"), Some(120));
        assert_eq!(offset("W. Europe Standard Time"), Some(120));
        assert_eq!(offset("Pacific Standard Time"), Some(-420));
        assert_eq!(offset("UTC"), Some(0));
        assert_eq!(offset("Etc/GMT+5"), Some(-300));
        assert_eq!(offset("Etc/GMT-3"), Some(180));
        assert_eq!(offset("UTC+01:00"), Some(60));
        assert_eq!(
            offset("(UTC+05:30) Chennai, Kolkata, Mumbai, New Delhi"),
            Some(330)
        );
        assert_eq!(offset("GMT-0300"), Some(-180));
        assert_eq!(offset("Mars/Olympus_Mons"), None);
        assert_eq!(offset(""), None);
        assert_eq!(offset("UTC+99"), None);
    }

    #[test]
    fn vtimezone_style_observances() {
        // Outlook writes DTSTART 1601-01-01 with the real month in the RRULE.
        let zone = TimeZone::from_observances(vec![
            Observance {
                offset_from: 120,
                offset_to: 60,
                start_wall_ms: utc(1601, 1, 1, 3, 0),
                rule: Rule::parse("FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=10"),
                rdates_wall_ms: Vec::new(),
            },
            Observance {
                offset_from: 60,
                offset_to: 120,
                start_wall_ms: utc(1601, 1, 1, 2, 0),
                rule: Rule::parse("FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=3"),
                rdates_wall_ms: Vec::new(),
            },
        ])
        .expect("zone");
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 29, 0, 59)), 60);
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 29, 1, 0)), 120);
        assert_eq!(zone.offset_minutes_at(utc(2026, 10, 25, 1, 0)), 60);

        // A rule that ended (UNTIL) is not applied afterwards; an RDATE-only observance applies once.
        let zone = TimeZone::from_observances(vec![
            Observance {
                offset_from: -240,
                offset_to: -300,
                start_wall_ms: utc(1970, 1, 1, 0, 0),
                rule: None,
                rdates_wall_ms: Vec::new(),
            },
            Observance {
                offset_from: -300,
                offset_to: -240,
                start_wall_ms: utc(1987, 4, 5, 2, 0),
                rule: Rule::parse("FREQ=YEARLY;BYMONTH=4;BYDAY=1SU;UNTIL=20060402T070000Z"),
                rdates_wall_ms: Vec::new(),
            },
            Observance {
                offset_from: -240,
                offset_to: -300,
                start_wall_ms: utc(1987, 10, 25, 2, 0),
                rule: Rule::parse("FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU"),
                rdates_wall_ms: Vec::new(),
            },
            Observance {
                offset_from: -300,
                offset_to: -240,
                start_wall_ms: utc(2007, 3, 11, 2, 0),
                rule: None,
                rdates_wall_ms: vec![utc(2008, 3, 9, 2, 0)],
            },
        ])
        .expect("zone");
        assert_eq!(zone.offset_minutes_at(utc(2005, 4, 10, 12, 0)), -240);
        assert_eq!(
            zone.offset_minutes_at(utc(2007, 4, 10, 12, 0)),
            -240,
            "DTSTART"
        );
        assert_eq!(
            zone.offset_minutes_at(utc(2008, 4, 10, 12, 0)),
            -240,
            "RDATE"
        );
        assert_eq!(
            zone.offset_minutes_at(utc(2009, 4, 10, 12, 0)),
            -300,
            "the 1987 rule ended in 2006"
        );

        let fixed = TimeZone::from_observances(vec![Observance {
            offset_from: 330,
            offset_to: 330,
            start_wall_ms: utc(1970, 1, 1, 0, 0),
            rule: None,
            rdates_wall_ms: Vec::new(),
        }])
        .expect("zone");
        assert_eq!(fixed, TimeZone::fixed(330));
        assert_eq!(TimeZone::from_observances(Vec::new()), None);
    }

    fn tzi(
        bias: i32,
        std_bias: i32,
        dst_bias: i32,
        std_date: [u16; 8],
        dst_date: [u16; 8],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [bias, std_bias, dst_bias] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for word in std_date.into_iter().chain(dst_date) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn windows_tzi_rules() {
        // W. Europe Standard Time: back to standard on the last Sunday of October 03:00, daylight on the last
        // Sunday of March 02:00.
        let bytes = tzi(
            -60,
            0,
            -60,
            [0, 10, 0, 5, 3, 0, 0, 0],
            [0, 3, 0, 5, 2, 0, 0, 0],
        );
        let zone = zone_from_tzi(&bytes, false).expect("zone");
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 29, 0, 59)), 60);
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 29, 1, 0)), 120);
        assert_eq!(zone.offset_minutes_at(utc(2026, 10, 25, 0, 59)), 120);
        assert_eq!(zone.offset_minutes_at(utc(2026, 10, 25, 1, 0)), 60);
        assert_eq!(zone_from_tzi(&bytes, true), Some(TimeZone::fixed(60)));
        // India: no daylight saving time.
        let bytes = tzi(-330, 0, -60, [0; 8], [0; 8]);
        assert_eq!(zone_from_tzi(&bytes, false), Some(TimeZone::fixed(330)));
        assert_eq!(zone_from_tzi(&bytes[..40], false), None);
        // Pacific: first Sunday of November 02:00 / second Sunday of March 02:00.
        let bytes = tzi(
            480,
            0,
            -60,
            [0, 11, 0, 1, 2, 0, 0, 0],
            [0, 3, 0, 2, 2, 0, 0, 0],
        );
        let zone = zone_from_tzi(&bytes, false).expect("zone");
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 8, 9, 59)), -480);
        assert_eq!(zone.offset_minutes_at(utc(2026, 3, 8, 10, 0)), -420);
    }

    #[test]
    fn local_zone_is_sane() {
        let zone = TimeZone::local();
        let offset = zone.offset_minutes_at(utc(2026, 7, 1, 12, 0));
        assert!((-14 * 60..=14 * 60).contains(&offset), "{offset}");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn registry_zone_by_windows_name() {
        let zone = TimeZone::by_name("Central European Standard Time").expect("registry or table");
        assert_eq!(zone.offset_minutes_at(utc(2026, 7, 1, 12, 0)), 120);
        assert_eq!(zone.offset_minutes_at(utc(2026, 1, 1, 12, 0)), 60);
        assert!(platform::zone_by_windows_name(r"..\..\Evil").is_none());
    }
}
