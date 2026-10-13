//! Loopback HTTP bridge for the browser extension (`GET http://127.0.0.1:47615/v1/state`).
//!
//! The extension polls the current blocking state. The only thing it may tell the app is which of the
//! app's own regex patterns it could not turn into network rules (`POST /v1/report` with
//! `{"skipped": string[]}`, see [`parse_report`] for the limits), accepted only from an extension origin with
//! `X-FocusMode-Extension: 1` and kept by `crate::blocklist` (`BlocklistInfo.extension_skipped`). Everything
//! else is read-only. Nothing is sent anywhere else.
//! CORS is granted only to extension origins; the `Host` header is checked against DNS rebinding.

use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{AppError, BRIDGE_PORT, BridgeState, Phase};
use focus_core::blocklist::{Blocklist, MAX_LINE_CHARS, MAX_PATTERNS};
use serde::Deserialize;
use tauri::AppHandle;
use tiny_http::{Header, Method, Request, Response, Server};

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
            let server = match Server::http(("127.0.0.1", BRIDGE_PORT)) {
                Ok(server) => server,
                Err(err) => {
                    crate::emit_startup_error(
                        &app,
                        AppError::new(
                            "bridge.port",
                            format!("cannot listen on 127.0.0.1:{BRIDGE_PORT}: {err}"),
                        ),
                    );
                    return;
                }
            };
            log::info!("extension bridge listening on 127.0.0.1:{BRIDGE_PORT}");
            for request in server.incoming_requests() {
                handle(&shared, &shields, &blocklist, request);
            }
        })?;
    Ok(())
}

fn handle(
    shared: &SharedBridge,
    shields: &ShieldsHandle,
    blocklist: &BlocklistService,
    mut request: Request,
) {
    let origin = header(&request, "Origin");
    let allowed_origin = origin.filter(|origin| is_extension_origin(origin));
    let marked = header(&request, "X-FocusMode-Extension").is_some_and(|value| value.trim() == "1");
    let from_extension = allowed_origin.is_some() || marked;

    if header(&request, "Host").is_some_and(|host| !is_loopback_host(&host)) {
        respond(request, text_response(403, "forbidden"));
        return;
    }

    let path = request.url().split('?').next().unwrap_or("").to_owned();
    let response = match (request.method(), path.as_str()) {
        (Method::Options, _) => preflight(text_response(204, ""), allowed_origin.as_deref()),
        (Method::Post, "/v1/report") => {
            // Both: a web page cannot send an extension `Origin`, and the custom header needs a preflight
            // that only extension origins pass.
            let response = if allowed_origin.is_some() && marked {
                take_report(&mut request, blocklist)
            } else {
                text_response(403, "forbidden")
            };
            cors(
                with_header(response, "Cache-Control", "no-store"),
                allowed_origin.as_deref(),
            )
        }
        (Method::Get, "/v1/state") => {
            let state = {
                let mut bridge = lock(shared);
                if from_extension {
                    let was_connected = bridge.extension_connected();
                    bridge.last_seen = Some(Instant::now());
                    if !was_connected {
                        log::info!("browser extension connected");
                        shields.refresh();
                    }
                }
                bridge.state()
            };
            let response = match serde_json::to_string(&state) {
                Ok(json) => with_header(
                    text_response(200, &json),
                    "Content-Type",
                    "application/json; charset=utf-8",
                ),
                Err(err) => text_response(500, &err.to_string()),
            };
            cors(
                with_header(response, "Cache-Control", "no-store"),
                allowed_origin.as_deref(),
            )
        }
        _ => cors(text_response(404, "not found"), allowed_origin.as_deref()),
    };
    respond(request, response);
}

/// `POST /v1/report` from the extension: 204 when taken (changed or not), 413 / 400 when refused.
fn take_report(request: &mut Request, blocklist: &BlocklistService) -> Response<Cursor<Vec<u8>>> {
    let parsed = read_body(request).and_then(|body| parse_report(&body));
    match parsed {
        Ok(skipped) => {
            blocklist.set_extension_skipped(&skipped);
            text_response(204, "")
        }
        Err(err) => {
            log::debug!("extension report refused: {err:?}");
            match err {
                ReportError::TooLarge => text_response(413, "too large"),
                ReportError::Unreadable | ReportError::Invalid => text_response(400, "bad request"),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportError {
    /// Body over [`REPORT_MAX_BYTES`].
    TooLarge,
    /// The body could not be read.
    Unreadable,
    /// Not `{"skipped": string[]}`, or over the entry limits.
    Invalid,
}

/// The request body, at most [`REPORT_MAX_BYTES`]; a larger declared length is refused unread.
fn read_body(request: &mut Request) -> Result<Vec<u8>, ReportError> {
    if request
        .body_length()
        .is_some_and(|len| len > REPORT_MAX_BYTES)
    {
        return Err(ReportError::TooLarge);
    }
    let limit = u64::try_from(REPORT_MAX_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut body = Vec::new();
    request
        .as_reader()
        .take(limit)
        .read_to_end(&mut body)
        .map_err(|_| ReportError::Unreadable)?;
    if body.len() > REPORT_MAX_BYTES {
        return Err(ReportError::TooLarge);
    }
    Ok(body)
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

fn respond(request: Request, response: Response<Cursor<Vec<u8>>>) {
    if let Err(err) = request.respond(response) {
        log::debug!("bridge response failed: {err}");
    }
}

fn header(request: &Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.as_str().to_owned())
}

fn text_response(status: u16, body: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_string(body).with_status_code(status)
}

fn with_header(
    response: Response<Cursor<Vec<u8>>>,
    name: &str,
    value: &str,
) -> Response<Cursor<Vec<u8>>> {
    match Header::from_bytes(name.as_bytes(), value.as_bytes()) {
        Ok(header) => response.with_header(header),
        Err(()) => response,
    }
}

fn cors(response: Response<Cursor<Vec<u8>>>, origin: Option<&str>) -> Response<Cursor<Vec<u8>>> {
    match origin {
        Some(origin) => with_header(
            with_header(response, "Access-Control-Allow-Origin", origin),
            "Vary",
            "Origin",
        ),
        None => response,
    }
}

fn preflight(
    response: Response<Cursor<Vec<u8>>>,
    origin: Option<&str>,
) -> Response<Cursor<Vec<u8>>> {
    if origin.is_none() {
        return response;
    }
    let response = cors(response, origin);
    let response = with_header(
        response,
        "Access-Control-Allow-Methods",
        "GET, POST, OPTIONS",
    );
    let response = with_header(
        response,
        "Access-Control-Allow-Headers",
        "X-FocusMode-Extension, Content-Type",
    );
    let response = with_header(response, "Access-Control-Allow-Private-Network", "true");
    with_header(response, "Access-Control-Max-Age", "600")
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
