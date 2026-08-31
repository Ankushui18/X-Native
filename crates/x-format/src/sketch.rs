//! Sketch (`.sketch`) file importer.
//!
//! A `.sketch` file is a plain ZIP archive of JSON files — no binary
//! schema to decode (unlike `.fig`), which makes this the more tractable
//! importer. Scope is deliberately bounded to what's common and
//! well-defined across Sketch's format history rather than attempting
//! every layer class and every style option:
//!
//! **Covered**: pages, artboards, groups, rectangles (incl. uniform
//! corner radius), ovals, text (plain string content), generic vector
//! shapes (shapePath/shapeGroup/star/polygon/triangle — via their `points`
//! array, converted to cubic-bezier `PathCmd`s), symbol masters and
//! instances (mapped to our Component/Instance, with TEXT overrides in
//! our render-effective `"text:…"` encoding), solid and linear/radial
//! gradient fills, rotation, opacity, visibility, the full border stack
//! (`style.borders`, multiple solid strokes), and shadow/blur effects
//! (`style.shadows`, `style.innerShadows`, `style.blur`) round-tripping
//! through the shared `Effect` enum on both import and export.
//!
//! **Not covered** (produces a plain rect/vector fallback or is silently
//! dropped — never a panic): bitmap image decoding (produces an `Image`
//! node referencing an asset name, not decoded pixel data), per-character
//! text styling (only the plain string is kept), gradient/pattern borders
//! (solid borders only), boolean group operations, non-text symbol
//! overrides (fill/style/nested symbol swaps), and Sketch resizing
//! constraints beyond our HPin/VPin.

use crate::import_ir::{lower, ImportDoc, ImportKind, ImportNode};
use crate::json::{self, V};
use crate::zipfile::ZipArchive;
use std::collections::HashMap;
use x_core::{Color, Document, Effect, Paint, PathCmd};

fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n") }
fn sk_color(c: Color) -> String { format!("{{\"red\":{},\"green\":{},\"blue\":{},\"alpha\":{}}}", c.r as f64/255.0, c.g as f64/255.0, c.b as f64/255.0, c.a as f64/255.0) }
fn sk_fill(p: &Paint, w: f64, h: f64) -> String { match p {
    Paint::Solid(c) => format!("{{\"isEnabled\":true,\"fillType\":0,\"color\":{}}}", sk_color(*c)),
    Paint::Variable(_) => format!("{{\"isEnabled\":true,\"fillType\":0,\"color\":{}}}", sk_color(Color::BLACK)),
    Paint::LinearGradient { start,end,stops } => format!("{{\"isEnabled\":true,\"fillType\":1,\"gradient\":{{\"gradientType\":0,\"from\":\"{{{}, {}}}\",\"to\":\"{{{}, {}}}\",\"stops\":[{}]}}}}", start.0/w.max(1.0),start.1/h.max(1.0),end.0/w.max(1.0),end.1/h.max(1.0),stops.iter().map(|(t,c)|format!("{{\"position\":{t},\"color\":{}}}",sk_color(*c))).collect::<Vec<_>>().join(",")),
    Paint::RadialGradient { center,radius,stops } => format!("{{\"isEnabled\":true,\"fillType\":1,\"gradient\":{{\"gradientType\":1,\"from\":\"{{{}, {}}}\",\"to\":\"{{{}, {}}}\",\"stops\":[{}]}}}}", center.0/w.max(1.0),center.1/h.max(1.0),(center.0+radius)/w.max(1.0),center.1/h.max(1.0),stops.iter().map(|(t,c)|format!("{{\"position\":{t},\"color\":{}}}",sk_color(*c))).collect::<Vec<_>>().join(",")),
} }

fn sk_path_points(path: &[PathCmd], w: f64, h: f64) -> (String, bool) {
    let mut pts: Vec<((f64, f64), (f64, f64), (f64, f64))> = Vec::new();
    let mut closed = false;
    for cmd in path {
        match *cmd {
            PathCmd::MoveTo(x, y) => pts.push(((x, y), (x, y), (x, y))),
            PathCmd::LineTo(x, y) => pts.push(((x, y), (x, y), (x, y))),
            PathCmd::CurveTo(x1, y1, x2, y2, x, y) => {
                if let Some(last) = pts.last_mut() { last.1 = (x1, y1); }
                pts.push(((x, y), (x, y), (x2, y2)));
            }
            PathCmd::Close => closed = true,
        }
    }
    let norm = |p: (f64, f64)| (p.0 / w.max(1.0), p.1 / h.max(1.0));
    let json = pts.into_iter().map(|(p, from, to)| {
        let (px, py) = norm(p); let (fx, fy) = norm(from); let (tx, ty) = norm(to);
        format!("{{\"_class\":\"curvePoint\",\"point\":\"{{{px}, {py}}}\",\"curveFrom\":\"{{{fx}, {fy}}}\",\"curveTo\":\"{{{tx}, {ty}}}\"}}")
    }).collect::<Vec<_>>().join(",");
    (json, closed)
}

fn sk_layer(n: &x_core::Node) -> String {
    use x_core::NodeKind;
    let (class, extra) = match &n.kind {
        NodeKind::Frame { .. } => ("artboard", String::new()), NodeKind::Group => ("group", String::new()),
        NodeKind::Component { name } => ("symbolMaster", format!(",\"symbolID\":\"{}\"",esc(name))), NodeKind::Instance { component } => ("symbolInstance",format!(",\"symbolID\":\"{}\",\"overrideValues\":[]",esc(component))),
        NodeKind::Rect { radius } => ("rectangle",format!(",\"fixedRadius\":{radius}")), NodeKind::Ellipse => ("oval",String::new()),
        NodeKind::Line => ("shapePath", format!(",\"isClosed\":false,\"points\":[{{\"_class\":\"curvePoint\",\"point\":\"{{0, 0}}\"}},{{\"_class\":\"curvePoint\",\"point\":\"{{1, 1}}\"}}]")),
        NodeKind::Text { text } => ("text",format!(",\"attributedString\":{{\"string\":\"{}\"}}",esc(text))),
        NodeKind::Vector { path } => { let (points, closed) = sk_path_points(path, n.w, n.h); ("shapePath", format!(",\"isClosed\":{closed},\"points\":[{points}]")) },
        NodeKind::Image { asset,.. } => ("bitmap",format!(",\"image\":{{\"_ref\":\"images/{}.png\"}}",esc(asset.trim_start_matches("asset://")))),
        NodeKind::VectorNetwork(_) => ("shapePath", String::new()),
    };
    let fills=n.active_fills().iter().map(|l|sk_fill(&l.paint,n.w,n.h)).collect::<Vec<_>>().join(",");
    let borders = n.active_strokes().iter()
        .map(|s| format!("{{\"isEnabled\":true,\"color\":{},\"thickness\":{}}}", sk_color(s.stroke.color), s.stroke.width))
        .collect::<Vec<_>>().join(",");
    let (mut shadows, mut inner_shadows, mut blur) = (String::new(), String::new(), String::new());
    for e in n.active_effects().into_iter() {
        match e.effect {
            x_core::Effect::DropShadow { dx, dy, blur: b, color } => {
                if !shadows.is_empty() { shadows.push(','); }
                shadows.push_str(&format!("{{\"isEnabled\":true,\"offsetX\":{dx},\"offsetY\":{dy},\"blurRadius\":{b},\"color\":{}}}", sk_color(color)));
            }
            x_core::Effect::InnerShadow { dx, dy, blur: b, color } => {
                if !inner_shadows.is_empty() { inner_shadows.push(','); }
                inner_shadows.push_str(&format!("{{\"isEnabled\":true,\"offsetX\":{dx},\"offsetY\":{dy},\"blurRadius\":{b},\"color\":{}}}", sk_color(color)));
            }
            x_core::Effect::LayerBlur { radius } | x_core::Effect::BackgroundBlur { radius } => {
                // Sketch has one `blur` slot per layer; last one wins if the
                // node somehow carries more than one — rare in practice.
                blur = format!("\"blur\":{{\"isEnabled\":true,\"radius\":{radius}}},");
            }
        }
    }
    let children=if n.children.is_empty(){String::new()}else{format!(",\"layers\":[{}]",n.children.iter().map(sk_layer).collect::<Vec<_>>().join(","))};
    format!("{{\"_class\":\"{class}\",\"do_objectID\":\"{}\",\"name\":\"{}\",\"isVisible\":{},\"rotation\":{},\"frame\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"style\":{{\"contextSettings\":{{\"opacity\":{}}},{blur}\"fills\":[{fills}],\"borders\":[{borders}],\"shadows\":[{shadows}],\"innerShadows\":[{inner_shadows}]}}{extra}{children}}}",esc(&n.id),esc(&n.id),n.visible,-n.transform.rotation.to_degrees(),n.transform.x,n.transform.y,n.w,n.h,n.opacity)
}

/// Export a real Sketch ZIP package using the documented JSON package shape.
pub fn export_sketch(doc: &Document) -> Vec<u8> {
    let refs=doc.pages.iter().enumerate().map(|(i,_)|format!("{{\"_class\":\"MSJSONFileReference\",\"_ref_class\":\"MSImmutablePage\",\"_ref\":\"pages/page-{}\"}}",i+1)).collect::<Vec<_>>().join(",");
    let mut files=vec![("document.json".into(),format!("{{\"_class\":\"document\",\"do_objectID\":\"x-designer\",\"pages\":[{refs}]}}").into_bytes()),("meta.json".into(),b"{\"app\":\"X Designer\",\"version\":1}".to_vec())];
    for (i,p) in doc.pages.iter().enumerate(){files.push((format!("pages/page-{}.json",i+1),format!("{{\"_class\":\"page\",\"do_objectID\":\"{}\",\"name\":\"{}\",\"layers\":[{}]}}",esc(&p.id),esc(&p.id),p.children.iter().map(sk_layer).collect::<Vec<_>>().join(",")).into_bytes()));}
    for asset in doc.assets.embedded_sorted() {
        if asset.mime == "image/png" {
            files.push((format!("images/{}.png", asset.hash), asset.bytes.clone()));
        }
    }
    crate::zipfile::write_stored(&files)
}

fn s<'a>(v: &'a V, key: &str) -> Option<&'a str> { v.get(key).and_then(|x| x.str()) }
fn n(v: &V, key: &str) -> Option<f64> { v.get(key).and_then(|x| x.num()) }
fn n_or(v: &V, key: &str, default: f64) -> f64 { n(v, key).unwrap_or(default) }
fn b_or(v: &V, key: &str, default: bool) -> bool { v.get(key).and_then(|x| x.boolean()).unwrap_or(default) }

fn frame_xywh(v: &V) -> (f64, f64, f64, f64) {
    match v.get("frame") {
        Some(f) => (n_or(f, "x", 0.0), n_or(f, "y", 0.0), n_or(f, "width", 0.0), n_or(f, "height", 0.0)),
        None => (0.0, 0.0, 0.0, 0.0),
    }
}

/// Sketch color objects are `{red, green, blue, alpha}` floats in 0..=1 —
/// directly compatible with peniko's `Color::rgba(f64...)`, no scaling.
fn sketch_color(v: &V) -> Option<Color> {
    Some(Color::rgba(n_or(v, "red", 0.0), n_or(v, "green", 0.0), n_or(v, "blue", 0.0), n_or(v, "alpha", 1.0)))
}

/// The first enabled fill in a layer's `style.fills` array, as our
/// `Paint`. Solid fills (`fillType` 0) map directly. Gradients
/// (`fillType` 1) dispatch on `gradient.gradientType`: 0 linear,
/// 1 radial, 2 angular. Angular sweeps have no equivalent in our `Paint`
/// enum and fall back to a linear gradient between the same anchors
/// rather than being dropped silently.
fn first_fill(layer: &V) -> Option<Paint> {
    // AUDIT FIX (caught live): Sketch gradient anchors are NORMALIZED
    // 0..1 within the layer frame; our Paint gradients are node-local
    // PIXELS. Without scaling, "{0,0}"->"{1,0}" is a 1px gradient that
    // renders as a near-solid end color.
    let (_, _, fw, fh) = frame_xywh(layer);
    let fills = layer.get("style")?.get("fills")?.arr()?;
    let fill = fills.iter().find(|f| b_or(f, "isEnabled", true))?;
    let fill_type = n_or(fill, "fillType", 0.0) as i64;
    if fill_type == 0 {
        return fill.get("color").and_then(sketch_color).map(Paint::Solid);
    }
    let g = fill.get("gradient")?;
    let stops: Vec<(f32, Color)> = g.get("stops")?.arr()?.iter()
        .filter_map(|st| Some((n_or(st, "position", 0.0) as f32, sketch_color(st.get("color")?)?)))
        .collect();
    if stops.is_empty() { return None; }
    let raw_from = g.get("from").and_then(point_pair).unwrap_or((0.0, 0.0));
    let raw_to = g.get("to").and_then(point_pair).unwrap_or((1.0, 1.0));
    let from = (raw_from.0 * fw, raw_from.1 * fh);
    let to = (raw_to.0 * fw, raw_to.1 * fh);
    let gradient_kind = n_or(g, "gradientType", 0.0) as i64;
    if gradient_kind == 1 {
        let radius = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        Some(Paint::RadialGradient { center: from, radius, stops })
    } else {
        Some(Paint::LinearGradient { start: from, end: to, stops })
    }
}

/// Sketch encodes gradient/point positions as the string `"{x, y}"`
/// (curly braces, not JSON) — a real, deliberate format quirk.
fn point_pair(v: &V) -> Option<(f64, f64)> {
    let raw = v.str()?;
    let inner = raw.trim_start_matches('{').trim_end_matches('}');
    let mut parts = inner.split(',').map(|p| p.trim().parse::<f64>());
    Some((parts.next()?.ok()?, parts.next()?.ok()?))
}

/// AUDIT FIX (was a real bug in the draft): layer opacity lives at
/// `style.contextSettings.opacity` — the draft called `.num()` on the
/// contextSettings OBJECT itself, which is always None, so every layer
/// imported fully opaque.
fn opacity(layer: &V) -> f32 {
    layer.get("style")
        .and_then(|s| s.get("contextSettings"))
        .and_then(|c| c.get("opacity"))
        .and_then(|o| o.num())
        .unwrap_or(1.0) as f32
}

/// Converts a Sketch `points` array (each point has a normalized 0..1
/// `point`/`curveFrom`/`curveTo` position within the shape's own frame)
/// into absolute-within-frame cubic-bezier `PathCmd`s. Every point is
/// treated as a bezier vertex whether or not it actually has curve
/// handles — a straight segment's handles just equal its own point,
/// which produces a degenerate (but correct) cubic equal to the line.
fn shape_points_to_path(points: &[V], w: f64, h: f64, closed: bool) -> Vec<PathCmd> {
    let abs = |p: (f64, f64)| (p.0 * w, p.1 * h);
    let pts: Vec<((f64, f64), (f64, f64), (f64, f64))> = points.iter().filter_map(|p| {
        let pos = point_pair(p.get("point")?)?;
        let curve_from = p.get("curveFrom").and_then(point_pair).unwrap_or(pos);
        let curve_to = p.get("curveTo").and_then(point_pair).unwrap_or(pos);
        Some((abs(pos), abs(curve_from), abs(curve_to)))
    }).collect();
    if pts.is_empty() { return vec![]; }
    let mut out = vec![PathCmd::MoveTo(pts[0].0.0, pts[0].0.1)];
    let n_pts = pts.len();
    let last_idx = if closed { n_pts } else { n_pts - 1 };
    for i in 0..last_idx {
        let (_, cur_curve_from, _) = pts[i];
        let (next_pos, _, next_curve_to) = pts[(i + 1) % n_pts];
        out.push(PathCmd::CurveTo(cur_curve_from.0, cur_curve_from.1, next_curve_to.0, next_curve_to.1, next_pos.0, next_pos.1));
    }
    if closed { out.push(PathCmd::Close); }
    out
}

/// Registry of symbolID -> human name, built by pre-scanning every page
/// (Sketch documents commonly keep a dedicated "Symbols" page, but
/// there's no rule requiring it) for `symbolMaster` layers before
/// converting anything — an instance can appear before its master.
fn collect_symbol_masters(layer: &V, out: &mut HashMap<String, String>) {
    if s(layer, "_class") == Some("symbolMaster") {
        if let (Some(sid), Some(name)) = (s(layer, "symbolID"), s(layer, "name")) {
            out.insert(sid.to_string(), name.to_string());
        }
    }
    if let Some(children) = layer.get("layers").and_then(|v| v.arr()) {
        for c in children { collect_symbol_masters(c, out); }
    }
}

/// Sketch `style.borders` (strokes). Each enabled solid border becomes a
/// stroke layer; the first is the primary `ir.stroke`, any rest stack as
/// `extra_strokes` (same convention as the Figma importer).
fn sketch_strokes(layer: &V) -> (Option<(Color, f64)>, Vec<(Color, f64)>) {
    let Some(borders) = layer.get("style").and_then(|s| s.get("borders")).and_then(V::arr) else { return (None, vec![]) };
    let mut solids = borders.iter()
        .filter(|b| b_or(b, "isEnabled", true))
        .filter_map(|b| Some((b.get("color").and_then(sketch_color)?, n_or(b, "thickness", 1.0))));
    (solids.next(), solids.collect())
}

/// Sketch `style.shadows` / `style.innerShadows` / `style.blur` — the
/// same shadow/blur vocabulary Figma exposes, so it lowers onto the same
/// shared `Effect` enum.
fn sketch_effects(layer: &V) -> Vec<Effect> {
    let mut out = vec![];
    let style = layer.get("style");
    if let Some(shadows) = style.and_then(|s| s.get("shadows")).and_then(V::arr) {
        for sh in shadows.iter().filter(|s| b_or(s, "isEnabled", true)) {
            let color = sh.get("color").and_then(sketch_color).unwrap_or(Color::rgba8(0, 0, 0, 255));
            out.push(Effect::DropShadow { dx: n_or(sh, "offsetX", 0.0), dy: n_or(sh, "offsetY", 0.0), blur: n_or(sh, "blurRadius", 0.0), color });
        }
    }
    if let Some(shadows) = style.and_then(|s| s.get("innerShadows")).and_then(V::arr) {
        for sh in shadows.iter().filter(|s| b_or(s, "isEnabled", true)) {
            let color = sh.get("color").and_then(sketch_color).unwrap_or(Color::rgba8(0, 0, 0, 255));
            out.push(Effect::InnerShadow { dx: n_or(sh, "offsetX", 0.0), dy: n_or(sh, "offsetY", 0.0), blur: n_or(sh, "blurRadius", 0.0), color });
        }
    }
    if let Some(blur) = style.and_then(|s| s.get("blur")) {
        if b_or(blur, "isEnabled", false) {
            out.push(Effect::LayerBlur { radius: n_or(blur, "radius", 0.0) });
        }
    }
    out
}

fn convert_layer(layer: &V, symbol_names: &HashMap<String, String>, diags: &mut Vec<String>) -> Option<ImportNode> {
    let class = s(layer, "_class")?;
    let (x, y, w, h) = frame_xywh(layer);
    let rotation_deg = n_or(layer, "rotation", 0.0);
    let fill = first_fill(layer);

    let kind = match class {
        "artboard" => ImportKind::Frame,
        "symbolMaster" => ImportKind::Component { name: s(layer, "name").unwrap_or("Symbol").to_string() },
        "group" | "shapeGroup" => ImportKind::Group,
        "rectangle" => {
            let radius = layer.get("points").and_then(|v| v.arr())
                .and_then(|pts| pts.first())
                .and_then(|p| p.get("cornerRadius")).and_then(|v| v.num())
                .or_else(|| n(layer, "fixedRadius"))
                .unwrap_or(0.0);
            ImportKind::Rect { radius }
        }
        "oval" => ImportKind::Ellipse,
        "text" => ImportKind::Text {
            content: layer.get("attributedString").and_then(|a| s(a, "string")).unwrap_or("").to_string(),
        },
        "shapePath" | "star" | "polygon" | "triangle" => {
            let points = layer.get("points").and_then(|v| v.arr()).cloned().unwrap_or_default();
            let closed = b_or(layer, "isClosed", true);
            ImportKind::Path { cmds: shape_points_to_path(&points, w, h, closed) }
        }
        "symbolInstance" => {
            let symbol_id = s(layer, "symbolID").unwrap_or("");
            let name = symbol_names.get(symbol_id).cloned().unwrap_or_else(|| symbol_id.to_string());
            let mut text_overrides = vec![];
            if let Some(overrides) = layer.get("overrideValues").and_then(|v| v.arr()) {
                for ov in overrides {
                    let (Some(prop), Some(value)) = (s(ov, "overrideName"), s(ov, "value")) else { continue };
                    // "text:" encoding is the IR lowering's job now — the
                    // importer just names the target layer.
                    if let Some(target) = prop.strip_suffix("_stringValue") {
                        text_overrides.push((target.to_string(), value.to_string()));
                    }
                }
            }
            ImportKind::Instance { component: name, text_overrides }
        }
        "bitmap" => {
            // real reference: layer.image._ref = "images/<sha>.png" —
            // matches the ImportDoc.assets key (the zip path stem), so
            // lower() rewrites it to the content-addressed asset:// id.
            // Layer name is only the fallback for malformed files.
            let asset = layer.get("image").and_then(|i| s(i, "_ref"))
                .map(|r| r.trim_start_matches("images/").trim_end_matches(".png").to_string())
                .unwrap_or_else(|| s(layer, "name").unwrap_or("image").to_string());
            ImportKind::Image { asset }
        }
        other => {
            diags.push(format!("sketch: skipped unsupported layer class '{other}'"));
            return None;
        }
    };

    let mut ir = ImportNode::new(kind).at(x, y).size(w, h);
    if let Some(id) = s(layer, "do_objectID") { ir = ir.id(id); }
    ir.fill = fill.clone();
    let (stroke, extra_strokes) = sketch_strokes(layer);
    ir.stroke = stroke;
    ir.extra_strokes = extra_strokes;
    ir.effects = sketch_effects(layer);
    // Sketch rotates clockwise-positive; native convention is CCW-positive.
    ir.rotation = -rotation_deg.to_radians();
    ir.opacity = opacity(layer);
    ir.visible = b_or(layer, "isVisible", true);

    if let Some(children) = layer.get("layers").and_then(|v| v.arr()) {
        for child in children {
            if let Some(cn) = convert_layer(child, symbol_names, diags) { ir.children.push(cn); }
        }
    }
    // Sketch shapeGroups carry the style; child shapePaths have none —
    // propagate the group fill to fill-less child paths (source-format
    // quirk, so it belongs HERE, not in the shared lowering).
    if class == "shapeGroup" {
        if let Some(p) = fill {
            for c in &mut ir.children {
                if matches!(c.kind, ImportKind::Path { .. }) && c.fill.is_none() {
                    c.fill = Some(p.clone());
                }
            }
        }
    }
    Some(ir)
}

pub fn import_sketch(bytes: &[u8]) -> Result<Document, String> {
    import_sketch_with_report(bytes).map(|(d, _)| d)
}

/// Import + fidelity report (import diagnostics UI).
pub fn import_sketch_with_report(bytes: &[u8]) -> Result<(Document, crate::ImportReport), String> {
    let archive = ZipArchive::open(bytes)?;
    let document_bytes = archive.read("document.json")?;
    let document_text = String::from_utf8_lossy(&document_bytes);
    let document_v = json::parse(&document_text)?;

    let page_paths: Vec<String> = document_v.get("pages").and_then(|v| v.arr())
        .map(|arr| arr.iter().filter_map(|p| s(p, "_ref").map(|r| format!("{r}.json"))).collect())
        .unwrap_or_default();

    let mut page_values: Vec<V> = Vec::new();
    for path in &page_paths {
        let raw = archive.read(path)?;
        let text = String::from_utf8_lossy(&raw);
        page_values.push(json::parse(&text)?);
    }
    // Fallback for older/nonstandard files that don't use document.json's
    // page-reference list: pick up every pages/*.json entry directly.
    if page_values.is_empty() {
        let matched = archive.read_matching(|name| name.starts_with("pages/") && name.ends_with(".json"))?;
        for raw in matched.values() {
            let text = String::from_utf8_lossy(raw);
            page_values.push(json::parse(&text)?);
        }
    }
    if page_values.is_empty() { return Err("sketch file contains no pages".into()); }

    let mut symbol_names = HashMap::new();
    for pv in &page_values { collect_symbol_masters(pv, &mut symbol_names); }

    // Sketch embeds bitmap assets under images/ — carry them in the IR
    // so the app shell can register them with the Assets loader.
    let mut doc = ImportDoc { source: "sketch", ..Default::default() };
    if let Ok(images) = archive.read_matching(|n| n.starts_with("images/") && n.ends_with(".png")) {
        for (name, data) in images {
            let stem = name.trim_start_matches("images/").trim_end_matches(".png").to_string();
            doc.assets.push((stem, data));
        }
    }
    for pv in &page_values {
        let mut page_ir = ImportNode::new(ImportKind::Frame);
        if let Some(id) = s(pv, "do_objectID") { page_ir = page_ir.id(id); }
        if let Some(layers) = pv.get("layers").and_then(|v| v.arr()) {
            for layer in layers {
                if let Some(n) = convert_layer(layer, &symbol_names, &mut doc.diagnostics) { page_ir.children.push(n); }
            }
        }
        doc.pages.push(page_ir);
    }
    Ok(crate::lower_with_report(doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use x_core::NodeKind;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        // Local minimal stored-entry zip writer mirroring zipfile.rs's
        // test helper — kept separate since it's only a fixture builder.
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, content) in files {
            let local_offset = out.len() as u32;
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(content.len() as u32).to_le_bytes());
            out.extend_from_slice(&(content.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(content);

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&(content.len() as u32).to_le_bytes());
            central.extend_from_slice(&(content.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&local_offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    const DOCUMENT_JSON: &str = r#"{"_class":"document","do_objectID":"doc-1","pages":[{"_class":"MSJSONFileReference","_ref_class":"MSImmutablePage","_ref":"pages/page-1"}]}"#;

    #[test]
    fn imports_a_rectangle_with_solid_fill_and_rotation() {
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"rectangle","do_objectID":"r1","name":"Rect","isVisible":true,"rotation":90,
             "frame":{"x":10,"y":20,"width":100,"height":50},
             "style":{"fills":[{"isEnabled":true,"fillType":0,"color":{"red":1,"green":0,"blue":0,"alpha":1}}]}}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        assert_eq!(doc.pages.len(), 1);
        let rect = &doc.pages[0].children[0];
        assert!(matches!(rect.kind, NodeKind::Rect { .. }));
        assert_eq!(rect.transform.x, 10.0);
        assert_eq!(rect.transform.y, 20.0);
        assert_eq!(rect.w, 100.0);
        assert_eq!(rect.h, 50.0);
        assert_eq!(rect.fill, Paint::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
        // Sketch rotates clockwise-positive; we store counter-clockwise-positive.
        assert!((rect.transform.rotation - (-std::f64::consts::FRAC_PI_2)).abs() < 1e-9);
    }

    #[test]
    fn exported_sketch_package_reimports() {
        let mut source=Document::new(); source.pages.push(x_core::Node::frame("Page",400.0,300.0).child(x_core::Node::ellipse("Dot",12.0,18.0,40.0,40.0,Color::rgb8(200,20,80))));
        let bytes=export_sketch(&source); let loaded=import_sketch(&bytes).expect("own Sketch package should import");
        assert_eq!(loaded.pages.len(),1); assert_eq!(loaded.pages[0].children.len(),1);
    }

    #[test]
    fn imports_a_linear_gradient_fill() {
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"oval","do_objectID":"o1","isVisible":true,"frame":{"x":0,"y":0,"width":10,"height":10},
             "style":{"fills":[{"isEnabled":true,"fillType":1,"gradient":{"gradientType":0,
                "from":"{0, 0}","to":"{1, 1}",
                "stops":[{"position":0,"color":{"red":0,"green":0,"blue":0,"alpha":1}},
                         {"position":1,"color":{"red":1,"green":1,"blue":1,"alpha":1}}]}}]}}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        match &doc.pages[0].children[0].fill {
            Paint::LinearGradient { start, end, stops } => {
                assert_eq!(*start, (0.0, 0.0));
                // anchors scale from normalized 0..1 to the 10x10 frame
                assert_eq!(*end, (10.0, 10.0));
                assert_eq!(stops.len(), 2);
            }
            other => panic!("expected LinearGradient, got {other:?}"),
        }
    }

    #[test]
    fn imports_symbol_master_and_instance_with_render_effective_text_override() {
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"symbolMaster","do_objectID":"m1","symbolID":"sym-abc","name":"Button","isVisible":true,
             "frame":{"x":0,"y":0,"width":100,"height":40},"layers":[
                {"_class":"text","do_objectID":"label","isVisible":true,"frame":{"x":10,"y":10,"width":80,"height":20},
                 "attributedString":{"string":"OK"},"style":{"fills":[]}}
             ]},
            {"_class":"symbolInstance","do_objectID":"i1","symbolID":"sym-abc","isVisible":true,
             "frame":{"x":200,"y":0,"width":100,"height":40},
             "overrideValues":[{"overrideName":"label_stringValue","value":"Save"}]}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        let master = &doc.pages[0].children[0];
        assert!(matches!(&master.kind, NodeKind::Component { name } if name == "Button"));
        let instance = &doc.pages[0].children[1];
        match &instance.kind {
            NodeKind::Instance { component } => assert_eq!(component, "Button"),
            other => panic!("expected Instance, got {other:?}"),
        }
        // AUDIT: must be render-effective — "text:" prefix, keyed by the
        // target layer id, exactly what build_render_tree consumes.
        assert_eq!(instance.overrides.get("label"), Some(&"text:Save".to_string()));
        assert_eq!(instance.transform.x, 200.0, "instance position preserved");
        // and the override actually takes effect through the render IR:
        let tree = x_render::build_render_tree(&doc.pages[0], &x_core::Variables::default());
        let texts: Vec<String> = tree.commands.iter().filter_map(|c| match c {
            x_render::RenderCommand::Glyphs { text, .. } => Some(text.clone()),
            _ => None,
        }).collect();
        assert!(texts.contains(&"Save".to_string()), "override renders: {texts:?}");
    }

    #[test]
    fn artboard_and_group_positions_survive() {
        // AUDIT regression test: draft dropped x/y for artboards + groups
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"artboard","do_objectID":"a1","isVisible":true,
             "frame":{"x":100,"y":50,"width":400,"height":300},"layers":[
                {"_class":"group","do_objectID":"g1","isVisible":true,
                 "frame":{"x":25,"y":35,"width":100,"height":80},"layers":[]}
             ]}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        let ab = &doc.pages[0].children[0];
        assert_eq!((ab.transform.x, ab.transform.y), (100.0, 50.0));
        let g = &ab.children[0];
        assert_eq!((g.transform.x, g.transform.y), (25.0, 35.0));
        // page auto-sized to content envelope, not left 0x0
        assert!(doc.pages[0].w >= 500.0 && doc.pages[0].h >= 350.0);
    }

    #[test]
    fn layer_opacity_is_read_from_context_settings() {
        // AUDIT regression test: draft read contextSettings as a number
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"rectangle","do_objectID":"r1","isVisible":true,
             "frame":{"x":0,"y":0,"width":10,"height":10},
             "style":{"contextSettings":{"_class":"graphicsContextSettings","blendMode":0,"opacity":0.35},
                      "fills":[{"isEnabled":true,"fillType":0,"color":{"red":0,"green":0,"blue":1,"alpha":1}}]}}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        assert!((doc.pages[0].children[0].opacity - 0.35).abs() < 1e-6);
    }

    #[test]
    fn shape_group_fill_propagates_to_child_paths() {
        // AUDIT regression test: draft left combined-shape children
        // transparent (Sketch styles live on the shapeGroup, not children)
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"shapeGroup","do_objectID":"sg1","isVisible":true,
             "frame":{"x":0,"y":0,"width":100,"height":100},
             "style":{"fills":[{"isEnabled":true,"fillType":0,"color":{"red":0,"green":1,"blue":0,"alpha":1}}]},
             "layers":[
                {"_class":"shapePath","do_objectID":"p1","isVisible":true,"isClosed":true,
                 "frame":{"x":0,"y":0,"width":100,"height":100},
                 "points":[{"point":"{0, 0}"},{"point":"{1, 0}"},{"point":"{1, 1}"}]}
             ]}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        let child = &doc.pages[0].children[0].children[0];
        assert!(matches!(child.kind, NodeKind::Vector { .. }));
        assert_eq!(child.fill, Paint::Solid(Color::rgba(0.0, 1.0, 0.0, 1.0)));
    }

    #[test]
    fn hidden_layer_is_kept_with_visible_false() {
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"rectangle","do_objectID":"r1","isVisible":false,"frame":{"x":0,"y":0,"width":1,"height":1},"style":{"fills":[]}},
            {"_class":"rectangle","do_objectID":"r2","isVisible":true,"frame":{"x":5,"y":5,"width":1,"height":1},"style":{"fills":[]}}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        assert_eq!(doc.pages[0].children.len(), 2);
        assert!(!doc.pages[0].children[0].visible);
        assert!(doc.pages[0].children[1].visible);
    }

    #[test]
    fn vector_star_converts_points_to_beziers() {
        let page = r#"{"_class":"page","do_objectID":"page-1","layers":[
            {"_class":"star","do_objectID":"s1","isVisible":true,"isClosed":true,
             "frame":{"x":0,"y":0,"width":100,"height":100},
             "style":{"fills":[{"isEnabled":true,"fillType":0,"color":{"red":1,"green":1,"blue":0,"alpha":1}}]},
             "points":[
               {"point":"{0.5, 0}","curveFrom":"{0.5, 0}","curveTo":"{0.5, 0}"},
               {"point":"{1, 1}","curveFrom":"{1, 1}","curveTo":"{1, 1}"},
               {"point":"{0, 1}","curveFrom":"{0, 1}","curveTo":"{0, 1}"}]}
        ]}"#;
        let zip = zip_of(&[("document.json", DOCUMENT_JSON.as_bytes()), ("pages/page-1.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("should import");
        match &doc.pages[0].children[0].kind {
            NodeKind::Vector { path } => {
                assert!(matches!(path[0], PathCmd::MoveTo(x, y) if x == 50.0 && y == 0.0));
                assert_eq!(path.iter().filter(|c| matches!(c, PathCmd::CurveTo(..))).count(), 3);
                assert!(matches!(path.last(), Some(PathCmd::Close)));
            }
            other => panic!("expected Vector, got {other:?}"),
        }
    }

    #[test]
    fn malformed_document_json_is_an_error_not_a_panic() {
        let zip = zip_of(&[("document.json", b"not json")]);
        assert!(import_sketch(&zip).is_err());
        assert!(import_sketch(b"not a zip at all").is_err());
    }

    #[test]
    fn pages_fallback_scan_without_document_refs() {
        // document.json with no "pages" array -> fallback to pages/*.json
        let docjson = br#"{"_class":"document","do_objectID":"doc-1"}"#;
        let page = r#"{"_class":"page","do_objectID":"page-9","layers":[
            {"_class":"oval","do_objectID":"o1","isVisible":true,"frame":{"x":0,"y":0,"width":10,"height":10},"style":{"fills":[]}}
        ]}"#;
        let zip = zip_of(&[("document.json", docjson), ("pages/page-9.json", page.as_bytes())]);
        let doc = import_sketch(&zip).expect("fallback scan works");
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].children.len(), 1);
    }
}
