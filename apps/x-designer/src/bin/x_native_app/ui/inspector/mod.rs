//! Dynamic Inspector System
//! 
//! Auto-calculating sections instead of hardcoded Y positions
//! Each section calculates its own height based on content

use egui::{self, Color32, Rect, Response, Sense, Ui, Vec2};
use super::tokens::*;

/// Inspector section types
#[derive(Clone, Copy, PartialEq)]
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
    Image,
    Styles,
    Constraints,
    Export,
}

impl InspectorSection {
    pub fn title(&self) -> &'static str {
        match self {
            Self::SelectionHeader => "Selection",
            Self::Position => "Position",
            Self::Layout => "Responsive Layout",
            Self::Appearance => "Appearance",
            Self::Fill => "Fill",
            Self::Stroke => "Stroke",
            Self::Effects => "Effects",
            Self::Typography => "Typography",
            Self::Component => "Component",
            Self::Image => "Image",
            Self::Styles => "Styles",
            Self::Constraints => "Constraints",
            Self::Export => "Export",
        }
    }

    pub fn default_height(&self) -> f32 {
        match self {
            Self::SelectionHeader => 60.0,
            Self::Position => 120.0,
            Self::Layout => 140.0,
            Self::Appearance => 80.0,
            Self::Fill => 100.0,
            Self::Stroke => 100.0,
            Self::Effects => 90.0,
            Self::Typography => 160.0,
            Self::Component => 120.0,
            Self::Image => 110.0,
            Self::Styles => 70.0,
            Self::Constraints => 90.0,
            Self::Export => 100.0,
        }
    }
}

/// Inspector state with resizable width
#[derive(Default)]
pub struct Inspector {
    /// Current width (resizable)
    pub width: f32,
    /// Currently expanded sections
    pub expanded: Vec<InspectorSection>,
    /// Dragging state for resize handle
    pub is_resizing: bool,
    /// Sections that are visible (based on selection)
    pub visible_sections: Vec<InspectorSection>,
}

impl Inspector {
    pub fn new() -> Self {
        let mut inspector = Self {
            width: INSPECTOR_W_DEFAULT,
            expanded: vec![
                InspectorSection::SelectionHeader,
                InspectorSection::Position,
                InspectorSection::Layout,
                InspectorSection::Fill,
            ],
            is_resizing: false,
            visible_sections: vec![
                InspectorSection::SelectionHeader,
                InspectorSection::Position,
                InspectorSection::Layout,
                InspectorSection::Appearance,
                InspectorSection::Fill,
                InspectorSection::Stroke,
                InspectorSection::Effects,
                InspectorSection::Typography,
                InspectorSection::Component,
                InspectorSection::Styles,
                InspectorSection::Constraints,
                InspectorSection::Export,
            ],
        };
        
        // Expand all by default for now
        inspector.expanded = inspector.visible_sections.clone();
        
        inspector
    }

    /// Render the inspector with dynamic sections
    pub fn render(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Header
            self.render_header(ui);
            
            ui.add_space(SPACE_2);
            
            // Scrollable area for sections
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for section in &self.visible_sections {
                            self.render_section(ui, *section);
                        }
                    });
                });
        });
    }

    fn render_header(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Properties")
                .font(egui::FontId::proportional(FONT_SIZE_SECTION))
                .color(C_TEXT_PRIMARY)
                .strong());
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Width resize indicator
                ui.label(egui::RichText::new(format!("{:.0}px", self.width))
                    .font(egui::FontId::proportional(FONT_SIZE_MICRO))
                    .color(C_TEXT_SECONDARY));
            });
        });
        
        // Separator
        ui.add_space(SPACE_1);
        ui.separator();
    }

    fn render_section(&mut self, ui: &mut Ui, section: InspectorSection) {
        let is_expanded = self.expanded.contains(&section);
        let section_id = ui.id().with(section as usize);
        
        // Section header (clickable to expand/collapse)
        let header_response = ui.horizontal(|ui| {
            // Expand/collapse icon
            let icon = if is_expanded { "▼" } else { "▶" };
            ui.label(egui::RichText::new(icon)
                .font(egui::FontId::proportional(FONT_SIZE_MICRO))
                .color(C_TEXT_SECONDARY));
            
            // Section title
            ui.label(egui::RichText::new(section.title())
                .font(egui::FontId::proportional(FONT_SIZE_LABEL))
                .color(C_TEXT_PRIMARY)
                .strong());
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Optional: add section-specific actions here
            });
        }).response;
        
        // Toggle expansion on click
        if header_response.clicked() {
            if is_expanded {
                self.expanded.retain(|&s| s != section);
            } else {
                self.expanded.push(section);
            }
        }
        
        // Section content (if expanded)
        if is_expanded {
            ui.indent(section_id, |ui| {
                ui.add_space(SPACE_2);
                
                // Render section-specific content
                self.render_section_content(ui, section);
                
                ui.add_space(SPACE_3);
                
                // Separator between sections
                ui.separator();
            });
        } else {
            ui.add_space(SPACE_1);
            ui.separator();
        }
    }

    fn render_section_content(&self, ui: &mut Ui, section: InspectorSection) {
        match section {
            InspectorSection::SelectionHeader => {
                ui.label(egui::RichText::new("Rectangle")
                    .font(egui::FontId::proportional(FONT_SIZE_LABEL))
                    .color(C_TEXT_PRIMARY));
                ui.label(egui::RichText::new("Layer 1")
                    .font(egui::FontId::proportional(FONT_SIZE_CAPTION))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Position => {
                self.render_position_fields(ui);
            }
            InspectorSection::Layout => {
                self.render_layout_fields(ui);
            }
            InspectorSection::Appearance => {
                ui.label(egui::RichText::new("Opacity: 100%")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Fill => {
                self.render_fill_fields(ui);
            }
            InspectorSection::Stroke => {
                ui.label(egui::RichText::new("No stroke")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Effects => {
                ui.label(egui::RichText::new("No effects")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Typography => {
                ui.label(egui::RichText::new("Inter Regular")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Component => {
                ui.label(egui::RichText::new("Main component")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_ACCENT));
            }
            InspectorSection::Styles => {
                ui.label(egui::RichText::new("No styles applied")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Constraints => {
                ui.label(egui::RichText::new("Scale")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            InspectorSection::Export => {
                ui.label(egui::RichText::new("Not configured")
                    .font(egui::FontId::proportional(FONT_SIZE_BODY))
                    .color(C_TEXT_SECONDARY));
            }
            _ => {}
        }
    }

    fn render_position_fields(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.render_field(ui, "X", "100");
            self.render_field(ui, "Y", "200");
        });
        ui.add_space(SPACE_2);
        ui.horizontal(|ui| {
            self.render_field(ui, "W", "320");
            self.render_field(ui, "H", "240");
        });
        ui.add_space(SPACE_2);
        self.render_field(ui, "Rotation", "0°");
    }

    fn render_layout_fields(&self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Responsive Layout")
            .font(egui::FontId::proportional(FONT_SIZE_CAPTION))
            .color(C_ACCENT));
        ui.add_space(SPACE_2);
        ui.label(egui::RichText::new("Auto-layout not enabled")
            .font(egui::FontId::proportional(FONT_SIZE_BODY))
            .color(C_TEXT_SECONDARY));
    }

    fn render_fill_fields(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Color preview
            ui.painter().rect_filled(
                egui::Rect::from_min_size(ui.available_rect_before_wrap().min, Vec2::new(20.0, 20.0)),
                egui::CornerRadius::from(RADIUS_SM as i8),
                C_ACCENT,
            );
            ui.add_space(SPACE_2);
            ui.label(egui::RichText::new("#7C5CFC")
                .font(egui::FontId::proportional(FONT_SIZE_BODY))
                .color(C_TEXT_PRIMARY));
        });
    }

    fn render_field(&self, ui: &mut Ui, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label)
                .font(egui::FontId::proportional(FONT_SIZE_CAPTION))
                .color(C_TEXT_SECONDARY));
            ui.add_space(SPACE_1);
            ui.label(egui::RichText::new(value)
                .font(egui::FontId::monospace(FONT_SIZE_BODY))
                .color(C_TEXT_PRIMARY));
        });
    }

    /// Calculate total height needed for all visible sections
    pub fn calculate_total_height(&self) -> f32 {
        let mut total = 60.0; // Header + padding
        
        for section in &self.visible_sections {
            total += SPACE_2; // Padding before section
            total += 24.0; // Header height
            total += SPACE_2; // Padding after header
            
            if self.expanded.contains(section) {
                total += section.default_height();
                total += SPACE_3; // Content padding
                total += 1.0; // Separator
            } else {
                total += SPACE_1; // Minimal space when collapsed
                total += 1.0; // Separator
            }
        }
        
        total
    }

    /// Get current width for resizing
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Set width (called by resize handler)
    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(INSPECTOR_W_MIN, INSPECTOR_W_MAX);
    }
}
