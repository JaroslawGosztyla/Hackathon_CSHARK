//! Developer tools for "Co dalej?": what the user can do next, read-only, from the tools they connect.
//!
//! One client per source, each behind [`WorkProvider`] (build one with [`provider`]):
//!
//! * [`github`]: issues assigned to the user, pull requests waiting for their review, their own pull requests with
//!   changes requested or failing checks, and participating notifications (polled with `If-Modified-Since`, the
//!   server's `X-Poll-Interval` is passed on).
//! * [`gitlab`]: assigned issues, merge requests waiting for the user's review, their own merge requests with
//!   changes requested or a failed pipeline, and pending to-do items (any GitLab instance).
//! * [`jira`]: assigned unresolved issues with priority, due date, status and the active sprint, plus watched
//!   issues updated in the last day; Jira Cloud (e-mail + API token, also scoped tokens through Atlassian's API
//!   gateway) and Data Center (personal access token).
//! * [`azure_devops`]: work items assigned to the user (WIQL) with the current team iteration, and pull requests
//!   waiting for their vote or with changes requested on their own.
//! * [`linear`]: assigned open issues with priority, due date and the active cycle, plus unread notifications.
//!
//! Every client returns [`WorkFetch::items`] as contract [`WorkItem`]s with `score: 0`, no `reasons` and
//! `unseen: false`: ranking, dedupe and display cleaning are `focus_core::work::rank`'s job, "unseen" is the
//! shell's. Item ids are `<source>:<key>` (e.g. `github:owner/repo#45`), so an assigned issue that is also a
//! notification merges into one item. Date-only due dates are the last millisecond of that day in local time.
//!
//! Tokens are read-only personal tokens stored DPAPI-encrypted ([`save_token`] / [`load_token`], one file per
//! source, [`token_path_for`]); they are never logged, never put into URLs and never part of an error message.
//! Requests go only to the tool the user connected (HTTPS, redirects only within the same origin). All calls are
//! **blocking** (10 s timeouts): use the clients on a worker thread, never inside an async runtime.

pub mod azure_devops;
pub mod github;
pub mod gitlab;
pub mod jira;
pub mod linear;

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use focus_core::api::{WorkConfig, WorkItem, WorkItemKind, WorkSource};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

use crate::calendar::TimeZone;
use crate::calendar::time::{MS_PER_DAY, parse_iso_date, parse_rfc3339};
use crate::net::network_error;
use crate::{PlatformError, secrets};

/// Message of every [`PlatformError::Auth`] from a work source (HTTP 401 / 403).
pub const AUTH_ERROR: &str = "token invalid or missing scope";
/// Longest token accepted (characters).
pub const MAX_TOKEN_LEN: usize = 4096;
/// Largest response body read (bytes); a larger one is an error.
const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
/// Redirects followed (same origin only).
pub(crate) const MAX_REDIRECTS: usize = 3;

/// Result of one [`WorkProvider::fetch`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkFetch {
    /// Signed-in user (login / display name), if known.
    pub account: Option<String>,
    /// Unranked items (`score` 0, no `reasons`, `unseen` false).
    pub items: Vec<WorkItem>,
    /// The server asks not to poll more often than this (GitHub `X-Poll-Interval`), in seconds.
    pub poll_interval_secs: Option<u32>,
    /// Optional parts that failed while the main list worked (English, no secrets, no URLs), e.g. GitHub
    /// notifications with a fine-grained token. Meant for the log and the settings detail.
    pub warnings: Vec<String>,
}

/// One connected tool. `fetch` is blocking (10 s per request); keep the provider between fetches (it caches
/// e.g. GitHub's `Last-Modified` and Jira's sprint field ids).
pub trait WorkProvider: Send {
    fn source(&self) -> WorkSource;
    /// Everything assigned to / waiting on the user now. `Err(Auth)` = the token is invalid or lacks a scope;
    /// `Err(Network)` = transport, HTTP or response errors; `Err(Invalid)` = unusable configuration.
    fn fetch(&mut self, now_ms: i64) -> Result<WorkFetch, PlatformError>;
}

/// The client of `source` with the settings of `config` and `token` (no I/O). `Invalid` when the token or the
/// configuration (Jira address, Azure DevOps organisation, an address that is not HTTPS) cannot be used.
pub fn provider(
    source: WorkSource,
    config: &WorkConfig,
    token: &str,
) -> Result<Box<dyn WorkProvider>, PlatformError> {
    let token = normalize_token(token)?;
    Ok(match source {
        WorkSource::Github => Box::new(github::GithubClient::new(token)),
        WorkSource::Gitlab => Box::new(gitlab::GitlabClient::new(&config.gitlab_url, token)?),
        WorkSource::Jira => Box::new(jira::JiraClient::new(
            &config.jira_url,
            &config.jira_email,
            token,
        )?),
        WorkSource::AzureDevops => Box::new(azure_devops::AzureDevopsClient::new(
            &config.azure_org,
            token,
        )?),
        WorkSource::Linear => Box::new(linear::LinearClient::new(token)),
    })
}

// -- token storage -------------------------------------------------------------------------------------------

/// File name of the encrypted token of `source` (inside the app data folder).
pub fn token_file_name(source: WorkSource) -> &'static str {
    match source {
        WorkSource::Github => "work-github-token.bin",
        WorkSource::Gitlab => "work-gitlab-token.bin",
        WorkSource::Jira => "work-jira-token.bin",
        WorkSource::AzureDevops => "work-azure-devops-token.bin",
        WorkSource::Linear => "work-linear-token.bin",
    }
}

/// Where the encrypted token of `source` lives: `dir` / [`token_file_name`].
pub fn token_path_for(dir: &Path, source: WorkSource) -> PathBuf {
    dir.join(token_file_name(source))
}

/// The token as stored and sent: trimmed, 1..=[`MAX_TOKEN_LEN`] visible ASCII characters (every tool's tokens
/// are). The error never repeats the value.
pub fn normalize_token(raw: &str) -> Result<String, PlatformError> {
    let token = raw.trim();
    if token.is_empty() {
        return Err(PlatformError::Invalid("the token is empty".into()));
    }
    if token.len() > MAX_TOKEN_LEN {
        return Err(PlatformError::Invalid("the token is too long".into()));
    }
    if !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(PlatformError::Invalid(
            "the token contains spaces or characters that are not allowed".into(),
        ));
    }
    Ok(token.to_owned())
}

/// Encrypts ([`normalize_token`]ed) `token` into `path` (DPAPI; `Unsupported` where encryption is unavailable,
/// a token is never written in plain text).
pub fn save_token(path: &Path, token: &str) -> Result<(), PlatformError> {
    let token = normalize_token(token)?;
    secrets::write_sealed(path, token.as_bytes())
}

/// The stored token; `Ok(None)` when there is none. A file that does not decrypt to a valid token is an error
/// (`Io`), without its contents.
pub fn load_token(path: &Path) -> Result<Option<String>, PlatformError> {
    let Some(bytes) = secrets::read_sealed(path)? else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| PlatformError::Io("the stored token is unreadable".into()))?;
    normalize_token(&text)
        .map(Some)
        .map_err(|_| PlatformError::Io("the stored token is unreadable".into()))
}

/// Forgets the stored token (a missing file is fine).
pub fn delete_token(path: &Path) -> Result<(), PlatformError> {
    secrets::delete_sealed(path)
}

/// Token wrapper whose `Debug` never shows the value.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Token(String);

impl Token {
    pub(crate) fn new(token: String) -> Self {
        Self(token)
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    /// The token as a header value marked sensitive (left out of reqwest's debug output).
    pub(crate) fn header(&self, prefix: &str) -> Result<HeaderValue, PlatformError> {
        let mut value = HeaderValue::from_str(&format!("{prefix}{}", self.0)).map_err(|_| {
            PlatformError::Invalid("the token contains characters that are not allowed".into())
        })?;
        value.set_sensitive(true);
        Ok(value)
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

// -- HTTP ----------------------------------------------------------------------------------------------------

/// A response read into memory.
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) headers: HeaderMap,
    pub(crate) body: String,
}

impl Reply {
    pub(crate) fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The body when the status is 2xx, else the mapped error ([`status_error`]).
    pub(crate) fn into_body(self, service: &str) -> Result<String, PlatformError> {
        if self.is_success() {
            Ok(self.body)
        } else {
            Err(status_error(self.status, &self.headers, service))
        }
    }
}

/// Sends the request and reads at most [`MAX_BODY_BYTES`]. Only transport errors are errors here; the status is
/// the caller's to check. No error text contains the URL.
pub(crate) fn send(request: reqwest::blocking::RequestBuilder) -> Result<Reply, PlatformError> {
    let response = request.send().map_err(network_error)?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let mut bytes = Vec::new();
    response
        .take(MAX_BODY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| {
            PlatformError::Network(format!("reading the response failed ({:?})", err.kind()))
        })?;
    if bytes.len() as u64 > MAX_BODY_BYTES {
        return Err(PlatformError::Network("the response is too large".into()));
    }
    Ok(Reply {
        status,
        headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

/// A non-2xx status → error: 401 / 403 → `Auth(`[`AUTH_ERROR`]`)` (except rate limits), everything else
/// `Network` with the service name and the status (never the URL or the body).
pub(crate) fn status_error(status: u16, headers: &HeaderMap, service: &str) -> PlatformError {
    if is_rate_limited(status, headers) {
        return PlatformError::Network(format!(
            "{service}: rate limited (HTTP {status}), try again later"
        ));
    }
    match status {
        401 | 403 => PlatformError::Auth(AUTH_ERROR.into()),
        300..=399 => PlatformError::Network(format!(
            "{service}: unexpected redirect (HTTP {status}), check the address"
        )),
        404 => PlatformError::Network(format!(
            "{service}: not found (HTTP 404), check the address"
        )),
        _ => PlatformError::Network(format!("{service}: HTTP {status}")),
    }
}

/// 429, or a 403 that GitHub (and others) send when a rate limit is exhausted.
fn is_rate_limited(status: u16, headers: &HeaderMap) -> bool {
    let remaining_zero = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == "0");
    status == 429 || (status == 403 && (remaining_zero || headers.contains_key("retry-after")))
}

/// JSON body → `T`; a parse error names only the position (no response text).
pub(crate) fn parse_json<T: DeserializeOwned>(
    body: &str,
    service: &str,
) -> Result<T, PlatformError> {
    serde_json::from_str(body).map_err(|err| {
        PlatformError::Network(format!(
            "{service}: unexpected response (line {}, column {})",
            err.line(),
            err.column()
        ))
    })
}

/// Each element of a JSON array that parses as `T` (a malformed element is skipped, not fatal).
pub(crate) fn parse_each<T: DeserializeOwned>(values: Vec<serde_json::Value>) -> Vec<T> {
    values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect()
}

/// `base` (an `https://` address, path allowed) checked and without trailing slashes.
pub(crate) fn https_base(raw: &str, what: &str) -> Result<String, PlatformError> {
    let trimmed = raw.trim().trim_end_matches('/');
    let url = reqwest::Url::parse(trimmed)
        .map_err(|_| PlatformError::Invalid(format!("the {what} address is not valid")))?;
    if url.scheme() != "https"
        || url.host_str().is_none_or(str::is_empty)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(PlatformError::Invalid(format!(
            "the {what} address must be an https:// address"
        )));
    }
    Ok(trimmed.to_owned())
}

/// `base` + `path` (already safe) + query pairs (encoded here).
pub(crate) fn build_url(
    base: &str,
    path: &str,
    query: &[(&str, &str)],
) -> Result<reqwest::Url, PlatformError> {
    let mut url = reqwest::Url::parse(&format!("{base}{path}"))
        .map_err(|_| PlatformError::Invalid("cannot build the request address".into()))?;
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    Ok(url)
}

/// Appends percent-encoded path segments to `url`.
pub(crate) fn push_segments(
    url: &mut reqwest::Url,
    segments: &[&str],
) -> Result<(), PlatformError> {
    url.path_segments_mut()
        .map_err(|_| PlatformError::Invalid("cannot build the request address".into()))?
        .pop_if_empty()
        .extend(segments);
    Ok(())
}

// -- items ---------------------------------------------------------------------------------------------------

/// Prefix of item ids.
pub(crate) fn source_prefix(source: WorkSource) -> &'static str {
    match source {
        WorkSource::Github => "github",
        WorkSource::Gitlab => "gitlab",
        WorkSource::Jira => "jira",
        WorkSource::AzureDevops => "azure-devops",
        WorkSource::Linear => "linear",
    }
}

/// Stable item id: `<source>:<key>`.
pub(crate) fn item_id(source: WorkSource, key: &str) -> String {
    format!("{}:{key}", source_prefix(source))
}

/// An item with id `<source>:<key>` and every optional fact empty.
pub(crate) fn new_item(
    source: WorkSource,
    kind: WorkItemKind,
    key: String,
    title: String,
    url: String,
    updated_ms: i64,
) -> WorkItem {
    WorkItem {
        id: item_id(source, &key),
        source,
        kind,
        key,
        title,
        url,
        project: None,
        priority_label: None,
        priority_rank: None,
        due_ms: None,
        status: None,
        in_sprint: false,
        updated_ms,
        score: 0,
        reasons: Vec::new(),
        unseen: false,
    }
}

/// Non-empty trimmed text.
pub(crate) fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

// -- time ----------------------------------------------------------------------------------------------------

/// An instant as the tools send it → UTC ms: RFC 3339 (`Z`, `+02:00`, fractions of any length) and Jira's
/// `+0200` offsets.
pub(crate) fn parse_instant(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Some(ms) = parse_rfc3339(text) {
        return Some(ms);
    }
    // Jira: `2026-10-12T09:15:30.123+0200` (offset without a colon).
    let bytes = text.as_bytes();
    if text.is_ascii() && text.len() > 5 {
        let (head, offset) = text.split_at(text.len() - 5);
        let ob = offset.as_bytes();
        if matches!(ob[0], b'+' | b'-')
            && ob[1..].iter().all(u8::is_ascii_digit)
            && bytes[text.len() - 6].is_ascii_digit()
        {
            return parse_rfc3339(&format!("{head}{}:{}", &offset[..3], &offset[3..]));
        }
    }
    None
}

/// The last millisecond of `days` (days since 1970-01-01) in `tz`.
pub(crate) fn end_of_local_day(days: i64, tz: &TimeZone) -> i64 {
    tz.wall_to_utc(
        days.saturating_mul(MS_PER_DAY)
            .saturating_add(MS_PER_DAY - 1),
    )
}

/// A date-only due date (`YYYY-MM-DD`, or the date part of `YYYY-MM-DDT…`) → last millisecond of that day in
/// `tz`: due today all day, overdue only once the day is over.
pub(crate) fn due_end_of_day(date: &str, tz: &TimeZone) -> Option<i64> {
    let date = date.trim();
    let date = match date.get(..10) {
        Some(head) if date.len() == 10 || date[10..].starts_with(['T', 't', ' ']) => head,
        _ => return None,
    };
    parse_iso_date(date).map(|days| end_of_local_day(days, tz))
}

/// Day number of the local date of `utc_ms` in `tz`.
pub(crate) fn local_day(utc_ms: i64, tz: &TimeZone) -> i64 {
    tz.utc_to_wall(utc_ms).div_euclid(MS_PER_DAY)
}

/// A date the tool stores as a date-time though users pick only a day (Azure DevOps due / target dates):
/// exactly midnight UTC means that UTC date, any other time the local date of the instant (the web UI stores
/// local midnight). → last millisecond of that day in `tz`.
pub(crate) fn date_like_due(text: &str, tz: &TimeZone) -> Option<i64> {
    let utc = parse_instant(text)?;
    let day = if utc.rem_euclid(MS_PER_DAY) == 0 {
        utc.div_euclid(MS_PER_DAY)
    } else {
        local_day(utc, tz)
    };
    Some(end_of_local_day(day, tz))
}

/// `true` when the local date of `now_ms` lies within `[start, end]` (`YYYY-MM-DD`, inclusive).
pub(crate) fn today_within(
    start: Option<&str>,
    end: Option<&str>,
    now_ms: i64,
    tz: &TimeZone,
) -> bool {
    let (Some(start), Some(end)) = (
        start.and_then(|d| parse_iso_date(d.get(..10)?)),
        end.and_then(|d| parse_iso_date(d.get(..10)?)),
    ) else {
        return false;
    };
    let today = local_day(now_ms, tz);
    start <= today && today <= end
}

// -- priority and status from names / labels ------------------------------------------------------------------

/// Words that name a priority, with the normalised rank (0 = highest ..= 4 = lowest).
const PRIORITY_WORDS: &[(&str, u32)] = &[
    ("highest", 0),
    ("blocker", 0),
    ("critical", 0),
    ("urgent", 0),
    ("showstopper", 0),
    ("emergency", 0),
    ("asap", 0),
    ("high", 1),
    ("major", 1),
    ("important", 1),
    ("medium", 2),
    ("normal", 2),
    ("moderate", 2),
    ("mid", 2),
    ("low", 3),
    ("minor", 3),
    ("lowest", 4),
    ("trivial", 4),
];

/// Label prefixes that introduce a priority value (`priority::1`, `prio: high`, `pri/low`), longest first.
const PRIORITY_PREFIXES: &[&str] = &["priority", "prio", "pri"];

/// Normalised rank of a priority name as the tool shows it: `P0`..`P4`, a leading digit (`1 - High`), or a
/// priority word (`Highest`, `Blocker`, `Critical`, `High`, `Major`, `Medium`, `Low`, `Minor`, `Lowest`,
/// `Trivial`, …). Digits above 4 are clamped. `None` when nothing matches.
pub fn priority_rank_from_name(name: &str) -> Option<u32> {
    let lower = name.trim().to_lowercase();
    let value = lower.trim_start_matches(['#', ' ']);
    let digits = value.strip_prefix('p').unwrap_or(value);
    let leading: String = digits.chars().take_while(char::is_ascii_digit).collect();
    if !leading.is_empty() && leading.len() <= 2 {
        let rest = &digits[leading.len()..];
        if rest.chars().next().is_none_or(|c| !c.is_alphanumeric()) {
            return leading.parse::<u32>().ok().map(|d| d.min(4));
        }
    }
    value
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .find_map(|word| {
            PRIORITY_WORDS
                .iter()
                .find(|(name, _)| *name == word)
                .map(|&(_, rank)| rank)
        })
}

/// Priority named by one label: `P0`..`P4`, `priority::1`, `priority: high`, `prio/low`, `high priority`,
/// `priority-critical`, or the bare `critical` / `urgent` / `blocker`. `None` for other labels.
pub fn label_priority(label: &str) -> Option<u32> {
    let lower = label.trim().to_lowercase();
    for prefix in PRIORITY_PREFIXES {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let value = rest.trim_start_matches([':', '/', '-', '_', ' ', '=', '.']);
            let had_separator = value.len() < rest.len();
            if !value.is_empty()
                && (had_separator || value.starts_with(|c: char| c.is_ascii_digit()))
            {
                return priority_rank_from_name(value);
            }
            return None;
        }
    }
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    match words.as_slice() {
        [single] if single.len() == 2 && single.starts_with('p') => {
            single[1..].parse::<u32>().ok().map(|d| d.min(4))
        }
        [single] if matches!(*single, "critical" | "urgent" | "blocker" | "showstopper") => Some(0),
        [value, "priority"] => priority_rank_from_name(value),
        _ => None,
    }
}

/// The highest priority among `labels`: (the label as written, rank).
pub(crate) fn priority_from_labels<'a>(
    labels: impl IntoIterator<Item = &'a str>,
) -> Option<(String, u32)> {
    labels
        .into_iter()
        .filter_map(|label| label_priority(label).map(|rank| (label.trim().to_owned(), rank)))
        .min_by_key(|(_, rank)| *rank)
}

/// The first label that means "in progress" (`In Progress`, `workflow::in progress`, `doing`, `WIP`, …).
pub(crate) fn status_from_labels<'a>(labels: impl IntoIterator<Item = &'a str>) -> Option<String> {
    labels
        .into_iter()
        .find(|label| focus_core::work::is_in_progress(label))
        .map(|label| label.trim().to_owned())
}

#[cfg(test)]
pub(crate) mod test_support {
    use crate::calendar::TimeZone;
    use crate::calendar::time::{MS_PER_DAY, MS_PER_HOUR, days_from_civil};

    /// UTC ms of a civil date and time.
    pub(crate) fn utc(year: i64, month: u32, day: u32, hour: i64, minute: i64) -> i64 {
        days_from_civil(year, month, day) * MS_PER_DAY + hour * MS_PER_HOUR + minute * 60_000
    }

    /// Warsaw in summer (UTC+2, fixed: enough for October samples before the switch).
    pub(crate) fn cest() -> TimeZone {
        TimeZone::fixed(120)
    }

    /// The last millisecond of the date in `tz`.
    pub(crate) fn end_of_day(year: i64, month: u32, day: u32, tz: &TimeZone) -> i64 {
        tz.wall_to_utc(days_from_civil(year, month, day) * MS_PER_DAY + MS_PER_DAY - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{cest, end_of_day, utc};
    use super::*;

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "focusmode-test-{}-{nanos}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn token_files_are_distinct_per_source() {
        let sources = [
            WorkSource::Github,
            WorkSource::Gitlab,
            WorkSource::Jira,
            WorkSource::AzureDevops,
            WorkSource::Linear,
        ];
        let names: std::collections::HashSet<&str> =
            sources.iter().map(|&s| token_file_name(s)).collect();
        assert_eq!(names.len(), sources.len());
        assert!(
            names
                .iter()
                .all(|n| n.starts_with("work-") && n.ends_with("-token.bin"))
        );
        let dir = Path::new("data");
        assert_eq!(
            token_path_for(dir, WorkSource::Linear),
            dir.join("work-linear-token.bin")
        );
    }

    #[test]
    fn tokens_are_normalized_without_echo() {
        assert_eq!(
            normalize_token("  ghp_abcDEF123  \n").expect("valid"),
            "ghp_abcDEF123"
        );
        for bad in ["", "   ", "two words", "tab\there", "zażółć", "line\nbreak"] {
            let err = normalize_token(bad).expect_err(bad);
            assert!(matches!(err, PlatformError::Invalid(_)), "{bad:?}");
            if !bad.trim().is_empty() {
                assert!(!err.to_string().contains(bad.trim()), "{bad:?} echoed");
            }
        }
        assert!(normalize_token(&"a".repeat(MAX_TOKEN_LEN)).is_ok());
        assert!(normalize_token(&"a".repeat(MAX_TOKEN_LEN + 1)).is_err());
    }

    #[test]
    fn token_debug_is_redacted() {
        let token = Token::new("glpat-secret".into());
        assert_eq!(format!("{token:?}"), "<redacted>");
        let header = token.header("Bearer ").expect("header");
        assert!(header.is_sensitive());
        assert_eq!(header.to_str().expect("ascii"), "Bearer glpat-secret");
    }

    #[test]
    fn missing_token_file_is_none() {
        let path = unique_temp_path("work-missing-token.bin");
        assert!(matches!(load_token(&path), Ok(None)));
        delete_token(&path).expect("delete missing");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn token_round_trip_is_encrypted() {
        let path = unique_temp_path("work-github-token.bin");
        save_token(&path, "  github_pat_11ABCDEFG0123456789_secretvalue ").expect("save");
        let raw = std::fs::read(&path).expect("raw");
        assert!(!String::from_utf8_lossy(&raw).contains("secretvalue"));
        assert_eq!(
            load_token(&path).expect("load").as_deref(),
            Some("github_pat_11ABCDEFG0123456789_secretvalue")
        );
        assert!(matches!(
            save_token(&path, "has space"),
            Err(PlatformError::Invalid(_))
        ));
        std::fs::write(&path, b"garbage").expect("corrupt");
        assert!(load_token(&path).is_err());
        delete_token(&path).expect("delete");
        assert!(matches!(load_token(&path), Ok(None)));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn token_is_never_stored_in_plain_text() {
        let path = unique_temp_path("work-github-token.bin");
        assert!(matches!(
            save_token(&path, "ghp_x"),
            Err(PlatformError::Unsupported)
        ));
        assert!(!path.exists());
    }

    #[test]
    fn statuses_map_to_errors() {
        let empty = HeaderMap::new();
        for status in [401, 403] {
            match status_error(status, &empty, "GitHub") {
                PlatformError::Auth(message) => assert_eq!(message, AUTH_ERROR),
                other => panic!("{status}: {other:?}"),
            }
        }
        let mut limited = HeaderMap::new();
        limited.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        assert!(matches!(
            status_error(403, &limited, "GitHub"),
            PlatformError::Network(ref m) if m.contains("rate limited")
        ));
        let mut retry = HeaderMap::new();
        retry.insert("retry-after", HeaderValue::from_static("60"));
        assert!(matches!(
            status_error(403, &retry, "GitHub"),
            PlatformError::Network(ref m) if m.contains("rate limited")
        ));
        assert!(matches!(
            status_error(429, &empty, "Jira"),
            PlatformError::Network(ref m) if m.contains("rate limited")
        ));
        assert!(matches!(
            status_error(404, &empty, "GitLab"),
            PlatformError::Network(ref m) if m.contains("GitLab") && m.contains("404")
        ));
        assert!(matches!(
            status_error(302, &empty, "GitLab"),
            PlatformError::Network(ref m) if m.contains("redirect")
        ));
        assert!(matches!(
            status_error(500, &empty, "Linear"),
            PlatformError::Network(ref m) if m == "Linear: HTTP 500"
        ));
    }

    #[test]
    fn json_errors_do_not_echo_the_body() {
        let err = parse_json::<Vec<u32>>("{\"secret\": \"value\"}", "Jira").expect_err("object");
        let text = err.to_string();
        assert!(text.contains("Jira") && text.contains("line 1"), "{text}");
        assert!(
            !text.contains("secret") && !text.contains("value"),
            "{text}"
        );
        let parsed: Vec<u32> = parse_each(vec![
            serde_json::json!(1),
            serde_json::json!("x"),
            serde_json::json!(3),
        ]);
        assert_eq!(parsed, vec![1, 3]);
    }

    #[test]
    fn base_addresses_must_be_https() {
        assert_eq!(
            https_base(" https://gitlab.example.com/gitlab/ ", "GitLab").expect("ok"),
            "https://gitlab.example.com/gitlab"
        );
        for bad in [
            "http://gitlab.example.com",
            "gitlab.example.com",
            "https://user:pw@gitlab.example.com",
            "https://gitlab.example.com/?x=1",
            "",
        ] {
            assert!(
                matches!(https_base(bad, "GitLab"), Err(PlatformError::Invalid(_))),
                "{bad}"
            );
        }
        let mut url = build_url("https://dev.azure.com", "/firma", &[("$top", "50")]).expect("url");
        push_segments(&mut url, &["My Project", "_apis"]).expect("segments");
        assert_eq!(
            url.as_str(),
            "https://dev.azure.com/firma/My%20Project/_apis?%24top=50"
        );
    }

    #[test]
    fn instants_parse_in_every_tool_format() {
        let expected = utc(2026, 10, 12, 7, 15) + 30_000;
        for text in [
            "2026-10-12T07:15:30Z",
            "2026-10-12T07:15:30.000Z",
            "2026-10-12T09:15:30.000+0200",
            "2026-10-12T09:15:30+02:00",
            "2026-10-12T00:15:30.0000000-0700",
            "2026-10-12T07:15:30.1234567Z",
        ] {
            let parsed = parse_instant(text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(parsed - parsed % 1000, expected, "{text}");
        }
        assert_eq!(
            parse_instant("2026-10-12T09:15:30.123+0200"),
            Some(expected + 123)
        );
        for bad in ["", "yesterday", "2026-10-12", "2026-10-12T09:15:30+2"] {
            assert_eq!(parse_instant(bad), None, "{bad}");
        }
    }

    #[test]
    fn date_only_due_dates_end_with_the_local_day() {
        let tz = cest();
        let due = due_end_of_day("2026-10-15", &tz).expect("date");
        assert_eq!(due, end_of_day(2026, 10, 15, &tz));
        // 23:59:59.999 in UTC+2 is 21:59:59.999 UTC.
        assert_eq!(due, utc(2026, 10, 15, 21, 59) + 59_999);
        assert_eq!(due_end_of_day("2026-10-15T07:00:00Z", &tz), Some(due));
        assert_eq!(due_end_of_day("2026-13-01", &tz), None);
        assert_eq!(due_end_of_day("2026-10-15junk", &tz), None);
        assert_eq!(due_end_of_day("", &tz), None);
    }

    #[test]
    fn date_like_due_dates_keep_the_picked_day() {
        let tz = cest();
        // Stored as midnight UTC (API, iteration style).
        assert_eq!(
            date_like_due("2026-10-15T00:00:00Z", &tz),
            Some(end_of_day(2026, 10, 15, &tz))
        );
        // Stored as local midnight by the web UI (UTC+2 → 22:00 UTC the day before).
        assert_eq!(
            date_like_due("2026-10-14T22:00:00Z", &tz),
            Some(end_of_day(2026, 10, 15, &tz))
        );
        let west = TimeZone::fixed(-300);
        assert_eq!(
            date_like_due("2026-10-15T00:00:00Z", &west),
            Some(end_of_day(2026, 10, 15, &west))
        );
        assert_eq!(date_like_due("soon", &tz), None);
    }

    #[test]
    fn today_within_compares_local_dates() {
        let tz = cest();
        let now = utc(2026, 10, 13, 23, 30); // 01:30 on the 14th in UTC+2
        assert!(today_within(
            Some("2026-10-14"),
            Some("2026-10-27"),
            now,
            &tz
        ));
        assert!(!today_within(
            Some("2026-10-01"),
            Some("2026-10-13"),
            now,
            &tz
        ));
        assert!(today_within(
            Some("2026-10-01T00:00:00Z"),
            Some("2026-10-14"),
            now,
            &tz
        ));
        assert!(!today_within(None, Some("2026-10-27"), now, &tz));
    }

    #[test]
    fn priority_names_are_normalised() {
        let cases = [
            ("Highest", Some(0)),
            ("Blocker", Some(0)),
            ("Critical", Some(0)),
            ("High", Some(1)),
            ("Major", Some(1)),
            ("Medium", Some(2)),
            ("Normal", Some(2)),
            ("Low", Some(3)),
            ("Minor", Some(3)),
            ("Lowest", Some(4)),
            ("Trivial", Some(4)),
            ("P0", Some(0)),
            ("p2", Some(2)),
            ("P7", Some(4)),
            ("1 - Critical", Some(1)),
            ("3", Some(3)),
            ("Must have", None),
            ("", None),
            ("2026 roadmap", None),
        ];
        for (name, expected) in cases {
            assert_eq!(priority_rank_from_name(name), expected, "{name}");
        }
    }

    #[test]
    fn labels_name_priorities_best_effort() {
        let cases = [
            ("P0", Some(0)),
            ("p3", Some(3)),
            ("priority::1", Some(1)),
            ("Priority: High", Some(1)),
            ("priority/low", Some(3)),
            ("prio-critical", Some(0)),
            ("priority_medium", Some(2)),
            ("pri:2", Some(2)),
            ("High priority", Some(1)),
            ("low-priority", Some(3)),
            ("critical", Some(0)),
            ("urgent", Some(0)),
            ("bug", None),
            ("private", None),
            ("priority", None),
            ("prioritized", None),
            ("high", None),
            ("p10", None),
            ("workflow::in progress", None),
        ];
        for (label, expected) in cases {
            assert_eq!(label_priority(label), expected, "{label}");
        }
        assert_eq!(
            priority_from_labels(["bug", "priority: low", "P1", "frontend"]),
            Some(("P1".to_owned(), 1))
        );
        assert_eq!(priority_from_labels(["bug", "frontend"]), None);
    }

    #[test]
    fn status_labels_mean_in_progress() {
        assert_eq!(
            status_from_labels(["bug", "workflow::in progress"]),
            Some("workflow::in progress".to_owned())
        );
        assert_eq!(status_from_labels(["Doing"]), Some("Doing".to_owned()));
        assert_eq!(status_from_labels(["bug", "frontend"]), None);
    }

    #[test]
    fn items_start_unranked() {
        let item = new_item(
            WorkSource::AzureDevops,
            WorkItemKind::Assigned,
            "#42".into(),
            "Fix".into(),
            "https://dev.azure.com/firma/P/_workitems/edit/42".into(),
            5,
        );
        assert_eq!(item.id, "azure-devops:#42");
        assert_eq!((item.score, item.unseen), (0, false));
        assert!(item.reasons.is_empty());
        assert_eq!(non_empty(Some("  ")), None);
        assert_eq!(non_empty(Some(" a ")).as_deref(), Some("a"));
    }

    /// Talks to the real services with an invalid token: every one must answer with an auth error (and never with
    /// a panic, a parse error or the token in the message).
    #[test]
    #[ignore = "network: sends an invalid token to GitHub, GitLab.com, Linear, Azure DevOps and a public Jira Cloud site"]
    fn live_invalid_tokens_are_auth_errors() {
        // Jira Cloud: the site refuses the token, the cloud id is read and the API gateway refuses it too.
        let config = WorkConfig {
            azure_org: "microsoft".into(),
            jira_url: "https://ecosystem.atlassian.net".into(),
            jira_email: "nobody@example.com".into(),
            ..WorkConfig::default()
        };
        let token = "focusmode-invalid-token-0123456789";
        for source in [
            WorkSource::Github,
            WorkSource::Gitlab,
            WorkSource::Linear,
            WorkSource::AzureDevops,
            WorkSource::Jira,
        ] {
            let mut client = provider(source, &config, token).expect("provider");
            let err = client.fetch(0).expect_err("an invalid token must fail");
            assert!(
                matches!(err, PlatformError::Auth(ref m) if m == AUTH_ERROR),
                "{source:?}: {err}"
            );
            assert!(!err.to_string().contains(token));
        }
    }

    #[test]
    fn providers_need_their_settings() {
        let mut config = WorkConfig::default();
        assert!(provider(WorkSource::Github, &config, "ghp_x").is_ok());
        assert!(provider(WorkSource::Linear, &config, "lin_api_x").is_ok());
        assert!(provider(WorkSource::Gitlab, &config, "glpat-x").is_ok());
        assert!(matches!(
            provider(WorkSource::Jira, &config, "x").err(),
            Some(PlatformError::Invalid(_))
        ));
        assert!(matches!(
            provider(WorkSource::AzureDevops, &config, "x").err(),
            Some(PlatformError::Invalid(_))
        ));
        assert!(matches!(
            provider(WorkSource::Github, &config, "  ").err(),
            Some(PlatformError::Invalid(_))
        ));
        config.jira_url = "https://firma.atlassian.net".into();
        config.azure_org = "firma".into();
        let jira = provider(WorkSource::Jira, &config, "x").expect("jira");
        assert_eq!(jira.source(), WorkSource::Jira);
        assert!(provider(WorkSource::AzureDevops, &config, "x").is_ok());
        config.gitlab_url = "http://gitlab.example.com".into();
        assert!(matches!(
            provider(WorkSource::Gitlab, &config, "x").err(),
            Some(PlatformError::Invalid(_))
        ));
    }
}
