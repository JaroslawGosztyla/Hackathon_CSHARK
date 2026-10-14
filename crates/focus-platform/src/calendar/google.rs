//! Google Calendar, read-only.
//!
//! * Sign-in: OAuth 2.0 for desktop apps, authorization code with PKCE (`S256`) and a loopback redirect to
//!   `http://127.0.0.1:<free port>/` (a `std::net::TcpListener` answers the single redirect). The user creates a
//!   Google Cloud OAuth client of type "Desktop app", enables the Google Calendar API and pastes the client id.
//!   Google documents the client secret of a Desktop client as not confidential, yet its token endpoint usually
//!   still asks for it: [`GoogleSettings::client_secret`] is sent when non-empty, and a "client_secret is missing"
//!   answer becomes a readable error.
//! * Scopes: [`SCOPES`] (`calendar.readonly`, plus `openid email` only to name the account in Settings).
//! * Flow: [`GoogleClient::begin_login`] binds the listener and returns the URL to open in the system browser;
//!   then call [`GoogleClient::poll_login`] until it is no longer [`GooglePoll::Pending`]. The browser gets a small
//!   Polish page ("Możesz zamknąć tę kartę").
//! * Events: `GET calendars/primary/events?singleEvents=true&orderBy=startTime&timeMin&timeMax` (recurring
//!   events expanded by Google, at most 3 pages of 250). Cancelled events, events the user declined, "free"
//!   (transparent) events and working-location / out-of-office / focus-time / birthday entries are left out.
//! * Token storage: JSON `{refreshToken, account}` sealed with DPAPI ([`crate::secrets`]) at
//!   [`GoogleSettings::token_path`]; access tokens live only in memory. `invalid_grant` (revoked access, or the
//!   7-day refresh token of an app left in the "Testing" publishing status) deletes the stored token; a refused
//!   client id or secret (`invalid_client`, e.g. a typo in Settings) keeps it.
//!
//! All calls are **blocking** (10 s timeouts): use [`GoogleClient`] on a worker thread, never inside an async
//! runtime. Tokens, authorization codes and the PKCE verifier are never logged.

use std::fmt;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::time::{MS_PER_DAY, parse_iso_date, parse_rfc3339};
use super::{CalEvent, TimeZone, clean_subject, format_utc, looks_online, meetings_only};
use crate::net::{LazyClient, first_line, network_error};
use crate::{PlatformError, base64, secrets, sha256};

/// Scopes requested at sign-in.
pub const SCOPES: &str = "https://www.googleapis.com/auth/calendar.readonly openid email";
/// How long a started sign-in waits for the browser redirect.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const EVENTS_URL: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";
const MAX_PAGES: usize = 3;
const PAGE_SIZE: u32 = 250;
const MAX_REDIRECTS: usize = 3;
/// Access tokens are treated as expired this long before their real expiry.
const EXPIRY_MARGIN: Duration = Duration::from_secs(120);
const DEFAULT_TOKEN_LIFETIME_SECS: u64 = 3600;
/// Pause between accept attempts while waiting for the redirect.
const ACCEPT_POLL: Duration = Duration::from_millis(50);
/// A browser connection must deliver its request within this time (idle pre-connections are dropped).
const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_REQUEST_BYTES: usize = 16 * 1024;
/// Calendar entries that are not meetings.
const SKIPPED_EVENT_TYPES: &[&str] = &["workingLocation", "outOfOffice", "focusTime", "birthday"];
/// Granted scopes that allow reading events.
const CALENDAR_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/calendar.readonly",
    "https://www.googleapis.com/auth/calendar",
    "https://www.googleapis.com/auth/calendar.events.readonly",
    "https://www.googleapis.com/auth/calendar.events",
];

/// Connection settings (from config + app data dir).
#[derive(Clone, PartialEq, Eq)]
pub struct GoogleSettings {
    /// OAuth client id of the user's Google Cloud "Desktop app" client.
    pub client_id: String,
    /// Client secret of that client, or empty (sent only when non-empty).
    pub client_secret: String,
    /// Where the encrypted refresh token is stored (e.g. `app_data_dir/google-token.bin`).
    pub token_path: PathBuf,
}

impl fmt::Debug for GoogleSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoogleSettings")
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &if self.client_secret.is_empty() {
                    ""
                } else {
                    "<redacted>"
                },
            )
            .field("token_path", &self.token_path)
            .finish()
    }
}

/// A started sign-in: open [`GoogleLogin::auth_url`] in the system browser, then call
/// [`GoogleClient::poll_login`] until it is no longer [`GooglePoll::Pending`]. Dropping it closes the listener.
pub struct GoogleLogin {
    /// URL of Google's consent screen (carries no secret, but there is no reason to log it).
    pub auth_url: String,
    pub expires_in_secs: u32,
    listener: TcpListener,
    redirect_uri: String,
    verifier: String,
    state: String,
    started_at: Instant,
}

impl GoogleLogin {
    /// The sign-in waited longer than [`LOGIN_TIMEOUT`].
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() >= Duration::from_secs(u64::from(self.expires_in_secs))
    }

    /// Loopback port of the redirect listener.
    pub fn port(&self) -> Option<u16> {
        self.listener.local_addr().ok().map(|addr| addr.port())
    }
}

impl fmt::Debug for GoogleLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoogleLogin")
            .field("redirect_uri", &self.redirect_uri)
            .field("expires_in_secs", &self.expires_in_secs)
            .field("verifier", &"<redacted>")
            .field("state", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// Result of one [`GoogleClient::poll_login`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GooglePoll {
    /// No redirect yet; call again.
    Pending,
    /// Signed in; the refresh token is stored. `account` = the e-mail of the Google account.
    Done { account: Option<String> },
    /// The sign-in ended without success (declined, timed out, misconfigured client…). English detail.
    Failed(String),
}

/// Google Calendar client. Blocking; see the module docs.
pub struct GoogleClient {
    settings: GoogleSettings,
    http: LazyClient,
    access: Option<AccessToken>,
}

struct AccessToken {
    token: String,
    expires_at: Instant,
}

impl GoogleClient {
    /// Cheap: no I/O, no network.
    pub fn new(settings: GoogleSettings) -> Self {
        Self {
            settings,
            http: LazyClient::new(MAX_REDIRECTS),
            access: None,
        }
    }

    pub fn settings(&self) -> &GoogleSettings {
        &self.settings
    }

    /// A client id is configured.
    pub fn is_configured(&self) -> bool {
        !self.client_id().is_empty()
    }

    /// A stored token exists and can be decrypted (it may still be revoked server-side).
    pub fn has_token(&self) -> bool {
        matches!(load_token(&self.settings.token_path), Ok(Some(_)))
    }

    /// E-mail of the signed-in account.
    pub fn account(&self) -> Option<String> {
        load_token(&self.settings.token_path)
            .ok()
            .flatten()
            .and_then(|stored| stored.account)
    }

    /// Binds the loopback listener and builds the consent URL (no network).
    pub fn begin_login(&self) -> Result<GoogleLogin, PlatformError> {
        if !secrets::SUPPORTED {
            return Err(PlatformError::Unsupported);
        }
        self.start_login()
    }

    fn start_login(&self) -> Result<GoogleLogin, PlatformError> {
        self.ensure_configured()?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}/");
        let verifier = base64::encode_url(&secrets::random_bytes(32)?);
        let state = base64::encode_url(&secrets::random_bytes(16)?);
        let auth_url = authorization_url(
            self.client_id(),
            &redirect_uri,
            &pkce_challenge(&verifier),
            &state,
        )?;
        Ok(GoogleLogin {
            auth_url,
            expires_in_secs: u32::try_from(LOGIN_TIMEOUT.as_secs()).unwrap_or(u32::MAX),
            listener,
            redirect_uri,
            verifier,
            state,
            started_at: Instant::now(),
        })
    }

    /// Waits up to `wait` for the browser redirect and finishes the sign-in when it arrives (token exchange,
    /// storage). Unrelated requests (favicon, a forged `state`) are answered and ignored. `Err` only for a broken
    /// listener.
    pub fn poll_login(
        &mut self,
        login: &GoogleLogin,
        wait: Duration,
    ) -> Result<GooglePoll, PlatformError> {
        let deadline = Instant::now() + wait;
        loop {
            if login.is_expired() {
                return Ok(GooglePoll::Failed("the sign-in timed out".into()));
            }
            match login.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(outcome) = self.handle_connection(login, stream) {
                        return Ok(outcome);
                    }
                    if Instant::now() >= deadline {
                        return Ok(GooglePoll::Pending);
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Ok(GooglePoll::Pending);
                    }
                    std::thread::sleep(ACCEPT_POLL.min(deadline - now));
                }
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => return Err(err.into()),
            }
        }
    }

    /// Meetings of the primary calendar overlapping `[start_ms, end_ms)`.
    pub fn events(&mut self, start_ms: i64, end_ms: i64) -> Result<Vec<CalEvent>, PlatformError> {
        if end_ms <= start_ms {
            return Ok(Vec::new());
        }
        self.ensure_configured()?;
        let local = TimeZone::local();
        let mut events = Vec::new();
        let mut page_token: Option<String> = None;
        for page in 0..MAX_PAGES {
            let url = events_url(start_ms, end_ms, page_token.as_deref())?;
            let body = self.api_get(&url)?;
            let parsed = parse_events_page(&body, &local)?;
            events.extend(parsed.events);
            match parsed.next_page_token {
                Some(token) if !token.is_empty() => {
                    if page + 1 == MAX_PAGES {
                        log::debug!(
                            "google calendar: more than {MAX_PAGES} pages, the rest is ignored"
                        );
                    }
                    page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(meetings_only(events, start_ms, end_ms))
    }

    /// Revokes the refresh token at Google (best effort) and forgets it locally.
    pub fn disconnect(&mut self) -> Result<(), PlatformError> {
        self.access = None;
        if let Ok(Some(stored)) = load_token(&self.settings.token_path)
            && let Err(err) = self.revoke(&stored.refresh_token)
        {
            log::info!("google calendar: revoking the token failed: {err}");
        }
        secrets::delete_sealed(&self.settings.token_path)
    }

    // -- internals ------------------------------------------------------------------------------------------

    fn client_id(&self) -> &str {
        self.settings.client_id.trim()
    }

    fn client_secret(&self) -> &str {
        self.settings.client_secret.trim()
    }

    fn ensure_configured(&self) -> Result<(), PlatformError> {
        if self.is_configured() {
            Ok(())
        } else {
            Err(PlatformError::Auth(
                "Google client id is not configured".into(),
            ))
        }
    }

    /// Answers one browser connection; `Some` when the sign-in is over.
    fn handle_connection(
        &mut self,
        login: &GoogleLogin,
        mut stream: TcpStream,
    ) -> Option<GooglePoll> {
        let (method, target) = read_request(&mut stream)?;
        if method != "GET" {
            respond(&mut stream, 405, Page::NotFound);
            return None;
        }
        match parse_callback(&target, &login.state) {
            Callback::Other => {
                respond(&mut stream, 404, Page::NotFound);
                None
            }
            Callback::BadState => {
                log::warn!("google calendar: ignoring a sign-in redirect with a wrong state");
                respond(&mut stream, 400, Page::BadState);
                None
            }
            Callback::Error(code) => {
                respond(&mut stream, 200, Page::Cancelled);
                Some(GooglePoll::Failed(if code == "access_denied" {
                    "sign-in was declined".into()
                } else {
                    format!("Google sign-in failed: {code}")
                }))
            }
            Callback::Code(code) => {
                let outcome = self
                    .exchange_code(login, &code)
                    .unwrap_or_else(|err| GooglePoll::Failed(err.to_string()));
                let page = if matches!(outcome, GooglePoll::Done { .. }) {
                    Page::Success
                } else {
                    Page::Failure
                };
                respond(&mut stream, 200, page);
                Some(outcome)
            }
        }
    }

    fn exchange_code(
        &mut self,
        login: &GoogleLogin,
        code: &str,
    ) -> Result<GooglePoll, PlatformError> {
        let client_id = self.client_id().to_owned();
        let client_secret = self.client_secret().to_owned();
        let params = with_client_secret(
            vec![
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", client_id.as_str()),
                ("redirect_uri", login.redirect_uri.as_str()),
                ("code_verifier", login.verifier.as_str()),
            ],
            &client_secret,
        );
        let response = self
            .http
            .get()?
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().map_err(network_error)?;
        if !status.is_success() {
            return Ok(GooglePoll::Failed(token_error_text(status.as_u16(), &body)));
        }
        let tokens: TokenResponse = serde_json::from_str(&body)
            .map_err(|err| PlatformError::Auth(format!("invalid token response: {err}")))?;
        let Some(refresh_token) = tokens.refresh_token.clone().filter(|t| !t.is_empty()) else {
            return Ok(GooglePoll::Failed(
                "Google returned no refresh token; remove FocusMode's access in your Google account and connect again"
                    .into(),
            ));
        };
        if let Some(scope) = tokens.scope.as_deref()
            && !has_calendar_scope(scope)
        {
            return Ok(GooglePoll::Failed(
                "calendar access was not granted (tick \"See your calendars\" on Google's consent screen)".into(),
            ));
        }
        let account = tokens.id_token.as_deref().and_then(email_from_id_token);
        save_token(
            &self.settings.token_path,
            &StoredToken {
                refresh_token,
                account: account.clone(),
            },
        )?;
        self.remember_access(&tokens);
        log::info!("google calendar: signed in");
        Ok(GooglePoll::Done { account })
    }

    fn remember_access(&mut self, tokens: &TokenResponse) {
        let lifetime = if tokens.expires_in == 0 {
            DEFAULT_TOKEN_LIFETIME_SECS
        } else {
            tokens.expires_in
        };
        self.access = Some(AccessToken {
            token: tokens.access_token.clone(),
            expires_at: Instant::now()
                + Duration::from_secs(lifetime).saturating_sub(EXPIRY_MARGIN),
        });
    }

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
        let stored = load_token(&self.settings.token_path)?
            .ok_or_else(|| PlatformError::Auth("not connected to Google Calendar".into()))?;
        let client_id = self.client_id().to_owned();
        let client_secret = self.client_secret().to_owned();
        let params = with_client_secret(
            vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", stored.refresh_token.as_str()),
                ("client_id", client_id.as_str()),
            ],
            &client_secret,
        );
        let response = self
            .http
            .get()?
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().map_err(network_error)?;
        if !status.is_success() {
            self.access = None;
            let (forget_token, error) = refresh_failure(status.as_u16(), &body);
            if forget_token && let Err(err) = secrets::delete_sealed(&self.settings.token_path) {
                log::warn!("google calendar: deleting the rejected token failed: {err}");
            }
            return Err(error);
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
            if let Err(err) = save_token(&self.settings.token_path, &updated) {
                log::warn!("google calendar: storing the rotated refresh token failed: {err}");
            }
        }
        self.remember_access(&tokens);
        Ok(tokens.access_token)
    }

    /// GET with the access token; retries once with a refreshed token on 401.
    fn api_get(&mut self, url: &str) -> Result<String, PlatformError> {
        for attempt in 0..2 {
            let token = self.access_token()?;
            let response = self
                .http
                .get()?
                .get(url)
                .bearer_auth(token)
                .send()
                .map_err(network_error)?;
            let status = response.status();
            if status.is_success() {
                return response.text().map_err(network_error);
            }
            let body = response.text().unwrap_or_default();
            if status.as_u16() == 401 && attempt == 0 {
                log::info!("google calendar: access token rejected, refreshing");
                self.access = None;
                continue;
            }
            return Err(api_error(status.as_u16(), &body));
        }
        Err(PlatformError::Auth(
            "reconnect needed: Google rejected the refreshed access token".into(),
        ))
    }

    fn revoke(&self, refresh_token: &str) -> Result<(), PlatformError> {
        let response = self
            .http
            .get()?
            .post(REVOKE_URL)
            .form(&[("token", refresh_token)])
            .send()
            .map_err(network_error)?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(PlatformError::Network(format!(
                "revoke endpoint answered HTTP {}",
                status.as_u16()
            )))
        }
    }
}

// -- OAuth helpers -----------------------------------------------------------------------------------------------

/// Token endpoint form (code exchange and refresh) with `client_secret` appended when the (trimmed) secret is
/// not empty. Google requires it for "Desktop app" clients even with PKCE.
fn with_client_secret<'a>(
    mut params: Vec<(&'static str, &'a str)>,
    client_secret: &'a str,
) -> Vec<(&'static str, &'a str)> {
    let client_secret = client_secret.trim();
    if !client_secret.is_empty() {
        params.push(("client_secret", client_secret));
    }
    params
}

/// A refused refresh → (forget the stored refresh token, error). Only `invalid_grant` (revoked access, an
/// expired refresh token) means the token is dead. A refused client id or secret (`invalid_client`,
/// `unauthorized_client`: a typo while editing them in Settings, a deleted OAuth client) keeps it, so correcting
/// the settings is enough; a new sign-in replaces it anyway.
fn refresh_failure(status: u16, body: &str) -> (bool, PlatformError) {
    let error: OAuthErrorResponse = serde_json::from_str(body).unwrap_or_default();
    match error.error.as_str() {
        "invalid_grant" => (
            true,
            PlatformError::Auth(format!(
                "reconnect needed: the Google sign-in expired or was revoked ({})",
                error.error
            )),
        ),
        "invalid_client" | "unauthorized_client" => (
            false,
            PlatformError::Auth(format!(
                "Google refused the OAuth client: check the client id and secret ({})",
                token_error_text(status, body)
            )),
        ),
        _ => (false, PlatformError::Auth(token_error_text(status, body))),
    }
}

/// PKCE `S256` challenge: base64url(SHA-256(verifier)) (RFC 7636).
fn pkce_challenge(verifier: &str) -> String {
    base64::encode_url(&sha256::sha256(verifier.as_bytes()))
}

fn authorization_url(
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
) -> Result<String, PlatformError> {
    let url = reqwest::Url::parse_with_params(
        AUTH_URL,
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            // A refresh token, and a new one on every consent (Google sends it only at the first consent).
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
    .map_err(|err| PlatformError::Auth(format!("cannot build the sign-in URL: {err}")))?;
    Ok(url.into())
}

fn events_url(
    start_ms: i64,
    end_ms: i64,
    page_token: Option<&str>,
) -> Result<String, PlatformError> {
    let mut url = reqwest::Url::parse(EVENTS_URL)
        .map_err(|err| PlatformError::Network(format!("events URL: {err}")))?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("timeMin", &format_utc(start_ms))
            .append_pair("timeMax", &format_utc(end_ms))
            .append_pair("singleEvents", "true")
            .append_pair("orderBy", "startTime")
            .append_pair("maxResults", &PAGE_SIZE.to_string());
        if let Some(token) = page_token {
            query.append_pair("pageToken", token);
        }
    }
    Ok(url.into())
}

/// Every granted scope string is checked; one calendar scope is enough.
fn has_calendar_scope(granted: &str) -> bool {
    granted
        .split_whitespace()
        .any(|scope| CALENDAR_SCOPES.contains(&scope))
}

/// The `email` claim of an ID token received directly from Google's token endpoint over TLS (OpenID Connect
/// allows skipping the signature check in that case). Only used as the account label.
fn email_from_id_token(id_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Claims {
        #[serde(default)]
        email: Option<String>,
    }
    let payload = id_token.split('.').nth(1)?;
    let bytes = base64::decode_url(payload)?;
    serde_json::from_slice::<Claims>(&bytes)
        .ok()?
        .email
        .map(|email| email.trim().to_owned())
        .filter(|email| !email.is_empty() && email.len() <= 320)
}

fn token_error_text(status: u16, body: &str) -> String {
    match serde_json::from_str::<OAuthErrorResponse>(body) {
        Ok(error) if !error.error.is_empty() => {
            let description = error
                .error_description
                .as_deref()
                .map(first_line)
                .unwrap_or("");
            if description.to_ascii_lowercase().contains("client_secret") {
                return format!(
                    "Google requires the client secret of this OAuth client ({}: {description})",
                    error.error
                );
            }
            if description.is_empty() {
                error.error
            } else {
                format!("{}: {description}", error.error)
            }
        }
        _ => format!("HTTP {status}"),
    }
}

/// Calendar API error → [`PlatformError`].
fn api_error(status: u16, body: &str) -> PlatformError {
    let parsed: ApiErrorResponse = serde_json::from_str(body).unwrap_or_default();
    let reasons: Vec<String> = parsed
        .error
        .errors
        .iter()
        .map(|item| item.reason.clone())
        .chain(parsed.error.details.iter().filter_map(|detail| {
            detail
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        }))
        .collect();
    let has = |names: &[&str]| reasons.iter().any(|r| names.contains(&r.as_str()));
    let detail = if parsed.error.message.is_empty() {
        format!("HTTP {status}")
    } else {
        format!("HTTP {status}: {}", first_line(&parsed.error.message))
    };
    match status {
        401 => PlatformError::Auth(format!("reconnect needed: {detail}")),
        403 | 429
            if has(&[
                "rateLimitExceeded",
                "userRateLimitExceeded",
                "RATE_LIMIT_EXCEEDED",
            ]) =>
        {
            PlatformError::Network(format!(
                "Google Calendar rate limit, try again later ({detail})"
            ))
        }
        403 if has(&["insufficientPermissions", "ACCESS_TOKEN_SCOPE_INSUFFICIENT"]) => {
            PlatformError::Auth(format!(
                "reconnect needed: Google Calendar access was not granted ({detail})"
            ))
        }
        403 if has(&["accessNotConfigured", "SERVICE_DISABLED"]) => PlatformError::Auth(format!(
            "the Google Calendar API is not enabled for this client id; enable it in the Google Cloud console ({detail})"
        )),
        403 => PlatformError::Auth(detail),
        404 => PlatformError::Network(format!("the primary calendar was not found ({detail})")),
        _ => PlatformError::Network(detail),
    }
}

// -- loopback redirect -----------------------------------------------------------------------------------------

/// What a request to the loopback listener is.
#[derive(Debug, PartialEq, Eq)]
enum Callback {
    /// Not the redirect (favicon, other paths, no parameters).
    Other,
    /// A redirect with a missing or wrong `state` (not from our sign-in).
    BadState,
    /// Google reported an error (`access_denied` when the user cancelled); sanitized code.
    Error(String),
    Code(String),
}

fn parse_callback(target: &str, expected_state: &str) -> Callback {
    if !target.starts_with('/') {
        return Callback::Other;
    }
    let Ok(url) = reqwest::Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Callback::Other;
    };
    if url.path() != "/" {
        return Callback::Other;
    }
    let (mut code, mut error, mut state) = (None, None, None);
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }
    if code.is_none() && error.is_none() {
        return Callback::Other;
    }
    if expected_state.is_empty() || state.as_deref() != Some(expected_state) {
        return Callback::BadState;
    }
    if let Some(error) = error {
        let sanitized: String = error
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(64)
            .collect();
        return Callback::Error(sanitized);
    }
    match code {
        Some(code) if !code.is_empty() => Callback::Code(code),
        _ => Callback::Other,
    }
}

/// Reads the request head; `(method, target)` of the request line. The whole read takes at most
/// [`REQUEST_READ_TIMEOUT`] (a client sending a byte now and then cannot hold the sign-in).
fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    let deadline = Instant::now() + REQUEST_READ_TIMEOUT;
    stream.set_nonblocking(false).ok()?;
    stream.set_write_timeout(Some(REQUEST_READ_TIMEOUT)).ok()?;
    let mut head = Vec::new();
    let mut buffer = [0u8; 2048];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || stream.set_read_timeout(Some(left)).is_err() {
            break;
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                head.extend_from_slice(&buffer[..read]);
                if head.windows(4).any(|w| w == b"\r\n\r\n") || head.len() >= MAX_REQUEST_BYTES {
                    break;
                }
            }
        }
    }
    let text = String::from_utf8_lossy(&head);
    parse_request_line(text.lines().next()?)
}

fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    let version = parts.next()?;
    version
        .starts_with("HTTP/")
        .then(|| (method.to_owned(), target.to_owned()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Success,
    Failure,
    Cancelled,
    BadState,
    NotFound,
}

impl Page {
    fn texts(self) -> (&'static str, &'static str) {
        match self {
            Page::Success => (
                "Połączono z Kalendarzem Google",
                "Możesz zamknąć tę kartę i wrócić do FocusMode.",
            ),
            Page::Failure => (
                "Nie udało się połączyć z Kalendarzem Google",
                "Możesz zamknąć tę kartę. Szczegóły znajdziesz w ustawieniach FocusMode.",
            ),
            Page::Cancelled => ("Logowanie przerwane", "Możesz zamknąć tę kartę."),
            Page::BadState => (
                "Nieprawidłowe żądanie logowania",
                "Możesz zamknąć tę kartę i spróbować ponownie w FocusMode.",
            ),
            Page::NotFound => ("Nie znaleziono", "Możesz zamknąć tę kartę."),
        }
    }

    fn html(self) -> String {
        let (title, message) = self.texts();
        format!(
            "<!doctype html><html lang=\"pl\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>FocusMode</title>\
             <style>body{{margin:0;min-height:100vh;display:grid;place-items:center;\
             font-family:system-ui,sans-serif;background:#15131f;color:#ece9f6}}\
             main{{max-width:28rem;padding:2rem;text-align:center}}h1{{font-size:1.4rem}}</style></head>\
             <body><main><h1>{title}</h1><p>{message}</p></main></body></html>"
        )
    }
}

fn respond(stream: &mut TcpStream, status: u16, page: Page) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let body = page.html();
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let written = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body.as_bytes()))
        .and_then(|()| stream.flush());
    if let Err(err) = written {
        log::debug!("google calendar: answering the browser failed: {err}");
    }
    let _ = stream.shutdown(Shutdown::Write);
}

// -- wire formats -----------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: u64,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OAuthErrorResponse {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ApiErrorResponse {
    #[serde(default)]
    error: ApiErrorBody,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ApiErrorBody {
    message: String,
    errors: Vec<ApiErrorItem>,
    details: Vec<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ApiErrorItem {
    reason: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct EventsPage {
    items: Option<Vec<serde_json::Value>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GoogleEvent {
    id: String,
    status: Option<String>,
    summary: Option<String>,
    start: Option<GoogleTime>,
    end: Option<GoogleTime>,
    transparency: Option<String>,
    event_type: Option<String>,
    attendees: Option<Vec<GoogleAttendee>>,
    hangout_link: Option<String>,
    conference_data: Option<serde_json::Value>,
    location: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GoogleTime {
    date: Option<String>,
    date_time: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GoogleAttendee {
    #[serde(rename = "self")]
    is_self: Option<bool>,
    response_status: Option<String>,
}

struct ParsedPage {
    events: Vec<CalEvent>,
    next_page_token: Option<String>,
}

fn parse_events_page(body: &str, local: &TimeZone) -> Result<ParsedPage, PlatformError> {
    let page: EventsPage = serde_json::from_str(body)
        .map_err(|err| PlatformError::Network(format!("invalid events response: {err}")))?;
    let events = page
        .items
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| serde_json::from_value::<GoogleEvent>(item).ok())
        .filter_map(|event| convert_event(event, local))
        .collect();
    Ok(ParsedPage {
        events,
        next_page_token: page.next_page_token,
    })
}

/// `None` for entries that are not meetings the user attends.
fn convert_event(event: GoogleEvent, local: &TimeZone) -> Option<CalEvent> {
    if event.status.as_deref() == Some("cancelled") {
        return None;
    }
    if event
        .event_type
        .as_deref()
        .is_some_and(|kind| SKIPPED_EVENT_TYPES.contains(&kind))
    {
        return None;
    }
    let declined = event
        .attendees
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|a| a.is_self == Some(true) && a.response_status.as_deref() == Some("declined"));
    if declined {
        return None;
    }
    let (start, end) = (event.start?, event.end?);
    let (start_ms, end_ms, all_day) = match (start.date_time, end.date_time) {
        (Some(s), Some(e)) => (parse_rfc3339(&s)?, parse_rfc3339(&e)?, false),
        _ => {
            let first = parse_iso_date(start.date.as_deref()?)?;
            let last = parse_iso_date(end.date.as_deref()?)?;
            (
                local.wall_to_utc(first * MS_PER_DAY),
                local.wall_to_utc(last * MS_PER_DAY),
                true,
            )
        }
    };
    let online = event.hangout_link.as_deref().is_some_and(|l| !l.is_empty())
        || event.conference_data.as_ref().is_some_and(|c| !c.is_null())
        || event.location.as_deref().is_some_and(looks_online)
        || event.description.as_deref().is_some_and(looks_online);
    Some(CalEvent {
        id: event.id,
        subject: clean_subject(event.summary.as_deref().unwrap_or("")),
        start_ms,
        end_ms,
        online,
        all_day,
        busy: event.transparency.as_deref() != Some("transparent"),
    })
}

// -- token storage ----------------------------------------------------------------------------------------------

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

fn load_token(path: &Path) -> Result<Option<StoredToken>, PlatformError> {
    let Some(plain) = secrets::read_sealed(path)? else {
        return Ok(None);
    };
    serde_json::from_slice(&plain)
        .map(Some)
        .map_err(|err| PlatformError::Io(format!("stored Google token is corrupted: {err}")))
}

fn save_token(path: &Path, token: &StoredToken) -> Result<(), PlatformError> {
    let plain = serde_json::to_vec(token)
        .map_err(|err| PlatformError::Io(format!("token encode: {err}")))?;
    secrets::write_sealed(path, &plain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::time::{MS_PER_HOUR, days_from_civil};

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

    fn settings(client_id: &str) -> GoogleSettings {
        GoogleSettings {
            client_id: client_id.into(),
            client_secret: String::new(),
            token_path: unique_temp_path("google-token.bin"),
        }
    }

    #[test]
    fn pkce_challenge_is_base64url_sha256() {
        // Cross-checked with `openssl dgst -sha256 -binary | openssl base64` (+/ → -_, no padding).
        let challenge = pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWoOEjXk");
        assert_eq!(challenge, "mhjLJKswqsV8ZoE32OTj1PuM71PZ91jv32g00FpQye0");
        assert_eq!(challenge.len(), 43);
        assert!(!challenge.contains(['+', '/', '=']));
    }

    #[test]
    fn authorization_url_has_the_flow_parameters() {
        let url = authorization_url(
            "123-abc.apps.googleusercontent.com",
            "http://127.0.0.1:51234/",
            "challenge",
            "state-1",
        )
        .expect("url");
        let parsed = reqwest::Url::parse(&url).expect("parse");
        assert_eq!(parsed.host_str(), Some("accounts.google.com"));
        let pairs: std::collections::HashMap<String, String> = parsed
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(pairs["client_id"], "123-abc.apps.googleusercontent.com");
        assert_eq!(pairs["redirect_uri"], "http://127.0.0.1:51234/");
        assert_eq!(pairs["response_type"], "code");
        assert_eq!(pairs["scope"], SCOPES);
        assert_eq!(pairs["code_challenge"], "challenge");
        assert_eq!(pairs["code_challenge_method"], "S256");
        assert_eq!(pairs["state"], "state-1");
        assert_eq!(pairs["access_type"], "offline");

        let events = events_url(0, 3_600_000, Some("next page")).expect("events url");
        assert!(events.starts_with(EVENTS_URL));
        assert!(events.contains("timeMin=1970-01-01T00%3A00%3A00Z"));
        assert!(events.contains("timeMax=1970-01-01T01%3A00%3A00Z"));
        assert!(events.contains("singleEvents=true"));
        assert!(events.contains("orderBy=startTime"));
        assert!(events.contains("pageToken=next+page"));
    }

    #[test]
    fn callbacks_are_classified() {
        assert_eq!(
            parse_callback("/?state=s1&code=4%2F0Ab&scope=x", "s1"),
            Callback::Code("4/0Ab".into())
        );
        assert_eq!(
            parse_callback("/?error=access_denied&state=s1", "s1"),
            Callback::Error("access_denied".into())
        );
        assert_eq!(
            parse_callback("/?error=%3Cscript%3E&state=s1", "s1"),
            Callback::Error("script".into())
        );
        assert_eq!(
            parse_callback("/?code=x&state=other", "s1"),
            Callback::BadState
        );
        assert_eq!(parse_callback("/?code=x", "s1"), Callback::BadState);
        assert_eq!(
            parse_callback("/?error=access_denied", "s1"),
            Callback::BadState
        );
        assert_eq!(parse_callback("/favicon.ico", "s1"), Callback::Other);
        assert_eq!(parse_callback("/", "s1"), Callback::Other);
        assert_eq!(
            parse_callback("/callback?code=x&state=s1", "s1"),
            Callback::Other
        );
        assert_eq!(
            parse_callback("http://evil/?code=x&state=s1", "s1"),
            Callback::Other
        );
        assert_eq!(parse_callback("/?code=&state=s1", "s1"), Callback::Other);
        assert_eq!(
            parse_request_line("GET /?code=1 HTTP/1.1"),
            Some(("GET".into(), "/?code=1".into()))
        );
        assert_eq!(parse_request_line("GET /"), None);
        assert_eq!(parse_request_line(""), None);
    }

    #[test]
    fn scopes_and_id_token() {
        assert!(has_calendar_scope(
            "openid https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/userinfo.email"
        ));
        assert!(!has_calendar_scope(
            "openid https://www.googleapis.com/auth/userinfo.email"
        ));
        let payload = base64::encode_url(
            br#"{"iss":"https://accounts.google.com","email":"ala@example.com"}"#,
        );
        let token = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.c2lnbmF0dXJl");
        assert_eq!(
            email_from_id_token(&token).as_deref(),
            Some("ala@example.com")
        );
        assert_eq!(email_from_id_token("not-a-jwt"), None);
        let no_email = format!("x.{}.y", base64::encode_url(br#"{"sub":"1"}"#));
        assert_eq!(email_from_id_token(&no_email), None);
    }

    #[test]
    fn token_errors_are_readable() {
        assert_eq!(
            token_error_text(
                400,
                r#"{"error":"invalid_request","error_description":"client_secret is missing."}"#
            ),
            "Google requires the client secret of this OAuth client (invalid_request: client_secret is missing.)"
        );
        assert_eq!(
            token_error_text(
                400,
                r#"{"error":"invalid_grant","error_description":"Bad Request"}"#
            ),
            "invalid_grant: Bad Request"
        );
        assert_eq!(
            token_error_text(400, r#"{"error":"invalid_grant"}"#),
            "invalid_grant"
        );
        assert_eq!(token_error_text(502, "<html>"), "HTTP 502");
    }

    #[test]
    fn only_a_dead_refresh_token_is_forgotten() {
        let (forget, err) = refresh_failure(
            400,
            r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#,
        );
        assert!(forget);
        assert!(matches!(err, PlatformError::Auth(ref m) if m.starts_with("reconnect needed")));
        // A wrong (half-typed) secret or client id keeps the sign-in: fixing the settings is enough.
        for body in [
            r#"{"error":"invalid_client","error_description":"Unauthorized"}"#,
            r#"{"error":"unauthorized_client"}"#,
        ] {
            let (forget, err) = refresh_failure(401, body);
            assert!(!forget, "{body}");
            assert!(
                matches!(err, PlatformError::Auth(ref m) if m.contains("check the client id and secret")),
                "{body}"
            );
        }
        let (forget, err) = refresh_failure(
            400,
            r#"{"error":"invalid_request","error_description":"client_secret is missing."}"#,
        );
        assert!(!forget);
        assert!(
            matches!(err, PlatformError::Auth(ref m) if m.starts_with("Google requires the client secret"))
        );
        assert!(!refresh_failure(503, "<html>").0);
    }

    #[test]
    fn client_secret_is_sent_only_when_set() {
        let base = || vec![("grant_type", "refresh_token"), ("client_id", "1-x")];
        assert_eq!(with_client_secret(base(), ""), base());
        assert_eq!(with_client_secret(base(), "  \t"), base());
        let with = with_client_secret(base(), " GOCSPX-abc_123 ");
        assert_eq!(with.len(), 3);
        assert_eq!(with[..2], base()[..]);
        assert_eq!(with[2], ("client_secret", "GOCSPX-abc_123"));
    }

    #[test]
    fn api_errors_are_classified() {
        let scope = r#"{"error":{"code":403,"message":"Request had insufficient authentication scopes.",
            "errors":[{"message":"Insufficient Permission","domain":"global","reason":"insufficientPermissions"}],
            "status":"PERMISSION_DENIED",
            "details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ACCESS_TOKEN_SCOPE_INSUFFICIENT"}]}}"#;
        assert!(
            matches!(api_error(403, scope), PlatformError::Auth(ref m) if m.starts_with("reconnect needed"))
        );
        let disabled = r#"{"error":{"code":403,"message":"Google Calendar API has not been used in project 1 before or it is disabled.",
            "errors":[{"reason":"accessNotConfigured"}],"details":[{"reason":"SERVICE_DISABLED"}]}}"#;
        assert!(
            matches!(api_error(403, disabled), PlatformError::Auth(ref m) if m.contains("not enabled"))
        );
        let limit = r#"{"error":{"code":403,"message":"Rate Limit Exceeded","errors":[{"reason":"rateLimitExceeded"}]}}"#;
        assert!(matches!(api_error(403, limit), PlatformError::Network(_)));
        assert!(
            matches!(api_error(401, ""), PlatformError::Auth(ref m) if m == "reconnect needed: HTTP 401")
        );
        assert!(matches!(api_error(404, "{}"), PlatformError::Network(_)));
        assert!(matches!(api_error(500, "oops"), PlatformError::Network(ref m) if m == "HTTP 500"));
    }

    #[test]
    fn events_page_is_converted() {
        let body = r#"{
          "kind": "calendar#events",
          "timeZone": "Europe/Warsaw",
          "nextPageToken": "CigKGjdrd",
          "items": [
            {"kind":"calendar#event","id":"standup_20261013T073000Z","status":"confirmed","summary":"Daily\nstandup",
             "start":{"dateTime":"2026-10-13T09:30:00+02:00","timeZone":"Europe/Warsaw"},
             "end":{"dateTime":"2026-10-13T09:45:00+02:00","timeZone":"Europe/Warsaw"},
             "hangoutLink":"https://meet.google.com/abc-defg-hij","eventType":"default",
             "attendees":[{"email":"ala@example.com","self":true,"responseStatus":"accepted"}]},
            {"id":"declined","status":"confirmed","summary":"Nope",
             "start":{"dateTime":"2026-10-13T11:00:00Z"},"end":{"dateTime":"2026-10-13T12:00:00Z"},
             "attendees":[{"email":"ala@example.com","self":true,"responseStatus":"declined"},
                          {"email":"bob@example.com","responseStatus":"accepted"}]},
            {"id":"cancelled","status":"cancelled"},
            {"id":"free","status":"confirmed","summary":"Blocker","transparency":"transparent",
             "start":{"dateTime":"2026-10-13T13:00:00Z"},"end":{"dateTime":"2026-10-13T14:00:00Z"}},
            {"id":"office","status":"confirmed","eventType":"workingLocation","summary":"Home",
             "start":{"date":"2026-10-13"},"end":{"date":"2026-10-14"}},
            {"id":"allday","status":"confirmed","summary":"Konferencja",
             "start":{"date":"2026-10-13"},"end":{"date":"2026-10-14"}},
            {"id":"zoom","status":"tentative","summary":"Partner call","location":"https://us02web.zoom.us/j/1",
             "start":{"dateTime":"2026-10-13T15:00:00Z"},"end":{"dateTime":"2026-10-13T15:30:00Z"}},
            {"id":"broken","start":{"dateTime":"yesterday"},"end":{"dateTime":"today"}},
            42
          ]
        }"#;
        let local = TimeZone::fixed(120);
        let page = parse_events_page(body, &local).expect("page");
        assert_eq!(page.next_page_token.as_deref(), Some("CigKGjdrd"));
        let ids: Vec<&str> = page.events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["standup_20261013T073000Z", "free", "allday", "zoom"]
        );

        let day = days_from_civil(2026, 10, 13) * MS_PER_DAY;
        let standup = &page.events[0];
        assert_eq!(standup.subject, "Daily standup");
        assert_eq!(standup.start_ms, day + 7 * MS_PER_HOUR + 30 * 60_000);
        assert_eq!(standup.end_ms - standup.start_ms, 15 * 60_000);
        assert!(standup.online && standup.busy && !standup.all_day);
        assert!(!page.events[1].busy);
        let all_day = &page.events[2];
        assert!(all_day.all_day);
        assert_eq!(
            all_day.start_ms,
            day - 2 * MS_PER_HOUR,
            "local midnight at UTC+2"
        );
        assert!(page.events[3].online);

        let meetings = meetings_only(page.events, day, day + MS_PER_DAY);
        let ids: Vec<&str> = meetings.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["standup_20261013T073000Z", "zoom"]);
        assert!(parse_events_page("<html>", &local).is_err());
        assert!(
            parse_events_page("{}", &local)
                .expect("empty")
                .events
                .is_empty()
        );
    }

    #[test]
    fn debug_output_redacts_secrets() {
        let mut s = settings("id");
        s.client_secret = "GOCSPX-secret".into();
        assert!(!format!("{s:?}").contains("GOCSPX-secret"));
        let token = StoredToken {
            refresh_token: "1//secret-refresh".into(),
            account: None,
        };
        assert!(!format!("{token:?}").contains("secret-refresh"));
    }

    #[test]
    fn client_without_configuration_or_token() {
        let mut client = GoogleClient::new(settings("  "));
        assert!(!client.is_configured());
        assert!(client.begin_login().is_err());
        assert!(matches!(
            client.events(0, 1000),
            Err(PlatformError::Auth(_))
        ));
        assert!(client.events(1000, 0).expect("empty range").is_empty());

        let mut client = GoogleClient::new(settings("123-abc.apps.googleusercontent.com"));
        assert!(client.is_configured());
        assert!(!client.has_token());
        assert_eq!(client.account(), None);
        // No stored token: fails before touching the network.
        assert!(matches!(
            client.events(0, 1000),
            Err(PlatformError::Auth(_)) | Err(PlatformError::Unsupported)
        ));
        assert!(client.disconnect().is_ok());
    }

    fn browser_request(port: u16, target: &str) -> TcpStream {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connect");
        write!(
            stream,
            "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUser-Agent: test\r\n\r\n"
        )
        .expect("send");
        stream
    }

    fn read_response(mut stream: TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout");
        let mut text = String::new();
        let _ = stream.read_to_string(&mut text);
        text
    }

    #[test]
    fn loopback_redirect_flow() {
        let mut client = GoogleClient::new(settings("123-abc.apps.googleusercontent.com"));
        let login = client.start_login().expect("login");
        let port = login.port().expect("port");
        assert!(login.auth_url.contains(&format!("127.0.0.1%3A{port}")));
        assert!(!format!("{login:?}").contains(&login.verifier));
        let short = Duration::from_millis(100);

        // Nothing yet.
        assert_eq!(
            client.poll_login(&login, short).expect("poll"),
            GooglePoll::Pending
        );

        // A favicon request and a forged redirect are answered and ignored.
        let favicon = browser_request(port, "/favicon.ico");
        assert_eq!(
            client.poll_login(&login, short).expect("poll"),
            GooglePoll::Pending
        );
        assert!(read_response(favicon).starts_with("HTTP/1.1 404"));
        let forged = browser_request(port, "/?code=stolen&state=wrong");
        assert_eq!(
            client.poll_login(&login, short).expect("poll"),
            GooglePoll::Pending
        );
        let page = read_response(forged);
        assert!(page.starts_with("HTTP/1.1 400"), "{page}");

        // The user cancels on Google's consent screen.
        let cancelled = browser_request(
            port,
            &format!("/?error=access_denied&state={}", login.state),
        );
        assert_eq!(
            client
                .poll_login(&login, Duration::from_secs(5))
                .expect("poll"),
            GooglePoll::Failed("sign-in was declined".into())
        );
        let page = read_response(cancelled);
        assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
        assert!(page.contains("Możesz zamknąć tę kartę"), "{page}");
        assert!(page.contains("charset=utf-8"));
        assert!(!client.has_token());
    }

    #[test]
    fn expired_login_fails() {
        let mut client = GoogleClient::new(settings("123-abc.apps.googleusercontent.com"));
        let mut login = client.start_login().expect("login");
        login.expires_in_secs = 0;
        assert!(login.is_expired());
        assert_eq!(
            client
                .poll_login(&login, Duration::from_millis(10))
                .expect("poll"),
            GooglePoll::Failed("the sign-in timed out".into())
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn token_store_round_trip() {
        let s = settings("123-abc.apps.googleusercontent.com");
        let token = StoredToken {
            refresh_token: "1//rt".into(),
            account: Some("ala@example.com".into()),
        };
        save_token(&s.token_path, &token).expect("save");
        let client = GoogleClient::new(s.clone());
        assert!(client.has_token());
        assert_eq!(client.account().as_deref(), Some("ala@example.com"));
        assert_eq!(load_token(&s.token_path).expect("load"), Some(token));
        secrets::delete_sealed(&s.token_path).expect("delete");
        assert!(!client.has_token());
    }
}
