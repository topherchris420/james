//! The only network path out of James: HTTP/1.1 to a numeric loopback port.
//!
//! This crate exists so that James can say, and mean, "nothing leaves this
//! computer". There is no URL type, no hostname, no DNS, no TLS, no proxy and
//! no redirect following. The sole address input is a port number, and the
//! socket address is built from `Ipv4Addr::LOCALHOST`, falling back to
//! `Ipv6Addr::LOCALHOST` for servers that only listen on `::1`. A caller cannot
//! express a remote destination, so neither a model nor a configuration string
//! can redirect James off the machine.
//!
//! Owning the socket also gives James something a general HTTP library does
//! not: an [`AbortHandle`] that closes the connection from another thread, so
//! that pressing Stop ends a streaming reply on the wire instead of merely
//! ignoring whatever arrives afterwards.
//!
//! Scope, on purpose: one request and one response per connection
//! (`Connection: close`), `Content-Length`, chunked and until-close bodies,
//! bounded header and body sizes, and connect, idle and total timeouts.
//! Anything else is reported as [`Error::Unsupported`] rather than guessed at.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Why an exchange failed. Variants are deliberately coarse: callers turn them
/// into user-facing text that names the local server they were talking to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Nothing accepted the connection on either loopback address in time.
    Unreachable(String),
    /// A read or write exceeded the idle limit, or the exchange exceeded its total limit.
    TimedOut,
    /// The peer closed the connection before the message was complete.
    Closed,
    /// Headers or body exceeded a configured limit.
    TooLarge,
    /// The response violated HTTP/1.1 framing.
    Malformed(String),
    /// Valid HTTP that this deliberately small client does not implement.
    Unsupported(String),
    /// The request was rejected before anything was sent.
    InvalidRequest(String),
    /// Another I/O failure, including a reset by the peer or by [`AbortHandle::abort`].
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(detail) => write!(f, "cannot connect on loopback: {detail}"),
            Self::TimedOut => f.write_str("timed out waiting for the local server"),
            Self::Closed => {
                f.write_str("the local server closed the connection before the reply was complete")
            }
            Self::TooLarge => f.write_str("the response exceeded its size limit"),
            Self::Malformed(detail) => write!(f, "malformed HTTP response: {detail}"),
            Self::Unsupported(detail) => write!(f, "unsupported HTTP response: {detail}"),
            Self::InvalidRequest(detail) | Self::Io(detail) => f.write_str(detail),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => Self::TimedOut,
            io::ErrorKind::UnexpectedEof => Self::Closed,
            _ => Self::Io(error.to_string()),
        }
    }
}

/// Time limits for one exchange. `idle` bounds every individual read and the
/// request write; `total` bounds the whole exchange from connect onwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub idle: Duration,
    pub total: Option<Duration>,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(3),
            idle: Duration::from_secs(60),
            total: None,
        }
    }
}

/// Size limits. `headers` covers the status line, headers and trailers
/// together; `body` is a ceiling on decoded body bytes per response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub headers: usize,
    pub body: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            headers: 16 * 1024,
            body: 8 * 1024 * 1024,
        }
    }
}

/// A port on this computer. That is the whole address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Client {
    port: u16,
    timeouts: Timeouts,
    limits: Limits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    fn token(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

impl Client {
    pub fn new(port: u16) -> Result<Self, Error> {
        if port == 0 {
            return Err(Error::InvalidRequest(
                "a loopback port must be between 1 and 65535".into(),
            ));
        }
        Ok(Self {
            port,
            timeouts: Timeouts::default(),
            limits: Limits::default(),
        })
    }

    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn port(self) -> u16 {
        self.port
    }

    /// Starts describing one request. Nothing is sent until [`Request::send`].
    pub fn request<'a>(&self, method: Method, path: &'a str) -> Request<'a> {
        Request {
            client: *self,
            method,
            path,
            headers: Vec::new(),
            body: None,
            on_connect: None,
        }
    }

    pub fn get<'a>(&self, path: &'a str) -> Request<'a> {
        self.request(Method::Get, path)
    }

    pub fn post<'a>(&self, path: &'a str) -> Request<'a> {
        self.request(Method::Post, path)
    }
}

/// The two addresses this crate can ever connect to. Both are loopback by the
/// standard library's own definition; a test pins that down.
fn candidates(port: u16) -> [SocketAddr; 2] {
    [
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, port, 0, 0)),
    ]
}

fn host_header(address: SocketAddr) -> String {
    match address {
        SocketAddr::V4(v4) => format!("127.0.0.1:{}", v4.port()),
        SocketAddr::V6(v6) => format!("[::1]:{}", v6.port()),
    }
}

/// One request under construction.
pub struct Request<'a> {
    client: Client,
    method: Method,
    path: &'a str,
    headers: Vec<(&'a str, String)>,
    body: Option<(&'a str, &'a [u8])>,
    on_connect: Option<Box<dyn FnOnce(AbortHandle) + 'a>>,
}

impl<'a> Request<'a> {
    pub fn header(mut self, name: &'a str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// Attaches a body with its media type. Content-Length is computed here.
    pub fn body(mut self, content_type: &'a str, bytes: &'a [u8]) -> Self {
        self.body = Some((content_type, bytes));
        self
    }

    /// Receives the abort handle as soon as the connection exists, before the
    /// request is written, so a cancellation can interrupt even the wait for
    /// the first byte of the response.
    pub fn on_connect(mut self, callback: impl FnOnce(AbortHandle) + 'a) -> Self {
        self.on_connect = Some(Box::new(callback));
        self
    }

    /// Connects, writes the request and reads the status line and headers.
    /// The body is left on the socket for the caller to read in its own time.
    pub fn send(self) -> Result<Response, Error> {
        let started = Instant::now();
        validate_path(self.path)?;
        for (name, value) in &self.headers {
            validate_header(name, value)?;
        }
        if let Some((content_type, _)) = self.body {
            validate_header("Content-Type", content_type)?;
        }
        let timeouts = self.client.timeouts;
        let (stream, address) = connect(self.client.port, timeouts, started)?;
        let abort = AbortHandle {
            stream: Arc::new(stream.try_clone()?),
            aborted: Arc::new(AtomicBool::new(false)),
        };
        stream.set_nodelay(true)?;
        stream.set_write_timeout(Some(timeouts.idle.max(Duration::from_millis(1))))?;
        if let Some(callback) = self.on_connect {
            callback(abort.clone());
        }

        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: james-loopback/{}\r\n",
            self.method.token(),
            self.path,
            host_header(address),
            env!("CARGO_PKG_VERSION")
        );
        for (name, value) in &self.headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        if let Some((content_type, bytes)) = self.body {
            head.push_str("Content-Type: ");
            head.push_str(content_type);
            head.push_str("\r\nContent-Length: ");
            head.push_str(&bytes.len().to_string());
            head.push_str("\r\n");
        }
        head.push_str("\r\n");

        let mut stream = stream;
        stream.write_all(head.as_bytes())?;
        if let Some((_, bytes)) = self.body {
            stream.write_all(bytes)?;
        }
        stream.flush()?;

        let clock = Clock { started, timeouts };
        let mut reader = BufReader::new(stream);
        let limits = self.client.limits;
        let mut header_bytes = 0_usize;
        let (status, headers) = loop {
            let (status, headers) =
                read_head(&mut reader, &clock, limits.headers, &mut header_bytes, &abort)?;
            // A 1xx interim response carries no body; the real one follows.
            if (100..200).contains(&status) {
                continue;
            }
            break (status, headers);
        };
        let framing = framing_for(status, &headers)?;
        Ok(Response {
            status,
            headers,
            body: Body {
                inner: reader,
                abort,
                clock,
                framing,
                cap: limits.body,
                header_budget: limits.headers.saturating_sub(header_bytes),
                consumed: 0,
                pending: Vec::new(),
            },
        })
    }
}

fn validate_path(path: &str) -> Result<(), Error> {
    if !path.starts_with('/')
        || path.len() > 2048
        || !path.bytes().all(|b| (0x21..=0x7e).contains(&b))
    {
        return Err(Error::InvalidRequest(
            "request path must be visible ASCII starting with '/'".into(),
        ));
    }
    Ok(())
}

fn validate_header(name: &str, value: &str) -> Result<(), Error> {
    let name_ok = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    let value_ok = value
        .bytes()
        .all(|b| b == b'\t' || (0x20..=0x7e).contains(&b));
    if name_ok && value_ok {
        Ok(())
    } else {
        Err(Error::InvalidRequest(format!(
            "header {name:?} has an invalid name or value"
        )))
    }
}

fn connect(
    port: u16,
    timeouts: Timeouts,
    started: Instant,
) -> Result<(TcpStream, SocketAddr), Error> {
    // IPv4 and IPv6 share one connect budget. Giving each address the full
    // timeout can double the wait, particularly on Windows where an unbound
    // IPv6 loopback port may time out instead of refusing immediately.
    let mut failures = Vec::new();
    for (index, address) in candidates(port).into_iter().enumerate() {
        debug_assert!(address.ip().is_loopback());
        let elapsed = started.elapsed();
        if let Some(total) = timeouts.total {
            if elapsed >= total {
                return Err(Error::TimedOut);
            }
        }
        let mut wait = timeouts.connect.saturating_sub(elapsed);
        if let Some(total) = timeouts.total {
            wait = wait.min(total - elapsed);
        }
        // Reserve half the initial budget for IPv6, even if an unbound IPv4
        // loopback port consumes its entire allowance on Windows.
        if index == 0 {
            wait /= 2;
        }
        if wait.is_zero() {
            break;
        }
        match TcpStream::connect_timeout(&address, wait) {
            Ok(stream) => return Ok((stream, address)),
            Err(error) => failures.push(format!("{address}: {error}")),
        }
    }
    if timeouts
        .total
        .is_some_and(|total| started.elapsed() >= total)
    {
        return Err(Error::TimedOut);
    }
    // A connect timeout is still an unreachable local service, not a
    // response read timeout. Preserve the individual socket errors for
    // diagnostics instead of classifying only the first address's error.
    let detail = if failures.is_empty() {
        "connection deadline elapsed".to_owned()
    } else {
        failures.join("; ")
    };
    Err(Error::Unreachable(format!("port {port}: {detail}")))
}

/// Tracks the exchange deadline and arms the socket read timeout before each
/// read, so a stalled server fails after `idle` and a slow one after `total`.
#[derive(Debug, Clone, Copy)]
struct Clock {
    started: Instant,
    timeouts: Timeouts,
}

impl Clock {
    // Winsock does not reliably wake another thread's blocking read when a
    // cloned socket is shut down. Poll in small slices so the cancellation
    // flag is observed promptly, while retaining the original idle and
    // exchange-wide deadlines.
    fn arm(&self, stream: &TcpStream, last_progress: Instant) -> Result<(), Error> {
        let elapsed_idle = last_progress.elapsed();
        if elapsed_idle >= self.timeouts.idle {
            return Err(Error::TimedOut);
        }
        let mut wait = (self.timeouts.idle - elapsed_idle).min(Duration::from_millis(100));
        if let Some(total) = self.timeouts.total {
            let elapsed = self.started.elapsed();
            if elapsed >= total {
                return Err(Error::TimedOut);
            }
            wait = wait.min(total - elapsed);
        }
        stream.set_read_timeout(Some(wait.max(Duration::from_millis(1))))?;
        Ok(())
    }
}

/// Reads one line (without its line ending) from the socket, refusing lines
/// that would push the accumulated header size past `cap`. `None` means the
/// peer closed before another line began; a partial final line is returned so
/// that the parser, not the transport, decides what it was.
fn read_line(
    reader: &mut BufReader<TcpStream>,
    clock: &Clock,
    cap: usize,
    used: &mut usize,
    abort: &AbortHandle,
) -> Result<Option<Vec<u8>>, Error> {
    let mut line = Vec::new();
    let mut last_progress = Instant::now();
    loop {
        if abort.is_aborted() {
            return Err(Error::Closed);
        }
        clock.arm(reader.get_ref(), last_progress)?;
        let available = match reader.fill_buf() {
            Ok(bytes) => bytes,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(error) => {
                return Err(if abort.is_aborted() {
                    Error::Closed
                } else {
                    error.into()
                });
            }
        };
        if abort.is_aborted() {
            return Err(Error::Closed);
        }
        if available.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            *used += line.len();
            if line.ends_with(b"\r") {
                line.pop();
            }
            return Ok(Some(line));
        }
        let (take, done) = match available.iter().position(|&b| b == b'\n') {
            Some(index) => (index + 1, true),
            None => (available.len(), false),
        };
        if *used + line.len() + take > cap {
            return Err(Error::TooLarge);
        }
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        last_progress = Instant::now();
        if done {
            *used += line.len();
            if line.ends_with(b"\n") {
                line.pop();
            }
            if line.ends_with(b"\r") {
                line.pop();
            }
            return Ok(Some(line));
        }
    }
}

fn read_head(
    reader: &mut BufReader<TcpStream>,
    clock: &Clock,
    cap: usize,
    used: &mut usize,
    abort: &AbortHandle,
) -> Result<(u16, Vec<(String, String)>), Error> {
    let status_line = read_line(reader, clock, cap, used, abort)?.ok_or(Error::Closed)?;
    let status = parse_status_line(&status_line)?;
    let mut headers = Vec::new();
    loop {
        let line = read_line(reader, clock, cap, used, abort)?.ok_or(Error::Closed)?;
        if line.is_empty() {
            break;
        }
        headers.push(parse_header(&line)?);
    }
    Ok((status, headers))
}

fn parse_status_line(line: &[u8]) -> Result<u16, Error> {
    let text = std::str::from_utf8(line)
        .map_err(|_| Error::Malformed("status line is not ASCII".into()))?;
    let mut parts = text.split(' ');
    let version = parts.next().unwrap_or_default();
    let code = parts.next().unwrap_or_default();
    if !(version == "HTTP/1.1" || version == "HTTP/1.0") {
        return Err(Error::Malformed(format!(
            "expected an HTTP/1.x status line, got {:?}",
            text.chars().take(40).collect::<String>()
        )));
    }
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Malformed(format!("invalid status code {code:?}")));
    }
    code.parse()
        .map_err(|_| Error::Malformed(format!("invalid status code {code:?}")))
}

fn parse_header(line: &[u8]) -> Result<(String, String), Error> {
    let text = std::str::from_utf8(line)
        .map_err(|_| Error::Malformed("header line is not UTF-8".into()))?;
    let (name, value) = text
        .split_once(':')
        .ok_or_else(|| Error::Malformed("header line without a colon".into()))?;
    let name = name.trim();
    if name.is_empty() || name.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(Error::Malformed("header line with an invalid name".into()));
    }
    Ok((name.to_ascii_lowercase(), value.trim().to_owned()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Framing {
    Length(u64),
    Chunked(Chunk),
    UntilClose,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chunk {
    Size,
    Data(u64),
    DataEnd,
    Trailers,
}

fn header_value<'h>(headers: &'h [(String, String)], name: &str) -> Option<&'h str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn framing_for(status: u16, headers: &[(String, String)]) -> Result<Framing, Error> {
    if let Some(encoding) = header_value(headers, "content-encoding") {
        if !encoding.trim().eq_ignore_ascii_case("identity") {
            return Err(Error::Unsupported(format!(
                "Content-Encoding {encoding:?}; James never asks for compression"
            )));
        }
    }
    if matches!(status, 204 | 304) {
        return Ok(Framing::Done);
    }
    if let Some(encoding) = header_value(headers, "transfer-encoding") {
        let codings: Vec<&str> = encoding
            .split(',')
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .collect();
        if codings.len() == 1 && codings[0].eq_ignore_ascii_case("chunked") {
            return Ok(Framing::Chunked(Chunk::Size));
        }
        return Err(Error::Unsupported(format!(
            "Transfer-Encoding {encoding:?}"
        )));
    }
    let lengths: Vec<&str> = headers
        .iter()
        .filter(|(key, _)| key == "content-length")
        .map(|(_, value)| value.as_str())
        .collect();
    match lengths.first() {
        None => Ok(Framing::UntilClose),
        Some(first) => {
            if lengths.iter().any(|value| value != first) {
                return Err(Error::Malformed("conflicting Content-Length values".into()));
            }
            let length: u64 = first
                .trim()
                .parse()
                .map_err(|_| Error::Malformed(format!("invalid Content-Length {first:?}")))?;
            Ok(if length == 0 {
                Framing::Done
            } else {
                Framing::Length(length)
            })
        }
    }
}

fn parse_chunk_size(line: &[u8]) -> Result<u64, Error> {
    let text = std::str::from_utf8(line)
        .map_err(|_| Error::Malformed("chunk size is not ASCII".into()))?;
    let digits = text.split(';').next().unwrap_or_default().trim();
    if digits.is_empty() || digits.len() > 16 {
        return Err(Error::Malformed(format!("invalid chunk size {digits:?}")));
    }
    u64::from_str_radix(digits, 16)
        .map_err(|_| Error::Malformed(format!("invalid chunk size {digits:?}")))
}

/// Closes the underlying socket from any thread. A read blocked on the other
/// side returns at once, so the owning worker observes the cancellation
/// without waiting for a timeout. Whether the server stops computing is the
/// server's own behaviour; James only knows it closed the connection.
#[derive(Debug, Clone)]
pub struct AbortHandle {
    stream: Arc<TcpStream>,
    aborted: Arc<AtomicBool>,
}

impl AbortHandle {
    pub fn abort(&self) {
        // Windows can report a local shutdown as a successful EOF. Keep a
        // separate cancellation bit so EOF cannot masquerade as completion.
        self.aborted.store(true, Ordering::Release);
        let _ = self.stream.shutdown(Shutdown::Both);
    }

    fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::Acquire)
    }
}

/// Status line and headers of one response, with the unread body attached.
#[derive(Debug)]
pub struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Body,
}

impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }

    /// First value of a header, matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        header_value(&self.headers, &name.to_ascii_lowercase())
    }

    /// Lower-case media type without parameters, e.g. `text/event-stream`.
    pub fn media_type(&self) -> Option<String> {
        self.header("content-type")
            .and_then(|value| value.split(';').next())
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
    }

    pub fn abort_handle(&self) -> AbortHandle {
        self.body.abort.clone()
    }

    pub fn into_body(self) -> Body {
        self.body
    }

    /// Reads the whole body, refusing anything larger than `cap` as early as
    /// the headers allow.
    pub fn read_to_end(self, cap: usize) -> Result<Vec<u8>, Error> {
        self.body.read_to_end(cap)
    }
}

/// The body of one response, decoded from its transfer framing as it is read.
#[derive(Debug)]
pub struct Body {
    inner: BufReader<TcpStream>,
    abort: AbortHandle,
    clock: Clock,
    framing: Framing,
    cap: usize,
    header_budget: usize,
    consumed: usize,
    pending: Vec<u8>,
}

impl Body {
    pub fn abort_handle(&self) -> AbortHandle {
        self.abort.clone()
    }

    /// True once the framing says the server finished its message. A body
    /// that ends because the peer went away is not complete.
    pub fn is_complete(&self) -> bool {
        self.framing == Framing::Done && self.pending.is_empty()
    }

    /// Reads some decoded body bytes. `Ok(0)` means the message is complete;
    /// a peer that disappears mid-message yields [`Error::Closed`] instead.
    pub fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if self.abort.is_aborted() && self.framing != Framing::Done {
            return Err(Error::Closed);
        }
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            match self.framing {
                Framing::Done => return Ok(0),
                Framing::UntilClose => {
                    let read = self.read_socket(buf)?;
                    if read == 0 {
                        self.framing = Framing::Done;
                    }
                    return self.account(read);
                }
                Framing::Length(remaining) => {
                    let want = buf
                        .len()
                        .min(usize::try_from(remaining).unwrap_or(usize::MAX));
                    let read = self.read_socket(&mut buf[..want])?;
                    if read == 0 {
                        return Err(Error::Closed);
                    }
                    let left = remaining - read as u64;
                    self.framing = if left == 0 {
                        Framing::Done
                    } else {
                        Framing::Length(left)
                    };
                    return self.account(read);
                }
                Framing::Chunked(Chunk::Size) => {
                    let mut used = 0;
                    let line = read_line(&mut self.inner, &self.clock, 1024, &mut used, &self.abort)?
                        .ok_or(Error::Closed)?;
                    let size = parse_chunk_size(&line)?;
                    self.framing = Framing::Chunked(if size == 0 {
                        Chunk::Trailers
                    } else {
                        Chunk::Data(size)
                    });
                }
                Framing::Chunked(Chunk::Data(remaining)) => {
                    let want = buf
                        .len()
                        .min(usize::try_from(remaining).unwrap_or(usize::MAX));
                    let read = self.read_socket(&mut buf[..want])?;
                    if read == 0 {
                        return Err(Error::Closed);
                    }
                    let left = remaining - read as u64;
                    self.framing = Framing::Chunked(if left == 0 {
                        Chunk::DataEnd
                    } else {
                        Chunk::Data(left)
                    });
                    return self.account(read);
                }
                Framing::Chunked(Chunk::DataEnd) => {
                    let mut used = 0;
                    let line = match read_line(&mut self.inner, &self.clock, 16, &mut used, &self.abort) {
                        Ok(Some(line)) => line,
                        Ok(None) => return Err(Error::Closed),
                        Err(Error::TooLarge) => Vec::from(b"x"),
                        Err(error) => return Err(error),
                    };
                    if !line.is_empty() {
                        return Err(Error::Malformed("chunk data not followed by CRLF".into()));
                    }
                    self.framing = Framing::Chunked(Chunk::Size);
                }
                Framing::Chunked(Chunk::Trailers) => {
                    let mut used = 0;
                    loop {
                        let line =
                            read_line(&mut self.inner, &self.clock, self.header_budget, &mut used, &self.abort)?
                                .ok_or(Error::Closed)?;
                        if line.is_empty() {
                            break;
                        }
                        parse_header(&line)?;
                    }
                    self.header_budget = self.header_budget.saturating_sub(used);
                    self.framing = Framing::Done;
                }
            }
        }
    }

    fn read_socket(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let last_progress = Instant::now();
        loop {
            if self.abort.is_aborted() {
                return Err(Error::Closed);
            }
            self.clock.arm(self.inner.get_ref(), last_progress)?;
            match self.inner.read(buf) {
                Ok(read) => {
                    if self.abort.is_aborted() {
                        return Err(Error::Closed);
                    }
                    return Ok(read);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(error) => {
                    return Err(if self.abort.is_aborted() {
                        Error::Closed
                    } else {
                        error.into()
                    });
                }
            }
        }
    }

    fn account(&mut self, read: usize) -> Result<usize, Error> {
        self.consumed = self.consumed.saturating_add(read);
        if self.consumed > self.cap {
            return Err(Error::TooLarge);
        }
        Ok(read)
    }

    /// Reads the rest of the body, refusing more than `cap` decoded bytes. A
    /// declared `Content-Length` above the cap fails before any body is read.
    pub fn read_to_end(mut self, cap: usize) -> Result<Vec<u8>, Error> {
        let cap = cap.min(self.cap);
        if let Framing::Length(length) = self.framing {
            if length > cap as u64 {
                return Err(Error::TooLarge);
            }
        }
        let mut out = std::mem::take(&mut self.pending);
        let mut buf = [0_u8; 8192];
        loop {
            let read = self.read_chunk(&mut buf)?;
            if read == 0 {
                return Ok(out);
            }
            if out.len() + read > cap {
                return Err(Error::TooLarge);
            }
            out.extend_from_slice(&buf[..read]);
        }
    }

    /// Reads at most `max` bytes and stops, without treating a longer body as
    /// an error. Suitable for a bounded look at an error response.
    pub fn read_prefix(mut self, max: usize) -> Result<Vec<u8>, Error> {
        let mut out = std::mem::take(&mut self.pending);
        out.truncate(max);
        let mut buf = [0_u8; 4096];
        while out.len() < max {
            let read = match self.read_chunk(&mut buf) {
                Ok(0) => break,
                Ok(read) => read,
                Err(Error::TooLarge) => break,
                Err(error) => return Err(error),
            };
            let room = max - out.len();
            out.extend_from_slice(&buf[..read.min(room)]);
        }
        Ok(out)
    }

    /// Returns the next line without its line ending, or `None` once the body
    /// is complete. A final line without a newline is still returned. Lines
    /// longer than `cap` are refused.
    pub fn next_line(&mut self, cap: usize) -> Result<Option<Vec<u8>>, Error> {
        let mut buf = [0_u8; 8192];
        loop {
            if let Some(index) = self.pending.iter().position(|&b| b == b'\n') {
                if index > cap {
                    return Err(Error::TooLarge);
                }
                let mut line: Vec<u8> = self.pending.drain(..=index).collect();
                line.pop();
                if line.ends_with(b"\r") {
                    line.pop();
                }
                return Ok(Some(line));
            }
            if self.pending.len() > cap {
                return Err(Error::TooLarge);
            }
            let read = self.read_chunk(&mut buf)?;
            if read == 0 {
                if self.pending.is_empty() {
                    return Ok(None);
                }
                let mut line = std::mem::take(&mut self.pending);
                if line.ends_with(b"\r") {
                    line.pop();
                }
                return Ok(Some(line));
            }
            self.pending.extend_from_slice(&buf[..read]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_only_two_addresses_are_loopback() {
        for address in candidates(4321) {
            assert!(address.ip().is_loopback(), "{address}");
            assert_eq!(address.port(), 4321);
        }
        assert_eq!(host_header(candidates(80)[0]), "127.0.0.1:80");
        assert_eq!(host_header(candidates(80)[1]), "[::1]:80");
    }

    #[test]
    fn status_lines_and_headers_are_parsed_strictly() {
        assert_eq!(parse_status_line(b"HTTP/1.1 200 OK"), Ok(200));
        assert_eq!(parse_status_line(b"HTTP/1.0 404"), Ok(404));
        assert!(parse_status_line(b"HTTP/2 200 OK").is_err());
        assert!(parse_status_line(b"HTTP/1.1 20 OK").is_err());
        assert!(parse_status_line(b"<html>").is_err());
        assert_eq!(
            parse_header(b"Content-Type:  text/event-stream "),
            Ok(("content-type".into(), "text/event-stream".into()))
        );
        assert!(parse_header(b"no colon here").is_err());
        assert!(parse_header(b"bad name: x").is_err());
    }

    #[test]
    fn framing_follows_rfc_precedence_and_refuses_what_it_cannot_decode() {
        let headers = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        assert_eq!(framing_for(200, &headers(&[])), Ok(Framing::UntilClose));
        assert_eq!(
            framing_for(200, &headers(&[("content-length", "12")])),
            Ok(Framing::Length(12))
        );
        assert_eq!(
            framing_for(200, &headers(&[("content-length", "0")])),
            Ok(Framing::Done)
        );
        assert_eq!(framing_for(204, &headers(&[])), Ok(Framing::Done));
        assert_eq!(
            framing_for(
                200,
                &headers(&[("content-length", "5"), ("transfer-encoding", "Chunked")])
            ),
            Ok(Framing::Chunked(Chunk::Size))
        );
        assert!(matches!(
            framing_for(200, &headers(&[("transfer-encoding", "gzip, chunked")])),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            framing_for(200, &headers(&[("content-encoding", "gzip")])),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            framing_for(
                200,
                &headers(&[("content-length", "5"), ("content-length", "6")])
            ),
            Err(Error::Malformed(_))
        ));
        assert!(matches!(
            framing_for(200, &headers(&[("content-length", "-1")])),
            Err(Error::Malformed(_))
        ));
    }

    #[test]
    fn chunk_sizes_are_hex_with_optional_extensions() {
        assert_eq!(parse_chunk_size(b"1a"), Ok(26));
        assert_eq!(parse_chunk_size(b"1A;name=value"), Ok(26));
        assert_eq!(parse_chunk_size(b"0"), Ok(0));
        assert!(parse_chunk_size(b"").is_err());
        assert!(parse_chunk_size(b"zz").is_err());
        assert!(parse_chunk_size(b"ffffffffffffffffff").is_err());
    }

    #[test]
    fn requests_are_validated_before_any_socket_exists() {
        assert!(Client::new(0).is_err());
        let client = Client::new(1).unwrap();
        assert!(matches!(
            client.get("v1/models").send(),
            Err(Error::InvalidRequest(_))
        ));
        assert!(matches!(
            client.get("/v1/models?x= y").send(),
            Err(Error::InvalidRequest(_))
        ));
        assert!(matches!(
            client
                .get("/v1/models")
                .header("Accept", "a\r\nInjected: yes")
                .send(),
            Err(Error::InvalidRequest(_))
        ));
        assert!(matches!(
            client.get("/v1/models").header("Bad Name", "x").send(),
            Err(Error::InvalidRequest(_))
        ));
    }

    #[test]
    fn io_errors_map_to_timeouts_and_closures() {
        assert_eq!(
            Error::from(io::Error::from(io::ErrorKind::WouldBlock)),
            Error::TimedOut
        );
        assert_eq!(
            Error::from(io::Error::from(io::ErrorKind::TimedOut)),
            Error::TimedOut
        );
        assert_eq!(
            Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Error::Closed
        );
        assert!(matches!(
            Error::from(io::Error::from(io::ErrorKind::ConnectionReset)),
            Error::Io(_)
        ));
    }
}
