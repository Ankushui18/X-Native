//! x-core — the document model: nodes, geometry, layout, variables,
//! components. No rendering, no editing, no IO.
#![allow(unused_imports)]

pub mod assets;
pub mod auto_layout;
pub mod bezier_clip;
pub mod booleans;
pub mod clip;
pub mod components;
pub mod document;
pub mod geometry;
pub mod grid;
pub mod image_transform;
pub mod layout_types;
pub mod library;
pub mod node;
pub mod paint;
pub mod pins;
pub mod prototype;
pub mod registry;
pub mod transform;
pub mod variables;

pub use assets::*;
pub use auto_layout::*;
pub use components::*;
pub use document::*;
pub use geometry::*;
pub use image_transform::*;
pub use layout_types::*;
pub use library::*;
pub use node::*;
pub use paint::*;
pub use pins::*;
pub use prototype::*;
pub use registry::*;
pub use transform::*;
pub use variables::*;

pub use std::f64::consts::PI;

// geometry + color live here (audit F12): x-core is the canonical provider
// of kurbo/peniko so pure-model crates never need the vello GPU stack.
pub use kurbo;
pub use peniko;
pub use peniko::Color;
