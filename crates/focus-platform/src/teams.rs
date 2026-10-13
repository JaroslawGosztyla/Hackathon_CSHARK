//! Microsoft Teams presence "Do not disturb" and the Microsoft 365 / Outlook calendar through Microsoft Graph.
//!
//! * Sign-in: OAuth 2.0 device code flow against the Microsoft identity platform (v2 endpoints) with a
//!   user-registered public client (Azure app registration: "Mobile and desktop applications" platform,
//!   "Allow public client flows" on, delegated `Presence.ReadWrite` + `User.Read`, plus `Calendars.Read` for the
//!   calendar). Scopes: [`login_scopes`]: [`SCOPES`] (with `Calendars.Read`) only when the caller wants the calendar
//!   (the shell: `calendar.microsoft` on at sign-in time), [`PRESENCE_SCOPES`] otherwise, so a Teams-only user never
//!   needs consent for the calendar. A sign-in without `Calendars.Read` (presence only, or made before the calendar
//!   integration) keeps asking for exactly the granted scopes when refreshing (asking for more would fail and drop
//!   the token), so presence keeps working, and [`TeamsClient::calendar_view`] answers
//!   `PlatformError::Auth("reconnect needed …")` until the user signs in again with the calendar.
//! * Calendar: `GET /me/calendarView` ([`TeamsClient::calendar_view`], read-only, times in UTC).
//! * Presence: `POST /me/presence/setUserPreferredPresence` (`DoNotDisturb`, with an expiration so a crashed app
//!   never leaves the user in DND forever) and `POST /me/presence/clearUserPreferredPresence`.
//!   Preferred presence is shown only while the user has a presence session (e.g. Teams is signed in somewhere).
//! * Current presence: `GET /me/presence` ([`TeamsClient::get_presence`], covered by `Presence.ReadWrite`). The shell
//!   reads it before focus: a user who appears offline ([`presence_blocks_dnd`]) must not become visible as
//!   "Do not disturb", and a user who already chose Do not disturb ([`presence_is_user_dnd`]) is left alone
//!   (we neither set nor clear it, because clearing the preferred presence would also drop the user's own choice).
//! * Token storage: JSON `{refreshToken, account, scopes}` encrypted with DPAPI ([`crate::secrets`]) and
//!   written atomically to [`TeamsSettings::token_path`]. Access tokens live only in memory. On non-Windows
//!   platforms storage is [`PlatformError::Unsupported`], so Teams is unavailable there.
//! * Refresh tokens are rotated whenever the token endpoint returns a new one; an `invalid_grant` answer deletes
//!   the stored token (the user has to connect again).
//!
//! All calls are **blocking** (reqwest blocking client, 10 s timeouts): create, use **and drop** [`TeamsClient`]
//! on a worker thread, never inside an async runtime (reqwest's blocking client panics there).
//! Tokens, device codes and refresh tokens are never logged.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Deserializer, Serialize};

use crate::PlatformError;
use crate::calendar::time::parse_iso_naive;
use crate::calendar::{CalEvent, TimeZone, format_utc, looks_online, meetings_only};
use crate::net::{LazyClient, first_line, network_error};

/// Delegated scopes requested at a sign-in that also reads the calendar.
pub const SCOPES: &str = "Presence.ReadWrite User.Read Calendars.Read offline_access";
/// Scopes of a presence-only sign-in (also of sign-ins made before the calendar integration).
pub const PRESENCE_SCOPES: &str = "Presence.ReadWrite User.Read offline_access";

/// Scopes for a new sign-in: [`SCOPES`] when it should also read the calendar, else [`PRESENCE_SCOPES`].
pub fn login_scopes(calendar: bool) -> &'static str {
    if calendar { SCOPES } else { PRESENCE_SCOPES }
}
/// Tenant used when the configured one is empty.
pub const DEFAULT_TENANT: &str = "organizations";
/// Longest Do-not-disturb expiration accepted by Graph (7 days), in minutes.
pub const MAX_DND_MINUTES: u32 = 7 * 24 * 60;

const LOGIN_BASE: &str = "https://login.microsoftonline.com";
const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
/// Default redirect limit of the HTTP client (as before the shared client).
const MAX_REDIRECTS: usize = 10;
/// Granted scopes that allow reading the calendar.
const CALENDAR_SCOPES: &[&str] = &["Calendars.Read", "Calendars.ReadWrite"];
const CALENDAR_SELECT: &str = "subject,start,end,showAs,isCancelled,isAllDay,responseStatus,onlineMeeting,isOnlineMeeting,location";
const CALENDAR_PAGE_SIZE: u32 = 100;
const CALENDAR_MAX_PAGES: usize = 3;
/// `calendarView` times in UTC instead of the mailbox zone.
const PREFER_UTC: &str = "outlook.timezone=\"UTC\"";
/// Access tokens are treated as expired this long before their real expiry.
const EXPIRY_MARGIN: Duration = Duration::from_secs(120);
/// Default access token lifetime when the token endpoint does not say.
const DEFAULT_TOKEN_LIFETIME_SECS: u64 = 3600;
const DEFAULT_POLL_INTERVAL_SECS: u32 = 5;
const DEFAULT_DEVICE_CODE_LIFETIME_SECS: u32 = 900;

/// Connection settings (from config + app data dir).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamsSettings {
    /// Application (client) id of the user's Azure app registration.
    pub client_id: String,
    /// Tenant id / domain, or `organizations` / `common`.
    pub tenant: String,
    /// Where the encrypted refresh token is stored (e.g. `app_data_dir/teams-token.bin`).
    pub token_path: PathBuf,
}

/// A started device code login. Show `user_code` + `verification_uri`, then call
/// [`TeamsClient::poll_login`] every `interval_secs` until it is no longer [`LoginPoll::Pending`].
#[derive(Clone)]
pub struct DeviceLogin {
    pub user_code: String,
    pub verification_uri: String,
    /// Instructions from Microsoft (English).
    pub message: String,
    pub expires_in_secs: u32,
    pub interval_secs: u32,
    device_code: String,
    /// Scopes requested by this sign-in ([`login_scopes`]).
    scopes: &'static str,
    started_at: Instant,
}

impl DeviceLogin {
    /// This sign-in asks for calendar access too.
    pub fn includes_calendar(&self) -> bool {
        calendar_scope_granted(self.scopes)
    }

    /// The code is past its lifetime (polling is pointless).
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() >= Duration::from_secs(u64::from(self.expires_in_secs))
    }
}

impl fmt::Debug for DeviceLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceLogin")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("expires_in_secs", &self.expires_in_secs)
            .field("interval_secs", &self.interval_secs)
            .field("device_code", &"<redacted>")
            .field("scopes", &self.scopes)
            .finish_non_exhaustive()
    }
}

/// Result of one [`TeamsClient::poll_login`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginPoll {
    /// The user has not finished signing in yet; poll again after `interval_secs`.
    Pending,
    /// Signed in; the refresh token is stored.
    Done { account: Option<String> },
    /// The login ended without success (declined, expired, misconfigured app...). English detail for logs/UI.
    Failed(String),
}

/// Presence of the signed-in user as Microsoft Graph reports it (`GET /me/presence`).
///
/// `availability`: `Available`, `AvailableIdle`, `Away`, `BeRightBack`, `Busy`, `BusyIdle`, `DoNotDisturb`,
/// `Offline`, `PresenceUnknown`. `activity`: e.g. `Available`, `InACall`, `InAMeeting`, `Presenting`,
/// `DoNotDisturb`, `Offline`, `OffWork`. Values are kept as Graph sends them (empty when missing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub availability: String,
    pub activity: String,
}

/// The user appears offline in Teams ("Appear offline", or not signed in anywhere): Do not disturb must **not**
/// be set, because it would make an invisible user visible.
pub fn presence_blocks_dnd(presence: &Presence) -> bool {
    presence.availability.trim().eq_ignore_ascii_case("Offline")
}

/// The user is already in Do not disturb (set by themselves, or while presenting / in focus time): do not set it
/// and do not clear it afterwards.
pub fn presence_is_user_dnd(presence: &Presence) -> bool {
    presence
        .availability
        .trim()
        .eq_ignore_ascii_case("DoNotDisturb")
}

/// Graph client for Teams presence and the calendar. Blocking; see the module docs for threading rules.
pub struct TeamsClient {
    settings: TeamsSettings,
    http: LazyClient,
    access: Option<AccessToken>,
}

/// In-memory access token.
struct AccessToken {
    token: String,
    expires_at: Instant,
    /// Scopes granted to the sign-in, as the token endpoint reported them (`None` = not reported).
    scopes: Option<String>,
}

impl TeamsClient {
    /// Cheap: no I/O, no network (the HTTP client is built on first use).
    pub fn new(settings: TeamsSettings) -> Self {
        Self {
            settings,
            http: LazyClient::new(MAX_REDIRECTS),
            access: None,
        }
    }

    /// Settings this client was created with (the shell recreates the client when they change).
    pub fn settings(&self) -> &TeamsSettings {
        &self.settings
    }

    /// A client id is configured.
    pub fn is_configured(&self) -> bool {
        !self.client_id().is_empty()
    }

    /// A stored token exists and can be decrypted (it may still be revoked server-side).
    pub fn has_token(&self) -> bool {
        matches!(token_store::load(&self.settings.token_path), Ok(Some(_)))
    }

    /// Signed-in account (UPN) stored alongside the token.
    pub fn account(&self) -> Option<String> {
        token_store::load(&self.settings.token_path)
            .ok()
            .flatten()
            .and_then(|stored| stored.account)
    }

    /// Whether the stored sign-in allows reading the calendar: `Some(false)` = it was made without
    /// `Calendars.Read` (connect again), `None` = no token, or a sign-in from before the calendar integration whose
    /// scopes were not recorded yet (they are after its next refresh).
    pub fn has_calendar_scope(&self) -> Option<bool> {
        if let Some(scopes) = self.access.as_ref().and_then(|a| a.scopes.as_deref()) {
            return Some(calendar_scope_granted(scopes));
        }
        token_store::load(&self.settings.token_path)
            .ok()
            .flatten()
            .and_then(|stored| stored.scopes)
            .map(|scopes| calendar_scope_granted(&scopes))
    }

    /// Starts a device code login (one HTTP request). `calendar`: also ask for `Calendars.Read`
    /// ([`login_scopes`]); a new sign-in replaces the stored one only when it succeeds.
    pub fn begin_login(&self, calendar: bool) -> Result<DeviceLogin, PlatformError> {
        if !token_store::SUPPORTED {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_configured()?;
        let url = format!(
            "{LOGIN_BASE}/{}/oauth2/v2.0/devicecode",
            tenant_segment(&self.settings.tenant)?
        );
        let scopes = login_scopes(calendar);
        let params = [("client_id", self.client_id()), ("scope", scopes)];
        let response = self
            .http()?
            .post(url.as_str())
            .form(&params)
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().map_err(network_error)?;
        if !status.is_success() {
            return Err(PlatformError::Auth(oauth_error_text(
                status.as_u16(),
                &body,
            )));
        }
        let parsed: DeviceCodeResponse = serde_json::from_str(&body)
            .map_err(|err| PlatformError::Auth(format!("invalid device code response: {err}")))?;
        Ok(device_login_from(parsed, scopes, Instant::now()))
    }

    /// One token request for a pending device login. OAuth outcomes are [`LoginPoll`] values; `Err` means a
    /// transport/storage problem (the caller may retry after `interval_secs`).
    pub fn poll_login(&mut self, login: &DeviceLogin) -> Result<LoginPoll, PlatformError> {
        if login.is_expired() {
            return Ok(LoginPoll::Failed("the sign-in code expired".into()));
        }
        self.ensure_configured()?;
        let url = self.token_url()?;
        let client_id = self.client_id().to_owned();
        let params = [
            ("grant_type", DEVICE_CODE_GRANT),
            ("client_id", client_id.as_str()),
            ("device_code", login.device_code.as_str()),
        ];
        let response = self
            .http()?
            .post(url.as_str())
            .form(&params)
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().map_err(network_error)?;

        if !status.is_success() {
            let error: OAuthErrorResponse = serde_json::from_str(&body).unwrap_or_default();
            if error.error.is_empty() {
                // Not an OAuth answer (proxy/server trouble): transient, the caller may poll again.
                return Err(PlatformError::Network(format!(
                    "token endpoint answered HTTP {}",
                    status.as_u16()
                )));
            }
            return Ok(classify_login_error(&error));
        }

        let tokens: TokenResponse = serde_json::from_str(&body)
            .map_err(|err| PlatformError::Auth(format!("invalid token response: {err}")))?;
        let Some(refresh_token) = tokens.refresh_token.clone().filter(|t| !t.is_empty()) else {
            return Ok(LoginPoll::Failed(
                "no refresh token returned (is offline_access allowed for the app?)".into(),
            ));
        };
        // A device code sign-in grants all requested scopes or fails.
        let scopes = granted_scopes(&tokens).or_else(|| Some(login.scopes.to_owned()));
        self.remember_access(&tokens, scopes.clone());

        let account = match self.fetch_account() {
            Ok(account) => account,
            Err(err) => {
                log::warn!("teams: reading the signed-in account failed: {err}");
                None
            }
        };
        token_store::save(
            &self.settings.token_path,
            &StoredToken {
                refresh_token,
                account: account.clone(),
                scopes,
            },
        )?;
        log::info!("teams: signed in");
        Ok(LoginPoll::Done { account })
    }

    /// Sets the preferred presence to Do not disturb for `minutes` (clamped to 1..=7 days).
    pub fn set_dnd(&mut self, minutes: u32) -> Result<(), PlatformError> {
        let body = serde_json::to_value(PreferredPresence::do_not_disturb(minutes))
            .map_err(|err| PlatformError::Os(format!("presence body: {err}")))?;
        self.graph_post("/me/presence/setUserPreferredPresence", Some(&body))?;
        log::info!("teams: presence set to do not disturb for {minutes} min");
        Ok(())
    }

    /// Clears the preferred presence (Teams goes back to automatic presence).
    pub fn clear_dnd(&mut self) -> Result<(), PlatformError> {
        self.graph_post("/me/presence/clearUserPreferredPresence", None)?;
        log::info!("teams: preferred presence cleared");
        Ok(())
    }

    /// Reads the current presence of the signed-in user (`GET /me/presence`).
    pub fn get_presence(&mut self) -> Result<Presence, PlatformError> {
        let body = self.graph_send(reqwest::Method::GET, "/me/presence", None)?;
        let presence = parse_presence(&body)?;
        log::debug!(
            "teams: presence is {}/{}",
            presence.availability,
            presence.activity
        );
        Ok(presence)
    }

    /// Meetings of the signed-in user's Microsoft 365 / Outlook calendar that overlap `[start_ms, end_ms)`
    /// (`GET /me/calendarView`, UTC, at most 3 pages of 100). Cancelled, declined, all-day and free / working
    /// elsewhere events are left out. Without calendar permission (an older sign-in, or a revoked consent) the
    /// error is `PlatformError::Auth("reconnect needed: …")`; presence keeps working with the same sign-in.
    pub fn calendar_view(
        &mut self,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<CalEvent>, PlatformError> {
        if end_ms <= start_ms {
            return Ok(Vec::new());
        }
        self.ensure_configured()?;
        // Refreshes when needed, which also tells which scopes the sign-in has.
        self.access_token()?;
        if self.has_calendar_scope() == Some(false) {
            return Err(reconnect_needed(
                "this sign-in has no calendar permission (Calendars.Read)",
            ));
        }
        let mut url = calendar_view_url(start_ms, end_ms);
        let mut events = Vec::new();
        for page in 0..CALENDAR_MAX_PAGES {
            let body = self
                .graph_request(reqwest::Method::GET, &url, None, Some(PREFER_UTC))
                .map_err(|err| match err {
                    PlatformError::Auth(detail) => reconnect_needed(&detail),
                    other => other,
                })?;
            let parsed = parse_calendar_page(&body)?;
            events.extend(parsed.events);
            match parsed.next_link {
                None => break,
                Some(_) if page + 1 == CALENDAR_MAX_PAGES => {
                    log::debug!(
                        "teams: calendar has more than {CALENDAR_MAX_PAGES} pages, the rest is ignored"
                    );
                }
                Some(next) if is_graph_url(&next) => url = next,
                Some(_) => {
                    log::warn!("teams: ignoring a calendar next page outside Microsoft Graph");
                    break;
                }
            }
        }
        Ok(meetings_only(events, start_ms, end_ms))
    }

    /// Forgets the stored token (and the in-memory access token).
    pub fn disconnect(&mut self) -> Result<(), PlatformError> {
        self.access = None;
        token_store::delete(&self.settings.token_path)
    }

    // -- internals ------------------------------------------------------------------------------------------

    fn client_id(&self) -> &str {
        self.settings.client_id.trim()
    }

    fn ensure_configured(&self) -> Result<(), PlatformError> {
        if self.is_configured() {
            Ok(())
        } else {
            Err(PlatformError::Auth(
                "Teams client id is not configured".into(),
            ))
        }
    }

    fn token_url(&self) -> Result<String, PlatformError> {
        Ok(format!(
            "{LOGIN_BASE}/{}/oauth2/v2.0/token",
            tenant_segment(&self.settings.tenant)?
        ))
    }

    fn http(&self) -> Result<&reqwest::blocking::Client, PlatformError> {
        self.http.get()
    }

    fn remember_access(&mut self, tokens: &TokenResponse, scopes: Option<String>) {
        let lifetime = if tokens.expires_in == 0 {
            DEFAULT_TOKEN_LIFETIME_SECS
        } else {
            tokens.expires_in
        };
        let lifetime = Duration::from_secs(lifetime).saturating_sub(EXPIRY_MARGIN);
        self.access = Some(AccessToken {
            token: tokens.access_token.clone(),
            expires_at: Instant::now() + lifetime,
            scopes,
        });
    }

    /// A valid access token from memory, or a fresh one via the stored refresh token.
    fn access_token(&mut self) -> Result<String, PlatformError> {
        if let Some(access) = &self.access
            && Instant::now() < access.expires_at
        {
            return Ok(access.token.clone());
        }
        self.refresh()
    }

    fn refresh(&mut self) -> Result<String, PlatformError> {
        self.ensure_configured()?;
        let stored = token_store::load(&self.settings.token_path)?
            .ok_or_else(|| PlatformError::Auth("not connected to Microsoft Teams".into()))?;
        let url = self.token_url()?;
        let client_id = self.client_id().to_owned();
        // Ask for exactly what the sign-in was granted: requesting a scope the user never consented to would
        // fail with invalid_grant and drop a token that still works for presence.
        let scope = refresh_scope(stored.scopes.as_deref());
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", client_id.as_str()),
            ("refresh_token", stored.refresh_token.as_str()),
            ("scope", scope),
        ];
        let response = self
            .http()?
            .post(url.as_str())
            .form(&params)
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().map_err(network_error)?;

        if !status.is_success() {
            let error: OAuthErrorResponse = serde_json::from_str(&body).unwrap_or_default();
            if refresh_token_is_dead(&error.error) {
                self.access = None;
                if let Err(err) = token_store::delete(&self.settings.token_path) {
                    log::warn!("teams: deleting the rejected token failed: {err}");
                }
                return Err(PlatformError::Auth(format!(
                    "sign-in expired, connect Teams again ({})",
                    error.error
                )));
            }
            return Err(PlatformError::Auth(oauth_error_text(
                status.as_u16(),
                &body,
            )));
        }

        let tokens: TokenResponse = serde_json::from_str(&body)
            .map_err(|err| PlatformError::Auth(format!("invalid token response: {err}")))?;
        let scopes = granted_scopes(&tokens).or_else(|| stored.scopes.clone());
        let rotated = tokens
            .refresh_token
            .as_deref()
            .filter(|rotated| !rotated.is_empty() && *rotated != stored.refresh_token);
        if rotated.is_some() || scopes != stored.scopes {
            let updated = StoredToken {
                refresh_token: rotated.unwrap_or(&stored.refresh_token).to_owned(),
                account: stored.account.clone(),
                scopes: scopes.clone(),
            };
            if let Err(err) = token_store::save(&self.settings.token_path, &updated) {
                log::warn!("teams: storing the refreshed sign-in failed: {err}");
            }
        }
        self.remember_access(&tokens, scopes);
        Ok(tokens.access_token)
    }

    fn fetch_account(&mut self) -> Result<Option<String>, PlatformError> {
        let token = self.access_token()?;
        let response = self
            .http()?
            .get(format!("{GRAPH_BASE}/me?$select=userPrincipalName,mail"))
            .bearer_auth(token)
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().map_err(network_error)?;
        if !status.is_success() {
            return Err(graph_error(status.as_u16(), &body));
        }
        let me: MeResponse = serde_json::from_str(&body)
            .map_err(|err| PlatformError::Network(format!("invalid /me response: {err}")))?;
        Ok(me.user_principal_name.or(me.mail).filter(|a| !a.is_empty()))
    }

    /// POST to Graph; retries once with a refreshed token on 401.
    fn graph_post(
        &mut self,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<(), PlatformError> {
        self.graph_send(reqwest::Method::POST, path, body)
            .map(|_| ())
    }

    /// Graph request (`path` under `/v1.0`) returning the response body; retries once with a refreshed token on 401.
    fn graph_send(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<String, PlatformError> {
        self.graph_request(method, &format!("{GRAPH_BASE}{path}"), body, None)
    }

    /// Graph request to a full Graph URL, with an optional `Prefer` header.
    fn graph_request(
        &mut self,
        method: reqwest::Method,
        url: &str,
        body: Option<&serde_json::Value>,
        prefer: Option<&str>,
    ) -> Result<String, PlatformError> {
        self.ensure_configured()?;
        if !is_graph_url(url) {
            // Never send the access token anywhere else.
            return Err(PlatformError::Network("not a Microsoft Graph URL".into()));
        }
        for attempt in 0..2 {
            let token = self.access_token()?;
            let request = self.http()?.request(method.clone(), url).bearer_auth(token);
            let request = match prefer {
                Some(value) => request.header("Prefer", value),
                None => request,
            };
            let request = match body {
                Some(json) => request.json(json),
                // IIS-fronted endpoints want an explicit length for an empty POST.
                None if method == reqwest::Method::POST => request.header("Content-Length", "0"),
                None => request,
            };
            let response = request.send().map_err(network_error)?;
            let status = response.status();
            if status.is_success() {
                return match response.text() {
                    Ok(text) => Ok(text),
                    // The change went through; only a body we need (GET) makes a failed read an error.
                    Err(err) if method == reqwest::Method::GET => Err(network_error(err)),
                    Err(_) => Ok(String::new()),
                };
            }
            let text = response.text().unwrap_or_default();
            if status.as_u16() == 401 && attempt == 0 {
                log::info!("teams: access token rejected, refreshing");
                self.access = None;
                continue;
            }
            return Err(graph_error(status.as_u16(), &text));
        }
        Err(PlatformError::Auth(
            "Microsoft Graph rejected the refreshed access token".into(),
        ))
    }
}

// -- wire formats -----------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default, deserialize_with = "lenient_u64")]
    expires_in: u64,
    #[serde(default, deserialize_with = "lenient_u64")]
    interval: u64,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default, deserialize_with = "lenient_u64")]
    expires_in: u64,
    /// Space-separated scopes of the access token (`Presence.ReadWrite User.Read Calendars.Read …`).
    #[serde(default)]
    scope: Option<String>,
}

/// The granted scopes the token endpoint reported (`None` when it did not).
fn granted_scopes(tokens: &TokenResponse) -> Option<String> {
    tokens
        .scope
        .as_deref()
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(str::to_owned)
}

/// `granted` (space-separated; short names or `https://graph.microsoft.com/…` URIs) includes `scope`.
fn scope_granted(granted: &str, scope: &str) -> bool {
    granted.split_whitespace().any(|item| {
        item.rsplit('/')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(scope))
    })
}

fn calendar_scope_granted(granted: &str) -> bool {
    CALENDAR_SCOPES
        .iter()
        .any(|scope| scope_granted(granted, scope))
}

/// Scopes to ask for when refreshing a sign-in that was granted `granted` (`None` = not recorded: a sign-in from
/// before the calendar integration).
fn refresh_scope(granted: Option<&str>) -> &'static str {
    match granted {
        Some(granted) if calendar_scope_granted(granted) => SCOPES,
        _ => PRESENCE_SCOPES,
    }
}

fn reconnect_needed(detail: &str) -> PlatformError {
    PlatformError::Auth(format!(
        "reconnect needed: connect Microsoft (Teams) again to allow calendar access ({detail})"
    ))
}

/// Only Graph URLs get the access token (paging links come from the response body).
fn is_graph_url(url: &str) -> bool {
    url.strip_prefix(GRAPH_BASE)
        .is_some_and(|rest| rest.starts_with('/'))
}

fn calendar_view_url(start_ms: i64, end_ms: i64) -> String {
    format!(
        "{GRAPH_BASE}/me/calendarView?startDateTime={}&endDateTime={}&$select={CALENDAR_SELECT}\
         &$orderby=start/dateTime&$top={CALENDAR_PAGE_SIZE}",
        format_utc(start_ms),
        format_utc(end_ms)
    )
}

/// One page of `calendarView` (events that are not meetings already dropped).
#[derive(Debug)]
struct CalendarPage {
    events: Vec<CalEvent>,
    next_link: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CalendarPageResponse {
    value: Option<Vec<serde_json::Value>>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GraphEvent {
    id: String,
    subject: Option<String>,
    start: Option<GraphDateTime>,
    end: Option<GraphDateTime>,
    show_as: Option<String>,
    is_cancelled: Option<bool>,
    is_all_day: Option<bool>,
    response_status: Option<GraphResponseStatus>,
    is_online_meeting: Option<bool>,
    online_meeting: Option<GraphOnlineMeeting>,
    location: Option<GraphLocation>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GraphDateTime {
    date_time: String,
    time_zone: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GraphResponseStatus {
    response: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GraphOnlineMeeting {
    join_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GraphLocation {
    display_name: Option<String>,
}

fn parse_calendar_page(body: &str) -> Result<CalendarPage, PlatformError> {
    let page: CalendarPageResponse = serde_json::from_str(body)
        .map_err(|err| PlatformError::Network(format!("invalid calendarView response: {err}")))?;
    let events = page
        .value
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| serde_json::from_value::<GraphEvent>(item).ok())
        .filter_map(graph_event)
        .collect();
    Ok(CalendarPage {
        events,
        next_link: page.next_link.filter(|link| !link.is_empty()),
    })
}

/// `None` for cancelled and declined events (and events without readable times).
fn graph_event(event: GraphEvent) -> Option<CalEvent> {
    if event.is_cancelled == Some(true) {
        return None;
    }
    let response = event
        .response_status
        .as_ref()
        .and_then(|status| status.response.as_deref());
    if response.is_some_and(|r| r.eq_ignore_ascii_case("declined")) {
        return None;
    }
    let start_ms = graph_time(event.start.as_ref()?)?;
    let end_ms = graph_time(event.end.as_ref()?)?;
    let show_as = event.show_as.as_deref().unwrap_or("busy");
    let busy =
        !(show_as.eq_ignore_ascii_case("free") || show_as.eq_ignore_ascii_case("workingElsewhere"));
    let online = event.is_online_meeting == Some(true)
        || event
            .online_meeting
            .as_ref()
            .and_then(|m| m.join_url.as_deref())
            .is_some_and(|url| !url.is_empty())
        || event
            .location
            .as_ref()
            .and_then(|l| l.display_name.as_deref())
            .is_some_and(looks_online);
    Some(CalEvent {
        id: event.id,
        subject: crate::calendar::clean_subject(event.subject.as_deref().unwrap_or("")),
        start_ms,
        end_ms,
        online,
        all_day: event.is_all_day == Some(true),
        busy,
    })
}

/// `{"dateTime":"2026-10-13T08:00:00.0000000","timeZone":"UTC"}` → UTC ms (other zones by name).
fn graph_time(value: &GraphDateTime) -> Option<i64> {
    let wall = parse_iso_naive(value.date_time.trim())?;
    let zone = value.time_zone.as_deref().map(str::trim).unwrap_or("UTC");
    if zone.is_empty() || zone.eq_ignore_ascii_case("UTC") {
        return Some(wall);
    }
    let zone = TimeZone::by_name(zone).unwrap_or_else(|| {
        log::debug!("teams: unknown calendar time zone, assuming UTC");
        TimeZone::utc()
    });
    Some(zone.wall_to_utc(wall))
}

#[derive(Debug, Default, Deserialize)]
struct OAuthErrorResponse {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct GraphErrorResponse {
    #[serde(default)]
    error: GraphErrorBody,
}

#[derive(Debug, Default, Deserialize)]
struct GraphErrorBody {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeResponse {
    #[serde(default)]
    user_principal_name: Option<String>,
    #[serde(default)]
    mail: Option<String>,
}

/// Answer of `GET /me/presence` (other fields, e.g. `id` or `outOfOfficeSettings`, are ignored).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresenceResponse {
    #[serde(default)]
    availability: Option<String>,
    #[serde(default)]
    activity: Option<String>,
}

fn parse_presence(body: &str) -> Result<Presence, PlatformError> {
    let parsed: PresenceResponse = serde_json::from_str(body)
        .map_err(|err| PlatformError::Network(format!("invalid presence response: {err}")))?;
    Ok(Presence {
        availability: parsed.availability.unwrap_or_default(),
        activity: parsed.activity.unwrap_or_default(),
    })
}

/// Body of `setUserPreferredPresence`.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PreferredPresence {
    availability: &'static str,
    activity: &'static str,
    expiration_duration: String,
}

impl PreferredPresence {
    fn do_not_disturb(minutes: u32) -> Self {
        Self {
            availability: "DoNotDisturb",
            activity: "DoNotDisturb",
            expiration_duration: iso_duration_minutes(minutes),
        }
    }
}

/// Accepts a JSON number or a numeric string (older identity endpoints send strings).
fn lenient_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(u64),
        Text(String),
    }
    match NumberOrString::deserialize(deserializer)? {
        NumberOrString::Number(n) => Ok(n),
        NumberOrString::Text(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

fn device_login_from(
    response: DeviceCodeResponse,
    scopes: &'static str,
    now: Instant,
) -> DeviceLogin {
    let expires_in_secs = match u32::try_from(response.expires_in) {
        Ok(0) => DEFAULT_DEVICE_CODE_LIFETIME_SECS,
        Ok(secs) => secs,
        Err(_) => u32::MAX,
    };
    let interval_secs = match u32::try_from(response.interval) {
        Ok(0) => DEFAULT_POLL_INTERVAL_SECS,
        Ok(secs) => secs.min(60),
        Err(_) => 60,
    };
    DeviceLogin {
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        message: response.message,
        expires_in_secs,
        interval_secs,
        device_code: response.device_code,
        scopes,
        started_at: now,
    }
}

/// Maps a failed device-code token request to a poll result.
fn classify_login_error(error: &OAuthErrorResponse) -> LoginPoll {
    match error.error.as_str() {
        "authorization_pending" | "slow_down" | "temporarily_unavailable" => LoginPoll::Pending,
        "authorization_declined" | "access_denied" => {
            LoginPoll::Failed("sign-in was declined".into())
        }
        "expired_token" | "code_expired" => LoginPoll::Failed("the sign-in code expired".into()),
        "bad_verification_code" => LoginPoll::Failed("the sign-in code is not valid".into()),
        "" => LoginPoll::Failed("sign-in failed (no error code)".into()),
        code => LoginPoll::Failed(match error.error_description.as_deref() {
            Some(description) => format!("{code}: {}", first_line(description)),
            None => code.to_owned(),
        }),
    }
}

/// OAuth errors after which the stored refresh token is useless.
fn refresh_token_is_dead(code: &str) -> bool {
    matches!(
        code,
        "invalid_grant" | "interaction_required" | "invalid_client" | "unauthorized_client"
    )
}

/// Readable OAuth error (`error: first line of error_description`), or the HTTP status.
fn oauth_error_text(status: u16, body: &str) -> String {
    match serde_json::from_str::<OAuthErrorResponse>(body) {
        Ok(error) if !error.error.is_empty() => match error.error_description.as_deref() {
            Some(description) => format!("{}: {}", error.error, first_line(description)),
            None => error.error,
        },
        _ => format!("HTTP {status}"),
    }
}

/// Graph error → [`PlatformError`] (401/403 = auth, everything else = network/service).
fn graph_error(status: u16, body: &str) -> PlatformError {
    let parsed: GraphErrorResponse = serde_json::from_str(body).unwrap_or_default();
    let detail = if parsed.error.code.is_empty() {
        format!("HTTP {status}")
    } else {
        format!(
            "HTTP {status} {}: {}",
            parsed.error.code,
            first_line(&parsed.error.message)
        )
    };
    match status {
        401 | 403 => PlatformError::Auth(detail),
        _ => PlatformError::Network(detail),
    }
}

/// Validates the tenant for use as a URL path segment (empty → [`DEFAULT_TENANT`]).
fn tenant_segment(tenant: &str) -> Result<String, PlatformError> {
    let tenant = tenant.trim();
    if tenant.is_empty() {
        return Ok(DEFAULT_TENANT.to_owned());
    }
    // No `.`/`..` path segments: the URL parser would resolve them and drop the tenant from the path.
    let valid = tenant.len() <= 128
        && !tenant.starts_with('.')
        && !tenant.ends_with('.')
        && !tenant.contains("..")
        && tenant
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'));
    if valid {
        Ok(tenant.to_owned())
    } else {
        Err(PlatformError::Auth(format!("invalid tenant {tenant:?}")))
    }
}

/// ISO 8601 duration for whole minutes, clamped to 1..=[`MAX_DND_MINUTES`] (`90` → `PT1H30M`).
fn iso_duration_minutes(minutes: u32) -> String {
    let minutes = minutes.clamp(1, MAX_DND_MINUTES);
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("PT{m}M"),
        (h, 0) => format!("PT{h}H"),
        (h, m) => format!("PT{h}H{m}M"),
    }
}

// -- token storage ----------------------------------------------------------------------------------------------

/// Stored (encrypted) sign-in state.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredToken {
    refresh_token: String,
    #[serde(default)]
    account: Option<String>,
    /// Granted scopes (space-separated); absent in tokens stored before the calendar integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scopes: Option<String>,
}

impl fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredToken")
            .field("refresh_token", &"<redacted>")
            .field("account", &self.account)
            .field("scopes", &self.scopes)
            .finish()
    }
}

mod token_store {
    use super::{Path, PlatformError, StoredToken};
    use crate::secrets;

    /// Encrypted storage is available on this platform.
    pub(super) const SUPPORTED: bool = secrets::SUPPORTED;

    pub(super) fn encode(token: &StoredToken) -> Result<Vec<u8>, PlatformError> {
        serde_json::to_vec(token).map_err(|err| PlatformError::Io(format!("token encode: {err}")))
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<StoredToken, PlatformError> {
        serde_json::from_slice(bytes)
            .map_err(|err| PlatformError::Io(format!("stored token is corrupted: {err}")))
    }

    /// `Ok(None)` when no token is stored.
    pub(super) fn load(path: &Path) -> Result<Option<StoredToken>, PlatformError> {
        match secrets::read_sealed(path)? {
            Some(plain) => decode(&plain).map(Some),
            None => Ok(None),
        }
    }

    pub(super) fn save(path: &Path, token: &StoredToken) -> Result<(), PlatformError> {
        secrets::write_sealed(path, &encode(token)?)
    }

    pub(super) fn delete(path: &Path) -> Result<(), PlatformError> {
        secrets::delete_sealed(path)
    }
}

#[cfg(test)]
mod tests {
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

    fn settings(client_id: &str, path: PathBuf) -> TeamsSettings {
        TeamsSettings {
            client_id: client_id.into(),
            tenant: String::new(),
            token_path: path,
        }
    }

    #[test]
    fn token_json_round_trip() {
        let token = StoredToken {
            refresh_token: "secret-refresh".into(),
            account: Some("ala@example.com".into()),
            scopes: None,
        };
        let bytes = token_store::encode(&token).expect("encode");
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        assert_eq!(
            text,
            r#"{"refreshToken":"secret-refresh","account":"ala@example.com"}"#
        );
        assert_eq!(token_store::decode(&bytes).expect("decode"), token);

        let legacy = token_store::decode(br#"{"refreshToken":"r"}"#).expect("decode");
        assert_eq!(legacy.account, None);
        assert_eq!(legacy.scopes, None);
        assert!(token_store::decode(b"not json").is_err());

        let with_scopes = StoredToken {
            scopes: Some("Presence.ReadWrite User.Read Calendars.Read".into()),
            ..token
        };
        let bytes = token_store::encode(&with_scopes).expect("encode");
        assert!(
            String::from_utf8_lossy(&bytes)
                .contains(r#""scopes":"Presence.ReadWrite User.Read Calendars.Read""#)
        );
        assert_eq!(token_store::decode(&bytes).expect("decode"), with_scopes);
    }

    #[test]
    fn scopes_decide_refresh_and_calendar_access() {
        let tokens: TokenResponse = serde_json::from_str(
            r#"{"access_token":"at","scope":"https://graph.microsoft.com/Calendars.Read Presence.ReadWrite User.Read profile openid email"}"#,
        )
        .expect("parse");
        let granted = granted_scopes(&tokens).expect("scopes");
        assert!(calendar_scope_granted(&granted));
        assert_eq!(refresh_scope(Some(&granted)), SCOPES);

        assert!(!calendar_scope_granted("Presence.ReadWrite User.Read"));
        assert!(calendar_scope_granted("calendars.readwrite"));
        assert!(!calendar_scope_granted("Calendars.ReadBasic"));
        assert_eq!(
            refresh_scope(Some("Presence.ReadWrite User.Read")),
            PRESENCE_SCOPES
        );
        assert_eq!(refresh_scope(None), PRESENCE_SCOPES, "older sign-ins");
        assert!(SCOPES.contains("Calendars.Read") && SCOPES.contains("offline_access"));
        assert!(!PRESENCE_SCOPES.contains("Calendars"));

        let empty: TokenResponse =
            serde_json::from_str(r#"{"access_token":"at","scope":"  "}"#).expect("parse");
        assert_eq!(granted_scopes(&empty), None);
        assert!(matches!(
            reconnect_needed("x"),
            PlatformError::Auth(ref m) if m.starts_with("reconnect needed")
        ));
    }

    #[test]
    fn calendar_view_url_and_graph_url_check() {
        let url = calendar_view_url(0, 3_600_000);
        assert!(url.starts_with(
            "https://graph.microsoft.com/v1.0/me/calendarView?startDateTime=1970-01-01T00:00:00Z&endDateTime=1970-01-01T01:00:00Z&$select="
        ));
        assert!(
            url.contains("isCancelled")
                && url.contains("responseStatus")
                && url.contains("$top=100")
        );
        assert!(is_graph_url(&url));
        assert!(is_graph_url(
            "https://graph.microsoft.com/v1.0/me/calendarView?$skip=100"
        ));
        assert!(!is_graph_url(
            "https://graph.microsoft.com.evil.example/v1.0/me"
        ));
        assert!(!is_graph_url("https://graph.microsoft.com/v1.0evil/me"));
        assert!(!is_graph_url("http://graph.microsoft.com/v1.0/me"));
    }

    #[test]
    fn calendar_page_is_filtered() {
        let body = r#"{
          "@odata.context": "https://graph.microsoft.com/v1.0/$metadata#users('x')/calendarView(subject,start,end)",
          "value": [
            {"@odata.etag":"W/\"1\"","id":"AAMkAGI2-1","subject":"Sprint review","isCancelled":false,"isAllDay":false,
             "showAs":"busy","isOnlineMeeting":true,"onlineMeeting":{"joinUrl":"https://teams.microsoft.com/l/meetup-join/1"},
             "responseStatus":{"response":"accepted","time":"2026-10-01T10:00:00Z"},
             "start":{"dateTime":"2026-10-13T08:00:00.0000000","timeZone":"UTC"},
             "end":{"dateTime":"2026-10-13T09:00:00.0000000","timeZone":"UTC"},
             "location":{"displayName":"Microsoft Teams Meeting"}},
            {"id":"declined","subject":"No","isCancelled":false,"showAs":"busy",
             "responseStatus":{"response":"declined"},
             "start":{"dateTime":"2026-10-13T10:00:00.0000000","timeZone":"UTC"},
             "end":{"dateTime":"2026-10-13T11:00:00.0000000","timeZone":"UTC"}},
            {"id":"cancelled","subject":"Canceled: X","isCancelled":true,
             "start":{"dateTime":"2026-10-13T10:00:00.0000000","timeZone":"UTC"},
             "end":{"dateTime":"2026-10-13T11:00:00.0000000","timeZone":"UTC"}},
            {"id":"free","subject":"Hold","showAs":"free","isCancelled":false,
             "responseStatus":{"response":"organizer"},
             "start":{"dateTime":"2026-10-13T12:00:00.0000000","timeZone":"UTC"},
             "end":{"dateTime":"2026-10-13T12:30:00.0000000","timeZone":"UTC"}},
            {"id":"tentative","subject":null,"showAs":"tentative","onlineMeeting":null,
             "responseStatus":{"response":"tentativelyAccepted"},
             "start":{"dateTime":"2026-10-13T14:00:00.0000000","timeZone":"UTC"},
             "end":{"dateTime":"2026-10-13T14:30:00.0000000","timeZone":"UTC"},
             "location":{"displayName":"https://zoom.us/j/1"}},
            {"id":"warsaw","subject":"Zone","showAs":"busy",
             "start":{"dateTime":"2026-10-13T16:00:00.0000000","timeZone":"Central European Standard Time"},
             "end":{"dateTime":"2026-10-13T16:30:00.0000000","timeZone":"Central European Standard Time"}},
            {"id":"allday","subject":"Holiday","isAllDay":true,"showAs":"oof",
             "start":{"dateTime":"2026-10-13T00:00:00.0000000","timeZone":"UTC"},
             "end":{"dateTime":"2026-10-14T00:00:00.0000000","timeZone":"UTC"}},
            {"id":"broken","start":{"dateTime":"soon"}},
            "junk"
          ],
          "@odata.nextLink": "https://graph.microsoft.com/v1.0/me/calendarView?startDateTime=x&$skip=100"
        }"#;
        let page = parse_calendar_page(body).expect("page");
        assert_eq!(
            page.next_link.as_deref(),
            Some("https://graph.microsoft.com/v1.0/me/calendarView?startDateTime=x&$skip=100")
        );
        let ids: Vec<&str> = page.events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["AAMkAGI2-1", "free", "tentative", "warsaw", "allday"]
        );
        let review = &page.events[0];
        let day = crate::calendar::time::days_from_civil(2026, 10, 13)
            * crate::calendar::time::MS_PER_DAY;
        let hour = crate::calendar::time::MS_PER_HOUR;
        assert_eq!(
            (review.start_ms, review.end_ms),
            (day + 8 * hour, day + 9 * hour)
        );
        assert!(review.online && review.busy && !review.all_day);
        assert_eq!(review.subject, "Sprint review");
        assert!(!page.events[1].busy);
        assert!(page.events[2].busy && page.events[2].online);
        assert_eq!(page.events[2].subject, "");
        assert_eq!(page.events[3].start_ms, day + 14 * hour, "16:00 CEST");
        assert!(page.events[4].all_day);

        let meetings = meetings_only(page.events, day, day + 24 * hour);
        let ids: Vec<&str> = meetings.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["AAMkAGI2-1", "tentative", "warsaw"]);

        let last = parse_calendar_page(r#"{"value":[]}"#).expect("empty");
        assert!(last.events.is_empty() && last.next_link.is_none());
        assert!(parse_calendar_page("<html>").is_err());
    }

    #[test]
    fn debug_output_redacts_secrets() {
        let token = StoredToken {
            refresh_token: "secret-refresh".into(),
            account: None,
            scopes: None,
        };
        assert!(!format!("{token:?}").contains("secret-refresh"));

        let login = device_login_from(
            DeviceCodeResponse {
                device_code: "secret-device-code".into(),
                user_code: "ABCD-EFGH".into(),
                verification_uri: "https://microsoft.com/devicelogin".into(),
                expires_in: 900,
                interval: 5,
                message: "To sign in...".into(),
            },
            SCOPES,
            Instant::now(),
        );
        let debug = format!("{login:?}");
        assert!(!debug.contains("secret-device-code"));
        assert!(debug.contains("ABCD-EFGH"));
    }

    #[test]
    fn device_code_response_parses_numbers_and_strings() {
        let parsed: DeviceCodeResponse = serde_json::from_str(
            r#"{"device_code":"dc","user_code":"UC","verification_uri":"https://microsoft.com/devicelogin",
                "expires_in":"900","interval":5,"message":"Go"}"#,
        )
        .expect("parse");
        let login = device_login_from(parsed, SCOPES, Instant::now());
        assert_eq!(login.expires_in_secs, 900);
        assert_eq!(login.interval_secs, 5);
        assert_eq!(login.user_code, "UC");
        assert!(!login.is_expired());
        assert!(login.includes_calendar());

        let parsed: DeviceCodeResponse =
            serde_json::from_str(r#"{"device_code":"dc","user_code":"UC","verification_uri":"u"}"#)
                .expect("parse");
        let login = device_login_from(parsed, PRESENCE_SCOPES, Instant::now());
        assert_eq!(login.expires_in_secs, DEFAULT_DEVICE_CODE_LIFETIME_SECS);
        assert_eq!(login.interval_secs, DEFAULT_POLL_INTERVAL_SECS);
        assert_eq!(login.message, "");
        assert!(!login.includes_calendar());
    }

    #[test]
    fn calendar_scope_is_requested_only_for_the_calendar() {
        assert_eq!(login_scopes(true), SCOPES);
        assert_eq!(login_scopes(false), PRESENCE_SCOPES);
        for calendar in [true, false] {
            let scopes = login_scopes(calendar);
            assert_eq!(scopes.contains("Calendars.Read"), calendar, "{scopes}");
            for needed in ["Presence.ReadWrite", "User.Read", "offline_access"] {
                assert!(scopes.contains(needed), "{scopes} lacks {needed}");
            }
            // A sign-in keeps asking for what it was granted when it refreshes.
            assert_eq!(refresh_scope(Some(scopes)), scopes);
        }
    }

    #[test]
    fn token_response_parses() {
        let parsed: TokenResponse = serde_json::from_str(
            r#"{"token_type":"Bearer","scope":"Presence.ReadWrite","expires_in":3599,
                "access_token":"at","refresh_token":"rt","id_token":"x"}"#,
        )
        .expect("parse");
        assert_eq!(parsed.access_token, "at");
        assert_eq!(parsed.refresh_token.as_deref(), Some("rt"));
        assert_eq!(parsed.expires_in, 3599);
    }

    #[test]
    fn login_errors_are_classified() {
        let error = |code: &str| OAuthErrorResponse {
            error: code.into(),
            error_description: Some("AADSTS70016: pending.\r\nTrace ID: 123".into()),
        };
        assert_eq!(
            classify_login_error(&error("authorization_pending")),
            LoginPoll::Pending
        );
        assert_eq!(
            classify_login_error(&error("slow_down")),
            LoginPoll::Pending
        );
        assert!(matches!(
            classify_login_error(&error("authorization_declined")),
            LoginPoll::Failed(_)
        ));
        assert!(matches!(
            classify_login_error(&error("expired_token")),
            LoginPoll::Failed(m) if m.contains("expired")
        ));
        assert_eq!(
            classify_login_error(&error("invalid_client")),
            LoginPoll::Failed("invalid_client: AADSTS70016: pending.".into())
        );
        assert!(matches!(
            classify_login_error(&OAuthErrorResponse::default()),
            LoginPoll::Failed(_)
        ));
    }

    #[test]
    fn oauth_and_graph_errors_are_readable() {
        assert_eq!(
            oauth_error_text(
                400,
                r#"{"error":"invalid_request","error_description":"AADSTS900144: missing client_id.\r\nTrace ID: x"}"#
            ),
            "invalid_request: AADSTS900144: missing client_id."
        );
        assert_eq!(oauth_error_text(502, "<html>"), "HTTP 502");

        let err = graph_error(
            403,
            r#"{"error":{"code":"Forbidden","message":"Missing scope.\nmore"}}"#,
        );
        assert!(
            matches!(err, PlatformError::Auth(ref m) if m == "HTTP 403 Forbidden: Missing scope.")
        );
        assert!(matches!(graph_error(503, ""), PlatformError::Network(ref m) if m == "HTTP 503"));
        assert!(refresh_token_is_dead("invalid_grant"));
        assert!(!refresh_token_is_dead("temporarily_unavailable"));
    }

    #[test]
    fn tenant_is_validated() {
        assert_eq!(tenant_segment("").expect("default"), DEFAULT_TENANT);
        assert_eq!(
            tenant_segment(" contoso.onmicrosoft.com ").expect("domain"),
            "contoso.onmicrosoft.com"
        );
        assert_eq!(
            tenant_segment("72f988bf-86f1-41af-91ab-2d7cd011db47").expect("guid"),
            "72f988bf-86f1-41af-91ab-2d7cd011db47"
        );
        assert!(tenant_segment("evil/../x").is_err());
        assert!(tenant_segment("a b").is_err());
        assert!(tenant_segment("x?y=1").is_err());
        assert!(tenant_segment("..").is_err());
        assert!(tenant_segment(".").is_err());
        assert!(tenant_segment("contoso..com").is_err());
    }

    #[test]
    fn durations_are_iso_8601() {
        assert_eq!(iso_duration_minutes(0), "PT1M");
        assert_eq!(iso_duration_minutes(45), "PT45M");
        assert_eq!(iso_duration_minutes(60), "PT1H");
        assert_eq!(iso_duration_minutes(50), "PT50M");
        assert_eq!(iso_duration_minutes(95), "PT1H35M");
        assert_eq!(iso_duration_minutes(u32::MAX), "PT168H");
    }

    #[test]
    fn presence_body_matches_graph() {
        let body = serde_json::to_value(PreferredPresence::do_not_disturb(50)).expect("json");
        assert_eq!(
            body,
            serde_json::json!({
                "availability": "DoNotDisturb",
                "activity": "DoNotDisturb",
                "expirationDuration": "PT50M"
            })
        );
    }

    fn presence(availability: &str, activity: &str) -> Presence {
        Presence {
            availability: availability.into(),
            activity: activity.into(),
        }
    }

    #[test]
    fn presence_response_parses() {
        let parsed = parse_presence(
            r#"{"@odata.context":"https://graph.microsoft.com/v1.0/$metadata#users('x')/presence/$entity",
                "id":"fa8bf3dc-eca7-46b7-bad1-db199b62afc3","availability":"Offline","activity":"OffWork",
                "statusMessage":null,"outOfOfficeSettings":{"message":null,"isOutOfOffice":false}}"#,
        )
        .expect("parse");
        assert_eq!(parsed, presence("Offline", "OffWork"));

        let partial = parse_presence(r#"{"availability":null}"#).expect("parse");
        assert_eq!(partial, presence("", ""));
        assert!(matches!(
            parse_presence("<html>"),
            Err(PlatformError::Network(_))
        ));
    }

    #[test]
    fn offline_blocks_dnd() {
        assert!(presence_blocks_dnd(&presence("Offline", "OffWork")));
        assert!(presence_blocks_dnd(&presence("Offline", "Offline")));
        assert!(presence_blocks_dnd(&presence("offline", "")));
        for availability in [
            "Available",
            "AvailableIdle",
            "Away",
            "BeRightBack",
            "Busy",
            "BusyIdle",
            "DoNotDisturb",
            "PresenceUnknown",
            "",
        ] {
            assert!(
                !presence_blocks_dnd(&presence(availability, "Available")),
                "{availability}"
            );
        }
    }

    #[test]
    fn existing_dnd_is_detected() {
        assert!(presence_is_user_dnd(&presence(
            "DoNotDisturb",
            "DoNotDisturb"
        )));
        assert!(presence_is_user_dnd(&presence(
            "DoNotDisturb",
            "Presenting"
        )));
        assert!(presence_is_user_dnd(&presence("donotdisturb", "")));
        for availability in [
            "Available",
            "Busy",
            "Away",
            "Offline",
            "PresenceUnknown",
            "",
        ] {
            assert!(
                !presence_is_user_dnd(&presence(availability, "DoNotDisturb")),
                "{availability}"
            );
        }
    }

    #[test]
    fn client_reports_configuration_without_network() {
        let path = unique_temp_path("teams-none.bin");
        let client = TeamsClient::new(settings("  ", path.clone()));
        assert!(!client.is_configured());
        assert!(!client.has_token());
        assert_eq!(client.account(), None);
        assert!(client.begin_login(false).is_err());
        assert!(client.begin_login(true).is_err());

        let mut client = TeamsClient::new(settings("11111111-2222-3333-4444-555555555555", path));
        assert!(client.is_configured());
        assert!(
            client.disconnect().is_ok(),
            "disconnect without a token is a no-op"
        );
        // No stored token → presence calls fail before touching the network.
        assert!(matches!(
            client.set_dnd(45),
            Err(PlatformError::Auth(_)) | Err(PlatformError::Unsupported)
        ));
        assert!(client.clear_dnd().is_err());
        assert!(matches!(
            client.get_presence(),
            Err(PlatformError::Auth(_)) | Err(PlatformError::Unsupported)
        ));
        assert!(matches!(
            client.calendar_view(0, 3_600_000),
            Err(PlatformError::Auth(_)) | Err(PlatformError::Unsupported)
        ));
        assert!(
            client
                .calendar_view(10, 10)
                .expect("empty range")
                .is_empty()
        );
        assert_eq!(client.has_calendar_scope(), None);
    }

    #[test]
    fn calendar_without_permission_needs_reconnect_without_network() {
        let path = unique_temp_path("teams-presence-only.bin");
        let mut client = TeamsClient::new(settings("11111111-2222-3333-4444-555555555555", path));
        // A valid in-memory access token of a presence-only sign-in: no refresh, no Graph call.
        client.access = Some(AccessToken {
            token: "at".into(),
            expires_at: Instant::now() + Duration::from_secs(600),
            scopes: Some("Presence.ReadWrite User.Read".into()),
        });
        assert_eq!(client.has_calendar_scope(), Some(false));
        let err = client
            .calendar_view(0, 3_600_000)
            .expect_err("no permission");
        assert!(
            matches!(err, PlatformError::Auth(ref m) if m.starts_with("reconnect needed")),
            "{err}"
        );
        // Paging links outside Graph never get the token.
        assert!(matches!(
            client.graph_request(
                reqwest::Method::GET,
                "https://evil.example/v1.0/me",
                None,
                None
            ),
            Err(PlatformError::Network(_))
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dpapi_token_store_round_trip() {
        let path = unique_temp_path("teams-token.bin");
        let token = StoredToken {
            refresh_token: "rt-123".into(),
            account: Some("ala@example.com".into()),
            scopes: None,
        };
        token_store::save(&path, &token).expect("save");
        let raw = std::fs::read(&path).expect("read raw");
        assert!(
            !String::from_utf8_lossy(&raw).contains("rt-123"),
            "stored encrypted"
        );
        assert_eq!(token_store::load(&path).expect("load"), Some(token.clone()));

        let client = TeamsClient::new(settings("id", path.clone()));
        assert!(client.has_token());
        assert_eq!(client.account().as_deref(), Some("ala@example.com"));
        assert_eq!(client.has_calendar_scope(), None, "older sign-in: unknown");

        // A token sealed by the module before the shared `secrets` module still decrypts (same entropy).
        let sealed =
            crate::secrets::protect(br#"{"refreshToken":"old","account":null}"#).expect("protect");
        std::fs::write(&path, sealed).expect("write");
        assert_eq!(
            token_store::load(&path)
                .expect("load")
                .map(|t| t.refresh_token),
            Some("old".to_owned())
        );

        token_store::save(
            &path,
            &StoredToken {
                scopes: Some("Presence.ReadWrite User.Read".into()),
                ..token.clone()
            },
        )
        .expect("save");
        assert_eq!(client.has_calendar_scope(), Some(false));
        token_store::save(
            &path,
            &StoredToken {
                scopes: Some(SCOPES.into()),
                ..token
            },
        )
        .expect("save");
        assert_eq!(client.has_calendar_scope(), Some(true));

        std::fs::write(&path, b"garbage").expect("corrupt");
        assert!(token_store::load(&path).is_err());
        assert!(!client.has_token());
        token_store::delete(&path).expect("delete");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn token_storage_is_unsupported_elsewhere() {
        let path = unique_temp_path("teams-token.bin");
        let token = StoredToken {
            refresh_token: "rt".into(),
            account: None,
            scopes: None,
        };
        assert!(matches!(
            token_store::save(&path, &token),
            Err(PlatformError::Unsupported)
        ));
        assert!(!token_store::SUPPORTED);
    }
}
