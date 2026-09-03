#[allow(unused_imports)]
use crate::*;
use kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use peniko::{Brush, Color, Fill, Gradient, Mix};
use std::collections::HashMap;

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

/// Ramer-Douglas-Peucker polyline simplification (pencil tool): drops
/// points whose perpendicular deviation from the first-last chord is
/// within `eps`. Keeps at least the endpoints.
pub fn simplify_polyline(pts: &[(f64, f64)], eps: f64) -> Vec<(f64, f64)> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let (x0, y0) = pts[0];
    let (x1, y1) = *pts.last().unwrap();
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = dx.hypot(dy);
    let mut max_d = 0.0f64;
    let mut idx = 0usize;
    for (i, (x, y)) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = if len < 1e-12 {
            (x - x0).hypot(y - y0)
        } else {
            (dy * (x - x0) - dx * (y - y0)).abs() / len
        };
        if d > max_d {
            max_d = d;
            idx = i;
        }
    }
    if max_d > eps {
        let mut left = simplify_polyline(&pts[..=idx], eps);
        let right = simplify_polyline(&pts[idx..], eps);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![pts[0], *pts.last().unwrap()]
    }
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
    fn default() -> Self {
        Self {
            focal: (0.5, 0.5),
            scale: 1.0,
            flip_h: false,
            flip_v: false,
        }
    }
}
impl ImagePlacement {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
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
    Frame {
        layout: Option<AutoLayout>,
    },
    Group,
    Rect {
        radius: f64,
    },
    Ellipse,
    /// Figma-style Section: a labelled container frame. The label is the
    /// node's `name`, drawn as a header by the renderer. Children render
    /// inside; behaves like a Frame for hit-testing/marquee/ungroup.
    Section,
    /// Elliptical arc: start/end angles in degrees (y-down space, 0 = east,
    /// increasing clockwise on screen). start == end means the full ellipse.
    Arc {
        start: f64,
        end: f64,
    },
    Line,
    Text {
        text: String,
    },
    Image {
        asset: String,
        fit: ImageFit,
        placement: ImagePlacement,
    },
    Vector {
        path: Vec<PathCmd>,
    },
    Component {
        name: String,
    },
    Instance {
        component: String,
    },
    /// Figma slice: an export region. Renders nothing itself (no fill, no
    /// stroke, no effects); exporting it captures the flattened canvas
    /// content inside its bounds. Slices are leaf nodes.
    Slice,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrototypeAction {
    pub destination: String,
    pub transition_ms: u32,
}

/// Per-node export preset (Figma's Export panel): a format/scale/quality/
/// suffix tuple. `Node.export_settings` is a list of these.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    pub format: String,
    pub scale: f64,
    pub quality: u8,
    pub suffix: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: "png".to_string(),
            scale: 1.0,
            quality: 90,
            suffix: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    /// Stable identity: the key every reference (prototype destinations,
    /// instance overrides, render keys, selection) points at. Never changes
    /// once a node exists — renaming a layer edits `name` instead, so
    /// references survive (Figma parity).
    pub id: String,
    /// User-facing display name. Independent of `id` (Figma separates name
    /// from identity); defaults to `id` for nodes created programmatically.
    pub name: String,
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
    /// Phase 2.12/P0: resize + per-child auto-layout constraints
    /// (absolute/fixed/sticky, align_self, grow/shrink/basis).
    pub constraints: ChildConstraints,
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
    /// Baseline offset (node-local px from the top edge to the first text
    /// baseline). Populated by the text pipeline from real font metrics;
    /// `None` falls back to a geometry heuristic in the auto-layout solver.
    pub baseline: Option<f64>,
    /// Component properties (Figma component properties) — meaningful only on
    /// Component masters; instances expose them as editable controls.
    pub props: Vec<ComponentProp>,
    /// Per-node export settings (Figma's Export panel): a list of
    /// format/scale/suffix presets. Exporting the node writes one file per
    /// entry. Empty means "no explicit exports" (the quick-format buttons
    /// still work). Most useful on slices, but any node may carry them.
    pub export_settings: Vec<ExportSettings>,
    /// Prototyping interactions (trigger → action). Rich Figma-parity model;
    /// the legacy `prototype` field above is kept only for old `.x` docs and
    /// is treated as an `OnClick → Navigate` interaction during playback.
    pub interactions: Vec<Interaction>,
    /// Flow starting point (Figma "starting frame" of a prototype flow).
    pub is_starting_point: bool,
    /// Clip/scroll behavior for a frame's overflowing content.
    pub overflow: Overflow,
    /// Current scroll offset (page px) for a scrollable frame.
    pub scroll: (f64, f64),
    /// Layout grid guides (Figma "layout grid"): visual column/row/grid
    /// overlays on a frame — guides, NOT auto layout. A frame may stack
    /// several (e.g. columns + rows). Meaningful only on Frame nodes.
    pub layout_grids: Vec<LayoutGridDef>,
}

impl Node {
    /// This node's paragraph wrap strategy (the "tw" binding; Text nodes).
    pub fn text_wrap(&self) -> TextWrap {
        TextWrap::parse(
            self.bindings
                .get("tw")
                .map(String::as_str)
                .unwrap_or("auto"),
        )
    }
}

/// Paragraph wrap strategy (Figma Aug-2026 text wrap). `Auto` is the
/// classic greedy first-fit; `Balance` evens line lengths per paragraph
/// (CSS `text-wrap: balance`); `Pretty` balances AND avoids a lone word
/// stranded on the last line (widows). Rides the node as the "tw"
/// binding, same as "ls"/"lh".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrap {
    #[default]
    Auto,
    Balance,
    Pretty,
}

impl TextWrap {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Balance => "balance",
            Self::Pretty => "pretty",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "balance" => Self::Balance,
            "pretty" => Self::Pretty,
            _ => Self::Auto,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Balance => "BALANCE",
            Self::Pretty => "PRETTY",
        }
    }
}

/// Layout-grid guide pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridPattern {
    #[default]
    Columns,
    Rows,
    /// Square grid of `LayoutGridDef::cell` px.
    Grid,
}

impl GridPattern {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Columns => "columns",
            Self::Rows => "rows",
            Self::Grid => "grid",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "rows" => Self::Rows,
            "grid" => Self::Grid,
            _ => Self::Columns,
        }
    }
}

/// One layout grid. Columns/Rows use `count`/`gutter`/`margin`; the Grid
/// pattern uses `cell` (square cell size) and ignores gutter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutGridDef {
    pub pattern: GridPattern,
    /// Column/row count (Columns/Rows).
    pub count: usize,
    /// Gap between columns/rows.
    pub gutter: f64,
    /// Outer margin: sides for Columns, top/bottom for Rows.
    pub margin: f64,
    /// Square cell size (Grid pattern only).
    pub cell: f64,
}

impl Default for LayoutGridDef {
    fn default() -> Self {
        Self {
            pattern: GridPattern::Columns,
            count: 12,
            gutter: 20.0,
            margin: 20.0,
            cell: 8.0,
        }
    }
}

impl LayoutGridDef {
    /// Guide bands for Columns/Rows, frame-local px: (x, y, w, h)
    /// rectangles to paint translucently. Empty for the Grid pattern.
    pub fn bands(&self, w: f64, h: f64) -> Vec<(f64, f64, f64, f64)> {
        match self.pattern {
            GridPattern::Grid => vec![],
            GridPattern::Columns => {
                let m = self.margin.clamp(0.0, w / 2.0);
                let inner = (w - 2.0 * m).max(0.0);
                let n = self.count.max(1);
                let g = self.gutter.clamp(0.0, inner / n as f64);
                let band = ((inner - g * (n - 1) as f64) / n as f64).max(0.0);
                (0..n)
                    .map(|i| (m + i as f64 * (band + g), 0.0, band, h))
                    .collect()
            }
            GridPattern::Rows => {
                let m = self.margin.clamp(0.0, h / 2.0);
                let inner = (h - 2.0 * m).max(0.0);
                let n = self.count.max(1);
                let g = self.gutter.clamp(0.0, inner / n as f64);
                let band = ((inner - g * (n - 1) as f64) / n as f64).max(0.0);
                (0..n)
                    .map(|i| (0.0, m + i as f64 * (band + g), w, band))
                    .collect()
            }
        }
    }

    /// Line positions for the square Grid pattern: (xs, ys), frame-local.
    pub fn grid_lines(&self, w: f64, h: f64) -> (Vec<f64>, Vec<f64>) {
        let step = self.cell.max(1.0);
        let upto = |len: f64| -> Vec<f64> {
            let mut v = vec![];
            let mut x = 0.0;
            while x <= len + 1e-9 {
                v.push(x);
                x += step;
            }
            v
        };
        (upto(w), upto(h))
    }
}

/// A styled sub-range of a Text node's string. `start`/`len` are CHAR
/// indices; out-of-range parts are ignored by the resolver (hostile or
/// hand-edited files can never panic the renderer).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextRun {
    pub start: usize,
    pub len: usize,
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub font: Option<String>,
    /// Font weight (400 normal, 700 bold…) — emitted by exports; shaping
    /// resolves through the font name when the family carries it.
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    /// Per-run letter-spacing override (px); None = node-level `ls` binding.
    pub ls: Option<f64>,
}

/// One resolved styled chunk of a Text node (renderer/sink facing).
#[derive(Debug, Clone, PartialEq)]
pub struct TextPart {
    pub text: String,
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub font: Option<String>,
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    pub ls: Option<f64>,
}

/// Split a Text node's string into styled parts. Unstyled ranges produce
/// parts with all-None styling; runs are clipped to the text; overlapping
/// runs are applied in order (last wins per range). Empty/absent runs
/// degrade to a single plain part.
pub fn plain_part(text: &str) -> TextPart {
    TextPart {
        text: text.to_string(),
        color: None,
        size: None,
        font: None,
        weight: None,
        italic: None,
        ls: None,
    }
}

pub fn resolve_text_parts(text: &str, runs: &[TextRun]) -> Vec<TextPart> {
    let total = text.chars().count();
    if runs.is_empty() || total == 0 {
        return vec![plain_part(text)];
    }
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
        while j < total && at[j] == style {
            j += 1;
        }
        let (color, size, font, weight, italic, ls) = match style.and_then(|k| runs.get(k)) {
            Some(r) => (r.color, r.size, r.font.clone(), r.weight, r.italic, r.ls),
            None => (None, None, None, None, None, None),
        };
        out.push(TextPart {
            text: chars[i..j].iter().collect(),
            color,
            size,
            font,
            weight,
            italic,
            ls,
        });
        i = j;
    }
    if out.is_empty() {
        out.push(plain_part(text));
    }
    out
}

impl Node {
    /// Dev-Mode annotation (Figma: notes on a layer for developers). Rides
    /// the bindings map under the reserved `note` key so it round-trips
    /// `.x` without a schema bump.
    pub fn note(&self) -> Option<&str> {
        self.bindings
            .get("note")
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
    }

    /// Set or clear the Dev-Mode annotation.
    pub fn set_note(&mut self, note: Option<&str>) {
        match note {
            Some(s) => {
                self.bindings.insert("note".into(), s.into());
            }
            None => {
                self.bindings.remove("note");
            }
        }
    }

    fn base(id: &str, kind: NodeKind, x: f64, y: f64, w: f64, h: f64, fill: Paint) -> Self {
        Self {
            id: id.into(),
            name: id.into(),
            kind,
            w,
            h,
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill,
            stroke: Stroke::default(),
            fill_layers: vec![],
            stroke_layers: vec![],
            effect_layers: vec![],
            visual_stacks_materialized: false,
            opacity: 1.0,
            children: vec![],
            dirty: true,
            visible: true,
            locked: false,
            prototype: None,
            overrides: HashMap::new(),
            corner_radii: None,
            blend: BlendKind::Normal,
            effects: vec![],
            is_mask: false,
            pin: (HPin::Left, VPin::Top),
            constraints: ChildConstraints::default(),
            bindings: HashMap::new(),
            text_metrics: None,
            text_runs: vec![],
            baseline: None,
            props: vec![],
            export_settings: vec![],
            interactions: vec![],
            is_starting_point: false,
            overflow: Overflow::default(),
            scroll: (0.0, 0.0),
            layout_grids: vec![],
        }
    }
    pub fn frame(id: &str, w: f64, h: f64) -> Self {
        Self::base(
            id,
            NodeKind::Frame { layout: None },
            0.0,
            0.0,
            w,
            h,
            Paint::Solid(Color::TRANSPARENT),
        )
    }
    pub fn group(id: &str, w: f64, h: f64) -> Self {
        Self::base(
            id,
            NodeKind::Group,
            0.0,
            0.0,
            w,
            h,
            Paint::Solid(Color::TRANSPARENT),
        )
    }
    pub fn rect(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Color) -> Self {
        Self::base(
            id,
            NodeKind::Rect { radius: 0.0 },
            x,
            y,
            w,
            h,
            Paint::Solid(fill),
        )
    }
    pub fn ellipse(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Color) -> Self {
        Self::base(id, NodeKind::Ellipse, x, y, w, h, Paint::Solid(fill))
    }
    /// Section container: subtle tint + the node name as its header label.
    pub fn section(id: &str, w: f64, h: f64) -> Self {
        let mut n = Self::base(
            id,
            NodeKind::Section,
            0.0,
            0.0,
            w,
            h,
            Paint::Solid(Color::from_rgba8(0x62, 0x74, 0x8b, 0x0d)),
        );
        n.name = "Section".into();
        n.corner_radii = Some([8.0; 4]);
        n.stroke.paint = Paint::Solid(Color::from_rgba8(0x62, 0x74, 0x8b, 0x5a));
        n.stroke.width = 1.0;
        n
    }
    /// Shape constructors mirror their Figma counterparts; the angle pair
    /// is intrinsic to an arc, so the arity is what it is.
    #[allow(clippy::too_many_arguments)]
    pub fn arc(
        id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        start: f64,
        end: f64,
        fill: Color,
    ) -> Self {
        Self::base(
            id,
            NodeKind::Arc { start, end },
            x,
            y,
            w,
            h,
            Paint::Solid(fill),
        )
    }
    pub fn line(id: &str, x: f64, y: f64, w: f64, h: f64, color: Color) -> Self {
        Self::base(
            id,
            NodeKind::Line,
            x,
            y,
            w,
            h,
            Paint::Solid(Color::TRANSPARENT),
        )
        .stroke(Stroke::solid(color, 2.0))
    }
    pub fn text(id: &str, x: f64, y: f64, w: f64, h: f64, text: &str) -> Self {
        Self::base(
            id,
            NodeKind::Text { text: text.into() },
            x,
            y,
            w,
            h,
            Paint::Solid(Color::BLACK),
        )
    }
    pub fn image(id: &str, x: f64, y: f64, w: f64, h: f64, asset: &str) -> Self {
        Self::base(
            id,
            NodeKind::Image {
                asset: asset.into(),
                fit: ImageFit::default(),
                placement: ImagePlacement::default(),
            },
            x,
            y,
            w,
            h,
            Paint::Solid(Color::from_rgb8(0xdd, 0xdd, 0xdd)),
        )
    }
    pub fn vector(id: &str, x: f64, y: f64, w: f64, h: f64, path: Vec<PathCmd>) -> Self {
        Self::base(
            id,
            NodeKind::Vector { path },
            x,
            y,
            w,
            h,
            Paint::Solid(Color::BLACK),
        )
    }
    pub fn component(id: &str, name: &str, w: f64, h: f64) -> Self {
        Self::base(
            id,
            NodeKind::Component { name: name.into() },
            0.0,
            0.0,
            w,
            h,
            Paint::Solid(Color::TRANSPARENT),
        )
    }
    pub fn instance(id: &str, component: &str, x: f64, y: f64, w: f64, h: f64) -> Self {
        Self::base(
            id,
            NodeKind::Instance {
                component: component.into(),
            },
            x,
            y,
            w,
            h,
            Paint::Solid(Color::TRANSPARENT),
        )
    }
    pub fn slice(id: &str, x: f64, y: f64, w: f64, h: f64) -> Self {
        Self::base(
            id,
            NodeKind::Slice,
            x,
            y,
            w,
            h,
            Paint::Solid(Color::TRANSPARENT),
        )
    }

    pub fn radius(mut self, r: f64) -> Self {
        if let NodeKind::Rect { .. } = self.kind {
            self.kind = NodeKind::Rect { radius: r }
        }
        self
    }
    pub fn corners(mut self, tl: f64, tr: f64, br: f64, bl: f64) -> Self {
        self.corner_radii = Some([tl, tr, br, bl]);
        self
    }
    pub fn rotate(mut self, r: f64) -> Self {
        self.transform.rotation = r;
        self
    }
    pub fn scale(mut self, x: f64, y: f64) -> Self {
        self.transform.scale_x = x;
        self.transform.scale_y = y;
        self
    }
    pub fn opacity(mut self, v: f32) -> Self {
        self.opacity = v.clamp(0.0, 1.0);
        self
    }
    pub fn stroke(mut self, s: Stroke) -> Self {
        self.stroke = s;
        self
    }
    pub fn fill_paint(mut self, p: Paint) -> Self {
        self.fill = p;
        self
    }
    pub fn blend(mut self, b: BlendKind) -> Self {
        self.blend = b;
        self
    }
    pub fn effect(mut self, e: Effect) -> Self {
        self.effects.push(e);
        self
    }
    /// Attach a rich-text style span (byte range) to a Text node.
    pub fn materialize_visual_stacks(&mut self) {
        if self.visual_stacks_materialized {
            return;
        }
        if self.fill_layers.is_empty() {
            self.fill_layers.push(PaintLayer::new(self.fill.clone()));
        }
        if self.stroke_layers.is_empty() && self.stroke.width > 0.0 {
            self.stroke_layers
                .push(StrokeLayer::new(self.stroke.clone()));
        }
        if self.effect_layers.is_empty() {
            self.effect_layers = self.effects.iter().cloned().map(EffectLayer::new).collect();
        }
        self.visual_stacks_materialized = true;
    }

    pub fn active_fills(&self) -> Vec<PaintLayer> {
        if !self.visual_stacks_materialized {
            vec![PaintLayer::new(self.fill.clone())]
        } else {
            self.fill_layers
                .iter()
                .filter(|l| l.visible && l.opacity > 0.0)
                .cloned()
                .collect()
        }
    }
    pub fn active_strokes(&self) -> Vec<StrokeLayer> {
        if !self.visual_stacks_materialized {
            if self.stroke.width > 0.0 {
                vec![StrokeLayer::new(self.stroke.clone())]
            } else {
                vec![]
            }
        } else {
            self.stroke_layers
                .iter()
                .filter(|l| l.visible && l.opacity > 0.0 && l.stroke.width > 0.0)
                .cloned()
                .collect()
        }
    }
    pub fn active_effects(&self) -> Vec<EffectLayer> {
        if !self.visual_stacks_materialized {
            self.effects.iter().cloned().map(EffectLayer::new).collect()
        } else {
            self.effect_layers
                .iter()
                .filter(|l| l.visible && l.opacity > 0.0)
                .cloned()
                .collect()
        }
    }
    pub fn pin(mut self, h: HPin, v: VPin) -> Self {
        self.pin = (h, v);
        self
    }
    pub fn locked(mut self, v: bool) -> Self {
        self.locked = v;
        self
    }
    pub fn mask(mut self, v: bool) -> Self {
        self.is_mask = v;
        self
    }
    pub fn child(mut self, n: Node) -> Self {
        self.children.push(n);
        self.dirty = true;
        self
    }
    /// Set the user-facing display name (does not touch `id`).
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }
    pub fn prototype(mut self, destination: &str, transition_ms: u32) -> Self {
        self.prototype = Some(PrototypeAction {
            destination: destination.into(),
            transition_ms,
        });
        self
    }
    pub fn interaction(mut self, i: Interaction) -> Self {
        self.interactions.push(i);
        self
    }
    pub fn starting_point(mut self, v: bool) -> Self {
        self.is_starting_point = v;
        self
    }
    pub fn override_prop(mut self, key: &str, value: &str) -> Self {
        self.overrides.insert(key.into(), value.into());
        self
    }
    pub fn auto_layout(mut self, layout: AutoLayout) -> Self {
        if let NodeKind::Frame { .. } = self.kind {
            self.kind = NodeKind::Frame {
                layout: Some(layout),
            }
        }
        self
    }
    /// Absolute-position this child inside its auto-layout parent (Figma ABSOLUTE).
    pub fn absolute(mut self) -> Self {
        self.constraints.is_absolute = true;
        self
    }
    /// Fixed positioning: ignores the parent's scroll offset (Figma FIXED).
    pub fn fixed(mut self) -> Self {
        self.constraints.fixed = true;
        self
    }
    /// Sticky positioning (top edge) inside a scrollable parent.
    pub fn sticky(mut self) -> Self {
        self.constraints.sticky = true;
        self
    }
    /// Clip/scroll overflow behavior for a frame.
    pub fn overflow(mut self, o: Overflow) -> Self {
        self.overflow = o;
        self
    }
    /// Set a frame's scroll offset (page px).
    pub fn scroll(mut self, x: f64, y: f64) -> Self {
        self.scroll = (x, y);
        self
    }
    /// Per-child cross-axis alignment override.
    pub fn align_self(mut self, a: Alignment) -> Self {
        self.constraints.align_self = Some(a);
        self
    }
    /// Flex-grow factor.
    pub fn grow(mut self, g: f64) -> Self {
        self.constraints.grow = g;
        self
    }
    /// Flex-shrink factor.
    pub fn shrink(mut self, s: f64) -> Self {
        self.constraints.shrink = s;
        self
    }
    /// Flex-basis (base main-axis size before grow/shrink).
    pub fn basis(mut self, b: f64) -> Self {
        self.constraints.basis = Some(b);
        self
    }
    /// Explicit baseline offset (top edge -> first text baseline, node-local px).
    pub fn baseline_offset(mut self, b: f64) -> Self {
        self.baseline = Some(b);
        self
    }
    /// Bind a property ("radius"/"opacity"/"fontsize"/"w"/"h") to a number variable.
    pub fn bind(mut self, prop: &str, var: &str) -> Self {
        self.bindings.insert(prop.into(), var.into());
        self
    }

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
        TextPart {
            text: text.into(),
            color,
            size,
            font: None,
            weight: None,
            italic: None,
            ls: None,
        }
    }

    #[test]
    fn no_runs_degrades_to_a_single_plain_part() {
        let parts = resolve_text_parts("Hello", &[]);
        assert_eq!(parts, vec![part("Hello", None, None)]);
    }

    #[test]
    fn empty_text_degrades_to_a_single_plain_part() {
        let parts = resolve_text_parts(
            "",
            &[TextRun {
                start: 0,
                len: 5,
                color: Some(Color::from_rgb8(255, 0, 0)),
                size: None,
                font: None,
                weight: None,
                italic: None,
                ls: None,
            }],
        );
        assert_eq!(parts, vec![part("", None, None)]);
    }

    #[test]
    fn styled_range_splits_into_three_parts() {
        // "hello": chars 1..3 styled red
        let runs = [TextRun {
            start: 1,
            len: 2,
            color: Some(Color::from_rgb8(255, 0, 0)),
            size: Some(30.0),
            font: None,
            weight: None,
            italic: None,
            ls: None,
        }];
        let parts = resolve_text_parts("hello", &runs);
        assert_eq!(
            parts,
            vec![
                part("h", None, None),
                part("el", Some(Color::from_rgb8(255, 0, 0)), Some(30.0)),
                part("lo", None, None),
            ]
        );
    }

    #[test]
    fn out_of_range_runs_are_clamped_not_panicking() {
        // hostile/hand-edited files: run far past the end
        let runs = [TextRun {
            start: 10,
            len: 50,
            color: Some(Color::from_rgb8(255, 0, 0)),
            size: None,
            font: None,
            weight: None,
            italic: None,
            ls: None,
        }];
        let parts = resolve_text_parts("abc", &runs);
        // range fully outside -> plain
        assert_eq!(parts, vec![part("abc", None, None)]);
        // partially outside clips to the text end
        let runs = [TextRun {
            start: 2,
            len: 50,
            color: Some(Color::from_rgb8(255, 0, 0)),
            size: None,
            font: None,
            weight: None,
            italic: None,
            ls: None,
        }];
        let parts = resolve_text_parts("abc", &runs);
        assert_eq!(
            parts,
            vec![
                part("ab", None, None),
                part("c", Some(Color::from_rgb8(255, 0, 0)), None)
            ]
        );
    }

    #[test]
    fn overlapping_runs_last_wins() {
        let runs = [
            TextRun {
                start: 0,
                len: 4,
                color: Some(Color::from_rgb8(255, 0, 0)),
                size: None,
                font: None,
                weight: None,
                italic: None,
                ls: None,
            },
            TextRun {
                start: 2,
                len: 2,
                color: Some(Color::from_rgb8(0, 0, 255)),
                size: None,
                font: None,
                weight: None,
                italic: None,
                ls: None,
            },
        ];
        let parts = resolve_text_parts("abcd", &runs);
        assert_eq!(
            parts,
            vec![
                part("ab", Some(Color::from_rgb8(255, 0, 0)), None),
                part("cd", Some(Color::from_rgb8(0, 0, 255)), None),
            ]
        );
    }

    #[test]
    fn whole_text_run_covers_every_char() {
        let runs = [TextRun {
            start: 0,
            len: 5,
            color: Some(Color::from_rgb8(255, 0, 0)),
            size: None,
            font: None,
            weight: None,
            italic: None,
            ls: None,
        }];
        let parts = resolve_text_parts("hello", &runs);
        assert_eq!(
            parts,
            vec![part("hello", Some(Color::from_rgb8(255, 0, 0)), None)]
        );
    }
}

#[cfg(test)]
mod simplify_tests {
    use super::*;

    #[test]
    fn collinear_points_collapse_to_two() {
        let pts = vec![(0.0, 0.0), (1.0, 0.5), (2.0, 1.0), (3.0, 1.5), (4.0, 2.0)];
        assert_eq!(simplify_polyline(&pts, 0.1), vec![(0.0, 0.0), (4.0, 2.0)]);
    }

    #[test]
    fn spikes_survive() {
        // a spike in the middle must survive simplification
        let pts = vec![(0.0, 0.0), (5.0, 5.0), (10.0, 0.0)];
        assert_eq!(simplify_polyline(&pts, 0.1), pts);
        // ...but a shallow wiggle within eps is dropped
        let pts = vec![(0.0, 0.0), (5.0, 0.4), (10.0, 0.0)];
        assert_eq!(simplify_polyline(&pts, 1.0), vec![(0.0, 0.0), (10.0, 0.0)]);
    }

    #[test]
    fn tiny_inputs_and_zero_eps() {
        assert_eq!(simplify_polyline(&[], 1.0), vec![]);
        assert_eq!(simplify_polyline(&[(1.0, 1.0)], 1.0), vec![(1.0, 1.0)]);
        let pts = vec![(0.0, 0.0), (1.0, 0.1), (2.0, 0.0)];
        assert_eq!(simplify_polyline(&pts, 0.0), pts, "eps 0 keeps everything");
    }
}

#[cfg(test)]
mod layout_grid_tests {
    use super::*;

    #[test]
    fn columns_bands_math() {
        let g = LayoutGridDef {
            pattern: GridPattern::Columns,
            count: 4,
            gutter: 10.0,
            margin: 10.0,
            cell: 8.0,
        };
        // frame 210 wide: inner 190, bands (190 - 3*10)/4 = 40
        let b = g.bands(210.0, 100.0);
        assert_eq!(b.len(), 4);
        assert_eq!(b[0], (10.0, 0.0, 40.0, 100.0));
        assert_eq!(b[1], (60.0, 0.0, 40.0, 100.0));
        assert_eq!(b[3], (160.0, 0.0, 40.0, 100.0));
        // last band ends exactly at the right margin
        assert!((b[3].0 + b[3].2 - 200.0).abs() < 1e-9);
    }

    #[test]
    fn rows_bands_and_clamping() {
        let g = LayoutGridDef {
            pattern: GridPattern::Rows,
            count: 3,
            gutter: 8.0,
            margin: 6.0,
            cell: 8.0,
        };
        let b = g.bands(50.0, 70.0);
        assert_eq!(b.len(), 3);
        // inner 58, band (58-16)/3 = 14
        assert_eq!(b[0], (0.0, 6.0, 50.0, 14.0));
        // degenerate: huge gutter clamps so bands never go negative
        let wild = LayoutGridDef {
            pattern: GridPattern::Rows,
            count: 2,
            gutter: 500.0,
            margin: 0.0,
            cell: 8.0,
        };
        let b = wild.bands(100.0, 100.0);
        assert_eq!(b.len(), 2);
        assert!(b[0].2 >= 0.0 && b[0].3 >= 0.0);
    }

    #[test]
    fn grid_pattern_lines() {
        let g = LayoutGridDef {
            pattern: GridPattern::Grid,
            count: 12,
            gutter: 20.0,
            margin: 20.0,
            cell: 8.0,
        };
        assert!(
            g.bands(100.0, 100.0).is_empty(),
            "grid draws lines, not bands"
        );
        let (xs, ys) = g.grid_lines(20.0, 12.0);
        assert_eq!(xs, vec![0.0, 8.0, 16.0]);
        assert_eq!(ys, vec![0.0, 8.0]);
        // zero cell falls back to 1px steps (never a loop)
        let z = LayoutGridDef {
            pattern: GridPattern::Grid,
            cell: 0.0,
            ..Default::default()
        };
        let (xs, _) = z.grid_lines(3.0, 3.0);
        assert_eq!(xs.len(), 4);
    }

    #[test]
    fn pattern_strings_roundtrip() {
        for p in [GridPattern::Columns, GridPattern::Rows, GridPattern::Grid] {
            assert_eq!(GridPattern::parse(p.to_str()), p);
        }
        assert_eq!(GridPattern::parse("nonsense"), GridPattern::Columns);
    }
}
