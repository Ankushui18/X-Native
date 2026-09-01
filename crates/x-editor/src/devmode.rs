

use vello::peniko::Color;
use x_core::*;
#[allow(unused_imports)]
use crate::*;

// ------------------------------------------------------------------ dev mode

/// Phase 10.4: dev-mode export — CSS for a node (the inspect panel's copy).
pub fn node_to_css(node: &Node, vars: &Variables) -> String {
    let mut css = String::new();
    css.push_str(&format!(".{} {{\n", node.id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")));
    css.push_str(&format!("  width: {}px;\n  height: {}px;\n", node.w, node.h));
    match &node.fill {
        Paint::Solid(c) if c.a > 0 => css.push_str(&format!("  background: {};\n", x_core::color_to_hex(*c))),
        Paint::Variable(n) => css.push_str(&format!("  background: {}; /* var: {} */\n", x_core::color_to_hex(vars.color(n, Color::BLACK)), n)),
        Paint::LinearGradient { stops, .. } => {
            let s: Vec<String> = stops.iter().map(|(t, c)| format!("{} {}%", x_core::color_to_hex(*c), t * 100.0)).collect();
            css.push_str(&format!("  background: linear-gradient(90deg, {});\n", s.join(", ")));
        }
        _ => {}
    }
    if let NodeKind::Rect { radius } = node.kind {
        if let Some([tl, tr, br, bl]) = node.corner_radii {
            css.push_str(&format!("  border-radius: {tl}px {tr}px {br}px {bl}px;\n"));
        } else if radius > 0.0 {
            css.push_str(&format!("  border-radius: {radius}px;\n"));
        }
    }
    if node.opacity < 1.0 { css.push_str(&format!("  opacity: {};\n", node.opacity)); }
    if node.transform.rotation != 0.0 { css.push_str(&format!("  transform: rotate({:.1}deg);\n", node.transform.rotation.to_degrees())); }
    for e in &node.effects {
        if let x_core::Effect::DropShadow { dx, dy, blur, color } = e {
            css.push_str(&format!("  box-shadow: {dx}px {dy}px {blur}px {};\n", x_core::color_to_hex(*color)));
        }
    }
    css.push_str("}\n");
    css
}

