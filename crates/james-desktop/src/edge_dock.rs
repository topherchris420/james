//! Monitor-aware placement calculations for a draggable companion window.
//!
//! Native monitor bounds must be supplied in physical desktop pixels. Egui's
//! `ViewportInfo::monitor_size` has no origin and cannot identify an edge on
//! secondary displays. Callers must obtain fresh bounds from the OS and use the
//! current viewport's pixels-per-point when converting its position.

use eframe::egui::{Pos2, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Monitor {
    /// Bounds in physical desktop pixels; monitor origins may be negative.
    pub bounds: Rect,
}

/// Returns a position in egui points for `OuterPosition`, snapping the current
/// viewport against the chosen edge of the monitor it overlaps most. Returns
/// `None` when geometry is unavailable or the window does not overlap any
/// supplied monitor, instead of guessing and risking a lost window.
pub fn dock_position(
    window_points: Rect,
    pixels_per_point: f32,
    viewport_monitor_size: Option<Vec2>,
    monitors: &[Monitor],
    edge: Edge,
) -> Option<Pos2> {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 || !valid_rect(window_points) {
        return None;
    }
    let window = Rect::from_min_max(
        Pos2::new(
            window_points.min.x * pixels_per_point,
            window_points.min.y * pixels_per_point,
        ),
        Pos2::new(
            window_points.max.x * pixels_per_point,
            window_points.max.y * pixels_per_point,
        ),
    );
    if !valid_rect(window) {
        return None;
    }
    let monitor = monitors
        .iter()
        .filter(|monitor| valid_rect(monitor.bounds))
        .filter_map(|monitor| {
            let intersection = monitor.bounds.intersect(window);
            let area = intersection.width().max(0.0) * intersection.height().max(0.0);
            (area > 0.0 && area.is_finite()).then_some((monitor.bounds, area))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))?
        .0;
    let current_size = viewport_monitor_size? * pixels_per_point;
    if !current_size.x.is_finite()
        || !current_size.y.is_finite()
        || (current_size.x - monitor.width()).abs() > 2.0
        || (current_size.y - monitor.height()).abs() > 2.0
        || window.width() > monitor.width()
        || window.height() > monitor.height()
        || !monitor.contains(window.center())
    {
        return None;
    }
    let x = match edge {
        Edge::Left => monitor.left(),
        Edge::Right => monitor.right() - window.width(),
    };
    let y = window
        .top()
        .clamp(monitor.top(), monitor.bottom() - window.height());
    Some(Pos2::new(x / pixels_per_point, y / pixels_per_point))
}

fn valid_rect(rect: Rect) -> bool {
    rect.min.x.is_finite()
        && rect.min.y.is_finite()
        && rect.max.x.is_finite()
        && rect.max.y.is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height))
    }

    #[test]
    fn snaps_with_negative_origin_and_scaling() {
        let monitors = [
            Monitor {
                bounds: rect(-2560.0, 0.0, 2560.0, 1440.0),
            },
            Monitor {
                bounds: rect(0.0, 0.0, 1920.0, 1080.0),
            },
        ];
        let window = rect(-1000.0, 200.0, 200.0, 300.0);
        let size = Some(Vec2::new(2048.0, 1152.0));
        assert_eq!(
            dock_position(window, 1.25, size, &monitors, Edge::Left),
            Some(Pos2::new(-2048.0, 200.0))
        );
        assert_eq!(
            dock_position(window, 1.25, size, &monitors, Edge::Right),
            Some(Pos2::new(-200.0, 200.0))
        );
    }

    #[test]
    fn chooses_monitor_with_largest_physical_overlap_and_clamps_vertically() {
        let monitors = [
            Monitor {
                bounds: rect(0.0, 0.0, 1000.0, 800.0),
            },
            Monitor {
                bounds: rect(1000.0, -200.0, 1000.0, 1000.0),
            },
        ];
        let window = rect(950.0, 650.0, 200.0, 200.0);
        assert_eq!(
            dock_position(
                window,
                1.0,
                Some(Vec2::splat(1000.0)),
                &monitors,
                Edge::Right
            ),
            Some(Pos2::new(1800.0, 600.0))
        );
    }

    #[test]
    fn refuses_missing_invalid_or_incompatible_geometry() {
        let window = rect(20.0, 20.0, 200.0, 100.0);
        let size = Some(Vec2::new(1000.0, 800.0));
        assert_eq!(dock_position(window, 1.0, size, &[], Edge::Left), None);
        assert_eq!(
            dock_position(
                window,
                0.0,
                size,
                &[Monitor {
                    bounds: rect(0.0, 0.0, 1000.0, 800.0)
                }],
                Edge::Left
            ),
            None
        );
        assert_eq!(
            dock_position(
                window,
                1.0,
                size,
                &[Monitor {
                    bounds: rect(1000.0, 0.0, 1000.0, 800.0)
                }],
                Edge::Left
            ),
            None
        );
        assert_eq!(
            dock_position(
                window,
                1.0,
                size,
                &[Monitor {
                    bounds: rect(0.0, 0.0, 100.0, 100.0)
                }],
                Edge::Left
            ),
            None
        );
        assert_eq!(
            dock_position(
                window,
                1.0,
                None,
                &[Monitor {
                    bounds: rect(0.0, 0.0, 1000.0, 800.0)
                }],
                Edge::Left
            ),
            None
        );
        assert_eq!(
            dock_position(
                window,
                1.0,
                Some(Vec2::splat(500.0)),
                &[Monitor {
                    bounds: rect(0.0, 0.0, 1000.0, 800.0)
                }],
                Edge::Left
            ),
            None
        );
    }
}
