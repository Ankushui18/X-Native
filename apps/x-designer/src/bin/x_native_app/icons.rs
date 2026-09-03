//! X-Native icon library — Design System v2 "Ink & Ember".
//!
//! One systematic stroke set for ALL chrome glyphs (toolbar, layer list,
//! left rail, status bar, menus): 24×24 design grid, 1.8-unit stroke with
//! round caps/joins, stroke only (no fills in chrome — consistent weight
//! at every rendered size, per the v2 iconography spec).
//!
//! Geometry is Lucide-inspired (open shapes, 45° cuts, optical circles)
//! but self-contained: zero assets, pure vector paths in the binary.

use super::*;

/// The full v2 set. The library ships complete — like an icon package —
/// so every future chrome surface draws from one system instead of
/// reintroducing ad-hoc glyph paths; not every variant has a call site yet.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    // ---- tools ----
    Move,
    Frame,
    Rect,
    Ellipse,
    LineTool,
    Pen,
    Text,
    Hand,
    Slice,
    Eyedropper,
    Pencil,
    Bucket,
    Brush,
    ArcTool,
    Hexagon,
    Star,
    Scale,
    // ---- system / navigation ----
    Search,
    Share,
    Settings,
    Plus,
    Minus,
    Check,
    Close,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    More,
    Eye,
    EyeOff,
    Lock,
    Trash,
    Copy,
    Undo,
    Redo,
    Magnet,
    Grid,
    Layers,
    File,
    Diamond,
    DiamondOut,
    Book,
    Play,
    ZoomIn,
    ZoomOut,
    Help,
    PanelLeft,
    PanelRight,
    FlipH,
    FlipV,
    ArrowUp,
    ArrowRight,
    ArrowDown,
    ArrowLeft,
    AlignLeft,
    AlignCenterH,
    AlignRight,
    AlignTop,
    AlignCenterV,
    AlignBottom,
    DistH,
    DistV,
    Image,
    Link,
    Corner,
    Download,
    Upload,
    Minimize,
    Dot,
}

/// Append a full circle to `p` as four bezier arcs (keeps every icon in
/// one stroked path).
fn circle(p: &mut vello::kurbo::BezPath, cx: f64, cy: f64, r: f64) {
    let k = 0.5523 * r;
    p.move_to((cx - r, cy));
    p.curve_to((cx - r, cy - k), (cx - k, cy - r), (cx, cy - r));
    p.curve_to((cx + k, cy - r), (cx + r, cy - k), (cx + r, cy));
    p.curve_to((cx + r, cy + k), (cx + k, cy + r), (cx, cy + r));
    p.curve_to((cx - k, cy + r), (cx - r, cy + k), (cx - r, cy));
    p.close_path();
}

/// Rounded-rect subpath.
fn rrect(p: &mut vello::kurbo::BezPath, x0: f64, y0: f64, x1: f64, y1: f64, r: f64) {
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    let k = 0.5523 * r;
    p.move_to((x0 + r, y0));
    p.line_to((x1 - r, y0));
    p.curve_to((x1 - r + k, y0), (x1, y0 + r - k), (x1, y0 + r));
    p.line_to((x1, y1 - r));
    p.curve_to((x1, y1 - r + k), (x1 - r + k, y1), (x1 - r, y1));
    p.line_to((x0 + r, y1));
    p.curve_to((x0 + r - k, y1), (x0, y1 - r + k), (x0, y1 - r));
    p.line_to((x0, y0 + r));
    p.curve_to((x0, y0 + r - k), (x0 + r - k, y0), (x0 + r, y0));
    p.close_path();
}

/// Elliptical arc subpath from `start` sweeping clockwise (y-down) by
/// `sweep` degrees. Move-only start (no close) so arcs stay open.
fn arc(p: &mut vello::kurbo::BezPath, cx: f64, cy: f64, rx: f64, ry: f64, start: f64, sweep: f64) {
    let pt = |deg: f64| {
        let t = deg.to_radians();
        (cx + rx * t.cos(), cy + ry * t.sin())
    };
    let tang = |deg: f64| {
        let t = deg.to_radians();
        (-rx * t.sin(), ry * t.cos())
    };
    let n = ((sweep.abs() / 90.0).ceil() as usize).max(1);
    let seg = sweep / n as f64;
    let kappa = 4.0 / 3.0 * (seg.abs().to_radians() / 4.0).tan();
    let p0 = pt(start);
    p.move_to(p0);
    for i in 0..n {
        let a0 = start + seg * i as f64;
        let a1 = a0 + seg;
        let (x0, y0) = pt(a0);
        let (x1, y1) = pt(a1);
        let (t0x, t0y) = tang(a0);
        let (t1x, t1y) = tang(a1);
        let s = kappa * if seg < 0.0 { -1.0 } else { 1.0 };
        p.curve_to(
            (x0 + t0x * s, y0 + t0y * s),
            (x1 - t1x * s, y1 - t1y * s),
            (x1, y1),
        );
    }
}

/// Paint `icon` centered at (cx, cy) inside a `size`×`size` box.
pub fn paint(s: &mut Scene, icon: Icon, cx: f64, cy: f64, size: f64, color: Color) {
    let scale = size / 24.0;
    let t = Affine::translate((cx - 12.0 * scale, cy - 12.0 * scale)) * Affine::scale(scale);
    let width = (1.8 * scale).max(1.0);
    let st = vello::kurbo::Stroke::new(width)
        .with_caps(vello::kurbo::Cap::Round)
        .with_join(vello::kurbo::Join::Round);
    let mut p = vello::kurbo::BezPath::new();

    match icon {
        // ---------------- tools ----------------
        Icon::Move => {
            // cursor arrow with tail notch
            p.move_to((5.0, 3.0));
            p.line_to((5.0, 18.5));
            p.line_to((9.6, 14.4));
            p.line_to((12.4, 20.4));
            p.line_to((14.8, 19.3));
            p.line_to((12.1, 13.5));
            p.line_to((18.2, 13.5));
            p.close_path();
        }
        Icon::Frame => {
            // Figma-style frame: hash mark
            p.move_to((3.0, 8.0));
            p.line_to((21.0, 8.0));
            p.move_to((3.0, 16.0));
            p.line_to((21.0, 16.0));
            p.move_to((8.0, 3.0));
            p.line_to((8.0, 21.0));
            p.move_to((16.0, 3.0));
            p.line_to((16.0, 21.0));
        }
        Icon::Rect => rrect(&mut p, 4.0, 4.0, 20.0, 20.0, 2.0),
        Icon::Ellipse => circle(&mut p, 12.0, 12.0, 8.0),
        Icon::LineTool => {
            p.move_to((5.0, 19.0));
            p.line_to((19.0, 5.0));
        }
        Icon::Pen => {
            // pen-tool: nib polygon + breathing dot
            p.move_to((11.0, 21.0));
            p.line_to((21.0, 11.0));
            p.line_to((17.0, 7.0));
            p.line_to((7.0, 17.0));
            p.close_path();
            p.move_to((7.0, 17.0));
            p.line_to((4.0, 20.0));
            circle(&mut p, 14.5, 14.5, 1.2);
        }
        Icon::Text => {
            p.move_to((5.0, 7.5));
            p.line_to((5.0, 5.0));
            p.line_to((19.0, 5.0));
            p.line_to((19.0, 7.5));
            p.move_to((12.0, 5.0));
            p.line_to((12.0, 19.0));
            p.move_to((9.0, 19.0));
            p.line_to((15.0, 19.0));
        }
        Icon::Hand => {
            // three fingers + palm sweep
            p.move_to((7.5, 11.5));
            p.line_to((7.5, 5.5));
            p.curve_to((7.5, 4.4), (9.2, 4.4), (9.2, 5.5));
            p.line_to((9.2, 11.0));
            p.move_to((11.2, 10.5));
            p.line_to((11.2, 4.2));
            p.curve_to((11.2, 3.1), (12.9, 3.1), (12.9, 4.2));
            p.line_to((12.9, 10.5));
            p.move_to((14.9, 11.0));
            p.line_to((14.9, 5.8));
            p.curve_to((14.9, 4.7), (16.6, 4.7), (16.6, 5.8));
            p.line_to((16.6, 12.0));
            p.move_to((16.6, 10.0));
            p.curve_to((18.4, 10.0), (20.0, 11.4), (20.0, 13.4));
            p.line_to((20.0, 15.5));
            p.curve_to((20.0, 18.9), (17.3, 21.0), (13.8, 21.0));
            p.line_to((12.0, 21.0));
            p.curve_to((10.4, 21.0), (9.2, 20.4), (8.1, 19.3));
            p.line_to((5.3, 16.4));
            p.curve_to((4.5, 15.6), (5.4, 14.3), (6.3, 15.0));
            p.line_to((7.5, 16.0));
        }
        Icon::Slice => {
            // scalpel: blade + handle edge
            p.move_to((3.0, 21.0));
            p.line_to((4.2, 16.4));
            p.line_to((15.5, 5.0));
            p.curve_to((16.6, 3.9), (18.4, 3.9), (19.5, 5.0));
            p.curve_to((20.6, 6.1), (20.6, 7.9), (19.5, 9.0));
            p.line_to((8.1, 20.4));
            p.close_path();
            p.move_to((3.0, 21.0));
            p.line_to((9.0, 19.5));
        }
        Icon::Eyedropper => {
            // pipette: hollow tip + barrel + bulb
            p.move_to((4.0, 20.0));
            p.line_to((4.0, 15.5));
            p.line_to((13.0, 6.5));
            p.line_to((17.5, 11.0));
            p.line_to((8.5, 20.0));
            p.close_path();
            p.move_to((13.0, 6.5));
            p.line_to((15.2, 4.3));
            p.curve_to((16.4, 3.1), (18.3, 3.1), (19.6, 4.4));
            p.curve_to((20.9, 5.7), (20.9, 7.6), (19.7, 8.8));
            p.line_to((17.5, 11.0));
        }
        Icon::Pencil => {
            p.move_to((19.6, 4.4));
            p.curve_to((18.3, 3.0), (16.2, 3.0), (15.0, 4.3));
            p.line_to((3.0, 16.4));
            p.line_to((2.2, 21.8));
            p.line_to((7.6, 21.0));
            p.line_to((19.6, 8.9));
            p.curve_to((20.9, 7.6), (20.9, 5.7), (19.6, 4.4));
            p.close_path();
            p.move_to((14.2, 5.2));
            p.line_to((18.7, 9.7));
        }
        Icon::Bucket => {
            // tilted bucket + falling drop
            p.move_to((5.5, 9.0));
            p.line_to((11.5, 3.2));
            p.line_to((15.5, 7.2));
            p.line_to((9.5, 13.2));
            p.close_path();
            p.move_to((5.5, 9.0));
            p.line_to((4.6, 15.8));
            p.move_to((9.5, 13.2));
            p.curve_to((11.0, 14.7), (13.0, 14.6), (14.0, 13.6));
            p.curve_to((15.0, 12.6), (16.4, 12.6), (17.2, 13.4));
            p.move_to((16.8, 17.2));
            p.curve_to((15.4, 18.9), (15.8, 20.9), (17.1, 20.9));
            p.curve_to((18.4, 20.9), (18.8, 18.9), (17.4, 17.2));
            p.close_path();
        }
        Icon::Brush => {
            // handle + ferrule + loaded tip
            p.move_to((19.8, 4.2));
            p.line_to((13.5, 10.5));
            p.move_to((20.5, 3.5));
            p.curve_to((21.4, 4.4), (21.4, 5.9), (20.5, 6.8));
            p.line_to((17.2, 10.1));
            p.move_to((13.5, 10.5));
            p.line_to((17.2, 10.1));
            p.move_to((13.5, 10.5));
            p.curve_to((15.3, 12.3), (15.6, 15.0), (14.0, 16.6));
            p.curve_to((12.0, 18.6), (9.0, 18.4), (7.2, 20.2));
            p.curve_to((6.2, 21.2), (4.7, 21.2), (3.8, 20.3));
            p.curve_to((2.9, 19.4), (2.9, 17.9), (3.9, 16.9));
            p.curve_to((5.6, 15.2), (5.5, 12.2), (7.5, 10.2));
            p.curve_to((9.0, 8.7), (11.7, 8.9), (13.5, 10.5));
        }
        Icon::ArcTool => arc(&mut p, 12.0, 12.0, 8.0, 8.0, -45.0, 270.0),
        Icon::Hexagon => {
            let pts = [
                (20.5, 12.0),
                (16.25, 19.36),
                (7.75, 19.36),
                (3.5, 12.0),
                (7.75, 4.64),
                (16.25, 4.64),
            ];
            p.move_to(pts[0]);
            for q in &pts[1..] {
                p.line_to(*q);
            }
            p.close_path();
        }
        Icon::Star => {
            let pts = [
                (12.0, 3.5),
                (14.12, 9.09),
                (20.08, 9.37),
                (15.42, 13.11),
                (17.0, 18.88),
                (12.0, 15.6),
                (7.0, 18.88),
                (8.58, 13.11),
                (3.92, 9.37),
                (9.88, 9.09),
            ];
            p.move_to(pts[0]);
            for q in &pts[1..] {
                p.line_to(*q);
            }
            p.close_path();
        }
        Icon::Scale => {
            // corner brackets + main diagonal
            p.move_to((14.0, 4.0));
            p.line_to((20.0, 4.0));
            p.line_to((20.0, 10.0));
            p.move_to((10.0, 20.0));
            p.line_to((4.0, 20.0));
            p.line_to((4.0, 14.0));
            p.move_to((20.0, 4.0));
            p.line_to((4.0, 20.0));
        }

        // ---------------- system ----------------
        Icon::Search => {
            circle(&mut p, 11.0, 11.0, 7.0);
            p.move_to((16.2, 16.2));
            p.line_to((21.0, 21.0));
        }
        Icon::Share => {
            // person + plus (invite)
            circle(&mut p, 9.5, 7.0, 3.2);
            p.move_to((3.8, 20.0));
            p.curve_to((3.8, 14.0), (6.4, 11.2), (9.5, 11.2));
            p.curve_to((12.6, 11.2), (15.2, 14.0), (15.2, 20.0));
            p.move_to((18.7, 14.0));
            p.line_to((18.7, 20.0));
            p.move_to((15.7, 17.0));
            p.line_to((21.7, 17.0));
        }
        Icon::Settings => {
            // sliders
            p.move_to((4.0, 7.0));
            p.line_to((12.6, 7.0));
            p.move_to((17.4, 7.0));
            p.line_to((20.0, 7.0));
            circle(&mut p, 15.0, 7.0, 2.2);
            p.move_to((4.0, 12.0));
            p.line_to((5.6, 12.0));
            p.move_to((10.4, 12.0));
            p.line_to((20.0, 12.0));
            circle(&mut p, 8.0, 12.0, 2.2);
            p.move_to((4.0, 17.0));
            p.line_to((14.6, 17.0));
            p.move_to((19.4, 17.0));
            p.line_to((20.0, 17.0));
            circle(&mut p, 17.0, 17.0, 2.2);
        }
        Icon::Plus => {
            p.move_to((12.0, 5.0));
            p.line_to((12.0, 19.0));
            p.move_to((5.0, 12.0));
            p.line_to((19.0, 12.0));
        }
        Icon::Minus => {
            p.move_to((5.0, 12.0));
            p.line_to((19.0, 12.0));
        }
        Icon::Check => {
            p.move_to((20.0, 6.0));
            p.line_to((9.5, 17.0));
            p.line_to((4.0, 11.5));
        }
        Icon::Close => {
            p.move_to((6.0, 6.0));
            p.line_to((18.0, 18.0));
            p.move_to((18.0, 6.0));
            p.line_to((6.0, 18.0));
        }
        Icon::ChevronDown => {
            p.move_to((6.0, 9.5));
            p.line_to((12.0, 15.5));
            p.line_to((18.0, 9.5));
        }
        Icon::ChevronUp => {
            p.move_to((6.0, 14.5));
            p.line_to((12.0, 8.5));
            p.line_to((18.0, 14.5));
        }
        Icon::ChevronLeft => {
            p.move_to((14.5, 6.0));
            p.line_to((8.5, 12.0));
            p.line_to((14.5, 18.0));
        }
        Icon::ChevronRight => {
            p.move_to((9.5, 6.0));
            p.line_to((15.5, 12.0));
            p.line_to((9.5, 18.0));
        }
        Icon::More => {
            circle(&mut p, 5.5, 12.0, 1.1);
            circle(&mut p, 12.0, 12.0, 1.1);
            circle(&mut p, 18.5, 12.0, 1.1);
        }
        Icon::Eye => {
            p.move_to((2.5, 12.0));
            p.curve_to((5.0, 6.8), (8.3, 4.5), (12.0, 4.5));
            p.curve_to((15.7, 4.5), (19.0, 6.8), (21.5, 12.0));
            p.curve_to((19.0, 17.2), (15.7, 19.5), (12.0, 19.5));
            p.curve_to((8.3, 19.5), (5.0, 17.2), (2.5, 12.0));
            p.close_path();
            circle(&mut p, 12.0, 12.0, 3.0);
        }
        Icon::EyeOff => {
            p.move_to((4.0, 4.0));
            p.line_to((20.0, 20.0));
            p.move_to((9.9, 5.1));
            p.curve_to((10.6, 4.7), (11.3, 4.5), (12.0, 4.5));
            p.curve_to((15.7, 4.5), (19.0, 6.8), (21.5, 12.0));
            p.curve_to((20.7, 13.7), (19.7, 15.1), (18.5, 16.2));
            p.move_to((6.7, 7.8));
            p.curve_to((5.2, 8.9), (3.8, 10.3), (2.5, 12.0));
            p.curve_to((5.0, 17.2), (8.3, 19.5), (12.0, 19.5));
            p.curve_to((13.5, 19.5), (14.9, 19.1), (16.2, 18.3));
        }
        Icon::Lock => {
            rrect(&mut p, 4.5, 10.5, 19.5, 20.5, 2.5);
            p.move_to((8.0, 10.5));
            p.line_to((8.0, 8.0));
            p.curve_to((8.0, 5.8), (9.8, 4.0), (12.0, 4.0));
            p.curve_to((14.2, 4.0), (16.0, 5.8), (16.0, 8.0));
            p.line_to((16.0, 10.5));
        }
        Icon::Trash => {
            p.move_to((4.0, 7.0));
            p.line_to((20.0, 7.0));
            p.move_to((18.5, 7.0));
            p.line_to((18.5, 19.0));
            p.curve_to((18.5, 20.1), (17.6, 21.0), (16.5, 21.0));
            p.line_to((7.5, 21.0));
            p.curve_to((6.4, 21.0), (5.5, 20.1), (5.5, 19.0));
            p.line_to((5.5, 7.0));
            p.move_to((9.0, 7.0));
            p.line_to((9.0, 4.8));
            p.curve_to((9.0, 3.8), (9.8, 3.0), (10.8, 3.0));
            p.line_to((13.2, 3.0));
            p.curve_to((14.2, 3.0), (15.0, 3.8), (15.0, 4.8));
            p.line_to((15.0, 7.0));
            p.move_to((10.0, 11.0));
            p.line_to((10.0, 17.0));
            p.move_to((14.0, 11.0));
            p.line_to((14.0, 17.0));
        }
        Icon::Copy => {
            rrect(&mut p, 8.5, 8.5, 20.5, 20.5, 2.0);
            p.move_to((5.5, 15.5));
            p.line_to((5.0, 15.5));
            p.curve_to((3.9, 15.5), (3.0, 14.6), (3.0, 13.5));
            p.line_to((3.0, 5.0));
            p.curve_to((3.0, 3.9), (3.9, 3.0), (5.0, 3.0));
            p.line_to((13.5, 3.0));
            p.curve_to((14.6, 3.0), (15.5, 3.9), (15.5, 5.0));
            p.line_to((15.5, 5.5));
        }
        Icon::Undo => {
            p.move_to((9.0, 14.0));
            p.line_to((4.0, 9.0));
            p.line_to((9.0, 4.0));
            p.move_to((4.0, 9.0));
            p.line_to((14.5, 9.0));
            p.curve_to((17.5, 9.0), (20.0, 11.5), (20.0, 14.5));
            p.curve_to((20.0, 17.5), (17.5, 20.0), (14.5, 20.0));
            p.line_to((11.0, 20.0));
        }
        Icon::Redo => {
            p.move_to((15.0, 14.0));
            p.line_to((20.0, 9.0));
            p.line_to((15.0, 4.0));
            p.move_to((20.0, 9.0));
            p.line_to((9.5, 9.0));
            p.curve_to((6.5, 9.0), (4.0, 11.5), (4.0, 14.5));
            p.curve_to((4.0, 17.5), (6.5, 20.0), (9.5, 20.0));
            p.line_to((13.0, 20.0));
        }
        Icon::Magnet => {
            p.move_to((8.0, 4.0));
            p.line_to((8.0, 10.0));
            p.curve_to((8.0, 13.3), (16.0, 13.3), (16.0, 10.0));
            p.line_to((16.0, 4.0));
            p.move_to((6.5, 7.0));
            p.line_to((9.5, 7.0));
            p.move_to((14.5, 7.0));
            p.line_to((17.5, 7.0));
        }
        Icon::Grid => {
            rrect(&mut p, 3.5, 3.5, 10.5, 10.5, 1.5);
            rrect(&mut p, 13.5, 3.5, 20.5, 10.5, 1.5);
            rrect(&mut p, 3.5, 13.5, 10.5, 20.5, 1.5);
            rrect(&mut p, 13.5, 13.5, 20.5, 20.5, 1.5);
        }
        Icon::Layers => {
            p.move_to((12.0, 2.5));
            p.line_to((21.5, 7.0));
            p.line_to((12.0, 11.5));
            p.line_to((2.5, 7.0));
            p.close_path();
            p.move_to((2.5, 12.0));
            p.line_to((12.0, 16.5));
            p.line_to((21.5, 12.0));
            p.move_to((2.5, 17.0));
            p.line_to((12.0, 21.5));
            p.line_to((21.5, 17.0));
        }
        Icon::File => {
            p.move_to((6.0, 3.0));
            p.line_to((14.5, 3.0));
            p.line_to((19.0, 7.5));
            p.line_to((19.0, 21.0));
            p.line_to((6.0, 21.0));
            p.close_path();
            p.move_to((14.5, 3.0));
            p.line_to((14.5, 7.5));
            p.line_to((19.0, 7.5));
        }
        Icon::Diamond => {
            p.move_to((12.0, 3.0));
            p.line_to((21.0, 12.0));
            p.line_to((12.0, 21.0));
            p.line_to((3.0, 12.0));
            p.close_path();
        }
        Icon::DiamondOut => {
            p.move_to((12.0, 3.0));
            p.line_to((21.0, 12.0));
            p.line_to((12.0, 21.0));
            p.line_to((3.0, 12.0));
            p.close_path();
            circle(&mut p, 12.0, 12.0, 1.0);
        }
        Icon::Book => {
            p.move_to((12.0, 6.2));
            p.curve_to((10.0, 4.6), (7.5, 4.0), (4.0, 4.0));
            p.line_to((4.0, 19.0));
            p.curve_to((7.5, 19.0), (10.0, 19.6), (12.0, 21.0));
            p.curve_to((14.0, 19.6), (16.5, 19.0), (20.0, 19.0));
            p.line_to((20.0, 4.0));
            p.curve_to((16.5, 4.0), (14.0, 4.6), (12.0, 6.2));
            p.close_path();
            p.move_to((12.0, 6.2));
            p.line_to((12.0, 21.0));
        }
        Icon::Play => {
            p.move_to((8.0, 5.5));
            p.line_to((18.5, 12.0));
            p.line_to((8.0, 18.5));
            p.close_path();
        }
        Icon::ZoomIn => {
            circle(&mut p, 11.0, 11.0, 7.0);
            p.move_to((16.2, 16.2));
            p.line_to((21.0, 21.0));
            p.move_to((11.0, 8.0));
            p.line_to((11.0, 14.0));
            p.move_to((8.0, 11.0));
            p.line_to((14.0, 11.0));
        }
        Icon::ZoomOut => {
            circle(&mut p, 11.0, 11.0, 7.0);
            p.move_to((16.2, 16.2));
            p.line_to((21.0, 21.0));
            p.move_to((8.0, 11.0));
            p.line_to((14.0, 11.0));
        }
        Icon::Help => {
            circle(&mut p, 12.0, 12.0, 9.0);
            p.move_to((9.4, 9.3));
            p.curve_to((9.4, 7.6), (10.6, 6.6), (12.0, 6.6));
            p.curve_to((13.4, 6.6), (14.6, 7.5), (14.6, 8.9));
            p.curve_to((14.6, 11.1), (11.6, 11.3), (11.6, 13.4));
            circle(&mut p, 12.0, 16.8, 0.7);
        }
        Icon::PanelLeft => {
            rrect(&mut p, 3.0, 4.0, 21.0, 20.0, 2.0);
            p.move_to((9.5, 4.0));
            p.line_to((9.5, 20.0));
        }
        Icon::PanelRight => {
            rrect(&mut p, 3.0, 4.0, 21.0, 20.0, 2.0);
            p.move_to((14.5, 4.0));
            p.line_to((14.5, 20.0));
        }
        Icon::FlipH => {
            p.move_to((12.0, 3.5));
            p.line_to((12.0, 20.5));
            p.move_to((9.0, 8.0));
            p.line_to((4.5, 12.0));
            p.line_to((9.0, 16.0));
            p.move_to((15.0, 8.0));
            p.line_to((19.5, 12.0));
            p.line_to((15.0, 16.0));
        }
        Icon::FlipV => {
            p.move_to((3.5, 12.0));
            p.line_to((20.5, 12.0));
            p.move_to((8.0, 9.0));
            p.line_to((12.0, 4.5));
            p.line_to((16.0, 9.0));
            p.move_to((8.0, 15.0));
            p.line_to((12.0, 19.5));
            p.line_to((16.0, 15.0));
        }
        Icon::ArrowUp => {
            p.move_to((12.0, 19.0));
            p.line_to((12.0, 5.0));
            p.move_to((6.0, 11.0));
            p.line_to((12.0, 5.0));
            p.line_to((18.0, 11.0));
        }
        Icon::ArrowRight => {
            p.move_to((5.0, 12.0));
            p.line_to((19.0, 12.0));
            p.move_to((13.0, 6.0));
            p.line_to((19.0, 12.0));
            p.line_to((13.0, 18.0));
        }
        Icon::ArrowDown => {
            p.move_to((12.0, 5.0));
            p.line_to((12.0, 19.0));
            p.move_to((6.0, 13.0));
            p.line_to((12.0, 19.0));
            p.line_to((18.0, 13.0));
        }
        Icon::ArrowLeft => {
            p.move_to((19.0, 12.0));
            p.line_to((5.0, 12.0));
            p.move_to((11.0, 6.0));
            p.line_to((5.0, 12.0));
            p.line_to((11.0, 18.0));
        }
        Icon::AlignLeft => {
            p.move_to((4.5, 3.5));
            p.line_to((4.5, 20.5));
            rrect(&mut p, 8.0, 6.5, 15.5, 10.5, 1.2);
            rrect(&mut p, 8.0, 13.5, 19.0, 17.5, 1.2);
        }
        Icon::AlignCenterH => {
            p.move_to((12.0, 3.5));
            p.line_to((12.0, 20.5));
            rrect(&mut p, 4.5, 6.5, 19.5, 10.5, 1.2);
            rrect(&mut p, 7.0, 13.5, 17.0, 17.5, 1.2);
        }
        Icon::AlignRight => {
            p.move_to((19.5, 3.5));
            p.line_to((19.5, 20.5));
            rrect(&mut p, 8.5, 6.5, 16.0, 10.5, 1.2);
            rrect(&mut p, 5.0, 13.5, 16.0, 17.5, 1.2);
        }
        Icon::AlignTop => {
            p.move_to((3.5, 4.5));
            p.line_to((20.5, 4.5));
            rrect(&mut p, 6.5, 8.0, 10.5, 15.5, 1.2);
            rrect(&mut p, 13.5, 8.0, 17.5, 19.0, 1.2);
        }
        Icon::AlignCenterV => {
            p.move_to((3.5, 12.0));
            p.line_to((20.5, 12.0));
            rrect(&mut p, 6.5, 4.5, 10.5, 19.5, 1.2);
            rrect(&mut p, 13.5, 7.0, 17.5, 17.0, 1.2);
        }
        Icon::AlignBottom => {
            p.move_to((3.5, 19.5));
            p.line_to((20.5, 19.5));
            rrect(&mut p, 6.5, 8.5, 10.5, 16.0, 1.2);
            rrect(&mut p, 13.5, 5.0, 17.5, 16.0, 1.2);
        }
        Icon::DistH => {
            p.move_to((3.0, 12.0));
            p.line_to((21.0, 12.0));
            p.move_to((7.0, 8.0));
            p.line_to((3.0, 12.0));
            p.line_to((7.0, 16.0));
            p.move_to((17.0, 8.0));
            p.line_to((21.0, 12.0));
            p.line_to((17.0, 16.0));
        }
        Icon::DistV => {
            p.move_to((12.0, 3.0));
            p.line_to((12.0, 21.0));
            p.move_to((8.0, 7.0));
            p.line_to((12.0, 3.0));
            p.line_to((16.0, 7.0));
            p.move_to((8.0, 17.0));
            p.line_to((12.0, 21.0));
            p.line_to((16.0, 17.0));
        }
        Icon::Image => {
            rrect(&mut p, 3.0, 5.0, 21.0, 19.0, 2.0);
            circle(&mut p, 8.5, 10.0, 1.6);
            p.move_to((3.0, 17.0));
            p.line_to((8.5, 12.5));
            p.line_to((12.5, 16.5));
            p.line_to((16.0, 13.0));
            p.line_to((21.0, 17.5));
        }
        Icon::Link => {
            p.move_to((9.5, 7.0));
            p.line_to((7.0, 7.0));
            p.curve_to((4.2, 7.0), (4.2, 17.0), (7.0, 17.0));
            p.line_to((9.5, 17.0));
            p.move_to((14.5, 7.0));
            p.line_to((17.0, 7.0));
            p.curve_to((19.8, 7.0), (19.8, 17.0), (17.0, 17.0));
            p.line_to((14.5, 17.0));
            p.move_to((8.5, 12.0));
            p.line_to((15.5, 12.0));
        }
        Icon::Corner => {
            p.move_to((4.0, 20.0));
            p.line_to((4.0, 9.0));
            p.curve_to((4.0, 6.2), (6.2, 4.0), (9.0, 4.0));
            p.line_to((20.0, 4.0));
        }
        Icon::Download => {
            p.move_to((12.0, 3.0));
            p.line_to((12.0, 15.0));
            p.move_to((7.5, 10.5));
            p.line_to((12.0, 15.0));
            p.line_to((16.5, 10.5));
            p.move_to((4.0, 15.0));
            p.line_to((4.0, 19.0));
            p.curve_to((4.0, 20.1), (4.9, 21.0), (6.0, 21.0));
            p.line_to((18.0, 21.0));
            p.curve_to((19.1, 21.0), (20.0, 20.1), (20.0, 19.0));
            p.line_to((20.0, 15.0));
        }
        Icon::Upload => {
            p.move_to((12.0, 15.0));
            p.line_to((12.0, 3.0));
            p.move_to((7.5, 7.5));
            p.line_to((12.0, 3.0));
            p.line_to((16.5, 7.5));
            p.move_to((4.0, 15.0));
            p.line_to((4.0, 19.0));
            p.curve_to((4.0, 20.1), (4.9, 21.0), (6.0, 21.0));
            p.line_to((18.0, 21.0));
            p.curve_to((19.1, 21.0), (20.0, 20.1), (20.0, 19.0));
            p.line_to((20.0, 15.0));
        }
        Icon::Minimize => {
            p.move_to((8.0, 3.5));
            p.line_to((8.0, 8.0));
            p.line_to((3.5, 8.0));
            p.move_to((16.0, 3.5));
            p.line_to((16.0, 8.0));
            p.line_to((20.5, 8.0));
            p.move_to((3.5, 16.0));
            p.line_to((8.0, 16.0));
            p.line_to((8.0, 20.5));
            p.move_to((16.0, 20.5));
            p.line_to((16.0, 16.0));
            p.line_to((20.5, 16.0));
        }
        Icon::Dot => circle(&mut p, 12.0, 12.0, 2.0),
    }

    s.stroke(&st, t, color, None, &p);
}

/// Tool → icon mapping (the full 17-tool dock).
pub fn tool_icon(tool: crate::state::Tool) -> Icon {
    use crate::state::Tool;
    match tool {
        Tool::Select => Icon::Move,
        Tool::Hand => Icon::Hand,
        Tool::Scale => Icon::Scale,
        Tool::Frame => Icon::Frame,
        Tool::Rectangle => Icon::Rect,
        Tool::Ellipse => Icon::Ellipse,
        Tool::Arc => Icon::ArcTool,
        Tool::Line => Icon::LineTool,
        Tool::Polygon => Icon::Hexagon,
        Tool::Star => Icon::Star,
        Tool::Text => Icon::Text,
        Tool::Pen => Icon::Pen,
        Tool::Slice => Icon::Slice,
        Tool::Eyedropper => Icon::Eyedropper,
        Tool::Pencil => Icon::Pencil,
        Tool::Bucket => Icon::Bucket,
        Tool::Brush => Icon::Brush,
    }
}

/// NodeKind → layer-list icon.
pub fn kind_icon(kind: &x_native::NodeKind) -> Icon {
    use x_native::NodeKind;
    match kind {
        NodeKind::Frame { .. } => Icon::Frame,
        NodeKind::Group => Icon::Layers,
        NodeKind::Section => Icon::PanelLeft,
        NodeKind::Rect { .. } => Icon::Rect,
        NodeKind::Ellipse => Icon::Ellipse,
        NodeKind::Arc { .. } => Icon::ArcTool,
        NodeKind::Line => Icon::LineTool,
        NodeKind::Text { .. } => Icon::Text,
        NodeKind::Image { .. } => Icon::Image,
        NodeKind::Vector { .. } => Icon::Pen,
        NodeKind::Component { .. } => Icon::Diamond,
        NodeKind::Instance { .. } => Icon::DiamondOut,
        NodeKind::Slice => Icon::Slice,
    }
}
