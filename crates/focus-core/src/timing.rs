//! Phase lengths and thresholds. Every duration the session uses comes from a [`TimingProfile`]
//! (no magic numbers in the logic or in the UI).

use crate::api::Config;

/// Default focus length in minutes (45 / 15 ratio).
pub const DEFAULT_FOCUS_MINUTES: u32 = 45;
/// Default break length in minutes.
pub const DEFAULT_BREAK_MINUTES: u32 = 15;
/// A running phase is auto-paused when two observations are further apart than this (sleep, hibernate).
pub const INTERRUPTION_GAP_SECS: u32 = 90;
/// Foreground samples without keyboard/mouse input for at least this long count as idle.
pub const IDLE_AFTER_SECS: u32 = 120;
/// Demo profile (`--demo` / `FOCUSMODE_DEMO=1`): focus length in seconds.
pub const DEMO_FOCUS_SECS: u32 = 40;
/// Demo profile: break length in seconds.
pub const DEMO_BREAK_SECS: u32 = 20;
/// Demo profile: idle threshold in seconds.
pub const DEMO_IDLE_AFTER_SECS: u32 = 30;
/// "Remind later" for a meeting that collides with a break: the reminders are due this many minutes
/// before the meeting, in this order; the last one offers no "remind later" (owner's 30 → 20 → 10 rule).
pub const MEETING_REMINDER_LEAD_MINUTES: [u32; 3] = [30, 20, 10];
/// A meeting longer than this never extends a focus or holds a break (`Extend` / `PauseAtBreak` are ignored):
/// a day-long "meeting" (a conference, a blocked afternoon) must not stop the breaks for hours. 4 hours.
pub const MAX_MEETING_HOLD_MS: u64 = 4 * 60 * 60 * MS_PER_SEC;

const SECS_PER_MINUTE: u32 = 60;
const MS_PER_SEC: u64 = 1000;

/// Durations used by one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingProfile {
    /// Focus phase length in seconds.
    pub focus_secs: u32,
    /// Break phase length in seconds.
    pub break_secs: u32,
    /// Auto-pause threshold for sleep/clock gaps in seconds; 0 disables gap detection.
    pub interruption_gap_secs: u32,
    /// Idle threshold for the productivity tracker in seconds.
    pub idle_after_secs: u32,
    /// Shortened demo profile.
    pub demo: bool,
}

impl TimingProfile {
    /// 45 min focus, 15 min break.
    pub fn normal() -> Self {
        Self {
            focus_secs: DEFAULT_FOCUS_MINUTES * SECS_PER_MINUTE,
            break_secs: DEFAULT_BREAK_MINUTES * SECS_PER_MINUTE,
            interruption_gap_secs: INTERRUPTION_GAP_SECS,
            idle_after_secs: IDLE_AFTER_SECS,
            demo: false,
        }
    }

    /// 40 s focus, 20 s break (presentation mode).
    pub fn demo() -> Self {
        Self {
            focus_secs: DEMO_FOCUS_SECS,
            break_secs: DEMO_BREAK_SECS,
            interruption_gap_secs: INTERRUPTION_GAP_SECS,
            idle_after_secs: DEMO_IDLE_AFTER_SECS,
            demo: true,
        }
    }

    /// [`TimingProfile::normal`] with the focus/break minutes taken from the config.
    ///
    /// A config is validated before it is stored, so 0 minutes cannot normally arrive here;
    /// it is still treated as 1 minute so a phase can never have zero length.
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            focus_secs: minutes_to_secs(cfg.focus_minutes),
            break_secs: minutes_to_secs(cfg.break_minutes),
            ..Self::normal()
        }
    }

    /// Focus phase length in milliseconds.
    pub fn focus_ms(&self) -> u64 {
        secs_to_ms(self.focus_secs)
    }

    /// Break phase length in milliseconds.
    pub fn break_ms(&self) -> u64 {
        secs_to_ms(self.break_secs)
    }

    /// Gap detection threshold in milliseconds (0 = disabled).
    pub fn interruption_gap_ms(&self) -> u64 {
        secs_to_ms(self.interruption_gap_secs)
    }
}

impl Default for TimingProfile {
    fn default() -> Self {
        Self::normal()
    }
}

fn minutes_to_secs(minutes: u32) -> u32 {
    minutes.max(1).saturating_mul(SECS_PER_MINUTE)
}

fn secs_to_ms(secs: u32) -> u64 {
    // u32::MAX * 1000 fits comfortably in u64.
    u64::from(secs) * MS_PER_SEC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_profile_is_45_15() {
        let p = TimingProfile::normal();
        assert_eq!(p.focus_secs, 2700);
        assert_eq!(p.break_secs, 900);
        assert_eq!(p.interruption_gap_secs, 90);
        assert_eq!(p.idle_after_secs, 120);
        assert!(!p.demo);
        assert_eq!(TimingProfile::default(), p);
    }

    #[test]
    fn demo_profile_is_40_20() {
        let p = TimingProfile::demo();
        assert_eq!((p.focus_secs, p.break_secs), (40, 20));
        assert_eq!(p.idle_after_secs, 30);
        assert!(p.demo);
        assert_eq!(p.focus_ms(), 40_000);
        assert_eq!(p.break_ms(), 20_000);
        assert_eq!(p.interruption_gap_ms(), 90_000);
    }

    #[test]
    fn meeting_reminders_are_30_20_10_minutes_before() {
        assert_eq!(MEETING_REMINDER_LEAD_MINUTES, [30, 20, 10]);
    }

    #[test]
    fn meeting_holds_are_capped_at_4_hours() {
        assert_eq!(MAX_MEETING_HOLD_MS, 14_400_000);
    }

    #[test]
    fn from_config_uses_configured_minutes() {
        let cfg = Config {
            focus_minutes: 25,
            break_minutes: 5,
            ..Config::default()
        };
        let p = TimingProfile::from_config(&cfg);
        assert_eq!((p.focus_secs, p.break_secs), (1500, 300));
        assert!(!p.demo);
        assert_eq!(p.interruption_gap_secs, INTERRUPTION_GAP_SECS);
    }

    #[test]
    fn from_config_never_produces_zero_or_overflowing_phases() {
        let cfg = Config {
            focus_minutes: 0,
            break_minutes: u32::MAX,
            ..Config::default()
        };
        let p = TimingProfile::from_config(&cfg);
        assert_eq!(p.focus_secs, 60);
        assert_eq!(p.break_secs, u32::MAX);
        assert_eq!(p.break_ms(), u64::from(u32::MAX) * 1000);
    }
}
