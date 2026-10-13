//! Loopback HTTP bridge for the browser extension (`GET http://127.0.0.1:47615/v1/state`).
//!
//! The extension polls the current blocking state. The only thing it may tell the app is which of the
//! app's own regex patterns it could not turn into network rules (`POST /v1/report` with
//! `{"skipped": string[]}`, see [`parse_report`] for the limits), accepted only from an extension origin with
//! `X-FocusMode-Extension: 1` and kept by `crate::blocklist` (`BlocklistInfo.extension_skipped`). Everything
//! else is read-only. Nothing is sent anywhere else.
//! CORS is granted only to extension origins; the `Host` header is checked against DNS rebinding.
//! HTTP itself is the minimal server in [`http`]: bounded heads and bodies, deadlines, one request per
//! connection, and the oldest connection gives way to a new one (ADR-018, "Utwardzenie mostu").

mod http;
#[cfg(test)]
mod server_tests;

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{AppError, BRIDGE_PORT, BridgeState, Phase};
use focus_core::blocklist::{Blocklist, MAX_LINE_CHARS, MAX_PATTERNS};
use serde::Deserialize;
use tauri::AppHandle;

use self::http::{BodyError, Handler, Head, Limits, Method, Next, Response};
use crate::blocklist::BlocklistService;
use crate::lock;
use crate::shields::ShieldsHandle;

/// The extension counts as connected when it polled within this window.
const EXTENSION_TIMEOUT: Duration = Duration::from_secs(90);
/// Limits of `POST /v1/report`: number of sources and characters per source, the same as the list the app
/// sends (the extension reports a subset of it)...
const REPORT_MAX_ENTRIES: usize = MAX_PATTERNS;
const REPORT_MAX_ENTRY_CHARS: usize = MAX_LINE_CHARS;
/// ...and a body that fits the longest such list as the extension's `JSON.stringify` writes it (at most 6
/// bytes per character, e.g. `\u0001`; quotes and a comma per source; the object around them), about 1.5 MB.
const REPORT_MAX_BYTES: usize = REPORT_MAX_ENTRIES * (REPORT_MAX_ENTRY_CHARS * 6 + 3) + 64;
/// Header the extension adds to every request.
const MARKER: &str = "X-FocusMode-Extension";

/// State shared between the session actor, the shields worker, the blocklist service, the tracker
/// and the HTTP thread.
#[derive(Debug, Clone)]
pub struct BridgeShared {
    pub phase: Phase,
    pub remaining_secs: u32,
    /// Sites shield is up (focus running or paused and `block_sites` on). Set by the shields worker.
    pub block_active: bool,
    /// Normalized domains of `websiteblacklist.txt`. Set by `crate::blocklist`.
    pub domains: Vec<String>,
    /// Regex sources of `websiteblacklist.txt`. Set by `crate::blocklist`.
    pub patterns: Vec<String>,
    /// Last request from the extension.
    pub last_seen: Option<Instant>,
}

pub type SharedBridge = Arc<Mutex<BridgeShared>>;

/// Empty list until `crate::blocklist` loads `websiteblacklist.txt`.
pub fn new_shared() -> SharedBridge {
    Arc::new(Mutex::new(BridgeShared {
        phase: Phase::Idle,
        remaining_secs: 0,
        block_active: false,
        domains: Vec::new(),
        patterns: Vec::new(),
        last_seen: None,
    }))
}

impl BridgeShared {
    /// The list served to the extension while blocking.
    pub fn set_blocklist(&mut self, list: &Blocklist) {
        self.domains = list.domains().to_vec();
        self.patterns = list.pattern_sources();
    }

    pub fn extension_connected(&self) -> bool {
        self.last_seen
            .is_some_and(|seen| seen.elapsed() < EXTENSION_TIMEOUT)
    }

    pub fn seen_secs_ago(&self) -> Option<u32> {
        self.last_seen
            .map(|seen| u32::try_from(seen.elapsed().as_secs()).unwrap_or(u32::MAX))
    }

    fn state(&self) -> BridgeState {
        let (blocklist, patterns) = if self.block_active {
            (self.domains.clone(), self.patterns.clone())
        } else {
            (Vec::new(), Vec::new())
        };
        BridgeState {
            active: self.block_active,
            blocklist,
            patterns,
            phase: self.phase,
            remaining_secs: self.remaining_secs,
        }
    }
}

/// Starts the HTTP thread. A busy port is reported as `bridge.port`; the app keeps running.
/// `blocklist` keeps the extension's reports.
pub fn spawn(
    app: AppHandle,
    shared: SharedBridge,
    shields: ShieldsHandle,
    blocklist: BlocklistService,
) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("focus-bridge".into())
        .spawn(move || {
            let cannot_listen = |err: std::io::Error| {
                crate::emit_startup_error(
                    &app,
                    AppError::new(
                        "bridge.port",
                        format!("cannot listen on 127.0.0.1:{BRIDGE_PORT}: {err}"),
                    ),
                );
            };
            let listener = match TcpListener::bind(("127.0.0.1", BRIDGE_PORT)) {
                Ok(listener) => listener,
                Err(err) => return cannot_listen(err),
            };
            log::info!("extension bridge listening on 127.0.0.1:{BRIDGE_PORT}");
            let router = Router {
                app: Arc::new(AppBridge {
                    shared,
                    shields,
                    blocklist,
                }),
            };
            // Returns only when the server cannot start (no runtime, socket not registered).
            if let Err(err) = http::serve(listener, Arc::new(router), Limits::DEFAULT) {
                cannot_listen(err);
            }
        })?;
    Ok(())
}

/// What the bridge serves: the running app ([`AppBridge`]) or a test double.
trait BridgeApp: Send + Sync + 'static {
    /// The state for `GET /v1/state`; `from_extension` marks the extension as connected.
    fn state(&self, from_extension: bool) -> BridgeState;
    /// The skipped patterns of an accepted `POST /v1/report`. May block briefly (runs off the server thread).
    fn report(&self, skipped: &[String]);
}

struct AppBridge {
    shared: SharedBridge,
    shields: ShieldsHandle,
    blocklist: BlocklistService,
}

impl BridgeApp for AppBridge {
    fn state(&self, from_extension: bool) -> BridgeState {
        let mut bridge = lock(&self.shared);
        if from_extension {
            let was_connected = bridge.extension_connected();
            bridge.last_seen = Some(Instant::now());
            if !was_connected {
                log::info!("browser extension connected");
                self.shields.refresh();
            }
        }
        bridge.state()
    }

    fn report(&self, skipped: &[String]) {
        self.blocklist.set_extension_skipped(skipped);
    }
}

/// Routes, origin checks and CORS of the bridge; HTTP itself is in [`http`].
struct Router<A> {
    app: Arc<A>,
}

/// Who sent a request, from its headers.
struct Caller {
    /// The `Origin` when it is an extension origin; CORS headers are granted only to it.
    allowed_origin: Option<String>,
    /// `X-FocusMode-Extension: 1`.
    marked: bool,
}

impl Caller {
    fn of(head: &Head) -> Self {
        Self {
            allowed_origin: head
                .header("Origin")
                .filter(|origin| is_extension_origin(origin))
                .map(str::to_owned),
            marked: head.header(MARKER).is_some_and(|value| value.trim() == "1"),
        }
    }

    fn origin(&self) -> Option<&str> {
        self.allowed_origin.as_deref()
    }
}

impl<A: BridgeApp> Handler for Router<A> {
    fn head(&self, head: &Head) -> Next {
        // One of each, so there is no question which one counts.
        if head.count("Origin") > 1 || head.count(MARKER) > 1 {
            return Next::Respond(Response::text(400, "bad request"));
        }
        if head
            .header("Host")
            .is_some_and(|host| !is_loopback_host(host))
        {
            return Next::Respond(Response::text(403, "forbidden"));
        }
        let caller = Caller::of(head);
        let origin = caller.origin();
        let response = match (head.method, head.path.as_str()) {
            (Method::Options, _) => preflight(Response::text(204, ""), origin),
            // Both: a web page cannot send an extension `Origin`, and the custom header needs a preflight
            // that only extension origins pass.
            (Method::Post, "/v1/report") if origin.is_some() && caller.marked => {
                return Next::ReadBody {
                    max_bytes: REPORT_MAX_BYTES,
                };
            }
            (Method::Post, "/v1/report") => {
                cors(no_store(Response::text(403, "forbidden")), origin)
            }
            (Method::Get, "/v1/state") => {
                let state = self.app.state(origin.is_some() || caller.marked);
                let response = match serde_json::to_string(&state) {
                    Ok(json) => Response::json(200, json),
                    Err(err) => Response::text(500, &err.to_string()),
                };
                cors(no_store(response), origin)
            }
            _ => cors(Response::text(404, "not found"), origin),
        };
        Next::Respond(response)
    }

    /// `POST /v1/report` from the extension: 204 when taken (changed or not), 413 / 400 when refused
    /// (408 / 503 when the body did not arrive in time).
    fn body(&self, head: &Head, body: Result<Vec<u8>, BodyError>) -> Response {
        let parsed = body
            .map_err(ReportError::from)
            .and_then(|body| parse_report(&body));
        let response = match parsed {
            Ok(skipped) => {
                self.app.report(&skipped);
                Response::text(204, "")
            }
            Err(err) => {
                log::debug!("extension report refused: {err:?}");
                match err {
                    ReportError::TooLarge => Response::text(413, "too large"),
                    ReportError::Unreadable | ReportError::Invalid => {
                        Response::text(400, "bad request")
                    }
                    ReportError::TimedOut => Response::text(408, "request timeout"),
                    ReportError::Busy => Response::text(503, "busy"),
                }
            }
        };
        cors(no_store(response), Caller::of(head).origin())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportError {
    /// Body over [`REPORT_MAX_BYTES`] (declared longer, never read) or of unknown length.
    TooLarge,
    /// The connection ended before the whole body arrived.
    Unreadable,
    /// The body did not arrive in time.
    TimedOut,
    /// Other reports were being read the whole time.
    Busy,
    /// Not `{"skipped": string[]}`, or over the entry limits.
    Invalid,
}

impl From<BodyError> for ReportError {
    fn from(err: BodyError) -> Self {
        match err {
            BodyError::TooLarge => Self::TooLarge,
            BodyError::Incomplete => Self::Unreadable,
            BodyError::TimedOut => Self::TimedOut,
            BodyError::Busy => Self::Busy,
        }
    }
}

#[derive(Deserialize)]
struct Report {
    skipped: Vec<String>,
}

/// `{"skipped": string[]}` with at most [`REPORT_MAX_ENTRIES`] sources of at most [`REPORT_MAX_ENTRY_CHARS`]
/// characters each (other fields are ignored).
fn parse_report(body: &[u8]) -> Result<Vec<String>, ReportError> {
    if body.len() > REPORT_MAX_BYTES {
        return Err(ReportError::TooLarge);
    }
    let report: Report = serde_json::from_slice(body).map_err(|_| ReportError::Invalid)?;
    let within_limits = report.skipped.len() <= REPORT_MAX_ENTRIES
        && report
            .skipped
            .iter()
            .all(|source| source.chars().count() <= REPORT_MAX_ENTRY_CHARS);
    if within_limits {
        Ok(report.skipped)
    } else {
        Err(ReportError::Invalid)
    }
}

fn no_store(response: Response) -> Response {
    response.with_header("Cache-Control", "no-store")
}

fn cors(response: Response, origin: Option<&str>) -> Response {
    match origin {
        Some(origin) => response
            .with_header("Access-Control-Allow-Origin", origin)
            .with_header("Vary", "Origin"),
        None => response,
    }
}

fn preflight(response: Response, origin: Option<&str>) -> Response {
    if origin.is_none() {
        return response;
    }
    cors(response, origin)
        .with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .with_header(
            "Access-Control-Allow-Headers",
            "X-FocusMode-Extension, Content-Type",
        )
        .with_header("Access-Control-Allow-Private-Network", "true")
        .with_header("Access-Control-Max-Age", "600")
}

fn is_extension_origin(origin: &str) -> bool {
    ["chrome-extension://", "moz-extension://", "extension://"]
        .iter()
        .any(|scheme| origin.starts_with(scheme))
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();
    let name = match host.rsplit_once(':') {
        Some((name, port)) if port.chars().all(|c| c.is_ascii_digit()) => name.to_owned(),
        _ => host,
    };
    matches!(name.as_str(), "127.0.0.1" | "localhost" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_origins() {
        assert!(is_extension_origin("chrome-extension://abcdef"));
        assert!(is_extension_origin("moz-extension://1234"));
        assert!(is_extension_origin("extension://1234"));
        assert!(!is_extension_origin("https://evil.example"));
        assert!(!is_extension_origin("null"));
    }

    #[test]
    fn loopback_hosts() {
        assert!(is_loopback_host("127.0.0.1:47615"));
        assert!(is_loopback_host("localhost:47615"));
        assert!(is_loopback_host("LOCALHOST"));
        assert!(!is_loopback_host("evil.example:47615"));
        assert!(!is_loopback_host("127.0.0.1.evil.example"));
    }

    #[test]
    fn reports_carry_only_a_bounded_list_of_strings() {
        assert_eq!(
            parse_report(br#"{"skipped": ["a\\d+", "(?<=x)y"]}"#),
            Ok(vec![r"a\d+".to_owned(), "(?<=x)y".to_owned()])
        );
        assert_eq!(parse_report(br#"{"skipped": []}"#), Ok(Vec::new()));
        // Unknown fields are ignored; anything else is refused.
        assert_eq!(
            parse_report(br#"{"skipped": ["x+"], "version": 2}"#),
            Ok(vec!["x+".to_owned()])
        );
        let refused: [&[u8]; 8] = [
            b"",
            b"null",
            b"[]",
            b"{}",
            br#"{"skipped": "x+"}"#,
            br#"{"skipped": [1]}"#,
            br#"{"skipped": [null]}"#,
            b"{\"skipped\": [\"x\"",
        ];
        for invalid in refused {
            assert_eq!(parse_report(invalid), Err(ReportError::Invalid));
        }

        let list = |count: usize, entry: &str| {
            serde_json::to_vec(&serde_json::json!({ "skipped": vec![entry; count] })).expect("json")
        };
        assert_eq!(
            parse_report(&list(REPORT_MAX_ENTRIES, "a")).map(|list| list.len()),
            Ok(REPORT_MAX_ENTRIES)
        );
        assert_eq!(
            parse_report(&list(REPORT_MAX_ENTRIES + 1, "a")),
            Err(ReportError::Invalid)
        );
        // Characters, not bytes: 500 "ż" (1000 bytes) fit, 501 do not.
        let longest = "ż".repeat(REPORT_MAX_ENTRY_CHARS);
        assert!(parse_report(&list(1, &longest)).is_ok());
        assert_eq!(
            parse_report(&list(1, &format!("{longest}a"))),
            Err(ReportError::Invalid)
        );
        // The longest list the app can send fits, even with every character escaped (`\u0001`, 6 bytes)...
        let worst = list(REPORT_MAX_ENTRIES, &"\u{1}".repeat(REPORT_MAX_ENTRY_CHARS));
        assert!(worst.len() <= REPORT_MAX_BYTES);
        assert_eq!(
            parse_report(&worst).map(|list| list.len()),
            Ok(REPORT_MAX_ENTRIES)
        );
        // ...and a larger body is refused, whatever the content.
        let large = list(10, &"a".repeat(REPORT_MAX_BYTES / 10));
        assert!(large.len() > REPORT_MAX_BYTES);
        assert_eq!(parse_report(&large), Err(ReportError::TooLarge));
    }

    #[test]
    fn inactive_state_hides_blocklist() {
        let shared = new_shared();
        let (list, _issues) = Blocklist::parse("reddit.com\n/youtube\\.com/shorts/\n");
        lock(&shared).set_blocklist(&list);
        let state = lock(&shared).state();
        assert!(!state.active);
        assert!(state.blocklist.is_empty());
        assert!(state.patterns.is_empty());
        lock(&shared).block_active = true;
        let state = lock(&shared).state();
        assert_eq!(state.blocklist, vec!["reddit.com".to_owned()]);
        assert_eq!(state.patterns, vec![r"youtube\.com/shorts".to_owned()]);
    }
}
