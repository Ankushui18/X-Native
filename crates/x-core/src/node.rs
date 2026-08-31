use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------------- nodes

/// Phase 2.6: editable vector path data. A vector node owns a list of
/// subpath commands in local coordinates — the pen tool's data model.
/// Rendered as a real filled (and optionally stroked) Vello path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCmd {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    /// Cubic bezier: control1, control2, endpoint.
    CurveTo(f64, f64, f64, f64, f64, f64),
    Close,
}

pub fn path_to_bez(cmds: &[PathCmd]) -> vello::kurbo::BezPath {
    let mut p = vello::kurbo::BezPath::new();
    for c in cmds {
        match *c {
            PathCmd::MoveTo(x, y) => p.move_to((x, y)),
            PathCmd::LineTo(x, y) => p.line_to((x, y)),
            PathCmd::CurveTo(x1, y1, x2, y2, x, y) => p.curve_to((x1, y1), (x2, y2), (x, y)),
            PathCmd::Close => p.close_path(),
        }
    }
    p
}

/// Designer-facing image placement on top of the fit mode: crop focal
/// point (which part of the image stays visible), extra zoom, and flips.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePlacement {
    /// crop focal point in 0..1 image space; (0.5, 0.5) = center
    pub focal: (f64, f64),
    /// extra zoom multiplier on top of the fit scale (1.0 = none)
    pub scale: f64,
    pub flip_h: bool,
    pub flip_v: bool,
}
impl Default for ImagePlacement {
    fn default() -> Self { Self { focal: (0.5, 0.5), scale: 1.0, flip_h: false, flip_v: false } }
}
impl ImagePlacement {
    pub fn is_default(&self) -> bool { *self == Self::default() }
}

/// Image fill behavior inside the node's box (Figma's fill modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFit {
    /// stretch to the box (aspect ignored)
    #[default]
    Fill,
    /// contain: fit inside, letterboxed
    Fit,
    /// cover: fill the box, cropped, centered
    Crop,
    /// natural size, positioned at offset
    Tile,
}

/// Phase P0: Text metrics for hit-testing and selection
#[derive(Debug, Clone, Default)]
pub struct TextMetrics {
    pub font_size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub max_width: f64,
    pub actual_width: f64,
    pub actual_height: f64,
    pub line_count: usize,
    pub caret_positions: Vec<(usize, f64, f64)>, // (char_index, x, y) in local coords
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointType {
    Corner,
    Smooth,
    Mirror,
    Auto,
}

/// Phase P0: Vector network point with bezier handles
#[derive(Debug, Clone, PartialEq)]
pub struct VectorPoint {
    pub id: usize,
    pub position: (f64, f64),
    pub incoming: Option<(f64, f64)>, // bezier handle relative to position
    pub outgoing: Option<(f64, f64)>,
    pub point_type: PointType,
}

/// Phase P0: Segment connecting vector points
#[derive(Debug, Clone, PartialEq)]
pub struct VectorSegment {
    pub start_point_id: usize,
    pub end_point_id: usize,
    pub stroke_width: f64,
    pub stroke_color: Color,
}

/// Phase P0: Vector network data structure
#[derive(Debug, Clone, PartialEq)]
pub struct VectorNetwork {
    pub points: Vec<VectorPoint>,
    pub segments: Vec<VectorSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Frame { layout: Option<AutoLayout> },
    Group,
    Rect { radius: f64 },
    Ellipse,
    Line,
    Text { text: String },
    Image { asset: String, fit: ImageFit, placement: ImagePlacement },
    Vector { path: Vec<PathCmd> },
    VectorNetwork(VectorNetwork),
    Component { name: String },
    Instance { component: String },
}

#[derive(Debug, Clone, PartialEq)] pub struct PrototypeAction { pub destination: String, pub transition_ms: u32 }

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub w: f64,
    pub h: f64,
    pub transform: Transform,
    pub fill: Paint,
    pub stroke: Stroke,
    /// Ordered visual stacks. Empty means a legacy node and falls back to
    /// `fill`, `stroke`, and `effects`; this keeps old `.x` documents valid.
    pub fill_layers: Vec<PaintLayer>,
    pub stroke_layers: Vec<StrokeLayer>,
    pub effect_layers: Vec<EffectLayer>,
    pub visual_stacks_materialized: bool,
    pub opacity: f32,
    pub children: Vec<Node>,
    pub dirty: bool,
    pub visible: bool,
    /// Phase 2: editor lock (excluded from hit testing).
    pub locked: bool,
    pub prototype: Option<PrototypeAction>,
    pub overrides: HashMap<String, String>,
    /// Phase 4.7: per-corner radii [tl, tr, br, bl]; overrides Rect's uniform radius.
    pub corner_radii: Option<[f64; 4]>,
    /// Phase 4: blend mode.
    pub blend: BlendKind,
    /// Phase 4: layer effects (shadows/blurs).
    pub effects: Vec<Effect>,
    /// Phase 2.12: resize constraints relative to the parent frame.
    pub pin: (HPin, VPin),
    /// Masks: when true, this node clips its FOLLOWING SIBLINGS inside
    /// the same parent (Figma "use as mask" semantics, simplified).
    pub is_mask: bool,
    /// P1: variable bindings — property -> variable name.
    /// Supported keys: "radius", "opacity", "fontsize", "w", "h".
    /// ("fill" binds via Paint::Variable; gap/padding via AutoLayout vars.)
    pub bindings: HashMap<String, String>,
    /// Phase P0: text metrics for selection and hit-testing
    pub text_metrics: Option<TextMetrics>,
}

impl Node {
    fn base(id: &str, kind: NodeKind, x: f64, y: f64, w: f64, h: f64, fill: Paint) -> Self {
        Self {
            id: id.into(), kind, w, h,
            transform: Transform { x, y, ..Default::default() },
            fill, stroke: Stroke::default(), fill_layers: vec![], stroke_layers: vec![], effect_layers: vec![], visual_stacks_materialized: false, opacity: 1.0,
            children: vec![], dirty: true, visible: true, locked: false,
            prototype: None, overrides: HashMap::new(),
            corner_radii: None, blend: BlendKind::Normal, effects: vec![],
            is_mask: false,
            pin: (HPin::Left, VPin::Top),
            bindings: HashMap::new(),
            text_metrics: None,
        }
    }
    pub fn frame(id: &str, w: f64, h: f64) -> Self { Self::base(id, NodeKind::Frame { layout: None }, 0.0, 0.0, w, h, Paint::Solid(Color::TRANSPARENT)) }
    pub fn group(id: &str, w: f64, h: f64) -> Self { Self::base(id, NodeKind::Group, 0.0, 0.0, w, h, Paint::Solid(Color::TRANSPARENT)) }
    pub fn rect(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Color) -> Self { Self::base(id, NodeKind::Rect { radius: 0.0 }, x, y, w, h, Paint::Solid(fill)) }
    pub fn ellipse(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Color) -> Self { Self::base(id, NodeKind::Ellipse, x, y, w, h, Paint::Solid(fill)) }
    pub fn line(id: &str, x: f64, y: f64, w: f64, h: f64, color: Color) -> Self { Self::base(id, NodeKind::Line, x, y, w, h, Paint::Solid(Color::TRANSPARENT)).stroke(Stroke { color, width: 2.0 }) }
    pub fn text(id: &str, x: f64, y: f64, w: f64, h: f64, text: &str) -> Self { Self::base(id, NodeKind::Text { text: text.into() }, x, y, w, h, Paint::Solid(Color::BLACK)) }
    pub fn image(id: &str, x: f64, y: f64, w: f64, h: f64, asset: &str) -> Self { Self::base(id, NodeKind::Image { asset: asset.into(), fit: ImageFit::default(), placement: ImagePlacement::default() }, x, y, w, h, Paint::Solid(Color::rgb8(0xdd, 0xdd, 0xdd))) }
    pub fn vector(id: &str, x: f64, y: f64, w: f64, h: f64, path: Vec<PathCmd>) -> Self { Self::base(id, NodeKind::Vector { path }, x, y, w, h, Paint::Solid(Color::BLACK)) }
    pub fn component(id: &str, name: &str, w: f64, h: f64) -> Self { Self::base(id, NodeKind::Component { name: name.into() }, 0.0, 0.0, w, h, Paint::Solid(Color::TRANSPARENT)) }
    pub fn instance(id: &str, component: &str, x: f64, y: f64, w: f64, h: f64) -> Self { Self::base(id, NodeKind::Instance { component: component.into() }, x, y, w, h, Paint::Solid(Color::TRANSPARENT)) }

    pub fn radius(mut self, r: f64) -> Self { if let NodeKind::Rect { .. } = self.kind { self.kind = NodeKind::Rect { radius: r } } self }
    pub fn corners(mut self, tl: f64, tr: f64, br: f64, bl: f64) -> Self { self.corner_radii = Some([tl, tr, br, bl]); self }
    pub fn rotate(mut self, r: f64) -> Self { self.transform.rotation = r; self }
    pub fn scale(mut self, x: f64, y: f64) -> Self { self.transform.scale_x = x; self.transform.scale_y = y; self }
    pub fn opacity(mut self, v: f32) -> Self { self.opacity = v.clamp(0.0, 1.0); self }
    pub fn stroke(mut self, s: Stroke) -> Self { self.stroke = s; self }
    pub fn fill_paint(mut self, p: Paint) -> Self { self.fill = p; self }
    pub fn blend(mut self, b: BlendKind) -> Self { self.blend = b; self }
    pub fn effect(mut self, e: Effect) -> Self { self.effects.push(e); self }

    pub fn materialize_visual_stacks(&mut self) {
        if self.visual_stacks_materialized { return; }
        if self.fill_layers.is_empty() { self.fill_layers.push(PaintLayer::new(self.fill.clone())); }
        if self.stroke_layers.is_empty() && self.stroke.width > 0.0 { self.stroke_layers.push(StrokeLayer::new(self.stroke)); }
        if self.effect_layers.is_empty() { self.effect_layers = self.effects.iter().cloned().map(EffectLayer::new).collect(); }
        self.visual_stacks_materialized = true;
    }

    pub fn active_fills(&self) -> Vec<PaintLayer> {
        if !self.visual_stacks_materialized { vec![PaintLayer::new(self.fill.clone())] }
        else { self.fill_layers.iter().filter(|l| l.visible && l.opacity > 0.0).cloned().collect() }
    }
    pub fn active_strokes(&self) -> Vec<StrokeLayer> {
        if !self.visual_stacks_materialized {
            if self.stroke.width > 0.0 { vec![StrokeLayer::new(self.stroke)] } else { vec![] }
        } else { self.stroke_layers.iter().filter(|l| l.visible && l.opacity > 0.0 && l.stroke.width > 0.0).cloned().collect() }
    }
    pub fn active_effects(&self) -> Vec<EffectLayer> {
        if !self.visual_stacks_materialized { self.effects.iter().cloned().map(EffectLayer::new).collect() }
        else { self.effect_layers.iter().filter(|l| l.visible && l.opacity > 0.0).cloned().collect() }
    }
    pub fn pin(mut self, h: HPin, v: VPin) -> Self { self.pin = (h, v); self }
    pub fn locked(mut self, v: bool) -> Self { self.locked = v; self }
    pub fn mask(mut self, v: bool) -> Self { self.is_mask = v; self }
    pub fn child(mut self, n: Node) -> Self { self.children.push(n); self.dirty = true; self }
    pub fn prototype(mut self, destination: &str, transition_ms: u32) -> Self { self.prototype = Some(PrototypeAction { destination: destination.into(), transition_ms }); self }
    pub fn override_prop(mut self, key: &str, value: &str) -> Self { self.overrides.insert(key.into(), value.into()); self }
    pub fn auto_layout(mut self, layout: AutoLayout) -> Self { if let NodeKind::Frame { .. } = self.kind { self.kind = NodeKind::Frame { layout: Some(layout) } } self }
    /// Bind a property ("radius"/"opacity"/"fontsize"/"w"/"h") to a number variable.
    pub fn bind(mut self, prop: &str, var: &str) -> Self { self.bindings.insert(prop.into(), var.into()); self }

    /// Resolve a bound numeric property against `vars`, else `fallback`.
    pub fn bound_number(&self, prop: &str, vars: &Variables, fallback: f64) -> f64 {
        match self.bindings.get(prop) {
            Some(name) => vars.number(name, fallback),
            None => fallback,
        }
    }
}
