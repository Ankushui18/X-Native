//! X-Native Icon System — Lucide.dev — Production — No Emoji
//! All icons stroke-only, 1.75px, rounded caps/joins

use vello::kurbo::{BezPath, Rect};
use vello::peniko::Color;
use vello::Scene;
use vello::kurbo::{Affine, Stroke as KStroke};

#[derive(Clone, Copy, Debug)]
pub enum Icon {
    Cursor,
    Frame,
    Type,
    Square,
    PenTool,
    Hand,
    Search,
    Plus,
    Minus,
    Home,
    ChevronDown,
    ChevronRight,
    ChevronLeft,
    Grid,
    Play,
    X,
    Check,
    Sparkles,
    ArrowDownUp,
    ArrowRightLeft,
    ArrowDown01,
    ArrowLeftRight,
    ArrowUpDown,
    Maximize,
    Lock,
    Unlock,
    Distribute,
    Command,
    Eye,
    EyeOff,
    Triangle,
    More,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    Layers,
    Tokens,
    Assets,
    Settings,
    Help,
    Message,
    Image,
    Book,
    Rotate,
    Corner,
    PaintBucket,
    Download,
    File,
    Folder,
    Board,
    Ellipse,
    Circle,
    Component,
    Library,
    Drafts,
}

fn stroke_style(s: f64) -> KStroke {
    KStroke::new(1.75 * s).with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round)
}
fn thin_stroke(s: f64) -> KStroke {
    KStroke::new(1.5 * s).with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round)
}

pub fn draw_icon(scene: &mut Scene, icon: Icon, x: f64, y: f64, size: f64, color: Color) {
    let s = size / 24.0;
    let stroke = stroke_style(s);
    let thin = thin_stroke(s);
    let mut paths: Vec<(BezPath, KStroke)> = Vec::new();
    let mut add = |p: BezPath, st: KStroke| paths.push((p, st));

    match icon {
        Icon::Cursor => {
            let mut p = BezPath::new();
            p.move_to((4.5*s+x, 5.0*s+y));
            p.line_to((8.5*s+x, 17.0*s+y));
            p.line_to((11.0*s+x, 11.0*s+y));
            p.line_to((15.0*s+x, 9.0*s+y));
            p.close_path();
            add(p, stroke);
        }
        Icon::Frame | Icon::Board => {
            let mut p = BezPath::new();
            p.move_to((3.0*s+x, 3.0*s+y)); p.line_to((21.0*s+x, 3.0*s+y)); p.line_to((21.0*s+x, 21.0*s+y)); p.line_to((3.0*s+x, 21.0*s+y)); p.close_path();
            add(p, stroke);
            let mut p2 = BezPath::new(); p2.move_to((3.0*s+x, 9.0*s+y)); p2.line_to((21.0*s+x, 9.0*s+y)); add(p2, thin.clone());
            let mut p3 = BezPath::new(); p3.move_to((9.0*s+x, 3.0*s+y)); p3.line_to((9.0*s+x, 21.0*s+y)); add(p3, thin.clone());
        }
        Icon::Type => {
            let mut p1 = BezPath::new(); p1.move_to((4.0*s+x, 6.0*s+y)); p1.line_to((20.0*s+x, 6.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((12.0*s+x, 6.0*s+y)); p2.line_to((12.0*s+x, 20.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Square => {
            let mut p = BezPath::new(); p.move_to((3.0*s+x, 3.0*s+y)); p.line_to((21.0*s+x, 3.0*s+y)); p.line_to((21.0*s+x, 21.0*s+y)); p.line_to((3.0*s+x, 21.0*s+y)); p.close_path(); add(p, stroke);
        }
        Icon::Ellipse | Icon::Circle => {
            let mut p = BezPath::new();
            p.move_to((12.0*s+x, 3.0*s+y));
            p.curve_to((17.0*s+x, 3.0*s+y), (21.0*s+x, 7.0*s+y), (21.0*s+x, 12.0*s+y));
            p.curve_to((21.0*s+x, 17.0*s+y), (17.0*s+x, 21.0*s+y), (12.0*s+x, 21.0*s+y));
            p.curve_to((7.0*s+x, 21.0*s+y), (3.0*s+x, 17.0*s+y), (3.0*s+x, 12.0*s+y));
            p.curve_to((3.0*s+x, 7.0*s+y), (7.0*s+x, 3.0*s+y), (12.0*s+x, 3.0*s+y));
            p.close_path();
            add(p, stroke);
        }
        Icon::PenTool => {
            let mut p = BezPath::new(); p.move_to((12.0*s+x, 19.0*s+y)); p.line_to((5.0*s+x, 12.0*s+y)); p.line_to((12.0*s+x, 5.0*s+y)); p.line_to((19.0*s+x, 12.0*s+y)); p.close_path(); add(p, stroke);
        }
        Icon::Hand => {
            let mut p = BezPath::new();
            p.move_to((8.0*s+x, 12.0*s+y)); p.line_to((8.0*s+x, 6.0*s+y));
            p.curve_to((8.0*s+x, 4.9*s+y), (8.9*s+x, 4.0*s+y), (10.0*s+x, 4.0*s+y));
            p.curve_to((11.1*s+x, 4.0*s+y), (12.0*s+x, 4.9*s+y), (12.0*s+x, 6.0*s+y));
            p.line_to((12.0*s+x, 11.0*s+y));
            add(p, stroke.clone());
            let mut p2 = BezPath::new();
            p2.move_to((12.0*s+x, 11.0*s+y)); p2.line_to((12.0*s+x, 5.0*s+y));
            p2.curve_to((12.0*s+x, 3.9*s+y), (12.9*s+x, 3.0*s+y), (14.0*s+x, 3.0*s+y));
            p2.curve_to((15.1*s+x, 3.0*s+y), (16.0*s+x, 3.9*s+y), (16.0*s+x, 5.0*s+y));
            p2.line_to((16.0*s+x, 12.0*s+y));
            add(p2, stroke.clone());
            let mut p3 = BezPath::new();
            p3.move_to((16.0*s+x, 12.0*s+y)); p3.line_to((16.0*s+x, 8.0*s+y));
            p3.curve_to((16.0*s+x, 6.9*s+y), (16.9*s+x, 6.0*s+y), (18.0*s+x, 6.0*s+y));
            p3.curve_to((19.1*s+x, 6.0*s+y), (20.0*s+x, 6.9*s+y), (20.0*s+x, 8.0*s+y));
            p3.line_to((20.0*s+x, 14.0*s+y));
            p3.curve_to((20.0*s+x, 17.5*s+y), (17.0*s+x, 21.0*s+y), (12.0*s+x, 21.0*s+y));
            p3.line_to((8.0*s+x, 21.0*s+y));
            p3.curve_to((6.0*s+x, 21.0*s+y), (4.0*s+x, 19.5*s+y), (4.0*s+x, 17.0*s+y));
            add(p3, stroke.clone());
        }
        Icon::Search => {
            let mut p1 = BezPath::new();
            p1.move_to((11.0*s+x, 17.0*s+y));
            p1.curve_to((14.31*s+x, 17.0*s+y), (17.0*s+x, 14.31*s+y), (17.0*s+x, 11.0*s+y));
            p1.curve_to((17.0*s+x, 7.69*s+y), (14.31*s+x, 5.0*s+y), (11.0*s+x, 5.0*s+y));
            p1.curve_to((7.69*s+x, 5.0*s+y), (5.0*s+x, 7.69*s+y), (5.0*s+x, 11.0*s+y));
            p1.curve_to((5.0*s+x, 14.31*s+y), (7.69*s+x, 17.0*s+y), (11.0*s+x, 17.0*s+y));
            p1.close_path(); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((20.0*s+x, 20.0*s+y)); p2.line_to((16.0*s+x, 16.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Plus => {
            let mut p1 = BezPath::new(); p1.move_to((12.0*s+x, 5.0*s+y)); p1.line_to((12.0*s+x, 19.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((5.0*s+x, 12.0*s+y)); p2.line_to((19.0*s+x, 12.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Minus => {
            let mut p = BezPath::new(); p.move_to((5.0*s+x, 12.0*s+y)); p.line_to((19.0*s+x, 12.0*s+y)); add(p, stroke.clone());
        }
        Icon::Home => {
            let mut p = BezPath::new(); p.move_to((3.0*s+x, 10.0*s+y)); p.line_to((12.0*s+x, 3.0*s+y)); p.line_to((21.0*s+x, 10.0*s+y)); p.line_to((21.0*s+x, 21.0*s+y)); p.line_to((3.0*s+x, 21.0*s+y)); p.close_path(); add(p, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((9.0*s+x, 21.0*s+y)); p2.line_to((9.0*s+x, 12.0*s+y)); p2.line_to((15.0*s+x, 12.0*s+y)); p2.line_to((15.0*s+x, 21.0*s+y)); add(p2, stroke.clone());
        }
        Icon::ChevronDown => {
            let mut p = BezPath::new(); p.move_to((6.0*s+x, 9.0*s+y)); p.line_to((12.0*s+x, 15.0*s+y)); p.line_to((18.0*s+x, 9.0*s+y)); add(p, stroke.clone());
        }
        Icon::ChevronRight => {
            let mut p = BezPath::new(); p.move_to((9.0*s+x, 6.0*s+y)); p.line_to((15.0*s+x, 12.0*s+y)); p.line_to((9.0*s+x, 18.0*s+y)); add(p, stroke.clone());
        }
        Icon::ChevronLeft => {
            let mut p = BezPath::new(); p.move_to((15.0*s+x, 6.0*s+y)); p.line_to((9.0*s+x, 12.0*s+y)); p.line_to((15.0*s+x, 18.0*s+y)); add(p, stroke.clone());
        }
        Icon::Grid => {
            for (ox, oy) in [(3.0,3.0),(15.0,3.0),(3.0,15.0),(15.0,15.0)] {
                let mut p = BezPath::new(); p.move_to((ox*s+x, oy*s+y)); p.line_to(((ox+6.0)*s+x, oy*s+y)); p.line_to(((ox+6.0)*s+x, (oy+6.0)*s+y)); p.line_to((ox*s+x, (oy+6.0)*s+y)); p.close_path(); add(p, stroke.clone());
            }
        }
        Icon::Play => {
            let mut p = BezPath::new(); p.move_to((6.0*s+x, 4.0*s+y)); p.line_to((20.0*s+x, 12.0*s+y)); p.line_to((6.0*s+x, 20.0*s+y)); p.close_path(); add(p, stroke.clone());
        }
        Icon::X => {
            let mut p1 = BezPath::new(); p1.move_to((6.0*s+x, 6.0*s+y)); p1.line_to((18.0*s+x, 18.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((18.0*s+x, 6.0*s+y)); p2.line_to((6.0*s+x, 18.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Check => {
            let mut p = BezPath::new(); p.move_to((5.0*s+x, 12.0*s+y)); p.line_to((10.0*s+x, 17.0*s+y)); p.line_to((19.0*s+x, 7.0*s+y)); add(p, stroke.clone());
        }
        Icon::Sparkles => {
            let mut p = BezPath::new(); p.move_to((12.0*s+x, 3.0*s+y)); p.line_to((13.5*s+x, 8.5*s+y)); p.line_to((19.0*s+x, 10.0*s+y)); p.line_to((13.5*s+x, 11.5*s+y)); p.line_to((12.0*s+x, 17.0*s+y)); p.line_to((10.5*s+x, 11.5*s+y)); p.line_to((5.0*s+x, 10.0*s+y)); p.line_to((10.5*s+x, 8.5*s+y)); p.close_path(); add(p, stroke.clone());
        }
        Icon::ArrowDownUp => {
            let mut p1 = BezPath::new(); p1.move_to((12.0*s+x, 3.0*s+y)); p1.line_to((12.0*s+x, 21.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((8.0*s+x, 7.0*s+y)); p2.line_to((12.0*s+x, 3.0*s+y)); p2.line_to((16.0*s+x, 7.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((8.0*s+x, 17.0*s+y)); p3.line_to((12.0*s+x, 21.0*s+y)); p3.line_to((16.0*s+x, 17.0*s+y)); add(p3, stroke.clone());
        }
        Icon::ArrowRightLeft => {
            let mut p1 = BezPath::new(); p1.move_to((3.0*s+x, 12.0*s+y)); p1.line_to((21.0*s+x, 12.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((7.0*s+x, 8.0*s+y)); p2.line_to((3.0*s+x, 12.0*s+y)); p2.line_to((7.0*s+x, 16.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((17.0*s+x, 8.0*s+y)); p3.line_to((21.0*s+x, 12.0*s+y)); p3.line_to((17.0*s+x, 16.0*s+y)); add(p3, stroke.clone());
        }
        Icon::ArrowDown01 | Icon::Triangle => {
            let mut p1 = BezPath::new(); p1.move_to((12.0*s+x, 4.0*s+y)); p1.line_to((12.0*s+x, 20.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((8.0*s+x, 16.0*s+y)); p2.line_to((12.0*s+x, 20.0*s+y)); p2.line_to((16.0*s+x, 16.0*s+y)); add(p2, stroke.clone());
        }
        Icon::ArrowLeftRight => {
            let mut p = BezPath::new(); p.move_to((8.0*s+x, 12.0*s+y)); p.line_to((16.0*s+x, 12.0*s+y)); add(p, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((12.0*s+x, 8.0*s+y)); p2.line_to((8.0*s+x, 12.0*s+y)); p2.line_to((12.0*s+x, 16.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((12.0*s+x, 8.0*s+y)); p3.line_to((16.0*s+x, 12.0*s+y)); p3.line_to((12.0*s+x, 16.0*s+y)); add(p3, stroke.clone());
        }
        Icon::ArrowUpDown => {
            let mut p = BezPath::new(); p.move_to((12.0*s+x, 8.0*s+y)); p.line_to((12.0*s+x, 16.0*s+y)); add(p, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((8.0*s+x, 12.0*s+y)); p2.line_to((12.0*s+x, 8.0*s+y)); p2.line_to((16.0*s+x, 12.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((8.0*s+x, 12.0*s+y)); p3.line_to((12.0*s+x, 16.0*s+y)); p3.line_to((16.0*s+x, 12.0*s+y)); add(p3, stroke.clone());
        }
        Icon::Maximize => {
            let mut p1 = BezPath::new(); p1.move_to((8.0*s+x, 3.0*s+y)); p1.line_to((3.0*s+x, 3.0*s+y)); p1.line_to((3.0*s+x, 8.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((21.0*s+x, 8.0*s+y)); p2.line_to((21.0*s+x, 3.0*s+y)); p2.line_to((16.0*s+x, 3.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((3.0*s+x, 16.0*s+y)); p3.line_to((3.0*s+x, 21.0*s+y)); p3.line_to((8.0*s+x, 21.0*s+y)); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((16.0*s+x, 21.0*s+y)); p4.line_to((21.0*s+x, 21.0*s+y)); p4.line_to((21.0*s+x, 16.0*s+y)); add(p4, stroke.clone());
        }
        Icon::Lock => {
            let mut p1 = BezPath::new(); p1.move_to((7.0*s+x, 11.0*s+y)); p1.line_to((17.0*s+x, 11.0*s+y)); p1.line_to((17.0*s+x, 20.0*s+y)); p1.line_to((7.0*s+x, 20.0*s+y)); p1.close_path(); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((9.0*s+x, 11.0*s+y)); p2.line_to((9.0*s+x, 8.0*s+y)); p2.curve_to((9.0*s+x, 6.34*s+y), (10.34*s+x, 5.0*s+y), (12.0*s+x, 5.0*s+y)); p2.curve_to((13.66*s+x, 5.0*s+y), (15.0*s+x, 6.34*s+y), (15.0*s+x, 8.0*s+y)); p2.line_to((15.0*s+x, 11.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Unlock => {
            let mut p1 = BezPath::new(); p1.move_to((7.0*s+x, 11.0*s+y)); p1.line_to((17.0*s+x, 11.0*s+y)); p1.line_to((17.0*s+x, 20.0*s+y)); p1.line_to((7.0*s+x, 20.0*s+y)); p1.close_path(); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((9.0*s+x, 11.0*s+y)); p2.line_to((9.0*s+x, 8.0*s+y)); p2.curve_to((9.0*s+x, 6.34*s+y), (10.34*s+x, 5.0*s+y), (12.0*s+x, 5.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Eye => {
            let mut p1 = BezPath::new();
            p1.move_to((2.0*s+x, 12.0*s+y));
            p1.curve_to((5.0*s+x, 6.0*s+y), (9.0*s+x, 3.0*s+y), (12.0*s+x, 3.0*s+y));
            p1.curve_to((15.0*s+x, 3.0*s+y), (19.0*s+x, 6.0*s+y), (22.0*s+x, 12.0*s+y));
            p1.curve_to((19.0*s+x, 18.0*s+y), (15.0*s+x, 21.0*s+y), (12.0*s+x, 21.0*s+y));
            p1.curve_to((9.0*s+x, 21.0*s+y), (5.0*s+x, 18.0*s+y), (2.0*s+x, 12.0*s+y));
            p1.close_path(); add(p1, stroke.clone());
            let mut p2 = BezPath::new();
            p2.move_to((12.0*s+x, 15.5*s+y));
            p2.curve_to((13.9*s+x, 15.5*s+y), (15.5*s+x, 13.9*s+y), (15.5*s+x, 12.0*s+y));
            p2.curve_to((15.5*s+x, 10.1*s+y), (13.9*s+x, 8.5*s+y), (12.0*s+x, 8.5*s+y));
            p2.curve_to((10.1*s+x, 8.5*s+y), (8.5*s+x, 10.1*s+y), (8.5*s+x, 12.0*s+y));
            p2.curve_to((8.5*s+x, 13.9*s+y), (10.1*s+x, 15.5*s+y), (12.0*s+x, 15.5*s+y));
            p2.close_path(); add(p2, stroke.clone());
        }
        Icon::EyeOff => {
            let mut p1 = BezPath::new();
            p1.move_to((3.0*s+x, 3.0*s+y)); p1.line_to((21.0*s+x, 21.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new();
            p2.move_to((10.5*s+x, 10.5*s+y));
            p2.curve_to((14.0*s+x, 7.0*s+y), (18.0*s+x, 7.0*s+y), (22.0*s+x, 12.0*s+y));
            add(p2, stroke.clone());
        }
        Icon::Distribute => {
            let mut p1 = BezPath::new(); p1.move_to((3.0*s+x, 12.0*s+y)); p1.line_to((21.0*s+x, 12.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((7.0*s+x, 5.0*s+y)); p2.line_to((7.0*s+x, 19.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((17.0*s+x, 5.0*s+y)); p3.line_to((17.0*s+x, 19.0*s+y)); add(p3, stroke.clone());
        }
        Icon::Command => {
            let mut p = BezPath::new(); p.move_to((8.0*s+x, 8.0*s+y)); p.line_to((16.0*s+x, 8.0*s+y)); p.line_to((16.0*s+x, 16.0*s+y)); p.line_to((8.0*s+x, 16.0*s+y)); p.close_path(); add(p, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((12.0*s+x, 4.0*s+y)); p2.line_to((12.0*s+x, 8.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((12.0*s+x, 16.0*s+y)); p3.line_to((12.0*s+x, 20.0*s+y)); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((4.0*s+x, 12.0*s+y)); p4.line_to((8.0*s+x, 12.0*s+y)); add(p4, stroke.clone());
            let mut p5 = BezPath::new(); p5.move_to((16.0*s+x, 12.0*s+y)); p5.line_to((20.0*s+x, 12.0*s+y)); add(p5, stroke.clone());
        }
        Icon::AlignLeft => {
            let mut p1 = BezPath::new(); p1.move_to((3.0*s+x, 4.0*s+y)); p1.line_to((3.0*s+x, 20.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((7.0*s+x, 7.0*s+y)); p2.line_to((21.0*s+x, 7.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((7.0*s+x, 12.0*s+y)); p3.line_to((15.0*s+x, 12.0*s+y)); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((7.0*s+x, 17.0*s+y)); p4.line_to((21.0*s+x, 17.0*s+y)); add(p4, stroke.clone());
        }
        Icon::AlignCenter => {
            let mut p1 = BezPath::new(); p1.move_to((12.0*s+x, 4.0*s+y)); p1.line_to((12.0*s+x, 20.0*s+y)); add(p1, thin.clone());
            let mut p2 = BezPath::new(); p2.move_to((5.0*s+x, 7.0*s+y)); p2.line_to((19.0*s+x, 7.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((8.0*s+x, 12.0*s+y)); p3.line_to((16.0*s+x, 12.0*s+y)); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((5.0*s+x, 17.0*s+y)); p4.line_to((19.0*s+x, 17.0*s+y)); add(p4, stroke.clone());
        }
        Icon::AlignRight => {
            let mut p1 = BezPath::new(); p1.move_to((21.0*s+x, 4.0*s+y)); p1.line_to((21.0*s+x, 20.0*s+y)); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((3.0*s+x, 7.0*s+y)); p2.line_to((17.0*s+x, 7.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((9.0*s+x, 12.0*s+y)); p3.line_to((17.0*s+x, 12.0*s+y)); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((3.0*s+x, 17.0*s+y)); p4.line_to((17.0*s+x, 17.0*s+y)); add(p4, stroke.clone());
        }
        Icon::AlignJustify => {
            for y0 in [7.0, 12.0, 17.0] {
                let mut p = BezPath::new(); p.move_to((3.0*s+x, y0*s+y)); p.line_to((21.0*s+x, y0*s+y)); add(p, stroke.clone());
            }
        }
        Icon::Layers => {
            let mut p1 = BezPath::new(); p1.move_to((12.0*s+x, 2.0*s+y)); p1.line_to((22.0*s+x, 7.0*s+y)); p1.line_to((12.0*s+x, 12.0*s+y)); p1.line_to((2.0*s+x, 7.0*s+y)); p1.close_path(); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((2.0*s+x, 12.0*s+y)); p2.line_to((12.0*s+x, 17.0*s+y)); p2.line_to((22.0*s+x, 12.0*s+y)); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((2.0*s+x, 17.0*s+y)); p3.line_to((12.0*s+x, 22.0*s+y)); p3.line_to((22.0*s+x, 17.0*s+y)); add(p3, stroke.clone());
        }
        Icon::File => {
            let mut p = BezPath::new(); p.move_to((7.0*s+x, 3.0*s+y)); p.line_to((15.0*s+x, 3.0*s+y)); p.line_to((19.0*s+x, 7.0*s+y)); p.line_to((19.0*s+x, 21.0*s+y)); p.line_to((7.0*s+x, 21.0*s+y)); p.close_path(); add(p, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((15.0*s+x, 3.0*s+y)); p2.line_to((15.0*s+x, 7.0*s+y)); p2.line_to((19.0*s+x, 7.0*s+y)); add(p2, stroke.clone());
        }
        Icon::Folder => {
            let mut p = BezPath::new(); p.move_to((3.0*s+x, 7.0*s+y)); p.line_to((3.0*s+x, 20.0*s+y)); p.line_to((21.0*s+x, 20.0*s+y)); p.line_to((21.0*s+x, 7.0*s+y)); p.close_path(); add(p, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((3.0*s+x, 7.0*s+y)); p2.line_to((9.0*s+x, 7.0*s+y)); p2.line_to((11.0*s+x, 4.0*s+y)); p2.line_to((21.0*s+x, 4.0*s+y)); add(p2, stroke.clone());
        }
        Icon::More => {
            for cx in [6.0, 12.0, 18.0] {
                let mut p = BezPath::new(); p.move_to((cx*s+x, 12.0*s+y)); p.line_to((cx*s+x, 12.0*s+y)); add(p, stroke.clone());
            }
        }
        Icon::Rotate | Icon::Corner => {
            let mut p = BezPath::new(); p.move_to((12.0*s+x, 3.0*s+y)); p.line_to((12.0*s+x, 3.0*s+y)); add(p, stroke.clone());
            let mut p2 = BezPath::new();
            p2.move_to((5.0*s+x, 8.0*s+y));
            p2.curve_to((5.0*s+x, 4.0*s+y), (8.0*s+x, 3.0*s+y), (12.0*s+x, 3.0*s+y));
            p2.curve_to((16.0*s+x, 3.0*s+y), (19.0*s+x, 6.0*s+y), (19.0*s+x, 12.0*s+y));
            add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((19.0*s+x, 12.0*s+y)); p3.line_to((15.0*s+x, 9.0*s+y)); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((19.0*s+x, 12.0*s+y)); p4.line_to((15.0*s+x, 15.0*s+y)); add(p4, stroke.clone());
        }
        Icon::Component => {
            let mut p = BezPath::new(); p.move_to((3.0*s+x, 9.0*s+y)); p.line_to((3.0*s+x, 9.0*s+y)); add(p, stroke.clone());
            let mut p1 = BezPath::new(); p1.move_to((3.0*s+x, 3.0*s+y)); p1.line_to((9.0*s+x, 3.0*s+y)); p1.line_to((9.0*s+x, 9.0*s+y)); p1.line_to((3.0*s+x, 9.0*s+y)); p1.close_path(); add(p1, stroke.clone());
            let mut p2 = BezPath::new(); p2.move_to((15.0*s+x, 3.0*s+y)); p2.line_to((21.0*s+x, 3.0*s+y)); p2.line_to((21.0*s+x, 9.0*s+y)); p2.line_to((15.0*s+x, 9.0*s+y)); p2.close_path(); add(p2, stroke.clone());
            let mut p3 = BezPath::new(); p3.move_to((3.0*s+x, 15.0*s+y)); p3.line_to((9.0*s+x, 15.0*s+y)); p3.line_to((9.0*s+x, 21.0*s+y)); p3.line_to((3.0*s+x, 21.0*s+y)); p3.close_path(); add(p3, stroke.clone());
            let mut p4 = BezPath::new(); p4.move_to((15.0*s+x, 15.0*s+y)); p4.line_to((21.0*s+x, 15.0*s+y)); p4.line_to((21.0*s+x, 21.0*s+y)); p4.line_to((15.0*s+x, 21.0*s+y)); p4.close_path(); add(p4, stroke.clone());
        }
        _ => {
            let mut p = BezPath::new(); p.move_to((6.0*s+x, 6.0*s+y)); p.line_to((18.0*s+x, 6.0*s+y)); p.line_to((18.0*s+x, 18.0*s+y)); p.line_to((6.0*s+x, 18.0*s+y)); p.close_path(); add(p, stroke.clone());
        }
    }

    for (p, st) in paths {
        scene.stroke(&st, Affine::IDENTITY, color, None, &p);
    }
}

pub fn draw_dropdown_chevron(scene: &mut Scene, x: f64, y: f64, color: Color) {
    let s = 12.0 / 24.0;
    let stroke = KStroke::new(1.2 * s).with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round);
    let mut p = BezPath::new();
    p.move_to((4.0*s+x, 6.0*s+y));
    p.line_to((8.0*s+x, 10.0*s+y));
    p.line_to((12.0*s+x, 6.0*s+y));
    scene.stroke(&stroke, Affine::IDENTITY, color, None, &p);
}

pub fn draw_chevron_right(scene: &mut Scene, x: f64, y: f64, size: f64, color: Color) {
    let s = size / 24.0;
    let stroke = KStroke::new(1.5 * s).with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round);
    let mut p = BezPath::new();
    p.move_to((9.0*s+x, 6.0*s+y));
    p.line_to((15.0*s+x, 12.0*s+y));
    p.line_to((9.0*s+x, 18.0*s+y));
    scene.stroke(&stroke, Affine::IDENTITY, color, None, &p);
}
