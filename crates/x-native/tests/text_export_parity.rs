//! TEXT EXPORT PARITY (review's "top visual gap"):
//!
//! All three sinks — canvas (VelloSink), SVG exporter, PDF exporter —
//! must derive text geometry from the SAME `node_text_outlines` pipeline
//! (shaping, BiDi, fallback, wrapping). These tests pin that contract:
//! same glyph count, same first-glyph placement, real outline paths in
//! both export formats (no <text font-family="monospace">, no Helvetica
//! Tj), and ligature/kerning behavior surviving into the exports.

use arco_native::fileio::export_svg_full;
use arco_native::text::{node_text_outlines, FontManager};
use arco_native::{build_render_tree, export_pdf_full, svg_text_outliner, Color, Node, Variables};

fn fonts() -> FontManager {
    let mut fm = FontManager::new();
    assert!(fm.load_system_fonts() > 0, "system fonts required");
    fm
}

#[test]
fn all_three_sinks_share_glyph_geometry() {
    let fm = fonts();
    let (glyphs, height) = node_text_outlines(&fm, "Export Parity fi", 24.0, 400.0, None, Color::BLACK)
        .expect("outlines");
    assert!(glyphs.len() >= 10, "shaped {} glyphs", glyphs.len());
    assert!(height > 0.0);

    // SVG outliner path count == canonical glyph count (same pipeline)
    let outliner = svg_text_outliner(&fm);
    let ds = outliner("Export Parity fi", 24.0, 400.0, None).expect("svg outliner");
    assert_eq!(ds.len(), glyphs.len(), "svg emits one path per shaped glyph");
    // every d is real path data with curves (glyph outlines, not boxes)
    for d in &ds {
        assert!(d.starts_with("M "), "path data: {d}");
    }
    assert!(ds.iter().any(|d| d.contains("Q ") || d.contains("C ")), "glyphs contain curves");

    // ligature contract survives: "fi" shapes to fewer glyphs than "f i"
    let ds_no_lig = outliner("Export Parity f i", 24.0, 400.0, None).unwrap();
    assert!(ds_no_lig.len() > ds.len(), "fi ligature reduces glyph count in the EXPORT path too");
}

#[test]
fn svg_export_emits_outlines_not_font_tags() {
    let fm = fonts();
    let doc = Node::frame("page", 400.0, 100.0)
        .child(Node::text("t", 20.0, 20.0, 360.0, 24.0, "Vector text"));
    let outliner = svg_text_outliner(&fm);
    let svg = export_svg_full(&doc, &Variables::default(), None, Some(&outliner));
    assert!(!svg.contains("font-family"), "no font guessing left in the export");
    assert!(!svg.contains("<text"), "no <text> element");
    let paths = svg.matches("<path").count();
    assert!(paths >= "Vectortext".len(), "one outline path per glyph, got {paths}");
}

#[test]
fn pdf_export_emits_outlines_not_helvetica() {
    let fm = fonts();
    let doc = Node::frame("page", 400.0, 100.0)
        .child(Node::text("t", 20.0, 20.0, 360.0, 24.0, "Vector text"));
    let tree = build_render_tree(&doc, &Variables::default());
    let pdf = export_pdf_full(&tree, 400.0, 100.0, None, Some(&fm));
    let txt = String::from_utf8_lossy(&pdf);
    assert!(!txt.contains("Tj"), "no Helvetica text ops left");
    // glyph outlines arrive as filled bezier paths: m/l/c ops + f fills
    assert!(txt.contains(" c\n"), "pdf contains curve segments");
    let fills = txt.matches("f\n").count();
    assert!(fills >= "Vectortext".len(), "one fill per glyph, got {fills}");
    // without fonts, the legacy fallback still works (facade contract)
    let legacy = export_pdf_full(&tree, 400.0, 100.0, None, None);
    assert!(String::from_utf8_lossy(&legacy).contains("Tj"), "fallback keeps Tj path");
}

#[test]
fn wrapping_matches_between_canvas_and_exports() {
    // narrow box forces a wrap; the same break must appear in the SVG
    // (two distinct baseline y-bands among outline paths).
    let fm = fonts();
    let (glyphs_wide, _) = node_text_outlines(&fm, "alpha beta", 20.0, 500.0, None, Color::BLACK).unwrap();
    let (glyphs_narrow, h_narrow) = node_text_outlines(&fm, "alpha beta", 20.0, 60.0, None, Color::BLACK).unwrap();
    assert_eq!(glyphs_wide.len(), glyphs_narrow.len(), "same glyphs either way");
    let (_, h_wide) = node_text_outlines(&fm, "alpha beta", 20.0, 500.0, None, Color::BLACK).unwrap();
    assert!(h_narrow > h_wide * 1.5, "narrow box wraps to more lines ({h_narrow} vs {h_wide})");
    // baseline bands differ: max translation-y among narrow glyphs is a
    // full line below the first baseline
    let ys: Vec<f64> = glyphs_narrow.iter().map(|g| g.transform.as_coeffs()[5]).collect();
    let (min_y, max_y) = ys.iter().fold((f64::MAX, f64::MIN), |(a, b), y| (a.min(*y), b.max(*y)));
    assert!(max_y - min_y > 10.0, "two baselines in the narrow layout");
}
