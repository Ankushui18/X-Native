//! Bounded undo for uncommitted inline text; document undo starts at commit.
use crate::state::App;

#[derive(Clone)]
pub struct TextSnapshot {
    buffer: String,
    runs: Vec<x_native::TextRun>,
    caret: usize,
    anchor: Option<usize>,
}
impl App {
    fn text_snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            buffer: self.text_buffer.clone(),
            runs: self.text_runs_edit.clone(),
            caret: self.text_caret,
            anchor: self.text_anchor,
        }
    }
    pub fn checkpoint_text(&mut self) {
        self.text_undo.push(self.text_snapshot());
        self.text_redo.clear();
        while self.text_undo.len() > 1
            && (self.text_undo.len() > 64
                || self
                    .text_undo
                    .iter()
                    .map(|s| s.buffer.len() + s.runs.len() * 128)
                    .sum::<usize>()
                    > 8 * 1024 * 1024)
        {
            self.text_undo.remove(0);
        }
    }
    pub fn undo_text(&mut self, redo: bool) -> bool {
        let snapshot = if redo {
            self.text_redo.pop()
        } else {
            self.text_undo.pop()
        };
        let Some(snapshot) = snapshot else {
            return false;
        };
        let old = self.text_snapshot();
        if redo {
            self.text_undo.push(old);
        } else {
            self.text_redo.push(old);
        }
        self.text_buffer = snapshot.buffer;
        self.text_runs_edit = snapshot.runs;
        self.text_caret = snapshot.caret;
        self.text_anchor = snapshot.anchor;
        true
    }
    pub fn pending_text_dirty(&self) -> bool {
        let Some(id) = &self.text_edit else {
            return false;
        };
        let Some(doc) = self.doc_opt() else {
            return false;
        };
        x_native::editor::find(&doc.editor_ref().root, id).is_some_and(|n| {
            matches!(&n.kind, x_native::NodeKind::Text { text } if text != &self.text_buffer)
                || n.text_runs != self.text_runs_edit
        })
    }
    pub fn copy_inline_text(&mut self) {
        if let Some((a, b)) = self.text_sel_range() {
            self.text_clipboard = self.text_buffer.chars().skip(a).take(b - a).collect();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(self.text_clipboard.clone());
            }
        }
    }
    pub fn clipboard_text(&self) -> String {
        arboard::Clipboard::new()
            .ok()
            .and_then(|mut c| c.get_text().ok())
            .unwrap_or_else(|| self.text_clipboard.clone())
    }
}
