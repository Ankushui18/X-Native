//! Main Application Shell
//! 
//! Orchestrates all UI components: nav rail, top bar, canvas, inspector, toolbar

use super::tokens::*;
use crate::{Scene, Rect, fill_rect, Color};

/// Draw the navigation rail
pub fn draw_nav_rail(ui: &mut Scene, win_h: f64, selected_index: usize) {
    // Nav rail background
    fill_rect(ui, Rect::new(0.0, 0.0, NAV_RAIL_W as f64, win_h), C_BG_SECONDARY);
    fill_rect(ui, Rect::new(NAV_RAIL_W as f64 - 1.0, 0.0, NAV_RAIL_W as f64, win_h), C_BORDER_SUBTLE);
    
    // Logo/brand area at top
    let logo_y = 12.0;
    fill_rect(ui, Rect::new(16.0, logo_y, 40.0, logo_y + 24.0), C_ACCENT);
    
    // Navigation items
    let nav_items = [
        ("Files", "◇"),
        ("Assets", "◆"),
        ("Components", "◈"),
        ("Variables", "◎"),
        ("Styles", "◌"),
        ("Libraries", "⌁"),
    ];
    
    let item_height = 48.0;
    let start_y = 60.0;
    
    for (i, (label, icon)) in nav_items.iter().enumerate() {
        let y = start_y + i as f64 * item_height;
        let is_selected = i == selected_index;
        
        // Background for selected/hover
        if is_selected {
            fill_rect(ui, Rect::new(8.0, y, NAV_RAIL_W as f64 - 16.0, y + 32.0), C_SELECTED);
            fill_rect(ui, Rect::new(8.0, y + 32.0 - 2.0, NAV_RAIL_W as f64 - 16.0, y + 32.0), C_ACCENT);
        }
        
        // Icon position (placeholder for actual icon rendering)
        let _icon_x = (NAV_RAIL_W as f64 - 16.0) / 2.0;
        
        // Tooltip on hover would be implemented here
    }
    
    // Settings at bottom
    let settings_y = win_h - 60.0;
    fill_rect(ui, Rect::new(16.0, settings_y, 40.0, settings_y + 32.0), C_HOVERBG);
}

/// Main shell layout calculator
pub struct ShellLayout {
    pub nav_rail_x: f64,
    pub nav_rail_w: f64,
    pub top_bar_y: f64,
    pub top_bar_h: f64,
    pub left_panel_x: f64,
    pub left_panel_w: f64,
    pub canvas_x: f64,
    pub canvas_y: f64,
    pub canvas_w: f64,
    pub canvas_h: f64,
    pub inspector_x: f64,
    pub inspector_w: f64,
    pub toolbar_y: f64,
    pub toolbar_h: f64,
}

impl ShellLayout {
    pub fn calculate(win_w: f64, win_h: f64, inspector_w: f64) -> Self {
        let nav_rail_w = NAV_RAIL_W as f64;
        let top_bar_h = TOP_BAR_H as f64;
        let toolbar_h = TOOLBAR_H as f64;
        
        // Left panel (layers/assets/components) - resizable in future
        let left_panel_w = LEFT_PANEL_W as f64;
        
        // Canvas area
        let canvas_x = nav_rail_w + left_panel_w;
        let canvas_y = top_bar_h;
        let canvas_w = win_w - nav_rail_w - left_panel_w - inspector_w;
        let canvas_h = win_h - top_bar_h - toolbar_h;
        
        // Inspector
        let inspector_x = win_w - inspector_w;
        
        // Toolbar at bottom
        let toolbar_y = win_h - toolbar_h;
        
        Self {
            nav_rail_x: 0.0,
            nav_rail_w,
            top_bar_y: 0.0,
            top_bar_h,
            left_panel_x: nav_rail_w,
            left_panel_w,
            canvas_x,
            canvas_y,
            canvas_w,
            canvas_h,
            inspector_x,
            inspector_w,
            toolbar_y,
            toolbar_h,
        }
    }
    
    pub fn canvas_rect(&self) -> Rect {
        Rect::new(self.canvas_x, self.canvas_y, self.canvas_x + self.canvas_w, self.canvas_y + self.canvas_h)
    }
}

/// Draw the complete application shell backgrounds
pub fn draw_shell_backgrounds(
    ui: &mut Scene,
    layout: &ShellLayout,
    win_w: f64,
    win_h: f64,
) {
    // Overall background
    fill_rect(ui, Rect::new(0.0, 0.0, win_w, win_h), C_BG);
    
    // Top bar background
    fill_rect(ui, Rect::new(layout.nav_rail_w, 0.0, win_w - layout.nav_rail_w, layout.top_bar_h), C_PANEL2);
    fill_rect(ui, Rect::new(layout.nav_rail_w, layout.top_bar_h - 1.0, win_w - layout.nav_rail_w, layout.top_bar_h), C_PANEL_EDGE);
    
    // Left panel background
    fill_rect(ui, Rect::new(layout.left_panel_x, layout.top_bar_h, layout.left_panel_w, win_h - layout.top_bar_h - layout.toolbar_h), C_PANEL);
    fill_rect(ui, Rect::new(layout.left_panel_x + layout.left_panel_w - 1.0, layout.top_bar_h, layout.left_panel_x + layout.left_panel_w, win_h - layout.top_bar_h - layout.toolbar_h), C_PANEL_EDGE);
    
    // Inspector background
    fill_rect(ui, Rect::new(layout.inspector_x, layout.top_bar_h, layout.inspector_w, win_h - layout.top_bar_h - layout.toolbar_h), C_PANEL);
    fill_rect(ui, Rect::new(layout.inspector_x - 1.0, layout.top_bar_h, layout.inspector_x, win_h - layout.top_bar_h - layout.toolbar_h), C_PANEL_EDGE);
    
    // Toolbar background
    fill_rect(ui, Rect::new(layout.nav_rail_w, layout.toolbar_y, win_w - layout.nav_rail_w, layout.toolbar_h), C_PANEL2);
    fill_rect(ui, Rect::new(layout.nav_rail_w, layout.toolbar_y - 1.0, win_w - layout.nav_rail_w, layout.toolbar_y), C_PANEL_EDGE);
}

/// Draw horizontal separator line
pub fn draw_separator(ui: &mut Scene, x: f64, y: f64, w: f64, color: Color) {
    fill_rect(ui, Rect::new(x, y, x + w, y + 1.0), color);
}

/// Draw vertical separator line
pub fn draw_v_separator(ui: &mut Scene, x: f64, y: f64, h: f64, color: Color) {
    fill_rect(ui, Rect::new(x, y, x + 1.0, y + h), color);
}