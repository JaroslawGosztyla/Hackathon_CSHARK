//! Beta collection (ADR-024): collection themes and ambience layers earned with the rewards (levels, the best streak,
//! goal days inside a seasonal window), never bought. Pure like the rest of the rewards: the shell passes the reward
//! day; nothing here reads a clock or a file.
//!
//! - The catalogue is fixed: [`THEME_RULES`] and [`AMBIENCE_RULES`] (named numbers, rule 2). Rain and wind are free.
//! - Seasonal items can be earned only inside their window ([`season_window`]); once earned they stay usable all year.
//!   The windows never overlap: January, February, Easter (March / April / early May), July, October, December.
//! - Easter: the calendar month of Western Easter Sunday ([`easter_sunday`], anonymous Gregorian algorithm), stretched
//!   to at least [`EASTER_MARGIN_DAYS`] days on both sides of Easter Sunday.
//! - The rules are state-based and idempotent ([`evaluate`]): whoever meets a rule gets the item at the next fact of
//!   the real ledger (or silently at load / when the rewards come on, `rewards::catch_up`). The demo preview never
//!   unlocks and never records seasonal goal days.
//! - Records live in `rewards.json` (`Ledger.unlocks`, `Ledger.seasons`): fixed ids, day numbers and years only. Ids
//!   and seasons are strings ([`key`], [`season_key`]) so unknown ones from a newer app are kept and ignored.
//! - Nothing here is ever deleted by switching the beta or the rewards off; only a fresh ledger starts empty.

use crate::api::{
    AmbienceLayer, AmbienceUnlock, AppError, RewardTheme, Season, SeasonWindow, ThemeUnlock,
    UnlockProgress, UnlockRule, UnlockState, UnlocksView,
};
use crate::config::CONFIG_INVALID;
use crate::rewards::{
    Ledger, MAX_SEASON_RECORDS, MAX_UNLOCK_RECORDS, RewardDay, SeasonProgress, UnlockRecord,
    civil_from_days, date_string, days_from_civil, days_in_month, level_for,
};

/// Goal days inside one window of a season that unlock its theme (and its ambience).
pub const SEASON_GOAL_DAYS: u32 = 3;
/// The Easter window reaches at least this many days before and after Easter Sunday.
pub const EASTER_MARGIN_DAYS: i32 = 7;

/// How every collection theme is earned, in catalogue order (`RewardTheme::ALL`).
pub const THEME_RULES: [(RewardTheme, UnlockRule); 10] = [
    (RewardTheme::Underwater, UnlockRule::Level { level: 3 }),
    (RewardTheme::RainyWindow, UnlockRule::BestStreak { days: 7 }),
    (RewardTheme::ForestCabin, UnlockRule::Level { level: 6 }),
    (
        RewardTheme::SakuraNight,
        UnlockRule::BestStreak { days: 14 },
    ),
    (RewardTheme::NewYear, seasonal(Season::NewYear)),
    (RewardTheme::Valentines, seasonal(Season::Valentines)),
    (RewardTheme::Easter, seasonal(Season::Easter)),
    (RewardTheme::Summer, seasonal(Season::Summer)),
    (RewardTheme::Halloween, seasonal(Season::Halloween)),
    (RewardTheme::Christmas, seasonal(Season::Christmas)),
];

/// How every ambience layer is earned, in catalogue order (`AmbienceLayer::ALL`).
pub const AMBIENCE_RULES: [(AmbienceLayer, UnlockRule); 9] = [
    (AmbienceLayer::Rain, UnlockRule::Free),
    (AmbienceLayer::Wind, UnlockRule::Free),
    (AmbienceLayer::Thunder, UnlockRule::Level { level: 2 }),
    (AmbienceLayer::Cafe, UnlockRule::Level { level: 4 }),
    (AmbienceLayer::Fireplace, UnlockRule::BestStreak { days: 5 }),
    (AmbienceLayer::Forest, UnlockRule::Level { level: 5 }),
    (AmbienceLayer::Ocean, UnlockRule::Level { level: 7 }),
    (
        AmbienceLayer::SpookyNight,
        UnlockRule::WithTheme {
            theme: RewardTheme::Halloween,
        },
    ),
    (
        AmbienceLayer::WinterCottage,
        UnlockRule::WithTheme {
            theme: RewardTheme::Christmas,
        },
    ),
];

const fn seasonal(season: Season) -> UnlockRule {
    UnlockRule::Seasonal {
        season,
        goal_days: SEASON_GOAL_DAYS,
    }
}

/// The rule of a collection theme.
pub fn theme_rule(theme: RewardTheme) -> UnlockRule {
    THEME_RULES
        .iter()
        .find(|(item, _)| *item == theme)
        .map_or(UnlockRule::Free, |(_, rule)| *rule)
}

/// The rule of an ambience layer.
pub fn ambience_rule(layer: AmbienceLayer) -> UnlockRule {
    AMBIENCE_RULES
        .iter()
        .find(|(item, _)| *item == layer)
        .map_or(UnlockRule::Free, |(_, rule)| *rule)
}

/// Western (Gregorian) Easter Sunday of `year` as `(month, day)`: the anonymous Gregorian algorithm (Meeus / Jones /
/// Butcher). Always between 22 March and 25 April.
pub fn easter_sunday(year: i32) -> (u32, u32) {
    let y = i64::from(year);
    let a = y.rem_euclid(19);
    let b = y.div_euclid(100);
    let c = y.rem_euclid(100);
    let d = b.div_euclid(4);
    let e = b.rem_euclid(4);
    let f = (b + 8).div_euclid(25);
    let g = (b - f + 1).div_euclid(3);
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let i = c.div_euclid(4);
    let k = c.rem_euclid(4);
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let m = (a + 11 * h + 22 * l).div_euclid(451);
    let month = (h + l - 7 * m + 114).div_euclid(31);
    let day = (h + l - 7 * m + 114).rem_euclid(31) + 1;
    (
        u32::try_from(month).unwrap_or(4),
        u32::try_from(day).unwrap_or(1),
    )
}

/// The window of `season` in `year`: first and last reward day, inclusive. One calendar month, except Easter: the month
/// of Easter Sunday, stretched so that it reaches at least [`EASTER_MARGIN_DAYS`] days before and after Easter Sunday
/// (Easter on 31 March 2024 → 1 March … 7 April; on 5 April 2026 → 29 March … 30 April; on 20 April 2025 → April).
pub fn season_window(season: Season, year: i32) -> (RewardDay, RewardDay) {
    let month_window = |month: u32| {
        (
            civil_day(year, month, 1),
            civil_day(year, month, days_in_month(year, month)),
        )
    };
    match season {
        Season::NewYear => month_window(1),
        Season::Valentines => month_window(2),
        Season::Summer => month_window(7),
        Season::Halloween => month_window(10),
        Season::Christmas => month_window(12),
        Season::Easter => {
            let (month, day) = easter_sunday(year);
            let easter = civil_day(year, month, day);
            let (start, end) = month_window(month);
            (
                start.min(easter - EASTER_MARGIN_DAYS),
                end.max(easter + EASTER_MARGIN_DAYS),
            )
        }
    }
}

/// `Some(month)` when `season`'s window in `year` is exactly one calendar month.
pub fn window_month(season: Season, year: i32) -> Option<u32> {
    let (start, end) = season_window(season, year);
    (1..=12).find(|&month| {
        start == civil_day(year, month, 1)
            && end == civil_day(year, month, days_in_month(year, month))
    })
}

fn civil_day(year: i32, month: u32, day: u32) -> RewardDay {
    RewardDay::try_from(days_from_civil(i64::from(year), month, day)).unwrap_or(RewardDay::MAX)
}

/// Longest id or season name kept from `rewards.json` (the known ones are far shorter; a longer one is garbage).
const MAX_RECORD_ID_LEN: usize = 64;

// ---------------------------------------------------------------------------------------------------------------
// Items and keys
// ---------------------------------------------------------------------------------------------------------------

/// One collection item (core only, not IPC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unlockable {
    Theme(RewardTheme),
    Ambience(AmbienceLayer),
}

impl Unlockable {
    /// Every collection item in catalogue order: the themes, then the ambience layers (so a `withTheme` layer is
    /// evaluated after its theme, in the same call).
    pub fn all() -> impl Iterator<Item = Unlockable> {
        RewardTheme::ALL
            .into_iter()
            .map(Unlockable::Theme)
            .chain(AmbienceLayer::ALL.into_iter().map(Unlockable::Ambience))
    }

    /// How the item is earned.
    pub fn rule(self) -> UnlockRule {
        match self {
            Unlockable::Theme(theme) => theme_rule(theme),
            Unlockable::Ambience(layer) => ambience_rule(layer),
        }
    }
}

/// The ledger id of an item: `"theme:<RewardTheme serde name>"` / `"ambience:<AmbienceLayer serde name>"` (a fixed
/// table, no serde at runtime; a test keeps it in step with the serde names).
pub fn key(item: Unlockable) -> &'static str {
    match item {
        Unlockable::Theme(theme) => match theme {
            RewardTheme::Underwater => "theme:underwater",
            RewardTheme::RainyWindow => "theme:rainyWindow",
            RewardTheme::ForestCabin => "theme:forestCabin",
            RewardTheme::SakuraNight => "theme:sakuraNight",
            RewardTheme::NewYear => "theme:newYear",
            RewardTheme::Valentines => "theme:valentines",
            RewardTheme::Easter => "theme:easter",
            RewardTheme::Summer => "theme:summer",
            RewardTheme::Halloween => "theme:halloween",
            RewardTheme::Christmas => "theme:christmas",
        },
        Unlockable::Ambience(layer) => match layer {
            AmbienceLayer::Rain => "ambience:rain",
            AmbienceLayer::Wind => "ambience:wind",
            AmbienceLayer::Thunder => "ambience:thunder",
            AmbienceLayer::Cafe => "ambience:cafe",
            AmbienceLayer::Fireplace => "ambience:fireplace",
            AmbienceLayer::Forest => "ambience:forest",
            AmbienceLayer::Ocean => "ambience:ocean",
            AmbienceLayer::SpookyNight => "ambience:spookyNight",
            AmbienceLayer::WinterCottage => "ambience:winterCottage",
        },
    }
}

/// The item of a ledger id; `None` for an unknown one (a newer app's item: kept in the ledger, ignored here).
pub fn parse_key(id: &str) -> Option<Unlockable> {
    Unlockable::all().find(|item| key(*item) == id)
}

/// The ledger name of a season (the `Season` serde name).
pub fn season_key(season: Season) -> &'static str {
    match season {
        Season::NewYear => "newYear",
        Season::Valentines => "valentines",
        Season::Easter => "easter",
        Season::Summer => "summer",
        Season::Halloween => "halloween",
        Season::Christmas => "christmas",
    }
}

/// The season of a ledger name; `None` for an unknown one.
pub fn parse_season(name: &str) -> Option<Season> {
    Season::ALL
        .into_iter()
        .find(|season| season_key(*season) == name)
}

/// The collection theme earned in `season`'s window.
pub fn season_theme(season: Season) -> RewardTheme {
    match season {
        Season::NewYear => RewardTheme::NewYear,
        Season::Valentines => RewardTheme::Valentines,
        Season::Easter => RewardTheme::Easter,
        Season::Summer => RewardTheme::Summer,
        Season::Halloween => RewardTheme::Halloween,
        Season::Christmas => RewardTheme::Christmas,
    }
}

/// Goal days `season`'s theme needs inside one window.
fn season_goal_days(season: Season) -> u32 {
    match theme_rule(season_theme(season)) {
        UnlockRule::Seasonal { goal_days, .. } => goal_days,
        _ => SEASON_GOAL_DAYS,
    }
}

/// The day `item` was unlocked; `None` while it is not (a free item has no record).
pub fn unlocked_on(ledger: &Ledger, item: Unlockable) -> Option<RewardDay> {
    let id = key(item);
    ledger
        .unlocks
        .iter()
        .find(|record| record.id == id)
        .map(|record| record.day)
}

/// `item` is usable: free, or unlocked in `ledger`.
pub fn is_unlocked(ledger: &Ledger, item: Unlockable) -> bool {
    item.rule() == UnlockRule::Free || unlocked_on(ledger, item).is_some()
}

// ---------------------------------------------------------------------------------------------------------------
// Seasons
// ---------------------------------------------------------------------------------------------------------------

fn civil_year(day: RewardDay) -> i32 {
    i32::try_from(civil_from_days(i64::from(day)).0).unwrap_or(i32::MAX)
}

/// The season whose window contains `day`, with the window's year (windows never overlap and never cross a year).
pub fn season_on(day: RewardDay) -> Option<(Season, i32)> {
    let year = civil_year(day);
    Season::ALL
        .into_iter()
        .find(|&season| {
            let (start, end) = season_window(season, year);
            start <= day && day <= end
        })
        .map(|season| (season, year))
}

/// The window of `season` shown on `today`: the open one, else the next one (this year's while it is still ahead,
/// else next year's). `(year, first day, last day)`.
fn shown_window(season: Season, today: RewardDay) -> (i32, RewardDay, RewardDay) {
    let year = civil_year(today);
    let (start, end) = season_window(season, year);
    if today <= end {
        (year, start, end)
    } else {
        let next = year.saturating_add(1);
        let (start, end) = season_window(season, next);
        (next, start, end)
    }
}

/// The window shown for `season` on `today` ([`SeasonWindow`]: the open one, else the next one).
pub fn window_view(season: Season, today: RewardDay) -> SeasonWindow {
    let (year, start, end) = shown_window(season, today);
    SeasonWindow {
        season,
        start: date_string(start),
        end: date_string(end),
        open: start <= today && today <= end,
        month: window_month(season, year),
    }
}

fn season_progress(ledger: &Ledger, season: Season) -> Option<&SeasonProgress> {
    let name = season_key(season);
    ledger
        .seasons
        .iter()
        .find(|progress| progress.season == name)
}

// ---------------------------------------------------------------------------------------------------------------
// Recording and evaluation
// ---------------------------------------------------------------------------------------------------------------

/// The daily goal was met on `day` (real ledger only): adds `day` to the progress of the season whose window contains
/// it, unless that season's theme is unlocked already. A new window year replaces the season's old entry (October 2025
/// and October 2026 never add up). Idempotent: the same day twice counts once.
pub fn record_goal_day(ledger: &mut Ledger, day: RewardDay) {
    let Some((season, year)) = season_on(day) else {
        return;
    };
    if is_unlocked(ledger, Unlockable::Theme(season_theme(season))) {
        return;
    }
    let need = usize::try_from(season_goal_days(season)).unwrap_or(usize::MAX);
    let name = season_key(season);
    match ledger
        .seasons
        .iter_mut()
        .find(|progress| progress.season == name)
    {
        Some(progress) if progress.year == year => {
            if !progress.days.contains(&day) && progress.days.len() < need {
                progress.days.push(day);
                progress.days.sort_unstable();
            }
        }
        Some(progress) if progress.year < year => {
            progress.year = year;
            progress.days = vec![day];
        }
        // A later window is recorded already (a clock that went back a year): kept as it is.
        Some(_) => {}
        None => ledger.seasons.push(SeasonProgress {
            season: name.to_owned(),
            year,
            days: vec![day],
        }),
    }
}

/// The best streak a rule sees (the current one counts the same).
fn best_streak(ledger: &Ledger) -> u32 {
    ledger.streak.best.max(ledger.streak.current)
}

/// `rule` holds for `ledger` now (`level` = the level to compare).
fn holds(ledger: &Ledger, rule: UnlockRule, level: u32) -> bool {
    match rule {
        UnlockRule::Free => true,
        UnlockRule::Level { level: need } => level >= need,
        UnlockRule::BestStreak { days } => best_streak(ledger) >= days,
        UnlockRule::Seasonal { season, goal_days } => {
            season_progress(ledger, season).is_some_and(|progress| {
                u32::try_from(progress.days.len()).unwrap_or(u32::MAX) >= goal_days
            })
        }
        UnlockRule::WithTheme { theme } => is_unlocked(ledger, Unlockable::Theme(theme)),
    }
}

/// Unlocks every item whose rule holds now and that has no record yet (real ledger only), recorded with reward day
/// `today`; returns the new ones in catalogue order (themes first, so a `withTheme` layer unlocks in the same call as
/// its theme). State-based: the level from `total_sparks`, the best streak, a season's goal days. Free items are
/// never recorded. Idempotent: a second call returns nothing.
pub fn evaluate(ledger: &mut Ledger, today: RewardDay) -> Vec<Unlockable> {
    let level = level_for(ledger.total_sparks).level;
    let mut unlocked = Vec::new();
    for item in Unlockable::all() {
        if is_unlocked(ledger, item) || !holds(ledger, item.rule(), level) {
            continue;
        }
        ledger.unlocks.push(UnlockRecord {
            id: key(item).to_owned(),
            day: today,
        });
        unlocked.push(item);
    }
    unlocked
}

// ---------------------------------------------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------------------------------------------

fn progress(have: u32, need: u32) -> UnlockProgress {
    UnlockProgress {
        have: have.min(need),
        need,
    }
}

/// State, progress and unlock date of one item.
fn item_view(
    ledger: &Ledger,
    item: Unlockable,
    today: RewardDay,
    level: u32,
) -> (UnlockState, Option<UnlockProgress>, Option<String>) {
    let rule = item.rule();
    if rule == UnlockRule::Free {
        return (UnlockState::Free, None, None);
    }
    if let Some(day) = unlocked_on(ledger, item) {
        return (UnlockState::Unlocked, None, Some(date_string(day)));
    }
    match rule {
        UnlockRule::Level { level: need } => {
            (UnlockState::Locked, Some(progress(level, need)), None)
        }
        UnlockRule::BestStreak { days } => (
            UnlockState::Locked,
            Some(progress(best_streak(ledger), days)),
            None,
        ),
        UnlockRule::Seasonal { season, goal_days } => {
            let (year, start, end) = shown_window(season, today);
            if start <= today && today <= end {
                let have = season_progress(ledger, season)
                    .filter(|entry| entry.year == year)
                    .map_or(0, |entry| {
                        u32::try_from(entry.days.len()).unwrap_or(u32::MAX)
                    });
                (
                    UnlockState::AvailableNow,
                    Some(progress(have, goal_days)),
                    None,
                )
            } else {
                (UnlockState::Locked, None, None)
            }
        }
        UnlockRule::Free | UnlockRule::WithTheme { .. } => (UnlockState::Locked, None, None),
    }
}

/// The collection as `RewardsStatus.unlocks` shows it on reward day `today`, in catalogue order. `level` is the level
/// the view shows (it can trail `ledger.total_sparks` while a level-up waits in focus, so no unlock shows early).
/// Seasonal themes carry their window: the open one, else the next one.
pub fn view(ledger: &Ledger, today: RewardDay, level: u32) -> UnlocksView {
    UnlocksView {
        themes: RewardTheme::ALL
            .into_iter()
            .map(|theme| {
                let item = Unlockable::Theme(theme);
                let rule = item.rule();
                let (state, progress, unlocked_on) = item_view(ledger, item, today, level);
                let window = match rule {
                    UnlockRule::Seasonal { season, .. } => Some(window_view(season, today)),
                    _ => None,
                };
                ThemeUnlock {
                    theme,
                    state,
                    rule,
                    progress,
                    window,
                    unlocked_on,
                }
            })
            .collect(),
        ambiences: AmbienceLayer::ALL
            .into_iter()
            .map(|layer| {
                let item = Unlockable::Ambience(layer);
                let (state, progress, unlocked_on) = item_view(ledger, item, today, level);
                AmbienceUnlock {
                    layer,
                    state,
                    rule: item.rule(),
                    progress,
                    unlocked_on,
                }
            })
            .collect(),
    }
}

/// `update_config`'s check of `Config.reward_theme` (the shell passes whether a theme is unlocked in the real ledger):
/// a collection theme can be picked only once it is unlocked (`config.invalid`, field `rewardTheme`). `None` is always
/// fine, and so is the theme already stored (a stale but unchanged value never blocks other settings: on screen it
/// falls back to the base theme while it is locked).
pub fn check_reward_theme(
    previous: Option<RewardTheme>,
    next: Option<RewardTheme>,
    unlocked: impl Fn(RewardTheme) -> bool,
) -> Result<(), AppError> {
    match next {
        Some(theme) if next != previous && !unlocked(theme) => Err(AppError {
            code: CONFIG_INVALID.to_owned(),
            message: "collection theme is not unlocked".to_owned(),
            field: Some("rewardTheme".to_owned()),
        }),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Repair of a ledger read from disk
// ---------------------------------------------------------------------------------------------------------------

/// Keeps every known entry and the first unknown ones until `max` entries in all (order kept).
fn cap_unknown<T>(items: &mut Vec<T>, max: usize, known: impl Fn(&T) -> bool) {
    let known_count = items.iter().filter(|item| known(item)).count();
    let mut room = max.saturating_sub(known_count);
    items.retain(|item| {
        if known(item) {
            return true;
        }
        let keep = room > 0;
        room = room.saturating_sub(1);
        keep
    });
}

/// The collection part of `Ledger::normalize`: unlock ids trimmed, empty or overlong ones dropped, one record per id
/// (the earliest day), oldest first, at most [`MAX_UNLOCK_RECORDS`] (known ids always kept); seasons: one entry per
/// season name (the highest year; days of the same year merged), days inside the window (known seasons), sorted,
/// unique and at most [`SEASON_GOAL_DAYS`], empty entries dropped, at most [`MAX_SEASON_RECORDS`] (known seasons
/// always kept). Unknown ids and seasons are kept (a newer app's) and ignored by the evaluation and the view.
pub(crate) fn normalize(ledger: &mut Ledger) {
    let mut records: Vec<UnlockRecord> = Vec::new();
    for record in std::mem::take(&mut ledger.unlocks) {
        let id = record.id.trim();
        if id.is_empty() || id.len() > MAX_RECORD_ID_LEN {
            continue;
        }
        match records.iter_mut().find(|kept| kept.id == id) {
            Some(kept) => kept.day = kept.day.min(record.day),
            None => records.push(UnlockRecord {
                id: id.to_owned(),
                day: record.day,
            }),
        }
    }
    records.sort_by_key(|record| record.day);
    cap_unknown(&mut records, MAX_UNLOCK_RECORDS, |record| {
        parse_key(&record.id).is_some()
    });
    ledger.unlocks = records;

    let goal_days = usize::try_from(SEASON_GOAL_DAYS).unwrap_or(usize::MAX);
    let mut seasons: Vec<SeasonProgress> = Vec::new();
    for entry in std::mem::take(&mut ledger.seasons) {
        let name = entry.season.trim();
        if name.is_empty() || name.len() > MAX_RECORD_ID_LEN {
            continue;
        }
        let mut days = entry.days;
        if let Some(season) = parse_season(name) {
            let (start, end) = season_window(season, entry.year);
            days.retain(|day| start <= *day && *day <= end);
        }
        match seasons.iter_mut().find(|kept| kept.season == name) {
            Some(kept) if kept.year == entry.year => kept.days.extend(days),
            Some(kept) if kept.year < entry.year => {
                kept.year = entry.year;
                kept.days = days;
            }
            Some(_) => {}
            None => seasons.push(SeasonProgress {
                season: name.to_owned(),
                year: entry.year,
                days,
            }),
        }
    }
    for entry in &mut seasons {
        entry.days.sort_unstable();
        entry.days.dedup();
        entry.days.truncate(goal_days);
    }
    seasons.retain(|entry| !entry.days.is_empty());
    cap_unknown(&mut seasons, MAX_SEASON_RECORDS, |entry| {
        parse_season(&entry.season).is_some()
    });
    ledger.seasons = seasons;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_follows_the_enum_order() {
        let themes: Vec<_> = THEME_RULES.iter().map(|(theme, _)| *theme).collect();
        assert_eq!(themes, RewardTheme::ALL);
        let layers: Vec<_> = AMBIENCE_RULES.iter().map(|(layer, _)| *layer).collect();
        assert_eq!(layers, AmbienceLayer::ALL);
        assert_eq!(
            theme_rule(RewardTheme::Underwater),
            UnlockRule::Level { level: 3 }
        );
        assert_eq!(
            theme_rule(RewardTheme::Halloween),
            UnlockRule::Seasonal {
                season: Season::Halloween,
                goal_days: 3
            }
        );
        assert_eq!(ambience_rule(AmbienceLayer::Rain), UnlockRule::Free);
        assert_eq!(
            ambience_rule(AmbienceLayer::SpookyNight),
            UnlockRule::WithTheme {
                theme: RewardTheme::Halloween
            }
        );
        // Every seasonal theme has its own season, and every season one theme.
        let mut seasons: Vec<Season> = THEME_RULES
            .iter()
            .filter_map(|(_, rule)| match rule {
                UnlockRule::Seasonal { season, .. } => Some(*season),
                _ => None,
            })
            .collect();
        seasons.sort();
        assert_eq!(seasons, Season::ALL);
    }

    #[test]
    fn easter_sunday_2024_to_2030() {
        let known = [
            (2024, (3, 31)),
            (2025, (4, 20)),
            (2026, (4, 5)),
            (2027, (3, 28)),
            (2028, (4, 16)),
            (2029, (4, 1)),
            (2030, (4, 21)),
        ];
        for (year, date) in known {
            assert_eq!(easter_sunday(year), date, "{year}");
        }
        // The extremes of the Gregorian computus.
        assert_eq!(easter_sunday(2285), (3, 22));
        assert_eq!(easter_sunday(2038), (4, 25));
    }

    #[test]
    fn season_windows() {
        let window = |season, year| {
            let (start, end) = season_window(season, year);
            (date_string(start), date_string(end))
        };
        let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
        assert_eq!(
            window(Season::Halloween, 2026),
            pair("2026-10-01", "2026-10-31")
        );
        assert_eq!(
            window(Season::Christmas, 2026),
            pair("2026-12-01", "2026-12-31")
        );
        assert_eq!(
            window(Season::NewYear, 2027),
            pair("2027-01-01", "2027-01-31")
        );
        assert_eq!(
            window(Season::Valentines, 2028),
            pair("2028-02-01", "2028-02-29")
        );
        assert_eq!(
            window(Season::Summer, 2026),
            pair("2026-07-01", "2026-07-31")
        );
        // Easter: the month of Easter Sunday, at least a week on both sides of it.
        assert_eq!(
            window(Season::Easter, 2024),
            pair("2024-03-01", "2024-04-07")
        );
        assert_eq!(
            window(Season::Easter, 2025),
            pair("2025-04-01", "2025-04-30")
        );
        assert_eq!(
            window(Season::Easter, 2026),
            pair("2026-03-29", "2026-04-30")
        );
        assert_eq!(
            window(Season::Easter, 2027),
            pair("2027-03-01", "2027-04-04")
        );
        assert_eq!(
            window(Season::Easter, 2028),
            pair("2028-04-01", "2028-04-30")
        );
        assert_eq!(
            window(Season::Easter, 2029),
            pair("2029-03-25", "2029-04-30")
        );
        assert_eq!(
            window(Season::Easter, 2030),
            pair("2030-04-01", "2030-04-30")
        );
        assert_eq!(
            window(Season::Easter, 2038),
            pair("2038-04-01", "2038-05-02")
        );

        assert_eq!(window_month(Season::Halloween, 2026), Some(10));
        assert_eq!(window_month(Season::Easter, 2025), Some(4));
        assert_eq!(window_month(Season::Easter, 2026), None);
    }

    #[test]
    fn windows_never_overlap() {
        for year in 2024..=2060 {
            let mut windows: Vec<(RewardDay, RewardDay)> = Season::ALL
                .iter()
                .map(|&season| season_window(season, year))
                .collect();
            windows.sort();
            for pair in windows.windows(2) {
                assert!(pair[0].1 < pair[1].0, "{year}: {pair:?}");
            }
        }
    }

    fn day(year: i32, month: u32, dom: u32) -> RewardDay {
        civil_day(year, month, dom)
    }

    fn ids(ledger: &Ledger) -> Vec<&str> {
        ledger
            .unlocks
            .iter()
            .map(|record| record.id.as_str())
            .collect()
    }

    #[test]
    fn keys_follow_the_serde_names() {
        for item in Unlockable::all() {
            let (prefix, name) = match item {
                Unlockable::Theme(theme) => ("theme", serde_json::to_value(theme)),
                Unlockable::Ambience(layer) => ("ambience", serde_json::to_value(layer)),
            };
            let name = name.expect("serialize");
            assert_eq!(
                key(item),
                format!("{prefix}:{}", name.as_str().expect("a string"))
            );
            assert_eq!(parse_key(key(item)), Some(item));
        }
        assert_eq!(Unlockable::all().count(), 19);
        for unknown in [
            "theme:aurora",
            "halloween",
            "theme:Halloween",
            " theme:halloween",
            "",
        ] {
            assert_eq!(parse_key(unknown), None, "{unknown:?}");
        }
        for season in Season::ALL {
            let name = serde_json::to_value(season).expect("serialize");
            assert_eq!(Some(season_key(season)), name.as_str());
            assert_eq!(parse_season(season_key(season)), Some(season));
            // Every season unlocks the theme whose rule names it.
            assert_eq!(
                theme_rule(season_theme(season)),
                UnlockRule::Seasonal {
                    season,
                    goal_days: SEASON_GOAL_DAYS
                }
            );
        }
        assert_eq!(parse_season("diwali"), None);
    }

    #[test]
    fn the_season_of_a_day_and_the_window_shown() {
        let today = day(2026, 10, 13);
        assert_eq!(season_on(today), Some((Season::Halloween, 2026)));
        assert_eq!(season_on(day(2026, 10, 1)), Some((Season::Halloween, 2026)));
        assert_eq!(
            season_on(day(2026, 10, 31)),
            Some((Season::Halloween, 2026))
        );
        assert_eq!(season_on(day(2026, 11, 1)), None);
        assert_eq!(season_on(day(2026, 9, 30)), None);
        assert_eq!(season_on(day(2027, 1, 1)), Some((Season::NewYear, 2027)));
        assert_eq!(
            season_on(day(2026, 12, 31)),
            Some((Season::Christmas, 2026))
        );
        // Easter 2026 (5 April): 29 March … 30 April.
        assert_eq!(season_on(day(2026, 3, 28)), None);
        assert_eq!(season_on(day(2026, 3, 29)), Some((Season::Easter, 2026)));
        assert_eq!(season_on(day(2026, 4, 30)), Some((Season::Easter, 2026)));
        assert_eq!(season_on(day(2026, 5, 1)), None);

        let open = window_view(Season::Halloween, today);
        assert_eq!(
            open,
            SeasonWindow {
                season: Season::Halloween,
                start: "2026-10-01".into(),
                end: "2026-10-31".into(),
                open: true,
                month: Some(10),
            }
        );
        // After the window: next year's.
        let next = window_view(Season::Halloween, day(2026, 11, 1));
        assert_eq!(
            (
                next.start.as_str(),
                next.end.as_str(),
                next.open,
                next.month
            ),
            ("2027-10-01", "2027-10-31", false, Some(10))
        );
        // Still ahead this year: this year's.
        let christmas = window_view(Season::Christmas, today);
        assert_eq!(
            (christmas.start.as_str(), christmas.open),
            ("2026-12-01", false)
        );
        // Easter seen in October 2026: 2027's window, not one calendar month.
        let easter = window_view(Season::Easter, today);
        assert_eq!(
            (
                easter.start.as_str(),
                easter.end.as_str(),
                easter.open,
                easter.month
            ),
            ("2027-03-01", "2027-04-04", false, None)
        );
        assert_eq!(window_view(Season::Easter, day(2026, 1, 10)).month, None);
        assert_eq!(window_view(Season::Easter, day(2025, 1, 10)).month, Some(4));
    }

    #[test]
    fn goal_days_are_recorded_once_per_day_and_per_window() {
        let mut ledger = Ledger::new(1);
        // Outside every window: nothing.
        record_goal_day(&mut ledger, day(2026, 9, 15));
        assert!(ledger.seasons.is_empty());
        // The same day twice counts once.
        record_goal_day(&mut ledger, day(2026, 10, 2));
        record_goal_day(&mut ledger, day(2026, 10, 2));
        record_goal_day(&mut ledger, day(2026, 10, 1));
        assert_eq!(
            ledger.seasons,
            [SeasonProgress {
                season: "halloween".into(),
                year: 2026,
                days: vec![day(2026, 10, 1), day(2026, 10, 2)],
            }]
        );
        // At most the days the theme needs.
        for dom in 3..=6 {
            record_goal_day(&mut ledger, day(2026, 10, dom));
        }
        assert_eq!(ledger.seasons[0].days.len(), 3);
        // A new window year replaces the old entry; an older one never comes back.
        let mut years = Ledger::new(1);
        record_goal_day(&mut years, day(2025, 10, 10));
        record_goal_day(&mut years, day(2025, 10, 11));
        record_goal_day(&mut years, day(2026, 10, 10));
        assert_eq!(years.seasons[0].year, 2026);
        assert_eq!(years.seasons[0].days, [day(2026, 10, 10)]);
        record_goal_day(&mut years, day(2025, 10, 12));
        assert_eq!(years.seasons[0].days, [day(2026, 10, 10)]);
        assert!(evaluate(&mut years, day(2026, 10, 10)).is_empty());
        // Once the theme is unlocked, its season records nothing more.
        years.unlocks.push(UnlockRecord {
            id: key(Unlockable::Theme(RewardTheme::Halloween)).into(),
            day: day(2026, 10, 10),
        });
        record_goal_day(&mut years, day(2026, 10, 20));
        assert_eq!(years.seasons[0].days, [day(2026, 10, 10)]);
        // Each season keeps its own entry.
        record_goal_day(&mut years, day(2026, 12, 24));
        assert_eq!(years.seasons.len(), 2);
    }

    #[test]
    fn evaluation_is_state_based_idempotent_and_in_catalogue_order() {
        let today = day(2026, 10, 13);
        let mut ledger = Ledger::new(1);
        assert!(
            evaluate(&mut ledger, today).is_empty(),
            "free items are never recorded"
        );
        assert!(ledger.unlocks.is_empty());
        assert!(is_unlocked(
            &ledger,
            Unlockable::Ambience(AmbienceLayer::Rain)
        ));
        assert_eq!(
            unlocked_on(&ledger, Unlockable::Ambience(AmbienceLayer::Rain)),
            None
        );
        // Level 8 (840 sparks), a best streak of 5 and three Halloween goal days, all at once.
        ledger.total_sparks = 840;
        ledger.streak.best = 5;
        for dom in [1, 5, 9] {
            record_goal_day(&mut ledger, day(2026, 10, dom));
        }
        let unlocked = evaluate(&mut ledger, today);
        assert_eq!(
            unlocked,
            [
                Unlockable::Theme(RewardTheme::Underwater),
                Unlockable::Theme(RewardTheme::ForestCabin),
                Unlockable::Theme(RewardTheme::Halloween),
                Unlockable::Ambience(AmbienceLayer::Thunder),
                Unlockable::Ambience(AmbienceLayer::Cafe),
                Unlockable::Ambience(AmbienceLayer::Fireplace),
                Unlockable::Ambience(AmbienceLayer::Forest),
                Unlockable::Ambience(AmbienceLayer::Ocean),
                Unlockable::Ambience(AmbienceLayer::SpookyNight),
            ]
        );
        assert_eq!(
            ids(&ledger),
            [
                "theme:underwater",
                "theme:forestCabin",
                "theme:halloween",
                "ambience:thunder",
                "ambience:cafe",
                "ambience:fireplace",
                "ambience:forest",
                "ambience:ocean",
                "ambience:spookyNight"
            ]
        );
        assert!(ledger.unlocks.iter().all(|record| record.day == today));
        assert!(evaluate(&mut ledger, today + 1).is_empty(), "idempotent");
        // The current streak counts like the best one.
        let mut current = Ledger::new(1);
        current.streak.current = 7;
        assert_eq!(
            evaluate(&mut current, today),
            [
                Unlockable::Theme(RewardTheme::RainyWindow),
                Unlockable::Ambience(AmbienceLayer::Fireplace)
            ]
        );
        // A `withTheme` layer whose record is missing (an older file) follows its theme at the next evaluation.
        let mut partial = Ledger::new(1);
        partial.unlocks.push(UnlockRecord {
            id: "theme:christmas".into(),
            day: day(2025, 12, 20),
        });
        assert_eq!(
            evaluate(&mut partial, today),
            [Unlockable::Ambience(AmbienceLayer::WinterCottage)]
        );
    }

    #[test]
    fn a_reward_theme_needs_its_unlock() {
        let locked = |_: RewardTheme| false;
        let unlocked = |_: RewardTheme| true;
        let err =
            check_reward_theme(None, Some(RewardTheme::Halloween), locked).expect_err("locked");
        assert_eq!(err.code, CONFIG_INVALID);
        assert_eq!(err.field.as_deref(), Some("rewardTheme"));
        assert_eq!(
            check_reward_theme(None, Some(RewardTheme::Halloween), unlocked),
            Ok(())
        );
        assert_eq!(
            check_reward_theme(Some(RewardTheme::Halloween), None, locked),
            Ok(())
        );
        assert_eq!(check_reward_theme(None, None, locked), Ok(()));
        // Unchanged: never blocks other settings, even while it is locked (it falls back to the base theme).
        assert_eq!(
            check_reward_theme(
                Some(RewardTheme::Underwater),
                Some(RewardTheme::Underwater),
                locked
            ),
            Ok(())
        );
        assert!(
            check_reward_theme(
                Some(RewardTheme::Underwater),
                Some(RewardTheme::Christmas),
                |theme| theme == RewardTheme::Underwater
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_records_survive_a_round_trip_and_the_repair() {
        let json = r#"{
            "version": 1, "totalSparks": 10,
            "unlocks": [
                { "id": "theme:aurora", "day": 20700 },
                { "id": " theme:underwater ", "day": 20735 },
                { "id": "theme:underwater", "day": 20730 },
                { "id": "", "day": 20731 }
            ],
            "seasons": [
                { "season": "diwali", "year": 2026, "days": [20750] },
                { "season": "halloween", "year": 2025, "days": [20380] },
                { "season": "halloween", "year": 2026, "days": [20738, 20727, 20738, 20727, 20700, 20733, 20734] },
                { "season": "christmas", "year": 2026, "days": [] }
            ]
        }"#;
        let mut ledger: Ledger = serde_json::from_str(json).expect("loads");
        ledger.normalize();
        assert_eq!(ids(&ledger), ["theme:aurora", "theme:underwater"]);
        assert_eq!(ledger.unlocks[1].day, 20730, "the earliest day wins");
        assert_eq!(ledger.seasons.len(), 2);
        assert_eq!(ledger.seasons[0].season, "diwali");
        let halloween = &ledger.seasons[1];
        assert_eq!(halloween.year, 2026);
        // 20700 is 2026-09-04 (outside the window); sorted, unique, at most three.
        assert_eq!(halloween.days, [20727, 20733, 20734]);
        // Kept on save, ignored by the view.
        let saved = serde_json::to_string(&ledger).expect("save");
        assert!(saved.contains("theme:aurora") && saved.contains("diwali"));
        let back: Ledger = serde_json::from_str(&saved).expect("loads again");
        assert_eq!(back, ledger);
        let shown = view(&ledger, day(2026, 10, 13), 1);
        assert_eq!(shown.themes.len(), RewardTheme::ALL.len());
        let underwater = &shown.themes[0];
        assert_eq!(underwater.state, UnlockState::Unlocked);
        assert_eq!(underwater.unlocked_on.as_deref(), Some("2026-10-04"));
        // The repair keeps every known record and caps the unknown ones.
        let mut crowded = Ledger::new(1);
        crowded.unlocks = (0..100)
            .map(|n| UnlockRecord {
                id: format!("theme:future{n}"),
                day: n,
            })
            .chain(std::iter::once(UnlockRecord {
                id: "theme:halloween".into(),
                day: 500,
            }))
            .collect();
        crowded.normalize();
        assert_eq!(crowded.unlocks.len(), MAX_UNLOCK_RECORDS);
        assert!(is_unlocked(
            &crowded,
            Unlockable::Theme(RewardTheme::Halloween)
        ));
        assert_eq!(crowded.unlocks[0].id, "theme:future0");
        crowded.seasons = (0..30)
            .map(|n| SeasonProgress {
                season: format!("future{n}"),
                year: 2026,
                days: vec![n],
            })
            .chain(std::iter::once(SeasonProgress {
                season: "summer".into(),
                year: 2026,
                days: vec![day(2026, 7, 4)],
            }))
            .collect();
        crowded.normalize();
        assert_eq!(crowded.seasons.len(), MAX_SEASON_RECORDS);
        assert!(crowded.seasons.iter().any(|entry| entry.season == "summer"));
    }

    #[test]
    fn the_view_follows_the_table_of_states() {
        let today = day(2026, 10, 13);
        // A fresh ledger (also right after "Zacznij od nowa"): only rain and wind are usable.
        let fresh = view(&Ledger::new(1), today, 1);
        let states: Vec<UnlockState> = fresh.themes.iter().map(|item| item.state).collect();
        assert_eq!(
            states,
            [
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::Locked,
                UnlockState::AvailableNow,
                UnlockState::Locked,
            ]
        );
        let layers: Vec<(AmbienceLayer, UnlockState)> = fresh
            .ambiences
            .iter()
            .map(|item| (item.layer, item.state))
            .collect();
        assert_eq!(layers[0], (AmbienceLayer::Rain, UnlockState::Free));
        assert_eq!(layers[1], (AmbienceLayer::Wind, UnlockState::Free));
        assert!(
            layers[2..]
                .iter()
                .all(|(_, state)| *state == UnlockState::Locked)
        );
        assert!(
            fresh.ambiences[..2]
                .iter()
                .all(|item| item.progress.is_none()
                    && item.unlocked_on.is_none()
                    && item.rule == UnlockRule::Free)
        );
        // Level and best-streak progress, capped at the need.
        assert_eq!(
            fresh.themes[0].progress,
            Some(UnlockProgress { have: 1, need: 3 })
        );
        assert_eq!(
            fresh.themes[1].progress,
            Some(UnlockProgress { have: 0, need: 7 })
        );
        assert_eq!(fresh.themes[0].window, None);
        let mut ledger = Ledger::new(1);
        ledger.streak.best = 9;
        let shown = view(&ledger, today, 9);
        assert_eq!(
            shown.themes[0].progress,
            Some(UnlockProgress { have: 3, need: 3 })
        );
        assert_eq!(
            shown.themes[1].progress,
            Some(UnlockProgress { have: 7, need: 7 })
        );
        assert_eq!(
            shown.ambiences[4].progress,
            Some(UnlockProgress { have: 5, need: 5 })
        );
        // Seasonal: available now with 2 of 3 days in October, and the other seasons with their next window.
        record_goal_day(&mut ledger, day(2026, 10, 2));
        record_goal_day(&mut ledger, day(2026, 10, 7));
        let october = view(&ledger, today, 1);
        let halloween = &october.themes[8];
        assert_eq!(halloween.theme, RewardTheme::Halloween);
        assert_eq!(halloween.state, UnlockState::AvailableNow);
        assert_eq!(
            halloween.progress,
            Some(UnlockProgress { have: 2, need: 3 })
        );
        assert_eq!(
            halloween
                .window
                .as_ref()
                .map(|window| (window.open, window.month)),
            Some((true, Some(10)))
        );
        assert_eq!(halloween.unlocked_on, None);
        let easter = &october.themes[6];
        assert_eq!((easter.state, easter.progress), (UnlockState::Locked, None));
        assert_eq!(
            easter.window.as_ref().map(|window| window.month),
            Some(None),
            "Easter 2027 is not one calendar month"
        );
        // A `withTheme` layer stays locked (no progress of its own) until its theme unlocks.
        let spooky = &october.ambiences[7];
        assert_eq!(
            (spooky.layer, spooky.state, spooky.progress),
            (AmbienceLayer::SpookyNight, UnlockState::Locked, None)
        );
        // November: the October days no longer count; the next window is October 2027.
        let november = view(&ledger, day(2026, 11, 2), 1);
        let halloween = &november.themes[8];
        assert_eq!(
            (halloween.state, halloween.progress),
            (UnlockState::Locked, None)
        );
        let window = halloween.window.as_ref().expect("window");
        assert_eq!((window.start.as_str(), window.open), ("2027-10-01", false));
        // Unlocked: the record day, no progress; a seasonal theme still says when its window comes again.
        record_goal_day(&mut ledger, day(2026, 10, 12));
        evaluate(&mut ledger, day(2026, 10, 12));
        let after = view(&ledger, day(2026, 11, 2), 1);
        let halloween = &after.themes[8];
        assert_eq!(halloween.state, UnlockState::Unlocked);
        assert_eq!(halloween.progress, None);
        assert_eq!(halloween.unlocked_on.as_deref(), Some("2026-10-12"));
        assert_eq!(
            halloween
                .window
                .as_ref()
                .map(|window| window.start.as_str()),
            Some("2027-10-01")
        );
        assert_eq!(after.ambiences[7].state, UnlockState::Unlocked);
        assert_eq!(
            after.ambiences[7].unlocked_on.as_deref(),
            Some("2026-10-12")
        );
    }
}
