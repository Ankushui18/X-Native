#[allow(unused_imports)]
use crate::*;
use x_core::*;

// -------------------------------------------------------------- constraints

/// Phase 2.12: apply pin constraints to `frame`'s children after the frame
/// resizes from (old_w, old_h) to its current (w, h).
pub fn apply_constraints(frame: &mut Node, old_w: f64, old_h: f64) {
    let (dw, dh) = (frame.w - old_w, frame.h - old_h);
    let (sx, sy) = (
        if old_w > 0.0 { frame.w / old_w } else { 1.0 },
        if old_h > 0.0 { frame.h / old_h } else { 1.0 },
    );
    for c in &mut frame.children {
        match c.pin.0 {
            HPin::Left => {}
            HPin::Right => c.transform.x += dw,
            HPin::CenterH => c.transform.x += dw / 2.0,
            HPin::StretchH => c.w += dw,
            HPin::ScaleH => {
                c.transform.x *= sx;
                c.w *= sx;
            }
        }
        match c.pin.1 {
            VPin::Top => {}
            VPin::Bottom => c.transform.y += dh,
            VPin::CenterV => c.transform.y += dh / 2.0,
            VPin::StretchV => c.h += dh,
            VPin::ScaleV => {
                c.transform.y *= sy;
                c.h *= sy;
            }
        }
        c.dirty = true;
    }
}
