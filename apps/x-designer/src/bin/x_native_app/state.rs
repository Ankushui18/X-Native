//! Application state — document + UI (Phase 1–2).

use x_native::editor::{find, Editor};
use x_native::{Color, Document, Node, Paint, Variables};

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

    pub fn is_create(self) -> bool {
        matches!(
            self,
            Tool::Frame | Tool::Rectangle | Tool::Ellipse | Tool::Line | Tool::Text
        )
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    Nw,
    Ne,
    Sw,
    Se,
}

#[derive(Clone, Debug)]
pub enum Drag {
    None,
    Pan { last: (f64, f64) },
    Create { start: (f64, f64) },
    Move { last: (f64, f64) },
    /// Corner resize of a single selected node
    Resize {
        id: String,
        handle: Handle,
        origin: (f64, f64, f64, f64), // x,y,w,h at start
    },
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
    pub drag: Drag,
    pub created_count: usize,
    pub create_preview: Option<(f64, f64, f64, f64)>,
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
            status: "Ready — Cmd/Ctrl+K for commands".into(),
            command_open: false,
            command_query: String::new(),
            layer_filter: String::new(),
            dirty: false,
            drag: Drag::None,
            created_count: 0,
            create_preview: None,
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
        self.status = "Blank canvas — F frame · R rect · T text · Cmd+K".into();
        self.dirty = false;
        self.drag = Drag::None;
        self.create_preview = None;
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

    pub fn selected_node(&self) -> Option<&Node> {
        if self.editor.selection.len() != 1 {
            return None;
        }
        let id = &self.editor.selection[0];
        if id == &self.editor.root.id {
            return None;
        }
        find(&self.editor.root, id)
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

    pub fn finish_create(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let (bx, by) = (x0.min(x1), y0.min(y1));
        let (mut bw, mut bh) = ((x1 - x0).abs(), (y1 - y0).abs());
        if bw < 4.0 && bh < 4.0 {
            match self.tool {
                Tool::Text => {
                    bw = 120.0;
                    bh = 24.0;
                }
                Tool::Line => {
                    bw = 100.0;
                    bh = 1.0;
                }
                _ => {
                    bw = 100.0;
                    bh = 100.0;
                }
            }
        }
        self.created_count += 1;
        let n = self.created_count;
        let root = self.editor.root.id.clone();
        let node = match self.tool {
            Tool::Frame => {
                let mut f = Node::frame(&format!("frame-{n}"), bw, bh);
                f.transform.x = bx;
                f.transform.y = by;
                f.fill = Paint::Solid(Color::WHITE);
                f.name = "Frame".into();
                f
            }
            Tool::Rectangle => Node::rect(
                &format!("rect-{n}"),
                bx,
                by,
                bw,
                bh,
                Color::from_rgb8(0xd9, 0xdc, 0xe3),
            ),
            Tool::Ellipse => Node::ellipse(
                &format!("ellipse-{n}"),
                bx,
                by,
                bw,
                bh,
                Color::from_rgb8(0xd9, 0xdc, 0xe3),
            ),
            Tool::Line => Node::line(
                &format!("line-{n}"),
                bx,
                by,
                bw.max(1.0),
                bh.max(1.0),
                Color::from_rgb8(0x0d, 0x12, 0x20),
            ),
            Tool::Text => {
                let mut t = Node::text(
                    &format!("text-{n}"),
                    bx,
                    by,
                    bw.max(40.0),
                    bh.clamp(14.0, 32.0),
                    "Text",
                );
                t.name = "Text".into();
                t
            }
            _ => return,
        };
        let id = node.id.clone();
        if self.editor.insert_node(&root, node) {
            self.editor.selection = vec![id];
            self.dirty = true;
            self.status = format!("Created {}", self.tool.label().to_lowercase());
        }
        self.tool = Tool::Select;
        self.create_preview = None;
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
