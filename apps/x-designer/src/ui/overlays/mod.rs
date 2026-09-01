// Overlays Module - Command Palette, Import Preview, Library Updates
// Modal and popover components

use crate::ui::tokens::*;

pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub selected_index: usize,
    pub results: Vec<CommandResult>,
}

#[derive(Clone)]
pub struct CommandResult {
    pub category: &'static str,
    pub label: &'static str,
    pub shortcut: Option<&'static str>,
    pub icon: &'static str,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected_index: 0,
            results: Vec::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.query.clear();
            self.selected_index = 0;
            self.search("");
        }
    }

    pub fn search(&mut self, query: &str) {
        self.query = query.to_string();
        // Future: implement actual search logic
        self.results = Self::get_all_commands();
    }

    fn get_all_commands() -> Vec<CommandResult> {
        vec![
            // Create
            CommandResult { category: "Create", label: "Rectangle", shortcut: Some("R"), icon: "□" },
            CommandResult { category: "Create", label: "Ellipse", shortcut: Some("O"), icon: "○" },
            CommandResult { category: "Create", label: "Frame", shortcut: Some("F"), icon: "◫" },
            CommandResult { category: "Create", label: "Component", shortcut: None, icon: "◈" },
            
            // Navigate
            CommandResult { category: "Navigate", label: "Layers", shortcut: Some("Shift+L"), icon: "◇" },
            CommandResult { category: "Navigate", label: "Assets", shortcut: Some("Shift+A"), icon: "◆" },
            CommandResult { category: "Navigate", label: "Variables", shortcut: None, icon: "◎" },
            
            // Actions
            CommandResult { category: "Actions", label: "Union", shortcut: Some("⌘+U"), icon: "⊔" },
            CommandResult { category: "Actions", label: "Subtract", shortcut: Some("⌘-S"), icon: "⊖" },
            CommandResult { category: "Actions", label: "Auto Layout", shortcut: Some("Shift+A"), icon: "≡" },
            
            // File
            CommandResult { category: "File", label: "Save", shortcut: Some("⌘+S"), icon: "💾" },
            CommandResult { category: "File", label: "Export SVG", shortcut: None, icon: "📐" },
            CommandResult { category: "File", label: "Export PDF", shortcut: None, icon: "📄" },
        ]
    }

    pub fn navigate_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn navigate_down(&mut self) {
        if self.selected_index < self.results.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn execute_selected(&self) {
        if let Some(result) = self.results.get(self.selected_index) {
            // Future: execute command
            println!("Executing: {}", result.label);
        }
    }
}

pub struct ImportPreview {
    pub visible: bool,
    pub file_path: String,
    pub preview_data: Option<String>,
}

impl ImportPreview {
    pub fn new() -> Self {
        Self {
            visible: false,
            file_path: String::new(),
            preview_data: None,
        }
    }

    pub fn show(&mut self, file_path: &str) {
        self.file_path = file_path.to_string();
        self.visible = true;
        // Future: load preview data
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

pub struct LibraryUpdate {
    pub visible: bool,
    pub library_name: String,
    pub updates_available: usize,
}

impl LibraryUpdate {
    pub fn new() -> Self {
        Self {
            visible: false,
            library_name: String::new(),
            updates_available: 0,
        }
    }

    pub fn show(&mut self, library_name: &str, count: usize) {
        self.library_name = library_name.to_string();
        self.updates_available = count;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}
