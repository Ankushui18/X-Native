//! Navigation Rail - Workflow-based left navigation
//! 
//! Replaces traditional tab system with 7-item workflow navigation
//! Each item opens a dedicated workspace in the adjacent panel

use egui::{self, Color32, Rect, Response, Sense, Ui, Vec2};
use super::tokens::*;

/// Navigation rail state
#[derive(Default)]
pub struct NavRail {
    /// Currently selected workflow index
    pub selected: usize,
    /// Hovered item index (for tooltips)
    pub hovered: Option<usize>,
}

impl NavRail {
    pub fn new() -> Self {
        Self {
            selected: 0, // Default to Files
            hovered: None,
        }
    }

    /// Render the navigation rail
    pub fn render(&mut self, ui: &mut Ui) -> Option<usize> {
        let mut new_selection = None;
        
        ui.vertical(|ui| {
            ui.add_space(SPACE_2);
            
            // X-Native Logo / Brand
            self.render_logo(ui);
            
            ui.add_space(SPACE_4);
            
            // Divider
            ui.separator();
            
            ui.add_space(SPACE_3);
            
            // Navigation items
            for (index, (icon, label)) in NAV_ITEMS.iter().enumerate() {
                let response = self.render_nav_item(ui, index, icon, label);
                
                if response.clicked() {
                    new_selection = Some(index);
                }
                
                if response.hovered() {
                    self.hovered = Some(index);
                }
            }
            
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(SPACE_2);
                
                // Settings at bottom
                let settings_resp = self.render_nav_item(ui, NAV_ITEMS.len() - 1, "⚙", "Settings");
                if settings_resp.clicked() {
                    new_selection = Some(NAV_ITEMS.len() - 1);
                }
            });
        });
        
        if let Some(new_sel) = new_selection {
            self.selected = new_sel;
        }
        
        new_selection
    }

    fn render_logo(&self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(SPACE_3);
            
            // Simple X logo
            ui.label(egui::RichText::new("X")
                .font(egui::FontId::proportional(24.0))
                .color(C_ACCENT)
                .strong());
            
            ui.add_space(SPACE_1);
            
            // Version or status indicator
            ui.label(egui::RichText::new("●")
                .font(egui::FontId::proportional(FONT_SIZE_MICRO))
                .color(Color32::from_rgb(82, 204, 150))); // Success green
        });
    }

    fn render_nav_item(&self, ui: &mut Ui, index: usize, icon: &str, label: &str) -> Response {
        let is_selected = index == self.selected;
        let is_hovered = self.hovered == Some(index);
        
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(NAV_RAIL_W - SPACE_2, ROW_HEIGHT_STD),
            Sense::click(),
        );
        
        // Background
        let bg_color = if is_selected {
            C_SELECTED
        } else if is_hovered {
            C_HOVERBG
        } else {
            Color32::TRANSPARENT
        };
        
        ui.painter().rect_filled(
            rect.shrink(SPACE_1),
            egui::CornerRadius::from(RADIUS_MD as i8),
            bg_color,
        );
        
        // Icon
        let icon_color = if is_selected {
            C_ACCENT
        } else if is_hovered {
            C_TEXT_PRIMARY
        } else {
            C_TEXT_SECONDARY
        };
        
        let icon_rect = Rect::from_center_size(
            rect.center(),
            Vec2::new(20.0, 20.0),
        );
        
        ui.painter().text(
            icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            *icon,
            egui::FontId::proportional(18.0),
            icon_color,
        );
        
        // Tooltip on hover
        if is_hovered && !response.has_focus() {
            egui::show_tooltip_at_pointer(ui.ctx(), ui.id().with("tooltip"), |ui| {
                ui.label(egui::RichText::new(*label)
                    .font(egui::FontId::proportional(FONT_SIZE_LABEL))
                    .color(C_TEXT_PRIMARY));
            });
        }
        
        response
    }

    /// Get the current workflow name
    pub fn current_workflow(&self) -> &str {
        NAV_ITEMS.get(self.selected).map(|(_, name)| *name).unwrap_or("Files")
    }

    /// Get icon for current workflow
    pub fn current_icon(&self) -> &str {
        NAV_ITEMS.get(self.selected).map(|(icon, _)| *icon).unwrap_or("◇")
    }
}
