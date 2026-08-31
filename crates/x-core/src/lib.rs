//! x-core — the document model: nodes, geometry, layout, variables,
//! components. No rendering, no editing, no IO.
#![allow(unused_imports)]

pub mod transform;
pub mod paint;
pub mod pins;
pub mod layout_types;
pub mod node;
pub mod document;
pub mod assets;
pub mod image_transform;
pub mod library;
pub mod geometry;
pub mod auto_layout;
pub mod variables;
pub mod registry;
pub mod components;
pub mod p0_features;

pub use transform::*;
pub use paint::*;
pub use pins::*;
pub use layout_types::*;
pub use node::*;
pub use document::*;
pub use assets::*;
pub use image_transform::*;
pub use library::*;
pub use geometry::*;
pub use auto_layout::*;
pub use variables::*;
pub use registry::*;
pub use components::*;
pub use p0_features::*;

pub use std::f64::consts::PI;
pub use vello::peniko::Color;
