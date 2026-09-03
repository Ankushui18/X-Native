//! Window + input loop. Phase 1–2: create tools, pan, select, delete, undo.

use std::sync::Arc;

use vello::kurbo::{Affine, Point};
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use crate::shell::{paint_shell, regions};
use crate::state::{AppState, Drag, Handle, Screen, Tool};
use crate::theme::*;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

struct Host {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    app: AppState,
    modifiers: ModifiersState,
    cursor: Point,
    mouse_down: bool,
}

pub fn run() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut host = Host {
        window: None,
        gpu: None,
        app: AppState::new_blank(),
        modifiers: ModifiersState::default(),
        cursor: Point::ZERO,
        mouse_down: false,
    };
    let _ = event_loop.run_app(&mut host);
}

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("X-Native")
            .with_inner_size(LogicalSize::new(1440.0, 900.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();
        self.app.win_w = size.width.max(1) as f64;
        self.app.win_h = size.height.max(1) as f64;

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("x-native"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .expect("device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: std::num::NonZeroUsize::new(1),
                ..Default::default()
            },
        )
        .expect("vello renderer");

        self.gpu = Some(Gpu {
            device,
            queue,
            renderer,
            surface,
            config,
        });
        self.window = Some(window);
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.app.win_w = size.width.max(1) as f64;
                self.app.win_h = size.height.max(1) as f64;
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.config.width = size.width.max(1);
                    gpu.config.height = size.height.max(1);
                    gpu.surface.configure(&gpu.device, &gpu.config);
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => window.request_redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point::new(position.x, position.y);
                self.on_move(self.cursor);
                if self.mouse_down {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        self.mouse_down = true;
                        self.on_press(self.cursor);
                    }
                    ElementState::Released => {
                        self.mouse_down = false;
                        self.on_release(self.cursor);
                    }
                }
                window.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 80.0,
                };
                if self.app.screen == Screen::Editor {
                    let factor = if dy > 0.0 { 1.1 } else { 0.9 };
                    self.app.zoom = (self.app.zoom * factor).clamp(0.05, 8.0);
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state,
                        ..
                    },
                ..
            } => {
                if state == ElementState::Released {
                    if let Key::Named(NamedKey::Space) = logical_key {
                        self.app.space_pan = false;
                        if self.app.tool == Tool::Hand {
                            // keep hand if explicitly chosen
                        }
                    }
                }
                if state == ElementState::Pressed {
                    self.on_key(logical_key);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

impl Host {
    fn world_point(&self, p: Point) -> Point {
        let r = regions(self.app.win_w, self.app.win_h);
        Point::new(
            (p.x - r.canvas.x0 - self.app.pan.0) / self.app.zoom,
            (p.y - r.canvas.y0 - self.app.pan.1) / self.app.zoom,
        )
    }

    fn handle_at(&self, p: Point) -> Option<(String, Handle)> {
        let r = regions(self.app.win_w, self.app.win_h);
        if self.app.editor.selection.len() != 1 {
            return None;
        }
        let id = self.app.editor.selection[0].clone();
        if id == self.app.editor.root.id {
            return None;
        }
        let n = x_native::editor::find(&self.app.editor.root, &id)?;
        let sx = r.canvas.x0 + self.app.pan.0 + n.transform.x * self.app.zoom;
        let sy = r.canvas.y0 + self.app.pan.1 + n.transform.y * self.app.zoom;
        let sw = n.w * self.app.zoom;
        let sh = n.h * self.app.zoom;
        let hs = 8.0; // hit size
        let corners = [
            (Handle::Nw, sx, sy),
            (Handle::Ne, sx + sw, sy),
            (Handle::Sw, sx, sy + sh),
            (Handle::Se, sx + sw, sy + sh),
        ];
        for (h, hx, hy) in corners {
            if (p.x - hx).abs() <= hs && (p.y - hy).abs() <= hs {
                return Some((id, h));
            }
        }
        None
    }


    fn on_key(&mut self, key: Key) {
        let cmd = self.modifiers.super_key() || self.modifiers.control_key();
        match key {
            Key::Character(c) if cmd && (c.as_str() == "k" || c.as_str() == "K") => {
                if self.app.screen == Screen::Editor {
                    self.app.command_open = !self.app.command_open;
                    self.app.command_query.clear();
                }
            }
            Key::Character(c) if cmd && (c.as_str() == "z" || c.as_str() == "Z") => {
                if self.app.editor.undo() {
                    self.app.status = "Undo".into();
                    self.app.dirty = true;
                }
            }
            Key::Named(NamedKey::Escape) => {
                self.app.command_open = false;
                self.app.editor.selection.clear();
                self.app.create_preview = None;
                self.app.drag = Drag::None;
            }
            Key::Named(NamedKey::Space) if self.app.screen == Screen::Editor => {
                self.app.space_pan = true;
            }
            Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace)
                if self.app.screen == Screen::Editor && !self.app.command_open =>
            {
                self.app.editor.delete_selection();
                self.app.status = "Deleted".into();
                self.app.dirty = true;
            }
            Key::Named(NamedKey::ArrowLeft) => self.app.editor.move_selection(-1.0, 0.0),
            Key::Named(NamedKey::ArrowRight) => self.app.editor.move_selection(1.0, 0.0),
            Key::Named(NamedKey::ArrowUp) => self.app.editor.move_selection(0.0, -1.0),
            Key::Named(NamedKey::ArrowDown) => self.app.editor.move_selection(0.0, 1.0),
            Key::Character(c) if self.app.screen == Screen::Editor && !self.app.command_open => {
                match c.as_str() {
                    "v" | "V" => self.app.tool = Tool::Select,
                    "f" | "F" => self.app.tool = Tool::Frame,
                    "r" | "R" => self.app.tool = Tool::Rectangle,
                    "o" | "O" => self.app.tool = Tool::Ellipse,
                    "l" | "L" => self.app.tool = Tool::Line,
                    "p" | "P" => self.app.tool = Tool::Pen,
                    "t" | "T" => self.app.tool = Tool::Text,
                    "h" | "H" => self.app.tool = Tool::Hand,
                    _ => return,
                }
                self.app.status = format!("Tool: {}", self.app.tool.label());
            }
            _ => {}
        }
    }

    fn on_press(&mut self, p: Point) {
        if self.app.screen == Screen::Home {
            self.app.open_editor_blank();
            return;
        }
        if self.app.command_open {
            self.app.command_open = false;
            return;
        }
        let r = regions(self.app.win_w, self.app.win_h);

        if r.tools.contains(p) {
            let tools = [
                Tool::Select,
                Tool::Frame,
                Tool::Rectangle,
                Tool::Ellipse,
                Tool::Line,
                Tool::Pen,
                Tool::Text,
                Tool::Hand,
            ];
            let idx = ((p.y - TITLE_H - 12.0) / 40.0).floor() as isize;
            if idx >= 0 && (idx as usize) < tools.len() {
                self.app.tool = tools[idx as usize];
                self.app.status = format!("Tool: {}", self.app.tool.label());
            }
            return;
        }

        if r.left.contains(p) && self.app.left_tab == crate::state::LeftTab::Layers {
            let pages_y0 = TITLE_H + 64.0;
            let pages_end = pages_y0 + self.app.pages.len() as f64 * ROW_H;
            if p.y >= pages_y0 && p.y < pages_end {
                let idx = ((p.y - pages_y0) / ROW_H) as usize;
                if self.app.pages.len() > 1 && p.x >= TOOL_W + LEFT_W - 28.0 {
                    self.app.delete_page(idx);
                } else {
                    self.app.switch_page(idx);
                }
                return;
            }
            if p.y >= pages_end && p.y < pages_end + ROW_H {
                self.app.add_page();
                return;
            }
            // Layer row select
            let layers_y = pages_end + 28.0 + 28.0;
            let rows = self.app.layer_rows();
            if p.y >= layers_y {
                let idx = ((p.y - layers_y) / ROW_H).floor() as isize;
                if idx >= 0 && (idx as usize) < rows.len() {
                    self.app.editor.selection = vec![rows[idx as usize].0.clone()];
                    self.app.status = format!("Selected {}", rows[idx as usize].1);
                }
            }
            return;
        }

        if r.canvas.contains(p) {
            if self.app.tool == Tool::Hand || self.app.space_pan {
                self.app.drag = Drag::Pan {
                    last: (p.x, p.y),
                };
                return;
            }
            if self.app.tool.is_create() {
                let w = self.world_point(p);
                self.app.drag = Drag::Create {
                    start: (w.x, w.y),
                };
                self.app.create_preview = Some((w.x, w.y, w.x, w.y));
                return;
            }
            // Resize handle hit first
            if let Some((id, handle)) = self.handle_at(p) {
                if let Some(n) = x_native::editor::find(&self.app.editor.root, &id) {
                    self.app.drag = Drag::Resize {
                        id,
                        handle,
                        origin: (n.transform.x, n.transform.y, n.w, n.h),
                    };
                    self.app.status = "Resize".into();
                    return;
                }
            }
            // Select
            let world = self.world_point(p);
            let shift = self.modifiers.shift_key();
            self.app.editor.click_select(world, shift, false);
            self.app
                .editor
                .selection
                .retain(|id| id != &self.app.editor.root.id);
            if !self.app.editor.selection.is_empty() {
                self.app.drag = Drag::Move {
                    last: (world.x, world.y),
                };
                self.app.status = format!("{} selected", self.app.editor.selection.len());
            } else {
                self.app.status = "No selection".into();
            }
        }
    }

    fn on_move(&mut self, p: Point) {
        match &self.app.drag {
            Drag::Pan { last } => {
                let dx = p.x - last.0;
                let dy = p.y - last.1;
                self.app.pan.0 += dx;
                self.app.pan.1 += dy;
                self.app.drag = Drag::Pan {
                    last: (p.x, p.y),
                };
            }
            Drag::Create { start } => {
                let w = self.world_point(p);
                self.app.create_preview = Some((start.0, start.1, w.x, w.y));
            }
            Drag::Move { last } => {
                let w = self.world_point(p);
                let dx = w.x - last.0;
                let dy = w.y - last.1;
                if dx.abs() > 0.01 || dy.abs() > 0.01 {
                    self.app.editor.move_selection(dx, dy);
                    self.app.dirty = true;
                    self.app.drag = Drag::Move {
                        last: (w.x, w.y),
                    };
                }
            }
            Drag::Resize { id, handle, origin } => {
                let wpt = self.world_point(p);
                let (ox, oy, ow, oh) = *origin;
                let id = id.clone();
                let handle = *handle;
                let (nx, ny, nw, nh) = match handle {
                    Handle::Se => {
                        (ox, oy, (wpt.x - ox).max(1.0), (wpt.y - oy).max(1.0))
                    }
                    Handle::Sw => {
                        let x1 = ox + ow;
                        let nw = (x1 - wpt.x).max(1.0);
                        (x1 - nw, oy, nw, (wpt.y - oy).max(1.0))
                    }
                    Handle::Ne => {
                        let y1 = oy + oh;
                        let nh = (y1 - wpt.y).max(1.0);
                        (ox, y1 - nh, (wpt.x - ox).max(1.0), nh)
                    }
                    Handle::Nw => {
                        let x1 = ox + ow;
                        let y1 = oy + oh;
                        let nw = (x1 - wpt.x).max(1.0);
                        let nh = (y1 - wpt.y).max(1.0);
                        (x1 - nw, y1 - nh, nw, nh)
                    }
                };
                // Apply: move delta + resize
                if let Some(n) = x_native::editor::find(&self.app.editor.root, &id) {
                    let dx = nx - n.transform.x;
                    let dy = ny - n.transform.y;
                    if dx.abs() > 0.01 || dy.abs() > 0.01 {
                        self.app.editor.move_selection(dx, dy);
                    }
                    self.app.editor.resize(&id, nw, nh);
                    self.app.dirty = true;
                }
            }
            Drag::None => {}
        }
    }

    fn on_release(&mut self, p: Point) {
        match self.app.drag.clone() {
            Drag::Create { start } => {
                let w = self.world_point(p);
                self.app.finish_create(start.0, start.1, w.x, w.y);
            }
            _ => {}
        }
        self.app.drag = Drag::None;
        self.app.create_preview = None;
    }

    fn redraw(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let mut scene = Scene::new();
        paint_shell(&mut scene, &self.app);

        if self.app.screen == Screen::Editor {
            let r = regions(self.app.win_w, self.app.win_h);
            let (doc_scene, _) =
                x_native::build_scene(&self.app.editor.root, None, &self.app.vars);
            let ox = r.canvas.x0 + self.app.pan.0;
            let oy = r.canvas.y0 + self.app.pan.1;
            scene.append(
                &doc_scene,
                Some(Affine::translate((ox, oy)).then_scale(self.app.zoom)),
            );
            // Re-paint selection overlays above document
            // (shell already drew outlines; document may cover them — redraw chrome selection on top via shell is under content)
            // Document is appended after shell canvas fill; selection was in shell before doc.
            // Re-draw selection on top:
            for id in &self.app.editor.selection {
                if id == &self.app.editor.root.id {
                    continue;
                }
                if let Some(n) = x_native::editor::find(&self.app.editor.root, id) {
                    let sx = r.canvas.x0 + self.app.pan.0 + n.transform.x * self.app.zoom;
                    let sy = r.canvas.y0 + self.app.pan.1 + n.transform.y * self.app.zoom;
                    let sw = n.w * self.app.zoom;
                    let sh = n.h * self.app.zoom;
                    crate::paint::stroke_rect(
                        &mut scene,
                        vello::kurbo::Rect::new(sx, sy, sx + sw, sy + sh),
                        C_ACCENT,
                        1.5,
                    );
                }
            }
            if let Some((x0, y0, x1, y1)) = self.app.create_preview {
                let sx0 = r.canvas.x0 + self.app.pan.0 + x0.min(x1) * self.app.zoom;
                let sy0 = r.canvas.y0 + self.app.pan.1 + y0.min(y1) * self.app.zoom;
                let sx1 = r.canvas.x0 + self.app.pan.0 + x0.max(x1) * self.app.zoom;
                let sy1 = r.canvas.y0 + self.app.pan.1 + y0.max(y1) * self.app.zoom;
                crate::paint::stroke_rect(
                    &mut scene,
                    vello::kurbo::Rect::new(sx0, sy0, sx1, sy1),
                    C_ACCENT,
                    1.5,
                );
            }
        }

        let surface_tex = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("surface: {e}");
                return;
            }
        };
        let view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let width = gpu.config.width;
        let height = gpu.config.height;
        // Note: render_to_texture expects a storage texture in some vello versions;
        // if present path fails, surface format may need STORAGE_BINDING — platform dependent.
        let _ = gpu.renderer.render_to_texture(
            &gpu.device,
            &gpu.queue,
            &scene,
            &view,
            &RenderParams {
                base_color: C_BASE,
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        );
        surface_tex.present();
    }
}
