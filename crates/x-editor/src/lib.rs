//! Phase 2 / 8 / 9 / 10 slices: the headless editing engine.
//!
//! Everything here is UI-independent on purpose: a winit window (Phase 1)
//! will translate mouse/keyboard events into these operations. That means
//! all of it is testable right now, in this sandbox, without a display.
//!
//! - hit testing (transform-aware, z-order-aware, lock/visibility-aware)
//! - Editor: selection + command-based mutations with full undo/redo
//! - move / resize / rotate / set-fill / reorder(z) / group / delete
//! - align & distribute
//! - snapping (grid + other-object edges)
//! - constraints solver (pins: left/right/center/stretch/scale)
//! - Phase 8: prototype Player (navigate/back state machine)
//! - Phase 9: SpatialGrid index for O(~1) point queries at 100K nodes
//! - Phase 10: named version checkpoints + dev-mode CSS export

pub mod align;
pub mod booleans;
pub mod commands;
pub mod constraints;
pub mod devmode;
pub mod editor_core;
pub mod prototype;
pub mod selection;
pub mod snapping;
pub mod spatial;
#[cfg(test)]
mod tests_mod;
pub mod vector_edit;

pub use align::*;
pub use booleans::{boolean_paths, node_to_path, BoolOp};
pub use commands::*;
pub use constraints::*;
pub use devmode::*;
pub use editor_core::*;
pub use prototype::*;
pub use selection::*;
pub use snapping::*;
pub use spatial::*;
pub use vector_edit::{anchor_at, anchors, segment_at, Anchor};
