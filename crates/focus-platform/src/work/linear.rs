//! Linear, read-only, through the GraphQL API (`https://api.linear.app/graphql`) with a personal API key
//! (`lin_api_…`, read access is enough) sent as `Authorization: <key>` (personal keys take no `Bearer` prefix).
//!
//! One query ([`QUERY`]) reads:
//! * the viewer's name and their assigned issues whose state type is not completed / canceled / duplicate (at most
//!   50, most recently updated first) → [`WorkItemKind::Assigned`] with priority (1 Urgent ..= 4 Low, 0 none),
//!   `dueDate` (a date) and "in sprint" when the issue's cycle is active;
//! * the viewer's latest notifications (unread, not snoozed, not archived): category `mentions` →
//!   [`WorkItemKind::Mention`], others → [`WorkItemKind::Notification`]; issue notifications use the issue
//!   identifier as the key (so they merge with the assigned issue).
//!
//! GraphQL errors arrive in `errors` (often with HTTP 200 or 400): `AUTHENTICATION_ERROR` / `FORBIDDEN` →
//! [`PlatformError::Auth`], `RATELIMITED` → a rate-limit error; when `viewer` still came back, other errors only
//! become warnings.

use focus_core::api::{WorkItem, WorkItemKind, WorkSource};
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    AUTH_ERROR, MAX_REDIRECTS, Token, WorkFetch, WorkProvider, due_end_of_day, item_id, new_item,
    non_empty, parse_instant, parse_json, send, status_error,
};
use crate::PlatformError;
use crate::calendar::TimeZone;
use crate::net::LazyClient;

const ENDPOINT: &str = "https://api.linear.app/graphql";
const SERVICE: &str = "Linear";

/// The whole read (one request).
pub const QUERY: &str = r#"query FocusModeWork {
  viewer {
    name
    displayName
    assignedIssues(
      first: 50
      orderBy: updatedAt
      filter: { state: { type: { nin: ["completed", "canceled", "duplicate"] } } }
    ) {
      nodes {
        identifier
        title
        url
        priority
        priorityLabel
        dueDate
        updatedAt
        state { name type }
        cycle { startsAt endsAt isActive }
        team { key name }
        project { name }
      }
    }
  }
  notifications(first: 30) {
    nodes {
      id
      category
      title
      url
      readAt
      snoozedUntilAt
      archivedAt
      updatedAt
      ... on IssueNotification {
        issue { identifier title url team { name } }
      }
    }
  }
}"#;

/// Linear client.
#[derive(Debug)]
pub struct LinearClient {
    token: Token,
    http: LazyClient,
}

impl LinearClient {
    /// No I/O.
    pub fn new(token: String) -> Self {
        Self {
            token: Token::new(token),
            http: LazyClient::same_origin(MAX_REDIRECTS),
        }
    }
}

impl WorkProvider for LinearClient {
    fn source(&self) -> WorkSource {
        WorkSource::Linear
    }

    fn fetch(&mut self, now_ms: i64) -> Result<WorkFetch, PlatformError> {
        let tz = TimeZone::local();
        let request = self
            .http
            .get()?
            .post(ENDPOINT)
            .header(ACCEPT, "application/json")
            .header(AUTHORIZATION, self.token.header("")?)
            .json(&json!({ "query": QUERY }));
        let reply = send(request)?;
        let response = parse_response(reply.status, &reply.body, &reply.headers)?;
        Ok(map_response(response, now_ms, &tz))
    }
}

// -- responses -----------------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GraphResponse {
    data: Option<GraphData>,
    errors: Vec<GraphError>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GraphError {
    message: Option<String>,
    extensions: Value,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GraphData {
    viewer: Option<Viewer>,
    notifications: Option<Connection>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Viewer {
    name: Option<String>,
    display_name: Option<String>,
    assigned_issues: Option<Connection>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Connection {
    nodes: Vec<Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Issue {
    identifier: Option<String>,
    title: Option<String>,
    url: Option<String>,
    priority: Option<f64>,
    priority_label: Option<String>,
    due_date: Option<String>,
    updated_at: Option<String>,
    state: Option<State>,
    cycle: Option<Cycle>,
    team: Option<Named>,
    project: Option<Named>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct State {
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Cycle {
    starts_at: Option<String>,
    ends_at: Option<String>,
    is_active: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Named {
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Notification {
    id: Option<String>,
    category: Option<String>,
    title: Option<String>,
    url: Option<String>,
    read_at: Option<String>,
    snoozed_until_at: Option<String>,
    archived_at: Option<String>,
    updated_at: Option<String>,
    issue: Option<Issue>,
}

/// The successful part of a response plus the non-fatal errors.
#[derive(Debug, Default)]
struct Parsed {
    viewer: Viewer,
    notifications: Vec<Value>,
    warnings: Vec<String>,
}

// -- mapping (pure) -------------------------------------------------------------------------------------------

fn error_code(error: &GraphError) -> String {
    error
        .extensions
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase()
}

/// Status + body → the data, or the error that makes it unusable.
fn parse_response(
    status: u16,
    body: &str,
    headers: &reqwest::header::HeaderMap,
) -> Result<Parsed, PlatformError> {
    if matches!(status, 401 | 403) {
        return Err(PlatformError::Auth(AUTH_ERROR.into()));
    }
    let parsed: Result<GraphResponse, PlatformError> = parse_json(body, SERVICE);
    let response = match parsed {
        Ok(response) => response,
        Err(err) if (200..300).contains(&status) => return Err(err),
        Err(_) => return Err(status_error(status, headers, SERVICE)),
    };
    let codes: Vec<String> = response.errors.iter().map(error_code).collect();
    if codes
        .iter()
        .any(|c| c == "AUTHENTICATION_ERROR" || c == "FORBIDDEN" || c == "UNAUTHENTICATED")
    {
        return Err(PlatformError::Auth(AUTH_ERROR.into()));
    }
    if codes.iter().any(|c| c == "RATELIMITED") || status == 429 {
        return Err(PlatformError::Network(
            "Linear: rate limited, try again later".into(),
        ));
    }
    let data = response.data.unwrap_or_default();
    let Some(viewer) = data.viewer else {
        let first = response
            .errors
            .first()
            .map(|e| match non_empty(Some(&error_code(e))) {
                Some(code) => code,
                None => "request failed".to_owned(),
            })
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(PlatformError::Network(format!("{SERVICE}: {first}")));
    };
    let warnings = response
        .errors
        .iter()
        .map(|e| {
            let code = error_code(e);
            let message = e
                .message
                .as_deref()
                .map(|m| m.lines().next().unwrap_or(""))
                .unwrap_or("");
            let message: String = message.chars().take(160).collect();
            format!(
                "Linear: {} {message}",
                if code.is_empty() { "error" } else { &code }
            )
            .trim_end()
            .to_owned()
        })
        .collect();
    Ok(Parsed {
        viewer,
        notifications: data.notifications.map(|c| c.nodes).unwrap_or_default(),
        warnings,
    })
}

fn map_response(parsed: Parsed, now_ms: i64, tz: &TimeZone) -> WorkFetch {
    let Parsed {
        viewer,
        notifications,
        warnings,
    } = parsed;
    let mut items: Vec<WorkItem> = viewer
        .assigned_issues
        .map(|c| c.nodes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|node| serde_json::from_value::<Issue>(node).ok())
        .filter_map(|issue| issue_item(&issue, now_ms, tz))
        .collect();
    items.extend(
        notifications
            .into_iter()
            .filter_map(|node| serde_json::from_value::<Notification>(node).ok())
            .filter_map(|n| notification_item(&n, now_ms)),
    );
    WorkFetch {
        account: non_empty(viewer.display_name.as_deref())
            .or_else(|| non_empty(viewer.name.as_deref())),
        items,
        poll_interval_secs: None,
        warnings,
    }
}

/// Linear priority (1 Urgent, 2 High, 3 Medium, 4 Low; 0 none) → rank 0 ..= 3.
fn priority_rank(priority: Option<f64>) -> Option<u32> {
    let value = priority?;
    if !(1.0..=4.0).contains(&value) {
        return None;
    }
    Some((value.round() as u32).saturating_sub(1))
}

fn cycle_is_active(cycle: &Cycle, now_ms: i64) -> bool {
    if let Some(active) = cycle.is_active {
        return active;
    }
    match (
        cycle.starts_at.as_deref().and_then(parse_instant),
        cycle.ends_at.as_deref().and_then(parse_instant),
    ) {
        (Some(start), Some(end)) => start <= now_ms && now_ms < end,
        _ => false,
    }
}

fn issue_item(issue: &Issue, now_ms: i64, tz: &TimeZone) -> Option<WorkItem> {
    let key = non_empty(issue.identifier.as_deref())?;
    let mut item = new_item(
        WorkSource::Linear,
        WorkItemKind::Assigned,
        key,
        issue.title.clone().unwrap_or_default(),
        issue.url.clone().unwrap_or_default(),
        issue
            .updated_at
            .as_deref()
            .and_then(parse_instant)
            .unwrap_or(0),
    );
    item.priority_rank = priority_rank(issue.priority);
    if item.priority_rank.is_some() {
        item.priority_label = non_empty(issue.priority_label.as_deref());
    }
    item.due_ms = issue
        .due_date
        .as_deref()
        .and_then(|date| due_end_of_day(date, tz));
    item.status = issue
        .state
        .as_ref()
        .and_then(|s| non_empty(s.name.as_deref()));
    item.in_sprint = issue
        .cycle
        .as_ref()
        .is_some_and(|cycle| cycle_is_active(cycle, now_ms));
    item.project = issue
        .project
        .as_ref()
        .and_then(|p| non_empty(p.name.as_deref()))
        .or_else(|| {
            issue
                .team
                .as_ref()
                .and_then(|t| non_empty(t.name.as_deref()))
        });
    Some(item)
}

fn notification_item(notification: &Notification, now_ms: i64) -> Option<WorkItem> {
    if notification.read_at.is_some() || notification.archived_at.is_some() {
        return None;
    }
    if notification
        .snoozed_until_at
        .as_deref()
        .and_then(parse_instant)
        .is_some_and(|until| until > now_ms)
    {
        return None;
    }
    let kind = if notification.category.as_deref() == Some("mentions") {
        WorkItemKind::Mention
    } else {
        WorkItemKind::Notification
    };
    let updated = notification
        .updated_at
        .as_deref()
        .and_then(parse_instant)
        .unwrap_or(0);
    let issue = notification.issue.as_ref();
    match issue.and_then(|i| non_empty(i.identifier.as_deref())) {
        Some(key) => {
            let issue = issue?;
            let mut item = new_item(
                WorkSource::Linear,
                kind,
                key,
                non_empty(issue.title.as_deref())
                    .or_else(|| non_empty(notification.title.as_deref()))
                    .unwrap_or_default(),
                non_empty(issue.url.as_deref())
                    .or_else(|| non_empty(notification.url.as_deref()))
                    .unwrap_or_default(),
                updated,
            );
            item.project = issue
                .team
                .as_ref()
                .and_then(|t| non_empty(t.name.as_deref()));
            Some(item)
        }
        None => {
            let id = non_empty(notification.id.as_deref())?;
            let mut item = new_item(
                WorkSource::Linear,
                kind,
                "Linear".to_owned(),
                notification.title.clone().unwrap_or_default(),
                notification.url.clone().unwrap_or_default(),
                updated,
            );
            item.id = item_id(WorkSource::Linear, &format!("notification-{id}"));
            Some(item)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::test_support::{cest, end_of_day, utc};
    use reqwest::header::HeaderMap;

    const RESPONSE: &str = r#"{
      "data": {
        "viewer": {
          "name": "Jan Kowalski",
          "displayName": "jan",
          "assignedIssues": {
            "nodes": [
              {
                "identifier": "ENG-7",
                "title": "Crash on launch for new users",
                "url": "https://linear.app/firma/issue/ENG-7/crash-on-launch-for-new-users",
                "priority": 1,
                "priorityLabel": "Urgent",
                "dueDate": "2026-10-13",
                "updatedAt": "2026-10-12T16:45:00.000Z",
                "state": {"name": "In Progress", "type": "started"},
                "cycle": {"startsAt": "2026-10-06T00:00:00.000Z", "endsAt": "2026-10-20T00:00:00.000Z", "isActive": true},
                "team": {"key": "ENG", "name": "Engineering"},
                "project": {"name": "Onboarding v2"}
              },
              {
                "identifier": "ENG-12",
                "title": "Tidy up feature flags",
                "url": "https://linear.app/firma/issue/ENG-12/tidy-up-feature-flags",
                "priority": 0,
                "priorityLabel": "No priority",
                "dueDate": null,
                "updatedAt": "2026-09-30T08:00:00.000Z",
                "state": {"name": "Todo", "type": "unstarted"},
                "cycle": {"startsAt": "2026-10-20T00:00:00.000Z", "endsAt": "2026-11-03T00:00:00.000Z"},
                "team": {"key": "ENG", "name": "Engineering"},
                "project": null
              },
              {"title": "no identifier"}
            ]
          }
        },
        "notifications": {
          "nodes": [
            {
              "id": "6c2f5a3e-1111-4c1a-9b7e-5f0f6f2a0001",
              "category": "mentions",
              "title": "Anna mentioned you in ENG-7",
              "url": "https://linear.app/firma/issue/ENG-7#comment-1",
              "readAt": null,
              "snoozedUntilAt": null,
              "archivedAt": null,
              "updatedAt": "2026-10-13T07:00:00.000Z",
              "issue": {"identifier": "ENG-7", "title": "Crash on launch for new users",
                        "url": "https://linear.app/firma/issue/ENG-7/crash-on-launch-for-new-users",
                        "team": {"name": "Engineering"}}
            },
            {
              "id": "6c2f5a3e-1111-4c1a-9b7e-5f0f6f2a0002",
              "category": "statusChanges",
              "title": "ENG-30 moved to Done",
              "url": "https://linear.app/firma/issue/ENG-30",
              "readAt": "2026-10-12T07:00:00.000Z",
              "updatedAt": "2026-10-12T06:00:00.000Z",
              "issue": {"identifier": "ENG-30", "title": "Read already"}
            },
            {
              "id": "6c2f5a3e-1111-4c1a-9b7e-5f0f6f2a0003",
              "category": "reminders",
              "title": "Snoozed",
              "url": "https://linear.app/firma/issue/ENG-31",
              "readAt": null,
              "snoozedUntilAt": "2026-10-14T08:00:00.000Z",
              "updatedAt": "2026-10-12T06:00:00.000Z",
              "issue": {"identifier": "ENG-31", "title": "Later"}
            },
            {
              "id": "6c2f5a3e-1111-4c1a-9b7e-5f0f6f2a0004",
              "category": "postsAndUpdates",
              "title": "Project update: Onboarding v2",
              "url": "https://linear.app/firma/project/onboarding-v2/updates",
              "readAt": null,
              "updatedAt": "2026-10-12T09:00:00.000Z"
            }
          ]
        }
      }
    }"#;

    fn now() -> i64 {
        utc(2026, 10, 13, 9, 0)
    }

    #[test]
    fn assigned_issues_and_notifications_map() {
        let tz = cest();
        let parsed = parse_response(200, RESPONSE, &HeaderMap::new()).expect("parsed");
        let fetch = map_response(parsed, now(), &tz);
        assert_eq!(fetch.account.as_deref(), Some("jan"));
        assert!(fetch.warnings.is_empty());
        assert_eq!(fetch.items.len(), 4);

        let first = &fetch.items[0];
        assert_eq!(first.id, "linear:ENG-7");
        assert_eq!(first.kind, WorkItemKind::Assigned);
        assert_eq!(first.priority_rank, Some(0));
        assert_eq!(first.priority_label.as_deref(), Some("Urgent"));
        assert_eq!(first.due_ms, Some(end_of_day(2026, 10, 13, &tz)));
        assert_eq!(first.status.as_deref(), Some("In Progress"));
        assert!(first.in_sprint);
        assert_eq!(first.project.as_deref(), Some("Onboarding v2"));
        assert_eq!(first.updated_ms, utc(2026, 10, 12, 16, 45));

        let second = &fetch.items[1];
        assert_eq!(second.id, "linear:ENG-12");
        assert_eq!(
            (second.priority_rank, second.priority_label.as_deref()),
            (None, None)
        );
        assert!(!second.in_sprint, "the next cycle has not started");
        assert_eq!(second.project.as_deref(), Some("Engineering"));

        let mention = &fetch.items[2];
        assert_eq!(mention.id, "linear:ENG-7", "merges with the assigned issue");
        assert_eq!(mention.kind, WorkItemKind::Mention);
        assert_eq!(mention.title, "Crash on launch for new users");

        let update = &fetch.items[3];
        assert_eq!(
            update.id,
            "linear:notification-6c2f5a3e-1111-4c1a-9b7e-5f0f6f2a0004"
        );
        assert_eq!(update.kind, WorkItemKind::Notification);
        assert_eq!(update.title, "Project update: Onboarding v2");
    }

    #[test]
    fn priorities_and_cycles() {
        assert_eq!(priority_rank(Some(1.0)), Some(0));
        assert_eq!(priority_rank(Some(2.0)), Some(1));
        assert_eq!(priority_rank(Some(3.0)), Some(2));
        assert_eq!(priority_rank(Some(4.0)), Some(3));
        assert_eq!(priority_rank(Some(0.0)), None);
        assert_eq!(priority_rank(Some(9.0)), None);
        assert_eq!(priority_rank(None), None);

        let by_dates = Cycle {
            starts_at: Some("2026-10-06T00:00:00.000Z".into()),
            ends_at: Some("2026-10-20T00:00:00.000Z".into()),
            is_active: None,
        };
        assert!(cycle_is_active(&by_dates, now()));
        assert!(!cycle_is_active(&by_dates, utc(2026, 10, 20, 0, 0)));
        let flagged = Cycle {
            is_active: Some(false),
            ..by_dates
        };
        assert!(!cycle_is_active(&flagged, now()));
    }

    #[test]
    fn graphql_errors_are_classified() {
        let headers = HeaderMap::new();
        let auth = r#"{"errors":[{"message":"Authentication required, not authenticated","extensions":{"type":"authentication error","code":"AUTHENTICATION_ERROR","userPresentableMessage":"You need to authenticate to access this operation."}}]}"#;
        assert!(matches!(
            parse_response(400, auth, &headers),
            Err(PlatformError::Auth(ref m)) if m == AUTH_ERROR
        ));
        assert!(matches!(
            parse_response(401, "", &headers),
            Err(PlatformError::Auth(_))
        ));
        let limited =
            r#"{"errors":[{"message":"Rate limit exceeded","extensions":{"code":"RATELIMITED"}}]}"#;
        assert!(matches!(
            parse_response(400, limited, &headers),
            Err(PlatformError::Network(ref m)) if m.contains("rate limited")
        ));
        assert!(matches!(
            parse_response(502, "<html>Bad gateway</html>", &headers),
            Err(PlatformError::Network(ref m)) if m.contains("502")
        ));
        let partial = r#"{"data":{"viewer":{"name":"Jan","assignedIssues":{"nodes":[]}},"notifications":null},
            "errors":[{"message":"Cannot query notifications\nsecond line","path":["notifications"],"extensions":{"code":"INTERNAL_SERVER_ERROR"}}]}"#;
        let parsed = parse_response(200, partial, &headers).expect("partial data");
        assert_eq!(
            parsed.warnings,
            vec!["Linear: INTERNAL_SERVER_ERROR Cannot query notifications".to_owned()]
        );
        let empty = r#"{"data":null,"errors":[{"message":"x","extensions":{"code":"GRAPHQL_VALIDATION_FAILED"}}]}"#;
        assert!(matches!(
            parse_response(400, empty, &headers),
            Err(PlatformError::Network(ref m)) if m.contains("GRAPHQL_VALIDATION_FAILED")
        ));
    }

    #[test]
    fn query_asks_for_the_documented_fields() {
        for part in [
            "assignedIssues(",
            "nin: [\"completed\", \"canceled\", \"duplicate\"]",
            "cycle { startsAt endsAt isActive }",
            "... on IssueNotification",
            "snoozedUntilAt",
        ] {
            assert!(QUERY.contains(part), "{part}");
        }
        assert!(!format!("{:?}", LinearClient::new("lin_api_secret".into())).contains("secret"));
    }
}
