// Design Tokens for X-Native
// Single source of truth for all UI values

// ============================================================================
// BRAND COLORS (Graphite & Violet)
// ============================================================================

pub const C_BG_PRIMARY: (f32, f32, f32) = (0.078, 0.082, 0.098);   // #141519
pub const C_BG_SECONDARY: (f32, f32, f32) = (0.106, 0.114, 0.137); // #1B1D23
pub const C_BG_TERTIARY: (f32, f32, f32) = (0.176, 0.188, 0.224);  // #2D3039
pub const C_BG_HOVER: (f32, f32, f32) = (0.22, 0.24, 0.28);        // Hover state
pub const C_BG_SELECTED: (f32, f32, f32) = (0.30, 0.15, 0.60);     // Violet selection

pub const C_ACCENT: (f32, f32, f32) = (0.486, 0.361, 0.988);       // #7C5CFC - X-Native Violet
pub const C_ACCENT_HOVER: (f32, f32, f32) = (0.55, 0.42, 1.0);     // Lighter violet
pub const C_ACCENT_PRESSED: (f32, f32, f32) = (0.40, 0.28, 0.85);  // Darker violet

pub const C_TEXT_PRIMARY: (f32, f32, f32) = (0.949, 0.953, 0.969); // #F2F3F7
pub const C_TEXT_SECONDARY: (f32, f32, f32) = (0.604, 0.620, 0.667); // #9A9EAA
pub const C_TEXT_DISABLED: (f32, f32, f32) = (0.35, 0.37, 0.42);    // Disabled text

pub const C_BORDER: (f32, f32, f32) = (0.18, 0.20, 0.24);          // Subtle borders
pub const C_BORDER_ACTIVE: (f32, f32, f32) = (0.30, 0.32, 0.38);   // Active borders

pub const C_SUCCESS: (f32, f32, f32) = (0.20, 0.80, 0.60);         // Success states
pub const C_WARNING: (f32, f32, f32) = (0.95, 0.75, 0.20);         // Warning states
pub const C_ERROR: (f32, f32, f32) = (0.95, 0.30, 0.30);           // Error states

// ============================================================================
// TYPOGRAPHY SCALE
// ============================================================================

pub const FONT_SIZE_XS: f64 = 8.0;    // Micro labels
pub const FONT_SIZE_SM: f64 = 10.0;   // Caption
pub const FONT_SIZE_MD: f64 = 12.0;   // Body/Label (default)
pub const FONT_SIZE_LG: f64 = 14.0;   // Section headers
pub const FONT_SIZE_XL: f64 = 16.0;   // Panel titles
pub const FONT_SIZE_2XL: f64 = 20.0;  // Modal titles

pub const FONT_WEIGHT_REGULAR: f32 = 400.0;
pub const FONT_WEIGHT_MEDIUM: f32 = 500.0;
pub const FONT_WEIGHT_SEMIBOLD: f32 = 600.0;

// Semantic text roles (to avoid hardcoded values)
#[derive(Clone, Copy)]
pub struct TextStyle {
    pub size: f64,
    pub weight: f32,
    pub color: (f32, f32, f32),
}

impl TextStyle {
    pub const MICRO: Self = Self { size: FONT_SIZE_XS, weight: FONT_WEIGHT_REGULAR, color: C_TEXT_SECONDARY };
    pub const CAPTION: Self = Self { size: FONT_SIZE_SM, weight: FONT_WEIGHT_REGULAR, color: C_TEXT_SECONDARY };
    pub const BODY: Self = Self { size: FONT_SIZE_MD, weight: FONT_WEIGHT_REGULAR, color: C_TEXT_PRIMARY };
    pub const LABEL: Self = Self { size: FONT_SIZE_MD, weight: FONT_WEIGHT_MEDIUM, color: C_TEXT_PRIMARY };
    pub const SECTION: Self = Self { size: FONT_SIZE_LG, weight: FONT_WEIGHT_SEMIBOLD, color: C_TEXT_PRIMARY };
    pub const TITLE: Self = Self { size: FONT_SIZE_XL, weight: FONT_WEIGHT_SEMIBOLD, color: C_TEXT_PRIMARY };
    pub const MODAL_TITLE: Self = Self { size: FONT_SIZE_2XL, weight: FONT_WEIGHT_SEMIBOLD, color: C_TEXT_PRIMARY };
}

// ============================================================================
// LAYOUT DIMENSIONS
// ============================================================================

pub const TOP_BAR_H: f64 = 48.0;
pub const BOTTOM_TOOLBAR_H: f64 = 56.0;
pub const NAV_RAIL_W: f64 = 52.0;
pub const LEFT_PANEL_MIN_W: f64 = 200.0;
pub const LEFT_PANEL_DEFAULT_W: f64 = 260.0;
pub const LEFT_PANEL_MAX_W: f64 = 400.0;
pub const INSPECTOR_MIN_W: f64 = 280.0;
pub const INSPECTOR_DEFAULT_W: f64 = 320.0;
pub const INSPECTOR_MAX_W: f64 = 440.0;

pub const ROW_HEIGHT_SM: f64 = 24.0;  // Compact lists
pub const ROW_HEIGHT_MD: f64 = 28.0;  // Default rows
pub const ROW_HEIGHT_LG: f64 = 32.0;  // Touch targets

// ============================================================================
// CORNER RADII
// ============================================================================

pub const RADIUS_NONE: f64 = 0.0;
pub const RADIUS_SM: f64 = 4.0;   // Small buttons, inputs
pub const RADIUS_MD: f64 = 6.0;   // Cards, panels
pub const RADIUS_LG: f64 = 8.0;   // Modals, large containers
pub const RADIUS_FULL: f64 = 999.0; // Pills, toggles

// ============================================================================
// SPACING SCALE (4px grid)
// ============================================================================

pub const SPACE_0: f64 = 0.0;
pub const SPACE_1: f64 = 4.0;
pub const SPACE_2: f64 = 6.0;
pub const SPACE_3: f64 = 8.0;
pub const SPACE_4: f64 = 12.0;
pub const SPACE_5: f64 = 16.0;
pub const SPACE_6: f64 = 20.0;
pub const SPACE_7: f64 = 24.0;
pub const SPACE_8: f64 = 32.0;

// ============================================================================
// BORDER WIDTHS
// ============================================================================

pub const BORDER_NONE: f64 = 0.0;
pub const BORDER_THIN: f64 = 1.0;
pub const BORDER_MED: f64 = 1.5;
pub const BORDER_THICK: f64 = 2.0;

// ============================================================================
// INTERACTION STATES
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
pub enum InteractionState {
    Default,
    Hover,
    Pressed,
    Selected,
    Disabled,
    Focus,
}

// ============================================================================
// ANIMATION TIMING
// ============================================================================

pub const ANIMATION_FAST: f64 = 0.15;   // 150ms - micro interactions
pub const ANIMATION_NORMAL: f64 = 0.25; // 250ms - standard transitions
pub const ANIMATION_SLOW: f64 = 0.35;   // 350ms - major state changes

// ============================================================================
// ACCESSIBILITY
// ============================================================================

pub const FOCUS_RING_WIDTH: f64 = 2.0;
pub const FOCUS_RING_COLOR: (f32, f32, f32) = C_ACCENT;
pub const MIN_TOUCH_TARGET: f64 = 32.0; // WCAG recommendation

// ============================================================================
// SCROLLBAR STYLING
// ============================================================================

pub const SCROLLBAR_W: f64 = 10.0;
pub const SCROLLBAR_THUMB_DEFAULT: (f32, f32, f32) = (0.30, 0.32, 0.38);
pub const SCROLLBAR_THUMB_HOVER: (f32, f32, f32) = (0.40, 0.42, 0.48);
pub const SCROLLBAR_TRACK: (f32, f32, f32) = (0.08, 0.09, 0.11);

// ============================================================================
// SHADOWS (for overlays/modals)
// ============================================================================

pub const SHADOW_SMALL: (f64, f64, f64, f32) = (0.0, 2.0, 8.0, 0.15);  // x, y, blur, alpha
pub const SHADOW_MEDIUM: (f64, f64, f64, f32) = (0.0, 4.0, 16.0, 0.20);
pub const SHADOW_LARGE: (f64, f64, f64, f32) = (0.0, 8.0, 32.0, 0.25);
