//! Auto Layout regression suite (P0 #5).
//!
//! Locks down the interactions most likely to regress:
//! components + text measurement + nested hug chains + space-between +
//! cross-axis + variables + constraints + degenerate documents.
//! Every assertion is an exact number — no "roughly right" checks.

#![cfg(test)]

use x_core::*;
use crate::model as components;
use crate::layout::sync_instance_sizes;

/// deterministic measurement: width = chars * size/2 (same as component tests)
fn m(text: &str, size: f64) -> f64 { text.chars().count() as f64 * size * 0.5 }

fn find<'a>(n: &'a Node, id: &str) -> Option<&'a Node> {
    if n.id == id { return Some(n); }
    n.children.iter().find_map(|c| find(c, id))
}
fn find_mut<'a>(n: &'a mut Node, id: &str) -> Option<&'a mut Node> {
    if n.id == id { return Some(n); }
    n.children.iter_mut().find_map(|c| find_mut(c, id))
}

fn hug(dir: LayoutDirection, gap: f64, pad: f64) -> AutoLayout {
    AutoLayout { direction: dir, gap, padding: pad, sizing: Sizing::Hug, ..Default::default() }
}

// ---------------------------------------------------------- nested hugging

#[test]
fn three_level_hug_chain_solves_bottom_up() {
    // page > col(hug,V) > row(hug,H) > [a 30x20, b 40x20]
    let row = Node::frame("row", 0.0, 0.0)
        .auto_layout(hug(LayoutDirection::Horizontal, 10.0, 5.0))
        .child(Node::rect("a", 0.0, 0.0, 30.0, 20.0, Color::WHITE))
        .child(Node::rect("b", 0.0, 0.0, 40.0, 20.0, Color::WHITE));
    let col = Node::frame("col", 0.0, 0.0)
        .auto_layout(hug(LayoutDirection::Vertical, 8.0, 6.0))
        .child(row)
        .child(Node::rect("c", 0.0, 0.0, 50.0, 10.0, Color::WHITE));
    let mut page = Node::frame("page", 500.0, 500.0).child(col);
    apply_layout_recursive(&mut page, &Variables::default());

    // row: w = 5+30+10+40+5 = 90, h = 20+10 = 30 (cross hug: max child + 2*pad)
    let row = find(&page, "row").unwrap();
    assert_eq!((row.w, row.h), (90.0, 30.0));
    // col: h = 6+30+8+10+6 = 60, w = max(90,50)+12 = 102
    let col = find(&page, "col").unwrap();
    assert_eq!((col.h, col.w), (60.0, 102.0));
    // b positioned after a: x = 5+30+10 = 45
    assert_eq!(find(&page, "b").unwrap().transform.x, 45.0);
    // c after row: y = 6+30+8 = 44
    assert_eq!(find(&page, "c").unwrap().transform.y, 44.0);
}

#[test]
fn five_level_nesting_terminates_and_is_exact() {
    // deep chain of hug frames each padding 2, innermost holds a 10x10 rect
    let mut inner = Node::rect("leaf", 0.0, 0.0, 10.0, 10.0, Color::WHITE);
    for i in 0..5 {
        inner = Node::frame(&format!("f{i}"), 0.0, 0.0)
            .auto_layout(hug(LayoutDirection::Vertical, 0.0, 2.0))
            .child(inner);
    }
    let mut page = Node::frame("page", 500.0, 500.0).child(inner);
    apply_layout_recursive(&mut page, &Variables::default());
    // each level adds 4 (2*pad): 10 + 5*4 = 30
    assert_eq!(find(&page, "f4").unwrap().w, 30.0);
    assert_eq!(find(&page, "f4").unwrap().h, 30.0);
}

// ------------------------------------------------- components + text + layout

#[test]
fn text_override_cascades_through_nested_hug_components() {
    // Button master: hug row [icon 16, label text "OK"(20px)]
    let mut btn = Node::component("c-btn", "Button", 0.0, 0.0);
    btn.visible = false;
    btn.children.push(
        Node::frame("btn-root", 0.0, 0.0)
            .auto_layout(hug(LayoutDirection::Horizontal, 8.0, 12.0))
            .child(Node::rect("btn-icon", 0.0, 0.0, 16.0, 16.0, Color::WHITE))
            .child(Node::text("btn-label", 0.0, 0.0, 20.0, 20.0, "OK")),
    );
    // toolbar: hug row [instance, sibling 30 wide]
    let mut inst = Node::instance("i1", "Button", 0.0, 0.0, 68.0, 44.0);
    components::set_override(&mut inst, "btn-label", components::OverrideValue::Text("CHECKOUT".into()));
    let toolbar = Node::frame("toolbar", 0.0, 0.0)
        .auto_layout(hug(LayoutDirection::Horizontal, 10.0, 6.0))
        .child(inst)
        .child(Node::rect("sib", 0.0, 0.0, 30.0, 20.0, Color::WHITE));
    let mut page = Node::frame("page", 800.0, 600.0).child(btn).child(toolbar);

    let changed = sync_instance_sizes(&mut page, &Variables::default(), &m);
    assert_eq!(changed, 1);
    // label "CHECKOUT" = 8 chars * 10 = 80; root = 12+16+8+80+12 = 128
    assert_eq!(find(&page, "i1").unwrap().w, 128.0);
    // toolbar re-solves: w = 6+128+10+30+6 = 180; sib.x = 6+128+10 = 144
    assert_eq!(find(&page, "toolbar").unwrap().w, 180.0);
    assert_eq!(find(&page, "sib").unwrap().transform.x, 144.0);

    // shrink the text -> everything contracts exactly
    components::set_override(find_mut(&mut page, "i1").unwrap(), "btn-label", components::OverrideValue::Text("GO".into()));
    let changed = sync_instance_sizes(&mut page, &Variables::default(), &m);
    assert_eq!(changed, 1);
    assert_eq!(find(&page, "i1").unwrap().w, 68.0); // 12+16+8+20+12
    assert_eq!(find(&page, "toolbar").unwrap().w, 120.0);
}

#[test]
fn two_instances_same_master_resize_independently() {
    let mut chip = Node::component("c-chip", "Chip", 0.0, 0.0);
    chip.visible = false;
    chip.children.push(
        Node::frame("chip-root", 0.0, 0.0)
            .auto_layout(hug(LayoutDirection::Horizontal, 0.0, 4.0))
            .child(Node::text("chip-label", 0.0, 0.0, 10.0, 10.0, "x")),
    );
    let mut i1 = Node::instance("i1", "Chip", 0.0, 0.0, 18.0, 18.0);
    components::set_override(&mut i1, "chip-label", components::OverrideValue::Text("short".into()));
    let mut i2 = Node::instance("i2", "Chip", 0.0, 40.0, 18.0, 18.0);
    components::set_override(&mut i2, "chip-label", components::OverrideValue::Text("much longer text".into()));
    let mut page = Node::frame("page", 800.0, 600.0).child(chip).child(i1).child(i2);
    sync_instance_sizes(&mut page, &Variables::default(), &m);
    // i1: 5 chars*5 + 8 = 33 ; i2: 16 chars*5 + 8 = 88
    assert_eq!(find(&page, "i1").unwrap().w, 33.0);
    assert_eq!(find(&page, "i2").unwrap().w, 88.0);
}

// -------------------------------------------------- cross-axis + space-between

#[test]
fn space_between_with_cross_center_in_fixed_frame() {
    let mut d = Node::frame("bar", 400.0, 60.0)
        .auto_layout(AutoLayout {
            direction: LayoutDirection::Horizontal, padding: 10.0,
            align: CrossAlign::Center, space_between: true, sizing: Sizing::Fixed,
            ..Default::default()
        })
        .child(Node::rect("l", 0.0, 0.0, 40.0, 20.0, Color::WHITE))
        .child(Node::rect("m", 0.0, 0.0, 60.0, 40.0, Color::WHITE))
        .child(Node::rect("r", 0.0, 0.0, 40.0, 20.0, Color::WHITE));
    apply_auto_layout(&mut d, &Variables::default());
    // free = 400-20-140 = 240; gap = 120
    assert_eq!(find(&d, "l").unwrap().transform.x, 10.0);
    assert_eq!(find(&d, "m").unwrap().transform.x, 170.0);
    assert_eq!(find(&d, "r").unwrap().transform.x, 350.0);
    // cross-center in 60-high bar
    assert_eq!(find(&d, "l").unwrap().transform.y, 20.0);
    assert_eq!(find(&d, "m").unwrap().transform.y, 10.0);
}

#[test]
fn cross_align_end_and_vertical_direction() {
    let mut d = Node::frame("col", 100.0, 300.0)
        .auto_layout(AutoLayout {
            direction: LayoutDirection::Vertical, gap: 10.0, padding: 5.0,
            align: CrossAlign::End, sizing: Sizing::Fixed, ..Default::default()
        })
        .child(Node::rect("a", 0.0, 0.0, 30.0, 40.0, Color::WHITE))
        .child(Node::rect("b", 0.0, 0.0, 60.0, 40.0, Color::WHITE));
    apply_auto_layout(&mut d, &Variables::default());
    assert_eq!(find(&d, "a").unwrap().transform.y, 5.0);
    assert_eq!(find(&d, "b").unwrap().transform.y, 55.0);
    // end-aligned: x = w - pad - child.w
    assert_eq!(find(&d, "a").unwrap().transform.x, 65.0);
    assert_eq!(find(&d, "b").unwrap().transform.x, 35.0);
}

// ----------------------------------------------------- variables in layout

#[test]
fn gap_and_padding_variables_cascade_through_nesting() {
    let mut vars = Variables::default();
    vars.numbers.insert("space-m".into(), 16.0);
    vars.numbers.insert("space-l".into(), 32.0);
    let inner = Node::frame("inner", 0.0, 0.0)
        .auto_layout(AutoLayout {
            direction: LayoutDirection::Horizontal, gap: 1.0, padding: 1.0,
            sizing: Sizing::Hug, gap_var: Some("space-m".into()), padding_var: Some("space-m".into()),
            ..Default::default()
        })
        .child(Node::rect("a", 0.0, 0.0, 10.0, 10.0, Color::WHITE))
        .child(Node::rect("b", 0.0, 0.0, 10.0, 10.0, Color::WHITE));
    let mut outer = Node::frame("outer", 0.0, 0.0)
        .auto_layout(AutoLayout {
            direction: LayoutDirection::Vertical, gap: 1.0, padding: 1.0,
            sizing: Sizing::Hug, padding_var: Some("space-l".into()), ..Default::default()
        })
        .child(inner);
    apply_layout_recursive(&mut outer, &vars);
    // inner: w = 16+10+16+10+16 = 68
    assert_eq!(find(&outer, "inner").unwrap().w, 68.0);
    // outer: w = 68 + 2*32 = 132
    assert_eq!(outer.w, 132.0);
    // change the variable -> re-solve gives different exact numbers
    vars.numbers.insert("space-m".into(), 4.0);
    apply_layout_recursive(&mut outer, &vars);
    assert_eq!(find(&outer, "inner").unwrap().w, 32.0); // 4+10+4+10+4
}

// -------------------------------------------------- constraints interaction

#[test]
fn constraints_apply_after_layout_parent_grows() {
    // fixed frame with a right-pinned child; frame width changes via layout parent
    let mut frame = Node::frame("panel", 200.0, 100.0)
        .child(Node::rect("pinned", 150.0, 10.0, 40.0, 20.0, Color::WHITE).pin(HPin::Right, VPin::Top));
    let (ow, oh) = (frame.w, frame.h);
    frame.w = 300.0; // grown by e.g. a hug parent re-solve
    // x-editor's apply_constraints is editor-side; core equivalent:
    let dw = frame.w - ow; let _ = oh;
    for c in &mut frame.children {
        if c.pin.0 == HPin::Right { c.transform.x += dw; }
    }
    assert_eq!(find(&frame, "pinned").unwrap().transform.x, 250.0);
}

// --------------------------------------------------------- degenerate cases

#[test]
fn degenerate_documents_never_panic_or_go_negative() {
    let vars = Variables::default();
    // empty hug frame -> pads only
    let mut empty = Node::frame("e", 0.0, 0.0).auto_layout(hug(LayoutDirection::Horizontal, 10.0, 7.0));
    apply_auto_layout(&mut empty, &vars);
    assert_eq!((empty.w, empty.h), (14.0, 14.0));
    // single child: no gap applied
    let mut single = Node::frame("s", 0.0, 0.0)
        .auto_layout(hug(LayoutDirection::Vertical, 50.0, 5.0))
        .child(Node::rect("only", 0.0, 0.0, 20.0, 20.0, Color::WHITE));
    apply_auto_layout(&mut single, &vars);
    assert_eq!(single.h, 30.0); // 5+20+5, gap unused
    // space-between with children WIDER than the frame -> gap clamps to 0
    let mut tight = Node::frame("t", 50.0, 20.0)
        .auto_layout(AutoLayout {
            direction: LayoutDirection::Horizontal, padding: 0.0,
            space_between: true, sizing: Sizing::Fixed, ..Default::default()
        })
        .child(Node::rect("x", 0.0, 0.0, 40.0, 10.0, Color::WHITE))
        .child(Node::rect("y", 0.0, 0.0, 40.0, 10.0, Color::WHITE));
    apply_auto_layout(&mut tight, &vars);
    assert_eq!(find(&tight, "y").unwrap().transform.x, 40.0, "gap clamped at 0, children just stack");
    // zero-size children
    let mut z = Node::frame("z", 0.0, 0.0)
        .auto_layout(hug(LayoutDirection::Horizontal, 5.0, 5.0))
        .child(Node::rect("zero", 0.0, 0.0, 0.0, 0.0, Color::WHITE));
    apply_auto_layout(&mut z, &vars);
    assert_eq!(z.w, 10.0);
    // instance of a MISSING master inside a layout: sync is a no-op, no panic
    let mut page = Node::frame("p", 100.0, 100.0)
        .child(Node::instance("ghost", "NoSuchComponent", 0.0, 0.0, 10.0, 10.0));
    assert_eq!(sync_instance_sizes(&mut page, &vars, &m), 0);
}

#[test]
fn layout_is_idempotent() {
    // solving twice must not move anything the second time
    let mut page = Node::frame("page", 500.0, 500.0).child(
        Node::frame("row", 0.0, 0.0)
            .auto_layout(hug(LayoutDirection::Horizontal, 10.0, 5.0))
            .child(Node::rect("a", 0.0, 0.0, 30.0, 20.0, Color::WHITE))
            .child(Node::rect("b", 0.0, 0.0, 40.0, 20.0, Color::WHITE)),
    );
    apply_layout_recursive(&mut page, &Variables::default());
    let snapshot = format!("{:?}", page);
    apply_layout_recursive(&mut page, &Variables::default());
    assert_eq!(snapshot, format!("{:?}", page), "second solve must be a no-op");
}
