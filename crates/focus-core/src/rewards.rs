//! Beta rewards (ADR-022): sparks, the daily goal, levels, the streak (days off, paused days and rest days), surprises
//! with a pity rule and the "focus is sacred" deferral of achievements.
//!
//! Pure: no clock, files, RNG source or network. The shell passes the reward day ([`reward_day`] from the local date
//! and time; a reward day starts at 04:00), the rules from the config ([`RewardRules::from_config`]) and the wall clock
//! for a moment's timestamp, and persists the [`Ledger`] as `rewards.json` (numbers, day numbers and session ids only:
//! never step texts, goal titles or app names). The tracker is never an input: no app is scored.
//!
//! - Every fact is credited to the reward day it arrives in (a whole block counts for the day it ended), and a clock
//!   that goes back never repeats or double-counts a day (`today = max(today, ledger.last_day)`).
//! - Nothing ever subtracts: Stop, Skip, Pause, sleep gaps, unticking and early stops only leave a bonus out, and never
//!   produce a message about it. All sums saturate.
//! - Achievements (daily goal, comeback, streak, rest days, level up, pond full, stickers) earned while focus goes on
//!   are recorded at once but announced only by the next moment outside focus (the break recap or the session end).

use serde::{Deserialize, Serialize};

use crate::api::{
    Collection, Config, DailyGoal, DayCell, DayMark, LevelView, MomentKind, Phase, PopupRewards,
    Rarity, RewardKind, RewardLine, RewardMoment, RewardsStatus, SessionSparks, StickerId,
    StreakState, StreakView, SurpriseOdds, TodayView, Weekday,
};
use crate::session::{PhaseEnd, ProgressFact};
use crate::timing::{BREAK_TAKEN_PERCENT, FULL_BLOCK_PERCENT, REWARD_DAY_START_MINUTE};

/// Days since 1970-01-01 (civil, proleptic Gregorian) of a reward day.
pub type RewardDay = i32;

/// Sparks for a Start (the first [`START_DAILY_LIMIT`] starts of a reward day).
pub const START_SPARKS: u32 = 5;
pub const START_DAILY_LIMIT: u8 = 3;
/// The first Start of a reward day.
pub const FIRST_OF_DAY_SPARKS: u32 = 5;
/// A Start with at least one small step planned (the first [`PLAN_DAILY_LIMIT`] of a day).
pub const PLAN_SPARKS: u32 = 2;
pub const PLAN_DAILY_LIMIT: u8 = 3;
/// A full block (see [`FULL_BLOCK_PERCENT`] and `TimingProfile::reward_min_block_secs`).
pub const FULL_BLOCK_SPARKS: u32 = 10;
/// A break that ran at least [`BREAK_TAKEN_PERCENT`] of its length.
pub const BREAK_TAKEN_SPARKS: u32 = 5;
/// Break → Focus after a real block.
pub const BACK_SPARKS: u32 = 5;
/// A step ticked for the first time in a session, at most [`STEP_SPARKS_DAILY_CAP`] a day.
pub const STEP_SPARKS: u32 = 3;
pub const STEP_SPARKS_DAILY_CAP: u32 = 45;
/// Every step ticked (at least [`ALL_STEPS_MIN`] steps), once per session.
pub const ALL_STEPS_SPARKS: u32 = 5;
pub const ALL_STEPS_MIN: u32 = 2;
/// "Zrobione? → Tak" for the session goal, once per session.
pub const GOAL_DONE_SPARKS: u32 = 10;
/// The daily goal reached; the bonus does not count towards the goal.
pub const DAILY_GOAL_BONUS: u32 = 10;
/// The daily goal met after an uncovered gap of at least [`COMEBACK_MIN_GAP`] work days.
pub const COMEBACK_SPARKS: u32 = 10;
pub const COMEBACK_MIN_GAP: u32 = 2;
/// Focus-time sparks per reward day; beyond it the minutes are the user's own ("pond full").
pub const MINUTE_SPARKS_DAILY_CAP: u32 = 300;
/// Sparks of a bubble, a pearl and a golden pearl.
pub const SURPRISE_SPARKS: [u32; 3] = [5, 15, 40];
/// Published odds of a full block, permille: nothing, bubble, pearl, golden pearl.
pub const ODDS_PERMILLE: [u16; 4] = [500, 320, 150, 30];
/// After this many full blocks without a pearl, the next surprise is at least a pearl.
pub const PITY_BLOCKS: u32 = 4;
/// Daily goal presets in sparks: light, regular, strong.
pub const DAILY_GOAL_SPARKS: [u32; 3] = [30, 60, 120];
/// Level spans: `min(LEVEL_BASE + LEVEL_STEP * (level - 1), LEVEL_SPAN_MAX)` sparks from a level to the next.
pub const LEVEL_BASE: u32 = 60;
pub const LEVEL_STEP: u32 = 20;
pub const LEVEL_SPAN_MAX: u32 = 500;
/// Rest days in the bank of a new ledger, the most the bank holds, and goal days per earned rest day.
pub const REST_BANK_START: u32 = 2;
pub const REST_BANK_MAX: u32 = 3;
pub const REST_EARN_EVERY: u32 = 5;
/// Streak lengths that are celebrated.
pub const STREAK_MILESTONES: [u32; 8] = [3, 7, 14, 30, 50, 100, 200, 365];
/// Reward days kept in the ledger (the newest ones).
pub const MAX_DAYS_KEPT: usize = 42;
/// Paused ranges kept (the newest ones).
pub const MAX_NEUTRAL_RANGES: usize = 20;
/// Session ids whose goal was rewarded, kept (the newest ones).
pub const MAX_GOAL_REWARDED: usize = 50;
/// A gap longer than this many days is not scanned: it counts as uncoverable.
pub const MAX_GAP_SCAN_DAYS: i32 = 400;
/// A clock going back by at most this many reward days is clamped to the last day seen; further back is taken as the
/// correction of a clock that was ahead (§14 of ADR-022).
pub const CLOCK_BACK_TOLERANCE_DAYS: RewardDay = 2;
/// `version` of `rewards.json`.
pub const LEDGER_VERSION: u32 = 1;

const MINUTES_PER_DAY: u32 = 24 * 60;
const PERMILLE: u64 = 1000;
const PERCENT: u64 = 100;
const DAYS_PER_WEEK: i32 = 7;

// ---------------------------------------------------------------------------------------------------------------
// Reward days
// ---------------------------------------------------------------------------------------------------------------

/// The reward day of a local date and time: before [`REWARD_DAY_START_MINUTE`] (04:00) it is still the previous day.
/// `None` for an invalid date or a minute outside the day.
pub fn reward_day(year: i32, month: u32, day: u32, minute_of_day: u32) -> Option<RewardDay> {
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || minute_of_day >= MINUTES_PER_DAY
    {
        return None;
    }
    let civil = days_from_civil(i64::from(year), month, day);
    let reward = if minute_of_day < REWARD_DAY_START_MINUTE {
        civil - 1
    } else {
        civil
    };
    RewardDay::try_from(reward).ok()
}

/// 0 = Monday ... 6 = Sunday (1970-01-01 was a Thursday).
pub fn weekday(day: RewardDay) -> u8 {
    u8::try_from((i64::from(day) + 3).rem_euclid(7)).unwrap_or(0)
}

/// `YYYY-MM-DD` of a reward day.
pub fn date_string(day: RewardDay) -> String {
    let (year, month, day) = civil_from_days(i64::from(day));
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a valid civil date (Howard Hinnant's `days_from_civil`).
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = i64::from((month + 9) % 12);
    let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The civil date of a day number (Howard Hinnant's `civil_from_days`).
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn weekday_index(day: Weekday) -> u8 {
    match day {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

/// Sparks of a daily goal preset.
pub fn daily_goal_sparks(goal: DailyGoal) -> u32 {
    match goal {
        DailyGoal::Light => DAILY_GOAL_SPARKS[0],
        DailyGoal::Regular => DAILY_GOAL_SPARKS[1],
        DailyGoal::Strong => DAILY_GOAL_SPARKS[2],
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Rules and ledger
// ---------------------------------------------------------------------------------------------------------------

/// What the config says about the rewards, in the form the rules need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardRules {
    /// `Config::rewards_active()`: facts are credited.
    pub active: bool,
    /// `Config::streak_counting()`: days with the daily goal count for the streak.
    pub counting: bool,
    /// Surprise rolls after full blocks.
    pub surprises: bool,
    /// The daily goal in sparks.
    pub daily_goal: u32,
    /// Days off, bit 0 = Monday.
    pub rest_weekdays: u8,
}

impl RewardRules {
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            active: cfg.rewards_active(),
            counting: cfg.streak_counting(),
            surprises: cfg.rewards.surprises,
            daily_goal: daily_goal_sparks(cfg.rewards.daily_goal),
            rest_weekdays: cfg
                .rewards
                .rest_weekdays
                .iter()
                .fold(0, |bits, day| bits | (1 << weekday_index(*day))),
        }
    }

    fn is_rest_weekday(&self, day: RewardDay) -> bool {
        self.rest_weekdays & (1 << weekday(day)) != 0
    }
}

/// The persistent state of the rewards (`rewards.json`, camelCase, every field optional). Numbers, day numbers and
/// session ids only. Not part of the IPC contract: the UI sees [`RewardsStatus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Ledger {
    pub version: u32,
    /// Seed of the surprise rolls (created once with the ledger).
    pub seed: u64,
    pub total_sparks: u32,
    /// The latest reward day a fact arrived in (a clock that goes back is clamped to it).
    pub last_day: RewardDay,
    /// The newest [`MAX_DAYS_KEPT`] days, sorted by day.
    pub days: Vec<DayRecord>,
    pub streak: StreakLedger,
    /// Paused ranges (inclusive), neutral for the streak; the newest [`MAX_NEUTRAL_RANGES`].
    pub neutral: Vec<DayRange>,
    /// Counting has been off since this day (streak paused or rewards off).
    pub paused_since: Option<RewardDay>,
    pub collection: Collection,
    pub surprises: SurpriseState,
    pub counters: Counters,
    /// P1 sticker album.
    pub stickers: Vec<StickerId>,
    /// Sessions whose goal was rewarded ("Zrobione? → Tak"), the newest [`MAX_GOAL_REWARDED`].
    pub goal_rewarded: Vec<String>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new(0)
    }
}

/// One reward day.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DayRecord {
    pub day: RewardDay,
    /// Every spark of the day, the daily-goal bonus included.
    pub sparks: u32,
    /// The bonus part of `sparks` (it does not count towards the goal).
    pub goal_bonus: u32,
    pub minute_sparks: u32,
    pub step_sparks: u32,
    pub starts: u8,
    pub plans: u8,
    pub goal_met: bool,
    /// A rest day from the bank covered this day.
    pub rest: bool,
    pub pond_full: bool,
}

/// The streak part of the ledger (the view has its own `api::StreakView`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StreakLedger {
    pub current: u32,
    pub best: u32,
    pub last_counted_day: Option<RewardDay>,
    pub rest_bank: u32,
    /// Goal days towards the next rest day (0..REST_EARN_EVERY).
    pub rest_progress: u32,
    pub active_days: u32,
}

impl Default for StreakLedger {
    fn default() -> Self {
        Self {
            current: 0,
            best: 0,
            last_counted_day: None,
            rest_bank: REST_BANK_START,
            rest_progress: 0,
            active_days: 0,
        }
    }
}

/// An inclusive range of days.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DayRange {
    pub from: RewardDay,
    pub to: RewardDay,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SurpriseState {
    /// Full blocks rolled since the last pearl or golden pearl.
    pub since_pearl: u32,
    /// The very first surprise was given (it is at least a bubble).
    pub first_done: bool,
}

/// Lifetime counts (only grow).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Counters {
    pub full_blocks: u32,
    pub breaks_taken: u32,
    pub steps_done: u32,
    pub backs: u32,
}

impl Ledger {
    /// A fresh ledger: nothing earned, [`REST_BANK_START`] rest days in the bank.
    pub fn new(seed: u64) -> Self {
        Self {
            version: LEDGER_VERSION,
            seed,
            total_sparks: 0,
            last_day: 0,
            days: Vec::new(),
            streak: StreakLedger::default(),
            neutral: Vec::new(),
            paused_since: None,
            collection: Collection::default(),
            surprises: SurpriseState::default(),
            counters: Counters::default(),
            stickers: Vec::new(),
            goal_rewarded: Vec::new(),
        }
    }

    /// Repairs a ledger read from disk: days sorted and unique (the newest [`MAX_DAYS_KEPT`]), the bank at most
    /// [`REST_BANK_MAX`], valid ranges (the newest [`MAX_NEUTRAL_RANGES`]), at most [`MAX_GOAL_REWARDED`] goal ids,
    /// unique stickers, `best >= current` and `last_day` not before any stored day.
    pub fn normalize(&mut self) {
        self.version = LEDGER_VERSION;
        self.days.sort_by_key(|record| record.day);
        self.days.dedup_by_key(|record| record.day);
        trim_front(&mut self.days, MAX_DAYS_KEPT);
        let streak = &mut self.streak;
        streak.rest_bank = streak.rest_bank.min(REST_BANK_MAX);
        streak.rest_progress = streak.rest_progress.min(REST_EARN_EVERY - 1);
        streak.best = streak.best.max(streak.current);
        self.neutral.retain(|range| range.from <= range.to);
        trim_front(&mut self.neutral, MAX_NEUTRAL_RANGES);
        trim_front(&mut self.goal_rewarded, MAX_GOAL_REWARDED);
        let mut seen = Vec::new();
        self.stickers.retain(|sticker| {
            let first = !seen.contains(sticker);
            seen.push(*sticker);
            first
        });
        if let Some(newest) = self.days.last() {
            self.last_day = self.last_day.max(newest.day);
        }
        if let Some(last) = self.streak.last_counted_day {
            self.last_day = self.last_day.max(last);
        }
    }

    /// The clock was ahead (a reward day more than [`CLOCK_BACK_TOLERANCE_DAYS`] after `day` was seen) and is right
    /// now: `day` becomes the latest day, and a streak day counted "in the future" moves back to the latest goal day
    /// on or before `day` (or the day before), so today can count again. Records are kept (sparks never go down).
    fn correct_clock(&mut self, day: RewardDay) {
        self.last_day = day;
        if self.streak.last_counted_day.is_some_and(|last| last > day) {
            let latest_goal = self
                .days
                .iter()
                .rev()
                .find(|record| record.day <= day && record.goal_met)
                .map(|record| record.day);
            self.streak.last_counted_day = Some(latest_goal.unwrap_or(day - 1));
        }
        if self.paused_since.is_some_and(|since| since > day) {
            self.paused_since = Some(day);
        }
    }

    fn record(&self, day: RewardDay) -> Option<&DayRecord> {
        self.days
            .binary_search_by_key(&day, |record| record.day)
            .ok()
            .and_then(|index| self.days.get(index))
    }

    fn record_mut(&mut self, day: RewardDay) -> &mut DayRecord {
        let index = match self.days.binary_search_by_key(&day, |record| record.day) {
            Ok(index) => index,
            Err(index) => {
                self.days.insert(
                    index,
                    DayRecord {
                        day,
                        ..DayRecord::default()
                    },
                );
                index
            }
        };
        &mut self.days[index]
    }

    /// Neutral for the streak: a day off, a paused range or a day since the pause began.
    fn is_neutral(&self, rules: &RewardRules, day: RewardDay) -> bool {
        rules.is_rest_weekday(day)
            || self
                .neutral
                .iter()
                .any(|range| range.from <= day && day <= range.to)
            || self.paused_since.is_some_and(|since| day >= since)
    }

    /// The work days strictly between `last` and `day` that nothing covers (not neutral, no rest day, no goal);
    /// `None` when the gap is longer than [`MAX_GAP_SCAN_DAYS`] (uncoverable).
    fn missing_days(
        &self,
        rules: &RewardRules,
        last: RewardDay,
        day: RewardDay,
    ) -> Option<Vec<RewardDay>> {
        if i64::from(day) - i64::from(last) - 1 > i64::from(MAX_GAP_SCAN_DAYS) {
            return None;
        }
        Some(
            (last.saturating_add(1)..day)
                .filter(|&between| {
                    !self.is_neutral(rules, between)
                        && !self
                            .record(between)
                            .is_some_and(|record| record.rest || record.goal_met)
                })
                .collect(),
        )
    }

    fn missing_count(&self, rules: &RewardRules, last: RewardDay, day: RewardDay) -> u32 {
        self.missing_days(rules, last, day)
            .map_or(u32::MAX, |days| {
                u32::try_from(days.len()).unwrap_or(u32::MAX)
            })
    }

    fn unlock(&mut self, sticker: StickerId, achievements: &mut Achievements) {
        if !self.stickers.contains(&sticker) {
            self.stickers.push(sticker);
            achievements.stickers.push(sticker);
        }
    }
}

/// The reward day the rules use for a clock that says `today`. A clock that went back by up to
/// [`CLOCK_BACK_TOLERANCE_DAYS`] (a time zone, a small correction) is clamped to the last day seen, so nothing repeats
/// or counts twice; a clock further back means the clock was ahead before, and its day is taken as it is.
fn effective_day(ledger: &Ledger, today: RewardDay) -> RewardDay {
    if today.saturating_add(CLOCK_BACK_TOLERANCE_DAYS) < ledger.last_day {
        today
    } else {
        today.max(ledger.last_day)
    }
}

/// [`effective_day`], recorded in the ledger (with [`Ledger::correct_clock`] after a clock that was ahead).
fn settle_day(ledger: &mut Ledger, today: RewardDay) -> RewardDay {
    let day = effective_day(ledger, today);
    if day < ledger.last_day {
        ledger.correct_clock(day);
    }
    ledger.last_day = day;
    day
}

fn trim_front<T>(items: &mut Vec<T>, keep: usize) {
    if items.len() > keep {
        let extra = items.len() - keep;
        items.drain(..extra);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Session tally and state
// ---------------------------------------------------------------------------------------------------------------

/// What one moment announces besides its sparks. Merged when deferred ([`Achievements::merge`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Achievements {
    pub surprise: Option<Rarity>,
    pub daily_goal: bool,
    pub comeback: bool,
    pub streak: Option<u32>,
    pub new_streak: bool,
    pub milestone: Option<u32>,
    pub rest_days_used: u32,
    pub rest_day_earned: bool,
    pub level_up: Option<u32>,
    pub levels_gained: u32,
    pub pond_full: bool,
    pub stickers: Vec<StickerId>,
}

impl Achievements {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Adds `later` (newer) to these: flags stay set, counts add up, the newest streak and the highest level win.
    fn merge(&mut self, later: Achievements) {
        self.surprise = later.surprise.or(self.surprise);
        self.daily_goal |= later.daily_goal;
        self.comeback |= later.comeback;
        self.streak = later.streak.or(self.streak);
        self.new_streak |= later.new_streak;
        self.milestone = later.milestone.or(self.milestone);
        self.rest_days_used = self.rest_days_used.saturating_add(later.rest_days_used);
        self.rest_day_earned |= later.rest_day_earned;
        self.level_up = self.level_up.max(later.level_up);
        self.levels_gained = self.levels_gained.saturating_add(later.levels_gained);
        self.pond_full |= later.pond_full;
        for sticker in later.stickers {
            if !self.stickers.contains(&sticker) {
                self.stickers.push(sticker);
            }
        }
    }
}

/// One running session (or the last one, until the next Start), in memory only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionTally {
    /// Empty for a tally created before its session was known (adopted by the next fact).
    pub session_id: String,
    pub demo: bool,
    pub sparks: u32,
    pub lines: Vec<RewardLine>,
    /// Steps paid in this session (each once).
    pub paid_steps: Vec<u32>,
    pub all_steps_paid: bool,
    /// The last focus block ran at least `reward_min_block_secs` (the back bonus needs it).
    pub last_block_real: bool,
    /// Earned while focus went on; announced by the next moment outside focus.
    pub deferred: Achievements,
    /// Lines that would reveal a deferred achievement (daily-goal bonus, comeback), shown with it.
    pub deferred_lines: Vec<RewardLine>,
    pub first_step_done: bool,
    /// The session ended (Stop).
    pub ended: bool,
    /// The rewards came on (or started again) at this wall time while the session ran: the phase running then pays
    /// only its time since (no full block, surprise or way-back bonus). Cleared when that phase ends.
    pub joined_ms: Option<i64>,
    /// While achievements wait in focus: the ledger as the view shows it (as it was before the first hidden
    /// achievement, plus the lines shown since), so the status reveals nothing early. Cleared when they are announced.
    pub shown: Option<Box<Ledger>>,
}

impl SessionTally {
    fn new(session_id: &str, demo: bool) -> Self {
        Self {
            session_id: session_id.to_owned(),
            demo,
            ..Self::default()
        }
    }

    /// A tally for a session that is already running when the rewards come on (or start again) at `now_wall_ms`;
    /// the next fact of that session adopts it. Accrual starts now: no retroactive Start bonus or full block.
    pub fn joined(now_wall_ms: i64) -> Self {
        Self {
            joined_ms: Some(now_wall_ms),
            ..Self::default()
        }
    }

    /// The running time of a phase that ran `ran_secs`, counting only from the moment the rewards joined (if they did).
    fn joined_secs(&self, ran_secs: u32, now_wall_ms: i64) -> u32 {
        self.joined_ms.map_or(ran_secs, |joined| {
            let since = now_wall_ms.saturating_sub(joined).max(0) / 1000;
            ran_secs.min(u32::try_from(since).unwrap_or(u32::MAX))
        })
    }
}

/// The ledger plus what lives only in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardsState {
    pub ledger: Ledger,
    pub session: Option<SessionTally>,
    /// Id of the next moment (starts at 1).
    pub next_moment_id: u32,
}

impl RewardsState {
    pub fn new(ledger: Ledger) -> Self {
        Self {
            ledger,
            session: None,
            next_moment_id: 1,
        }
    }
}

/// Which ledger a fact goes to: the real one (demo facts are ignored) or the demo preview (everything applies).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerMode {
    Real,
    Preview,
}

// ---------------------------------------------------------------------------------------------------------------
// Applying facts
// ---------------------------------------------------------------------------------------------------------------

/// One step of crediting: the lines and achievements of the moment being built.
struct Pass<'a> {
    ledger: &'a mut Ledger,
    today: RewardDay,
    rules: &'a RewardRules,
    lines: Vec<RewardLine>,
    achievements: Achievements,
}

impl<'a> Pass<'a> {
    fn new(ledger: &'a mut Ledger, today: RewardDay, rules: &'a RewardRules) -> Self {
        Self {
            ledger,
            today,
            rules,
            lines: Vec::new(),
            achievements: Achievements::default(),
        }
    }

    fn day(&mut self) -> &mut DayRecord {
        self.ledger.record_mut(self.today)
    }

    /// Adds `sparks` (worth `count` items) of `kind` to the day, the total and the moment; a level up and the daily
    /// goal are checked after every credit.
    fn credit(&mut self, kind: RewardKind, sparks: u32, count: u32) {
        if sparks == 0 {
            return;
        }
        let before = level_for(self.ledger.total_sparks).level;
        self.ledger.total_sparks = self.ledger.total_sparks.saturating_add(sparks);
        let day = self.day();
        day.sparks = day.sparks.saturating_add(sparks);
        add_line(&mut self.lines, kind, sparks, count);
        let after = level_for(self.ledger.total_sparks).level;
        if after > before {
            let achievements = &mut self.achievements;
            achievements.level_up = achievements.level_up.max(Some(after));
            achievements.levels_gained = achievements.levels_gained.saturating_add(after - before);
            if after >= LEVEL5_STICKER_LEVEL {
                self.ledger
                    .unlock(StickerId::Level5, &mut self.achievements);
            }
            if after >= GOLDEN_STICKER_LEVEL {
                self.ledger
                    .unlock(StickerId::GoldenPearl, &mut self.achievements);
            }
        }
        if !matches!(kind, RewardKind::DailyGoal | RewardKind::Comeback) {
            self.check_goal();
        }
    }

    /// The daily goal is met once a day: the bonus, then the streak (while counting).
    fn check_goal(&mut self) {
        let goal = self.rules.daily_goal;
        let day = self.day();
        if day.goal_met || day.sparks.saturating_sub(day.goal_bonus) < goal {
            return;
        }
        day.goal_met = true;
        day.goal_bonus = day.goal_bonus.saturating_add(DAILY_GOAL_BONUS);
        self.achievements.daily_goal = true;
        self.ledger
            .unlock(StickerId::DailyGoal, &mut self.achievements);
        self.credit(RewardKind::DailyGoal, DAILY_GOAL_BONUS, 1);
        if self.rules.counting {
            self.count_day();
        }
    }

    /// Counts today for the streak (once): +1 after a covered gap, a new streak after an uncovered one.
    fn count_day(&mut self) {
        let today = self.today;
        let mut comeback = false;
        match self.ledger.streak.last_counted_day {
            None => self.ledger.streak.current = 1,
            Some(last) if today <= last => return,
            Some(last) => {
                let missing = self.ledger.missing_days(self.rules, last, today);
                let bank = self.ledger.streak.rest_bank;
                match missing {
                    Some(days) if days.is_empty() => {
                        self.ledger.streak.current = self.ledger.streak.current.saturating_add(1);
                    }
                    Some(days) if u32::try_from(days.len()).is_ok_and(|count| count <= bank) => {
                        let used = u32::try_from(days.len()).unwrap_or(bank);
                        self.ledger.streak.rest_bank = bank - used;
                        for day in days {
                            self.ledger.record_mut(day).rest = true;
                        }
                        self.ledger.streak.current = self.ledger.streak.current.saturating_add(1);
                        self.achievements.rest_days_used =
                            self.achievements.rest_days_used.saturating_add(used);
                        self.ledger
                            .unlock(StickerId::RestDay, &mut self.achievements);
                    }
                    missing => {
                        // A new start: the bank is kept, the days away are never mentioned.
                        self.ledger.streak.current = 1;
                        self.achievements.new_streak = true;
                        let gap = missing.map_or(u32::MAX, |days| {
                            u32::try_from(days.len()).unwrap_or(u32::MAX)
                        });
                        comeback = gap >= COMEBACK_MIN_GAP;
                    }
                }
            }
        }
        let streak = &mut self.ledger.streak;
        streak.best = streak.best.max(streak.current);
        streak.active_days = streak.active_days.saturating_add(1);
        streak.last_counted_day = Some(today);
        streak.rest_progress = streak.rest_progress.saturating_add(1);
        if streak.rest_progress >= REST_EARN_EVERY {
            streak.rest_progress = 0;
            if streak.rest_bank < REST_BANK_MAX {
                streak.rest_bank += 1;
                self.achievements.rest_day_earned = true;
            }
        }
        let current = streak.current;
        let active_days = streak.active_days;
        if STREAK_MILESTONES.contains(&current) {
            self.achievements.milestone = Some(current);
        }
        self.achievements.streak = Some(current);
        if current >= THREE_DAYS_STICKER_STREAK {
            self.ledger
                .unlock(StickerId::ThreeDays, &mut self.achievements);
        }
        if active_days >= REST_DAY_STICKER_ACTIVE_DAYS {
            self.ledger
                .unlock(StickerId::RestDay, &mut self.achievements);
        }
        if comeback {
            self.achievements.comeback = true;
            self.ledger
                .unlock(StickerId::Comeback, &mut self.achievements);
            self.credit(RewardKind::Comeback, COMEBACK_SPARKS, 1);
        }
    }

    fn finish(self) -> (Vec<RewardLine>, Achievements) {
        let ledger = self.ledger;
        trim_front(&mut ledger.days, MAX_DAYS_KEPT);
        (self.lines, self.achievements)
    }
}

// P1 sticker album thresholds (deterministic, no luck required).
const PLANNER_STICKER_STEPS: u32 = 3;
const GOOD_BREAK_STICKER_BREAKS: u32 = 5;
const AMAZED_STICKER_FULL_BLOCKS: u32 = 15;
const TOGETHER_STICKER_FULL_BLOCKS: u32 = 10;
const REST_DAY_STICKER_ACTIVE_DAYS: u32 = 14;
const THREE_DAYS_STICKER_STREAK: u32 = 3;
const GOLDEN_STICKER_LEVEL: u32 = 10;
const LEVEL5_STICKER_LEVEL: u32 = 5;

fn add_line(lines: &mut Vec<RewardLine>, kind: RewardKind, sparks: u32, count: u32) {
    match lines.iter_mut().find(|line| line.kind == kind) {
        Some(line) => {
            line.sparks = line.sparks.saturating_add(sparks);
            line.count = line.count.saturating_add(count);
        }
        None => lines.push(RewardLine {
            kind,
            sparks,
            count,
        }),
    }
}

/// Lines that would announce a deferred achievement during focus.
fn reveals_achievement(kind: RewardKind) -> bool {
    matches!(kind, RewardKind::DailyGoal | RewardKind::Comeback)
}

/// The tally of `session_id`: the current one, an adoptable one (created before its session was known), or a new
/// one (rewards turned on mid-session: no retroactive Start bonus, and the running phase pays only from now on).
fn tally_for<'a>(
    session: &'a mut Option<SessionTally>,
    session_id: &str,
    demo: bool,
    now_wall_ms: i64,
) -> &'a mut SessionTally {
    let reuse = session.as_ref().is_some_and(|tally| {
        tally.session_id == session_id || (tally.session_id.is_empty() && !tally.ended)
    });
    if !reuse {
        *session = None;
    }
    let tally = session.get_or_insert_with(|| SessionTally {
        joined_ms: Some(now_wall_ms),
        ..SessionTally::new(session_id, demo)
    });
    if tally.session_id.is_empty() {
        tally.session_id = session_id.to_owned();
        tally.demo = demo;
    }
    tally
}

/// Moment details that are not lines or achievements.
#[derive(Debug, Clone, Copy, Default)]
struct Extras {
    first_step: bool,
    quiet: bool,
    /// The lines belong to the session in `state.session` (adds them to its tally).
    own_session: bool,
}

/// Adds lines that are shown during focus to the masked ledger of the view ([`SessionTally::shown`]).
fn add_shown(shown: &mut Ledger, today: RewardDay, lines: &[RewardLine]) {
    shown.last_day = shown.last_day.max(today);
    for line in lines {
        shown.total_sparks = shown.total_sparks.saturating_add(line.sparks);
        let day = shown.record_mut(today);
        day.sparks = day.sparks.saturating_add(line.sparks);
    }
}

/// Builds the moment of one fact. While focus goes on (`phase_after == Focus`) the achievements and the lines that
/// would reveal them wait in the tally, and the view keeps showing `before` (the ledger before this fact) plus the
/// lines shown; otherwise everything that waited is announced now. The announced lines count for the session's
/// tally (`own_session`). `None` when there is nothing to show (a step tick always shows: the mascot reacts).
#[allow(clippy::too_many_arguments)]
fn emit(
    session: &mut Option<SessionTally>,
    next_moment_id: &mut u32,
    kind: MomentKind,
    lines: Vec<RewardLine>,
    achievements: Achievements,
    phase_after: Phase,
    extras: Extras,
    mode: LedgerMode,
    now_wall_ms: i64,
    before: Ledger,
    today: RewardDay,
) -> Option<RewardMoment> {
    let (lines, shown) = if phase_after == Phase::Focus {
        // Focus runs, so an ended tally belongs to an earlier session: the achievements wait in a tally the running
        // session adopts with its next fact.
        if session.as_ref().is_some_and(|tally| tally.ended) {
            *session = None;
        }
        let tally = session.get_or_insert_with(SessionTally::default);
        let (hidden, shown_lines): (Vec<RewardLine>, Vec<RewardLine>) = lines
            .into_iter()
            .partition(|line| reveals_achievement(line.kind));
        let hides = !hidden.is_empty() || !achievements.is_empty();
        for line in hidden {
            add_line(
                &mut tally.deferred_lines,
                line.kind,
                line.sparks,
                line.count,
            );
        }
        tally.deferred.merge(achievements);
        match tally.shown.as_deref_mut() {
            Some(masked) => add_shown(masked, today, &shown_lines),
            None if hides => {
                let mut masked = before;
                add_shown(&mut masked, today, &shown_lines);
                tally.shown = Some(Box::new(masked));
            }
            None => {}
        }
        (shown_lines, Achievements::default())
    } else {
        let (mut all, mut all_lines) = match session.as_mut() {
            Some(tally) => {
                tally.shown = None;
                (
                    std::mem::take(&mut tally.deferred),
                    std::mem::take(&mut tally.deferred_lines),
                )
            }
            None => (Achievements::default(), Vec::new()),
        };
        all.merge(achievements);
        for line in lines {
            add_line(&mut all_lines, line.kind, line.sparks, line.count);
        }
        (all_lines, all)
    };
    if extras.own_session
        && let Some(tally) = session.as_mut()
    {
        for line in &lines {
            tally.sparks = tally.sparks.saturating_add(line.sparks);
            add_line(&mut tally.lines, line.kind, line.sparks, line.count);
        }
    }
    let session_total = if extras.own_session {
        session.as_ref().map_or(0, |tally| tally.sparks)
    } else {
        0
    };
    let tick = matches!(kind, MomentKind::Step | MomentKind::AllSteps);
    if lines.is_empty() && shown.is_empty() && !tick {
        return None;
    }
    let id = *next_moment_id;
    *next_moment_id = next_moment_id.wrapping_add(1).max(1);
    Some(RewardMoment {
        id,
        kind,
        at_ms: now_wall_ms,
        sparks: lines
            .iter()
            .fold(0, |sum: u32, line| sum.saturating_add(line.sparks)),
        session_total,
        lines,
        surprise: shown.surprise,
        daily_goal: shown.daily_goal,
        comeback: shown.comeback,
        streak: shown.streak,
        new_streak: shown.new_streak,
        milestone: shown.milestone,
        rest_days_used: shown.rest_days_used,
        rest_day_earned: shown.rest_day_earned,
        level_up: shown.level_up,
        levels_gained: shown.levels_gained,
        pond_full: shown.pond_full,
        stickers: shown.stickers,
        first_step: extras.first_step,
        quiet: extras.quiet,
        preview: mode == LedgerMode::Preview,
        sample: false,
    })
}

/// Applies one session fact at reward day `today` and returns the moment to show (`None` when nothing happened).
/// Inactive rules credit nothing; [`LedgerMode::Real`] ignores demo facts (they belong to the preview).
pub fn apply(
    state: &mut RewardsState,
    today: RewardDay,
    rules: &RewardRules,
    mode: LedgerMode,
    fact: &ProgressFact,
    now_wall_ms: i64,
) -> Option<RewardMoment> {
    let profile = *fact.profile();
    if !rules.active || (mode == LedgerMode::Real && profile.demo) {
        return None;
    }
    let RewardsState {
        ledger,
        session,
        next_moment_id,
    } = state;
    let today = settle_day(ledger, today);
    let before = ledger.clone();
    let mut extras = Extras {
        own_session: true,
        ..Extras::default()
    };

    let (kind, phase_after, lines, achievements) = match fact {
        ProgressFact::Started {
            session_id, steps, ..
        } => {
            *session = Some(SessionTally::new(session_id, profile.demo));
            let mut pass = Pass::new(ledger, today, rules);
            let day = pass.day();
            day.starts = day.starts.saturating_add(1);
            let starts = day.starts;
            if starts <= START_DAILY_LIMIT {
                pass.credit(RewardKind::Start, START_SPARKS, 1);
            }
            if starts == 1 {
                pass.credit(RewardKind::FirstOfDay, FIRST_OF_DAY_SPARKS, 1);
            }
            if *steps >= 1 {
                let day = pass.day();
                if day.plans < PLAN_DAILY_LIMIT {
                    day.plans += 1;
                    pass.credit(RewardKind::Plan, PLAN_SPARKS, 1);
                }
            }
            if *steps >= PLANNER_STICKER_STEPS {
                pass.ledger
                    .unlock(StickerId::Planner, &mut pass.achievements);
            }
            let (lines, achievements) = pass.finish();
            (MomentKind::Start, Phase::Focus, lines, achievements)
        }
        ProgressFact::FocusEnded {
            session_id,
            cycle,
            ran_secs,
            how,
            held,
            ..
        } => {
            let tally = tally_for(session, session_id, profile.demo, now_wall_ms);
            // A block the rewards joined midway pays only its minutes since; it is never a full or real block.
            let joined = tally.joined_ms.is_some();
            let counted_secs = tally.joined_secs(*ran_secs, now_wall_ms);
            tally.joined_ms = None;
            let mut pass = Pass::new(ledger, today, rules);
            let units = counted_secs / profile.spark_unit_secs.max(1);
            let day = pass.day();
            let paid = units.min(MINUTE_SPARKS_DAILY_CAP.saturating_sub(day.minute_sparks));
            day.minute_sparks = day.minute_sparks.saturating_add(paid);
            let pond_full = day.minute_sparks >= MINUTE_SPARKS_DAILY_CAP && !day.pond_full;
            if pond_full {
                day.pond_full = true;
                pass.achievements.pond_full = true;
            }
            pass.credit(RewardKind::FocusTime, paid, paid);

            let real = !joined && *ran_secs >= profile.reward_min_block_secs;
            let full = real
                && u64::from(*ran_secs) * PERCENT
                    >= u64::from(profile.focus_secs) * u64::from(FULL_BLOCK_PERCENT);
            if full {
                pass.credit(RewardKind::FullBlock, FULL_BLOCK_SPARKS, 1);
                let counters = &mut pass.ledger.counters;
                counters.full_blocks = counters.full_blocks.saturating_add(1);
                let full_blocks = counters.full_blocks;
                if rules.surprises {
                    roll_and_credit(&mut pass, session_id, *cycle);
                }
                if full_blocks >= TOGETHER_STICKER_FULL_BLOCKS {
                    pass.ledger
                        .unlock(StickerId::Together, &mut pass.achievements);
                }
                if full_blocks >= AMAZED_STICKER_FULL_BLOCKS {
                    pass.ledger
                        .unlock(StickerId::Amazed, &mut pass.achievements);
                }
            }
            tally.last_block_real = real;
            extras.quiet = *held;
            let (lines, achievements) = pass.finish();
            if *how == PhaseEnd::Stopped {
                tally.ended = true;
                (MomentKind::SessionEnd, Phase::Idle, lines, achievements)
            } else {
                (MomentKind::BreakStart, Phase::Break, lines, achievements)
            }
        }
        ProgressFact::BreakEnded {
            session_id,
            ran_secs,
            how,
            ..
        } => {
            let tally = tally_for(session, session_id, profile.demo, now_wall_ms);
            // A break the rewards joined midway counts only its time since.
            let counted_secs = tally.joined_secs(*ran_secs, now_wall_ms);
            tally.joined_ms = None;
            let mut pass = Pass::new(ledger, today, rules);
            if u64::from(counted_secs) * PERCENT
                >= u64::from(profile.break_secs) * u64::from(BREAK_TAKEN_PERCENT)
            {
                pass.credit(RewardKind::BreakTaken, BREAK_TAKEN_SPARKS, 1);
                let counters = &mut pass.ledger.counters;
                counters.breaks_taken = counters.breaks_taken.saturating_add(1);
                if counters.breaks_taken >= GOOD_BREAK_STICKER_BREAKS {
                    pass.ledger
                        .unlock(StickerId::GoodBreak, &mut pass.achievements);
                }
            }
            if *how == PhaseEnd::Stopped {
                tally.ended = true;
                let (lines, achievements) = pass.finish();
                (MomentKind::SessionEnd, Phase::Idle, lines, achievements)
            } else {
                if tally.last_block_real {
                    pass.credit(RewardKind::BackFromBreak, BACK_SPARKS, 1);
                    let counters = &mut pass.ledger.counters;
                    counters.backs = counters.backs.saturating_add(1);
                    pass.ledger
                        .unlock(StickerId::Comeback, &mut pass.achievements);
                }
                tally.last_block_real = false;
                let (lines, achievements) = pass.finish();
                (MomentKind::BackFromBreak, Phase::Focus, lines, achievements)
            }
        }
        ProgressFact::StepDone {
            session_id,
            step_id,
            first_time,
            done,
            total,
            phase,
            ..
        } => {
            let tally = tally_for(session, session_id, profile.demo, now_wall_ms);
            let mut pass = Pass::new(ledger, today, rules);
            let mut kind = MomentKind::Step;
            if *first_time && !tally.paid_steps.contains(step_id) {
                tally.paid_steps.push(*step_id);
                let counters = &mut pass.ledger.counters;
                counters.steps_done = counters.steps_done.saturating_add(1);
                let day = pass.day();
                let paid = STEP_SPARKS.min(STEP_SPARKS_DAILY_CAP.saturating_sub(day.step_sparks));
                day.step_sparks = day.step_sparks.saturating_add(paid);
                pass.credit(RewardKind::Step, paid, 1);
                pass.ledger
                    .unlock(StickerId::FirstStep, &mut pass.achievements);
                extras.first_step = !tally.first_step_done;
                tally.first_step_done = true;
            }
            // Whichever tick completes the plan (a tick again after an untick too), once per session.
            if done == total && *total >= ALL_STEPS_MIN && !tally.all_steps_paid {
                tally.all_steps_paid = true;
                pass.credit(RewardKind::AllSteps, ALL_STEPS_SPARKS, 1);
                kind = MomentKind::AllSteps;
            }
            let (lines, achievements) = pass.finish();
            (kind, *phase, lines, achievements)
        }
    };
    emit(
        session,
        next_moment_id,
        kind,
        lines,
        achievements,
        phase_after,
        extras,
        mode,
        now_wall_ms,
        before,
        today,
    )
}

/// Rolls the surprise of a full block and credits it (pity counter, first-ever rule, collection).
fn roll_and_credit(pass: &mut Pass<'_>, session_id: &str, cycle: u32) {
    let surprises = pass.ledger.surprises;
    let rarity = roll_surprise(
        pass.ledger.seed,
        session_id,
        cycle,
        surprises.since_pearl,
        surprises.first_done,
    );
    let state = &mut pass.ledger.surprises;
    state.since_pearl = match rarity {
        Some(Rarity::Pearl | Rarity::GoldenPearl) => 0,
        _ => state.since_pearl.saturating_add(1),
    };
    let Some(rarity) = rarity else {
        return;
    };
    state.first_done = true;
    let collection = &mut pass.ledger.collection;
    let sparks = match rarity {
        Rarity::Bubble => {
            collection.bubbles = collection.bubbles.saturating_add(1);
            SURPRISE_SPARKS[0]
        }
        Rarity::Pearl => {
            collection.pearls = collection.pearls.saturating_add(1);
            SURPRISE_SPARKS[1]
        }
        Rarity::GoldenPearl => {
            collection.golden = collection.golden.saturating_add(1);
            SURPRISE_SPARKS[2]
        }
    };
    pass.achievements.surprise = Some(rarity);
    if matches!(rarity, Rarity::Pearl | Rarity::GoldenPearl) {
        pass.ledger
            .unlock(StickerId::Amazed, &mut pass.achievements);
    }
    if rarity == Rarity::GoldenPearl {
        pass.ledger
            .unlock(StickerId::GoldenPearl, &mut pass.achievements);
    }
    pass.credit(RewardKind::Surprise, sparks, 1);
}

/// "Zrobione? → Tak" for the goal of session `session_id` (the latest summary of this app run): +10 once per session
/// id. `phase` = the phase now (a new session may already run). A demo session's answer goes to the preview
/// ([`LedgerMode::Preview`]); the shell never passes a demo session with [`LedgerMode::Real`].
pub fn goal_done(
    state: &mut RewardsState,
    today: RewardDay,
    rules: &RewardRules,
    mode: LedgerMode,
    session_id: &str,
    phase: Phase,
    now_wall_ms: i64,
) -> Option<RewardMoment> {
    if !rules.active
        || session_id.is_empty()
        || state.ledger.goal_rewarded.iter().any(|id| id == session_id)
    {
        return None;
    }
    let RewardsState {
        ledger,
        session,
        next_moment_id,
    } = state;
    let today = settle_day(ledger, today);
    let before = ledger.clone();
    ledger.goal_rewarded.push(session_id.to_owned());
    trim_front(&mut ledger.goal_rewarded, MAX_GOAL_REWARDED);
    let mut pass = Pass::new(ledger, today, rules);
    pass.credit(RewardKind::GoalDone, GOAL_DONE_SPARKS, 1);
    let (lines, achievements) = pass.finish();
    let own_session = session
        .as_ref()
        .is_some_and(|tally| tally.session_id == session_id);
    emit(
        session,
        next_moment_id,
        MomentKind::GoalDone,
        lines,
        achievements,
        phase,
        Extras {
            own_session,
            ..Extras::default()
        },
        mode,
        now_wall_ms,
        before,
        today,
    )
}

/// The config changed from `old` to `new` rules. Counting off starts a pause (`paused_since`) when there is a streak to
/// protect (without a counted day a pause means nothing, so a ledger nobody used never changes); counting on again
/// turns the paused days into a neutral range (a pause that began today leaves none). While the rewards are on, a lower
/// daily goal can meet today at once (an `Update` moment; deferred in focus) and a day already met but not counted
/// (met while paused) counts now. A higher goal never un-meets a day. Nothing changes when nothing applies.
pub fn config_changed(
    state: &mut RewardsState,
    today: RewardDay,
    old: &RewardRules,
    new: &RewardRules,
    phase: Phase,
    now_wall_ms: i64,
) -> Option<RewardMoment> {
    let RewardsState {
        ledger,
        session,
        next_moment_id,
    } = state;
    let today = effective_day(ledger, today);
    if today < ledger.last_day {
        ledger.correct_clock(today);
    }
    if !old.active && new.active && phase != Phase::Idle {
        // On again while a session runs: its facts were dropped until now, so accrual starts now.
        *session = Some(SessionTally::joined(now_wall_ms));
    }
    if old.counting
        && !new.counting
        && ledger.paused_since.is_none()
        && ledger.streak.last_counted_day.is_some()
    {
        ledger.paused_since = Some(today);
        ledger.last_day = today;
    }
    let mut resumed = false;
    if !old.counting && new.counting {
        if let Some(since) = ledger.paused_since.take() {
            if since < today {
                ledger.neutral.push(DayRange {
                    from: since,
                    to: today - 1,
                });
                trim_front(&mut ledger.neutral, MAX_NEUTRAL_RANGES);
            }
            ledger.last_day = today;
        }
        resumed = true;
    }
    if !new.active {
        return None;
    }
    let record = ledger.record(today)?;
    let can_meet =
        !record.goal_met && record.sparks.saturating_sub(record.goal_bonus) >= new.daily_goal;
    let uncounted =
        resumed && new.counting && record.goal_met && ledger.streak.last_counted_day != Some(today);
    if !can_meet && !uncounted {
        return None;
    }
    ledger.last_day = today;
    let before = ledger.clone();
    let mut pass = Pass::new(ledger, today, new);
    if can_meet {
        pass.check_goal();
    } else {
        pass.count_day();
    }
    let (lines, achievements) = pass.finish();
    emit(
        session,
        next_moment_id,
        MomentKind::Update,
        lines,
        achievements,
        phase,
        Extras {
            own_session: phase != Phase::Idle,
            ..Extras::default()
        },
        LedgerMode::Real,
        now_wall_ms,
        before,
        today,
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Surprises and levels
// ---------------------------------------------------------------------------------------------------------------

/// The surprise of one full block, deterministic per (seed, session, cycle): golden pearl, pearl, bubble or nothing
/// with the [`ODDS_PERMILLE`] odds; at least a pearl after [`PITY_BLOCKS`] blocks without one, and the first surprise
/// ever is at least a bubble.
pub fn roll_surprise(
    seed: u64,
    session_id: &str,
    cycle: u32,
    since_pearl: u32,
    first_done: bool,
) -> Option<Rarity> {
    let roll = splitmix64(seed ^ fnv1a64(session_id.as_bytes()) ^ u64::from(cycle)) % PERMILLE;
    let [_, bubble, pearl, golden] = ODDS_PERMILLE.map(u64::from);
    let mut rarity = if roll < golden {
        Some(Rarity::GoldenPearl)
    } else if roll < golden + pearl {
        Some(Rarity::Pearl)
    } else if roll < golden + pearl + bubble {
        Some(Rarity::Bubble)
    } else {
        None
    };
    if since_pearl >= PITY_BLOCKS && !matches!(rarity, Some(Rarity::Pearl | Rarity::GoldenPearl)) {
        rarity = Some(Rarity::Pearl);
    }
    if !first_done && rarity.is_none() {
        rarity = Some(Rarity::Bubble);
    }
    rarity
}

fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

/// The published odds ("?" popover).
pub fn odds() -> SurpriseOdds {
    SurpriseOdds {
        none_permille: ODDS_PERMILLE[0],
        bubble_permille: ODDS_PERMILLE[1],
        pearl_permille: ODDS_PERMILLE[2],
        golden_permille: ODDS_PERMILLE[3],
        pity_blocks: PITY_BLOCKS,
    }
}

/// Sparks from `level` to the next.
fn level_span(level: u32) -> u32 {
    LEVEL_BASE
        .saturating_add(LEVEL_STEP.saturating_mul(level.saturating_sub(1)))
        .min(LEVEL_SPAN_MAX)
}

/// The level of `total` sparks (1-based, no cap) and the progress towards the next one.
pub fn level_for(total: u32) -> LevelView {
    let mut level = 1_u32;
    let mut into = total;
    loop {
        let span = level_span(level);
        if span >= LEVEL_SPAN_MAX {
            // Every further level is the same span: no loop over thousands of levels.
            return LevelView {
                level: level.saturating_add(into / span),
                into: into % span,
                span,
            };
        }
        if into < span {
            return LevelView { level, into, span };
        }
        into -= span;
        level += 1;
    }
}

/// The reward line of a phase popup.
pub fn popup_rewards(moment: &RewardMoment) -> PopupRewards {
    PopupRewards {
        sparks: moment.sparks,
        daily_goal: moment.daily_goal,
        streak: moment.streak,
        level_up: moment.level_up,
        surprise: moment.surprise,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------------------------------------------

/// Everything in [`RewardsStatus`] except `revision`, `preview` and `pending` (the shell fills those). Read-only.
pub fn view(state: &RewardsState, today: RewardDay, rules: &RewardRules) -> RewardsStatus {
    let today = effective_day(&state.ledger, today);
    // While achievements wait in focus, the view shows the ledger as it was before them (plus the lines shown since):
    // no early goal, streak, level or bonus sparks (§3.7). Pause bookkeeping is always current.
    let waiting = state
        .session
        .as_ref()
        .filter(|tally| !tally.ended)
        .and_then(|tally| tally.shown.as_deref().map(|shown| (tally, shown)));
    let masked = waiting.map(|(_, shown)| Ledger {
        paused_since: state.ledger.paused_since,
        neutral: state.ledger.neutral.clone(),
        ..shown.clone()
    });
    let ledger = masked.as_ref().unwrap_or(&state.ledger);
    let mut level = level_for(ledger.total_sparks);
    if let Some((tally, _)) = waiting
        && let Some(up) = tally.deferred.level_up
    {
        // A level reached by a line shown in focus waits too: the bar shows the level before it, full.
        let before = up.saturating_sub(tally.deferred.levels_gained).max(1);
        if level.level > before {
            let span = level_span(before);
            level = LevelView {
                level: before,
                into: span,
                span,
            };
        }
    }
    let record = ledger.record(today);
    RewardsStatus {
        revision: 0,
        active: rules.active,
        preview: false,
        total_sparks: ledger.total_sparks,
        level,
        today: TodayView {
            date: date_string(today),
            sparks: record.map_or(0, |record| record.sparks),
            goal: rules.daily_goal,
            goal_met: record.is_some_and(|record| record.goal_met),
            pond_full: record.is_some_and(|record| record.pond_full),
        },
        streak: streak_view(ledger, rules, today),
        week: week(ledger, rules, today),
        session: state
            .session
            .as_ref()
            .filter(|tally| !tally.session_id.is_empty())
            .map(|tally| SessionSparks {
                session_id: tally.session_id.clone(),
                sparks: tally.sparks,
                lines: tally.lines.clone(),
            }),
        collection: ledger.collection,
        odds: odds(),
        stickers: ledger.stickers.clone(),
        pending: Vec::new(),
    }
}

fn streak_view(ledger: &Ledger, rules: &RewardRules, today: RewardDay) -> StreakView {
    let streak = &ledger.streak;
    let mut view = StreakView {
        state: StreakState::First,
        days: 0,
        best: streak.best,
        active_days: streak.active_days,
        today_counted: false,
        rest_bank: streak.rest_bank,
        rest_bank_max: REST_BANK_MAX,
        rest_reserved: 0,
        next_rest_in: REST_EARN_EVERY - streak.rest_progress.min(REST_EARN_EVERY - 1),
    };
    let Some(last) = streak.last_counted_day else {
        return view;
    };
    view.today_counted = last == today;
    if ledger.paused_since.is_some() || !rules.counting {
        // Paused: the streak as it stood when the pause began. One that was already waiting stays waiting (a pause
        // never brings an old number back only to take it away again).
        let since = ledger.paused_since.unwrap_or(today);
        let missing = ledger.missing_count(rules, last, since);
        if missing <= streak.rest_bank {
            view.state = StreakState::Paused;
            view.days = streak.current;
            view.rest_reserved = missing;
        } else {
            view.state = StreakState::Waiting;
        }
        return view;
    }
    let missing = ledger.missing_count(rules, last, today);
    if missing <= streak.rest_bank {
        view.state = StreakState::Alive;
        view.days = streak.current;
        view.rest_reserved = missing;
    } else {
        view.state = StreakState::Waiting;
    }
    view
}

fn week(ledger: &Ledger, rules: &RewardRules, today: RewardDay) -> Vec<DayCell> {
    let monday = today - RewardDay::from(weekday(today));
    (monday..monday + DAYS_PER_WEEK)
        .map(|day| {
            let record = ledger.record(day);
            let mark = if day > today {
                DayMark::Future
            } else if record.is_some_and(|record| record.goal_met) {
                DayMark::Goal
            } else if day == today {
                DayMark::Today
            } else if record.is_some_and(|record| record.rest) {
                DayMark::Rest
            } else if ledger.is_neutral(rules, day) {
                DayMark::Neutral
            } else {
                DayMark::Quiet
            };
            DayCell {
                date: date_string(day),
                mark,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::TimingProfile;

    /// Monday 2026-10-12.
    const MON: RewardDay = 20_738;
    const NOW: i64 = 1_791_885_600_000;
    const SEED: u64 = 0x1234_5678_9ABC_DEF0;

    fn rules() -> RewardRules {
        RewardRules::from_config(&Config {
            beta_features: true,
            ..Config::default()
        })
    }

    fn no_surprises() -> RewardRules {
        RewardRules {
            surprises: false,
            ..rules()
        }
    }

    fn fresh() -> RewardsState {
        RewardsState::new(Ledger::new(SEED))
    }

    fn normal() -> TimingProfile {
        TimingProfile::normal()
    }

    fn started(id: &str, steps: u32) -> ProgressFact {
        ProgressFact::Started {
            session_id: id.to_owned(),
            steps,
            profile: normal(),
        }
    }

    fn focus_ended(id: &str, cycle: u32, ran_secs: u32, how: PhaseEnd) -> ProgressFact {
        ProgressFact::FocusEnded {
            session_id: id.to_owned(),
            cycle,
            ran_secs,
            how,
            held: false,
            profile: normal(),
        }
    }

    fn break_ended(id: &str, cycle: u32, ran_secs: u32, how: PhaseEnd) -> ProgressFact {
        ProgressFact::BreakEnded {
            session_id: id.to_owned(),
            cycle,
            ran_secs,
            how,
            profile: normal(),
        }
    }

    fn step_done(
        id: &str,
        step: u32,
        first: bool,
        done: u32,
        total: u32,
        phase: Phase,
    ) -> ProgressFact {
        ProgressFact::StepDone {
            session_id: id.to_owned(),
            step_id: step,
            first_time: first,
            done,
            total,
            phase,
            profile: normal(),
        }
    }

    /// Applies a fact to the real ledger. Most tests feed a block or a break without its Start: the tally is set up
    /// as if that Start had been seen (no start bonus). The tests of unknown sessions call [`apply`] directly.
    fn real(
        state: &mut RewardsState,
        day: RewardDay,
        rules: &RewardRules,
        fact: &ProgressFact,
    ) -> Option<RewardMoment> {
        seen(state, fact);
        apply(state, day, rules, LedgerMode::Real, fact, NOW)
    }

    fn seen(state: &mut RewardsState, fact: &ProgressFact) {
        if matches!(fact, ProgressFact::Started { .. }) {
            return;
        }
        let id = fact.session_id();
        let known = state.session.as_ref().is_some_and(|tally| {
            tally.session_id == id || (tally.session_id.is_empty() && !tally.ended)
        });
        if !known {
            state.session = Some(SessionTally::new(id, fact.profile().demo));
        }
    }

    fn line(moment: &RewardMoment, kind: RewardKind) -> u32 {
        moment
            .lines
            .iter()
            .find(|line| line.kind == kind)
            .map_or(0, |line| line.sparks)
    }

    fn day_of(state: &RewardsState, day: RewardDay) -> DayRecord {
        state.ledger.record(day).cloned().unwrap_or_default()
    }

    /// Meets the daily goal on `day` with one full 45-minute block of session `id` (no surprises).
    fn goal_day(
        state: &mut RewardsState,
        day: RewardDay,
        rules: &RewardRules,
        id: &str,
    ) -> RewardMoment {
        real(state, day, rules, &started(id, 0));
        real(
            state,
            day,
            rules,
            &focus_ended(id, 1, 2700, PhaseEnd::Deadline),
        )
        .expect("a block")
    }

    // 1. Reward days -------------------------------------------------------------------------------------------

    #[test]
    fn reward_days_start_at_four_in_the_morning() {
        let before = reward_day(2026, 10, 13, 3 * 60 + 59).expect("valid");
        assert_eq!(before, MON);
        assert_eq!(date_string(before), "2026-10-12");
        assert_eq!(weekday(before), 0, "Monday");
        let at_four = reward_day(2026, 10, 13, 4 * 60).expect("valid");
        assert_eq!(at_four, MON + 1);
        assert_eq!(date_string(at_four), "2026-10-13");
        assert_eq!(weekday(at_four), 1, "Tuesday");
        assert_eq!(reward_day(2024, 2, 29, 720), Some(19_782));
        assert_eq!(reward_day(1970, 1, 1, 240), Some(0));
        assert_eq!(reward_day(1970, 1, 1, 0), Some(-1));
        assert_eq!(weekday(0), 3, "1970-01-01 was a Thursday");
        assert_eq!(weekday(-1), 2);
        for (year, month, day, minute) in [
            (2023, 2, 29, 600),
            (2026, 13, 1, 600),
            (2026, 0, 1, 600),
            (2026, 4, 31, 600),
            (2026, 1, 0, 600),
            (2026, 1, 1, 1440),
        ] {
            assert_eq!(
                reward_day(year, month, day, minute),
                None,
                "{year}-{month}-{day} {minute}"
            );
        }
        assert_eq!(
            reward_day(2000, 2, 29, 600).map(date_string).as_deref(),
            Some("2000-02-29")
        );
        assert_eq!(
            reward_day(1900, 2, 29, 600),
            None,
            "1900 was not a leap year"
        );
        // Round trip over several years, including leap days and year ends.
        for day in (MON - 800..MON + 800).step_by(7) {
            let text = date_string(day);
            let year = text[..4].parse().expect("year");
            let month = text[5..7].parse().expect("month");
            let dom = text[8..].parse().expect("day");
            assert_eq!(reward_day(year, month, dom, 600), Some(day), "{text}");
        }
    }

    // 2. A clock going back ------------------------------------------------------------------------------------

    #[test]
    fn a_clock_going_back_is_clamped_to_the_last_day() {
        let mut state = fresh();
        let rules = no_surprises();
        goal_day(&mut state, MON + 1, &rules, "a");
        let streak = state.ledger.streak.clone();
        // The clock says Monday again: everything goes to Tuesday, nothing counts twice.
        let moment = real(&mut state, MON, &rules, &started("b", 0)).expect("start");
        assert_eq!(
            line(&moment, RewardKind::FirstOfDay),
            0,
            "Tuesday already had a start"
        );
        assert_eq!(day_of(&state, MON).sparks, 0);
        assert_eq!(day_of(&state, MON + 1).starts, 2);
        real(
            &mut state,
            MON,
            &rules,
            &focus_ended("b", 1, 2700, PhaseEnd::Deadline),
        );
        assert_eq!(state.ledger.streak, streak);
        assert_eq!(state.ledger.last_day, MON + 1);

        // A clock that was ahead (one fact dated 400 days on) and is right again: the real days count again.
        let mut ahead = fresh();
        goal_day(&mut ahead, MON, &rules, "a0");
        real(&mut ahead, MON + 400, &rules, &started("x", 0));
        for n in 1..=3 {
            goal_day(&mut ahead, MON + n, &rules, &format!("a{n}"));
        }
        assert_eq!(ahead.ledger.last_day, MON + 3);
        let shown = view(&ahead, MON + 3, &rules);
        assert_eq!(shown.today.date, date_string(MON + 3));
        assert_eq!(
            (shown.streak.state, shown.streak.days),
            (StreakState::Alive, 4)
        );
        assert_eq!(shown.streak.active_days, 4);
        // Even with a goal day counted "in the future", the streak moves on with the real days.
        let mut counted = fresh();
        goal_day(&mut counted, MON, &rules, "c0");
        goal_day(&mut counted, MON + 400, &rules, "c400");
        assert_eq!(counted.ledger.streak.last_counted_day, Some(MON + 400));
        for n in 1..=3 {
            goal_day(&mut counted, MON + n, &rules, &format!("c{n}"));
        }
        let streak = view(&counted, MON + 3, &rules).streak;
        assert_eq!((streak.state, streak.days), (StreakState::Alive, 4));
        assert!(streak.today_counted);
        // Two days back is still a time zone or a small correction: clamped.
        let mut west = fresh();
        goal_day(&mut west, MON + 2, &rules, "w");
        real(&mut west, MON, &rules, &started("w2", 0));
        assert_eq!(west.ledger.last_day, MON + 2);
        assert_eq!(day_of(&west, MON + 2).starts, 2);
    }

    // 3. Starts ------------------------------------------------------------------------------------------------

    #[test]
    fn starts_pay_within_their_daily_limits() {
        let mut state = fresh();
        let rules = rules();
        let first = real(&mut state, MON, &rules, &started("s1", 2)).expect("start");
        assert_eq!(first.kind, MomentKind::Start);
        assert_eq!(first.sparks, 12, "start 5 + first of day 5 + plan 2");
        assert_eq!(
            first.lines,
            [
                RewardLine {
                    kind: RewardKind::Start,
                    sparks: 5,
                    count: 1
                },
                RewardLine {
                    kind: RewardKind::FirstOfDay,
                    sparks: 5,
                    count: 1
                },
                RewardLine {
                    kind: RewardKind::Plan,
                    sparks: 2,
                    count: 1
                },
            ]
        );
        assert_eq!(first.session_total, 12);
        let second = real(&mut state, MON, &rules, &started("s2", 1)).expect("start");
        assert_eq!(second.sparks, 7);
        let third = real(&mut state, MON, &rules, &started("s3", 0)).expect("start");
        assert_eq!(third.sparks, 5);
        // The fourth start and the fourth plan pay nothing: no moment at all.
        real(&mut state, MON, &rules, &started("s4", 1)).expect("third plan");
        assert_eq!(real(&mut state, MON, &rules, &started("s5", 3)), None);
        let day = day_of(&state, MON);
        assert_eq!((day.starts, day.plans, day.sparks), (5, 3, 12 + 7 + 5 + 2));
        // Ids grow from 1.
        assert_eq!((first.id, second.id, third.id), (1, 2, 3));
    }

    // 4. A whole block -----------------------------------------------------------------------------------------

    #[test]
    fn the_first_block_ever_meets_the_goal_and_starts_the_streak() {
        let mut state = fresh();
        let rules = rules();
        real(&mut state, MON, &rules, &started("s1", 0));
        let moment = real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s1", 1, 2700, PhaseEnd::Deadline),
        )
        .expect("break recap");
        assert_eq!(moment.kind, MomentKind::BreakStart);
        assert_eq!(line(&moment, RewardKind::FocusTime), 45);
        assert_eq!(moment.lines[0].count, 45);
        assert_eq!(line(&moment, RewardKind::FullBlock), 10);
        assert_eq!(line(&moment, RewardKind::DailyGoal), 10);
        let surprise = moment
            .surprise
            .expect("the first surprise is at least a bubble");
        assert_eq!(
            line(&moment, RewardKind::Surprise),
            match surprise {
                Rarity::Bubble => 5,
                Rarity::Pearl => 15,
                Rarity::GoldenPearl => 40,
            }
        );
        assert_eq!(
            roll_surprise(SEED, "s1", 1, 0, false),
            Some(surprise),
            "fixed seed, fixed result"
        );
        assert!(moment.daily_goal);
        assert_eq!(moment.streak, Some(1));
        assert!(!moment.new_streak && !moment.comeback);
        assert_eq!(moment.level_up, Some(2));
        assert_eq!(moment.levels_gained, 1);
        assert_eq!(moment.session_total, 10 + moment.sparks);
        assert_eq!(level_for(state.ledger.total_sparks).level, 2);
        assert_eq!(state.ledger.counters.full_blocks, 1);
        assert_eq!(state.ledger.streak.current, 1);
    }

    // 5. Full-block boundaries ---------------------------------------------------------------------------------

    #[test]
    fn a_full_block_needs_80_percent_and_ten_minutes() {
        let rules = no_surprises();
        let full = |ran: u32, focus_secs: u32, how: PhaseEnd| {
            let mut state = fresh();
            let fact = ProgressFact::FocusEnded {
                session_id: "s".into(),
                cycle: 1,
                ran_secs: ran,
                how,
                held: false,
                profile: TimingProfile {
                    focus_secs,
                    ..normal()
                },
            };
            let moment = real(&mut state, MON, &rules, &fact);
            moment.is_some_and(|m| line(&m, RewardKind::FullBlock) == FULL_BLOCK_SPARKS)
        };
        assert!(full(2160, 2700, PhaseEnd::Deadline));
        assert!(!full(2159, 2700, PhaseEnd::Skipped));
        assert!(!full(599, 900, PhaseEnd::Deadline), "15-minute profile");
        assert!(
            !full(599, 600, PhaseEnd::Deadline),
            "80 % of 10 minutes, but under the minimum"
        );
        assert!(full(600, 600, PhaseEnd::Deadline));
        assert!(
            full(2200, 2700, PhaseEnd::Stopped),
            "an early stop past 80 % still counts"
        );
        assert!(full(3600, 2700, PhaseEnd::Deadline), "a meeting extension");
    }

    // 6. The minute cap ----------------------------------------------------------------------------------------

    #[test]
    fn minute_sparks_stop_at_the_daily_cap_and_the_pond_fills_once() {
        let mut state = fresh();
        let rules = no_surprises();
        let mut paid = 0;
        let mut ponds = 0;
        for cycle in 1..=8 {
            let moment = real(
                &mut state,
                MON,
                &rules,
                &focus_ended("s", cycle, 2700, PhaseEnd::Deadline),
            )
            .expect("block");
            paid += line(&moment, RewardKind::FocusTime);
            ponds += u32::from(moment.pond_full);
        }
        assert_eq!(paid, MINUTE_SPARKS_DAILY_CAP);
        assert_eq!(ponds, 1);
        assert!(day_of(&state, MON).pond_full);
        assert_eq!(day_of(&state, MON).minute_sparks, 300);
        let next = real(
            &mut state,
            MON + 1,
            &rules,
            &focus_ended("s", 9, 2700, PhaseEnd::Deadline),
        )
        .expect("block");
        assert_eq!(
            line(&next, RewardKind::FocusTime),
            45,
            "a new day, a new pond"
        );
        assert!(!next.pond_full);
        // Spark units follow the profile (demo: one per second).
        let mut demo = fresh();
        let fact = ProgressFact::FocusEnded {
            session_id: "d".into(),
            cycle: 1,
            ran_secs: 40,
            how: PhaseEnd::Deadline,
            held: true,
            profile: TimingProfile::demo(),
        };
        seen(&mut demo, &fact);
        let moment =
            apply(&mut demo, MON, &rules, LedgerMode::Preview, &fact, NOW).expect("demo block");
        assert_eq!(line(&moment, RewardKind::FocusTime), 40);
        assert_eq!(
            line(&moment, RewardKind::FullBlock),
            10,
            "demo minimum is 20 s"
        );
        assert!(moment.quiet, "a held break is quiet");
        assert!(moment.preview);
    }

    // 7. Breaks ------------------------------------------------------------------------------------------------

    #[test]
    fn breaks_pay_at_half_their_length_and_the_way_back_needs_a_real_block() {
        let rules = no_surprises();
        let mut state = fresh();
        real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 1, 2700, PhaseEnd::Deadline),
        );
        let back = real(
            &mut state,
            MON,
            &rules,
            &break_ended("s", 1, 450, PhaseEnd::Deadline),
        )
        .expect("back");
        assert_eq!(back.kind, MomentKind::BackFromBreak);
        assert_eq!(
            (
                line(&back, RewardKind::BreakTaken),
                line(&back, RewardKind::BackFromBreak)
            ),
            (5, 5)
        );
        assert_eq!(state.ledger.counters.breaks_taken, 1);
        assert_eq!(state.ledger.counters.backs, 1);

        // 449 of 900 s is no break taken; a short block earns no way back.
        real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 2, 300, PhaseEnd::Skipped),
        );
        let skipped = real(
            &mut state,
            MON,
            &rules,
            &break_ended("s", 2, 449, PhaseEnd::Skipped),
        );
        assert_eq!(skipped, None);

        // Stop in a break past 50 %: the break counts in the session end.
        real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 3, 2700, PhaseEnd::Deadline),
        );
        let end = real(
            &mut state,
            MON,
            &rules,
            &break_ended("s", 3, 600, PhaseEnd::Stopped),
        )
        .expect("end");
        assert_eq!(end.kind, MomentKind::SessionEnd);
        assert_eq!(line(&end, RewardKind::BreakTaken), 5);
        assert_eq!(line(&end, RewardKind::BackFromBreak), 0);
        assert!(state.session.as_ref().is_some_and(|tally| tally.ended));
    }

    // 8. Steps -------------------------------------------------------------------------------------------------

    #[test]
    fn steps_pay_once_per_session_within_the_daily_cap() {
        let rules = no_surprises();
        let mut state = fresh();
        real(&mut state, MON, &rules, &started("s", 3));
        let first = real(
            &mut state,
            MON,
            &rules,
            &step_done("s", 1, true, 1, 3, Phase::Focus),
        )
        .expect("tick");
        assert_eq!(
            (first.kind, first.sparks, first.first_step),
            (MomentKind::Step, 3, true)
        );
        // Untick and tick again: nothing more, but the mascot still reacts.
        let again = real(
            &mut state,
            MON,
            &rules,
            &step_done("s", 1, false, 1, 3, Phase::Focus),
        )
        .expect("tick");
        assert_eq!(
            (again.kind, again.sparks, again.first_step),
            (MomentKind::Step, 0, false)
        );
        assert!(again.lines.is_empty());
        let second = real(
            &mut state,
            MON,
            &rules,
            &step_done("s", 2, true, 2, 3, Phase::Focus),
        )
        .expect("tick");
        assert!(!second.first_step);
        let last = real(
            &mut state,
            MON,
            &rules,
            &step_done("s", 3, true, 3, 3, Phase::Focus),
        )
        .expect("tick");
        assert_eq!(last.kind, MomentKind::AllSteps);
        assert_eq!(
            (
                line(&last, RewardKind::Step),
                line(&last, RewardKind::AllSteps)
            ),
            (3, 5)
        );
        // All done again (after an untick): no second bonus.
        let redo = real(
            &mut state,
            MON,
            &rules,
            &step_done("s", 3, false, 3, 3, Phase::Focus),
        )
        .expect("tick");
        assert_eq!((redo.kind, redo.sparks), (MomentKind::Step, 0));
        // A single step is never "all steps".
        real(&mut state, MON, &rules, &started("one", 1));
        let only = real(
            &mut state,
            MON,
            &rules,
            &step_done("one", 1, true, 1, 1, Phase::Focus),
        )
        .expect("tick");
        assert_eq!(only.kind, MomentKind::Step);
        assert_eq!(state.ledger.counters.steps_done, 4);
        // Tick A, untick A, tick B, tick A again: the tick again completes the plan and pays "all steps" (once).
        real(&mut state, MON, &rules, &started("ab", 2));
        real(
            &mut state,
            MON,
            &rules,
            &step_done("ab", 1, true, 1, 2, Phase::Focus),
        );
        real(
            &mut state,
            MON,
            &rules,
            &step_done("ab", 2, true, 1, 2, Phase::Focus),
        );
        let completes = real(
            &mut state,
            MON,
            &rules,
            &step_done("ab", 1, false, 2, 2, Phase::Focus),
        )
        .expect("tick");
        assert_eq!(completes.kind, MomentKind::AllSteps);
        assert_eq!(
            (
                line(&completes, RewardKind::Step),
                line(&completes, RewardKind::AllSteps)
            ),
            (0, ALL_STEPS_SPARKS)
        );
        let once = real(
            &mut state,
            MON,
            &rules,
            &step_done("ab", 2, false, 2, 2, Phase::Focus),
        )
        .expect("tick");
        assert_eq!((once.kind, once.sparks), (MomentKind::Step, 0));

        // The daily cap: 15 paid steps, then nothing.
        let mut capped = fresh();
        let mut paid = 0;
        for n in 1..=20 {
            let id = format!("c{n}");
            real(&mut capped, MON, &rules, &started(&id, 1));
            let tick = real(
                &mut capped,
                MON,
                &rules,
                &step_done(&id, 1, true, 1, 1, Phase::Break),
            )
            .expect("tick");
            paid += line(&tick, RewardKind::Step);
        }
        assert_eq!(paid, STEP_SPARKS_DAILY_CAP);
        assert_eq!(day_of(&capped, MON).step_sparks, 45);
    }

    // 9. The daily goal ----------------------------------------------------------------------------------------

    #[test]
    fn the_daily_goal_is_met_once_and_its_bonus_does_not_count() {
        let rules = RewardRules {
            daily_goal: 60,
            ..no_surprises()
        };
        let mut state = fresh();
        real(&mut state, MON, &rules, &started("s", 0));
        // 10 + 35 = 45 sparks (no full block): under 60.
        let block = real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 1, 2100, PhaseEnd::Skipped),
        )
        .expect("block");
        assert!(!block.daily_goal);
        assert_eq!(day_of(&state, MON).sparks, 45);
        // A lower goal meets today at once (idle: shown now).
        let lower = RewardRules {
            daily_goal: 30,
            ..rules
        };
        let update =
            config_changed(&mut state, MON, &rules, &lower, Phase::Idle, NOW).expect("update");
        assert_eq!(update.kind, MomentKind::Update);
        assert!(update.daily_goal);
        assert_eq!(update.streak, Some(1));
        assert_eq!(line(&update, RewardKind::DailyGoal), DAILY_GOAL_BONUS);
        let day = day_of(&state, MON);
        assert!(day.goal_met);
        // The day shows 55 sparks; 45 of them count towards the goal.
        assert_eq!((day.sparks, day.goal_bonus), (55, 10));
        assert_eq!(view(&state, MON, &lower).today.sparks, 55);
        // A higher goal never un-meets it; the bonus never counts towards a goal.
        let higher = RewardRules {
            daily_goal: 120,
            ..lower
        };
        assert_eq!(
            config_changed(&mut state, MON, &lower, &higher, Phase::Idle, NOW),
            None
        );
        assert!(day_of(&state, MON).goal_met);
        let more = real(
            &mut state,
            MON,
            &higher,
            &focus_ended("s", 2, 2700, PhaseEnd::Deadline),
        )
        .expect("block");
        assert!(!more.daily_goal, "met once per day");
        assert_eq!(state.ledger.streak.active_days, 1);
        // A goal met through the bonus alone is impossible: the bonus comes only after the goal.
        let mut edge = fresh();
        edge.ledger.record_mut(MON).goal_bonus = 20;
        edge.ledger.record_mut(MON).sparks = 20;
        let goal_10 = RewardRules {
            daily_goal: 10,
            ..no_surprises()
        };
        let start = real(&mut edge, MON, &goal_10, &started("e", 0)).expect("start");
        assert_eq!(start.sparks, 10);
        assert!(
            day_of(&edge, MON).goal_met,
            "10 counted sparks meet a goal of 10"
        );
        assert_eq!(day_of(&edge, MON).goal_bonus, 30);
    }

    // 10. Focus is sacred --------------------------------------------------------------------------------------

    fn has_achievements(moment: &RewardMoment) -> bool {
        moment.surprise.is_some()
            || moment.daily_goal
            || moment.comeback
            || moment.streak.is_some()
            || moment.new_streak
            || moment.milestone.is_some()
            || moment.rest_days_used > 0
            || moment.rest_day_earned
            || moment.level_up.is_some()
            || moment.levels_gained > 0
            || moment.pond_full
            || !moment.stickers.is_empty()
            || moment
                .lines
                .iter()
                .any(|line| matches!(line.kind, RewardKind::DailyGoal | RewardKind::Comeback))
    }

    #[test]
    fn achievements_wait_while_focus_goes_on() {
        let rules = no_surprises();
        let mut state = fresh();
        let mut in_focus = Vec::new();
        in_focus.extend(real(&mut state, MON, &rules, &started("s", 5)));
        // 12 at Start, then 5 steps (15) and all steps (5): 32 >= 30 during focus.
        for step in 1..=5 {
            in_focus.extend(real(
                &mut state,
                MON,
                &rules,
                &step_done("s", step, true, step, 5, Phase::Focus),
            ));
        }
        assert!(
            day_of(&state, MON).goal_met,
            "the ledger is updated at once"
        );
        assert_eq!(state.ledger.streak.current, 1);
        assert!(
            in_focus.iter().all(|moment| !has_achievements(moment)),
            "{in_focus:#?}"
        );
        assert_eq!(
            in_focus.last().map(|moment| moment.kind),
            Some(MomentKind::AllSteps)
        );
        assert_eq!(
            in_focus.last().map(|moment| moment.sparks),
            Some(8),
            "step 3 + all steps 5, no bonus"
        );
        // The status gives nothing away either: the sparks shown so far, no goal, no streak yet.
        let during = view(&state, MON, &rules);
        assert_eq!(during.today.sparks, 32);
        assert_eq!(during.total_sparks, 32);
        assert!(!during.today.goal_met);
        assert_eq!(during.streak.state, StreakState::First);
        assert!(!during.streak.today_counted);
        assert_eq!(during.week[0].mark, DayMark::Today);
        assert_eq!(during.session.as_ref().map(|s| s.sparks), Some(32));

        // The break recap announces everything, the daily-goal bonus line included.
        let recap = real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 1, 2700, PhaseEnd::Deadline),
        )
        .expect("recap");
        assert!(recap.daily_goal);
        assert_eq!(recap.streak, Some(1));
        assert_eq!(line(&recap, RewardKind::DailyGoal), 10);
        assert_eq!(recap.level_up, Some(2));
        let after = view(&state, MON, &rules);
        assert!(after.today.goal_met && after.streak.today_counted);
        assert_eq!(after.streak.state, StreakState::Alive);
        assert_eq!(after.total_sparks, state.ledger.total_sparks);
        assert_eq!(after.level.level, 2);
        assert!(
            state
                .session
                .as_ref()
                .is_some_and(|tally| tally.shown.is_none())
        );

        // Met by a Start in focus after sparks earlier that day: the chip shows the Start's +7, not the bonus.
        let mut met = fresh();
        met.ledger.total_sparks = 28;
        let day = met.ledger.record_mut(MON);
        day.sparks = 28;
        day.starts = 1;
        let start = real(&mut met, MON, &rules, &started("p", 1)).expect("start");
        assert_eq!((start.sparks, start.daily_goal), (7, false));
        assert!(day_of(&met, MON).goal_met, "the ledger is updated at once");
        let shown = view(&met, MON, &rules);
        assert_eq!((shown.today.sparks, shown.today.goal_met), (35, false));
        assert_eq!(shown.streak.state, StreakState::First);
        // A level reached by a line shown in focus waits too: the bar stays full at the level before.
        let mut level = fresh();
        level.ledger.total_sparks = 58;
        level.ledger.record_mut(MON).starts = 1;
        real(&mut level, MON, &rules, &started("l", 1)).expect("start");
        assert_eq!(level.ledger.total_sparks, 65);
        assert_eq!(
            view(&level, MON, &rules).level,
            LevelView {
                level: 1,
                into: 60,
                span: 60
            }
        );
        let shown_up = real(
            &mut level,
            MON,
            &rules,
            &focus_ended("l", 1, 60, PhaseEnd::Skipped),
        )
        .expect("break");
        assert_eq!(shown_up.level_up, Some(2));
        assert_eq!(view(&level, MON, &rules).level.level, 2);
        assert!(
            state
                .session
                .as_ref()
                .is_some_and(|tally| tally.deferred.is_empty())
        );

        // Back in focus: a config change that meets nothing new shows nothing; the way back carries only its lines.
        let back = real(
            &mut state,
            MON,
            &rules,
            &break_ended("s", 1, 900, PhaseEnd::Deadline),
        )
        .expect("back");
        assert!(!has_achievements(&back));
        // Into the next focus, a level up from the back bonus would wait too.
        let mut leveled = fresh();
        leveled.ledger.total_sparks = 55;
        real(
            &mut leveled,
            MON,
            &rules,
            &focus_ended("x", 1, 2700, PhaseEnd::Deadline),
        );
        let before = leveled.ledger.total_sparks;
        let lvl = level_for(before).level;
        leveled.ledger.total_sparks = cumulative(lvl + 1) - 3;
        let way_back = real(
            &mut leveled,
            MON,
            &rules,
            &break_ended("x", 1, 900, PhaseEnd::Deadline),
        )
        .expect("back");
        assert!(!has_achievements(&way_back));
        assert!(
            leveled
                .session
                .as_ref()
                .is_some_and(|tally| tally.deferred.level_up.is_some())
        );
        // A step in a break shows its achievements at once (the deferred ones too).
        let tick = real(
            &mut leveled,
            MON,
            &rules,
            &step_done("x", 1, true, 1, 2, Phase::Break),
        )
        .expect("tick");
        assert_eq!(tick.level_up, Some(lvl + 1));

        // config_changed during focus defers a newly met goal.
        let strict = RewardRules {
            daily_goal: 120,
            ..rules
        };
        let mut later = fresh();
        real(&mut later, MON, &strict, &started("l", 0));
        real(
            &mut later,
            MON,
            &strict,
            &focus_ended("l", 1, 2700, PhaseEnd::Deadline),
        );
        real(
            &mut later,
            MON,
            &strict,
            &break_ended("l", 1, 900, PhaseEnd::Deadline),
        );
        assert_eq!(
            config_changed(&mut later, MON, &strict, &rules, Phase::Focus, NOW),
            None
        );
        let end = real(
            &mut later,
            MON,
            &rules,
            &focus_ended("l", 2, 60, PhaseEnd::Stopped),
        )
        .expect("end");
        assert!(end.daily_goal && end.streak == Some(1));
        assert_eq!(end.kind, MomentKind::SessionEnd);
    }

    fn cumulative(level: u32) -> u32 {
        (1..level).map(level_span).sum()
    }

    #[test]
    fn deferred_achievements_merge_in_one_recap() {
        let mut a = Achievements {
            daily_goal: true,
            streak: Some(4),
            level_up: Some(3),
            levels_gained: 1,
            rest_days_used: 1,
            stickers: vec![StickerId::DailyGoal],
            ..Achievements::default()
        };
        a.merge(Achievements {
            streak: Some(5),
            level_up: Some(4),
            levels_gained: 1,
            rest_days_used: 1,
            pond_full: true,
            stickers: vec![StickerId::DailyGoal, StickerId::Level5],
            ..Achievements::default()
        });
        assert_eq!(a.streak, Some(5));
        assert_eq!((a.level_up, a.levels_gained), (Some(4), 2));
        assert_eq!(a.rest_days_used, 2);
        assert!(a.daily_goal && a.pond_full);
        assert_eq!(a.stickers, [StickerId::DailyGoal, StickerId::Level5]);
    }

    // 11. Levels -----------------------------------------------------------------------------------------------

    #[test]
    fn levels_follow_the_growing_spans() {
        let view = |level, into, span| LevelView { level, into, span };
        assert_eq!(level_for(0), view(1, 0, 60));
        assert_eq!(level_for(59), view(1, 59, 60));
        assert_eq!(level_for(60), view(2, 0, 80));
        assert_eq!(level_for(140), view(3, 0, 100));
        assert_eq!(level_for(360), view(5, 0, 140));
        assert_eq!(level_for(1260), view(10, 0, 240));
        assert_eq!(level_span(22), 480);
        assert_eq!(level_span(23), 500);
        assert_eq!(level_span(1000), 500);
        let l23 = cumulative(23);
        assert_eq!(level_for(l23), view(23, 0, 500));
        assert_eq!(level_for(l23 + 1250), view(25, 250, 500));
        let max = level_for(u32::MAX);
        assert!(max.level > 8_000_000 && max.into < 500);
    }

    // 12. Streak -----------------------------------------------------------------------------------------------

    #[test]
    fn the_streak_counts_goal_days_across_days_off_and_rest_days() {
        let rules = no_surprises();
        let mut state = fresh();
        assert_eq!(goal_day(&mut state, MON, &rules, "a").streak, Some(1));
        assert_eq!(goal_day(&mut state, MON + 1, &rules, "b").streak, Some(2));
        // Fri → Mon over the weekend (days off): +1.
        let fri = MON + 4;
        let mut weekend = fresh();
        goal_day(&mut weekend, fri, &rules, "f");
        let monday = goal_day(&mut weekend, fri + 3, &rules, "m");
        assert_eq!(monday.streak, Some(2));
        assert_eq!(monday.rest_days_used, 0);

        // A one-day gap (Wednesday) with a bank of 2: covered, the day marked as rest.
        let wed = MON + 2;
        let covered = goal_day(&mut state, wed + 1, &rules, "c");
        assert_eq!(covered.streak, Some(3));
        assert_eq!(covered.rest_days_used, 1);
        assert_eq!(covered.milestone, Some(3));
        assert_eq!(state.ledger.streak.rest_bank, 1);
        assert!(day_of(&state, wed).rest);

        // A three-day work gap with a bank of 2: a new streak, the bank kept, a comeback bonus.
        let mut gap = fresh();
        goal_day(&mut gap, MON, &rules, "g1");
        let back = goal_day(&mut gap, MON + 4, &rules, "g2");
        assert_eq!(back.streak, Some(1));
        assert!(back.new_streak && back.comeback);
        assert_eq!(line(&back, RewardKind::Comeback), COMEBACK_SPARKS);
        assert_eq!(gap.ledger.streak.rest_bank, 2);
        assert_eq!(gap.ledger.streak.best, 1);
        assert_eq!(gap.ledger.streak.active_days, 2, "never goes down");
        // A one-day gap with an empty bank: a new streak without a comeback.
        let mut empty = fresh();
        empty.ledger.streak.rest_bank = 0;
        goal_day(&mut empty, MON, &rules, "e1");
        let restart = goal_day(&mut empty, MON + 2, &rules, "e2");
        assert!(restart.new_streak && !restart.comeback);
        // A jump of weeks is uncoverable: a new streak, best and totals kept.
        let total = gap.ledger.total_sparks;
        let far = goal_day(&mut gap, MON + 100, &rules, "g3");
        assert!(far.new_streak && far.comeback);
        assert!(gap.ledger.total_sparks > total);
    }

    #[test]
    fn rest_days_are_earned_every_five_goal_days_up_to_three() {
        let rules = no_surprises();
        let mut state = fresh();
        let mut earned = Vec::new();
        let mut milestones = Vec::new();
        for n in 0..10 {
            let moment = goal_day(&mut state, MON + n, &rules, &format!("s{n}"));
            earned.push(moment.rest_day_earned);
            milestones.extend(moment.milestone);
        }
        // Weekends count too when the goal is met (an active day on a day off).
        assert_eq!(state.ledger.streak.current, 10);
        assert_eq!(
            earned.iter().filter(|e| **e).count(),
            1,
            "5th day: 2 → 3; 10th: already full"
        );
        assert!(earned[4]);
        assert_eq!(state.ledger.streak.rest_bank, REST_BANK_MAX);
        assert_eq!(milestones, [3, 7]);
        assert_eq!(state.ledger.streak.best, 10);
    }

    // 13. Pause ------------------------------------------------------------------------------------------------

    #[test]
    fn a_pause_turns_its_days_neutral() {
        let on = no_surprises();
        let off = RewardRules {
            counting: false,
            ..on
        };
        let mut state = fresh();
        goal_day(&mut state, MON, &on, "a");
        // Paused on Tuesday, back on Friday: Tue..Thu are neutral.
        assert_eq!(
            config_changed(&mut state, MON + 1, &on, &off, Phase::Idle, NOW),
            None
        );
        assert_eq!(state.ledger.paused_since, Some(MON + 1));
        assert_eq!(
            view(&state, MON + 2, &off).streak.state,
            StreakState::Paused
        );
        assert_eq!(view(&state, MON + 2, &off).streak.days, 1);
        config_changed(&mut state, MON + 4, &off, &on, Phase::Idle, NOW);
        assert_eq!(state.ledger.paused_since, None);
        assert_eq!(
            state.ledger.neutral,
            [DayRange {
                from: MON + 1,
                to: MON + 3
            }]
        );
        assert_eq!(goal_day(&mut state, MON + 4, &on, "b").streak, Some(2));

        // Off and on the same day: no range.
        config_changed(&mut state, MON + 7, &on, &off, Phase::Idle, NOW);
        config_changed(&mut state, MON + 7, &off, &on, Phase::Idle, NOW);
        assert_eq!(state.ledger.neutral.len(), 1);

        // More than 20 ranges: the oldest goes.
        for n in 0..25 {
            let day = MON + 10 + n * 2;
            config_changed(&mut state, day, &on, &off, Phase::Idle, NOW);
            config_changed(&mut state, day + 1, &off, &on, Phase::Idle, NOW);
        }
        assert_eq!(state.ledger.neutral.len(), MAX_NEUTRAL_RANGES);
        assert_eq!(
            state.ledger.neutral.last(),
            Some(&DayRange {
                from: MON + 58,
                to: MON + 58
            })
        );

        // Without a streak a pause records nothing (a ledger nobody used never changes).
        let mut unused = fresh();
        assert_eq!(
            config_changed(&mut unused, MON, &on, &off, Phase::Idle, NOW),
            None
        );
        assert_eq!(unused, fresh());
        // A goal met while paused counts when counting comes back that day.
        let mut paused = fresh();
        config_changed(&mut paused, MON, &on, &off, Phase::Idle, NOW);
        goal_day(&mut paused, MON, &off, "p");
        assert_eq!(paused.ledger.streak.last_counted_day, None);
        let update =
            config_changed(&mut paused, MON, &off, &on, Phase::Idle, NOW).expect("counted now");
        assert_eq!(update.streak, Some(1));

        // A streak that was already waiting when the pause began stays waiting (no old number comes back only to
        // go again); one alive then shows paused with its days.
        let mut waiting = fresh();
        goal_day(&mut waiting, MON, &on, "w");
        // Tue, Wed and Thu without a goal (3 > the bank of 2), paused on Friday.
        config_changed(&mut waiting, MON + 4, &on, &off, Phase::Idle, NOW);
        let during = view(&waiting, MON + 7, &off).streak;
        assert_eq!((during.state, during.days), (StreakState::Waiting, 0));
        config_changed(&mut waiting, MON + 7, &off, &on, Phase::Idle, NOW);
        let after = view(&waiting, MON + 7, &on).streak;
        assert_eq!((after.state, after.days), (StreakState::Waiting, 0));
        let mut alive = fresh();
        goal_day(&mut alive, MON, &on, "a");
        // Tue without a goal (covered by the bank), paused on Wednesday.
        config_changed(&mut alive, MON + 2, &on, &off, Phase::Idle, NOW);
        let held = view(&alive, MON + 9, &off).streak;
        assert_eq!(
            (held.state, held.days, held.rest_reserved),
            (StreakState::Paused, 1, 1)
        );
    }

    // 14. Surprises --------------------------------------------------------------------------------------------

    #[test]
    fn surprise_odds_match_the_published_ones() {
        let mut counts = [0_u32; 4];
        for n in 0..20_000 {
            for cycle in 1..=5 {
                let index = match roll_surprise(SEED, &format!("{NOW}-{n}"), cycle, 0, true) {
                    None => 0,
                    Some(Rarity::Bubble) => 1,
                    Some(Rarity::Pearl) => 2,
                    Some(Rarity::GoldenPearl) => 3,
                };
                counts[index] += 1;
            }
        }
        for (count, odds) in counts.iter().zip(ODDS_PERMILLE) {
            let share = f64::from(*count) / 100_000.0;
            let expected = f64::from(odds) / 1000.0;
            assert!((share - expected).abs() <= 0.015, "{counts:?}");
        }
        assert_eq!(
            ODDS_PERMILLE.iter().map(|o| u32::from(*o)).sum::<u32>(),
            1000
        );
    }

    #[test]
    fn pity_first_surprise_and_switching_surprises_off() {
        // Pity: after 4 blocks without a pearl, at least a pearl.
        for n in 0..200 {
            let id = format!("p{n}");
            let rolled = roll_surprise(SEED, &id, 1, PITY_BLOCKS, true);
            assert!(
                matches!(rolled, Some(Rarity::Pearl | Rarity::GoldenPearl)),
                "{rolled:?}"
            );
            assert!(
                roll_surprise(SEED, &id, 1, 0, false).is_some(),
                "the first is at least a bubble"
            );
        }
        // Deterministic.
        assert_eq!(
            roll_surprise(SEED, "x", 3, 1, true),
            roll_surprise(SEED, "x", 3, 1, true)
        );

        // In the ledger: the pity counter resets on a pearl and the collection grows.
        let rules = rules();
        let mut state = fresh();
        let mut pearls_after_pity = true;
        for cycle in 1..=40 {
            let since = state.ledger.surprises.since_pearl;
            let moment = real(
                &mut state,
                MON,
                &rules,
                &focus_ended("s", cycle, 2700, PhaseEnd::Deadline),
            )
            .expect("block");
            if since >= PITY_BLOCKS {
                pearls_after_pity &=
                    matches!(moment.surprise, Some(Rarity::Pearl | Rarity::GoldenPearl));
            }
            assert!(state.ledger.surprises.since_pearl <= PITY_BLOCKS);
        }
        assert!(pearls_after_pity);
        let c = state.ledger.collection;
        assert!(c.bubbles + c.pearls + c.golden >= 10, "{c:?}");
        assert!(state.ledger.surprises.first_done);

        // Surprises off: no roll, the counter stays.
        let mut off = fresh();
        off.ledger.surprises.since_pearl = 2;
        let moment = real(
            &mut off,
            MON,
            &no_surprises(),
            &focus_ended("o", 1, 2700, PhaseEnd::Deadline),
        )
        .expect("block");
        assert_eq!(moment.surprise, None);
        assert_eq!(off.ledger.surprises.since_pearl, 2);
        assert_eq!(off.ledger.collection, Collection::default());
        // A block that is not full never rolls.
        let short = real(
            &mut off,
            MON,
            &rules,
            &focus_ended("o", 2, 900, PhaseEnd::Skipped),
        )
        .expect("minutes");
        assert_eq!(short.surprise, None);
    }

    // 15. Goal done --------------------------------------------------------------------------------------------

    #[test]
    fn a_session_goal_pays_once() {
        let rules = no_surprises();
        let mut state = fresh();
        real(&mut state, MON, &rules, &started("s", 0));
        real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 1, 60, PhaseEnd::Stopped),
        );
        let done = goal_done(
            &mut state,
            MON,
            &rules,
            LedgerMode::Real,
            "s",
            Phase::Idle,
            NOW,
        )
        .expect("goal");
        assert_eq!(
            (done.kind, done.sparks),
            (MomentKind::GoalDone, GOAL_DONE_SPARKS)
        );
        assert_eq!(done.session_total, 10 + 1 + 10);
        assert_eq!(
            goal_done(
                &mut state,
                MON,
                &rules,
                LedgerMode::Real,
                "s",
                Phase::Idle,
                NOW
            ),
            None
        );
        // Inactive: nothing.
        let inactive = RewardRules {
            active: false,
            ..rules
        };
        assert_eq!(
            goal_done(
                &mut state,
                MON,
                &inactive,
                LedgerMode::Real,
                "other",
                Phase::Idle,
                NOW
            ),
            None
        );
        assert!(!state.ledger.goal_rewarded.contains(&"other".to_owned()));
        // The 51st id drops the oldest.
        for n in 0..MAX_GOAL_REWARDED {
            goal_done(
                &mut state,
                MON,
                &rules,
                LedgerMode::Real,
                &format!("g{n}"),
                Phase::Idle,
                NOW,
            )
            .expect("goal");
        }
        assert_eq!(state.ledger.goal_rewarded.len(), MAX_GOAL_REWARDED);
        assert!(!state.ledger.goal_rewarded.contains(&"s".to_owned()));
        assert_eq!(
            state.ledger.goal_rewarded.last().map(String::as_str),
            Some("g49")
        );
        // An answer for another session during focus does not touch the running tally.
        real(&mut state, MON, &rules, &started("now", 0));
        let other = goal_done(
            &mut state,
            MON,
            &rules,
            LedgerMode::Real,
            "old",
            Phase::Focus,
            NOW,
        )
        .expect("goal");
        assert_eq!(other.session_total, 0);
        assert_eq!(state.session.as_ref().map(|tally| tally.sparks), Some(5));
    }

    // 16. Demo -------------------------------------------------------------------------------------------------

    #[test]
    fn demo_facts_only_reach_the_preview() {
        let rules = rules();
        let demo = ProgressFact::Started {
            session_id: "d".into(),
            steps: 1,
            profile: TimingProfile::demo(),
        };
        let mut state = fresh();
        let before = state.clone();
        assert_eq!(
            apply(&mut state, MON, &rules, LedgerMode::Real, &demo, NOW),
            None
        );
        assert_eq!(state, before);
        let moment =
            apply(&mut state, MON, &rules, LedgerMode::Preview, &demo, NOW).expect("preview");
        assert!(moment.preview);
        assert_eq!(moment.sparks, 12);
        // Inactive rules credit nothing at all.
        let inactive = RewardRules {
            active: false,
            ..rules
        };
        let mut quiet = fresh();
        assert_eq!(real(&mut quiet, MON, &inactive, &started("s", 1)), None);
        assert_eq!(quiet, fresh());
    }

    // 17. View -------------------------------------------------------------------------------------------------

    #[test]
    fn the_view_shows_the_week_the_streak_and_the_odds() {
        let rules = no_surprises();
        let mut state = fresh();
        let first = view(&state, MON, &rules);
        assert_eq!(first.streak.state, StreakState::First);
        assert_eq!(first.streak.days, 0);
        assert_eq!(
            (
                first.streak.rest_bank,
                first.streak.rest_bank_max,
                first.streak.next_rest_in
            ),
            (2, 3, 5)
        );
        assert_eq!(first.today.goal, 30);
        assert_eq!(first.today.date, "2026-10-12");
        assert_eq!(first.session, None);
        assert_eq!(
            first.odds,
            SurpriseOdds {
                none_permille: 500,
                bubble_permille: 320,
                pearl_permille: 150,
                golden_permille: 30,
                pity_blocks: 4,
            }
        );
        assert_eq!(
            view(
                &state,
                MON,
                &RewardRules {
                    daily_goal: 120,
                    ..rules
                }
            )
            .today
            .goal,
            120
        );

        // Mon goal, Tue nothing (rest), Wed goal, Thu nothing, today Fri.
        goal_day(&mut state, MON, &rules, "a");
        goal_day(&mut state, MON + 2, &rules, "b");
        state.ledger.neutral.push(DayRange {
            from: MON - 3,
            to: MON - 1,
        });
        let fri = view(&state, MON + 4, &rules);
        let marks: Vec<DayMark> = fri.week.iter().map(|cell| cell.mark).collect();
        assert_eq!(
            marks,
            [
                DayMark::Goal,
                DayMark::Rest,
                DayMark::Goal,
                DayMark::Quiet,
                DayMark::Today,
                DayMark::Future,
                DayMark::Future
            ]
        );
        assert_eq!(fri.week[0].date, "2026-10-12");
        assert_eq!(fri.week[6].date, "2026-10-18");
        // Thursday is missing but the bank (1 left) covers it: alive, one reserved.
        assert_eq!(fri.streak.state, StreakState::Alive);
        assert_eq!(
            (
                fri.streak.days,
                fri.streak.rest_reserved,
                fri.streak.today_counted
            ),
            (2, 1, false)
        );
        assert_eq!(
            fri.session.as_ref().map(|s| s.session_id.as_str()),
            Some("b")
        );
        // The weekend shows as neutral once it is past; last week's paused range too.
        let next_mon = view(&state, MON + 7, &rules);
        let marks: Vec<DayMark> = next_mon.week.iter().map(|cell| cell.mark).collect();
        assert_eq!(marks[0], DayMark::Today);
        let last_week = week(&state.ledger, &rules, MON + 6);
        assert_eq!(last_week[5].mark, DayMark::Neutral);
        assert_eq!(last_week[6].mark, DayMark::Today);
        assert_eq!(
            week(&state.ledger, &rules, MON - 1)[4].mark,
            DayMark::Neutral,
            "paused Friday before"
        );
        // Thu + Fri + next Mon missing: more than the bank → waiting for the next goal.
        let waiting = view(&state, MON + 8, &rules);
        assert_eq!(waiting.streak.state, StreakState::Waiting);
        assert_eq!(waiting.streak.days, 0);
        assert_eq!((waiting.streak.best, waiting.streak.active_days), (2, 2));
        // Today counted.
        goal_day(&mut state, MON + 8, &rules, "c");
        let counted = view(&state, MON + 8, &rules);
        assert!(counted.streak.today_counted);
        assert!(counted.today.goal_met);
        assert_eq!(counted.week[1].mark, DayMark::Goal);
        // Paused.
        let paused = view(
            &state,
            MON + 9,
            &RewardRules {
                counting: false,
                ..rules
            },
        );
        assert_eq!(paused.streak.state, StreakState::Paused);
        assert_eq!(paused.streak.days, 1);
        assert_eq!(paused.total_sparks, state.ledger.total_sparks);
        assert!(
            !view(
                &state,
                MON,
                &RewardRules {
                    active: false,
                    ..rules
                }
            )
            .active
        );
    }

    // 18. The ledger on disk -----------------------------------------------------------------------------------

    #[test]
    fn the_ledger_round_trips_and_repairs_itself() {
        let rules = rules();
        let mut state = fresh();
        goal_day(&mut state, MON, &rules, "1760342400000-4242-3");
        goal_done(
            &mut state,
            MON,
            &rules,
            LedgerMode::Real,
            "1760342400000-4242-3",
            Phase::Idle,
            NOW,
        );
        let json = serde_json::to_string(&state.ledger).expect("serialize");
        assert!(
            json.contains("\"totalSparks\"")
                && json.contains("\"lastCountedDay\"")
                && json.contains("\"goalRewarded\"")
        );
        let back: Ledger = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, state.ledger);

        // Missing fields take their defaults (a fresh bank of 2).
        let sparse: Ledger = serde_json::from_str(
            r#"{"version":1,"totalSparks":62,"days":[{"day":20739}],"streak":{"current":3}}"#,
        )
        .expect("sparse");
        assert_eq!(sparse.total_sparks, 62);
        assert_eq!(sparse.streak.rest_bank, REST_BANK_START);
        assert_eq!(sparse.days[0].sparks, 0);
        assert_eq!(
            serde_json::from_str::<Ledger>("{}").expect("empty"),
            Ledger::default()
        );

        let mut messy = Ledger::new(1);
        messy.version = 0;
        messy.days = (0..60)
            .rev()
            .map(|n| DayRecord {
                day: MON - n,
                ..DayRecord::default()
            })
            .collect();
        messy.days.push(DayRecord {
            day: MON,
            sparks: 9,
            ..DayRecord::default()
        });
        messy.streak.rest_bank = 9;
        messy.streak.rest_progress = 7;
        messy.streak.current = 4;
        messy.neutral = (0..30).map(|n| DayRange { from: n, to: n }).collect();
        messy.neutral.push(DayRange { from: 5, to: 1 });
        messy.goal_rewarded = (0..70).map(|n| n.to_string()).collect();
        messy.stickers = vec![StickerId::Level5, StickerId::Level5, StickerId::FirstStep];
        messy.normalize();
        assert_eq!(messy.version, LEDGER_VERSION);
        assert_eq!(messy.days.len(), MAX_DAYS_KEPT);
        assert!(messy.days.windows(2).all(|pair| pair[0].day < pair[1].day));
        assert_eq!(messy.days.last().map(|record| record.day), Some(MON));
        assert_eq!(messy.streak.rest_bank, REST_BANK_MAX);
        assert!(messy.streak.rest_progress < REST_EARN_EVERY);
        assert_eq!(messy.streak.best, 4);
        assert_eq!(messy.neutral.len(), MAX_NEUTRAL_RANGES);
        assert!(messy.neutral.iter().all(|range| range.from <= range.to));
        assert_eq!(messy.goal_rewarded.len(), MAX_GOAL_REWARDED);
        assert_eq!(messy.goal_rewarded.last().map(String::as_str), Some("69"));
        assert_eq!(messy.stickers, [StickerId::Level5, StickerId::FirstStep]);
        assert_eq!(messy.last_day, MON);
        // Days stay capped while the app runs.
        let mut long = fresh();
        for n in 0..60 {
            real(
                &mut long,
                MON + n,
                &no_surprises(),
                &started(&format!("l{n}"), 0),
            );
        }
        assert_eq!(long.ledger.days.len(), MAX_DAYS_KEPT);
    }

    #[test]
    fn the_ledger_holds_numbers_and_ids_only() {
        let rules = rules();
        let mut state = fresh();
        real(&mut state, MON, &rules, &started("1760342400000-4242-3", 2));
        real(
            &mut state,
            MON,
            &rules,
            &step_done("1760342400000-4242-3", 1, true, 1, 2, Phase::Focus),
        );
        goal_done(
            &mut state,
            MON,
            &rules,
            LedgerMode::Real,
            "1760342400000-4242-3",
            Phase::Focus,
            NOW,
        );
        let json = serde_json::to_value(&state.ledger).expect("serialize");
        fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
            match value {
                serde_json::Value::String(text) => out.push(text.clone()),
                serde_json::Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
                serde_json::Value::Object(map) => map.values().for_each(|item| strings(item, out)),
                _ => {}
            }
        }
        // The only strings: sticker ids and the session ids of rewarded goals.
        let mut stickers = Vec::new();
        strings(&json["stickers"], &mut stickers);
        assert!(!stickers.is_empty());
        let mut goals = Vec::new();
        strings(&json["goalRewarded"], &mut goals);
        assert_eq!(goals, ["1760342400000-4242-3"]);
        let mut all = Vec::new();
        strings(&json, &mut all);
        assert_eq!(all.len(), stickers.len() + goals.len(), "{all:?}");
    }

    // 19. Stickers (P1) ----------------------------------------------------------------------------------------

    #[test]
    fn stickers_unlock_deterministically_and_wait_in_focus() {
        let rules = no_surprises();
        let mut state = fresh();
        // Planner: a Start with 3 steps (announced later: focus).
        let start = real(&mut state, MON, &rules, &started("s", 3)).expect("start");
        assert!(start.stickers.is_empty());
        assert!(state.ledger.stickers.contains(&StickerId::Planner));
        // First step: unlocked in focus, shown with the recap.
        real(
            &mut state,
            MON,
            &rules,
            &step_done("s", 1, true, 1, 3, Phase::Focus),
        );
        let recap = real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 1, 2700, PhaseEnd::Deadline),
        )
        .expect("recap");
        assert!(recap.stickers.contains(&StickerId::Planner));
        assert!(recap.stickers.contains(&StickerId::FirstStep));
        assert!(recap.stickers.contains(&StickerId::DailyGoal));
        // The way back: comeback sticker (deferred into the next focus).
        real(
            &mut state,
            MON,
            &rules,
            &break_ended("s", 1, 900, PhaseEnd::Deadline),
        );
        assert!(state.ledger.stickers.contains(&StickerId::Comeback));
        // Level 5 at 360 sparks, golden pearl at level 10, three days, good break, together, amazed, rest day.
        let mut big = fresh();
        big.ledger.total_sparks = 355;
        let up = real(
            &mut big,
            MON,
            &rules,
            &focus_ended("b", 1, 2700, PhaseEnd::Deadline),
        )
        .expect("block");
        assert!(up.stickers.contains(&StickerId::Level5));
        big.ledger.total_sparks = cumulative(10) - 1;
        let ten = real(
            &mut big,
            MON,
            &rules,
            &focus_ended("b", 2, 600, PhaseEnd::Skipped),
        )
        .expect("block");
        assert!(ten.stickers.contains(&StickerId::GoldenPearl));
        let mut days = fresh();
        for n in 0..3 {
            goal_day(&mut days, MON + n, &rules, &format!("d{n}"));
        }
        assert!(days.ledger.stickers.contains(&StickerId::ThreeDays));
        let mut breaks = fresh();
        for n in 1..=5 {
            real(
                &mut breaks,
                MON,
                &rules,
                &break_ended("k", n, 900, PhaseEnd::Skipped),
            );
        }
        assert!(breaks.ledger.stickers.contains(&StickerId::GoodBreak));
        let mut blocks = fresh();
        for n in 1..=15 {
            real(
                &mut blocks,
                MON,
                &rules,
                &focus_ended("z", n, 2700, PhaseEnd::Deadline),
            );
            let together = blocks.ledger.stickers.contains(&StickerId::Together);
            assert_eq!(together, n >= 10, "{n}");
        }
        assert!(blocks.ledger.stickers.contains(&StickerId::Amazed));
        let mut rest = fresh();
        goal_day(&mut rest, MON, &rules, "r1");
        goal_day(&mut rest, MON + 2, &rules, "r2");
        assert!(rest.ledger.stickers.contains(&StickerId::RestDay));
        // Each sticker unlocks once.
        let mut unique = state.ledger.stickers.clone();
        unique.dedup();
        assert_eq!(unique.len(), state.ledger.stickers.len());
    }

    #[test]
    fn popup_line_and_session_view_follow_the_moment() {
        let rules = rules();
        let mut state = fresh();
        real(&mut state, MON, &rules, &started("s", 0));
        let recap = real(
            &mut state,
            MON,
            &rules,
            &focus_ended("s", 1, 2700, PhaseEnd::Deadline),
        )
        .expect("recap");
        let popup = popup_rewards(&recap);
        assert_eq!(popup.sparks, recap.sparks);
        assert!(popup.daily_goal);
        assert_eq!(
            (popup.streak, popup.level_up, popup.surprise),
            (Some(1), Some(2), recap.surprise)
        );
        let session = view(&state, MON, &rules).session.expect("session");
        assert_eq!(session.sparks, 10 + recap.sparks);
        assert_eq!(session.sparks, recap.session_total);
        assert_eq!(
            session.lines.iter().map(|line| line.sparks).sum::<u32>(),
            session.sparks
        );
    }

    #[test]
    fn facts_of_an_unknown_session_start_a_tally_without_a_start_bonus() {
        let rules = no_surprises();
        let unseen = |state: &mut RewardsState, fact: &ProgressFact| {
            apply(state, MON, &rules, LedgerMode::Real, fact, NOW)
        };
        // A block of a session whose Start was never seen (the rewards came on, or started again, meanwhile):
        // accrual starts now, so nothing of it is paid after the fact.
        let mut state = fresh();
        assert_eq!(
            unseen(
                &mut state,
                &focus_ended("late", 1, 2700, PhaseEnd::Deadline)
            ),
            None
        );
        let tally = state.session.as_ref().expect("a tally");
        assert_eq!(tally.session_id, "late");
        assert_eq!((tally.joined_ms, tally.last_block_real), (None, false));
        assert_eq!(state.ledger.total_sparks, 0);
        // Joined ten minutes before the block ended: those minutes count, the block is no full one, and no way back
        // follows it. The break after it and the next block are whole.
        let mut joined = fresh();
        joined.session = Some(SessionTally::joined(NOW - 600_000));
        let block =
            unseen(&mut joined, &focus_ended("j", 1, 2700, PhaseEnd::Deadline)).expect("minutes");
        assert_eq!(
            (
                line(&block, RewardKind::FocusTime),
                line(&block, RewardKind::FullBlock)
            ),
            (10, 0)
        );
        assert_eq!(block.surprise, None);
        let back =
            unseen(&mut joined, &break_ended("j", 1, 900, PhaseEnd::Deadline)).expect("break");
        assert_eq!(
            (
                line(&back, RewardKind::BreakTaken),
                line(&back, RewardKind::BackFromBreak)
            ),
            (5, 0)
        );
        let next =
            unseen(&mut joined, &focus_ended("j", 2, 2700, PhaseEnd::Deadline)).expect("block");
        assert_eq!(
            (
                line(&next, RewardKind::FocusTime),
                line(&next, RewardKind::FullBlock)
            ),
            (45, FULL_BLOCK_SPARKS)
        );
        // Joined in a break five minutes before it ended: 300 of 900 s is no break taken.
        let mut in_break = fresh();
        in_break.session = Some(SessionTally::joined(NOW - 300_000));
        assert_eq!(
            unseen(&mut in_break, &break_ended("k", 1, 900, PhaseEnd::Deadline)),
            None
        );
        // Turned on while a session runs: the running session adopts a tally that joined now.
        let off = RewardRules {
            active: false,
            counting: false,
            ..rules
        };
        let mut on_again = fresh();
        config_changed(&mut on_again, MON, &off, &rules, Phase::Focus, NOW);
        assert_eq!(
            on_again.session.as_ref().and_then(|tally| tally.joined_ms),
            Some(NOW)
        );
        let mut idle = fresh();
        config_changed(&mut idle, MON, &off, &rules, Phase::Idle, NOW);
        assert_eq!(idle, fresh(), "in Idle the next Start begins the tally");
        // A tally made before its session was known (a config change in focus) is adopted.
        let mut adopted = fresh();
        adopted.session = Some(SessionTally {
            deferred: Achievements {
                daily_goal: true,
                ..Achievements::default()
            },
            ..SessionTally::default()
        });
        let recap = real(
            &mut adopted,
            MON,
            &rules,
            &focus_ended("x", 1, 60, PhaseEnd::Skipped),
        )
        .expect("recap");
        assert!(recap.daily_goal);
        assert_eq!(
            adopted
                .session
                .as_ref()
                .map(|tally| tally.session_id.as_str()),
            Some("x")
        );

        // Rewards switched on during a later session: the last tally ended, so a goal met in focus waits in a new tally
        // that the running session adopts.
        let strict = RewardRules {
            daily_goal: 120,
            ..rules
        };
        let mut later = fresh();
        real(&mut later, MON, &strict, &started("old", 0));
        real(
            &mut later,
            MON,
            &strict,
            &focus_ended("old", 1, 2700, PhaseEnd::Stopped),
        );
        assert_eq!(
            config_changed(&mut later, MON, &strict, &rules, Phase::Focus, NOW),
            None
        );
        let recap = real(
            &mut later,
            MON,
            &rules,
            &focus_ended("new", 1, 600, PhaseEnd::Skipped),
        )
        .expect("recap");
        assert!(recap.daily_goal && recap.streak == Some(1));
        assert_eq!(line(&recap, RewardKind::DailyGoal), DAILY_GOAL_BONUS);
    }
}
