//! Overlays System
//! 
//! Command palette (⌘K), import preview, library updates, and other modal overlays

use egui::{self, Color32, Key, KeyboardShortcut, Modifiers, Rect, Response, Sense, Ui, Vec2};
use super::tokens::*;

/// Overlay types
#[derive(Clone, PartialEq)]
pub enum OverlayType {
    None,
    CommandPalette,
    ImportPreview,
    LibraryUpdate,
    Shortcuts,
}

/// Overlay system state
#[derive(Default)]
pub struct Overlays {
    /// Currently active overlay
    pub active: OverlayType,
    /// Command palette search query
    pub command_query: String,
    /// Selected command index
    pub selected_command: usize,
    /// Available commands (filtered by query)
    pub commands: Vec<CommandItem>,
}

/// Command item for the palette
pub struct CommandItem {
    pub icon: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub shortcut: Option<KeyboardShortcut>,
    pub action: fn(), // Placeholder for actual action
}

impl Overlays {
    pub fn new() -> Self {
        let mut overlays = Self {
            active: OverlayType::None,
            command_query: String::new(),
            selected_command: 0,
            commands: vec![],
        };
        
        // Initialize default commands
        overlays.commands = overlays.get_all_commands();
        
        overlays
    }

    fn get_all_commands(&self) -> Vec<CommandItem> {
        vec![
            // Create
            CommandItem {
                icon: "□",
                label: "Rectangle",
                category: "Create",
                shortcut: Some(KeyboardShortcut::new(Modifiers::NONE, Key::R)),
                action: || {},
            },
            CommandItem {
                icon: "○",
                label: "Ellipse",
                category: "Create",
                shortcut: Some(KeyboardShortcut::new(Modifiers::NONE, Key::O)),
                action: || {},
            },
            CommandItem {
                icon: "◇",
                label: "Frame",
                category: "Create",
                shortcut: Some(KeyboardShortcut::new(Modifiers::NONE, Key::F)),
                action: || {},
            },
            CommandItem {
                icon: "T",
                label: "Text",
                category: "Create",
                shortcut: Some(KeyboardShortcut::new(Modifiers::NONE, Key::T)),
                action: || {},
            },
            
            // Navigate
            CommandItem {
                icon: "◇",
                label: "Layers",
                category: "Navigate",
                shortcut: Some(KeyboardShortcut::new(Modifiers::SHIFT, Key::L)),
                action: || {},
            },
            CommandItem {
                icon: "◆",
                label: "Assets",
                category: "Navigate",
                shortcut: Some(KeyboardShortcut::new(Modifiers::SHIFT, Key::A)),
                action: || {},
            },
            CommandItem {
                icon: "◈",
                label: "Components",
                category: "Navigate",
                shortcut: None,
                action: || {},
            },
            CommandItem {
                icon: "◎",
                label: "Variables",
                category: "Navigate",
                shortcut: None,
                action: || {},
            },
            
            // Actions
            CommandItem {
                icon: "∪",
                label: "Union Selection",
                category: "Actions",
                shortcut: Some(KeyboardShortcut::new(Modifiers::CTRL, Key::Plus)),
                action: || {},
            },
            CommandItem {
                icon: "−",
                label: "Subtract Selection",
                category: "Actions",
                shortcut: Some(KeyboardShortcut::new(Modifiers::CTRL, Key::Minus)),
                action: || {},
            },
            
            // File
            CommandItem {
                icon: "💾",
                label: "Save",
                category: "File",
                shortcut: Some(KeyboardShortcut::new(Modifiers::CTRL, Key::S)),
                action: || {},
            },
            CommandItem {
                icon: "📤",
                label: "Export SVG",
                category: "File",
                shortcut: None,
                action: || {},
            },
        ]
    }

    /// Check for overlay toggle shortcuts
    pub fn check_shortcuts(&mut self, ctx: &egui::Context) {
        // Cmd+K for command palette
        if ctx.input_mut(|i| i.consume_shortcut(KeyboardShortcut::new(Modifiers::COMMAND, Key::K))) {
            if self.active == OverlayType::CommandPalette {
                self.active = OverlayType::None;
            } else {
                self.active = OverlayType::CommandPalette;
                self.command_query.clear();
                self.selected_command = 0;
            }
        }
        
        // Escape to close any overlay
        if self.active != OverlayType::None {
            if ctx.input_mut(|i| i.key_pressed(Key::Escape)) {
                self.active = OverlayType::None;
            }
        }
    }

    /// Render active overlay
    pub fn render(&mut self, ctx: &egui::Context) {
        match self.active {
            OverlayType::None => {}
            OverlayType::CommandPalette => self.render_command_palette(ctx),
            OverlayType::ImportPreview => self.render_import_preview(ctx),
            OverlayType::LibraryUpdate => self.render_library_update(ctx),
            OverlayType::Shortcuts => self.render_shortcuts(ctx),
        }
    }

    fn render_command_palette(&mut self, ctx: &egui::Context) {
        let screen_size = ctx.screen_rect().size();
        
        egui::Window::new("Command Palette")
            .id(egui::Id::new("command_palette"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .fixed_size([500.0, 400.0])
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .frame(egui::Frame::window(&ctx.style())
                .fill(C_BG_SECONDARY)
                .stroke(egui::Stroke::new(BORDER_MEDIUM, C_BORDER))
                .rounding(RADIUS_LG.into())
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 8.0),
                    blur: 24.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(100),
                }))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Search input
                    ui.add_space(SPACE_3);
                    
                    let search_response = ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("⌘K")
                            .font(egui::FontId::proportional(FONT_SIZE_LABEL))
                            .color(C_TEXT_SECONDARY));
                        ui.add_space(SPACE_2);
                        
                        let text_edit = egui::TextEdit::singleline(&mut self.command_query)
                            .desired_width(420.0)
                            .hint_text("Search commands...")
                            .font(egui::FontId::proportional(FONT_SIZE_LABEL))
                            .desired_height(ROW_HEIGHT_STD);
                        
                        ui.add(text_edit)
                    }).response;
                    
                    if search_response.clicked() {
                        // Focus on text edit
                    }
                    
                    ui.add_space(SPACE_2);
                    ui.separator();
                    ui.add_space(SPACE_2);
                    
                    // Filter commands based on query
                    let filtered_commands: Vec<&CommandItem> = self.commands.iter()
                        .filter(|cmd| {
                            if self.command_query.is_empty() {
                                true
                            } else {
                                cmd.label.to_lowercase().contains(&self.command_query.to_lowercase()) ||
                                cmd.category.to_lowercase().contains(&self.command_query.to_lowercase())
                            }
                        })
                        .collect();
                    
                    // Commands list
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (index, cmd) in filtered_commands.iter().enumerate() {
                                let is_selected = index == self.selected_command;
                                
                                let response = ui.horizontal(|ui| {
                                    ui.add_space(SPACE_3);
                                    
                                    // Icon
                                    ui.label(egui::RichText::new(cmd.icon)
                                        .font(egui::FontId::proportional(FONT_SIZE_LABEL))
                                        .color(if is_selected { C_ACCENT } else { C_TEXT_SECONDARY }));
                                    
                                    ui.add_space(SPACE_3);
                                    
                                    // Label
                                    ui.label(egui::RichText::new(cmd.label)
                                        .font(egui::FontId::proportional(FONT_SIZE_BODY))
                                        .color(if is_selected { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }));
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        // Category badge
                                        ui.label(egui::RichText::new(cmd.category)
                                            .font(egui::FontId::proportional(FONT_SIZE_CAPTION))
                                            .color(C_TEXT_DISABLED));
                                        
                                        if let Some(shortcut) = cmd.shortcut {
                                            ui.add_space(SPACE_2);
                                            // Show shortcut hint
                                            ui.label(egui::RichText::new(format!("{:?}", shortcut.modifiers))
                                                .font(egui::FontId::monospace(FONT_SIZE_MICRO))
                                                .color(C_TEXT_DISABLED));
                                        }
                                    });
                                }).response;
                                
                                if response.clicked() {
                                    // Execute command
                                    (cmd.action)();
                                    self.active = OverlayType::None;
                                }
                                
                                if response.hovered() {
                                    self.selected_command = index;
                                }
                                
                                // Highlight selected
                                if is_selected {
                                    ui.painter().rect_filled(
                                        response.rect.shrink(SPACE_1),
                                        egui::CornerRadius::from(RADIUS_SM as i8),
                                        C_SELECTED,
                                    );
                                }
                            }
                        });
                });
            });
    }

    fn render_import_preview(&mut self, ctx: &egui::Context) {
        // Placeholder for import preview overlay
        egui::Window::new("Import Preview")
            .id(egui::Id::new("import_preview"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.label("Import preview coming soon...");
            });
    }

    fn render_library_update(&mut self, ctx: &egui::Context) {
        // Placeholder for library update overlay
        egui::Window::new("Library Updates")
            .id(egui::Id::new("library_update"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([500.0, 350.0])
            .show(ctx, |ui| {
                ui.label("Library updates available...");
            });
    }

    fn render_shortcuts(&mut self, ctx: &egui::Context) {
        // Placeholder for shortcuts overlay
        egui::Window::new("Keyboard Shortcuts")
            .id(egui::Id::new("shortcuts"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([700.0, 500.0])
            .show(ctx, |ui| {
                ui.label("Keyboard shortcuts reference...");
            });
    }

    /// Get current overlay type
    pub fn is_open(&self) -> bool {
        self.active != OverlayType::None
    }

    /// Close current overlay
    pub fn close(&mut self) {
        self.active = OverlayType::None;
    }
}
