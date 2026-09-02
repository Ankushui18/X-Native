#[allow(unused_imports)]
use super::*;


pub fn run() {
    // production reliability: full recovery chain (exact -> autosave ->
    // lenient -> backups), then legacy-hash upgrade on the loaded doc
    let mut recovery_note: Option<String> = None;
    let doc = if std::path::Path::new(DOC_PATH).exists() {
        match x_native::fileio::open_with_recovery(DOC_PATH) {
            Some((mut d, outcome)) => {
                use x_native::fileio::OpenOutcome::*;
                match &outcome {
                    Clean => {}
                    RecoveredFromAutosave => recovery_note = Some("recovered unsaved changes from autosave".into()),
                    RecoveredLenient(n) => recovery_note = Some(format!("recovered corrupt document ({n} note(s))")),
                    RecoveredFromBackup(b) => recovery_note = Some(format!("recovered from backup {b}")),
                }
                let upgraded = x_native::fileio::upgrade_legacy_library_hashes(&mut d);
                if !upgraded.is_empty() { eprintln!("library integrity: upgraded legacy hashes for {upgraded:?}"); }
                d
            }
            None => demo_document(),
        }
    } else { demo_document() };
    x_native::fileio::push_recent(DOC_PATH);
    let vars = doc.variables.clone();
    let styles = doc.styles.clone();
    let doc_assets = doc.assets.clone();
    let doc_lib_deps = doc.library_deps.clone();
    let doc_lib_snaps = doc.library_snapshots.clone();
    let pages = if doc.pages.is_empty() { vec![Node::frame("page-1", 1600.0, 1000.0)] } else { doc.pages };
    let root = pages[0].clone();

    let mut app = App {
        editor: Editor::new(root), vars, tool: Tool::Select,
        polygon_sides: 6, star_points: 5, star_inner_ratio: 0.4, rect_radius: 4.0,
        gradient_stop: 0, gradient_editing: false, fill_layer_index: 0, stroke_layer_index: 0, effect_layer_index: 0,
        pan: (60.0, 40.0), zoom: 0.6,
        cursor: Point::ZERO, drag: Drag::None, shift: false, ctrl: false,
        alt: false, alt_dupe_done: false,
        status: "ready".into(), created_count: 0,
        win_w: 1280.0, win_h: 800.0, layer_rows: vec![],
        focus: Focus::None,
        last_click: std::time::Instant::now(),
        last_click_pos: Point::ZERO,
        pages, page_idx: 0,
        present: None,
        guides: vec![],
        stamping: None,
        hover: None,
        layers_scroll: 0,
        chrome_hidden: false,
        help_open: false,
        inspector_tab: 0,
        rulers: true, // mockup shows rulers on
        user_guides: vec![],
        outline_view: false,
        space_pan: false,
        layer_filter: String::new(),
        assets: x_native::Assets::new(),
        store: doc_assets,
        fonts: {
            let mut fm = x_native::text::FontManager::new();
            let n = fm.load_system_fonts();
            if n > 0 { eprintln!("typography: loaded {n} system font(s)"); }
            fm
        },
        sysfonts: x_native::text::SystemFonts::enumerate(),
        gfonts: x_native::text::GoogleFonts::new(),
        font_query: String::new(),
        font_scroll: 0,
        font_results: vec![],
        font_weights: vec![],
        pen_target: None,
        pen_placing: None,
        pen_pending_out: None,
        node_edit: None,
        anchor_drag: None,
        handle_drag: None,
        ctx_menu: x_native::ui::Menu::default(),
        tooltip: x_native::ui::TooltipState::default(),
        t0: std::time::Instant::now(),
        styles,
        style_query: String::new(),
        style_sel: None,
        asset_browser: false,
        asset_query: String::new(),
        asset_sel: None,
        library_deps: doc_lib_deps,
        library_snapshots: doc_lib_snaps,
        library_update: None,
        library_review: false,
        library_integrity: vec![],
        dirty_since_save: false,
        last_autosave: std::time::Instant::now(),
        frame_times: std::collections::VecDeque::with_capacity(64),
        perf_hud: false,
        asset_scroll: 0,
        asset_sort: 0,
        asset_drag: None,
        saved_undo_depth: 0,
        scene_cache: x_native::FrameCache::new(),
        phase_ms: (0.0, 0.0, 0.0),
        encode_skipped: false,
        layer_rows_fp: None,
        import_pending: None,
        last_cmd: None,
        left_tab: 0,
        menu_open: None,
        minimap: false,
        screen: Screen::Dashboard,
        dash_files: vec![],
        doc_path: DOC_PATH.to_string(),
        thumbs_collapsed: std::fs::read_to_string(".xprefs").map(|t| t.contains("thumbs=collapsed")).unwrap_or(false),
        dbl: false,
        dash_query: String::new(),
        dash_ctx_path: None,
        page_ctx_idx: None,
    };
    // load-time integrity sweep (review item): verify every pinned snapshot
    {
        let mut d = Document::new();
        d.library_deps = app.library_deps.clone();
        d.library_snapshots = app.library_snapshots.clone();
        let mut verdicts = vec![];
        for (id, st) in x_native::fileio::verify_document_libraries(&d) {
            let msg = format!("{st:?}");
            let ok = matches!(st, x_native::fileio::IntegrityStatus::Verified);
            if !ok {
                eprintln!("library integrity: {id}: {msg}");
                app.status = format!("LIBRARY WARNING: {id} {msg} — FROZEN");
            }
            verdicts.push((id.clone(), ok));
            app.library_integrity.push((id, msg));
        }
        // FREEZE-ON-CORRUPT: unverified snapshots removed from resolution;
        // bound values stay at last-applied (never resolve corrupt data)
        let frozen = x_native::freeze_unverified(&mut app.library_snapshots, &verdicts);
        if !frozen.is_empty() { eprintln!("library integrity: frozen {frozen:?}"); }
    }
    if let Some(note) = recovery_note { app.status = note; }
    // Phase 4.2: load any PNGs sitting next to the app as named assets;
    // an Image node with asset "logo" renders assets/logo.png if present.
    if let Ok(entries) = std::fs::read_dir("assets") {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "png") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let _ = app.assets.load_png(stem, path.to_str().unwrap_or_default());
                }
            }
        }
        if !app.assets.is_empty() { app.status = format!("loaded {} image asset(s)", app.assets.len()); }
    }
    // decode any embedded asset:// records carried by the opened document
    {
        let decoded = app.assets.sync_store(&app.store);
        if decoded > 0 { eprintln!("assets: decoded {decoded} embedded image(s) from the document store"); }
    }
    app.rebuild_layer_rows();
    // first run: persist the seeded demo doc so "Brand Dashboard" exists
    // as a real FILE the dashboard can list/open (persistent doc model)
    if !std::path::Path::new(&app.doc_path).exists() {
        app.save_document();
    }
    app.scan_dash_files();
    // font browser: full search over system families + the entire
    // Google Fonts catalog (fetched once, disk-cached for offline)
    app.refresh_font_results();
    let gf_count = app.gfonts.catalog().map(|c| c.len()).unwrap_or(0);
    eprintln!("fonts: {} system families + {} google families",
        app.sysfonts.families.len(), gf_count);

    let event_loop = EventLoop::new().expect("create event loop (needs a display)");
    let mut host = AppHost { app, window: None, gpu: None };
    event_loop.run_app(&mut host).expect("event loop");
}

/// wgpu 29 / vello 0.10 surface state. vello 0.10 dropped direct surface
/// rendering, so the pattern is: render the scene into an offscreen
/// Rgba8Unorm storage texture, then blit that onto the swapchain frame
/// with `wgpu::util::TextureBlitter`.
struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    offscreen_view: wgpu::TextureView,
    blitter: wgpu::util::TextureBlitter,
}

impl Gpu {
    fn new(window: &Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::default(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface), force_fallback_adapter: false,
        })).expect("no adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None, required_features: wgpu::Features::empty(), required_limits: wgpu::Limits::default(),
            ..Default::default()
        })).expect("no device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied()
            .find(|f| matches!(f, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm))
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format, width: size.width.max(1), height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync, desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto, view_formats: vec![],
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, RendererOptions {
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: std::num::NonZeroUsize::new(1),
            ..Default::default()
        }).expect("create vello renderer");

        Self {
            offscreen_view: Self::make_offscreen(&device, config.width, config.height),
            blitter: wgpu::util::TextureBlitter::new(&device, config.format),
            surface, device, queue, config, renderer,
        }
    }

    /// vello's compute target: Rgba8Unorm with STORAGE_BINDING (rendered by
    /// vello) + TEXTURE_BINDING (sampled by the blitter).
    fn make_offscreen(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello offscreen"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.offscreen_view = Self::make_offscreen(&self.device, self.config.width, self.config.height);
    }
}

/// winit 0.30 ApplicationHandler: the window (and GPU state) is created on
/// `resumed` instead of before the event loop runs.
struct AppHost {
    app: App,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
}

impl winit::application::ApplicationHandler for AppHost {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() { return; }
        let window = Arc::new(event_loop.create_window(
            winit::window::Window::default_attributes()
                .with_title("X Native Beta")
                .with_inner_size(PhysicalSize::new(1280, 800)),
        ).expect("create window"));
        self.gpu = Some(Gpu::new(&window));
        self.window = Some(window);
        // kick the first frame once everything exists
        if let Some(w) = self.window.as_ref() { w.request_redraw(); }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let app = &mut self.app;
        let window = self.window.as_ref().expect("window");
        let gpu = self.gpu.as_mut().expect("gpu");
        match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(new_size) => {
                    gpu.resize(new_size.width, new_size.height);
                    app.win_w = gpu.config.width as f64;
                    app.win_h = gpu.config.height as f64;
                    window.request_redraw();
                }
                WindowEvent::ModifiersChanged(m) => {
                    app.shift = m.state().shift_key();
                    app.ctrl = m.state().control_key() || m.state().super_key();
                    app.alt = m.state().alt_key();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    app.mouse_move(Point::new(position.x, position.y));
                    if app.ctx_menu.open {
                        app.ctx_menu.hover(position.x, position.y);
                        window.request_redraw();
                    }
                    // retained tooltip over the bottom toolbar
                    let now = app.t0.elapsed().as_millis() as u64;
                    let bar = app.bottom_bar_rect();
                    if bar.contains(app.cursor) && !app.chrome_hidden {
                        let idx = ((app.cursor.x - bar.x0 - 8.0) / 38.0).floor();
                        if idx >= 0.0 && (idx as usize) < Tool::ALL.len() {
                            let t = Tool::ALL[idx as usize];
                            app.tooltip.hover(&format!("{}  {}", t.name(), t.label()), app.cursor.x, bar.y0, now);
                        } else { app.tooltip.leave(); }
                    } else { app.tooltip.leave(); }
                    app.tooltip.tick(now, 350, false);
                    if app.tooltip.visible { window.request_redraw(); }
                    if app.drag != Drag::None { window.request_redraw(); }
                }
                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    // open context menu consumes the next left click
                    if app.ctx_menu.open && state == ElementState::Pressed {
                        if let Some(i) = app.ctx_menu.click(app.cursor.x, app.cursor.y) {
                            if app.screen == Screen::Dashboard {
                                if let Some(path) = app.dash_ctx_path.clone() {
                                    match i {
                                        0 => app.open_file(&path),
                                        1 => {
                                            app.focus = Focus::DashRename { path: path.clone(), buffer: String::new() };
                                            app.status = "type the new file name, Enter commits".into();
                                        }
                                        2 => app.duplicate_dash_file(&path),
                                        3 => app.delete_dash_file(&path),
                                        _ => {}
                                    }
                                }
                                app.dash_ctx_path = None;
                                window.request_redraw();
                                return;
                            }
                            if let Some(idx) = app.page_ctx_idx.take() {
                                match i {
                                    0 => app.start_page_rename(idx),
                                    1 => { app.switch_page(idx); app.duplicate_page(); }
                                    2 => { app.switch_page(idx); app.reorder_page(-1); }
                                    3 => { app.switch_page(idx); app.reorder_page(1); }
                                    4 => app.delete_page(idx),
                                    _ => {}
                                }
                                window.request_redraw();
                                return;
                            }
                            app.run_menu_action(i);
                        }
                        window.request_redraw();
                        return;
                    }
                    match state {
                        ElementState::Pressed => app.mouse_down(app.cursor),
                        ElementState::Released => app.mouse_up(app.cursor),
                    }
                    window.request_redraw();
                }
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => {
                    if app.screen == Screen::Dashboard {
                        // card context menu: rename / duplicate / delete
                        app.dash_ctx_path = None;
                        for (tag, r, kind) in app.dash_layout() {
                            if kind == 0 && r.contains(app.cursor) { app.dash_ctx_path = Some(tag); break; }
                        }
                        if let Some(path) = app.dash_ctx_path.clone() {
                            let deletable = path != "document.x";
                            app.ctx_menu.items = vec![
                                x_native::ui::MenuItem { label: "OPEN".into(), shortcut: None, enabled: true },
                                x_native::ui::MenuItem { label: "RENAME".into(), shortcut: None, enabled: true },
                                x_native::ui::MenuItem { label: "DUPLICATE".into(), shortcut: None, enabled: true },
                                x_native::ui::MenuItem { label: "DELETE".into(), shortcut: None, enabled: deletable },
                            ];
                            app.ctx_menu.open_at(app.cursor.x, app.cursor.y);
                            window.request_redraw();
                        }
                        return;
                    }
                    // pages-list context menu: rename / duplicate / move / delete
                    if app.left_tab == 0 && app.cursor.x < LAYERS_W && app.cursor.y > TOP_H {
                        let pages_y0 = TOP_H + LPAGES_Y0;
                        let pages_end = pages_y0 + app.pages.len() as f64 * ROW_H;
                        if app.cursor.y >= pages_y0 && app.cursor.y < pages_end {
                            let idx = ((app.cursor.y - pages_y0) / ROW_H) as usize;
                            if idx < app.pages.len() {
                                app.page_ctx_idx = Some(idx);
                                app.ctx_menu.items = vec![
                                    x_native::ui::MenuItem { label: "RENAME".into(), shortcut: None, enabled: true },
                                    x_native::ui::MenuItem { label: "DUPLICATE".into(), shortcut: None, enabled: true },
                                    x_native::ui::MenuItem { label: "MOVE UP".into(), shortcut: None, enabled: idx > 0 },
                                    x_native::ui::MenuItem { label: "MOVE DOWN".into(), shortcut: None, enabled: idx + 1 < app.pages.len() },
                                    x_native::ui::MenuItem { label: "DELETE".into(), shortcut: None, enabled: app.pages.len() > 1 },
                                ];
                                app.ctx_menu.open_at(app.cursor.x, app.cursor.y);
                                window.request_redraw();
                            }
                            return;
                        }
                    }
                    // retained context menu (x-ui): select under cursor, then open
                    if app.canvas_rect().contains(app.cursor) && app.present.is_none() {
                        let wp = app.world_point(app.cursor);
                        // preserve multi-selection when right-clicking inside it
                        let hit = x_native::editor::hit_test(&app.editor.root, wp)
                            .and_then(|id| x_native::editor::top_level_ancestor(&app.editor.root, &id).or(Some(id)));
                        let inside_selection = hit.as_ref().is_some_and(|h| app.editor.selection.contains(h));
                        if !inside_selection {
                            app.editor.click_select(wp, false, false);
                        }
                        let has_sel = !app.editor.selection.is_empty();
                        let two = app.editor.selection.len() == 2;
                        app.ctx_menu.items = vec![
                            x_native::ui::MenuItem { label: "CUT".into(), shortcut: Some("⌘X".into()), enabled: has_sel },
                            x_native::ui::MenuItem { label: "COPY".into(), shortcut: Some("⌘C".into()), enabled: has_sel },
                            x_native::ui::MenuItem { label: "PASTE".into(), shortcut: Some("⌘V".into()), enabled: app.editor.clipboard_len() > 0 },
                            x_native::ui::MenuItem { label: "DUPLICATE".into(), shortcut: Some("⌘D".into()), enabled: has_sel },
                            x_native::ui::MenuItem { label: "DELETE".into(), shortcut: Some("DEL".into()), enabled: has_sel },
                            x_native::ui::MenuItem { label: "BRING TO FRONT".into(), shortcut: Some("⌘]".into()), enabled: has_sel },
                            x_native::ui::MenuItem { label: "SEND TO BACK".into(), shortcut: Some("⌘[".into()), enabled: has_sel },
                            x_native::ui::MenuItem { label: "GROUP".into(), shortcut: Some("⌘G".into()), enabled: app.editor.selection.len() >= 2 },
                            x_native::ui::MenuItem { label: "UNION".into(), shortcut: None, enabled: two },
                            x_native::ui::MenuItem { label: "SUBTRACT".into(), shortcut: None, enabled: two },
                            x_native::ui::MenuItem { label: "INTERSECT".into(), shortcut: None, enabled: two },
                            x_native::ui::MenuItem { label: "EXCLUDE".into(), shortcut: None, enabled: two },
                            x_native::ui::MenuItem { label: "USE AS MASK".into(), shortcut: None, enabled: has_sel },
                        ];
                        app.ctx_menu.open_at(app.cursor.x, app.cursor.y);
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if app.asset_browser {
                        let dy = match delta { MouseScrollDelta::LineDelta(_, y) => y as f64, MouseScrollDelta::PixelDelta(p) => p.y / 40.0 };
                        if dy < 0.0 { app.asset_scroll += 1; }
                        else if app.asset_scroll > 0 { app.asset_scroll -= 1; }
                        window.request_redraw();
                        return;
                    }
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x as f64 * 30.0, y as f64 * 30.0),
                        MouseScrollDelta::PixelDelta(p) => (p.x, p.y),
                    };
                    // wheel over layers panel scrolls the tree instead of the canvas
                    if app.cursor.x > app.win_w - INSPECTOR_W && app.cursor.y > TOP_H {
                        if dy < 0.0 { app.font_scroll = app.font_scroll.saturating_add(3); }
                        else if dy > 0.0 { app.font_scroll = app.font_scroll.saturating_sub(3); }
                    } else if app.cursor.x < LAYERS_W && app.cursor.y > TOP_H {
                        if dy < 0.0 { app.layers_scroll = app.layers_scroll.saturating_add(2); }
                        else if dy > 0.0 { app.layers_scroll = app.layers_scroll.saturating_sub(2); }
                    } else if app.ctrl {
                        let factor = (1.0 + dy / 300.0).clamp(0.5, 2.0);
                        let before = app.world_point(app.cursor);
                        app.zoom = (app.zoom * factor).clamp(0.05, 16.0);
                        let after = app.world_point(app.cursor);
                        app.pan.0 += (after.x - before.x) * app.zoom;
                        app.pan.1 += (after.y - before.y) * app.zoom;
                    } else {
                        app.pan.0 += dx;
                        app.pan.1 += dy;
                    }
                    window.request_redraw();
                }
                WindowEvent::KeyboardInput { event: kev, .. } => {
                    if kev.state == ElementState::Released {
                        if let Key::Named(NamedKey::Space) = &kev.logical_key { app.space_pan = false; }
                    }
                    if kev.state == ElementState::Pressed {
                        // text/field editing captures keystrokes first
                        if app.focus != Focus::None {
                            match &kev.logical_key {
                                Key::Named(NamedKey::Enter) => app.commit_focus(),
                                Key::Named(NamedKey::Tab) => app.focus_next_field(),
                                Key::Named(NamedKey::Escape) => {
                                    if app.focus == Focus::LayerSearch { app.layer_filter.clear(); }
                                    app.cancel_focus();
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    match &mut app.focus {
                                        Focus::TextNode { buffer, caret, sel_anchor, .. } => {
                                            // range-aware backspace: selection deletes the range
                                            if let Some(a) = sel_anchor.take() {
                                                let (lo, hi) = (a.min(*caret), a.max(*caret));
                                                buffer.replace_range(lo..hi, "");
                                                *caret = lo;
                                            } else if *caret > 0 {
                                                let mut c = *caret - 1;
                                                while c > 0 && !buffer.is_char_boundary(c) { c -= 1; }
                                                buffer.remove(c);
                                                *caret = c;
                                            }
                                        }
                                        Focus::Field { buffer, .. } => { buffer.pop(); }
                                        Focus::LayerSearch => { app.layer_filter.pop(); }
                                        Focus::LayerRename { buffer, .. } => { buffer.pop(); }
                                        Focus::FontSearch => { app.font_query.pop(); app.refresh_font_results(); }
                                        Focus::StyleSearch => { app.style_query.pop(); }
                                        Focus::StyleRename { buffer, .. } => { buffer.pop(); }
                                        Focus::AssetSearch => { app.asset_query.pop(); }
                                        Focus::AssetRename { buffer, .. } => { buffer.pop(); }
                                        Focus::PageRename { buffer, .. } => { buffer.pop(); }
                                        Focus::DashSearch => { app.dash_query.pop(); }
                                        Focus::DashRename { buffer, .. } => { buffer.pop(); }
                                        Focus::None => {}
                                    }
                                }
                                Key::Named(NamedKey::Delete) => {
                                    if let Focus::TextNode { buffer, caret, sel_anchor, .. } = &mut app.focus {
                                        if let Some(a) = sel_anchor.take() {
                                            let (lo, hi) = (a.min(*caret), a.max(*caret));
                                            buffer.replace_range(lo..hi, "");
                                            *caret = lo;
                                        } else if *caret < buffer.len() { buffer.remove(*caret); }
                                    }
                                }
                                Key::Named(NamedKey::ArrowLeft) => {
                                    if let Focus::TextNode { caret, buffer, sel_anchor, .. } = &mut app.focus {
                                        if app.shift { if sel_anchor.is_none() { *sel_anchor = Some(*caret); } }
                                        else { *sel_anchor = None; }
                                        if *caret > 0 {
                                            let mut c = *caret - 1;
                                            while c > 0 && !buffer.is_char_boundary(c) { c -= 1; }
                                            *caret = c;
                                        }
                                    }
                                }
                                Key::Named(NamedKey::ArrowRight) => {
                                    if let Focus::TextNode { caret, buffer, sel_anchor, .. } = &mut app.focus {
                                        if app.shift { if sel_anchor.is_none() { *sel_anchor = Some(*caret); } }
                                        else { *sel_anchor = None; }
                                        if *caret < buffer.len() {
                                            let mut c = *caret + 1;
                                            while c < buffer.len() && !buffer.is_char_boundary(c) { c += 1; }
                                            *caret = c;
                                        }
                                    }
                                }
                                Key::Named(NamedKey::Home) => {
                                    if let Focus::TextNode { caret, sel_anchor, .. } = &mut app.focus {
                                        if app.shift { if sel_anchor.is_none() { *sel_anchor = Some(*caret); } } else { *sel_anchor = None; }
                                        *caret = 0;
                                    }
                                }
                                Key::Named(NamedKey::End) => {
                                    if let Focus::TextNode { caret, buffer, sel_anchor, .. } = &mut app.focus {
                                        if app.shift { if sel_anchor.is_none() { *sel_anchor = Some(*caret); } } else { *sel_anchor = None; }
                                        *caret = buffer.len();
                                    }
                                }
                                Key::Named(NamedKey::Space) => {
                                    match &mut app.focus {
                                        Focus::TextNode { buffer, caret, sel_anchor, .. } => {
                                            if let Some(a) = sel_anchor.take() {
                                                let (lo, hi) = (a.min(*caret), a.max(*caret));
                                                buffer.replace_range(lo..hi, "");
                                                *caret = lo;
                                            }
                                            buffer.insert(*caret, ' ');
                                            *caret += 1;
                                        }
                                        Focus::StyleRename { buffer, .. } | Focus::LayerRename { buffer, .. } | Focus::AssetRename { buffer, .. } | Focus::PageRename { buffer, .. } | Focus::DashRename { buffer, .. } => buffer.push(' '),
                                        Focus::DashSearch => app.dash_query.push(' '),
                                        _ => {}
                                    }
                                }
                                Key::Character(c) => {
                                    // text-editing clipboard: Ctrl+C/V/X/A on the buffer
                                    // (Ctrl+V pulls the OS clipboard via xclip/wl-paste)
                                    if app.ctrl {
                                        if let Focus::TextNode { buffer, caret, sel_anchor, .. } = &mut app.focus {
                                            match c.as_str().to_ascii_lowercase().as_str() {
                                                "a" => {
                                                    // REAL select-all: anchor 0, caret end
                                                    *sel_anchor = Some(0);
                                                    *caret = buffer.len();
                                                    app.status = "all text selected".into();
                                                }
                                                "c" => {
                                                    // copy the RANGE if any, else the whole buffer
                                                    let (lo, hi) = sel_anchor.map(|a| (a.min(*caret), a.max(*caret))).unwrap_or((0, buffer.len()));
                                                    crate::os_clipboard_set(&buffer[lo..hi]);
                                                    app.status = format!("copied {} char(s) to OS clipboard", hi - lo);
                                                }
                                                "x" => {
                                                    let (lo, hi) = sel_anchor.take().map(|a| (a.min(*caret), a.max(*caret))).unwrap_or((0, buffer.len()));
                                                    crate::os_clipboard_set(&buffer[lo..hi]);
                                                    buffer.replace_range(lo..hi, "");
                                                    *caret = lo;
                                                    app.status = "cut to OS clipboard".into();
                                                }
                                                "v" => {
                                                    if let Some(t) = crate::os_clipboard_get() {
                                                        if let Some(a) = sel_anchor.take() {
                                                            let (lo, hi) = (a.min(*caret), a.max(*caret));
                                                            buffer.replace_range(lo..hi, "");
                                                            *caret = lo;
                                                        }
                                                        buffer.insert_str(*caret, &t);
                                                        *caret += t.len();
                                                        app.status = format!("pasted {} char(s) from OS clipboard", t.len());
                                                    } else { app.status = "OS clipboard empty/unavailable".into(); }
                                                }
                                                _ => {}
                                            }
                                            window.request_redraw();
                                            return;
                                        }
                                    }
                                    match &mut app.focus {
                                        Focus::TextNode { buffer, caret, sel_anchor, .. } => {
                                            if let Some(a) = sel_anchor.take() {
                                                let (lo, hi) = (a.min(*caret), a.max(*caret));
                                                buffer.replace_range(lo..hi, "");
                                                *caret = lo;
                                            }
                                            buffer.insert_str(*caret, c.as_str());
                                            *caret += c.as_str().len();
                                        }
                                        Focus::Field { buffer, .. } => {
                                            for ch in c.chars() {
                                                if ch.is_ascii_digit() || ch == '-' || ch == '.' { buffer.push(ch); }
                                            }
                                        }
                                        Focus::LayerSearch => app.layer_filter.push_str(c.as_str()),
                                        Focus::LayerRename { buffer, .. } => buffer.push_str(c.as_str()),
                                        Focus::FontSearch => { app.font_query.push_str(c.as_str()); app.refresh_font_results(); }
                                        Focus::StyleSearch => app.style_query.push_str(c.as_str()),
                                        Focus::StyleRename { buffer, .. } => buffer.push_str(c.as_str()),
                                        Focus::AssetSearch => app.asset_query.push_str(c.as_str()),
                                        Focus::AssetRename { buffer, .. } => buffer.push_str(c.as_str()),
                                        Focus::PageRename { buffer, .. } => buffer.push_str(c.as_str()),
                                        Focus::DashSearch => app.dash_query.push_str(c.as_str()),
                                        Focus::DashRename { buffer, .. } => buffer.push_str(c.as_str()),
                                        Focus::None => {}
                                    }
                                }
                                _ => {}
                            }
                            // live-preview text edits directly on the node
                            if let Focus::TextNode { id, buffer, .. } = &app.focus {
                                let id = id.clone(); let buf = buffer.clone();
                                if let Some(n) = x_native::editor::find_mut(&mut app.editor.root, &id) {
                                    if let x_native::NodeKind::Text { text } = &mut n.kind { *text = buf; }
                                }
                            }
                            window.request_redraw();
                            return;
                        }
                        let nudge = if app.shift { 10.0 } else { 1.0 };
                        if let Key::Named(NamedKey::Space) = &kev.logical_key {
                            app.space_pan = true;
                            window.request_redraw();
                            return;
                        }
                        match &kev.logical_key {
                            Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace) => { app.editor.delete_selection(); app.status = "deleted".into(); }
                            Key::Named(NamedKey::ArrowLeft) => app.editor.move_selection(-nudge, 0.0),
                            Key::Named(NamedKey::ArrowRight) => app.editor.move_selection(nudge, 0.0),
                            Key::Named(NamedKey::ArrowUp) => app.editor.move_selection(0.0, -nudge),
                            Key::Named(NamedKey::ArrowDown) => app.editor.move_selection(0.0, nudge),
                            Key::Named(NamedKey::Escape) => {
                                if app.menu_open.is_some() { app.menu_open = None; }
                                else if app.pen_target.is_some() { app.pen_target = None; app.pen_placing = None; app.pen_pending_out = None; app.status = "pen: path finished".into(); }
                                else if app.node_edit.is_some() { app.node_edit = None; app.status = "node edit: done".into(); }
                                else if app.help_open { app.help_open = false; }
                                else if app.present.is_some() { app.present = None; app.status = "exited presentation".into(); }
                                else if let Some(id) = app.editor.selection.first().cloned() {
                                    // Esc selects parent; at top level it deselects
                                    let root_id = app.editor.root.id.clone();
                                    let parent = x_native::editor::top_level_ancestor(&app.editor.root, &id);
                                    match parent {
                                        Some(pid) if pid != id => { app.editor.selection = vec![pid]; app.status = "selected parent".into(); }
                                        _ => { app.editor.selection.clear(); app.tool = Tool::Select; }
                                    }
                                    let _ = root_id;
                                }
                                else { app.editor.selection.clear(); app.tool = Tool::Select; }
                            }
                            Key::Character(c) => {
                                let ch = c.as_str();
                                if app.ctrl {
                                    // NB: with Shift held, winit reports the
                                    // UPPERCASE char ("Z"), so normalize.
                                    let lower = ch.to_ascii_lowercase();
                                    let cmd_t0 = std::time::Instant::now(); // command latency probe
                                    match lower.as_str() {
                                        "z" => { if app.shift { app.editor.redo(); } else { app.editor.undo(); } app.status = "undo/redo".into(); }
                                        "f" if app.shift => {
                                            app.perf_hud = !app.perf_hud;
                                            app.status = if app.perf_hud { "perf HUD on".into() } else { "perf HUD off".into() };
                                        }
                                        "d" => { app.editor.duplicate_selection((16.0, 16.0)); app.status = "duplicated".into(); }
                                        "c" => app.clipboard_copy(),
                                        "x" => app.clipboard_cut(),
                                        "v" => app.clipboard_paste(),
                                        "s" => { if app.shift { app.save_document_as(); } else { app.save_document(); } },
                                        "i" => app.start_import(),
                                        "o" => app.choose_and_open_document(),
                                        "e" => {
                                            if app.alt { app.export_png_now(); }
                                            else if app.shift { app.export_pdf_now(); }
                                            else { app.export_svg_now(); }
                                        }
                                        "p" => app.enter_present(),
                                        "." => { app.chrome_hidden = !app.chrome_hidden; app.status = if app.chrome_hidden { "UI hidden".into() } else { "UI shown".into() }; }
                                        "y" => { app.outline_view = !app.outline_view; app.status = if app.outline_view { "outline view".into() } else { "normal view".into() }; }
                                        ";" => { app.user_guides.clear(); app.status = "guides cleared".into(); }
                                        "0" => { app.zoom = 1.0; app.status = "zoom 100%".into(); }
                                        "1" => {
                                            // zoom-to-fit the page in the canvas area
                                            let cw = app.win_w - TOOLBAR_W - LAYERS_W - INSPECTOR_W - 40.0;
                                            let chh = app.win_h - TOP_H - 40.0;
                                            let pw = app.editor.root.w.max(1.0);
                                            let ph = app.editor.root.h.max(1.0);
                                            app.zoom = (cw / pw).min(chh / ph).clamp(0.02, 4.0);
                                            app.pan = (20.0, 20.0);
                                            app.status = format!("zoom to fit ({:.0}%)", app.zoom * 100.0);
                                        }
                                        "g" => {
                                            if app.shift {
                                                if let Some(id) = app.editor.selection.first().cloned() {
                                                    if app.editor.ungroup(&id) { app.status = "ungrouped".into(); }
                                                }
                                            } else if app.editor.selection.len() >= 2 {
                                                let gid = format!("group-{}", app.editor.undo_depth());
                                                app.editor.group_selection(&gid);
                                                app.status = format!("grouped -> {gid}");
                                            }
                                        }
                                        "a" => { app.editor.select_all(); app.status = format!("{} selected", app.editor.selection.len()); }
                                        "k" => {
                                            let n = app.editor.selection.len();
                                            let name = format!("Component{}", app.editor.component_names().len() + 1);
                                            if app.editor.make_component(&name) {
                                                app.status = format!("created component {name} from {n} node(s)");
                                            } else {
                                                app.status = "select sibling nodes first (⌥⌘K)".into();
                                            }
                                        }
                                        "]" => { if let Some(id) = app.editor.selection.first().cloned() { app.editor.bring_to_front(&id); app.status = "to front".into(); } }
                                        "[" => { if let Some(id) = app.editor.selection.first().cloned() { app.editor.send_to_back(&id); app.status = "to back".into(); } }
                                        _ => {}
                                    }
                                    app.last_cmd = Some((format!("ctrl+{lower}"), cmd_t0.elapsed().as_secs_f32() * 1000.0));
                                } else {
                                    match ch.to_ascii_lowercase().as_str() {
                                        "?" | "/" => { app.help_open = !app.help_open; }
                                        "a" => {
                                            if app.shift {
                                                app.asset_browser = !app.asset_browser;
                                                if app.asset_browser {
                                                    app.status = "asset browser (Shift+A to close)".into();
                                                } else {
                                                    // GPU/thumbnail EVICTION: closing drops decoded
                                                    // asset:// images the DOCUMENT doesn't reference
                                                    // (store keeps raw bytes; re-decode on demand)
                                                    let mut keep = std::collections::HashSet::new();
                                                    x_native::collect_asset_ids(&app.editor.root, &mut keep);
                                                    for (i, pg) in app.pages.iter().enumerate() {
                                                        if i != app.page_idx { x_native::collect_asset_ids(pg, &mut keep); }
                                                    }
                                                    let freed = app.assets.evict_except(&keep);
                                                    app.status = format!("asset browser closed ({:.1}MB thumbnails evicted)", freed as f64 / 1e6);
                                                }
                                            }
                                        }
                                        "h" if app.shift => {
                                            let ids = app.editor.selection.clone();
                                            let depth = app.editor.undo_depth();
                                            for id in ids { app.editor.flip_node(&id, true); }
                                            app.editor.merge_last(app.editor.undo_depth().saturating_sub(depth));
                                            app.status = "flipped horizontally".into();
                                        }
                                        "v" if app.shift => {
                                            let ids = app.editor.selection.clone();
                                            let depth = app.editor.undo_depth();
                                            for id in ids { app.editor.flip_node(&id, false); }
                                            app.editor.merge_last(app.editor.undo_depth().saturating_sub(depth));
                                            app.status = "flipped vertically".into();
                                        }
                                        "v" => app.tool = Tool::Select,
                                        "h" => app.tool = Tool::Hand,
                                        "k" => app.tool = Tool::Scale,
                                        "f" => app.tool = Tool::Frame,
                                        "r" => {
                                            if app.shift { app.rulers = !app.rulers; app.status = if app.rulers { "rulers on (click ruler to add guide)".into() } else { "rulers off".into() }; }
                                            else { app.tool = Tool::Rectangle; }
                                        }
                                        "o" => app.tool = Tool::Ellipse,
                                        "l" => app.tool = Tool::Line,
                                        "p" => app.tool = Tool::Polygon,
                                        "s" => app.tool = Tool::Star,
                                        "t" => app.tool = Tool::Text,
                                        "b" => { app.tool = Tool::Pen; app.pen_target = None; }
                                        "]" => { if let Some(id) = app.editor.selection.first().cloned() { app.editor.bring_forward(&id); app.status = "forward".into(); } }
                                        "[" => { if let Some(id) = app.editor.selection.first().cloned() { app.editor.send_backward(&id); app.status = "backward".into(); } }
                                        _ => {}
                                    }
                                    // "]"/"[" report their own status (z-order
                                    // change), not a tool switch — don't stomp it.
                                    if !app.shift && ch != "]" && ch != "[" { app.status = format!("tool: {:?}", app.tool); }
                                }
                            }
                            _ => {}
                        }
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    let frame_t0 = std::time::Instant::now();
                    // dirty tracking: any undoable command since last save
                    if app.editor.undo_depth() != app.saved_undo_depth { app.dirty_since_save = true; }
                    // autosave every 30s while dirty (atomic, non-blocking-ish)
                    if app.dirty_since_save && app.last_autosave.elapsed().as_secs() >= 30 {
                        app.pages[app.page_idx] = app.editor.root.clone();
                        let mut d = Document::new();
                        d.variables = app.vars.clone();
                        d.styles = app.styles.clone();
                        d.assets = app.store.clone();
                        d.library_deps = app.library_deps.clone();
                        d.library_snapshots = app.library_snapshots.clone();
                        d.pages = app.pages.clone();
                        let text = x_native::fileio::save_x(&d);
                        if x_native::fileio::autosave(&app.doc_path, &text).is_ok() {
                            eprintln!("autosave: {} bytes", text.len());
                        }
                        app.last_autosave = std::time::Instant::now();
                    }
                    let scene = app.build_display_scene();
                    let frame = match gpu.surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                            gpu.surface.configure(&gpu.device, &gpu.config);
                            return;
                        }
                        // Timeout / Occluded / Validation: skip this frame
                        _ => return,
                    };
                    let _ = gpu.renderer.render_to_texture(&gpu.device, &gpu.queue, &scene, &gpu.offscreen_view, &RenderParams {
                        base_color: if app.present.is_some() { Color::BLACK } else { C_CANVAS },
                        width: gpu.config.width, height: gpu.config.height,
                        antialiasing_method: AaConfig::Msaa16,
                    });
                    let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let mut blit_encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    gpu.blitter.copy(&gpu.device, &mut blit_encoder, &gpu.offscreen_view, &frame_view);
                    gpu.queue.submit([blit_encoder.finish()]);
                    frame.present();
                    // frame-time instrumentation (rolling 64 frames)
                    let ms = frame_t0.elapsed().as_secs_f32() * 1000.0;
                    if app.frame_times.len() >= 64 { app.frame_times.pop_front(); }
                    app.frame_times.push_back(ms);
                    // keep animating while a presentation transition runs
                    if app.present.as_ref().is_some_and(|p| p.transition.is_some()) {
                        window.request_redraw();
                    }
                }
                _ => {}
        }
    }
}
