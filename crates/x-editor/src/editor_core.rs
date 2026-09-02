
use x_core::*;
#[allow(unused_imports)]
use crate::*;

// Paste strategy for clipboard operations.
// `WithOffset`/`CenteredInView` are planned paste strategies kept as part
// of the model's public surface; the editor currently lowers everything
// through `InPlace` + an explicit move.
#[allow(dead_code)]
enum PasteStrategy {
    WithOffset((f64, f64)),
    InPlace,
    CenteredInView,
}

// ------------------------------------------------------------------- editor

/// Phase 2: selection + undoable document mutations + Phase 10 checkpoints.
pub struct Editor {
    pub root: Node,
    pub selection: Vec<String>,
    undo_stack: Vec<Vec<Command>>,
    redo_stack: Vec<Vec<Command>>,
    /// Group/Ungroup are structural; store whole-tree snapshots for them.
    snapshots: Vec<(usize, Node)>,
    /// Phase 10.2: named version checkpoints.
    pub checkpoints: Vec<(String, Node)>,
    /// Phase 2.7: internal clipboard (copied subtrees).
    clipboard: Vec<Node>,
    /// Phase P0: text editing mode
    text_edit_mode: Option<TextEditState>,
    /// Phase P0: corner drag state
    corner_drag_state: Option<CornerDragState>,
}

/// Phase P0: Text edit mode state
#[derive(Debug, Clone)]
pub struct TextEditState {
    pub node_id: String,
    pub selection_start: usize,
    pub selection_end: usize,
    pub cursor_visible: bool,
}

/// Phase P0: Corner drag state for radius adjustment
#[derive(Debug, Clone)]
pub struct CornerDragState {
    pub node_id: String,
    pub corner: Corner,
    pub initial_radius: f64,
    pub initial_mouse_pos: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner { TopLeft, TopRight, BottomRight, BottomLeft }

/// Give a subtree fresh ids via `rename`; only the subtree ROOT's new id is
/// recorded into `out` (that's what selection/paste callers care about).
fn remap_ids(node: &mut Node, rename: &mut impl FnMut(&str) -> String, out: &mut Vec<String>) {
    node.id = rename(&node.id);
    out.push(node.id.clone());
    fn walk(n: &mut Node, rename: &mut impl FnMut(&str) -> String) {
        for c in &mut n.children {
            c.id = rename(&c.id);
            walk(c, rename);
        }
    }
    walk(node, rename);
}

impl Editor {
    /// Rename a layer while preserving references used by instance overrides.
    /// The whole root replacement makes the id + reference rewrite atomic.
    pub fn rename_node(&mut self, id: &str, new_id: &str) -> bool {
        let new_id = new_id.trim();
        if new_id.is_empty() || id == new_id || find(&self.root, new_id).is_some() { return false; }
        if find(&self.root, id).is_none() { return false; }
        let before = Box::new(self.root.clone());
        let mut after = self.root.clone();
        if let Some(node) = find_mut(&mut after, id) { node.id = new_id.to_string(); }
        fn rewrite(n: &mut Node, old: &str, new: &str) {
            if let Some(v) = n.overrides.remove(old) { n.overrides.insert(new.to_string(), v); }
            for c in &mut n.children { rewrite(c, old, new); }
        }
        rewrite(&mut after, id, new_id);
        let root_id = self.root.id.clone();
        self.push_replace(&root_id, before, after);
        for selected in &mut self.selection { if selected == id { *selected = new_id.to_string(); } }
        true
    }
    pub fn new(root: Node) -> Self {
        Self { 
            root, 
            selection: vec![], 
            undo_stack: vec![], 
            redo_stack: vec![], 
            snapshots: vec![], 
            checkpoints: vec![], 
            clipboard: vec![],
            text_edit_mode: None,
            corner_drag_state: None,
        }
    }

    // -- selection ---------------------------------------------------------
    /// industry-standard click: plain click selects the top-level object under the
    /// cursor; `deep` (Ctrl+click) selects the exact nested node;
    /// `shift` toggles membership.
    pub fn click_select(&mut self, p: Point, shift: bool, deep: bool) {
        let hit = hit_test(&self.root, p);
        let target = match hit {
            Some(id) if !deep => top_level_ancestor(&self.root, &id).unwrap_or(id),
            Some(id) => id,
            None => { if !shift { self.selection.clear(); } return; }
        };
        if shift {
            if let Some(i) = self.selection.iter().position(|s| s == &target) { self.selection.remove(i); }
            else { self.selection.push(target); }
        } else {
            self.selection = vec![target];
        }
    }

    /// double-click to drill: drill one level deeper from the current
    /// selection toward the deep hit. Returns the newly selected id.
    pub fn drill_into(&mut self, p: Point) -> Option<String> {
        let deep = hit_test(&self.root, p)?;
        // path from root to deep node
        fn path_to(node: &Node, id: &str, path: &mut Vec<String>) -> bool {
            if node.id == id { return true; }
            for c in &node.children {
                path.push(c.id.clone());
                if path_to(c, id, path) { return true; }
                path.pop();
            }
            false
        }
        let mut path = vec![];
        path_to(&self.root, &deep, &mut path);
        // current selection position along the path -> next one deeper
        let cur = self.selection.first();
        let idx = cur.and_then(|c| path.iter().position(|p| p == c));
        let next = match idx {
            Some(i) if i + 1 < path.len() => path[i + 1].clone(),
            Some(_) => deep,
            None => path.first().cloned().unwrap_or(deep),
        };
        self.selection = vec![next.clone()];
        Some(next)
    }

    pub fn click(&mut self, p: Point, shift: bool) {
        match hit_test(&self.root, p) {
            Some(id) => {
                if shift {
                    if let Some(i) = self.selection.iter().position(|s| s == &id) { self.selection.remove(i); }
                    else { self.selection.push(id); }
                } else {
                    self.selection = vec![id];
                }
            }
            None => if !shift { self.selection.clear(); },
        }
    }
    pub fn marquee(&mut self, rect: Rect) { self.selection = hit_test_rect(&self.root, rect); }

    // -- undoable ops ------------------------------------------------------
    pub(crate) fn push_cmds(&mut self, cmds: Vec<Command>) { self.push(cmds); }

    fn push(&mut self, cmds: Vec<Command>) {
        let applied: Vec<Command> = cmds.into_iter().filter(|c| apply(&mut self.root, c)).collect();
        if !applied.is_empty() {
            self.undo_stack.push(applied);
            self.redo_stack.clear();
        }
    }

    pub fn move_selection(&mut self, dx: f64, dy: f64) {
        let cmds = self.selection.iter().map(|id| Command::Move { id: id.clone(), dx, dy }).collect();
        self.push(cmds);
    }
    pub fn resize(&mut self, id: &str, w: f64, h: f64) {
        if let Some(n) = find(&self.root, id) {
            let cmd = Command::Resize { id: id.into(), from: (n.w, n.h), to: (w, h) };
            self.push(vec![cmd]);
        }
    }
    pub fn rotate(&mut self, id: &str, angle: f64) {
        if let Some(n) = find(&self.root, id) {
            let cmd = Command::Rotate { id: id.into(), from: n.transform.rotation, to: angle };
            self.push(vec![cmd]);
        }
    }
    pub fn set_fill(&mut self, id: &str, paint: Paint) {
        if let Some(n) = find(&self.root, id) {
            if !n.visual_stacks_materialized {
                let cmd = Command::SetFill { id: id.into(), from: n.fill.clone(), to: paint };
                self.push(vec![cmd]);
            } else {
                let _ = self.mutate_visual_stack(id, move |node| {
                    if let Some(layer) = node.fill_layers.last_mut() { layer.paint = paint; }
                    else { node.fill_layers.push(PaintLayer::new(paint)); }
                });
            }
        }
    }

    /// Ordered visual-stack mutation. Every operation swaps the whole node,
    /// so add/remove/reorder/toggle remain one atomic undo step.
    pub fn mutate_visual_stack(&mut self, id: &str, f: impl FnOnce(&mut Node)) -> bool {
        let Some(n) = find(&self.root, id) else { return false };
        let before = Box::new(n.clone());
        let mut after = n.clone();
        after.materialize_visual_stacks();
        f(&mut after);
        after.dirty = true;
        self.push_replace(id, before, after);
        true
    }

    pub fn add_fill_layer(&mut self, id: &str, paint: Paint) -> bool {
        self.mutate_visual_stack(id, move |n| n.fill_layers.push(PaintLayer::new(paint)))
    }
    pub fn add_stroke_layer(&mut self, id: &str, stroke: Stroke) -> bool {
        self.mutate_visual_stack(id, move |n| n.stroke_layers.push(StrokeLayer::new(stroke)))
    }
    pub fn add_effect_layer(&mut self, id: &str, effect: Effect) -> bool {
        self.mutate_visual_stack(id, move |n| n.effect_layers.push(EffectLayer::new(effect)))
    }
    pub fn remove_fill_layer(&mut self, id: &str, index: usize) -> bool {
        self.mutate_visual_stack(id, move |n| { if index < n.fill_layers.len() { n.fill_layers.remove(index); } })
    }
    pub fn remove_stroke_layer(&mut self, id: &str, index: usize) -> bool {
        self.mutate_visual_stack(id, move |n| { if index < n.stroke_layers.len() { n.stroke_layers.remove(index); } })
    }
    pub fn remove_effect_layer(&mut self, id: &str, index: usize) -> bool {
        self.mutate_visual_stack(id, move |n| { if index < n.effect_layers.len() { n.effect_layers.remove(index); } })
    }
    pub fn move_fill_layer(&mut self, id: &str, from: usize, to: usize) -> bool {
        self.mutate_visual_stack(id, move |n| {
            if from < n.fill_layers.len() && to < n.fill_layers.len() { let v = n.fill_layers.remove(from); n.fill_layers.insert(to, v); }
        })
    }
    pub fn move_stroke_layer(&mut self, id: &str, from: usize, to: usize) -> bool {
        self.mutate_visual_stack(id, move |n| {
            if from < n.stroke_layers.len() && to < n.stroke_layers.len() { let v = n.stroke_layers.remove(from); n.stroke_layers.insert(to, v); }
        })
    }
    pub fn move_effect_layer(&mut self, id: &str, from: usize, to: usize) -> bool {
        self.mutate_visual_stack(id, move |n| {
            if from < n.effect_layers.len() && to < n.effect_layers.len() { let v = n.effect_layers.remove(from); n.effect_layers.insert(to, v); }
        })
    }
    pub fn set_text(&mut self, id: &str, text: &str) {
        if let Some(n) = find(&self.root, id) {
            if let NodeKind::Text { text: old } = &n.kind {
                let cmd = Command::SetText { id: id.into(), from: old.clone(), to: text.into() };
                self.push(vec![cmd]);
            }
        }
    }
    /// Set (or clear, with None) a frame's auto layout, re-solve child
    /// positions immediately, all as ONE undoable ReplaceNode command.
    pub fn set_auto_layout(&mut self, id: &str, layout: Option<x_core::AutoLayout>, vars: &Variables) -> bool {
        let Some(n) = find(&self.root, id) else { return false };
        if !matches!(n.kind, NodeKind::Frame { .. }) { return false; }
        let before = Box::new(n.clone());
        let mut after = n.clone();
        after.kind = NodeKind::Frame { layout: layout.clone() };
        if layout.is_some() { x_core::apply_auto_layout(&mut after, vars); }
        let cmd = Command::ReplaceNode { id: id.into(), before, after: Box::new(after) };
        self.push(vec![cmd]);
        true
    }

    /// Current auto layout of a frame, if any.
    pub fn auto_layout_of(&self, id: &str) -> Option<x_core::AutoLayout> {
        match find(&self.root, id)?.kind {
            NodeKind::Frame { ref layout } => layout.clone(),
            _ => None,
        }
    }

    /// Phase 2.3 (Scale tool): scale a node AND its whole subtree
    /// uniformly — sizes, child offsets, strokes, corner radii. One
    /// undoable ReplaceNode.
    pub fn scale_node(&mut self, id: &str, factor: f64) -> bool {
        if factor <= 0.0 { return false; }
        let Some(n) = find(&self.root, id) else { return false };
        let before = Box::new(n.clone());
        let mut after = n.clone();
        fn scale_subtree(n: &mut Node, f: f64, scale_own_pos: bool) {
            if scale_own_pos { n.transform.x *= f; n.transform.y *= f; }
            n.w *= f;
            n.h *= f;
            n.stroke.width *= f;
            if let NodeKind::Rect { radius } = &mut n.kind { *radius *= f; }
            if let Some(r) = &mut n.corner_radii { for v in r.iter_mut() { *v *= f; } }
            if let NodeKind::Vector { path } = &mut n.kind {
                for c in path.iter_mut() {
                    match c {
                        x_core::PathCmd::MoveTo(x, y) | x_core::PathCmd::LineTo(x, y) => { *x *= f; *y *= f; }
                        x_core::PathCmd::CurveTo(x1, y1, x2, y2, x, y) => { *x1 *= f; *y1 *= f; *x2 *= f; *y2 *= f; *x *= f; *y *= f; }
                        x_core::PathCmd::Close => {}
                    }
                }
            }
            for c in &mut n.children { scale_subtree(c, f, true); }
        }
        // the root of the scale keeps its own x/y (scales in place)
        scale_subtree(&mut after, factor, false);
        let cmd = Command::ReplaceNode { id: id.into(), before, after: Box::new(after) };
        self.push(vec![cmd]);
        true
    }

    /// Flip a layer without flattening it. Negative transform scale preserves
    /// editability and is serialized like any other transform.
    pub fn flip_node(&mut self, id: &str, horizontal: bool) -> bool {
        let Some(n) = find(&self.root, id) else { return false };
        let before = Box::new(n.clone());
        let mut after = n.clone();
        if horizontal { after.transform.scale_x *= -1.0; }
        else { after.transform.scale_y *= -1.0; }
        self.push_replace(id, before, after);
        true
    }

    /// Phase 8: set/clear a prototype link (click -> navigate to destination).
    pub fn set_prototype(&mut self, id: &str, action: Option<x_core::PrototypeAction>) {
        if let Some(n) = find(&self.root, id) {
            let cmd = Command::SetPrototype { id: id.into(), from: n.prototype.clone(), to: action };
            self.push(vec![cmd]);
        }
    }

    pub fn set_opacity(&mut self, id: &str, v: f32) {
        if let Some(n) = find(&self.root, id) {
            let cmd = Command::SetOpacity { id: id.into(), from: n.opacity, to: v };
            self.push(vec![cmd]);
        }
    }
    pub fn delete_selection(&mut self) {
        let mut cmds = vec![];
        for id in self.selection.clone() {
            if let Some(p) = find_parent_mut(&mut self.root, &id) {
                if let Some(i) = p.children.iter().position(|c| c.id == id) {
                    cmds.push(Command::Delete { parent_id: p.id.clone(), index: i, node: p.children[i].clone() });
                }
            }
        }
        // Delete back-to-front so stored indices stay valid.
        cmds.sort_by(|a, b| match (a, b) {
            (Command::Delete { index: ia, .. }, Command::Delete { index: ib, .. }) => ib.cmp(ia),
            _ => std::cmp::Ordering::Equal,
        });
        self.push(cmds);
        self.selection.clear();
    }
    /// Phase 2.8: z-order. bring_to_front / send_to_back.
    pub fn bring_to_front(&mut self, id: &str) {
        if let Some(p) = find_parent_mut(&mut self.root, id) {
            let last = p.children.len() - 1;
            if let Some(from) = p.children.iter().position(|c| c.id == id) {
                if from != last { self.push(vec![Command::Reorder { id: id.into(), from, to: last }]); }
            }
        }
    }
    pub fn send_to_back(&mut self, id: &str) {
        if let Some(p) = find_parent_mut(&mut self.root, id) {
            if let Some(from) = p.children.iter().position(|c| c.id == id) {
                if from != 0 { self.push(vec![Command::Reorder { id: id.into(), from, to: 0 }]); }
            }
        }
    }
    /// Move one step forward in z-order (swap with the next-higher
    /// sibling) — Figma's plain ⌘] "Bring Forward", distinct from the
    /// full jump-to-front above.
    pub fn bring_forward(&mut self, id: &str) {
        if let Some(p) = find_parent_mut(&mut self.root, id) {
            if let Some(from) = p.children.iter().position(|c| c.id == id) {
                let to = from + 1;
                if to < p.children.len() { self.push(vec![Command::Reorder { id: id.into(), from, to }]); }
            }
        }
    }
    /// Move one step backward in z-order (swap with the next-lower
    /// sibling) — Figma's plain ⌘[ "Send Backward".
    pub fn send_backward(&mut self, id: &str) {
        if let Some(p) = find_parent_mut(&mut self.root, id) {
            if let Some(from) = p.children.iter().position(|c| c.id == id) {
                if from > 0 { self.push(vec![Command::Reorder { id: id.into(), from, to: from - 1 }]); }
            }
        }
    }
    /// Phase 2.9: group the current selection (snapshot-undo).
    pub fn group_selection(&mut self, group_id: &str) {
        if self.selection.len() < 2 { return; }
        let snapshot = self.root.clone();
        // find common parent of first selected node; require all share it.
        let first = self.selection[0].clone();
        let parent_id = match find_parent_mut(&mut self.root, &first) { Some(p) => p.id.clone(), None => return };
        let indices: Vec<usize> = {
            let p = find(&self.root, &parent_id).unwrap();
            self.selection.iter().filter_map(|id| p.children.iter().position(|c| &c.id == id)).collect()
        };
        if indices.len() != self.selection.len() { return; } // not siblings
        let cmd = Command::Group { parent_id, indices, group_id: group_id.into() };
        if apply(&mut self.root, &cmd) {
            self.snapshots.push((self.undo_stack.len(), snapshot));
            self.undo_stack.push(vec![cmd]);
            self.redo_stack.clear();
            self.selection = vec![group_id.to_string()];
        }
    }

    /// Figma Ctrl+Shift+G: dissolve a group/frame, re-parenting children to
    /// the grandparent at the group's spot with positions preserved.
    pub fn ungroup(&mut self, id: &str) -> bool {
        let Some(g) = find(&self.root, id) else { return false };
        if !matches!(g.kind, NodeKind::Group | NodeKind::Frame { .. }) { return false; }
        let snapshot = self.root.clone();
        let (gx, gy) = (g.transform.x, g.transform.y);
        let Some(parent) = find_parent_mut(&mut self.root, id) else { return false };
        let Some(pos) = parent.children.iter().position(|c| c.id == id) else { return false };
        let mut group = parent.children.remove(pos);
        let mut ids = vec![];
        for mut child in group.children.drain(..) {
            child.transform.x += gx;
            child.transform.y += gy;
            ids.push(child.id.clone());
            parent.children.insert(pos, child);
        }
        self.snapshots.push((self.undo_stack.len(), snapshot));
        self.undo_stack.push(vec![Command::Group { parent_id: String::new(), indices: vec![], group_id: id.into() }]);
        self.redo_stack.clear();
        self.selection = ids;
        true
    }

    /// Figma Ctrl+A: select all top-level children of the page (or of the
    /// selected frame if one frame is selected).
    pub fn select_all(&mut self) {
        let scope = if self.selection.len() == 1 {
            find(&self.root, &self.selection[0])
                .filter(|n| matches!(n.kind, NodeKind::Frame { .. } | NodeKind::Group) && !n.children.is_empty())
        } else { None };
        let source = scope.unwrap_or(&self.root);
        self.selection = source.children.iter().filter(|c| c.visible && !c.locked).map(|c| c.id.clone()).collect();
    }

    /// Undoable constraint-pin change (Figma constraints panel).
    pub fn set_pin(&mut self, id: &str, h: x_core::HPin, v: x_core::VPin) {
        if let Some(n) = find(&self.root, id) {
            let before = Box::new(n.clone());
            let mut after = n.clone();
            after.pin = (h, v);
            self.push(vec![Command::ReplaceNode { id: id.into(), before, after: Box::new(after) }]);
        }
    }

    pub fn undo(&mut self) -> bool {
        let Some(cmds) = self.undo_stack.pop() else { return false };
        // Structural command? restore the snapshot taken before it.
        if matches!(cmds.first(), Some(Command::Group { .. })) {
            if let Some(pos) = self.snapshots.iter().rposition(|(depth, _)| *depth == self.undo_stack.len()) {
                let (_, snap) = self.snapshots.remove(pos);
                let redo_state = self.root.clone();
                self.root = snap;
                self.redo_stack.push(cmds);
                self.snapshots.push((usize::MAX, redo_state)); // stash for redo
                return true;
            }
        }
        for cmd in cmds.iter().rev() { apply(&mut self.root, &invert(cmd)); }
        self.redo_stack.push(cmds);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(cmds) = self.redo_stack.pop() else { return false };
        if matches!(cmds.first(), Some(Command::Group { .. })) {
            if let Some(pos) = self.snapshots.iter().rposition(|(d, _)| *d == usize::MAX) {
                let (_, state) = self.snapshots.remove(pos);
                self.snapshots.push((self.undo_stack.len(), self.root.clone()));
                self.root = state;
                self.undo_stack.push(cmds);
                return true;
            }
        }
        for cmd in &cmds { apply(&mut self.root, cmd); }
        self.undo_stack.push(cmds);
        true
    }

    /// Merge the last `n` undo entries into a single undo step. UI drags
    /// call move/resize once per mouse event (each pushing an entry);
    /// on mouse-up they merge the whole gesture so one Ctrl+Z reverts it.
    pub fn merge_last(&mut self, n: usize) {
        if n <= 1 || self.undo_stack.len() < n { return; }
        let at = self.undo_stack.len() - n;
        let mut merged = vec![];
        for group in self.undo_stack.drain(at..) { merged.extend(group); }
        self.undo_stack.push(merged);
    }
    /// Number of undo entries (lets the UI count a gesture's commands).
    pub fn undo_depth(&self) -> usize { self.undo_stack.len() }

    /// Approximate undo-history bytes (ReplaceNode snapshots dominate).
    pub fn history_bytes(&self) -> usize {
        fn cmds_bytes(cmds: &[Command]) -> usize {
            cmds.iter().map(|c| match c {
                Command::ReplaceNode { before, after, .. } =>
                    node_size(before) + node_size(after) + 64,
                Command::Delete { node, .. } | Command::Insert { node, .. } => node_size(node) + 64,
                _ => 96,
            }).sum()
        }
        fn node_size(n: &x_core::Node) -> usize {
            let mut b = std::mem::size_of::<x_core::Node>() + n.id.len();
            b += n.children.iter().map(node_size).sum::<usize>();
            b
        }
        self.undo_stack.iter().map(|g| cmds_bytes(g)).sum::<usize>()
            + self.redo_stack.iter().map(|g| cmds_bytes(g)).sum::<usize>()
    }

    /// Insert a new node under `parent_id` (undoable). Returns success.
    /// Undoable whole-node swap (mask flags, image placement, style binds…).
    pub fn replace_node(&mut self, id: &str, after: Node) -> bool {
        let Some(before) = find(&self.root, id) else { return false };
        let cmd = Command::ReplaceNode { id: id.into(), before: Box::new(before.clone()), after: Box::new(after) };
        self.push(vec![cmd]);
        true
    }

    pub fn insert_node(&mut self, parent_id: &str, node: Node) -> bool {
        let Some(p) = find(&self.root, parent_id) else { return false };
        let cmd = Command::Insert { parent_id: parent_id.into(), index: p.children.len(), node };
        let depth = self.undo_stack.len();
        self.push(vec![cmd]);
        self.undo_stack.len() > depth
    }

    /// Phase 5.2: turn the current selection into a Component definition.
    /// The selected nodes become children of a hidden master (placed at the
    /// document root), and the selection is replaced in-place by an Instance
    /// of it — same flow as Figma's "create component". One undo step
    /// (snapshot-based, like group).
    pub fn make_component(&mut self, name: &str) -> bool {
        if self.selection.is_empty() { return false; }
        let snapshot = self.root.clone();
        let first = self.selection[0].clone();
        let Some(parent_id) = find_parent_mut(&mut self.root, &first).map(|p| p.id.clone()) else { return false };
        // all selected must be siblings under the same parent
        let indices: Vec<usize> = {
            let p = find(&self.root, &parent_id).unwrap();
            self.selection.iter().filter_map(|id| p.children.iter().position(|c| &c.id == id)).collect()
        };
        if indices.len() != self.selection.len() { return false; }

        let p = find_mut(&mut self.root, &parent_id).unwrap();
        let mut sorted = indices.clone();
        sorted.sort_unstable();
        let mut members: Vec<Node> = vec![];
        for &i in sorted.iter().rev() { members.insert(0, p.children.remove(i)); }
        // collective bounds -> master size; members re-based to (0,0)
        let x0 = members.iter().map(|n| n.transform.x).fold(f64::INFINITY, f64::min);
        let y0 = members.iter().map(|n| n.transform.y).fold(f64::INFINITY, f64::min);
        let x1 = members.iter().map(|n| n.transform.x + n.w).fold(f64::NEG_INFINITY, f64::max);
        let y1 = members.iter().map(|n| n.transform.y + n.h).fold(f64::NEG_INFINITY, f64::max);
        let (w, h) = (x1 - x0, y1 - y0);
        let mut master = Node::component(&format!("comp-{name}"), name, w, h);
        for mut m in members { m.transform.x -= x0; m.transform.y -= y0; master.children.push(m); }
        master.visible = false; // masters live hidden at the root
        let instance_id = format!("{name}-1");
        let instance = Node::instance(&instance_id, name, x0, y0, w, h);
        // instance replaces the members at their original spot
        let p = find_mut(&mut self.root, &parent_id).unwrap();
        p.children.insert(sorted[0], instance);
        self.root.children.push(master);

        self.snapshots.push((self.undo_stack.len(), snapshot));
        self.undo_stack.push(vec![Command::Group { parent_id, indices: sorted, group_id: instance_id.clone() }]); // snapshot-undo reuses Group's path
        self.redo_stack.clear();
        self.selection = vec![instance_id];
        true
    }

    /// Phase 5.2: stamp a new Instance of `component` at (x, y). Undoable.
    /// Returns the new instance id.
    pub fn place_instance(&mut self, component: &str, x: f64, y: f64) -> Option<String> {
        // find the master to copy its size
        fn find_master<'a>(n: &'a Node, name: &str) -> Option<&'a Node> {
            if let NodeKind::Component { name: c } = &n.kind { if c == name { return Some(n); } }
            n.children.iter().find_map(|c| find_master(c, name))
        }
        let (w, h) = { let m = find_master(&self.root, component)?; (m.w, m.h) };
        // unique id
        let mut i = 1usize;
        let id = loop {
            let cand = format!("{component}-{i}");
            if find(&self.root, &cand).is_none() { break cand; }
            i += 1;
        };
        let node = Node::instance(&id, component, x, y, w, h);
        let root_id = self.root.id.clone();
        if self.insert_node(&root_id, node) { Some(id) } else { None }
    }

    /// Detach an instance into a plain group (undoable, Figma Ctrl+Alt+B).
    pub fn detach_selected_instance(&mut self, vars: &Variables) -> bool {
        let Some(id) = self.selection.first().cloned() else { return false };
        let Some(inst) = find(&self.root, &id) else { return false };
        let Some(detached) = x_components::detach_instance(&self.root, inst, vars) else { return false };
        let Some(parent) = find_parent_mut(&mut self.root, &id) else { return false };
        let Some(pos) = parent.children.iter().position(|c| c.id == id) else { return false };
        let parent_id = parent.id.clone();
        let old = parent.children[pos].clone();
        let new_id = detached.id.clone();
        self.push_cmds(vec![
            Command::Delete { parent_id: parent_id.clone(), index: pos, node: old },
            Command::Insert { parent_id, index: pos, node: detached },
        ]);
        self.selection = vec![new_id];
        true
    }

    /// Swap the selected instance's component / switch variant (undoable).
    pub fn swap_instance(&mut self, id: &str, to_component: &str) -> bool {
        let Some(n) = find(&self.root, id) else { return false };
        if !matches!(n.kind, NodeKind::Instance { .. }) { return false; }
        let before = Box::new(n.clone());
        let mut after = n.clone();
        if let NodeKind::Instance { component } = &mut after.kind { *component = to_component.to_string(); }
        self.push_cmds(vec![Command::ReplaceNode { id: id.into(), before, after: Box::new(after) }]);
        true
    }

    /// Names of all Component masters in the document.
    pub fn component_names(&self) -> Vec<String> {
        fn walk(n: &Node, out: &mut Vec<String>) {
            if let NodeKind::Component { name } = &n.kind { out.push(name.clone()); }
            for c in &n.children { walk(c, out); }
        }
        let mut v = vec![];
        walk(&self.root, &mut v);
        v
    }

    // -- Phase 2.7: copy / paste / duplicate --------------------------------
    /// Cut = copy + delete (undoable via delete_selection's command).
    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }

    /// How many nodes the internal clipboard holds (UI enablement).
    pub fn clipboard_len(&self) -> usize { self.clipboard.len() }

    /// Copy the current selection into the editor clipboard.
    pub fn copy(&mut self) {
        self.clipboard = self.selection.iter()
            .filter_map(|id| find(&self.root, id).cloned())
            .collect();
    }
    /// Paste clipboard contents into `parent_id` (undoable). Every pasted
    /// subtree gets fresh ids ("<old>-copy", "<old>-copy-2", ...) so ids stay
    /// unique — instance overrides keyed by INTERNAL component ids still
    /// work because those live inside component definitions, not the copy.
    pub fn paste(&mut self, parent_id: &str, offset: (f64, f64)) -> Vec<String> {
        // Collect every id already in the document once, then allocate
        // unique "-copy" ids against that set (no repeated tree scans).
        let mut taken = std::collections::HashSet::new();
        fn collect_ids(n: &Node, set: &mut std::collections::HashSet<String>) {
            set.insert(n.id.clone());
            for c in &n.children { collect_ids(c, set); }
        }
        collect_ids(&self.root, &mut taken);

        let mut new_root_ids = vec![];
        let mut cmds = vec![];
        for node in self.clipboard.clone() {
            let mut copy = node;
            let mut roots = vec![];
            remap_ids(&mut copy, &mut |old| {
                let mut candidate = format!("{old}-copy");
                let mut i = 1;
                while taken.contains(&candidate) {
                    i += 1;
                    candidate = format!("{old}-copy-{i}");
                }
                taken.insert(candidate.clone());
                candidate
            }, &mut roots);
            if let Some(rid) = roots.first() { new_root_ids.push(rid.clone()); }
            copy.transform.x += offset.0;
            copy.transform.y += offset.1;
            let index = find(&self.root, parent_id).map(|p| p.children.len()).unwrap_or(0);
            cmds.push(Command::Insert { parent_id: parent_id.into(), index, node: copy });
        }
        self.push(cmds);
        new_root_ids
    }
    /// Duplicate = copy + paste-in-place with a small offset, one call.
    pub fn duplicate_selection(&mut self, offset: (f64, f64)) -> Vec<String> {
        self.copy();
        let parent = self.selection.first()
            .and_then(|id| find_parent_mut(&mut self.root, id).map(|p| p.id.clone()))
            .unwrap_or_else(|| self.root.id.clone());
        let ids = self.paste(&parent, offset);
        self.selection = ids.clone();
        ids
    }

    // -- Phase P0: Paste variants -------------------------------------------
    /// Paste at exact same world coordinates (paste in place)
    pub fn paste_in_place(&mut self, parent_id: &str) -> Vec<String> {
        self.paste_with_strategy(parent_id, PasteStrategy::InPlace)
    }

    /// Paste over current selection, replacing it
    pub fn paste_over_selection(&mut self, parent_id: &str) -> Vec<String> {
        self.delete_selection();
        self.paste_in_place(parent_id)
    }

    fn paste_with_strategy(&mut self, parent_id: &str, strategy: PasteStrategy) -> Vec<String> {
        let mut taken = std::collections::HashSet::new();
        fn collect_ids(n: &Node, set: &mut std::collections::HashSet<String>) {
            set.insert(n.id.clone());
            for c in &n.children { collect_ids(c, set); }
        }
        collect_ids(&self.root, &mut taken);

        let mut new_root_ids = vec![];
        let mut cmds = vec![];
        for node in self.clipboard.clone() {
            let mut copy = node;
            let mut roots = vec![];
            remap_ids(&mut copy, &mut |old| {
                let mut candidate = format!("{old}-copy");
                let mut i = 1;
                while taken.contains(&candidate) {
                    i += 1;
                    candidate = format!("{old}-copy-{i}");
                }
                taken.insert(candidate.clone());
                candidate
            }, &mut roots);
            if let Some(rid) = roots.first() { new_root_ids.push(rid.clone()); }
            
            match strategy {
                PasteStrategy::WithOffset(offset) => {
                    copy.transform.x += offset.0;
                    copy.transform.y += offset.1;
                }
                PasteStrategy::InPlace => {
                    // Keep original coordinates
                }
                PasteStrategy::CenteredInView => {
                    // TODO: implement view-centering
                }
            }
            
            let index = find(&self.root, parent_id).map(|p| p.children.len()).unwrap_or(0);
            cmds.push(Command::Insert { parent_id: parent_id.into(), index, node: copy });
        }
        self.push(cmds);
        new_root_ids
    }

    // -- Phase P0: Text editing ---------------------------------------------
    /// Start editing text node at given id
    pub fn start_text_edit(&mut self, node_id: &str) -> bool {
        if let Some(node) = find(&self.root, node_id) {
            if matches!(node.kind, NodeKind::Text { .. }) {
                self.text_edit_mode = Some(TextEditState {
                    node_id: node_id.to_string(),
                    selection_start: 0,
                    selection_end: 0,
                    cursor_visible: true,
                });
                return true;
            }
        }
        false
    }

    /// Update text selection range
    pub fn update_text_selection(&mut self, start: usize, end: usize) {
        if let Some(state) = &mut self.text_edit_mode {
            state.selection_start = start;
            state.selection_end = end;
        }
    }

    /// Insert text at current cursor position
    pub fn insert_text(&mut self, text: &str) {
        if let Some(state) = &self.text_edit_mode {
            let node_id = state.node_id.clone();
            let start = state.selection_start;
            let end = state.selection_end;
            if let Some(node) = find_mut(&mut self.root, &node_id) {
                if let NodeKind::Text { text: current } = &mut node.kind {
                    current.replace_range(start..end, text);
                    // editing text invalidates rich-run char ranges
                    node.text_runs.clear();
                    // Update selection to end of inserted text
                    if let Some(state) = &mut self.text_edit_mode {
                        state.selection_start = start + text.len();
                        state.selection_end = state.selection_start;
                    }
                }
            }
        }
    }

    /// Exit text edit mode
    pub fn exit_text_edit(&mut self) {
        self.text_edit_mode = None;
    }

    // -- Phase P0: Corner radius handles ------------------------------------
    /// Start dragging a corner handle to adjust radius
    pub fn start_corner_drag(&mut self, node_id: &str, corner: Corner) -> bool {
        if let Some(node) = find(&self.root, node_id) {
            if let NodeKind::Rect { radius } = node.kind {
                self.corner_drag_state = Some(CornerDragState {
                    node_id: node_id.to_string(),
                    corner,
                    initial_radius: radius,
                    initial_mouse_pos: (0.0, 0.0), // Will be set by caller
                });
                return true;
            }
        }
        false
    }

    /// Update corner drag with mouse delta
    pub fn update_corner_drag(&mut self, mouse_delta_x: f64) {
        if let Some(state) = &mut self.corner_drag_state {
            if let Some(node) = find_mut(&mut self.root, &state.node_id) {
                if let NodeKind::Rect { radius } = &mut node.kind {
                    let delta = if state.corner == Corner::TopLeft || state.corner == Corner::BottomLeft {
                        mouse_delta_x
                    } else {
                        -mouse_delta_x
                    };
                    let max_radius = (node.w.min(node.h) / 2.0).max(0.0);
                    *radius = (state.initial_radius + delta).max(0.0).min(max_radius);
                }
            }
        }
    }

    /// End corner drag
    pub fn end_corner_drag(&mut self) {
        self.corner_drag_state = None;
    }

    // -- Phase 10.2: checkpoints -------------------------------------------
    pub fn checkpoint(&mut self, name: &str) { self.checkpoints.push((name.into(), self.root.clone())); }
    pub fn restore_checkpoint(&mut self, name: &str) -> bool {
        if let Some((_, snap)) = self.checkpoints.iter().find(|(n, _)| n == name) {
            self.root = snap.clone();
            self.undo_stack.clear();
            self.redo_stack.clear();
            true
        } else { false }
    }
}
