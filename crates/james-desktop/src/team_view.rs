//! The Team page: James's persistent assistants, their tasks, what they
//! remember and what they are waiting on you for.
//!
//! A worker thread owns the `james-agents` runtime (and its journal in
//! `<James data folder>/team/`); this page sends it commands and draws the
//! snapshots it returns. The runtime only advances while a model is chosen
//! and the team is not paused. Cancelling cuts an in-flight model call at
//! once through the runtime's cancel signal. A file write is shown here with
//! its exact content and digest and happens only when you approve that
//! content. Nothing on this page reaches the conversation, the octopus, the
//! reviewed-input path or the Lab.
use crate::{JamesApp, numeric_port, theme};
use eframe::egui::{self, RichText};
use james_agents::clock::{now_ms, rfc3339};
use james_agents::memory::Scope;
use james_agents::model::CancelSignal;
use james_agents::protocol::Observation;
use james_agents::runtime::{Activity, Recovery, State, TaskState, Turn};
use james_agents::{LocalModel, Runtime, StepOutcome, one_line};
use james_models::LocalProvider;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::Duration;

const MAX_EVENTS_SHOWN: usize = 2000;

#[derive(Debug, Clone)]
pub struct EventLine {
    pub seq: u64,
    pub at: String,
    pub actor: String,
    pub task: Option<String>,
    pub generation: &'static str,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub state: State,
    pub events: Vec<EventLine>,
    /// Text by digest, for pending proposals and live memory entries.
    pub texts: BTreeMap<String, String>,
    pub verified: Result<usize, String>,
    pub recovery: Recovery,
    pub model: Option<String>,
    pub journal: PathBuf,
    pub working_on: Option<String>,
    pub paused: bool,
}

enum Command {
    Submit(String, String),
    Decide(String, bool, String),
    Resume(String),
    Remember(Scope, String),
    Forget(String),
    Workspace(PathBuf),
    SaveRoutine {
        name: String,
        cron: String,
        offset: i32,
        assignee: String,
        objective: String,
    },
    RemoveRoutine(String),
    Model(Option<(LocalProvider, u16, String)>),
    Pause(bool),
}

enum Update {
    Ready(CancelSignal),
    Snapshot(Box<Snapshot>),
    Working(String),
    Error(String),
}

struct Worker {
    tx: Sender<Command>,
    rx: Receiver<Update>,
    cancel: Option<CancelSignal>,
}

pub struct TeamState {
    worker: Option<Worker>,
    pub snapshot: Option<Snapshot>,
    pub error: Option<String>,
    pub objective: String,
    pub assignee: String,
    pub workspace: String,
    pub selected: Option<String>,
    pub memory_draft: String,
    pub memory_scope: String,
    pub routine_name: String,
    pub routine_cron: String,
    pub routine_offset: String,
    pub routine_assignee: String,
    pub routine_objective: String,
    model_sent: Option<Option<(LocalProvider, u16, String)>>,
}

impl TeamState {
    pub fn new() -> Self {
        Self {
            worker: None,
            snapshot: None,
            error: None,
            objective: String::new(),
            assignee: "coordinator".into(),
            workspace: String::new(),
            selected: None,
            memory_draft: String::new(),
            memory_scope: "team".into(),
            routine_name: String::new(),
            routine_cron: "0 9 * * 1-5".into(),
            routine_offset: "0".into(),
            routine_assignee: "researcher".into(),
            routine_objective: String::new(),
            model_sent: None,
        }
    }

    pub fn started(&self) -> bool {
        self.worker.is_some()
    }

    /// Whether the page should keep repainting to show progress.
    pub fn active(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|s| s.working_on.is_some())
    }

    pub fn start(&mut self, dir: PathBuf) {
        if self.worker.is_some() {
            return;
        }
        let (tx, worker_rx) = mpsc::channel();
        let (worker_tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("james-team".into())
            .spawn(move || run(dir, worker_rx, worker_tx))
            .map_err(|e| self.error = Some(format!("The team could not start: {e}")))
            .ok();
        self.worker = Some(Worker {
            tx,
            rx,
            cancel: None,
        });
    }

    fn send(&mut self, command: Command) {
        if let Some(w) = &self.worker
            && w.tx.send(command).is_err()
        {
            self.error = Some("The team stopped unexpectedly. Restart James to reopen it.".into());
        }
    }

    pub fn poll(&mut self) {
        let Some(worker) = self.worker.as_mut() else {
            return;
        };
        loop {
            match worker.rx.try_recv() {
                Ok(Update::Ready(signal)) => worker.cancel = Some(signal),
                Ok(Update::Snapshot(s)) => {
                    self.snapshot = Some(*s);
                }
                Ok(Update::Working(task)) => {
                    if let Some(s) = self.snapshot.as_mut() {
                        s.working_on = Some(task);
                    }
                }
                Ok(Update::Error(e)) => self.error = Some(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.error.is_none() {
                        self.error = Some("The team stopped. Restart James to reopen it.".into());
                    }
                    break;
                }
            }
        }
    }

    /// Keeps the team on the model chosen in Settings.
    pub fn follow_model(&mut self, provider: LocalProvider, port: Option<u16>, model: &str) {
        let wanted = match (port, model.trim()) {
            (Some(port), name) if !name.is_empty() => Some((provider, port, name.to_owned())),
            _ => None,
        };
        if self.worker.is_some() && self.model_sent.as_ref() != Some(&wanted) {
            self.model_sent = Some(wanted.clone());
            self.send(Command::Model(wanted));
        }
    }

    /// Stops a task and its sub-tasks now, cutting a model call in flight.
    pub fn cancel(&mut self, task: &str) {
        let ids = self
            .snapshot
            .as_ref()
            .map(|s| s.state.subtree(task))
            .unwrap_or_default();
        if let Some(signal) = self.worker.as_ref().and_then(|w| w.cancel.clone()) {
            signal.request(ids);
        }
        self.send(Command::Pause(
            self.snapshot.as_ref().is_some_and(|s| s.paused),
        ));
    }
}

impl Default for TeamState {
    fn default() -> Self {
        Self::new()
    }
}

fn run(dir: PathBuf, rx: Receiver<Command>, tx: Sender<Update>) {
    let (mut rt, recovery) = match Runtime::open(&dir, None, now_ms()) {
        Ok(opened) => opened,
        Err(e) => {
            let _ = tx.send(Update::Error(format!(
                "The team's journal could not be opened ({e}). It was left untouched in {}.",
                dir.display()
            )));
            return;
        }
    };
    let _ = tx.send(Update::Ready(rt.cancel_signal()));
    let mut model: Option<LocalModel> = None;
    let mut paused = false;
    let send = |rt: &Runtime, model: &Option<LocalModel>, paused: bool| {
        let _ = tx.send(Update::Snapshot(Box::new(snapshot(
            rt, &recovery, model, paused,
        ))));
    };
    send(&rt, &model, paused);
    loop {
        let mut changed = false;
        loop {
            match rx.try_recv() {
                Ok(cmd) => {
                    handle(&mut rt, &mut model, &mut paused, cmd, &tx);
                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        if rt.tick(now_ms()).is_ok_and(|created| !created.is_empty()) {
            changed = true;
        }
        let mut worked = false;
        if !paused && let Some(m) = model.as_mut() {
            if let Some(next) = rt.next_runnable() {
                let _ = tx.send(Update::Working(next));
            }
            match rt.step(m, now_ms()) {
                Ok(StepOutcome::Idle) => {}
                Ok(_) => {
                    worked = true;
                    changed = true;
                }
                Err(e) => {
                    let _ = tx.send(Update::Error(format!("The team stopped working: {e}")));
                    paused = true;
                    changed = true;
                }
            }
        }
        if changed {
            send(&rt, &model, paused);
        }
        if !worked {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(cmd) => {
                    handle(&mut rt, &mut model, &mut paused, cmd, &tx);
                    send(&rt, &model, paused);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

fn handle(
    rt: &mut Runtime,
    model: &mut Option<LocalModel>,
    paused: &mut bool,
    cmd: Command,
    tx: &Sender<Update>,
) {
    let now = now_ms();
    let result = match cmd {
        Command::Submit(objective, assignee) => rt.submit(&objective, &assignee, now).map(|_| ()),
        Command::Decide(id, approve, sha) => rt.decide(&id, approve, &sha, now).map(|_| ()),
        Command::Resume(id) => rt.resume(&id, now),
        Command::Remember(scope, text) => rt.remember(scope, &text, now).map(|_| ()),
        Command::Forget(id) => rt.forget(&id, now),
        Command::Workspace(path) => rt.set_workspace(&path, now),
        Command::SaveRoutine {
            name,
            cron,
            offset,
            assignee,
            objective,
        } => rt
            .save_routine(None, &name, &cron, offset, &assignee, &objective, true, now)
            .map(|_| ()),
        Command::RemoveRoutine(id) => rt.remove_routine(&id, now),
        Command::Model(choice) => {
            *model = choice
                .and_then(|(provider, port, name)| LocalModel::new(provider, port, name).ok());
            Ok(())
        }
        Command::Pause(p) => {
            *paused = p;
            // A cancellation requested through the signal is recorded here too.
            rt.step_cancellations(now)
        }
    };
    if let Err(e) = result {
        let _ = tx.send(Update::Error(e.to_string()));
    }
}

fn snapshot(
    rt: &Runtime,
    recovery: &Recovery,
    model: &Option<LocalModel>,
    paused: bool,
) -> Snapshot {
    use james_agents::model::ModelPort;
    let state = rt.state().clone();
    let mut texts = BTreeMap::new();
    for a in state.approvals.values().filter(|a| a.is_pending()) {
        if let Some(t) = rt.blob(&a.content_sha256) {
            texts.insert(a.content_sha256.clone(), t);
        }
    }
    for m in state.memory.values().filter(|m| !m.forgotten) {
        if let Some(t) = rt.blob(&m.content_sha256) {
            texts.insert(m.content_sha256.clone(), t);
        }
    }
    let all = rt.events();
    let events = all[all.len().saturating_sub(MAX_EVENTS_SHOWN)..]
        .iter()
        .map(|e| EventLine {
            seq: e.seq,
            at: e.at.clone(),
            actor: e.actor.clone(),
            task: e.task_id.clone(),
            generation: e.generation.as_str(),
            summary: describe(e),
        })
        .collect();
    Snapshot {
        state,
        events,
        texts,
        verified: rt.verify().map(|_| all.len()).map_err(|e| e.to_string()),
        recovery: recovery.clone(),
        model: model.as_ref().map(|m| m.label()),
        journal: rt.journal_path().to_owned(),
        working_on: None,
        paused,
    }
}

fn describe(e: &james_agents::journal::Event) -> String {
    let s = |key: &str| e.data[key].as_str().unwrap_or("").to_owned();
    match e.kind.as_str() {
        "task.created" => format!(
            "task created for {}",
            e.data["task"]["assignee"].as_str().unwrap_or("")
        ),
        "agent.output" => s("summary"),
        "agent.output_invalid" => format!("reply refused: {}", s("reason")),
        "model.called" => format!("model answered ({})", s("model")),
        "model.failed" => format!("model error: {}", s("error")),
        "task.observed" => Observation::from_value(&e.data["observation"])
            .map(|o| o.summary())
            .unwrap_or_default(),
        "task.finished" => format!("{}: {}", s("state"), one_line(&s("result"), 120)),
        "task.cancelled" => "cancelled".into(),
        "task.interrupted" => s("reason"),
        "approval.requested" => format!(
            "asks you to approve writing {}",
            e.data["approval"]["action"]["path"].as_str().unwrap_or("")
        ),
        "approval.decided" => format!("you {}", s("verdict")),
        "approval.closed" => format!("{}: {}", s("outcome"), s("detail")),
        "memory.written" => format!(
            "memory {} written",
            e.data["memory"]["memory_id"].as_str().unwrap_or("")
        ),
        "memory.forgotten" => format!("memory {} forgotten", s("memory_id")),
        "routine.fired" => format!("routine {} started this task", s("routine_id")),
        "routine.missed" => format!(
            "routine {} missed {} run(s) while James was closed",
            s("routine_id"),
            e.data["count"]
        ),
        other => other.replace('.', " "),
    }
}

fn state_label(task_state: TaskState, waiting_on_team: bool) -> &'static str {
    match task_state {
        TaskState::Queued => "queued",
        TaskState::Running if waiting_on_team => "waiting on the team",
        TaskState::Running => "working",
        TaskState::AwaitingApproval => "needs your approval",
        TaskState::Succeeded => "done",
        TaskState::Failed => "failed",
        TaskState::Cancelled => "cancelled",
        TaskState::Interrupted => "interrupted — needs you",
    }
}

pub fn data_dir(session: &Path) -> PathBuf {
    session
        .parent()
        .map(|p| p.join("team"))
        .unwrap_or_else(|| PathBuf::from("team"))
}

pub fn ui(app: &mut JamesApp, ui: &mut egui::Ui) {
    if !app.team.started() {
        app.team.start(data_dir(&app.session));
    }
    let port = numeric_port(&app.port).ok();
    let (provider, model) = (app.provider, app.model.clone());
    let team = &mut app.team;
    team.follow_model(provider, port, &model);
    team.poll();
    let accent = theme::accent(ui);
    ui.heading("Team");
    ui.label(
        "Four assistants that work on tasks you give them. They read only the folder you choose, every step is recorded, \
         and nothing is written without your approval of the exact content.",
    );
    if let Some(error) = team.error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(ui.visuals().error_fg_color, &error);
            if ui.small_button("Dismiss").clicked() {
                team.error = None;
            }
        });
    }
    let Some(snap) = team.snapshot.clone() else {
        ui.label("Opening the team's journal…");
        return;
    };
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        match &snap.model {
            Some(m) => ui.label(format!("Model: {m}")),
            None => ui.colored_label(
                ui.visuals().warn_fg_color,
                "Choose a model in Settings. Tasks wait until then.",
            ),
        };
        ui.separator();
        match &snap.verified {
            Ok(n) => ui.label(format!("Journal verified: {n} records")),
            Err(e) => ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Journal check failed: {e}"),
            ),
        };
        ui.separator();
        let label = if snap.paused {
            "Resume the team"
        } else {
            "Pause the team"
        };
        if ui.button(label).clicked() {
            let paused = !snap.paused;
            team.send(Command::Pause(paused));
        }
    });
    if !snap.recovery.interrupted.is_empty()
        || !snap.recovery.missed_routines.is_empty()
        || snap.recovery.journal.torn_tail.is_some()
    {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "When James reopened: {} task(s) were interrupted and wait for you; {} routine run(s) were missed and not run late{}.",
                snap.recovery.interrupted.len(),
                snap.recovery.missed_routines.iter().map(|(_, n)| n).sum::<usize>(),
                if snap.recovery.journal.torn_tail.is_some() { "; an unfinished final record was set aside" } else { "" }
            ),
        );
    }

    ui.add_space(10.0);
    ui.label(RichText::new("ASSISTANTS").small().color(accent));
    egui::Grid::new("team-roster")
        .striped(true)
        .num_columns(3)
        .show(ui, |ui| {
            for (id, activity) in snap.state.activity() {
                let Some(a) = snap.state.agents.get(&id) else {
                    continue;
                };
                ui.label(RichText::new(&a.display_name).strong())
                    .on_hover_text(format!("{} · revision {}", a.id, a.revision));
                ui.label(&a.role).on_hover_text(
                    a.capabilities
                        .iter()
                        .map(|c| c.plain())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                ui.label(match activity {
                    Activity::Idle => "idle".to_owned(),
                    Activity::Working(t) => format!("working on {t}"),
                    Activity::WaitingOnTeam(t) => format!("{t}: waiting on the team"),
                    Activity::WaitingOnPerson(t) => format!("{t}: waiting for you"),
                });
                ui.end_row();
            }
        });

    needs_you(team, ui, &snap);

    ui.add_space(10.0);
    ui.label(RichText::new("NEW TASK").small().color(accent));
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Workspace folder");
        ui.add(
            egui::TextEdit::singleline(&mut team.workspace)
                .desired_width(360.0)
                .hint_text(
                    snap.state
                        .workspace
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "A folder the team may read".into()),
                ),
        )
        .labelled_by(label.id);
        if ui.button("Use this folder").clicked() && !team.workspace.trim().is_empty() {
            let path = PathBuf::from(team.workspace.trim());
            team.send(Command::Workspace(path));
            team.workspace.clear();
        }
    });
    let label = ui.label("Task");
    ui.add(egui::TextEdit::multiline(&mut team.objective).desired_rows(3).desired_width(f32::INFINITY).hint_text(
        "For example: read docs/ and draft notes/brief.md listing what must happen before release, citing each line.",
    ))
    .labelled_by(label.id);
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Give it to");
        egui::ComboBox::from_id_salt("team-assignee")
            .selected_text(
                snap.state
                    .agents
                    .get(&team.assignee)
                    .map(|a| a.display_name.clone())
                    .unwrap_or_default(),
            )
            .show_ui(ui, |ui| {
                for a in snap.state.agents.values() {
                    ui.selectable_value(&mut team.assignee, a.id.clone(), &a.display_name);
                }
            })
            .response
            .labelled_by(label.id);
        if ui.button("Start task").clicked() {
            if team.objective.trim().is_empty() {
                team.error = Some("Describe the task first.".into());
            } else {
                let objective = std::mem::take(&mut team.objective);
                let assignee = team.assignee.clone();
                team.send(Command::Submit(objective, assignee));
            }
        }
    });

    tasks(team, ui, &snap, accent);
    memory(team, ui, &snap, accent);
    routines(team, ui, &snap, accent);
    ui.add_space(8.0);
    ui.small(format!(
        "Everything on this page is kept in {}.",
        snap.journal.display()
    ));
}

fn needs_you(team: &mut TeamState, ui: &mut egui::Ui, snap: &Snapshot) {
    let pending: Vec<_> = snap
        .state
        .approvals
        .values()
        .filter(|a| a.is_pending())
        .cloned()
        .collect();
    let interrupted: Vec<_> = snap
        .state
        .tasks
        .values()
        .filter(|t| t.state == TaskState::Interrupted)
        .cloned()
        .collect();
    let unknown: Vec<_> = snap
        .state
        .approvals
        .values()
        .filter(|a| a.outcome.as_deref() == Some("unknown"))
        .cloned()
        .collect();
    if pending.is_empty() && interrupted.is_empty() && unknown.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(
        RichText::new("NEEDS YOU")
            .small()
            .color(ui.visuals().warn_fg_color),
    );
    for a in pending {
        ui.group(|ui| {
            let who = snap
                .state
                .agents
                .get(&a.requested_by)
                .map(|p| p.display_name.clone())
                .unwrap_or(a.requested_by.clone());
            ui.label(RichText::new(format!("{who} wants to write {}", a.path)).strong());
            ui.small(format!(
                "{} · {} · expires {} · exact action sha256 {}",
                a.id,
                match &a.previous_sha256 {
                    Some(p) => format!("replaces a file with sha256 {}", &p[..12]),
                    None => "creates a new file".into(),
                },
                rfc3339(a.expires_at),
                &a.action_sha256[..16]
            ));
            let content = snap
                .texts
                .get(&a.content_sha256)
                .cloned()
                .unwrap_or_else(|| "(content unavailable)".into());
            egui::ScrollArea::vertical()
                .id_salt(format!("proposal-{}", a.id))
                .max_height(220.0)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(content).monospace()).wrap());
                });
            ui.horizontal(|ui| {
                if ui.button("Approve this exact change").clicked() {
                    team.send(Command::Decide(a.id.clone(), true, a.action_sha256.clone()));
                }
                if ui.button("Decline").clicked() {
                    team.send(Command::Decide(
                        a.id.clone(),
                        false,
                        a.action_sha256.clone(),
                    ));
                }
            });
        });
    }
    for t in interrupted {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} ({}) was interrupted when James stopped. Nothing was retried.",
                t.id, t.assignee
            ));
            if ui.button(format!("Resume {}", t.id)).clicked() {
                team.send(Command::Resume(t.id.clone()));
            }
        });
    }
    for a in unknown {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "{}: whether {} was written is unknown. Check the file yourself. {}",
                a.id,
                a.path,
                a.detail.clone().unwrap_or_default()
            ),
        );
    }
}

fn tasks(team: &mut TeamState, ui: &mut egui::Ui, snap: &Snapshot, accent: egui::Color32) {
    ui.add_space(10.0);
    ui.label(RichText::new("TASKS").small().color(accent));
    if snap.state.tasks.is_empty() {
        ui.label("No tasks yet.");
        return;
    }
    let roots: Vec<String> = snap
        .state
        .tasks
        .values()
        .rev()
        .filter(|t| t.parent.is_none())
        .map(|t| t.id.clone())
        .take(20)
        .collect();
    for root in roots {
        for id in snap.state.subtree(&root) {
            let t = &snap.state.tasks[&id];
            ui.horizontal_wrapped(|ui| {
                ui.add_space(t.depth as f32 * 18.0);
                let working = snap.working_on.as_deref() == Some(&id);
                let state = if working {
                    "thinking…"
                } else {
                    state_label(t.state, t.waiting.is_some())
                };
                let selected = team.selected.as_deref() == Some(&id);
                if ui
                    .selectable_label(selected, format!("{id} · {} · {state}", t.assignee))
                    .on_hover_text(&t.objective)
                    .clicked()
                {
                    team.selected = if selected { None } else { Some(id.clone()) };
                }
                ui.label(one_line(&t.objective, 90));
                if t.parent.is_none()
                    && !t.state.is_terminal()
                    && ui.small_button(format!("Cancel {id}")).clicked()
                {
                    team.cancel(&id);
                }
            });
        }
    }
    if let Some(id) = team.selected.clone()
        && let Some(t) = snap.state.tasks.get(&id)
    {
        ui.group(|ui| {
            ui.label(RichText::new(format!("{id}: {}", t.assignee)).strong());
            ui.label(format!("Objective: {}", t.objective));
            ui.small(format!(
                "Steps used {} of {} ({} lent to sub-tasks) · depth {} · created by {}",
                t.steps_used, t.budget.max_steps, t.reserved, t.depth, t.created_by
            ));
            if let Some(result) = &t.result {
                ui.label(format!(
                    "Result ({}, written by {}):",
                    t.state.as_str(),
                    t.result_generation.map(|g| g.as_str()).unwrap_or("unknown")
                ));
                ui.add(egui::Label::new(RichText::new(result).monospace()).wrap());
            }
            let actions = t
                .transcript
                .iter()
                .filter(|x| matches!(x, Turn::Action { .. }))
                .count();
            ui.small(format!("{actions} replies used; timeline:"));
            for e in snap
                .events
                .iter()
                .filter(|e| e.task.as_deref() == Some(&id))
            {
                ui.small(format!(
                    "#{} {} {} [{}] {}",
                    e.seq,
                    &e.at[11..19],
                    e.actor,
                    e.generation,
                    one_line(&e.summary, 160)
                ));
            }
        });
    }
}

fn memory(team: &mut TeamState, ui: &mut egui::Ui, snap: &Snapshot, accent: egui::Color32) {
    ui.add_space(10.0);
    ui.label(RichText::new("MEMORY").small().color(accent));
    ui.small("Each entry shows who can see it, who wrote it and where it came from. Forget deletes its words; the journal keeps a note that it existed.");
    for m in snap.state.memory.values().rev().take(50) {
        ui.horizontal_wrapped(|ui| {
            let text = if m.forgotten {
                "(forgotten)".to_owned()
            } else {
                snap.texts
                    .get(&m.content_sha256)
                    .cloned()
                    .unwrap_or_else(|| "(missing)".into())
            };
            ui.label(RichText::new(&m.id).strong());
            ui.label(one_line(&text, 140));
            let sources: Vec<String> = m.sources.iter().map(|s| s.reference.clone()).collect();
            ui.small(format!(
                "{} · by {} ({}) · {}{}",
                m.scope.plain(),
                m.author,
                m.generation.as_str(),
                if sources.is_empty() {
                    "no sources".into()
                } else {
                    format!("from {}", sources.join(", "))
                },
                if m.superseded_by.is_some() {
                    " · superseded"
                } else {
                    ""
                }
            ));
            if !m.forgotten && ui.small_button(format!("Forget {}", m.id)).clicked() {
                team.send(Command::Forget(m.id.clone()));
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Add to memory");
        ui.add(egui::TextEdit::singleline(&mut team.memory_draft).desired_width(320.0))
            .labelled_by(label.id);
        egui::ComboBox::from_id_salt("team-memory-scope")
            .selected_text(if team.memory_scope == "team" {
                "Whole team".to_owned()
            } else {
                format!("Only {}", team.memory_scope)
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut team.memory_scope, "team".to_owned(), "Whole team");
                for a in snap.state.agents.keys() {
                    ui.selectable_value(&mut team.memory_scope, a.clone(), format!("Only {a}"));
                }
            });
        if ui.button("Remember").clicked() && !team.memory_draft.trim().is_empty() {
            let scope = if team.memory_scope == "team" {
                Scope::Team
            } else {
                Scope::Agent(team.memory_scope.clone())
            };
            let text = std::mem::take(&mut team.memory_draft);
            team.send(Command::Remember(scope, text));
        }
    });
}

fn routines(team: &mut TeamState, ui: &mut egui::Ui, snap: &Snapshot, accent: egui::Color32) {
    ui.add_space(10.0);
    ui.label(RichText::new("ROUTINES").small().color(accent));
    ui.small("A routine starts a task on a schedule while James is open. It never approves anything, and runs missed while James was closed are not run late.");
    for r in snap.state.routines.values() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&r.name).strong());
            ui.label(format!(
                "“{}” at UTC{:+}h for {}: {}",
                r.cron.text(),
                r.utc_offset_minutes as f32 / 60.0,
                r.assignee,
                one_line(&r.objective, 80)
            ));
            if ui.small_button(format!("Remove {}", r.id)).clicked() {
                team.send(Command::RemoveRoutine(r.id.clone()));
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        let l = ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut team.routine_name).desired_width(120.0))
            .labelled_by(l.id);
        let l = ui.label("Schedule (cron)");
        ui.add(egui::TextEdit::singleline(&mut team.routine_cron).desired_width(110.0))
            .labelled_by(l.id);
        let l = ui.label("UTC offset (minutes)");
        ui.add(egui::TextEdit::singleline(&mut team.routine_offset).desired_width(50.0))
            .labelled_by(l.id);
    });
    ui.horizontal_wrapped(|ui| {
        let l = ui.label("Routine task");
        ui.add(egui::TextEdit::singleline(&mut team.routine_objective).desired_width(320.0))
            .labelled_by(l.id);
        egui::ComboBox::from_id_salt("team-routine-assignee")
            .selected_text(team.routine_assignee.clone())
            .show_ui(ui, |ui| {
                for a in snap.state.agents.keys() {
                    ui.selectable_value(&mut team.routine_assignee, a.clone(), a);
                }
            });
        if ui.button("Add routine").clicked() {
            let offset = team.routine_offset.trim().parse::<i32>().unwrap_or(0);
            team.send(Command::SaveRoutine {
                name: team.routine_name.clone(),
                cron: team.routine_cron.clone(),
                offset,
                assignee: team.routine_assignee.clone(),
                objective: team.routine_objective.clone(),
            });
        }
    });
}
