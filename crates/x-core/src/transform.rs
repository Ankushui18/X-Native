use std::collections::HashMap;
pub use kurbo::{Affine, Rect};
pub use kurbo::Point;
use kurbo::{Circle, RoundedRect, RoundedRectRadii, Shape};
use peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// ---------------------------------------------------------------- transform

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform { pub x: f64, pub y: f64, pub rotation: f64, pub scale_x: f64, pub scale_y: f64 }
impl Default for Transform { fn default() -> Self { Self { x: 0.0, y: 0.0, rotation: 0.0, scale_x: 1.0, scale_y: 1.0 } } }
impl Transform {
    pub fn matrix(self, w: f64, h: f64) -> Affine {
        let (cx, cy) = (w / 2.0, h / 2.0);
        Affine::translate((self.x + cx, self.y + cy))
            * Affine::rotate(self.rotation)
            * Affine::scale_non_uniform(self.scale_x, self.scale_y)
            * Affine::translate((-cx, -cy))
    }
}

