//! X-Native — FINAL v45 UI (`ui/dashboard-v2.html` + `ui/v45-final-editor-28px.html`).

mod board_ui;
mod chrome;
mod clipboard;
mod command;
mod context_menu;
mod dashboard;
mod editor_ui;
mod fonts;
mod gpu_target;
mod icons;
mod jobs;
mod loading;
mod paint;
mod run;
mod session;
mod state;
mod text_session;
mod theme;

fn main() {
    run::run();
}
