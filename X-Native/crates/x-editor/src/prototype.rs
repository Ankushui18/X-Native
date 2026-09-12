use crate::selection::hit_test;
#[allow(unused_imports)]
use crate::*;
use x_core::kurbo::Point;
use x_core::peniko::Color;
use x_core::*;

// ------------------------------------------------------- prototype playback

/// Phase 8: minimal prototype player. Frames with `prototype` actions
/// navigate on "click"; Back pops the navigation stack. Transition metadata
/// (duration) is surfaced so a renderer can animate.
pub struct Player<'a> {
    pub doc: &'a Node,
    pub current: String,
    /// navigation history (back target + depth); pub so hosts can render
    /// "back" affordances
    pub stack: Vec<String>,
}
impl<'a> Player<'a> {
    pub fn new(doc: &'a Node, start: &str) -> Self {
        Self {
            doc,
            current: start.into(),
            stack: vec![],
        }
    }
    /// Click at `point` inside the current top-level frame. The nearest
    /// ancestor of the hit node carrying an `OnClick` interaction fires it —
    /// rich `Node::interactions` first, legacy `prototype` links via the
    /// same `effective_interactions` path the runtime uses everywhere else.
    /// `Navigate`/`ScrollTo` switch frames, `Back` pops the history.
    /// Returns transition ms when the click navigated.
    pub fn click(&mut self, point: Point) -> Option<u32> {
        let frame = find(self.doc, &self.current)?;
        let hit_id = hit_test(frame, point)?;
        let (_owner, ix) = find_interaction_for(frame, &hit_id, Trigger::OnClick)?;
        let dest = match &ix.action {
            x_core::Action::Navigate { destination } => destination.clone(),
            x_core::Action::ScrollTo { destination } => destination.clone(),
            x_core::Action::Back => {
                return if self.back() {
                    Some(ix.transition_ms)
                } else {
                    None
                };
            }
            _ => return None,
        };
        if find(self.doc, &dest).is_some() {
            self.stack.push(self.current.clone());
            self.current = dest;
            Some(ix.transition_ms)
        } else {
            None
        }
    }
    pub fn back(&mut self) -> bool {
        if let Some(prev) = self.stack.pop() {
            self.current = prev;
            true
        } else {
            false
        }
    }
}

// ------------------------------------------------------------ smart animate

/// Phase 8.3: smart animate. Given two frames, nodes with MATCHING IDS are
/// interpolated (position, size, rotation, opacity, solid fill color) at
/// progress `t` in [0,1]; the result is a renderable in-between frame.
/// Nodes only present in `to` fade in; nodes only in `from` fade out —
/// the same matching rule comparable tools use.
pub fn smart_animate(from: &Node, to: &Node, t: f64) -> Node {
    let t = t.clamp(0.0, 1.0);
    let mut frame = to.clone();
    frame.id = format!("{}~{}@{t:.3}", from.id, to.id);

    fn collect<'n>(n: &'n Node, map: &mut std::collections::HashMap<String, &'n Node>) {
        map.insert(n.id.clone(), n);
        for c in &n.children {
            collect(c, map);
        }
    }
    let mut from_map = std::collections::HashMap::new();
    for c in &from.children {
        collect(c, &mut from_map);
    }

    fn lerp(a: f64, b: f64, t: f64) -> f64 {
        a + (b - a) * t
    }
    fn lerp_color(a: Color, b: Color, t: f64) -> Color {
        // components are linear f32 rgba; interpolate per channel
        let lerp = |x: f32, y: f32| (x as f64 + (y as f64 - x as f64) * t) as f32;
        Color::new([
            lerp(a.components[0], b.components[0]),
            lerp(a.components[1], b.components[1]),
            lerp(a.components[2], b.components[2]),
            lerp(a.components[3], b.components[3]),
        ])
    }

    fn blend_tree(node: &mut Node, from_map: &std::collections::HashMap<String, &Node>, t: f64) {
        if let Some(src) = from_map.get(&node.id) {
            node.transform.x = lerp(src.transform.x, node.transform.x, t);
            node.transform.y = lerp(src.transform.y, node.transform.y, t);
            node.transform.rotation = lerp(src.transform.rotation, node.transform.rotation, t);
            node.w = lerp(src.w, node.w, t);
            node.h = lerp(src.h, node.h, t);
            node.opacity = lerp(src.opacity as f64, node.opacity as f64, t) as f32;
            if let (Paint::Solid(a), Paint::Solid(b)) = (&src.fill, &node.fill.clone()) {
                node.fill = Paint::Solid(lerp_color(*a, *b, t));
            }
        } else {
            // new in `to`: fade in
            node.opacity = (node.opacity as f64 * t) as f32;
        }
        for c in &mut node.children {
            blend_tree(c, from_map, t);
        }
    }
    for c in &mut frame.children {
        blend_tree(c, &from_map, t);
    }

    // nodes that existed in `from` but not in `to`: fade OUT (append ghosts)
    let mut to_ids = std::collections::HashSet::new();
    fn ids(n: &Node, set: &mut std::collections::HashSet<String>) {
        set.insert(n.id.clone());
        for c in &n.children {
            ids(c, set);
        }
    }
    for c in &to.children {
        ids(c, &mut to_ids);
    }
    for c in &from.children {
        if !to_ids.contains(&c.id) {
            let mut ghost = c.clone();
            ghost.opacity = (ghost.opacity as f64 * (1.0 - t)) as f32;
            frame.children.push(ghost);
        }
    }
    frame
}
