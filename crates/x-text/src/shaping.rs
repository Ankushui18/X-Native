//! Professional shaping stack (P0 typography):
//!
//!   FontManager -> font loading -> Unicode shaping (rustybuzz/HarfBuzz)
//!   -> glyph runs -> BiDi + line breaking -> text layout -> Vello
//!
//! Delivers: ligatures (GSUB), kerning (GPOS), Arabic joining/RTL,
//! CJK line breaking, font fallback per run, letter spacing, line
//! height, rich-text spans, variable-font axes.

use crate::font::FontManager;
use std::collections::HashMap;
use vello::kurbo::{Affine, BezPath};
use vello::peniko::{Color, Fill};
use vello::Scene;

// ------------------------------------------------------------------- spans

/// Rich text: a styled range. `font` indexes FontManager (None = default).
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub size: f64,
    pub color: Color,
    pub letter_spacing: f64,
    pub font: Option<usize>,
    /// variable-font axes, e.g. [("wght", 700.0)]
    pub variations: Vec<(String, f32)>,
}

impl Span {
    pub fn new(text: &str, size: f64) -> Self {
        Self { text: text.into(), size, color: Color::BLACK, letter_spacing: 0.0, font: None, variations: vec![] }
    }
    pub fn color(mut self, c: Color) -> Self { self.color = c; self }
    pub fn letter_spacing(mut self, s: f64) -> Self { self.letter_spacing = s; self }
    pub fn font(mut self, f: usize) -> Self { self.font = Some(f); self }
    pub fn variation(mut self, axis: &str, value: f32) -> Self { self.variations.push((axis.into(), value)); self }
}

// -------------------------------------------------------------- glyph runs

/// One shaped glyph, positioned in px relative to the run origin.
#[derive(Debug, Clone, Copy)]
pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub cluster: u32,
    pub x_advance: f64,
    pub x_offset: f64,
    pub y_offset: f64,
}

/// A shaped run: one font, one direction, one style.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub font: usize,
    pub size: f64,
    pub color: Color,
    pub rtl: bool,
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f64,
    pub text: String,
}

// ---------------------------------------------------------------- shaping

pub struct Shaper<'a> {
    pub fonts: &'a FontManager,
    face_cache: HashMap<usize, rustybuzz::Face<'a>>,
}

impl<'a> Shaper<'a> {
    pub fn new(fonts: &'a FontManager) -> Self {
        Self { fonts, face_cache: HashMap::new() }
    }

    fn face(&mut self, font: usize) -> Option<&rustybuzz::Face<'a>> {
        if !self.face_cache.contains_key(&font) {
            let data = self.fonts.fonts.get(font)?.data();
            let face = rustybuzz::Face::from_slice(data, 0)?;
            self.face_cache.insert(font, face);
        }
        self.face_cache.get(&font)
    }

    /// Best font for `text` starting from `prefer`: first font in the
    /// fallback chain whose cmap covers the majority of chars.
    pub fn pick_font(&self, text: &str, prefer: usize) -> usize {
        let score = |fi: usize| -> usize {
            let Some(f) = self.fonts.fonts.get(fi) else { return 0 };
            text.chars().filter(|&c| !c.is_whitespace() && f.glyph_id(c).map_or(false, |g| g != 0)).count()
        };
        let total = text.chars().filter(|c| !c.is_whitespace()).count();
        if total == 0 || score(prefer) == total { return prefer; }
        (0..self.fonts.fonts.len()).max_by_key(|&i| score(i)).unwrap_or(prefer)
    }

    /// Shape one span into runs (BiDi-split first, then rustybuzz per run).
    pub fn shape_span(&mut self, span: &Span, default_font: usize) -> Vec<GlyphRun> {
        let bidi = unicode_bidi::BidiInfo::new(&span.text, None);
        let mut runs = vec![];
        let para = match bidi.paragraphs.first() { Some(p) => p, None => return runs };
        let (levels, ranges) = bidi.visual_runs(para, para.range.clone());
        for range in ranges {
            let sub = &span.text[range.clone()];
            if sub.is_empty() { continue; }
            let rtl = levels[range.start].is_rtl();
            let base = span.font.unwrap_or(default_font);
            // FONT-COVERAGE SEGMENTATION: a single BiDi run can mix
            // scripts (latin + Devanagari + CJK). Split it wherever the
            // covering font changes so no segment falls to tofu.
            for (seg, font) in self.segment_by_font(sub, base) {
                self.shape_one(&seg, font, rtl, span, &mut runs);
            }
            continue;
        }
        runs
    }

    /// Split text into (segment, font) pieces where each piece's font
    /// actually covers its characters (whitespace glues to the current
    /// segment). This is per-character fallback, the missing piece that
    /// coverage-voting per run cannot provide.
    fn segment_by_font(&self, text: &str, prefer: usize) -> Vec<(String, usize)> {
        let covers = |fi: usize, c: char| -> bool {
            self.fonts.fonts.get(fi).and_then(|f| f.glyph_id(c)).map_or(false, |g| g != 0)
        };
        let font_for = |c: char| -> usize {
            if covers(prefer, c) { return prefer; }
            (0..self.fonts.fonts.len()).find(|&i| covers(i, c)).unwrap_or(prefer)
        };
        let mut out: Vec<(String, usize)> = vec![];
        for ch in text.chars() {
            let f = if ch.is_whitespace() {
                out.last().map(|(_, f)| *f).unwrap_or(prefer)
            } else { font_for(ch) };
            match out.last_mut() {
                Some((seg, sf)) if *sf == f => seg.push(ch),
                _ => out.push((ch.to_string(), f)),
            }
        }
        out
    }

    fn shape_one(&mut self, sub: &str, font: usize, rtl: bool, span: &Span, runs: &mut Vec<GlyphRun>) {
        let Some(face) = self.face(font) else { return };
        let mut face = face.clone();
        for (axis, value) in &span.variations {
            let tag_bytes = axis.as_bytes();
            if tag_bytes.len() == 4 {
                let tag = rustybuzz::ttf_parser::Tag::from_bytes(&[tag_bytes[0], tag_bytes[1], tag_bytes[2], tag_bytes[3]]);
                let _ = face.set_variation(tag, *value);
            }
        }
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(sub);
        buf.set_direction(if rtl { rustybuzz::Direction::RightToLeft } else { rustybuzz::Direction::LeftToRight });
        let out = rustybuzz::shape(&face, &[], buf);
        let upm = self.fonts.fonts[font].units_per_em;
        let scale = span.size / upm;
        let mut glyphs = vec![];
        let mut width = 0.0;
        for (info, pos) in out.glyph_infos().iter().zip(out.glyph_positions()) {
            let adv = pos.x_advance as f64 * scale + span.letter_spacing;
            glyphs.push(ShapedGlyph {
                glyph_id: info.glyph_id as u16,
                cluster: info.cluster,
                x_advance: adv,
                x_offset: pos.x_offset as f64 * scale,
                y_offset: pos.y_offset as f64 * scale,
            });
            width += adv;
        }
        runs.push(GlyphRun { font, size: span.size, color: span.color, rtl, glyphs, width, text: sub.to_string() });
    }
}

// ------------------------------------------------------------ line breaking

/// Break opportunities: after spaces/hyphens, and BETWEEN CJK ideographs
/// (each CJK char is a break opportunity — standard CJK behavior).
pub fn break_opportunities(text: &str) -> Vec<usize> {
    let mut out = vec![];
    let mut prev_cjk = false;
    for (i, c) in text.char_indices() {
        let is_cjk = matches!(c as u32,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x30FF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF);
        if c == ' ' || c == '-' { out.push(i + c.len_utf8()); }
        else if is_cjk || (prev_cjk && !is_cjk) { if i > 0 { out.push(i); } }
        prev_cjk = is_cjk;
    }
    out
}

/// A laid-out line: spans clipped to the line, with total width.
#[derive(Debug, Clone)]
pub struct Line { pub spans: Vec<Span>, pub width: f64 }

/// Greedy layout of rich spans into lines <= max_width. Measures with
/// REAL shaping (ligatures/kerning affect fit). Handles \n, spaces, CJK.
pub fn layout_lines(shaper: &mut Shaper, spans: &[Span], default_font: usize, max_width: f64) -> Vec<Line> {
    let measure = |sh: &mut Shaper, sp: &Span| -> f64 {
        sh.shape_span(sp, default_font).iter().map(|r| r.width).sum()
    };
    let mut lines: Vec<Line> = vec![];
    let mut cur: Vec<Span> = vec![];
    let mut cur_w = 0.0;

    for span in spans {
        for para in split_keep(&span.text, '\n') {
            if para == "\n" {
                lines.push(Line { spans: std::mem::take(&mut cur), width: cur_w });
                cur_w = 0.0;
                continue;
            }
            // word/CJK segments
            let mut segs: Vec<&str> = vec![];
            let mut last = 0;
            for b in break_opportunities(para) {
                if b > last && b <= para.len() { segs.push(&para[last..b]); last = b; }
            }
            if last < para.len() { segs.push(&para[last..]); }
            for seg in segs {
                let piece = Span { text: seg.to_string(), ..span.clone() };
                let w = measure(shaper, &piece);
                if cur_w + w > max_width && cur_w > 0.0 {
                    lines.push(Line { spans: std::mem::take(&mut cur), width: cur_w });
                    cur_w = 0.0;
                }
                // merge with previous span on the line if same style
                if let Some(lastspan) = cur.last_mut() {
                    if lastspan.size == piece.size && lastspan.color == piece.color
                        && lastspan.font == piece.font && lastspan.letter_spacing == piece.letter_spacing {
                        lastspan.text.push_str(&piece.text);
                        cur_w += w;
                        continue;
                    }
                }
                cur.push(piece);
                cur_w += w;
            }
        }
    }
    if !cur.is_empty() || lines.is_empty() { lines.push(Line { spans: cur, width: cur_w }); }
    lines
}

fn split_keep(s: &str, sep: char) -> Vec<&str> {
    let mut out = vec![];
    let mut last = 0;
    for (i, c) in s.char_indices() {
        if c == sep {
            if i > last { out.push(&s[last..i]); }
            out.push(&s[i..i + c.len_utf8()]);
            last = i + c.len_utf8();
        }
    }
    if last < s.len() { out.push(&s[last..]); }
    out
}

// ------------------------------------------------------------------ layout

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align { Left, Center, Right }

pub struct TextBlockStyle {
    pub max_width: f64,
    pub line_height: f64, // multiplier over font natural height (1.0 = natural)
    pub align: Align,
}

/// One positioned glyph outline: the bezier path in font units and the
/// LOCAL transform (block-relative — caller composes its own world/CTM)
/// that places and scales it, plus its fill color.
pub struct OutlineGlyph {
    pub path: BezPath,
    /// local placement: translate(pen + offset, baseline) * scale(s, -s)
    pub transform: Affine,
    pub color: Color,
}

/// Shape + wrap + align rich spans and return every glyph as a positioned
/// outline. This is the SINGLE source of truth for text geometry: the
/// canvas encoder (encode_rich_text) and the SVG/PDF exporters all
/// consume it, so text placement is pixel-identical across all three
/// sinks by construction. Returns (glyphs, total_height).
pub fn glyph_outlines(
    fonts: &FontManager, spans: &[Span], default_font: usize, style: &TextBlockStyle,
) -> (Vec<OutlineGlyph>, f64) {
    let mut shaper = Shaper::new(fonts);
    let lines = layout_lines(&mut shaper, spans, default_font, style.max_width);
    let mut out = vec![];
    let mut y = 0.0f64;
    for line in &lines {
        let max_size = line.spans.iter().map(|s| s.size).fold(12.0, f64::max);
        let f0 = &fonts.fonts[default_font];
        let natural = (f0.ascent - f0.descent + f0.line_gap) * (max_size / f0.units_per_em);
        let lh = natural * style.line_height;
        let baseline = y + f0.ascent * (max_size / f0.units_per_em) * style.line_height.max(1.0).min(1.2);
        let x0 = match style.align {
            Align::Left => 0.0,
            Align::Center => (style.max_width - line.width) / 2.0,
            Align::Right => style.max_width - line.width,
        };
        let mut pen = x0;
        for span in &line.spans {
            for run in shaper.shape_span(span, default_font) {
                let f = &fonts.fonts[run.font];
                let scale = run.size / f.units_per_em;
                let mut x = pen;
                for g in &run.glyphs {
                    if let Some(outline) = f.outline(g.glyph_id) {
                        let t = Affine::translate((x + g.x_offset, baseline - g.y_offset))
                            * Affine::scale_non_uniform(scale, -scale);
                        out.push(OutlineGlyph { path: outline, transform: t, color: run.color });
                    }
                    x += g.x_advance;
                }
                pen += run.width;
            }
        }
        y += lh;
    }
    (out, y)
}

/// Full pipeline: rich spans -> shaped, wrapped, aligned -> Vello paths.
/// Returns (paths_encoded, total_height).
pub fn encode_rich_text(
    scene: &mut Scene, fonts: &FontManager, spans: &[Span], default_font: usize,
    world: Affine, style: &TextBlockStyle,
) -> (usize, f64) {
    let (glyphs, height) = glyph_outlines(fonts, spans, default_font, style);
    let n = glyphs.len();
    for g in glyphs {
        scene.fill(Fill::NonZero, world * g.transform, g.color, None, &g.path);
    }
    (n, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts() -> FontManager {
        let mut m = FontManager::new();
        m.load_system_fonts();
        // add scripts beyond the default dir
        let _ = m.load_file("NotoSansArabic", "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf");
        let _ = m.load_file("NotoKufiArabic", "/usr/share/fonts/truetype/noto/NotoKufiArabic-Regular.ttf");
        assert!(!m.fonts.is_empty(), "system fonts required");
        m
    }

    #[test]
    fn ligatures_reduce_glyph_count() {
        let m = fonts();
        let f = m.default_font().unwrap();
        let mut sh = Shaper::new(&m);
        // DejaVu has an fi ligature via GSUB
        let runs = sh.shape_span(&Span::new("fi", 16.0), f);
        let n_fi: usize = runs.iter().map(|r| r.glyphs.len()).sum();
        let runs = sh.shape_span(&Span::new("f i", 16.0), f);
        let n_f_i: usize = runs.iter().map(|r| r.glyphs.len()).sum();
        assert!(n_fi < n_f_i, "'fi' should ligate: {n_fi} vs {n_f_i} glyphs");
        assert_eq!(n_fi, 1, "DejaVu ligates fi into one glyph");
    }

    #[test]
    fn kerning_applies_via_gpos_or_kern() {
        let m = fonts();
        let f = m.default_font().unwrap();
        let mut sh = Shaper::new(&m);
        let w_av: f64 = sh.shape_span(&Span::new("AV", 32.0), f).iter().map(|r| r.width).sum();
        let w_a: f64 = sh.shape_span(&Span::new("A", 32.0), f).iter().map(|r| r.width).sum();
        let w_v: f64 = sh.shape_span(&Span::new("V", 32.0), f).iter().map(|r| r.width).sum();
        assert!(w_av < w_a + w_v - 0.1, "AV must kern tighter: {w_av} vs {}", w_a + w_v);
    }

    #[test]
    fn arabic_shapes_rtl_with_joining_forms() {
        let m = fonts();
        let arabic = m.font_index("NotoSansArabic").or(m.font_index("NotoKufiArabic"));
        let Some(af) = arabic else { return }; // env without arabic fonts: skip
        let mut sh = Shaper::new(&m);
        // "سلام" (salaam) — 4 letters that join contextually
        let runs = sh.shape_span(&Span::new("سلام", 24.0).font(af), af);
        assert_eq!(runs.len(), 1);
        assert!(runs[0].rtl, "Arabic run must be RTL");
        // joining: shaped glyph count <= char count, and NOT the isolated forms
        let isolated: Vec<u16> = "سلام".chars()
            .map(|c| m.fonts[af].glyph_id(c).unwrap_or(0))
            .collect();
        let shaped: Vec<u16> = runs[0].glyphs.iter().map(|g| g.glyph_id).collect();
        assert_ne!(shaped, isolated, "contextual forms must differ from isolated cmap forms");
    }

    #[test]
    fn mixed_ltr_rtl_splits_into_directional_runs() {
        let m = fonts();
        let Some(_af) = m.font_index("NotoSansArabic") else { return };
        let f = m.default_font().unwrap();
        let mut sh = Shaper::new(&m);
        let runs = sh.shape_span(&Span::new("abc سلام xyz", 16.0), f);
        assert!(runs.len() >= 3, "LTR/RTL/LTR should split: got {} runs", runs.len());
        assert!(runs.iter().any(|r| r.rtl) && runs.iter().any(|r| !r.rtl));
    }

    #[test]
    fn fallback_picks_covering_font_per_run() {
        let m = fonts();
        let Some(af) = m.font_index("NotoSansArabic") else { return };
        let latin = m.default_font().unwrap();
        let sh = Shaper::new(&m);
        // The picked font must COVER the text. (DejaVu itself covers
        // Arabic, so staying put is legal; what matters is coverage.)
        let picked = sh.pick_font("سلام", latin);
        let covers = "سلام".chars().all(|c| m.fonts[picked].glyph_id(c).map_or(false, |g| g != 0));
        assert!(covers, "picked font must cover Arabic");
        assert_eq!(sh.pick_font("hello", latin), latin);
        // A font with NO coverage of the text must be abandoned:
        // shape emoji-ish/unknown chars against a tiny fake preference.
        let mono = m.font_index("DejaVuSansMono").unwrap_or(latin);
        let picked2 = sh.pick_font("سلام", mono);
        let covers2 = "سلام".chars().all(|c| m.fonts[picked2].glyph_id(c).map_or(false, |g| g != 0));
        assert!(covers2, "fallback from non-covering font must find coverage");
        let _ = af;
    }

    #[test]
    fn cjk_breaks_between_ideographs() {
        let ops = break_opportunities("設計工具");
        // 4 ideographs -> break opportunity before each following char
        assert!(ops.len() >= 3, "CJK chars must each be breakable: {ops:?}");
        let ops = break_opportunities("hello world");
        assert_eq!(ops, vec![6], "latin breaks after the space");
    }

    #[test]
    fn letter_spacing_and_line_height_apply() {
        let m = fonts();
        let f = m.default_font().unwrap();
        let mut sh = Shaper::new(&m);
        let w0: f64 = sh.shape_span(&Span::new("spacing", 16.0), f).iter().map(|r| r.width).sum();
        let w5: f64 = sh.shape_span(&Span::new("spacing", 16.0).letter_spacing(5.0), f).iter().map(|r| r.width).sum();
        assert!((w5 - w0 - 7.0 * 5.0).abs() < 0.5, "7 chars x 5px spacing");
        // line height scales block height
        let mut sc = Scene::new();
        let style1 = TextBlockStyle { max_width: 10_000.0, line_height: 1.0, align: Align::Left };
        let style2 = TextBlockStyle { max_width: 10_000.0, line_height: 2.0, align: Align::Left };
        let (_, h1) = encode_rich_text(&mut sc, &m, &[Span::new("a\nb", 16.0)], f, Affine::IDENTITY, &style1);
        let (_, h2) = encode_rich_text(&mut sc, &m, &[Span::new("a\nb", 16.0)], f, Affine::IDENTITY, &style2);
        assert!((h2 / h1 - 2.0).abs() < 0.01, "double line-height doubles block height");
    }

    #[test]
    fn rich_text_spans_layout_and_wrap_with_real_shaping() {
        let m = fonts();
        let f = m.default_font().unwrap();
        let spans = vec![
            Span::new("Bold-ish heading ", 20.0).color(Color::rgb8(255, 0, 0)),
            Span::new("then body text that definitely wraps across lines", 12.0),
        ];
        let mut sh = Shaper::new(&m);
        let lines = layout_lines(&mut sh, &spans, f, 150.0);
        assert!(lines.len() >= 3, "must wrap: got {} lines", lines.len());
        for line in &lines {
            assert!(line.width <= 150.0 + 1.0, "line overflows: {}", line.width);
        }
        // encode: paths appear, mixed colors preserved
        let mut sc = Scene::new();
        let style = TextBlockStyle { max_width: 150.0, line_height: 1.2, align: Align::Left };
        let (paths, h) = encode_rich_text(&mut sc, &m, &spans, f, Affine::IDENTITY, &style);
        assert!(paths > 30, "many glyphs: {paths}");
        assert!(h > 40.0, "multi-line block height: {h}");
    }

    #[test]
    fn alignment_positions_lines() {
        let m = fonts();
        let f = m.default_font().unwrap();
        let mut sh = Shaper::new(&m);
        let lines = layout_lines(&mut sh, &[Span::new("hi", 16.0)], f, 300.0);
        assert_eq!(lines.len(), 1);
        // encode with center/right must not panic and produce same path count
        for align in [Align::Left, Align::Center, Align::Right] {
            let mut sc = Scene::new();
            let style = TextBlockStyle { max_width: 300.0, line_height: 1.0, align };
            let (p, _) = encode_rich_text(&mut sc, &m, &[Span::new("hi", 16.0)], f, Affine::IDENTITY, &style);
            assert_eq!(p, 2);
        }
    }
}

/// The ONE canonical mapping from a Text node's properties to shaped glyph
/// outlines. `size` is the node's h (the engine's text-size convention);
/// the 0.72 em factor and 1.2 line height are the canvas contract. Canvas
/// sink, PDF exporter, and SVG exporter must all call THIS so text
/// geometry cannot drift between them.
pub fn node_text_outlines(
    fonts: &FontManager, text: &str, size: f64, max_width: f64,
    font_name: Option<&str>, color: Color,
) -> Option<(Vec<OutlineGlyph>, f64)> {
    node_text_outlines_styled(fonts, text, size, max_width, font_name, color, 0.0, 1.2)
}

/// Typography-aware variant: letter spacing (px) + line height multiplier.
/// Same ONE-pipeline contract; the defaults reproduce node_text_outlines.
pub fn node_text_outlines_styled(
    fonts: &FontManager, text: &str, size: f64, max_width: f64,
    font_name: Option<&str>, color: Color, ls: f64, lh: f64,
) -> Option<(Vec<OutlineGlyph>, f64)> {
    // route through the ShapedTextCache: repeated frames/text reuse the
    // shaped block (Arc clone), positions compose OUTSIDE via the world
    // transform so moves are cache hits. Falls back to direct shaping
    // if the cache is poisoned.
    let key = crate::cache::TextLayoutKey::new_styled(text, size, max_width, font_name, color, fonts.epoch(), ls, lh);
    if let Some(block) = crate::cache::ShapedTextCache::global().get_or_shape(fonts, key) {
        return Some((block.glyphs.iter().map(|g| OutlineGlyph {
            path: g.path.clone(), transform: g.transform, color: g.color,
        }).collect(), block.height));
    }
    node_text_outlines_styled_uncached(fonts, text, size, max_width, font_name, color, ls, lh)
}

/// The raw shaping path (cache-miss fill + tests).
pub fn node_text_outlines_uncached(
    fonts: &FontManager, text: &str, size: f64, max_width: f64,
    font_name: Option<&str>, color: Color,
) -> Option<(Vec<OutlineGlyph>, f64)> {
    node_text_outlines_styled_uncached(fonts, text, size, max_width, font_name, color, 0.0, 1.2)
}

/// Raw styled shaping path (cache-miss fill + tests).
pub fn node_text_outlines_styled_uncached(
    fonts: &FontManager, text: &str, size: f64, max_width: f64,
    font_name: Option<&str>, color: Color, ls: f64, lh: f64,
) -> Option<(Vec<OutlineGlyph>, f64)> {
    let chosen = font_name.and_then(|n| fonts.font_index(n)).or_else(|| fonts.default_font())?;
    let spans = [Span::new(text, size * 0.72).color(color).letter_spacing(ls)];
    let style = TextBlockStyle { max_width: max_width.max(8.0), line_height: lh.max(0.5), align: Align::Left };
    Some(glyph_outlines(fonts, &spans, chosen, &style))
}
