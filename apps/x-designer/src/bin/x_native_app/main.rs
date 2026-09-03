//! X Native — beta app shell (modular binary).
//! Modules: theme (constants), state (Tool/Drag/Focus/App), app (impl App:
//! input + chrome), helpers (draw utils), demo (starter doc), run (event loop).

use std::sync::Arc;
use vello::kurbo::{Affine, Point, Rect, Shape};
use vello::peniko::Fill;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use x_native::editor::{
    find, node_gap, node_measurements, node_to_compose, node_to_css, node_to_swift, node_to_xml,
    node_tokens, parent_id, selection_assets, Editor,
};
use x_native::text::{encode_text, measure};
use x_native::{
    eval_expr, format_cond, format_expr, parse_cond_text, parse_expr_text, Action, Alignment,
    Animation, AutoLayout, BlendKind, Color, Condition, CrossAlign, Direction, Distribute,
    Document, Effect, ExportSettings, Expr, GradSpace, GridPattern, Interaction, LayoutDirection,
    LayoutGridDef, Node, Overflow, OverlayPosition, Paint, StrokeAlign, Trigger, Value, Variables,
};

mod app;
mod chrome;
mod demo;
mod helpers;
mod icons;
mod run;
mod state;
mod theme;

pub use demo::*;
pub use helpers::*;
pub use run::*;
pub use state::*;
pub use theme::*;

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
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().map(|st| st.success()).unwrap_or(false) {
                return;
            }
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
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn main() {
    run();
}
