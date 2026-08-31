//! x-render — scene encoding for Vello: shapes, gradients, effects,
//! blend layers, image assets, component-instance resolution, stress scenes.

pub mod assets;
pub mod scene;
pub mod stress;
pub mod ir;
pub mod sinks;
pub mod frame_cache;
#[cfg(test)]
mod tests_mod;

pub use assets::*;
pub use scene::*;
pub use stress::*;
pub use ir::{build_render_tree, render_via_ir, RenderCommand, RenderTree, VelloSink};
pub use sinks::{thumbnail_scene, export_pdf, export_pdf_with_assets, export_pdf_full, SceneCache};
pub use frame_cache::{FrameCache, FrameCacheStats};

pub use vello::peniko::Color;
