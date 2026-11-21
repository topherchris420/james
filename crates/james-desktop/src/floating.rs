//! A small, movable companion window and an in-process timer.
use crate::edge_dock::{self, Edge, Monitor};
use crate::octopus::{Mood, Octopus};
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
    octopus: Octopus,
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
            octopus: Octopus::default(),
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
    pub fn launcher(&mut self, ui: &mut egui::Ui) -> bool {
        let clicked = ui
            .button(if self.visible {
                "Hide octopus"
            } else {
                "Octopus"
            })
            .on_hover_text("Show or hide the floating octopus, timer and reminders")
            .clicked();
        if clicked {
            self.visible = !self.visible;
            ui.ctx().request_repaint();
        }
        clicked
    }

    fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    fn show_timer(&mut self, ui: &mut egui::Ui, remaining: Option<Duration>) {
        match remaining {
            Some(left) if left.is_zero() => {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Timer finished");
                    if ui.add(crate::theme::primary(ui, "Dismiss")).clicked() {
                        self.deadline = None;
                    }
                });
            }
            Some(left) => {
                let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}:{:02}", seconds / 60, seconds % 60))
                            .size(23.0)
                            .monospace(),
                    );
                    if ui.button("Cancel timer").clicked() {
                        self.deadline = None;
                    }
                });
                let fraction = left.as_secs_f32() / (self.minutes as f32 * 60.0);
                ui.add(egui::ProgressBar::new(fraction).desired_height(4.0));
                ui.ctx().request_repaint_after(Duration::from_secs(1));
            }
            None => {
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.minutes)
                            .range(1..=1440)
                            .suffix(" min"),
                    )
                    .widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::DragValue,
                            true,
                            "Timer minutes",
                        )
                    });
                    if ui.add(crate::theme::primary(ui, "Start timer")).clicked() {
                        self.deadline = Some(
                            Instant::now() + Duration::from_secs(u64::from(self.minutes) * 60),
                        );
                    }
                });
            }
        }
    }

    fn reminder_rows(&mut self, ui: &mut egui::Ui, now: SystemTime, due: bool) {
        let mut remove = None;
        for (index, reminder) in self
            .reminders
            .iter()
            .enumerate()
            .filter(|(_, r)| (r.due <= now) == due)
        {
            ui.horizontal_wrapped(|ui| {
                if due {
                    ui.strong(format!("Due: {}", reminder.text));
                } else {
                    let minutes = reminder
                        .due
                        .duration_since(now)
                        .unwrap_or_default()
                        .as_secs()
                        .div_ceil(60);
                    ui.label(format!("{} · {minutes} min", reminder.text));
                }
                if ui.small_button("Remove").clicked() {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            self.reminders.remove(index);
            self.changed = true;
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
    fn reminder_editor(&mut self, ui: &mut egui::Ui, now: SystemTime) {
        if ui
            .checkbox(
                &mut self.remember_reminders,
                "Save reminders on this computer",
            )
            .changed()
        {
            self.changed = true;
        }
        ui.add(
            egui::TextEdit::singleline(&mut self.reminder_text)
                .hint_text("Reminder text")
                .char_limit(MAX_REMINDER_CHARS)
                .desired_width(ui.available_width()),
        )
        .widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Reminder text")
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.reminder_minutes)
                    .range(1..=1440)
                    .prefix("In ")
                    .suffix(" min"),
            );
            let can_add =
                !self.reminder_text.trim().is_empty() && self.reminders.len() < MAX_REMINDERS;
            if ui
                .add_enabled(can_add, egui::Button::new("Add reminder"))
                .clicked()
            {
                self.add_reminder(now);
            }
        });
        self.reminder_rows(ui, now, false);
        ui.small("Due reminders open this companion while James is running. Saved reminders resume when James reopens. No system notifications are sent.");
    }

    fn content(
        &mut self,
        ui: &mut egui::Ui,
        status: &str,
        native: bool,
        reduce_motion: bool,
    ) -> (bool, bool) {
        let actions = ui
            .horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Octopus").size(20.0).strong());
                let open = ui.button("Open James").clicked();
                let hide = ui.button("Hide window").clicked();
                (open, hide)
            })
            .inner;
        if status != "Off" {
            ui.small(status);
        }
        let now = SystemTime::now();
        self.reminder_rows(ui, now, true);
        let remaining = self.remaining(Instant::now());
        let mood = Mood::from_remaining(remaining);
        self.octopus.portrait(ui, mood, reduce_motion);
        ui.strong(mood.description());
        self.show_timer(ui, remaining);
        ui.small("Timer runs while James is open.");
        if cfg!(windows) && native {
            ui.horizontal_wrapped(|ui| {
                let enabled = self.dock_pending.is_none();
                if ui
                    .add_enabled(enabled, egui::Button::new("Dock left"))
                    .clicked()
                {
                    self.request_dock(ui.ctx(), Edge::Left);
                }
                if ui
                    .add_enabled(enabled, egui::Button::new("Dock right"))
                    .clicked()
                {
                    self.request_dock(ui.ctx(), Edge::Right);
                }
            });
            if let Some(error) = &self.dock_error {
                ui.small(error);
            }
        }
        ui.separator();
        egui::CollapsingHeader::new("Reminders")
            .show_unindented(ui, |ui| self.reminder_editor(ui, now));
        if self.reminders.iter().any(|r| !r.announced) {
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
        if cfg!(windows) {
            self.media.show(ui);
        }
        actions
    }

    /// Returns true when the person asks to bring the main window forward.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        status: &str,
        capture_in_progress: bool,
        reduce_motion: bool,
    ) -> bool {
        if !self.visible || capture_in_progress {
            self.octopus = Octopus::default();
            return false;
        }

        let builder = ViewportBuilder::default()
            .with_title("James companion")
            .with_inner_size([324.0, if cfg!(windows) { 425.0 } else { 362.0 }])
            .with_min_inner_size([210.0, 140.0])
            .with_always_on_top()
            .with_taskbar(false);
        let (open_main, close) = ctx.show_viewport_immediate(
            ViewportId::from_hash_of("james-floating"),
            builder,
            |ctx, class| {
                if class == ViewportClass::Embedded {
                    egui::Window::new("James companion")
                        .default_size([308.0, 350.0])
                        .vscroll(true)
                        .show(ctx, |ui| self.content(ui, status, false, reduce_motion))
                        .and_then(|window| window.inner)
                        .unwrap_or_default()
                } else {
                    self.finish_dock(ctx);
                    let (open, hide) = egui::CentralPanel::default()
                        .show(ctx, |ui| {
                            egui::ScrollArea::vertical()
                                .show(ui, |ui| self.content(ui, status, true, reduce_motion))
                                .inner
                        })
                        .inner;
                    (open, hide || ctx.input(|i| i.viewport().close_requested()))
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
        let now = Instant::now();
        let mut state = Floating::default();
        assert_eq!(Mood::from_remaining(state.remaining(now)), Mood::Rest);
        state.deadline = Some(now + Duration::from_secs(30));
        assert_eq!(Mood::from_remaining(state.remaining(now)), Mood::Watch);
        assert_eq!(
            Mood::from_remaining(state.remaining(now + Duration::from_secs(30))),
            Mood::Bell
        );
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
