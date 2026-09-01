use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Gradient, Mix};
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutoLayout {
    pub direction: LayoutDirection,
    pub gap: f64,
    pub padding: f64,
    pub sizing: Sizing,
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

