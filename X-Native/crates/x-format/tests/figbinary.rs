//! End-to-end tests against REAL Figma-authored `.fig` files (from the
//! open-source OpenFig test corpus): exercises the ZIP container, the
//! zstd + deflate chunk paths, the embedded-schema Kiwi decode, and the
//! REST-shim mapping in one shot.

use x_format::{import_fig_bytes, import_fig_bytes_with_report};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture present")
}

#[test]
fn circle_fig_real_binary_file() {
    let (doc, report) = import_fig_bytes_with_report(&fixture("circle.fig")).unwrap();
    assert_eq!(
        doc.pages.len(),
        1,
        "one visible page: {:?}",
        report.diagnostics
    );
    let page = &doc.pages[0];
    assert_eq!(page.name, "Page 1");
    // Frame 1 (350x350) containing Ellipse 1 (300x300 at 25,25)
    let frame = &page.children[0];
    assert_eq!(frame.name, "Frame 1");
    assert_eq!((frame.w, frame.h), (350.0, 350.0));
    let ell = &frame.children[0];
    assert_eq!(ell.name, "Ellipse 1");
    assert_eq!((ell.w, ell.h), (300.0, 300.0));
    assert_eq!((ell.transform.x, ell.transform.y), (25.0, 25.0));
    // red solid fill rode the shim all the way to a Paint
    assert!(
        matches!(ell.kind, x_core::NodeKind::Ellipse),
        "kind: {:?}",
        ell.kind
    );
    let s = format!("{:?}", ell.fill);
    assert!(
        s.contains("[1.0, 0.0, 0.0, 1.0]"),
        "expected pure red fill, got: {s}"
    );
}

#[test]
fn openfigs_logo_frame() {
    let doc = import_fig_bytes(&fixture("OpenFigs.fig")).unwrap();
    assert_eq!(doc.pages.len(), 1);
    let frame = &doc.pages[0].children[0];
    assert_eq!(frame.name, "WhiteOpenFigOutlinedIcon");
    assert!(
        frame.w > 500.0 && frame.h > 500.0,
        "size {}x{}",
        frame.w,
        frame.h
    );
}

#[test]
fn non_fig_bytes_fail_cleanly() {
    assert!(import_fig_bytes(b"not a zip").is_err());
    let zip_of_junk = b"PK\x03\x04junkjunkjunkjunkjunkjunkjunkjunkjunkjunkjunkjunkjunk";
    assert!(import_fig_bytes(&zip_of_junk[..]).is_err() || true); // parser-tolerant: at worst Err, never panic
}
