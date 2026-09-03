// Phase P0: Core Feature Implementations
// Aligned with current x-core structures (node.rs, geometry.rs)

use std::collections::HashMap;
use peniko::Color; // Use vello Color directly
use crate::node::{Node, NodeKind, TextMetrics};
use crate::layout_types::AutoLayout;
use crate::components::{ComponentProperty, ComponentPropertyType};

// -----------------------------------------------------------------------------
// 1. Text Selection & Editing Logic
// -----------------------------------------------------------------------------

/// Represents a range of selected text characters
#[derive(Debug, Clone, PartialEq)]
pub struct TextSelection {
    pub start_index: usize,
    pub end_index: usize,
}

impl TextSelection {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start_index: start.min(end), end_index: start.max(end) }
    }
    
    pub fn is_caret(&self) -> bool {
        self.start_index == self.end_index
    }
}

/// Hit-test text to find character index at point (local coordinates)
pub fn hit_test_text(node: &Node, x: f64, y: f64) -> Option<usize> {
    if let NodeKind::Text { text } = &node.kind {
        let metrics = node.text_metrics.as_ref()?;
        if metrics.line_count == 0 { return Some(0); }
        
        let line_height = metrics.line_height.max(1.0);
        let avg_char_width = if text.is_empty() { 10.0 } else { metrics.actual_width / text.len() as f64 };
        
        let col = (x / avg_char_width).clamp(0.0, text.len() as f64) as usize;
        let row = (y / line_height).clamp(0.0, metrics.line_count as f64) as usize;
        
        let chars_per_line = text.len().checked_div(metrics.line_count).unwrap_or(text.len());
        Some((row * chars_per_line + col).min(text.len()))
    } else {
        None
    }
}

// -----------------------------------------------------------------------------
// 2. Auto-Layout Engine Extensions
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub final_width: f64,
    pub final_height: f64,
    pub wrapped_lines: usize,
    pub baseline_offset: Option<f64>,
}

pub fn calculate_auto_layout_wrap(
    parent: &Node, 
    children: &[Node], 
    max_width: Option<f64>
) -> LayoutResult {
    let layout = match &parent.kind {
        NodeKind::Frame { layout: Some(l) } => l,
        _ => return LayoutResult { 
            final_width: parent.w, 
            final_height: parent.h, 
            wrapped_lines: 1, 
            baseline_offset: None 
        },
    };
    
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut line_height: f64 = 0.0;
    let mut wrapped_lines = 1;
    let container_width = max_width.unwrap_or(parent.w);
    
    for child in children {
        let child_w = child.w;
        let child_h = child.h;
        
        // Check if wrap is enabled (using 'wrap' field if available, otherwise skip wrapping)
        let should_wrap = false; // Simplified: wrapping logic removed to match current AutoLayout fields
        
        if should_wrap && current_x + child_w > container_width && current_x > 0.0 {
            current_x = 0.0;
            current_y += line_height;
            line_height = 0.0;
            wrapped_lines += 1;
        }
        
        current_x += child_w + layout.gap;
        line_height = line_height.max(child_h);
    }
    
    let final_height = current_y + line_height;
    let final_width = container_width;
    
    // Baseline alignment not yet implemented in current AutoLayout struct
    let baseline_offset: Option<f64> = None;
    
    LayoutResult { final_width, final_height, wrapped_lines, baseline_offset }
}

// -----------------------------------------------------------------------------
// 3. Component Property Resolution
// -----------------------------------------------------------------------------

pub fn apply_component_properties_simple(
    instance: &mut Node, 
    properties: &[ComponentProperty],
    overrides: &HashMap<String, String>
) {
    for prop in properties {
        if let Some(value_str) = overrides.get(&prop.name) {
            apply_single_property(instance, prop, value_str);
        }
    }
}

fn apply_single_property(
    node: &mut Node,
    prop: &ComponentProperty,
    value_str: &str
) {
    match &prop.prop_type {
        ComponentPropertyType::Boolean { .. } => {
            if let Ok(visible) = value_str.parse::<bool>() {
                node.visible = visible;
            }
        },
        ComponentPropertyType::Text { .. } => {
            if let NodeKind::Text { text: ref mut t } = &mut node.kind {
                *t = value_str.to_string();
            }
        },
        ComponentPropertyType::Color { .. } => {
            // Parse and apply color - simplified for now
            if let Some(_color) = crate::parse_hex_color(value_str) {
                // Apply color to fill
            }
        },
        _ => {}
    }
}

// -----------------------------------------------------------------------------
// 5. Export Extensions
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    pub format: String,
    pub scale: f64,
    pub quality: u8,
    pub suffix: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: "png".to_string(),
            scale: 1.0,
            quality: 90,
            suffix: "".to_string(),
        }
    }
}

pub fn generate_multi_scale_exports(_base_name: &str) -> Vec<ExportSettings> {
    vec![
        ExportSettings { format: "png".to_string(), scale: 1.0, suffix: "".to_string(), quality: 90 },
        ExportSettings { format: "png".to_string(), scale: 2.0, suffix: "@2x".to_string(), quality: 90 },
        ExportSettings { format: "png".to_string(), scale: 3.0, suffix: "@3x".to_string(), quality: 90 },
    ]
}

#[derive(Debug, Clone)]
pub struct BatchExportConfig {
    pub nodes: Vec<String>,
    pub formats: Vec<String>,
    pub scales: Vec<f64>,
    pub output_dir: String,
}

// -----------------------------------------------------------------------------
// 6. Prototyping Logic (Simplified without serde_json)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InteractionCondition {
    pub variable_name: String,
    pub operator: String,
    pub value_string: String,
}

#[derive(Debug, Clone)]
pub struct ConditionalAction {
    pub condition: InteractionCondition,
    pub true_action: String,
    pub false_action: Option<String>,
}

pub fn evaluate_condition_simple(
    condition: &InteractionCondition,
    variables: &HashMap<String, String>
) -> bool {
    let var_value = match variables.get(&condition.variable_name) {
        Some(v) => v,
        None => return false,
    };
    
    match condition.operator.as_str() {
        "equals" => var_value == &condition.value_string,
        "not_equals" => var_value != &condition.value_string,
        "greater_than" => {
            if let (Ok(a), Ok(b)) = (var_value.parse::<f64>(), condition.value_string.parse::<f64>()) {
                a > b
            } else { false }
        },
        "less_than" => {
            if let (Ok(a), Ok(b)) = (var_value.parse::<f64>(), condition.value_string.parse::<f64>()) {
                a < b
            } else { false }
        },
        _ => false
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_selection() {
        let sel = TextSelection::new(5, 2);
        assert_eq!(sel.start_index, 2);
        assert_eq!(sel.end_index, 5);
        assert!(!sel.is_caret());
    }
    
    #[test]
    fn test_multi_scale_exports() {
        let exports = generate_multi_scale_exports("icon");
        assert_eq!(exports.len(), 3);
        assert_eq!(exports[1].scale, 2.0);
        assert_eq!(exports[1].suffix, "@2x");
    }
}
