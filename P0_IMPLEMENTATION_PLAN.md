# P0 Critical Gaps Implementation Plan

This document provides detailed implementation plans for the 7 critical P0 features blocking professional use of X-Native. Each section includes current state, what's missing, and step-by-step implementation guidance.

---

## 1. Text Selection & Editing - Mouse Hit-Mapping for Text Ranges

### Current State
- Basic text node rendering exists in `x-render/src/ir.rs` (Glyphs command)
- Hit testing in `x-editor/src/selection.rs` only handles bounding boxes
- No character-level hit detection or text range selection

### What's Missing
- Character position to screen coordinate mapping
- Screen coordinate to character index reverse mapping  
- Visual text selection highlight rendering
- Text cursor positioning and blinking
- Keyboard navigation (arrow keys, word jumps, line jumps)
- Copy/cut/paste within text editor

### Implementation Steps

#### Step 1.1: Add Text Layout Metrics to x-core
**File**: `/workspace/X-Native/crates/x-core/src/node.rs`

Add text metrics tracking:
```rust
pub struct TextMetrics {
    pub font_size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub max_width: f64,
    pub actual_width: f64,
    pub actual_height: f64,
    pub line_count: usize,
    pub caret_graphemes: Vec<(usize, f64, f64)>, // (char_index, x, y)
}
```

Add to Node struct:
```rust
pub text_metrics: Option<TextMetrics>,
```

#### Step 1.2: Implement Character Hit Testing
**File**: `/workspace/X-Native/crates/x-editor/src/selection.rs`

Add new functions:
```rust
/// Map screen point to character index in text node
pub fn hit_test_text(node: &Node, point: Point, world_transform: Affine) -> Option<usize> {
    if let NodeKind::Text { text } = &node.kind {
        let local = world_transform.inverse() * point;
        // Use cached caret positions or compute on fly
        if let Some(metrics) = &node.text_metrics {
            for (i, (_, x, y)) in metrics.caret_graphemes.iter().enumerate() {
                let next_y = if i + 1 < metrics.caret_graphemes.len() {
                    metrics.caret_graphemes[i + 1].2
                } else {
                    y + metrics.line_height
                };
                if local.y >= *y && local.y < next_y {
                    // Found line, now find character
                    return Some(i);
                }
            }
        }
    }
    None
}

/// Get character bounds for selection highlighting
pub fn get_char_bounds(node: &Node, start: usize, end: usize) -> Option<Vec<Rect>> {
    // Return list of rects for selected range
    None // TODO
}
```

#### Step 1.3: Add Text Edit Mode to Editor
**File**: `/workspace/X-Native/crates/x-editor/src/editor_core.rs`

Add text editing state:
```rust
pub enum TextEditMode {
    None,
    Editing { 
        node_id: String,
        selection_start: usize,
        selection_end: usize,
        cursor_visible: bool,
    },
}

pub struct Editor {
    // ... existing fields
    pub text_edit_mode: TextEditMode,
}
```

Add methods:
```rust
pub fn start_text_edit(&mut self, node_id: &str) -> bool {
    if let Some(node) = find(&self.root, node_id) {
        if matches!(node.kind, NodeKind::Text { .. }) {
            self.text_edit_mode = TextEditMode::Editing {
                node_id: node_id.to_string(),
                selection_start: 0,
                selection_end: 0,
                cursor_visible: true,
            };
            return true;
        }
    }
    false
}

pub fn update_text_selection(&mut self, start: usize, end: usize) {
    if let TextEditMode::Editing { selection_start, selection_end, .. } = &mut self.text_edit_mode {
        *selection_start = start;
        *selection_end = end;
    }
}

pub fn insert_text(&mut self, text: &str) {
    if let TextEditMode::Editing { node_id, selection_start, selection_end, .. } = &self.text_edit_mode {
        if let Some(node) = find_mut(&mut self.root, node_id) {
            if let NodeKind::Text { text: current } = &mut node.kind {
                let start = *selection_start;
                let end = *selection_end;
                current.replace_range(start..end, text);
                *selection_start = start + text.len();
                *selection_end = *selection_start;
            }
        }
    }
}
```

#### Step 1.4: Render Text Selection Highlights
**File**: `/workspace/X-Native/crates/x-render/src/ir.rs`

Modify `lower` function to emit selection highlights when in edit mode.

---

## 2. Auto Layout Complete - Wrap, Min/Max, Baseline, Absolute Children

### Current State
- Basic auto-layout exists in `x-core/src/auto_layout.rs`
- Supports horizontal/vertical direction, gap, padding
- Missing advanced features

### What's Missing
- Text wrapping (wrap mode)
- Min/max width/height constraints
- Baseline alignment
- Absolute positioned children within auto-layout frames
- Hug contents behavior
- Fill container behavior

### Implementation Steps

#### Step 2.1: Extend AutoLayout Struct
**File**: `/workspace/X-Native/crates/x-core/src/auto_layout.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoLayoutWrap {
    NoWrap,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Min,
    Center,
    Max,
    Baseline, // NEW
}

#[derive(Debug, Clone)]
pub struct AutoLayout {
    pub direction: Axis,
    pub gap: f64,
    pub padding: Padding,
    pub align_items: Alignment,
    pub justify_content: Alignment,
    pub wrap: AutoLayoutWrap, // NEW
    pub min_width: Option<f64>, // NEW
    pub max_width: Option<f64>, // NEW
    pub min_height: Option<f64>, // NEW
    pub max_height: Option<f64>, // NEW
    pub resize_on_wrap: bool, // NEW
}
```

Add child constraints:
```rust
// In Node struct or as separate struct
pub struct ChildConstraints {
    pub align_self: Option<Alignment>,
    pub grow: f64,
    pub shrink: f64,
    pub basis: Option<f64>,
    pub is_absolute: bool, // NEW - removed from normal flow
}
```

#### Step 2.2: Implement Wrap Logic
**File**: `/workspace/X-Native/crates/x-core/src/auto_layout.rs`

```rust
fn layout_wrapped(children: &[&Node], gap: f64, container_width: f64) -> Vec<LayoutLine> {
    let mut lines = vec![];
    let mut current_line = vec![];
    let mut current_width = 0.0;
    
    for child in children {
        let child_width = child.w;
        if current_width + child_width > container_width && !current_line.is_empty() {
            lines.push(LayoutLine { children: current_line, width: current_width });
            current_line = vec![];
            current_width = 0.0;
        }
        current_line.push(child);
        current_width += child_width + gap;
    }
    
    if !current_line.is_empty() {
        lines.push(LayoutLine { children: current_line, width: current_width - gap });
    }
    
    lines
}
```

#### Step 2.3: Implement Baseline Alignment
Calculate text baseline offsets and align items to baseline rather than center/min/max.

---

## 3. Component Properties - Boolean, Text, Instance-Swap Properties

### Current State
- Components and instances exist
- Basic overrides via HashMap<String, String>
- No typed property system

### What's Missing
- Boolean properties (toggle visibility, etc.)
- Text properties (editable text overrides)
- Instance-swap properties (swap component instances)
- Property panels and UI
- Nested override resolution

### Implementation Steps

#### Step 3.1: Define Component Property Types
**File**: `/workspace/X-Native/crates/x-core/src/components.rs`

```rust
#[derive(Debug, Clone)]
pub enum ComponentPropertyType {
    Boolean { default: bool },
    Text { default: String },
    InstanceSwap { allowed_components: Vec<String>, default: Option<String> },
    Color { default: Color },
    Number { default: f64, min: Option<f64>, max: Option<f64> },
}

#[derive(Debug, Clone)]
pub struct ComponentProperty {
    pub name: String,
    pub id: String, // unique within component
    pub prop_type: ComponentPropertyType,
    pub preferred_input: Option<String>, // UI hint
}

#[derive(Debug, Clone)]
pub struct ComponentDefinition {
    pub name: String,
    pub root: Node,
    pub properties: Vec<ComponentProperty>, // NEW
    pub property_bindings: HashMap<String, PropertyBinding>, // NEW
}

#[derive(Debug, Clone)]
pub struct PropertyBinding {
    pub property_id: String,
    pub target_node_id: String,
    pub target_property: String, // "visible", "text", "fill", etc.
}
```

#### Step 3.2: Resolve Properties on Instances
**File**: `/workspace/X-Native/crates/x-core/src/components.rs`

```rust
pub fn resolve_instance_properties(
    instance: &mut Node,
    component: &ComponentDefinition,
    instance_overrides: &HashMap<String, PropertyValue>,
) {
    for binding in &component.property_bindings {
        if let Some(value) = instance_overrides.get(&binding.property_id) {
            apply_property_to_node(instance, &binding.target_node_id, &binding.target_property, value);
        }
    }
}

fn apply_property_to_node(
    node: &mut Node,
    target_id: &str,
    target_prop: &str,
    value: &PropertyValue,
) {
    if let Some(target) = find_mut(node, target_id) {
        match target_prop {
            "visible" => {
                if let PropertyValue::Boolean(v) = value {
                    target.visible = *v;
                }
            }
            "text" => {
                if let PropertyValue::Text(v) = value {
                    if let NodeKind::Text { text } = &mut target.kind {
                        *text = v.clone();
                    }
                }
            }
            "swap" => {
                if let PropertyValue::InstanceSwap(component_name) = value {
                    target.kind = NodeKind::Instance { component: component_name.clone() };
                }
            }
            _ => {}
        }
    }
}
```

---

## 4. Paste Behavior - Paste-in-Place, Paste-Over-Selection

### Current State
- Basic paste exists in `x-editor/src/editor_core.rs`
- Only paste with offset into parent
- Missing Figma-style paste behaviors

### What's Missing
- Paste in place (same coordinates)
- Paste over selection (replace selected)
- Paste to current page/frame context
- System clipboard integration

### Implementation Steps

#### Step 4.1: Add Paste Variants
**File**: `/workspace/X-Native/crates/x-editor/src/editor_core.rs`

```rust
/// Paste at exact same world coordinates (paste in place)
pub fn paste_in_place(&mut self, parent_id: &str) -> Vec<String> {
    self.paste_with_strategy(parent_id, PasteStrategy::InPlace)
}

/// Paste over current selection, replacing it
pub fn paste_over_selection(&mut self, parent_id: &str) -> Vec<String> {
    // Delete current selection first
    self.delete_selection();
    // Then paste in place
    self.paste_in_place(parent_id)
}

enum PasteStrategy {
    WithOffset((f64, f64)),
    InPlace,
    CenteredInView,
}

fn paste_with_strategy(&mut self, parent_id: &str, strategy: PasteStrategy) -> Vec<String> {
    // Similar to paste but applies different offset logic
    // ...
}
```

#### Step 4.2: System Clipboard Integration
**File**: Create `/workspace/X-Native/crates/x-editor/src/clipboard.rs`

```rust
use arboard::Clipboard as SystemClipboard;

pub struct ClipboardManager {
    internal: Vec<Node>,
    system: Option<SystemClipboard>,
}

impl ClipboardManager {
    pub fn copy_to_system(&mut self, nodes: &[Node]) -> Result<(), String> {
        // Serialize to JSON/SVG
        let serialized = serde_json::to_string(nodes).map_err(|e| e.to_string())?;
        if let Some(clip) = &mut self.system {
            clip.set_text(serialized).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    
    pub fn paste_from_system(&mut self) -> Option<Vec<Node>> {
        if let Some(clip) = &mut self.system {
            if let Ok(text) = clip.get_text() {
                if let Ok(nodes) = serde_json::from_str(&text) {
                    return Some(nodes);
                }
            }
        }
        None
    }
}
```

---

## 5. Corner Radius Handles - On-Canvas Drag Handles

### Current State
- Corner radii exist (uniform and per-corner)
- No interactive handles for adjustment

### What's Missing
- Visual handle rendering at corners
- Drag interaction for radius adjustment
- Individual corner handle control
- Handle snapping and constraints

### Implementation Steps

#### Step 5.1: Calculate Handle Positions
**File**: `/workspace/X-Native/crates/x-editor/src/selection.rs`

```rust
pub struct CornerHandle {
    pub corner: Corner, // TopLeft, TopRight, BottomRight, BottomLeft
    pub position: Point, // world coordinates
    pub radius_handle_offset: Point, // where the drag handle appears
}

pub fn get_corner_handles(node: &Node, world_transform: Affine) -> Vec<CornerHandle> {
    let mut handles = vec![];
    
    let corners = [
        (Corner::TopLeft, 0.0, 0.0),
        (Corner::TopRight, node.w, 0.0),
        (Corner::BottomRight, node.w, node.h),
        (Corner::BottomLeft, 0.0, node.h),
    ];
    
    for (corner, x, y) in corners {
        let world_pos = world_transform * Point::new(x, y);
        handles.push(CornerHandle {
            corner,
            position: world_pos,
            radius_handle_offset: calculate_radius_handle_position(corner, x, y, node.w, node.h),
        });
    }
    
    handles
}

fn calculate_radius_handle_position(corner: Corner, x: f64, y: f64, w: f64, h: f64) -> Point {
    // Position handle along edge based on current radius
    match corner {
        Corner::TopLeft => Point::new(x + 20.0, y), // 20px along top edge
        Corner::TopRight => Point::new(x - 20.0, y),
        Corner::BottomRight => Point::new(x - 20.0, y),
        Corner::BottomLeft => Point::new(x + 20.0, y),
    }
}
```

#### Step 5.2: Add Corner Drag Interaction
**File**: `/workspace/X-Native/crates/x-editor/src/editor_core.rs`

```rust
pub fn start_corner_drag(&mut self, node_id: &str, corner: Corner) -> bool {
    if let Some(node) = find(&self.root, node_id) {
        if let NodeKind::Rect { radius } = node.kind {
            self.corner_drag_state = Some(CornerDragState {
                node_id: node_id.to_string(),
                corner,
                initial_radius: radius,
                initial_mouse_pos: self.last_mouse_pos,
            });
            return true;
        }
    }
    false
}

pub fn update_corner_drag(&mut self, mouse_delta: Point) {
    if let Some(state) = &mut self.corner_drag_state {
        if let Some(node) = find_mut(&mut self.root, &state.node_id) {
            if let NodeKind::Rect { radius } = &mut node.kind {
                let delta = if state.corner == Corner::TopLeft || state.corner == Corner::BottomLeft {
                    mouse_delta.x
                } else {
                    -mouse_delta.x
                };
                *radius = (*radius + delta).max(0.0).min(node.w.min(node.h) / 2.0);
            }
        }
    }
}
```

---

## 6. Vector Networks - Branching Paths, Pencil Tool, Width Profiles

### Current State
- Basic vector paths exist with PathCmd
- Simple path editing in `x-editor/src/vector_edit.rs`
- No branching/networks, no pencil, no variable width

### What's Missing
- Vector networks (multiple paths sharing points)
- Pencil/freehand drawing tool
- Width profiles (variable stroke width)
- Pen tool with bezier handles
- Join/connect/disconnect operations

### Implementation Steps

#### Step 6.1: Redesign Vector Data Structure
**File**: `/workspace/X-Native/crates/x-core/src/node.rs`

```rust
#[derive(Debug, Clone)]
pub struct VectorPoint {
    pub id: usize,
    pub position: Point,
    pub incoming: Option<Point>, // bezier handle
    pub outgoing: Option<Point>, // bezier handle
    pub point_type: PointType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointType {
    Corner,
    Smooth,
    Mirror,
    Auto,
}

#[derive(Debug, Clone)]
pub struct VectorSegment {
    pub start_point_id: usize,
    pub end_point_id: usize,
    pub stroke_width: f64,
    pub width_profile: Vec<f64>, // width at intervals along segment
}

#[derive(Debug, Clone)]
pub struct VectorNetwork {
    pub points: Vec<VectorPoint>,
    pub segments: Vec<VectorSegment>,
    pub fills: Vec<FillRegion>, // regions defined by closed loops
}

// Update NodeKind
pub enum NodeKind {
    // ... existing
    Vector { network: VectorNetwork }, // NEW
    VectorLegacy { path: Vec<PathCmd> }, // Keep for backwards compat
}
```

#### Step 6.2: Implement Pencil Tool
**File**: `/workspace/X-Native/crates/x-editor/src/vector_edit.rs`

```rust
pub struct PencilTool {
    points: Vec<Point>,
    is_drawing: bool,
    simplify_tolerance: f64,
}

impl PencilTool {
    pub fn start_drawing(&mut self, start: Point) {
        self.points = vec![start];
        self.is_drawing = true;
    }
    
    pub fn add_point(&mut self, point: Point) {
        if self.is_drawing {
            // Simplify as we go
            if should_add_point(&self.points, point, self.simplify_tolerance) {
                self.points.push(point);
            }
        }
    }
    
    pub fn finish_drawing(self) -> VectorNetwork {
        // Convert raw points to network with bezier fitting
        fit_beziers_to_points(&self.points)
    }
}

fn fit_beziers_to_points(points: &[Point]) -> VectorNetwork {
    // Use least-squares bezier fitting algorithm
    // ...
}
```

#### Step 6.3: Implement Width Profiles
Add stroke width variation along segments using pressure data or manual editing.

---

## 7. Export Completeness - JPG, Slices, Batch Export, Multi-Scale

### Current State
- Basic export exists (PNG, SVG, PDF mentioned)
- Single frame export only

### What's Missing
- JPG/JPEG export with quality settings
- Slice/export region definitions
- Batch export multiple frames/assets
- Multi-scale exports (@1x, @2x, @3x)
- Export presets and naming conventions

### Implementation Steps

#### Step 7.1: Add Export Configuration
**File**: `/workspace/X-Native/crates/x-core/src/lib.rs` or new file

```rust
#[derive(Debug, Clone)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub scale: f64,
    pub suffix: String,
    pub quality: u8, // for JPG
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    PNG,
    JPG { quality: u8 },
    SVG,
    PDF,
}

#[derive(Debug, Clone)]
pub struct ExportSlice {
    pub id: String,
    pub name: String,
    pub bounds: Rect, // relative to parent
    pub export_settings: Vec<ExportSettings>,
}

#[derive(Debug, Clone)]
pub struct ExportPreset {
    pub name: String,
    pub settings: Vec<ExportSettings>,
}
```

#### Step 7.2: Implement Multi-Format Export
**File**: `/workspace/X-Native/crates/x-render/src/sinks.rs`

```rust
pub fn export_to_jpeg(scene: &Scene, width: u32, height: u32, quality: u8) -> Vec<u8> {
    // Render scene to RGBA buffer
    let rgba = render_scene_to_rgba(scene, width, height);
    
    // Use jpeg crate to encode
    use jpeg_encoder::{ColorType, Encoder};
    let mut encoder = Encoder::new(Vec::new(), quality);
    let image = jpeg_encoder::Image::new(rgba, width, height, ColorType::Rgba);
    encoder.encode(&image).unwrap();
    encoder.into_inner()
}

pub fn export_batch(root: &Node, slices: &[ExportSlice], output_dir: &Path) -> Result<(), String> {
    for slice in slices {
        for setting in &slice.export_settings {
            let filename = format!("{}_{}{}", slice.name, setting.suffix, format_extension(setting.format));
            let path = output_dir.join(&filename);
            
            // Extract sub-region and render at scale
            let scaled_bounds = slice.bounds.inflate_by(setting.scale);
            export_node_region(root, &scaled_bounds, setting, &path)?;
        }
    }
    Ok(())
}
```

#### Step 7.3: Add Export Panel Data Structures
Track export settings per node in the document.

---

## Implementation Priority & Timeline

### Week 1-2: Foundation
- Text metrics infrastructure (1.1)
- AutoLayout extensions (2.1)
- Component property types (3.1)

### Week 3-4: Core Functionality
- Text hit testing (1.2)
- AutoLayout wrap logic (2.2)
- Property resolution (3.2)
- Paste variants (4.1)

### Week 5-6: Interaction
- Text edit mode (1.3)
- Corner handles (5.1, 5.2)
- Vector network structure (6.1)

### Week 7-8: Polish
- Export formats (7.1, 7.2)
- Pencil tool (6.2)
- System clipboard (4.2)

### Week 9-10: Testing & Bug Fixes
- Comprehensive testing
- Performance optimization
- Documentation

---

## Testing Strategy

Each feature needs:
1. Unit tests in respective crates
2. Integration tests in `x-native/tests/`
3. Manual QA test scenarios documented

Example test structure:
```rust
#[cfg(test)]
mod text_selection_tests {
    #[test]
    fn test_hit_test_single_line() { /* ... */ }
    
    #[test]
    fn test_hit_test_multiline() { /* ... */ }
    
    #[test]
    fn test_selection_keyboard_navigation() { /* ... */ }
}
```

---

## Notes

- Maintain backward compatibility with existing `.x` files
- Document all breaking changes
- Update README with new capabilities
- Consider feature flags for gradual rollout
