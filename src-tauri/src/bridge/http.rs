//! A minimal HTTP/1.1 server for the extension bridge: only what `GET /v1/state` and `POST /v1/report`
//! need, written so that a misbehaving local client can neither exhaust memory nor keep the bridge from
//! answering the extension (why it replaced `tiny_http`; ADR-018, "Utwardzenie mostu").
//!
//! - One request per connection: every answer carries `Connection: close` and the connection is closed
//!   after it. A body that was not read is never drained (closing may reset the connection instead).
//! - The head (request line and header fields) has at most [`Limits::max_head_bytes`] bytes and
//!   [`Limits::max_headers`] fields (`431` otherwise) and must arrive within [`Limits::head_timeout`] of
//!   the connection being accepted (`408`). It is a total deadline, so trickling bytes does not extend it.
//! - A body is read only when the [`Handler`] asks for one, and only with a declared `Content-Length` of
//!   at most its limit: a larger declared length or a `Transfer-Encoding` body is refused
//!   ([`BodyError::TooLarge`]) before any body byte is read or any buffer is sized from it. The body
//!   grows as bytes arrive and must arrive within [`Limits::body_timeout`]; at most
//!   [`Limits::max_body_readers`] bodies are read or handled at once. Other methods than `POST` may not
//!   carry a body (`400`), and conflicting or malformed framing headers are refused (`400`).
//! - Above [`Limits::max_connections`] open connections a new one closes the oldest ones, but only those
//!   open for at least [`Limits::connection_grace`] (time enough to send a request) and never one reading
//!   a body, so idle or slow clients cannot lock the extension out, and a flood of new connections cannot
//!   close the extension's before its request was read. While [`Limits::max_open`] connections are open
//!   (all within their grace), a new one is closed at once, so memory stays bounded. The runtime takes one
//!   new connection per turn, so the connections already taken run in between. Every step has a deadline,
//!   so each connection ends within a few seconds.
//! - Runs on its own single-threaded Tokio runtime; handler work that may block runs on its blocking pool.

use std::collections::BTreeMap;
use std::io;
use std::net::TcpListener as StdTcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio::task::AbortHandle;
use tokio::time::{Instant, timeout_at};

use crate::lock;

/// Pause after a failed `accept` (e.g. no free sockets), so a lasting error does not spin.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);
/// One read while looking for the end of the head.
const HEAD_CHUNK: usize = 1024;
/// First room for a body; it grows as bytes arrive, never from the declared length alone.
const BODY_CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    /// Request line and header fields together, without the empty line that ends them.
    pub max_head_bytes: usize,
    pub max_headers: usize,
    /// The whole head must arrive within this time after the connection was accepted.
    pub head_timeout: Duration,
    /// The whole body must arrive within this time after the head.
    pub body_timeout: Duration,
    /// The whole answer must be written within this time.
    pub write_timeout: Duration,
    /// Open connections; at the limit a new connection closes the oldest ones past their grace (never a
    /// body reader).
    pub max_connections: usize,
    /// How long after being accepted a connection is not closed to make room.
    pub connection_grace: Duration,
    /// Open connections at most, those within their grace included; at this count a new connection is
    /// closed at once. About 20 KiB each at most (head buffer and task).
    pub max_open: usize,
    /// Bodies read or being handled at the same time.
    pub max_body_readers: usize,
}

impl Limits {
    /// The extension sends about 1 KB of headers at once and gives up after 2 s (`FETCH_TIMEOUT_MS`).
    pub(super) const DEFAULT: Self = Self {
        max_head_bytes: 16 * 1024,
        max_headers: 48,
        head_timeout: Duration::from_secs(2),
        body_timeout: Duration::from_secs(5),
        write_timeout: Duration::from_secs(2),
        max_connections: 64,
        connection_grace: Duration::from_millis(100),
        max_open: 1024,
        max_body_readers: 2,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Method {
    Get,
    Head,
    Post,
    Options,
    Other,
}

/// Request line and header fields of a request.
#[derive(Debug)]
pub(super) struct Head {
    pub method: Method,
    /// Request target up to the query (`?`).
    pub path: String,
    http_1_1: bool,
    headers: Vec<(String, String)>,
}

impl Head {
    /// The first value of a header field; names compare case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// How many times a header field occurs.
    pub fn count(&self, name: &str) -> usize {
        self.headers
            .iter()
            .filter(|(field, _)| field.eq_ignore_ascii_case(name))
            .count()
    }
}

/// A complete answer; `Content-Length` and `Connection: close` are added when it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Response {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
}

impl Response {
    /// A plain-text answer (an empty body gets no `Content-Type`).
    pub fn text(status: u16, body: &str) -> Self {
        let response = Self {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        };
        if body.is_empty() {
            response
        } else {
            response.with_header("Content-Type", "text/plain; charset=utf-8")
        }
    }

    pub fn json(status: u16, body: String) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into_bytes(),
        }
        .with_header("Content-Type", "application/json; charset=utf-8")
    }

    /// Adds a header field; a value with a control character (e.g. a line break) is left out.
    pub fn with_header(mut self, name: &'static str, value: &str) -> Self {
        if value.bytes().all(is_field_byte) {
            self.headers.push((name, value.to_owned()));
        }
        self
    }

    #[cfg(test)]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn to_bytes(&self, head_only: bool) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status, reason(self.status));
        for (name, value) in &self.headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        // A 204 has neither a body nor a `Content-Length` (RFC 9110 §8.6).
        let has_body = self.status != 204;
        if has_body {
            head.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        }
        head.push_str("Connection: close\r\n\r\n");
        let mut bytes = head.into_bytes();
        if has_body && !head_only {
            bytes.extend_from_slice(&self.body);
        }
        bytes
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Content Too Large",
        417 => "Expectation Failed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "Unknown",
    }
}

/// What the server does with a request once its head was read.
pub(super) enum Next {
    Respond(Response),
    /// Read a body of at most `max_bytes` bytes, then answer with [`Handler::body`].
    ReadBody {
        max_bytes: usize,
    },
}

/// Why a body asked for with [`Next::ReadBody`] was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyError {
    /// Declared longer than allowed, or not declared up front (`Transfer-Encoding`). Nothing was read.
    TooLarge,
    /// The connection ended before the declared length arrived.
    Incomplete,
    /// The body did not arrive within [`Limits::body_timeout`].
    TimedOut,
    /// Other bodies were being read for the whole [`Limits::body_timeout`].
    Busy,
}

pub(super) trait Handler: Send + Sync + 'static {
    /// The answer to a request from its head, or [`Next::ReadBody`]. Runs on the server thread: keep it short.
    fn head(&self, head: &Head) -> Next;
    /// The answer to a request after [`Next::ReadBody`]. Runs on a blocking thread.
    fn body(&self, head: &Head, body: Result<Vec<u8>, BodyError>) -> Response;
}

/// Serves `listener` on the calling thread until the process ends. Fails only when the runtime cannot start.
pub(super) fn serve<H: Handler>(
    listener: StdTcpListener,
    handler: Arc<H>,
    limits: Limits,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    runtime.block_on(async move {
        let listener = TcpListener::from_std(listener)?;
        accept_loop(listener, handler, limits).await;
        Ok(())
    })
}

struct Shared<H> {
    handler: Arc<H>,
    limits: Limits,
    /// Connections that may be closed to make room, oldest first (by accept order).
    open: Mutex<BTreeMap<u64, Open>>,
    body_readers: Semaphore,
}

struct Open {
    task: AbortHandle,
    accepted: Instant,
}

impl<H> Shared<H> {
    /// Closes the oldest connections past their grace until a new one fits; false when the new one
    /// cannot be taken (every place up to [`Limits::max_open`] is held by a connection within its grace).
    fn make_room(&self, now: Instant) -> bool {
        let mut open = lock(&self.open);
        open.retain(|_, connection| !connection.task.is_finished());
        let mut closing = Vec::new();
        while open.len() >= self.limits.max_connections.max(1) {
            // Accept order is time order: when the oldest is still within its grace, all are.
            let Some(oldest) = open.first_entry() else {
                break;
            };
            if now.saturating_duration_since(oldest.get().accepted) < self.limits.connection_grace {
                break;
            }
            closing.push(oldest.remove().task);
        }
        let fits = open.len() < self.limits.max_open.max(self.limits.max_connections).max(1);
        drop(open);
        // The task ends (and its `Slot` leaves the list) the next time the runtime polls it.
        for task in closing {
            task.abort();
            log::debug!("extension bridge: too many connections, closed the oldest one");
        }
        fits
    }
}

async fn accept_loop<H: Handler>(listener: TcpListener, handler: Arc<H>, limits: Limits) {
    let shared = Arc::new(Shared {
        handler,
        limits,
        open: Mutex::new(BTreeMap::new()),
        body_readers: Semaphore::new(limits.max_body_readers),
    });
    let mut next_id: u64 = 0;
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _peer)) => stream,
            Err(err) => {
                log::debug!("extension bridge: accept failed: {err}");
                // A client that reset before it was accepted costs only its own connection (and can be
                // repeated at will, so it must not pause the bridge); anything else (e.g. no free sockets)
                // gets a short pause so a lasting error does not spin. Either way: keep listening.
                if !is_connection_error(&err) {
                    tokio::time::sleep(ACCEPT_BACKOFF).await;
                }
                continue;
            }
        };
        let accepted = Instant::now();
        if shared.make_room(accepted) {
            next_id = next_id.wrapping_add(1);
            // The task cannot run before it is listed: this runtime has one thread and nothing is
            // awaited in between. A task that has already finished is dropped by `make_room` anyway.
            let task = tokio::spawn(connection(Arc::clone(&shared), stream, next_id));
            lock(&shared.open).insert(
                next_id,
                Open {
                    task: task.abort_handle(),
                    accepted,
                },
            );
        } else {
            log::debug!("extension bridge: too many new connections, closed the newest one");
            drop(stream);
        }
        // One accept per turn of the runtime: the connections already taken run before the next one,
        // instead of a whole backlog being taken in one burst.
        tokio::task::yield_now().await;
    }
}

/// An `accept` error that concerns only the connection being accepted.
fn is_connection_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

/// A connection's place in [`Shared::open`]; leaves it when the connection ends or starts reading a body.
struct Slot<H> {
    shared: Arc<Shared<H>>,
    id: u64,
}

impl<H> Slot<H> {
    /// From now on the connection is not closed to make room.
    fn protect(&self) {
        lock(&self.shared.open).remove(&self.id);
    }
}

impl<H> Drop for Slot<H> {
    fn drop(&mut self) {
        self.protect();
    }
}

struct Answer {
    response: Response,
    /// `HEAD`: the head of the answer without its body.
    head_only: bool,
}

impl From<Response> for Answer {
    fn from(response: Response) -> Self {
        Self {
            response,
            head_only: false,
        }
    }
}

async fn connection<H: Handler>(shared: Arc<Shared<H>>, mut stream: TcpStream, id: u64) {
    let slot = Slot {
        shared: Arc::clone(&shared),
        id,
    };
    // Best effort: only delays small answers when it fails.
    let _ = stream.set_nodelay(true);
    let Some(answer) = exchange(&shared, &slot, &mut stream).await else {
        return;
    };
    let status = answer.response.status;
    let bytes = answer.response.to_bytes(answer.head_only);
    // Only the bytes on the wire stay while a slow client reads them (the state can be ~2 MB).
    drop(answer);
    let deadline = Instant::now() + shared.limits.write_timeout;
    let written = timeout_at(deadline, async {
        stream.write_all(&bytes).await?;
        stream.shutdown().await
    })
    .await;
    if !matches!(written, Ok(Ok(()))) {
        log::debug!("extension bridge: answer {status} not delivered");
    }
    // Dropping the stream closes the connection; a body that was not read is not drained.
}

/// Reads one request and decides the answer; `None` when the client left before sending a head.
async fn exchange<H: Handler>(
    shared: &Shared<H>,
    slot: &Slot<H>,
    stream: &mut TcpStream,
) -> Option<Answer> {
    let limits = &shared.limits;
    let deadline = Instant::now() + limits.head_timeout;
    let (raw, rest) = match timeout_at(deadline, read_head(stream, limits.max_head_bytes)).await {
        Ok(Ok(read)) => read,
        Ok(Err(HeadError::Closed)) => return None,
        Ok(Err(HeadError::TooLarge)) => return Some(too_large_head().into()),
        Err(_elapsed) => return Some(Response::text(408, "request timeout").into()),
    };
    let head = match parse_head(&raw, limits.max_headers) {
        Ok(head) => head,
        Err(refused) => return Some(refused.into()),
    };
    let head_only = head.method == Method::Head;
    let response = match framing(&head) {
        Err(refused) => refused,
        Ok(framing) => match shared.handler.head(&head) {
            Next::Respond(response) => response,
            Next::ReadBody { max_bytes } => {
                // The reader permit is held until the handler is done with the body, so at most
                // `max_body_readers` bodies are in memory at once, also when the handler is slow.
                let (body, _permit) =
                    read_body(shared, slot, stream, &head, framing, rest, max_bytes).await;
                let handler = Arc::clone(&shared.handler);
                tokio::task::spawn_blocking(move || handler.body(&head, body))
                    .await
                    .unwrap_or_else(|err| {
                        log::warn!("extension bridge: request handler failed: {err}");
                        Response::text(500, "internal error")
                    })
            }
        },
    };
    Some(Answer {
        response,
        head_only,
    })
}

fn too_large_head() -> Response {
    Response::text(431, "request header fields too large")
}

fn bad_request() -> Response {
    Response::text(400, "bad request")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadError {
    /// The connection ended (or failed) before the head was complete.
    Closed,
    /// No empty line within the size limit.
    TooLarge,
}

/// Reads up to the empty line that ends the head, never more than `max` bytes in total. Returns the
/// head without that line and whatever was read after it (the start of a body).
async fn read_head<R: AsyncRead + Unpin>(
    stream: &mut R,
    max: usize,
) -> Result<(Vec<u8>, Vec<u8>), HeadError> {
    let mut head = Vec::with_capacity(HEAD_CHUNK);
    let mut chunk = [0_u8; HEAD_CHUNK];
    loop {
        let room = max.saturating_sub(head.len()).min(HEAD_CHUNK);
        if room == 0 {
            return Err(HeadError::TooLarge);
        }
        let read = match stream.read(&mut chunk[..room]).await {
            Ok(0) | Err(_) => return Err(HeadError::Closed),
            Ok(read) => read,
        };
        let scan_from = head.len().saturating_sub(3);
        head.extend_from_slice(&chunk[..read]);
        if let Some(at) = head[scan_from..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            let end = scan_from + at;
            let rest = head.split_off(end + 4);
            head.truncate(end);
            return Ok((head, rest));
        }
    }
}

/// Request line and header fields; `400`, `431` (too many fields) or `505` when refused.
fn parse_head(raw: &[u8], max_headers: usize) -> Result<Head, Response> {
    let text = std::str::from_utf8(raw).map_err(|_| bad_request())?;
    let mut lines = text.split("\r\n");
    let mut parts = lines.next().unwrap_or_default().split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad_request());
    };
    if !is_token(method) || target.is_empty() || !target.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(bad_request());
    }
    let http_1_1 = match version {
        "HTTP/1.1" => true,
        "HTTP/1.0" => false,
        other if other.starts_with("HTTP/") => {
            return Err(Response::text(505, "http version not supported"));
        }
        _ => return Err(bad_request()),
    };
    let method = match method {
        "GET" => Method::Get,
        "HEAD" => Method::Head,
        "POST" => Method::Post,
        "OPTIONS" => Method::Options,
        _ => Method::Other,
    };

    let mut headers = Vec::new();
    for line in lines {
        if headers.len() >= max_headers {
            return Err(too_large_head());
        }
        // A line folded onto the previous one (leading space) has no token before its colon.
        let Some((name, value)) = line.split_once(':') else {
            return Err(bad_request());
        };
        let value = value.trim_matches(|c| c == ' ' || c == '\t');
        if !is_token(name) || !value.bytes().all(is_field_byte) {
            return Err(bad_request());
        }
        headers.push((name.to_owned(), value.to_owned()));
    }

    Ok(Head {
        method,
        path: target.split('?').next().unwrap_or_default().to_owned(),
        http_1_1,
        headers,
    })
}

/// RFC 9110 `token` (method and field names).
fn is_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

/// A byte allowed in a field value: no control characters except tab.
fn is_field_byte(byte: u8) -> bool {
    byte == b'\t' || (byte >= 0x20 && byte != 0x7f)
}

/// How the body of a request is delimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Framing {
    None,
    Length(u64),
    /// `Transfer-Encoding`: the length is not known up front.
    Unbounded,
}

/// The body framing, or the refusal of a request that cannot be framed safely: several `Host` or
/// `Content-Length` fields, a length that is not a plain number, both a length and a transfer coding,
/// a body on another method than `POST` (`400`), or an `Expect` other than `100-continue` (`417`).
fn framing(head: &Head) -> Result<Framing, Response> {
    let hosts = head.count("Host");
    if hosts > 1 || (hosts == 0 && head.http_1_1) {
        return Err(bad_request());
    }
    match head.header("Expect") {
        None => {}
        Some(value) if value.eq_ignore_ascii_case("100-continue") && head.count("Expect") == 1 => {}
        Some(_) => return Err(Response::text(417, "expectation failed")),
    }
    let chunked = head.count("Transfer-Encoding") > 0;
    let framing = match (head.count("Content-Length"), head.header("Content-Length")) {
        (0, _) if chunked => Framing::Unbounded,
        (0, _) => Framing::None,
        (1, Some(value)) if !chunked => {
            Framing::Length(parse_length(value).ok_or_else(bad_request)?)
        }
        _ => return Err(bad_request()),
    };
    let has_body = !matches!(framing, Framing::None | Framing::Length(0));
    if has_body && head.method != Method::Post {
        return Err(bad_request());
    }
    Ok(framing)
}

/// Digits only (no sign, no list); `None` also when it does not fit a `u64`.
fn parse_length(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// The body asked for by [`Next::ReadBody`]. Refuses a length over `max_bytes` (or an unknown one) before
/// reading anything; otherwise waits for a reader permit, sends `100 Continue` when the client expects it,
/// and reads exactly the declared length within [`Limits::body_timeout`]. Returns the permit (when one was
/// taken) for the caller to hold while the body is in use.
async fn read_body<'a, H>(
    shared: &'a Shared<H>,
    slot: &Slot<H>,
    stream: &mut TcpStream,
    head: &Head,
    framing: Framing,
    rest: Vec<u8>,
    max_bytes: usize,
) -> (Result<Vec<u8>, BodyError>, Option<SemaphorePermit<'a>>) {
    let length = match framing {
        Framing::None => return (Ok(Vec::new()), None),
        Framing::Unbounded => return (Err(BodyError::TooLarge), None),
        Framing::Length(length) => match usize::try_from(length) {
            Ok(length) if length <= max_bytes => length,
            _ => return (Err(BodyError::TooLarge), None),
        },
    };
    if length == 0 {
        return (Ok(Vec::new()), None);
    }
    let deadline = Instant::now() + shared.limits.body_timeout;
    let Ok(Ok(permit)) = timeout_at(deadline, shared.body_readers.acquire()).await else {
        return (Err(BodyError::Busy), None);
    };
    slot.protect();

    let mut body = rest;
    body.truncate(length);
    // Only `100-continue` gets this far (see `framing`).
    if body.len() < length && head.header("Expect").is_some() {
        let sent = timeout_at(deadline, stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")).await;
        if !matches!(sent, Ok(Ok(()))) {
            return (Err(BodyError::Incomplete), Some(permit));
        }
    }
    let missing = length - body.len();
    body.reserve(missing.min(BODY_CHUNK));
    let limit = u64::try_from(missing).unwrap_or(u64::MAX);
    let read = match timeout_at(deadline, (&mut *stream).take(limit).read_to_end(&mut body)).await {
        Err(_elapsed) => Err(BodyError::TimedOut),
        Ok(Ok(_)) if body.len() == length => Ok(body),
        Ok(_) => Err(BodyError::Incomplete),
    };
    (read, Some(permit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(raw: &str) -> Result<Head, u16> {
        parse_head(raw.as_bytes(), Limits::DEFAULT.max_headers).map_err(|refused| refused.status)
    }

    fn framed(raw: &str) -> Result<Framing, u16> {
        let head = head(raw)?;
        framing(&head).map_err(|refused| refused.status)
    }

    #[test]
    fn parses_the_extension_head() {
        let head = head(
            "GET /v1/state?x=1 HTTP/1.1\r\nHost: 127.0.0.1:47615\r\nOrigin:chrome-extension://abc\r\n\
             X-FocusMode-Extension:  1 \t",
        )
        .expect("head");
        assert_eq!(head.method, Method::Get);
        assert_eq!(head.path, "/v1/state");
        assert_eq!(head.header("host"), Some("127.0.0.1:47615"));
        assert_eq!(head.header("ORIGIN"), Some("chrome-extension://abc"));
        assert_eq!(head.header("x-focusmode-extension"), Some("1"));
        assert_eq!(head.count("Content-Length"), 0);
    }

    #[test]
    fn refuses_malformed_heads() {
        for raw in [
            "",
            "GET /v1/state",
            "GET  /v1/state HTTP/1.1",
            "GET /v1/state HTTP/1.1 extra",
            "GET /v1/st\u{7f}ate HTTP/1.1",
            "GET /v1/stąte HTTP/1.1",
            "G(T /v1/state HTTP/1.1",
            "GET /v1/state HTTX/1.1",
            "GET /v1/state HTTP/1.1\r\nNo colon",
            "GET /v1/state HTTP/1.1\r\nHost : 127.0.0.1",
            "GET /v1/state HTTP/1.1\r\nHost: 127.0.0.1\r\n folded",
            "GET /v1/state HTTP/1.1\r\nHost: 127.0.0.1\nX-Smuggled: 1",
            "GET /v1/state HTTP/1.1\r\nHost: 127.0.0.1\rX: 1",
        ] {
            assert_eq!(head(raw).map(|_| ()), Err(400), "{raw:?}");
        }
        assert_eq!(head("GET /v1/state HTTP/2.0").map(|_| ()), Err(505));
        let many = format!(
            "GET /v1/state HTTP/1.1{}",
            "\r\nX-A: 1".repeat(Limits::DEFAULT.max_headers + 1)
        );
        assert_eq!(head(&many).map(|_| ()), Err(431));
        assert!(head(&many.replacen("\r\nX-A: 1", "", 1)).is_ok());
    }

    #[test]
    fn frames_bodies_strictly() {
        let post = "POST /v1/report HTTP/1.1\r\nHost: 127.0.0.1";
        assert_eq!(framed(post), Ok(Framing::None));
        assert_eq!(
            framed(&format!("{post}\r\nContent-Length: 12")),
            Ok(Framing::Length(12))
        );
        assert_eq!(
            framed(&format!("{post}\r\nContent-Length: 18446744073709551615")),
            Ok(Framing::Length(u64::MAX))
        );
        assert_eq!(
            framed(&format!("{post}\r\nTransfer-Encoding: chunked")),
            Ok(Framing::Unbounded)
        );
        assert_eq!(
            framed(&format!("{post}\r\nTransfer-Encoding: gzip, chunked")),
            Ok(Framing::Unbounded)
        );
        for refused in [
            "Content-Length: 18446744073709551616",
            "Content-Length: -1",
            "Content-Length: +1",
            "Content-Length: 1e3",
            "Content-Length: 0x10",
            "Content-Length: 1, 1",
            "Content-Length: ",
            "Content-Length: 1\r\nContent-Length: 1",
            "Content-Length: 1\r\nTransfer-Encoding: chunked",
            "Host: 127.0.0.1",
        ] {
            assert_eq!(
                framed(&format!("{post}\r\n{refused}")),
                Err(400),
                "{refused:?}"
            );
        }
        assert_eq!(
            framed(&format!("{post}\r\nExpect: 100-Continue")),
            Ok(Framing::None)
        );
        assert_eq!(framed(&format!("{post}\r\nExpect: 200-ok")), Err(417));
        // HTTP/1.1 needs `Host`; HTTP/1.0 does not.
        assert_eq!(framed("GET /v1/state HTTP/1.1"), Err(400));
        assert_eq!(framed("GET /v1/state HTTP/1.0"), Ok(Framing::None));
        // Only `POST` carries a body.
        let get = "GET /v1/state HTTP/1.1\r\nHost: 127.0.0.1";
        assert_eq!(
            framed(&format!("{get}\r\nContent-Length: 0")),
            Ok(Framing::Length(0))
        );
        assert_eq!(framed(&format!("{get}\r\nContent-Length: 5")), Err(400));
        assert_eq!(
            framed(&format!("{get}\r\nTransfer-Encoding: chunked")),
            Err(400)
        );
        let options = "OPTIONS /v1/report HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2";
        assert_eq!(framed(options), Err(400));
    }

    #[test]
    fn writes_closing_answers() {
        let text = Response::text(404, "not found").with_header("Vary", "Origin");
        assert_eq!(
            String::from_utf8_lossy(&text.to_bytes(false)),
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain; charset=utf-8\r\nVary: Origin\r\n\
             Content-Length: 9\r\nConnection: close\r\n\r\nnot found"
        );
        assert_eq!(
            String::from_utf8_lossy(&text.to_bytes(true)),
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain; charset=utf-8\r\nVary: Origin\r\n\
             Content-Length: 9\r\nConnection: close\r\n\r\n"
        );
        assert_eq!(
            String::from_utf8_lossy(&Response::text(204, "").to_bytes(false)),
            "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n"
        );
        // A value that would split the head is left out.
        let split = Response::text(204, "").with_header("Access-Control-Allow-Origin", "a\r\nX: 1");
        assert_eq!(split.header("Access-Control-Allow-Origin"), None);
    }

    #[tokio::test]
    async fn reads_heads_within_the_limit() {
        let mut input: &[u8] = b"GET / HTTP/1.1\r\nHost: x\r\n\r\nbody";
        let (head, rest) = read_head(&mut input, 64).await.expect("head");
        assert_eq!(head, b"GET / HTTP/1.1\r\nHost: x");
        assert_eq!(rest, b"body");
        // Exactly at the limit without an end: refused after reading `max` bytes, never more.
        let long = vec![b'a'; 100];
        let mut input: &[u8] = &long;
        assert_eq!(read_head(&mut input, 64).await, Err(HeadError::TooLarge));
        assert_eq!(input.len(), 36);
        let mut input: &[u8] = b"GET / HTTP/1.1\r\n";
        assert_eq!(read_head(&mut input, 64).await, Err(HeadError::Closed));
    }
}
