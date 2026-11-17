//! The team's append-only journal: one canonical `agent-event/v1` record per
//! line, each naming the previous record's digest. Every piece of team state
//! (agents, tasks, memory, approvals, routines) is a fold over this file;
//! nothing else is authoritative.
//!
//! Opening verifies the whole chain. A final line without its newline is a
//! write the process did not finish: it is moved aside to
//! `journal.jsonl.torn-<n>` and the journal continues from the last complete
//! record. Any other damage refuses to open, so a broken history is never
//! silently continued. Each append is flushed to disk before it returns.

use crate::Generation;
use crate::canonical::{self, sha256_hex};
use crate::clock::{Millis, rfc3339};
use serde_json::{Map, Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const EVENT_SCHEMA: &str = "agent-event/v1";
pub const SYSTEM: &str = "james-desktop";
/// The most a single journal line may hold.
pub const MAX_LINE: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub seq: u64,
    pub at: String,
    pub actor: String,
    pub kind: String,
    pub task_id: Option<String>,
    pub generation: Generation,
    pub data: Value,
    pub prev: Option<String>,
    pub sha256: String,
}

impl Event {
    pub fn to_value(&self) -> Value {
        json!({
            "schema": EVENT_SCHEMA,
            "system": SYSTEM,
            "seq": self.seq,
            "at": self.at,
            "actor": self.actor,
            "kind": self.kind,
            "task_id": self.task_id,
            "generation": self.generation.as_str(),
            "data": self.data,
            "prev": self.prev,
            "sha256": self.sha256,
        })
    }

    fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        if o.get("schema")?.as_str()? != EVENT_SCHEMA || o.get("system")?.as_str()? != SYSTEM {
            return None;
        }
        Some(Self {
            seq: o.get("seq")?.as_u64()?,
            at: o.get("at")?.as_str()?.to_owned(),
            actor: o.get("actor")?.as_str()?.to_owned(),
            kind: o.get("kind")?.as_str()?.to_owned(),
            task_id: match o.get("task_id")? {
                Value::Null => None,
                Value::String(s) => Some(s.clone()),
                _ => return None,
            },
            generation: Generation::parse(o.get("generation")?.as_str()?)?,
            data: o.get("data").filter(|d| d.is_object())?.clone(),
            prev: match o.get("prev")? {
                Value::Null => None,
                Value::String(s) => Some(s.clone()),
                _ => return None,
            },
            sha256: o.get("sha256")?.as_str()?.to_owned(),
        })
    }
}

#[derive(Debug)]
pub enum JournalError {
    Io(io::Error),
    /// The record at this line does not parse, link or match its digest.
    Corrupt {
        line: usize,
        reason: &'static str,
    },
    Canonical(canonical::CanonicalError),
    TooLarge,
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "journal I/O: {e}"),
            Self::Corrupt { line, reason } => write!(f, "journal line {line}: {reason}"),
            Self::Canonical(e) => write!(f, "journal record: {e}"),
            Self::TooLarge => f.write_str("journal record is too large"),
        }
    }
}
impl std::error::Error for JournalError {}
impl From<io::Error> for JournalError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// What opening found, for the person to see.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenReport {
    pub records: usize,
    /// Where an unfinished final write was moved, if there was one.
    pub torn_tail: Option<PathBuf>,
}

pub struct Journal {
    path: PathBuf,
    file: Option<File>,
    events: Vec<Event>,
}

/// Input for one new record. The journal assigns `seq`, `prev` and `sha256`.
pub struct Draft<'a> {
    pub at: Millis,
    pub actor: &'a str,
    pub kind: &'a str,
    pub task_id: Option<&'a str>,
    pub generation: Generation,
    pub data: Value,
}

impl Journal {
    /// An in-memory journal for tests and dry runs; nothing touches disk.
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::new(),
            file: None,
            events: Vec::new(),
        }
    }

    pub fn open(path: &Path) -> Result<(Self, OpenReport), JournalError> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut report = OpenReport::default();
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        let complete = match bytes.iter().rposition(|b| *b == b'\n') {
            Some(i) => i + 1,
            None => 0,
        };
        if complete < bytes.len() {
            let mut n = 1;
            let torn = loop {
                let candidate = path.with_extension(format!("jsonl.torn-{n}"));
                if !candidate.exists() {
                    break candidate;
                }
                n += 1;
            };
            fs::write(&torn, &bytes[complete..])?;
            let f = OpenOptions::new().write(true).open(path)?;
            f.set_len(complete as u64)?;
            f.sync_all()?;
            report.torn_tail = Some(torn);
        }
        let mut events = Vec::new();
        let mut prev: Option<String> = None;
        for (index, line) in bytes[..complete].split(|b| *b == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let line_no = index + 1;
            let corrupt = |reason| JournalError::Corrupt {
                line: line_no,
                reason,
            };
            let value: Value = serde_json::from_slice(line).map_err(|_| corrupt("not JSON"))?;
            let event = Event::from_value(&value).ok_or(corrupt("not an agent-event/v1 record"))?;
            if event.seq != events.len() as u64 + 1 {
                return Err(corrupt("sequence number out of order"));
            }
            if event.prev != prev {
                return Err(corrupt("does not link to the previous record"));
            }
            let canonical_text = canonical::canonical(&value).map_err(JournalError::Canonical)?;
            if canonical_text.as_bytes() != line {
                return Err(corrupt("not written in canonical form"));
            }
            if !canonical::verify_sealed(&value) {
                return Err(corrupt("digest does not match the record"));
            }
            prev = Some(event.sha256.clone());
            events.push(event);
        }
        report.records = events.len();
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok((
            Self {
                path: path.to_owned(),
                file: Some(file),
                events,
            },
            report,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn head(&self) -> Option<&str> {
        self.events.last().map(|e| e.sha256.as_str())
    }

    /// Seals, writes and flushes one record, then returns it.
    pub fn append(&mut self, draft: Draft<'_>) -> Result<&Event, JournalError> {
        let mut body = Map::new();
        body.insert("schema".into(), EVENT_SCHEMA.into());
        body.insert("system".into(), SYSTEM.into());
        body.insert("seq".into(), (self.events.len() as u64 + 1).into());
        body.insert("at".into(), rfc3339(draft.at).into());
        body.insert("actor".into(), draft.actor.into());
        body.insert("kind".into(), draft.kind.into());
        body.insert(
            "task_id".into(),
            draft.task_id.map(Value::from).unwrap_or(Value::Null),
        );
        body.insert("generation".into(), draft.generation.as_str().into());
        body.insert(
            "data".into(),
            if draft.data.is_object() {
                draft.data
            } else {
                json!({ "value": draft.data })
            },
        );
        body.insert(
            "prev".into(),
            self.head()
                .map(|h| Value::String(h.to_owned()))
                .unwrap_or(Value::Null),
        );
        let sealed = canonical::seal(Value::Object(body)).map_err(JournalError::Canonical)?;
        let line = canonical::canonical(&sealed).map_err(JournalError::Canonical)?;
        if line.len() > MAX_LINE {
            return Err(JournalError::TooLarge);
        }
        if let Some(file) = self.file.as_mut() {
            let mut bytes = line.into_bytes();
            bytes.push(b'\n');
            file.write_all(&bytes)?;
            file.sync_data()?;
        }
        let event = Event::from_value(&sealed).expect("a record this journal sealed parses");
        self.events.push(event);
        Ok(self.events.last().expect("just pushed"))
    }

    /// Recomputes every link and digest of the records in memory.
    pub fn verify(&self) -> Result<(), JournalError> {
        let mut prev: Option<&str> = None;
        for (i, e) in self.events.iter().enumerate() {
            let corrupt = |reason| JournalError::Corrupt {
                line: i + 1,
                reason,
            };
            if e.prev.as_deref() != prev || e.seq != i as u64 + 1 {
                return Err(corrupt("does not link"));
            }
            if !canonical::verify_sealed(&e.to_value()) {
                return Err(corrupt("digest does not match the record"));
            }
            prev = Some(&e.sha256);
        }
        Ok(())
    }
}

/// SHA-256 of text, for naming content without storing it.
pub fn text_digest(text: &str) -> String {
    sha256_hex(text.as_bytes())
}
