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

pub mod selection;
pub mod commands;
pub mod editor_core;
pub mod align;
pub mod snapping;
pub mod constraints;
pub mod prototype;
pub mod spatial;
pub mod devmode;
pub mod vector_edit;
pub mod booleans;
#[cfg(test)]
mod tests_mod;

pub use selection::*;
pub use commands::*;
pub use editor_core::*;
pub use align::*;
pub use snapping::*;
pub use constraints::*;
pub use prototype::*;
pub use spatial::*;
pub use devmode::*;
pub use vector_edit::{anchors, anchor_at, Anchor};
pub use booleans::{BoolOp, boolean_paths, node_to_path};
