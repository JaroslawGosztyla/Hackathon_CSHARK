//! RFC 5545 recurrence rules (`RRULE`): parsing and expansion.
//!
//! Supported: `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY` with `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` (with ordinals for
//! MONTHLY / YEARLY), `BYMONTHDAY`, `BYMONTH`, `BYSETPOS` and `WKST`. A rule with any other part (`BYYEARDAY`,
//! `BYWEEKNO`, `BYHOUR`, …) or another frequency is rejected ([`Rule::parse`] returns `None`); the caller then
//! keeps only the first occurrence (DTSTART) instead of guessing.
//!
//! Expansion works on wall-clock time of the event's zone: every occurrence keeps DTSTART's time of day, so a
//! 10:00 meeting stays at 10:00 across daylight saving time changes.

use super::time::{MS_PER_DAY, civil_from_days, days_from_civil, days_in_month, weekday};

/// Upper bound of periods (days / weeks / months / years) walked for one rule.
const MAX_PERIODS: i64 = 100_000;
/// Last year an occurrence may fall in.
const MAX_YEAR: i64 = 9999;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// `UNTIL` as written: UTC instant, local date-time, or date (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Until {
    Utc(i64),
    Wall(i64),
    Date(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rule {
    pub(crate) freq: Freq,
    /// At least 1.
    pub(crate) interval: u32,
    pub(crate) count: Option<u32>,
    pub(crate) until: Option<Until>,
    /// `(ordinal, weekday)`; ordinal 0 = every such weekday, weekday 0 = Monday … 6 = Sunday.
    pub(crate) by_day: Vec<(i32, u32)>,
    pub(crate) by_month_day: Vec<i32>,
    pub(crate) by_month: Vec<u32>,
    pub(crate) by_set_pos: Vec<i32>,
    /// Week start, 0 = Monday (default).
    pub(crate) wkst: u32,
}

impl Rule {
    /// Parses an `RRULE` value (`FREQ=WEEKLY;BYDAY=MO,WE`); `None` when it is invalid or uses unsupported parts.
    pub(crate) fn parse(text: &str) -> Option<Rule> {
        let mut freq = None;
        let mut rule = Rule {
            freq: Freq::Daily,
            interval: 1,
            count: None,
            until: None,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_set_pos: Vec::new(),
            wkst: 0,
        };
        for part in text.trim().split(';').filter(|p| !p.trim().is_empty()) {
            let (key, value) = part.split_once('=')?;
            let key = key.trim().to_ascii_uppercase();
            let value = value.trim().to_ascii_uppercase();
            match key.as_str() {
                "FREQ" => {
                    freq = Some(match value.as_str() {
                        "DAILY" => Freq::Daily,
                        "WEEKLY" => Freq::Weekly,
                        "MONTHLY" => Freq::Monthly,
                        "YEARLY" => Freq::Yearly,
                        _ => return None,
                    });
                }
                "INTERVAL" => rule.interval = value.parse::<u32>().ok()?.max(1),
                "COUNT" => {
                    let count = value.parse::<u32>().ok()?;
                    if count == 0 {
                        return None;
                    }
                    rule.count = Some(count);
                }
                "UNTIL" => rule.until = Some(parse_until(&value)?),
                "BYDAY" => rule.by_day = list(&value, parse_by_day)?,
                "BYMONTHDAY" => {
                    rule.by_month_day = list(&value, |v| int_in(v, 31).filter(|d| *d != 0))?;
                }
                "BYMONTH" => {
                    rule.by_month = list(&value, |v| {
                        v.parse::<u32>().ok().filter(|m| (1..=12).contains(m))
                    })?;
                }
                "BYSETPOS" => {
                    rule.by_set_pos = list(&value, |v| int_in(v, 366).filter(|p| *p != 0))?;
                }
                "WKST" => rule.wkst = weekday_code(&value)?,
                key if key.starts_with("X-") => {}
                _ => return None,
            }
        }
        rule.freq = freq?;
        if rule.count.is_some() && rule.until.is_some() {
            // Not allowed together (RFC 5545); be lenient and honour both.
            log::debug!("calendar: RRULE with both COUNT and UNTIL");
        }
        Some(rule)
    }

    /// A yearly rule on the `ordinal` `weekday` of `month` (e.g. last Sunday of October: `10, -1, 6`).
    pub(crate) fn yearly_nth(month: u32, ordinal: i32, weekday: u32) -> Rule {
        Rule {
            freq: Freq::Yearly,
            interval: 1,
            count: None,
            until: None,
            by_day: vec![(ordinal, weekday)],
            by_month_day: Vec::new(),
            by_month: vec![month],
            by_set_pos: Vec::new(),
            wkst: 0,
        }
    }

    /// The occurrence (local wall time `wall_ms`, instant `utc_ms`) is after `UNTIL`.
    pub(crate) fn is_after_until(&self, wall_ms: i64, utc_ms: i64) -> bool {
        match self.until {
            None => false,
            Some(Until::Utc(until)) => utc_ms > until,
            Some(Until::Wall(until)) => wall_ms > until,
            Some(Until::Date(day)) => wall_ms.div_euclid(MS_PER_DAY) > day,
        }
    }

    /// Candidate days of `year` for a YEARLY expansion (`start_month` / `start_day` from DTSTART fill in what the
    /// rule leaves open), sorted, before `BYSETPOS`.
    pub(crate) fn yearly_days(&self, year: i64, start_month: u32, start_day: u32) -> Vec<i64> {
        let mut days = Vec::new();
        if !self.by_month.is_empty() {
            let mut months = self.by_month.clone();
            months.sort_unstable();
            months.dedup();
            for month in months {
                days.extend(self.month_days(year, month, start_day));
            }
        } else if !self.by_month_day.is_empty() {
            for month in 1..=12 {
                days.extend(self.month_days(year, month, start_day));
            }
        } else if !self.by_day.is_empty() {
            let first = days_from_civil(year, 1, 1);
            let last = days_from_civil(year + 1, 1, 1) - 1;
            for &(ordinal, wd) in &self.by_day {
                days.extend(nth_weekday_in(first, last, ordinal, wd));
            }
        } else if start_day <= days_in_month(year, start_month) {
            days.push(days_from_civil(year, start_month, start_day));
        }
        days.sort_unstable();
        days.dedup();
        days
    }

    /// Candidate days of one month for MONTHLY / YEARLY expansion.
    fn month_days(&self, year: i64, month: u32, default_day: u32) -> Vec<i64> {
        if !self.by_month.is_empty() && !self.by_month.contains(&month) {
            return Vec::new();
        }
        let dim = days_in_month(year, month);
        if dim == 0 {
            return Vec::new();
        }
        let first = days_from_civil(year, month, 1);
        let last = first + i64::from(dim) - 1;
        let by_month_day: Option<Vec<i64>> = (!self.by_month_day.is_empty()).then(|| {
            self.by_month_day
                .iter()
                .filter_map(|&d| resolve_month_day(d, dim))
                .map(|d| first + i64::from(d) - 1)
                .collect()
        });
        let by_day: Option<Vec<i64>> = (!self.by_day.is_empty()).then(|| {
            self.by_day
                .iter()
                .flat_map(|&(ordinal, wd)| nth_weekday_in(first, last, ordinal, wd))
                .collect()
        });
        let mut days = match (by_month_day, by_day) {
            (Some(a), Some(b)) => a.into_iter().filter(|d| b.contains(d)).collect(),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) if default_day <= dim => vec![first + i64::from(default_day) - 1],
            (None, None) => Vec::new(),
        };
        days.sort_unstable();
        days.dedup();
        days
    }

    fn matches_day_filters(&self, day: i64) -> bool {
        let (year, month, dom) = civil_from_days(day);
        if !self.by_month.is_empty() && !self.by_month.contains(&month) {
            return false;
        }
        if !self.by_month_day.is_empty() {
            let dim = days_in_month(year, month);
            if !self
                .by_month_day
                .iter()
                .any(|&d| resolve_month_day(d, dim) == Some(dom))
            {
                return false;
            }
        }
        if !self.by_day.is_empty() && !self.by_day.iter().any(|&(_, wd)| wd == weekday(day)) {
            return false;
        }
        true
    }

    /// First day of the week containing `day`, by `WKST`.
    fn week_start(&self, day: i64) -> i64 {
        day - i64::from((weekday(day) + 7 - self.wkst) % 7)
    }

    /// Period `index` (0 = the one containing DTSTART): its first day and its candidate days (sorted, after
    /// `BYSETPOS`). `None` past year 9999.
    fn period(&self, index: i64, start_day: i64) -> Option<(i64, Vec<i64>)> {
        let (y0, m0, d0) = civil_from_days(start_day);
        let step = index.checked_mul(i64::from(self.interval))?;
        let (first, days) = match self.freq {
            Freq::Daily => {
                let day = start_day.checked_add(step)?;
                let days = if self.matches_day_filters(day) {
                    vec![day]
                } else {
                    Vec::new()
                };
                (day, days)
            }
            Freq::Weekly => {
                let first = self
                    .week_start(start_day)
                    .checked_add(step.checked_mul(7)?)?;
                let mut days: Vec<i64> = if self.by_day.is_empty() {
                    vec![first + i64::from((weekday(start_day) + 7 - self.wkst) % 7)]
                } else {
                    self.by_day
                        .iter()
                        .map(|&(_, wd)| first + i64::from((wd + 7 - self.wkst) % 7))
                        .collect()
                };
                days.retain(|&day| {
                    let (_, month, _) = civil_from_days(day);
                    self.by_month.is_empty() || self.by_month.contains(&month)
                });
                days.sort_unstable();
                days.dedup();
                (first, days)
            }
            Freq::Monthly => {
                let month_index = (y0 * 12 + i64::from(m0) - 1).checked_add(step)?;
                let year = month_index.div_euclid(12);
                let month = (month_index.rem_euclid(12) + 1) as u32;
                if year > MAX_YEAR {
                    return None;
                }
                (
                    days_from_civil(year, month, 1),
                    self.month_days(year, month, d0),
                )
            }
            Freq::Yearly => {
                let year = y0.checked_add(step)?;
                if year > MAX_YEAR {
                    return None;
                }
                (days_from_civil(year, 1, 1), self.yearly_days(year, m0, d0))
            }
        };
        if civil_from_days(first).0 > MAX_YEAR {
            return None;
        }
        Some((first, apply_set_pos(days, &self.by_set_pos)))
    }

    /// Index of the first period that can contain `day` or a later day (only valid without `COUNT`, which has to
    /// be counted from DTSTART).
    fn period_index_near(&self, start_day: i64, day: i64) -> i64 {
        let interval = i64::from(self.interval);
        let periods = match self.freq {
            Freq::Daily => day - start_day,
            Freq::Weekly => (day - self.week_start(start_day)).div_euclid(7),
            Freq::Monthly => {
                let (y0, m0, _) = civil_from_days(start_day);
                let (y, m, _) = civil_from_days(day);
                (y * 12 + i64::from(m)) - (y0 * 12 + i64::from(m0))
            }
            Freq::Yearly => civil_from_days(day).0 - civil_from_days(start_day).0,
        };
        periods.div_euclid(interval).max(0)
    }
}

/// [`expand_budgeted`] without a shared budget.
#[cfg(test)]
pub(crate) fn expand(
    rule: &Rule,
    start_wall: i64,
    lo_wall: i64,
    hi_wall: i64,
    to_utc: &dyn Fn(i64) -> i64,
    max: usize,
) -> Vec<i64> {
    let mut budget = u64::MAX;
    expand_budgeted(rule, start_wall, lo_wall, hi_wall, to_utc, max, &mut budget)
}

/// Occurrence starts (local wall ms) within `[lo_wall, hi_wall]`, in order, at most `max`.
///
/// DTSTART (`start_wall`) is always the first occurrence and counts towards `COUNT`. `to_utc` converts a wall time
/// of the event's zone to an instant (for a UTC `UNTIL`).
///
/// `budget` is the number of periods (days / weeks / months / years) still allowed to be walked; every period
/// walked takes one and the expansion stops early when none is left. One budget is shared by all the rules of a
/// calendar file, so a hostile file (thousands of `COUNT` rules starting decades ago) cannot keep the fetch busy
/// for minutes.
pub(crate) fn expand_budgeted(
    rule: &Rule,
    start_wall: i64,
    lo_wall: i64,
    hi_wall: i64,
    to_utc: &dyn Fn(i64) -> i64,
    max: usize,
    budget: &mut u64,
) -> Vec<i64> {
    let mut out = Vec::new();
    if max == 0 {
        return out;
    }
    if (lo_wall..=hi_wall).contains(&start_wall) {
        out.push(start_wall);
    }
    let mut counted: u32 = 1;
    if rule.count == Some(1) || out.len() >= max {
        return out;
    }
    let start_day = start_wall.div_euclid(MS_PER_DAY);
    let time_of_day = start_wall.rem_euclid(MS_PER_DAY);
    let first_period = if rule.count.is_none() {
        rule.period_index_near(start_day, lo_wall.div_euclid(MS_PER_DAY) - 1)
    } else {
        0
    };
    for index in first_period..first_period.saturating_add(MAX_PERIODS) {
        let Some(left) = budget.checked_sub(1) else {
            break;
        };
        *budget = left;
        let Some((period_first, days)) = rule.period(index, start_day) else {
            break;
        };
        if period_first.saturating_mul(MS_PER_DAY) > hi_wall {
            break;
        }
        for day in days {
            let wall = day * MS_PER_DAY + time_of_day;
            if wall <= start_wall {
                continue;
            }
            if wall > hi_wall || rule.is_after_until(wall, to_utc(wall)) {
                return out;
            }
            if let Some(count) = rule.count {
                if counted >= count {
                    return out;
                }
                counted += 1;
            }
            if wall >= lo_wall {
                out.push(wall);
                if out.len() >= max {
                    return out;
                }
            }
        }
    }
    out
}

fn parse_until(value: &str) -> Option<Until> {
    if let Some((wall, utc)) = super::time::parse_ical_date_time(value) {
        return Some(if utc {
            Until::Utc(wall)
        } else {
            Until::Wall(wall)
        });
    }
    super::time::parse_ical_date(value).map(Until::Date)
}

fn list<T>(value: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(parse)
        .collect()
}

fn int_in(value: &str, limit: i32) -> Option<i32> {
    value
        .trim_start_matches('+')
        .parse::<i32>()
        .ok()
        .filter(|v| v.abs() <= limit)
}

fn weekday_code(code: &str) -> Option<u32> {
    Some(match code {
        "MO" => 0,
        "TU" => 1,
        "WE" => 2,
        "TH" => 3,
        "FR" => 4,
        "SA" => 5,
        "SU" => 6,
        _ => return None,
    })
}

/// `MO`, `1MO`, `-1FR`, `+2TU` → (ordinal, weekday).
fn parse_by_day(value: &str) -> Option<(i32, u32)> {
    if value.len() < 2 || !value.is_ascii() {
        return None;
    }
    let (ordinal, code) = value.split_at(value.len() - 2);
    let ordinal = if ordinal.is_empty() {
        0
    } else {
        int_in(ordinal, 53).filter(|o| *o != 0)?
    };
    Some((ordinal, weekday_code(code)?))
}

/// Day of month for a (possibly negative) `BYMONTHDAY` value.
fn resolve_month_day(value: i32, days_in_month: u32) -> Option<u32> {
    let dim = days_in_month as i32;
    let day = if value > 0 { value } else { dim + value + 1 };
    (1..=dim).contains(&day).then_some(day as u32)
}

/// Days in `[first, last]` that are `weekday`: all of them (ordinal 0), the n-th, or the n-th from the end.
fn nth_weekday_in(first: i64, last: i64, ordinal: i32, wd: u32) -> Vec<i64> {
    let first_match = first + i64::from((wd + 7 - weekday(first)) % 7);
    let all: Vec<i64> = (0..)
        .map(|week| first_match + 7 * week)
        .take_while(|day| *day <= last)
        .collect();
    match ordinal {
        0 => all,
        n if n > 0 => all.get(n as usize - 1).copied().into_iter().collect(),
        n => {
            let back = n.unsigned_abs() as usize;
            all.len()
                .checked_sub(back)
                .and_then(|i| all.get(i).copied())
                .into_iter()
                .collect()
        }
    }
}

fn apply_set_pos(days: Vec<i64>, positions: &[i32]) -> Vec<i64> {
    if positions.is_empty() {
        return days;
    }
    let len = days.len() as i64;
    let mut picked: Vec<i64> = positions
        .iter()
        .filter_map(|&pos| {
            let index = if pos > 0 {
                i64::from(pos) - 1
            } else {
                len + i64::from(pos)
            };
            (0..len).contains(&index).then(|| days[index as usize])
        })
        .collect();
    picked.sort_unstable();
    picked.dedup();
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::time::{MS_PER_HOUR, MS_PER_MINUTE};

    fn wall(y: i64, m: u32, d: u32, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * MS_PER_DAY + h * MS_PER_HOUR + min * MS_PER_MINUTE
    }

    fn ymd(wall: i64) -> (i64, u32, u32) {
        civil_from_days(wall.div_euclid(MS_PER_DAY))
    }

    fn all(rule: &str, start: i64, hi: i64) -> Vec<(i64, u32, u32)> {
        let rule = Rule::parse(rule).expect("rule");
        expand(&rule, start, i64::MIN / 4, hi, &|w| w, 10_000)
            .into_iter()
            .map(ymd)
            .collect()
    }

    #[test]
    fn parses_parts() {
        let rule =
            Rule::parse("FREQ=WEEKLY;UNTIL=20261231T083000Z;INTERVAL=2;BYDAY=MO,-1FR,+2TU;WKST=SU")
                .expect("rule");
        assert_eq!(rule.freq, Freq::Weekly);
        assert_eq!(rule.interval, 2);
        assert_eq!(rule.wkst, 6);
        assert_eq!(rule.by_day, vec![(0, 0), (-1, 4), (2, 1)]);
        assert_eq!(rule.until, Some(Until::Utc(wall(2026, 12, 31, 8, 30))));
        let rule = Rule::parse("freq=monthly;bymonthday=-1,15;count=3;X-FOO=1").expect("rule");
        assert_eq!(rule.by_month_day, vec![-1, 15]);
        assert_eq!(rule.count, Some(3));
        assert_eq!(
            Rule::parse("FREQ=YEARLY;UNTIL=20301231").map(|r| r.until),
            Some(Some(Until::Date(days_from_civil(2030, 12, 31))))
        );

        for unsupported in [
            "FREQ=HOURLY",
            "FREQ=YEARLY;BYWEEKNO=20",
            "FREQ=YEARLY;BYYEARDAY=100",
            "FREQ=DAILY;BYHOUR=9",
            "BYDAY=MO",
            "FREQ=WEEKLY;BYDAY=XX",
            "FREQ=WEEKLY;COUNT=0",
            "FREQ=MONTHLY;BYMONTHDAY=0",
            "FREQ=MONTHLY;BYMONTH=13",
            "FREQ=DAILY;UNTIL=tomorrow",
            "FREQ",
        ] {
            assert_eq!(Rule::parse(unsupported), None, "{unsupported}");
        }
    }

    #[test]
    fn weekly_workdays_with_count() {
        // Monday 2026-10-12.
        let got = all(
            "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;COUNT=7",
            wall(2026, 10, 12, 9, 30),
            i64::MAX / 4,
        );
        assert_eq!(
            got,
            vec![
                (2026, 10, 12),
                (2026, 10, 13),
                (2026, 10, 14),
                (2026, 10, 15),
                (2026, 10, 16),
                (2026, 10, 19),
                (2026, 10, 20)
            ]
        );
    }

    #[test]
    fn biweekly_respects_week_start() {
        // RFC 5545 example: the WKST changes the result of an every-other-week rule.
        let start = wall(1997, 8, 5, 9, 0); // Tuesday
        assert_eq!(
            all(
                "FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU;WKST=MO",
                start,
                i64::MAX / 4
            ),
            vec![(1997, 8, 5), (1997, 8, 10), (1997, 8, 19), (1997, 8, 24)]
        );
        assert_eq!(
            all(
                "FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU;WKST=SU",
                start,
                i64::MAX / 4
            ),
            vec![(1997, 8, 5), (1997, 8, 17), (1997, 8, 19), (1997, 8, 31)]
        );
    }

    #[test]
    fn monthly_ordinals_and_set_pos() {
        let start = wall(2026, 1, 13, 10, 0); // 2nd Tuesday of January
        assert_eq!(
            all("FREQ=MONTHLY;BYDAY=2TU;COUNT=4", start, i64::MAX / 4),
            vec![(2026, 1, 13), (2026, 2, 10), (2026, 3, 10), (2026, 4, 14)]
        );
        let start = wall(2026, 1, 30, 15, 0); // last Friday of January
        assert_eq!(
            all("FREQ=MONTHLY;BYDAY=-1FR;COUNT=3", start, i64::MAX / 4),
            vec![(2026, 1, 30), (2026, 2, 27), (2026, 3, 27)]
        );
        // Last workday of the month (Outlook writes it with BYSETPOS).
        let start = wall(2026, 1, 30, 15, 0);
        assert_eq!(
            all(
                "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1;COUNT=3",
                start,
                i64::MAX / 4
            ),
            vec![(2026, 1, 30), (2026, 2, 27), (2026, 3, 31)]
        );
        // The 31st only in months that have one.
        let start = wall(2026, 1, 31, 8, 0);
        assert_eq!(
            all("FREQ=MONTHLY;COUNT=4", start, i64::MAX / 4),
            vec![(2026, 1, 31), (2026, 3, 31), (2026, 5, 31), (2026, 7, 31)]
        );
        // Every last day of the month, every 2 months.
        let start = wall(2026, 1, 31, 8, 0);
        assert_eq!(
            all(
                "FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=-1;COUNT=3",
                start,
                i64::MAX / 4
            ),
            vec![(2026, 1, 31), (2026, 3, 31), (2026, 5, 31)]
        );
    }

    #[test]
    fn yearly_and_daily() {
        let start = wall(2024, 2, 29, 12, 0);
        assert_eq!(
            all("FREQ=YEARLY;COUNT=3", start, i64::MAX / 4),
            vec![(2024, 2, 29), (2028, 2, 29), (2032, 2, 29)]
        );
        let start = wall(2026, 3, 29, 2, 0);
        assert_eq!(
            all(
                "FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU;COUNT=3",
                start,
                i64::MAX / 4
            ),
            vec![(2026, 3, 29), (2027, 3, 28), (2028, 3, 26)]
        );
        // Thanksgiving: 4th Thursday of November.
        let start = wall(2026, 11, 26, 18, 0);
        assert_eq!(
            all(
                "FREQ=YEARLY;BYMONTH=11;BYDAY=4TH;COUNT=2",
                start,
                i64::MAX / 4
            ),
            vec![(2026, 11, 26), (2027, 11, 25)]
        );
        let start = wall(2026, 10, 9, 9, 0); // Friday
        assert_eq!(
            all(
                "FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;UNTIL=20261014T235959Z",
                start,
                i64::MAX / 4
            ),
            vec![
                (2026, 10, 9),
                (2026, 10, 12),
                (2026, 10, 13),
                (2026, 10, 14)
            ]
        );
        let start = wall(2026, 10, 9, 9, 0);
        assert_eq!(
            all("FREQ=DAILY;INTERVAL=3;UNTIL=20261018", start, i64::MAX / 4),
            vec![
                (2026, 10, 9),
                (2026, 10, 12),
                (2026, 10, 15),
                (2026, 10, 18)
            ]
        );
    }

    #[test]
    fn until_in_utc_uses_the_zone() {
        // 09:00 local at UTC+2 is 07:00 UTC: an UNTIL of 07:30 UTC on the 14th keeps the 14th.
        let rule = Rule::parse("FREQ=DAILY;UNTIL=20261014T073000Z").expect("rule");
        let got: Vec<_> = expand(
            &rule,
            wall(2026, 10, 12, 9, 0),
            i64::MIN / 4,
            i64::MAX / 4,
            &|w| w - 2 * MS_PER_HOUR,
            100,
        )
        .into_iter()
        .map(ymd)
        .collect();
        assert_eq!(got, vec![(2026, 10, 12), (2026, 10, 13), (2026, 10, 14)]);
    }

    #[test]
    fn fast_forward_matches_full_walk() {
        let cases = [
            ("FREQ=DAILY", wall(2010, 5, 3, 9, 15)),
            (
                "FREQ=DAILY;INTERVAL=3;BYDAY=MO,WE,FR",
                wall(2011, 1, 3, 9, 15),
            ),
            ("FREQ=WEEKLY;BYDAY=MO,TH", wall(2012, 6, 4, 13, 0)),
            (
                "FREQ=WEEKLY;INTERVAL=3;WKST=SU;BYDAY=SU,TU",
                wall(2015, 2, 3, 7, 0),
            ),
            ("FREQ=MONTHLY;BYDAY=1MO,-1FR", wall(2009, 9, 7, 16, 0)),
            (
                "FREQ=MONTHLY;INTERVAL=5;BYMONTHDAY=1,-1",
                wall(2001, 1, 1, 16, 0),
            ),
            (
                "FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU",
                wall(1999, 10, 31, 3, 0),
            ),
            ("FREQ=YEARLY;INTERVAL=2", wall(2000, 10, 14, 3, 0)),
        ];
        let lo = wall(2026, 10, 1, 0, 0);
        let hi = wall(2027, 1, 31, 0, 0);
        for (text, start) in cases {
            let rule = Rule::parse(text).expect("rule");
            let fast = expand(&rule, start, lo, hi, &|w| w, 10_000);
            let full: Vec<i64> = expand(&rule, start, i64::MIN / 4, hi, &|w| w, usize::MAX)
                .into_iter()
                .filter(|w| *w >= lo)
                .collect();
            assert_eq!(fast, full, "{text}");
            assert!(!fast.is_empty(), "{text}");
        }
    }

    #[test]
    fn limits_hold() {
        let rule = Rule::parse("FREQ=DAILY").expect("rule");
        let start = wall(2026, 1, 1, 9, 0);
        assert_eq!(
            expand(&rule, start, start, wall(2030, 1, 1, 0, 0), &|w| w, 5).len(),
            5
        );
        assert!(expand(&rule, start, start, start, &|w| w, 0).is_empty());
        // Window before DTSTART: nothing.
        assert!(
            expand(
                &rule,
                start,
                wall(2025, 1, 1, 0, 0),
                wall(2025, 2, 1, 0, 0),
                &|w| w,
                10
            )
            .is_empty()
        );
        // An impossible date ends at the period cap / year limit instead of looping forever.
        let rule = Rule::parse("FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=30").expect("rule");
        assert_eq!(
            expand(&rule, start, start, i64::MAX / 4, &|w| w, 10),
            vec![start]
        );
        let rule = Rule::parse("FREQ=YEARLY;INTERVAL=4000").expect("rule");
        assert_eq!(
            expand(&rule, start, start, i64::MAX / 4, &|w| w, 10).len(),
            2,
            "2026 and 6026; 10026 is past year 9999"
        );
    }

    #[test]
    fn a_shared_budget_bounds_the_work() {
        let rule = Rule::parse("FREQ=DAILY;COUNT=99999999").expect("rule");
        let start = wall(1970, 1, 1, 9, 0);
        let (lo, hi) = (wall(2026, 10, 13, 0, 0), wall(2026, 10, 14, 0, 0));
        let mut budget = u64::MAX;
        let all = expand_budgeted(&rule, start, lo, hi, &|w| w, 100, &mut budget);
        assert_eq!(all, vec![wall(2026, 10, 13, 9, 0)]);
        let walked = u64::MAX - budget;
        assert!(walked > 20_000, "a COUNT rule walks from DTSTART: {walked}");

        // Each walked period takes one unit; an exhausted budget stops at once and stays at zero.
        let mut budget = 1_000;
        assert!(expand_budgeted(&rule, start, lo, hi, &|w| w, 100, &mut budget).is_empty());
        assert_eq!(budget, 0);
        assert!(expand_budgeted(&rule, start, lo, hi, &|w| w, 100, &mut budget).is_empty());
        // DTSTART inside the window is still returned without any budget.
        assert_eq!(
            expand_budgeted(&rule, lo + 1, lo, hi, &|w| w, 100, &mut budget),
            vec![lo + 1]
        );
        assert_eq!(budget, 0);
    }

    #[test]
    fn helper_math() {
        assert_eq!(resolve_month_day(-1, 28), Some(28));
        assert_eq!(resolve_month_day(31, 30), None);
        assert_eq!(resolve_month_day(-31, 30), None);
        let first = days_from_civil(2026, 10, 1);
        let last = days_from_civil(2026, 10, 31);
        let sundays = nth_weekday_in(first, last, 0, 6);
        assert_eq!(sundays.len(), 4);
        assert_eq!(
            nth_weekday_in(first, last, -1, 6),
            vec![days_from_civil(2026, 10, 25)]
        );
        assert_eq!(nth_weekday_in(first, last, 5, 6), Vec::<i64>::new());
        assert_eq!(apply_set_pos(vec![1, 2, 3], &[-1, 1, 7]), vec![1, 3]);
    }
}
