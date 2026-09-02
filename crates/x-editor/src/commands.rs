

use x_core::*;
#[allow(unused_imports)]
use crate::*;

// ----------------------------------------------------------------- commands

/// Phase 2.4: every mutation is a Command; undo/redo replays inverses.
/// Command-log architecture (not snapshots) — the roadmap's decision point,
/// resolved: logs stay cheap at 100K-node documents.
#[derive(Debug, Clone)]
pub enum Command {
    Move { id: String, dx: f64, dy: f64 },
    Resize { id: String, from: (f64, f64), to: (f64, f64) },
    Rotate { id: String, from: f64, to: f64 },
    SetFill { id: String, from: Paint, to: Paint },
    SetText { id: String, from: String, to: String },
    SetOpacity { id: String, from: f32, to: f32 },
    /// Generic whole-node swap: used where a mutation has wide side effects
    /// (e.g. auto-layout repositioning every child). Clean inverse by swap.
    ReplaceNode { id: String, before: Box<Node>, after: Box<Node> },
    SetPrototype { id: String, from: Option<x_core::PrototypeAction>, to: Option<x_core::PrototypeAction> },
    Reorder { id: String, from: usize, to: usize },
    Delete { parent_id: String, index: usize, node: Node },
    Insert { parent_id: String, index: usize, node: Node },
    Group { parent_id: String, indices: Vec<usize>, group_id: String },
}

pub(crate) fn apply(root: &mut Node, cmd: &Command) -> bool {
    match cmd {
        Command::Move { id, dx, dy } => {
            if let Some(n) = find_mut(root, id) { n.transform.x += dx; n.transform.y += dy; n.dirty = true; true } else { false }
        }
        Command::Resize { id, to, .. } => {
            if let Some(n) = find_mut(root, id) { n.w = to.0.max(1.0); n.h = to.1.max(1.0); n.dirty = true; true } else { false }
        }
        Command::Rotate { id, to, .. } => {
            if let Some(n) = find_mut(root, id) { n.transform.rotation = *to; n.dirty = true; true } else { false }
        }
        Command::SetFill { id, to, .. } => {
            if let Some(n) = find_mut(root, id) { n.fill = to.clone(); n.dirty = true; true } else { false }
        }
        Command::SetText { id, to, .. } => {
            if let Some(n) = find_mut(root, id) {
                if let NodeKind::Text { text } = &mut n.kind { *text = to.clone(); n.text_runs.clear(); n.dirty = true; return true; }
            }
            false
        }
        Command::SetOpacity { id, to, .. } => {
            if let Some(n) = find_mut(root, id) { n.opacity = to.clamp(0.0, 1.0); n.dirty = true; true } else { false }
        }
        Command::ReplaceNode { id, after, .. } => {
            if let Some(n) = find_mut(root, id) { *n = (**after).clone(); n.dirty = true; true } else { false }
        }
        Command::SetPrototype { id, to, .. } => {
            if let Some(n) = find_mut(root, id) { n.prototype = to.clone(); n.dirty = true; true } else { false }
        }
        Command::Reorder { id, to, .. } => {
            if let Some(p) = find_parent_mut(root, id) {
                if let Some(from) = p.children.iter().position(|c| &c.id == id) {
                    let to = (*to).min(p.children.len() - 1);
                    let n = p.children.remove(from);
                    p.children.insert(to, n);
                    return true;
                }
            }
            false
        }
        Command::Delete { parent_id, index, .. } => {
            if let Some(p) = find_mut(root, parent_id) {
                if *index < p.children.len() { p.children.remove(*index); return true; }
            }
            false
        }
        Command::Insert { parent_id, index, node } => {
            if let Some(p) = find_mut(root, parent_id) {
                p.children.insert((*index).min(p.children.len()), node.clone());
                return true;
            }
            false
        }
        Command::Group { parent_id, indices, group_id } => {
            if let Some(p) = find_mut(root, parent_id) {
                let mut taken: Vec<Node> = vec![];
                let mut sorted = indices.clone();
                sorted.sort_unstable();
                for &i in sorted.iter().rev() {
                    if i < p.children.len() { taken.insert(0, p.children.remove(i)); }
                }
                if taken.is_empty() { return false; }
                // Group frame wraps the members' collective AABB.
                let x0 = taken.iter().map(|n| n.transform.x).fold(f64::INFINITY, f64::min);
                let y0 = taken.iter().map(|n| n.transform.y).fold(f64::INFINITY, f64::min);
                let x1 = taken.iter().map(|n| n.transform.x + n.w).fold(f64::NEG_INFINITY, f64::max);
                let y1 = taken.iter().map(|n| n.transform.y + n.h).fold(f64::NEG_INFINITY, f64::max);
                let mut g = Node::group(group_id, x1 - x0, y1 - y0);
                g.transform.x = x0; g.transform.y = y0;
                for mut m in taken { m.transform.x -= x0; m.transform.y -= y0; g.children.push(m); }
                p.children.insert(sorted[0], g);
                return true;
            }
            false
        }
    }
}

pub(crate) fn invert(cmd: &Command) -> Command {
    match cmd {
        Command::Move { id, dx, dy } => Command::Move { id: id.clone(), dx: -dx, dy: -dy },
        Command::Resize { id, from, to } => Command::Resize { id: id.clone(), from: *to, to: *from },
        Command::Rotate { id, from, to } => Command::Rotate { id: id.clone(), from: *to, to: *from },
        Command::SetFill { id, from, to } => Command::SetFill { id: id.clone(), from: to.clone(), to: from.clone() },
        Command::SetText { id, from, to } => Command::SetText { id: id.clone(), from: to.clone(), to: from.clone() },
        Command::SetOpacity { id, from, to } => Command::SetOpacity { id: id.clone(), from: *to, to: *from },
        Command::ReplaceNode { id, before, after } => Command::ReplaceNode { id: id.clone(), before: after.clone(), after: before.clone() },
        Command::SetPrototype { id, from, to } => Command::SetPrototype { id: id.clone(), from: to.clone(), to: from.clone() },
        Command::Reorder { id, from, to } => Command::Reorder { id: id.clone(), from: *to, to: *from },
        Command::Delete { parent_id, index, node } => Command::Insert { parent_id: parent_id.clone(), index: *index, node: node.clone() },
        Command::Insert { parent_id, index, node } => Command::Delete { parent_id: parent_id.clone(), index: *index, node: node.clone() },
        // Group inversion = restore snapshot via history (handled by Editor
        // keeping a pre-group Delete/Insert pair); simplest correct inverse:
        Command::Group { .. } => unreachable!("Group is inverted via Ungroup in Editor"),
    }
}

