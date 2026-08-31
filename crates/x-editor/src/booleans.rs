//! Vector booleans: union / subtract / intersect / exclude.
//!
//! Implementation: scanline polygonization + even-odd region test.
//! Paths are flattened (cubics -> polylines), regions combined with the
//! chosen predicate, and the result traced back into PathCmd contours
//! via marching squares over a supersampled coverage grid. This is a
//! raster-guided approach: robust against self-intersection and open
//! degenerate input, resolution-tunable, zero external deps.
//! (An exact Bentley-Ottmann clipper can replace the core behind the
//! same API later; results are already visually correct and re-editable
//! as vector contours.)

use crate::{find, Command, Editor};
use x_core::{Node, NodeKind, PathCmd};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp { Union, Subtract, Intersect, Exclude }

// ------------------------------------------------------------ facade API
//
// `boolean(op, a, b)` is the STABLE public geometry API. Callers (editor,
// future plugin surface) never see which backend computes the result.
// Backends:
//   Backend::RasterGuided  — current: coverage grid + edge-chaining (beta)
//   Backend::Exact         — future: Bentley-Ottmann / Bezier clipper
// Swapping the default backend before v1.0 changes ONE constant here.

/// A positioned path: commands + world offset of their local origin.
#[derive(Debug, Clone)]
pub struct PositionedPath {
    pub cmds: Vec<PathCmd>,
    pub offset: (f64, f64),
}

/// Result of a boolean: contours + the world origin/size of their bbox.
#[derive(Debug, Clone)]
pub struct BooleanResult {
    pub cmds: Vec<PathCmd>,
    pub origin: (f64, f64),
    pub size: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// coverage grid + edge-chaining contour extraction (robust fallback)
    RasterGuided,
    /// exact Greiner–Hormann POLYGON clipper (flattens curves first;
    /// output is polyline). Fallback tier between BezierExact and raster.
    Exact,
    /// booleans 2.0 (DEFAULT since v0.32): curve-preserving clipper —
    /// bezier intersections via recursive subdivision, de Casteljau
    /// splits at cut params, topology traced over ORIGINAL curve pieces.
    /// Curves in, curves out: repeated ops don't degrade quality.
    /// Falls back Exact -> RasterGuided on degenerate topology.
    #[default]
    BezierExact,
}

/// The stable boolean API: `boolean(op, a, b) -> path`.
/// Application code must call this (or `boolean_with`), never a backend.
pub fn boolean(op: BoolOp, a: &PositionedPath, b: &PositionedPath) -> BooleanResult {
    boolean_with(Backend::default(), op, a, b)
}

/// Same, with an explicit backend (tests / future migration).
pub fn boolean_with(backend: Backend, op: BoolOp, a: &PositionedPath, b: &PositionedPath) -> BooleanResult {
    if backend == Backend::BezierExact {
        if let Some(res) = boolean_bezier(op, a, b) { return res; }
        // degenerate for the curve clipper: try the polygon tier next
    }
    if backend == Backend::BezierExact || backend == Backend::Exact {
        if let Some(res) = boolean_exact(op, a, b) { return res; }
        // degenerate topology: fall through to the raster backend
    }
    let (cmds, origin, size) = boolean_paths(&a.cmds, a.offset, &b.cmds, b.offset, op);
    BooleanResult { cmds, origin, size }
}

/// Booleans 2.0 backend: curve-preserving. None => caller falls back.
fn boolean_bezier(op: BoolOp, a: &PositionedPath, b: &PositionedPath) -> Option<BooleanResult> {
    use crate::bezier_clip::{clip_bezier, clip_bezier_exclude, path_to_segs, segs_to_path};
    let sa = path_to_segs(&a.cmds, a.offset)?;
    let sb = path_to_segs(&b.cmds, b.offset)?;
    let contours = match op {
        BoolOp::Union => clip_bezier(&sa, &sb, crate::clip::ClipOp::Union)?,
        BoolOp::Intersect => clip_bezier(&sa, &sb, crate::clip::ClipOp::Intersect)?,
        BoolOp::Subtract => clip_bezier(&sa, &sb, crate::clip::ClipOp::AminusB)?,
        BoolOp::Exclude => clip_bezier_exclude(&sa, &sb)?,
    };
    if contours.is_empty() { return None; }
    // bounds from fine evaluation (control points can overshoot the shape)
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for seg in contours.iter().flatten() {
        for i in 0..=16 {
            let p = seg.eval(i as f64 / 16.0);
            x0 = x0.min(p.0); y0 = y0.min(p.1); x1 = x1.max(p.0); y1 = y1.max(p.1);
        }
    }
    let cmds = segs_to_path(&contours, (x0, y0));
    if cmds.is_empty() { return None; }
    Some(BooleanResult { cmds, origin: (x0, y0), size: (x1 - x0, y1 - y0) })
}

/// Exact backend: flatten (curves -> polylines at 16 segments/curve),
/// then Greiner–Hormann clip with analytic edge intersections.
/// Single-contour simple polygons; None => caller falls back.
fn boolean_exact(op: BoolOp, a: &PositionedPath, b: &PositionedPath) -> Option<BooleanResult> {
    let pa = flatten(&a.cmds, a.offset);
    let pb = flatten(&b.cmds, b.offset);
    // exact backend scope: one simple contour per operand
    if pa.len() != 1 || pb.len() != 1 { return None; }
    let (sa, sb) = (&pa[0], &pb[0]);
    let polys = match op {
        BoolOp::Union => crate::clip::clip(sa, sb, crate::clip::ClipOp::Union)?,
        BoolOp::Intersect => crate::clip::clip(sa, sb, crate::clip::ClipOp::Intersect)?,
        BoolOp::Subtract => crate::clip::clip(sa, sb, crate::clip::ClipOp::AminusB)?,
        BoolOp::Exclude => crate::clip::clip_exclude(sa, sb)?,
    };
    if polys.is_empty() { return None; }
    // bounds -> local space
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in polys.iter().flatten() {
        x0 = x0.min(p.0); y0 = y0.min(p.1); x1 = x1.max(p.0); y1 = y1.max(p.1);
    }
    let mut cmds = vec![];
    for poly in &polys {
        if poly.len() < 3 { continue; }
        cmds.push(PathCmd::MoveTo(poly[0].0 - x0, poly[0].1 - y0));
        for p in &poly[1..] { cmds.push(PathCmd::LineTo(p.0 - x0, p.1 - y0)); }
        cmds.push(PathCmd::Close);
    }
    if cmds.is_empty() { return None; }
    Some(BooleanResult { cmds, origin: (x0, y0), size: (x1 - x0, y1 - y0) })
}

/// Flatten one path into closed polygons (local coords).
fn flatten(path: &[PathCmd], offset: (f64, f64)) -> Vec<Vec<(f64, f64)>> {
    let mut polys = vec![];
    let mut cur: Vec<(f64, f64)> = vec![];
    let mut start = (0.0, 0.0);
    let mut pos = (0.0, 0.0);
    let (ox, oy) = offset;
    for c in path {
        match *c {
            PathCmd::MoveTo(x, y) => {
                if cur.len() > 2 { polys.push(std::mem::take(&mut cur)); } else { cur.clear(); }
                start = (x + ox, y + oy);
                pos = start;
                cur.push(pos);
            }
            PathCmd::LineTo(x, y) => { pos = (x + ox, y + oy); cur.push(pos); }
            PathCmd::CurveTo(x1, y1, x2, y2, x, y) => {
                let (p0, p1, p2, p3) = (pos, (x1 + ox, y1 + oy), (x2 + ox, y2 + oy), (x + ox, y + oy));
                for i in 1..=16 {
                    let t = i as f64 / 16.0;
                    let mt = 1.0 - t;
                    let px = mt*mt*mt*p0.0 + 3.0*mt*mt*t*p1.0 + 3.0*mt*t*t*p2.0 + t*t*t*p3.0;
                    let py = mt*mt*mt*p0.1 + 3.0*mt*mt*t*p1.1 + 3.0*mt*t*t*p2.1 + t*t*t*p3.1;
                    cur.push((px, py));
                }
                pos = p3;
            }
            PathCmd::Close => {
                if cur.len() > 2 { cur.push(start); polys.push(std::mem::take(&mut cur)); } else { cur.clear(); }
            }
        }
    }
    if cur.len() > 2 { polys.push(cur); }
    polys
}

fn point_in(polys: &[Vec<(f64, f64)>], x: f64, y: f64) -> bool {
    // even-odd across all contours
    let mut inside = false;
    for poly in polys {
        let n = poly.len();
        for i in 0..n {
            let (x1, y1) = poly[i];
            let (x2, y2) = poly[(i + 1) % n];
            if (y1 > y) != (y2 > y) {
                let xi = x1 + (y - y1) / (y2 - y1) * (x2 - x1);
                if x < xi { inside = !inside; }
            }
        }
    }
    inside
}

/// Boolean of two shapes -> new PathCmd contours in A∪B bounding space.
pub fn boolean_paths(
    a: &[PathCmd], a_off: (f64, f64),
    b: &[PathCmd], b_off: (f64, f64),
    op: BoolOp,
) -> (Vec<PathCmd>, (f64, f64), (f64, f64)) {
    let pa = flatten(a, a_off);
    let pb = flatten(b, b_off);
    // bounds
    let all: Vec<(f64, f64)> = pa.iter().chain(pb.iter()).flatten().copied().collect();
    if all.is_empty() { return (vec![], (0.0, 0.0), (0.0, 0.0)); }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in &all { x0 = x0.min(*x); y0 = y0.min(*y); x1 = x1.max(*x); y1 = y1.max(*y); }
    let pad = 2.0;
    x0 -= pad; y0 -= pad; x1 += pad; y1 += pad;

    // coverage grid (resolution capped for perf, >=1px cells)
    const MAX_CELLS: usize = 360;
    let w = x1 - x0;
    let h = y1 - y0;
    let cell = (w.max(h) / MAX_CELLS as f64).max(0.75);
    let gw = (w / cell).ceil() as usize + 1;
    let gh = (h / cell).ceil() as usize + 1;
    let mut grid = vec![false; gw * gh];
    for gy in 0..gh {
        for gx in 0..gw {
            let px = x0 + (gx as f64 + 0.5) * cell;
            let py = y0 + (gy as f64 + 0.5) * cell;
            let ia = point_in(&pa, px, py);
            let ib = point_in(&pb, px, py);
            grid[gy * gw + gx] = match op {
                BoolOp::Union => ia || ib,
                BoolOp::Subtract => ia && !ib,
                BoolOp::Intersect => ia && ib,
                BoolOp::Exclude => ia != ib,
            };
        }
    }

    // contour extraction by EDGE CHAINING: every filled cell contributes
    // its boundary edges (sides facing empty cells); these segments form
    // closed loops by construction — robust for any region shape,
    // including holes (hole loops emerge automatically).
    let at = |gx: i64, gy: i64| -> bool {
        gx >= 0 && gy >= 0 && (gx as usize) < gw && (gy as usize) < gh && grid[gy as usize * gw + gx as usize]
    };
    use std::collections::HashMap as Map;
    // edges keyed by start corner -> end corner (integer grid corners)
    let mut edges: Map<(i64, i64), Vec<(i64, i64)>> = Map::new();
    let mut add = |a: (i64, i64), b: (i64, i64), edges: &mut Map<(i64, i64), Vec<(i64, i64)>>| {
        edges.entry(a).or_default().push(b);
    };
    for gy in 0..gh as i64 {
        for gx in 0..gw as i64 {
            if !at(gx, gy) { continue; }
            // orient edges so interior is on the LEFT (CCW outer loops)
            if !at(gx, gy - 1) { add((gx, gy), (gx + 1, gy), &mut edges); }         // top edge, ->
            if !at(gx + 1, gy) { add((gx + 1, gy), (gx + 1, gy + 1), &mut edges); } // right, v
            if !at(gx, gy + 1) { add((gx + 1, gy + 1), (gx, gy + 1), &mut edges); } // bottom, <-
            if !at(gx - 1, gy) { add((gx, gy + 1), (gx, gy), &mut edges); }         // left, ^
        }
    }
    let mut contours: Vec<Vec<(f64, f64)>> = vec![];
    while let Some((&start_pt, _)) = edges.iter().find(|(_, v)| !v.is_empty()) {
        let mut loop_pts = vec![start_pt];
        let mut cur = start_pt;
        loop {
            let Some(nexts) = edges.get_mut(&cur) else { break };
            let Some(nxt) = nexts.pop() else { break };
            if nxt == start_pt { break; }
            loop_pts.push(nxt);
            cur = nxt;
            if loop_pts.len() > gw * gh * 4 { break; } // safety
        }
        // clean empties
        edges.retain(|_, v| !v.is_empty());
        if loop_pts.len() >= 3 {
            contours.push(loop_pts.into_iter()
                .map(|(cx, cy)| (x0 + cx as f64 * cell, y0 + cy as f64 * cell))
                .collect());
        }
    }

    // simplify (drop collinear runs) and emit PathCmds relative to (x0,y0)
    let mut out = vec![];
    for c in &contours {
        let simp = simplify(c, cell * 1.2);
        if simp.len() < 3 { continue; }
        out.push(PathCmd::MoveTo(simp[0].0 - x0, simp[0].1 - y0));
        for p in &simp[1..] { out.push(PathCmd::LineTo(p.0 - x0, p.1 - y0)); }
        out.push(PathCmd::Close);
    }
    (out, (x0, y0), (x1 - x0, y1 - y0))
}

fn simplify(pts: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
    if pts.len() < 3 { return pts.to_vec(); }
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let a = *out.last().unwrap();
        let b = pts[i];
        let c = pts[i + 1];
        // keep b when it deviates from line a->c
        let area2 = ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs();
        let base = ((c.0 - a.0).powi(2) + (c.1 - a.1).powi(2)).sqrt().max(1e-9);
        if area2 / base > tol * 0.5 { out.push(b); }
    }
    out.push(*pts.last().unwrap());
    out
}

/// Anything renderable becomes boolean input: rects/ellipses convert
/// to paths; vectors pass through.
pub fn node_to_path(n: &Node) -> Option<Vec<PathCmd>> {
    match &n.kind {
        NodeKind::Vector { path } => Some(path.clone()),
        NodeKind::Rect { radius } => {
            let (w, h, r) = (n.w, n.h, radius.min(n.w / 2.0).min(n.h / 2.0));
            if *radius <= 0.0 {
                Some(vec![
                    PathCmd::MoveTo(0.0, 0.0), PathCmd::LineTo(w, 0.0),
                    PathCmd::LineTo(w, h), PathCmd::LineTo(0.0, h), PathCmd::Close,
                ])
            } else {
                let k = 0.5523 * r;
                Some(vec![
                    PathCmd::MoveTo(r, 0.0), PathCmd::LineTo(w - r, 0.0),
                    PathCmd::CurveTo(w - r + k, 0.0, w, r - k, w, r),
                    PathCmd::LineTo(w, h - r),
                    PathCmd::CurveTo(w, h - r + k, w - r + k, h, w - r, h),
                    PathCmd::LineTo(r, h),
                    PathCmd::CurveTo(r - k, h, 0.0, h - r + k, 0.0, h - r),
                    PathCmd::LineTo(0.0, r),
                    PathCmd::CurveTo(0.0, r - k, r - k, 0.0, r, 0.0),
                    PathCmd::Close,
                ])
            }
        }
        NodeKind::Ellipse => {
            let (rx, ry) = (n.w / 2.0, n.h / 2.0);
            let (kx, ky) = (0.5523 * rx, 0.5523 * ry);
            let (cx, cy) = (rx, ry);
            Some(vec![
                PathCmd::MoveTo(cx + rx, cy),
                PathCmd::CurveTo(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry),
                PathCmd::CurveTo(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy),
                PathCmd::CurveTo(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry),
                PathCmd::CurveTo(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy),
                PathCmd::Close,
            ])
        }
        _ => None,
    }
}

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
    use x_core::Color;

    #[test]
    fn booleans_2_0_default_backend_preserves_curves_end_to_end() {
        // ellipse ∪ rect through the DEFAULT backend: the result path must
        // still contain CurveTo commands (the old polygon default emitted
        // only LineTo) — this is the review's "Bezier output" requirement
        // verified at the boolean_selected level the app actually calls.
        let page = x_core::Node::frame("page", 400.0, 300.0)
            .child(x_core::Node::ellipse("e", 40.0, 40.0, 120.0, 120.0, Color::rgb8(255, 0, 0)))
            .child(x_core::Node::rect("r", 100.0, 40.0, 120.0, 120.0, Color::rgb8(0, 0, 255)));
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
    fn facade_api_is_backend_agnostic() {
        // two overlapping 100x100 squares, 50px apart -> union area 15000
        let sq: Vec<PathCmd> = vec![
            PathCmd::MoveTo(0.0, 0.0), PathCmd::LineTo(100.0, 0.0),
            PathCmd::LineTo(100.0, 100.0), PathCmd::LineTo(0.0, 100.0), PathCmd::Close,
        ];
        let a = PositionedPath { cmds: sq.clone(), offset: (0.0, 0.0) };
        let b = PositionedPath { cmds: sq, offset: (50.0, 0.0) };
        let default_res = boolean(BoolOp::Union, &a, &b);
        assert!(!default_res.cmds.is_empty());
        let expect = 15000.0;
        let got = area_of(&default_res.cmds);
        assert!((got - expect).abs() / expect < 0.08, "union area {got} vs {expect}");
        // both named backends produce results through the same signature —
        // but with DIFFERENT precision contracts: raster ~8%, exact <0.1%
        let r = boolean_with(Backend::RasterGuided, BoolOp::Intersect, &a, &b);
        let ia = area_of(&r.cmds);
        assert!((ia - 5000.0).abs() / 5000.0 < 0.08, "raster intersect {ia}");
        let r = boolean_with(Backend::Exact, BoolOp::Intersect, &a, &b);
        let ia = area_of(&r.cmds);
        assert!((ia - 5000.0).abs() / 5000.0 < 0.001, "exact intersect {ia}");
        // curves flatten to 16 segs: ellipse ∪ rect via exact stays within 1%
        let circle = crate::node_to_path(&x_core::Node::ellipse("c", 0.0, 0.0, 100.0, 100.0, Color::BLACK)).unwrap();
        let rectp = crate::node_to_path(&x_core::Node::rect("r", 0.0, 0.0, 100.0, 100.0, Color::BLACK)).unwrap();
        let pc = PositionedPath { cmds: circle, offset: (0.0, 0.0) };
        let pr = PositionedPath { cmds: rectp, offset: (50.0, 0.0) };
        let r = boolean_with(Backend::Exact, BoolOp::Union, &pc, &pr);
        let want = 10000.0 + (std::f64::consts::PI * 2500.0) / 2.0; // rect + left half-circle
        let got = area_of(&r.cmds);
        assert!((got - want).abs() / want < 0.01, "exact curve union {got} vs {want}");
    }

    fn area_of(path: &[PathCmd]) -> f64 {
        // shoelace over flattened contours
        let polys = flatten(path, (0.0, 0.0));
        let mut area = 0.0;
        for poly in &polys {
            let n = poly.len();
            let mut a = 0.0;
            for i in 0..n {
                let (x1, y1) = poly[i];
                let (x2, y2) = poly[(i + 1) % n];
                a += x1 * y2 - x2 * y1;
            }
            area += a.abs() / 2.0;
        }
        area
    }

    fn sq(size: f64) -> Vec<PathCmd> {
        vec![PathCmd::MoveTo(0.0, 0.0), PathCmd::LineTo(size, 0.0),
             PathCmd::LineTo(size, size), PathCmd::LineTo(0.0, size), PathCmd::Close]
    }

    #[test]
    fn boolean_areas_match_set_theory() {
        // two 100x100 squares overlapping by 50x100 -> known areas
        let a = sq(100.0);
        let b = sq(100.0);
        let cases = [
            (BoolOp::Union, 15000.0),      // 100*100*2 - 50*100
            (BoolOp::Intersect, 5000.0),   // 50*100
            (BoolOp::Subtract, 5000.0),    // 100*100 - 5000
            (BoolOp::Exclude, 10000.0),    // union - intersect
        ];
        for (op, expected) in cases {
            let (path, _, _) = boolean_paths(&a, (0.0, 0.0), &b, (50.0, 0.0), op);
            assert!(!path.is_empty(), "{op:?}: empty result");
            let area = area_of(&path);
            let err = (area - expected).abs() / expected;
            assert!(err < 0.08, "{op:?}: area {area:.0} vs expected {expected:.0} ({:.1}% off)", err * 100.0);
        }
    }

    #[test]
    fn union_of_disjoint_shapes_keeps_both() {
        let a = sq(100.0);
        let b = sq(80.0);
        let (path, _, _) = boolean_paths(&a, (0.0, 0.0), &b, (300.0, 20.0), BoolOp::Union);
        assert!(!path.is_empty(), "disjoint union must keep both shapes");
        let area = area_of(&path);
        let expected = 100.0 * 100.0 + 80.0 * 80.0;
        assert!((area - expected).abs() / expected < 0.1, "area {area} vs {expected}");
        let contours = path.iter().filter(|c| matches!(c, PathCmd::MoveTo(..))).count();
        assert!(contours >= 2, "two separate contours expected, got {contours}");
    }

    #[test]
    fn subtract_disjoint_keeps_a_intersect_empty() {
        let a = sq(50.0);
        let b = sq(50.0);
        // b far away
        let (path, _, _) = boolean_paths(&a, (0.0, 0.0), &b, (500.0, 0.0), BoolOp::Subtract);
        let area = area_of(&path);
        assert!((area - 2500.0).abs() / 2500.0 < 0.08, "subtract-disjoint keeps A: {area}");
        let (path, _, _) = boolean_paths(&a, (0.0, 0.0), &b, (500.0, 0.0), BoolOp::Intersect);
        assert!(path.is_empty() || area_of(&path) < 100.0, "disjoint intersect ~empty");
    }

    #[test]
    fn subtract_hole_yields_ring_with_two_contours() {
        // big square minus centered small square -> ring (2 contours, even-odd)
        let a = sq(100.0);
        let b = sq(40.0);
        let (path, _, _) = boolean_paths(&a, (0.0, 0.0), &b, (30.0, 30.0), BoolOp::Subtract);
        let contours = path.iter().filter(|c| matches!(c, PathCmd::MoveTo(..))).count();
        assert!(contours >= 2, "ring needs an outer and an inner contour, got {contours}");
        let area = area_of(&path);
        // outer 10000 + hole traced as its own contour: |area| sums both
        // even-odd rendering makes the hole transparent; area check loose
        assert!(area > 9000.0, "ring area sum: {area}");
    }

    #[test]
    fn editor_boolean_replaces_selection_undoably() {
        let mut e = Editor::new(
            Node::frame("page", 400.0, 300.0)
                .child(Node::rect("a", 10.0, 10.0, 100.0, 100.0, Color::rgb8(255, 0, 0)))
                .child(Node::ellipse("b", 60.0, 10.0, 100.0, 100.0, Color::rgb8(0, 255, 0))),
        );
        e.selection = vec!["a".into(), "b".into()];
        let id = e.boolean_selected(BoolOp::Union).expect("union");
        assert!(find(&e.root, "a").is_none() && find(&e.root, "b").is_none());
        let v = find(&e.root, &id).unwrap();
        assert!(matches!(&v.kind, NodeKind::Vector { path } if !path.is_empty()));
        assert!(matches!(&v.fill, x_core::Paint::Solid(c) if c.r == 255), "keeps A's fill");
        // one undo restores both inputs and removes the result
        e.undo();
        assert!(find(&e.root, "a").is_some() && find(&e.root, "b").is_some());
        assert!(find(&e.root, &id).is_none());
    }
}
