//! First-run setup: five short steps that end with a real reply from a model
//! on this computer. Progress is saved after every step, so closing James
//! halfway resumes at the same step. James never installs software,
//! downloads models, changes the model app's settings or grants permissions;
//! each step says what the person does and checks that it worked.
use crate::{Ask, JamesApp, Page, chat, copyable, markdown, session::SetupStep, theme};
use eframe::egui::{self, RichText};
use james_models::{LocalProvider, Support};

const LM_STUDIO: &str = "https://lmstudio.ai/";
const OLLAMA: &str = "https://ollama.com/download";

/// Which operating system the instructions are written for.
fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Mac"
    } else {
        "Linux"
    }
}

fn go(app: &mut JamesApp, step: SetupStep) {
    app.setup.step = step;
    app.save_session();
}

fn progress(ui: &mut egui::Ui, step: SetupStep) {
    let accent = theme::accent(ui);
    ui.horizontal_wrapped(|ui| {
        for (index, shown) in SetupStep::SHOWN.iter().enumerate() {
            let text = format!("{} {}", index + 1, shown.title());
            if *shown == step {
                ui.label(RichText::new(text).strong().color(accent));
            } else if *shown < step {
                ui.label(RichText::new(text).weak());
            } else {
                ui.label(RichText::new(text).weak().small());
            }
            if index + 1 < SetupStep::SHOWN.len() {
                ui.label(RichText::new("›").weak());
            }
        }
    });
    let position = SetupStep::SHOWN
        .iter()
        .position(|s| *s == step)
        .unwrap_or(0)
        + 1;
    ui.small(format!("Step {position} of {}", SetupStep::SHOWN.len()));
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_max_width(720.0);
            add(ui);
        });
}

pub fn ui(app: &mut JamesApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(12.0);
    progress(ui, app.setup.step);
    ui.add_space(10.0);
    match app.setup.step {
        SetupStep::Welcome | SetupStep::Done => welcome(app, ui),
        SetupStep::ModelApp => model_app(app, ui),
        SetupStep::Connect => connect(app, ui),
        SetupStep::Test => test(app, ui),
        SetupStep::Screen => screen(app, ui, ctx),
    }
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.small("Stuck?");
        ui.hyperlink_to(
            RichText::new("Read the getting started guide").small(),
            crate::HELP_URL,
        );
        if ui
            .small_button("Skip setup and use Settings")
            .on_hover_text("Leave this guide; you can run it again from Settings.")
            .clicked()
        {
            go(app, SetupStep::Done);
            app.page = Page::Settings;
        }
    });
}

fn welcome(app: &mut JamesApp, ui: &mut egui::Ui) {
    card(ui, |ui| {
        ui.label(RichText::new("Welcome to James").size(26.0).strong());
        ui.label("James is a desktop companion that talks to an AI model running on your own computer. Setup takes a few minutes, plus however long your model takes to download.");
        ui.add_space(8.0);
        for (title, body) in [
            (
                "It talks only to this computer.",
                "James connects to a model app on this computer and nowhere else. It has no internet connection of its own and no account.",
            ),
            (
                "It sees your screen only when you ask.",
                "Nothing is captured in the background. You see each screenshot first.",
            ),
            (
                "Nothing happens without you.",
                "On Windows, James can suggest one click or keystroke, but only you can approve it, once, after seeing exactly what it will do.",
            ),
        ] {
            ui.label(RichText::new(title).strong().color(theme::accent(ui)));
            ui.label(body);
            ui.add_space(4.0);
        }
        ui.small(
            "James is an early preview. Answers come from the model you choose and can be wrong.",
        );
        ui.add_space(8.0);
        if ui.add(theme::primary(ui, "Get started")).clicked() {
            go(app, SetupStep::ModelApp);
        }
    });
}

fn model_app(app: &mut JamesApp, ui: &mut egui::Ui) {
    card(ui, |ui| {
        ui.label(RichText::new("Get a model app").size(22.0).strong());
        ui.label("James doesn't include an AI model. A free model app runs the model on this computer, and James talks to it. Pick one:");
        ui.add_space(8.0);
        ui.label(
            RichText::new("LM Studio — easiest, no terminal needed")
                .strong()
                .color(theme::accent(ui)),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("1. Download it from");
            ui.hyperlink_to("lmstudio.ai", LM_STUDIO);
            ui.label(match os_name() {
                "Windows" => "and run the installer.",
                "Mac" => "and drag it to Applications.",
                _ => "(an AppImage: allow it to run as a program, then open it).",
            });
        });
        ui.label("2. Search for Gemma 3 4B, download it and load it. It is about 3 GB and can also read images, which screen questions need.");
        ui.label("3. Open the Developer tab and switch the server on, so its status says Running.");
        ui.add_space(8.0);
        ui.label(
            RichText::new("Ollama — runs quietly in the background")
                .strong()
                .color(theme::accent(ui)),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("1. Download it from");
            ui.hyperlink_to("ollama.com", OLLAMA);
            ui.label(match os_name() {
                "Windows" => "and run the installer. Ollama starts in the system tray.",
                "Mac" => "and move it to Applications. Ollama starts in the menu bar.",
                _ => "and follow the Linux instructions there.",
            });
        });
        ui.label(if os_name() == "Linux" {
            "2. Get a model by running this in a terminal:"
        } else {
            "2. Get a model: in the Ollama app, choose gemma3:4b in the model menu and send it a message to download it, or run this in a terminal:"
        });
        copyable(ui, "ollama pull gemma3:4b");
        ui.add_space(8.0);
        ui.small("Already use llama.cpp's llama-server or another OpenAI-compatible server? Start it, then continue. About 8 GB of memory is enough for a 4B model. James never downloads or installs anything for you.");
        ui.small("Some model apps can also forward requests to cloud services. James shows you when a model app says a model runs elsewhere, and never picks such a model for you.");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                go(app, SetupStep::Welcome);
            }
            if ui
                .add(theme::primary(ui, "My model app is running"))
                .clicked()
            {
                go(app, SetupStep::Connect);
                app.find_server();
            }
        });
    });
}

fn connect(app: &mut JamesApp, ui: &mut egui::Ui) {
    card(ui, |ui| {
        ui.label(RichText::new("Connect to your model").size(22.0).strong());
        ui.label("James asks the usual places LM Studio (port 1234), Ollama (11434) and llama.cpp (8080) listen, on this computer only, and only when you press the button.");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!app.busy(), theme::primary(ui, "Find my model server"))
                .clicked()
            {
                app.find_server();
            }
            if app.busy() {
                theme::busy(ui, app.prefs.reduce_motion);
                ui.small("Looking…");
            }
        });
        if let Some(note) = &app.setup_note {
            ui.add_space(6.0);
            let found = !app.model.trim().is_empty();
            ui.colored_label(
                if found {
                    ui.visuals().text_color()
                } else {
                    ui.visuals().warn_fg_color
                },
                note,
            );
        }
        if !app.model.trim().is_empty() {
            ui.add_space(6.0);
            let previous = app.model.clone();
            ui.horizontal(|ui| {
                let label = ui.label("Model:");
                egui::ComboBox::from_id_salt("setup-model")
                    .selected_text(app.model.trim())
                    .show_ui(ui, |ui| {
                        for model in app.available_models.clone() {
                            let remote = app.remote_models.contains(&model);
                            let text = if remote {
                                format!("{model} (runs elsewhere)")
                            } else {
                                model.clone()
                            };
                            ui.selectable_value(&mut app.model, model, text);
                        }
                    })
                    .response
                    .labelled_by(label.id);
            });
            if app.model != previous {
                // The note described what Find chose; the person has chosen now.
                app.setup_note = None;
                app.save_session();
                app.model_changed();
            }
            chat::model_facts(app, ui);
        } else if app.setup_note.is_some() {
            ui.add_space(6.0);
            ui.label("What to check:");
            for provider in [LocalProvider::LmStudio, LocalProvider::Ollama] {
                ui.small(format!(
                    "{}: {}",
                    provider.label(),
                    crate::start_hint(provider)
                ));
            }
            ui.small("Using a different port? Choose Skip setup and use Settings, pick your provider and enter the port.");
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                go(app, SetupStep::ModelApp);
            }
            let chat_ok = app.info_now().is_none_or(|i| i.chat != Support::No);
            if ui
                .add_enabled(
                    !app.model.trim().is_empty() && chat_ok && !app.busy(),
                    theme::primary(ui, "Next: say hello"),
                )
                .clicked()
            {
                app.setup_reply = None;
                go(app, SetupStep::Test);
            }
        });
    });
}

fn test(app: &mut JamesApp, ui: &mut egui::Ui) {
    if app.model.trim().is_empty() {
        // Interrupted after the model was removed: connect again first.
        go(app, SetupStep::Connect);
        return;
    }
    card(ui, |ui| {
        ui.label(RichText::new("Say hello").size(22.0).strong());
        ui.label(format!(
            "James will send one short test message to {} through {}. The reply isn't kept in your conversation.",
            app.model.trim(),
            app.provider.label()
        ));
        ui.small(
            "The first reply can take a minute while the model app loads the model into memory.",
        );
        ui.add_space(6.0);
        let testing = app.asking == Some(Ask::Test);
        ui.horizontal(|ui| {
            if testing {
                if ui.add(theme::danger(ui, "Stop")).clicked() {
                    app.stop();
                }
                theme::busy(ui, app.prefs.reduce_motion);
                ui.small(if app.thinking {
                    "The model is thinking…"
                } else {
                    "Waiting for the reply…"
                });
            } else if ui
                .add_enabled(
                    !app.busy(),
                    theme::primary(
                        ui,
                        if app.setup_reply.is_some() {
                            "Send it again"
                        } else {
                            "Send a test message"
                        },
                    ),
                )
                .clicked()
            {
                app.setup_reply = None;
                app.setup_sent_at = Some(std::time::Instant::now());
                app.send(Ask::Test);
            }
        });
        if testing && let Some(partial) = &app.streaming {
            egui::Frame::group(ui.style()).show(ui, |ui| markdown::show(ui, partial, "setup-test"));
        }
        match &app.setup_reply {
            Some(Ok(reply)) => {
                egui::Frame::group(ui.style())
                    .show(ui, |ui| markdown::show(ui, reply, "setup-reply"));
                let seconds = app.setup_answered_in.unwrap_or_default();
                ui.colored_label(
                    theme::accent(ui),
                    format!(
                        "It works. {} answered in {seconds:.0} seconds.",
                        app.model.trim()
                    ),
                );
            }
            Some(Err(error)) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
                ui.hyperlink_to("Troubleshooting help", crate::TROUBLE_URL);
            }
            None => {}
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!testing, egui::Button::new("Back")).clicked() {
                go(app, SetupStep::Connect);
            }
            if app.setup.test_passed {
                if ui
                    .add_enabled(!testing, theme::primary(ui, "Next: screen and safety"))
                    .clicked()
                {
                    go(app, SetupStep::Screen);
                }
            } else if ui
                .add_enabled(!testing, egui::Button::new("Skip the test"))
                .on_hover_text("Continue without a confirmed reply. Questions may fail until the model answers.")
                .clicked()
            {
                go(app, SetupStep::Screen);
            }
        });
    });
}

fn screen(app: &mut JamesApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    card(ui, |ui| {
        ui.label(
            RichText::new("Your screen, and staying in control")
                .size(22.0)
                .strong(),
        );
        ui.label("Guide me and screen questions send one picture of your main screen to your model, only when you press the button. James hides for three seconds so you can show the right window, then shows you the picture.");
        match app.vision() {
            Support::Yes => ui.label(format!("{} can read images, so screen questions will work.", app.model.trim())),
            Support::No => ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("{} can't read images. For screen questions, choose a vision model such as Gemma 3 later in Settings. Chat works as it is.", app.model.trim()),
            ),
            Support::Unknown => ui.label("Your model app didn't say whether this model can read images. If screen questions fail, choose a vision model such as Gemma 3."),
        };
        ui.add_space(4.0);
        ui.label(
            RichText::new("Permission")
                .strong()
                .color(theme::accent(ui)),
        );
        ui.label(match os_name() {
            "Mac" => "The first capture asks for Screen Recording permission. Allow James in System Settings > Privacy & Security > Screen & System Audio Recording, then quit and reopen James. James can't grant this for you.",
            "Windows" => "Windows doesn't ask for permission to take a screenshot.",
            _ => "Linux needs a screenshot tool: gnome-screenshot, spectacle, grim (Wayland) or ImageMagick (X11). Your desktop may ask you to allow it.",
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!app.busy() && app.screen.is_none(), egui::Button::new("Try a screenshot (optional)"))
                .on_hover_text("James minimizes for three seconds, captures your main screen and shows it here. Nothing is sent to the model.")
                .clicked()
            {
                app.setup_capture = true;
                app.capture(ctx);
            }
            if app.screen.is_some() && ui.button("Discard it").clicked() {
                app.screen = None;
            }
        });
        if let Some(screen) = &app.screen {
            ui.add(
                egui::Image::new(&screen.texture)
                    .max_size(egui::vec2(360.0, 220.0))
                    .maintain_aspect_ratio(true),
            )
            .on_hover_text("Your screenshot. It stays in memory and was not sent anywhere.");
            ui.small("This picture is in memory only. It goes to your model only if you attach it to a question.");
        }
        ui.add_space(4.0);
        ui.label(
            RichText::new("Clicks and keystrokes")
                .strong()
                .color(theme::accent(ui)),
        );
        if james_platform::can_control_input() {
            ui.label("James can press one key, click once or type text, but only after you see exactly what it will do and choose Allow once. Then it waits for you to check the result. Try it in a window where a mistake doesn't matter, because sent input can't be undone.");
        } else {
            ui.label("On this computer James never clicks or types. It explains what to do and you do it.");
        }
        ui.label(
            RichText::new("Your words")
                .strong()
                .color(theme::accent(ui)),
        );
        ui.label("Conversations are saved on this computer only, so they're there next time. You can turn that off or delete everything in Settings > Privacy.");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                go(app, SetupStep::Test);
            }
            if ui.add(theme::primary(ui, "Finish setup")).clicked() {
                go(app, SetupStep::Done);
                app.page = Page::Chat;
                app.focus_composer = true;
                app.info(format!("You're set up. Ask {} anything.", app.model.trim()));
            }
        });
    });
}
