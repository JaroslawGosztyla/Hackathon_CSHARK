//! "Co dalej?": ranking of the work items fetched from the developer tools (pure).
//!
//! The shell fetches issues, pull / merge requests and notifications (GitHub, GitLab, Jira, Azure DevOps,
//! Linear) with read-only tokens and passes them to [`rank`] with the wall clock. Nothing here reads a clock,
//! a file or the network, and nothing is sent anywhere.
//!
//! # Tiers (owner's spec)
//!
//! Every item of a tier ranks above every item of the next one, whatever its other attributes:
//!
//! 1. **Assigned** to the user ([`WorkItemKind::Assigned`]: issues, tickets, work items).
//! 2. **Waiting on the user**: review requested, then changes requested / failing CI on the user's own pull /
//!    merge request, then direct mentions.
//! 3. **Other notifications** (informational).
//!
//! Inside a tier the order is: active sprint / cycle first, then priority (highest → lowest; unknown counts as
//! medium), then due date (overdue, due within 24 h, due within 3 days, later or none), then status ("in
//! progress" before "to do"), then age: older first in tiers 1 and 2 (it has waited longer), newer first in
//! tier 3 (fresh information). Remaining ties are broken by `key`, then `id`.
//!
//! # Score
//!
//! [`WorkItem::score`] encodes that order as decimal digits (higher first), so it can be read at a glance:
//!
//! | digits | meaning |
//! |---|---|
//! | `10^9` | tier: 3 = assigned, 2 = waiting on the user, 1 = notification |
//! | `10^8` | tier 2 only: 3 = review requested, 2 = changes requested / CI failed, 1 = mention |
//! | `10^7` | 1 = in the active sprint / cycle / iteration |
//! | `10^6` | priority: 4 = highest (rank 0) … 0 = lowest (rank 4); unknown = 2 |
//! | `10^5` | due: 3 = overdue, 2 = due within 24 h, 1 = due within 3 days, 0 = later / none |
//! | `10^4` | 1 = status "in progress" ([`is_in_progress`]) |
//! | `10^0..10^3` | hours since the last update, at most 9 999 (tier 3: 9 999 minus that) |
//!
//! # Due dates
//!
//! `due_ms` is the moment the item is due. For a date-only due date (Jira `duedate`, GitLab `due_date`,
//! Linear `dueDate`) the shell passes the **end of that day in local time** (the last millisecond before the
//! next local midnight): the item is then "due today" all day and "overdue" only once the day has passed.

use std::cmp::Reverse;
use std::collections::HashMap;

use crate::api::{SessionGoal, WorkItem, WorkItemKind, WorkReason};

/// [`rank`] returns at most this many items.
pub const MAX_ITEMS: usize = 50;
/// A tier-2 item not updated for longer than this gets [`WorkReason::WaitingLong`].
pub const WAITING_LONG_MS: i64 = 3 * MS_PER_DAY;
/// Due within this time (but not within 24 h) still ranks above "later / none".
pub const DUE_SOON_MS: i64 = 3 * MS_PER_DAY;
/// `priority_rank` at or below this gets [`WorkReason::HighPriority`] (0 = highest, 1 = high).
pub const HIGH_PRIORITY_MAX_RANK: u32 = 1;
/// Lowest normalised priority; larger ranks are clamped to it.
pub const LOWEST_PRIORITY_RANK: u32 = 4;
/// Rank used for ordering when the tool has no priority (medium).
pub const UNKNOWN_PRIORITY_RANK: u32 = 2;
/// Longest title kept (characters); a longer one is shortened with `…`.
pub const MAX_TITLE_CHARS: usize = 300;
/// Longest key kept (characters).
pub const MAX_KEY_CHARS: usize = 120;
/// Longest project, priority label or status kept (characters).
pub const MAX_LABEL_CHARS: usize = 80;
/// Longest URL kept; a longer one is dropped (empty).
pub const MAX_URL_LEN: usize = 2048;

const MS_PER_HOUR: i64 = 60 * 60 * 1000;
const MS_PER_DAY: i64 = 24 * MS_PER_HOUR;
const MAX_AGE_HOURS: u32 = 9_999;

const TIER_WEIGHT: u32 = 1_000_000_000;
const GROUP_WEIGHT: u32 = 100_000_000;
const SPRINT_WEIGHT: u32 = 10_000_000;
const PRIORITY_WEIGHT: u32 = 1_000_000;
const DUE_WEIGHT: u32 = 100_000;
const STATUS_WEIGHT: u32 = 10_000;

/// Status names (letters and digits only, lowercase) that mean "in progress" when they appear anywhere in
/// the status, e.g. "In Progress", "workflow::in progress" (GitLab scoped label), "Dev in progress".
const IN_PROGRESS_PARTS: [&str; 15] = [
    "inprogress",
    "inreview",
    "codereview",
    "indevelopment",
    "doing",
    "wtoku",
    "wtrakcie",
    "wrealizacji",
    "inarbeit",
    "inbearbeitung",
    "encours",
    "encurso",
    "enprogreso",
    "incorso",
    "vrabote",
];
/// Status names (letters and digits only, lowercase) that mean "in progress" only as the whole status
/// ("Active" in Azure DevOps Agile / CMMI, "started" as Linear's state type, Jira's status category
/// "indeterminate"), so that e.g. "Inactive" does not match.
const IN_PROGRESS_NAMES: [&str; 9] = [
    "active",
    "started",
    "review",
    "wip",
    "development",
    "implementing",
    "implementation",
    "indeterminate",
    "workinprogress",
];

/// Ranking tier of a kind: 1 = assigned to the user, 2 = waiting on the user, 3 = other notification.
pub fn tier(kind: WorkItemKind) -> u32 {
    match kind {
        WorkItemKind::Assigned => 1,
        WorkItemKind::ReviewRequested
        | WorkItemKind::ChangesRequested
        | WorkItemKind::CiFailed
        | WorkItemKind::Mention => 2,
        WorkItemKind::Notification => 3,
    }
}

/// Order inside tier 2: 3 = review requested (others wait), 2 = changes requested / CI failed on the user's
/// own pull / merge request, 1 = mention; 0 for the other tiers.
fn group(kind: WorkItemKind) -> u32 {
    match kind {
        WorkItemKind::ReviewRequested => 3,
        WorkItemKind::ChangesRequested | WorkItemKind::CiFailed => 2,
        WorkItemKind::Mention => 1,
        WorkItemKind::Assigned | WorkItemKind::Notification => 0,
    }
}

/// Dedupes, cleans, scores, explains and sorts the work items (best first, at most [`MAX_ITEMS`]).
///
/// - Items with the same `id` become one: the copy of the best tier wins (then the stronger tier-2 kind, then
///   the copy with more fields set, then the newer one) and its missing fields are filled from the others
///   (`in_sprint` and `unseen` if any copy has them, `updated_ms` is the latest).
/// - `score` and `reasons` are computed here (incoming values are ignored); see the module docs.
/// - Text is cleaned for display: control characters become spaces, text is trimmed and shortened
///   ([`MAX_TITLE_CHARS`], [`MAX_KEY_CHARS`], [`MAX_LABEL_CHARS`]; empty labels become `None`), a URL that is
///   not `http(s)://` becomes empty and `priority_rank` is clamped to [`LOWEST_PRIORITY_RANK`].
/// - Ties are ordered by `key`, then `id`, so the result does not depend on the input order.
///
/// All arithmetic saturates; nothing panics for any `now_ms` or item times.
pub fn rank(items: Vec<WorkItem>, now_ms: i64) -> Vec<WorkItem> {
    let mut by_id: HashMap<String, usize> = HashMap::with_capacity(items.len());
    let mut copies: Vec<Vec<WorkItem>> = Vec::with_capacity(items.len());
    for item in items {
        let item = cleaned(item);
        match by_id.get(&item.id).and_then(|&index| copies.get_mut(index)) {
            Some(same_id) => same_id.push(item),
            None => {
                by_id.insert(item.id.clone(), copies.len());
                copies.push(vec![item]);
            }
        }
    }
    let mut unique: Vec<WorkItem> = copies.into_iter().filter_map(merged).collect();
    for item in &mut unique {
        let (score, reasons) = score(item, now_ms);
        item.score = score;
        item.reasons = reasons;
    }
    unique.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.key.cmp(&b.key))
            .then_with(|| a.id.cmp(&b.id))
    });
    unique.truncate(MAX_ITEMS);
    unique
}

/// `true` when a status as the tool names it means "being worked on": "In Progress", "Doing", "In Review",
/// "Code review", "Active", "WIP", GitLab's "workflow::in progress", Linear's "started" and the Polish,
/// German, French, Spanish, Italian and Russian names of "in progress" (case, spaces and punctuation ignored).
pub fn is_in_progress(status: &str) -> bool {
    let name: String = status
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .map(transliterate_cyrillic)
        .collect();
    !name.is_empty()
        && (IN_PROGRESS_PARTS.iter().any(|part| name.contains(part))
            || IN_PROGRESS_NAMES.contains(&name.as_str()))
}

/// The goal as kept in the session: key and title cleaned like work items ([`rank`]), a URL that is not
/// `http(s)://` emptied. `None` when both key and title are empty.
pub fn normalize_goal(goal: SessionGoal) -> Option<SessionGoal> {
    let key = clean_text(&goal.key, MAX_KEY_CHARS);
    let title = clean_text(&goal.title, MAX_TITLE_CHARS);
    if key.is_empty() && title.is_empty() {
        return None;
    }
    Some(SessionGoal {
        source: goal.source,
        key,
        title,
        url: safe_url(&goal.url),
    })
}

/// Score and reasons of one (cleaned) item at `now_ms`; see the module docs for the layout.
fn score(item: &WorkItem, now_ms: i64) -> (u32, Vec<WorkReason>) {
    let tier = tier(item.kind);
    let group = group(item.kind);
    let mut reasons = Vec::new();
    match item.kind {
        WorkItemKind::Assigned => reasons.push(WorkReason::Assigned),
        WorkItemKind::ReviewRequested => reasons.push(WorkReason::BlockingOthers),
        WorkItemKind::Mention => reasons.push(WorkReason::Mentioned),
        WorkItemKind::ChangesRequested | WorkItemKind::CiFailed | WorkItemKind::Notification => {}
    }

    let sprint = u32::from(item.in_sprint);
    if item.in_sprint {
        reasons.push(WorkReason::InSprint);
    }

    let rank = item
        .priority_rank
        .unwrap_or(UNKNOWN_PRIORITY_RANK)
        .min(LOWEST_PRIORITY_RANK);
    if item
        .priority_rank
        .is_some_and(|rank| rank <= HIGH_PRIORITY_MAX_RANK)
    {
        reasons.push(WorkReason::HighPriority);
    }
    let priority = LOWEST_PRIORITY_RANK - rank;

    let due = match item.due_ms {
        Some(due_ms) if due_ms < now_ms => {
            reasons.push(WorkReason::Overdue);
            3
        }
        Some(due_ms) if due_ms.saturating_sub(now_ms) < MS_PER_DAY => {
            reasons.push(WorkReason::DueToday);
            2
        }
        Some(due_ms) if due_ms.saturating_sub(now_ms) < DUE_SOON_MS => 1,
        _ => 0,
    };

    let in_progress = item.status.as_deref().is_some_and(is_in_progress);
    if in_progress {
        reasons.push(WorkReason::InProgress);
    }
    let status = u32::from(in_progress);

    let waited_ms = now_ms.saturating_sub(item.updated_ms).max(0);
    if tier == 2 && waited_ms > WAITING_LONG_MS {
        reasons.push(WorkReason::WaitingLong);
    }
    let hours = u32::try_from(waited_ms / MS_PER_HOUR)
        .unwrap_or(MAX_AGE_HOURS)
        .min(MAX_AGE_HOURS);
    let age = if tier == 3 {
        MAX_AGE_HOURS - hours
    } else {
        hours
    };

    let score = (4 - tier) * TIER_WEIGHT
        + group * GROUP_WEIGHT
        + sprint * SPRINT_WEIGHT
        + priority * PRIORITY_WEIGHT
        + due * DUE_WEIGHT
        + status * STATUS_WEIGHT
        + age;
    (score, reasons)
}

/// One item from all copies with the same id: the preferred copy ([`preference`]) is the base and the
/// others, from the most to the least preferred, fill its missing fields. Independent of the input order.
fn merged(mut copies: Vec<WorkItem>) -> Option<WorkItem> {
    copies.sort_by(|a, b| preference(b).cmp(&preference(a)));
    let mut copies = copies.into_iter();
    let mut kept = copies.next()?;
    for other in copies {
        fill(&mut kept, other);
    }
    Some(kept)
}

/// Total order of the copies of one item (greater = preferred): best tier, then the stronger tier-2 kind,
/// then more fields set, then the newer one; the remaining fields only make the order total.
fn preference(item: &WorkItem) -> impl Ord + '_ {
    (
        (
            Reverse(tier(item.kind)),
            group(item.kind),
            richness(item),
            item.updated_ms,
        ),
        (
            &item.title,
            &item.url,
            &item.key,
            &item.status,
            &item.project,
            &item.priority_label,
        ),
        (
            item.priority_rank,
            item.due_ms,
            item.in_sprint,
            item.unseen,
            item.kind as u8,
            item.source as u8,
        ),
    )
}

/// Fills the fields `kept` lacks from `other` (same id).
fn fill(kept: &mut WorkItem, other: WorkItem) {
    kept.project = kept.project.take().or(other.project);
    kept.priority_rank = kept.priority_rank.or(other.priority_rank);
    kept.priority_label = kept.priority_label.take().or(other.priority_label);
    kept.due_ms = kept.due_ms.or(other.due_ms);
    kept.status = kept.status.take().or(other.status);
    if kept.title.is_empty() {
        kept.title = other.title;
    }
    if kept.url.is_empty() {
        kept.url = other.url;
    }
    if kept.key.is_empty() {
        kept.key = other.key;
    }
    kept.in_sprint |= other.in_sprint;
    kept.unseen |= other.unseen;
    kept.updated_ms = kept.updated_ms.max(other.updated_ms);
}

/// How many optional facts the item carries (dedupe prefers the richer copy).
fn richness(item: &WorkItem) -> usize {
    [
        item.project.is_some(),
        item.priority_label.is_some(),
        item.priority_rank.is_some(),
        item.due_ms.is_some(),
        item.status.is_some(),
        item.in_sprint,
        !item.title.is_empty(),
        !item.url.is_empty(),
    ]
    .into_iter()
    .filter(|set| *set)
    .count()
}

/// The item as displayed: see [`rank`].
fn cleaned(item: WorkItem) -> WorkItem {
    WorkItem {
        key: clean_text(&item.key, MAX_KEY_CHARS),
        title: clean_text(&item.title, MAX_TITLE_CHARS),
        url: safe_url(&item.url),
        project: clean_label(item.project),
        priority_label: clean_label(item.priority_label),
        priority_rank: item
            .priority_rank
            .map(|rank| rank.min(LOWEST_PRIORITY_RANK)),
        status: clean_label(item.status),
        ..item
    }
}

fn clean_label(label: Option<String>) -> Option<String> {
    label
        .map(|label| clean_text(&label, MAX_LABEL_CHARS))
        .filter(|label| !label.is_empty())
}

/// Control characters become spaces, trimmed, at most `max_chars` characters (a longer text keeps its first
/// `max_chars - 1` characters plus `…`).
fn clean_text(raw: &str, max_chars: usize) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }
    let mut short: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    short.truncate(short.trim_end().len());
    short.push('…');
    short
}

/// `url` (trimmed) when it is an `http(s)://` link with a host, without whitespace or control characters
/// and at most [`MAX_URL_LEN`] bytes long; otherwise empty (the UI then offers no link).
fn safe_url(url: &str) -> String {
    let url = url.trim();
    let rest = ["https://", "http://"].iter().find_map(|scheme| {
        url.get(..scheme.len())
            .filter(|prefix| prefix.eq_ignore_ascii_case(scheme))
            .and_then(|_| url.get(scheme.len()..))
    });
    let valid = url.len() <= MAX_URL_LEN
        && rest.is_some_and(|rest| !rest.is_empty() && !rest.starts_with(['/', '?', '#', '@']))
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    if valid { url.to_owned() } else { String::new() }
}

/// Russian "в работе" → "vrabote" (lowercase Cyrillic letters only; everything else unchanged).
fn transliterate_cyrillic(c: char) -> char {
    match c {
        'в' => 'v',
        'р' => 'r',
        'а' => 'a',
        'б' => 'b',
        'о' => 'o',
        'т' => 't',
        'е' => 'e',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::WorkSource;

    const NOW: i64 = 1_760_000_000_000;
    const HOUR: i64 = MS_PER_HOUR;
    const DAY: i64 = MS_PER_DAY;

    const ALL_KINDS: [WorkItemKind; 6] = [
        WorkItemKind::Assigned,
        WorkItemKind::ReviewRequested,
        WorkItemKind::ChangesRequested,
        WorkItemKind::CiFailed,
        WorkItemKind::Mention,
        WorkItemKind::Notification,
    ];

    /// A bare item updated just now: no sprint, priority, due date or status.
    fn item(key: &str, kind: WorkItemKind) -> WorkItem {
        WorkItem {
            id: format!("jira:{key}"),
            source: WorkSource::Jira,
            kind,
            key: key.to_owned(),
            title: format!("Title of {key}"),
            url: format!("https://firma.atlassian.net/browse/{key}"),
            project: None,
            priority_label: None,
            priority_rank: None,
            due_ms: None,
            status: None,
            in_sprint: false,
            updated_ms: NOW,
            score: 0,
            reasons: Vec::new(),
            unseen: false,
        }
    }

    fn assigned(key: &str) -> WorkItem {
        item(key, WorkItemKind::Assigned)
    }

    fn keys(items: &[WorkItem]) -> Vec<&str> {
        items.iter().map(|item| item.key.as_str()).collect()
    }

    fn ranked_keys(items: Vec<WorkItem>) -> Vec<String> {
        rank(items, NOW).into_iter().map(|item| item.key).collect()
    }

    fn only(item: WorkItem) -> WorkItem {
        let mut ranked = rank(vec![item], NOW);
        assert_eq!(ranked.len(), 1);
        ranked.remove(0)
    }

    #[test]
    fn kinds_map_to_the_owner_tiers() {
        assert_eq!(tier(WorkItemKind::Assigned), 1);
        for kind in [
            WorkItemKind::ReviewRequested,
            WorkItemKind::ChangesRequested,
            WorkItemKind::CiFailed,
            WorkItemKind::Mention,
        ] {
            assert_eq!(tier(kind), 2, "{kind:?}");
        }
        assert_eq!(tier(WorkItemKind::Notification), 3);
    }

    #[test]
    fn empty_input_ranks_to_nothing() {
        assert!(rank(Vec::new(), NOW).is_empty());
    }

    /// Every combination of attributes for every kind: tier 1 always above tier 2 above tier 3.
    #[test]
    fn every_tier_ranks_above_the_next_whatever_the_attributes() {
        let mut items = Vec::new();
        for (k, kind) in ALL_KINDS.into_iter().enumerate() {
            for in_sprint in [false, true] {
                for priority_rank in [None, Some(0), Some(1), Some(2), Some(3), Some(4), Some(99)] {
                    for due_ms in [
                        None,
                        Some(NOW - 30 * DAY),
                        Some(NOW + HOUR),
                        Some(NOW + 2 * DAY),
                        Some(NOW + 40 * DAY),
                    ] {
                        for status in [None, Some("In Progress"), Some("To Do")] {
                            for updated_ms in [NOW, NOW - 5 * DAY, NOW - 3_000 * DAY, NOW + DAY] {
                                let n = items.len();
                                items.push(WorkItem {
                                    id: format!("id-{n}"),
                                    key: format!("K{k}-{n}"),
                                    in_sprint,
                                    priority_rank,
                                    due_ms,
                                    status: status.map(str::to_owned),
                                    updated_ms,
                                    ..item("X", kind)
                                });
                            }
                        }
                    }
                }
            }
        }
        // Scores, without the 50-item cap.
        let scored: Vec<(u32, u32)> = items
            .iter()
            .map(|it| (tier(it.kind), score(&cleaned(it.clone()), NOW).0))
            .collect();
        for t in 1..=3 {
            let lowest = scored
                .iter()
                .filter(|(tier, _)| *tier == t)
                .map(|(_, s)| *s)
                .min()
                .expect("items of the tier");
            let highest_below = scored
                .iter()
                .filter(|(tier, _)| *tier > t)
                .map(|(_, s)| *s)
                .max();
            if let Some(highest_below) = highest_below {
                assert!(
                    lowest > highest_below,
                    "tier {t}: {lowest} <= {highest_below}"
                );
            }
            // The tier digit is the score's leading digit.
            assert!(
                scored
                    .iter()
                    .filter(|(tier, _)| *tier == t)
                    .all(|(_, s)| s / TIER_WEIGHT == 4 - t)
            );
        }

        // The ranked list (capped at 50) is ordered by tier and takes tier 1 first.
        let ranked = rank(items.clone(), NOW);
        assert_eq!(ranked.len(), MAX_ITEMS);
        let tiers: Vec<u32> = ranked.iter().map(|it| tier(it.kind)).collect();
        assert!(tiers.windows(2).all(|w| w[0] <= w[1]), "{tiers:?}");
        assert!(tiers.iter().all(|t| *t == 1));
        assert!(ranked.windows(2).all(|w| w[0].score >= w[1].score));

        // Without tier-1 items, the waiting ones come before notifications.
        let waiting_and_notes: Vec<WorkItem> = items
            .into_iter()
            .filter(|it| it.kind != WorkItemKind::Assigned)
            .filter(|it| it.kind == WorkItemKind::Notification || it.in_sprint)
            .filter(|it| it.kind == WorkItemKind::Notification || it.priority_rank == Some(4))
            .filter(|it| it.kind == WorkItemKind::Notification || it.due_ms.is_none())
            .filter(|it| it.kind == WorkItemKind::Notification || it.status.is_none())
            .filter(|it| it.kind == WorkItemKind::Notification || it.updated_ms == NOW)
            .collect();
        let ranked = rank(waiting_and_notes, NOW);
        let tiers: Vec<u32> = ranked.iter().map(|it| tier(it.kind)).collect();
        assert!(tiers.windows(2).all(|w| w[0] <= w[1]), "{tiers:?}");
        assert!(tiers.contains(&2) && tiers.contains(&3), "{tiers:?}");
    }

    #[test]
    fn a_plain_assigned_item_beats_the_strongest_notification_and_mention() {
        let strongest = |key: &str, kind| WorkItem {
            in_sprint: true,
            priority_rank: Some(0),
            due_ms: Some(NOW - DAY),
            status: Some("In Progress".to_owned()),
            updated_ms: NOW - 500 * DAY,
            ..item(key, kind)
        };
        let ranked = ranked_keys(vec![
            strongest("NOTE", WorkItemKind::Notification),
            strongest("MENTION", WorkItemKind::Mention),
            strongest("REVIEW", WorkItemKind::ReviewRequested),
            WorkItem {
                priority_rank: Some(4),
                updated_ms: NOW + DAY,
                ..assigned("PLAIN")
            },
        ]);
        assert_eq!(ranked, ["PLAIN", "REVIEW", "MENTION", "NOTE"]);
    }

    #[test]
    fn assigned_order_is_sprint_then_priority_then_due_then_status_then_age() {
        // Each item beats the next one by exactly one criterion, in the owner's order, even though the later
        // ones are better in every criterion after it.
        let a = WorkItem {
            in_sprint: true,
            priority_rank: Some(4),
            updated_ms: NOW,
            ..assigned("A")
        };
        let b = WorkItem {
            in_sprint: false,
            priority_rank: Some(0),
            due_ms: Some(NOW + 40 * DAY),
            updated_ms: NOW,
            ..assigned("B")
        };
        let c = WorkItem {
            priority_rank: Some(1),
            due_ms: Some(NOW - DAY),
            status: Some("In Progress".to_owned()),
            updated_ms: NOW - 400 * DAY,
            ..assigned("C")
        };
        let d = WorkItem {
            priority_rank: Some(1),
            due_ms: Some(NOW + 2 * HOUR),
            status: Some("To Do".to_owned()),
            updated_ms: NOW,
            ..assigned("D")
        };
        let e = WorkItem {
            priority_rank: Some(1),
            due_ms: Some(NOW + 2 * DAY),
            status: Some("In Progress".to_owned()),
            updated_ms: NOW,
            ..assigned("E")
        };
        let f = WorkItem {
            priority_rank: Some(1),
            due_ms: Some(NOW + 2 * DAY),
            status: Some("To Do".to_owned()),
            updated_ms: NOW - 300 * DAY,
            ..assigned("F")
        };
        let g = WorkItem {
            priority_rank: Some(1),
            due_ms: Some(NOW + 2 * DAY),
            status: Some("To Do".to_owned()),
            updated_ms: NOW - 3 * DAY,
            ..assigned("G")
        };
        let h = WorkItem {
            priority_rank: Some(1),
            due_ms: Some(NOW + 20 * DAY),
            status: None,
            updated_ms: NOW,
            ..assigned("H")
        };
        let expected = ["A", "B", "C", "D", "E", "F", "G", "H"];
        let items = vec![h, g, f, e, d, c, b, a];
        assert_eq!(ranked_keys(items.clone()), expected);
        let mut reversed = items;
        reversed.reverse();
        assert_eq!(ranked_keys(reversed), expected);
    }

    #[test]
    fn priority_goes_from_highest_to_lowest_and_unknown_counts_as_medium() {
        let with_rank = |key: &str, rank: Option<u32>| WorkItem {
            priority_rank: rank,
            ..assigned(key)
        };
        let ranked = ranked_keys(vec![
            with_rank("P4", Some(4)),
            with_rank("P2", Some(2)),
            with_rank("P0", Some(0)),
            with_rank("PX", None),
            with_rank("P3", Some(3)),
            with_rank("P1", Some(1)),
            with_rank("P9", Some(9)),
        ]);
        // Unknown ties with medium (P2) and is ordered by key; 9 is clamped to the lowest.
        assert_eq!(ranked, ["P0", "P1", "P2", "PX", "P3", "P4", "P9"]);
    }

    #[test]
    fn due_dates_rank_overdue_then_today_then_soon_then_later_or_none() {
        let with_due = |key: &str, due: Option<i64>| WorkItem {
            due_ms: due,
            ..assigned(key)
        };
        let ranked = ranked_keys(vec![
            with_due("1-none", None),
            with_due("2-later", Some(NOW + 10 * DAY)),
            with_due("3-soon", Some(NOW + 2 * DAY)),
            with_due("4-today", Some(NOW + 3 * HOUR)),
            with_due("5-overdue", Some(NOW - 1)),
            with_due("6-long-overdue", Some(NOW - 90 * DAY)),
        ]);
        // Later and none tie (ordered by key); overdue ones tie with each other.
        assert_eq!(
            ranked,
            [
                "5-overdue",
                "6-long-overdue",
                "4-today",
                "3-soon",
                "1-none",
                "2-later"
            ]
        );
    }

    #[test]
    fn due_boundaries() {
        let due_digit = |due_ms: i64| {
            let ranked = only(WorkItem {
                due_ms: Some(due_ms),
                ..assigned("A")
            });
            (ranked.score / DUE_WEIGHT % 10, ranked.reasons)
        };
        use WorkReason::{Assigned, DueToday, Overdue};
        assert_eq!(due_digit(NOW - 1), (3, vec![Assigned, Overdue]));
        assert_eq!(due_digit(NOW), (2, vec![Assigned, DueToday]));
        assert_eq!(due_digit(NOW + DAY - 1), (2, vec![Assigned, DueToday]));
        assert_eq!(due_digit(NOW + DAY), (1, vec![Assigned]));
        assert_eq!(due_digit(NOW + DUE_SOON_MS - 1), (1, vec![Assigned]));
        assert_eq!(due_digit(NOW + DUE_SOON_MS), (0, vec![Assigned]));
        // A date-only due date passed as the end of the local day: due today all day, overdue the day after.
        let end_of_day = NOW + 5 * HOUR;
        assert_eq!(due_digit(end_of_day).0, 2);
        let day_after = rank(
            vec![WorkItem {
                due_ms: Some(end_of_day),
                ..assigned("A")
            }],
            end_of_day + 1,
        );
        assert_eq!(day_after[0].reasons, [Assigned, Overdue]);
    }

    #[test]
    fn in_progress_ranks_before_to_do() {
        let with_status = |key: &str, status: Option<&str>| WorkItem {
            status: status.map(str::to_owned),
            ..assigned(key)
        };
        let ranked = ranked_keys(vec![
            with_status("A-todo", Some("To Do")),
            with_status("B-none", None),
            with_status("C-doing", Some("Doing")),
            with_status("D-review", Some("In Review")),
            with_status("E-progress", Some("IN PROGRESS")),
        ]);
        assert_eq!(
            ranked,
            ["C-doing", "D-review", "E-progress", "A-todo", "B-none"]
        );
    }

    #[test]
    fn recognizes_in_progress_status_names() {
        for status in [
            "In Progress",
            "in progress",
            "IN_PROGRESS",
            "in-progress",
            "Doing",
            "In Review",
            "Code Review",
            "Active",
            "WIP",
            "Started",
            "workflow::In Progress",
            "workflow::doing",
            "Dev in progress",
            "In Development",
            "W toku",
            "W trakcie",
            "In Arbeit",
            "En cours",
            "En curso",
            "In corso",
            "В работе",
            "indeterminate",
        ] {
            assert!(is_in_progress(status), "{status}");
        }
        for status in [
            "",
            "   ",
            "To Do",
            "Todo",
            "Backlog",
            "Open",
            "New",
            "Selected for Development",
            "Ready for review",
            "Inactive",
            "Done",
            "Closed",
            "Resolved",
            "Blocked",
            "Do zrobienia",
        ] {
            assert!(!is_in_progress(status), "{status}");
        }
    }

    #[test]
    fn older_assigned_items_come_first_and_ties_go_by_key() {
        let updated = |key: &str, ago: i64| WorkItem {
            updated_ms: NOW - ago,
            ..assigned(key)
        };
        let ranked = ranked_keys(vec![
            updated("NEW", 0),
            updated("B-WEEK", 7 * DAY),
            updated("A-WEEK", 7 * DAY),
            updated("YEAR", 365 * DAY),
            updated("FUTURE", -DAY),
            updated("HOUR", HOUR),
        ]);
        // A future update time counts as "just now" (ties with NEW, ordered by key).
        assert_eq!(
            ranked,
            ["YEAR", "A-WEEK", "B-WEEK", "HOUR", "FUTURE", "NEW"]
        );

        // Same key from two sources: the id decides.
        let github = WorkItem {
            id: "github:X-1".to_owned(),
            source: WorkSource::Github,
            ..assigned("X-1")
        };
        let ranked = rank(vec![assigned("X-1"), github], NOW);
        assert_eq!(
            ranked.iter().map(|it| it.id.as_str()).collect::<Vec<_>>(),
            ["github:X-1", "jira:X-1"]
        );
    }

    #[test]
    fn waiting_items_go_review_then_changes_or_ci_then_mentions() {
        let ranked = ranked_keys(vec![
            item("MENTION", WorkItemKind::Mention),
            item("CI", WorkItemKind::CiFailed),
            item("REVIEW", WorkItemKind::ReviewRequested),
            item("CHANGES", WorkItemKind::ChangesRequested),
        ]);
        // Changes requested and failing CI are one group (ordered by key).
        assert_eq!(ranked, ["REVIEW", "CHANGES", "CI", "MENTION"]);

        // Inside a group the one waiting longer comes first.
        let ranked = ranked_keys(vec![
            WorkItem {
                updated_ms: NOW - HOUR,
                ..item("R-HOUR", WorkItemKind::ReviewRequested)
            },
            WorkItem {
                updated_ms: NOW - 6 * DAY,
                ..item("R-WEEK", WorkItemKind::ReviewRequested)
            },
            WorkItem {
                updated_ms: NOW - 30 * DAY,
                ..item("M-MONTH", WorkItemKind::Mention)
            },
        ]);
        assert_eq!(ranked, ["R-WEEK", "R-HOUR", "M-MONTH"]);
    }

    #[test]
    fn notifications_come_newest_first() {
        let note = |key: &str, ago: i64| WorkItem {
            updated_ms: NOW - ago,
            ..item(key, WorkItemKind::Notification)
        };
        let ranked = ranked_keys(vec![
            note("OLD", 10 * DAY),
            note("NEW", 0),
            note("MID", DAY),
        ]);
        assert_eq!(ranked, ["NEW", "MID", "OLD"]);
        assert!(only(note("X", 0)).reasons.is_empty());
    }

    #[test]
    fn score_digits_follow_the_documented_layout() {
        let best = only(WorkItem {
            in_sprint: true,
            priority_rank: Some(0),
            due_ms: Some(NOW - 1),
            status: Some("In Progress".to_owned()),
            updated_ms: NOW - 10 * HOUR,
            ..assigned("A")
        });
        assert_eq!(best.score, 3_014_310_010);

        let review = only(WorkItem {
            updated_ms: NOW - 50 * HOUR - 1,
            ..item("R", WorkItemKind::ReviewRequested)
        });
        assert_eq!(review.score, 2_302_000_050);

        let note = only(WorkItem {
            priority_rank: Some(4),
            updated_ms: NOW - 2 * HOUR,
            ..item("N", WorkItemKind::Notification)
        });
        assert_eq!(note.score, 1_000_009_997);

        // Ages beyond the cap tie; nothing overflows.
        let ancient = only(WorkItem {
            updated_ms: i64::MIN,
            ..assigned("OLD")
        });
        assert_eq!(ancient.score % STATUS_WEIGHT, MAX_AGE_HOURS);
    }

    #[test]
    fn reasons_explain_the_score() {
        use WorkReason::*;
        let full = only(WorkItem {
            in_sprint: true,
            priority_rank: Some(1),
            due_ms: Some(NOW + HOUR),
            status: Some("Doing".to_owned()),
            updated_ms: NOW - 10 * DAY,
            ..assigned("A")
        });
        assert_eq!(
            full.reasons,
            [Assigned, InSprint, HighPriority, DueToday, InProgress]
        );

        let review = only(WorkItem {
            updated_ms: NOW - 4 * DAY,
            due_ms: Some(NOW - DAY),
            ..item("R", WorkItemKind::ReviewRequested)
        });
        assert_eq!(review.reasons, [BlockingOthers, Overdue, WaitingLong]);

        let mention = only(item("M", WorkItemKind::Mention));
        assert_eq!(mention.reasons, [Mentioned]);

        for kind in [WorkItemKind::ChangesRequested, WorkItemKind::CiFailed] {
            assert!(only(item("C", kind)).reasons.is_empty(), "{kind:?}");
            let stale = only(WorkItem {
                updated_ms: NOW - 5 * DAY,
                ..item("C", kind)
            });
            assert_eq!(stale.reasons, [WaitingLong], "{kind:?}");
        }

        // High priority means rank 0 or 1 only; unknown is not high.
        for (rank, high) in [
            (Some(0), true),
            (Some(1), true),
            (Some(2), false),
            (None, false),
        ] {
            let it = only(WorkItem {
                priority_rank: rank,
                ..assigned("P")
            });
            assert_eq!(it.reasons.contains(&HighPriority), high, "{rank:?}");
        }
    }

    #[test]
    fn waiting_long_is_only_for_tier_two_after_more_than_three_days() {
        for kind in ALL_KINDS {
            for (ago, long) in [
                (0, false),
                (WAITING_LONG_MS, false),
                (WAITING_LONG_MS + 1, true),
                (100 * DAY, true),
            ] {
                let it = only(WorkItem {
                    updated_ms: NOW - ago,
                    ..item("W", kind)
                });
                assert_eq!(
                    it.reasons.contains(&WorkReason::WaitingLong),
                    long && tier(kind) == 2,
                    "{kind:?} {ago}"
                );
            }
        }
    }

    #[test]
    fn incoming_scores_and_reasons_are_replaced() {
        let forged = WorkItem {
            score: u32::MAX,
            reasons: vec![WorkReason::Overdue, WorkReason::HighPriority],
            ..item("NOTE", WorkItemKind::Notification)
        };
        let ranked = rank(vec![forged, assigned("A")], NOW);
        assert_eq!(keys(&ranked), ["A", "NOTE"]);
        assert!(ranked[1].reasons.is_empty());
        assert!(ranked[1].score < TIER_WEIGHT * 2);
    }

    #[test]
    fn duplicates_keep_the_best_tier_and_fill_missing_fields() {
        let mention = WorkItem {
            id: "github:o/r#7".to_owned(),
            source: WorkSource::Github,
            kind: WorkItemKind::Mention,
            key: "o/r#7".to_owned(),
            project: Some("o/r".to_owned()),
            due_ms: Some(NOW + HOUR),
            updated_ms: NOW - HOUR,
            unseen: true,
            ..item("o/r#7", WorkItemKind::Mention)
        };
        let assigned_copy = WorkItem {
            kind: WorkItemKind::Assigned,
            title: "Fix login".to_owned(),
            status: Some("In Progress".to_owned()),
            project: None,
            due_ms: None,
            updated_ms: NOW - 2 * DAY,
            unseen: false,
            ..mention.clone()
        };
        for items in [
            vec![mention.clone(), assigned_copy.clone()],
            vec![assigned_copy.clone(), mention.clone()],
        ] {
            let ranked = rank(items, NOW);
            assert_eq!(ranked.len(), 1);
            let merged = &ranked[0];
            assert_eq!(merged.kind, WorkItemKind::Assigned);
            assert_eq!(merged.title, "Fix login");
            assert_eq!(merged.project.as_deref(), Some("o/r"));
            assert_eq!(merged.due_ms, Some(NOW + HOUR));
            assert_eq!(merged.status.as_deref(), Some("In Progress"));
            assert_eq!(merged.updated_ms, NOW - HOUR);
            assert!(merged.unseen);
            assert_eq!(
                merged.reasons,
                [
                    WorkReason::Assigned,
                    WorkReason::DueToday,
                    WorkReason::InProgress
                ]
            );
        }
    }

    #[test]
    fn duplicates_of_one_tier_keep_the_richest_copy() {
        let poor = WorkItem {
            title: "Poor".to_owned(),
            updated_ms: NOW,
            ..assigned("A-1")
        };
        let rich = WorkItem {
            title: "Rich".to_owned(),
            priority_label: Some("Highest".to_owned()),
            priority_rank: Some(0),
            in_sprint: true,
            updated_ms: NOW - DAY,
            ..assigned("A-1")
        };
        for items in [
            vec![poor.clone(), rich.clone()],
            vec![rich.clone(), poor.clone()],
        ] {
            let ranked = rank(items, NOW);
            assert_eq!(ranked.len(), 1);
            assert_eq!(ranked[0].title, "Rich");
            assert_eq!(ranked[0].priority_label.as_deref(), Some("Highest"));
            assert_eq!(ranked[0].updated_ms, NOW);
            assert!(ranked[0].in_sprint);
        }

        // Equal richness: the newer copy wins; a priority label without rank yields to a ranked one.
        let labelled = WorkItem {
            title: "Old".to_owned(),
            priority_label: Some("Urgent".to_owned()),
            updated_ms: NOW - DAY,
            ..assigned("B-1")
        };
        let ranked_copy = WorkItem {
            title: "New".to_owned(),
            priority_rank: Some(1),
            updated_ms: NOW,
            ..assigned("B-1")
        };
        let merged = &rank(vec![labelled, ranked_copy], NOW)[0];
        assert_eq!(merged.title, "New");
        assert_eq!(merged.priority_rank, Some(1));
        assert_eq!(merged.priority_label.as_deref(), Some("Urgent"));
    }

    #[test]
    fn keeps_the_best_fifty() {
        let mut items: Vec<WorkItem> = (0..60)
            .map(|n| WorkItem {
                updated_ms: NOW - n * HOUR,
                ..item(&format!("N-{n:02}"), WorkItemKind::Notification)
            })
            .collect();
        items.push(assigned("ZZZ"));
        let ranked = rank(items, NOW);
        assert_eq!(ranked.len(), MAX_ITEMS);
        assert_eq!(ranked[0].key, "ZZZ");
        assert_eq!(ranked[1].key, "N-00");
        assert_eq!(ranked[MAX_ITEMS - 1].key, "N-48");
    }

    #[test]
    fn result_does_not_depend_on_input_order() {
        let items: Vec<WorkItem> = (0..30_i64)
            .map(|n| {
                let kind = ALL_KINDS[usize::try_from(n).expect("small") % ALL_KINDS.len()];
                WorkItem {
                    in_sprint: n % 4 == 0,
                    priority_rank: u32::try_from(n % 6).ok().filter(|r| *r < 5),
                    due_ms: (n % 3 == 0).then_some(NOW + (n - 10) * DAY),
                    status: (n % 5 == 0).then(|| "In Progress".to_owned()),
                    updated_ms: NOW - (n % 7) * DAY,
                    ..item(&format!("K-{}", n % 9), kind)
                }
            })
            .map(|it| WorkItem {
                id: format!("{}-{:?}", it.key, it.kind),
                ..it
            })
            .collect();
        let forward = rank(items.clone(), NOW);
        let mut reversed = items.clone();
        reversed.reverse();
        assert_eq!(rank(reversed, NOW), forward);
        let mut rotated = items;
        rotated.rotate_left(11);
        assert_eq!(rank(rotated, NOW), forward);
    }

    #[test]
    fn cleans_text_links_and_priorities() {
        let messy = only(WorkItem {
            key: "  PROJ-1\n".to_owned(),
            title: format!("  Fix\tthe\r\nbug {}", "x".repeat(400)),
            url: "javascript:alert(1)".to_owned(),
            project: Some("   ".to_owned()),
            priority_label: Some(" High ".to_owned()),
            priority_rank: Some(17),
            status: Some("\u{7}".to_owned()),
            ..assigned("PROJ-1")
        });
        assert_eq!(messy.key, "PROJ-1");
        assert!(messy.title.starts_with("Fix the  bug x"), "{}", messy.title);
        assert_eq!(messy.title.chars().count(), MAX_TITLE_CHARS);
        assert!(messy.title.ends_with('…'));
        assert_eq!(messy.url, "");
        assert_eq!(messy.project, None);
        assert_eq!(messy.priority_label.as_deref(), Some("High"));
        assert_eq!(messy.priority_rank, Some(LOWEST_PRIORITY_RANK));
        assert_eq!(messy.status, None);

        for (url, kept) in [
            ("https://github.com/o/r/issues/1", true),
            ("  HTTPS://gitlab.com/g/p/-/issues/2 ", true),
            ("http://jira.intranet:8080/browse/X-1", true),
            ("https://", false),
            ("https:///path", false),
            ("https://@evil", false),
            ("file:///C:/Windows/system32/calc.exe", false),
            ("ms-settings:privacy", false),
            ("https://a b", false),
            ("", false),
        ] {
            let it = only(WorkItem {
                url: url.to_owned(),
                ..assigned("U")
            });
            assert_eq!(it.url.is_empty(), !kept, "{url}");
            if kept {
                assert_eq!(it.url, url.trim());
            }
        }
        let long = format!("https://example.com/{}", "a".repeat(MAX_URL_LEN));
        assert_eq!(safe_url(&long), "");
    }

    #[test]
    fn extreme_times_never_panic() {
        for now in [i64::MIN, -1, 0, NOW, i64::MAX] {
            let items: Vec<WorkItem> = ALL_KINDS
                .into_iter()
                .flat_map(|kind| {
                    [i64::MIN, -1, 0, NOW, i64::MAX].map(|t| WorkItem {
                        id: format!("{kind:?}-{t}"),
                        due_ms: Some(t),
                        updated_ms: t,
                        priority_rank: Some(u32::MAX),
                        ..item("K", kind)
                    })
                })
                .collect();
            let ranked = rank(items, now);
            assert_eq!(ranked.len(), 30);
            let tiers: Vec<u32> = ranked.iter().map(|it| tier(it.kind)).collect();
            assert!(tiers.windows(2).all(|w| w[0] <= w[1]), "{now}: {tiers:?}");
        }
    }

    #[test]
    fn normalizes_session_goals() {
        let goal = |key: &str, title: &str, url: &str| SessionGoal {
            source: WorkSource::Linear,
            key: key.to_owned(),
            title: title.to_owned(),
            url: url.to_owned(),
        };
        assert_eq!(
            normalize_goal(goal(
                " ENG-7 ",
                "Ship\nit ",
                "https://linear.app/x/issue/ENG-7"
            )),
            Some(goal("ENG-7", "Ship it", "https://linear.app/x/issue/ENG-7"))
        );
        assert_eq!(
            normalize_goal(goal("ENG-7", "", "javascript:alert(1)")),
            Some(goal("ENG-7", "", ""))
        );
        assert_eq!(normalize_goal(goal("  ", "\t", "https://x.dev")), None);
        let long = normalize_goal(goal("K", &"y".repeat(1000), "")).expect("goal");
        assert_eq!(long.title.chars().count(), MAX_TITLE_CHARS);
    }
}
