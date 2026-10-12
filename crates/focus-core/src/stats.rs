//! Productivity statistics: foreground-app usage during focus and the session summary.

use std::cmp::Reverse;
use std::collections::HashMap;

use crate::api::{AppUsage, SessionSummary, UsageReport};
use crate::session::SessionTotals;

const MS_PER_SEC: u64 = 1000;

/// Accumulates foreground samples of one session. Process names are grouped
/// case-insensitively; the first spelling seen is the one reported.
#[derive(Debug, Clone, Default)]
pub struct UsageAccumulator {
    /// Lowercase process name → (reported name, milliseconds).
    apps: HashMap<String, AppTime>,
    idle_ms: u64,
    unknown_ms: u64,
}

#[derive(Debug, Clone)]
struct AppTime {
    name: String,
    ms: u64,
}

impl UsageAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `ms` of focus time. `idle` counts as idle regardless of the process;
    /// `process` `None` (or blank) counts as unknown.
    pub fn add_sample(&mut self, process: Option<&str>, idle: bool, ms: u64) {
        if idle {
            self.idle_ms = self.idle_ms.saturating_add(ms);
            return;
        }
        match process.map(str::trim).filter(|name| !name.is_empty()) {
            Some(name) => {
                let entry = self
                    .apps
                    .entry(name.to_lowercase())
                    .or_insert_with(|| AppTime {
                        name: name.to_owned(),
                        ms: 0,
                    });
                entry.ms = entry.ms.saturating_add(ms);
            }
            None => self.unknown_ms = self.unknown_ms.saturating_add(ms),
        }
    }

    /// Usage so far: apps sorted by seconds (descending) then by name, each value rounded to
    /// whole seconds (apps that round to 0 s are left out); `tracked_secs` is the sum of all parts.
    pub fn report(&self) -> UsageReport {
        let mut apps: Vec<AppUsage> = self
            .apps
            .values()
            .map(|app| AppUsage {
                process: app.name.clone(),
                secs: ms_to_rounded_secs(app.ms),
            })
            .filter(|app| app.secs > 0)
            .collect();
        apps.sort_by_cached_key(|app| (Reverse(app.secs), app.process.to_lowercase()));

        let idle_secs = ms_to_rounded_secs(self.idle_ms);
        let unknown_secs = ms_to_rounded_secs(self.unknown_ms);
        let tracked_secs = apps
            .iter()
            .fold(idle_secs.saturating_add(unknown_secs), |sum, app| {
                sum.saturating_add(app.secs)
            });

        UsageReport {
            apps,
            idle_secs,
            unknown_secs,
            tracked_secs,
        }
    }

    /// Forgets all samples (new session).
    pub fn clear(&mut self) {
        self.apps.clear();
        self.idle_ms = 0;
        self.unknown_ms = 0;
    }

    /// No time recorded yet.
    pub fn is_empty(&self) -> bool {
        self.idle_ms == 0 && self.unknown_ms == 0 && self.apps.values().all(|app| app.ms == 0)
    }
}

/// Merges session totals and usage into the summary DTO kept in history.
pub fn summarize(totals: &SessionTotals, usage: UsageReport) -> SessionSummary {
    SessionSummary {
        session_id: totals.session_id.clone(),
        started_at_ms: totals.started_at_ms,
        ended_at_ms: totals.ended_at_ms,
        focused_secs: totals.focused_secs,
        break_secs: totals.break_secs,
        cycles_completed: totals.cycles_completed,
        usage,
        demo: totals.demo,
    }
}

fn ms_to_rounded_secs(ms: u64) -> u32 {
    u32::try_from(ms.saturating_add(MS_PER_SEC / 2) / MS_PER_SEC).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(process: &str, secs: u32) -> AppUsage {
        AppUsage {
            process: process.to_owned(),
            secs,
        }
    }

    #[test]
    fn empty_accumulator_reports_nothing() {
        let acc = UsageAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.report(), UsageReport::default());
    }

    #[test]
    fn groups_process_names_case_insensitively_keeping_first_spelling() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("Code.exe"), false, 1000);
        acc.add_sample(Some("code.EXE"), false, 2000);
        acc.add_sample(Some("  CODE.exe "), false, 1000);
        let report = acc.report();
        assert_eq!(report.apps, vec![usage("Code.exe", 4)]);
        assert_eq!(report.tracked_secs, 4);
    }

    #[test]
    fn idle_wins_over_process_and_missing_process_is_unknown() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("chrome.exe"), true, 3000);
        acc.add_sample(None, true, 1000);
        acc.add_sample(None, false, 2000);
        acc.add_sample(Some("   "), false, 1000);
        acc.add_sample(Some("chrome.exe"), false, 5000);
        let report = acc.report();
        assert_eq!(report.idle_secs, 4);
        assert_eq!(report.unknown_secs, 3);
        assert_eq!(report.apps, vec![usage("chrome.exe", 5)]);
        assert_eq!(report.tracked_secs, 12);
    }

    #[test]
    fn sorts_by_seconds_then_name() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("slack.exe"), false, 10_000);
        acc.add_sample(Some("Code.exe"), false, 60_000);
        acc.add_sample(Some("chrome.exe"), false, 10_000);
        acc.add_sample(Some("Teams.exe"), false, 30_000);
        acc.add_sample(Some("brave.exe"), false, 10_000);
        let report = acc.report();
        assert_eq!(
            report.apps,
            vec![
                usage("Code.exe", 60),
                usage("Teams.exe", 30),
                usage("brave.exe", 10),
                usage("chrome.exe", 10),
                usage("slack.exe", 10),
            ]
        );
        assert_eq!(report.tracked_secs, 120);
    }

    #[test]
    fn rounds_to_nearest_second_and_drops_zero_second_apps() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("a.exe"), false, 1499);
        acc.add_sample(Some("b.exe"), false, 1500);
        acc.add_sample(Some("tiny.exe"), false, 499);
        acc.add_sample(None, false, 500);
        acc.add_sample(None, true, 2499);
        let report = acc.report();
        assert_eq!(report.apps, vec![usage("b.exe", 2), usage("a.exe", 1)]);
        assert_eq!(report.unknown_secs, 1);
        assert_eq!(report.idle_secs, 2);
        assert_eq!(report.tracked_secs, 2 + 1 + 1 + 2);
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("a.exe"), false, u64::MAX);
        acc.add_sample(Some("a.exe"), false, u64::MAX);
        acc.add_sample(None, true, u64::MAX);
        let report = acc.report();
        assert_eq!(report.apps, vec![usage("a.exe", u32::MAX)]);
        assert_eq!(report.idle_secs, u32::MAX);
        assert_eq!(report.tracked_secs, u32::MAX);
    }

    #[test]
    fn clear_forgets_everything() {
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("Code.exe"), false, 5000);
        acc.add_sample(None, true, 5000);
        assert!(!acc.is_empty());
        acc.clear();
        assert!(acc.is_empty());
        assert_eq!(acc.report(), UsageReport::default());
        // A new spelling wins after clearing.
        acc.add_sample(Some("CODE.EXE"), false, 1000);
        assert_eq!(acc.report().apps, vec![usage("CODE.EXE", 1)]);
    }

    #[test]
    fn summarize_merges_totals_and_usage() {
        let totals = SessionTotals {
            session_id: "abc".to_owned(),
            started_at_ms: 1_000,
            ended_at_ms: 9_000,
            focused_secs: 5,
            break_secs: 2,
            cycles_completed: 1,
            demo: true,
        };
        let mut acc = UsageAccumulator::new();
        acc.add_sample(Some("Code.exe"), false, 5000);
        let report = acc.report();
        let summary = summarize(&totals, report.clone());
        assert_eq!(
            summary,
            SessionSummary {
                session_id: "abc".to_owned(),
                started_at_ms: 1_000,
                ended_at_ms: 9_000,
                focused_secs: 5,
                break_secs: 2,
                cycles_completed: 1,
                usage: report,
                demo: true,
            }
        );
    }
}
