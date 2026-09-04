//! Application state — document + UI, no old chrome inheritance.

use x_native::editor::Editor;
use x_native::{Document, Node, Variables};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Frame,
    Rectangle,
    Ellipse,
    Line,
    Pen,
    Text,
    Hand,
    Zoom,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Frame => "Frame",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Line => "Line",
            Tool::Pen => "Pen",
            Tool::Text => "Text",
            Tool::Hand => "Hand",
            Tool::Zoom => "Zoom",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Frame => "F",
            Tool::Rectangle => "R",
            Tool::Ellipse => "O",
            Tool::Line => "L",
            Tool::Pen => "P",
            Tool::Text => "T",
            Tool::Hand => "H",
            Tool::Zoom => "Z",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Editor,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LeftTab {
    Layers,
    Assets,
    Components,
    Variables,
}

pub struct AppState {
    pub screen: Screen,
    pub win_w: f64,
    pub win_h: f64,
    pub doc_name: String,
    pub pages: Vec<Node>,
    pub page_idx: usize,
    pub editor: Editor,
    pub vars: Variables,
    pub tool: Tool,
    pub left_tab: LeftTab,
    pub zoom: f64,
    pub pan: (f64, f64),
    pub space_pan: bool,
    pub status: String,
    pub command_open: bool,
    pub command_query: String,
    pub layer_filter: String,
    pub dirty: bool,
}

impl AppState {
    pub fn new_blank() -> Self {
        let page = Node::frame("page-1", 1600.0, 1000.0);
        let editor = Editor::new(page.clone());
        Self {
            screen: Screen::Home,
            win_w: 1440.0,
            win_h: 900.0,
            doc_name: "Untitled".into(),
            pages: vec![page],
            page_idx: 0,
            editor,
            vars: Variables::default(),
            tool: Tool::Select,
            left_tab: LeftTab::Layers,
            zoom: 1.0,
            pan: (80.0, 60.0),
            space_pan: false,
            status: "Ready — ⌘K for commands".into(),
            command_open: false,
            command_query: String::new(),
            layer_filter: String::new(),
            dirty: false,
        }
    }

    pub fn open_editor_blank(&mut self) {
        let page = Node::frame("page-1", 1600.0, 1000.0);
        self.pages = vec![page.clone()];
        self.page_idx = 0;
        self.editor = Editor::new(page);
        self.doc_name = "Untitled".into();
        self.screen = Screen::Editor;
        self.tool = Tool::Select;
        self.zoom = 0.5;
        self.pan = (80.0, 60.0);
        self.status = "Blank canvas — F frame · R rectangle · T text · ⌘K commands".into();
        self.dirty = false;
    }

    pub fn current_page_name(&self) -> String {
        self.pages
            .get(self.page_idx)
            .map(|p| {
                if p.name.is_empty() {
                    p.id.clone()
                } else {
                    p.name.clone()
                }
            })
            .unwrap_or_else(|| "Page".into())
    }

    /// Layer rows: children of page only (never the page root).
    pub fn layer_rows(&self) -> Vec<(String, String, usize)> {
        fn walk(n: &Node, depth: usize, out: &mut Vec<(String, String, usize)>) {
            let name = if n.name.is_empty() {
                n.id.clone()
            } else {
                n.name.clone()
            };
            out.push((n.id.clone(), name, depth));
            for c in &n.children {
                walk(c, depth + 1, out);
            }
        }
        let mut rows = vec![];
        for c in &self.editor.root.children {
            walk(c, 0, &mut rows);
        }
        if !self.layer_filter.is_empty() {
            let q = self.layer_filter.to_ascii_lowercase();
            rows.retain(|(id, name, _)| {
                id.to_ascii_lowercase().contains(&q) || name.to_ascii_lowercase().contains(&q)
            });
        }
        rows
    }

    pub fn add_page(&mut self) {
        self.pages[self.page_idx] = self.editor.root.clone();
        let id = format!("page-{}", self.pages.len() + 1);
        let page = Node::frame(&id, 1600.0, 1000.0);
        self.pages.push(page.clone());
        self.page_idx = self.pages.len() - 1;
        self.editor = Editor::new(page);
        self.status = format!("New page: {id}");
        self.dirty = true;
    }

    pub fn switch_page(&mut self, idx: usize) {
        if idx >= self.pages.len() || idx == self.page_idx {
            return;
        }
        self.pages[self.page_idx] = self.editor.root.clone();
        self.page_idx = idx;
        self.editor = Editor::new(self.pages[idx].clone());
        self.editor.selection.clear();
        self.status = format!("Page: {}", self.current_page_name());
    }

    pub fn delete_page(&mut self, idx: usize) {
        if self.pages.len() <= 1 {
            self.status = "Can't delete the last page".into();
            return;
        }
        if idx >= self.pages.len() {
            return;
        }
        self.pages.remove(idx);
        if self.page_idx >= self.pages.len() {
            self.page_idx = self.pages.len() - 1;
        } else if idx < self.page_idx {
            self.page_idx -= 1;
        }
        self.editor = Editor::new(self.pages[self.page_idx].clone());
        self.editor.selection.clear();
        self.status = "Page deleted".into();
        self.dirty = true;
    }

    pub fn document_snapshot(&self) -> Document {
        let mut pages = self.pages.clone();
        if let Some(p) = pages.get_mut(self.page_idx) {
            *p = self.editor.root.clone();
        }
        let mut d = Document::new();
        d.pages = pages;
        d.variables = self.vars.clone();
        d
    }
}
