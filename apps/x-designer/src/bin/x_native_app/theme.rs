//! Graphite & Signal — chrome tokens only. No raw hex in panels.

use vello::peniko::Color;

// Surfaces
pub const C_BASE: Color = Color::from_rgb8(0x0c, 0x0e, 0x12);
pub const C_CANVAS: Color = Color::from_rgb8(0x12, 0x15, 0x1c);
pub const C_PANEL: Color = Color::from_rgb8(0x16, 0x1a, 0x22);
pub const C_RAISED: Color = Color::from_rgb8(0x1c, 0x22, 0x2d);
pub const C_HOVER: Color = Color::from_rgb8(0x25, 0x2b, 0x38);
pub const C_ACTIVE: Color = Color::from_rgb8(0x2c, 0x34, 0x44);
pub const C_FIELD: Color = Color::from_rgb8(0x10, 0x13, 0x1a);

// Borders
pub const C_EDGE: Color = Color::from_rgb8(0x2a, 0x31, 0x40);
pub const C_EDGE_2: Color = Color::from_rgb8(0x3a, 0x44, 0x58);

// Text
pub const C_TEXT: Color = Color::from_rgb8(0xe8, 0xeb, 0xf2);
pub const C_DIM: Color = Color::from_rgb8(0x9a, 0xa3, 0xb5);
pub const C_FAINT: Color = Color::from_rgb8(0x6b, 0x73, 0x85);
pub const C_ON_ACCENT: Color = Color::from_rgb8(0x04, 0x10, 0x16);

// Signal accent
pub const C_ACCENT: Color = Color::from_rgb8(0x2d, 0xd4, 0xbf);
pub const C_ACCENT_HOV: Color = Color::from_rgb8(0x5e, 0xea, 0xd4);
pub const C_ACCENT_MUTED: Color = Color::from_rgba8(0x2d, 0xd4, 0xbf, 0x33);
pub const C_DANGER: Color = Color::from_rgb8(0xf8, 0x71, 0x71);
pub const C_SELECTED: Color = Color::from_rgba8(0x2d, 0xd4, 0xbf, 0x28);

// Geometry
pub const TITLE_H: f64 = 36.0;
pub const TOOL_W: f64 = 48.0;
pub const LEFT_W: f64 = 240.0;
pub const RIGHT_W: f64 = 260.0;
pub const STATUS_H: f64 = 24.0;
pub const ROW_H: f64 = 26.0;
pub const PAD: f64 = 12.0;
pub const RADIUS_SM: f64 = 4.0;
pub const RADIUS_MD: f64 = 6.0;
pub const RADIUS_LG: f64 = 8.0;

pub const FONT_CAPTION: f64 = 11.0;
pub const FONT_BODY: f64 = 12.0;
pub const FONT_TITLE: f64 = 14.0;
