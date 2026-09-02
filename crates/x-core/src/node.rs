use std::collections::HashMap;
use kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use peniko::{Brush, Color, Fill, Gradient, Mix};
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

pub fn path_to_bez(cmds: &[PathCmd]) -> kurbo::BezPath {
    let mut p = kurbo::BezPath::new();
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

/// Image fill behavior inside the node's box (fill modes).
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

// NOTE: the Phase-P0 `VectorNetwork` experiment (VectorPoint/VectorSegment/
// PointType + the NodeKind::VectorNetwork variant) was removed 2026-09-02:
// it was never constructed anywhere (no importer, no editor op, no test)
// and every renderer carried a TODO for it. Vector paths are served by
// NodeKind::Vector. The .x deserializer never had a "vector_network" case
// (unknown tags load as frames), so no file-format compatibility is lost.

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
    /// the same parent (mask semantics semantics, simplified).
    pub is_mask: bool,
    /// P1: variable bindings — property -> variable name.
    /// Supported keys: "radius", "opacity", "fontsize", "w", "h".
    /// ("fill" binds via Paint::Variable; gap/padding via AutoLayout vars.)
    pub bindings: HashMap<String, String>,
    /// Phase P0: text metrics for selection and hit-testing
    pub text_metrics: Option<TextMetrics>,
    /// Rich text: styled sub-ranges of a Text node's string (CHAR-index
    /// based: start/len into the text's char vector). Empty = plain text.
    /// Only applies to Text nodes (like corner_radii only applies to
    /// Rects). Editing the text clears these — ranges would be stale.
    pub text_runs: Vec<TextRun>,
}

/// A styled sub-range of a Text node's string. `start`/`len` are CHAR
/// indices; out-of-range parts are ignored by the resolver (hostile or
/// hand-edited files can never panic the renderer).
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub start: usize,
    pub len: usize,
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub font: Option<String>,
}

/// One resolved styled chunk of a Text node (renderer/sink facing).
#[derive(Debug, Clone, PartialEq)]
pub struct TextPart {
    pub text: String,
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub font: Option<String>,
}

/// Split a Text node's string into styled parts. Unstyled ranges produce
/// parts with all-None styling; runs are clipped to the text; overlapping
/// runs are applied in order (last wins per range). Empty/absent runs
/// degrade to a single plain part.
pub fn resolve_text_parts(text: &str, runs: &[TextRun]) -> Vec<TextPart> {
    let total = text.chars().count();
    if runs.is_empty() || total == 0 { return vec![TextPart { text: text.to_string(), color: None, size: None, font: None }]; }
    // char-index -> style lookup, last run wins
    let mut at: Vec<Option<usize>> = vec![None; total];
    for (i, r) in runs.iter().enumerate() {
        let end = r.start.saturating_add(r.len).min(total);
        for a in &mut at[r.start.min(total)..end] {
            *a = Some(i);
        }
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<TextPart> = vec![];
    let mut i = 0;
    while i < total {
        let style = at[i];
        let mut j = i + 1;
        while j < total && at[j] == style { j += 1; }
        let (color, size, font) = match style.and_then(|k| runs.get(k)) {
            Some(r) => (r.color, r.size, r.font.clone()),
            None => (None, None, None),
        };
        out.push(TextPart { text: chars[i..j].iter().collect(), color, size, font });
        i = j;
    }
    if out.is_empty() { out.push(TextPart { text: text.to_string(), color: None, size: None, font: None }); }
    out
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
            text_runs: vec![],
        }
    }
    pub fn frame(id: &str, w: f64, h: f64) -> Self { Self::base(id, NodeKind::Frame { layout: None }, 0.0, 0.0, w, h, Paint::Solid(Color::TRANSPARENT)) }
    pub fn group(id: &str, w: f64, h: f64) -> Self { Self::base(id, NodeKind::Group, 0.0, 0.0, w, h, Paint::Solid(Color::TRANSPARENT)) }
    pub fn rect(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Color) -> Self { Self::base(id, NodeKind::Rect { radius: 0.0 }, x, y, w, h, Paint::Solid(fill)) }
    pub fn ellipse(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Color) -> Self { Self::base(id, NodeKind::Ellipse, x, y, w, h, Paint::Solid(fill)) }
    pub fn line(id: &str, x: f64, y: f64, w: f64, h: f64, color: Color) -> Self { Self::base(id, NodeKind::Line, x, y, w, h, Paint::Solid(Color::TRANSPARENT)).stroke(Stroke::solid(color, 2.0)) }
    pub fn text(id: &str, x: f64, y: f64, w: f64, h: f64, text: &str) -> Self { Self::base(id, NodeKind::Text { text: text.into() }, x, y, w, h, Paint::Solid(Color::BLACK)) }
    pub fn image(id: &str, x: f64, y: f64, w: f64, h: f64, asset: &str) -> Self { Self::base(id, NodeKind::Image { asset: asset.into(), fit: ImageFit::default(), placement: ImagePlacement::default() }, x, y, w, h, Paint::Solid(Color::from_rgb8(0xdd, 0xdd, 0xdd))) }
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
        if self.stroke_layers.is_empty() && self.stroke.width > 0.0 { self.stroke_layers.push(StrokeLayer::new(self.stroke.clone())); }
        if self.effect_layers.is_empty() { self.effect_layers = self.effects.iter().cloned().map(EffectLayer::new).collect(); }
        self.visual_stacks_materialized = true;
    }

    pub fn active_fills(&self) -> Vec<PaintLayer> {
        if !self.visual_stacks_materialized { vec![PaintLayer::new(self.fill.clone())] }
        else { self.fill_layers.iter().filter(|l| l.visible && l.opacity > 0.0).cloned().collect() }
    }
    pub fn active_strokes(&self) -> Vec<StrokeLayer> {
        if !self.visual_stacks_materialized {
            if self.stroke.width > 0.0 { vec![StrokeLayer::new(self.stroke.clone())] } else { vec![] }
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

#[cfg(test)]
mod text_run_tests {
    use super::*;

    fn part(text: &str, color: Option<Color>, size: Option<f64>) -> TextPart {
        TextPart { text: text.into(), color, size, font: None }
    }

    #[test]
    fn no_runs_degrades_to_a_single_plain_part() {
        let parts = resolve_text_parts("Hello", &[]);
        assert_eq!(parts, vec![part("Hello", None, None)]);
    }

    #[test]
    fn empty_text_degrades_to_a_single_plain_part() {
        let parts = resolve_text_parts("", &[TextRun { start: 0, len: 5, color: Some(Color::from_rgb8(255, 0, 0)), size: None, font: None }]);
        assert_eq!(parts, vec![part("", None, None)]);
    }

    #[test]
    fn styled_range_splits_into_three_parts() {
        // "hello": chars 1..3 styled red
        let runs = [TextRun { start: 1, len: 2, color: Some(Color::from_rgb8(255, 0, 0)), size: Some(30.0), font: None }];
        let parts = resolve_text_parts("hello", &runs);
        assert_eq!(parts, vec![
            part("h", None, None),
            part("el", Some(Color::from_rgb8(255, 0, 0)), Some(30.0)),
            part("lo", None, None),
        ]);
    }

    #[test]
    fn out_of_range_runs_are_clamped_not_panicking() {
        // hostile/hand-edited files: run far past the end
        let runs = [TextRun { start: 10, len: 50, color: Some(Color::from_rgb8(255, 0, 0)), size: None, font: None }];
        let parts = resolve_text_parts("abc", &runs);
        // range fully outside -> plain
        assert_eq!(parts, vec![part("abc", None, None)]);
        // partially outside clips to the text end
        let runs = [TextRun { start: 2, len: 50, color: Some(Color::from_rgb8(255, 0, 0)), size: None, font: None }];
        let parts = resolve_text_parts("abc", &runs);
        assert_eq!(parts, vec![part("ab", None, None), part("c", Some(Color::from_rgb8(255, 0, 0)), None)]);
    }

    #[test]
    fn overlapping_runs_last_wins() {
        let runs = [
            TextRun { start: 0, len: 4, color: Some(Color::from_rgb8(255, 0, 0)), size: None, font: None },
            TextRun { start: 2, len: 2, color: Some(Color::from_rgb8(0, 0, 255)), size: None, font: None },
        ];
        let parts = resolve_text_parts("abcd", &runs);
        assert_eq!(parts, vec![
            part("ab", Some(Color::from_rgb8(255, 0, 0)), None),
            part("cd", Some(Color::from_rgb8(0, 0, 255)), None),
        ]);
    }

    #[test]
    fn whole_text_run_covers_every_char() {
        let runs = [TextRun { start: 0, len: 5, color: Some(Color::from_rgb8(255, 0, 0)), size: None, font: None }];
        let parts = resolve_text_parts("hello", &runs);
        assert_eq!(parts, vec![part("hello", Some(Color::from_rgb8(255, 0, 0)), None)]);
    }
}
