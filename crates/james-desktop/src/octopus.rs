//! The floating companion is a set of studio-rendered poses from one sculpture.
//! Only a greeting advances the artwork. An open, resting companion can sleep.
use eframe::egui::{
    self, Align2, Color32, Rect, Sense, StrokeKind, TextureHandle, TextureId, pos2, vec2,
};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mood {
    Rest,
    Watch,
    Bell,
}

impl Mood {
    pub fn from_remaining(remaining: Option<Duration>) -> Self {
        match remaining {
            None => Self::Rest,
            Some(left) if left.is_zero() => Self::Bell,
            Some(_) => Self::Watch,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Rest => "Octopus is resting",
            Self::Watch => "Octopus is watching your timer",
            Self::Bell => "Octopus is ringing the bell",
        }
    }

    fn sheet(self) -> &'static [u8] {
        match self {
            Self::Rest => include_bytes!("../assets/octopus/rest.webp"),
            Self::Watch => include_bytes!("../assets/octopus/watch.webp"),
            Self::Bell => include_bytes!("../assets/octopus/bell.webp"),
        }
    }
}

const COLUMNS: usize = 4;
const ROWS: usize = 3;
const LAST_FRAME: usize = COLUMNS * ROWS - 1;
const WAVE_SECONDS: f64 = 1.8;

/// Each sheet goes from the grounded pose to a fully lifted arm. Playing part
/// of that arc backwards at the top gives the tip a little answering wave.
fn wave_frame(elapsed: f64, still: bool) -> usize {
    if still {
        return LAST_FRAME;
    }
    let beats = [
        (0.0, 0.0),
        (0.62, 1.0),
        (0.88, 0.70),
        (1.15, 1.0),
        (WAVE_SECONDS, 0.0),
    ];
    for pair in beats.windows(2) {
        let (start, from) = pair[0];
        let (end, to) = pair[1];
        if elapsed <= end {
            let t = ((elapsed - start) / (end - start)).clamp(0.0, 1.0);
            let eased = t * t * (3.0 - 2.0 * t);
            return ((from + (to - from) * eased) * LAST_FRAME as f64).round() as usize;
        }
    }
    0
}

/// A broad, feathered pool of reflected light grounds the transparent sculpture
/// on both the warm paper and graphite backgrounds.
fn studio_light(painter: &egui::Painter, rect: Rect, paper: Color32, light: Color32) {
    const SIDES: u32 = 40;
    let center = rect.min + rect.size() * vec2(0.47, 0.65);
    let radius = rect.size() * vec2(0.46, 0.33);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, light);
    for (ring, (scale, color)) in [(0.48, light.lerp_to_gamma(paper, 0.34)), (1.0, paper)]
        .into_iter()
        .enumerate()
    {
        for i in 0..SIDES {
            let a = std::f32::consts::TAU * i as f32 / SIDES as f32;
            mesh.colored_vertex(center + vec2(a.cos(), a.sin()) * radius * scale, color);
            let here = 1 + i;
            let next = 1 + (i + 1) % SIDES;
            if ring == 0 {
                mesh.add_triangle(0, here, next);
            } else {
                mesh.add_triangle(here, here + SIDES, next);
                mesh.add_triangle(next, here + SIDES, next + SIDES);
            }
        }
    }
    painter.add(mesh);
}

const STILLS: &[u8] = include_bytes!("../assets/octopus/stills.webp");
const POSE_SIZE: egui::Vec2 = vec2(448.0, 304.0);

fn load_sheet(ctx: &egui::Context, bytes: &[u8]) -> TextureHandle {
    let pixels = image::load_from_memory_with_format(bytes, image::ImageFormat::WebP)
        .expect("validated bundled octopus artwork")
        .into_rgba8();
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [pixels.width() as usize, pixels.height() as usize],
        pixels.as_raw(),
    );
    ctx.load_texture("james-octopus", image, egui::TextureOptions::LINEAR)
}

fn cell_uv(column: usize, row: usize, columns: usize, rows: usize) -> Rect {
    Rect::from_min_max(
        pos2(column as f32 / columns as f32, row as f32 / rows as f32),
        pos2(
            (column + 1) as f32 / columns as f32,
            (row + 1) as f32 / rows as f32,
        ),
    )
}

#[derive(Default)]
pub struct Octopus {
    stills: Option<TextureHandle>,
    wave_sheet: Option<(Mood, TextureHandle)>,
    loading: Option<(Mood, Receiver<TextureHandle>)>,
    wave_started: Option<(Mood, f64)>,
}

impl Octopus {
    fn greet(&mut self, ctx: &egui::Context, mood: Mood, still: bool, now: f64) {
        if still
            || self
                .wave_sheet
                .as_ref()
                .is_some_and(|(cached, _)| *cached == mood)
        {
            self.loading = None;
            self.wave_started = Some((mood, now));
            return;
        }
        if self
            .loading
            .as_ref()
            .is_some_and(|(pending, _)| *pending == mood)
        {
            return;
        }
        // A first greeting may need a full sheet. Keep the timer responsive
        // and start the gesture's clock only after the artwork is ready.
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        let viewport = ctx.viewport_id();
        self.loading = Some((mood, receiver));
        self.wave_started = None;
        std::thread::spawn(move || {
            if sender.send(load_sheet(&ctx, mood.sheet())).is_ok() {
                ctx.request_repaint_of(viewport);
            }
        });
    }

    fn finish_loading(&mut self, mood: Mood, now: f64) {
        let Some((requested, receiver)) = &self.loading else {
            return;
        };
        if *requested == mood {
            match receiver.try_recv() {
                Ok(texture) => {
                    self.wave_sheet = Some((mood, texture));
                    self.wave_started = Some((mood, now));
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => (),
            }
        }
        self.loading = None;
    }

    fn sprite(
        &mut self,
        ctx: &egui::Context,
        mood: Mood,
        frame: usize,
        animated: bool,
    ) -> (TextureId, Rect) {
        if animated
            && let Some((cached, sheet)) = &self.wave_sheet
            && *cached == mood
        {
            return (
                sheet.id(),
                cell_uv(frame % COLUMNS, frame / COLUMNS, COLUMNS, ROWS),
            );
        }
        let sheet = self.stills.get_or_insert_with(|| load_sheet(ctx, STILLS));
        // Still columns follow the Rest, Watch, Bell declaration order.
        (
            sheet.id(),
            cell_uv(mood as usize, usize::from(frame == LAST_FRAME), 3, 2),
        )
    }

    /// The entire portrait is one keyboard, pointer and accessibility target.
    pub fn portrait(&mut self, ui: &mut egui::Ui, mood: Mood, reduce_motion: bool) {
        let width = ui.available_width();
        let height = (width * 0.59).min(180.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let now = ui.input(|i| i.time);
        if response.clicked() {
            self.greet(ui.ctx(), mood, reduce_motion, now);
        }
        self.finish_loading(mood, now);
        let elapsed = self
            .wave_started
            .filter(|(started_mood, _)| *started_mood == mood)
            .map(|(_, start)| now - start)
            .filter(|t| *t < WAVE_SECONDS);
        let greeting = elapsed.is_some() || self.loading.is_some();
        response.widget_info(|| {
            let label = if greeting {
                "Octopus waves back. Wave again"
            } else {
                "Wave hello"
            };
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
        });
        if !ui.is_rect_visible(rect) {
            return;
        }
        let frame = if let Some(elapsed) = elapsed {
            let remaining = WAVE_SECONDS - elapsed;
            let delay = if reduce_motion {
                remaining
            } else {
                remaining.min(1.0 / 24.0)
            };
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(delay));
            wave_frame(elapsed, reduce_motion)
        } else {
            self.wave_started = None;
            0
        };

        let painter = ui.painter_at(rect);
        let visuals = ui.visuals();
        let paper = if visuals.dark_mode {
            Color32::from_rgb(39, 38, 48)
        } else {
            Color32::from_rgb(242, 238, 233)
        };
        let border = if response.has_focus() || response.hovered() {
            visuals.widgets.hovered.bg_stroke
        } else {
            visuals.widgets.noninteractive.bg_stroke
        };
        painter.rect(rect, 9, paper, border, StrokeKind::Inside);
        let light = if visuals.dark_mode {
            Color32::from_rgb(73, 67, 82)
        } else {
            Color32::from_rgb(255, 252, 244)
        };
        studio_light(&painter, rect, paper, light);
        // Keep the plate's aspect ratio at every window width and text scale.
        let (texture, uv) = self.sprite(ui.ctx(), mood, frame, elapsed.is_some() && !reduce_motion);
        let mut inside = rect.shrink2(vec2(5.0, 6.0));
        if width < 230.0 {
            inside.max.y -= 14.0;
        }
        let scale = (inside.width() / POSE_SIZE.x)
            .min(inside.height() / POSE_SIZE.y)
            .max(0.0);
        let artwork = Rect::from_center_size(inside.center(), POSE_SIZE * scale);
        painter.image(texture, artwork, uv, Color32::WHITE);
        let caption = if greeting { "Hello!" } else { "Wave hello" };
        let font = egui::TextStyle::Small.resolve(ui.style());
        let ink = visuals.strong_text_color();
        let galley = painter.layout_no_wrap(caption.into(), font, ink);
        let tag = Align2::RIGHT_BOTTOM.anchor_size(
            rect.right_bottom() - vec2(8.0, 7.0),
            galley.size() + vec2(12.0, 6.0),
        );
        painter.galley(tag.min + vec2(6.0, 3.0), galley, ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_timer_state_has_a_complete_sheet() {
        let stills = image::load_from_memory(STILLS).unwrap();
        assert_eq!((stills.width(), stills.height()), (448 * 3, 304 * 2));
        for mood in [Mood::Rest, Mood::Watch, Mood::Bell] {
            let image = image::load_from_memory(mood.sheet()).unwrap();
            assert_eq!(
                (image.width(), image.height()),
                ((448 * COLUMNS) as u32, (304 * ROWS) as u32)
            );
        }
    }

    #[test]
    fn a_cold_wave_starts_when_its_sheet_arrives_and_tracks_the_current_timer() {
        let (send, receive) = mpsc::channel();
        let mut pet = Octopus {
            loading: Some((Mood::Rest, receive)),
            ..Octopus::default()
        };
        pet.finish_loading(Mood::Rest, 1.0);
        assert!(pet.wave_started.is_none());
        let ctx = egui::Context::default();
        let pixel = egui::ColorImage::new([1, 1], Color32::WHITE);
        let texture = ctx.load_texture("test pose", pixel, egui::TextureOptions::LINEAR);
        send.send(texture.clone()).unwrap();
        pet.finish_loading(Mood::Rest, 8.0);
        assert_eq!(pet.wave_started, Some((Mood::Rest, 8.0)));

        let (send, receive) = mpsc::channel();
        pet.loading = Some((Mood::Watch, receive));
        pet.wave_started = None;
        send.send(texture).unwrap();
        pet.finish_loading(Mood::Bell, 10.0);
        assert!(pet.loading.is_none());
        assert!(pet.wave_started.is_none());
    }

    #[test]
    fn the_wave_returns_to_rest_and_reduced_motion_holds_one_pose() {
        assert_eq!(wave_frame(0.0, false), 0);
        assert_eq!(wave_frame(0.62, false), LAST_FRAME);
        assert!(wave_frame(0.88, false) < LAST_FRAME);
        assert_eq!(wave_frame(WAVE_SECONDS, false), 0);
        for t in [0.0, 0.62, 1.4] {
            assert_eq!(wave_frame(t, true), LAST_FRAME);
        }
    }
}
