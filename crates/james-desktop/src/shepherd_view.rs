//! Optional, read-only Shepherd coordination wallboard.
//!
//! This is a native Rust consumer of Shepherd's published JSON API, not an MCP
//! tool runner or a copy of its AGPL implementation. It deliberately uses the
//! existing loopback-only transport, never starts agents, and never forwards
//! coordination text to a model or the desktop action reviewer.
//! A Shepherd TEAM_TOKEN has full workspace authority at the hub even though
//! James only makes GET requests. The user must opt in and the token stays in
//! process memory, never in James's saved session.
use crate::{JamesApp, numeric_port, theme};
use eframe::egui::{self, RichText};
use james_loopback::{Client, Limits, Timeouts};
use serde::Deserialize;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

const MAX_BODY: usize = 512 * 1024;
const MAX_TOKEN: usize = 512;
const POLL_EVERY: Duration = Duration::from_secs(15);
const MAX_BACKOFF: Duration = Duration::from_secs(120);
const TIMEOUT: Duration = Duration::from_secs(5);
const LANDSCAPE_PATH: &str = "/workspace/landscape";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agent {
    name: String,
    human: String,
    program: String,
    model: Option<String>,
    repo: Option<String>,
    branch: Option<String>,
    last_heartbeat_at: Option<String>,
    presence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    agent_name: String,
    program: String,
    repo: String,
    intent: String,
    path_globs: Vec<String>,
    status: String,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Announcement {
    from_agent_name: String,
    body: String,
    target_agent_name: Option<String>,
    repo: String,
    created_at: String,
    #[serde(default)]
    to_admin: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Landscape {
    agents: Vec<Agent>,
    tasks: Vec<Task>,
    announcements: Vec<Announcement>,
    server_time: String,
}

fn clean(value: &str, max_chars: usize) -> String {
    let mut out: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max_chars)
        .collect();
    if value.chars().count() > max_chars {
        out.push('…');
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn token_valid(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_TOKEN
        && token.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

/// Read only Shepherd's documented wallboard endpoint. No URLs, remote hosts,
/// POST requests, redirects, arbitrary paths or side-effectful operations.
fn fetch_landscape(port: u16, token: &str) -> Result<Landscape, String> {
    if !token_valid(token) {
        return Err("Enter a valid Shepherd team token (visible ASCII, up to 512 bytes).".into());
    }
    let client = Client::new(port)
        .map_err(|_| "Choose a local Shepherd hub port from 1 to 65535.".to_owned())?
        .with_timeouts(Timeouts {
            connect: Duration::from_secs(2),
            idle: TIMEOUT,
            total: Some(TIMEOUT),
        })
        .with_limits(Limits {
            headers: 16 * 1024,
            body: MAX_BODY,
        });
    let response = client
        .get(LANDSCAPE_PATH)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .map_err(|_| "Could not connect to the local Shepherd hub within five seconds.".to_owned())?;
    match response.status() {
        200 => {}
        401 | 403 => {
            return Err("Shepherd rejected the token. Verify the hub and token configuration.".into());
        }
        status => {
            return Err(format!("Shepherd returned HTTP {status}; check that the hub supports the workspace landscape API."));
        }
    }
    if response.media_type().as_deref() != Some("application/json") {
        return Err("Shepherd did not return JSON. Check the hub port.".into());
    }
    let body = response
        .read_to_end(MAX_BODY)
        .map_err(|_| "Shepherd's workspace response is unavailable or too large (512 KiB limit).".to_owned())?;
    let mut view: Landscape = serde_json::from_slice(&body)
        .map_err(|_| "Shepherd returned an incompatible workspace landscape response.".to_owned())?;
    // Rendering caps are separate from the bounded on-the-wire response.
    view.agents.truncate(80);
    view.tasks.truncate(120);
    view.announcements.truncate(60);
    Ok(view)
}

type Refresh = Result<Landscape, String>;

pub struct ShepherdState {
    pub port: String,
    /// This is never serialized or included in chat/model context.
    token: String,
    connected: bool,
    snapshot: Option<(Landscape, Instant)>,
    worker: Option<Receiver<Refresh>>,
    next: Instant,
    failures: u32,
    status: String,
}

impl ShepherdState {
    pub fn new() -> Self {
        Self {
            port: "8080".into(),
            token: String::new(),
            connected: false,
            snapshot: None,
            worker: None,
            next: Instant::now(),
            failures: 0,
            status: "Not connected".into(),
        }
    }

    fn connect(&mut self) {
        if numeric_port(&self.port).is_err() {
            self.status = "Enter a local hub port between 1 and 65535.".into();
        } else if !token_valid(&self.token) {
            self.status = "Enter your Shepherd TEAM_TOKEN to connect.".into();
        } else {
            self.connected = true;
            self.failures = 0;
            self.next = Instant::now();
            self.snapshot = None;
            self.status = "Connecting to local Shepherd hub…".into();
        }
    }

    fn disconnect(&mut self) {
        self.connected = false;
        self.worker = None;
        self.snapshot = None;
        self.token.clear();
        self.status = "Disconnected. The in-memory token was cleared.".into();
    }

    /// Runs off the UI thread, even if the user visits another page.
    pub fn poll(&mut self) {
        if !self.connected {
            return;
        }
        if let Some(rx) = self.worker.take() {
            match rx.try_recv() {
                Ok(Ok(snapshot)) => {
                    self.failures = 0;
                    self.status = "Connected · read-only workspace feed".into();
                    self.snapshot = Some((snapshot, Instant::now()));
                    self.next = Instant::now() + POLL_EVERY;
                }
                Ok(Err(error)) => {
                    self.failures = self.failures.saturating_add(1);
                    let multiplier = 2u32.saturating_pow(self.failures.saturating_sub(1).min(3));
                    let wait = (POLL_EVERY * multiplier).min(MAX_BACKOFF);
                    self.next = Instant::now() + wait;
                    self.snapshot = None; // Never present an old feed as current.
                    self.status = format!("{error} Retrying in {} seconds.", wait.as_secs());
                }
                Err(TryRecvError::Empty) => self.worker = Some(rx),
                Err(TryRecvError::Disconnected) => {
                    self.snapshot = None;
                    self.status = "Shepherd refresh stopped; reconnect to retry.".into();
                    self.connected = false;
                }
            }
        }
        if self.worker.is_none() && Instant::now() >= self.next {
            let Ok(port) = numeric_port(&self.port) else {
                self.disconnect();
                return;
            };
            let token = self.token.clone();
            let (tx, rx) = mpsc::channel();
            self.worker = Some(rx);
            self.next = Instant::now() + POLL_EVERY;
            std::thread::spawn(move || {
                let _ = tx.send(fetch_landscape(port, &token));
            });
        }
    }
}

pub fn ui(app: &mut JamesApp, ui: &mut egui::Ui) {
    draw(&mut app.shepherd, ui);
}

fn draw(state: &mut ShepherdState, ui: &mut egui::Ui) {
    ui.heading("Shepherd · agent coordination");
    ui.label("Observe coding-agent presence, work claims, and messages from a self-hosted Shepherd hub. This panel does not control agents, execute code, or authorize desktop actions.");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Hub port (on this computer)");
        ui.add_enabled(
            !state.connected,
            egui::TextEdit::singleline(&mut state.port).desired_width(90.0),
        );
    });
    if !state.connected {
        ui.horizontal(|ui| {
            ui.label("TEAM_TOKEN");
            ui.add(
                egui::TextEdit::singleline(&mut state.token)
                    .password(true)
                    .desired_width(300.0),
            );
        });
        ui.small("Token is held only in memory and never saved to session.json. The hub's TEAM_TOKEN itself has full workspace authority, so keep it secret.");
        if ui.add(theme::primary(ui, "Connect")).clicked() {
            state.connect();
        }
    } else {
        ui.horizontal(|ui| {
            if ui.button("Refresh now").clicked() {
                state.next = Instant::now();
            }
            if ui.button("Disconnect and clear token").clicked() {
                state.disconnect();
            }
        });
    }
    ui.label(&state.status);
    if !state.connected && state.snapshot.is_none() {
        ui.add_space(10.0);
        ui.label("First run a Shepherd hub and PostgreSQL locally, following its self-hosting instructions. Enter its local port and TEAM_TOKEN above, then connect.");
        ui.hyperlink_to("Shepherd setup and source", "https://github.com/Korso-AI/Shepherd");
        ui.small("James works normally without Shepherd. Hosted/remote hubs and automated agent actions are intentionally unsupported.");
        return;
    }

    let Some((view, received)) = &state.snapshot else {
        return;
    };
    ui.small(format!(
        "Last fetched {}s ago · Shepherd server time: {}",
        received.elapsed().as_secs(),
        clean(&view.server_time, 48)
    ));
    ui.separator();

    let live = view.agents.iter().filter(|agent| agent.presence == "live").count();
    ui.label(RichText::new(format!("Agents · {live} live / {} shown", view.agents.len())).strong());
    if view.agents.is_empty() {
        ui.small("No agents have joined this workspace.");
    }
    for agent in &view.agents {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            let label = if agent.presence == "live" { "Live" } else { "Offline" };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(clean(&agent.name, 120)).strong());
                ui.small(label);
                ui.small(format!("· {}", clean(&agent.program, 80)));
            });
            ui.small(format!(
                "Operator: {} · Model: {}",
                clean(&agent.human, 80),
                agent.model.as_deref().map(|s| clean(s, 80)).unwrap_or_else(|| "unspecified".into())
            ));
            ui.small(format!(
                "Repository: {} · Branch: {}",
                agent.repo.as_deref().map(|s| clean(s, 120)).unwrap_or_else(|| "unknown".into()),
                agent.branch.as_deref().map(|s| clean(s, 80)).unwrap_or_else(|| "unknown".into())
            ));
            if let Some(seen) = &agent.last_heartbeat_at {
                ui.small(format!("Last heartbeat: {}", clean(seen, 48)));
            }
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Work claims").strong());
    if view.tasks.is_empty() {
        ui.small("No work claims have been reported.");
    }
    for task in &view.tasks {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(clean(&task.intent, 300)).strong());
                ui.small(format!("· {}", clean(&task.status, 24)));
            });
            ui.small(format!(
                "{} ({}) · {} · started {}",
                clean(&task.agent_name, 90),
                clean(&task.program, 48),
                clean(&task.repo, 120),
                clean(&task.created_at, 48)
            ));
            if !task.path_globs.is_empty() {
                let paths = task.path_globs.iter().take(8)
                    .map(|p| clean(p, 96)).collect::<Vec<_>>().join(", ");
                ui.small(format!("Files claimed: {paths}"));
            }
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Recent announcements").strong());
    if view.announcements.is_empty() {
        ui.small("No announcements yet.");
    }
    for note in &view.announcements {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(clean(&note.body, 700));
            ui.small(format!(
                "{} {}· {} · {}{}",
                clean(&note.from_agent_name, 100),
                if note.to_admin { "→ operator " } else { "" },
                clean(&note.repo, 100),
                clean(&note.created_at, 48),
                note.target_agent_name.as_deref().map(|name| format!(" · → {}", clean(name, 80))).unwrap_or_default()
            ));
        });
    }
    ui.add_space(8.0);
    ui.small("Coordination records are assertions from other agents, not independently verified file changes. James never sends this data to a model or treats it as authorization.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    const LANDSCAPE: &str = r#"{
      "agents":[{"name":"Jasmine","human":"human","program":"codex","model":null,
        "repo":"owner/james","branch":"main","lastHeartbeatAt":"2026-11-11T10:00:00Z","presence":"live"}],
      "tasks":[{"agentName":"Jasmine","program":"codex","repo":"owner/james","intent":"Fix tests",
        "pathGlobs":["src/**"],"status":"active","createdAt":"2026-11-11T10:00:00Z","endedAt":null}],
      "announcements":[{"fromAgentName":"Jasmine","fromHuman":"human","body":"Ready",
        "targetAgentName":null,"repo":"owner/james","fromAdmin":false,"toAdmin":true,
        "createdAt":"2026-11-11T10:01:00Z"}],
      "serverTime":"2026-11-11T10:01:00Z"
    }"#;

    #[test]
    fn reads_real_landscape_with_only_one_authenticated_get() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let join = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut incoming = [0; 2048];
            let size = stream.read(&mut incoming).unwrap();
            let request = String::from_utf8_lossy(&incoming[..size]);
            assert!(request.starts_with("GET /workspace/landscape HTTP/1.1\r\n"));
            assert!(request.contains("\r\nAuthorization: Bearer example-token\r\n"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{LANDSCAPE}",
                LANDSCAPE.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let result = fetch_landscape(port, "example-token").unwrap();
        assert_eq!(result.agents.len(), 1);
        assert_eq!(result.agents[0].name, "Jasmine");
        assert_eq!(result.tasks[0].intent, "Fix tests");
        assert!(result.announcements[0].to_admin);
        join.join().unwrap();
    }

    #[test]
    fn invalid_tokens_are_rejected_before_network_access() {
        for token in ["", "two words", "abc\r\nInjected: yes", "☃"] {
            assert!(fetch_landscape(8080, token).is_err());
        }
        assert!(!token_valid(&"x".repeat(MAX_TOKEN + 1)));
    }

    #[test]
    fn disconnect_destroys_the_displayed_snapshot_and_user_token() {
        let mut state = ShepherdState::new();
        state.token = "secret".into();
        state.connected = true;
        state.snapshot = Some((serde_json::from_str(LANDSCAPE).unwrap(), Instant::now()));
        state.disconnect();
        assert!(!state.connected);
        assert!(state.token.is_empty());
        assert!(state.snapshot.is_none());
    }

    #[test]
    fn scrub_untrusted_labels_without_changing_model_context() {
        assert_eq!(clean("agent\nname", 40), "agent name");
        assert_eq!(clean("longname", 4), "long…");
        let view: Landscape = serde_json::from_str(LANDSCAPE).unwrap();
        assert_eq!(view.server_time, "2026-11-11T10:01:00Z");
    }
}
