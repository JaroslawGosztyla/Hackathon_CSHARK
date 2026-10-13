//! Azure DevOps Services (`https://dev.azure.com/<org>`), read-only, with an organisation-scoped personal access
//! token sent as HTTP Basic (empty user name, the PAT as password). Scopes: Work Items (Read), Code (Read),
//! Project and Team (Read). Global PATs stop working on 1 December 2026; org-scoped ones keep working.
//!
//! * Account: `GET /_apis/connectionData` (`authenticatedUser.id` and `providerDisplayName`).
//! * Assigned: organisation-level WIQL ([`ASSIGNED_WIQL`], at most [`MAX_WORK_ITEMS`]) and
//!   `POST /_apis/wit/workitemsbatch` for title, state, type, project, iteration, priority (1..=4) and the due /
//!   target date → [`WorkItemKind::Assigned`]. "In sprint" = the item's iteration path is the current iteration of
//!   its project's default team (`GET /<project>/_apis/work/teamsettings/iterations?$timeframe=current`, at most
//!   [`MAX_ITERATION_PROJECTS`] projects).
//! * Pull requests per project (`GET /_apis/projects`, the [`MAX_PR_PROJECTS`] most recently updated):
//!   `searchCriteria.reviewerId=<me>` where the user has not voted yet → [`WorkItemKind::ReviewRequested`];
//!   `searchCriteria.creatorId=<me>` with a reviewer vote "waiting for author" (-5) or "rejected" (-10) →
//!   [`WorkItemKind::ChangesRequested`].
//!
//! Every request sends `X-TFS-FedAuthRedirect: Suppress`, so a rejected PAT gets `401` instead of a `302` to the
//! interactive sign-in page; a `203` with an HTML sign-in page (older behaviour) is [`PlatformError::Auth`] too.

use std::collections::HashMap;

use focus_core::api::{WorkItem, WorkItemKind, WorkSource};
use reqwest::header::ACCEPT;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    AUTH_ERROR, MAX_REDIRECTS, Reply, Token, WorkFetch, WorkProvider, build_url, date_like_due,
    new_item, non_empty, parse_each, parse_instant, parse_json, push_segments, send,
};
use crate::PlatformError;
use crate::calendar::TimeZone;
use crate::net::LazyClient;

const HOST: &str = "https://dev.azure.com";
const API_VERSION: &str = "7.1";
/// `connectionData` is only versioned as a preview.
const CONNECTION_API_VERSION: &str = "7.1-preview";
const SERVICE: &str = "Azure DevOps";
/// Work items assigned to the user that are not finished.
pub const ASSIGNED_WIQL: &str = "Select [System.Id] From WorkItems \
     Where [System.AssignedTo] = @Me And [System.State] Not In ('Closed', 'Done', 'Removed', 'Resolved') \
     Order By [Microsoft.VSTS.Common.Priority] Asc, [System.ChangedDate] Desc";
pub const MAX_WORK_ITEMS: usize = 50;
pub const MAX_ITERATION_PROJECTS: usize = 5;
pub const MAX_PR_PROJECTS: usize = 10;
/// Fields read per work item.
const FIELDS: &[&str] = &[
    "System.Id",
    "System.Title",
    "System.State",
    "System.WorkItemType",
    "System.TeamProject",
    "System.IterationPath",
    "System.ChangedDate",
    "Microsoft.VSTS.Common.Priority",
    "Microsoft.VSTS.Scheduling.DueDate",
    "Microsoft.VSTS.Scheduling.TargetDate",
];

/// Azure DevOps client.
#[derive(Debug)]
pub struct AzureDevopsClient {
    org: String,
    token: Token,
    http: LazyClient,
}

impl AzureDevopsClient {
    /// `org` = the organisation name (`firma` of `https://dev.azure.com/firma`). No I/O.
    pub fn new(org: &str, token: String) -> Result<Self, PlatformError> {
        let org = org.trim();
        if org.is_empty() {
            return Err(PlatformError::Invalid(
                "the Azure DevOps organisation is not set".into(),
            ));
        }
        if org.len() > 64
            || org.bytes().all(|b| b == b'.')
            || !org
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(PlatformError::Invalid(
                "the Azure DevOps organisation name is not valid".into(),
            ));
        }
        Ok(Self {
            org: org.to_owned(),
            token: Token::new(token),
            http: LazyClient::same_origin(MAX_REDIRECTS),
        })
    }

    /// `https://dev.azure.com/<org>/<segments…>?<query>&api-version=7.1`. `raw_query` is appended as is (it holds
    /// `$`-parameters the service expects unencoded).
    fn url(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        raw_query: &str,
    ) -> Result<reqwest::Url, PlatformError> {
        self.versioned_url(segments, query, raw_query, Some(API_VERSION))
    }

    /// [`Self::url`] with another `api-version` (`None` = the server's default).
    fn versioned_url(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        raw_query: &str,
        api_version: Option<&str>,
    ) -> Result<reqwest::Url, PlatformError> {
        let mut url = build_url(HOST, "", &[])?;
        push_segments(&mut url, &[self.org.as_str()])?;
        push_segments(&mut url, segments)?;
        if !query.is_empty() || api_version.is_some() {
            let mut pairs = url.query_pairs_mut();
            pairs.extend_pairs(query);
            if let Some(version) = api_version {
                pairs.append_pair("api-version", version);
            }
        }
        if !raw_query.is_empty() {
            let full = match url.query() {
                Some(existing) if !existing.is_empty() => format!("{existing}&{raw_query}"),
                _ => raw_query.to_owned(),
            };
            url.set_query(Some(&full));
        }
        Ok(url)
    }

    /// `connectionData` (the signed-in identity). It is a Location API without a released version: asked
    /// without `api-version` first, then once with the preview version if the server insists on one.
    fn connection(&self) -> Result<AzConnection, PlatformError> {
        let segments = ["_apis", "connectionData"];
        let reply = self.send(self.http.get()?.get(self.versioned_url(
            &segments,
            &[],
            "",
            None,
        )?))?;
        let body = if reply.status == 400 {
            let url = self.versioned_url(&segments, &[], "", Some(CONNECTION_API_VERSION))?;
            Self::body(self.send(self.http.get()?.get(url))?)?
        } else {
            Self::body(reply)?
        };
        parse_json(&body, SERVICE)
    }

    fn send(&self, request: reqwest::blocking::RequestBuilder) -> Result<Reply, PlatformError> {
        send(
            request
                .header(ACCEPT, "application/json")
                // A rejected token gets 401 instead of a 302 to the interactive sign-in page.
                .header("X-TFS-FedAuthRedirect", "Suppress")
                .basic_auth("", Some(self.token.expose())),
        )
    }

    fn body(reply: Reply) -> Result<String, PlatformError> {
        // A rejected PAT can be answered with 203 and an HTML sign-in page.
        if reply.status == 203 || (reply.is_success() && reply.body.trim_start().starts_with('<')) {
            return Err(PlatformError::Auth(AUTH_ERROR.into()));
        }
        reply.into_body(SERVICE)
    }

    fn get(&self, url: reqwest::Url) -> Result<String, PlatformError> {
        Self::body(self.send(self.http.get()?.get(url))?)
    }

    fn post(&self, url: reqwest::Url, body: &Value) -> Result<Reply, PlatformError> {
        self.send(self.http.get()?.post(url).json(body))
    }

    fn work_items(&self, ids: &[u64]) -> Result<Vec<AzWorkItem>, PlatformError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = self.url(&["_apis", "wit", "workitemsbatch"], &[], "")?;
        let reply = self.post(
            url.clone(),
            &json!({ "ids": ids, "fields": FIELDS, "errorPolicy": "omit" }),
        )?;
        // A field missing from the organisation's processes fails the whole batch: ask for all fields instead.
        let body = if reply.status == 400 {
            Self::body(self.post(url, &json!({ "ids": ids, "errorPolicy": "omit" }))?)?
        } else {
            Self::body(reply)?
        };
        let page: AzList = parse_json(&body, SERVICE)?;
        Ok(parse_each(page.value))
    }

    /// Current iteration path of each project's default team.
    fn current_iterations(
        &self,
        projects: &[String],
        warnings: &mut Vec<String>,
    ) -> HashMap<String, String> {
        let mut current = HashMap::new();
        let mut failures = 0;
        for project in projects.iter().take(MAX_ITERATION_PROJECTS) {
            let result = self
                .url(
                    &[
                        project.as_str(),
                        "_apis",
                        "work",
                        "teamsettings",
                        "iterations",
                    ],
                    &[],
                    "$timeframe=current",
                )
                .and_then(|url| self.get(url))
                .and_then(|body| parse_json::<AzList>(&body, SERVICE));
            match result {
                Ok(list) => {
                    if let Some(path) = current_iteration_path(list) {
                        current.insert(project.clone(), path);
                    }
                }
                Err(err) => {
                    failures += 1;
                    log::debug!("azure devops: reading the current iteration failed: {err}");
                }
            }
        }
        if failures > 0 {
            warnings.push(format!(
                "Azure DevOps: the current iteration of {failures} projects could not be read"
            ));
        }
        current
    }

    /// Projects to look for pull requests in, most recently updated first.
    fn projects(&self) -> Result<Vec<String>, PlatformError> {
        let url = self.url(
            &["_apis", "projects"],
            &[("stateFilter", "wellFormed")],
            "$top=100",
        )?;
        let list: AzList = parse_json(&self.get(url)?, SERVICE)?;
        Ok(project_names(parse_each(list.value)))
    }

    fn pull_requests(
        &self,
        project: &str,
        criterion: &str,
        user_id: &str,
    ) -> Result<Vec<AzPullRequest>, PlatformError> {
        let url = self.url(
            &[project, "_apis", "git", "pullrequests"],
            &[(criterion, user_id), ("searchCriteria.status", "active")],
            "$top=50",
        )?;
        let list: AzList = parse_json(&self.get(url)?, SERVICE)?;
        Ok(parse_each(list.value))
    }

    fn pull_request_items(&self, user: &AzUser, warnings: &mut Vec<String>) -> Vec<WorkItem> {
        let projects = match self.projects() {
            Ok(projects) => projects,
            Err(err) => {
                warnings.push(format!("Azure DevOps projects: {err}"));
                return Vec::new();
            }
        };
        let mut items = Vec::new();
        let mut failures = 0;
        for project in projects.iter().take(MAX_PR_PROJECTS) {
            match self.pull_requests(project, "searchCriteria.reviewerId", &user.id) {
                Ok(prs) => items.extend(
                    prs.iter()
                        .filter(|pr| awaits_vote(pr, &user.id))
                        .filter_map(|pr| pr_item(pr, WorkItemKind::ReviewRequested, &self.org)),
                ),
                Err(err) => {
                    failures += 1;
                    log::debug!("azure devops: reading pull requests failed: {err}");
                }
            }
            match self.pull_requests(project, "searchCriteria.creatorId", &user.id) {
                Ok(prs) => items.extend(
                    prs.iter()
                        .filter(|pr| changes_requested(pr))
                        .filter_map(|pr| pr_item(pr, WorkItemKind::ChangesRequested, &self.org)),
                ),
                Err(err) => {
                    failures += 1;
                    log::debug!("azure devops: reading pull requests failed: {err}");
                }
            }
        }
        if failures > 0 {
            warnings.push(format!(
                "Azure DevOps: {failures} pull request lists could not be read (the token needs Code (Read))"
            ));
        }
        items
    }
}

impl WorkProvider for AzureDevopsClient {
    fn source(&self) -> WorkSource {
        WorkSource::AzureDevops
    }

    fn fetch(&mut self, _now_ms: i64) -> Result<WorkFetch, PlatformError> {
        let tz = TimeZone::local();
        let user = self.connection()?.authenticated_user;
        // An anonymous identity (empty or all-zero id) means the token was not accepted.
        if user.id.bytes().all(|b| b == b'0' || b == b'-') {
            return Err(PlatformError::Auth(AUTH_ERROR.into()));
        }

        let wiql_url = self.url(
            &["_apis", "wit", "wiql"],
            &[],
            &format!("$top={MAX_WORK_ITEMS}"),
        )?;
        let wiql: AzWiql = parse_json(
            &Self::body(self.post(wiql_url, &json!({ "query": ASSIGNED_WIQL }))?)?,
            SERVICE,
        )?;
        let ids: Vec<u64> = wiql
            .work_items
            .iter()
            .map(|reference| reference.id)
            .filter(|&id| id > 0)
            .take(MAX_WORK_ITEMS)
            .collect();
        let work_items = self.work_items(&ids)?;

        let mut warnings = Vec::new();
        let projects = distinct_projects(&work_items);
        let iterations = self.current_iterations(&projects, &mut warnings);
        let mut items: Vec<WorkItem> = work_items
            .iter()
            .filter_map(|wi| work_item(wi, &self.org, &iterations, &tz))
            .collect();
        items.extend(self.pull_request_items(&user, &mut warnings));
        Ok(WorkFetch {
            account: non_empty(user.provider_display_name.as_deref()),
            items,
            poll_interval_secs: None,
            warnings,
        })
    }
}

// -- responses -----------------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzConnection {
    authenticated_user: AzUser,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzUser {
    id: String,
    provider_display_name: Option<String>,
}

/// `{ "count": n, "value": [...] }` (the iterations sample in the docs says `values`).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AzList {
    #[serde(alias = "values")]
    value: Vec<Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzWiql {
    work_items: Vec<AzReference>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AzReference {
    id: u64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AzWorkItem {
    id: u64,
    fields: serde_json::Map<String, Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzIteration {
    path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzProject {
    name: Option<String>,
    last_update_time: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzPullRequest {
    pull_request_id: u64,
    title: Option<String>,
    creation_date: Option<String>,
    is_draft: bool,
    repository: AzRepository,
    reviewers: Vec<AzReviewer>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzRepository {
    name: Option<String>,
    project: AzProjectRef,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AzProjectRef {
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AzReviewer {
    id: String,
    vote: i32,
    has_declined: bool,
}

// -- mapping (pure) -------------------------------------------------------------------------------------------

fn field<'a>(item: &'a AzWorkItem, name: &str) -> Option<&'a Value> {
    item.fields.get(name).filter(|v| !v.is_null())
}

fn field_text<'a>(item: &'a AzWorkItem, name: &str) -> Option<&'a str> {
    field(item, name).and_then(Value::as_str)
}

fn distinct_projects(items: &[AzWorkItem]) -> Vec<String> {
    let mut projects: Vec<String> = Vec::new();
    for item in items {
        if let Some(project) = non_empty(field_text(item, "System.TeamProject"))
            && !projects.contains(&project)
        {
            projects.push(project);
        }
    }
    projects
}

fn current_iteration_path(list: AzList) -> Option<String> {
    parse_each::<AzIteration>(list.value)
        .into_iter()
        .find_map(|iteration| non_empty(iteration.path.as_deref()))
}

/// Project names, most recently updated first.
fn project_names(mut projects: Vec<AzProject>) -> Vec<String> {
    projects.sort_by_key(|p| {
        std::cmp::Reverse(
            p.last_update_time
                .as_deref()
                .and_then(parse_instant)
                .unwrap_or(0),
        )
    });
    projects
        .into_iter()
        .filter_map(|p| non_empty(p.name.as_deref()))
        .collect()
}

/// `https://dev.azure.com/<org>/<segments…>` (each segment percent-encoded).
fn web_url(org: &str, segments: &[&str]) -> String {
    let mut url = match reqwest::Url::parse(HOST) {
        Ok(url) => url,
        Err(_) => return String::new(),
    };
    if push_segments(&mut url, &[org]).is_err() || push_segments(&mut url, segments).is_err() {
        return String::new();
    }
    url.into()
}

/// Priority 1 (highest) ..= 4 → rank 0 ..= 3.
fn priority_of(item: &AzWorkItem) -> Option<u32> {
    let value = field(item, "Microsoft.VSTS.Common.Priority")?;
    let priority = value
        .as_u64()
        .or_else(|| value.as_f64().map(|f| f as u64))
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))?;
    (1..=4)
        .contains(&priority)
        .then(|| u32::try_from(priority - 1).unwrap_or(3))
}

fn work_item(
    item: &AzWorkItem,
    org: &str,
    current_iterations: &HashMap<String, String>,
    tz: &TimeZone,
) -> Option<WorkItem> {
    if item.id == 0 {
        return None;
    }
    let project = non_empty(field_text(item, "System.TeamProject"));
    let id = item.id.to_string();
    let url = match &project {
        Some(project) => web_url(org, &[project.as_str(), "_workitems", "edit", id.as_str()]),
        None => web_url(org, &["_workitems", "edit", id.as_str()]),
    };
    let mut out = new_item(
        WorkSource::AzureDevops,
        WorkItemKind::Assigned,
        format!("#{id}"),
        field_text(item, "System.Title")
            .unwrap_or_default()
            .to_owned(),
        url,
        field_text(item, "System.ChangedDate")
            .and_then(parse_instant)
            .unwrap_or(0),
    );
    if let Some(rank) = priority_of(item) {
        out.priority_rank = Some(rank);
        out.priority_label = Some(format!("P{}", rank + 1));
    }
    out.due_ms = field_text(item, "Microsoft.VSTS.Scheduling.DueDate")
        .or_else(|| field_text(item, "Microsoft.VSTS.Scheduling.TargetDate"))
        .and_then(|text| date_like_due(text, tz));
    out.status = non_empty(field_text(item, "System.State"));
    out.in_sprint = match (&project, field_text(item, "System.IterationPath")) {
        (Some(project), Some(path)) => current_iterations.get(project).is_some_and(|current| {
            path.eq_ignore_ascii_case(current)
                || path
                    .to_lowercase()
                    .starts_with(&format!("{}\\", current.to_lowercase()))
        }),
        _ => false,
    };
    out.project = project;
    Some(out)
}

/// The user is a reviewer who has not voted or declined yet (a team request without the user's own entry counts).
fn awaits_vote(pr: &AzPullRequest, user_id: &str) -> bool {
    if pr.is_draft {
        return false;
    }
    match pr
        .reviewers
        .iter()
        .find(|r| r.id.eq_ignore_ascii_case(user_id))
    {
        Some(me) => me.vote == 0 && !me.has_declined,
        None => true,
    }
}

/// A reviewer voted "waiting for author" (-5) or "rejected" (-10).
fn changes_requested(pr: &AzPullRequest) -> bool {
    pr.reviewers.iter().any(|r| r.vote < 0)
}

fn pr_item(pr: &AzPullRequest, kind: WorkItemKind, org: &str) -> Option<WorkItem> {
    let repo = non_empty(pr.repository.name.as_deref())?;
    let project = non_empty(pr.repository.project.name.as_deref())?;
    if pr.pull_request_id == 0 {
        return None;
    }
    let id = pr.pull_request_id.to_string();
    let mut item = new_item(
        WorkSource::AzureDevops,
        kind,
        format!("{repo}!{id}"),
        pr.title.clone().unwrap_or_default(),
        web_url(
            org,
            &[
                project.as_str(),
                "_git",
                repo.as_str(),
                "pullrequest",
                id.as_str(),
            ],
        ),
        pr.creation_date
            .as_deref()
            .and_then(parse_instant)
            .unwrap_or(0),
    );
    item.project = Some(project);
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::test_support::{cest, end_of_day, utc};

    const CONNECTION: &str = r#"{
      "authenticatedUser": {
        "id": "8c8c7d32-6b1b-47f4-b2e9-30b477b5ab3d",
        "descriptor": "Microsoft.IdentityModel.Claims.ClaimsIdentity;…",
        "subjectDescriptor": "aad.ODhjN2QzMi02YjFi",
        "providerDisplayName": "Jan Kowalski",
        "isActive": true,
        "properties": {"Account": {"$type": "System.String", "$value": "jan@firma.pl"}},
        "resourceVersion": 2,
        "metaTypeId": 0
      },
      "authorizedUser": {"id": "8c8c7d32-6b1b-47f4-b2e9-30b477b5ab3d"},
      "instanceId": "3f1b2c1e-0000-0000-0000-000000000000",
      "deploymentType": "hosted",
      "locationServiceData": {}
    }"#;

    const WIQL: &str = r#"{
      "queryType": "flat",
      "queryResultType": "workItem",
      "asOf": "2026-10-13T08:00:00.000Z",
      "columns": [{"referenceName": "System.Id", "name": "ID", "url": "https://dev.azure.com/firma/_apis/wit/fields/System.Id"}],
      "sortColumns": [],
      "workItems": [
        {"id": 300, "url": "https://dev.azure.com/firma/_apis/wit/workItems/300"},
        {"id": 299, "url": "https://dev.azure.com/firma/_apis/wit/workItems/299"}
      ]
    }"#;

    const BATCH: &str = r#"{
      "count": 3,
      "value": [
        {
          "id": 300,
          "rev": 7,
          "fields": {
            "System.Id": 300,
            "System.Title": "Payment retries",
            "System.State": "Active",
            "System.WorkItemType": "User Story",
            "System.TeamProject": "Fabrikam Fiber",
            "System.IterationPath": "Fabrikam Fiber\\Release 1\\Sprint 3",
            "System.ChangedDate": "2026-10-12T14:03:12.567Z",
            "Microsoft.VSTS.Common.Priority": 1,
            "Microsoft.VSTS.Scheduling.TargetDate": "2026-10-14T22:00:00Z"
          },
          "url": "https://dev.azure.com/firma/_apis/wit/workItems/300"
        },
        {
          "id": 299,
          "rev": 2,
          "fields": {
            "System.Id": 299,
            "System.Title": "Update the README",
            "System.State": "New",
            "System.TeamProject": "Fabrikam Fiber",
            "System.IterationPath": "Fabrikam Fiber",
            "System.ChangedDate": "2026-10-01T08:00:00Z",
            "Microsoft.VSTS.Common.Priority": 3,
            "Microsoft.VSTS.Scheduling.DueDate": "2026-10-20T00:00:00Z"
          }
        },
        null
      ]
    }"#;

    const ITERATIONS: &str = r#"{
      "count": 1,
      "value": [
        {
          "id": "a589a806-bf11-4d4f-a031-c19813331553",
          "name": "Release 1",
          "path": "Fabrikam Fiber\\Release 1",
          "attributes": {"startDate": "2026-10-06T00:00:00Z", "finishDate": "2026-10-19T00:00:00Z", "timeFrame": "current"},
          "url": "https://dev.azure.com/firma/…/_apis/work/teamsettings/iterations/a589a806"
        }
      ]
    }"#;

    const PROJECTS: &str = r#"{
      "count": 2,
      "value": [
        {"id": "eb6e4656-77fc-42a1-9181-4c6d8e9da5d1", "name": "Old Project", "state": "wellFormed",
         "revision": 11, "visibility": "private", "lastUpdateTime": "2025-01-01T00:00:00Z"},
        {"id": "6ce954b1-ce1f-45d1-b94d-e6bf2464ba2c", "name": "Fabrikam Fiber", "state": "wellFormed",
         "revision": 400, "visibility": "private", "lastUpdateTime": "2026-10-01T00:00:00Z"}
      ]
    }"#;

    const PULLS: &str = r#"{
      "value": [
        {
          "repository": {"id": "3411ebc1", "name": "web-app", "url": "…", "project": {"id": "a7573007", "name": "Fabrikam Fiber", "state": "unchanged"}},
          "pullRequestId": 22,
          "status": "active",
          "createdBy": {"id": "d6245f20", "displayName": "Normal Paulk"},
          "creationDate": "2026-10-11T16:30:31.6655471Z",
          "title": "A new feature",
          "isDraft": false,
          "reviewers": [
            {"id": "8c8c7d32-6b1b-47f4-b2e9-30b477b5ab3d", "vote": 0, "displayName": "Jan Kowalski"},
            {"id": "98d08d98", "vote": 10, "displayName": "[Fabrikam]\\Team", "isContainer": true}
          ]
        },
        {
          "repository": {"name": "web-app", "project": {"name": "Fabrikam Fiber"}},
          "pullRequestId": 23,
          "creationDate": "2026-10-10T16:30:31Z",
          "title": "Already approved",
          "isDraft": false,
          "reviewers": [{"id": "8c8c7d32-6b1b-47f4-b2e9-30b477b5ab3d", "vote": 10}]
        },
        {
          "repository": {"name": "web-app", "project": {"name": "Fabrikam Fiber"}},
          "pullRequestId": 24,
          "creationDate": "2026-10-09T16:30:31Z",
          "title": "Waiting for author",
          "isDraft": false,
          "reviewers": [{"id": "d6245f20", "vote": -5}]
        }
      ],
      "count": 3
    }"#;

    const USER_ID: &str = "8c8c7d32-6b1b-47f4-b2e9-30b477b5ab3d";

    fn work_items() -> Vec<AzWorkItem> {
        let list: AzList = parse_json(BATCH, SERVICE).expect("batch");
        parse_each(list.value)
    }

    #[test]
    fn connection_data_names_the_user() {
        let connection: AzConnection = parse_json(CONNECTION, SERVICE).expect("connection");
        assert_eq!(connection.authenticated_user.id, USER_ID);
        assert_eq!(
            connection
                .authenticated_user
                .provider_display_name
                .as_deref(),
            Some("Jan Kowalski")
        );
        let wiql: AzWiql = parse_json(WIQL, SERVICE).expect("wiql");
        let ids: Vec<u64> = wiql.work_items.iter().map(|w| w.id).collect();
        assert_eq!(ids, vec![300, 299]);
    }

    #[test]
    fn work_items_map_with_priority_dates_and_iteration() {
        let tz = cest();
        let items = work_items();
        assert_eq!(items.len(), 2, "omitted (null) items are skipped");
        assert_eq!(distinct_projects(&items), vec!["Fabrikam Fiber".to_owned()]);
        let iterations_list: AzList = parse_json(ITERATIONS, SERVICE).expect("iterations");
        let mut iterations = HashMap::new();
        iterations.insert(
            "Fabrikam Fiber".to_owned(),
            current_iteration_path(iterations_list).expect("current"),
        );
        let mapped: Vec<WorkItem> = items
            .iter()
            .filter_map(|wi| work_item(wi, "firma", &iterations, &tz))
            .collect();

        let first = &mapped[0];
        assert_eq!(first.id, "azure-devops:#300");
        assert_eq!(first.key, "#300");
        assert_eq!(first.title, "Payment retries");
        assert_eq!(
            first.url,
            "https://dev.azure.com/firma/Fabrikam%20Fiber/_workitems/edit/300"
        );
        assert_eq!(first.project.as_deref(), Some("Fabrikam Fiber"));
        assert_eq!(first.priority_rank, Some(0));
        assert_eq!(first.priority_label.as_deref(), Some("P1"));
        assert_eq!(first.status.as_deref(), Some("Active"));
        // Local midnight of the 15th in UTC+2, stored as 22:00 UTC on the 14th.
        assert_eq!(first.due_ms, Some(end_of_day(2026, 10, 15, &tz)));
        assert!(first.in_sprint, "Sprint 3 lies under the current iteration");
        assert_eq!(first.updated_ms, utc(2026, 10, 12, 14, 3) + 12_567);

        let second = &mapped[1];
        assert_eq!(second.priority_rank, Some(2));
        assert_eq!(second.due_ms, Some(end_of_day(2026, 10, 20, &tz)));
        assert!(
            !second.in_sprint,
            "the project root is not the current iteration"
        );
    }

    #[test]
    fn projects_are_sorted_by_activity() {
        let list: AzList = parse_json(PROJECTS, SERVICE).expect("projects");
        assert_eq!(
            project_names(parse_each(list.value)),
            vec!["Fabrikam Fiber".to_owned(), "Old Project".to_owned()]
        );
    }

    #[test]
    fn pull_requests_waiting_for_the_user() {
        let list: AzList = parse_json(PULLS, SERVICE).expect("pulls");
        let prs: Vec<AzPullRequest> = parse_each(list.value);
        assert!(awaits_vote(&prs[0], USER_ID));
        assert!(!awaits_vote(&prs[1], USER_ID), "already approved");
        assert!(awaits_vote(&prs[2], USER_ID), "requested through a team");
        assert!(!changes_requested(&prs[0]));
        assert!(changes_requested(&prs[2]));

        let item = pr_item(&prs[0], WorkItemKind::ReviewRequested, "firma").expect("item");
        assert_eq!(item.id, "azure-devops:web-app!22");
        assert_eq!(
            item.url,
            "https://dev.azure.com/firma/Fabrikam%20Fiber/_git/web-app/pullrequest/22"
        );
        assert_eq!(item.project.as_deref(), Some("Fabrikam Fiber"));
        assert_eq!(item.updated_ms, utc(2026, 10, 11, 16, 30) + 31_665);

        let mut draft = AzPullRequest {
            is_draft: true,
            ..AzPullRequest::default()
        };
        assert!(!awaits_vote(&draft, USER_ID));
        draft.is_draft = false;
        draft.reviewers.push(AzReviewer {
            id: USER_ID.to_uppercase(),
            vote: 0,
            has_declined: true,
        });
        assert!(!awaits_vote(&draft, USER_ID), "declined");
    }

    #[test]
    fn urls_keep_dollar_parameters_and_encode_segments() {
        let client = AzureDevopsClient::new("firma", "pat".into()).expect("client");
        let url = client
            .url(
                &["My Project", "_apis", "git", "pullrequests"],
                &[("searchCriteria.reviewerId", USER_ID)],
                "$top=50",
            )
            .expect("url");
        assert_eq!(
            url.as_str(),
            format!(
                "https://dev.azure.com/firma/My%20Project/_apis/git/pullrequests?searchCriteria.reviewerId={USER_ID}&api-version=7.1&$top=50"
            )
        );
        let unversioned = client
            .versioned_url(&["_apis", "connectionData"], &[], "", None)
            .expect("url");
        assert_eq!(
            unversioned.as_str(),
            "https://dev.azure.com/firma/_apis/connectionData"
        );
        let raw_only = client
            .versioned_url(&["_apis", "wit", "wiql"], &[], "$top=50", None)
            .expect("url");
        assert_eq!(
            raw_only.as_str(),
            "https://dev.azure.com/firma/_apis/wit/wiql?$top=50"
        );
        for bad in ["", "..", "a/b", "a b", &"x".repeat(65)] {
            assert!(
                matches!(
                    AzureDevopsClient::new(bad, "pat".into()),
                    Err(PlatformError::Invalid(_))
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn sign_in_pages_are_auth_errors() {
        let reply = Reply {
            status: 203,
            headers: reqwest::header::HeaderMap::new(),
            body: "<!DOCTYPE html><html>Sign in</html>".into(),
        };
        assert!(matches!(
            AzureDevopsClient::body(reply),
            Err(PlatformError::Auth(_))
        ));
        let html_200 = Reply {
            status: 200,
            headers: reqwest::header::HeaderMap::new(),
            body: " <html></html>".into(),
        };
        assert!(matches!(
            AzureDevopsClient::body(html_200),
            Err(PlatformError::Auth(_))
        ));
        let ok = Reply {
            status: 200,
            headers: reqwest::header::HeaderMap::new(),
            body: "{}".into(),
        };
        assert_eq!(AzureDevopsClient::body(ok).expect("ok"), "{}");
    }
}
