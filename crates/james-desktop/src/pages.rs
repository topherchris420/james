//! Screen context, Computer control and Settings.
use crate::{
    JamesApp, Page, Tone, chat, copyable, guide, numeric_port, review,
    session::{SetupStep, ThemeChoice},
    theme,
};
use eframe::egui::{self, RichText};
use james_models::{LocalProvider, Support};
use james_platform::{Action, Key};

/// Draws a ring where an approved click will land.
fn mark_click(ui: &egui::Ui, rect: egui::Rect, action: &Action, width: u32, height: u32) {
    if let Action::Click { x, y } = action {
        let pos = rect.min
            + egui::vec2(
                *x as f32 / width as f32 * rect.width(),
                *y as f32 / height as f32 * rect.height(),
            );
        let color = theme::accent(ui);
        ui.painter()
            .circle_stroke(pos, 12.0, egui::Stroke::new(3.0_f32, color));
        ui.painter()
            .circle_stroke(pos, 3.0, egui::Stroke::new(2.0_f32, color));
    }
}

/// Draws Guide me's numbered marks where its steps are on a screenshot.
pub fn mark_steps(ui: &egui::Ui, rect: egui::Rect, marks: &[guide::Mark], size: (u32, u32)) {
    let fill = theme::accent(ui);
    let ink = ui.visuals().panel_fill;
    for mark in marks {
        let pos = rect.min
            + egui::vec2(
                mark.x as f32 / size.0 as f32 * rect.width(),
                mark.y as f32 / size.1 as f32 * rect.height(),
            );
        ui.painter()
            .circle(pos, 11.0, fill, egui::Stroke::new(2.0_f32, ink));
        ui.painter().text(
            pos,
            egui::Align2::CENTER_CENTER,
            mark.step.to_string(),
            egui::FontId::proportional(13.0),
            ink,
        );
    }
}

pub fn screen(app: &mut JamesApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.heading("Screen context");
    ui.label("Capture your main screen when you choose. James hides for 3 seconds so you can bring the right app to the front, then shows you the picture. Nothing is captured in the background.");
    if ui
        .add_enabled(!app.busy(), theme::primary(ui, "Capture primary screen"))
        .clicked()
    {
        app.capture(ctx);
    }
    let mut picked = None;
    let mut discard = false;
    if let Some(screen) = &app.screen {
        let age = screen.at.elapsed().as_secs();
        ui.label(format!(
            "{} × {} pixels · captured {age} seconds ago",
            screen.capture.width, screen.capture.height
        ));
        if let Some(target) = &screen.capture.target {
            ui.small(format!("In front when captured: {}", target.describe()));
        }
        if age > 60 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "This screenshot is more than a minute old. Capture again before asking about the screen or reviewing an action.",
            );
        }
        let sees = app.vision();
        ui.add_enabled(
            sees != Support::No,
            egui::Checkbox::new(
                &mut app.attach_screen,
                "Attach this screenshot to my next question",
            ),
        )
        .on_disabled_hover_text("The current model can't read images.");
        if sees == Support::Unknown {
            ui.small("Your model app didn't say whether this model reads images; if it can't, the question will fail with an explanation.");
        }
        if james_platform::can_control_input() {
            ui.small("Click the picture to choose a point for a click. Nothing is clicked yet: you review it first.");
        }
        let response = ui
            .add(
                egui::Image::new(&screen.texture)
                    .max_size(egui::vec2(ui.available_width(), 440.0))
                    .maintain_aspect_ratio(true)
                    .sense(egui::Sense::click()),
            )
            .on_hover_text("Your screenshot, kept in memory only");
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                "Screenshot of your primary screen",
            )
        });
        if response.clicked()
            && james_platform::can_control_input()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let x = ((pos.x - response.rect.min.x) / response.rect.width()
                * screen.capture.width as f32) as u32;
            let y = ((pos.y - response.rect.min.y) / response.rect.height()
                * screen.capture.height as f32) as u32;
            picked = Some((
                x.min(screen.capture.width - 1),
                y.min(screen.capture.height - 1),
            ));
        }
        if let Some(p) = &app.pending {
            mark_click(ui, response.rect, &p.proposal.action, p.width, p.height);
        }
        let guide_marks = app.guide_marks.as_ref();
        if let Some(shown) = guide_marks.filter(|g| g.screenshot == screen.at) {
            mark_steps(
                ui,
                response.rect,
                &shown.marks,
                (screen.capture.width, screen.capture.height),
            );
            ui.small("The numbers mark the steps in James's last Guide me answer. Your model placed them, and they may be off.");
        }
        discard = ui.button("Discard screenshot").clicked();
    } else {
        ui.add_space(8.0);
        ui.label(RichText::new("No screenshot yet.").strong());
        ui.label("Use one to ask a vision model about what's on screen, or press Guide me under the message box to capture and ask in one step.");
    }
    if discard {
        app.screen = None;
        app.pending = None;
        app.attach_screen = false;
    }
    if let Some((x, y)) = picked {
        app.action_x = x;
        app.action_y = y;
        app.propose(
            Action::Click { x, y },
            "Point selected by you in the screenshot preview.".into(),
            false,
        );
    }
    ui.add_space(6.0);
    ui.small("Screenshots stay in memory and are never saved by James. A model's answer about a screenshot is saved as text with your conversation.");
}

fn recovery_ui(app: &mut JamesApp, ui: &mut egui::Ui) {
    let warn = ui.visuals().warn_fg_color;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(
            RichText::new("Check the last approved action before doing anything else")
                .strong()
                .color(warn),
        );
        match &app.recovery {
            review::Recovery::Pending(record) => {
                ui.label(format!("James was asked to: {}.", review::describe(&record.action)));
                if let Some(target) = &record.target {
                    ui.label(format!("In: {}", target.describe()));
                }
                if record.recorded_at > 0 {
                    let ago = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|now| now.as_secs().saturating_sub(record.recorded_at))
                        .unwrap_or_default();
                    ui.small(format!(
                        "Approved {} ago.",
                        if ago < 120 {
                            format!("{ago} seconds")
                        } else if ago < 7200 {
                            format!("{} minutes", ago / 60)
                        } else {
                            format!("{} hours", ago / 3600)
                        }
                    ));
                }
                ui.label(record.outcome_text());
            }
            review::Recovery::Unreadable(why) => {
                ui.label(format!("{why}. Think about the last action you approved in James."));
            }
            review::Recovery::Clear => {}
        }
        ui.label("Look at that app now. James never repeats an action by itself. When you've checked what happened (and fixed anything if needed), confirm below.");
        if ui
            .add(theme::primary(ui, "I checked the app — continue"))
            .clicked()
        {
            match review::acknowledge(&app.intent_path()) {
                Ok(()) => {
                    app.recovery = review::read_intent(&app.intent_path());
                    app.info("Thanks. You can review a new action now.");
                }
                Err(e) => app.fail(e),
            }
        }
    });
    ui.add_space(8.0);
}

pub fn actions(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.heading("Computer control");
    // A record must always be reviewable, whatever this platform can do now.
    if app.recovery.needs_check() {
        recovery_ui(app, ui);
    }
    if !james_platform::can_control_input() {
        ui.label("On this computer James never clicks or types. Use Guide me (under the message box) for step-by-step help with what's on screen; you stay in control of every action.");
        ui.small("Reviewed clicks and keystrokes are available on Windows only. macOS and Linux need their own, separately tested safety design, which doesn't exist yet.");
        return;
    }
    ui.label("James can perform one click, one key press or one piece of typing at a time, only after you approve exactly that action. Approve only while the right app is ready: a click or Enter can submit, send or delete things, and sent input can't be undone.");
    let enabled = !app.busy() && !app.recovery.needs_check();
    ui.add_space(4.0);
    ui.label(RichText::new("Prepare an action yourself").strong());
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            let x = ui.label("Click at x");
            ui.add(egui::DragValue::new(&mut app.action_x)).labelled_by(x.id);
            let y = ui.label("y");
            ui.add(egui::DragValue::new(&mut app.action_y)).labelled_by(y.id);
            if ui.button("Review click").clicked() {
                app.propose(
                    Action::Click {
                        x: app.action_x,
                        y: app.action_y,
                    },
                    "Manual click requested by you.".into(),
                    false,
                );
            }
        });
        ui.horizontal(|ui| {
            let label = ui.label("Press key");
            egui::ComboBox::from_id_salt("action-key")
                .selected_text(review::key_name(&app.action_key))
                .show_ui(ui, |ui| {
                    for key in [
                        Key::Enter,
                        Key::Tab,
                        Key::Escape,
                        Key::Backspace,
                        Key::Delete,
                        Key::Up,
                        Key::Down,
                        Key::Left,
                        Key::Right,
                        Key::Home,
                        Key::End,
                        Key::Space,
                    ] {
                        let name = review::key_name(&key);
                        ui.selectable_value(&mut app.action_key, key, name);
                    }
                })
                .response
                .labelled_by(label.id);
            if ui.button("Review key").clicked() {
                app.propose(
                    Action::Key {
                        key: app.action_key.clone(),
                    },
                    "Manual key requested by you.".into(),
                    false,
                );
            }
        });
        ui.horizontal(|ui| {
            let label = ui.label("Type text");
            ui.add(egui::TextEdit::singleline(&mut app.action_text).desired_width(320.0))
                .labelled_by(label.id);
            if ui.button("Review typing").clicked() {
                app.propose(
                    Action::TypeText {
                        text: app.action_text.clone(),
                    },
                    "Manual text insertion requested by you.".into(),
                    false,
                );
            }
        });
        ui.small("Or capture your screen and press Suggest one action under the message box to let a vision model propose one.");
    });
    ui.separator();
    let Some(pending) = &app.pending else {
        if app.scheduled.is_none() {
            ui.label("No action is waiting for your approval.");
        }
        return;
    };
    ui.label(
        RichText::new("Waiting for your approval")
            .size(18.0)
            .strong(),
    );
    ui.label(
        RichText::new(review::describe(&pending.proposal.action))
            .size(17.0)
            .strong()
            .color(theme::accent(ui)),
    );
    match &pending.target {
        Some(target) => ui.label(format!(
            "In: {}. James sends nothing unless this window is in front when the countdown ends.",
            target.describe()
        )),
        None => ui.colored_label(
            ui.visuals().warn_fg_color,
            "James couldn't tell which window was in front when the screenshot was taken, so it can't check the target before sending. Make sure the right app is in front.",
        ),
    };
    ui.label(format!("Why: {}", pending.proposal.rationale));
    if pending.from_model {
        ui.small(
            "This reason was written by the model. It is not a guarantee; judge the action itself.",
        );
    }
    if let Some(screen) = &app.screen {
        let response = ui.add(
            egui::Image::new(&screen.texture)
                .max_size(egui::vec2(ui.available_width(), 260.0))
                .maintain_aspect_ratio(true),
        );
        mark_click(
            ui,
            response.rect,
            &pending.proposal.action,
            pending.width,
            pending.height,
        );
    }
    let fresh = pending.captured_at.elapsed() <= crate::SCREEN_MAX_AGE;
    if !fresh {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "This review has expired because the screenshot is over a minute old. Capture the screen again.",
        );
    }
    ui.small("Allow once starts a 5-second countdown so you can switch to the app. Esc or Cancel stops it. There is no system-wide stop key.");
    // Defense in depth: this page already returns early without input support.
    let can_input = james_platform::can_control_input();
    let mut allow = false;
    let mut deny = false;
    ui.horizontal(|ui| {
        allow = ui
            .add_enabled(
                enabled && fresh && can_input,
                theme::primary(ui, "Allow once"),
            )
            .on_hover_text("Approve this one action. It runs after a 5-second countdown.")
            .clicked();
        deny = ui.button("Deny").clicked();
    });
    if allow {
        app.allow_once();
    } else if deny {
        app.pending = None;
        app.info("Action denied. Nothing was sent.");
    }
}

pub fn settings(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.heading("Settings");
    model_settings(app, ui);
    ui.separator();
    appearance(app, ui);
    ui.separator();
    voice_settings(app, ui);
    ui.separator();
    guide_settings(app, ui);
    ui.separator();
    privacy(app, ui);
    ui.separator();
    help(app, ui);
}

fn model_settings(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Local model").size(18.0).strong());
    ui.label("Choose the model app running on this computer. James connects only to 127.0.0.1 (this computer) on the port below.");
    let previous_provider = app.provider;
    ui.horizontal(|ui| {
        let label = ui.label("Model app");
        egui::ComboBox::from_id_salt("provider")
            .selected_text(app.provider.label())
            .show_ui(ui, |ui| {
                for provider in LocalProvider::ALL {
                    ui.selectable_value(&mut app.provider, provider, provider.label());
                }
            })
            .response
            .labelled_by(label.id);
    });
    if app.provider != previous_provider {
        app.port = app.provider.default_port().to_string();
        app.model.clear();
        app.available_models.clear();
        app.remote_models.clear();
        app.info(format!(
            "Selected {}. Start its server, then press Discover models.",
            app.provider.label()
        ));
        app.save_session();
        app.model_changed();
    }
    ui.horizontal(|ui| {
        let label = ui.label("Port on this computer");
        ui.add(egui::TextEdit::singleline(&mut app.port).desired_width(80.0))
            .labelled_by(label.id);
        if numeric_port(&app.port).is_err() {
            ui.colored_label(
                ui.visuals().error_fg_color,
                "Enter a number from 1 to 65535",
            );
        }
    });
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!app.busy(), egui::Button::new("Discover models"))
            .on_hover_text("List the models offered by the app and port above.")
            .clicked()
        {
            app.discover();
        }
        if ui
            .add_enabled(!app.busy(), egui::Button::new("Find my model server"))
            .on_hover_text("Check the usual ports of LM Studio, Ollama and llama.cpp on this computer and use the first server with a model.")
            .clicked()
        {
            app.find_server();
        }
    });
    let previous_model = app.model.clone();
    ui.horizontal(|ui| {
        let label = ui.label("Model");
        let response = egui::ComboBox::from_id_salt("models")
            .selected_text(if app.model.trim().is_empty() {
                "Not selected"
            } else {
                app.model.trim()
            })
            .show_ui(ui, |ui| {
                for model in app.available_models.clone() {
                    let text = if app.remote_models.contains(&model) {
                        format!("{model} (runs elsewhere)")
                    } else {
                        model.clone()
                    };
                    ui.selectable_value(&mut app.model, model, text);
                }
            })
            .response;
        response.labelled_by(label.id);
    });
    if app.available_models.is_empty() {
        ui.horizontal(|ui| {
            let label = ui.label("Or type a model ID");
            ui.add(egui::TextEdit::singleline(&mut app.model).desired_width(280.0))
                .labelled_by(label.id);
        });
    }
    if app.model != previous_model && app.available_models.contains(&app.model) {
        app.save_session();
        app.model_changed();
    }
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            app.save_session();
            app.model_changed();
            app.info("Settings saved.");
        }
        if ui
            .add_enabled(
                !app.model.trim().is_empty(),
                egui::Button::new("Check what this model can do"),
            )
            .clicked()
        {
            app.model_info = None;
            app.probe_model();
        }
    });
    chat::model_facts(app, ui);
    ui.small("No cloud connection, account or API key is ever used by James itself. A model app may relay to a cloud service if you set it up that way; James shows it when the app says so.");
}

fn appearance(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Appearance and accessibility")
            .size(18.0)
            .strong(),
    );
    let before = app.prefs.clone();
    ui.horizontal(|ui| {
        ui.label("Theme");
        ui.selectable_value(
            &mut app.prefs.theme,
            ThemeChoice::System,
            "Match my computer",
        );
        ui.selectable_value(&mut app.prefs.theme, ThemeChoice::Light, "Light");
        ui.selectable_value(&mut app.prefs.theme, ThemeChoice::Dark, "Dark");
    });
    ui.checkbox(&mut app.prefs.high_contrast, "High contrast")
        .on_hover_text("Black and white with strong outlines and a bright accent");
    let mut scale = app.prefs.text_scale();
    ui.horizontal(|ui| {
        let label = ui.label("Text and controls size");
        ui.add(
            egui::Slider::new(&mut scale, 0.8..=2.0)
                .step_by(0.1)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .labelled_by(label.id);
    });
    app.prefs.text_scale = scale;
    ui.checkbox(&mut app.prefs.reduce_motion, "Reduce motion")
        .on_hover_text("No animations, spinners or smooth scrolling");
    ui.small("Ctrl+plus and Ctrl+minus (⌘ on a Mac) also change the size. James describes its buttons, fields and messages to screen readers through AccessKit, but this hasn't been tested with a real screen reader yet; please report anything that doesn't work.");
    if app.prefs != before {
        app.save_session();
    }
}

fn voice_settings(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Voice").size(18.0).strong());
    if ui
        .checkbox(&mut app.prefs.speak_replies, "Read new answers aloud")
        .changed()
    {
        app.save_session();
    }
    ui.label("Talk starts a hands-free conversation: James listens, answers aloud, then listens again. It pauses after 15 seconds of silence; say \"stop\" or press Esc to end. Dictate puts one utterance into the message box for you to check.");
    ui.label("Voice can ask and listen but never approves or performs clicks and typing.");
    ui.small(james_platform::voice_support());
    ui.add_space(4.0);
    ui.strong("Listening");
    if app.listening.is_ready() {
        ui.colored_label(
            theme::accent(ui),
            format!("Ready · {}", app.listening.describe()),
        );
    } else {
        ui.colored_label(ui.visuals().warn_fg_color, app.listening.describe());
    }
    if james_platform::uses_whisper() {
        whisper_settings(app, ui);
    }
}

fn guide_settings(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Guide me").size(18.0).strong());
    if ui
        .checkbox(
            &mut app.prefs.guide_marks,
            "Number the steps on my screenshot",
        )
        .changed()
    {
        app.save_session();
    }
    ui.label("Your model decides where each number goes, and smaller models often put them in the wrong place. The written steps are the same either way: find the label each step names before you click.");
}

/// whisper.cpp setup for macOS and Linux: optional explicit paths, a check
/// button, and copyable setup steps. Nothing is downloaded by James itself.
fn whisper_settings(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label("Listening runs whisper.cpp on this computer. Each utterance is recorded only until you pause, transcribed, and its temporary audio file deleted immediately.");
    let label = ui.label("whisper.cpp program (leave blank to find whisper-cli automatically)");
    ui.add(
        egui::TextEdit::singleline(&mut app.whisper_program)
            .hint_text("/opt/homebrew/bin/whisper-cli"),
    )
    .labelled_by(label.id);
    let label = ui
        .label("Speech model file (leave blank to use the best model in the James models folder)");
    ui.add(egui::TextEdit::singleline(&mut app.whisper_model).hint_text("…/ggml-base.en.bin"))
        .labelled_by(label.id);
    if ui.button("Check listening setup").clicked() {
        app.refresh_listening();
        app.save_session();
    }
    egui::CollapsingHeader::new("Set up listening")
        .default_open(!app.listening.is_ready())
        .show(ui, |ui| {
            let install = if cfg!(target_os = "macos") {
                "brew install whisper-cpp sox"
            } else {
                "git clone https://github.com/ggml-org/whisper.cpp ~/whisper.cpp && cmake -S ~/whisper.cpp -B ~/whisper.cpp/build && cmake --build ~/whisper.cpp/build -j --config Release"
            };
            ui.label("1. Install whisper.cpp and a microphone recorder:");
            copyable(ui, install);
            if !cfg!(target_os = "macos") {
                ui.small("Most Linux desktops already have a recorder (parec or arecord). Otherwise install pulseaudio-utils, alsa-utils, sox or ffmpeg.");
            }
            if let Some(dir) = james_platform::whisper_models_dir() {
                ui.label("2. Download a speech model (about 150 MB; base.en is a good start for English):");
                copyable(
                    ui,
                    &format!(
                        "curl -L --create-dirs -o \"{}/ggml-base.en.bin\" https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
                        dir.display()
                    ),
                );
                ui.small("For other languages use ggml-base.bin or ggml-small.bin instead.");
            }
            ui.label("3. Choose Check listening setup above.");
            if cfg!(target_os = "macos") {
                ui.small("macOS asks once for microphone access the first time James listens.");
            }
        });
}

fn privacy(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Privacy and your data").size(18.0).strong());
    if ui
        .checkbox(
            &mut app.keep_history,
            "Keep my conversations on this computer",
        )
        .on_hover_text("When off, conversations exist only while James is open.")
        .changed()
        && app.save_session()
    {
        app.info(if app.keep_history {
            "Conversations will be saved on this computer."
        } else {
            "Conversations are no longer saved. The saved copy was removed; this conversation stays until you close James."
        });
    }
    egui::Grid::new("what-is-kept")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            for (what, how) in [
                ("Conversation and settings", if app.keep_history {
                    "Saved in session.json, readable only by your account. Not encrypted."
                } else {
                    "Settings saved in session.json; conversations not saved."
                }),
                ("Last approved action", "action-intent.json, including any text to type, until you confirm you checked it."),
                ("Screenshots", "Memory only. Never saved by James."),
                ("Microphone audio", "Deleted right after it becomes text. Never leaves this computer."),
                ("Crash reports", "crash.log, no conversation text, never sent anywhere."),
            ] {
                ui.label(RichText::new(what).strong());
                ui.label(how);
                ui.end_row();
            }
        });
    ui.small(format!(
        "Folder: {}",
        app.session
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    ));
    ui.small("Your model app keeps its own logs and history under its own settings. Full-disk encryption (BitLocker, FileVault or LUKS) protects these files if your computer is lost.");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !app.busy() && !app.messages.is_empty(),
                egui::Button::new("Clear conversation…"),
            )
            .clicked()
        {
            app.confirm_clear = true;
        }
        if ui
            .add_enabled(!app.busy(), theme::danger(ui, "Delete all James data…"))
            .clicked()
        {
            app.confirm_delete = true;
        }
    });
}

fn help(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Help, updates and about").size(18.0).strong());
    ui.horizontal_wrapped(|ui| {
        if ui.button("Run guided setup again").clicked() {
            app.setup.step = SetupStep::Welcome;
            app.setup_note = None;
            app.setup_reply = None;
            app.page = Page::Chat;
            app.save_session();
        }
        if ui.button("Keyboard shortcuts").clicked() {
            app.show_shortcuts = true;
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.hyperlink_to("Getting started and troubleshooting", crate::HELP_URL);
        ui.hyperlink_to("Report a problem", crate::ISSUES_URL);
    });
    ui.label(format!("You're using James {}.", env!("CARGO_PKG_VERSION")));
    ui.horizontal_wrapped(|ui| {
        ui.label("James never checks for updates by itself, because it never connects to the internet.");
        ui.hyperlink_to("See the newest release", crate::RELEASES_URL)
            .on_hover_text("Opens the releases page in your web browser. Each release lists SHA-256 checksums so you can check your download.");
    });
    ui.small("James is free software under the MIT license, built by Vers3Dynamics with egui and other open-source libraries; their licenses come with James in THIRD-PARTY-NOTICES.txt.");
    if app.tone == Tone::Error {
        ui.hyperlink_to("Troubleshooting for the last error", crate::TROUBLE_URL);
    }
}
