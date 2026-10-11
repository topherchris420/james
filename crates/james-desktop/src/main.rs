//! James desktop milestone 1: local text chat, explicit model selection,
//! human-readable status, and local conversation storage. No tool executor.
use eframe::egui::{self, Color32, RichText};
use james_models::{ChatMessage, LocalClient};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

const LAB_URL: &str = "https://github.com/topherchris420/lop-nur-twin";
const TEAL: Color32 = Color32::from_rgb(62, 193, 178);
const SYSTEM_CONTEXT: &str = "You are James, the evidence-oriented lead research assistant of Vers3Dynamics R.A.I.N. Lab. Work with intellectual humility. Distinguish evidence, hypotheses, and uncertainty. The R.A.I.N. Lab's other research perspectives are Jasmine, Luca, and Elena. This desktop application is currently NOT connected to the Lab, its papers, experiments, its memory or files. Do not claim you inspected or recalled them. You can discuss information the user directly provides. Do not claim you can access the screen, run tools, or change files. The user retains authority over every real-world action.";
const MAX_SAVED_MESSAGES: usize = 80;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct SavedSession {
    port: u16,
    model: String,
    messages: Vec<ChatMessage>,
}

impl Default for SavedSession {
    fn default() -> Self {
        Self {
            port: 1234,
            model: String::new(),
            messages: Vec::new(),
        }
    }
}

fn session_path() -> PathBuf {
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path)
            .join("Vers3Dynamics")
            .join("James")
            .join("session.json");
    }
    if let Some(path) = std::env::var_os("HOME") {
        return PathBuf::from(path).join(".james").join("session.json");
    }
    PathBuf::from("james-session.json")
}

fn load_session(path: &PathBuf) -> SavedSession {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<SavedSession>(&content).ok())
        .unwrap_or_default()
}

enum WorkerResult {
    Models(Result<Vec<String>, String>),
    Chat(Result<String, String>),
}

struct JamesApp {
    port: String,
    model: String,
    available_models: Vec<String>,
    messages: Vec<ChatMessage>,
    draft: String,
    status: String,
    worker: Option<Receiver<WorkerResult>>,
    session: PathBuf,
}

impl Default for JamesApp {
    fn default() -> Self {
        let session = session_path();
        let saved = load_session(&session);
        Self {
            port: saved.port.to_string(),
            model: saved.model,
            available_models: Vec::new(),
            messages: saved.messages.into_iter().rev().take(MAX_SAVED_MESSAGES).collect::<Vec<_>>()
                .into_iter().rev().collect(),
            draft: String::new(),
            status: "Start LM Studio's Local Server, then discover models.".to_owned(),
            worker: None,
            session,
        }
    }
}

impl JamesApp {
    fn current_port(&self) -> Result<u16, String> {
        self.port.parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| "Enter a valid localhost port (1–65535).".to_owned())
    }

    fn save(&mut self) {
        let port = match self.current_port() {
            Ok(port) => port,
            Err(_) => return,
        };
        let data = SavedSession {
            port,
            model: self.model.clone(),
            messages: self.messages.iter().rev().take(MAX_SAVED_MESSAGES)
                .cloned().collect::<Vec<_>>().into_iter().rev().collect(),
        };
        let result = (|| -> Result<(), String> {
            if let Some(parent) = self.session.parent() {
                std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            let data = serde_json::to_vec_pretty(&data).map_err(|err| err.to_string())?;
            std::fs::write(&self.session, data).map_err(|err| err.to_string())?;
            Ok(())
        })();
        if let Err(error) = result {
            self.status = format!("Could not save local conversation: {error}");
        }
    }

    fn discover(&mut self) {
        let Ok(port) = self.current_port() else {
            self.status = "Invalid port. Use LM Studio's numeric localhost port.".into();
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        self.status = format!("Checking 127.0.0.1:{port} for loaded models...");
        std::thread::spawn(move || {
            let outcome = LocalClient::new(port)
                .and_then(|client| client.models())
                .map_err(|err| err.to_string());
            let _ = tx.send(WorkerResult::Models(outcome));
        });
    }

    fn send(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let text = self.draft.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let Ok(port) = self.current_port() else {
            self.status = "Select a numeric localhost port before chatting.".into();
            return;
        };
        let model = self.model.trim().to_owned();
        if model.is_empty() {
            self.status = "Load a model in LM Studio and click Discover models first.".into();
            return;
        }
        self.messages.push(ChatMessage::new("user", text));
        self.draft.clear();
        self.save();
        let mut context = vec![ChatMessage::new("system", SYSTEM_CONTEXT)];
        let recent: Vec<_> = self.messages.iter()
            .rev().take(39).cloned().collect::<Vec<_>>()
            .into_iter().rev().collect();
        context.extend(recent);
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        self.status = format!("Asking {model} through 127.0.0.1:{port}...");
        std::thread::spawn(move || {
            let outcome = LocalClient::new(port)
                .and_then(|client| client.chat(&model, &context))
                .map_err(|err| err.to_string());
            let _ = tx.send(WorkerResult::Chat(outcome));
        });
    }

    fn receive(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.worker.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(WorkerResult::Models(Ok(models))) => {
                self.available_models = models;
                if self.available_models.is_empty() {
                    self.status = "Server connected, but no models were reported. Load one in LM Studio.".into();
                } else {
                    if !self.available_models.contains(&self.model) {
                        self.model = self.available_models[0].clone();
                    }
                    self.status = format!(
                        "Connected to localhost. {} model(s) found.",
                        self.available_models.len()
                    );
                }
                self.save();
            }
            Ok(WorkerResult::Models(Err(error))) => {
                self.status = error;
            }
            Ok(WorkerResult::Chat(Ok(reply))) => {
                self.messages.push(ChatMessage::new("assistant", reply));
                self.status = "Response received from the selected local endpoint.".into();
                self.save();
            }
            Ok(WorkerResult::Chat(Err(error))) => {
                self.status = format!("Chat failed. Your message remains in history. {error}");
            }
            Err(TryRecvError::Disconnected) => {
                self.status = "Model worker stopped without a response.".into();
            }
            Err(TryRecvError::Empty) => {
                self.worker = Some(rx);
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
    }
}

impl eframe::App for JamesApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive(ctx);
        let busy = self.worker.is_some();

        egui::TopBottomPanel::top("brand").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("JAMES").size(26.0).strong().color(TEAL));
                ui.separator();
                ui.label("R.A.I.N. Lab · Desktop research companion");
            });
            ui.add_space(10.0);
        });

        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(260.0)
            .min_width(210.0)
            .show(ctx, |ui| {
                ui.heading("Local model");
                ui.label("LM Studio Local Server");
                ui.label("127.0.0.1 only · HTTP");
                ui.horizontal(|ui| {
                    ui.label("Port");
                    ui.text_edit_singleline(&mut self.port);
                });
                if ui.add_enabled(!busy, egui::Button::new("Discover models")).clicked() {
                    self.discover();
                }
                ui.add_space(8.0);
                ui.label("Model ID");
                ui.text_edit_singleline(&mut self.model);
                if !self.available_models.is_empty() {
                    egui::ComboBox::from_id_salt("loaded-models")
                        .selected_text(self.model.as_str())
                        .show_ui(ui, |ui| {
                            for available in &self.available_models {
                                ui.selectable_value(&mut self.model, available.clone(), available);
                            }
                        });
                }
                if ui.add_enabled(!busy, egui::Button::new("Save model choice")).clicked() {
                    self.save();
                    self.status = "Local settings saved.".into();
                }

                ui.separator();
                ui.heading("His laboratory");
                ui.label("James is R.A.I.N. Lab's evidence-oriented lead researcher alongside Jasmine, Luca and Elena.");
                ui.hyperlink_to("Open R.A.I.N. Lab project", LAB_URL);
                ui.small("Live Lab memory and experiment retrieval are not connected in this milestone.");
                ui.separator();
                ui.heading("Privacy & authority");
                ui.small("Chats go only to the configured loopback endpoint. Verify the backend itself uses local inference.");
                ui.small("Conversation text is stored unencrypted on this computer. No screen capture, tools, voice or automatic cloud fallback.");
                if ui.add_enabled(!busy, egui::Button::new("Clear saved conversation")).clicked() {
                    self.messages.clear();
                    self.save();
                    self.status = "Conversation cleared from James's local session file.".into();
                }
                ui.separator();
                ui.small(format!("Storage: {}", self.session.display()));
            });

        egui::TopBottomPanel::bottom("composer").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("Ask James").strong());
            let compose = ui.add(
                egui::TextEdit::multiline(&mut self.draft)
                    .desired_rows(3)
                    .hint_text("Ask a question, propose an experiment, or discuss evidence..."),
            );
            let shortcut = compose.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl);
            ui.horizontal(|ui| {
                if ui.add_enabled(!busy, egui::Button::new("Send to local model")).clicked() || (shortcut && !busy) {
                    self.send();
                }
                if busy {
                    ui.spinner();
                    ui.label("Working...");
                } else {
                    ui.small("Ctrl+Enter to send");
                }
            });
            ui.separator();
            ui.label(RichText::new(&self.status).small().color(TEAL));
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Conversation");
            ui.small("Research dialogue only. Model output is not verified scientific evidence.");
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if self.messages.is_empty() {
                        ui.add_space(30.0);
                        ui.label(RichText::new("A laboratory without walls.").size(21.0));
                        ui.add_space(8.0);
                        ui.label("Connect a locally loaded model to begin a conversation with James.");
                    }
                    for item in &self.messages {
                        let (name, color) = if item.role == "user" {
                            ("YOU", Color32::LIGHT_GRAY)
                        } else {
                            ("JAMES", TEAL)
                        };
                        ui.add_space(10.0);
                        ui.label(RichText::new(name).strong().color(color));
                        ui.label(&item.content);
                        ui.add_space(8.0);
                        ui.separator();
                    }
                });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 760.0])
            .with_min_inner_size([720.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "James | R.A.I.N. Lab",
        options,
        Box::new(|_cc| Ok(Box::<JamesApp>::default())),
    )
}
