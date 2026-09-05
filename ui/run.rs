//! Window + input loop — Native Exact Match — DESIGN/PROTOTYPE/INSPECT tabs — HTML source of truth
//! Left: DRAFTS + LAYERS/ASSETS/TOKENS + PAGES + PAGE 3 tree
//! Right: DESIGN shows Normal W H X Y LAYOUT CONSTRAINTS TEXT FILL STROKE SHADOW BLUR EXPORT
//! v22 colors preserved

use std::sync::Arc;

use vello::kurbo::{Affine, Point};
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use crate::shell::{paint_shell, regions_for};
use crate::state::{AppState, InspectorAction, InspectorField, InspectorTab, LeftTab, Screen, Tool};
use crate::theme::*;
use x_native as x_core;
use x_native::editor as x_editor;

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
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes()
            .with_title("X-Native")
            .with_inner_size(LogicalSize::new(1440.0, 900.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();
        self.app.win_w = size.width.max(1) as f64;
        self.app.win_h = size.height.max(1) as f64;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(window.clone())));
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).expect("adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("x-native"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        )).expect("device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
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
        ).expect("vello renderer");

        self.gpu = Some(Gpu { device, queue, renderer, surface, config });
        self.window = Some(window);
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else { return; };
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
                let p = Point::new(position.x, position.y);
                self.cursor = p;
                // panel resize dragging
                if self.app.resizing_left {
                    let new_w = p.x.clamp(200.0, 480.0);
                    self.app.left_w = new_w;
                    window.request_redraw();
                } else if self.app.resizing_right {
                    let new_rw = (self.app.win_w - p.x).clamp(240.0, 480.0);
                    self.app.right_w = new_rw;
                    window.request_redraw();
                } else {
                    // hover resize cursor hint — show resize arrows when near edge (handled via status)
                    let left_edge = self.app.left_w;
                    let right_edge = self.app.win_w - self.app.right_w;
                    if (p.x - left_edge).abs() < 6.0 && p.y > TITLE_H && p.y < self.app.win_h - STATUS_H {
                        self.app.status = "↔ Resize left panel — drag to 200-480".into();
                    } else if (p.x - right_edge).abs() < 6.0 && p.y > TITLE_H && p.y < self.app.win_h - STATUS_H {
                        self.app.status = "↔ Resize right panel — drag to 240-480".into();
                    }
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => {
                        // check resize handles first
                        let left_edge = self.app.left_w;
                        let right_edge = self.app.win_w - self.app.right_w;
                        if (self.cursor.x - left_edge).abs() < 6.0 && self.cursor.y > TITLE_H && self.cursor.y < self.app.win_h - STATUS_H {
                            self.app.resizing_left = true;
                            self.app.status = "Resizing left panel…".into();
                        } else if (self.cursor.x - right_edge).abs() < 6.0 && self.cursor.y > TITLE_H && self.cursor.y < self.app.win_h - STATUS_H {
                            self.app.resizing_right = true;
                            self.app.status = "Resizing right panel…".into();
                        } else {
                            self.on_click(self.cursor);
                        }
                        window.request_redraw();
                    }
                    ElementState::Released => {
                        if self.app.resizing_left || self.app.resizing_right {
                            self.app.resizing_left = false;
                            self.app.resizing_right = false;
                            self.app.status = format!("Panels: left {:.0}px right {:.0}px", self.app.left_w, self.app.right_w);
                            window.request_redraw();
                        }
                    }
                }
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
            WindowEvent::ModifiersChanged(m) => { self.modifiers = m.state(); }
            WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state: ElementState::Pressed, .. }, .. } => {
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
        if let Some(edit) = self.app.inspector_edit.as_mut() {
            match key {
                Key::Named(NamedKey::Enter) => {
                    let field = edit.field;
                    let buf = edit.buffer.clone();
                    self.app.inspector_edit = None;
                    self.apply_inspector_field(field, buf);
                    return;
                }
                Key::Named(NamedKey::Escape) => {
                    self.app.inspector_edit = None;
                    self.app.status = "Edit canceled".into();
                    return;
                }
                Key::Named(NamedKey::Backspace) => { edit.buffer.pop(); return; }
                Key::Character(c) => {
                    if edit.field == InspectorField::TextContent || edit.field == InspectorField::ComponentName || edit.field == InspectorField::InstanceSwap || edit.field == InspectorField::PrototypeDest || edit.field == InspectorField::DocName {
                        edit.buffer.push_str(&c);
                    } else if c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '#' || ch == ' ') {
                        edit.buffer.push_str(&c);
                    }
                    return;
                }
                _ => {}
            }
        }
        match key {
            Key::Character(c) if cmd && (c == "k" || c == "K") => {
                if self.app.screen == Screen::Editor {
                    self.app.command_open = !self.app.command_open;
                    self.app.command_query.clear();
                }
            }
            Key::Named(NamedKey::Escape) => { self.app.command_open = false; }
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
                    "g" | "G" if cmd => { self.app.editor.group_selection("group-1"); self.app.status = "Grouped".into(); self.app.dirty = true; },
                    "z" | "Z" if cmd => { self.app.editor.undo(); self.app.status = "Undo".into(); },
                    "y" | "Y" if cmd => { self.app.editor.redo(); self.app.status = "Redo".into(); },
                    _ => {}
                }
                if !cmd { self.app.status = format!("Tool: {}", self.app.tool.label()); }
            }
            _ => {}
        }
    }

    fn apply_inspector_field(&mut self, field: InspectorField, buf: String) {
        // DocName doesn't need selection
        if field == InspectorField::DocName {
            self.app.doc_name = if buf.trim().is_empty() { "Untitled".into() } else { buf.clone() };
            self.app.status = format!("Renamed to {}", self.app.doc_name);
            self.app.dirty = true;
            return;
        }
        let Some(sel_id) = self.app.editor.selection.first().cloned() else { return; };
        let node_opt = x_native::editor::find(&self.app.editor.root, &sel_id).cloned();
        let Some(node) = node_opt else { return; };
        match field {
            InspectorField::X => if let Ok(new_x) = buf.parse::<f64>() { let dx = new_x - node.transform.x; self.app.editor.move_selection(dx, 0.0); self.app.status = format!("X -> {}", new_x); self.app.dirty = true; },
            InspectorField::Y => if let Ok(new_y) = buf.parse::<f64>() { let dy = new_y - node.transform.y; self.app.editor.move_selection(0.0, dy); self.app.status = format!("Y -> {}", new_y); self.app.dirty = true; },
            InspectorField::W => if let Ok(new_w) = buf.parse::<f64>() { if new_w>0.0 { self.app.editor.resize(&sel_id, new_w, node.h); self.app.status = format!("W -> {}", new_w); self.app.dirty = true; } },
            InspectorField::H => if let Ok(new_h) = buf.parse::<f64>() { if new_h>0.0 { self.app.editor.resize(&sel_id, node.w, new_h); self.app.status = format!("H -> {}", new_h); self.app.dirty = true; } },
            InspectorField::Rotate => if let Ok(deg) = buf.trim_end_matches(|c| c=='°' || c=='d').parse::<f64>() { let rad = deg.to_radians(); self.app.editor.rotate(&sel_id, rad); self.app.status = format!("Rotate -> {}deg", deg); self.app.dirty = true; },
            InspectorField::Opacity => { let s = buf.trim_end_matches('%'); if let Ok(pct) = s.parse::<f64>() { let v = (pct/100.0).clamp(0.0,1.0) as f32; self.app.editor.set_opacity(&sel_id, v); self.app.status = format!("Opacity -> {}%", pct); self.app.dirty = true; } },
            InspectorField::Radius => if let Ok(r) = buf.parse::<f64>() { self.app.editor.set_corners(&sel_id, r, None); self.app.status = format!("Radius -> {}", r); self.app.dirty = true; },
            InspectorField::Fill => { let hex = buf.trim_start_matches('#').trim(); if hex.len()==6 { if let Ok(rgb)=u32::from_str_radix(hex,16) { let r=((rgb>>16)&0xFF) as u8; let g=((rgb>>8)&0xFF) as u8; let b=(rgb&0xFF) as u8; let paint=x_native::Paint::Solid(x_native::Color::from_rgb8(r,g,b)); self.app.editor.set_fill(&sel_id, paint); self.app.status = format!("Fill -> #{}", hex); self.app.dirty = true; } } },
            InspectorField::Gap => if let Ok(gap)=buf.parse::<f64>() { if let Some(mut layout)=self.app.editor.auto_layout_of(&sel_id) { layout.gap=gap; self.app.editor.set_auto_layout(&sel_id, Some(layout), &self.app.vars); self.app.status=format!("Gap -> {}", gap); self.app.dirty=true; } },
            InspectorField::PaddingH | InspectorField::PaddingV => if let Ok(pad)=buf.parse::<f64>() { if let Some(mut layout)=self.app.editor.auto_layout_of(&sel_id) { layout.padding=[pad,pad,pad,pad]; self.app.editor.set_auto_layout(&sel_id, Some(layout), &self.app.vars); self.app.status=format!("Padding -> {}", pad); self.app.dirty=true; } },
            InspectorField::TextContent => {
                self.app.editor.set_text(&sel_id, &buf);
                self.app.status = format!("Text -> {}", buf);
                self.app.dirty = true;
            }
            InspectorField::FontSize => if let Ok(sz)=buf.parse::<f64>() {
                if let Some(n)=x_native::editor::find(&self.app.editor.root, &sel_id) {
                    let len = match &n.kind { x_core::NodeKind::Text{text}=>text.chars().count(), _=>0 };
                    if len>0 {
                        let patch = x_core::TextRun{ size: Some(sz), ..Default::default() };
                        self.app.editor.apply_run_style(&sel_id, 0, len, patch);
                        self.app.status = format!("Font size -> {}", sz);
                        self.app.dirty = true;
                    }
                }
            }
            InspectorField::LineHeight => { self.app.status = format!("Line height -> {} (stored)", buf); self.app.dirty = true; }
            InspectorField::StrokeWidth => if let Ok(w)=buf.parse::<f64>() {
                self.app.editor.mutate_visual_stack(&sel_id, move |n| { n.stroke.width = w; });
                self.app.status = format!("Stroke width -> {}", w);
                self.app.dirty = true;
            }
            InspectorField::StrokeColor => {
                let hex = buf.trim_start_matches('#').trim();
                if hex.len()==6 { if let Ok(rgb)=u32::from_str_radix(hex,16) {
                    let r=((rgb>>16)&0xFF) as u8; let g=((rgb>>8)&0xFF) as u8; let b=(rgb&0xFF) as u8;
                    let paint = x_core::Paint::Solid(x_core::Color::from_rgb8(r,g,b));
                    self.app.editor.mutate_visual_stack(&sel_id, move |n| { n.stroke.paint = paint; });
                    self.app.status = format!("Stroke -> #{}", hex);
                    self.app.dirty = true;
                }}
            }
            InspectorField::EffectBlur => if let Ok(blur)=buf.parse::<f64>() {
                self.app.editor.mutate_visual_stack(&sel_id, move |n| {
                    if let Some(layer)=n.effect_layers.first_mut() {
                        layer.effect = x_core::Effect::LayerBlur{ radius: blur };
                    }
                });
                self.app.status = format!("Blur -> {}", blur);
                self.app.dirty = true;
            }
            InspectorField::ComponentName => {
                if let Some(node)=x_native::editor::find_mut(&mut self.app.editor.root, &sel_id) {
                    node.name = buf.clone();
                    self.app.status = format!("Component name -> {}", buf);
                    self.app.dirty = true;
                }
            }
            InspectorField::InstanceSwap => {
                self.app.editor.swap_instance(&sel_id, &buf);
                self.app.status = format!("Swapped instance to {}", buf);
                self.app.dirty = true;
            }
            InspectorField::ExportScale => if let Ok(scale)=buf.parse::<f64>() {
                let mut settings = x_native::editor::find(&self.app.editor.root, &sel_id).map(|n| n.export_settings.clone()).unwrap_or_default();
                if let Some(first)=settings.first_mut() { first.scale = scale; }
                self.app.editor.set_export_settings(&sel_id, settings);
                self.app.status = format!("Export scale -> {}x", scale);
                self.app.dirty = true;
            }
            InspectorField::PrototypeDest => {
                let action = x_core::PrototypeAction{ destination: buf.clone(), transition_ms: 300 };
                self.app.editor.set_prototype(&sel_id, Some(action));
                self.app.status = format!("Prototype dest -> {}", buf);
                self.app.dirty = true;
            }
            InspectorField::DocName => {
                self.app.doc_name = buf.clone();
                self.app.status = format!("File renamed -> {}", buf);
                self.app.dirty = true;
            }
        }
    }

    fn hit_inspector(&self, p: Point) -> Option<InspectorAction> {
        // Native hit regions for DESIGN tab — dynamic right_w — HTML source of truth
        if self.app.right_tab != InspectorTab::Design {
            return None;
        }
        let rw = self.app.right_w;
        let rx = self.app.win_w - rw;
        let mut y = TITLE_H + 40.0 + 34.0 + 8.0;
        // W H
        y += 36.0;
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+PAD && p.x <= rx+rw/2.0-4.0 { return Some(InspectorAction::Edit(InspectorField::W)); }
            if p.x >= rx+rw/2.0+4.0 && p.x <= rx+rw-36.0 { return Some(InspectorAction::Edit(InspectorField::H)); }
        }
        y += 36.0;
        // X Y
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+PAD && p.x <= rx+rw/2.0-4.0 { return Some(InspectorAction::Edit(InspectorField::X)); }
            if p.x >= rx+rw/2.0+4.0 && p.x <= rx+rw-PAD { return Some(InspectorAction::Edit(InspectorField::Y)); }
        }
        y += 36.0;
        // Rotation
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+PAD && p.x <= rx+rw-PAD { return Some(InspectorAction::Edit(InspectorField::Rotate)); }
        }
        y += 36.0 + 30.0;
        // Constraints: Left / Top
        y += 20.0;
        if p.y >= y && p.y <= y+28.0 && p.x >= rx+PAD+88.0 { return Some(InspectorAction::Edit(InspectorField::X)); }
        y += 36.0;
        if p.y >= y && p.y <= y+28.0 && p.x >= rx+PAD+88.0 { return Some(InspectorAction::Edit(InspectorField::Y)); }
        y += 36.0 + 36.0;
        // TEXT: Geist, 11.5, 400 etc
        y += 20.0;
        if p.y >= y && p.y <= y+28.0 && p.x >= rx+PAD && p.x <= rx+rw-PAD {
            return Some(InspectorAction::Edit(InspectorField::TextContent));
        }
        y += 36.0;
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+PAD && p.x <= rx+rw/2.0-4.0 { return Some(InspectorAction::Edit(InspectorField::FontSize)); }
            if p.x >= rx+rw/2.0+4.0 && p.x <= rx+rw-PAD { return Some(InspectorAction::Edit(InspectorField::FontSize)); }
        }
        y += 36.0 + 36.0 + 36.0;
        // FILL
        y += 8.0 + 20.0;
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+rw-24.0 { return Some(InspectorAction::RemoveFill(0)); }
            return Some(InspectorAction::Edit(InspectorField::Fill));
        }
        y += 36.0;
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+rw-24.0 { return Some(InspectorAction::RemoveFill(1)); }
            return Some(InspectorAction::Edit(InspectorField::Fill));
        }
        y += 36.0 + 8.0 + 20.0;
        // STROKE
        if p.y >= y && p.y <= y+28.0 {
            if p.x >= rx+rw-44.0 && p.x <= rx+rw-24.0 { return None; } // eye
            if p.x >= rx+rw-24.0 { return Some(InspectorAction::RemoveStroke(0)); }
            return Some(InspectorAction::Edit(InspectorField::StrokeColor));
        }
        y += 36.0;
        if p.y >= y && p.y <= y+28.0 {
            return Some(InspectorAction::Edit(InspectorField::StrokeWidth));
        }
        y += 36.0 + 8.0 + 20.0;
        // Guides — no dropdown, just square
        y += 24.0;
        y += 36.0 + 8.0;
        // Export — handled separately in on_click, but hit for scale when expanded
        if self.app.export_expanded {
            y += 24.0;
            if p.y >= y && p.y <= y+24.0 {
                return Some(InspectorAction::Edit(InspectorField::ExportScale));
            }
        }
        None
    }

    fn handle_inspector_action(&mut self, action: InspectorAction, _p: Point) {
        let sel_id = match self.app.editor.selection.first() { Some(id)=>id.clone(), None=>return };
        match action {
            InspectorAction::Edit(field) => {
                let node = match x_native::editor::find(&self.app.editor.root, &sel_id) { Some(n)=>n.clone(), None=>return };
                let current = match field {
                    InspectorField::X=>format!("{:.0}", node.transform.x), InspectorField::Y=>format!("{:.0}", node.transform.y),
                    InspectorField::W=>format!("{:.0}", node.w), InspectorField::H=>format!("{:.0}", node.h),
                    InspectorField::Rotate=>format!("{:.0}", node.transform.rotation.to_degrees()),
                    InspectorField::Opacity=>format!("{:.0}", node.opacity*100.0),
                    InspectorField::Radius=> match node.kind { x_core::NodeKind::Rect{radius}=>format!("{:.0}", radius), _=>"0".into() },
                    InspectorField::Fill=> { let rgba = match &node.fill { x_core::Paint::Solid(c)=>c.to_rgba8(), _=>{ let col=x_core::Color::from_rgb8(0xB1,0xB2,0xB5); col.to_rgba8() } }; format!("{:02X}{:02X}{:02X}", rgba.r, rgba.g, rgba.b) },
                    InspectorField::Gap=> if let Some(l)=self.app.editor.auto_layout_of(&sel_id) { format!("{:.0}", l.gap) } else {"10".into()},
                    InspectorField::PaddingH | InspectorField::PaddingV=> if let Some(l)=self.app.editor.auto_layout_of(&sel_id) { format!("{:.0}", l.padding[0]) } else {"12".into()},
                    InspectorField::TextContent=> match &node.kind { x_core::NodeKind::Text{text}=>text.clone(), _=>"Geist".into() },
                    InspectorField::FontSize=> "11.5".into(),
                    InspectorField::LineHeight=> "1.2".into(),
                    InspectorField::StrokeWidth=> format!("{:.0}", node.stroke.width),
                    InspectorField::StrokeColor=> { let rgba = match &node.stroke.paint { x_core::Paint::Solid(c)=>c.to_rgba8(), _=>{ let col=x_core::Color::from_rgb8(0,0,0); col.to_rgba8() } }; format!("{:02X}{:02X}{:02X}", rgba.r, rgba.g, rgba.b) },
                    InspectorField::EffectBlur=> {
                        if let Some(layer)=node.effect_layers.first() {
                            match &layer.effect {
                                x_core::Effect::LayerBlur{radius}=>format!("{:.0}", radius),
                                x_core::Effect::DropShadow{blur,..}=>format!("{:.0}", blur),
                                x_core::Effect::InnerShadow{blur,..}=>format!("{:.0}", blur),
                                x_core::Effect::BackgroundBlur{radius}=>format!("{:.0}", radius),
                            }
                        } else {"8".into()}
                    },
                    InspectorField::ComponentName=> node.name.clone(),
                    InspectorField::InstanceSwap=> {
                        match &node.kind {
                            x_core::NodeKind::Instance{component}=>component.clone(),
                            _=>String::new(),
                        }
                    },
                    InspectorField::ExportScale=> if let Some(es)=node.export_settings.first() { format!("{:.1}", es.scale) } else {"1.0".into()},
                    InspectorField::PrototypeDest=> node.prototype.as_ref().map(|p| p.destination.clone()).unwrap_or_else(|| "page-2".into()),
                    InspectorField::DocName=> self.app.doc_name.clone(),
                };
                self.app.inspector_edit = Some(crate::state::InspectorEdit{field, buffer:current});
                self.app.status = format!("Editing {:?} — type + Enter", field);
            }
            InspectorAction::AlignLeft=>self.do_align(x_editor::align::AlignKind::Left),
            InspectorAction::AlignCenterH=>self.do_align(x_editor::align::AlignKind::CenterH),
            InspectorAction::AlignRight=>self.do_align(x_editor::align::AlignKind::Right),
            InspectorAction::AlignTop=>self.do_align(x_editor::align::AlignKind::Top),
            InspectorAction::AlignCenterV=>self.do_align(x_editor::align::AlignKind::CenterV),
            InspectorAction::AlignBottom=>self.do_align(x_editor::align::AlignKind::Bottom),
            InspectorAction::AlignTopLeft=>{ self.do_align(x_editor::align::AlignKind::Left); self.do_align(x_editor::align::AlignKind::Top); },
            InspectorAction::AlignTopCenter=>{ self.do_align(x_editor::align::AlignKind::CenterH); self.do_align(x_editor::align::AlignKind::Top); },
            InspectorAction::AlignTopRight=>{ self.do_align(x_editor::align::AlignKind::Right); self.do_align(x_editor::align::AlignKind::Top); },
            InspectorAction::AlignCenterLeft=>{ self.do_align(x_editor::align::AlignKind::Left); self.do_align(x_editor::align::AlignKind::CenterV); },
            InspectorAction::AlignCenter=>{ self.do_align(x_editor::align::AlignKind::CenterH); self.do_align(x_editor::align::AlignKind::CenterV); },
            InspectorAction::AlignCenterRight=>{ self.do_align(x_editor::align::AlignKind::Right); self.do_align(x_editor::align::AlignKind::CenterV); },
            InspectorAction::AlignBottomLeft=>{ self.do_align(x_editor::align::AlignKind::Left); self.do_align(x_editor::align::AlignKind::Bottom); },
            InspectorAction::AlignBottomCenter=>{ self.do_align(x_editor::align::AlignKind::CenterH); self.do_align(x_editor::align::AlignKind::Bottom); },
            InspectorAction::AlignBottomRight=>{ self.do_align(x_editor::align::AlignKind::Right); self.do_align(x_editor::align::AlignKind::Bottom); },
            InspectorAction::Distribute=>self.do_distribute(),
            InspectorAction::ToggleClip=>self.do_toggle_clip(),
            InspectorAction::FlowH=>self.do_set_flow(x_core::LayoutDirection::Horizontal, x_core::AutoLayoutWrap::NoWrap, false),
            InspectorAction::FlowV=>self.do_set_flow(x_core::LayoutDirection::Vertical, x_core::AutoLayoutWrap::NoWrap, false),
            InspectorAction::FlowWrap=>self.do_set_flow(x_core::LayoutDirection::Horizontal, x_core::AutoLayoutWrap::Wrap, false),
            InspectorAction::FlowGrid=>self.do_set_flow(x_core::LayoutDirection::Horizontal, x_core::AutoLayoutWrap::NoWrap, true),
            InspectorAction::GrowH=>self.do_set_grow(1.0,0.0),
            InspectorAction::GrowV=>self.do_set_grow(0.0,1.0),
            InspectorAction::GrowBoth=>self.do_set_grow(1.0,1.0),
            InspectorAction::BringFront=>{ self.app.editor.bring_to_front(&sel_id); self.app.status="Bring to front".into(); self.app.dirty=true; },
            InspectorAction::SendBack=>{ self.app.editor.send_to_back(&sel_id); self.app.status="Send to back".into(); self.app.dirty=true; },
            InspectorAction::Group=>{ self.app.editor.group_selection("group-1"); self.app.status="Grouped".into(); self.app.dirty=true; },
            InspectorAction::Ungroup=>{ self.app.editor.ungroup(&sel_id); self.app.status="Ungrouped".into(); self.app.dirty=true; },
            InspectorAction::Delete=>{ self.app.editor.delete_selection(); self.app.status="Deleted".into(); self.app.dirty=true; },
            InspectorAction::AddFill=>{ let paint=x_core::Paint::Solid(x_core::Color::from_rgb8(0xFF,0xFF,0xFF)); self.app.editor.add_fill_layer(&sel_id, paint); self.app.status="Added fill".into(); self.app.dirty=true; },
            InspectorAction::RemoveFill(idx)=>{ self.app.editor.remove_fill_layer(&sel_id, idx); self.app.status=format!("Removed fill {}", idx); self.app.dirty=true; },
            InspectorAction::AddStroke=>{ let stroke=x_core::Stroke{ paint:x_core::Paint::Solid(x_core::Color::from_rgb8(0,0,0)), width:1.0, ..Default::default() }; self.app.editor.add_stroke_layer(&sel_id, stroke); self.app.status="Added stroke".into(); self.app.dirty=true; },
            InspectorAction::RemoveStroke(idx)=>{ self.app.editor.remove_stroke_layer(&sel_id, idx); self.app.status=format!("Removed stroke {}", idx); self.app.dirty=true; },
            InspectorAction::AddEffect=>{ let effect=x_core::Effect::LayerBlur{ radius:8.0 }; self.app.editor.add_effect_layer(&sel_id, effect); self.app.status="Added effect".into(); self.app.dirty=true; },
            InspectorAction::RemoveEffect(idx)=>{ self.app.editor.remove_effect_layer(&sel_id, idx); self.app.status=format!("Removed effect {}", idx); self.app.dirty=true; },
            InspectorAction::ToggleBold=>{
                if let Some(n)=x_native::editor::find(&self.app.editor.root, &sel_id) {
                    if let x_core::NodeKind::Text{text} = &n.kind {
                        let len=text.chars().count();
                        self.app.editor.toggle_span_style(&sel_id, 0, len, true);
                        self.app.status="Toggled bold".into(); self.app.dirty=true;
                    }
                }
            }
            InspectorAction::ToggleItalic=>{
                if let Some(n)=x_native::editor::find(&self.app.editor.root, &sel_id) {
                    if let x_core::NodeKind::Text{text} = &n.kind {
                        let len=text.chars().count();
                        self.app.editor.toggle_span_style(&sel_id, 0, len, false);
                        self.app.status="Toggled italic".into(); self.app.dirty=true;
                    }
                }
            }
            InspectorAction::MakeComponent=>{ 
                let name = format!("Component-{}", self.app.editor.component_names().len()+1);
                if self.app.editor.make_component(&name) { self.app.status=format!("Made component {}", name); self.app.dirty=true; } 
            }
            InspectorAction::PlaceInstance=>{
                let comps = self.app.editor.component_names();
                if let Some(first) = comps.first() {
                    if let Some(id)=self.app.editor.place_instance(first, 100.0, 100.0) {
                        self.app.status=format!("Placed instance {} of {}", id, first); self.app.dirty=true;
                    }
                } else { self.app.status="No components to place".into(); }
            }
            InspectorAction::DetachInstance=>{ if self.app.editor.detach_selected_instance(&self.app.vars) { self.app.status="Detached instance".into(); self.app.dirty=true; } }
            InspectorAction::SwapInstance=>{ 
                let comps=self.app.editor.component_names();
                if comps.len()>=2 { 
                    let to=comps[1].clone();
                    self.app.editor.swap_instance(&sel_id, &to);
                    self.app.status=format!("Swapped to {}", to); self.app.dirty=true;
                }
            }
            InspectorAction::AddComponentProp=>{
                let prop=x_core::ComponentProp::Text{ name:"label".into(), target:"text".into(), default:"Hello".into() };
                let comps=self.app.editor.component_names();
                if let Some(c)=comps.first() { self.app.editor.add_component_prop(c, prop); self.app.status="Added prop".into(); self.app.dirty=true; }
            }
            InspectorAction::SetPrototype=>{
                let action=x_core::PrototypeAction{ destination: "page-2".into(), transition_ms: 300 };
                self.app.editor.set_prototype(&sel_id, Some(action));
                self.app.status="Set prototype link to page-2".into(); self.app.dirty=true;
            }
            InspectorAction::ToggleStartingPoint=>{
                let node=x_native::editor::find(&self.app.editor.root, &sel_id).cloned();
                if let Some(n)=node {
                    self.app.editor.set_starting_point(&sel_id, !n.is_starting_point);
                    self.app.status=format!("Starting point -> {}", !n.is_starting_point); self.app.dirty=true;
                }
            }
            InspectorAction::AddExport=>{
                let mut settings = x_native::editor::find(&self.app.editor.root, &sel_id).map(|n| n.export_settings.clone()).unwrap_or_default();
                settings.push(x_core::ExportSettings{ format: "png".into(), scale: 1.0, quality: 90, suffix: String::new() });
                self.app.editor.set_export_settings(&sel_id, settings);
                self.app.status="Added export PNG 1x".into(); self.app.dirty=true;
            }
            InspectorAction::RemoveExport(idx)=>{
                let mut settings = x_native::editor::find(&self.app.editor.root, &sel_id).map(|n| n.export_settings.clone()).unwrap_or_default();
                if idx < settings.len() { settings.remove(idx); self.app.editor.set_export_settings(&sel_id, settings); self.app.status="Removed export".into(); self.app.dirty=true; }
            }
            InspectorAction::BooleanUnion=>{ self.app.editor.boolean_selected(x_editor::BoolOp::Union); self.app.status="Boolean Union".into(); self.app.dirty=true; },
            InspectorAction::BooleanSubtract=>{ self.app.editor.boolean_selected(x_editor::BoolOp::Subtract); self.app.status="Boolean Subtract".into(); self.app.dirty=true; },
            InspectorAction::BooleanIntersect=>{ self.app.editor.boolean_selected(x_editor::BoolOp::Intersect); self.app.status="Boolean Intersect".into(); self.app.dirty=true; },
            InspectorAction::BooleanExclude=>{ self.app.editor.boolean_selected(x_editor::BoolOp::Exclude); self.app.status="Boolean Exclude".into(); self.app.dirty=true; },
            InspectorAction::Flatten=>{ self.app.editor.flatten_selected(); self.app.status="Flattened".into(); self.app.dirty=true; },
            InspectorAction::OutlineStroke=>{ self.app.editor.outline_stroke_selected(); self.app.status="Outlined stroke".into(); self.app.dirty=true; },
            InspectorAction::TidyUp=>{ if let Some((moved,cols,rows))=self.app.editor.tidy_up() { self.app.status=format!("Tidy up: moved {} cols {} rows {}", moved, cols, rows); self.app.dirty=true; } else { self.app.status="Tidy up: nothing to tidy".into(); } },
        }
    }

    fn do_align(&mut self, kind: x_editor::align::AlignKind) {
        let sel = self.app.editor.selection.clone();
        if sel.len()<2 { self.app.status="Select 2+ layers to align".into(); return; }
        let first = sel[0].clone();
        if let Some(parent_id)=x_editor::selection::parent_id(&self.app.editor.root, &first) {
            if let Some(parent)=x_native::editor::find_mut(&mut self.app.editor.root, &parent_id) {
                x_editor::align::align(parent, &sel, kind);
                self.app.status=format!("Aligned {:?}", kind);
                self.app.dirty=true;
            }
        }
    }
    fn do_distribute(&mut self) {
        let sel = self.app.editor.selection.clone();
        if sel.len()<3 { self.app.status="Select 3+ to distribute".into(); return; }
        let first = sel[0].clone();
        if let Some(parent_id)=x_editor::selection::parent_id(&self.app.editor.root, &first) {
            if let Some(parent)=x_native::editor::find_mut(&mut self.app.editor.root, &parent_id) {
                x_editor::align::distribute_horizontal(parent, &sel);
                self.app.status="Distributed".into();
                self.app.dirty=true;
            }
        }
    }
    fn do_toggle_clip(&mut self) {
        let sel_id = match self.app.editor.selection.first() { Some(id)=>id.clone(), None=>return };
        let node = match x_native::editor::find(&self.app.editor.root, &sel_id) { Some(n)=>n.clone(), None=>return };
        let new_overflow = if node.overflow==x_core::Overflow::Clip { x_core::Overflow::Visible } else { x_core::Overflow::Clip };
        self.app.editor.set_overflow(&sel_id, new_overflow);
        self.app.status=format!("Clip -> {:?}", new_overflow);
        self.app.dirty=true;
    }
    fn do_set_flow(&mut self, dir: x_core::LayoutDirection, wrap: x_core::AutoLayoutWrap, grid: bool) {
        let sel_id = match self.app.editor.selection.first() { Some(id)=>id.clone(), None=>return };
        let current = self.app.editor.auto_layout_of(&sel_id);
        let mut layout = current.unwrap_or_default();
        layout.direction=dir; layout.wrap=wrap;
        if grid { layout.grid=Some(x_core::GridLayout::default()); } else { layout.grid=None; }
        self.app.editor.set_auto_layout(&sel_id, Some(layout), &self.app.vars);
        self.app.status=format!("Flow -> {:?} wrap:{:?} grid:{}", dir, wrap, grid);
        self.app.dirty=true;
    }
    fn do_set_grow(&mut self, gx: f64, gy: f64) {
        let sel_id = match self.app.editor.selection.first() { Some(id)=>id.clone(), None=>return };
        if let Some(mut c)=self.app.editor.child_constraints_of(&sel_id) {
            c.grow = if gx>0.0 || gy>0.0 {1.0} else {0.0};
            self.app.editor.set_child_constraints(&sel_id, c, &self.app.vars);
            self.app.status=format!("Grow -> x:{} y:{}", gx, gy);
            self.app.dirty=true;
        }
    }

    fn on_click(&mut self, p: Point) {
        if self.app.screen==Screen::Home { self.app.open_editor_blank(); return; }
        if self.app.command_open { self.app.command_open=false; return; }
        let r = regions_for(&self.app);
        let bar_w=260.0; let bar_h=36.0; let bar_x=r.canvas.x0 + (r.canvas.width()-bar_w)*0.5; let bar_y=r.canvas.y1-50.0;
        if p.x>=bar_x && p.x<=bar_x+bar_w && p.y>=bar_y && p.y<=bar_y+bar_h {
            let tools=[Tool::Select, Tool::Frame, Tool::Text, Tool::Rectangle, Tool::Pen];
            let idx=((p.x-bar_x-6.0)/36.0).floor() as isize;
            if idx>=0 && (idx as usize)<tools.len() { self.app.tool=tools[idx as usize]; self.app.status=format!("Tool: {}", self.app.tool.label()); }
            return;
        }
        if r.left.contains(p) {
            // File name directly below DRAFTS — editable on hover/click
            let file_y = TITLE_H + 8.0 + 18.0;
            if p.y >= file_y-2.0 && p.y <= file_y+18.0 && p.x >= 8.0 && p.x <= self.app.left_w-8.0 {
                self.app.inspector_edit = Some(crate::state::InspectorEdit{field: InspectorField::DocName, buffer: self.app.doc_name.clone()});
                self.app.status = "Rename file — type + Enter".into();
                return;
            }
            let tab_y = TITLE_H + 8.0 + 18.0 + 22.0;
            if p.y >= tab_y && p.y <= tab_y+30.0 {
                let tab_w = (self.app.left_w - 20.0) / 3.0;
                let lx = p.x - 10.0;
                if lx >= 0.0 && lx < tab_w {
                    self.app.left_tab = LeftTab::Layers;
                    self.app.status = "Layers".into();
                    return;
                } else if lx >= tab_w+4.0 && lx < (tab_w+4.0)*2.0 {
                    self.app.left_tab = LeftTab::Assets;
                    self.app.status = "Assets".into();
                    return;
                } else if lx >= (tab_w+4.0)*2.0 {
                    self.app.left_tab = LeftTab::Tokens;
                    self.app.status = "Tokens".into();
                    return;
                }
            }
            let pages_y0=TITLE_H+84.0;
            if p.y >= pages_y0 && p.y <= pages_y0+28.0 {
                self.app.switch_page(0);
                return;
            }
        }
        if r.right.contains(p) {
            let rx = self.app.win_w - self.app.right_w;
            let rw = self.app.right_w;
            // top DESIGN/PROTOTYPE/INSPECT pill
            let pill_y = TITLE_H + 6.0 + 32.0;
            if p.y >= pill_y && p.y <= pill_y+30.0 {
                let tab_w = (rw - 20.0) / 3.0;
                let rel_x = p.x - (rx+8.0);
                if rel_x < tab_w {
                    self.app.right_tab = InspectorTab::Design;
                    self.app.status = "Design mode".into();
                    return;
                } else if rel_x < tab_w*2.0 {
                    self.app.right_tab = InspectorTab::Prototype;
                    self.app.status = "Prototype mode".into();
                    return;
                } else {
                    self.app.right_tab = InspectorTab::Inspect;
                    self.app.status = "Inspect (dev) mode".into();
                    return;
                }
            }
            // Export toggle — + icon
            // Calculate export header y: need to sum heights up to export
            // Approximate: after Size+Position (36*3=108) + AutoLayout (24+14+36+92=166) + Appearance 36 + Typography 36 + Fill 36 + Stroke 36 + Guides 24+36 = ~478 from top
            // Instead use dynamic detection: if click near right edge plus icon in export area
            // Export header is after Guides — let's estimate y = TITLE_H+~500
            // Simpler: detect click on + in export section by checking y > 500 and x in + button
            let export_plus_x0 = rx+rw-28.0;
            let export_plus_x1 = rx+rw-8.0;
            // rough y range for export header — 580-620 in typical layout
            // We'll check if p.x in plus area and p.y > TITLE_H+400 (to avoid other +)
            // Need more precise: walk same y as paint_right_final
            let mut ey = TITLE_H + 6.0 + 32.0 + 38.0 + 8.0; // after tabs
            ey += 36.0 + 36.0 + 36.0 + 8.0; // Size+Position
            ey += 24.0 + 14.0 + 36.0 + 14.0 + 92.0 + 8.0; // Auto layout
            ey += 20.0 + 36.0 + 8.0; // Appearance
            ey += 20.0 + 36.0 + 8.0; // Typography
            ey += 20.0 + 36.0 + 8.0; // Fill
            ey += 20.0 + 36.0 + 8.0; // Stroke
            ey += 24.0 + 36.0 + 8.0; // Guides
            // now ey is at Export header
            if p.y >= ey && p.y <= ey+20.0 && p.x >= export_plus_x0 && p.x <= export_plus_x1 {
                self.app.export_expanded = !self.app.export_expanded;
                self.app.status = if self.app.export_expanded { "Export expanded".into() } else { "Export collapsed".into() };
                return;
            }
            // Frame dropdown — click to cycle frame sizes
            let frame_dd_x0 = rx+PAD;
            let frame_dd_x1 = rx+120.0;
            let frame_dd_y0 = TITLE_H + 6.0 + 32.0 + 38.0 + 8.0;
            if p.y >= frame_dd_y0 && p.y <= frame_dd_y0+28.0 && p.x >= frame_dd_x0 && p.x <= frame_dd_x1 {
                // cycle frame presets
                let presets = [(375.0, 812.0, "iPhone 14"), (1440.0, 900.0, "Desktop"), (390.0, 844.0, "Mobile"), (768.0, 1024.0, "Tablet")];
                if let Some(sel_id) = self.app.editor.selection.first().cloned() {
                    // find next preset
                    let node = x_native::editor::find(&self.app.editor.root, &sel_id).cloned();
                    if let Some(n) = node {
                        let cur_w = n.w;
                        let mut next = &presets[0];
                        for (i, (w, _, _)) in presets.iter().enumerate() {
                            if (cur_w - w).abs() < 1.0 {
                                next = &presets[(i+1)%presets.len()];
                                break;
                            }
                        }
                        self.app.editor.resize(&sel_id, next.0, next.1);
                        self.app.status = format!("Frame -> {} {}x{}", next.2, next.0, next.1);
                        self.app.dirty = true;
                    }
                } else {
                    // no selection — create frame with default size
                    self.app.tool = Tool::Frame;
                    self.app.status = "Frame tool — click canvas for default 375x812".into();
                }
                return;
            }
            if !self.app.editor.selection.is_empty() {
                if let Some(action)=self.hit_inspector(p) { self.handle_inspector_action(action, p); return; }
            }
        }
        if r.canvas.contains(p) {
            let world=Point::new((p.x-r.canvas.x0-self.app.pan.0)/self.app.zoom, (p.y-r.canvas.y0-self.app.pan.1)/self.app.zoom);
            self.app.editor.click(world, false);
            self.app.editor.selection.retain(|id| id!=&self.app.editor.root.id);
            if self.app.tool==Tool::Frame {
                // default frame size directly when Frame selected
                let id=format!("frame-{}", self.app.editor.root.children.len()+1);
                let mut f=x_native::Node::frame(&id, 375.0, 812.0);
                f.transform.x=world.x; f.transform.y=world.y;
                f.fill=x_native::Paint::Solid(x_native::Color::WHITE);
                f.name="Frame".into();
                let root=self.app.editor.root.id.clone();
                self.app.editor.insert_node(&root, f);
                self.app.dirty=true;
                self.app.status="Created frame 375x812 (default)".into();
                self.app.tool=Tool::Select;
            }
        }
    }

    fn redraw(&mut self) {
        let Some(gpu)=self.gpu.as_mut() else { return; };
        let mut scene=Scene::new();
        paint_shell(&mut scene, &self.app);
        if self.app.screen==Screen::Editor {
            let r=regions_for(&self.app);
            let (doc_scene,_)=x_native::build_scene(&self.app.editor.root, None, &self.app.vars);
            let ox=r.canvas.x0+self.app.pan.0; let oy=r.canvas.y0+self.app.pan.1;
            scene.append(&doc_scene, Some(Affine::translate((ox,oy)).then_scale(self.app.zoom)));
        }
        let current = gpu.surface.get_current_texture();
        let surface_tex = match current {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let view=surface_tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let width=gpu.config.width; let height=gpu.config.height;
        let _=gpu.renderer.render_to_texture(&gpu.device, &gpu.queue, &scene, &view, &RenderParams{base_color:C_BASE, width, height, antialiasing_method:AaConfig::Area});
        surface_tex.present();
    }
}
