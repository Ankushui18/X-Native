#[allow(unused_imports)]
use super::*;

/// (stop index being edited, handle A, handle B, all stops)
pub type GradientGeom = (usize, Point, Point, Vec<(f32, Color)>);

impl App {
    /// Shared creation geometry for preview and commit.
    /// Shift constrains shapes to a square/circle; Alt draws from center.
    pub fn creation_rect(&self, start: Point, end: Point) -> Rect {
        let mut dx = end.x - start.x;
        let mut dy = end.y - start.y;
        if self.shift && self.tool == Tool::Line {
            let length = (dx * dx + dy * dy).sqrt();
            let step = std::f64::consts::FRAC_PI_4;
            let angle = (dy.atan2(dx) / step).round() * step;
            dx = angle.cos() * length;
            dy = angle.sin() * length;
        } else if self.shift && !matches!(self.tool, Tool::Text) {
            let side = dx.abs().max(dy.abs());
            dx = if dx < 0.0 { -side } else { side };
            dy = if dy < 0.0 { -side } else { side };
        }
        let (a, b) = if self.alt {
            (
                Point::new(start.x - dx, start.y - dy),
                Point::new(start.x + dx, start.y + dy),
            )
        } else {
            (start, Point::new(start.x + dx, start.y + dy))
        };
        Rect::new(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y))
    }

    pub fn canvas_origin(&self) -> (f64, f64) {
        if self.chrome_hidden {
            (0.0, 0.0)
        } else {
            (TOOLBAR_W + LAYERS_W, TOP_H)
        }
    }
    /// effective height of the bottom pages strip (collapsible — Figma
    /// gives the canvas maximum viewport; state persists in .xprefs)
    // Figma has no bottom artboard-thumbnail strip — pages are switched
    // from the simple list at the top of the Layers panel instead, so no
    // vertical space is reserved for a strip anymore.
    pub fn thumbs_h(&self) -> f64 {
        0.0
    }

    pub fn canvas_rect(&self) -> Rect {
        if self.chrome_hidden {
            return Rect::new(0.0, 0.0, self.win_w, self.win_h);
        }
        Rect::new(
            TOOLBAR_W + LAYERS_W,
            TOP_H,
            self.win_w - INSPECTOR_W,
            self.win_h - self.thumbs_h() - STATUS_H,
        )
    }

    /// bottom page-thumbnail strip (mockup); collapsed = slim toggle bar
    pub fn thumbs_rect(&self) -> Rect {
        Rect::new(
            LAYERS_W,
            self.win_h - self.thumbs_h() - STATUS_H,
            self.win_w - INSPECTOR_W,
            self.win_h - STATUS_H,
        )
    }

    pub fn toggle_thumbs(&mut self) {
        self.thumbs_collapsed = !self.thumbs_collapsed;
        let _ = std::fs::write(
            ".xprefs",
            if self.thumbs_collapsed {
                "thumbs=collapsed"
            } else {
                "thumbs=open"
            },
        );
        self.status = if self.thumbs_collapsed {
            "pages panel collapsed".into()
        } else {
            "pages panel shown".into()
        };
    }
    /// minimap minimap rect (bottom-right of the canvas).
    pub fn minimap_rect(&self) -> Rect {
        let c = self.canvas_rect();
        Rect::new(
            c.x1 - 176.0,
            c.y1 - BOTTOM_BAR_H - 116.0,
            c.x1 - 12.0,
            c.y1 - BOTTOM_BAR_H - 12.0,
        )
    }

    /// Every visible frame in the document with its screen-space top-left
    /// corner, in DOCUMENT order (parents before children, so later entries
    /// paint on top). Figma labels EVERY frame (nested included); this walks
    /// the full tree with the composed world transform, unlike the old
    /// top-level-only loop that used the frame's local x/y.
    pub fn all_frame_labels(&self) -> Vec<(String, String, Point)> {
        fn walk(
            node: &Node,
            parent: Affine,
            camera: Affine,
            out: &mut Vec<(String, String, Point)>,
        ) {
            let world = parent * node.transform.matrix(node.w, node.h);
            if node.visible && matches!(node.kind, x_native::NodeKind::Frame { .. }) {
                let tl = camera * (world * Point::ZERO);
                out.push((node.id.clone(), node.name.clone(), tl));
            }
            for c in &node.children {
                walk(c, world, camera, out);
            }
        }
        let mut out = vec![];
        let cam = self.camera();
        let root_world = self
            .editor
            .root
            .transform
            .matrix(self.editor.root.w, self.editor.root.h);
        for child in &self.editor.root.children {
            walk(child, root_world, cam, &mut out);
        }
        out
    }

    /// Screen-space bounds for every visible slice (dashed outline + label on
    /// the canvas, like Figma). Returns (id, quad-bounds-in-screen-space).
    pub fn all_slice_bounds(&self) -> Vec<(String, Rect)> {
        fn walk(node: &Node, parent: Affine, camera: Affine, out: &mut Vec<(String, Rect)>) {
            let world = parent * node.transform.matrix(node.w, node.h);
            if node.visible && matches!(node.kind, x_native::NodeKind::Slice) {
                out.push((node.id.clone(), quad_bounds(camera * world, node.w, node.h)));
            }
            for c in &node.children {
                walk(c, world, camera, out);
            }
        }
        let mut out = vec![];
        let cam = self.camera();
        let root_world = self
            .editor
            .root
            .transform
            .matrix(self.editor.root.w, self.editor.root.h);
        for child in &self.editor.root.children {
            walk(child, root_world, cam, &mut out);
        }
        out
    }

    /// Screen-space hit rects for the interactive frame name labels (Figma's
    /// "◇ Frame name" floating above each frame), TOPMOST-first so a nested
    /// frame's label takes priority over its parent's. Mirrors the draw loop
    /// geometry exactly. Click = select + drag; double-click = inline rename.
    pub fn frame_label_rects(&self) -> Vec<(String, Rect)> {
        let mut out = vec![];
        for (id, name, tl) in self.all_frame_labels() {
            if tl.y < TOP_H + 26.0 {
                continue;
            }
            // label drawn at (tl.x + 14, tl.y - 18) at 8.5px, diamond at
            // (tl.x + 5, tl.y - 12); pad generously for grabbing.
            let w = ui_measure(&name, 8.5);
            let r = Rect::new(tl.x + 1.0, tl.y - 26.0, tl.x + 14.0 + w + 6.0, tl.y - 2.0);
            out.push((id, r));
        }
        out.reverse(); // topmost (deepest) first
        out
    }

    /// Figma-style floating toolbar, centered at the bottom of the canvas.
    pub fn bottom_bar_rect(&self) -> Rect {
        // tool row now lives centered in the header's second row (mockup);
        // pitch 40 (34px slot + 6px gap) + 13px side padding each end —
        // wide enough for all 17 tools (v2 fixed the 38/40 mismatch that
        // pushed the last icon past the pill's right edge)
        let w = Tool::ALL.len() as f64 * 40.0 + 20.0;
        let cx = self.win_w / 2.0;
        Rect::new(cx - w / 2.0, TAB_H + 6.0, cx + w / 2.0, TOP_H - 6.0)
    }
    pub fn camera(&self) -> Affine {
        let (ox, oy) = self.canvas_origin();
        Affine::translate((ox + self.pan.0, oy + self.pan.1)) * Affine::scale(self.zoom)
    }
    pub fn world_point(&self, screen: Point) -> Point {
        let (ox, oy) = self.canvas_origin();
        Point::new(
            (screen.x - ox - self.pan.0) / self.zoom,
            (screen.y - oy - self.pan.1) / self.zoom,
        )
    }

    pub fn rebuild_layer_rows(&mut self) {
        // virtualization: skip the full-tree walk when nothing changed
        // (undo depth + selection hash + filter act as the fingerprint)
        let fp = (
            self.editor.undo_depth(),
            self.layer_filter.clone(),
            self.editor.root.children.len(),
        );
        if self.layer_rows_fp == Some(fp.clone()) && !self.layer_rows.is_empty() {
            return;
        }
        self.layer_rows_fp = Some(fp);
        fn walk(n: &Node, depth: usize, out: &mut Vec<(String, String, usize, &'static str)>) {
            out.push((n.id.clone(), n.name.clone(), depth, kind_label(n)));
            for c in &n.children {
                walk(c, depth + 1, out);
            }
        }
        let mut rows = vec![];
        walk(&self.editor.root, 0, &mut rows);
        if !self.layer_filter.is_empty() {
            let q = self.layer_filter.to_ascii_lowercase();
            rows.retain(|(id, name, _, k)| {
                id.to_ascii_lowercase().contains(&q)
                    || name.to_ascii_lowercase().contains(&q)
                    || k.to_ascii_lowercase().contains(&q)
            });
        }
        self.layer_rows = rows;
    }

    /// Grouped styles-browser layout: single source of truth shared by the
    /// chrome painter and the click hit-test (prevents geometry drift).
    /// Returns rows of (name, kind_label, chip_rect, is_header).
    /// Rows of (name, kind_label, rect, row_kind) — row_kind: 0 chip,
    /// 1 header, 2 search box, 3 management action (name = action tag).
    pub fn styles_layout(&self) -> Vec<(String, &'static str, Rect, u8)> {
        let ix = self.win_w - INSPECTOR_W;
        let sy0 = TOP_H + IY_STYLES;
        let mut out = vec![];
        // search box row
        let mut cy = sy0 + 16.0;
        out.push((
            String::new(),
            "SEARCH",
            Rect::new(ix + 12.0, cy - 2.0, self.win_w - 12.0, cy + 12.0),
            2,
        ));
        cy += 20.0;
        let q = self.style_query.to_ascii_lowercase();
        let mut names: Vec<&String> = self
            .styles
            .keys()
            .filter(|n| q.is_empty() || n.to_ascii_lowercase().contains(&q))
            .collect();
        names.sort();
        for (kind, _header) in [
            ("PAINT", "PAINT STYLES"),
            ("TEXT", "TEXT STYLES"),
            ("FX", "EFFECT STYLES"),
        ] {
            let group: Vec<&&String> = names
                .iter()
                .filter(|n| self.styles[n.as_str()].kind_label() == kind)
                .collect();
            if group.is_empty() {
                continue;
            }
            // mockup-compact: no group header rows — chips flow inline
            let mut cx = ix + 12.0;
            for name in group.into_iter().take(8) {
                let short = if name.len() > 12 {
                    &name[..12]
                } else {
                    name.as_str()
                };
                // chip text includes the usage count ("Primary 3");
                // current page counts from the LIVE editor tree
                let usage: usize = self
                    .pages
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        if i == self.page_idx {
                            x_native::style_usage(&self.editor.root, name)
                        } else {
                            x_native::style_usage(p, name)
                        }
                    })
                    .sum();
                let text = if usage > 0 {
                    format!("{short} {usage}")
                } else {
                    short.to_string()
                };
                let cw = x_native::text::measure(&text, 7.0) + 18.0;
                if cx + cw > self.win_w - 8.0 {
                    cx = ix + 12.0;
                    cy += 18.0;
                }
                out.push((
                    (*name).clone(),
                    kind,
                    Rect::new(cx, cy - 2.0, cx + cw, cy + 12.0),
                    0,
                ));
                cx += cw + 4.0;
            }
            cy += 18.0;
        }
        // management row for the selected style: REN DUP DEL DET
        if let Some(sel) = &self.style_sel {
            if self.styles.contains_key(sel) {
                let mut bx = ix + 12.0;
                for act in ["REN", "DUP", "DEL", "DET"] {
                    out.push((
                        act.to_string(),
                        "ACT",
                        Rect::new(bx, cy - 2.0, bx + 34.0, cy + 12.0),
                        3,
                    ));
                    bx += 38.0;
                }
            }
        }
        out
    }

    // -------------------------------------------------------- asset browser

    /// Overlay panel rect (Shift+A).
    pub fn asset_panel_rect(&self) -> Rect {
        let w = 560.0f64.min(self.win_w - 160.0);
        let h = 420.0f64.min(self.win_h - 160.0);
        let x0 = (self.win_w - w) / 2.0;
        let y0 = (self.win_h - h) / 2.0;
        Rect::new(x0, y0, x0 + w, y0 + h)
    }

    /// Shared layout for the asset browser (painter + hit-test):
    /// rows of (tag, rect, kind) — kind: 0 tile, 1 search, 2 action.
    /// Tag is the asset id for tiles, action name for actions.
    pub fn asset_layout(&self) -> Vec<(String, Rect, u8)> {
        let panel = self.asset_panel_rect();
        let mut out = vec![];
        // search box
        out.push((
            String::new(),
            Rect::new(
                panel.x0 + 16.0,
                panel.y0 + 34.0,
                panel.x1 - 16.0,
                panel.y0 + 52.0,
            ),
            1,
        ));
        // action row (operates on asset_sel) + sort chips
        let mut bx = panel.x0 + 16.0;
        for act in ["PLACE", "REPLACE", "RENAME", "DEL UNUSED"] {
            let w = x_native::text::measure(act, 8.0) + 16.0;
            out.push((
                act.to_string(),
                Rect::new(bx, panel.y0 + 58.0, bx + w, panel.y0 + 74.0),
                2,
            ));
            bx += w + 8.0;
        }
        for (i, srt) in ["NAME", "SIZE", "USED"].iter().enumerate() {
            let w = x_native::text::measure(srt, 7.5) + 12.0;
            out.push((
                format!("SORT{i}"),
                Rect::new(bx, panel.y0 + 58.0, bx + w, panel.y0 + 74.0),
                2,
            ));
            bx += w + 6.0;
        }
        // thumbnail grid: filtered, SORTED (name/size/usage), SCROLLED
        let recs = self.sorted_assets();
        let cell = 120.0;
        let cols = ((panel.width() - 32.0) / (cell + 10.0)).floor().max(1.0) as usize;
        let row_h = cell * 0.75 + 26.0;
        let visible_rows = ((panel.y1 - 10.0 - (panel.y0 + 86.0)) / row_h)
            .floor()
            .max(1.0) as usize;
        let start = self.asset_scroll * cols;
        for (i, id) in recs
            .iter()
            .enumerate()
            .skip(start)
            .take(cols * visible_rows)
        {
            let vi = i - start;
            let (col, row) = (vi % cols, vi / cols);
            let x = panel.x0 + 16.0 + col as f64 * (cell + 10.0);
            let y = panel.y0 + 86.0 + row as f64 * row_h;
            out.push((id.1.clone(), Rect::new(x, y, x + cell, y + cell * 0.75), 0));
            let _ = id.0;
        }
        out
    }

    /// Filtered + sorted asset ids for the browser (sort: 0 name, 1 size
    /// desc, 2 usage desc). Returned as (sort_key_debug, id).
    pub fn sorted_assets(&self) -> Vec<(String, String)> {
        let q = self.asset_query.to_ascii_lowercase();
        let mut recs: Vec<&x_native::AssetRecord> = self
            .store
            .iter_sorted()
            .into_iter()
            .filter(|r| {
                q.is_empty() || r.name.to_ascii_lowercase().contains(&q) || r.mime.contains(&q)
            })
            .collect();
        match self.asset_sort {
            1 => recs.sort_by(|a, b| b.bytes.len().cmp(&a.bytes.len()).then(a.id.cmp(&b.id))),
            2 => {
                let usage = |id: &str| -> usize {
                    self.pages
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            if i == self.page_idx {
                                x_native::asset_usage(&self.editor.root, id)
                            } else {
                                x_native::asset_usage(p, id)
                            }
                        })
                        .sum()
                };
                recs.sort_by(|a, b| usage(&b.id).cmp(&usage(&a.id)).then(a.id.cmp(&b.id)));
            }
            _ => recs.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id))),
        }
        recs.into_iter()
            .map(|r| (r.name.clone(), r.id.clone()))
            .collect()
    }

    /// Asset browser actions (asset_sel target).
    pub fn run_asset_action(&mut self, act: &str) {
        match act {
            "PLACE" => {
                let Some(id) = self.asset_sel.clone() else {
                    self.status = "select an asset tile first".into();
                    return;
                };
                let dims = self
                    .store
                    .get(&id)
                    .and_then(|r| r.dimensions)
                    .unwrap_or((160, 120));
                self.created_count += 1;
                let nid = format!("image-{}", self.created_count);
                let mut n = Node::image(&nid, 60.0, 60.0, dims.0 as f64, dims.1 as f64, &id);
                n.transform.x = 60.0;
                n.transform.y = 60.0;
                let root_id = self.editor.root.id.clone();
                self.editor.insert_node(&root_id, n);
                self.editor.selection = vec![nid.clone()];
                self.asset_browser = false;
                self.status = format!("placed {nid} on canvas");
            }
            "REPLACE" => {
                let Some(id) = self.asset_sel.clone() else {
                    self.status = "select an asset tile first".into();
                    return;
                };
                let Some(sel) = self.editor.selection.first().cloned() else {
                    self.status = "select an image layer on canvas first".into();
                    return;
                };
                if let Some(nm) = x_native::editor::find_mut(&mut self.editor.root, &sel) {
                    if let x_native::NodeKind::Image { asset, .. } = &mut nm.kind {
                        *asset = id.clone();
                        nm.dirty = true;
                        self.status = format!("{sel} now uses {}", &id[..24.min(id.len())]);
                        return;
                    }
                }
                self.status = "selected layer is not an image".into();
            }
            "RENAME" => {
                let Some(id) = self.asset_sel.clone() else {
                    self.status = "select an asset tile first".into();
                    return;
                };
                // rename polish: start EMPTY (select-all semantics) — typing
                // replaces the old name instead of appending to it
                self.focus = Focus::AssetRename {
                    id,
                    buffer: String::new(),
                };
                self.status = "type the new display name, Enter to commit (empty = keep)".into();
            }
            "DEL UNUSED" => {
                let mut used = std::collections::HashSet::new();
                x_native::collect_asset_ids(&self.editor.root, &mut used);
                for (i, p) in self.pages.iter().enumerate() {
                    if i != self.page_idx {
                        x_native::collect_asset_ids(p, &mut used);
                    }
                }
                let dropped = self.store.retain_used(&used);
                if self
                    .asset_sel
                    .as_deref()
                    .is_some_and(|s| self.store.get(s).is_none())
                {
                    self.asset_sel = None;
                }
                self.status = format!("deleted {dropped} unused asset(s)");
            }
            _ => {}
        }
    }

    /// Clicks inside the asset browser overlay.
    pub fn click_asset_browser(&mut self, p: Point) {
        let panel = self.asset_panel_rect();
        if !panel.contains(p) {
            self.asset_browser = false;
            return;
        }
        for (tag, r, kind) in self.asset_layout() {
            if !r.contains(p) {
                continue;
            }
            match kind {
                1 => {
                    self.focus = Focus::AssetSearch;
                    self.status = "type to filter assets".into();
                }
                2 => {
                    if let Some(srt) = tag.strip_prefix("SORT") {
                        self.asset_sort = srt.parse().unwrap_or(0);
                        self.asset_scroll = 0;
                        self.status = format!(
                            "sorted by {}",
                            ["name", "size", "usage"][self.asset_sort as usize]
                        );
                    } else {
                        self.run_asset_action(&tag);
                    }
                }
                _ => {
                    // tile: select; double-click places
                    let double = self.last_click.elapsed().as_millis() < 400;
                    let _ = double;
                    self.asset_sel = Some(tag.clone());
                    self.asset_drag = Some(tag.clone()); // drag-to-canvas armed
                    let r = self.store.get(&tag);
                    self.status = match r {
                        Some(rec) => format!(
                            "{} | {} | {}x{} | used {}x",
                            rec.name,
                            rec.mime,
                            rec.dimensions.map(|d| d.0).unwrap_or(0),
                            rec.dimensions.map(|d| d.1).unwrap_or(0),
                            self.pages
                                .iter()
                                .enumerate()
                                .map(|(i, pg)| if i == self.page_idx {
                                    x_native::asset_usage(&self.editor.root, &tag)
                                } else {
                                    x_native::asset_usage(pg, &tag)
                                })
                                .sum::<usize>()
                        ),
                        None => "asset missing".into(),
                    };
                }
            }
            return;
        }
    }

    // ------------------------------------------------------------ libraries

    /// Shared LIBS-tab layout (painter + hit-test, no drift): rows of
    /// (tag, rect, kind) — kind: 0 header-text, 1 link-btn, 2 check-btn,
    /// 3 update-banner (tag=library_id), 4 component chip
    /// (tag="lib|comp"), 5 plain text row.
    pub fn libs_layout(&self) -> Vec<(String, Rect, u8)> {
        let ix = self.win_w - INSPECTOR_W;
        let mut out = vec![];
        let iy = TOP_H + 36.0;
        out.push((
            "LIBRARIES".into(),
            Rect::new(ix + 12.0, iy - 3.0, ix + 80.0, iy + 11.0),
            0,
        ));
        out.push((
            "LINK .XLIB".into(),
            Rect::new(ix + 84.0, iy - 3.0, ix + 146.0, iy + 11.0),
            1,
        ));
        out.push((
            "CHECK UPD".into(),
            Rect::new(ix + 150.0, iy - 3.0, ix + 212.0, iy + 11.0),
            2,
        ));
        out.push((
            "PUB .XLIB".into(),
            Rect::new(ix + 216.0, iy - 3.0, ix + 278.0, iy + 11.0),
            7,
        ));
        // search box: filters components/styles/variables across libraries
        let search_txt = if self.lib_query.is_empty() {
            "SEARCH LIBS…".to_string()
        } else {
            format!("SEARCH: {}", self.lib_query)
        };
        out.push((
            search_txt,
            Rect::new(ix + 12.0, iy + 15.0, ix + 212.0, iy + 29.0),
            6,
        ));
        let mut y = iy + 36.0;
        let q = self.lib_query.to_lowercase();
        if !q.is_empty() {
            y += 14.0; // RESULTS header row
                       // ---- filtered flat results across every linked library ----
            let mut hits = 0usize;
            for dep in &self.library_deps {
                let Some(lib) = self.library_snapshots.get(&dep.library_id) else {
                    continue;
                };
                for c in lib.components.iter().take(24) {
                    if let x_native::NodeKind::Component { name } = &c.kind {
                        if name.to_lowercase().contains(&q) {
                            let w = x_native::text::measure(name, 8.0) + 18.0;
                            out.push((
                                format!("{}|{name}", dep.library_id),
                                Rect::new(ix + 12.0, y - 2.0, ix + 12.0 + w, y + 12.0),
                                4,
                            ));
                            y += 16.0;
                            hits += 1;
                        }
                    }
                }
                let mut names: Vec<&String> = lib.styles.keys().collect();
                names.sort();
                for nm in names.iter().take(24) {
                    if nm.to_lowercase().contains(&q) {
                        let short = nm.split_once('/').map(|(_, v)| v).unwrap_or(nm);
                        let w = x_native::text::measure(short, 8.0) + 24.0;
                        out.push((
                            format!("{}|sty|{nm}", dep.library_id),
                            Rect::new(ix + 12.0, y - 2.0, ix + 12.0 + w, y + 12.0),
                            8,
                        ));
                        y += 16.0;
                        hits += 1;
                    }
                }
                let vars = lib.variables.catalog();
                for (_, nm, _) in vars.iter().take(24) {
                    if nm.to_lowercase().contains(&q) {
                        out.push((
                            format!("  var {nm}"),
                            Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0),
                            5,
                        ));
                        y += 12.0;
                        hits += 1;
                    }
                }
            }
            out.insert(
                0,
                (
                    format!("RESULTS ({hits})"),
                    Rect::new(ix + 12.0, iy + 36.0, ix + 90.0, iy + 48.0),
                    5,
                ),
            );
            return out;
        }
        for dep in &self.library_deps {
            let Some(lib) = self.library_snapshots.get(&dep.library_id) else {
                continue;
            };
            out.push((
                lib.name.clone(),
                Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 24.0),
                0,
            ));
            if let Some((_, newer, _)) = &self.library_update {
                if newer.library_id == dep.library_id {
                    out.push((
                        dep.library_id.clone(),
                        Rect::new(ix + 12.0, y + 26.0, self.win_w - 12.0, y + 44.0),
                        3,
                    ));
                    y += 22.0;
                }
            }
            y += 30.0;
            // usage stats: style bindings + placed component instances
            let comps = lib.components.len();
            let styles_n = lib.styles.len();
            let vars_n = lib.variables.catalog().len();
            let uses = self.library_use_count(lib);
            out.push((
                format!(
                    "  v{} · {comps} comps · {styles_n} styles · {vars_n} vars · {uses} uses",
                    lib.version
                ),
                Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0),
                5,
            ));
            y += 14.0;
            out.push((
                "STYLES — CLICK TO APPLY".into(),
                Rect::new(ix + 12.0, y - 2.0, ix + 130.0, y + 10.0),
                5,
            ));
            y += 13.0;
            let mut names: Vec<&String> = lib.styles.keys().collect();
            names.sort();
            for nm in names.iter().take(5) {
                let short = nm.split_once('/').map(|(_, v)| v).unwrap_or(nm);
                let w = x_native::text::measure(short, 8.0) + 24.0;
                out.push((
                    format!("{}|sty|{nm}", dep.library_id),
                    Rect::new(ix + 12.0, y - 2.0, ix + 12.0 + w, y + 12.0),
                    8,
                ));
                y += 16.0;
            }
            let vars_n = lib.variables.colors.len() + lib.variables.numbers.len();
            if vars_n > 0 {
                out.push((
                    format!("VARIABLES ({vars_n})"),
                    Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0),
                    5,
                ));
                y += 13.0;
            }
            if !lib.components.is_empty() {
                out.push((
                    "COMPONENTS — CLICK TO PLACE".into(),
                    Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0),
                    5,
                ));
                y += 13.0;
                for c in lib.components.iter().take(6) {
                    if let x_native::NodeKind::Component { name } = &c.kind {
                        let w = x_native::text::measure(name, 8.0) + 18.0;
                        out.push((
                            format!("{}|{name}", dep.library_id),
                            Rect::new(ix + 12.0, y - 2.0, ix + 12.0 + w, y + 12.0),
                            4,
                        ));
                        y += 18.0;
                    }
                }
            }
            if !lib.assets.is_empty() {
                out.push((
                    format!("ASSETS ({}) — SHIFT+A BROWSER", lib.assets.len()),
                    Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0),
                    5,
                ));
                y += 13.0;
            }
            y += 8.0;
        }
        out
    }

    /// Link `library.xlib` from the working dir: pin its version, snapshot
    /// it, record the integrity hash, register its assets.
    pub fn link_library(&mut self) {
        let Ok(text) = std::fs::read_to_string("library.xlib") else {
            self.status = "no library.xlib in working dir".into();
            return;
        };
        match x_native::fileio::load_xlib(&text) {
            Ok(lib) => {
                if self
                    .library_deps
                    .iter()
                    .any(|d| d.library_id == lib.library_id)
                {
                    self.status = format!("{} already linked — use CHECK UPDATES", lib.name);
                    return;
                }
                // library assets flow into the document store (content-
                // addressed: dedup is automatic) and the render cache
                for rec in lib.assets.iter_sorted() {
                    self.store
                        .register(&rec.name, rec.bytes.clone(), rec.source);
                }
                self.assets.sync_store(&self.store);
                let dep = x_native::LibraryDependency {
                    library_id: lib.library_id.clone(),
                    resolved_version: lib.version,
                    snapshot_hash: x_native::fileio::library_hash(&lib),
                    source_path: "library.xlib".into(),
                };
                self.status = format!(
                    "linked {} v{} ({} styles, {} components)",
                    lib.name,
                    lib.version,
                    lib.styles.len(),
                    lib.components.len()
                );
                self.library_snapshots.insert(lib.library_id.clone(), lib);
                self.library_deps.push(dep);
            }
            Err(e) => self.status = format!("xlib load FAILED: {e}"),
        }
    }

    /// Re-read each dep's source_path; if a newer version exists, stage it
    /// for review (diff via the ENGINE's diff_library — no app-level diff).
    pub fn check_library_updates(&mut self) {
        for (i, dep) in self.library_deps.iter().enumerate() {
            let Ok(text) = std::fs::read_to_string(&dep.source_path) else {
                continue;
            };
            let Ok(newer) = x_native::fileio::load_xlib(&text) else {
                continue;
            };
            if newer.library_id == dep.library_id && newer.version > dep.resolved_version {
                let pinned = &self.library_snapshots[&dep.library_id];
                let changes = x_native::diff_library(pinned, &newer);
                self.status = format!(
                    "update available: {} v{} -> v{} ({} change(s))",
                    newer.name,
                    dep.resolved_version,
                    newer.version,
                    changes.len()
                );
                self.library_update = Some((i, newer, changes));
                return;
            }
        }
        self.status = "all libraries up to date".into();
    }

    /// Accept the staged update — a straight call into the engine's
    /// accept_update (repins + swaps snapshot + re-resolves consumers).
    pub fn accept_library_update(&mut self) {
        let Some((idx, newer, _)) = self.library_update.take() else {
            return;
        };
        self.library_review = false;
        let dep = &mut self.library_deps[idx];
        // pages incl. the live editor tree
        let mut all: Vec<Node> = vec![self.editor.root.clone()];
        for (i, p) in self.pages.iter().enumerate() {
            if i != self.page_idx {
                all.push(p.clone());
            }
        }
        let new_hash = x_native::fileio::library_hash(&newer);
        let (changes, updated) =
            x_native::accept_update(dep, &mut self.library_snapshots, &mut all, newer);
        dep.snapshot_hash = new_hash;
        let mut it = all.into_iter();
        self.editor.root = it.next().unwrap();
        for (i, p) in self.pages.iter_mut().enumerate() {
            if i != self.page_idx {
                if let Some(np) = it.next() {
                    *p = np;
                }
            }
        }
        self.status = format!(
            "accepted v{}: {} change(s), {updated} consumer(s) updated",
            dep.resolved_version,
            changes.len()
        );
    }

    /// Place an instance of a LIBRARY component: the master is added to
    /// the page's component registry ONCE (hidden), instances reference it
    /// by name — same dependency semantics as styles, no per-instance clone.
    pub fn place_library_component(&mut self, lib_id: &str, comp_name: &str) {
        let Some(lib) = self.library_snapshots.get(lib_id) else {
            return;
        };
        let Some(master) = lib.components.iter().find(
            |c| matches!(&c.kind, x_native::NodeKind::Component { name } if name == comp_name),
        ) else {
            return;
        };
        // registry: one hidden master per (library, component), stable id
        let reg_id = format!("libmaster-{lib_id}-{comp_name}");
        if x_native::editor::find(&self.editor.root, &reg_id).is_none() {
            let mut m = master.clone();
            m.id = reg_id.clone();
            m.name = comp_name.to_string();
            m.visible = false; // registry entry, not page content
            let root_id = self.editor.root.id.clone();
            self.editor.insert_node(&root_id, m);
        }
        self.created_count += 1;
        let iid = format!("lib-inst-{}", self.created_count);
        let inst = Node::instance(&iid, comp_name, 80.0, 80.0, master.w, master.h);
        let root_id = self.editor.root.id.clone();
        self.editor.insert_node(&root_id, inst);
        self.editor.selection = vec![iid.clone()];
        self.status = format!("placed {comp_name} instance from {lib_id}");
    }

    /// Total uses of a library in the open document: nodes bound to any of
    /// its styles plus instances of its components (only when placed from
    /// this library — the registry master must exist).
    pub fn library_use_count(&self, lib: &x_native::Library) -> usize {
        let mut uses = 0usize;
        for name in lib.styles.keys() {
            let r = x_native::LibraryRef::style(&lib.library_id, name);
            uses += x_native::library_style_usage(&self.editor.root, &r);
            for (i, pg) in self.pages.iter().enumerate() {
                if i != self.page_idx {
                    uses += x_native::library_style_usage(pg, &r);
                }
            }
        }
        for c in &lib.components {
            if let x_native::NodeKind::Component { name } = &c.kind {
                let reg = format!("libmaster-{}-{name}", lib.library_id);
                if find(&self.editor.root, &reg).is_some() {
                    uses += x_native::count_instances(&self.editor.root, name);
                    for (i, pg) in self.pages.iter().enumerate() {
                        if i != self.page_idx {
                            uses += x_native::count_instances(pg, name);
                        }
                    }
                }
            }
        }
        uses
    }

    /// Bind the selected node to a library style (click a style chip in the
    /// LIBS tab): pins the `library://id/style/name` ref and applies the
    /// pinned definition immediately.
    pub fn apply_library_style(&mut self, lib_id: &str, style_name: &str) {
        let Some(lib) = self.library_snapshots.get(lib_id) else {
            return;
        };
        let Some(st) = lib.styles.get(style_name) else {
            return;
        };
        let Some(id) = self.editor.selection.first().cloned() else {
            self.status = "select a layer first".into();
            return;
        };
        let key = x_native::binding_key_for(st).to_string();
        let uri = x_native::LibraryRef::style(lib_id, style_name).uri();
        if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
            n.bindings.insert(key, uri);
            x_native::apply_style(n, st);
            self.status = format!("{id} -> library style {style_name}");
        }
    }

    /// Publish the current document as a `.xlib`: styles + variables +
    /// every visible component master. Version bumps when the same library
    /// id is already linked (publish -> check updates -> accept cycle).
    pub fn publish_library(&mut self) {
        let id = "local-kit".to_string();
        let version = self
            .library_deps
            .iter()
            .find(|d| d.library_id == id)
            .map(|d| d.resolved_version + 1)
            .unwrap_or(1);
        let lib = x_native::library_from_parts(
            &self.styles,
            &self.vars,
            &self.pages,
            &id,
            "Local Kit",
            version,
        );
        let path = format!("{id}.xlib");
        match std::fs::write(&path, x_native::fileio::save_xlib(&lib)) {
            Ok(_) => {
                self.status =
                    format!("published {path} v{version} — LINK .XLIB in any doc to use it");
            }
            Err(e) => self.status = format!("publish failed: {e}"),
        }
    }

    /// Style management actions (REN/DUP/DEL/DET) for `style_sel`.
    pub fn run_style_action(&mut self, act: &str) {
        let Some(name) = self.style_sel.clone() else {
            return;
        };
        if !self.styles.contains_key(&name) {
            return;
        }
        match act {
            "REN" => {
                self.focus = Focus::StyleRename {
                    from: name.clone(),
                    buffer: name.clone(),
                };
                self.status = "type the new name, Enter to rename every consumer".into();
            }
            "DUP" => {
                let mut copy = format!("{name} copy");
                let mut n = 2;
                while self.styles.contains_key(&copy) {
                    copy = format!("{name} copy {n}");
                    n += 1;
                }
                let def = self.styles[&name].clone();
                self.styles.insert(copy.clone(), def);
                self.style_sel = Some(copy.clone());
                self.status = format!("duplicated -> {copy}");
            }
            "DEL" => {
                // deleting detaches every consumer first (values keep)
                let mut detached = 0usize;
                fn detach_all(n: &mut Node, name: &str, detached: &mut usize) {
                    for (k, _) in x_native::STYLE_BINDING_KEYS {
                        if n.bindings.get(k).map(String::as_str) == Some(name) {
                            n.bindings.remove(k);
                            *detached += 1;
                        }
                    }
                    for c in &mut n.children {
                        detach_all(c, name, detached);
                    }
                }
                detach_all(&mut self.editor.root, &name, &mut detached);
                for (i, pg) in self.pages.iter_mut().enumerate() {
                    if i != self.page_idx {
                        detach_all(pg, &name, &mut detached);
                    }
                }
                self.styles.remove(&name);
                self.style_sel = None;
                self.status =
                    format!("deleted {name} ({detached} consumer(s) detached, values kept)");
            }
            "DET" => {
                // detach the SELECTED LAYER from this style only
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(nm) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                        let mut done = false;
                        for (k, _) in x_native::STYLE_BINDING_KEYS {
                            if nm.bindings.get(k).map(String::as_str) == Some(name.as_str()) {
                                done |= x_native::detach_style(nm, k);
                            }
                        }
                        self.status = if done {
                            format!("{id} detached from {name} (values kept)")
                        } else {
                            format!("{id} was not bound to {name}")
                        };
                    }
                } else {
                    self.status = "select a layer to detach".into();
                }
            }
            _ => {}
        }
    }

    /// Commit a style rename (Enter in Focus::StyleRename).
    pub fn commit_style_rename(&mut self, from: &str, to: &str) {
        // rename across the LIVE editor tree and all other pages
        let mut all: Vec<Node> = vec![self.editor.root.clone()];
        for (i, p) in self.pages.iter().enumerate() {
            if i != self.page_idx {
                all.push(p.clone());
            }
        }
        match x_native::rename_style(&mut self.styles, &mut all, from, to) {
            Some(rebound) => {
                let mut it = all.into_iter();
                self.editor.root = it.next().unwrap();
                let mut rest = it;
                for (i, p) in self.pages.iter_mut().enumerate() {
                    if i != self.page_idx {
                        if let Some(np) = rest.next() {
                            *p = np;
                        }
                    }
                }
                self.style_sel = Some(to.to_string());
                self.status = format!("renamed {from} -> {to} ({rebound} consumer(s) rebound)");
            }
            None => self.status = "rename refused (empty/duplicate name?)".to_string(),
        }
    }

    pub fn selected_single(&self) -> Option<&Node> {
        if self.editor.selection.len() == 1 {
            find(&self.editor.root, &self.editor.selection[0])
        } else {
            None
        }
    }

    /// Current display value of a component property on an instance: the
    /// instance's override for the bound target wins, else the prop default.
    pub fn prop_value(&self, instance: &Node, prop: &x_native::ComponentProp) -> String {
        use x_native::{typed_overrides, OverrideValue};
        let ov = typed_overrides(instance);
        match prop {
            x_native::ComponentProp::Text {
                target, default, ..
            } => match ov.get(target) {
                Some(OverrideValue::Text(t)) => t.clone(),
                _ => default.clone(),
            },
            x_native::ComponentProp::Bool {
                target, default, ..
            } => match ov.get(target) {
                Some(OverrideValue::Visible(v)) => v.to_string(),
                _ => default.to_string(),
            },
            x_native::ComponentProp::Swap {
                target, default, ..
            } => match ov.get(target) {
                Some(OverrideValue::Swap(c)) => c.clone(),
                _ => default.clone(),
            },
            x_native::ComponentProp::Number {
                target, default, ..
            } => match ov.get(target) {
                Some(OverrideValue::Number(n)) => format!("{n}"),
                _ => format!("{default}"),
            },
            x_native::ComponentProp::Slot { name, default, .. } => {
                match x_native::slot_content(instance, name) {
                    Some(c) => c.name.clone(),
                    None => default.clone().unwrap_or_else(|| "-".into()),
                }
            }
        }
    }

    /// Variant-grid geometry for a set: (variants, prop_names, ix, y0, col_w).
    /// `y0` is the top of the first DATA row; the column header sits 10px
    /// above it. Variants are capped at 10 rows; property names are the union
    /// across variants in first-seen order.
    #[allow(clippy::type_complexity)]
    pub fn variant_grid_layout(
        &self,
        set: &str,
    ) -> Option<(Vec<String>, Vec<String>, f64, f64, f64)> {
        let variants: Vec<String> = x_native::variants_of(&self.editor.root, set)
            .iter()
            .map(|s| s.to_string())
            .take(10)
            .collect();
        if variants.is_empty() {
            return None;
        }
        let mut prop_names: Vec<String> = vec![];
        for v in &variants {
            for p in self.editor.component_props(v) {
                let pn = p.name().to_string();
                if !prop_names.contains(&pn) {
                    prop_names.push(pn);
                }
            }
        }
        let ix = self.win_w - INSPECTOR_W;
        let y0 = TOP_H + IY_SEC + 56.0;
        let name_w = 68.0;
        let col_w = (INSPECTOR_W - 24.0 - name_w) / prop_names.len().max(1) as f64;
        Some((variants, prop_names, ix, y0, col_w))
    }

    /// Fetch a property by name on a variant; if the variant lacks it, clone
    /// its definition from a sibling in the same set (so a clicked empty cell
    /// adopts the shared column).
    pub fn variant_prop(
        &mut self,
        variant: &str,
        set: &str,
        prop_name: &str,
    ) -> Option<x_native::ComponentProp> {
        if let Some(p) = self
            .editor
            .component_props(variant)
            .into_iter()
            .find(|p| p.name() == prop_name)
        {
            return Some(p);
        }
        let variants: Vec<String> = x_native::variants_of(&self.editor.root, set)
            .iter()
            .map(|s| s.to_string())
            .collect();
        for v in &variants {
            if v == variant {
                continue;
            }
            if let Some(tpl) = self
                .editor
                .component_props(v)
                .into_iter()
                .find(|p| p.name() == prop_name)
            {
                let clone = tpl.clone();
                if self.editor.add_component_prop(variant, tpl) {
                    return Some(clone);
                }
            }
        }
        None
    }

    /// Short designer-facing type label for a component property.
    pub fn prop_kind_label(prop: &x_native::ComponentProp) -> &'static str {
        match prop {
            x_native::ComponentProp::Text { .. } => "TEXT",
            x_native::ComponentProp::Bool { .. } => "BOOL",
            x_native::ComponentProp::Swap { .. } => "SWAP",
            x_native::ComponentProp::Number { .. } => "NUM",
            x_native::ComponentProp::Slot { .. } => "SLOT",
        }
    }
    /// Selection AABB in SCREEN space (for handles).
    pub fn selection_screen_bounds(&self) -> Option<Rect> {
        let id = self.editor.selection.first()?;
        let (world, w, h) = world_transform_of(&self.editor.root, id)?;
        Some(quad_bounds(self.camera() * world, w, h))
    }

    /// A node's transform-origin pivot in SCREEN space: the local point
    /// (origin_x*w, origin_y*h) carried through the world + camera matrices.
    pub fn transform_pivot_screen(&self, id: &str) -> Option<Point> {
        let node = find(&self.editor.root, id)?;
        let (world, w, h) = world_transform_of(&self.editor.root, id)?;
        let local = Point::new(node.transform.origin_x * w, node.transform.origin_y * h);
        Some(self.camera() * (world * local))
    }

    /// Figma handle model: 0-3 = corners (TL,TR,BL,BR), 4=left edge,    /// 5=right, 6=top, 7=bottom. Corners win over edges.
    pub fn handle_at(&self, p: Point) -> Option<u8> {
        let b = self.selection_screen_bounds()?;
        let corners = [(b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)];
        for (i, (cx, cy)) in corners.iter().enumerate() {
            if (p.x - cx).abs() <= 6.0 && (p.y - cy).abs() <= 6.0 {
                return Some(i as u8);
            }
        }
        // edges: within 4px of the line, between the corner zones
        let inside_y = p.y > b.y0 + 8.0 && p.y < b.y1 - 8.0;
        let inside_x = p.x > b.x0 + 8.0 && p.x < b.x1 - 8.0;
        if inside_y && (p.x - b.x0).abs() <= 4.0 {
            return Some(4);
        }
        if inside_y && (p.x - b.x1).abs() <= 4.0 {
            return Some(5);
        }
        if inside_x && (p.y - b.y0).abs() <= 4.0 {
            return Some(6);
        }
        if inside_x && (p.y - b.y1).abs() <= 4.0 {
            return Some(7);
        }
        None
    }

    /// Corner-radius handle geometry: four small circular handles just INSIDE
    /// each corner (Figma's radius diamonds) for the selected Rect. Returns
    /// (corner_index 0..3, center) in screen space, ordered TL,TR,BR,BL.
    pub fn radius_handles(&self) -> Vec<(u8, Point)> {
        let Some(id) = self.editor.selection.first().cloned() else {
            return vec![];
        };
        let Some(n) = find(&self.editor.root, &id) else {
            return vec![];
        };
        if !matches!(n.kind, x_native::NodeKind::Rect { .. }) {
            return vec![];
        };
        let Some(b) = self.selection_screen_bounds() else {
            return vec![];
        };
        let inset = 8.0f64.min(b.width() / 4.0).min(b.height() / 4.0).max(4.0);
        let cs = [(b.x0, b.y0), (b.x1, b.y0), (b.x1, b.y1), (b.x0, b.y1)];
        cs.iter()
            .enumerate()
            .map(|(i, (cx, cy))| {
                let px = cx + if i % 3 == 0 { inset } else { -inset };
                let py = cy + if i < 2 { inset } else { -inset };
                (i as u8, Point::new(px, py))
            })
            .collect()
    }

    /// Hit-test the corner-radius handles. Returns the corner index 0..3 when
    /// the pointer is on one.
    pub fn radius_handle_at(&self, p: Point) -> Option<u8> {
        self.radius_handles().iter().find_map(|(c, pt)| {
            if (p.x - pt.x).abs() <= 6.0 && (p.y - pt.y).abs() <= 6.0 {
                Some(*c)
            } else {
                None
            }
        })
    }

    /// Figma rotation: no visible knob — an invisible hotspot in the ring    /// JUST OUTSIDE each corner (8..24px out, beyond the resize square).
    pub fn rotate_handle_at(&self, p: Point) -> bool {
        let Some(b) = self.selection_screen_bounds() else {
            return false;
        };
        let outside = p.x < b.x0 - 4.0 || p.x > b.x1 + 4.0 || p.y < b.y0 - 4.0 || p.y > b.y1 + 4.0;
        if !outside {
            return false;
        }
        for (cx, cy) in [(b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)] {
            let d = ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt();
            if d > 6.0 && d <= 24.0 {
                return true;
            }
        }
        false
    }

    // ---------------------------------------------------------- text focus

    pub fn commit_focus(&mut self) {
        match std::mem::replace(&mut self.focus, Focus::None) {
            Focus::TextNode {
                id,
                buffer,
                original,
                ..
            } => {
                if buffer.trim().is_empty() {
                    // Figma discards an empty text box on blur instead of
                    // leaving a stray empty layer behind.
                    self.editor.selection = vec![id.clone()];
                    self.editor.delete_selection();
                    self.status = "empty text discarded".into();
                } else if buffer != original {
                    self.editor.set_text(&id, &buffer);
                    self.status = format!("text: {buffer}");
                }
            }
            Focus::Field { id, field, buffer } => {
                if buffer.trim().is_empty() {
                    // select-all semantics: nothing typed -> keep old value
                    let _ = (&id, field);
                } else if let Ok(v) = buffer.trim().parse::<f64>() {
                    match field {
                        0 | 1 => {
                            if let Some(n) = find(&self.editor.root, &id) {
                                let (dx, dy) = if field == 0 {
                                    (v - n.transform.x, 0.0)
                                } else {
                                    (0.0, v - n.transform.y)
                                };
                                let keep = self.editor.selection.clone();
                                self.editor.selection = vec![id.clone()];
                                self.editor.move_selection(dx, dy);
                                self.editor.selection = keep;
                            }
                        }
                        2 | 3 => {
                            if let Some(n) = find(&self.editor.root, &id) {
                                let (w, h) = if field == 2 { (v, n.h) } else { (n.w, v) };
                                self.editor.resize(&id, w.max(1.0), h.max(1.0));
                            }
                        }
                        _ => {}
                    }
                    self.status = format!("set {} = {v}", ["X", "Y", "W", "H"][field as usize]);
                }
            }
            Focus::LayerSearch => {}
            Focus::LayerRename { id, buffer } => {
                let name = buffer.trim();
                self.status = if self.editor.rename_node(&id, name) {
                    format!("renamed → {name}")
                } else {
                    "rename refused: empty name".into()
                };
            }
            Focus::FontSearch => {}
            Focus::StyleSearch => {}
            Focus::LibSearch => {}
            Focus::StyleRename { from, buffer } => {
                self.commit_style_rename(&from, buffer.trim());
            }
            Focus::AssetSearch => {}
            Focus::PageRename { idx, buffer } => {
                self.commit_page_rename(idx, &buffer);
            }
            Focus::DashSearch => {}
            Focus::DashRename { path, buffer } => {
                self.commit_dash_rename(&path, &buffer);
            }
            Focus::AssetRename { id, buffer } => {
                if buffer.trim().is_empty() {
                    self.status = "rename cancelled (kept old name)".into();
                } else if self.store.rename(&id, &buffer) {
                    self.status = format!("asset renamed -> {}", buffer.trim());
                } else {
                    self.status = "rename refused".into();
                }
            }
            Focus::Prop {
                instance_id,
                prop_name,
                buffer,
            } => {
                let value = buffer.trim();
                if value.is_empty() {
                    self.status = format!("{prop_name}: cancelled (kept old value)");
                } else if self.editor.set_prop_value(&instance_id, &prop_name, value) {
                    self.status = format!("{prop_name} = {value}");
                } else {
                    self.status = format!("{prop_name}: invalid value");
                }
            }
            Focus::VariantProp {
                component,
                prop_name,
                buffer,
            } => {
                let value = buffer.trim();
                if value.is_empty() {
                    self.status = format!("{prop_name}: cancelled (kept old value)");
                } else if self.editor.set_prop_default(&component, &prop_name, value) {
                    self.status = format!("{component} · {prop_name} = {value}");
                } else {
                    self.status = format!("{prop_name}: invalid value");
                }
            }
            Focus::Proto {
                node_id,
                index,
                field,
                buffer,
            } => self.commit_proto_edit(&node_id, index, field, &buffer),
            Focus::PresentVar { name, buffer } => {
                let text = buffer.trim();
                if self.vars.numbers.contains_key(&name) {
                    if text.is_empty() {
                        self.status = format!("{name}: cancelled (kept old value)");
                    } else if let Ok(e) = parse_expr_text(text) {
                        match eval_expr(&e, &self.vars) {
                            Value::Num(n) if n.is_finite() => {
                                self.vars.numbers.insert(name.clone(), n);
                                self.status = format!("{name} = {n}");
                            }
                            Value::Str(sv) => match sv.parse::<f64>() {
                                Ok(n) => {
                                    self.vars.numbers.insert(name.clone(), n);
                                    self.status = format!("{name} = {n}");
                                }
                                Err(_) => self.status = format!("{name}: needs a number"),
                            },
                            _ => self.status = format!("{name}: needs a number"),
                        }
                    } else {
                        self.status = format!("{name}: invalid expression");
                    }
                } else if self.vars.strings.contains_key(&name) {
                    if text.is_empty() {
                        self.status = format!("{name}: cancelled (kept old value)");
                    } else {
                        self.vars.strings.insert(name.clone(), text.to_string());
                        self.status = format!("{name} = {text}");
                    }
                } else {
                    self.status = format!("{name}: no longer an input variable");
                }
            }
            Focus::CodeRef { node_id, buffer } => {
                let link = buffer.trim();
                if link.is_empty() {
                    if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &node_id) {
                        n.bindings.remove("code");
                    }
                    self.status = format!("{node_id}: code link cleared");
                } else if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &node_id)
                {
                    n.bindings.insert("code".into(), link.to_string());
                    self.status = format!("{node_id} -> {link}");
                }
            }
            Focus::None => {}
        }
    }

    /// Tab in a numeric field: commit, then focus the next field (X->Y->W->H).
    pub fn focus_next_field(&mut self) {
        if let Focus::Field { id, field, .. } = self.focus.clone() {
            self.commit_focus();
            let next = (field + 1) % 4;
            self.focus = Focus::Field {
                id,
                field: next,
                buffer: String::new(),
            };
            self.status = format!(
                "type new {} (Tab cycles)",
                ["X", "Y", "W", "H"][next as usize]
            );
        }
    }

    pub fn cancel_focus(&mut self) {
        if let Focus::TextNode { id, original, .. } = &self.focus {
            let id = id.clone();
            let orig = original.clone();
            if orig.trim().is_empty() {
                // freshly created, never had real content — Esc discards it
                // instead of leaving an empty layer, same as commit does.
                self.editor.selection = vec![id];
                self.editor.delete_selection();
            } else if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                // restore original content directly (no undo entry for the cancel)
                if let x_native::NodeKind::Text { text } = &mut n.kind {
                    *text = orig;
                    n.text_runs.clear();
                }
            }
        }
        self.focus = Focus::None;
    }

    /// Refresh a text node's stored baseline offset from real font metrics so
    /// auto-layout baseline alignment lines up with the rendered glyphs.
    pub fn refresh_text_baseline(&mut self, id: &str) {
        let b = if let Some(n) = find(&self.editor.root, id) {
            if matches!(n.kind, x_native::NodeKind::Text { .. }) {
                let lh = n
                    .bindings
                    .get("lh")
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(1.2);
                let font = n.bindings.get("font").map(|s| s.as_str());
                x_native::text::node_text_baseline(&self.fonts, n.h, font, lh)
            } else {
                None
            }
        } else {
            None
        };
        if let (Some(b), Some(n)) = (b, x_native::editor::find_mut(&mut self.editor.root, id)) {
            n.baseline = Some(b);
        }
    }

    // ---------------------------------------------------------------- pages

    pub fn switch_page(&mut self, idx: usize) {
        if idx >= self.pages.len() || idx == self.page_idx {
            return;
        }
        self.commit_focus();
        self.pages[self.page_idx] = self.editor.root.clone();
        self.page_idx = idx;
        self.editor = Editor::new(self.pages[idx].clone());
        self.status = format!("page: {}", self.pages[idx].name);
    }

    // ------------------------------------------- dashboard <-> editor
    // standard lifecycle: Home (recent files) -> open file -> editor ->
    // back to Home (auto-saves; card thumbnail + modified time update).

    /// Scan persistent documents: ./document.x plus ./files/*.x
    pub fn scan_dash_files(&mut self) {
        let mut out: Vec<DashFile> = vec![];
        let mut paths = vec!["document.x".to_string()];
        // ./files first, then the per-user fallback (see new_file) so
        // documents created outside a writable CWD still show as cards
        let mut scan_dirs: Vec<std::path::PathBuf> = vec!["files".into()];
        if let Some(home) = user_files_dir() {
            scan_dirs.push(home);
        }
        for dir in &scan_dirs {
            if let Ok(rd) = std::fs::read_dir(dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.extension().is_some_and(|x| x == "x") {
                        let ps = p.to_string_lossy().to_string();
                        if !paths.contains(&ps) {
                            paths.push(ps);
                        }
                    }
                }
            }
        }
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let (d2, _) = x_native::fileio::load_x_lenient(&text);
            if d2.doc.pages.is_empty() {
                continue;
            }
            let name = if d2.metadata.name.is_empty() || d2.metadata.name == "X Native document" {
                if path == "document.x" {
                    "Brand Dashboard".to_string()
                } else {
                    std::path::Path::new(&path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                }
            } else {
                d2.metadata.name.clone()
            };
            let modified = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .map(|e| {
                    let s = e.as_secs();
                    if s < 60 {
                        "just now".to_string()
                    } else if s < 3600 {
                        format!("{} min ago", s / 60)
                    } else if s < 86400 {
                        format!("{} hr ago", s / 3600)
                    } else {
                        format!("{} day(s) ago", s / 86400)
                    }
                })
                .unwrap_or_default();
            // real IR thumbnail of page 1
            let pg = &d2.doc.pages[0];
            let tree = x_native::build_render_tree(pg, &d2.doc.variables);
            let (thumb, _) =
                x_native::thumbnail_scene(&tree, pg.w.max(1.0), pg.h.max(1.0), 216.0, 130.0);
            out.push(DashFile {
                path,
                name,
                modified,
                pages: d2.doc.pages.len(),
                thumb: Some(thumb),
            });
        }
        self.dash_files = out;
    }

    /// Open a document from the dashboard into the editor.
    pub fn open_file(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else {
            self.status = format!("can't read {path}");
            return;
        };
        let (d2, notes) = x_native::fileio::load_x_lenient(&text);
        if d2.doc.pages.is_empty() {
            self.status = "file has no pages".into();
            return;
        }
        self.doc_path = path.to_string();
        self.pages = d2.doc.pages;
        self.page_idx = 0;
        self.editor = Editor::new(self.pages[0].clone());
        self.vars = d2.doc.variables;
        self.styles = d2.doc.styles;
        self.store = d2.doc.assets;
        self.library_deps = d2.doc.library_deps.clone();
        self.library_snapshots = d2.doc.library_snapshots.clone();
        let _ = self.assets.sync_store(&self.store);
        x_native::resolve_styles(&mut self.editor.root, &self.styles);
        self.rebuild_layer_rows();
        self.dirty_since_save = false;
        self.saved_undo_depth = self.editor.undo_depth();
        self.screen = Screen::Editor;
        self.scene_cache = x_native::FrameCache::new();
        self.status = if notes.is_empty() {
            format!("opened {path}")
        } else {
            format!("opened {path} ({} recovery note(s))", notes.len())
        };
        x_native::fileio::push_recent(path);
    }

    /// Create a fresh document and open it. Writes under ./files first
    /// (the cargo-run / repo workflow); when the working directory is NOT
    /// writable — a double-clicked binary lands in `/` or a read-only
    /// install dir — falls back to ~/x-native-files so a new file ALWAYS
    /// lands somewhere real. The status names the exact path, or the
    /// exact error; nothing fails silently anymore.
    pub fn new_file(&mut self) {
        let mut d = Document::new();
        d.pages.push(Node::frame("page-1", 1600.0, 1000.0));
        let mut dirs: Vec<std::path::PathBuf> = vec!["files".into()];
        if let Some(home) = user_files_dir() {
            dirs.push(home);
        }
        let mut last_err = "no writable location".to_string();
        for dir in &dirs {
            if let Err(e) = std::fs::create_dir_all(dir) {
                last_err = format!("{}: {e}", dir.display());
                continue;
            }
            let mut n = 1;
            let path = loop {
                let p = dir.join(format!("untitled-{n}.x"));
                if !p.exists() {
                    break p;
                }
                n += 1;
            };
            let mut d2 = x_native::fileio::DocumentV2::default();
            d2.metadata.name = format!("Untitled {n}");
            d2.doc = d.clone();
            let written = x_native::fileio::atomic_write(
                &path.to_string_lossy(),
                x_native::fileio::save_x_v2(&d2).as_bytes(),
            );
            match written {
                Ok(()) => {
                    self.status = format!("created {}", path.display());
                    self.open_file(&path.to_string_lossy());
                    return;
                }
                Err(e) => last_err = format!("{}: {e}", path.display()),
            }
        }
        self.status = format!("new file failed — {last_err}");
    }

    /// Adjust one corner radius (0=TL 1=TR 2=BR 3=BL) or all (idx None).
    /// Promotes uniform radius -> corner_radii[4] on first per-corner edit.
    /// Undoable (routes through `Editor::set_corners`).
    pub fn adjust_corner(&mut self, idx: Option<usize>, delta: f64) {
        let Some(id) = self.editor.selection.first().cloned() else {
            return;
        };
        // Compute the target values first (immutable borrow), then mutate.
        let target: Option<(f64, Option<[f64; 4]>)> =
            x_native::editor::find(&self.editor.root, &id).and_then(|n| {
                if let x_native::NodeKind::Rect { radius } = &n.kind {
                    match idx {
                        None => Some(((*radius + delta).max(0.0), None)),
                        Some(k) => {
                            let c = n.corner_radii.unwrap_or([*radius; 4]);
                            let mut c2 = c;
                            c2[k] = (c2[k] + delta).max(0.0);
                            Some((*radius, Some(c2)))
                        }
                    }
                } else {
                    None
                }
            });
        if let Some((r, corners)) = target {
            self.editor.set_corners(&id, r, corners);
            match corners {
                Some(c2) => {
                    self.status = format!(
                        "corners {:.0}/{:.0}/{:.0}/{:.0}",
                        c2[0], c2[1], c2[2], c2[3]
                    )
                }
                None => self.status = format!("radius {:.0}", r),
            }
        }
    }

    /// Move the current page left/right in the page order (reorder).
    pub fn reorder_page(&mut self, dir: i32) {
        let i = self.page_idx;
        let j = if dir < 0 {
            i.checked_sub(1)
        } else {
            if i + 1 < self.pages.len() {
                Some(i + 1)
            } else {
                None
            }
        };
        let Some(j) = j else {
            self.status = "page already at the edge".into();
            return;
        };
        self.pages[i] = self.editor.root.clone();
        self.pages.swap(i, j);
        self.page_idx = j;
        self.editor = Editor::new(self.pages[j].clone());
        self.dirty_since_save = true;
        self.status = format!("page moved to position {}", j + 1);
    }

    /// Paste SVG markup from the OS clipboard as editable nodes
    /// (cross-app interop: copy in any tool -> paste here).
    pub fn paste_svg_from_clipboard(&mut self) {
        let Some(text) = crate::os_clipboard_get() else {
            self.status = "OS clipboard empty/unavailable".into();
            return;
        };
        if !text.trim_start().starts_with("<svg") && !text.contains("<svg") {
            self.status = "clipboard has no SVG markup".into();
            return;
        }
        match x_native::fileio::import_svg(&text) {
            Ok(mut root) => {
                // place at the cursor's world point, keep nodes editable
                let wp = if self.canvas_rect().contains(self.cursor) {
                    self.world_point(self.cursor)
                } else {
                    Point::new(60.0, 60.0)
                };
                let tag = format!("svgpaste{}", self.editor.undo_depth());
                fn resuffix(n: &mut Node, tag: &str) {
                    n.id = format!("{}-{}", n.id, tag);
                    n.name = n.id.clone();
                    for c in &mut n.children {
                        resuffix(c, tag);
                    }
                }
                resuffix(&mut root, &tag);
                root.transform.x = wp.x;
                root.transform.y = wp.y;
                let count = {
                    fn c(n: &Node) -> usize {
                        1 + n.children.iter().map(c).sum::<usize>()
                    }
                    c(&root)
                };
                let root_id = self.editor.root.id.clone();
                self.editor.insert_node(&root_id, root);
                self.rebuild_layer_rows();
                self.status = format!("pasted SVG from clipboard ({count} editable node(s))");
            }
            Err(e) => self.status = format!("clipboard SVG parse failed: {e}"),
        }
    }

    /// Copy the selection as SVG markup onto the OS clipboard —
    /// cross-app interop (paste into any tool that accepts SVG).
    pub fn copy_as_svg(&mut self) {
        if self.editor.selection.is_empty() {
            self.status = "nothing selected".into();
            return;
        }
        // wrap the selected subtrees in a temp frame sized to their bounds
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut picked = vec![];
        for id in &self.editor.selection {
            if let Some(n) = find(&self.editor.root, id) {
                min_x = min_x.min(n.transform.x);
                min_y = min_y.min(n.transform.y);
                max_x = max_x.max(n.transform.x + n.w);
                max_y = max_y.max(n.transform.y + n.h);
                picked.push(n.clone());
            }
        }
        if picked.is_empty() {
            return;
        }
        let mut frame = Node::frame("clip", (max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
        for mut n in picked {
            n.transform.x -= min_x;
            n.transform.y -= min_y;
            frame = frame.child(n);
        }
        let outliner = x_native::svg_text_outliner(&self.fonts);
        let svg = x_native::fileio::export_svg_full(&frame, &self.vars, None, Some(&outliner));
        crate::os_clipboard_set(&svg);
        self.status = format!(
            "copied {} object(s) as SVG to OS clipboard",
            self.editor.selection.len()
        );
    }

    /// Rename a file's display name (metadata; the .x path is stable).
    pub fn commit_dash_rename(&mut self, path: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            self.status = "rename cancelled".into();
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            self.status = "file unreadable".into();
            return;
        };
        let (mut d2, _) = x_native::fileio::load_x_lenient(&text);
        d2.metadata.name = name.to_string();
        let out = x_native::fileio::save_x_v2(&d2);
        let _ = x_native::fileio::atomic_write(path, out.as_bytes());
        self.scan_dash_files();
        self.status = format!("renamed to {name}");
    }

    pub fn duplicate_dash_file(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let (mut d2, _) = x_native::fileio::load_x_lenient(&text);
        d2.metadata.name = format!("{} copy", d2.metadata.name);
        let _ = std::fs::create_dir_all("files");
        let mut n = 1;
        let new_path = loop {
            let p = format!("files/copy-{n}.x");
            if !std::path::Path::new(&p).exists() {
                break p;
            }
            n += 1;
        };
        let _ =
            x_native::fileio::atomic_write(&new_path, x_native::fileio::save_x_v2(&d2).as_bytes());
        self.scan_dash_files();
        self.status = format!("duplicated -> {new_path}");
    }

    pub fn delete_dash_file(&mut self, path: &str) {
        if path == "document.x" {
            self.status = "Brand Dashboard is protected — duplicate it instead".into();
            return;
        }
        let _ = std::fs::remove_file(path);
        self.scan_dash_files();
        self.status = format!("deleted {path}");
    }

    /// Editor -> Dashboard: persist the document, refresh the cards.
    pub fn back_to_dashboard(&mut self) {
        self.commit_focus();
        self.save_document();
        self.screen = Screen::Dashboard;
        self.scan_dash_files();
        self.status = "saved — dashboard".into();
    }

    /// double-click a page row/cell -> inline rename (standard behavior)
    pub fn start_page_rename(&mut self, idx: usize) {
        if idx >= self.pages.len() {
            return;
        }
        self.focus = Focus::PageRename {
            idx,
            buffer: String::new(),
        };
        self.status = format!(
            "rename page '{}' — type new name, Enter commits, Esc cancels",
            self.pages[idx].name
        );
    }

    pub fn commit_page_rename(&mut self, idx: usize, name: &str) {
        let name = name.trim();
        if name.is_empty() || idx >= self.pages.len() {
            self.status = "rename cancelled".into();
            return;
        }
        let old = self.pages[idx].name.clone();
        self.pages[idx].name = name.to_string();
        if idx == self.page_idx {
            self.editor.root.name = name.to_string();
        }
        self.dirty_since_save = true;
        self.status = format!("page renamed: {old} -> {name}");
    }

    pub fn delete_page(&mut self, idx: usize) {
        if self.pages.len() <= 1 {
            self.status = "can't delete the last page".into();
            return;
        }
        if idx >= self.pages.len() {
            return;
        }
        let name = self.pages[idx].id.clone();
        self.pages.remove(idx);
        if self.page_idx >= self.pages.len() {
            self.page_idx = self.pages.len() - 1;
        }
        if idx <= self.page_idx || self.page_idx >= self.pages.len() {
            self.page_idx = self.page_idx.min(self.pages.len() - 1);
        }
        self.editor = Editor::new(self.pages[self.page_idx].clone());
        self.dirty_since_save = true;
        self.status = format!("deleted page {name}");
    }

    pub fn duplicate_page(&mut self) {
        self.pages[self.page_idx] = self.editor.root.clone();
        let mut copy = self.pages[self.page_idx].clone();
        copy.id = format!("{} copy", copy.id);
        copy.name = format!("{} copy", copy.name);
        // ids inside must be unique — suffix every node id
        fn resuffix(n: &mut Node, tag: &str) {
            n.id = format!("{}-{}", n.id, tag);
            n.name = n.id.clone();
            for c in &mut n.children {
                resuffix(c, tag);
            }
        }
        let tag = format!("d{}", self.pages.len());
        for c in &mut copy.children {
            resuffix(c, &tag);
        }
        self.pages.insert(self.page_idx + 1, copy);
        self.switch_page(self.page_idx + 1);
        self.dirty_since_save = true;
        self.status = "page duplicated".into();
    }

    // --------------------------------------------- clipboard (standard keys)
    pub fn clipboard_copy(&mut self) {
        let n = self.editor.selection.len();
        if n == 0 {
            self.status = "nothing selected to copy".into();
            return;
        }
        self.editor.copy();
        self.status = format!("copied {n} object(s)");
    }

    pub fn clipboard_cut(&mut self) {
        let n = self.editor.selection.len();
        if n == 0 {
            self.status = "nothing selected to cut".into();
            return;
        }
        self.editor.cut();
        self.status = format!("cut {n} object(s)");
    }

    /// Frame under the cursor (page root when hovering empty canvas) —
    /// Figma's paste target.
    fn paste_target_parent(&self) -> String {
        let mut parent = self.editor.root.id.clone();
        if self.canvas_rect().contains(self.cursor) {
            let wp = self.world_point(self.cursor);
            for child in &self.editor.root.children {
                if !matches!(child.kind, x_native::NodeKind::Frame { .. }) {
                    continue;
                }
                let r = Rect::new(
                    child.transform.x,
                    child.transform.y,
                    child.transform.x + child.w,
                    child.transform.y + child.h,
                );
                if r.contains(wp) {
                    parent = child.id.clone();
                }
            }
        }
        parent
    }

    pub fn clipboard_paste(&mut self) {
        if self.editor.clipboard_len() == 0 {
            self.status = "clipboard empty".into();
            return;
        }
        // Multi-paste (Sketch 2026.2): several containers selected ->
        // paste one copy into EACH, in a single undoable step.
        let containers: Vec<String> = self
            .editor
            .selection
            .iter()
            .filter(|id| {
                find(&self.editor.root, id).is_some_and(|n| {
                    matches!(
                        n.kind,
                        x_native::NodeKind::Frame { .. }
                            | x_native::NodeKind::Group
                            | x_native::NodeKind::Section
                    )
                })
            })
            .cloned()
            .collect();
        if containers.len() > 1 {
            let per = self.editor.clipboard_len();
            let targets: Vec<(String, (f64, f64))> =
                containers.iter().map(|c| (c.clone(), (0.0, 0.0))).collect();
            let ids = self.editor.paste_into_each(&targets);
            self.editor.selection = ids.clone();
            self.status = format!(
                "multi-pasted {per} object(s) into {} container(s)",
                containers.len()
            );
            return;
        }
        // Figma: standard Paste is paste IN PLACE (same coordinates as the
        // original) into the frame under the cursor, else the page root.
        let parent = self.paste_target_parent();
        let into_frame = parent != self.editor.root.id;
        let ids = self.editor.paste_in_place(&parent);
        self.editor.selection = ids.clone();
        self.status = if into_frame {
            format!("pasted {} object(s) into {parent}", ids.len())
        } else {
            format!("pasted {} object(s)", ids.len())
        };
    }

    /// Figma Cmd+Shift+V: paste ON TOP of the selection, matching the selected
    /// object's x/y, inserted above it in the layer order. Nothing selected ->
    /// behaves like regular paste (in place).
    pub fn clipboard_paste_over_selection(&mut self) {
        if self.editor.clipboard_len() == 0 {
            self.status = "clipboard empty".into();
            return;
        }
        if self.editor.selection.is_empty() {
            self.clipboard_paste();
            return;
        }
        let (ox, oy) = self.editor.clipboard_origin().unwrap_or((0.0, 0.0));
        // paste a sibling copy on top of EVERY selected layer (Sketch's
        // multi-paste "Paste Over"), all in one undoable step
        let targets: Vec<(String, (f64, f64))> = self
            .editor
            .selection
            .iter()
            .map(|sel| {
                let (sx, sy) = find(&self.editor.root, sel)
                    .map(|n| (n.transform.x, n.transform.y))
                    .unwrap_or((0.0, 0.0));
                let parent = parent_id(&self.editor.root, sel)
                    .unwrap_or_else(|| self.editor.root.id.clone());
                (parent, (sx - ox, sy - oy))
            })
            .collect();
        let n_sel = targets.len();
        let ids = self.editor.paste_into_each(&targets);
        self.editor.selection = ids.clone();
        self.status = if n_sel > 1 {
            format!("pasted over {n_sel} layers")
        } else {
            format!("pasted {} object(s) over selection", ids.len())
        };
    }

    /// True when there are unsaved edits (dirty flag OR undo depth moved past
    /// the last save point) — used by the close/save lifecycle.
    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty_since_save || self.editor.undo_depth() != self.saved_undo_depth
    }

    /// Figma Cmd+Alt+Shift+V: paste TO REPLACE — remove the selection and drop
    /// the clipboard contents in its place.
    pub fn clipboard_paste_to_replace(&mut self) {
        if self.editor.clipboard_len() == 0 {
            self.status = "clipboard empty".into();
            return;
        }
        if self.editor.selection.is_empty() {
            self.clipboard_paste();
            return;
        }
        // capture each selected layer's parent + position BEFORE deleting,
        // then drop a clipboard copy in each slot (Sketch's multi-paste
        // "Paste and Replace" across the whole selection)
        let (ox, oy) = self.editor.clipboard_origin().unwrap_or((0.0, 0.0));
        let slots: Vec<(String, (f64, f64))> = self
            .editor
            .selection
            .iter()
            .map(|sel| {
                let (sx, sy) = find(&self.editor.root, sel)
                    .map(|n| (n.transform.x, n.transform.y))
                    .unwrap_or((0.0, 0.0));
                let parent = parent_id(&self.editor.root, sel)
                    .unwrap_or_else(|| self.editor.root.id.clone());
                (parent, (sx - ox, sy - oy))
            })
            .collect();
        let n_sel = slots.len();
        let ids = self.editor.paste_over_each(&slots);
        self.editor.selection = ids.clone();
        self.status = if n_sel > 1 {
            format!("replaced {n_sel} layers with {} object(s)", ids.len())
        } else {
            format!("replaced selection with {} object(s)", ids.len())
        };
    }

    // ---------------------------------------------------- close/save lifecycle
    /// macOS-standard close guard: unsaved edits raise a Save / Don't Save /
    /// Cancel sheet before the window closes.
    pub fn confirm_close(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_close = true;
        } else {
            self.exit_requested = true;
        }
    }

    pub fn close_dialog_rect(&self) -> Rect {
        let w = 420.0;
        let h = 150.0;
        Rect::new(
            (self.win_w - w) / 2.0,
            (self.win_h - h) / 2.0,
            (self.win_w + w) / 2.0,
            (self.win_h + h) / 2.0,
        )
    }

    /// Close-dialog buttons: (tag, rect) — 0 = Don't Save, 1 = Cancel, 2 = Save.
    pub fn close_dialog_buttons(&self) -> [(u8, Rect); 3] {
        let p = self.close_dialog_rect();
        let bw = 108.0;
        let bh = 26.0;
        let y = p.y1 - 40.0;
        // macOS order: [Don't Save] [Cancel] [Save (default, rightmost)]
        [
            (
                0,
                Rect::new(p.x1 - 3.0 * bw - 24.0, y, p.x1 - 2.0 * bw - 24.0, y + bh),
            ),
            (
                1,
                Rect::new(p.x1 - 2.0 * bw - 16.0, y, p.x1 - 1.0 * bw - 16.0, y + bh),
            ),
            (2, Rect::new(p.x1 - 1.0 * bw - 8.0, y, p.x1 - 8.0, y + bh)),
        ]
    }

    // ---------------------------------------------------- prototype sharing
    /// Stable share token for the current document (deterministic from its
    /// name, so the link survives save/reopen).
    pub fn share_token(&self) -> String {
        let key = std::path::Path::new(&self.doc_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        x_native::fileio::v2::fnv1a128(&key)[..10].to_string()
    }

    /// The live prototype share link (Figma-style `.../proto/<token>`).
    pub fn share_link(&self) -> String {
        format!("https://share.xnative.app/proto/{}", self.share_token())
    }

    pub fn share_dialog_rect(&self) -> Rect {
        let w = 460.0;
        let h = 230.0;
        Rect::new(
            (self.win_w - w) / 2.0,
            (self.win_h - h) / 2.0,
            (self.win_w + w) / 2.0,
            (self.win_h + h) / 2.0,
        )
    }

    /// Share-dialog controls: (tag, rect). 0 = permission toggle, 1 = copy
    /// link (the link box AND the copy button), 2 = Done.
    pub fn share_dialog_controls(&self) -> Vec<(u8, Rect)> {
        let p = self.share_dialog_rect();
        // 0 = permission toggle pill, 1 = link box AND copy button, 2 = Done
        vec![
            (
                0,
                Rect::new(p.x0 + 20.0, p.y0 + 78.0, p.x0 + 200.0, p.y0 + 96.0),
            ),
            (
                1,
                Rect::new(p.x0 + 20.0, p.y0 + 108.0, p.x1 - 20.0, p.y0 + 132.0),
            ),
            (
                1,
                Rect::new(p.x0 + 20.0, p.y0 + 146.0, p.x0 + 120.0, p.y0 + 172.0),
            ),
            (
                2,
                Rect::new(p.x1 - 108.0, p.y0 + 146.0, p.x1 - 20.0, p.y0 + 172.0),
            ),
        ]
    }

    /// Resolve a click on the share dialog. Returns true when it was handled
    /// (modal — swallows everything else).
    pub fn share_dialog_click(&mut self, p: Point) -> bool {
        if !self.share_open {
            return false;
        }
        if !self.share_dialog_rect().contains(p) {
            self.share_open = false;
            return true;
        }
        for (tag, r) in self.share_dialog_controls() {
            if r.contains(p) {
                match tag {
                    0 => {
                        self.share_public = !self.share_public;
                        self.status = if self.share_public {
                            "prototype now viewable by anyone with the link".into()
                        } else {
                            "prototype now private (only you)".into()
                        };
                    }
                    1 => {
                        crate::os_clipboard_set(&self.share_link());
                        self.status = "prototype link copied to clipboard".into();
                    }
                    2 => {
                        self.share_open = false;
                    }
                    _ => {}
                }
                return true;
            }
        }
        true
    }

    /// Resolve a click on the close dialog. Returns true when the click hit
    /// the dialog (modal — swallows everything else).
    pub fn close_dialog_click(&mut self, p: Point) -> bool {
        if !self.pending_close {
            return false;
        }
        if !self.close_dialog_rect().contains(p) {
            return true;
        } // modal: absorb
        for (tag, r) in self.close_dialog_buttons() {
            if r.contains(p) {
                match tag {
                    0 => {
                        self.pending_close = false;
                        self.exit_requested = true;
                    }
                    1 => {
                        self.pending_close = false;
                    }
                    2 => {
                        self.save_document();
                        self.pending_close = false;
                        self.exit_requested = true;
                    }
                    _ => {}
                }
                return true;
            }
        }
        true
    }

    pub fn add_page(&mut self) {
        self.pages[self.page_idx] = self.editor.root.clone();
        let id = format!("page-{}", self.pages.len() + 1);
        self.pages.push(Node::frame(&id, 1600.0, 1000.0));
        let idx = self.pages.len() - 1;
        self.page_idx = idx;
        self.editor = Editor::new(self.pages[idx].clone());
        self.status = format!("new page: {id}");
    }

    // ---------------------------------------------------------------- input

    // ---------------------------------------------------------- presenting

    pub fn enter_present(&mut self) {
        self.commit_focus();
        self.pages[self.page_idx] = self.editor.root.clone();
        // start at a flow starting point when one exists, else the current page
        let start = self
            .pages
            .iter()
            .position(|p| p.is_starting_point)
            .unwrap_or(self.page_idx);
        self.present = Some(Present::new(start));
        self.arm_delayed(start);
        self.status = "PRESENTING — Esc exits".into();
    }

    /// Arm every AfterDelay interaction on page `idx`, scheduled from now.
    fn arm_delayed(&mut self, idx: usize) {
        let now = std::time::Instant::now();
        let mut events = vec![];
        for (_, ms, i) in x_native::delayed_interactions(&self.pages[idx]) {
            events.push((
                now + std::time::Duration::from_millis(ms as u64),
                idx,
                i.action.clone(),
                i.transition_ms,
                i.animation,
            ));
        }
        if let Some(pr) = &mut self.present {
            pr.delayed = events;
        }
    }

    /// Fire any due AfterDelay events. Called once per present render frame.
    pub fn present_tick(&mut self) {
        let now = std::time::Instant::now();
        let due: Vec<(usize, Action, u32, Animation)> = {
            let Some(pr) = &self.present else { return };
            pr.delayed
                .iter()
                .filter(|(t, ..)| *t <= now)
                .map(|(_, idx, a, ms, anim)| (*idx, a.clone(), *ms, *anim))
                .collect()
        };
        if due.is_empty() {
            return;
        }
        if let Some(pr) = &mut self.present {
            pr.delayed.retain(|(t, ..)| *t > now);
        }
        for (idx, a, ms, anim) in due {
            self.run_present_action(idx, &a, ms, anim);
        }
    }

    /// The page indices reachable as destinations/overlays in `pages`.
    fn page_index_by_id(&self, id: &str) -> Option<usize> {
        self.pages.iter().position(|pg| pg.id == id)
    }

    /// Walk up from `node` (a hit) to the nearest node carrying interactions
    /// matching `trigger`, returning its id + the matching interaction.
    fn interaction_for(page: &Node, hit: &str, trigger: Trigger) -> Option<(String, Interaction)> {
        x_native::find_interaction_for(page, hit, trigger)
    }

    fn run_present_action(&mut self, idx: usize, action: &Action, ms: u32, anim: Animation) {
        match action {
            Action::Navigate { destination } => {
                if let Some(dst) = self.page_index_by_id(destination) {
                    if let Some(pr) = &mut self.present {
                        pr.back_stack.push(idx);
                        pr.overlays.clear();
                        pr.delayed.clear();
                        pr.press = None;
                        pr.dragging = false;
                        pr.transition =
                            Some((idx, dst, std::time::Instant::now(), ms.max(80), anim));
                    }
                }
            }
            Action::OpenOverlay { overlay, position } => {
                if let Some(ov) = self.page_index_by_id(overlay) {
                    if let Some(pr) = &mut self.present {
                        pr.overlays.push((ov, *position));
                    }
                }
            }
            Action::SwapOverlay { overlay } => {
                if let Some(ov) = self.page_index_by_id(overlay) {
                    if let Some(pr) = &mut self.present {
                        let pos = pr
                            .overlays
                            .last()
                            .map(|(_, p)| *p)
                            .unwrap_or(OverlayPosition::Center);
                        if !pr.overlays.is_empty() {
                            *pr.overlays.last_mut().unwrap() = (ov, pos);
                        } else {
                            pr.overlays.push((ov, pos));
                        }
                    }
                }
            }
            Action::CloseOverlay => {
                if let Some(pr) = &mut self.present {
                    pr.overlays.pop();
                }
            }
            Action::ScrollTo { destination } => {
                self.present_scroll_to(destination);
            }
            Action::SetVar { name, value } => {
                let v = x_native::eval_expr(value, &self.vars);
                self.vars.set(name, v.clone());
                self.status = match v {
                    x_native::Value::Num(n) => format!("set {name} = {}", x_native::format_num(n)),
                    x_native::Value::Str(s) => format!("set {name} = \"{s}\""),
                    x_native::Value::Bool(b) => format!("set {name} = {b}"),
                };
            }
            Action::SetMode { mode } => {
                self.vars.set_mode(mode);
                self.status = format!("mode: {mode}");
            }
            Action::Cond { cond, then, els } => {
                let branch = if x_native::condition_holds(cond, &self.vars) {
                    Some(&**then)
                } else {
                    els.as_deref()
                };
                if let Some(a) = branch {
                    self.run_present_action(idx, a, ms, anim);
                }
            }
            Action::Back => {
                if let Some(pr) = &mut self.present {
                    if let Some(prev) = pr.back_stack.pop() {
                        pr.overlays.clear();
                        pr.delayed.clear();
                        pr.press = None;
                        pr.dragging = false;
                        pr.transition =
                            Some((idx, prev, std::time::Instant::now(), ms.max(80), anim));
                    }
                }
            }
        }
    }

    /// Present-mode key dispatch: fire the first `KeyDown` interaction
    /// whose key matches. Returns true when an interaction consumed the key.
    pub fn present_key_down(&mut self, key: &str) -> bool {
        let Some(pr) = &self.present else {
            return false;
        };
        let idx = pr.current;
        let page = self.pages[idx].clone();
        if let Some((node_id, i)) = x_native::find_key_interaction(&page, key) {
            self.status = format!("key {key} -> {node_id}");
            self.run_present_action(idx, &i.action, i.transition_ms, i.animation);
            true
        } else {
            false
        }
    }

    /// Map a screen point into page coordinates for the page at `idx`.
    fn present_world_point(&self, idx: usize, p: Point) -> Point {
        let page = &self.pages[idx];
        let (scale, ox, oy) = self.present_fit(page, self.present_device);
        Point::new((p.x - ox) / scale, (p.y - oy) / scale)
    }

    /// The scrollable frame under a world point on page `idx`, walking
    /// topmost-deepest-first so nested scroll containers resolve correctly.
    fn scrollable_frame_at(&self, idx: usize, wp: Point) -> Option<String> {
        fn walk(n: &Node, wp: Point) -> Option<String> {
            for c in n.children.iter().rev() {
                if !c.visible {
                    continue;
                }
                let inside = wp.x >= c.transform.x
                    && wp.x <= c.transform.x + c.w
                    && wp.y >= c.transform.y
                    && wp.y <= c.transform.y + c.h;
                if inside {
                    if let Some(inner) =
                        walk(c, Point::new(wp.x - c.transform.x, wp.y - c.transform.y))
                    {
                        return Some(inner);
                    }
                    if c.overflow.scrollable() {
                        return Some(c.id.clone());
                    }
                }
            }
            None
        }
        walk(&self.pages[idx], wp)
    }

    /// Wheel scroll in present mode: scroll the scrollable frame under the
    /// cursor (clamped to the frame's content extent).
    pub fn present_scroll(&mut self, dx: f64, dy: f64) {
        let Some(pr) = &self.present else { return };
        if pr.transition.is_some() {
            return;
        }
        let idx = pr.overlays.last().map(|(i, _)| *i).unwrap_or(pr.current);
        let wp = self.present_world_point(idx, self.cursor);
        let Some(fid) = self.scrollable_frame_at(idx, wp) else {
            return;
        };
        // find the frame's content extent to clamp the offset
        let (max_x, max_y) = self.scroll_extent(&self.pages[idx], &fid);
        let cur = self
            .present
            .as_ref()
            .and_then(|p| p.scrolls.get(&fid).copied())
            .unwrap_or((0.0, 0.0));
        let nx = (cur.0 - dx).clamp(0.0, max_x);
        let ny = (cur.1 - dy).clamp(0.0, max_y);
        if let Some(pr) = &mut self.present {
            pr.scrolls.insert(fid, (nx, ny));
        }
    }

    /// The maximum scroll offset for a scrollable frame: content bounds minus
    /// the frame's box, never below zero.
    fn scroll_extent(&self, page: &Node, id: &str) -> (f64, f64) {
        let Some(f) = x_native::editor::find(page, id) else {
            return (0.0, 0.0);
        };
        let mut max_x = 0.0f64;
        let mut max_y = 0.0f64;
        fn walk(n: &Node, ox: f64, oy: f64, mx: &mut f64, my: &mut f64) {
            for c in &n.children {
                let cx = ox + c.transform.x;
                let cy = oy + c.transform.y;
                *mx = mx.max(cx + c.w);
                *my = my.max(cy + c.h);
                walk(c, cx, cy, mx, my);
            }
        }
        walk(f, 0.0, 0.0, &mut max_x, &mut max_y);
        ((max_x - f.w).max(0.0), (max_y - f.h).max(0.0))
    }

    /// Scroll the nearest scrollable ancestor of `destination` so the target
    /// comes into view (Figma's "Scroll to" action).
    fn present_scroll_to(&mut self, destination: &str) {
        let Some(pr) = &self.present else { return };
        let idx = pr.overlays.last().map(|(i, _)| *i).unwrap_or(pr.current);
        // walk to find the target and its nearest scrollable ancestor
        let mut result: Option<(String, f64, f64)> = None;
        fn walk(
            n: &Node,
            dest: &str,
            scroll_ancestor: Option<&Node>,
            out: &mut Option<(String, f64, f64)>,
        ) {
            let ancestor = if n.overflow.scrollable() {
                Some(n)
            } else {
                scroll_ancestor
            };
            for c in &n.children {
                if c.id == dest {
                    if let Some(a) = ancestor {
                        // target's top-left relative to the scrollable ancestor
                        *out = Some((a.id.clone(), c.transform.x, c.transform.y));
                    }
                }
                walk(c, dest, ancestor, out);
            }
        }
        walk(&self.pages[idx], destination, None, &mut result);
        if let Some((fid, tx, ty)) = result {
            if let Some(_f) = x_native::editor::find(&self.pages[idx], &fid) {
                let (max_x, max_y) = self.scroll_extent(&self.pages[idx], &fid);
                let ox = tx.clamp(0.0, max_x);
                let oy = ty.clamp(0.0, max_y);
                if let Some(pr) = &mut self.present {
                    pr.scrolls.insert(fid, (ox, oy));
                }
            }
        }
    }

    pub fn present_click(&mut self, _p: Point) {
        if self.present.is_none() {
            return;
        }
        let current = self.present.as_ref().unwrap().current;
        let idx = self
            .present
            .as_ref()
            .unwrap()
            .overlays
            .last()
            .map(|(i, _)| *i)
            .unwrap_or(current);
        if self.present.as_ref().unwrap().transition.is_some() {
            return;
        }
        // resolve the press (recorded on mouse-down) into click or drag
        let (pressed, dragging) = {
            let pr = self.present.as_ref().unwrap();
            (pr.press.clone(), pr.dragging)
        };
        if let Some(pr) = &mut self.present {
            pr.press = None;
            pr.dragging = false;
        }

        if let Some((hit_id, _)) = pressed {
            let page = &self.pages[idx];
            if dragging {
                if let Some((_, i)) = Self::interaction_for(page, &hit_id, Trigger::OnDrag) {
                    let (action, ms, anim) = (i.action.clone(), i.transition_ms, i.animation);
                    self.run_present_action(idx, &action, ms, anim);
                }
                return;
            }
            if let Some((_, i)) = Self::interaction_for(page, &hit_id, Trigger::OnClick) {
                let (action, ms, anim) = (i.action.clone(), i.transition_ms, i.animation);
                self.run_present_action(idx, &action, ms, anim);
                return;
            }
        }
        // fallback: click-anywhere advances to the next page
        let overlays_empty = self
            .present
            .as_ref()
            .map(|p| p.overlays.is_empty())
            .unwrap_or(true);
        if overlays_empty {
            let next = (current + 1) % self.pages.len().max(1);
            if next != current {
                if let Some(pr) = &mut self.present {
                    pr.transition = Some((
                        current,
                        next,
                        std::time::Instant::now(),
                        350,
                        Animation::SmartAnimate,
                    ));
                }
            }
        }
    }

    /// Mouse press (OnPress trigger + press tracking for click/drag) —
    /// evaluated on the topmost open overlay or the current page.
    /// Input chips for exposed variables along the bottom of present mode
    /// (empty when not presenting or nothing is exposed). Numbers show the
    /// live value; strings likewise; SetVar actions update them mid-play.
    pub fn present_var_rects(&self) -> Vec<(String, Rect)> {
        if self.present.is_none() {
            return vec![];
        }
        let mut out = vec![];
        let mut x = 12.0;
        let y0 = self.win_h - 46.0;
        for name in self.vars.exposed.iter() {
            if !self.vars.numbers.contains_key(name) && !self.vars.strings.contains_key(name) {
                continue;
            }
            let val = match &self.focus {
                Focus::PresentVar { name: n, buffer } if n == name => buffer.clone(),
                _ => {
                    if self.vars.numbers.contains_key(name) {
                        format!("{}", self.vars.number(name, 0.0))
                    } else {
                        self.vars.string(name, "")
                    }
                }
            };
            let text = format!("{name}: {val}");
            let w = measure(&text, 8.5) + 18.0;
            out.push((name.clone(), Rect::new(x, y0, x + w, y0 + 18.0)));
            x += w + 8.0;
            if x > self.win_w - 60.0 {
                break;
            }
        }
        out
    }

    pub fn present_press(&mut self, p: Point) {
        if self
            .present
            .as_ref()
            .is_none_or(|pr| pr.transition.is_some())
        {
            return;
        }
        // exposed-variable input chips capture the press first
        for (name, r) in self.present_var_rects() {
            if r.contains(p) {
                let val = if self.vars.numbers.contains_key(&name) {
                    format!("{}", self.vars.number(&name, 0.0))
                } else {
                    self.vars.string(&name, "")
                };
                self.focus = Focus::PresentVar { name, buffer: val };
                self.status = "edit exposed variable — Enter applies".into();
                return;
            }
        }
        // clicking elsewhere commits an in-progress edit
        if matches!(self.focus, Focus::PresentVar { .. }) {
            self.commit_focus();
        }
        let Some(pr) = &self.present else { return };
        let idx = pr.overlays.last().map(|(i, _)| *i).unwrap_or(pr.current);
        let wp = self.present_world_point(idx, p);
        let hit = x_native::editor::hit_test(&self.pages[idx], wp);
        if let Some(pr) = &mut self.present {
            pr.press = hit.clone().map(|id| (id, wp));
            pr.dragging = false;
        }
        if let Some(hit_id) = hit {
            if let Some((_, i)) = Self::interaction_for(&self.pages[idx], &hit_id, Trigger::OnPress)
            {
                let (action, ms, anim) = (i.action.clone(), i.transition_ms, i.animation);
                self.run_present_action(idx, &action, ms, anim);
            }
        }
    }

    /// Mouse move (hover/enter/leave triggers). Hover actions fire once per
    /// hovered node (not every frame), so "while hovering" behaves like a
    /// state rather than spamming overlays/navigation.
    pub fn present_hover(&mut self, p: Point) {
        let Some(pr) = &self.present else { return };
        if pr.transition.is_some() {
            return;
        }
        let idx = pr.overlays.last().map(|(i, _)| *i).unwrap_or(pr.current);
        let wp = self.present_world_point(idx, p);
        // drag detection: a press that moves beyond the threshold becomes a drag
        let pressed = self.present.as_ref().and_then(|p| p.press.clone());
        if let Some((_, start)) = &pressed {
            if (*start - wp).hypot() > 5.0 {
                if let Some(pr) = &mut self.present {
                    pr.dragging = true;
                }
            }
        }
        if pressed.is_some() {
            return;
        } // no hover while pressing/dragging
        let new_hover = x_native::editor::hit_test(&self.pages[idx], wp);
        let prev = self.present.as_ref().and_then(|p| p.hover.clone());
        if let Some(pr) = &mut self.present {
            pr.hover = new_hover.clone();
        }

        let mut to_run: Option<(Action, u32, Animation)> = None;
        {
            let page = &self.pages[idx];
            if let Some(hit) = &new_hover {
                if prev.as_ref() != Some(hit) {
                    // entered a node: fire MouseEnter (fall back to OnHover)
                    if let Some((_, i)) = Self::interaction_for(page, hit, Trigger::MouseEnter) {
                        to_run = Some((i.action.clone(), i.transition_ms, i.animation));
                    } else if let Some((_, i)) = Self::interaction_for(page, hit, Trigger::OnHover)
                    {
                        to_run = Some((i.action.clone(), i.transition_ms, i.animation));
                    }
                }
            }
            if to_run.is_none() {
                if let Some(prev_id) = prev {
                    if new_hover.as_ref() != Some(&prev_id) {
                        if let Some((_, i)) =
                            Self::interaction_for(page, &prev_id, Trigger::MouseLeave)
                        {
                            to_run = Some((i.action.clone(), i.transition_ms, i.animation));
                        }
                    }
                }
            }
        }
        if let Some((action, ms, anim)) = to_run {
            self.run_present_action(idx, &action, ms, anim);
        }
    }

    /// Recursively apply the present session's scroll offsets to a page clone.
    fn apply_scrolls(node: &mut Node, scrolls: &std::collections::HashMap<String, (f64, f64)>) {
        if node.overflow.scrollable() {
            if let Some(&(x, y)) = scrolls.get(&node.id) {
                node.scroll = (x, y);
            }
        }
        for c in &mut node.children {
            Self::apply_scrolls(c, scrolls);
        }
    }

    /// The frames to draw while presenting, bottom-first: `[0]` is the base
    /// page (possibly mid-transition); subsequent entries are open overlays
    /// with their anchor position.
    pub fn present_frames(&mut self) -> Vec<(Node, Option<OverlayPosition>)> {
        if self.present.is_none() {
            return vec![];
        }
        let scrolls = self.present.as_ref().unwrap().scrolls.clone();
        let mut out = vec![];
        let mut armed = None;
        {
            let pr = self.present.as_mut().unwrap();
            if let Some((from, to, started, ms, anim)) = pr.transition {
                let t = started.elapsed().as_millis() as f64 / ms as f64;
                if t >= 1.0 {
                    pr.current = to;
                    pr.transition = None;
                    armed = Some(to);
                    out.push((self.pages[to].clone(), None));
                } else {
                    let te = if t < 0.5 {
                        2.0 * t * t
                    } else {
                        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                    };
                    match anim {
                        Animation::Instant => out.push((self.pages[to].clone(), None)),
                        Animation::Dissolve => {
                            // dissolve: crossfade handled by opacity in the scene;
                            // emit `to` as the base (the caller fades it in).
                            let mut f = self.pages[to].clone();
                            f.opacity = (f.opacity as f64 * te) as f32;
                            out.push((f, None));
                        }
                        _ => out.push((
                            x_native::editor::smart_animate(&self.pages[from], &self.pages[to], te),
                            None,
                        )),
                    }
                }
            } else {
                out.push((self.pages[pr.current].clone(), None));
            }
            for &(oi, pos) in &pr.overlays {
                out.push((self.pages[oi].clone(), Some(pos)));
            }
        }
        if let Some(to) = armed {
            self.arm_delayed(to);
        }
        // apply session scroll offsets to each rendered frame
        for (frame, _) in &mut out {
            Self::apply_scrolls(frame, &scrolls);
        }
        out
    }

    /// Screen rect (window px) inside the device bezel, plus bezel thickness
    /// and corner radius. None when `device` is None.
    fn present_screen_rect(&self, device: DeviceFrame) -> Option<(Rect, f64, f64)> {
        if device == DeviceFrame::None {
            return None;
        }
        let bezel = device.bezel();
        let aspect = device.aspect();
        let avail_w = self.win_w - 2.0 * bezel;
        let avail_h = self.win_h - 2.0 * bezel;
        let mut sw = avail_w;
        let mut sh = sw / aspect;
        if sh > avail_h {
            sh = avail_h;
            sw = sh * aspect;
        }
        let sx = (self.win_w - sw) / 2.0;
        let sy = (self.win_h - sh) / 2.0;
        Some((Rect::new(sx, sy, sx + sw, sy + sh), bezel, device.corner()))
    }

    /// Fit the base page into the window (or the device screen when set).
    pub fn present_fit(&self, base: &Node, device: DeviceFrame) -> (f64, f64, f64) {
        match self.present_screen_rect(device) {
            Some((screen, _, _)) => {
                let scale =
                    (screen.width() / base.w.max(1.0)).min(screen.height() / base.h.max(1.0));
                let ox = screen.x0 + (screen.width() - base.w * scale) / 2.0;
                let oy = screen.y0 + (screen.height() - base.h * scale) / 2.0;
                (scale, ox, oy)
            }
            None => {
                let scale = (self.win_w / base.w.max(1.0)).min(self.win_h / base.h.max(1.0));
                let ox = (self.win_w - base.w * scale) / 2.0;
                let oy = (self.win_h - base.h * scale) / 2.0;
                (scale, ox, oy)
            }
        }
    }

    /// Draw the device bezel body + screen letterbox (behind the page).
    pub fn draw_device_bezel(&self, ui: &mut Scene, device: DeviceFrame) {
        if let Some((screen, bezel, corner)) = self.present_screen_rect(device) {
            let outer = screen.inflate(bezel, bezel);
            fill_rrect(ui, outer, corner, Color::from_rgb8(0x11, 0x12, 0x16));
            fill_rect(ui, screen, Color::from_rgb8(0x07, 0x08, 0x0c));
        }
    }

    /// Draw device foreground chrome (notch + home indicator for phones).
    pub fn draw_device_foreground(&self, ui: &mut Scene, device: DeviceFrame) {
        if device != DeviceFrame::Phone {
            return;
        }
        if let Some((screen, _, _)) = self.present_screen_rect(device) {
            // notch: dark pill centered at the top edge
            let nw = screen.width() * 0.42;
            let nh = 24.0;
            let notch = Rect::new(
                screen.x0 + (screen.width() - nw) / 2.0,
                screen.y0,
                screen.x0 + (screen.width() + nw) / 2.0,
                screen.y0 + nh,
            );
            fill_rrect(ui, notch, 12.0, Color::from_rgb8(0x11, 0x12, 0x16));
            // home indicator: thin bar near the bottom
            let bw = screen.width() * 0.30;
            let _bh = 4.0;
            let bar = Rect::new(
                screen.x0 + (screen.width() - bw) / 2.0,
                screen.y1 - 12.0,
                screen.x0 + (screen.width() + bw) / 2.0,
                screen.y1 - 8.0,
            );
            fill_rrect(ui, bar, 2.0, Color::from_rgba8(0xff, 0xff, 0xff, 120));
        }
    }

    /// Offset (in base-page units) of an overlay's top-left from the canvas
    /// top-left, per its anchor position. Used by the present renderer.
    pub fn overlay_offset(
        &self,
        base: &Node,
        overlay: &Node,
        position: OverlayPosition,
    ) -> (f64, f64) {
        match position {
            OverlayPosition::TopLeft => (0.0, 0.0),
            OverlayPosition::TopRight => (base.w - overlay.w, 0.0),
            OverlayPosition::BottomLeft => (0.0, base.h - overlay.h),
            OverlayPosition::BottomRight => (base.w - overlay.w, base.h - overlay.h),
            OverlayPosition::Manual(x, y) => (x, y),
            OverlayPosition::Center => ((base.w - overlay.w) / 2.0, (base.h - overlay.h) / 2.0),
        }
    }

    pub fn mouse_down(&mut self, p: Point) {
        let cmd_t0 = std::time::Instant::now();
        self.mouse_down_inner(p);
        self.last_cmd = Some(("click".into(), cmd_t0.elapsed().as_secs_f32() * 1000.0));
    }

    fn mouse_down_inner(&mut self, p: Point) {
        if self.present.is_some() {
            self.present_press(p);
            return;
        }
        let double =
            self.last_click.elapsed().as_millis() < 400 && (p - self.last_click_pos).hypot() < 6.0;
        self.last_click = std::time::Instant::now();
        self.last_click_pos = p;
        self.dbl = double;
        // ---------- unsaved-changes close dialog (modal, swallows all) ----------
        if self.pending_close {
            self.close_dialog_click(p);
            return;
        }
        // ---------- prototype share dialog (modal) ----------
        if self.share_open {
            self.share_dialog_click(p);
            return;
        }
        // ---------- dashboard screen swallows all clicks ----------
        if self.screen == Screen::Dashboard {
            if self.focus != Focus::None {
                self.commit_focus();
            }
            for (tag, r, kind) in self.dash_layout() {
                if !r.contains(p) {
                    continue;
                }
                match kind {
                    1 => self.new_file(),
                    2 => {
                        self.focus = Focus::DashSearch;
                        self.status = "type to filter files".into();
                    }
                    _ => {
                        if double {
                            // double-click name area (below thumb) = rename
                            if p.y > r.y0 + 140.0 {
                                self.focus = Focus::DashRename {
                                    path: tag.clone(),
                                    buffer: String::new(),
                                };
                                self.status = "type the new file name, Enter commits".into();
                                return;
                            }
                        }
                        self.open_file(&tag);
                    }
                }
                return;
            }
            return;
        }

        // an active text/field edit commits when clicking elsewhere
        if self.focus != Focus::None {
            self.commit_focus();
        }

        // help overlay swallows clicks
        if self.help_open {
            self.help_open = false;
            return;
        }
        // asset browser overlay swallows clicks
        if self.asset_browser {
            self.click_asset_browser(p);
            return;
        }
        // import preview overlay: Accept / Cancel
        if self.import_pending.is_some() {
            let panel = Rect::new(
                self.win_w / 2.0 - 260.0,
                self.win_h / 2.0 - 190.0,
                self.win_w / 2.0 + 260.0,
                self.win_h / 2.0 + 190.0,
            );
            let acc = Rect::new(
                panel.x0 + 20.0,
                panel.y1 - 40.0,
                panel.x0 + 110.0,
                panel.y1 - 16.0,
            );
            let can = Rect::new(
                panel.x0 + 120.0,
                panel.y1 - 40.0,
                panel.x0 + 210.0,
                panel.y1 - 16.0,
            );
            if acc.contains(p) {
                if let Some((src, d, report)) = self.import_pending.take() {
                    let count = d.pages.len();
                    for rec in d.assets.iter_sorted() {
                        self.store
                            .register(&rec.name, rec.bytes.clone(), rec.source);
                    }
                    let decoded = self.assets.sync_store(&self.store);
                    self.pages.extend(d.pages);
                    self.status = format!("imported {count} {src} page(s), {decoded} asset(s), {} diagnostic(s) logged", report.diagnostics.len());
                }
                return;
            }
            if can.contains(p) || !panel.contains(p) {
                self.import_pending = None;
                self.status = "import cancelled — nothing changed".into();
            }
            return;
        }
        // library review overlay: Accept / Cancel
        if self.library_review {
            let panel = Rect::new(
                self.win_w / 2.0 - 220.0,
                self.win_h / 2.0 - 160.0,
                self.win_w / 2.0 + 220.0,
                self.win_h / 2.0 + 160.0,
            );
            let acc = Rect::new(
                panel.x0 + 20.0,
                panel.y1 - 40.0,
                panel.x0 + 110.0,
                panel.y1 - 16.0,
            );
            let can = Rect::new(
                panel.x0 + 120.0,
                panel.y1 - 40.0,
                panel.x0 + 210.0,
                panel.y1 - 16.0,
            );
            if acc.contains(p) {
                self.accept_library_update();
                return;
            }
            if can.contains(p) || !panel.contains(p) {
                self.library_review = false;
                self.status = "update kept pending (pinned version unchanged)".into();
            }
            return;
        }
        // chrome first (mockup layout: two-row header, left panel, right
        // inspector, bottom thumbnail strip + status bar)
        if !self.chrome_hidden {
            // X logo (tab strip) = Home: back to the dashboard (auto-saves)
            if p.y < TAB_H && p.x < 110.0 {
                self.back_to_dashboard();
                return;
            }
            // open dropdown menu swallows the click first (real menus)
            if self.menu_open.is_some() {
                for (label_, _, tag, r) in self.menu_layout() {
                    if r.contains(p) {
                        if !self.menu_item_enabled(&tag) {
                            self.status = format!("{} — not applicable now", label_);
                            return; // menu stays open, like real apps
                        }
                        self.menu_open = None;
                        let t0 = std::time::Instant::now();
                        self.run_menu_tag(&tag);
                        self.last_cmd = Some((
                            label_.to_ascii_lowercase(),
                            t0.elapsed().as_secs_f32() * 1000.0,
                        ));
                        return;
                    }
                }
                // click on another menu title switches; elsewhere closes
                for (i, r) in self.menu_title_rects() {
                    if r.contains(p) {
                        self.menu_open = if self.menu_open == Some(i) {
                            None
                        } else {
                            Some(i)
                        };
                        return;
                    }
                }
                self.menu_open = None;
                return;
            }
            // menu titles open their dropdown
            for (i, r) in self.menu_title_rects() {
                if r.contains(p) {
                    self.menu_open = Some(i);
                    self.status = format!("{} menu", MENUS[i].0);
                    return;
                }
            }
            let bar = self.bottom_bar_rect(); // tools centered in header row 2
            if bar.contains(p) {
                self.click_bottom_bar(p);
                return;
            }
            // header row 2: zoom widget + Present button
            if p.y >= TAB_H && p.y < TOP_H {
                let (bm, bl, bp, shr, ppr, pr) = self.header_rects();
                if bm.contains(p) {
                    self.zoom = (self.zoom / 1.25).clamp(0.05, 16.0);
                    self.status = format!("zoom {}%", (self.zoom * 100.0).round());
                    return;
                }
                if bp.contains(p) {
                    self.zoom = (self.zoom * 1.25).clamp(0.05, 16.0);
                    self.status = format!("zoom {}%", (self.zoom * 100.0).round());
                    return;
                }
                if bl.contains(p) {
                    let cw = self.win_w - LAYERS_W - INSPECTOR_W - 40.0;
                    let chh = self.win_h - TOP_H - self.thumbs_h() - STATUS_H - 40.0;
                    self.zoom = (cw / self.editor.root.w.max(1.0))
                        .min(chh / self.editor.root.h.max(1.0))
                        .clamp(0.02, 4.0);
                    self.pan = (20.0, 20.0);
                    self.status = "zoom to fit".into();
                    return;
                }
                // Share ghost + Prototype ghost + Present pill
                if shr.contains(p) {
                    self.share_open = !self.share_open;
                    self.status = if self.share_open {
                        "share dialog open".into()
                    } else {
                        "share closed".into()
                    };
                    return;
                }
                if pr.contains(p) {
                    self.enter_present();
                    return;
                }
                if ppr.contains(p) {
                    self.inspector_tab = 1;
                    self.status = "prototype tab".into();
                    return;
                }
                return;
            }
            // status bar swallows clicks
            if p.y >= self.win_h - STATUS_H {
                return;
            }
        }
        if p.x < LAYERS_W && p.y > TOP_H {
            self.click_left_sidebar(p);
            return;
        }
        if p.x > self.win_w - INSPECTOR_W && p.y > TOP_H {
            self.click_inspector(p);
            return;
        }
        if p.y < TOP_H {
            return;
        }

        // advanced-stroke popover (Figma opens it left over the canvas):
        // clicks on its controls apply; clicks anywhere else close it.
        if self.stroke_advanced_open {
            if let Some((panel, controls)) = self.stroke_advanced_geometry() {
                if panel.contains(p) {
                    if let Some(tag) = controls
                        .iter()
                        .find(|(_, r)| r.contains(p))
                        .map(|(t, _)| *t)
                    {
                        self.apply_stroke_advanced(tag);
                    }
                    return;
                }
            }
            self.stroke_advanced_open = false;
            return;
        }

        // hand tool or held spacebar -> pan drag
        if self.tool == Tool::Hand || self.space_pan {
            self.drag = Drag::Pan { start: p };
            return;
        }
        // scale tool: needs a selection; vertical drag scales it
        if self.tool == Tool::Scale {
            if let Some(id) = self.editor.selection.first() {
                let _ = id;
                self.drag = Drag::Scale {
                    start_y: p.y,
                    applied: 1.0,
                    cmds: self.editor.undo_depth(),
                };
            } else {
                // click selects first, industry-standard
                let wp = self.world_point(p);
                self.editor.click(wp, false);
                if !self.editor.selection.is_empty() {
                    self.status = format!("scale: drag vertically ({})", self.editor.selection[0]);
                }
            }
            return;
        }
        // rulers: click in a ruler strip drops a guide at that spot
        if self.rulers && !self.chrome_hidden {
            let c = self.canvas_rect();
            if p.y >= c.y0 && p.y <= c.y0 + 16.0 && p.x >= c.x0 + 16.0 {
                let wp = self.world_point(p);
                self.user_guides.push((false, wp.y.round()));
                self.status = format!("guide at y={}", wp.y.round());
                return;
            }
            if p.x >= c.x0 && p.x <= c.x0 + 16.0 && p.y >= c.y0 + 16.0 {
                let wp = self.world_point(p);
                self.user_guides.push((true, wp.x.round()));
                self.status = format!("guide at x={}", wp.x.round());
                return;
            }
        }
        // minimap click -> jump viewport there
        if !self.chrome_hidden && self.minimap && self.minimap_rect().contains(p) {
            let mm = self.minimap_rect();
            let page = &self.editor.root;
            let sx = mm.width() / page.w.max(1.0);
            let sy = mm.height() / page.h.max(1.0);
            let s = sx.min(sy);
            let wx = (p.x - mm.x0) / s;
            let wy = (p.y - mm.y0) / s;
            let c = self.canvas_rect();
            self.pan.0 = (c.width() / 2.0) - wx * self.zoom - (c.x0 - self.canvas_origin().0);
            self.pan.1 = (c.height() / 2.0) - wy * self.zoom;
            self.status = "minimap jump".into();
            return;
        }
        // frame name labels are interactive (Figma): single click = select +
        // drag the frame; double-click = inline rename. Only in the Select
        // (move) tool, matching Figma where the title is grabbed with V.
        if !self.chrome_hidden && self.tool == Tool::Select {
            for (id, r) in self.frame_label_rects() {
                if r.contains(p) {
                    if double {
                        let cur = find(&self.editor.root, &id)
                            .map(|n| n.name.clone())
                            .unwrap_or_else(|| id.clone());
                        self.focus = Focus::LayerRename {
                            id: id.clone(),
                            buffer: cur.clone(),
                        };
                        self.status = format!("rename frame: {cur}");
                    } else {
                        self.editor.selection = vec![id.clone()];
                        self.drag = Drag::Move {
                            start: p,
                            cmds: self.editor.undo_depth(),
                        };
                        self.status = format!("selected {id}");
                    }
                    return;
                }
            }
        }
        let wp = self.world_point(p);
        // ---- eyedropper: pick the clicked node's fill, apply to selection ----
        if self.tool == Tool::Eyedropper {
            let hit = x_native::editor::hit_test(&self.editor.root, wp).and_then(|id| {
                x_native::editor::top_level_ancestor(&self.editor.root, &id).or(Some(id))
            });
            let picked = hit.and_then(|src| {
                find(&self.editor.root, &src).and_then(|n| match &n.fill {
                    Paint::Solid(c) => Some((*c, None)),
                    Paint::Variable(v) => Some((self.vars.color(v, Color::BLACK), Some(v.clone()))),
                    _ => None,
                })
            });
            match picked {
                Some((c, var)) => {
                    let hex = format!(
                        "#{:02X}{:02X}{:02X}",
                        c.to_rgba8().r,
                        c.to_rgba8().g,
                        c.to_rgba8().b
                    );
                    let src = x_native::editor::hit_test(&self.editor.root, wp)
                        .and_then(|id| {
                            x_native::editor::top_level_ancestor(&self.editor.root, &id)
                                .or(Some(id))
                        })
                        .unwrap_or_default();
                    let targets: Vec<String> = self
                        .editor
                        .selection
                        .iter()
                        .filter(|id| **id != src)
                        .cloned()
                        .collect();
                    if targets.is_empty() {
                        self.status = format!(
                            "picked {hex}{} — select a layer, then click to apply",
                            var.map(|v| format!(" (var {v})")).unwrap_or_default()
                        );
                    } else {
                        let n = targets.len();
                        for t in &targets {
                            if let Some(paint) = var
                                .as_ref()
                                .map(|v| Paint::Variable(v.clone()))
                                .or(Some(Paint::Solid(c)))
                            {
                                self.editor.set_fill(t, paint);
                            }
                        }
                        self.status = format!(
                            "picked {hex} -> applied to {n} layer(s){}",
                            var.map(|v| format!(" (bound to var {v})"))
                                .unwrap_or_default()
                        );
                    }
                }
                None => self.status = "eyedropper: no fill under the cursor".into(),
            }
            return;
        }
        // ---- paint bucket: fill the clicked node with the active fill ----
        // (the dual of the eyedropper: it picks, this one applies)
        if self.tool == Tool::Bucket {
            if let Some(id) = x_native::editor::hit_test(&self.editor.root, wp) {
                let paint = self
                    .selected_single()
                    .map(|n| n.fill.clone())
                    .unwrap_or(Paint::Solid(C_ACCENT));
                let hex = match &paint {
                    Paint::Solid(c) => format!(
                        "#{:02X}{:02X}{:02X}",
                        c.to_rgba8().r,
                        c.to_rgba8().g,
                        c.to_rgba8().b
                    ),
                    Paint::Variable(v) => {
                        let c = self.vars.color(v, Color::BLACK);
                        format!(
                            "var {v} (#{:02X}{:02X}{:02X})",
                            c.to_rgba8().r,
                            c.to_rgba8().g,
                            c.to_rgba8().b
                        )
                    }
                    _ => "gradient".into(),
                };
                self.editor.set_fill(&id, paint);
                self.status = format!("filled {id} with {hex}");
            } else {
                self.status = "paint bucket: click a layer to fill it".into();
            }
            return;
        }
        // ---- brush: start a variable-width stroke ----
        if self.tool == Tool::Brush {
            self.brush_pts = vec![(wp.x, wp.y)];
            self.brush_w = vec![BRUSH_WMAX];
            self.drag = Drag::Brush;
            return;
        }
        // ---- pencil: start a freehand stroke ----
        if self.tool == Tool::Pencil {
            self.pencil_pts = vec![(wp.x, wp.y)];
            self.drag = Drag::Pencil;
            return;
        }
        // ---- pen tool: click to place anchors; click near start closes ----
        if self.tool == Tool::Pen {
            match &self.pen_target {
                None => {
                    self.created_count += 1;
                    let id = format!("path-{}", self.created_count);
                    let mut v = Node::vector(&id, 0.0, 0.0, 1.0, 1.0, vec![]);
                    v.fill = Paint::Solid(Color::from_rgba8(0x0d, 0x99, 0xff, 120));
                    v.stroke = x_native::Stroke::solid(Color::from_rgb8(0x0d, 0x99, 0xff), 2.0);
                    let root_id = self.editor.root.id.clone();
                    self.editor.insert_node(&root_id, v);
                    self.editor.pen_add_anchor(&id, wp.x, wp.y);
                    self.editor.selection = vec![id.clone()];
                    self.pen_target = Some(id.clone());
                    self.pen_pending_out = None;
                    // standard: keep the button-down window open so a
                    // drag right after this click pulls a curve handle
                    // instead of only ever placing a straight corner.
                    self.pen_placing = Some((0, wp, self.editor.undo_depth()));
                    self.status = "pen: click to add anchors, drag to curve, click start to close, Esc to finish".into();
                }
                Some(id) => {
                    let id = id.clone();
                    // close if clicking near the first anchor
                    let close = if let Some(n) = find(&self.editor.root, &id) {
                        if let x_native::NodeKind::Vector { path } = &n.kind {
                            x_native::editor::anchors(path)
                                .first()
                                .map(|a| {
                                    ((a.x - wp.x).powi(2) + (a.y - wp.y).powi(2)).sqrt()
                                        < 8.0 / self.zoom
                                })
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if close {
                        self.editor.pen_close(&id);
                        self.pen_target = None;
                        self.pen_placing = None;
                        self.pen_pending_out = None;
                        self.node_edit = Some(id);
                        self.tool = Tool::Select;
                        self.status = "pen: closed — node edit active".into();
                    } else {
                        // if the last anchor was dragged, its pulled tangent
                        // becomes this new segment's departure (c1) handle
                        let out_c1 = self.pen_pending_out.take().and_then(|(dx, dy)| {
                            find(&self.editor.root, &id).and_then(|n| {
                                if let x_native::NodeKind::Vector { path } = &n.kind {
                                    x_native::editor::anchors(path)
                                        .last()
                                        .map(|a| (a.x + dx, a.y + dy))
                                } else {
                                    None
                                }
                            })
                        });
                        self.editor.pen_add_anchor_curved(&id, wp.x, wp.y, out_c1);
                        let idx = find(&self.editor.root, &id)
                            .and_then(|n| {
                                if let x_native::NodeKind::Vector { path } = &n.kind {
                                    Some(x_native::editor::anchors(path).len().saturating_sub(1))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0);
                        self.pen_placing = Some((idx, wp, self.editor.undo_depth()));
                    }
                }
            }
            return;
        }
        // ---- node-edit mode: grab a HANDLE or an anchor under the cursor ----
        if let Some(vid) = self.node_edit.clone() {
            if let Some(n) = find(&self.editor.root, &vid) {
                if let x_native::NodeKind::Vector { path } = &n.kind {
                    // anchors are in node-local coords; our pen paths live at 0,0
                    let local = (wp.x - n.transform.x, wp.y - n.transform.y);
                    // vector eraser (Shift+E): start collecting segments;
                    // everything under the cursor by release goes away in
                    // ONE undo step
                    if self.vector_eraser {
                        self.eraser_hits.clear();
                        if let Some(si) =
                            x_native::editor::segment_at(path, local.0, local.1, 8.0 / self.zoom)
                        {
                            self.eraser_hits.push(si);
                        }
                        self.drag = Drag::Erase;
                        self.status = "erasing…".into();
                        return;
                    }
                    // bezier control handles first (smaller targets win)
                    let tol = 6.0 / self.zoom;
                    for (ai, a) in x_native::editor::anchors(path).iter().enumerate() {
                        if let Some((hx, hy)) = a.in_handle {
                            if ((hx - local.0).powi(2) + (hy - local.1).powi(2)).sqrt() <= tol {
                                self.handle_drag = Some((ai, false, self.editor.undo_depth()));
                                self.status = format!("dragging in-handle of anchor {ai}");
                                return;
                            }
                        }
                        if let Some((hx, hy)) = self.editor.out_handle(&vid, ai) {
                            if ((hx - local.0).powi(2) + (hy - local.1).powi(2)).sqrt() <= tol {
                                self.handle_drag = Some((ai, true, self.editor.undo_depth()));
                                self.status = format!("dragging out-handle of anchor {ai}");
                                return;
                            }
                        }
                    }
                    if let Some(ai) =
                        x_native::editor::anchor_at(path, local.0, local.1, 8.0 / self.zoom)
                    {
                        if self.alt {
                            self.editor.delete_anchor(&vid, ai);
                            self.status = format!("anchor {ai} deleted");
                        } else if self.ctrl {
                            self.editor.convert_anchor(&vid, ai);
                            self.status = format!("anchor {ai} converted");
                        } else {
                            self.anchor_drag = Some((ai, self.editor.undo_depth()));
                            self.status = format!("dragging anchor {ai}");
                        }
                        return;
                    }
                }
            }
            // click elsewhere exits node edit
            self.node_edit = None;
        }
        // component stamping takes priority over tools
        if let Some(name) = self.stamping.take() {
            if let Some(id) = self.editor.place_instance(&name, wp.x, wp.y) {
                self.editor.selection = vec![id.clone()];
                self.status = format!("placed {id}");
            }
            return;
        }
        match self.tool {
            Tool::Select => {
                // Gradient geometry and stops are first-class canvas handles.
                // They take precedence over the selection box while the
                // selected fill is in gradient-edit mode.
                if self.gradient_editing {
                    if let Some((fill, handle)) = self.gradient_handle_at(p) {
                        if handle >= 2 {
                            self.gradient_stop = handle - 2;
                            if self.alt {
                                let id = self.editor.selection[0].clone();
                                let stop = handle - 2;
                                self.editor.mutate_visual_stack(&id, |n| {
                                    if let Some(layer) = n.fill_layers.get_mut(fill) {
                                        let stops = match &mut layer.paint {
                                            Paint::LinearGradient { stops, .. }
                                            | Paint::RadialGradient { stops, .. } => stops,
                                            _ => return,
                                        };
                                        if stops.len() > 2 && stop < stops.len() {
                                            stops.remove(stop);
                                        }
                                    }
                                });
                                self.gradient_stop =
                                    self.gradient_stop.min(self.gradient_stop.saturating_sub(1));
                                self.status = "gradient stop removed".into();
                                return;
                            }
                        }
                        self.drag = Drag::Gradient {
                            fill,
                            handle,
                            cmds: self.editor.undo_depth(),
                        };
                        self.status = if handle < 2 {
                            "dragging gradient geometry".into()
                        } else {
                            format!("dragging gradient stop {}", handle - 1)
                        };
                        return;
                    }
                    if double {
                        if let Some((fill, position)) = self.gradient_line_position(p) {
                            let id = self.editor.selection[0].clone();
                            let mut inserted = 0usize;
                            self.editor.mutate_visual_stack(&id, |n| {
                                if let Some(layer) = n.fill_layers.get_mut(fill) {
                                    let stops = match &mut layer.paint {
                                        Paint::LinearGradient { stops, .. }
                                        | Paint::RadialGradient { stops, .. } => stops,
                                        _ => return,
                                    };
                                    let color = gradient_color_at(stops, position);
                                    inserted = stops
                                        .iter()
                                        .position(|(p, _)| *p > position)
                                        .unwrap_or(stops.len());
                                    stops.insert(inserted, (position, color));
                                }
                            });
                            self.gradient_stop = inserted;
                            self.status = format!("gradient stop {} added", inserted + 1);
                            return;
                        }
                    }
                }
                if self.rotate_handle_at(p) {
                    if let Some(n) = self.selected_single() {
                        // rotate about the node's transform-origin pivot
                        // (Figma: the 9-point origin, default center)
                        let pivot = self.transform_pivot_screen(&n.id).unwrap_or_else(|| {
                            let b = self
                                .selection_screen_bounds()
                                .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
                            Point::new((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0)
                        });
                        let a0 = (p.y - pivot.y).atan2(p.x - pivot.x);
                        self.drag = Drag::Rotate {
                            center: pivot,
                            start_angle: a0,
                            orig: n.transform.rotation,
                            cmds: self.editor.undo_depth(),
                        };
                        return;
                    }
                }
                if let Some(corner) = self.handle_at(p) {
                    if let Some(n) = self.selected_single() {
                        self.drag = Drag::Resize {
                            corner,
                            start_world: wp,
                            orig: (n.transform.x, n.transform.y, n.w, n.h),
                            cmds: self.editor.undo_depth(),
                        };
                        return;
                    }
                }
                // corner-radius handles win over nothing else (single Rect)
                if let Some(rh) = self.radius_handle_at(p) {
                    if let Some(n) = self.selected_single() {
                        if let x_native::NodeKind::Rect { radius } = &n.kind {
                            self.drag = Drag::Radius {
                                corner: rh,
                                uniform: n.corner_radii.is_none(),
                                start_world: wp,
                                orig: (*radius, n.corner_radii),
                                cmds: self.editor.undo_depth(),
                            };
                            return;
                        }
                    }
                }
                if double {
                    // drill-in double-click: drill into the hierarchy;
                    // Vector -> node-edit mode; Text -> inline edit.
                    if let Some(next) = self.editor.drill_into(wp) {
                        if let Some(n) = find(&self.editor.root, &next) {
                            if matches!(n.kind, x_native::NodeKind::Vector { .. }) {
                                self.node_edit = Some(next.clone());
                                self.status = "node edit: drag anchors, ctrl+click converts, alt+click deletes, Esc done".into();
                                self.drag = Drag::None;
                                return;
                            }
                            if let x_native::NodeKind::Text { text } = &n.kind {
                                self.focus = Focus::TextNode {
                                    id: n.id.clone(),
                                    buffer: text.clone(),
                                    original: text.clone(),
                                    caret: text.len(),
                                    sel_anchor: None,
                                };
                                self.status = "editing text — Enter commits, Esc cancels".into();
                                self.drag = Drag::None;
                                return;
                            }
                        }
                        self.status = format!("entered {next}");
                        self.drag = Drag::None;
                        return;
                    }
                }
                // standard: plain click = top-level object; Ctrl+click = deep select
                self.editor.click_select(wp, self.shift, self.ctrl);
                if self.editor.selection.is_empty() {
                    self.drag = Drag::Marquee {
                        start_world: wp,
                        contained: self.alt,
                    };
                } else {
                    self.alt_dupe_done = false;
                    self.drag = Drag::Move {
                        start: p,
                        cmds: self.editor.undo_depth(),
                    };
                    self.status = format!("selected {}", self.editor.selection.join(", "));
                }
            }
            _ => self.drag = Drag::Create { start_world: wp },
        }
    }

    /// Rebuild the font browser results from the current query:
    /// system families first, then Google catalog matches.
    pub fn refresh_font_results(&mut self) {
        let q = self.font_query.to_ascii_lowercase();
        let mut out: Vec<(String, FontSource)> = vec![];
        for fam in self.sysfonts.families.keys() {
            if q.is_empty() || fam.to_ascii_lowercase().contains(&q) {
                out.push((
                    fam.clone(),
                    FontSource::System {
                        family: fam.clone(),
                        style: String::new(),
                    },
                ));
            }
            if out.len() >= 40 && q.is_empty() {
                break;
            }
        }
        for f in self.gfonts.search(if q.is_empty() { "a" } else { &q }) {
            if out.iter().any(|(l, _)| l.eq_ignore_ascii_case(&f.family)) {
                continue;
            }
            out.push((
                format!("{} (G)", f.family),
                FontSource::Google {
                    family: f.family.clone(),
                    weight: 400,
                },
            ));
            if out.len() >= 80 {
                break;
            }
        }
        self.font_scroll = 0;
        self.font_results = out;
    }

    /// Bind the selected Text node to a font from the results (loads it
    /// into the FontManager on demand; Google fonts download+cache).
    pub fn apply_font(&mut self, idx: usize) {
        let Some((label, source)) = self.font_results.get(idx).cloned() else {
            return;
        };
        let Some(id) = self.editor.selection.first().cloned() else {
            return;
        };
        let loaded = match &source {
            FontSource::System { family, style } => self
                .sysfonts
                .load_into(&mut self.fonts, family, style)
                .map(|i| self.fonts.fonts[i].name.clone()),
            FontSource::Google { family, weight } => self
                .gfonts
                .load_into(&mut self.fonts, family, *weight)
                .map(|i| self.fonts.fonts[i].name.clone()),
        };
        match loaded {
            Ok(name) => {
                if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                    n.bindings.insert("font".into(), name.clone());
                    n.dirty = true;
                }
                // populate the weight picker for google families
                self.font_weights.clear();
                if let FontSource::Google { family, .. } = &source {
                    if let Some(f) = self.gfonts.family(family) {
                        for w in f.weights() {
                            self.font_weights.push((family.clone(), w, false));
                        }
                        if f.has_italic() {
                            self.font_weights.push((family.clone(), 400, true));
                        }
                    }
                }
                self.status = format!("font: {label}");
            }
            Err(e) => self.status = format!("font failed: {e}"),
        }
    }

    /// Apply a specific weight/italic cut of the current google family.
    pub fn apply_font_weight(&mut self, idx: usize) {
        let Some((family, weight, italic)) = self.font_weights.get(idx).cloned() else {
            return;
        };
        let Some(id) = self.editor.selection.first().cloned() else {
            return;
        };
        match self
            .gfonts
            .load_style_into(&mut self.fonts, &family, weight, italic)
        {
            Ok(i) => {
                let name = self.fonts.fonts[i].name.clone();
                if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                    n.bindings.insert("font".into(), name);
                    n.dirty = true;
                }
                self.status = format!("{family} {weight}{}", if italic { " italic" } else { "" });
            }
            Err(e) => self.status = format!("weight failed: {e}"),
        }
    }

    /// Context-menu action dispatch (indices match the items built in run.rs)
    pub fn run_menu_action(&mut self, i: usize) {
        // context-menu order: CUT COPY PASTE DUPLICATE DELETE FRONT BACK
        // GROUP FRAME-SELECTION UNION SUBTRACT INTERSECT EXCLUDE MASK
        // (run.rs builds items)
        match i {
            0 => self.clipboard_cut(),
            1 => self.clipboard_copy(),
            2 => self.clipboard_paste(),
            3 => {
                self.editor.duplicate_selection((16.0, 16.0));
                self.status = "duplicated".into();
            }
            4 => {
                self.editor.delete_selection();
                self.status = "deleted".into();
            }
            5 => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.bring_to_front(&id);
                    self.status = "to front".into();
                }
            }
            6 => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.send_to_back(&id);
                    self.status = "to back".into();
                }
            }
            7 => {
                let gid = format!("group-{}", self.editor.undo_depth());
                self.editor.group_selection(&gid);
                self.status = "grouped".into();
            }
            8 => {
                let fid = format!("frame-{}", self.editor.undo_depth());
                self.editor.frame_selection(&fid);
                self.status = format!("frame selection -> {fid}");
            }
            9..=12 => {
                use x_native::editor::BoolOp::*;
                let op = [Union, Subtract, Intersect, Exclude][i - 9];
                match self.editor.boolean_selected(op) {
                    Some(id) => self.status = format!("{op:?} -> {id}"),
                    None => self.status = "boolean needs 2 shape nodes".into(),
                }
            }
            13 => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                        n.is_mask = !n.is_mask;
                        self.status = format!("{id} mask: {}", n.is_mask);
                    }
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------- document commands
    // Shared by keyboard shortcuts AND the real dropdown menus — one code
    // path for save/open/import/export, so the menu can never drift from
    // the shortcut behavior.

    pub fn save_document(&mut self) {
        self.pages[self.page_idx] = self.editor.root.clone();
        let mut d = Document::new();
        d.variables = self.vars.clone();
        d.styles = self.styles.clone();
        d.assets = self.store.clone();
        d.library_deps = self.library_deps.clone();
        d.library_snapshots = self.library_snapshots.clone();
        d.pages = self.pages.clone();
        // v2 contract: validate, then save deterministic v2
        let issues = x_native::fileio::validate(&d);
        let mut d2 = x_native::fileio::DocumentV2::default();
        // keep the file's display name stable (dashboard identity)
        d2.metadata.name = self
            .dash_files
            .iter()
            .find(|f| f.path == self.doc_path)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| {
                if self.doc_path == "document.x" {
                    "Brand Dashboard".into()
                } else {
                    std::path::Path::new(&self.doc_path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                }
            });
        d2.metadata.app_version = "0.43".into();
        for p in &d.pages {
            x_native::fileio::v2::backfill_uuids(p, &mut d2.uuids);
        }
        d2.metadata.uuid = x_native::fileio::v2::fnv1a128(&d2.metadata.name);
        d2.doc = d;
        let text = x_native::fileio::save_x_v2(&d2);
        // reliability: history rotation + atomic publish + stale-autosave clear
        x_native::fileio::rotate_backups(&self.doc_path);
        self.status = match x_native::fileio::atomic_write(&self.doc_path, text.as_bytes()) {
            Ok(_) if issues.is_empty() => format!(
                "saved v2 ({} pages, atomic, {} backup(s))",
                d2.doc.pages.len(),
                x_native::fileio::list_backups(&self.doc_path).len()
            ),
            Ok(_) => format!("saved v2 with {} validation issue(s)", issues.len()),
            Err(_) => "save FAILED".into(),
        };
        x_native::fileio::clear_autosave(&self.doc_path);
        self.dirty_since_save = false;
        self.saved_undo_depth = self.editor.undo_depth();
        x_native::fileio::push_recent(&self.doc_path);
    }

    /// Native save panel for first-save / Save As. The normal Save command
    /// remains instant once a document has a path.
    pub fn save_document_as(&mut self) {
        let suggested = std::path::Path::new(&self.doc_path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("Untitled.x");
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save X Designer document")
            .set_file_name(suggested)
            .add_filter("X Designer document", &["x"])
            .save_file()
        else {
            self.status = "save cancelled".into();
            return;
        };
        self.doc_path = path.to_string_lossy().to_string();
        self.save_document();
        self.scan_dash_files();
    }

    pub fn open_document(&mut self) {
        if let Ok(text) = std::fs::read_to_string(&self.doc_path) {
            let (d2, notes) = x_native::fileio::load_x_lenient(&text);
            if !d2.doc.pages.is_empty() {
                self.pages = d2.doc.pages;
                self.page_idx = 0;
                self.editor = Editor::new(self.pages[0].clone());
                self.vars = d2.doc.variables;
                self.styles = d2.doc.styles;
                self.store = d2.doc.assets;
                self.library_deps = d2.doc.library_deps.clone();
                self.library_snapshots = d2.doc.library_snapshots.clone();
                let decoded = self.assets.sync_store(&self.store);
                if decoded > 0 {
                    eprintln!("assets: decoded {decoded} embedded image(s)");
                }
                // style consumers re-sync on open (standard semantics)
                x_native::resolve_styles(&mut self.editor.root, &self.styles);
                self.status = if notes.is_empty() {
                    format!("loaded ({} pages)", self.pages.len())
                } else {
                    format!(
                        "RECOVERED ({} pages, {} note(s))",
                        self.pages.len(),
                        notes.len()
                    )
                };
                // integrity sweep LAST so warnings win the status line
                self.library_integrity.clear();
                let mut dv = Document::new();
                dv.library_deps = self.library_deps.clone();
                dv.library_snapshots = self.library_snapshots.clone();
                for (lid, st) in x_native::fileio::verify_document_libraries(&dv) {
                    if !matches!(st, x_native::fileio::IntegrityStatus::Verified) {
                        self.status = format!("LIBRARY WARNING: {lid} {st:?}");
                    }
                    self.library_integrity.push((lid, format!("{st:?}")));
                }
            }
        } else {
            self.status = "no document.x to open".into();
        }
    }

    pub fn choose_and_open_document(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Open X Designer document")
            .add_filter("X Designer document", &["x"])
            .pick_file()
        else {
            self.status = "open cancelled".into();
            return;
        };
        self.open_file(&path.to_string_lossy());
    }

    /// Choose an external file and stage it in the import-preview overlay.
    /// Nothing lands in the document until the designer accepts the preview.
    pub fn start_import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import into X Designer")
            .add_filter("Supported design files", &["sketch", "json", "svg", "png"])
            .add_filter("Sketch file", &["sketch"])
            .add_filter("Design JSON", &["json"])
            .add_filter("SVG", &["svg"])
            .add_filter("PNG", &["png"])
            .pick_file()
        else {
            self.status = "import cancelled".into();
            return;
        };
        self.stage_import_path(path);
    }

    pub fn start_figma_import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import Figma REST API JSON")
            .add_filter("Figma REST API JSON", &["json"])
            .pick_file()
        else {
            self.status = "import cancelled".into();
            return;
        };
        self.stage_import_path(path);
    }

    pub fn start_sketch_import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import Sketch document")
            .add_filter("Sketch document", &["sketch"])
            .pick_file()
        else {
            self.status = "import cancelled".into();
            return;
        };
        self.stage_import_path(path);
    }

    fn stage_import_path(&mut self, path: std::path::PathBuf) {
        // (source label, import outcome with per-format report)
        type Staged = (
            String,
            Result<(Document, x_native::fileio::ImportReport), String>,
        );
        let mut result: Option<Staged> = None;
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "sketch" {
            let bytes = match std::fs::read(&path) {
                Ok(v) => v,
                Err(e) => {
                    self.status = format!("import failed: {e}");
                    return;
                }
            };
            result = Some((
                "sketch".into(),
                x_native::fileio::import_sketch_with_report(&bytes),
            ));
        } else if ext == "json" {
            let text = match std::fs::read_to_string(&path) {
                Ok(v) => v,
                Err(e) => {
                    self.status = format!("import failed: {e}");
                    return;
                }
            };
            result = Some((
                "figma".into(),
                x_native::fileio::import_figma_json(&text).map(|d| {
                    let r = x_native::fileio::ImportReport {
                        nodes_imported: d.pages.iter().map(count_nodes).sum(),
                        assets_imported: d.assets.len(),
                        diagnostics: vec![],
                    };
                    (d, r)
                }),
            ));
        } else if ext == "svg" {
            let text = match std::fs::read_to_string(&path) {
                Ok(v) => v,
                Err(e) => {
                    self.status = format!("import failed: {e}");
                    return;
                }
            };
            result = Some((
                "svg".into(),
                x_native::fileio::import_svg(&text).map(|root| {
                    let mut d = Document::new();
                    d.pages.push(root);
                    let r = x_native::fileio::ImportReport {
                        nodes_imported: d.pages.iter().map(count_nodes).sum(),
                        assets_imported: 0,
                        diagnostics: vec![],
                    };
                    (d, r)
                }),
            ));
        } else if ext == "png" {
            let bytes = match std::fs::read(&path) {
                Ok(v) => v,
                Err(e) => {
                    self.status = format!("import failed: {e}");
                    return;
                }
            };
            let asset_name = path.file_stem().and_then(|v| v.to_str()).unwrap_or("image");
            let _ = self
                .assets
                .load_png(asset_name, path.to_string_lossy().as_ref());
            result = Some((
                "png".into(),
                x_native::fileio::import_png(asset_name, &bytes).map(|d| {
                    let r = x_native::fileio::ImportReport {
                        nodes_imported: d.pages.iter().map(count_nodes).sum(),
                        assets_imported: d.assets.len(),
                        diagnostics: vec![],
                    };
                    (d, r)
                }),
            ));
        }
        match result {
            Some((src, Ok((d, report)))) if !d.pages.is_empty() => {
                self.status = format!(
                    "{src}: {} node(s), {} asset(s), {} diagnostic(s) — review the preview",
                    report.nodes_imported,
                    report.assets_imported,
                    report.diagnostics.len()
                );
                self.import_pending = Some((src, d, report));
            }
            Some((src, Ok(_))) => self.status = format!("{src} file has no pages"),
            Some((src, Err(e))) => self.status = format!("{src} import FAILED: {e}"),
            None => self.status = "unsupported import format".into(),
        }
    }

    pub fn export_svg_now(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export SVG")
            .set_file_name("export.svg")
            .add_filter("SVG", &["svg"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let outliner = x_native::svg_text_outliner(&self.fonts);
        let resolver =
            |name: &str| -> Option<Vec<u8>> { std::fs::read(format!("assets/{name}.png")).ok() };
        let svg = x_native::fileio::export_svg_full(
            &self.editor.root,
            &self.vars,
            Some(&resolver),
            Some(&outliner),
        );
        self.status = if std::fs::write(&path, svg).is_ok() {
            format!("exported {}", path.display())
        } else {
            "export FAILED".into()
        };
    }

    pub fn export_png_now(&mut self) {
        let scale = self.export_scale;
        let node = self.export_target();
        let suffix = if scale == 1.0 {
            "".to_string()
        } else {
            format!("@{scale:.0}x")
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(format!("Export PNG{suffix}"))
            .set_file_name(format!("export{suffix}.png"))
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let doc = self.editor.root.clone();
        let node = node.clone();
        self.status = match export_raster_file(
            &doc,
            &node,
            &self.vars,
            &self.assets,
            &self.fonts,
            path.to_string_lossy().as_ref(),
            x_native::RasterFormat::Png,
            scale,
        ) {
            Ok((w, h)) => format!("exported {} ({w}x{h})", path.display()),
            Err(e) => format!("png export FAILED: {e}"),
        };
    }

    pub fn export_jpg_now(&mut self) {
        let scale = self.export_scale;
        let node = self.export_target();
        let suffix = if scale == 1.0 {
            "".to_string()
        } else {
            format!("@{scale:.0}x")
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(format!("Export JPG{suffix}"))
            .set_file_name(format!("export{suffix}.jpg"))
            .add_filter("JPEG", &["jpg", "jpeg"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let doc = self.editor.root.clone();
        let node = node.clone();
        self.status = match export_raster_file(
            &doc,
            &node,
            &self.vars,
            &self.assets,
            &self.fonts,
            path.to_string_lossy().as_ref(),
            x_native::RasterFormat::Jpg(90),
            scale,
        ) {
            Ok((w, h)) => format!("exported {} ({w}x{h})", path.display()),
            Err(e) => format!("jpg export FAILED: {e}"),
        };
    }

    /// The node raster export targets: a single selected node (its own size),
    /// otherwise the whole page.
    fn export_target(&self) -> Node {
        if self.editor.selection.len() == 1 {
            if let Some(id) = self.editor.selection.first() {
                if id != &self.editor.root.id {
                    if let Some(n) = x_native::editor::find(&self.editor.root, id) {
                        return n.clone();
                    }
                }
            }
        }
        self.editor.root.clone()
    }

    pub fn export_pdf_now(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export PDF")
            .set_file_name("export.pdf")
            .add_filter("PDF", &["pdf"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let tree = x_native::build_render_tree(&self.editor.root, &self.vars);
        let pdf = x_native::export_pdf_full(
            &tree,
            self.editor.root.w,
            self.editor.root.h,
            Some(&self.assets),
            Some(&self.fonts),
        );
        self.status = if std::fs::write(&path, pdf).is_ok() {
            format!("exported {}", path.display())
        } else {
            "pdf export FAILED".into()
        };
    }

    pub fn export_tokens_now(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Design Tokens (W3C DTCG)")
            .set_file_name("tokens.json")
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let doc = self.document_snapshot();
        let json = x_native::editor::export_tokens(&doc);
        self.status = if std::fs::write(&path, json).is_ok() {
            format!("exported {}", path.display())
        } else {
            "token export FAILED".into()
        };
    }

    fn document_snapshot(&mut self) -> Document {
        self.pages[self.page_idx] = self.editor.root.clone();
        let mut d = Document::new();
        d.variables = self.vars.clone();
        d.styles = self.styles.clone();
        d.assets = self.store.clone();
        d.library_deps = self.library_deps.clone();
        d.library_snapshots = self.library_snapshots.clone();
        d.pages = self.pages.clone();
        d
    }

    pub fn export_figma_now(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Figma-compatible JSON")
            .set_file_name("x-designer-export.json")
            .add_filter("Figma REST API JSON", &["json"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let doc = self.document_snapshot();
        let json = x_native::fileio::export_figma_json(&doc);
        self.status = match std::fs::write(&path, json) {
            Ok(_) => format!("exported Figma-compatible JSON: {}", path.display()),
            Err(e) => format!("Figma export FAILED: {e}"),
        };
    }

    pub fn export_sketch_now(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Sketch-compatible document")
            .set_file_name("x-designer.sketch")
            .add_filter("Sketch document", &["sketch"])
            .save_file()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let doc = self.document_snapshot();
        let bytes = x_native::fileio::export_sketch(&doc);
        self.status = match std::fs::write(&path, bytes) {
            Ok(_) => format!("exported Sketch-compatible file: {}", path.display()),
            Err(e) => format!("Sketch export FAILED: {e}"),
        };
    }

    // ------------------------------------------------- header dropdown menus
    // REAL menus (session 46): geometry shared between painter and click
    // handler via menu_title_rects()/menu_layout(); every item dispatches
    // through run_menu_tag into the SAME methods the shortcuts use.

    /// header right cluster (mockup): zoom pill halves + Share ghost +
    /// Prototype ghost + Present pill, laid out from the RIGHT edge (header
    /// spans full width).
    /// Returns (zoom_minus, zoom_label, zoom_plus, share, prototype, present).
    pub fn header_rects(&self) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
        let r2y = TAB_H;
        let pw = ui_measure("Present", 11.0) + 40.0;
        let pr = Rect::new(
            self.win_w - 48.0 - pw,
            r2y + 8.0,
            self.win_w - 48.0,
            TOP_H - 8.0,
        );
        let ptw = ui_measure("Prototype", 11.0) + 40.0;
        let ppr = Rect::new(pr.x0 - 10.0 - ptw, r2y + 8.0, pr.x0 - 10.0, TOP_H - 8.0);
        let sw = ui_measure("Share", 11.0) + 40.0;
        let shr = Rect::new(ppr.x0 - 10.0 - sw, r2y + 8.0, ppr.x0 - 10.0, TOP_H - 8.0);
        let zx = shr.x0 - 16.0 - 102.0;
        let bm = Rect::new(zx, r2y + 8.0, zx + 22.0, TOP_H - 8.0);
        let bl = Rect::new(zx + 24.0, r2y + 8.0, zx + 78.0, TOP_H - 8.0);
        let bp = Rect::new(zx + 80.0, r2y + 8.0, zx + 102.0, TOP_H - 8.0);
        (bm, bl, bp, shr, ppr, pr)
    }

    /// clickable title rects for File/Edit/View/Object/Help in header row 2
    pub fn menu_title_rects(&self) -> Vec<(usize, Rect)> {
        let mut out = vec![];
        let mut mx = 16.0;
        for (i, (title, _)) in MENUS.iter().enumerate() {
            let w = ui_measure(title, 11.0);
            out.push((
                i,
                Rect::new(mx - 6.0, TAB_H + 4.0, mx + w + 6.0, TOP_H - 4.0),
            ));
            mx += w + 24.0;
        }
        out
    }

    /// rows of the OPEN dropdown: (label, shortcut, action tag, rect)
    pub fn menu_layout(&self) -> Vec<(String, String, String, Rect)> {
        let Some(mi) = self.menu_open else {
            return vec![];
        };
        let title_r = self.menu_title_rects()[mi].1;
        let mut rows: Vec<(String, String, String)> = vec![];
        for (l, s, t) in MENUS[mi].1 {
            rows.push((l.to_string(), s.to_string(), t.to_string()));
        }
        // File menu: dynamic "Open Recent" section (MRU, most recent first)
        if mi == 0 {
            let files = x_native::fileio::recent_files();
            if !files.is_empty() {
                rows.push(("──────────────".into(), String::new(), "sep".into()));
                for (i, path) in files.iter().enumerate() {
                    let name = self
                        .dash_files
                        .iter()
                        .find(|f| &f.path == path)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| {
                            std::path::Path::new(path)
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        });
                    rows.push((name, String::new(), format!("file.recent:{i}")));
                }
                rows.push((
                    "Clear Recent".into(),
                    String::new(),
                    "file.clear_recent".into(),
                ));
            }
        }
        let mut w: f64 = 168.0;
        for (l, s, _) in &rows {
            w = w.max(ui_measure(l, 9.0) + ui_measure(s, 7.0) + 56.0);
        }
        let x0 = title_r.x0;
        rows.iter()
            .enumerate()
            .map(|(i, (l, s, t))| {
                let y = TOP_H + 4.0 + i as f64 * 24.0;
                (
                    l.clone(),
                    s.clone(),
                    t.clone(),
                    Rect::new(x0, y, x0 + w, y + 24.0),
                )
            })
            .collect()
    }

    /// Advanced-stroke popover geometry, shared by the painter and click
    /// handler (no drift). Anchored to the align pill in the Stroke row and
    /// opening left over the canvas (Figma opens advanced stroke this way).
    /// Controls are tagged: cap start 0..4, cap end 10..14, join 20..22,
    /// dash-preset cycle 30, dash offset -/+ 31/32, miter -/+ 33/34.
    pub fn stroke_advanced_geometry(&self) -> Option<(Rect, Vec<(u8, Rect)>)> {
        if !self.stroke_advanced_open {
            return None;
        }
        let ix = self.win_w - INSPECTOR_W;
        let ry = TOP_H + IY_STROKEROW;
        let pill_x1 = ix + INSPECTOR_W - 6.0; // align pill right edge
        let pw = 236.0;
        let ph = 200.0;
        let mut x0 = pill_x1 - 12.0 - pw;
        if x0 < 8.0 {
            x0 = 8.0;
        }
        let y0 = ry - 2.0;
        let panel = Rect::new(x0, y0, x0 + pw, y0 + ph);
        let px = x0 + 8.0;
        let cw = pw - 16.0;
        let mut c = Vec::new();
        let seg5 = |c: &mut Vec<(u8, Rect)>, base: u8, y: f64| {
            let sw = (cw - 8.0) / 5.0;
            for i in 0..5 {
                c.push((
                    base + i,
                    Rect::new(
                        px + i as f64 * (sw + 2.0),
                        y,
                        px + i as f64 * (sw + 2.0) + sw,
                        y + 18.0,
                    ),
                ));
            }
        };
        // Join (3 segments)
        {
            let sw = (cw - 4.0) / 3.0;
            let y = y0 + 20.0;
            for i in 0..3 {
                c.push((
                    20 + i,
                    Rect::new(
                        px + i as f64 * (sw + 2.0),
                        y,
                        px + i as f64 * (sw + 2.0) + sw,
                        y + 18.0,
                    ),
                ));
            }
        }
        // Cap start / end (5 segments each)
        seg5(&mut c, 0, y0 + 56.0);
        seg5(&mut c, 10, y0 + 100.0);
        // Dash preset cycle (full-width button)
        c.push((30, Rect::new(px, y0 + 138.0, px + cw, y0 + 156.0)));
        // Dash offset steppers (right-aligned pair)
        c.push((
            31,
            Rect::new(px + cw - 52.0, y0 + 162.0, px + cw - 36.0, y0 + 178.0),
        ));
        c.push((
            32,
            Rect::new(px + cw - 34.0, y0 + 162.0, px + cw - 18.0, y0 + 178.0),
        ));
        // Miter angle steppers (right-aligned pair)
        c.push((
            33,
            Rect::new(px + cw - 52.0, y0 + 182.0, px + cw - 36.0, y0 + 198.0),
        ));
        c.push((
            34,
            Rect::new(px + cw - 34.0, y0 + 182.0, px + cw - 18.0, y0 + 198.0),
        ));
        Some((panel, c))
    }

    /// Apply an advanced-stroke control (tag from stroke_advanced_geometry).
    /// Routes through mutate_visual_stack so every tweak is one undo step.
    pub fn apply_stroke_advanced(&mut self, tag: u8) {
        let Some(id) = self.editor.selection.first().cloned() else {
            return;
        };
        let idx = self.stroke_layer_index;
        let caps = [
            x_native::StrokeCap::None,
            x_native::StrokeCap::Round,
            x_native::StrokeCap::Square,
            x_native::StrokeCap::Arrow,
            x_native::StrokeCap::Triangle,
        ];
        let joins = [
            x_native::StrokeJoin::Miter,
            x_native::StrokeJoin::Bevel,
            x_native::StrokeJoin::Round,
        ];
        let dash_presets: Vec<Vec<f64>> = vec![
            vec![],
            vec![8.0, 4.0],
            vec![2.0, 4.0],
            vec![12.0, 4.0, 2.0, 4.0],
            vec![1.0, 3.0],
        ];
        let next_dash = |d: &[f64]| {
            let pos = dash_presets.iter().position(|p| p == d).unwrap_or(0);
            dash_presets[(pos + 1) % dash_presets.len()].clone()
        };
        match tag {
            // cap start / cap end (5-way segmented)
            t @ 0..=4 => {
                let c = caps[t as usize];
                self.editor.mutate_visual_stack(&id, move |nm| {
                    if let Some(l) = nm.stroke_layers.get_mut(idx) {
                        l.options.cap_start = c;
                    }
                });
                self.status = "cap start set".into();
            }
            t @ 10..=14 => {
                let c = caps[(t - 10) as usize];
                self.editor.mutate_visual_stack(&id, move |nm| {
                    if let Some(l) = nm.stroke_layers.get_mut(idx) {
                        l.options.cap_end = c;
                    }
                });
                self.status = "cap end set".into();
            }
            // join (3-way segmented)
            t @ 20..=22 => {
                let j = joins[(t - 20) as usize];
                self.editor.mutate_visual_stack(&id, move |nm| {
                    if let Some(l) = nm.stroke_layers.get_mut(idx) {
                        l.options.join = j;
                    }
                });
                self.status = "stroke join set".into();
            }
            // dash preset cycle
            30 => {
                let cur = self
                    .selected_stroke_options()
                    .map(|o| o.dash)
                    .unwrap_or_default();
                let nd = next_dash(&cur);
                self.editor.mutate_visual_stack(&id, move |nm| {
                    if let Some(l) = nm.stroke_layers.get_mut(idx) {
                        l.options.dash = nd;
                    }
                });
                self.status = "dash pattern changed".into();
            }
            // dash offset steppers
            31 | 32 => {
                let dir = if tag == 31 { -4.0 } else { 4.0 };
                self.editor.mutate_visual_stack(&id, move |nm| {
                    if let Some(l) = nm.stroke_layers.get_mut(idx) {
                        l.options.dash_offset = (l.options.dash_offset + dir).max(0.0);
                    }
                });
                self.status = "dash offset changed".into();
            }
            // miter limit steppers
            33 | 34 => {
                let dir = if tag == 33 { -1.0 } else { 1.0 };
                self.editor.mutate_visual_stack(&id, move |nm| {
                    if let Some(l) = nm.stroke_layers.get_mut(idx) {
                        l.options.miter_limit = (l.options.miter_limit + dir).clamp(1.0, 60.0);
                    }
                });
                self.status = "miter limit changed".into();
            }
            _ => {}
        }
    }

    /// Current StrokeOptions of the selected stroke layer (default if none).
    pub fn selected_stroke_options(&self) -> Option<x_native::StrokeOptions> {
        let n = self.selected_single()?;
        let stroke_len = if !n.visual_stacks_materialized {
            if n.stroke.width > 0.0 {
                1
            } else {
                0
            }
        } else {
            n.stroke_layers.len()
        };
        let idx = self.stroke_layer_index.min(stroke_len.saturating_sub(1));
        n.stroke_layers
            .get(idx)
            .map(|l| l.options.clone())
            .or_else(|| {
                if stroke_len > 0 {
                    Some(x_native::StrokeOptions::default())
                } else {
                    None
                }
            })
    }

    /// Polish: menu items gray out when they can't apply right now.
    pub fn menu_item_enabled(&self, tag: &str) -> bool {
        let has_sel = !self.editor.selection.is_empty();
        match tag {
            "edit.undo" => self.editor.undo_depth() > 0,
            "edit.duplicate" | "edit.delete" | "obj.front" | "obj.back" | "obj.forward"
            | "obj.backward" | "obj.mask" => has_sel,
            "edit.cut" | "edit.copy" => has_sel,
            "edit.paste" | "edit.paste_over" | "edit.paste_replace" => {
                self.editor.clipboard_len() > 0
            }
            "page.delete" => self.pages.len() > 1,
            "obj.group" | "obj.union" | "obj.subtract" | "obj.intersect" | "obj.exclude" => {
                self.editor.selection.len() >= 2
            }
            "obj.ungroup"
            | "obj.component"
            | "obj.frame_selection"
            | "obj.section"
            | "obj.flatten"
            | "edit.select_similar"
            | "edit.select_inside" => has_sel,
            "obj.tidy" => {
                self.editor.selection.len() >= 2
                    || self
                        .editor
                        .selection
                        .first()
                        .and_then(|id| find(&self.editor.root, id))
                        .is_some_and(|n| {
                            matches!(
                                n.kind,
                                x_native::NodeKind::Group
                                    | x_native::NodeKind::Section
                                    | x_native::NodeKind::Frame { .. }
                            ) && n.children.len() >= 2
                        })
            }
            "obj.outline" => self
                .editor
                .selection
                .first()
                .and_then(|id| find(&self.editor.root, id))
                .is_some_and(|n| n.stroke.width > 0.0),
            "obj.to_grid" | "obj.to_stack" => self
                .editor
                .selection
                .iter()
                .filter_map(|id| find(&self.editor.root, id))
                .any(|n| matches!(n.kind, x_native::NodeKind::Frame { .. })),
            "obj.detach_instance" | "obj.reset_overrides" => self
                .editor
                .selection
                .iter()
                .filter_map(|id| find(&self.editor.root, id))
                .any(|n| matches!(n.kind, x_native::NodeKind::Instance { .. })),
            "obj.combine_variants" => self.editor.selection.len() >= 2,
            "arr.disth" | "arr.distv" => self.editor.selection.len() >= 3,
            "edit.copy_svg" => has_sel,
            "page.left" => self.page_idx > 0,
            "page.right" => self.page_idx + 1 < self.pages.len(),
            "arr.fliph" | "arr.flipv" => has_sel,
            t if t.starts_with("arr.") => self.editor.selection.len() >= 2,
            "sep" => false,
            t if t.starts_with("file.recent:") => true,
            "file.clear_recent" => !x_native::fileio::recent_files().is_empty(),
            "noop" => false,
            _ => true,
        }
    }

    pub fn run_menu_tag(&mut self, tag: &str) {
        match tag {
            "file.new_page" => self.add_page(),
            "file.new" => self.new_file(),
            "file.dashboard" => self.back_to_dashboard(),
            "file.clear_recent" => {
                x_native::fileio::clear_recent();
                self.status = "recent documents cleared".into();
            }
            t if t.starts_with("file.recent:") => {
                let idx: usize = t.trim_start_matches("file.recent:").parse().unwrap_or(0);
                if let Some(path) = x_native::fileio::recent_files().get(idx) {
                    self.open_file(path);
                } else {
                    self.status = "recent document no longer available".into();
                }
            }
            "edit.cut" => self.clipboard_cut(),
            "edit.copy" => self.clipboard_copy(),
            "edit.paste" => self.clipboard_paste(),
            "edit.paste_over" => self.clipboard_paste_over_selection(),
            "edit.paste_replace" => self.clipboard_paste_to_replace(),
            "page.rename" => self.start_page_rename(self.page_idx),
            "page.left" => self.reorder_page(-1),
            "page.right" => self.reorder_page(1),
            "edit.paste_svg" => self.paste_svg_from_clipboard(),
            "page.duplicate" => self.duplicate_page(),
            "page.delete" => {
                let i = self.page_idx;
                self.delete_page(i);
            }
            "view.pages" => self.toggle_thumbs(),
            "file.open" => self.choose_and_open_document(),
            "file.save" => self.save_document(),
            "file.save_as" => self.save_document_as(),
            "file.import" => self.start_import(),
            "file.import_x" => self.choose_and_open_document(),
            "file.import_figma" => self.start_figma_import(),
            "file.import_sketch" => self.start_sketch_import(),
            "file.export_x" => self.save_document_as(),
            "file.export_figma" => self.export_figma_now(),
            "file.export_sketch" => self.export_sketch_now(),
            "file.export_svg" => self.export_svg_now(),
            "file.export_png" => self.export_png_now(),
            "file.export_jpg" => self.export_jpg_now(),
            "file.export_pdf" => self.export_pdf_now(),
            "file.export_tokens" => self.export_tokens_now(),
            "file.export_2x" => {
                self.export_scale = 2.0;
                self.export_png_now();
            }
            "file.export_3x" => {
                self.export_scale = 3.0;
                self.export_png_now();
            }
            "file.export_1x" => {
                self.export_scale = 1.0;
                self.export_png_now();
            }
            "file.batch_export" => self.batch_export_now(),
            "file.share" => {
                self.share_open = true;
                self.status = "share dialog open".into();
            }
            "edit.undo" => {
                self.editor.undo();
                self.status = "undo".into();
            }
            "edit.redo" => {
                self.editor.redo();
                self.status = "redo".into();
            }
            "edit.duplicate" => {
                self.editor.duplicate_selection((16.0, 16.0));
                self.status = "duplicated".into();
            }
            "edit.delete" => {
                self.editor.delete_selection();
                self.status = "deleted".into();
            }
            "edit.select_all" => {
                self.editor.select_all();
                self.status = format!("{} selected", self.editor.selection.len());
            }
            "edit.select_similar" => {
                let n = self.editor.select_similar();
                self.status = format!("selected {n} similar node(s)");
            }
            "edit.select_inside" => {
                let n = self.editor.select_inside();
                if n > 0 {
                    self.status = format!("selected {n} node(s) inside");
                } else {
                    self.status = "nothing selected".into();
                }
            }
            "view.zoom_in" => {
                self.zoom = (self.zoom * 1.25).clamp(0.05, 16.0);
                self.status = format!("zoom {}%", (self.zoom * 100.0).round());
            }
            "view.zoom_out" => {
                self.zoom = (self.zoom / 1.25).clamp(0.05, 16.0);
                self.status = format!("zoom {}%", (self.zoom * 100.0).round());
            }
            "view.zoom_100" => {
                self.zoom = 1.0;
                self.status = "zoom 100%".into();
            }
            "view.zoom_fit" => {
                let cw = self.win_w - LAYERS_W - INSPECTOR_W - 40.0;
                let chh = self.win_h - TOP_H - self.thumbs_h() - STATUS_H - 40.0;
                self.zoom = (cw / self.editor.root.w.max(1.0))
                    .min(chh / self.editor.root.h.max(1.0))
                    .clamp(0.02, 4.0);
                self.pan = (20.0, 20.0);
                self.status = format!("zoom to fit ({:.0}%)", self.zoom * 100.0);
            }
            "view.rulers" => {
                self.rulers = !self.rulers;
                self.status = if self.rulers {
                    "rulers on".into()
                } else {
                    "rulers off".into()
                };
            }
            "view.outline" => {
                self.outline_view = !self.outline_view;
                self.status = if self.outline_view {
                    "outline view".into()
                } else {
                    "normal view".into()
                };
            }
            "view.vars" => {
                self.inspector_tab = 2;
                self.status = "variables tab".into();
            }
            "view.minimap" => {
                self.minimap = !self.minimap;
                self.status = if self.minimap {
                    "minimap on".into()
                } else {
                    "minimap off".into()
                };
            }
            "view.hud" => {
                self.perf_hud = !self.perf_hud;
                self.status = if self.perf_hud {
                    "perf HUD on".into()
                } else {
                    "perf HUD off".into()
                };
            }
            "view.hide_ui" => {
                self.chrome_hidden = true;
                self.status = "UI hidden (⌘. to show)".into();
            }
            "arr.fliph" | "arr.flipv" => {
                let horizontal = tag == "arr.fliph";
                let ids = self.editor.selection.clone();
                let depth = self.editor.undo_depth();
                for id in ids {
                    self.editor.flip_node(&id, horizontal);
                }
                self.editor
                    .merge_last(self.editor.undo_depth().saturating_sub(depth));
                self.status = if horizontal {
                    "flipped horizontally".into()
                } else {
                    "flipped vertically".into()
                };
            }
            "obj.group" => {
                if self.editor.selection.len() >= 2 {
                    let gid = format!("group-{}", self.editor.undo_depth());
                    self.editor.group_selection(&gid);
                    self.status = format!("grouped -> {gid}");
                } else {
                    self.status = "select 2+ nodes to group".into();
                }
            }
            "obj.frame_selection" => {
                if !self.editor.selection.is_empty() {
                    let fid = format!("frame-{}", self.editor.undo_depth());
                    self.editor.frame_selection(&fid);
                    self.status = format!("frame selection -> {fid}");
                } else {
                    self.status = "select 1+ nodes to frame".into();
                }
            }
            "obj.tidy" => match self.editor.tidy_up() {
                Some((moved, cols, rows)) => {
                    self.status = format!("tidied {moved} node(s) into {rows}x{cols} grid");
                }
                None => {
                    self.status = "select 2+ siblings (or one group/frame/section)".into();
                }
            },
            "obj.section" => {
                let sid = format!("section-{}", self.editor.undo_depth());
                let n = self.editor.selection.len();
                self.editor.section_selection(&sid);
                if self.editor.selection.first().is_some_and(|s| s == &sid) {
                    self.status = format!("section -> {sid} ({n} node(s))");
                } else {
                    self.status = "select sibling nodes first".into();
                }
            }
            "obj.ungroup" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if self.editor.ungroup(&id) {
                        self.status = "ungrouped".into();
                    }
                }
            }
            "obj.front" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.bring_to_front(&id);
                    self.status = "to front".into();
                }
            }
            "obj.back" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.send_to_back(&id);
                    self.status = "to back".into();
                }
            }
            "obj.forward" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.bring_forward(&id);
                    self.status = "forward".into();
                }
            }
            "obj.backward" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.send_backward(&id);
                    self.status = "backward".into();
                }
            }
            "obj.union" | "obj.subtract" | "obj.intersect" | "obj.exclude" => {
                use x_native::editor::BoolOp::*;
                let op = match tag {
                    "obj.union" => Union,
                    "obj.subtract" => Subtract,
                    "obj.intersect" => Intersect,
                    _ => Exclude,
                };
                match self.editor.boolean_selected(op) {
                    Some(id) => self.status = format!("{op:?} -> {id}"),
                    None => self.status = "boolean needs 2 shape nodes".into(),
                }
            }
            "obj.flatten" => match self.editor.flatten_selected() {
                Some(id) => self.status = format!("flatten -> {id}"),
                None => self.status = "select one group or shape to flatten".into(),
            },
            "obj.outline" => match self.editor.outline_stroke_selected() {
                Some(id) => self.status = format!("outline stroke -> {id}"),
                None => self.status = "select one stroked shape (width > 0)".into(),
            },
            "obj.mask" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                        n.is_mask = !n.is_mask;
                        self.status = format!("{id} mask: {}", n.is_mask);
                    }
                }
            }
            "obj.component" => {
                let n = self.editor.selection.len();
                let name = format!("Component{}", self.editor.component_names().len() + 1);
                if self.editor.make_component(&name) {
                    self.status = format!("created component {name} from {n} node(s)");
                } else {
                    self.status = "select sibling nodes first".into();
                }
            }
            "obj.detach_instance" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    match self.editor.detach(&id, &self.vars) {
                        Some(new_id) => {
                            self.editor.selection = vec![new_id.clone()];
                            self.status = format!("detached -> {new_id}");
                        }
                        None => self.status = "select an instance to detach".into(),
                    }
                }
            }
            "obj.to_grid" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    // 3-column auto grid, 8px gaps — a sensible gallery start
                    let layout = x_native::AutoLayout {
                        grid: Some(x_native::GridLayout::default()),
                        ..Default::default()
                    };
                    if self.editor.set_auto_layout(&id, Some(layout), &self.vars) {
                        self.status = "converted to grid (3 auto cols, 8px gap)".into();
                    } else {
                        self.status = "select a frame".into();
                    }
                }
            }
            "obj.to_stack" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    let layout = x_native::AutoLayout {
                        grid: None,
                        ..Default::default()
                    };
                    if self.editor.set_auto_layout(&id, Some(layout), &self.vars) {
                        self.status = "converted to vertical stack".into();
                    } else {
                        self.status = "select a frame".into();
                    }
                }
            }
            "obj.grid" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                        if matches!(n.kind, x_native::NodeKind::Frame { .. }) {
                            if n.layout_grids.is_empty() {
                                n.layout_grids.push(LayoutGridDef::default());
                                self.status =
                                    "layout grid: 12 columns — tune it in the Design panel".into();
                            } else {
                                n.layout_grids.clear();
                                self.status = "layout grids removed".into();
                            }
                        } else {
                            self.status = "layout grids apply to frames".into();
                        }
                    }
                }
            }
            "obj.reset_overrides" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if self.editor.reset_instance_overrides(&id) {
                        self.status = "overrides reset (slot content kept)".into();
                    } else {
                        self.status = "select an instance".into();
                    }
                }
            }
            "arr.left" | "arr.centerh" | "arr.right" | "arr.top" | "arr.centerv" | "arr.bottom" => {
                use x_native::editor::AlignKind::*;
                let kind = match tag {
                    "arr.left" => Left,
                    "arr.centerh" => CenterH,
                    "arr.right" => Right,
                    "arr.top" => Top,
                    "arr.centerv" => CenterV,
                    _ => Bottom,
                };
                let ids = self.editor.selection.clone();
                if ids.len() >= 2 {
                    x_native::editor::align(&mut self.editor.root, &ids, kind);
                    self.status = format!("aligned {:?}", kind);
                } else {
                    self.status = "select 2+ layers to align".into();
                }
            }
            "arr.disth" | "arr.distv" => {
                // distribute-spacing: sort by axis, equalize the gaps
                let ids = self.editor.selection.clone();
                if ids.len() < 3 {
                    self.status = "select 3+ layers to distribute".into();
                } else {
                    let horizontal = tag == "arr.disth";
                    let mut items: Vec<(String, f64, f64)> = ids
                        .iter()
                        .filter_map(|id| {
                            find(&self.editor.root, id).map(|n| {
                                (
                                    id.clone(),
                                    if horizontal {
                                        n.transform.x
                                    } else {
                                        n.transform.y
                                    },
                                    if horizontal { n.w } else { n.h },
                                )
                            })
                        })
                        .collect();
                    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    let first = items.first().unwrap().clone();
                    let last = items.last().unwrap().clone();
                    let span = (last.1 + last.2) - first.1;
                    let content: f64 = items.iter().map(|(_, _, sz)| sz).sum();
                    let gap = (span - content) / (items.len() - 1) as f64;
                    let mut cursor = first.1;
                    for (id, _, sz) in &items {
                        if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, id) {
                            if horizontal {
                                n.transform.x = cursor;
                            } else {
                                n.transform.y = cursor;
                            }
                            n.dirty = true;
                        }
                        cursor += sz + gap;
                    }
                    self.status = format!(
                        "distributed {} layers ({})",
                        items.len(),
                        if horizontal { "horizontal" } else { "vertical" }
                    );
                }
            }
            "edit.copy_svg" => self.copy_as_svg(),
            "help.shortcuts" => self.help_open = true,
            _ => {}
        }
    }

    // ------------------------------------------------- left panel icon tabs
    // (session 46): Layers/Assets/Components/Library are REAL panels now.
    // Geometry shared between painter and click handler.

    pub fn left_tab_rects(&self) -> Vec<(usize, Rect)> {
        // mockup: 4 equal columns, icon above label, full-height strip
        let cw = LAYERS_W / 4.0;
        (0..LEFT_TABS.len())
            .map(|i| {
                (
                    i,
                    Rect::new(i as f64 * cw, TOP_H, (i + 1) as f64 * cw, TOP_H + LTAB_H),
                )
            })
            .collect()
    }

    /// Content rows/tiles of the non-Layers left tabs.
    /// kinds: 1 asset tile, 2 document component row, 3 library component
    /// row ("libid|name"), 4 open-full-manager button, 5 plain label
    pub fn left_panel_layout(&self) -> Vec<(String, Rect, u8)> {
        let mut out = vec![];
        let y0 = TOP_H + LTAB_H + 26.0; // below the tab strip + caption
        match self.left_tab {
            1 => {
                // ASSETS: 2-column grid of document asset tiles
                let tile_w = (LAYERS_W - 34.0) / 2.0;
                let tile_h = tile_w * 0.72;
                for (i, (_name, id)) in self.sorted_assets().iter().enumerate() {
                    let col = (i % 2) as f64;
                    let row = (i / 2) as f64;
                    let x = 12.0 + col * (tile_w + 10.0);
                    let y = y0 + row * (tile_h + 26.0);
                    if y + tile_h > self.win_h - 40.0 {
                        break;
                    }
                    out.push((id.clone(), Rect::new(x, y, x + tile_w, y + tile_h), 1));
                }
            }
            2 => {
                // COMPONENTS: document components, then linked library ones
                let comps = self.editor.component_names();
                let mut y = y0;
                for name in &comps {
                    out.push((
                        name.clone(),
                        Rect::new(8.0, y, LAYERS_W - 8.0, y + ROW_H - 2.0),
                        2,
                    ));
                    y += ROW_H;
                }
                for dep in &self.library_deps {
                    let Some(lib) = self.library_snapshots.get(&dep.library_id) else {
                        continue;
                    };
                    if lib.components.is_empty() {
                        continue;
                    }
                    out.push((
                        format!("{} v{}", lib.name, dep.resolved_version),
                        Rect::new(8.0, y + 8.0, LAYERS_W - 8.0, y + 22.0),
                        5,
                    ));
                    y += 26.0;
                    for c in lib.components.iter().take(10) {
                        if let x_native::NodeKind::Component { name } = &c.kind {
                            out.push((
                                format!("{}|{name}", dep.library_id),
                                Rect::new(8.0, y, LAYERS_W - 8.0, y + ROW_H - 2.0),
                                3,
                            ));
                            y += ROW_H;
                        }
                    }
                }
            }
            3 => {
                // LIBRARY: linked library summaries + jump to full manager
                let mut y = y0;
                for dep in &self.library_deps {
                    let Some(lib) = self.library_snapshots.get(&dep.library_id) else {
                        continue;
                    };
                    out.push((
                        format!("{} v{}", lib.name, dep.resolved_version),
                        Rect::new(8.0, y, LAYERS_W - 8.0, y + 16.0),
                        5,
                    ));
                    y += 18.0;
                    let ok = self
                        .library_integrity
                        .iter()
                        .find(|(id, _)| *id == dep.library_id)
                        .map(|(_, s)| s.starts_with("Verified"))
                        .unwrap_or(true);
                    let badge = format!(
                        "{} style(s), {} comp(s){}",
                        lib.styles.len(),
                        lib.components.len(),
                        if ok { "" } else { " — INTEGRITY!" }
                    );
                    out.push((badge, Rect::new(8.0, y, LAYERS_W - 8.0, y + 14.0), 5));
                    y += 22.0;
                }
                out.push((
                    "OPEN LIBRARY MANAGER".into(),
                    Rect::new(12.0, y + 6.0, LAYERS_W - 12.0, y + 28.0),
                    4,
                ));
            }
            _ => {}
        }
        out
    }

    /// Export section (mockup, bottom of the Design inspector): buttons
    /// (label, action tag, rect) — geometry shared painter/click.
    pub fn export_layout(&self) -> Vec<(&'static str, &'static str, Rect)> {
        let ix = self.win_w - INSPECTOR_W;
        let y = self.win_h - self.thumbs_h() - STATUS_H - 90.0;
        let bw = (INSPECTOR_W - 24.0 - 16.0) / 3.0;
        [
            ("X", "file.export_x"),
            ("FIG", "file.export_figma"),
            ("SKETCH", "file.export_sketch"),
            ("PNG", "file.export_png"),
            ("JPG", "file.export_jpg"),
            ("SVG", "file.export_svg"),
            ("PDF", "file.export_pdf"),
            ("BATCH", "file.batch_export"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (l, t))| {
            let col = i % 3;
            let row = i / 3;
            (
                *l,
                *t,
                Rect::new(
                    ix + 12.0 + col as f64 * (bw + 8.0),
                    y + row as f64 * 28.0,
                    ix + 12.0 + col as f64 * (bw + 8.0) + bw,
                    y + row as f64 * 28.0 + 24.0,
                ),
            )
        })
        .collect()
    }

    /// The single selected node whose per-node export settings are shown in
    /// the Export panel, or None when the selection is empty/multi/root.
    pub fn selected_export_node_id(&self) -> Option<String> {
        if self.editor.selection.len() != 1 {
            return None;
        }
        let id = self.editor.selection.first()?.clone();
        if id == self.editor.root.id {
            return None;
        }
        Some(id)
    }

    /// Per-node export-settings panel geometry (Design tab), shared painter /
    /// click. Sits above the quick-format buttons. `rows` are top-to-bottom.
    #[allow(clippy::type_complexity)]
    pub fn export_settings_layout(
        &self,
    ) -> Option<(String, Vec<(usize, Rect, Rect, Rect)>, Rect, Rect, f64)> {
        let id = self.selected_export_node_id()?;
        let n = find(&self.editor.root, &id)?;
        let count = n.export_settings.len();
        let ix = self.win_w - INSPECTOR_W;
        let qy = self.win_h - self.thumbs_h() - STATUS_H - 90.0; // quick buttons top
        let w = self.win_w - ix - 24.0;
        // export button (bottom of the panel)
        let export_btn = Rect::new(ix + 12.0, qy - 40.0, ix + 12.0 + w, qy - 12.0);
        // "+ add" row
        let add = Rect::new(
            ix + 12.0,
            export_btn.y0 - 8.0 - 22.0,
            ix + 12.0 + 110.0,
            export_btn.y0 - 8.0,
        );
        // setting rows, stacked upward
        let mut rows: Vec<(usize, Rect, Rect, Rect)> = Vec::new();
        let mut y = add.y0 - 8.0;
        for i in (0..count).rev() {
            let row = Rect::new(ix + 12.0, y - 24.0, ix + 12.0 + w, y);
            let fmt = Rect::new(row.x0, row.y0, row.x0 + 52.0, row.y1);
            let scale = Rect::new(row.x0 + 58.0, row.y0, row.x0 + 96.0, row.y1);
            let remove = Rect::new(row.x1 - 24.0, row.y0, row.x1, row.y1);
            rows.push((i, fmt, scale, remove));
            y -= 28.0;
        }
        rows.reverse();
        let top = y; // panel top (header baseline ~ top - 8)
        Some((id, rows, add, export_btn, top))
    }

    fn cycle_export_format(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut s = n.export_settings.clone();
            if let Some(e) = s.get_mut(index) {
                e.format = match e.format.as_str() {
                    "png" => "jpg".into(),
                    "jpg" => "svg".into(),
                    _ => "png".into(),
                };
            }
            self.editor.set_export_settings(id, s);
        }
    }

    fn cycle_export_scale(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut s = n.export_settings.clone();
            if let Some(e) = s.get_mut(index) {
                e.scale = match e.scale {
                    1.0 => 2.0,
                    2.0 => 3.0,
                    _ => 1.0,
                };
                e.suffix = match e.scale {
                    1.0 => String::new(),
                    x => format!("@{x:.0}x"),
                };
            }
            self.editor.set_export_settings(id, s);
        }
    }

    fn add_export_setting(&mut self, id: &str) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut s = n.export_settings.clone();
            s.push(ExportSettings::default());
            self.editor.set_export_settings(id, s);
        }
    }

    fn remove_export_setting(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut s = n.export_settings.clone();
            if index < s.len() {
                s.remove(index);
            }
            self.editor.set_export_settings(id, s);
        }
    }

    /// Write one file per export setting on the selected node into a chosen
    /// folder (Figma's "Export" panel action).
    pub fn export_selected_settings(&mut self) {
        let Some(id) = self.selected_export_node_id() else {
            self.status = "select a layer to export".into();
            return;
        };
        let Some(node) = find(&self.editor.root, &id).cloned() else {
            return;
        };
        if node.export_settings.is_empty() {
            self.status = "add an export setting first (+)".into();
            return;
        }
        let Some(dir) = rfd::FileDialog::new()
            .set_title("Export folder")
            .pick_folder()
        else {
            self.status = "export cancelled".into();
            return;
        };
        let doc = self.editor.root.clone();
        let dir = dir.to_string_lossy().to_string();
        let msg =
            match export_node_settings(&doc, &node, &self.vars, &self.assets, &self.fonts, &dir) {
                Ok(n) => format!("exported {n} file(s) to {dir}"),
                Err(e) => format!("export FAILED: {e}"),
            };
        self.status = msg;
    }

    /// Which nodes a batch export covers: the multi-selection when ≥2 layers
    /// are selected, else every non-root node that carries export settings.
    fn collect_batch_nodes(&self) -> Vec<Node> {
        if self.editor.selection.len() >= 2 {
            return self
                .editor
                .selection
                .iter()
                .filter_map(|id| find(&self.editor.root, id).cloned())
                .collect();
        }
        fn walk(n: &Node, out: &mut Vec<Node>) {
            for c in &n.children {
                if !c.export_settings.is_empty() {
                    out.push(c.clone());
                }
                walk(c, out);
            }
        }
        let mut out = vec![];
        walk(&self.editor.root, &mut out);
        out
    }

    /// Batch export: multi-selection, or every layer with export settings,
    /// into one folder (Figma's slice/selection batch-export workflow).
    pub fn batch_export_now(&mut self) {
        let nodes = self.collect_batch_nodes();
        if nodes.is_empty() {
            self.status =
                "nothing to export — select several layers, or add export settings".into();
            return;
        }
        let Some(dir) = rfd::FileDialog::new()
            .set_title("Batch export folder")
            .pick_folder()
        else {
            self.status = "batch export cancelled".into();
            return;
        };
        let doc = self.editor.root.clone();
        let dir = dir.to_string_lossy().to_string();
        let n = nodes.len();
        let msg =
            match batch_export_nodes(&doc, &nodes, &self.vars, &self.assets, &self.fonts, &dir) {
                Ok(c) => format!("batch exported {c} file(s) from {n} layer(s) to {dir}"),
                Err(e) => format!("batch export FAILED: {e}"),
            };
        self.status = msg;
    }

    /// Friendly label for an interaction action (Prototype panel).
    pub fn action_label(a: &Action) -> String {
        match a {
            Action::Navigate { .. } => "Navigate".into(),
            Action::OpenOverlay { .. } => "Open overlay".into(),
            Action::SwapOverlay { .. } => "Swap overlay".into(),
            Action::CloseOverlay => "Close overlay".into(),
            Action::ScrollTo { .. } => "Scroll to".into(),
            Action::Back => "Back".into(),
            Action::SetVar { .. } => "Set variable".into(),
            Action::SetMode { .. } => "Set mode".into(),
            Action::Cond { .. } => "Conditional".into(),
        }
    }

    /// Page ids (destinations) an interaction can target: every other page.
    /// Short label for a nested then/else action chip.
    pub fn nested_label(&self, a: &Action) -> String {
        match a {
            Action::Navigate { destination } => short_id(&self.dest_label(destination)),
            Action::Back => "back".into(),
            Action::SetVar { name, value } => format!("{name} = {}", format_expr(value)),
            Action::SetMode { mode } => format!("mode: {mode}"),
            _ => a.kind().to_string(),
        }
    }

    fn proto_destinations(&self) -> Vec<String> {
        self.pages
            .iter()
            .map(|p| p.id.clone())
            .filter(|id| id != &self.editor.root.id)
            .collect()
    }

    /// Display form of a prototype destination id: the page's NAME when it
    /// resolves to a page, else the raw id (Figma shows the target's name).
    pub fn dest_label(&self, id: &str) -> String {
        self.pages
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    /// Prototype-panel geometry for the selected node (painter + click share).
    pub fn prototype_ui(&self) -> Option<ProtoUi> {
        let id = self.selected_single()?.id.clone();
        let n = find(&self.editor.root, &id)?;
        let count = n.interactions.len();
        let ix = self.win_w - INSPECTOR_W;
        let w = self.win_w - ix - 24.0;
        let y0 = TOP_H + 36.0;
        let start_toggle = Rect::new(ix + 12.0, y0, ix + 12.0 + w, y0 + 18.0);
        let mut rows = Vec::new();
        let mut y = y0 + 34.0;
        for i in 0..count {
            // logic actions (Cond) and overlay+keydown pairs need a third line
            let tall = matches!(
                n.interactions.get(i),
                Some(x) if matches!(x.action, Action::Cond { .. })
                    || (matches!(x.trigger, Trigger::KeyDown { .. })
                        && matches!(x.action, Action::OpenOverlay { .. }))
            );
            let trigger = Rect::new(ix + 12.0, y, ix + 12.0 + 96.0, y + 18.0);
            let action = Rect::new(ix + 12.0 + 102.0, y, ix + 12.0 + w - 24.0, y + 18.0);
            let remove = Rect::new(ix + 12.0 + w - 20.0, y, ix + 12.0 + w, y + 18.0);
            let dest = Rect::new(ix + 12.0, y + 22.0, ix + 12.0 + 110.0, y + 40.0);
            let pos = Rect::new(
                ix + 12.0 + 116.0,
                y + 22.0,
                ix + 12.0 + 116.0 + 64.0,
                y + 40.0,
            );
            let anim = Rect::new(ix + 12.0 + w - 86.0, y + 22.0, ix + 12.0 + w, y + 40.0);
            let extra = Rect::new(ix + 12.0, y + 44.0, ix + 12.0 + 150.0, y + 62.0);
            rows.push(ProtoRowUi {
                index: i,
                trigger,
                action,
                dest,
                pos,
                anim,
                remove,
                extra,
            });
            y += if tall { 68.0 } else { 46.0 };
        }
        let add = Rect::new(ix + 12.0, y + 4.0, ix + 12.0 + 138.0, y + 22.0);
        Some(ProtoUi {
            id,
            start_toggle,
            add,
            rows,
        })
    }

    fn toggle_starting_point(&mut self, id: &str) {
        if let Some(n) = find(&self.editor.root, id) {
            self.editor.set_starting_point(id, !n.is_starting_point);
        }
    }

    fn cycle_trigger(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if let Some(i) = list.get_mut(index) {
                i.trigger = match i.trigger {
                    Trigger::OnClick => Trigger::OnHover,
                    Trigger::OnHover => Trigger::OnPress,
                    Trigger::OnPress => Trigger::OnDrag,
                    Trigger::OnDrag => Trigger::AfterDelay { ms: 500 },
                    Trigger::AfterDelay { .. } => Trigger::MouseEnter,
                    Trigger::MouseEnter => Trigger::MouseLeave,
                    Trigger::MouseLeave => Trigger::KeyDown { key: "a".into() },
                    Trigger::KeyDown { .. } => Trigger::OnClick,
                };
            }
            self.editor.set_interactions(id, list);
        }
    }

    fn cycle_action(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if let Some(i) = list.get_mut(index) {
                let dest = i.action.target().unwrap_or("").to_string();
                i.action = match i.action.kind() {
                    "navigate" => Action::OpenOverlay {
                        overlay: dest,
                        position: OverlayPosition::Center,
                    },
                    "overlay" => Action::SwapOverlay { overlay: dest },
                    "swap" => Action::ScrollTo { destination: dest },
                    "scroll" => Action::CloseOverlay,
                    "close" => {
                        let (name, value) = self.proto_setvar_default();
                        Action::SetVar { name, value }
                    }
                    "setvar" => Action::SetMode {
                        mode: self.proto_first_mode(),
                    },
                    "setmode" => Action::Cond {
                        cond: self.proto_default_cond(),
                        then: Box::new(match self.proto_destinations().first() {
                            Some(d) => Action::Navigate {
                                destination: d.clone(),
                            },
                            None => Action::Back,
                        }),
                        els: None,
                    },
                    _ => Action::Navigate { destination: dest },
                };
            }
            self.editor.set_interactions(id, list);
        }
    }

    fn cycle_dest(&mut self, id: &str, index: usize) {
        let dests = self.proto_destinations();
        if dests.is_empty() {
            return;
        }
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if let Some(i) = list.get_mut(index) {
                let cur = i.action.target().unwrap_or("").to_string();
                let next = match dests.iter().position(|d| d == &cur) {
                    Some(p) => dests[(p + 1) % dests.len()].clone(),
                    None => dests[0].clone(),
                };
                let pos = match &i.action {
                    Action::OpenOverlay { position, .. } => *position,
                    _ => OverlayPosition::Center,
                };
                i.action = match i.action.kind() {
                    "overlay" => Action::OpenOverlay {
                        overlay: next,
                        position: pos,
                    },
                    "swap" => Action::SwapOverlay { overlay: next },
                    "scroll" => Action::ScrollTo { destination: next },
                    _ => Action::Navigate { destination: next },
                };
            }
            self.editor.set_interactions(id, list);
        }
    }

    fn cycle_pos(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if let Some(i) = list.get_mut(index) {
                if let Action::OpenOverlay {
                    overlay: _,
                    position,
                } = &mut i.action
                {
                    *position = match *position {
                        OverlayPosition::Center => OverlayPosition::TopLeft,
                        OverlayPosition::TopLeft => OverlayPosition::TopRight,
                        OverlayPosition::TopRight => OverlayPosition::BottomLeft,
                        OverlayPosition::BottomLeft => OverlayPosition::BottomRight,
                        OverlayPosition::BottomRight => OverlayPosition::Manual(0.0, 0.0),
                        OverlayPosition::Manual(..) => OverlayPosition::Center,
                    };
                }
            }
            self.editor.set_interactions(id, list);
        }
    }

    fn cycle_anim(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if let Some(i) = list.get_mut(index) {
                i.animation = match i.animation {
                    Animation::Instant => Animation::Dissolve,
                    Animation::Dissolve => Animation::SmartAnimate,
                    Animation::SmartAnimate => Animation::SlideIn,
                    Animation::SlideIn => Animation::MoveIn(Direction::Bottom),
                    Animation::MoveIn(d) => Animation::MoveOut(d),
                    Animation::MoveOut(_) => Animation::SlideOut,
                    Animation::SlideOut => Animation::Instant,
                };
            }
            self.editor.set_interactions(id, list);
        }
    }

    /// First number variable (sorted) — the default SetVar target.
    fn proto_first_number_var(&self) -> Option<String> {
        let mut names: Vec<String> = self.vars.numbers.keys().cloned().collect();
        names.sort();
        names.into_iter().next()
    }

    /// Default "name = value" for a fresh SetVar: first number var + 1,
    /// or count = 0 when the document has no number variables.
    fn proto_setvar_default(&self) -> (String, Expr) {
        match self.proto_first_number_var() {
            Some(v) => (
                v.clone(),
                Expr::Add(Box::new(Expr::Var(v)), Box::new(Expr::num(1.0))),
            ),
            None => ("count".into(), Expr::num(0.0)),
        }
    }

    fn proto_setvar_action(&self) -> Action {
        let (name, value) = self.proto_setvar_default();
        Action::SetVar { name, value }
    }

    /// First defined mode (fallback "dark") for a fresh SetMode.
    fn proto_first_mode(&self) -> String {
        self.vars
            .mode_names()
            .into_iter()
            .next()
            .unwrap_or_else(|| "dark".into())
    }

    /// Default condition for a fresh Cond: first number var >= 2, else 1 == 1.
    fn proto_default_cond(&self) -> Condition {
        let fallback = |_| parse_cond_text("1 == 1").expect("literal condition");
        match self.proto_first_number_var() {
            Some(v) => parse_cond_text(&format!("{v} >= 2")).unwrap_or_else(fallback),
            None => fallback(String::new()),
        }
    }

    /// Next nested (then/else) action kind. `wrap_none`: the else slot can
    /// become None (after Navigate); the then slot wraps to Back instead.
    fn proto_next_nested(&self, cur: &Action, wrap_none: bool) -> Option<Action> {
        if wrap_none && matches!(cur, Action::Navigate { .. }) {
            return None;
        }
        Some(match cur {
            Action::Navigate { .. } => Action::Back,
            Action::Back => self.proto_setvar_action(),
            Action::SetVar { .. } => Action::SetMode {
                mode: self.proto_first_mode(),
            },
            Action::SetMode { .. } => match self.proto_destinations().first() {
                Some(d) => Action::Navigate {
                    destination: d.clone(),
                },
                None => self.proto_setvar_action(),
            },
            _ => self.proto_setvar_action(),
        })
    }

    /// Cycle the nested then (which==0) / else (which==1) action of a Cond.
    fn proto_cycle_nested(&mut self, id: &str, index: usize, which: u8) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if let Some(i) = list.get_mut(index) {
                if let Action::Cond { then, els, .. } = &mut i.action {
                    if which == 0 {
                        let next = self.proto_next_nested(then, false).unwrap_or(Action::Back);
                        **then = next;
                    } else {
                        *els = match els.as_deref() {
                            None => Some(Box::new(self.proto_setvar_action())),
                            Some(a) => self.proto_next_nested(a, true).map(Box::new),
                        };
                    }
                }
            }
            self.editor.set_interactions(id, list);
        }
    }

    /// Dest chip click: edit SetVar / cycle SetMode modes / edit Cond text;
    /// destination-having actions cycle destinations as before.
    fn proto_dest_click(&mut self, id: &str, index: usize) {
        let act = find(&self.editor.root, id)
            .and_then(|n| n.interactions.get(index))
            .map(|i| i.action.clone());
        match act {
            Some(Action::SetVar { name, value }) => {
                self.focus = Focus::Proto {
                    node_id: id.into(),
                    index,
                    field: 0,
                    buffer: format!("{name} = {}", format_expr(&value)),
                };
                self.status = "edit variable (name = expression) — Enter to apply".into();
            }
            Some(Action::SetMode { mode }) => {
                let mut modes = vec!["default".to_string()];
                modes.extend(self.vars.mode_names());
                let next = match modes.iter().position(|m| *m == mode) {
                    Some(p) => modes[(p + 1) % modes.len()].clone(),
                    None => modes[0].clone(),
                };
                if let Some(n) = find(&self.editor.root, id) {
                    let mut list = n.interactions.clone();
                    if let Some(i) = list.get_mut(index) {
                        i.action = Action::SetMode { mode: next };
                    }
                    self.editor.set_interactions(id, list);
                }
            }
            Some(Action::Cond { cond, .. }) => {
                self.focus = Focus::Proto {
                    node_id: id.into(),
                    index,
                    field: 1,
                    buffer: format_cond(&cond),
                };
                self.status = "edit condition (lhs op rhs) — Enter to apply".into();
            }
            _ => self.cycle_dest(id, index),
        }
    }

    /// Pos chip click: overlay position (as before), Cond then-branch
    /// (edit when SetVar, else cycle kind), KeyDown key otherwise.
    fn proto_pos_click(&mut self, id: &str, index: usize) {
        let info = find(&self.editor.root, id)
            .and_then(|n| n.interactions.get(index))
            .map(|i| (i.trigger.clone(), i.action.clone()));
        let Some((trigger, action)) = info else {
            return;
        };
        match &action {
            Action::OpenOverlay { .. } => self.cycle_pos(id, index),
            Action::Cond { then, .. } => {
                if let Action::SetVar { name, value } = &**then {
                    self.focus = Focus::Proto {
                        node_id: id.into(),
                        index,
                        field: 3,
                        buffer: format!("{name} = {}", format_expr(value)),
                    };
                    self.status = "edit then-branch variable — Enter to apply".into();
                } else {
                    self.proto_cycle_nested(id, index, 0);
                }
            }
            _ => {
                if let Trigger::KeyDown { key } = &trigger {
                    self.focus = Focus::Proto {
                        node_id: id.into(),
                        index,
                        field: 2,
                        buffer: key.clone(),
                    };
                    self.status = "edit key — Enter to apply".into();
                }
            }
        }
    }

    /// Extra (third-line) chip click: Cond else-branch (edit when SetVar,
    /// else cycle None -> SetVar -> SetMode -> Back -> Navigate -> None),
    /// KeyDown key when an overlay occupies the pos chip.
    fn proto_extra_click(&mut self, id: &str, index: usize) {
        let info = find(&self.editor.root, id)
            .and_then(|n| n.interactions.get(index))
            .map(|i| (i.trigger.clone(), i.action.clone()));
        let Some((trigger, action)) = info else {
            return;
        };
        match &action {
            Action::Cond { els, .. } => match els.as_deref() {
                Some(Action::SetVar { name, value }) => {
                    self.focus = Focus::Proto {
                        node_id: id.into(),
                        index,
                        field: 4,
                        buffer: format!("{name} = {}", format_expr(value)),
                    };
                    self.status = "edit else-branch variable — Enter to apply".into();
                }
                _ => self.proto_cycle_nested(id, index, 1),
            },
            _ => {
                if let Trigger::KeyDown { key } = &trigger {
                    self.focus = Focus::Proto {
                        node_id: id.into(),
                        index,
                        field: 2,
                        buffer: key.clone(),
                    };
                    self.status = "edit key — Enter to apply".into();
                }
            }
        }
    }

    /// Apply a Focus::Proto text edit (Enter). Parse errors keep the old
    /// value and surface in the status bar.
    fn commit_proto_edit(&mut self, id: &str, index: usize, field: u8, buffer: &str) {
        enum ProtoEdit {
            SetVar(String, Expr),
            Expr(Expr),
            Cond(Condition),
            Key(String),
        }
        let text = buffer.trim();
        if text.is_empty() {
            self.status = "edit cancelled (kept old value)".into();
            return;
        }
        let is_name = |s: &str| {
            let mut cs = s.chars();
            matches!(cs.next(), Some(c) if c.is_alphabetic() || c == '_')
                && cs.all(|c| c.is_alphanumeric() || c == '_')
        };
        let parsed = match field {
            0 | 3 | 4 => match text.split_once('=') {
                // "name = expr" (lhs must be a bare identifier)
                Some((l, r)) if is_name(l.trim()) => {
                    parse_expr_text(r.trim()).map(|e| ProtoEdit::SetVar(l.trim().to_string(), e))
                }
                // bare expression: keep the old variable name
                _ => parse_expr_text(text).map(ProtoEdit::Expr),
            },
            1 => parse_cond_text(text).map(ProtoEdit::Cond),
            _ => Ok(ProtoEdit::Key(text.to_string())),
        };
        let edit = match parsed {
            Ok(e) => e,
            Err(e) => {
                self.status = format!("parse error: {e}");
                return;
            }
        };
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            let apply = |a: &mut Action, e: &ProtoEdit| match (&mut *a, e) {
                (Action::SetVar { name, value }, ProtoEdit::SetVar(n2, ex)) => {
                    *name = n2.clone();
                    *value = ex.clone();
                    true
                }
                (Action::SetVar { value, .. }, ProtoEdit::Expr(ex)) => {
                    *value = ex.clone();
                    true
                }
                _ => false,
            };
            let ok = if let Some(i) = list.get_mut(index) {
                match field {
                    0 => apply(&mut i.action, &edit),
                    1 => match (&mut i.action, &edit) {
                        (Action::Cond { cond, .. }, ProtoEdit::Cond(c)) => {
                            *cond = c.clone();
                            true
                        }
                        _ => false,
                    },
                    2 => match (&mut i.trigger, &edit) {
                        (Trigger::KeyDown { key }, ProtoEdit::Key(k)) => {
                            *key = k.clone();
                            true
                        }
                        _ => false,
                    },
                    3 => match &mut i.action {
                        Action::Cond { then, .. } => apply(then, &edit),
                        _ => false,
                    },
                    _ => match &mut i.action {
                        Action::Cond {
                            els: Some(inner), ..
                        } => apply(inner, &edit),
                        _ => false,
                    },
                }
            } else {
                false
            };
            if ok {
                self.editor.set_interactions(id, list);
                self.status = match (&edit, field) {
                    (ProtoEdit::SetVar(n, e), 0) => format!("set {n} = {}", format_expr(e)),
                    (ProtoEdit::SetVar(n, e), 3) => format!("then: set {n} = {}", format_expr(e)),
                    (ProtoEdit::SetVar(n, e), _) => format!("else: set {n} = {}", format_expr(e)),
                    (ProtoEdit::Expr(_), 0) => "value updated".into(),
                    (ProtoEdit::Expr(_), 3) => "then: value updated".into(),
                    (ProtoEdit::Expr(_), _) => "else: value updated".into(),
                    (ProtoEdit::Cond(c), _) => format!("condition: {}", format_cond(c)),
                    (ProtoEdit::Key(k), _) => format!("key: {k}"),
                };
            } else {
                self.status = "row changed — edit cancelled".into();
            }
        }
    }

    fn add_interaction(&mut self, id: &str) {
        let dest = self
            .proto_destinations()
            .first()
            .cloned()
            .unwrap_or_default();
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            list.push(Interaction::click(&dest));
            self.editor.set_interactions(id, list);
        }
    }

    fn remove_interaction(&mut self, id: &str, index: usize) {
        if let Some(n) = find(&self.editor.root, id) {
            let mut list = n.interactions.clone();
            if index < list.len() {
                list.remove(index);
            }
            self.editor.set_interactions(id, list);
        }
    }

    pub fn click_bottom_bar(&mut self, p: Point) {
        let bar = self.bottom_bar_rect();
        let idx = ((p.x - bar.x0 - 8.0) / 38.0).floor();
        if idx >= 0.0 && (idx as usize) < Tool::ALL.len() {
            self.tool = Tool::ALL[idx as usize];
            self.status = format!("tool: {:?}", self.tool);
        }
    }

    pub fn mouse_move(&mut self, p: Point) {
        // presentation mode: hover/enter/leave triggers
        if self.present.is_some() {
            self.cursor = p;
            self.present_hover(p);
            return;
        }
        // brush stroke: collect points + speed-driven widths
        if self.drag == Drag::Brush {
            self.cursor = p;
            let wp = self.world_point(p);
            let min_step = 1.5 / self.zoom.max(0.05);
            if let Some((lx, ly)) = self.brush_pts.last() {
                let d = (lx - wp.x).hypot(ly - wp.y);
                if d > min_step {
                    // speed in screen px/event: faster = thinner
                    let speed = (d * self.zoom).min(12.0);
                    let target = BRUSH_WMAX - (BRUSH_WMAX - BRUSH_WMIN) * (speed / 12.0);
                    let prev = *self.brush_w.last().unwrap_or(&BRUSH_WMAX);
                    let w = prev * 0.6 + target * 0.4;
                    self.brush_pts.push((wp.x, wp.y));
                    self.brush_w.push(w);
                }
            }
            return;
        }
        // pencil stroke: collect world points (screen-px spacing)
        if self.drag == Drag::Pencil {
            self.cursor = p;
            let wp = self.world_point(p);
            let min_step = 1.5 / self.zoom.max(0.05);
            if let Some((lx, ly)) = self.pencil_pts.last() {
                if (lx - wp.x).hypot(ly - wp.y) > min_step {
                    self.pencil_pts.push((wp.x, wp.y));
                }
            }
            return;
        }
        // vector eraser: accumulate the segments swept by the cursor
        if self.drag == Drag::Erase {
            self.cursor = p;
            if let Some(vid) = self.node_edit.clone() {
                let wp = self.world_point(p);
                if let Some(n) = find(&self.editor.root, &vid) {
                    if let x_native::NodeKind::Vector { path } = &n.kind {
                        let local = (wp.x - n.transform.x, wp.y - n.transform.y);
                        if let Some(si) =
                            x_native::editor::segment_at(path, local.0, local.1, 8.0 / self.zoom)
                        {
                            if !self.eraser_hits.contains(&si) {
                                self.eraser_hits.push(si);
                            }
                        }
                    }
                }
            }
            return;
        }
        // Figma-style pen tool: a drag right after placing an anchor pulls        // a bezier handle out of it instead of leaving a plain corner.
        if let (Tool::Pen, Some((idx, _, _))) = (self.tool, self.pen_placing) {
            self.cursor = p;
            if let Some(id) = self.pen_target.clone() {
                let wp = self.world_point(p);
                if let Some(n) = find(&self.editor.root, &id) {
                    if let x_native::NodeKind::Vector { path } = &n.kind {
                        if let Some(a) = x_native::editor::anchors(path).get(idx).copied() {
                            let (dx, dy) = (wp.x - a.x, wp.y - a.y);
                            if idx > 0 {
                                self.editor.pen_shape_incoming(&id, idx, dx, dy);
                            }
                            self.pen_pending_out = if dx.abs() > 0.01 || dy.abs() > 0.01 {
                                Some((dx, dy))
                            } else {
                                None
                            };
                            self.status = "pen: dragging curve handle — release to continue".into();
                        }
                    }
                }
            }
            return;
        }
        if let Drag::Gradient { fill, handle, cmds } = self.drag {
            let mut active_handle = handle;
            if let Some(id) = self.editor.selection.first().cloned() {
                if let Some((world, _, _)) = world_transform_of(&self.editor.root, &id) {
                    let local = (self.camera() * world).inverse() * p;
                    self.editor.mutate_visual_stack(&id, |n| {
                        if let Some(layer) = n.fill_layers.get_mut(fill) {
                            match &mut layer.paint {
                                Paint::LinearGradient {
                                    start, end, stops, ..
                                } => match handle {
                                    0 => *start = (local.x, local.y),
                                    1 => *end = (local.x, local.y),
                                    h => {
                                        let vx = end.0 - start.0;
                                        let vy = end.1 - start.1;
                                        let len2 = vx * vx + vy * vy;
                                        if len2 > 0.0001 && h - 2 < stops.len() {
                                            let t = (((local.x - start.0) * vx
                                                + (local.y - start.1) * vy)
                                                / len2)
                                                .clamp(0.0, 1.0)
                                                as f32;
                                            stops[h - 2].0 = t;
                                            stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                                            if let Some(i) = stops
                                                .iter()
                                                .position(|(p, _)| (*p - t).abs() < f32::EPSILON)
                                            {
                                                active_handle = i + 2;
                                            }
                                        }
                                    }
                                },
                                Paint::RadialGradient {
                                    center,
                                    radius,
                                    stops,
                                    ..
                                } => match handle {
                                    0 => *center = (local.x, local.y),
                                    1 => *radius = (local.x - center.0).hypot(local.y - center.1),
                                    h if h - 2 < stops.len() => {
                                        let r = (*radius).max(0.0001);
                                        stops[h - 2].0 =
                                            ((local.x - center.0).hypot(local.y - center.1) / r)
                                                .clamp(0.0, 1.0)
                                                as f32;
                                        let t = stops[h - 2].0;
                                        stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                                        if let Some(i) = stops
                                            .iter()
                                            .position(|(p, _)| (*p - t).abs() < f32::EPSILON)
                                        {
                                            active_handle = i + 2;
                                        }
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                    });
                }
            }
            if active_handle >= 2 {
                self.gradient_stop = active_handle - 2;
            }
            self.drag = Drag::Gradient {
                fill,
                handle: active_handle,
                cmds,
            };
        } else if let Drag::Move { start, .. } = self.drag {
            let d = (p - start) / self.zoom;
            if d.x != 0.0 || d.y != 0.0 {
                // Alt+drag = duplicate, then move the copy
                if self.alt && !self.alt_dupe_done {
                    self.alt_dupe_done = true;
                    let ids = self.editor.duplicate_selection((0.0, 0.0));
                    self.status = format!("alt-duplicated {}", ids.join(", "));
                }
                self.editor.move_selection(d.x.round(), d.y.round());
                // magnetic snap: pull edges/centers onto neighbors
                if self.editor.selection.len() == 1 {
                    let id = self.editor.selection[0].clone();
                    let (sx, sy) =
                        x_native::editor::snap_delta(&self.editor.root, &id, 4.0 / self.zoom);
                    if sx != 0.0 || sy != 0.0 {
                        self.editor.move_selection(sx, sy);
                    }
                    self.guides = x_native::editor::alignment_guides(&self.editor.root, &id, 1.0);
                } else {
                    self.guides = vec![];
                }
                self.drag = match self.drag {
                    Drag::Move { cmds, .. } => Drag::Move { start: p, cmds },
                    d => d,
                };
            }
        } else if let Drag::Resize {
            corner,
            start_world,
            orig,
            cmds,
        } = self.drag
        {
            let wp = self.world_point(p);
            let (dx, dy) = (wp.x - start_world.x, wp.y - start_world.y);
            let (x, y, w, h) = orig;
            let id = self.editor.selection[0].clone();
            // corner: 0 TL, 1 TR, 2 BL, 3 BR
            let (mut nw, mut nh) = match corner {
                0 => (w - dx, h - dy),
                1 => (w + dx, h - dy),
                2 => (w - dx, h + dy),
                3 => (w + dx, h + dy),
                4 => (w - dx, h), // left edge
                5 => (w + dx, h), // right edge
                6 => (w, h - dy), // top edge
                _ => (w, h + dy), // bottom edge
            };
            // Shift = lock aspect ratio to the original w:h (corners only)
            if self.shift && corner < 4 && w > 0.0 && h > 0.0 {
                let ratio = w / h;
                if (nw / w).abs() > (nh / h).abs() {
                    nh = nw / ratio;
                } else {
                    nw = nh * ratio;
                }
            }
            // Alt = resize from the center, growing/shrinking both sides
            // equally instead of anchoring the opposite edge.
            if self.alt {
                nw = w + (nw - w) * 2.0;
                nh = h + (nh - h) * 2.0;
            }
            self.editor.resize(&id, nw.max(2.0), nh.max(2.0));
            if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                if self.alt {
                    // keep the shape centered on its original center
                    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                    n.transform.x = cx - nw.max(2.0) / 2.0;
                    n.transform.y = cy - nh.max(2.0) / 2.0;
                } else {
                    // opposite corner stays fixed
                    match corner {
                        0 => {
                            n.transform.x = x + dx;
                            n.transform.y = y + dy;
                        }
                        1 => {
                            n.transform.y = y + dy;
                        }
                        2 => {
                            n.transform.x = x + dx;
                        }
                        4 => {
                            n.transform.x = x + dx;
                        }
                        6 => {
                            n.transform.y = y + dy;
                        }
                        _ => {}
                    }
                }
            }
            self.drag = Drag::Resize {
                corner,
                start_world,
                orig,
                cmds,
            };
        } else if let Drag::Radius {
            corner,
            uniform,
            start_world,
            orig,
            cmds,
        } = self.drag
        {
            let wp = self.world_point(p);
            // drag distance along the diagonal (positive = bigger radius)
            let delta = ((wp.x - start_world.x) + (wp.y - start_world.y)) * 0.5;
            let (radius, corners) = orig;
            let id = self.editor.selection[0].clone();
            if uniform {
                // Figma default: every corner shares one radius
                let nr = (radius + delta).max(0.0);
                self.editor.set_corners(&id, nr, None);
            } else {
                // per-corner override: promote then nudge just this corner
                let c = corners.unwrap_or([radius; 4]);
                let mut c2 = c;
                c2[corner as usize] = (c[corner as usize] + delta).max(0.0);
                self.editor.set_corners(&id, radius, Some(c2));
            }
            self.drag = Drag::Radius {
                corner,
                uniform,
                start_world,
                orig,
                cmds,
            };
        } else if let Some((ai, outgoing, _)) = self.handle_drag {
            if let Some(vid) = self.node_edit.clone() {
                let wp = self.world_point(p);
                if let Some(n) = find(&self.editor.root, &vid) {
                    let local = (wp.x - n.transform.x, wp.y - n.transform.y);
                    // Alt breaks the tangent (independent handle); the
                    // default drag mirrors the opposite handle, same as
                    // Figma/Illustrator's smooth-point behavior.
                    self.editor
                        .move_handle(&vid, ai, outgoing, local.0, local.1, !self.alt);
                }
            }
            self.cursor = p;
            return;
        } else if let Some((ai, _)) = self.anchor_drag {
            if let Some(vid) = self.node_edit.clone() {
                let wp = self.world_point(p);
                if let Some(n) = find(&self.editor.root, &vid) {
                    let local = (wp.x - n.transform.x, wp.y - n.transform.y);
                    self.editor.move_anchor(&vid, ai, local.0, local.1);
                }
            }
            self.cursor = p;
            return;
        } else if let Drag::Pan { start } = self.drag {
            self.pan.0 += p.x - start.x;
            self.pan.1 += p.y - start.y;
            self.drag = Drag::Pan { start: p };
        } else if let Drag::Scale {
            start_y,
            applied,
            cmds,
        } = self.drag
        {
            // target factor from total drag distance: 200px up = +100%
            let target = (1.0 - (p.y - start_y) / 200.0).clamp(0.2, 5.0);
            let step = target / applied;
            if (step - 1.0).abs() > 0.01 {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.scale_node(&id, step);
                    self.drag = Drag::Scale {
                        start_y,
                        applied: target,
                        cmds,
                    };
                    self.status = format!("scale {:.0}%", target * 100.0);
                }
            }
        } else if self.drag == Drag::None && self.tool == Tool::Select && self.present.is_none() {
            // hover highlight (only inside canvas, not over chrome). Collapse
            // groups like a plain click so the outline matches what a click
            // would select (Figma shows the hover outline on the same object).
            self.hover = if self.canvas_rect().contains(p) {
                x_native::editor::hit_test(&self.editor.root, self.world_point(p))
                    .map(|id| {
                        x_native::editor::nearest_group_ancestor(&self.editor.root, &id)
                            .unwrap_or(id)
                    })
                    .filter(|id| !self.editor.selection.contains(id))
            } else {
                None
            };
        }
        if let Drag::Rotate {
            center,
            start_angle,
            orig,
            cmds,
        } = self.drag
        {
            let a = (p.y - center.y).atan2(p.x - center.x);
            let mut angle = orig + (a - start_angle);
            if self.shift {
                // snap to 15° steps
                let step = 15f64.to_radians();
                angle = (angle / step).round() * step;
            }
            if let Some(id) = self.editor.selection.first().cloned() {
                self.editor.rotate(&id, angle);
            }
            self.drag = Drag::Rotate {
                center,
                start_angle,
                orig,
                cmds,
            };
        }
        self.cursor = p;
    }

    pub fn mouse_up(&mut self, p: Point) {
        // presentation mode: OnClick fires on release (Figma "on click")
        if self.present.is_some() {
            self.present_click(p);
            return;
        }
        // text drag-select ends on release (the selected range stays)
        if let Drag::TextSelect { .. } = self.drag {
            self.drag = Drag::None;
            return;
        }
        // Figma-style pen tool: releasing after a curve-handle drag merges        // the drag's incremental commands into the anchor's placement step,
        // and keeps pen_pending_out so the NEXT anchor inherits the tangent.
        if let Some((_, _, depth)) = self.pen_placing.take() {
            let n = self.editor.undo_depth().saturating_sub(depth);
            self.editor.merge_last(n);
            self.status =
                "pen: click to add anchors, drag to curve, click start to close, Esc to finish"
                    .into();
            return;
        }
        // asset drag-to-canvas: release OUTSIDE the browser panel drops the
        // dragged asset as a new image node at the cursor's world position
        if let Some(aid) = self.asset_drag.take() {
            // armed from the Shift+A browser OR the left Assets tab; drop on
            // the canvas creates the image node at the cursor's world point
            let from_browser = self.asset_browser && !self.asset_panel_rect().contains(p);
            let from_left_tab = !self.asset_browser && self.left_tab == 1;
            if (from_browser || from_left_tab) && p.x > LAYERS_W && p.x < self.win_w - INSPECTOR_W {
                let wp = self.world_point(p);
                let dims = self
                    .store
                    .get(&aid)
                    .and_then(|r| r.dimensions)
                    .unwrap_or((160, 120));
                self.created_count += 1;
                let nid = format!("image-{}", self.created_count);
                let mut n = Node::image(&nid, wp.x, wp.y, dims.0 as f64, dims.1 as f64, &aid);
                n.transform.x = wp.x;
                n.transform.y = wp.y;
                let root_id = self.editor.root.id.clone();
                self.editor.insert_node(&root_id, n);
                self.editor.selection = vec![nid.clone()];
                self.asset_browser = false;
                self.status = format!("dropped {nid} at cursor");
                return;
            }
        }
        if let Some((_, _, depth)) = self.handle_drag.take() {
            let n = self.editor.undo_depth().saturating_sub(depth);
            self.editor.merge_last(n);
            let _ = p;
            return;
        }
        if let Some((_, depth)) = self.anchor_drag.take() {
            let n = self.editor.undo_depth().saturating_sub(depth);
            self.editor.merge_last(n);
            let _ = p;
            return;
        }
        self.guides.clear();
        match self.drag {
            Drag::TextSelect { .. } => {} // text selection: caret updates are transient
            Drag::Move { cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
            }
            Drag::Resize { cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
                self.status = "resized".into();
            }
            Drag::Rotate { cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
                if let Some(node) = self.selected_single() {
                    self.status =
                        format!("rotated to {:.0} deg", node.transform.rotation.to_degrees());
                }
            }
            Drag::Gradient { cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
                self.status = "gradient updated".into();
            }
            Drag::Radius { cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
                self.status = "corner radius updated".into();
            }
            Drag::Erase => {
                let hits = std::mem::take(&mut self.eraser_hits);
                if let Some(vid) = self.node_edit.clone() {
                    if !hits.is_empty() && self.editor.erase_segments(&vid, &hits) {
                        self.status = format!("erased {} segment(s)", hits.len());
                    } else {
                        self.status = "eraser: nothing under the drag".into();
                    }
                }
            }
            Drag::Brush => {
                let pts = std::mem::take(&mut self.brush_pts);
                let mut widths = std::mem::take(&mut self.brush_w);
                if pts.len() >= 2 && widths.len() == pts.len() {
                    // taper both ends toward the minimum width
                    if let Some(w) = widths.first_mut() {
                        *w = (*w + BRUSH_WMIN) * 0.5;
                    }
                    if let Some(w) = widths.last_mut() {
                        *w = (*w + BRUSH_WMIN) * 0.5;
                    }
                    let outline = x_native::booleans::stroke_outline_variable(&pts, &widths);
                    let (mut minx, mut miny) = (f64::INFINITY, f64::INFINITY);
                    let (mut maxx, mut maxy) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
                    for c in &outline {
                        if let x_native::PathCmd::MoveTo(x, y) | x_native::PathCmd::LineTo(x, y) = c
                        {
                            minx = minx.min(*x);
                            miny = miny.min(*y);
                            maxx = maxx.max(*x);
                            maxy = maxy.max(*y);
                        }
                    }
                    let cmds: Vec<x_native::PathCmd> = outline
                        .into_iter()
                        .map(|c| match c {
                            x_native::PathCmd::MoveTo(x, y) => {
                                x_native::PathCmd::MoveTo(x - minx, y - miny)
                            }
                            x_native::PathCmd::LineTo(x, y) => {
                                x_native::PathCmd::LineTo(x - minx, y - miny)
                            }
                            other => other,
                        })
                        .collect();
                    self.created_count += 1;
                    let id = format!("brush-{}", self.created_count);
                    let mut v = Node::vector(
                        &id,
                        0.0,
                        0.0,
                        (maxx - minx).max(1.0),
                        (maxy - miny).max(1.0),
                        cmds,
                    );
                    v.transform.x = minx;
                    v.transform.y = miny;
                    v.fill = Paint::Solid(PALETTE[3]);
                    let root_id = self.editor.root.id.clone();
                    self.editor.insert_node(&root_id, v);
                    self.editor.selection = vec![id.clone()];
                    let (wlo, whi) = (
                        widths.iter().cloned().fold(f64::INFINITY, f64::min),
                        widths.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    );
                    self.status = format!(
                        "brush: {} pts, {:.1}-{:.1}px wide -> {id}",
                        pts.len(),
                        wlo,
                        whi
                    );
                } else {
                    self.status = "brush: drag to paint (speed controls width)".into();
                }
            }
            Drag::Pencil => {
                let pts = std::mem::take(&mut self.pencil_pts);
                let eps = 2.0 / self.zoom.max(0.05);
                let simplified = x_native::simplify_polyline(&pts, eps);
                if simplified.len() >= 2 {
                    let minx = simplified.iter().fold(f64::INFINITY, |a, q| a.min(q.0));
                    let miny = simplified.iter().fold(f64::INFINITY, |a, q| a.min(q.1));
                    let maxx = simplified.iter().fold(f64::NEG_INFINITY, |a, q| a.max(q.0));
                    let maxy = simplified.iter().fold(f64::NEG_INFINITY, |a, q| a.max(q.1));
                    let cmds: Vec<x_native::PathCmd> = simplified
                        .iter()
                        .enumerate()
                        .map(|(i, (x, y))| {
                            if i == 0 {
                                x_native::PathCmd::MoveTo(x - minx, y - miny)
                            } else {
                                x_native::PathCmd::LineTo(x - minx, y - miny)
                            }
                        })
                        .collect();
                    self.created_count += 1;
                    let id = format!("sketch-{}", self.created_count);
                    let mut v = Node::vector(
                        &id,
                        0.0,
                        0.0,
                        (maxx - minx).max(1.0),
                        (maxy - miny).max(1.0),
                        cmds,
                    );
                    v.transform.x = minx;
                    v.transform.y = miny;
                    v.fill = Paint::Solid(Color::TRANSPARENT);
                    v.stroke = x_native::Stroke::solid(PALETTE[3], 2.0);
                    let root_id = self.editor.root.id.clone();
                    self.editor.insert_node(&root_id, v);
                    self.editor.selection = vec![id.clone()];
                    self.status = format!(
                        "pencil: {} point(s) simplified from {} -> {id}",
                        simplified.len(),
                        pts.len()
                    );
                } else {
                    self.status = "pencil: drag to sketch".into();
                }
            }
            Drag::Marquee {
                start_world,
                contained,
            } => {
                let wp = self.world_point(p);
                let r = Rect::new(
                    start_world.x.min(wp.x),
                    start_world.y.min(wp.y),
                    start_world.x.max(wp.x),
                    start_world.y.max(wp.y),
                );
                if r.width() > 2.0 && r.height() > 2.0 {
                    if contained {
                        self.editor.marquee_contained(r);
                    } else {
                        self.editor.marquee(r);
                    }
                    self.status = format!(
                        "marquee: {} selected{}",
                        self.editor.selection.len(),
                        if contained { " (fully contained)" } else { "" }
                    );
                }
            }
            Drag::Create { start_world } => {
                let wp = self.world_point(p);
                let r = self.creation_rect(start_world, wp);
                let is_click = r.width() < 3.0 && r.height() < 3.0;
                // Every creation tool places a sensible default-size shape
                // on a plain click — a drag is how you size it, not a
                // requirement to create anything, same as Figma.
                self.created_count += 1;
                let id = format!(
                    "{}-{}",
                    self.tool.label().to_lowercase(),
                    self.created_count
                );
                // A bare click has no dragged box yet — use each tool's
                // Figma-equivalent default size at the click point.
                let (bx, by, bw, bh) = if is_click {
                    match self.tool {
                        Tool::Text => (start_world.x, start_world.y, 100.0, 24.0),
                        Tool::Line => (start_world.x, start_world.y, 100.0, 0.0),
                        _ => (start_world.x, start_world.y, 100.0, 100.0),
                    }
                } else {
                    (r.x0, r.y0, r.width(), r.height())
                };
                let node = match self.tool {
                    Tool::Rectangle => {
                        Node::rect(&id, bx, by, bw, bh, C_ACCENT).radius(self.rect_radius)
                    }
                    Tool::Ellipse => Node::ellipse(&id, bx, by, bw, bh, PALETTE[1]),
                    Tool::Arc => Node::arc(
                        &id,
                        bx,
                        by,
                        bw,
                        bh,
                        self.arc_start,
                        self.arc_end,
                        PALETTE[1],
                    ),
                    Tool::Line => Node::line(&id, bx, by, bw, bh.max(2.0), Color::WHITE),
                    // Starts empty: an un-typed placeholder string would
                    // commit as real content if you clicked away without
                    // typing, which Figma never does.
                    Tool::Text => Node::text(&id, bx, by, bw, bh.clamp(12.0, 64.0), ""),
                    Tool::Polygon => {
                        let mut n = Node::vector(
                            &id,
                            0.0,
                            0.0,
                            bw,
                            bh,
                            regular_polygon(self.polygon_sides, bw, bh),
                        );
                        n.transform.x = bx;
                        n.transform.y = by;
                        n.fill = Paint::Solid(PALETTE[2]);
                        n
                    }
                    Tool::Star => {
                        let mut n = Node::vector(
                            &id,
                            0.0,
                            0.0,
                            bw,
                            bh,
                            star_path_with_ratio(self.star_points, bw, bh, self.star_inner_ratio),
                        );
                        n.transform.x = bx;
                        n.transform.y = by;
                        n.fill = Paint::Solid(PALETTE[4]);
                        n
                    }
                    Tool::Frame
                    | Tool::Select
                    | Tool::Hand
                    | Tool::Scale
                    | Tool::Pen
                    | Tool::Eyedropper
                    | Tool::Pencil
                    | Tool::Bucket
                    | Tool::Brush => {
                        let mut f = Node::frame(&id, bw, bh);
                        f.transform.x = bx;
                        f.transform.y = by;
                        f.fill = Paint::Solid(Color::from_rgb8(0x38, 0x38, 0x38));
                        f
                    }
                    Tool::Slice => Node::slice(&id, bx, by, bw, bh),
                };
                let created_tool = self.tool;
                let root_id = self.editor.root.id.clone();
                self.editor.insert_node(&root_id, node);
                // text nodes: store their font-accurate baseline offset.
                self.refresh_text_baseline(&id);
                self.editor.selection = vec![id.clone()];
                self.rebuild_layer_rows();
                self.tool = Tool::Select;
                if created_tool == Tool::Text {
                    // Drop straight into typing, like Figma — no
                    // separate double-click is needed after creation.
                    self.focus = Focus::TextNode {
                        id: id.clone(),
                        buffer: String::new(),
                        original: String::new(),
                        caret: 0,
                        sel_anchor: None,
                    };
                    self.status =
                        "editing text — Enter/Esc commits, empty text is discarded".into();
                } else {
                    self.status = format!("created {id}");
                }
            }
            Drag::Pan { .. } => {}
            Drag::Scale { applied, cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
                if (applied - 1.0).abs() > 0.001 {
                    self.status = format!("scaled to {:.0}%", applied * 100.0);
                }
            }
            Drag::None => {}
        }
        self.drag = Drag::None;
    }

    pub fn click_left_sidebar(&mut self, p: Point) {
        // icon tab row: Layers / Assets / Components / Library (REAL tabs)
        if p.y >= TOP_H && p.y <= TOP_H + LTAB_H {
            for (i, r) in self.left_tab_rects() {
                if r.contains(p) {
                    self.left_tab = i as u8;
                    self.status = format!("{} panel", LEFT_TABS[i]);
                    return;
                }
            }
        }
        // non-Layers tabs: geometry shared with the painter
        if self.left_tab != 0 {
            for (tag, r, kind) in self.left_panel_layout() {
                if !r.contains(p) {
                    continue;
                }
                match kind {
                    1 => {
                        // asset tile: select + arm drag-to-canvas (same
                        // semantics as the Shift+A browser tiles)
                        self.asset_sel = Some(tag.clone());
                        self.asset_drag = Some(tag.clone());
                        let rec = self.store.get(&tag);
                        self.status = match rec {
                            Some(rec) => {
                                format!("{} | {} — drag to canvas to place", rec.name, rec.mime)
                            }
                            None => "asset missing".into(),
                        };
                    }
                    2 => {
                        self.status = format!("click canvas to place {tag}");
                        self.stamping = Some(tag);
                    }
                    3 => {
                        if let Some((lib, comp)) = tag.split_once('|') {
                            let (lib, comp) = (lib.to_string(), comp.to_string());
                            self.place_library_component(&lib, &comp);
                        }
                    }
                    4 => {
                        self.inspector_tab = 3;
                        self.status = "library manager (LIBS tab)".into();
                    }
                    _ => {}
                }
                return;
            }
            return;
        }
        // search box (mockup: top of the panel)
        if p.y >= TOP_H + LSEARCH_Y0
            && p.y <= TOP_H + LSEARCH_Y1
            && p.x >= 10.0
            && p.x <= LAYERS_W - 10.0
        {
            self.focus = Focus::LayerSearch;
            self.status = "type to filter layers".into();
            return;
        }
        // PAGES section (below icon tabs + search)
        let pages_y0 = TOP_H + LPAGES_Y0;
        let pages_end = pages_y0 + self.pages.len() as f64 * ROW_H;
        if p.y >= pages_y0 && p.y < pages_end {
            let idx = ((p.y - pages_y0) / ROW_H) as usize;
            if idx < self.pages.len() {
                if self.dbl && idx == self.page_idx {
                    self.start_page_rename(idx);
                    return;
                }
                self.switch_page(idx);
            }
            return;
        }
        // "+ new page" row
        if p.y >= pages_end && p.y < pages_end + ROW_H {
            self.add_page();
            return;
        }
        self.click_layers(p);
    }

    pub fn click_layers(&mut self, p: Point) {
        // search box next to the LAYERS header (position depends on pages count)
        let header_y = TOP_H + LPAGES_Y0 + (self.pages.len() as f64 + 1.0) * ROW_H + 6.0;
        if p.y >= header_y - 6.0 && p.y <= header_y + 14.0 && p.x > 70.0 {
            self.focus = Focus::LayerSearch;
            self.status = "type to filter layers, Enter/Esc done".into();
            return;
        }
        // ASSETS section: bottom of the panel, one row per component
        let comps = self.editor.component_names();
        if !comps.is_empty() {
            let assets_y = self.win_h - 30.0 - comps.len() as f64 * ROW_H;
            if p.y >= assets_y {
                let idx = ((p.y - assets_y) / ROW_H).floor();
                if idx >= 0.0 && (idx as usize) < comps.len() {
                    let name = comps[idx as usize].clone();
                    self.status = format!("click canvas to place {name}");
                    self.stamping = Some(name);
                    return;
                }
            }
        }
        let layers_list_y = TOP_H + LPAGES_Y0 + (self.pages.len() as f64 + 1.0) * ROW_H + 26.0;
        let idx = ((p.y - layers_list_y) / ROW_H).floor();
        let idx = if idx >= 0.0 {
            idx as usize + self.layers_scroll
        } else {
            return;
        };
        if idx < self.layer_rows.len() {
            let id = self.layer_rows[idx].0.clone();
            let name = self.layer_rows[idx].1.clone();
            if self.dbl && p.x < LAYERS_W - 40.0 {
                self.focus = Focus::LayerRename {
                    id: id.clone(),
                    buffer: name.clone(),
                };
                self.status = format!("rename layer: {name}");
                return;
            }
            // eye / lock click zones (right side of the row)
            if p.x >= LAYERS_W - 40.0 && p.x < LAYERS_W - 24.0 {
                if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                    n.visible = !n.visible;
                    self.status = format!("{} {}", id, if n.visible { "shown" } else { "hidden" });
                }
                return;
            }
            if p.x >= LAYERS_W - 24.0 {
                if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                    n.locked = !n.locked;
                    self.status =
                        format!("{} {}", id, if n.locked { "locked" } else { "unlocked" });
                }
                return;
            }
            if self.shift {
                if let Some(i) = self.editor.selection.iter().position(|s| s == &id) {
                    self.editor.selection.remove(i);
                } else {
                    self.editor.selection.push(id.clone());
                }
            } else {
                self.editor.selection = vec![id.clone()];
            }
            self.status = format!("layer: {id}");
        }
    }

    /// mockup tab strip: Design | Prototype | Inspect (Vars/Libs stay
    /// reachable via View menu + left Library tab). Shared painter/click.
    pub fn inspector_tabs(&self) -> Vec<(&'static str, u8, Rect)> {
        let ix = self.win_w - INSPECTOR_W;
        let mut out = vec![];
        let mut x = ix + 12.0;
        for (name, idx) in [("Design", 0u8), ("Prototype", 1), ("Inspect", 4)] {
            let w = ui_measure(name, 8.5) + 12.0;
            out.push((name, idx, Rect::new(x, TOP_H + 4.0, x + w, TOP_H + 24.0)));
            x += w + 6.0;
        }
        out
    }

    pub fn click_inspector(&mut self, p: Point) {
        // Design/Prototype/Inspect tab switch (shared rects)
        let ix = self.win_w - INSPECTOR_W;
        if p.y >= TOP_H + 4.0 && p.y <= TOP_H + 24.0 {
            for (name, idx, r) in self.inspector_tabs() {
                if r.contains(p) {
                    self.inspector_tab = idx;
                    self.status = format!("{} tab", name.to_ascii_lowercase());
                    return;
                }
            }
        }
        // Creation-tool defaults. Painter and hit zones use the same rows.
        if self.selected_single().is_none()
            && matches!(
                self.tool,
                Tool::Rectangle | Tool::Polygon | Tool::Star | Tool::Arc
            )
        {
            let row = ((p.y - (TOP_H + 58.0)) / 30.0).floor() as isize;
            let ix = self.win_w - INSPECTOR_W;
            let delta = if p.x >= ix + 212.0 && p.x <= ix + 230.0 {
                -1.0
            } else if p.x >= ix + 232.0 && p.x <= ix + 250.0 {
                1.0
            } else {
                0.0
            };
            if delta != 0.0 {
                match (self.tool, row) {
                    (Tool::Rectangle, 0) => {
                        self.rect_radius = (self.rect_radius + delta * 2.0).clamp(0.0, 100.0)
                    }
                    (Tool::Polygon, 0) => {
                        self.polygon_sides =
                            ((self.polygon_sides as isize + delta as isize).clamp(3, 60)) as usize
                    }
                    (Tool::Star, 0) => {
                        self.star_points =
                            ((self.star_points as isize + delta as isize).clamp(3, 60)) as usize
                    }
                    (Tool::Star, 1) => {
                        self.star_inner_ratio =
                            (self.star_inner_ratio + delta * 0.05).clamp(0.05, 0.95)
                    }
                    (Tool::Arc, 0) => {
                        self.arc_start = (self.arc_start + delta * 15.0 + 360.0) % 360.0;
                    }
                    (Tool::Arc, 1) => {
                        self.arc_end = (self.arc_end + delta * 15.0) % 360.0;
                    }
                    _ => return,
                }
                self.status = "tool defaults updated".into();
                return;
            }
        }
        // Export section buttons (Design tab, mockup)
        if self.inspector_tab == 0 {
            // per-node export-settings panel (a single layer is selected)
            if let Some((id, rows, add, export_btn, _top)) = self.export_settings_layout() {
                if export_btn.contains(p) {
                    self.export_selected_settings();
                    return;
                }
                if add.contains(p) {
                    self.add_export_setting(&id);
                    return;
                }
                for (i, fmt, scale, remove) in &rows {
                    if remove.contains(p) {
                        self.remove_export_setting(&id, *i);
                        return;
                    }
                    if fmt.contains(p) {
                        self.cycle_export_format(&id, *i);
                        return;
                    }
                    if scale.contains(p) {
                        self.cycle_export_scale(&id, *i);
                        return;
                    }
                }
            }
            for (l, tag, r) in self.export_layout() {
                if r.contains(p) {
                    let (l, tag) = (l.to_string(), tag.to_string());
                    let t0 = std::time::Instant::now();
                    self.run_menu_tag(&tag);
                    self.last_cmd =
                        Some((format!("export {l}"), t0.elapsed().as_secs_f32() * 1000.0));
                    return;
                }
            }
        }
        // LIBRARIES tab interactions — SHARED layout with the painter
        if self.inspector_tab == 3 {
            for (tag, r, kind) in self.libs_layout() {
                if !r.contains(p) {
                    continue;
                }
                match kind {
                    1 => {
                        self.link_library();
                        return;
                    }
                    2 => {
                        self.check_library_updates();
                        return;
                    }
                    3 => {
                        self.library_review = true;
                        self.status = "review the changes, then Accept or Cancel".into();
                        return;
                    }
                    4 => {
                        if let Some((lib, comp)) = tag.split_once('|') {
                            let (lib, comp) = (lib.to_string(), comp.to_string());
                            self.place_library_component(&lib, &comp);
                        }
                        return;
                    }
                    6 => {
                        self.focus = Focus::LibSearch;
                        self.status = "type to filter components, styles, variables".into();
                        return;
                    }
                    7 => {
                        self.publish_library();
                        return;
                    }
                    8 => {
                        // tag: lib|sty|style name (names may contain '/')
                        let mut it = tag.splitn(3, '|');
                        if let (Some(lib), Some(_), Some(name)) = (it.next(), it.next(), it.next())
                        {
                            let (lib, name) = (lib.to_string(), name.to_string());
                            self.apply_library_style(&lib, &name);
                        }
                        return;
                    }
                    _ => {}
                }
            }
            return;
        }
        // frame presets (Frame tool active, nothing selected)
        if self.selected_single().is_none() && self.tool == Tool::Frame {
            let ix = self.win_w - INSPECTOR_W;
            for (i, (name, w, h)) in FRAME_PRESETS.iter().enumerate() {
                let y = TOP_H + 50.0 + i as f64 * 24.0;
                if p.x >= ix + 12.0 && p.x <= ix + INSPECTOR_W - 24.0 && p.y >= y && p.y <= y + 19.0
                {
                    self.created_count += 1;
                    let id = format!("frame-{}", self.created_count);
                    let wp = self.world_point(Point::new(
                        self.canvas_rect().x0 + 60.0,
                        self.canvas_rect().y0 + 60.0,
                    ));
                    let mut f = Node::frame(&id, *w, *h);
                    f.transform.x = wp.x.max(0.0);
                    f.transform.y = wp.y.max(0.0);
                    f.fill = Paint::Solid(Color::from_rgb8(0xff, 0xff, 0xff));
                    let root_id = self.editor.root.id.clone();
                    self.editor.insert_node(&root_id, f);
                    self.editor.selection = vec![id.clone()];
                    self.status = format!("created {id} ({name})");
                    self.tool = Tool::Select;
                    return;
                }
            }
        }
        let x0 = self.win_w - INSPECTOR_W + 12.0;
        // numeric fields: X Y (row y=TOP_H+66) and W H (row y=TOP_H+84),
        // matching inspector line positions; click one to type a new value.
        if let Some(n) = self.selected_single() {
            let id = n.id.clone();
            let _vals = [n.transform.x, n.transform.y, n.w, n.h];
            let rows = [
                (0u8, TOP_H + IY_XY),
                (1, TOP_H + IY_XY),
                (2, TOP_H + IY_WH),
                (3, TOP_H + IY_WH),
            ];
            for (field, ry) in rows {
                let fx = x0 + if field % 2 == 0 { 0.0 } else { 108.0 };
                if p.x >= fx - 2.0 && p.x <= fx + 100.0 && p.y >= ry - 3.0 && p.y <= ry + 14.0 {
                    // polish: select-all semantics — buffer starts EMPTY so
                    // typing REPLACES the value; Enter empty = keep old
                    self.focus = Focus::Field {
                        id,
                        field,
                        buffer: String::new(),
                    };
                    self.status = format!(
                        "type new {} (Enter commits, Tab next, Esc cancels)",
                        ["X", "Y", "W", "H"][field as usize]
                    );
                    return;
                }
            }
        }
        // skew steppers + 9-point transform-origin (Design tab, single select)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                let id = n.id.clone();
                // skew ∠X / ∠Y steppers
                let fy = TOP_H + IY_SKEW;
                if p.y >= fy - 3.0 && p.y <= fy + 14.0 {
                    for col in 0..2usize {
                        let fx = ix + 10.0 + col as f64 * 108.0;
                        let delta = if p.x >= fx + 64.0 && p.x <= fx + 80.0 {
                            -5.0f64.to_radians()
                        } else if p.x >= fx + 82.0 && p.x <= fx + 98.0 {
                            5.0f64.to_radians()
                        } else {
                            continue;
                        };
                        let (mut sx, mut sy) = (n.transform.skew_x, n.transform.skew_y);
                        if col == 0 {
                            sx += delta;
                        } else {
                            sy += delta;
                        }
                        self.editor.skew(&id, sx, sy);
                        self.status =
                            format!("skew ∠X {:.0}° ∠Y {:.0}°", sx.to_degrees(), sy.to_degrees());
                        return;
                    }
                }
                // 9-point transform-origin grid
                let gy = TOP_H + IY_ORIGIN_GRID;
                let gx0 = ix + 14.0;
                let points = [
                    (0.0, 0.0),
                    (0.5, 0.0),
                    (1.0, 0.0),
                    (0.0, 0.5),
                    (0.5, 0.5),
                    (1.0, 0.5),
                    (0.0, 1.0),
                    (0.5, 1.0),
                    (1.0, 1.0),
                ];
                for (i, (ox, oy)) in points.iter().enumerate() {
                    let (cx, cy) = (i % 3, i / 3);
                    let (px, py) = (gx0 + 7.0 + cx as f64 * 22.0, gy + 7.0 + cy as f64 * 22.0);
                    if (p.x - px).abs() <= 8.0 && (p.y - py).abs() <= 8.0 {
                        self.editor.set_origin(&id, *ox, *oy);
                        self.status =
                            format!("transform origin ({:.0},{:.0})", ox * 100.0, oy * 100.0);
                        return;
                    }
                }
            }
        }
        // VARIABLES tab interactions
        if self.inspector_tab == 2 {
            let ix2 = self.win_w - INSPECTOR_W;
            let y0 = TOP_H + 34.0;
            // mode chips
            if p.y >= y0 - 3.0 && p.y <= y0 + 13.0 {
                let mut mx = ix2 + 56.0;
                let modes = {
                    let mut v = vec!["default".to_string()];
                    v.extend(self.vars.mode_names());
                    v
                };
                for m in &modes {
                    let w = x_native::text::measure(m, 8.0) + 12.0;
                    if p.x >= mx && p.x <= mx + w {
                        self.vars.active_mode = if m == "default" {
                            None
                        } else {
                            Some(m.clone())
                        };
                        self.status = format!("mode: {m}");
                        return;
                    }
                    mx += w + 6.0;
                }
            }
            // catalog rows: expose toggles + bind actions
            let cat = self.vars.catalog();
            let mut y = y0 + 26.0;
            let mut last_col = String::new();
            for (collection, name, kind) in cat.iter().take(24) {
                if *collection != last_col {
                    y += 16.0;
                    last_col = collection.clone();
                }
                if p.y >= y - 2.0 && p.y <= y + 14.0 {
                    // expose-to-prototype toggle (numbers + strings)
                    if (*kind == "number" || *kind == "string")
                        && p.x >= self.win_w - 114.0
                        && p.x < self.win_w - 88.0
                    {
                        if self.vars.exposed.contains(name) {
                            self.vars.exposed.remove(name);
                            self.status = format!("{name}: hidden from prototype viewers");
                        } else {
                            self.vars.exposed.insert(name.clone());
                            self.status = format!("{name}: exposed — editable in present mode");
                        }
                        return;
                    }
                    if let Some(id) = self.editor.selection.first().cloned() {
                        match *kind {
                            "color" if p.x >= self.win_w - 48.0 => {
                                self.editor.set_fill(&id, Paint::Variable(name.clone()));
                                self.status = format!("fill of {id} -> var {name}");
                                return;
                            }
                            "number" if p.x >= self.win_w - 80.0 && p.x < self.win_w - 48.0 => {
                                if let Some(n) =
                                    x_native::editor::find_mut(&mut self.editor.root, &id)
                                {
                                    n.bindings.insert("radius".into(), name.clone());
                                    self.status = format!("radius of {id} -> var {name}");
                                }
                                return;
                            }
                            "number" if p.x >= self.win_w - 48.0 => {
                                if let Some(n) =
                                    x_native::editor::find_mut(&mut self.editor.root, &id)
                                {
                                    n.bindings.insert("opacity".into(), name.clone());
                                    self.status = format!("opacity of {id} -> var {name}");
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                y += 18.0;
            }
            return;
        }
        // COMPONENT section: variant chips + detach (instances)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if let x_native::NodeKind::Instance { component } = n.kind.clone() {
                    let id = n.id.clone();
                    let ix2 = self.win_w - INSPECTOR_W;
                    let cy0 = TOP_H + IY_SEC;
                    // variant chips
                    if let Some((set, _)) = component.split_once('/') {
                        let vars_list: Vec<String> = x_native::variants_of(&self.editor.root, set)
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        let mut vx = ix2 + 12.0;
                        let vy = cy0 + 16.0;
                        for vname in vars_list.iter().take(4) {
                            let short = vname.split_once('/').map(|(_, v)| v).unwrap_or(vname);
                            let cw = x_native::text::measure(short, 7.5) + 10.0;
                            if p.x >= vx && p.x <= vx + cw && p.y >= vy - 2.0 && p.y <= vy + 12.0 {
                                self.editor.swap_instance(&id, vname);
                                self.status = format!("variant: {short}");
                                return;
                            }
                            vx += cw + 4.0;
                        }
                    }
                    // detach
                    if p.x >= ix2 + 150.0
                        && p.x <= ix2 + 208.0
                        && p.y >= cy0 + 14.0
                        && p.y <= cy0 + 30.0
                    {
                        let v = self.vars.clone();
                        if self.editor.detach_selected_instance(&v) {
                            self.status = "detached".into();
                        }
                        return;
                    }
                    // ---- component property controls (Figma component props) ----
                    let props = self.editor.component_props(&component);
                    if !props.is_empty() {
                        let py0 = cy0 + 52.0;
                        let comps: Vec<String> = self.editor.component_names();
                        for (i, prop) in props.iter().enumerate() {
                            let py = py0 + i as f64 * 18.0;
                            let pname = prop.name().to_string();
                            let value = self.prop_value(n, prop);
                            match prop {
                                x_native::ComponentProp::Text { .. } => {
                                    if p.x >= ix2 + 120.0
                                        && p.x <= ix2 + INSPECTOR_W - 12.0
                                        && p.y >= py - 2.0
                                        && p.y <= py + 12.0
                                    {
                                        self.focus = Focus::Prop {
                                            instance_id: id.clone(),
                                            prop_name: pname.clone(),
                                            buffer: String::new(),
                                        };
                                        self.status = format!("type new {pname} (Enter commits)");
                                        return;
                                    }
                                }
                                x_native::ComponentProp::Bool { .. } => {
                                    if p.x >= ix2 + 120.0
                                        && p.x <= ix2 + 160.0
                                        && p.y >= py - 2.0
                                        && p.y <= py + 12.0
                                    {
                                        let next = if value == "true" { "false" } else { "true" };
                                        if self.editor.set_prop_value(&id, &pname, next) {
                                            self.status = format!("{pname} = {next}");
                                        }
                                        return;
                                    }
                                }
                                x_native::ComponentProp::Swap { .. } => {
                                    if p.x >= ix2 + 120.0
                                        && p.x <= ix2 + INSPECTOR_W - 12.0
                                        && p.y >= py - 2.0
                                        && p.y <= py + 12.0
                                    {
                                        if comps.is_empty() {
                                            self.status = "no components to swap to".into();
                                            return;
                                        }
                                        let pos =
                                            comps.iter().position(|c| c == &value).unwrap_or(0);
                                        let next = &comps[(pos + 1) % comps.len()];
                                        if self.editor.set_prop_value(&id, &pname, next) {
                                            self.status = format!("{pname} = {next}");
                                        }
                                        return;
                                    }
                                }
                                x_native::ComponentProp::Number { min, max, .. } => {
                                    let val = value.parse::<f64>().unwrap_or(0.0);
                                    let step = if self.shift { 10.0 } else { 1.0 };
                                    // minus
                                    if p.x >= ix2 + 120.0
                                        && p.x <= ix2 + 136.0
                                        && p.y >= py - 2.0
                                        && p.y <= py + 12.0
                                    {
                                        let v = (val - step).max(min.unwrap_or(f64::NEG_INFINITY));
                                        if self.editor.set_prop_value(&id, &pname, &format!("{v}"))
                                        {
                                            self.status = format!("{pname} = {v}");
                                        }
                                        return;
                                    }
                                    // plus
                                    if p.x >= ix2 + 198.0
                                        && p.x <= ix2 + 214.0
                                        && p.y >= py - 2.0
                                        && p.y <= py + 12.0
                                    {
                                        let v = (val + step).min(max.unwrap_or(f64::INFINITY));
                                        if self.editor.set_prop_value(&id, &pname, &format!("{v}"))
                                        {
                                            self.status = format!("{pname} = {v}");
                                        }
                                        return;
                                    }
                                    // value box -> type
                                    if p.x >= ix2 + 138.0
                                        && p.x <= ix2 + 196.0
                                        && p.y >= py - 2.0
                                        && p.y <= py + 12.0
                                    {
                                        self.focus = Focus::Prop {
                                            instance_id: id.clone(),
                                            prop_name: pname.clone(),
                                            buffer: String::new(),
                                        };
                                        self.status = format!("type new {pname} (Enter commits)");
                                        return;
                                    }
                                }
                                x_native::ComponentProp::Slot { .. } => {
                                    // slot content is set by pasting/inserting into the
                                    // instance (or via libraries); the row is display-only
                                }
                            }
                        }
                    }
                }
                // ---- component authoring (masters): +TEXT/+BOOL/+SWAP/+NUM + delete ----
                if let x_native::NodeKind::Component { name } = n.kind.clone() {
                    let ix2 = self.win_w - INSPECTOR_W;
                    let cy0 = TOP_H + IY_SEC;
                    let by = cy0 + 16.0;
                    let is_variant = name.contains('/');
                    let set = name.split_once('/').map(|(s, _)| s.to_string());
                    let names = ["TEXT", "BOOL", "SWAP", "NUM"];
                    for (i, t) in names.iter().enumerate() {
                        let bx = ix2 + 12.0 + i as f64 * 56.0;
                        if p.x >= bx && p.x <= bx + 52.0 && p.y >= by - 2.0 && p.y <= by + 12.0 {
                            // unique name: "Property", "Property 2", "Property 3" …
                            let existing = self.editor.component_props(&name);
                            let pname = {
                                let mut n = 1;
                                loop {
                                    let candidate = if n == 1 {
                                        "Property".to_string()
                                    } else {
                                        format!("Property {n}")
                                    };
                                    if !existing.iter().any(|p| p.name() == candidate) {
                                        break candidate;
                                    }
                                    n += 1;
                                }
                            };
                            // bind a sensible default target: the first child of the
                            // matching kind (text / instance / any), seeding the
                            // default from that child so the property works at once.
                            let master =
                                x_native::components::find_master(&self.editor.root, &name);
                            let child = match master {
                                Some(m) => match *t {
                                    "TEXT" => m.children.iter().find(|c| {
                                        matches!(c.kind, x_native::NodeKind::Text { .. })
                                    }),
                                    "SWAP" => m.children.iter().find(|c| {
                                        matches!(c.kind, x_native::NodeKind::Instance { .. })
                                    }),
                                    "SLOT" => m.children.iter().find(|c| {
                                        matches!(c.kind, x_native::NodeKind::Frame { .. })
                                    }),
                                    _ => m.children.first(),
                                },
                                None => None,
                            };
                            let (target, text_def, num_def, vis_def) = match child {
                                Some(c) => {
                                    let text = match &c.kind {
                                        x_native::NodeKind::Text { text } => text.clone(),
                                        _ => String::new(),
                                    };
                                    (c.id.clone(), text, c.w, c.visible)
                                }
                                None => (String::new(), String::new(), 100.0, true),
                            };
                            let prop = match *t {
                                "TEXT" => x_native::ComponentProp::Text {
                                    name: pname,
                                    target,
                                    default: text_def,
                                },
                                "BOOL" => x_native::ComponentProp::Bool {
                                    name: pname,
                                    target,
                                    default: vis_def,
                                },
                                "SWAP" => x_native::ComponentProp::Swap {
                                    name: pname,
                                    target,
                                    default: String::new(),
                                },
                                "SLOT" => x_native::ComponentProp::Slot {
                                    name: pname,
                                    target,
                                    default: None,
                                },
                                _ => x_native::ComponentProp::Number {
                                    name: pname,
                                    target,
                                    default: num_def,
                                    min: None,
                                    max: None,
                                },
                            };
                            // variant member: add to the whole set so all rows share
                            // the column; otherwise just this master.
                            if let Some(s) = &set {
                                let n = self.editor.add_component_prop_to_set(s, prop);
                                self.status = format!("added {t} property to {n} variant(s)");
                            } else if self.editor.add_component_prop(&name, prop) {
                                self.status = format!("added {t} property to {name}");
                            }
                            return;
                        }
                    }
                    if is_variant {
                        // ---- variant grid cell clicks ----
                        if let Some((variants, prop_names, gx, gy0, col_w)) =
                            self.variant_grid_layout(set.as_deref().unwrap_or(""))
                        {
                            // variant name cell → select that master
                            for (ri, v) in variants.iter().enumerate() {
                                let ry = gy0 + ri as f64 * 18.0;
                                if p.x >= gx + 12.0
                                    && p.x <= gx + 12.0 + 68.0
                                    && p.y >= ry - 2.0
                                    && p.y <= ry + 12.0
                                {
                                    if let Some(m) =
                                        x_native::components::find_master(&self.editor.root, v)
                                    {
                                        let id = m.id.clone();
                                        self.editor.selection = vec![id];
                                        self.status = format!("selected variant {v}");
                                    }
                                    return;
                                }
                            }
                            // property cells
                            for (ri, v) in variants.iter().enumerate() {
                                let ry = gy0 + ri as f64 * 18.0;
                                if p.y < ry - 2.0 || p.y > ry + 12.0 {
                                    continue;
                                }
                                for (ci, pn) in prop_names.iter().enumerate() {
                                    let cx = gx + 12.0 + 68.0 + ci as f64 * col_w;
                                    if p.x < cx || p.x > cx + col_w - 2.0 {
                                        continue;
                                    }
                                    let Some(prop) =
                                        self.variant_prop(v, set.as_deref().unwrap_or(""), pn)
                                    else {
                                        continue;
                                    };
                                    match prop {
                                        x_native::ComponentProp::Bool { default, .. } => {
                                            let next = if default { "false" } else { "true" };
                                            if self.editor.set_prop_default(v, pn, next) {
                                                self.status = format!("{v} · {pn} = {next}");
                                            }
                                        }
                                        x_native::ComponentProp::Swap { .. } => {
                                            let comps = self.editor.component_names();
                                            if comps.is_empty() {
                                                self.status = "no components to swap to".into();
                                                return;
                                            }
                                            let pos =
                                                comps.iter().position(|c| c == v).unwrap_or(0);
                                            let next = &comps[(pos + 1) % comps.len()];
                                            if self.editor.set_prop_default(v, pn, next) {
                                                self.status = format!("{v} · {pn} = {next}");
                                            }
                                        }
                                        x_native::ComponentProp::Text { .. }
                                        | x_native::ComponentProp::Number { .. } => {
                                            self.focus = Focus::VariantProp {
                                                component: v.clone(),
                                                prop_name: pn.clone(),
                                                buffer: String::new(),
                                            };
                                            self.status =
                                                format!("type new {pn} for {v} (Enter commits)");
                                        }
                                        x_native::ComponentProp::Slot { .. } => {
                                            // slot defaults are component names; no inline
                                            // cycle control in the variant grid yet
                                        }
                                    }
                                    return;
                                }
                            }
                        }
                    } else {
                        // ---- flat list: delete buttons ----
                        let props = self.editor.component_props(&name);
                        let py0 = by + 18.0;
                        for (i, prop) in props.iter().enumerate() {
                            let py = py0 + i as f64 * 24.0;
                            if p.x >= ix2 + INSPECTOR_W - 26.0
                                && p.x <= ix2 + INSPECTOR_W - 14.0
                                && p.y >= py - 2.0
                                && p.y <= py + 12.0
                            {
                                let pname = prop.name().to_string();
                                if self.editor.remove_component_prop(&name, &pname) {
                                    self.status = format!("removed property {pname}");
                                }
                                return;
                            }
                        }
                    }
                }
            }
        }
        // IMAGE controls: fit chips + replace (Design tab, image nodes)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if let x_native::NodeKind::Image { asset, .. } = &n.kind {
                    let id = n.id.clone();
                    let cur_asset = asset.clone();
                    let ix2 = self.win_w - INSPECTOR_W;
                    let iy = TOP_H + IY_SEC;
                    // fit chips
                    if p.y >= iy + 14.0 && p.y <= iy + 30.0 {
                        for (i, fit) in [
                            x_native::ImageFit::Fill,
                            x_native::ImageFit::Fit,
                            x_native::ImageFit::Crop,
                            x_native::ImageFit::Tile,
                        ]
                        .iter()
                        .enumerate()
                        {
                            let bx = ix2 + 12.0 + i as f64 * 48.0;
                            if p.x >= bx && p.x <= bx + 44.0 {
                                if let Some(nm) =
                                    x_native::editor::find_mut(&mut self.editor.root, &id)
                                {
                                    if let x_native::NodeKind::Image { fit: f, .. } = &mut nm.kind {
                                        *f = *fit;
                                        nm.dirty = true;
                                    }
                                }
                                self.status = format!("image fit: {:?}", fit);
                                return;
                            }
                        }
                    }
                    // replace: cycle to the next loaded asset
                    if p.y >= iy + 36.0
                        && p.y <= iy + 52.0
                        && p.x >= ix2 + 12.0
                        && p.x <= ix2 + 96.0
                    {
                        let names = self.assets.names();
                        if names.is_empty() {
                            self.status = "no assets loaded (drop PNGs in assets/)".into();
                            return;
                        }
                        let pos = names.iter().position(|a| *a == cur_asset).unwrap_or(0);
                        let next = names[(pos + 1) % names.len()].clone();
                        if let Some(nm) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                            if let x_native::NodeKind::Image { asset: a, .. } = &mut nm.kind {
                                *a = next.clone();
                                nm.dirty = true;
                            }
                        }
                        self.status = format!("image -> {next}");
                        return;
                    }
                    // placement: focal X/Y steppers, scale, flips, reset
                    let mut placed: Option<String> = None;
                    {
                        let py = iy + 58.0;
                        let zy = iy + 78.0;
                        let edit = |app: &mut Self,
                                    f: &dyn Fn(&mut x_native::ImagePlacement),
                                    what: &str| {
                            if let Some(nm) = x_native::editor::find_mut(&mut app.editor.root, &id)
                            {
                                if let x_native::NodeKind::Image { placement, .. } = &mut nm.kind {
                                    f(placement);
                                    nm.dirty = true;
                                    return Some(format!(
                                        "{what}: fx={:.2} fy={:.2} s={:.2} fh={} fv={}",
                                        placement.focal.0,
                                        placement.focal.1,
                                        placement.scale,
                                        placement.flip_h,
                                        placement.flip_v
                                    ));
                                }
                            }
                            None
                        };
                        if p.y >= py - 3.0 && p.y <= py + 11.0 {
                            if p.x >= ix2 + 56.0 && p.x <= ix2 + 71.0 {
                                placed = edit(
                                    self,
                                    &|pl| pl.focal.0 = (pl.focal.0 - 0.1).max(0.0),
                                    "focal x-",
                                );
                            } else if p.x >= ix2 + 74.0 && p.x <= ix2 + 89.0 {
                                placed = edit(
                                    self,
                                    &|pl| pl.focal.0 = (pl.focal.0 + 0.1).min(1.0),
                                    "focal x+",
                                );
                            } else if p.x >= ix2 + 156.0 && p.x <= ix2 + 171.0 {
                                placed = edit(
                                    self,
                                    &|pl| pl.focal.1 = (pl.focal.1 - 0.1).max(0.0),
                                    "focal y-",
                                );
                            } else if p.x >= ix2 + 174.0 && p.x <= ix2 + 189.0 {
                                placed = edit(
                                    self,
                                    &|pl| pl.focal.1 = (pl.focal.1 + 0.1).min(1.0),
                                    "focal y+",
                                );
                            }
                        } else if p.y >= zy - 3.0 && p.y <= zy + 11.0 {
                            if p.x >= ix2 + 84.0 && p.x <= ix2 + 99.0 {
                                placed = edit(
                                    self,
                                    &|pl| pl.scale = (pl.scale - 0.1).max(0.1),
                                    "scale-",
                                );
                            } else if p.x >= ix2 + 102.0 && p.x <= ix2 + 117.0 {
                                placed = edit(
                                    self,
                                    &|pl| pl.scale = (pl.scale + 0.1).min(4.0),
                                    "scale+",
                                );
                            } else if p.x >= ix2 + 124.0 && p.x <= ix2 + 146.0 {
                                placed = edit(self, &|pl| pl.flip_h = !pl.flip_h, "flip h");
                            } else if p.x >= ix2 + 150.0 && p.x <= ix2 + 172.0 {
                                placed = edit(self, &|pl| pl.flip_v = !pl.flip_v, "flip v");
                            } else if p.x >= ix2 + 178.0 && p.x <= ix2 + 214.0 {
                                placed = edit(
                                    self,
                                    &|pl| *pl = x_native::ImagePlacement::default(),
                                    "reset crop",
                                );
                            }
                        }
                    }
                    if let Some(msg) = placed {
                        self.status = msg;
                        return;
                    }
                }
            }
        }
        // STYLES: create-from-selection (+P/+T/+FX) and apply chips
        // (text nodes hand this slot to the font browser)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if matches!(n.kind, x_native::NodeKind::Text { .. }) { /* font browser owns the slot */
                } else {
                    let id = n.id.clone();
                    let ix2 = self.win_w - INSPECTOR_W;
                    let sy0 = TOP_H + IY_STYLES;
                    // create buttons
                    if p.y >= sy0 - 3.0 && p.y <= sy0 + 11.0 {
                        for i in 0..3usize {
                            let bx = ix2 + 70.0 + i as f64 * 32.0;
                            if p.x >= bx && p.x <= bx + 28.0 {
                                let count = self.styles.len() + 1;
                                let (name, style) = match i {
                                    0 => (
                                        format!("Paint/{count}"),
                                        x_native::Style::Paint {
                                            fill: n.fill.clone(),
                                        },
                                    ),
                                    1 => (
                                        format!("Text/{count}"),
                                        x_native::Style::Text {
                                            font: n
                                                .bindings
                                                .get("font")
                                                .cloned()
                                                .unwrap_or_default(),
                                            size: n.h,
                                            letter_spacing: 0.0,
                                            line_height: 1.2,
                                        },
                                    ),
                                    _ => (
                                        format!("FX/{count}"),
                                        x_native::Style::Effect {
                                            effects: n.effects.clone(),
                                        },
                                    ),
                                };
                                self.styles.insert(name.clone(), style);
                                self.status = format!("style created: {name}");
                                return;
                            }
                        }
                    }
                    // styles browser: EXACT same geometry as the painter
                    {
                        for (name, _kind, r, row_kind) in self.styles_layout() {
                            if !(p.x >= r.x0 && p.x <= r.x1 && p.y >= r.y0 && p.y <= r.y1) {
                                continue;
                            }
                            match row_kind {
                                1 => continue, // header
                                2 => {
                                    self.focus = Focus::StyleSearch;
                                    self.status = "type to filter styles".into();
                                    return;
                                }
                                3 => {
                                    self.run_style_action(&name);
                                    return;
                                }
                                _ => {
                                    let s = self.styles[&name].clone();
                                    if self.shift {
                                        // SHIFT+click = redefine the style FROM the selection,
                                        // then propagate to every bound consumer (all pages)
                                        let newdef = if let Some(sel) = self.selected_single() {
                                            match &s {
                                                x_native::Style::Paint { .. } => {
                                                    x_native::Style::Paint {
                                                        fill: sel.fill.clone(),
                                                    }
                                                }
                                                x_native::Style::Text { .. } => {
                                                    x_native::Style::Text {
                                                        font: sel
                                                            .bindings
                                                            .get("font")
                                                            .cloned()
                                                            .unwrap_or_default(),
                                                        size: sel.h,
                                                        letter_spacing: 0.0,
                                                        line_height: 1.2,
                                                    }
                                                }
                                                x_native::Style::Effect { .. } => {
                                                    x_native::Style::Effect {
                                                        effects: sel.effects.clone(),
                                                    }
                                                }
                                            }
                                        } else {
                                            s.clone()
                                        };
                                        self.styles.insert(name.clone(), newdef);
                                        let mut updated = x_native::resolve_styles(
                                            &mut self.editor.root,
                                            &self.styles,
                                        );
                                        for (i, pg) in self.pages.iter_mut().enumerate() {
                                            if i != self.page_idx {
                                                updated +=
                                                    x_native::resolve_styles(pg, &self.styles);
                                            }
                                        }
                                        self.status = format!("style {name} redefined -> {updated} consumer(s) updated");
                                    } else if self.ctrl {
                                        // CTRL+click = select for management (REN/DUP/DEL/DET)
                                        self.style_sel = Some(name.clone());
                                        self.status = format!(
                                            "style selected: {name} (REN/DUP/DEL/DET below)"
                                        );
                                    } else {
                                        if let Some(nm) =
                                            x_native::editor::find_mut(&mut self.editor.root, &id)
                                        {
                                            x_native::bind_style(nm, &name, &s);
                                        }
                                        self.style_sel = Some(name.clone());
                                        self.status = format!("applied style: {name}");
                                    }
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        // Fill / Stroke / Effects section interactions (mockup sections,
        // geometry from the IY_* map shared with the painter)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                let id = n.id.clone();
                let fill_len = if n.visual_stacks_materialized {
                    n.fill_layers.len()
                } else {
                    1
                };
                let fill_idx = self.fill_layer_index.min(fill_len.saturating_sub(1));
                let fill = n
                    .fill_layers
                    .get(fill_idx)
                    .map(|l| l.paint.clone())
                    .unwrap_or_else(|| n.fill.clone());
                let fill_visible = n
                    .fill_layers
                    .get(fill_idx)
                    .map(|l| l.visible)
                    .unwrap_or(!n.visual_stacks_materialized || !n.fill_layers.is_empty());
                let stroke_len = if !n.visual_stacks_materialized {
                    if n.stroke.width > 0.0 {
                        1
                    } else {
                        0
                    }
                } else {
                    n.stroke_layers.len()
                };
                let stroke_idx = self.stroke_layer_index.min(stroke_len.saturating_sub(1));
                let stroke = n
                    .stroke_layers
                    .get(stroke_idx)
                    .map(|l| l.stroke.clone())
                    .unwrap_or_else(|| n.stroke.clone());
                let fx_len = if !n.visual_stacks_materialized {
                    n.effects.len()
                } else {
                    n.effect_layers.len()
                };
                let ix2 = self.win_w - INSPECTOR_W;
                let next_blend = |b: BlendKind| match b {
                    BlendKind::Normal => BlendKind::Darken,
                    BlendKind::Darken => BlendKind::Multiply,
                    BlendKind::Multiply => BlendKind::ColorBurn,
                    BlendKind::ColorBurn => BlendKind::Lighten,
                    BlendKind::Lighten => BlendKind::Screen,
                    BlendKind::Screen => BlendKind::ColorDodge,
                    BlendKind::ColorDodge => BlendKind::Overlay,
                    BlendKind::Overlay => BlendKind::SoftLight,
                    BlendKind::SoftLight => BlendKind::HardLight,
                    BlendKind::HardLight => BlendKind::Difference,
                    BlendKind::Difference => BlendKind::Exclusion,
                    BlendKind::Exclusion => BlendKind::Hue,
                    BlendKind::Hue => BlendKind::Saturation,
                    BlendKind::Saturation => BlendKind::Color,
                    BlendKind::Color => BlendKind::Luminosity,
                    BlendKind::Luminosity => BlendKind::Normal,
                };
                // Fill header row: GR toggle
                let hy = TOP_H + IY_FILL_HDR;
                if fill_len > 0
                    && p.y >= hy - 3.0
                    && p.y <= hy + 12.0
                    && p.x >= ix2 + 40.0
                    && p.x <= ix2 + 74.0
                {
                    self.fill_layer_index = (fill_idx + 1) % fill_len;
                    self.status = format!("fill layer {} selected", self.fill_layer_index + 1);
                    return;
                }
                if p.y >= hy - 3.0 && p.y <= hy + 12.0 {
                    if p.x >= ix2 + 78.0 && p.x <= ix2 + 106.0 && fill_len > 0 {
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.fill_layers.get_mut(fill_idx) {
                                l.blend = next_blend(l.blend);
                            }
                        });
                        self.status = "fill blend mode changed".into();
                        return;
                    }
                    if p.x >= ix2 + 108.0 && p.x <= ix2 + 126.0 && fill_idx + 1 < fill_len {
                        self.editor.move_fill_layer(&id, fill_idx, fill_idx + 1);
                        self.fill_layer_index += 1;
                        return;
                    }
                    if p.x >= ix2 + 128.0 && p.x <= ix2 + 146.0 && fill_idx > 0 {
                        self.editor.move_fill_layer(&id, fill_idx, fill_idx - 1);
                        self.fill_layer_index -= 1;
                        return;
                    }
                    if p.x >= ix2 + 148.0 && p.x <= ix2 + 166.0 {
                        self.editor.remove_fill_layer(&id, fill_idx);
                        self.fill_layer_index = self.fill_layer_index.saturating_sub(1);
                        self.status = "fill layer removed".into();
                        return;
                    }
                }
                if p.y >= hy - 3.0 && p.y <= hy + 11.0 && p.x >= ix2 + INSPECTOR_W - 28.0 {
                    self.editor.add_fill_layer(&id, Paint::Solid(Color::WHITE));
                    self.fill_layer_index = fill_len;
                    self.status = "fill layer added".into();
                    return;
                }
                if p.y >= hy - 2.0 && p.y <= hy + 12.0 && p.x >= ix2 + 178.0 && p.x <= ix2 + 206.0 {
                    let new_fill = match &fill {
                        Paint::LinearGradient { .. } | Paint::RadialGradient { .. } => {
                            Paint::Solid(C_ACCENT)
                        }
                        Paint::Solid(c) => Paint::LinearGradient {
                            start: (0.0, 0.0),
                            end: (1.0, 0.0),
                            stops: vec![(0.0, *c), (1.0, Color::from_rgb8(0x8e, 0x2d, 0xe2))],
                            space: GradSpace::Srgb,
                        },
                        other => other.clone(),
                    };
                    let w = self.selected_single().map(|n| n.w).unwrap_or(100.0);
                    let new_fill = if let Paint::LinearGradient { start, stops, .. } = new_fill {
                        Paint::LinearGradient {
                            start,
                            end: (w, 0.0),
                            stops,
                            space: GradSpace::Srgb,
                        }
                    } else {
                        new_fill
                    };
                    self.editor.set_fill(&id, new_fill);
                    self.gradient_editing = !matches!(
                        fill,
                        Paint::LinearGradient { .. } | Paint::RadialGradient { .. }
                    );
                    self.gradient_stop = 0;
                    self.status = "gradient toggled".into();
                    return;
                }
                // Fill row eye: toggle node visibility (mockup per-row eye)
                let fry = TOP_H + IY_FILLROW;
                if matches!(
                    &fill,
                    Paint::LinearGradient { .. } | Paint::RadialGradient { .. }
                ) && p.y >= fry - 2.0
                    && p.y <= fry + 15.0
                {
                    let row_x1 = ix2 + INSPECTOR_W - 12.0;
                    // interpolation chip: SRGB ⇄ OKLAB (top fill only)
                    if p.x >= ix2 + 86.0 && p.x <= ix2 + 122.0 {
                        let new_space = match &fill {
                            Paint::LinearGradient { space, .. }
                            | Paint::RadialGradient { space, .. } => {
                                if *space == GradSpace::Srgb {
                                    GradSpace::Oklab
                                } else {
                                    GradSpace::Srgb
                                }
                            }
                            _ => GradSpace::Srgb,
                        };
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(layer) = nm.fill_layers.get_mut(fill_idx) {
                                match &mut layer.paint {
                                    Paint::LinearGradient { space, .. }
                                    | Paint::RadialGradient { space, .. } => *space = new_space,
                                    _ => {}
                                }
                            }
                        });
                        self.status = format!(
                            "gradient interpolation: {}",
                            if new_space == GradSpace::Oklab {
                                "OKLab (perceptual)"
                            } else {
                                "sRGB"
                            }
                        );
                        return;
                    }
                    let stop_count = match &fill {
                        Paint::LinearGradient { stops, .. }
                        | Paint::RadialGradient { stops, .. } => stops.len(),
                        _ => 0,
                    };
                    for i in 0..stop_count {
                        let cx = row_x1 - 92.0 + i as f64 * 16.0;
                        if (p.x - cx).abs() <= 8.0 {
                            self.gradient_stop = i;
                            self.status = format!("gradient stop {} selected", i + 1);
                            return;
                        }
                    }
                }
                if p.y >= fry - 2.0 && p.y <= fry + 15.0 && p.x >= ix2 + INSPECTOR_W - 36.0 {
                    self.editor.mutate_visual_stack(&id, |nm| {
                        if let Some(layer) = nm.fill_layers.get_mut(fill_idx) {
                            layer.visible = !layer.visible;
                        }
                    });
                    self.status = format!("fill {}", if fill_visible { "hidden" } else { "shown" });
                    return;
                }
                if fill_len > 0 && p.y >= fry - 2.0 && p.y <= fry + 15.0 {
                    let row_x1 = ix2 + INSPECTOR_W - 12.0;
                    if p.x >= row_x1 - 70.0 && p.x <= row_x1 - 38.0 {
                        let increase = p.x >= row_x1 - 54.0;
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.fill_layers.get_mut(fill_idx) {
                                l.opacity =
                                    (l.opacity + if increase { 0.1 } else { -0.1 }).clamp(0.0, 1.0);
                            }
                        });
                        self.status = "fill opacity changed".into();
                        return;
                    }
                }
                let shy = TOP_H + IY_STROKE_HDR;
                if p.y >= shy - 3.0
                    && p.y <= shy + 12.0
                    && p.x >= ix2 + 40.0
                    && p.x <= ix2 + 74.0
                    && stroke_len > 0
                {
                    self.stroke_layer_index = (stroke_idx + 1) % stroke_len;
                    self.status = format!("stroke layer {} selected", self.stroke_layer_index + 1);
                    return;
                }
                if p.y >= shy - 3.0 && p.y <= shy + 12.0 && stroke_len > 0 {
                    if p.x >= ix2 + 78.0 && p.x <= ix2 + 106.0 {
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.blend = next_blend(l.blend);
                            }
                        });
                        self.status = "stroke blend mode changed".into();
                        return;
                    }
                    if p.x >= ix2 + 108.0 && p.x <= ix2 + 126.0 && stroke_idx + 1 < stroke_len {
                        self.editor
                            .move_stroke_layer(&id, stroke_idx, stroke_idx + 1);
                        self.stroke_layer_index += 1;
                        return;
                    }
                    if p.x >= ix2 + 128.0 && p.x <= ix2 + 146.0 && stroke_idx > 0 {
                        self.editor
                            .move_stroke_layer(&id, stroke_idx, stroke_idx - 1);
                        self.stroke_layer_index -= 1;
                        return;
                    }
                    if p.x >= ix2 + 148.0 && p.x <= ix2 + 166.0 {
                        self.editor.remove_stroke_layer(&id, stroke_idx);
                        self.stroke_layer_index = self.stroke_layer_index.saturating_sub(1);
                        self.status = "stroke layer removed".into();
                        return;
                    }
                }
                if p.y >= shy - 3.0 && p.y <= shy + 11.0 && p.x >= ix2 + INSPECTOR_W - 28.0 {
                    self.editor.add_stroke_layer(
                        &id,
                        x_native::Stroke::solid(Color::from_rgb8(0xe5, 0xe7, 0xeb), 1.0),
                    );
                    self.stroke_layer_index = stroke_len;
                    self.status = "stroke layer added".into();
                    return;
                }
                // Stroke row: width -/+ steppers + swatch cycles palette
                let sry = TOP_H + IY_STROKEROW;
                if p.y >= sry - 2.0 && p.y <= sry + 15.0 {
                    let row_x1 = ix2 + INSPECTOR_W - 12.0;
                    if p.x >= row_x1 - 68.0 && p.x <= row_x1 - 6.0 {
                        // right 16px of the pill = chevron -> advanced-stroke popover
                        if p.x >= row_x1 - 24.0 {
                            self.stroke_advanced_open = !self.stroke_advanced_open;
                            self.status = if self.stroke_advanced_open {
                                "advanced stroke open".into()
                            } else {
                                "advanced stroke closed".into()
                            };
                            return;
                        }
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.options.align = match l.options.align {
                                    StrokeAlign::Inside => StrokeAlign::Center,
                                    StrokeAlign::Center => StrokeAlign::Outside,
                                    StrokeAlign::Outside => StrokeAlign::Inside,
                                };
                            }
                        });
                        self.status = "stroke alignment changed".into();
                        return;
                    }
                    if p.x >= row_x1 - 112.0 && p.x <= row_x1 - 96.0 {
                        let w0 = stroke.width;
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.stroke.width = (l.stroke.width - 1.0).max(0.0);
                            }
                        });
                        self.status = format!("stroke {:.0}", (w0 - 1.0).max(0.0));
                        return;
                    }
                    if p.x >= row_x1 - 94.0 && p.x <= row_x1 - 78.0 {
                        let w0 = stroke.width;
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.stroke.width += 1.0;
                                if l.stroke
                                    .solid_color()
                                    .map(|c| c.components[3] == 0.0)
                                    .unwrap_or(false)
                                {
                                    l.stroke.set_solid_color(Color::from_rgb8(0xe5, 0xe7, 0xeb));
                                }
                            }
                        });
                        self.status = format!("stroke {:.0}", w0 + 1.0);
                        return;
                    }
                    if p.x >= ix2 + 12.0 && p.x <= ix2 + 34.0 {
                        // swatch click: cycle stroke color through the palette
                        let cur = stroke.solid_color().unwrap_or(Color::TRANSPARENT);
                        let pos = PALETTE
                            .iter()
                            .position(|c| *c == cur)
                            .map(|i| (i + 1) % PALETTE.len())
                            .unwrap_or(0);
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.stroke.set_solid_color(PALETTE[pos]);
                                if l.stroke.width == 0.0 {
                                    l.stroke.width = 1.0;
                                }
                            }
                        });
                        self.status =
                            format!("stroke color -> {}", x_native::color_to_hex(PALETTE[pos]));
                        return;
                    }
                }
                // Effects: + button adds drop shadow; row eye removes it
                let fxh = TOP_H + IY_FX_HDR;
                let effect_idx = self.effect_layer_index.min(fx_len.saturating_sub(1));
                if p.y >= fxh - 3.0
                    && p.y <= fxh + 12.0
                    && p.x >= ix2 + 40.0
                    && p.x <= ix2 + 74.0
                    && fx_len > 0
                {
                    self.effect_layer_index = (effect_idx + 1) % fx_len;
                    self.status = format!("effect layer {} selected", self.effect_layer_index + 1);
                    return;
                }
                if p.y >= fxh - 3.0 && p.y <= fxh + 12.0 && fx_len > 0 {
                    if p.x >= ix2 + 78.0 && p.x <= ix2 + 106.0 {
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.effect_layers.get_mut(effect_idx) {
                                l.blend = next_blend(l.blend);
                            }
                        });
                        self.status = "effect blend mode changed".into();
                        return;
                    }
                    if p.x >= ix2 + 108.0 && p.x <= ix2 + 126.0 && effect_idx + 1 < fx_len {
                        self.editor
                            .move_effect_layer(&id, effect_idx, effect_idx + 1);
                        self.effect_layer_index += 1;
                        return;
                    }
                    if p.x >= ix2 + 128.0 && p.x <= ix2 + 146.0 && effect_idx > 0 {
                        self.editor
                            .move_effect_layer(&id, effect_idx, effect_idx - 1);
                        self.effect_layer_index -= 1;
                        return;
                    }
                    if p.x >= ix2 + 148.0 && p.x <= ix2 + 166.0 {
                        self.editor.remove_effect_layer(&id, effect_idx);
                        self.effect_layer_index = self.effect_layer_index.saturating_sub(1);
                        self.status = "effect layer removed".into();
                        return;
                    }
                }
                if p.y >= fxh - 3.0 && p.y <= fxh + 11.0 && p.x >= ix2 + INSPECTOR_W - 28.0 {
                    self.editor.add_effect_layer(
                        &id,
                        x_native::Effect::DropShadow {
                            dx: 4.0,
                            dy: 6.0,
                            blur: 10.0,
                            color: Color::from_rgba8(0, 0, 0, 160),
                        },
                    );
                    self.effect_layer_index = fx_len;
                    self.status = "drop shadow added".into();
                    return;
                }
                if fx_len > 0 {
                    for i in 0..fx_len.min(4) {
                        let ry = TOP_H + IY_FXROW + i as f64 * 18.0;
                        if p.y >= ry - 2.0 && p.y <= ry + 14.0 {
                            if p.x >= ix2 + INSPECTOR_W - 36.0 {
                                self.editor.mutate_visual_stack(&id, |nm| {
                                    if i < nm.effect_layers.len() {
                                        nm.effect_layers[i].visible = !nm.effect_layers[i].visible;
                                    }
                                });
                                self.status = "effect visibility toggled".into();
                                return;
                            }
                            if p.x >= ix2 + 12.0 && p.x <= ix2 + 150.0 {
                                self.effect_layer_index = i;
                                self.editor.mutate_visual_stack(&id, |nm| {
                                    if let Some(layer) = nm.effect_layers.get_mut(i) {
                                        layer.effect = match layer.effect.clone() {
                                            Effect::DropShadow {
                                                dx,
                                                dy,
                                                blur,
                                                color,
                                            } => Effect::InnerShadow {
                                                dx,
                                                dy,
                                                blur,
                                                color,
                                            },
                                            Effect::InnerShadow { blur, .. } => {
                                                Effect::LayerBlur { radius: blur }
                                            }
                                            Effect::LayerBlur { radius } => {
                                                Effect::BackgroundBlur { radius }
                                            }
                                            Effect::BackgroundBlur { radius } => {
                                                Effect::DropShadow {
                                                    dx: 4.0,
                                                    dy: 6.0,
                                                    blur: radius,
                                                    color: Color::from_rgba8(0, 0, 0, 160),
                                                }
                                            }
                                        };
                                    }
                                });
                                self.status = "effect type changed".into();
                                return;
                            }
                            if p.x >= ix2 + 154.0 && p.x <= ix2 + 218.0 {
                                let delta = if p.x < ix2 + 186.0 { -1.0 } else { 1.0 };
                                self.editor.mutate_visual_stack(&id, |nm| {
                                    if let Some(layer) = nm.effect_layers.get_mut(i) {
                                        match &mut layer.effect {
                                            Effect::DropShadow { blur, .. }
                                            | Effect::InnerShadow { blur, .. } => {
                                                *blur = (*blur + delta).max(0.0)
                                            }
                                            Effect::LayerBlur { radius }
                                            | Effect::BackgroundBlur { radius } => {
                                                *radius = (*radius + delta).max(0.0)
                                            }
                                        }
                                    }
                                });
                                self.status = "effect radius changed".into();
                                return;
                            }
                        }
                    }
                }
            }
        }
        // alignment row (Design tab): operates on multi-selection industry-standard
        if self.inspector_tab == 0 && !self.editor.selection.is_empty() {
            let ix2 = self.win_w - INSPECTOR_W;
            let ay = TOP_H + 24.0;
            if p.y >= ay - 2.0 && p.y <= ay + 14.0 {
                for i in 0..6usize {
                    let x = ix2 + 12.0 + i as f64 * 32.0;
                    if p.x >= x && p.x <= x + 28.0 {
                        use x_native::editor::AlignKind::*;
                        let kind = [Left, CenterH, Right, Top, CenterV, Bottom][i];
                        let ids = self.editor.selection.clone();
                        if ids.len() >= 2 {
                            x_native::editor::align(&mut self.editor.root, &ids, kind);
                            self.status = format!("aligned {:?}", kind);
                        } else if let Some(id) = ids.first() {
                            // single selection: align within its parent frame
                            let rootw = self.editor.root.w;
                            let rooth = self.editor.root.h;
                            if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, id) {
                                match kind {
                                    Left => n.transform.x = 0.0,
                                    Right => n.transform.x = rootw - n.w,
                                    CenterH => n.transform.x = (rootw - n.w) / 2.0,
                                    Top => n.transform.y = 0.0,
                                    Bottom => n.transform.y = rooth - n.h,
                                    CenterV => n.transform.y = (rooth - n.h) / 2.0,
                                }
                            }
                            self.status = format!("aligned {:?} to page", kind);
                        }
                        return;
                    }
                }
            }
        }
        // constraints rows — INSPECT tab (mockup's third tab)
        if self.inspector_tab == 4 {
            let ix2 = self.win_w - INSPECTOR_W;
            let cy = TOP_H + IY_CONSTRAINTS;
            if let Some(id) = self.editor.selection.first().cloned() {
                if p.y >= cy + 14.0 && p.y <= cy + 30.0 {
                    for i in 0..5usize {
                        let x = ix2 + 12.0 + i as f64 * 34.0;
                        if p.x >= x && p.x <= x + 30.0 {
                            use x_native::HPin::*;
                            let h = [Left, Right, CenterH, StretchH, ScaleH][i];
                            let v = find(&self.editor.root, &id)
                                .map(|n| n.pin.1)
                                .unwrap_or_default();
                            self.editor.set_pin(&id, h, v);
                            self.status = format!("h-pin {:?}", h);
                            return;
                        }
                    }
                }
                if p.y >= cy + 34.0 && p.y <= cy + 50.0 {
                    for i in 0..5usize {
                        let x = ix2 + 12.0 + i as f64 * 34.0;
                        if p.x >= x && p.x <= x + 30.0 {
                            use x_native::VPin::*;
                            let v = [Top, Bottom, CenterV, StretchV, ScaleV][i];
                            let h = find(&self.editor.root, &id)
                                .map(|n| n.pin.0)
                                .unwrap_or_default();
                            self.editor.set_pin(&id, h, v);
                            self.status = format!("v-pin {:?}", v);
                            return;
                        }
                    }
                }
                // code language tabs
                let langs = ["CSS", "SWIFT", "COMPOSE", "XML"];
                if p.y >= TOP_H + IY_CODE_TABS - 3.0 && p.y <= TOP_H + IY_CODE_TABS + 13.0 {
                    for (i, _) in langs.iter().enumerate() {
                        let bx = ix2 + 12.0 + i as f64 * 66.0;
                        if p.x >= bx && p.x <= bx + 60.0 {
                            self.inspect_lang = i as u8;
                            self.status = format!("code: {}", langs[i].to_lowercase());
                            return;
                        }
                    }
                }
                // copy code to OS clipboard
                if p.y >= TOP_H + IY_CODE_COPY
                    && p.y <= TOP_H + IY_CODE_COPY + 16.0
                    && p.x >= ix2 + 12.0
                    && p.x <= ix2 + 120.0
                {
                    let code = match self.inspect_lang {
                        0 => node_to_css(find(&self.editor.root, &id).unwrap(), &self.vars),
                        1 => node_to_swift(find(&self.editor.root, &id).unwrap(), &self.vars),
                        2 => node_to_compose(find(&self.editor.root, &id).unwrap(), &self.vars),
                        _ => node_to_xml(find(&self.editor.root, &id).unwrap(), &self.vars),
                    };
                    crate::os_clipboard_set(&code);
                    self.status = "code copied to clipboard".into();
                    return;
                }
                // code connect chip: edit the node's source link
                let conn = TOP_H + IY_CODE_COPY + 24.0;
                if p.y >= conn - 3.0
                    && p.y <= conn + 13.0
                    && p.x >= ix2 + 90.0
                    && p.x <= self.win_w - 12.0
                {
                    let buffer = find(&self.editor.root, &id)
                        .and_then(|n| n.bindings.get("code").cloned())
                        .unwrap_or_default();
                    self.focus = Focus::CodeRef {
                        node_id: id.clone(),
                        buffer,
                    };
                    self.status = "type the source path or URL — Enter links, empty clears".into();
                    return;
                }
            }
        }
        // FONT BROWSER + typography steppers (text node, Design tab)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if matches!(n.kind, x_native::NodeKind::Text { .. }) {
                    let id = n.id.clone();
                    let ixf = self.win_w - INSPECTOR_W;
                    let fy = TOP_H + IY_FONT;
                    // typography boxes: Size / Sp(letter) / Lh(line height);
                    // click upper half = increase, lower half = decrease
                    {
                        let ry = fy + 16.0;
                        if p.y >= ry - 3.0 && p.y <= ry + 14.0 {
                            for k in 0..3usize {
                                let bx = ixf + 12.0 + k as f64 * 74.0;
                                if p.x >= bx + 52.0 && p.x <= bx + 68.0 {
                                    let up = p.y < ry + 5.5;
                                    match k {
                                        0 => {
                                            let nh = (n.h + if up { 2.0 } else { -2.0 })
                                                .clamp(6.0, 400.0);
                                            let w = n.w;
                                            self.editor.resize(&id, w, nh);
                                            self.status = format!("text size {nh:.0}");
                                        }
                                        1 => {
                                            let cur = n
                                                .bindings
                                                .get("ls")
                                                .and_then(|v| v.parse::<f64>().ok())
                                                .unwrap_or(0.0);
                                            let nv = (cur + if up { 0.5 } else { -0.5 })
                                                .clamp(-5.0, 40.0);
                                            if let Some(nm) = x_native::editor::find_mut(
                                                &mut self.editor.root,
                                                &id,
                                            ) {
                                                nm.bindings.insert("ls".into(), format!("{nv}"));
                                                nm.dirty = true;
                                            }
                                            self.status = format!("letter spacing {nv:.1}");
                                        }
                                        _ => {
                                            let cur = n
                                                .bindings
                                                .get("lh")
                                                .and_then(|v| v.parse::<f64>().ok())
                                                .unwrap_or(1.2);
                                            let nv =
                                                (cur + if up { 0.1 } else { -0.1 }).clamp(0.6, 3.0);
                                            if let Some(nm) = x_native::editor::find_mut(
                                                &mut self.editor.root,
                                                &id,
                                            ) {
                                                nm.bindings.insert("lh".into(), format!("{nv:.2}"));
                                                nm.dirty = true;
                                            }
                                            self.status = format!("line height {nv:.2}");
                                        }
                                    }
                                    return;
                                }
                            }
                        }
                    }
                    // text-wrap chip: AUTO -> BALANCE -> PRETTY (Figma
                    // Aug-2026 text wrap), riding the search row's end
                    if p.y >= fy + 34.0
                        && p.y <= fy + 50.0
                        && p.x >= self.win_w - 52.0
                        && p.x <= self.win_w - 12.0
                    {
                        let next = match n.text_wrap() {
                            x_native::TextWrap::Auto => x_native::TextWrap::Balance,
                            x_native::TextWrap::Balance => x_native::TextWrap::Pretty,
                            x_native::TextWrap::Pretty => x_native::TextWrap::Auto,
                        };
                        if let Some(nm) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                            nm.bindings.insert("tw".into(), next.to_str().to_string());
                            nm.dirty = true;
                        }
                        self.status = format!("text wrap: {}", next.to_str());
                        return;
                    }
                    // search box (moved below the typography row)
                    if p.y >= fy + 34.0 && p.y <= fy + 50.0 && p.x >= ixf + 12.0 {
                        self.focus = Focus::FontSearch;
                        self.status = "type to search all fonts".into();
                        return;
                    }
                    // result rows
                    let visible = FONT_ROWS;
                    let start = self
                        .font_scroll
                        .min(self.font_results.len().saturating_sub(1));
                    for row in start..(start + visible).min(self.font_results.len()) {
                        let y = fy + 58.0 + (row - start) as f64 * 18.0;
                        if p.x >= ixf + 12.0 && p.y >= y - 2.0 && p.y <= y + 12.0 {
                            self.apply_font(row);
                            return;
                        }
                    }
                    // weight chips
                    if !self.font_weights.is_empty() {
                        let wy = fy + 58.0 + visible as f64 * 18.0 + 14.0;
                        let mut wx = ixf + 12.0;
                        let mut wrow = wy + 12.0;
                        for i in 0..self.font_weights.len() {
                            let (_, w, italic) = &self.font_weights[i];
                            let text = if *italic {
                                "IT".to_string()
                            } else {
                                format!("{w}")
                            };
                            let cw = x_native::text::measure(&text, 7.5) + 10.0;
                            if wx + cw > self.win_w - 12.0 {
                                wx = ixf + 12.0;
                                wrow += 18.0;
                            }
                            if p.x >= wx
                                && p.x <= wx + cw
                                && p.y >= wrow - 2.0
                                && p.y <= wrow + 12.0
                            {
                                self.apply_font_weight(i);
                                return;
                            }
                            wx += cw + 4.0;
                        }
                    }
                }
            }
        }
        // opacity -/+ buttons
        if let Some(n) = self.selected_single() {
            let id = n.id.clone();
            let op = n.opacity;
            let ix = self.win_w - INSPECTOR_W;
            // Appearance row (mockup): opacity -/+ in the left box,
            // corner radius -/+ in the right box (rects)
            let ry = TOP_H + IY_APP_ROW;
            if p.y >= ry - 3.0 && p.y <= ry + 14.0 {
                if p.x >= ix + 74.0 && p.x <= ix + 90.0 {
                    self.editor.set_opacity(&id, (op - 0.1).max(0.0));
                    self.status = "opacity -".into();
                    return;
                }
                if p.x >= ix + 92.0 && p.x <= ix + 108.0 {
                    self.editor.set_opacity(&id, (op + 0.1).min(1.0));
                    self.status = "opacity +".into();
                    return;
                }
                // radius steppers (right box); for arcs it steers START
                let rad = |app: &mut Self, d: f64| -> Option<String> {
                    let nm = x_native::editor::find_mut(&mut app.editor.root, &id)?;
                    match &mut nm.kind {
                        x_native::NodeKind::Rect { radius } => {
                            *radius = (*radius + d).max(0.0);
                            nm.dirty = true;
                            Some(format!("radius {radius:.0}"))
                        }
                        x_native::NodeKind::Arc { start, .. } => {
                            *start = (*start + d + 360.0) % 360.0;
                            nm.dirty = true;
                            Some(format!("arc start {start:.0}°"))
                        }
                        _ => None,
                    }
                };
                if p.x >= ix + 182.0 && p.x <= ix + 198.0 {
                    if let Some(r) = rad(self, -2.0) {
                        self.status = r;
                    }
                    return;
                }
                if p.x >= ix + 200.0 && p.x <= ix + 216.0 {
                    if let Some(r) = rad(self, 2.0) {
                        self.status = r;
                    }
                    return;
                }
            }
            // per-corner mini boxes (TL TR BR BL): top half = +2, bottom = -2
            let ccy = TOP_H + IY_CORNERS;
            if p.y >= ccy - 3.0 && p.y <= ccy + 14.0 {
                // arcs: single END-angle box in this row (top +15 / bottom -15)
                let is_arc = x_native::editor::find(&self.editor.root, &id)
                    .is_some_and(|n| matches!(n.kind, x_native::NodeKind::Arc { .. }));
                if is_arc {
                    if p.x >= ix + 12.0 && p.x <= ix + 108.0 {
                        let delta = if p.y < ccy + 5.5 { 15.0 } else { -15.0 };
                        if let Some(nm) = x_native::editor::find_mut(&mut self.editor.root, &id) {
                            if let x_native::NodeKind::Arc { end, .. } = &mut nm.kind {
                                *end = (*end + delta + 360.0) % 360.0;
                                nm.dirty = true;
                                self.status = format!("arc end {end:.0}°");
                            }
                        }
                    }
                    return;
                }
                for k in 0..4usize {
                    let bx = ix + 12.0 + k as f64 * 54.0;
                    if p.x >= bx && p.x <= bx + 48.0 {
                        let delta = if p.y < ccy + 5.5 { 2.0 } else { -2.0 };
                        self.adjust_corner(Some(k), delta);
                        return;
                    }
                }
            }
        }
        // palette swatches (one row, mockup compact)
        let y0 = TOP_H + IY_PAL;
        for (i, color) in PALETTE.iter().enumerate() {
            let sx = x0 + i as f64 * 27.0;
            let sy = y0;
            if p.x >= sx && p.x <= sx + 16.0 && p.y >= sy && p.y <= sy + 16.0 {
                if let Some(id) = self.editor.selection.first().cloned() {
                    let stop = self.gradient_stop;
                    let is_gradient = self
                        .selected_single()
                        .map(|n| {
                            let p = n.fill_layers.last().map(|l| &l.paint).unwrap_or(&n.fill);
                            matches!(
                                p,
                                Paint::LinearGradient { .. } | Paint::RadialGradient { .. }
                            )
                        })
                        .unwrap_or(false);
                    if is_gradient {
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(layer) = nm.fill_layers.last_mut() {
                                match &mut layer.paint {
                                    Paint::LinearGradient { stops, .. }
                                    | Paint::RadialGradient { stops, .. }
                                        if stop < stops.len() =>
                                    {
                                        stops[stop].1 = *color
                                    }
                                    _ => {}
                                }
                            }
                        });
                        self.status = format!(
                            "gradient stop {} -> {}",
                            stop + 1,
                            x_native::color_to_hex(*color)
                        );
                    } else {
                        self.editor.set_fill(&id, Paint::Solid(*color));
                        self.status = format!("fill {} -> {}", id, x_native::color_to_hex(*color));
                    }
                }
                return;
            }
        }
        // prototype panel (Prototype tab): starting point + interaction rows
        if self.inspector_tab == 1 {
            if let Some(ui2) = self.prototype_ui() {
                if ui2.start_toggle.contains(p) {
                    self.toggle_starting_point(&ui2.id);
                    return;
                }
                if ui2.add.contains(p) {
                    self.add_interaction(&ui2.id);
                    return;
                }
                for row in &ui2.rows {
                    if row.remove.contains(p) {
                        self.remove_interaction(&ui2.id, row.index);
                        return;
                    }
                    if row.trigger.contains(p) {
                        self.cycle_trigger(&ui2.id, row.index);
                        return;
                    }
                    if row.action.contains(p) {
                        self.cycle_action(&ui2.id, row.index);
                        return;
                    }
                    if row.dest.contains(p) {
                        self.proto_dest_click(&ui2.id, row.index);
                        return;
                    }
                    if row.pos.contains(p) {
                        self.proto_pos_click(&ui2.id, row.index);
                        return;
                    }
                    if row.anim.contains(p) {
                        self.cycle_anim(&ui2.id, row.index);
                        return;
                    }
                    if row.extra.contains(p) {
                        self.proto_extra_click(&ui2.id, row.index);
                        return;
                    }
                }
            }
        }
        // auto layout controls (frames only; mockup's Auto Layout section)
        if let Some(n) = self.selected_single() {
            if matches!(n.kind, x_native::NodeKind::Frame { .. }) {
                let id = n.id.clone();
                let (nw, nh) = (n.w, n.h);
                let ix = self.win_w - INSPECTOR_W;
                let ly = TOP_H + IY_AL_HDR;
                let vars = self.vars.clone();
                // NONE / H / V
                if p.y >= ly + 16.0 && p.y <= ly + 34.0 {
                    for i in 0..3usize {
                        let bx = ix + 12.0 + i as f64 * 52.0;
                        if p.x >= bx && p.x <= bx + 46.0 {
                            let current = self.editor.auto_layout_of(&id);
                            let new_layout = match i {
                                0 => None,
                                _ => {
                                    let mut l = current.clone().unwrap_or(AutoLayout {
                                        gap: 16.0,
                                        padding: [16.0; 4],
                                        align: CrossAlign::Center,
                                        ..Default::default()
                                    });
                                    l.direction = if i == 1 {
                                        LayoutDirection::Horizontal
                                    } else {
                                        LayoutDirection::Vertical
                                    };
                                    Some(l)
                                }
                            };
                            self.editor.set_auto_layout(&id, new_layout, &vars);
                            self.status =
                                format!("layout: {}", ["none", "horizontal", "vertical"][i]);
                            return;
                        }
                    }
                }
                // GAP / PAD steppers
                if let Some(l) = self.editor.auto_layout_of(&id) {
                    for (row, is_gap) in [(0usize, true), (1, false)] {
                        let ry = ly + 44.0 + row as f64 * 22.0;
                        if p.y >= ry - 3.0 && p.y <= ry + 12.0 {
                            let delta = if p.x >= ix + 140.0 && p.x <= ix + 158.0 {
                                -4.0
                            } else if p.x >= ix + 162.0 && p.x <= ix + 180.0 {
                                4.0
                            } else {
                                continue;
                            };
                            let mut nl = l.clone();
                            if is_gap {
                                nl.gap = (nl.gap + delta).max(0.0);
                            } else {
                                nl.padding = nl.padding.map(|p| (p + delta).max(0.0));
                            }
                            self.editor.set_auto_layout(&id, Some(nl.clone()), &vars);
                            self.status = format!(
                                "gap {:.0} pad {}",
                                nl.gap,
                                if nl.uniform_pad() {
                                    format!("{:.0}", nl.padding[0])
                                } else {
                                    format!(
                                        "L{:.0} R{:.0} T{:.0} B{:.0}",
                                        nl.padding[0], nl.padding[1], nl.padding[2], nl.padding[3]
                                    )
                                }
                            );
                            return;
                        }
                    }
                }
                // cross-axis alignment: MIN / CEN / MAX / BASE
                if p.y >= ly + 64.0 && p.y <= ly + 82.0 {
                    if let Some(mut l) = self.editor.auto_layout_of(&id) {
                        for (i, a) in [
                            CrossAlign::Start,
                            CrossAlign::Center,
                            CrossAlign::End,
                            CrossAlign::Baseline,
                        ]
                        .iter()
                        .enumerate()
                        {
                            let bx = ix + 12.0 + i as f64 * 52.0;
                            if p.x >= bx && p.x <= bx + 46.0 {
                                l.align = *a;
                                self.editor.set_auto_layout(&id, Some(l), &vars);
                                self.status = "cross-axis align changed".into();
                                return;
                            }
                        }
                    }
                }
                // MIN/MAX width + height steppers
                if let Some(l) = self.editor.auto_layout_of(&id) {
                    for row in 0..2usize {
                        let ry = ly + 88.0 + row as f64 * 22.0;
                        if p.y >= ry - 2.0 && p.y <= ry + 12.0 {
                            for col in 0..2usize {
                                let lx = ix + 12.0 + col as f64 * 130.0;
                                let delta = if p.x >= lx + 84.0 && p.x <= lx + 100.0 {
                                    -4.0
                                } else if p.x >= lx + 102.0 && p.x <= lx + 118.0 {
                                    4.0
                                } else {
                                    continue;
                                };
                                let mut nl = l.clone();
                                let slot: &mut Option<f64> = match (row, col) {
                                    (0, 0) => &mut nl.min_width,
                                    (0, 1) => &mut nl.max_width,
                                    (1, 0) => &mut nl.min_height,
                                    (1, 1) => &mut nl.max_height,
                                    _ => unreachable!(),
                                };
                                let base = if row == 0 { nw } else { nh };
                                *slot = match (delta > 0.0, *slot) {
                                    (true, Some(v)) => Some(v + 4.0),
                                    (true, None) => Some(base + 4.0),
                                    (false, Some(v)) => Some((v - 4.0).max(0.0)),
                                    (false, None) => None,
                                };
                                self.editor.set_auto_layout(&id, Some(nl), &vars);
                                self.status = "min/max changed".into();
                                return;
                            }
                        }
                    }
                }
                // LAYOUT GRID chips (visual guides)
                {
                    let gy = ly + 92.0;
                    if p.y >= gy - 2.0 && p.y <= gy + 12.0 {
                        let hit = |x0: f64, w: f64| p.x >= ix + x0 && p.x <= ix + x0 + w;
                        let counts = [12usize, 8, 6, 4, 3, 2, 16, 24];
                        let cells = [8.0f64, 4.0, 16.0, 24.0, 32.0];
                        let guts = [20.0f64, 16.0, 12.0, 8.0, 24.0, 0.0];
                        let margs = [20.0f64, 0.0, 40.0, 24.0, 32.0, 48.0];
                        let cur = find(&self.editor.root, &id)
                            .and_then(|n| n.layout_grids.first().copied());
                        let next = if hit(56.0, 64.0) {
                            // pattern cycle: off -> columns -> rows -> grid -> off
                            match cur {
                                None => Some(LayoutGridDef::default()),
                                Some(mut g) => {
                                    if g.pattern == GridPattern::Grid {
                                        if let Some(n) =
                                            x_native::editor::find_mut(&mut self.editor.root, &id)
                                        {
                                            n.layout_grids.clear();
                                        }
                                        self.status = "layout grid off".into();
                                        None
                                    } else {
                                        g.pattern = match g.pattern {
                                            GridPattern::Columns => GridPattern::Rows,
                                            _ => GridPattern::Grid,
                                        };
                                        Some(g)
                                    }
                                }
                            }
                        } else if hit(124.0, 40.0) {
                            match cur {
                                Some(mut g) if g.pattern == GridPattern::Grid => {
                                    g.cell = cells
                                        .iter()
                                        .find(|c| **c > g.cell)
                                        .copied()
                                        .unwrap_or(cells[0]);
                                    Some(g)
                                }
                                Some(mut g) => {
                                    g.count = counts
                                        .iter()
                                        .find(|c| **c > g.count)
                                        .copied()
                                        .unwrap_or(counts[0]);
                                    Some(g)
                                }
                                None => None,
                            }
                        } else if hit(168.0, 40.0) {
                            match cur {
                                Some(mut g) if g.pattern != GridPattern::Grid => {
                                    g.gutter = guts
                                        .iter()
                                        .find(|v| **v < g.gutter)
                                        .copied()
                                        .unwrap_or(guts[0]);
                                    Some(g)
                                }
                                _ => None,
                            }
                        } else if hit(212.0, 44.0) {
                            match cur {
                                Some(mut g) => {
                                    g.margin = margs
                                        .iter()
                                        .find(|v| **v != g.margin && **v > g.margin)
                                        .copied()
                                        .or_else(|| margs.iter().find(|v| **v != g.margin).copied())
                                        .unwrap_or(margs[0]);
                                    Some(g)
                                }
                                None => None,
                            }
                        } else {
                            None
                        };
                        if let Some(g) = next {
                            if let Some(n) = x_native::editor::find_mut(&mut self.editor.root, &id)
                            {
                                if n.layout_grids.is_empty() {
                                    n.layout_grids.push(g);
                                } else {
                                    n.layout_grids[0] = g;
                                }
                            }
                            self.status = format!(
                                "layout grid: {}",
                                match g.pattern {
                                    GridPattern::Columns => format!("{} columns", g.count),
                                    GridPattern::Rows => format!("{} rows", g.count),
                                    GridPattern::Grid => format!("{:.0}px cells", g.cell),
                                }
                            );
                        }
                        return;
                    }
                }
                // SPACE distribution cycle (Packed / Between / Around / Evenly)
                if let Some(mut l) = self.editor.auto_layout_of(&id) {
                    let ry = ly + 108.0;
                    let r = Rect::new(ix + 96.0, ry - 2.0, ix + 190.0, ry + 14.0);
                    if r.contains(p) {
                        l.distribute = match l.distribute {
                            Distribute::Packed => Distribute::Between,
                            Distribute::Between => Distribute::Around,
                            Distribute::Around => Distribute::Evenly,
                            Distribute::Evenly => Distribute::Packed,
                        };
                        let d = l.distribute;
                        self.editor.set_auto_layout(&id, Some(l), &vars);
                        self.status = format!("spacing: {}", d.to_str());
                        return;
                    }
                }
                // OVERFLOW cycle (clip content / scroll) — frames only
                {
                    let ry = ly + 132.0;
                    let r = Rect::new(ix + 96.0, ry - 2.0, ix + 190.0, ry + 14.0);
                    if r.contains(p) {
                        let next = match n.overflow {
                            Overflow::Visible => Overflow::Clip,
                            Overflow::Clip => Overflow::ScrollY,
                            Overflow::ScrollY => Overflow::ScrollX,
                            Overflow::ScrollX => Overflow::ScrollBoth,
                            Overflow::ScrollBoth => Overflow::Visible,
                        };
                        self.editor.set_overflow(&id, next);
                        self.status = format!("overflow: {}", next.label());
                        return;
                    }
                }
            }
            // child auto-layout controls (non-frame child of an auto-layout frame)
            if !matches!(n.kind, x_native::NodeKind::Frame { .. }) {
                let id = n.id.clone();
                let parent_has_layout = parent_id(&self.editor.root, &id)
                    .and_then(|pid| self.editor.auto_layout_of(&pid))
                    .is_some();
                if parent_has_layout {
                    let ix = self.win_w - INSPECTOR_W;
                    let ly = TOP_H + IY_AL_HDR;
                    let vars = self.vars.clone();
                    // ABSOLUTE / FIXED / STICKY toggles
                    for (i, label) in ["ABSOLUTE", "FIXED", "STICKY"].iter().enumerate() {
                        let bx = ix + 12.0 + i as f64 * 74.0;
                        let r = Rect::new(bx, ly + 16.0, bx + 70.0, ly + 34.0);
                        if r.contains(p) {
                            let mut c = self.editor.child_constraints_of(&id).unwrap_or_default();
                            match i {
                                0 => c.is_absolute = !c.is_absolute,
                                1 => c.fixed = !c.fixed,
                                2 => c.sticky = !c.sticky,
                                _ => {}
                            }
                            self.editor.set_child_constraints(&id, c, &vars);
                            self.status = format!("{label} toggled");
                            return;
                        }
                    }
                    // ALIGN SELF: AUTO / MIN / CEN / MAX / BASE
                    if p.y >= ly + 44.0 && p.y <= ly + 62.0 {
                        for (i, a) in [
                            None,
                            Some(Alignment::Min),
                            Some(Alignment::Center),
                            Some(Alignment::Max),
                            Some(Alignment::Baseline),
                        ]
                        .iter()
                        .enumerate()
                        {
                            let bx = ix + 12.0 + i as f64 * 46.0;
                            if p.x >= bx && p.x <= bx + 44.0 {
                                let mut c =
                                    self.editor.child_constraints_of(&id).unwrap_or_default();
                                c.align_self = *a;
                                self.editor.set_child_constraints(&id, c, &vars);
                                self.status = "align self changed".into();
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn gradient_geometry(&self) -> Option<GradientGeom> {
        if !self.gradient_editing || self.editor.selection.len() != 1 {
            return None;
        }
        let id = &self.editor.selection[0];
        let n = find(&self.editor.root, id)?;
        let fill = self
            .fill_layer_index
            .min(n.fill_layers.len().saturating_sub(1));
        let layer = n.fill_layers.get(fill)?;
        let (start, end, stops) = match &layer.paint {
            Paint::LinearGradient {
                start, end, stops, ..
            } => (*start, *end, stops.clone()),
            Paint::RadialGradient {
                center,
                radius,
                stops,
                ..
            } => (*center, (center.0 + *radius, center.1), stops.clone()),
            _ => return None,
        };
        let (world, _, _) = world_transform_of(&self.editor.root, id)?;
        let tx = self.camera() * world;
        Some((
            fill,
            tx * Point::new(start.0, start.1),
            tx * Point::new(end.0, end.1),
            stops,
        ))
    }

    fn gradient_handle_at(&self, p: Point) -> Option<(usize, usize)> {
        let (fill, start, end, stops) = self.gradient_geometry()?;
        let near = |a: Point| {
            let d = a - p;
            d.x.hypot(d.y) <= 9.0
        };
        if near(start) {
            return Some((fill, 0));
        }
        if near(end) {
            return Some((fill, 1));
        }
        for (i, (t, _)) in stops.iter().enumerate().rev() {
            let q = start + (end - start) * *t as f64;
            if near(q) {
                return Some((fill, i + 2));
            }
        }
        None
    }

    fn gradient_line_position(&self, p: Point) -> Option<(usize, f32)> {
        let (fill, start, end, _) = self.gradient_geometry()?;
        let v = end - start;
        let len2 = v.x * v.x + v.y * v.y;
        if len2 <= 0.01 {
            return None;
        }
        let t = (((p.x - start.x) * v.x + (p.y - start.y) * v.y) / len2).clamp(0.0, 1.0);
        let q = start + v * t;
        let d = q - p;
        if d.x.hypot(d.y) <= 8.0 {
            Some((fill, t as f32))
        } else {
            None
        }
    }

    // ------------------------------------------------------------ rendering
}

fn gradient_color_at(stops: &[(f32, Color)], t: f32) -> Color {
    let Some(first) = stops.first() else {
        return Color::TRANSPARENT;
    };
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b.0 {
            let u = if b.0 > a.0 {
                (t - a.0) / (b.0 - a.0)
            } else {
                0.0
            };
            let mixc = |x: f32, y: f32| (x + (y - x) * u).clamp(0.0, 1.0);
            return Color::from_rgba8(
                (mixc(a.1.components[0], b.1.components[0]) * 255.0).round() as u8,
                (mixc(a.1.components[1], b.1.components[1]) * 255.0).round() as u8,
                (mixc(a.1.components[2], b.1.components[2]) * 255.0).round() as u8,
                (mixc(a.1.components[3], b.1.components[3]) * 255.0).round() as u8,
            );
        }
    }
    stops.last().map(|s| s.1).unwrap_or(Color::TRANSPARENT)
}

/// Per-user document dir used when the working directory is not writable
/// (double-clicked binary, read-only install): `~/x-native-files` on every
/// desktop platform — HOME on Unix-likes, USERPROFILE on Windows.
fn user_files_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    if home.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(home).join("x-native-files"))
}
