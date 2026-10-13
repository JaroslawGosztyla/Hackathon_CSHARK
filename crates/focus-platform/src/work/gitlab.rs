//! GitLab (gitlab.com or a self-managed instance), read-only, with a personal access token (`read_api`) sent in
//! the `PRIVATE-TOKEN` header.
//!
//! * Account: `GET /api/v4/user` (`username`, `id`).
//! * Assigned: `GET /api/v4/issues?scope=assigned_to_me&state=opened` → [`WorkItemKind::Assigned`] with
//!   `due_date` (date only), priority from labels (`priority::1`, `P2`, …), "in progress" from labels
//!   (`workflow::in progress`, `Doing`, …) and the current iteration (`iteration.state` 2, or today between its
//!   start and due date) as "in sprint".
//! * Review requested: `GET /api/v4/merge_requests?scope=all&reviewer_id=<me>&state=opened`, without the ones
//!   the user already reviewed or approved (`GET …/merge_requests/:iid/reviewers`, reviewer `state`; at most
//!   [`MAX_REVIEW_CHECKS`] are checked) → [`WorkItemKind::ReviewRequested`].
//! * The user's own open merge requests (`scope=created_by_me`, at most [`MAX_OWN_MRS`]):
//!   `detailed_merge_status` `requested_changes` → [`WorkItemKind::ChangesRequested`], else a `failed` head
//!   pipeline (`GET /projects/:id/merge_requests/:iid`) → [`WorkItemKind::CiFailed`].
//! * To-do items: `GET /api/v4/todos?state=pending`; `mentioned` / `directly_addressed` →
//!   [`WorkItemKind::Mention`], others → [`WorkItemKind::Notification`].
//!
//! Requests stay on the configured origin (redirects elsewhere are refused, the header would follow them).

use focus_core::api::{WorkItem, WorkItemKind, WorkSource};
use reqwest::header::ACCEPT;
use serde::Deserialize;

use super::{
    MAX_REDIRECTS, Token, WorkFetch, WorkProvider, build_url, due_end_of_day, https_base, item_id,
    new_item, non_empty, parse_each, parse_instant, parse_json, priority_from_labels, send,
    status_from_labels, today_within,
};
use crate::PlatformError;
use crate::calendar::TimeZone;
use crate::net::LazyClient;

const SERVICE: &str = "GitLab";
/// Merge requests waiting for review whose reviewer state is checked (one request each).
pub const MAX_REVIEW_CHECKS: usize = 10;
/// Own open merge requests checked for changes requested / a failed pipeline (one request each).
pub const MAX_OWN_MRS: usize = 10;
/// GitLab's iteration state "current".
const ITERATION_CURRENT: u64 = 2;
/// Reviewer states that mean the user already did their part.
const REVIEW_DONE: &[&str] = &["reviewed", "requested_changes", "approved"];

/// GitLab client.
#[derive(Debug)]
pub struct GitlabClient {
    base: String,
    token: Token,
    http: LazyClient,
}

impl GitlabClient {
    /// `base_url` = the instance (`https://gitlab.com`, `https://gitlab.example.com/gitlab`); empty = gitlab.com.
    /// No I/O.
    pub fn new(base_url: &str, token: String) -> Result<Self, PlatformError> {
        let base_url = if base_url.trim().is_empty() {
            focus_core::config::DEFAULT_GITLAB_URL
        } else {
            base_url
        };
        Ok(Self {
            base: https_base(base_url, "GitLab")?,
            token: Token::new(token),
            http: LazyClient::same_origin(MAX_REDIRECTS),
        })
    }

    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<String, PlatformError> {
        let url = build_url(&self.base, &format!("/api/v4{path}"), query)?;
        let request = self
            .http
            .get()?
            .get(url)
            .header(ACCEPT, "application/json")
            .header("PRIVATE-TOKEN", self.token.header("")?);
        send(request)?.into_body(SERVICE)
    }

    fn get_list<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<T>, PlatformError> {
        let values: Vec<serde_json::Value> = parse_json(&self.get(path, query)?, SERVICE)?;
        Ok(parse_each(values))
    }

    /// Review requests the user has not acted on yet. A reviewer list that cannot be read keeps the request.
    fn review_items(
        &self,
        user_id: u64,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<WorkItem>, PlatformError> {
        let id = user_id.to_string();
        let requests: Vec<GlMergeRequest> = self.get_list(
            "/merge_requests",
            &[
                ("scope", "all"),
                ("reviewer_id", id.as_str()),
                ("state", "opened"),
                ("order_by", "updated_at"),
                ("per_page", "50"),
            ],
        )?;
        let mut failures = 0;
        let mut items = Vec::new();
        for (index, mr) in requests.iter().enumerate() {
            if index < MAX_REVIEW_CHECKS && mr.project_id > 0 && mr.iid > 0 {
                let path = format!(
                    "/projects/{}/merge_requests/{}/reviewers",
                    mr.project_id, mr.iid
                );
                match self.get_list::<GlReviewer>(&path, &[]) {
                    Ok(reviewers) if review_done(&reviewers, user_id) => continue,
                    Ok(_) => {}
                    Err(err) => {
                        failures += 1;
                        log::debug!("gitlab: reading reviewers failed: {err}");
                    }
                }
            }
            items.extend(mr_item(mr, WorkItemKind::ReviewRequested));
        }
        if failures > 0 {
            warnings.push(format!(
                "GitLab: the review state of {failures} merge requests could not be read"
            ));
        }
        Ok(items)
    }

    /// Own merge requests with changes requested or a failed pipeline.
    fn own_items(&self, warnings: &mut Vec<String>) -> Result<Vec<WorkItem>, PlatformError> {
        let per_page = MAX_OWN_MRS.to_string();
        let own: Vec<GlMergeRequest> = self.get_list(
            "/merge_requests",
            &[
                ("scope", "created_by_me"),
                ("state", "opened"),
                ("order_by", "updated_at"),
                ("per_page", per_page.as_str()),
            ],
        )?;
        let mut failures = 0;
        let mut items = Vec::new();
        for mr in own.iter().take(MAX_OWN_MRS) {
            if mr.detailed_merge_status.as_deref() == Some("requested_changes") {
                items.extend(mr_item(mr, WorkItemKind::ChangesRequested));
                continue;
            }
            if mr.project_id == 0 || mr.iid == 0 {
                continue;
            }
            let path = format!("/projects/{}/merge_requests/{}", mr.project_id, mr.iid);
            match self
                .get(&path, &[])
                .and_then(|body| parse_json::<GlMergeRequest>(&body, SERVICE))
            {
                Ok(detail) if pipeline_failed(&detail) => {
                    items.extend(mr_item(&detail, WorkItemKind::CiFailed));
                }
                Ok(_) => {}
                Err(err) => {
                    failures += 1;
                    log::debug!("gitlab: reading a merge request failed: {err}");
                }
            }
        }
        if failures > 0 {
            warnings.push(format!(
                "GitLab: {failures} of your merge requests could not be checked for pipelines"
            ));
        }
        Ok(items)
    }
}

impl WorkProvider for GitlabClient {
    fn source(&self) -> WorkSource {
        WorkSource::Gitlab
    }

    fn fetch(&mut self, now_ms: i64) -> Result<WorkFetch, PlatformError> {
        let tz = TimeZone::local();
        let user: GlUser = parse_json(&self.get("/user", &[])?, SERVICE)?;
        let issues: Vec<GlIssue> = self.get_list(
            "/issues",
            &[
                ("scope", "assigned_to_me"),
                ("state", "opened"),
                ("order_by", "updated_at"),
                ("per_page", "50"),
            ],
        )?;
        let mut items: Vec<WorkItem> = issues
            .iter()
            .filter_map(|issue| issue_item(issue, now_ms, &tz))
            .collect();
        let mut warnings = Vec::new();
        if user.id == 0 {
            warnings.push("GitLab review requests: the user id is unknown".into());
        } else {
            match self.review_items(user.id, &mut warnings) {
                Ok(reviews) => items.extend(reviews),
                Err(err) => warnings.push(format!("GitLab review requests: {err}")),
            }
        }
        match self.own_items(&mut warnings) {
            Ok(own) => items.extend(own),
            Err(err) => warnings.push(format!("GitLab own merge requests: {err}")),
        }
        match self.get_list::<GlTodo>("/todos", &[("state", "pending"), ("per_page", "50")]) {
            Ok(todos) => items.extend(todos.iter().filter_map(todo_item)),
            Err(err) => warnings.push(format!("GitLab to-do items: {err}")),
        }
        Ok(WorkFetch {
            account: non_empty(user.username.as_deref())
                .or_else(|| non_empty(user.name.as_deref())),
            items,
            poll_interval_secs: None,
            warnings,
        })
    }
}

// -- responses -----------------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlUser {
    id: u64,
    username: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlReferences {
    full: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlIteration {
    /// 1 upcoming, 2 current, 3 closed (some versions send a name).
    state: serde_json::Value,
    start_date: Option<String>,
    due_date: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlIssue {
    iid: u64,
    title: Option<String>,
    web_url: Option<String>,
    references: GlReferences,
    labels: Vec<String>,
    due_date: Option<String>,
    updated_at: Option<String>,
    iteration: Option<GlIteration>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlPipeline {
    status: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlMergeRequest {
    iid: u64,
    project_id: u64,
    title: Option<String>,
    web_url: Option<String>,
    references: GlReferences,
    labels: Vec<String>,
    updated_at: Option<String>,
    detailed_merge_status: Option<String>,
    head_pipeline: Option<GlPipeline>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlReviewer {
    user: GlUser,
    state: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlProject {
    path_with_namespace: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlTarget {
    iid: Option<u64>,
    title: Option<String>,
    references: Option<GlReferences>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GlTodo {
    id: u64,
    project: Option<GlProject>,
    action_name: Option<String>,
    target_type: Option<String>,
    target: Option<GlTarget>,
    target_url: Option<String>,
    body: Option<String>,
    updated_at: Option<String>,
    created_at: Option<String>,
}

// -- mapping (pure) -------------------------------------------------------------------------------------------

/// `group/sub/project` of a full reference (`group/sub/project#12`, `group/project!34`).
fn project_of(reference: &str) -> Option<String> {
    let cut = reference.rfind(['#', '!'])?;
    non_empty(Some(&reference[..cut]))
}

fn updated(text: Option<&str>) -> i64 {
    text.and_then(parse_instant).unwrap_or(0)
}

fn issue_item(issue: &GlIssue, now_ms: i64, tz: &TimeZone) -> Option<WorkItem> {
    let key = non_empty(issue.references.full.as_deref())?;
    let mut item = new_item(
        WorkSource::Gitlab,
        WorkItemKind::Assigned,
        key.clone(),
        issue.title.clone().unwrap_or_default(),
        issue.web_url.clone().unwrap_or_default(),
        updated(issue.updated_at.as_deref()),
    );
    item.project = project_of(&key);
    let labels = issue.labels.iter().map(String::as_str);
    if let Some((label, rank)) = priority_from_labels(labels.clone()) {
        item.priority_label = Some(label);
        item.priority_rank = Some(rank);
    }
    item.status = status_from_labels(labels);
    item.due_ms = issue
        .due_date
        .as_deref()
        .and_then(|date| due_end_of_day(date, tz));
    item.in_sprint = issue
        .iteration
        .as_ref()
        .is_some_and(|it| iteration_is_current(it, now_ms, tz));
    Some(item)
}

fn iteration_is_current(iteration: &GlIteration, now_ms: i64, tz: &TimeZone) -> bool {
    let by_state = match &iteration.state {
        serde_json::Value::Number(n) => n.as_u64() == Some(ITERATION_CURRENT),
        serde_json::Value::String(s) => matches!(s.as_str(), "current" | "started"),
        _ => false,
    };
    by_state
        || today_within(
            iteration.start_date.as_deref(),
            iteration.due_date.as_deref(),
            now_ms,
            tz,
        )
}

fn mr_item(mr: &GlMergeRequest, kind: WorkItemKind) -> Option<WorkItem> {
    let key = non_empty(mr.references.full.as_deref())?;
    let mut item = new_item(
        WorkSource::Gitlab,
        kind,
        key.clone(),
        mr.title.clone().unwrap_or_default(),
        mr.web_url.clone().unwrap_or_default(),
        updated(mr.updated_at.as_deref()),
    );
    item.project = project_of(&key);
    if let Some((label, rank)) = priority_from_labels(mr.labels.iter().map(String::as_str)) {
        item.priority_label = Some(label);
        item.priority_rank = Some(rank);
    }
    Some(item)
}

/// The user (by id) is a reviewer who already reviewed, requested changes or approved.
fn review_done(reviewers: &[GlReviewer], user_id: u64) -> bool {
    reviewers.iter().any(|reviewer| {
        reviewer.user.id == user_id
            && reviewer
                .state
                .as_deref()
                .is_some_and(|state| REVIEW_DONE.contains(&state))
    })
}

fn pipeline_failed(mr: &GlMergeRequest) -> bool {
    mr.head_pipeline.as_ref().and_then(|p| p.status.as_deref()) == Some("failed")
}

/// A pending to-do item. Issues and merge requests use their full reference as the key (so they merge with the
/// lists above); other targets get an id of their own.
fn todo_item(todo: &GlTodo) -> Option<WorkItem> {
    let kind = match todo.action_name.as_deref() {
        Some("mentioned" | "directly_addressed") => WorkItemKind::Mention,
        _ => WorkItemKind::Notification,
    };
    let project = todo
        .project
        .as_ref()
        .and_then(|p| non_empty(p.path_with_namespace.as_deref()));
    let target = todo.target.as_ref();
    let sigil = match todo.target_type.as_deref() {
        Some("Issue" | "WorkItem") => Some('#'),
        Some("MergeRequest") => Some('!'),
        _ => None,
    };
    let reference = target
        .and_then(|t| t.references.as_ref())
        .and_then(|r| non_empty(r.full.as_deref()))
        .or_else(|| match (&project, sigil, target.and_then(|t| t.iid)) {
            (Some(path), Some(sigil), Some(iid)) => Some(format!("{path}{sigil}{iid}")),
            _ => None,
        });
    let title = target
        .and_then(|t| non_empty(t.title.as_deref()))
        .or_else(|| non_empty(todo.body.as_deref()))
        .unwrap_or_default();
    let url = todo.target_url.clone().unwrap_or_default();
    let updated_ms = updated(todo.updated_at.as_deref().or(todo.created_at.as_deref()));
    let mut item = match (sigil, reference) {
        (Some(_), Some(key)) => new_item(WorkSource::Gitlab, kind, key, title, url, updated_ms),
        _ => {
            let key = project.clone().unwrap_or_else(|| "GitLab".to_owned());
            let mut item = new_item(WorkSource::Gitlab, kind, key, title, url, updated_ms);
            item.id = item_id(WorkSource::Gitlab, &format!("todo-{}", todo.id));
            item
        }
    };
    item.project = project.or_else(|| project_of(&item.key));
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::test_support::{cest, end_of_day, utc};

    /// Trimmed `GET /issues?scope=assigned_to_me` response.
    const ISSUES: &str = r##"[
      {
        "id": 76,
        "iid": 6,
        "project_id": 8,
        "title": "Consequatur vero maxime deserunt laboriosam est voluptas dolorem.",
        "description": "Ratione dolores corrupti mollitia soluta quia.",
        "state": "opened",
        "created_at": "2026-09-22T13:11:26.718Z",
        "updated_at": "2026-10-12T11:58:33.452Z",
        "closed_at": null,
        "labels": ["foo", "priority::1", "workflow::in progress"],
        "milestone": {"id": 1, "iid": 1, "project_id": 8, "title": "v1.0", "due_date": "2026-11-01", "state": "active"},
        "assignees": [{"id": 9, "username": "me", "name": "Me"}],
        "author": {"id": 1, "username": "root"},
        "type": "ISSUE",
        "issue_type": "issue",
        "due_date": "2026-10-14",
        "web_url": "https://gitlab.example.com/acme/backend/-/issues/6",
        "references": {"short": "#6", "relative": "#6", "full": "acme/backend#6"},
        "iteration": {
          "id": 90, "iid": 4, "sequence": 2, "group_id": 162,
          "title": null, "description": null,
          "state": 2,
          "created_at": "2026-09-01T09:00:00.000Z",
          "updated_at": "2026-09-01T09:00:00.000Z",
          "start_date": "2026-10-06",
          "due_date": "2026-10-19",
          "web_url": "https://gitlab.example.com/groups/acme/-/iterations/90"
        },
        "weight": null
      },
      {
        "iid": 9,
        "title": "Update docs",
        "labels": [],
        "milestone": null,
        "due_date": null,
        "updated_at": "2026-10-01T08:00:00Z",
        "web_url": "https://gitlab.example.com/acme/docs/-/issues/9",
        "references": {"short": "#9", "relative": "#9", "full": "acme/docs#9"},
        "iteration": {"state": 1, "start_date": "2026-10-20", "due_date": "2026-11-02"}
      },
      {"iid": 10, "title": "no references"}
    ]"##;

    const MERGE_REQUESTS: &str = r##"[
      {
        "id": 101,
        "iid": 34,
        "project_id": 8,
        "title": "Draft: Add caching layer",
        "state": "opened",
        "draft": true,
        "updated_at": "2026-10-10T07:30:00.000Z",
        "labels": ["P2"],
        "detailed_merge_status": "requested_changes",
        "web_url": "https://gitlab.example.com/acme/backend/-/merge_requests/34",
        "references": {"short": "!34", "relative": "!34", "full": "acme/backend!34"},
        "reviewers": [{"id": 12, "username": "alice"}]
      },
      {
        "iid": 35,
        "project_id": 8,
        "title": "Fix flaky test",
        "updated_at": "2026-10-11T07:30:00.000Z",
        "labels": [],
        "detailed_merge_status": "ci_must_pass",
        "web_url": "https://gitlab.example.com/acme/backend/-/merge_requests/35",
        "references": {"full": "acme/backend!35"},
        "head_pipeline": {"id": 77, "status": "failed", "ref": "fix-flaky", "sha": "a1b2c3"}
      }
    ]"##;

    const REVIEWERS: &str = r##"[
      {"user": {"id": 9, "username": "me", "name": "Me", "state": "active"}, "state": "approved", "created_at": "2026-10-10T07:30:00Z"},
      {"user": {"id": 12, "username": "alice", "name": "Alice", "state": "active"}, "state": "unreviewed", "created_at": "2026-10-10T07:30:00Z"}
    ]"##;

    const TODOS: &str = r##"[
      {
        "id": 102,
        "project": {"id": 8, "name": "backend", "path_with_namespace": "acme/backend"},
        "author": {"id": 12, "username": "alice"},
        "action_name": "mentioned",
        "target_type": "Issue",
        "target": {"id": 76, "iid": 6, "project_id": 8, "title": "Consequatur vero", "state": "opened",
                   "references": {"full": "acme/backend#6"}},
        "target_url": "https://gitlab.example.com/acme/backend/-/issues/6#note_1",
        "body": "@me can you look?",
        "state": "pending",
        "created_at": "2026-10-12T10:00:00.000Z",
        "updated_at": "2026-10-12T10:05:00.000Z"
      },
      {
        "id": 103,
        "project": {"path_with_namespace": "acme/web"},
        "action_name": "build_failed",
        "target_type": "MergeRequest",
        "target": {"iid": 3, "title": "Bump deps", "state": "opened"},
        "target_url": "https://gitlab.example.com/acme/web/-/merge_requests/3",
        "state": "pending",
        "created_at": "2026-10-11T10:00:00.000Z",
        "updated_at": null
      },
      {
        "id": 104,
        "project": null,
        "action_name": "member_access_requested",
        "target_type": "Namespace",
        "target": null,
        "target_url": "https://gitlab.example.com/groups/acme/-/group_members",
        "body": "bob requested access",
        "state": "pending",
        "created_at": "2026-10-11T09:00:00.000Z",
        "updated_at": "2026-10-11T09:00:00.000Z"
      }
    ]"##;

    fn parse<T: serde::de::DeserializeOwned>(body: &str) -> Vec<T> {
        parse_each(parse_json::<Vec<serde_json::Value>>(body, SERVICE).expect("json"))
    }

    #[test]
    fn assigned_issues_map_with_iteration_labels_and_due_date() {
        let tz = cest();
        let now = utc(2026, 10, 13, 9, 0);
        let issues: Vec<GlIssue> = parse(ISSUES);
        let items: Vec<WorkItem> = issues
            .iter()
            .filter_map(|i| issue_item(i, now, &tz))
            .collect();
        assert_eq!(items.len(), 2, "issues without a reference are skipped");
        let first = &items[0];
        assert_eq!(first.id, "gitlab:acme/backend#6");
        assert_eq!(first.kind, WorkItemKind::Assigned);
        assert_eq!(
            first.url,
            "https://gitlab.example.com/acme/backend/-/issues/6"
        );
        assert_eq!(first.project.as_deref(), Some("acme/backend"));
        assert_eq!(first.priority_label.as_deref(), Some("priority::1"));
        assert_eq!(first.priority_rank, Some(1));
        assert_eq!(first.status.as_deref(), Some("workflow::in progress"));
        assert_eq!(first.due_ms, Some(end_of_day(2026, 10, 14, &tz)));
        assert!(first.in_sprint);
        assert_eq!(first.updated_ms, utc(2026, 10, 12, 11, 58) + 33_452);

        let second = &items[1];
        assert_eq!(second.key, "acme/docs#9");
        assert!(!second.in_sprint, "upcoming iteration");
        assert_eq!((second.due_ms, second.priority_rank), (None, None));
    }

    #[test]
    fn iteration_by_dates_when_state_is_missing() {
        let tz = cest();
        let iteration = GlIteration {
            state: serde_json::Value::Null,
            start_date: Some("2026-10-06".into()),
            due_date: Some("2026-10-19".into()),
        };
        assert!(iteration_is_current(
            &iteration,
            utc(2026, 10, 13, 9, 0),
            &tz
        ));
        assert!(!iteration_is_current(
            &iteration,
            utc(2026, 10, 25, 9, 0),
            &tz
        ));
        let named = GlIteration {
            state: serde_json::json!("current"),
            ..GlIteration::default()
        };
        assert!(iteration_is_current(&named, 0, &tz));
    }

    #[test]
    fn merge_requests_map_and_pipelines_fail() {
        let mrs: Vec<GlMergeRequest> = parse(MERGE_REQUESTS);
        let item = mr_item(&mrs[0], WorkItemKind::ChangesRequested).expect("item");
        assert_eq!(item.id, "gitlab:acme/backend!34");
        assert_eq!(item.key, "acme/backend!34");
        assert_eq!(item.project.as_deref(), Some("acme/backend"));
        assert_eq!(item.priority_rank, Some(2));
        assert_eq!(
            item.url,
            "https://gitlab.example.com/acme/backend/-/merge_requests/34"
        );
        assert_eq!(
            mrs[0].detailed_merge_status.as_deref(),
            Some("requested_changes")
        );
        assert!(!pipeline_failed(&mrs[0]));
        assert!(pipeline_failed(&mrs[1]));
        assert_eq!((mrs[1].project_id, mrs[1].iid), (8, 35));
    }

    #[test]
    fn reviewed_merge_requests_are_dropped() {
        let reviewers: Vec<GlReviewer> = parse(REVIEWERS);
        assert!(review_done(&reviewers, 9), "approved by me");
        assert!(!review_done(&reviewers, 12), "alice has not reviewed");
        assert!(!review_done(&reviewers, 99), "not a reviewer");
        let older: Vec<GlReviewer> = parse(r##"[{"user": {"id": 9, "username": "me"}}]"##);
        assert!(
            !review_done(&older, 9),
            "no state (older GitLab) keeps the request"
        );
    }

    #[test]
    fn todos_map_to_mentions_and_notifications() {
        let todos: Vec<GlTodo> = parse(TODOS);
        let items: Vec<WorkItem> = todos.iter().filter_map(todo_item).collect();
        assert_eq!(items.len(), 3);

        assert_eq!(
            items[0].id, "gitlab:acme/backend#6",
            "merges with the assigned issue"
        );
        assert_eq!(items[0].kind, WorkItemKind::Mention);
        assert_eq!(items[0].title, "Consequatur vero");
        assert_eq!(items[0].updated_ms, utc(2026, 10, 12, 10, 5));

        assert_eq!(items[1].id, "gitlab:acme/web!3");
        assert_eq!(items[1].kind, WorkItemKind::Notification);
        assert_eq!(items[1].project.as_deref(), Some("acme/web"));
        assert_eq!(
            items[1].updated_ms,
            utc(2026, 10, 11, 10, 0),
            "falls back to created_at"
        );

        assert_eq!(items[2].id, "gitlab:todo-104");
        assert_eq!(items[2].title, "bob requested access");
        assert_eq!(items[2].project, None);
    }

    #[test]
    fn base_url_defaults_to_gitlab_com() {
        let client = GitlabClient::new("", "glpat-x".into()).expect("client");
        assert_eq!(client.base, "https://gitlab.com");
        let own = GitlabClient::new("https://gitlab.example.com/gitlab/", "glpat-x".into())
            .expect("client");
        assert_eq!(own.base, "https://gitlab.example.com/gitlab");
        assert!(GitlabClient::new("http://gitlab.example.com", "glpat-x".into()).is_err());
        assert!(!format!("{own:?}").contains("glpat-x"));
        assert_eq!(
            project_of("group/sub/proj#12").as_deref(),
            Some("group/sub/proj")
        );
        assert_eq!(project_of("#12"), None);
    }
}
