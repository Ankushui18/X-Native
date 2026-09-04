//! Window + input loop. UI is new; editor APIs from x-native.

use std::sync::Arc;

use vello::kurbo::{Affine, Point, Rect};
use vello::peniko::Color;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use crate::shell::{paint_shell, regions};
use crate::state::{AppState, Screen, Tool};
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

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            window.clone(),
        ));
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

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
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
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.on_click(self.cursor);
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
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.on_key(logical_key);
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

impl Host {
    fn on_key(&mut self, key: Key) {
        let cmd = self.modifiers.super_key() || self.modifiers.control_key();
        match key {
            Key::Character(c) if cmd && (c == "k" || c == "K") => {
                if self.app.screen == Screen::Editor {
                    self.app.command_open = !self.app.command_open;
                    self.app.command_query.clear();
                }
            }
            Key::Named(NamedKey::Escape) => {
                self.app.command_open = false;
            }
            Key::Named(NamedKey::Space) if self.app.screen == Screen::Editor => {
                self.app.space_pan = true;
                self.app.tool = Tool::Hand;
            }
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
                    _ => {}
                }
                self.app.status = format!("Tool: {}", self.app.tool.label());
            }
            _ => {}
        }
    }

    fn on_click(&mut self, p: Point) {
        if self.app.screen == Screen::Home {
            // Any click on home primary options area → blank editor
            self.app.open_editor_blank();
            return;
        }
        if self.app.command_open {
            self.app.command_open = false;
            return;
        }
        let r = regions(self.app.win_w, self.app.win_h);
        // Tools
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
        // Left: new page / switch page
        if r.left.contains(p) && self.app.left_tab == crate::state::LeftTab::Layers {
            let pages_y0 = TITLE_H + 64.0;
            let pages_end = pages_y0 + self.app.pages.len() as f64 * ROW_H;
            if p.y >= pages_y0 && p.y < pages_end {
                let idx = ((p.y - pages_y0) / ROW_H) as usize;
                // Trash zone
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
        }
        // Canvas: clear selection on empty click (page root not selectable)
        if r.canvas.contains(p) {
            let world = Point::new(
                (p.x - r.canvas.x0 - self.app.pan.0) / self.app.zoom,
                (p.y - r.canvas.y0 - self.app.pan.1) / self.app.zoom,
            );
            self.app.editor.click(world, false);
            // Never keep page root selected
            self.app
                .editor
                .selection
                .retain(|id| id != &self.app.editor.root.id);
            if self.app.tool == Tool::Frame {
                // Place a starter white frame at click
                let id = format!("frame-{}", self.app.editor.root.children.len() + 1);
                let mut f = x_native::Node::frame(&id, 400.0, 300.0);
                f.transform.x = world.x;
                f.transform.y = world.y;
                f.fill = x_native::Paint::Solid(x_native::Color::WHITE);
                f.name = "Frame".into();
                let root = self.app.editor.root.id.clone();
                self.app.editor.insert_node(&root, f);
                self.app.dirty = true;
                self.app.status = "Created frame".into();
                self.app.tool = Tool::Select;
            }
        }
    }

    fn redraw(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let mut scene = Scene::new();
        paint_shell(&mut scene, &self.app);

        // Composite document content into canvas region
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
        }

        let surface_tex = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let width = gpu.config.width;
        let height = gpu.config.height;
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
        // Present
        surface_tex.present();
    }
}
