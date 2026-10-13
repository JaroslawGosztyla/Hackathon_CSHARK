//! Microsoft Teams presence "Do not disturb" through Microsoft Graph.
//!
//! * Sign-in: OAuth 2.0 device code flow against the Microsoft identity platform (v2 endpoints) with a
//!   user-registered public client (Azure app registration: "Mobile and desktop applications" platform,
//!   "Allow public client flows" on, delegated `Presence.ReadWrite` + `User.Read`).
//!   Scopes: `Presence.ReadWrite User.Read offline_access`.
//! * Presence: `POST /me/presence/setUserPreferredPresence` (`DoNotDisturb`, with an expiration so a crashed app
//!   never leaves the user in DND forever) and `POST /me/presence/clearUserPreferredPresence`.
//!   Preferred presence is shown only while the user has a presence session (e.g. Teams is signed in somewhere).
//! * Current presence: `GET /me/presence` ([`TeamsClient::get_presence`], covered by `Presence.ReadWrite`). The shell
//!   reads it before focus: a user who appears offline ([`presence_blocks_dnd`]) must not become visible as
//!   "Do not disturb", and a user who already chose Do not disturb ([`presence_is_user_dnd`]) is left alone
//!   (we neither set nor clear it, because clearing the preferred presence would also drop the user's own choice).
//! * Token storage: JSON `{refreshToken, account}` encrypted with DPAPI (current user, `CryptProtectData`) and
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
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Deserializer, Serialize};

use crate::PlatformError;

/// Delegated scopes requested at sign-in.
pub const SCOPES: &str = "Presence.ReadWrite User.Read offline_access";
/// Tenant used when the configured one is empty.
pub const DEFAULT_TENANT: &str = "organizations";
/// Longest Do-not-disturb expiration accepted by Graph (7 days), in minutes.
pub const MAX_DND_MINUTES: u32 = 7 * 24 * 60;

const LOGIN_BASE: &str = "https://login.microsoftonline.com";
const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
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
    started_at: Instant,
}

impl DeviceLogin {
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

/// Graph client for Teams presence. Blocking; see the module docs for threading rules.
pub struct TeamsClient {
    settings: TeamsSettings,
    http: OnceLock<reqwest::blocking::Client>,
    access: Option<AccessToken>,
}

/// In-memory access token.
struct AccessToken {
    token: String,
    expires_at: Instant,
}

impl TeamsClient {
    /// Cheap: no I/O, no network (the HTTP client is built on first use).
    pub fn new(settings: TeamsSettings) -> Self {
        Self {
            settings,
            http: OnceLock::new(),
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

    /// Starts a device code login (one HTTP request).
    pub fn begin_login(&self) -> Result<DeviceLogin, PlatformError> {
        if !token_store::SUPPORTED {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_configured()?;
        let url = format!(
            "{LOGIN_BASE}/{}/oauth2/v2.0/devicecode",
            tenant_segment(&self.settings.tenant)?
        );
        let params = [("client_id", self.client_id()), ("scope", SCOPES)];
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
        Ok(device_login_from(parsed, Instant::now()))
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
        self.remember_access(&tokens);

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
        if let Some(client) = self.http.get() {
            return Ok(client);
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .connect_timeout(HTTP_TIMEOUT)
            .user_agent(concat!("FocusMode/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .build()
            .map_err(network_error)?;
        // Another initializer cannot race us (`&self` on one thread), but stay correct either way.
        let _ = self.http.set(client);
        self.http
            .get()
            .ok_or_else(|| PlatformError::Network("HTTP client unavailable".into()))
    }

    fn remember_access(&mut self, tokens: &TokenResponse) {
        let lifetime = if tokens.expires_in == 0 {
            DEFAULT_TOKEN_LIFETIME_SECS
        } else {
            tokens.expires_in
        };
        let lifetime = Duration::from_secs(lifetime).saturating_sub(EXPIRY_MARGIN);
        self.access = Some(AccessToken {
            token: tokens.access_token.clone(),
            expires_at: Instant::now() + lifetime,
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
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", client_id.as_str()),
            ("refresh_token", stored.refresh_token.as_str()),
            ("scope", SCOPES),
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
        if let Some(rotated) = tokens.refresh_token.as_deref()
            && !rotated.is_empty()
            && rotated != stored.refresh_token
        {
            let updated = StoredToken {
                refresh_token: rotated.to_owned(),
                account: stored.account.clone(),
            };
            if let Err(err) = token_store::save(&self.settings.token_path, &updated) {
                log::warn!("teams: storing the rotated refresh token failed: {err}");
            }
        }
        self.remember_access(&tokens);
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

    /// Graph request returning the response body; retries once with a refreshed token on 401.
    fn graph_send(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<String, PlatformError> {
        self.ensure_configured()?;
        let url = format!("{GRAPH_BASE}{path}");
        for attempt in 0..2 {
            let token = self.access_token()?;
            let request = self
                .http()?
                .request(method.clone(), url.as_str())
                .bearer_auth(token);
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

fn device_login_from(response: DeviceCodeResponse, now: Instant) -> DeviceLogin {
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

/// Identity platform descriptions carry trace/correlation ids on later lines.
fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("").trim()
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

fn network_error(err: reqwest::Error) -> PlatformError {
    let kind = if err.is_timeout() {
        "timed out"
    } else if err.is_connect() {
        "connection failed"
    } else {
        "request failed"
    };
    // URLs carry no secrets here, but keep messages short; the cause chain says what actually failed.
    let err = err.without_url();
    let mut detail = err.to_string();
    let mut source = std::error::Error::source(&err);
    let mut depth = 0;
    while let Some(cause) = source
        && depth < 4
    {
        detail.push_str(": ");
        detail.push_str(&cause.to_string());
        source = std::error::Error::source(cause);
        depth += 1;
    }
    PlatformError::Network(format!("{kind}: {detail}"))
}

// -- token storage ----------------------------------------------------------------------------------------------

/// Stored (encrypted) sign-in state.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredToken {
    refresh_token: String,
    #[serde(default)]
    account: Option<String>,
}

impl fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredToken")
            .field("refresh_token", &"<redacted>")
            .field("account", &self.account)
            .finish()
    }
}

mod token_store {
    use std::ffi::OsString;
    use std::fs;
    use std::io::ErrorKind;

    use super::{Path, PathBuf, PlatformError, StoredToken};

    /// Encrypted storage is available on this platform.
    pub(super) const SUPPORTED: bool = cfg!(target_os = "windows");

    pub(super) fn encode(token: &StoredToken) -> Result<Vec<u8>, PlatformError> {
        serde_json::to_vec(token).map_err(|err| PlatformError::Io(format!("token encode: {err}")))
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<StoredToken, PlatformError> {
        serde_json::from_slice(bytes)
            .map_err(|err| PlatformError::Io(format!("stored token is corrupted: {err}")))
    }

    /// `Ok(None)` when no token is stored.
    pub(super) fn load(path: &Path) -> Result<Option<StoredToken>, PlatformError> {
        let sealed = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let plain = protect::unprotect(&sealed)?;
        decode(&plain).map(Some)
    }

    pub(super) fn save(path: &Path, token: &StoredToken) -> Result<(), PlatformError> {
        let plain = encode(token)?;
        let sealed = protect::protect(&plain)?;
        write_atomic(path, &sealed)
    }

    pub(super) fn delete(path: &Path) -> Result<(), PlatformError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    fn temp_path(path: &Path) -> PathBuf {
        let mut name = OsString::from(path.as_os_str());
        name.push(".tmp");
        PathBuf::from(name)
    }

    pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PlatformError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let tmp = temp_path(path);
        fs::write(&tmp, bytes)?;
        if let Err(err) = fs::rename(&tmp, path) {
            let _ = fs::remove_file(&tmp);
            return Err(err.into());
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    mod protect {
        use std::ffi::c_void;

        use windows::Win32::Foundation::{HLOCAL, LocalFree};
        use windows::Win32::Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
        };
        use windows::core::w;

        use crate::PlatformError;

        /// Extra entropy so other apps of the same user cannot trivially unprotect the blob.
        const ENTROPY: &[u8] = b"FocusMode/teams-token/v1";

        fn blob(data: &[u8]) -> Result<CRYPT_INTEGER_BLOB, PlatformError> {
            let len = u32::try_from(data.len())
                .map_err(|_| PlatformError::Io("token too large".into()))?;
            Ok(CRYPT_INTEGER_BLOB {
                cbData: len,
                // DPAPI never writes through the input pointers.
                pbData: data.as_ptr().cast_mut(),
            })
        }

        /// Copies and frees a DPAPI output blob.
        fn take(output: CRYPT_INTEGER_BLOB) -> Vec<u8> {
            if output.pbData.is_null() {
                return Vec::new();
            }
            let len = usize::try_from(output.cbData).unwrap_or(0);
            // SAFETY: on success DPAPI returns `cbData` readable bytes at `pbData`, allocated with LocalAlloc.
            let bytes = unsafe { std::slice::from_raw_parts(output.pbData, len) }.to_vec();
            // SAFETY: freed exactly once with the allocator DPAPI documents (LocalFree).
            let _ = unsafe { LocalFree(Some(HLOCAL(output.pbData.cast::<c_void>()))) };
            bytes
        }

        pub(super) fn protect(plain: &[u8]) -> Result<Vec<u8>, PlatformError> {
            let input = blob(plain)?;
            let entropy = blob(ENTROPY)?;
            let mut output = CRYPT_INTEGER_BLOB::default();
            // SAFETY: input/entropy point to live slices for the call; `output` is a valid out blob.
            unsafe {
                CryptProtectData(
                    &input,
                    w!("FocusMode Teams token"),
                    Some(&entropy as *const CRYPT_INTEGER_BLOB),
                    None,
                    None,
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
            .map_err(|err| PlatformError::Os(format!("CryptProtectData failed: {err}")))?;
            Ok(take(output))
        }

        pub(super) fn unprotect(sealed: &[u8]) -> Result<Vec<u8>, PlatformError> {
            let input = blob(sealed)?;
            let entropy = blob(ENTROPY)?;
            let mut output = CRYPT_INTEGER_BLOB::default();
            // SAFETY: input/entropy point to live slices for the call; `output` is a valid out blob.
            unsafe {
                CryptUnprotectData(
                    &input,
                    None,
                    Some(&entropy as *const CRYPT_INTEGER_BLOB),
                    None,
                    None,
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
            .map_err(|err| PlatformError::Os(format!("CryptUnprotectData failed: {err}")))?;
            Ok(take(output))
        }
    }

    #[cfg(not(target_os = "windows"))]
    mod protect {
        use crate::PlatformError;

        pub(super) fn protect(_plain: &[u8]) -> Result<Vec<u8>, PlatformError> {
            Err(PlatformError::Unsupported)
        }

        pub(super) fn unprotect(_sealed: &[u8]) -> Result<Vec<u8>, PlatformError> {
            Err(PlatformError::Unsupported)
        }
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
        assert!(token_store::decode(b"not json").is_err());
    }

    #[test]
    fn debug_output_redacts_secrets() {
        let token = StoredToken {
            refresh_token: "secret-refresh".into(),
            account: None,
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
        let login = device_login_from(parsed, Instant::now());
        assert_eq!(login.expires_in_secs, 900);
        assert_eq!(login.interval_secs, 5);
        assert_eq!(login.user_code, "UC");
        assert!(!login.is_expired());

        let parsed: DeviceCodeResponse =
            serde_json::from_str(r#"{"device_code":"dc","user_code":"UC","verification_uri":"u"}"#)
                .expect("parse");
        let login = device_login_from(parsed, Instant::now());
        assert_eq!(login.expires_in_secs, DEFAULT_DEVICE_CODE_LIFETIME_SECS);
        assert_eq!(login.interval_secs, DEFAULT_POLL_INTERVAL_SECS);
        assert_eq!(login.message, "");
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
        assert!(client.begin_login().is_err());

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
    }

    #[test]
    fn atomic_write_replaces_file() {
        let path = unique_temp_path("atomic.bin");
        token_store::write_atomic(&path, b"one").expect("first write");
        token_store::write_atomic(&path, b"two").expect("second write");
        assert_eq!(std::fs::read(&path).expect("read"), b"two");
        token_store::delete(&path).expect("delete");
        token_store::delete(&path).expect("delete twice");
        assert!(!path.exists());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dpapi_token_store_round_trip() {
        let path = unique_temp_path("teams-token.bin");
        let token = StoredToken {
            refresh_token: "rt-123".into(),
            account: Some("ala@example.com".into()),
        };
        token_store::save(&path, &token).expect("save");
        let raw = std::fs::read(&path).expect("read raw");
        assert!(
            !String::from_utf8_lossy(&raw).contains("rt-123"),
            "stored encrypted"
        );
        assert_eq!(token_store::load(&path).expect("load"), Some(token));

        let client = TeamsClient::new(settings("id", path.clone()));
        assert!(client.has_token());
        assert_eq!(client.account().as_deref(), Some("ala@example.com"));

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
        };
        assert!(matches!(
            token_store::save(&path, &token),
            Err(PlatformError::Unsupported)
        ));
        assert!(!token_store::SUPPORTED);
    }
}
