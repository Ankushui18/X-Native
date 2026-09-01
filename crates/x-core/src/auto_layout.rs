use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------- auto layout

/// Auto Layout v2: gap/padding variables, cross-axis alignment,
/// space-between, main-axis hug AND cross-axis hug.
pub fn apply_auto_layout(node: &mut Node, vars: &Variables) {
    let layout = match &node.kind { NodeKind::Frame { layout: Some(l) } => l.clone(), _ => return };
    let gap0 = layout.gap_var.as_deref().map(|n| vars.number(n, layout.gap)).unwrap_or(layout.gap);
    let padding = layout.padding_var.as_deref().map(|n| vars.number(n, layout.padding)).unwrap_or(layout.padding);

    let horizontal = layout.direction == LayoutDirection::Horizontal;
    let n = node.children.len();
    let content_main: f64 = node.children.iter().map(|c| if horizontal { c.w } else { c.h }).sum();
    let container_main = if horizontal { node.w } else { node.h };

    // space-between: distribute leftover space as gap (Fixed frames, 2+ children).
    let gap = if layout.space_between && layout.sizing == Sizing::Fixed && n > 1 {
        ((container_main - 2.0 * padding - content_main) / (n as f64 - 1.0)).max(0.0)
    } else { gap0 };

    let cross_extent = node.children.iter().map(|c| if horizontal { c.h } else { c.w }).fold(0.0f64, f64::max);
    let container_cross = if layout.sizing == Sizing::Hug { cross_extent + 2.0 * padding } else if horizontal { node.h } else { node.w };

    let mut cursor = padding;
    for child in &mut node.children {
        let child_cross = if horizontal { child.h } else { child.w };
        let cross_pos = match layout.align {
            CrossAlign::Start => padding,
            CrossAlign::Center => (container_cross - child_cross) / 2.0,
            CrossAlign::End => container_cross - padding - child_cross,
        };
        if horizontal {
            child.transform.x = cursor; child.transform.y = cross_pos;
            cursor += child.w + gap;
        } else {
            child.transform.y = cursor; child.transform.x = cross_pos;
            cursor += child.h + gap;
        }
        child.dirty = true;
    }
    if layout.sizing == Sizing::Hug {
        let main = if n > 0 { cursor - gap + padding } else { 2.0 * padding };
        if horizontal { node.w = main; node.h = container_cross; } else { node.h = main; node.w = container_cross; }
    }
    node.dirty = false;
}

/// Phase 5.1: recursive layout solve — children first (post-order), so a Hug
/// child reports its final size before the parent positions it.
pub fn apply_layout_recursive(node: &mut Node, vars: &Variables) {
    for child in &mut node.children { apply_layout_recursive(child, vars); }
    apply_auto_layout(node, vars);
}

