//! Figma importer — REST-API JSON documents (`GET /v1/files/:key` shape).
//!
//! Honest scope: the binary `.fig` format is proprietary, undocumented,
//! and version-unstable — NOT parsed here. What IS parsed is the JSON
//! document the official Figma REST API returns (and which tools like
//! `figma-export` save to disk): `{"document": {"children": [pages]}}`.
//! That's the interoperable path Figma itself supports.
//!
//! **Covered**: pages (CANVAS), FRAME/GROUP/COMPONENT/INSTANCE,
//! RECTANGLE (incl. cornerRadius), ELLIPSE, LINE, TEXT (characters +
//! style.fontSize), VECTOR (fillGeometry SVG path data), solid fills,
//! linear/radial gradient fills (gradientHandlePositions), per-node
//! opacity, visibility, rotation, absoluteBoundingBox geometry
//! (converted to parent-relative), instance componentId -> component
//! name resolution, the full stroke stack (multiple solid strokes, not
//! just the first), layer effects (drop/inner shadow, layer/background
//! blur) round-tripping through `effects`, and auto-layout (`layoutMode`)
//! mapped onto our native `AutoLayout` model — approximated where the two
//! models don't line up 1:1 (Figma's 4-sided padding averages to our one
//! uniform value; a frame only hugs if BOTH axes ask to, since our model
//! has one `sizing` flag for both).
//!
//! **Not covered** (fallback/skip, never panic): boolean ops (imported
//! as their rendered fillGeometry when present), constraints, image
//! fills (placeholder asset name, not the actual pixel bytes).
//!
//! Everything lowers through the SHARED Import IR — this file only
//! parses; semantics live in import_ir::lower().

use crate::import_ir::{ImportDoc, ImportKind, ImportNode};
use crate::json::{self, V};
use crate::svg_import::parse_path_d;
use std::collections::HashMap;
use x_core::{Color, Document, Paint};

fn esc_json(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r") }
fn path_d(path: &[x_core::PathCmd]) -> String { path.iter().map(|c| match c { x_core::PathCmd::MoveTo(x,y) => format!("M {x} {y}"), x_core::PathCmd::LineTo(x,y) => format!("L {x} {y}"), x_core::PathCmd::CurveTo(x1,y1,x2,y2,x,y) => format!("C {x1} {y1} {x2} {y2} {x} {y}"), x_core::PathCmd::Close => "Z".into() }).collect::<Vec<_>>().join(" ") }
fn figma_color_json(c: Color) -> String { format!("{{\"r\":{},\"g\":{},\"b\":{},\"a\":{}}}", c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0, c.a as f64 / 255.0) }
fn figma_paint_json(p: &Paint, w: f64, h: f64) -> String {
    match p {
        Paint::Solid(c) => format!("{{\"type\":\"SOLID\",\"visible\":true,\"color\":{}}}", figma_color_json(*c)),
        Paint::Variable(_) => "{\"type\":\"SOLID\",\"visible\":true,\"color\":{\"r\":0,\"g\":0,\"b\":0,\"a\":1}}".into(),
        Paint::LinearGradient { start, end, stops } => format!("{{\"type\":\"GRADIENT_LINEAR\",\"visible\":true,\"gradientHandlePositions\":[{{\"x\":{},\"y\":{}}},{{\"x\":{},\"y\":{}}}],\"gradientStops\":[{}]}}", start.0 / w.max(1.0), start.1 / h.max(1.0), end.0 / w.max(1.0), end.1 / h.max(1.0), stops.iter().map(|(t,c)| format!("{{\"position\":{t},\"color\":{}}}", figma_color_json(*c))).collect::<Vec<_>>().join(",")),
        Paint::RadialGradient { center, radius, stops } => format!("{{\"type\":\"GRADIENT_RADIAL\",\"visible\":true,\"gradientHandlePositions\":[{{\"x\":{},\"y\":{}}},{{\"x\":{},\"y\":{}}}],\"gradientStops\":[{}]}}", center.0 / w.max(1.0), center.1 / h.max(1.0), (center.0 + radius) / w.max(1.0), center.1 / h.max(1.0), stops.iter().map(|(t,c)| format!("{{\"position\":{t},\"color\":{}}}", figma_color_json(*c))).collect::<Vec<_>>().join(",")),
    }
}

fn figma_effect_json(e: x_core::EffectLayer) -> Option<String> {
    if !e.visible || e.opacity <= 0.0 { return None; }
    use x_core::Effect::*;
    Some(match e.effect {
        DropShadow { dx, dy, blur, color } => format!("{{\"type\":\"DROP_SHADOW\",\"visible\":true,\"radius\":{blur},\"color\":{},\"offset\":{{\"x\":{dx},\"y\":{dy}}}}}", figma_color_json(color)),
        InnerShadow { dx, dy, blur, color } => format!("{{\"type\":\"INNER_SHADOW\",\"visible\":true,\"radius\":{blur},\"color\":{},\"offset\":{{\"x\":{dx},\"y\":{dy}}}}}", figma_color_json(color)),
        LayerBlur { radius } => format!("{{\"type\":\"LAYER_BLUR\",\"visible\":true,\"radius\":{radius}}}"),
        BackgroundBlur { radius } => format!("{{\"type\":\"BACKGROUND_BLUR\",\"visible\":true,\"radius\":{radius}}}"),
    })
}

fn figma_layout_json(l: &x_core::AutoLayout) -> String {
    let mode = match l.direction { x_core::LayoutDirection::Horizontal => "HORIZONTAL", x_core::LayoutDirection::Vertical => "VERTICAL" };
    let axis_mode = |hug: bool| if hug { "AUTO" } else { "FIXED" };
    let (main_mode, cross_mode) = (axis_mode(l.sizing == x_core::Sizing::Hug), axis_mode(l.sizing == x_core::Sizing::Hug));
    let counter_align = match l.align { x_core::CrossAlign::Start => "MIN", x_core::CrossAlign::Center => "CENTER", x_core::CrossAlign::End => "MAX" };
    let primary_align = if l.space_between { "SPACE_BETWEEN" } else { "MIN" };
    let wrap = if l.wrap == x_core::AutoLayoutWrap::Wrap { "WRAP" } else { "NO_WRAP" };
    format!(",\"layoutMode\":\"{mode}\",\"itemSpacing\":{},\"paddingLeft\":{p},\"paddingRight\":{p},\"paddingTop\":{p},\"paddingBottom\":{p},\"primaryAxisSizingMode\":\"{main_mode}\",\"counterAxisSizingMode\":\"{cross_mode}\",\"primaryAxisAlignItems\":\"{primary_align}\",\"counterAxisAlignItems\":\"{counter_align}\",\"layoutWrap\":\"{wrap}\"", l.gap, p = l.padding)
}

fn export_node(n: &x_core::Node, parent: (f64, f64)) -> String {
    use x_core::NodeKind;
    let ax = parent.0 + n.transform.x; let ay = parent.1 + n.transform.y;
    let (ty, extra) = match &n.kind {
        NodeKind::Frame { layout } => ("FRAME", layout.as_ref().map(figma_layout_json).unwrap_or_default()), NodeKind::Group => ("GROUP", String::new()),
        NodeKind::Component { name } => ("COMPONENT", format!(",\"description\":\"{}\"", esc_json(name))),
        NodeKind::Instance { component } => ("INSTANCE", format!(",\"componentId\":\"{}\"", esc_json(component))),
        NodeKind::Rect { radius } => ("RECTANGLE", format!(",\"cornerRadius\":{radius}")), NodeKind::Ellipse => ("ELLIPSE", String::new()),
        NodeKind::Line => ("LINE", String::new()), NodeKind::Text { text } => ("TEXT", format!(",\"characters\":\"{}\",\"style\":{{\"fontSize\":{}}}", esc_json(text), n.h)),
        NodeKind::Vector { path } => ("VECTOR", format!(",\"fillGeometry\":[{{\"path\":\"{}\",\"windingRule\":\"NONZERO\"}}]", esc_json(&path_d(path)))),
        NodeKind::Image { .. } => ("RECTANGLE", String::new()),
        NodeKind::VectorNetwork(_) => ("VECTOR", String::new()),
    };
    let fills = n.active_fills().iter().map(|l| figma_paint_json(&l.paint, n.w, n.h)).collect::<Vec<_>>().join(",");
    let strokes = n.active_strokes();
    let stroke_json = if strokes.is_empty() { String::new() } else {
        let paints = strokes.iter().map(|s| figma_paint_json(&Paint::Solid(s.stroke.color), n.w, n.h)).collect::<Vec<_>>().join(",");
        // Figma has one strokeWeight per node regardless of stack depth —
        // use the first (topmost) stroke's width, same lossy convention
        // real Figma exports use.
        format!(",\"strokes\":[{paints}],\"strokeWeight\":{}", strokes[0].stroke.width)
    };
    let effects = n.active_effects().into_iter().filter_map(figma_effect_json).collect::<Vec<_>>().join(",");
    let children = if n.children.is_empty() { String::new() } else { format!(",\"children\":[{}]", n.children.iter().map(|c| export_node(c, (ax, ay))).collect::<Vec<_>>().join(",")) };
    format!("{{\"id\":\"{}\",\"name\":\"{}\",\"type\":\"{ty}\",\"visible\":{},\"opacity\":{},\"rotation\":{},\"absoluteBoundingBox\":{{\"x\":{ax},\"y\":{ay},\"width\":{},\"height\":{}}},\"fills\":[{fills}]{stroke_json},\"effects\":[{effects}]{extra}{children}}}", esc_json(&n.id), esc_json(&n.id), n.visible, n.opacity, -n.transform.rotation, n.w, n.h)
}

/// Export an editable Figma REST-compatible JSON document. This is the
/// documented interchange representation; it is deliberately not labelled
/// as the proprietary binary `.fig` format.
pub fn export_figma_json(doc: &Document) -> String {
    let pages = doc.pages.iter().enumerate().map(|(i,p)| { let name = if p.id.is_empty() { format!("Page {}", i + 1) } else { p.id.clone() }; format!("{{\"id\":\"{}\",\"name\":\"{}\",\"type\":\"CANVAS\",\"children\":[{}]}}", esc_json(&p.id), esc_json(&name), p.children.iter().map(|c| export_node(c, (0.0, 0.0))).collect::<Vec<_>>().join(",")) }).collect::<Vec<_>>().join(",");
    format!("{{\"name\":\"X Designer export\",\"components\":{{}},\"document\":{{\"id\":\"0:0\",\"type\":\"DOCUMENT\",\"children\":[{pages}]}}}}")
}

fn s<'a>(v: &'a V, key: &str) -> Option<&'a str> { v.get(key).and_then(|x| x.str()) }
fn n_or(v: &V, key: &str, d: f64) -> f64 { v.get(key).and_then(|x| x.num()).unwrap_or(d) }

/// Figma colors: {r,g,b,a} floats 0..=1.
fn figma_color(v: &V) -> Color {
    Color::rgba(n_or(v, "r", 0.0), n_or(v, "g", 0.0), n_or(v, "b", 0.0), n_or(v, "a", 1.0))
}

/// First visible fill -> Paint. Gradient handles are normalized to the
/// bounding box; scale to pixels (the same lesson the Sketch importer
/// learned live in session 33).
fn first_fill(node: &V, w: f64, h: f64) -> Option<Paint> {
    let fills = node.get("fills")?.arr()?;
    let f = fills.iter().find(|f| f.get("visible").and_then(V::boolean).unwrap_or(true))?;
    let opacity = n_or(f, "opacity", 1.0) as f32;
    match s(f, "type") {
        Some("SOLID") => {
            let mut c = f.get("color").map(figma_color)?;
            c.a = (c.a as f32 * opacity * 255.0 / 255.0).min(255.0) as u8;
            if opacity < 1.0 { c.a = (opacity * 255.0) as u8; }
            Some(Paint::Solid(c))
        }
        Some(t @ ("GRADIENT_LINEAR" | "GRADIENT_RADIAL")) => {
            let stops: Vec<(f32, Color)> = f.get("gradientStops")?.arr()?.iter()
                .filter_map(|st| Some((n_or(st, "position", 0.0) as f32, st.get("color").map(figma_color)?)))
                .collect();
            if stops.is_empty() { return None; }
            let handles = f.get("gradientHandlePositions").and_then(V::arr);
            let hp = |i: usize| -> (f64, f64) {
                handles.and_then(|h| h.get(i))
                    .map(|p| (n_or(p, "x", 0.0) * w, n_or(p, "y", 0.0) * h))
                    .unwrap_or((0.0, 0.0))
            };
            if t == "GRADIENT_LINEAR" {
                Some(Paint::LinearGradient { start: hp(0), end: hp(1), stops })
            } else {
                let c = hp(0);
                let e = hp(1);
                let r = ((e.0 - c.0).powi(2) + (e.1 - c.1).powi(2)).sqrt().max(1.0);
                Some(Paint::RadialGradient { center: c, radius: r, stops })
            }
        }
        _ => None, // IMAGE fills etc. — handled as placeholder by caller
    }
}

/// absoluteBoundingBox: {x, y, width, height} in canvas coords.
fn bbox(v: &V) -> (f64, f64, f64, f64) {
    match v.get("absoluteBoundingBox") {
        Some(b) => (n_or(b, "x", 0.0), n_or(b, "y", 0.0), n_or(b, "width", 0.0), n_or(b, "height", 0.0)),
        None => (0.0, 0.0, 0.0, 0.0),
    }
}

fn collect_component_names(v: &V, out: &mut HashMap<String, String>) {
    // file-level "components" map: id -> {name}
    if let Some(V::Obj(m)) = v.get("components") {
        for (id, c) in m {
            if let Some(name) = s(c, "name") { out.insert(id.clone(), name.to_string()); }
        }
    }
}

/// Layer effects (shadows/blurs): DROP_SHADOW, INNER_SHADOW, LAYER_BLUR,
/// BACKGROUND_BLUR — Figma's full effect set maps 1:1 onto our `Effect`.
fn figma_effects(node: &V) -> Vec<x_core::Effect> {
    let Some(arr) = node.get("effects").and_then(V::arr) else { return vec![] };
    arr.iter()
        .filter(|e| e.get("visible").and_then(V::boolean).unwrap_or(true))
        .filter_map(|e| {
            let radius = n_or(e, "radius", 0.0);
            let off = e.get("offset");
            let dx = off.map(|o| n_or(o, "x", 0.0)).unwrap_or(0.0);
            let dy = off.map(|o| n_or(o, "y", 0.0)).unwrap_or(0.0);
            let color = e.get("color").map(figma_color).unwrap_or(Color::rgba8(0, 0, 0, 255));
            match s(e, "type") {
                Some("DROP_SHADOW") => Some(x_core::Effect::DropShadow { dx, dy, blur: radius, color }),
                Some("INNER_SHADOW") => Some(x_core::Effect::InnerShadow { dx, dy, blur: radius, color }),
                Some("LAYER_BLUR") => Some(x_core::Effect::LayerBlur { radius }),
                Some("BACKGROUND_BLUR") => Some(x_core::Effect::BackgroundBlur { radius }),
                _ => None,
            }
        })
        .collect()
}

/// Figma auto-layout ("layoutMode") -> our native `AutoLayout`. Figma has
/// per-side padding and independent primary/counter sizing modes; our
/// model has one uniform `padding` and one `sizing` flag for both axes,
/// so this is a documented, deliberate approximation, not a bug:
/// padding is averaged across the four sides, and the frame only hugs
/// if BOTH axes asked to (otherwise it stays Fixed, which is the safer
/// failure mode — a wrongly-collapsed frame is worse than a wrongly-
/// static one).
fn figma_auto_layout(node: &V) -> Option<x_core::AutoLayout> {
    let mode = s(node, "layoutMode")?;
    let direction = match mode {
        "HORIZONTAL" => x_core::LayoutDirection::Horizontal,
        "VERTICAL" => x_core::LayoutDirection::Vertical,
        _ => return None, // "NONE" or unrecognized: no auto-layout
    };
    let pads = [n_or(node, "paddingLeft", 0.0), n_or(node, "paddingRight", 0.0), n_or(node, "paddingTop", 0.0), n_or(node, "paddingBottom", 0.0)];
    let padding = pads.iter().sum::<f64>() / 4.0;
    let hug_main = s(node, "primaryAxisSizingMode") == Some("AUTO");
    let hug_cross = s(node, "counterAxisSizingMode") == Some("AUTO");
    let sizing = if hug_main && hug_cross { x_core::Sizing::Hug } else { x_core::Sizing::Fixed };
    let align = match s(node, "counterAxisAlignItems") {
        Some("CENTER") => x_core::CrossAlign::Center,
        Some("MAX") => x_core::CrossAlign::End,
        _ => x_core::CrossAlign::Start,
    };
    let space_between = s(node, "primaryAxisAlignItems") == Some("SPACE_BETWEEN");
    let wrap = if s(node, "layoutWrap") == Some("WRAP") { x_core::AutoLayoutWrap::Wrap } else { x_core::AutoLayoutWrap::NoWrap };
    Some(x_core::AutoLayout {
        direction, gap: n_or(node, "itemSpacing", 0.0), padding, sizing,
        align, space_between, wrap, ..Default::default()
    })
}

fn convert(node: &V, parent_abs: (f64, f64), components: &HashMap<String, String>) -> Option<ImportNode> {
    let ty = s(node, "type")?;
    let (ax, ay, w, h) = bbox(node);
    // Figma gives absolute canvas coords; our tree is parent-relative
    let (x, y) = (ax - parent_abs.0, ay - parent_abs.1);
    let visible = node.get("visible").and_then(V::boolean).unwrap_or(true);
    let opacity = n_or(node, "opacity", 1.0) as f32;
    // Figma rotation: radians, counter-clockwise positive in its docs;
    // our native convention is clockwise-positive in y-down space, which
    // matches Figma's on-screen behavior directly (both y-down) — Figma's
    // "rotation" field is already the visual angle, negated.
    let rotation = -n_or(node, "rotation", 0.0);
    let fill = first_fill(node, w, h);

    let kind = match ty {
        "FRAME" => ImportKind::Frame,
        "GROUP" => ImportKind::Group,
        "COMPONENT" | "COMPONENT_SET" => ImportKind::Component {
            name: s(node, "name").unwrap_or("Component").to_string(),
        },
        "INSTANCE" => {
            let cid = s(node, "componentId").unwrap_or("");
            ImportKind::Instance {
                component: components.get(cid).cloned().unwrap_or_else(|| cid.to_string()),
                text_overrides: vec![], // REST JSON bakes overrides into children
            }
        }
        "RECTANGLE" => ImportKind::Rect { radius: n_or(node, "cornerRadius", 0.0) },
        "ELLIPSE" => ImportKind::Ellipse,
        "LINE" => ImportKind::Line,
        "TEXT" => ImportKind::Text { content: s(node, "characters").unwrap_or("").to_string() },
        "VECTOR" | "STAR" | "REGULAR_POLYGON" | "BOOLEAN_OPERATION" => {
            // fillGeometry: [{path: "M...Z", windingRule}] — SVG path data
            // in node-local coords; reuse the SVG importer's d-parser (one
            // parser, shared semantics).
            let cmds = node.get("fillGeometry").and_then(V::arr)
                .and_then(|g| g.first())
                .and_then(|g0| s(g0, "path"))
                .map(parse_path_d)
                .unwrap_or_default();
            if cmds.is_empty() { ImportKind::Rect { radius: 0.0 } } else { ImportKind::Path { cmds } }
        }
        "SLICE" => return None,
        _ => return None, // unknown type: skip, never guess
    };

    let mut ir = ImportNode::new(kind).at(x, y).size(w, h);
    if let Some(id) = s(node, "id") { ir = ir.id(id); }
    ir.rotation = rotation;
    ir.opacity = opacity;
    ir.visible = visible;
    ir.fill = fill;
    ir.layout = figma_auto_layout(node);
    if let Some(strokes) = node.get("strokes").and_then(V::arr) {
        let mut solids = strokes.iter()
            .filter(|f| f.get("visible").and_then(V::boolean).unwrap_or(true))
            .filter(|f| s(f, "type") == Some("SOLID"))
            .map(|st| st.get("color").map(figma_color).unwrap_or(Color::BLACK));
        let weight = n_or(node, "strokeWeight", 1.0);
        if let Some(c) = solids.next() {
            ir.stroke = Some((c, weight));
        }
        // any additional stroke paints stack on top (same weight — Figma
        // only exposes one strokeWeight per node regardless of stack depth)
        ir.extra_strokes = solids.map(|c| (c, weight)).collect();
    }
    ir.effects = figma_effects(node);
    if let Some(children) = node.get("children").and_then(V::arr) {
        for c in children {
            if let Some(cn) = convert(c, (ax, ay), components) { ir.children.push(cn); }
        }
    }
    Some(ir)
}

/// Parse a Figma REST-API JSON document into the shared Import IR, then
/// lower to a native Document.
pub fn import_figma_json(text: &str) -> Result<Document, String> {
    let v = json::parse(text)?;
    let document = v.get("document").ok_or("not a Figma REST JSON file (no \"document\")")?;
    let mut components = HashMap::new();
    collect_component_names(&v, &mut components);

    let mut doc = ImportDoc { source: "figma", ..Default::default() };
    let pages = document.get("children").and_then(V::arr).ok_or("document has no pages")?;
    for page in pages {
        if s(page, "type") != Some("CANVAS") { continue; }
        let mut page_ir = ImportNode::new(ImportKind::Frame);
        if let Some(id) = s(page, "id") { page_ir = page_ir.id(id); }
        if let Some(children) = page.get("children").and_then(V::arr) {
            // page-level children are positioned at absolute canvas coords;
            // shift the envelope so content starts near the origin
            let (mut minx, mut miny) = (f64::MAX, f64::MAX);
            let mut kids = vec![];
            for c in children {
                if let Some(cn) = convert(c, (0.0, 0.0), &components) {
                    minx = minx.min(cn.x); miny = miny.min(cn.y);
                    kids.push(cn);
                }
            }
            if minx == f64::MAX { minx = 0.0; miny = 0.0; }
            for mut k in kids {
                k.x -= minx - 40.0;
                k.y -= miny - 40.0;
                page_ir.children.push(k);
            }
        }
        doc.pages.push(page_ir);
    }
    if doc.pages.is_empty() { return Err("figma file contains no canvases".into()); }
    Ok(crate::import_ir::lower(doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use x_core::NodeKind;

    const FIXTURE: &str = r##"{
      "name": "Test file",
      "components": { "1:10": { "name": "Chip" } },
      "document": {
        "id": "0:0", "type": "DOCUMENT",
        "children": [{
          "id": "0:1", "type": "CANVAS", "name": "Page 1",
          "children": [
            { "id": "1:1", "type": "FRAME", "name": "Screen",
              "absoluteBoundingBox": {"x": 100, "y": 200, "width": 400, "height": 300},
              "fills": [{"type": "SOLID", "color": {"r": 1, "g": 1, "b": 1, "a": 1}}],
              "children": [
                { "id": "1:2", "type": "RECTANGLE", "cornerRadius": 8,
                  "absoluteBoundingBox": {"x": 120, "y": 220, "width": 100, "height": 60},
                  "fills": [{"type": "GRADIENT_LINEAR",
                    "gradientHandlePositions": [{"x":0,"y":0},{"x":1,"y":0}],
                    "gradientStops": [
                      {"position": 0, "color": {"r": 1, "g": 0, "b": 0, "a": 1}},
                      {"position": 1, "color": {"r": 0, "g": 0, "b": 1, "a": 1}}]}] },
                { "id": "1:3", "type": "TEXT", "characters": "Hello Figma",
                  "absoluteBoundingBox": {"x": 120, "y": 300, "width": 200, "height": 24},
                  "fills": [] },
                { "id": "1:4", "type": "VECTOR",
                  "absoluteBoundingBox": {"x": 300, "y": 220, "width": 50, "height": 50},
                  "fillGeometry": [{"path": "M 0 0 L 50 0 L 25 50 Z"}],
                  "fills": [{"type": "SOLID", "color": {"r": 0, "g": 1, "b": 0, "a": 1}}] },
                { "id": "1:5", "type": "INSTANCE", "componentId": "1:10",
                  "absoluteBoundingBox": {"x": 300, "y": 300, "width": 80, "height": 30},
                  "fills": [] }
              ] }
          ] }]
      }
    }"##;

    #[test]
    fn imports_rest_json_through_the_shared_ir() {
        let doc = import_figma_json(FIXTURE).expect("import");
        assert_eq!(doc.pages.len(), 1);
        let frame = &doc.pages[0].children[0];
        assert!(matches!(frame.kind, NodeKind::Frame { .. }));
        assert_eq!((frame.w, frame.h), (400.0, 300.0));
        // children converted to PARENT-RELATIVE coords (120-100=20 …)
        let rect = &frame.children[0];
        assert_eq!((rect.transform.x, rect.transform.y), (20.0, 20.0));
        assert!(matches!(rect.kind, NodeKind::Rect { radius } if radius == 8.0));
        // gradient handles scaled to node pixels
        match &rect.fill {
            Paint::LinearGradient { start, end, .. } => {
                assert_eq!(*start, (0.0, 0.0));
                assert_eq!(*end, (100.0, 0.0));
            }
            other => panic!("expected gradient, got {other:?}"),
        }
        // shared-IR text default: black (empty fills array)
        let text = &frame.children[1];
        assert_eq!(text.fill, Paint::Solid(Color::BLACK));
        assert!(matches!(&text.kind, NodeKind::Text { text } if text == "Hello Figma"));
        // vector fillGeometry parsed via the SHARED svg d-parser
        let vec_node = &frame.children[2];
        match &vec_node.kind {
            NodeKind::Vector { path } => assert_eq!(path.len(), 4),
            other => panic!("expected vector, got {other:?}"),
        }
        // instance resolves componentId -> component name via the file map
        let inst = &frame.children[3];
        assert!(matches!(&inst.kind, NodeKind::Instance { component } if component == "Chip"));
        // ids are figma's (sanitized: "1:2" keeps the colon)
        assert_eq!(rect.id, "1:2");
    }

    #[test]
    fn non_figma_json_is_an_error() {
        assert!(import_figma_json("{}").is_err());
        assert!(import_figma_json("not json").is_err());
    }

    #[test]
    fn exported_figma_json_reimports_editable_nodes() {
        let mut doc = Document::new(); doc.pages.push(x_core::Node::frame("Page", 400.0, 300.0).child(x_core::Node::rect("Card", 10.0, 20.0, 120.0, 80.0, Color::rgb8(20, 40, 200)).radius(12.0)));
        let json = export_figma_json(&doc); let loaded = import_figma_json(&json).expect("own Figma JSON should import");
        assert_eq!(loaded.pages.len(), 1); assert_eq!(loaded.pages[0].children.len(), 1);
    }
}
