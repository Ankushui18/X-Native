#[allow(unused_imports)]
use super::*;

pub fn count_nodes(n: &Node) -> usize {
    1 + n.children.iter().map(count_nodes).sum::<usize>()
}

/// Short display form of a node id (truncated, with an ellipsis) for chips.
pub fn short_id(id: &str) -> String {
    if id.chars().count() > 8 {
        format!("{}…", id.chars().take(7).collect::<String>())
    } else {
        id.to_string()
    }
}

pub fn fill_rrect(s: &mut Scene, r: Rect, radius: f64, c: Color) {
    s.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        c,
        None,
        &vello::kurbo::RoundedRect::from_rect(r, radius).into_path(0.1),
    );
}

/// Tool glyph — delegated to the shared icon library (Design System v2:
/// one stroke-only 24-grid set at a consistent 1.8 weight for all chrome).
pub fn draw_tool_icon(s: &mut Scene, tool: Tool, cx: f64, cy: f64, c: Color) {
    icons::paint(s, icons::tool_icon(tool), cx, cy, 16.0, c);
}

pub fn fill_rect(s: &mut Scene, r: Rect, c: Color) {
    s.fill(Fill::NonZero, Affine::IDENTITY, c, None, &r.into_path(0.1));
}
pub fn stroke_rect(s: &mut Scene, r: Rect, c: Color, w: f64) {
    s.stroke(
        &vello::kurbo::Stroke::new(w),
        Affine::IDENTITY,
        c,
        None,
        &r.into_path(0.1),
    );
}
// Global UI font (shaped real typography for ALL chrome text — the
// blocky vector font is retired). ShapedTextCache makes this cheap:
// every distinct label shapes once per (text,size,color).
thread_local! {
    static UI_FONTS: x_native::text::FontManager = {
        let mut fm = x_native::text::FontManager::new();
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
        if let Some((glyphs, _)) =
            x_native::text::node_text_outlines(fm, text, key_size, 10_000.0, None, c)
        {
            let world = Affine::translate((x, y - size * 0.18));
            for g in &glyphs {
                s.fill(
                    vello::peniko::Fill::NonZero,
                    world * g.transform,
                    g.color,
                    None,
                    &g.path,
                );
            }
        } else {
            encode_text(s, text, Affine::translate((x, y)), size, c);
        }
    })
}

/// Measure UI text with the real font (falls back to grid metrics).
pub fn ui_measure(text: &str, size: f64) -> f64 {
    UI_FONTS.with(|fm| {
        if fm.fonts.is_empty() {
            return measure(text, size);
        }
        if let Some(f) = fm.default_font() {
            return fm.measure(text, f, size * 1.42 * 0.72);
        }
        measure(text, size)
    })
}

pub fn world_transform_of(root: &Node, id: &str) -> Option<(Affine, f64, f64)> {
    fn walk(node: &Node, parent: Affine, id: &str) -> Option<(Affine, f64, f64)> {
        let world = parent * node.transform.matrix(node.w, node.h);
        if node.id == id {
            return Some((world, node.w, node.h));
        }
        node.children.iter().find_map(|c| walk(c, world, id))
    }
    walk(root, Affine::IDENTITY, id)
}

/// Regular n-gon inscribed in (w,h), point-up.
pub fn regular_polygon(sides: usize, w: f64, h: f64) -> Vec<x_native::PathCmd> {
    use x_native::PathCmd::*;
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
pub fn star_path_with_ratio(
    points: usize,
    w: f64,
    h: f64,
    inner_ratio: f64,
) -> Vec<x_native::PathCmd> {
    use x_native::PathCmd::*;
    let (rx, ry, cx, cy) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
    let mut out = vec![];
    for i in 0..(points * 2) {
        let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::PI / points as f64;
        let inner = inner_ratio.clamp(0.05, 0.95);
        let (fx, fy) = if i % 2 == 0 {
            (1.0, 1.0)
        } else {
            (inner, inner)
        };
        let (x, y) = (cx + rx * fx * a.cos(), cy + ry * fy * a.sin());
        out.push(if i == 0 { MoveTo(x, y) } else { LineTo(x, y) });
    }
    out.push(Close);
    out
}

pub fn star_path(points: usize, w: f64, h: f64) -> Vec<x_native::PathCmd> {
    star_path_with_ratio(points, w, h, 0.4)
}

pub fn quad_bounds(world: Affine, w: f64, h: f64) -> Rect {
    let pts = [
        world * Point::new(0.0, 0.0),
        world * Point::new(w, 0.0),
        world * Point::new(w, h),
        world * Point::new(0.0, h),
    ];
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
pub fn paint_ui_ops(scene: &mut Scene, ops: &[x_native::ui::PaintOp]) {
    for op in ops {
        match op {
            x_native::ui::PaintOp::Rect {
                r,
                color,
                alpha,
                radius,
            } => {
                let rect = Rect::new(r.x, r.y, r.x + r.w, r.y + r.h);
                let c = Color::from_rgba8(color[0], color[1], color[2], *alpha);
                if *radius > 0.0 {
                    fill_rrect(scene, rect, *radius, c);
                } else {
                    fill_rect(scene, rect, c);
                }
            }
            x_native::ui::PaintOp::Border { r, color, width } => {
                let rect = Rect::new(r.x, r.y, r.x + r.w, r.y + r.h);
                stroke_rect(
                    scene,
                    rect,
                    Color::from_rgb8(color[0], color[1], color[2]),
                    *width,
                );
            }
            x_native::ui::PaintOp::Text {
                x,
                y,
                size,
                color,
                text,
            } => {
                label(
                    scene,
                    text,
                    *x,
                    *y,
                    *size,
                    Color::from_rgb8(color[0], color[1], color[2]),
                );
            }
        }
    }
}

/// Export a single node's subtree as SVG. The node is cloned at its own
/// origin; component instances resolve against the node's own subtree (raster
/// export via `export_raster_file` resolves against the full document).
pub fn export_svg_node(
    node: &Node,
    vars: &Variables,
    fonts: &x_native::text::FontManager,
) -> String {
    let mut n = node.clone();
    n.transform.x = 0.0;
    n.transform.y = 0.0;
    let outliner = x_native::svg_text_outliner(fonts);
    let resolver =
        |name: &str| -> Option<Vec<u8>> { std::fs::read(format!("assets/{name}.png")).ok() };
    x_native::fileio::export_svg_full(&n, vars, Some(&resolver), Some(&outliner))
}

/// Write one file per entry in a node's `export_settings` into `out_dir`.
/// Filenames are `{node_id}{suffix}.{ext}`. PNG/JPG rasterize via
/// `export_raster_file`; SVG uses `export_svg_node`. Returns the number of
#[allow(clippy::too_many_arguments)]
/// files written.
pub fn export_node_settings(
    doc: &Node,
    node: &Node,
    vars: &Variables,
    assets: &x_native::Assets,
    fonts: &x_native::text::FontManager,
    out_dir: &str,
) -> Result<usize, String> {
    let settings = node.export_settings.clone();
    if settings.is_empty() {
        return Err("no export settings on this node".into());
    }
    let base = node.id.clone();
    let mut count = 0usize;
    for s in &settings {
        let ext = match s.format.as_str() {
            "jpg" | "jpeg" => "jpg",
            "svg" => "svg",
            _ => "png",
        };
        let file = format!("{}{}.{}", base, s.suffix, ext);
        let path = std::path::Path::new(out_dir).join(&file);
        if ext == "svg" {
            let svg = export_svg_node(node, vars, fonts);
            std::fs::write(&path, svg).map_err(|e| e.to_string())?;
        } else {
            let fmt = if ext == "jpg" {
                x_native::RasterFormat::Jpg(s.quality)
            } else {
                x_native::RasterFormat::Png
            };
            export_raster_file(
                doc,
                node,
                vars,
                assets,
                fonts,
                path.to_string_lossy().as_ref(),
                fmt,
                s.scale,
            )?;
        }
        count += 1;
    }
    Ok(count)
}

/// Batch-export a set of nodes into `out_dir`. Each node uses its own
/// `export_settings`; a node with none falls back to a single 1x PNG. Returns
/// the total number of files written.
pub fn batch_export_nodes(
    doc: &Node,
    nodes: &[Node],
    vars: &Variables,
    assets: &x_native::Assets,
    fonts: &x_native::text::FontManager,
    out_dir: &str,
) -> Result<usize, String> {
    let mut count = 0usize;
    for node in nodes {
        if node.export_settings.is_empty() {
            let path = std::path::Path::new(out_dir).join(format!("{}.png", node.id));
            export_raster_file(
                doc,
                node,
                vars,
                assets,
                fonts,
                path.to_string_lossy().as_ref(),
                x_native::RasterFormat::Png,
                1.0,
            )?;
            count += 1;
        } else {
            count += export_node_settings(doc, node, vars, assets, fonts, out_dir)?;
        }
    }
    Ok(count)
}

/// CPU raster export of a node's subtree to a file (PNG or JPG) at a scale
/// factor. `doc` is the full document (for component resolution); `node` is
/// the subtree to render — the whole page when `node == doc`, otherwise that
/// node at its own origin (its size, rotation kept, position zeroed). No GPU
/// is required, so export works headless and is deterministic.
#[allow(clippy::too_many_arguments)]
pub fn export_raster_file(
    doc: &Node,
    node: &Node,
    vars: &Variables,
    assets: &x_native::Assets,
    fonts: &x_native::text::FontManager,
    path: &str,
    format: x_native::RasterFormat,
    scale: f64,
) -> Result<(u32, u32), String> {
    // Slices export the flattened canvas under their bounds, not themselves.
    let is_slice = matches!(node.kind, x_native::NodeKind::Slice);
    let (tree, w, h) = if is_slice {
        let (tree, w, h) = x_native::build_render_tree_slice(doc, &node.id, vars)
            .ok_or("slice not found for export")?;
        (tree, w, h)
    } else if node.id == doc.id {
        (x_native::build_render_tree(doc, vars), node.w, node.h)
    } else {
        (
            x_native::build_render_tree_of(doc, &node.id, vars)
                .ok_or("node not found for export")?,
            node.w,
            node.h,
        )
    };
    // whole-page export keeps the old opaque-white look; a single node exports
    // transparent (it draws its own fill) — JPG always composites on white.
    let background = match format {
        x_native::RasterFormat::Png if node.id == doc.id => Some(Color::WHITE),
        x_native::RasterFormat::Jpg(_) => Some(Color::WHITE),
        _ => None,
    };
    let (bytes, w, h) = x_native::export_raster(
        &tree,
        w,
        h,
        format,
        scale,
        background,
        Some(assets),
        Some(fonts),
    )?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    Ok((w, h))
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
        s.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            c,
            None,
            &vello::kurbo::Circle::new((cx, cy), 1.7),
        );
    } else {
        // slash through
        s.stroke(
            &st,
            Affine::IDENTITY,
            c,
            None,
            &vello::kurbo::Line::new((cx - 5.0, cy + 4.5), (cx + 5.0, cy - 4.5)),
        );
    }
}

/// Small bordered stepper button with a centered - or + glyph.
pub fn draw_stepper(s: &mut Scene, r: Rect, plus: bool, hover: bool, c: Color) {
    if hover {
        fill_rrect(s, r, 4.0, C_HOVER);
    }
    let st = vello::kurbo::Stroke::new(1.2).with_caps(vello::kurbo::Cap::Round);
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    s.stroke(
        &st,
        Affine::IDENTITY,
        c,
        None,
        &vello::kurbo::Line::new((cx - 3.0, cy), (cx + 3.0, cy)),
    );
    if plus {
        s.stroke(
            &st,
            Affine::IDENTITY,
            c,
            None,
            &vello::kurbo::Line::new((cx, cy - 3.0), (cx, cy + 3.0)),
        );
    }
}

/// Alignment icon (bar + object box) for the 6-button align row.
/// i: 0 left, 1 center-h, 2 right, 3 top, 4 middle, 5 bottom.
pub fn draw_align_icon(s: &mut Scene, i: usize, r: Rect, c: Color) {
    let st = vello::kurbo::Stroke::new(1.2).with_caps(vello::kurbo::Cap::Round);
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    let bar = |s: &mut Scene, a: (f64, f64), b: (f64, f64)| {
        s.stroke(
            &st,
            Affine::IDENTITY,
            c,
            None,
            &vello::kurbo::Line::new(a, b),
        );
    };
    let boxf = |s: &mut Scene, x0: f64, y0: f64, x1: f64, y1: f64| {
        s.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            c,
            None,
            &vello::kurbo::RoundedRect::new(x0, y0, x1, y1, 1.0),
        );
    };
    match i {
        0 => {
            bar(s, (cx - 6.0, cy - 5.5), (cx - 6.0, cy + 5.5));
            boxf(s, cx - 3.5, cy - 3.0, cx + 6.0, cy + 3.0);
        }
        1 => {
            bar(s, (cx, cy - 5.5), (cx, cy + 5.5));
            boxf(s, cx - 4.5, cy - 3.0, cx + 4.5, cy + 3.0);
        }
        2 => {
            bar(s, (cx + 6.0, cy - 5.5), (cx + 6.0, cy + 5.5));
            boxf(s, cx - 6.0, cy - 3.0, cx + 3.5, cy + 3.0);
        }
        3 => {
            bar(s, (cx - 5.5, cy - 6.0), (cx + 5.5, cy - 6.0));
            boxf(s, cx - 3.0, cy - 3.5, cx + 3.0, cy + 6.0);
        }
        4 => {
            bar(s, (cx - 5.5, cy), (cx + 5.5, cy));
            boxf(s, cx - 3.0, cy - 4.5, cx + 3.0, cy + 4.5);
        }
        _ => {
            bar(s, (cx - 5.5, cy + 6.0), (cx + 5.5, cy + 6.0));
            boxf(s, cx - 3.0, cy - 6.0, cx + 3.0, cy + 3.5);
        }
    }
}

/// Thin section separator line across the inspector.
pub fn draw_section_sep(s: &mut Scene, ix: f64, win_w: f64, y: f64) {
    fill_rect(s, Rect::new(ix + 8.0, y, win_w - 8.0, y + 1.0), C_EDGE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_export_writes_per_node_files() {
        let doc = Node::frame("page", 400.0, 300.0)
            .child(Node::rect(
                "rect-1",
                0.0,
                0.0,
                50.0,
                50.0,
                Color::from_rgb8(255, 0, 0),
            ))
            .child(Node::slice("slice-1", 100.0, 100.0, 80.0, 60.0));
        // give the slice two presets; the rect gets the default 1x PNG
        let mut nodes = vec![];
        fn walk(n: &Node, out: &mut Vec<Node>) {
            for c in &n.children {
                if !c.export_settings.is_empty() {
                    out.push(c.clone());
                }
                walk(c, out);
            }
        }
        // build a doc clone with settings attached to the slice
        let mut d = doc.clone();
        if let Some(s) = x_native::editor::find_mut(&mut d, "slice-1") {
            s.export_settings = vec![
                ExportSettings {
                    format: "png".into(),
                    scale: 1.0,
                    quality: 90,
                    suffix: "".into(),
                },
                ExportSettings {
                    format: "png".into(),
                    scale: 2.0,
                    quality: 90,
                    suffix: "@2x".into(),
                },
            ];
        }
        walk(&d, &mut nodes);
        // batch with both the slice (has settings) and the rect (defaults to 1x png)
        let rect = x_native::editor::find(&d, "rect-1").unwrap().clone();
        let batch = vec![nodes[0].clone(), rect];
        let dir = std::env::temp_dir().join(format!("xnat_batch_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let vars = Variables::default();
        let assets = x_native::Assets::new();
        let fonts = x_native::text::FontManager::new();
        let count = batch_export_nodes(
            &d,
            &batch,
            &vars,
            &assets,
            &fonts,
            dir.to_string_lossy().as_ref(),
        )
        .unwrap();
        assert_eq!(count, 3, "slice 2 presets + rect default");
        assert!(dir.join("slice-1.png").exists());
        assert!(dir.join("slice-1@2x.png").exists());
        assert!(dir.join("rect-1.png").exists());
        // PNG magic on one output
        let bytes = std::fs::read(dir.join("slice-1.png")).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
