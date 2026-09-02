//! Editor integration for vector booleans. The geometry core lives in
//! `x_core::booleans` (shared with the format importers — Sketch boolean
//! shapeGroups flatten through it); this module keeps the historical
//! `x_editor::booleans` path alive and adds the undoable editor command.

pub use x_core::booleans::{Backend, BoolOp, BooleanResult, PositionedPath, boolean, boolean_paths, boolean_with, node_to_path};

use crate::{Command, Editor, find};
use x_core::Node;

impl Editor {
    /// Boolean the two selected nodes -> one new Vector node (undoable).
    /// Keeps the FIRST node's fill; deletes both inputs.
    pub fn boolean_selected(&mut self, op: BoolOp) -> Option<String> {
        if self.selection.len() != 2 { return None; }
        let (ida, idb) = (self.selection[0].clone(), self.selection[1].clone());
        let na = find(&self.root, &ida)?.clone();
        let nb = find(&self.root, &idb)?.clone();
        let pa = node_to_path(&na)?;
        let pb = node_to_path(&nb)?;
        let res = boolean(op,
            &PositionedPath { cmds: pa, offset: (na.transform.x, na.transform.y) },
            &PositionedPath { cmds: pb, offset: (nb.transform.x, nb.transform.y) },
        );
        let (path, origin, size) = (res.cmds, res.origin, res.size);
        if path.is_empty() { return None; }
        let new_id = format!("bool-{}", self.undo_depth());
        let mut v = Node::vector(&new_id, 0.0, 0.0, size.0, size.1, path);
        v.transform.x = origin.0;
        v.transform.y = origin.1;
        v.fill = na.fill.clone();
        // undoable: delete both inputs, insert result (snapshot style)
        let parent_id = self.root.id.clone();
        let idx_a = self.root.children.iter().position(|c| c.id == ida)?;
        let node_a = self.root.children[idx_a].clone();
        let idx_b0 = self.root.children.iter().position(|c| c.id == idb)?;
        let node_b = self.root.children[idx_b0].clone();
        // delete higher index first
        let (first, second) = if idx_a > idx_b0 { (idx_a, idx_b0) } else { (idx_b0, idx_a) };
        let (nfirst, nsecond) = if idx_a > idx_b0 { (node_a.clone(), node_b.clone()) } else { (node_b.clone(), node_a.clone()) };
        self.push_cmds(vec![
            Command::Delete { parent_id: parent_id.clone(), index: first, node: nfirst },
            Command::Delete { parent_id: parent_id.clone(), index: second, node: nsecond },
            Command::Insert { parent_id, index: second, node: v },
        ]);
        self.selection = vec![new_id.clone()];
        Some(new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x_core::{Color, NodeKind, PathCmd};

    #[test]
    fn booleans_2_0_default_backend_preserves_curves_end_to_end() {
        // ellipse ∪ rect through the DEFAULT backend: the result path must
        // still contain CurveTo commands (the old polygon default emitted
        // only LineTo) — this is the review's "Bezier output" requirement
        // verified at the boolean_selected level the app actually calls.
        let page = x_core::Node::frame("page", 400.0, 300.0)
            .child(x_core::Node::ellipse("e", 40.0, 40.0, 120.0, 120.0, Color::from_rgb8(255, 0, 0)))
            .child(x_core::Node::rect("r", 100.0, 40.0, 120.0, 120.0, Color::from_rgb8(0, 0, 255)));
        let mut ed = crate::Editor::new(page);
        ed.selection = vec!["e".into(), "r".into()];
        let id = ed.boolean_selected(BoolOp::Union).expect("union");
        let n = crate::find(&ed.root, &id).unwrap();
        let x_core::NodeKind::Vector { path } = &n.kind else { panic!("not a vector") };
        let curves = path.iter().filter(|c| matches!(c, PathCmd::CurveTo(..))).count();
        assert!(curves >= 2, "union of ellipse+rect keeps {curves} real curve segments");
        // and a SECOND boolean on the result still preserves curves
        // (the anti-degradation property, applied through the real API)
        let idx = ed.root.children.iter().position(|c| c.id == id).unwrap();
        let mut bite = x_core::Node::rect("bite", 60.0, 90.0, 40.0, 40.0, Color::BLACK);
        bite.transform.x = 60.0; bite.transform.y = 90.0;
        ed.root.children.insert(idx, bite);
        ed.selection = vec![id.clone(), "bite".into()];
        let id2 = ed.boolean_selected(BoolOp::Subtract).expect("second op");
        let n2 = crate::find(&ed.root, &id2).unwrap();
        let x_core::NodeKind::Vector { path: p2 } = &n2.kind else { panic!() };
        let curves2 = p2.iter().filter(|c| matches!(c, PathCmd::CurveTo(..))).count();
        assert!(curves2 >= 2, "second-generation boolean still has {curves2} curves");
    }

    #[test]
    fn editor_boolean_replaces_selection_undoably() {
        let mut e = Editor::new(
            Node::frame("page", 400.0, 300.0)
                .child(Node::rect("a", 10.0, 10.0, 100.0, 100.0, Color::from_rgb8(255, 0, 0)))
                .child(Node::ellipse("b", 60.0, 10.0, 100.0, 100.0, Color::from_rgb8(0, 255, 0))),
        );
        e.selection = vec!["a".into(), "b".into()];
        let id = e.boolean_selected(BoolOp::Union).expect("union");
        assert!(find(&e.root, "a").is_none() && find(&e.root, "b").is_none());
        let v = find(&e.root, &id).unwrap();
        assert!(matches!(&v.kind, NodeKind::Vector { path } if !path.is_empty()));
        assert!(matches!(&v.fill, x_core::Paint::Solid(c) if c.to_rgba8().r == 255), "keeps A's fill");
        // one undo restores both inputs and removes the result
        e.undo();
        assert!(find(&e.root, "a").is_some() && find(&e.root, "b").is_some());
        assert!(find(&e.root, &id).is_none());
    }
}
