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
//!
//! ADR-023 adds the window and the exceptions:
//! * [`tint_window`]: the warm window of one local date, the fixed hours or local sunset → sunrise
//!   ([`crate::sun`]), with the fixed hours as the fallback without a place and on polar days and nights;
//!   [`window_level`] is [`tint_level`] for that window.
//! * [`pause_reason`] and [`TintExceptions`]: a colour-critical app (`TintConfig.pause_apps`, exact exe names
//!   compared case-insensitively) or a full-screen app in front suspends the warmth. It fades out over
//!   [`PAUSE_FADE_MS`] and comes back the same way once the exception has been gone for [`PAUSE_RELEASE_MS`]
//!   (Alt+Tab past it does not flicker). The shell samples the foreground and passes monotonic milliseconds.

use crate::api::{
    TintConfig, TintMode, TintPause, TintPauseReason, TintSchedule, TintScheduleState,
};
use crate::sun::{self, SunDay};

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
/// Colour-critical apps that suspend the warmth by default (`TintConfig.pause_apps`, ADR-023). Exe names are
/// matched case-insensitively, so `Lightroom.exe` covers both editions (Lightroom Classic is `lightroom.exe`).
pub const DEFAULT_PAUSE_APPS: [&str; 17] = [
    "Photoshop.exe",
    "Illustrator.exe",
    "Lightroom.exe",
    "AfterFX.exe",
    "Adobe Premiere Pro.exe",
    "InDesign.exe",
    "Figma.exe",
    "Affinity Photo 2.exe",
    "Affinity Designer 2.exe",
    "Affinity Publisher 2.exe",
    "gimp-2.10.exe",
    "gimp-3.0.exe",
    "krita.exe",
    "Resolve.exe",
    "blender.exe",
    "Substance 3D Painter.exe",
    "CaptureOne.exe",
];

/// An exception (colour-critical or full-screen app) must be gone this long before the warmth comes back, so a quick
/// Alt+Tab past it, the taskbar or a notification does not make the screen flicker (ADR-023).
pub const PAUSE_RELEASE_MS: u64 = 2_000;
/// The warmth fades out over this when an exception starts and back in when it ends (ADR-023).
pub const PAUSE_FADE_MS: u64 = 1_200;

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
            schedule: TintSchedule::Fixed,
            location: None,
            pause_for_apps: true,
            pause_apps: DEFAULT_PAUSE_APPS
                .iter()
                .map(|&app| app.to_owned())
                .collect(),
            pause_for_fullscreen: false,
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
/// (rejected by config validation) never tints. Always the fixed hours; [`window_level`] takes the window of
/// [`tint_window`] (sunset → sunrise).
pub fn tint_level(cfg: &TintConfig, minute_of_day: f64) -> u32 {
    window_level(
        cfg,
        &fixed_window(cfg, TintScheduleState::Fixed),
        minute_of_day,
    )
}

/// [`tint_level`] with the window opening at `window.start_minutes` and closing at `window.end_minutes` instead of
/// the fixed hours (strength and fades from `cfg`).
pub fn window_level(cfg: &TintConfig, window: &TintWindow, minute_of_day: f64) -> u32 {
    if !cfg.enabled || !minute_of_day.is_finite() {
        return 0;
    }
    let day = f64::from(MINUTES_PER_DAY);
    let start = f64::from(window.start_minutes % MINUTES_PER_DAY);
    let end = f64::from(window.end_minutes % MINUTES_PER_DAY);
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

/// A local calendar date: `(year, month 1..=12, day 1..=31)`.
pub type LocalDate = (i32, u32, u32);

/// The warm window in effect on one local date (`TintStatus.schedule`, `window_start_minutes`, `window_end_minutes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TintWindow {
    pub schedule: TintScheduleState,
    /// Minutes after local midnight, `0..1440`: where the warmth starts fading in (sunset for `Sun`).
    pub start_minutes: u32,
    /// Minutes after local midnight, `0..1440`: where the screen is back to normal (sunrise for `Sun`).
    pub end_minutes: u32,
}

/// The warm window on the local `date` (ADR-023). `Fixed`: `TintConfig.start_minutes` → `end_minutes`. `Sun`: local
/// sunset → sunrise of that date at `TintConfig.location` for `utc_offset_minutes` (see [`sun::utc_offset_minutes`]),
/// rounded to whole minutes. The fixed hours stand in without a usable location (`NoLocation`) and when the sun does
/// not set (`PolarDay`) or rise (`PolarNight`) that day, also when sunset and sunrise round to the same minute.
///
/// The date is the one of the moment the level is computed for: after midnight the window closing at this morning's
/// sunrise is the one that opened at last evening's sunset (minutes apart from this evening's).
pub fn tint_window(cfg: &TintConfig, date: LocalDate, utc_offset_minutes: i32) -> TintWindow {
    if cfg.schedule == TintSchedule::Fixed {
        return fixed_window(cfg, TintScheduleState::Fixed);
    }
    let Some(location) = &cfg.location else {
        return fixed_window(cfg, TintScheduleState::NoLocation);
    };
    let (year, month, day) = date;
    match sun::sun_day(
        year,
        month,
        day,
        location.latitude,
        location.longitude,
        utc_offset_minutes,
    ) {
        None => fixed_window(cfg, TintScheduleState::NoLocation),
        Some(SunDay::PolarDay) => fixed_window(cfg, TintScheduleState::PolarDay),
        Some(SunDay::PolarNight) => fixed_window(cfg, TintScheduleState::PolarNight),
        Some(SunDay::Times { sunrise, sunset }) => {
            let start = minute_in_day(sunset);
            let end = minute_in_day(sunrise);
            if start != end {
                return TintWindow {
                    schedule: TintScheduleState::Sun,
                    start_minutes: start,
                    end_minutes: end,
                };
            }
            // Sunset and sunrise within the same minute: a night (or a day) of almost nothing.
            let day_length = sunset - sunrise;
            if day_length >= f64::from(MINUTES_PER_DAY / 2) {
                fixed_window(cfg, TintScheduleState::PolarDay)
            } else {
                fixed_window(cfg, TintScheduleState::PolarNight)
            }
        }
    }
}

fn fixed_window(cfg: &TintConfig, schedule: TintScheduleState) -> TintWindow {
    TintWindow {
        schedule,
        start_minutes: cfg.start_minutes % MINUTES_PER_DAY,
        end_minutes: cfg.end_minutes % MINUTES_PER_DAY,
    }
}

/// A local time in minutes (any day) → the whole minute of the day `0..1440`.
fn minute_in_day(minutes: f64) -> u32 {
    let day = f64::from(MINUTES_PER_DAY);
    // Within 0..1440 after `rem_euclid`; 1440 itself (rounding up from 1439.5) wraps to 0.
    (minutes.rem_euclid(day).round() as u32) % MINUTES_PER_DAY
}

/// `process` (an exe name from the foreground window, e.g. `Photoshop.exe`) is one of `TintConfig.pause_apps`, compared
/// case-insensitively. Exact names only: no paths, no prefixes, no window titles.
pub fn is_pause_app(cfg: &TintConfig, process: &str) -> bool {
    let process = process.trim();
    if process.is_empty() {
        return false;
    }
    let wanted = process.to_lowercase();
    cfg.pause_apps
        .iter()
        .any(|app| app.trim().to_lowercase() == wanted)
}

/// What suspends the warmth right now, before the hysteresis of [`TintExceptions`]: a colour-critical app in front
/// (`pause_for_apps`), else a full-screen app or game in front (`pause_for_fullscreen`). `foreground` is the exe name of
/// the foreground window (`None` when unknown), `full_screen` whether it covers the screen (or the user presents).
pub fn pause_reason(
    cfg: &TintConfig,
    foreground: Option<&str>,
    full_screen: bool,
) -> Option<TintPause> {
    let process = foreground.map(str::trim).filter(|name| !name.is_empty());
    if cfg.pause_for_apps
        && let Some(app) = process.filter(|name| is_pause_app(cfg, name))
    {
        return Some(TintPause {
            reason: TintPauseReason::App,
            app: Some(app.to_owned()),
        });
    }
    if cfg.pause_for_fullscreen && full_screen {
        return Some(TintPause {
            reason: TintPauseReason::Fullscreen,
            app: process.map(str::to_owned),
        });
    }
    None
}

/// The exceptions over time (ADR-023): a pause starts at once, ends [`PAUSE_RELEASE_MS`] after the first sample that
/// no longer saw it (unless it shows up again meanwhile), and the warmth follows with a linear fade of
/// [`PAUSE_FADE_MS`] each way, starting at the sample that changed the direction. Times are monotonic milliseconds
/// from the shell.
#[derive(Debug, Clone, PartialEq)]
pub struct TintExceptions {
    pause: Option<TintPause>,
    /// The first sample without the exception since it was last seen.
    gone_since_ms: Option<u64>,
    /// Share of the scheduled warmth shown: 1 = all of it, 0 = paused.
    factor: f64,
    /// The fade under way: when it started, from which factor and towards which.
    fade: Option<Fade>,
    /// A sample was taken since [`TintExceptions::reset`]; until then the next sample jumps (no fade).
    started: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fade {
    start_ms: u64,
    from: f64,
    target: f64,
}

impl Default for TintExceptions {
    fn default() -> Self {
        Self::new()
    }
}

impl TintExceptions {
    pub fn new() -> Self {
        Self {
            pause: None,
            gone_since_ms: None,
            factor: 1.0,
            fade: None,
            started: false,
        }
    }

    /// Takes a sample at `now_ms`: `seen` is [`pause_reason`] for the foreground now. The first sample after
    /// [`Self::new`] / [`Self::reset`] applies the result without a fade (the warmth was not shown while nobody
    /// watched, e.g. outside the warm window).
    pub fn observe(&mut self, now_ms: u64, seen: Option<TintPause>) {
        match seen {
            Some(pause) => {
                self.pause = Some(pause);
                self.gone_since_ms = None;
            }
            None if self.pause.is_some() => {
                let since = *self.gone_since_ms.get_or_insert(now_ms);
                if now_ms.saturating_sub(since) >= PAUSE_RELEASE_MS {
                    self.pause = None;
                    self.gone_since_ms = None;
                }
            }
            None => self.gone_since_ms = None,
        }
        let target = self.target();
        if !self.started {
            self.started = true;
            self.factor = target;
            self.fade = None;
            return;
        }
        if self.factor == target {
            self.fade = None;
            return;
        }
        let fade = match self.fade {
            Some(fade) if fade.target == target => fade,
            // A new direction: the fade starts here, from where the warmth is now.
            _ => Fade {
                start_ms: now_ms,
                from: self.factor,
                target,
            },
        };
        self.fade = Some(fade);
        // A few seconds of milliseconds at most here: exact in f64.
        let progress = now_ms.saturating_sub(fade.start_ms) as f64 / PAUSE_FADE_MS as f64;
        // Only ever towards the target (a sample out of order never moves the warmth back).
        self.factor = if target < fade.from {
            (fade.from - progress).max(target).min(self.factor)
        } else {
            (fade.from + progress).min(target).max(self.factor)
        };
        if self.factor == target {
            self.fade = None;
        }
    }

    /// Forgets everything (no pause, full warmth): the next [`Self::observe`] starts afresh.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// The exception in effect (kept through the release hysteresis), `None` when the warmth may show.
    pub fn paused_by(&self) -> Option<&TintPause> {
        self.pause.as_ref()
    }

    /// Nothing pending: no pause, no hysteresis running and no fade under way.
    pub fn is_settled(&self) -> bool {
        self.pause.is_none() && self.gone_since_ms.is_none() && !self.is_fading()
    }

    /// A fade is under way (the shell then re-applies the level often for a smooth change).
    pub fn is_fading(&self) -> bool {
        self.factor != self.target()
    }

    fn target(&self) -> f64 {
        if self.pause.is_some() { 0.0 } else { 1.0 }
    }

    /// `scheduled` (`0..=100`) as shown with the exceptions: scaled by the fade, 0 while fully paused.
    pub fn level(&self, scheduled: u32) -> u32 {
        let scheduled = scheduled.min(MAX_STRENGTH);
        // Within 0..=scheduled, so the cast is exact.
        (f64::from(scheduled) * self.factor.clamp(0.0, 1.0)).round() as u32
    }
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
        // ADR-023: fixed hours, no location, colour-critical apps on, full screen off.
        assert_eq!(cfg.schedule, TintSchedule::Fixed);
        assert_eq!(cfg.location, None);
        assert!(cfg.pause_for_apps && !cfg.pause_for_fullscreen);
        assert_eq!(cfg.pause_apps, DEFAULT_PAUSE_APPS);
    }

    #[test]
    fn default_pause_apps_are_distinct_exe_names() {
        let mut seen = std::collections::HashSet::new();
        for app in DEFAULT_PAUSE_APPS {
            assert!(app.to_lowercase().ends_with(".exe"), "{app}");
            assert!(seen.insert(app.to_lowercase()), "{app} listed twice");
        }
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

    // -----------------------------------------------------------------------
    // ADR-023: sunset → sunrise window
    // -----------------------------------------------------------------------

    fn sun_at(latitude: f64, longitude: f64) -> TintConfig {
        TintConfig {
            schedule: TintSchedule::Sun,
            location: Some(crate::api::TintLocation {
                latitude,
                longitude,
                city: None,
            }),
            ..evening()
        }
    }

    const WARSZAWA: (f64, f64) = (52.2297, 21.0122);
    const TROMSO: (f64, f64) = (69.6492, 18.9553);

    #[test]
    fn fixed_schedule_window_is_the_configured_hours() {
        let cfg = evening();
        let window = tint_window(&cfg, (2026, 10, 13), 120);
        assert_eq!(
            window,
            TintWindow {
                schedule: TintScheduleState::Fixed,
                start_minutes: 1080,
                end_minutes: 540
            }
        );
        // A location alone does not switch the schedule.
        let located = TintConfig {
            schedule: TintSchedule::Fixed,
            ..sun_at(WARSZAWA.0, WARSZAWA.1)
        };
        assert_eq!(tint_window(&located, (2026, 10, 13), 120), window);
        for minute in (0..MINUTES_PER_DAY).step_by(7) {
            let minute = f64::from(minute);
            assert_eq!(
                window_level(&cfg, &window, minute),
                tint_level(&cfg, minute)
            );
        }
    }

    #[test]
    fn sun_schedule_opens_at_sunset_and_closes_at_sunrise() {
        // Warsaw on 2026-10-13 (UTC+2): sunrise 06:57, sunset 17:46 (published, ±1 min).
        let cfg = sun_at(WARSZAWA.0, WARSZAWA.1);
        let window = tint_window(&cfg, (2026, 10, 13), 120);
        assert_eq!(window.schedule, TintScheduleState::Sun);
        assert!(
            window.start_minutes.abs_diff(17 * 60 + 46) <= 1,
            "{window:?}"
        );
        assert!(window.end_minutes.abs_diff(6 * 60 + 57) <= 1, "{window:?}");
        // Fades in from sunset, full at night, fades out until sunrise; nothing in daylight.
        let sunset = f64::from(window.start_minutes);
        let sunrise = f64::from(window.end_minutes);
        assert_eq!(window_level(&cfg, &window, at(12, 0)), 0);
        assert_eq!(window_level(&cfg, &window, sunset), 0);
        assert_eq!(window_level(&cfg, &window, sunset + 15.0), 23);
        assert_eq!(window_level(&cfg, &window, sunset + 30.0), 45);
        assert_eq!(window_level(&cfg, &window, at(23, 0)), 45);
        assert_eq!(window_level(&cfg, &window, at(3, 0)), 45);
        assert_eq!(window_level(&cfg, &window, sunrise - 15.0), 23);
        assert_eq!(window_level(&cfg, &window, sunrise), 0);
        // The fixed hours (18:00 → 09:00) would still be warm at 08:00; the sun schedule is not.
        assert!(tint_level(&cfg, at(8, 0)) > 0);
        assert_eq!(window_level(&cfg, &window, at(8, 0)), 0);
        // Midsummer: a short night.
        let june = tint_window(&cfg, (2026, 6, 21), 120);
        assert!(june.start_minutes.abs_diff(21 * 60 + 1) <= 1, "{june:?}");
        assert!(june.end_minutes.abs_diff(4 * 60 + 14) <= 1, "{june:?}");
        // Disabled stays off whatever the window.
        let off = TintConfig {
            enabled: false,
            ..cfg
        };
        assert_eq!(window_level(&off, &window, at(23, 0)), 0);
    }

    #[test]
    fn sun_schedule_falls_back_to_the_fixed_hours() {
        let fixed = |schedule| TintWindow {
            schedule,
            start_minutes: DEFAULT_START_MINUTES,
            end_minutes: DEFAULT_END_MINUTES,
        };
        let no_place = TintConfig {
            location: None,
            ..sun_at(0.0, 0.0)
        };
        assert_eq!(
            tint_window(&no_place, (2026, 6, 21), 0),
            fixed(TintScheduleState::NoLocation)
        );
        let tromso = sun_at(TROMSO.0, TROMSO.1);
        assert_eq!(
            tint_window(&tromso, (2026, 6, 21), 120),
            fixed(TintScheduleState::PolarDay)
        );
        assert_eq!(
            tint_window(&tromso, (2026, 12, 21), 60),
            fixed(TintScheduleState::PolarNight)
        );
        // An impossible date from the shell: the fixed hours, never a panic.
        assert_eq!(
            tint_window(&tromso, (2026, 2, 31), 60),
            fixed(TintScheduleState::NoLocation)
        );
        // The fallback window tints like the fixed schedule.
        let window = tint_window(&tromso, (2026, 12, 21), 60);
        assert_eq!(window_level(&tromso, &window, at(3, 0)), 45);
        assert_eq!(window_level(&tromso, &window, at(12, 0)), 0);
    }

    #[test]
    fn sunset_after_midnight_wraps_into_the_day() {
        // Reykjavík, 2026-06-21 (UTC): sunset about 00:04 the next day, sunrise 02:55.
        let cfg = sun_at(64.1466, -21.9426);
        let window = tint_window(&cfg, (2026, 6, 21), 0);
        assert_eq!(window.schedule, TintScheduleState::Sun);
        assert!(window.start_minutes <= 6, "{window:?}");
        assert!(window.end_minutes.abs_diff(2 * 60 + 55) <= 1, "{window:?}");
        assert_eq!(window_level(&cfg, &window, at(23, 0)), 0);
        assert!(window_level(&cfg, &window, at(1, 30)) > 0);
        assert_eq!(minute_in_day(1439.6), 0);
        assert_eq!(minute_in_day(-0.2), 0);
        assert_eq!(minute_in_day(1444.4), 4);
        assert_eq!(minute_in_day(-60.0), 1380);
    }

    // -----------------------------------------------------------------------
    // ADR-023: exceptions
    // -----------------------------------------------------------------------

    fn app_pause(app: &str) -> Option<TintPause> {
        Some(TintPause {
            reason: TintPauseReason::App,
            app: Some(app.to_owned()),
        })
    }

    #[test]
    fn colour_critical_apps_match_exact_names_case_insensitively() {
        let cfg = TintConfig::default();
        assert!(is_pause_app(&cfg, "Photoshop.exe"));
        assert!(is_pause_app(&cfg, "photoshop.EXE"));
        // Lightroom Classic's exe is lower case; one entry covers both editions.
        assert!(is_pause_app(&cfg, "lightroom.exe"));
        assert!(is_pause_app(&cfg, "Lightroom.exe"));
        assert!(is_pause_app(&cfg, "Adobe Premiere Pro.exe"));
        assert!(is_pause_app(&cfg, " krita.exe "));
        for other in [
            "",
            " ",
            "Photoshop",
            "Photoshop.exe.bak",
            "NotPhotoshop.exe",
            "C:\\Program Files\\Adobe\\Photoshop.exe",
            "Code.exe",
        ] {
            assert!(!is_pause_app(&cfg, other), "{other}");
        }
        let custom = TintConfig {
            pause_apps: vec!["Ąćę.exe".into()],
            ..TintConfig::default()
        };
        assert!(is_pause_app(&custom, "ąĆĘ.EXE"));
        assert!(!is_pause_app(&custom, "Photoshop.exe"));
    }

    #[test]
    fn pause_reason_prefers_the_app_and_respects_the_switches() {
        let cfg = TintConfig {
            pause_for_fullscreen: true,
            ..TintConfig::default()
        };
        assert_eq!(
            pause_reason(&cfg, Some("Photoshop.exe"), true),
            app_pause("Photoshop.exe")
        );
        assert_eq!(
            pause_reason(&cfg, Some("game.exe"), true),
            Some(TintPause {
                reason: TintPauseReason::Fullscreen,
                app: Some("game.exe".into())
            })
        );
        assert_eq!(
            pause_reason(&cfg, None, true),
            Some(TintPause {
                reason: TintPauseReason::Fullscreen,
                app: None
            })
        );
        assert_eq!(pause_reason(&cfg, Some("game.exe"), false), None);
        assert_eq!(pause_reason(&cfg, None, false), None);
        assert_eq!(pause_reason(&cfg, Some("  "), false), None);

        // Defaults: apps on, full screen off (games keep the warmth, ADR-015).
        let defaults = TintConfig::default();
        assert_eq!(pause_reason(&defaults, Some("game.exe"), true), None);
        assert_eq!(
            pause_reason(&defaults, Some("blender.exe"), false),
            app_pause("blender.exe")
        );
        let apps_off = TintConfig {
            pause_for_apps: false,
            ..cfg
        };
        assert_eq!(pause_reason(&apps_off, Some("Photoshop.exe"), false), None);
        assert_eq!(
            pause_reason(&apps_off, Some("Photoshop.exe"), true).map(|p| p.reason),
            Some(TintPauseReason::Fullscreen)
        );
    }

    #[test]
    fn a_pause_fades_out_at_once_and_back_after_the_hysteresis() {
        let mut ex = TintExceptions::new();
        assert!(ex.is_settled() && !ex.is_fading());
        ex.observe(0, None);
        assert_eq!(ex.level(45), 45);
        assert!(ex.is_settled());

        // Photoshop comes to the front: paused at once, the warmth fades out over PAUSE_FADE_MS from this sample on
        // (the second since the previous sample does not count).
        ex.observe(1_000, app_pause("Photoshop.exe"));
        assert_eq!(ex.paused_by(), app_pause("Photoshop.exe").as_ref());
        assert!(ex.is_fading() && !ex.is_settled());
        assert_eq!(ex.level(40), 40);
        ex.observe(1_000 + PAUSE_FADE_MS / 2, app_pause("Photoshop.exe"));
        assert_eq!(ex.level(40), 20);
        ex.observe(1_000 + PAUSE_FADE_MS, app_pause("Photoshop.exe"));
        assert_eq!(ex.level(40), 0);
        assert!(!ex.is_fading());

        // Gone (Alt+Tab, the taskbar): still paused through the hysteresis.
        let gone = 10_000;
        ex.observe(gone, None);
        ex.observe(gone + PAUSE_RELEASE_MS - 1, None);
        assert!(ex.paused_by().is_some());
        assert_eq!(ex.level(40), 0);
        // Back within the hysteresis: the countdown starts over.
        ex.observe(gone + PAUSE_RELEASE_MS - 1, app_pause("Photoshop.exe"));
        ex.observe(gone + PAUSE_RELEASE_MS + 500, None);
        ex.observe(gone + 2 * PAUSE_RELEASE_MS, None);
        assert!(ex.paused_by().is_some(), "only 1.5 s gone");

        // Gone for good: released, then the warmth fades back in.
        let released = gone + PAUSE_RELEASE_MS + 500 + PAUSE_RELEASE_MS;
        ex.observe(released, None);
        assert_eq!(ex.paused_by(), None);
        assert!(ex.is_fading());
        ex.observe(released + PAUSE_FADE_MS / 4, None);
        assert_eq!(ex.level(40), 10);
        ex.observe(released + PAUSE_FADE_MS, None);
        assert_eq!(ex.level(40), 40);
        assert!(ex.is_settled());
    }

    #[test]
    fn switching_between_exceptions_keeps_the_pause() {
        let mut ex = TintExceptions::new();
        ex.observe(0, app_pause("Photoshop.exe"));
        // The first sample jumps (nobody watched before): fully paused at once.
        assert_eq!(ex.level(45), 0);
        assert!(!ex.is_fading());
        ex.observe(500, app_pause("Illustrator.exe"));
        assert_eq!(ex.paused_by(), app_pause("Illustrator.exe").as_ref());
        let full_screen = Some(TintPause {
            reason: TintPauseReason::Fullscreen,
            app: Some("game.exe".into()),
        });
        ex.observe(900, full_screen.clone());
        assert_eq!(ex.paused_by(), full_screen.as_ref());
        assert_eq!(ex.level(45), 0);
    }

    #[test]
    fn a_fade_reverses_smoothly_and_reset_forgets() {
        let mut ex = TintExceptions::new();
        // Paused from the first sample, gone at 1 s, released at 3 s: the warmth fades back in from 0.
        ex.observe(0, app_pause("krita.exe"));
        ex.observe(1_000, None);
        ex.observe(1_000 + PAUSE_RELEASE_MS, None);
        assert_eq!(ex.paused_by(), None);
        let released = 1_000 + PAUSE_RELEASE_MS;
        ex.observe(released + PAUSE_FADE_MS / 4, None);
        assert_eq!(ex.level(100), 25);
        // Back in front halfway: fades out again from where it is, not from 100.
        let back = released + PAUSE_FADE_MS / 4;
        ex.observe(back, app_pause("krita.exe"));
        assert_eq!(ex.level(100), 25);
        ex.observe(back + PAUSE_FADE_MS / 8, app_pause("krita.exe"));
        let level = ex.level(100);
        assert!(level > 0 && level < 25, "{level}");
        // A sample out of order (never from the shell) does not move the warmth back.
        let before = ex.level(100);
        ex.observe(back, app_pause("krita.exe"));
        assert_eq!(ex.level(100), before);
        ex.observe(back + PAUSE_FADE_MS, app_pause("krita.exe"));
        assert_eq!(ex.level(100), 0);
        assert!(!ex.is_fading() && !ex.is_settled());

        ex.observe(10_000, app_pause("krita.exe"));
        ex.reset();
        assert_eq!(ex, TintExceptions::new());
        assert_eq!(ex.level(70), 70);
        assert_eq!(ex.level(250), MAX_STRENGTH);
        assert_eq!(TintExceptions::default(), TintExceptions::new());
    }
}
