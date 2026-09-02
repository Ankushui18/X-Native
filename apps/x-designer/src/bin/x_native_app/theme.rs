#[allow(unused_imports)]
use super::*;

pub const DOC_PATH: &str = "document.x";
pub const SVG_PATH: &str = "export.svg";

// ---- workspace layout constants ----
// Professional ergonomics with X-Native's own visual identity.
// Values are semantic and shared by paint + hit testing so density changes
// cannot create dead click areas.

pub const TOOLBAR_W: f64 = 0.0; // tools live in the header center now

// Panel dimensions (optimized for readability and workflow)
pub const LAYERS_W: f64 = 260.0;  // Left panel width (increased for better layer name visibility)
pub const INSPECTOR_W: f64 = 300.0; // Right panel - wider for better property editing UX

// Top bar: streamlined two-row header
pub const TAB_H: f64 = 28.0;   // Tab strip height
pub const TOP_H: f64 = 48.0;   // Total top bar (reduced from 72px for modern look)

// Bottom bar: minimal status area
pub const BOTTOM_BAR_H: f64 = 32.0; // Status bar
pub const THUMBS_H: f64 = 96.0;     // Page thumbnail strip
pub const STATUS_H: f64 = 24.0;     // Status text line

// Row heights for consistent spacing
pub const ROW_H: f64 = 28.0;  // Standard row height (increased for better touch targets and readability)

// X workspace palette: professional graphite surfaces with distinctive violet accent.
// This is X-Native's own visual identity - not a clone of any other tool.
// Contrast is deliberately stepped (panel -> raised -> hover -> field) so
// hierarchy remains legible on all displays without bright divider noise.

// Backgrounds (Graphite Series - X-Native signature look)
pub const C_PANEL: Color = Color::from_rgb8(0x14, 0x15, 0x19);  // Primary panel background
pub const C_PANEL2: Color = Color::from_rgb8(0x1b, 0x1d, 0x23); // Elevated surfaces
pub const C_PANEL_EDGE: Color = Color::from_rgb8(0x2d, 0x30, 0x39); // Subtle borders

// Text Hierarchy
pub const C_TEXT: Color = Color::from_rgb8(0xf2, 0xf3, 0xf7);  // Primary text
pub const C_DIM: Color = Color::from_rgb8(0x9a, 0x9e, 0xaa);   // Secondary text

// Brand Accent (X-Native Violet - our signature color)
pub const C_ACCENT: Color = Color::from_rgb8(0x7c, 0x5c, 0xfc); // Primary brand color

// Canvas & Interaction
pub const C_CANVAS: Color = Color::from_rgb8(0x22, 0x24, 0x2a); // Canvas background
pub const C_FIELD: Color = Color::from_rgb8(0x1e, 0x20, 0x27);   // Input fields (slightly darker for depth)

// Section headers
pub const C_SECTION: Color = Color::from_rgb8(0xd7, 0xd9, 0xe1);

// ---- inspector Design-tab section y-map (offsets from TOP_H) ----
// ONE map consumed by BOTH the chrome painter and click_inspector so the
// mockup sections can't drift from their hit-tests.
pub const IY_ALIGN: f64 = 24.0; // alignment icon row (mockup top strip)
pub const IY_NAME: f64 = 50.0; // (name now rides the Position header)
pub const IY_POS_HDR: f64 = 50.0; // "Position"
pub const IY_XY: f64 = 68.0; // X / Y field boxes
pub const IY_WH: f64 = 90.0; // W / H field boxes
pub const IY_ROT: f64 = 112.0; // rotation + type-transform boxes
pub const IY_AL_HDR: f64 = 140.0; // "Responsive Layout" header (+ chip)
pub const IY_APP_HDR: f64 = 232.0; // "Appearance" header
pub const IY_APP_ROW: f64 = 248.0; // opacity + corner radius fields
pub const IY_CORNERS: f64 = 272.0; // per-corner radii mini-boxes (rects)
pub const IY_FILL_HDR: f64 = 300.0; // "Fill" + GR toggle
pub const IY_FILLROW: f64 = 316.0; // swatch + hex + eye
pub const IY_PAL: f64 = 336.0; // palette swatch row (8 across)
pub const IY_STROKE_HDR: f64 = 358.0; // "Stroke"
pub const IY_STROKEROW: f64 = 374.0; // swatch + hex + width -/+ + INSIDE
pub const IY_FX_HDR: f64 = 398.0; // "Effects" + add buttons
pub const IY_FXROW: f64 = 414.0; // one row per effect (18px each)
pub const IY_SEC: f64 = 140.0; // Image / Component section (shares the Auto Layout slot — a node is never frame AND image)
pub const IY_CONSTRAINTS: f64 = 116.0; // constraints grid (INSPECT tab)
pub const IY_STYLES: f64 = 500.0; // styles browser below expanded effects
pub const IY_FONT: f64 = 500.0; // font browser (text nodes — replaces styles/constraints)
/// font browser rows visible (shared painter/click)
pub const FONT_ROWS: usize = 5;

/// left panel icon tabs (mockup): index == App::left_tab
pub const LEFT_TABS: [&str; 4] = ["Layers", "Assets", "Components", "Library"];
/// mockup: icon ABOVE label => taller tab strip
pub const LTAB_H: f64 = 54.0;
/// left panel search field (below the tab strip)
pub const LSEARCH_Y0: f64 = LTAB_H + 10.0;
pub const LSEARCH_Y1: f64 = LTAB_H + 32.0;
/// Pages section header + first page row (Layers tab)
pub const LPAGES_HDR: f64 = LTAB_H + 44.0;
pub const LPAGES_Y0: f64 = LTAB_H + 60.0;

/// header dropdown menus (REAL, not visual): (title, items);
/// item = (label, shortcut hint, action tag consumed by run_menu_tag)
/// (label, shortcut hint, action tag)
pub type MenuItemDef = (&'static str, &'static str, &'static str);

pub const MENUS: [(&str, &[MenuItemDef]); 7] = [
    ("File", &[
        ("New File", "", "file.new"),
        ("New Page", "", "file.new_page"),
        ("Open", "⌘O", "file.open"),
        ("Save", "⌘S", "file.save"),
        ("Save As...", "⇧⌘S", "file.save_as"),
        ("Import...", "⌘I", "file.import"),
        ("Import X Document...", "", "file.import_x"),
        ("Import Design JSON (Figma REST)...", "", "file.import_figma"),
        ("Import Sketch Package...", "", "file.import_sketch"),
        ("Export X Document...", "", "file.export_x"),
        ("Export Design JSON (Figma-compatible)...", "", "file.export_figma"),
        ("Export Sketch Package...", "", "file.export_sketch"),
        ("Export SVG", "⌘E", "file.export_svg"),
        ("Export PNG", "⌥⌘E", "file.export_png"),
        ("Export PDF", "⇧⌘E", "file.export_pdf"),
        ("Rename Page", "", "page.rename"),
        ("Duplicate Page", "", "page.duplicate"),
        ("Move Page Left", "", "page.left"),
        ("Move Page Right", "", "page.right"),
        ("Delete Page", "", "page.delete"),
        ("Back to Dashboard", "", "file.dashboard"),
    ]),
    ("Edit", &[
        ("Undo", "⌘Z", "edit.undo"),
        ("Redo", "⇧⌘Z", "edit.redo"),
        ("Cut", "⌘X", "edit.cut"),
        ("Copy", "⌘C", "edit.copy"),
        ("Paste", "⌘V", "edit.paste"),
        ("Duplicate", "⌘D", "edit.duplicate"),
        ("Delete", "DEL", "edit.delete"),
        ("Select All", "⌘A", "edit.select_all"),
        ("Copy as SVG", "", "edit.copy_svg"),
        ("Paste SVG from Clipboard", "", "edit.paste_svg"),
    ]),
    ("View", &[
        ("Zoom In", "", "view.zoom_in"),
        ("Zoom Out", "", "view.zoom_out"),
        ("Zoom 100%", "⌘0", "view.zoom_100"),
        ("Zoom to Fit", "⌘1", "view.zoom_fit"),
        ("Rulers", "SHIFT+R", "view.rulers"),
        ("Outline View", "⌘Y", "view.outline"),
        ("Pages Panel", "", "view.pages"),
        ("Variables", "", "view.vars"),
        ("Minimap", "", "view.minimap"),
        ("Perf HUD", "⇧⌘F", "view.hud"),
        ("Hide UI", "⌘.", "view.hide_ui"),
    ]),
    ("Object", &[
        ("Group", "⌘G", "obj.group"),
        ("Ungroup", "⇧⌘G", "obj.ungroup"),
        ("Bring to Front", "⌘]", "obj.front"),
        ("Send to Back", "⌘[", "obj.back"),
        ("Bring Forward", "]", "obj.forward"),
        ("Send Backward", "[", "obj.backward"),
        ("Union", "", "obj.union"),
        ("Subtract", "", "obj.subtract"),
        ("Intersect", "", "obj.intersect"),
        ("Exclude", "", "obj.exclude"),
        ("Use as Mask", "", "obj.mask"),
        ("Create Component", "⌥⌘K", "obj.component"),
    ]),
    ("Arrange", &[
        ("Align Left", "", "arr.left"),
        ("Align Center", "", "arr.centerh"),
        ("Align Right", "", "arr.right"),
        ("Align Top", "", "arr.top"),
        ("Align Middle", "", "arr.centerv"),
        ("Align Bottom", "", "arr.bottom"),
        ("Distribute Horizontal", "", "arr.disth"),
        ("Distribute Vertical", "", "arr.distv"),
        ("Flip Horizontal", "SHIFT+H", "arr.fliph"),
        ("Flip Vertical", "SHIFT+V", "arr.flipv"),
    ]),
    ("Plugins", &[
        ("No plugins installed", "", "noop"),
    ]),
    ("Help", &[
        ("Keyboard Shortcuts", "?", "help.shortcuts"),
    ]),
];

/// Common device frame presets with X-Native naming (name, w, h).
pub const FRAME_PRESETS: [(&str, f64, f64); 5] = [
    ("PHONE 390X844", 390.0, 844.0),
    ("TABLET 820X1180", 820.0, 1180.0),
    ("DESKTOP 1440X1024", 1440.0, 1024.0),
    ("WATCH 198X242", 198.0, 242.0),
    ("SLIDE 1920X1080", 1920.0, 1080.0),
];

pub const PALETTE: [Color; 8] = [
    Color::from_rgb8(0x7c, 0x5c, 0xfc),
    Color::from_rgb8(0xf2, 0x48, 0x22),
    Color::from_rgb8(0x2e, 0xcc, 0x71),
    Color::from_rgb8(0x9b, 0x59, 0xb6),
    Color::from_rgb8(0xff, 0xd7, 0x00),
    Color::from_rgb8(0xff, 0xff, 0xff),
    Color::from_rgb8(0x55, 0x55, 0x55),
    Color::from_rgb8(0x11, 0x11, 0x11),
];

// ---- X-Native Typography Scale (professional hierarchy) ----
// Standardized font sizes for consistent visual rhythm across the interface.
// Based on 1.25 major third scale for harmonious proportions.

pub const FONT_SIZE_XS: f64 = 8.0;    // Captions, metadata, micro labels
pub const FONT_SIZE_SM: f64 = 10.0;   // Secondary text, helper labels
pub const FONT_SIZE_MD: f64 = 12.0;   // Body text, input fields, default UI
pub const FONT_SIZE_LG: f64 = 14.0;   // Section headers, emphasis
pub const FONT_SIZE_XL: f64 = 16.0;   // Panel titles, important labels
pub const FONT_SIZE_2XL: f64 = 20.0;  // Logo, major headings, hero text

// Line height multipliers for readability
pub const LINE_HEIGHT_TIGHT: f64 = 1.2;    // Headings, compact layouts
pub const LINE_HEIGHT_NORMAL: f64 = 1.5;   // Body text, standard UI
pub const LINE_HEIGHT_RELAXED: f64 = 1.75; // Spacious layouts, reading mode

// ---- Enhanced Interaction States ----
// Complete state palette for accessible, polished interactions.

pub const C_HOVERBG: Color = Color::from_rgb8(0x2a, 0x2d, 0x36); // Hover states (subtle lift)
pub const C_PRESSED: Color = Color::from_rgb8(0x3a, 0x3d, 0x46); // Active/pressed states
pub const C_SELECTED: Color = Color::from_rgba8(0x7c, 0x5c, 0xfc, 35); // Selected items (increased from 20 alpha)
pub const C_FOCUS_RING: Color = Color::from_rgba8(0x7c, 0x5c, 0xfc, 180); // Keyboard focus indicator

// ---- Standardized Corner Radii ----
// Three radii for visual consistency across all UI elements.

pub const RADIUS_SM: f64 = 4.0;  // Small elements: chips, tags, tight spaces
pub const RADIUS_MD: f64 = 6.0;  // Medium elements: buttons, inputs, cards
pub const RADIUS_LG: f64 = 8.0;  // Large elements: panels, modals, toolbars

// ---- Border Weights ----
// Consistent stroke weights for clarity and hierarchy.

pub const BORDER_THIN: f64 = 1.0;   // Dividers, subtle separators
pub const BORDER_NORMAL: f64 = 1.5; // Input fields, interactive elements
pub const BORDER_THICK: f64 = 2.0;  // Focus rings, active states, emphasis

// ---- Padding Scale ----
// 4px base unit for consistent spacing throughout the interface.

pub const PAD_1: f64 = 4.0;   // Tight spacing, icon margins
pub const PAD_2: f64 = 8.0;   // Standard padding, button interiors
pub const PAD_3: f64 = 12.0;  // Comfortable spacing, panel margins
pub const PAD_4: f64 = 16.0;  // Generous padding, card interiors
pub const PAD_5: f64 = 24.0;  // Section spacing, modal padding
pub const PAD_6: f64 = 32.0;  // Hero spacing, large gaps

// ---- Animation Timing ----
// Professional motion durations for smooth, responsive interactions.

pub const ANIM_FAST: f64 = 100.0;   // Quick feedback: hover, toggle
pub const ANIM_NORMAL: f64 = 200.0; // Standard transitions: panel slides
pub const ANIM_SLOW: f64 = 300.0;   // Major state changes: modal open/close

// Minimum touch target size for accessibility (WCAG recommendation)
pub const MIN_TOUCH_TARGET: f64 = 44.0;
