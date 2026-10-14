//! The bridge over real sockets: the extension's own requests, and clients that misbehave on purpose
//! (huge declared bodies, chunked bodies, slow or oversized heads, many connections at once).

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use focus_core::api::{BridgeState, Phase};

use super::http::{self, Limits};
use super::{BridgeApp, REPORT_MAX_BYTES, REPORT_MAX_ENTRIES, REPORT_MAX_ENTRY_CHARS, Router};
use crate::lock;

/// The largest single allocation made in this test binary, so a test can show that no buffer was ever
/// sized from a declared `Content-Length` (the old server allocated the whole declared length).
static LARGEST_ALLOCATION: AtomicUsize = AtomicUsize::new(0);
/// Far above anything a test here allocates on purpose (a report is at most ~1.5 MB).
const ALLOCATION_ALARM: usize = 256 * 1024 * 1024;

struct TrackLargest;

// SAFETY: every call goes to `System` unchanged; the wrapper only records sizes.
unsafe impl GlobalAlloc for TrackLargest {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST_ALLOCATION.fetch_max(layout.size(), Ordering::Relaxed);
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        LARGEST_ALLOCATION.fetch_max(layout.size(), Ordering::Relaxed);
        // SAFETY: the caller upholds `GlobalAlloc::alloc_zeroed`'s contract.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds `GlobalAlloc::dealloc`'s contract.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LARGEST_ALLOCATION.fetch_max(new_size, Ordering::Relaxed);
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: TrackLargest = TrackLargest;

const ORIGIN: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop";
/// Answers that must not wait for any deadline come well within this.
const FAST: Duration = Duration::from_secs(1);
/// A request line that HTTP-inspecting security software lets through at once. Such software (found on a
/// Windows dev machine) holds back an incomplete `GET` / `POST` head on loopback until it is complete, so a
/// partial head sent to test the server would never reach it. Until the head is complete the server
/// treats every method alike.
const UNINSPECTED: &str = "FOO /v1/state HTTP/1.1\r\n";

#[derive(Default)]
struct FakeApp {
    /// `state` calls that marked the extension as seen.
    polls: AtomicUsize,
    reports: Mutex<Vec<Vec<String>>>,
    /// `report` calls so far, counted on entry.
    reports_entered: AtomicUsize,
    /// While true, `report` blocks (a slow handler, e.g. waiting for a blocklist reload).
    reports_held: Mutex<bool>,
    reports_released: Condvar,
}

impl FakeApp {
    fn sample() -> BridgeState {
        BridgeState {
            active: true,
            blocklist: vec!["reddit.com".to_owned()],
            patterns: vec![r"youtube\.com/shorts".to_owned()],
            phase: Phase::Focus,
            remaining_secs: 42,
        }
    }

    fn polls(&self) -> usize {
        self.polls.load(Ordering::SeqCst)
    }

    fn reports(&self) -> Vec<Vec<String>> {
        lock(&self.reports).clone()
    }

    fn hold_reports(&self, held: bool) {
        *lock(&self.reports_held) = held;
        self.reports_released.notify_all();
    }
}

impl BridgeApp for FakeApp {
    fn state(&self, from_extension: bool) -> BridgeState {
        if from_extension {
            self.polls.fetch_add(1, Ordering::SeqCst);
        }
        Self::sample()
    }

    fn report(&self, skipped: &[String]) {
        self.reports_entered.fetch_add(1, Ordering::SeqCst);
        let mut held = lock(&self.reports_held);
        while *held {
            held = self
                .reports_released
                .wait(held)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        drop(held);
        lock(&self.reports).push(skipped.to_vec());
    }
}

/// Deadlines long enough that an answer within [`FAST`] cannot have come from one of them.
fn patient() -> Limits {
    Limits {
        head_timeout: Duration::from_secs(10),
        body_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        ..Limits::DEFAULT
    }
}

fn start(limits: Limits) -> (SocketAddr, Arc<FakeApp>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    (addr, serve(listener, limits))
}

/// Serves an already bound listener (connections may be waiting in its queue).
fn serve(listener: TcpListener, limits: Limits) -> Arc<FakeApp> {
    let app = Arc::new(FakeApp::default());
    let router = Router {
        app: Arc::clone(&app),
    };
    thread::Builder::new()
        .name("bridge-test-server".into())
        .spawn(move || http::serve(listener, Arc::new(router), limits))
        .expect("server thread");
    app
}

fn connect(addr: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(15)))
        .expect("write timeout");
    stream
}

#[derive(Debug)]
struct Reply {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Reply {
    fn parse(raw: &[u8]) -> Self {
        let end = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or_else(|| panic!("no complete head in {:?}", String::from_utf8_lossy(raw)));
        let head = String::from_utf8_lossy(&raw[..end]).into_owned();
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|status| status.parse().ok())
            .expect("status");
        Self {
            status,
            head,
            body: raw[end + 4..].to_vec(),
        }
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (field, value) = line.split_once(':')?;
            field.eq_ignore_ascii_case(name).then_some(value.trim())
        })
    }
}

/// Everything until the server closes the connection; `None` when it reset it instead (it closed with
/// bytes of ours unread). Fails when the connection is still open after the read timeout.
fn read_until_closed(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut raw = Vec::new();
    match stream.read_to_end(&mut raw) {
        Ok(_) => Some(raw),
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
            ) =>
        {
            None
        }
        Err(err) => panic!("the server kept the connection open: {err}"),
    }
}

/// Sends `raw` and reads the answer, which must end with the server closing the connection.
fn send(addr: SocketAddr, raw: &[u8]) -> Reply {
    let mut stream = connect(addr);
    stream.write_all(raw).expect("send");
    let raw = read_until_closed(&mut stream).expect("answer, not a reset");
    let reply = Reply::parse(&raw);
    assert_eq!(reply.header("Connection"), Some("close"), "{}", reply.head);
    reply
}

/// Reads one head (e.g. `100 Continue`) without waiting for the connection to close.
fn read_interim(stream: &mut TcpStream) -> String {
    let mut raw = Vec::new();
    let mut byte = [0_u8; 1];
    while !raw.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).expect("interim answer");
        raw.push(byte[0]);
    }
    String::from_utf8_lossy(&raw).into_owned()
}

fn state_request(addr: SocketAddr) -> String {
    format!(
        "GET /v1/state HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: application/json\r\n\
         X-FocusMode-Extension: 1\r\nOrigin: {ORIGIN}\r\nCache-Control: no-cache\r\n\
         Sec-Fetch-Mode: cors\r\nConnection: keep-alive\r\n\r\n",
        addr.port()
    )
}

/// The head of an extension report; `framing` is its `Content-Length` / `Transfer-Encoding` line(s).
fn report_head(addr: SocketAddr, framing: &str) -> String {
    format!(
        "POST /v1/report HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\n\
         X-FocusMode-Extension: 1\r\nOrigin: {ORIGIN}\r\n{framing}\r\n\r\n",
        addr.port()
    )
}

fn report_request(addr: SocketAddr, body: &str) -> String {
    let head = report_head(addr, &format!("Content-Length: {}", body.len()));
    format!("{head}{body}")
}

fn assert_state_served(addr: SocketAddr) {
    let reply = send(addr, state_request(addr).as_bytes());
    assert_eq!(reply.status, 200, "{}", reply.head);
    let state: BridgeState = serde_json::from_slice(&reply.body).expect("state json");
    assert_eq!(state, FakeApp::sample());
}

#[test]
fn serves_the_extension_requests() {
    let (addr, app) = start(Limits::DEFAULT);
    let port = addr.port();

    let reply = send(addr, state_request(addr).as_bytes());
    assert_eq!(reply.status, 200, "{}", reply.head);
    assert_eq!(
        serde_json::from_slice::<BridgeState>(&reply.body).expect("state json"),
        FakeApp::sample()
    );
    assert_eq!(reply.header("Access-Control-Allow-Origin"), Some(ORIGIN));
    assert_eq!(reply.header("Vary"), Some("Origin"));
    assert_eq!(reply.header("Cache-Control"), Some("no-store"));
    assert_eq!(
        reply.header("Content-Type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(
        reply.header("Content-Length"),
        Some(reply.body.len().to_string().as_str())
    );
    assert_eq!(app.polls(), 1);

    // Read-only clients (curl, a web page) get the state, but no CORS and no "extension seen".
    let reply = send(
        addr,
        format!("GET /v1/state?t=1 HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").as_bytes(),
    );
    assert_eq!(reply.status, 200);
    assert_eq!(reply.header("Access-Control-Allow-Origin"), None);
    assert_eq!(app.polls(), 1);

    // Chrome's preflight for the report (custom header, private network).
    let reply = send(
        addr,
        format!(
            "OPTIONS /v1/report HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {ORIGIN}\r\n\
             Access-Control-Request-Method: POST\r\n\
             Access-Control-Request-Headers: content-type,x-focusmode-extension\r\n\
             Access-Control-Request-Private-Network: true\r\n\r\n"
        )
        .as_bytes(),
    );
    assert_eq!(reply.status, 204);
    assert!(reply.body.is_empty());
    assert_eq!(reply.header("Access-Control-Allow-Origin"), Some(ORIGIN));
    assert_eq!(
        reply.header("Access-Control-Allow-Methods"),
        Some("GET, POST, OPTIONS")
    );
    assert_eq!(
        reply.header("Access-Control-Allow-Headers"),
        Some("X-FocusMode-Extension, Content-Type")
    );
    assert_eq!(
        reply.header("Access-Control-Allow-Private-Network"),
        Some("true")
    );
    assert_eq!(reply.header("Access-Control-Max-Age"), Some("600"));
    // A web page's preflight gets no CORS headers, so its browser will not send the report.
    let reply = send(
        addr,
        format!(
            "OPTIONS /v1/report HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: https://evil.example\r\n\r\n"
        )
        .as_bytes(),
    );
    assert_eq!(reply.status, 204);
    assert_eq!(reply.header("Access-Control-Allow-Origin"), None);

    // The report itself.
    let reply = send(
        addr,
        report_request(addr, r#"{"skipped":["a\\d+","(?<=x)y"]}"#).as_bytes(),
    );
    assert_eq!(reply.status, 204, "{}", reply.head);
    assert!(reply.body.is_empty());
    assert_eq!(reply.header("Content-Length"), None);
    assert_eq!(reply.header("Access-Control-Allow-Origin"), Some(ORIGIN));
    assert_eq!(reply.header("Cache-Control"), Some("no-store"));
    assert_eq!(
        app.reports(),
        vec![vec![r"a\d+".to_owned(), "(?<=x)y".to_owned()]]
    );

    // The longest report the app can cause (every character escaped) is taken as well.
    let worst = serde_json::json!({
        "skipped": vec!["\u{1}".repeat(REPORT_MAX_ENTRY_CHARS); REPORT_MAX_ENTRIES]
    })
    .to_string();
    assert!(worst.len() <= REPORT_MAX_BYTES);
    let reply = send(addr, report_request(addr, &worst).as_bytes());
    assert_eq!(reply.status, 204, "{}", reply.head);
    assert_eq!(app.reports().last().map(Vec::len), Some(REPORT_MAX_ENTRIES));

    // `Expect: 100-continue` (not sent by Chrome, but cheap to honour).
    let body = r#"{"skipped":["z+"]}"#;
    let mut stream = connect(addr);
    let head = report_head(
        addr,
        &format!("Expect: 100-continue\r\nContent-Length: {}", body.len()),
    );
    stream.write_all(head.as_bytes()).expect("send head");
    assert!(read_interim(&mut stream).starts_with("HTTP/1.1 100 "));
    stream.write_all(body.as_bytes()).expect("send body");
    let reply = Reply::parse(&read_until_closed(&mut stream).expect("answer"));
    assert_eq!(reply.status, 204);
    assert_eq!(app.reports().last(), Some(&vec!["z+".to_owned()]));

    let refused = [
        // Not JSON / not the right JSON / over the entry limits: read, then refused.
        (report_request(addr, "{\"skipped\":"), 400, Some(ORIGIN)),
        (
            report_request(addr, r#"{"skipped":[1]}"#),
            400,
            Some(ORIGIN),
        ),
        (report_request(addr, ""), 400, Some(ORIGIN)),
        // No marker header, or a web origin: refused without waiting for the body.
        (
            report_head(addr, "Content-Length: 14").replace("X-FocusMode-Extension: 1\r\n", ""),
            403,
            Some(ORIGIN),
        ),
        (
            report_head(addr, "Content-Length: 14").replace(ORIGIN, "https://evil.example"),
            403,
            None,
        ),
        // DNS rebinding.
        (
            state_request(addr).replace("127.0.0.1", "evil.example"),
            403,
            None,
        ),
        // Two origins.
        (
            state_request(addr).replace("Origin:", "Origin: https://evil.example\r\nOrigin:"),
            400,
            None,
        ),
        (
            format!("GET /v1/other HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {ORIGIN}\r\n\r\n"),
            404,
            Some(ORIGIN),
        ),
        (
            format!("PUT /v1/report HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
            404,
            None,
        ),
    ];
    let taken = app.reports().len();
    for (request, status, cors) in refused {
        let reply = send(addr, request.as_bytes());
        assert_eq!(reply.status, status, "{request:?} -> {}", reply.head);
        assert_eq!(
            reply.header("Access-Control-Allow-Origin"),
            cors,
            "{request:?}"
        );
    }
    assert_eq!(app.reports().len(), taken);

    // `HEAD` has no route; the answer has no body.
    let reply = send(
        addr,
        format!("HEAD /v1/state HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes(),
    );
    assert_eq!(reply.status, 404);
    assert!(reply.body.is_empty());
}

#[test]
fn refuses_a_huge_declared_body_without_reading_it() {
    let (addr, app) = start(patient());

    for length in ["2147483648", "18446744073709551615"] {
        let started = Instant::now();
        let reply = send(
            addr,
            report_head(addr, &format!("Content-Length: {length}")).as_bytes(),
        );
        // Answered at once and closed: neither waiting for nor draining the 2 GB that never come.
        assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
        assert_eq!(reply.status, 413, "{}", reply.head);
        assert_eq!(reply.header("Access-Control-Allow-Origin"), Some(ORIGIN));
        assert_eq!(reply.header("Cache-Control"), Some("no-store"));
    }
    // A length that is not even a `u64` is malformed.
    let reply = send(
        addr,
        report_head(addr, "Content-Length: 99999999999999999999").as_bytes(),
    );
    assert_eq!(reply.status, 400);

    // A client that does stream its 2 GB: the bridge answers and closes without reading on.
    let mut stream = connect(addr);
    let mut writer = stream.try_clone().expect("clone");
    let started = Instant::now();
    let streaming = thread::spawn(move || {
        let head = report_head(addr, "Content-Length: 2147483648");
        let mut sent = writer.write_all(head.as_bytes()).map(|()| 0_usize);
        let chunk = vec![b'a'; 64 * 1024];
        while let Ok(total) = sent {
            if total >= 2_147_483_648 {
                break;
            }
            sent = writer.write_all(&chunk).map(|()| total + chunk.len());
        }
        sent.err()
    });
    if let Some(raw) = read_until_closed(&mut stream) {
        assert_eq!(Reply::parse(&raw).status, 413);
    }
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    // The writer stops because the connection is gone, long before 2 GB.
    assert!(streaming.join().expect("writer").is_some());

    assert!(app.reports().is_empty());
    assert_state_served(addr);
    let largest = LARGEST_ALLOCATION.load(Ordering::Relaxed);
    assert!(
        largest < ALLOCATION_ALARM,
        "largest allocation {largest} bytes"
    );
}

#[test]
fn refuses_chunked_reports_and_bodies_on_reads_unread() {
    let (addr, app) = start(patient());
    let port = addr.port();

    let started = Instant::now();
    let reply = send(
        addr,
        report_head(addr, "Transfer-Encoding: chunked").as_bytes(),
    );
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    assert_eq!(reply.status, 413, "{}", reply.head);
    assert_eq!(reply.header("Access-Control-Allow-Origin"), Some(ORIGIN));

    // Conflicting framing is malformed.
    let reply = send(
        addr,
        report_head(addr, "Transfer-Encoding: chunked\r\nContent-Length: 4").as_bytes(),
    );
    assert_eq!(reply.status, 400);
    let reply = send(
        addr,
        report_head(addr, "Content-Length: 4\r\nContent-Length: 4").as_bytes(),
    );
    assert_eq!(reply.status, 400);

    // A body on `GET` / `OPTIONS` is refused at once, without waiting for it.
    for (method, framing) in [
        ("GET /v1/state", "Content-Length: 5"),
        ("GET /v1/state", "Transfer-Encoding: chunked"),
        ("OPTIONS /v1/report", "Content-Length: 1048576"),
    ] {
        let started = Instant::now();
        let reply = send(
            addr,
            format!(
                "{method} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {ORIGIN}\r\n\
                 X-FocusMode-Extension: 1\r\n{framing}\r\n\r\n"
            )
            .as_bytes(),
        );
        assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
        assert_eq!(reply.status, 400, "{method} {framing}");
    }
    assert_eq!(app.polls(), 0);
    assert!(app.reports().is_empty());
    assert_state_served(addr);
}

#[test]
fn slow_heads_time_out_as_a_whole() {
    let head_timeout = Duration::from_millis(300);
    let (addr, _app) = start(Limits {
        head_timeout,
        ..Limits::DEFAULT
    });

    // Half a head, then nothing. (The deadline starts after `connect`, so `started` comes first.)
    let started = Instant::now();
    let mut stream = connect(addr);
    stream
        .write_all(b"GET /v1/state HTTP/1.1\r\nHost: 127.0.0.1\r\n")
        .expect("send");
    let reply = Reply::parse(&read_until_closed(&mut stream).expect("answer"));
    assert_eq!(reply.status, 408);
    assert!(started.elapsed() >= head_timeout);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());

    // A byte every 50 ms: each read is quick, but the head as a whole is late.
    let mut stream = connect(addr);
    let mut writer = stream.try_clone().expect("clone");
    let started = Instant::now();
    let trickle = thread::spawn(move || {
        let mut sent = writer.write_all(format!("{UNINSPECTED}X-Slow: ").as_bytes());
        let until = Instant::now() + Duration::from_secs(5);
        while sent.is_ok() && Instant::now() < until {
            thread::sleep(Duration::from_millis(50));
            sent = writer.write_all(b"a");
        }
    });
    if let Some(raw) = read_until_closed(&mut stream) {
        assert_eq!(Reply::parse(&raw).status, 408);
    }
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    trickle.join().expect("trickle");

    assert_state_served(addr);
}

#[test]
fn oversized_heads_are_refused() {
    let (addr, _app) = start(patient());
    let max = Limits::DEFAULT.max_head_bytes;

    // Exactly the limit without an end: everything sent was read, so the answer is not lost to a reset.
    let mut huge = format!("{UNINSPECTED}Host: 127.0.0.1:{}\r\nX-Big: ", addr.port());
    huge.push_str(&"a".repeat(max - huge.len()));
    assert_eq!(huge.len(), max);
    let started = Instant::now();
    let reply = send(addr, huge.as_bytes());
    assert_eq!(reply.status, 431);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());

    // Too many fields.
    let many = state_request(addr).replace(
        "\r\n\r\n",
        &format!(
            "{}\r\n\r\n",
            "\r\nX-Many: 1".repeat(Limits::DEFAULT.max_headers)
        ),
    );
    let reply = send(addr, many.as_bytes());
    assert_eq!(reply.status, 431);

    // A large head below the limits is fine.
    let large = state_request(addr).replace(
        "\r\n\r\n",
        &format!("\r\nX-Large: {}\r\n\r\n", "b".repeat(max / 2)),
    );
    let reply = send(addr, large.as_bytes());
    assert_eq!(reply.status, 200);
}

#[test]
fn stalled_bodies_time_out() {
    let body_timeout = Duration::from_millis(300);
    let (addr, app) = start(Limits {
        body_timeout,
        ..patient()
    });
    let started = Instant::now();
    let mut stream = connect(addr);
    let head = report_head(addr, "Content-Length: 100");
    stream
        .write_all(format!("{head}{{\"skipped\":").as_bytes())
        .expect("send");
    let reply = Reply::parse(&read_until_closed(&mut stream).expect("answer"));
    assert_eq!(reply.status, 408);
    assert_eq!(reply.header("Access-Control-Allow-Origin"), Some(ORIGIN));
    assert!(started.elapsed() >= body_timeout);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    assert!(app.reports().is_empty());
}

#[test]
fn bodies_are_read_one_at_a_time_when_limited() {
    let (addr, app) = start(Limits {
        max_body_readers: 1,
        ..patient()
    });
    let first = r#"{"skipped":["first"]}"#;
    let second = r#"{"skipped":["second"]}"#;

    // `100 Continue` comes once the first report holds the only reader permit.
    let mut holder = connect(addr);
    let head = report_head(
        addr,
        &format!("Expect: 100-continue\r\nContent-Length: {}", first.len()),
    );
    holder.write_all(head.as_bytes()).expect("send head");
    assert!(read_interim(&mut holder).starts_with("HTTP/1.1 100 "));

    // The second report waits for the permit...
    let mut waiting = connect(addr);
    waiting
        .write_all(report_request(addr, second).as_bytes())
        .expect("send");
    waiting
        .set_read_timeout(Some(Duration::from_millis(300)))
        .expect("read timeout");
    let mut byte = [0_u8; 1];
    let early = waiting.read(&mut byte);
    assert!(
        early
            .as_ref()
            .is_err_and(|err| matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)),
        "answered while the permit was taken: {early:?}"
    );

    // ...and is read once the first one is done.
    holder.write_all(first.as_bytes()).expect("send body");
    let reply = Reply::parse(&read_until_closed(&mut holder).expect("answer"));
    assert_eq!(reply.status, 204);
    waiting
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("read timeout");
    let reply = Reply::parse(&read_until_closed(&mut waiting).expect("answer"));
    assert_eq!(reply.status, 204);
    assert_eq!(
        app.reports(),
        vec![vec!["first".to_owned()], vec!["second".to_owned()]]
    );
}

#[test]
fn idle_connections_give_way_to_the_extension() {
    let connection_grace = Duration::from_millis(50);
    let (addr, app) = start(Limits {
        max_connections: 8,
        connection_grace,
        ..patient()
    });

    // A report being read is never closed to make room.
    let body = r#"{"skipped":["kept"]}"#;
    let mut reader = connect(addr);
    let head = report_head(
        addr,
        &format!("Expect: 100-continue\r\nContent-Length: {}", body.len()),
    );
    reader.write_all(head.as_bytes()).expect("send head");
    assert!(read_interim(&mut reader).starts_with("HTTP/1.1 100 "));

    // Far more idle connections than the limit, all within the 10 s head deadline and past their grace.
    let mut idle: Vec<TcpStream> = (0..30).map(|_| connect(addr)).collect();
    thread::sleep(connection_grace * 3);
    let started = Instant::now();
    assert_state_served(addr);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    assert_eq!(app.polls(), 1);

    // The oldest idle connections were closed for it.
    idle[0]
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("read timeout");
    assert!(read_until_closed(&mut idle[0]).is_none_or(|raw| raw.is_empty()));

    reader.write_all(body.as_bytes()).expect("send body");
    let reply = Reply::parse(&read_until_closed(&mut reader).expect("answer"));
    assert_eq!(reply.status, 204);
    assert_eq!(app.reports(), vec![vec!["kept".to_owned()]]);
}

#[test]
fn parallel_extension_requests_are_all_answered() {
    let (addr, app) = start(Limits::DEFAULT);
    // Idle clients next to the extension (under the connection limit, so none of them is closed).
    let _idle: Vec<TcpStream> = (0..40).map(|_| connect(addr)).collect();

    let clients: Vec<_> = (0..16)
        .map(|client| {
            thread::spawn(move || {
                for _ in 0..5 {
                    assert_state_served(addr);
                }
                let body = format!(r#"{{"skipped":["client{client}"]}}"#);
                send(addr, report_request(addr, &body).as_bytes()).status
            })
        })
        .collect();
    for client in clients {
        assert_eq!(client.join().expect("client"), 204);
    }
    assert_eq!(app.polls(), 16 * 5);
    assert_eq!(app.reports().len(), 16);
}

#[test]
fn a_slow_report_handler_keeps_its_reader_permit() {
    let (addr, app) = start(Limits {
        max_body_readers: 1,
        ..patient()
    });
    app.hold_reports(true);

    // The first report is read and handed to a handler that does not finish...
    let mut first = connect(addr);
    first
        .write_all(report_request(addr, r#"{"skipped":["first"]}"#).as_bytes())
        .expect("send");
    let until = Instant::now() + Duration::from_secs(10);
    while app.reports_entered.load(Ordering::SeqCst) == 0 {
        assert!(
            Instant::now() < until,
            "the first report never reached the handler"
        );
        thread::sleep(Duration::from_millis(5));
    }

    // ...so the second one is not read meanwhile (no `100 Continue`): bodies in memory stay bounded...
    let second = r#"{"skipped":["second"]}"#;
    let mut waiting = connect(addr);
    let head = report_head(
        addr,
        &format!("Expect: 100-continue\r\nContent-Length: {}", second.len()),
    );
    waiting.write_all(head.as_bytes()).expect("send head");
    waiting
        .set_read_timeout(Some(Duration::from_millis(300)))
        .expect("read timeout");
    let mut byte = [0_u8; 1];
    let early = waiting.read(&mut byte);
    assert!(
        early
            .as_ref()
            .is_err_and(|err| matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)),
        "read a second body while the first was still being handled: {early:?}"
    );
    // ...while the state is still served.
    assert_state_served(addr);

    app.hold_reports(false);
    let reply = Reply::parse(&read_until_closed(&mut first).expect("answer"));
    assert_eq!(reply.status, 204);
    waiting
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("read timeout");
    assert!(read_interim(&mut waiting).starts_with("HTTP/1.1 100 "));
    waiting.write_all(second.as_bytes()).expect("send body");
    let reply = Reply::parse(&read_until_closed(&mut waiting).expect("answer"));
    assert_eq!(reply.status, 204);
    assert_eq!(
        app.reports(),
        vec![vec!["first".to_owned()], vec!["second".to_owned()]]
    );
}

#[test]
fn a_connection_flood_does_not_starve_the_extension() {
    let max_connections = Limits::DEFAULT.max_connections;
    let (addr, app) = start(Limits {
        max_connections,
        ..patient()
    });

    // Clients that open connections as fast as they can and send nothing, keeping the newest ones open.
    let stop = Arc::new(AtomicBool::new(false));
    let flooders: Vec<_> = (0..4)
        .map(|_| {
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut held = VecDeque::new();
                let mut opened = 0_usize;
                // Bounded, so a fast machine does not run out of loopback ports.
                while !stop.load(Ordering::Relaxed) && opened < 1500 {
                    if let Ok(stream) = TcpStream::connect(addr) {
                        opened += 1;
                        held.push_back(stream);
                        if held.len() > 64 {
                            held.pop_front();
                        }
                    }
                }
                opened
            })
        })
        .collect();
    thread::sleep(Duration::from_millis(20));

    // Every extension request in the middle of it is answered, well before the extension gives up (2 s).
    for request in 0..50 {
        let started = Instant::now();
        let mut stream = connect(addr);
        stream
            .write_all(state_request(addr).as_bytes())
            .expect("send");
        let raw = read_until_closed(&mut stream).unwrap_or_default();
        assert!(
            !raw.is_empty(),
            "request {request} was closed unanswered to make room"
        );
        assert_eq!(Reply::parse(&raw).status, 200);
        assert!(
            started.elapsed() < Duration::from_millis(1500),
            "request {request} took {:?}",
            started.elapsed()
        );
    }
    stop.store(true, Ordering::Relaxed);
    let opened: usize = flooders
        .into_iter()
        .map(|flooder| flooder.join().expect("flooder"))
        .sum();
    // The flood did outnumber the connection limit, so connections were being closed to make room.
    assert!(opened > 2 * max_connections, "only {opened} connections");
    assert_eq!(app.polls(), 50);
}

#[test]
fn queued_connections_do_not_close_the_extension_unread() {
    // Connections that wait in the listen queue before the server takes any of them: 40 that send
    // nothing, the extension's request, then 60 more that send nothing. Taken in one burst, the 60 newer
    // ones would close the extension's connection (the limit is 16) before its request was read.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    let older: Vec<TcpStream> = (0..40).map(|_| connect(addr)).collect();
    let mut extension = connect(addr);
    extension
        .write_all(state_request(addr).as_bytes())
        .expect("send");
    let newer: Vec<TcpStream> = (0..60).map(|_| connect(addr)).collect();

    let app = serve(
        listener,
        Limits {
            max_connections: 16,
            ..patient()
        },
    );
    let raw = read_until_closed(&mut extension).unwrap_or_default();
    assert!(!raw.is_empty(), "closed unanswered to make room");
    assert_eq!(Reply::parse(&raw).status, 200);
    assert_eq!(app.polls(), 1);
    drop((older, newer));
}

#[test]
fn new_connections_do_not_close_a_young_one_before_its_request() {
    // The extension's connection waits in the listen queue ahead of 60 that send nothing, and its request
    // comes only after the server has taken them all (a client a little slow to write). Without the grace
    // the newer ones would have closed it (the limit is 16) before the request arrived.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("address");
    let mut extension = connect(addr);
    let newer: Vec<TcpStream> = (0..60).map(|_| connect(addr)).collect();

    let app = serve(
        listener,
        Limits {
            max_connections: 16,
            connection_grace: Duration::from_millis(500),
            ..patient()
        },
    );
    thread::sleep(Duration::from_millis(50));
    // A connection already closed by the server may refuse the write; the read below tells.
    let _ = extension.write_all(state_request(addr).as_bytes());
    let raw = read_until_closed(&mut extension).unwrap_or_default();
    assert!(!raw.is_empty(), "closed unanswered to make room");
    assert_eq!(Reply::parse(&raw).status, 200);
    assert_eq!(app.polls(), 1);
    drop(newer);
}

#[test]
fn new_connections_beyond_the_hard_limit_are_closed_at_once() {
    let connection_grace = Duration::from_millis(200);
    let (addr, app) = start(Limits {
        max_connections: 2,
        connection_grace,
        max_open: 4,
        ..patient()
    });

    // Four idle connections fill every place while they are within their grace...
    let held: Vec<TcpStream> = (0..4).map(|_| connect(addr)).collect();
    let started = Instant::now();
    // ...so a fifth one is closed unanswered, at once (not after the 10 s head deadline).
    let mut refused = connect(addr);
    refused
        .write_all(state_request(addr).as_bytes())
        .expect("send");
    assert!(read_until_closed(&mut refused).is_none_or(|raw| raw.is_empty()));
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    assert_eq!(app.polls(), 0);

    // Past their grace they give way again.
    thread::sleep(connection_grace * 2);
    assert_state_served(addr);
    assert_eq!(app.polls(), 1);
    drop(held);
}
