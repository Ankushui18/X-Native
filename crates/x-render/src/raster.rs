//! CPU raster export (PNG / JPG): RenderTree -> tiny-skia Pixmap -> bytes.
//!
//! Headless, deterministic, no GPU. This is the backend for Figma's "Export"
//! surface — it reuses the same `RenderCommand` IR the GPU path and the PDF
//! sink consume, so raster export cannot drift from the canvas.
//!
//! Supports arbitrary scale (@1x/@2x/@3x), PNG (straight alpha, transparent
//! background) and JPG (composited onto a background, quality knob).

use crate::ir::{RenderCommand, RenderTree};
use tiny_skia as ts;
use vello::kurbo::{Affine, BezPath, PathEl, Shape};
use vello::peniko::{Brush, Color, Mix};
use x_core::StrokeOptions;

// ------------------------------------------------------------- conversions

fn to_transform(a: Affine) -> ts::Transform {
    // kurbo Affine::as_coeffs() = [a, b, c, d, e, f] for matrix [[a, c, e], [b, d, f]]
    // tiny-skia Transform::from_row(sx, ky, kx, sy, tx, ty) for matrix [[sx,kx,tx],[ky,sy,ty]]
    let c = a.as_coeffs();
    ts::Transform::from_row(
        c[0] as f32,
        c[1] as f32,
        c[2] as f32,
        c[3] as f32,
        c[4] as f32,
        c[5] as f32,
    )
}

fn to_color(c: Color) -> ts::Color {
    let t = c.to_rgba8();
    ts::Color::from_rgba8(t.r, t.g, t.b, t.a)
}

fn to_path(path: &BezPath) -> Option<ts::Path> {
    let mut pb = ts::PathBuilder::new();
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(p1, p2) => {
                pb.quad_to(p1.x as f32, p1.y as f32, p2.x as f32, p2.y as f32)
            }
            PathEl::CurveTo(p1, p2, p3) => pb.cubic_to(
                p1.x as f32,
                p1.y as f32,
                p2.x as f32,
                p2.y as f32,
                p3.x as f32,
                p3.y as f32,
            ),
            PathEl::ClosePath => pb.close(),
        }
    }
    pb.finish()
}

fn to_shader(g: &vello::peniko::Gradient) -> Option<ts::Shader<'static>> {
    use vello::peniko::GradientKind;
    if g.stops.is_empty() {
        return None;
    }
    let stops: Vec<ts::GradientStop> = g
        .stops
        .iter()
        .map(|s| ts::GradientStop::new(s.offset, to_color(s.color.to_alpha_color())))
        .collect();
    match g.kind {
        GradientKind::Linear(p) => ts::LinearGradient::new(
            ts::Point::from_xy(p.start.x as f32, p.start.y as f32),
            ts::Point::from_xy(p.end.x as f32, p.end.y as f32),
            stops,
            ts::SpreadMode::Pad,
            ts::Transform::identity(),
        ),
        GradientKind::Radial(p) => ts::RadialGradient::new(
            ts::Point::from_xy(p.start_center.x as f32, p.start_center.y as f32),
            p.start_radius,
            ts::Point::from_xy(p.end_center.x as f32, p.end_center.y as f32),
            p.end_radius,
            stops,
            ts::SpreadMode::Pad,
            ts::Transform::identity(),
        ),
        GradientKind::Sweep { .. } => None,
    }
}

fn to_paint(brush: &Brush) -> ts::Paint<'static> {
    let mut paint = ts::Paint::default();
    match brush {
        Brush::Solid(c) => paint.set_color(to_color(*c)),
        Brush::Gradient(g) => match to_shader(g) {
            Some(sh) => paint.shader = sh,
            None => paint.set_color(ts::Color::BLACK),
        },
        // image brush: not a raster fill source; fall back to black
        Brush::Image(_) => paint.set_color(ts::Color::BLACK),
    }
    paint
}

#[allow(clippy::field_reassign_with_default)]
fn to_stroke(options: &StrokeOptions, width: f64) -> ts::Stroke {
    use x_core::{StrokeCap, StrokeJoin};
    let cap = match options.cap_start {
        StrokeCap::Round => ts::LineCap::Round,
        StrokeCap::Square => ts::LineCap::Square,
        _ => ts::LineCap::Butt,
    };
    let join = match options.join {
        StrokeJoin::Round => ts::LineJoin::Round,
        StrokeJoin::Bevel => ts::LineJoin::Bevel,
        StrokeJoin::Miter => ts::LineJoin::Miter,
    };
    let mut stroke = ts::Stroke {
        width: width.max(0.0) as f32,
        line_cap: cap,
        line_join: join,
        miter_limit: options.miter_limit as f32,
        dash: None,
    };
    if !options.dash.is_empty() {
        let dashes: Vec<f32> = options.dash.iter().map(|d| *d as f32).collect();
        stroke.dash = ts::StrokeDash::new(dashes, options.dash_offset as f32);
    }
    stroke
}

fn mix_to_blend(mix: Mix) -> Option<ts::BlendMode> {
    match mix {
        Mix::Normal => None,
        Mix::Multiply => Some(ts::BlendMode::Multiply),
        Mix::Screen => Some(ts::BlendMode::Screen),
        Mix::Overlay => Some(ts::BlendMode::Overlay),
        Mix::Darken => Some(ts::BlendMode::Darken),
        Mix::Lighten => Some(ts::BlendMode::Lighten),
        Mix::ColorDodge => Some(ts::BlendMode::ColorDodge),
        Mix::ColorBurn => Some(ts::BlendMode::ColorBurn),
        Mix::HardLight => Some(ts::BlendMode::HardLight),
        Mix::SoftLight => Some(ts::BlendMode::SoftLight),
        Mix::Difference => Some(ts::BlendMode::Difference),
        Mix::Exclusion => Some(ts::BlendMode::Exclusion),
        Mix::Hue => Some(ts::BlendMode::Hue),
        Mix::Saturation => Some(ts::BlendMode::Saturation),
        Mix::Color => Some(ts::BlendMode::Color),
        Mix::Luminosity => Some(ts::BlendMode::Luminosity),
    }
}

/// An 8-bit alpha mask filled uniformly with `value` (0..255).
fn filled_mask(w: u32, h: u32, value: u8) -> Option<ts::Mask> {
    let mut m = ts::Mask::new(w, h)?;
    m.data_mut().fill(value);
    Some(m)
}

/// Multiply mask `a` (in place) by mask `b` (per-pixel alpha product).
fn mask_multiply(a: &mut ts::Mask, b: &ts::Mask) {
    let b = b.data().to_vec();
    for (x, y) in a.data_mut().iter_mut().zip(b.iter()) {
        *x = (((*x as u32) * (*y as u32)) / 255) as u8;
    }
}

// ------------------------------------------------------------- rasterizer

/// One pushed context (a PushLayer or PushClip). `mask` is the combined
/// clip+alpha restriction in effect inside this context; `blend` is the
/// innermost non-normal blend mode (applied per-draw — see note).
struct Ctx {
    mask: Option<ts::Mask>,
    blend: ts::BlendMode,
}

/// Renders a `RenderTree` into a `tiny_skia::Pixmap` at a scale factor.
pub struct RasterSink<'a> {
    pub assets: Option<&'a crate::Assets>,
    pub fonts: Option<&'a x_text::FontManager>,
    pix: ts::Pixmap,
    stack: Vec<Ctx>,
}

impl<'a> RasterSink<'a> {
    pub fn new(
        assets: Option<&'a crate::Assets>,
        fonts: Option<&'a x_text::FontManager>,
        page_w: f64,
        page_h: f64,
        scale: f64,
        background: Option<Color>,
    ) -> Option<Self> {
        let w = (page_w * scale).round().max(1.0) as u32;
        let h = (page_h * scale).round().max(1.0) as u32;
        let mut pix = ts::Pixmap::new(w, h)?;
        pix.fill(match background {
            Some(c) => to_color(c),
            None => ts::Color::TRANSPARENT,
        });
        Some(Self {
            assets,
            fonts,
            pix,
            stack: Vec::new(),
        })
    }

    pub fn render(mut self, tree: &RenderTree) -> ts::Pixmap {
        for cmd in &tree.commands {
            match cmd {
                RenderCommand::FillPath {
                    transform,
                    path,
                    brush,
                    ..
                } => {
                    if let Some(p) = to_path(path) {
                        let mut paint = to_paint(brush);
                        let mask = self.stack.last().and_then(|c| c.mask.as_ref());
                        paint.blend_mode = self
                            .stack
                            .last()
                            .map(|c| c.blend)
                            .unwrap_or(ts::BlendMode::SourceOver);
                        self.pix.fill_path(
                            &p,
                            &paint,
                            ts::FillRule::Winding,
                            to_transform(*transform),
                            mask,
                        );
                    }
                }
                RenderCommand::StrokePath {
                    transform,
                    path,
                    brush,
                    width,
                    options,
                    ..
                } => {
                    if let Some(p) = to_path(path) {
                        let mut paint = to_paint(brush);
                        let mask = self.stack.last().and_then(|c| c.mask.as_ref());
                        paint.blend_mode = self
                            .stack
                            .last()
                            .map(|c| c.blend)
                            .unwrap_or(ts::BlendMode::SourceOver);
                        let stroke = to_stroke(options, *width);
                        self.pix
                            .stroke_path(&p, &paint, &stroke, to_transform(*transform), mask);
                    }
                }
                RenderCommand::Glyphs {
                    transform,
                    text,
                    size,
                    brush,
                    max_width,
                    font,
                    letter_spacing,
                    line_height,
                    wrap,
                    ..
                } => {
                    let mut drew = false;
                    if let Some(fm) = self.fonts {
                        if let Some((glyphs, _)) = x_text::node_text_outlines_styled(
                            fm,
                            text,
                            *size,
                            *max_width,
                            font.as_deref(),
                            Color::WHITE,
                            *letter_spacing,
                            *line_height,
                            *wrap,
                        ) {
                            for gl in glyphs {
                                if let Some(p) = to_path(&gl.path) {
                                    let full = *transform * gl.transform;
                                    let mut paint = to_paint(brush);
                                    let mask = self.stack.last().and_then(|c| c.mask.as_ref());
                                    paint.blend_mode = self
                                        .stack
                                        .last()
                                        .map(|c| c.blend)
                                        .unwrap_or(ts::BlendMode::SourceOver);
                                    self.pix.fill_path(
                                        &p,
                                        &paint,
                                        ts::FillRule::Winding,
                                        to_transform(full),
                                        mask,
                                    );
                                }
                            }
                            drew = true;
                        }
                    }
                    if !drew {
                        // no font manager: placeholder box so text is not silently lost
                        if let Some(p) = to_path(
                            &vello::kurbo::Rect::new(0.0, 0.0, *max_width, *size).into_path(0.1),
                        ) {
                            let mut paint = ts::Paint::default();
                            paint.set_color(ts::Color::from_rgba8(0xcc, 0xcc, 0xcc, 0x80));
                            let mask = self.stack.last().and_then(|c| c.mask.as_ref());
                            self.pix.fill_path(
                                &p,
                                &paint,
                                ts::FillRule::Winding,
                                to_transform(*transform),
                                mask,
                            );
                        }
                    }
                }
                RenderCommand::Image {
                    transform,
                    asset,
                    w,
                    h,
                    fit,
                    placement,
                    ..
                } => {
                    if let Some(img) = self.assets.and_then(|a| a.get(asset)) {
                        let resolved = x_core::resolve_image_placement(
                            *fit,
                            placement,
                            *w,
                            *h,
                            img.image.width as f64,
                            img.image.height as f64,
                        );
                        // vello Image is straight-alpha RGBA8; tiny-skia wants premultiplied
                        let mut pixmap =
                            ts::Pixmap::new(img.image.width, img.image.height).unwrap();
                        let blob = img.image.data.data();
                        {
                            let dst = pixmap.data_mut();
                            for i in 0..(img.image.width * img.image.height) as usize {
                                let r = blob.get(i * 4).copied().unwrap_or(0) as u32;
                                let g = blob.get(i * 4 + 1).copied().unwrap_or(0) as u32;
                                let b = blob.get(i * 4 + 2).copied().unwrap_or(0) as u32;
                                let a = blob.get(i * 4 + 3).copied().unwrap_or(255) as u32;
                                dst[i * 4] = ((r * a + 127) / 255) as u8;
                                dst[i * 4 + 1] = ((g * a + 127) / 255) as u8;
                                dst[i * 4 + 2] = ((b * a + 127) / 255) as u8;
                                dst[i * 4 + 3] = a as u8;
                            }
                        }
                        // clip to the node box
                        if let Some(box_path) =
                            to_path(&vello::kurbo::Rect::new(0.0, 0.0, *w, *h).into_path(0.1))
                        {
                            let mut clip =
                                filled_mask(self.pix.width(), self.pix.height(), 255).unwrap();
                            clip.fill_path(
                                &box_path,
                                ts::FillRule::Winding,
                                true,
                                to_transform(*transform),
                            );
                            // intersect with any outer clip
                            let mask = match self.stack.last().and_then(|c| c.mask.as_ref()) {
                                Some(outer) => {
                                    let mut m = outer.clone();
                                    mask_multiply(&mut m, &clip);
                                    m
                                }
                                None => clip,
                            };
                            let blend = self
                                .stack
                                .last()
                                .map(|c| c.blend)
                                .unwrap_or(ts::BlendMode::SourceOver);
                            for draw in &resolved.draws {
                                let t = *transform * *draw;
                                let paint = ts::PixmapPaint {
                                    blend_mode: blend,
                                    quality: ts::FilterQuality::Bilinear,
                                    opacity: 1.0,
                                };
                                self.pix.draw_pixmap(
                                    0,
                                    0,
                                    pixmap.as_ref(),
                                    &paint,
                                    to_transform(t),
                                    Some(&mask),
                                );
                            }
                        }
                    } else {
                        // missing asset: gray box (matches the Vello sink)
                        if let Some(p) =
                            to_path(&vello::kurbo::Rect::new(0.0, 0.0, *w, *h).into_path(0.1))
                        {
                            let mut paint = ts::Paint::default();
                            paint.set_color(ts::Color::from_rgba8(0xdd, 0xdd, 0xdd, 0xff));
                            let mask = self.stack.last().and_then(|c| c.mask.as_ref());
                            paint.blend_mode = self
                                .stack
                                .last()
                                .map(|c| c.blend)
                                .unwrap_or(ts::BlendMode::SourceOver);
                            self.pix.fill_path(
                                &p,
                                &paint,
                                ts::FillRule::Winding,
                                to_transform(*transform),
                                mask,
                            );
                        }
                    }
                }
                RenderCommand::PushLayer { mix, alpha, .. } => {
                    let blend = mix_to_blend(*mix).unwrap_or(ts::BlendMode::SourceOver);
                    let parent_blend = self
                        .stack
                        .last()
                        .map(|c| c.blend)
                        .unwrap_or(ts::BlendMode::SourceOver);
                    // normal-blend group with alpha < 1 -> scale the mask; else a blend-mode group
                    if blend == ts::BlendMode::SourceOver && *alpha < 1.0 {
                        let scaled = (*alpha * 255.0).clamp(0.0, 255.0) as u8;
                        let new_mask = match self.stack.last().and_then(|c| c.mask.as_ref()) {
                            Some(outer) => {
                                let mut m = outer.clone();
                                for v in m.data_mut() {
                                    *v = (((*v as u32) * (scaled as u32)) / 255) as u8;
                                }
                                Some(m)
                            }
                            None => filled_mask(self.pix.width(), self.pix.height(), scaled),
                        };
                        self.stack.push(Ctx {
                            mask: new_mask,
                            blend: parent_blend,
                        });
                    } else {
                        let m = self.stack.last().and_then(|c| c.mask.as_ref()).cloned();
                        self.stack.push(Ctx { mask: m, blend });
                    }
                }
                RenderCommand::PushClip {
                    transform, path, ..
                } => {
                    let parent_blend = self
                        .stack
                        .last()
                        .map(|c| c.blend)
                        .unwrap_or(ts::BlendMode::SourceOver);
                    if let Some(p) = to_path(path) {
                        let new_mask = match self.stack.last().and_then(|c| c.mask.as_ref()) {
                            Some(outer) => {
                                let mut m = outer.clone();
                                m.intersect_path(
                                    &p,
                                    ts::FillRule::Winding,
                                    true,
                                    to_transform(*transform),
                                );
                                m
                            }
                            None => {
                                let mut m =
                                    filled_mask(self.pix.width(), self.pix.height(), 255).unwrap();
                                m.intersect_path(
                                    &p,
                                    ts::FillRule::Winding,
                                    true,
                                    to_transform(*transform),
                                );
                                m
                            }
                        };
                        self.stack.push(Ctx {
                            mask: Some(new_mask),
                            blend: parent_blend,
                        });
                    } else {
                        let m = self.stack.last().and_then(|c| c.mask.as_ref()).cloned();
                        self.stack.push(Ctx {
                            mask: m,
                            blend: parent_blend,
                        });
                    }
                }
                RenderCommand::PopLayer => {
                    self.stack.pop();
                }
            }
        }
        self.pix
    }
}

// ------------------------------------------------------------- encoding

/// Raster format for export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RasterFormat {
    Png,
    Jpg(u8), // quality 0..=100
}

/// Encode a rendered pixmap to PNG bytes.
pub fn encode_png(pix: &ts::Pixmap) -> Result<Vec<u8>, String> {
    pix.encode_png().map_err(|e| e.to_string())
}

/// Encode a rendered pixmap to JPG bytes, composited onto white (JPG has no
/// alpha), at the given quality.
pub fn encode_jpg(pix: &ts::Pixmap, quality: u8) -> Result<Vec<u8>, String> {
    let w = pix.width();
    let h = pix.height();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in pix.pixels() {
        let c = px.demultiply();
        // unpremultiplied color over white
        let a = c.alpha() as u32;
        let r = ((c.red() as u32 * a) + (255 * (255 - a))) / 255;
        let g = ((c.green() as u32 * a) + (255 * (255 - a))) / 255;
        let b = ((c.blue() as u32 * a) + (255 * (255 - a))) / 255;
        rgb.push(r as u8);
        rgb.push(g as u8);
        rgb.push(b as u8);
    }
    let mut out = Vec::new();
    let enc = jpeg_encoder::Encoder::new(&mut out, quality.min(100));
    enc.encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Full raster export: RenderTree -> encoded bytes.
#[allow(clippy::too_many_arguments)]
pub fn export_raster(
    tree: &RenderTree,
    page_w: f64,
    page_h: f64,
    format: RasterFormat,
    scale: f64,
    background: Option<Color>,
    assets: Option<&crate::Assets>,
    fonts: Option<&x_text::FontManager>,
) -> Result<(Vec<u8>, u32, u32), String> {
    let sink = RasterSink::new(assets, fonts, page_w, page_h, scale, background)
        .ok_or("raster pixmap allocation failed")?;
    let pix = sink.render(tree);
    let (w, h) = (pix.width(), pix.height());
    let bytes = match format {
        RasterFormat::Png => encode_png(&pix)?,
        RasterFormat::Jpg(q) => encode_jpg(&pix, q)?,
    };
    Ok((bytes, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{build_render_tree, build_render_tree_slice};
    use x_core::{Node, Variables};

    fn doc() -> Node {
        Node::frame("page", 200.0, 100.0)
            .child(Node::rect(
                "r",
                10.0,
                10.0,
                100.0,
                50.0,
                Color::from_rgb8(255, 0, 0),
            ))
            .child(Node::ellipse(
                "e",
                120.0,
                10.0,
                60.0,
                60.0,
                Color::from_rgb8(0, 0, 255),
            ))
    }

    fn sample_px(pix: &ts::Pixmap, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let px = pix
            .pixel(x, y)
            .unwrap_or(ts::PremultipliedColorU8::TRANSPARENT);
        let c = px.demultiply();
        (c.red(), c.green(), c.blue(), c.alpha())
    }

    #[test]
    fn raster_fills_shapes_and_scales() {
        let d = doc();
        let tree = build_render_tree(&d, &Variables::default());
        let (bytes, w, h) = export_raster(
            &tree,
            200.0,
            100.0,
            RasterFormat::Png,
            1.0,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!((w, h), (200, 100));
        assert!(bytes.len() > 8);
        assert_eq!(&bytes[1..4], b"PNG", "valid PNG signature");
        // @2x doubles dimensions
        let (_, w2, h2) = export_raster(
            &tree,
            200.0,
            100.0,
            RasterFormat::Png,
            2.0,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!((w2, h2), (400, 200));
    }

    #[test]
    fn raster_hits_expected_colors() {
        let d = doc();
        let tree = build_render_tree(&d, &Variables::default());
        let sink = RasterSink::new(None, None, 200.0, 100.0, 1.0, None).unwrap();
        let pix = sink.render(&tree);
        // center of the red rect (10..110, 10..60)
        let (r, g, b, _) = sample_px(&pix, 60, 35);
        assert!(r > 200 && g < 60 && b < 60, "red rect center = {r},{g},{b}");
        // center of the blue ellipse (120..180, 10..70)
        let (r, g, b, _) = sample_px(&pix, 150, 40);
        assert!(
            b > 200 && r < 60 && g < 60,
            "blue ellipse center = {r},{g},{b}"
        );
        // transparent background corner (0..10)
        let (_, _, _, a) = sample_px(&pix, 2, 2);
        assert_eq!(a, 0, "transparent background");
    }

    #[test]
    fn jpg_encodes_with_white_background() {
        let d = doc();
        let tree = build_render_tree(&d, &Variables::default());
        let (bytes, w, h) = export_raster(
            &tree,
            200.0,
            100.0,
            RasterFormat::Jpg(90),
            1.0,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!((w, h), (200, 100));
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "JPEG SOI marker");
    }

    #[test]
    fn slice_exports_region_at_origin() {
        // a red rect at (0,0) and a blue rect at (100,100); slice only the
        // blue rect's region. The export must be 40x40, blue at origin, and
        // NOT contain the red rect (which sits outside the slice).
        let d = Node::frame("page", 400.0, 300.0)
            .child(Node::rect(
                "a",
                0.0,
                0.0,
                50.0,
                50.0,
                Color::from_rgb8(255, 0, 0),
            ))
            .child(Node::rect(
                "b",
                100.0,
                100.0,
                40.0,
                40.0,
                Color::from_rgb8(0, 0, 255),
            ))
            .child(Node::slice("s", 100.0, 100.0, 40.0, 40.0));
        let (tree, w, h) = build_render_tree_slice(&d, "s", &Variables::default()).unwrap();
        let (bytes, pw, ph) =
            export_raster(&tree, w, h, RasterFormat::Png, 1.0, None, None, None).unwrap();
        assert_eq!((pw, ph), (40, 40));
        assert_eq!(&bytes[1..4], b"PNG");
        let sink = RasterSink::new(None, None, w, h, 1.0, None).unwrap();
        let pix = sink.render(&tree);
        // center of the slice -> blue (from rect b)
        let (r, g, b, _) = sample_px(&pix, 20, 20);
        assert!(
            b > 200 && r < 60 && g < 60,
            "slice center should be blue = {r},{g},{b}"
        );
        // no red anywhere: the red rect is entirely outside the slice region
        let mut any_red = false;
        for y in 0..40u32 {
            for x in 0..40u32 {
                let (r, g, b, a) = sample_px(&pix, x, y);
                if a > 0 && r > 200 && g < 60 && b < 60 {
                    any_red = true;
                }
            }
        }
        assert!(!any_red, "red rect must not bleed into the slice");
    }

    #[test]
    fn clip_masks_restrict_draws() {
        // frame with a circular mask over a red rect -> corners clipped out
        let d = Node::frame("page", 100.0, 100.0)
            .child(Node::ellipse("m", 0.0, 0.0, 80.0, 80.0, Color::WHITE).mask(true))
            .child(Node::rect(
                "r",
                0.0,
                0.0,
                100.0,
                100.0,
                Color::from_rgb8(255, 0, 0),
            ));
        let tree = build_render_tree(&d, &Variables::default());
        let sink = RasterSink::new(None, None, 100.0, 100.0, 1.0, Some(Color::WHITE)).unwrap();
        let pix = sink.render(&tree);
        // inside the mask circle (center) -> red
        let (r, _, _, _) = sample_px(&pix, 40, 40);
        assert!(r > 200, "center should be red = {r}");
        // corner outside the circle (95,95) -> white background
        let (r, g, b, _) = sample_px(&pix, 95, 95);
        assert!(
            r > 240 && g > 240 && b > 240,
            "corner should be white = {r},{g},{b}"
        );
    }
}
