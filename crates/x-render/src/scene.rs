use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Fill};
use vello::Scene;
use x_core::*;
#[allow(unused_imports)]
use crate::*;

// ----------------------------------------------------------------- encoding

pub fn build_scene(root: &Node, viewport: Option<Viewport>, vars: &Variables) -> (Scene, SceneStats) {
    build_scene_with_assets(root, viewport, vars, None)
}

/// Phase 4.2: like `build_scene`, but Image nodes whose `asset` is present
/// in `assets` render the actual decoded bitmap instead of a placeholder.
pub fn build_scene_with_assets(root: &Node, viewport: Option<Viewport>, vars: &Variables, assets: Option<&Assets>) -> (Scene, SceneStats) {
    build_scene_full(root, viewport, vars, assets, None)
}

/// Full pipeline: assets + real typography. When `fonts` is Some, Text
/// nodes render with real TTF outlines (kerned, word-wrapped to node
/// width); otherwise the built-in stroke font keeps working.
pub fn build_scene_full(root: &Node, viewport: Option<Viewport>, vars: &Variables, assets: Option<&Assets>, fonts: Option<&x_text::FontManager>) -> (Scene, SceneStats) {
    let mut scene = Scene::new();
    let mut stats = SceneStats::default();
    let mut registry = ComponentRegistry::new();
    collect_components(root, &mut registry);
    let empty = HashMap::new();
    let ctx = EncodeCtx { assets, fonts };
    encode(&mut scene, root, Affine::IDENTITY, viewport, vars, &mut stats, &registry, &empty, 0, &ctx);
    (scene, stats)
}

pub struct EncodeCtx<'a> {
    pub assets: Option<&'a Assets>,
    pub fonts: Option<&'a x_text::FontManager>,
}

fn shape_for_rect(node: &Node, radius: f64) -> vello::kurbo::BezPath {
    if let Some([tl, tr, br, bl]) = node.corner_radii {
        RoundedRect::from_rect(Rect::new(0.0, 0.0, node.w, node.h), RoundedRectRadii::new(tl, tr, br, bl)).into_path(0.1)
    } else if radius > 0.0 {
        RoundedRect::new(0.0, 0.0, node.w, node.h, radius).into_path(0.1)
    } else {
        Rect::new(0.0, 0.0, node.w, node.h).into_path(0.1)
    }
}

fn encode_drop_shadows(scene: &mut Scene, node: &Node, world: Affine, shape: &impl Shape, stats: &mut SceneStats) {
    for effect in &node.effects {
        if let Effect::DropShadow { dx, dy, blur, color } = effect {
            // No blur primitive in Vello 0.1: widen by the blur radius and
            // reduce alpha, which reads as a soft-ish shadow at small radii.
            let grow = blur * 0.5;
            let b = shape.bounding_box();
            let sx = if b.width() > 0.0 { (b.width() + grow * 2.0) / b.width() } else { 1.0 };
            let sy = if b.height() > 0.0 { (b.height() + grow * 2.0) / b.height() } else { 1.0 };
            let t = world * Affine::translate((dx - grow, dy - grow)) * Affine::scale_non_uniform(sx, sy);
            scene.fill(Fill::NonZero, t, color.multiply_alpha(0.55 * node.opacity), None, shape);
            stats.paths += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn encode(scene: &mut Scene, node: &Node, parent: Affine, viewport: Option<Viewport>, vars: &Variables, stats: &mut SceneStats, registry: &ComponentRegistry, overrides: &HashMap<String, String>, depth: u32, ctx: &EncodeCtx) {
    stats.nodes += 1;
    if node.dirty { stats.dirty_nodes += 1; }
    // typed overrides that alter traversal: visible / opacity / swap
    let mut swapped: Option<Node> = None;
    let node = {
        let mut effective_visible = node.visible;
        let mut effective_opacity: Option<f32> = None;
        if let Some(raw) = overrides.get(&node.id) {
            if let Some(v) = raw.strip_prefix("visible:") {
                if let Ok(b) = v.parse::<bool>() { effective_visible = b; }
            } else if let Some(o) = raw.strip_prefix("opacity:") {
                if let Ok(f) = o.parse::<f32>() { effective_opacity = Some(f); }
            } else if let Some(c) = raw.strip_prefix("swap:") {
                if matches!(node.kind, NodeKind::Instance { .. }) {
                    let mut n2 = node.clone();
                    n2.kind = NodeKind::Instance { component: c.to_string() };
                    swapped = Some(n2);
                }
            }
        }
        if !effective_visible { return; }
        if let Some(op) = effective_opacity {
            let mut n2 = swapped.take().unwrap_or_else(|| node.clone());
            n2.opacity = op.clamp(0.0, 1.0);
            swapped = Some(n2);
        }
        swapped.as_ref().unwrap_or(node)
    };
    if !node.visible { return; }
    let world = parent * node.transform.matrix(node.w, node.h);
    // P1 binding: opacity -> number variable (0..1)
    let node_storage;
    let node = if node.bindings.contains_key("opacity") {
        let mut n2 = node.clone();
        n2.opacity = node.bound_number("opacity", vars, node.opacity as f64) as f32;
        node_storage = n2;
        &node_storage
    } else { node };
    let b = bounds(world, node.w, node.h);
    if let Some(v) = viewport {
        if !intersects(b, Rect::new(v.x, v.y, v.x + v.w, v.y + v.h)) { stats.culled += 1; return; }
    }

    // Phase 4: blend layer around this node + its subtree.
    let blend = node.blend.mix();
    if let Some(mix) = blend {
        scene.push_layer(Fill::NonZero, mix, 1.0, Affine::IDENTITY, &b);
    }

    let mut frame_clip_shape: Option<vello::kurbo::BezPath> = None;

    match &node.kind {
        NodeKind::Rect { radius } => {
            let bound_radius = node.bound_number("radius", vars, *radius);
            let shape = shape_for_rect(node, bound_radius);
            encode_drop_shadows(scene, node, world, &shape, stats);
            scene.fill(Fill::NonZero, world, &brush_with_alpha(effective_brush(node, overrides, vars), node.opacity), None, &shape);
            if node.stroke.width > 0.0 {
                scene.stroke(&vello::kurbo::Stroke::new(node.stroke.width), world, &brush_with_alpha(paint_brush(&node.stroke.paint, vars), node.opacity), None, &shape);
                stats.paths += 1;
            }
            stats.paths += 1;
        }
        NodeKind::Ellipse => {
            let r = node.w.min(node.h) / 2.0;
            let shape = Circle::new((r, r), r);
            let t = world * Affine::scale_non_uniform(node.w / node.h, 1.0);
            encode_drop_shadows(scene, node, t, &shape.into_path(0.1), stats);
            scene.fill(Fill::NonZero, t, &brush_with_alpha(effective_brush(node, overrides, vars), node.opacity), None, &shape);
            stats.paths += 1;
        }
        NodeKind::Line => {
            let shape = Rect::new(0.0, 0.0, node.w.max(node.stroke.width), node.stroke.width.max(1.0)).into_path(0.1);
            scene.fill(Fill::NonZero, world, &brush_with_alpha(paint_brush(&node.stroke.paint, vars), node.opacity), None, &shape);
            stats.paths += 1;
        }
        NodeKind::Image { asset, .. } => {
            if let Some(img) = ctx.assets.and_then(|a| a.get(asset)) {
                // draw the decoded bitmap scaled into the node's box
                let sx = node.w / img.image.width as f64;
                let sy = node.h / img.image.height as f64;
                scene.draw_image(img, world * Affine::scale_non_uniform(sx, sy));
                stats.paths += 1;
            } else {
                let shape = Rect::new(0.0, 0.0, node.w, node.h).into_path(0.1);
                scene.fill(Fill::NonZero, world, &effective_brush(node, overrides, vars), None, &shape);
                stats.paths += 1;
            }
        }
        NodeKind::Text { text } => {
            let content = effective_text(node, overrides).unwrap_or(text);
            let color = effective_fill(node, overrides, vars).multiply_alpha(node.opacity);
            // Real typography when a FontManager is present; node.h is the
            // font size (em), node.w the wrap width.
            let drew = if let Some(fm) = ctx.fonts {
                if let Some(font) = fm.default_font() {
                    stats.paths += fm.encode_text_block(scene, content, world, font, node.h * 0.72, Some(node.w.max(8.0)), color);
                    true
                } else { false }
            } else { false };
            if !drew {
                stats.paths += x_text::encode_text(scene, content, world, node.h, color);
            }
        }
        NodeKind::Vector { path } => {
            // Phase 2.6: real editable vector paths render as filled shapes.
            if !path.is_empty() {
                let bez = path_to_bez(path);
                encode_drop_shadows(scene, node, world, &bez, stats);
                scene.fill(Fill::NonZero, world, &brush_with_alpha(effective_brush(node, overrides, vars), node.opacity), None, &bez);
                if node.stroke.width > 0.0 {
                    scene.stroke(&vello::kurbo::Stroke::new(node.stroke.width), world, &brush_with_alpha(paint_brush(&node.stroke.paint, vars), node.opacity), None, &bez);
                    stats.paths += 1;
                }
                stats.paths += 1;
            }
        }
        NodeKind::Instance { component } => {
            if depth < MAX_INSTANCE_DEPTH {
                // swap override targeting THIS instance id (set by a parent
                // instance) has already been applied by the parent pass;
                // here resolve our own component name.
                if let Some(def) = registry.get(component.as_str()) {
                    for child in &def.children {
                        encode(scene, child, world, viewport, vars, stats, registry, &node.overrides, depth + 1, ctx);
                    }
                }
            }
        }
        NodeKind::Frame { .. } => {
            // Frames draw their background fill when it isn't transparent
            // (matches Figma: frames have fills; groups do not). Corner
            // radii apply here too, same as a Rect node.
            let color = effective_fill(node, overrides, vars);
            let shape = shape_for_rect(node, 0.0);
            if color.components[3] > 0.0 {
                encode_drop_shadows(scene, node, world, &shape, stats);
                scene.fill(Fill::NonZero, world, &brush_with_alpha(effective_brush(node, overrides, vars), node.opacity), None, &shape);
                stats.paths += 1;
            }
            // Clip children to the frame's own (rounded) bounds ONLY when
            // it actually has corner radii — the case where clipping is
            // visually load-bearing. Square frames behave like groups
            // (no unconditional clip), which also keeps the direct
            // encoder in lockstep with the IR lowering in ir.rs.
            let rounded = node.corner_radii
                .map(|[tl, tr, br, bl]| tl > 0.0 || tr > 0.0 || br > 0.0 || bl > 0.0)
                .unwrap_or(false);
            if rounded { frame_clip_shape = Some(shape); }
        }
        NodeKind::Group | NodeKind::Component { .. } => {}
    }
    // Clip children to the frame's own rounded bounds (only set when the
    // frame has corner radii), so a child's shadow/overflow can't bleed
    // past the rounded corner — see the Frame branch above.
    if let Some(shape) = &frame_clip_shape {
        scene.push_clip_layer(Fill::NonZero, world, shape);
    }
    for child in &node.children { encode(scene, child, world, viewport, vars, stats, registry, overrides, depth, ctx); }
    if frame_clip_shape.is_some() { scene.pop_layer(); }

    if blend.is_some() { scene.pop_layer(); }
}

fn brush_with_alpha(brush: Brush, alpha: f32) -> Brush {
    if alpha >= 1.0 { return brush; }
    match brush {
        Brush::Solid(c) => Brush::Solid(c.multiply_alpha(alpha)),
        Brush::Gradient(mut g) => {
            for stop in g.stops.iter_mut() { stop.color = stop.color.multiply_alpha(alpha); }
            Brush::Gradient(g)
        }
        other => other,
    }
}

