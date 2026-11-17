//! The team's supervisor.
//!
//! All state is a fold over the journal ([`State::apply`]); every change goes
//! through [`Runtime::emit`], which appends a record and then applies it, so
//! what is on screen and what is on disk cannot disagree.
//!
//! Work advances one bounded unit per [`Runtime::step`]: one model call and
//! the one action it names. The desktop drives steps from a worker thread;
//! tests drive them directly. A step never waits for a person: an action that
//! needs one (a file write) parks its task until [`Runtime::decide`].
//!
//! Recovery is honest rather than optimistic. When the journal is reopened, a
//! task that was mid-step is marked **interrupted** and waits for the person
//! to resume it; queued work continues; an approved write that may or may not
//! have reached the disk is reconciled by reading the file, and when the file
//! is neither the old nor the new content the outcome is **unknown** and the
//! person is told. Nothing with an effect is retried automatically. (The
//! "report interrupted work, never replay it" rule follows SuperBot's
//! `runner.rs`, MIT; this is an independent implementation.)

use crate::canonical::{self, sha256_hex};
use crate::clock::{Millis, parse_rfc3339, rfc3339};
use crate::identity::{AgentProfile, Capability, default_team};
use crate::journal::{Draft, Event, Journal, JournalError, OpenReport};
use crate::memory::{BlobStore, MemoryEntry, Scope, Source};
use crate::model::{CancelSignal, ModelPort, ModelRequest};
use crate::protocol::{self, Action, MemoryChoice, Observation};
use crate::schedule::{Cron, Routine};
use crate::tools::{Workspace, clean_relative};
use crate::{Generation, clip, one_line};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How many levels of delegation a root task may have below it.
    pub max_depth: u32,
    /// How many sub-tasks one task may hand out.
    pub max_children: usize,
    /// Model calls a root task may use, including its sub-tasks' shares.
    pub root_steps: u32,
    /// Wall-clock life of a root task.
    pub deadline_ms: i64,
    /// How long a pending approval stays open.
    pub approval_ttl_ms: i64,
    /// Unusable replies before a task fails.
    pub max_invalid: u32,
    /// Consecutive model errors before a task fails.
    pub max_model_failures: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_children: 4,
            root_steps: 40,
            deadline_ms: 30 * 60_000,
            approval_ttl_ms: 15 * 60_000,
            max_invalid: 3,
            max_model_failures: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    AwaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "awaiting_approval" => Self::AwaitingApproval,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Waiting {
    Children,
    Approval(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Turn {
    Action {
        generation: Generation,
        action: Value,
    },
    Observation(Observation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub max_depth: u32,
    pub max_steps: u32,
    pub deadline_ms: Option<Millis>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub parent: Option<String>,
    pub root: String,
    pub depth: u32,
    pub created_by: String,
    pub assignee: String,
    pub objective: String,
    pub budget: Budget,
    /// Steps handed to sub-tasks and not yet returned.
    pub reserved: u32,
    pub steps_used: u32,
    pub invalid_replies: u32,
    pub model_failures: u32,
    pub last_error: Option<String>,
    pub state: TaskState,
    pub waiting: Option<Waiting>,
    pub children: Vec<String>,
    pub transcript: Vec<Turn>,
    /// Files this task read, as the host observed them.
    pub sources: Vec<Source>,
    pub result: Option<String>,
    pub created_at: Millis,
    pub finished_at: Option<Millis>,
    /// Generation of the words in `result`.
    pub result_generation: Option<Generation>,
}

impl Task {
    pub fn steps_left(&self) -> u32 {
        self.budget
            .max_steps
            .saturating_sub(self.steps_used + self.reserved)
    }

    /// The sealed `agent-task/v1` record of this task as it stands.
    pub fn record(&self) -> Value {
        let state = match (self.state, &self.waiting) {
            (TaskState::Running, Some(Waiting::Approval(_))) => TaskState::AwaitingApproval,
            (s, _) => s,
        };
        canonical::seal(json!({
            "schema": "agent-task/v1",
            "system": "james-desktop",
            "task_id": self.id,
            "parent_task_id": self.parent,
            "root_task_id": self.root,
            "depth": self.depth,
            "created_by": self.created_by,
            "assignee": self.assignee,
            "objective": self.objective,
            "budget": {
                "max_depth": self.budget.max_depth,
                "max_steps": self.budget.max_steps,
                "max_model_calls": self.budget.max_steps,
                "max_tokens": null,
                "deadline": self.budget.deadline_ms.map(rfc3339),
            },
            "state": state.as_str(),
            "created_at": rfc3339(self.created_at),
        }))
        .expect("task fields are finite")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Approval {
    pub id: String,
    pub task_id: String,
    pub requested_by: String,
    pub path: String,
    pub content_sha256: String,
    pub previous_sha256: Option<String>,
    pub action_sha256: String,
    pub expires_at: Millis,
    pub record: Value,
    pub verdict: Option<String>,
    pub applying: bool,
    /// `applied`, `not_applied`, `denied`, `expired`, `voided` or `unknown`.
    pub outcome: Option<String>,
    pub detail: Option<String>,
}

impl Approval {
    pub fn is_pending(&self) -> bool {
        self.verdict.is_none() && self.outcome.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Working(String),
    WaitingOnTeam(String),
    WaitingOnPerson(String),
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub agents: BTreeMap<String, AgentProfile>,
    pub tasks: BTreeMap<String, Task>,
    pub memory: BTreeMap<String, MemoryEntry>,
    pub approvals: BTreeMap<String, Approval>,
    pub routines: BTreeMap<String, Routine>,
    pub workspace: Option<PathBuf>,
    pub last_event_at: Option<Millis>,
    /// Records of kinds this version does not know (from a newer James).
    pub unknown_kinds: usize,
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

impl State {
    /// Applies one journal record. Pure: the same records always give the same state.
    pub fn apply(&mut self, e: &Event) {
        let at = parse_rfc3339(&e.at).unwrap_or(0);
        self.last_event_at = Some(at);
        let d = &e.data;
        let task_id = e.task_id.clone().unwrap_or_default();
        match e.kind.as_str() {
            "agent.registered" => {
                if let Some(p) = d
                    .get("identity")
                    .and_then(|r| AgentProfile::from_record(r, str_of(d, "instructions")))
                {
                    self.agents.insert(p.id.clone(), p);
                }
            }
            "workspace.chosen" => self.workspace = Some(PathBuf::from(str_of(d, "path"))),
            "task.created" => {
                let r = &d["task"];
                let budget = &r["budget"];
                let task = Task {
                    id: str_of(r, "task_id").into(),
                    parent: r["parent_task_id"].as_str().map(str::to_owned),
                    root: str_of(r, "root_task_id").into(),
                    depth: r["depth"].as_u64().unwrap_or(0) as u32,
                    created_by: str_of(r, "created_by").into(),
                    assignee: str_of(r, "assignee").into(),
                    objective: str_of(r, "objective").into(),
                    budget: Budget {
                        max_depth: budget["max_depth"].as_u64().unwrap_or(0) as u32,
                        max_steps: budget["max_steps"].as_u64().unwrap_or(1) as u32,
                        deadline_ms: budget["deadline"].as_str().and_then(parse_rfc3339),
                    },
                    reserved: 0,
                    steps_used: 0,
                    invalid_replies: 0,
                    model_failures: 0,
                    last_error: None,
                    state: TaskState::Queued,
                    waiting: None,
                    children: Vec::new(),
                    transcript: Vec::new(),
                    sources: Vec::new(),
                    result: None,
                    created_at: at,
                    finished_at: None,
                    result_generation: None,
                };
                if let Some(parent) = task.parent.as_ref().and_then(|p| self.tasks.get_mut(p)) {
                    parent.children.push(task.id.clone());
                    parent.reserved += task.budget.max_steps;
                }
                self.tasks.insert(task.id.clone(), task);
            }
            "task.started" | "task.ready" => {
                if let Some(t) = self
                    .tasks
                    .get_mut(&task_id)
                    .filter(|t| !t.state.is_terminal())
                {
                    t.state = TaskState::Running;
                    t.waiting = None;
                }
            }
            "task.waiting" => {
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.waiting = Some(Waiting::Children);
                }
            }
            "model.called" | "model.failed" => {
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.steps_used += 1;
                    if e.kind == "model.failed" {
                        t.model_failures += 1;
                        t.last_error = Some(str_of(d, "error").to_owned());
                    } else {
                        t.model_failures = 0;
                    }
                }
            }
            "agent.output" => {
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.transcript.push(Turn::Action {
                        generation: e.generation,
                        action: d["action"].clone(),
                    });
                }
            }
            "agent.output_invalid" => {
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.invalid_replies += 1;
                    t.transcript.push(Turn::Observation(Observation::Refused {
                        reason: format!("Your reply could not be used: {}", str_of(d, "reason")),
                    }));
                }
            }
            "task.observed" => {
                if let (Some(t), Some(o)) = (
                    self.tasks.get_mut(&task_id),
                    Observation::from_value(&d["observation"]),
                ) {
                    t.transcript.push(Turn::Observation(o));
                    if let Some(src) = d.get("source").filter(|s| s.is_object()) {
                        t.sources.push(Source {
                            kind: "file".into(),
                            reference: str_of(src, "path").into(),
                            sha256: src["sha256"].as_str().map(str::to_owned),
                        });
                    }
                }
            }
            "task.finished" | "task.cancelled" => {
                let state = if e.kind == "task.cancelled" {
                    TaskState::Cancelled
                } else {
                    TaskState::parse(str_of(d, "state")).unwrap_or(TaskState::Failed)
                };
                let mut share = None;
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.state = state;
                    t.waiting = None;
                    t.finished_at = Some(at);
                    if e.kind == "task.finished" {
                        t.result = Some(str_of(d, "result").to_owned());
                        t.result_generation = Generation::parse(str_of(d, "result_generation"));
                    }
                    share = t
                        .parent
                        .clone()
                        .map(|p| (p, t.budget.max_steps, t.steps_used.min(t.budget.max_steps)));
                }
                // A finished sub-task's unused share goes back to its parent.
                if let Some((parent, given, used)) = share {
                    if let Some(p) = self.tasks.get_mut(&parent) {
                        p.reserved = p.reserved.saturating_sub(given);
                        p.steps_used += used;
                    }
                }
            }
            "task.interrupted" => {
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.state = TaskState::Interrupted;
                    t.waiting = None;
                }
            }
            "task.resumed" => {
                if let Some(t) = self.tasks.get_mut(&task_id) {
                    t.state = TaskState::Queued;
                    t.waiting = None;
                    t.invalid_replies = 0;
                    t.model_failures = 0;
                }
            }
            "approval.requested" => {
                let r = &d["approval"];
                let a = &r["action"];
                let approval = Approval {
                    id: str_of(r, "request_id").into(),
                    task_id: str_of(r, "task_id").into(),
                    requested_by: str_of(r, "requested_by").into(),
                    path: str_of(a, "path").into(),
                    content_sha256: str_of(a, "content_sha256").into(),
                    previous_sha256: a["previous_sha256"].as_str().map(str::to_owned),
                    action_sha256: str_of(r, "action_sha256").into(),
                    expires_at: r["expires_at"]
                        .as_str()
                        .and_then(parse_rfc3339)
                        .unwrap_or(0),
                    record: r.clone(),
                    verdict: None,
                    applying: false,
                    outcome: None,
                    detail: None,
                };
                if let Some(t) = self.tasks.get_mut(&approval.task_id) {
                    t.state = TaskState::AwaitingApproval;
                    t.waiting = Some(Waiting::Approval(approval.id.clone()));
                }
                self.approvals.insert(approval.id.clone(), approval);
            }
            "approval.decided" => {
                if let Some(a) = self.approvals.get_mut(str_of(d, "request_id")) {
                    a.verdict = Some(str_of(d, "verdict").to_owned());
                    let decision = json!({
                        "by": "person",
                        "verdict": str_of(d, "verdict"),
                        "at": e.at,
                        "action_sha256": str_of(d, "action_sha256"),
                    });
                    if let Value::Object(map) = &mut a.record {
                        map.insert("decision".into(), decision);
                    }
                    a.record = canonical::seal(a.record.clone()).unwrap_or(Value::Null);
                }
            }
            "approval.applying" => {
                if let Some(a) = self.approvals.get_mut(str_of(d, "request_id")) {
                    a.applying = true;
                }
            }
            "approval.closed" => {
                if let Some(a) = self.approvals.get_mut(str_of(d, "request_id")) {
                    a.outcome = Some(str_of(d, "outcome").to_owned());
                    a.detail = Some(str_of(d, "detail").to_owned());
                    let tid = a.task_id.clone();
                    let id = a.id.clone();
                    if let Some(t) = self.tasks.get_mut(&tid) {
                        if t.waiting == Some(Waiting::Approval(id)) && !t.state.is_terminal() {
                            t.waiting = None;
                            if t.state == TaskState::AwaitingApproval {
                                t.state = TaskState::Running;
                            }
                        }
                    }
                }
            }
            "memory.written" => {
                if let Some(entry) = MemoryEntry::from_record(&d["memory"]) {
                    if let Some(old) = entry
                        .supersedes
                        .as_ref()
                        .and_then(|s| self.memory.get_mut(s))
                    {
                        old.superseded_by = Some(entry.id.clone());
                    }
                    self.memory.insert(entry.id.clone(), entry);
                }
            }
            "memory.forgotten" => {
                if let Some(m) = self.memory.get_mut(str_of(d, "memory_id")) {
                    m.forgotten = true;
                }
            }
            "routine.saved" => {
                if let Ok(cron) = Cron::parse(str_of(d, "cron")) {
                    let id = str_of(d, "routine_id").to_owned();
                    let last = self.routines.get(&id).and_then(|r| r.last_fired_minute);
                    self.routines.insert(
                        id.clone(),
                        Routine {
                            id,
                            name: str_of(d, "name").into(),
                            cron,
                            utc_offset_minutes: d["utc_offset_minutes"].as_i64().unwrap_or(0)
                                as i32,
                            assignee: str_of(d, "assignee").into(),
                            objective: str_of(d, "objective").into(),
                            enabled: d["enabled"].as_bool().unwrap_or(true),
                            created_at: at,
                            last_fired_minute: last,
                        },
                    );
                }
            }
            "routine.removed" => {
                self.routines.remove(str_of(d, "routine_id"));
            }
            "routine.fired" => {
                if let Some(r) = self.routines.get_mut(str_of(d, "routine_id")) {
                    r.last_fired_minute = d["minute"].as_i64();
                }
            }
            "routine.missed" | "runtime.opened" => {}
            _ => self.unknown_kinds += 1,
        }
    }

    /// What each assistant is doing now.
    pub fn activity(&self) -> Vec<(String, Activity)> {
        self.agents
            .keys()
            .map(|agent| {
                let mine: Vec<&Task> = self
                    .tasks
                    .values()
                    .filter(|t| &t.assignee == agent && !t.state.is_terminal())
                    .collect();
                let activity = if let Some(t) =
                    mine.iter().find(|t| t.state == TaskState::AwaitingApproval)
                {
                    Activity::WaitingOnPerson(t.id.clone())
                } else if let Some(t) = mine.iter().find(|t| t.state == TaskState::Interrupted) {
                    Activity::WaitingOnPerson(t.id.clone())
                } else if let Some(t) = mine.iter().find(|t| t.waiting == Some(Waiting::Children)) {
                    Activity::WaitingOnTeam(t.id.clone())
                } else if let Some(t) = mine.first() {
                    Activity::Working(t.id.clone())
                } else {
                    Activity::Idle
                };
                (agent.clone(), activity)
            })
            .collect()
    }

    /// A task and every task below it, depth first.
    pub fn subtree(&self, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![id.to_owned()];
        while let Some(next) = stack.pop() {
            if let Some(t) = self.tasks.get(&next) {
                out.push(next);
                stack.extend(t.children.iter().rev().cloned());
            }
        }
        out
    }

    fn next_id(&self, prefix: &str, count: usize) -> String {
        format!("{prefix}-{:06}", count + 1)
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    Journal(JournalError),
    Refused(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Journal(e) => write!(f, "{e}"),
            Self::Refused(r) => f.write_str(r),
        }
    }
}
impl std::error::Error for RuntimeError {}
impl From<JournalError> for RuntimeError {
    fn from(e: JournalError) -> Self {
        Self::Journal(e)
    }
}

fn refused<T>(reason: impl Into<String>) -> Result<T, RuntimeError> {
    Err(RuntimeError::Refused(reason.into()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    /// Nothing can run: every task is finished or waiting for the person.
    Idle,
    /// One unit of work was done on this task.
    Progress(String),
    /// The step's model call was cut by a cancellation.
    Cancelled(String),
}

/// What reopening found and did, for the person.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recovery {
    pub journal: OpenReport,
    pub interrupted: Vec<String>,
    pub reconciled: Vec<(String, String)>,
    pub missed_routines: Vec<(String, usize)>,
}

pub struct Runtime {
    journal: Journal,
    blobs: BlobStore,
    workspace: Option<Workspace>,
    state: State,
    limits: Limits,
    cancel: CancelSignal,
}

impl Runtime {
    /// Opens (or starts) the team stored in `dir`, then recovers.
    pub fn open(
        dir: &Path,
        workspace: Option<&Path>,
        now: Millis,
    ) -> Result<(Self, Recovery), RuntimeError> {
        let (journal, report) = Journal::open(&dir.join("journal.jsonl"))?;
        let blobs = BlobStore::on_disk(dir.join("blobs"))
            .map_err(|e| RuntimeError::Journal(JournalError::Io(e)))?;
        let mut runtime = Self::from_parts(journal, blobs);
        let mut recovery = Recovery {
            journal: report,
            ..Default::default()
        };
        runtime.choose_workspace_or_restore(workspace, now)?;
        runtime.recover(now, &mut recovery)?;
        Ok((runtime, recovery))
    }

    /// A team that lives only in memory (tests, dry runs).
    pub fn in_memory(workspace: Option<&Path>, now: Millis) -> Result<Self, RuntimeError> {
        let mut runtime = Self::from_parts(Journal::in_memory(), BlobStore::in_memory());
        runtime.choose_workspace_or_restore(workspace, now)?;
        let mut recovery = Recovery::default();
        runtime.recover(now, &mut recovery)?;
        Ok(runtime)
    }

    fn from_parts(journal: Journal, blobs: BlobStore) -> Self {
        let mut state = State::default();
        for e in journal.events() {
            state.apply(e);
        }
        Self {
            journal,
            blobs,
            workspace: None,
            state,
            limits: Limits::default(),
            cancel: CancelSignal::default(),
        }
    }

    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn events(&self) -> &[Event] {
        self.journal.events()
    }
    pub fn journal_path(&self) -> &Path {
        self.journal.path()
    }
    pub fn verify(&self) -> Result<(), JournalError> {
        self.journal.verify()
    }
    pub fn cancel_signal(&self) -> CancelSignal {
        self.cancel.clone()
    }
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }
    /// Stored text by digest (memory, proposed file content).
    pub fn blob(&self, sha: &str) -> Option<String> {
        self.blobs.get(sha)
    }
    pub fn memory_text(&self, id: &str) -> Option<String> {
        self.state
            .memory
            .get(id)
            .filter(|m| !m.forgotten)
            .and_then(|m| self.blobs.get(&m.content_sha256))
    }

    fn emit(
        &mut self,
        at: Millis,
        actor: &str,
        kind: &str,
        task_id: Option<&str>,
        generation: Generation,
        data: Value,
    ) -> Result<(), RuntimeError> {
        let event = self
            .journal
            .append(Draft {
                at,
                actor,
                kind,
                task_id,
                generation,
                data,
            })?
            .clone();
        self.state.apply(&event);
        Ok(())
    }

    fn observe(
        &mut self,
        task: &str,
        at: Millis,
        observation: Observation,
        source: Option<Value>,
    ) -> Result<(), RuntimeError> {
        let mut data = json!({ "observation": observation.to_value() });
        if let Some(s) = source {
            data["source"] = s;
        }
        self.emit(
            at,
            "host",
            "task.observed",
            Some(task),
            Generation::Host,
            data,
        )
    }

    fn choose_workspace_or_restore(
        &mut self,
        workspace: Option<&Path>,
        now: Millis,
    ) -> Result<(), RuntimeError> {
        if self.state.agents.is_empty() {
            for profile in default_team(&rfc3339(now)) {
                let data = json!({ "identity": profile.identity_record(), "instructions": profile.instructions });
                self.emit(
                    now,
                    "host",
                    "agent.registered",
                    None,
                    Generation::Host,
                    data,
                )?;
            }
        }
        match workspace {
            Some(path) => self.set_workspace(path, now)?,
            None => {
                self.workspace = self
                    .state
                    .workspace
                    .as_deref()
                    .and_then(|p| Workspace::new(p).ok());
            }
        }
        Ok(())
    }

    /// Adds an assistant, or records a new revision of one. Capabilities come
    /// only from here, the person's side; nothing an agent writes reaches it.
    pub fn register(&mut self, mut profile: AgentProfile, now: Millis) -> Result<(), RuntimeError> {
        if !crate::identity::valid_agent_id(&profile.id) {
            return refused("an assistant id is 2 to 64 lowercase letters, digits or dashes");
        }
        profile.revision = self
            .state
            .agents
            .get(&profile.id)
            .map(|p| p.revision + 1)
            .unwrap_or(1);
        profile.created_at = rfc3339(now);
        let data =
            json!({ "identity": profile.identity_record(), "instructions": profile.instructions });
        self.emit(
            now,
            "person",
            "agent.registered",
            None,
            Generation::Person,
            data,
        )
    }

    /// Chooses the folder assistants may read and propose writes in.
    pub fn set_workspace(&mut self, path: &Path, now: Millis) -> Result<(), RuntimeError> {
        let ws = Workspace::new(path).map_err(|e| RuntimeError::Refused(e.0))?;
        let root = ws.root().to_string_lossy().into_owned();
        if self.state.workspace.as_deref() != Some(ws.root()) {
            self.emit(
                now,
                "person",
                "workspace.chosen",
                None,
                Generation::Person,
                json!({ "path": root }),
            )?;
        }
        self.workspace = Some(ws);
        Ok(())
    }

    fn recover(&mut self, now: Millis, recovery: &mut Recovery) -> Result<(), RuntimeError> {
        let mid_step: Vec<String> = self
            .state
            .tasks
            .values()
            .filter(|t| t.state == TaskState::Running && t.waiting.is_none())
            .map(|t| t.id.clone())
            .collect();
        for id in mid_step {
            let reason = "James stopped while this task was working. Nothing was retried; resume it if you still want it.";
            self.emit(
                now,
                "host",
                "task.interrupted",
                Some(&id),
                Generation::Host,
                json!({ "reason": reason }),
            )?;
            recovery.interrupted.push(id);
        }
        let applying: Vec<Approval> = self
            .state
            .approvals
            .values()
            .filter(|a| a.applying && a.outcome.is_none())
            .cloned()
            .collect();
        for a in applying {
            let current = self
                .workspace
                .as_ref()
                .and_then(|ws| ws.digest_of(&a.path).ok());
            let (outcome, detail) = match current {
                Some(Some(c)) if c == a.content_sha256 => (
                    "applied",
                    "The approved content was written before James stopped; confirmed by reading the file.",
                ),
                Some(c) if c == a.previous_sha256 => (
                    "not_applied",
                    "James stopped before writing. Nothing was written; ask again if you still want it.",
                ),
                _ => (
                    "unknown",
                    "James stopped while writing and the file is neither the old nor the approved content. Check it yourself.",
                ),
            };
            self.close_approval(&a, outcome, detail, now)?;
            recovery.reconciled.push((a.id.clone(), outcome.to_owned()));
        }
        if let Some(last) = self.state.last_event_at {
            let routines: Vec<Routine> = self
                .state
                .routines
                .values()
                .filter(|r| r.enabled)
                .cloned()
                .collect();
            for r in routines {
                let after = last
                    .max(r.created_at)
                    .max(r.last_fired_minute.map(|m| m * 60_000).unwrap_or(0));
                let due = r
                    .cron
                    .due_between(after, now - 60_000, r.utc_offset_minutes, 10_000);
                if let (Some(first), Some(latest)) = (due.first(), due.last()) {
                    self.emit(
                        now,
                        "host",
                        "routine.missed",
                        None,
                        Generation::Host,
                        json!({ "routine_id": r.id, "count": due.len(), "first": rfc3339(*first), "last": rfc3339(*latest) }),
                    )?;
                    recovery.missed_routines.push((r.id.clone(), due.len()));
                }
            }
        }
        self.expire_approvals(now)?;
        let data = json!({
            "records": recovery.journal.records,
            "torn_tail": recovery.journal.torn_tail.as_ref().map(|p| p.to_string_lossy().into_owned()),
            "interrupted": recovery.interrupted,
        });
        self.emit(now, "host", "runtime.opened", None, Generation::Host, data)
    }

    // ── what the person does ─────────────────────────────────────────────

    /// Gives a new task to one assistant. Returns its id.
    pub fn submit(
        &mut self,
        objective: &str,
        assignee: &str,
        now: Millis,
    ) -> Result<String, RuntimeError> {
        if objective.trim().is_empty() || objective.chars().count() > protocol::MAX_OBJECTIVE {
            return refused("describe the task in 1 to 2000 characters");
        }
        if !self.state.agents.contains_key(assignee) {
            return refused(format!("there is no assistant called {assignee}"));
        }
        let budget = Budget {
            max_depth: self.limits.max_depth,
            max_steps: self.limits.root_steps,
            deadline_ms: Some(now + self.limits.deadline_ms),
        };
        self.create_task(None, "person", assignee, objective, budget, now)
    }

    fn create_task(
        &mut self,
        parent: Option<&Task>,
        created_by: &str,
        assignee: &str,
        objective: &str,
        budget: Budget,
        now: Millis,
    ) -> Result<String, RuntimeError> {
        let id = self.state.next_id("T", self.state.tasks.len());
        let (root, depth) = match parent {
            Some(p) => (p.root.clone(), p.depth + 1),
            None => (id.clone(), 0),
        };
        let task = Task {
            id: id.clone(),
            parent: parent.map(|p| p.id.clone()),
            root,
            depth,
            created_by: created_by.into(),
            assignee: assignee.into(),
            objective: objective.into(),
            budget,
            reserved: 0,
            steps_used: 0,
            invalid_replies: 0,
            model_failures: 0,
            last_error: None,
            state: TaskState::Queued,
            waiting: None,
            children: vec![],
            transcript: vec![],
            sources: vec![],
            result: None,
            created_at: now,
            finished_at: None,
            result_generation: None,
        };
        let generation = if created_by == "person" {
            Generation::Person
        } else {
            Generation::Host
        };
        self.emit(
            now,
            created_by,
            "task.created",
            Some(&id),
            generation,
            json!({ "task": task.record() }),
        )?;
        Ok(id)
    }

    /// Stops a task and everything below it. An in-flight model call for one
    /// of them is cut at once.
    pub fn cancel(&mut self, task_id: &str, now: Millis) -> Result<(), RuntimeError> {
        if !self.state.tasks.contains_key(task_id) {
            return refused("no such task");
        }
        self.cancel.request(self.state.subtree(task_id));
        self.process_cancellations(now)
    }

    /// Records cancellations requested through the [`CancelSignal`] from
    /// another thread, without running a step.
    pub fn step_cancellations(&mut self, now: Millis) -> Result<(), RuntimeError> {
        self.process_cancellations(now)
    }

    fn process_cancellations(&mut self, now: Millis) -> Result<(), RuntimeError> {
        let requested = self.cancel.take_requested();
        let mut tops = Vec::new();
        for id in &requested {
            let parent_cancelled = self
                .state
                .tasks
                .get(id)
                .and_then(|t| t.parent.clone())
                .is_some_and(|p| requested.contains(&p));
            for t in self.state.subtree(id) {
                let Some(task) = self.state.tasks.get(&t).cloned() else {
                    continue;
                };
                if task.state.is_terminal() {
                    continue;
                }
                self.void_approvals_of(&t, now)?;
                self.emit(
                    now,
                    "person",
                    "task.cancelled",
                    Some(&t),
                    Generation::Person,
                    json!({ "by": "person" }),
                )?;
            }
            if !parent_cancelled {
                tops.push(id.clone());
            }
        }
        for id in tops {
            self.notify_parent(&id, now)?;
        }
        Ok(())
    }

    fn void_approvals_of(&mut self, task_id: &str, now: Millis) -> Result<(), RuntimeError> {
        let pending: Vec<Approval> = self
            .state
            .approvals
            .values()
            .filter(|a| a.task_id == task_id && a.is_pending())
            .cloned()
            .collect();
        for a in pending {
            self.close_approval(
                &a,
                "voided",
                "The task was cancelled, so its proposal was withdrawn.",
                now,
            )?;
        }
        Ok(())
    }

    /// Puts an interrupted task back in the queue.
    pub fn resume(&mut self, task_id: &str, now: Millis) -> Result<(), RuntimeError> {
        match self.state.tasks.get(task_id) {
            Some(t) if t.state == TaskState::Interrupted => {}
            Some(_) => return refused("only an interrupted task can be resumed"),
            None => return refused("no such task"),
        }
        self.emit(
            now,
            "person",
            "task.resumed",
            Some(task_id),
            Generation::Person,
            json!({ "by": "person" }),
        )?;
        self.observe(
            task_id,
            now,
            Observation::Note {
                text: "The person resumed this task after an interruption.".into(),
            },
            None,
        )
    }

    /// The person's decision on one pending write, bound to the digest of the
    /// exact action they were shown. Returns the outcome.
    pub fn decide(
        &mut self,
        request_id: &str,
        approve: bool,
        shown_action_sha256: &str,
        now: Millis,
    ) -> Result<String, RuntimeError> {
        let Some(a) = self.state.approvals.get(request_id).cloned() else {
            return refused("no such approval request");
        };
        if !a.is_pending() {
            return refused("this request was already decided or closed");
        }
        if now > a.expires_at {
            self.expire_approvals(now)?;
            return refused("this request expired; ask the assistant again");
        }
        if shown_action_sha256 != a.action_sha256 {
            return refused("what you were shown is not what is pending; nothing was decided");
        }
        let verdict = if approve { "approved" } else { "denied" };
        self.emit(
            now,
            "person",
            "approval.decided",
            Some(&a.task_id),
            Generation::Person,
            json!({ "request_id": a.id, "verdict": verdict, "action_sha256": a.action_sha256 }),
        )?;
        if !approve {
            self.close_approval(
                &a,
                "denied",
                "The person declined this change. Nothing was written.",
                now,
            )?;
            return Ok("denied".into());
        }
        self.emit(
            now,
            "person",
            "approval.applying",
            Some(&a.task_id),
            Generation::Person,
            json!({ "request_id": a.id }),
        )?;
        let content = self.blobs.get(&a.content_sha256);
        let written = match (&self.workspace, content) {
            (None, _) => Err("no workspace folder is open".to_owned()),
            (_, None) => Err("the proposed content is missing from James's store".to_owned()),
            (Some(ws), Some(text)) => ws
                .apply_write(&a.path, &text, a.previous_sha256.as_deref())
                .map_err(|e| e.0),
        };
        let (outcome, detail) = match written {
            Ok(sha) => (
                "applied",
                format!("Wrote {} (sha256 {}).", a.path, &sha[..12]),
            ),
            Err(reason) => ("not_applied", format!("Nothing was written: {reason}.")),
        };
        self.close_approval(&a, outcome, &detail, now)?;
        Ok(outcome.into())
    }

    fn close_approval(
        &mut self,
        a: &Approval,
        outcome: &str,
        detail: &str,
        now: Millis,
    ) -> Result<(), RuntimeError> {
        self.emit(
            now,
            "host",
            "approval.closed",
            Some(&a.task_id),
            Generation::Host,
            json!({ "request_id": a.id, "outcome": outcome, "detail": detail }),
        )?;
        let live = self
            .state
            .tasks
            .get(&a.task_id)
            .is_some_and(|t| !t.state.is_terminal());
        if live {
            self.observe(
                &a.task_id,
                now,
                Observation::Approval {
                    request_id: a.id.clone(),
                    outcome: outcome.into(),
                    detail: detail.into(),
                },
                None,
            )?;
        }
        Ok(())
    }

    fn expire_approvals(&mut self, now: Millis) -> Result<(), RuntimeError> {
        let expired: Vec<Approval> = self
            .state
            .approvals
            .values()
            .filter(|a| a.is_pending() && now > a.expires_at)
            .cloned()
            .collect();
        for a in expired {
            self.close_approval(
                &a,
                "expired",
                "Nobody decided in time. Nothing was written.",
                now,
            )?;
        }
        Ok(())
    }

    /// The person keeps a memory entry.
    pub fn remember(
        &mut self,
        scope: Scope,
        content: &str,
        now: Millis,
    ) -> Result<String, RuntimeError> {
        self.write_memory(
            scope,
            content,
            "person",
            None,
            Generation::Person,
            vec![],
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write_memory(
        &mut self,
        scope: Scope,
        content: &str,
        author: &str,
        task_id: Option<&str>,
        generation: Generation,
        sources: Vec<Source>,
        now: Millis,
    ) -> Result<String, RuntimeError> {
        if content.trim().is_empty() || content.len() > crate::memory::MAX_MEMORY_BYTES {
            return refused("a memory entry holds 1 to 4096 bytes");
        }
        let digest = sha256_hex(content.as_bytes());
        if let Some(existing) = self
            .state
            .memory
            .values()
            .find(|m| !m.forgotten && m.scope == scope && m.content_sha256 == digest)
        {
            return refused(format!(
                "This is already kept as {}; nothing new was stored.",
                existing.id
            ));
        }
        let sha = self
            .blobs
            .put(content)
            .map_err(|e| RuntimeError::Journal(JournalError::Io(e)))?;
        let entry = MemoryEntry {
            id: self.state.next_id("M", self.state.memory.len()),
            scope,
            content_sha256: sha,
            content_bytes: content.len(),
            author: author.into(),
            task_id: task_id.map(str::to_owned),
            generation,
            sources: if sources.is_empty() && generation == Generation::Person {
                vec![Source {
                    kind: "person".into(),
                    reference: "Team page".into(),
                    sha256: None,
                }]
            } else {
                sources
            },
            supersedes: None,
            superseded_by: None,
            created_at: rfc3339(now),
            forgotten: false,
        };
        let id = entry.id.clone();
        self.emit(
            now,
            author,
            "memory.written",
            task_id,
            generation,
            json!({ "memory": entry.record() }),
        )?;
        Ok(id)
    }

    /// Deletes a memory entry's words; the journal keeps a tombstone.
    pub fn forget(&mut self, memory_id: &str, now: Millis) -> Result<(), RuntimeError> {
        let Some(entry) = self.state.memory.get(memory_id).cloned() else {
            return refused("no such memory entry");
        };
        if entry.forgotten {
            return Ok(());
        }
        let shared =
            self.state.memory.values().any(|m| {
                m.id != entry.id && !m.forgotten && m.content_sha256 == entry.content_sha256
            });
        let pending_write = self
            .state
            .approvals
            .values()
            .any(|a| a.content_sha256 == entry.content_sha256 && a.is_pending());
        if !shared && !pending_write {
            self.blobs
                .delete(&entry.content_sha256)
                .map_err(|e| RuntimeError::Journal(JournalError::Io(e)))?;
        }
        self.emit(
            now,
            "person",
            "memory.forgotten",
            None,
            Generation::Person,
            json!({ "memory_id": memory_id }),
        )
    }

    /// Adds or replaces a routine.
    #[allow(clippy::too_many_arguments)]
    pub fn save_routine(
        &mut self,
        id: Option<&str>,
        name: &str,
        cron: &str,
        utc_offset_minutes: i32,
        assignee: &str,
        objective: &str,
        enabled: bool,
        now: Millis,
    ) -> Result<String, RuntimeError> {
        let cron = Cron::parse(cron).map_err(RuntimeError::Refused)?;
        if !self.state.agents.contains_key(assignee) {
            return refused(format!("there is no assistant called {assignee}"));
        }
        if name.trim().is_empty()
            || objective.trim().is_empty()
            || !(-840..=840).contains(&utc_offset_minutes)
        {
            return refused("a routine needs a name, a task and a UTC offset within ±14 hours");
        }
        let id = id
            .map(str::to_owned)
            .unwrap_or_else(|| format!("R-{:04}", self.state.routines.len() + 1));
        self.emit(
            now,
            "person",
            "routine.saved",
            None,
            Generation::Person,
            json!({
                "routine_id": id, "name": one_line(name, 80), "cron": cron.text(),
                "utc_offset_minutes": utc_offset_minutes, "assignee": assignee,
                "objective": clip(objective, protocol::MAX_OBJECTIVE), "enabled": enabled,
            }),
        )?;
        Ok(id)
    }

    pub fn remove_routine(&mut self, id: &str, now: Millis) -> Result<(), RuntimeError> {
        if !self.state.routines.contains_key(id) {
            return refused("no such routine");
        }
        self.emit(
            now,
            "person",
            "routine.removed",
            None,
            Generation::Person,
            json!({ "routine_id": id }),
        )
    }

    /// Creates the tasks of routines due this minute. Returns their ids.
    pub fn tick(&mut self, now: Millis) -> Result<Vec<String>, RuntimeError> {
        let minute = now.div_euclid(60_000);
        let due: Vec<Routine> = self
            .state
            .routines
            .values()
            .filter(|r| {
                r.enabled
                    && r.last_fired_minute != Some(minute)
                    && r.cron.matches(now, r.utc_offset_minutes)
            })
            .cloned()
            .collect();
        let mut created = Vec::new();
        for r in due {
            let budget = Budget {
                max_depth: self.limits.max_depth,
                max_steps: self.limits.root_steps,
                deadline_ms: Some(now + self.limits.deadline_ms),
            };
            let objective = format!("(Routine “{}”) {}", r.name, r.objective);
            let id = self.create_task(
                None,
                "host",
                &r.assignee,
                &clip(&objective, protocol::MAX_OBJECTIVE),
                budget,
                now,
            )?;
            self.emit(
                now,
                "host",
                "routine.fired",
                Some(&id),
                Generation::Host,
                json!({ "routine_id": r.id, "minute": minute, "task_id": id }),
            )?;
            created.push(id);
        }
        Ok(created)
    }

    // ── what the team does ───────────────────────────────────────────────

    /// The task the next [`Runtime::step`] would advance, if any.
    pub fn next_runnable(&self) -> Option<String> {
        self.state
            .tasks
            .values()
            .find(|t| {
                t.state == TaskState::Queued
                    || (t.state == TaskState::Running && t.waiting.is_none())
            })
            .map(|t| t.id.clone())
    }

    /// Advances one task by one model call and the action it names.
    pub fn step(
        &mut self,
        model: &mut dyn ModelPort,
        now: Millis,
    ) -> Result<StepOutcome, RuntimeError> {
        self.process_cancellations(now)?;
        self.expire_approvals(now)?;
        let Some(id) = self.next_runnable() else {
            return Ok(StepOutcome::Idle);
        };
        let task = self.state.tasks[&id].clone();
        if task.state == TaskState::Queued {
            self.emit(
                now,
                "host",
                "task.started",
                Some(&id),
                Generation::Host,
                json!({}),
            )?;
        }
        let stop = if task.budget.deadline_ms.is_some_and(|d| now > d) {
            Some("The task ran past its time limit.".to_owned())
        } else if task.steps_left() == 0 {
            Some("The task used up its step budget.".to_owned())
        } else if task.invalid_replies >= self.limits.max_invalid {
            Some(format!(
                "The assistant gave {} unusable replies in a row.",
                task.invalid_replies
            ))
        } else if task.model_failures >= self.limits.max_model_failures {
            Some(format!(
                "The model failed {} times: {}",
                task.model_failures,
                task.last_error.clone().unwrap_or_default()
            ))
        } else {
            None
        };
        if let Some(reason) = stop {
            self.finish(&id, TaskState::Failed, &reason, Generation::Host, now)?;
            return Ok(StepOutcome::Progress(id));
        }
        let Some(agent) = self.state.agents.get(&task.assignee).cloned() else {
            self.finish(
                &id,
                TaskState::Failed,
                "Its assistant no longer exists.",
                Generation::Host,
                now,
            )?;
            return Ok(StepOutcome::Progress(id));
        };
        let request = self.build_request(&task, &agent);
        self.cancel.begin(&id);
        let reply = model.complete(&request, &self.cancel);
        let aborted = self.cancel.aborted();
        self.cancel.end();
        if aborted {
            self.process_cancellations(now)?;
            return Ok(StepOutcome::Cancelled(id));
        }
        let prompt_sha256 =
            sha256_hex(format!("{}\u{0}{}", request.system, request.user).as_bytes());
        match reply {
            Err(error) => {
                let data = json!({ "model": model.label(), "error": clip(&error, 400), "prompt_sha256": prompt_sha256 });
                self.emit(
                    now,
                    "host",
                    "model.failed",
                    Some(&id),
                    Generation::Host,
                    data,
                )?;
            }
            Ok(reply) => {
                let data = json!({
                    "model": clip(&reply.model, 200),
                    "prompt_sha256": prompt_sha256,
                    "response": clip(&reply.text, 16_000),
                    "response_sha256": sha256_hex(reply.text.as_bytes()),
                    "tokens": null,
                });
                self.emit(
                    now,
                    &agent.id,
                    "model.called",
                    Some(&id),
                    reply.generation,
                    data,
                )?;
                match protocol::parse_action(&reply.text) {
                    Err(reason) => {
                        self.emit(
                            now,
                            "host",
                            "agent.output_invalid",
                            Some(&id),
                            Generation::Host,
                            json!({ "reason": reason }),
                        )?;
                    }
                    Ok(action) => {
                        let data =
                            json!({ "action": action.to_value(), "summary": action.summary() });
                        self.emit(
                            now,
                            &agent.id,
                            "agent.output",
                            Some(&id),
                            reply.generation,
                            data,
                        )?;
                        self.execute(&id, &agent, action, reply.generation, now)?;
                    }
                }
            }
        }
        Ok(StepOutcome::Progress(id))
    }

    fn build_request(&self, task: &Task, agent: &AgentProfile) -> ModelRequest {
        let team: Vec<&AgentProfile> = self.state.agents.values().collect();
        let memory: Vec<(String, String)> = self
            .state
            .memory
            .values()
            .filter(|m| m.visible_to(&agent.id, &task.root))
            .filter_map(|m| {
                let text = self.blobs.get(&m.content_sha256)?;
                let from = if m.sources.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", from {}",
                        m.sources
                            .iter()
                            .map(|s| s.reference.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                Some((
                    format!("{} ({}, by {}{from})", m.id, m.scope.plain(), m.author),
                    text,
                ))
            })
            .collect();
        let mut observations = Vec::new();
        let mut previous_actions = Vec::new();
        let history: Vec<String> = task
            .transcript
            .iter()
            .map(|turn| match turn {
                Turn::Action { action, .. } => {
                    previous_actions.push(action.clone());
                    format!(
                        "You replied: {}",
                        canonical::canonical(action).unwrap_or_default()
                    )
                }
                Turn::Observation(o) => {
                    observations.push(o.clone());
                    match o {
                        Observation::Tool { tool, ok, text } => {
                            format!(
                                "Host: {tool} {}:\n<data>\n{}\n</data>",
                                if *ok { "result" } else { "error" },
                                clip(text, 12_000)
                            )
                        }
                        Observation::Child {
                            agent,
                            state,
                            result,
                            task_id,
                        } => {
                            format!(
                                "Host: {agent} ({task_id}) {state}:\n<data>\n{}\n</data>",
                                clip(result, 6000)
                            )
                        }
                        Observation::Approval {
                            outcome, detail, ..
                        } => format!("Host: your proposal was {outcome}. {detail}"),
                        Observation::Refused { reason } => format!("Host: refused. {reason}"),
                        Observation::Note { text } => format!("Host: {text}"),
                    }
                }
            })
            .collect();
        ModelRequest {
            agent_id: agent.id.clone(),
            task_id: task.id.clone(),
            objective: task.objective.clone(),
            observations,
            previous_actions,
            system: protocol::system_prompt(agent, &team),
            user: protocol::user_prompt(&task.objective, &memory, &history),
        }
    }

    fn execute(
        &mut self,
        id: &str,
        agent: &AgentProfile,
        action: Action,
        generation: Generation,
        now: Millis,
    ) -> Result<(), RuntimeError> {
        if let Some(cap) = action.requires() {
            if !agent.can(cap) {
                let reason = format!("{} cannot {}.", agent.display_name, cap.plain());
                return self.observe(id, now, Observation::Refused { reason }, None);
            }
        }
        let task = self.state.tasks[id].clone();
        match action {
            Action::ListFiles { path } => {
                let result = match &self.workspace {
                    None => Err("No workspace folder is open.".to_owned()),
                    Some(ws) => ws.list(&path).map_err(|e| e.0),
                };
                let obs = match result {
                    Ok(entries) if entries.is_empty() => Observation::Tool {
                        tool: "list_files".into(),
                        ok: true,
                        text: "(empty folder)".into(),
                    },
                    Ok(entries) => Observation::Tool {
                        tool: "list_files".into(),
                        ok: true,
                        text: entries
                            .iter()
                            .map(|e| {
                                if e.is_dir {
                                    format!("{}/", e.name)
                                } else {
                                    format!("{} ({} bytes)", e.name, e.bytes)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    },
                    Err(e) => Observation::Tool {
                        tool: "list_files".into(),
                        ok: false,
                        text: e,
                    },
                };
                self.observe(id, now, obs, None)
            }
            Action::ReadFile { path } => {
                let result = match &self.workspace {
                    None => Err("No workspace folder is open.".to_owned()),
                    Some(ws) => ws.read(&path).map_err(|e| e.0),
                };
                match result {
                    Ok(read) => {
                        let numbered: Vec<String> = read
                            .text
                            .lines()
                            .enumerate()
                            .map(|(i, l)| format!("{:>4}| {l}", i + 1))
                            .collect();
                        let mut text = format!(
                            "{} (sha256 {})\n{}",
                            read.path,
                            &read.sha256[..12],
                            numbered.join("\n")
                        );
                        if read.truncated {
                            text.push_str("\n(only the first 64 KiB are shown)");
                        }
                        let source = json!({ "path": read.path, "sha256": read.sha256 });
                        self.observe(
                            id,
                            now,
                            Observation::Tool {
                                tool: "read_file".into(),
                                ok: true,
                                text: clip(&text, 20_000),
                            },
                            Some(source),
                        )
                    }
                    Err(e) => self.observe(
                        id,
                        now,
                        Observation::Tool {
                            tool: "read_file".into(),
                            ok: false,
                            text: e,
                        },
                        None,
                    ),
                }
            }
            Action::VerifyCitations { text } => {
                let obs = match &self.workspace {
                    None => Observation::Tool {
                        tool: "verify_citations".into(),
                        ok: false,
                        text: "No workspace folder is open.".into(),
                    },
                    Some(ws) => {
                        let checks = ws.verify_citations(&text);
                        let failed: Vec<String> = checks
                            .iter()
                            .filter(|c| !c.exists || c.quote_matches == Some(false))
                            .map(|c| {
                                format!(
                                    "[{}:{}] {}",
                                    c.path,
                                    c.line,
                                    if !c.exists {
                                        "no such line"
                                    } else {
                                        "the quote is not on that line"
                                    }
                                )
                            })
                            .collect();
                        let summary = if checks.is_empty() {
                            "No [path:line] citations were found in the text.".to_owned()
                        } else {
                            format!(
                                "{} citations checked by the host: {} hold, {} fail.{}",
                                checks.len(),
                                checks.len() - failed.len(),
                                failed.len(),
                                if failed.is_empty() {
                                    String::new()
                                } else {
                                    format!("\n{}", failed.join("\n"))
                                }
                            )
                        };
                        Observation::Tool {
                            tool: "verify_citations".into(),
                            ok: true,
                            text: summary,
                        }
                    }
                };
                self.observe(id, now, obs, None)
            }
            Action::Delegate {
                to,
                objective,
                max_steps,
            } => match self.check_delegation(&task, &to, max_steps) {
                Err(reason) => self.observe(id, now, Observation::Refused { reason }, None),
                Ok(budget) => {
                    let child =
                        self.create_task(Some(&task), &agent.id, &to, &objective, budget, now)?;
                    self.emit(
                        now,
                        "host",
                        "task.waiting",
                        Some(id),
                        Generation::Host,
                        json!({ "on": "children", "child": child }),
                    )
                }
            },
            Action::Remember { scope, content } => {
                let scope = match scope {
                    MemoryChoice::Agent => Scope::Agent(agent.id.clone()),
                    MemoryChoice::Task => Scope::Task(task.root.clone()),
                    MemoryChoice::Team => Scope::Team,
                };
                let sources = task.sources.clone();
                match self.write_memory(
                    scope,
                    &content,
                    &agent.id,
                    Some(id),
                    generation,
                    sources,
                    now,
                ) {
                    Ok(m) => self.observe(
                        id,
                        now,
                        Observation::Note {
                            text: format!("Kept as memory {m}."),
                        },
                        None,
                    ),
                    Err(RuntimeError::Refused(reason)) => {
                        self.observe(id, now, Observation::Refused { reason }, None)
                    }
                    Err(e) => Err(e),
                }
            }
            Action::ProposeWrite { path, content } => {
                self.propose_write(&task, agent, &path, &content, now)
            }
            Action::Finish { result } => {
                self.finish(id, TaskState::Succeeded, &result, generation, now)
            }
            Action::Fail { reason } => self.finish(id, TaskState::Failed, &reason, generation, now),
        }
    }

    fn check_delegation(
        &self,
        parent: &Task,
        to: &str,
        max_steps: Option<u32>,
    ) -> Result<Budget, String> {
        if !self.state.agents.contains_key(to) {
            return Err(format!(
                "There is no assistant called “{}”.",
                one_line(to, 40)
            ));
        }
        if to == parent.assignee {
            return Err("You cannot delegate to yourself.".into());
        }
        let mut cursor = parent.parent.clone();
        while let Some(pid) = cursor {
            let Some(p) = self.state.tasks.get(&pid) else {
                break;
            };
            if p.assignee == to {
                return Err(format!(
                    "{to} is already waiting on this work further up; handing it back would be a cycle."
                ));
            }
            cursor = p.parent.clone();
        }
        if parent.budget.max_depth == 0 {
            return Err(
                "No delegation depth is left at this level; do the work yourself or finish.".into(),
            );
        }
        let open_children = parent.children.len();
        if open_children >= self.limits.max_children {
            return Err(format!(
                "This task already handed out {open_children} sub-tasks, the most allowed."
            ));
        }
        let left = parent.steps_left();
        if left < 3 {
            return Err("Not enough of this task's budget is left to delegate.".into());
        }
        let wanted = max_steps.unwrap_or(left / 2).max(1);
        if wanted > left - 1 {
            return Err(format!(
                "You have {left} steps left; a sub-task can have at most {}.",
                left - 1
            ));
        }
        Ok(Budget {
            max_depth: parent.budget.max_depth - 1,
            max_steps: wanted,
            deadline_ms: parent.budget.deadline_ms,
        })
    }

    fn propose_write(
        &mut self,
        task: &Task,
        agent: &AgentProfile,
        path: &str,
        content: &str,
        now: Millis,
    ) -> Result<(), RuntimeError> {
        let Some(ws) = self.workspace.as_ref() else {
            return self.observe(
                &task.id,
                now,
                Observation::Refused {
                    reason: "No workspace folder is open.".into(),
                },
                None,
            );
        };
        let relative = match clean_relative(path) {
            Ok(p) if !p.as_os_str().is_empty() => p.to_string_lossy().replace('\\', "/"),
            Ok(_) => {
                return self.observe(
                    &task.id,
                    now,
                    Observation::Refused {
                        reason: "Name a file.".into(),
                    },
                    None,
                );
            }
            Err(e) => {
                return self.observe(&task.id, now, Observation::Refused { reason: e.0 }, None);
            }
        };
        let previous = match ws.digest_of(&relative) {
            Ok(p) => p,
            Err(e) => {
                return self.observe(&task.id, now, Observation::Refused { reason: e.0 }, None);
            }
        };
        let content_sha256 = self
            .blobs
            .put(content)
            .map_err(|e| RuntimeError::Journal(JournalError::Io(e)))?;
        let action = json!({
            "kind": "file_write",
            "path": relative,
            "bytes": content.len(),
            "content_sha256": content_sha256,
            "previous_sha256": previous,
        });
        let action_sha256 = canonical::digest(&action).expect("finite");
        let request_id = self.state.next_id("A", self.state.approvals.len());
        let record = canonical::seal(json!({
            "schema": "agent-approval/v1",
            "system": "james-desktop",
            "request_id": request_id,
            "task_id": task.id,
            "requested_by": agent.id,
            "effect": "file_write",
            "action": action,
            "action_sha256": action_sha256,
            "expires_at": rfc3339(now + self.limits.approval_ttl_ms),
            "decision": null,
        }))
        .expect("finite");
        self.emit(
            now,
            "host",
            "approval.requested",
            Some(&task.id),
            Generation::Host,
            json!({ "approval": record }),
        )
    }

    fn finish(
        &mut self,
        id: &str,
        state: TaskState,
        result: &str,
        generation: Generation,
        now: Millis,
    ) -> Result<(), RuntimeError> {
        // Anything still open below a finishing task is cancelled with it.
        let below: Vec<String> = self.state.subtree(id).into_iter().skip(1).collect();
        for t in below {
            if self
                .state
                .tasks
                .get(&t)
                .is_some_and(|t| !t.state.is_terminal())
            {
                self.void_approvals_of(&t, now)?;
                self.emit(
                    now,
                    "host",
                    "task.cancelled",
                    Some(&t),
                    Generation::Host,
                    json!({ "by": "host", "reason": "its parent finished" }),
                )?;
            }
        }
        self.void_approvals_of(id, now)?;
        let data = json!({ "state": state.as_str(), "result": clip(result, protocol::MAX_RESULT), "result_generation": generation.as_str() });
        self.emit(
            now,
            "host",
            "task.finished",
            Some(id),
            Generation::Host,
            data,
        )?;
        self.notify_parent(id, now)
    }

    fn notify_parent(&mut self, id: &str, now: Millis) -> Result<(), RuntimeError> {
        let Some(task) = self.state.tasks.get(id).cloned() else {
            return Ok(());
        };
        let Some(parent) = task
            .parent
            .as_ref()
            .and_then(|p| self.state.tasks.get(p))
            .cloned()
        else {
            return Ok(());
        };
        if parent.state.is_terminal() {
            return Ok(());
        }
        let obs = Observation::Child {
            task_id: task.id.clone(),
            agent: task.assignee.clone(),
            state: task.state.as_str().into(),
            result: task.result.clone().unwrap_or_else(|| "(no result)".into()),
        };
        self.observe(&parent.id, now, obs, None)?;
        let all_done = parent.children.iter().all(|c| {
            self.state
                .tasks
                .get(c)
                .is_some_and(|t| t.state.is_terminal())
        });
        if all_done && parent.waiting == Some(Waiting::Children) {
            self.emit(
                now,
                "host",
                "task.ready",
                Some(&parent.id),
                Generation::Host,
                json!({}),
            )?;
        }
        Ok(())
    }

    /// Exported contract records, for inspection and interop tests.
    pub fn identity_records(&self) -> Vec<Value> {
        self.state
            .agents
            .values()
            .map(AgentProfile::identity_record)
            .collect()
    }
    pub fn task_records(&self) -> Vec<Value> {
        self.state.tasks.values().map(Task::record).collect()
    }
    pub fn memory_records(&self) -> Vec<Value> {
        self.state
            .memory
            .values()
            .map(MemoryEntry::record)
            .collect()
    }
    pub fn approval_records(&self) -> Vec<Value> {
        self.state
            .approvals
            .values()
            .map(|a| a.record.clone())
            .collect()
    }
    pub fn event_records(&self) -> Vec<Value> {
        self.journal.events().iter().map(Event::to_value).collect()
    }
    /// Agents that may be named in a capability check, for the UI.
    pub fn capability_matrix(&self) -> Vec<(String, BTreeSet<Capability>)> {
        self.state
            .agents
            .values()
            .map(|a| (a.id.clone(), a.capabilities.clone()))
            .collect()
    }
}
