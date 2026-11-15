//! A small, movable companion window and an in-process timer.
use crate::edge_dock::{self, Edge, Monitor};
use crate::session::{self, SavedReminder};
use eframe::egui::{self, ViewportBuilder, ViewportClass, ViewportCommand, ViewportId};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type DockResult = (
    Edge,
    Result<Vec<james_platform::Display>, james_platform::Error>,
);

pub struct Floating {
    pub visible: bool,
    minutes: u32,
    deadline: Option<Instant>,
    reminder_text: String,
    reminder_minutes: u32,
    reminders: Vec<Reminder>,
    remember_reminders: bool,
    changed: bool,
    media: crate::media_widget::MediaWidget,
    dock_pending: Option<Receiver<DockResult>>,
    dock_error: Option<String>,
}

struct Reminder {
    text: String,
    due: SystemTime,
    announced: bool,
}

const MAX_REMINDERS: usize = 10;
const MAX_REMINDER_CHARS: usize = 100;

impl Default for Floating {
    fn default() -> Self {
        Self {
            visible: false,
            minutes: 5,
            deadline: None,
            reminder_text: String::new(),
            reminder_minutes: 5,
            reminders: Vec::new(),
            remember_reminders: false,
            changed: false,
            media: crate::media_widget::MediaWidget::default(),
            dock_pending: None,
            dock_error: None,
        }
    }
}

impl Floating {
    pub fn restore(remember: bool, reminders: Vec<SavedReminder>) -> Self {
        let reminders = if remember {
            session::bounded_reminders(reminders)
        } else {
            Vec::new()
        };
        Self {
            remember_reminders: remember,
            reminders: reminders
                .into_iter()
                .filter_map(|r| {
                    Some(Reminder {
                        text: r.text,
                        due: UNIX_EPOCH.checked_add(Duration::from_secs(r.due_unix_seconds))?,
                        announced: false,
                    })
                })
                .collect(),
            ..Self::default()
        }
    }
    pub fn saved_reminders(&self) -> (bool, Vec<SavedReminder>) {
        (
            self.remember_reminders,
            if self.remember_reminders {
                self.reminders
                    .iter()
                    .map(|r| SavedReminder {
                        text: r.text.clone(),
                        due_unix_seconds: r
                            .due
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
        )
    }
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }
    /// A tiny octopus lives in the companion. It reflects local timer state,
    /// never an AI action, and remains recognizable without emoji fonts.
    pub fn pet_line(&self) -> &'static str {
        if let Some(deadline) = self.deadline {
            if deadline.saturating_duration_since(Instant::now()).is_zero() {
                return "Octopus is ringing the bell";
            }
            return "Octopus is watching your timer";
        }
        "Octopus is resting"
    }

    /// Paint a small octopus directly with egui so every supported platform
    /// can render it without font/emoji fallback or external image assets.
    fn paint_octopus(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let center = rect.center();
        let color = match self.deadline {
            Some(deadline) if deadline <= Instant::now() => egui::Color32::from_rgb(217, 106, 109),
            Some(_) => egui::Color32::from_rgb(67, 157, 186),
            None => egui::Color32::from_rgb(153, 125, 199),
        };

        // Eight curved-looking, rounded arms, drawn first behind the mantle.
        for arm in 0..8 {
            let lane = arm as f32 - 3.5;
            let base = egui::pos2(center.x + lane * 1.5, center.y + 3.0);
            let elbow = egui::pos2(
                center.x + lane * 2.5,
                center.y + 9.0 + (arm % 2) as f32,
            );
            let tip = egui::pos2(
                center.x + lane * 3.7,
                center.y + 7.0 - (arm % 3) as f32,
            );
            painter.add(egui::Shape::line(
                vec![base, elbow, tip],
                egui::Stroke::new(2.6, color),
            ));
            painter.circle_filled(tip, 1.3, color);
        }

        painter.circle_filled(egui::pos2(center.x, center.y - 4.0), 10.0, color);
        for dx in [-3.8, 3.8] {
            let eye = egui::pos2(center.x + dx, center.y - 5.5);
            painter.circle_filled(eye, 2.6, egui::Color32::WHITE);
            painter.circle_filled(eye, 1.2, egui::Color32::from_rgb(35, 43, 59));
        }
    }
    pub fn tick(&mut self, ctx: &egui::Context) {
        self.tick_at(SystemTime::now());
        if self.reminders.iter().any(|r| !r.announced) {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
    fn tick_at(&mut self, now: SystemTime) {
        for reminder in &mut self.reminders {
            if !reminder.announced && now >= reminder.due {
                reminder.announced = true;
                self.visible = true;
            }
        }
    }
    fn request_dock(&mut self, ctx: &egui::Context, edge: Edge) {
        if self.dock_pending.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = sender.send((edge, james_platform::list_display_bounds()));
            ctx.request_repaint();
        });
        self.dock_error = None;
        self.dock_pending = Some(receiver);
    }

    fn finish_dock(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.dock_pending else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.dock_pending = None;
                self.dock_error = Some("Could not read display positions.".into());
                return;
            }
        };
        self.dock_pending = None;
        let (edge, displays) = result;
        let displays = match displays {
            Ok(displays) => displays,
            Err(_) => {
                self.dock_error = Some("Could not read display positions.".into());
                return;
            }
        };
        let monitors: Vec<Monitor> = displays
            .iter()
            .filter(|display| display.primary)
            .filter_map(|display| display.bounds)
            .filter(|bounds| bounds.x == 0 && bounds.y == 0)
            .map(|bounds| Monitor {
                bounds: egui::Rect::from_min_size(
                    egui::pos2(bounds.x as f32, bounds.y as f32),
                    egui::vec2(bounds.width as f32, bounds.height as f32),
                ),
            })
            .collect();
        let (window, size) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
        let position = window.and_then(|window| {
            edge_dock::dock_position(window, ctx.pixels_per_point(), size, &monitors, edge)
        });
        if let Some(position) = position {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(position));
            self.dock_error = None;
        } else {
            self.dock_error = Some("Display geometry changed or is unavailable; move the companion onto the primary display and try again.".into());
        }
    }
    fn add_reminder(&mut self, now: SystemTime) -> bool {
        let text = self.reminder_text.trim();
        if text.is_empty()
            || text.chars().count() > MAX_REMINDER_CHARS
            || self.reminders.len() >= MAX_REMINDERS
            || text.chars().any(char::is_control)
        {
            return false;
        }
        self.reminders.push(Reminder {
            text: text.to_owned(),
            due: now + Duration::from_secs(u64::from(self.reminder_minutes.clamp(1, 1440)) * 60),
            announced: false,
        });
        self.changed = true;
        self.reminder_text.clear();
        true
    }
    /// Returns true when the person asks to bring the main window forward.
    pub fn show(&mut self, ctx: &egui::Context, status: &str, capture_in_progress: bool) -> bool {
        if !self.visible || capture_in_progress {
            return false;
        }
        let mut open_main = false;
        let mut close = false;
        let builder = ViewportBuilder::default()
            .with_title("James companion")
            .with_inner_size([300.0, 310.0])
            .with_min_inner_size([210.0, 140.0])
            .with_always_on_top()
            .with_taskbar(false);
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("james-floating"),
            builder,
            |ctx, class| {
                let content = |ui: &mut egui::Ui,
                               state: &mut Self,
                               open_main: &mut bool,
                               close: &mut bool| {
                    ui.horizontal(|ui| {
                        ui.strong("James");
                        if ui.button("Open").clicked() {
                            *open_main = true;
                        }
                        if ui.button("Close tab").clicked() {
                            *close = true;
                        }
                    });
                    ui.small(status);
                    ui.horizontal(|ui| {
                        state.paint_octopus(ui);
                        ui.label(egui::RichText::new(state.pet_line()).weak());
                    });
                    if cfg!(windows) && class != ViewportClass::Embedded {
                        ui.horizontal(|ui| {
                            let enabled = state.dock_pending.is_none();
                            if ui.add_enabled(enabled, egui::Button::new("Dock left")).clicked() { state.request_dock(ctx, Edge::Left); }
                            if ui.add_enabled(enabled, egui::Button::new("Dock right")).clicked() { state.request_dock(ctx, Edge::Right); }
                        });
                        if let Some(error) = &state.dock_error { ui.small(error); }
                    }
                    ui.separator();
                    if let Some(deadline) = state.deadline {
                        let left = deadline.saturating_duration_since(Instant::now());
                        if left.is_zero() {
                            ui.strong("Timer finished");
                            if ui.button("Dismiss").clicked() {
                                state.deadline = None;
                            }
                        } else {
                            let seconds = left.as_secs().saturating_add(1);
                            ui.label(format!("Timer: {}:{:02}", seconds / 60, seconds % 60));
                            if ui.button("Cancel timer").clicked() {
                                state.deadline = None;
                            }
                            ctx.request_repaint_after(Duration::from_secs(1));
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut state.minutes)
                                    .range(1..=1440)
                                    .suffix(" min"),
                            );
                            if ui.button("Start timer").clicked() {
                                state.deadline = Some(
                                    Instant::now()
                                        + Duration::from_secs(u64::from(state.minutes) * 60),
                                );
                            }
                        });
                    }
                    ui.small("Timer runs while James is open.");
                    ui.separator();
                    ui.strong("Reminders");
                    if ui.checkbox(&mut state.remember_reminders, "Save reminders on this computer").changed() { state.changed = true; }
                    let reminder_input = ui.add(egui::TextEdit::singleline(&mut state.reminder_text)
                        .hint_text("Reminder text")
                        .char_limit(MAX_REMINDER_CHARS));
                    reminder_input.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Reminder text"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut state.reminder_minutes).range(1..=1440).prefix("In ").suffix(" min"));
                        let can_add = !state.reminder_text.trim().is_empty() && state.reminders.len() < MAX_REMINDERS;
                        if ui.add_enabled(can_add, egui::Button::new("Add reminder")).clicked() {
                            state.add_reminder(SystemTime::now());
                        }
                    });
                    let mut remove = None;
                    for (index, reminder) in state.reminders.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            if SystemTime::now() >= reminder.due {
                                ui.strong(format!("Due: {}", reminder.text));
                            } else {
                                let remaining = reminder.due.duration_since(SystemTime::now()).unwrap_or_default().as_secs().div_ceil(60);
                                ui.label(format!("{} · {remaining} min", reminder.text));
                            }
                            if ui.small_button("Remove").clicked() { remove = Some(index); }
                        });
                    }
                    if let Some(index) = remove { state.reminders.remove(index); state.changed = true; }
                    if !state.reminders.is_empty() { ctx.request_repaint_after(Duration::from_secs(1)); }
                    ui.small("Due reminders open this companion while James is running. Saved reminders resume when James reopens. No system notifications are sent.");
                    ui.separator();
                    state.media.show(ui);
                };
                if class == ViewportClass::Embedded {
                    egui::Window::new("James companion")
                        .show(ctx, |ui| content(ui, self, &mut open_main, &mut close));
                } else {
                    self.finish_dock(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| content(ui, self, &mut open_main, &mut close));
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        close = true;
                    }
                }
            },
        );
        if close {
            self.visible = false;
        }
        if open_main {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }
        open_main
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_octopus_reflects_the_timer() {
        let state = Floating::default();
        assert_eq!(state.pet_line(), "Octopus is resting");
        let mut state = Floating {
            deadline: Some(Instant::now() + Duration::from_secs(30)),
            ..Floating::default()
        };
        assert_eq!(state.pet_line(), "Octopus is watching your timer");
        state.deadline = Some(Instant::now() - Duration::from_secs(1));
        assert_eq!(state.pet_line(), "Octopus is ringing the bell");
    }
    #[test]
    fn restored_overdue_reminders_open_once_and_unsaved_text_is_omitted() {
        let now = UNIX_EPOCH + Duration::from_secs(200);
        let saved = vec![SavedReminder {
            text: "Check the oven".into(),
            due_unix_seconds: 100,
        }];
        let mut state = Floating::restore(false, saved.clone());
        state.tick_at(now);
        assert!(!state.visible);
        assert!(state.saved_reminders().1.is_empty());
        let mut state = Floating::restore(true, saved.clone());
        state.tick_at(now);
        assert!(state.visible);
        state.visible = false;
        state.tick_at(now);
        assert!(!state.visible);
        assert_eq!(state.saved_reminders(), (true, saved));
    }
    #[test]
    fn reminders_are_bounded_and_keep_independent_deadlines() {
        let mut state = Floating::default();
        let now = SystemTime::now();
        assert!(!state.add_reminder(now));
        state.reminder_text = "First".into();
        state.reminder_minutes = 2;
        assert!(state.add_reminder(now));
        assert_eq!(
            state.reminders[0].due.duration_since(now).unwrap(),
            Duration::from_secs(120)
        );
        for n in 1..MAX_REMINDERS {
            state.reminder_text = format!("Reminder {n}");
            assert!(state.add_reminder(now));
        }
        state.reminder_text = "One too many".into();
        assert!(!state.add_reminder(now));
        assert_eq!(state.reminders.len(), MAX_REMINDERS);
    }
}
