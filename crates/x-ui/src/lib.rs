//! x-ui — retained editor-chrome UI layer (P1).
//!
//! Separates Editor UI from the Document Renderer. Widgets are retained
//! objects with identity, state, layout, and SEMANTICS — not per-frame
//! pixel code. One tree owns:
//! - hit testing & event routing (click, hover, key)
//! - keyboard focus (Tab / Shift+Tab traversal, Enter/Space activation)
//! - accessibility semantics (role, label, value, focus order — the
//!   screen-reader contract, AccessKit-shaped)
//! - theme tokens: high-contrast palette + global UI scale
//! - painting through any FnMut(&Paint...) — Vello today, anything later
//!
//! The design app migrates chrome onto this incrementally; new chrome
//! (menus, dialogs, fields) starts here so it stops being pixel code.

pub mod containers;
pub use containers::{ScrollView, Dropdown, DropdownEvent, Menu, MenuItem, TooltipState, Modal, ModalEvent, MENU_ROW_H, MENU_W};

use std::collections::HashMap;

// ------------------------------------------------------------------ theme

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub panel: [u8; 3],
    pub text: [u8; 3],
    pub dim: [u8; 3],
    pub accent: [u8; 3],
    pub focus_ring: [u8; 3],
    /// global UI scale (accessibility: scalable UI)
    pub scale: f64,
    pub high_contrast: bool,
    pub reduced_motion: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            panel: [0x1b, 0x1d, 0x23], text: [0xf2, 0xf3, 0xf7], dim: [0x9a, 0x9e, 0xaa],
            accent: [0x7c, 0x5c, 0xfc], focus_ring: [0xa9, 0x96, 0xff],
            scale: 1.0, high_contrast: false, reduced_motion: false,
        }
    }
}

impl Theme {
    pub fn high_contrast() -> Self {
        Self {
            panel: [0x00, 0x00, 0x00], text: [0xff, 0xff, 0xff], dim: [0xdd, 0xdd, 0xdd],
            accent: [0x9f, 0x88, 0xff], focus_ring: [0xff, 0xff, 0x00],
            high_contrast: true, ..Self::default()
        }
    }
}

// -------------------------------------------------------------- semantics

/// Screen-reader roles (AccessKit-compatible subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role { Button, TextField, Checkbox, Tab, List, ListItem, Slider, Label, Panel, MenuItem }

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticsNode {
    pub id: WidgetId,
    pub role: Role,
    pub label: String,
    pub value: Option<String>,
    pub focused: bool,
    pub disabled: bool,
}

// ---------------------------------------------------------------- widgets

pub type WidgetId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiRect { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }
impl UiRect {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Button { text: String },
    Checkbox { text: String, checked: bool },
    TextField { text: String, cursor: usize, placeholder: String },
    Tab { text: String, active: bool },
    Label { text: String },
    Slider { value: f64, min: f64, max: f64 },
}

#[derive(Debug, Clone)]
pub struct Widget {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub rect: UiRect,
    pub label: String,       // accessibility label (falls back to text)
    pub disabled: bool,
    pub visible: bool,
    pub tab_index: Option<u32>, // focus order; None = not focusable
}

// ------------------------------------------------------------------ events

#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    Clicked(WidgetId),
    Toggled(WidgetId, bool),
    TextChanged(WidgetId, String),
    Submitted(WidgetId, String),
    ValueChanged(WidgetId, f64),
    FocusMoved(WidgetId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    PointerDown { x: f64, y: f64 },
    PointerMove { x: f64, y: f64 },
    Key(KeyInput),
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyInput {
    Tab { shift: bool },
    Enter,
    Space,
    Backspace,
    Escape,
    Left,
    Right,
    Char(char),
}

// -------------------------------------------------------------------- tree

#[derive(Default)]
pub struct UiTree {
    pub widgets: Vec<Widget>,
    index: HashMap<WidgetId, usize>,
    pub focus: Option<WidgetId>,
    pub hover: Option<WidgetId>,
    pub theme: Theme,
    next_id: WidgetId,
}

impl UiTree {
    pub fn new() -> Self { Self { next_id: 1, ..Default::default() } }

    pub fn add(&mut self, kind: WidgetKind, rect: UiRect, tab_index: Option<u32>) -> WidgetId {
        let id = self.next_id;
        self.next_id += 1;
        let label = match &kind {
            WidgetKind::Button { text } | WidgetKind::Checkbox { text, .. }
            | WidgetKind::Tab { text, .. } | WidgetKind::Label { text } => text.clone(),
            WidgetKind::TextField { placeholder, .. } => placeholder.clone(),
            WidgetKind::Slider { .. } => "slider".into(),
        };
        self.widgets.push(Widget { id, kind, rect, label, disabled: false, visible: true, tab_index });
        self.index.insert(id, self.widgets.len() - 1);
        id
    }

    pub fn get(&self, id: WidgetId) -> Option<&Widget> { self.index.get(&id).map(|&i| &self.widgets[i]) }
    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut Widget> {
        let i = *self.index.get(&id)?;
        self.widgets.get_mut(i)
    }
    pub fn set_label(&mut self, id: WidgetId, label: &str) {
        if let Some(w) = self.get_mut(id) { w.label = label.into(); }
    }

    // ---- focus system ----

    fn focus_order(&self) -> Vec<WidgetId> {
        let mut f: Vec<&Widget> = self.widgets.iter()
            .filter(|w| w.tab_index.is_some() && w.visible && !w.disabled)
            .collect();
        f.sort_by_key(|w| w.tab_index.unwrap());
        f.iter().map(|w| w.id).collect()
    }

    pub fn focus_next(&mut self) -> Option<WidgetId> { self.focus_step(1) }
    pub fn focus_prev(&mut self) -> Option<WidgetId> { self.focus_step(-1) }

    fn focus_step(&mut self, dir: i64) -> Option<WidgetId> {
        let order = self.focus_order();
        if order.is_empty() { return None; }
        let next = match self.focus.and_then(|f| order.iter().position(|&o| o == f)) {
            Some(i) => order[((i as i64 + dir).rem_euclid(order.len() as i64)) as usize],
            None => if dir > 0 { order[0] } else { *order.last().unwrap() },
        };
        self.focus = Some(next);
        Some(next)
    }

    // ---- event routing ----

    pub fn handle(&mut self, ev: InputEvent) -> Vec<UiEvent> {
        let mut out = vec![];
        match ev {
            InputEvent::PointerMove { x, y } => {
                self.hover = self.widgets.iter().rev()
                    .find(|w| w.visible && !w.disabled && w.rect.contains(x, y))
                    .map(|w| w.id);
            }
            InputEvent::PointerDown { x, y } => {
                let hit = self.widgets.iter().rev()
                    .find(|w| w.visible && !w.disabled && w.rect.contains(x, y))
                    .map(|w| (w.id, w.kind.clone(), w.rect));
                if let Some((id, kind, rect)) = hit {
                    if self.get(id).unwrap().tab_index.is_some() {
                        self.focus = Some(id);
                        out.push(UiEvent::FocusMoved(id));
                    }
                    match kind {
                        WidgetKind::Button { .. } | WidgetKind::Tab { .. } => out.push(UiEvent::Clicked(id)),
                        WidgetKind::Checkbox { checked, .. } => {
                            let nv = !checked;
                            if let Some(w) = self.get_mut(id) {
                                if let WidgetKind::Checkbox { checked, .. } = &mut w.kind { *checked = nv; }
                            }
                            out.push(UiEvent::Toggled(id, nv));
                        }
                        WidgetKind::Slider { min, max, .. } => {
                            let t = ((x - rect.x) / rect.w).clamp(0.0, 1.0);
                            let nv = min + t * (max - min);
                            if let Some(w) = self.get_mut(id) {
                                if let WidgetKind::Slider { value, .. } = &mut w.kind { *value = nv; }
                            }
                            out.push(UiEvent::ValueChanged(id, nv));
                        }
                        WidgetKind::TextField { .. } | WidgetKind::Label { .. } => {}
                    }
                } else {
                    self.focus = None;
                }
            }
            InputEvent::Key(k) => {
                match k {
                    KeyInput::Tab { shift } => {
                        let id = if shift { self.focus_prev() } else { self.focus_next() };
                        if let Some(id) = id { out.push(UiEvent::FocusMoved(id)); }
                    }
                    _ => if let Some(fid) = self.focus {
                        out.extend(self.key_to_focused(fid, k));
                    },
                }
            }
        }
        out
    }

    fn key_to_focused(&mut self, id: WidgetId, k: KeyInput) -> Vec<UiEvent> {
        let mut out = vec![];
        let Some(w) = self.get_mut(id) else { return out };
        match (&mut w.kind, k) {
            (WidgetKind::Button { .. } | WidgetKind::Tab { .. }, KeyInput::Enter | KeyInput::Space) =>
                out.push(UiEvent::Clicked(id)),
            (WidgetKind::Checkbox { checked, .. }, KeyInput::Enter | KeyInput::Space) => {
                *checked = !*checked;
                let nv = *checked;
                out.push(UiEvent::Toggled(id, nv));
            }
            (WidgetKind::TextField { text, cursor, .. }, KeyInput::Char(c)) => {
                text.insert(*cursor, c);
                *cursor += c.len_utf8();
                let t = text.clone();
                out.push(UiEvent::TextChanged(id, t));
            }
            (WidgetKind::TextField { text, cursor, .. }, KeyInput::Backspace) => {
                if *cursor > 0 {
                    let prev = text[..*cursor].chars().last().map(|c| c.len_utf8()).unwrap_or(0);
                    *cursor -= prev;
                    text.remove(*cursor);
                    let t = text.clone();
                    out.push(UiEvent::TextChanged(id, t));
                }
            }
            (WidgetKind::TextField { cursor, .. }, KeyInput::Left) => { *cursor = cursor.saturating_sub(1); }
            (WidgetKind::TextField { text, cursor, .. }, KeyInput::Right) => {
                if *cursor < text.len() { *cursor += 1; }
            }
            (WidgetKind::TextField { text, .. }, KeyInput::Enter) => {
                let t = text.clone();
                out.push(UiEvent::Submitted(id, t));
            }
            (WidgetKind::Slider { value, min, max }, KeyInput::Left) => {
                *value = (*value - (*max - *min) / 20.0).max(*min);
                let nv = *value;
                out.push(UiEvent::ValueChanged(id, nv));
            }
            (WidgetKind::Slider { value, min, max }, KeyInput::Right) => {
                *value = (*value + (*max - *min) / 20.0).min(*max);
                let nv = *value;
                out.push(UiEvent::ValueChanged(id, nv));
            }
            _ => {}
        }
        out
    }

    // ---- accessibility export ----

    /// Semantics tree for screen readers (AccessKit-shaped snapshot).
    pub fn semantics(&self) -> Vec<SemanticsNode> {
        self.widgets.iter().filter(|w| w.visible).map(|w| SemanticsNode {
            id: w.id,
            role: match &w.kind {
                WidgetKind::Button { .. } => Role::Button,
                WidgetKind::Checkbox { .. } => Role::Checkbox,
                WidgetKind::TextField { .. } => Role::TextField,
                WidgetKind::Tab { .. } => Role::Tab,
                WidgetKind::Label { .. } => Role::Label,
                WidgetKind::Slider { .. } => Role::Slider,
            },
            label: w.label.clone(),
            value: match &w.kind {
                WidgetKind::TextField { text, .. } => Some(text.clone()),
                WidgetKind::Checkbox { checked, .. } => Some(checked.to_string()),
                WidgetKind::Slider { value, .. } => Some(format!("{value:.2}")),
                _ => None,
            },
            focused: self.focus == Some(w.id),
            disabled: w.disabled,
        }).collect()
    }
}


// ---------------------------------------------------------------- painting

/// Paint instruction — backend-agnostic; the app maps these to Vello.
#[derive(Debug, Clone, PartialEq)]
pub enum PaintOp {
    Rect { r: UiRect, color: [u8; 3], alpha: u8, radius: f64 },
    Border { r: UiRect, color: [u8; 3], width: f64 },
    Text { x: f64, y: f64, size: f64, color: [u8; 3], text: String },
}

/// Retained paint pass: widgets -> ops, honoring theme scale/contrast and
/// drawing the focus ring (keyboard visibility, WCAG).
pub fn paint(tree: &UiTree) -> Vec<PaintOp> {
    let th = &tree.theme;
    let s = th.scale;
    let mut ops = vec![];
    for w in &tree.widgets {
        if !w.visible { continue; }
        let r = UiRect { x: w.rect.x * s, y: w.rect.y * s, w: w.rect.w * s, h: w.rect.h * s };
        let focused = tree.focus == Some(w.id);
        let hovered = tree.hover == Some(w.id);
        match &w.kind {
            WidgetKind::Button { text } | WidgetKind::Tab { text, active: _ } => {
                let active = matches!(&w.kind, WidgetKind::Tab { active: true, .. });
                let bg = if active { th.accent } else if hovered { [0x33, 0x36, 0x3d] } else { th.panel };
                ops.push(PaintOp::Rect { r, color: bg, alpha: 255, radius: 5.0 * s });
                ops.push(PaintOp::Text { x: r.x + 8.0 * s, y: r.y + r.h / 2.0 - 5.0 * s, size: 9.0 * s, color: if active { [255, 255, 255] } else { th.text }, text: text.clone() });
            }
            WidgetKind::Checkbox { text, checked } => {
                ops.push(PaintOp::Border { r: UiRect { x: r.x, y: r.y, w: 12.0 * s, h: 12.0 * s }, color: th.dim, width: 1.0 });
                if *checked {
                    ops.push(PaintOp::Rect { r: UiRect { x: r.x + 2.0 * s, y: r.y + 2.0 * s, w: 8.0 * s, h: 8.0 * s }, color: th.accent, alpha: 255, radius: 1.0 });
                }
                ops.push(PaintOp::Text { x: r.x + 18.0 * s, y: r.y, size: 9.0 * s, color: th.text, text: text.clone() });
            }
            WidgetKind::TextField { text, placeholder, .. } => {
                ops.push(PaintOp::Rect { r, color: [0x1a, 0x1c, 0x20], alpha: 255, radius: 4.0 * s });
                ops.push(PaintOp::Border { r, color: if focused { th.accent } else { th.dim }, width: 1.0 });
                let shown = if text.is_empty() { placeholder } else { text };
                let color = if text.is_empty() { th.dim } else { th.text };
                ops.push(PaintOp::Text { x: r.x + 6.0 * s, y: r.y + r.h / 2.0 - 5.0 * s, size: 9.0 * s, color, text: shown.clone() });
            }
            WidgetKind::Label { text } => {
                ops.push(PaintOp::Text { x: r.x, y: r.y, size: 9.0 * s, color: th.dim, text: text.clone() });
            }
            WidgetKind::Slider { value, min, max } => {
                ops.push(PaintOp::Rect { r: UiRect { x: r.x, y: r.y + r.h / 2.0 - 2.0, w: r.w, h: 4.0 }, color: th.dim, alpha: 160, radius: 2.0 });
                let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
                ops.push(PaintOp::Rect { r: UiRect { x: r.x + t * r.w - 5.0, y: r.y + r.h / 2.0 - 7.0, w: 10.0, h: 14.0 }, color: th.accent, alpha: 255, radius: 5.0 });
            }
        }
        if focused {
            // focus ring: always visible for keyboard users
            ops.push(PaintOp::Border { r: UiRect { x: r.x - 2.0, y: r.y - 2.0, w: r.w + 4.0, h: r.h + 4.0 }, color: th.focus_ring, width: 2.0 });
        }
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> (UiTree, WidgetId, WidgetId, WidgetId, WidgetId) {
        let mut t = UiTree::new();
        let b = t.add(WidgetKind::Button { text: "Save".into() }, UiRect { x: 0.0, y: 0.0, w: 60.0, h: 20.0 }, Some(1));
        let c = t.add(WidgetKind::Checkbox { text: "Rulers".into(), checked: false }, UiRect { x: 0.0, y: 30.0, w: 80.0, h: 16.0 }, Some(2));
        let f = t.add(WidgetKind::TextField { text: String::new(), cursor: 0, placeholder: "Search".into() }, UiRect { x: 0.0, y: 60.0, w: 120.0, h: 20.0 }, Some(3));
        let s = t.add(WidgetKind::Slider { value: 0.5, min: 0.0, max: 1.0 }, UiRect { x: 0.0, y: 90.0, w: 100.0, h: 16.0 }, Some(4));
        (t, b, c, f, s)
    }

    #[test]
    fn tab_cycles_focus_in_order_and_wraps() {
        let (mut t, b, c, f, s) = sample_tree();
        assert_eq!(t.handle(InputEvent::Key(KeyInput::Tab { shift: false })), vec![UiEvent::FocusMoved(b)]);
        t.handle(InputEvent::Key(KeyInput::Tab { shift: false }));
        assert_eq!(t.focus, Some(c));
        t.handle(InputEvent::Key(KeyInput::Tab { shift: false }));
        t.handle(InputEvent::Key(KeyInput::Tab { shift: false }));
        assert_eq!(t.focus, Some(s));
        t.handle(InputEvent::Key(KeyInput::Tab { shift: false }));
        assert_eq!(t.focus, Some(b), "wraps to start");
        t.handle(InputEvent::Key(KeyInput::Tab { shift: true }));
        assert_eq!(t.focus, Some(s), "shift-tab goes back");
        let _ = f;
    }

    #[test]
    fn keyboard_activates_focused_widgets() {
        let (mut t, b, c, _f, s) = sample_tree();
        t.focus = Some(b);
        assert_eq!(t.handle(InputEvent::Key(KeyInput::Enter)), vec![UiEvent::Clicked(b)]);
        t.focus = Some(c);
        assert_eq!(t.handle(InputEvent::Key(KeyInput::Space)), vec![UiEvent::Toggled(c, true)]);
        t.focus = Some(s);
        let ev = t.handle(InputEvent::Key(KeyInput::Right));
        assert!(matches!(ev[0], UiEvent::ValueChanged(_, v) if v > 0.5));
    }

    #[test]
    fn text_field_edits_via_keyboard() {
        let (mut t, _b, _c, f, _s) = sample_tree();
        t.focus = Some(f);
        for ch in "hi!".chars() { t.handle(InputEvent::Key(KeyInput::Char(ch))); }
        assert!(matches!(&t.get(f).unwrap().kind, WidgetKind::TextField { text, .. } if text == "hi!"));
        t.handle(InputEvent::Key(KeyInput::Backspace));
        assert!(matches!(&t.get(f).unwrap().kind, WidgetKind::TextField { text, .. } if text == "hi"));
        let ev = t.handle(InputEvent::Key(KeyInput::Enter));
        assert_eq!(ev, vec![UiEvent::Submitted(f, "hi".into())]);
    }

    #[test]
    fn pointer_clicks_focus_and_activate() {
        let (mut t, b, c, _f, s) = sample_tree();
        let ev = t.handle(InputEvent::PointerDown { x: 10.0, y: 10.0 });
        assert!(ev.contains(&UiEvent::FocusMoved(b)));
        assert!(ev.contains(&UiEvent::Clicked(b)));
        let ev = t.handle(InputEvent::PointerDown { x: 10.0, y: 35.0 });
        assert!(ev.contains(&UiEvent::Toggled(c, true)));
        // slider jump-to-position
        let ev = t.handle(InputEvent::PointerDown { x: 75.0, y: 95.0 });
        assert!(matches!(ev.iter().find(|e| matches!(e, UiEvent::ValueChanged(..))), Some(UiEvent::ValueChanged(_, v)) if (*v - 0.75).abs() < 0.01));
        let _ = s;
        // click empty space clears focus
        t.handle(InputEvent::PointerDown { x: 500.0, y: 500.0 });
        assert_eq!(t.focus, None);
    }

    #[test]
    fn semantics_tree_reports_roles_labels_values_focus() {
        let (mut t, b, c, f, _s) = sample_tree();
        t.focus = Some(f);
        t.handle(InputEvent::Key(KeyInput::Char('x')));
        let sem = t.semantics();
        let by_id = |id| sem.iter().find(|n| n.id == id).unwrap();
        assert_eq!(by_id(b).role, Role::Button);
        assert_eq!(by_id(b).label, "Save");
        assert_eq!(by_id(c).role, Role::Checkbox);
        assert_eq!(by_id(c).value.as_deref(), Some("false"));
        assert_eq!(by_id(f).role, Role::TextField);
        assert_eq!(by_id(f).value.as_deref(), Some("x"));
        assert!(by_id(f).focused);
        assert!(!by_id(b).focused);
    }

    #[test]
    fn paint_scales_with_theme_and_draws_focus_ring() {
        let (mut t, b, _c, _f, _s) = sample_tree();
        t.focus = Some(b);
        let ops1 = paint(&t);
        // focus ring present
        assert!(ops1.iter().any(|o| matches!(o, PaintOp::Border { color, .. } if *color == t.theme.focus_ring)));
        // scale 2x doubles geometry
        t.theme.scale = 2.0;
        let ops2 = paint(&t);
        let first_rect = |ops: &[PaintOp]| ops.iter().find_map(|o| match o { PaintOp::Rect { r, .. } => Some(*r), _ => None }).unwrap();
        assert_eq!(first_rect(&ops2).w, first_rect(&ops1).w * 2.0);
        // high contrast palette switches colors
        t.theme = Theme::high_contrast();
        let ops3 = paint(&t);
        assert!(ops3.iter().any(|o| matches!(o, PaintOp::Text { color, .. } if *color == [255, 255, 255])));
    }

    #[test]
    fn disabled_and_hidden_widgets_skip_focus_and_hit() {
        let (mut t, b, c, _f, _s) = sample_tree();
        t.get_mut(b).unwrap().disabled = true;
        t.get_mut(c).unwrap().visible = false;
        let order = { t.handle(InputEvent::Key(KeyInput::Tab { shift: false })); t.focus };
        assert!(order != Some(b) && order != Some(c), "focus must skip disabled+hidden");
        let ev = t.handle(InputEvent::PointerDown { x: 10.0, y: 10.0 });
        assert!(ev.iter().all(|e| !matches!(e, UiEvent::Clicked(id) if *id == b)));
    }
}
