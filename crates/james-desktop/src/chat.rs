//! The conversation: who James is talking to, the messages, and the composer.
use crate::{Ask, JamesApp, Page, Tone, guide, markdown, pages, theme, voice};
use eframe::egui::{self, RichText};
use james_models::Support;

const EXAMPLES: [&str; 3] = [
    "Explain the difference between a hypothesis and a finding, with an example.",
    "Help me plan a small experiment to test whether my houseplants need more light.",
    "What should I check before trusting a result from a simulation?",
];

/// A compact line in the title bar: which model, through which app, and
/// whether its server says it runs somewhere else.
pub fn connection_badge(app: &mut JamesApp, ui: &mut egui::Ui) {
    let model = app.model.trim().to_owned();
    if model.is_empty() {
        if ui
            .button("No model connected")
            .on_hover_text("Open setup to connect a model on this computer")
            .clicked()
        {
            app.page = Page::Chat;
        }
        return;
    }
    let remote = app.info_now().and_then(|i| i.remote_host.clone());
    let (text, color) = match &remote {
        Some(_) => (
            format!("{model} · runs elsewhere"),
            ui.visuals().warn_fg_color,
        ),
        None => (
            format!("{model} · {}", app.provider.label()),
            theme::accent(ui),
        ),
    };
    let hover = match remote {
        Some(host) => format!(
            "{} on this computer (port {}) says this model runs on {host}. Your messages leave this computer through it.",
            app.provider.label(),
            app.port.trim()
        ),
        None => format!(
            "James talks only to {} on this computer (127.0.0.1, port {}). That app decides where the model runs; LM Studio, Ollama and llama.cpp run models on this computer unless you set them up otherwise.",
            app.provider.label(),
            app.port.trim()
        ),
    };
    if ui
        .add(egui::Button::new(RichText::new(text).color(color)).frame(false))
        .on_hover_text(hover)
        .clicked()
    {
        app.page = Page::Settings;
    }
}

/// What the server says about the current model, in one or two short lines.
pub fn model_facts(app: &JamesApp, ui: &mut egui::Ui) {
    let Some(info) = app.info_now() else {
        if app.info_worker.is_some() {
            ui.small("Asking the model server what this model can do…");
        }
        return;
    };
    if info.is_unknown() {
        ui.small("The model server didn't say what this model can do. Screen questions may or may not work.");
        return;
    }
    let mut facts = Vec::new();
    facts.push(match info.vision {
        Support::Yes => "can read images".to_owned(),
        Support::No => "can't read images".to_owned(),
        Support::Unknown => "image support not stated".to_owned(),
    });
    if info.chat == Support::No {
        facts.push("not a chat model".to_owned());
    }
    if let Some(tokens) = info.context_tokens {
        facts.push(format!("{tokens}-token context window"));
    }
    match info.loaded {
        Some(true) => facts.push("loaded".to_owned()),
        Some(false) => facts.push("loads on first use".to_owned()),
        None => {}
    }
    ui.small(format!(
        "{}: {} (according to {}).",
        app.model.trim(),
        facts.join(", "),
        app.provider.label()
    ))
    .on_hover_text(format!(
        "James asked {} on this computer ({}). It never guesses from a model's name.",
        app.provider.label(),
        info.source.unwrap_or("its model list")
    ));
    if let Some(host) = &info.remote_host {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "{} says this model runs on {host}, not on this computer. What you send it leaves this computer.",
                app.provider.label()
            ),
        );
    }
    if info.chat == Support::No {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "This model can't chat (it is probably an embedding model). Choose another model in Settings.",
        );
    }
}

/// The screenshot a Guide me answer is about, with its steps marked, while
/// that answer and that screenshot are both current.
fn guide_screenshot(app: &JamesApp, ui: &mut egui::Ui, answer: &str) {
    let (Some(shown), Some(screen)) = (&app.guide_marks, &app.screen) else {
        return;
    };
    if shown.answer != answer || shown.screenshot != screen.at {
        return;
    }
    ui.add_space(4.0);
    let response = ui.add(
        egui::Image::new(&screen.texture)
            .max_size(egui::vec2(ui.available_width(), 260.0))
            .maintain_aspect_ratio(true),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Image,
            true,
            "Your screenshot with the steps marked",
        )
    });
    let size = (screen.capture.width, screen.capture.height);
    pages::mark_steps(ui, response.rect, &shown.marks, size);
    ui.small("Your model placed these numbers, and they may be off: find the label each step names before you click.");
}

fn message_ui(app: &mut JamesApp, ui: &mut egui::Ui, index: usize, last_answer: bool) {
    let message = &app.messages[index];
    let user = message.role == "user";
    let accent = theme::accent(ui);
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 8))
        .corner_radius(6.0)
        .fill(if user {
            ui.visuals().faint_bg_color
        } else {
            ui.visuals().panel_fill
        });
    let mut copy = false;
    let mut again = false;
    let mut read = false;
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if user { "You" } else { "James" })
                    .strong()
                    .color(accent),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                copy = ui
                    .small_button("Copy")
                    .on_hover_text("Copy this message")
                    .clicked();
                if !user && last_answer {
                    again = ui
                        .add_enabled(!app.busy(), egui::Button::new("Ask again").small())
                        .on_hover_text("Send your last question again and replace this answer. A screenshot is not sent again.")
                        .clicked();
                }
                if !user {
                    read = ui
                        .add_enabled(!app.busy(), egui::Button::new("Read aloud").small())
                        .clicked();
                }
            });
        });
        if user {
            ui.label(&message.content);
        } else {
            markdown::show(ui, &message.content, ("message", index));
            if last_answer {
                guide_screenshot(app, ui, &message.content);
            }
        }
    });
    if copy {
        ui.ctx().copy_text(app.messages[index].content.clone());
        app.info("Copied.");
    }
    if again {
        app.ask_again();
    }
    if read {
        let text = voice::speakable(&app.messages[index].content);
        app.speak(text);
    }
}

/// A notice about an internal error in an earlier run, with the saved
/// report so the user can choose to include it in a bug report.
fn crash_notice(app: &mut JamesApp, ui: &mut egui::Ui) {
    let Some(path) = &app.crash_report else {
        return;
    };
    let mut dismiss = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "James ran into an internal error in an earlier session.",
        );
        ui.label("A short report was saved on this computer. It contains the James version, your operating system and where the error happened, not your conversations. Nothing was sent anywhere.");
        ui.small(format!("Report: {}", path.display()));
        ui.horizontal(|ui| {
            ui.hyperlink_to("Report the problem", crate::ISSUES_URL);
            if ui.button("Dismiss and delete the report").clicked() {
                dismiss = true;
            }
        });
    });
    if dismiss && let Some(path) = app.crash_report.take() {
        let _ = std::fs::remove_file(path);
    }
    ui.add_space(8.0);
}

pub fn conversation(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.heading("Conversation");
    model_facts(app, ui);
    if !app.keep_history {
        ui.small("This conversation isn't saved: it disappears when you close James (Settings > Privacy).");
    }
    ui.separator();
    crash_notice(app, ui);
    if app.recovery.needs_check() {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "An approved action hasn't been checked yet.",
            );
            if ui.button("Review it on Computer control").clicked() {
                app.page = Page::Actions;
            }
        });
    }
    if app.messages.is_empty() && app.streaming.is_none() && !app.thinking {
        ui.add_space(24.0);
        ui.label(RichText::new("A laboratory without walls.").size(22.0));
        ui.label(format!(
            "You're talking to {} through {} on this computer. Type below and press Send, or try one of these:",
            app.model.trim(),
            app.provider.label()
        ));
        ui.add_space(6.0);
        for example in EXAMPLES {
            if ui
                .button(example)
                .on_hover_text("Put this question in the message box")
                .clicked()
            {
                app.draft = example.to_owned();
                app.focus_composer = true;
            }
        }
        ui.add_space(6.0);
        ui.small("Guide me looks at your screen and suggests next steps. Talk starts a spoken conversation. Answers come from your model and can be wrong.");
        return;
    }
    let last_answer = app.messages.iter().rposition(|m| m.role == "assistant");
    for index in 0..app.messages.len() {
        // Ask again can remove messages while the list is being drawn.
        if index >= app.messages.len() {
            break;
        }
        ui.add_space(8.0);
        message_ui(
            app,
            ui,
            index,
            Some(index) == last_answer && index + 1 == app.messages.len(),
        );
    }
    if app.thinking || app.streaming.is_some() {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("James").strong().color(theme::accent(ui)));
            theme::busy(ui, app.prefs.reduce_motion);
            ui.small(if app.streaming.is_some() {
                "replying as the model writes"
            } else {
                "the model is thinking before it answers"
            });
        });
        if let Some(partial) = &app.streaming {
            let partial = if app.asking == Some(Ask::Guide) {
                guide::hide_partial_marks(partial)
            } else {
                partial
            };
            markdown::show(ui, partial, "streaming");
        }
    }
    if let Some(note) = &app.history_note {
        ui.add_space(6.0);
        ui.small(note);
    }
    // The last question got no answer (an error, or James closed mid-reply).
    if app.worker.is_none()
        && app.messages.last().is_some_and(|m| m.role == "user")
        && ui
            .add(egui::Button::new("Try again"))
            .on_hover_text("Send your last question again")
            .clicked()
    {
        app.ask_again();
    }
}

/// The message box and the buttons under it. Stop is first and prominent
/// whenever something is running.
pub fn composer(app: &mut JamesApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("composer").show(ctx, |ui| {
        ui.add_space(6.0);
        let send_keys = if cfg!(target_os = "macos") { "⌘ Return" } else { "Ctrl+Enter" };
        let compose = ui.add(
            egui::TextEdit::multiline(&mut app.draft)
                .id(egui::Id::new("message-box"))
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text(format!("Ask James… ({send_keys} sends)")),
        );
        compose.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Message to James")
        });
        if std::mem::take(&mut app.focus_composer) {
            compose.request_focus();
        }
        let shortcut = compose.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
        ui.horizontal_wrapped(|ui| {
            let busy = app.busy();
            if busy {
                if ui
                    .add(theme::danger(ui, "Stop (Esc)"))
                    .on_hover_text("Stops the reply, listening, speaking, capture or the countdown. Input that was already sent cannot be undone.")
                    .clicked()
                {
                    app.stop();
                }
            } else if app.pending.is_some()
                && ui
                    .add(theme::danger(ui, "Cancel action"))
                    .clicked()
            {
                app.stop();
            }
            let can_send = !busy && !app.draft.trim().is_empty();
            if ui
                .add_enabled(can_send, theme::primary(ui, "Send"))
                .on_hover_text(format!("Send ({send_keys})"))
                .clicked()
                || (can_send && shortcut)
            {
                app.send(Ask::Chat);
                app.focus_composer = true;
            }
            let can_listen = app.listening.is_ready();
            let listen_hint = if can_listen {
                "Finish the current task first.".to_owned()
            } else {
                format!("{} Set up listening in Settings.", app.listening.describe())
            };
            if app.voice == voice::Phase::Off {
                if ui
                    .add_enabled(!busy && can_listen, egui::Button::new("Talk"))
                    .on_hover_text("Hands-free conversation: speak, hear the answer, keep going. Say \"stop\" or press Esc to end.")
                    .on_disabled_hover_text(&listen_hint)
                    .clicked()
                {
                    app.talk();
                }
            } else if ui.button("End conversation").clicked() {
                app.stop();
                app.info("Conversation ended.");
            }
            let sees = app.vision() != Support::No;
            let image_hint = format!(
                "“{}” can't read images, according to its server. Choose a vision model, such as Gemma 3, in Settings.",
                app.model.trim()
            );
            if ui
                .add_enabled(!busy && sees, egui::Button::new("Guide me"))
                .on_hover_text("James takes one snapshot of your screen and explains what to do next. It never clicks or types for you.")
                .on_disabled_hover_text(if sees { "Finish the current task first." } else { image_hint.as_str() })
                .clicked()
            {
                app.guide(ctx);
            }
            let can_act = james_platform::can_control_input();
            if ui
                .add_enabled(
                    !busy && app.screen.is_some() && can_act && sees && !app.recovery.needs_check(),
                    egui::Button::new("Suggest one action"),
                )
                .on_hover_text("The model proposes one click, key or text for you to review. Nothing happens until you choose Allow once.")
                .on_disabled_hover_text(if !can_act {
                    "Computer input is Windows-only for now. Use Guide me instead."
                } else if !sees {
                    image_hint.as_str()
                } else if app.recovery.needs_check() {
                    "Check the last action on Computer control first."
                } else {
                    "Capture a screen first (Screen context)."
                })
                .clicked()
            {
                app.send(Ask::Plan);
            }
            if ui
                .add_enabled(!busy && can_listen, egui::Button::new("Dictate"))
                .on_hover_text("Speak once; the words go into the message box for you to check.")
                .on_disabled_hover_text(&listen_hint)
                .clicked()
            {
                app.dictate_once();
            }
            if app.attach_screen {
                ui.label(RichText::new("Screenshot attached").color(theme::accent(ui)));
            }
            if app.lab.include {
                ui.label(RichText::new("Lab snapshot attached").color(theme::accent(ui)));
            }
        });
        if let Some(w) = &app.worker {
            ui.horizontal(|ui| {
                theme::busy(ui, app.prefs.reduce_motion);
                ui.small(w.label);
            });
        }
        let color = match app.tone {
            Tone::Info => ui.visuals().text_color(),
            Tone::Warn => ui.visuals().warn_fg_color,
            Tone::Error => ui.visuals().error_fg_color,
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&app.status).small().color(color));
            if app.tone == Tone::Error {
                ui.hyperlink_to(RichText::new("Troubleshooting").small(), crate::TROUBLE_URL);
            }
        });
        ui.add_space(6.0);
    });
}
