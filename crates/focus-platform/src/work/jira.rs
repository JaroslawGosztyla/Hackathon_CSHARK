//! Jira, read-only: Jira Cloud with the account e-mail + an API token (HTTP Basic), Jira Data Center / Server with a
//! personal access token (`Authorization: Bearer`) when no e-mail is configured.
//!
//! * Account: `GET /rest/api/{3|2}/myself` (`displayName`).
//! * Sprint field: `GET /rest/api/{3|2}/field`, every custom field whose `schema.custom` ends with `:gh-sprint`
//!   (cached for the client's lifetime once read).
//! * Assigned: JQL [`ASSIGNED_JQL`] → [`WorkItemKind::Assigned`] with priority, due date (date only), status and
//!   "in sprint" (a sprint of the issue with state `active`; Data Center may send the legacy
//!   `…Sprint@…[…,state=ACTIVE,…]` strings).
//!   Cloud searches with `POST /rest/api/3/search/jql` (the old `/rest/api/3/search` was removed in 2025; the new
//!   endpoint returns only ids unless `fields` are listed), Data Center with `POST /rest/api/2/search`.
//! * Watched: JQL [`WATCHED_JQL`] (issues the user watches, assigned to someone else, updated in the last day) →
//!   [`WorkItemKind::Notification`] (Jira has no notifications API; this part degrades into a warning).
//! * Scoped API tokens (Cloud) only work through Atlassian's API gateway: when the site refuses the token, the
//!   site's cloud id is read from `/_edge/tenant_info` (no credentials sent) and the requests go to
//!   `https://api.atlassian.com/ex/jira/<cloudId>` instead. Scopes: `read:jira-work`, `read:jira-user`.

use focus_core::api::{WorkItem, WorkItemKind, WorkSource};
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    MAX_REDIRECTS, Token, WorkFetch, WorkProvider, build_url, due_end_of_day, https_base, new_item,
    non_empty, parse_instant, parse_json, priority_rank_from_name, send,
};
use crate::PlatformError;
use crate::calendar::TimeZone;
use crate::net::LazyClient;

const SERVICE: &str = "Jira";
/// Issues assigned to the user and not done. `priority DESC` lists the highest priority first; the order only
/// decides which issues fit into [`MAX_ASSIGNED`] (ranking happens in `focus_core::work`).
pub const ASSIGNED_JQL: &str = "assignee = currentUser() AND statusCategory != Done \
     ORDER BY priority DESC, duedate ASC, updated DESC";
/// Watched issues of others that changed in the last day.
pub const WATCHED_JQL: &str = "watcher = currentUser() AND (assignee != currentUser() OR assignee is EMPTY) \
     AND statusCategory != Done AND updated >= -1d ORDER BY updated DESC";
pub const MAX_ASSIGNED: u32 = 50;
pub const MAX_WATCHED: u32 = 20;
/// Atlassian's API gateway for scoped API tokens.
const GATEWAY: &str = "https://api.atlassian.com/ex/jira";
/// Fields read for every issue (plus the sprint fields).
const FIELDS: &[&str] = &[
    "summary", "priority", "duedate", "status", "project", "updated",
];

/// Jira client; caches the sprint field ids and the gateway address.
#[derive(Debug)]
pub struct JiraClient {
    site: String,
    email: Option<String>,
    token: Token,
    http: LazyClient,
    gateway: Option<String>,
    sprint_fields: Option<Vec<String>>,
}

impl JiraClient {
    /// `site` = `https://firma.atlassian.net` or the Data Center address; `email` empty = Data Center PAT. No I/O.
    pub fn new(site: &str, email: &str, token: String) -> Result<Self, PlatformError> {
        if site.trim().is_empty() {
            return Err(PlatformError::Invalid("the Jira address is not set".into()));
        }
        Ok(Self {
            site: https_base(site, "Jira")?,
            email: non_empty(Some(email)),
            token: Token::new(token),
            http: LazyClient::same_origin(MAX_REDIRECTS),
            gateway: None,
            sprint_fields: None,
        })
    }

    fn is_cloud(&self) -> bool {
        self.email.is_some()
    }

    fn api_prefix(&self) -> String {
        let base = self.gateway.as_deref().unwrap_or(&self.site);
        let version = if self.is_cloud() { "3" } else { "2" };
        format!("{base}/rest/api/{version}")
    }

    fn authorized(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, PlatformError> {
        let request = request.header(ACCEPT, "application/json");
        Ok(match &self.email {
            Some(email) => request.basic_auth(email, Some(self.token.expose())),
            None => request.header(AUTHORIZATION, self.token.header("Bearer ")?),
        })
    }

    fn get(&self, path: &str) -> Result<String, PlatformError> {
        let url = build_url(&self.api_prefix(), path, &[])?;
        let request = self.authorized(self.http.get()?.get(url))?;
        send(request)?.into_body(SERVICE)
    }

    fn post(&self, path: &str, body: &Value) -> Result<String, PlatformError> {
        let url = build_url(&self.api_prefix(), path, &[])?;
        let request = self.authorized(self.http.get()?.post(url).json(body))?;
        send(request)?.into_body(SERVICE)
    }

    /// `/myself`; on Cloud a refused token is retried once through the API gateway (scoped tokens).
    fn myself(&mut self) -> Result<JiraUser, PlatformError> {
        match self.get("/myself") {
            Err(PlatformError::Auth(message)) if self.is_cloud() && self.gateway.is_none() => {
                let Some(cloud_id) = self.cloud_id() else {
                    return Err(PlatformError::Auth(message));
                };
                self.gateway = Some(format!("{GATEWAY}/{cloud_id}"));
                let retried = self.get("/myself");
                if retried.is_err() {
                    self.gateway = None;
                }
                parse_json(&retried?, SERVICE)
            }
            other => parse_json(&other?, SERVICE),
        }
    }

    /// The site's cloud id from `/_edge/tenant_info` (anonymous request), if it is a Cloud site.
    fn cloud_id(&self) -> Option<String> {
        let url = build_url(&self.site, "/_edge/tenant_info", &[]).ok()?;
        let request = self
            .http
            .get()
            .ok()?
            .get(url)
            .header(ACCEPT, "application/json");
        let body = send(request).ok()?.into_body(SERVICE).ok()?;
        parse_cloud_id(&body)
    }

    /// Ids of the sprint custom fields (cached after the first success; empty on failure).
    fn sprint_fields(&mut self, warnings: &mut Vec<String>) -> Vec<String> {
        if let Some(ids) = &self.sprint_fields {
            return ids.clone();
        }
        match self
            .get("/field")
            .and_then(|body| parse_json::<Vec<Value>>(&body, SERVICE))
        {
            Ok(fields) => {
                let ids = sprint_field_ids(&fields);
                self.sprint_fields = Some(ids.clone());
                ids
            }
            Err(err) => {
                warnings.push(format!("Jira sprint field: {err}"));
                Vec::new()
            }
        }
    }

    fn search(
        &self,
        jql: &str,
        max: u32,
        sprint_fields: &[String],
    ) -> Result<Vec<Value>, PlatformError> {
        let fields: Vec<&str> = FIELDS
            .iter()
            .copied()
            .chain(sprint_fields.iter().map(String::as_str))
            .collect();
        let (path, body) = if self.is_cloud() {
            (
                "/search/jql",
                json!({ "jql": jql, "fields": fields, "maxResults": max }),
            )
        } else {
            (
                "/search",
                json!({ "jql": jql, "fields": fields, "maxResults": max, "startAt": 0 }),
            )
        };
        let page: SearchPage = parse_json(&self.post(path, &body)?, SERVICE)?;
        Ok(page.issues)
    }
}

impl WorkProvider for JiraClient {
    fn source(&self) -> WorkSource {
        WorkSource::Jira
    }

    fn fetch(&mut self, _now_ms: i64) -> Result<WorkFetch, PlatformError> {
        let tz = TimeZone::local();
        let user = self.myself()?;
        let mut warnings = Vec::new();
        let sprint_fields = self.sprint_fields(&mut warnings);
        let assigned = self.search(ASSIGNED_JQL, MAX_ASSIGNED, &sprint_fields)?;
        let mut items: Vec<WorkItem> = assigned
            .iter()
            .filter_map(|issue| {
                issue_item(
                    issue,
                    WorkItemKind::Assigned,
                    &self.site,
                    &sprint_fields,
                    &tz,
                )
            })
            .collect();
        match self.search(WATCHED_JQL, MAX_WATCHED, &sprint_fields) {
            Ok(watched) => items.extend(watched.iter().filter_map(|issue| {
                issue_item(
                    issue,
                    WorkItemKind::Notification,
                    &self.site,
                    &sprint_fields,
                    &tz,
                )
            })),
            Err(err) => warnings.push(format!("Jira watched issues: {err}")),
        }
        Ok(WorkFetch {
            account: non_empty(user.display_name.as_deref()),
            items,
            poll_interval_secs: None,
            warnings,
        })
    }
}

// -- responses -----------------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct JiraUser {
    display_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SearchPage {
    issues: Vec<Value>,
}

// -- mapping (pure) -------------------------------------------------------------------------------------------

/// `{"cloudId":"…"}` → the id when it is a safe path segment: a UUID, sometimes with a suffix
/// (`fff89a1a-52c8-11e5-91b2-3c15c2bd8caa-ecosystem`), i.e. letters, digits and dashes.
fn parse_cloud_id(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let id = value.get("cloudId")?.as_str()?.trim();
    ((8..=100).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !id.starts_with('-'))
    .then(|| id.to_owned())
}

/// Ids of the custom fields of type `com.pyxis.greenhopper.jira:gh-sprint`.
fn sprint_field_ids(fields: &[Value]) -> Vec<String> {
    fields
        .iter()
        .filter(|field| {
            field
                .pointer("/schema/custom")
                .and_then(Value::as_str)
                .is_some_and(|custom| custom.ends_with(":gh-sprint"))
        })
        .filter_map(|field| field.get("id").and_then(Value::as_str))
        .filter(|id| id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        .map(str::to_owned)
        .collect()
}

/// A sprint field value is (or contains) an active sprint: Cloud / newer Data Center objects with
/// `"state": "active"`, or legacy `…Sprint@1a2b[id=1,…,state=ACTIVE,…]` strings.
fn has_active_sprint(value: &Value) -> bool {
    match value {
        Value::Array(sprints) => sprints.iter().any(has_active_sprint),
        Value::Object(sprint) => sprint
            .get("state")
            .and_then(Value::as_str)
            .is_some_and(|state| state.eq_ignore_ascii_case("active")),
        Value::String(text) => text.to_ascii_lowercase().contains("state=active"),
        _ => false,
    }
}

/// Normalised rank of a Jira priority: by name, else by the default priority ids 1 (Highest) ..= 5 (Lowest).
fn jira_priority_rank(priority: &Value) -> Option<u32> {
    let by_name = priority
        .get("name")
        .and_then(Value::as_str)
        .and_then(priority_rank_from_name);
    by_name.or_else(|| {
        priority
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse::<u32>().ok())
            .filter(|id| (1..=5).contains(id))
            .map(|id| id - 1)
    })
}

fn text_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn issue_item(
    issue: &Value,
    kind: WorkItemKind,
    site: &str,
    sprint_fields: &[String],
    tz: &TimeZone,
) -> Option<WorkItem> {
    let key = non_empty(issue.get("key").and_then(Value::as_str))?;
    if !key
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let fields = issue.get("fields").unwrap_or(&Value::Null);
    let mut item = new_item(
        WorkSource::Jira,
        kind,
        key.clone(),
        text_at(fields, "/summary").unwrap_or_default().to_owned(),
        format!("{site}/browse/{key}"),
        text_at(fields, "/updated")
            .and_then(parse_instant)
            .unwrap_or(0),
    );
    item.project = non_empty(text_at(fields, "/project/name"))
        .or_else(|| non_empty(text_at(fields, "/project/key")));
    if let Some(priority) = fields.get("priority").filter(|p| p.is_object()) {
        item.priority_label = non_empty(text_at(priority, "/name"));
        item.priority_rank = jira_priority_rank(priority);
    }
    item.due_ms = text_at(fields, "/duedate").and_then(|date| due_end_of_day(date, tz));
    item.status = non_empty(text_at(fields, "/status/name"));
    item.in_sprint = sprint_fields
        .iter()
        .filter_map(|id| fields.get(id.as_str()))
        .any(has_active_sprint);
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::test_support::{cest, end_of_day, utc};

    const SITE: &str = "https://firma.atlassian.net";

    /// Trimmed `GET /rest/api/3/field` response.
    const FIELDS_RESPONSE: &str = r#"[
      {"id": "summary", "key": "summary", "name": "Summary", "custom": false, "orderable": true,
       "navigable": true, "searchable": true, "clauseNames": ["summary"],
       "schema": {"type": "string", "system": "summary"}},
      {"id": "customfield_10020", "key": "customfield_10020", "name": "Sprint", "untranslatedName": "Sprint",
       "custom": true, "orderable": true, "navigable": true, "searchable": true,
       "clauseNames": ["cf[10020]", "Sprint"],
       "schema": {"type": "array", "items": "json", "custom": "com.pyxis.greenhopper.jira:gh-sprint", "customId": 10020}},
      {"id": "customfield_10016", "name": "Story point estimate", "custom": true,
       "schema": {"type": "number", "custom": "com.pyxis.greenhopper.jira:jsw-story-points", "customId": 10016}},
      {"id": "customfield_10104", "name": "Sprint (old)", "custom": true,
       "schema": {"type": "array", "items": "string", "custom": "com.pyxis.greenhopper.jira:gh-sprint", "customId": 10104}}
    ]"#;

    /// Trimmed `POST /rest/api/3/search/jql` response.
    const SEARCH_RESPONSE: &str = r#"{
      "issues": [
        {
          "expand": "renderedFields,names,schema,operations,editmeta,changelog,versionedRepresentations",
          "id": "10068",
          "self": "https://firma.atlassian.net/rest/api/3/issue/10068",
          "key": "PROJ-123",
          "fields": {
            "summary": "Checkout fails for EU customers",
            "updated": "2026-10-12T09:15:30.123+0200",
            "duedate": "2026-10-13",
            "project": {"self": "…", "id": "10000", "key": "PROJ", "name": "Shop", "projectTypeKey": "software"},
            "priority": {"self": "…", "iconUrl": "…", "name": "High", "id": "2"},
            "status": {
              "self": "…", "description": "", "iconUrl": "…", "name": "In Progress", "id": "3",
              "statusCategory": {"self": "…", "id": 4, "key": "indeterminate", "colorName": "yellow", "name": "In Progress"}
            },
            "customfield_10020": [
              {"id": 36, "name": "SHOP Sprint 41", "state": "closed", "boardId": 1,
               "startDate": "2026-09-15T08:00:00.000Z", "endDate": "2026-09-29T08:00:00.000Z"},
              {"id": 37, "name": "SHOP Sprint 42", "state": "active", "boardId": 1, "goal": "Checkout",
               "startDate": "2026-10-06T08:00:00.000Z", "endDate": "2026-10-20T08:00:00.000Z"}
            ]
          }
        },
        {
          "id": "10070",
          "key": "PROJ-130",
          "fields": {
            "summary": "Update the privacy policy",
            "updated": "2026-10-01T12:00:00.000+0000",
            "duedate": null,
            "project": {"key": "PROJ", "name": "Shop"},
            "priority": {"name": "Custom", "id": "1"},
            "status": {"name": "To Do", "statusCategory": {"key": "new"}},
            "customfield_10020": null
          }
        },
        {"id": "10071", "key": "../../etc", "fields": {}},
        {"id": "10072", "fields": {"summary": "no key"}}
      ],
      "nextPageToken": null,
      "isLast": true
    }"#;

    /// Legacy Data Center `POST /rest/api/2/search` issue with sprint strings.
    const DC_ISSUE: &str = r#"{
      "id": "20001",
      "key": "OPS-7",
      "fields": {
        "summary": "Rotate certificates",
        "updated": "2026-10-10T08:00:00.000+0200",
        "duedate": "2026-10-11",
        "project": {"key": "OPS", "name": "Operations"},
        "priority": {"name": "Blocker", "id": "1"},
        "status": {"name": "Open", "statusCategory": {"key": "new"}},
        "customfield_10104": [
          "com.atlassian.greenhopper.service.sprint.Sprint@6d1c1e1a[id=12,rapidViewId=3,state=ACTIVE,name=OPS Sprint 12,startDate=2026-10-06T09:00:00.000+02:00,endDate=2026-10-20T09:00:00.000+02:00,completeDate=<null>,sequence=12]"
        ]
      }
    }"#;

    #[test]
    fn sprint_fields_are_discovered() {
        let fields: Vec<Value> = serde_json::from_str(FIELDS_RESPONSE).expect("json");
        assert_eq!(
            sprint_field_ids(&fields),
            vec![
                "customfield_10020".to_owned(),
                "customfield_10104".to_owned()
            ]
        );
        assert!(sprint_field_ids(&[]).is_empty());
    }

    #[test]
    fn cloud_issues_map_with_priority_due_status_and_sprint() {
        let tz = cest();
        let page: SearchPage = parse_json(SEARCH_RESPONSE, SERVICE).expect("page");
        let sprint = vec!["customfield_10020".to_owned()];
        let items: Vec<WorkItem> = page
            .issues
            .iter()
            .filter_map(|i| issue_item(i, WorkItemKind::Assigned, SITE, &sprint, &tz))
            .collect();
        assert_eq!(items.len(), 2, "malformed keys are skipped");
        let first = &items[0];
        assert_eq!(first.id, "jira:PROJ-123");
        assert_eq!(first.key, "PROJ-123");
        assert_eq!(first.title, "Checkout fails for EU customers");
        assert_eq!(first.url, "https://firma.atlassian.net/browse/PROJ-123");
        assert_eq!(first.project.as_deref(), Some("Shop"));
        assert_eq!(first.priority_label.as_deref(), Some("High"));
        assert_eq!(first.priority_rank, Some(1));
        assert_eq!(first.status.as_deref(), Some("In Progress"));
        assert_eq!(first.due_ms, Some(end_of_day(2026, 10, 13, &tz)));
        assert!(first.in_sprint);
        assert_eq!(first.updated_ms, utc(2026, 10, 12, 7, 15) + 30_123);
        assert_eq!((first.score, first.unseen), (0, false));

        let second = &items[1];
        assert_eq!(second.priority_label.as_deref(), Some("Custom"));
        assert_eq!(
            second.priority_rank,
            Some(0),
            "default priority id 1 = Highest"
        );
        assert_eq!(second.due_ms, None);
        assert!(!second.in_sprint);
        assert_eq!(second.updated_ms, utc(2026, 10, 1, 12, 0));
    }

    #[test]
    fn data_center_sprint_strings_count() {
        let tz = cest();
        let issue: Value = serde_json::from_str(DC_ISSUE).expect("json");
        let item = issue_item(
            &issue,
            WorkItemKind::Notification,
            "https://jira.example.com/jira",
            &["customfield_10104".to_owned()],
            &tz,
        )
        .expect("item");
        assert!(item.in_sprint);
        assert_eq!(item.kind, WorkItemKind::Notification);
        assert_eq!(item.priority_rank, Some(0));
        assert_eq!(item.url, "https://jira.example.com/jira/browse/OPS-7");
        assert_eq!(item.due_ms, Some(end_of_day(2026, 10, 11, &tz)));
        // Without the sprint field id nothing is in a sprint.
        let without = issue_item(&issue, WorkItemKind::Assigned, SITE, &[], &tz).expect("item");
        assert!(!without.in_sprint);
    }

    #[test]
    fn active_sprints_are_recognised() {
        assert!(has_active_sprint(
            &json!([{"state": "closed"}, {"state": "ACTIVE"}])
        ));
        assert!(!has_active_sprint(&json!([{"state": "future"}])));
        assert!(has_active_sprint(&json!({"state": "active"})));
        assert!(!has_active_sprint(&json!(
            "Sprint@1[id=1,state=CLOSED,name=x]"
        )));
        assert!(!has_active_sprint(&Value::Null));
    }

    #[test]
    fn priorities_by_name_or_default_id() {
        assert_eq!(
            jira_priority_rank(&json!({"name": "Highest", "id": "1"})),
            Some(0)
        );
        assert_eq!(
            jira_priority_rank(&json!({"name": "Lowest", "id": "5"})),
            Some(4)
        );
        assert_eq!(
            jira_priority_rank(&json!({"name": "Minor", "id": "10002"})),
            Some(3)
        );
        assert_eq!(
            jira_priority_rank(&json!({"name": "Sev A", "id": "3"})),
            Some(2)
        );
        assert_eq!(
            jira_priority_rank(&json!({"name": "Sev A", "id": "10001"})),
            None
        );
    }

    #[test]
    fn cloud_ids_are_validated() {
        assert_eq!(
            parse_cloud_id(r#"{"cloudId":"a436116f-02ce-4520-8fbb-7301462a1674"}"#).as_deref(),
            Some("a436116f-02ce-4520-8fbb-7301462a1674")
        );
        // Real id of an older site (seen on ecosystem.atlassian.net).
        assert_eq!(
            parse_cloud_id(r#"{"cloudId":"fff89a1a-52c8-11e5-91b2-3c15c2bd8caa-ecosystem"}"#)
                .as_deref(),
            Some("fff89a1a-52c8-11e5-91b2-3c15c2bd8caa-ecosystem")
        );
        assert_eq!(parse_cloud_id(r#"{"cloudId":"../../x"}"#), None);
        assert_eq!(parse_cloud_id(r#"{"cloudId":"abc/def-0123"}"#), None);
        assert_eq!(parse_cloud_id(r#"{"cloudId":"abc"}"#), None);
        assert_eq!(parse_cloud_id("<html>"), None);
        assert_eq!(parse_cloud_id("{}"), None);
    }

    #[test]
    fn cloud_and_data_center_differ() {
        let cloud =
            JiraClient::new(SITE, "ja@firma.pl", "ATATT3xSecretValue".into()).expect("cloud");
        assert!(cloud.is_cloud());
        assert_eq!(cloud.api_prefix(), "https://firma.atlassian.net/rest/api/3");
        let mut dc =
            JiraClient::new("https://jira.example.com/jira/", " ", "pat".into()).expect("dc");
        assert!(!dc.is_cloud());
        assert_eq!(dc.api_prefix(), "https://jira.example.com/jira/rest/api/2");
        dc.gateway = Some(format!("{GATEWAY}/abc"));
        assert_eq!(
            dc.api_prefix(),
            "https://api.atlassian.com/ex/jira/abc/rest/api/2"
        );
        assert!(matches!(
            JiraClient::new("", "", "x".into()),
            Err(PlatformError::Invalid(_))
        ));
        assert!(!format!("{cloud:?}").contains("SecretValue"));
        assert!(ASSIGNED_JQL.contains("assignee = currentUser()"));
        assert!(WATCHED_JQL.contains("watcher = currentUser()"));
    }
}
