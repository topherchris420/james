//! A small, movable companion window and an in-process timer.
use eframe::egui::{self, ViewportBuilder, ViewportClass, ViewportCommand, ViewportId};
use std::time::{Duration, Instant};

pub struct Floating {
    pub visible: bool,
    minutes: u32,
    deadline: Option<Instant>,
}

impl Default for Floating {
    fn default() -> Self {
        Self {
            visible: false,
            minutes: 5,
            deadline: None,
        }
    }
}

impl Floating {
    /// Returns true when the person asks to bring the main window forward.
    pub fn show(&mut self, ctx: &egui::Context, status: &str, capture_in_progress: bool) -> bool {
        if !self.visible || capture_in_progress {
            return false;
        }
        let mut open_main = false;
        let mut close = false;
        let builder = ViewportBuilder::default()
            .with_title("James companion")
            .with_inner_size([230.0, 150.0])
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
                };
                if class == ViewportClass::Embedded {
                    egui::Window::new("James companion")
                        .show(ctx, |ui| content(ui, self, &mut open_main, &mut close));
                } else {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| content(ui, self, &mut open_main, &mut close));
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
