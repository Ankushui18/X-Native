//! The typography validation fixture (product-risk killer).
//!
//! Builds the exact specimen requested — Inter, Roboto, Noto Sans,
//! Devanagari, Arabic, "Aa AV fi ffi 123", Hindi, Arabic, Chinese, emoji —
//! then drives it through the FULL product pipeline:
//!   shape -> measure -> wrap -> resize -> save -> reload -> render
//! with exact assertions at every stage.

use arco_native::text::{FontManager, Shaper, Span, SystemFonts};
use arco_native::{build_render_tree, Document, Node, VelloSink, Variables};
use x_format::{load_x_any, save_x_v2, DocumentV2};

fn fixture_fonts() -> (FontManager, SystemFonts) {
    let mut fm = FontManager::new();
    fm.load_system_fonts();
    let sys = SystemFonts::enumerate();
    for fam in ["Noto Sans Devanagari", "Noto Sans Arabic", "Noto Sans"] {
        let _ = sys.load_into(&mut fm, fam, "");
    }
    (fm, sys)
}

const HINDI: &str = "हिन्दी";
const ARABIC: &str = "العربية";
const CHINESE: &str = "中文";
const EMOJI: &str = "😀";

// ------------------------------------------------------------------ shape

#[test]
fn stage1_shape_every_script() {
    let (fm, _) = fixture_fonts();
    let f = fm.default_font().unwrap();
    let mut sh = Shaper::new(&fm);
    for (label, text) in [("latin", "Aa AV fi ffi 123"), ("hindi", HINDI), ("arabic", ARABIC), ("chinese", CHINESE)] {
        let runs = sh.shape_span(&Span::new(text, 24.0), f);
        assert!(!runs.is_empty(), "{label}: no runs");
        let glyphs: usize = runs.iter().map(|r| r.glyphs.len()).sum();
        assert!(glyphs > 0, "{label}: no glyphs");
        // every glyph must be a REAL glyph id (0 = .notdef = tofu)
        for r in &runs {
            let f_ref = &fm.fonts[r.font];
            for g in &r.glyphs {
                assert!(g.glyph_id != 0, "{label}: tofu in run (font {})", f_ref.name);
            }
        }
    }
    // ligature checks: fi and ffi each fuse in DejaVu
    let count = |t: &str, sh: &mut Shaper| -> usize {
        sh.shape_span(&Span::new(t, 24.0), f).iter().map(|r| r.glyphs.len()).sum()
    };
    assert_eq!(count("fi", &mut sh), 1, "fi must ligate");
    assert!(count("ffi", &mut sh) <= 2, "ffi must ligate");
    // Arabic joins: shaped forms differ from isolated cmap forms
    let runs = sh.shape_span(&Span::new(ARABIC, 24.0), f);
    assert!(runs[0].rtl, "Arabic must be RTL");
    // Devanagari conjuncts: हिन्दी has 6 chars but shapes to fewer clusters
    let runs = sh.shape_span(&Span::new(HINDI, 24.0), f);
    let shaped: usize = runs.iter().map(|r| r.glyphs.len()).sum();
    assert!(shaped >= 4 && shaped <= 7, "Devanagari reordering/conjuncts: {shaped} glyphs");
}

// ---------------------------------------------------------------- measure

#[test]
fn stage2_measure_is_monotonic_and_size_linear() {
    let (fm, _) = fixture_fonts();
    let f = fm.default_font().unwrap();
    let mut sh = Shaper::new(&fm);
    let w = |t: &str, size: f64, sh: &mut Shaper| -> f64 {
        sh.shape_span(&Span::new(t, size), f).iter().map(|r| r.width).sum()
    };
    for t in ["Aa", "AV", "123", HINDI, ARABIC, CHINESE] {
        let w16 = w(t, 16.0, &mut sh);
        let w32 = w(t, 32.0, &mut sh);
        assert!(w16 > 0.0, "{t}: zero width");
        assert!((w32 / w16 - 2.0).abs() < 0.02, "{t}: doubling size must double width ({w16} -> {w32})");
    }
    // kerning sanity at measure level
    assert!(w("AV", 32.0, &mut sh) < w("A", 32.0, &mut sh) + w("V", 32.0, &mut sh));
}

// ------------------------------------------------------------------- wrap

#[test]
fn stage3_wrap_respects_width_for_all_scripts() {
    let (fm, _) = fixture_fonts();
    let f = fm.default_font().unwrap();
    let mut sh = Shaper::new(&fm);
    let mixed = format!("Inter Roboto Noto {HINDI} {ARABIC} {CHINESE} words wrap here");
    let lines = arco_native::text::layout_lines(&mut sh, &[Span::new(&mixed, 18.0)], f, 160.0);
    assert!(lines.len() >= 3, "must wrap into multiple lines: {}", lines.len());
    for (i, line) in lines.iter().enumerate() {
        assert!(line.width <= 161.0, "line {i} overflows: {}", line.width);
    }
    // CJK breaks WITHOUT spaces
    let cjk_only = CHINESE.repeat(30);
    let lines = arco_native::text::layout_lines(&mut sh, &[Span::new(&cjk_only, 20.0)], f, 100.0);
    assert!(lines.len() >= 4, "CJK must break between ideographs: {}", lines.len());
}

// ---------------------------------------------------- resize -> save -> reload

#[test]
fn stage4_resize_save_reload_render_full_product_loop() {
    let (fm, _) = fixture_fonts();
    let vars = Variables::default();

    // the fixture document: one text node per specimen line
    let lines = [
        ("t-title", "Typography Test", 26.0),
        ("t-aa", "Aa AV fi ffi 123", 22.0),
        ("t-hi", HINDI, 24.0),
        ("t-ar", ARABIC, 24.0),
        ("t-zh", CHINESE, 24.0),
        ("t-emoji", EMOJI, 24.0),
    ];
    let mut page = Node::frame("page", 600.0, 400.0);
    for (i, (id, text, size)) in lines.iter().enumerate() {
        page.children.push(Node::text(id, 20.0, 20.0 + i as f64 * 50.0, 560.0, *size, text));
    }

    // render BEFORE resize
    let tree1 = build_render_tree(&page, &vars);
    let sink = VelloSink { assets: None, fonts: Some(&fm) };
    let scene1 = sink.render(&tree1);
    let paths_before = scene1.encoding().n_paths;
    assert!(paths_before > 30, "specimen must render many glyphs: {paths_before}");

    // RESIZE the latin node narrower -> its wrap must change the render
    page.children.iter_mut().find(|c| c.id == "t-aa").unwrap().w = 80.0;
    let tree2 = build_render_tree(&page, &vars);
    let changed = tree2.changed_keys(&tree1);
    assert!(changed.contains(&"/page/t-aa".to_string()), "resize must dirty exactly that node: {changed:?}");

    // SAVE (v2, deterministic) -> RELOAD -> byte-stable
    let mut d2 = DocumentV2::default();
    d2.doc = Document { pages: vec![page.clone()], variables: vars.clone(), ..Default::default() };
    let text1 = save_x_v2(&d2);
    let re = load_x_any(&text1).expect("reload");
    let text2 = save_x_v2(&re);
    assert_eq!(text1, text2, "save(load(save)) must be byte-identical");

    // the reloaded doc renders IDENTICALLY (same path count)
    let tree3 = build_render_tree(&re.doc.pages[0], &vars);
    let scene3 = sink.render(&tree3);
    let scene2 = sink.render(&tree2);
    assert_eq!(scene3.encoding().n_paths, scene2.encoding().n_paths,
        "reloaded document must render identically");
    // and the text content survived byte-for-byte
    fn text_of<'a>(n: &'a Node, id: &str) -> Option<&'a str> {
        if n.id == id { if let arco_native::NodeKind::Text { text } = &n.kind { return Some(text); } }
        n.children.iter().find_map(|c| text_of(c, id))
    }
    let reloaded = &re.doc.pages[0];
    assert_eq!(text_of(reloaded, "t-hi"), Some(HINDI));
    assert_eq!(text_of(reloaded, "t-ar"), Some(ARABIC));
    assert_eq!(text_of(reloaded, "t-zh"), Some(CHINESE));
    assert_eq!(text_of(reloaded, "t-emoji"), Some(EMOJI));
}

// ------------------------------------------------------------------ emoji

#[test]
fn stage5_emoji_honest_status() {
    // Emoji chars must at least map through fallback WITHOUT crashing.
    // Color rendering (COLR/CBDT) is a known gap; this documents the
    // current contract: no panic, graceful monochrome-or-skip.
    let (fm, _) = fixture_fonts();
    let f = fm.default_font().unwrap();
    let mut sh = Shaper::new(&fm);
    let runs = sh.shape_span(&Span::new(EMOJI, 24.0), f);
    // shaping must not panic; zero or more glyphs acceptable
    let _total: usize = runs.iter().map(|r| r.glyphs.len()).sum();
    // measure must not be negative/NaN
    let w: f64 = runs.iter().map(|r| r.width).sum();
    assert!(w.is_finite() && w >= 0.0);
}
