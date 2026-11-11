//! The conversation: who James is talking to, the messages, and the composer.
use crate::{Ask, JamesApp, Page, Tone, guide, markdown, pages, theme, voice};
use eframe::egui::{self, RichText};
use james_models::{ChatMessage, Support};

const EXAMPLES: [(&str, &str); 3] = [
    (
        "Check a claim",
        "Help me check a claim. What evidence would support it, and what would change my mind?",
    ),
    (
        "Plan an experiment",
        "Help me design a small experiment with a control and a measurable outcome.",
    ),
    (
        "Interpret a result",
        "What should I check before trusting a result from a simulation?",
    ),
];
const HISTORY_SEARCH: &str = "history-search";

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
        ui.close_menu();
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
                        .on_hover_text("Send your last question again and replace this answer. Documents, Lab context and screenshots from the earlier request are not restored; reattach documents for a source-based follow-up.")
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
    if index < app.messages.len() && app.messages[index].role == "assistant" {
        crate::evidence_view::answer(app, ui, index);
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

/// A portable Markdown transcript of the visible conversation. Screenshots,
/// unsent drafts, system prompts and action records are never included.
pub fn transcript(messages: &[ChatMessage]) -> String {
    let mut out = String::from("# Conversation with James\n");
    for message in messages {
        let name = match message.role.as_str() {
            "user" => "You",
            "assistant" => "James",
            _ => continue,
        };
        out.push_str(&format!("\n## {name}\n\n{}\n", message.content));
    }
    out
}

/// Stays above the scroll area so conversation controls remain reachable
/// even after a long reply.
pub fn header(app: &mut JamesApp, ui: &mut egui::Ui) {
    let narrow = app.uses_compact_menus(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        if !narrow {
            ui.heading("Conversation");
        }
        if ui
            .add_enabled(!app.busy(), egui::Button::new("History"))
            .clicked()
        {
            app.show_history = true;
            ui.data_mut(|data| data.insert_temp(egui::Id::new(HISTORY_SEARCH).with("focus"), true));
        }
        if narrow {
            ui.menu_button("Options", |ui| conversation_actions(app, ui));
        } else {
            conversation_actions(app, ui);
        }
    });
    if !narrow {
        model_facts(app, ui);
    }
    if !app.keep_history {
        ui.small("This conversation isn't saved: it disappears when you close James (Settings > Privacy).");
    }
    ui.separator();
}

fn conversation_actions(app: &mut JamesApp, ui: &mut egui::Ui) {
    if ui
        .add_enabled(
            !app.busy() && !app.messages.is_empty(),
            egui::Button::new("Copy conversation"),
        )
        .on_hover_text("Copy every message as Markdown, without screenshots or your unsent draft")
        .clicked()
    {
        ui.ctx().copy_text(transcript(&app.messages));
        app.info("Conversation copied as Markdown.");
        ui.close_menu();
    }
    if ui
        .add_enabled(
            !app.busy() && app.has_conversation_context(),
            egui::Button::new("New conversation"),
        )
        .on_hover_text("Start fresh and keep these messages in History (Ctrl+N, or ⌘N on Mac)")
        .clicked()
    {
        app.request_new_conversation();
        ui.close_menu();
    }
    if ui
        .add_enabled(app.receipt.is_some(), egui::Button::new("Request evidence"))
        .clicked()
    {
        app.show_evidence = true;
        ui.close_menu();
    }
}

pub fn history(app: &mut JamesApp, ctx: &egui::Context) {
    if !app.show_history {
        return;
    }
    let mut open = true;
    let mut resume = None;
    let mut remove = None;
    egui::Window::new("Conversation history")
        .open(&mut open)
        .default_width(440.0)
        .show(ctx, |ui| {
            ui.label(if app.keep_history {
                "Previous conversations saved on this computer."
            } else {
                "History is kept in memory until James closes."
            });
            ui.small("Up to 20 previous conversations within the shared history limit. Screenshots, drafts and approvals are not saved in history.");
            ui.horizontal_wrapped(|ui| {
                let search = ui.add(egui::TextEdit::singleline(&mut app.history_query)
                    .id(egui::Id::new(HISTORY_SEARCH))
                    .hint_text("Search titles and messages…")
                    .desired_width(260.0));
                search.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search conversation history"));
                if !ui.is_sizing_pass() && ui.data_mut(|data| data.remove_temp::<bool>(egui::Id::new(HISTORY_SEARCH).with("focus"))).unwrap_or_default() {
                    search.request_focus();
                }
                if !app.history_query.is_empty() && ui.button("Clear search").clicked() {
                    app.history_query.clear();
                    search.request_focus();
                }
            });
            let rows: Vec<_> = app.archives.iter()
                .filter(|conversation| crate::conversations::matches(conversation, &app.history_query))
                .map(|conversation| (conversation.id.clone(), conversation.title.clone(), conversation.messages.len()))
                .collect();
            if !app.archives.is_empty() && rows.is_empty() {
                ui.label("No conversations match. Try a different word.");
            }
            if let Some(id) = app.history_delete.clone() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label("Delete this saved conversation? This cannot be undone.");
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!app.busy(), theme::danger(ui, "Delete saved conversation")).clicked() {
                            remove = Some(id);
                        }
                        if ui.button("Keep conversation").clicked() {
                            app.history_delete = None;
                        }
                    });
                });
            }
            if app.archives.is_empty() {
                ui.add_space(10.0);
                ui.label("No previous conversations yet. Start a New conversation to keep this one here.");
            }
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for (id, title, count) in &rows {
                    ui.push_id(id, |ui| {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(RichText::new(title).strong());
                            ui.small(format!("{count} messages"));
                            ui.horizontal(|ui| {
                                if ui.add_enabled(!app.busy(), egui::Button::new("Open conversation")).clicked() {
                                    resume = Some(id.clone());
                                }
                                if ui.add_enabled(!app.busy(), egui::Button::new("Delete")).clicked() {
                                    app.history_delete = Some(id.clone());
                                }
                            });
                        });
                    });
                }
            });
        });
    app.show_history &= open;
    if let Some(id) = remove {
        crate::conversations::remove(&mut app.archives, &id);
        app.history_delete = None;
        if app.save_session() {
            app.info("Saved conversation deleted.");
        }
    }
    if let Some(id) = resume {
        app.request_conversation(id);
    }
}

pub fn conversation(app: &mut JamesApp, ui: &mut egui::Ui) {
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
        ui.heading("Ask James");
        ui.label("Start with a question or choose a prompt to edit.");
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (label, prompt) in EXAMPLES {
                if ui
                    .add_enabled(app.draft.is_empty(), egui::Button::new(label))
                    .on_hover_text(prompt)
                    .on_disabled_hover_text("Your draft is already in the message box.")
                    .clicked()
                {
                    app.draft = prompt.to_owned();
                    app.focus_composer = true;
                }
            }
        });
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

fn context_menu(app: &mut JamesApp, ui: &mut egui::Ui, hidden_context: usize) {
    let label = if hidden_context == 0 {
        "Add context".into()
    } else {
        format!("Context ({hidden_context})")
    };
    ui.menu_button(label, |ui| {
        if hidden_context > 0 {
            ui.small("With this question");
            ui.horizontal_wrapped(|ui| context_items(app, ui));
            ui.separator();
        }
        if ui.button("Attach file…").clicked() {
            app.show_attachment = true;
            ui.close_menu();
        }
        if ui.button("Workspace files…").clicked() {
            app.show_workspace = true;
            ui.close_menu();
        }
        if ui.button("Screen context…").clicked() {
            app.page = Page::Screen;
            ui.close_menu();
        }
        if ui.button("R.A.I.N. Lab…").clicked() {
            app.page = Page::Lab;
            ui.close_menu();
        }
    });
}

/// Show what the Send button will include, with a way to detach each item.
fn attached_context(app: &mut JamesApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("attached-context")
        .max_height(72.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.add_enabled_ui(!app.busy(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.small("With this question");
                    context_items(app, ui);
                });
            });
        });
}

fn context_items(app: &mut JamesApp, ui: &mut egui::Ui) {
    if let Some(file) = &app.attachment {
        let short: String = file.name.chars().take(28).collect();
        let title = if short.len() < file.name.len() {
            format!("{short}…")
        } else {
            short
        };
        if ui.small_button(title).on_hover_text(&file.name).clicked() {
            app.show_attachment = true;
            ui.close_menu();
        }
        if ui.small_button("Remove file").clicked() {
            app.attachment = None;
            app.attachment_path.clear();
        }
    }
    if let Some(snapshot) = &app.workspace_context {
        let count = snapshot.files().count();
        if ui.small_button(format!("Workspace ({count})")).clicked() {
            app.show_workspace = true;
            ui.close_menu();
        }
        if ui.small_button("Remove workspace files").clicked() {
            app.workspace_context = None;
        }
    }
    if app.attach_screen {
        let label = if app.screen.as_ref().is_some_and(|s| s.selection.is_some()) {
            "Screen selection"
        } else {
            "Screenshot"
        };
        if ui.small_button(label).clicked() {
            app.page = Page::Screen;
            ui.close_menu();
        }
        if ui.small_button("Detach screenshot").clicked() {
            app.attach_screen = false;
        }
    }
    if app.lab.include {
        if ui.small_button("Lab snapshot").clicked() {
            app.page = Page::Lab;
            ui.close_menu();
        }
        if ui.small_button("Stop sharing Lab").clicked() {
            app.lab.include = false;
        }
    }
}

fn input_actions(app: &mut JamesApp, ui: &mut egui::Ui, ctx: &egui::Context, busy: bool) {
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
            ui.close_menu();
        }
    } else if ui.button("End conversation").clicked() {
        app.stop();
        app.info("Conversation ended.");
        ui.close_menu();
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
            ui.close_menu();
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
            ui.close_menu();
    }
    if ui
        .add_enabled(!busy && can_listen, egui::Button::new("Dictate"))
        .on_hover_text("Speak once; the words go into the message box for you to check.")
        .on_disabled_hover_text(&listen_hint)
        .clicked()
    {
        app.dictate_once();
        ui.close_menu();
    }
}

/// The message box and the buttons under it. Stop is first and prominent
/// whenever something is running.
pub fn composer(app: &mut JamesApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("composer").show(ctx, |ui| {
        ui.add_space(6.0);
        let send_keys = if cfg!(target_os = "macos") { "⌘ Return" } else { "Ctrl+Enter" };
        let narrow = app.uses_compact_menus(ctx);
        let context_count = [app.attachment.is_some(), app.workspace_context.is_some(), app.attach_screen, app.lab.include]
            .into_iter().filter(|included| *included).count();
        if !narrow && context_count > 0 {
            attached_context(app, ui);
        }
        let draft_height = (ctx.screen_rect().height() * 0.23).clamp(38.0, 150.0);
        let compose = egui::ScrollArea::vertical()
            .id_salt("draft-scroll")
            .max_height(draft_height)
            .min_scrolled_height(0.0)
            .auto_shrink([false, true])
            .show(ui, |ui| ui.add(
                egui::TextEdit::multiline(&mut app.draft)
                    .id(egui::Id::new("message-box"))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text(format!("Ask James… ({send_keys} sends)")),
            )).inner;
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
            ui.add_enabled_ui(!busy, |ui| context_menu(app, ui, if narrow { context_count } else { 0 }));
            if narrow {
                ui.menu_button("Tools", |ui| input_actions(app, ui, ctx, busy));
            } else {
                input_actions(app, ui, ctx, busy);
            }
        });
        let color = match app.tone {
            Tone::Info => ui.visuals().text_color(),
            Tone::Warn => ui.visuals().warn_fg_color,
            Tone::Error => ui.visuals().error_fg_color,
        };
        ui.horizontal_wrapped(|ui| {
            if app.worker.is_some() {
                theme::busy(ui, app.prefs.reduce_motion);
            }
            let status = egui::Label::new(RichText::new(&app.status).small().color(color));
            if narrow && app.tone == Tone::Info {
                ui.add(status.truncate()).on_hover_text(&app.status);
            } else {
                ui.add(status.wrap());
            }
            if app.tone == Tone::Error {
                ui.hyperlink_to(RichText::new("Troubleshooting").small(), crate::TROUBLE_URL);
            }
        });
        ui.add_space(6.0);
    });
    if app.show_attachment {
        let mut open = true;
        let mut attach = false;
        egui::Window::new("Attachment")
            .open(&mut open)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.label("Drop a file into James or enter its path below.");
                let path = ui.add(
                    egui::TextEdit::singleline(&mut app.attachment_path)
                        .hint_text("File path")
                        .desired_width(340.0),
                );
                path.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::TextEdit,
                        true,
                        "Attachment file path",
                    )
                });
                let can_attach = !app.busy() && !app.attachment_path.trim().is_empty();
                let enter =
                    path.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                attach = ui
                    .add_enabled(can_attach, egui::Button::new("Attach"))
                    .clicked()
                    || (can_attach && enter);
                ui.small("PDF text, UTF-8 text, PNG, JPEG or WebP");
                if let Some(attachment) = &app.attachment {
                    ui.separator();
                    ui.label(RichText::new(&attachment.name).strong());
                    ui.small("Included with your next chat question.");
                    if let crate::attachments::Content::Text(text) = &attachment.content {
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .show(ui, |ui| {
                                ui.add(egui::Label::new(RichText::new(text).monospace()).wrap());
                            });
                    }
                    if ui
                        .add_enabled(!app.busy(), egui::Button::new("Remove attachment"))
                        .clicked()
                    {
                        app.attachment = None;
                        app.attachment_path.clear();
                    }
                }
            });
        app.show_attachment = open && !attach;
        if attach {
            app.attach_file(std::path::PathBuf::from(app.attachment_path.trim()));
        }
    }
    if app.show_workspace {
        let mut open = true;
        let mut list = false;
        let mut preview = false;
        let mut attach = false;
        egui::Window::new("Workspace files").open(&mut open).resizable(true).vscroll(true).show(ctx, |ui| {
            ui.label("Choose a directory and up to three relative UTF-8 text files (4 KiB each).");
            ui.label("Directory");
            if ui.add_enabled(!app.busy() && app.workspace_pending.is_none(), egui::TextEdit::singleline(&mut app.workspace_directory).desired_width(350.0)).changed() { app.workspace_entries.clear(); }
            if ui.add_enabled(!app.busy() && app.workspace_pending.is_none(), egui::Button::new("List directory")).clicked() { list = true; }
            egui::ScrollArea::vertical().id_salt("workspace_entries").max_height(100.0).show(ui, |ui| {
                for entry in &app.workspace_entries {
                    if entry.is_directory { ui.small(format!("{}/ (enter a relative file path below)", entry.name)); }
                    else if ui.small_button(&entry.name).clicked() {
                        if !app.workspace_paths.is_empty() { app.workspace_paths.push('\n'); }
                        app.workspace_paths.push_str(&entry.name);
                    }
                }
            });
            ui.label("Relative file paths, one per line");
            ui.add_enabled(!app.busy() && app.workspace_pending.is_none(), egui::TextEdit::multiline(&mut app.workspace_paths).desired_rows(3).desired_width(350.0));
            if ui.add_enabled(!app.busy() && app.workspace_pending.is_none(), egui::Button::new("Preview selected files")).clicked() { preview = true; }
            if let Some(snapshot) = &app.workspace_pending {
                ui.separator();
                ui.label(format!("Selected directory: {}", snapshot.root().display()));
                egui::ScrollArea::vertical().id_salt("workspace_preview").max_height(240.0).show(ui, |ui| {
                    for (path, content) in snapshot.files() {
                        ui.label(RichText::new(path).strong());
                        ui.add(egui::Label::new(RichText::new(content).monospace()).wrap());
                    }
                });
                ui.small("These exact contents will be sent to the model service with your next chat question. The service may relay them elsewhere.");
                if ui.button("Attach selected files").clicked() { attach = true; }
                if ui.button("Discard workspace preview").clicked() { app.workspace_pending = None; }
            }
            if let Some(snapshot) = &app.workspace_context {
                ui.separator();
                ui.label(format!("Currently attached from: {}", snapshot.root().display()));
                egui::ScrollArea::vertical().id_salt("workspace_attached").max_height(150.0).show(ui, |ui| {
                    for (path, content) in snapshot.files() {
                        ui.label(RichText::new(path).strong());
                        ui.add(egui::Label::new(RichText::new(content).monospace()).wrap());
                    }
                });
                if ui.button("Remove attached workspace files").clicked() { app.workspace_context = None; }
            }
        });
        app.show_workspace = open && !attach;
        if attach {
            app.workspace_context = app.workspace_pending.take();
            app.info("Workspace files attached to the next chat question.");
        }
        if list {
            app.list_workspace();
        }
        if preview {
            app.preview_workspace();
        }
    }
}
