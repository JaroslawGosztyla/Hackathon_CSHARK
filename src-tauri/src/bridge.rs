//! Loopback HTTP bridge for the browser extension (`GET http://127.0.0.1:47615/v1/state`).
//!
//! Read-only: the extension polls the current blocking state. Nothing is sent anywhere else.
//! CORS is granted only to extension origins; the `Host` header is checked against DNS rebinding.

use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use focus_core::api::{AppError, BRIDGE_PORT, BridgeState, Phase};
use focus_core::blocklist::Blocklist;
use tauri::AppHandle;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::lock;
use crate::shields::ShieldsHandle;

/// The extension counts as connected when it polled within this window.
const EXTENSION_TIMEOUT: Duration = Duration::from_secs(90);

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
pub fn spawn(app: AppHandle, shared: SharedBridge, shields: ShieldsHandle) -> anyhow::Result<()> {
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
                handle(&shared, &shields, request);
            }
        })?;
    Ok(())
}

fn handle(shared: &SharedBridge, shields: &ShieldsHandle, request: Request) {
    let origin = header(&request, "Origin");
    let allowed_origin = origin.filter(|origin| is_extension_origin(origin));
    let from_extension = allowed_origin.is_some()
        || header(&request, "X-FocusMode-Extension").is_some_and(|value| value.trim() == "1");

    if header(&request, "Host").is_some_and(|host| !is_loopback_host(&host)) {
        respond(request, text_response(403, "forbidden"));
        return;
    }

    let path = request.url().split('?').next().unwrap_or("").to_owned();
    let response = match (request.method(), path.as_str()) {
        (Method::Options, _) => preflight(text_response(204, ""), allowed_origin.as_deref()),
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
    let response = with_header(response, "Access-Control-Allow-Methods", "GET, OPTIONS");
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
