//! X-Native Chrome — Native Implementation — HTML Source of Truth
//! Editor: v45-final-editor-28px.html — 28x28 logo, top tabs flush + next to X, left icons by type, right Size+Position Frame dropdown, Auto Layout 84x84 dark no 09 + dark icon, Guides no dropdown, Export collapsed + toggles
//! Dashboard: dashboard-v2.html — 40px top #111111 line #1F1F1F, 260px sidebar, #060606 main, 3-col grid 180px cards
//! Library: Lucide stroke 1.75px rounded — same as HTML — no external product naming
//! Design System: DESIGN_SYSTEM_FINAL.md — theme.rs tokens only

use std::{num::NonZeroUsize, sync::Arc};
use vello::{
    kurbo::{Affine, BezPath, Ellipse, Point, Rect, RoundedRect},
    peniko::Color,
    AaConfig, RenderParams, Renderer, RendererOptions, Scene,
};
use wgpu::{Backends, SurfaceConfiguration};
use winit::{
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, ModifiersState, NamedKey},
    window::Window,
};
use x_native::text::{encode_rich_text, Align, FontManager, Span, SystemFonts, TextBlockStyle};

#[derive(Clone, Copy)]
struct Palette {
    bg: Color,
    canvas: Color,
    panel: Color,
    field: Color,
    field2: Color,
    line: Color,
    line2: Color,
    text: Color,
    muted: Color,
    dim: Color,
    faint: Color,
    green: Color,
    avatar: Color,
    team_l: Color,
    team_d: Color,
    draft_dot: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            bg: crate::theme::C_BG,
            canvas: crate::theme::C_CANVAS,
            panel: crate::theme::C_PANEL,
            field: crate::theme::C_FIELD,
            field2: crate::theme::C_FIELD_2,
            line: crate::theme::C_LINE,
            line2: crate::theme::C_LINE_2,
            text: crate::theme::C_TEXT,
            muted: crate::theme::C_MUTED,
            dim: crate::theme::C_DIM,
            faint: crate::theme::C_FAINT,
            green: crate::theme::C_ACCENT_GREEN,
            avatar: crate::theme::C_AVATAR,
            team_l: crate::theme::C_TEAM_L,
            team_d: crate::theme::C_TEAM_D,
            draft_dot: crate::theme::C_DRAFT_DOT,
        }
    }
}

pub struct XNativeApp {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    renderer: Renderer,
    scene: Scene,
    fonts: FontManager,
    font: usize,
    palette: Palette,
    mouse: (f64, f64),
    selected: bool,
    zoom: f64,
    active_tool: usize,
    command_open: bool,
    modifiers: ModifiersState,
    left_w: f64,
    right_w: f64,
    export_expanded: bool,
}

impl XNativeApp {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps
                .present_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::PresentMode::Fifo)
                .unwrap_or(caps.present_modes[0]),
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: NonZeroUsize::new(1),
                ..Default::default()
            },
        )
        .expect("renderer");
        let mut fonts = FontManager::new();
        fonts.load_system_fonts();
        let font = SystemFonts::enumerate()
            .load_into(&mut fonts, "Inter", "Regular")
            .ok()
            .or_else(|| fonts.font_index("Inter Regular"))
            .or_else(|| fonts.font_index("Inter"))
            .or_else(|| fonts.default_font())
            .unwrap_or(0);
        Self {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            scene: Scene::new(),
            fonts,
            font,
            palette: Palette::default(),
            mouse: (0.0, 0.0),
            selected: false,
            zoom: 1.0,
            active_tool: 0,
            command_open: false,
            modifiers: ModifiersState::default(),
            left_w: 280.0,
            right_w: 340.0,
            export_expanded: false,
        }
    }

    pub fn handle_event(&mut self, event: Event<()>, elwt: &ActiveEventLoop) {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => {
                    self.resize(size.width, size.height);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.mouse = (position.x, position.y);
                    self.window.request_redraw();
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => {
                    self.on_click();
                    self.window.request_redraw();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let d = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 80.0,
                    };
                    let factor = if d > 0.0 { 1.12 } else { 0.89 };
                    self.zoom = (self.zoom * factor).clamp(0.05, 16.0);
                    self.window.request_redraw();
                }
                WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    self.on_key(logical_key);
                    self.window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    self.render();
                }
                _ => {}
            },
            Event::AboutToWait => self.window.request_redraw(),
            _ => {},
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
    }
    fn top_h(&self) -> f64 { 36.0 }
    fn left_w_val(&self) -> f64 { self.left_w }
    fn right_w_val(&self) -> f64 { self.right_w }

    fn on_click(&mut self) {
        let (x, y) = self.mouse;
        if self.command_open {
            self.command_open = false;
            return;
        }
        // Bottom toolbar
        let tools_x = (self.config.width as f64 * 0.5 - 130.0).max(self.left_w_val() + 24.0);
        let ty = self.config.height as f64 - 60.0;
        if y >= ty && y < ty + 36.0 && x >= tools_x && x < tools_x + 260.0 {
            self.active_tool = ((x - tools_x) / 36.0).floor().clamp(0.0, 5.0) as usize;
            return;
        }
        // Right panel export toggle
        let rx = self.config.width as f64 - self.right_w_val();
        let rw = self.right_w_val();
        let mut ey = 36.0 + 6.0 + 32.0 + 38.0 + 8.0 + 36.0 + 36.0 + 36.0 + 8.0 + 24.0 + 14.0 + 36.0 + 14.0 + 92.0 + 8.0 + 20.0 + 36.0 + 8.0 + 20.0 + 36.0 + 8.0 + 20.0 + 36.0 + 8.0 + 20.0 + 36.0 + 8.0 + 24.0 + 36.0 + 8.0;
        if y >= ey && y <= ey + 20.0 && x >= rx + rw - 28.0 && x <= rx + rw - 8.0 {
            self.export_expanded = !self.export_expanded;
            return;
        }
        // Canvas click toggles selection
        let left = self.left_w_val();
        let top = self.top_h();
        let right = self.right_w_val();
        let cw = self.config.width as f64 - left - right;
        let ch = self.config.height as f64 - top - 24.0;
        if x >= left && x <= left + cw && y >= top && y <= top + ch {
            self.selected = !self.selected;
        }
    }

    fn on_key(&mut self, key: Key) {
        let command = self.modifiers.super_key() || self.modifiers.control_key();
        match key {
            Key::Character(c) if command && c.eq_ignore_ascii_case("k") => {
                self.command_open = !self.command_open
            }
            Key::Named(NamedKey::Escape) => self.command_open = false,
            Key::Character(c) if c.eq_ignore_ascii_case("v") => self.active_tool = 0,
            Key::Character(c) if c.eq_ignore_ascii_case("f") => self.active_tool = 1,
            Key::Character(c) if c.eq_ignore_ascii_case("t") => self.active_tool = 2,
            Key::Character(c) if c.eq_ignore_ascii_case("r") => self.active_tool = 3,
            Key::Character(c) if c.eq_ignore_ascii_case("o") => self.active_tool = 4,
            Key::Character(c) if c.eq_ignore_ascii_case("p") => self.active_tool = 5,
            _ => {}
        }
    }

    fn render(&mut self) {
        self.scene.reset();
        let w = self.config.width as f64;
        let h = self.config.height as f64;
        let p = self.palette;
        let left = self.left_w_val();
        let right = self.right_w_val();
        let top = self.top_h();
        // Base #090909
        self.rect(0.0, 0.0, w, h, p.bg, 0.0);
        // Canvas #060606
        self.rect(left, top, w - left - right, h - top - 24.0, p.canvas, 0.0);
        // Frame white 375x420 default
        let s = ((w - left - right) / 800.0).min((h - top - 24.0) / 600.0).min(1.0) * self.zoom;
        let aw = 375.0 * s;
        let ah = 420.0 * s;
        let ax = left + (w - left - right - aw) * 0.5;
        let ay = top + (h - top - 24.0 - ah) * 0.5;
        self.round(ax, ay, aw, ah, Color::WHITE, 8.0 * s);
        self.draw_text("Frame", ax + aw * 0.5 - 20.0 * s, ay + ah * 0.5, 14.0 * s, Color::from_rgba8(0, 0, 0, 20));

        self.left_panel_native(p);
        self.right_panel_native(p);
        self.top_bar_native(p);
        self.bottom_toolbar_native(p);
        self.status_bar_native(p);

        if self.command_open {
            self.command_palette(p);
        }

        let output = self.surface.get_current_texture();
        let frame = match output {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let params = RenderParams {
            base_color: p.bg,
            width: self.config.width,
            height: self.config.height,
            antialiasing_method: AaConfig::Area,
        };
        if self
            .renderer
            .render_to_texture(&self.device, &self.queue, &self.scene, &view, &params)
            .is_ok()
        {
            frame.present();
        }
    }

    // === TOP BAR — FINAL 28x28 logo, tabs flush, + next to X ===
    fn top_bar_native(&mut self, p: Palette) {
        let w = self.config.width as f64;
        let top = self.top_h();
        self.rect(0.0, 0.0, w, top, p.panel, 0.0);
        self.line(0.0, top - 0.5, w, top - 0.5, p.line, 1.0);
        // Logo 28x28 — rect #1A1A1A + green #1BCB55 + white X — exact from v45 HTML
        self.round(6.0, (top - 28.0) * 0.5, 28.0, 28.0, p.field, 4.0);
        self.round(20.0, (top - 28.0) * 0.5 + 4.0, 10.0, 16.0, p.green, 2.0);
        self.draw_text("X", 13.0, (top - 28.0) * 0.5 + 7.0, 12.0, p.text);
        // Tabs flush — no floating gap — h = TITLE_H
        let mut tx = 44.0;
        let tabs = [
            ("Untitled", false, false),
            ("DESIGN_SYSTEM.md", true, true),
            ("Liquor App", false, false),
        ];
        for (name, active, is_md) in tabs.iter() {
            let tw = 132.0;
            if *active {
                self.rect(tx, 0.0, tw, top, p.field, 0.0);
                self.rect(tx, 0.0, tw, 1.5, p.text, 0.0);
            } else {
                self.rect(tx, 0.0, tw, top, p.panel, 0.0);
            }
            self.line(tx + tw, 0.0, tx + tw, top, p.line, 1.0);
            if *is_md {
                self.round(tx + 8.0, 13.0, 12.0, 12.0, Color::from_rgb8(0x51, 0x9A, 0xBA), 2.0);
                self.draw_text("✦", tx + 10.0, 13.0, 8.0, p.text);
                self.draw_text(name, tx + 26.0, 12.0, 10.0, if *active { p.text } else { p.dim });
            } else {
                self.draw_text(name, tx + 26.0, 12.0, 10.0, if *active { p.text } else { p.dim });
            }
            tx += tw;
        }
        self.rect(tx, 0.0, 32.0, top, p.panel, 0.0);
        self.draw_text("+", tx + 10.0, 11.0, 14.0, p.dim);
        // Profile S 12%
        let prx = w - 120.0;
        self.round(prx, 8.0, 24.0, 24.0, p.avatar, 12.0);
        self.draw_text("S", prx + 8.0, 12.0, 10.0, Color::BLACK);
        self.draw_text("12%", prx + 32.0, 12.0, 10.0, p.muted);
    }

    // === LEFT PANEL — FINAL — icons by type, resizable, file name below Draft editable ===
    fn left_panel_native(&mut self, p: Palette) {
        let top = self.top_h();
        let lw = self.left_w_val();
        let h = self.config.height as f64 - top - 24.0;
        self.rect(0.0, top, lw, h, p.panel, 0.0);
        self.line(lw - 0.5, top, lw - 0.5, top + h, p.line, 1.0);
        let mut y = top + 8.0;
        self.draw_text("DRAFTS", 32.0, y + 2.0, 9.0, p.dim);
        y += 18.0;
        // File name directly below Draft — green dot + editable
        self.round(8.0, y - 2.0, 4.0, 4.0, p.draft_dot, 2.0);
        self.draw_text("Liquor Delivery App UI", 20.0, y, 11.0, p.text);
        y += 22.0;
        // Pill tabs LAYERS/ASSETS/TOKENS active #222222 border #2A2A2A
        self.round(8.0, y, lw - 16.0, 30.0, p.bg, 8.0);
        self.stroke_round(8.0, y, lw - 16.0, 30.0, p.line, 1.0, 8.0);
        let tab_w = (lw - 20.0) / 3.0;
        self.round(10.0, y + 2.0, tab_w, 26.0, p.field2, 6.0);
        self.stroke_round(10.0, y + 2.0, tab_w, 26.0, p.line2, 1.0, 6.0);
        self.draw_text("LAYERS", 18.0, y + 9.0, 9.0, p.text);
        self.draw_text("ASSETS", 18.0 + tab_w, y + 9.0, 9.0, p.dim);
        self.draw_text("TOKENS", 18.0 + tab_w * 2.0, y + 9.0, 9.0, p.dim);
        y += 40.0;
        self.draw_text("PAGES", 12.0, y, 9.0, p.dim);
        y += 18.0;
        self.round(8.0, y, lw - 16.0, 28.0, p.field, 6.0);
        self.stroke_round(8.0, y, lw - 16.0, 28.0, p.line2, 1.0, 6.0);
        self.draw_text("Page 3", 32.0, y + 8.0, 11.0, p.text);
        y += 36.0;
        self.line(0.0, y, lw, y, p.line, 1.0);
        y += 8.0;
        self.draw_text("PAGE 3", 12.0, y + 2.0, 9.0, p.dim);
        y += 20.0;
        let items = [
            ("Board", 0),
            ("order-details", 0),
            ("Rectangle 12", 0),
            ("payment-methods", 0),
            ("pay-row", 1),
            ("section-header", 2),
            ("Ellipse 3", 2),
            ("Vector", 2),
        ];
        for (name, indent) in items.iter() {
            if y > top + h - 20.0 { break; }
            let ix = 8.0 + *indent as f64 * 16.0;
            self.draw_text(name, ix + 30.0, y + 6.0, 11.0, p.muted);
            y += 22.0;
        }
    }

    // === RIGHT PANEL — FINAL v45 — Design/Prototype/Inspect unified, Size+Position Frame, Auto Layout 84x84 dark, Guides no dropdown, Export collapsed ===
    fn right_panel_native(&mut self, p: Palette) {
        let rw = self.right_w_val();
        let rx = self.config.width as f64 - rw;
        let top = self.top_h();
        let h = self.config.height as f64 - top - 24.0;
        self.rect(rx, top, rw, h, p.panel, 0.0);
        self.line(rx + 0.5, top, rx + 0.5, top + h, p.line, 1.0);
        let mut y = top + 6.0;
        // Avatar S + 12%
        self.round(rx + 8.0, y, 24.0, 24.0, p.avatar, 12.0);
        self.draw_text("S", rx + 16.0, y + 5.0, 10.0, Color::BLACK);
        self.draw_text("12%", rx + 40.0, y + 6.0, 10.0, p.muted);
        y += 32.0;
        // Pill tabs DESIGN/PROTOTYPE/INSPECT — no border between profile and tabs
        self.round(rx + 8.0, y, rw - 16.0, 30.0, p.bg, 8.0);
        self.stroke_round(rx + 8.0, y, rw - 16.0, 30.0, p.line, 1.0, 8.0);
        let tab_w = (rw - 20.0) / 3.0;
        self.round(rx + 10.0, y + 2.0, tab_w, 26.0, p.field2, 6.0);
        self.stroke_round(rx + 10.0, y + 2.0, tab_w, 26.0, p.line2, 1.0, 6.0);
        self.draw_text("DESIGN", rx + 18.0, y + 9.0, 9.0, p.text);
        self.draw_text("PROTOTYPE", rx + 14.0 + tab_w, y + 9.0, 9.0, p.dim);
        self.draw_text("INSPECT", rx + 22.0 + tab_w * 2.0, y + 9.0, 9.0, p.dim);
        y += 38.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Size+Position combined — no heading, Frame replaces Normal
        self.round(rx + 12.0, y, 108.0, 28.0, p.field, 8.0);
        self.stroke_round(rx + 12.0, y, 108.0, 28.0, p.line, 1.0, 8.0);
        self.draw_text("Frame", rx + 22.0, y + 8.0, 11.0, p.text);
        self.round(rx + 128.0, y, 68.0, 28.0, p.field, 8.0);
        self.stroke_round(rx + 128.0, y, 68.0, 28.0, p.line, 1.0, 8.0);
        self.draw_text("100%", rx + 144.0, y + 8.0, 11.0, p.text);
        y += 36.0;
        self.round(rx + 12.0, y, rw * 0.5 - 16.0, 28.0, p.field, 8.0);
        self.draw_text("W 375", rx + 20.0, y + 8.0, 10.0, p.text);
        self.round(rx + rw * 0.5 + 4.0, y, rw * 0.5 - 28.0, 28.0, p.field, 8.0);
        self.draw_text("H 420", rx + rw * 0.5 + 12.0, y + 8.0, 10.0, p.text);
        y += 36.0;
        self.round(rx + 12.0, y, rw * 0.5 - 16.0, 28.0, p.field, 8.0);
        self.draw_text("X 0", rx + 20.0, y + 8.0, 10.0, p.text);
        self.round(rx + rw * 0.5 + 4.0, y, rw * 0.5 - 16.0, 28.0, p.field, 8.0);
        self.draw_text("Y 60", rx + rw * 0.5 + 12.0, y + 8.0, 10.0, p.text);
        y += 36.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Auto Layout — smaller 84x84 dark, no 09, + dark icon
        self.draw_text("Auto layout", rx + 12.0, y + 2.0, 11.0, p.text);
        self.round(rx + rw - 28.0, y, 20.0, 20.0, p.field, 6.0);
        self.stroke_round(rx + rw - 28.0, y, 20.0, 20.0, p.line, 1.0, 6.0);
        self.draw_text("+", rx + rw - 22.0, y + 2.0, 12.0, p.dim);
        y += 24.0;
        self.draw_text("Flow", rx + 12.0, y, 9.0, p.dim);
        y += 14.0;
        let fw = (rw - 24.0 - 9.0) / 4.0;
        for i in 0..4 {
            let fx = rx + 12.0 + i as f64 * (fw + 3.0);
            let active = i == 0 || i == 2;
            self.round(fx, y, fw, 28.0, if active { p.field2 } else { p.field }, 8.0);
            self.stroke_round(fx, y, fw, 28.0, if active { p.line2 } else { p.line }, 1.0, 8.0);
        }
        y += 36.0;
        self.draw_text("Alignment", rx + 12.0, y, 9.0, p.dim);
        self.draw_text("Gap", rx + 12.0 + 96.0, y, 9.0, p.dim);
        y += 14.0;
        // Alignment 84x84 dark card #1A1A1A border #1F1F1F rounded 12 cross #2A2A2A dots #777777 active white halo
        self.round(rx + 12.0, y, 84.0, 84.0, p.field, 12.0);
        self.stroke_round(rx + 12.0, y, 84.0, 84.0, p.line, 1.0, 12.0);
        self.line(rx + 24.0, y + 42.0, rx + 84.0, y + 42.0, p.line2, 1.0);
        self.line(rx + 54.0, y + 12.0, rx + 54.0, y + 72.0, p.line2, 1.0);
        self.round(rx + 96.0, y, rw - 120.0, 28.0, p.field, 8.0);
        self.stroke_round(rx + 96.0, y, rw - 120.0, 28.0, p.line, 1.0, 8.0);
        self.draw_text("Gap 5", rx + 104.0, y + 8.0, 10.0, p.text);
        y += 92.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Appearance
        self.draw_text("Appearance", rx + 12.0, y + 2.0, 10.0, p.text);
        y += 20.0;
        self.round(rx + 12.0, y, rw * 0.5 - 16.0, 28.0, p.field, 8.0);
        self.draw_text("Opacity 100%", rx + 20.0, y + 8.0, 10.0, p.text);
        self.round(rx + rw * 0.5 + 4.0, y, rw * 0.5 - 16.0, 28.0, p.field, 8.0);
        self.draw_text("Radius 0", rx + rw * 0.5 + 12.0, y + 8.0, 10.0, p.text);
        y += 36.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Typography full
        self.draw_text("Typography", rx + 12.0, y + 2.0, 10.0, p.text);
        y += 20.0;
        self.round(rx + 12.0, y, rw - 24.0, 28.0, p.field, 8.0);
        self.draw_text("Manrope Regular 14", rx + 22.0, y + 8.0, 11.0, p.text);
        y += 36.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Fill above Stroke
        self.draw_text("Fill", rx + 12.0, y + 2.0, 10.0, p.text);
        y += 20.0;
        self.round(rx + 12.0, y, rw - 24.0, 28.0, p.field, 8.0);
        self.round(rx + 18.0, y + 6.0, 16.0, 16.0, Color::WHITE, 4.0);
        self.draw_text("FFFFFF 100%", rx + 40.0, y + 8.0, 11.0, p.text);
        y += 36.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        self.draw_text("Stroke", rx + 12.0, y + 2.0, 10.0, p.text);
        y += 20.0;
        self.round(rx + 12.0, y, rw - 24.0, 28.0, p.field, 8.0);
        self.round(rx + 18.0, y + 6.0, 16.0, 16.0, Color::BLACK, 4.0);
        self.draw_text("000000 Outside 1", rx + 40.0, y + 8.0, 10.0, p.text);
        y += 36.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Guides — no dropdown icon, Square 16 dark style
        self.draw_text("Guides", rx + 12.0, y + 2.0, 10.0, p.text);
        self.round(rx + rw - 28.0, y, 20.0, 20.0, p.field, 6.0);
        self.stroke_round(rx + rw - 28.0, y, 20.0, 20.0, p.line, 1.0, 6.0);
        y += 24.0;
        self.round(rx + 12.0, y, rw - 24.0, 28.0, p.field, 8.0);
        self.stroke_round(rx + 12.0, y, rw - 24.0, 28.0, p.line, 1.0, 8.0);
        self.draw_text("Square 16", rx + 40.0, y + 8.0, 10.0, p.text);
        y += 36.0;
        self.line(rx, y, rx + rw, y, p.line, 1.0);
        y += 8.0;
        // Export — styled like other tabs, collapsed + toggles
        self.draw_text("Export", rx + 12.0, y + 2.0, 10.0, p.text);
        self.round(rx + rw - 28.0, y, 20.0, 20.0, p.field, 6.0);
        self.stroke_round(rx + rw - 28.0, y, 20.0, 20.0, p.line, 1.0, 6.0);
        self.draw_text("+", rx + rw - 22.0, y + 2.0, 12.0, p.dim);
        y += 24.0;
        if self.export_expanded {
            self.round(rx + 12.0, y, 56.0, 24.0, p.field, 6.0);
            self.draw_text("PNG", rx + 22.0, y + 6.0, 10.0, p.text);
            self.round(rx + 76.0, y, 44.0, 24.0, p.field, 6.0);
            self.draw_text("1x", rx + 86.0, y + 6.0, 10.0, p.text);
            self.round(rx + 128.0, y, rw - 152.0, 24.0, p.field, 6.0);
            self.draw_text("Suffix", rx + 136.0, y + 6.0, 10.0, p.dim);
            y += 32.0;
            self.round(rx + 12.0, y, rw - 24.0, 28.0, p.field2, 6.0);
            self.stroke_round(rx + 12.0, y, rw - 24.0, 28.0, p.line2, 1.0, 6.0);
            self.draw_text("EXPORT 1 ELEMENT", rx + rw * 0.5 - 44.0, y + 8.0, 9.0, p.muted);
        } else {
            self.draw_text("No exports — click + to add", rx + 12.0, y + 4.0, 10.0, p.faint);
        }
    }

    fn bottom_toolbar_native(&mut self, p: Palette) {
        let w = self.config.width as f64;
        let bar_w = 260.0;
        let bar_h = 36.0;
        let bar_x = (w - bar_w) * 0.5;
        let bar_y = self.config.height as f64 - 50.0;
        self.round(bar_x, bar_y, bar_w, bar_h, Color::from_rgba8(0x1A, 0x1A, 0x1A, 0xDD), 12.0);
        self.stroke_round(bar_x, bar_y, bar_w, bar_h, p.line2, 1.0, 12.0);
        for i in 0..6 {
            let bx = bar_x + 6.0 + i as f64 * 36.0;
            if i == self.active_tool {
                self.round(bx, bar_y + 4.0, 28.0, 28.0, p.text, 7.0);
            }
        }
    }

    fn status_bar_native(&mut self, p: Palette) {
        let y = self.config.height as f64 - 24.0;
        let w = self.config.width as f64;
        self.rect(0.0, y, w, 24.0, p.panel, 0.0);
        self.line(0.0, y, w, y, p.line, 1.0);
        self.draw_text("Ready — ⌘K commands", 12.0, y + 6.0, 10.0, p.dim);
    }

    fn command_palette(&mut self, p: Palette) {
        let w = self.config.width as f64;
        let h = self.config.height as f64;
        self.rect(0.0, 0.0, w, h, Color::from_rgba8(0, 0, 0, 120), 0.0);
        let pw = 440.0;
        let ph = 280.0;
        let x = (w - pw) * 0.5;
        let y = h * 0.18;
        self.round(x, y, pw, ph, Color::from_rgb8(0x1E, 0x1E, 0x1E), 10.0);
        self.stroke_round(x, y, pw, ph, p.line2, 1.0, 10.0);
        self.round(x + 12.0, y + 12.0, pw - 24.0, 32.0, p.field, 8.0);
        self.draw_text("Type a command…", x + 24.0, y + 22.0, 11.0, p.faint);
    }

    // Helpers
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, _r: f64) {
        self.scene
            .fill(vello::peniko::Fill::NonZero, &Affine::IDENTITY, c, None, &Rect::new(x, y, x + w, y + h));
    }
    fn round(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, r: f64) {
        let rr = RoundedRect::new(x, y, x + w, y + h, r);
        self.scene
            .fill(vello::peniko::Fill::NonZero, &Affine::IDENTITY, c, None, &rr);
    }
    fn stroke_round(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64, r: f64) {
        let rr = RoundedRect::new(x, y, x + w, y + h, r);
        self.scene
            .stroke(&vello::peniko::Stroke::new(width), &Affine::IDENTITY, c, None, &rr);
    }
    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, c: Color, width: f64) {
        let mut path = BezPath::new();
        path.move_to(Point::new(x1, y1));
        path.line_to(Point::new(x2, y2));
        self.scene
            .stroke(&vello::peniko::Stroke::new(width), &Affine::IDENTITY, c, None, &path);
    }
    fn draw_text(&mut self, text: &str, x: f64, y: f64, size: f64, c: Color) {
        let spans = [Span::new(text, size).color(c).font(self.font)];
        let style = TextBlockStyle {
            max_width: 1000.0,
            line_height: 1.2,
            align: Align::Left,
            wrap: x_native::TextWrap::NoWrap,
        };
        let _ = encode_rich_text(
            &mut self.scene,
            &self.fonts,
            &spans,
            self.font,
            Affine::translate((x, y)),
            &style,
        );
    }
}
