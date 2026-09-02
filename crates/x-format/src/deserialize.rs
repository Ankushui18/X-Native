use x_core::*;
use crate::json::{P, V};
#[allow(unused_imports)]
use crate::*;

fn parse_paint(v: &V) -> Paint {
    let t = v.get("t").and_then(V::str).unwrap_or("solid");
    match t {
        "var" => Paint::Variable(v.get("name").and_then(V::str).unwrap_or("").into()),
        "linear" => Paint::LinearGradient {
            start: (v.get("x0").and_then(V::num).unwrap_or(0.0), v.get("y0").and_then(V::num).unwrap_or(0.0)),
            end: (v.get("x1").and_then(V::num).unwrap_or(0.0), v.get("y1").and_then(V::num).unwrap_or(0.0)),
            stops: parse_stops(v),
        },
        "radial" => Paint::RadialGradient {
            center: (v.get("cx").and_then(V::num).unwrap_or(0.0), v.get("cy").and_then(V::num).unwrap_or(0.0)),
            radius: v.get("r").and_then(V::num).unwrap_or(0.0),
            stops: parse_stops(v),
        },
        _ => Paint::Solid(v.get("c").and_then(V::str).and_then(parse_hex_color).unwrap_or(Color::TRANSPARENT)),
    }
}
fn parse_stops(v: &V) -> Vec<(f32, Color)> {
    v.get("stops").and_then(V::arr).map(|a| {
        a.iter().filter_map(|s| {
            let pair = s.arr()?;
            Some((pair.first()?.num()? as f32, parse_hex_color(pair.get(1)?.str()?)?))
        }).collect()
    }).unwrap_or_default()
}

fn parse_blend(v: Option<&str>) -> BlendKind {
    match v { Some("darken") => BlendKind::Darken, Some("multiply") => BlendKind::Multiply, Some("color-burn") => BlendKind::ColorBurn, Some("lighten") => BlendKind::Lighten, Some("screen") => BlendKind::Screen, Some("color-dodge") => BlendKind::ColorDodge, Some("overlay") => BlendKind::Overlay, Some("soft-light") => BlendKind::SoftLight, Some("hard-light") => BlendKind::HardLight, Some("difference") => BlendKind::Difference, Some("exclusion") => BlendKind::Exclusion, Some("hue") => BlendKind::Hue, Some("saturation") => BlendKind::Saturation, Some("color") => BlendKind::Color, Some("luminosity") => BlendKind::Luminosity, _ => BlendKind::Normal }
}
fn parse_cap(v: Option<&str>) -> StrokeCap {
    match v { Some("round") => StrokeCap::Round, Some("square") => StrokeCap::Square, Some("arrow") => StrokeCap::Arrow, Some("triangle") => StrokeCap::Triangle, _ => StrokeCap::None }
}

fn parse_effect(v: &V) -> Option<Effect> {
    let c = v.get("c").and_then(V::str).and_then(parse_hex_color).unwrap_or(Color::BLACK);
    let (dx, dy, blur) = (v.get("dx").and_then(V::num).unwrap_or(0.0), v.get("dy").and_then(V::num).unwrap_or(0.0), v.get("blur").and_then(V::num).unwrap_or(0.0));
    match v.get("t").and_then(V::str) {
        Some("drop") => Some(Effect::DropShadow { dx, dy, blur, color: c }),
        Some("inner") => Some(Effect::InnerShadow { dx, dy, blur, color: c }),
        Some("blur") => Some(Effect::LayerBlur { radius: v.get("r").and_then(V::num).unwrap_or(0.0) }),
        Some("bgblur") => Some(Effect::BackgroundBlur { radius: v.get("r").and_then(V::num).unwrap_or(0.0) }),
        _ => None,
    }
}

fn parse_kind(v: &V) -> NodeKind {
    match v.get("t").and_then(V::str).unwrap_or("frame") {
        "group" => NodeKind::Group,
        "rect" => NodeKind::Rect { radius: v.get("radius").and_then(V::num).unwrap_or(0.0) },
        "ellipse" => NodeKind::Ellipse,
        "line" => NodeKind::Line,
        "text" => NodeKind::Text { text: v.get("text").and_then(V::str).unwrap_or("").into() },
        "image" => NodeKind::Image {
            asset: v.get("asset").and_then(V::str).unwrap_or("").into(),
            placement: ImagePlacement {
                focal: (v.get("fx").and_then(V::num).unwrap_or(0.5), v.get("fy").and_then(V::num).unwrap_or(0.5)),
                scale: v.get("scale").and_then(V::num).unwrap_or(1.0),
                flip_h: v.get("fliph").and_then(V::boolean).unwrap_or(false),
                flip_v: v.get("flipv").and_then(V::boolean).unwrap_or(false),
            },
            fit: match v.get("fit").and_then(V::str) {
                Some("fit") => ImageFit::Fit,
                Some("crop") => ImageFit::Crop,
                Some("tile") => ImageFit::Tile,
                _ => ImageFit::Fill,
            },
        },
        "vector" => NodeKind::Vector {
            path: v.get("path").and_then(V::arr).map(|a| {
                a.iter().filter_map(|cmd| {
                    let c = cmd.arr()?;
                    match c.first()?.str()? {
                        "M" => Some(PathCmd::MoveTo(c.get(1)?.num()?, c.get(2)?.num()?)),
                        "L" => Some(PathCmd::LineTo(c.get(1)?.num()?, c.get(2)?.num()?)),
                        "C" => Some(PathCmd::CurveTo(c.get(1)?.num()?, c.get(2)?.num()?, c.get(3)?.num()?, c.get(4)?.num()?, c.get(5)?.num()?, c.get(6)?.num()?)),
                        "Z" => Some(PathCmd::Close),
                        _ => None,
                    }
                }).collect()
            }).unwrap_or_default(),
        },
        "component" => NodeKind::Component { name: v.get("name").and_then(V::str).unwrap_or("").into() },
        "instance" => NodeKind::Instance { component: v.get("component").and_then(V::str).unwrap_or("").into() },
        _ => {
            let layout = v.get("layout").map(|l| AutoLayout {
                direction: if l.get("dir").and_then(V::str) == Some("h") { LayoutDirection::Horizontal } else { LayoutDirection::Vertical },
                gap: l.get("gap").and_then(V::num).unwrap_or(0.0),
                padding: parse_padding(l.get("padding")),
                sizing: if l.get("sizing").and_then(V::str) == Some("hug") { Sizing::Hug } else { Sizing::Fixed },
                align: match l.get("align").and_then(V::str) { Some("center") => CrossAlign::Center, Some("end") => CrossAlign::End, _ => CrossAlign::Start },
                space_between: l.get("space_between").and_then(V::boolean).unwrap_or(false),
                cross_sizing: l.get("cross_sizing").and_then(V::str).map(|s| if s == "hug" { Sizing::Hug } else { Sizing::Fixed }),
                gap_var: l.get("gap_var").and_then(V::str).map(String::from),
                padding_var: l.get("padding_var").and_then(V::str).map(String::from),
                max_height: Some(f64::INFINITY),
                max_width: Some(f64::INFINITY),
                min_height: Some(0.0),
                min_width: Some(0.0),
                wrap: x_core::AutoLayoutWrap::NoWrap,
                resize_on_wrap: true,
            });
            NodeKind::Frame { layout }
        }
    }
}

/// `"padding"` accepts the legacy scalar (all four sides) or the
/// `[left, right, top, bottom]` array written for non-uniform padding.
pub(crate) fn parse_padding(v: Option<&V>) -> Padding {
    match v {
        Some(V::Num(n)) => [*n; 4],
        Some(V::Arr(a)) if a.len() == 4 => [
            a[0].num().unwrap_or(0.0), a[1].num().unwrap_or(0.0),
            a[2].num().unwrap_or(0.0), a[3].num().unwrap_or(0.0),
        ],
        _ => [0.0; 4],
    }
}

pub(crate) fn parse_node(v: &V) -> Node {
    let kind = v.get("kind").map(parse_kind).unwrap_or(NodeKind::Group);
    let mut n = Node::frame("", 0.0, 0.0);
    n.kind = kind;
    n.id = v.get("id").and_then(V::str).unwrap_or("").into();
    n.transform.x = v.get("x").and_then(V::num).unwrap_or(0.0);
    n.transform.y = v.get("y").and_then(V::num).unwrap_or(0.0);
    n.transform.rotation = v.get("rotation").and_then(V::num).unwrap_or(0.0);
    n.w = v.get("w").and_then(V::num).unwrap_or(0.0);
    n.h = v.get("h").and_then(V::num).unwrap_or(0.0);
    n.opacity = v.get("opacity").and_then(V::num).unwrap_or(1.0) as f32;
    n.visible = v.get("visible").and_then(V::boolean).unwrap_or(true);
    n.locked = v.get("locked").and_then(V::boolean).unwrap_or(false);
    n.is_mask = v.get("mask").and_then(V::boolean).unwrap_or(false);
    n.fill = v.get("fill").map(parse_paint).unwrap_or(Paint::Solid(Color::TRANSPARENT));
    if let Some(s) = v.get("stroke") {
        // "paint" (gradients) or legacy "color" (solid)
        let paint = s.get("paint").map(parse_paint)
            .or_else(|| s.get("color").and_then(V::str).and_then(parse_hex_color).map(Paint::Solid))
            .unwrap_or(Paint::Solid(Color::BLACK));
        n.stroke = Stroke { paint, width: s.get("width").and_then(V::num).unwrap_or(0.0) };
    }
    if let Some(layers) = v.get("fill_layers").and_then(V::arr) {
        n.visual_stacks_materialized = true;
        n.fill_layers = layers.iter().filter_map(|l| Some(PaintLayer {
            paint: parse_paint(l.get("paint")?),
            opacity: l.get("opacity").and_then(V::num).unwrap_or(1.0) as f32,
            visible: l.get("visible").and_then(V::boolean).unwrap_or(true),
            blend: parse_blend(l.get("blend").and_then(V::str)),
        })).collect();
    }
    if let Some(layers) = v.get("stroke_layers").and_then(V::arr) {
        n.visual_stacks_materialized = true;
        n.stroke_layers = layers.iter().map(|l| StrokeLayer {
            stroke: Stroke {
                paint: l.get("paint").map(parse_paint)
                    .or_else(|| l.get("color").and_then(V::str).and_then(parse_hex_color).map(Paint::Solid))
                    .unwrap_or(Paint::Solid(Color::BLACK)),
                width: l.get("width").and_then(V::num).unwrap_or(0.0),
            },
            opacity: l.get("opacity").and_then(V::num).unwrap_or(1.0) as f32,
            visible: l.get("visible").and_then(V::boolean).unwrap_or(true),
            blend: parse_blend(l.get("blend").and_then(V::str)),
            options: StrokeOptions {
                align: match l.get("align").and_then(V::str) { Some("inside") => StrokeAlign::Inside, Some("outside") => StrokeAlign::Outside, _ => StrokeAlign::Center },
                cap_start: parse_cap(l.get("cap_start").and_then(V::str)),
                cap_end: parse_cap(l.get("cap_end").and_then(V::str)),
                join: match l.get("join").and_then(V::str) { Some("bevel") => StrokeJoin::Bevel, Some("round") => StrokeJoin::Round, _ => StrokeJoin::Miter },
                dash: l.get("dash").and_then(V::arr).map(|a| a.iter().filter_map(V::num).collect()).unwrap_or_default(),
                dash_offset: l.get("dash_offset").and_then(V::num).unwrap_or(0.0),
                miter_limit: l.get("miter").and_then(V::num).unwrap_or(4.0),
            },
        }).collect();
    }
    if let Some(layers) = v.get("effect_layers").and_then(V::arr) {
        n.visual_stacks_materialized = true;
        n.effect_layers = layers.iter().filter_map(|l| Some(EffectLayer {
            effect: parse_effect(l.get("effect")?)?,
            opacity: l.get("opacity").and_then(V::num).unwrap_or(1.0) as f32,
            visible: l.get("visible").and_then(V::boolean).unwrap_or(true),
            blend: parse_blend(l.get("blend").and_then(V::str)),
        })).collect();
    }
    if let Some(p) = v.get("pin").and_then(V::str) {
        let mut it = p.split_whitespace();
        n.pin = (
            match it.next() { Some("right") => HPin::Right, Some("center") => HPin::CenterH, Some("stretch") => HPin::StretchH, Some("scale") => HPin::ScaleH, _ => HPin::Left },
            match it.next() { Some("bottom") => VPin::Bottom, Some("center") => VPin::CenterV, Some("stretch") => VPin::StretchV, Some("scale") => VPin::ScaleV, _ => VPin::Top },
        );
    }
    if let Some(c) = v.get("corners").and_then(V::arr) {
        if c.len() == 4 { n.corner_radii = Some([c[0].num().unwrap_or(0.0), c[1].num().unwrap_or(0.0), c[2].num().unwrap_or(0.0), c[3].num().unwrap_or(0.0)]); }
    }
    if let Some(rs) = v.get("textRuns").and_then(V::arr) {
        for r in rs {
            n.text_runs.push(TextRun {
                start: r.get("start").and_then(V::num).unwrap_or(0.0) as usize,
                len: r.get("len").and_then(V::num).unwrap_or(0.0) as usize,
                color: r.get("color").and_then(V::str).and_then(parse_hex_color),
                size: r.get("size").and_then(V::num),
                font: r.get("font").and_then(V::str).map(Into::into),
            });
        }
    }
    n.blend = parse_blend(v.get("blend").and_then(V::str));
    if let Some(fx) = v.get("effects").and_then(V::arr) {
        for e in fx {
            if let Some(effect) = parse_effect(e) { n.effects.push(effect); }
        }
    }
    if let Some(p) = v.get("prototype") {
        n.prototype = Some(PrototypeAction {
            destination: p.get("to").and_then(V::str).unwrap_or("").into(),
            transition_ms: p.get("ms").and_then(V::num).unwrap_or(0.0) as u32,
        });
    }
    if let Some(V::Obj(m)) = v.get("bindings") {
        for (k, val) in m { if let V::Str(s) = val { n.bindings.insert(k.clone(), s.clone()); } }
    }
    if let Some(V::Obj(m)) = v.get("overrides") {
        for (k, val) in m { if let V::Str(s) = val { n.overrides.insert(k.clone(), s.clone()); } }
    }
    if let Some(kids) = v.get("children").and_then(V::arr) {
        n.children = kids.iter().map(parse_node).collect();
    }
    n.dirty = false;
    n
}

/// Load a `.x` v1 document. Unknown fields are ignored (forward-compatible).
pub fn load_x(text: &str) -> Result<Document, String> {
    let v = P::new(text).value()?;
    if v.get("format").and_then(V::str) != Some("x-native") { return Err("not an x-native file".into()); }
    let version = v.get("version").and_then(V::num).unwrap_or(0.0) as u32;
    if version > X_FORMAT_VERSION { return Err(format!("file version {version} is newer than supported {X_FORMAT_VERSION}")); }
    let mut doc = Document::new();
    if let Some(vars) = v.get("variables") {
        if let Some(V::Obj(m)) = vars.get("colors") {
            for (k, val) in m { if let Some(c) = val.str().and_then(parse_hex_color) { doc.variables.colors.insert(k.clone(), c); } }
        }
        if let Some(V::Obj(m)) = vars.get("numbers") {
            for (k, val) in m { if let Some(n) = val.num() { doc.variables.numbers.insert(k.clone(), n); } }
        }
        if let Some(V::Obj(m)) = vars.get("strings") {
            for (k, val) in m { if let V::Str(s) = val { doc.variables.strings.insert(k.clone(), s.clone()); } }
        }
        if let Some(V::Obj(m)) = vars.get("bools") {
            for (k, val) in m { if let Some(b) = val.boolean() { doc.variables.bools.insert(k.clone(), b); } }
        }
        if let Some(V::Obj(m)) = vars.get("collections") {
            for (k, val) in m { if let V::Str(s) = val { doc.variables.collections.insert(k.clone(), s.clone()); } }
        }
        if let Some(V::Obj(m)) = vars.get("modes") {
            for (mode, table) in m {
                if let V::Obj(entries) = table {
                    let mut t = std::collections::HashMap::new();
                    for (k, val) in entries { if let Some(c) = val.str().and_then(parse_hex_color) { t.insert(k.clone(), c); } }
                    doc.variables.modes.insert(mode.clone(), t);
                }
            }
        }
    }
    if let Some(V::Obj(styles)) = v.get("styles") {
        for (name, sv) in styles {
            if let Some(s) = parse_style_v(sv) { doc.styles.insert(name.clone(), s); }
        }
    }
    if let Some(assets) = v.get("assets").and_then(V::arr) {
        for a in assets {
            let (Some(data), Some(name)) = (a.get("data").and_then(V::str), a.get("name").and_then(V::str)) else { continue };
            if let Some(bytes) = crate::b64::debase64(data) {
                // re-register: id is re-derived from content, so a
                // tampered/corrupt record can't hijack another asset's id
                doc.assets.register(name, bytes, AssetSource::Embedded);
            }
        }
    }
    if let Some(libs) = v.get("libraries").and_then(V::arr) {
        for d in libs {
            let (Some(id), Some(ver)) = (d.get("library_id").and_then(V::str), d.get("resolved_version").and_then(V::num)) else { continue };
            doc.library_deps.push(LibraryDependency {
                library_id: id.to_string(),
                resolved_version: ver as u32,
                snapshot_hash: d.get("snapshot_hash").and_then(V::str).unwrap_or("").to_string(),
                source_path: d.get("source_path").and_then(V::str).unwrap_or("").to_string(),
            });
            // snapshot is a nested xlib object: re-serialize the V back to
            // text is wasteful; parse it directly through load_xlib by
            // reconstructing minimal JSON — instead, snapshot objects are
            // parsed inline here via the same field readers.
            if let Some(snap) = d.get("snapshot") {
                if snap.get("format").and_then(V::str) == Some("x-native-library") {
                    if let Some(l) = crate::xlib::parse_library_v(snap) {
                        doc.library_snapshots.insert(id.to_string(), l);
                    }
                }
            }
        }
    }
    if let Some(pages) = v.get("pages").and_then(V::arr) {
        doc.pages = pages.iter().map(parse_node).collect();
    }
    Ok(doc)
}

pub fn save_x_file(doc: &Document, path: &str) -> std::io::Result<()> { std::fs::write(path, save_x(doc)) }
pub fn load_x_file(path: &str) -> Result<Document, String> {
    load_x(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
}


/// Shared style decoder (.x documents AND .xlib libraries).
pub(crate) fn parse_style_v(sv: &V) -> Option<Style> {
    match sv.get("t").and_then(V::str) {
        Some("paint") => sv.get("fill").map(|f| Style::Paint { fill: parse_paint(f) }),
        Some("text") => Some(Style::Text {
            font: sv.get("font").and_then(V::str).unwrap_or("").into(),
            size: sv.get("size").and_then(V::num).unwrap_or(0.0),
            letter_spacing: sv.get("ls").and_then(V::num).unwrap_or(0.0),
            line_height: sv.get("lh").and_then(V::num).unwrap_or(0.0),
        }),
        Some("effect") => {
            let mut effects = Vec::new();
            if let Some(fx) = sv.get("effects").and_then(V::arr) {
                for e in fx {
                    let c = e.get("c").and_then(V::str).and_then(parse_hex_color).unwrap_or(Color::BLACK);
                    let (dx, dy, blur) = (e.get("dx").and_then(V::num).unwrap_or(0.0), e.get("dy").and_then(V::num).unwrap_or(0.0), e.get("blur").and_then(V::num).unwrap_or(0.0));
                    match e.get("t").and_then(V::str) {
                        Some("drop") => effects.push(Effect::DropShadow { dx, dy, blur, color: c }),
                        Some("inner") => effects.push(Effect::InnerShadow { dx, dy, blur, color: c }),
                        Some("blur") => effects.push(Effect::LayerBlur { radius: e.get("r").and_then(V::num).unwrap_or(0.0) }),
                        Some("bgblur") => effects.push(Effect::BackgroundBlur { radius: e.get("r").and_then(V::num).unwrap_or(0.0) }),
                        _ => {}
                    }
                }
            }
            Some(Style::Effect { effects })
        }
        _ => None,
    }
}

/// str -> Color for library variable tables (shared helper).
pub(crate) fn parse_hex_color_v(s: &str) -> Option<Color> { parse_hex_color(s) }
