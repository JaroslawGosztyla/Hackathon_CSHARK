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
/// Beta rewards (ADR-022), an exception like [`MEETING_REMINDER_LEAD_MINUTES`]: a reward day starts at 04:00 local
/// time (minute of the day), so a late evening still counts for the day it began.
pub const REWARD_DAY_START_MINUTE: u32 = 240;
/// Beta rewards (ADR-022): a focus block is "full" when it ran at least this share (percent) of the profile's focus
/// length (and at least [`TimingProfile::reward_min_block_secs`]).
pub const FULL_BLOCK_PERCENT: u32 = 80;
/// Beta rewards (ADR-022): a break counts as "taken" when it ran at least this share (percent) of its length.
pub const BREAK_TAKEN_PERCENT: u32 = 50;
/// Normal profile: one focus-time spark per this many seconds of running focus (ADR-022).
pub const SPARK_UNIT_SECS: u32 = 60;
/// Normal profile: a block shorter than this never counts as a real or full block (ADR-022), 10 minutes.
pub const REWARD_MIN_BLOCK_SECS: u32 = 600;
/// Demo profile: one focus-time spark per second.
pub const DEMO_SPARK_UNIT_SECS: u32 = 1;
/// Demo profile: the shortest real block in seconds (half the demo focus).
pub const DEMO_REWARD_MIN_BLOCK_SECS: u32 = 20;

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
    /// Beta rewards (ADR-022): one focus-time spark per this many seconds of running focus (never 0 in use).
    pub spark_unit_secs: u32,
    /// Beta rewards (ADR-022): a block that ran shorter than this is no real block (no full-block or back bonus).
    pub reward_min_block_secs: u32,
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
            spark_unit_secs: SPARK_UNIT_SECS,
            reward_min_block_secs: REWARD_MIN_BLOCK_SECS,
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
            spark_unit_secs: DEMO_SPARK_UNIT_SECS,
            reward_min_block_secs: DEMO_REWARD_MIN_BLOCK_SECS,
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
        assert_eq!((p.spark_unit_secs, p.reward_min_block_secs), (60, 600));
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
        assert_eq!((p.spark_unit_secs, p.reward_min_block_secs), (1, 20));
    }

    #[test]
    fn reward_thresholds_are_named_exceptions() {
        assert_eq!(REWARD_DAY_START_MINUTE, 4 * 60);
        assert_eq!((FULL_BLOCK_PERCENT, BREAK_TAKEN_PERCENT), (80, 50));
        // The demo minimum is a real share of the demo focus, so a full demo block can be earned.
        let demo = TimingProfile::demo();
        assert!(demo.reward_min_block_secs * 100 <= demo.focus_secs * FULL_BLOCK_PERCENT);
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
        assert_eq!((p.spark_unit_secs, p.reward_min_block_secs), (60, 600));
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
