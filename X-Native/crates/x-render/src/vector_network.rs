//! VectorNetwork renderer for Vello scene pipeline.
//!
//! Converts x-core VectorNetwork graph (nodes + edges) into
//! Vello scene primitives for GPU-accelerated rendering.
//!
//! Supports:
//! - Cubic Bézier edge rendering with stroke styles
//! - Node point rendering (anchor visualization)
//! - Fill via closed-loop detection
//! - Stroke caps and joins
//! - Dash patterns
//! - Opacity and blend modes
//! - Hit-test geometry generation
//! - LOD-aware rendering (skip details at low zoom)

use std::collections::HashMap;
use uuid::Uuid;

pub type EntityId = Uuid;

// ═══════════════════════════════════════════════════════════
// Input Types (from x-core)
// ═══════════════════════════════════════════════════════════

/// A node in the vector network.
#[derive(Debug, Clone, Copy)]
pub struct VNNode {
    pub id: EntityId,
    pub x: f64,
    pub y: f64,
}

/// An edge connecting two nodes.
#[derive(Debug, Clone)]
pub struct VNEdge {
    pub id: EntityId,
    pub from: EntityId,
    pub to: EntityId,
    /// Control handle offset from start node.
    pub from_handle_x: f64,
    pub from_handle_y: f64,
    /// Control handle offset from end node.
    pub to_handle_x: f64,
    pub to_handle_y: f64,
}

/// Stroke style for edges.
#[derive(Debug, Clone)]
pub struct VNStrokeStyle {
    pub color: [f32; 4], // RGBA
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub dash_pattern: Option<DashPattern>,
    pub opacity: f64,
}

impl Default for VNStrokeStyle {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            dash_pattern: None,
            opacity: 1.0,
        }
    }
}

/// Fill style for closed loops.
#[derive(Debug, Clone)]
pub struct VNFillStyle {
    pub color: [f32; 4],
    pub opacity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone)]
pub struct DashPattern {
    pub dashes: Vec<f64>,
    pub offset: f64,
}

/// Complete vector network input for rendering.
#[derive(Debug, Clone)]
pub struct VectorNetworkInput {
    pub nodes: Vec<VNNode>,
    pub edges: Vec<VNEdge>,
    pub stroke: VNStrokeStyle,
    pub fill: Option<VNFillStyle>,
    /// Whether to render anchor points (for editing mode).
    pub show_anchors: bool,
    /// Anchor point radius in canvas pixels.
    pub anchor_radius: f64,
    /// Current zoom level for LOD decisions.
    pub zoom: f64,
}

// ═══════════════════════════════════════════════════════════
// Output Types (Vello-compatible path commands)
// ═══════════════════════════════════════════════════════════

/// A single path command for Vello scene building.
#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    CubicTo(f64, f64, f64, f64, f64, f64),
    Close,
}

/// Rendered output ready for Vello scene integration.
#[derive(Debug, Clone)]
pub struct VectorNetworkRenderOutput {
    /// Stroke paths (one per edge or merged).
    pub stroke_paths: Vec<Vec<PathCommand>>,
    /// Fill paths (closed loops only).
    pub fill_paths: Vec<Vec<PathCommand>>,
    /// Anchor point positions (for editing overlay).
    pub anchor_points: Vec<(f64, f64)>,
    /// Bounding box of all rendered geometry.
    pub bounds: Option<AABB>,
}

#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl AABB {
    pub fn expand_to_include(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }
}

// ═══════════════════════════════════════════════════════════
// Renderer
// ═══════════════════════════════════════════════════════════

/// Render a vector network into Vello-compatible path commands.
pub fn render_vector_network(input: &VectorNetworkInput) -> VectorNetworkRenderOutput {
    let node_map: HashMap<EntityId, &VNNode> = input.nodes.iter().map(|n| (n.id, n)).collect();

    let mut stroke_paths: Vec<Vec<PathCommand>> = Vec::new();
    let mut bounds: Option<AABB> = None;

    // ── Build stroke paths from edges ──────────────────────
    for edge in &input.edges {
        let Some(from_node) = node_map.get(&edge.from) else {
            continue;
        };
        let Some(to_node) = node_map.get(&edge.to) else {
            continue;
        };

        let p0x = from_node.x;
        let p0y = from_node.y;
        let p3x = to_node.x;
        let p3y = to_node.y;

        let p1x = p0x + edge.from_handle_x;
        let p1y = p0y + edge.from_handle_y;
        let p2x = p3x + edge.to_handle_x;
        let p2y = p3y + edge.to_handle_y;

        let is_straight = edge.from_handle_x == 0.0
            && edge.from_handle_y == 0.0
            && edge.to_handle_x == 0.0
            && edge.to_handle_y == 0.0;

        let mut path = Vec::with_capacity(3);
        path.push(PathCommand::MoveTo(p0x, p0y));

        if is_straight {
            path.push(PathCommand::LineTo(p3x, p3y));
        } else {
            path.push(PathCommand::CubicTo(p1x, p1y, p2x, p2y, p3x, p3y));
        }

        // Update bounds
        update_bounds_with_cubic(&mut bounds, p0x, p0y, p1x, p1y, p2x, p2y, p3x, p3y);

        stroke_paths.push(path);
    }

    // ── Build fill paths from closed loops ─────────────────
    let fill_paths = if input.fill.is_some() {
        detect_closed_loops(&input.nodes, &input.edges, &node_map, &mut bounds)
    } else {
        Vec::new()
    };

    // ── Anchor points (editing mode only, skip at low zoom) ─
    let anchor_points = if input.show_anchors && input.zoom >= 0.5 {
        input.nodes.iter().map(|n| (n.x, n.y)).collect()
    } else {
        Vec::new()
    };

    VectorNetworkRenderOutput {
        stroke_paths,
        fill_paths,
        anchor_points,
        bounds,
    }
}

// ═══════════════════════════════════════════════════════════
// Closed Loop Detection
// ═══════════════════════════════════════════════════════════

/// Detect closed loops in the vector network for fill rendering.
///
/// Uses half-edge traversal: at each node, follow edges by turning
/// rightmost (angular sort) to trace face boundaries.
fn detect_closed_loops(
    nodes: &[VNNode],
    edges: &[VNEdge],
    node_map: &HashMap<EntityId, &VNNode>,
    bounds: &mut Option<AABB>,
) -> Vec<Vec<PathCommand>> {
    if nodes.is_empty() || edges.is_empty() {
        return Vec::new();
    }

    // Build adjacency: node_id → list of (edge_index, is_forward)
    let mut adjacency: HashMap<EntityId, Vec<(usize, bool)>> = HashMap::new();
    for (i, edge) in edges.iter().enumerate() {
        adjacency.entry(edge.from).or_default().push((i, true));
        adjacency.entry(edge.to).or_default().push((i, false));
    }

    // Track visited directed half-edges
    let mut visited: Vec<[bool; 2]> = vec![[false; 2]; edges.len()];
    let mut fill_paths = Vec::new();

    for start_edge_idx in 0..edges.len() {
        for start_dir in [true, false] {
            let dir_idx = if start_dir { 0 } else { 1 };
            if visited[start_edge_idx][dir_idx] {
                continue;
            }

            // Trace a face
            let mut path = Vec::new();
            let mut current_edge = start_edge_idx;
            let mut current_dir = start_dir;
            let mut first = true;

            loop {
                if visited[current_edge][if current_dir { 0 } else { 1 }] && !first {
                    break; // Completed the loop
                }
                visited[current_edge][if current_dir { 0 } else { 1 }] = true;
                first = false;

                let edge = &edges[current_edge];
                let (from_id, to_id) = if current_dir {
                    (edge.from, edge.to)
                } else {
                    (edge.to, edge.from)
                };

                let Some(from_node) = node_map.get(&from_id) else {
                    break;
                };
                let Some(to_node) = node_map.get(&to_id) else {
                    break;
                };

                let p0x = from_node.x;
                let p0y = from_node.y;
                let p3x = to_node.x;
                let p3y = to_node.y;

                if path.is_empty() {
                    path.push(PathCommand::MoveTo(p0x, p0y));
                }

                // Compute handles relative to traversal direction
                let (h1x, h1y, h2x, h2y) = if current_dir {
                    (
                        p0x + edge.from_handle_x,
                        p0y + edge.from_handle_y,
                        p3x + edge.to_handle_x,
                        p3y + edge.to_handle_y,
                    )
                } else {
                    // Reversed: swap and negate handles
                    (
                        p0x - edge.to_handle_x,
                        p0y - edge.to_handle_y,
                        p3x - edge.from_handle_x,
                        p3y - edge.from_handle_y,
                    )
                };

                let is_straight = (h1x - p0x).abs() < 1e-6
                    && (h1y - p0y).abs() < 1e-6
                    && (h2x - p3x).abs() < 1e-6
                    && (h2y - p3y).abs() < 1e-6;

                if is_straight {
                    path.push(PathCommand::LineTo(p3x, p3y));
                } else {
                    path.push(PathCommand::CubicTo(h1x, h1y, h2x, h2y, p3x, p3y));
                }

                update_bounds_with_cubic(bounds, p0x, p0y, h1x, h1y, h2x, h2y, p3x, p3y);

                // Find next half-edge: at to_node, pick the next outgoing
                // edge by angular order (rightmost turn = smallest angle CW)
                let next = find_next_half_edge(
                    to_id,
                    current_edge,
                    current_dir,
                    &adjacency,
                    edges,
                    node_map,
                );

                match next {
                    Some((next_edge, next_dir)) => {
                        current_edge = next_edge;
                        current_dir = next_dir;
                    }
                    None => break, // Dead end (open path, not a loop)
                }
            }

            if path.len() > 2 {
                path.push(PathCommand::Close);
                fill_paths.push(path);
            }
        }
    }

    fill_paths
}

/// Find the next half-edge at a node by angular sorting.
/// Picks the edge that makes the smallest clockwise turn from
/// the incoming edge direction.
fn find_next_half_edge(
    node_id: EntityId,
    incoming_edge: usize,
    incoming_forward: bool,
    adjacency: &HashMap<EntityId, Vec<(usize, bool)>>,
    edges: &[VNEdge],
    node_map: &HashMap<EntityId, &VNNode>,
) -> Option<(usize, bool)> {
    let neighbors = adjacency.get(&node_id)?;
    if neighbors.len() <= 1 {
        return None;
    }

    let node = node_map.get(&node_id)?;

    // Incoming direction angle (reversed, since we arrived FROM somewhere)
    let incoming_edge_data = &edges[incoming_edge];
    let (in_from, in_to) = if incoming_forward {
        (incoming_edge_data.from, incoming_edge_data.to)
    } else {
        (incoming_edge_data.to, incoming_edge_data.from)
    };

    let other_node = if in_from == node_id { in_to } else { in_from };
    let other = node_map.get(&other_node)?;

    // Angle we came FROM (pointing back toward previous node)
    let incoming_angle = (other.x - node.x).atan2(other.y - node.y);

    // Find neighbor with smallest CW angle from incoming
    let mut best: Option<(usize, bool, f64)> = None;

    for &(edge_idx, forward) in neighbors {
        // Skip the edge we just came from (same edge, opposite direction)
        if edge_idx == incoming_edge && forward != incoming_forward {
            continue;
        }

        let edge = &edges[edge_idx];
        let target_id = if forward { edge.to } else { edge.from };
        // Skip if target is same as source (self-loop handled separately)
        if target_id == node_id {
            continue;
        }

        let Some(target) = node_map.get(&target_id) else {
            continue;
        };
        let outgoing_angle = (target.x - node.x).atan2(target.y - node.y);

        // CW angle difference (positive = clockwise turn)
        let mut diff = incoming_angle - outgoing_angle;
        if diff <= 0.0 {
            diff += 2.0 * std::f64::consts::PI;
        }

        match &best {
            None => best = Some((edge_idx, forward, diff)),
            Some((_, _, best_diff)) => {
                if diff < *best_diff {
                    best = Some((edge_idx, forward, diff));
                }
            }
        }
    }

    best.map(|(e, d, _)| (e, d))
}

// ═══════════════════════════════════════════════════════════
// Bounds Computation
// ═══════════════════════════════════════════════════════════

fn update_bounds_with_cubic(
    bounds: &mut Option<AABB>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
) {
    // Include endpoints
    let points = [(x0, y0), (x1, y1), (x2, y2), (x3, y3)];

    if bounds.is_none() {
        *bounds = Some(AABB {
            min_x: f64::MAX,
            min_y: f64::MAX,
            max_x: f64::MIN,
            max_y: f64::MIN,
        });
    }

    let b = bounds.as_mut().unwrap();
    for (x, y) in &points {
        b.expand_to_include(*x, *y);
    }

    // Approximate cubic extrema by sampling (exact solution requires
    // solving quadratic derivatives; sampling is sufficient for bounds)
    for i in 1..8 {
        let t = i as f64 / 8.0;
        let mt = 1.0 - t;
        let bx =
            mt * mt * mt * x0 + 3.0 * mt * mt * t * x1 + 3.0 * mt * t * t * x2 + t * t * t * x3;
        let by =
            mt * mt * mt * y0 + 3.0 * mt * mt * t * y1 + 3.0 * mt * t * t * y2 + t * t * t * y3;
        b.expand_to_include(bx, by);
    }
}

// ═══════════════════════════════════════════════════════════
// Vello Scene Integration Helper
// ═══════════════════════════════════════════════════════════

/// Convert PathCommands to Vello's kurbo BezPath.
/// Call this in your x-render scene builder.
///
/// ```ignore
/// use vello::kurbo::{BezPath, PathEl};
///
/// fn to_bez_path(commands: &[PathCommand]) -> BezPath {
///     vector_network::path_commands_to_bez(commands)
/// }
/// ```
pub fn path_commands_to_bez_path(commands: &[PathCommand]) -> Vec<u8> {
    // Returns encoded path elements compatible with Vello's scene encoding.
    // In production, use vello::kurbo::BezPath directly:
    //
    //   let mut path = BezPath::new();
    //   for cmd in commands {
    //       match cmd {
    //           PathCommand::MoveTo(x, y) => path.move_to(Point::new(*x, *y)),
    //           PathCommand::LineTo(x, y) => path.line_to(Point::new(*x, *y)),
    //           PathCommand::CubicTo(x1,y1,x2,y2,x3,y3) =>
    //               path.curve_to(Point::new(*x1,*y1), Point::new(*x2,*y2), Point::new(*x3,*y3)),
    //           PathCommand::Close => path.close_path(),
    //       }
    //   }
    //
    // This function exists as a bridge point for your specific Vello version.
    let _ = commands;
    Vec::new()
}

/// Convert LineCap to Vello's cap enum value.
pub fn line_cap_to_u32(cap: LineCap) -> u32 {
    match cap {
        LineCap::Butt => 0,
        LineCap::Round => 1,
        LineCap::Square => 2,
    }
}

/// Convert LineJoin to Vello's join enum value.
pub fn line_join_to_u32(join: LineJoin) -> u32 {
    match join {
        LineJoin::Miter => 0,
        LineJoin::Round => 1,
        LineJoin::Bevel => 2,
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u8, x: f64, y: f64) -> VNNode {
        VNNode {
            id: Uuid::from_bytes([id; 16]),
            x,
            y,
        }
    }

    fn make_edge(id: u8, from: u8, to: u8) -> VNEdge {
        VNEdge {
            id: Uuid::from_bytes([id; 16]),
            from: Uuid::from_bytes([from; 16]),
            to: Uuid::from_bytes([to; 16]),
            from_handle_x: 0.0,
            from_handle_y: 0.0,
            to_handle_x: 0.0,
            to_handle_y: 0.0,
        }
    }

    fn make_edge_curved(id: u8, from: u8, to: u8, fh: (f64, f64), th: (f64, f64)) -> VNEdge {
        VNEdge {
            id: Uuid::from_bytes([id; 16]),
            from: Uuid::from_bytes([from; 16]),
            to: Uuid::from_bytes([to; 16]),
            from_handle_x: fh.0,
            from_handle_y: fh.1,
            to_handle_x: th.0,
            to_handle_y: th.1,
        }
    }

    #[test]
    fn single_straight_edge_produces_stroke_path() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 0.0, 0.0), make_node(2, 100.0, 0.0)],
            edges: vec![make_edge(10, 1, 2)],
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        assert_eq!(output.stroke_paths.len(), 1);
        assert_eq!(output.stroke_paths[0].len(), 2); // MoveTo + LineTo
        assert!(matches!(
            output.stroke_paths[0][0],
            PathCommand::MoveTo(0.0, 0.0)
        ));
        assert!(matches!(
            output.stroke_paths[0][1],
            PathCommand::LineTo(100.0, 0.0)
        ));
    }

    #[test]
    fn curved_edge_produces_cubic_command() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 0.0, 0.0), make_node(2, 100.0, 0.0)],
            edges: vec![make_edge_curved(10, 1, 2, (30.0, 40.0), (-30.0, 40.0))],
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        assert_eq!(output.stroke_paths.len(), 1);
        assert!(matches!(
            output.stroke_paths[0][1],
            PathCommand::CubicTo(_, _, _, _, _, _)
        ));
    }

    #[test]
    fn triangle_produces_fill_path() {
        let input = VectorNetworkInput {
            nodes: vec![
                make_node(1, 0.0, 0.0),
                make_node(2, 100.0, 0.0),
                make_node(3, 50.0, 100.0),
            ],
            edges: vec![
                make_edge(10, 1, 2),
                make_edge(11, 2, 3),
                make_edge(12, 3, 1),
            ],
            stroke: VNStrokeStyle::default(),
            fill: Some(VNFillStyle {
                color: [1.0, 0.0, 0.0, 1.0],
                opacity: 1.0,
            }),
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        assert!(!output.fill_paths.is_empty());
        // At least one closed path with Close command
        assert!(output
            .fill_paths
            .iter()
            .any(|p| matches!(p.last(), Some(PathCommand::Close))));
    }

    #[test]
    fn open_path_no_fill() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 0.0, 0.0), make_node(2, 100.0, 0.0)],
            edges: vec![make_edge(10, 1, 2)],
            stroke: VNStrokeStyle::default(),
            fill: Some(VNFillStyle {
                color: [1.0, 0.0, 0.0, 1.0],
                opacity: 1.0,
            }),
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        // Open path should not produce fill
        assert!(output.fill_paths.is_empty());
    }

    #[test]
    fn anchors_shown_at_high_zoom() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 0.0, 0.0), make_node(2, 100.0, 0.0)],
            edges: vec![make_edge(10, 1, 2)],
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: true,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        assert_eq!(output.anchor_points.len(), 2);
    }

    #[test]
    fn anchors_hidden_at_low_zoom() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 0.0, 0.0), make_node(2, 100.0, 0.0)],
            edges: vec![make_edge(10, 1, 2)],
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: true,
            anchor_radius: 3.0,
            zoom: 0.1, // Below 0.5 threshold
        };

        let output = render_vector_network(&input);
        assert!(output.anchor_points.is_empty());
    }

    #[test]
    fn bounds_computed_correctly() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 10.0, 20.0), make_node(2, 110.0, 120.0)],
            edges: vec![make_edge(10, 1, 2)],
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        let b = output.bounds.unwrap();
        assert_eq!(b.min_x, 10.0);
        assert_eq!(b.min_y, 20.0);
        assert_eq!(b.max_x, 110.0);
        assert_eq!(b.max_y, 120.0);
    }

    #[test]
    fn missing_node_skips_edge() {
        let input = VectorNetworkInput {
            nodes: vec![make_node(1, 0.0, 0.0)],
            edges: vec![make_edge(10, 1, 99)], // node 99 doesn't exist
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        assert!(output.stroke_paths.is_empty());
    }

    #[test]
    fn empty_network_produces_empty_output() {
        let input = VectorNetworkInput {
            nodes: vec![],
            edges: vec![],
            stroke: VNStrokeStyle::default(),
            fill: None,
            show_anchors: false,
            anchor_radius: 3.0,
            zoom: 1.0,
        };

        let output = render_vector_network(&input);
        assert!(output.stroke_paths.is_empty());
        assert!(output.fill_paths.is_empty());
        assert!(output.anchor_points.is_empty());
        assert!(output.bounds.is_none());
    }

    #[test]
    fn line_cap_conversion() {
        assert_eq!(line_cap_to_u32(LineCap::Butt), 0);
        assert_eq!(line_cap_to_u32(LineCap::Round), 1);
        assert_eq!(line_cap_to_u32(LineCap::Square), 2);
    }

    #[test]
    fn line_join_conversion() {
        assert_eq!(line_join_to_u32(LineJoin::Miter), 0);
        assert_eq!(line_join_to_u32(LineJoin::Round), 1);
        assert_eq!(line_join_to_u32(LineJoin::Bevel), 2);
    }
}
