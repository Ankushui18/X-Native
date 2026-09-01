//! ShapedTextCache — the review's #1 optimization target.
//!
//! Shaping (rustybuzz + BiDi + fallback + wrapping) ran per Glyphs
//! command per frame; in the mixed benchmark that made encode dominate
//! (10k = 138ms). This cache memoizes the ENTIRE shaped block:
//!
//!   TextNode → TextLayoutKey → ShapedTextCache → { glyph runs,
//!   positions, fallback decisions, metrics } → render
//!
//! Key contract (review): everything that changes shaping is in the key —
//! font binding (family+weight+style resolved through the fallback chain),
//! size, letter spacing, line height, text content, width constraint,
//! direction/script (implicit in content), features (via font binding).
//! NOT in the key: node position — output glyphs are BLOCK-LOCAL and the
//! world transform composes outside, so moving text is a cache HIT by
//! construction.
//!
//! Invalidation: content/font/size/width changes make a different key
//! (natural miss). `epoch` bumps flush everything (font hot-reload).
//! Entries are Arc'd: hits clone a pointer, not glyph vectors.

use crate::font::FontManager;
use crate::shaping::{node_text_outlines_styled_uncached, OutlineGlyph};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use vello::peniko::Color;

// ------------------------------------------------------------------- key

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextLayoutKey {
    pub text: String,
    /// resolved font binding ("Inter 700" / None=default) — carries
    /// family+weight+style+features; fallback decisions depend on it
    pub font: Option<String>,
    /// f64 bits: exact size/width/spacing keys without float-Eq issues
    pub size_bits: u64,
    pub max_width_bits: u64,
    pub letter_spacing_bits: u64,
    pub line_height_bits: u64,
    /// color is NOT shaping-relevant but is baked into OutlineGlyph;
    /// keying it keeps entries correct at negligible cost
    pub color: [u8; 4],
    /// FontManager generation: font loads/unloads flush stale entries
    pub font_epoch: u64,
}

impl TextLayoutKey {
    pub fn new(text: &str, size: f64, max_width: f64, font: Option<&str>, color: Color, font_epoch: u64) -> Self {
        Self::new_styled(text, size, max_width, font, color, font_epoch, 0.0, 1.2)
    }

    /// Typography-aware key (letter spacing px, line height multiplier).
    pub fn new_styled(text: &str, size: f64, max_width: f64, font: Option<&str>, color: Color, font_epoch: u64, ls: f64, lh: f64) -> Self {
        Self {
            text: text.to_string(),
            font: font.map(str::to_string),
            size_bits: size.to_bits(),
            max_width_bits: max_width.to_bits(),
            letter_spacing_bits: ls.to_bits(),
            line_height_bits: lh.to_bits(),
            color: [color.r, color.g, color.b, color.a],
            font_epoch,
        }
    }
}

// ----------------------------------------------------------------- entry

/// The memoized shaped block: glyph runs, positions (block-local
/// transforms), fallback decisions (baked into the outlines), metrics.
pub struct ShapedBlock {
    pub glyphs: Vec<OutlineGlyph>,
    pub height: f64,
}

// ----------------------------------------------------------------- cache

#[derive(Default)]
pub struct ShapedTextCache {
    map: Mutex<HashMap<TextLayoutKey, Arc<ShapedBlock>>>,
    pub hits: std::sync::atomic::AtomicU64,
    pub misses: std::sync::atomic::AtomicU64,
    /// approximate resident bytes + budget-eviction count
    pub bytes: std::sync::atomic::AtomicUsize,
    pub evictions: std::sync::atomic::AtomicU64,
}

/// rough per-block footprint: path elements dominate
fn block_bytes(b: &ShapedBlock) -> usize {
    b.glyphs.iter().map(|g| g.path.elements().len() * 48 + 96).sum::<usize>() + 64
}

const MAX_ENTRIES: usize = 4096;
/// byte budget for cached glyph outlines (~paths dominate);
/// exceeding either bound triggers eviction
const MAX_BYTES: usize = 64 * 1024 * 1024;

impl ShapedTextCache {
    pub fn new() -> Self { Self::default() }

    /// Global process cache (sinks are stateless; the cache is shared).
    pub fn global() -> &'static ShapedTextCache {
        static G: OnceLock<ShapedTextCache> = OnceLock::new();
        G.get_or_init(ShapedTextCache::new)
    }

    pub fn get_or_shape(&self, fonts: &FontManager, key: TextLayoutKey) -> Option<Arc<ShapedBlock>> {
        use std::sync::atomic::Ordering::Relaxed;
        if let Some(hit) = self.map.lock().ok()?.get(&key).cloned() {
            self.hits.fetch_add(1, Relaxed);
            return Some(hit);
        }
        self.misses.fetch_add(1, Relaxed);
        let size = f64::from_bits(key.size_bits);
        let max_width = f64::from_bits(key.max_width_bits);
        let color = Color::rgba8(key.color[0], key.color[1], key.color[2], key.color[3]);
        let ls = f64::from_bits(key.letter_spacing_bits);
        let lh = f64::from_bits(key.line_height_bits);
        let (glyphs, height) = node_text_outlines_styled_uncached(fonts, &key.text, size, max_width, key.font.as_deref(), color, ls, lh)?;
        let block = Arc::new(ShapedBlock { glyphs, height });
        let mut map = self.map.lock().ok()?;
        // budget enforcement: entry count OR approximate byte size
        if map.len() >= MAX_ENTRIES || self.bytes.load(std::sync::atomic::Ordering::Relaxed) >= MAX_BYTES {
            map.clear();
            self.bytes.store(0, std::sync::atomic::Ordering::Relaxed);
            self.evictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        self.bytes.fetch_add(block_bytes(&block), std::sync::atomic::Ordering::Relaxed);
        map.insert(key, block.clone());
        Some(block)
    }

    pub fn len(&self) -> usize { self.map.lock().map(|m| m.len()).unwrap_or(0) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn clear(&self) {
        if let Ok(mut m) = self.map.lock() { m.clear(); }
        self.bytes.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn stats(&self) -> (u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (self.hits.load(Relaxed), self.misses.load(Relaxed))
    }
    /// (approx resident bytes, budget evictions)
    pub fn memory(&self) -> (usize, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (self.bytes.load(Relaxed), self.evictions.load(Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts() -> FontManager {
        let mut fm = FontManager::new();
        assert!(fm.load_system_fonts() > 0);
        fm
    }

    fn key(text: &str, size: f64, width: f64, font: Option<&str>) -> TextLayoutKey {
        TextLayoutKey::new(text, size, width, font, Color::BLACK, 0)
    }

    #[test]
    fn unchanged_text_is_a_hit() {
        let fm = fonts();
        let c = ShapedTextCache::new();
        let a = c.get_or_shape(&fm, key("Hello", 16.0, 400.0, None)).unwrap();
        let b = c.get_or_shape(&fm, key("Hello", 16.0, 400.0, None)).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "same key -> same Arc, no reshape");
        assert_eq!(c.stats(), (1, 1));
    }

    #[test]
    fn text_change_is_a_miss() {
        let fm = fonts();
        let c = ShapedTextCache::new();
        c.get_or_shape(&fm, key("Hello", 16.0, 400.0, None)).unwrap();
        c.get_or_shape(&fm, key("Hello!", 16.0, 400.0, None)).unwrap();
        assert_eq!(c.stats().1, 2, "content change reshapes");
    }

    #[test]
    fn size_and_font_changes_are_misses() {
        // the review's exact example: Hello/16px vs Hello/24px = 2 entries
        let fm = fonts();
        let c = ShapedTextCache::new();
        c.get_or_shape(&fm, key("Hello", 16.0, 400.0, None)).unwrap();
        c.get_or_shape(&fm, key("Hello", 24.0, 400.0, None)).unwrap();
        assert_eq!(c.len(), 2, "16px and 24px are separate entries");
        c.get_or_shape(&fm, key("Hello", 16.0, 400.0, Some("DejaVu Sans"))).unwrap();
        assert_eq!(c.stats().1, 3, "font change reshapes");
    }

    #[test]
    fn width_change_rewraps() {
        let fm = fonts();
        let c = ShapedTextCache::new();
        let wide = c.get_or_shape(&fm, key("alpha beta gamma delta", 20.0, 500.0, None)).unwrap();
        let narrow = c.get_or_shape(&fm, key("alpha beta gamma delta", 20.0, 60.0, None)).unwrap();
        assert_eq!(c.stats().1, 2, "width constraint is in the key");
        assert!(narrow.height > wide.height * 1.5, "narrow entry actually rewrapped");
    }

    #[test]
    fn position_change_is_a_hit_by_construction() {
        // THE critical property: moving a node never reshapes. Output is
        // block-local; position lives in the world transform composed by
        // the sink, so the key contains no position at all.
        let fm = fonts();
        let c = ShapedTextCache::new();
        let a = c.get_or_shape(&fm, key("move me", 16.0, 300.0, None)).unwrap();
        // "node moved": same node text drawn at a different world position
        let b = c.get_or_shape(&fm, key("move me", 16.0, 300.0, None)).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(c.stats(), (1, 1), "position change -> pure hit");
    }

    #[test]
    fn epoch_bump_invalidates() {
        let fm = fonts();
        let c = ShapedTextCache::new();
        c.get_or_shape(&fm, TextLayoutKey::new("x", 16.0, 100.0, None, Color::BLACK, 0)).unwrap();
        c.get_or_shape(&fm, TextLayoutKey::new("x", 16.0, 100.0, None, Color::BLACK, 1)).unwrap();
        assert_eq!(c.stats().1, 2, "font epoch is part of the key");
    }
}
