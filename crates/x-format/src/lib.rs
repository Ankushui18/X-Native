//! Phase 7 slice: the native `.x` document format + SVG export.
//!
//! `.x` v1 is versioned JSON (schema field first, forward-compatible:
//! unknown keys are skipped on load). Written with a purpose-built emitter
//! and read with a purpose-built recursive-descent parser — zero new
//! dependencies, which matters given this crate's pinned dependency tree.
//! A binary (postcard/flatbuffers) format can replace the encoding behind
//! the same save/load API later.

pub mod serialize;
pub mod deserialize;
pub mod svg_export;
pub mod svg_import;
pub mod v2;
#[cfg(test)]
mod tests_mod;
pub(crate) mod json;
pub(crate) mod b64;
pub mod xlib;
pub mod reliability;
pub(crate) mod zipfile;
pub mod sketch;
pub mod import_ir;
pub mod figma;
pub mod png_import;

pub use serialize::*;
pub use deserialize::*;
pub use svg_export::*;
pub use svg_import::*;
pub use sketch::{import_sketch, import_sketch_with_report, export_sketch};
pub use import_ir::{lower, lower_with_report, ImportDoc, ImportNode, ImportKind, ImportReport};
pub use figma::{import_figma_json, export_figma_json};
pub use png_import::import_png;
pub use xlib::{save_xlib, load_xlib, library_hash, verify_dependency, verify_document_libraries, IntegrityStatus};
pub use reliability::*;
pub use v2::{load_checked, load_x_any, save_x_v2, load_x_lenient, validate, list_pages, load_page, migrate_v1_to_v2, DocumentV2, Metadata, Issue, SCHEMA_VERSION};

pub const X_FORMAT_VERSION: u32 = 1;
