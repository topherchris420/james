//! James's look: Vers3Dynamics teal on a quiet dark or light background, a
//! high-contrast variant of each, adjustable text size and reduced motion.
//! Text colours meet WCAG AA contrast (4.5:1) against their backgrounds; the
//! high-contrast themes aim for AAA (7:1).
use crate::session::{Preferences, ThemeChoice};
use eframe::egui::{
    self, Color32, FontFamily, FontId, Stroke, TextStyle, Visuals, style::ScrollAnimation,
};

/// The brand teal, used on dark backgrounds.
pub const TEAL: Color32 = Color32::from_rgb(62, 193, 178);
/// A deeper teal that keeps 6:1 contrast on white.
const TEAL_DEEP: Color32 = Color32::from_rgb(11, 110, 102);

struct Palette {
    dark: bool,
    panel: Color32,
    window: Color32,
    field: Color32,
    faint: Color32,
    text: Color32,
    accent: Color32,
    on_accent: Color32,
    warn: Color32,
    error: Color32,
    stroke: Color32,
    stroke_width: f32,
}

fn palette(dark: bool, high_contrast: bool) -> Palette {
    match (dark, high_contrast) {
        (true, false) => Palette {
            dark,
            panel: Color32::from_rgb(16, 23, 25),
            window: Color32::from_rgb(20, 28, 31),
            field: Color32::from_rgb(11, 16, 18),
            faint: Color32::from_rgb(25, 34, 38),
            text: Color32::from_rgb(227, 236, 236),
            accent: TEAL,
            on_accent: Color32::from_rgb(4, 20, 20),
            warn: Color32::from_rgb(242, 193, 78),
            error: Color32::from_rgb(255, 122, 122),
            stroke: Color32::from_rgb(52, 66, 70),
            stroke_width: 1.0,
        },
        (false, false) => Palette {
            dark,
            panel: Color32::from_rgb(244, 247, 247),
            window: Color32::WHITE,
            field: Color32::WHITE,
            faint: Color32::from_rgb(233, 239, 239),
            text: Color32::from_rgb(26, 37, 38),
            accent: TEAL_DEEP,
            on_accent: Color32::WHITE,
            warn: Color32::from_rgb(138, 83, 0),
            error: Color32::from_rgb(180, 35, 24),
            stroke: Color32::from_rgb(190, 204, 204),
            stroke_width: 1.0,
        },
        (true, true) => Palette {
            dark,
            panel: Color32::BLACK,
            window: Color32::BLACK,
            field: Color32::BLACK,
            faint: Color32::from_rgb(20, 20, 20),
            text: Color32::WHITE,
            accent: Color32::from_rgb(255, 212, 0),
            on_accent: Color32::BLACK,
            warn: Color32::from_rgb(255, 212, 0),
            error: Color32::from_rgb(255, 140, 140),
            stroke: Color32::WHITE,
            stroke_width: 2.0,
        },
        (false, true) => Palette {
            dark,
            panel: Color32::WHITE,
            window: Color32::WHITE,
            field: Color32::WHITE,
            faint: Color32::from_rgb(235, 235, 235),
            text: Color32::BLACK,
            accent: Color32::from_rgb(0, 51, 170),
            on_accent: Color32::WHITE,
            warn: Color32::from_rgb(110, 60, 0),
            error: Color32::from_rgb(150, 0, 0),
            stroke: Color32::BLACK,
            stroke_width: 2.0,
        },
    }
}

fn visuals(dark: bool, high_contrast: bool) -> Visuals {
    let p = palette(dark, high_contrast);
    let mut v = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = p.panel;
    v.window_fill = p.window;
    v.extreme_bg_color = p.field;
    v.faint_bg_color = p.faint;
    v.code_bg_color = p.faint;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.error;
    v.override_text_color = high_contrast.then_some(p.text);
    v.selection.bg_fill = if high_contrast {
        p.accent
    } else {
        p.accent.gamma_multiply(0.45)
    };
    // Also the outline of a focused text box: thick enough to find at a glance.
    v.selection.stroke = Stroke::new(
        p.stroke_width + 1.0,
        if high_contrast { p.on_accent } else { p.text },
    );
    v.window_stroke = Stroke::new(p.stroke_width, p.stroke);
    for (widget, bg) in [
        (&mut v.widgets.noninteractive, p.panel),
        (&mut v.widgets.inactive, p.faint),
        (&mut v.widgets.hovered, p.faint),
        (&mut v.widgets.active, p.faint),
        (&mut v.widgets.open, p.faint),
    ] {
        widget.fg_stroke.color = p.text;
        widget.bg_fill = bg;
        widget.weak_bg_fill = bg;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(p.stroke_width, p.stroke);
    v.widgets.inactive.bg_stroke = Stroke::new(p.stroke_width, p.stroke);
    // A clearly visible ring shows where keyboard focus and the pointer are.
    v.widgets.hovered.bg_stroke = Stroke::new(p.stroke_width + 1.0, p.accent);
    v.widgets.active.bg_stroke = Stroke::new(p.stroke_width + 1.0, p.accent);
    v
}

/// Applies the person's display preferences. Call when they change.
pub fn apply(ctx: &egui::Context, prefs: &Preferences) {
    ctx.set_visuals_of(egui::Theme::Dark, visuals(true, prefs.high_contrast));
    ctx.set_visuals_of(egui::Theme::Light, visuals(false, prefs.high_contrast));
    ctx.set_theme(match prefs.theme {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Light => egui::ThemePreference::Light,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
    });
    let reduce = prefs.reduce_motion;
    ctx.all_styles_mut(|style| {
        style.animation_time = if reduce { 0.0 } else { 1.0 / 12.0 };
        style.scroll_animation = if reduce {
            ScrollAnimation::none()
        } else {
            ScrollAnimation::default()
        };
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        // Larger targets are easier to hit with a mouse, touchpad or touch.
        style.spacing.interact_size.y = 26.0;
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(22.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(14.0, FontFamily::Monospace),
            ),
        ]
        .into();
    });
    ctx.set_zoom_factor(prefs.text_scale());
}

/// The accent colour of the active theme.
pub fn accent(ui: &egui::Ui) -> Color32 {
    ui.visuals().hyperlink_color
}

/// Text that sits on an accent-coloured button.
pub fn on_accent(ui: &egui::Ui) -> Color32 {
    let accent = accent(ui);
    let [r, g, b, _] = accent.to_array();
    // Relative brightness decides between near-black and white text.
    if 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b) > 140.0 {
        Color32::from_rgb(4, 20, 20)
    } else {
        Color32::WHITE
    }
}

/// The one button on a screen that moves the person forward.
pub fn primary(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text.to_owned())
            .strong()
            .color(on_accent(ui)),
    )
    .fill(accent(ui))
}

/// A button for stopping or deleting: clearly marked, never the default.
pub fn danger(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text.to_owned())
            .strong()
            .color(ui.visuals().error_fg_color),
    )
    .stroke(Stroke::new(1.5_f32, ui.visuals().error_fg_color))
}

/// Shows that work is in progress: a spinner, or plain text when motion is reduced.
pub fn busy(ui: &mut egui::Ui, reduce_motion: bool) {
    if reduce_motion {
        ui.label("…");
    } else {
        ui.spinner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG relative luminance.
    fn luminance(c: Color32) -> f32 {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        let [r, g, b, _] = c.to_array();
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn every_theme_keeps_text_readable() {
        for dark in [true, false] {
            for high in [false, true] {
                let p = palette(dark, high);
                let floor = if high { 7.0 } else { 4.5 };
                for (name, fg, bg) in [
                    ("text on panel", p.text, p.panel),
                    ("text on field", p.text, p.field),
                    ("accent on panel", p.accent, p.panel),
                    ("warning on panel", p.warn, p.panel),
                    ("error on panel", p.error, p.panel),
                    ("button text on accent", p.on_accent, p.accent),
                ] {
                    let ratio = contrast(fg, bg);
                    assert!(
                        ratio >= floor,
                        "{name} is {ratio:.2}:1 (dark {dark}, high contrast {high})"
                    );
                }
            }
        }
    }
}
