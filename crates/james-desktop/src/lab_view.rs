//! R.A.I.N. Lab: a read-only window onto a Lab running on this computer.
//! James reads the Lab's status and recent results, explains them in plain
//! words with how far each can be trusted, and shares a snapshot with the
//! model only when the person turns that on. It never starts, steers or
//! approves anything in the Lab.
use crate::{JamesApp, numeric_port, theme};
use eframe::egui::{self, RichText};
use james_lab::{LabClient, ResearchContext, RuntimeStatus, describe_stage};
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

const REFRESH: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// A snapshot older than this is not attached to questions.
const FRESH: Duration = Duration::from_secs(15);

type Refresh = Result<(ResearchContext, Option<RuntimeStatus>), String>;

pub struct LabState {
    pub port: String,
    pub connected: bool,
    /// Attach the latest snapshot to model questions.
    pub include: bool,
    pub snapshot: Option<(ResearchContext, Instant)>,
    pub provenance: Option<String>,
    pub status: String,
    worker: Option<Receiver<Refresh>>,
    finder: Option<Receiver<Option<u16>>>,
    next: Instant,
    failures: u32,
}

impl LabState {
    pub fn new(port: u16) -> Self {
        Self {
            port: port.to_string(),
            connected: false,
            include: false,
            snapshot: None,
            provenance: None,
            status: "Not connected".into(),
            worker: None,
            finder: None,
            next: Instant::now(),
            failures: 0,
        }
    }

    pub fn finding(&self) -> bool {
        self.finder.is_some()
    }

    pub fn connect(&mut self) {
        self.connected = true;
        self.failures = 0;
        self.next = Instant::now();
        self.status = "Connecting…".into();
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.include = false;
        self.worker = None;
        self.snapshot = None;
        self.provenance = None;
        self.status = "Not connected".into();
    }

    /// Looks for the Lab on the ports its development server usually uses.
    pub fn find(&mut self) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(james_lab::find_lab(&james_lab::CANDIDATE_PORTS));
        });
        self.finder = Some(rx);
        self.status = "Looking for R.A.I.N. Lab on this computer…".into();
    }

    /// The snapshot text to attach to a question, if it is fresh enough.
    pub fn fresh_context(&self) -> Result<String, String> {
        match &self.snapshot {
            Some((snapshot, at)) if at.elapsed() <= FRESH => Ok(snapshot.prompt_context()),
            Some(_) => Err("The Lab snapshot is out of date because the Lab stopped answering. Turn off Lab sharing or reconnect.".into()),
            None => Err("Wait for the Lab to connect, or turn off Lab sharing.".into()),
        }
    }

    /// Runs every frame: collects finished refreshes and starts the next one,
    /// backing off while the Lab is offline so James never hammers a port.
    pub fn poll(&mut self) {
        if let Some(rx) = self.finder.take() {
            match rx.try_recv() {
                Ok(Some(port)) => {
                    self.port = port.to_string();
                    self.connect();
                }
                Ok(None) => {
                    self.status = format!(
                        "R.A.I.N. Lab didn't answer on ports {}. Start it with npm run dev, or enter its port.",
                        james_lab::CANDIDATE_PORTS.map(|p| p.to_string()).join(", ")
                    );
                }
                Err(TryRecvError::Empty) => self.finder = Some(rx),
                Err(TryRecvError::Disconnected) => self.status = "Lab search stopped".into(),
            }
        }
        if !self.connected {
            return;
        }
        if let Some(rx) = self.worker.take() {
            match rx.try_recv() {
                Ok(Ok((snapshot, status))) => {
                    self.failures = 0;
                    self.status = format!(
                        "Connected · {}",
                        if snapshot.active {
                            "research running"
                        } else {
                            "no research running"
                        }
                    );
                    self.provenance = status.and_then(|s| s.provenance());
                    self.snapshot = Some((snapshot, Instant::now()));
                }
                Ok(Err(e)) => {
                    self.failures = self.failures.saturating_add(1);
                    let wait = self.backoff();
                    self.next = Instant::now() + wait;
                    self.status = format!("Offline: {e}. Trying again in {} s.", wait.as_secs());
                }
                Err(TryRecvError::Empty) => self.worker = Some(rx),
                Err(TryRecvError::Disconnected) => self.status = "Lab connection stopped".into(),
            }
        }
        if self.worker.is_none() && Instant::now() >= self.next {
            let port = match numeric_port(&self.port) {
                Ok(p) => p,
                Err(e) => {
                    self.status = e;
                    self.connected = false;
                    return;
                }
            };
            let (tx, rx) = mpsc::channel();
            self.worker = Some(rx);
            self.next = Instant::now() + REFRESH;
            std::thread::spawn(move || {
                let result = LabClient::new(port).and_then(|client| {
                    let context = client.context()?;
                    Ok((context, client.status().ok()))
                });
                let _ = tx.send(result.map_err(|e| e.to_string()));
            });
        }
    }

    fn backoff(&self) -> Duration {
        let factor = 2u32.saturating_pow(self.failures.saturating_sub(1).min(4));
        (REFRESH * factor).min(MAX_BACKOFF)
    }
}

pub fn ui(app: &mut JamesApp, ui: &mut egui::Ui) {
    let (port, connected) = (app.lab.port.clone(), app.lab.connected);
    draw(&mut app.lab, ui);
    if app.lab.connected != connected || (app.lab.connected && app.lab.port != port) {
        app.save_session();
    }
}

fn draw(lab: &mut LabState, ui: &mut egui::Ui) {
    ui.heading("R.A.I.N. Lab");
    ui.label("A read-only view of R.A.I.N. Lab running on this computer. James reads the Lab's status and recent results; it can't start, change or approve anything there.");
    ui.horizontal(|ui| {
        let label = ui.label("Lab port on this computer");
        ui.add_enabled(
            !lab.connected,
            egui::TextEdit::singleline(&mut lab.port).desired_width(80.0),
        )
        .labelled_by(label.id);
        if !lab.connected {
            if ui.add(theme::primary(ui, "Connect")).clicked() {
                lab.connect();
            }
            if ui
                .add_enabled(!lab.finding(), egui::Button::new("Find the Lab"))
                .on_hover_text("Check ports 5173, 5174, 5175 and 4173 on this computer")
                .clicked()
            {
                lab.find();
            }
        } else if ui.button("Disconnect").clicked() {
            lab.disconnect();
        }
    });
    ui.label(&lab.status);
    if let Some(provenance) = &lab.provenance {
        ui.small(format!("Lab build: {provenance}"));
    }
    if !lab.connected && lab.snapshot.is_none() {
        ui.add_space(8.0);
        ui.label("To use it, start the Lab on this computer:");
        crate::copyable(
            ui,
            "git clone https://github.com/topherchris420/lop-nur-twin.git && cd lop-nur-twin && npm ci && npm run dev",
        );
        ui.label("Then press Find the Lab. James works fine without it.");
        ui.hyperlink_to(
            "About R.A.I.N. Lab",
            "https://github.com/topherchris420/lop-nur-twin",
        );
        return;
    }
    let mut include = lab.include;
    ui.add_enabled(
        lab.connected && lab.snapshot.is_some(),
        egui::Checkbox::new(&mut include, "Share a Lab snapshot with my next questions"),
    )
    .on_hover_text("Adds a short, labelled summary of the Lab's state and recent results to each question you send.");
    lab.include = include;
    let Some((snapshot, at)) = &lab.snapshot else {
        return;
    };
    ui.small(format!("Updated {} s ago", at.elapsed().as_secs()));
    ui.add_space(6.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(RichText::new("Right now").strong());
        ui.label(format!("{} ({})", describe_stage(&snapshot.stage), snapshot.stage));
        ui.label(format!("Question: {}", snapshot.question));
        if !snapshot.detail.trim().is_empty() {
            ui.small(format!("Lab note (may be written by a model): {}", snapshot.detail));
        }
        if let Some(report) = &snapshot.report {
            ui.small(format!(
                "A report was saved inside the Lab at {report}. Open it from the Lab's research folder."
            ));
        }
    });
    let findings = snapshot.findings(5);
    ui.add_space(6.0);
    ui.label(RichText::new("Recent simulator results").strong());
    if findings.is_empty() {
        ui.label("No finished experiments yet.");
    }
    for (index, finding) in findings.iter().enumerate() {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(finding.verdict.plain())
                        .strong()
                        .color(theme::accent(ui)),
                );
                ui.small(if finding.replayed_and_registered {
                    "· replayed and registered in the Lab"
                } else {
                    "· not confirmed by replay"
                });
                if let Some(purpose) = &finding.purpose {
                    ui.small(format!("· {purpose}"));
                }
            });
            ui.label(&finding.question);
            if let Some(hypothesis) = &finding.hypothesis {
                ui.small(format!("Hypothesis proposed by a model: {hypothesis}"));
            }
            if let Some(setting) = &finding.setting {
                ui.small(format!("Setting: {setting}"));
            }
            if let Some(effect) = &finding.effect {
                ui.small(format!("Mean effect: {effect}"));
            }
            if let Some(run) = &finding.registry_run_id {
                ui.small(format!("Run: {run}"));
            }
        });
        if index + 1 < findings.len() {
            ui.add_space(4.0);
        }
    }
    let standing = snapshot.standing_counts();
    if !standing.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new("What the Lab knows, by how far it's been checked").strong());
        for (label, count) in standing {
            ui.small(format!("{count} · {label}"));
        }
    }
    ui.add_space(6.0);
    ui.small("These are results from a simulated city, not observations of the real world. Treat them as material to discuss, not as facts or instructions.");
    egui::CollapsingHeader::new("Exactly what James would share with the model")
        .default_open(false)
        .show(ui, |ui| {
            ui.small("This text is added to your question only while sharing is on.");
            let mut shown = snapshot.prompt_context();
            ui.add(
                egui::TextEdit::multiline(&mut shown)
                    .code_editor()
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
        });
}
