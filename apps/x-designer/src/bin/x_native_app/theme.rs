#[allow(unused_imports)]
use super::*;

pub const DOC_PATH: &str = "document.x";
pub const SVG_PATH: &str = "export.svg";

// ---- workspace layout constants (Design System v2 "Ink & Ember") ----
// Semantic and shared by paint + hit testing so density changes cannot
// create dead click areas. Window regions per the v2 map:

pub const TOOLBAR_W: f64 = 0.0; // tools live in the header center now

// Panel dimensions
pub const LAYERS_W: f64 = 264.0; // left panel total (incl. 44 rail)
pub const RAIL_W: f64 = 44.0; // left icon rail
pub const INSPECTOR_W: f64 = 288.0; // right panel

// Top: document tab strip + tool dock header
pub const TAB_H: f64 = 30.0; // document tab strip
pub const HDR_H: f64 = 42.0; // tool dock header row
pub const TOP_H: f64 = TAB_H + HDR_H; // 72

// Bottom bars
pub const BOTTOM_BAR_H: f64 = 28.0; // status bar
pub const THUMBS_H: f64 = 88.0; // page thumbnail strip
pub const STATUS_H: f64 = 24.0; // status text line

pub const ROW_H: f64 = 28.0; // rows everywhere
pub const PAD: f64 = 16.0; // panel horizontal padding

// =============================================================
// X-Native Design System v2 — "Ink & Ember" (dark, default)
// Cool ink ramp (hue ~222, low sat) + exactly ONE warm accent.
// Chrome paint and hit-testing consume these names, never raw hex.
// Full spec: X_NATIVE_DESIGN_SYSTEM.md (v2 supersedes v1 wholly).
// =============================================================

// ---- Surface ramp (6 steps, brightness-first hierarchy) ----
pub const C_BASE: Color = Color::from_rgb8(0x0e, 0x11, 0x17); // window backdrop
pub const C_CANVAS: Color = Color::from_rgb8(0x13, 0x17, 0x20); // artboard area
pub const C_PANEL: Color = Color::from_rgb8(0x18, 0x1d, 0x27); // header/tabs/panels
pub const C_PANEL2: Color = Color::from_rgb8(0x1f, 0x25, 0x31); // raised: menus, popovers
pub const C_HOVER: Color = Color::from_rgb8(0x26, 0x2d, 0x3b); // hover fill
pub const C_PRESSED: Color = Color::from_rgb8(0x2c, 0x34, 0x44); // pressed fill / active tool bg
pub const C_FIELD: Color = Color::from_rgb8(0x11, 0x15, 0x1d); // input wells (recessed)

// ---- Borders: two strengths only ----
pub const C_EDGE: Color = Color::from_rgb8(0x23, 0x2a, 0x38); // subtle
pub const C_EDGE_2: Color = Color::from_rgb8(0x30, 0x39, 0x49); // strong

// ---- Text: 4 levels ----
pub const C_TEXT: Color = Color::from_rgb8(0xea, 0xec, 0xf1); // primary
pub const C_DIM: Color = Color::from_rgb8(0xa8, 0xaf, 0xbf); // secondary + section headers
pub const C_FAINT: Color = Color::from_rgb8(0x7e, 0x86, 0x98); // tertiary (readable floor)
pub const C_OFF: Color = Color::from_rgb8(0x4a, 0x51, 0x60); // disabled

// ---- Accent: Ember — the only warm color in chrome ----
pub const C_ACCENT: Color = Color::from_rgb8(0xf9, 0x7b, 0x22); // base
pub const C_ACCENT_HOV: Color = Color::from_rgb8(0xff, 0x90, 0x49);
pub const C_ACCENT_PRS: Color = Color::from_rgb8(0xe1, 0x6a, 0x12);
pub const C_ACCENT_TXT: Color = Color::from_rgb8(0xff, 0xa2, 0x59); // accent as text / focus ring
pub const C_ON_ACCENT: Color = Color::from_rgb8(0x1a, 0x12, 0x06); // ink-on-ember (6.9:1)

// ---- Semantic (functional only) ----
pub const C_OK: Color = Color::from_rgb8(0x3e, 0xcf, 0x8e);
pub const C_WARN: Color = Color::from_rgb8(0xf5, 0xb8, 0x3d);
pub const C_DANGER: Color = Color::from_rgb8(0xf2, 0x54, 0x5b);
pub const C_INFO: Color = Color::from_rgb8(0x5c, 0xa8, 0xff);

// ---- Canvas & system (drawn on the artboard, not chrome) ----
pub const C_SELECT: Color = C_ACCENT; // selection stroke
pub const C_HOVERLN: Color = Color::from_rgba8(0xea, 0xec, 0xf1, 140); // hover outline ~55%
pub const C_SNAP: Color = Color::from_rgb8(0xff, 0x4d, 0x6d); // snap + ruler guides
pub const C_GRID: Color = Color::from_rgba8(0xea, 0xec, 0xf1, 36); // grid dots ~14%
pub const C_SHADOW: Color = Color::from_rgba8(0, 0, 0, 102); // raised shadow ~40%
pub const C_SHADOW2: Color = Color::from_rgba8(0, 0, 0, 128); // modal shadow ~50%
pub const C_SCRIM: Color = Color::from_rgba8(0, 0, 0, 128); // modal scrim

// ---- v1 compatibility aliases (same names, v2 values) ----
pub const C_PANEL_EDGE: Color = C_EDGE;
pub const C_HOVERBG: Color = C_HOVER; // row/item hover fill
pub const C_SECTION: Color = C_DIM; // section headers: C_DIM as 11px caps

/// Alternate fill for Alt (fully-contained) marquee — muted teal.
pub const C_SKEW_ALT: Color = Color::from_rgba8(0x2f, 0xc8, 0xb0, 26);
// ---- inspector Design-tab section y-map (offsets from TOP_H) ----
// ONE map consumed by BOTH the chrome painter and click_inspector so the
// mockup sections can't drift from their hit-tests.
pub const IY_ALIGN: f64 = 24.0; // alignment icon row (mockup top strip)
pub const IY_NAME: f64 = 50.0; // (name now rides the Position header)
pub const IY_POS_HDR: f64 = 50.0; // "Position"
pub const IY_XY: f64 = 68.0; // X / Y field boxes
pub const IY_WH: f64 = 90.0; // W / H field boxes
pub const IY_ROT: f64 = 112.0; // rotation + type-transform boxes
pub const IY_SKEW: f64 = 130.0; // skew ∠X / ∠Y stepper row
pub const IY_ORIGIN_HDR: f64 = 150.0; // "Transform origin" label
pub const IY_ORIGIN_GRID: f64 = 162.0; // 9-point transform-origin grid (3 rows x 14px)
pub const IY_CONSTRAINTS: f64 = 116.0; // constraints grid (INSPECT tab)
pub const IY_AL_HDR: f64 = 190.0; // "Responsive Layout" header (+ chip)
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
pub const IY_SEC: f64 = 140.0; // Image / Component section (shares the Auto Layout slot — a node is never frame AND image)pub const IY_CONSTRAINTS: f64 = 116.0; // constraints grid (INSPECT tab)
/// Inspect-tab section y-offsets (below the constraints grid).
pub const IY_CODE_HDR: f64 = 178.0; // "Code" section header
pub const IY_CODE_TABS: f64 = 194.0; // CSS / Swift / Compose language tabs
pub const IY_CODE_BOX: f64 = 212.0; // code snippet block
pub const IY_CODE_COPY: f64 = 356.0; // copy button
pub const IY_TOKENS_HDR: f64 = 378.0; // "Tokens" header
pub const IY_TOKENS_ROW: f64 = 394.0; // token rows (18px each)
pub const IY_MEASURE_HDR: f64 = 448.0; // "Measurements" header
pub const IY_MEASURE_ROW: f64 = 464.0; // measurement rows
pub const IY_GAP_HDR: f64 = 512.0; // "Hover gap" header
pub const IY_GAP_ROW: f64 = 528.0; // hovered-node gap rows
pub const IY_ASSETS_HDR: f64 = 548.0; // "Assets" header
pub const IY_ASSETS_ROW: f64 = 564.0; // referenced image/component rows
pub const IY_STYLES: f64 = 572.0; // styles browser below expanded effects
pub const IY_FONT: f64 = 572.0; // font browser (text nodes — replaces styles/constraints)
/// font browser rows visible (shared painter/click)
pub const FONT_ROWS: usize = 5;

/// left panel icon tabs (mockup): index == App::left_tab
pub const LEFT_TABS: [&str; 4] = ["Layers", "Assets", "Components", "Library"];
/// left rail height (v2: 44px rail; icon + compact label)
pub const LTAB_H: f64 = RAIL_W;
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
    (
        "File",
        &[
            ("New File", "", "file.new"),
            ("New Page", "", "file.new_page"),
            ("Open", "⌘O", "file.open"),
            ("Save", "⌘S", "file.save"),
            ("Save As...", "⇧⌘S", "file.save_as"),
            ("Import...", "⌘I", "file.import"),
            ("Import X Document...", "", "file.import_x"),
            (
                "Import Design JSON (Figma REST)...",
                "",
                "file.import_figma",
            ),
            ("Import Sketch Package...", "", "file.import_sketch"),
            ("Export X Document...", "", "file.export_x"),
            (
                "Export Design JSON (Figma-compatible)...",
                "",
                "file.export_figma",
            ),
            ("Export Sketch Package...", "", "file.export_sketch"),
            ("Export SVG", "⌘E", "file.export_svg"),
            ("Export PNG", "⌥⌘E", "file.export_png"),
            ("Export JPG", "", "file.export_jpg"),
            ("Export PNG @1x", "", "file.export_1x"),
            ("Export PNG @2x", "", "file.export_2x"),
            ("Export PNG @3x", "", "file.export_3x"),
            ("Export PDF", "⇧⌘E", "file.export_pdf"),
            ("Export Tokens (W3C)", "⇧⌘T", "file.export_tokens"),
            ("Batch Export...", "", "file.batch_export"),
            ("Share Prototype...", "", "file.share"),
            ("Rename Page", "", "page.rename"),
            ("Duplicate Page", "", "page.duplicate"),
            ("Move Page Left", "", "page.left"),
            ("Move Page Right", "", "page.right"),
            ("Delete Page", "", "page.delete"),
            ("Back to Dashboard", "", "file.dashboard"),
        ],
    ),
    (
        "Edit",
        &[
            ("Undo", "⌘Z", "edit.undo"),
            ("Redo", "⇧⌘Z", "edit.redo"),
            ("Cut", "⌘X", "edit.cut"),
            ("Copy", "⌘C", "edit.copy"),
            ("Paste", "⌘V", "edit.paste"),
            ("Paste Over Selection", "⇧⌘V", "edit.paste_over"),
            ("Paste to Replace", "⌥⇧⌘V", "edit.paste_replace"),
            ("Duplicate", "⌘D", "edit.duplicate"),
            ("Delete", "DEL", "edit.delete"),
            ("Select All", "⌘A", "edit.select_all"),
            ("Select Similar", "", "edit.select_similar"),
            ("Select Inside", "", "edit.select_inside"),
            ("Copy as SVG", "", "edit.copy_svg"),
            ("Paste SVG from Clipboard", "", "edit.paste_svg"),
        ],
    ),
    (
        "View",
        &[
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
        ],
    ),
    (
        "Object",
        &[
            ("Group", "⌘G", "obj.group"),
            ("Frame selection", "⌥⌘G", "obj.frame_selection"),
            ("Section Selection", "", "obj.section"),
            ("Tidy Up", "", "obj.tidy"),
            ("Ungroup", "⇧⌘G", "obj.ungroup"),
            ("Bring to Front", "⌘]", "obj.front"),
            ("Send to Back", "⌘[", "obj.back"),
            ("Bring Forward", "]", "obj.forward"),
            ("Send Backward", "[", "obj.backward"),
            ("Union", "", "obj.union"),
            ("Subtract", "", "obj.subtract"),
            ("Intersect", "", "obj.intersect"),
            ("Exclude", "", "obj.exclude"),
            ("Flatten Selection", "", "obj.flatten"),
            ("Outline Stroke", "", "obj.outline"),
            ("Use as Mask", "", "obj.mask"),
            ("Create Component", "⌥⌘K", "obj.component"),
            (
                "Detach Instance",
                "\u{21e7}\u{2318}D",
                "obj.detach_instance",
            ),
            ("Reset Overrides", "", "obj.reset_overrides"),
            ("Convert to Grid", "", "obj.to_grid"),
            ("Convert to Stack", "", "obj.to_stack"),
            ("Add Layout Grid", "", "obj.grid"),
        ],
    ),
    (
        "Arrange",
        &[
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
        ],
    ),
    ("Plugins", &[("No plugins installed", "", "noop")]),
    ("Help", &[("Keyboard Shortcuts", "?", "help.shortcuts")]),
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
    Color::from_rgb8(0xf9, 0x7b, 0x22), // Ember — brand first swatch
    Color::from_rgb8(0xf2, 0x48, 0x22),
    Color::from_rgb8(0x2e, 0xcc, 0x71),
    Color::from_rgb8(0x9b, 0x59, 0xb6),
    Color::from_rgb8(0xff, 0xd7, 0x00),
    Color::from_rgb8(0xff, 0xff, 0xff),
    Color::from_rgb8(0x55, 0x55, 0x55),
    Color::from_rgb8(0x11, 0x11, 0x11),
];

// ---- Typography scale (v2): only 9 sizes exist; 11px is the floor ----
pub const FONT_SIZE_XS: f64 = 11.0; // caption: kbd, meta, section headers
pub const FONT_SIZE_SM: f64 = 12.0; // secondary text, empty-state body
pub const FONT_SIZE_MD: f64 = 13.0; // body: menus, buttons, inputs, values
pub const FONT_SIZE_LG: f64 = 14.0; // emphasis in lists, dialog headers
pub const FONT_SIZE_XL: f64 = 16.0; // modal titles
pub const FONT_SIZE_2XL: f64 = 20.0; // empty-state / dashboard headlines

// Line height multipliers for readability
pub const LINE_HEIGHT_TIGHT: f64 = 1.2; // Headings, compact layouts
pub const LINE_HEIGHT_NORMAL: f64 = 1.5; // Body text, standard UI
pub const LINE_HEIGHT_RELAXED: f64 = 1.75; // Spacious layouts, reading mode

// ---- Interaction states (Ember tints; alpha over base, never mixed hex) ----
pub const C_SELECTED: Color = Color::from_rgba8(0xf9, 0x7b, 0x22, 36); // selected rows @ ~0.14
pub const C_FOCUS_RING: Color = Color::from_rgba8(0xff, 0xa2, 0x59, 180); // accent-as-text ring

// ---- Corner radii (v2): floating/containing things only ----
pub const R_XS: f64 = 3.0; // swatches, chips, kbd
pub const R_SM: f64 = 6.0; // buttons, inputs, icon buttons, rows, thumbs
pub const R_MD: f64 = 10.0; // menus, popovers, tooltips, palette, HUD, minimap
pub const R_LG: f64 = 14.0; // modals, dashboard cards
pub const R_XL: f64 = 20.0; // empty-state cards

// v1 names kept as aliases
pub const RADIUS_SM: f64 = R_SM;
pub const RADIUS_MD: f64 = R_MD;
pub const RADIUS_LG: f64 = R_LG;

// ---- Border Weights ----
// Consistent stroke weights for clarity and hierarchy.

pub const BORDER_THIN: f64 = 1.0; // Dividers, subtle separators
pub const BORDER_NORMAL: f64 = 1.5; // Input fields, interactive elements
pub const BORDER_THICK: f64 = 2.0; // Focus rings, active states, emphasis

// ---- Padding Scale ----
// 4px base unit for consistent spacing throughout the interface.

pub const PAD_1: f64 = 4.0; // Tight spacing, icon margins
pub const PAD_2: f64 = 8.0; // Standard padding, button interiors
pub const PAD_3: f64 = 12.0; // Comfortable spacing, panel margins
pub const PAD_4: f64 = 16.0; // Generous padding, card interiors
pub const PAD_5: f64 = 24.0; // Section spacing, modal padding
pub const PAD_6: f64 = 32.0; // Hero spacing, large gaps

// ---- Animation Timing ----
// Professional motion durations for smooth, responsive interactions.

pub const ANIM_FAST: f64 = 100.0; // Quick feedback: hover, toggle
pub const ANIM_NORMAL: f64 = 200.0; // Standard transitions: panel slides
pub const ANIM_SLOW: f64 = 300.0; // Major state changes: modal open/close

// Minimum touch target size for accessibility (WCAG recommendation)
pub const MIN_TOUCH_TARGET: f64 = 44.0;
