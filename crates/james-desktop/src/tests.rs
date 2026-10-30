//! Unit tests for the desktop's decisions, and GUI tests that drive the real
//! interface headlessly through its accessibility tree (egui_kittest).
use super::*;

mod gui;

#[test]
fn transcript_only_includes_conversation_roles() {
    let text = chat::transcript(&[
        ChatMessage::new("system", "private instructions"),
        ChatMessage::new("user", "Question"),
        ChatMessage::new("assistant", "```rust\nlet x = 1;\n```"),
        ChatMessage::new("tool", "internal record"),
    ]);
    assert_eq!(
        text,
        "# Conversation with James\n\n## You\n\nQuestion\n\n## James\n\n```rust\nlet x = 1;\n```\n"
    );
}

#[test]
fn allow_once_refuses_a_model_click_until_the_person_checked_it() {
    let dir = session::scratch_dir("second-look");
    let mut app = JamesApp::new(dir.join("session.json"));
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |_| {});
    app.screen = Some(Screen {
        capture: Screenshot {
            png: Vec::new(),
            width: 100,
            height: 100,
            target: None,
            james_hidden: Some(true),
        },
        texture: ctx.load_texture(
            "test-screen",
            egui::ColorImage::new([4, 4], egui::Color32::WHITE),
            egui::TextureOptions::LINEAR,
        ),
        at: Instant::now(),
    });
    app.propose(Action::Click { x: 10, y: 10 }, "Press Send".into(), true);
    app.allow_once();
    assert!(
        app.scheduled.is_none(),
        "an unchecked model click is never scheduled"
    );
    assert!(app.pending.is_some(), "the review stays for the person");
    assert!(
        app.status.contains("I checked what this does"),
        "{}",
        app.status
    );
    app.pending.as_mut().unwrap().checked = true;
    app.allow_once();
    assert!(app.scheduled.is_some());
    // No frame is drawn, so the countdown never ends and nothing is sent.
    app.scheduled = None;
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failed_retry_restores_the_conversation_draft_and_attachment() {
    let dir = session::scratch_dir("retry-failure");
    let mut app = JamesApp::new(dir.join("session.json"));
    app.messages = vec![
        ChatMessage::new("user", "An earlier question"),
        ChatMessage::new("assistant", "An earlier answer"),
    ];
    app.draft = "My next question".into();
    app.attach_screen = true;
    let messages = app.messages.clone();
    // No model is selected, so send refuses before launching a worker.
    app.ask_again();
    assert_eq!(app.messages, messages);
    assert_eq!(app.draft, "My next question");
    assert!(app.attach_screen);
    assert!(!app.busy());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_partial_reply_at_the_message_limit_survives_saving_and_restart() {
    let dir = session::scratch_dir("partial-limit");
    let path = dir.join("session.json");
    let mut app = JamesApp::new(path.clone());
    app.messages.push(ChatMessage::new("user", "A question"));
    app.asking = Some(Ask::Chat);
    app.streaming = Some("x".repeat(session::MAX_MESSAGE_BYTES));
    assert!(app.keep_partial_reply("Stopped by you before the reply finished."));
    assert_eq!(app.messages.len(), 2);
    let reply = &app.messages[1];
    assert!(reply.content.len() <= session::MAX_MESSAGE_BYTES);
    assert!(reply.content.contains("Message shortened"));
    assert_eq!(session::load(&path).messages, app.messages);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn release_notes_show_once_after_an_upgrade() {
    let changelog = "# Changelog\n\nIntro.\n\n## Unreleased\n\n- Next.\n\n\
        ## 1.2.0 — 2026-01-01\n\nSee [the guide](docs/GUIDE.md) or [the site](https://x.org/).\n\n\
        ### Added\n\n- A thing.\n\n## 1.1.0 — 2025-12-01\n\n- Older.\n";
    let notes = release_notes(changelog, "1.2.0").unwrap();
    assert!(
        notes.starts_with(&format!("See [the guide]({SOURCE_URL}docs/GUIDE.md)")),
        "{notes}"
    );
    assert!(notes.contains("[the site](https://x.org/)"), "{notes}");
    assert!(notes.ends_with("- A thing."), "{notes}");
    // A version matches whole: 1.2 is not 1.2.0.
    assert_eq!(release_notes(changelog, "1.2"), None);
    assert_eq!(release_notes(changelog, "9.9.9"), None);
    // Only an upgrade shows them: not a first run, and not a restart.
    assert!(upgrade_notes("1.1.0", "1.2.0", changelog).is_some());
    assert_eq!(upgrade_notes("", "1.2.0", changelog), None);
    assert_eq!(upgrade_notes("1.2.0", "1.2.0", changelog), None);
}

#[test]
fn this_version_ships_its_release_notes() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        release_notes(CHANGELOG, version).is_some(),
        "CHANGELOG.md needs a \"## {version}\" section before this version is built"
    );
}

#[test]
fn stop_before_the_connection_exists_still_closes_it_at_once() {
    use std::io::{Read, Write};
    // A model server that accepts, then says nothing for ten seconds.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0_u8; 4096];
        let _ = socket.read(&mut buf);
        std::thread::sleep(Duration::from_secs(10));
        let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
    });
    let (tx, rx) = mpsc::channel();
    let job = Job {
        cancel: Cancellation::default(),
        tx,
        abort: AbortSlot::default(),
    };
    // Stop arrives first, while the worker has not connected yet.
    job.cancel.cancel();
    job.abort.abort();
    let started = Instant::now();
    let result = LocalClient::new(port).unwrap().chat_stream(
        "model",
        &[ChatMessage::new("user", "Hello")],
        None,
        |event| job.on_stream(event),
    );
    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the late connection must be closed on arrival: {:?}",
        started.elapsed()
    );
    // A cancelled job reports no progress.
    job.send(WorkerResult::Delta("late".into()));
    assert!(rx.try_recv().is_err());
    drop(server);
}

#[test]
fn failures_come_with_a_next_step_and_the_servers_own_words() {
    let message = explain(
        LocalProvider::LmStudio,
        1234,
        "gemma",
        ApiError::Unreachable("port 1234: connection refused".into()),
    );
    assert!(message.starts_with("James can't reach LM Studio on port 1234."));
    assert!(message.contains("Developer tab"));
    assert!(!message.contains("refused"), "{message}");
    let missing = explain(
        LocalProvider::Ollama,
        11434,
        "gemma3:4b",
        ApiError::Transport(
            "Local model server returned HTTP 404: model 'gemma3:4b' not found".into(),
        ),
    );
    assert!(
        missing.starts_with("Ollama doesn't have the model “gemma3:4b” ready."),
        "{missing}"
    );
    assert!(missing.contains("ollama pull gemma3:4b"));
    assert!(missing.contains("The server said: Local model server returned HTTP 404"));
    let full = explain(
        LocalProvider::LlamaCpp,
        8080,
        "qwen",
        ApiError::Transport("Local model server returned HTTP 400: request (3234 tokens) exceeds the available context size (1024 tokens), try increasing it".into()),
    );
    assert!(
        full.starts_with("This conversation no longer fits"),
        "{full}"
    );
    let images = explain(
        LocalProvider::LlamaCpp,
        8081,
        "qwen",
        ApiError::Transport("Local model server returned HTTP 500: image input is not supported - hint: if this is unexpected, you may need to provide the mmproj".into()),
    );
    assert!(images.starts_with("“qwen” can't read images."), "{images}");
    let cloud = explain(
        LocalProvider::Ollama,
        11434,
        "gpt-oss:20b-cloud",
        ApiError::Transport("Local model server returned HTTP 401: Unauthorized".into()),
    );
    assert!(cloud.contains("James never signs in for you"), "{cloud}");
    // Anything unrecognised is shown exactly as the server said it.
    let other = explain(
        LocalProvider::Ollama,
        11434,
        "x",
        ApiError::Protocol("Malformed JSON in the Ollama stream".into()),
    );
    assert_eq!(other, "Malformed JSON in the Ollama stream");
}

#[test]
fn find_prefers_a_server_with_a_local_chat_model() {
    let (chosen, note) = describe_found(&[], false);
    assert!(chosen.is_none());
    assert!(note.contains("1234, 11434, 8080"), "{note}");

    let empty = FoundServer {
        provider: LocalProvider::LmStudio,
        port: 1234,
        models: Vec::new(),
        remote: Vec::new(),
    };
    let (chosen, note) = describe_found(std::slice::from_ref(&empty), false);
    assert_eq!(chosen, Some(&empty));
    assert!(note.contains("no models yet"), "{note}");
    // A working setup is never swapped for a server with nothing to run.
    let (chosen, note) = describe_found(std::slice::from_ref(&empty), true);
    assert!(chosen.is_none());
    assert!(
        note.ends_with("Your current model settings are unchanged."),
        "{note}"
    );

    // A server offering only cloud models is reported, never chosen as working.
    let cloud_only = FoundServer {
        provider: LocalProvider::Ollama,
        port: 11434,
        models: vec!["gpt-oss:20b-cloud".into()],
        remote: vec!["gpt-oss:20b-cloud".into()],
    };
    let (chosen, note) = describe_found(std::slice::from_ref(&cloud_only), true);
    assert!(chosen.is_none());
    assert!(
        note.contains("only lists models that run on another computer"),
        "{note}"
    );

    let ollama = FoundServer {
        provider: LocalProvider::Ollama,
        port: 11434,
        models: vec!["nomic-embed-text:latest".into(), "qwen3:4b".into()],
        remote: Vec::new(),
    };
    let servers = [empty, cloud_only, ollama.clone()];
    let (chosen, note) = describe_found(&servers, true);
    assert_eq!(chosen, Some(&ollama));
    assert!(note.contains("Ollama on port 11434"), "{note}");
    assert!(note.contains("qwen3:4b"), "{note}");
}

#[test]
fn a_panic_on_any_thread_leaves_a_private_report() {
    let dir = session::scratch_dir("crash");
    let path = session::crash_report_path(&dir.join("session.json"));
    record_panics(path.clone());
    let result = std::thread::Builder::new()
        .name("worker".into())
        .spawn(|| panic!("simulated failure"))
        .unwrap()
        .join();
    // Restore the default hook for the rest of the test binary.
    drop(std::panic::take_hook());
    assert!(result.is_err());
    let report = std::fs::read_to_string(&path).unwrap();
    assert!(report.starts_with(&format!("James {} on ", env!("CARGO_PKG_VERSION"))));
    assert!(report.contains("thread worker"), "{report}");
    assert!(report.contains("simulated failure"), "{report}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_bundled_icon_decodes() {
    let icon = app_icon().expect("icon");
    assert_eq!((icon.width, icon.height), (256, 256));
    assert_eq!(icon.rgba.len(), 256 * 256 * 4);
}

#[test]
fn endpoints_are_numeric_loopback_ports() {
    for bad in ["0", "65536", "https://host", "1234/path", ""] {
        assert!(numeric_port(bad).is_err());
    }
    assert_eq!(numeric_port(" 1234 ").unwrap(), 1234);
}

#[test]
fn screenshots_are_scaled_for_the_model_and_clicks_mapped_back() {
    assert_eq!(scaled_size(1920, 1080, 1600), (1600, 900));
    assert_eq!(scaled_size(3840, 2160, 1600), (1600, 900));
    assert_eq!(scaled_size(1080, 1920, 1600), (900, 1600));
    assert_eq!(scaled_size(1280, 800, 1600), (1280, 800));
    // The centre of the scaled image is the centre of the screen.
    assert_eq!(
        to_screen((800, 450), (1600, 900), (3840, 2160)),
        (1921, 1081)
    );
    assert_eq!(
        to_screen((1599, 899), (1600, 900), (3840, 2160)),
        (3838, 2158)
    );
    // Out-of-range model coordinates are clamped, then validated and reviewed.
    assert_eq!(
        to_screen((5000, 5000), (1600, 900), (1920, 1080)),
        (1919, 1079)
    );
    assert_eq!(to_screen((10, 20), (1280, 800), (1280, 800)), (10, 20));

    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2400, 1200, image::Rgba([10, 20, 30, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let small = prepare_image(png.get_ref(), 1600).unwrap();
    let decoded = image::load_from_memory(&small).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (1600, 800));
    assert!(prepare_image(b"not a png", 1600).is_err());
}

#[test]
fn history_is_trimmed_to_fit_a_known_context_window() {
    let messages: Vec<ChatMessage> = (0..40)
        .map(|i| {
            ChatMessage::new(
                if i % 2 == 0 { "user" } else { "assistant" },
                "x".repeat(1000),
            )
        })
        .collect();
    // Unknown window: the most recent 30 messages that fit 160 KB.
    let (recent, left_out) = recent_history(&messages, history_budget(None, 0));
    assert_eq!((recent.len(), left_out), (30, 10));
    // A 4,096-token window leaves room for about 9 KB of history here.
    let budget = history_budget(Some(4096), 500);
    assert_eq!(budget, (4096 - 1024) * 3 - 500);
    let (recent, left_out) = recent_history(&messages, budget);
    assert_eq!((recent.len(), left_out), (8, 32));
    assert_eq!(recent.last(), messages.last());
    // A window too small for anything sends no history rather than failing.
    assert_eq!(history_budget(Some(100), 10_000), 0);
}
