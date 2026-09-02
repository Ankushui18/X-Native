use std::collections::HashMap;
use kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// ------------------------------------------------------------------- layout

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutDirection { Horizontal, #[default] Vertical }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sizing { #[default] Fixed, Hug }
/// Phase 5.1: cross-axis alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAlign { #[default] Start, Center, End }
/// Phase P0: AutoLayout wrap mode for text wrapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoLayoutWrap { #[default] NoWrap, Wrap }
/// Phase P0: Alignment with baseline support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment { #[default] Min, Center, Max, Baseline }
/// Phase P0: Child constraints within auto-layout
#[derive(Debug, Clone)]
pub struct ChildConstraints {
    pub align_self: Option<Alignment>,
    pub grow: f64,
    pub shrink: f64,
    pub basis: Option<f64>,
    pub is_absolute: bool, // removed from normal flow
}
impl Default for ChildConstraints {
    fn default() -> Self {
        Self { align_self: None, grow: 0.0, shrink: 1.0, basis: None, is_absolute: false }
    }
}

/// Per-side frame padding: `[left, right, top, bottom]`.
pub type Padding = [f64; 4];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutoLayout {
    pub direction: LayoutDirection,
    pub gap: f64,
    pub padding: Padding,
    /// Main-axis sizing (`Hug` sizes the frame to its content).
    pub sizing: Sizing,
    /// Independent cross-axis sizing; `None` follows `sizing` (legacy
    /// behavior: one flag for both axes).
    pub cross_sizing: Option<Sizing>,
    pub gap_var: Option<String>,
    pub padding_var: Option<String>,
    /// Phase 5.1: cross-axis alignment of children.
    pub align: CrossAlign,
    /// Phase 5.1: distribute free main-axis space between children
    /// (overrides `gap` when the frame is Fixed-sized and children fit).
    pub space_between: bool,
    /// Phase P0: wrap mode
    pub wrap: AutoLayoutWrap,
    /// Phase P0: min/max constraints
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_height: Option<f64>,
    /// Phase P0: resize on wrap
    pub resize_on_wrap: bool,
}

impl AutoLayout {
    /// True when all four sides carry the same value (serializes as the
    /// legacy scalar `"padding":N`, keeping old files byte-stable).
    pub fn uniform_pad(&self) -> bool {
        let [l, r, t, b] = self.padding;
        l == r && r == t && t == b
    }
    /// Cross-axis sizing with the `None`-follows-`sizing` fallback applied.
    pub fn cross(&self) -> Sizing { self.cross_sizing.unwrap_or(self.sizing) }
}

