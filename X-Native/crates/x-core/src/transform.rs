#[allow(unused_imports)]
use crate::*;
pub use kurbo::Point;
pub use kurbo::{Affine, Rect};
use kurbo::{Circle, RoundedRect, RoundedRectRadii, Shape};
use peniko::{Brush, Color, Fill, Gradient, Mix};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------- transform

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    /// Skew angles (radians) applied after scale (Figma's shear transform).
    pub skew_x: f64,
    pub skew_y: f64,
    /// Transform-origin pivot in NORMALIZED 0..1 local space (Figma's 9-point
    /// origin). (0.5, 0.5) = center (the default); (0,0) = top-left, etc.
    pub origin_x: f64,
    pub origin_y: f64,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
            skew_y: 0.0,
            origin_x: 0.5,
            origin_y: 0.5,
        }
    }
}
impl Transform {
    /// The pivot point in node-local px (the transform-origin).
    pub fn pivot(self, w: f64, h: f64) -> (f64, f64) {
        (self.origin_x * w, self.origin_y * h)
    }
    pub fn matrix(self, w: f64, h: f64) -> Affine {
        let (px, py) = self.pivot(w, h);
        Affine::translate((self.x + px, self.y + py))
            * Affine::rotate(self.rotation)
            * Affine::scale_non_uniform(self.scale_x, self.scale_y)
            * Affine::skew(self.skew_x, self.skew_y)
            * Affine::translate((-px, -py))
    }
}

impl Transform {
    /// QR decomposition of an affine into our rotate/scale/skew representation.
    /// A zero-origin pivot makes the translation exact. Used when moving a
    /// copied subtree out of a transformed parent into page space.
    pub fn from_affine(matrix: Affine) -> Self {
        let [a, b, c, d, e, f] = matrix.as_coeffs();
        let sx = a.hypot(b);
        if sx > 1e-12 {
            Self {
                x: e,
                y: f,
                rotation: b.atan2(a),
                scale_x: sx,
                scale_y: (a * d - b * c) / sx,
                skew_x: ((a * c + b * d) / (sx * sx)).atan(),
                skew_y: 0.0,
                origin_x: 0.0,
                origin_y: 0.0,
            }
        } else {
            Self {
                x: e,
                y: f,
                rotation: (-c).atan2(d),
                scale_x: 0.0,
                scale_y: c.hypot(d),
                origin_x: 0.0,
                origin_y: 0.0,
                ..Self::default()
            }
        }
    }
}
