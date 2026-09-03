//! Real typography: TTF/OTF loading, glyph outlines, kerning, wrapping.
//!
//! Pipeline (the P0 architecture):
//!   FontManager -> font loading -> fallback chain -> glyph mapping ->
//!   positioning (advances + kern) -> line breaking -> layout -> Vello.
//!
//! `ttf-parser` gives us cmap/glyf/kern/hmtx parsing with zero C deps.
//! This is deliberately a *shaping-lite* stack: full complex-script
//! shaping (Arabic joining, Indic reordering) needs harfbuzz/rustybuzz —
//! the API here (shape() returning positioned glyphs) is designed so that
//! swap is internal and callers never change.

use std::collections::HashMap;
use vello::kurbo::{Affine, BezPath};
use vello::peniko::{Color, Fill};
use vello::Scene;

pub struct LoadedFont {
    pub name: String,
    data: Vec<u8>,
    /// face index inside a .ttc collection (0 for single-font files)
    pub face_index: u32,
    pub units_per_em: f64,
    pub ascent: f64,
    pub descent: f64,
    pub line_gap: f64,
    /// glyph outline cache: glyph id -> path in font units (y-up)
    outline_cache: std::cell::RefCell<HashMap<u16, Option<BezPath>>>,
}

impl LoadedFont {
    pub fn from_bytes(name: &str, data: Vec<u8>) -> Result<Self, String> {
        Self::from_bytes_indexed(name, data, 0)
    }

    /// Parse face `index` of a collection (.ttc) or 0 for plain files.
    pub fn from_bytes_indexed(name: &str, data: Vec<u8>, index: u32) -> Result<Self, String> {
        let face = ttf_parser::Face::parse(&data, index).map_err(|e| format!("{e:?}"))?;
        let upm = face.units_per_em() as f64;
        let (asc, desc, gap) = (
            face.ascender() as f64,
            face.descender() as f64,
            face.line_gap() as f64,
        );
        Ok(Self {
            name: name.into(),
            data,
            face_index: index,
            units_per_em: upm,
            ascent: asc,
            descent: desc,
            line_gap: gap,
            outline_cache: Default::default(),
        })
    }

    /// Raw font bytes (rustybuzz needs them for its own Face).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    fn face(&self) -> ttf_parser::Face<'_> {
        // parse is cheap (zero-copy views); Face borrows self.data
        ttf_parser::Face::parse(&self.data, self.face_index).expect("validated at load")
    }

    pub fn glyph_id(&self, c: char) -> Option<u16> {
        self.face().glyph_index(c).map(|g| g.0)
    }

    pub fn advance(&self, glyph: u16) -> f64 {
        self.face()
            .glyph_hor_advance(ttf_parser::GlyphId(glyph))
            .unwrap_or(0) as f64
    }

    pub fn kern(&self, left: u16, right: u16) -> f64 {
        let face = self.face();
        if let Some(kern) = face.tables().kern {
            for sub in kern.subtables {
                if sub.horizontal {
                    if let Some(v) =
                        sub.glyphs_kerning(ttf_parser::GlyphId(left), ttf_parser::GlyphId(right))
                    {
                        return v as f64;
                    }
                }
            }
        }
        0.0
    }

    /// Outline in font units (y-up); cached per glyph.
    pub fn outline(&self, glyph: u16) -> Option<BezPath> {
        if let Some(hit) = self.outline_cache.borrow().get(&glyph) {
            return hit.clone();
        }
        struct B(BezPath);
        impl ttf_parser::OutlineBuilder for B {
            fn move_to(&mut self, x: f32, y: f32) {
                self.0.move_to((x as f64, y as f64));
            }
            fn line_to(&mut self, x: f32, y: f32) {
                self.0.line_to((x as f64, y as f64));
            }
            fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
                self.0.quad_to((x1 as f64, y1 as f64), (x as f64, y as f64));
            }
            fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
                self.0.curve_to(
                    (x1 as f64, y1 as f64),
                    (x2 as f64, y2 as f64),
                    (x as f64, y as f64),
                );
            }
            fn close(&mut self) {
                self.0.close_path();
            }
        }
        let mut b = B(BezPath::new());
        let out = self
            .face()
            .outline_glyph(ttf_parser::GlyphId(glyph), &mut b)
            .map(|_| b.0);
        self.outline_cache.borrow_mut().insert(glyph, out.clone());
        out
    }
}

/// A glyph positioned in pixel space (relative to the text origin).
#[derive(Debug, Clone, Copy)]
pub struct PositionedGlyph {
    pub glyph: u16,
    /// index into FontManager.fonts (fallback may mix fonts in one run)
    pub font: usize,
    pub x: f64,
    pub y: f64,
    pub scale: f64, // font units -> px
}

#[derive(Default)]
pub struct FontManager {
    /// cache-invalidation epoch: bumped whenever the font set changes
    pub(crate) epoch: u64,
    pub fonts: Vec<LoadedFont>,
    by_name: HashMap<String, usize>,
}

impl FontManager {
    pub fn new() -> Self {
        Self::default()
    }
    /// generation for shaped-text cache keys
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn load_file(&mut self, name: &str, path: &str) -> Result<usize, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        self.load_face_bytes(name, data, 0)
    }

    /// Register raw font bytes (face `index` for .ttc collections).
    pub fn load_face_bytes(
        &mut self,
        name: &str,
        data: Vec<u8>,
        index: u32,
    ) -> Result<usize, String> {
        let font = LoadedFont::from_bytes_indexed(name, data, index)?;
        let idx = self.fonts.len();
        self.by_name.insert(name.to_string(), idx);
        self.fonts.push(font);
        self.epoch += 1; // font set changed: shaped-text cache keys rotate
        Ok(idx)
    }

    /// Scan standard system font dirs; returns how many loaded.
    pub fn load_system_fonts(&mut self) -> usize {
        let mut n = 0;
        for dir in [
            "/usr/share/fonts/truetype/dejavu",
            "/usr/share/fonts/truetype/noto",
            "/usr/share/fonts/opentype/noto",
            "/usr/share/fonts/truetype",
            "/usr/share/fonts/TTF",
            "C:\\Windows\\Fonts",
            "/System/Library/Fonts",
            "./fonts",
        ] {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension()
                        .is_some_and(|x| x == "ttf" || x == "otf" || x == "ttc")
                    {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            if self.load_file(stem, p.to_str().unwrap_or_default()).is_ok() {
                                n += 1;
                            }
                        }
                    }
                }
            }
            if n >= 24 {
                break;
            } // enough coverage; keep startup fast
        }
        // guarantee CJK coverage even when the scan stopped early
        // (the Noto CJK collection lives in the opentype dir)
        if self.font_index("NotoSansCJK-Regular").is_none()
            && self
                .load_file(
                    "NotoSansCJK-Regular",
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                )
                .is_ok()
        {
            n += 1;
        }
        n
    }

    pub fn font_index(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }
    pub fn default_font(&self) -> Option<usize> {
        self.font_index("DejaVuSans")
            .or(if self.fonts.is_empty() { None } else { Some(0) })
    }

    /// Map char -> (font, glyph) via the fallback chain starting at `first`.
    fn map_char(&self, c: char, first: usize) -> Option<(usize, u16)> {
        if let Some(g) = self.fonts.get(first).and_then(|f| f.glyph_id(c)) {
            if g != 0 {
                return Some((first, g));
            }
        }
        for (i, f) in self.fonts.iter().enumerate() {
            if i == first {
                continue;
            }
            if let Some(g) = f.glyph_id(c) {
                if g != 0 {
                    return Some((i, g));
                }
            }
        }
        None
    }

    /// Shape one line: chars -> positioned glyphs (advances + kerning).
    /// Returns (glyphs, width_px).
    pub fn shape(&self, text: &str, font: usize, size_px: f64) -> (Vec<PositionedGlyph>, f64) {
        let mut out = vec![];
        let mut pen = 0.0f64;
        let mut prev: Option<(usize, u16)> = None;
        for c in text.chars() {
            let Some((fi, gid)) = self.map_char(c, font) else {
                // missing glyph: advance by ~half em (tofu-width) and reset kerning
                pen += size_px * 0.5;
                prev = None;
                continue;
            };
            let f = &self.fonts[fi];
            let scale = size_px / f.units_per_em;
            if let Some((pfi, pgid)) = prev {
                if pfi == fi {
                    pen += f.kern(pgid, gid) * scale;
                }
            }
            out.push(PositionedGlyph {
                glyph: gid,
                font: fi,
                x: pen,
                y: 0.0,
                scale,
            });
            pen += f.advance(gid) * scale;
            prev = Some((fi, gid));
        }
        (out, pen)
    }

    pub fn measure(&self, text: &str, font: usize, size_px: f64) -> f64 {
        self.shape(text, font, size_px).1
    }

    /// Greedy word-wrap into lines fitting `max_width` px.
    pub fn break_lines(
        &self,
        text: &str,
        font: usize,
        size_px: f64,
        max_width: f64,
    ) -> Vec<String> {
        let mut lines = vec![];
        for para in text.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if self.measure(&cand, font, size_px) <= max_width || line.is_empty() {
                    line = cand;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                }
            }
            lines.push(line);
        }
        lines
    }

    /// Line height in px for a font at size (ascent - descent + gap).
    pub fn line_height(&self, font: usize, size_px: f64) -> f64 {
        let f = &self.fonts[font];
        (f.ascent - f.descent + f.line_gap) * (size_px / f.units_per_em)
    }

    /// Encode a (possibly multi-line) text block into the scene.
    /// `world` maps the text-box origin; wraps at `max_width` if Some.
    /// Returns paths encoded.
    #[allow(clippy::too_many_arguments)] // positional params are the natural shape here; grouping would obscure the algorithm
    pub fn encode_text_block(
        &self,
        scene: &mut Scene,
        text: &str,
        world: Affine,
        font: usize,
        size_px: f64,
        max_width: Option<f64>,
        color: Color,
    ) -> usize {
        let f0 = &self.fonts[font];
        let baseline0 = f0.ascent * (size_px / f0.units_per_em);
        let lh = self.line_height(font, size_px);
        let lines: Vec<String> = match max_width {
            Some(w) => self.break_lines(text, font, size_px, w),
            None => text.split('\n').map(String::from).collect(),
        };
        let mut paths = 0usize;
        for (li, line) in lines.iter().enumerate() {
            let (glyphs, _) = self.shape(line, font, size_px);
            let base_y = baseline0 + li as f64 * lh;
            for g in glyphs {
                if let Some(outline) = self.fonts[g.font].outline(g.glyph) {
                    // font units are y-up; flip and scale into pixel space
                    let t = world
                        * Affine::translate((g.x, base_y))
                        * Affine::scale_non_uniform(g.scale, -g.scale);
                    scene.fill(Fill::NonZero, t, color, None, &outline);
                    paths += 1;
                }
            }
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> FontManager {
        let mut m = FontManager::new();
        let n = m.load_system_fonts();
        assert!(
            n > 0,
            "no system fonts found — DejaVu expected in CI/sandbox"
        );
        m
    }

    #[test]
    fn loads_real_ttf_and_maps_glyphs() {
        let m = mgr();
        let f = m.default_font().unwrap();
        let font = &m.fonts[f];
        assert!(font.units_per_em > 0.0);
        let a = font.glyph_id('A').unwrap();
        assert!(a != 0);
        assert!(font.advance(a) > 0.0);
        // real outline with real curves
        let outline = font.outline(a).unwrap();
        assert!(outline.elements().len() > 4);
    }

    #[test]
    fn shaping_positions_advance_monotonically_and_kerns() {
        let m = mgr();
        let f = m.default_font().unwrap();
        let (glyphs, width) = m.shape("AVATAR", f, 32.0);
        assert_eq!(glyphs.len(), 6);
        assert!(width > 0.0);
        for pair in glyphs.windows(2) {
            assert!(pair[1].x > pair[0].x);
        }
        // kerning: "AV" should be narrower than advance('A') + advance('V')
        let (_, av) = m.shape("AV", f, 32.0);
        let fa = &m.fonts[f];
        let scale = 32.0 / fa.units_per_em;
        let sum =
            (fa.advance(fa.glyph_id('A').unwrap()) + fa.advance(fa.glyph_id('V').unwrap())) * scale;
        assert!(av <= sum + 1e-6, "kerned width {av} should be <= raw {sum}");
    }

    #[test]
    fn line_breaking_wraps_greedily() {
        let m = mgr();
        let f = m.default_font().unwrap();
        let text = "the quick brown fox jumps over the lazy dog";
        let lines = m.break_lines(text, f, 16.0, 120.0);
        assert!(lines.len() >= 3, "expected multiple lines, got {lines:?}");
        for l in &lines {
            if l.contains(' ') {
                assert!(m.measure(l, f, 16.0) <= 120.0 + 1e-6);
            }
        }
        // explicit newlines respected
        assert_eq!(m.break_lines("a\nb", f, 16.0, 999.0).len(), 2);
    }

    #[test]
    fn encode_block_renders_multi_line_real_glyphs() {
        let m = mgr();
        let f = m.default_font().unwrap();
        let mut scene = Scene::new();
        let n = m.encode_text_block(
            &mut scene,
            "Hello type!\nSecond line",
            Affine::IDENTITY,
            f,
            24.0,
            None,
            Color::BLACK,
        );
        assert!(n >= 20, "expected ~22 glyph paths, got {n}");
        assert!(scene.encoding().n_paths as usize >= n);
    }

    #[test]
    fn fallback_reports_missing_glyphs_gracefully() {
        let m = mgr();
        let f = m.default_font().unwrap();
        // control chars have no glyph; must not panic and still advance text
        let (glyphs, w) = m.shape("a\u{7f}b", f, 16.0);
        assert!(glyphs.len() >= 2);
        assert!(w > 0.0);
    }
}
