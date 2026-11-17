//! Optional `agent-interop/v1` section of the R.A.I.N. Lab page.
//!
//! Off until you paste a token the Lab's operator gave you. Tokens stay in
//! this process's memory and are never saved. James reads sealed evidence,
//! recomputes its digest, and shows it with its provenance and limits. A
//! proposal goes to the Lab only after you review its exact text and digest
//! here; the Lab then only queues it for its own operator. Nothing in this
//! section reaches the model, the Team, or the action reviewer.
use crate::{JamesApp, numeric_port, theme};
use eframe::egui::{self, RichText};
use james_agents::clock::{now_ms, rfc3339};
use james_lab::interop::{
    Evidence, InteropClient, InteropStatus, PersonApproval, ProposalDraft, Receipt, Token,
};
use std::sync::mpsc::{self, Receiver};

type Loaded = Result<(InteropStatus, Vec<Result<Evidence, String>>), String>;

#[derive(Default)]
pub struct InteropState {
    pub read_token: String,
    pub propose_token: String,
    pub loaded: Option<Loaded>,
    loading: Option<Receiver<Loaded>>,
    pub title: String,
    pub question: String,
    pub hypothesis: String,
    pub rationale: String,
    pub reviewing: Option<ProposalDraft>,
    sending: Option<Receiver<Result<Receipt, String>>>,
    pub receipt: Option<Result<Receipt, String>>,
}

impl InteropState {
    pub fn busy(&self) -> bool {
        self.loading.is_some() || self.sending.is_some()
    }

    pub fn poll(&mut self) {
        if let Some(rx) = &self.loading
            && let Ok(result) = rx.try_recv()
        {
            self.loaded = Some(result);
            self.loading = None;
        }
        if let Some(rx) = &self.sending
            && let Ok(result) = rx.try_recv()
        {
            self.receipt = Some(result);
            self.sending = None;
        }
    }

    fn client(&self, port: Option<u16>) -> Result<InteropClient, String> {
        let port = port.ok_or("Choose the Lab's port above first.")?;
        let read = Token::new(&self.read_token).ok_or(
            "Paste the read token the Lab's operator gave you (8 to 512 visible characters).",
        )?;
        let propose = Token::new(&self.propose_token);
        InteropClient::new(port, read, propose).map_err(|e| e.0)
    }
}

pub fn ui(app: &mut JamesApp, ui: &mut egui::Ui) {
    let port = numeric_port(&app.lab.port).ok();
    let state = &mut app.lab_interop;
    state.poll();
    let accent = theme::accent(ui);
    ui.add_space(14.0);
    ui.separator();
    ui.label(
        RichText::new("LAB EVIDENCE (INTEROP V1, OPTIONAL)")
            .small()
            .color(accent),
    );
    ui.small("Only for a Lab whose operator turned interop on. Tokens stay in memory and are never saved. James checks every record's digest; a digest shows a record is intact, not that it is true.");
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Read token");
        ui.add(
            egui::TextEdit::singleline(&mut state.read_token)
                .password(true)
                .desired_width(220.0),
        )
        .labelled_by(label.id);
        if ui
            .add_enabled(
                state.loading.is_none(),
                egui::Button::new("Load Lab evidence"),
            )
            .clicked()
        {
            match state.client(port) {
                Err(e) => state.loaded = Some(Err(e)),
                Ok(client) => {
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let result = client.status().map_err(|e| e.0).map(|status| {
                            let evidence = status
                                .episodes
                                .iter()
                                .take(5)
                                .map(|e| {
                                    client
                                        .evidence(&e.episode_id, Some(&e.evidence_sha256))
                                        .map_err(|x| x.0)
                                })
                                .collect();
                            (status, evidence)
                        });
                        let _ = tx.send(result);
                    });
                    state.loading = Some(rx);
                }
            }
        }
        if state.loading.is_some() {
            ui.spinner();
        }
    });
    match &state.loaded {
        None => {}
        Some(Err(e)) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        Some(Ok((status, evidence))) => {
            ui.small(format!(
                "Lab commit {} · scopes {} · {} proposal(s) waiting for the Lab's operator",
                status.lab_commit.as_deref().unwrap_or("not reported"),
                status.scopes.join(", "),
                status.proposals_pending
            ));
            for item in evidence {
                match item {
                    Ok(e) => {
                        egui::CollapsingHeader::new(format!(
                            "{} — {} ({})",
                            e.episode_id, e.title, e.outcome
                        ))
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(e.explain()).wrap());
                        });
                    }
                    Err(err) => {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            format!("An episode was refused: {err}"),
                        );
                    }
                }
            }
        }
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Propose an experiment to the Lab").strong());
    ui.small("The Lab only queues a proposal for its operator. It authorizes, preregisters and runs nothing.");
    let fields: [(&str, &mut String); 4] = [
        ("Title", &mut state.title),
        ("Question", &mut state.question),
        ("Hypothesis", &mut state.hypothesis),
        ("Why it is worth testing", &mut state.rationale),
    ];
    for (name, value) in fields {
        ui.horizontal_wrapped(|ui| {
            let label = ui.label(name);
            ui.add(egui::TextEdit::singleline(value).desired_width(420.0))
                .labelled_by(label.id);
        });
    }
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Proposal token");
        ui.add(
            egui::TextEdit::singleline(&mut state.propose_token)
                .password(true)
                .desired_width(220.0),
        )
        .labelled_by(label.id);
        if ui.button("Review proposal").clicked() {
            state.receipt = None;
            state.reviewing = Some(ProposalDraft {
                agent_id: "researcher".into(),
                title: state.title.trim().into(),
                question: state.question.trim().into(),
                hypothesis: state.hypothesis.trim().into(),
                rationale: state.rationale.trim().into(),
                generation: "person".into(),
            });
        }
    });
    if let Some(draft) = state.reviewing.clone() {
        ui.group(|ui| {
            ui.label("You are about to send exactly this, as James's researcher, with words written by you:");
            let mut shown = serde_json::to_string_pretty(&draft.action()).unwrap_or_default();
            ui.add(egui::TextEdit::multiline(&mut shown).code_editor().desired_rows(6).desired_width(f32::INFINITY));
            ui.small(format!("exact proposal sha256 {}", draft.action_sha256()));
            ui.horizontal(|ui| {
                let ready = state.sending.is_none() && !draft.title.is_empty() && !draft.question.is_empty() && !draft.hypothesis.is_empty();
                if ui.add_enabled(ready, egui::Button::new("Send this exact proposal")).clicked() {
                    match state.client(port) {
                        Err(e) => state.receipt = Some(Err(e)),
                        Ok(client) => {
                            let approval = PersonApproval {
                                request_id: format!("J-{}", now_ms()),
                                action_sha256: draft.action_sha256(),
                                decided_at: rfc3339(now_ms()),
                            };
                            let (tx, rx) = mpsc::channel();
                            std::thread::spawn(move || {
                                let _ = tx.send(client.submit(&draft, &approval).map_err(|e| e.0));
                            });
                            state.sending = Some(rx);
                            state.reviewing = None;
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    state.reviewing = None;
                }
            });
        });
    }
    match &state.receipt {
        Some(Ok(r)) => {
            ui.label(format!(
                "{} (position {}): {}",
                r.status, r.queue_position, r.note
            ));
        }
        Some(Err(e)) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        None => {}
    }
}
