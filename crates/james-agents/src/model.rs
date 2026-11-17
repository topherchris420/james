//! Where an assistant's next reply comes from.
//!
//! [`LocalModel`] asks a model server on this computer (Ollama, LM Studio,
//! llama.cpp) through `james-models`, whose only network path is the
//! loopback-only `james-loopback`. [`ScriptedModel`] is a deterministic
//! policy for tests and offline demonstrations; its replies are recorded as
//! `scripted`, never as a model's.
//!
//! No cloud or external harness adapter ships. A remote provider would need
//! its own, visibly labelled transport and the person's explicit opt-in
//! (James ADR 008); this trait does not make one acceptable by itself.

use crate::Generation;
use crate::protocol::Observation;
use james_loopback::AbortHandle;
use james_models::{ChatMessage, LocalClient, LocalProvider, StreamEvent};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Everything a reply may depend on. `system` and `user` are the exact text a
/// language model receives; the structured fields serve scripted policies.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub agent_id: String,
    pub task_id: String,
    pub objective: String,
    pub observations: Vec<Observation>,
    /// Actions this agent already took in this task, as JSON.
    pub previous_actions: Vec<serde_json::Value>,
    pub system: String,
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReply {
    pub text: String,
    /// What answered, as the adapter names it.
    pub model: String,
    pub generation: Generation,
}

/// Shared between the runtime's thread and whoever may cancel: requesting a
/// cancellation aborts an in-flight model call for an affected task.
#[derive(Clone, Default)]
pub struct CancelSignal {
    inner: Arc<CancelInner>,
}

#[derive(Default)]
struct CancelInner {
    requested: Mutex<BTreeSet<String>>,
    current: Mutex<Option<(String, AbortHandle)>>,
    current_task: Mutex<Option<String>>,
    aborted: AtomicBool,
}

impl CancelSignal {
    /// Ask for these tasks to stop. The runtime records the cancellation on
    /// its next step; an in-flight call for one of them is cut now.
    pub fn request(&self, task_ids: impl IntoIterator<Item = String>) {
        let ids: Vec<String> = task_ids.into_iter().collect();
        let mut requested = self
            .inner
            .requested
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        requested.extend(ids.iter().cloned());
        drop(requested);
        let current_task = self
            .inner
            .current_task
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if current_task.is_some_and(|t| ids.contains(&t)) {
            self.inner.aborted.store(true, Ordering::SeqCst);
            if let Some((_, handle)) = self
                .inner
                .current
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                handle.abort();
            }
        }
    }

    pub(crate) fn take_requested(&self) -> BTreeSet<String> {
        std::mem::take(
            &mut *self
                .inner
                .requested
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        )
    }

    pub(crate) fn begin(&self, task_id: &str) {
        self.inner.aborted.store(false, Ordering::SeqCst);
        *self
            .inner
            .current_task
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(task_id.to_owned());
    }

    pub(crate) fn end(&self) {
        *self
            .inner
            .current_task
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self.inner.current.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Whether the call in flight was cut by a cancellation.
    pub fn aborted(&self) -> bool {
        self.inner.aborted.load(Ordering::SeqCst)
    }

    /// Adapters register the live connection so a cancellation can cut it.
    pub fn attach(&self, handle: AbortHandle) {
        let task = self
            .inner
            .current_task
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if self.aborted() {
            handle.abort();
        }
        *self.inner.current.lock().unwrap_or_else(|e| e.into_inner()) = task.map(|t| (t, handle));
    }
}

pub trait ModelPort: Send {
    /// How records name this source, e.g. `llama.cpp server · qwen2.5-1.5b`.
    fn label(&self) -> String;
    fn complete(
        &mut self,
        request: &ModelRequest,
        cancel: &CancelSignal,
    ) -> Result<ModelReply, String>;
}

/// A model server on this computer.
pub struct LocalModel {
    client: LocalClient,
    model: String,
}

impl LocalModel {
    pub fn new(
        provider: LocalProvider,
        port: u16,
        model: impl Into<String>,
    ) -> Result<Self, String> {
        let client = LocalClient::for_provider(provider, port).map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            model: model.into(),
        })
    }
}

impl ModelPort for LocalModel {
    fn label(&self) -> String {
        format!(
            "{} on port {} · {}",
            self.client.provider().label(),
            self.client.port(),
            self.model
        )
    }

    fn complete(
        &mut self,
        request: &ModelRequest,
        cancel: &CancelSignal,
    ) -> Result<ModelReply, String> {
        let messages = [
            ChatMessage::new("system", &request.system),
            ChatMessage::new("user", &request.user),
        ];
        let text = self
            .client
            .chat_stream(&self.model, &messages, None, |event| {
                if let StreamEvent::Connected(handle) = event {
                    cancel.attach(handle);
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(ModelReply {
            text,
            model: self.label(),
            generation: Generation::Model,
        })
    }
}

/// A deterministic policy standing in for a model. Every reply it gives is
/// recorded as `scripted`.
pub struct ScriptedModel {
    name: String,
    policy: Box<dyn FnMut(&ModelRequest) -> String + Send>,
}

impl ScriptedModel {
    pub fn new(
        name: impl Into<String>,
        policy: impl FnMut(&ModelRequest) -> String + Send + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            policy: Box::new(policy),
        }
    }

    /// Replies from a fixed list per agent, in order; "fail" once exhausted.
    pub fn queue(replies: Vec<(&'static str, &'static str)>) -> Self {
        let mut queues: std::collections::HashMap<String, std::collections::VecDeque<String>> =
            Default::default();
        for (agent, reply) in replies {
            queues
                .entry(agent.to_owned())
                .or_default()
                .push_back(reply.to_owned());
        }
        Self::new("scripted queue", move |req| {
            queues
                .get_mut(&req.agent_id)
                .and_then(|q| q.pop_front())
                .unwrap_or_else(|| r#"{"action":"fail","reason":"script exhausted"}"#.to_owned())
        })
    }
}

impl ModelPort for ScriptedModel {
    fn label(&self) -> String {
        format!("scripted policy · {}", self.name)
    }
    fn complete(
        &mut self,
        request: &ModelRequest,
        _cancel: &CancelSignal,
    ) -> Result<ModelReply, String> {
        Ok(ModelReply {
            text: (self.policy)(request),
            model: self.label(),
            generation: Generation::Scripted,
        })
    }
}
