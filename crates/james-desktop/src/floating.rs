//! A small, movable companion window and an in-process timer.
use crate::edge_dock::{self, Edge, Monitor};
use eframe::egui::{self, ViewportBuilder, ViewportClass, ViewportCommand, ViewportId};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

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
    media: crate::media_widget::MediaWidget,
    dock_pending: Option<Receiver<DockResult>>,
    dock_error: Option<String>,
}

struct Reminder {
    text: String,
    due: Instant,
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
            media: crate::media_widget::MediaWidget::default(),
            dock_pending: None,
            dock_error: None,
        }
    }
}

impl Floating {
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
    fn add_reminder(&mut self, now: Instant) -> bool {
        let text = self.reminder_text.trim();
        if text.is_empty()
            || text.chars().count() > MAX_REMINDER_CHARS
            || self.reminders.len() >= MAX_REMINDERS
        {
            return false;
        }
        self.reminders.push(Reminder {
            text: text.to_owned(),
            due: now + Duration::from_secs(u64::from(self.reminder_minutes.clamp(1, 1440)) * 60),
        });
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
                    let reminder_input = ui.add(egui::TextEdit::singleline(&mut state.reminder_text)
                        .hint_text("Reminder text")
                        .char_limit(MAX_REMINDER_CHARS));
                    reminder_input.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Reminder text"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut state.reminder_minutes).range(1..=1440).prefix("In ").suffix(" min"));
                        let can_add = !state.reminder_text.trim().is_empty() && state.reminders.len() < MAX_REMINDERS;
                        if ui.add_enabled(can_add, egui::Button::new("Add reminder")).clicked() {
                            state.add_reminder(Instant::now());
                        }
                    });
                    let mut remove = None;
                    for (index, reminder) in state.reminders.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            if Instant::now() >= reminder.due {
                                ui.strong(format!("Due: {}", reminder.text));
                            } else {
                                let remaining = reminder.due.saturating_duration_since(Instant::now()).as_secs().div_ceil(60);
                                ui.label(format!("{} · {remaining} min", reminder.text));
                            }
                            if ui.small_button("Remove").clicked() { remove = Some(index); }
                        });
                    }
                    if let Some(index) = remove { state.reminders.remove(index); }
                    if !state.reminders.is_empty() { ctx.request_repaint_after(Duration::from_secs(1)); }
                    ui.small("Reminders appear here while James is open; they are not saved or sent as system notifications.");
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
    fn reminders_are_bounded_and_keep_independent_deadlines() {
        let mut state = Floating::default();
        let now = Instant::now();
        assert!(!state.add_reminder(now));
        state.reminder_text = "First".into();
        state.reminder_minutes = 2;
        assert!(state.add_reminder(now));
        assert_eq!(
            state.reminders[0].due.duration_since(now),
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
