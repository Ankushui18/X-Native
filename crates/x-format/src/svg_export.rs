use x_core::*;
use crate::b64::base64;
#[allow(unused_imports)]
use crate::*;

// --------------------------------------------------------------- SVG export

/// Phase 7.6: export a node tree as standalone SVG. Rect/ellipse/line/text
/// (as vector strokes), gradients, opacity, rotation all map 1:1.
/// Asset resolver for image nodes: name -> raw PNG file bytes.
pub type SvgAssetResolver<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;

/// Text outliner for text nodes (TEXT PARITY): given (text, size,
/// max_width, font_binding), returns shaped glyph outlines as SVG path
/// data strings, each pre-positioned in node-local coordinates. Injected
/// by the caller (the arco_native facade wires x-text's shaping pipeline
/// in) because x-format must not depend on x-text — the crate dependency
/// graph is test-enforced.
pub type SvgTextOutliner<'a> = &'a dyn Fn(&str, f64, f64, Option<&str>) -> Option<Vec<String>>;

pub fn export_svg(root: &Node, vars: &Variables) -> String {
    export_svg_full(root, vars, None, None)
}

pub fn export_svg_with_assets(root: &Node, vars: &Variables, assets: Option<SvgAssetResolver>) -> String {
    export_svg_full(root, vars, assets, None)
}

/// SVG export with embedded images (base64 data URIs; fit modes map to
/// preserveAspectRatio) and — when `text_outliner` is given — TEXT
/// PARITY: text nodes emit shaped glyph outline paths identical to the
/// canvas render instead of a `<text font-family="monospace">` guess.
pub fn export_svg_full(root: &Node, vars: &Variables, assets: Option<SvgAssetResolver>, text_outliner: Option<SvgTextOutliner>) -> String {
    let mut defs = String::new();
    let mut body = String::new();
    let mut grad_id = 0usize;
    // component registry: instances resolve to their master's children
    let mut registry: std::collections::HashMap<String, &Node> = Default::default();
    fn collect<'a>(n: &'a Node, reg: &mut std::collections::HashMap<String, &'a Node>) {
        if let NodeKind::Component { name } = &n.kind { reg.insert(name.clone(), n); }
        for c in &n.children { collect(c, reg); }
    }
    collect(root, &mut registry);
    svg_node(root, vars, &mut body, &mut defs, &mut grad_id, &registry, assets, text_outliner);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n<defs>\n{}</defs>\n{}</svg>\n",
        root.w, root.h, root.w, root.h, defs, body
    )
}

fn svg_fill(p: &Paint, vars: &Variables, defs: &mut String, grad_id: &mut usize) -> String {
    match p {
        Paint::Solid(c) => if c.a == 0 { "none".into() } else { color_to_hex(*c) },
        Paint::Variable(n) => color_to_hex(vars.color(n, Color::BLACK)),
        Paint::LinearGradient { start, end, stops } => {
            *grad_id += 1;
            let id = format!("g{grad_id}");
            defs.push_str(&format!("<linearGradient id=\"{id}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" gradientUnits=\"userSpaceOnUse\">", start.0, start.1, end.0, end.1));
            for (t, c) in stops { defs.push_str(&format!("<stop offset=\"{}\" stop-color=\"{}\"/>", t, color_to_hex(*c))); }
            defs.push_str("</linearGradient>\n");
            format!("url(#{id})")
        }
        Paint::RadialGradient { center, radius, stops } => {
            *grad_id += 1;
            let id = format!("g{grad_id}");
            defs.push_str(&format!("<radialGradient id=\"{id}\" cx=\"{}\" cy=\"{}\" r=\"{}\" gradientUnits=\"userSpaceOnUse\">", center.0, center.1, radius));
            for (t, c) in stops { defs.push_str(&format!("<stop offset=\"{}\" stop-color=\"{}\"/>", t, color_to_hex(*c))); }
            defs.push_str("</radialGradient>\n");
            format!("url(#{id})")
        }
    }
}

fn mask_shape_svg(n: &Node) -> String {
    match &n.kind {
        NodeKind::Rect { radius } => format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"white\"/>", n.transform.x, n.transform.y, n.w, n.h, radius),
        NodeKind::Ellipse => format!("<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"white\"/>", n.transform.x + n.w / 2.0, n.transform.y + n.h / 2.0, n.w / 2.0, n.h / 2.0),
        NodeKind::Vector { path } => {
            let mut d = String::new();
            for c in path {
                match c {
                    PathCmd::MoveTo(x, y) => d.push_str(&format!("M {} {} ", x + n.transform.x, y + n.transform.y)),
                    PathCmd::LineTo(x, y) => d.push_str(&format!("L {} {} ", x + n.transform.x, y + n.transform.y)),
                    PathCmd::CurveTo(x1, y1, x2, y2, x, y) => d.push_str(&format!("C {} {} {} {} {} {} ", x1 + n.transform.x, y1 + n.transform.y, x2 + n.transform.x, y2 + n.transform.y, x + n.transform.x, y + n.transform.y)),
                    PathCmd::Close => d.push_str("Z "),
                }
            }
            format!("<path d=\"{}\" fill=\"white\"/>", d.trim_end())
        }
        _ => String::new(),
    }
}

fn svg_stroke_options(layer: &StrokeLayer) -> String {
    let cap = match layer.options.cap_start { StrokeCap::Round => "round", StrokeCap::Square => "square", _ => "butt" };
    let join = match layer.options.join { StrokeJoin::Round => "round", StrokeJoin::Bevel => "bevel", StrokeJoin::Miter => "miter" };
    let dash = if layer.options.dash.is_empty() { String::new() } else { format!(" stroke-dasharray=\"{}\" stroke-dashoffset=\"{}\"", layer.options.dash.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "), layer.options.dash_offset) };
    format!(" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\" stroke-miterlimit=\"{}\"{dash}", layer.options.miter_limit)
}
fn svg_blend(blend: BlendKind) -> &'static str {
    match blend { BlendKind::Normal => "", BlendKind::Darken => " style=\"mix-blend-mode:darken\"", BlendKind::Multiply => " style=\"mix-blend-mode:multiply\"", BlendKind::ColorBurn => " style=\"mix-blend-mode:color-burn\"", BlendKind::Lighten => " style=\"mix-blend-mode:lighten\"", BlendKind::Screen => " style=\"mix-blend-mode:screen\"", BlendKind::ColorDodge => " style=\"mix-blend-mode:color-dodge\"", BlendKind::Overlay => " style=\"mix-blend-mode:overlay\"", BlendKind::SoftLight => " style=\"mix-blend-mode:soft-light\"", BlendKind::HardLight => " style=\"mix-blend-mode:hard-light\"", BlendKind::Difference => " style=\"mix-blend-mode:difference\"", BlendKind::Exclusion => " style=\"mix-blend-mode:exclusion\"", BlendKind::Hue => " style=\"mix-blend-mode:hue\"", BlendKind::Saturation => " style=\"mix-blend-mode:saturation\"", BlendKind::Color => " style=\"mix-blend-mode:color\"", BlendKind::Luminosity => " style=\"mix-blend-mode:luminosity\"" }
}

#[allow(clippy::too_many_arguments)]
fn svg_node(n: &Node, vars: &Variables, body: &mut String, defs: &mut String, grad_id: &mut usize, registry: &std::collections::HashMap<String, &Node>, assets: Option<SvgAssetResolver>, text_outliner: Option<SvgTextOutliner>) {
    if !n.visible { return; }
    let mut tf = format!("translate({} {})", n.transform.x, n.transform.y);
    if n.transform.rotation != 0.0 {
        tf.push_str(&format!(" rotate({} {} {})", n.transform.rotation.to_degrees(), n.w / 2.0, n.h / 2.0));
    }
    let op = if n.opacity < 1.0 { format!(" opacity=\"{}\"", n.opacity) } else { String::new() };
    body.push_str(&format!("<g transform=\"{tf}\"{op}>"));
    match &n.kind {
        NodeKind::Rect { radius } => {
            let r = n.corner_radii.map(|c| c[0]).unwrap_or(*radius);
            for layer in n.active_fills() {
                let fill = svg_fill(&layer.paint, vars, defs, grad_id);
                body.push_str(&format!("<rect width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" opacity=\"{}\"{}/>", n.w, n.h, r, fill, layer.opacity, svg_blend(layer.blend)));
            }
            for layer in n.active_strokes() {
                body.push_str(&format!("<rect width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" opacity=\"{}\"{}{}/>", n.w, n.h, r, color_to_hex(layer.stroke.color), layer.stroke.width, layer.opacity, svg_blend(layer.blend), svg_stroke_options(&layer)));
            }
        }
        NodeKind::Ellipse => {
            for layer in n.active_fills() {
                let fill = svg_fill(&layer.paint, vars, defs, grad_id);
                body.push_str(&format!("<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" opacity=\"{}\"{}/>", n.w / 2.0, n.h / 2.0, n.w / 2.0, n.h / 2.0, fill, layer.opacity, svg_blend(layer.blend)));
            }
            for layer in n.active_strokes() {
                body.push_str(&format!("<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" opacity=\"{}\"{}{}/>", n.w / 2.0, n.h / 2.0, n.w / 2.0, n.h / 2.0, color_to_hex(layer.stroke.color), layer.stroke.width, layer.opacity, svg_blend(layer.blend), svg_stroke_options(&layer)));
            }
        }
        NodeKind::Line => {
            for layer in n.active_strokes() { body.push_str(&format!("<line x1=\"0\" y1=\"0\" x2=\"{}\" y2=\"0\" stroke=\"{}\" stroke-width=\"{}\" opacity=\"{}\"{}{}/>", n.w, color_to_hex(layer.stroke.color), layer.stroke.width.max(1.0), layer.opacity, svg_blend(layer.blend), svg_stroke_options(&layer))); }
        }
        NodeKind::Text { text } => {
            // TEXT PARITY: shaped glyph outlines when an outliner is
            // injected (identical geometry to the canvas render).
            let outlines = text_outliner.and_then(|outline| outline(text, n.h, n.w, n.bindings.get("font").map(String::as_str)));
            for layer in n.active_fills() {
                let fill = svg_fill(&layer.paint, vars, defs, grad_id);
                if let Some(paths) = &outlines {
                    for d in paths {
                        body.push_str(&format!("<path d=\"{d}\" fill=\"{fill}\" opacity=\"{}\"{}/>", layer.opacity, svg_blend(layer.blend)));
                    }
                } else {
                    body.push_str(&format!("<text y=\"{}\" font-size=\"{}\" font-family=\"monospace\" fill=\"{}\" opacity=\"{}\"{}>{}</text>", n.h * 0.8, n.h * 0.8, fill, layer.opacity, svg_blend(layer.blend), text.replace('&', "&amp;").replace('<', "&lt;")));
                }
            }
        }
        NodeKind::Vector { path } => {
            let mut d = String::new();
            for c in path {
                match c {
                    PathCmd::MoveTo(x, y) => d.push_str(&format!("M {x} {y} ")),
                    PathCmd::LineTo(x, y) => d.push_str(&format!("L {x} {y} ")),
                    PathCmd::CurveTo(x1, y1, x2, y2, x, y) => d.push_str(&format!("C {x1} {y1} {x2} {y2} {x} {y} ")),
                    PathCmd::Close => d.push_str("Z "),
                }
            }
            for layer in n.active_fills() {
                let fill = svg_fill(&layer.paint, vars, defs, grad_id);
                body.push_str(&format!("<path d=\"{}\" fill=\"{}\" opacity=\"{}\"{}/>", d.trim_end(), fill, layer.opacity, svg_blend(layer.blend)));
            }
            for layer in n.active_strokes() { body.push_str(&format!("<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" opacity=\"{}\"{}{}/>", d.trim_end(), color_to_hex(layer.stroke.color), layer.stroke.width, layer.opacity, svg_blend(layer.blend), svg_stroke_options(&layer))); }
        }
        NodeKind::Image { asset, fit, placement } => {
            // CANONICAL image transform model: identical resolution to the
            // canvas sink (x_core::resolve_image_placement) — the SVG gets
            // the exact same matrices, not a preserveAspectRatio guess.
            if let Some((bytes, iw, ih)) = assets.and_then(|r| r(asset))
                .and_then(|b| probe_dimensions(&b).map(|(w0, h0)| (b, w0 as f64, h0 as f64)))
            {
                let resolved = resolve_image_placement(*fit, placement, n.w, n.h, iw, ih);
                let b64 = base64(&bytes);
                *grad_id += 1;
                let clip = format!("imgclip{grad_id}");
                defs.push_str(&format!(
                    "<clipPath id=\"{clip}\"><rect width=\"{}\" height=\"{}\"/></clipPath>\n", n.w, n.h));
                body.push_str(&format!("<g clip-path=\"url(#{clip})\">"));
                for draw in &resolved.draws {
                    let c = draw.as_coeffs();
                    body.push_str(&format!(
                        "<image width=\"{iw}\" height=\"{ih}\" preserveAspectRatio=\"none\" transform=\"matrix({} {} {} {} {} {})\" href=\"data:image/png;base64,{b64}\"/>",
                        c[0], c[1], c[2], c[3], c[4], c[5]));
                }
                body.push_str("</g>");
            } else {
                // no resolver / unknown asset / undecodable header:
                // placeholder matches the canvas fallback
                body.push_str(&format!("<rect width=\"{}\" height=\"{}\" fill=\"#dddddd\"/>", n.w, n.h));
            }
        }
        NodeKind::Instance { component } => {
            // resolve to master content (matches the render IR semantics)
            if let Some(master) = registry.get(component) {
                for c in &master.children { svg_node(c, vars, body, defs, grad_id, registry, assets, text_outliner); }
            }
        }
        _ => {}
    }
    // children with Figma mask semantics: a mask child clips FOLLOWING siblings
    let mut open_masks = 0usize;
    for c in &n.children {
        if c.is_mask && c.visible {
            let shape = mask_shape_svg(c);
            if !shape.is_empty() {
                *grad_id += 1;
                let mid = format!("mask{grad_id}");
                defs.push_str(&format!("<mask id=\"{mid}\">{shape}</mask>\n"));
                body.push_str(&format!("<g mask=\"url(#{mid})\">"));
                open_masks += 1;
            }
            continue; // masks paint nothing themselves
        }
        svg_node(c, vars, body, defs, grad_id, registry, assets, text_outliner);
    }
    for _ in 0..open_masks { body.push_str("</g>"); }
    body.push_str("</g>\n");
}
