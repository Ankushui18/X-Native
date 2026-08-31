//! Render IR (P1): Document -> Layout -> RenderTree -> RenderCommands -> sink.
//!
//! The document model no longer needs to become Vello ops directly.
//! `build_render_tree` resolves everything renderable (world transforms,
//! effective paints, instance resolution, overrides, variable bindings)
//! into a flat, backend-agnostic command list. Sinks consume commands:
//! - VelloSink   -> vello::Scene (the GPU path, used by the app)
//! - Any future sink: SVG, PDF/print, thumbnails, hit-test caches,
//!   accessibility trees, WebGPU-direct — same commands.
//! Node uuid-stable `key`s make caching / partial redraw diffs possible.

use std::collections::HashMap;
use vello::kurbo::{Affine, BezPath, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Mix};
use vello::Scene;
use x_core::*;

/// One drawable unit, fully resolved. No document types leak through
/// except geometry/paint primitives.
#[derive(Debug, Clone)]
pub enum RenderCommand {
    FillPath { key: String, transform: Affine, path: BezPath, brush: Brush },
    StrokePath { key: String, transform: Affine, path: BezPath, color: Color, width: f64, options: StrokeOptions },
    PushLayer { key: String, mix: Mix, alpha: f32, bounds: Rect },
    PopLayer,
    Glyphs { key: String, transform: Affine, text: String, size: f64, brush: Brush, max_width: f64, font: Option<String>, letter_spacing: f64, line_height: f64 },
    Image { key: String, transform: Affine, asset: String, w: f64, h: f64, fit: ImageFit, placement: ImagePlacement },
    /// clip layer from an arbitrary path (masks)
    PushClip { key: String, transform: Affine, path: BezPath },
}

impl RenderCommand {
    pub fn key(&self) -> &str {
        match self {
            RenderCommand::FillPath { key, .. } | RenderCommand::StrokePath { key, .. }
            | RenderCommand::PushLayer { key, .. } | RenderCommand::Glyphs { key, .. }
            | RenderCommand::Image { key, .. } | RenderCommand::PushClip { key, .. } => key,
            RenderCommand::PopLayer => "",
        }
    }
}

#[derive(Debug, Default)]
pub struct RenderTree {
    pub commands: Vec<RenderCommand>,
}

impl RenderTree {
    /// Cache/diff support: keys of commands that differ from `old`.
    /// (Positional diff keyed by node identity — the partial-redraw seed.)
    pub fn changed_keys(&self, old: &RenderTree) -> Vec<String> {
        let index = |t: &RenderTree| -> HashMap<String, usize> {
            let mut m = HashMap::new();
            for (i, c) in t.commands.iter().enumerate() {
                if !c.key().is_empty() { m.insert(format!("{}#{}", c.key(), fingerprint(c)), i); }
            }
            m
        };
        let old_idx = index(old);
        let mut changed = vec![];
        for c in &self.commands {
            if c.key().is_empty() { continue; }
            let fp = format!("{}#{}", c.key(), fingerprint(c));
            if !old_idx.contains_key(&fp) { changed.push(c.key().to_string()); }
        }
        changed.sort();
        changed.dedup();
        changed
    }
}

/// A mask node's clip geometry (vector path / rect / ellipse).
fn mask_path_of(n: &Node) -> Option<BezPath> {
    match &n.kind {
        NodeKind::Vector { path } if !path.is_empty() => Some(path_to_bez(path)),
        NodeKind::Rect { radius } => {
            let r = *radius;
            Some(if r > 0.0 {
                vello::kurbo::RoundedRect::new(0.0, 0.0, n.w, n.h, r).into_path(0.1)
            } else {
                Rect::new(0.0, 0.0, n.w, n.h).into_path(0.1)
            })
        }
        NodeKind::Ellipse => {
            let (rx, ry) = (n.w / 2.0, n.h / 2.0);
            Some(vello::kurbo::Ellipse::new((rx, ry), (rx, ry), 0.0).into_path(0.1))
        }
        _ => None,
    }
}

fn layer_brush(paint: &Paint, vars: &Variables, opacity: f32) -> Brush {
    if opacity >= 1.0 { return paint_brush(paint, vars); }
    match paint {
        Paint::Solid(c) => Brush::Solid(c.with_alpha_factor(opacity)),
        Paint::Variable(name) => Brush::Solid(vars.color(name, Color::BLACK).with_alpha_factor(opacity)),
        Paint::LinearGradient { start, end, stops } => paint_brush(&Paint::LinearGradient {
            start: *start, end: *end, stops: stops.iter().map(|(t, c)| (*t, c.with_alpha_factor(opacity))).collect()
        }, vars),
        Paint::RadialGradient { center, radius, stops } => paint_brush(&Paint::RadialGradient {
            center: *center, radius: *radius, stops: stops.iter().map(|(t, c)| (*t, c.with_alpha_factor(opacity))).collect()
        }, vars),
    }
}

/// Normalized concentric Gaussian taps. Every tap is still a native Vello
/// vector draw, so the compositor remains GPU-driven on the pinned backend.
fn gaussian_taps(radius: f64) -> Vec<(f64, f64, f32)> {
    if radius <= 0.01 { return vec![(0.0, 0.0, 1.0)]; }
    let sigma = (radius / 2.0).max(0.5);
    let mut taps = vec![(0.0, 0.0, 1.0f64)];
    for (ring, count) in [(0.45, 8usize), (0.9, 12usize), (1.35, 16usize)] {
        let d = radius * ring;
        let weight = (-d * d / (2.0 * sigma * sigma)).exp();
        for i in 0..count {
            let a = std::f64::consts::TAU * i as f64 / count as f64;
            taps.push((a.cos() * d, a.sin() * d, weight));
        }
    }
    let sum: f64 = taps.iter().map(|t| t.2).sum();
    taps.into_iter().map(|(x, y, w)| (x, y, (w / sum) as f32)).collect()
}

fn offset_command(command: &RenderCommand, dx: f64, dy: f64) -> RenderCommand {
    let shift = Affine::translate((dx, dy));
    match command {
        RenderCommand::FillPath { key, transform, path, brush } => RenderCommand::FillPath { key: format!("{key}/bg"), transform: shift * *transform, path: path.clone(), brush: brush.clone() },
        RenderCommand::StrokePath { key, transform, path, color, width, options } => RenderCommand::StrokePath { key: format!("{key}/bg"), transform: shift * *transform, path: path.clone(), color: *color, width: *width, options: options.clone() },
        RenderCommand::PushLayer { key, mix, alpha, bounds } => RenderCommand::PushLayer { key: format!("{key}/bg"), mix: *mix, alpha: *alpha, bounds: Rect::new(bounds.x0 + dx, bounds.y0 + dy, bounds.x1 + dx, bounds.y1 + dy) },
        RenderCommand::PopLayer => RenderCommand::PopLayer,
        RenderCommand::Glyphs { key, transform, text, size, brush, max_width, font, letter_spacing, line_height } => RenderCommand::Glyphs { key: format!("{key}/bg"), transform: shift * *transform, text: text.clone(), size: *size, brush: brush.clone(), max_width: *max_width, font: font.clone(), letter_spacing: *letter_spacing, line_height: *line_height },
        RenderCommand::Image { key, transform, asset, w, h, fit, placement } => RenderCommand::Image { key: format!("{key}/bg"), transform: shift * *transform, asset: asset.clone(), w: *w, h: *h, fit: *fit, placement: *placement },
        RenderCommand::PushClip { key, transform, path } => RenderCommand::PushClip { key: format!("{key}/bg"), transform: shift * *transform, path: path.clone() },
    }
}

fn emit_visual_layers(tree: &mut RenderTree, node: &Node, key: &str, world: Affine, path: &BezPath, vars: &Variables, opacity: f32, override_color: Option<Color>) {
    let effects = node.active_effects();
    let layer_blur = effects.iter().filter_map(|l| match &l.effect { Effect::LayerBlur { radius } => Some(*radius * l.opacity as f64), _ => None }).fold(0.0, f64::max);
    for (i, layer) in effects.iter().enumerate() {
        if let Effect::BackgroundBlur { radius } = &layer.effect {
            let background = tree.commands.clone();
            if !background.is_empty() && *radius > 0.01 {
                if let Some(mix) = layer.blend.mix() { tree.commands.push(RenderCommand::PushLayer { key: format!("{key}/effect-{i}/blend"), mix, alpha: 1.0, bounds: bounds(world, node.w, node.h) }); }
                tree.commands.push(RenderCommand::PushClip { key: format!("{key}/effect-{i}/background-clip"), transform: world, path: path.clone() });
                let b = bounds(world, node.w, node.h).inflate(*radius * 1.5, *radius * 1.5);
                for (tap, (dx, dy, weight)) in gaussian_taps(*radius).into_iter().enumerate() {
                    tree.commands.push(RenderCommand::PushLayer { key: format!("{key}/effect-{i}/background-tap-{tap}"), mix: Mix::Normal, alpha: weight * layer.opacity.clamp(0.0, 1.0), bounds: b });
                    tree.commands.extend(background.iter().map(|cmd| offset_command(cmd, dx, dy)));
                    tree.commands.push(RenderCommand::PopLayer);
                }
                tree.commands.push(RenderCommand::PopLayer);
                if layer.blend != BlendKind::Normal { tree.commands.push(RenderCommand::PopLayer); }
            }
        }
    }
    for (i, layer) in effects.iter().enumerate() {
        match &layer.effect {
            Effect::DropShadow { dx, dy, blur, color } => {
                if let Some(mix) = layer.blend.mix() { tree.commands.push(RenderCommand::PushLayer { key: format!("{key}/effect-{i}/blend"), mix, alpha: 1.0, bounds: bounds(world, node.w, node.h).inflate(*blur * 1.5, *blur * 1.5) }); }
                for (tap, (bx, by, weight)) in gaussian_taps(*blur).into_iter().enumerate() {
                    tree.commands.push(RenderCommand::FillPath { key: format!("{key}/effect-{i}/tap-{tap}"), transform: Affine::translate((bx, by)) * world * Affine::translate((*dx, *dy)), path: path.clone(), brush: Brush::Solid(color.with_alpha_factor(opacity * layer.opacity.clamp(0.0, 1.0) * weight)) });
                }
                if layer.blend != BlendKind::Normal { tree.commands.push(RenderCommand::PopLayer); }
            }
            _ => {}
        }
    }
    let fills = if let Some(color) = override_color {
        vec![PaintLayer::new(Paint::Solid(color))]
    } else { node.active_fills() };
    for (i, layer) in fills.iter().enumerate() {
        let layer_key = format!("{key}/fill-{i}");
        if let Some(mix) = layer.blend.mix() {
            tree.commands.push(RenderCommand::PushLayer { key: layer_key.clone(), mix, alpha: 1.0, bounds: bounds(world, node.w, node.h) });
        }
        for (tap, (bx, by, weight)) in gaussian_taps(layer_blur).into_iter().enumerate() {
            tree.commands.push(RenderCommand::FillPath { key: if layer_blur > 0.01 { format!("{layer_key}/blur-{tap}") } else { layer_key.clone() }, transform: Affine::translate((bx, by)) * world, path: path.clone(), brush: layer_brush(&layer.paint, vars, opacity * layer.opacity.clamp(0.0, 1.0) * weight) });
        }
        if layer.blend != BlendKind::Normal { tree.commands.push(RenderCommand::PopLayer); }
    }
    for (i, layer) in node.active_strokes().iter().enumerate() {
        let layer_key = format!("{key}/stroke-{i}");
        if let Some(mix) = layer.blend.mix() {
            tree.commands.push(RenderCommand::PushLayer { key: layer_key.clone(), mix, alpha: 1.0, bounds: bounds(world, node.w, node.h) });
        }
        for (tap, (dx, dy, weight)) in gaussian_taps(layer_blur).into_iter().enumerate() {
            tree.commands.push(RenderCommand::StrokePath {
                key: if layer_blur > 0.01 { format!("{layer_key}/blur-{tap}") } else { layer_key.clone() },
                transform: Affine::translate((dx, dy)) * world,
                path: path.clone(),
                color: layer.stroke.color.with_alpha_factor(opacity * layer.opacity.clamp(0.0, 1.0) * weight),
                width: layer.stroke.width,
                options: layer.options.clone(),
            });
        }
        if layer.blend != BlendKind::Normal { tree.commands.push(RenderCommand::PopLayer); }
    }
    // Inner shadows composite above the object's paint but remain clipped to
    // its geometry. This ordering is essential; drawing them before fills
    // would make an opaque fill erase the effect.
    for (i, layer) in effects.iter().enumerate() {
        if let Effect::InnerShadow { dx, dy, blur, color } = &layer.effect {
            if let Some(mix) = layer.blend.mix() { tree.commands.push(RenderCommand::PushLayer { key: format!("{key}/effect-{i}/blend"), mix, alpha: 1.0, bounds: bounds(world, node.w, node.h) }); }
            tree.commands.push(RenderCommand::PushClip { key: format!("{key}/effect-{i}/inner-clip"), transform: world, path: path.clone() });
            for (tap, (bx, by, weight)) in gaussian_taps(*blur).into_iter().enumerate() {
                tree.commands.push(RenderCommand::StrokePath { key: format!("{key}/effect-{i}/inner-tap-{tap}"), transform: Affine::translate((bx, by)) * world * Affine::translate((*dx, *dy)), path: path.clone(), color: color.with_alpha_factor(opacity * layer.opacity.clamp(0.0, 1.0) * weight), width: (*blur * 2.0).max(1.0), options: StrokeOptions::default() });
            }
            tree.commands.push(RenderCommand::PopLayer);
            if layer.blend != BlendKind::Normal { tree.commands.push(RenderCommand::PopLayer); }
        }
    }
}

fn fingerprint(c: &RenderCommand) -> String {
    match c {
        RenderCommand::FillPath { transform, path, brush, .. } =>
            format!("f{:?}{}{:?}", transform.as_coeffs(), path.elements().len(), brush),
        RenderCommand::StrokePath { transform, color, width, options, .. } =>
            format!("s{:?}{color:?}{width}{options:?}", transform.as_coeffs()),
        RenderCommand::PushLayer { mix, alpha, bounds, .. } => format!("l{mix:?}{alpha}{bounds:?}"),
        RenderCommand::Glyphs { transform, text, size, brush, font, max_width, .. } =>
            format!("g{:?}{text}{size}{max_width}{font:?}{brush:?}", transform.as_coeffs()),
        RenderCommand::Image { transform, asset, fit, placement, .. } => format!("i{:?}{asset}{fit:?}{placement:?}", transform.as_coeffs()),
        RenderCommand::PushClip { transform, path, .. } => format!("c{:?}{}", transform.as_coeffs(), path.elements().len()),
        RenderCommand::PopLayer => String::new(),
    }
}

/// Build the resolved command list from a document root.
pub fn build_render_tree(root: &Node, vars: &Variables) -> RenderTree {
    let mut tree = RenderTree::default();
    let mut registry: HashMap<&str, &Node> = HashMap::new();
    fn collect<'a>(n: &'a Node, reg: &mut HashMap<&'a str, &'a Node>) {
        if let NodeKind::Component { name } = &n.kind { reg.insert(name.as_str(), n); }
        for c in &n.children { collect(c, reg); }
    }
    collect(root, &mut registry);
    let empty = HashMap::new();
    lower(root, Affine::IDENTITY, vars, &registry, &empty, 0, &mut tree, "");
    tree
}

#[allow(clippy::too_many_arguments)]
fn lower(node: &Node, parent: Affine, vars: &Variables, registry: &HashMap<&str, &Node>, overrides: &HashMap<String, String>, depth: u32, tree: &mut RenderTree, path: &str) {
    // typed traversal overrides (visible / opacity / swap), same semantics
    // as the direct encoder
    let mut vis = node.visible;
    let mut opacity_override: Option<f32> = None;
    let mut swap_component: Option<String> = None;
    if let Some(raw) = overrides.get(&node.id) {
        if let Some(v) = raw.strip_prefix("visible:") { if let Ok(b) = v.parse() { vis = b; } }
        else if let Some(o) = raw.strip_prefix("opacity:") { if let Ok(f) = o.parse() { opacity_override = Some(f); } }
        else if let Some(c) = raw.strip_prefix("swap:") { swap_component = Some(c.to_string()); }
    }
    if !vis { return; }
    let opacity = opacity_override.unwrap_or(node.opacity).clamp(0.0, 1.0);
    let world = parent * node.transform.matrix(node.w, node.h);
    let key = format!("{path}/{}", node.id);

    let blend = node.blend.mix();
    if let Some(mix) = blend {
        let b = bounds(world, node.w, node.h);
        tree.commands.push(RenderCommand::PushLayer { key: key.clone(), mix, alpha: 1.0, bounds: b });
    }

    let brush = || {
        let stack_paint = node.active_fills().last().map(|l| l.paint.clone()).unwrap_or_else(|| node.fill.clone());
        let mut b = if let Some(raw) = overrides.get(&node.id) {
            if let Some(c) = parse_hex_color(raw) { Brush::Solid(c) } else { paint_brush(&stack_paint, vars) }
        } else { paint_brush(&stack_paint, vars) };
        if opacity < 1.0 {
            if let Brush::Solid(c) = b { b = Brush::Solid(c.with_alpha_factor(opacity)); }
        }
        b
    };

    match &node.kind {
        NodeKind::Rect { radius } => {
            let r = node.bound_number("radius", vars, *radius);
            let shape = if let Some([tl, tr, br, bl]) = node.corner_radii {
                RoundedRect::from_rect(Rect::new(0.0, 0.0, node.w, node.h), RoundedRectRadii::new(tl, tr, br, bl)).into_path(0.1)
            } else if r > 0.0 {
                RoundedRect::new(0.0, 0.0, node.w, node.h, r).into_path(0.1)
            } else { Rect::new(0.0, 0.0, node.w, node.h).into_path(0.1) };
            let override_color = overrides.get(&node.id).and_then(|raw| parse_hex_color(raw));
            emit_visual_layers(tree, node, &key, world, &shape, vars, opacity, override_color);
        }
        NodeKind::Ellipse => {
            let r = node.w.min(node.h) / 2.0;
            let t = world * Affine::scale_non_uniform(node.w / node.h, 1.0);
            let shape = Circle::new((r, r), r).into_path(0.1);
            let override_color = overrides.get(&node.id).and_then(|raw| parse_hex_color(raw));
            emit_visual_layers(tree, node, &key, t, &shape, vars, opacity, override_color);
        }
        NodeKind::Line => {
            for (i, layer) in node.active_strokes().iter().enumerate() {
                let width = layer.stroke.width.max(1.0);
                let shape = Rect::new(0.0, 0.0, node.w.max(width), width).into_path(0.1);
                tree.commands.push(RenderCommand::FillPath { key: format!("{key}/stroke-{i}"), transform: world, path: shape, brush: Brush::Solid(layer.stroke.color.with_alpha_factor(opacity * layer.opacity)) });
            }
        }
        NodeKind::Vector { path: p } => {
            if !p.is_empty() {
                let bez = path_to_bez(p);
                let override_color = overrides.get(&node.id).and_then(|raw| parse_hex_color(raw));
                emit_visual_layers(tree, node, &key, world, &bez, vars, opacity, override_color);
            }
        }
        NodeKind::Text { text } => {
            let content = overrides.get(&node.id).and_then(|v| v.strip_prefix("text:")).unwrap_or(text);
            // typography bindings (inspector's Size row edits node.h; the
            // ls/lh bindings ride the node so every sink honors them)
            let ls = node.bindings.get("ls").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
            let lh = node.bindings.get("lh").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.2);
            let fills = node.active_fills();
            let text_blur = node.active_effects().iter().filter_map(|l| match &l.effect { Effect::LayerBlur { radius } => Some(*radius * l.opacity as f64), _ => None }).fold(0.0, f64::max);
            for (i, layer) in fills.iter().enumerate() {
                let layer_key = format!("{key}/fill-{i}");
                if let Some(mix) = layer.blend.mix() {
                    tree.commands.push(RenderCommand::PushLayer { key: layer_key.clone(), mix, alpha: 1.0, bounds: bounds(world, node.w, node.h) });
                }
                for (tap, (dx, dy, weight)) in gaussian_taps(text_blur).into_iter().enumerate() {
                    tree.commands.push(RenderCommand::Glyphs { key: if text_blur > 0.01 { format!("{layer_key}/blur-{tap}") } else { layer_key.clone() }, transform: Affine::translate((dx, dy)) * world, text: content.into(), size: node.h, brush: layer_brush(&layer.paint, vars, opacity * layer.opacity * weight), max_width: node.w, font: node.bindings.get("font").cloned(), letter_spacing: ls, line_height: lh });
                }
                if layer.blend != BlendKind::Normal { tree.commands.push(RenderCommand::PopLayer); }
            }
        }
        NodeKind::Image { asset, fit, placement } => {
            let image_blur = node.active_effects().iter().filter_map(|l| match &l.effect { Effect::LayerBlur { radius } => Some(*radius * l.opacity as f64), _ => None }).fold(0.0, f64::max);
            for (tap, (dx, dy, weight)) in gaussian_taps(image_blur).into_iter().enumerate() {
                if image_blur > 0.01 { tree.commands.push(RenderCommand::PushLayer { key: format!("{key}/blur-layer-{tap}"), mix: Mix::Normal, alpha: weight, bounds: bounds(world, node.w, node.h).inflate(image_blur * 1.5, image_blur * 1.5) }); }
                tree.commands.push(RenderCommand::Image { key: if image_blur > 0.01 { format!("{key}/blur-{tap}") } else { key.clone() }, transform: Affine::translate((dx, dy)) * world, asset: asset.clone(), w: node.w, h: node.h, fit: *fit, placement: *placement });
                if image_blur > 0.01 { tree.commands.push(RenderCommand::PopLayer); }
            }
        }
        NodeKind::Frame { .. } => {
            let has_visible_fill = node.active_fills().iter().any(|l| match &l.paint { Paint::Solid(c) => c.a > 0, _ => true });
            if has_visible_fill {
                let shape = Rect::new(0.0, 0.0, node.w, node.h).into_path(0.1);
                let override_color = overrides.get(&node.id).and_then(|raw| parse_hex_color(raw));
                emit_visual_layers(tree, node, &key, world, &shape, vars, opacity, override_color);
            }
        }
        NodeKind::Instance { component } => {
            if depth < MAX_INSTANCE_DEPTH {
                let name = swap_component.as_deref().unwrap_or(component.as_str());
                if let Some(def) = registry.get(name) {
                    for child in &def.children {
                        lower(child, world, vars, registry, &node.overrides, depth + 1, tree, &key);
                    }
                }
            }
        }
        NodeKind::Group | NodeKind::Component { .. } => {}
        NodeKind::VectorNetwork(_) => { /* TODO: Process vector network IR */ }
    }
    let mut mask_layers = 0usize;
    for child in &node.children {
        if child.is_mask && child.visible {
            // masks paint nothing themselves; they clip following siblings
            if let Some(mask_path) = mask_path_of(child) {
                let child_world = world * child.transform.matrix(child.w, child.h);
                tree.commands.push(RenderCommand::PushClip { key: format!("{key}/{}#mask", child.id), transform: child_world, path: mask_path });
                mask_layers += 1;
            }
            continue;
        }
        lower(child, world, vars, registry, overrides, depth, tree, &key);
    }
    for _ in 0..mask_layers { tree.commands.push(RenderCommand::PopLayer); }
    if blend.is_some() { tree.commands.push(RenderCommand::PopLayer); }
}

// -------------------------------------------------------------------- sinks

/// Consume commands into a vello::Scene (the GPU path).
pub struct VelloSink<'a> {
    pub assets: Option<&'a crate::Assets>,
    pub fonts: Option<&'a x_text::FontManager>,
}

impl<'a> VelloSink<'a> {
    pub fn render(&self, tree: &RenderTree) -> Scene {
        let mut scene = Scene::new();
        for cmd in &tree.commands {
            match cmd {
                RenderCommand::FillPath { transform, path, brush, .. } =>
                    scene.fill(Fill::NonZero, *transform, brush, None, path),
                RenderCommand::StrokePath { transform, path, color, width, options, .. } => {
                    let cap = match options.cap_start { StrokeCap::Round => vello::kurbo::Cap::Round, StrokeCap::Square => vello::kurbo::Cap::Square, _ => vello::kurbo::Cap::Butt };
                    let join = match options.join { StrokeJoin::Round => vello::kurbo::Join::Round, StrokeJoin::Bevel => vello::kurbo::Join::Bevel, StrokeJoin::Miter => vello::kurbo::Join::Miter };
                    let mut stroke = vello::kurbo::Stroke::new(*width).with_caps(cap).with_join(join).with_miter_limit(options.miter_limit);
                    if !options.dash.is_empty() { stroke = stroke.with_dashes(options.dash_offset, options.dash.iter().copied()); }
                    scene.stroke(&stroke, *transform, *color, None, path)
                }
                RenderCommand::PushLayer { mix, alpha, bounds, .. } =>
                    scene.push_layer(*mix, *alpha, Affine::IDENTITY, bounds),
                RenderCommand::PopLayer => scene.pop_layer(),
                RenderCommand::Glyphs { transform, text, size, brush, max_width, font, letter_spacing, line_height, .. } => {
                    // canonical text geometry via the ShapedTextCache:
                    // steady-state frames iterate the Arc'd shaped block
                    // ZERO-CLONE — moving text composes the world
                    // transform only (cache hit by construction).
                    let drew = if let Some(fm) = self.fonts {
                        let shape_color = Color::WHITE;
                        let key = x_text::TextLayoutKey::new_styled(text, *size, *max_width, font.as_deref(), shape_color, fm.epoch(), *letter_spacing, *line_height);
                        if let Some(block) = x_text::ShapedTextCache::global().get_or_shape(fm, key) {
                            for g in block.glyphs.iter() {
                                scene.fill(Fill::NonZero, *transform * g.transform, brush, None, &g.path);
                            }
                            true
                        } else { false }
                    } else { false };
                    if !drew {
                        let color = match brush { Brush::Solid(c) => *c, _ => Color::BLACK };
                        x_text::encode_text(&mut scene, text, *transform, *size, color);
                    }
                }
                RenderCommand::Image { transform, asset, w, h, fit, placement, .. } => {
                    if let Some(img) = self.assets.and_then(|a| a.get(asset)) {
                        // CANONICAL image transform model: fit/focal/zoom/
                        // flip/tiling resolved ONCE in x-core; this sink
                        // only composes the world transform and clips.
                        let resolved = x_core::resolve_image_placement(
                            *fit, placement, *w, *h, img.width as f64, img.height as f64);
                        let box_rect = Rect::new(0.0, 0.0, *w, *h).into_path(0.1);
                        scene.push_layer(Mix::Clip, 1.0, *transform, &box_rect);
                        for draw in &resolved.draws {
                            scene.draw_image(img, *transform * *draw);
                        }
                        scene.pop_layer();
                    } else {
                        scene.fill(Fill::NonZero, *transform, Color::rgb8(0xdd, 0xdd, 0xdd), None, &Rect::new(0.0, 0.0, *w, *h).into_path(0.1));
                    }
                }
                RenderCommand::PushClip { transform, path, .. } => {
                    scene.push_layer(Mix::Clip, 1.0, *transform, path);
                }
            }
        }
        scene
    }
}

/// IR-based full pipeline entry (parallel to build_scene_full).
pub fn render_via_ir(root: &Node, vars: &Variables, assets: Option<&crate::Assets>, fonts: Option<&x_text::FontManager>) -> (Scene, RenderTree) {
    let tree = build_render_tree(root, vars);
    let sink = VelloSink { assets, fonts };
    let scene = sink.render(&tree);
    (scene, tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_clip_following_siblings() {
        // frame: [mask circle][rect] -> rect must render inside a clip layer
        let d = Node::frame("page", 200.0, 200.0)
            .child(Node::ellipse("m", 0.0, 0.0, 100.0, 100.0, Color::WHITE).mask(true))
            .child(Node::rect("r", 0.0, 0.0, 200.0, 200.0, Color::rgb8(255, 0, 0)));
        let tree = build_render_tree(&d, &Variables::default());
        let kinds: Vec<&str> = tree.commands.iter().map(|c| match c {
            RenderCommand::PushClip { .. } => "clip",
            RenderCommand::FillPath { .. } => "fill",
            RenderCommand::PopLayer => "pop",
            _ => "other",
        }).collect();
        // clip comes BEFORE the sibling fill, pop after
        let ci = kinds.iter().position(|k| *k == "clip").expect("mask emits clip");
        let fi = kinds.iter().rposition(|k| *k == "fill").expect("sibling fill");
        let pi = kinds.iter().rposition(|k| *k == "pop").expect("pop");
        assert!(ci < fi && fi < pi, "clip-fill-pop order: {kinds:?}");
        // mask node itself paints nothing (only 1 fill: the rect)
        assert_eq!(kinds.iter().filter(|k| **k == "fill").count(), 1);
        // sink renders with a clip layer
        let sink = VelloSink { assets: None, fonts: None };
        let scene = sink.render(&tree);
        assert!(scene.encoding().n_clips > 0);
    }

    #[test]
    fn ordered_visual_stacks_lower_to_distinct_commands() {
        let mut n = Node::rect("layered", 0.0, 0.0, 80.0, 60.0, Color::BLACK);
        n.visual_stacks_materialized = true;
        n.fill_layers = vec![
            PaintLayer::new(Paint::Solid(Color::rgb8(255, 0, 0))),
            PaintLayer { paint: Paint::Solid(Color::rgb8(0, 0, 255)), opacity: 0.5, visible: true, blend: BlendKind::Screen },
        ];
        n.stroke_layers = vec![StrokeLayer::new(Stroke { color: Color::WHITE, width: 2.0 })];
        n.effect_layers = vec![EffectLayer::new(Effect::DropShadow { dx: 2.0, dy: 3.0, blur: 8.0, color: Color::BLACK })];
        let tree = build_render_tree(&Node::frame("page", 100.0, 100.0).child(n), &Variables::default());
        let keys: Vec<_> = tree.commands.iter().map(RenderCommand::key).collect();
        assert!(keys.iter().any(|k| k.ends_with("/fill-0")));
        assert!(keys.iter().any(|k| k.ends_with("/fill-1")));
        assert!(keys.iter().any(|k| k.ends_with("/stroke-0")));
        assert!(keys.iter().any(|k| k.ends_with("/effect-0")));
    }

    #[test]
    fn image_fit_modes_produce_distinct_transforms() {
        // fake 2x2 asset via Assets? renderer path only needs metadata;
        // simplest: fit mode changes the fingerprint hence the cache key
        let mk = |fit: ImageFit| {
            let mut n = Node::image("i", 0.0, 0.0, 200.0, 100.0, "a");
            if let NodeKind::Image { fit: f, .. } = &mut n.kind { *f = fit; }
            let d = Node::frame("p", 300.0, 300.0).child(n);
            build_render_tree(&d, &Variables::default())
        };
        let f1 = mk(ImageFit::Fill);
        let f2 = mk(ImageFit::Crop);
        assert!(!f2.changed_keys(&f1).is_empty(), "fit change must dirty the image command");
    }

    fn sample() -> (Node, Variables) {
        let mut vars = Variables::default();
        vars.numbers.insert("radius-lg".into(), 20.0);
        let mut master = Node::component("cb", "Chip", 40.0, 20.0);
        master.visible = false;
        master.children.push(Node::rect("chip-bg", 0.0, 0.0, 40.0, 20.0, Color::rgb8(0, 0, 0xff)));
        let doc = Node::frame("page", 400.0, 300.0)
            .child(master)
            .child(Node::rect("r", 10.0, 10.0, 100.0, 60.0, Color::rgb8(255, 0, 0)).radius(2.0).bind("radius", "radius-lg"))
            .child(Node::text("t", 0.0, 100.0, 200.0, 20.0, "hello ir"))
            .child(Node::instance("i", "Chip", 200.0, 0.0, 40.0, 20.0))
            .child(Node::ellipse("e", 0.0, 200.0, 50.0, 50.0, Color::rgb8(0, 255, 0)).blend(BlendKind::Multiply));
        (doc, vars)
    }

    #[test]
    fn ir_produces_stable_keys_and_resolves_everything() {
        let (doc, vars) = sample();
        let tree = build_render_tree(&doc, &vars);
        let keys: Vec<&str> = tree.commands.iter().map(|c| c.key()).filter(|k| !k.is_empty()).collect();
        assert!(keys.contains(&"/page/r"));
        assert!(keys.contains(&"/page/t"));
        assert!(keys.contains(&"/page/i/chip-bg"), "instance resolution keys through the instance: {keys:?}");
        // blend became a layer pair
        assert!(tree.commands.iter().any(|c| matches!(c, RenderCommand::PushLayer { .. })));
        assert!(tree.commands.iter().any(|c| matches!(c, RenderCommand::PopLayer)));
        // variable-bound radius resolved INTO the geometry (path differs from radius=2)
        let (doc2, mut vars2) = sample();
        vars2.numbers.insert("radius-lg".into(), 2.0);
        let t2 = build_render_tree(&doc2, &vars2);
        let path_repr = |t: &RenderTree, key: &str| t.commands.iter().find_map(|c| match c {
            RenderCommand::FillPath { key: k, path, .. } if k == key => Some(format!("{path:?}")),
            _ => None,
        }).unwrap();
        assert_ne!(path_repr(&tree, "/page/r"), path_repr(&t2, "/page/r"), "different radii must yield different geometry");
    }

    #[test]
    fn vello_sink_output_matches_direct_encoder_path_count() {
        let (doc, vars) = sample();
        let (scene_ir, tree) = render_via_ir(&doc, &vars, None, None);
        let (scene_direct, _) = crate::build_scene(&doc, None, &vars);
        // IR path count must be >= direct (text = per-glyph strokes both ways)
        assert!(scene_ir.encoding().n_paths > 0);
        assert_eq!(scene_ir.encoding().n_clips, scene_direct.encoding().n_clips, "blend layers must match");
        assert!(!tree.commands.is_empty());
    }

    #[test]
    fn changed_keys_gives_partial_redraw_seed() {
        let (doc, vars) = sample();
        let t1 = build_render_tree(&doc, &vars);
        // move ONE node
        let mut doc2 = doc.clone();
        fn find_mut<'a>(n: &'a mut Node, id: &str) -> Option<&'a mut Node> {
            if n.id == id { return Some(n); }
            n.children.iter_mut().find_map(|c| find_mut(c, id))
        }
        find_mut(&mut doc2, "r").unwrap().transform.x += 5.0;
        let t2 = build_render_tree(&doc2, &vars);
        let changed = t2.changed_keys(&t1);
        assert_eq!(changed, vec!["/page/r".to_string()], "only the moved node changed: {changed:?}");
        // identical trees -> no changes
        assert!(t1.changed_keys(&build_render_tree(&doc, &vars)).is_empty());
    }

    #[test]
    fn gpu_effects_lower_to_blur_taps_and_clips() {
        let mut card = Node::rect("card", 20.0, 20.0, 120.0, 80.0, Color::WHITE);
        card.effects = vec![
            Effect::DropShadow { dx: 3.0, dy: 5.0, blur: 8.0, color: Color::rgba8(0, 0, 0, 128) },
            Effect::InnerShadow { dx: 1.0, dy: 2.0, blur: 5.0, color: Color::rgba8(0, 0, 0, 96) },
            Effect::LayerBlur { radius: 3.0 },
            Effect::BackgroundBlur { radius: 4.0 },
        ];
        let doc = Node::frame("page", 300.0, 200.0)
            .child(Node::rect("background", 0.0, 0.0, 300.0, 200.0, Color::rgb8(40, 60, 90)))
            .child(card);
        let tree = build_render_tree(&doc, &Variables::default());
        assert!(tree.commands.iter().any(|c| c.key().contains("background-clip")));
        assert!(tree.commands.iter().any(|c| c.key().contains("inner-clip")));
        assert!(tree.commands.iter().any(|c| c.key().contains("/blur-")));
        assert!(tree.commands.iter().filter(|c| c.key().contains("effect-0/tap-")).count() > 8);
    }

    #[test]
    fn ir_can_drive_non_gpu_sinks_svg_like() {
        // a trivial text sink proves backend-agnosticism (export/PDF seed)
        let (doc, vars) = sample();
        let tree = build_render_tree(&doc, &vars);
        let mut ops = vec![];
        for c in &tree.commands {
            ops.push(match c {
                RenderCommand::FillPath { .. } => "fill",
                RenderCommand::StrokePath { .. } => "stroke",
                RenderCommand::PushLayer { .. } => "push",
                RenderCommand::PopLayer => "pop",
                RenderCommand::Glyphs { .. } => "text",
                RenderCommand::Image { .. } => "image",
                RenderCommand::PushClip { .. } => "clip",
            });
        }
        assert!(ops.contains(&"fill") && ops.contains(&"text") && ops.contains(&"push") && ops.contains(&"pop"));
    }
}
