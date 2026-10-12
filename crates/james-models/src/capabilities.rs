//! What a model can do, as far as its own server says.
//!
//! James never infers a capability from a model's name. Each provider gets
//! one or two read-only, bounded requests on the same loopback port, and
//! anything the server does not state stays [`Support::Unknown`]. A failed
//! probe is never an error the user has to deal with: it just leaves the
//! facts unknown, and the UI says so instead of pretending.

use crate::{LocalClient, LocalProvider, read_ok_body};
use james_loopback::{Client, Limits, Timeouts};
use serde_json::Value;
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_BYTES: usize = 1_048_576;

/// Whether a model can do something.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Support {
    Yes,
    No,
    /// The server did not say. James lets the user try and reports the
    /// server's answer, rather than claiming the feature works.
    #[default]
    Unknown,
}

impl Support {
    fn from_bool(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }
}

/// Facts about one model, each one stated by the server it runs on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelInfo {
    /// Can answer chat messages (an embedding-only model cannot).
    pub chat: Support,
    /// Can read an attached image.
    pub vision: Support,
    /// Tokens the server will actually accept for one request right now. Only
    /// an effective, configured window is reported here, never the larger
    /// maximum a model was trained for.
    pub context_tokens: Option<u32>,
    /// In memory now. `Some(false)` means the first request will load it first.
    pub loaded: Option<bool>,
    /// The server says this model runs on another computer, such as an
    /// Ollama cloud model. Replies then leave this machine through the
    /// server, even though James itself only talks to loopback.
    pub remote_host: Option<String>,
    /// Which server route the facts came from, for the settings screen.
    pub source: Option<&'static str>,
}

impl ModelInfo {
    /// True when the server confirmed nothing at all.
    pub fn is_unknown(&self) -> bool {
        self.source.is_none()
    }
}

impl LocalClient {
    fn probe(&self) -> Option<Client> {
        Client::new(self.port).ok().map(|client| {
            client
                .with_timeouts(Timeouts {
                    connect: Duration::from_secs(2),
                    idle: PROBE_TIMEOUT,
                    total: Some(PROBE_TIMEOUT),
                })
                .with_limits(Limits {
                    headers: 16 * 1024,
                    body: PROBE_BYTES,
                })
        })
    }

    fn probe_get(&self, path: &str) -> Option<String> {
        let response = self
            .probe()?
            .get(path)
            .header("Accept", "application/json")
            .send()
            .ok()?;
        read_ok_body(response).ok()
    }

    /// Asks the server what `model` can do. Read-only and bounded to a few
    /// seconds; unanswered questions stay [`Support::Unknown`].
    pub fn model_info(&self, model: &str) -> ModelInfo {
        let model = model.trim();
        if model.is_empty() || model.len() > 256 {
            return ModelInfo::default();
        }
        match self.provider {
            LocalProvider::Ollama => self.ollama_info(model),
            LocalProvider::LmStudio => self
                .probe_get("/api/v0/models")
                .and_then(|body| lm_studio_entry(&body, model))
                .or_else(|| self.listing_info(model))
                .unwrap_or_default(),
            LocalProvider::LlamaCpp => {
                let mut info = self.listing_info(model).unwrap_or_default();
                if let Some((vision, n_ctx)) =
                    self.probe_get("/props").and_then(|body| llama_props(&body))
                {
                    if info.vision == Support::Unknown {
                        info.vision = vision;
                    }
                    info.context_tokens = info.context_tokens.or(n_ctx);
                    info.source.get_or_insert("llama.cpp /props");
                }
                info
            }
            LocalProvider::OpenAiCompatible => self.listing_info(model).unwrap_or_default(),
        }
    }

    /// The OpenAI-style model list, which llama.cpp and some other servers
    /// annotate with input modalities and the configured context.
    fn listing_info(&self, model: &str) -> Option<ModelInfo> {
        self.probe_get("/v1/models")
            .and_then(|body| openai_entry(&body, model))
    }

    fn ollama_info(&self, model: &str) -> ModelInfo {
        let request = serde_json::json!({ "model": model }).to_string();
        let show = self.probe().and_then(|client| {
            let response = client
                .post("/api/show")
                .header("Accept", "application/json")
                .body("application/json", request.as_bytes())
                .send()
                .ok()?;
            read_ok_body(response).ok()
        });
        let mut info = show.and_then(|body| ollama_show(&body)).unwrap_or_default();
        // Ollama 0.40 marks cloud models in its list, not in /api/show.
        if info.remote_host.is_none()
            && let Some(body) = self.probe_get("/api/tags")
        {
            info.remote_host = ollama_listed_remote(&body, model);
        }
        if let Some(body) = self.probe_get("/api/ps") {
            let (loaded, context) = ollama_running(&body, model);
            info.loaded = Some(loaded);
            info.context_tokens = context;
            info.source.get_or_insert("Ollama /api/ps");
        }
        info
    }
}

fn positive_u32(value: Option<&Value>) -> Option<u32> {
    value
        .and_then(Value::as_u64)
        .filter(|tokens| *tokens > 0)
        .and_then(|tokens| u32::try_from(tokens).ok())
}

fn bounded_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|text| {
            text.chars()
                .filter(|c| !c.is_control())
                .take(200)
                .collect::<String>()
        })
        .filter(|text| !text.trim().is_empty())
}

/// Ollama's `/api/show`: `capabilities` names what the model does
/// ("completion", "vision", "embedding", ...), and cloud models carry
/// `remote_host`.
pub(crate) fn ollama_show(body: &str) -> Option<ModelInfo> {
    let value: Value = serde_json::from_str(body).ok()?;
    let mut info = ModelInfo {
        remote_host: bounded_text(value.get("remote_host")),
        source: Some("Ollama /api/show"),
        ..ModelInfo::default()
    };
    if let Some(capabilities) = value.get("capabilities").and_then(Value::as_array) {
        let has = |name: &str| capabilities.iter().any(|c| c.as_str() == Some(name));
        info.chat = Support::from_bool(has("completion"));
        info.vision = Support::from_bool(has("vision"));
    }
    Some(info)
}

/// Where Ollama's own list says `model` runs, if not on this computer.
pub(crate) fn ollama_listed_remote(body: &str, model: &str) -> Option<String> {
    if !crate::parse_ollama_remote(body)
        .iter()
        .any(|name| name == model)
    {
        return None;
    }
    let value: Value = serde_json::from_str(body).ok()?;
    let host = value
        .get("models")?
        .as_array()?
        .iter()
        .find(|row| row.get("name").and_then(Value::as_str) == Some(model))
        .and_then(|row| bounded_text(row.get("remote_host")));
    Some(host.unwrap_or_else(|| "a cloud service (named as a cloud model)".into()))
}

/// Ollama's `/api/ps` lists the models in memory with the context window
/// each was loaded with.
pub(crate) fn ollama_running(body: &str, model: &str) -> (bool, Option<u32>) {
    let Some(rows) = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("models").and_then(Value::as_array).cloned())
    else {
        return (false, None);
    };
    rows.iter()
        .find(|row| {
            ["name", "model"]
                .iter()
                .any(|key| row.get(key).and_then(Value::as_str) == Some(model))
        })
        .map(|row| (true, positive_u32(row.get("context_length"))))
        .unwrap_or((false, None))
}

/// LM Studio's `/api/v0/models`: `type` is "llm", "vlm" or "embeddings" and
/// `state` is "loaded" or "not-loaded".
pub(crate) fn lm_studio_entry(body: &str, model: &str) -> Option<ModelInfo> {
    let value: Value = serde_json::from_str(body).ok()?;
    let row = value
        .get("data")?
        .as_array()?
        .iter()
        .find(|row| row.get("id").and_then(Value::as_str) == Some(model))?;
    let kind = row.get("type").and_then(Value::as_str).unwrap_or_default();
    let (chat, vision) = match kind {
        "vlm" => (Support::Yes, Support::Yes),
        "llm" => (Support::Yes, Support::No),
        "embeddings" | "embedding" => (Support::No, Support::No),
        _ => (Support::Unknown, Support::Unknown),
    };
    let loaded = row
        .get("state")
        .and_then(Value::as_str)
        .map(|state| state == "loaded");
    Some(ModelInfo {
        chat,
        vision,
        context_tokens: if loaded == Some(true) {
            positive_u32(row.get("loaded_context_length"))
        } else {
            None
        },
        loaded,
        remote_host: None,
        source: Some("LM Studio /api/v0/models"),
    })
}

/// An OpenAI-style `/v1/models` entry. llama.cpp adds
/// `architecture.input_modalities` and `meta.n_ctx`; servers that add
/// neither leave everything unknown.
pub(crate) fn openai_entry(body: &str, model: &str) -> Option<ModelInfo> {
    let value: Value = serde_json::from_str(body).ok()?;
    let row = value
        .get("data")?
        .as_array()?
        .iter()
        .find(|row| row.get("id").and_then(Value::as_str) == Some(model))?;
    let modalities = row
        .pointer("/architecture/input_modalities")
        .and_then(Value::as_array);
    let context = positive_u32(row.pointer("/meta/n_ctx"));
    if modalities.is_none() && context.is_none() {
        return None;
    }
    Some(ModelInfo {
        vision: modalities
            .map(|list| Support::from_bool(list.iter().any(|m| m.as_str() == Some("image"))))
            .unwrap_or_default(),
        context_tokens: context,
        source: Some("/v1/models"),
        ..ModelInfo::default()
    })
}

/// llama.cpp's `/props` for a single-model server: `modalities.vision` and
/// the per-request context in `default_generation_settings.n_ctx`. A router
/// reports `n_ctx` 0 and no modalities, which leaves both unknown.
pub(crate) fn llama_props(body: &str) -> Option<(Support, Option<u32>)> {
    let value: Value = serde_json::from_str(body).ok()?;
    let vision = value
        .pointer("/modalities/vision")
        .and_then(Value::as_bool)
        .map(Support::from_bool)
        .unwrap_or_default();
    let n_ctx = positive_u32(value.pointer("/default_generation_settings/n_ctx"));
    (vision != Support::Unknown || n_ctx.is_some()).then_some((vision, n_ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_capabilities_come_from_the_server_not_the_name() {
        let info = ollama_show(
            r#"{"capabilities":["completion","vision"],"model_info":{"gemma3.context_length":131072}}"#,
        )
        .unwrap();
        assert_eq!((info.chat, info.vision), (Support::Yes, Support::Yes));
        // The trained maximum is not the window Ollama will actually use.
        assert_eq!(info.context_tokens, None);
        assert_eq!(info.remote_host, None);

        // A name that sounds visual proves nothing; the list does.
        let info = ollama_show(r#"{"capabilities":["completion","tools"]}"#).unwrap();
        assert_eq!(info.vision, Support::No);
        let info = ollama_show(r#"{"capabilities":["embedding"]}"#).unwrap();
        assert_eq!(info.chat, Support::No);
        // An older server without the list says nothing either way.
        let info = ollama_show(r#"{"modelfile":"FROM x"}"#).unwrap();
        assert_eq!(
            (info.chat, info.vision),
            (Support::Unknown, Support::Unknown)
        );
        assert!(ollama_show("not json").is_none());
    }

    #[test]
    fn an_ollama_cloud_model_is_reported_as_remote() {
        let info = ollama_show(
            r#"{"capabilities":["completion","tools","thinking"],"remote_model":"gpt-oss:120b","remote_host":"https://ollama.com:443"}"#,
        )
        .unwrap();
        assert_eq!(info.remote_host.as_deref(), Some("https://ollama.com:443"));
        // Control characters from the server never reach the UI.
        let info = ollama_show(r#"{"remote_host":"evil\u001b[2Jhost"}"#).unwrap();
        assert_eq!(info.remote_host.as_deref(), Some("evil[2Jhost"));
    }

    #[test]
    fn ollama_cloud_models_are_found_in_the_list_when_show_is_silent() {
        // Recorded from Ollama 0.40.3: /api/tags marks the cloud model.
        let tags = r#"{"models":[{"name":"gpt-oss:20b-cloud","remote_model":"gpt-oss:20b","remote_host":"https://ollama.com","capabilities":["completion","tools","thinking"]},{"name":"gemma3:4b","capabilities":["completion","vision"]},{"name":"other:7b-cloud"}]}"#;
        assert_eq!(
            ollama_listed_remote(tags, "gpt-oss:20b-cloud").as_deref(),
            Some("https://ollama.com")
        );
        assert_eq!(ollama_listed_remote(tags, "gemma3:4b"), None);
        assert!(ollama_listed_remote(tags, "other:7b-cloud").is_some());
    }

    #[test]
    fn ollama_running_models_report_their_loaded_context() {
        let body = r#"{"models":[{"name":"gemma3:4b","model":"gemma3:4b","context_length":8192},{"name":"other:1b","context_length":0}]}"#;
        assert_eq!(ollama_running(body, "gemma3:4b"), (true, Some(8192)));
        assert_eq!(ollama_running(body, "other:1b"), (true, None));
        assert_eq!(ollama_running(body, "qwen3:4b"), (false, None));
        assert_eq!(ollama_running("{}", "qwen3:4b"), (false, None));
    }

    #[test]
    fn lm_studio_types_and_load_state_are_read_exactly() {
        let body = r#"{"data":[
            {"id":"google/gemma-3-4b","type":"vlm","state":"loaded","max_context_length":131072,"loaded_context_length":4096},
            {"id":"qwen2.5-7b-instruct","type":"llm","state":"not-loaded","max_context_length":32768},
            {"id":"text-embedding-nomic","type":"embeddings","state":"not-loaded"},
            {"id":"mystery","type":"something-new"}]}"#;
        let gemma = lm_studio_entry(body, "google/gemma-3-4b").unwrap();
        assert_eq!((gemma.chat, gemma.vision), (Support::Yes, Support::Yes));
        assert_eq!(
            (gemma.loaded, gemma.context_tokens),
            (Some(true), Some(4096))
        );
        let qwen = lm_studio_entry(body, "qwen2.5-7b-instruct").unwrap();
        assert_eq!(qwen.vision, Support::No);
        // Not loaded: the window it will get is not known yet.
        assert_eq!((qwen.loaded, qwen.context_tokens), (Some(false), None));
        assert_eq!(
            lm_studio_entry(body, "text-embedding-nomic").unwrap().chat,
            Support::No
        );
        let mystery = lm_studio_entry(body, "mystery").unwrap();
        assert_eq!(
            (mystery.chat, mystery.vision),
            (Support::Unknown, Support::Unknown)
        );
        assert!(lm_studio_entry(body, "absent").is_none());
    }

    #[test]
    fn llama_cpp_listing_and_props_report_vision_and_context() {
        let listing = r#"{"object":"list","data":[{"id":"smolvlm","object":"model","architecture":{"input_modalities":["text","image"],"output_modalities":["text"]},"meta":{"n_ctx":4096,"n_ctx_train":8192}}]}"#;
        let info = openai_entry(listing, "smolvlm").unwrap();
        assert_eq!(info.vision, Support::Yes);
        assert_eq!(info.context_tokens, Some(4096));
        let text_only = r#"{"data":[{"id":"qwen","architecture":{"input_modalities":["text"]}}]}"#;
        assert_eq!(openai_entry(text_only, "qwen").unwrap().vision, Support::No);
        // A plain OpenAI-compatible listing says nothing about capabilities.
        assert!(openai_entry(r#"{"data":[{"id":"plain"}]}"#, "plain").is_none());

        let props = r#"{"default_generation_settings":{"n_ctx":8192},"modalities":{"vision":false,"audio":false}}"#;
        assert_eq!(llama_props(props), Some((Support::No, Some(8192))));
        let router = r#"{"role":"router","default_generation_settings":{"params":{},"n_ctx":0}}"#;
        assert_eq!(llama_props(router), None);
    }

    #[test]
    fn unusable_inputs_ask_nothing() {
        let client = LocalClient::new(9).unwrap();
        assert!(client.model_info("  ").is_unknown());
        assert!(client.model_info(&"x".repeat(300)).is_unknown());
    }
}
