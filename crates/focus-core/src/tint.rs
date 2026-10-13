//! Warm screen tint (ADR-014): how strong the warmth is at a given local time of day, and what
//! colour temperature / per-channel gain a strength stands for.
//!
//! Pure: the shell passes the local minute of the day (read through `focus-platform`) and applies
//! the result with Windows Night light or the display gamma ramps (`TintMode`). Both work after
//! composition, so they change the colours on the screen without dimming it and stay out of
//! screenshots. The settings preview bypasses the schedule and shows a fixed strength.
//!
//! Strength → temperature: `0` is [`NEUTRAL_KELVIN`] (screen untouched), `100` is
//! [`WARMEST_KELVIN`]; in between the temperature is interpolated in mireds (`1e6 / K`), where equal
//! steps look about equally large, so the slider feels even across its range.

use crate::api::{TintConfig, TintMode};

/// Minutes in a day; schedule times are `0..MINUTES_PER_DAY` minutes after local midnight.
pub const MINUTES_PER_DAY: u32 = 24 * MINUTES_PER_HOUR;
/// Full tint strength never exceeds this.
pub const MAX_STRENGTH: u32 = 100;
/// Longest fade (in and out) in minutes.
pub const MAX_TRANSITION_MINUTES: u32 = 180;
/// Default schedule start: 18:00.
pub const DEFAULT_START_MINUTES: u32 = 18 * MINUTES_PER_HOUR;
/// Default schedule end (screen back to normal): 09:00.
pub const DEFAULT_END_MINUTES: u32 = 9 * MINUTES_PER_HOUR;
/// Default full strength.
pub const DEFAULT_STRENGTH: u32 = 45;
/// Default fade length in minutes.
pub const DEFAULT_TRANSITION_MINUTES: u32 = 30;
/// Colour temperature of an untouched sRGB screen (level 0): gains `[1, 1, 1]`.
pub const NEUTRAL_KELVIN: u32 = 6500;
/// Colour temperature at full strength (level 100), about candlelight.
pub const WARMEST_KELVIN: u32 = 1900;
/// [`rgb_gain_for_kelvin`] treats lower temperatures as this one.
pub const MIN_KELVIN: u32 = 1000;

const MINUTES_PER_HOUR: u32 = 60;
const SECS_PER_MINUTE: f64 = 60.0;
const MIREDS_PER_KELVIN: f64 = 1_000_000.0;

impl Default for TintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_minutes: DEFAULT_START_MINUTES,
            end_minutes: DEFAULT_END_MINUTES,
            strength: DEFAULT_STRENGTH,
            transition_minutes: DEFAULT_TRANSITION_MINUTES,
            mode: TintMode::NightLight,
        }
    }
}

/// Colour temperature in kelvin for a tint level `0..=100` (higher levels are treated as 100).
///
/// `0` → [`NEUTRAL_KELVIN`] (6500 K), `100` → [`WARMEST_KELVIN`] (1900 K), strictly decreasing
/// in between (linear in mireds, rounded to whole kelvin): e.g. 20 → about 4380 K,
/// 45 → about 3110 K, 70 → about 2400 K.
pub fn kelvin_for_level(level: u32) -> u32 {
    let t = f64::from(level.min(MAX_STRENGTH)) / f64::from(MAX_STRENGTH);
    let neutral = MIREDS_PER_KELVIN / f64::from(NEUTRAL_KELVIN);
    let warmest = MIREDS_PER_KELVIN / f64::from(WARMEST_KELVIN);
    let mireds = neutral + (warmest - neutral) * t;
    // Between 1900 and 6500, so the cast is exact after rounding.
    (MIREDS_PER_KELVIN / mireds)
        .round()
        .clamp(f64::from(WARMEST_KELVIN), f64::from(NEUTRAL_KELVIN)) as u32
}

/// Per-channel gains `[red, green, blue]` (each `0..=1`) that turn a 6500 K white into the colour
/// of a black body at `kelvin`, for scaling gamma ramps (`ramp[i] * gain`).
///
/// A curve fit of the black-body colour in sRGB (Tanner Helland's approximation), normalised so
/// that [`NEUTRAL_KELVIN`] is exactly `[1, 1, 1]`. Only warming is supported: temperatures at or
/// above 6500 K return `[1, 1, 1]`, below [`MIN_KELVIN`] count as `MIN_KELVIN`. Red stays `1.0`;
/// green and blue fall as the temperature drops (blue reaches 0 at about 1900 K).
pub fn rgb_gain_for_kelvin(kelvin: u32) -> [f32; 3] {
    let neutral = f64::from(NEUTRAL_KELVIN) / 100.0;
    let t = f64::from(kelvin.clamp(MIN_KELVIN, NEUTRAL_KELVIN)) / 100.0;
    let green = warm_green(t) / warm_green(neutral);
    let blue = warm_blue(t) / warm_blue(neutral);
    [1.0, unit(green), unit(blue)]
}

/// Green channel (0..=255 scale) of a black body at `t` hundred kelvin, for `t <= 66`.
fn warm_green(t: f64) -> f64 {
    99.470_802_586_1 * t.ln() - 161.119_568_166_1
}

/// Blue channel (0..=255 scale) of a black body at `t` hundred kelvin, for `t <= 66`
/// (no blue at or below 1900 K).
fn warm_blue(t: f64) -> f64 {
    if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    }
}

/// `value` clamped to `0..=1` as `f32` (a NaN becomes 0).
fn unit(value: f64) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0) as f32
}

/// Local wall-clock time as a fractional minute of the day (`18:15:30` → `1095.5`).
pub fn minute_of_day(hour: u32, minute: u32, second: u32) -> f64 {
    f64::from(hour) * f64::from(MINUTES_PER_HOUR)
        + f64::from(minute)
        + f64::from(second) / SECS_PER_MINUTE
}

/// Tint strength `0..=100` at `minute_of_day` (local, may be fractional; taken modulo a day).
///
/// `0` when the schedule is disabled or the time is outside the window. The window runs from
/// `start_minutes` to `end_minutes` and may cross midnight (18:00 → 09:00). The level fades in
/// linearly over `transition_minutes` after the start, stays at `strength`, and fades out over
/// `transition_minutes` before the end; when the window is shorter than two fades, each fade takes
/// half of it. A transition of 0 switches at the exact start and end. A window with `start == end`
/// (rejected by config validation) never tints.
pub fn tint_level(cfg: &TintConfig, minute_of_day: f64) -> u32 {
    if !cfg.enabled || !minute_of_day.is_finite() {
        return 0;
    }
    let day = f64::from(MINUTES_PER_DAY);
    let start = f64::from(cfg.start_minutes % MINUTES_PER_DAY);
    let end = f64::from(cfg.end_minutes % MINUTES_PER_DAY);
    let duration = (end - start).rem_euclid(day);
    let offset = (minute_of_day - start).rem_euclid(day);
    if duration <= 0.0 || offset >= duration {
        return 0;
    }
    let rise = f64::from(cfg.transition_minutes).min(duration / 2.0);
    let factor = if rise > 0.0 {
        (offset.min(duration - offset) / rise).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let strength = f64::from(cfg.strength.min(MAX_STRENGTH));
    // A whole number within 0..=100, so the cast is exact.
    (strength * factor).round().clamp(0.0, strength) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evening() -> TintConfig {
        TintConfig {
            enabled: true,
            ..TintConfig::default()
        }
    }

    fn at(hour: u32, minute: u32) -> f64 {
        minute_of_day(hour, minute, 0)
    }

    #[test]
    fn defaults_are_off_from_18_to_9() {
        let cfg = TintConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.start_minutes, 1080);
        assert_eq!(cfg.end_minutes, 540);
        assert_eq!(cfg.strength, 45);
        assert_eq!(cfg.transition_minutes, 30);
        assert_eq!(cfg.mode, TintMode::NightLight);
        assert_eq!(MINUTES_PER_DAY, 1440);
    }

    #[test]
    fn level_maps_to_kelvin_from_6500_to_1900() {
        assert_eq!(kelvin_for_level(0), 6500);
        assert_eq!(kelvin_for_level(100), 1900);
        assert_eq!(kelvin_for_level(101), 1900);
        assert_eq!(kelvin_for_level(u32::MAX), 1900);
        // Linear in mireds: 1e6 / (1e6/6500 + (1e6/1900 - 1e6/6500) * level / 100).
        assert_eq!(kelvin_for_level(20), 4379);
        assert_eq!(kelvin_for_level(45), 3111);
        assert_eq!(kelvin_for_level(50), 2940);
        assert_eq!(kelvin_for_level(70), 2412);
        let mut previous = kelvin_for_level(0);
        for level in 1..=MAX_STRENGTH {
            let kelvin = kelvin_for_level(level);
            assert!(kelvin < previous, "not decreasing at {level}");
            // Smooth: no step larger than the first one (the curve flattens towards warm).
            assert!(
                previous - kelvin <= 6500 - kelvin_for_level(1),
                "jump at {level}"
            );
            previous = kelvin;
        }
    }

    fn approx(actual: [f32; 3], expected: [f32; 3]) -> bool {
        actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 0.005)
    }

    #[test]
    fn neutral_temperature_has_unit_gains() {
        assert_eq!(rgb_gain_for_kelvin(NEUTRAL_KELVIN), [1.0, 1.0, 1.0]);
        assert_eq!(rgb_gain_for_kelvin(kelvin_for_level(0)), [1.0, 1.0, 1.0]);
        // Cooler than neutral is not supported: no change.
        assert_eq!(rgb_gain_for_kelvin(9300), [1.0, 1.0, 1.0]);
        assert_eq!(rgb_gain_for_kelvin(u32::MAX), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn warm_temperatures_keep_red_and_cut_green_and_blue() {
        let gains = rgb_gain_for_kelvin;
        assert!(
            approx(gains(5000), [1.0, 0.897, 0.824]),
            "{:?}",
            gains(5000)
        );
        assert!(
            approx(gains(3400), [1.0, 0.746, 0.541]),
            "{:?}",
            gains(3400)
        );
        assert!(
            approx(gains(2700), [1.0, 0.656, 0.350]),
            "{:?}",
            gains(2700)
        );
        assert!(approx(gains(1900), [1.0, 0.518, 0.0]), "{:?}", gains(1900));
        assert!(approx(gains(1000), [1.0, 0.267, 0.0]), "{:?}", gains(1000));
        assert_eq!(gains(0), gains(MIN_KELVIN));
        assert_eq!(gains(500), gains(MIN_KELVIN));

        let mut previous = gains(NEUTRAL_KELVIN);
        for kelvin in (MIN_KELVIN..NEUTRAL_KELVIN).rev().step_by(10) {
            let gain = gains(kelvin);
            assert_eq!(gain[0], 1.0, "red at {kelvin}");
            assert!(gain[1] <= previous[1] && gain[2] <= previous[2], "{kelvin}");
            assert!(gain.iter().all(|g| (0.0..=1.0).contains(g)), "{kelvin}");
            assert!(gain[2] <= gain[1], "blue above green at {kelvin}");
            previous = gain;
        }
    }

    #[test]
    fn every_level_has_valid_gains() {
        let mut previous = [1.0, 1.0, 1.0];
        for level in 0..=MAX_STRENGTH {
            let gain = rgb_gain_for_kelvin(kelvin_for_level(level));
            assert_eq!(gain[0], 1.0);
            assert!(gain[1] <= previous[1] && gain[2] <= previous[2], "{level}");
            previous = gain;
        }
        assert!(previous[1] > 0.5, "full strength keeps some green");
    }

    #[test]
    fn disabled_never_tints() {
        let cfg = TintConfig::default();
        for minute in (0..MINUTES_PER_DAY).step_by(15) {
            assert_eq!(tint_level(&cfg, f64::from(minute)), 0, "{minute}");
        }
    }

    #[test]
    fn evening_schedule_across_midnight() {
        let cfg = evening();
        for (time, expected) in [
            (at(12, 0), 0),
            (at(17, 59), 0),
            (at(18, 0), 0),
            (at(18, 15), 23), // half of 45, rounded
            (at(18, 30), 45),
            (at(21, 0), 45),
            (at(0, 0), 45),
            (at(3, 0), 45),
            (at(8, 30), 45),
            (at(8, 45), 23),
            (at(9, 0), 0),
            (at(9, 1), 0),
        ] {
            assert_eq!(tint_level(&cfg, time), expected, "minute {time}");
        }
    }

    #[test]
    fn fades_are_linear_and_monotonic() {
        let cfg = TintConfig {
            strength: 100,
            ..evening()
        };
        assert_eq!(tint_level(&cfg, at(18, 3)), 10);
        assert_eq!(tint_level(&cfg, at(18, 6)), 20);
        assert_eq!(tint_level(&cfg, at(8, 54)), 20);
        let mut previous = 0;
        for second in 0..=(30 * 60) {
            let level = tint_level(&cfg, at(18, 0) + f64::from(second) / 60.0);
            assert!(level >= previous, "fade-in went down at {second} s");
            previous = level;
        }
        assert_eq!(previous, 100);
        for second in 0..=(30 * 60) {
            let level = tint_level(&cfg, at(8, 30) + f64::from(second) / 60.0);
            assert!(level <= previous, "fade-out went up at {second} s");
            previous = level;
        }
        assert_eq!(previous, 0);
    }

    #[test]
    fn accepts_fractional_and_out_of_range_minutes() {
        let cfg = evening();
        assert_eq!(tint_level(&cfg, minute_of_day(18, 15, 0)), 23);
        assert_eq!(minute_of_day(18, 15, 30), 1095.5);
        // Taken modulo a day, also negative.
        assert_eq!(tint_level(&cfg, at(3, 0) + 1440.0), 45);
        assert_eq!(tint_level(&cfg, at(3, 0) - 1440.0), 45);
        assert_eq!(tint_level(&cfg, -0.5), 45); // 23:59:30
        assert_eq!(tint_level(&cfg, at(12, 0) + 7.0 * 1440.0), 0);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(tint_level(&cfg, bad), 0);
        }
    }

    #[test]
    fn daytime_window_without_midnight() {
        let cfg = TintConfig {
            start_minutes: at(13, 0) as u32,
            end_minutes: at(15, 0) as u32,
            strength: 60,
            transition_minutes: 20,
            ..evening()
        };
        for (time, expected) in [
            (at(12, 59), 0),
            (at(13, 0), 0),
            (at(13, 10), 30),
            (at(13, 20), 60),
            (at(14, 0), 60),
            (at(14, 50), 30),
            (at(15, 0), 0),
            (at(23, 0), 0),
            (at(3, 0), 0),
        ] {
            assert_eq!(tint_level(&cfg, time), expected, "minute {time}");
        }
    }

    #[test]
    fn zero_transition_is_a_step() {
        let cfg = TintConfig {
            transition_minutes: 0,
            ..evening()
        };
        assert_eq!(tint_level(&cfg, at(17, 59) + 0.99), 0);
        assert_eq!(tint_level(&cfg, at(18, 0)), 45);
        assert_eq!(tint_level(&cfg, at(8, 59) + 0.99), 45);
        assert_eq!(tint_level(&cfg, at(9, 0)), 0);
    }

    #[test]
    fn transition_longer_than_half_the_window() {
        // 60-minute window, 180-minute fades: each fade takes 30 minutes, peak at the middle.
        let cfg = TintConfig {
            start_minutes: at(23, 30) as u32,
            end_minutes: at(0, 30) as u32,
            strength: 80,
            transition_minutes: 180,
            ..evening()
        };
        assert_eq!(tint_level(&cfg, at(23, 30)), 0);
        assert_eq!(tint_level(&cfg, at(23, 45)), 40);
        assert_eq!(tint_level(&cfg, at(0, 0)), 80);
        assert_eq!(tint_level(&cfg, at(0, 15)), 40);
        assert_eq!(tint_level(&cfg, at(0, 30)), 0);
        assert_eq!(tint_level(&cfg, at(12, 0)), 0);
    }

    #[test]
    fn degenerate_window_and_strength_clamp() {
        let empty = TintConfig {
            end_minutes: DEFAULT_START_MINUTES,
            ..evening()
        };
        for minute in (0..MINUTES_PER_DAY).step_by(30) {
            assert_eq!(tint_level(&empty, f64::from(minute)), 0, "{minute}");
        }
        let strong = TintConfig {
            strength: 250,
            ..evening()
        };
        assert_eq!(tint_level(&strong, at(3, 0)), MAX_STRENGTH);
        let none = TintConfig {
            strength: 0,
            ..evening()
        };
        assert_eq!(tint_level(&none, at(3, 0)), 0);
        // Times past midnight in the config are taken modulo a day.
        let wrapped = TintConfig {
            start_minutes: DEFAULT_START_MINUTES + MINUTES_PER_DAY,
            ..evening()
        };
        assert_eq!(tint_level(&wrapped, at(3, 0)), 45);
    }
}
