//! X-Native Design Tokens
//! 
//! Single source of truth for all visual properties
//! Enforces consistency across the entire application

// ============================================================================
// BRAND COLORS - Graphite & Violet Identity
// ============================================================================

/// Primary background (darkest)
pub const C_BG: egui::Color32 = egui::Color32::from_rgb(20, 21, 25);      // #141519
/// Secondary background (panels)
pub const C_BG_SECONDARY: egui::Color32 = egui::Color32::from_rgb(27, 29, 35); // #1B1D23
/// Tertiary background (inputs, fields)
pub const C_BG_TERTIARY: egui::Color32 = egui::Color32::from_rgb(45, 48, 57);   // #2D3039
/// Hover background
pub const C_HOVERBG: egui::Color32 = egui::Color32::from_rgb(60, 63, 73);       // #3C3F49
/// Selected background
pub const C_SELECTED: egui::Color32 = egui::Color32::from_rgb(70, 73, 85);      // #464955

/// X-Native Violet accent
pub const C_ACCENT: egui::Color32 = egui::Color32::from_rgb(124, 92, 252);     // #7C5CFC
/// Accent hover
pub const C_ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(108, 78, 232); // #6C4EE8
/// Accent pressed
pub const C_ACCENT_PRESSED: egui::Color32 = egui::Color32::from_rgb(92, 64, 212); // #5C40D4

// ============================================================================
// TEXT COLORS - Hierarchical System
// ============================================================================

/// Primary text (headings, labels)
pub const C_TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(242, 243, 247);  // #F2F3F7
/// Secondary text (descriptions, captions)
pub const C_TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(154, 158, 170); // #9A9EAA
/// Disabled text
pub const C_TEXT_DISABLED: egui::Color32 = egui::Color32::from_rgb(90, 93, 105);    // #5A5D69
/// Error text
pub const C_TEXT_ERROR: egui::Color32 = egui::Color32::from_rgb(255, 107, 107);     // #FF6B6B
/// Success text
pub const C_TEXT_SUCCESS: egui::Color32 = egui::Color32::from_rgb(82, 204, 150);    // #52CC96

// ============================================================================
// BORDER & DIVIDER COLORS
// ============================================================================

/// Subtle borders (panel dividers)
pub const C_BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(50, 53, 62);    // #32353E
/// Standard borders (inputs, buttons)
pub const C_BORDER: egui::Color32 = egui::Color32::from_rgb(70, 73, 85);          // #464955
/// Strong borders (focus states, active elements)
pub const C_BORDER_STRONG: egui::Color32 = egui::Color32::from_rgb(124, 92, 252);  // #7C5CFC
/// Focus ring color
pub const C_FOCUS: egui::Color32 = egui::Color32::from_rgba_unmultiplied(124, 92, 252, 100); // #7C5CFC with alpha

// ============================================================================
// TYPOGRAPHY SCALE - Semantic Roles
// ============================================================================

/// Micro text (8px) - icons, tiny labels
pub const FONT_SIZE_MICRO: f32 = 8.0;
/// Caption text (10px) - helper text, metadata
pub const FONT_SIZE_CAPTION: f32 = 10.0;
/// Body text (12px) - standard UI text
pub const FONT_SIZE_BODY: f32 = 12.0;
/// Label text (14px) - button labels, field labels
pub const FONT_SIZE_LABEL: f32 = 14.0;
/// Section text (16px) - section headers
pub const FONT_SIZE_SECTION: f32 = 16.0;
/// Title text (20px) - panel titles, modal headers
pub const FONT_SIZE_TITLE: f32 = 20.0;

// Font families
pub const FONT_FAMILY_PROPORTIONAL: &str = "Inter";
pub const FONT_FAMILY_MONOSPACE: &str = "JetBrains Mono";

// ============================================================================
// LAYOUT DIMENSIONS - Consistent Spacing System
// ============================================================================

// --- Panel Dimensions ---
/// Top bar height (reduced from 72px)
pub const TOP_BAR_H: f32 = 48.0;
/// Bottom toolbar height
pub const TOOLBAR_H: f32 = 56.0;
/// Left panel default width
pub const LEFT_PANEL_W: f32 = 260.0;
/// Right inspector default width (increased from 280px)
pub const INSPECTOR_W_DEFAULT: f32 = 320.0;
/// Inspector minimum width
pub const INSPECTOR_W_MIN: f32 = 280.0;
/// Inspector maximum width
pub const INSPECTOR_W_MAX: f32 = 440.0;
/// Navigation rail width
pub const NAV_RAIL_W: f32 = 56.0;

// --- Spacing Scale (4px grid) ---
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 20.0;
pub const SPACE_6: f32 = 24.0;
pub const SPACE_7: f32 = 28.0;
pub const SPACE_8: f32 = 32.0;

// --- Row Heights ---
/// Standard row height (increased from 22px)
pub const ROW_HEIGHT_STD: f32 = 28.0;
/// Compact row height
pub const ROW_HEIGHT_COMPACT: f32 = 24.0;
/// Tall row height (for thumbnails)
pub const ROW_HEIGHT_TALL: f32 = 48.0;

// --- Corner Radii ---
/// Small radius (buttons, inputs)
pub const RADIUS_SM: f32 = 4.0;
/// Medium radius (panels, cards)
pub const RADIUS_MD: f32 = 6.0;
/// Large radius (modals, overlays)
pub const RADIUS_LG: f32 = 8.0;

// --- Border Widths ---
pub const BORDER_THIN: f32 = 1.0;
pub const BORDER_MEDIUM: f32 = 1.5;
pub const BORDER_THICK: f32 = 2.0;

// ============================================================================
// INTERACTION STATES
// ============================================================================

/// Hover state modifier (lighten/darken factor)
pub const HOVER_FACTOR: f32 = 0.1;
/// Pressed state modifier
pub const PRESSED_FACTOR: f32 = 0.2;
/// Disabled opacity
pub const DISABLED_OPACITY: f32 = 0.5;
/// Focus ring width
pub const FOCUS_WIDTH: f32 = 2.0;

// ============================================================================
// ANIMATION TIMING
// ============================================================================

/// Fast animation (hover states)
pub const ANIM_FAST: f32 = 0.15;
/// Normal animation (panel transitions)
pub const ANIM_NORMAL: f32 = 0.25;
/// Slow animation (modal fade-in)
pub const ANIM_SLOW: f32 = 0.4;

// ============================================================================
// ACCESSIBILITY
// ============================================================================

/// Minimum touch target size (WCAG)
pub const TOUCH_TARGET_MIN: f32 = 44.0;
/// Minimum contrast ratio (WCAG AA)
pub const CONTRAST_RATIO_AA: f32 = 4.5;
/// Minimum contrast ratio (WCAG AAA)
pub const CONTRAST_RATIO_AAA: f32 = 7.0;

// ============================================================================
// PERFORMANCE TARGETS
// ============================================================================

/// Target FPS for smooth UI
pub const TARGET_FPS: u32 = 120;
/// Maximum frame time budget
pub const FRAME_BUDGET_MS: f32 = 8.33; // 120 FPS

// ============================================================================
// NAVIGATION WORKFLOWS
// ============================================================================

pub const NAV_ITEMS: &[(&str, &str)] = &[
    ("◇", "Files"),
    ("◆", "Assets"),
    ("◈", "Components"),
    ("◎", "Variables"),
    ("◌", "Styles"),
    ("⌁", "Libraries"),
    ("⚙", "Settings"),
];

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Apply hover effect to a color
pub fn apply_hover(color: egui::Color32) -> egui::Color32 {
    let factor = HOVER_FACTOR;
    egui::Color32::from_rgb(
        (color.r() as f32 * (1.0 + factor)).min(255.0) as u8,
        (color.g() as f32 * (1.0 + factor)).min(255.0) as u8,
        (color.b() as f32 * (1.0 + factor)).min(255.0) as u8,
    )
}

/// Apply pressed effect to a color
pub fn apply_pressed(color: egui::Color32) -> egui::Color32 {
    let factor = PRESSED_FACTOR;
    egui::Color32::from_rgb(
        (color.r() as f32 * (1.0 - factor)).max(0.0) as u8,
        (color.g() as f32 * (1.0 - factor)).max(0.0) as u8,
        (color.b() as f32 * (1.0 - factor)).max(0.0) as u8,
    )
}

/// Check if color meets WCAG AA contrast ratio against background
pub fn meets_contrast_aa(foreground: egui::Color32, background: egui::Color32) -> bool {
    // Simplified check - full implementation would calculate luminance
    let diff = (foreground.r() as i32 - background.r() as i32).abs()
             + (foreground.g() as i32 - background.g() as i32).abs()
             + (foreground.b() as i32 - background.b() as i32).abs();
    diff > 100 // Rough approximation
}
