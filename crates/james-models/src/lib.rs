//! Local-only, synchronous HTTP adapter for an OpenAI-compatible model server.
//! Works with any server exposing `/v1/models` and `/v1/chat/completions` on
//! loopback, including LM Studio (default port 1234), Ollama (port 11434) and
//! llama.cpp's `llama-server` (port 8080).
//! Only 127.0.0.1 and an explicit numeric port are ever used; it cannot be
//! directed to a remote host by a model or a configuration string.
//! Network locality does NOT prove that the inference backend itself is local.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::Read;
use std::time::Duration;

const MAX_REPLY_BYTES: u64 = 1_048_576;
const MAX_MESSAGES: usize = 80;
const MAX_MESSAGE_BYTES: usize = 32_768;
const MAX_TOTAL_BYTES: usize = 262_144;
const MAX_ERROR_BYTES: u64 = 4_096;
const MAX_ERROR_DETAIL_CHARS: usize = 300;

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

#[derive(Debug, Clone, Copy)]
pub struct LocalClient {
    port: u16,
}

impl LocalClient {
    pub fn new(port: u16) -> Result<Self, ApiError> {
        if port == 0 {
            return Err(ApiError::InvalidConfig(
                "Choose a nonzero model server port".into(),
            ));
        }
        Ok(Self { port })
    }

    fn url(&self, path: &str) -> String {
        // The host is a compile-time constant and path is never user-provided.
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new()
            .redirects(0)
            .try_proxy_from_env(false)
            .timeout(Duration::from_secs(90))
            .timeout_connect(Duration::from_secs(3))
            .timeout_read(Duration::from_secs(90))
            .timeout_write(Duration::from_secs(10))
            .build()
    }

    pub fn models(&self) -> Result<Vec<String>, ApiError> {
        let response = Self::agent()
            .get(&self.url("/v1/models"))
            .call()
            .map_err(map_http_error)?;
        let body = read_limited(response)?;
        parse_models(&body)
    }

    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String, ApiError> {
        self.chat_with_image(model, messages, None)
    }

    /// Attach one explicitly selected PNG to the final user turn only.
    /// Image bytes never enter ChatMessage or conversation persistence.
    pub fn chat_with_image(
        &self,
        model: &str,
        messages: &[ChatMessage],
        png: Option<&[u8]>,
    ) -> Result<String, ApiError> {
        let body = build_request(model, messages, png)?;
        let response = Self::agent()
            .post(&self.url("/v1/chat/completions"))
            .set("Content-Type", "application/json")
            .send_string(&body)
            .map_err(map_http_error)?;
        let body = read_limited(response)?;
        parse_chat(&body)
    }
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
    serde_json::to_string(&serde_json::json!({"model":model,"messages":wire,"stream":false}))
        .map_err(|err| ApiError::Protocol(format!("Cannot serialize request: {err}")))
}

fn map_http_error(error: ureq::Error) -> ApiError {
    match error {
        ureq::Error::Status(code, response) => {
            let mut body = String::new();
            // The server's explanation (e.g. a model without image support) is
            // the actionable part; read a bounded amount and show it as data.
            let _ = response
                .into_reader()
                .take(MAX_ERROR_BYTES)
                .read_to_string(&mut body);
            match server_error_detail(&body) {
                Some(detail) => ApiError::Transport(format!(
                    "The local model server returned HTTP {code}: {detail}"
                )),
                None => ApiError::Transport(format!(
                    "The local model server returned HTTP {code}. Check the selected model and server."
                )),
            }
        }
        ureq::Error::Transport(err) => ApiError::Transport(format!(
            "Cannot reach the local model server on the selected localhost port: {err}"
        )),
    }
}

/// Extracts a short, printable message from an OpenAI-style error body.
/// Some servers (e.g. LM Studio) wrap the engine's JSON error inside the
/// outer string; Ollama returns a flat `{"error":"..."}`. Both are handled.
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

fn read_limited(response: ureq::Response) -> Result<String, ApiError> {
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_REPLY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| {
            ApiError::Transport(format!("Reading local model response failed: {err}"))
        })?;
    if bytes.len() as u64 > MAX_REPLY_BYTES {
        return Err(ApiError::Protocol(
            "Local model response exceeded 1 MiB limit".into(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| ApiError::Protocol("Local model response was not UTF-8".into()))
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
    use std::thread;

    fn mock(body: &'static str, status: &'static str) -> (LocalClient, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            // Consume the full request before replying. Closing with unread
            // bytes can reset the connection on Windows, making tests flaky.
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(
                line.starts_with("GET /v1/models ")
                    || line.starts_with("POST /v1/chat/completions ")
            );
            let mut body_size = 0_usize;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line == "\n" {
                    break;
                }
                if let Some(value) = line.strip_prefix("Content-Length: ") {
                    body_size = value.trim().parse().unwrap();
                } else if let Some(value) = line.strip_prefix("content-length: ") {
                    body_size = value.trim().parse().unwrap();
                }
            }
            if body_size > 0 {
                assert!(body_size <= 262_144);
                let mut request_body = vec![0_u8; body_size];
                reader.read_exact(&mut request_body).unwrap();
            }
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).unwrap();
            socket.flush().unwrap();
        });
        (LocalClient::new(port).unwrap(), worker)
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
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let client = LocalClient::new(listener.local_addr().unwrap().port()).unwrap();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://192.0.2.1/v1/models\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        assert!(client.models().is_err());
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
        worker.join().unwrap();
    }

    #[test]
    fn localhost_chat_returns_assistant_content() {
        let (client, worker) = mock(
            r#"{"choices":[{"message":{"role":"assistant","content":"A testable idea."}}]}"#,
            "200 OK",
        );
        let result = client
            .chat("qwen-local", &[ChatMessage::new("user", "Propose a test")])
            .unwrap();
        assert_eq!(result, "A testable idea.");
        worker.join().unwrap();
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
            "The local model server returned HTTP 400: image input is not supported - hint: if this is unexpected, you may need to provide the mmproj"
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
}
