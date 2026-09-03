//! Primitive chrome painting (Vello Scene).

use vello::kurbo::{Affine, BezPath, Point, Rect, RoundedRect, Stroke as KStroke};
use vello::peniko::{Color, Fill};
use vello::Scene;

pub fn fill_rect(s: &mut Scene, r: Rect, c: Color) {
    let mut p = BezPath::new();
    p.move_to((r.x0, r.y0));
    p.line_to((r.x1, r.y0));
    p.line_to((r.x1, r.y1));
    p.line_to((r.x0, r.y1));
    p.close_path();
    s.fill(Fill::NonZero, Affine::IDENTITY, c, None, &p);
}

pub fn fill_rrect(s: &mut Scene, r: Rect, radius: f64, c: Color) {
    let rr = RoundedRect::from_rect(r, radius);
    s.fill(Fill::NonZero, Affine::IDENTITY, c, None, &rr);
}

pub fn stroke_rect(s: &mut Scene, r: Rect, c: Color, w: f64) {
    let mut p = BezPath::new();
    p.move_to((r.x0 + 0.5, r.y0 + 0.5));
    p.line_to((r.x1 - 0.5, r.y0 + 0.5));
    p.line_to((r.x1 - 0.5, r.y1 - 0.5));
    p.line_to((r.x0 + 0.5, r.y1 - 0.5));
    p.close_path();
    s.stroke(&KStroke::new(w), Affine::IDENTITY, c, None, &p);
}

pub fn hline(s: &mut Scene, x0: f64, x1: f64, y: f64, c: Color) {
    let mut p = BezPath::new();
    p.move_to((x0, y + 0.5));
    p.line_to((x1, y + 0.5));
    s.stroke(&KStroke::new(1.0), Affine::IDENTITY, c, None, &p);
}

pub fn vline(s: &mut Scene, x: f64, y0: f64, y1: f64, c: Color) {
    let mut p = BezPath::new();
    p.move_to((x + 0.5, y0));
    p.line_to((x + 0.5, y1));
    s.stroke(&KStroke::new(1.0), Affine::IDENTITY, c, None, &p);
}

/// Approximate text width for layout (glyph-accurate path later).
pub fn measure(text: &str, size: f64) -> f64 {
    (text.len() as f64) * size * 0.55
}

pub fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Draw a simple solid bar as a text stand-in for chrome labels.
/// Real text shaping is wired through `x_text` on content nodes; chrome
/// labels use this until the glyph atlas path is attached to the shell.
pub fn label_bar(s: &mut Scene, text: &str, x: f64, y: f64, size: f64, c: Color) {
    let w = measure(text, size).max(4.0);
    let h = (size * 0.35).max(2.0);
    fill_rrect(
        s,
        Rect::new(x, y + size * 0.35, x + w, y + size * 0.35 + h),
        1.0,
        c,
    );
}
