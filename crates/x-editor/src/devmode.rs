

use x_core::peniko::Color;
use x_core::*;
#[allow(unused_imports)]
use crate::*;

// ------------------------------------------------------------------ dev mode

/// Phase 10.4: dev-mode export — CSS for a node (the inspect panel's copy).
pub fn node_to_css(node: &Node, vars: &Variables) -> String {
    let mut css = String::new();
    css.push_str(&format!(".{} {{\n", node.id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")));
    // dev-mode CSS assumes the absolute-layout context the editor renders in
    css.push_str("  position: absolute;\n");
    css.push_str(&format!("  left: {}px;\n  top: {}px;\n", node.transform.x, node.transform.y));
    css.push_str(&format!("  width: {}px;\n  height: {}px;\n", node.w, node.h));
    // resize pins (Sketch resizing constraints / Figma constraints), when
    // they differ from the left/top default — the inspect panel's hint
    if (node.pin.0, node.pin.1) != (HPin::Left, VPin::Top) {
        css.push_str(&format!("  /* resize: pinned {} / {} */\n", pin_h_name(node.pin.0), pin_v_name(node.pin.1)));
    }
    match &node.fill {
        Paint::Solid(c) if c.components[3] > 0.0 => css.push_str(&format!("  background: {};\n", x_core::color_to_hex(*c))),
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
    if node.stroke.width > 0.0 {
        match node.stroke.solid_color() {
            Some(c) if c.components[3] > 0.0 => css.push_str(&format!("  border: {}px solid {};\n", node.stroke.width, x_core::color_to_hex(c))),
            _ if node.stroke.solid_color().is_none() => css.push_str(&format!("  border: {}px solid; /* gradient stroke */\n", node.stroke.width)),
            _ => {}
        }
    }
    if let NodeKind::Text { .. } = node.kind {
        // h IS the font size; font/ls/lh ride the bindings (typography
        // bindings — the same source the render sinks honor)
        css.push_str(&format!("  font-size: {}px;\n", node.h));
        let font = node.bindings.get("font").cloned().unwrap_or_else(|| "sans-serif".into());
        css.push_str(&format!("  font-family: \"{font}\";\n"));
        let lh = node.bindings.get("lh").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.2);
        css.push_str(&format!("  line-height: {lh};\n"));
        let ls = node.bindings.get("ls").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        if ls != 0.0 { css.push_str(&format!("  letter-spacing: {ls}px;\n")); }
        // rich runs are per-character styling — CSS can't express them in
        // one rule, so surface the count as a hint instead of lying
        if !node.text_runs.is_empty() {
            css.push_str(&format!("  /* {} rich text run(s): per-character styling */\n", node.text_runs.len()));
        }
    }
    if node.opacity < 1.0 { css.push_str(&format!("  opacity: {};\n", node.opacity)); }
    if node.transform.rotation != 0.0 { css.push_str(&format!("  transform: rotate({:.1}deg);\n", node.transform.rotation.to_degrees())); }
    for e in &node.effects {
        if let x_core::Effect::DropShadow { dx, dy, blur, color } = e {
            css.push_str(&format!("  box-shadow: {dx}px {dy}px {blur}px {};\n", x_core::color_to_hex(*color)));
        }
    }
    if let NodeKind::Image { asset, fit, .. } = &node.kind {
        css.push_str(&format!("  background-image: url(\"{asset}\"); /* fit: {fit:?} */\n"));
    }
    css.push_str("}\n");
    css
}

fn pin_h_name(p: HPin) -> &'static str {
    match p {
        HPin::Left => "left",
        HPin::Right => "right",
        HPin::CenterH => "center-h",
        HPin::StretchH => "stretch-h",
        HPin::ScaleH => "scale-h",
    }
}

fn pin_v_name(p: VPin) -> &'static str {
    match p {
        VPin::Top => "top",
        VPin::Bottom => "bottom",
        VPin::CenterV => "center-v",
        VPin::StretchV => "stretch-v",
        VPin::ScaleV => "scale-v",
    }
}

