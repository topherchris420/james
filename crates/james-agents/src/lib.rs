//! James's local multi-agent runtime.
//!
//! A small team of persistent assistants (Coordinator, Researcher, Writer,
//! Reviewer by default) works on tasks a person gives them. Everything they do
//! is recorded in a hash-chained journal ([`journal`]); all state is a fold
//! over it. The host, never a model, decides what is allowed:
//!
//! - a model's reply is one JSON action from a closed vocabulary
//!   ([`protocol::Action`]); anything else is refused and recorded;
//! - each action needs a capability the agent's profile holds;
//! - delegation is bounded by depth, width and a budget carved out of the
//!   delegator's own, and refuses cycles ([`runtime`]);
//! - memory is scoped and carries provenance the host derives ([`memory`]);
//! - file reads stay inside one workspace folder ([`tools`]);
//! - a file write is only ever a proposal: it waits for the person's decision
//!   on the exact content, and is re-checked against the file when applied;
//! - routines create tasks on a schedule but never approve anything
//!   ([`schedule`]).
//!
//! There is no OS sandbox here. These limits are enforced by this crate's
//! code, not by the operating system.

pub use james_interop::canonical;
pub mod clock;
pub mod identity;
pub mod journal;
pub mod memory;
pub mod model;
pub mod protocol;
pub mod runtime;
pub mod schedule;
pub mod tools;

pub use identity::{AgentProfile, Capability, default_team};
pub use model::{CancelSignal, LocalModel, ModelPort, ModelReply, ModelRequest, ScriptedModel};
pub use runtime::{Runtime, RuntimeError, StepOutcome};

/// Who wrote the words in a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Generation {
    /// A language model wrote them.
    Model,
    /// Fixed text or a rule-based policy.
    Scripted,
    /// The person using James.
    Person,
    /// James's own code.
    Host,
}

impl Generation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Scripted => "scripted",
            Self::Person => "person",
            Self::Host => "host",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "model" => Self::Model,
            "scripted" => Self::Scripted,
            "person" => Self::Person,
            "host" => Self::Host,
            _ => return None,
        })
    }
}

pub use james_interop::AGENT_INTEROP_V1_MANIFEST_SHA256;

/// Bounded single-line text for display and prompts: control characters
/// become spaces and the result is at most `max` characters.
pub fn clip(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}

/// `clip`, then collapse whitespace onto one line.
pub fn one_line(text: &str, max: usize) -> String {
    clip(text, max)
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
