//! Graphite & Signal — chrome tokens only. No raw hex in panels.
//! This is the single source of truth for chrome color; chrome.rs's
//! Palette struct is built FROM these constants (see Palette::default),
//! so there is only ever one accent value in the whole app.

use vello::peniko::Color;

// Surfaces
pub const C_BASE: Color = Color::from_rgb8(0x11, 0x13, 0x18);
pub const C_CANVAS: Color = Color::from_rgb8(0x0e, 0x10, 0x14);
pub const C_PANEL: Color = Color::from_rgb8(0x17, 0x1a, 0x20);
pub const C_RAISED: Color = Color::from_rgb8(0x1d, 0x21, 0x29);
pub const C_HOVER: Color = Color::from_rgb8(0x25, 0x2b, 0x38);
pub const C_ACTIVE: Color = Color::from_rgb8(0x2c, 0x34, 0x44);
pub const C_FIELD: Color = Color::from_rgb8(0x10, 0x12, 0x18);

// Borders
pub const C_EDGE: Color = Color::from_rgb8(0x2b, 0x30, 0x3a);
pub const C_EDGE_2: Color = Color::from_rgb8(0x3a, 0x44, 0x58);

// Text
pub const C_TEXT: Color = Color::from_rgb8(0xf1, 0xf3, 0xf7);
pub const C_DIM: Color = Color::from_rgb8(0xa1, 0xa8, 0xb5);
pub const C_FAINT: Color = Color::from_rgb8(0x68, 0x71, 0x81);
pub const C_ON_ACCENT: Color = Color::from_rgb8(0x04, 0x10, 0x16);

// Accent — Electric blue. This is the ONE accent in chrome (selection,
// focus, primary actions) — do not introduce a second brand color.
pub const C_ACCENT: Color = Color::from_rgb8(0x00, 0x99, 0xff);
pub const C_ACCENT_HOV: Color = Color::from_rgb8(0x33, 0xad, 0xff);
pub const C_ACCENT_MUTED: Color = Color::from_rgba8(0x00, 0x99, 0xff, 0x2b);
pub const C_DANGER: Color = Color::from_rgb8(0xf8, 0x71, 0x71);
pub const C_SELECTED: Color = Color::from_rgba8(0x00, 0x99, 0xff, 0x28);

// Geometry
pub const TITLE_H: f64 = 48.0;
pub const TOOL_W: f64 = 48.0;
pub const LEFT_W: f64 = 232.0;
pub const RIGHT_W: f64 = 296.0;
pub const STATUS_H: f64 = 24.0;
pub const ROW_H: f64 = 26.0;
pub const PAD: f64 = 12.0;
pub const RADIUS_SM: f64 = 6.0;
pub const RADIUS_MD: f64 = 8.0;
pub const RADIUS_LG: f64 = 10.0;

pub const FONT_CAPTION: f64 = 11.0;
pub const FONT_BODY: f64 = 12.0;
pub const FONT_TITLE: f64 = 14.0;
