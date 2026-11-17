//! Scoped, inspectable memory.
//!
//! An entry belongs to one agent, to one task tree, or to the whole team.
//! The host decides which entries a prompt may include; an agent cannot name
//! a scope it does not hold. Provenance (who wrote it, in which task, from
//! which files) is filled in by the host from what actually happened, not
//! from what a model claims.
//!
//! The words themselves live in content-addressed files beside the journal.
//! **Forget** deletes those bytes; the journal keeps a tombstone that says an
//! entry existed and was forgotten. Text a model already wrote while the
//! entry was visible stays in that task's record.

use crate::Generation;
use crate::canonical::{self, sha256_hex};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

pub const MAX_MEMORY_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Agent(String),
    Task(String),
    Team,
}

impl Scope {
    pub fn to_value(&self) -> Value {
        match self {
            Self::Agent(a) => json!({ "kind": "agent", "agent_id": a }),
            Self::Task(t) => json!({ "kind": "task", "task_id": t }),
            Self::Team => json!({ "kind": "team" }),
        }
    }
    pub fn from_value(v: &Value) -> Option<Self> {
        match v.get("kind")?.as_str()? {
            "agent" => Some(Self::Agent(v.get("agent_id")?.as_str()?.to_owned())),
            "task" => Some(Self::Task(v.get("task_id")?.as_str()?.to_owned())),
            "team" => Some(Self::Team),
            _ => None,
        }
    }
    pub fn plain(&self) -> String {
        match self {
            Self::Agent(a) => format!("only {a}"),
            Self::Task(t) => format!("task {t} and its sub-tasks"),
            Self::Team => "the whole team".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// `file`, `run`, `record`, `message` or `person`.
    pub kind: String,
    pub reference: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    pub id: String,
    pub scope: Scope,
    pub content_sha256: String,
    pub content_bytes: usize,
    pub author: String,
    pub task_id: Option<String>,
    pub generation: Generation,
    pub sources: Vec<Source>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub forgotten: bool,
}

impl MemoryEntry {
    /// The sealed `agent-memory/v1` record.
    pub fn record(&self) -> Value {
        let sources: Vec<Value> = self
            .sources
            .iter()
            .map(|s| json!({ "kind": s.kind, "ref": s.reference, "sha256": s.sha256 }))
            .collect();
        canonical::seal(json!({
            "schema": "agent-memory/v1",
            "system": "james-desktop",
            "memory_id": self.id,
            "scope": self.scope.to_value(),
            "content_sha256": self.content_sha256,
            "content_bytes": self.content_bytes,
            "provenance": {
                "author": self.author,
                "task_id": self.task_id,
                "generation": self.generation.as_str(),
                "sources": sources,
            },
            "supersedes": self.supersedes,
            "created_at": self.created_at,
        }))
        .expect("memory fields are finite")
    }

    pub fn from_record(v: &Value) -> Option<Self> {
        if v.get("schema")?.as_str()? != "agent-memory/v1" || !canonical::verify_sealed(v) {
            return None;
        }
        let p = v.get("provenance")?;
        let sources = p
            .get("sources")?
            .as_array()?
            .iter()
            .map(|s| {
                Some(Source {
                    kind: s.get("kind")?.as_str()?.to_owned(),
                    reference: s.get("ref")?.as_str()?.to_owned(),
                    sha256: s.get("sha256")?.as_str().map(str::to_owned),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            id: v.get("memory_id")?.as_str()?.to_owned(),
            scope: Scope::from_value(v.get("scope")?)?,
            content_sha256: v.get("content_sha256")?.as_str()?.to_owned(),
            content_bytes: v.get("content_bytes")?.as_u64()? as usize,
            author: p.get("author")?.as_str()?.to_owned(),
            task_id: p.get("task_id")?.as_str().map(str::to_owned),
            generation: Generation::parse(p.get("generation")?.as_str()?)?,
            sources,
            supersedes: v.get("supersedes")?.as_str().map(str::to_owned),
            superseded_by: None,
            created_at: v.get("created_at")?.as_str()?.to_owned(),
            forgotten: false,
        })
    }

    /// Whether an agent working inside `task_root` may see this entry.
    pub fn visible_to(&self, agent: &str, task_root: &str) -> bool {
        !self.forgotten
            && self.superseded_by.is_none()
            && match &self.scope {
                Scope::Agent(a) => a == agent,
                Scope::Task(t) => t == task_root,
                Scope::Team => true,
            }
    }
}

/// Content-addressed text, on disk beside the journal or in memory for tests.
pub struct BlobStore {
    dir: Option<PathBuf>,
    memory: HashMap<String, String>,
}

impl BlobStore {
    pub fn on_disk(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        Ok(Self {
            dir: Some(dir),
            memory: HashMap::new(),
        })
    }
    pub fn in_memory() -> Self {
        Self {
            dir: None,
            memory: HashMap::new(),
        }
    }
    fn path(&self, sha: &str) -> Option<PathBuf> {
        let ok = sha.len() == 64
            && sha
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
        self.dir
            .as_ref()
            .filter(|_| ok)
            .map(|d| d.join(format!("{sha}.txt")))
    }
    pub fn put(&mut self, text: &str) -> io::Result<String> {
        let sha = sha256_hex(text.as_bytes());
        match self.path(&sha) {
            Some(path) if !path.exists() => {
                let tmp = path.with_extension("tmp");
                fs::write(&tmp, text)?;
                fs::rename(&tmp, &path)?;
            }
            Some(_) => {}
            None => {
                self.memory.insert(sha.clone(), text.to_owned());
            }
        }
        Ok(sha)
    }
    /// The text, only if its bytes still match their name.
    pub fn get(&self, sha: &str) -> Option<String> {
        let text = match self.path(sha) {
            Some(path) => fs::read_to_string(path).ok()?,
            None if self.dir.is_none() => self.memory.get(sha)?.clone(),
            None => return None,
        };
        (sha256_hex(text.as_bytes()) == sha).then_some(text)
    }
    pub fn delete(&mut self, sha: &str) -> io::Result<()> {
        match self.path(sha) {
            Some(path) => match fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            },
            None => {
                self.memory.remove(sha);
                Ok(())
            }
        }
    }
}
