//! Import IR — the common intermediate representation every importer
//! targets (review: "don't let Sketch → Node, SVG → Node, Figma → Node
//! each develop completely different semantics").
//!
//! Pipeline:
//!
//!   External file → importer → **ImportDoc (this IR)** → lower() →
//!   X-Native Document → render IR → editor
//!
//! Importers only PARSE; every shared semantic decision lives in ONE
//! place — `lower()`:
//!   * id generation, sanitization, and cross-page uniqueness
//!   * kind-appropriate fill defaults (text = black, shapes = transparent)
//!   * opacity clamping, NaN/∞ scrubbing of all geometry
//!   * page auto-sizing to the content envelope (+margin, sane minimum)
//!   * instance text-override encoding (the render-effective `"text:"`)
//!   * rotation stored in the ONE native convention (radians, positive =
//!     clockwise in y-down screen space; importers convert at parse time
//!     and document their source convention)
//!
//! A conformance suite (`import_conformance.rs`) runs the SAME assertions
//! over every importer, so a new source format can't drift.

use std::collections::{HashMap, HashSet};
use x_core::*;

// ---------------------------------------------------------------------- IR

#[derive(Debug, Clone)]
pub enum ImportKind {
    Frame,
    Group,
    Rect { radius: f64 },
    Ellipse,
    Line,
    Text { content: String },
    Path { cmds: Vec<PathCmd> },
    Image { asset: String },
    Component { name: String },
    Instance { component: String, text_overrides: Vec<(String, String)> },
}

#[derive(Debug, Clone)]
pub struct ImportNode {
    /// source-file id, if the format has one; lower() falls back to a
    /// generated id and dedupes collisions either way
    pub id: Option<String>,
    pub kind: ImportKind,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// radians, positive = clockwise in y-down screen space (the native
    /// convention). Importers convert their source's convention here.
    pub rotation: f64,
    /// None = "source specified nothing" -> lower() picks the kind default
    pub fill: Option<Paint>,
    /// Primary (first) stroke — kept as a plain tuple for backward
    /// compatibility with every existing importer call site.
    pub stroke: Option<(Color, f64)>,
    /// Any strokes beyond the first (Figma/Sketch both support stacking
    /// multiple stroke paints on one layer). Same width convention as
    /// `stroke`; importers that don't support multi-stroke just leave
    /// this empty and nothing changes for them.
    pub extra_strokes: Vec<(Color, f64)>,
    /// Layer effects (shadows/blurs). Empty = none, never guessed.
    pub effects: Vec<Effect>,
    /// Auto-layout (Figma "layoutMode" / Sketch resizing stacks). None =
    /// source has no auto-layout on this node — only meaningful on
    /// `ImportKind::Frame`; lower() ignores it for any other kind.
    pub layout: Option<AutoLayout>,
    pub opacity: f32,
    pub visible: bool,
    pub children: Vec<ImportNode>,
}

impl ImportNode {
    pub fn new(kind: ImportKind) -> Self {
        Self { id: None, kind, x: 0.0, y: 0.0, w: 0.0, h: 0.0, rotation: 0.0, fill: None, stroke: None, extra_strokes: vec![], effects: vec![], layout: None, opacity: 1.0, visible: true, children: vec![] }
    }
    pub fn id(mut self, id: impl Into<String>) -> Self { self.id = Some(id.into()); self }
    pub fn at(mut self, x: f64, y: f64) -> Self { self.x = x; self.y = y; self }
    pub fn size(mut self, w: f64, h: f64) -> Self { self.w = w; self.h = h; self }
    pub fn fill(mut self, p: Paint) -> Self { self.fill = Some(p); self }
    pub fn child(mut self, c: ImportNode) -> Self { self.children.push(c); self }
}

/// What an importer hands to `lower()`: pages plus any binary assets the
/// source embedded (registered by name so Image nodes resolve).
#[derive(Debug, Clone, Default)]
pub struct ImportDoc {
    pub pages: Vec<ImportNode>,
    /// asset name -> raw PNG bytes (decoded/registered by the app shell)
    pub assets: Vec<(String, Vec<u8>)>,
    /// which importer produced this (diagnostics / conformance)
    pub source: &'static str,
    /// importer-side diagnostics: source constructs that were skipped or
    /// approximated (importers push; lower() adds its own; the app shows
    /// them after import so fidelity is MEASURABLE per file)
    pub diagnostics: Vec<String>,
}

/// Import result with per-file fidelity diagnostics.
#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub nodes_imported: usize,
    pub assets_imported: usize,
    pub diagnostics: Vec<String>,
}

// ------------------------------------------------------------------ lower

fn clean(v: f64) -> f64 { if v.is_finite() { v } else { 0.0 } }

fn sanitize_id(raw: &str) -> String {
    let s: String = raw.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' { c } else { '-' }).collect();
    if s.is_empty() { "node".into() } else { s }
}

/// THE single importer → Document lowering. All shared import semantics
/// live here; importers never construct `Node` themselves.
pub fn lower(doc: ImportDoc) -> Document { lower_with_report(doc).0 }

/// Same lowering, plus the fidelity report (import diagnostics item).
pub fn lower_with_report(doc: ImportDoc) -> (Document, ImportReport) {
    let mut out = Document::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut counter = 0usize;
    // component names seen, for instance-ref diagnostics (kept permissive:
    // unknown refs stay as-is so partially-imported files still open)
    let mut component_names: HashMap<String, ()> = HashMap::new();
    for p in &doc.pages { collect_components(p, &mut component_names); }

    // register embedded assets into the content-addressed store and build
    // the source-name -> asset:// id map that Image nodes rewrite through
    let mut asset_ids: HashMap<String, String> = HashMap::new();
    for (name, bytes) in doc.assets {
        let id = out.assets.register(&name, bytes, AssetSource::Embedded);
        asset_ids.insert(name, id);
    }

    let mut report = ImportReport { diagnostics: doc.diagnostics.clone(), ..Default::default() };
    report.assets_imported = asset_ids.len();
    for (pi, page_ir) in doc.pages.into_iter().enumerate() {
        let mut page = lower_node(page_ir, &mut used, &mut counter, true, &asset_ids);
        // shared page semantics: a page is always a Frame, auto-sized to
        // its content envelope when the source gave no/zero size
        if page.w <= 0.0 || page.h <= 0.0 {
            let (mut mw, mut mh) = (0.0f64, 0.0f64);
            for c in &page.children {
                mw = mw.max(c.transform.x + c.w);
                mh = mh.max(c.transform.y + c.h);
            }
            page.w = (mw + 40.0).max(800.0);
            page.h = (mh + 40.0).max(600.0);
        }
        if page.id.is_empty() { page.id = format!("page-{}", pi + 1); }
        fn count(n: &Node) -> usize { 1 + n.children.iter().map(count).sum::<usize>() }
        report.nodes_imported += count(&page);
        out.pages.push(page);
    }
    (out, report)
}

fn collect_components(n: &ImportNode, out: &mut HashMap<String, ()>) {
    if let ImportKind::Component { name } = &n.kind { out.insert(name.clone(), ()); }
    for c in &n.children { collect_components(c, out); }
}

fn lower_node(ir: ImportNode, used: &mut HashSet<String>, counter: &mut usize, is_page: bool, asset_ids: &HashMap<String, String>) -> Node {
    // ---- id: sanitize source id or generate; dedupe globally
    let base = match &ir.id {
        Some(raw) => sanitize_id(raw),
        None => { *counter += 1; format!("import-{counter}") }
    };
    let mut id = base.clone();
    let mut n = 2;
    while !used.insert(id.clone()) { id = format!("{base}-{n}"); n += 1; }

    let (x, y, w, h) = (clean(ir.x), clean(ir.y), clean(ir.w).max(0.0), clean(ir.h).max(0.0));

    // ---- kind + kind-default fills (THE shared defaults table)
    let mut node = match ir.kind {
        ImportKind::Frame => {
            let mut f = Node::frame(&id, w, h);
            f.transform.x = x; f.transform.y = y;
            f.fill = ir.fill.clone().unwrap_or(Paint::Solid(if is_page { Color::TRANSPARENT } else { Color::WHITE }));
            if let Some(layout) = ir.layout.clone() { f = f.auto_layout(layout); }
            f
        }
        ImportKind::Group => {
            let mut g = Node::group(&id, w, h);
            g.transform.x = x; g.transform.y = y;
            if let Some(p) = ir.fill.clone() { g.fill = p; }
            g
        }
        ImportKind::Rect { radius } => Node::rect(&id, x, y, w, h, Color::TRANSPARENT)
            .radius(clean(radius).max(0.0))
            .fill_paint(ir.fill.clone().unwrap_or(Paint::Solid(Color::TRANSPARENT))),
        ImportKind::Ellipse => Node::ellipse(&id, x, y, w, h, Color::TRANSPARENT)
            .fill_paint(ir.fill.clone().unwrap_or(Paint::Solid(Color::TRANSPARENT))),
        ImportKind::Line => {
            let (c, sw) = ir.stroke.unwrap_or((Color::BLACK, 2.0));
            let mut l = Node::line(&id, x, y, w.max(1.0), h, c);
            l.stroke.width = sw.max(0.5);
            l
        }
        ImportKind::Text { content } => Node::text(&id, x, y, w, h, &content)
            // shared semantic: unstyled text is BLACK, never transparent
            .fill_paint(ir.fill.clone().unwrap_or(Paint::Solid(Color::BLACK))),
        ImportKind::Path { cmds } => {
            let cmds: Vec<PathCmd> = cmds.into_iter().map(|c| match c {
                PathCmd::MoveTo(a, b) => PathCmd::MoveTo(clean(a), clean(b)),
                PathCmd::LineTo(a, b) => PathCmd::LineTo(clean(a), clean(b)),
                PathCmd::CurveTo(a, b, c2, d, e, f) => PathCmd::CurveTo(clean(a), clean(b), clean(c2), clean(d), clean(e), clean(f)),
                PathCmd::Close => PathCmd::Close,
            }).collect();
            Node::vector(&id, x, y, w, h, cmds)
                .fill_paint(ir.fill.clone().unwrap_or(Paint::Solid(Color::TRANSPARENT)))
        }
        ImportKind::Image { asset } => {
            // shared semantic: embedded assets resolve to their stable
            // asset:// id; unknown names stay as legacy filename refs
            let asset_ref = asset_ids.get(&asset).cloned().unwrap_or(asset);
            let mut i = Node::image(&id, x, y, w, h, &asset_ref);
            i.transform.x = x; i.transform.y = y;
            i
        }
        ImportKind::Component { name } => {
            let mut c = Node::component(&id, &name, w, h);
            c.transform.x = x; c.transform.y = y;
            c
        }
        ImportKind::Instance { component, text_overrides } => {
            let mut inst = Node::instance(&id, &component, x, y, w, h);
            for (target, value) in text_overrides {
                // shared semantic: render-effective override encoding
                inst.overrides.insert(sanitize_id(&target), format!("text:{value}"));
            }
            inst
        }
    };

    // ---- shared scalar semantics
    if let Some((c, sw)) = ir.stroke {
        if !matches!(node.kind, NodeKind::Line) && sw > 0.0 {
            node.stroke = Stroke { color: c, width: clean(sw) };
        }
    }
    if !ir.extra_strokes.is_empty() && !matches!(node.kind, NodeKind::Line) {
        node.materialize_visual_stacks();
        for (c, sw) in &ir.extra_strokes {
            if *sw > 0.0 {
                node.stroke_layers.push(StrokeLayer::new(Stroke { color: *c, width: clean(*sw) }));
            }
        }
    }
    if !ir.effects.is_empty() {
        node.effects = ir.effects;
    }
    node.transform.rotation = clean(ir.rotation);
    node.opacity = if ir.opacity.is_finite() { ir.opacity.clamp(0.0, 1.0) } else { 1.0 };
    node.visible = ir.visible;

    for c in ir.children {
        let cn = lower_node(c, used, counter, false, asset_ids);
        node.children.push(cn);
    }
    node
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_dedupes_colliding_and_sanitizes_ids() {
        let doc = ImportDoc {
            pages: vec![ImportNode::new(ImportKind::Frame).id("p")
                .child(ImportNode::new(ImportKind::Rect { radius: 0.0 }).id("a b/c").size(10.0, 10.0))
                .child(ImportNode::new(ImportKind::Rect { radius: 0.0 }).id("a b/c").size(10.0, 10.0))
                .child(ImportNode::new(ImportKind::Ellipse).size(5.0, 5.0))],
            ..Default::default()
        };
        let d = lower(doc);
        let p = &d.pages[0];
        assert_eq!(p.children[0].id, "a-b-c");
        assert_eq!(p.children[1].id, "a-b-c-2", "collision deduped");
        assert!(p.children[2].id.starts_with("import-"), "missing id generated");
    }

    #[test]
    fn lower_scrubs_nan_and_clamps_opacity() {
        let mut r = ImportNode::new(ImportKind::Rect { radius: f64::NAN }).id("r").size(f64::INFINITY, 10.0);
        r.opacity = 7.5;
        r.rotation = f64::NAN;
        let doc = ImportDoc { pages: vec![ImportNode::new(ImportKind::Frame).id("p").child(r)], ..Default::default() };
        let d = lower(doc);
        let n = &d.pages[0].children[0];
        assert_eq!(n.w, 0.0);
        assert_eq!(n.opacity, 1.0);
        assert_eq!(n.transform.rotation, 0.0);
    }

    #[test]
    fn lower_pages_autosize_and_text_defaults_black() {
        let doc = ImportDoc {
            pages: vec![ImportNode::new(ImportKind::Frame).id("p")
                .child(ImportNode::new(ImportKind::Text { content: "hi".into() }).id("t").at(900.0, 700.0).size(100.0, 20.0))],
            ..Default::default()
        };
        let d = lower(doc);
        assert!(d.pages[0].w >= 1040.0, "page envelops content");
        assert_eq!(d.pages[0].children[0].fill, Paint::Solid(Color::BLACK));
    }

    #[test]
    fn lower_encodes_instance_text_overrides_render_effective() {
        let doc = ImportDoc {
            pages: vec![ImportNode::new(ImportKind::Frame).id("p")
                .child(ImportNode::new(ImportKind::Instance {
                    component: "Button".into(),
                    text_overrides: vec![("label".into(), "Buy now".into())],
                }).id("i1").size(80.0, 30.0))],
            ..Default::default()
        };
        let d = lower(doc);
        assert_eq!(d.pages[0].children[0].overrides.get("label"), Some(&"text:Buy now".to_string()));
    }
}
