#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Hand,
    Scale,
    Frame,
    Rectangle,
    Ellipse,
    Arc,
    Line,
    Polygon,
    Star,
    Text,
    Pen,
    Slice,
    Eyedropper,
    Pencil,
    Bucket,
    Brush,
}
impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Hand => "H",
            Tool::Scale => "K",
            Tool::Frame => "F",
            Tool::Rectangle => "R",
            Tool::Ellipse => "O",
            Tool::Arc => "⇧O",
            Tool::Line => "L",
            Tool::Polygon => "P",
            Tool::Star => "ST",
            Tool::Text => "T",
            Tool::Pen => "PEN",
            Tool::Slice => "S",
            Tool::Eyedropper => "I",
            Tool::Pencil => "⇧P",
            Tool::Bucket => "⇧I",
            Tool::Brush => "⇧B",
        }
    }
    pub const ALL: [Tool; 17] = [
        Tool::Select,
        Tool::Hand,
        Tool::Scale,
        Tool::Frame,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Arc,
        Tool::Line,
        Tool::Polygon,
        Tool::Star,
        Tool::Text,
        Tool::Pen,
        Tool::Slice,
        Tool::Eyedropper,
        Tool::Pencil,
        Tool::Bucket,
        Tool::Brush,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "MOVE",
            Tool::Hand => "HAND",
            Tool::Scale => "SCALE",
            Tool::Frame => "FRAME",
            Tool::Rectangle => "RECTANGLE",
            Tool::Ellipse => "ELLIPSE",
            Tool::Arc => "ARC",
            Tool::Line => "LINE",
            Tool::Polygon => "POLYGON",
            Tool::Star => "STAR",
            Tool::Text => "TEXT",
            Tool::Pen => "PEN",
            Tool::Slice => "SLICE",
            Tool::Eyedropper => "EYEDROPPER",
            Tool::Pencil => "PENCIL",
            Tool::Bucket => "PAINT BUCKET",
            Tool::Brush => "BRUSH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drag {
    None,
    Move {
        start: Point,
        cmds: usize,
    },
    Create {
        start_world: Point,
    },
    Marquee {
        start_world: Point,
        contained: bool,
    },
    Resize {
        corner: u8,
        start_world: Point,
        orig: (f64, f64, f64, f64),
        cmds: usize,
    }, // x,y,w,h
    /// Corner-radius drag. `corner` = which handle (0..3); `uniform` = all
    /// four corners scale together (Figma's default) vs per-corner override.
    Radius {
        corner: u8,
        uniform: bool,
        start_world: Point,
        orig: (f64, Option<[f64; 4]>),
        cmds: usize,
    },
    Rotate {
        center: Point,
        start_angle: f64,
        orig: f64,
        cmds: usize,
    },
    Pan {
        start: Point,
    },
    /// Scale tool: vertical drag scales the selected subtree.
    Scale {
        start_y: f64,
        applied: f64,
        cmds: usize,
    },
    /// Pencil freehand stroke in progress (points live on
    /// `App::pencil_pts` — Drag stays Copy).
    Pencil,
    /// Brush stroke in progress (points + per-point widths live on
    /// `App::brush_pts` / `App::brush_w` — Drag stays Copy).
    Brush,
    /// Click-drag inside an actively edited Text node: `anchor` is the byte
    /// offset where the press landed; the caret follows the cursor and the
    /// range between anchor and caret is selected.
    TextSelect {
        anchor: usize,
    },
    /// Direct manipulation of a selected fill gradient. handle 0/1 are the
    /// geometry endpoints; handle 2+n is stop n.
    Gradient {
        fill: usize,
        handle: usize,
        cmds: usize,
    },
    /// Vector-eraser drag: segments under the cursor accumulate in
    /// `App::eraser_hits` (target node = `App::node_edit`); erased on
    /// release as one undo step.
    Erase,
}

/// Text-input focus: either inline canvas text editing or a numeric
/// inspector field. Keyboard chars route here when active.
/// Which top-level experience is showing (standard lifecycle):
/// Dashboard (recent files) or the Editor for the open document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Editor,
}

/// One card on the dashboard: a persistent .x document on disk.
pub struct DashFile {
    pub path: String,
    pub name: String,
    pub modified: String,
    pub pages: usize,
    pub thumb: Option<Scene>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    None,
    /// Text editing in a Prototype-panel logic chip.
    /// field: 0 = SetVar "name = expr", 1 = Cond condition text,
    /// 2 = KeyDown key, 3 = then-branch SetVar, 4 = else-branch SetVar.
    Proto {
        node_id: String,
        index: usize,
        field: u8,
        buffer: String,
    },
    /// Editing an exposed variable's value in present mode.
    PresentVar {
        name: String,
        buffer: String,
    },
    /// LIBS-tab search box.
    LibSearch,
    /// Code Connect: link a node to its source (bindings["code"]).
    CodeRef {
        node_id: String,
        buffer: String,
    },
    /// editing the text CONTENT of a Text node; original kept for Esc-cancel;
    /// caret = byte index into buffer; sel_anchor = other end of the
    /// selection range (Shift+arrows / Ctrl+A), None = no selection
    TextNode {
        id: String,
        buffer: String,
        original: String,
        caret: usize,
        sel_anchor: Option<usize>,
    },
    /// editing X/Y/W/H (field 0..4) of the selected node
    Field {
        id: String,
        field: u8,
        buffer: String,
    },
    /// editing a component-property value on an instance (text / number /
    /// instance-swap); buffer holds the typed value, Enter commits, Esc cancels
    Prop {
        instance_id: String,
        prop_name: String,
        buffer: String,
    },
    /// editing a variant's property DEFAULT in the variant grid; Enter commits
    /// via set_prop_default, Esc cancels
    VariantProp {
        component: String,
        prop_name: String,
        buffer: String,
    },
    /// typing in the layers-panel search box (Sketch-style filter)
    LayerSearch,
    LayerRename {
        id: String,
        buffer: String,
    },
    /// typing in the inspector font browser search box
    FontSearch,
    /// typing in the styles-section search box
    StyleSearch,
    /// renaming a style (management row); buffer holds the new name
    StyleRename {
        from: String,
        buffer: String,
    },
    /// typing in the asset-browser search box
    AssetSearch,
    /// renaming an asset (display name only, id stays content-derived)
    AssetRename {
        id: String,
        buffer: String,
    },
    /// renaming a page (double-click its row); Enter commits, Esc cancels
    PageRename {
        idx: usize,
        buffer: String,
    },
    /// dashboard: typing in the file search box
    DashSearch,
    /// dashboard: renaming a file card (display name in metadata)
    DashRename {
        path: String,
        buffer: String,
    },
}

/// Brush dynamics: full stroke widths in px; speed maps [0, 12] screen
/// px/event onto [WMAX, WMIN] (faster = thinner), EMA-smoothed.
pub const BRUSH_WMIN: f64 = 1.5;
pub const BRUSH_WMAX: f64 = 9.0;

pub struct App {
    pub editor: Editor,
    pub vars: Variables,
    pub tool: Tool,
    pub polygon_sides: usize,
    /// arc tool defaults: start/end sweep in degrees
    pub arc_start: f64,
    pub arc_end: f64,
    pub star_points: usize,
    pub star_inner_ratio: f64,
    pub rect_radius: f64,
    pub gradient_stop: usize,
    pub gradient_editing: bool,
    pub fill_layer_index: usize,
    pub stroke_layer_index: usize,
    pub effect_layer_index: usize,
    /// advanced-stroke popover (caps/join/dash/miter) open over the inspector
    pub stroke_advanced_open: bool,
    /// macOS Retina scale factor (logical -> physical pixel ratio); 1.0 on
    /// standard-DPI and non-mac platforms. All layout uses LOGICAL coords;
    /// the render path scales the scene by this at draw time.
    pub scale_factor: f64,
    /// unsaved-changes close dialog is showing (Save / Don't Save / Cancel)
    pub pending_close: bool,
    /// set true once the user resolves the close dialog with Save/Don't Save;
    /// the event loop exits on the next frame.
    pub exit_requested: bool,
    /// live prototype share dialog open (link + copy + permission toggle)
    pub share_open: bool,
    /// share permission: true = "Anyone with the link", false = "Only you"
    pub share_public: bool,
    pub pan: (f64, f64),
    pub zoom: f64,
    pub cursor: Point,
    pub drag: Drag,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// alt was held at drag start -> duplicate then move (Figma Alt+drag)
    pub alt_dupe_done: bool,
    pub status: String,
    /// raster export scale (@1x/@2x/@3x) for PNG/JPG
    pub export_scale: f64,
    pub created_count: usize,
    pub win_w: f64,
    pub win_h: f64,
    /// flattened (id, depth, kind_label) rows for the layers panel
    pub layer_rows: Vec<(String, String, usize, &'static str)>,
    pub focus: Focus,
    pub last_click: std::time::Instant,
    pub last_click_pos: Point,
    /// Phase 6.5: all pages; `page_idx` is the one loaded in the editor.
    pub pages: Vec<Node>,
    pub page_idx: usize,
    /// Phase 8: presentation mode. When Some, canvas renders a playback
    /// frame instead of the editor; transitions smart-animate between pages.
    pub present: Option<Present>,
    /// device frame (bezel) drawn around the prototype in present mode
    pub present_device: DeviceFrame,
    /// smart guides found during the current move drag (world coords)
    pub guides: Vec<(bool, f64)>,
    /// Phase 5.2: component pending placement — next canvas click stamps it
    pub stamping: Option<String>,
    /// hover highlight target (Select tool, nothing dragging)
    pub hover: Option<String>,
    /// layers panel scroll offset (rows)
    pub layers_scroll: usize,
    /// minimap hide interface (Ctrl+.)
    pub chrome_hidden: bool,
    /// rulers on/off (Shift+R in standard)
    pub rulers: bool,
    /// user guides in page coords: (vertical?, coord)
    pub user_guides: Vec<(bool, f64)>,
    /// outline view (Ctrl+Y): strokes only, no fills
    pub outline_view: bool,
    /// right-sidebar tab: 0 = Design, 1 = Prototype (properties panel)
    pub inspector_tab: u8,
    /// Inspect-tab code language: 0 = CSS, 1 = SwiftUI, 2 = Jetpack Compose.
    pub inspect_lang: u8,
    /// "?" shortcuts overlay
    pub help_open: bool,
    /// spacebar held -> temporary hand tool (standard)
    pub space_pan: bool,
    /// minimap layer list filter
    pub layer_filter: String,
    /// decoded image assets (GPU-side cache; Phase 4.2)
    pub assets: x_native::Assets,
    /// document-level content-addressed asset manager (asset:// ids);
    /// embedded records persist inside .x — the render cache above is a
    /// decoded view of this store plus legacy assets/ files
    pub store: x_native::AssetStore,
    /// real typography (P0): system TTFs via x-text FontManager
    pub fonts: x_native::text::FontManager,
    /// retained UI: right-click context menu (x-ui)
    pub ctx_menu: x_native::ui::Menu,
    /// retained UI: delayed tooltip state (x-ui)
    pub tooltip: x_native::ui::TooltipState,
    /// app start instant for tooltip timing
    pub t0: std::time::Instant,
    /// enumerated system font database (P0 fonts)
    pub sysfonts: x_native::text::SystemFonts,
    /// google fonts client (disk-cached)
    pub gfonts: x_native::text::GoogleFonts,
    /// font browser: query, scroll offset, resolved results
    pub font_query: String,
    pub font_scroll: usize,
    pub font_results: Vec<(String, FontSource)>,
    /// weights offered for the last applied google family
    pub font_weights: Vec<(String, u32, bool)>,
    /// pen tool: id of the vector node being drawn (None until first click)
    pub pen_target: Option<String>,
    /// pen tool: (anchor idx just placed, gesture undo depth) while the
    /// mouse button is still held after that click — a drag during this
    /// window pulls a Figma-style curve handle instead of a corner point.
    pub pen_placing: Option<(usize, Point, usize)>,
    /// pen tool: outgoing handle offset (world-space delta from the last
    /// anchor) pulled out by that anchor's placement drag; consumed as the
    /// c1 control point when the NEXT anchor is added, so a dragged point
    /// continues its tangent smoothly into the following segment.
    pub pen_pending_out: Option<(f64, f64)>,
    /// node-edit mode: vector id whose anchors are shown/editable
    pub node_edit: Option<String>,
    /// anchor being dragged: (anchor index, gesture undo depth)
    pub anchor_drag: Option<(usize, usize)>,
    /// bezier handle being dragged: (anchor idx, outgoing?, undo depth)
    pub handle_drag: Option<(usize, bool, usize)>,
    /// vector-edit eraser (Shift+E, Figma Draw parity): while on, a drag
    /// across path segments collects them and erases them on release.
    pub vector_eraser: bool,
    /// end-anchor indices of segments collected during the eraser drag
    pub eraser_hits: Vec<usize>,
    /// named reusable styles (Figma paint/text/effect styles), persisted in .x
    pub styles: std::collections::HashMap<String, x_native::Style>,
    /// styles browser: filter query + selected style (management target)
    pub style_query: String,
    /// LIBS-tab search filter (components/styles/variables across libraries).
    pub lib_query: String,
    /// World-space points of the pencil stroke being drawn.
    pub pencil_pts: Vec<(f64, f64)>,
    /// Brush stroke: world-space points + parallel full widths (px).
    pub brush_pts: Vec<(f64, f64)>,
    pub brush_w: Vec<f64>,
    pub style_sel: Option<String>,
    /// asset browser overlay (Shift+A): open flag, filter, selection
    pub asset_browser: bool,
    pub asset_query: String,
    pub asset_sel: Option<String>,
    /// document library state: pinned deps + snapshots (persisted in .x)
    pub library_deps: Vec<x_native::LibraryDependency>,
    pub library_snapshots: std::collections::HashMap<String, x_native::Library>,
    /// a newer .xlib detected on disk, awaiting review: (dep idx, newer lib, changes)
    pub library_update: Option<(usize, x_native::Library, Vec<x_native::LibraryChange>)>,
    /// review overlay open?
    pub library_review: bool,
    /// integrity results from the last load (library_id -> ok?)
    pub library_integrity: Vec<(String, String)>,
    /// reliability: unsaved-changes flag + last autosave instant
    pub dirty_since_save: bool,
    pub last_autosave: std::time::Instant,
    /// frame-time instrumentation: rolling last-64 frame durations (ms)
    pub frame_times: std::collections::VecDeque<f32>,
    /// show the fps/frame-time HUD (Ctrl+Shift+F)
    pub perf_hud: bool,
    /// asset browser scroll row + sort mode (0 name, 1 size, 2 usage)
    pub asset_scroll: usize,
    pub asset_sort: u8,
    /// asset browser drag-to-canvas: asset id being dragged
    pub asset_drag: Option<String>,
    /// undo depth at last clean save (dirty detection)
    pub saved_undo_depth: usize,
    /// incremental render: dirty-subtree frame cache (skips lowering AND
    /// encoding for unchanged subtrees/frames)
    pub scene_cache: x_native::FrameCache,
    /// per-phase timings for the HUD: (ir_ms, encode_ms, chrome_ms)
    pub phase_ms: (f32, f32, f32),
    /// scene-cache hit flag for the HUD
    pub encode_skipped: bool,
    /// layer-rows rebuild fingerprint (skip identical walks)
    pub layer_rows_fp: Option<(usize, String, usize)>,
    /// staged import awaiting preview-accept: (source, doc, report)
    pub import_pending: Option<(String, Document, x_native::fileio::ImportReport)>,
    /// last command's latency (name, ms) for the HUD/status
    pub last_cmd: Option<(String, f32)>,
    /// left panel tab: 0 Layers, 1 Assets, 2 Components, 3 Library (mockup)
    pub left_tab: u8,
    /// open header dropdown menu (index into MENU_TITLES), None = closed
    pub menu_open: Option<usize>,
    /// minimap overlay (View menu; mockup hides it, so default OFF)
    pub minimap: bool,
    /// dashboard vs editor (standard Home -> file -> editor lifecycle)
    pub screen: Screen,
    /// dashboard cards (scanned from document.x + files/*.x)
    pub dash_files: Vec<DashFile>,
    /// path of the OPEN document (was the DOC_PATH const; now per-file)
    pub doc_path: String,
    /// bottom pages panel collapsed (persisted in .xprefs)
    pub thumbs_collapsed: bool,
    /// last mouse_down was a double-click (for row double-click actions)
    pub dbl: bool,
    /// dashboard search query (filters cards)
    pub dash_query: String,
    /// dashboard context-menu target: file path the menu acts on
    pub dash_ctx_path: Option<String>,
}

/// Device frame shown around a prototype in present mode (Figma's device
/// frames). `None` presents bare; others draw a bezel around the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFrame {
    None,
    Phone,
    Tablet,
    Desktop,
}

impl DeviceFrame {
    pub fn next(self) -> Self {
        match self {
            DeviceFrame::None => DeviceFrame::Phone,
            DeviceFrame::Phone => DeviceFrame::Tablet,
            DeviceFrame::Tablet => DeviceFrame::Desktop,
            DeviceFrame::Desktop => DeviceFrame::None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            DeviceFrame::None => "None",
            DeviceFrame::Phone => "Phone",
            DeviceFrame::Tablet => "Tablet",
            DeviceFrame::Desktop => "Desktop",
        }
    }
    /// screen aspect ratio (w / h)
    pub fn aspect(self) -> f64 {
        match self {
            DeviceFrame::Phone => 390.0 / 844.0,
            DeviceFrame::Tablet => 4.0 / 3.0,
            DeviceFrame::Desktop => 16.0 / 10.0,
            DeviceFrame::None => 0.0,
        }
    }
    /// bezel thickness (in screen px, pre-fit)
    pub fn bezel(self) -> f64 {
        match self {
            DeviceFrame::Phone => 26.0,
            DeviceFrame::Tablet => 40.0,
            DeviceFrame::Desktop => 22.0,
            DeviceFrame::None => 0.0,
        }
    }
    pub fn corner(self) -> f64 {
        match self {
            DeviceFrame::Phone => 44.0,
            DeviceFrame::Tablet => 28.0,
            DeviceFrame::Desktop => 12.0,
            DeviceFrame::None => 0.0,
        }
    }
}

/// One interaction row in the Prototype panel (geometry shared painter/click).
pub struct ProtoRowUi {
    pub index: usize,
    pub trigger: Rect,
    pub action: Rect,
    pub dest: Rect,
    pub pos: Rect,
    pub anim: Rect,
    pub remove: Rect,
    /// Third-line chip (Cond else-branch / overlay+keydown key).
    pub extra: Rect,
}

/// Full Prototype-panel geometry for the selected node.
pub struct ProtoUi {
    pub id: String,
    pub start_toggle: Rect,
    pub add: Rect,
    pub rows: Vec<ProtoRowUi>,
}

pub struct Present {
    /// index of the page being shown
    pub current: usize,
    /// active transition: (from_idx, to_idx, started, duration_ms, anim)
    pub transition: Option<(usize, usize, std::time::Instant, u32, Animation)>,
    /// open overlays (page index + anchor position), topmost LAST
    pub overlays: Vec<(usize, OverlayPosition)>,
    /// navigation history for Back
    pub back_stack: Vec<usize>,
    /// last hovered node id (for hover/enter/leave triggers)
    pub hover: Option<String>,
    /// press start (node id + page-world point) for click/drag resolution
    pub press: Option<(String, Point)>,
    /// the active press has exceeded the drag threshold
    pub dragging: bool,
    /// armed AfterDelay events: (due, page_idx, action, ms, anim)
    pub delayed: Vec<(std::time::Instant, usize, Action, u32, Animation)>,
    /// scroll offsets per scrollable frame id (page px)
    pub scrolls: std::collections::HashMap<String, (f64, f64)>,
}

impl Present {
    pub fn new(current: usize) -> Self {
        Self {
            current,
            transition: None,
            overlays: vec![],
            back_stack: vec![],
            hover: None,
            press: None,
            dragging: false,
            delayed: vec![],
            scrolls: std::collections::HashMap::new(),
        }
    }
}

pub fn kind_label(n: &Node) -> &'static str {
    use x_native::NodeKind::*;
    match &n.kind {
        Frame { .. } => "FRAME",
        Group => "GROUP",
        Section => "SECTION",
        Rect { .. } => "RECT",
        Ellipse => "ELLIPSE",
        Arc { .. } => "ARC",
        Line => "LINE",
        Text { .. } => "TEXT",
        Image { .. } => "IMAGE",
        Vector { .. } => "VECTOR",
        Component { .. } => "COMP",
        Instance { .. } => "INST",
        Slice => "SLICE",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FontSource {
    System { family: String, style: String },
    Google { family: String, weight: u32 },
}
