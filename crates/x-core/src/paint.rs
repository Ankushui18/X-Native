use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------------- paint

/// Phase 4: gradients join solid and variable-bound paints.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Solid(Color),
    Variable(String),
    LinearGradient { start: (f64, f64), end: (f64, f64), stops: Vec<(f32, Color)> },
    RadialGradient { center: (f64, f64), radius: f64, stops: Vec<(f32, Color)> },
}

/// One ordered fill entry. Layers are painted back-to-front in vector order.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintLayer {
    pub paint: Paint,
    pub opacity: f32,
    pub visible: bool,
    pub blend: BlendKind,
}
impl PaintLayer {
    pub fn new(paint: Paint) -> Self { Self { paint, opacity: 1.0, visible: true, blend: BlendKind::Normal } }
}

/// One ordered stroke entry. Geometry options can expand here without
/// changing the node or file schema again.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeLayer {
    pub stroke: Stroke,
    pub opacity: f32,
    pub visible: bool,
    pub blend: BlendKind,
    pub options: StrokeOptions,
}
impl StrokeLayer {
    pub fn new(stroke: Stroke) -> Self { Self { stroke, opacity: 1.0, visible: true, blend: BlendKind::Normal, options: StrokeOptions::default() } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeAlign { Inside, #[default] Center, Outside }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeCap { #[default] None, Round, Square, Arrow, Triangle }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeJoin { #[default] Miter, Bevel, Round }
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeOptions {
    pub align: StrokeAlign,
    pub cap_start: StrokeCap,
    pub cap_end: StrokeCap,
    pub join: StrokeJoin,
    pub dash: Vec<f64>,
    pub dash_offset: f64,
    pub miter_limit: f64,
}
impl Default for StrokeOptions {
    fn default() -> Self { Self { align: StrokeAlign::Center, cap_start: StrokeCap::None, cap_end: StrokeCap::None, join: StrokeJoin::Miter, dash: vec![], dash_offset: 0.0, miter_limit: 4.0 } }
}

#[derive(Debug, Clone, Copy, PartialEq)] pub struct Stroke { pub color: Color, pub width: f64 }
impl Default for Stroke { fn default() -> Self { Self { color: Color::BLACK, width: 0.0 } } }

/// Phase 4: blend modes. Applied as a Vello mix layer around the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendKind { #[default] Normal, Darken, Multiply, ColorBurn, Lighten, Screen, ColorDodge, Overlay, SoftLight, HardLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity }
impl BlendKind {
    pub fn mix(self) -> Option<Mix> {
        match self {
            BlendKind::Normal => None,
            BlendKind::Multiply => Some(Mix::Multiply),
            BlendKind::Screen => Some(Mix::Screen),
            BlendKind::Overlay => Some(Mix::Overlay),
            BlendKind::Darken => Some(Mix::Darken),
            BlendKind::Lighten => Some(Mix::Lighten),
            BlendKind::ColorBurn => Some(Mix::ColorBurn),
            BlendKind::ColorDodge => Some(Mix::ColorDodge),
            BlendKind::SoftLight => Some(Mix::SoftLight),
            BlendKind::HardLight => Some(Mix::HardLight),
            BlendKind::Difference => Some(Mix::Difference),
            BlendKind::Exclusion => Some(Mix::Exclusion),
            BlendKind::Hue => Some(Mix::Hue),
            BlendKind::Saturation => Some(Mix::Saturation),
            BlendKind::Color => Some(Mix::Color),
            BlendKind::Luminosity => Some(Mix::Luminosity),
        }
    }
}

/// Ordered layer effects. The renderer lowers these to normalized GPU-vector
/// Gaussian taps on the pinned Vello backend, including clipped background
/// replay for background blur and clipped edge compositing for inner shadow.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    DropShadow { dx: f64, dy: f64, blur: f64, color: Color },
    InnerShadow { dx: f64, dy: f64, blur: f64, color: Color },
    LayerBlur { radius: f64 },
    BackgroundBlur { radius: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectLayer {
    pub effect: Effect,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendKind,
}
impl EffectLayer {
    pub fn new(effect: Effect) -> Self { Self { effect, visible: true, opacity: 1.0, blend: BlendKind::Normal } }
}
