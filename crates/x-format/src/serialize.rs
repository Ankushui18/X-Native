use x_core::*;
#[allow(unused_imports)]
use crate::*;

// ------------------------------------------------------------------ emitter

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn paint_json(p: &Paint) -> String {
    match p {
        Paint::Solid(c) => format!("{{\"t\":\"solid\",\"c\":\"{}\"}}", color_to_hex(*c)),
        Paint::Variable(n) => format!("{{\"t\":\"var\",\"name\":\"{}\"}}", esc(n)),
        Paint::LinearGradient { start, end, stops } => format!(
            "{{\"t\":\"linear\",\"x0\":{},\"y0\":{},\"x1\":{},\"y1\":{},\"stops\":[{}]}}",
            start.0, start.1, end.0, end.1,
            stops.iter().map(|(t, c)| format!("[{},\"{}\"]", t, color_to_hex(*c))).collect::<Vec<_>>().join(",")
        ),
        Paint::RadialGradient { center, radius, stops } => format!(
            "{{\"t\":\"radial\",\"cx\":{},\"cy\":{},\"r\":{},\"stops\":[{}]}}",
            center.0, center.1, radius,
            stops.iter().map(|(t, c)| format!("[{},\"{}\"]", t, color_to_hex(*c))).collect::<Vec<_>>().join(",")
        ),
    }
}

fn blend_name(b: BlendKind) -> &'static str {
    match b { BlendKind::Normal => "normal", BlendKind::Darken => "darken", BlendKind::Multiply => "multiply", BlendKind::ColorBurn => "color-burn", BlendKind::Lighten => "lighten", BlendKind::Screen => "screen", BlendKind::ColorDodge => "color-dodge", BlendKind::Overlay => "overlay", BlendKind::SoftLight => "soft-light", BlendKind::HardLight => "hard-light", BlendKind::Difference => "difference", BlendKind::Exclusion => "exclusion", BlendKind::Hue => "hue", BlendKind::Saturation => "saturation", BlendKind::Color => "color", BlendKind::Luminosity => "luminosity" }
}
fn cap_name(c: StrokeCap) -> &'static str { match c { StrokeCap::None => "none", StrokeCap::Round => "round", StrokeCap::Square => "square", StrokeCap::Arrow => "arrow", StrokeCap::Triangle => "triangle" } }

fn effect_json(e: &Effect) -> String {
    match e {
        Effect::DropShadow { dx, dy, blur, color } => format!("{{\"t\":\"drop\",\"dx\":{dx},\"dy\":{dy},\"blur\":{blur},\"c\":\"{}\"}}", color_to_hex(*color)),
        Effect::InnerShadow { dx, dy, blur, color } => format!("{{\"t\":\"inner\",\"dx\":{dx},\"dy\":{dy},\"blur\":{blur},\"c\":\"{}\"}}", color_to_hex(*color)),
        Effect::LayerBlur { radius } => format!("{{\"t\":\"blur\",\"r\":{radius}}}"),
        Effect::BackgroundBlur { radius } => format!("{{\"t\":\"bgblur\",\"r\":{radius}}}"),
    }
}

fn kind_json(k: &NodeKind) -> String {
    match k {
        NodeKind::Frame { layout: None } => "{\"t\":\"frame\"}".into(),
        NodeKind::Frame { layout: Some(l) } => format!(
            "{{\"t\":\"frame\",\"layout\":{{\"dir\":\"{}\",\"gap\":{},\"padding\":{},\"sizing\":\"{}\",\"align\":\"{}\",\"space_between\":{}{}{}}}}}",
            if l.direction == LayoutDirection::Horizontal { "h" } else { "v" },
            l.gap, l.padding,
            if l.sizing == Sizing::Hug { "hug" } else { "fixed" },
            match l.align { CrossAlign::Start => "start", CrossAlign::Center => "center", CrossAlign::End => "end" },
            l.space_between,
            l.gap_var.as_deref().map(|v| format!(",\"gap_var\":\"{}\"", esc(v))).unwrap_or_default(),
            l.padding_var.as_deref().map(|v| format!(",\"padding_var\":\"{}\"", esc(v))).unwrap_or_default(),
        ),
        NodeKind::Group => "{\"t\":\"group\"}".into(),
        NodeKind::Rect { radius } => format!("{{\"t\":\"rect\",\"radius\":{radius}}}"),
        NodeKind::Ellipse => "{\"t\":\"ellipse\"}".into(),
        NodeKind::Line => "{\"t\":\"line\"}".into(),
        NodeKind::Text { text } => format!("{{\"t\":\"text\",\"text\":\"{}\"}}", esc(text)),
        NodeKind::Image { asset, fit, placement } => {
            let mut s = format!("{{\"t\":\"image\",\"asset\":\"{}\",\"fit\":\"{}\"", esc(asset), match fit { ImageFit::Fill => "fill", ImageFit::Fit => "fit", ImageFit::Crop => "crop", ImageFit::Tile => "tile" });
            if !placement.is_default() {
                s.push_str(&format!(",\"fx\":{},\"fy\":{},\"scale\":{},\"fliph\":{},\"flipv\":{}",
                    placement.focal.0, placement.focal.1, placement.scale, placement.flip_h, placement.flip_v));
            }
            s.push('}');
            s
        }
        NodeKind::Vector { path } => {
            let cmds: Vec<String> = path.iter().map(|c| match c {
                PathCmd::MoveTo(x, y) => format!("[\"M\",{x},{y}]"),
                PathCmd::LineTo(x, y) => format!("[\"L\",{x},{y}]"),
                PathCmd::CurveTo(x1, y1, x2, y2, x, y) => format!("[\"C\",{x1},{y1},{x2},{y2},{x},{y}]"),
                PathCmd::Close => "[\"Z\"]".into(),
            }).collect();
            format!("{{\"t\":\"vector\",\"path\":[{}]}}", cmds.join(","))
        }
        NodeKind::Component { name } => format!("{{\"t\":\"component\",\"name\":\"{}\"}}", esc(name)),
        NodeKind::Instance { component } => format!("{{\"t\":\"instance\",\"component\":\"{}\"}}", esc(component)),
        NodeKind::VectorNetwork(_) => "{\"t\":\"vector_network\"}".into(),
    }
}

pub(crate) fn node_json(n: &Node, out: &mut String) {
    out.push_str(&format!(
        "{{\"id\":\"{}\",\"kind\":{},\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"rotation\":{},\"opacity\":{},\"visible\":{},\"locked\":{},\"fill\":{}",
        esc(&n.id), kind_json(&n.kind),
        n.transform.x, n.transform.y, n.w, n.h, n.transform.rotation, n.opacity, n.visible, n.locked,
        paint_json(&n.fill),
    ));
    if n.stroke.width > 0.0 {
        out.push_str(&format!(",\"stroke\":{{\"color\":\"{}\",\"width\":{}}}", color_to_hex(n.stroke.color), n.stroke.width));
    }
    if n.visual_stacks_materialized {
        let layers = n.fill_layers.iter().map(|l| format!(
            "{{\"paint\":{},\"opacity\":{},\"visible\":{},\"blend\":\"{}\"}}",
            paint_json(&l.paint), l.opacity, l.visible, blend_name(l.blend)
        )).collect::<Vec<_>>().join(",");
        out.push_str(&format!(",\"fill_layers\":[{layers}]"));
    }
    if n.visual_stacks_materialized {
        let layers = n.stroke_layers.iter().map(|l| format!(
            "{{\"color\":\"{}\",\"width\":{},\"opacity\":{},\"visible\":{},\"blend\":\"{}\",\"align\":\"{}\",\"cap_start\":\"{}\",\"cap_end\":\"{}\",\"join\":\"{}\",\"dash\":[{}],\"dash_offset\":{},\"miter\":{}}}",
            color_to_hex(l.stroke.color), l.stroke.width, l.opacity, l.visible, blend_name(l.blend),
            match l.options.align { StrokeAlign::Inside => "inside", StrokeAlign::Center => "center", StrokeAlign::Outside => "outside" },
            cap_name(l.options.cap_start), cap_name(l.options.cap_end),
            match l.options.join { StrokeJoin::Miter => "miter", StrokeJoin::Bevel => "bevel", StrokeJoin::Round => "round" },
            l.options.dash.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","), l.options.dash_offset, l.options.miter_limit
        )).collect::<Vec<_>>().join(",");
        out.push_str(&format!(",\"stroke_layers\":[{layers}]"));
    }
    if n.visual_stacks_materialized {
        let layers = n.effect_layers.iter().map(|l| format!(
            "{{\"effect\":{},\"opacity\":{},\"visible\":{},\"blend\":\"{}\"}}", effect_json(&l.effect), l.opacity, l.visible, blend_name(l.blend)
        )).collect::<Vec<_>>().join(",");
        out.push_str(&format!(",\"effect_layers\":[{layers}]"));
    }
    if let Some([tl, tr, br, bl]) = n.corner_radii {
        out.push_str(&format!(",\"corners\":[{tl},{tr},{br},{bl}]"));
    }
    if n.blend != BlendKind::Normal {
        let b = blend_name(n.blend);
        out.push_str(&format!(",\"blend\":\"{b}\""));
    }
    if !n.effects.is_empty() {
        let fx: Vec<String> = n.effects.iter().map(effect_json).collect();
        out.push_str(&format!(",\"effects\":[{}]", fx.join(",")));
    }
    if let Some(p) = &n.prototype {
        out.push_str(&format!(",\"prototype\":{{\"to\":\"{}\",\"ms\":{}}}", esc(&p.destination), p.transition_ms));
    }
    if n.is_mask { out.push_str(",\"mask\":true"); }
    if !n.bindings.is_empty() {
        let mut keys: Vec<_> = n.bindings.keys().collect();
        keys.sort();
        let kv: Vec<String> = keys.iter().map(|k| format!("\"{}\":\"{}\"", esc(k), esc(&n.bindings[*k]))).collect();
        out.push_str(&format!(",\"bindings\":{{{}}}", kv.join(",")));
    }
    if !n.overrides.is_empty() {
        let mut keys: Vec<_> = n.overrides.keys().collect();
        keys.sort();
        let kv: Vec<String> = keys.iter().map(|k| format!("\"{}\":\"{}\"", esc(k), esc(&n.overrides[*k]))).collect();
        out.push_str(&format!(",\"overrides\":{{{}}}", kv.join(",")));
    }
    if !n.children.is_empty() {
        out.push_str(",\"children\":[");
        for (i, c) in n.children.iter().enumerate() {
            if i > 0 { out.push(','); }
            node_json(c, out);
        }
        out.push(']');
    }
    out.push('}');
}

/// Serialize a Document to `.x` v1 JSON.
pub fn save_x(doc: &Document) -> String {
    let mut out = format!("{{\"format\":\"x-native\",\"version\":{X_FORMAT_VERSION},");
    // variables
    let mut colors: Vec<_> = doc.variables.colors.iter().collect();
    colors.sort_by_key(|(k, _)| k.clone());
    let mut numbers: Vec<_> = doc.variables.numbers.iter().collect();
    numbers.sort_by_key(|(k, _)| k.clone());
    out.push_str("\"variables\":{\"colors\":{");
    out.push_str(&colors.iter().map(|(k, v)| format!("\"{}\":\"{}\"", esc(k), color_to_hex(**v))).collect::<Vec<_>>().join(","));
    out.push_str("},\"numbers\":{");
    out.push_str(&numbers.iter().map(|(k, v)| format!("\"{}\":{}", esc(k), v)).collect::<Vec<_>>().join(","));
    // strings, bools, collections, modes (P1 additions)
    let mut strs: Vec<_> = doc.variables.strings.iter().collect(); strs.sort_by_key(|(k, _)| k.clone());
    out.push_str("},\"strings\":{");
    out.push_str(&strs.iter().map(|(k, v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect::<Vec<_>>().join(","));
    let mut bls: Vec<_> = doc.variables.bools.iter().collect(); bls.sort_by_key(|(k, _)| k.clone());
    out.push_str("},\"bools\":{");
    out.push_str(&bls.iter().map(|(k, v)| format!("\"{}\":{}", esc(k), v)).collect::<Vec<_>>().join(","));
    let mut cols: Vec<_> = doc.variables.collections.iter().collect(); cols.sort_by_key(|(k, _)| k.clone());
    out.push_str("},\"collections\":{");
    out.push_str(&cols.iter().map(|(k, v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect::<Vec<_>>().join(","));
    let mut mds: Vec<_> = doc.variables.modes.iter().collect(); mds.sort_by_key(|(k, _)| k.clone());
    out.push_str("},\"modes\":{");
    let mode_strs: Vec<String> = mds.iter().map(|(mode, table)| {
        let mut entries: Vec<_> = table.iter().collect(); entries.sort_by_key(|(k, _)| k.clone());
        let inner: Vec<String> = entries.iter().map(|(k, c)| format!("\"{}\":\"{}\"", esc(k), color_to_hex(**c))).collect();
        format!("\"{}\":{{{}}}", esc(mode), inner.join(","))
    }).collect();
    out.push_str(&mode_strs.join(","));
    out.push_str("}},");
    // named styles (Figma paint/text/effect styles). Sorted for determinism.
    let mut style_keys: Vec<_> = doc.styles.keys().collect();
    style_keys.sort();
    out.push_str("\"styles\":{");
    let style_strs: Vec<String> = style_keys.iter().map(|k| {
        format!("\"{}\":{}", esc(k), style_json(&doc.styles[k.as_str()]))
    }).collect();
    out.push_str(&style_strs.join(","));
    // content-addressed EMBEDDED assets (asset:// store) — external
    // references stay machine-local and are not serialized
    out.push_str("},\"assets\":[");
    let asset_strs: Vec<String> = doc.assets.embedded_sorted().iter().map(|r| format!(
        "{{\"id\":\"{}\",\"mime\":\"{}\",\"name\":\"{}\"{},\"data\":\"{}\"}}",
        esc(&r.id), esc(&r.mime), esc(&r.name),
        r.dimensions.map(|(w, h)| format!(",\"w\":{w},\"h\":{h}")).unwrap_or_default(),
        crate::b64::base64(&r.bytes),
    )).collect();
    out.push_str(&asset_strs.join(","));
    // pinned library dependencies + their version snapshots (documents
    // stay self-contained: render identically without the .xlib on disk)
    out.push_str("],\"libraries\":[");
    let mut deps = doc.library_deps.clone();
    deps.sort_by(|a, b| a.library_id.cmp(&b.library_id));
    let dep_strs: Vec<String> = deps.iter().map(|d| {
        let snap = doc.library_snapshots.get(&d.library_id)
            .map(|l| crate::xlib::save_xlib(l))
            .unwrap_or_else(|| "null".into());
        format!("{{\"library_id\":\"{}\",\"resolved_version\":{},\"snapshot_hash\":\"{}\",\"source_path\":\"{}\",\"snapshot\":{}}}",
            esc(&d.library_id), d.resolved_version, esc(&d.snapshot_hash), esc(&d.source_path), snap)
    }).collect();
    out.push_str(&dep_strs.join(","));
    out.push_str("],\"pages\":[");
    for (i, p) in doc.pages.iter().enumerate() {
        if i > 0 { out.push(','); }
        node_json(p, &mut out);
    }
    out.push_str("]}");
    out
}


/// Shared style encoder (used by .x documents AND .xlib libraries).
pub(crate) fn style_json(s: &Style) -> String {
    match s {
        Style::Paint { fill } => format!("{{\"t\":\"paint\",\"fill\":{}}}", paint_json(fill)),
        Style::Text { font, size, letter_spacing, line_height } => format!(
            "{{\"t\":\"text\",\"font\":\"{}\",\"size\":{size},\"ls\":{letter_spacing},\"lh\":{line_height}}}", esc(font)),
        Style::Effect { effects } => {
            let fx: Vec<String> = effects.iter().map(|e| match e {
                Effect::DropShadow { dx, dy, blur, color } => format!("{{\"t\":\"drop\",\"dx\":{dx},\"dy\":{dy},\"blur\":{blur},\"c\":\"{}\"}}", color_to_hex(*color)),
                Effect::InnerShadow { dx, dy, blur, color } => format!("{{\"t\":\"inner\",\"dx\":{dx},\"dy\":{dy},\"blur\":{blur},\"c\":\"{}\"}}", color_to_hex(*color)),
                Effect::LayerBlur { radius } => format!("{{\"t\":\"blur\",\"r\":{radius}}}"),
                Effect::BackgroundBlur { radius } => format!("{{\"t\":\"bgblur\",\"r\":{radius}}}"),
            }).collect();
            format!("{{\"t\":\"effect\",\"effects\":[{}]}}", fx.join(","))
        }
    }
}
