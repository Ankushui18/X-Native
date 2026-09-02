use std::collections::HashMap;
use vello::peniko::Color;
use x_core::*;
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn feature_model_has_expected_nodes() {
        let d = Node::frame("r", 500.0, 500.0)
            .child(Node::text("t", 0.0, 0.0, 100.0, 20.0, "hello"))
            .child(Node::image("i", 0.0, 30.0, 50.0, 50.0, "asset-1"))
            .child(Node::component("c", "Button", 100.0, 40.0))
            .child(Node::instance("x", "Button", 0.0, 90.0, 100.0, 40.0));
        assert_eq!(d.children.len(), 4)
    }

    #[test]
    fn auto_layout_positions_children() {
        let mut d = Node::frame("r", 100.0, 100.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Horizontal, gap: 10.0, padding: [5.0; 4], sizing: Sizing::Fixed, ..Default::default() })
            .child(Node::rect("a", 0.0, 0.0, 20.0, 20.0, Color::WHITE))
            .child(Node::rect("b", 0.0, 0.0, 30.0, 20.0, Color::WHITE));
        apply_auto_layout(&mut d, &Variables::default());
        assert_eq!(d.children[0].transform.x, 5.0);
        assert_eq!(d.children[1].transform.x, 35.0)
    }

    #[test]
    fn viewport_culls_offscreen_nodes() {
        let d = Node::frame("r", 1000.0, 1000.0)
            .child(Node::rect("on", 10.0, 10.0, 20.0, 20.0, Color::WHITE))
            .child(Node::rect("off", 900.0, 900.0, 20.0, 20.0, Color::WHITE));
        let (_, s) = build_scene(&d, Some(Viewport { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }), &Variables::default());
        assert_eq!(s.paths, 1);
        assert_eq!(s.culled, 1)
    }

    #[test]
    fn rotation_and_radius_render() {
        let d = Node::rect("a", 0.0, 0.0, 40.0, 20.0, Color::WHITE).radius(8.0).rotate(1.0).opacity(0.5);
        let (scene, s) = build_scene(&d, None, &Variables::default());
        assert_eq!(s.paths, 1);
        assert_eq!(scene.encoding().n_paths, 1)
    }

    #[test]
    fn stress_10k() {
        let (_, s) = build_scene(&benchmark_scene(10_000), None, &Variables::default());
        assert_eq!(s.nodes, 10_001);
        assert_eq!(s.paths, 10_000)
    }

    #[test]
    fn number_variable_resolves_into_layout_gap() {
        let mut vars = Variables::default();
        vars.numbers.insert("gap".into(), 40.0);
        let mut d = Node::frame("r", 400.0, 100.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Horizontal, gap: 10.0, padding: [0.0; 4], gap_var: Some("gap".into()), ..Default::default() })
            .child(Node::rect("a", 0.0, 0.0, 20.0, 20.0, Color::WHITE))
            .child(Node::rect("b", 0.0, 0.0, 20.0, 20.0, Color::WHITE));
        apply_auto_layout(&mut d, &vars);
        assert_eq!(d.children[1].transform.x, 60.0); // 20 + 40, not 20 + 10
    }

    #[test]
    fn number_variable_missing_falls_back_to_literal() {
        let mut d = Node::frame("r", 400.0, 100.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Horizontal, gap: 10.0, padding: [0.0; 4], gap_var: Some("missing".into()), ..Default::default() })
            .child(Node::rect("a", 0.0, 0.0, 20.0, 20.0, Color::WHITE))
            .child(Node::rect("b", 0.0, 0.0, 20.0, 20.0, Color::WHITE));
        apply_auto_layout(&mut d, &Variables::default());
        assert_eq!(d.children[1].transform.x, 30.0);
    }

    #[test]
    fn instance_resolves_component_children_and_renders_them() {
        let mut master = Node::component("def", "Button", 100.0, 40.0)
            .child(Node::rect("bg", 0.0, 0.0, 100.0, 40.0, Color::BLACK));
        master.visible = false;
        let d = Node::frame("r", 500.0, 500.0)
            .child(master)
            .child(Node::instance("i1", "Button", 10.0, 10.0, 100.0, 40.0));
        let (_, s) = build_scene(&d, None, &Variables::default());
        assert_eq!(s.paths, 1); // the instance's resolved bg, master hidden
    }

    #[test]
    fn instance_override_changes_resolved_child_fill() {
        let bg = Node::rect("bg", 0.0, 0.0, 100.0, 40.0, Color::BLACK);
        let mut ovr = HashMap::new();
        ovr.insert("bg".to_string(), "#ff0000".to_string());
        let c = effective_fill(&bg, &ovr, &Variables::default());
        let rgba = c.to_rgba8();
        assert_eq!((rgba.r, rgba.g, rgba.b), (255, 0, 0));
    }

    #[test]
    fn self_referencing_instance_does_not_infinite_loop() {
        let master = Node::component("def", "Evil", 50.0, 50.0)
            .child(Node::instance("self", "Evil", 0.0, 0.0, 50.0, 50.0));
        let d = Node::frame("r", 500.0, 500.0)
            .child(master)
            .child(Node::instance("i", "Evil", 0.0, 0.0, 50.0, 50.0));
        let (_, s) = build_scene(&d, None, &Variables::default());
        assert!(s.nodes > 0); // terminated
    }

    // ---- v0.4 additions ----

    #[test]
    fn text_renders_real_paths() {
        let d = Node::text("t", 0.0, 0.0, 200.0, 24.0, "HELLO 123");
        let (scene, s) = build_scene(&d, None, &Variables::default());
        // "HELLO 123" = 8 visible glyphs (space is free) = 8 stroke paths
        assert_eq!(s.paths, 8);
        assert_eq!(scene.encoding().n_paths, 8);
    }

    #[test]
    fn text_override_replaces_content() {
        let label = Node::text("label", 0.0, 0.0, 100.0, 20.0, "OLD");
        let mut ovr = HashMap::new();
        ovr.insert("label".to_string(), "text:NEW".to_string());
        assert_eq!(effective_text(&label, &ovr), Some("NEW"));
    }

    #[test]
    fn gradient_paint_encodes() {
        let d = Node::rect("g", 0.0, 0.0, 100.0, 100.0, Color::WHITE)
            .fill_paint(Paint::LinearGradient { start: (0.0, 0.0), end: (100.0, 0.0), stops: vec![(0.0, Color::from_rgb8(255, 0, 0)), (1.0, Color::from_rgb8(0, 0, 255))] });
        let (scene, s) = build_scene(&d, None, &Variables::default());
        assert_eq!(s.paths, 1);
        assert_eq!(scene.encoding().n_paths, 1);
    }

    #[test]
    fn drop_shadow_adds_a_path() {
        let d = Node::rect("s", 0.0, 0.0, 100.0, 50.0, Color::WHITE)
            .effect(Effect::DropShadow { dx: 4.0, dy: 4.0, blur: 8.0, color: Color::BLACK });
        let (_, s) = build_scene(&d, None, &Variables::default());
        assert_eq!(s.paths, 2); // shadow + fill
    }

    #[test]
    fn blend_mode_pushes_layer() {
        let plain = Node::rect("p", 0.0, 0.0, 50.0, 50.0, Color::WHITE);
        let blended = Node::rect("b", 0.0, 0.0, 50.0, 50.0, Color::WHITE).blend(BlendKind::Multiply);
        let (s1, _) = build_scene(&plain, None, &Variables::default());
        let (s2, _) = build_scene(&blended, None, &Variables::default());
        // The mix layer adds a clip path to the encoding.
        assert!(s2.encoding().n_clips > s1.encoding().n_clips);
    }

    #[test]
    fn vector_node_renders_real_path() {
        let star = Node::vector("v", 0.0, 0.0, 100.0, 100.0, vec![
            PathCmd::MoveTo(50.0, 0.0),
            PathCmd::LineTo(61.0, 35.0),
            PathCmd::LineTo(98.0, 35.0),
            PathCmd::LineTo(68.0, 57.0),
            PathCmd::LineTo(79.0, 91.0),
            PathCmd::LineTo(50.0, 70.0),
            PathCmd::LineTo(21.0, 91.0),
            PathCmd::LineTo(32.0, 57.0),
            PathCmd::LineTo(2.0, 35.0),
            PathCmd::LineTo(39.0, 35.0),
            PathCmd::Close,
        ]);
        let (scene, s) = build_scene(&star, None, &Variables::default());
        assert_eq!(s.paths, 1);
        assert_eq!(scene.encoding().n_paths, 1);
        // empty vector renders nothing (no phantom paths)
        let empty = Node::vector("e", 0.0, 0.0, 10.0, 10.0, vec![]);
        let (_, s2) = build_scene(&empty, None, &Variables::default());
        assert_eq!(s2.paths, 0);
    }

    #[test]
    fn png_asset_decodes_and_renders() {
        // write a tiny 2x2 red PNG, decode via Assets, render via Image node
        let path = std::env::temp_dir().join("xnative_asset_test.png");
        {
            let f = std::fs::File::create(&path).unwrap();
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), 2, 2);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[255,0,0,255, 255,0,0,255, 255,0,0,255, 255,0,0,255]).unwrap();
        }
        let mut assets = Assets::new();
        assets.load_png("logo", path.to_str().unwrap()).expect("decode");
        assert_eq!(assets.len(), 1);
        assert_eq!(assets.get("logo").unwrap().image.width, 2);

        let d = Node::image("img", 0.0, 0.0, 100.0, 100.0, "logo");
        let (_, s) = build_scene_with_assets(&d, None, &Variables::default(), Some(&assets));
        assert_eq!(s.paths, 1);
        // without assets it still renders the placeholder (no panic)
        let (_, s2) = build_scene(&d, None, &Variables::default());
        assert_eq!(s2.paths, 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn per_corner_radii_render() {
        let d = Node::rect("c", 0.0, 0.0, 80.0, 40.0, Color::WHITE).corners(0.0, 20.0, 0.0, 20.0);
        let (_, s) = build_scene(&d, None, &Variables::default());
        assert_eq!(s.paths, 1);
    }

    #[test]
    fn layout_v2_cross_axis_center_and_space_between() {
        let mut d = Node::frame("r", 400.0, 100.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Horizontal, padding: [0.0; 4], align: CrossAlign::Center, space_between: true, ..Default::default() })
            .child(Node::rect("a", 0.0, 0.0, 50.0, 40.0, Color::WHITE))
            .child(Node::rect("b", 0.0, 0.0, 50.0, 60.0, Color::WHITE));
        apply_auto_layout(&mut d, &Variables::default());
        assert_eq!(d.children[0].transform.y, 30.0); // (100-40)/2
        assert_eq!(d.children[1].transform.y, 20.0); // (100-60)/2
        assert_eq!(d.children[1].transform.x, 350.0); // pushed to far edge
    }

    #[test]
    fn layout_v2_recursive_hug_propagates() {
        let inner = Node::frame("inner", 0.0, 0.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Vertical, gap: 10.0, padding: [5.0; 4], sizing: Sizing::Hug, ..Default::default() })
            .child(Node::rect("a", 0.0, 0.0, 30.0, 20.0, Color::WHITE))
            .child(Node::rect("b", 0.0, 0.0, 30.0, 20.0, Color::WHITE));
        let mut outer = Node::frame("outer", 0.0, 0.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Horizontal, gap: 0.0, padding: [0.0; 4], sizing: Sizing::Hug, ..Default::default() })
            .child(inner);
        apply_layout_recursive(&mut outer, &Variables::default());
        // inner hugged: h = 5+20+10+20+5 = 60, w = 30+10 = 40
        assert_eq!(outer.children[0].h, 60.0);
        assert_eq!(outer.children[0].w, 40.0);
        // outer hugged around inner
        assert_eq!(outer.w, 40.0);
        assert_eq!(outer.h, 60.0);
    }

    #[test]
    fn variables_v2_modes_and_aliases() {
        let mut vars = Variables::default();
        vars.colors.insert("bg".into(), Color::from_rgb8(255, 255, 255));
        let mut dark = HashMap::new();
        dark.insert("bg".to_string(), Color::from_rgb8(0, 0, 0));
        vars.modes.insert("dark".into(), dark);
        vars.aliases.insert("surface".into(), "bg".into());

        // no mode: alias chases to base value
        assert_eq!(vars.color("surface", Color::TRANSPARENT).to_rgba8().r, 255);
        // dark mode wins over base
        vars.active_mode = Some("dark".into());
        assert_eq!(vars.color("surface", Color::TRANSPARENT).to_rgba8().r, 0);
        // strings + bools exist
        vars.strings.insert("brand".into(), "X Native".into());
        vars.bools.insert("beta".into(), true);
        assert_eq!(vars.string("brand", ""), "X Native");
        assert!(vars.boolean("beta", false));
    }

    #[test]
    fn alias_cycle_terminates() {
        let mut vars = Variables::default();
        vars.aliases.insert("a".into(), "b".into());
        vars.aliases.insert("b".into(), "a".into());
        // must not hang; falls back
        assert_eq!((vars.color("a", Color::from_rgb8(1, 2, 3)).components[0] * 255.0).round() as u8, 1);
    }
}


#[cfg(test)]
mod variable_bindings {
    use super::*;
    

    #[test]
    fn radius_and_opacity_bind_to_number_variables() {
        let mut vars = Variables::default();
        vars.numbers.insert("radius-lg".into(), 20.0);
        vars.numbers.insert("dim".into(), 0.25);
        let d = Node::frame("page", 200.0, 200.0)
            .child(Node::rect("r", 0.0, 0.0, 100.0, 60.0, Color::WHITE)
                .radius(2.0).bind("radius", "radius-lg").bind("opacity", "dim"));
        // renders without panic and produces the path
        let (_, s) = build_scene(&d, None, &vars);
        assert_eq!(s.paths, 1);
        // resolution helpers give bound values
        let n = &d.children[0];
        assert_eq!(n.bound_number("radius", &vars, 2.0), 20.0);
        assert_eq!(n.bound_number("opacity", &vars, 1.0), 0.25);
        // missing variable -> fallback
        assert_eq!(n.bound_number("fontsize", &vars, 16.0), 16.0);
    }
}

#[cfg(test)]
mod component2_render {
    use super::*;
    
    use x_components::{set_override, OverrideValue};

    fn doc_with_masters() -> Node {
        let mut icon_a = Node::component("ca", "Icon/Check", 16.0, 16.0);
        icon_a.visible = false;
        icon_a.children.push(Node::rect("ic-a", 0.0, 0.0, 16.0, 16.0, Color::from_rgb8(0, 0xff, 0)));
        let mut icon_b = Node::component("cb", "Icon/Cross", 16.0, 16.0);
        icon_b.visible = false;
        // cross = TWO rects so swap changes path count
        icon_b.children.push(Node::rect("ic-b1", 0.0, 0.0, 16.0, 4.0, Color::from_rgb8(0xff, 0, 0)));
        icon_b.children.push(Node::rect("ic-b2", 0.0, 6.0, 16.0, 4.0, Color::from_rgb8(0xff, 0, 0)));
        let mut btn = Node::component("cbtn", "Button", 100.0, 40.0);
        btn.visible = false;
        btn.children.push(Node::rect("bg", 0.0, 0.0, 100.0, 40.0, Color::BLACK));
        btn.children.push(Node::instance("slot", "Icon/Check", 4.0, 4.0, 16.0, 16.0));
        Node::frame("page", 800.0, 600.0).child(icon_a).child(icon_b).child(btn)
    }

    #[test]
    fn visibility_override_hides_node_inside_instance() {
        let mut doc = doc_with_masters();
        let mut i = Node::instance("i1", "Button", 200.0, 0.0, 100.0, 40.0);
        let (_, s_before) = { doc.children.push(i.clone()); let r = build_scene(&doc, None, &Variables::default()); doc.children.pop(); r };
        set_override(&mut i, "bg", OverrideValue::Visible(false));
        doc.children.push(i);
        let (_, s_after) = build_scene(&doc, None, &Variables::default());
        assert_eq!(s_after.paths, s_before.paths - 1, "bg should vanish");
    }

    #[test]
    fn swap_override_replaces_nested_component() {
        let mut doc = doc_with_masters();
        let mut i = Node::instance("i1", "Button", 200.0, 0.0, 100.0, 40.0);
        let (_, s_check) = { doc.children.push(i.clone()); let r = build_scene(&doc, None, &Variables::default()); doc.children.pop(); r };
        set_override(&mut i, "slot", OverrideValue::Swap("Icon/Cross".into()));
        doc.children.push(i);
        let (_, s_cross) = build_scene(&doc, None, &Variables::default());
        // Check icon = 1 path; Cross icon = 2 paths
        assert_eq!(s_cross.paths, s_check.paths + 1, "swap to 2-path icon adds a path");
    }
}

#[cfg(test)]
mod typography_integration {
    use super::*;
    

    #[test]
    fn text_node_renders_with_real_font_when_available() {
        let mut fm = x_text::FontManager::new();
        if fm.load_system_fonts() == 0 { return; } // headless env w/o fonts: skip
        let d = Node::text("t", 0.0, 0.0, 400.0, 24.0, "Real Type");
        let (scene, s) = build_scene_full(&d, None, &Variables::default(), None, Some(&fm));
        // "Real Type" = 8 visible glyphs (space skipped as no-outline? no—space HAS no outline) -> >= 8 filled outlines
        assert!(s.paths >= 8, "expected real glyph paths, got {}", s.paths);
        assert!(scene.encoding().n_paths >= 8);
        // segment-font path still works without fonts
        let (_, s2) = build_scene(&d, None, &Variables::default());
        assert!(s2.paths > 0);
    }
}

