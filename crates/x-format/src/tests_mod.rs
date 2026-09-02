use x_core::*;
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use x_editor::find;
    use x_core::{Effect, HPin, VPin};

    fn sample_doc() -> Document {
        let mut doc = Document::new();
        doc.variables.colors.insert("brand".into(), Color::from_rgb8(0x0d, 0x99, 0xff));
        doc.variables.numbers.insert("gap-lg".into(), 28.0);
        let page = Node::frame("page-1", 800.0, 600.0)
            .auto_layout(AutoLayout { direction: LayoutDirection::Horizontal, gap: 20.0, padding: [24.0; 4], align: CrossAlign::Center, space_between: true, gap_var: Some("gap-lg".into()), ..Default::default() })
            .child(
                Node::rect("card", 10.0, 20.0, 240.0, 120.0, Color::from_rgb8(255, 0, 0))
                    .radius(16.0).rotate(0.3).opacity(0.9)
                    .corners(1.0, 2.0, 3.0, 4.0)
                    .blend(BlendKind::Multiply)
                    .effect(Effect::DropShadow { dx: 0.0, dy: 4.0, blur: 12.0, color: Color::from_rgba8(0, 0, 0, 128) })
                    .pin(HPin::Right, VPin::Bottom)
                    .prototype("page-2", 250),
            )
            .child(Node::text("label", 0.0, 0.0, 120.0, 20.0, "Hello \"X\"\nworld"))
            .child(Node::instance("i1", "Button", 0.0, 0.0, 100.0, 40.0).override_prop("bg", "#00ff00").override_prop("label", "text:Buy"))
            .child(Node::rect("grad", 0.0, 0.0, 100.0, 100.0, Color::WHITE).fill_paint(Paint::LinearGradient { start: (0.0, 0.0), end: (100.0, 0.0), stops: vec![(0.0, Color::from_rgb8(255, 0, 0)), (1.0, Color::from_rgb8(0, 0, 255))] }));
        doc.pages.push(page);
        doc.pages.push(Node::frame("page-2", 800.0, 600.0));
        doc
    }

    #[test]
    fn styles_roundtrip_through_x_format() {
        let mut doc = sample_doc();
        doc.styles.insert("Brand/Primary".into(), Style::Paint { fill: Paint::Solid(Color::from_rgb8(0x0d, 0x99, 0xff)) });
        doc.styles.insert("Brand/Grad".into(), Style::Paint { fill: Paint::LinearGradient { start: (0.0, 0.0), end: (100.0, 0.0), stops: vec![(0.0, Color::from_rgb8(255, 90, 0)), (1.0, Color::from_rgb8(142, 45, 226))] } });
        doc.styles.insert("Heading/H1".into(), Style::Text { font: "Inter 700".into(), size: 34.0, letter_spacing: 0.5, line_height: 1.3 });
        doc.styles.insert("Elevation/2".into(), Style::Effect { effects: vec![Effect::DropShadow { dx: 0.0, dy: 4.0, blur: 12.0, color: Color::from_rgba8(0, 0, 0, 128) }, Effect::LayerBlur { radius: 2.0 }] });
        let text = save_x(&doc);
        let loaded = load_x(&text).expect("load styles");
        assert_eq!(loaded.styles.len(), 4);
        assert_eq!(loaded.styles["Brand/Primary"], Style::Paint { fill: Paint::Solid(Color::from_rgb8(0x0d, 0x99, 0xff)) });
        match &loaded.styles["Heading/H1"] {
            Style::Text { font, size, letter_spacing, line_height } => {
                assert_eq!(font, "Inter 700");
                assert_eq!(*size, 34.0);
                assert_eq!(*letter_spacing, 0.5);
                assert_eq!(*line_height, 1.3);
            }
            other => panic!("expected text style, got {other:?}"),
        }
        match &loaded.styles["Elevation/2"] {
            Style::Effect { effects } => assert_eq!(effects.len(), 2),
            other => panic!("expected effect style, got {other:?}"),
        }
        // determinism: save(load(save)) is byte-identical with styles present
        assert_eq!(save_x(&loaded), text);
    }

    #[test]
    fn x_format_roundtrips_everything() {
        let doc = sample_doc();
        let text = save_x(&doc);
        let loaded = load_x(&text).expect("load");
        assert_eq!(loaded.pages.len(), 2);
        assert_eq!(loaded.variables.colors.get("brand").unwrap().to_rgba8().r, 0x0d);
        assert_eq!(*loaded.variables.numbers.get("gap-lg").unwrap(), 28.0);

        let page = &loaded.pages[0];
        if let NodeKind::Frame { layout: Some(l) } = &page.kind {
            assert_eq!(l.direction, LayoutDirection::Horizontal);
            assert_eq!(l.align, CrossAlign::Center);
            assert!(l.space_between);
            assert_eq!(l.gap_var.as_deref(), Some("gap-lg"));
        } else { panic!("layout lost"); }

        let card = find(page, "card").unwrap();
        assert_eq!(card.w, 240.0);
        assert_eq!(card.transform.rotation, 0.3);
        assert_eq!(card.corner_radii, Some([1.0, 2.0, 3.0, 4.0]));
        assert_eq!(card.blend, BlendKind::Multiply);
        assert_eq!(card.effects.len(), 1);
        assert_eq!(card.prototype.as_ref().unwrap().destination, "page-2");
        assert!((card.opacity - 0.9).abs() < 1e-6);

        let label = find(page, "label").unwrap();
        assert!(matches!(&label.kind, NodeKind::Text { text } if text == "Hello \"X\"\nworld"));

        let inst = find(page, "i1").unwrap();
        assert_eq!(inst.overrides.get("bg").map(String::as_str), Some("#00ff00"));
        assert_eq!(inst.overrides.get("label").map(String::as_str), Some("text:Buy"));

        let grad = find(page, "grad").unwrap();
        assert!(matches!(&grad.fill, Paint::LinearGradient { stops, .. } if stops.len() == 2));
    }

    #[test]
    fn text_runs_roundtrip_through_x_format() {
        let mut doc = sample_doc();
        let n = doc.pages[0].children.iter_mut().find(|c| c.id == "label").unwrap();
        n.text_runs = vec![
            TextRun { start: 0, len: 5, color: Some(Color::from_rgb8(255, 0, 0)), size: Some(28.0), font: Some("Inter".into()) },
            TextRun { start: 6, len: 5, color: None, size: Some(14.0), font: None },
        ];
        let a = save_x(&doc);
        assert!(a.contains("\"textRuns\""), "runs serialize");
        let back = load_x(&a).unwrap();
        let n2 = find(&back.pages[0], "label").unwrap();
        assert_eq!(n2.text_runs.len(), 2);
        assert_eq!(n2.text_runs[0].start, 0);
        assert_eq!(n2.text_runs[0].len, 5);
        let c = n2.text_runs[0].color.expect("color survives");
        assert!(c.components[0] > 0.99 && c.components[1] < 0.01);
        assert_eq!(n2.text_runs[0].size, Some(28.0));
        assert_eq!(n2.text_runs[0].font.as_deref(), Some("Inter"));
        assert_eq!(n2.text_runs[1].size, Some(14.0));
        assert!(n2.text_runs[1].color.is_none());
        // byte-stable double round-trip
        assert_eq!(a, save_x(&back));
    }

    #[test]
    fn plain_text_stays_byte_identical_without_runs() {
        let doc = sample_doc();
        let a = save_x(&doc);
        assert!(!a.contains("textRuns"), "no runs key for plain text");
        assert_eq!(a, save_x(&load_x(&a).unwrap()));
    }

    #[test]
    fn gradient_strokes_survive_x_roundtrip() {
        // gradient strokes serialize through "paint" (solid keeps the
        // legacy "color" key, so old files stay byte-identical)
        let mut doc = sample_doc();
        let n = doc.pages[0].children.iter_mut().find(|c| c.id == "card").unwrap();
        n.stroke = x_core::Stroke {
            paint: Paint::LinearGradient {
                start: (0.0, 0.0), end: (100.0, 0.0),
                stops: vec![(0.0, Color::from_rgb8(255, 0, 0)), (1.0, Color::from_rgb8(0, 0, 255))],
            },
            width: 3.0,
        };
        let a = save_x(&doc);
        let back = load_x(&a).unwrap();
        let n2 = find(&back.pages[0], "card").unwrap();
        match &n2.stroke.paint {
            Paint::LinearGradient { start, end, stops } => {
                assert_eq!(*start, (0.0, 0.0));
                assert_eq!(*end, (100.0, 0.0));
                assert_eq!(stops.len(), 2);
            }
            other => panic!("gradient stroke lost: {other:?}"),
        }
        assert_eq!(n2.stroke.width, 3.0);
        // and the save is byte-stable across a second round-trip
        let b = save_x(&back);
        assert_eq!(a, b);
    }

    #[test]
    fn double_roundtrip_is_stable() {
        let doc = sample_doc();
        let a = save_x(&doc);
        let b = save_x(&load_x(&a).unwrap());
        assert_eq!(a, b, "save(load(save(x))) must equal save(x)");
    }

    #[test]
    fn rejects_wrong_format_and_newer_versions() {
        assert!(load_x("{\"format\":\"figma\",\"version\":1}").is_err());
        assert!(load_x(&format!("{{\"format\":\"x-native\",\"version\":{}}}", X_FORMAT_VERSION + 1)).is_err());
        assert!(load_x("not json at all").is_err());
    }

    #[test]
    fn file_roundtrip_on_disk() {
        let doc = sample_doc();
        let path = std::env::temp_dir().join("xnative_test.x");
        let path = path.to_str().unwrap();
        save_x_file(&doc, path).unwrap();
        let loaded = load_x_file(path).unwrap();
        assert_eq!(loaded.pages.len(), 2);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn vector_path_roundtrips_through_x_format() {
        let mut doc = Document::new();
        let tri = Node::vector("tri", 0.0, 0.0, 100.0, 100.0, vec![
            PathCmd::MoveTo(50.0, 0.0),
            PathCmd::LineTo(100.0, 100.0),
            PathCmd::CurveTo(75.0, 90.0, 25.0, 90.0, 0.0, 100.0),
            PathCmd::Close,
        ]);
        doc.pages.push(Node::frame("p", 200.0, 200.0).child(tri));
        let loaded = load_x(&save_x(&doc)).unwrap();
        let n = find(&loaded.pages[0], "tri").unwrap();
        if let NodeKind::Vector { path } = &n.kind {
            assert_eq!(path.len(), 4);
            assert_eq!(path[0], PathCmd::MoveTo(50.0, 0.0));
            assert!(matches!(path[2], PathCmd::CurveTo(..)));
            assert_eq!(path[3], PathCmd::Close);
        } else { panic!("vector kind lost") }
    }

    #[test]
    fn svg_import_basic_shapes() {
        let svg = r##"<?xml version="1.0"?>
        <svg xmlns="http://www.w3.org/2000/svg" width="400" height="300">
          <rect id="bg" x="10" y="20" width="100" height="50" rx="8" fill="#ff0000"/>
          <circle id="c1" cx="200" cy="100" r="40" fill="#00ff00"/>
          <g id="grp" transform="translate(50 60)">
            <ellipse cx="30" cy="20" rx="30" ry="20" fill="#0000ff"/>
          </g>
          <text id="label" x="10" y="290" font-size="20" fill="#000000">Hi there</text>
        </svg>"##;
        let root = import_svg(svg).expect("import");
        assert_eq!((root.w, root.h), (400.0, 300.0));
        let bg = find(&root, "bg").unwrap();
        assert_eq!((bg.transform.x, bg.w), (10.0, 100.0));
        assert!(matches!(bg.kind, NodeKind::Rect { radius } if radius == 8.0));
        assert!(matches!(&bg.fill, Paint::Solid(c) if c.to_rgba8().r == 255 && c.to_rgba8().g == 0));
        let c1 = find(&root, "c1").unwrap();
        assert_eq!((c1.transform.x, c1.w), (160.0, 80.0)); // cx-r, 2r
        let grp = find(&root, "grp").unwrap();
        assert_eq!((grp.transform.x, grp.transform.y), (50.0, 60.0));
        assert_eq!(grp.children.len(), 1);
        let label = find(&root, "label").unwrap();
        assert!(matches!(&label.kind, NodeKind::Text { text } if text == "Hi there"));
    }

    #[test]
    fn svg_import_path_with_relative_commands() {
        let svg = r##"<svg width="100" height="100">
          <path id="p" d="M 10 10 l 20 0 L 30 30 h 10 v 10 C 35 45 25 45 20 40 z" fill="#123456"/>
        </svg>"##;
        let root = import_svg(svg).unwrap();
        let p = find(&root, "p").unwrap();
        if let NodeKind::Vector { path } = &p.kind {
            assert_eq!(path[0], PathCmd::MoveTo(10.0, 10.0));
            assert_eq!(path[1], PathCmd::LineTo(30.0, 10.0));  // relative l resolved
            assert_eq!(path[2], PathCmd::LineTo(30.0, 30.0));
            assert_eq!(path[3], PathCmd::LineTo(40.0, 30.0));  // h
            assert_eq!(path[4], PathCmd::LineTo(40.0, 40.0));  // v
            assert!(matches!(path[5], PathCmd::CurveTo(..)));
            assert_eq!(*path.last().unwrap(), PathCmd::Close);
        } else { panic!("not a vector") }
        // imported vector actually renders
        let (_, s) = x_render::build_scene(&root, None, &Variables::default());
        assert!(s.paths >= 1);
    }

    #[test]
    fn svg_roundtrip_export_then_import() {
        // Export our own scene, re-import it, and check the shapes survive.
        let page = Node::frame("page", 500.0, 400.0)
            .child(Node::rect("r1", 20.0, 30.0, 120.0, 60.0, Color::from_rgb8(0x0d, 0x99, 0xff)).radius(10.0))
            .child(Node::ellipse("e1", 200.0, 50.0, 80.0, 80.0, Color::from_rgb8(0xf2, 0x48, 0x22)));
        let svg = export_svg(&page, &Variables::default());
        let re = import_svg(&svg).expect("re-import own export");
        // our exporter wraps each node in <g transform=translate(...)>, so
        // position lands on the wrapping group; shape + fill must survive.
        fn count_kind(n: &Node, pred: &dyn Fn(&NodeKind) -> bool) -> usize {
            (pred(&n.kind) as usize) + n.children.iter().map(|c| count_kind(c, pred)).sum::<usize>()
        }
        assert_eq!(count_kind(&re, &|k| matches!(k, NodeKind::Rect { .. })), 1);
        assert_eq!(count_kind(&re, &|k| matches!(k, NodeKind::Ellipse)), 1);
        let (_, s) = x_render::build_scene(&re, None, &Variables::default());
        assert_eq!(s.paths, 2);
    }

    #[test]
    fn full_variable_engine_roundtrips() {
        let mut doc = Document::new();
        let v = &mut doc.variables;
        v.colors.insert("bg".into(), Color::from_rgb8(0xff, 0xff, 0xff));
        v.numbers.insert("gap".into(), 12.0);
        v.strings.insert("brand".into(), "X Native".into());
        v.bools.insert("beta".into(), true);
        v.collections.insert("bg".into(), "Semantic".into());
        v.collections.insert("gap".into(), "Primitives".into());
        let mut dark = std::collections::HashMap::new();
        dark.insert("bg".to_string(), Color::from_rgb8(0x11, 0x11, 0x11));
        v.modes.insert("dark".into(), dark);
        doc.pages.push(Node::frame("page", 100.0, 100.0));
        let text = save_x(&doc);
        let loaded = load_x(&text).unwrap();
        let lv = &loaded.variables;
        assert_eq!(lv.string("brand", ""), "X Native");
        assert!(lv.boolean("beta", false));
        assert_eq!(lv.collection_of("bg"), "Semantic");
        assert_eq!(lv.collection_of("gap"), "Primitives");
        assert_eq!(lv.collection_of("unknown"), "Local");
        assert_eq!(lv.modes["dark"]["bg"].to_rgba8().r, 0x11);
        assert_eq!(save_x(&load_x(&text).unwrap()), text);
        // catalog groups by collection for the UI
        let cat = lv.catalog();
        assert!(cat.contains(&("Semantic".into(), "bg".into(), "color")));
        assert!(cat.contains(&("Primitives".into(), "gap".into(), "number")));
    }

    #[test]
    fn mask_and_image_fit_roundtrip() {
        let mut doc = Document::new();
        let mut img = Node::image("img", 0.0, 0.0, 100.0, 80.0, "photo");
        if let NodeKind::Image { fit, .. } = &mut img.kind { *fit = ImageFit::Crop; }
        doc.pages.push(Node::frame("p", 400.0, 300.0)
            .child(Node::ellipse("m", 0.0, 0.0, 50.0, 50.0, Color::WHITE).mask(true))
            .child(img));
        let loaded = load_x(&save_x(&doc)).unwrap();
        let m = find(&loaded.pages[0], "m").unwrap();
        assert!(m.is_mask, "mask flag must roundtrip");
        let i = find(&loaded.pages[0], "img").unwrap();
        assert!(matches!(i.kind, NodeKind::Image { fit: ImageFit::Crop, .. }), "fit must roundtrip");
        assert_eq!(save_x(&load_x(&save_x(&doc)).unwrap()), save_x(&doc));
    }

    #[test]
    fn variable_bindings_roundtrip() {
        let mut doc = Document::new();
        doc.variables.numbers.insert("radius-lg".into(), 20.0);
        doc.variables.collections.insert("radius-lg".into(), "Primitives".into());
        doc.pages.push(Node::frame("page", 100.0, 100.0)
            .child(Node::rect("r", 0.0, 0.0, 50.0, 50.0, Color::WHITE).bind("radius", "radius-lg").bind("opacity", "dim")));
        let loaded = load_x(&save_x(&doc)).unwrap();
        let n = find(&loaded.pages[0], "r").unwrap();
        assert_eq!(n.bindings.get("radius").map(String::as_str), Some("radius-lg"));
        assert_eq!(n.bindings.get("opacity").map(String::as_str), Some("dim"));
        assert_eq!(save_x(&load_x(&save_x(&doc)).unwrap()), save_x(&doc));
    }

    #[test]
    fn component_system_2_serialization_roundtrips() {
        use x_components::{set_override, typed_overrides, OverrideValue};
        let mut doc = Document::new();
        // master with variants + typed overrides on instances
        let mut m1 = Node::component("c1", "Button/Primary", 100.0, 40.0);
        m1.visible = false;
        m1.children.push(Node::rect("bg", 0.0, 0.0, 100.0, 40.0, Color::from_rgb8(0, 0, 0xff)));
        m1.children.push(Node::text("label", 8.0, 8.0, 80.0, 16.0, "OK"));
        m1.children.push(Node::instance("nested", "Icon", 4.0, 4.0, 16.0, 16.0));
        let mut inst = Node::instance("i1", "Button/Primary", 200.0, 100.0, 100.0, 40.0);
        set_override(&mut inst, "label", OverrideValue::Text("Buy now".into()));
        set_override(&mut inst, "bg", OverrideValue::Fill(Color::from_rgb8(0xff, 0, 0)));
        set_override(&mut inst, "icon", OverrideValue::Visible(false));
        set_override(&mut inst, "half", OverrideValue::Opacity(0.5));
        set_override(&mut inst, "nested", OverrideValue::Swap("Icon/Cross".into()));
        doc.pages.push(Node::frame("page", 800.0, 600.0).child(m1).child(inst));

        let text = save_x(&doc);
        let loaded = load_x(&text).expect("load");
        let page = &loaded.pages[0];
        let li = find(page, "i1").unwrap();
        let t = typed_overrides(li);
        assert_eq!(t.get("label"), Some(&OverrideValue::Text("Buy now".into())));
        assert_eq!(t.get("bg"), Some(&OverrideValue::Fill(Color::from_rgb8(0xff, 0, 0))));
        assert_eq!(t.get("icon"), Some(&OverrideValue::Visible(false)));
        assert_eq!(t.get("half"), Some(&OverrideValue::Opacity(0.5)));
        assert_eq!(t.get("nested"), Some(&OverrideValue::Swap("Icon/Cross".into())));
        // variant name survives; master hidden flag survives
        let lm = find(page, "c1").unwrap();
        assert!(matches!(&lm.kind, NodeKind::Component { name } if name == "Button/Primary"));
        assert!(!lm.visible);
        // double-roundtrip stability with typed overrides present
        assert_eq!(save_x(&load_x(&text).unwrap()), text);
    }

    #[test]
    fn component_autolayout_state_roundtrips() {
        // hug frame inside a master serializes with sizing=hug intact
        let mut doc = Document::new();
        let mut comp = Node::component("c", "Chip", 60.0, 24.0);
        comp.visible = false;
        comp.children.push(
            Node::frame("chip-root", 0.0, 0.0).auto_layout(AutoLayout {
                direction: LayoutDirection::Horizontal, gap: 4.0, padding: [6.0; 4],
                sizing: x_core::Sizing::Hug, ..Default::default()
            }).child(Node::text("chip-label", 0.0, 0.0, 30.0, 12.0, "tag")),
        );
        doc.pages.push(Node::frame("page", 400.0, 300.0).child(comp));
        let loaded = load_x(&save_x(&doc)).unwrap();
        let root = find(&loaded.pages[0], "chip-root").unwrap();
        if let NodeKind::Frame { layout: Some(l) } = &root.kind {
            assert_eq!(l.sizing, x_core::Sizing::Hug);
            assert_eq!(l.gap, 4.0);
        } else { panic!("hug layout lost in serialization"); }
    }

    #[test]
    fn svg_export_contains_shapes_and_gradients() {
        let doc = sample_doc();
        let svg = export_svg(&doc.pages[0], &doc.variables);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("linearGradient"));
        assert!(svg.contains("rotate("));
    }

    #[test]
    fn visual_stacks_roundtrip_with_layer_state() {
        let mut node = Node::rect("stacked", 0.0, 0.0, 100.0, 80.0, Color::BLACK);
        node.visual_stacks_materialized = true;
        node.fill_layers = vec![
            PaintLayer::new(Paint::Solid(Color::from_rgb8(255, 0, 0))),
            PaintLayer { paint: Paint::LinearGradient { start: (0.0, 0.0), end: (100.0, 0.0), stops: vec![(0.0, Color::WHITE), (1.0, Color::BLACK)] }, opacity: 0.65, visible: false, blend: BlendKind::Screen },
        ];
        node.stroke_layers = vec![StrokeLayer { stroke: Stroke::solid(Color::WHITE, 3.0), opacity: 0.8, visible: true, blend: BlendKind::Multiply, options: StrokeOptions::default() }];
        node.effect_layers = vec![EffectLayer { effect: Effect::DropShadow { dx: 2.0, dy: 4.0, blur: 12.0, color: Color::BLACK }, opacity: 0.5, visible: false, blend: BlendKind::SoftLight }];
        let mut doc = Document::new();
        doc.pages.push(Node::frame("page", 200.0, 200.0).child(node));
        let loaded = load_x(&save_x(&doc)).expect("visual stacks reload");
        let n = find(&loaded.pages[0], "stacked").unwrap();
        assert_eq!(n.fill_layers.len(), 2);
        assert_eq!(n.stroke_layers.len(), 1);
        assert_eq!(n.effect_layers.len(), 1);
        assert!(!n.fill_layers[1].visible);
        assert_eq!(n.fill_layers[1].blend, BlendKind::Screen);
        assert_eq!(n.stroke_layers[0].stroke.width, 3.0);
        assert!(!n.effect_layers[0].visible);
        assert_eq!(n.effect_layers[0].blend, BlendKind::SoftLight);
    }

    #[test]
    fn extended_blend_modes_roundtrip_without_fallback() {
        for blend in [BlendKind::ColorBurn, BlendKind::ColorDodge, BlendKind::SoftLight, BlendKind::HardLight, BlendKind::Difference, BlendKind::Exclusion, BlendKind::Hue, BlendKind::Saturation, BlendKind::Color, BlendKind::Luminosity] {
            let mut node = Node::rect("blend", 0.0, 0.0, 40.0, 40.0, Color::WHITE);
            node.blend = blend;
            let mut doc = Document::new();
            doc.pages.push(Node::frame("page", 100.0, 100.0).child(node));
            let loaded = load_x(&save_x(&doc)).expect("extended blend should deserialize");
            assert_eq!(loaded.pages[0].children[0].blend, blend);
        }
    }

    #[test]
    fn per_side_padding_cross_sizing_and_pin_round_trip() {
        let mut doc = Document::new();
        let page = Node::frame("p", 200.0, 200.0).child(
            Node::frame("f", 80.0, 60.0)
                .auto_layout(AutoLayout {
                    direction: LayoutDirection::Horizontal, gap: 4.0,
                    padding: [10.0, 4.0, 6.0, 2.0], sizing: Sizing::Fixed,
                    cross_sizing: Some(Sizing::Hug), ..Default::default()
                })
                .pin(HPin::StretchH, VPin::CenterV),
        );
        doc.pages.push(page);
        let text = save_x(&doc);
        // non-uniform padding + cross_sizing + pin must be written
        assert!(text.contains("\"padding\":[10,4,6,2]"), "per-side padding serializes as an array: {text}");
        assert!(text.contains("\"cross_sizing\":\"hug\""));
        assert!(text.contains("\"pin\":\"stretch center\""));
        let loaded = load_x(&text).expect("load");
        let f = find(&loaded.pages[0], "f").unwrap();
        let NodeKind::Frame { layout: Some(l) } = &f.kind else { panic!("frame layout") };
        assert_eq!(l.padding, [10.0, 4.0, 6.0, 2.0]);
        assert_eq!(l.cross_sizing, Some(Sizing::Hug));
        assert_eq!(f.pin, (HPin::StretchH, VPin::CenterV));
        // byte-stable across save/load
        assert_eq!(save_x(&loaded), text);
    }

    #[test]
    fn uniform_padding_and_default_pins_stay_legacy_shaped() {
        let mut doc = Document::new();
        doc.pages.push(Node::frame("p", 100.0, 100.0).child(
            Node::frame("f", 40.0, 30.0)
                .auto_layout(AutoLayout { direction: LayoutDirection::Vertical, gap: 2.0, padding: [8.0; 4], sizing: Sizing::Hug, ..Default::default() }),
        ));
        let text = save_x(&doc);
        assert!(text.contains("\"padding\":8") && !text.contains("\"padding\":["), "uniform padding stays scalar: {text}");
        assert!(!text.contains("\"pin\""), "default Left/Top pin is never written");
        // legacy scalar parses back as all-four-sides
        let legacy = text.replace("\"padding\":8", "\"padding\":12");
        let loaded = load_x(&legacy).unwrap();
        let f = find(&loaded.pages[0], "f").unwrap();
        let NodeKind::Frame { layout: Some(l) } = &f.kind else { panic!("frame layout") };
        assert_eq!(l.padding, [12.0; 4], "legacy scalar padding loads onto all four sides");
    }
}
