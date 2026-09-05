//! X-Native Design System — Native Tokens
//! Complete production-ready design system — no external naming
//! Base #090909, Panel #111111, Canvas #060606, Accent #0099FF
//! Tokens: bg, panel, field, line, text, accent, radius, spacing

use vello::peniko::Color;

// Backgrounds
pub const C_BG: Color = Color::from_rgb8(0x09, 0x09, 0x09); // #090909
pub const C_CANVAS: Color = Color::from_rgb8(0x06, 0x06, 0x06); // #060606
pub const C_PANEL: Color = Color::from_rgb8(0x11, 0x11, 0x11); // #111111
pub const C_PANEL_2: Color = Color::from_rgb8(0x14, 0x14, 0x14); // #141414
pub const C_PANEL_3: Color = Color::from_rgb8(0x1C, 0x1C, 0x1C); // #1C1C1C
pub const C_RAISED: Color = Color::from_rgb8(0x1E, 0x1E, 0x1E); // #1E1E1E
pub const C_FIELD: Color = Color::from_rgb8(0x1A, 0x1A, 0x1A); // #1A1A1A
pub const C_FIELD_2: Color = Color::from_rgb8(0x22, 0x22, 0x22); // #222222
pub const C_HOVER: Color = Color::from_rgb8(0x25, 0x25, 0x25); // #252525
pub const C_ACTIVE: Color = Color::from_rgb8(0x2A, 0x2A, 0x2A); // #2A2A2A
pub const C_ACTIVE_2: Color = Color::from_rgb8(0x33, 0x33, 0x3E); // #33333E

// Borders
pub const C_LINE: Color = Color::from_rgb8(0x1F, 0x1F, 0x1F); // #1F1F1F
pub const C_LINE_2: Color = Color::from_rgb8(0x2A, 0x2A, 0x2A); // #2A2A2A
pub const C_LINE_3: Color = Color::from_rgb8(0x33, 0x33, 0x33); // #333333

// Text
pub const C_TEXT: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF); // #FFFFFF
pub const C_TEXT_2: Color = Color::from_rgb8(0xE5, 0xE5, 0xE5); // #E5E5E5
pub const C_MUTED: Color = Color::from_rgb8(0x99, 0x99, 0x99); // #999999
pub const C_DIM: Color = Color::from_rgb8(0x77, 0x77, 0x77); // #777777
pub const C_FAINT: Color = Color::from_rgb8(0x3A, 0x3A, 0x3A); // #3A3A3A
pub const C_ON_ACCENT: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);

// Accent — Native — Green #1BCB55 for logo, Yellow #FFEB3B avatar, Blue legacy #0099FF optional
pub const C_ACCENT: Color = Color::from_rgb8(0x00, 0x99, 0xFF);
pub const C_ACCENT_HOV: Color = Color::from_rgb8(0x33, 0xAD, 0xFF);
pub const C_ACCENT_DIM: Color = Color::from_rgba8(0x00, 0x99, 0xFF, 0x18);
pub const C_ACCENT_GREEN: Color = Color::from_rgb8(0x1B, 0xCB, 0x55); // #1BCB55 logo
pub const C_AVATAR: Color = Color::from_rgb8(0xFF, 0xEB, 0x3B); // #FFEB3B
pub const C_TEAM_L: Color = Color::from_rgb8(0x5B, 0x7C, 0xFF); // #5B7CFF
pub const C_TEAM_D: Color = Color::from_rgb8(0xFF, 0x7A, 0x45); // #FF7A45
pub const C_DRAFT_DOT: Color = Color::from_rgb8(0x2E, 0xCC, 0x71); // #2ECC71
pub const C_MD_BADGE: Color = Color::from_rgb8(0x51, 0x9A, 0xBA); // #519ABA
pub const C_DANGER: Color = Color::from_rgb8(0xFF, 0x55, 0x55);
pub const C_SELECTED: Color = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0x10);

// Legacy aliases for existing code
pub const C_BASE: Color = C_BG;
pub const C_EDGE: Color = C_LINE;
pub const C_EDGE_2: Color = C_LINE_2;

// Geometry — Production — Exact v22: no vertical tool rail, tools in bottom floating bar
pub const TITLE_H: f64 = 40.0;
pub const TOOL_W: f64 = 0.0; // v22 exact — no side rail, bottom bar only
pub const LEFT_W: f64 = 280.0;
pub const RIGHT_W: f64 = 320.0;
pub const STATUS_H: f64 = 24.0;
pub const ROW_H: f64 = 26.0;
pub const PAD: f64 = 12.0;
pub const RADIUS_SM: f64 = 6.0;
pub const RADIUS_MD: f64 = 8.0;
pub const RADIUS_LG: f64 = 10.0;

pub const FONT_CAPTION: f64 = 10.0;
pub const FONT_BODY: f64 = 11.0;
pub const FONT_TITLE: f64 = 12.0;
