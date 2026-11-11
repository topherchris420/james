//! Transparent, non-interactive guidance over the captured primary display.
//!
//! A captured pixel is only useful for desktop guidance if its physical
//! display geometry still matches. Until monitor-specific DPI positioning is
//! verified on all platforms, this deliberately supports Windows primary
//! displays only, and hides the marks if geometry cannot be confirmed.

use crate::screen_context::Stroke;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2, ViewportBuilder, ViewportClass, ViewportId};

#[derive(Default)]
pub struct Overlay {
    active: Option<Guidance>,
    pub error: Option<String>,
}

/// A numbered instruction positioned in source-snapshot pixels. Coordinates
/// must describe a location on the original, uncropped primary capture.
#[derive(Clone, Debug, PartialEq)]
pub struct GuideMark {
    pub step: u32,
    pub x: f64,
    pub y: f64,
    pub label: String,
}

struct Guidance {
    width: u32,
    height: u32,
    strokes: Vec<Stroke>,
    marks: Vec<GuideMark>,
}

impl Overlay {
    /// Place the existing snapshot marks on the primary display, without
    /// taking another screenshot or giving the overlay mouse input.
    pub fn open(
        &mut self,
        ctx: &egui::Context,
        width: u32,
        height: u32,
        primary: bool,
        strokes: &[Stroke],
        marks: &[GuideMark],
    ) -> Result<(), String> {
        if !cfg!(windows) {
            return Err("Desktop guidance is currently available on Windows only.".into());
        }
        if !primary {
            return Err("Desktop guidance currently supports the primary display only.".into());
        }
        if ctx.embed_viewports() {
            return Err("Desktop guidance requires a native window.".into());
        }
        if strokes.is_empty() && marks.is_empty() {
            return Err("Add a point, drawing, or guidance step first.".into());
        }
        let marks = validated_marks(width, height, marks)?;
        let (window, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
        checked_geometry(width, height, ctx.pixels_per_point(), window, monitor)?;
        self.active = Some(Guidance {
            width,
            height,
            strokes: strokes.to_vec(),
            marks,
        });
        self.error = None;
        Ok(())
    }

    pub fn close(&mut self) {
        self.active = None;
        self.error = None;
    }

    pub fn is_open(&self) -> bool {
        self.active.is_some()
    }

    /// Call every frame; omit the native viewport during capture. The parent
    /// still provides a visible way to close it because it is click-through.
    pub fn show(&mut self, ctx: &egui::Context, capture_in_progress: bool) {
        if capture_in_progress {
            return;
        }
        let Some(guidance) = self.active.as_ref() else {
            return;
        };
        // Recheck geometry even if the application is otherwise idle, so a
        // display-mode change does not leave stale guidance visible.
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        let (window, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
        let size = match checked_geometry(
            guidance.width,
            guidance.height,
            ctx.pixels_per_point(),
            window,
            monitor,
        ) {
            Ok(size) => size,
            Err(error) => {
                self.active = None;
                self.error = Some(error);
                return;
            }
        };
        let builder = ViewportBuilder::default()
            .with_title("James desktop guidance")
            .with_position(Pos2::ZERO)
            .with_inner_size(size)
            .with_decorations(false)
            .with_transparent(true)
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_always_on_top();
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("james-guidance"),
            builder,
            |ctx, class| {
                if class == ViewportClass::Embedded {
                    return;
                }
                let actual = ctx.input(|i| i.viewport().inner_rect);
                // Do not draw until the native viewport reports its geometry.
                // This avoids misdirecting the user after a DPI or display change.
                if !viewport_matches(actual, size) {
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                    return;
                }
                let painter = ctx.layer_painter(egui::LayerId::background());
                let scale = size.x / guidance.width as f32;
                for stroke in &guidance.strokes {
                    let color = Color32::from_rgba_unmultiplied(
                        stroke.color.rgba()[0],
                        stroke.color.rgba()[1],
                        stroke.color.rgba()[2],
                        255,
                    );
                    let points: Vec<Pos2> = stroke
                        .points
                        .iter()
                        .map(|p| Pos2::new(p.x as f32 * scale, p.y as f32 * scale))
                        .collect();
                    let width = (stroke.width as f32 * scale).max(2.0);
                    if let Some(point) = points.first() {
                        if points.len() == 1 {
                            painter.circle_stroke(*point, 10.0, egui::Stroke::new(3.0_f32, color));
                            painter.circle_filled(*point, 3.0, color);
                        } else {
                            painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
                        }
                    }
                }
                for mark in &guidance.marks {
                    let point = Pos2::new(mark.x as f32 * scale, mark.y as f32 * scale);
                    let color = Color32::from_rgb(62, 193, 178);
                    painter.circle_filled(point, 14.0, color);
                    painter.text(
                        point,
                        egui::Align2::CENTER_CENTER,
                        mark.step.to_string(),
                        egui::FontId::proportional(15.0),
                        Color32::BLACK,
                    );
                    if !mark.label.is_empty() {
                        let galley = painter.layout(
                            mark.label.clone(),
                            egui::FontId::proportional(15.0),
                            Color32::WHITE,
                            (size.x - 16.0).max(1.0),
                        );
                        let box_size = galley.size() + Vec2::new(16.0, 10.0);
                        let x = (point.x + 18.0).min((size.x - box_size.x).max(0.0));
                        let y =
                            (point.y - box_size.y * 0.5).clamp(0.0, (size.y - box_size.y).max(0.0));
                        let rect = Rect::from_min_size(Pos2::new(x, y), box_size);
                        painter.rect_filled(rect, 4.0, Color32::from_black_alpha(225));
                        painter.galley(rect.min + Vec2::new(8.0, 5.0), galley, Color32::WHITE);
                    }
                }
            },
        );
    }
}

fn validated_marks(width: u32, height: u32, marks: &[GuideMark]) -> Result<Vec<GuideMark>, String> {
    if marks.len() > 20 {
        return Err("Desktop guidance supports at most 20 steps.".into());
    }
    marks
        .iter()
        .map(|mark| {
            if !(1..=999).contains(&mark.step)
                || !mark.x.is_finite()
                || !mark.y.is_finite()
                || mark.x < 0.0
                || mark.y < 0.0
                || mark.x >= f64::from(width)
                || mark.y >= f64::from(height)
            {
                return Err("A guidance step is outside the captured display.".into());
            }
            Ok(GuideMark {
                step: mark.step,
                x: mark.x,
                y: mark.y,
                label: mark
                    .label
                    .chars()
                    .map(|c| if c.is_control() { ' ' } else { c })
                    .take(100)
                    .collect(),
            })
        })
        .collect()
}

/// On Windows the primary display begins at physical (0, 0). Egui reports
/// both window and monitor sizes in points, scaled by the same factor.
fn checked_geometry(
    width: u32,
    height: u32,
    scale: f32,
    window: Option<Rect>,
    monitor: Option<Vec2>,
) -> Result<Vec2, String> {
    if width == 0 || height == 0 || !scale.is_finite() || scale <= 0.0 {
        return Err("Cannot determine desktop guidance scaling.".into());
    }
    let size = Vec2::new(width as f32 / scale, height as f32 / scale);
    let monitor = monitor.ok_or("Cannot determine the current display size.")?;
    let window = window.ok_or("Cannot determine the James window position.")?;
    if (size.x - monitor.x).abs() > 2.0 || (size.y - monitor.y).abs() > 2.0 {
        return Err("The display changed since this snapshot. Capture it again.".into());
    }
    let center = window.center();
    if center.x < 0.0 || center.y < 0.0 || center.x >= size.x || center.y >= size.y {
        return Err("Move James to the primary display before showing desktop guidance.".into());
    }
    Ok(size)
}

fn viewport_matches(actual: Option<Rect>, expected: Vec2) -> bool {
    actual.is_some_and(|rect| {
        rect.min.x.abs() <= 2.0
            && rect.min.y.abs() <= 2.0
            && (rect.width() - expected.x).abs() <= 2.0
            && (rect.height() - expected.y).abs() <= 2.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_geometry_handles_scaling_and_rejects_other_monitors() {
        let window = Some(Rect::from_min_size(
            Pos2::new(100.0, 100.0),
            Vec2::new(500.0, 400.0),
        ));
        let monitor = Some(Vec2::new(1280.0, 720.0));
        assert_eq!(
            checked_geometry(1920, 1080, 1.5, window, monitor),
            Ok(Vec2::new(1280.0, 720.0))
        );
        let other = Some(Rect::from_min_size(
            Pos2::new(-600.0, 0.0),
            Vec2::new(500.0, 400.0),
        ));
        assert!(checked_geometry(1920, 1080, 1.5, other, monitor).is_err());
        assert!(checked_geometry(1920, 1080, 1.5, window, Some(Vec2::new(1200.0, 720.0))).is_err());
    }

    #[test]
    fn viewport_must_cover_captured_display_before_marks_appear() {
        let expected = Vec2::new(1280.0, 720.0);
        assert!(!viewport_matches(None, expected));
        assert!(viewport_matches(
            Some(Rect::from_min_size(Pos2::ZERO, expected)),
            expected
        ));
        assert!(!viewport_matches(
            Some(Rect::from_min_size(Pos2::new(20.0, 0.0), expected)),
            expected
        ));
    }

    #[test]
    fn steps_are_bounded_to_capture_and_labels_are_sanitized() {
        let mark = GuideMark {
            step: 3,
            x: 9.0,
            y: 19.0,
            label: "Open\nmenu".into(),
        };
        assert_eq!(
            validated_marks(10, 20, std::slice::from_ref(&mark)).unwrap()[0].label,
            "Open menu"
        );
        assert!(validated_marks(9, 20, &[mark]).is_err());
        assert!(
            validated_marks(
                10,
                20,
                &[GuideMark {
                    step: 1,
                    x: f64::NAN,
                    y: 0.0,
                    label: String::new()
                }]
            )
            .is_err()
        );
        assert!(
            validated_marks(
                10,
                20,
                &[GuideMark {
                    step: 0,
                    x: 1.0,
                    y: 1.0,
                    label: String::new(),
                }]
            )
            .is_err()
        );
    }
}
