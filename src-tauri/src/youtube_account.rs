//! YouTube sign-in for Premium (ADR-021, ARCHITECTURE §6.13).
//!
//! YouTube Premium (no ads) applies only where the youtube.com cookies are first-party: in the hidden `media`
//! window, which loads youtube.com top-level. The embedded player in the main window runs on `tauri.localhost`,
//! where the same cookies are third-party. So a signed-in user gets every YouTube item in the `media` window (the
//! player decides that from `YoutubeAccount`); a signed-out user keeps today's routing.
//!
//! Signing in happens on Google's own page in a visible `youtube-login` window that shares the WebView2 profile
//! (and so the cookies) with `main` and `media`. That window gets no initialization script and no `eval`: the app
//! never sees what the user types. The shell only checks which cookie *names* exist on youtube.com
//! ([`SIGN_IN_COOKIES`]); cookie values are never kept, compared or logged, and neither are account names or
//! URLs with their parameters (hosts only).
//!
//! Sign-out deletes the google.com / youtube.com cookies of the profile except the cookie choice ([`KEEP_COOKIES`]),
//! so YouTube does not ask about cookies again. Every change goes out as `youtube://account` with a revision
//! (ADR-012); a change of the sign-in state also reloads an open `media` window, so its page picks up the new cookies.
//!
//! WebView2 cookie calls block until the main thread answers (and deadlock on the main thread itself), so every
//! read and delete runs on a blocking worker.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{AppError, YoutubeAccount, events};
use tauri::webview::{Cookie, NewWindowResponse};
use tauri::{
    AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry,
};

use crate::window_util::{open_link_externally, with_main_webview_options};
use crate::{AppState, MAIN_WINDOW, lock, texts};

/// Label of the sign-in window.
pub const SIGN_IN_WINDOW: &str = "youtube-login";
/// Google's sign-in page for YouTube; Google forwards it to the current form and back to YouTube afterwards.
const SIGN_IN_URL: &str = "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2F";
const WIDTH: f64 = 480.0;
const HEIGHT: f64 = 640.0;
const MIN_WIDTH: f64 = 380.0;
const MIN_HEIGHT: f64 = 480.0;
/// How often the open sign-in window is checked (cookie names, page address).
const SIGN_IN_POLL: Duration = Duration::from_millis(1500);
/// Stops checking after this long; the window stays, and closing it checks the cookies once more.
const SIGN_IN_WAIT: Duration = Duration::from_secs(30 * 60);
/// Signed in but the page is still on its way back to YouTube: closes the window anyway after this long.
const LAND_WAIT: Duration = Duration::from_secs(8);
/// Signed in and back on YouTube: the window closes this much later (the page finishes storing its cookies).
const CLOSE_DELAY: Duration = Duration::from_secs(1);
/// After a sign-out (or a cleared browser profile) WebView2 deletes cookies asynchronously: a sign-in cookie that
/// is still there is read again this many times, [`SIGNED_OUT_RECHECK`] apart, before it counts.
const SIGNED_OUT_CHECKS: u32 = 15;
const SIGNED_OUT_RECHECK: Duration = Duration::from_millis(400);
/// The startup check is tried this many times, [`STARTUP_RECHECK`] apart, while the cookies cannot be read.
const STARTUP_CHECKS: u32 = 5;
const STARTUP_RECHECK: Duration = Duration::from_secs(3);
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com/";
/// Any of these cookie names on youtube.com means "signed in". Only the names are ever compared.
const SIGN_IN_COOKIES: [&str; 3] = ["LOGIN_INFO", "SAPISID", "__Secure-3PAPISID"];
/// The youtube.com page sets this one itself once the sign-in comes back to YouTube.
const YOUTUBE_LOGIN_COOKIE: &str = "LOGIN_INFO";
/// Google's cookie choice: kept on sign-out, so YouTube plays without asking about cookies again.
const KEEP_COOKIES: [&str; 2] = ["SOCS", "CONSENT"];
/// Google hosts of the sign-in flow the window may open itself (plus youtube.com and `accounts.google.<country>`).
/// `gds.google.com` shows the "make sure you can always sign in" step; `www.google.com` the "unusual traffic" check.
const SIGN_IN_HOSTS: [&str; 5] = [
    "accounts.google.com",
    "consent.google.com",
    "myaccount.google.com",
    "gds.google.com",
    "www.google.com",
];
/// YouTube pages the sign-in comes back to.
const YOUTUBE_PAGE_HOSTS: [&str; 4] = [
    "www.youtube.com",
    "youtube.com",
    "m.youtube.com",
    "music.youtube.com",
];

const ERR_SIGN_IN: &str = "youtube.signIn";
const ERR_SIGN_OUT: &str = "youtube.signOut";

/// Managed handle to the YouTube account state (cheap to clone).
#[derive(Clone, Default)]
pub struct YoutubeAccountHandle {
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    state: Mutex<Inner>,
    /// Serializes sign-in and sign-out.
    ops: tokio::sync::Mutex<()>,
    /// Last link handed to the system browser (rate limit).
    last_external: Mutex<Option<Instant>>,
}

#[derive(Default)]
struct Inner {
    revision: u32,
    known: bool,
    signed_in: bool,
    sign_in_open: bool,
    browser_rejected: bool,
    /// Bumped for every created sign-in window, so late events and checks of a closed one are ignored.
    instance: u64,
    /// The app closed the current sign-in window itself (signed in, sign-out, cleared browser data): its `Destroyed`
    /// needs no cookie check of its own, which could otherwise read the cookies before a running deletion is applied
    /// and publish "signed in" after the deleting caller's check.
    closed_by_app: bool,
}

impl Inner {
    fn status(&self) -> YoutubeAccount {
        YoutubeAccount {
            revision: self.revision,
            known: self.known,
            signed_in: self.signed_in,
            sign_in_open: self.sign_in_open,
            browser_rejected: self.browser_rejected,
        }
    }
}

/// What one look at the sign-in window found.
struct Probe {
    /// Some sign-in cookie exists (`None`: the cookies could not be read).
    signed_in: Option<bool>,
    /// The flow is back on YouTube: the page is a youtube.com page or YouTube stored its own login cookie.
    landed: bool,
    /// Google refused this browser ("This browser or app may not be secure").
    rejected: bool,
}

impl YoutubeAccountHandle {
    pub fn status(&self) -> YoutubeAccount {
        lock(&self.shared.state).status()
    }

    /// Startup: reads the sign-in cookies once in the background (retried a few times while the main webview cannot
    /// answer yet). Until then `known` stays false and the player routes YouTube as when signed out.
    pub fn start(&self, app: &AppHandle) {
        let this = self.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            for attempt in 0..STARTUP_CHECKS {
                if attempt > 0 {
                    tokio::time::sleep(STARTUP_RECHECK).await;
                }
                if this.check(&app, false).await.is_some() {
                    return;
                }
            }
            log::warn!(
                "YouTube: cannot read the sign-in cookies, YouTube plays as when signed out"
            );
        });
    }

    /// Reads the sign-in cookies in the background and publishes the result (after the privacy panel cleared the
    /// browser data, ADR-020). `expect_signed_out`: the caller has just deleted the cookies, which WebView2 applies
    /// asynchronously, so a sign-in cookie that is still there is read again a few times first.
    pub fn refresh(&self, app: &AppHandle, expect_signed_out: bool) {
        let this = self.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            this.check(&app, expect_signed_out).await;
        });
    }

    /// Closes the sign-in window if it is open (the privacy panel does this before clearing the browser data).
    pub fn close_sign_in_window(&self, app: &AppHandle) {
        if let Some(window) = app.get_webview_window(SIGN_IN_WINDOW) {
            self.destroy_sign_in_window(&window);
        }
    }

    /// Closes the sign-in window on the app's own initiative (see [`Inner::closed_by_app`]).
    fn destroy_sign_in_window(&self, window: &WebviewWindow) {
        lock(&self.shared.state).closed_by_app = true;
        if let Err(err) = window.destroy() {
            lock(&self.shared.state).closed_by_app = false;
            log::warn!("cannot close the YouTube sign-in window: {err}");
        }
    }

    /// Opens the sign-in window on Google's page, or brings the open one to the front (after a refusal by Google it
    /// starts over on the sign-in page). The window closes by itself once the user is signed in.
    pub async fn sign_in(&self, app: &AppHandle) -> Result<(), AppError> {
        let _serial = self.shared.ops.lock().await;
        let url = Url::parse(SIGN_IN_URL)
            .map_err(|err| AppError::new(ERR_SIGN_IN, format!("bad sign-in address: {err}")))?;

        if let Some(window) = app.get_webview_window(SIGN_IN_WINDOW) {
            if lock(&self.shared.state).browser_rejected {
                log::info!("YouTube: starting the sign-in again after Google's refusal");
                if let Err(err) = window.navigate(url) {
                    log::warn!("cannot reload the YouTube sign-in page: {err}");
                }
                self.update(app, |inner| {
                    let changed = inner.browser_rejected;
                    inner.browser_rejected = false;
                    changed
                });
            }
            bring_to_front(&window);
            return Ok(());
        }

        // Published as open before the window exists, so its `Destroyed` can never come before this.
        let mut instance = 0;
        self.update(app, |inner| {
            inner.instance = inner.instance.wrapping_add(1);
            instance = inner.instance;
            inner.sign_in_open = true;
            inner.browser_rejected = false;
            inner.closed_by_app = false;
            true
        });
        log::info!("YouTube: opening the sign-in window");
        let window = match self.builder(app, url, instance).build() {
            Ok(window) => window,
            Err(err) => {
                self.update(app, |inner| {
                    if inner.instance != instance || !inner.sign_in_open {
                        return false;
                    }
                    inner.sign_in_open = false;
                    true
                });
                return Err(AppError::new(
                    ERR_SIGN_IN,
                    format!("cannot create the sign-in window: {err}"),
                ));
            }
        };
        let this = self.clone();
        let handle = app.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                this.closed(&handle, instance);
            }
        });
        self.watch_sign_in(app, instance);
        Ok(())
    }

    /// Deletes the google.com / youtube.com cookies of the shared profile except the cookie choice, then checks
    /// that the sign-in cookies are gone. An open sign-in window is closed first.
    pub async fn sign_out(&self, app: &AppHandle) -> Result<(), AppError> {
        let _serial = self.shared.ops.lock().await;
        self.close_sign_in_window(app);
        let main = app
            .get_webview_window(MAIN_WINDOW)
            .ok_or_else(|| AppError::new(ERR_SIGN_OUT, "the main window is gone"))?;
        let deleted = tauri::async_runtime::spawn_blocking(move || delete_account_cookies(&main))
            .await
            .map_err(|err| AppError::new(ERR_SIGN_OUT, format!("sign-out failed: {err}")))??;
        log::info!(
            "YouTube: signing out, {deleted} Google / YouTube cookies deleted (cookie choice kept)"
        );
        match self.check(app, true).await {
            Some(false) => Ok(()),
            Some(true) => Err(AppError::new(
                ERR_SIGN_OUT,
                "the sign-in cookies are still there",
            )),
            None => Err(AppError::new(
                ERR_SIGN_OUT,
                "cannot read the cookies after signing out",
            )),
        }
    }

    fn builder<'a>(
        &self,
        app: &'a AppHandle,
        url: Url,
        instance: u64,
    ) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
        // No initialization script and no `eval`: the page is Google's alone (rule 11).
        let builder = WebviewWindowBuilder::new(app, SIGN_IN_WINDOW, WebviewUrl::External(url))
            .title(texts::current().youtube_sign_in_title)
            .inner_size(WIDTH, HEIGHT)
            .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
            .resizable(true)
            .center()
            .visible(true)
            .focused(true);
        // Same WebView2 environment as `main` and `media`: one profile, so the cookies are shared.
        let builder = with_main_webview_options(app, builder);

        let nav_this = self.clone();
        let nav_app = app.clone();
        let popup_this = self.clone();
        let popup_app = app.clone();
        builder
            .on_navigation(move |url| {
                if is_rejected_page(url) {
                    nav_this.rejected(&nav_app, instance);
                }
                if is_allowed_navigation(url) {
                    return true;
                }
                open_link_externally(
                    &nav_app,
                    &nav_this.shared.last_external,
                    url,
                    "sign-in window",
                );
                false
            })
            .on_new_window(move |url, _features| {
                open_link_externally(
                    &popup_app,
                    &popup_this.shared.last_external,
                    &url,
                    "sign-in window",
                );
                NewWindowResponse::Deny
            })
    }

    /// Checks the open sign-in window until the user is signed in (then closes it) or it goes away.
    fn watch_sign_in(&self, app: &AppHandle, instance: u64) {
        let this = self.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let deadline = Instant::now() + SIGN_IN_WAIT;
            let mut signed_in_since: Option<Instant> = None;
            while Instant::now() < deadline {
                tokio::time::sleep(SIGN_IN_POLL).await;
                if lock(&this.shared.state).instance != instance {
                    return;
                }
                let Some(window) = app.get_webview_window(SIGN_IN_WINDOW) else {
                    return;
                };
                let Ok(probe) =
                    tauri::async_runtime::spawn_blocking(move || probe_window(&window)).await
                else {
                    continue;
                };
                if probe.rejected {
                    this.rejected(&app, instance);
                }
                if probe.signed_in != Some(true) {
                    signed_in_since = None;
                    continue;
                }
                let since = *signed_in_since.get_or_insert_with(Instant::now);
                if !probe.landed && since.elapsed() < LAND_WAIT {
                    continue;
                }
                // Published only once the hand-over to YouTube is done, so the reload of an open YouTube window
                // (`set_signed_in`) already gets every sign-in cookie.
                tokio::time::sleep(CLOSE_DELAY).await;
                this.set_signed_in(&app, true);
                if lock(&this.shared.state).instance == instance
                    && let Some(window) = app.get_webview_window(SIGN_IN_WINDOW)
                {
                    log::info!("YouTube: signed in, closing the sign-in window");
                    this.destroy_sign_in_window(&window);
                }
                return;
            }
            log::info!("YouTube: no sign-in yet, the sign-in window stays open");
        });
    }

    /// Reads the sign-in cookies through the main window and publishes the result; `None` when they cannot be read.
    async fn check(&self, app: &AppHandle, expect_signed_out: bool) -> Option<bool> {
        let attempts = if expect_signed_out {
            SIGNED_OUT_CHECKS
        } else {
            1
        };
        let mut found = None;
        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(SIGNED_OUT_RECHECK).await;
            }
            let main = app.get_webview_window(MAIN_WINDOW)?;
            found = tauri::async_runtime::spawn_blocking(move || read_signed_in(&main))
                .await
                .ok()
                .flatten();
            if found != Some(true) {
                break;
            }
        }
        if let Some(signed_in) = found {
            self.set_signed_in(app, signed_in);
        }
        found
    }

    /// Publishes the sign-in state. A change after the first check reloads an open YouTube window, so its page
    /// plays with the new cookies (Premium right after signing in).
    fn set_signed_in(&self, app: &AppHandle, signed_in: bool) {
        let mut flipped = false;
        let mut first = false;
        self.update(app, |inner| {
            if inner.known && inner.signed_in == signed_in {
                return false;
            }
            flipped = inner.known;
            first = !inner.known;
            inner.known = true;
            inner.signed_in = signed_in;
            true
        });
        if first {
            log::info!(
                "YouTube: {}",
                if signed_in {
                    "signed in (YouTube plays in the YouTube window)"
                } else {
                    "not signed in"
                }
            );
        }
        if flipped {
            log::info!(
                "YouTube: {}",
                if signed_in { "signed in" } else { "signed out" }
            );
            if let Some(state) = app.try_state::<AppState>() {
                state.media.reload(app);
            }
        }
    }

    /// Google refused the embedded browser; the window stays so its explanation remains readable.
    fn rejected(&self, app: &AppHandle, instance: u64) {
        let mut newly = false;
        self.update(app, |inner| {
            if inner.instance != instance || inner.browser_rejected {
                return false;
            }
            inner.browser_rejected = true;
            newly = true;
            true
        });
        if newly {
            log::info!("YouTube: Google refused the sign-in in the app's browser");
        }
    }

    fn closed(&self, app: &AppHandle, instance: u64) {
        let mut closed_by_user = None;
        self.update(app, |inner| {
            if inner.instance != instance || !inner.sign_in_open {
                return false;
            }
            inner.sign_in_open = false;
            closed_by_user = Some(!inner.closed_by_app);
            inner.closed_by_app = false;
            true
        });
        if closed_by_user == Some(true) {
            log::info!("YouTube: sign-in window closed");
            // The user may have finished signing in just before closing it.
            self.refresh(app, false);
        }
    }

    /// Applies `change`; when it reports a change, bumps the revision and emits `youtube://account`.
    fn update(&self, app: &AppHandle, change: impl FnOnce(&mut Inner) -> bool) {
        let status = {
            let mut inner = lock(&self.shared.state);
            if !change(&mut inner) {
                return;
            }
            inner.revision = inner.revision.wrapping_add(1);
            inner.status()
        };
        if let Err(err) = app.emit(events::YOUTUBE_ACCOUNT, status) {
            log::warn!("cannot emit the YouTube account state: {err}");
        }
    }
}

fn bring_to_front(window: &WebviewWindow) {
    if let Err(err) = window.unminimize() {
        log::debug!("sign-in window unminimize failed: {err}");
    }
    if let Err(err) = window.show() {
        log::debug!("sign-in window show failed: {err}");
    }
    if let Err(err) = window.set_focus() {
        log::debug!("sign-in window set_focus failed: {err}");
    }
}

/// Blocking: one look at the sign-in window (page address and youtube.com cookie names).
fn probe_window(window: &WebviewWindow) -> Probe {
    let page = window.url().ok();
    let names = youtube_cookie_names(window);
    Probe {
        signed_in: names.as_ref().map(|names| has_sign_in_cookie(names)),
        landed: page.as_ref().is_some_and(is_youtube_page)
            || names
                .as_ref()
                .is_some_and(|names| names.iter().any(|name| name == YOUTUBE_LOGIN_COOKIE)),
        rejected: page.as_ref().is_some_and(is_rejected_page),
    }
}

/// Blocking: whether the shared profile holds a youtube.com sign-in cookie (`None`: cannot read the cookies).
fn read_signed_in(webview: &WebviewWindow) -> Option<bool> {
    youtube_cookie_names(webview).map(|names| has_sign_in_cookie(&names))
}

/// Blocking: names of the cookies WebView2 would send to youtube.com. The values are dropped right here.
fn youtube_cookie_names(webview: &WebviewWindow) -> Option<Vec<String>> {
    let origin = Url::parse(YOUTUBE_ORIGIN).ok()?;
    match webview.cookies_for_url(origin) {
        Ok(cookies) => Some(
            cookies
                .iter()
                .map(|cookie| cookie.name().to_owned())
                .collect(),
        ),
        Err(err) => {
            log::warn!("cannot read the YouTube cookies: {err}");
            None
        }
    }
}

fn has_sign_in_cookie(names: &[String]) -> bool {
    names
        .iter()
        .any(|name| SIGN_IN_COOKIES.contains(&name.as_str()))
}

/// Blocking: deletes every google.com / youtube.com / google.<country> cookie except the cookie choice.
/// Returns how many cookies were found to delete.
fn delete_account_cookies(webview: &WebviewWindow) -> Result<usize, AppError> {
    let cookies = webview
        .cookies()
        .map_err(|err| AppError::new(ERR_SIGN_OUT, format!("cannot read the cookies: {err}")))?;
    let mut deleted = 0;
    for cookie in cookies {
        if KEEP_COOKIES.contains(&cookie.name())
            || !cookie.domain().is_some_and(is_account_cookie_domain)
        {
            continue;
        }
        for variant in deletion_variants(&cookie) {
            if let Err(err) = webview.delete_cookie(variant) {
                log::debug!("cannot delete a YouTube cookie: {err}");
            }
        }
        deleted += 1;
    }
    Ok(deleted)
}

/// WebView2 deletes a cookie by name, path and *exact* domain, and tells a domain cookie (`.youtube.com`) from a
/// host-only one (`youtube.com`). The `cookie` crate (0.18) hides that leading dot: `domain()` strips it, and that
/// is what Tauri hands to WebView2 when deleting. So the deletion goes out for both forms (the one that does not
/// exist is a no-op): the host-only form as read, and the domain form spelled `..domain`, which `domain()` turns
/// into `.domain` (pinned by a test below). `__Host-` cookies are host-only by definition.
fn deletion_variants(cookie: &Cookie<'static>) -> Vec<Cookie<'static>> {
    let mut variants = vec![cookie.clone()];
    if !cookie.name().starts_with("__Host-")
        && let Some(domain) = cookie.domain()
    {
        let mut dotted = cookie.clone();
        dotted.set_domain(format!("..{}", domain.trim_start_matches('.')));
        variants.push(dotted);
    }
    variants
}

/// Top-level navigations the sign-in window may perform itself: https on Google's sign-in hosts and YouTube.
fn is_allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "about" => url.as_str() == "about:blank",
        "https" => url.host_str().is_some_and(|host| {
            SIGN_IN_HOSTS.contains(&host)
                || in_domain(host, "youtube.com")
                || is_google_country_accounts_host(host)
        }),
        _ => false,
    }
}

/// Google's "This browser or app may not be secure" page (`/signin/rejected`, also under `/v3/`).
fn is_rejected_page(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| {
            host == "accounts.google.com" || is_google_country_accounts_host(host)
        })
        && url.path().contains("/signin/rejected")
}

/// A YouTube page outside the sign-in hand-over (`/signin?action_handle_signin=…`).
fn is_youtube_page(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| YOUTUBE_PAGE_HOSTS.contains(&host))
        && !url.path().starts_with("/signin")
}

/// `accounts.google.<country>` (`accounts.google.pl`, `accounts.google.co.uk`): Google can pass a sign-in
/// through them.
pub(crate) fn is_google_country_accounts_host(host: &str) -> bool {
    host.strip_prefix("accounts.")
        .is_some_and(is_google_country_domain)
}

/// `google.<country>`: `google.pl`, `google.co.uk`, `google.com.au` (not `google.com` itself).
fn is_google_country_domain(domain: &str) -> bool {
    let Some(rest) = domain.strip_prefix("google.") else {
        return false;
    };
    let country = rest
        .strip_prefix("co.")
        .or_else(|| rest.strip_prefix("com."))
        .unwrap_or(rest);
    country.len() == 2 && country.bytes().all(|b| b.is_ascii_lowercase())
}

/// Cookie domains whose cookies belong to the Google / YouTube sign-in: youtube.com, google.com and
/// google.<country>, with their subdomains (a leading dot is ignored).
fn is_account_cookie_domain(domain: &str) -> bool {
    let domain = domain.trim_start_matches('.').to_ascii_lowercase();
    if in_domain(&domain, "youtube.com") || in_domain(&domain, "google.com") {
        return true;
    }
    std::iter::once(domain.as_str())
        .chain(
            domain
                .match_indices('.')
                .filter_map(|(at, _)| domain.get(at + 1..)),
        )
        .any(is_google_country_domain)
}

/// `host` is `domain` or one of its subdomains.
fn in_domain(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|sub| sub.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(raw: &str) -> Url {
        Url::parse(raw).expect("test url")
    }

    #[test]
    fn sign_in_window_navigation_guard() {
        for ok in [
            SIGN_IN_URL,
            "https://accounts.google.com/v3/signin/identifier?continue=x",
            "https://accounts.google.com/v3/signin/challenge/pwd",
            "https://accounts.google.pl/accounts/SetSID",
            "https://accounts.google.co.uk/accounts/SetSID",
            "https://accounts.google.com.au/accounts/SetSID",
            "https://accounts.youtube.com/accounts/SetSID",
            "https://www.youtube.com/signin?action_handle_signin=true",
            "https://www.youtube.com/",
            "https://consent.youtube.com/m?continue=x",
            "https://consent.google.com/ml?continue=x",
            "https://myaccount.google.com/",
            "https://gds.google.com/web/recoveryoptions",
            "https://www.google.com/sorry/index",
            "about:blank",
        ] {
            assert!(is_allowed_navigation(&url(ok)), "{ok}");
        }
        for bad in [
            "http://accounts.google.com/",
            "https://accounts.google.com.evil.com/",
            "https://accounts.google.evil/",
            "https://accounts.google.community/",
            "https://evilgoogle.com/",
            "https://youtube.com.evil.com/",
            "https://sites.google.com/view/x",
            "https://docs.google.com/forms/d/x",
            "https://support.google.com/accounts",
            "https://policies.google.com/privacy",
            "https://google.com/",
            "https://example.com/",
            "about:srcdoc",
            "data:text/html,hi",
            "javascript:alert(1)",
        ] {
            assert!(!is_allowed_navigation(&url(bad)), "{bad}");
        }
    }

    #[test]
    fn rejected_page_and_landing() {
        assert!(is_rejected_page(&url(
            "https://accounts.google.com/v3/signin/rejected?continue=x&dsh=1"
        )));
        assert!(is_rejected_page(&url(
            "https://accounts.google.com/signin/rejected?rrk=46"
        )));
        assert!(is_rejected_page(&url(
            "https://accounts.google.de/signin/rejected"
        )));
        assert!(!is_rejected_page(&url(
            "https://accounts.google.com/v3/signin/identifier"
        )));
        assert!(!is_rejected_page(&url(
            "https://example.com/signin/rejected"
        )));

        assert!(is_youtube_page(&url("https://www.youtube.com/")));
        assert!(is_youtube_page(&url("https://m.youtube.com/feed")));
        assert!(!is_youtube_page(&url(
            "https://www.youtube.com/signin?action_handle_signin=true"
        )));
        assert!(!is_youtube_page(&url(
            "https://accounts.youtube.com/accounts/SetSID"
        )));
        assert!(!is_youtube_page(&url(
            "https://consent.youtube.com/m?continue=x"
        )));
    }

    #[test]
    fn sign_in_cookie_names_only() {
        let names = |list: &[&str]| list.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
        assert!(!has_sign_in_cookie(&names(&[
            "YSC",
            "VISITOR_INFO1_LIVE",
            "SOCS",
            "PREF"
        ])));
        assert!(has_sign_in_cookie(&names(&["YSC", "SAPISID"])));
        assert!(has_sign_in_cookie(&names(&["__Secure-3PAPISID"])));
        assert!(has_sign_in_cookie(&names(&["LOGIN_INFO"])));
        assert!(!has_sign_in_cookie(&names(&["sapisid", "LOGIN_INFO_X"])));
    }

    #[test]
    fn account_cookie_domains() {
        for ok in [
            ".youtube.com",
            "youtube.com",
            "www.youtube.com",
            "accounts.youtube.com",
            ".google.com",
            "accounts.google.com",
            ".google.pl",
            "accounts.google.co.uk",
            ".google.com.au",
            ".GOOGLE.COM",
        ] {
            assert!(is_account_cookie_domain(ok), "{ok}");
        }
        for bad in [
            ".youtube-nocookie.com",
            "notyoutube.com",
            ".googlevideo.com",
            ".doubleclick.net",
            "google.community",
            "evilgoogle.com",
            "google.pl.evil.com",
            "localhost",
            "",
        ] {
            assert!(!is_account_cookie_domain(bad), "{bad}");
        }
    }

    #[test]
    fn deletion_covers_domain_and_host_cookies() {
        // Pins the `cookie` crate behaviour the deletion relies on (see `deletion_variants`).
        let domain_cookie = Cookie::build(("SAPISID", "x"))
            .domain(".youtube.com")
            .path("/")
            .secure(true)
            .build();
        assert_eq!(domain_cookie.domain(), Some("youtube.com"));
        let domains: Vec<_> = deletion_variants(&domain_cookie)
            .iter()
            .map(|cookie| cookie.domain().map(str::to_owned))
            .collect();
        assert_eq!(
            domains,
            [
                Some("youtube.com".to_owned()),
                Some(".youtube.com".to_owned())
            ]
        );
        let variants = deletion_variants(&domain_cookie);
        assert!(variants.iter().all(|cookie| cookie.name() == "SAPISID"
            && cookie.path() == Some("/")
            && cookie.secure() == Some(true)));

        let host_cookie = Cookie::build(("__Host-GAPS", "x"))
            .domain("accounts.google.com")
            .path("/")
            .build();
        let domains: Vec<_> = deletion_variants(&host_cookie)
            .iter()
            .map(|cookie| cookie.domain().map(str::to_owned))
            .collect();
        assert_eq!(domains, [Some("accounts.google.com".to_owned())]);
    }

    #[test]
    fn country_domains() {
        for ok in ["google.pl", "google.de", "google.co.uk", "google.com.au"] {
            assert!(is_google_country_domain(ok), "{ok}");
        }
        for bad in [
            "google.com",
            "google.community",
            "google.pl.evil.com",
            "google.c0",
            "google.",
            "notgoogle.pl",
        ] {
            assert!(!is_google_country_domain(bad), "{bad}");
        }
        assert!(is_google_country_accounts_host("accounts.google.pl"));
        assert!(!is_google_country_accounts_host("mail.google.pl"));
        assert!(!is_google_country_accounts_host("accounts.google.com"));
    }

    #[test]
    fn kept_cookies_are_the_cookie_choice() {
        assert_eq!(KEEP_COOKIES, ["SOCS", "CONSENT"]);
        for name in SIGN_IN_COOKIES {
            assert!(!KEEP_COOKIES.contains(&name));
        }
    }
}
