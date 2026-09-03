//! X-Native Designer — greenfield UI shell (Graphite & Signal).
//!
//! Engine foundation: x-core / x-editor / x-render / x-format.
//! No inheritance from previous chrome.

mod paint;
mod shell;
mod state;
mod theme;

mod run;

fn main() {
    run::run();
}
