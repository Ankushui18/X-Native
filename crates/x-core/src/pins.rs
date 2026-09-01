use std::collections::HashMap;
use vello::kurbo::{Affine, Circle, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill, Gradient, Mix};
#[allow(unused_imports)]
use crate::*;

// -------------------------------------------------------------- constraints

/// Phase 2.12: resize constraints (how a child reacts when its frame resizes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HPin { #[default] Left, Right, CenterH, StretchH, ScaleH }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VPin { #[default] Top, Bottom, CenterV, StretchV, ScaleV }

