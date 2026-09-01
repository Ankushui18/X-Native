//! X Native — beta app shell (modular binary).
//! Modules: theme (constants), state (Tool/Drag/Focus/App), app (impl App:
//! input + chrome), helpers (draw utils), demo (starter doc), run (event loop).

use arco_native::editor::{find, hit_test_rect, Editor};
use arco_native::fileio::{export_svg, load_x_file, save_x_file};
use arco_native::text::{encode_text, measure};
use arco_native::{
    build_scene, AutoLayout, BlendKind, Color, CrossAlign, Document, Effect, LayoutDirection, Node,
    Paint, StrokeAlign, Variables, PI,
};
use std::sync::Arc;
use vello::kurbo::{Affine, Point, Rect, Shape};
use vello::peniko::Fill;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

mod theme;
mod state;
mod app;
mod chrome;
mod helpers;
mod demo;
mod run;

pub use theme::*;
pub use state::*;
pub use helpers::*;
pub use demo::*;
pub use run::*;

/// OS clipboard bridge (native macOS, then Wayland/X11 fallbacks).
/// Editor-object clipboard stays internal; this carries TEXT + exported
/// SVG across apps — the interop slice that matters most.
pub fn os_clipboard_set(text: &str) {
    use std::io::Write;
    let candidates: [(&str, &[&str]); 3] = [
        ("pbcopy", &[]),
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
    ];
    for (cmd, args) in candidates {
        if let Ok(mut child) = std::process::Command::new(cmd)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() { let _ = stdin.write_all(text.as_bytes()); }
            if child.wait().map(|st| st.success()).unwrap_or(false) { return; }
        }
    }
}

pub fn os_clipboard_get() -> Option<String> {
    let candidates: [(&str, &[&str]); 3] = [
        ("pbpaste", &[]),
        ("wl-paste", &["--no-newline"]),
        ("xclip", &["-selection", "clipboard", "-o"]),
    ];
    for (cmd, args) in candidates {
        if let Ok(out) = std::process::Command::new(cmd).args(args).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                if !s.is_empty() { return Some(s); }
            }
        }
    }
    None
}

fn main() { pollster::block_on(run()); }
