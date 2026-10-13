//! Sunset and sunrise for the warm screen schedule (ADR-023), computed offline from coordinates the user typed or
//! picked from [`CITIES`], with the NOAA solar position algorithm (the equations of the NOAA Solar Calculator, after
//! Jean Meeus, *Astronomical Algorithms*).
//!
//! Pure: no location service, no network, no clock. The shell passes the local date and the UTC offset in effect
//! ([`utc_offset_minutes`] derives it from the local wall clock and the UTC time). Accuracy: about a minute against
//! published tables between ±65° latitude; closer to the poles the sun grazes the horizon and the minutes are less
//! certain, which matters little there (polar day and polar night fall back to the fixed hours in `tint`).

use crate::api::SunCity;

/// Zenith angle of the sun's centre at sunrise and sunset: 90° plus 34′ of atmospheric refraction and 16′ of the
/// solar radius (the standard used by published sunrise tables).
pub const SUNRISE_ZENITH_DEGREES: f64 = 90.833;

const MINUTES_PER_DAY: f64 = 1440.0;
const NOON_MINUTES: f64 = 720.0;
/// The Earth turns by one degree of longitude (or hour angle) in four minutes.
const MINUTES_PER_DEGREE: f64 = 4.0;
/// Julian day of 1970-01-01 00:00 UTC.
const JULIAN_DAY_UNIX_EPOCH: f64 = 2_440_587.5;
/// Julian day of the J2000.0 epoch (2000-01-01 12:00 TT).
const JULIAN_DAY_J2000: f64 = 2_451_545.0;
const DAYS_PER_JULIAN_CENTURY: f64 = 36_525.0;
/// The position of the sun is re-evaluated at the estimated event time this many times (one is within a minute at
/// mid latitudes; three settle the last seconds at high latitudes).
const REFINE_STEPS: usize = 3;
const MS_PER_MINUTE: i64 = 60_000;
const WHOLE_MINUTES_PER_DAY: i64 = 1440;
/// Every UTC offset in use is a multiple of 15 minutes; rounding to it absorbs a minute that ticked over between
/// reading the local and the UTC clock.
const UTC_OFFSET_STEP_MINUTES: i64 = 15;
/// No time zone is further than this from UTC (UTC-12 … UTC+14, with room to spare).
const MAX_UTC_OFFSET_MINUTES: i64 = 18 * 60;

/// Sunrise and sunset on one local date.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SunDay {
    /// Minutes after local midnight of the date, with fractions. Outside `0..1440` when the event falls on the date
    /// before or after (in Reykjavík in June the sun sets a few minutes after midnight: `sunset` ≈ 1444).
    Times { sunrise: f64, sunset: f64 },
    /// The sun stays above the horizon all day.
    PolarDay,
    /// The sun stays below the horizon all day.
    PolarNight,
}

/// Sunrise and sunset at `latitude` / `longitude` (degrees, north and east positive) on the local date
/// `year-month-day`, in local time for `utc_offset_minutes` (`+120` for UTC+2). `None` for an invalid date or
/// coordinates outside -90..=90 / -180..=180.
pub fn sun_day(
    year: i32,
    month: u32,
    day: u32,
    latitude: f64,
    longitude: f64,
    utc_offset_minutes: i32,
) -> Option<SunDay> {
    if !is_valid_date(year, month, day)
        || !latitude.is_finite()
        || !longitude.is_finite()
        || latitude.abs() > 90.0
        || longitude.abs() > 180.0
    {
        return None;
    }
    let offset = f64::from(utc_offset_minutes);
    // Times below are minutes after 00:00 UTC of the date.
    let midnight_jd = days_from_civil(i64::from(year), month, day) as f64 + JULIAN_DAY_UNIX_EPOCH;
    let mean_noon = NOON_MINUTES - MINUTES_PER_DEGREE * longitude;
    // The local date's noon may fall on the UTC date before or after (UTC+14 at 157° W): shift by whole days so the
    // events belong to the local date.
    let day_shift =
        ((mean_noon + offset - NOON_MINUTES) / MINUTES_PER_DAY).round() * MINUTES_PER_DAY;
    let solar_noon = |minutes: f64| {
        mean_noon
            - solar_position(midnight_jd + minutes / MINUTES_PER_DAY).equation_of_time
            - day_shift
    };

    let mut noon = mean_noon - day_shift;
    for _ in 0..REFINE_STEPS {
        noon = solar_noon(noon);
    }
    let at_noon = solar_position(midnight_jd + noon / MINUTES_PER_DAY);
    let half_day = match hour_angle(latitude, at_noon.declination) {
        HourAngle::Degrees(degrees) => degrees,
        HourAngle::AlwaysUp => return Some(SunDay::PolarDay),
        HourAngle::AlwaysDown => return Some(SunDay::PolarNight),
    };

    // `side`: -1 for sunrise (before noon), +1 for sunset.
    let event = |side: f64| {
        let mut minutes = noon + side * MINUTES_PER_DEGREE * half_day;
        for _ in 0..REFINE_STEPS {
            let sun = solar_position(midnight_jd + minutes / MINUTES_PER_DAY);
            let degrees = match hour_angle(latitude, sun.declination) {
                HourAngle::Degrees(degrees) => degrees,
                HourAngle::AlwaysUp => 180.0,
                HourAngle::AlwaysDown => 0.0,
            };
            minutes = solar_noon(minutes) + side * MINUTES_PER_DEGREE * degrees;
        }
        minutes + offset
    };
    Some(SunDay::Times {
        sunrise: event(-1.0),
        sunset: event(1.0),
    })
}

/// The UTC offset in effect, in minutes (`+120` for UTC+2): the local wall clock (`year-month-day`, `minute_of_day`
/// after local midnight) against the UTC time `utc_ms` (Unix milliseconds) read at the same moment, rounded to a
/// multiple of 15 minutes. `None` for an invalid date or an offset no time zone has.
pub fn utc_offset_minutes(
    year: i32,
    month: u32,
    day: u32,
    minute_of_day: u32,
    utc_ms: i64,
) -> Option<i32> {
    if !is_valid_date(year, month, day) || f64::from(minute_of_day) >= MINUTES_PER_DAY {
        return None;
    }
    let local = days_from_civil(i64::from(year), month, day)
        .checked_mul(WHOLE_MINUTES_PER_DAY)?
        .checked_add(i64::from(minute_of_day))?;
    let utc = utc_ms.div_euclid(MS_PER_MINUTE);
    let difference = local.checked_sub(utc)?;
    let half_step = UTC_OFFSET_STEP_MINUTES / 2;
    let rounded =
        (difference + half_step).div_euclid(UTC_OFFSET_STEP_MINUTES) * UTC_OFFSET_STEP_MINUTES;
    if rounded.abs() > MAX_UTC_OFFSET_MINUTES {
        return None;
    }
    i32::try_from(rounded).ok()
}

/// The UTC calendar date `(year, month, day)` of `utc_ms` (Unix milliseconds); for platforms without a local time.
pub fn utc_date(utc_ms: i64) -> (i32, u32, u32) {
    let days = utc_ms.div_euclid(MS_PER_MINUTE * WHOLE_MINUTES_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    (i32::try_from(year).unwrap_or(i32::MAX), month, day)
}

/// Built-in places for the sunset / sunrise schedule (`list_sun_cities`): every Polish voivodeship capital (both seats
/// where a voivodeship has two) and major cities around the world. `(id, name, ISO country, latitude, longitude)`.
/// Names are the cities' own names, romanised where the script is not Latin (`Kyiv`, `Tokyo`); the UI adds the
/// localized country. Polish cities first, then the rest, each alphabetical.
pub const CITIES: [(&str, &str, &str, f64, f64); 43] = [
    ("pl-bialystok", "Białystok", "PL", 53.1325, 23.1688),
    ("pl-bydgoszcz", "Bydgoszcz", "PL", 53.1235, 18.0084),
    ("pl-gdansk", "Gdańsk", "PL", 54.3520, 18.6466),
    (
        "pl-gorzow-wielkopolski",
        "Gorzów Wielkopolski",
        "PL",
        52.7368,
        15.2288,
    ),
    ("pl-katowice", "Katowice", "PL", 50.2649, 19.0238),
    ("pl-kielce", "Kielce", "PL", 50.8661, 20.6286),
    ("pl-krakow", "Kraków", "PL", 50.0647, 19.9450),
    ("pl-lublin", "Lublin", "PL", 51.2465, 22.5684),
    ("pl-lodz", "Łódź", "PL", 51.7592, 19.4560),
    ("pl-olsztyn", "Olsztyn", "PL", 53.7784, 20.4801),
    ("pl-opole", "Opole", "PL", 50.6751, 17.9213),
    ("pl-poznan", "Poznań", "PL", 52.4064, 16.9252),
    ("pl-rzeszow", "Rzeszów", "PL", 50.0412, 21.9991),
    ("pl-szczecin", "Szczecin", "PL", 53.4285, 14.5528),
    ("pl-torun", "Toruń", "PL", 53.0138, 18.5984),
    ("pl-warszawa", "Warszawa", "PL", 52.2297, 21.0122),
    ("pl-wroclaw", "Wrocław", "PL", 51.1079, 17.0385),
    ("pl-zielona-gora", "Zielona Góra", "PL", 51.9356, 15.5062),
    ("nl-amsterdam", "Amsterdam", "NL", 52.3676, 4.9041),
    ("cn-beijing", "Beijing", "CN", 39.9042, 116.4074),
    ("de-berlin", "Berlin", "DE", 52.5200, 13.4050),
    ("ie-dublin", "Dublin", "IE", 53.3498, -6.2603),
    ("fi-helsinki", "Helsinki", "FI", 60.1699, 24.9384),
    ("ua-kyiv", "Kyiv", "UA", 50.4501, 30.5234),
    ("pt-lisboa", "Lisboa", "PT", 38.7223, -9.1393),
    ("gb-london", "London", "GB", 51.5074, -0.1278),
    ("us-los-angeles", "Los Angeles", "US", 34.0522, -118.2437),
    ("es-madrid", "Madrid", "ES", 40.4168, -3.7038),
    ("ru-moskva", "Moskva", "RU", 55.7558, 37.6173),
    ("de-munchen", "München", "DE", 48.1351, 11.5820),
    ("us-new-york", "New York", "US", 40.7128, -74.0060),
    ("no-oslo", "Oslo", "NO", 59.9139, 10.7522),
    ("fr-paris", "Paris", "FR", 48.8566, 2.3522),
    ("cz-praha", "Praha", "CZ", 50.0755, 14.4378),
    ("is-reykjavik", "Reykjavík", "IS", 64.1466, -21.9426),
    ("it-roma", "Roma", "IT", 41.9028, 12.4964),
    ("br-sao-paulo", "São Paulo", "BR", -23.5505, -46.6333),
    ("sg-singapore", "Singapore", "SG", 1.3521, 103.8198),
    ("se-stockholm", "Stockholm", "SE", 59.3293, 18.0686),
    ("au-sydney", "Sydney", "AU", -33.8688, 151.2093),
    ("jp-tokyo", "Tokyo", "JP", 35.6762, 139.6503),
    ("ca-toronto", "Toronto", "CA", 43.6532, -79.3832),
    ("at-wien", "Wien", "AT", 48.2082, 16.3738),
];

/// [`CITIES`] as `SunCity` values, in list order (`list_sun_cities`).
pub fn cities() -> Vec<SunCity> {
    CITIES.iter().map(to_city).collect()
}

/// The built-in city with `id`, if any.
pub fn city(id: &str) -> Option<SunCity> {
    CITIES.iter().find(|entry| entry.0 == id).map(to_city)
}

fn to_city(&(id, name, country, latitude, longitude): &(&str, &str, &str, f64, f64)) -> SunCity {
    SunCity {
        id: id.to_owned(),
        name: name.to_owned(),
        country: country.to_owned(),
        latitude,
        longitude,
    }
}

/// Declination and equation of time of the sun at one moment.
struct SolarPosition {
    /// Radians.
    declination: f64,
    /// Apparent minus mean solar time, minutes.
    equation_of_time: f64,
}

/// The NOAA Solar Calculator equations at Julian day `jd` (UTC; the difference to TT is far below a minute here).
fn solar_position(jd: f64) -> SolarPosition {
    let t = (jd - JULIAN_DAY_J2000) / DAYS_PER_JULIAN_CENTURY;
    // Geometric mean longitude and anomaly of the sun, orbit eccentricity.
    let mean_longitude = (280.466_46 + t * (36_000.769_83 + t * 0.000_303_2)).rem_euclid(360.0);
    let mean_anomaly = 357.529_11 + t * (35_999.050_29 - 0.000_153_7 * t);
    let eccentricity = 0.016_708_634 - t * (0.000_042_037 + 0.000_000_126_7 * t);
    let m = mean_anomaly.to_radians();
    let center = m.sin() * (1.914_602 - t * (0.004_817 + 0.000_014 * t))
        + (2.0 * m).sin() * (0.019_993 - 0.000_101 * t)
        + (3.0 * m).sin() * 0.000_289;
    let true_longitude = mean_longitude + center;
    let omega = (125.04 - 1_934.136 * t).to_radians();
    let apparent_longitude = (true_longitude - 0.005_69 - 0.004_78 * omega.sin()).to_radians();
    let mean_obliquity =
        23.0 + (26.0 + (21.448 - t * (46.815 + t * (0.000_59 - t * 0.001_813))) / 60.0) / 60.0;
    let obliquity = (mean_obliquity + 0.002_56 * omega.cos()).to_radians();
    let declination = (obliquity.sin() * apparent_longitude.sin()).asin();

    let y = (obliquity / 2.0).tan().powi(2);
    let l0 = mean_longitude.to_radians();
    let e = eccentricity;
    let equation = y * (2.0 * l0).sin() - 2.0 * e * m.sin()
        + 4.0 * e * y * m.sin() * (2.0 * l0).cos()
        - 0.5 * y * y * (4.0 * l0).sin()
        - 1.25 * e * e * (2.0 * m).sin();
    SolarPosition {
        declination,
        equation_of_time: MINUTES_PER_DEGREE * equation.to_degrees(),
    }
}

enum HourAngle {
    /// Hour angle of sunrise / sunset, degrees (0..=180).
    Degrees(f64),
    AlwaysUp,
    AlwaysDown,
}

/// Hour angle at which the sun's centre reaches [`SUNRISE_ZENITH_DEGREES`] at `latitude` (degrees) with the sun at
/// `declination` (radians).
fn hour_angle(latitude: f64, declination: f64) -> HourAngle {
    let lat = latitude.to_radians();
    let cos_angle = (SUNRISE_ZENITH_DEGREES.to_radians().cos() - lat.sin() * declination.sin())
        / (lat.cos() * declination.cos());
    if cos_angle.is_nan() {
        // Only at a pole exactly, where cos(latitude) is 0 and the sign of the numerator decides.
        return if lat.signum() == declination.signum() {
            HourAngle::AlwaysUp
        } else {
            HourAngle::AlwaysDown
        };
    }
    if cos_angle > 1.0 {
        HourAngle::AlwaysDown
    } else if cos_angle < -1.0 {
        HourAngle::AlwaysUp
    } else {
        HourAngle::Degrees(cos_angle.acos().to_degrees())
    }
}

fn is_valid_date(year: i32, month: u32, day: u32) -> bool {
    (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a valid civil date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = i64::from((month + 9) % 12);
    let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The civil date of a day number (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    // 1..=31 and 1..=12 by construction.
    let day = (day_of_year - (153 * month_from_march + 2) / 5 + 1) as u32;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Published time (US Naval Observatory, "Rise/Set/Transit" for the date and offset) → minutes.
    fn hm(text: &str) -> f64 {
        let (h, m) = text.split_once(':').expect("HH:MM");
        f64::from(h.parse::<u32>().expect("hour") * 60 + m.parse::<u32>().expect("minute"))
    }

    /// Asserts sunrise and sunset within 2 minutes of the published times (the tables round to the minute).
    #[track_caller]
    fn assert_times(
        date: (i32, u32, u32),
        place: (f64, f64),
        offset_hours: f64,
        sunrise: &str,
        sunset: &str,
    ) {
        let offset = (offset_hours * 60.0) as i32;
        let Some(SunDay::Times {
            sunrise: rise,
            sunset: set,
        }) = sun_day(date.0, date.1, date.2, place.0, place.1, offset)
        else {
            panic!("no sunrise / sunset for {date:?} at {place:?}");
        };
        let close = |actual: f64, published: f64| (actual - published).abs() <= 2.0;
        assert!(
            close(rise, hm(sunrise)),
            "sunrise {rise:.1} vs {sunrise} on {date:?} at {place:?}"
        );
        assert!(
            close(set, hm(sunset)),
            "sunset {set:.1} vs {sunset} on {date:?} at {place:?}"
        );
    }

    const WARSZAWA: (f64, f64) = (52.2297, 21.0122);
    const LONDON: (f64, f64) = (51.5074, -0.1278);
    const TROMSO: (f64, f64) = (69.6492, 18.9553);

    #[test]
    fn warsaw_through_the_year() {
        assert_times((2026, 6, 21), WARSZAWA, 2.0, "04:14", "21:01");
        assert_times((2026, 12, 21), WARSZAWA, 1.0, "07:43", "15:25");
        assert_times((2026, 3, 20), WARSZAWA, 1.0, "05:39", "17:49");
        assert_times((2026, 10, 13), WARSZAWA, 2.0, "06:57", "17:46");
    }

    #[test]
    fn west_of_greenwich_and_across_the_date_line() {
        assert_times((2026, 6, 21), LONDON, 1.0, "04:43", "21:22");
        assert_times((2026, 12, 21), LONDON, 0.0, "08:04", "15:53");
        // New York in summer time, Los Angeles in winter: sunset after 00:00 UTC.
        assert_times((2026, 7, 4), (40.7128, -74.0060), -4.0, "05:30", "20:30");
        assert_times((2026, 12, 21), (34.0522, -118.2437), -8.0, "06:55", "16:48");
        // Tokyo, and Auckland at UTC+12 (local noon is 00:23 UTC).
        assert_times((2026, 10, 13), (35.6762, 139.6503), 9.0, "05:46", "17:09");
        assert_times((2026, 6, 21), (-36.8485, 174.7633), 12.0, "07:34", "17:12");
    }

    #[test]
    fn southern_summer_and_the_equator() {
        assert_times((2026, 1, 15), (-33.8688, 151.2093), 11.0, "05:59", "20:09");
        assert_times((2026, 3, 20), (1.3521, 103.8198), 8.0, "07:09", "19:15");
    }

    #[test]
    fn sunset_after_midnight_in_reykjavik() {
        // The tables list the sunset of the evening before at 00:04; this evening's falls at about 00:04 too, after
        // midnight, so it is reported past 1440.
        let Some(SunDay::Times { sunrise, sunset }) = sun_day(2026, 6, 21, 64.1466, -21.9426, 0)
        else {
            panic!("Reykjavík has a sunset in June");
        };
        assert!((sunrise - hm("02:55")).abs() <= 2.0, "{sunrise}");
        assert!(
            (sunset - (MINUTES_PER_DAY + hm("00:04"))).abs() <= 2.0,
            "{sunset}"
        );
    }

    #[test]
    fn polar_day_and_polar_night() {
        assert_eq!(
            sun_day(2026, 6, 21, TROMSO.0, TROMSO.1, 120),
            Some(SunDay::PolarDay)
        );
        assert_eq!(
            sun_day(2026, 5, 25, TROMSO.0, TROMSO.1, 120),
            Some(SunDay::PolarDay)
        );
        assert_eq!(
            sun_day(2026, 12, 21, TROMSO.0, TROMSO.1, 60),
            Some(SunDay::PolarNight)
        );
        // The southern hemisphere the other way round, and the poles themselves.
        assert_eq!(
            sun_day(2026, 6, 21, -75.0, 0.0, 0),
            Some(SunDay::PolarNight)
        );
        assert_eq!(sun_day(2026, 12, 21, -75.0, 0.0, 0), Some(SunDay::PolarDay));
        assert_eq!(sun_day(2026, 6, 21, 90.0, 0.0, 0), Some(SunDay::PolarDay));
        assert_eq!(
            sun_day(2026, 6, 21, -90.0, 0.0, 0),
            Some(SunDay::PolarNight)
        );
        // Tromsø has ordinary days in spring.
        assert!(matches!(
            sun_day(2026, 3, 20, TROMSO.0, TROMSO.1, 60),
            Some(SunDay::Times { .. })
        ));
    }

    #[test]
    fn local_date_far_from_utc_keeps_its_own_events() {
        // Kiribati's Line Islands: UTC+14 at 157° W. The local date's sunrise and sunset stay on that date.
        let Some(SunDay::Times { sunrise, sunset }) = sun_day(2026, 6, 21, 1.87, -157.4, 14 * 60)
        else {
            panic!("equator");
        };
        assert!((0.0..MINUTES_PER_DAY).contains(&sunrise), "{sunrise}");
        assert!((0.0..MINUTES_PER_DAY).contains(&sunset), "{sunset}");
        assert!(sunrise < sunset);
        // The same place on the next local date is about a day's events later, not a day earlier.
        let Some(SunDay::Times { sunrise: next, .. }) = sun_day(2026, 6, 22, 1.87, -157.4, 14 * 60)
        else {
            panic!("equator");
        };
        assert!((next - sunrise).abs() < 2.0, "{sunrise} → {next}");
    }

    #[test]
    fn invalid_input_has_no_answer() {
        assert_eq!(sun_day(2026, 2, 30, 52.0, 21.0, 60), None);
        assert_eq!(sun_day(2026, 13, 1, 52.0, 21.0, 60), None);
        assert_eq!(sun_day(2026, 6, 0, 52.0, 21.0, 60), None);
        assert_eq!(sun_day(2026, 6, 1, 91.0, 21.0, 60), None);
        assert_eq!(sun_day(2026, 6, 1, 52.0, -180.5, 60), None);
        assert_eq!(sun_day(2026, 6, 1, f64::NAN, 21.0, 60), None);
        assert_eq!(sun_day(2026, 6, 1, 52.0, f64::INFINITY, 60), None);
        assert!(sun_day(2024, 2, 29, 52.0, 21.0, 60).is_some(), "leap day");
        assert_eq!(sun_day(2026, 2, 29, 52.0, 21.0, 60), None);
    }

    #[test]
    fn utc_offset_from_the_two_clocks() {
        // 2026-10-13 10:00:30 UTC.
        let utc = 1_791_885_630_000;
        assert_eq!(utc_offset_minutes(2026, 10, 13, 12 * 60, utc), Some(120));
        assert_eq!(utc_offset_minutes(2026, 10, 13, 10 * 60, utc), Some(0));
        assert_eq!(utc_offset_minutes(2026, 10, 13, 5 * 60, utc), Some(-300));
        // India and Nepal: half and quarter hours.
        assert_eq!(
            utc_offset_minutes(2026, 10, 13, 15 * 60 + 30, utc),
            Some(330)
        );
        assert_eq!(
            utc_offset_minutes(2026, 10, 13, 15 * 60 + 45, utc),
            Some(345)
        );
        // Across the date line both ways.
        assert_eq!(utc_offset_minutes(2026, 10, 14, 0, utc), Some(840));
        assert_eq!(utc_offset_minutes(2026, 10, 12, 22 * 60, utc), Some(-720));
        // A minute ticking over between the two reads is absorbed.
        assert_eq!(
            utc_offset_minutes(2026, 10, 13, 12 * 60 + 1, utc),
            Some(120)
        );
        assert_eq!(
            utc_offset_minutes(2026, 10, 13, 11 * 60 + 59, utc),
            Some(120)
        );
        // Nonsense.
        assert_eq!(utc_offset_minutes(2026, 10, 15, 12 * 60, utc), None);
        assert_eq!(utc_offset_minutes(2026, 10, 13, 1440, utc), None);
        assert_eq!(utc_offset_minutes(2026, 2, 30, 0, utc), None);
    }

    #[test]
    fn utc_dates() {
        assert_eq!(utc_date(0), (1970, 1, 1));
        assert_eq!(utc_date(1_791_885_630_000), (2026, 10, 13));
        assert_eq!(utc_date(-1), (1969, 12, 31));
        assert_eq!(utc_date(951_782_400_000), (2000, 2, 29));
        for days in [-1_000_000_i64, -1, 0, 59, 10_957, 20_739, 1_000_000] {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days, "{days}");
        }
    }

    #[test]
    fn built_in_cities() {
        let list = cities();
        assert_eq!(list.len(), CITIES.len());
        assert!((40..=45).contains(&list.len()));
        let mut ids = std::collections::HashSet::new();
        for city in &list {
            assert!(ids.insert(city.id.clone()), "{} twice", city.id);
            assert!(
                city.id.len() <= crate::config::MAX_CITY_ID_CHARS
                    && city.id.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{}",
                city.id
            );
            assert!(
                city.id
                    .starts_with(&format!("{}-", city.country.to_lowercase()))
            );
            assert_eq!(city.country.len(), 2);
            assert!(
                crate::config::LATITUDE_RANGE.contains(&city.latitude),
                "{}",
                city.id
            );
            assert!(
                crate::config::LONGITUDE_RANGE.contains(&city.longitude),
                "{}",
                city.id
            );
            // Every built-in city has a sunrise and a sunset today.
            assert!(
                matches!(
                    sun_day(2026, 10, 13, city.latitude, city.longitude, 0),
                    Some(SunDay::Times { .. })
                ),
                "{}",
                city.id
            );
        }
        // Every voivodeship capital (both seats of Kujawsko-Pomorskie and Lubuskie).
        let polish = list.iter().filter(|c| c.country == "PL").count();
        assert_eq!(polish, 18);
        assert_eq!(
            city("pl-warszawa").map(|c| c.name),
            Some("Warszawa".to_owned())
        );
        assert_eq!(
            city("de-munchen").map(|c| c.name),
            Some("München".to_owned())
        );
        assert_eq!(city("xx-nowhere"), None);
    }
}
