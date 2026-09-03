#[allow(unused_imports)]
use super::*;

/// Session 47: the starter document now recreates the USER'S MOCKUP —
/// a white "Desktop - 1440" landing-page frame (header, hero, feature
/// cards, chart card) plus the mockup's page set (Dashboard, Analytics,
/// Users, Settings, Mobile). Every node is a plain editable Node, so the
/// layers tree mirrors the mockup's (Header / Hero Section / Features).
pub fn demo_document() -> Document {
    let mut vars = Variables::default();
    vars.numbers.insert("gap-lg".into(), 28.0);
    // exposed for prototype viewers: edit it live in present mode
    vars.exposed.insert("gap-lg".into());
    vars.colors
        .insert("brand".into(), Color::from_rgb8(0x33, 0x66, 0xff));
    vars.colors
        .insert("surface".into(), Color::from_rgb8(0xff, 0xff, 0xff));
    vars.numbers.insert("radius-md".into(), 12.0);
    vars.numbers.insert("radius-lg".into(), 24.0);
    vars.numbers.insert("opacity-dim".into(), 0.4);
    vars.collections.insert("brand".into(), "Semantic".into());
    vars.collections.insert("surface".into(), "Semantic".into());
    vars.collections
        .insert("gap-lg".into(), "Primitives".into());
    vars.collections
        .insert("radius-md".into(), "Primitives".into());
    vars.collections
        .insert("radius-lg".into(), "Primitives".into());
    vars.collections
        .insert("opacity-dim".into(), "Primitives".into());
    let mut dark = std::collections::HashMap::new();
    dark.insert("surface".to_string(), Color::from_rgb8(0x20, 0x22, 0x27));
    dark.insert("brand".to_string(), Color::from_rgb8(0x4d, 0xb8, 0xff));
    vars.modes.insert("dark".into(), dark);

    let ink = Color::from_rgb8(0x0d, 0x12, 0x20);
    let muted = Color::from_rgb8(0x6b, 0x72, 0x80);
    let brand = Color::from_rgb8(0x33, 0x66, 0xff);
    let border = Color::from_rgb8(0xe5, 0xe7, 0xeb);
    let lav = Color::from_rgb8(0xc7, 0xd2, 0xfe); // soft indigo blobs
    let lav2 = Color::from_rgb8(0xe0, 0xe7, 0xff); // icon tiles

    // ---------------- Header group ----------------
    let header = Node::group("Header", 1440.0, 90.0)
        .child(Node::rect("Logo Mark", 48.0, 30.0, 26.0, 26.0, brand).radius(7.0))
        .child(Node::text("Logo", 84.0, 34.0, 120.0, 19.0, "Brand"))
        .child(Node::text(
            "Nav Product",
            560.0,
            38.0,
            90.0,
            13.0,
            "Product",
        ))
        .child(Node::text(
            "Nav Solutions",
            660.0,
            38.0,
            100.0,
            13.0,
            "Solutions",
        ))
        .child(Node::text(
            "Nav Resources",
            770.0,
            38.0,
            100.0,
            13.0,
            "Resources",
        ))
        .child(Node::text(
            "Nav Pricing",
            880.0,
            38.0,
            80.0,
            13.0,
            "Pricing",
        ))
        .child(Node::rect("CTA Header", 1240.0, 24.0, 130.0, 38.0, brand).radius(9.0))
        .child({
            let mut t = Node::text("CTA Header Label", 1262.0, 36.0, 100.0, 12.0, "Get Started");
            t.fill = Paint::Solid(Color::WHITE);
            t
        });

    // ---------------- Hero section ----------------
    let mut chart_line = Node::vector(
        "Chart Line",
        0.0,
        0.0,
        160.0,
        60.0,
        vec![
            x_native::PathCmd::MoveTo(0.0, 52.0),
            x_native::PathCmd::LineTo(28.0, 34.0),
            x_native::PathCmd::LineTo(56.0, 44.0),
            x_native::PathCmd::LineTo(84.0, 18.0),
            x_native::PathCmd::LineTo(112.0, 30.0),
            x_native::PathCmd::LineTo(140.0, 6.0),
            x_native::PathCmd::LineTo(160.0, 14.0),
        ],
    );
    chart_line.transform.x = 962.0;
    chart_line.transform.y = 300.0;
    chart_line.fill = Paint::Solid(Color::from_rgba8(0, 0, 0, 0));
    chart_line.stroke = x_native::Stroke::solid(brand, 3.0);

    let hero = Node::group("Hero Section", 1440.0, 480.0)
        // soft indigo blobs behind the cards (mockup right side)
        .child(
            Node::rect("Blob 1", 760.0, 140.0, 210.0, 90.0, lav)
                .radius(28.0)
                .opacity(0.55),
        )
        .child(
            Node::rect("Blob 2", 800.0, 250.0, 260.0, 110.0, lav)
                .radius(34.0)
                .opacity(0.45),
        )
        .child(
            Node::rect("Blob 3", 740.0, 380.0, 200.0, 90.0, lav)
                .radius(28.0)
                .opacity(0.5),
        )
        // badge
        .child(
            Node::rect(
                "Badge",
                96.0,
                140.0,
                250.0,
                30.0,
                Color::from_rgb8(0xef, 0xf2, 0xff),
            )
            .radius(15.0),
        )
        .child({
            let mut t = Node::text("Badge New", 112.0, 149.0, 34.0, 11.0, "New");
            t.fill = Paint::Solid(brand);
            t
        })
        .child({
            let mut t = Node::text(
                "Badge Text",
                152.0,
                149.0,
                190.0,
                11.0,
                "Design at the speed of ideas",
            );
            t.fill = Paint::Solid(muted);
            t
        })
        // heading (two lines) + accent word
        .child(Node::text(
            "Heading 1",
            94.0,
            190.0,
            560.0,
            44.0,
            "Create stunning",
        ))
        .child(Node::text("Heading 2", 94.0, 240.0, 400.0, 44.0, "designs"))
        .child({
            let mut t = Node::text("Heading Accent", 296.0, 240.0, 260.0, 44.0, "together");
            t.fill = Paint::Solid(Color::from_rgb8(0x7c, 0x83, 0xff));
            t
        })
        // subtext
        .child({
            let mut t = Node::text(
                "Subheading 1",
                96.0,
                308.0,
                460.0,
                15.0,
                "X-Native helps teams design, prototype, and",
            );
            t.fill = Paint::Solid(muted);
            t
        })
        .child({
            let mut t = Node::text(
                "Subheading 2",
                96.0,
                330.0,
                400.0,
                15.0,
                "launch better products, faster.",
            );
            t.fill = Paint::Solid(muted);
            t
        })
        // CTAs
        .child(
            Node::rect("CTA Button", 96.0, 378.0, 160.0, 42.0, brand)
                .radius(9.0)
                .effect(Effect::DropShadow {
                    dx: 0.0,
                    dy: 6.0,
                    blur: 14.0,
                    color: Color::from_rgba8(0x33, 0x66, 0xff, 70),
                }),
        )
        .child({
            let mut t = Node::text("CTA Label", 118.0, 391.0, 130.0, 13.0, "Get Started Free");
            t.fill = Paint::Solid(Color::WHITE);
            t
        })
        .child({
            let mut t = Node::text("Watch Demo", 286.0, 391.0, 110.0, 13.0, "Watch Demo");
            t.fill = Paint::Solid(ink);
            t
        })
        // color chips card (mockup: Primary/Secondary/Success swatches)
        .child(
            Node::rect("Chips Card", 806.0, 160.0, 190.0, 170.0, Color::WHITE)
                .radius(14.0)
                .effect(Effect::DropShadow {
                    dx: 0.0,
                    dy: 8.0,
                    blur: 22.0,
                    color: Color::from_rgba8(0x10, 0x18, 0x30, 40),
                }),
        )
        .child(Node::rect("Chip Primary", 822.0, 178.0, 22.0, 22.0, brand).radius(6.0))
        .child({
            let mut t = Node::text("Chip P Name", 854.0, 180.0, 70.0, 11.0, "Primary");
            t.fill = Paint::Solid(ink);
            t
        })
        .child({
            let mut t = Node::text("Chip P Hex", 854.0, 194.0, 70.0, 9.0, "#3366FF");
            t.fill = Paint::Solid(muted);
            t
        })
        .child(
            Node::rect(
                "Chip Secondary",
                822.0,
                232.0,
                22.0,
                22.0,
                Color::from_rgb8(0x66, 0x33, 0xff),
            )
            .radius(6.0),
        )
        .child({
            let mut t = Node::text("Chip S Name", 854.0, 234.0, 80.0, 11.0, "Secondary");
            t.fill = Paint::Solid(ink);
            t
        })
        .child({
            let mut t = Node::text("Chip S Hex", 854.0, 248.0, 70.0, 9.0, "#6633FF");
            t.fill = Paint::Solid(muted);
            t
        })
        .child(
            Node::rect(
                "Chip Success",
                822.0,
                286.0,
                22.0,
                22.0,
                Color::from_rgb8(0x22, 0xc5, 0x5e),
            )
            .radius(6.0),
        )
        .child({
            let mut t = Node::text("Chip G Name", 854.0, 288.0, 70.0, 11.0, "Success");
            t.fill = Paint::Solid(ink);
            t
        })
        .child({
            let mut t = Node::text("Chip G Hex", 854.0, 302.0, 70.0, 9.0, "#22C55E");
            t.fill = Paint::Solid(muted);
            t
        })
        // chart card (mockup: +12.5% / 3.2k / line chart)
        .child(
            Node::rect("Chart Card", 946.0, 210.0, 200.0, 170.0, Color::WHITE)
                .radius(14.0)
                .effect(Effect::DropShadow {
                    dx: 0.0,
                    dy: 8.0,
                    blur: 22.0,
                    color: Color::from_rgba8(0x10, 0x18, 0x30, 40),
                }),
        )
        .child({
            let mut t = Node::text("Chart Delta", 962.0, 228.0, 60.0, 10.0, "+12.5%");
            t.fill = Paint::Solid(Color::from_rgb8(0x22, 0xc5, 0x5e));
            t
        })
        .child(Node::text("Chart Value", 962.0, 244.0, 90.0, 26.0, "3.2k"))
        .child(chart_line)
        // avatar strip pill
        .child(
            Node::rect("Avatars Pill", 900.0, 452.0, 190.0, 44.0, Color::WHITE)
                .radius(22.0)
                .effect(Effect::DropShadow {
                    dx: 0.0,
                    dy: 6.0,
                    blur: 16.0,
                    color: Color::from_rgba8(0x10, 0x18, 0x30, 36),
                }),
        )
        .child(Node::ellipse(
            "Avatar 1",
            912.0,
            460.0,
            28.0,
            28.0,
            Color::from_rgb8(0xf2, 0xa2, 0x66),
        ))
        .child(Node::ellipse(
            "Avatar 2",
            934.0,
            460.0,
            28.0,
            28.0,
            Color::from_rgb8(0x8e, 0x6a, 0x4f),
        ))
        .child(Node::ellipse(
            "Avatar 3",
            956.0,
            460.0,
            28.0,
            28.0,
            Color::from_rgb8(0x5b, 0x8d, 0xef),
        ))
        .child({
            let mut t = Node::text("Avatar Count", 996.0, 474.0, 40.0, 12.0, "+24");
            t.fill = Paint::Solid(ink);
            t
        });

    // ---------------- Feature cards ----------------
    let feature = |i: usize, x: f64, title: &str, l1: &str, l2: &str| -> Vec<Node> {
        let n = |s: &str| format!("{s} {i}");
        vec![
            Node::rect(&n("Feature Card"), x, 620.0, 330.0, 190.0, Color::WHITE)
                .radius(12.0)
                .stroke(x_native::Stroke::solid(border, 1.0)),
            Node::rect(&n("Feature Icon"), x + 24.0, 644.0, 40.0, 40.0, lav2).radius(10.0),
            Node::text(&n("Feature Title"), x + 24.0, 706.0, 220.0, 16.0, title),
            {
                let mut t = Node::text(&n("Feature Body A"), x + 24.0, 734.0, 280.0, 12.0, l1);
                t.fill = Paint::Solid(muted);
                t
            },
            {
                let mut t = Node::text(&n("Feature Body B"), x + 24.0, 752.0, 280.0, 12.0, l2);
                t.fill = Paint::Solid(muted);
                t
            },
        ]
    };
    let mut features = Node::group("Features", 1440.0, 260.0);
    for n in feature(
        1,
        96.0,
        "Design System",
        "Create and maintain consistent",
        "design systems with ease.",
    ) {
        features = features.child(n);
    }
    for n in feature(
        2,
        462.0,
        "Components",
        "Build reusable components",
        "that scale with your product.",
    ) {
        features = features.child(n);
    }
    for n in feature(
        3,
        828.0,
        "Collaboration",
        "Work together in real-time",
        "with your entire team.",
    ) {
        features = features.child(n);
    }

    // ---------------- the Desktop frame ----------------
    let mut desktop = Node::frame("Desktop - 1440", 1440.0, 1024.0)
        .child(header)
        .child(hero)
        .child(features);
    desktop.transform.x = 80.0;
    desktop.transform.y = 60.0;
    desktop.fill = Paint::Solid(Color::WHITE);

    let mut dashboard = Node::frame("Dashboard", 1600.0, 1200.0);
    dashboard.fill = Paint::Solid(Color::from_rgba8(0, 0, 0, 0));
    let dashboard = dashboard.child(desktop);

    // ---------------- secondary pages (mockup page strip) ----------------
    let dark_page = |name: &str, accent: Color| -> Node {
        let bg = Node::rect(
            &format!("{name} BG"),
            80.0,
            60.0,
            1440.0,
            900.0,
            Color::from_rgb8(0x14, 0x16, 0x1b),
        )
        .radius(4.0);
        let side = Node::rect(
            &format!("{name} Sidebar"),
            80.0,
            60.0,
            220.0,
            900.0,
            Color::from_rgb8(0x1b, 0x1e, 0x25),
        );
        let bar = Node::rect(
            &format!("{name} Topbar"),
            300.0,
            60.0,
            1220.0,
            64.0,
            Color::from_rgb8(0x1b, 0x1e, 0x25),
        );
        let mut page = Node::frame(name, 1600.0, 1200.0);
        page.fill = Paint::Solid(Color::from_rgba8(0, 0, 0, 0));
        let mut page = page.child(bg).child(side).child(bar);
        for i in 0..3 {
            page = page.child(
                Node::rect(
                    &format!("{name} Card {}", i + 1),
                    340.0 + i as f64 * 390.0,
                    170.0,
                    350.0,
                    200.0,
                    Color::from_rgb8(0x20, 0x23, 0x2b),
                )
                .radius(10.0),
            );
        }
        page = page.child(
            Node::rect(
                &format!("{name} Hero"),
                340.0,
                410.0,
                740.0,
                330.0,
                Color::from_rgb8(0x20, 0x23, 0x2b),
            )
            .radius(10.0),
        );
        page = page.child(
            Node::rect(&format!("{name} Accent"), 360.0, 430.0, 180.0, 12.0, accent).radius(5.0),
        );
        page
    };

    let mut doc = Document::new();
    doc.variables = vars.clone();
    doc.styles.insert(
        "Brand/Blue".into(),
        x_native::Style::Paint {
            fill: Paint::Solid(brand),
        },
    );
    doc.styles.insert(
        "Elev/Card".into(),
        x_native::Style::Effect {
            effects: vec![Effect::DropShadow {
                dx: 0.0,
                dy: 8.0,
                blur: 22.0,
                color: Color::from_rgba8(0x10, 0x18, 0x30, 40),
            }],
        },
    );
    let mut p = dashboard;
    x_native::apply_layout_recursive(&mut p, &vars);
    doc.pages.push(p);
    doc.pages.push(dark_page("Analytics", brand));
    doc.pages
        .push(dark_page("Users", Color::from_rgb8(0x22, 0xc5, 0x5e)));
    doc.pages
        .push(dark_page("Settings", Color::from_rgb8(0xf2, 0xa2, 0x66)));
    doc.pages
        .push(dark_page("Mobile", Color::from_rgb8(0x7c, 0x83, 0xff)));
    doc
}
