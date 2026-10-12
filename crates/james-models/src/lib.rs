//! Local-only HTTP adapters for selected inference providers.
//!
//! The transport is `james-loopback`: a port number is the whole address, so
//! neither a model nor a configuration string can direct a request off this
//! machine. Replies stream as the server generates them. The caller receives
//! every piece of assistant text in order, plus an [`AbortHandle`] that ends
//! the transfer on the wire when the user presses Stop.
//!
//! Network locality does NOT prove that the inference backend itself is local:
//! a loopback server may relay to a cloud model. James reports the endpoint it
//! talked to; the server's own configuration decides the rest.

use base64::{Engine as _, engine::general_purpose::STANDARD};
pub use james_loopback::AbortHandle;
use james_loopback::{Client, Error as HttpError, Limits, Timeouts};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::time::Duration;

/// Discovery and non-streaming bodies.
const MAX_REPLY_BYTES: usize = 1_048_576;
/// Raw bytes of one streamed reply: JSON framing around at most 32 KiB of text.
const MAX_STREAM_BYTES: usize = 4 * 1_048_576;
/// One server-sent event or NDJSON line.
const MAX_FRAME_BYTES: usize = 262_144;
const MAX_MESSAGES: usize = 80;
const MAX_MESSAGE_BYTES: usize = 32_768;
const MAX_TOTAL_BYTES: usize = 262_144;
const MAX_ERROR_BYTES: usize = 4_096;
const MAX_ERROR_DETAIL_CHARS: usize = 300;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
/// How long James waits between two pieces of a reply, including before the
/// first one. Prompt processing on a CPU can take a long time, so the server
/// is only judged gone when it falls silent for this long; a long reply that
/// keeps arriving is never cut off by a total timeout.
pub const REPLY_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    InvalidConfig(String),
    Transport(String),
    Protocol(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(s) | Self::Transport(s) | Self::Protocol(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for ApiError {}

/// What a streaming request reports while it runs.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// The request is on the wire. [`AbortHandle::abort`] ends the transfer at
    /// once, including while James is still waiting for the first byte.
    Connected(AbortHandle),
    /// One piece of assistant text, in order. Concatenating every delta gives
    /// exactly the text the call returns.
    Delta(String),
}

/// User-selected API protocol. Other OpenAI-compatible servers use the same
/// bounded /v1 endpoints; they must still be configured on loopback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalProvider {
    #[default]
    LmStudio,
    Ollama,
    LlamaCpp,
    OpenAiCompatible,
}
impl LocalProvider {
    pub const ALL: [Self; 4] = [
        Self::LmStudio,
        Self::Ollama,
        Self::LlamaCpp,
        Self::OpenAiCompatible,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::LmStudio => "LM Studio",
            Self::Ollama => "Ollama",
            Self::LlamaCpp => "llama.cpp server",
            Self::OpenAiCompatible => "Other OpenAI-compatible server",
        }
    }
    pub fn default_port(self) -> u16 {
        match self {
            Self::LmStudio | Self::OpenAiCompatible => 1234,
            Self::Ollama => 11434,
            Self::LlamaCpp => 8080,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalClient {
    port: u16,
    provider: LocalProvider,
}

impl LocalClient {
    pub fn new(port: u16) -> Result<Self, ApiError> {
        Self::for_provider(LocalProvider::LmStudio, port)
    }

    pub fn for_provider(provider: LocalProvider, port: u16) -> Result<Self, ApiError> {
        if port == 0 {
            return Err(ApiError::InvalidConfig(
                "Choose a nonzero local server port".into(),
            ));
        }
        Ok(Self { port, provider })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn provider(&self) -> LocalProvider {
        self.provider
    }

    fn http(&self, timeouts: Timeouts, body_cap: usize) -> Result<Client, ApiError> {
        Client::new(self.port)
            .map(|client| {
                client.with_timeouts(timeouts).with_limits(Limits {
                    headers: 16 * 1024,
                    body: body_cap,
                })
            })
            .map_err(map_http_error)
    }

    pub fn models(&self) -> Result<Vec<String>, ApiError> {
        let ollama = self.provider == LocalProvider::Ollama;
        let path = if ollama { "/api/tags" } else { "/v1/models" };
        let response = self
            .http(
                Timeouts {
                    connect: CONNECT_TIMEOUT,
                    idle: DISCOVERY_TIMEOUT,
                    total: Some(DISCOVERY_TIMEOUT),
                },
                MAX_REPLY_BYTES,
            )?
            .get(path)
            .header("Accept", "application/json")
            .send()
            .map_err(map_http_error)?;
        let body = read_ok_body(response)?;
        if ollama {
            parse_ollama_models(&body)
        } else {
            parse_models(&body)
        }
    }

    /// Whole reply, assembled from the stream. Prefer [`Self::chat_stream`]
    /// when the user is watching.
    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String, ApiError> {
        self.chat_stream(model, messages, None, |_| {})
    }

    /// Attach one explicitly selected PNG to the final user turn only.
    /// Image bytes never enter ChatMessage or conversation persistence.
    pub fn chat_with_image(
        &self,
        model: &str,
        messages: &[ChatMessage],
        png: Option<&[u8]>,
    ) -> Result<String, ApiError> {
        self.chat_stream(model, messages, png, |_| {})
    }

    /// Streams one reply. `on_event` sees [`StreamEvent::Connected`] once,
    /// then every [`StreamEvent::Delta`] in order; the returned string is their
    /// concatenation. On an error the deltas already delivered are all the
    /// caller has, and they are an honest partial reply, not a complete one.
    ///
    /// A server that ignores `stream: true` and answers with one JSON document
    /// is accepted; its text arrives as a single delta.
    pub fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        png: Option<&[u8]>,
        mut on_event: impl FnMut(StreamEvent),
    ) -> Result<String, ApiError> {
        let ollama = self.provider == LocalProvider::Ollama;
        let request = if ollama {
            build_ollama_request(model, messages, png)?
        } else {
            build_request(model, messages, png)?
        };
        let path = if ollama {
            "/api/chat"
        } else {
            "/v1/chat/completions"
        };
        let response = self
            .http(
                Timeouts {
                    connect: CONNECT_TIMEOUT,
                    idle: REPLY_IDLE_TIMEOUT,
                    total: None,
                },
                MAX_STREAM_BYTES,
            )?
            .post(path)
            .header(
                "Accept",
                "text/event-stream, application/x-ndjson, application/json",
            )
            .body("application/json", request.as_bytes())
            .on_connect(|handle| on_event(StreamEvent::Connected(handle)))
            .send()
            .map_err(map_http_error)?;
        if response.status() != 200 {
            return Err(status_error(response));
        }
        let abort = response.abort_handle();
        let mut body = response.into_body();

        // Skip leading blank lines, then look at the first real one.
        let first = loop {
            match body.next_line(MAX_FRAME_BYTES).map_err(map_http_error)? {
                None => return Err(no_content(ollama)),
                Some(line) if line.iter().all(u8::is_ascii_whitespace) => continue,
                Some(line) => break utf8_line(line)?,
            }
        };
        if !ollama && first.trim_start().starts_with('{') {
            // Not an event stream: the server sent one JSON document instead.
            let mut whole = first.into_bytes();
            whole.push(b'\n');
            whole.extend(body.read_to_end(MAX_REPLY_BYTES).map_err(map_http_error)?);
            let whole = String::from_utf8(whole)
                .map_err(|_| ApiError::Protocol("Local model response was not UTF-8".into()))?;
            let content = parse_chat(&whole)?;
            on_event(StreamEvent::Delta(content.clone()));
            return Ok(content);
        }

        let mut content = String::new();
        let mut pending = Some(first);
        loop {
            let line = match pending.take() {
                Some(line) => line,
                None => match body.next_line(MAX_FRAME_BYTES) {
                    Ok(Some(line)) => utf8_line(line)?,
                    Ok(None) => break,
                    Err(error) => return Err(map_http_error(error)),
                },
            };
            let frame = if ollama {
                parse_ndjson_line(&line)?
            } else {
                parse_sse_line(&line)?
            };
            if let Some(delta) = frame.text {
                if content.len() + delta.len() > MAX_MESSAGE_BYTES {
                    abort.abort();
                    return Err(ApiError::Protocol(
                        "Assistant reply exceeds the 32 KiB conversation message limit".into(),
                    ));
                }
                if !delta.is_empty() {
                    content.push_str(&delta);
                    on_event(StreamEvent::Delta(delta));
                }
            }
            if frame.finished {
                break;
            }
        }
        if content.trim().is_empty() {
            return Err(no_content(ollama));
        }
        Ok(content)
    }
}

fn no_content(ollama: bool) -> ApiError {
    ApiError::Protocol(if ollama {
        "Ollama returned no assistant content".into()
    } else {
        "Chat response had no assistant message content".into()
    })
}

fn utf8_line(line: Vec<u8>) -> Result<String, ApiError> {
    String::from_utf8(line)
        .map_err(|_| ApiError::Protocol("Local model stream was not UTF-8".into()))
}

/// One decoded stream frame: possibly some text, possibly the end marker.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Frame {
    text: Option<String>,
    finished: bool,
}

/// One line of a `text/event-stream` body from an OpenAI-compatible server.
/// Comments and non-data fields carry nothing; `data: [DONE]` ends the reply.
fn parse_sse_line(line: &str) -> Result<Frame, ApiError> {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with(':') {
        return Ok(Frame::default());
    }
    let Some((field, value)) = line.split_once(':') else {
        return Err(ApiError::Protocol(
            "Malformed frame in the local model stream".into(),
        ));
    };
    if field != "data" {
        return Ok(Frame::default());
    }
    let payload = value.trim();
    if payload == "[DONE]" {
        return Ok(Frame {
            text: None,
            finished: true,
        });
    }
    let value: Value = serde_json::from_str(payload)
        .map_err(|_| ApiError::Protocol("Malformed JSON in the local model stream".into()))?;
    if let Some(detail) = server_error_detail(payload) {
        if value.get("error").is_some() {
            return Err(ApiError::Protocol(format!(
                "Local model server reported: {detail}"
            )));
        }
    }
    Ok(Frame {
        text: value
            .pointer("/choices/0/delta/content")
            .and_then(Value::as_str)
            .map(str::to_owned),
        finished: false,
    })
}

/// One line of Ollama's native `/api/chat` stream. The final object carries
/// `done: true`; a non-streaming answer is the same shape on a single line.
fn parse_ndjson_line(line: &str) -> Result<Frame, ApiError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(Frame::default());
    }
    let value: Value = serde_json::from_str(line)
        .map_err(|_| ApiError::Protocol("Malformed JSON in the Ollama stream".into()))?;
    if value.get("error").is_some() {
        let detail = server_error_detail(line).unwrap_or_else(|| "unknown error".into());
        return Err(ApiError::Protocol(format!(
            "Local model server reported: {detail}"
        )));
    }
    Ok(Frame {
        text: value
            .pointer("/message/content")
            .and_then(Value::as_str)
            .map(str::to_owned),
        finished: value.get("done").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn build_request(
    model: &str,
    messages: &[ChatMessage],
    png: Option<&[u8]>,
) -> Result<String, ApiError> {
    validate_request(model, messages)?;
    let mut wire =
        serde_json::to_value(messages).map_err(|err| ApiError::Protocol(err.to_string()))?;
    if let Some(bytes) = png {
        if bytes.len() > 16 * 1024 * 1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(ApiError::InvalidConfig(
                "Screenshot must be a PNG of at most 16 MiB".into(),
            ));
        }
        let last = messages.last().expect("validated nonempty conversation");
        if last.role != "user" {
            return Err(ApiError::InvalidConfig(
                "A screenshot requires a final user question".into(),
            ));
        }
        wire[messages.len() - 1]["content"] = serde_json::json!([
            {"type":"text", "text":last.content},
            {"type":"image_url", "image_url":{"url":format!("data:image/png;base64,{}", STANDARD.encode(bytes))}}
        ]);
    }
    serde_json::to_string(&serde_json::json!({"model":model,"messages":wire,"stream":true}))
        .map_err(|err| ApiError::Protocol(format!("Cannot serialize request: {err}")))
}

fn build_ollama_request(
    model: &str,
    messages: &[ChatMessage],
    png: Option<&[u8]>,
) -> Result<String, ApiError> {
    validate_request(model, messages)?;
    let mut wire =
        serde_json::to_value(messages).map_err(|err| ApiError::Protocol(err.to_string()))?;
    if let Some(bytes) = png {
        if bytes.len() > 16 * 1024 * 1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(ApiError::InvalidConfig(
                "Screenshot must be a PNG of at most 16 MiB".into(),
            ));
        }
        if messages.last().is_none_or(|last| last.role != "user") {
            return Err(ApiError::InvalidConfig(
                "A screenshot requires a final user question".into(),
            ));
        }
        wire[messages.len() - 1]["images"] = serde_json::json!([STANDARD.encode(bytes)]);
    }
    serde_json::to_string(&serde_json::json!({
        "model": model, "messages": wire, "stream": true
    }))
    .map_err(|err| ApiError::Protocol(format!("Cannot serialize request: {err}")))
}

fn map_http_error(error: HttpError) -> ApiError {
    match error {
        HttpError::Unreachable(detail) => ApiError::Transport(format!(
            "Cannot reach the selected local model server: {detail}"
        )),
        HttpError::TimedOut => ApiError::Transport(format!(
            "The local model server went silent for {} seconds. It may be busy with a long prompt; try again or choose a smaller model.",
            REPLY_IDLE_TIMEOUT.as_secs()
        )),
        HttpError::Closed => ApiError::Transport(
            "The local model server closed the connection before the reply was complete".into(),
        ),
        HttpError::TooLarge => {
            ApiError::Protocol("Local model response exceeded its size limit".into())
        }
        HttpError::Malformed(detail) | HttpError::Unsupported(detail) => ApiError::Protocol(
            format!("Unexpected response from the local model server: {detail}"),
        ),
        HttpError::InvalidRequest(detail) => ApiError::InvalidConfig(detail),
        HttpError::Io(detail) => ApiError::Transport(format!(
            "Connection to the local model server failed: {detail}"
        )),
    }
}

/// Turns a non-200 response into an error that quotes the server's own
/// explanation (for example, a model without image support) as data.
fn status_error(response: james_loopback::Response) -> ApiError {
    let code = response.status();
    let body = response
        .into_body()
        .read_prefix(MAX_ERROR_BYTES)
        .unwrap_or_default();
    match server_error_detail(&String::from_utf8_lossy(&body)) {
        Some(detail) => {
            ApiError::Transport(format!("Local model server returned HTTP {code}: {detail}"))
        }
        None => ApiError::Transport(format!(
            "Local model server returned HTTP {code}. Check the selected model and provider."
        )),
    }
}

fn read_ok_body(response: james_loopback::Response) -> Result<String, ApiError> {
    if response.status() != 200 {
        return Err(status_error(response));
    }
    let bytes = response
        .read_to_end(MAX_REPLY_BYTES)
        .map_err(|error| match error {
            HttpError::TooLarge => {
                ApiError::Protocol("Local model response exceeded 1 MiB limit".into())
            }
            other => map_http_error(other),
        })?;
    String::from_utf8(bytes)
        .map_err(|_| ApiError::Protocol("Local model response was not UTF-8".into()))
}

/// Extracts a short, printable message from an OpenAI-style, Ollama or LM Studio
/// error body. LM Studio may wrap the engine's JSON error inside the outer string.
fn server_error_detail(body: &str) -> Option<String> {
    fn message(value: &Value) -> Option<&str> {
        let error = value.get("error")?;
        error
            .as_str()
            .or_else(|| error.get("message").and_then(Value::as_str))
    }
    let mut detail = message(&serde_json::from_str(body).ok()?)?.to_owned();
    if let Some(inner) = detail
        .find('{')
        .and_then(|start| serde_json::from_str::<Value>(&detail[start..]).ok())
        .and_then(|value| message(&value).map(str::to_owned))
    {
        detail = inner;
    }
    let detail: String = detail
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_ERROR_DETAIL_CHARS)
        .collect();
    let detail = detail.trim();
    (!detail.is_empty()).then(|| detail.to_owned())
}

fn validate_request(model: &str, messages: &[ChatMessage]) -> Result<(), ApiError> {
    if model.trim().is_empty() || model.len() > 256 {
        return Err(ApiError::InvalidConfig("Select a valid model ID".into()));
    }
    if messages.is_empty() || messages.len() > MAX_MESSAGES {
        return Err(ApiError::InvalidConfig(
            "Conversation length is outside the allowed range".into(),
        ));
    }
    let mut total_bytes = 0_usize;
    for message in messages {
        if !matches!(message.role.as_str(), "system" | "user" | "assistant")
            || message.content.len() > MAX_MESSAGE_BYTES
        {
            return Err(ApiError::InvalidConfig(
                "Invalid role or oversized message".into(),
            ));
        }
        total_bytes = total_bytes.saturating_add(message.content.len());
    }
    if total_bytes > MAX_TOTAL_BYTES {
        return Err(ApiError::InvalidConfig(
            "Conversation context exceeds 256 KiB".into(),
        ));
    }
    Ok(())
}

pub fn parse_models(body: &str) -> Result<Vec<String>, ApiError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| ApiError::Protocol("Malformed JSON in /v1/models".into()))?;
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::Protocol("Model list did not contain a data array".into()))?;
    let mut models: Vec<String> = rows
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .map(str::to_owned)
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}

/// Ollama's native model discovery returns models[].name, not data[].id.
pub fn parse_ollama_models(body: &str) -> Result<Vec<String>, ApiError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| ApiError::Protocol("Malformed JSON in /api/tags".into()))?;
    let rows = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::Protocol("Ollama model list did not contain models".into()))?;
    let mut models: Vec<String> = rows
        .iter()
        .filter_map(|item| item.get("name").and_then(Value::as_str))
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .map(str::to_owned)
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}

/// Native Ollama /api/chat result. Tool-only responses are not misrepresented
/// as visible answers or executable actions.
pub fn parse_ollama_chat(body: &str) -> Result<String, ApiError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| ApiError::Protocol("Malformed JSON in Ollama chat response".into()))?;
    let content = value
        .pointer("/message/content")
        .and_then(Value::as_str)
        .filter(|answer| !answer.trim().is_empty())
        .ok_or_else(|| ApiError::Protocol("Ollama returned no assistant content".into()))?;
    if content.len() > MAX_MESSAGE_BYTES {
        return Err(ApiError::Protocol(
            "Assistant reply exceeds the 32 KiB conversation message limit".into(),
        ));
    }
    Ok(content.to_owned())
}

/// A complete (non-streamed) OpenAI-compatible chat completion.
pub fn parse_chat(body: &str) -> Result<String, ApiError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| ApiError::Protocol("Malformed JSON in chat response".into()))?;
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .filter(|answer| !answer.trim().is_empty())
        .ok_or_else(|| {
            ApiError::Protocol("Chat response had no assistant message content".into())
        })?;
    if content.len() > MAX_MESSAGE_BYTES {
        return Err(ApiError::Protocol(
            "Assistant reply exceeds the 32 KiB conversation message limit".into(),
        ));
    }
    Ok(content.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    enum Step {
        Write(Vec<u8>),
        Sleep(Duration),
    }

    /// A scripted server on a random loopback port. It reads the whole request
    /// (so closing never resets the client's send), then plays `steps` and
    /// returns the raw request for inspection.
    fn serve(steps: Vec<Step>) -> (u16, thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut request = Vec::new();
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(
                line.starts_with("GET /v1/models ")
                    || line.starts_with("POST /v1/chat/completions ")
                    || line.starts_with("GET /api/tags ")
                    || line.starts_with("POST /api/chat "),
                "{line}"
            );
            request.extend_from_slice(line.as_bytes());
            let mut body_size = 0_usize;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                request.extend_from_slice(line.as_bytes());
                if line == "\r\n" || line == "\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        body_size = value.trim().parse().unwrap();
                    }
                }
            }
            if body_size > 0 {
                assert!(body_size <= 262_144 + 20 * 1024 * 1024);
                let mut request_body = vec![0_u8; body_size];
                reader.read_exact(&mut request_body).unwrap();
                request.extend_from_slice(&request_body);
            }
            for step in steps {
                match step {
                    Step::Write(bytes) => {
                        if socket.write_all(&bytes).is_err() {
                            break;
                        }
                    }
                    Step::Sleep(duration) => thread::sleep(duration),
                }
            }
            let _ = socket.flush();
            request
        });
        (port, worker)
    }

    fn mock(
        body: &'static str,
        status: &'static str,
    ) -> (LocalClient, thread::JoinHandle<Vec<u8>>) {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (port, worker) = serve(vec![Step::Write(response.into_bytes())]);
        (LocalClient::new(port).unwrap(), worker)
    }

    fn chunk(data: &[u8]) -> Vec<u8> {
        let mut out = format!("{:x}\r\n", data.len()).into_bytes();
        out.extend_from_slice(data);
        out.extend_from_slice(b"\r\n");
        out
    }

    const SSE_HEAD: &[u8] =
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
    const NDJSON_HEAD: &[u8] =
        b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";

    fn sse_delta(text: &str) -> String {
        format!(
            "data: {}\n\n",
            serde_json::json!({"id":"x","choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]})
        )
    }

    /// Collects stream events; the abort handle is kept for cancellation tests.
    fn collect(
        client: &LocalClient,
        model: &str,
        messages: &[ChatMessage],
    ) -> (Result<String, ApiError>, Vec<String>, Option<AbortHandle>) {
        let mut deltas = Vec::new();
        let mut handle = None;
        let result = client.chat_stream(model, messages, None, |event| match event {
            StreamEvent::Connected(abort) => {
                assert!(deltas.is_empty(), "Connected must come first");
                handle = Some(abort);
            }
            StreamEvent::Delta(text) => deltas.push(text),
        });
        (result, deltas, handle)
    }

    #[test]
    fn vision_attaches_only_to_last_user_turn_and_preserves_text_history() {
        let messages = vec![
            ChatMessage::new("system", "Context"),
            ChatMessage::new("user", "What is here?"),
        ];
        let wire: Value = serde_json::from_str(
            &build_request("vision", &messages, Some(b"\x89PNG\r\n\x1a\n")).unwrap(),
        )
        .unwrap();
        assert_eq!(wire["messages"][0]["content"], "Context");
        assert_eq!(wire["messages"][1]["content"][0]["text"], "What is here?");
        assert_eq!(
            wire["messages"][1]["content"][1]["image_url"]["url"],
            "data:image/png;base64,iVBORw0KGgo="
        );
        assert_eq!(wire["stream"], true);
        assert_eq!(messages[1].content, "What is here?");
        assert!(build_request("vision", &messages, Some(b"not png")).is_err());
        assert!(
            build_request(
                "vision",
                &[ChatMessage::new("assistant", "Hi")],
                Some(b"\x89PNG\r\n\x1a\n")
            )
            .is_err()
        );
    }

    #[test]
    fn loopback_client_refuses_redirects() {
        let (port, worker) = serve(vec![Step::Write(
            b"HTTP/1.1 302 Found\r\nLocation: http://192.0.2.1/v1/models\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        )]);
        let client = LocalClient::new(port).unwrap();
        let error = client.models().unwrap_err().to_string();
        assert!(error.contains("HTTP 302"), "{error}");
        worker.join().unwrap();
    }

    #[test]
    fn rejects_zero_port_and_invalid_payloads() {
        assert!(LocalClient::new(0).is_err());
        assert!(validate_request("", &[ChatMessage::new("user", "Hi")]).is_err());
        assert!(validate_request("qwen", &[ChatMessage::new("tool", "Hi")]).is_err());
        assert!(validate_request("qwen", &[ChatMessage::new("user", "x".repeat(32_769))]).is_err());
    }

    #[test]
    fn parses_models_with_deterministic_order_and_deduplication() {
        let items =
            parse_models(r#"{"data":[{"id":"z"},{"id":"a"},{"id":"a"},{"other":1}]}"#).unwrap();
        assert_eq!(items, vec!["a", "z"]);
        assert!(parse_models(r#"{"hello":true}"#).is_err());
    }

    #[test]
    fn parses_chat_and_fails_on_missing_content() {
        assert_eq!(
            parse_chat(r#"{"choices":[{"message":{"content":"Evidence remains tentative."}}]}"#)
                .unwrap(),
            "Evidence remains tentative."
        );
        assert!(parse_chat(r#"{"choices":[]}"#).is_err());
    }

    #[test]
    fn localhost_model_discovery_uses_expected_path() {
        let (client, worker) = mock(r#"{"data":[{"id":"qwen-local"}]}"#, "200 OK");
        assert_eq!(client.models().unwrap(), vec!["qwen-local"]);
        let request = String::from_utf8(worker.join().unwrap()).unwrap();
        assert!(request.starts_with("GET /v1/models HTTP/1.1\r\n"));
        assert!(request.contains("\r\nAccept: application/json\r\n"));
    }

    #[test]
    fn a_server_that_ignores_streaming_is_still_understood() {
        let (client, worker) = mock(
            r#"{"choices":[{"message":{"role":"assistant","content":"A testable idea."}}]}"#,
            "200 OK",
        );
        let (result, deltas, handle) = collect(
            &client,
            "qwen-local",
            &[ChatMessage::new("user", "Propose a test")],
        );
        assert_eq!(result.unwrap(), "A testable idea.");
        assert_eq!(deltas, vec!["A testable idea."]);
        assert!(handle.is_some());
        let request = String::from_utf8(worker.join().unwrap()).unwrap();
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
        assert!(
            request.contains(
                "\r\nAccept: text/event-stream, application/x-ndjson, application/json\r\n"
            )
        );
        assert!(request.ends_with(r#""stream":true}"#), "{request}");
    }

    #[test]
    fn sse_deltas_arrive_in_order_across_fragmented_chunks() {
        let mut stream = SSE_HEAD.to_vec();
        // A comment, a role-only first delta and an event id carry no text.
        stream.extend(chunk(b": keep-alive\n\n"));
        stream.extend(chunk(
            b"data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\nid: 7\n",
        ));
        let frames = format!(
            "{}{}{}",
            sse_delta("Evidence "),
            sse_delta("comes first, caf\u{e9}"),
            sse_delta(".")
        );
        // Split frames at awkward places: inside "data:", inside the JSON and
        // between the two bytes of the UTF-8 "é".
        let bytes = frames.as_bytes();
        let cut_inside_e_acute = frames.find("caf\u{e9}").unwrap() + 4;
        let mut steps = vec![Step::Write(stream)];
        let cuts = [
            3_usize,
            20,
            cut_inside_e_acute,
            bytes.len() - 5,
            bytes.len(),
        ];
        let mut start = 0;
        for cut in cuts {
            steps.push(Step::Write(chunk(&bytes[start..cut])));
            steps.push(Step::Sleep(Duration::from_millis(15)));
            start = cut;
        }
        steps.push(Step::Write(chunk(
            b"data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        )));
        steps.push(Step::Write(b"0\r\n\r\n".to_vec()));
        let (port, worker) = serve(steps);
        let client = LocalClient::for_provider(LocalProvider::LlamaCpp, port).unwrap();
        let (result, deltas, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert_eq!(result.unwrap(), "Evidence comes first, café.");
        assert_eq!(deltas, vec!["Evidence ", "comes first, café", "."]);
        worker.join().unwrap();
    }

    #[test]
    fn ollama_ndjson_stream_ends_at_done_and_keeps_order() {
        let mut stream = NDJSON_HEAD.to_vec();
        let lines = [
            r#"{"model":"qwen2.5:7b","message":{"role":"assistant","content":"Local "},"done":false}"#,
            r#"{"model":"qwen2.5:7b","message":{"role":"assistant","content":"answer."},"done":false}"#,
            r#"{"model":"qwen2.5:7b","message":{"role":"assistant","content":""},"done":true,"total_duration":1}"#,
            r#"{"ignored":"anything after done is never read as content"}"#,
        ];
        let joined = lines.join("\n") + "\n";
        let bytes = joined.as_bytes();
        stream.extend(chunk(&bytes[..30]));
        stream.extend(chunk(&bytes[30..]));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::for_provider(LocalProvider::Ollama, port).unwrap();
        let (result, deltas, _) =
            collect(&client, "qwen2.5:7b", &[ChatMessage::new("user", "Hello")]);
        assert_eq!(result.unwrap(), "Local answer.");
        assert_eq!(deltas, vec!["Local ", "answer."]);
        let request = String::from_utf8(worker.join().unwrap()).unwrap();
        assert!(request.starts_with("POST /api/chat HTTP/1.1\r\n"));
        assert!(request.ends_with(r#""stream":true}"#), "{request}");
    }

    #[test]
    fn a_malformed_frame_fails_closed_after_delivering_earlier_text() {
        let mut stream = SSE_HEAD.to_vec();
        stream.extend(chunk(sse_delta("So far ").as_bytes()));
        stream.extend(chunk(
            b"data: {\"choices\": [{\"delta\": {\"content\": \"never\n\n",
        ));
        stream.extend(chunk(sse_delta("not seen").as_bytes()));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::new(port).unwrap();
        let (result, deltas, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert_eq!(
            result,
            Err(ApiError::Protocol(
                "Malformed JSON in the local model stream".into()
            ))
        );
        assert_eq!(deltas, vec!["So far "]);
        worker.join().unwrap();

        let mut stream = SSE_HEAD.to_vec();
        stream.extend(chunk(b"<html>not an event stream</html>\n"));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::new(port).unwrap();
        let (result, deltas, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert!(matches!(result, Err(ApiError::Protocol(_))), "{result:?}");
        assert!(deltas.is_empty());
        worker.join().unwrap();
    }

    #[test]
    fn a_disconnect_mid_stream_is_an_error_not_a_finished_reply() {
        let mut stream = SSE_HEAD.to_vec();
        stream.extend(chunk(sse_delta("Partial ").as_bytes()));
        stream.extend(chunk(sse_delta("reply").as_bytes()));
        // Announce a chunk and vanish: HTTP framing says the message is incomplete.
        stream.extend_from_slice(b"40\r\ndata: {\"choices\":[{\"delta\":{\"con");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::new(port).unwrap();
        let (result, deltas, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert_eq!(
            result,
            Err(ApiError::Transport(
                "The local model server closed the connection before the reply was complete".into()
            ))
        );
        assert_eq!(deltas, vec!["Partial ", "reply"]);
        worker.join().unwrap();
    }

    #[test]
    fn a_clean_end_without_a_done_marker_is_a_complete_reply() {
        // Some servers close after the last event without `[DONE]`; the chunked
        // terminator proves they meant to stop.
        let mut stream = SSE_HEAD.to_vec();
        stream.extend(chunk(sse_delta("Whole").as_bytes()));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::new(port).unwrap();
        let (result, deltas, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert_eq!(result.unwrap(), "Whole");
        assert_eq!(deltas, vec!["Whole"]);
        worker.join().unwrap();
    }

    #[test]
    fn an_oversized_streamed_reply_is_cut_off_and_the_connection_closed() {
        let mut stream = SSE_HEAD.to_vec();
        let piece = "x".repeat(1000);
        for _ in 0..40 {
            stream.extend(chunk(sse_delta(&piece).as_bytes()));
        }
        stream.extend(chunk(b"data: [DONE]\n\n"));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::new(port).unwrap();
        let (result, deltas, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert_eq!(
            result,
            Err(ApiError::Protocol(
                "Assistant reply exceeds the 32 KiB conversation message limit".into()
            ))
        );
        let delivered: usize = deltas.iter().map(String::len).sum();
        assert!(delivered <= MAX_MESSAGE_BYTES, "{delivered}");
        assert_eq!(deltas.len(), 32);
        worker.join().unwrap();
    }

    #[test]
    fn abort_ends_a_stalled_stream_promptly_and_keeps_what_arrived() {
        let mut head = SSE_HEAD.to_vec();
        head.extend(chunk(sse_delta("First").as_bytes()));
        let (port, worker) = serve(vec![
            Step::Write(head),
            Step::Sleep(Duration::from_secs(5)),
            Step::Write(chunk(sse_delta("late").as_bytes())),
            Step::Write(b"0\r\n\r\n".to_vec()),
        ]);
        let client = LocalClient::new(port).unwrap();
        let handle: Arc<Mutex<Option<AbortHandle>>> = Arc::new(Mutex::new(None));
        let deltas: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let aborter = {
            let handle = Arc::clone(&handle);
            let deltas = Arc::clone(&deltas);
            thread::spawn(move || {
                // Wait for the first delta, then cancel like a Stop button would.
                let started = Instant::now();
                while deltas.lock().unwrap().is_empty() {
                    assert!(started.elapsed() < Duration::from_secs(4), "no first delta");
                    thread::sleep(Duration::from_millis(10));
                }
                handle.lock().unwrap().as_ref().unwrap().abort();
            })
        };
        let started = Instant::now();
        let result =
            client.chat_stream(
                "qwen",
                &[ChatMessage::new("user", "Go")],
                None,
                |event| match event {
                    StreamEvent::Connected(abort) => *handle.lock().unwrap() = Some(abort),
                    StreamEvent::Delta(text) => deltas.lock().unwrap().push(text),
                },
            );
        assert!(matches!(result, Err(ApiError::Transport(_))), "{result:?}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "abort must not wait for the server: {:?}",
            started.elapsed()
        );
        assert_eq!(*deltas.lock().unwrap(), vec!["First"]);
        aborter.join().unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn error_frames_inside_a_stream_are_surfaced_as_server_messages() {
        let mut stream = NDJSON_HEAD.to_vec();
        stream.extend(chunk(b"{\"error\":\"model 'nope' not found\"}\n"));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::for_provider(LocalProvider::Ollama, port).unwrap();
        let (result, _, _) = collect(&client, "nope", &[ChatMessage::new("user", "Go")]);
        assert_eq!(
            result,
            Err(ApiError::Protocol(
                "Local model server reported: model 'nope' not found".into()
            ))
        );
        worker.join().unwrap();

        let mut stream = SSE_HEAD.to_vec();
        stream.extend(chunk(
            b"data: {\"error\":{\"message\":\"context length exceeded\",\"type\":\"invalid_request_error\"}}\n\n",
        ));
        stream.extend_from_slice(b"0\r\n\r\n");
        let (port, worker) = serve(vec![Step::Write(stream)]);
        let client = LocalClient::new(port).unwrap();
        let (result, _, _) = collect(&client, "qwen", &[ChatMessage::new("user", "Go")]);
        assert_eq!(
            result,
            Err(ApiError::Protocol(
                "Local model server reported: context length exceeded".into()
            ))
        );
        worker.join().unwrap();
    }

    #[test]
    fn frames_are_decoded_strictly() {
        assert_eq!(parse_sse_line("").unwrap(), Frame::default());
        assert_eq!(parse_sse_line(": ping").unwrap(), Frame::default());
        assert_eq!(parse_sse_line("event: message").unwrap(), Frame::default());
        assert_eq!(
            parse_sse_line("data: [DONE]").unwrap(),
            Frame {
                text: None,
                finished: true
            }
        );
        assert_eq!(
            parse_sse_line(r#"data:{"choices":[{"delta":{"content":"hi"}}]}"#)
                .unwrap()
                .text
                .as_deref(),
            Some("hi")
        );
        assert_eq!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#)
                .unwrap()
                .text,
            None
        );
        assert!(parse_sse_line("data: {oops").is_err());
        assert!(parse_sse_line("garbage without a field").is_err());
        assert_eq!(
            parse_ndjson_line(r#"{"message":{"content":"a"},"done":false}"#).unwrap(),
            Frame {
                text: Some("a".into()),
                finished: false
            }
        );
        assert_eq!(
            parse_ndjson_line(r#"{"message":{"content":""},"done":true}"#).unwrap(),
            Frame {
                text: Some(String::new()),
                finished: true
            }
        );
        assert!(parse_ndjson_line("{").is_err());
    }

    #[test]
    fn ollama_native_routes_and_payloads_work_with_loopback_fixture() {
        let (client, worker) = mock(
            r#"{"models":[{"name":"qwen2.5:7b"},{"name":"qwen2.5:7b"},{"name":"llava:latest"}]}"#,
            "200 OK",
        );
        let client = LocalClient::for_provider(LocalProvider::Ollama, client.port).unwrap();
        assert_eq!(client.models().unwrap(), vec!["llava:latest", "qwen2.5:7b"]);
        let request = String::from_utf8(worker.join().unwrap()).unwrap();
        assert!(request.starts_with("GET /api/tags HTTP/1.1\r\n"));
        // A non-streaming Ollama answer is one NDJSON line with done:true.
        let (client, worker) = mock(
            r#"{"message":{"role":"assistant","content":"Local answer."},"done":true}"#,
            "200 OK",
        );
        let client = LocalClient::for_provider(LocalProvider::Ollama, client.port).unwrap();
        assert_eq!(
            client
                .chat("qwen2.5:7b", &[ChatMessage::new("user", "Hello")])
                .unwrap(),
            "Local answer."
        );
        worker.join().unwrap();
    }

    #[test]
    fn ollama_vision_request_is_explicit_and_bounded() {
        let input = [ChatMessage::new("user", "Describe this")];
        let wire: Value = serde_json::from_str(
            &build_ollama_request("vision", &input, Some(b"\x89PNG\r\n\x1a\n")).unwrap(),
        )
        .unwrap();
        assert_eq!(wire["messages"][0]["images"][0], "iVBORw0KGgo=");
        assert_eq!(wire["stream"], true);
        assert!(build_ollama_request("vision", &input, Some(b"bad")).is_err());
        assert!(parse_ollama_models(r#"{"models":null}"#).is_err());
        assert!(parse_ollama_chat(r#"{"message":{"content":""}}"#).is_err());
    }

    #[test]
    fn provider_defaults_are_explicit_and_local_only() {
        assert_eq!(LocalProvider::LmStudio.default_port(), 1234);
        assert_eq!(LocalProvider::Ollama.default_port(), 11434);
        assert_eq!(LocalProvider::LlamaCpp.default_port(), 8080);
        assert!(LocalClient::for_provider(LocalProvider::OpenAiCompatible, 0).is_err());
    }

    #[test]
    fn failed_server_status_is_not_presented_as_a_reply() {
        let (client, worker) = mock("{}", "404 Not Found");
        let err = client.models().unwrap_err();
        assert!(err.to_string().contains("HTTP 404"));
        worker.join().unwrap();
    }

    #[test]
    fn server_error_message_is_surfaced() {
        let (client, worker) = mock(
            r#"{"error":"Engine protocol predict request returned 500: {\"error\":{\"code\":500,\"message\":\"image input is not supported - hint: if this is unexpected, you may need to provide the mmproj\",\"type\":\"server_error\"}}"}"#,
            "400 Bad Request",
        );
        let err = client
            .chat("qwen-local", &[ChatMessage::new("user", "Hi")])
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "Local model server returned HTTP 400: image input is not supported - hint: if this is unexpected, you may need to provide the mmproj"
        );
        worker.join().unwrap();
    }

    #[test]
    fn server_error_detail_is_bounded_and_printable() {
        assert_eq!(
            server_error_detail(r#"{"error":{"message":"Model not loaded"}}"#).as_deref(),
            Some("Model not loaded")
        );
        let long = format!(r#"{{"error":"a\nb{}"}}"#, "x".repeat(1000));
        let detail = server_error_detail(&long).unwrap();
        assert!(detail.starts_with("a b") && detail.chars().count() == MAX_ERROR_DETAIL_CHARS);
        assert_eq!(server_error_detail("not json"), None);
        assert_eq!(server_error_detail(r#"{"error":"  "}"#), None);
    }

    #[test]
    fn nothing_listening_is_a_transport_error() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let err = LocalClient::new(port).unwrap().models().unwrap_err();
        assert!(matches!(err, ApiError::Transport(_)), "{err:?}");
        assert!(
            err.to_string()
                .starts_with("Cannot reach the selected local model server")
        );
    }
}
