//! Persistent assistant identities and the capabilities the host checks.
//!
//! An identity is recorded once in the journal (`agent.registered`) and keeps
//! its id for good. A change of role or capabilities is a new revision, never
//! an edit in place. Capabilities are a closed set: an agent cannot name a new
//! one, and nothing a model writes can add one.

use crate::canonical;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// List and read files inside the chosen workspace folder.
    ReadWorkspace,
    /// Write text as a task result.
    Draft,
    /// Propose a file write, which waits for the person.
    ProposeWrite,
    /// Hand a sub-task to another assistant.
    Delegate,
    /// Ask the host to check citations against the workspace.
    Review,
    /// Keep a memory entry.
    Remember,
}

impl Capability {
    pub const ALL: [Self; 6] = [
        Self::ReadWorkspace,
        Self::Draft,
        Self::ProposeWrite,
        Self::Delegate,
        Self::Review,
        Self::Remember,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadWorkspace => "read_workspace",
            Self::Draft => "draft",
            Self::ProposeWrite => "propose_write",
            Self::Delegate => "delegate",
            Self::Review => "review",
            Self::Remember => "remember",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
    pub fn plain(self) -> &'static str {
        match self {
            Self::ReadWorkspace => "read files in the workspace folder",
            Self::Draft => "write drafts",
            Self::ProposeWrite => "propose a file change for your approval",
            Self::Delegate => "hand work to another assistant",
            Self::Review => "check citations against the files",
            Self::Remember => "keep notes in memory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProfile {
    pub id: String,
    pub display_name: String,
    pub role: String,
    pub capabilities: BTreeSet<Capability>,
    pub revision: u32,
    pub created_at: String,
    /// Host-written guidance added to this agent's prompt.
    pub instructions: String,
}

impl AgentProfile {
    pub fn can(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// The sealed `agent-identity/v1` record.
    pub fn identity_record(&self) -> Value {
        let caps: Vec<&str> = self.capabilities.iter().map(|c| c.as_str()).collect();
        canonical::seal(json!({
            "schema": "agent-identity/v1",
            "system": "james-desktop",
            "agent_id": self.id,
            "display_name": self.display_name,
            "role": self.role,
            "kind": "model-seat",
            "capabilities": caps,
            "revision": self.revision,
            "created_at": self.created_at,
        }))
        .expect("identity fields are finite")
    }

    pub fn from_record(record: &Value, instructions: &str) -> Option<Self> {
        if record.get("schema")?.as_str()? != "agent-identity/v1"
            || !canonical::verify_sealed(record)
        {
            return None;
        }
        let id = record.get("agent_id")?.as_str()?;
        if !valid_agent_id(id) {
            return None;
        }
        let capabilities = record
            .get("capabilities")?
            .as_array()?
            .iter()
            .map(|c| c.as_str().and_then(Capability::parse))
            .collect::<Option<BTreeSet<_>>>()?;
        Some(Self {
            id: id.to_owned(),
            display_name: record.get("display_name")?.as_str()?.to_owned(),
            role: record.get("role")?.as_str()?.to_owned(),
            capabilities,
            revision: u32::try_from(record.get("revision")?.as_u64()?).ok()?,
            created_at: record.get("created_at")?.as_str()?.to_owned(),
            instructions: instructions.to_owned(),
        })
    }
}

/// `^[a-z][a-z0-9-]{1,63}$`, the contract's agent id.
pub fn valid_agent_id(id: &str) -> bool {
    let b = id.as_bytes();
    (2..=64).contains(&b.len())
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// James's built-in team.
pub fn default_team(created_at: &str) -> Vec<AgentProfile> {
    use Capability::*;
    let make =
        |id: &str, name: &str, role: &str, caps: &[Capability], instructions: &str| AgentProfile {
            id: id.into(),
            display_name: name.into(),
            role: role.into(),
            capabilities: caps.iter().copied().collect(),
            revision: 1,
            created_at: created_at.into(),
            instructions: instructions.into(),
        };
    vec![
        make(
            "coordinator",
            "Coordinator",
            "Plans the work, hands parts to the team and assembles the result",
            &[Delegate, Draft, Remember],
            "Break the person's request into parts. Delegate reading to the researcher, writing to the writer and checking to the reviewer. Finish with a short result that says what was done and what still needs the person.",
        ),
        make(
            "researcher",
            "Researcher",
            "Finds and quotes what the workspace files actually say",
            &[ReadWorkspace, Draft, Remember],
            "Read only what you need. Quote exact lines and cite each as [path:line]. Never state something the files do not say.",
        ),
        make(
            "writer",
            "Writer",
            "Drafts text from the researcher's findings and proposes files",
            &[Draft, ProposeWrite, Remember],
            "Write plainly. Keep every citation you were given. A file you propose waits for the person's approval.",
        ),
        make(
            "reviewer",
            "Reviewer",
            "Checks drafts against their sources before the person sees them",
            &[ReadWorkspace, Review, Draft],
            "Use verify_citations on the draft. Report each citation that fails. Do not rewrite the draft.",
        ),
    ]
}
