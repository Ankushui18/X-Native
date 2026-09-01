// Inspector Module - Dynamic, resizable properties panel
// Replaces hardcoded IY_* constants with auto-calculating sections

use crate::ui::tokens::*;

pub struct Inspector {
    pub width: f64,
    pub sections: Vec<InspectorSection>,
}

pub enum InspectorSection {
    SelectionHeader,
    Position,
    Layout,
    Appearance,
    Fill,
    Stroke,
    Effects,
    Typography,
    Component,
    Export,
}

impl InspectorSection {
    /// Calculate section height dynamically based on content
    pub fn height(&self) -> f64 {
        match self {
            Self::SelectionHeader => 48.0,
            Self::Position => 120.0,
            Self::Layout => 140.0,
            Self::Appearance => 80.0,
            Self::Fill => 100.0,
            Self::Stroke => 120.0,
            Self::Effects => 100.0,
            Self::Typography => 160.0,
            Self::Component => 90.0,
            Self::Export => 70.0,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::SelectionHeader => "",
            Self::Position => "Position",
            Self::Layout => "Responsive Layout",
            Self::Appearance => "Appearance",
            Self::Fill => "Fill",
            Self::Stroke => "Stroke",
            Self::Effects => "Effects",
            Self::Typography => "Typography",
            Self::Component => "Component",
            Self::Export => "Export",
        }
    }

    pub fn is_collapsible(&self) -> bool {
        !matches!(self, Self::SelectionHeader)
    }
}

impl Inspector {
    pub fn new() -> Self {
        Self {
            width: INSPECTOR_DEFAULT_W,
            sections: vec![
                InspectorSection::SelectionHeader,
                InspectorSection::Position,
                InspectorSection::Layout,
                InspectorSection::Appearance,
                InspectorSection::Fill,
                InspectorSection::Stroke,
                InspectorSection::Effects,
                InspectorSection::Typography,
                InspectorSection::Component,
                InspectorSection::Export,
            ],
        }
    }

    /// Get total height needed for all sections
    pub fn total_height(&self) -> f64 {
        self.sections.iter().map(|s| s.height()).sum()
    }

    /// Get Y position for a specific section (auto-calculated)
    pub fn section_y(&self, index: usize) -> f64 {
        self.sections[..index].iter().map(|s| s.height()).sum()
    }

    /// Resize inspector width within bounds
    pub fn set_width(&mut self, width: f64) {
        self.width = width.clamp(INSPECTOR_MIN_W, INSPECTOR_MAX_W);
    }

    /// Toggle section collapse state
    pub fn toggle_section(&mut self, index: usize) {
        // Future: implement collapse/expand logic
    }
}
