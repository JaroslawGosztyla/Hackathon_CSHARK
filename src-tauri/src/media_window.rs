//! Separate YouTube window ("media") for links that refuse embedding (YouTube errors 101/150/153,
//! e.g. the Lofi Girl live stream) or cannot be embedded at all (`/@handle/live`, `/channel/<id>/live`).
//!
//! The link is loaded top-level, like a normal browser tab, in the default WebView2 profile (the
//! YouTube consent cookie persists). The remote page has no IPC permissions (the default capability
//! covers only `main`, and Tauri never lets remote origins reach app commands without a `remote`
//! capability), so the app drives it one way only: fire-and-forget `eval` of fixed scripts that talk
//! to a tiny controller injected by an initialization script. Nothing user-controlled is ever
//! spliced into a script: only the fixed literals `'play'` / `'pause'` and an integer volume.
//!
//! Navigation is limited to YouTube / Google sign-in and consent hosts; any other link (and every
//! popup) opens in the system browser instead.
//!
//! The window is a hidden background player: created invisible, X only hides it, and the app
//! destroys it when the music source changes. Until the YouTube cookie choice is stored (`SOCS`),
//! YouTube plays nothing, so on that first run the window is shown and hides itself after the choice.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{AppError, MediaAction, MediaWindowStatus, events};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::{MAIN_WINDOW, lock, texts};

/// Label of the YouTube window.
pub const MEDIA_WINDOW: &str = "media";
const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 380.0;
const MIN_WIDTH: f64 = 320.0;
const MIN_HEIGHT: f64 = 200.0;
/// Gap between the window and the work area edge (logical px).
const SCREEN_MARGIN: f64 = 16.0;
/// Title bar and borders on top of the inner size (logical px, generous).
const FRAME_ALLOWANCE: f64 = 40.0;
/// Volume used until the UI sends one (0..=100).
const DEFAULT_VOLUME: u8 = 100;
/// At most one link opened in the system browser per this interval (a page cannot spam tabs).
const EXTERNAL_OPEN_EVERY: Duration = Duration::from_secs(2);
/// `open` right after our own `close` waits this long for the old window to be gone (the label is
/// only free after `Destroyed`), polling every [`CLOSE_POLL`].
const CLOSE_WAIT: Duration = Duration::from_millis(1500);
const CLOSE_POLL: Duration = Duration::from_millis(20);
/// How often the first-run window checks whether the YouTube cookie choice was made.
const CONSENT_POLL: Duration = Duration::from_millis(1500);
/// Gives up waiting for the cookie choice (the window then simply stays where the user put it).
const CONSENT_WAIT: Duration = Duration::from_secs(15 * 60);
/// After the choice YouTube reloads the page; hiding a moment later lets playback start while visible.
const CONSENT_HIDE_DELAY: Duration = Duration::from_millis(2500);
/// Google consent cookie. It exists from the first visit; the choice is in its value (see [`consent_decided`]).
const CONSENT_COOKIE: &str = "SOCS";
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com/";

/// Hosts accepted by `open_media_window`.
const OPEN_HOSTS: [&str; 5] = [
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtu.be",
];
/// The window may navigate to these domains and their subdomains (YouTube, media CDNs).
const NAVIGATION_DOMAINS: [&str; 4] =
    ["youtube.com", "gstatic.com", "googlevideo.com", "ytimg.com"];
/// Google hosts the consent and sign-in flows use. Not all of `google.com`: other subdomains
/// (Sites, Docs, Forms, policies, support) carry arbitrary content and belong in the system browser.
const GOOGLE_FLOW_HOSTS: [&str; 4] = [
    "accounts.google.com",
    "consent.google.com",
    "myaccount.google.com",
    "www.google.com",
];

/// Injected into every top-level document of the media window (before the page scripts run); the
/// player controller is installed on YouTube hosts only. `__WANT__` and `__VOLUME__` are replaced
/// with fixed literals (see [`init_script`]).
///
/// `window.__focusmodeMedia.apply()` applies the desired state now and keeps re-applying it for 20 s
/// (the player loads late and autoplay may start on its own); every SPA navigation re-arms that window.
/// A trusted click or key press in the page hands control to the user until the next `apply()`.
const INIT_SCRIPT: &str = r#"(function () {
  'use strict';
  if (window.top !== window.self || window.__focusmodeMedia) return;
  // Tauri's opener plugin turns new-tab link clicks into an IPC call that this remote page is not
  // allowed to make, which would swallow the click. Capture them first and request a new window
  // instead: the shell denies it and opens the link in the system browser.
  window.addEventListener('click', function (event) {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.altKey) return;
    var path = typeof event.composedPath === 'function' ? event.composedPath() : [];
    var link = null;
    for (var i = 0; i < path.length; i++) {
      if (path[i] && typeof path[i].nodeName === 'string' && path[i].nodeName.toUpperCase() === 'A') {
        link = path[i];
        break;
      }
    }
    if (!link || typeof link.href !== 'string' || !/^https?:/i.test(link.href)) return;
    if (link.target !== '_blank' && !event.ctrlKey && !event.shiftKey) return;
    event.preventDefault();
    window.open(link.href, '_blank', 'noopener');
  }, true);
  var host = location.hostname;
  if (host !== 'youtu.be' && host !== 'youtube.com' && !/\.youtube\.com$/.test(host)) return;
  var ENFORCE_MS = 20000;
  var media = {
    want: __WANT__,
    volume: __VOLUME__,
    until: Date.now() + ENFORCE_MS,
    userTouched: false,
    href: location.href,
    apply: function () {
      media.userTouched = false;
      media.until = Date.now() + ENFORCE_MS;
      enforce();
    },
    applyVolume: function () {
      try {
        setVolume(clampVolume());
      } catch (e) { /* the page changed under us */ }
    }
  };
  function clampVolume() {
    var v = Math.round(Number(media.volume));
    return isFinite(v) ? Math.max(0, Math.min(100, v)) : 100;
  }
  function playerApi() {
    var p = document.getElementById('movie_player');
    return p && typeof p.playVideo === 'function' && typeof p.pauseVideo === 'function' &&
      typeof p.getPlayerState === 'function' ? p : null;
  }
  function setVolume(vol) {
    var p = playerApi();
    if (p && typeof p.setVolume === 'function') {
      if (typeof p.getVolume !== 'function' || p.getVolume() !== vol) p.setVolume(vol);
      if (vol > 0 && typeof p.isMuted === 'function' && p.isMuted() && typeof p.unMute === 'function') p.unMute();
      return;
    }
    var v = document.querySelector('video');
    if (v) {
      if (Math.abs(v.volume - vol / 100) > 0.005) v.volume = vol / 100;
      if (vol > 0 && v.muted) v.muted = false;
    }
  }
  function enforce() {
    var play = media.want === 'play';
    try {
      setVolume(clampVolume());
      var p = playerApi();
      if (p) {
        // -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued
        var state = p.getPlayerState();
        if (play) {
          if (state === -1 || state === 2 || state === 5) p.playVideo();
        } else if (state === -1 || state === 1 || state === 3) {
          p.pauseVideo();
        }
        return;
      }
      var v = document.querySelector('video');
      if (!v) return;
      if (play) {
        if (v.paused && !v.ended) {
          var started = v.play();
          if (started && typeof started.catch === 'function') started.catch(function () {});
        }
      } else if (!v.paused) {
        v.pause();
      }
    } catch (e) { /* the page changed under us; the next tick retries */ }
  }
  function touched(event) {
    if (event.isTrusted) media.userTouched = true;
  }
  window.addEventListener('pointerdown', touched, true);
  window.addEventListener('keydown', touched, true);
  setInterval(function () {
    if (location.href !== media.href) {
      media.href = location.href;
      if (!media.userTouched) media.until = Date.now() + ENFORCE_MS;
    }
    if (!media.userTouched && Date.now() < media.until) enforce();
  }, 500);
  window.__focusmodeMedia = media;
})();"#;

/// Managed handle to the media window state (cheap to clone).
#[derive(Clone, Default)]
pub struct MediaWindow {
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    state: Mutex<Inner>,
    /// Serializes open/control so two calls never race to create the window.
    ops: tokio::sync::Mutex<()>,
    /// Last link handed to the system browser (rate limit).
    last_external: Mutex<Option<Instant>>,
}

#[derive(Default)]
struct Inner {
    revision: u32,
    open: bool,
    url: Option<String>,
    title: Option<String>,
    /// Desired playback (the session wants music now).
    play: bool,
    /// Desired volume 0..=100; `None` until the first open.
    volume: Option<u8>,
    /// Bumped for every created window, so late events of a closed window are ignored.
    instance: u64,
    /// We asked the current window to close and it is not destroyed yet.
    closing: bool,
}

impl Inner {
    fn status(&self) -> MediaWindowStatus {
        MediaWindowStatus {
            revision: self.revision,
            open: self.open,
            url: self.url.clone(),
            title: self.title.clone(),
        }
    }

    fn volume(&self) -> u8 {
        self.volume.unwrap_or(DEFAULT_VOLUME)
    }
}

impl MediaWindow {
    pub fn status(&self) -> MediaWindowStatus {
        lock(&self.shared.state).status()
    }

    /// Opens the window on `raw_url`, or reuses it (navigating only when the link changed).
    /// `default_volume` (0..=1, from the config) is used until the UI sends a volume.
    pub async fn open(
        &self,
        app: &AppHandle,
        raw_url: &str,
        play: bool,
        default_volume: f32,
    ) -> Result<(), AppError> {
        let url = parse_media_url(raw_url)?;
        let _serial = self.shared.ops.lock().await;
        self.wait_for_requested_close(app).await;
        let (volume, previous_url) = {
            let mut inner = lock(&self.shared.state);
            inner.play = play;
            let volume = *inner
                .volume
                .get_or_insert_with(|| volume_percent(f64::from(default_volume)));
            (volume, inner.url.clone())
        };

        if let Some(window) = app.get_webview_window(MEDIA_WINDOW) {
            if previous_url.as_deref() == Some(url.as_str()) {
                // Same link: keep the page, just apply the wanted state. The status is emitted even
                // when nothing changed: the UI needs an `open` status after every request to
                // recognize a later close by the user.
                eval(&window, &state_script(play, volume));
                self.update(app, |inner| {
                    inner.open = true;
                    true
                });
                return Ok(());
            }
            log::info!("media window: navigating to {}", host_of(&url));
            window.navigate(url.clone()).map_err(|err| {
                AppError::new("media.window", format!("cannot open the link: {err}"))
            })?;
            // The page-load hook applies the wanted state to the new document.
            self.update(app, |inner| {
                inner.open = true;
                inner.url = Some(url.to_string());
                inner.title = None;
                true
            });
            return Ok(());
        }

        let instance = {
            let mut inner = lock(&self.shared.state);
            inner.instance = inner.instance.wrapping_add(1);
            inner.title = None;
            inner.closing = false;
            inner.instance
        };
        log::info!("media window: opening {}", host_of(&url));
        let needs_consent = !has_youtube_consent(app);
        if needs_consent {
            log::info!("media window: no YouTube consent cookie yet, showing the window in front");
        }
        let window = self
            .builder(app, url.clone(), instance, play, volume, needs_consent)
            .build()
            .map_err(|err| {
                AppError::new(
                    "media.window",
                    format!("cannot create the YouTube window: {err}"),
                )
            })?;
        let this = self.clone();
        let handle = app.clone();
        let hide_target = window.clone();
        window.on_window_event(move |event| match event {
            // The window is only a background player: X hides it and the music keeps playing.
            // The app itself destroys it (MediaAction::Close), which does not come through here.
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Err(err) = hide_target.hide() {
                    log::warn!("cannot hide the media window: {err}");
                }
            }
            tauri::WindowEvent::Destroyed => this.destroyed(&handle, instance),
            _ => {}
        });
        if needs_consent {
            self.hide_after_consent(app, instance);
        }
        self.update(app, |inner| {
            if inner.instance != instance {
                return false;
            }
            // Keeps a title that may already have arrived while `build` returned.
            inner.open = true;
            inner.url = Some(url.to_string());
            true
        });
        Ok(())
    }

    /// Play / pause / volume / show / close. Updates the desired state even when the window is
    /// closed (the next open uses it); acting on the window is a no-op when it is closed.
    pub async fn control(
        &self,
        app: &AppHandle,
        action: MediaAction,
        volume: Option<f64>,
    ) -> Result<(), AppError> {
        let _serial = self.shared.ops.lock().await;
        let window = app.get_webview_window(MEDIA_WINDOW);
        match action {
            MediaAction::Play | MediaAction::Pause => {
                let (play, volume) = {
                    let mut inner = lock(&self.shared.state);
                    inner.play = action == MediaAction::Play;
                    (inner.play, inner.volume())
                };
                if let Some(window) = window {
                    eval(&window, &state_script(play, volume));
                }
            }
            MediaAction::SetVolume => {
                let Some(volume) = volume.filter(|volume| volume.is_finite()) else {
                    return Err(AppError {
                        code: "media.window".into(),
                        message: "setVolume needs a volume between 0 and 1".into(),
                        field: Some("volume".into()),
                    });
                };
                let volume = volume_percent(volume);
                lock(&self.shared.state).volume = Some(volume);
                if let Some(window) = window {
                    eval(&window, &volume_script(volume));
                }
            }
            MediaAction::Show => {
                if let Some(window) = window {
                    if let Err(err) = window.unminimize() {
                        log::debug!("media window unminimize failed: {err}");
                    }
                    if let Err(err) = window.show() {
                        log::debug!("media window show failed: {err}");
                    }
                    if let Err(err) = window.set_focus() {
                        log::debug!("media window set_focus failed: {err}");
                    }
                }
            }
            MediaAction::Close => {
                if let Some(window) = window {
                    lock(&self.shared.state).closing = true;
                    // `destroy`, not `close`: a user close request only hides the window.
                    if let Err(err) = window.destroy() {
                        lock(&self.shared.state).closing = false;
                        return Err(AppError::new(
                            "media.window",
                            format!("cannot close the YouTube window: {err}"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// After our own `close` the window keeps its label until `Destroyed`; an `open` in that gap
    /// would navigate a dying window. Waits (bounded) until it is gone, then `open` builds a new one.
    async fn wait_for_requested_close(&self, app: &AppHandle) {
        if !lock(&self.shared.state).closing {
            return;
        }
        let deadline = Instant::now() + CLOSE_WAIT;
        while app.get_webview_window(MEDIA_WINDOW).is_some() {
            if Instant::now() >= deadline {
                log::warn!("media window: the closing window is still there, reusing it");
                return;
            }
            tokio::time::sleep(CLOSE_POLL).await;
        }
    }

    fn builder<'a>(
        &self,
        app: &'a AppHandle,
        url: Url,
        instance: u64,
        play: bool,
        volume: u8,
        needs_consent: bool,
    ) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
        let mut builder = WebviewWindowBuilder::new(app, MEDIA_WINDOW, WebviewUrl::External(url))
            .title(texts::MEDIA_WINDOW_TITLE)
            .inner_size(WIDTH, HEIGHT)
            .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
            .resizable(true)
            // A hidden background player. Exception: until the YouTube cookie choice is made, YouTube
            // shows its cookie dialog over the player and plays nothing, so on that first run the
            // window comes up centered and in front, and hides itself once the choice is saved.
            .visible(needs_consent)
            .focused(needs_consent)
            .initialization_script(init_script(play, volume));
        builder = match bottom_right_position(app).filter(|_| !needs_consent) {
            Some((x, y)) => builder.position(x, y),
            None => builder.center(),
        };

        // WebView2 shares one browser process per data directory, and all its webviews must use the
        // same environment options, or the second webview fails to be created.
        if let Some(main) = app
            .config()
            .app
            .windows
            .iter()
            .find(|window| window.label == MAIN_WINDOW)
        {
            if let Some(args) = &main.additional_browser_args {
                builder = builder.additional_browser_args(args);
            }
            builder = builder.browser_extensions_enabled(main.browser_extensions_enabled);
            #[cfg(windows)]
            if matches!(
                main.scroll_bar_style,
                tauri::utils::config::ScrollBarStyle::FluentOverlay
            ) {
                builder = builder.scroll_bar_style(tauri::webview::ScrollBarStyle::FluentOverlay);
            }
        }

        let nav_app = app.clone();
        let nav_shared = Arc::clone(&self.shared);
        let popup_app = app.clone();
        let popup_shared = Arc::clone(&self.shared);
        let title_this = self.clone();
        let title_app = app.clone();
        let load_this = self.clone();
        builder
            .on_navigation(move |url| {
                if is_allowed_navigation(url) {
                    return true;
                }
                open_externally(&nav_app, &nav_shared, url);
                false
            })
            .on_new_window(move |url, _features| {
                open_externally(&popup_app, &popup_shared, &url);
                NewWindowResponse::Deny
            })
            .on_document_title_changed(move |_window, title| {
                title_this.title_changed(&title_app, instance, &title);
            })
            .on_page_load(move |window, _payload| {
                // Every new document (reload, consent redirect, `navigate`) gets the current wanted
                // state; the init script only knows the state from window creation. When `Started`
                // arrives before the init script ran, the call is a no-op and `Finished` applies it.
                let (play, volume) = {
                    let inner = lock(&load_this.shared.state);
                    (inner.play, inner.volume())
                };
                eval(&window, &state_script(play, volume));
            })
    }

    fn title_changed(&self, app: &AppHandle, instance: u64, raw: &str) {
        let Some(title) = clean_title(raw) else {
            return;
        };
        self.update(app, |inner| {
            if inner.instance != instance || inner.title.as_deref() == Some(&title) {
                return false;
            }
            inner.title = Some(title);
            // Before the window is published as open, the title is stored and goes out with it.
            inner.open
        });
    }

    /// First run: waits (bounded) until YouTube stored the cookie choice, then hides the window.
    fn hide_after_consent(&self, app: &AppHandle, instance: u64) {
        let this = self.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let deadline = Instant::now() + CONSENT_WAIT;
            while Instant::now() < deadline {
                tokio::time::sleep(CONSENT_POLL).await;
                if lock(&this.shared.state).instance != instance {
                    return;
                }
                let Some(window) = app.get_webview_window(MEDIA_WINDOW) else {
                    return;
                };
                if webview_has_consent(&window) {
                    tokio::time::sleep(CONSENT_HIDE_DELAY).await;
                    if lock(&this.shared.state).instance == instance {
                        log::info!("media window: YouTube cookie choice saved, hiding the window");
                        if let Err(err) = window.hide() {
                            log::warn!("cannot hide the media window: {err}");
                        }
                    }
                    return;
                }
            }
            log::info!("media window: no YouTube cookie choice yet, leaving the window visible");
        });
    }

    fn destroyed(&self, app: &AppHandle, instance: u64) {
        self.update(app, |inner| {
            if inner.instance != instance {
                return false;
            }
            inner.closing = false;
            if !inner.open {
                return false;
            }
            log::info!("media window closed");
            inner.open = false;
            inner.title = None;
            true
        });
    }

    /// Applies `change`; when it reports a change, bumps the revision and emits `media://window`.
    fn update(&self, app: &AppHandle, change: impl FnOnce(&mut Inner) -> bool) {
        let status = {
            let mut inner = lock(&self.shared.state);
            if !change(&mut inner) {
                return;
            }
            inner.revision = inner.revision.wrapping_add(1);
            inner.status()
        };
        if let Err(err) = app.emit(events::MEDIA_WINDOW, status) {
            log::warn!("cannot emit media window status: {err}");
        }
    }
}

/// Accepts only `https` links on the YouTube / YouTube Music hosts (`media.url` otherwise).
fn parse_media_url(raw: &str) -> Result<Url, AppError> {
    let invalid = |why: &str| AppError::new("media.url", format!("not a YouTube link: {why}"));
    let url = Url::parse(raw.trim()).map_err(|_| invalid("cannot parse the URL"))?;
    if url.scheme() != "https" {
        return Err(invalid("only https links are allowed"));
    }
    if !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
        return Err(invalid("credentials and ports are not allowed"));
    }
    match url.host_str() {
        Some(host) if OPEN_HOSTS.contains(&host) => Ok(url),
        _ => Err(invalid("the host is not YouTube or YouTube Music")),
    }
}

/// Top-level navigations the media window may perform itself.
fn is_allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "about" => url.as_str() == "about:blank",
        "https" => url.host_str().is_some_and(|host| {
            host == "youtu.be"
                || GOOGLE_FLOW_HOSTS.contains(&host)
                || NAVIGATION_DOMAINS.iter().any(|domain| {
                    host == *domain
                        || host
                            .strip_suffix(domain)
                            .is_some_and(|sub| sub.ends_with('.'))
                })
        }),
        _ => false,
    }
}

/// Hands an http(s) link the media window must not open itself to the system browser (rate-limited).
fn open_externally(app: &AppHandle, shared: &Shared, url: &Url) {
    if !matches!(url.scheme(), "http" | "https") {
        log::debug!("media window: blocked a {} link", url.scheme());
        return;
    }
    {
        let mut last = lock(&shared.last_external);
        if last.is_some_and(|at| at.elapsed() < EXTERNAL_OPEN_EVERY) {
            log::debug!("media window: external link ignored (rate limit)");
            return;
        }
        *last = Some(Instant::now());
    }
    log::info!("media window: opening {} in the browser", host_of(url));
    let app = app.clone();
    let link = url.to_string();
    // Off the main thread: starting the browser can take a moment.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) = app.opener().open_url(link, None::<&str>) {
            log::warn!("cannot open a link in the browser: {err}");
        }
    });
}

/// The YouTube cookie choice is already stored in the shared WebView2 profile (read through the main
/// webview). Must not run on the main thread: WebView2 cookie reads deadlock there.
fn has_youtube_consent(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN_WINDOW)
        .is_some_and(|main| webview_has_consent(&main))
}

fn webview_has_consent(webview: &WebviewWindow) -> bool {
    let Ok(origin) = Url::parse(YOUTUBE_ORIGIN) else {
        return false;
    };
    match webview.cookies_for_url(origin) {
        Ok(cookies) => cookies
            .iter()
            .any(|cookie| cookie.name() == CONSENT_COOKIE && consent_decided(cookie.value())),
        Err(err) => {
            log::warn!("cannot read YouTube cookies: {err}");
            false
        }
    }
}

/// `SOCS` is a base64 protobuf whose first field (tag `0x08`) holds the cookie choice: `0` until the
/// user picks an option (YouTube sets `CAAaBg…` on the first visit), non-zero after "accept all"
/// (`CAIS…`) or "reject all".
fn consent_decided(socs: &str) -> bool {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let bytes = socs.as_bytes();
    let (Some(a), Some(b), Some(c)) = (
        bytes.first().copied().and_then(sextet),
        bytes.get(1).copied().and_then(sextet),
        bytes.get(2).copied().and_then(sextet),
    ) else {
        return false;
    };
    let tag = (a << 2) | (b >> 4);
    let choice = ((b & 0x0f) << 4) | (c >> 2);
    tag == 0x08 && choice != 0
}

/// Near the bottom-right corner of the primary monitor's work area (logical px).
fn bottom_right_position(app: &AppHandle) -> Option<(f64, f64)> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let area = monitor.work_area();
    let left = f64::from(area.position.x) / scale;
    let top = f64::from(area.position.y) / scale;
    let right = left + f64::from(area.size.width) / scale;
    let bottom = top + f64::from(area.size.height) / scale;
    let x = (right - WIDTH - SCREEN_MARGIN).max(left);
    let y = (bottom - HEIGHT - FRAME_ALLOWANCE - SCREEN_MARGIN).max(top);
    Some((x, y))
}

/// Fire-and-forget script evaluation; failures are only logged.
fn eval(window: &WebviewWindow, script: &str) {
    if let Err(err) = window.eval(script) {
        log::warn!("cannot control the media window: {err}");
    }
}

fn want_literal(play: bool) -> &'static str {
    if play { "'play'" } else { "'pause'" }
}

fn init_script(play: bool, volume: u8) -> String {
    INIT_SCRIPT
        .replace("__WANT__", want_literal(play))
        .replace("__VOLUME__", &volume.to_string())
}

/// Sets the wanted play state and volume, then applies them (and keeps applying them for a while).
fn state_script(play: bool, volume: u8) -> String {
    format!(
        "window.__focusmodeMedia && (window.__focusmodeMedia.want = {}, window.__focusmodeMedia.volume = {volume}, window.__focusmodeMedia.apply());",
        want_literal(play)
    )
}

/// Changes only the volume (does not touch play/pause, so it never overrides the user's pause).
fn volume_script(volume: u8) -> String {
    format!(
        "window.__focusmodeMedia && (window.__focusmodeMedia.volume = {volume}, window.__focusmodeMedia.applyVolume());"
    )
}

/// 0..=1 → 0..=100.
fn volume_percent(volume: f64) -> u8 {
    let volume = if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        1.0
    };
    // In 0.0..=100.0 after the clamp, so the cast is exact.
    (volume * 100.0).round() as u8
}

/// Page title without the YouTube suffix and the "(3) " notification counter; `None` when it is
/// not a real title yet (empty, the bare site name, or the URL WebView2 reports while loading).
fn clean_title(raw: &str) -> Option<String> {
    let mut title = raw.trim();
    for suffix in [" - YouTube Music", " - YouTube"] {
        if let Some(stripped) = title.strip_suffix(suffix) {
            title = stripped.trim_end();
            break;
        }
    }
    if let Some(rest) = title.strip_prefix('(')
        && let Some((count, after)) = rest.split_once(')')
        && !count.is_empty()
        && count.chars().all(|c| c.is_ascii_digit() || c == '+')
    {
        title = after.trim_start();
    }
    let looks_like_url = title.starts_with("http://")
        || title.starts_with("https://")
        || title.contains("youtube.com/")
        || title.contains("youtu.be/");
    if title.is_empty() || title == "YouTube" || title == "YouTube Music" || looks_like_url {
        None
    } else {
        Some(title.to_owned())
    }
}

/// Host only: links may carry personal playlist ids in the query, which do not belong in the log.
fn host_of(url: &Url) -> &str {
    url.host_str().unwrap_or("?")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(raw: &str) -> Url {
        Url::parse(raw).expect("test url")
    }

    #[test]
    fn accepts_only_https_youtube_links() {
        for ok in [
            "https://www.youtube.com/watch?v=jfKfPfyJRdk",
            "  https://youtube.com/@LofiGirl/live ",
            "https://m.youtube.com/channel/UCSJ4gkVC6NrvII8umztf0Ow/live",
            "https://music.youtube.com/playlist?list=PL123",
            "https://youtu.be/jfKfPfyJRdk",
            "https://WWW.YouTube.com/watch?v=x",
        ] {
            assert!(parse_media_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://www.youtube.com/watch?v=x",
            "https://www.youtube.com.evil.com/watch?v=x",
            "https://evilyoutube.com/",
            "https://gaming.youtube.com/",
            "https://user:pw@www.youtube.com/",
            "https://www.youtube.com:8443/",
            "javascript:alert(1)",
            "file:///C:/x.html",
            "not a url",
            "",
        ] {
            let err = parse_media_url(bad).expect_err(bad);
            assert_eq!(err.code, "media.url");
            assert_eq!(err.field, None);
        }
    }

    #[test]
    fn navigation_guard() {
        for ok in [
            "https://www.youtube.com/watch?v=x",
            "https://consent.youtube.com/m?continue=x",
            "https://accounts.youtube.com/accounts/SetSID",
            "https://youtu.be/x",
            "https://accounts.google.com/ServiceLogin",
            "https://consent.google.com/ml?continue=x",
            "https://www.google.com/sorry/index",
            "https://www.gstatic.com/x.js",
            "https://rr1---sn-abc.googlevideo.com/videoplayback",
            "https://i.ytimg.com/vi/x/hq.jpg",
            "about:blank",
        ] {
            assert!(is_allowed_navigation(&url(ok)), "{ok}");
        }
        for bad in [
            "http://www.youtube.com/",
            "https://example.com/",
            "https://notyoutube.com/",
            "https://youtube.com.evil.com/",
            "https://google.com.evil.com/",
            "https://sites.google.com/view/anything",
            "https://docs.google.com/forms/d/x",
            "https://policies.google.com/privacy",
            "https://google.com/",
            "about:srcdoc",
            "data:text/html,hi",
            "file:///C:/x",
        ] {
            assert!(!is_allowed_navigation(&url(bad)), "{bad}");
        }
    }

    #[test]
    fn consent_cookie_values() {
        // Observed 2026-10-13: first visit (no choice yet) vs after "Zaakceptuj wszystko".
        assert!(!consent_decided("CAAaBgiAxrDWBg"));
        assert!(consent_decided(
            "CAISNQgDEitib3FfaWRlbnRpdHlmcm9udGVuZHVpc2VydmVyXzIwMjYxMDExLjA0X3AwGgJwbCACGgYIgMaw1gY"
        ));
        assert!(consent_decided("CAESEwgDEgk"));
        assert!(!consent_decided(""));
        assert!(!consent_decided("CA"));
        assert!(!consent_decided("YES+cb"));
    }

    #[test]
    fn titles() {
        assert_eq!(
            clean_title("lofi hip hop radio 📚 - beats to relax/study to - YouTube").as_deref(),
            Some("lofi hip hop radio 📚 - beats to relax/study to")
        );
        assert_eq!(
            clean_title("(3) Song - YouTube Music").as_deref(),
            Some("Song")
        );
        assert_eq!(clean_title("(Live) Song").as_deref(), Some("(Live) Song"));
        assert_eq!(clean_title("YouTube"), None);
        assert_eq!(clean_title("YouTube Music"), None);
        assert_eq!(clean_title("   "), None);
        assert_eq!(clean_title("www.youtube.com/watch?v=x"), None);
        assert_eq!(clean_title("https://consent.youtube.com/m"), None);
    }

    #[test]
    fn volumes() {
        assert_eq!(volume_percent(0.0), 0);
        assert_eq!(volume_percent(0.555), 56);
        assert_eq!(volume_percent(1.0), 100);
        assert_eq!(volume_percent(7.0), 100);
        assert_eq!(volume_percent(-1.0), 0);
        assert_eq!(volume_percent(f64::NAN), 100);
    }

    #[test]
    fn scripts_embed_only_fixed_literals() {
        let script = init_script(false, 42);
        assert!(!script.contains("__WANT__") && !script.contains("__VOLUME__"));
        assert!(script.contains("want: 'pause',"));
        assert!(script.contains("volume: 42,"));
        assert!(init_script(true, 0).contains("want: 'play',"));
        assert_eq!(
            state_script(true, 7),
            "window.__focusmodeMedia && (window.__focusmodeMedia.want = 'play', window.__focusmodeMedia.volume = 7, window.__focusmodeMedia.apply());"
        );
        assert!(volume_script(100).contains("volume = 100, window.__focusmodeMedia.applyVolume()"));
    }
}
