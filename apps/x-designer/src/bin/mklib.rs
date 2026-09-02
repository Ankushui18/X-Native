fn main() {
    let mut lib = x_native::Library {
        library_id: "brand-kit".into(),
        name: "Brand Kit".into(),
        version: 1,
        ..Default::default()
    };
    lib.styles.insert("Brand/Primary".into(), x_native::Style::Paint { fill: x_native::Paint::Solid(x_native::Color::from_rgb8(0x33,0x66,0xff)) });
    lib.styles.insert("Brand/Warn".into(), x_native::Style::Paint { fill: x_native::Paint::Solid(x_native::Color::from_rgb8(0xff,0x5a,0x00)) });
    let mut btn = x_native::Node::rect("lib-btn", 0.0, 0.0, 120.0, 40.0, x_native::Color::from_rgb8(0x33,0x66,0xff));
    btn.kind = x_native::NodeKind::Component { name: "LibButton".into() };
    lib.components.push(btn);
    std::fs::write("library.xlib", x_native::fileio::save_xlib(&lib)).unwrap();
    println!("wrote library.xlib v{}", lib.version);
}
