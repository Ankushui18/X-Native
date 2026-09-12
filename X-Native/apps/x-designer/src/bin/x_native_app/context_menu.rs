//! Context menu system for X-Native Designer.
//!
//! Shared right-click menu infrastructure used by:
//! - Canvas selection
//! - Layers rows
//! - Pages panel
//! - Inspector fields
//! - Tool rail
//!
//! Produces paint commands so it can plug into the existing
//! x_native_app/paint.rs + icons.rs rendering style.

// ═══════════════════════════════════════════════════════════
// Theme constants
// ═══════════════════════════════════════════════════════════

const MENU_WIDTH: f64 = 220.0;
const ROW_HEIGHT: f64 = 28.0;
const SEPARATOR_HEIGHT: f64 = 7.0;
const PADDING_X: f64 = 8.0;
const ICON_SIZE: f64 = 14.0;
const ICON_GAP: f64 = 8.0;
const SHORTCUT_WIDTH: f64 = 62.0;

// The menu is painted from the shared palette (crates/x-ui/src/design_system.rs)
// exactly like every other surface: these are role lookups, not a second set of
// literals. Packed 0xAARRGGBB is what the command list below carries.
const P: x_native::ui::ColorTokens = x_native::ui::ColorTokens::GRAPHITE;
const BG_MENU: u32 = crate::theme::argb(P.surface);
const BG_HOVER: u32 = crate::theme::argb(P.surface_hover);
const BG_DISABLED: u32 = crate::theme::argb(P.surface);
const BORDER: u32 = crate::theme::argb(P.border);
const TEXT_PRIMARY: u32 = crate::theme::argb(P.text_primary);
const TEXT_SECONDARY: u32 = crate::theme::argb(P.text_secondary);
const TEXT_DISABLED: u32 = crate::theme::argb(P.text_placeholder);
const TEXT_DANGER: u32 = crate::theme::argb(P.danger);
const ACCENT: u32 = crate::theme::argb(P.accent);

// ═══════════════════════════════════════════════════════════
// Menu Target
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextTarget {
    CanvasEmpty,
    CanvasSelection {
        selected_count: usize,
        contains_frame: bool,
        contains_component_instance: bool,
        contains_vector: bool,
        contains_text: bool,
    },
    LayerRow {
        entity_id: u64,
        selected_count: usize,
        is_container: bool,
        is_visible: bool,
        is_locked: bool,
        is_component_instance: bool,
    },
    Page {
        page_id: u64,
        is_active: bool,
    },
    InspectorProperty {
        entity_id: u64,
        property_name: String,
        has_variable_binding: bool,
    },
    ToolRail {
        tool_id: String,
        has_alternates: bool,
    },
}

// ═══════════════════════════════════════════════════════════
// Actions
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextAction {
    // File/page
    NewPage,
    DuplicatePage,
    RenamePage,
    DeletePage,
    SetAsActivePage,

    // Edit
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    Rename,

    // Object
    Group,
    Ungroup,
    FrameSelection,
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
    Lock,
    Unlock,
    Hide,
    Show,

    // Components
    CreateComponent,
    DetachInstance,
    ResetOverrides,
    SwapInstance,

    // Boolean/vector
    BooleanUnion,
    BooleanSubtract,
    BooleanIntersect,
    BooleanExclude,
    Flatten,
    OutlineStroke,
    EditVector,

    // Layout
    AddAutoLayout,
    RemoveAutoLayout,
    SelectChildren,

    // Export/copy
    CopyAsSvg,
    CopyAsCss,
    CopyAsPng,
    ExportSelection,

    // Inspector variables
    BindVariable,
    UnbindVariable,
    GoToVariable,

    // Tool rail
    ShowToolAlternates,
    ResetTool,
    CustomizeToolbar,

    // Canvas
    PasteHere,
    SelectAll,
    ToggleGrid,
    ToggleRulers,

    // Dev/help
    Inspect,
    OpenCommandPalette,
}

impl ContextAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NewPage => "New Page",
            Self::DuplicatePage => "Duplicate Page",
            Self::RenamePage => "Rename Page",
            Self::DeletePage => "Delete Page",
            Self::SetAsActivePage => "Set as Active Page",

            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Duplicate => "Duplicate",
            Self::Delete => "Delete",
            Self::Rename => "Rename",

            Self::Group => "Group",
            Self::Ungroup => "Ungroup",
            Self::FrameSelection => "Frame Selection",
            Self::BringToFront => "Bring to Front",
            Self::BringForward => "Bring Forward",
            Self::SendBackward => "Send Backward",
            Self::SendToBack => "Send to Back",
            Self::Lock => "Lock",
            Self::Unlock => "Unlock",
            Self::Hide => "Hide",
            Self::Show => "Show",

            Self::CreateComponent => "Create Component",
            Self::DetachInstance => "Detach Instance",
            Self::ResetOverrides => "Reset Overrides",
            Self::SwapInstance => "Swap Instance",

            Self::BooleanUnion => "Union Selection",
            Self::BooleanSubtract => "Subtract Selection",
            Self::BooleanIntersect => "Intersect Selection",
            Self::BooleanExclude => "Exclude Selection",
            Self::Flatten => "Flatten",
            Self::OutlineStroke => "Outline Stroke",
            Self::EditVector => "Edit Vector",

            Self::AddAutoLayout => "Add Auto Layout",
            Self::RemoveAutoLayout => "Remove Auto Layout",
            Self::SelectChildren => "Select Children",

            Self::CopyAsSvg => "Copy as SVG",
            Self::CopyAsCss => "Copy as CSS",
            Self::CopyAsPng => "Copy as PNG",
            Self::ExportSelection => "Export Selection",

            Self::BindVariable => "Bind Variable",
            Self::UnbindVariable => "Unbind Variable",
            Self::GoToVariable => "Go to Variable",

            Self::ShowToolAlternates => "Show Alternates",
            Self::ResetTool => "Reset Tool",
            Self::CustomizeToolbar => "Customize Toolbar",

            Self::PasteHere => "Paste Here",
            Self::SelectAll => "Select All",
            Self::ToggleGrid => "Toggle Grid",
            Self::ToggleRulers => "Toggle Rulers",

            Self::Inspect => "Inspect",
            Self::OpenCommandPalette => "Open Command Palette",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::NewPage => "file-plus",
            Self::DuplicatePage | Self::Duplicate | Self::Copy => "copy",
            Self::RenamePage | Self::Rename => "pencil",
            Self::DeletePage | Self::Delete => "trash",
            Self::SetAsActivePage => "check",

            Self::Cut => "scissors",
            Self::Paste | Self::PasteHere => "clipboard",

            Self::Group => "group",
            Self::Ungroup => "ungroup",
            Self::FrameSelection => "frame",
            Self::BringToFront | Self::BringForward => "arrow-up",
            Self::SendBackward | Self::SendToBack => "arrow-down",
            Self::Lock => "lock",
            Self::Unlock => "unlock",
            Self::Hide => "eye-off",
            Self::Show => "eye",

            Self::CreateComponent => "component",
            Self::DetachInstance => "unlink",
            Self::ResetOverrides => "rotate-ccw",
            Self::SwapInstance => "replace",

            Self::BooleanUnion => "combine",
            Self::BooleanSubtract => "minus-square",
            Self::BooleanIntersect => "intersect",
            Self::BooleanExclude => "split-square-horizontal",
            Self::Flatten => "layers",
            Self::OutlineStroke => "pen-line",
            Self::EditVector => "pen-tool",

            Self::AddAutoLayout | Self::RemoveAutoLayout => "layout-list",
            Self::SelectChildren => "list-tree",

            Self::CopyAsSvg | Self::CopyAsCss | Self::CopyAsPng => "code",
            Self::ExportSelection => "download",

            Self::BindVariable => "link",
            Self::UnbindVariable => "unlink",
            Self::GoToVariable => "variable",

            Self::ShowToolAlternates => "chevrons-right",
            Self::ResetTool => "rotate-ccw",
            Self::CustomizeToolbar => "settings",

            Self::SelectAll => "maximize",
            Self::ToggleGrid => "grid",
            Self::ToggleRulers => "ruler",

            Self::Inspect => "inspect",
            Self::OpenCommandPalette => "command",
        }
    }

    pub fn shortcut(&self) -> Option<&'static str> {
        match self {
            Self::Cut => Some("⌘X"),
            Self::Copy => Some("⌘C"),
            Self::Paste => Some("⌘V"),
            Self::Duplicate => Some("⌘D"),
            Self::Delete => Some("⌫"),
            Self::Rename => Some("↵"),
            Self::Group => Some("⌘G"),
            Self::Ungroup => Some("⇧⌘G"),
            Self::BringForward => Some("⌘]"),
            Self::SendBackward => Some("⌘["),
            Self::BringToFront => Some("⌥⌘]"),
            Self::SendToBack => Some("⌥⌘["),
            Self::SelectAll => Some("⌘A"),
            Self::OpenCommandPalette => Some("⌘K"),
            Self::CopyAsSvg => Some("⇧⌘C"),
            Self::Flatten => Some("⌘E"),
            Self::AddAutoLayout => Some("⇧A"),
            _ => None,
        }
    }

    pub fn is_danger(&self) -> bool {
        matches!(self, Self::Delete | Self::DeletePage)
    }
}

// ═══════════════════════════════════════════════════════════
// Menu Items
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum ContextMenuItem {
    Action {
        action: ContextAction,
        enabled: bool,
    },
    Separator,
    Submenu {
        label: &'static str,
        icon: &'static str,
        enabled: bool,
        items: Vec<ContextMenuItem>,
    },
}

impl ContextMenuItem {
    pub fn height(&self) -> f64 {
        match self {
            Self::Separator => SEPARATOR_HEIGHT,
            _ => ROW_HEIGHT,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Context Menu State
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub open: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub items: Vec<ContextMenuItem>,
    pub hovered_index: Option<usize>,
    pub target: Option<ContextTarget>,
    pub submenu_open_index: Option<usize>,
    pub submenu_x: f64,
    pub submenu_y: f64,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self {
            open: false,
            x: 0.0,
            y: 0.0,
            width: MENU_WIDTH,
            items: Vec::new(),
            hovered_index: None,
            target: None,
            submenu_open_index: None,
            submenu_x: 0.0,
            submenu_y: 0.0,
        }
    }

    pub fn open_for_target(
        &mut self,
        target: ContextTarget,
        x: f64,
        y: f64,
        screen_w: f64,
        screen_h: f64,
    ) {
        self.items = build_menu_items(&target);
        self.target = Some(target);

        let h = menu_height(&self.items);
        self.x = clamp_menu_x(x, self.width, screen_w);
        self.y = clamp_menu_y(y, h, screen_h);

        self.open = true;
        self.hovered_index = None;
        self.submenu_open_index = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.items.clear();
        self.hovered_index = None;
        self.target = None;
        self.submenu_open_index = None;
    }

    pub fn is_inside(&self, px: f64, py: f64) -> bool {
        if !self.open {
            return false;
        }

        let h = menu_height(&self.items);
        if px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + h {
            return true;
        }

        if let Some(idx) = self.submenu_open_index {
            if let Some(ContextMenuItem::Submenu { items, .. }) = self.items.get(idx) {
                let sh = menu_height(items);
                if px >= self.submenu_x
                    && px <= self.submenu_x + self.width
                    && py >= self.submenu_y
                    && py <= self.submenu_y + sh
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn update_hover(&mut self, px: f64, py: f64, screen_w: f64, screen_h: f64) {
        if !self.open {
            return;
        }

        self.hovered_index = hit_test_items(&self.items, self.x, self.y, self.width, px, py);

        if let Some(idx) = self.hovered_index {
            if let Some(ContextMenuItem::Submenu { items, enabled, .. }) = self.items.get(idx) {
                if *enabled {
                    self.submenu_open_index = Some(idx);
                    self.submenu_x = clamp_menu_x(self.x + self.width - 2.0, self.width, screen_w);
                    let row_y = item_top_y(&self.items, self.y, idx);
                    self.submenu_y = clamp_menu_y(row_y, menu_height(items), screen_h);
                }
            } else {
                self.submenu_open_index = None;
            }
        }
    }

    pub fn click(&mut self, px: f64, py: f64) -> Option<ContextAction> {
        if !self.open {
            return None;
        }

        // Main menu
        if let Some(idx) = hit_test_items(&self.items, self.x, self.y, self.width, px, py) {
            match self.items.get(idx) {
                Some(ContextMenuItem::Action { action, enabled }) if *enabled => {
                    let action = action.clone();
                    self.close();
                    return Some(action);
                }
                Some(ContextMenuItem::Submenu { .. }) => {
                    return None;
                }
                _ => return None,
            }
        }

        // Submenu
        if let Some(sub_idx) = self.submenu_open_index {
            if let Some(ContextMenuItem::Submenu { items, .. }) = self.items.get(sub_idx) {
                if let Some(idx) =
                    hit_test_items(items, self.submenu_x, self.submenu_y, self.width, px, py)
                {
                    match items.get(idx) {
                        Some(ContextMenuItem::Action { action, enabled }) if *enabled => {
                            let action = action.clone();
                            self.close();
                            return Some(action);
                        }
                        _ => return None,
                    }
                }
            }
        }

        // Click outside closes
        self.close();
        None
    }

    pub fn height(&self) -> f64 {
        menu_height(&self.items)
    }
}

// ═══════════════════════════════════════════════════════════
// Menu Construction
// ═══════════════════════════════════════════════════════════

pub fn build_menu_items(target: &ContextTarget) -> Vec<ContextMenuItem> {
    use ContextAction::*;
    use ContextMenuItem::*;

    match target {
        ContextTarget::CanvasEmpty => vec![
            Action {
                action: PasteHere,
                enabled: true,
            },
            Action {
                action: SelectAll,
                enabled: true,
            },
            Separator,
            Action {
                action: ToggleGrid,
                enabled: true,
            },
            Action {
                action: ToggleRulers,
                enabled: true,
            },
            Separator,
            Action {
                action: OpenCommandPalette,
                enabled: true,
            },
        ],

        ContextTarget::CanvasSelection {
            selected_count,
            contains_frame,
            contains_component_instance,
            contains_vector,
            contains_text: _,
        } => {
            let multi = *selected_count > 1;
            let mut items = vec![
                Action {
                    action: Cut,
                    enabled: true,
                },
                Action {
                    action: Copy,
                    enabled: true,
                },
                Action {
                    action: Duplicate,
                    enabled: true,
                },
                Separator,
                Action {
                    action: Group,
                    enabled: multi,
                },
                Action {
                    action: Ungroup,
                    enabled: true,
                },
                Action {
                    action: FrameSelection,
                    enabled: true,
                },
                Separator,
                Submenu {
                    label: "Arrange",
                    icon: "layers",
                    enabled: true,
                    items: vec![
                        Action {
                            action: BringToFront,
                            enabled: true,
                        },
                        Action {
                            action: BringForward,
                            enabled: true,
                        },
                        Action {
                            action: SendBackward,
                            enabled: true,
                        },
                        Action {
                            action: SendToBack,
                            enabled: true,
                        },
                    ],
                },
                Submenu {
                    label: "Boolean",
                    icon: "combine",
                    enabled: multi,
                    items: vec![
                        Action {
                            action: BooleanUnion,
                            enabled: multi,
                        },
                        Action {
                            action: BooleanSubtract,
                            enabled: multi,
                        },
                        Action {
                            action: BooleanIntersect,
                            enabled: multi,
                        },
                        Action {
                            action: BooleanExclude,
                            enabled: multi,
                        },
                    ],
                },
                Separator,
            ];

            if *contains_vector {
                items.push(Action {
                    action: EditVector,
                    enabled: true,
                });
                items.push(Action {
                    action: OutlineStroke,
                    enabled: true,
                });
                items.push(Action {
                    action: Flatten,
                    enabled: true,
                });
                items.push(Separator);
            }

            if *contains_frame {
                items.push(Action {
                    action: AddAutoLayout,
                    enabled: true,
                });
                items.push(Action {
                    action: RemoveAutoLayout,
                    enabled: true,
                });
                items.push(Separator);
            }

            if *contains_component_instance {
                items.push(Action {
                    action: DetachInstance,
                    enabled: true,
                });
                items.push(Action {
                    action: ResetOverrides,
                    enabled: true,
                });
                items.push(Action {
                    action: SwapInstance,
                    enabled: true,
                });
                items.push(Separator);
            } else {
                items.push(Action {
                    action: CreateComponent,
                    enabled: true,
                });
                items.push(Separator);
            }

            items.push(Submenu {
                label: "Copy as",
                icon: "code",
                enabled: true,
                items: vec![
                    Action {
                        action: CopyAsSvg,
                        enabled: true,
                    },
                    Action {
                        action: CopyAsCss,
                        enabled: true,
                    },
                    Action {
                        action: CopyAsPng,
                        enabled: true,
                    },
                ],
            });
            items.push(Action {
                action: ExportSelection,
                enabled: true,
            });
            items.push(Separator);
            items.push(Action {
                action: Lock,
                enabled: true,
            });
            items.push(Action {
                action: Hide,
                enabled: true,
            });
            items.push(Action {
                action: Delete,
                enabled: true,
            });

            items
        }

        ContextTarget::LayerRow {
            selected_count,
            is_container,
            is_visible,
            is_locked,
            is_component_instance,
            ..
        } => {
            let multi = *selected_count > 1;
            let mut items = vec![
                Action {
                    action: Rename,
                    enabled: !multi,
                },
                Action {
                    action: Duplicate,
                    enabled: true,
                },
                Separator,
                Action {
                    action: if *is_visible { Hide } else { Show },
                    enabled: true,
                },
                Action {
                    action: if *is_locked { Unlock } else { Lock },
                    enabled: true,
                },
                Separator,
                Action {
                    action: BringToFront,
                    enabled: true,
                },
                Action {
                    action: SendToBack,
                    enabled: true,
                },
            ];

            if *is_container {
                items.push(Separator);
                items.push(Action {
                    action: SelectChildren,
                    enabled: true,
                });
            }

            if *is_component_instance {
                items.push(Separator);
                items.push(Action {
                    action: DetachInstance,
                    enabled: true,
                });
                items.push(Action {
                    action: ResetOverrides,
                    enabled: true,
                });
            }

            items.push(Separator);
            items.push(Action {
                action: Delete,
                enabled: true,
            });
            items
        }

        ContextTarget::Page { is_active, .. } => vec![
            Action {
                action: SetAsActivePage,
                enabled: !*is_active,
            },
            Separator,
            Action {
                action: NewPage,
                enabled: true,
            },
            Action {
                action: DuplicatePage,
                enabled: true,
            },
            Action {
                action: RenamePage,
                enabled: true,
            },
            Separator,
            Action {
                action: DeletePage,
                enabled: true,
            },
        ],

        ContextTarget::InspectorProperty {
            has_variable_binding,
            ..
        } => vec![
            Action {
                action: if *has_variable_binding {
                    UnbindVariable
                } else {
                    BindVariable
                },
                enabled: true,
            },
            Action {
                action: GoToVariable,
                enabled: *has_variable_binding,
            },
            Separator,
            Action {
                action: Copy,
                enabled: true,
            },
            Action {
                action: Paste,
                enabled: true,
            },
        ],

        ContextTarget::ToolRail { has_alternates, .. } => vec![
            Action {
                action: ShowToolAlternates,
                enabled: *has_alternates,
            },
            Action {
                action: ResetTool,
                enabled: true,
            },
            Separator,
            Action {
                action: CustomizeToolbar,
                enabled: true,
            },
        ],
    }
}

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

fn menu_height(items: &[ContextMenuItem]) -> f64 {
    items.iter().map(|i| i.height()).sum()
}

fn clamp_menu_x(x: f64, w: f64, screen_w: f64) -> f64 {
    if x + w > screen_w - 8.0 {
        (screen_w - w - 8.0).max(8.0)
    } else {
        x.max(8.0)
    }
}

fn clamp_menu_y(y: f64, h: f64, screen_h: f64) -> f64 {
    if y + h > screen_h - 8.0 {
        (screen_h - h - 8.0).max(8.0)
    } else {
        y.max(8.0)
    }
}

fn item_top_y(items: &[ContextMenuItem], start_y: f64, index: usize) -> f64 {
    let mut y = start_y;
    for item in items.iter().take(index) {
        y += item.height();
    }
    y
}

fn hit_test_items(
    items: &[ContextMenuItem],
    x: f64,
    y: f64,
    w: f64,
    px: f64,
    py: f64,
) -> Option<usize> {
    if px < x || px > x + w {
        return None;
    }

    let mut cy = y;
    for (i, item) in items.iter().enumerate() {
        let h = item.height();
        if py >= cy && py <= cy + h {
            if matches!(item, ContextMenuItem::Separator) {
                return None;
            }
            return Some(i);
        }
        cy += h;
    }

    None
}

// ═══════════════════════════════════════════════════════════
// Rendering
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum ContextPaintCommand {
    FillRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: u32,
        radius: f64,
    },
    StrokeRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: u32,
        radius: f64,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: u32,
        size: f64,
    },
    Icon {
        x: f64,
        y: f64,
        name: String,
        size: f64,
        color: u32,
    },
    HLine {
        x: f64,
        y: f64,
        w: f64,
        color: u32,
    },
    ChevronRight {
        x: f64,
        y: f64,
        color: u32,
    },
}

pub fn render_context_menu(menu: &ContextMenu) -> Vec<ContextPaintCommand> {
    let mut cmds = Vec::new();

    if !menu.open {
        return cmds;
    }

    render_menu_items(
        &mut cmds,
        &menu.items,
        menu.x,
        menu.y,
        menu.width,
        menu.hovered_index,
    );

    if let Some(sub_idx) = menu.submenu_open_index {
        if let Some(ContextMenuItem::Submenu { items, .. }) = menu.items.get(sub_idx) {
            render_menu_items(
                &mut cmds,
                items,
                menu.submenu_x,
                menu.submenu_y,
                menu.width,
                None,
            );
        }
    }

    cmds
}

fn render_menu_items(
    cmds: &mut Vec<ContextPaintCommand>,
    items: &[ContextMenuItem],
    x: f64,
    y: f64,
    w: f64,
    hovered_index: Option<usize>,
) {
    let h = menu_height(items);

    cmds.push(ContextPaintCommand::FillRect {
        x,
        y,
        w,
        h,
        color: BG_MENU,
        radius: 6.0,
    });

    cmds.push(ContextPaintCommand::StrokeRect {
        x,
        y,
        w,
        h,
        color: BORDER,
        radius: 6.0,
    });

    let mut cy = y;

    for (i, item) in items.iter().enumerate() {
        match item {
            ContextMenuItem::Separator => {
                let line_y = cy + SEPARATOR_HEIGHT / 2.0;
                cmds.push(ContextPaintCommand::HLine {
                    x: x + PADDING_X,
                    y: line_y,
                    w: w - PADDING_X * 2.0,
                    color: BORDER,
                });
                cy += SEPARATOR_HEIGHT;
            }

            ContextMenuItem::Action { action, enabled } => {
                let hovered = hovered_index == Some(i);
                if hovered && *enabled {
                    cmds.push(ContextPaintCommand::FillRect {
                        x: x + 4.0,
                        y: cy + 2.0,
                        w: w - 8.0,
                        h: ROW_HEIGHT - 4.0,
                        color: BG_HOVER,
                        radius: 4.0,
                    });
                }

                let color = if !*enabled {
                    TEXT_DISABLED
                } else if action.is_danger() {
                    TEXT_DANGER
                } else {
                    TEXT_PRIMARY
                };

                cmds.push(ContextPaintCommand::Icon {
                    x: x + PADDING_X,
                    y: cy + 7.0,
                    name: action.icon().to_string(),
                    size: ICON_SIZE,
                    color,
                });

                cmds.push(ContextPaintCommand::Text {
                    x: x + PADDING_X + ICON_SIZE + ICON_GAP,
                    y: cy + 7.0,
                    text: action.label().to_string(),
                    color,
                    size: 12.0,
                });

                if let Some(shortcut) = action.shortcut() {
                    cmds.push(ContextPaintCommand::Text {
                        x: x + w - SHORTCUT_WIDTH,
                        y: cy + 7.0,
                        text: shortcut.to_string(),
                        color: TEXT_SECONDARY,
                        size: 11.0,
                    });
                }

                cy += ROW_HEIGHT;
            }

            ContextMenuItem::Submenu {
                label,
                icon,
                enabled,
                ..
            } => {
                let hovered = hovered_index == Some(i);
                if hovered && *enabled {
                    cmds.push(ContextPaintCommand::FillRect {
                        x: x + 4.0,
                        y: cy + 2.0,
                        w: w - 8.0,
                        h: ROW_HEIGHT - 4.0,
                        color: BG_HOVER,
                        radius: 4.0,
                    });
                }

                let color = if *enabled {
                    TEXT_PRIMARY
                } else {
                    TEXT_DISABLED
                };

                cmds.push(ContextPaintCommand::Icon {
                    x: x + PADDING_X,
                    y: cy + 7.0,
                    name: (*icon).to_string(),
                    size: ICON_SIZE,
                    color,
                });

                cmds.push(ContextPaintCommand::Text {
                    x: x + PADDING_X + ICON_SIZE + ICON_GAP,
                    y: cy + 7.0,
                    text: (*label).to_string(),
                    color,
                    size: 12.0,
                });

                cmds.push(ContextPaintCommand::ChevronRight {
                    x: x + w - 18.0,
                    y: cy + 8.0,
                    color: TEXT_SECONDARY,
                });

                cy += ROW_HEIGHT;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_empty_menu() {
        let items = build_menu_items(&ContextTarget::CanvasEmpty);
        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::PasteHere,
                ..
            }
        )));
        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::OpenCommandPalette,
                ..
            }
        )));
    }

    #[test]
    fn selection_menu_contains_boolean_when_multi() {
        let items = build_menu_items(&ContextTarget::CanvasSelection {
            selected_count: 2,
            contains_frame: false,
            contains_component_instance: false,
            contains_vector: false,
            contains_text: false,
        });

        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Submenu {
                label: "Boolean",
                enabled: true,
                ..
            }
        )));
    }

    #[test]
    fn selection_menu_disables_boolean_when_single() {
        let items = build_menu_items(&ContextTarget::CanvasSelection {
            selected_count: 1,
            contains_frame: false,
            contains_component_instance: false,
            contains_vector: false,
            contains_text: false,
        });

        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Submenu {
                label: "Boolean",
                enabled: false,
                ..
            }
        )));
    }

    #[test]
    fn vector_selection_gets_vector_actions() {
        let items = build_menu_items(&ContextTarget::CanvasSelection {
            selected_count: 1,
            contains_frame: false,
            contains_component_instance: false,
            contains_vector: true,
            contains_text: false,
        });

        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::EditVector,
                ..
            }
        )));
        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::OutlineStroke,
                ..
            }
        )));
    }

    #[test]
    fn component_instance_gets_detach_actions() {
        let items = build_menu_items(&ContextTarget::CanvasSelection {
            selected_count: 1,
            contains_frame: false,
            contains_component_instance: true,
            contains_vector: false,
            contains_text: false,
        });

        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::DetachInstance,
                ..
            }
        )));
    }

    #[test]
    fn layer_row_visibility_action_changes() {
        let visible_items = build_menu_items(&ContextTarget::LayerRow {
            entity_id: 1,
            selected_count: 1,
            is_container: false,
            is_visible: true,
            is_locked: false,
            is_component_instance: false,
        });

        assert!(visible_items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::Hide,
                ..
            }
        )));

        let hidden_items = build_menu_items(&ContextTarget::LayerRow {
            entity_id: 1,
            selected_count: 1,
            is_container: false,
            is_visible: false,
            is_locked: false,
            is_component_instance: false,
        });

        assert!(hidden_items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::Show,
                ..
            }
        )));
    }

    #[test]
    fn page_menu_active_disables_set_active() {
        let items = build_menu_items(&ContextTarget::Page {
            page_id: 1,
            is_active: true,
        });

        assert!(items.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::SetAsActivePage,
                enabled: false,
            }
        )));
    }

    #[test]
    fn inspector_variable_menu() {
        let bound = build_menu_items(&ContextTarget::InspectorProperty {
            entity_id: 1,
            property_name: "fill".into(),
            has_variable_binding: true,
        });

        assert!(bound.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::UnbindVariable,
                ..
            }
        )));
        assert!(bound.iter().any(|i| matches!(
            i,
            ContextMenuItem::Action {
                action: ContextAction::GoToVariable,
                enabled: true
            }
        )));
    }

    #[test]
    fn menu_clamps_to_screen() {
        let mut menu = ContextMenu::new();
        menu.open_for_target(ContextTarget::CanvasEmpty, 1900.0, 1000.0, 1920.0, 1080.0);
        assert!(menu.x + menu.width <= 1920.0);
        assert!(menu.y + menu.height() <= 1080.0);
    }

    #[test]
    fn click_action_executes_and_closes() {
        let mut menu = ContextMenu::new();
        menu.open_for_target(ContextTarget::CanvasEmpty, 100.0, 100.0, 1920.0, 1080.0);

        let action = menu.click(110.0, 110.0);
        assert_eq!(action, Some(ContextAction::PasteHere));
        assert!(!menu.open);
    }

    #[test]
    fn click_outside_closes() {
        let mut menu = ContextMenu::new();
        menu.open_for_target(ContextTarget::CanvasEmpty, 100.0, 100.0, 1920.0, 1080.0);

        let action = menu.click(500.0, 500.0);
        assert_eq!(action, None);
        assert!(!menu.open);
    }

    #[test]
    fn render_closed_menu_empty() {
        let menu = ContextMenu::new();
        let cmds = render_context_menu(&menu);
        assert!(cmds.is_empty());
    }
}
