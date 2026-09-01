
use vello::kurbo::{Affine, Point, Rect};

use x_core::*;
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------- hit testing

/// Topmost hittable node id at `point` (world coords). Children are on top
/// of parents; later siblings are on top of earlier ones (paint order).
/// Locked / hidden nodes (and their subtrees, if hidden) are skipped.
pub fn hit_test(root: &Node, point: Point) -> Option<String> {
    fn walk(node: &Node, parent: Affine, point: Point, out: &mut Option<String>) {
        if !node.visible { return; }
        let world = parent * node.transform.matrix(node.w, node.h);
        // children first? No — paint order is parent, then children in order,
        // so later hits simply overwrite `out`.
        if !node.locked {
            let local = world.inverse() * point;
            let inside = match node.kind {
                NodeKind::Ellipse => {
                    let (rx, ry) = (node.w / 2.0, node.h / 2.0);
                    let (dx, dy) = ((local.x - rx) / rx, (local.y - ry) / ry);
                    dx * dx + dy * dy <= 1.0
                }
                // Plain Groups have no paintable body (no fill/stroke of their
                // own in Figma's model), so clicks pass through empty group
                // area to whatever is beneath. Frames, master Components, and
                // Instances DO have a real fill/stroke — like Figma, clicking
                // their body OR their stroke/border must select and let the
                // user drag the container itself, not just its children.
                NodeKind::Group => false,
                NodeKind::Frame { .. } | NodeKind::Component { .. } | NodeKind::Instance { .. } => {
                    local.x >= 0.0 && local.y >= 0.0 && local.x <= node.w && local.y <= node.h
                }
                _ => local.x >= 0.0 && local.y >= 0.0 && local.x <= node.w && local.y <= node.h,
            };
            if inside { *out = Some(node.id.clone()); }
        }
        for child in &node.children { walk(child, world, point, out); }
    }
    let mut out = None;
    // The document root is the canvas, not a selectable object. Nested
    // frames/components remain hittable via `walk`.
    let world = Affine::IDENTITY * root.transform.matrix(root.w, root.h);
    for child in &root.children {
        walk(child, world, point, &mut out);
    }
    out
}

/// All node ids whose world AABB intersects `rect` (marquee select).
pub fn hit_test_rect(root: &Node, rect: Rect) -> Vec<String> {
    fn walk(node: &Node, parent: Affine, rect: Rect, out: &mut Vec<String>) {
        if !node.visible { return; }
        let world = parent * node.transform.matrix(node.w, node.h);
        if !node.locked && !matches!(node.kind, NodeKind::Group) {
            let b = bounds(world, node.w, node.h);
            if b.x0 < rect.x1 && b.x1 > rect.x0 && b.y0 < rect.y1 && b.y1 > rect.y0 {
                out.push(node.id.clone());
            }
        }
        for child in &node.children { walk(child, world, rect, out); }
    }
    let mut out = vec![];
    let world = Affine::IDENTITY * root.transform.matrix(root.w, root.h);
    for child in &root.children {
        walk(child, world, rect, &mut out);
    }
    out
}

/// industry-standard selection model: a plain click selects the TOP-LEVEL object
/// (direct child of the page) that contains the hit; only deep-select
/// (Ctrl/Cmd+click) or double-click drills into nested children.
/// Maps a (deep) hit id to its top-level ancestor's id.
pub fn top_level_ancestor(root: &Node, id: &str) -> Option<String> {
    for child in &root.children {
        if child.id == id || find(child, id).is_some() {
            return Some(child.id.clone());
        }
    }
    None
}

// ------------------------------------------------------------ tree plumbing

pub fn find<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
    if node.id == id { return Some(node); }
    node.children.iter().find_map(|c| find(c, id))
}
pub fn find_mut<'a>(node: &'a mut Node, id: &str) -> Option<&'a mut Node> {
    if node.id == id { return Some(node); }
    node.children.iter_mut().find_map(|c| find_mut(c, id))
}
pub(crate) fn find_parent_mut<'a>(node: &'a mut Node, id: &str) -> Option<&'a mut Node> {
    if node.children.iter().any(|c| c.id == id) { return Some(node); }
    node.children.iter_mut().find_map(|c| find_parent_mut(c, id))
}

