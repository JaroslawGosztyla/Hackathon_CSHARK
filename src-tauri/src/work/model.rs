//! Pure parts of the work service ("Co dalej?"): per-source settings and statuses, seen / announced ids,
//! items held back during focus, polling and backoff intervals, token validation and error redaction.
//! No IO, no clock (times come in as arguments), so everything here is unit-tested.

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::time::{Duration, Instant};

use focus_core::api::{
    IntegrationState, Phase, WorkConfig, WorkItem, WorkSource, WorkSourceStatus,
};
use serde::{Deserialize, Serialize};

/// The sources in the order of `WorkStatus::sources`.
pub const SOURCES: [WorkSource; 5] = [
    WorkSource::Github,
    WorkSource::Gitlab,
    WorkSource::Jira,
    WorkSource::AzureDevops,
    WorkSource::Linear,
];
/// Regular polling interval of every source.
pub const POLL_EVERY: Duration = Duration::from_secs(5 * 60);
/// First retry after a failed fetch; doubled after every further failure up to [`MAX_BACKOFF`].
pub const FIRST_BACKOFF: Duration = Duration::from_secs(60);
/// Longest wait after failed fetches.
pub const MAX_BACKOFF: Duration = Duration::from_secs(30 * 60);
/// A poll hint (GitHub `X-Poll-Interval`) above this is capped (a broken header must not stop polling).
pub const MAX_POLL_HINT: Duration = Duration::from_secs(60 * 60);
/// Most seen ids kept (`work-seen.json`); the oldest are forgotten first.
pub const MAX_SEEN: usize = 2000;
/// Most announced ids kept in memory.
pub const MAX_KNOWN: usize = 5000;
/// Longest error detail shown in a source status.
const MAX_DETAIL_CHARS: usize = 200;

/// Slot of a source in fixed-size arrays (the order of [`SOURCES`]).
pub fn source_index(source: WorkSource) -> usize {
    match source {
        WorkSource::Github => 0,
        WorkSource::Gitlab => 1,
        WorkSource::Jira => 2,
        WorkSource::AzureDevops => 3,
        WorkSource::Linear => 4,
    }
}

/// Name of the tool for logs and error messages.
pub fn source_name(source: WorkSource) -> &'static str {
    match source {
        WorkSource::Github => "GitHub",
        WorkSource::Gitlab => "GitLab",
        WorkSource::Jira => "Jira",
        WorkSource::AzureDevops => "Azure DevOps",
        WorkSource::Linear => "Linear",
    }
}

/// What one source is configured with (`WorkConfig`, already normalized). A change resets the source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceSettings {
    pub enabled: bool,
    /// GitLab instance or Jira site (`https://…`, no trailing slash); empty for the other sources.
    pub base_url: String,
    /// Jira Cloud account e-mail (empty = Data Center personal access token).
    pub email: String,
    /// Azure DevOps organisation.
    pub org: String,
}

pub fn settings(config: &WorkConfig, source: WorkSource) -> SourceSettings {
    match source {
        WorkSource::Github => SourceSettings {
            enabled: config.github,
            ..SourceSettings::default()
        },
        WorkSource::Gitlab => SourceSettings {
            enabled: config.gitlab,
            base_url: config.gitlab_url.clone(),
            ..SourceSettings::default()
        },
        WorkSource::Jira => SourceSettings {
            enabled: config.jira,
            base_url: config.jira_url.clone(),
            email: config.jira_email.clone(),
            ..SourceSettings::default()
        },
        WorkSource::AzureDevops => SourceSettings {
            enabled: config.azure_devops,
            org: config.azure_org.clone(),
            ..SourceSettings::default()
        },
        WorkSource::Linear => SourceSettings {
            enabled: config.linear,
            ..SourceSettings::default()
        },
    }
}

/// English hint of what the user still has to set up, `None` when the source can be read.
pub type Missing = Option<&'static str>;

pub fn missing(
    source: WorkSource,
    settings: &SourceSettings,
    has_token: bool,
    secrets_supported: bool,
) -> Missing {
    if !secrets_supported {
        return Some("encrypted token storage is not available on this system");
    }
    let address = match source {
        WorkSource::Gitlab if settings.base_url.is_empty() => Some("enter the GitLab address"),
        WorkSource::Jira if settings.base_url.is_empty() => Some("enter the Jira site address"),
        WorkSource::AzureDevops if settings.org.is_empty() => {
            Some("enter the Azure DevOps organisation")
        }
        _ => None,
    };
    address.or_else(|| (!has_token).then_some("add an access token"))
}

/// What a source last reported, for its status line.
#[derive(Debug, Clone, Copy, Default)]
pub struct Reported<'a> {
    /// Redacted error of the last fetch.
    pub error: Option<&'a str>,
    /// Redacted warning of the last successful fetch (an optional part failed).
    pub warning: Option<&'a str>,
    /// A fetch succeeded since the source was (re)configured.
    pub synced: bool,
}

/// The status line of one source.
pub fn source_status(
    source: WorkSource,
    enabled: bool,
    missing: Missing,
    reported: Reported<'_>,
    account: Option<String>,
) -> WorkSourceStatus {
    let (state, detail) = if !enabled {
        (IntegrationState::Off, None)
    } else if let Some(missing) = missing {
        (IntegrationState::Unavailable, Some(missing.to_owned()))
    } else if let Some(error) = reported.error {
        (IntegrationState::Error, Some(error.to_owned()))
    } else if reported.synced {
        (IntegrationState::On, reported.warning.map(str::to_owned))
    } else {
        (IntegrationState::On, Some("not synced yet".to_owned()))
    };
    WorkSourceStatus {
        source,
        state,
        detail,
        account: if enabled { account } else { None },
    }
}

/// Wait after `failures` failed fetches in a row (1 → [`FIRST_BACKOFF`], doubling, at most [`MAX_BACKOFF`]).
pub fn backoff(failures: u32) -> Duration {
    let doublings = failures.saturating_sub(1).min(16);
    FIRST_BACKOFF
        .saturating_mul(1_u32 << doublings)
        .min(MAX_BACKOFF)
}

/// Shortest gap between two fetches of a source that sent a poll hint (GitHub `X-Poll-Interval`, seconds).
pub fn poll_gap(hint_secs: Option<u32>) -> Duration {
    hint_secs.map_or(Duration::ZERO, |secs| {
        Duration::from_secs(u64::from(secs)).min(MAX_POLL_HINT)
    })
}

/// Wait until the next regular fetch after a successful one.
pub fn next_poll(gap: Duration) -> Duration {
    POLL_EVERY.max(gap)
}

/// When a source is fetched: the regular interval, the tool's poll hint and the backoff after failures.
#[derive(Debug, Clone)]
pub struct Poll {
    next_due: Instant,
    /// Shortest gap between two fetch starts ([`poll_gap`]).
    gap: Duration,
    last_started: Option<Instant>,
    /// Failed fetches in a row.
    failures: u32,
}

impl Poll {
    /// Due at once.
    pub fn new(now: Instant) -> Self {
        Self {
            next_due: now,
            gap: Duration::ZERO,
            last_started: None,
            failures: 0,
        }
    }

    pub fn next_due(&self) -> Instant {
        self.next_due
    }

    /// A fetch may start at `now`. `forced` (refresh, token or settings change, session start) skips the
    /// regular interval and the backoff, but never the tool's poll hint: a forced fetch it holds back is due
    /// at the earliest allowed moment instead.
    pub fn may_start(&mut self, now: Instant, forced: bool) -> bool {
        if !forced {
            return now >= self.next_due;
        }
        let earliest = self.last_started.map_or(now, |started| started + self.gap);
        if now < earliest {
            self.next_due = self.next_due.min(earliest);
            return false;
        }
        true
    }

    /// A fetch started; if its result never arrives, the next one is due after the regular interval.
    pub fn started(&mut self, now: Instant) {
        self.last_started = Some(now);
        self.next_due = now + POLL_EVERY;
    }

    /// The fetch could not start (no thread): try again soon.
    pub fn not_started(&mut self, now: Instant) {
        self.next_due = now + FIRST_BACKOFF;
    }

    pub fn succeeded(&mut self, now: Instant, hint_secs: Option<u32>) {
        self.failures = 0;
        self.gap = poll_gap(hint_secs);
        self.next_due = now + next_poll(self.gap);
    }

    /// Schedules the retry with [`backoff`]; returns the failures in a row.
    pub fn failed(&mut self, now: Instant) -> u32 {
        self.failures = self.failures.saturating_add(1);
        self.next_due = now + backoff(self.failures).max(self.gap);
        self.failures
    }
}

/// What a source's new list brought.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrivals {
    /// Items neither announced before nor seen.
    pub new: usize,
    /// How many to announce now (a `WorkDigest`); 0 when nothing is new, it is a baseline or they are held.
    pub announce: u32,
}

/// Which new items are announced now and which wait for the end of focus (`holdDuringFocus`).
#[derive(Debug, Clone)]
pub struct Announcer {
    /// Ids already announced or part of a baseline (this run only).
    known: IdSet,
    held: Held,
    focus: bool,
    hold: bool,
}

impl Announcer {
    pub fn new(hold: bool) -> Self {
        Self {
            known: IdSet::new(MAX_KNOWN),
            held: Held::default(),
            focus: false,
            hold,
        }
    }

    /// Focus runs and `holdDuringFocus` is on: new items wait for the break.
    pub fn holding(&self) -> bool {
        self.focus && self.hold
    }

    /// A source's fresh list. Its `baseline` (the first list since the start or a reconfiguration of the
    /// source) only records the items; later lists announce what is new, or hold it during focus.
    pub fn list(&mut self, items: &[WorkItem], baseline: bool, seen: &IdSet) -> Arrivals {
        let new = new_ids(items, &self.known, seen);
        for id in &new {
            self.known.touch(id);
        }
        let count = new.len();
        let announce = if baseline || new.is_empty() {
            0
        } else if self.holding() {
            self.held.add(new);
            0
        } else {
            u32::try_from(count).unwrap_or(u32::MAX)
        };
        Arrivals {
            new: count,
            announce,
        }
    }

    /// The session phase; returns the digest to show now (the held items when focus ended, else 0).
    pub fn phase(&mut self, phase: Phase, seen: &IdSet) -> u32 {
        self.focus = phase == Phase::Focus;
        self.release(seen)
    }

    /// `holdDuringFocus` changed; turned off during focus, the held items are announced now.
    pub fn set_hold(&mut self, hold: bool, seen: &IdSet) -> u32 {
        self.hold = hold;
        self.release(seen)
    }

    /// Held items the user has not seen yet (`WorkStatus::held_count`).
    pub fn held_count(&self, seen: &IdSet) -> u32 {
        self.held.count(seen)
    }

    fn release(&mut self, seen: &IdSet) -> u32 {
        if self.holding() || self.held.is_empty() {
            return 0;
        }
        self.held.take(seen)
    }
}

/// An error detail safe to show and log: `secret` (the token, when known) and links are replaced, control
/// characters become spaces, at most [`MAX_DETAIL_CHARS`] characters.
pub fn redact(detail: &str, secret: Option<&str>) -> String {
    let mut text: String = detail
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if let Some(secret) = secret.map(str::trim).filter(|secret| secret.len() >= 4) {
        text = text.replace(secret, "<token>");
    }
    let text = text
        .split(' ')
        .map(|word| if word.contains("://") { "<link>" } else { word })
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.trim();
    if text.chars().count() <= MAX_DETAIL_CHARS {
        text.to_owned()
    } else {
        let mut short: String = text.chars().take(MAX_DETAIL_CHARS - 1).collect();
        short.push('…');
        short
    }
}

/// Ids in insertion order with a cap: the oldest go first. [`IdSet::touch`] makes an id the newest again.
#[derive(Debug, Clone)]
pub struct IdSet {
    order: VecDeque<String>,
    set: HashSet<String>,
    cap: usize,
}

impl IdSet {
    pub fn new(cap: usize) -> Self {
        Self {
            order: VecDeque::new(),
            set: HashSet::new(),
            cap: cap.max(1),
        }
    }

    /// From stored ids (oldest first); empty and duplicate ids are skipped.
    pub fn from_ids(ids: impl IntoIterator<Item = String>, cap: usize) -> Self {
        let mut set = Self::new(cap);
        for id in ids {
            set.touch(&id);
        }
        set
    }

    pub fn contains(&self, id: &str) -> bool {
        self.set.contains(id)
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Adds `id` (or makes it the newest); `true` when it was not there yet.
    pub fn touch(&mut self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        let added = if self.set.contains(id) {
            if self.order.back().is_some_and(|last| last == id) {
                return false;
            }
            self.order.retain(|kept| kept != id);
            false
        } else {
            self.set.insert(id.to_owned());
            true
        };
        self.order.push_back(id.to_owned());
        while self.order.len() > self.cap {
            if let Some(oldest) = self.order.pop_front() {
                self.set.remove(&oldest);
            }
        }
        added
    }

    /// Oldest first.
    pub fn ids(&self) -> Vec<String> {
        self.order.iter().cloned().collect()
    }
}

/// `work-seen.json`: ids of the items the user has seen in "Co dalej?" (oldest first; no titles).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoredSeen {
    pub seen: Vec<String>,
}

/// Distinct ids of `items` that were neither announced (`known`) nor seen.
pub fn new_ids(items: &[WorkItem], known: &IdSet, seen: &IdSet) -> Vec<String> {
    let mut found = HashSet::new();
    items
        .iter()
        .filter(|item| !known.contains(&item.id) && !seen.contains(&item.id))
        .filter(|item| found.insert(item.id.as_str()))
        .map(|item| item.id.clone())
        .collect()
}

/// New items held back during focus (`holdDuringFocus`), shown as one digest at the break / after the
/// session. Items the user has seen meanwhile (e.g. opened the list during focus) no longer count.
#[derive(Debug, Clone, Default)]
pub struct Held {
    ids: BTreeSet<String>,
}

impl Held {
    pub fn add(&mut self, ids: impl IntoIterator<Item = String>) {
        self.ids.extend(ids);
    }

    /// Held items the user has not seen yet (`WorkStatus::held_count`).
    pub fn count(&self, seen: &IdSet) -> u32 {
        let count = self.ids.iter().filter(|id| !seen.contains(id)).count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// The count of the digest to show now; empties the hold.
    pub fn take(&mut self, seen: &IdSet) -> u32 {
        let count = self.count(seen);
        self.ids.clear();
        count
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::api::WorkItemKind;

    fn item(id: &str) -> WorkItem {
        WorkItem {
            id: id.into(),
            source: WorkSource::Github,
            kind: WorkItemKind::Assigned,
            key: id.into(),
            title: format!("Title {id}"),
            url: String::new(),
            project: None,
            priority_label: None,
            priority_rank: None,
            due_ms: None,
            status: None,
            in_sprint: false,
            updated_ms: 0,
            score: 0,
            reasons: Vec::new(),
            unseen: false,
        }
    }

    #[test]
    fn sources_have_fixed_slots_and_settings() {
        let slots: Vec<usize> = SOURCES.iter().map(|&s| source_index(s)).collect();
        assert_eq!(slots, [0, 1, 2, 3, 4]);
        let config = WorkConfig {
            gitlab: true,
            jira: true,
            jira_url: "https://firma.atlassian.net".into(),
            jira_email: "ja@firma.pl".into(),
            azure_org: "firma".into(),
            ..WorkConfig::default()
        };
        assert_eq!(
            settings(&config, WorkSource::Gitlab),
            SourceSettings {
                enabled: true,
                base_url: "https://gitlab.com".into(),
                ..SourceSettings::default()
            }
        );
        let jira = settings(&config, WorkSource::Jira);
        assert_eq!(
            (jira.enabled, jira.base_url.as_str(), jira.email.as_str()),
            (true, "https://firma.atlassian.net", "ja@firma.pl")
        );
        let azure = settings(&config, WorkSource::AzureDevops);
        assert_eq!((azure.enabled, azure.org.as_str()), (false, "firma"));
        // The hold setting resets no source.
        let held = WorkConfig {
            hold_during_focus: false,
            ..config.clone()
        };
        for source in SOURCES {
            assert_eq!(settings(&held, source), settings(&config, source));
        }
    }

    #[test]
    fn missing_names_the_first_thing_to_set_up() {
        let empty = SourceSettings {
            enabled: true,
            ..SourceSettings::default()
        };
        assert_eq!(
            missing(WorkSource::Jira, &empty, false, true),
            Some("enter the Jira site address")
        );
        assert_eq!(
            missing(WorkSource::AzureDevops, &empty, true, true),
            Some("enter the Azure DevOps organisation")
        );
        assert_eq!(
            missing(WorkSource::Gitlab, &empty, true, true),
            Some("enter the GitLab address")
        );
        assert_eq!(
            missing(WorkSource::Github, &empty, false, true),
            Some("add an access token")
        );
        assert_eq!(missing(WorkSource::Linear, &empty, true, true), None);
        assert!(
            missing(WorkSource::Linear, &empty, true, false)
                .is_some_and(|hint| hint.contains("encrypted"))
        );
    }

    #[test]
    fn statuses_follow_the_source_state() {
        let synced = Reported {
            synced: true,
            ..Reported::default()
        };
        let off = source_status(WorkSource::Github, false, None, synced, Some("me".into()));
        assert_eq!(
            (off.state, off.detail, off.account),
            (IntegrationState::Off, None, None)
        );
        let failed_before = Reported {
            error: Some("x"),
            ..Reported::default()
        };
        let missing = source_status(
            WorkSource::Jira,
            true,
            Some("add an access token"),
            failed_before,
            None,
        );
        assert_eq!(missing.state, IntegrationState::Unavailable);
        let failed = source_status(
            WorkSource::Jira,
            true,
            None,
            Reported {
                error: Some("HTTP 401"),
                warning: Some("ignored"),
                synced: true,
            },
            None,
        );
        assert_eq!(
            (failed.state, failed.detail.as_deref()),
            (IntegrationState::Error, Some("HTTP 401"))
        );
        let waiting = source_status(WorkSource::Linear, true, None, Reported::default(), None);
        assert_eq!(
            (waiting.state, waiting.detail.as_deref()),
            (IntegrationState::On, Some("not synced yet"))
        );
        let on = source_status(WorkSource::Linear, true, None, synced, Some("Ola".into()));
        assert_eq!(
            (on.state, on.detail, on.account.as_deref()),
            (IntegrationState::On, None, Some("Ola"))
        );
        let partial = source_status(
            WorkSource::Github,
            true,
            None,
            Reported {
                warning: Some("notifications need a classic token"),
                ..synced
            },
            None,
        );
        assert_eq!(
            (partial.state, partial.detail.as_deref()),
            (
                IntegrationState::On,
                Some("notifications need a classic token")
            )
        );
    }

    #[test]
    fn backoff_doubles_up_to_thirty_minutes() {
        let minutes: Vec<u64> = (1..=8).map(|n| backoff(n).as_secs() / 60).collect();
        assert_eq!(minutes, [1, 2, 4, 8, 16, 30, 30, 30]);
        assert_eq!(backoff(0), FIRST_BACKOFF);
        assert_eq!(backoff(u32::MAX), MAX_BACKOFF);
    }

    #[test]
    fn poll_hints_never_shorten_the_interval() {
        assert_eq!(poll_gap(None), Duration::ZERO);
        assert_eq!(poll_gap(Some(60)), Duration::from_secs(60));
        assert_eq!(poll_gap(Some(u32::MAX)), MAX_POLL_HINT);
        assert_eq!(next_poll(poll_gap(Some(60))), POLL_EVERY);
        assert_eq!(
            next_poll(poll_gap(Some(900))),
            Duration::from_secs(900),
            "a longer hint (server load) is respected"
        );
    }

    #[test]
    fn redact_hides_links_and_the_token() {
        let detail =
            "HTTP 401 for https://api.github.com/notifications?x=1 with ghp_SECRET123\nretry";
        let redacted = redact(detail, Some("ghp_SECRET123"));
        assert_eq!(redacted, "HTTP 401 for <link> with <token> retry");
        assert_eq!(redact("short", Some("abc")), "short");
        let long = redact(&"x ".repeat(300), None);
        assert!(long.chars().count() <= MAX_DETAIL_CHARS);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn id_sets_keep_the_newest_ids() {
        let mut set = IdSet::new(3);
        assert!(set.touch("a"));
        assert!(set.touch("b"));
        assert!(!set.touch("a"), "already there");
        assert!(set.touch("c"));
        assert!(set.touch("d"));
        // "b" was the oldest after "a" was touched again.
        assert_eq!(set.ids(), ["a", "c", "d"]);
        assert!(!set.contains("b"));
        assert!(!set.touch(""));
        let stored = IdSet::from_ids(["x".into(), "y".into(), "x".into(), String::new()], 10);
        assert_eq!(stored.ids(), ["y", "x"]);
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn new_ids_skip_announced_seen_and_duplicates() {
        let mut known = IdSet::new(10);
        known.touch("github:a");
        let mut seen = IdSet::new(10);
        seen.touch("github:b");
        let items = [
            item("github:a"),
            item("github:b"),
            item("github:c"),
            item("github:c"),
            item("github:d"),
        ];
        assert_eq!(new_ids(&items, &known, &seen), ["github:c", "github:d"]);
    }

    #[test]
    fn held_items_count_only_while_unseen() {
        let mut held = Held::default();
        assert!(held.is_empty());
        held.add(["a".to_owned(), "b".to_owned(), "c".to_owned()]);
        held.add(["a".to_owned()]);
        let mut seen = IdSet::new(10);
        assert_eq!(held.count(&seen), 3);
        seen.touch("b");
        assert_eq!(held.count(&seen), 2);
        assert_eq!(held.take(&seen), 2);
        assert!(held.is_empty());
        assert_eq!(held.take(&seen), 0);
    }

    #[test]
    fn polling_follows_interval_backoff_and_hint() {
        let t0 = Instant::now();
        let mut poll = Poll::new(t0);
        assert!(poll.may_start(t0, false), "due at once");
        poll.started(t0);
        poll.succeeded(t0, Some(60));
        assert_eq!(poll.next_due(), t0 + POLL_EVERY);
        assert!(!poll.may_start(t0 + Duration::from_secs(299), false));
        assert!(poll.may_start(t0 + POLL_EVERY, false));
        // A refresh within the poll hint waits for it…
        let soon = t0 + Duration::from_secs(30);
        assert!(!poll.may_start(soon, true));
        assert_eq!(poll.next_due(), t0 + Duration::from_secs(60));
        // …and goes through after it.
        assert!(poll.may_start(t0 + Duration::from_secs(60), true));

        // Failures back off 1, 2, 4 … minutes (never below the hint) and a forced fetch skips the backoff.
        let t1 = t0 + Duration::from_secs(600);
        poll.started(t1);
        assert_eq!(poll.failed(t1), 1);
        assert_eq!(poll.next_due(), t1 + Duration::from_secs(60));
        assert_eq!(poll.failed(t1), 2);
        assert_eq!(poll.next_due(), t1 + Duration::from_secs(120));
        assert!(!poll.may_start(t1 + Duration::from_secs(119), false));
        assert!(poll.may_start(t1 + Duration::from_secs(61), true));
        for _ in 0..10 {
            poll.failed(t1);
        }
        assert_eq!(poll.next_due(), t1 + MAX_BACKOFF);
        poll.succeeded(t1, None);
        assert_eq!(poll.next_due(), t1 + POLL_EVERY);
        // Without a hint a refresh may start at once.
        assert!(poll.may_start(t1 + Duration::from_secs(1), true));
        poll.not_started(t1);
        assert_eq!(poll.next_due(), t1 + FIRST_BACKOFF);
    }

    fn items(ids: &[&str]) -> Vec<WorkItem> {
        ids.iter().map(|id| item(id)).collect()
    }

    #[test]
    fn new_items_are_announced_outside_focus() {
        let seen = IdSet::new(10);
        let mut announcer = Announcer::new(true);
        // The first list of a source is the baseline.
        let first = announcer.list(&items(&["a", "b"]), true, &seen);
        assert_eq!(
            first,
            Arrivals {
                new: 2,
                announce: 0
            }
        );
        // Later lists announce only what is new.
        let later = announcer.list(&items(&["a", "b", "c"]), false, &seen);
        assert_eq!(
            later,
            Arrivals {
                new: 1,
                announce: 1
            }
        );
        let again = announcer.list(&items(&["a", "b", "c"]), false, &seen);
        assert_eq!(
            again,
            Arrivals {
                new: 0,
                announce: 0
            }
        );
        // In a break as well.
        assert_eq!(announcer.phase(Phase::Break, &seen), 0);
        assert_eq!(
            announcer.list(&items(&["d"]), false, &seen),
            Arrivals {
                new: 1,
                announce: 1
            }
        );
    }

    #[test]
    fn focus_holds_new_items_until_the_break() {
        let mut seen = IdSet::new(10);
        let mut announcer = Announcer::new(true);
        announcer.list(&items(&["a"]), true, &seen);
        assert_eq!(announcer.phase(Phase::Focus, &seen), 0);
        assert_eq!(
            announcer.list(&items(&["a", "b", "c"]), false, &seen),
            Arrivals {
                new: 2,
                announce: 0
            }
        );
        assert_eq!(
            announcer.list(&items(&["a", "b", "c", "d"]), false, &seen),
            Arrivals {
                new: 1,
                announce: 0
            }
        );
        assert_eq!(announcer.held_count(&seen), 3);
        // The user looked at the list during focus: "b" is no longer news.
        seen.touch("b");
        assert_eq!(announcer.held_count(&seen), 2);
        // A paused focus is still focus; the break releases the digest once.
        assert_eq!(announcer.phase(Phase::Focus, &seen), 0);
        assert_eq!(announcer.phase(Phase::Break, &seen), 2);
        assert_eq!(announcer.held_count(&seen), 0);
        assert_eq!(announcer.phase(Phase::Idle, &seen), 0);

        // Stopping the session releases them too.
        announcer.phase(Phase::Focus, &seen);
        announcer.list(&items(&["e"]), false, &seen);
        assert_eq!(announcer.phase(Phase::Idle, &seen), 1);
    }

    #[test]
    fn turning_holding_off_releases_and_stops_holding() {
        let seen = IdSet::new(10);
        let mut announcer = Announcer::new(true);
        announcer.phase(Phase::Focus, &seen);
        announcer.list(&items(&["a"]), true, &seen);
        announcer.list(&items(&["a", "b"]), false, &seen);
        assert_eq!(announcer.set_hold(true, &seen), 0);
        assert_eq!(announcer.set_hold(false, &seen), 1);
        // Without holding, focus announces at once.
        assert_eq!(
            announcer.list(&items(&["a", "b", "c"]), false, &seen),
            Arrivals {
                new: 1,
                announce: 1
            }
        );
        assert_eq!(announcer.phase(Phase::Break, &seen), 0);
    }

    #[test]
    fn stored_seen_reads_old_and_partial_files() {
        let stored: StoredSeen = serde_json::from_str("{}").expect("empty object");
        assert!(stored.seen.is_empty());
        let stored: StoredSeen = serde_json::from_str(r#"{"seen":["jira:PROJ-1"]}"#).expect("ids");
        assert_eq!(stored.seen, ["jira:PROJ-1"]);
    }
}
