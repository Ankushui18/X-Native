use std::collections::HashMap;
use kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------- auto layout

/// Auto Layout v2: gap/padding variables, cross-axis alignment,
/// space-between, main-axis hug AND cross-axis hug.
pub fn apply_auto_layout(node: &mut Node, vars: &Variables) {
    let layout = match &node.kind { NodeKind::Frame { layout: Some(l) } => l.clone(), _ => return };
    let gap0 = layout.gap_var.as_deref().map(|n| vars.number(n, layout.gap)).unwrap_or(layout.gap);
    // A padding variable overrides ALL four sides (documented uniform override).
    let [pl, pr, pt, pb] = layout.padding_var.as_deref()
        .map(|n| [vars.number(n, layout.padding[0]); 4])
        .unwrap_or(layout.padding);

    let horizontal = layout.direction == LayoutDirection::Horizontal;
    let n = node.children.len();
    let (m0, m1, c0, c1) = if horizontal { (pl, pr, pt, pb) } else { (pt, pb, pl, pr) };

    if layout.wrap == AutoLayoutWrap::Wrap && layout.sizing == Sizing::Fixed {
        apply_wrap_layout(node, &layout, gap0, m0, m1, c0, c1, horizontal);
        return;
    }

    let content_main: f64 = node.children.iter().map(|c| if horizontal { c.w } else { c.h }).sum();
    let container_main = if horizontal { node.w } else { node.h };

    // space-between: distribute leftover space as gap (Fixed frames, 2+ children).
    let gap = if layout.space_between && layout.sizing == Sizing::Fixed && n > 1 {
        ((container_main - m0 - m1 - content_main) / (n as f64 - 1.0)).max(0.0)
    } else { gap0 };

    let cross_extent = node.children.iter().map(|c| if horizontal { c.h } else { c.w }).fold(0.0f64, f64::max);
    let hug_cross = layout.cross() == Sizing::Hug;
    let container_cross = if hug_cross { cross_extent + c0 + c1 } else if horizontal { node.h } else { node.w };

    let mut cursor = m0;
    for child in &mut node.children {
        let child_cross = if horizontal { child.h } else { child.w };
        let cross_pos = match layout.align {
            CrossAlign::Start => c0,
            CrossAlign::Center => (container_cross - child_cross) / 2.0,
            CrossAlign::End => container_cross - c1 - child_cross,
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
    if layout.sizing == Sizing::Hug || hug_cross {
        let main = if n > 0 { cursor - gap + m1 } else { m0 + m1 };
        if horizontal {
            if layout.sizing == Sizing::Hug { node.w = main; }
            if hug_cross { node.h = container_cross; }
        } else {
            if layout.sizing == Sizing::Hug { node.h = main; }
            if hug_cross { node.w = container_cross; }
        }
    }
    node.dirty = false;
}

/// Wrap mode: pack children along the main axis until the fixed size is
/// exceeded, then start a new row (horizontal layout) or column (vertical
/// layout) — same idea as Figma's "Wrap" auto-layout, used for chip lists,
/// tag grids, etc. `gap` is reused as both the item gap within a row and
/// the gap between rows (this engine models a single gap value).
fn apply_wrap_layout(node: &mut Node, layout: &AutoLayout, gap: f64, m0: f64, m1: f64, c0: f64, c1: f64, horizontal: bool) {
    let container_main = if horizontal { node.w } else { node.h };
    let avail_main = (container_main - m0 - m1).max(0.0);

    let mut rows: Vec<Vec<usize>> = vec![];
    let mut cur_row: Vec<usize> = vec![];
    let mut cur_main = 0.0f64;
    for (i, child) in node.children.iter().enumerate() {
        let cm = if horizontal { child.w } else { child.h };
        let would_be = if cur_row.is_empty() { cm } else { cur_main + gap + cm };
        if !cur_row.is_empty() && would_be > avail_main {
            rows.push(std::mem::take(&mut cur_row));
            cur_main = cm;
        } else {
            cur_main = would_be;
        }
        cur_row.push(i);
    }
    if !cur_row.is_empty() { rows.push(cur_row); }

    let mut cross_cursor = c0;
    for row in &rows {
        let row_cross_extent = row.iter()
            .map(|&i| { let c = &node.children[i]; if horizontal { c.h } else { c.w } })
            .fold(0.0f64, f64::max);
        let mut main_cursor = m0;
        for &i in row {
            let child = &mut node.children[i];
            let cm = if horizontal { child.w } else { child.h };
            let child_cross = if horizontal { child.h } else { child.w };
            let cross_pos = match layout.align {
                CrossAlign::Start => cross_cursor,
                CrossAlign::Center => cross_cursor + (row_cross_extent - child_cross) / 2.0,
                CrossAlign::End => cross_cursor + row_cross_extent - child_cross,
            };
            if horizontal { child.transform.x = main_cursor; child.transform.y = cross_pos; }
            else { child.transform.y = main_cursor; child.transform.x = cross_pos; }
            child.dirty = true;
            main_cursor += cm + gap;
        }
        cross_cursor += row_cross_extent + gap;
    }
    // total cross extent = last cursor minus the trailing gap, plus the
    // far-side padding; falls back to just the padding when there are no
    // rows at all (an empty auto-layout frame).
    let total_cross = if rows.is_empty() { c0 + c1 } else { cross_cursor - gap + c1 };
    if layout.resize_on_wrap || layout.cross() == Sizing::Hug {
        if horizontal { node.h = total_cross; } else { node.w = total_cross; }
    }
    node.dirty = false;
}

/// Phase 5.1: recursive layout solve — children first (post-order), so a Hug
/// child reports its final size before the parent positions it.
pub fn apply_layout_recursive(node: &mut Node, vars: &Variables) {
    for child in &mut node.children { apply_layout_recursive(child, vars); }
    apply_auto_layout(node, vars);
}

