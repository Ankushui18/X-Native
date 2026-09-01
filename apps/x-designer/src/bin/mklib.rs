fn main() {
    let mut lib = arco_native::Library::default();
    lib.library_id = "brand-kit".into();
    lib.name = "Brand Kit".into();
    lib.version = 1;
    lib.styles.insert("Brand/Primary".into(), arco_native::Style::Paint { fill: arco_native::Paint::Solid(arco_native::Color::rgb8(0x33,0x66,0xff)) });
    lib.styles.insert("Brand/Warn".into(), arco_native::Style::Paint { fill: arco_native::Paint::Solid(arco_native::Color::rgb8(0xff,0x5a,0x00)) });
    let mut btn = arco_native::Node::rect("lib-btn", 0.0, 0.0, 120.0, 40.0, arco_native::Color::rgb8(0x33,0x66,0xff));
    btn.kind = arco_native::NodeKind::Component { name: "LibButton".into() };
    lib.components.push(btn);
    std::fs::write("library.xlib", arco_native::fileio::save_xlib(&lib)).unwrap();
    println!("wrote library.xlib v{}", lib.version);
}
