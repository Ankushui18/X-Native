use std::collections::HashMap;
use vello::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
#[allow(unused_imports)]
use crate::*;

// ------------------------------------------------------------- image assets

/// Phase 4.2: decoded image assets, keyed by the asset name that
/// `NodeKind::Image` references. Load PNGs once, render everywhere.
#[derive(Default)]
pub struct Assets { images: HashMap<String, ImageBrush> }
impl Assets {
    pub fn new() -> Self { Self::default() }
    /// Decode a PNG (any bit depth/color type png-crate supports -> RGBA8).
    pub fn load_png(&mut self, name: &str, path: &str) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        self.load_png_bytes(name, &bytes)
    }

    /// Same decode from in-memory bytes (the AssetStore sync path).
    pub fn load_png_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; reader.output_buffer_size().ok_or("bad png size")?];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width, info.height);
        let rgba: Vec<u8> = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => buf[..info.buffer_size()].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
            png::ColorType::Grayscale => buf[..info.buffer_size()].iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::GrayscaleAlpha => buf[..info.buffer_size()].chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
            other => return Err(format!("unsupported color type {other:?}")),
        };
        self.images.insert(name.into(), ImageBrush {
            image: ImageData {
                data: Blob::from(rgba),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::Alpha,
                width: w,
                height: h,
            },
            sampler: Default::default(),
        });
        Ok(())
    }
    /// Sync from the document's content-addressed AssetStore: decode every
    /// PNG image record under its asset:// id (idempotent — already-decoded
    /// ids are skipped). Returns how many NEW images were decoded.
    pub fn sync_store(&mut self, store: &x_core::AssetStore) -> usize {
        let mut added = 0;
        for rec in store.iter_sorted() {
            if rec.mime != "image/png" || self.images.contains_key(&rec.id) { continue; }
            if self.load_png_bytes(&rec.id.clone(), &rec.bytes).is_ok() { added += 1; }
        }
        added
    }

    /// direct insert of a decoded image (tests / procedural content)
    pub fn insert_raw(&mut self, name: &str, img: ImageBrush) { self.images.insert(name.into(), img); }

    /// Resident decoded bytes (RGBA8: w*h*4 per image) — GPU-side cache
    /// footprint for the memory breakdown.
    pub fn memory_bytes(&self) -> usize {
        self.images.values().map(|i| (i.image.width * i.image.height * 4) as usize).sum()
    }

    /// Evict decoded images NOT in the keep set (e.g. thumbnails scrolled
    /// far away). Returns bytes freed. Content-addressed store keeps the
    /// raw bytes, so eviction is always safe — re-decode on demand.
    pub fn evict_except(&mut self, keep: &std::collections::HashSet<String>) -> usize {
        let before = self.memory_bytes();
        self.images.retain(|k, _| keep.contains(k) || !k.starts_with("asset://"));
        before - self.memory_bytes()
    }
    pub fn get(&self, name: &str) -> Option<&ImageBrush> { self.images.get(name) }
    /// Sorted asset names (image replace UI / pickers).
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.images.keys().cloned().collect();
        v.sort();
        v
    }
    pub fn len(&self) -> usize { self.images.len() }
    pub fn is_empty(&self) -> bool { self.images.is_empty() }
}

