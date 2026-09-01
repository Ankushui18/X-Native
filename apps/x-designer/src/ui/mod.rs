// X-Native UI Module Architecture
// Phase 1: Modularize monolithic chrome.rs and app.rs

pub mod shell;        // Main application shell layout
pub mod topbar;       // Top header bar
pub mod toolbar;      // Bottom floating toolbar
pub mod nav_rail;     // Left navigation rail (Files, Assets, Components, etc.)
pub mod layers;       // Layers panel
pub mod assets;       // Assets browser
pub mod components;   // Components library
pub mod variables;    // Variables workspace
pub mod styles;       // Styles panel
pub mod libraries;    // Libraries manager
pub mod inspector;    // Right properties inspector
pub mod overlays;     // Command palette, import preview, etc.
pub mod tokens;       // Design tokens and theme constants

// Re-export commonly used types
pub use shell::Shell;
pub use tokens::DesignTokens;
