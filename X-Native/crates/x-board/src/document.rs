//! Board Document - separate from Design documents
//! Lighter weight, no Auto Layout/Components/Variables

use crate::{BoardNode, Connector};
use serde::{Deserialize, Serialize};

/// Board-specific document kind
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum BoardKind {
    #[default]
    Brainstorm,
    UserFlow,
    Wireframe,
    MindMap,
    Whiteboard,
}

/// Board-specific document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardDocument {
    pub metadata: BoardMetadata,
    pub pages: Vec<BoardPage>,
    pub kind: BoardKind,
    /// Board-specific settings (infinite canvas, grid visibility, etc.)
    pub settings: BoardSettings,
    /// Shared stickies library (optional)
    pub sticky_styles: Vec<StickyStyle>,
}

/// Simple board metadata without heavy design file features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardMetadata {
    pub name: String,
    pub version: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardPage {
    pub id: String,
    pub name: String,
    pub nodes: Vec<BoardNode>,
    /// Connectors between nodes (board-specific)
    pub connectors: Vec<Connector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSettings {
    /// Infinite canvas enabled (no fixed artboards)
    pub infinite_canvas: bool,
    /// Show dot grid background
    pub show_grid: bool,
    /// Grid size in points
    pub grid_size: f32,
    /// Snap to grid
    pub snap_to_grid: bool,
    /// Show connector lines
    pub show_connectors: bool,
}

impl Default for BoardSettings {
    fn default() -> Self {
        Self {
            infinite_canvas: true,
            show_grid: true,
            grid_size: 20.0,
            snap_to_grid: false,
            show_connectors: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyStyle {
    pub id: String,
    pub name: String,
    pub background_color: [f32; 4],
    pub text_color: [f32; 4],
    pub font_size: f32,
}

impl BoardDocument {
    pub fn new(name: &str, kind: BoardKind) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            metadata: BoardMetadata {
                name: name.to_string(),
                version: "1.0".to_string(),
                created_at: now.clone(),
                modified_at: now,
            },
            pages: vec![BoardPage {
                id: "page-1".to_string(),
                name: "Page 1".to_string(),
                nodes: Vec::new(),
                connectors: Vec::new(),
            }],
            kind,
            settings: BoardSettings::default(),
            sticky_styles: Self::default_sticky_styles(),
        }
    }

    fn default_sticky_styles() -> Vec<StickyStyle> {
        vec![
            StickyStyle {
                id: "yellow".to_string(),
                name: "Yellow".to_string(),
                background_color: [1.0, 0.95, 0.6, 1.0],
                text_color: [0.2, 0.2, 0.2, 1.0],
                font_size: 14.0,
            },
            StickyStyle {
                id: "blue".to_string(),
                name: "Blue".to_string(),
                background_color: [0.6, 0.8, 1.0, 1.0],
                text_color: [0.1, 0.1, 0.2, 1.0],
                font_size: 14.0,
            },
            StickyStyle {
                id: "green".to_string(),
                name: "Green".to_string(),
                background_color: [0.7, 0.95, 0.7, 1.0],
                text_color: [0.1, 0.2, 0.1, 1.0],
                font_size: 14.0,
            },
            StickyStyle {
                id: "pink".to_string(),
                name: "Pink".to_string(),
                background_color: [1.0, 0.8, 0.9, 1.0],
                text_color: [0.2, 0.1, 0.15, 1.0],
                font_size: 14.0,
            },
        ]
    }

    pub fn add_page(&mut self, name: &str) -> &mut BoardPage {
        let id = format!("page-{}", self.pages.len() + 1);
        self.pages.push(BoardPage {
            id,
            name: name.to_string(),
            nodes: Vec::new(),
            connectors: Vec::new(),
        });
        self.pages.last_mut().expect("page was just pushed")
    }

    pub fn current_page(&self) -> &BoardPage {
        &self.pages[0] // Simplified - would track active page
    }

    pub fn current_page_mut(&mut self) -> &mut BoardPage {
        &mut self.pages[0]
    }

    /// Add a node to the current page
    pub fn add_node(&mut self, node: BoardNode) {
        self.current_page_mut().nodes.push(node);
    }

    /// Add a connector to the current page's connector store
    pub fn add_connector(&mut self, connector: Connector) {
        self.current_page_mut().connectors.push(connector);
    }

    /// Set selection to specific nodes
    pub fn set_selection(&mut self, node_ids: Vec<String>) {
        // Store selection in page or document level
        // For now, we'll track it via selected node IDs
        // In a full implementation, this would be stored in page state
        let _ = node_ids; // Placeholder for selection tracking
    }

    /// Marquee select nodes within a rectangle
    pub fn marquee_select(&mut self, rect: x_core::Rect) {
        let mut selected = Vec::new();
        for node in &self.current_page().nodes {
            let bbox = node.bounding_box();
            if x_core::intersects(rect, bbox) {
                selected.push(node.id().clone());
            }
        }
        self.set_selection(selected);
    }

    /// Find nearest node at a world position
    pub fn find_nearest_node(&self, point: glam::Vec2) -> Option<String> {
        use x_core::Rect;

        // Small hit tolerance
        let tol = 10.0;
        let hit_rect = Rect::new(
            point.x as f64 - tol,
            point.y as f64 - tol,
            point.x as f64 + tol,
            point.y as f64 + tol,
        );

        for node in &self.current_page().nodes {
            let bbox = node.bounding_box();
            if x_core::intersects(hit_rect, bbox) {
                return Some(node.id().clone());
            }
        }
        None
    }
}
