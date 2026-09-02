#[allow(unused_imports)]
use super::*;

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
            (Point::new(start.x - dx, start.y - dy), Point::new(start.x + dx, start.y + dy))
        } else {
            (start, Point::new(start.x + dx, start.y + dy))
        };
        Rect::new(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y))
    }

    pub fn canvas_origin(&self) -> (f64, f64) {
        if self.chrome_hidden { (0.0, 0.0) } else { (TOOLBAR_W + LAYERS_W, TOP_H) }
    }
    /// effective height of the bottom pages strip (collapsible — Figma
    /// gives the canvas maximum viewport; state persists in .xprefs)
    // Figma has no bottom artboard-thumbnail strip — pages are switched
    // from the simple list at the top of the Layers panel instead, so no
    // vertical space is reserved for a strip anymore.
    pub fn thumbs_h(&self) -> f64 { 0.0 }

    pub fn canvas_rect(&self) -> Rect {
        if self.chrome_hidden { return Rect::new(0.0, 0.0, self.win_w, self.win_h); }
        Rect::new(TOOLBAR_W + LAYERS_W, TOP_H, self.win_w - INSPECTOR_W,
                  self.win_h - self.thumbs_h() - STATUS_H)
    }

    /// bottom page-thumbnail strip (mockup); collapsed = slim toggle bar
    pub fn thumbs_rect(&self) -> Rect {
        Rect::new(LAYERS_W, self.win_h - self.thumbs_h() - STATUS_H, self.win_w - INSPECTOR_W, self.win_h - STATUS_H)
    }

    pub fn toggle_thumbs(&mut self) {
        self.thumbs_collapsed = !self.thumbs_collapsed;
        let _ = std::fs::write(".xprefs", if self.thumbs_collapsed { "thumbs=collapsed" } else { "thumbs=open" });
        self.status = if self.thumbs_collapsed { "pages panel collapsed".into() } else { "pages panel shown".into() };
    }
    /// minimap minimap rect (bottom-right of the canvas).
    pub fn minimap_rect(&self) -> Rect {
        let c = self.canvas_rect();
        Rect::new(c.x1 - 176.0, c.y1 - BOTTOM_BAR_H - 116.0, c.x1 - 12.0, c.y1 - BOTTOM_BAR_H - 12.0)
    }

    /// standard floating toolbar, centered at the bottom of the canvas.
    pub fn bottom_bar_rect(&self) -> Rect {
        // tool row now lives centered in the header's second row (mockup)
        let w = Tool::ALL.len() as f64 * 38.0 + 16.0;
        let cx = self.win_w / 2.0;
        Rect::new(cx - w / 2.0, TAB_H + 6.0, cx + w / 2.0, TOP_H - 6.0)
    }
    pub fn camera(&self) -> Affine {
        let (ox, oy) = self.canvas_origin();
        Affine::translate((ox + self.pan.0, oy + self.pan.1)) * Affine::scale(self.zoom)
    }
    pub fn world_point(&self, screen: Point) -> Point {
        let (ox, oy) = self.canvas_origin();
        Point::new((screen.x - ox - self.pan.0) / self.zoom, (screen.y - oy - self.pan.1) / self.zoom)
    }

    pub fn rebuild_layer_rows(&mut self) {
        // virtualization: skip the full-tree walk when nothing changed
        // (undo depth + selection hash + filter act as the fingerprint)
        let fp = (self.editor.undo_depth(), self.layer_filter.clone(), self.editor.root.children.len());
        if self.layer_rows_fp == Some(fp.clone()) && !self.layer_rows.is_empty() { return; }
        self.layer_rows_fp = Some(fp);
        fn walk(n: &Node, depth: usize, out: &mut Vec<(String, usize, &'static str)>) {
            out.push((n.id.clone(), depth, kind_label(n)));
            for c in &n.children { walk(c, depth + 1, out); }
        }
        let mut rows = vec![];
        walk(&self.editor.root, 0, &mut rows);
        if !self.layer_filter.is_empty() {
            let q = self.layer_filter.to_ascii_lowercase();
            rows.retain(|(id, _, k)| id.to_ascii_lowercase().contains(&q) || k.to_ascii_lowercase().contains(&q));
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
        out.push((String::new(), "SEARCH", Rect::new(ix + 12.0, cy - 2.0, self.win_w - 12.0, cy + 12.0), 2));
        cy += 20.0;
        let q = self.style_query.to_ascii_lowercase();
        let mut names: Vec<&String> = self.styles.keys()
            .filter(|n| q.is_empty() || n.to_ascii_lowercase().contains(&q))
            .collect();
        names.sort();
        for (kind, _header) in [("PAINT", "PAINT STYLES"), ("TEXT", "TEXT STYLES"), ("FX", "EFFECT STYLES")] {
            let group: Vec<&&String> = names.iter().filter(|n| self.styles[n.as_str()].kind_label() == kind).collect();
            if group.is_empty() { continue; }
            // mockup-compact: no group header rows — chips flow inline
            let mut cx = ix + 12.0;
            for name in group.into_iter().take(8) {
                let short = if name.len() > 12 { &name[..12] } else { name.as_str() };
                // chip text includes the usage count ("Primary 3");
                // current page counts from the LIVE editor tree
                let usage: usize = self.pages.iter().enumerate()
                    .map(|(i, p)| if i == self.page_idx { arco_native::style_usage(&self.editor.root, name) } else { arco_native::style_usage(p, name) })
                    .sum();
                let text = if usage > 0 { format!("{short} {usage}") } else { short.to_string() };
                let cw = arco_native::text::measure(&text, 7.0) + 18.0;
                if cx + cw > self.win_w - 8.0 { cx = ix + 12.0; cy += 18.0; }
                out.push(((*name).clone(), kind, Rect::new(cx, cy - 2.0, cx + cw, cy + 12.0), 0));
                cx += cw + 4.0;
            }
            cy += 18.0;
        }
        // management row for the selected style: REN DUP DEL DET
        if let Some(sel) = &self.style_sel {
            if self.styles.contains_key(sel) {
                let mut bx = ix + 12.0;
                for act in ["REN", "DUP", "DEL", "DET"] {
                    out.push((act.to_string(), "ACT", Rect::new(bx, cy - 2.0, bx + 34.0, cy + 12.0), 3));
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
        out.push((String::new(), Rect::new(panel.x0 + 16.0, panel.y0 + 34.0, panel.x1 - 16.0, panel.y0 + 52.0), 1));
        // action row (operates on asset_sel) + sort chips
        let mut bx = panel.x0 + 16.0;
        for act in ["PLACE", "REPLACE", "RENAME", "DEL UNUSED"] {
            let w = arco_native::text::measure(act, 8.0) + 16.0;
            out.push((act.to_string(), Rect::new(bx, panel.y0 + 58.0, bx + w, panel.y0 + 74.0), 2));
            bx += w + 8.0;
        }
        for (i, srt) in ["NAME", "SIZE", "USED"].iter().enumerate() {
            let w = arco_native::text::measure(srt, 7.5) + 12.0;
            out.push((format!("SORT{i}"), Rect::new(bx, panel.y0 + 58.0, bx + w, panel.y0 + 74.0), 2));
            bx += w + 6.0;
        }
        // thumbnail grid: filtered, SORTED (name/size/usage), SCROLLED
        let recs = self.sorted_assets();
        let cell = 120.0;
        let cols = ((panel.width() - 32.0) / (cell + 10.0)).floor().max(1.0) as usize;
        let row_h = cell * 0.75 + 26.0;
        let visible_rows = ((panel.y1 - 10.0 - (panel.y0 + 86.0)) / row_h).floor().max(1.0) as usize;
        let start = self.asset_scroll * cols;
        for (i, id) in recs.iter().enumerate().skip(start).take(cols * visible_rows) {
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
        let mut recs: Vec<&arco_native::AssetRecord> = self.store.iter_sorted().into_iter()
            .filter(|r| q.is_empty() || r.name.to_ascii_lowercase().contains(&q) || r.mime.contains(&q))
            .collect();
        match self.asset_sort {
            1 => recs.sort_by(|a, b| b.bytes.len().cmp(&a.bytes.len()).then(a.id.cmp(&b.id))),
            2 => {
                let usage = |id: &str| -> usize {
                    self.pages.iter().enumerate()
                        .map(|(i, p)| if i == self.page_idx { arco_native::asset_usage(&self.editor.root, id) } else { arco_native::asset_usage(p, id) })
                        .sum()
                };
                recs.sort_by(|a, b| usage(&b.id).cmp(&usage(&a.id)).then(a.id.cmp(&b.id)));
            }
            _ => recs.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id))),
        }
        recs.into_iter().map(|r| (r.name.clone(), r.id.clone())).collect()
    }

    /// Asset browser actions (asset_sel target).
    pub fn run_asset_action(&mut self, act: &str) {
        match act {
            "PLACE" => {
                let Some(id) = self.asset_sel.clone() else { self.status = "select an asset tile first".into(); return };
                let dims = self.store.get(&id).and_then(|r| r.dimensions).unwrap_or((160, 120));
                self.created_count += 1;
                let nid = format!("image-{}", self.created_count);
                let mut n = Node::image(&nid, 60.0, 60.0, dims.0 as f64, dims.1 as f64, &id);
                n.transform.x = 60.0; n.transform.y = 60.0;
                let root_id = self.editor.root.id.clone();
                self.editor.insert_node(&root_id, n);
                self.editor.selection = vec![nid.clone()];
                self.asset_browser = false;
                self.status = format!("placed {nid} on canvas");
            }
            "REPLACE" => {
                let Some(id) = self.asset_sel.clone() else { self.status = "select an asset tile first".into(); return };
                let Some(sel) = self.editor.selection.first().cloned() else { self.status = "select an image layer on canvas first".into(); return };
                if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &sel) {
                    if let arco_native::NodeKind::Image { asset, .. } = &mut nm.kind {
                        *asset = id.clone();
                        nm.dirty = true;
                        self.status = format!("{sel} now uses {}", &id[..24.min(id.len())]);
                        return;
                    }
                }
                self.status = "selected layer is not an image".into();
            }
            "RENAME" => {
                let Some(id) = self.asset_sel.clone() else { self.status = "select an asset tile first".into(); return };
                // rename polish: start EMPTY (select-all semantics) — typing
                // replaces the old name instead of appending to it
                self.focus = Focus::AssetRename { id, buffer: String::new() };
                self.status = "type the new display name, Enter to commit (empty = keep)".into();
            }
            "DEL UNUSED" => {
                let mut used = std::collections::HashSet::new();
                arco_native::collect_asset_ids(&self.editor.root, &mut used);
                for (i, p) in self.pages.iter().enumerate() {
                    if i != self.page_idx { arco_native::collect_asset_ids(p, &mut used); }
                }
                let dropped = self.store.retain_used(&used);
                if self.asset_sel.as_deref().is_some_and(|s| self.store.get(s).is_none()) { self.asset_sel = None; }
                self.status = format!("deleted {dropped} unused asset(s)");
            }
            _ => {}
        }
    }

    /// Clicks inside the asset browser overlay.
    pub fn click_asset_browser(&mut self, p: Point) {
        let panel = self.asset_panel_rect();
        if !panel.contains(p) { self.asset_browser = false; return; }
        for (tag, r, kind) in self.asset_layout() {
            if !r.contains(p) { continue; }
            match kind {
                1 => { self.focus = Focus::AssetSearch; self.status = "type to filter assets".into(); }
                2 => {
                    if let Some(srt) = tag.strip_prefix("SORT") {
                        self.asset_sort = srt.parse().unwrap_or(0);
                        self.asset_scroll = 0;
                        self.status = format!("sorted by {}", ["name", "size", "usage"][self.asset_sort as usize]);
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
                        Some(rec) => format!("{} | {} | {}x{} | used {}x", rec.name, rec.mime,
                            rec.dimensions.map(|d| d.0).unwrap_or(0), rec.dimensions.map(|d| d.1).unwrap_or(0),
                            self.pages.iter().enumerate().map(|(i, pg)| if i == self.page_idx { arco_native::asset_usage(&self.editor.root, &tag) } else { arco_native::asset_usage(pg, &tag) }).sum::<usize>()),
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
        out.push(("LIBRARIES".into(), Rect::new(ix + 12.0, iy - 3.0, ix + 80.0, iy + 11.0), 0));
        out.push(("LINK .XLIB".into(), Rect::new(ix + 84.0, iy - 3.0, ix + 146.0, iy + 11.0), 1));
        out.push(("CHECK UPD".into(), Rect::new(ix + 150.0, iy - 3.0, ix + 212.0, iy + 11.0), 2));
        let mut y = iy + 22.0;
        for dep in &self.library_deps {
            let Some(lib) = self.library_snapshots.get(&dep.library_id) else { continue };
            out.push((lib.name.clone(), Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 24.0), 0));
            if let Some((_, newer, _)) = &self.library_update {
                if newer.library_id == dep.library_id {
                    out.push((dep.library_id.clone(), Rect::new(ix + 12.0, y + 26.0, self.win_w - 12.0, y + 44.0), 3));
                    y += 22.0;
                }
            }
            y += 30.0;
            out.push(("STYLES".into(), Rect::new(ix + 12.0, y - 2.0, ix + 80.0, y + 10.0), 5));
            y += 13.0;
            let mut names: Vec<&String> = lib.styles.keys().collect();
            names.sort();
            for nm in names.iter().take(5) {
                out.push((format!("  {nm}"), Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0), 5));
                y += 12.0;
            }
            let vars_n = lib.variables.colors.len() + lib.variables.numbers.len();
            if vars_n > 0 {
                out.push((format!("VARIABLES ({vars_n})"), Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0), 5));
                y += 13.0;
            }
            if !lib.components.is_empty() {
                out.push(("COMPONENTS — CLICK TO PLACE".into(), Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0), 5));
                y += 13.0;
                for c in lib.components.iter().take(6) {
                    if let arco_native::NodeKind::Component { name } = &c.kind {
                        let w = arco_native::text::measure(name, 8.0) + 18.0;
                        out.push((format!("{}|{name}", dep.library_id), Rect::new(ix + 12.0, y - 2.0, ix + 12.0 + w, y + 12.0), 4));
                        y += 18.0;
                    }
                }
            }
            if lib.assets.len() > 0 {
                out.push((format!("ASSETS ({}) — SHIFT+A BROWSER", lib.assets.len()), Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 10.0), 5));
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
        match arco_native::fileio::load_xlib(&text) {
            Ok(lib) => {
                if self.library_deps.iter().any(|d| d.library_id == lib.library_id) {
                    self.status = format!("{} already linked — use CHECK UPDATES", lib.name);
                    return;
                }
                // library assets flow into the document store (content-
                // addressed: dedup is automatic) and the render cache
                for rec in lib.assets.iter_sorted() {
                    self.store.register(&rec.name, rec.bytes.clone(), rec.source);
                }
                self.assets.sync_store(&self.store);
                let dep = arco_native::LibraryDependency {
                    library_id: lib.library_id.clone(),
                    resolved_version: lib.version,
                    snapshot_hash: arco_native::fileio::library_hash(&lib),
                    source_path: "library.xlib".into(),
                };
                self.status = format!("linked {} v{} ({} styles, {} components)",
                    lib.name, lib.version, lib.styles.len(), lib.components.len());
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
            let Ok(text) = std::fs::read_to_string(&dep.source_path) else { continue };
            let Ok(newer) = arco_native::fileio::load_xlib(&text) else { continue };
            if newer.library_id == dep.library_id && newer.version > dep.resolved_version {
                let pinned = &self.library_snapshots[&dep.library_id];
                let changes = arco_native::diff_library(pinned, &newer);
                self.status = format!("update available: {} v{} -> v{} ({} change(s))",
                    newer.name, dep.resolved_version, newer.version, changes.len());
                self.library_update = Some((i, newer, changes));
                return;
            }
        }
        self.status = "all libraries up to date".into();
    }

    /// Accept the staged update — a straight call into the engine's
    /// accept_update (repins + swaps snapshot + re-resolves consumers).
    pub fn accept_library_update(&mut self) {
        let Some((idx, newer, _)) = self.library_update.take() else { return };
        self.library_review = false;
        let dep = &mut self.library_deps[idx];
        // pages incl. the live editor tree
        let mut all: Vec<Node> = vec![self.editor.root.clone()];
        for (i, p) in self.pages.iter().enumerate() {
            if i != self.page_idx { all.push(p.clone()); }
        }
        let new_hash = arco_native::fileio::library_hash(&newer);
        let (changes, updated) = arco_native::accept_update(dep, &mut self.library_snapshots, &mut all, newer);
        dep.snapshot_hash = new_hash;
        let mut it = all.into_iter();
        self.editor.root = it.next().unwrap();
        for (i, p) in self.pages.iter_mut().enumerate() {
            if i != self.page_idx { if let Some(np) = it.next() { *p = np; } }
        }
        self.status = format!("accepted v{}: {} change(s), {updated} consumer(s) updated",
            dep.resolved_version, changes.len());
    }

    /// Place an instance of a LIBRARY component: the master is added to
    /// the page's component registry ONCE (hidden), instances reference it
    /// by name — same dependency semantics as styles, no per-instance clone.
    pub fn place_library_component(&mut self, lib_id: &str, comp_name: &str) {
        let Some(lib) = self.library_snapshots.get(lib_id) else { return };
        let Some(master) = lib.components.iter().find(|c|
            matches!(&c.kind, arco_native::NodeKind::Component { name } if name == comp_name)) else { return };
        // registry: one hidden master per (library, component), stable id
        let reg_id = format!("libmaster-{lib_id}-{comp_name}");
        if arco_native::editor::find(&self.editor.root, &reg_id).is_none() {
            let mut m = master.clone();
            m.id = reg_id.clone();
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

    /// Style management actions (REN/DUP/DEL/DET) for `style_sel`.
    pub fn run_style_action(&mut self, act: &str) {
        let Some(name) = self.style_sel.clone() else { return };
        if !self.styles.contains_key(&name) { return; }
        match act {
            "REN" => {
                self.focus = Focus::StyleRename { from: name.clone(), buffer: name.clone() };
                self.status = "type the new name, Enter to rename every consumer".into();
            }
            "DUP" => {
                let mut copy = format!("{name} copy");
                let mut n = 2;
                while self.styles.contains_key(&copy) { copy = format!("{name} copy {n}"); n += 1; }
                let def = self.styles[&name].clone();
                self.styles.insert(copy.clone(), def);
                self.style_sel = Some(copy.clone());
                self.status = format!("duplicated -> {copy}");
            }
            "DEL" => {
                // deleting detaches every consumer first (values keep)
                let mut detached = 0usize;
                fn detach_all(n: &mut Node, name: &str, detached: &mut usize) {
                    for (k, _) in arco_native::STYLE_BINDING_KEYS {
                        if n.bindings.get(k).map(String::as_str) == Some(name) {
                            n.bindings.remove(k);
                            *detached += 1;
                        }
                    }
                    for c in &mut n.children { detach_all(c, name, detached); }
                }
                detach_all(&mut self.editor.root, &name, &mut detached);
                for (i, pg) in self.pages.iter_mut().enumerate() {
                    if i != self.page_idx { detach_all(pg, &name, &mut detached); }
                }
                self.styles.remove(&name);
                self.style_sel = None;
                self.status = format!("deleted {name} ({detached} consumer(s) detached, values kept)");
            }
            "DET" => {
                // detach the SELECTED LAYER from this style only
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                        let mut done = false;
                        for (k, _) in arco_native::STYLE_BINDING_KEYS {
                            if nm.bindings.get(k).map(String::as_str) == Some(name.as_str()) {
                                done |= arco_native::detach_style(nm, k);
                            }
                        }
                        self.status = if done { format!("{id} detached from {name} (values kept)") }
                            else { format!("{id} was not bound to {name}") };
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
            if i != self.page_idx { all.push(p.clone()); }
        }
        match arco_native::rename_style(&mut self.styles, &mut all, from, to) {
            Some(rebound) => {
                let mut it = all.into_iter();
                self.editor.root = it.next().unwrap();
                let mut rest = it;
                for (i, p) in self.pages.iter_mut().enumerate() {
                    if i != self.page_idx { if let Some(np) = rest.next() { *p = np; } }
                }
                self.style_sel = Some(to.to_string());
                self.status = format!("renamed {from} -> {to} ({rebound} consumer(s) rebound)");
            }
            None => self.status = format!("rename refused (empty/duplicate name?)"),
        }
    }

    pub fn selected_single(&self) -> Option<&Node> {
        if self.editor.selection.len() == 1 { find(&self.editor.root, &self.editor.selection[0]) } else { None }
    }

    /// Selection AABB in SCREEN space (for handles).
    pub fn selection_screen_bounds(&self) -> Option<Rect> {
        let id = self.editor.selection.first()?;
        let (world, w, h) = world_transform_of(&self.editor.root, id)?;
        Some(quad_bounds(self.camera() * world, w, h))
    }

    /// handle model: 0-3 = corners (TL,TR,BL,BR), 4=left edge,
    /// 5=right, 6=top, 7=bottom. Corners win over edges.
    pub fn handle_at(&self, p: Point) -> Option<u8> {
        let b = self.selection_screen_bounds()?;
        let corners = [(b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)];
        for (i, (cx, cy)) in corners.iter().enumerate() {
            if (p.x - cx).abs() <= 6.0 && (p.y - cy).abs() <= 6.0 { return Some(i as u8); }
        }
        // edges: within 4px of the line, between the corner zones
        let inside_y = p.y > b.y0 + 8.0 && p.y < b.y1 - 8.0;
        let inside_x = p.x > b.x0 + 8.0 && p.x < b.x1 - 8.0;
        if inside_y && (p.x - b.x0).abs() <= 4.0 { return Some(4); }
        if inside_y && (p.x - b.x1).abs() <= 4.0 { return Some(5); }
        if inside_x && (p.y - b.y0).abs() <= 4.0 { return Some(6); }
        if inside_x && (p.y - b.y1).abs() <= 4.0 { return Some(7); }
        None
    }

    /// rotation mode: no visible knob — an invisible hotspot in the ring
    /// JUST OUTSIDE each corner (8..24px out, beyond the resize square).
    pub fn rotate_handle_at(&self, p: Point) -> bool {
        let Some(b) = self.selection_screen_bounds() else { return false };
        let outside = p.x < b.x0 - 4.0 || p.x > b.x1 + 4.0 || p.y < b.y0 - 4.0 || p.y > b.y1 + 4.0;
        if !outside { return false; }
        for (cx, cy) in [(b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)] {
            let d = ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt();
            if d > 6.0 && d <= 24.0 { return true; }
        }
        false
    }

    // ---------------------------------------------------------- text focus

    pub fn commit_focus(&mut self) {
        match std::mem::replace(&mut self.focus, Focus::None) {
            Focus::TextNode { id, buffer, original, .. } => {
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
                                let (dx, dy) = if field == 0 { (v - n.transform.x, 0.0) } else { (0.0, v - n.transform.y) };
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
                self.status = if self.editor.rename_node(&id, name) { format!("renamed {id} → {name}") } else { "rename refused: empty or duplicate name".into() };
            }
            Focus::FontSearch => {}
            Focus::StyleSearch => {}
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
            Focus::None => {}
        }
    }

    /// Tab in a numeric field: commit, then focus the next field (X->Y->W->H).
    pub fn focus_next_field(&mut self) {
        if let Focus::Field { id, field, .. } = self.focus.clone() {
            self.commit_focus();
            let next = (field + 1) % 4;
            self.focus = Focus::Field { id, field: next, buffer: String::new() };
            self.status = format!("type new {} (Tab cycles)", ["X", "Y", "W", "H"][next as usize]);
        }
    }

    pub fn cancel_focus(&mut self) {
        if let Focus::TextNode { id, original, .. } = &self.focus {
            let id = id.clone(); let orig = original.clone();
            if orig.trim().is_empty() {
                // freshly created, never had real content — Esc discards it
                // instead of leaving an empty layer, same as commit does.
                self.editor.selection = vec![id];
                self.editor.delete_selection();
            } else if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                // restore original content directly (no undo entry for the cancel)
                if let arco_native::NodeKind::Text { text } = &mut n.kind { *text = orig; }
            }
        }
        self.focus = Focus::None;
    }

    // ---------------------------------------------------------------- pages

    pub fn switch_page(&mut self, idx: usize) {
        if idx >= self.pages.len() || idx == self.page_idx { return; }
        self.commit_focus();
        self.pages[self.page_idx] = self.editor.root.clone();
        self.page_idx = idx;
        self.editor = Editor::new(self.pages[idx].clone());
        self.status = format!("page: {}", self.pages[idx].id);
    }

    // ------------------------------------------- dashboard <-> editor
    // standard lifecycle: Home (recent files) -> open file -> editor ->
    // back to Home (auto-saves; card thumbnail + modified time update).

    /// Scan persistent documents: ./document.x plus ./files/*.x
    pub fn scan_dash_files(&mut self) {
        let mut out: Vec<DashFile> = vec![];
        let mut paths = vec!["document.x".to_string()];
        if let Ok(rd) = std::fs::read_dir("files") {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "x") {
                    paths.push(p.to_string_lossy().to_string());
                }
            }
        }
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let (d2, _) = arco_native::fileio::load_x_lenient(&text);
            if d2.doc.pages.is_empty() { continue; }
            let name = if d2.metadata.name.is_empty() || d2.metadata.name == "X Native document" {
                if path == "document.x" { "Brand Dashboard".to_string() }
                else { std::path::Path::new(&path).file_stem().unwrap_or_default().to_string_lossy().to_string() }
            } else { d2.metadata.name.clone() };
            let modified = std::fs::metadata(&path).ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .map(|e| {
                    let s = e.as_secs();
                    if s < 60 { "just now".to_string() }
                    else if s < 3600 { format!("{} min ago", s / 60) }
                    else if s < 86400 { format!("{} hr ago", s / 3600) }
                    else { format!("{} day(s) ago", s / 86400) }
                }).unwrap_or_default();
            // real IR thumbnail of page 1
            let pg = &d2.doc.pages[0];
            let tree = arco_native::build_render_tree(pg, &d2.doc.variables);
            let (thumb, _) = arco_native::thumbnail_scene(&tree, pg.w.max(1.0), pg.h.max(1.0), 216.0, 130.0);
            out.push(DashFile { path, name, modified, pages: d2.doc.pages.len(), thumb: Some(thumb) });
        }
        self.dash_files = out;
    }

    /// Open a document from the dashboard into the editor.
    pub fn open_file(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else {
            self.status = format!("can't read {path}");
            return;
        };
        let (d2, notes) = arco_native::fileio::load_x_lenient(&text);
        if d2.doc.pages.is_empty() { self.status = "file has no pages".into(); return; }
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
        arco_native::resolve_styles(&mut self.editor.root, &self.styles);
        self.rebuild_layer_rows();
        self.dirty_since_save = false;
        self.saved_undo_depth = self.editor.undo_depth();
        self.screen = Screen::Editor;
        self.scene_cache = arco_native::FrameCache::new();
        self.status = if notes.is_empty() { format!("opened {path}") } else { format!("opened {path} ({} recovery note(s))", notes.len()) };
        arco_native::fileio::push_recent(path);
    }

    /// Create a fresh document under files/ and open it.
    pub fn new_file(&mut self) {
        let _ = std::fs::create_dir_all("files");
        let mut n = 1;
        let path = loop {
            let p = format!("files/untitled-{n}.x");
            if !std::path::Path::new(&p).exists() { break p; }
            n += 1;
        };
        let mut d = Document::new();
        d.pages.push(Node::frame("page-1", 1600.0, 1000.0));
        let mut d2 = arco_native::fileio::DocumentV2::default();
        d2.metadata.name = format!("Untitled {n}");
        d2.doc = d;
        let _ = arco_native::fileio::atomic_write(&path, arco_native::fileio::save_x_v2(&d2).as_bytes());
        self.open_file(&path);
    }

    /// Adjust one corner radius (0=TL 1=TR 2=BR 3=BL) or all (idx None).
    /// Promotes uniform radius -> corner_radii[4] on first per-corner edit.
    pub fn adjust_corner(&mut self, idx: Option<usize>, delta: f64) {
        let Some(id) = self.editor.selection.first().cloned() else { return };
        if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
            if let arco_native::NodeKind::Rect { radius } = &mut n.kind {
                match idx {
                    None => {
                        // uniform edit clears per-corner overrides
                        n.corner_radii = None;
                        *radius = (*radius + delta).max(0.0);
                        n.dirty = true;
                        self.status = format!("radius {:.0}", *radius);
                    }
                    Some(k) => {
                        let mut c = n.corner_radii.unwrap_or([*radius; 4]);
                        c[k] = (c[k] + delta).max(0.0);
                        n.corner_radii = Some(c);
                        n.dirty = true;
                        self.status = format!("corners {:.0}/{:.0}/{:.0}/{:.0}", c[0], c[1], c[2], c[3]);
                    }
                }
            }
        }
    }

    /// Move the current page left/right in the page order (reorder).
    pub fn reorder_page(&mut self, dir: i32) {
        let i = self.page_idx;
        let j = if dir < 0 { i.checked_sub(1) } else { if i + 1 < self.pages.len() { Some(i + 1) } else { None } };
        let Some(j) = j else { self.status = "page already at the edge".into(); return };
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
        match arco_native::fileio::import_svg(&text) {
            Ok(mut root) => {
                // place at the cursor's world point, keep nodes editable
                let wp = if self.canvas_rect().contains(self.cursor) {
                    self.world_point(self.cursor)
                } else { Point::new(60.0, 60.0) };
                let tag = format!("svgpaste{}", self.editor.undo_depth());
                fn resuffix(n: &mut Node, tag: &str) {
                    n.id = format!("{}-{}", n.id, tag);
                    for c in &mut n.children { resuffix(c, tag); }
                }
                resuffix(&mut root, &tag);
                root.transform.x = wp.x;
                root.transform.y = wp.y;
                let count = { fn c(n: &Node) -> usize { 1 + n.children.iter().map(c).sum::<usize>() } c(&root) };
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
        if self.editor.selection.is_empty() { self.status = "nothing selected".into(); return; }
        // wrap the selected subtrees in a temp frame sized to their bounds
        let mut min_x = f64::MAX; let mut min_y = f64::MAX;
        let mut max_x = f64::MIN; let mut max_y = f64::MIN;
        let mut picked = vec![];
        for id in &self.editor.selection {
            if let Some(n) = find(&self.editor.root, id) {
                min_x = min_x.min(n.transform.x); min_y = min_y.min(n.transform.y);
                max_x = max_x.max(n.transform.x + n.w); max_y = max_y.max(n.transform.y + n.h);
                picked.push(n.clone());
            }
        }
        if picked.is_empty() { return; }
        let mut frame = Node::frame("clip", (max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
        for mut n in picked {
            n.transform.x -= min_x; n.transform.y -= min_y;
            frame = frame.child(n);
        }
        let outliner = arco_native::svg_text_outliner(&self.fonts);
        let svg = arco_native::fileio::export_svg_full(&frame, &self.vars, None, Some(&outliner));
        crate::os_clipboard_set(&svg);
        self.status = format!("copied {} object(s) as SVG to OS clipboard", self.editor.selection.len());
    }

    /// Rename a file's display name (metadata; the .x path is stable).
    pub fn commit_dash_rename(&mut self, path: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() { self.status = "rename cancelled".into(); return; }
        let Ok(text) = std::fs::read_to_string(path) else { self.status = "file unreadable".into(); return };
        let (mut d2, _) = arco_native::fileio::load_x_lenient(&text);
        d2.metadata.name = name.to_string();
        let out = arco_native::fileio::save_x_v2(&d2);
        let _ = arco_native::fileio::atomic_write(path, out.as_bytes());
        self.scan_dash_files();
        self.status = format!("renamed to {name}");
    }

    pub fn duplicate_dash_file(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else { return };
        let (mut d2, _) = arco_native::fileio::load_x_lenient(&text);
        d2.metadata.name = format!("{} copy", d2.metadata.name);
        let _ = std::fs::create_dir_all("files");
        let mut n = 1;
        let new_path = loop {
            let p = format!("files/copy-{n}.x");
            if !std::path::Path::new(&p).exists() { break p; }
            n += 1;
        };
        let _ = arco_native::fileio::atomic_write(&new_path, arco_native::fileio::save_x_v2(&d2).as_bytes());
        self.scan_dash_files();
        self.status = format!("duplicated -> {new_path}");
    }

    pub fn delete_dash_file(&mut self, path: &str) {
        if path == "document.x" { self.status = "Brand Dashboard is protected — duplicate it instead".into(); return; }
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
        if idx >= self.pages.len() { return; }
        self.focus = Focus::PageRename { idx, buffer: String::new() };
        self.status = format!("rename page '{}' — type new name, Enter commits, Esc cancels", self.pages[idx].id);
    }

    pub fn commit_page_rename(&mut self, idx: usize, name: &str) {
        let name = name.trim();
        if name.is_empty() || idx >= self.pages.len() { self.status = "rename cancelled".into(); return; }
        let old = self.pages[idx].id.clone();
        self.pages[idx].id = name.to_string();
        if idx == self.page_idx { self.editor.root.id = name.to_string(); }
        self.dirty_since_save = true;
        self.status = format!("page renamed: {old} -> {name}");
    }

    pub fn delete_page(&mut self, idx: usize) {
        if self.pages.len() <= 1 { self.status = "can't delete the last page".into(); return; }
        if idx >= self.pages.len() { return; }
        let name = self.pages[idx].id.clone();
        self.pages.remove(idx);
        if self.page_idx >= self.pages.len() { self.page_idx = self.pages.len() - 1; }
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
        // ids inside must be unique — suffix every node id
        fn resuffix(n: &mut Node, tag: &str) {
            n.id = format!("{}-{}", n.id, tag);
            for c in &mut n.children { resuffix(c, tag); }
        }
        let tag = format!("d{}", self.pages.len());
        for c in &mut copy.children { resuffix(c, &tag); }
        self.pages.insert(self.page_idx + 1, copy);
        self.switch_page(self.page_idx + 1);
        self.dirty_since_save = true;
        self.status = "page duplicated".into();
    }

    // --------------------------------------------- clipboard (standard keys)
    pub fn clipboard_copy(&mut self) {
        let n = self.editor.selection.len();
        if n == 0 { self.status = "nothing selected to copy".into(); return; }
        self.editor.copy();
        self.status = format!("copied {n} object(s)");
    }

    pub fn clipboard_cut(&mut self) {
        let n = self.editor.selection.len();
        if n == 0 { self.status = "nothing selected to cut".into(); return; }
        self.editor.cut();
        self.status = format!("cut {n} object(s)");
    }

    pub fn clipboard_paste(&mut self) {
        if self.editor.clipboard_len() == 0 { self.status = "clipboard empty".into(); return; }
        // standard behavior: paste into the top-level FRAME under the cursor
        // (when hovering one); otherwise the page root.
        let mut parent = self.editor.root.id.clone();
        if self.canvas_rect().contains(self.cursor) {
            let wp = self.world_point(self.cursor);
            for child in &self.editor.root.children {
                if !matches!(child.kind, arco_native::NodeKind::Frame { .. }) { continue; }
                let r = Rect::new(child.transform.x, child.transform.y,
                                  child.transform.x + child.w, child.transform.y + child.h);
                if r.contains(wp) { parent = child.id.clone(); }
            }
        }
        let into_frame = parent != self.editor.root.id;
        let ids = self.editor.paste(&parent, (16.0, 16.0));
        self.editor.selection = ids.clone();
        self.status = if into_frame {
            format!("pasted {} object(s) into {parent}", ids.len())
        } else {
            format!("pasted {} object(s)", ids.len())
        };
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
        self.present = Some(Present { current: self.page_idx, transition: None });
        self.status = "PRESENTING — click advances, Esc exits".into();
    }

    pub fn present_click(&mut self, p: Point) {
        let Some(pr) = &self.present else { return };
        if pr.transition.is_some() { return; }
        let current = pr.current;
        // map screen point back into page coordinates (same fit as rendering)
        let page = &self.pages[current];
        let scale = (self.win_w / page.w.max(1.0)).min(self.win_h / page.h.max(1.0));
        let ox = (self.win_w - page.w * scale) / 2.0;
        let oy = (self.win_h - page.h * scale) / 2.0;
        let wp = Point::new((p.x - ox) / scale, (p.y - oy) / scale);
        // hit a node with a prototype link? navigate to its destination page
        let mut target: Option<(usize, u32)> = None;
        if let Some(hit_id) = arco_native::editor::hit_test(page, wp) {
            // walk up ancestors for the nearest prototype action
            fn proto_for<'a>(n: &'a Node, target: &str) -> Option<&'a arco_native::PrototypeAction> {
                if n.id == target { return n.prototype.as_ref(); }
                for c in &n.children {
                    if let Some(a) = proto_for(c, target) { return Some(a); }
                    if arco_native::editor::find(c, target).is_some() {
                        return c.prototype.as_ref().or_else(|| proto_for(c, target));
                    }
                }
                None
            }
            if let Some(act) = proto_for(page, &hit_id) {
                if let Some(idx) = self.pages.iter().position(|pg| pg.id == act.destination) {
                    target = Some((idx, act.transition_ms.max(80)));
                }
            }
        }
        // fallback: click-anywhere advances to the next page
        let (next, ms) = target.unwrap_or(((current + 1) % self.pages.len(), 350));
        if next != current {
            if let Some(pr) = &mut self.present {
                pr.transition = Some((current, next, std::time::Instant::now(), ms));
            }
        }
    }

    /// The frame to draw while presenting (owned; may be an interpolation).
    pub fn present_frame(&mut self) -> Option<Node> {
        let pr = self.present.as_mut()?;
        if let Some((from, to, started, ms)) = pr.transition {
            let t = started.elapsed().as_millis() as f64 / ms as f64;
            if t >= 1.0 {
                pr.current = to;
                pr.transition = None;
            } else {
                // ease-in-out
                let te = if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 };
                return Some(arco_native::editor::smart_animate(&self.pages[from], &self.pages[to], te));
            }
        }
        Some(self.pages[pr.current].clone())
    }

    pub fn mouse_down(&mut self, p: Point) {
        let cmd_t0 = std::time::Instant::now();
        let r = self.mouse_down_inner(p);
        self.last_cmd = Some(("click".into(), cmd_t0.elapsed().as_secs_f32() * 1000.0));
        r
    }

    fn mouse_down_inner(&mut self, p: Point) {
        if self.present.is_some() { self.present_click(p); return; }
        let double = self.last_click.elapsed().as_millis() < 400
            && (p - self.last_click_pos).hypot() < 6.0;
        self.last_click = std::time::Instant::now();
        self.last_click_pos = p;
        self.dbl = double;
        // ---------- dashboard screen swallows all clicks ----------
        if self.screen == Screen::Dashboard {
            if self.focus != Focus::None { self.commit_focus(); }
            for (tag, r, kind) in self.dash_layout() {
                if !r.contains(p) { continue; }
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
                                self.focus = Focus::DashRename { path: tag.clone(), buffer: String::new() };
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
        if self.focus != Focus::None { self.commit_focus(); }

        // help overlay swallows clicks
        if self.help_open { self.help_open = false; return; }
        // asset browser overlay swallows clicks
        if self.asset_browser { self.click_asset_browser(p); return; }
        // import preview overlay: Accept / Cancel
        if self.import_pending.is_some() {
            let panel = Rect::new(self.win_w / 2.0 - 260.0, self.win_h / 2.0 - 190.0,
                                  self.win_w / 2.0 + 260.0, self.win_h / 2.0 + 190.0);
            let acc = Rect::new(panel.x0 + 20.0, panel.y1 - 40.0, panel.x0 + 110.0, panel.y1 - 16.0);
            let can = Rect::new(panel.x0 + 120.0, panel.y1 - 40.0, panel.x0 + 210.0, panel.y1 - 16.0);
            if acc.contains(p) {
                if let Some((src, d, report)) = self.import_pending.take() {
                    let count = d.pages.len();
                    for rec in d.assets.iter_sorted() {
                        self.store.register(&rec.name, rec.bytes.clone(), rec.source);
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
            let panel = Rect::new(self.win_w / 2.0 - 220.0, self.win_h / 2.0 - 160.0,
                                  self.win_w / 2.0 + 220.0, self.win_h / 2.0 + 160.0);
            let acc = Rect::new(panel.x0 + 20.0, panel.y1 - 40.0, panel.x0 + 110.0, panel.y1 - 16.0);
            let can = Rect::new(panel.x0 + 120.0, panel.y1 - 40.0, panel.x0 + 210.0, panel.y1 - 16.0);
            if acc.contains(p) { self.accept_library_update(); return; }
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
                        self.last_cmd = Some((label_.to_ascii_lowercase(), t0.elapsed().as_secs_f32() * 1000.0));
                        return;
                    }
                }
                // click on another menu title switches; elsewhere closes
                for (i, r) in self.menu_title_rects() {
                    if r.contains(p) {
                        self.menu_open = if self.menu_open == Some(i) { None } else { Some(i) };
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
            if bar.contains(p) { self.click_bottom_bar(p); return; }
            // header row 2: zoom widget + Present button
            if p.y >= TAB_H && p.y < TOP_H {
                let (bm, bl, bp, ppr, pr) = self.header_rects();
                if bm.contains(p) { self.zoom = (self.zoom / 1.25).clamp(0.05, 16.0); self.status = format!("zoom {}%", (self.zoom * 100.0).round()); return; }
                if bp.contains(p) { self.zoom = (self.zoom * 1.25).clamp(0.05, 16.0); self.status = format!("zoom {}%", (self.zoom * 100.0).round()); return; }
                if bl.contains(p) {
                    let cw = self.win_w - LAYERS_W - INSPECTOR_W - 40.0;
                    let chh = self.win_h - TOP_H - self.thumbs_h() - STATUS_H - 40.0;
                    self.zoom = (cw / self.editor.root.w.max(1.0)).min(chh / self.editor.root.h.max(1.0)).clamp(0.02, 4.0);
                    self.pan = (20.0, 20.0);
                    self.status = "zoom to fit".into();
                    return;
                }
                // Present pill (accent) + Prototype ghost button
                if pr.contains(p) { self.enter_present(); return; }
                if ppr.contains(p) {
                    self.inspector_tab = 1;
                    self.status = "prototype tab".into();
                    return;
                }
                return;
            }
            // status bar swallows clicks
            if p.y >= self.win_h - STATUS_H { return; }
        }
        if p.x < LAYERS_W && p.y > TOP_H { self.click_left_sidebar(p); return; }
        if p.x > self.win_w - INSPECTOR_W && p.y > TOP_H { self.click_inspector(p); return; }
        if p.y < TOP_H { return; }

        // hand tool or held spacebar -> pan drag
        if self.tool == Tool::Hand || self.space_pan {
            self.drag = Drag::Pan { start: p };
            return;
        }
        // scale tool: needs a selection; vertical drag scales it
        if self.tool == Tool::Scale {
            if let Some(id) = self.editor.selection.first() {
                let _ = id;
                self.drag = Drag::Scale { start_y: p.y, applied: 1.0, cmds: self.editor.undo_depth() };
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
        let wp = self.world_point(p);
        // ---- pen tool: click to place anchors; click near start closes ----
        if self.tool == Tool::Pen {
            match &self.pen_target {
                None => {
                    self.created_count += 1;
                    let id = format!("path-{}", self.created_count);
                    let mut v = Node::vector(&id, 0.0, 0.0, 1.0, 1.0, vec![]);
                    v.fill = Paint::Solid(Color::rgba8(0x0d, 0x99, 0xff, 120));
                    v.stroke = arco_native::Stroke { color: Color::rgb8(0x0d, 0x99, 0xff), width: 2.0 };
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
                        if let arco_native::NodeKind::Vector { path } = &n.kind {
                            arco_native::editor::anchors(path).first()
                                .map(|a| ((a.x - wp.x).powi(2) + (a.y - wp.y).powi(2)).sqrt() < 8.0 / self.zoom)
                                .unwrap_or(false)
                        } else { false }
                    } else { false };
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
                                if let arco_native::NodeKind::Vector { path } = &n.kind {
                                    arco_native::editor::anchors(path).last().map(|a| (a.x + dx, a.y + dy))
                                } else { None }
                            })
                        });
                        self.editor.pen_add_anchor_curved(&id, wp.x, wp.y, out_c1);
                        let idx = find(&self.editor.root, &id).and_then(|n| {
                            if let arco_native::NodeKind::Vector { path } = &n.kind {
                                Some(arco_native::editor::anchors(path).len().saturating_sub(1))
                            } else { None }
                        }).unwrap_or(0);
                        self.pen_placing = Some((idx, wp, self.editor.undo_depth()));
                    }
                }
            }
            return;
        }
        // ---- node-edit mode: grab a HANDLE or an anchor under the cursor ----
        if let Some(vid) = self.node_edit.clone() {
            if let Some(n) = find(&self.editor.root, &vid) {
                if let arco_native::NodeKind::Vector { path } = &n.kind {
                    // anchors are in node-local coords; our pen paths live at 0,0
                    let local = (wp.x - n.transform.x, wp.y - n.transform.y);
                    // bezier control handles first (smaller targets win)
                    let tol = 6.0 / self.zoom;
                    for (ai, a) in arco_native::editor::anchors(path).iter().enumerate() {
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
                    if let Some(ai) = arco_native::editor::anchor_at(path, local.0, local.1, 8.0 / self.zoom) {
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
                                            Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => stops,
                                            _ => return,
                                        };
                                        if stops.len() > 2 && stop < stops.len() { stops.remove(stop); }
                                    }
                                });
                                self.gradient_stop = self.gradient_stop.min(self.gradient_stop.saturating_sub(1));
                                self.status = "gradient stop removed".into();
                                return;
                            }
                        }
                        self.drag = Drag::Gradient { fill, handle, cmds: self.editor.undo_depth() };
                        self.status = if handle < 2 { "dragging gradient geometry".into() } else { format!("dragging gradient stop {}", handle - 1) };
                        return;
                    }
                    if double {
                        if let Some((fill, position)) = self.gradient_line_position(p) {
                            let id = self.editor.selection[0].clone();
                            let mut inserted = 0usize;
                            self.editor.mutate_visual_stack(&id, |n| {
                                if let Some(layer) = n.fill_layers.get_mut(fill) {
                                    let stops = match &mut layer.paint {
                                        Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => stops,
                                        _ => return,
                                    };
                                    let color = gradient_color_at(stops, position);
                                    inserted = stops.iter().position(|(p, _)| *p > position).unwrap_or(stops.len());
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
                        if let Some(b) = self.selection_screen_bounds() {
                            let center = Point::new((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0);
                            let a0 = (p.y - center.y).atan2(p.x - center.x);
                            self.drag = Drag::Rotate { center, start_angle: a0, orig: n.transform.rotation, cmds: self.editor.undo_depth() };
                            return;
                        }
                    }
                }
                if let Some(corner) = self.handle_at(p) {
                    if let Some(n) = self.selected_single() {
                        self.drag = Drag::Resize { corner, start_world: wp, orig: (n.transform.x, n.transform.y, n.w, n.h), cmds: self.editor.undo_depth() };
                        return;
                    }
                }
                if double {
                    // drill-in double-click: drill into the hierarchy;
                    // Vector -> node-edit mode; Text -> inline edit.
                    if let Some(next) = self.editor.drill_into(wp) {
                        if let Some(n) = find(&self.editor.root, &next) {
                            if matches!(n.kind, arco_native::NodeKind::Vector { .. }) {
                                self.node_edit = Some(next.clone());
                                self.status = "node edit: drag anchors, ctrl+click converts, alt+click deletes, Esc done".into();
                                self.drag = Drag::None;
                                return;
                            }
                            if let arco_native::NodeKind::Text { text } = &n.kind {
                                self.focus = Focus::TextNode { id: n.id.clone(), buffer: text.clone(), original: text.clone(), caret: text.len(), sel_anchor: None };
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
                    self.drag = Drag::Marquee { start_world: wp };
                } else {
                    self.alt_dupe_done = false;
                    self.drag = Drag::Move { start: p, cmds: self.editor.undo_depth() };
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
                out.push((fam.clone(), FontSource::System { family: fam.clone(), style: String::new() }));
            }
            if out.len() >= 40 && q.is_empty() { break; }
        }
        for f in self.gfonts.search(if q.is_empty() { "a" } else { &q }) {
            if out.iter().any(|(l, _)| l.eq_ignore_ascii_case(&f.family)) { continue; }
            out.push((format!("{} (G)", f.family), FontSource::Google { family: f.family.clone(), weight: 400 }));
            if out.len() >= 80 { break; }
        }
        self.font_scroll = 0;
        self.font_results = out;
    }

    /// Bind the selected Text node to a font from the results (loads it
    /// into the FontManager on demand; Google fonts download+cache).
    pub fn apply_font(&mut self, idx: usize) {
        let Some((label, source)) = self.font_results.get(idx).cloned() else { return };
        let Some(id) = self.editor.selection.first().cloned() else { return };
        let loaded = match &source {
            FontSource::System { family, style } =>
                self.sysfonts.load_into(&mut self.fonts, family, style).map(|i| self.fonts.fonts[i].name.clone()),
            FontSource::Google { family, weight } =>
                self.gfonts.load_into(&mut self.fonts, family, *weight).map(|i| self.fonts.fonts[i].name.clone()),
        };
        match loaded {
            Ok(name) => {
                if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
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
        let Some((family, weight, italic)) = self.font_weights.get(idx).cloned() else { return };
        let Some(id) = self.editor.selection.first().cloned() else { return };
        match self.gfonts.load_style_into(&mut self.fonts, &family, weight, italic) {
            Ok(i) => {
                let name = self.fonts.fonts[i].name.clone();
                if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
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
        // GROUP UNION SUBTRACT INTERSECT EXCLUDE MASK (run.rs builds items)
        match i {
            0 => self.clipboard_cut(),
            1 => self.clipboard_copy(),
            2 => self.clipboard_paste(),
            3 => { self.editor.duplicate_selection((16.0, 16.0)); self.status = "duplicated".into(); }
            4 => { self.editor.delete_selection(); self.status = "deleted".into(); }
            5 => { if let Some(id) = self.editor.selection.first().cloned() { self.editor.bring_to_front(&id); self.status = "to front".into(); } }
            6 => { if let Some(id) = self.editor.selection.first().cloned() { self.editor.send_to_back(&id); self.status = "to back".into(); } }
            7 => { let gid = format!("group-{}", self.editor.undo_depth()); self.editor.group_selection(&gid); self.status = "grouped".into(); }
            8 | 9 | 10 | 11 => {
                use arco_native::editor::BoolOp::*;
                let op = [Union, Subtract, Intersect, Exclude][i - 8];
                match self.editor.boolean_selected(op) {
                    Some(id) => self.status = format!("{op:?} -> {id}"),
                    None => self.status = "boolean needs 2 shape nodes".into(),
                }
            }
            12 => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
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
        let issues = arco_native::fileio::validate(&d);
        let mut d2 = arco_native::fileio::DocumentV2::default();
        // keep the file's display name stable (dashboard identity)
        d2.metadata.name = self.dash_files.iter()
            .find(|f| f.path == self.doc_path)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| if self.doc_path == "document.x" { "Brand Dashboard".into() } else {
                std::path::Path::new(&self.doc_path).file_stem().unwrap_or_default().to_string_lossy().to_string()
            });
        d2.metadata.app_version = "0.43".into();
        for p in &d.pages { arco_native::fileio::v2::backfill_uuids(p, &mut d2.uuids); }
        d2.metadata.uuid = arco_native::fileio::v2::fnv1a128(&d2.metadata.name);
        d2.doc = d;
        let text = arco_native::fileio::save_x_v2(&d2);
        // reliability: history rotation + atomic publish + stale-autosave clear
        arco_native::fileio::rotate_backups(&self.doc_path);
        self.status = match arco_native::fileio::atomic_write(&self.doc_path, text.as_bytes()) {
            Ok(_) if issues.is_empty() => format!("saved v2 ({} pages, atomic, {} backup(s))", d2.doc.pages.len(), arco_native::fileio::list_backups(&self.doc_path).len()),
            Ok(_) => format!("saved v2 with {} validation issue(s)", issues.len()),
            Err(_) => "save FAILED".into(),
        };
        arco_native::fileio::clear_autosave(&self.doc_path);
        self.dirty_since_save = false;
        self.saved_undo_depth = self.editor.undo_depth();
        arco_native::fileio::push_recent(&self.doc_path);
    }

    /// Native save panel for first-save / Save As. The normal Save command
    /// remains instant once a document has a path.
    pub fn save_document_as(&mut self) {
        let suggested = std::path::Path::new(&self.doc_path)
            .file_name().and_then(|v| v.to_str()).unwrap_or("Untitled.x");
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save X Designer document")
            .set_file_name(suggested)
            .add_filter("X Designer document", &["x"])
            .save_file() else {
                self.status = "save cancelled".into();
                return;
            };
        self.doc_path = path.to_string_lossy().to_string();
        self.save_document();
        self.scan_dash_files();
    }

    pub fn open_document(&mut self) {
        if let Ok(text) = std::fs::read_to_string(&self.doc_path) {
            let (d2, notes) = arco_native::fileio::load_x_lenient(&text);
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
                if decoded > 0 { eprintln!("assets: decoded {decoded} embedded image(s)"); }
                // style consumers re-sync on open (standard semantics)
                arco_native::resolve_styles(&mut self.editor.root, &self.styles);
                self.status = if notes.is_empty() {
                    format!("loaded ({} pages)", self.pages.len())
                } else {
                    format!("RECOVERED ({} pages, {} note(s))", self.pages.len(), notes.len())
                };
                // integrity sweep LAST so warnings win the status line
                self.library_integrity.clear();
                let mut dv = Document::new();
                dv.library_deps = self.library_deps.clone();
                dv.library_snapshots = self.library_snapshots.clone();
                for (lid, st) in arco_native::fileio::verify_document_libraries(&dv) {
                    if !matches!(st, arco_native::fileio::IntegrityStatus::Verified) {
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
            .pick_file() else {
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
            .pick_file() else {
                self.status = "import cancelled".into();
                return;
            };
        self.stage_import_path(path);
    }

    pub fn start_figma_import(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Import Figma REST API JSON")
            .add_filter("Figma REST API JSON", &["json"]).pick_file() else {
                self.status = "import cancelled".into(); return;
            };
        self.stage_import_path(path);
    }

    pub fn start_sketch_import(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Import Sketch document")
            .add_filter("Sketch document", &["sketch"]).pick_file() else {
                self.status = "import cancelled".into(); return;
            };
        self.stage_import_path(path);
    }

    fn stage_import_path(&mut self, path: std::path::PathBuf) {
        let mut result: Option<(String, Result<(Document, arco_native::fileio::ImportReport), String>)> = None;
        let ext = path.extension().and_then(|v| v.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == "sketch" {
            let bytes = match std::fs::read(&path) { Ok(v) => v, Err(e) => { self.status = format!("import failed: {e}"); return; } };
            result = Some(("sketch".into(), arco_native::fileio::import_sketch_with_report(&bytes)));
        } else if ext == "json" {
            let text = match std::fs::read_to_string(&path) { Ok(v) => v, Err(e) => { self.status = format!("import failed: {e}"); return; } };
            result = Some(("figma".into(), arco_native::fileio::import_figma_json(&text).map(|d| {
                let r = arco_native::fileio::ImportReport {
                    nodes_imported: d.pages.iter().map(count_nodes).sum(),
                    assets_imported: d.assets.len(),
                    diagnostics: vec![],
                };
                (d, r)
            })));
        } else if ext == "svg" {
            let text = match std::fs::read_to_string(&path) { Ok(v) => v, Err(e) => { self.status = format!("import failed: {e}"); return; } };
            result = Some(("svg".into(), arco_native::fileio::import_svg(&text).map(|root| {
                let mut d = Document::new();
                d.pages.push(root);
                let r = arco_native::fileio::ImportReport {
                    nodes_imported: d.pages.iter().map(count_nodes).sum(),
                    assets_imported: 0,
                    diagnostics: vec![],
                };
                (d, r)
            })));
        } else if ext == "png" {
            let bytes = match std::fs::read(&path) { Ok(v) => v, Err(e) => { self.status = format!("import failed: {e}"); return; } };
            let asset_name = path.file_stem().and_then(|v| v.to_str()).unwrap_or("image");
            let _ = self.assets.load_png(asset_name, path.to_string_lossy().as_ref());
            result = Some(("png".into(), arco_native::fileio::import_png(asset_name, &bytes).map(|d| {
                let r = arco_native::fileio::ImportReport {
                    nodes_imported: d.pages.iter().map(count_nodes).sum(),
                    assets_imported: d.assets.len(),
                    diagnostics: vec![],
                };
                (d, r)
            })));
        }
        match result {
            Some((src, Ok((d, report)))) if !d.pages.is_empty() => {
                self.status = format!("{src}: {} node(s), {} asset(s), {} diagnostic(s) — review the preview",
                    report.nodes_imported, report.assets_imported, report.diagnostics.len());
                self.import_pending = Some((src, d, report));
            }
            Some((src, Ok(_))) => self.status = format!("{src} file has no pages"),
            Some((src, Err(e))) => self.status = format!("{src} import FAILED: {e}"),
            None => self.status = "unsupported import format".into(),
        }
    }

    pub fn export_svg_now(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Export SVG")
            .set_file_name("export.svg").add_filter("SVG", &["svg"]).save_file() else {
            self.status = "export cancelled".into(); return;
        };
        let outliner = arco_native::svg_text_outliner(&self.fonts);
        let resolver = |name: &str| -> Option<Vec<u8>> { std::fs::read(format!("assets/{name}.png")).ok() };
        let svg = arco_native::fileio::export_svg_full(&self.editor.root, &self.vars, Some(&resolver), Some(&outliner));
        self.status = if std::fs::write(&path, svg).is_ok() { format!("exported {}", path.display()) } else { "export FAILED".into() };
    }

    pub fn export_png_now(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Export PNG")
            .set_file_name("export.png").add_filter("PNG", &["png"]).save_file() else {
            self.status = "export cancelled".into(); return;
        };
        self.status = match export_png(&self.editor.root, &self.vars, &self.assets, &self.fonts, path.to_string_lossy().as_ref()) {
            Ok((w, h)) => format!("exported {} ({w}x{h})", path.display()),
            Err(e) => format!("png export FAILED: {e}"),
        };
    }

    pub fn export_pdf_now(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Export PDF")
            .set_file_name("export.pdf").add_filter("PDF", &["pdf"]).save_file() else {
            self.status = "export cancelled".into(); return;
        };
        let tree = arco_native::build_render_tree(&self.editor.root, &self.vars);
        let pdf = arco_native::export_pdf_full(&tree, self.editor.root.w, self.editor.root.h, Some(&self.assets), Some(&self.fonts));
        self.status = if std::fs::write(&path, pdf).is_ok() { format!("exported {}", path.display()) } else { "pdf export FAILED".into() };
    }

    fn document_snapshot(&mut self) -> Document {
        self.pages[self.page_idx] = self.editor.root.clone();
        let mut d = Document::new(); d.variables = self.vars.clone(); d.styles = self.styles.clone(); d.assets = self.store.clone(); d.library_deps = self.library_deps.clone(); d.library_snapshots = self.library_snapshots.clone(); d.pages = self.pages.clone(); d
    }

    pub fn export_figma_now(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Export Figma-compatible JSON").set_file_name("x-designer-export.json").add_filter("Figma REST API JSON", &["json"]).save_file() else { self.status = "export cancelled".into(); return; };
        let doc = self.document_snapshot(); let json = arco_native::fileio::export_figma_json(&doc);
        self.status = match std::fs::write(&path, json) { Ok(_) => format!("exported Figma-compatible JSON: {}", path.display()), Err(e) => format!("Figma export FAILED: {e}") };
    }

    pub fn export_sketch_now(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Export Sketch-compatible document").set_file_name("x-designer.sketch").add_filter("Sketch document", &["sketch"]).save_file() else { self.status = "export cancelled".into(); return; };
        let doc = self.document_snapshot(); let bytes = arco_native::fileio::export_sketch(&doc);
        self.status = match std::fs::write(&path, bytes) { Ok(_) => format!("exported Sketch-compatible file: {}", path.display()), Err(e) => format!("Sketch export FAILED: {e}") };
    }

    // ------------------------------------------------- header dropdown menus
    // REAL menus (session 46): geometry shared between painter and click
    // handler via menu_title_rects()/menu_layout(); every item dispatches
    // through run_menu_tag into the SAME methods the shortcuts use.

    /// header right cluster (mockup): zoom pill halves + Prototype ghost +
    /// Present pill, laid out from the RIGHT edge (header spans full width).
    /// Returns (zoom_minus, zoom_label, zoom_plus, prototype, present).
    pub fn header_rects(&self) -> (Rect, Rect, Rect, Rect, Rect) {
        let r2y = TAB_H;
        let pw = ui_measure("Present", 9.5) + 40.0;
        let pr = Rect::new(self.win_w - 48.0 - pw, r2y + 8.0, self.win_w - 48.0, TOP_H - 8.0);
        let ptw = ui_measure("Prototype", 9.5) + 40.0;
        let ppr = Rect::new(pr.x0 - 10.0 - ptw, r2y + 8.0, pr.x0 - 10.0, TOP_H - 8.0);
        let zx = ppr.x0 - 16.0 - 102.0;
        let bm = Rect::new(zx, r2y + 8.0, zx + 22.0, TOP_H - 8.0);
        let bl = Rect::new(zx + 24.0, r2y + 8.0, zx + 78.0, TOP_H - 8.0);
        let bp = Rect::new(zx + 80.0, r2y + 8.0, zx + 102.0, TOP_H - 8.0);
        (bm, bl, bp, ppr, pr)
    }

    /// clickable title rects for File/Edit/View/Object/Help in header row 2
    pub fn menu_title_rects(&self) -> Vec<(usize, Rect)> {
        let mut out = vec![];
        let mut mx = 16.0;
        for (i, (title, _)) in MENUS.iter().enumerate() {
            let w = ui_measure(title, 9.5);
            out.push((i, Rect::new(mx - 6.0, TAB_H + 4.0, mx + w + 6.0, TOP_H - 4.0)));
            mx += w + 22.0;
        }
        out
    }

    /// rows of the OPEN dropdown: (label, shortcut, action tag, rect)
    pub fn menu_layout(&self) -> Vec<(String, String, String, Rect)> {
        let Some(mi) = self.menu_open else { return vec![] };
        let (_, items) = MENUS[mi];
        let title_r = self.menu_title_rects()[mi].1;
        let mut w: f64 = 168.0;
        for (l, s, _) in items {
            w = w.max(ui_measure(l, 9.0) + ui_measure(s, 7.0) + 56.0);
        }
        let x0 = title_r.x0;
        items.iter().enumerate().map(|(i, (l, s, t))| {
            let y = TOP_H + 4.0 + i as f64 * 24.0;
            (l.to_string(), s.to_string(), t.to_string(), Rect::new(x0, y, x0 + w, y + 24.0))
        }).collect()
    }

    /// Polish: menu items gray out when they can't apply right now.
    pub fn menu_item_enabled(&self, tag: &str) -> bool {
        let has_sel = !self.editor.selection.is_empty();
        match tag {
            "edit.undo" => self.editor.undo_depth() > 0,
            "edit.duplicate" | "edit.delete" | "obj.front" | "obj.back" | "obj.forward" | "obj.backward" | "obj.mask" => has_sel,
            "edit.cut" | "edit.copy" => has_sel,
            "edit.paste" => self.editor.clipboard_len() > 0,
            "page.delete" => self.pages.len() > 1,
            "obj.group" | "obj.union" | "obj.subtract" | "obj.intersect" | "obj.exclude" =>
                self.editor.selection.len() >= 2,
            "obj.ungroup" | "obj.component" => has_sel,
            "arr.disth" | "arr.distv" => self.editor.selection.len() >= 3,
            "edit.copy_svg" => has_sel,
            "page.left" => self.page_idx > 0,
            "page.right" => self.page_idx + 1 < self.pages.len(),
            "arr.fliph" | "arr.flipv" => has_sel,
            t if t.starts_with("arr.") => self.editor.selection.len() >= 2,
            "noop" => false,
            _ => true,
        }
    }

    pub fn run_menu_tag(&mut self, tag: &str) {
        match tag {
            "file.new_page" => self.add_page(),
            "file.new" => self.new_file(),
            "file.dashboard" => self.back_to_dashboard(),
            "edit.cut" => self.clipboard_cut(),
            "edit.copy" => self.clipboard_copy(),
            "edit.paste" => self.clipboard_paste(),
            "page.rename" => self.start_page_rename(self.page_idx),
            "page.left" => self.reorder_page(-1),
            "page.right" => self.reorder_page(1),
            "edit.paste_svg" => self.paste_svg_from_clipboard(),
            "page.duplicate" => self.duplicate_page(),
            "page.delete" => { let i = self.page_idx; self.delete_page(i); }
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
            "file.export_pdf" => self.export_pdf_now(),
            "edit.undo" => { self.editor.undo(); self.status = "undo".into(); }
            "edit.redo" => { self.editor.redo(); self.status = "redo".into(); }
            "edit.duplicate" => { self.editor.duplicate_selection((16.0, 16.0)); self.status = "duplicated".into(); }
            "edit.delete" => { self.editor.delete_selection(); self.status = "deleted".into(); }
            "edit.select_all" => { self.editor.select_all(); self.status = format!("{} selected", self.editor.selection.len()); }
            "view.zoom_in" => { self.zoom = (self.zoom * 1.25).clamp(0.05, 16.0); self.status = format!("zoom {}%", (self.zoom * 100.0).round()); }
            "view.zoom_out" => { self.zoom = (self.zoom / 1.25).clamp(0.05, 16.0); self.status = format!("zoom {}%", (self.zoom * 100.0).round()); }
            "view.zoom_100" => { self.zoom = 1.0; self.status = "zoom 100%".into(); }
            "view.zoom_fit" => {
                let cw = self.win_w - LAYERS_W - INSPECTOR_W - 40.0;
                let chh = self.win_h - TOP_H - self.thumbs_h() - STATUS_H - 40.0;
                self.zoom = (cw / self.editor.root.w.max(1.0)).min(chh / self.editor.root.h.max(1.0)).clamp(0.02, 4.0);
                self.pan = (20.0, 20.0);
                self.status = format!("zoom to fit ({:.0}%)", self.zoom * 100.0);
            }
            "view.rulers" => { self.rulers = !self.rulers; self.status = if self.rulers { "rulers on".into() } else { "rulers off".into() }; }
            "view.outline" => { self.outline_view = !self.outline_view; self.status = if self.outline_view { "outline view".into() } else { "normal view".into() }; }
            "view.vars" => { self.inspector_tab = 2; self.status = "variables tab".into(); }
            "view.minimap" => { self.minimap = !self.minimap; self.status = if self.minimap { "minimap on".into() } else { "minimap off".into() }; }
            "view.hud" => { self.perf_hud = !self.perf_hud; self.status = if self.perf_hud { "perf HUD on".into() } else { "perf HUD off".into() }; }
            "view.hide_ui" => { self.chrome_hidden = true; self.status = "UI hidden (⌘. to show)".into(); }
            "arr.fliph" | "arr.flipv" => {
                let horizontal = tag == "arr.fliph";
                let ids = self.editor.selection.clone();
                let depth = self.editor.undo_depth();
                for id in ids { self.editor.flip_node(&id, horizontal); }
                self.editor.merge_last(self.editor.undo_depth().saturating_sub(depth));
                self.status = if horizontal { "flipped horizontally".into() } else { "flipped vertically".into() };
            }
            "obj.group" => {
                if self.editor.selection.len() >= 2 {
                    let gid = format!("group-{}", self.editor.undo_depth());
                    self.editor.group_selection(&gid);
                    self.status = format!("grouped -> {gid}");
                } else { self.status = "select 2+ nodes to group".into(); }
            }
            "obj.ungroup" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if self.editor.ungroup(&id) { self.status = "ungrouped".into(); }
                }
            }
            "obj.front" => { if let Some(id) = self.editor.selection.first().cloned() { self.editor.bring_to_front(&id); self.status = "to front".into(); } }
            "obj.back" => { if let Some(id) = self.editor.selection.first().cloned() { self.editor.send_to_back(&id); self.status = "to back".into(); } }
            "obj.forward" => { if let Some(id) = self.editor.selection.first().cloned() { self.editor.bring_forward(&id); self.status = "forward".into(); } }
            "obj.backward" => { if let Some(id) = self.editor.selection.first().cloned() { self.editor.send_backward(&id); self.status = "backward".into(); } }
            "obj.union" | "obj.subtract" | "obj.intersect" | "obj.exclude" => {
                use arco_native::editor::BoolOp::*;
                let op = match tag { "obj.union" => Union, "obj.subtract" => Subtract, "obj.intersect" => Intersect, _ => Exclude };
                match self.editor.boolean_selected(op) {
                    Some(id) => self.status = format!("{op:?} -> {id}"),
                    None => self.status = "boolean needs 2 shape nodes".into(),
                }
            }
            "obj.mask" => {
                if let Some(id) = self.editor.selection.first().cloned() {
                    if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
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
                } else { self.status = "select sibling nodes first".into(); }
            }
            "arr.left" | "arr.centerh" | "arr.right" | "arr.top" | "arr.centerv" | "arr.bottom" => {
                use arco_native::editor::AlignKind::*;
                let kind = match tag {
                    "arr.left" => Left, "arr.centerh" => CenterH, "arr.right" => Right,
                    "arr.top" => Top, "arr.centerv" => CenterV, _ => Bottom,
                };
                let ids = self.editor.selection.clone();
                if ids.len() >= 2 {
                    arco_native::editor::align(&mut self.editor.root, &ids, kind);
                    self.status = format!("aligned {:?}", kind);
                } else { self.status = "select 2+ layers to align".into(); }
            }
            "arr.disth" | "arr.distv" => {
                // distribute-spacing: sort by axis, equalize the gaps
                let ids = self.editor.selection.clone();
                if ids.len() < 3 { self.status = "select 3+ layers to distribute".into(); }
                else {
                    let horizontal = tag == "arr.disth";
                    let mut items: Vec<(String, f64, f64)> = ids.iter().filter_map(|id| {
                        find(&self.editor.root, id).map(|n| (id.clone(),
                            if horizontal { n.transform.x } else { n.transform.y },
                            if horizontal { n.w } else { n.h }))
                    }).collect();
                    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    let first = items.first().unwrap().clone();
                    let last = items.last().unwrap().clone();
                    let span = (last.1 + last.2) - first.1;
                    let content: f64 = items.iter().map(|(_, _, sz)| sz).sum();
                    let gap = (span - content) / (items.len() - 1) as f64;
                    let mut cursor = first.1;
                    for (id, _, sz) in &items {
                        if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, id) {
                            if horizontal { n.transform.x = cursor; } else { n.transform.y = cursor; }
                            n.dirty = true;
                        }
                        cursor += sz + gap;
                    }
                    self.status = format!("distributed {} layers ({})", items.len(), if horizontal { "horizontal" } else { "vertical" });
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
            .map(|i| (i, Rect::new(i as f64 * cw, TOP_H, (i + 1) as f64 * cw, TOP_H + LTAB_H)))
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
                    if y + tile_h > self.win_h - 40.0 { break; }
                    out.push((id.clone(), Rect::new(x, y, x + tile_w, y + tile_h), 1));
                }
            }
            2 => {
                // COMPONENTS: document components, then linked library ones
                let comps = self.editor.component_names();
                let mut y = y0;
                for name in &comps {
                    out.push((name.clone(), Rect::new(8.0, y, LAYERS_W - 8.0, y + ROW_H - 2.0), 2));
                    y += ROW_H;
                }
                for dep in &self.library_deps {
                    let Some(lib) = self.library_snapshots.get(&dep.library_id) else { continue };
                    if lib.components.is_empty() { continue }
                    out.push((format!("{} v{}", lib.name, dep.resolved_version), Rect::new(8.0, y + 8.0, LAYERS_W - 8.0, y + 22.0), 5));
                    y += 26.0;
                    for c in lib.components.iter().take(10) {
                        if let arco_native::NodeKind::Component { name } = &c.kind {
                            out.push((format!("{}|{name}", dep.library_id), Rect::new(8.0, y, LAYERS_W - 8.0, y + ROW_H - 2.0), 3));
                            y += ROW_H;
                        }
                    }
                }
            }
            3 => {
                // LIBRARY: linked library summaries + jump to full manager
                let mut y = y0;
                for dep in &self.library_deps {
                    let Some(lib) = self.library_snapshots.get(&dep.library_id) else { continue };
                    out.push((format!("{} v{}", lib.name, dep.resolved_version), Rect::new(8.0, y, LAYERS_W - 8.0, y + 16.0), 5));
                    y += 18.0;
                    let ok = self.library_integrity.iter().find(|(id, _)| *id == dep.library_id)
                        .map(|(_, s)| s.starts_with("Verified")).unwrap_or(true);
                    let badge = format!("{} style(s), {} comp(s){}", lib.styles.len(), lib.components.len(),
                        if ok { "" } else { " — INTEGRITY!" });
                    out.push((badge, Rect::new(8.0, y, LAYERS_W - 8.0, y + 14.0), 5));
                    y += 22.0;
                }
                out.push(("OPEN LIBRARY MANAGER".into(), Rect::new(12.0, y + 6.0, LAYERS_W - 12.0, y + 28.0), 4));
            }
            _ => {}
        }
        out
    }

    /// Export section (mockup, bottom of the Design inspector): buttons
    /// (label, action tag, rect) — geometry shared painter/click.
    pub fn export_layout(&self) -> Vec<(&'static str, &'static str, Rect)> {
        let ix = self.win_w - INSPECTOR_W;
        let y = self.win_h - self.thumbs_h() - STATUS_H - 66.0;
        let bw = (INSPECTOR_W - 24.0 - 16.0) / 3.0;
        [("X", "file.export_x"), ("FIG", "file.export_figma"), ("SKETCH", "file.export_sketch"), ("PNG", "file.export_png"), ("SVG", "file.export_svg"), ("PDF", "file.export_pdf")]
            .iter().enumerate()
            .map(|(i, (l, t))| { let col=i%3; let row=i/3; (*l, *t, Rect::new(ix + 12.0 + col as f64 * (bw + 8.0), y + row as f64 * 28.0, ix + 12.0 + col as f64 * (bw + 8.0) + bw, y + row as f64 * 28.0 + 24.0)) })
            .collect()
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
        // standard pen tool: a drag right after placing an anchor pulls
        // a bezier handle out of it instead of leaving a plain corner.
        if let (Tool::Pen, Some((idx, _, _))) = (self.tool, self.pen_placing) {
            self.cursor = p;
            if let Some(id) = self.pen_target.clone() {
                let wp = self.world_point(p);
                if let Some(n) = find(&self.editor.root, &id) {
                    if let arco_native::NodeKind::Vector { path } = &n.kind {
                        if let Some(a) = arco_native::editor::anchors(path).get(idx).copied() {
                            let (dx, dy) = (wp.x - a.x, wp.y - a.y);
                            if idx > 0 { self.editor.pen_shape_incoming(&id, idx, dx, dy); }
                            self.pen_pending_out = if dx.abs() > 0.01 || dy.abs() > 0.01 { Some((dx, dy)) } else { None };
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
                                Paint::LinearGradient { start, end, stops } => {
                                    match handle {
                                        0 => *start = (local.x, local.y),
                                        1 => *end = (local.x, local.y),
                                        h => {
                                            let vx = end.0 - start.0; let vy = end.1 - start.1;
                                            let len2 = vx * vx + vy * vy;
                                            if len2 > 0.0001 && h - 2 < stops.len() {
                                                let t = (((local.x - start.0) * vx + (local.y - start.1) * vy) / len2).clamp(0.0, 1.0) as f32;
                                                stops[h - 2].0 = t;
                                                stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                                                if let Some(i) = stops.iter().position(|(p, _)| (*p - t).abs() < f32::EPSILON) { active_handle = i + 2; }
                                            }
                                        }
                                    }
                                }
                                Paint::RadialGradient { center, radius, stops } => {
                                    match handle {
                                        0 => *center = (local.x, local.y),
                                        1 => *radius = (local.x - center.0).hypot(local.y - center.1),
                                        h if h - 2 < stops.len() => {
                                            let r = (*radius).max(0.0001);
                                            stops[h - 2].0 = ((local.x - center.0).hypot(local.y - center.1) / r).clamp(0.0, 1.0) as f32;
                                            let t = stops[h - 2].0;
                                            stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                                            if let Some(i) = stops.iter().position(|(p, _)| (*p - t).abs() < f32::EPSILON) { active_handle = i + 2; }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }
            }
            if active_handle >= 2 { self.gradient_stop = active_handle - 2; }
            self.drag = Drag::Gradient { fill, handle: active_handle, cmds };
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
                    let (sx, sy) = arco_native::editor::snap_delta(&self.editor.root, &id, 4.0 / self.zoom);
                    if sx != 0.0 || sy != 0.0 { self.editor.move_selection(sx, sy); }
                    self.guides = arco_native::editor::alignment_guides(&self.editor.root, &id, 1.0);
                } else { self.guides = vec![]; }
                self.drag = match self.drag { Drag::Move { cmds, .. } => Drag::Move { start: p, cmds }, d => d };
            }
        } else if let Drag::Resize { corner, start_world, orig, cmds } = self.drag {
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
                if (nw / w).abs() > (nh / h).abs() { nh = nw / ratio; } else { nw = nh * ratio; }
            }
            // Alt = resize from the center, growing/shrinking both sides
            // equally instead of anchoring the opposite edge.
            if self.alt {
                nw = w + (nw - w) * 2.0;
                nh = h + (nh - h) * 2.0;
            }
            self.editor.resize(&id, nw.max(2.0), nh.max(2.0));
            if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                if self.alt {
                    // keep the shape centered on its original center
                    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                    n.transform.x = cx - nw.max(2.0) / 2.0;
                    n.transform.y = cy - nh.max(2.0) / 2.0;
                } else {
                    // opposite corner stays fixed
                    match corner {
                        0 => { n.transform.x = x + dx; n.transform.y = y + dy; }
                        1 => { n.transform.y = y + dy; }
                        2 => { n.transform.x = x + dx; }
                        4 => { n.transform.x = x + dx; }
                        6 => { n.transform.y = y + dy; }
                        _ => {}
                    }
                }
            }
            self.drag = Drag::Resize { corner, start_world, orig, cmds };
        } else if let Some((ai, outgoing, _)) = self.handle_drag {
            if let Some(vid) = self.node_edit.clone() {
                let wp = self.world_point(p);
                if let Some(n) = find(&self.editor.root, &vid) {
                    let local = (wp.x - n.transform.x, wp.y - n.transform.y);
                    // Alt breaks the tangent (independent handle); the
                    // default drag mirrors the opposite handle, same as
                    // Figma/Illustrator's smooth-point behavior.
                    self.editor.move_handle(&vid, ai, outgoing, local.0, local.1, !self.alt);
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
        } else if let Drag::Scale { start_y, applied, cmds } = self.drag {
            // target factor from total drag distance: 200px up = +100%
            let target = (1.0 - (p.y - start_y) / 200.0).clamp(0.2, 5.0);
            let step = target / applied;
            if (step - 1.0).abs() > 0.01 {
                if let Some(id) = self.editor.selection.first().cloned() {
                    self.editor.scale_node(&id, step);
                    self.drag = Drag::Scale { start_y, applied: target, cmds };
                    self.status = format!("scale {:.0}%", target * 100.0);
                }
            }
        } else if self.drag == Drag::None && self.tool == Tool::Select && self.present.is_none() {
            // hover highlight (only inside canvas, not over chrome)
            self.hover = if self.canvas_rect().contains(p) {
                arco_native::editor::hit_test(&self.editor.root, self.world_point(p))
                    .filter(|id| !self.editor.selection.contains(id))
            } else { None };
        }
        if let Drag::Rotate { center, start_angle, orig, cmds } = self.drag {
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
            self.drag = Drag::Rotate { center, start_angle, orig, cmds };
        }
        self.cursor = p;
    }

    pub fn mouse_up(&mut self, p: Point) {
        // standard pen tool: releasing after a curve-handle drag merges
        // the drag's incremental commands into the anchor's placement step,
        // and keeps pen_pending_out so the NEXT anchor inherits the tangent.
        if let Some((_, _, depth)) = self.pen_placing.take() {
            let n = self.editor.undo_depth().saturating_sub(depth);
            self.editor.merge_last(n);
            self.status = "pen: click to add anchors, drag to curve, click start to close, Esc to finish".into();
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
                let dims = self.store.get(&aid).and_then(|r| r.dimensions).unwrap_or((160, 120));
                self.created_count += 1;
                let nid = format!("image-{}", self.created_count);
                let mut n = Node::image(&nid, wp.x, wp.y, dims.0 as f64, dims.1 as f64, &aid);
                n.transform.x = wp.x; n.transform.y = wp.y;
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
                    self.status = format!("rotated to {:.0} deg", node.transform.rotation.to_degrees());
                }
            }
            Drag::Gradient { cmds, .. } => {
                let n = self.editor.undo_depth().saturating_sub(cmds);
                self.editor.merge_last(n);
                self.status = "gradient updated".into();
            }
            Drag::Marquee { start_world } => {
                let wp = self.world_point(p);
                let r = Rect::new(start_world.x.min(wp.x), start_world.y.min(wp.y), start_world.x.max(wp.x), start_world.y.max(wp.y));
                if r.width() > 2.0 && r.height() > 2.0 {
                    self.editor.selection = hit_test_rect(&self.editor.root, r);
                    self.status = format!("marquee: {} selected", self.editor.selection.len());
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
                let id = format!("{}-{}", self.tool.label().to_lowercase(), self.created_count);
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
                    Tool::Rectangle => Node::rect(&id, bx, by, bw, bh, C_ACCENT).radius(self.rect_radius),
                    Tool::Ellipse => Node::ellipse(&id, bx, by, bw, bh, PALETTE[1]),
                    Tool::Line => Node::line(&id, bx, by, bw, bh.max(2.0), Color::WHITE),
                    // Starts empty: an un-typed placeholder string would
                    // commit as real content if you clicked away without
                    // typing, which Figma never does.
                    Tool::Text => Node::text(&id, bx, by, bw, bh.clamp(12.0, 64.0), ""),
                    Tool::Polygon => {
                        let mut n = Node::vector(&id, 0.0, 0.0, bw, bh, regular_polygon(self.polygon_sides, bw, bh));
                        n.transform.x = bx; n.transform.y = by;
                        n.fill = Paint::Solid(PALETTE[2]);
                        n
                    }
                    Tool::Star => {
                        let mut n = Node::vector(&id, 0.0, 0.0, bw, bh, star_path_with_ratio(self.star_points, bw, bh, self.star_inner_ratio));
                        n.transform.x = bx; n.transform.y = by;
                        n.fill = Paint::Solid(PALETTE[4]);
                        n
                    }
                    Tool::Frame | Tool::Select | Tool::Hand | Tool::Scale | Tool::Pen => {
                        let mut f = Node::frame(&id, bw, bh);
                        f.transform.x = bx; f.transform.y = by;
                        f.fill = Paint::Solid(Color::rgb8(0x38, 0x38, 0x38));
                        f
                    }
                };
                let created_tool = self.tool;
                let root_id = self.editor.root.id.clone();
                self.editor.insert_node(&root_id, node);
                self.editor.selection = vec![id.clone()];
                self.rebuild_layer_rows();
                self.tool = Tool::Select;
                if created_tool == Tool::Text {
                    // Drop straight into typing, like Figma — no
                    // separate double-click is needed after creation.
                    self.focus = Focus::TextNode { id: id.clone(), buffer: String::new(), original: String::new(), caret: 0, sel_anchor: None };
                    self.status = "editing text — Enter/Esc commits, empty text is discarded".into();
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
                if !r.contains(p) { continue; }
                match kind {
                    1 => {
                        // asset tile: select + arm drag-to-canvas (same
                        // semantics as the Shift+A browser tiles)
                        self.asset_sel = Some(tag.clone());
                        self.asset_drag = Some(tag.clone());
                        let rec = self.store.get(&tag);
                        self.status = match rec {
                            Some(rec) => format!("{} | {} — drag to canvas to place", rec.name, rec.mime),
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
        if p.y >= TOP_H + LSEARCH_Y0 && p.y <= TOP_H + LSEARCH_Y1 && p.x >= 10.0 && p.x <= LAYERS_W - 10.0 {
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
                if self.dbl && idx == self.page_idx { self.start_page_rename(idx); return; }
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
        let idx = if idx >= 0.0 { idx as usize + self.layers_scroll } else { return };
        if idx < self.layer_rows.len() {
            let id = self.layer_rows[idx].0.clone();
            if self.dbl && p.x < LAYERS_W - 40.0 {
                self.focus = Focus::LayerRename { id: id.clone(), buffer: id.clone() };
                self.status = format!("rename layer: {id}");
                return;
            }
            // eye / lock click zones (right side of the row)
            if p.x >= LAYERS_W - 40.0 && p.x < LAYERS_W - 24.0 {
                if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                    n.visible = !n.visible;
                    self.status = format!("{} {}", id, if n.visible { "shown" } else { "hidden" });
                }
                return;
            }
            if p.x >= LAYERS_W - 24.0 {
                if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                    n.locked = !n.locked;
                    self.status = format!("{} {}", id, if n.locked { "locked" } else { "unlocked" });
                }
                return;
            }
            if self.shift {
                if let Some(i) = self.editor.selection.iter().position(|s| s == &id) { self.editor.selection.remove(i); }
                else { self.editor.selection.push(id.clone()); }
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
        if self.selected_single().is_none() && matches!(self.tool, Tool::Rectangle | Tool::Polygon | Tool::Star) {
            let row = ((p.y - (TOP_H + 58.0)) / 30.0).floor() as isize;
            let ix = self.win_w - INSPECTOR_W;
            let delta = if p.x >= ix + 212.0 && p.x <= ix + 230.0 { -1.0 }
                else if p.x >= ix + 232.0 && p.x <= ix + 250.0 { 1.0 }
                else { 0.0 };
            if delta != 0.0 {
                match (self.tool, row) {
                    (Tool::Rectangle, 0) => self.rect_radius = (self.rect_radius + delta * 2.0).clamp(0.0, 100.0),
                    (Tool::Polygon, 0) => self.polygon_sides = ((self.polygon_sides as isize + delta as isize).clamp(3, 60)) as usize,
                    (Tool::Star, 0) => self.star_points = ((self.star_points as isize + delta as isize).clamp(3, 60)) as usize,
                    (Tool::Star, 1) => self.star_inner_ratio = (self.star_inner_ratio + delta * 0.05).clamp(0.05, 0.95),
                    _ => return,
                }
                self.status = "tool defaults updated".into();
                return;
            }
        }
        // Export section buttons (Design tab, mockup)
        if self.inspector_tab == 0 {
            for (l, tag, r) in self.export_layout() {
                if r.contains(p) {
                    let (l, tag) = (l.to_string(), tag.to_string());
                    let t0 = std::time::Instant::now();
                    self.run_menu_tag(&tag);
                    self.last_cmd = Some((format!("export {l}"), t0.elapsed().as_secs_f32() * 1000.0));
                    return;
                }
            }
        }
        // LIBRARIES tab interactions — SHARED layout with the painter
        if self.inspector_tab == 3 {
            for (tag, r, kind) in self.libs_layout() {
                if !r.contains(p) { continue; }
                match kind {
                    1 => { self.link_library(); return; }
                    2 => { self.check_library_updates(); return; }
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
                if p.x >= ix + 12.0 && p.x <= ix + INSPECTOR_W - 24.0 && p.y >= y && p.y <= y + 19.0 {
                    self.created_count += 1;
                    let id = format!("frame-{}", self.created_count);
                    let wp = self.world_point(Point::new(self.canvas_rect().x0 + 60.0, self.canvas_rect().y0 + 60.0));
                    let mut f = Node::frame(&id, *w, *h);
                    f.transform.x = wp.x.max(0.0); f.transform.y = wp.y.max(0.0);
                    f.fill = Paint::Solid(Color::rgb8(0xff, 0xff, 0xff));
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
            let vals = [n.transform.x, n.transform.y, n.w, n.h];
            let rows = [(0u8, TOP_H + IY_XY), (1, TOP_H + IY_XY), (2, TOP_H + IY_WH), (3, TOP_H + IY_WH)];
            for (field, ry) in rows {
                let fx = x0 + if field % 2 == 0 { 0.0 } else { 108.0 };
                if p.x >= fx - 2.0 && p.x <= fx + 100.0 && p.y >= ry - 3.0 && p.y <= ry + 14.0 {
                    // polish: select-all semantics — buffer starts EMPTY so
                    // typing REPLACES the value; Enter empty = keep old
                    self.focus = Focus::Field { id, field, buffer: String::new() };
                    self.status = format!("type new {} (Enter commits, Tab next, Esc cancels)", ["X", "Y", "W", "H"][field as usize]);
                    return;
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
                    let w = arco_native::text::measure(m, 8.0) + 12.0;
                    if p.x >= mx && p.x <= mx + w {
                        self.vars.active_mode = if m == "default" { None } else { Some(m.clone()) };
                        self.status = format!("mode: {m}");
                        return;
                    }
                    mx += w + 6.0;
                }
            }
            // catalog rows: bind actions
            let cat = self.vars.catalog();
            let mut y = y0 + 26.0;
            let mut last_col = String::new();
            for (collection, name, kind) in cat.iter().take(24) {
                if *collection != last_col { y += 16.0; last_col = collection.clone(); }
                if p.y >= y - 2.0 && p.y <= y + 14.0 {
                    if let Some(id) = self.editor.selection.first().cloned() {
                        match *kind {
                            "color" if p.x >= self.win_w - 48.0 => {
                                self.editor.set_fill(&id, Paint::Variable(name.clone()));
                                self.status = format!("fill of {id} -> var {name}");
                                return;
                            }
                            "number" if p.x >= self.win_w - 80.0 && p.x < self.win_w - 48.0 => {
                                if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                                    n.bindings.insert("radius".into(), name.clone());
                                    self.status = format!("radius of {id} -> var {name}");
                                }
                                return;
                            }
                            "number" if p.x >= self.win_w - 48.0 => {
                                if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
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
                if let arco_native::NodeKind::Instance { component } = n.kind.clone() {
                    let id = n.id.clone();
                    let ix2 = self.win_w - INSPECTOR_W;
                    let cy0 = TOP_H + IY_SEC;
                    // variant chips
                    if let Some((set, _)) = component.split_once('/') {
                        let vars_list: Vec<String> = arco_native::components::variants_of(&self.editor.root, set)
                            .iter().map(|s| s.to_string()).collect();
                        let mut vx = ix2 + 12.0;
                        let vy = cy0 + 16.0;
                        for vname in vars_list.iter().take(4) {
                            let short = vname.split_once('/').map(|(_, v)| v).unwrap_or(vname);
                            let cw = arco_native::text::measure(short, 7.5) + 10.0;
                            if p.x >= vx && p.x <= vx + cw && p.y >= vy - 2.0 && p.y <= vy + 12.0 {
                                self.editor.swap_instance(&id, vname);
                                self.status = format!("variant: {short}");
                                return;
                            }
                            vx += cw + 4.0;
                        }
                    }
                    // detach
                    if p.x >= ix2 + 150.0 && p.x <= ix2 + 208.0 && p.y >= cy0 + 14.0 && p.y <= cy0 + 30.0 {
                        let v = self.vars.clone();
                        if self.editor.detach_selected_instance(&v) {
                            self.status = "detached".into();
                        }
                        return;
                    }
                }
            }
        }
        // IMAGE controls: fit chips + replace (Design tab, image nodes)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if let arco_native::NodeKind::Image { asset, .. } = &n.kind {
                    let id = n.id.clone();
                    let cur_asset = asset.clone();
                    let ix2 = self.win_w - INSPECTOR_W;
                    let iy = TOP_H + IY_SEC;
                    // fit chips
                    if p.y >= iy + 14.0 && p.y <= iy + 30.0 {
                        for (i, fit) in [arco_native::ImageFit::Fill, arco_native::ImageFit::Fit, arco_native::ImageFit::Crop, arco_native::ImageFit::Tile].iter().enumerate() {
                            let bx = ix2 + 12.0 + i as f64 * 48.0;
                            if p.x >= bx && p.x <= bx + 44.0 {
                                if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                                    if let arco_native::NodeKind::Image { fit: f, .. } = &mut nm.kind { *f = *fit; nm.dirty = true; }
                                }
                                self.status = format!("image fit: {:?}", fit);
                                return;
                            }
                        }
                    }
                    // replace: cycle to the next loaded asset
                    if p.y >= iy + 36.0 && p.y <= iy + 52.0 && p.x >= ix2 + 12.0 && p.x <= ix2 + 96.0 {
                        let names = self.assets.names();
                        if names.is_empty() {
                            self.status = "no assets loaded (drop PNGs in assets/)".into();
                            return;
                        }
                        let pos = names.iter().position(|a| *a == cur_asset).unwrap_or(0);
                        let next = names[(pos + 1) % names.len()].clone();
                        if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                            if let arco_native::NodeKind::Image { asset: a, .. } = &mut nm.kind { *a = next.clone(); nm.dirty = true; }
                        }
                        self.status = format!("image -> {next}");
                        return;
                    }
                    // placement: focal X/Y steppers, scale, flips, reset
                    let mut placed: Option<String> = None;
                    {
                        let py = iy + 58.0;
                        let zy = iy + 78.0;
                        let mut edit = |app: &mut Self, f: &dyn Fn(&mut arco_native::ImagePlacement), what: &str| {
                            if let Some(nm) = arco_native::editor::find_mut(&mut app.editor.root, &id) {
                                if let arco_native::NodeKind::Image { placement, .. } = &mut nm.kind {
                                    f(placement);
                                    nm.dirty = true;
                                    return Some(format!("{what}: fx={:.2} fy={:.2} s={:.2} fh={} fv={}",
                                        placement.focal.0, placement.focal.1, placement.scale, placement.flip_h, placement.flip_v));
                                }
                            }
                            None
                        };
                        if p.y >= py - 3.0 && p.y <= py + 11.0 {
                            if p.x >= ix2 + 56.0 && p.x <= ix2 + 71.0 { placed = edit(self, &|pl| pl.focal.0 = (pl.focal.0 - 0.1).max(0.0), "focal x-"); }
                            else if p.x >= ix2 + 74.0 && p.x <= ix2 + 89.0 { placed = edit(self, &|pl| pl.focal.0 = (pl.focal.0 + 0.1).min(1.0), "focal x+"); }
                            else if p.x >= ix2 + 156.0 && p.x <= ix2 + 171.0 { placed = edit(self, &|pl| pl.focal.1 = (pl.focal.1 - 0.1).max(0.0), "focal y-"); }
                            else if p.x >= ix2 + 174.0 && p.x <= ix2 + 189.0 { placed = edit(self, &|pl| pl.focal.1 = (pl.focal.1 + 0.1).min(1.0), "focal y+"); }
                        } else if p.y >= zy - 3.0 && p.y <= zy + 11.0 {
                            if p.x >= ix2 + 84.0 && p.x <= ix2 + 99.0 { placed = edit(self, &|pl| pl.scale = (pl.scale - 0.1).max(0.1), "scale-"); }
                            else if p.x >= ix2 + 102.0 && p.x <= ix2 + 117.0 { placed = edit(self, &|pl| pl.scale = (pl.scale + 0.1).min(4.0), "scale+"); }
                            else if p.x >= ix2 + 124.0 && p.x <= ix2 + 146.0 { placed = edit(self, &|pl| pl.flip_h = !pl.flip_h, "flip h"); }
                            else if p.x >= ix2 + 150.0 && p.x <= ix2 + 172.0 { placed = edit(self, &|pl| pl.flip_v = !pl.flip_v, "flip v"); }
                            else if p.x >= ix2 + 178.0 && p.x <= ix2 + 214.0 { placed = edit(self, &|pl| *pl = arco_native::ImagePlacement::default(), "reset crop"); }
                        }
                    }
                    if let Some(msg) = placed { self.status = msg; return; }
                }
            }
        }
        // STYLES: create-from-selection (+P/+T/+FX) and apply chips
        // (text nodes hand this slot to the font browser)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if matches!(n.kind, arco_native::NodeKind::Text { .. }) { /* font browser owns the slot */ }
                else {
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
                                0 => (format!("Paint/{count}"), arco_native::Style::Paint { fill: n.fill.clone() }),
                                1 => (format!("Text/{count}"), arco_native::Style::Text {
                                    font: n.bindings.get("font").cloned().unwrap_or_default(),
                                    size: n.h, letter_spacing: 0.0, line_height: 1.2 }),
                                _ => (format!("FX/{count}"), arco_native::Style::Effect { effects: n.effects.clone() }),
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
                        if !(p.x >= r.x0 && p.x <= r.x1 && p.y >= r.y0 && p.y <= r.y1) { continue; }
                        match row_kind {
                            1 => continue, // header
                            2 => {
                                self.focus = Focus::StyleSearch;
                                self.status = "type to filter styles".into();
                                return;
                            }
                            3 => { self.run_style_action(&name); return; }
                            _ => {
                                let s = self.styles[&name].clone();
                                if self.shift {
                                    // SHIFT+click = redefine the style FROM the selection,
                                    // then propagate to every bound consumer (all pages)
                                    let newdef = if let Some(sel) = self.selected_single() {
                                        match &s {
                                            arco_native::Style::Paint { .. } => arco_native::Style::Paint { fill: sel.fill.clone() },
                                            arco_native::Style::Text { .. } => arco_native::Style::Text {
                                                font: sel.bindings.get("font").cloned().unwrap_or_default(),
                                                size: sel.h, letter_spacing: 0.0, line_height: 1.2 },
                                            arco_native::Style::Effect { .. } => arco_native::Style::Effect { effects: sel.effects.clone() },
                                        }
                                    } else { s.clone() };
                                    self.styles.insert(name.clone(), newdef);
                                    let mut updated = arco_native::resolve_styles(&mut self.editor.root, &self.styles);
                                    for (i, pg) in self.pages.iter_mut().enumerate() {
                                        if i != self.page_idx { updated += arco_native::resolve_styles(pg, &self.styles); }
                                    }
                                    self.status = format!("style {name} redefined -> {updated} consumer(s) updated");
                                } else if self.ctrl {
                                    // CTRL+click = select for management (REN/DUP/DEL/DET)
                                    self.style_sel = Some(name.clone());
                                    self.status = format!("style selected: {name} (REN/DUP/DEL/DET below)");
                                } else {
                                    if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                                        arco_native::bind_style(nm, &name, &s);
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
                let fill_len = if n.visual_stacks_materialized { n.fill_layers.len() } else { 1 };
                let fill_idx = self.fill_layer_index.min(fill_len.saturating_sub(1));
                let fill = n.fill_layers.get(fill_idx).map(|l| l.paint.clone()).unwrap_or_else(|| n.fill.clone());
                let fill_visible = n.fill_layers.get(fill_idx).map(|l| l.visible).unwrap_or(!n.visual_stacks_materialized || !n.fill_layers.is_empty());
                let stroke_len = if !n.visual_stacks_materialized { if n.stroke.width > 0.0 { 1 } else { 0 } } else { n.stroke_layers.len() };
                let stroke_idx = self.stroke_layer_index.min(stroke_len.saturating_sub(1));
                let stroke = n.stroke_layers.get(stroke_idx).map(|l| l.stroke).unwrap_or(n.stroke);
                let fx_len = if !n.visual_stacks_materialized { n.effects.len() } else { n.effect_layers.len() };
                let ix2 = self.win_w - INSPECTOR_W;
                let next_blend = |b: BlendKind| match b {
                    BlendKind::Normal => BlendKind::Darken, BlendKind::Darken => BlendKind::Multiply, BlendKind::Multiply => BlendKind::ColorBurn,
                    BlendKind::ColorBurn => BlendKind::Lighten, BlendKind::Lighten => BlendKind::Screen, BlendKind::Screen => BlendKind::ColorDodge,
                    BlendKind::ColorDodge => BlendKind::Overlay, BlendKind::Overlay => BlendKind::SoftLight, BlendKind::SoftLight => BlendKind::HardLight,
                    BlendKind::HardLight => BlendKind::Difference, BlendKind::Difference => BlendKind::Exclusion, BlendKind::Exclusion => BlendKind::Hue,
                    BlendKind::Hue => BlendKind::Saturation, BlendKind::Saturation => BlendKind::Color, BlendKind::Color => BlendKind::Luminosity,
                    BlendKind::Luminosity => BlendKind::Normal,
                };
                // Fill header row: GR toggle
                let hy = TOP_H + IY_FILL_HDR;
                if fill_len > 0 && p.y >= hy - 3.0 && p.y <= hy + 12.0 && p.x >= ix2 + 40.0 && p.x <= ix2 + 74.0 {
                    self.fill_layer_index = (fill_idx + 1) % fill_len; self.status = format!("fill layer {} selected", self.fill_layer_index + 1); return;
                }
                if p.y >= hy - 3.0 && p.y <= hy + 12.0 {
                    if p.x >= ix2 + 78.0 && p.x <= ix2 + 106.0 && fill_len > 0 {
                        self.editor.mutate_visual_stack(&id, |nm| { if let Some(l) = nm.fill_layers.get_mut(fill_idx) { l.blend = next_blend(l.blend); } });
                        self.status = "fill blend mode changed".into(); return;
                    }
                    if p.x >= ix2 + 108.0 && p.x <= ix2 + 126.0 && fill_idx + 1 < fill_len { self.editor.move_fill_layer(&id, fill_idx, fill_idx + 1); self.fill_layer_index += 1; return; }
                    if p.x >= ix2 + 128.0 && p.x <= ix2 + 146.0 && fill_idx > 0 { self.editor.move_fill_layer(&id, fill_idx, fill_idx - 1); self.fill_layer_index -= 1; return; }
                    if p.x >= ix2 + 148.0 && p.x <= ix2 + 166.0 { self.editor.remove_fill_layer(&id, fill_idx); self.fill_layer_index = self.fill_layer_index.saturating_sub(1); self.status = "fill layer removed".into(); return; }
                }
                if p.y >= hy - 3.0 && p.y <= hy + 11.0 && p.x >= ix2 + INSPECTOR_W - 28.0 {
                    self.editor.add_fill_layer(&id, Paint::Solid(Color::WHITE));
                    self.fill_layer_index = fill_len;
                    self.status = "fill layer added".into();
                    return;
                }
                if p.y >= hy - 2.0 && p.y <= hy + 12.0 && p.x >= ix2 + 178.0 && p.x <= ix2 + 206.0 {
                    let new_fill = match &fill {
                        Paint::LinearGradient { .. } | Paint::RadialGradient { .. } => Paint::Solid(C_ACCENT),
                        Paint::Solid(c) => Paint::LinearGradient {
                            start: (0.0, 0.0), end: (1.0, 0.0),
                            stops: vec![(0.0, *c), (1.0, Color::rgb8(0x8e, 0x2d, 0xe2))],
                        },
                        other => other.clone(),
                    };
                    let w = self.selected_single().map(|n| n.w).unwrap_or(100.0);
                    let new_fill = if let Paint::LinearGradient { start, stops, .. } = new_fill {
                        Paint::LinearGradient { start, end: (w, 0.0), stops }
                    } else { new_fill };
                    self.editor.set_fill(&id, new_fill);
                    self.gradient_editing = !matches!(fill, Paint::LinearGradient { .. } | Paint::RadialGradient { .. });
                    self.gradient_stop = 0;
                    self.status = "gradient toggled".into();
                    return;
                }
                // Fill row eye: toggle node visibility (mockup per-row eye)
                let fry = TOP_H + IY_FILLROW;
                if matches!(&fill, Paint::LinearGradient { .. } | Paint::RadialGradient { .. }) && p.y >= fry - 2.0 && p.y <= fry + 15.0 {
                    let row_x1 = ix2 + INSPECTOR_W - 12.0;
                    let stop_count = match &fill { Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => stops.len(), _ => 0 };
                    for i in 0..stop_count {
                        let cx = row_x1 - 92.0 + i as f64 * 16.0;
                        if (p.x - cx).abs() <= 8.0 { self.gradient_stop = i; self.status = format!("gradient stop {} selected", i + 1); return; }
                    }
                }
                if p.y >= fry - 2.0 && p.y <= fry + 15.0 && p.x >= ix2 + INSPECTOR_W - 36.0 {
                    self.editor.mutate_visual_stack(&id, |nm| {
                        if let Some(layer) = nm.fill_layers.get_mut(fill_idx) { layer.visible = !layer.visible; }
                    });
                    self.status = format!("fill {}", if fill_visible { "hidden" } else { "shown" });
                    return;
                }
                if fill_len > 0 && p.y >= fry - 2.0 && p.y <= fry + 15.0 {
                    let row_x1 = ix2 + INSPECTOR_W - 12.0;
                    if p.x >= row_x1 - 70.0 && p.x <= row_x1 - 38.0 {
                        let increase = p.x >= row_x1 - 54.0;
                        self.editor.mutate_visual_stack(&id, |nm| { if let Some(l) = nm.fill_layers.get_mut(fill_idx) { l.opacity = (l.opacity + if increase { 0.1 } else { -0.1 }).clamp(0.0, 1.0); } });
                        self.status = "fill opacity changed".into(); return;
                    }
                }
                let shy = TOP_H + IY_STROKE_HDR;
                if p.y >= shy - 3.0 && p.y <= shy + 12.0 && p.x >= ix2 + 40.0 && p.x <= ix2 + 74.0 && stroke_len > 0 {
                    self.stroke_layer_index = (stroke_idx + 1) % stroke_len; self.status = format!("stroke layer {} selected", self.stroke_layer_index + 1); return;
                }
                if p.y >= shy - 3.0 && p.y <= shy + 12.0 && stroke_len > 0 {
                    if p.x >= ix2 + 78.0 && p.x <= ix2 + 106.0 {
                        self.editor.mutate_visual_stack(&id, |nm| { if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) { l.blend = next_blend(l.blend); } });
                        self.status = "stroke blend mode changed".into(); return;
                    }
                    if p.x >= ix2 + 108.0 && p.x <= ix2 + 126.0 && stroke_idx + 1 < stroke_len { self.editor.move_stroke_layer(&id, stroke_idx, stroke_idx + 1); self.stroke_layer_index += 1; return; }
                    if p.x >= ix2 + 128.0 && p.x <= ix2 + 146.0 && stroke_idx > 0 { self.editor.move_stroke_layer(&id, stroke_idx, stroke_idx - 1); self.stroke_layer_index -= 1; return; }
                    if p.x >= ix2 + 148.0 && p.x <= ix2 + 166.0 { self.editor.remove_stroke_layer(&id, stroke_idx); self.stroke_layer_index = self.stroke_layer_index.saturating_sub(1); self.status = "stroke layer removed".into(); return; }
                }
                if p.y >= shy - 3.0 && p.y <= shy + 11.0 && p.x >= ix2 + INSPECTOR_W - 28.0 {
                    self.editor.add_stroke_layer(&id, arco_native::Stroke { color: Color::rgb8(0xe5, 0xe7, 0xeb), width: 1.0 });
                    self.stroke_layer_index = stroke_len;
                    self.status = "stroke layer added".into();
                    return;
                }
                // Stroke row: width -/+ steppers + swatch cycles palette
                let sry = TOP_H + IY_STROKEROW;
                if p.y >= sry - 2.0 && p.y <= sry + 15.0 {
                    let row_x1 = ix2 + INSPECTOR_W - 12.0;
                    if p.x >= row_x1 - 68.0 && p.x <= row_x1 - 6.0 {
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.options.align = match l.options.align { StrokeAlign::Inside => StrokeAlign::Center, StrokeAlign::Center => StrokeAlign::Outside, StrokeAlign::Outside => StrokeAlign::Inside };
                            }
                        });
                        self.status = "stroke alignment changed".into();
                        return;
                    }
                    if p.x >= row_x1 - 112.0 && p.x <= row_x1 - 96.0 {
                        let w0 = stroke.width;
                        self.editor.mutate_visual_stack(&id, |nm| { if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) { l.stroke.width = (l.stroke.width - 1.0).max(0.0); } });
                        self.status = format!("stroke {:.0}", (w0 - 1.0).max(0.0));
                        return;
                    }
                    if p.x >= row_x1 - 94.0 && p.x <= row_x1 - 78.0 {
                        let w0 = stroke.width;
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.stroke.width += 1.0;
                                if l.stroke.color.a == 0 { l.stroke.color = Color::rgb8(0xe5, 0xe7, 0xeb); }
                            }
                        });
                        self.status = format!("stroke {:.0}", w0 + 1.0);
                        return;
                    }
                    if p.x >= ix2 + 12.0 && p.x <= ix2 + 34.0 {
                        // swatch click: cycle stroke color through the palette
                        let cur = stroke.color;
                        let pos = PALETTE.iter().position(|c| *c == cur).map(|i| (i + 1) % PALETTE.len()).unwrap_or(0);
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(l) = nm.stroke_layers.get_mut(stroke_idx) {
                                l.stroke.color = PALETTE[pos];
                                if l.stroke.width == 0.0 { l.stroke.width = 1.0; }
                            }
                        });
                        self.status = format!("stroke color -> {}", arco_native::color_to_hex(PALETTE[pos]));
                        return;
                    }
                }
                // Effects: + button adds drop shadow; row eye removes it
                let fxh = TOP_H + IY_FX_HDR;
                let effect_idx = self.effect_layer_index.min(fx_len.saturating_sub(1));
                if p.y >= fxh - 3.0 && p.y <= fxh + 12.0 && p.x >= ix2 + 40.0 && p.x <= ix2 + 74.0 && fx_len > 0 {
                    self.effect_layer_index = (effect_idx + 1) % fx_len; self.status = format!("effect layer {} selected", self.effect_layer_index + 1); return;
                }
                if p.y >= fxh - 3.0 && p.y <= fxh + 12.0 && fx_len > 0 {
                    if p.x >= ix2 + 78.0 && p.x <= ix2 + 106.0 {
                        self.editor.mutate_visual_stack(&id, |nm| { if let Some(l) = nm.effect_layers.get_mut(effect_idx) { l.blend = next_blend(l.blend); } });
                        self.status = "effect blend mode changed".into(); return;
                    }
                    if p.x >= ix2 + 108.0 && p.x <= ix2 + 126.0 && effect_idx + 1 < fx_len { self.editor.move_effect_layer(&id, effect_idx, effect_idx + 1); self.effect_layer_index += 1; return; }
                    if p.x >= ix2 + 128.0 && p.x <= ix2 + 146.0 && effect_idx > 0 { self.editor.move_effect_layer(&id, effect_idx, effect_idx - 1); self.effect_layer_index -= 1; return; }
                    if p.x >= ix2 + 148.0 && p.x <= ix2 + 166.0 { self.editor.remove_effect_layer(&id, effect_idx); self.effect_layer_index = self.effect_layer_index.saturating_sub(1); self.status = "effect layer removed".into(); return; }
                }
                if p.y >= fxh - 3.0 && p.y <= fxh + 11.0 && p.x >= ix2 + INSPECTOR_W - 28.0 {
                    self.editor.add_effect_layer(&id, arco_native::Effect::DropShadow {
                        dx: 4.0, dy: 6.0, blur: 10.0, color: Color::rgba8(0, 0, 0, 160) });
                    self.effect_layer_index = fx_len;
                    self.status = "drop shadow added".into();
                    return;
                }
                if fx_len > 0 {
                    for i in 0..fx_len.min(4) {
                        let ry = TOP_H + IY_FXROW + i as f64 * 18.0;
                        if p.y >= ry - 2.0 && p.y <= ry + 14.0 {
                            if p.x >= ix2 + INSPECTOR_W - 36.0 {
                                self.editor.mutate_visual_stack(&id, |nm| { if i < nm.effect_layers.len() { nm.effect_layers[i].visible = !nm.effect_layers[i].visible; } });
                                self.status = "effect visibility toggled".into();
                                return;
                            }
                            if p.x >= ix2 + 12.0 && p.x <= ix2 + 150.0 {
                                self.effect_layer_index = i;
                                self.editor.mutate_visual_stack(&id, |nm| {
                                    if let Some(layer) = nm.effect_layers.get_mut(i) {
                                        layer.effect = match layer.effect.clone() {
                                            Effect::DropShadow { dx, dy, blur, color } => Effect::InnerShadow { dx, dy, blur, color },
                                            Effect::InnerShadow { blur, .. } => Effect::LayerBlur { radius: blur },
                                            Effect::LayerBlur { radius } => Effect::BackgroundBlur { radius },
                                            Effect::BackgroundBlur { radius } => Effect::DropShadow { dx: 4.0, dy: 6.0, blur: radius, color: Color::rgba8(0, 0, 0, 160) },
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
                                            Effect::DropShadow { blur, .. } | Effect::InnerShadow { blur, .. } => *blur = (*blur + delta).max(0.0),
                                            Effect::LayerBlur { radius } | Effect::BackgroundBlur { radius } => *radius = (*radius + delta).max(0.0),
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
                        use arco_native::editor::AlignKind::*;
                        let kind = [Left, CenterH, Right, Top, CenterV, Bottom][i];
                        let ids = self.editor.selection.clone();
                        if ids.len() >= 2 {
                            arco_native::editor::align(&mut self.editor.root, &ids, kind);
                            self.status = format!("aligned {:?}", kind);
                        } else if let Some(id) = ids.first() {
                            // single selection: align within its parent frame 
                            let rootw = self.editor.root.w; let rooth = self.editor.root.h;
                            if let Some(n) = arco_native::editor::find_mut(&mut self.editor.root, id) {
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
                            use arco_native::HPin::*;
                            let h = [Left, Right, CenterH, StretchH, ScaleH][i];
                            let v = find(&self.editor.root, &id).map(|n| n.pin.1).unwrap_or_default();
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
                            use arco_native::VPin::*;
                            let v = [Top, Bottom, CenterV, StretchV, ScaleV][i];
                            let h = find(&self.editor.root, &id).map(|n| n.pin.0).unwrap_or_default();
                            self.editor.set_pin(&id, h, v);
                            self.status = format!("v-pin {:?}", v);
                            return;
                        }
                    }
                }
            }
        }
        // FONT BROWSER + typography steppers (text node, Design tab)
        if self.inspector_tab == 0 {
            if let Some(n) = self.selected_single() {
                if matches!(n.kind, arco_native::NodeKind::Text { .. }) {
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
                                            let nh = (n.h + if up { 2.0 } else { -2.0 }).clamp(6.0, 400.0);
                                            let w = n.w;
                                            self.editor.resize(&id, w, nh);
                                            self.status = format!("text size {nh:.0}");
                                        }
                                        1 => {
                                            let cur = n.bindings.get("ls").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                                            let nv = (cur + if up { 0.5 } else { -0.5 }).clamp(-5.0, 40.0);
                                            if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
                                                nm.bindings.insert("ls".into(), format!("{nv}"));
                                                nm.dirty = true;
                                            }
                                            self.status = format!("letter spacing {nv:.1}");
                                        }
                                        _ => {
                                            let cur = n.bindings.get("lh").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.2);
                                            let nv = (cur + if up { 0.1 } else { -0.1 }).clamp(0.6, 3.0);
                                            if let Some(nm) = arco_native::editor::find_mut(&mut self.editor.root, &id) {
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
                    // search box (moved below the typography row)
                    if p.y >= fy + 34.0 && p.y <= fy + 50.0 && p.x >= ixf + 12.0 {
                        self.focus = Focus::FontSearch;
                        self.status = "type to search all fonts".into();
                        return;
                    }
                    // result rows
                    let visible = FONT_ROWS;
                    let start = self.font_scroll.min(self.font_results.len().saturating_sub(1));
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
                            let text = if *italic { "IT".to_string() } else { format!("{w}") };
                            let cw = arco_native::text::measure(&text, 7.5) + 10.0;
                            if wx + cw > self.win_w - 12.0 { wx = ixf + 12.0; wrow += 18.0; }
                            if p.x >= wx && p.x <= wx + cw && p.y >= wrow - 2.0 && p.y <= wrow + 12.0 {
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
                // radius steppers (right box)
                let rad = |app: &mut Self, d: f64| {
                    if let Some(nm) = arco_native::editor::find_mut(&mut app.editor.root, &id) {
                        if let arco_native::NodeKind::Rect { radius } = &mut nm.kind {
                            *radius = (*radius + d).max(0.0);
                            nm.dirty = true;
                            return Some(*radius);
                        }
                    }
                    None
                };
                if p.x >= ix + 182.0 && p.x <= ix + 198.0 {
                    if let Some(r) = rad(self, -2.0) { self.status = format!("radius {r:.0}"); }
                    return;
                }
                if p.x >= ix + 200.0 && p.x <= ix + 216.0 {
                    if let Some(r) = rad(self, 2.0) { self.status = format!("radius {r:.0}"); }
                    return;
                }
            }
            // per-corner mini boxes (TL TR BR BL): top half = +2, bottom = -2
            let ccy = TOP_H + IY_CORNERS;
            if p.y >= ccy - 3.0 && p.y <= ccy + 14.0 {
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
                    let is_gradient = self.selected_single().map(|n| {
                        let p = n.fill_layers.last().map(|l| &l.paint).unwrap_or(&n.fill);
                        matches!(p, Paint::LinearGradient { .. } | Paint::RadialGradient { .. })
                    }).unwrap_or(false);
                    if is_gradient {
                        self.editor.mutate_visual_stack(&id, |nm| {
                            if let Some(layer) = nm.fill_layers.last_mut() {
                                match &mut layer.paint {
                                    Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } if stop < stops.len() => stops[stop].1 = *color,
                                    _ => {}
                                }
                            }
                        });
                        self.status = format!("gradient stop {} -> {}", stop + 1, arco_native::color_to_hex(*color));
                    } else {
                        self.editor.set_fill(&id, Paint::Solid(*color));
                        self.status = format!("fill {} -> {}", id, arco_native::color_to_hex(*color));
                    }
                }
                return;
            }
        }
        // prototype link buttons (Prototype tab)
        if self.inspector_tab == 1 { if let Some(n) = self.selected_single() {
            let id = n.id.clone();
            let ix = self.win_w - INSPECTOR_W;
            let py = TOP_H + 40.0;
            if p.y >= py + 16.0 && p.y <= py + 60.0 {
                let mut bx = ix + 12.0;
                let mut by = py + 16.0;
                if p.x >= bx && p.x <= bx + 46.0 && p.y >= by && p.y <= by + 18.0 {
                    self.editor.set_prototype(&id, None);
                    self.status = format!("{id}: link cleared");
                    return;
                }
                bx += 52.0;
                let root_id = self.editor.root.id.clone();
                let page_ids: Vec<String> = self.pages.iter().map(|pg| pg.id.clone()).filter(|pid| pid != &root_id).collect();
                for pid in page_ids {
                    if bx + 60.0 > self.win_w - 8.0 { bx = ix + 12.0; by += 22.0; }
                    if p.x >= bx && p.x <= bx + 56.0 && p.y >= by && p.y <= by + 18.0 {
                        self.editor.set_prototype(&id, Some(arco_native::PrototypeAction { destination: pid.clone(), transition_ms: 350 }));
                        self.status = format!("{id} -> {pid} on click");
                        return;
                    }
                    bx += 62.0;
                }
            }
        }}
        // auto layout controls (frames only; mockup's Auto Layout section)
        if let Some(n) = self.selected_single() {
            if matches!(n.kind, arco_native::NodeKind::Frame { .. }) {
                let id = n.id.clone();
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
                                        gap: 16.0, padding: 16.0, align: CrossAlign::Center, ..Default::default()
                                    });
                                    l.direction = if i == 1 { LayoutDirection::Horizontal } else { LayoutDirection::Vertical };
                                    Some(l)
                                }
                            };
                            self.editor.set_auto_layout(&id, new_layout, &vars);
                            self.status = format!("layout: {}", ["none", "horizontal", "vertical"][i]);
                            return;
                        }
                    }
                }
                // GAP / PAD steppers
                if let Some(l) = self.editor.auto_layout_of(&id) {
                    for (row, is_gap) in [(0usize, true), (1, false)] {
                        let ry = ly + 44.0 + row as f64 * 22.0;
                        if p.y >= ry - 3.0 && p.y <= ry + 12.0 {
                            let delta = if p.x >= ix + 140.0 && p.x <= ix + 158.0 { -4.0 }
                                else if p.x >= ix + 162.0 && p.x <= ix + 180.0 { 4.0 }
                                else { continue };
                            let mut nl = l.clone();
                            if is_gap { nl.gap = (nl.gap + delta).max(0.0); } else { nl.padding = (nl.padding + delta).max(0.0); }
                            self.editor.set_auto_layout(&id, Some(nl.clone()), &vars);
                            self.status = format!("gap {:.0} pad {:.0}", nl.gap, nl.padding);
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn gradient_geometry(&self) -> Option<(usize, Point, Point, Vec<(f32, Color)>)> {
        if !self.gradient_editing || self.editor.selection.len() != 1 { return None; }
        let id = &self.editor.selection[0];
        let n = find(&self.editor.root, id)?;
        let fill = self.fill_layer_index.min(n.fill_layers.len().saturating_sub(1));
        let layer = n.fill_layers.get(fill)?;
        let (start, end, stops) = match &layer.paint {
            Paint::LinearGradient { start, end, stops } => (*start, *end, stops.clone()),
            Paint::RadialGradient { center, radius, stops } => (*center, (center.0 + *radius, center.1), stops.clone()),
            _ => return None,
        };
        let (world, _, _) = world_transform_of(&self.editor.root, id)?;
        let tx = self.camera() * world;
        Some((fill, tx * Point::new(start.0, start.1), tx * Point::new(end.0, end.1), stops))
    }

    fn gradient_handle_at(&self, p: Point) -> Option<(usize, usize)> {
        let (fill, start, end, stops) = self.gradient_geometry()?;
        let near = |a: Point| { let d = a - p; d.x.hypot(d.y) <= 9.0 };
        if near(start) { return Some((fill, 0)); }
        if near(end) { return Some((fill, 1)); }
        for (i, (t, _)) in stops.iter().enumerate().rev() {
            let q = start + (end - start) * *t as f64;
            if near(q) { return Some((fill, i + 2)); }
        }
        None
    }

    fn gradient_line_position(&self, p: Point) -> Option<(usize, f32)> {
        let (fill, start, end, _) = self.gradient_geometry()?;
        let v = end - start;
        let len2 = v.x * v.x + v.y * v.y;
        if len2 <= 0.01 { return None; }
        let t = (((p.x - start.x) * v.x + (p.y - start.y) * v.y) / len2).clamp(0.0, 1.0);
        let q = start + v * t;
        let d = q - p;
        if d.x.hypot(d.y) <= 8.0 { Some((fill, t as f32)) } else { None }
    }

    // ------------------------------------------------------------ rendering

}

fn gradient_color_at(stops: &[(f32, Color)], t: f32) -> Color {
    let Some(first) = stops.first() else { return Color::TRANSPARENT; };
    if t <= first.0 { return first.1; }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b.0 {
            let u = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 0.0 };
            let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * u).round() as u8;
            return Color::rgba8(mix(a.1.r, b.1.r), mix(a.1.g, b.1.g), mix(a.1.b, b.1.b), mix(a.1.a, b.1.a));
        }
    }
    stops.last().map(|s| s.1).unwrap_or(Color::TRANSPARENT)
}
