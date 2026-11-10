//! User-provided visual context, expressed in source-image pixel coordinates.
//!
//! These marks are an independent James interaction. They are composited into
//! the captured PNG before any crop or model image resizing takes place.

use image::{ImageFormat, ImageReader, Rgba, RgbaImage};
use std::io::Cursor;

pub const MAX_STROKES: usize = 32;
pub const MAX_POINTS: usize = 2048;
pub const MAX_BRUSH_WIDTH: f64 = 64.0;
const MAX_IMAGE_EDGE: u32 = 32_768;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrushColor {
    #[default]
    Accent,
    Blue,
    Green,
    Amber,
    Red,
    White,
}

impl BrushColor {
    /// The same opaque colour can be used for the on-screen preview.
    pub const fn rgba(self) -> [u8; 4] {
        match self {
            Self::Accent => [62, 193, 178, 255],
            Self::Blue => [77, 163, 255, 255],
            Self::Green => [90, 210, 115, 255],
            Self::Amber => [242, 193, 78, 255],
            Self::Red => [255, 100, 100, 255],
            Self::White => [255, 255, 255, 255],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    /// One point paints a dot; additional points join with round ends.
    pub points: Vec<Point>,
    pub color: BrushColor,
    /// Diameter in source-image pixels, in the inclusive range 1..=64.
    pub width: f64,
}

/// Composite bounded, opaque annotation strokes without changing PNG dimensions.
///
/// Points outside the image are clipped; a stroke which crosses the image is
/// still drawn. Invalid stroke data and invalid/oversized PNGs return an error.
pub fn annotate_png(png: &[u8], strokes: &[Stroke]) -> Result<Vec<u8>, String> {
    validate_strokes(strokes)?;
    let mut reader = ImageReader::with_format(Cursor::new(png), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_EDGE);
    limits.max_image_height = Some(MAX_IMAGE_EDGE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let mut image = reader
        .decode()
        .map_err(|error| format!("Cannot read screen capture PNG: {error}"))?
        .into_rgba8();

    for stroke in strokes {
        let ink = Rgba(stroke.color.rgba());
        let radius = stroke.width / 2.0;
        if stroke.points.len() == 1 {
            paint_segment(&mut image, stroke.points[0], stroke.points[0], radius, ink);
        } else {
            for pair in stroke.points.windows(2) {
                paint_segment(&mut image, pair[0], pair[1], radius, ink);
            }
        }
    }

    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| format!("Cannot encode annotated screen capture: {error}"))?;
    Ok(output.into_inner())
}

fn validate_strokes(strokes: &[Stroke]) -> Result<(), String> {
    if strokes.len() > MAX_STROKES {
        return Err(format!(
            "Use at most {MAX_STROKES} screen annotation strokes."
        ));
    }
    let mut points = 0usize;
    for stroke in strokes {
        points = points.saturating_add(stroke.points.len());
        if points > MAX_POINTS {
            return Err(format!(
                "Use at most {MAX_POINTS} screen annotation points."
            ));
        }
        if stroke.points.is_empty() {
            return Err("A screen annotation stroke needs at least one point.".to_owned());
        }
        if !stroke.width.is_finite() || !(1.0..=MAX_BRUSH_WIDTH).contains(&stroke.width) {
            return Err("Screen annotation width must be between 1 and 64 pixels.".to_owned());
        }
        // A generous coordinate range avoids overflow in distance calculations
        // while allowing gestures which start or finish far outside a capture.
        if stroke.points.iter().any(|point| {
            !point.x.is_finite()
                || !point.y.is_finite()
                || point.x.abs() > 1e9
                || point.y.abs() > 1e9
        }) {
            return Err("Screen annotation coordinates are invalid.".to_owned());
        }
    }
    Ok(())
}

fn paint_segment(image: &mut RgbaImage, a: Point, b: Point, radius: f64, ink: Rgba<u8>) {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return;
    }
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length_squared = dx * dx + dy * dy;
    let top = (a.y.min(b.y) - radius).ceil().max(0.0);
    let bottom = (a.y.max(b.y) + radius).floor().min(f64::from(height - 1));
    if top > bottom {
        return;
    }
    let radius_squared = radius * radius;
    for y in top as u32..=bottom as u32 {
        // Only inspect the portion of this row near the segment. This avoids
        // scanning the entire bounding rectangle of a long diagonal stroke.
        let (low, high) = if dy.abs() > f64::EPSILON {
            let first = (f64::from(y) - radius - a.y) / dy;
            let second = (f64::from(y) + radius - a.y) / dy;
            let start = first.min(second).max(0.0);
            let end = first.max(second).min(1.0);
            if start > end {
                continue;
            }
            let first_x = a.x + dx * start;
            let last_x = a.x + dx * end;
            (first_x.min(last_x) - radius, first_x.max(last_x) + radius)
        } else {
            (a.x.min(b.x) - radius, a.x.max(b.x) + radius)
        };
        let left = low.ceil().max(0.0);
        let right = high.floor().min(f64::from(width - 1));
        if left > right {
            continue;
        }
        for x in left as u32..=right as u32 {
            let px = f64::from(x) - a.x;
            let py = f64::from(y) - a.y;
            let t = if length_squared > 0.0 {
                ((px * dx + py * dy) / length_squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let distance_x = px - t * dx;
            let distance_y = py - t * dy;
            if distance_x * distance_x + distance_y * distance_y <= radius_squared {
                image.put_pixel(x, y, ink);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_png(width: u32, height: u32) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        RgbaImage::from_pixel(width, height, Rgba([10, 20, 30, 255]))
            .write_to(&mut output, ImageFormat::Png)
            .expect("test PNG should encode");
        output.into_inner()
    }

    fn stroke(points: &[(f64, f64)]) -> Stroke {
        Stroke {
            points: points.iter().map(|&(x, y)| Point { x, y }).collect(),
            color: BrushColor::Accent,
            width: 4.0,
        }
    }

    fn decoded(png: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(png, ImageFormat::Png)
            .expect("annotated PNG should decode")
            .into_rgba8()
    }

    #[test]
    fn paints_dot_and_round_connected_stroke_without_resizing() {
        let png = blank_png(30, 20);
        let result = annotate_png(
            &png,
            &[
                stroke(&[(5.0, 5.0)]),
                Stroke {
                    color: BrushColor::Red,
                    ..stroke(&[(12.0, 5.0), (20.0, 5.0), (20.0, 12.0)])
                },
            ],
        )
        .expect("valid marks should paint");
        let image = decoded(&result);
        assert_eq!(image.dimensions(), (30, 20));
        assert_eq!(image.get_pixel(5, 5).0, BrushColor::Accent.rgba());
        assert_eq!(image.get_pixel(7, 5).0, BrushColor::Accent.rgba());
        assert_eq!(image.get_pixel(5, 8).0, [10, 20, 30, 255]);
        assert_eq!(image.get_pixel(16, 5).0, BrushColor::Red.rgba());
        assert_eq!(image.get_pixel(20, 9).0, BrushColor::Red.rgba());
        assert_eq!(image.get_pixel(0, 19).0, [10, 20, 30, 255]);
    }

    #[test]
    fn clips_outside_points_but_keeps_crossing_segments() {
        let png = blank_png(16, 12);
        let result = annotate_png(
            &png,
            &[
                stroke(&[(-100.0, 4.0), (100.0, 4.0)]),
                stroke(&[(-100.0, -100.0)]),
                stroke(&[(100.0, 100.0)]),
            ],
        )
        .expect("off-image marks should clip");
        let image = decoded(&result);
        assert_eq!(image.dimensions(), (16, 12));
        assert_eq!(image.get_pixel(0, 4).0, BrushColor::Accent.rgba());
        assert_eq!(image.get_pixel(15, 4).0, BrushColor::Accent.rgba());
        assert_eq!(image.get_pixel(0, 11).0, [10, 20, 30, 255]);
    }

    #[test]
    fn diagonal_strokes_cover_the_line_in_either_direction() {
        let png = blank_png(16, 16);
        for points in [
            [(-100.0, -100.0), (100.0, 100.0)],
            [(100.0, 100.0), (-100.0, -100.0)],
        ] {
            let image = decoded(&annotate_png(&png, &[stroke(&points)]).unwrap());
            for index in 0..16 {
                assert_eq!(image.get_pixel(index, index).0, BrushColor::Accent.rgba());
            }
            assert_eq!(image.get_pixel(0, 15).0, [10, 20, 30, 255]);
        }
    }

    #[test]
    fn no_marks_preserve_original_pixels() {
        let png = blank_png(13, 7);
        assert_eq!(decoded(&annotate_png(&png, &[]).unwrap()), decoded(&png));
    }

    #[test]
    fn rejects_malformed_png_and_invalid_stroke_values() {
        assert!(annotate_png(b"not a PNG", &[]).is_err());
        let png = blank_png(8, 8);
        for width in [0.0, -1.0, 65.0, f64::NAN, f64::INFINITY] {
            let mut mark = stroke(&[(1.0, 1.0)]);
            mark.width = width;
            assert!(annotate_png(&png, &[mark]).is_err());
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e20] {
            assert!(annotate_png(&png, &[stroke(&[(value, 1.0)])]).is_err());
            assert!(annotate_png(&png, &[stroke(&[(1.0, value)])]).is_err());
        }
        assert!(annotate_png(&png, &[stroke(&[])]).is_err());
    }

    #[test]
    fn enforces_stroke_and_total_point_limits() {
        let png = blank_png(8, 8);
        let mark = stroke(&[(1.0, 1.0)]);
        assert!(annotate_png(&png, &vec![mark.clone(); MAX_STROKES]).is_ok());
        assert!(annotate_png(&png, &vec![mark; MAX_STROKES + 1]).is_err());

        let mark = stroke(&vec![(1.0, 1.0); MAX_POINTS]);
        assert!(annotate_png(&png, std::slice::from_ref(&mark)).is_ok());
        assert!(annotate_png(&png, &[mark, stroke(&[(2.0, 2.0)])]).is_err());
    }
}
