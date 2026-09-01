use std::collections::HashMap;
use vello::kurbo::{Affine, Point, Rect};
use vello::peniko::Color;
use x_core::*;
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use x_core::{Color, Node};

    fn doc() -> Node {
        Node::frame("page", 800.0, 600.0)
            .child(Node::rect("a", 10.0, 10.0, 100.0, 50.0, Color::rgb8(255, 0, 0)))
            .child(Node::rect("b", 200.0, 10.0, 100.0, 50.0, Color::rgb8(0, 255, 0)))
            .child(Node::ellipse("c", 400.0, 10.0, 80.0, 80.0, Color::rgb8(0, 0, 255)))
    }

    #[test]
    fn hit_test_finds_topmost() {
        let d = Node::frame("page", 800.0, 600.0)
            .child(Node::rect("under", 0.0, 0.0, 100.0, 100.0, Color::WHITE))
            .child(Node::rect("over", 50.0, 50.0, 100.0, 100.0, Color::WHITE));
        assert_eq!(hit_test(&d, Point::new(75.0, 75.0)), Some("over".into()));
        assert_eq!(hit_test(&d, Point::new(25.0, 25.0)), Some("under".into()));
        assert_eq!(hit_test(&d, Point::new(500.0, 500.0)), None);
    }

    #[test]
    fn hit_test_respects_ellipse_shape_and_lock() {
        let mut d = doc();
        // corner of the ellipse's AABB is OUTSIDE the ellipse
        assert_eq!(hit_test(&d, Point::new(402.0, 12.0)), None);
        // center is inside
        assert_eq!(hit_test(&d, Point::new(440.0, 50.0)), Some("c".into()));
        find_mut(&mut d, "c").unwrap().locked = true;
        assert_eq!(hit_test(&d, Point::new(440.0, 50.0)), None);
    }

    #[test]
    fn hit_test_respects_rotation() {
        let d = Node::frame("page", 400.0, 400.0)
            .child(Node::rect("r", 100.0, 100.0, 100.0, 20.0, Color::WHITE).rotate(std::f64::consts::FRAC_PI_2));
        // rotated 90° about center (150,110): occupies x∈[140,160], y∈[60,160]
        assert_eq!(hit_test(&d, Point::new(150.0, 70.0)), Some("r".into()));
        assert_eq!(hit_test(&d, Point::new(105.0, 110.0)), None); // original spot now empty
    }

    #[test]
    fn marquee_selects_intersecting() {
        let mut e = Editor::new(doc());
        e.marquee(Rect::new(0.0, 0.0, 320.0, 100.0));
        assert_eq!(e.selection, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn move_undo_redo_roundtrip() {
        let mut e = Editor::new(doc());
        e.selection = vec!["a".into()];
        e.move_selection(30.0, 40.0);
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 40.0);
        assert!(e.undo());
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 10.0);
        assert!(e.redo());
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 40.0);
        assert!(!e.redo()); // stack empty
    }

    #[test]
    fn resize_rotate_fill_text_are_undoable() {
        let mut e = Editor::new(doc().child(Node::text("t", 0.0, 200.0, 100.0, 20.0, "OLD")));
        e.resize("a", 150.0, 75.0);
        e.rotate("a", 0.5);
        e.set_fill("a", Paint::Solid(Color::rgb8(1, 2, 3)));
        e.set_text("t", "NEW");
        assert_eq!(find(&e.root, "a").unwrap().w, 150.0);
        assert!(matches!(&find(&e.root, "t").unwrap().kind, NodeKind::Text{text} if text=="NEW"));
        e.undo(); e.undo(); e.undo(); e.undo();
        let a = find(&e.root, "a").unwrap();
        assert_eq!((a.w, a.transform.rotation), (100.0, 0.0));
        assert!(matches!(&a.fill, Paint::Solid(c) if c.r==255));
        assert!(matches!(&find(&e.root, "t").unwrap().kind, NodeKind::Text{text} if text=="OLD"));
    }

    #[test]
    fn delete_and_undo_restores_at_same_index() {
        let mut e = Editor::new(doc());
        e.selection = vec!["b".into()];
        e.delete_selection();
        assert!(find(&e.root, "b").is_none());
        e.undo();
        assert_eq!(e.root.children[1].id, "b"); // back at index 1, not appended
    }

    #[test]
    fn z_order_ops() {
        let mut e = Editor::new(doc());
        e.bring_to_front("a");
        assert_eq!(e.root.children.last().unwrap().id, "a");
        e.send_to_back("a");
        assert_eq!(e.root.children[0].id, "a");
        e.undo(); // back to front
        assert_eq!(e.root.children.last().unwrap().id, "a");
    }

    #[test]
    fn group_and_undo() {
        let mut e = Editor::new(doc());
        e.selection = vec!["a".into(), "b".into()];
        e.group_selection("g1");
        let g = find(&e.root, "g1").expect("group exists");
        assert_eq!(g.children.len(), 2);
        assert_eq!(g.transform.x, 10.0); // group wraps collective bounds
        assert_eq!(g.children[0].transform.x, 0.0); // members re-based
        assert!(e.undo());
        assert!(find(&e.root, "g1").is_none());
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 10.0);
    }

    #[test]
    fn align_and_distribute() {
        let mut d = doc();
        let ids = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        align(&mut d, &ids, AlignKind::Top);
        assert!(d.children.iter().all(|c| c.transform.y == 10.0));
        // spread them out then distribute
        d.children[0].transform.x = 0.0;
        d.children[1].transform.x = 50.0;
        d.children[2].transform.x = 400.0;
        distribute_horizontal(&mut d, &ids);
        let xs: Vec<f64> = d.children.iter().map(|c| c.transform.x).collect();
        let gap1 = xs[1] - (xs[0] + d.children[0].w);
        let gap2 = xs[2] - (xs[1] + d.children[1].w);
        assert!((gap1 - gap2).abs() < 1e-9);
    }

    #[test]
    fn snapping_to_edges_and_grid() {
        let s = Snapper { grid: 8.0, threshold: 6.0 };
        let others = vec![("b".to_string(), Rect::new(200.0, 0.0, 300.0, 50.0))];
        // proposed left edge 204, within 6 of b's left edge 200 -> snap to 200
        let (x, hit) = s.snap_x(204.0, 100.0, &others);
        assert_eq!(x, 200.0);
        assert_eq!(hit, Some("b".into()));
        // far from everything -> falls back to 8px grid
        let (x, hit) = s.snap_x(701.0, 100.0, &[]);
        assert_eq!(x, 704.0);
        assert!(hit.is_none());
    }

    #[test]
    fn constraints_solver() {
        let mut f = Node::frame("f", 400.0, 300.0)
            .child(Node::rect("right", 300.0, 10.0, 80.0, 40.0, Color::WHITE).pin(x_core::HPin::Right, x_core::VPin::Top))
            .child(Node::rect("stretch", 10.0, 10.0, 380.0, 40.0, Color::WHITE).pin(x_core::HPin::StretchH, x_core::VPin::Top))
            .child(Node::rect("center", 150.0, 100.0, 100.0, 40.0, Color::WHITE).pin(x_core::HPin::CenterH, x_core::VPin::CenterV));
        let (ow, oh) = (f.w, f.h);
        f.w = 600.0; f.h = 400.0;
        apply_constraints(&mut f, ow, oh);
        assert_eq!(find(&f, "right").unwrap().transform.x, 500.0);   // +200
        assert_eq!(find(&f, "stretch").unwrap().w, 580.0);           // +200
        assert_eq!(find(&f, "center").unwrap().transform.x, 250.0);  // +100
        assert_eq!(find(&f, "center").unwrap().transform.y, 150.0);  // +50
    }

    #[test]
    fn prototype_player_navigates_and_goes_back() {
        let doc = Node::frame("doc", 2000.0, 800.0)
            .child(Node::frame("screen-1", 400.0, 800.0)
                .child(Node::rect("cta", 100.0, 700.0, 200.0, 60.0, Color::WHITE).prototype("screen-2", 300)))
            .child(Node::frame("screen-2", 400.0, 800.0)
                .child(Node::rect("back-btn", 10.0, 10.0, 60.0, 40.0, Color::WHITE)));
        let mut p = Player::new(&doc, "screen-1");
        let ms = p.click(Point::new(200.0, 730.0));
        assert_eq!(ms, Some(300));
        assert_eq!(p.current, "screen-2");
        assert!(p.back());
        assert_eq!(p.current, "screen-1");
        assert!(!p.back());
    }

    #[test]
    fn spatial_grid_indexes_100k_and_queries_fast() {
        let scene = x_render::benchmark_scene(100_000);
        let t0 = std::time::Instant::now();
        let grid = SpatialGrid::build(&scene, 256.0);
        let build_ms = t0.elapsed().as_millis();
        assert_eq!(grid.len(), 100_000);
        let t1 = std::time::Instant::now();
        let mut total = 0usize;
        for i in 0..1000 { total += grid.query_point(Point::new((i * 4) as f64, (i * 4) as f64)).len(); }
        let query_us = t1.elapsed().as_micros();
        assert!(total > 0);
        // generous sandbox bounds; on real hardware these are far lower
        assert!(build_ms < 5000, "grid build too slow: {build_ms}ms");
        assert!(query_us < 2_000_000, "1000 queries too slow: {query_us}us");
    }

    #[test]
    fn merge_last_collapses_a_drag_gesture() {
        let mut e = Editor::new(doc());
        e.selection = vec!["a".into()];
        let before = e.undo_depth();
        e.move_selection(5.0, 0.0);
        e.move_selection(5.0, 0.0);
        e.move_selection(5.0, 0.0);
        e.merge_last(e.undo_depth() - before);
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 25.0);
        e.undo(); // ONE undo reverts the whole gesture
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 10.0);
    }

    #[test]
    fn insert_node_is_undoable() {
        let mut e = Editor::new(doc());
        assert!(e.insert_node("page", Node::rect("new", 5.0, 5.0, 10.0, 10.0, Color::WHITE)));
        assert!(find(&e.root, "new").is_some());
        e.undo();
        assert!(find(&e.root, "new").is_none());
    }

    #[test]
    fn copy_paste_remaps_ids_and_is_undoable() {
        let mut e = Editor::new(doc());
        e.selection = vec!["a".into()];
        e.copy();
        let ids = e.paste("page", (20.0, 20.0));
        assert_eq!(ids, vec!["a-copy".to_string()]);
        let copy = find(&e.root, "a-copy").unwrap();
        assert_eq!(copy.transform.x, 30.0); // 10 + 20 offset
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 10.0); // original untouched
        // second paste gets a distinct id
        let ids2 = e.paste("page", (40.0, 40.0));
        assert_eq!(ids2, vec!["a-copy-2".to_string()]);
        e.undo();
        assert!(find(&e.root, "a-copy-2").is_none());
        assert!(find(&e.root, "a-copy").is_some());
    }

    #[test]
    fn duplicate_selects_the_new_copies() {
        let mut e = Editor::new(doc());
        e.selection = vec!["b".into()];
        let ids = e.duplicate_selection((10.0, 10.0));
        assert_eq!(e.selection, ids);
        assert!(find(&e.root, "b-copy").is_some());
    }

    #[test]
    fn paste_remaps_nested_child_ids_too() {
        let mut e = Editor::new(
            Node::frame("page", 800.0, 600.0)
                .child(Node::group("g", 100.0, 100.0).child(Node::rect("inner", 0.0, 0.0, 50.0, 50.0, Color::WHITE))),
        );
        e.selection = vec!["g".into()];
        e.copy();
        e.paste("page", (0.0, 0.0));
        let pasted = find(&e.root, "g-copy").unwrap();
        assert_eq!(pasted.children[0].id, "inner-copy"); // child renamed, no dup ids
    }

    #[test]
    fn smart_animate_interpolates_matching_ids() {
        let from = Node::frame("s1", 400.0, 400.0)
            .child(Node::rect("box", 0.0, 0.0, 100.0, 100.0, Color::rgb8(255, 0, 0)))
            .child(Node::rect("leaving", 300.0, 300.0, 50.0, 50.0, Color::WHITE));
        let to = Node::frame("s2", 400.0, 400.0)
            .child(Node::rect("box", 200.0, 100.0, 200.0, 100.0, Color::rgb8(0, 0, 255)))
            .child(Node::rect("entering", 0.0, 300.0, 50.0, 50.0, Color::WHITE));
        let mid = smart_animate(&from, &to, 0.5);
        let boxn = find(&mid, "box").unwrap();
        assert_eq!(boxn.transform.x, 100.0); // (0+200)/2
        assert_eq!(boxn.transform.y, 50.0);
        assert_eq!(boxn.w, 150.0);
        if let Paint::Solid(c) = &boxn.fill {
            assert_eq!((c.r, c.b), (128, 128)); // red->blue midpoint
        } else { panic!("expected solid fill") }
        // entering fades in, leaving fades out
        assert!((find(&mid, "entering").unwrap().opacity - 0.5).abs() < 1e-6);
        assert!((find(&mid, "leaving").unwrap().opacity - 0.5).abs() < 1e-6);
        // endpoints match the destinations exactly
        let end = smart_animate(&from, &to, 1.0);
        assert_eq!(find(&end, "box").unwrap().transform.x, 200.0);
        assert_eq!(find(&end, "leaving").unwrap().opacity, 0.0);
    }

    #[test]
    fn smart_animate_frames_are_renderable() {
        let from = Node::frame("s1", 400.0, 400.0).child(Node::rect("box", 0.0, 0.0, 100.0, 100.0, Color::WHITE));
        let to = Node::frame("s2", 400.0, 400.0).child(Node::rect("box", 200.0, 200.0, 100.0, 100.0, Color::WHITE));
        let mid = smart_animate(&from, &to, 0.25);
        let (_, s) = x_render::build_scene(&mid, None, &Variables::default());
        assert_eq!(s.paths, 1);
    }

    #[test]
    fn set_opacity_is_undoable() {
        let mut e = Editor::new(doc());
        e.set_opacity("a", 0.4);
        assert!((find(&e.root, "a").unwrap().opacity - 0.4).abs() < 1e-6);
        e.undo();
        assert_eq!(find(&e.root, "a").unwrap().opacity, 1.0);
    }

    #[test]
    fn alignment_guides_detect_edges_and_centers() {
        // b: x∈[200,300] y∈[10,60]; move a to share b's top edge and left edge
        let mut d = doc();
        find_mut(&mut d, "a").unwrap().transform.x = 200.0; // left edges align
        let g = alignment_guides(&d, "a", 1.0);
        assert!(g.contains(&(true, 200.0)), "left-edge guide missing: {g:?}");
        assert!(g.contains(&(false, 10.0)), "top-edge guide missing: {g:?}");
        // center alignment: move a so its center-x hits b's center-x (250)
        find_mut(&mut d, "a").unwrap().transform.x = 200.0; // a: [200,300] center 250 == b center
        let g = alignment_guides(&d, "a", 1.0);
        assert!(g.contains(&(true, 250.0)), "center guide missing: {g:?}");
        // far away -> no guides
        find_mut(&mut d, "a").unwrap().transform.x = 500.0;
        find_mut(&mut d, "a").unwrap().transform.y = 400.0;
        let g = alignment_guides(&d, "a", 1.0);
        assert!(g.is_empty(), "expected no guides, got {g:?}");
    }

    #[test]
    fn set_auto_layout_solves_and_undoes_atomically() {
        use x_core::{AutoLayout, LayoutDirection, Sizing};
        let mut e = Editor::new(
            Node::frame("page", 800.0, 600.0).child(
                Node::frame("f", 400.0, 200.0)
                    .child(Node::rect("a", 300.0, 90.0, 50.0, 40.0, Color::WHITE))
                    .child(Node::rect("b", 20.0, 15.0, 70.0, 40.0, Color::WHITE)),
            ),
        );
        let vars = Variables::default();
        assert!(e.set_auto_layout("f", Some(AutoLayout {
            direction: LayoutDirection::Horizontal, gap: 10.0, padding: 8.0,
            sizing: Sizing::Fixed, ..Default::default()
        }), &vars));
        // children re-stacked: a at padding, b after a + gap
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 8.0);
        assert_eq!(find(&e.root, "b").unwrap().transform.x, 68.0); // 8+50+10
        assert!(e.auto_layout_of("f").is_some());
        // ONE undo restores the scattered originals AND removes the layout
        assert!(e.undo());
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 300.0);
        assert_eq!(find(&e.root, "b").unwrap().transform.x, 20.0);
        assert!(e.auto_layout_of("f").is_none());
        // redo brings it back
        assert!(e.redo());
        assert_eq!(find(&e.root, "b").unwrap().transform.x, 68.0);
        // clearing layout keeps positions but removes the layout config
        assert!(e.set_auto_layout("f", None, &vars));
        assert!(e.auto_layout_of("f").is_none());
        assert_eq!(find(&e.root, "b").unwrap().transform.x, 68.0);
        // non-frames are rejected
        assert!(!e.set_auto_layout("a", None, &vars));
    }

    #[test]
    fn make_component_and_place_instances() {
        let mut e = Editor::new(doc());
        e.selection = vec!["a".into(), "b".into()];
        assert!(e.make_component("Card"));
        // selection is now the replacing instance
        assert_eq!(e.selection, vec!["Card-1".to_string()]);
        let inst = find(&e.root, "Card-1").unwrap();
        assert!(matches!(&inst.kind, NodeKind::Instance { component } if component == "Card"));
        assert_eq!(inst.transform.x, 10.0); // collective origin of a+b
        assert_eq!(inst.w, 290.0);          // spans a(10..110) to b(200..300)
        // master exists, hidden, with re-based members
        let master = find(&e.root, "comp-Card").unwrap();
        assert!(!master.visible);
        assert_eq!(master.children.len(), 2);
        assert_eq!(master.children[0].transform.x, 0.0);
        // originals moved off the page INTO the master (still findable there)
        assert!(!e.root.children.iter().any(|c| c.id == "a"));
        assert!(master.children.iter().any(|c| c.id == "a"));
        // rendering resolves the instance -> master children paths
        let (_, s) = x_render::build_scene(&e.root, None, &Variables::default());
        assert_eq!(s.paths, 3); // c (ellipse) + 2 resolved members
        // stamp two more instances
        let id2 = e.place_instance("Card", 400.0, 300.0).unwrap();
        assert_eq!(id2, "Card-2");
        let (_, s) = x_render::build_scene(&e.root, None, &Variables::default());
        assert_eq!(s.paths, 5);
        // editing the MASTER's child updates every instance render
        assert_eq!(e.component_names(), vec!["Card".to_string()]);
        // undo the placement, then undo the componentization entirely
        e.undo();
        assert!(find(&e.root, "Card-2").is_none());
        e.undo();
        assert!(find(&e.root, "a").is_some());
        assert!(find(&e.root, "comp-Card").is_none());
    }

    #[test]
    fn scale_tool_scales_subtree_uniformly() {
        let mut e = Editor::new(
            Node::frame("page", 800.0, 600.0).child(
                Node::frame("f", 200.0, 100.0)
                    .child(Node::rect("r", 20.0, 10.0, 50.0, 30.0, Color::WHITE).radius(8.0)),
            ),
        );
        assert!(e.scale_node("f", 2.0));
        let f = find(&e.root, "f").unwrap();
        assert_eq!((f.w, f.h), (400.0, 200.0));
        let r = find(&e.root, "r").unwrap();
        assert_eq!((r.transform.x, r.transform.y), (40.0, 20.0)); // offsets scaled
        assert_eq!((r.w, r.h), (100.0, 60.0));
        assert!(matches!(r.kind, NodeKind::Rect { radius } if radius == 16.0)); // radius scaled
        e.undo();
        assert_eq!(find(&e.root, "f").unwrap().w, 200.0);
        assert_eq!(find(&e.root, "r").unwrap().w, 50.0);
        // zero/negative factor rejected
        assert!(!e.scale_node("f", 0.0));
    }

    #[test]
    fn set_prototype_is_undoable() {
        let mut e = Editor::new(doc());
        e.set_prototype("a", Some(x_core::PrototypeAction { destination: "page-2".into(), transition_ms: 300 }));
        assert_eq!(find(&e.root, "a").unwrap().prototype.as_ref().unwrap().destination, "page-2");
        e.undo();
        assert!(find(&e.root, "a").unwrap().prototype.is_none());
        e.redo();
        // clearing works too
        e.set_prototype("a", None);
        assert!(find(&e.root, "a").unwrap().prototype.is_none());
    }

    #[test]
    fn figma_click_selects_top_level_then_drills() {
        // page > group g > rect inner
        let d = Node::frame("page", 800.0, 600.0).child(
            Node::group("g", 200.0, 200.0)
                .child(Node::rect("inner", 10.0, 10.0, 100.0, 100.0, Color::WHITE)),
        );
        let mut e = Editor::new(d);
        // plain click on inner selects TOP-LEVEL group (standard behavior)
        e.click_select(Point::new(50.0, 50.0), false, false);
        assert_eq!(e.selection, vec!["g".to_string()]);
        // deep click (ctrl) selects the exact node
        e.click_select(Point::new(50.0, 50.0), false, true);
        assert_eq!(e.selection, vec!["inner".to_string()]);
        // drill: from g, double-click goes one level down
        e.selection = vec!["g".into()];
        let next = e.drill_into(Point::new(50.0, 50.0));
        assert_eq!(next.as_deref(), Some("inner"));
    }

    #[test]
    fn ungroup_dissolves_and_preserves_positions() {
        let d = Node::frame("page", 800.0, 600.0).child({
            let mut g = Node::group("g", 200.0, 100.0)
                .child(Node::rect("a", 5.0, 6.0, 50.0, 40.0, Color::WHITE))
                .child(Node::rect("b", 60.0, 6.0, 50.0, 40.0, Color::WHITE));
            g.transform.x = 100.0; g.transform.y = 50.0; g
        });
        let mut e = Editor::new(d);
        assert!(e.ungroup("g"));
        assert!(find(&e.root, "g").is_none());
        let a = find(&e.root, "a").unwrap();
        assert_eq!((a.transform.x, a.transform.y), (105.0, 56.0)); // world position preserved
        assert_eq!(e.selection, vec!["a".to_string(), "b".to_string()]);
        // snapshot-undo restores the group
        assert!(e.undo());
        assert!(find(&e.root, "g").is_some());
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 5.0);
    }

    #[test]
    fn select_all_scopes_to_selected_frame() {
        let d = Node::frame("page", 800.0, 600.0)
            .child(Node::rect("x", 0.0, 0.0, 10.0, 10.0, Color::WHITE))
            .child(Node::frame("f", 200.0, 200.0)
                .child(Node::rect("c1", 0.0, 0.0, 10.0, 10.0, Color::WHITE))
                .child(Node::rect("c2", 20.0, 0.0, 10.0, 10.0, Color::WHITE)));
        let mut e = Editor::new(d);
        e.select_all();
        assert_eq!(e.selection.len(), 2); // x and f
        // with frame f selected, select-all scopes inside it
        e.selection = vec!["f".into()];
        e.select_all();
        assert_eq!(e.selection, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[test]
    fn snap_delta_pulls_to_nearby_edges() {
        let mut d = doc();
        // a: x∈[10,110]; b: x∈[200,300]. Move a so its right edge is 3px from b's left.
        find_mut(&mut d, "a").unwrap().transform.x = 97.0; // right edge 197, b.x0=200, d=3
        let (dx, _) = snap_delta(&d, "a", 6.0);
        assert_eq!(dx, 3.0); // pulled right to touch exactly
        // vertical: a.y=10 already aligns with b.y=10 -> dy 0 (already snapped)
        let (_, dy) = snap_delta(&d, "a", 6.0);
        assert_eq!(dy, 0.0);
        // far away -> no snap
        find_mut(&mut d, "a").unwrap().transform.x = 400.0;
        find_mut(&mut d, "a").unwrap().transform.y = 400.0;
        assert_eq!(snap_delta(&d, "a", 6.0), (0.0, 0.0));
    }

    #[test]
    fn set_pin_is_undoable() {
        let mut e = Editor::new(doc());
        e.set_pin("a", x_core::HPin::Right, x_core::VPin::Bottom);
        assert_eq!(find(&e.root, "a").unwrap().pin, (x_core::HPin::Right, x_core::VPin::Bottom));
        e.undo();
        assert_eq!(find(&e.root, "a").unwrap().pin, (x_core::HPin::Left, x_core::VPin::Top));
    }

    #[test]
    fn detach_is_undoable_and_applies_overrides() {
        use x_components::{set_override, OverrideValue};
        let mut comp = Node::component("c", "Chip", 60.0, 24.0);
        comp.visible = false;
        comp.children.push(Node::rect("chip-bg", 0.0, 0.0, 60.0, 24.0, Color::BLACK));
        let mut inst = Node::instance("i1", "Chip", 100.0, 100.0, 60.0, 24.0);
        set_override(&mut inst, "chip-bg", OverrideValue::Fill(Color::rgb8(0, 0xff, 0)));
        let mut e = Editor::new(Node::frame("page", 400.0, 300.0).child(comp).child(inst));
        assert!(e.detach_selected_instance(&Variables::default()) == false); // nothing selected
        e.selection = vec!["i1".into()];
        assert!(e.detach_selected_instance(&Variables::default()));
        assert!(find(&e.root, "i1").is_none());
        let g = find(&e.root, "i1-detached").unwrap();
        assert_eq!(g.transform.x, 100.0);
        let bg = g.children.iter().find(|c| c.id == "chip-bg").unwrap();
        assert!(matches!(&bg.fill, Paint::Solid(c) if c.g == 0xff));
        e.undo();
        assert!(find(&e.root, "i1").is_some());
        assert!(find(&e.root, "i1-detached").is_none());
    }

    #[test]
    fn swap_instance_is_undoable() {
        let mut e = Editor::new(
            Node::frame("page", 400.0, 300.0)
                .child(Node::instance("i", "Button/Primary", 0.0, 0.0, 100.0, 40.0)),
        );
        assert!(e.swap_instance("i", "Button/Danger"));
        assert!(matches!(&find(&e.root, "i").unwrap().kind, NodeKind::Instance { component } if component == "Button/Danger"));
        e.undo();
        assert!(matches!(&find(&e.root, "i").unwrap().kind, NodeKind::Instance { component } if component == "Button/Primary"));
    }

    #[test]
    fn checkpoints_restore() {
        let mut e = Editor::new(doc());
        e.checkpoint("v1");
        e.selection = vec!["a".into()];
        e.move_selection(500.0, 0.0);
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 510.0);
        assert!(e.restore_checkpoint("v1"));
        assert_eq!(find(&e.root, "a").unwrap().transform.x, 10.0);
    }

    #[test]
    fn dev_mode_css_export() {
        let n = Node::rect("card", 0.0, 0.0, 240.0, 120.0, Color::rgb8(0x0d, 0x99, 0xff))
            .radius(16.0).opacity(0.9)
            .effect(x_core::Effect::DropShadow { dx: 0.0, dy: 4.0, blur: 12.0, color: Color::rgba8(0, 0, 0, 128) });
        let css = node_to_css(&n, &Variables::default());
        assert!(css.contains("width: 240px"));
        assert!(css.contains("background: #0d99ff"));
        assert!(css.contains("border-radius: 16px"));
        assert!(css.contains("box-shadow: 0px 4px 12px"));
    }
}

