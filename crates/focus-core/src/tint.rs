//! Warm screen tint schedule (ADR-014): how strong the overlay is at a given local
//! time of day.
//!
//! Pure: the shell passes the local minute of the day (read through `focus-platform`) and draws
//! the overlay windows (ADR-009). The settings preview bypasses this module and shows a fixed
//! strength.

use crate::api::TintConfig;

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

const MINUTES_PER_HOUR: u32 = 60;
const SECS_PER_MINUTE: f64 = 60.0;

impl Default for TintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_minutes: DEFAULT_START_MINUTES,
            end_minutes: DEFAULT_END_MINUTES,
            strength: DEFAULT_STRENGTH,
            transition_minutes: DEFAULT_TRANSITION_MINUTES,
        }
    }
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
        assert_eq!(MINUTES_PER_DAY, 1440);
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
            enabled: true,
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
            enabled: true,
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
