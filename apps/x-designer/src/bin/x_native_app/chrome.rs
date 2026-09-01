#[allow(unused_imports)]
use super::*;

impl App {
    pub fn build_display_scene(&mut self) -> Scene {
        // presentation mode: full-window playback, no chrome
        if self.present.is_some() {
            if let Some(frame) = self.present_frame() {
                let mut ui = Scene::new();
                let (scene, _) = arco_native::build_scene_full(&frame, None, &self.vars, Some(&self.assets), if self.fonts.fonts.is_empty() { None } else { Some(&self.fonts) });
                // fit page into window
                let scale = (self.win_w / frame.w.max(1.0)).min(self.win_h / frame.h.max(1.0));
                let ox = (self.win_w - frame.w * scale) / 2.0;
                let oy = (self.win_h - frame.h * scale) / 2.0;
                ui.append(&scene, Some(Affine::translate((ox, oy)) * Affine::scale(scale)));
                label(&mut ui, "PRESENTING - ESC TO EXIT", 10.0, self.win_h - STATUS_H - 8.0, C_DIM);
                return ui;
            }
        }
        // ---------- DASHBOARD screen (X-Native home with recent files) ----------
        if self.screen == Screen::Dashboard {
            return self.build_dashboard_scene();
        }
        self.rebuild_layer_rows();
        let chrome_t0 = std::time::Instant::now();
        let mut ui = Scene::new();

        // document, clipped to canvas
        let canvas = self.canvas_rect();
        // AUDIT FIX: C_CANVAS was declared in theme.rs but never actually
        // painted — the pasteboard around frames rendered as pure
        // transparent (black void on most backends) instead of a real
        // canvas backdrop. Fill it before anything else draws over it.
        fill_rect(&mut ui, canvas, C_CANVAS);
        ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY, &canvas);
        // soft drop shadow behind each top-level frame — pages read as
        // "paper" lifted off the pasteboard instead of flat rectangles
        // sitting directly on it. Cheap: one rect per frame, same screen
        // transform already used for the frame-name labels below; never
        // touches the cached document scene, so it can't affect the
        // dirty-subtree cache hit rate.
        {
            let cam = self.camera();
            for child in &self.editor.root.children {
                if !matches!(child.kind, arco_native::NodeKind::Frame { .. }) || !child.visible { continue; }
                let tl = cam * Point::new(child.transform.x, child.transform.y);
                let br = cam * Point::new(child.transform.x + child.w, child.transform.y + child.h);
                let shadow = Rect::new(tl.x + 2.0, tl.y + 4.0, br.x + 2.0, br.y + 7.0);
                fill_rect(&mut ui, shadow, Color::rgba8(0, 0, 0, 60));
            }
        }
        if self.outline_view {
            // X-Native outline mode: strokes only, no fills for clean structure view
            fn outline_walk(n: &Node, parent: Affine, cam: Affine, ui: &mut Scene) {
                if !n.visible { return; }
                let world = parent * n.transform.matrix(n.w, n.h);
                let b = quad_bounds(cam * world, n.w, n.h);
                if !matches!(n.kind, arco_native::NodeKind::Component { .. }) {
                    stroke_rect(ui, b, Color::rgb8(0x9a, 0x9a, 0x9a), 1.0);
                }
                for c in &n.children { outline_walk(c, world, cam, ui); }
            }
            let cam = self.camera();
            let root = self.editor.root.clone();
            for c in &root.children { outline_walk(c, Affine::IDENTITY, cam, &mut ui); }
        } else {
            // DIRTY-SUBTREE IR REUSE: hash walk decides; unchanged frames
            // skip lowering AND encoding, partial changes re-lower only
            // the moved top-level subtree and splice cached segments.
            let mut cache = std::mem::take(&mut self.scene_cache); // borrow split
            // VIEWPORT CULLING: canvas rect -> world space, +25% margin so
            // small pans stay cache hits and edge pops are impossible
            let vp = {
                let c = self.canvas_rect();
                let inv = self.camera().inverse();
                let a = inv * Point::new(c.x0, c.y0);
                let b = inv * Point::new(c.x1, c.y1);
                let w = (b.x - a.x).abs();
                let h = (b.y - a.y).abs();
                Rect::new(a.x.min(b.x) - w * 0.25, a.y.min(b.y) - h * 0.25,
                          a.x.max(b.x) + w * 0.25, a.y.max(b.y) + h * 0.25)
            };
            let doc_scene = {
                let sink = arco_native::VelloSink {
                    assets: Some(&self.assets),
                    fonts: if self.fonts.fonts.is_empty() { None } else { Some(&self.fonts) },
                };
                cache.render_viewport(&self.editor.root, &self.vars, &sink, Some(vp)).clone()
            };
            let st = cache.stats;
            self.scene_cache = cache;
            self.encode_skipped = st.full_hit;
            self.phase_ms.0 = st.hash_ms + st.lower_ms;
            self.phase_ms.1 = st.encode_ms;
            ui.append(&doc_scene, Some(self.camera()));
        }

        // ---- frame name labels above top-level frames (professional design tool standard)
        // "◇ Desktop - 1440" floating over each frame) ----
        for child in &self.editor.root.children {
            if !matches!(child.kind, arco_native::NodeKind::Frame { .. }) { continue; }
            if !child.visible { continue; }
            let tl = self.camera() * Point::new(child.transform.x, child.transform.y);
            if tl.y < TOP_H + 30.0 { continue; }
            let d = 3.5;
            let (cx, cy) = (tl.x + 6.0, tl.y - 14.0);
            let mut dia = vello::kurbo::BezPath::new();
            dia.move_to((cx, cy - d)); dia.line_to((cx + d, cy));
            dia.line_to((cx, cy + d)); dia.line_to((cx - d, cy));
            dia.close_path();
            ui.stroke(&vello::kurbo::Stroke::new(1.1), Affine::IDENTITY, C_DIM, None, &dia);
            let selected = self.editor.selection.contains(&child.id);
            label(&mut ui, &child.id, tl.x + 18.0, tl.y - 20.0, 9.0,
                if selected { C_ACCENT } else { C_DIM });
        }

        // user guides (cyan, professional canvas guides)
        for (vertical, coord) in &self.user_guides {
            let line = if *vertical {
                let a = self.camera() * Point::new(*coord, -100000.0);
                let b = self.camera() * Point::new(*coord, 100000.0);
                vello::kurbo::Line::new(a, b)
            } else {
                let a = self.camera() * Point::new(-100000.0, *coord);
                let b = self.camera() * Point::new(100000.0, *coord);
                vello::kurbo::Line::new(a, b)
            };
            ui.stroke(&vello::kurbo::Stroke::new(1.0), Affine::IDENTITY, Color::rgba8(0x00, 0xbc, 0xd4, 180), None, &line);
        }

        // smart guides (red lines) while dragging
        for (vertical, coord) in &self.guides {
            let line = if *vertical {
                let a = self.camera() * Point::new(*coord, -100000.0);
                let b = self.camera() * Point::new(*coord, 100000.0);
                vello::kurbo::Line::new(a, b)
            } else {
                let a = self.camera() * Point::new(-100000.0, *coord);
                let b = self.camera() * Point::new(100000.0, *coord);
                vello::kurbo::Line::new(a, b)
            };
            ui.stroke(&vello::kurbo::Stroke::new(1.0), Affine::IDENTITY, Color::rgb8(0xff, 0x3b, 0x30), None, &line);
        }

        // ---- node-edit anchors (vector editing) ----
        if let Some(vid) = &self.node_edit {
            if let Some(n) = find(&self.editor.root, vid) {
                if let arco_native::NodeKind::Vector { path } = &n.kind {
                    for (ai, a) in arco_native::editor::anchors(path).iter().enumerate() {
                        let a = *a;
                        // outgoing handle (c1 of next segment)
                        if let Some((ox, oy)) = self.editor.out_handle(vid, ai) {
                            let sp0 = self.camera() * Point::new(a.x + n.transform.x, a.y + n.transform.y);
                            let hp = self.camera() * Point::new(ox + n.transform.x, oy + n.transform.y);
                            ui.stroke(&vello::kurbo::Stroke::new(1.0), Affine::IDENTITY, Color::rgba8(0x7c, 0x5c, 0xfc, 170), None,
                                &vello::kurbo::Line::new((sp0.x, sp0.y), (hp.x, hp.y)));
                            ui.fill(Fill::NonZero, Affine::IDENTITY, C_ACCENT, None, &vello::kurbo::Circle::new((hp.x, hp.y), 2.5));
                        }
                        let sp = self.camera() * Point::new(a.x + n.transform.x, a.y + n.transform.y);
                        let r = Rect::new(sp.x - 4.0, sp.y - 4.0, sp.x + 4.0, sp.y + 4.0);
                        // curve anchors round, corner anchors square (standard vector convention)
                        if a.in_handle.is_some() {
                            ui.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &vello::kurbo::Circle::new((sp.x, sp.y), 4.0));
                            ui.stroke(&vello::kurbo::Stroke::new(1.4), Affine::IDENTITY, C_ACCENT, None, &vello::kurbo::Circle::new((sp.x, sp.y), 4.0));
                        } else {
                            fill_rect(&mut ui, r, Color::WHITE);
                            stroke_rect(&mut ui, r, C_ACCENT, 1.4);
                        }
                        // incoming control handle line
                        if let Some((hx, hy)) = a.in_handle {
                            let hp = self.camera() * Point::new(hx + n.transform.x, hy + n.transform.y);
                            ui.stroke(&vello::kurbo::Stroke::new(1.0), Affine::IDENTITY, Color::rgba8(0x7c, 0x5c, 0xfc, 170), None,
                                &vello::kurbo::Line::new((sp.x, sp.y), (hp.x, hp.y)));
                            ui.fill(Fill::NonZero, Affine::IDENTITY, C_ACCENT, None, &vello::kurbo::Circle::new((hp.x, hp.y), 2.5));
                        }
                    }
                }
            }
        }
        // hover highlight (thin outline, no handles)
        if let Some(hid) = &self.hover {
            if let Some((world, w, h)) = world_transform_of(&self.editor.root, hid) {
                let b = quad_bounds(self.camera() * world, w, h);
                stroke_rect(&mut ui, b, Color::rgba8(0x7c, 0x5c, 0xfc, 170), 1.0);
            }
        }
        // prototype link badges: small purple arrow chip on linked nodes
        {
            fn walk_badges(n: &Node, parent: Affine, cam: Affine, ui: &mut Scene) {
                let world = parent * n.transform.matrix(n.w, n.h);
                if n.prototype.is_some() && n.visible {
                    let b = quad_bounds(cam * world, n.w, n.h);
                    let chip = Rect::new(b.x1 - 16.0, b.y0 - 8.0, b.x1 + 2.0, b.y0 + 8.0);
                    fill_rect(ui, chip, PALETTE[3]);
                    label(ui, ">", chip.x0 + 4.0, chip.y0 + 2.0, 8.0, Color::WHITE);
                }
                for c in &n.children { walk_badges(c, world, cam, ui); }
            }
            let cam = self.camera();
            let root = self.editor.root.clone();
            walk_badges(&root, Affine::IDENTITY, cam, &mut ui);
        }
        // selection outlines + handles
        for id in self.editor.selection.clone() {
            if let Some((world, w, h)) = world_transform_of(&self.editor.root, &id) {
                let b = quad_bounds(self.camera() * world, w, h);
                let editing_this = matches!(&self.focus, Focus::TextNode { id: eid, .. } if eid == &id);
                stroke_rect(&mut ui, b.inflate(1.5, 1.5), if editing_this { PALETTE[4] } else { C_ACCENT }, 1.5);
                if editing_this {
                    // selection range highlight + caret line (x measured by
                    // shaping prefixes with the node's own typography)
                    if let (Focus::TextNode { buffer, caret, sel_anchor, .. }, Some(n)) = (&self.focus, find(&self.editor.root, &id)) {
                        let ls = n.bindings.get("ls").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                        let x_at = |idx: usize| -> f64 {
                            let pre = &buffer[..idx.min(buffer.len())];
                            b.x0 + (arco_native::text::measure(pre, n.h * 0.72)
                                + ls * pre.chars().count() as f64) * self.zoom
                        };
                        if let Some(a) = sel_anchor {
                            let (lo, hi) = ((*a).min(*caret), (*a).max(*caret));
                            if hi > lo {
                                fill_rect(&mut ui, Rect::new(x_at(lo), b.y0 + 2.0, x_at(hi), b.y1 - 2.0),
                                    Color::rgba8(0x7c, 0x5c, 0xfc, 72));
                            }
                        }
                        let cx = x_at(*caret);
                        ui.stroke(&vello::kurbo::Stroke::new(1.6), Affine::IDENTITY, PALETTE[4], None,
                            &vello::kurbo::Line::new((cx, b.y0 + 2.0), (cx, b.y1 - 2.0)));
                    }
                }
                if self.editor.selection.len() == 1 && !editing_this {
                    // X-Native: 4 small corner squares only (no knob, no stem,
                    // no edge dots — edges are grabbable but invisible;
                    // rotation lives in the invisible ring outside corners)
                    for (cx, cy) in [(b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)] {
                        let hr = Rect::new(cx - 3.0, cy - 3.0, cx + 3.0, cy + 3.0);
                        fill_rect(&mut ui, hr, Color::WHITE);
                        stroke_rect(&mut ui, hr, C_ACCENT, 1.0);
                    }
                }
                if self.editor.selection.len() == 1 && self.gradient_editing && !editing_this {
                    if let Some((_, start, end, stops)) = self.gradient_geometry() {
                        ui.stroke(&vello::kurbo::Stroke::new(1.5), Affine::IDENTITY, Color::rgba8(0xff, 0xff, 0xff, 220), None,
                            &vello::kurbo::Line::new((start.x, start.y), (end.x, end.y)));
                        ui.stroke(&vello::kurbo::Stroke::new(3.5), Affine::IDENTITY, C_ACCENT, None,
                            &vello::kurbo::Line::new((start.x, start.y), (end.x, end.y)));
                        for (i, (t, color)) in stops.iter().enumerate() {
                            let q = start + (end - start) * *t as f64;
                            ui.fill(Fill::NonZero, Affine::IDENTITY, *color, None, &vello::kurbo::Circle::new((q.x, q.y), 5.5));
                            ui.stroke(&vello::kurbo::Stroke::new(if i == self.gradient_stop { 2.0 } else { 1.0 }), Affine::IDENTITY,
                                if i == self.gradient_stop { Color::WHITE } else { C_ACCENT }, None, &vello::kurbo::Circle::new((q.x, q.y), 5.5));
                        }
                        for q in [start, end] {
                            ui.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &vello::kurbo::Circle::new((q.x, q.y), 4.0));
                            ui.stroke(&vello::kurbo::Stroke::new(1.5), Affine::IDENTITY, C_ACCENT, None, &vello::kurbo::Circle::new((q.x, q.y), 4.0));
                        }
                    }
                }
                if self.editor.selection.len() == 1 {
                    // X-Native dimension badge: violet pill under the selection
                    if let Some(n) = find(&self.editor.root, &id) {
                        let text = format!("{:.0} X {:.0}", n.w, n.h);
                        let tw = arco_native::text::measure(&text, 9.0);
                        let bx = (b.x0 + b.x1) / 2.0 - tw / 2.0 - 6.0;
                        let by = b.y1 + 8.0;
                        let badge = vello::kurbo::RoundedRect::new(bx, by, bx + tw + 14.0, by + 18.0, 4.0);
                        ui.fill(Fill::NonZero, Affine::IDENTITY, C_ACCENT, None, &badge);
                        label(&mut ui, &text, bx + 6.0, by + 4.0, 9.0, Color::WHITE);
                    }
                }
                if editing_this {
                    // caret hint: yellow underline across the text box
                    ui.stroke(&vello::kurbo::Stroke::new(2.0), Affine::IDENTITY, PALETTE[4], None,
                        &vello::kurbo::Line::new((b.x0, b.y1 + 3.0), (b.x1, b.y1 + 3.0)));
                }
            }
        }
        // live marquee / create preview
        match self.drag {
            Drag::Marquee { start_world } => {
                let a = self.camera() * start_world;
                let bpt = self.cursor;
                let r = Rect::new(a.x.min(bpt.x), a.y.min(bpt.y), a.x.max(bpt.x), a.y.max(bpt.y));
                ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgba8(0x7c, 0x5c, 0xfc, 30), None, &r.into_path(0.1));
                stroke_rect(&mut ui, r, C_ACCENT, 1.0);
            }
            Drag::Create { start_world } => {
                let world = self.creation_rect(start_world, self.world_point(self.cursor));
                let a = self.camera() * Point::new(world.x0, world.y0);
                let b = self.camera() * Point::new(world.x1, world.y1);
                let r = Rect::new(a.x, a.y, b.x, b.y);
                ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgba8(0x7c, 0x5c, 0xfc, 30), None, &r.into_path(0.1));
                stroke_rect(&mut ui, r, C_ACCENT, 1.0);
            }
            _ => {}
        }
        ui.pop_layer();

        // X-Native \"hide interface\": canvas only + tiny hint
        if self.chrome_hidden {
            label(&mut ui, "⌘. TO SHOW UI", 10.0, self.win_h - STATUS_H - 8.0, C_DIM);
            return ui;
        }

        // ---------- chrome: two-row header (mockup) ----------
        // row 1: tab strip — logo + document tab
        fill_rect(&mut ui, Rect::new(0.0, 0.0, self.win_w, TAB_H), C_PANEL);
        {
            // X logo mark (accent glyph)
            let mut xmark = vello::kurbo::BezPath::new();
            xmark.move_to((14.0, 10.0)); xmark.line_to((19.0, 16.0)); xmark.line_to((14.0, 22.0));
            xmark.line_to((17.5, 22.0)); xmark.line_to((21.0, 17.8)); xmark.line_to((24.5, 22.0));
            xmark.line_to((28.0, 22.0)); xmark.line_to((23.0, 16.0)); xmark.line_to((28.0, 10.0));
            xmark.line_to((24.5, 10.0)); xmark.line_to((21.0, 14.2)); xmark.line_to((17.5, 10.0));
            xmark.close_path();
            ui.fill(Fill::NonZero, Affine::IDENTITY, C_ACCENT, None, &xmark);
            label(&mut ui, "X-NATIVE", 34.0, 11.0, 10.0, C_TEXT);
            // active document tab
            // real file name (dashboard identity), not a hardcoded string
            let doc_name = self.dash_files.iter().find(|f| f.path == self.doc_path)
                .map(|f| f.name.clone())
                .unwrap_or_else(|| if self.doc_path == "document.x" { "Brand Dashboard".into() } else {
                    std::path::Path::new(&self.doc_path).file_stem().unwrap_or_default().to_string_lossy().to_string()
                });
            let doc_name = doc_name.as_str();
            let tabw = ui_measure(doc_name, 10.0) + 48.0;
            let tab = Rect::new(130.0, 0.0, 130.0 + tabw, TAB_H);
            fill_rect(&mut ui, tab, C_PANEL2);
            fill_rect(&mut ui, Rect::new(tab.x0, 0.0, tab.x1, 2.0), C_ACCENT);
            label(&mut ui, doc_name, tab.x0 + 16.0, 11.0, 10.0,
                if self.dirty_since_save { PALETTE[4] } else { C_TEXT });
            if self.dirty_since_save {
                ui.fill(Fill::NonZero, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Circle::new((tab.x1 - 16.0, TAB_H / 2.0), 3.5));
            }
            label(&mut ui, "+", tab.x1 + 14.0, 8.0, 13.0, C_DIM);
            // caret next to product name (mockup)
            {
                let nx = 36.0 + ui_measure("X-NATIVE", 10.0) + 10.0;
                let mut car = vello::kurbo::BezPath::new();
                car.move_to((nx, 14.0)); car.line_to((nx + 7.0, 14.0)); car.line_to((nx + 3.5, 18.0));
                car.close_path();
                ui.fill(Fill::NonZero, Affine::IDENTITY, C_DIM, None, &car);
            }
            // window controls (visual, mockup right corner)
            {
                let wy = TAB_H / 2.0;
                let st = vello::kurbo::Stroke::new(1.3);
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((self.win_w - 80.0, wy), (self.win_w - 70.0, wy)));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &Rect::new(self.win_w - 56.0, wy - 5.0, self.win_w - 46.0, wy + 5.0).to_path(0.1));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((self.win_w - 32.0, wy - 5.0), (self.win_w - 22.0, wy + 5.0)));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((self.win_w - 32.0, wy + 5.0), (self.win_w - 22.0, wy - 5.0)));
            }
        }
        // row 2: menus + centered tools + zoom + Present
        let r2y = TAB_H;
        fill_rect(&mut ui, Rect::new(0.0, r2y, self.win_w, TOP_H), C_PANEL2);
        fill_rect(&mut ui, Rect::new(0.0, TOP_H - 1.0, self.win_w, TOP_H), C_PANEL_EDGE);
        {
            // menu titles (REAL dropdowns — geometry shared with the click
            // handler via menu_title_rects)
            for (i, r) in self.menu_title_rects() {
                let open = self.menu_open == Some(i);
                if open { fill_rrect(&mut ui, r, 5.0, C_HOVERBG); }
                label(&mut ui, MENUS[i].0, r.x0 + 6.0, r2y + 17.0, 9.5, if open { Color::WHITE } else { C_TEXT });
            }
            // centered tool row (moved from the floating bottom bar)
            let bar = self.bottom_bar_rect();
            let bar_shadow = Rect::new(bar.x0 + 1.0, bar.y0 + 3.0, bar.x1 + 1.0, bar.y1 + 4.0);
            fill_rrect(&mut ui, bar_shadow, 10.0, Color::rgba8(0, 0, 0, 80));
            fill_rrect(&mut ui, bar, 10.0, C_PANEL);
            ui.stroke(&vello::kurbo::Stroke::new(1.2), Affine::IDENTITY, C_PANEL_EDGE, None,
                &vello::kurbo::RoundedRect::new(bar.x0, bar.y0, bar.x1, bar.y1, 10.0));
            for (i, t) in Tool::ALL.iter().enumerate() {
                let cx = bar.x0 + 10.0 + i as f64 * 40.0 + 17.0;
                let cy = (bar.y0 + bar.y1) / 2.0;
                let slot = Rect::new(cx - 17.0, bar.y0 + 5.0, cx + 17.0, bar.y1 - 5.0);
                if *t == self.tool {
                    fill_rrect(&mut ui, slot, 7.0, C_ACCENT);
                } else if slot.contains(self.cursor) {
                    fill_rrect(&mut ui, slot, 7.0, C_HOVERBG);
                }
                draw_tool_icon(&mut ui, *t, cx, cy, if *t == self.tool { Color::WHITE } else { C_DIM });
            }
            // zoom pill + Prototype + Present from ONE shared geometry
            // (header_rects — the click handler uses the same rects)
            let (bm, bl, bp, ppr, pr) = self.header_rects();
            fill_rrect(&mut ui, Rect::new(bm.x0, bm.y0, bp.x1, bp.y1), 7.0, C_FIELD);
            label(&mut ui, "-", bm.x0 + 10.0, r2y + 17.0, 12.0, C_DIM);
            let ztxt = format!("{}%", (self.zoom * 100.0).round());
            let tw = ui_measure(&ztxt, 9.5);
            label(&mut ui, &ztxt, bl.x0 + (bl.width() - tw) / 2.0, r2y + 18.0, 9.5, C_TEXT);
            label(&mut ui, "+", bp.x0 + 9.0, r2y + 17.0, 12.0, C_DIM);
            // Prototype ghost button (mockup: play outline + label)
            {
                let st = vello::kurbo::Stroke::new(1.3)
                    .with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round);
                let mut tri = vello::kurbo::BezPath::new();
                tri.move_to((ppr.x0 + 10.0, ppr.y0 + 9.0));
                tri.line_to((ppr.x0 + 10.0, ppr.y1 - 9.0));
                tri.line_to((ppr.x0 + 19.0, (ppr.y0 + ppr.y1) / 2.0));
                tri.close_path();
                ui.stroke(&st, Affine::IDENTITY, C_TEXT, None, &tri);
                label(&mut ui, "Prototype", ppr.x0 + 25.0, r2y + 17.0, 9.5, C_TEXT);
            }
            // Present button (accent pill, mockup's > Present)
            fill_rrect(&mut ui, pr, 6.0, if pr.contains(self.cursor) { Color::rgb8(0x4d, 0x7a, 0xff) } else { C_ACCENT });
            let mut tri = vello::kurbo::BezPath::new();
            tri.move_to((pr.x0 + 12.0, pr.y0 + 8.0));
            tri.line_to((pr.x0 + 12.0, pr.y1 - 8.0));
            tri.line_to((pr.x0 + 21.0, (pr.y0 + pr.y1) / 2.0));
            tri.close_path();
            ui.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &tri);
            label(&mut ui, "Present", pr.x0 + 26.0, r2y + 17.0, 9.5, Color::WHITE);
            // avatar circle (mockup, visual, far right)
            {
                let ac = (self.win_w - 24.0, (r2y + TOP_H) / 2.0);
                ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(0x8e, 0x6a, 0x4f), None,
                    &vello::kurbo::Circle::new(ac, 12.0));
                ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(0xe8, 0xc9, 0xa8), None,
                    &vello::kurbo::Circle::new((ac.0, ac.1 - 3.0), 5.0));
                ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(0xe8, 0xc9, 0xa8), None,
                    &vello::kurbo::Circle::new((ac.0, ac.1 + 9.0), 8.0));
                ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY,
                    &vello::kurbo::Circle::new(ac, 12.0).to_path(0.1));
                ui.pop_layer();
            }
        }
        // status line (bottom status bar, mockup)
        let status_line = match &self.focus {
            Focus::TextNode { buffer, .. } => format!("TEXT> {buffer}_"),
            Focus::Field { field, buffer, .. } => format!("{}> {buffer}_", ["X", "Y", "W", "H"][*field as usize]),
            Focus::LayerSearch => format!("FIND> {}_", self.layer_filter),
            Focus::LayerRename { id, buffer } => format!("RENAME {id} → {buffer}_"),
            Focus::FontSearch => format!("FONT> {}_", self.font_query),
            Focus::StyleSearch => format!("STYLE> {}_", self.style_query),
            Focus::StyleRename { from, buffer } => format!("RENAME {from} -> {buffer}_"),
            Focus::AssetSearch => format!("ASSET> {}_", self.asset_query),
            Focus::AssetRename { buffer, .. } => format!("ASSET NAME> {buffer}_"),
            Focus::PageRename { idx, buffer } => format!("RENAME PAGE '{}' > {buffer}_", self.pages.get(*idx).map(|p| p.id.as_str()).unwrap_or("?")),
            Focus::DashSearch => format!("FIND FILE> {}_", self.dash_query),
            Focus::DashRename { buffer, .. } => format!("FILE NAME> {buffer}_"),
            Focus::None => self.status.clone(),
        };
        // ---------- bottom page-thumbnail strip (mockup; collapsible) ----------
        if self.thumbs_collapsed {
            // slim bar: page name + count + expand chevron; click = expand
            let tr = self.thumbs_rect();
            fill_rect(&mut ui, Rect::new(0.0, tr.y0, self.win_w, tr.y1), C_PANEL);
            fill_rect(&mut ui, Rect::new(0.0, tr.y0, self.win_w, tr.y0 + 1.0), C_PANEL_EDGE);
            label(&mut ui, &format!("Pages ({}) — {}", self.pages.len(), self.editor.root.id), tr.x0 + 14.0, tr.y0 + 6.0, 8.5, C_DIM);
            // up chevron
            let (cx, cy) = (tr.x1 - 16.0, tr.y0 + 11.0);
            let st = vello::kurbo::Stroke::new(1.4).with_caps(vello::kurbo::Cap::Round);
            let mut ch = vello::kurbo::BezPath::new();
            ch.move_to((cx - 5.0, cy + 2.5)); ch.line_to((cx, cy - 2.5)); ch.line_to((cx + 5.0, cy + 2.5));
            ui.stroke(&st, Affine::IDENTITY, C_DIM, None, &ch);
        } else {
            let tr = self.thumbs_rect();
            fill_rect(&mut ui, Rect::new(0.0, tr.y0, self.win_w, tr.y1), C_PANEL);
            fill_rect(&mut ui, Rect::new(0.0, tr.y0, self.win_w, tr.y0 + 1.0), C_PANEL_EDGE);
            // collapse chevron (down) at the strip's right edge
            {
                let (cx, cy) = (tr.x1 - 16.0, tr.y0 + 11.0);
                let st = vello::kurbo::Stroke::new(1.4).with_caps(vello::kurbo::Cap::Round);
                let mut ch = vello::kurbo::BezPath::new();
                ch.move_to((cx - 5.0, cy - 2.5)); ch.line_to((cx, cy + 2.5)); ch.line_to((cx + 5.0, cy - 2.5));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None, &ch);
            }
            let cell_w = 96.0;
            let cell_h = 54.0;
            let mut x = tr.x0 + 14.0;
            let ty = tr.y0 + 10.0;
            for (i, page) in self.pages.iter().enumerate() {
                let cell = Rect::new(x, ty, x + cell_w, ty + cell_h);
                let cell_hover = cell.contains(self.cursor);
                // page thumbnail through the REAL render IR
                let page_ref = if i == self.page_idx { &self.editor.root } else { page };
                let tree = arco_native::build_render_tree(page_ref, &self.vars);
                let (thumb, _) = arco_native::thumbnail_scene(&tree, page_ref.w.max(1.0), page_ref.h.max(1.0), cell_w, cell_h);
                fill_rrect(&mut ui, cell, 4.0, Color::rgb8(0x30, 0x32, 0x38));
                ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY, &cell.to_path(0.1));
                ui.append(&thumb, Some(Affine::translate((x, ty))));
                ui.pop_layer();
                if i == self.page_idx {
                    stroke_rect(&mut ui, cell, C_ACCENT, 2.0);
                    fill_rrect(&mut ui, Rect::new(x, ty + cell_h + 4.0, x + cell_w, ty + cell_h + 18.0), 3.0, C_ACCENT);
                } else if cell_hover {
                    stroke_rect(&mut ui, cell, Color::rgba8(0x7c, 0x5c, 0xfc, 170), 1.5);
                }
                let renaming = matches!(&self.focus, Focus::PageRename { idx, .. } if *idx == i);
                let name = if renaming {
                    if let Focus::PageRename { buffer, .. } = &self.focus {
                        format!("{}_", if buffer.is_empty() { page_ref.id.as_str() } else { buffer.as_str() })
                    } else { page_ref.id.clone() }
                } else { page_ref.id.chars().take(12).collect::<String>() };
                let nw = ui_measure(&name, 8.0);
                label(&mut ui, &name, x + (cell_w - nw) / 2.0, ty + cell_h + 6.0, 8.0,
                    if renaming { PALETTE[4] } else if i == self.page_idx { Color::WHITE } else { C_DIM });
                x += cell_w + 12.0;
                if x + cell_w > tr.x1 - 90.0 { break; }
            }
            // + New Page cell
            let cell = Rect::new(x, ty, x + cell_w, ty + cell_h);
            stroke_rect(&mut ui, cell, C_PANEL_EDGE, 1.0);
            let pw = ui_measure("+ New Page", 8.0);
            label(&mut ui, "+", x + cell_w / 2.0 - 4.0, ty + cell_h / 2.0 - 8.0, 13.0, C_DIM);
            label(&mut ui, "+ New Page", x + (cell_w - pw) / 2.0, ty + cell_h + 6.0, 8.0, C_DIM);
        }
        {
            let sy = self.win_h - STATUS_H;
            fill_rect(&mut ui, Rect::new(0.0, sy, self.win_w, self.win_h), C_PANEL);
            fill_rect(&mut ui, Rect::new(0.0, sy, self.win_w, sy + 1.0), C_PANEL_EDGE);
            // green ready dot
            ui.fill(Fill::NonZero, Affine::IDENTITY, PALETTE[2], None,
                &vello::kurbo::Circle::new((15.0, sy + STATUS_H / 2.0), 3.5));
            label(&mut ui, &status_line, 28.0, sy + 8.0, 9.5,
                if self.focus == Focus::None { C_DIM } else { PALETTE[4] });
            // right side: selection geometry + zoom (mockup)
            if let Some(n) = self.selected_single() {
                let info = format!("X: {:.0}   Y: {:.0}   W: {:.0}   H: {:.0}   {}%",
                    n.transform.x, n.transform.y, n.w, n.h, (self.zoom * 100.0).round());
                let iw = ui_measure(&info, 9.5);
                label(&mut ui, &info, self.win_w - iw - 20.0, sy + 8.0, 9.5, C_DIM);
            } else {
                let info = format!("{}%", (self.zoom * 100.0).round());
                let iw = ui_measure(&info, 9.5);
                label(&mut ui, &info, self.win_w - iw - 20.0, sy + 8.0, 9.5, C_DIM);
            }
        }

        // ---------- left panel (mockup: icon tabs + search + tree) ----------
        fill_rect(&mut ui, Rect::new(0.0, TOP_H, LAYERS_W, self.win_h), C_PANEL);
        fill_rect(&mut ui, Rect::new(LAYERS_W - 1.0, TOP_H, LAYERS_W, self.win_h), C_PANEL_EDGE);
        // icon tab row (mockup: icon ABOVE label, 4 equal columns),
        // geometry shared with click_left_sidebar via left_tab_rects
        for (i, r) in self.left_tab_rects() {
            let active = self.left_tab == i as u8;
            let c = if active { C_ACCENT } else { C_DIM };
            if active {
                let hl = Rect::new(r.x0 + 6.0, r.y0 + 5.0, r.x1 - 6.0, r.y1 - 6.0);
                fill_rrect(&mut ui, hl, 9.0, Color::rgba8(0x7c, 0x5c, 0xfc, 32));
            } else if r.contains(self.cursor) {
                let hl = Rect::new(r.x0 + 6.0, r.y0 + 5.0, r.x1 - 6.0, r.y1 - 6.0);
                fill_rrect(&mut ui, hl, 9.0, Color::rgba8(0xff, 0xff, 0xff, 10));
            }
            let cx = (r.x0 + r.x1) / 2.0;
            let iy = TOP_H + 20.0;
            let st = vello::kurbo::Stroke::new(1.5)
                .with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round);
            match i {
                0 => { // Layers: stacked diamonds
                    for (k, dy) in [(0i32, 0.0f64), (1, 5.5)] {
                        let _ = k;
                        let mut d = vello::kurbo::BezPath::new();
                        d.move_to((cx, iy - 6.5 + dy)); d.line_to((cx + 7.5, iy - 2.0 + dy));
                        d.line_to((cx, iy + 3.5 + dy)); d.line_to((cx - 7.5, iy - 2.0 + dy));
                        d.close_path();
                        ui.stroke(&st, Affine::IDENTITY, c, None, &d);
                    }
                }
                1 => { // Assets: picture frame w/ dot
                    let fr = Rect::new(cx - 7.5, iy - 6.5, cx + 7.5, iy + 6.5);
                    ui.stroke(&st, Affine::IDENTITY, c, None, &fr.to_path(0.1));
                    ui.fill(Fill::NonZero, Affine::IDENTITY, c, None,
                        &vello::kurbo::Circle::new((cx - 3.5, iy - 2.5), 1.8));
                    let mut m = vello::kurbo::BezPath::new();
                    m.move_to((cx - 6.5, iy + 5.5)); m.line_to((cx - 1.5, iy - 1.5)); m.line_to((cx + 6.5, iy + 5.5));
                    ui.stroke(&st, Affine::IDENTITY, c, None, &m);
                }
                2 => { // Components: four diamonds
                    for (dx, dy) in [(-3.8f64, -3.8f64), (3.8, -3.8), (-3.8, 3.8), (3.8, 3.8)] {
                        let mut d = vello::kurbo::BezPath::new();
                        d.move_to((cx + dx, iy + dy - 3.2)); d.line_to((cx + dx + 3.2, iy + dy));
                        d.line_to((cx + dx, iy + dy + 3.2)); d.line_to((cx + dx - 3.2, iy + dy));
                        d.close_path();
                        ui.stroke(&st, Affine::IDENTITY, c, None, &d);
                    }
                }
                _ => { // Library: book
                    let fr = Rect::new(cx - 6.5, iy - 6.5, cx + 6.5, iy + 6.5);
                    ui.stroke(&st, Affine::IDENTITY, c, None, &fr.to_path(0.1));
                    ui.stroke(&st, Affine::IDENTITY, c, None,
                        &vello::kurbo::Line::new((cx - 2.5, iy - 6.5), (cx - 2.5, iy + 6.5)));
                }
            }
            let tw = ui_measure(LEFT_TABS[i], 8.5);
            label(&mut ui, LEFT_TABS[i], cx - tw / 2.0, TOP_H + 38.0, 8.5, c);
            if active {
                fill_rect(&mut ui, Rect::new(cx - tw / 2.0 - 2.5, TOP_H + LTAB_H - 5.0, cx + tw / 2.0 + 2.5, TOP_H + LTAB_H - 2.5), C_ACCENT);
            }
        }
        // ---- non-Layers tabs paint their own panel content and skip the
        // layers tree entirely (shared geometry via left_panel_layout) ----
        if self.left_tab != 0 {
            match self.left_tab {
                1 => {
                    label(&mut ui, &format!("Document assets ({})", self.store.len()), 12.0, TOP_H + LTAB_H + 14.0, 8.0, C_DIM);
                    if self.store.is_empty() {
                        label(&mut ui, "NO ASSETS YET", 12.0, TOP_H + LTAB_H + 36.0, 8.0, C_DIM);
                        label(&mut ui, "IMPORT SKETCH / SVG / PNG  (⌘I)", 12.0, TOP_H + LTAB_H + 50.0, 7.5, C_DIM);
                    }
                }
                2 => {
                    label(&mut ui, "Components — click to stamp", 12.0, TOP_H + LTAB_H + 14.0, 8.0, C_DIM);
                    if self.editor.component_names().is_empty() && self.library_deps.is_empty() {
                        label(&mut ui, "NONE YET — ⌥⌘K FROM SELECTION", 12.0, TOP_H + LTAB_H + 36.0, 7.5, C_DIM);
                    }
                }
                3 => {
                    label(&mut ui, "Linked libraries", 12.0, TOP_H + LTAB_H + 14.0, 8.0, C_DIM);
                    if self.library_deps.is_empty() {
                        label(&mut ui, "NO LIBRARIES LINKED", 12.0, TOP_H + LTAB_H + 36.0, 8.0, C_DIM);
                    }
                }
                _ => {}
            }
            for (tag, r, kind) in self.left_panel_layout() {
                match kind {
                    1 => {
                        // asset tile with a real decoded thumbnail
                        let selected = self.asset_sel.as_deref() == Some(tag.as_str());
                        fill_rrect(&mut ui, r, 4.0, Color::rgb8(0x1a, 0x1c, 0x20));
                        if self.assets.get(&tag).is_none() {
                            if let Some(rec) = self.store.get(&tag) {
                                if rec.mime == "image/png" {
                                    let _ = self.assets.load_png_bytes(&tag.clone(), &rec.bytes.clone());
                                }
                            }
                        }
                        if let Some(img) = self.assets.get(&tag) {
                            let (iw, ih) = (img.width as f64, img.height as f64);
                            let s = (r.width() / iw).min(r.height() / ih).min(4.0);
                            let (ox, oy) = (r.x0 + (r.width() - iw * s) / 2.0, r.y0 + (r.height() - ih * s) / 2.0);
                            ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY, &r.to_path(0.1));
                            ui.draw_image(img, Affine::translate((ox, oy)) * Affine::scale(s));
                            ui.pop_layer();
                        } else {
                            label(&mut ui, "NO PREVIEW", r.x0 + 10.0, r.y0 + r.height() / 2.0 - 4.0, 7.0, C_DIM);
                        }
                        stroke_rect(&mut ui, r, if selected { C_ACCENT } else { C_PANEL_EDGE }, if selected { 2.0 } else { 1.0 });
                        if let Some(rec) = self.store.get(&tag) {
                            label(&mut ui, &rec.name.chars().take(14).collect::<String>(), r.x0 + 2.0, r.y1 + 4.0, 7.0,
                                if selected { C_TEXT } else { C_DIM });
                        }
                    }
                    2 | 3 => {
                        let name = tag.split_once('|').map(|(_, c)| c.to_string()).unwrap_or_else(|| tag.clone());
                        let stamping_this = kind == 2 && self.stamping.as_deref() == Some(tag.as_str());
                        if stamping_this { fill_rrect(&mut ui, r, 4.0, Color::rgba8(0x7c, 0x5c, 0xfc, 70)); }
                        // X-Native component diamond
                        let d = 4.0;
                        let (cx, cy) = (r.x0 + 10.0, (r.y0 + r.y1) / 2.0);
                        let mut dia = vello::kurbo::BezPath::new();
                        dia.move_to((cx, cy - d)); dia.line_to((cx + d, cy));
                        dia.line_to((cx, cy + d)); dia.line_to((cx - d, cy));
                        dia.close_path();
                        ui.fill(Fill::NonZero, Affine::IDENTITY, PALETTE[3], None, &dia);
                        label(&mut ui, &name.chars().take(22).collect::<String>(), r.x0 + 22.0, r.y0 + 3.0, 8.5,
                            if stamping_this { Color::WHITE } else { C_TEXT });
                    }
                    4 => {
                        stroke_rect(&mut ui, r, C_ACCENT, 1.0);
                        label(&mut ui, &tag, r.x0 + 12.0, r.y0 + 6.0, 8.0, C_ACCENT);
                    }
                    _ => { label(&mut ui, &tag, r.x0 + 4.0, r.y0 + 1.0, 8.0, C_DIM); }
                }
            }
        } else {
        // search box (mockup style, uses the layer filter)
        {
            let sr = Rect::new(10.0, TOP_H + LSEARCH_Y0, LAYERS_W - 10.0, TOP_H + LSEARCH_Y1);
            fill_rrect(&mut ui, sr, 8.0, C_FIELD);
            let active = self.focus == Focus::LayerSearch;
            if active { stroke_rect(&mut ui, sr, C_ACCENT, 1.0); }
            // magnifier glyph (mockup)
            {
                let (mx, my) = (sr.x0 + 14.0, (sr.y0 + sr.y1) / 2.0 - 1.0);
                let st = vello::kurbo::Stroke::new(1.2).with_caps(vello::kurbo::Cap::Round);
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None, &vello::kurbo::Circle::new((mx, my), 3.5));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((mx + 2.6, my + 2.6), (mx + 5.6, my + 5.6)));
            }
            let shown = if self.layer_filter.is_empty() && !active { "Search layers".into() }
                else { format!("{}{}", self.layer_filter, if active { "_" } else { "" }) };
            label(&mut ui, &shown, sr.x0 + 26.0, sr.y0 + 6.0, 8.5,
                if self.layer_filter.is_empty() && !active { C_DIM } else { C_TEXT });
        }
        // PAGES section (compact; the thumbnail strip is the main page UI)
        label(&mut ui, "Pages", 12.0, TOP_H + LPAGES_HDR + 6.0, 9.0, C_DIM);
        label(&mut ui, "+", LAYERS_W - 22.0, TOP_H + LPAGES_HDR + 4.0, 11.0, C_DIM);
        let pages_y0 = TOP_H + LPAGES_Y0 + 6.0;
        for (i, pg) in self.pages.iter().enumerate() {
            let y = pages_y0 + i as f64 * ROW_H;
            let row_r = Rect::new(4.0, y - 1.0, LAYERS_W - 8.0, y + ROW_H - 3.0);
            if i == self.page_idx {
                fill_rrect(&mut ui, row_r, 5.0, C_SELECTED);
            } else if row_r.contains(self.cursor) {
                fill_rrect(&mut ui, row_r, 5.0, C_HOVERBG);
            }
            // IR-powered page thumbnail chip (thumbnail_scene sink)
            {
                let live = if i == self.page_idx { &self.editor.root } else { pg };
                let tree = arco_native::build_render_tree(live, &self.vars);
                let (thumb, _) = arco_native::thumbnail_scene(&tree, live.w.max(1.0), live.h.max(1.0), 26.0, 14.0);
                fill_rect(&mut ui, Rect::new(18.0, y, 44.0, y + 14.0), Color::rgb8(0x1a, 0x1c, 0x20));
                ui.append(&thumb, Some(Affine::translate((18.0, y))));
                stroke_rect(&mut ui, Rect::new(18.0, y, 44.0, y + 14.0), C_PANEL_EDGE, 1.0);
            }
            label(&mut ui, &pg.id, 50.0, y, 9.0, if i == self.page_idx { Color::WHITE } else { C_TEXT });
        }
        let plus_y = pages_y0 + self.pages.len() as f64 * ROW_H;
        label(&mut ui, "+ New Page", 20.0, plus_y, 8.0, C_DIM);
        let layers_header_y = plus_y + ROW_H + 6.0;
        // LAYERS header (search lives at the top of the panel now)
        label(&mut ui, "Layers", 12.0, layers_header_y, 9.0, C_DIM);
        if !self.layer_filter.is_empty() {
            label(&mut ui, &format!("filtered: {}", self.layer_filter), 70.0, layers_header_y, 8.0, C_ACCENT);
        }
        let layers_list_y = layers_header_y + 20.0;
        // LAYER VIRTUALIZATION: only the visible window of rows is cloned
        // and painted — 100k-node documents render ~40 rows, not 100k.
        self.layers_scroll = self.layers_scroll.min(self.layer_rows.len().saturating_sub(1));
        let visible_rows = (((self.win_h - layers_list_y) / ROW_H).ceil() as usize).min(200) + 1;
        let win_start = self.layers_scroll;
        let rows: Vec<(String, usize, &'static str)> = self.layer_rows.iter()
            .skip(win_start).take(visible_rows).cloned().collect();
        if win_start > 0 {
            label(&mut ui, &format!("... {} MORE ABOVE", win_start), 12.0, layers_list_y - 14.0, 7.0, C_DIM);
        }
        let below = self.layer_rows.len().saturating_sub(win_start + rows.len());
        if below > 0 {
            label(&mut ui, &format!("... {} MORE BELOW", below), 12.0, self.win_h - 12.0, 7.0, C_DIM);
        }
        // proportional scrollbar (polish: scroll affordance like the mockup)
        if self.layer_rows.len() > rows.len() {
            let track_y0 = layers_list_y;
            let track_y1 = self.win_h - 16.0;
            let track_h = (track_y1 - track_y0).max(24.0);
            let frac = rows.len() as f64 / self.layer_rows.len() as f64;
            let thumb_h = (track_h * frac).max(24.0);
            let pos = win_start as f64 / (self.layer_rows.len() - rows.len()).max(1) as f64;
            let ty = track_y0 + (track_h - thumb_h) * pos;
            fill_rrect(&mut ui, Rect::new(LAYERS_W - 7.0, track_y0, LAYERS_W - 3.0, track_y1), 2.0, Color::rgba8(0xff, 0xff, 0xff, 10));
            fill_rrect(&mut ui, Rect::new(LAYERS_W - 7.0, ty, LAYERS_W - 3.0, ty + thumb_h), 2.0, Color::rgba8(0xff, 0xff, 0xff, 46));
        }
        for (vi, (id, depth, klabel)) in rows.iter().enumerate() {
            let i = win_start + vi;
            let y = layers_list_y + (i - self.layers_scroll) as f64 * ROW_H;
            if y > self.win_h - ROW_H { break; }
            let selected = self.editor.selection.contains(id);
            let row_r = Rect::new(4.0, y - 1.0, LAYERS_W - 8.0, y + ROW_H - 3.0);
            let row_hover = row_r.contains(self.cursor);
            if selected { fill_rrect(&mut ui, row_r, 5.0, C_SELECTED); }
            else if row_hover { fill_rrect(&mut ui, row_r, 5.0, C_HOVERBG); }
            let node_ref = find(&self.editor.root, id);
            let x = 10.0 + *depth as f64 * 14.0;
            let _ = klabel;
            // mockup: caret for containers, then a TYPE GLYPH per kind
            if let Some(n) = node_ref {
                let icon_c = if selected { Color::WHITE } else { C_DIM };
                if !n.children.is_empty() {
                    // expand caret (all rows expanded — tree is flat-walked)
                    let mut car = vello::kurbo::BezPath::new();
                    car.move_to((x - 7.0, y + 3.0)); car.line_to((x - 1.0, y + 3.0)); car.line_to((x - 4.0, y + 8.0));
                    car.close_path();
                    ui.fill(Fill::NonZero, Affine::IDENTITY, C_DIM, None, &car);
                }
                let (gx, gy) = (x + 6.0, y + 5.0);
                let st = vello::kurbo::Stroke::new(1.2)
                    .with_caps(vello::kurbo::Cap::Round).with_join(vello::kurbo::Join::Round);
                use arco_native::NodeKind::*;
                match &n.kind {
                    Text { .. } => { label(&mut ui, "T", gx - 3.0, y - 1.0, 9.0, icon_c); }
                    Frame { .. } | Group => {
                        ui.stroke(&st, Affine::IDENTITY, icon_c, None,
                            &vello::kurbo::Rect::new(gx - 4.5, gy - 4.5, gx + 4.5, gy + 4.5).to_path(0.1));
                    }
                    Rect { .. } => {
                        ui.stroke(&st, Affine::IDENTITY, icon_c, None,
                            &vello::kurbo::RoundedRect::new(gx - 4.5, gy - 4.5, gx + 4.5, gy + 4.5, 2.0).to_path(0.1));
                    }
                    Ellipse => {
                        ui.stroke(&st, Affine::IDENTITY, icon_c, None,
                            &vello::kurbo::Circle::new((gx, gy), 4.5));
                    }
                    Line => {
                        ui.stroke(&st, Affine::IDENTITY, icon_c, None,
                            &vello::kurbo::Line::new((gx - 4.5, gy + 4.0), (gx + 4.5, gy - 4.0)));
                    }
                    Image { .. } => {
                        ui.stroke(&st, Affine::IDENTITY, icon_c, None,
                            &vello::kurbo::Rect::new(gx - 5.0, gy - 4.0, gx + 5.0, gy + 4.0).to_path(0.1));
                        let mut m = vello::kurbo::BezPath::new();
                        m.move_to((gx - 4.0, gy + 3.0)); m.line_to((gx - 1.0, gy - 1.0)); m.line_to((gx + 4.0, gy + 3.0));
                        ui.stroke(&st, Affine::IDENTITY, icon_c, None, &m);
                    }
                    Vector { .. } => { label(&mut ui, "~", gx - 4.0, y - 1.0, 9.0, icon_c); }
                    Component { .. } | Instance { .. } => {
                        let d = 4.5;
                        let mut dia = vello::kurbo::BezPath::new();
                        dia.move_to((gx, gy - d)); dia.line_to((gx + d, gy));
                        dia.line_to((gx, gy + d)); dia.line_to((gx - d, gy));
                        dia.close_path();
                        ui.stroke(&st, Affine::IDENTITY, PALETTE[3], None, &dia);
                    }
                }
            }
            let name = if id.len() > 18 { &id[..18] } else { id };
            label(&mut ui, name, x + 18.0, y, 9.0, if selected { Color::WHITE } else { C_TEXT });
            // eye + lock affordances at the row's right (hover or engaged)
            if let Some(n) = node_ref {
                let eye_x = LAYERS_W - 40.0;
                let lock_x = LAYERS_W - 22.0;
                if !n.visible { label(&mut ui, "-", eye_x + 3.0, y, 9.0, C_DIM); }
                else if row_hover { label(&mut ui, "O", eye_x + 2.0, y, 8.0, C_DIM); }
                if n.locked { label(&mut ui, "*", lock_x + 3.0, y, 9.0, PALETTE[4]); }
                else if row_hover { label(&mut ui, "*", lock_x + 3.0, y, 8.0, Color::rgba8(0x8f, 0x93, 0x9b, 120)); }
            }
        }

        // ASSETS section at the bottom of the layers panel
        let comps = self.editor.component_names();
        if !comps.is_empty() {
            let assets_y = self.win_h - 30.0 - comps.len() as f64 * ROW_H;
            fill_rect(&mut ui, Rect::new(0.0, assets_y - 22.0, LAYERS_W, assets_y - 21.0), C_PANEL_EDGE);
            label(&mut ui, "ASSETS", 12.0, assets_y - 16.0, 11.0, C_DIM);
            for (i, name) in comps.iter().enumerate() {
                let y = assets_y + i as f64 * ROW_H;
                let stamping_this = self.stamping.as_deref() == Some(name.as_str());
                let asset_r = Rect::new(4.0, y - 1.0, LAYERS_W - 8.0, y + ROW_H - 3.0);
                if stamping_this {
                    fill_rrect(&mut ui, asset_r, 5.0, C_SELECTED);
                }
                // diamond marker, X-Native style
                let d = 5.0;
                let (cx, cy) = (16.0, y + 5.0);
                let mut diamond = vello::kurbo::BezPath::new();
                diamond.move_to((cx, cy - d));
                diamond.line_to((cx + d, cy));
                diamond.line_to((cx, cy + d));
                diamond.line_to((cx - d, cy));
                diamond.close_path();
                ui.fill(Fill::NonZero, Affine::IDENTITY, PALETTE[3], None, &diamond);
                label(&mut ui, name, 30.0, y, 9.0, if stamping_this { Color::WHITE } else { C_TEXT });
            }
        }
        } // end Layers tab (left_tab == 0)

        // inspector
        let ix = self.win_w - INSPECTOR_W;
        fill_rect(&mut ui, Rect::new(ix, TOP_H, self.win_w, self.win_h), C_PANEL);
        fill_rect(&mut ui, Rect::new(ix, TOP_H, ix + 1.0, self.win_h), C_PANEL_EDGE);
        // Design | Prototype tabs (X-Native properties panel)
        for (name, idx, r) in self.inspector_tabs() {
            let active = self.inspector_tab == idx;
            if active {
                let hl = Rect::new(r.x0 + 2.0, TOP_H + 6.0, r.x1 - 2.0, TOP_H + 24.0);
                fill_rrect(&mut ui, hl, 6.0, Color::rgba8(0x7c, 0x5c, 0xfc, 24));
            }
            label(&mut ui, name, r.x0 + 6.0, TOP_H + 11.0, 8.5, if active { Color::WHITE } else { C_DIM });
            if active {
                let tw = ui_measure(name, 8.5);
                fill_rect(&mut ui, Rect::new(r.x0 + 6.0, TOP_H + 24.0, r.x0 + 6.0 + tw, TOP_H + 26.0), C_ACCENT);
            }
        }
        // Vars/Libs indicators when active via menu / left rail
        if self.inspector_tab == 2 { label(&mut ui, "VARIABLES", self.win_w - 70.0, TOP_H + 11.0, 7.5, C_ACCENT); }
        if self.inspector_tab == 3 { label(&mut ui, "LIBRARIES", self.win_w - 70.0, TOP_H + 11.0, 7.5, C_ACCENT); }
        fill_rect(&mut ui, Rect::new(ix, TOP_H + 28.0, self.win_w, TOP_H + 29.0), C_PANEL_EDGE);
        if self.inspector_tab == 1 {
            // Prototype tab with nothing selected
            if self.selected_single().is_none() {
                label(&mut ui, "SELECT A LAYER TO LINK", ix + 12.0, TOP_H + 44.0, 8.5, C_DIM);
            }
        }
        // ---- Export section (mockup, Design tab): REAL export buttons,
        // geometry shared with click_inspector via export_layout() ----
        if self.inspector_tab == 0 {
            let ey = self.win_h - THUMBS_H - STATUS_H - 60.0;
            fill_rect(&mut ui, Rect::new(ix + 8.0, ey - 2.0, self.win_w - 8.0, ey - 1.0), C_PANEL_EDGE);
            label(&mut ui, "Export", ix + 12.0, ey + 4.0, 10.0, C_SECTION);
            for (l, _tag, r) in self.export_layout() {
                let hover = r.contains(self.cursor);
                if hover { fill_rrect(&mut ui, r, 5.0, C_ACCENT); }
                else { fill_rrect(&mut ui, r, 5.0, C_FIELD); }
                let tw = ui_measure(l, 8.5);
                label(&mut ui, l, r.x0 + (r.width() - tw) / 2.0, r.y0 + 7.0, 8.5,
                    if hover { Color::WHITE } else { C_TEXT });
            }
        }
        if self.inspector_tab == 2 {
            // ---- VARIABLES tab: collections + modes + bind-to-selection ----
            let mut y = TOP_H + 40.0;
            // mode switcher row
            label(&mut ui, "MODE:", ix + 12.0, y, 9.0, C_DIM);
            let mut mx = ix + 56.0;
            let modes = {
                let mut v = vec!["default".to_string()];
                v.extend(self.vars.mode_names());
                v
            };
            for m in &modes {
                let active = match (&self.vars.active_mode, m.as_str()) {
                    (None, "default") => true,
                    (Some(am), name) => am == name,
                    _ => false,
                };
                let w = arco_native::text::measure(m, 8.0) + 12.0;
                let r = Rect::new(mx, y - 3.0, mx + w, y + 13.0);
                if active { fill_rrect(&mut ui, r, 4.0, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                label(&mut ui, m, mx + 6.0, y, 8.0, if active { Color::WHITE } else { C_TEXT });
                mx += w + 6.0;
            }
            y += 26.0;
            // catalog grouped by collection
            let cat = self.vars.catalog();
            let mut last_col = String::new();
            for (collection, name, kind) in cat.iter().take(24) {
                if *collection != last_col {
                    label(&mut ui, collection, ix + 12.0, y, 9.0, C_DIM);
                    y += 16.0;
                    last_col = collection.clone();
                }
                // swatch/value + name + bind hints
                match *kind {
                    "color" => {
                        let c = self.vars.color(name, Color::BLACK);
                        fill_rrect(&mut ui, Rect::new(ix + 14.0, y, ix + 26.0, y + 12.0), 3.0, c);
                    }
                    "number" => {
                        let v = self.vars.number(name, 0.0);
                        label(&mut ui, &format!("{v:.0}"), ix + 14.0, y + 1.0, 8.0, C_TEXT);
                    }
                    "string" => { label(&mut ui, "S", ix + 16.0, y + 1.0, 8.0, C_TEXT); }
                    _ => { label(&mut ui, "B", ix + 16.0, y + 1.0, 8.0, C_TEXT); }
                }
                label(&mut ui, name, ix + 34.0, y + 1.0, 8.5, C_TEXT);
                // bind buttons when something is selected
                if self.selected_single().is_some() {
                    match *kind {
                        "color" => { label(&mut ui, "Fill", self.win_w - 44.0, y + 1.0, 7.5, C_ACCENT); }
                        "number" => {
                            label(&mut ui, "RAD", self.win_w - 76.0, y + 1.0, 7.5, C_ACCENT);
                            label(&mut ui, "OPA", self.win_w - 44.0, y + 1.0, 7.5, C_ACCENT);
                        }
                        _ => {}
                    }
                }
                y += 18.0;
                if y > self.win_h - 40.0 { break; }
            }
            if cat.is_empty() {
                label(&mut ui, "NO VARIABLES YET", ix + 12.0, y, 8.5, C_DIM);
                label(&mut ui, "DEMO SEEDS SOME ON STARTUP", ix + 12.0, y + 14.0, 7.5, C_DIM);
            }
        }
        // ---- LIBRARIES tab: strict client of diff_library/accept_update,
        // geometry SHARED with click_inspector via libs_layout() ----
        if self.inspector_tab == 3 {
            if self.library_deps.is_empty() {
                label(&mut ui, "Libraries", ix + 12.0, TOP_H + 36.0, 10.0, C_DIM);
                let r1 = Rect::new(ix + 84.0, TOP_H + 33.0, ix + 146.0, TOP_H + 47.0);
                let r2 = Rect::new(ix + 150.0, TOP_H + 33.0, ix + 212.0, TOP_H + 47.0);
                stroke_rect(&mut ui, r1, C_PANEL_EDGE, 1.0);
                label(&mut ui, "LINK .XLIB", r1.x0 + 4.0, r1.y0 + 2.0, 7.0, C_TEXT);
                stroke_rect(&mut ui, r2, C_PANEL_EDGE, 1.0);
                label(&mut ui, "CHECK UPD", r2.x0 + 4.0, r2.y0 + 2.0, 7.0, C_TEXT);
                label(&mut ui, "NO LIBRARIES LINKED", ix + 12.0, TOP_H + 64.0, 8.5, C_DIM);
                label(&mut ui, "PUT library.xlib NEXT TO THE APP,", ix + 12.0, TOP_H + 78.0, 7.5, C_DIM);
                label(&mut ui, "THEN CLICK LINK .XLIB", ix + 12.0, TOP_H + 90.0, 7.5, C_DIM);
            }
            for (tag, r, kind) in self.libs_layout() {
                match kind {
                    0 => {
                        if tag == "LIBRARIES" { label(&mut ui, &tag, r.x0, r.y0 + 2.0, 10.0, C_DIM); }
                        else {
                            // library header card: name + version + integrity badge
                            label(&mut ui, &tag, r.x0, r.y0 + 2.0, 10.0, C_TEXT);
                            if let Some(dep) = self.library_deps.iter().find(|d| self.library_snapshots.get(&d.library_id).map(|l| l.name == tag).unwrap_or(false)) {
                                let ok = self.library_integrity.iter().find(|(id, _)| *id == dep.library_id)
                                    .map(|(_, s)| s.starts_with("Verified")).unwrap_or(true);
                                let badge = format!("v{} • LINKED{}", dep.resolved_version, if ok { "" } else { " • INTEGRITY!" });
                                label(&mut ui, &badge, r.x0, r.y0 + 15.0, 7.5, if ok { C_DIM } else { PALETTE[1] });
                            }
                        }
                    }
                    1 | 2 => {
                        stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0);
                        label(&mut ui, &tag, r.x0 + 4.0, r.y0 + 2.0, 7.0, C_TEXT);
                    }
                    3 => {
                        fill_rrect(&mut ui, r, 3.0, Color::rgba8(0x7c, 0x5c, 0xfc, 60));
                        stroke_rect(&mut ui, r, C_ACCENT, 1.0);
                        if let Some((_, newer, changes)) = &self.library_update {
                            label(&mut ui, &format!("UPDATE v{} ({} CHANGES) — REVIEW", newer.version, changes.len()),
                                r.x0 + 6.0, r.y0 + 5.0, 7.5, C_TEXT);
                        }
                    }
                    4 => {
                        stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0);
                        let name = tag.split_once('|').map(|(_, c)| c).unwrap_or(&tag);
                        let d = 3.5;
                        let (cx, cy) = (r.x0 + 7.0, r.y0 + 7.0);
                        let mut dia = vello::kurbo::BezPath::new();
                        dia.move_to((cx, cy - d)); dia.line_to((cx + d, cy));
                        dia.line_to((cx, cy + d)); dia.line_to((cx - d, cy));
                        dia.close_path();
                        ui.fill(Fill::NonZero, Affine::IDENTITY, PALETTE[3], None, &dia);
                        label(&mut ui, name, r.x0 + 14.0, r.y0 + 2.0, 8.0, C_TEXT);
                    }
                    _ => { label(&mut ui, &tag, r.x0, r.y0 + 2.0, 8.0, if tag.starts_with("  ") { C_TEXT } else { C_DIM }); }
                }
            }
        }
        // ---- library update REVIEW overlay (Accept/Cancel) ----
        if self.library_review {
            if let Some((idx, newer, changes)) = &self.library_update {
                let dep = &self.library_deps[*idx];
                let pinned = self.library_snapshots.get(&dep.library_id);
                let panel = Rect::new(self.win_w / 2.0 - 220.0, self.win_h / 2.0 - 160.0,
                                      self.win_w / 2.0 + 220.0, self.win_h / 2.0 + 160.0);
                fill_rect(&mut ui, Rect::new(0.0, 0.0, self.win_w, self.win_h), Color::rgba8(0, 0, 0, 130));
                fill_rrect(&mut ui, panel, 12.0, Color::rgba8(0x24, 0x26, 0x2b, 252));
                stroke_rect(&mut ui, panel, C_PANEL_EDGE, 1.0);
                label(&mut ui, "UPDATE AVAILABLE", panel.x0 + 20.0, panel.y0 + 14.0, 12.0, C_TEXT);
                label(&mut ui, &newer.name, panel.x0 + 20.0, panel.y0 + 34.0, 10.0, C_TEXT);
                label(&mut ui, &format!("CURRENT v{}   AVAILABLE v{}", dep.resolved_version, newer.version),
                    panel.x0 + 20.0, panel.y0 + 50.0, 8.5, C_DIM);
                let mut ry = panel.y0 + 72.0;
                for ch in changes.iter().take(9) {
                    use arco_native::LibraryChange::*;
                    let (txt, col) = match ch {
                        StyleModified(n) => {
                            // old -> new color preview for paint styles
                            let old = pinned.and_then(|l| l.styles.get(n));
                            let newv = newer.styles.get(n);
                            if let (Some(arco_native::Style::Paint { fill: of }), Some(arco_native::Style::Paint { fill: nf })) = (old, newv) {
                                let oc = match of { Paint::Solid(c) => *c, _ => Color::WHITE };
                                let nc = match nf { Paint::Solid(c) => *c, _ => Color::WHITE };
                                fill_rect(&mut ui, Rect::new(panel.x0 + 150.0, ry, panel.x0 + 166.0, ry + 10.0), oc);
                                label(&mut ui, "->", panel.x0 + 170.0, ry, 8.0, C_DIM);
                                fill_rect(&mut ui, Rect::new(panel.x0 + 186.0, ry, panel.x0 + 202.0, ry + 10.0), nc);
                            }
                            (format!("~ {n}"), PALETTE[4])
                        }
                        StyleAdded(n) => (format!("+ {n}"), PALETTE[2]),
                        StyleRemoved(n) => (format!("- {n}"), PALETTE[1]),
                        VariableChanged(n) => (format!("~ var {n}"), PALETTE[4]),
                        ComponentAdded(n) => (format!("+ comp {n}"), PALETTE[2]),
                        ComponentRemoved(n) => (format!("- comp {n}"), PALETTE[1]),
                    };
                    label(&mut ui, &txt, panel.x0 + 20.0, ry, 8.5, col);
                    ry += 16.0;
                }
                // Accept / Cancel
                let acc = Rect::new(panel.x0 + 20.0, panel.y1 - 40.0, panel.x0 + 110.0, panel.y1 - 16.0);
                fill_rrect(&mut ui, acc, 4.0, C_ACCENT);
                label(&mut ui, "ACCEPT", acc.x0 + 22.0, acc.y0 + 7.0, 9.0, Color::WHITE);
                let can = Rect::new(panel.x0 + 120.0, panel.y1 - 40.0, panel.x0 + 210.0, panel.y1 - 16.0);
                stroke_rect(&mut ui, can, C_PANEL_EDGE, 1.0);
                label(&mut ui, "CANCEL", can.x0 + 22.0, can.y0 + 7.0, 9.0, C_TEXT);
            }
        }
        if let Some(n) = self.selected_single() {
            if self.inspector_tab == 0 {
            // ---- Position section (mockup): header + X/Y + W/H boxes ----
            label(&mut ui, "Position", ix + 12.0, TOP_H + IY_POS_HDR, 10.0, C_SECTION);
            {
                // selected node name, right-aligned on the header line
                let nm = format!("{} ({})", n.id, kind_label(n));
                let nm = if nm.len() > 22 { format!("{}…", &nm[..21]) } else { nm };
                let w = ui_measure(&nm, 7.5);
                label(&mut ui, &nm, self.win_w - 12.0 - w, TOP_H + IY_POS_HDR + 2.0, 7.5, C_DIM);
            }
            let vals = [n.transform.x, n.transform.y, n.w, n.h];
            let names = ["X", "Y", "W", "H"];
            let rot_deg = n.transform.rotation.to_degrees();
            let opacity = n.opacity;
            for f in 0..4u8 {
                let fy = if f < 2 { TOP_H + IY_XY } else { TOP_H + IY_WH };
                let fx = ix + 12.0 + if f % 2 == 1 { 108.0 } else { 0.0 };
                let r = Rect::new(fx - 2.0, fy - 3.0, fx + 100.0, fy + 14.0);
                let active = matches!(&self.focus, Focus::Field { field, .. } if *field == f);
                let hover = r.contains(self.cursor);
                if active {
                    fill_rrect(&mut ui, r, 6.0, C_FIELD);
                    stroke_rect(&mut ui, r, C_ACCENT, 1.2);
                    if let Focus::Field { buffer, .. } = &self.focus {
                        if buffer.is_empty() {
                            // select-all look: old value shown "selected"
                            // (accent wash); first keystroke replaces it
                            let old_txt = format!("{:.0}", vals[f as usize]);
                            let tw = ui_measure(&old_txt, 9.0);
                            fill_rrect(&mut ui, Rect::new(fx + 24.0, fy - 1.0, fx + 28.0 + tw, fy + 12.0), 3.0, Color::rgba8(0x7c, 0x5c, 0xfc, 90));
                            label(&mut ui, names[f as usize], fx + 8.0, fy + 1.0, 8.5, C_DIM);
                            label(&mut ui, &old_txt, fx + 26.0, fy + 1.0, 9.0, Color::WHITE);
                        } else {
                            label(&mut ui, &format!("{}  {buffer}_", names[f as usize]), fx + 8.0, fy + 1.0, 9.0, Color::WHITE);
                        }
                    }
                } else {
                    fill_rrect(&mut ui, r, 6.0, C_FIELD);
                    if hover { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                    label(&mut ui, names[f as usize], fx + 8.0, fy + 1.0, 8.5, C_DIM);
                    label(&mut ui, &format!("{:.0}", vals[f as usize]), fx + 26.0, fy + 1.0, 9.0, C_TEXT);
                }
            }
            // rotation row (mockup: ∠ 0° box + transform dropdown box)
            {
                let fy = TOP_H + IY_ROT;
                let r1 = Rect::new(ix + 10.0, fy - 3.0, ix + 112.0, fy + 14.0);
                fill_rrect(&mut ui, r1, 6.0, C_FIELD);
                // angle glyph
                let st = vello::kurbo::Stroke::new(1.2);
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((r1.x0 + 8.0, fy + 9.0), (r1.x0 + 16.0, fy + 9.0)));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((r1.x0 + 8.0, fy + 9.0), (r1.x0 + 14.0, fy + 2.0)));
                label(&mut ui, &format!("{:.0}°", rot_deg), r1.x0 + 22.0, fy + 1.0, 9.0, C_TEXT);
                let r2 = Rect::new(ix + 118.0, fy - 3.0, ix + 220.0, fy + 14.0);
                fill_rrect(&mut ui, r2, 6.0, C_FIELD);
                label(&mut ui, kind_label(n), r2.x0 + 8.0, fy + 1.0, 8.5, C_DIM);
                label(&mut ui, "v", r2.x1 - 14.0, fy + 1.0, 8.0, C_DIM);
            }
            // ---- Appearance section (mockup): opacity + corner radius ----
            {
                let hy = TOP_H + IY_APP_HDR;
                draw_section_sep(&mut ui, ix, self.win_w, hy - 8.0);
                label(&mut ui, "Appearance", ix + 12.0, hy, 10.0, C_SECTION);
                let fy = TOP_H + IY_APP_ROW;
                let r1 = Rect::new(ix + 10.0, fy - 3.0, ix + 112.0, fy + 14.0);
                fill_rrect(&mut ui, r1, 6.0, C_FIELD);
                // opacity ring glyph
                ui.stroke(&vello::kurbo::Stroke::new(1.2), Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Circle::new((r1.x0 + 12.0, fy + 6.0), 4.5));
                label(&mut ui, &format!("{:.0}%", opacity * 100.0), r1.x0 + 24.0, fy + 1.0, 9.0, C_TEXT);
                let bm1 = Rect::new(r1.x1 - 38.0, fy - 1.0, r1.x1 - 22.0, fy + 12.0);
                let bp1 = Rect::new(r1.x1 - 20.0, fy - 1.0, r1.x1 - 4.0, fy + 12.0);
                draw_stepper(&mut ui, bm1, false, bm1.contains(self.cursor), C_DIM);
                draw_stepper(&mut ui, bp1, true, bp1.contains(self.cursor), C_DIM);
                let r2 = Rect::new(ix + 118.0, fy - 3.0, ix + 220.0, fy + 14.0);
                fill_rrect(&mut ui, r2, 6.0, C_FIELD);
                // corner glyph
                let mut cg = vello::kurbo::BezPath::new();
                cg.move_to((r2.x0 + 8.0, fy + 11.0));
                cg.line_to((r2.x0 + 8.0, fy + 5.0));
                cg.curve_to((r2.x0 + 8.0, fy + 2.0), (r2.x0 + 11.0, fy + 2.0), (r2.x0 + 14.0, fy + 2.0));
                ui.stroke(&vello::kurbo::Stroke::new(1.2), Affine::IDENTITY, C_DIM, None, &cg);
                let rad = if let arco_native::NodeKind::Rect { radius } = &n.kind { *radius } else { 0.0 };
                label(&mut ui, &format!("{rad:.0}"), r2.x0 + 22.0, fy + 1.0, 9.0, C_TEXT);
                let bm2 = Rect::new(r2.x1 - 38.0, fy - 1.0, r2.x1 - 22.0, fy + 12.0);
                let bp2 = Rect::new(r2.x1 - 20.0, fy - 1.0, r2.x1 - 4.0, fy + 12.0);
                draw_stepper(&mut ui, bm2, false, bm2.contains(self.cursor), C_DIM);
                draw_stepper(&mut ui, bp2, true, bp2.contains(self.cursor), C_DIM);
                // ---- independent corner radii (rects): TL TR BR BL mini
                // boxes; click top half = +2, bottom = -2; uniform stepper
                // above clears the per-corner overrides ----
                if let arco_native::NodeKind::Rect { radius } = &n.kind {
                    let cy2 = TOP_H + IY_CORNERS;
                    let c = n.corner_radii.unwrap_or([*radius; 4]);
                    let names4 = ["TL", "TR", "BR", "BL"];
                    for k in 0..4usize {
                        let bx = ix + 12.0 + k as f64 * 54.0;
                        let r = Rect::new(bx, cy2 - 3.0, bx + 48.0, cy2 + 14.0);
                        fill_rrect(&mut ui, r, 5.0, C_FIELD);
                        if r.contains(self.cursor) { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                        label(&mut ui, names4[k], bx + 5.0, cy2 + 1.5, 6.5, C_DIM);
                        label(&mut ui, &format!("{:.0}", c[k]), bx + 22.0, cy2 + 1.0, 8.5, C_TEXT);
                    }
                    if n.corner_radii.is_some() {
                        label(&mut ui, "mixed", ix + INSPECTOR_W - 42.0, cy2 + 2.0, 7.0, C_ACCENT);
                    }
                }
            }
            // ---- Auto Layout section header (mockup order; body only
            // for frames, painted by the existing auto-layout block) ----
            {
                let hy = TOP_H + IY_AL_HDR;
                draw_section_sep(&mut ui, ix, self.win_w, hy - 8.0);
                label(&mut ui, "Responsive Layout", ix + 12.0, hy, 10.0, C_SECTION);
                if !matches!(n.kind, arco_native::NodeKind::Frame { .. }) {
                    let br = Rect::new(ix + INSPECTOR_W - 28.0, hy - 3.0, ix + INSPECTOR_W - 12.0, hy + 11.0);
                    draw_stepper(&mut ui, br, true, br.contains(self.cursor), C_DIM);
                    label(&mut ui, "Available on frames", ix + 12.0, hy + 18.0, 7.5, C_DIM);
                }
            }
            // alignment row: REAL align icons (bar + object), hover slots
            {
                let ay = TOP_H + IY_ALIGN;
                for i in 0..6usize {
                    let x = ix + 12.0 + i as f64 * 32.0;
                    let r = Rect::new(x, ay - 2.0, x + 28.0, ay + 14.0);
                    if r.contains(self.cursor) { fill_rrect(&mut ui, r, 4.0, C_HOVERBG); }
                    draw_align_icon(&mut ui, i, r, if r.contains(self.cursor) { C_TEXT } else { C_DIM });
                }
                draw_section_sep(&mut ui, ix, self.win_w, ay + 18.0);
            }
            // FONT BROWSER (text nodes, Design tab): search over ALL
            // system families + the full Google Fonts catalog
            if matches!(n.kind, arco_native::NodeKind::Text { .. }) && self.inspector_tab == 0 {
                let fy = TOP_H + IY_FONT;
                draw_section_sep(&mut ui, ix, self.win_w, fy - 8.0);
                label(&mut ui, "Font", ix + 12.0, fy, 10.0, C_SECTION);
                // ---- typography row (X-Native: size, letter spacing,
                // line height as editable steppers) ----
                {
                    let ty2 = fy + 152.0 + 118.0; // painted BELOW the browser block
                    let _ = ty2;
                }
                // Size / LS / LH boxes directly under the header
                {
                    let ry = fy + 16.0;
                    let ls = n.bindings.get("ls").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                    let lh = n.bindings.get("lh").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.2);
                    let vals3 = [format!("{:.0}", n.h), format!("{ls:.1}"), format!("{lh:.2}")];
                    let tags = ["Size", "Sp", "Lh"];
                    for k in 0..3usize {
                        let bx = ix + 12.0 + k as f64 * 74.0;
                        let r = Rect::new(bx, ry - 3.0, bx + 68.0, ry + 14.0);
                        fill_rrect(&mut ui, r, 5.0, C_FIELD);
                        label(&mut ui, tags[k], bx + 5.0, ry + 1.5, 7.0, C_DIM);
                        label(&mut ui, &vals3[k], bx + 26.0, ry + 1.0, 8.5, C_TEXT);
                        // tiny up/down arrows on the right edge
                        label(&mut ui, "^", r.x1 - 12.0, ry - 2.0, 7.0, C_DIM);
                        label(&mut ui, "v", r.x1 - 12.0, ry + 6.0, 7.0, C_DIM);
                    }
                }
                // search box
                {
                    let sr = Rect::new(ix + 12.0, fy + 34.0, self.win_w - 12.0, fy + 50.0);
                    let active = self.focus == Focus::FontSearch;
                    stroke_rect(&mut ui, sr, if active { PALETTE[4] } else { C_PANEL_EDGE }, 1.0);
                    let shown = if self.font_query.is_empty() && !active { "SEARCH 2000+ FONTS".into() }
                        else { format!("{}{}", self.font_query, if active { "_" } else { "" }) };
                    label(&mut ui, &shown, sr.x0 + 4.0, sr.y0 + 4.0, 7.5,
                        if self.font_query.is_empty() && !active { C_DIM } else { C_TEXT });
                }
                // scrollable results
                let current = n.bindings.get("font").cloned().unwrap_or_else(|| "DEFAULT".into());
                let visible = FONT_ROWS;
                let start = self.font_scroll.min(self.font_results.len().saturating_sub(1));
                for (row, (name, _)) in self.font_results.iter().enumerate().skip(start).take(visible) {
                    let y = fy + 58.0 + (row - start) as f64 * 18.0;
                    let active = current.starts_with(name.trim_end_matches(" (G)"));
                    let r = Rect::new(ix + 12.0, y - 2.0, self.win_w - 12.0, y + 12.0);
                    if active { fill_rrect(&mut ui, r, 3.0, C_ACCENT); }
                    else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                    let display = if name.len() > 24 { &name[..24] } else { name };
                    label(&mut ui, display, ix + 18.0, y, 8.0, if active { Color::WHITE } else { C_TEXT });
                }
                if self.font_results.len() > visible {
                    label(&mut ui, &format!("{}/{} scroll", start + visible.min(self.font_results.len() - start), self.font_results.len()), ix + 12.0, fy + 58.0 + visible as f64 * 18.0, 7.0, C_DIM);
                }
                // weight chips for the applied google family
                if !self.font_weights.is_empty() {
                    let wy = fy + 58.0 + visible as f64 * 18.0 + 14.0;
                    label(&mut ui, "WEIGHTS", ix + 12.0, wy, 8.0, C_DIM);
                    let mut wx = ix + 12.0;
                    let mut wrow = wy + 12.0;
                    for (_, w, italic) in &self.font_weights {
                        let text = if *italic { "IT".to_string() } else { format!("{w}") };
                        let cw = arco_native::text::measure(&text, 7.5) + 10.0;
                        if wx + cw > self.win_w - 12.0 { wx = ix + 12.0; wrow += 18.0; }
                        let r = Rect::new(wx, wrow - 2.0, wx + cw, wrow + 12.0);
                        let is_current = current.ends_with(&format!(" {w}")) && !*italic
                            || (*italic && current.ends_with("Italic"));
                        if is_current { fill_rrect(&mut ui, r, 3.0, C_ACCENT); }
                        else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                        label(&mut ui, &text, wx + 5.0, wrow, 7.5, if is_current { Color::WHITE } else { C_TEXT });
                        wx += cw + 4.0;
                    }
                }
            }
            // ---- Fill section (mockup): header + swatch row w/ hex + eye ----
            {
                let hy = TOP_H + IY_FILL_HDR;
                draw_section_sep(&mut ui, ix, self.win_w, hy - 8.0);
                label(&mut ui, "Fill", ix + 12.0, hy, 10.0, C_SECTION);
                let fills = if !n.visual_stacks_materialized { vec![arco_native::PaintLayer::new(n.fill.clone())] } else { n.fill_layers.clone() };
                let fill_idx = self.fill_layer_index.min(fills.len().saturating_sub(1));
                let no_fill = Paint::Solid(Color::TRANSPARENT);
                let top_fill = fills.get(fill_idx).map(|l| &l.paint).unwrap_or(&no_fill);
                let fill_opacity = fills.get(fill_idx).map(|l| l.opacity).unwrap_or(0.0);
                let fill_blend = fills.get(fill_idx).map(|l| l.blend).unwrap_or(BlendKind::Normal);
                label(&mut ui, &format!("{}/{}", if fills.is_empty() { 0 } else { fill_idx + 1 }, fills.len()), ix + 42.0, hy + 1.0, 8.0, C_DIM);
                label(&mut ui, blend_short(fill_blend), ix + 90.0, hy + 1.0, 8.0, C_DIM);
                for (i, t) in ["U", "D", "X"].iter().enumerate() {
                    let r = Rect::new(ix + 108.0 + i as f64 * 20.0, hy - 2.0, ix + 126.0 + i as f64 * 20.0, hy + 12.0);
                    stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); label(&mut ui, t, r.x0 + 5.0, hy, 7.5, C_DIM);
                }
                let grad_on = matches!(top_fill, Paint::LinearGradient { .. } | Paint::RadialGradient { .. });
                let gr = Rect::new(ix + 176.0, hy - 2.0, ix + 204.0, hy + 12.0);
                if grad_on { fill_rrect(&mut ui, gr, 3.0, C_ACCENT); } else { stroke_rect(&mut ui, gr, C_PANEL_EDGE, 1.0); }
                label(&mut ui, "GR", gr.x0 + 6.0, hy, 8.0, if grad_on { Color::WHITE } else { C_TEXT });
                let add = Rect::new(ix + INSPECTOR_W - 28.0, hy - 3.0, ix + INSPECTOR_W - 12.0, hy + 11.0);
                draw_stepper(&mut ui, add, true, add.contains(self.cursor), C_DIM);
                // fill row: color swatch + hex + 100% + eye toggle
                let ry = TOP_H + IY_FILLROW;
                let row = Rect::new(ix + 12.0, ry - 2.0, ix + INSPECTOR_W - 12.0, ry + 15.0);
                fill_rrect(&mut ui, row, 5.0, C_FIELD);
                let (chip, hex) = match top_fill {
                    Paint::Solid(c) => (*c, arco_native::color_to_hex(*c).trim_start_matches('#').to_ascii_uppercase()),
                    Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } =>
                        (stops.first().map(|s| s.1).unwrap_or(C_DIM), "GRADIENT".to_string()),
                    Paint::Variable(v) => (C_ACCENT, format!("VAR {v}").to_ascii_uppercase()),
                };
                let visible_fill = fills.get(fill_idx).map(|l| l.visible).unwrap_or(true) && chip.a > 0;
                fill_rrect(&mut ui, Rect::new(row.x0 + 5.0, ry + 1.0, row.x0 + 17.0, ry + 13.0), 3.0, chip);
                stroke_rect(&mut ui, Rect::new(row.x0 + 5.0, ry + 1.0, row.x0 + 17.0, ry + 13.0), C_PANEL_EDGE, 1.0);
                label(&mut ui, &hex, row.x0 + 24.0, ry + 2.0, 8.5, C_TEXT);
                if let Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } = top_fill {
                    for (i, (_, color)) in stops.iter().take(2).enumerate() {
                        let cx = row.x1 - 92.0 + i as f64 * 16.0;
                        ui.fill(Fill::NonZero, Affine::IDENTITY, *color, None, &vello::kurbo::Circle::new((cx, ry + 6.5), 5.0));
                        ui.stroke(&vello::kurbo::Stroke::new(if self.gradient_stop == i { 2.0 } else { 1.0 }), Affine::IDENTITY, if self.gradient_stop == i { C_ACCENT } else { C_PANEL_EDGE }, None, &vello::kurbo::Circle::new((cx, ry + 6.5), 6.0));
                    }
                }
                label(&mut ui, &format!("{:.0}%", fill_opacity * 100.0), row.x1 - 66.0, ry + 2.0, 8.5, C_DIM);
                // eye toggle (visibility) — REAL eye glyph
                draw_eye(&mut ui, row.x1 - 18.0, ry + 6.5, visible_fill,
                    if visible_fill { C_DIM } else { PALETTE[1] });
                // palette row: 8 swatches in ONE row (mockup compact)
                for (i, color) in PALETTE.iter().enumerate() {
                    let sx = ix + 12.0 + i as f64 * 27.0;
                    let sy = TOP_H + IY_PAL;
                    let r = Rect::new(sx, sy, sx + 16.0, sy + 16.0);
                    fill_rrect(&mut ui, r, 3.0, *color);
                    stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0);
                }
            }
            // ---- Stroke section (mockup): header + swatch/hex/width/INSIDE ----
            {
                let hy = TOP_H + IY_STROKE_HDR;
                draw_section_sep(&mut ui, ix, self.win_w, hy - 8.0);
                label(&mut ui, "Stroke", ix + 12.0, hy, 10.0, C_SECTION);
                let strokes = if !n.visual_stacks_materialized {
                    if n.stroke.width > 0.0 { vec![arco_native::StrokeLayer::new(n.stroke)] } else { vec![] }
                } else { n.stroke_layers.clone() };
                let stroke_idx = self.stroke_layer_index.min(strokes.len().saturating_sub(1));
                let stroke_blend = strokes.get(stroke_idx).map(|l| l.blend).unwrap_or(BlendKind::Normal);
                label(&mut ui, &format!("{}/{}", if strokes.is_empty() { 0 } else { stroke_idx + 1 }, strokes.len()), ix + 52.0, hy + 1.0, 8.0, C_DIM);
                label(&mut ui, blend_short(stroke_blend), ix + 90.0, hy + 1.0, 8.0, C_DIM);
                for (i, t) in ["U", "D", "X"].iter().enumerate() {
                    let r = Rect::new(ix + 108.0 + i as f64 * 20.0, hy - 2.0, ix + 126.0 + i as f64 * 20.0, hy + 12.0);
                    stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); label(&mut ui, t, r.x0 + 5.0, hy, 7.5, C_DIM);
                }
                let add = Rect::new(ix + INSPECTOR_W - 28.0, hy - 3.0, ix + INSPECTOR_W - 12.0, hy + 11.0);
                draw_stepper(&mut ui, add, true, add.contains(self.cursor), C_DIM);
                let ry = TOP_H + IY_STROKEROW;
                let row = Rect::new(ix + 12.0, ry - 2.0, ix + INSPECTOR_W - 12.0, ry + 15.0);
                fill_rrect(&mut ui, row, 5.0, C_FIELD);
                let active_stroke = strokes.get(stroke_idx).map(|l| l.stroke).unwrap_or(n.stroke);
                let sc = active_stroke.color;
                let hex = if sc.a == 0 { "None".to_string() } else { arco_native::color_to_hex(sc).trim_start_matches('#').to_ascii_uppercase() };
                fill_rrect(&mut ui, Rect::new(row.x0 + 5.0, ry + 1.0, row.x0 + 17.0, ry + 13.0), 3.0, if sc.a == 0 { C_PANEL2 } else { sc });
                stroke_rect(&mut ui, Rect::new(row.x0 + 5.0, ry + 1.0, row.x0 + 17.0, ry + 13.0), C_PANEL_EDGE, 1.0);
                label(&mut ui, &hex, row.x0 + 24.0, ry + 2.0, 8.5, C_TEXT);
                // width value | - + steppers | Inside pill (no overlaps)
                let wtxt = format!("{:.0}", active_stroke.width);
                let wtw = ui_measure(&wtxt, 8.5);
                label(&mut ui, &wtxt, row.x1 - 118.0 - wtw, ry + 2.0, 8.5, C_TEXT);
                let bm = Rect::new(row.x1 - 112.0, ry - 1.0, row.x1 - 96.0, ry + 12.0);
                let bp = Rect::new(row.x1 - 94.0, ry - 1.0, row.x1 - 78.0, ry + 12.0);
                draw_stepper(&mut ui, bm, false, bm.contains(self.cursor), C_DIM);
                draw_stepper(&mut ui, bp, true, bp.contains(self.cursor), C_DIM);
                let pill = Rect::new(row.x1 - 68.0, ry - 1.0, row.x1 - 6.0, ry + 13.0);
                fill_rrect(&mut ui, pill, 4.0, C_PANEL2);
                let align = strokes.get(stroke_idx).map(|l| match l.options.align { StrokeAlign::Inside => "Inside", StrokeAlign::Center => "Center", StrokeAlign::Outside => "Outside" }).unwrap_or("Center");
                let itw = ui_measure(align, 7.5);
                label(&mut ui, align, pill.x0 + (pill.width() - itw) / 2.0 - 4.0, ry + 2.5, 7.5, C_DIM);
                label(&mut ui, "v", pill.x1 - 12.0, ry + 3.0, 7.0, C_DIM);
            }
            // ---- Effects section (mockup): rows w/ eye toggles + add ----
            {
                let hy = TOP_H + IY_FX_HDR;
                draw_section_sep(&mut ui, ix, self.win_w, hy - 8.0);
                label(&mut ui, "Effects", ix + 12.0, hy, 10.0, C_SECTION);
                // + adds a drop shadow
                let addr = Rect::new(ix + INSPECTOR_W - 28.0, hy - 3.0, ix + INSPECTOR_W - 12.0, hy + 11.0);
                draw_stepper(&mut ui, addr, true, addr.contains(self.cursor), C_DIM);
                let effects = if !n.visual_stacks_materialized { n.effects.iter().cloned().map(arco_native::EffectLayer::new).collect::<Vec<_>>() } else { n.effect_layers.clone() };
                let effect_idx = self.effect_layer_index.min(effects.len().saturating_sub(1));
                label(&mut ui, &format!("{}/{}", if effects.is_empty() { 0 } else { effect_idx + 1 }, effects.len()), ix + 58.0, hy + 1.0, 8.0, C_DIM);
                if let Some(layer) = effects.get(effect_idx) { label(&mut ui, blend_short(layer.blend), ix + 90.0, hy + 1.0, 8.0, C_DIM); }
                for (i, t) in ["U", "D", "X"].iter().enumerate() {
                    let r = Rect::new(ix + 108.0 + i as f64 * 20.0, hy - 2.0, ix + 126.0 + i as f64 * 20.0, hy + 12.0);
                    stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); label(&mut ui, t, r.x0 + 5.0, hy, 7.5, C_DIM);
                }
                if effects.is_empty() {
                    label(&mut ui, "None — click + to add a drop shadow", ix + 12.0, TOP_H + IY_FXROW + 2.0, 7.5, C_DIM);
                }
                for (i, layer) in effects.iter().take(4).enumerate() {
                    let fx = &layer.effect;
                    let ry = TOP_H + IY_FXROW + i as f64 * 18.0;
                    let row = Rect::new(ix + 12.0, ry - 2.0, ix + INSPECTOR_W - 12.0, ry + 14.0);
                    fill_rrect(&mut ui, row, 5.0, C_FIELD);
                    let name = match fx {
                        Effect::DropShadow { .. } => "Drop Shadow",
                        Effect::InnerShadow { .. } => "Inner Shadow",
                        Effect::LayerBlur { .. } => "Blur",
                        Effect::BackgroundBlur { .. } => "Background Blur",
                    };
                    // small effect glyph (circle) like the mockup
                    ui.stroke(&vello::kurbo::Stroke::new(1.2), Affine::IDENTITY, C_DIM, None,
                        &vello::kurbo::Circle::new((row.x0 + 11.0, ry + 6.0), 4.0));
                    label(&mut ui, name, row.x0 + 22.0, ry + 1.0, 8.5, C_TEXT);
                    let radius = match fx { Effect::DropShadow { blur, .. } | Effect::InnerShadow { blur, .. } => *blur, Effect::LayerBlur { radius } | Effect::BackgroundBlur { radius } => *radius };
                    label(&mut ui, &format!("− {:.0} +", radius), row.x1 - 88.0, ry + 1.0, 8.0, if i == effect_idx { C_ACCENT } else { C_DIM });
                    // per-row eye (removes this effect) — REAL eye glyph
                    draw_eye(&mut ui, row.x1 - 18.0, ry + 6.0, layer.visible, C_DIM);
                }
            }
            } // end Design tab
            // ---- prototype link section (Prototype tab, X-Native style) ----
            if self.inspector_tab == 1 {
                let py = TOP_H + 44.0;
                label(&mut ui, "Prototype", ix + 12.0, py, 10.0, C_DIM);
                let current_dest = n.prototype.as_ref().map(|a| a.destination.clone());
                // one button per OTHER page + NONE
                let mut bx = ix + 12.0;
                let mut by = py + 16.0;
                let none_r = Rect::new(bx, by, bx + 46.0, by + 18.0);
                if current_dest.is_none() { fill_rect(&mut ui, none_r, C_ACCENT); } else { stroke_rect(&mut ui, none_r, C_PANEL_EDGE, 1.0); }
                label(&mut ui, "NONE", bx + 6.0, by + 4.0, 8.0, if current_dest.is_none() { Color::WHITE } else { C_TEXT });
                bx += 52.0;
                for pg in self.pages.iter() {
                    if pg.id == self.editor.root.id { continue; }
                    if bx + 60.0 > self.win_w - 8.0 { bx = ix + 12.0; by += 22.0; }
                    let r = Rect::new(bx, by, bx + 56.0, by + 18.0);
                    let active = current_dest.as_deref() == Some(pg.id.as_str());
                    if active { fill_rect(&mut ui, r, PALETTE[3]); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                    let name = if pg.id.len() > 7 { &pg.id[..7] } else { &pg.id };
                    label(&mut ui, name, bx + 4.0, by + 4.0, 7.5, if active { Color::WHITE } else { C_TEXT });
                    bx += 62.0;
                }
            }
            // ---- component section (instances, Design tab) ----
            if let arco_native::NodeKind::Instance { component } = &n.kind {
                let cy0 = TOP_H + IY_SEC;
                label(&mut ui, "Component", ix + 12.0, cy0, 10.0, C_DIM);
                label(&mut ui, component, ix + 90.0, cy0, 8.5, C_TEXT);
                // variant chips when the component belongs to a Set/Name
                if let Some((set, _)) = component.split_once('/') {
                    let vars_list = arco_native::components::variants_of(&self.editor.root, set);
                    let mut vx = ix + 12.0;
                    let vy = cy0 + 16.0;
                    for vname in vars_list.iter().take(4) {
                        let short = vname.split_once('/').map(|(_, v)| v).unwrap_or(vname);
                        let cw = arco_native::text::measure(short, 7.5) + 10.0;
                        let r = Rect::new(vx, vy - 2.0, vx + cw, vy + 12.0);
                        let active = *vname == component.as_str();
                        if active { fill_rrect(&mut ui, r, 3.0, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                        label(&mut ui, short, vx + 5.0, vy, 7.5, if active { Color::WHITE } else { C_TEXT });
                        vx += cw + 4.0;
                    }
                }
                // detach
                let dr = Rect::new(ix + 150.0, cy0 + 14.0, ix + 208.0, cy0 + 30.0);
                stroke_rect(&mut ui, dr, C_PANEL_EDGE, 1.0);
                label(&mut ui, "DETACH", ix + 156.0, cy0 + 18.0, 7.5, C_TEXT);
            }
            // ---- image controls (image nodes, Design tab) ----
            if let arco_native::NodeKind::Image { asset, fit, placement } = &n.kind {
                if self.inspector_tab == 0 {
                    let iy = TOP_H + IY_SEC;
                    label(&mut ui, "Image", ix + 12.0, iy, 10.0, C_DIM);
                    label(&mut ui, asset, ix + 60.0, iy, 8.5, C_TEXT);
                    // fit-mode chips (X-Native fill/fit/crop/tile)
                    let cur = match fit { arco_native::ImageFit::Fill => 0, arco_native::ImageFit::Fit => 1, arco_native::ImageFit::Crop => 2, arco_native::ImageFit::Tile => 3 };
                    for (i, name) in ["FILL", "FIT", "CROP", "TILE"].iter().enumerate() {
                        let bx = ix + 12.0 + i as f64 * 48.0;
                        let r = Rect::new(bx, iy + 14.0, bx + 44.0, iy + 30.0);
                        if i == cur { fill_rrect(&mut ui, r, 3.0, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                        label(&mut ui, name, bx + 6.0, iy + 17.0, 7.5, if i == cur { Color::WHITE } else { C_TEXT });
                    }
                    // replace: cycle through the loaded asset library
                    let rr = Rect::new(ix + 12.0, iy + 36.0, ix + 96.0, iy + 52.0);
                    stroke_rect(&mut ui, rr, C_PANEL_EDGE, 1.0);
                    label(&mut ui, "REPLACE >", ix + 18.0, iy + 39.0, 7.5, C_TEXT);
                    label(&mut ui, &format!("{} ASSET(S) LOADED", self.assets.len()), ix + 104.0, iy + 39.0, 7.0, C_DIM);
                    // focal point steppers (crop/fill positioning)
                    let py = iy + 58.0;
                    label(&mut ui, &format!("X {:>3.0}%", placement.focal.0 * 100.0), ix + 12.0, py, 8.5, C_TEXT);
                    label(&mut ui, &format!("Y {:>3.0}%", placement.focal.1 * 100.0), ix + 112.0, py, 8.5, C_TEXT);
                    for bx in [ix + 56.0, ix + 74.0, ix + 156.0, ix + 174.0] {
                        stroke_rect(&mut ui, Rect::new(bx, py - 3.0, bx + 15.0, py + 11.0), C_PANEL_EDGE, 1.0);
                    }
                    label(&mut ui, "-", ix + 60.0, py - 1.0, 9.0, C_TEXT);
                    label(&mut ui, "+", ix + 78.0, py - 1.0, 9.0, C_TEXT);
                    label(&mut ui, "-", ix + 160.0, py - 1.0, 9.0, C_TEXT);
                    label(&mut ui, "+", ix + 178.0, py - 1.0, 9.0, C_TEXT);
                    // scale stepper + flips + reset
                    let zy = iy + 78.0;
                    label(&mut ui, &format!("SCALE {:>3.0}%", placement.scale * 100.0), ix + 12.0, zy, 8.5, C_TEXT);
                    for bx in [ix + 84.0, ix + 102.0] {
                        stroke_rect(&mut ui, Rect::new(bx, zy - 3.0, bx + 15.0, zy + 11.0), C_PANEL_EDGE, 1.0);
                    }
                    label(&mut ui, "-", ix + 88.0, zy - 1.0, 9.0, C_TEXT);
                    label(&mut ui, "+", ix + 106.0, zy - 1.0, 9.0, C_TEXT);
                    for (i, (t, on)) in [("FH", placement.flip_h), ("FV", placement.flip_v)].iter().enumerate() {
                        let bx = ix + 124.0 + i as f64 * 26.0;
                        let r = Rect::new(bx, zy - 3.0, bx + 22.0, zy + 11.0);
                        if *on { fill_rrect(&mut ui, r, 3.0, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                        label(&mut ui, t, bx + 4.0, zy - 1.0, 8.0, if *on { Color::WHITE } else { C_TEXT });
                    }
                    let rst = Rect::new(ix + 178.0, zy - 3.0, ix + 214.0, zy + 11.0);
                    stroke_rect(&mut ui, rst, C_PANEL_EDGE, 1.0);
                    label(&mut ui, "RESET", ix + 181.0, zy - 1.0, 7.0, C_TEXT);
                }
            }
            // ---- styles (X-Native reusable paint/text/effect styles);
            // text nodes hand the slot to the font browser ----
            if self.inspector_tab == 0 && !matches!(n.kind, arco_native::NodeKind::Text { .. }) {
                let sy0 = TOP_H + IY_STYLES;
                draw_section_sep(&mut ui, ix, self.win_w, sy0 - 8.0);
                label(&mut ui, "Styles", ix + 12.0, sy0, 10.0, C_SECTION);
                // create-from-selection buttons
                for (i, t) in ["+P", "+T", "+FX"].iter().enumerate() {
                    let bx = ix + 70.0 + i as f64 * 32.0;
                    let r = Rect::new(bx, sy0 - 3.0, bx + 28.0, sy0 + 11.0);
                    stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0);
                    label(&mut ui, t, bx + 5.0, sy0 - 1.0, 8.0, C_TEXT);
                }
                // grouped browser (PAINT / TEXT / EFFECT sections), shared
                // geometry with click_inspector via styles_layout()
                let bound: Vec<String> = n.bindings.iter()
                    .filter(|(k, _)| k.starts_with("style:"))
                    .map(|(_, v)| v.clone()).collect();
                for (name, _kind, r, row_kind) in self.styles_layout() {
                    match row_kind {
                        1 => { label(&mut ui, &name, r.x0, r.y0 + 2.0, 7.5, C_DIM); }
                        2 => {
                            // search box (Focus::StyleSearch)
                            let active = self.focus == Focus::StyleSearch;
                            stroke_rect(&mut ui, r, if active { PALETTE[4] } else { C_PANEL_EDGE }, 1.0);
                            let shown = if self.style_query.is_empty() && !active { "SEARCH STYLES".into() }
                                else { format!("{}{}", self.style_query, if active { "_" } else { "" }) };
                            label(&mut ui, &shown, r.x0 + 4.0, r.y0 + 2.0, 7.0,
                                if self.style_query.is_empty() && !active { C_DIM } else { C_TEXT });
                        }
                        3 => {
                            // management actions for the selected style
                            let danger = name == "DEL";
                            stroke_rect(&mut ui, r, if danger { PALETTE[1] } else { C_PANEL_EDGE }, 1.0);
                            label(&mut ui, &name, r.x0 + 5.0, r.y0 + 2.0, 7.0, C_TEXT);
                        }
                        _ => {
                            let linked = bound.iter().any(|b| *b == name);
                            let selected = self.style_sel.as_deref() == Some(name.as_str());
                            if linked { fill_rrect(&mut ui, r, 3.0, C_ACCENT); }
                            else if selected { fill_rrect(&mut ui, r, 3.0, Color::rgba8(0x4d, 0xb8, 0xff, 70)); stroke_rect(&mut ui, r, PALETTE[4], 1.0); }
                            else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                            if let arco_native::Style::Paint { fill } = &self.styles[name.as_str()] {
                                let c = match fill { Paint::Solid(c) => *c, Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => stops.first().map(|(_, c)| *c).unwrap_or(Color::WHITE), Paint::Variable(_) => C_ACCENT };
                                fill_rect(&mut ui, Rect::new(r.x0 + 3.0, r.y0 + 3.0, r.x0 + 11.0, r.y0 + 11.0), c);
                            }
                            let usage: usize = self.pages.iter().enumerate()
                                .map(|(i, p)| if i == self.page_idx { arco_native::style_usage(&self.editor.root, &name) } else { arco_native::style_usage(p, &name) })
                                .sum();
                            let short = if name.len() > 12 { &name[..12] } else { name.as_str() };
                            let text = if usage > 0 { format!("{short} {usage}") } else { short.to_string() };
                            label(&mut ui, &text, r.x0 + 13.0, r.y0 + 2.0, 7.0, if linked { Color::WHITE } else { C_TEXT });
                        }
                    }
                }
                // rename input overlay
                if let Focus::StyleRename { from, buffer } = &self.focus {
                    label(&mut ui, &format!("RENAME {from} -> {buffer}_ (ENTER)"), ix + 12.0, self.win_h - 60.0, 8.0, PALETTE[4]);
                }
                if self.styles.is_empty() {
                    label(&mut ui, "NONE YET - +P/+T/+FX FROM SELECTION", ix + 12.0, sy0 + 16.0, 7.0, C_DIM);
                }
            }
            // ---- constraints: now lives on the INSPECT tab (mockup's
            // third tab; Design tab matches the mockup's section list) ----
            if self.inspector_tab == 4 {
                let cy = TOP_H + IY_CONSTRAINTS;
                label(&mut ui, &format!("{}  ({})", n.id, kind_label(n)), ix + 12.0, TOP_H + 44.0, 9.0, C_TEXT);
                label(&mut ui, &format!("X {:.0}  Y {:.0}  W {:.0}  H {:.0}  ROT {:.0}°",
                    n.transform.x, n.transform.y, n.w, n.h, n.transform.rotation.to_degrees()),
                    ix + 12.0, TOP_H + 64.0, 8.0, C_DIM);
                let fill_hex = match &n.fill {
                    Paint::Solid(c) => arco_native::color_to_hex(*c),
                    Paint::LinearGradient { .. } => "linear-gradient".into(),
                    Paint::RadialGradient { .. } => "radial-gradient".into(),
                    Paint::Variable(v) => format!("var({v})"),
                };
                label(&mut ui, &format!("FILL {fill_hex}   STROKE {} / {:.0}",
                    arco_native::color_to_hex(n.stroke.color), n.stroke.width),
                    ix + 12.0, TOP_H + 80.0, 8.0, C_DIM);
                label(&mut ui, &format!("OPACITY {:.0}%   EFFECTS {}", n.opacity * 100.0, n.effects.len()),
                    ix + 12.0, TOP_H + 96.0, 8.0, C_DIM);
                label(&mut ui, "Constraints", ix + 12.0, cy, 10.0, C_SECTION);
                let hpins = ["L", "R", "CH", "SH", "SC"];
                let vpins = ["T", "B", "CV", "SV", "SC"];
                let (cur_h, cur_v) = n.pin;
                let hi = match cur_h { arco_native::HPin::Left => 0, arco_native::HPin::Right => 1, arco_native::HPin::CenterH => 2, arco_native::HPin::StretchH => 3, arco_native::HPin::ScaleH => 4 };
                let vi = match cur_v { arco_native::VPin::Top => 0, arco_native::VPin::Bottom => 1, arco_native::VPin::CenterV => 2, arco_native::VPin::StretchV => 3, arco_native::VPin::ScaleV => 4 };
                for (i, lbl) in hpins.iter().enumerate() {
                    let x = ix + 12.0 + i as f64 * 34.0;
                    let r = Rect::new(x, cy + 14.0, x + 30.0, cy + 30.0);
                    if i == hi { fill_rect(&mut ui, r, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                    label(&mut ui, lbl, x + 5.0, cy + 17.0, 7.5, if i == hi { Color::WHITE } else { C_TEXT });
                }
                for (i, lbl) in vpins.iter().enumerate() {
                    let x = ix + 12.0 + i as f64 * 34.0;
                    let r = Rect::new(x, cy + 34.0, x + 30.0, cy + 50.0);
                    if i == vi { fill_rect(&mut ui, r, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                    label(&mut ui, lbl, x + 5.0, cy + 37.0, 7.5, if i == vi { Color::WHITE } else { C_TEXT });
                }
            }
            // ---- auto layout section (frames only, Design tab) ----
            let is_frame = matches!(n.kind, arco_native::NodeKind::Frame { .. });
            if is_frame && self.inspector_tab == 0 {
                let id = n.id.clone();
                let layout = self.editor.auto_layout_of(&id);
                let ly = TOP_H + IY_AL_HDR;
                // NONE / H / V buttons
                let opts = ["NONE", "H", "V"];
                let active = match &layout {
                    None => 0usize,
                    Some(l) if l.direction == LayoutDirection::Horizontal => 1,
                    Some(_) => 2,
                };
                for (i, o) in opts.iter().enumerate() {
                    let bx = ix + 12.0 + i as f64 * 52.0;
                    let r = Rect::new(bx, ly + 16.0, bx + 46.0, ly + 34.0);
                    if i == active { fill_rect(&mut ui, r, C_ACCENT); } else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                    label(&mut ui, o, bx + 8.0, ly + 20.0, 9.0, if i == active { Color::WHITE } else { C_TEXT });
                }
                if let Some(l) = &layout {
                    // GAP and PAD steppers
                    for (row, (name, val)) in [("GAP", l.gap), ("PAD", l.padding)].iter().enumerate() {
                        let ry = ly + 44.0 + row as f64 * 22.0;
                        label(&mut ui, &format!("{name}: {val:.0}"), ix + 12.0, ry, 9.5, C_TEXT);
                        let bm = Rect::new(ix + 140.0, ry - 3.0, ix + 158.0, ry + 12.0);
                        let bp = Rect::new(ix + 162.0, ry - 3.0, ix + 180.0, ry + 12.0);
                        stroke_rect(&mut ui, bm, C_PANEL_EDGE, 1.0);
                        stroke_rect(&mut ui, bp, C_PANEL_EDGE, 1.0);
                        label(&mut ui, "-", ix + 146.0, ry - 1.0, 10.0, C_TEXT);
                        label(&mut ui, "+", ix + 166.0, ry - 1.0, 10.0, C_TEXT);
                    }
                }
            }
        } else if self.tool == Tool::Frame {
            // X-Native: frame presets in the right panel when Frame tool active
            label(&mut ui, "FRAME PRESETS", ix + 12.0, TOP_H + 34.0, 10.0, C_DIM);
            for (i, (name, _, _)) in FRAME_PRESETS.iter().enumerate() {
                let y = TOP_H + 56.0 + i as f64 * 24.0;
                let r = Rect::new(ix + 12.0, y, ix + INSPECTOR_W - 24.0, y + 19.0);
                stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0);
                label(&mut ui, name, ix + 18.0, y + 4.0, 8.5, C_TEXT);
            }
        } else if matches!(self.tool, Tool::Rectangle | Tool::Polygon | Tool::Star) {
            label(&mut ui, "TOOL OPTIONS", ix + 12.0, TOP_H + 40.0, 10.0, C_SECTION);
            let rows: Vec<(&str, String)> = match self.tool {
                Tool::Rectangle => vec![("Corner radius", format!("{:.0}", self.rect_radius))],
                Tool::Polygon => vec![("Sides", self.polygon_sides.to_string())],
                Tool::Star => vec![("Points", self.star_points.to_string()), ("Inner ratio", format!("{:.0}%", self.star_inner_ratio * 100.0))],
                _ => vec![],
            };
            for (i, (name, value)) in rows.iter().enumerate() {
                let y = TOP_H + 64.0 + i as f64 * 30.0;
                label(&mut ui, name, ix + 12.0, y + 4.0, 9.0, C_DIM);
                let field = Rect::new(ix + 132.0, y, ix + 208.0, y + 22.0);
                fill_rrect(&mut ui, field, 4.0, C_FIELD);
                label(&mut ui, value, field.x0 + 8.0, y + 5.0, 9.0, C_TEXT);
                let minus = Rect::new(ix + 212.0, y, ix + 230.0, y + 22.0);
                let plus = Rect::new(ix + 232.0, y, ix + 250.0, y + 22.0);
                stroke_rect(&mut ui, minus, C_PANEL_EDGE, 1.0);
                stroke_rect(&mut ui, plus, C_PANEL_EDGE, 1.0);
                label(&mut ui, "-", minus.x0 + 6.0, y + 4.0, 10.0, C_TEXT);
                label(&mut ui, "+", plus.x0 + 5.0, y + 4.0, 10.0, C_TEXT);
            }
            label(&mut ui, "SHIFT  CONSTRAIN", ix + 12.0, TOP_H + 136.0, 8.0, C_DIM);
            label(&mut ui, "ALT  DRAW FROM CENTER", ix + 12.0, TOP_H + 152.0, 8.0, C_DIM);
        } else if self.editor.selection.len() > 1 {
            label(&mut ui, &format!("{} layers selected", self.editor.selection.len()), ix + 12.0, TOP_H + 34.0, 9.5, C_SECTION);
            label(&mut ui, "USE THE ALIGN ROW OR", ix + 12.0, TOP_H + 52.0, 8.0, C_DIM);
            label(&mut ui, "⌘G TO GROUP", ix + 12.0, TOP_H + 64.0, 8.0, C_DIM);
        } else if self.inspector_tab == 0 {
            // friendly empty state: quick-start card — DESIGN tab only
            // (VARS/LIBS tabs draw their own content above)
            let card = Rect::new(ix + 10.0, TOP_H + 30.0, self.win_w - 10.0, TOP_H + 168.0);
            fill_rrect(&mut ui, card, 8.0, Color::rgba8(0x2a, 0x2c, 0x33, 200));
            label(&mut ui, "Get started", card.x0 + 10.0, card.y0 + 8.0, 9.5, C_SECTION);
            for (i, line) in [
                "R  DRAW A RECTANGLE",
                "T  ADD TEXT",
                "F  PHONE/DESKTOP FRAME",
                "⌘P  PLAY PROTOTYPE",
                "?  ALL SHORTCUTS",
            ].iter().enumerate() {
                label(&mut ui, line, card.x0 + 10.0, card.y0 + 30.0 + i as f64 * 20.0, 8.0, C_DIM);
            }
        }


        // ---------- rulers (X-Native Shift+R) ----------
        if self.rulers {
            let c = self.canvas_rect();
            fill_rect(&mut ui, Rect::new(c.x0, c.y0, c.x1, c.y0 + 16.0), Color::rgba8(0x1a, 0x1a, 0x1a, 240));
            fill_rect(&mut ui, Rect::new(c.x0, c.y0, c.x0 + 16.0, c.y1), Color::rgba8(0x1a, 0x1a, 0x1a, 240));
            // ticks every 100 page units
            let step = 100.0 * self.zoom;
            if step > 20.0 {
                let (ox, oy) = self.canvas_origin();
                let start_x = ((c.x0 - ox - self.pan.0) / step).floor() as i64;
                let end_x = ((c.x1 - ox - self.pan.0) / step).ceil() as i64;
                for i in start_x..=end_x {
                    let sx = ox + self.pan.0 + i as f64 * step;
                    if sx < c.x0 + 16.0 || sx > c.x1 { continue; }
                    ui.stroke(&vello::kurbo::Stroke::new(1.0), Affine::IDENTITY, C_DIM, None,
                        &vello::kurbo::Line::new((sx, c.y0 + 10.0), (sx, c.y0 + 16.0)));
                    label(&mut ui, &format!("{}", i * 100), sx + 2.0, c.y0 + 2.0, 6.0, C_DIM);
                }
                let start_y = ((c.y0 - oy - self.pan.1) / step).floor() as i64;
                let end_y = ((c.y1 - oy - self.pan.1) / step).ceil() as i64;
                for i in start_y..=end_y {
                    let sy = oy + self.pan.1 + i as f64 * step;
                    if sy < c.y0 + 16.0 || sy > c.y1 { continue; }
                    ui.stroke(&vello::kurbo::Stroke::new(1.0), Affine::IDENTITY, C_DIM, None,
                        &vello::kurbo::Line::new((c.x0 + 10.0, sy), (c.x0 + 16.0, sy)));
                    label(&mut ui, &format!("{}", i * 100), c.x0 + 2.0, sy + 2.0, 6.0, C_DIM);
                }
            }
        }

        self.phase_ms.2 = chrome_t0.elapsed().as_secs_f32() * 1000.0 - self.phase_ms.0 - self.phase_ms.1;
        // ---------- perf HUD (Ctrl+Shift+F): frame-time instrumentation ----------
        if self.perf_hud && !self.frame_times.is_empty() {
            let n = self.frame_times.len() as f32;
            let avg: f32 = self.frame_times.iter().sum::<f32>() / n;
            let max = self.frame_times.iter().cloned().fold(0.0f32, f32::max);
            let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };
            let hud = Rect::new(LAYERS_W + 10.0, TOP_H + 8.0, LAYERS_W + 330.0, TOP_H + 98.0);
            fill_rrect(&mut ui, hud, 6.0, Color::rgba8(0, 0, 0, 180));
            label(&mut ui, &format!("FRAME {avg:.2} MS AVG / {max:.2} MS MAX", ), hud.x0 + 8.0, hud.y0 + 6.0, 8.5, C_TEXT);
            // phase breakdown: evidence-driven optimization (review)
            let (ir, enc, chrome) = self.phase_ms;
            let other = (avg - ir - enc - chrome).max(0.0);
            let (h_txt, m_txt) = arco_native::text::ShapedTextCache::global().stats();
            label(&mut ui, &format!("IR {ir:.1}  ENCODE {enc:.1}{}  CHROME {chrome:.1}  OTHER {other:.1}",
                if self.encode_skipped { " (CACHED)" } else { "" }), hud.x0 + 8.0, hud.y0 + 18.0, 7.5, C_TEXT);
            label(&mut ui, &format!("{fps:.0} FPS | TEXT CACHE {h_txt}H/{m_txt}M | IR CACHE {}",
                if self.encode_skipped { "HIT" } else { "ENC" }), hud.x0 + 8.0, hud.y0 + 30.0, 7.0, C_DIM);
            label(&mut ui, &format!("CULLED {}", self.scene_cache.stats.culled), hud.x1 - 74.0, hud.y0 + 6.0, 7.5,
                if self.scene_cache.stats.culled > 0 { PALETTE[2] } else { C_DIM });
            if let Some((name, ms)) = &self.last_cmd {
                label(&mut ui, &format!("LAST CMD {name} {ms:.2}MS"),
                    hud.x0 + 8.0, hud.y0 + 41.0, 7.0, if *ms > 16.7 { PALETTE[1] } else { C_DIM });
            }
            // MEMORY BREAKDOWN (review item 3): doc / caches / gpu / undo / libs
            {
                let mut d = Document::new();
                d.pages = self.pages.clone();
                d.styles = self.styles.clone();
                d.assets = self.store.clone();
                d.library_snapshots = self.library_snapshots.clone();
                let m = d.memory_breakdown();
                let (txt_b, _) = arco_native::text::ShapedTextCache::global().memory();
                let gpu_b = self.assets.memory_bytes();
                let seg_b = self.scene_cache.memory_bytes();
                let undo_b = self.editor.history_bytes();
                label(&mut ui, &format!(
                    "MEM DOC {:.1} ASSETS {:.1} LIBS {:.1} | TXT {:.1} SEG {:.1} GPU {:.1} UNDO {:.1} MB",
                    m.pages as f64 / 1e6, m.assets as f64 / 1e6, m.libraries as f64 / 1e6,
                    txt_b as f64 / 1e6, seg_b as f64 / 1e6, gpu_b as f64 / 1e6, undo_b as f64 / 1e6),
                    hud.x0 + 8.0, hud.y0 + 52.0, 6.5, C_DIM);
            }
            // sparkline
            let base = hud.y1 - 6.0;
            let scale = (36.0 / max.max(1.0)) as f64;
            for (i, ms) in self.frame_times.iter().enumerate() {
                let x = hud.x0 + 8.0 + i as f64 * 3.4;
                let h = (*ms as f64 * scale).min(34.0);
                let c = if *ms > 16.7 { PALETTE[1] } else { PALETTE[2] };
                fill_rect(&mut ui, Rect::new(x, base - h, x + 2.4, base), c);
            }
        }

        // ---------- asset browser overlay (Shift+A) ----------
        if self.asset_browser {
            // THUMBNAIL VIRTUALIZATION: decode ONLY tiles in the visible
            // window (+ the next page as preload). sync_store decodes on
            // demand per id; far-away assets stay raw bytes in the store.
            {
                let visible: Vec<String> = self.asset_layout().iter()
                    .filter(|(_, _, k)| *k == 0)
                    .map(|(id, _, _)| id.clone())
                    .collect();
                for id in &visible {
                    if self.assets.get(id).is_none() {
                        if let Some(rec) = self.store.get(id) {
                            if rec.mime == "image/png" {
                                let _ = self.assets.load_png_bytes(&id.clone(), &rec.bytes.clone());
                            }
                        }
                    }
                }
            }
            let panel = self.asset_panel_rect();
            fill_rect(&mut ui, Rect::new(0.0, 0.0, self.win_w, self.win_h), Color::rgba8(0, 0, 0, 120));
            fill_rrect(&mut ui, panel, 12.0, Color::rgba8(0x24, 0x26, 0x2b, 252));
            stroke_rect(&mut ui, panel, C_PANEL_EDGE, 1.0);
            label(&mut ui, &format!("ASSETS ({})", self.store.len()), panel.x0 + 16.0, panel.y0 + 12.0, 12.0, C_TEXT);
            label(&mut ui, "SHIFT+A CLOSE", panel.x1 - 110.0, panel.y0 + 14.0, 8.0, C_DIM);
            for (tag, r, kind) in self.asset_layout() {
                match kind {
                    1 => {
                        let active = self.focus == Focus::AssetSearch;
                        stroke_rect(&mut ui, r, if active { PALETTE[4] } else { C_PANEL_EDGE }, 1.0);
                        let shown = if self.asset_query.is_empty() && !active { "SEARCH NAME OR MIME".into() }
                            else { format!("{}{}", self.asset_query, if active { "_" } else { "" }) };
                        label(&mut ui, &shown, r.x0 + 6.0, r.y0 + 4.0, 8.0,
                            if self.asset_query.is_empty() && !active { C_DIM } else { C_TEXT });
                    }
                    2 => {
                        if let Some(srt) = tag.strip_prefix("SORT") {
                            let idx: u8 = srt.parse().unwrap_or(0);
                            let active = self.asset_sort == idx;
                            if active { fill_rrect(&mut ui, r, 3.0, C_ACCENT); }
                            else { stroke_rect(&mut ui, r, C_PANEL_EDGE, 1.0); }
                            label(&mut ui, ["NAME", "SIZE", "USED"][idx as usize], r.x0 + 6.0, r.y0 + 3.0, 7.5,
                                if active { Color::WHITE } else { C_TEXT });
                        } else {
                            let danger = tag == "DEL UNUSED";
                            stroke_rect(&mut ui, r, if danger { PALETTE[1] } else { C_PANEL_EDGE }, 1.0);
                            label(&mut ui, &tag, r.x0 + 8.0, r.y0 + 3.0, 8.0, C_TEXT);
                        }
                    }
                    _ => {
                        let selected = self.asset_sel.as_deref() == Some(tag.as_str());
                        fill_rrect(&mut ui, r, 4.0, Color::rgba8(0x17, 0x18, 0x1c, 255));
                        // real thumbnail from the decoded GPU cache
                        if let Some(img) = self.assets.get(&tag) {
                            let (iw, ih) = (img.width as f64, img.height as f64);
                            let s = (r.width() / iw).min(r.height() / ih).min(4.0);
                            let (ox, oy) = (r.x0 + (r.width() - iw * s) / 2.0, r.y0 + (r.height() - ih * s) / 2.0);
                            ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY, &r.to_path(0.1));
                            ui.draw_image(img, Affine::translate((ox, oy)) * Affine::scale(s));
                            ui.pop_layer();
                        } else {
                            label(&mut ui, "NO PREVIEW", r.x0 + 20.0, r.y0 + r.height() / 2.0 - 4.0, 8.0, C_DIM);
                        }
                        stroke_rect(&mut ui, r, if selected { C_ACCENT } else { C_PANEL_EDGE }, if selected { 2.0 } else { 1.0 });
                        // caption: name + dims + usage
                        if let Some(rec) = self.store.get(&tag) {
                            let usage: usize = self.pages.iter().enumerate()
                                .map(|(i, pg)| if i == self.page_idx { arco_native::asset_usage(&self.editor.root, &tag) } else { arco_native::asset_usage(pg, &tag) })
                                .sum();
                            let dims = rec.dimensions.map(|(w0, h0)| format!("{w0}x{h0}")).unwrap_or_default();
                            let cap = format!("{} {dims} {}x", rec.name.chars().take(12).collect::<String>(), usage);
                            label(&mut ui, &cap, r.x0 + 2.0, r.y1 + 3.0, 7.0, if selected { C_TEXT } else { C_DIM });
                        }
                    }
                }
            }
            if self.store.is_empty() {
                label(&mut ui, "NO ASSETS YET — IMPORT A SKETCH/PNG OR DROP PNGS IN assets/", panel.x0 + 16.0, panel.y0 + 100.0, 9.0, C_DIM);
            }
        }

        // ---------- import preview overlay (report + accept/cancel) ----------
        if let Some((src, doc, report)) = &self.import_pending {
            let panel = Rect::new(self.win_w / 2.0 - 260.0, self.win_h / 2.0 - 190.0,
                                  self.win_w / 2.0 + 260.0, self.win_h / 2.0 + 190.0);
            fill_rect(&mut ui, Rect::new(0.0, 0.0, self.win_w, self.win_h), Color::rgba8(0, 0, 0, 130));
            fill_rrect(&mut ui, panel, 12.0, Color::rgba8(0x24, 0x26, 0x2b, 252));
            stroke_rect(&mut ui, panel, C_PANEL_EDGE, 1.0);
            label(&mut ui, &format!("IMPORT PREVIEW — {}", src.to_uppercase()), panel.x0 + 20.0, panel.y0 + 14.0, 12.0, C_TEXT);
            label(&mut ui, &format!("{} PAGE(S), {} NODE(S), {} ASSET(S)",
                doc.pages.len(), report.nodes_imported, report.assets_imported),
                panel.x0 + 20.0, panel.y0 + 34.0, 9.0, C_DIM);
            // page thumbnails through the real IR (preview = actual render)
            let mut tx = panel.x0 + 20.0;
            for page in doc.pages.iter().take(3) {
                let tree = arco_native::build_render_tree(page, &doc.variables);
                let (thumb, _scale) = arco_native::thumbnail_scene(&tree, page.w, page.h, 150.0, 100.0);
                let frame_r = Rect::new(tx, panel.y0 + 52.0, tx + 150.0, panel.y0 + 152.0);
                fill_rect(&mut ui, frame_r, Color::WHITE);
                ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY, &frame_r.to_path(0.1));
                ui.append(&thumb, Some(Affine::translate((tx, panel.y0 + 52.0))));
                ui.pop_layer();
                stroke_rect(&mut ui, frame_r, C_PANEL_EDGE, 1.0);
                label(&mut ui, &page.id.chars().take(16).collect::<String>(), tx, panel.y0 + 156.0, 7.0, C_DIM);
                tx += 160.0;
            }
            // diagnostics (fidelity report)
            let mut dy = panel.y0 + 176.0;
            label(&mut ui, if report.diagnostics.is_empty() { "NO FIDELITY WARNINGS" } else { "FIDELITY WARNINGS:" },
                panel.x0 + 20.0, dy, 8.5, if report.diagnostics.is_empty() { PALETTE[2] } else { PALETTE[4] });
            dy += 15.0;
            for d in report.diagnostics.iter().take(8) {
                label(&mut ui, &d.chars().take(70).collect::<String>(), panel.x0 + 24.0, dy, 7.5, C_DIM);
                dy += 12.0;
            }
            if report.diagnostics.len() > 8 {
                label(&mut ui, &format!("… {} MORE", report.diagnostics.len() - 8), panel.x0 + 24.0, dy, 7.5, C_DIM);
            }
            let acc = Rect::new(panel.x0 + 20.0, panel.y1 - 40.0, panel.x0 + 110.0, panel.y1 - 16.0);
            fill_rrect(&mut ui, acc, 4.0, C_ACCENT);
            label(&mut ui, "ACCEPT", acc.x0 + 22.0, acc.y0 + 7.0, 9.0, Color::WHITE);
            let can = Rect::new(panel.x0 + 120.0, panel.y1 - 40.0, panel.x0 + 210.0, panel.y1 - 16.0);
            stroke_rect(&mut ui, can, C_PANEL_EDGE, 1.0);
            label(&mut ui, "CANCEL", can.x0 + 22.0, can.y0 + 7.0, 9.0, C_TEXT);
        }

        // ---------- "?" shortcuts overlay ----------
        if self.help_open {
            let c = self.canvas_rect();
            let panel = Rect::new(c.x0 + 120.0, c.y0 + 60.0, c.x1 - 120.0, c.y1 - 100.0);
            fill_rect(&mut ui, c, Color::rgba8(0, 0, 0, 120));
            fill_rrect(&mut ui, panel, 12.0, Color::rgba8(0x24, 0x26, 0x2b, 250));
            stroke_rect(&mut ui, panel, C_PANEL_EDGE, 1.0);
            label(&mut ui, "KEYBOARD SHORTCUTS", panel.x0 + 20.0, panel.y0 + 14.0, 12.0, C_TEXT);
            let cols = [
                ["V MOVE", "H HAND / SPACE", "K SCALE", "F FRAME", "R RECT", "O ELLIPSE", "L LINE", "P POLYGON", "S STAR", "T TEXT"],
                ["⌘Z UNDO", "⇧⌘Z REDO", "⌘D DUPLICATE", "⌥+DRAG COPY", "⌘G GROUP", "⇧⌘G UNGROUP", "⌘A SELECT ALL", "DEL DELETE", "ESC PARENT/CLOSE", "ARROWS NUDGE (⇧=10)"],
                ["⌘S SAVE", "⌘O OPEN", "⌘I IMPORT", "⌘E EXPORT SVG", "⌥⌘K COMPONENT", "⌘P PRESENT", "⌘0/1 ZOOM", "⇧R RULERS", "⌘Y OUTLINE", "⌘. HIDE UI"],
            ];
            for (ci, col) in cols.iter().enumerate() {
                let cx = panel.x0 + 20.0 + ci as f64 * ((panel.width() - 40.0) / 3.0);
                for (ri, item) in col.iter().enumerate() {
                    label(&mut ui, item, cx, panel.y0 + 44.0 + ri as f64 * 22.0, 8.0, if ri % 2 == 0 { C_TEXT } else { C_DIM });
                }
            }
            label(&mut ui, "PRESS ? OR ESC TO CLOSE", panel.x0 + 20.0, panel.y1 - 24.0, 8.0, C_DIM);
        }

        // ---------- minimap (X-Native; OFF by default — mockup has none;
        // toggle via View ▸ Minimap) ----------
        if self.minimap {
            let mm = self.minimap_rect();
            fill_rrect(&mut ui, Rect::new(mm.x0 + 2.0, mm.y0 + 2.0, mm.x1 + 2.0, mm.y1 + 2.0), 8.0, Color::rgba8(0, 0, 0, 80));
            fill_rrect(&mut ui, mm, 8.0, Color::rgba8(0x24, 0x26, 0x2b, 235));
            stroke_rect(&mut ui, mm, C_PANEL_EDGE, 1.0);
            let page = &self.editor.root;
            let s = (mm.width() / page.w.max(1.0)).min(mm.height() / page.h.max(1.0));
            // page outline
            stroke_rect(&mut ui, Rect::new(mm.x0, mm.y0, mm.x0 + page.w * s, mm.y0 + page.h * s), C_DIM, 1.0);
            // top-level children as blocks
            for c in &page.children {
                if !c.visible { continue; }
                let r = Rect::new(
                    mm.x0 + c.transform.x * s, mm.y0 + c.transform.y * s,
                    mm.x0 + (c.transform.x + c.w) * s, mm.y0 + (c.transform.y + c.h) * s,
                );
                let col = match &c.fill { Paint::Solid(col) if col.a > 0 => *col, _ => C_DIM };
                fill_rect(&mut ui, r, col.with_alpha_factor(0.9));
            }
            // viewport rectangle
            let c = self.canvas_rect();
            let (ox, oy) = self.canvas_origin();
            let vx0 = (c.x0 - ox - self.pan.0) / self.zoom;
            let vy0 = (c.y0 - oy - self.pan.1) / self.zoom;
            let vx1 = (c.x1 - ox - self.pan.0) / self.zoom;
            let vy1 = (c.y1 - oy - self.pan.1) / self.zoom;
            let vr = Rect::new(
                (mm.x0 + vx0 * s).max(mm.x0), (mm.y0 + vy0 * s).max(mm.y0),
                (mm.x0 + vx1 * s).min(mm.x1), (mm.y0 + vy1 * s).min(mm.y1),
            );
            if vr.x1 > vr.x0 && vr.y1 > vr.y0 { stroke_rect(&mut ui, vr, C_ACCENT, 1.2); }
        }

        // ---------- open header dropdown menu (REAL menus, painted above
        // every panel; geometry shared with mouse_down via menu_layout) ----
        if self.menu_open.is_some() {
            let rows = self.menu_layout();
            if let (Some(first), Some(last)) = (rows.first(), rows.last()) {
                let panel = Rect::new(first.3.x0, first.3.y0, first.3.x1, last.3.y1)
                    .inflate(0.0, 4.0);
                fill_rrect(&mut ui, panel, 8.0, Color::rgba8(0x20, 0x22, 0x28, 252));
                stroke_rect(&mut ui, panel, C_PANEL_EDGE, 1.0);
                for (label_, shortcut, tag, r) in &rows {
                    let enabled = self.menu_item_enabled(tag);
                    let hover = enabled && r.contains(self.cursor);
                    if hover { fill_rrect(&mut ui, Rect::new(r.x0 + 4.0, r.y0 + 1.0, r.x1 - 4.0, r.y1 - 1.0), 5.0, C_ACCENT); }
                    let fg = if !enabled { Color::rgba8(0x84, 0x88, 0x92, 110) }
                        else if hover { Color::WHITE } else { C_TEXT };
                    label(&mut ui, label_, r.x0 + 14.0, r.y0 + 7.0, 9.0, fg);
                    if !shortcut.is_empty() {
                        let sw = ui_measure(shortcut, 7.0);
                        label(&mut ui, shortcut, r.x1 - sw - 12.0, r.y0 + 8.0, 7.0,
                            if !enabled { Color::rgba8(0x84, 0x88, 0x92, 90) } else if hover { Color::WHITE } else { C_DIM });
                    }
                }
            }
        }

        // ---------- retained overlays (x-ui): context menu + tooltip ----------
        {
            let theme = arco_native::ui::Theme::default();
            paint_ui_ops(&mut ui, &self.ctx_menu.paint(&theme));
            paint_ui_ops(&mut ui, &self.tooltip.paint(&theme));
        }

        ui
    }
}

fn blend_short(blend: BlendKind) -> &'static str {
    match blend {
        BlendKind::Normal => "N", BlendKind::Darken => "DK", BlendKind::Multiply => "M", BlendKind::ColorBurn => "CB",
        BlendKind::Lighten => "LT", BlendKind::Screen => "S", BlendKind::ColorDodge => "CD", BlendKind::Overlay => "O",
        BlendKind::SoftLight => "SL", BlendKind::HardLight => "HL", BlendKind::Difference => "DF", BlendKind::Exclusion => "EX",
        BlendKind::Hue => "H", BlendKind::Saturation => "ST", BlendKind::Color => "C", BlendKind::Luminosity => "LU",
    }
}

// ------------------------------------------------------------ small helpers




impl App {
    /// Card geometry shared by painter and click handler (no drift).
    /// Cards matching the search query (name substring, case-insensitive).
    pub fn dash_visible(&self) -> Vec<usize> {
        let q = self.dash_query.to_ascii_lowercase();
        self.dash_files.iter().enumerate()
            .filter(|(_, f)| q.is_empty() || f.name.to_ascii_lowercase().contains(&q))
            .map(|(i, _)| i).collect()
    }

    pub fn dash_layout(&self) -> Vec<(String, Rect, u8)> {
        // kinds: 0 file card, 1 New File button, 2 search box
        let mut out = vec![];
        out.push(("new".into(), Rect::new(self.win_w - 150.0, 24.0, self.win_w - 32.0, 58.0), 1));
        out.push(("search".into(), Rect::new(self.win_w - 420.0, 28.0, self.win_w - 170.0, 54.0), 2));
        let cols = ((self.win_w - 64.0) / 248.0).max(1.0) as usize;
        for (slot, i) in self.dash_visible().into_iter().enumerate() {
            let f = &self.dash_files[i];
            let col = slot % cols;
            let row = slot / cols;
            let x = 32.0 + col as f64 * 248.0;
            let y = 120.0 + row as f64 * 210.0;
            out.push((f.path.clone(), Rect::new(x, y, x + 232.0, y + 178.0), 0));
        }
        out
    }

    pub fn build_dashboard_scene(&mut self) -> Scene {
        let mut ui = Scene::new();
        // Base surface reads slightly darker than the editor's panel so the
        // grid of cards (drawn a step lighter) reads as the focal layer —
        // an identity choice, not a copy of any competitor's home screen.
        fill_rect(&mut ui, Rect::new(0.0, 0.0, self.win_w, self.win_h), Color::rgb8(0x0f, 0x10, 0x13));

        // ---- header: wordmark + tagline, search pill, primary action ----
        let mut xmark = vello::kurbo::BezPath::new();
        xmark.move_to((32.0, 30.0)); xmark.line_to((38.0, 37.0)); xmark.line_to((32.0, 44.0));
        xmark.line_to((36.0, 44.0)); xmark.line_to((40.0, 39.4)); xmark.line_to((44.0, 44.0));
        xmark.line_to((48.0, 44.0)); xmark.line_to((42.0, 37.0)); xmark.line_to((48.0, 30.0));
        xmark.line_to((44.0, 30.0)); xmark.line_to((40.0, 34.6)); xmark.line_to((36.0, 30.0));
        xmark.close_path();
        ui.fill(Fill::NonZero, Affine::IDENTITY, C_ACCENT, None, &xmark);
        label(&mut ui, "X-NATIVE", 58.0, 28.0, 12.0, C_TEXT);
        label(&mut ui, "Design, natively.", 58.0, 44.0, 8.0, C_DIM);

        // primary action: solid violet pill, right-aligned
        for (tag, r, kind) in self.dash_layout() {
            if kind == 1 {
                let hover = r.contains(self.cursor);
                fill_rrect(&mut ui, r, r.height() / 2.0, if hover { Color::rgb8(0x8f, 0x72, 0xff) } else { C_ACCENT });
                let plus_cx = r.x0 + 18.0;
                let plus_cy = (r.y0 + r.y1) / 2.0;
                let ps = vello::kurbo::Stroke::new(1.6).with_caps(vello::kurbo::Cap::Round);
                ui.stroke(&ps, Affine::IDENTITY, Color::WHITE, None, &vello::kurbo::Line::new((plus_cx - 4.0, plus_cy), (plus_cx + 4.0, plus_cy)));
                ui.stroke(&ps, Affine::IDENTITY, Color::WHITE, None, &vello::kurbo::Line::new((plus_cx, plus_cy - 4.0), (plus_cx, plus_cy + 4.0)));
                let tw = ui_measure("New file", 9.5);
                label(&mut ui, "New file", plus_cx + 12.0, r.y0 + (r.height() - 9.5) / 2.0 + 8.0, 9.5, Color::WHITE);
                let _ = (tag, tw);
            } else if kind == 2 {
                let active = self.focus == Focus::DashSearch;
                fill_rrect(&mut ui, r, r.height() / 2.0, Color::rgb8(0x1a, 0x1c, 0x22));
                stroke_rect(&mut ui, r, if active { C_ACCENT } else { Color::rgb8(0x26, 0x28, 0x30) }, if active { 1.3 } else { 1.0 });
                let (mx, my) = (r.x0 + 18.0, (r.y0 + r.y1) / 2.0 - 1.0);
                let st = vello::kurbo::Stroke::new(1.2).with_caps(vello::kurbo::Cap::Round);
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None, &vello::kurbo::Circle::new((mx, my), 3.5));
                ui.stroke(&st, Affine::IDENTITY, C_DIM, None,
                    &vello::kurbo::Line::new((mx + 2.6, my + 2.6), (mx + 5.6, my + 5.6)));
                let shown = if self.dash_query.is_empty() && !active { "Search files".into() }
                    else { format!("{}{}", self.dash_query, if active { "_" } else { "" }) };
                label(&mut ui, &shown, r.x0 + 32.0, r.y0 + 8.0, 9.0,
                    if self.dash_query.is_empty() && !active { C_DIM } else { C_TEXT });
            }
        }

        // hairline separating header chrome from the file grid
        stroke_rect(&mut ui, Rect::new(0.0, 72.0, self.win_w, 72.0), Color::rgb8(0x1d, 0x1f, 0x25), 1.0);

        // ---- section label with an accent tick, count as a soft chip ----
        fill_rrect(&mut ui, Rect::new(32.0, 98.0, 35.0, 112.0), 1.5, C_ACCENT);
        label(&mut ui, "Recents", 44.0, 100.0, 13.5, C_TEXT);
        if !self.dash_files.is_empty() {
            let count = format!("{}", self.dash_files.len());
            let cw = ui_measure(&count, 8.0);
            let chip = Rect::new(112.0, 100.0, 112.0 + cw + 14.0, 116.0);
            fill_rrect(&mut ui, chip, 8.0, Color::rgb8(0x22, 0x24, 0x2b));
            label(&mut ui, &count, chip.x0 + 7.0, 103.0, 8.0, C_DIM);
        }

        // ---- file grid: elevated cards with a fake drop shadow ----
        for (tag, r, kind) in self.dash_layout() {
            if kind != 0 { continue; }
            let hover = r.contains(self.cursor);
            let f = self.dash_files.iter().find(|f| f.path == tag);
            if hover {
                let shadow = Rect::new(r.x0 + 1.0, r.y0 + 3.0, r.x1 + 1.0, r.y1 + 5.0);
                fill_rrect(&mut ui, shadow, 10.0, Color::rgba8(0, 0, 0, 90));
            }
            fill_rrect(&mut ui, r, 10.0, Color::rgb8(0x1a, 0x1c, 0x22));
            stroke_rect(&mut ui, r, if hover { C_ACCENT } else { Color::rgb8(0x24, 0x26, 0x2e) }, if hover { 1.4 } else { 1.0 });
            // thumbnail area
            let tr = Rect::new(r.x0 + 8.0, r.y0 + 8.0, r.x1 - 8.0, r.y0 + 138.0);
            fill_rrect(&mut ui, tr, 6.0, Color::rgb8(0x25, 0x27, 0x2e));
            if let Some(f) = f {
                if let Some(thumb) = &f.thumb {
                    ui.push_layer(vello::peniko::Mix::Clip, 1.0, Affine::IDENTITY, &tr.to_path(0.1));
                    ui.append(thumb, Some(Affine::translate((tr.x0, tr.y0))));
                    ui.pop_layer();
                } else {
                    // no render yet: a quiet placeholder mark instead of a blank void
                    let cx = (tr.x0 + tr.x1) / 2.0;
                    let cy = (tr.y0 + tr.y1) / 2.0;
                    ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(0x35, 0x37, 0x40), None,
                        &vello::kurbo::RoundedRect::new(cx - 14.0, cy - 10.0, cx + 14.0, cy + 10.0, 3.0));
                }
                let renaming = matches!(&self.focus, Focus::DashRename { path, .. } if *path == f.path);
                if renaming {
                    if let Focus::DashRename { buffer, .. } = &self.focus {
                        let shown = if buffer.is_empty() { format!("{}_", f.name) } else { format!("{buffer}_") };
                        label(&mut ui, &shown, r.x0 + 10.0, r.y0 + 147.0, 9.5, PALETTE[4]);
                    }
                } else {
                    label(&mut ui, &f.name.chars().take(24).collect::<String>(), r.x0 + 10.0, r.y0 + 147.0, 9.5, C_TEXT);
                }
                label(&mut ui, &format!("{} page{} · {}", f.pages, if f.pages == 1 { "" } else { "s" }, f.modified), r.x0 + 10.0, r.y0 + 163.0, 7.5, C_DIM);
            }
        }

        if self.dash_files.is_empty() {
            let cx = self.win_w / 2.0;
            let cy = self.win_h / 2.0 - 20.0;
            ui.fill(Fill::NonZero, Affine::IDENTITY, Color::rgb8(0x22, 0x24, 0x2b), None,
                &vello::kurbo::RoundedRect::new(cx - 22.0, cy - 16.0, cx + 22.0, cy + 16.0, 6.0));
            let msg = "No files yet";
            let mw = ui_measure(msg, 11.0);
            label(&mut ui, msg, cx - mw / 2.0, cy + 32.0, 11.0, C_TEXT);
            let sub = "Click New file to start designing";
            let sw = ui_measure(sub, 8.5);
            label(&mut ui, sub, cx - sw / 2.0, cy + 48.0, 8.5, C_DIM);
        }

        label(&mut ui, "Double-click a name to rename · right-click a card for duplicate/delete", 32.0, self.win_h - 46.0, 7.5, C_DIM);
        label(&mut ui, &self.status, 32.0, self.win_h - 28.0, 8.5, C_DIM);
        // context menu overlay (rename/duplicate/delete on cards)
        if self.ctx_menu.open {
            let theme = arco_native::ui::Theme::default();
            paint_ui_ops(&mut ui, &self.ctx_menu.paint(&theme));
        }
        ui
    }
}
