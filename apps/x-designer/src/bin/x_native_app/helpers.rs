#[allow(unused_imports)]
use super::*;

pub fn count_nodes(n: &Node) -> usize { 1 + n.children.iter().map(count_nodes).sum::<usize>() }

pub fn fill_rrect(s: &mut Scene, r: Rect, radius: f64, c: Color) {
    s.fill(Fill::NonZero, Affine::IDENTITY, c, None, &vello::kurbo::RoundedRect::from_rect(r, radius).into_path(0.1));
}

/// Vector tool icons drawn at (cx, cy), ~16px box. Real glyphs, not letters.
pub fn draw_tool_icon(s: &mut Scene, tool: Tool, cx: f64, cy: f64, c: Color) {
    use vello::kurbo::{BezPath, Circle as KCircle, Line as KLine};
    let st = vello::kurbo::Stroke::new(1.6).with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round);
    let t = Affine::translate((cx - 8.0, cy - 8.0)); // local 0..16 box
    match tool {
        Tool::Select => {
            let mut p = BezPath::new();
            p.move_to((4.0, 1.0)); p.line_to((4.0, 13.0)); p.line_to((7.2, 10.2));
            p.line_to((9.4, 15.0)); p.line_to((11.4, 14.1)); p.line_to((9.2, 9.4));
            p.line_to((13.0, 9.0)); p.close_path();
            s.fill(Fill::NonZero, t, c, None, &p);
        }
        Tool::Hand => {
            for (x, y0) in [(5.0, 3.0), (8.0, 2.0), (11.0, 3.0)] {
                s.stroke(&st, t, c, None, &KLine::new((x, y0), (x, 8.0)));
            }
            let mut palm = BezPath::new();
            palm.move_to((3.5, 8.0)); palm.line_to((12.5, 8.0)); palm.line_to((12.0, 13.0));
            palm.line_to((5.5, 13.5)); palm.line_to((3.0, 10.0)); palm.close_path();
            s.fill(Fill::NonZero, t, c, None, &palm);
        }
        Tool::Scale => {
            s.stroke(&st, t, c, None, &Rect::new(2.0, 6.0, 10.0, 14.0).into_path(0.1));
            s.stroke(&st, t, c, None, &KLine::new((7.0, 9.0), (14.0, 2.0)));
            s.stroke(&st, t, c, None, &KLine::new((10.0, 2.0), (14.0, 2.0)));
            s.stroke(&st, t, c, None, &KLine::new((14.0, 2.0), (14.0, 6.0)));
        }
        Tool::Frame => {
            s.stroke(&st, t, c, None, &KLine::new((5.0, 1.0), (5.0, 15.0)));
            s.stroke(&st, t, c, None, &KLine::new((11.0, 1.0), (11.0, 15.0)));
            s.stroke(&st, t, c, None, &KLine::new((1.0, 5.0), (15.0, 5.0)));
            s.stroke(&st, t, c, None, &KLine::new((1.0, 11.0), (15.0, 11.0)));
        }
        Tool::Rectangle => { s.stroke(&st, t, c, None, &Rect::new(2.0, 3.0, 14.0, 13.0).into_path(0.1)); }
        Tool::Ellipse => { s.stroke(&st, t, c, None, &KCircle::new((8.0, 8.0), 6.0)); }
        Tool::Line => { s.stroke(&st, t, c, None, &KLine::new((2.0, 14.0), (14.0, 2.0))); }
        Tool::Polygon => {
            let mut p = BezPath::new();
            for (i, cmd) in regular_polygon(6, 14.0, 14.0).iter().enumerate() {
                match cmd {
                    arco_native::PathCmd::MoveTo(x, y) => p.move_to((*x + 1.0, *y + 1.0)),
                    arco_native::PathCmd::LineTo(x, y) => p.line_to((*x + 1.0, *y + 1.0)),
                    arco_native::PathCmd::Close => p.close_path(),
                    _ => { let _ = i; }
                }
            }
            s.stroke(&st, t, c, None, &p);
        }
        Tool::Star => {
            let mut p = BezPath::new();
            for cmd in star_path(5, 15.0, 15.0) {
                match cmd {
                    arco_native::PathCmd::MoveTo(x, y) => p.move_to((x + 0.5, y + 0.5)),
                    arco_native::PathCmd::LineTo(x, y) => p.line_to((x + 0.5, y + 0.5)),
                    arco_native::PathCmd::Close => p.close_path(),
                    _ => {}
                }
            }
            s.fill(Fill::NonZero, t, c, None, &p);
        }
        Tool::Pen => {
            let mut p = BezPath::new();
            p.move_to((8.0, 1.0)); p.line_to((11.5, 4.5)); p.line_to((6.5, 12.0));
            p.line_to((3.0, 13.0)); p.line_to((4.0, 9.5)); p.close_path();
            s.stroke(&st, t, c, None, &p);
            s.stroke(&st, t, c, None, &KLine::new((4.0, 12.0), (2.0, 14.0)));
        }
        Tool::Text => {
            s.stroke(&st, t, c, None, &KLine::new((3.0, 3.0), (13.0, 3.0)));
            s.stroke(&st, t, c, None, &KLine::new((8.0, 3.0), (8.0, 14.0)));
        }
    }
}

pub fn fill_rect(s: &mut Scene, r: Rect, c: Color) {
    s.fill(Fill::NonZero, Affine::IDENTITY, c, None, &r.into_path(0.1));
}
pub fn stroke_rect(s: &mut Scene, r: Rect, c: Color, w: f64) {
    s.stroke(&vello::kurbo::Stroke::new(w), Affine::IDENTITY, c, None, &r.into_path(0.1));
}
/// Global UI font (shaped real typography for ALL chrome text — the
/// blocky vector font is retired). ShapedTextCache makes this cheap:
/// every distinct label shapes once per (text,size,color).
thread_local! {
    static UI_FONTS: arco_native::text::FontManager = {
        let mut fm = arco_native::text::FontManager::new();
        fm.load_system_fonts();
        fm
    };
}

pub fn label(s: &mut Scene, text: &str, x: f64, y: f64, size: f64, c: Color) {
    UI_FONTS.with(|fm| {
        if fm.fonts.is_empty() {
            encode_text(s, text, Affine::translate((x, y)), size, c);
            return;
        }
        // node_text_outlines sizes text as node-height (0.72em factor):
        // scale so `size` behaves like a font pixel size for UI labels
        let key_size = size * 1.42;
        if let Some((glyphs, _)) = arco_native::text::node_text_outlines(
            fm, text, key_size, 10_000.0, None, c) {
            let world = Affine::translate((x, y - size * 0.18));
            for g in &glyphs {
                s.fill(vello::peniko::Fill::NonZero, world * g.transform, g.color, None, &g.path);
            }
        } else {
            encode_text(s, text, Affine::translate((x, y)), size, c);
        }
    })
}

/// Measure UI text with the real font (falls back to grid metrics).
pub fn ui_measure(text: &str, size: f64) -> f64 {
    UI_FONTS.with(|fm| {
        if fm.fonts.is_empty() { return measure(text, size); }
        if let Some(f) = fm.default_font() {
            return fm.measure(text, f, size * 1.42 * 0.72);
        }
        measure(text, size)
    })
}

pub fn world_transform_of(root: &Node, id: &str) -> Option<(Affine, f64, f64)> {
    fn walk(node: &Node, parent: Affine, id: &str) -> Option<(Affine, f64, f64)> {
        let world = parent * node.transform.matrix(node.w, node.h);
        if node.id == id { return Some((world, node.w, node.h)); }
        node.children.iter().find_map(|c| walk(c, world, id))
    }
    walk(root, Affine::IDENTITY, id)
}

/// Regular n-gon inscribed in (w,h), point-up.
pub fn regular_polygon(sides: usize, w: f64, h: f64) -> Vec<arco_native::PathCmd> {
    use arco_native::PathCmd::*;
    let (rx, ry, cx, cy) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
    let mut out = vec![];
    for i in 0..sides {
        let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / sides as f64;
        let (x, y) = (cx + rx * a.cos(), cy + ry * a.sin());
        out.push(if i == 0 { MoveTo(x, y) } else { LineTo(x, y) });
    }
    out.push(Close);
    out
}

/// n-point star inscribed in (w,h), point-up.
pub fn star_path_with_ratio(points: usize, w: f64, h: f64, inner_ratio: f64) -> Vec<arco_native::PathCmd> {
    use arco_native::PathCmd::*;
    let (rx, ry, cx, cy) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
    let mut out = vec![];
    for i in 0..(points * 2) {
        let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::PI / points as f64;
        let inner = inner_ratio.clamp(0.05, 0.95);
        let (fx, fy) = if i % 2 == 0 { (1.0, 1.0) } else { (inner, inner) };
        let (x, y) = (cx + rx * fx * a.cos(), cy + ry * fy * a.sin());
        out.push(if i == 0 { MoveTo(x, y) } else { LineTo(x, y) });
    }
    out.push(Close);
    out
}

pub fn star_path(points: usize, w: f64, h: f64) -> Vec<arco_native::PathCmd> {
    star_path_with_ratio(points, w, h, 0.4)
}

pub fn quad_bounds(world: Affine, w: f64, h: f64) -> Rect {
    let pts = [world * Point::new(0.0, 0.0), world * Point::new(w, 0.0), world * Point::new(w, h), world * Point::new(0.0, h)];
    let xs = pts.iter().map(|p| p.x);
    let ys = pts.iter().map(|p| p.y);
    Rect::new(
        xs.clone().fold(f64::INFINITY, f64::min),
        ys.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.fold(f64::NEG_INFINITY, f64::max),
    )
}


/// Bridge: x-ui PaintOps -> Vello scene (one place, all retained widgets).
pub fn paint_ui_ops(scene: &mut Scene, ops: &[arco_native::ui::PaintOp]) {
    for op in ops {
        match op {
            arco_native::ui::PaintOp::Rect { r, color, alpha, radius } => {
                let rect = Rect::new(r.x, r.y, r.x + r.w, r.y + r.h);
                let c = Color::rgba8(color[0], color[1], color[2], *alpha);
                if *radius > 0.0 { fill_rrect(scene, rect, *radius, c); } else { fill_rect(scene, rect, c); }
            }
            arco_native::ui::PaintOp::Border { r, color, width } => {
                let rect = Rect::new(r.x, r.y, r.x + r.w, r.y + r.h);
                stroke_rect(scene, rect, Color::rgb8(color[0], color[1], color[2]), *width);
            }
            arco_native::ui::PaintOp::Text { x, y, size, color, text } => {
                label(scene, text, *x, *y, *size, Color::rgb8(color[0], color[1], color[2]));
            }
        }
    }
}

/// Ctrl+Alt+E: export the current page as a real rendered PNG. Renders the
/// document (via the IR sink, with assets + fonts, no editor chrome) into an
/// offscreen wgpu texture at 1x page size, reads it back, writes `path`.
/// Returns Err(msg) if no adapter/device is available.
pub fn export_png(
    root: &Node,
    vars: &Variables,
    assets: &arco_native::Assets,
    fonts: &arco_native::text::FontManager,
    path: &str,
) -> Result<(u32, u32), String> {
    let width = root.w.max(1.0).min(4096.0) as u32;
    let height = root.h.max(1.0).min(4096.0) as u32;
    let (scene, _tree) = arco_native::render_via_ir(root, vars, Some(assets), Some(fonts));

    pollster::block_on(async move {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or("no wgpu adapter for PNG export")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            }, None)
            .await
            .map_err(|e| e.to_string())?;
        let mut renderer = Renderer::new(&device, RendererOptions {
            surface_format: None,
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: std::num::NonZeroUsize::new(1),
        }).map_err(|e| e.to_string())?;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("png export target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        renderer.render_to_texture(&device, &queue, &scene, &view, &RenderParams {
            base_color: Color::WHITE,
            width, height,
            antialiasing_method: AaConfig::Area,
        }).map_err(|e| format!("{e:?}"))?;

        let bpp = 4u32;
        let unpadded = width * bpp;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = (unpadded + align - 1) / align * align;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("png export readback"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::ImageCopyBuffer { buffer: &buf, layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(height) } },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        queue.submit(Some(enc.finish()));
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().map_err(|e| e.to_string())?.map_err(|e| format!("{e:?}"))?;
        let data = slice.get_mapped_range();
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for y in 0..height as usize {
            let s = y * padded as usize;
            let d = y * width as usize * 4;
            pixels[d..d + width as usize * 4].copy_from_slice(&data[s..s + width as usize * 4]);
        }
        drop(data);
        buf.unmap();

        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let w = std::io::BufWriter::new(file);
        let mut pe = png::Encoder::new(w, width, height);
        pe.set_color(png::ColorType::Rgba);
        pe.set_depth(png::BitDepth::Eight);
        let mut writer = pe.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&pixels).map_err(|e| e.to_string())?;
        Ok((width, height))
    })
}

// ---- session 50: inspector polish glyphs (painter-only helpers) ----

/// Real eye glyph (almond outline + iris) for visibility toggles.
pub fn draw_eye(s: &mut Scene, cx: f64, cy: f64, on: bool, c: Color) {
    let st = vello::kurbo::Stroke::new(1.1).with_caps(vello::kurbo::Cap::Round);
    let mut p = vello::kurbo::BezPath::new();
    p.move_to((cx - 5.5, cy));
    p.curve_to((cx - 2.5, cy - 4.0), (cx + 2.5, cy - 4.0), (cx + 5.5, cy));
    p.curve_to((cx + 2.5, cy + 4.0), (cx - 2.5, cy + 4.0), (cx - 5.5, cy));
    p.close_path();
    s.stroke(&st, Affine::IDENTITY, c, None, &p);
    if on {
        s.fill(Fill::NonZero, Affine::IDENTITY, c, None, &vello::kurbo::Circle::new((cx, cy), 1.7));
    } else {
        // slash through
        s.stroke(&st, Affine::IDENTITY, c, None,
            &vello::kurbo::Line::new((cx - 5.0, cy + 4.5), (cx + 5.0, cy - 4.5)));
    }
}

/// Small bordered stepper button with a centered - or + glyph.
pub fn draw_stepper(s: &mut Scene, r: Rect, plus: bool, hover: bool, c: Color) {
    if hover { fill_rrect(s, r, 4.0, C_HOVERBG); }
    let st = vello::kurbo::Stroke::new(1.2).with_caps(vello::kurbo::Cap::Round);
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    s.stroke(&st, Affine::IDENTITY, c, None, &vello::kurbo::Line::new((cx - 3.0, cy), (cx + 3.0, cy)));
    if plus { s.stroke(&st, Affine::IDENTITY, c, None, &vello::kurbo::Line::new((cx, cy - 3.0), (cx, cy + 3.0))); }
}

/// Alignment icon (bar + object box) for the 6-button align row.
/// i: 0 left, 1 center-h, 2 right, 3 top, 4 middle, 5 bottom.
pub fn draw_align_icon(s: &mut Scene, i: usize, r: Rect, c: Color) {
    let st = vello::kurbo::Stroke::new(1.2).with_caps(vello::kurbo::Cap::Round);
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    let bar = |s: &mut Scene, a: (f64, f64), b: (f64, f64)| {
        s.stroke(&st, Affine::IDENTITY, c, None, &vello::kurbo::Line::new(a, b));
    };
    let boxf = |s: &mut Scene, x0: f64, y0: f64, x1: f64, y1: f64| {
        s.fill(Fill::NonZero, Affine::IDENTITY, c, None,
            &vello::kurbo::RoundedRect::new(x0, y0, x1, y1, 1.0));
    };
    match i {
        0 => { bar(s, (cx - 6.0, cy - 5.5), (cx - 6.0, cy + 5.5)); boxf(s, cx - 3.5, cy - 3.0, cx + 6.0, cy + 3.0); }
        1 => { bar(s, (cx, cy - 5.5), (cx, cy + 5.5)); boxf(s, cx - 4.5, cy - 3.0, cx + 4.5, cy + 3.0); }
        2 => { bar(s, (cx + 6.0, cy - 5.5), (cx + 6.0, cy + 5.5)); boxf(s, cx - 6.0, cy - 3.0, cx + 3.5, cy + 3.0); }
        3 => { bar(s, (cx - 5.5, cy - 6.0), (cx + 5.5, cy - 6.0)); boxf(s, cx - 3.0, cy - 3.5, cx + 3.0, cy + 6.0); }
        4 => { bar(s, (cx - 5.5, cy), (cx + 5.5, cy)); boxf(s, cx - 3.0, cy - 4.5, cx + 3.0, cy + 4.5); }
        _ => { bar(s, (cx - 5.5, cy + 6.0), (cx + 5.5, cy + 6.0)); boxf(s, cx - 3.0, cy - 6.0, cx + 3.0, cy + 3.5); }
    }
}

/// Thin section separator line across the inspector.
pub fn draw_section_sep(s: &mut Scene, ix: f64, win_w: f64, y: f64) {
    fill_rect(s, Rect::new(ix + 8.0, y, win_w - 8.0, y + 1.0), C_PANEL_EDGE);
}
