//! GUI tests: the real interface, driven headlessly through its
//! accessibility tree, the way a keyboard or screen-reader user reaches it.
//! Model and Lab servers are scripted sockets on random loopback ports.
use super::super::*;
use egui_kittest::{Harness, kittest::Queryable};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};

type Chunks = Vec<(Duration, Vec<u8>)>;
type Handler = dyn Fn(&str, &str) -> Chunks + Send + Sync;

/// A scripted local server. `handler` gets each request line and body and
/// returns what to write, with an optional pause before each piece.
fn fake_server(handler: impl Fn(&str, &str) -> Chunks + Send + Sync + 'static) -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handler: Arc<Handler> = Arc::new(handler);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let handler = Arc::clone(&handler);
            std::thread::spawn(move || {
                let mut socket = stream;
                let _ = socket.set_read_timeout(Some(Duration::from_secs(10)));
                let mut reader = BufReader::new(socket.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    return;
                }
                let request = line.trim().to_owned();
                let mut length = 0;
                loop {
                    line.clear();
                    if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':')
                        && name.eq_ignore_ascii_case("content-length")
                    {
                        length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; length];
                let _ = reader.read_exact(&mut body);
                for (pause, bytes) in handler(&request, &String::from_utf8_lossy(&body)) {
                    std::thread::sleep(pause);
                    if socket.write_all(&bytes).is_err() {
                        return;
                    }
                }
            });
        }
    });
    port
}

fn json(body: &str) -> Chunks {
    vec![(
        Duration::ZERO,
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes(),
    )]
}

fn chunk(data: &str) -> Vec<u8> {
    format!("{:x}\r\n{data}\r\n", data.len()).into_bytes()
}

/// An Ollama-style streamed reply, one NDJSON line per piece.
fn ndjson(pieces: &[&str], pause: Duration) -> Chunks {
    let mut out = vec![(
        Duration::ZERO,
        b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec(),
    )];
    for piece in pieces {
        let line =
            serde_json::json!({"message": {"role": "assistant", "content": piece}, "done": false});
        out.push((pause, chunk(&format!("{line}\n"))));
    }
    out.push((
        Duration::ZERO,
        chunk("{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true}\n"),
    ));
    out.push((Duration::ZERO, b"0\r\n\r\n".to_vec()));
    out
}

/// An Ollama with `gemma3:4b` (reads images), a text-only model and a cloud
/// model. Chat replies come from `reply`.
fn ollama(reply: impl Fn(&str) -> Chunks + Send + Sync + 'static) -> u16 {
    fake_server(move |request, body| {
        if request.starts_with("GET /api/tags") {
            json(
                r#"{"models":[{"name":"gemma3:4b"},{"name":"qwen2.5:0.5b"},{"name":"gpt-oss:20b-cloud","remote_host":"https://ollama.com"}]}"#,
            )
        } else if request.starts_with("POST /api/show") {
            if body.contains("gemma3") {
                json(r#"{"capabilities":["completion","vision"]}"#)
            } else {
                json(r#"{"capabilities":["completion","tools"]}"#)
            }
        } else if request.starts_with("GET /api/ps") {
            json(r#"{"models":[{"name":"gemma3:4b","context_length":4096}]}"#)
        } else if request.starts_with("POST /api/chat") {
            reply(body)
        } else {
            vec![(
                Duration::ZERO,
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            )]
        }
    })
}

fn closed_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A James with its own scratch profile, set up with `model` on `port`.
fn james_on(port: u16, model: Option<&str>, step: SetupStep) -> JamesApp {
    let dir = session::scratch_dir("gui");
    let path = dir.join("session.json");
    let saved = SavedSession {
        provider: LocalProvider::Ollama,
        port,
        model: model.unwrap_or_default().into(),
        setup: Some(Setup {
            step,
            test_passed: step == SetupStep::Done,
        }),
        ..SavedSession::default()
    };
    session::save(&path, &saved).unwrap();
    JamesApp::new(path)
}

fn james(model: Option<&str>) -> JamesApp {
    let step = if model.is_some() {
        SetupStep::Done
    } else {
        SetupStep::Welcome
    };
    james_on(closed_port(), model, step)
}

fn harness(app: JamesApp) -> Harness<'static, JamesApp> {
    harness_sized(app, egui::vec2(1120.0, 820.0), 1.0)
}

fn harness_sized(
    app: JamesApp,
    size: egui::Vec2,
    pixels_per_point: f32,
) -> Harness<'static, JamesApp> {
    let mut h = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(pixels_per_point)
        .build_state(|ctx, app: &mut JamesApp| app.ui(ctx), app);
    frames(&mut h, 3);
    h
}

fn frames(h: &mut Harness<'static, JamesApp>, count: usize) {
    for _ in 0..count {
        h.step();
    }
}

/// Steps frames until `done`, letting background threads finish.
fn wait_until(h: &mut Harness<'static, JamesApp>, what: &str, done: impl Fn(&JamesApp) -> bool) {
    for _ in 0..500 {
        h.step();
        if done(h.state()) {
            frames(h, 2);
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}; status: {}", h.state().status);
}

fn type_message(h: &mut Harness<'static, JamesApp>, text: &str) {
    h.get_by_label("Message to James").click();
    frames(h, 1);
    h.get_by_label("Message to James").type_text(text);
    frames(h, 1);
}

#[test]
fn typing_in_the_message_box_fills_the_draft() {
    let mut h = harness(james(Some("gemma3:4b")));
    type_message(&mut h, "Hello there");
    assert_eq!(h.state().draft, "Hello there");
}

#[test]
fn first_run_setup_saves_each_step_and_resumes_after_a_restart() {
    let mut h = harness(james(None));
    assert!(h.query_by_label("Welcome to James").is_some());
    // The composer is hidden until setup is done: one thing at a time.
    assert!(h.query_by_label("Message to James").is_none());
    h.get_by_label("Get started").click();
    frames(&mut h, 2);
    assert_eq!(h.state().setup.step, SetupStep::ModelApp);
    assert!(h.query_by_label("Get a model app").is_some());
    // Closing James here and opening it again resumes at the same step.
    let path = h.state().session.clone();
    drop(h);
    let mut h = harness(JamesApp::new(path));
    assert!(h.query_by_label("Get a model app").is_some());
    h.get_by_label("Back").click();
    frames(&mut h, 2);
    assert_eq!(h.state().setup.step, SetupStep::Welcome);
}

#[test]
fn setup_finds_a_model_says_hello_and_keeps_the_test_out_of_history() {
    let port = ollama(|_| ndjson(&["Hello! ", "I'm working."], Duration::from_millis(30)));
    let mut h = harness(james_on(port, Some("gemma3:4b"), SetupStep::Test));
    wait_until(&mut h, "the model facts", |app| app.info_now().is_some());
    assert_eq!(h.state().vision(), Support::Yes);
    h.get_by_label("Send a test message").click();
    wait_until(&mut h, "the test reply", |app| app.setup_reply.is_some());
    assert_eq!(
        h.state().setup_reply,
        Some(Ok("Hello! I'm working.".to_owned()))
    );
    assert!(h.state().setup.test_passed);
    assert!(
        h.state().messages.is_empty(),
        "the test is not a conversation"
    );
    assert!(
        h.query_by_label_contains("It works. gemma3:4b answered in")
            .is_some()
    );
    h.get_by_label("Next: screen and safety").click();
    frames(&mut h, 2);
    h.get_by_label("Finish setup").click();
    frames(&mut h, 2);
    assert_eq!(h.state().setup.step, SetupStep::Done);
    // The conversation is ready, with the message box focused for typing.
    assert!(h.get_by_label("Message to James").is_focused());
}

#[test]
fn a_reply_streams_in_renders_markdown_and_is_saved() {
    let port = ollama(|_| {
        ndjson(
            &["**Evidence** first.\n\n", "1. Replicate\n", "2. Compare"],
            Duration::from_millis(40),
        )
    });
    let mut h = harness(james_on(port, Some("gemma3:4b"), SetupStep::Done));
    type_message(&mut h, "How do I check a result?");
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    wait_until(&mut h, "the reply", |app| {
        app.worker.is_none() && app.messages.len() == 2
    });
    let reply = &h.state().messages[1];
    assert_eq!(
        reply.content,
        "**Evidence** first.\n\n1. Replicate\n2. Compare"
    );
    // Markdown is drawn as text, without the asterisks.
    assert!(h.query_by_label("Evidence first.").is_some());
    assert!(h.query_by_label("Replicate").is_some());
    assert!(h.state().draft.is_empty());
    let saved = std::fs::read_to_string(&h.state().session).unwrap();
    assert!(saved.contains("How do I check a result?") && saved.contains("Replicate"));
}

#[test]
fn stop_closes_the_reply_and_keeps_what_arrived_marked_incomplete() {
    let port = ollama(|_| {
        let mut chunks = ndjson(&["First part. "], Duration::ZERO);
        // Then the server stalls for a long time before the rest.
        chunks.insert(
            2,
            (
                Duration::from_secs(20),
                chunk("{\"message\":{\"content\":\"never\"},\"done\":false}\n"),
            ),
        );
        chunks
    });
    let mut h = harness(james_on(port, Some("gemma3:4b"), SetupStep::Done));
    type_message(&mut h, "Tell me a long story");
    h.get_by_label("Send").click();
    wait_until(&mut h, "the first words", |app| app.streaming.is_some());
    assert!(
        h.query_by_label("Stop (Esc)").is_some(),
        "Stop is shown while busy"
    );
    let started = Instant::now();
    h.press_key(egui::Key::Escape);
    wait_until(&mut h, "the stop", |app| app.worker.is_none());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "Stop must not wait for the server"
    );
    let last = h.state().messages.last().unwrap();
    assert!(last.content.starts_with("First part."), "{}", last.content);
    assert!(
        last.content
            .ends_with("[Stopped by you before the reply finished.]")
    );
    assert!(h.state().status.starts_with("Stopped."));
}

#[test]
fn a_model_server_that_is_not_running_gets_advice_and_a_retry() {
    let mut h = harness(james(Some("gemma3:4b")));
    type_message(&mut h, "Hello?");
    h.get_by_label("Send").click();
    wait_until(&mut h, "the failure", |app| app.worker.is_none());
    assert!(
        h.state()
            .status
            .starts_with("James can't reach Ollama on port"),
        "{}",
        h.state().status
    );
    assert_eq!(h.state().tone, Tone::Error);
    // The question is kept and can be sent again once the server is running.
    assert_eq!(h.state().messages.len(), 1);
    assert!(h.query_by_label("Troubleshooting").is_some());
    assert!(h.query_by_label("Try again").is_some());
}

#[test]
fn ask_again_replaces_the_last_answer() {
    let count = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&count);
    let port = ollama(move |_| {
        let n = seen.fetch_add(1, Ordering::SeqCst) + 1;
        ndjson(&[&format!("Answer number {n}.")], Duration::ZERO)
    });
    let mut h = harness(james_on(port, Some("gemma3:4b"), SetupStep::Done));
    type_message(&mut h, "Pick a number");
    h.get_by_label("Send").click();
    wait_until(&mut h, "the first answer", |app| {
        app.messages.len() == 2 && app.worker.is_none()
    });
    h.get_by_label("Ask again").click();
    wait_until(&mut h, "the second answer", |app| {
        app.worker.is_none()
            && app
                .messages
                .last()
                .is_some_and(|m| m.content == "Answer number 2.")
    });
    let messages = &h.state().messages;
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert_eq!(messages[0].content, "Pick a number");
}

#[test]
fn a_text_only_model_cannot_be_asked_about_the_screen() {
    let port = ollama(|_| ndjson(&["ok"], Duration::ZERO));
    let mut h = harness(james_on(port, Some("qwen2.5:0.5b"), SetupStep::Done));
    wait_until(&mut h, "the model facts", |app| app.info_now().is_some());
    assert_eq!(h.state().vision(), Support::No);
    assert!(h.get_by_label("Guide me").is_disabled());
    assert!(h.query_by_label_contains("can't read images").is_some());
}

#[test]
fn a_cloud_model_is_flagged_before_anything_is_sent() {
    let port = ollama(|_| ndjson(&["ok"], Duration::ZERO));
    let mut h = harness(james_on(port, Some("gpt-oss:20b-cloud"), SetupStep::Done));
    wait_until(&mut h, "the model facts", |app| app.info_now().is_some());
    assert_eq!(
        h.state().info_now().unwrap().remote_host.as_deref(),
        Some("https://ollama.com")
    );
    assert!(
        h.query_by_label_contains("runs on https://ollama.com, not on this computer")
            .is_some()
    );
    assert!(
        h.query_by_label("gpt-oss:20b-cloud · runs elsewhere")
            .is_some()
    );
}

#[test]
fn the_keyboard_alone_reaches_every_main_control() {
    let mut h = harness(james(Some("gemma3:4b")));
    let mut reached = Vec::new();
    for _ in 0..40 {
        h.press_key(egui::Key::Tab);
        frames(&mut h, 1);
        if let Some(node) = h.query_by(|node| node.is_focused())
            && let Some(label) = node.label()
        {
            reached.push(label);
        }
    }
    for control in [
        "Conversation",
        "Screen context",
        "Computer control",
        "Settings",
        "Message to James",
        "Guide me",
    ] {
        assert!(
            reached.iter().any(|l| l == control),
            "Tab never reached {control}: {reached:?}"
        );
    }
}

#[test]
fn shortcuts_switch_pages_and_show_their_own_list() {
    let mut h = harness(james(Some("gemma3:4b")));
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Num5);
    frames(&mut h, 2);
    assert_eq!(h.state().page, Page::Settings);
    h.press_key(egui::Key::F1);
    frames(&mut h, 2);
    assert!(h.query_by_label("Send your message").is_some());
    h.press_key(egui::Key::Escape);
    frames(&mut h, 2);
    assert!(!h.state().show_shortcuts);
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::L);
    frames(&mut h, 2);
    assert_eq!(h.state().page, Page::Chat);
    assert!(h.get_by_label("Message to James").is_focused());
}

#[test]
fn deleting_everything_asks_first_and_keeps_an_unchecked_action_record() {
    let mut app = james(Some("gemma3:4b"));
    app.messages
        .push(ChatMessage::new("user", "private question"));
    app.save_session();
    let session = app.session.clone();
    let intent = session::intent_path(&session);
    std::fs::write(&intent, r#"{"schema":"james-action-intent/v2","action":{"kind":"key","key":"tab"},"state":"requires-human-verification"}"#).unwrap();
    app.recovery = review::read_intent(&intent);
    let mut h = harness(app);
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Num5);
    frames(&mut h, 2);
    h.get_by_label("Delete all James data…").click();
    frames(&mut h, 2);
    assert!(session.exists(), "nothing is deleted before confirming");
    assert!(
        h.query_by_label_contains("The record of the last approved action stays")
            .is_some()
    );
    h.get_by_label("Delete my James data").click();
    frames(&mut h, 2);
    assert!(!session.exists());
    assert!(
        intent.exists(),
        "an unchecked action record is never deleted silently"
    );
    assert!(h.state().messages.is_empty());
    assert_eq!(h.state().setup.step, SetupStep::Welcome);
}

#[test]
fn turning_off_history_removes_the_saved_conversation() {
    let mut app = james(Some("gemma3:4b"));
    app.messages
        .push(ChatMessage::new("user", "keep this private"));
    app.save_session();
    let mut h = harness(app);
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Num5);
    frames(&mut h, 2);
    h.get_by_label("Keep my conversations on this computer")
        .click();
    frames(&mut h, 2);
    let saved = std::fs::read_to_string(&h.state().session).unwrap();
    assert!(!saved.contains("keep this private"));
    // Still on screen until James closes.
    assert_eq!(h.state().messages.len(), 1);
}

#[test]
fn an_unchecked_action_is_explained_and_cleared_only_by_the_person() {
    let app = james(Some("gemma3:4b"));
    let intent = session::intent_path(&app.session);
    std::fs::write(&intent, r#"{"schema":"james-action-intent/v2","id":"1","recorded_at":1,"action":{"kind":"type_text","text":"hello"},"screen_width":100,"screen_height":100,"target":{"window":5,"process_id":42,"process_name":"notepad","title":"Untitled"},"state":"requires-human-verification"}"#).unwrap();
    // A fresh start finds the record left by a crash.
    let mut h = harness(JamesApp::new(app.session.clone()));
    drop(app);
    assert!(
        h.query_by_label("An approved action hasn't been checked yet.")
            .is_some()
    );
    h.get_by_label("Review it on Computer control").click();
    frames(&mut h, 2);
    assert!(
        h.query_by_label("James was asked to: Type “hello” (5 characters).")
            .is_some()
    );
    assert!(h.query_by_label("In: “Untitled” (notepad)").is_some());
    assert!(
        h.query_by_label_contains("James stopped before it learned")
            .is_some()
    );
    h.get_by_label("I checked the app — continue").click();
    frames(&mut h, 2);
    assert!(!intent.exists());
    assert!(!h.state().recovery.needs_check());
}

#[test]
fn the_layout_holds_at_small_sizes_high_dpi_large_text_and_every_theme() {
    for (size, ppp) in [
        (egui::vec2(760.0, 560.0), 1.0),
        (egui::vec2(1120.0, 820.0), 2.0),
        (egui::vec2(1920.0, 1080.0), 1.25),
    ] {
        for (scale, high_contrast, theme) in [
            (1.0, false, session::ThemeChoice::Dark),
            (1.5, true, session::ThemeChoice::Light),
            (2.0, false, session::ThemeChoice::System),
        ] {
            let mut app = james(Some("gemma3:4b"));
            app.prefs.text_scale = scale;
            app.prefs.high_contrast = high_contrast;
            app.prefs.theme = theme;
            app.prefs.reduce_motion = true;
            let mut h = harness_sized(app, size, ppp);
            for page in [
                Page::Chat,
                Page::Screen,
                Page::Actions,
                Page::Lab,
                Page::Settings,
            ] {
                h.state_mut().page = page;
                frames(&mut h, 2);
            }
            h.state_mut().page = Page::Chat;
            frames(&mut h, 2);
            // The core controls exist and fit inside the window.
            for control in ["Message to James", "Guide me", "Settings"] {
                let node = h.get_by_label(control);
                let bounds = node.bounding_box().expect("visible control");
                let limit = (size.x * ppp, size.y * ppp);
                assert!(
                    bounds.x1 <= f64::from(limit.0) + 1.0 && bounds.y1 <= f64::from(limit.1) + 1.0,
                    "{control} at {bounds:?} is outside {size:?} @ {ppp} (text {scale})"
                );
            }
        }
    }
}

fn lab_server() -> u16 {
    fake_server(|request, _| {
        if request.starts_with("GET /api/rain/discovery ") {
            json(&serde_json::json!({
                "schema": "rain-discovery-view/v1", "available": true, "active": false, "stage": "STOPPED",
                "detail": "Completed bounded session", "question": "How do crowds respond to a metro closure?",
                "model": null, "report": "reports/DS-1.md",
                "history": [{"question": "Does a closure push people outward?", "hypothesis": "People walk farther",
                    "verdict": "supported", "replay": true, "registry_run_id": "V3D-EXP-0001-RUN-0001",
                    "purpose": "exploratory", "design": {"scenario": "metro_closure"}, "unit": "m",
                    "measurements": {"primary_delta_mean": 4.5}}],
                "knowledge": {"nodes": [{"standing": "verified_simulator"}, {"standing": "model_suggested"}]}
            }).to_string())
        } else if request.starts_with("GET /api/rain/status ") {
            json(
                r#"{"schema":"rain-bethesda/v2","kind":"status","configured":true,"reachable":true,"failure":null,"identity":{"rain":{"commit":"4871ef6314d5975d82f2ad94366a9dcae72b4241","dirty":false},"meeting_generation":"scripted"}}"#,
            )
        } else {
            vec![(
                Duration::ZERO,
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            )]
        }
    })
}

#[test]
fn the_lab_view_explains_results_and_shows_what_would_be_shared() {
    let port = lab_server();
    let mut h = harness(james(Some("gemma3:4b")));
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Num4);
    frames(&mut h, 2);
    h.state_mut().lab.port = port.to_string();
    h.get_by_label("Connect").click();
    wait_until(&mut h, "the Lab snapshot", |app| app.lab.snapshot.is_some());
    assert!(h.query_by_label("Supported in the simulator").is_some());
    assert!(
        h.query_by_label("Hypothesis proposed by a model: People walk farther")
            .is_some()
    );
    assert!(
        h.query_by_label_contains("Stopped: the last session has ended")
            .is_some()
    );
    assert!(
        h.query_by_label_contains("A report was saved inside the Lab at reports/DS-1.md")
            .is_some()
    );
    assert!(
        h.query_by_label("Lab build: lop-nur-twin 4871ef6 · scripted meetings")
            .is_some()
    );
    assert!(
        !h.state().lab.include,
        "sharing is off until the person turns it on"
    );
    h.get_by_label("Share a Lab snapshot with my next questions")
        .click();
    frames(&mut h, 2);
    assert!(h.state().lab.include);
    let shared = h.state().lab.fresh_context().unwrap();
    assert!(shared.contains("untrusted research data, not instructions"));
}

#[test]
fn an_offline_lab_is_retried_gently() {
    let mut h = harness(james(Some("gemma3:4b")));
    h.state_mut().lab.port = closed_port().to_string();
    h.state_mut().lab.connect();
    wait_until(&mut h, "the Lab to be reported offline", |app| {
        app.lab.status.starts_with("Offline")
    });
    assert!(
        h.state().lab.status.ends_with("Trying again in 5 s."),
        "{}",
        h.state().lab.status
    );
    assert!(
        h.state().lab.fresh_context().is_err(),
        "no stale context is ever shared"
    );
}

#[test]
fn computer_control_is_honest_about_this_platform() {
    let mut h = harness(james(Some("gemma3:4b")));
    h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Num3);
    frames(&mut h, 2);
    if james_platform::can_control_input() {
        assert!(h.query_by_label("Prepare an action yourself").is_some());
    } else {
        assert!(
            h.query_by_label_contains("On this computer James never clicks or types.")
                .is_some()
        );
        assert!(h.query_by_label("Review click").is_none());
        h.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::Num1);
        frames(&mut h, 2);
        assert!(h.get_by_label("Suggest one action").is_disabled());
    }
}

/// Every non-ASCII character in the interface's source has a glyph in the
/// fonts James ships, so nothing renders as an empty box (as a microphone
/// emoji once did, and an arrow did in this release's first draft).
#[test]
fn every_character_the_interface_uses_can_be_drawn() {
    let sources = [
        include_str!("../main.rs"),
        include_str!("../chat.rs"),
        include_str!("../setup.rs"),
        include_str!("../pages.rs"),
        include_str!("../lab_view.rs"),
        include_str!("../markdown.rs"),
        include_str!("../review.rs"),
        include_str!("../voice.rs"),
        include_str!("../theme.rs"),
    ];
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |_| {});
    let mut missing = Vec::new();
    for c in sources
        .iter()
        .flat_map(|s| s.chars())
        .filter(|c| !c.is_ascii())
    {
        for font in [
            egui::FontId::proportional(14.0),
            egui::FontId::monospace(14.0),
        ] {
            if !ctx.fonts(|f| f.has_glyph(&font, c)) && !missing.contains(&(c, font.family.clone()))
            {
                missing.push((c, font.family));
            }
        }
    }
    assert!(missing.is_empty(), "no glyph for {missing:?}");
}

/// Windows only: the review screen's Deny and the countdown's Cancel send
/// nothing. (No test here ever lets the countdown finish.)
#[cfg(windows)]
mod review_screen {
    use super::*;

    fn with_screen(h: &mut Harness<'static, JamesApp>) {
        let texture = h.ctx.load_texture(
            "test-screen",
            egui::ColorImage::new([4, 4], egui::Color32::WHITE),
            egui::TextureOptions::LINEAR,
        );
        h.state_mut().screen = Some(Screen {
            capture: Screenshot {
                png: Vec::new(),
                width: 100,
                height: 100,
                target: None,
            },
            texture,
            at: Instant::now(),
        });
    }

    #[test]
    fn deny_sends_nothing_and_clears_the_review() {
        let mut h = harness(james(Some("gemma3:4b")));
        with_screen(&mut h);
        h.state_mut().propose(
            Action::Key { key: Key::Enter },
            "Submit the form".into(),
            true,
        );
        frames(&mut h, 2);
        assert!(h.query_by_label("Press Enter").is_some());
        assert!(h.query_by_label_contains("written by the model").is_some());
        h.get_by_label("Deny").click();
        frames(&mut h, 2);
        assert!(h.state().pending.is_none() && h.state().scheduled.is_none());
        assert_eq!(h.state().status, "Action denied. Nothing was sent.");
        assert!(!session::intent_path(&h.state().session).exists());
    }

    #[test]
    fn cancelling_the_countdown_sends_nothing() {
        let mut h = harness(james(Some("gemma3:4b")));
        with_screen(&mut h);
        h.state_mut()
            .propose(Action::Key { key: Key::Tab }, "Manual".into(), false);
        frames(&mut h, 2);
        h.get_by_label("Allow once").click();
        frames(&mut h, 2);
        assert!(h.state().scheduled.is_some());
        assert!(h.query_by_label("Cancel (Esc)").is_some());
        h.press_key(egui::Key::Escape);
        frames(&mut h, 2);
        assert!(h.state().scheduled.is_none());
        assert_eq!(
            h.state().status,
            "Pending action cancelled. Nothing was sent."
        );
        assert!(!session::intent_path(&h.state().session).exists());
    }
}
