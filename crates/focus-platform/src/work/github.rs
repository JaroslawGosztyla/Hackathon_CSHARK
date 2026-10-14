//! GitHub (github.com), read-only, with a personal access token (classic, or fine-grained).
//!
//! * Account: `GET /user` (`login`).
//! * Assigned: `GET /search/issues?q=is:open is:issue assignee:@me archived:false` → [`WorkItemKind::Assigned`].
//!   Priority from labels (`P0`..`P4`, `priority: high`, …), "in progress" from labels, the milestone's
//!   `due_on` as the due date (GitHub milestones are dates).
//! * Review requested: `is:open is:pr review-requested:@me archived:false` → [`WorkItemKind::ReviewRequested`]
//!   (GitHub drops the request once the user reviewed).
//! * The user's own open pull requests (`author:@me`, at most [`MAX_OWN_PULLS`]): `GET /pulls/{n}` for the head
//!   SHA, `GET /pulls/{n}/reviews` (a reviewer whose latest approving / rejecting review requests changes →
//!   [`WorkItemKind::ChangesRequested`]) and `GET /commits/{sha}/check-runs` (a failed, timed-out or
//!   startup-failed run; else the combined commit status `failure` / `error`) → [`WorkItemKind::CiFailed`].
//!   Fine-grained tokens cannot call the Checks API: a refused check-runs request falls back to the commit
//!   status, and a refused status leaves CI unknown (no item, no warning).
//! * Notifications: `GET /notifications?participating=true` (unread only) with `If-Modified-Since` set to the
//!   last `Last-Modified` (a 304 reuses the previous items and costs no rate limit); `X-Poll-Interval` is passed on
//!   as [`WorkFetch::poll_interval_secs`]. Reason `mention` → [`WorkItemKind::Mention`], others →
//!   [`WorkItemKind::Notification`]. GitHub serves notifications only to classic tokens (scope `notifications`);
//!   with a fine-grained token this part is skipped with a warning.
//!
//! Scopes: classic `repo` (or `public_repo`) + `notifications`; fine-grained: Issues, Pull requests and Commit
//! statuses (read; GitHub Actions check runs are not readable with it). Blocking, see the module docs of
//! [`crate::work`].

use focus_core::api::{WorkItem, WorkItemKind, WorkSource};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, IF_MODIFIED_SINCE, LAST_MODIFIED};
use serde::Deserialize;

use super::{
    MAX_REDIRECTS, Reply, Token, WorkFetch, WorkProvider, due_end_of_day, item_id, new_item,
    non_empty, parse_each, parse_instant, parse_json, priority_from_labels, push_segments, send,
    status_error, status_from_labels,
};
use crate::PlatformError;
use crate::calendar::TimeZone;
use crate::net::LazyClient;

const API: &str = "https://api.github.com";
/// Current REST API version (2022-11-28 stays supported until at least March 2028).
const API_VERSION: &str = "2026-03-10";
const MEDIA_TYPE: &str = "application/vnd.github+json";
const SERVICE: &str = "GitHub";
/// Results per search (GitHub allows 100).
const SEARCH_PAGE: &str = "50";
/// Own open pull requests checked for reviews and CI (each costs up to 4 requests).
pub const MAX_OWN_PULLS: usize = 10;
/// Default `X-Poll-Interval` of the notifications API.
pub const DEFAULT_POLL_SECS: u32 = 60;
/// Check-run conclusions that mean "CI failed".
const FAILED_CONCLUSIONS: &[&str] = &["failure", "timed_out", "startup_failure"];

/// GitHub client; keeps the notifications `Last-Modified` and items between fetches.
#[derive(Debug)]
pub struct GithubClient {
    token: Token,
    http: LazyClient,
    notifications: Option<NotificationCache>,
}

#[derive(Debug, Clone)]
struct NotificationCache {
    last_modified: String,
    items: Vec<WorkItem>,
}

impl GithubClient {
    /// Cheap: no I/O.
    pub fn new(token: String) -> Self {
        Self {
            token: Token::new(token),
            http: LazyClient::same_origin(MAX_REDIRECTS),
            notifications: None,
        }
    }

    fn request(
        &self,
        url: reqwest::Url,
        if_modified_since: Option<&str>,
    ) -> Result<Reply, PlatformError> {
        let mut request = self
            .http
            .get()?
            .get(url)
            .header(ACCEPT, MEDIA_TYPE)
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(AUTHORIZATION, self.token.header("Bearer ")?);
        if let Some(since) = if_modified_since {
            request = request.header(IF_MODIFIED_SINCE, since);
        }
        send(request)
    }

    fn get(&self, url: reqwest::Url) -> Result<String, PlatformError> {
        self.request(url, None)?.into_body(SERVICE)
    }

    fn api(segments: &[&str], query: &[(&str, &str)]) -> Result<reqwest::Url, PlatformError> {
        let mut url = super::build_url(API, "", query)?;
        push_segments(&mut url, segments)?;
        Ok(url)
    }

    fn search(&self, query: &str, per_page: &str) -> Result<Vec<GhIssue>, PlatformError> {
        let url = Self::api(
            &["search", "issues"],
            &[
                ("q", query),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", per_page),
            ],
        )?;
        parse_search(&self.get(url)?)
    }

    /// Own pull requests that need the user: changes requested or CI failed.
    fn own_pull_items(
        &self,
        pulls: &[GhIssue],
        login: &str,
        tz: &TimeZone,
        warnings: &mut Vec<String>,
    ) -> Vec<WorkItem> {
        let mut items = Vec::new();
        let mut failures = 0;
        for pull in pulls.iter().take(MAX_OWN_PULLS) {
            match self.pull_state(pull, login) {
                Ok(Some(kind)) => items.extend(issue_item(pull, kind, tz)),
                Ok(None) => {}
                Err(err) => {
                    failures += 1;
                    log::debug!("github: checking an own pull request failed: {err}");
                }
            }
        }
        if failures > 0 {
            warnings.push(format!(
                "GitHub: {failures} of your pull requests could not be checked for reviews and CI"
            ));
        }
        items
    }

    fn pull_state(
        &self,
        pull: &GhIssue,
        login: &str,
    ) -> Result<Option<WorkItemKind>, PlatformError> {
        let Some((owner, repo)) = repo_of(&pull.repository_url) else {
            return Ok(None);
        };
        let number = pull.number.to_string();
        let reviews_url = Self::api(
            &["repos", owner, repo, "pulls", &number, "reviews"],
            &[("per_page", "100")],
        )?;
        let reviews: Vec<serde_json::Value> = parse_json(&self.get(reviews_url)?, SERVICE)?;
        if changes_requested(&parse_each(reviews), login) {
            return Ok(Some(WorkItemKind::ChangesRequested));
        }
        let pull_url = Self::api(&["repos", owner, repo, "pulls", &number], &[])?;
        let detail: GhPull = parse_json(&self.get(pull_url)?, SERVICE)?;
        let Some(sha) = detail.head.and_then(|h| h.sha).filter(|s| is_sha(s)) else {
            return Ok(None);
        };
        // Fine-grained tokens cannot call the Checks API (GitHub's documented limitation): a refused check-runs
        // request falls back to the combined commit status, and a refused status means "CI unknown", not an error.
        let runs_url = Self::api(
            &["repos", owner, repo, "commits", &sha, "check-runs"],
            &[("filter", "latest"), ("per_page", "100")],
        )?;
        match self.get(runs_url) {
            Ok(body) => {
                let runs: GhCheckRuns = parse_json(&body, SERVICE)?;
                if checks_failed(&runs) {
                    return Ok(Some(WorkItemKind::CiFailed));
                }
            }
            Err(PlatformError::Auth(_)) => {}
            Err(err) => return Err(err),
        }
        let status_url = Self::api(&["repos", owner, repo, "commits", &sha, "status"], &[])?;
        let status: GhCombinedStatus = match self.get(status_url) {
            Ok(body) => parse_json(&body, SERVICE)?,
            Err(PlatformError::Auth(_)) => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(status_failed(&status).then_some(WorkItemKind::CiFailed))
    }

    /// Unread participating notifications (cached on 304) and the poll interval.
    fn notification_items(&mut self, warnings: &mut Vec<String>) -> (Vec<WorkItem>, Option<u32>) {
        let url = match Self::api(
            &["notifications"],
            &[("participating", "true"), ("per_page", "50")],
        ) {
            Ok(url) => url,
            Err(err) => {
                warnings.push(format!("GitHub notifications: {err}"));
                return (Vec::new(), None);
            }
        };
        let since = self
            .notifications
            .as_ref()
            .map(|cache| cache.last_modified.clone());
        let reply = match self.request(url, since.as_deref()) {
            Ok(reply) => reply,
            Err(err) => {
                warnings.push(format!("GitHub notifications: {err}"));
                return (self.cached_notifications(), None);
            }
        };
        let poll = poll_interval(&reply.headers);
        if reply.status == 304 {
            return (self.cached_notifications(), poll);
        }
        if !reply.is_success() {
            let err = status_error(reply.status, &reply.headers, "GitHub notifications");
            warnings.push(if matches!(err, PlatformError::Auth(_)) {
                "GitHub notifications need a classic token with the notifications scope".into()
            } else {
                err.to_string()
            });
            self.notifications = None;
            return (Vec::new(), poll);
        }
        match parse_json::<Vec<serde_json::Value>>(&reply.body, "GitHub notifications") {
            Ok(threads) => {
                let items = notification_items(parse_each(threads));
                self.notifications = reply
                    .headers
                    .get(LAST_MODIFIED)
                    .and_then(|v| v.to_str().ok())
                    .map(|last_modified| NotificationCache {
                        last_modified: last_modified.to_owned(),
                        items: items.clone(),
                    });
                (items, poll)
            }
            Err(err) => {
                warnings.push(err.to_string());
                (self.cached_notifications(), poll)
            }
        }
    }

    fn cached_notifications(&self) -> Vec<WorkItem> {
        self.notifications
            .as_ref()
            .map(|cache| cache.items.clone())
            .unwrap_or_default()
    }
}

impl WorkProvider for GithubClient {
    fn source(&self) -> WorkSource {
        WorkSource::Github
    }

    fn fetch(&mut self, _now_ms: i64) -> Result<WorkFetch, PlatformError> {
        let tz = TimeZone::local();
        let user: GhUser = parse_json(&self.get(Self::api(&["user"], &[])?)?, SERVICE)?;
        let login = user.login.unwrap_or_default();
        let mut warnings = Vec::new();

        let assigned = self.search("is:open is:issue assignee:@me archived:false", SEARCH_PAGE)?;
        let mut items: Vec<WorkItem> = assigned
            .iter()
            .filter_map(|issue| issue_item(issue, WorkItemKind::Assigned, &tz))
            .collect();

        let reviews = self.search(
            "is:open is:pr review-requested:@me archived:false",
            SEARCH_PAGE,
        )?;
        items.extend(
            reviews
                .iter()
                .filter_map(|pull| issue_item(pull, WorkItemKind::ReviewRequested, &tz)),
        );

        match self.search(
            "is:open is:pr author:@me archived:false",
            &MAX_OWN_PULLS.to_string(),
        ) {
            Ok(own) => items.extend(self.own_pull_items(&own, &login, &tz, &mut warnings)),
            Err(err) => warnings.push(format!("GitHub own pull requests: {err}")),
        }

        let (notifications, poll) = self.notification_items(&mut warnings);
        items.extend(notifications);
        Ok(WorkFetch {
            account: non_empty(Some(&login)),
            items,
            poll_interval_secs: Some(poll.unwrap_or(DEFAULT_POLL_SECS)),
            warnings,
        })
    }
}

// -- responses -----------------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhUser {
    login: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhSearch {
    items: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct GhIssue {
    number: u64,
    title: Option<String>,
    html_url: Option<String>,
    repository_url: String,
    labels: Vec<GhLabel>,
    milestone: Option<GhMilestone>,
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct GhLabel {
    name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct GhMilestone {
    due_on: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhPull {
    head: Option<GhHead>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhHead {
    sha: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhReview {
    user: Option<GhUser>,
    state: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhCheckRuns {
    check_runs: Vec<GhCheckRun>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhCheckRun {
    conclusion: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhCombinedStatus {
    state: Option<String>,
    total_count: u32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhThread {
    id: String,
    unread: bool,
    reason: Option<String>,
    updated_at: Option<String>,
    subject: GhSubject,
    repository: GhRepository,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhSubject {
    title: Option<String>,
    url: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GhRepository {
    full_name: Option<String>,
    html_url: Option<String>,
}

// -- mapping (pure) -------------------------------------------------------------------------------------------

fn parse_search(body: &str) -> Result<Vec<GhIssue>, PlatformError> {
    let page: GhSearch = parse_json(body, SERVICE)?;
    Ok(parse_each(page.items))
}

/// `owner`, `repo` from `https://api.github.com/repos/owner/repo`.
fn repo_of(repository_url: &str) -> Option<(&str, &str)> {
    let rest = repository_url.split("/repos/").nth(1)?;
    let mut parts = rest.trim_end_matches('/').split('/');
    let owner = parts.next().filter(|p| !p.is_empty())?;
    let repo = parts.next().filter(|p| !p.is_empty())?;
    parts.next().is_none().then_some((owner, repo))
}

fn is_sha(text: &str) -> bool {
    (7..=64).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A search result (issue or pull request) as an item of `kind`.
fn issue_item(issue: &GhIssue, kind: WorkItemKind, tz: &TimeZone) -> Option<WorkItem> {
    let (owner, repo) = repo_of(&issue.repository_url)?;
    if issue.number == 0 {
        return None;
    }
    let full_name = format!("{owner}/{repo}");
    let key = format!("{full_name}#{}", issue.number);
    let labels: Vec<&str> = issue
        .labels
        .iter()
        .filter_map(|label| label.name.as_deref())
        .collect();
    let mut item = new_item(
        WorkSource::Github,
        kind,
        key,
        issue.title.clone().unwrap_or_default(),
        issue.html_url.clone().unwrap_or_default(),
        issue
            .updated_at
            .as_deref()
            .and_then(parse_instant)
            .unwrap_or(0),
    );
    item.project = Some(full_name);
    if let Some((label, rank)) = priority_from_labels(labels.iter().copied()) {
        item.priority_label = Some(label);
        item.priority_rank = Some(rank);
    }
    item.status = status_from_labels(labels.iter().copied());
    item.due_ms = issue
        .milestone
        .as_ref()
        .filter(|m| m.state.as_deref() != Some("closed"))
        .and_then(|m| m.due_on.as_deref())
        .and_then(|due| due_end_of_day(due, tz));
    Some(item)
}

/// A reviewer (other than the author) whose latest approving / rejecting review requests changes. Reviews come
/// in chronological order; comments and pending reviews do not change a reviewer's decision.
fn changes_requested(reviews: &[GhReview], author: &str) -> bool {
    let mut latest: Vec<(String, bool)> = Vec::new();
    for review in reviews {
        let Some(login) = review.user.as_ref().and_then(|u| u.login.as_deref()) else {
            continue;
        };
        if login.eq_ignore_ascii_case(author) {
            continue;
        }
        let requested = match review.state.as_deref() {
            Some("CHANGES_REQUESTED") => true,
            Some("APPROVED" | "DISMISSED") => false,
            _ => continue,
        };
        match latest.iter_mut().find(|(name, _)| name == login) {
            Some(entry) => entry.1 = requested,
            None => latest.push((login.to_owned(), requested)),
        }
    }
    latest.iter().any(|(_, requested)| *requested)
}

fn checks_failed(runs: &GhCheckRuns) -> bool {
    runs.check_runs.iter().any(|run| {
        run.conclusion
            .as_deref()
            .is_some_and(|c| FAILED_CONCLUSIONS.contains(&c))
    })
}

fn status_failed(status: &GhCombinedStatus) -> bool {
    status.total_count > 0 && matches!(status.state.as_deref(), Some("failure" | "error"))
}

/// `X-Poll-Interval` in seconds (1..=86400).
fn poll_interval(headers: &HeaderMap) -> Option<u32> {
    headers
        .get("x-poll-interval")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|secs| (1..=86_400).contains(secs))
}

fn notification_items(threads: Vec<GhThread>) -> Vec<WorkItem> {
    threads.iter().filter_map(thread_item).collect()
}

/// An unread notification thread. Issues, pull requests and discussions get the key `owner/repo#n` (so they
/// merge with search results); other subjects (releases, CI runs, commits) are keyed by the repository with an
/// id of their own.
fn thread_item(thread: &GhThread) -> Option<WorkItem> {
    if !thread.unread {
        return None;
    }
    let full_name = non_empty(thread.repository.full_name.as_deref())?;
    let repo_html = thread
        .repository
        .html_url
        .clone()
        .unwrap_or_else(|| format!("https://github.com/{full_name}"));
    let kind_name = thread.subject.kind.as_deref().unwrap_or("");
    let number = thread
        .subject
        .url
        .as_deref()
        .and_then(|url| url.trim_end_matches('/').rsplit('/').next())
        .and_then(|n| n.parse::<u64>().ok())
        .filter(|&n| n > 0);
    let web_path = match kind_name {
        "Issue" => Some("issues"),
        "PullRequest" => Some("pull"),
        "Discussion" => Some("discussions"),
        _ => None,
    };
    let kind = if thread.reason.as_deref() == Some("mention") {
        WorkItemKind::Mention
    } else {
        WorkItemKind::Notification
    };
    let title = thread.subject.title.clone().unwrap_or_default();
    let updated = thread
        .updated_at
        .as_deref()
        .and_then(parse_instant)
        .unwrap_or(0);
    let mut item = match (web_path, number) {
        (Some(path), Some(n)) => new_item(
            WorkSource::Github,
            kind,
            format!("{full_name}#{n}"),
            title,
            format!("{repo_html}/{path}/{n}"),
            updated,
        ),
        _ => {
            let url = match kind_name {
                "Release" => format!("{repo_html}/releases"),
                "CheckSuite" => format!("{repo_html}/actions"),
                _ => repo_html,
            };
            let mut item = new_item(
                WorkSource::Github,
                kind,
                full_name.clone(),
                title,
                url,
                updated,
            );
            item.id = item_id(WorkSource::Github, &format!("thread-{}", thread.id));
            item
        }
    };
    item.project = Some(full_name);
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::test_support::{cest, end_of_day, utc};
    use reqwest::header::HeaderValue;

    /// Trimmed `GET /search/issues` response (fields as GitHub sends them).
    const SEARCH_ASSIGNED: &str = r#"{
      "total_count": 3,
      "incomplete_results": false,
      "items": [
        {
          "url": "https://api.github.com/repos/acme/api/issues/45",
          "repository_url": "https://api.github.com/repos/acme/api",
          "html_url": "https://github.com/acme/api/issues/45",
          "id": 3301245981,
          "node_id": "I_kwDOAbc123",
          "number": 45,
          "title": "Login fails with SSO",
          "user": {"login": "octo", "id": 1},
          "labels": [
            {"id": 1, "name": "bug", "color": "d73a4a", "default": true, "description": null},
            {"id": 2, "name": "priority: high", "color": "b60205", "default": false, "description": ""},
            {"id": 3, "name": "In progress", "color": "0e8a16", "default": false, "description": null}
          ],
          "state": "open",
          "locked": false,
          "assignee": {"login": "me"},
          "assignees": [{"login": "me"}],
          "milestone": {
            "url": "https://api.github.com/repos/acme/api/milestones/3",
            "number": 3,
            "title": "Sprint 42",
            "state": "open",
            "due_on": "2026-10-15T07:00:00Z"
          },
          "comments": 4,
          "created_at": "2026-09-30T08:00:00Z",
          "updated_at": "2026-10-12T09:30:00Z",
          "closed_at": null,
          "author_association": "MEMBER",
          "body": "Steps to reproduce",
          "score": 1.0
        },
        {
          "url": "https://api.github.com/repos/acme/web/issues/7",
          "repository_url": "https://api.github.com/repos/acme/web",
          "html_url": "https://github.com/acme/web/issues/7",
          "number": 7,
          "title": "Dark mode",
          "labels": [],
          "state": "open",
          "milestone": null,
          "updated_at": "2026-10-01T10:00:00Z"
        },
        {"number": "not a number"}
      ]
    }"#;

    const SEARCH_REVIEWS: &str = r#"{
      "total_count": 1,
      "incomplete_results": false,
      "items": [
        {
          "repository_url": "https://api.github.com/repos/acme/api",
          "html_url": "https://github.com/acme/api/pull/51",
          "number": 51,
          "title": "Add retry to the HTTP client",
          "labels": [{"name": "P2"}],
          "state": "open",
          "draft": false,
          "pull_request": {
            "url": "https://api.github.com/repos/acme/api/pulls/51",
            "html_url": "https://github.com/acme/api/pull/51",
            "merged_at": null
          },
          "updated_at": "2026-10-09T15:00:00Z"
        }
      ]
    }"#;

    const REVIEWS: &str = r#"[
      {"id": 1, "user": {"login": "alice"}, "body": "", "state": "CHANGES_REQUESTED",
       "submitted_at": "2026-10-10T10:00:00Z", "commit_id": "a1b2c3d"},
      {"id": 2, "user": {"login": "bob"}, "body": "nit", "state": "COMMENTED",
       "submitted_at": "2026-10-10T11:00:00Z", "commit_id": "a1b2c3d"},
      {"id": 3, "user": {"login": "alice"}, "body": "", "state": "APPROVED",
       "submitted_at": "2026-10-11T10:00:00Z", "commit_id": "e4f5a6b"},
      {"id": 4, "user": {"login": "bob"}, "body": "", "state": "CHANGES_REQUESTED",
       "submitted_at": "2026-10-11T12:00:00Z", "commit_id": "e4f5a6b"},
      {"id": 5, "user": {"login": "bob"}, "body": "", "state": "COMMENTED",
       "submitted_at": "2026-10-11T13:00:00Z", "commit_id": "e4f5a6b"}
    ]"#;

    const CHECK_RUNS: &str = r#"{
      "total_count": 3,
      "check_runs": [
        {"id": 11, "name": "build", "status": "completed", "conclusion": "success"},
        {"id": 12, "name": "test", "status": "completed", "conclusion": "failure"},
        {"id": 13, "name": "lint", "status": "in_progress", "conclusion": null}
      ]
    }"#;

    const NOTIFICATIONS: &str = r#"[
      {
        "id": "15001",
        "unread": true,
        "reason": "mention",
        "updated_at": "2026-10-12T08:00:00Z",
        "last_read_at": null,
        "subject": {
          "title": "Login fails with SSO",
          "url": "https://api.github.com/repos/acme/api/issues/45",
          "latest_comment_url": "https://api.github.com/repos/acme/api/issues/comments/99",
          "type": "Issue"
        },
        "repository": {"id": 1, "full_name": "acme/api", "html_url": "https://github.com/acme/api", "private": true},
        "url": "https://api.github.com/notifications/threads/15001",
        "subscription_url": "https://api.github.com/notifications/threads/15001/subscription"
      },
      {
        "id": "15002",
        "unread": true,
        "reason": "review_requested",
        "updated_at": "2026-10-11T08:00:00Z",
        "subject": {"title": "Add retry", "url": "https://api.github.com/repos/acme/api/pulls/51", "latest_comment_url": null, "type": "PullRequest"},
        "repository": {"full_name": "acme/api", "html_url": "https://github.com/acme/api"}
      },
      {
        "id": "15003",
        "unread": true,
        "reason": "ci_activity",
        "updated_at": "2026-10-12T09:00:00Z",
        "subject": {"title": "CI workflow run failed for main branch", "url": null, "latest_comment_url": null, "type": "CheckSuite"},
        "repository": {"full_name": "acme/web", "html_url": "https://github.com/acme/web"}
      },
      {
        "id": "15004",
        "unread": false,
        "reason": "comment",
        "updated_at": "2026-10-10T09:00:00Z",
        "subject": {"title": "Old", "url": "https://api.github.com/repos/acme/web/issues/3", "type": "Issue"},
        "repository": {"full_name": "acme/web", "html_url": "https://github.com/acme/web"}
      }
    ]"#;

    #[test]
    fn assigned_issues_map_with_labels_and_milestones() {
        let tz = cest();
        let issues = parse_search(SEARCH_ASSIGNED).expect("search");
        assert_eq!(issues.len(), 2, "the malformed item is skipped");
        let items: Vec<WorkItem> = issues
            .iter()
            .filter_map(|i| issue_item(i, WorkItemKind::Assigned, &tz))
            .collect();
        let first = &items[0];
        assert_eq!(first.id, "github:acme/api#45");
        assert_eq!(first.key, "acme/api#45");
        assert_eq!(first.kind, WorkItemKind::Assigned);
        assert_eq!(first.source, WorkSource::Github);
        assert_eq!(first.title, "Login fails with SSO");
        assert_eq!(first.url, "https://github.com/acme/api/issues/45");
        assert_eq!(first.project.as_deref(), Some("acme/api"));
        assert_eq!(first.priority_label.as_deref(), Some("priority: high"));
        assert_eq!(first.priority_rank, Some(1));
        assert_eq!(first.status.as_deref(), Some("In progress"));
        assert_eq!(first.due_ms, Some(end_of_day(2026, 10, 15, &tz)));
        assert_eq!(first.updated_ms, utc(2026, 10, 12, 9, 30));
        assert_eq!(
            (first.score, first.unseen, first.in_sprint),
            (0, false, false)
        );
        assert!(first.reasons.is_empty());

        let second = &items[1];
        assert_eq!(second.key, "acme/web#7");
        assert_eq!(
            (
                second.priority_rank,
                second.due_ms,
                second.status.as_deref()
            ),
            (None, None, None)
        );
    }

    #[test]
    fn review_requests_map_to_tier_two() {
        let tz = cest();
        let pulls = parse_search(SEARCH_REVIEWS).expect("search");
        let item = issue_item(&pulls[0], WorkItemKind::ReviewRequested, &tz).expect("item");
        assert_eq!(item.id, "github:acme/api#51");
        assert_eq!(item.kind, WorkItemKind::ReviewRequested);
        assert_eq!(item.url, "https://github.com/acme/api/pull/51");
        assert_eq!(item.priority_rank, Some(2));
        assert_eq!(item.updated_ms, utc(2026, 10, 9, 15, 0));
    }

    #[test]
    fn latest_decisive_review_counts() {
        let reviews: Vec<GhReview> = parse_each(serde_json::from_str(REVIEWS).expect("json"));
        // alice approved after requesting changes; bob's latest decisive review requests changes.
        assert!(changes_requested(&reviews, "me"));
        // bob is the author: his own reviews do not count, alice approved.
        assert!(!changes_requested(&reviews, "BOB"));
        assert!(!changes_requested(&[], "me"));
        let dismissed: Vec<GhReview> = parse_each(
            serde_json::from_str(
                r#"[{"user":{"login":"x"},"state":"CHANGES_REQUESTED"},{"user":{"login":"x"},"state":"DISMISSED"}]"#,
            )
            .expect("json"),
        );
        assert!(!changes_requested(&dismissed, "me"));
    }

    #[test]
    fn failing_checks_and_statuses() {
        let runs: GhCheckRuns = parse_json(CHECK_RUNS, SERVICE).expect("runs");
        assert!(checks_failed(&runs));
        let green: GhCheckRuns = parse_json(
            r#"{"total_count":1,"check_runs":[{"conclusion":"success"},{"conclusion":"neutral"},{"conclusion":"cancelled"}]}"#,
            SERVICE,
        )
        .expect("runs");
        assert!(!checks_failed(&green));
        let timed_out: GhCheckRuns =
            parse_json(r#"{"check_runs":[{"conclusion":"timed_out"}]}"#, SERVICE).expect("runs");
        assert!(checks_failed(&timed_out));

        let failed: GhCombinedStatus = parse_json(
            r#"{"state":"failure","total_count":2,"statuses":[]}"#,
            SERVICE,
        )
        .expect("status");
        assert!(status_failed(&failed));
        let none: GhCombinedStatus = parse_json(
            r#"{"state":"pending","total_count":0,"statuses":[]}"#,
            SERVICE,
        )
        .expect("status");
        assert!(!status_failed(&none));
        let error_without_statuses: GhCombinedStatus =
            parse_json(r#"{"state":"error","total_count":0}"#, SERVICE).expect("status");
        assert!(!status_failed(&error_without_statuses));
    }

    #[test]
    fn notifications_map_mentions_and_others() {
        let threads: Vec<GhThread> = parse_each(serde_json::from_str(NOTIFICATIONS).expect("json"));
        let items = notification_items(threads);
        assert_eq!(items.len(), 3, "read threads are skipped");

        assert_eq!(items[0].id, "github:acme/api#45");
        assert_eq!(items[0].kind, WorkItemKind::Mention);
        assert_eq!(items[0].url, "https://github.com/acme/api/issues/45");
        assert_eq!(items[0].updated_ms, utc(2026, 10, 12, 8, 0));

        assert_eq!(items[1].id, "github:acme/api#51");
        assert_eq!(items[1].kind, WorkItemKind::Notification);
        assert_eq!(items[1].url, "https://github.com/acme/api/pull/51");

        assert_eq!(items[2].id, "github:thread-15003");
        assert_eq!(items[2].key, "acme/web");
        assert_eq!(items[2].url, "https://github.com/acme/web/actions");
        assert_eq!(items[2].project.as_deref(), Some("acme/web"));
        assert_eq!(items[2].title, "CI workflow run failed for main branch");
    }

    #[test]
    fn poll_interval_header() {
        let mut headers = HeaderMap::new();
        assert_eq!(poll_interval(&headers), None);
        headers.insert("x-poll-interval", HeaderValue::from_static("60"));
        assert_eq!(poll_interval(&headers), Some(60));
        headers.insert("x-poll-interval", HeaderValue::from_static("0"));
        assert_eq!(poll_interval(&headers), None);
        headers.insert("x-poll-interval", HeaderValue::from_static("soon"));
        assert_eq!(poll_interval(&headers), None);
    }

    #[test]
    fn repositories_and_shas_are_parsed_strictly() {
        assert_eq!(
            repo_of("https://api.github.com/repos/acme/api"),
            Some(("acme", "api"))
        );
        assert_eq!(repo_of("https://api.github.com/repos/acme"), None);
        assert_eq!(repo_of("https://api.github.com/repos/a/b/c"), None);
        assert_eq!(repo_of(""), None);
        assert!(is_sha("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"));
        assert!(!is_sha("../../x"));
        assert!(!is_sha("abc"));
    }

    #[test]
    fn urls_are_encoded() {
        let url = GithubClient::api(
            &["search", "issues"],
            &[("q", "is:open is:issue assignee:@me archived:false")],
        )
        .expect("url");
        assert_eq!(
            url.as_str(),
            "https://api.github.com/search/issues?q=is%3Aopen+is%3Aissue+assignee%3A%40me+archived%3Afalse"
        );
    }

    #[test]
    fn debug_never_shows_the_token() {
        let client = GithubClient::new("ghp_supersecret".into());
        assert!(!format!("{client:?}").contains("supersecret"));
    }
}
