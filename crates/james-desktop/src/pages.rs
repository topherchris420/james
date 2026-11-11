//! Screen context, Computer control and Settings.
use crate::{
    JamesApp, Page, PixelRect, ScreenTool, Tone, chat, copyable, guide, numeric_port, outline_rect,
    pixel_rect_from_drag, pixel_rect_to_screen, review,
    screen_context::{self, BrushColor, Point, Stroke},
    session::{SetupStep, ThemeChoice},
    theme,
};
use eframe::egui::{self, RichText};
use james_models::{LocalProvider, Support};
use james_platform::{Action, Key};

fn paint_annotations(ui: &egui::Ui, shown: egui::Rect, source: PixelRect, strokes: &[Stroke]) {
    let painter = ui.painter().with_clip_rect(shown);
    let scale = shown.width() / source.width.max(1) as f32;
    let position = |p: Point| {
        shown.min
            + egui::vec2(
                (p.x as f32 - source.x as f32) * scale,
                (p.y as f32 - source.y as f32) * shown.height() / source.height.max(1) as f32,
            )
    };
    for stroke in strokes {
        let [r, g, b, a] = stroke.color.rgba();
        let color = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        let width = stroke.width as f32 * scale;
        for &point in &stroke.points {
            painter.circle_filled(position(point), width / 2.0, color);
        }
        for pair in stroke.points.windows(2) {
            painter.line_segment(
                [position(pair[0]), position(pair[1])],
                egui::Stroke::new(width, color),
            );
        }
    }
}

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
    ui.label("Capture a screen when you choose. James hides for 3 seconds so you can bring the right app to the front, then shows you the picture. Nothing is captured in the background.");
    app.load_displays();
    let displays = app.displays.clone().unwrap_or_default();
    if displays.len() > 1 {
        ui.horizontal(|ui| {
            ui.label("Monitor:");
            let selected = displays
                .iter()
                .find(|d| d.id == app.capture_display)
                .map(|d| d.label.clone())
                .unwrap_or_else(|| "Primary display".to_owned());
            egui::ComboBox::from_id_salt("capture-display")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for d in &displays {
                        ui.selectable_value(&mut app.capture_display, d.id.clone(), &d.label);
                    }
                });
        });
        ui.small("Reviewed clicks work on the primary monitor only. Other monitors are for questions and Guide me.");
    }
    if ui
        .add_enabled(!app.busy(), theme::primary(ui, "Capture screen"))
        .clicked()
    {
        app.capture(ctx);
    }
    let mut picked = None;
    let mut discard = false;
    let prior_annotations = app.annotations.clone();
    let mut overlay_error = None;
    // None leaves the selection as it is; Some(x) sets it after the read-only
    // borrow of the screenshot ends.
    let mut set_selection: Option<Option<PixelRect>> = None;
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
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut app.screen_tool, ScreenTool::Area, "Select area");
            ui.selectable_value(&mut app.screen_tool, ScreenTool::Point, "Point");
            ui.selectable_value(&mut app.screen_tool, ScreenTool::Draw, "Draw");
            egui::ComboBox::from_id_salt("mark-color")
                .selected_text(format!("{:?}", app.brush_color))
                .show_ui(ui, |ui| {
                    for color in [
                        BrushColor::Accent,
                        BrushColor::Blue,
                        BrushColor::Green,
                        BrushColor::Amber,
                        BrushColor::Red,
                        BrushColor::White,
                    ] {
                        ui.selectable_value(&mut app.brush_color, color, format!("{color:?}"));
                    }
                });
            if james_platform::can_control_input() && screen.from_primary {
                ui.selectable_value(
                    &mut app.screen_tool,
                    ScreenTool::ReviewedClick,
                    "Reviewed click",
                );
            }
            if ui
                .add_enabled(!app.annotations.is_empty(), egui::Button::new("Undo mark"))
                .clicked()
            {
                app.annotations.pop();
            }
            if ui
                .add_enabled(
                    !app.annotations.is_empty(),
                    egui::Button::new("Clear marks"),
                )
                .clicked()
            {
                app.annotations.clear();
            }
        });
        ui.small(match app.screen_tool {
            ScreenTool::Area => "Drag a rectangle to send just that area with your next attached question.",
            ScreenTool::Point => "Click to mark what your question refers to. Pointing does not operate the app.",
            ScreenTool::Draw => "Drag to circle or underline content. Your marks are included in attached screen questions.",
            ScreenTool::ReviewedClick => "Choose a click for review. Nothing is clicked until you approve it on Computer control.",
        });
        let response = ui
            .add(
                egui::Image::new(&screen.texture)
                    .max_size(egui::vec2(ui.available_width(), 440.0))
                    .maintain_aspect_ratio(true)
                    .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_text("Your screenshot, kept in memory only");
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Image, true, "Screenshot of your screen")
        });
        if response.clicked()
            && app.screen_tool == ScreenTool::ReviewedClick
            && james_platform::can_control_input()
            && screen.from_primary
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
        let shown = response.rect;
        let point = |pos: egui::Pos2| Point {
            x: (((pos.x - shown.min.x) / shown.width()).clamp(0.0, 1.0)
                * (screen.capture.width - 1) as f32) as f64,
            y: (((pos.y - shown.min.y) / shown.height()).clamp(0.0, 1.0)
                * (screen.capture.height - 1) as f32) as f64,
        };
        if app.screen_tool == ScreenTool::Point
            && response.clicked()
            && app.annotations.len() < screen_context::MAX_STROKES
            && app
                .annotations
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
                < screen_context::MAX_POINTS
            && let Some(pos) = response.interact_pointer_pos()
        {
            app.annotations.push(Stroke {
                points: vec![point(pos)],
                color: app.brush_color,
                width: 18.0,
            });
        }
        if app.screen_tool == ScreenTool::Draw
            && response.drag_started()
            && app.annotations.len() < screen_context::MAX_STROKES
            && app
                .annotations
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
                < screen_context::MAX_POINTS
            && let Some(pos) = response.interact_pointer_pos()
        {
            app.annotations.push(Stroke {
                points: vec![point(pos)],
                color: app.brush_color,
                width: 6.0,
            });
            app.drawing = true;
        }
        if app.screen_tool == ScreenTool::Draw
            && response.dragged()
            && app.drawing
            && app
                .annotations
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
                < screen_context::MAX_POINTS
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(stroke) = app.annotations.last_mut()
        {
            let next = point(pos);
            if stroke
                .points
                .last()
                .is_none_or(|last| (last.x - next.x).hypot(last.y - next.y) >= 1.0)
            {
                stroke.points.push(next);
            }
        }
        if response.drag_stopped() {
            app.drawing = false;
        }
        if app.screen_tool == ScreenTool::Area && response.drag_started() {
            app.region_drag_start = response.interact_pointer_pos();
        }
        if app.screen_tool == ScreenTool::Area
            && response.dragged()
            && let (Some(start), Some(now)) =
                (app.region_drag_start, response.interact_pointer_pos())
        {
            outline_rect(
                ui.painter(),
                egui::Rect::from_two_pos(start, now).intersect(shown),
                egui::Stroke::new(2.0_f32, theme::accent(ui)),
            );
        }
        if app.screen_tool == ScreenTool::Area && response.drag_stopped() {
            let start = app.region_drag_start.take();
            if let (Some(start), Some(end)) = (start, response.interact_pointer_pos()) {
                set_selection = Some(pixel_rect_from_drag(
                    shown,
                    start,
                    end,
                    screen.capture.width,
                    screen.capture.height,
                ));
            }
        }
        if let Some(rect) = screen.selection {
            outline_rect(
                ui.painter(),
                pixel_rect_to_screen(shown, rect, screen.capture.width, screen.capture.height),
                egui::Stroke::new(2.0_f32, theme::accent(ui)),
            );
        }
        paint_annotations(
            ui,
            shown,
            PixelRect {
                x: 0,
                y: 0,
                width: screen.capture.width,
                height: screen.capture.height,
            },
            &app.annotations,
        );
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
        if app.overlay.is_open() {
            if ui.button("Hide desktop guidance").clicked() {
                app.overlay.close();
            }
        } else if ui
            .add_enabled(
                !app.busy() && age <= 60,
                egui::Button::new("Show marks on desktop"),
            )
            .clicked()
        {
            let marks = app
                .guide_marks
                .as_ref()
                .filter(|g| g.screenshot == screen.at)
                .map(|g| {
                    g.marks
                        .iter()
                        .map(|m| crate::overlay::GuideMark {
                            step: m.step,
                            x: f64::from(m.x),
                            y: f64::from(m.y),
                            label: String::new(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Err(error) = app.overlay.open(
                ctx,
                screen.capture.width,
                screen.capture.height,
                screen.from_primary,
                &app.annotations,
                &marks,
            ) {
                overlay_error = Some(error);
            }
        }
        ui.small("Desktop marks are visual guidance only. Model-placed numbers may be wrong; verify the control before acting.");
        if let Some(error) = &app.overlay.error {
            ui.colored_label(ui.visuals().warn_fg_color, error);
        }
        if let Some(rect) = screen.selection {
            ui.small(format!(
                "Your next attached question will be about the selected {} × {} area only.",
                rect.width, rect.height
            ));
            if ui.button("Use whole screen").clicked() {
                set_selection = Some(None);
            }
            ui.label(RichText::new("Selected area preview").strong());
            let uv = egui::Rect::from_min_max(
                egui::pos2(
                    rect.x as f32 / screen.capture.width as f32,
                    rect.y as f32 / screen.capture.height as f32,
                ),
                egui::pos2(
                    (rect.x + rect.width) as f32 / screen.capture.width as f32,
                    (rect.y + rect.height) as f32 / screen.capture.height as f32,
                ),
            );
            let preview = ui.add(
                egui::Image::new(&screen.texture).uv(uv).fit_to_exact_size(
                    egui::vec2(rect.width as f32, rect.height as f32)
                        * (ui.available_width() / rect.width as f32)
                            .min(180.0 / rect.height as f32)
                            .min(1.0),
                ),
            );
            preview.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Image,
                    true,
                    "Selected area sent with your question",
                )
            });
            paint_annotations(ui, preview.rect, rect, &app.annotations);
        } else if sees != Support::No {
            ui.small("Drag across the picture to ask about just one area; otherwise the whole screen is sent.");
        }
        if !app.annotations.is_empty() {
            ui.small("Colored marks are included with attached questions. Guide me and reviewed action suggestions use the original full screenshot.");
        }
        discard = ui.button("Discard screenshot").clicked();
    } else {
        ui.add_space(8.0);
        ui.label(RichText::new("No screenshot yet.").strong());
        ui.label("Use one to ask a vision model about what's on screen, or press Guide me under the message box to capture and ask in one step.");
    }
    if discard {
        app.screen = None;
        app.overlay.close();
        app.annotations.clear();
        app.drawing = false;
        app.region_drag_start = None;
        app.pending = None;
        app.attach_screen = false;
    }
    if app.annotations != prior_annotations {
        app.overlay.close();
    }
    if let Some(error) = overlay_error {
        app.fail(error);
    }
    if let Some(selection) = set_selection
        && let Some(screen) = app.screen.as_mut()
    {
        screen.selection = selection;
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

/// The action exactly as it will be (or was) sent. Typing is shown in full,
/// in a scrollable box, so no character of an approved input is hidden.
fn exact_action(ui: &mut egui::Ui, lead: &str, action: &Action, id: &str) {
    let Action::TypeText { text } = action else {
        ui.label(format!("{lead}: {}.", review::describe(action)));
        return;
    };
    ui.label(format!(
        "{lead} type {} characters, exactly:",
        text.chars().count()
    ));
    egui::Frame::group(ui.style()).show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(id)
            .max_height(180.0)
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(text).monospace()).wrap());
            });
    });
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
                exact_action(ui, "James was asked to", &record.action, "recovery-text");
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
    if let Some((target, text)) = &app.external_dictation {
        ui.heading("Review dictation for another app");
        ui.label(format!(
            "Destination: {} ({})",
            target.title, target.process_name
        ));
        ui.label(format!("Focused control: {}", target.focus_class));
        ui.label("Text to send:");
        egui::ScrollArea::vertical()
            .id_salt("external_dictation_review")
            .max_height(140.0)
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(text).monospace()).wrap());
            });
        ui.small("After choosing Send, switch back to the same field within three seconds. Windows checks the window and native control, but a custom app can change fields inside that control. Check the result; sending may be partial.");
        if ui.button("Copy dictation").clicked() {
            ui.ctx().copy_text(text.clone());
        }
        if app.external_attempted {
            ui.small("A delivery was attempted. Check the destination before copying or dictating again.");
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !app.busy() && !app.external_attempted,
                    theme::primary(ui, "Send reviewed dictation in 3 seconds"),
                )
                .clicked()
            {
                app.deliver_external_dictation();
            }
            if ui
                .add_enabled(!app.busy(), egui::Button::new("Discard dictation"))
                .clicked()
            {
                app.external_dictation = None;
                app.external_attempted = false;
            }
        });
        ui.separator();
    }
    egui::CollapsingHeader::new("Create a new text file with the model").show(ui, |ui| {
        ui.small("Choose a directory and one new filename. Your filename and instructions are sent to the configured model service, which may relay them elsewhere. Existing files cannot be replaced.");
        ui.label("Destination directory");
        ui.add_enabled(!app.busy() && app.file_proposal.is_none() && app.file_source.is_none(), egui::TextEdit::singleline(&mut app.file_directory).desired_width(420.0));
        egui::CollapsingHeader::new("Revise an existing file as a new copy").show(ui, |ui| {
            ui.small("Optional: preview one UTF-8 file in this directory (up to 4 KiB). The preview freezes the source sent with your instructions. The original file stays intact, including later changes made by other apps.");
            let label = ui.label("Source filename relative to this directory");
            ui.add_enabled(!app.busy() && app.file_proposal.is_none() && app.file_source.is_none(), egui::TextEdit::singleline(&mut app.file_source_name).desired_width(300.0)).labelled_by(label.id);
            if ui.add_enabled(!app.busy() && app.file_proposal.is_none() && app.file_source.is_none(), egui::Button::new("Preview revision source")).clicked() { app.preview_file_source(); }
        });
        if let Some(source) = &app.file_source {
            ui.label(format!("Frozen source: {}", source.root().join(source.filename()).display()));
            egui::ScrollArea::vertical().id_salt("revision_source_review").max_height(180.0).show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(source.content()).monospace()).wrap());
            });
            if ui.add_enabled(!app.busy() && app.file_proposal.is_none(), egui::Button::new("Remove revision source")).clicked() { app.file_source = None; app.file_source_name.clear(); }
        }
        ui.label("New filename");
        ui.add_enabled(!app.busy() && app.file_proposal.is_none(), egui::TextEdit::singleline(&mut app.file_name).desired_width(300.0));
        ui.label(if app.file_source.is_some() { "How should the revised copy change?" } else { "What should the file contain?" });
        ui.add_enabled(!app.busy() && app.file_proposal.is_none(), egui::TextEdit::multiline(&mut app.file_instruction).desired_rows(3).desired_width(420.0));
        if ui.add_enabled(!app.busy() && app.file_proposal.is_none(), egui::Button::new("Prepare file for review")).clicked() { app.propose_new_file(); }
        if let Some(proposal) = &app.file_proposal {
            ui.separator();
            ui.label(format!("New file: {}", proposal.root().join(proposal.filename()).display()));
            ui.label("Review the complete generated text:");
            egui::ScrollArea::vertical().id_salt("new_file_review").max_height(240.0).show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(proposal.content()).monospace()).wrap());
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!app.busy(), theme::primary(ui, "Create reviewed file")).clicked() { app.create_reviewed_file(); }
                if ui.add_enabled(!app.busy(), egui::Button::new("Discard file proposal")).clicked() { app.file_proposal = None; }
            });
        }
    });
    egui::CollapsingHeader::new("Open a Windows settings page").show(ui, |ui| {
        if !cfg!(windows) { ui.label("Opening these settings pages is available on Windows only."); return; }
        ui.small("Choose a page yourself or ask the configured model to select one. Review the destination before opening it. James opens the page; you change the settings.");
        ui.add_enabled_ui(!app.busy() && app.settings_proposal.is_none(), |ui| {
            egui::ComboBox::from_id_salt("settings_destination").selected_text(app.settings_destination.label()).show_ui(ui, |ui| {
                for page in crate::settings_page::SettingsPage::ALL { ui.selectable_value(&mut app.settings_destination, page, page.label()); }
            });
            if ui.button("Review selected settings page").clicked() { app.settings_proposal = Some(crate::settings_page::Proposal::manual(app.settings_destination)); }
            let label = ui.label("What setting do you need help finding?");
            ui.add(egui::TextEdit::singleline(&mut app.settings_instruction).desired_width(420.0).char_limit(4096)).labelled_by(label.id);
            if ui.button("Ask model for a settings page").clicked() { app.propose_settings_page(); }
        });
        if let Some(proposal) = &app.settings_proposal {
            ui.strong(format!("Windows {} settings", proposal.page().label()));
            ui.label(proposal.page().uri());
            ui.label(proposal.reason());
            let label = format!("Open reviewed {} settings", proposal.page().label());
            ui.horizontal(|ui| {
                if ui.add_enabled(!app.busy(), theme::primary(ui, &label)).clicked() { app.open_reviewed_settings(); }
                if ui.add_enabled(!app.busy(), egui::Button::new("Discard settings proposal")).clicked() { app.settings_proposal = None; }
            });
        }
    });
    ui.separator();
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
    match &pending.proposal.action {
        Action::TypeText { .. } => {
            exact_action(ui, "James will", &pending.proposal.action, "review-text")
        }
        action => {
            ui.label(
                RichText::new(review::describe(action))
                    .size(17.0)
                    .strong()
                    .color(theme::accent(ui)),
            );
        }
    }
    if let Some(note) = &pending.review_note {
        // A second model pass's check, framed and labelled so it never reads
        // as James's own guarantee.
        egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .show(ui, |ui| {
                ui.label(RichText::new("Second-pass review").strong());
                ui.label(note);
                ui.small(
                    "A second model pass wrote this check. It is not a guarantee; judge the action itself.",
                );
            });
    }
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
    if pending.from_model {
        // Framed and labelled, so the model's words never read as James's.
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(format!("Why: {}", pending.proposal.rationale));
            ui.small(
                "This reason was written by the model. It is not a guarantee; judge the action itself.",
            );
        });
    } else {
        ui.label(format!("Why: {}", pending.proposal.rationale));
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
    // A model's click, typing or Enter, Space, Delete or Backspace can press,
    // send, submit or delete: it takes a deliberate second step to approve.
    let second_look = pending.from_model && review::needs_second_look(&pending.proposal.action);
    let mut checked = pending.checked;
    if second_look {
        ui.add_enabled(
            enabled && fresh,
            egui::Checkbox::new(&mut checked, "I checked what this does and where it goes"),
        )
        .on_hover_text("A model proposed this. Check the screenshot, the window and every character first: a click, typing or Enter, Space, Delete or Backspace can press buttons, send, submit or delete.");
    }
    ui.small("Allow once starts a 5-second countdown so you can switch to the app. Esc or Cancel stops it. There is no system-wide stop key.");
    // Defense in depth: this page already returns early without input support.
    let can_input = james_platform::can_control_input();
    let mut allow = false;
    let mut deny = false;
    ui.horizontal(|ui| {
        allow = ui
            .add_enabled(
                enabled && fresh && can_input && (checked || !second_look),
                theme::primary(ui, "Allow once"),
            )
            .on_hover_text("Approve this one action. It runs after a 5-second countdown.")
            .on_disabled_hover_text(if second_look && !checked {
                "Tick “I checked what this does and where it goes” first."
            } else {
                "Not available while another task runs, an action waits to be checked, or the screenshot is too old."
            })
            .clicked();
        deny = ui.button("Deny").clicked();
    });
    if let Some(pending) = app.pending.as_mut() {
        pending.checked = checked;
    }
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
    review_settings(app, ui);
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
    reading_voice(app, ui);
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

/// The reading-voice picker: the installed voices James can read answers with.
/// The list loads once off the UI thread; an empty list leaves only the default.
fn reading_voice(app: &mut JamesApp, ui: &mut egui::Ui) {
    app.load_voices();
    let voices = app.available_voices.clone();
    ui.horizontal(|ui| {
        let label = ui.label("Reading voice");
        let selected = if app.tts_voice.trim().is_empty() {
            "System default".to_owned()
        } else {
            app.tts_voice.clone()
        };
        let output = egui::ComboBox::from_id_salt("tts-voice")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                let mut changed = ui
                    .selectable_value(&mut app.tts_voice, String::new(), "System default")
                    .changed();
                match &voices {
                    None => {
                        ui.label("Finding installed voices…");
                    }
                    Some(voices) if voices.is_empty() => {
                        ui.label("No installed voices were found.");
                    }
                    Some(voices) => {
                        for voice in voices {
                            changed |= ui
                                .selectable_value(&mut app.tts_voice, voice.clone(), voice)
                                .changed();
                        }
                    }
                }
                changed
            });
        output.response.labelled_by(label.id);
        if output.inner.unwrap_or(false) {
            app.save_session();
        }
    });
    ui.small("James reads answers with this installed system voice. Add more voices in your operating system's speech settings.");
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

fn review_settings(app: &mut JamesApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Reviewed actions").size(18.0).strong());
    if ui
        .checkbox(
            &mut app.prefs.review_actions,
            "Double-check a proposed action before I review it",
        )
        .changed()
    {
        app.save_session();
    }
    ui.label("When Suggest one action proposes a click, key or typing, a second model pass looks at the same screenshot and writes a short note — flagging anything irreversible or off-target — shown with the action on the review card. You still approve every action yourself. This costs one extra model request, so it adds a short wait.");
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
            app.conversation_change = None;
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
    let shortcut = app.prefs.shortcut_chords.summon.label();
    let previous = app.prefs.clone();
    if ui
        .checkbox(
            &mut app.prefs.summon_shortcut,
            format!("Summon James from other apps with {shortcut}"),
        )
        .changed()
    {
        app.summon_error = None;
        app.save_shortcut_preferences(previous);
    }
    ui.small("Summoning opens the compact companion. The shortcut does not capture your screen or start the microphone.");
    let shortcut = app.prefs.shortcut_chords.talk.label();
    let previous = app.prefs.clone();
    if ui
        .checkbox(
            &mut app.prefs.talk_shortcut,
            format!("Start Talk with {shortcut}"),
        )
        .changed()
    {
        app.talk_shortcut_error = None;
        app.save_shortcut_preferences(previous);
    }
    let shortcut = app.prefs.shortcut_chords.dictate.label();
    let previous = app.prefs.clone();
    if ui
        .checkbox(
            &mut app.prefs.dictation_shortcut,
            format!("Start Dictate with {shortcut}"),
        )
        .changed()
    {
        app.dictation_shortcut_error = None;
        app.save_shortcut_preferences(previous);
    }
    egui::CollapsingHeader::new("Customize global shortcuts").show(ui, |ui| {
        ui.small("Use Ctrl, Alt, Shift or Meta (Cmd on macOS) with a letter, digit or Space. Include Ctrl, Alt or Meta. Letter keys refer to physical key positions. Disable a shortcut before exchanging its chord with another action.");
        for (index, label) in ["Summon shortcut", "Talk shortcut", "Dictation shortcut"].into_iter().enumerate() {
            ui.push_id(index, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.label(label);
                    ui.add(egui::TextEdit::singleline(&mut app.shortcut_drafts[index]).desired_width(190.0).char_limit(80)).labelled_by(response.id);
                    if ui.button(format!("Apply {label}")).clicked() { app.apply_shortcut_draft(index); }
                });
            });
        }
        ui.small("If another app owns a new chord, James keeps the previously registered chord and reports the error.");
    });
    if let Some(error) = &app.shortcut_config_error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    ui.small("These shortcuts start the microphone. Dictation ends when you pause; Talk keeps listening after each reply until stopped.");
    if cfg!(windows)
        && ui
            .checkbox(
                &mut app.prefs.dictation_other_app,
                "Use the dictation shortcut for another app",
            )
            .changed()
    {
        app.save_session();
    }
    if cfg!(windows) {
        ui.small("Focus a field in another app, then press the enabled dictation shortcut. Review the transcript and destination in James before sending.");
    }
    for error in [
        app.talk_shortcut_error.clone(),
        app.dictation_shortcut_error.clone(),
    ]
    .into_iter()
    .flatten()
    {
        ui.colored_label(ui.visuals().warn_fg_color, error);
    }
    if (app.talk_shortcut_error.is_some() || app.dictation_shortcut_error.is_some())
        && ui.button("Retry voice shortcuts").clicked()
    {
        app.talk_shortcut_error = None;
        app.dictation_shortcut_error = None;
    }
    if ui
        .button(if app.floating.visible {
            "Hide floating companion"
        } else {
            "Show floating companion"
        })
        .clicked()
    {
        app.floating.visible = !app.floating.visible;
    }
    ui.small("The floating companion stays on top and includes a timer while James is open.");
    if let Some(error) = app.summon_error.clone() {
        ui.colored_label(ui.visuals().warn_fg_color, error);
        if ui.button("Retry shortcut registration").clicked() {
            app.summon_error = None;
        }
    }
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
