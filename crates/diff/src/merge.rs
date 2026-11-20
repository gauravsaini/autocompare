//! Editable text buffer and 3-way Take merge session.
//!
//! Layer 0: no panics, no unwrap/expect in non-test code, no unsafe.

#![allow(dead_code)]

use crate::merge3::{Merge3Block, merge3_blocks};

// ----------------------------------------------------------------------------
// Editable Text Buffer with Undo / Redo
// ----------------------------------------------------------------------------

/// An edit action recorded in the undo stack.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BufferAction {
    InsertLine {
        idx: usize,
        content: String,
    },
    DeleteLine {
        idx: usize,
        content: String,
    },
    ReplaceLine {
        idx: usize,
        old: String,
        new: String,
    },
    SpliceLines {
        idx: usize,
        deleted: Vec<String>,
        inserted: Vec<String>,
    },
}

/// An in-memory editable text buffer organized by lines, supporting undo/redo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextBuffer {
    lines: Vec<String>,
    undo_stack: Vec<BufferAction>,
    redo_stack: Vec<BufferAction>,
    is_dirty: bool,
}

impl TextBuffer {
    /// Creates a new empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a buffer from a multi-line string.
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            Vec::new()
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };
        Self {
            lines,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_dirty: false,
        }
    }

    /// Creates a buffer directly from a vector of line strings.
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self {
            lines,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_dirty: false,
        }
    }

    /// Returns the number of lines in the buffer.
    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    /// Returns true if the buffer has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Borrows the lines of the buffer.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Gets a reference to a single line by 0-based index.
    pub fn line(&self, idx: usize) -> Option<&str> {
        self.lines.get(idx).map(|s| s.as_str())
    }

    /// Inserts a new line at 0-based `idx`. If `idx >= len`, appends to the end.
    pub fn insert_line(&mut self, idx: usize, content: String) {
        let insert_idx = idx.min(self.lines.len());
        self.lines.insert(insert_idx, content.clone());
        self.undo_stack.push(BufferAction::InsertLine {
            idx: insert_idx,
            content,
        });
        self.redo_stack.clear();
        self.is_dirty = true;
    }

    /// Deletes a line at 0-based `idx`, returning the deleted content.
    pub fn delete_line(&mut self, idx: usize) -> Option<String> {
        if idx >= self.lines.len() {
            return None;
        }
        let removed = self.lines.remove(idx);
        self.undo_stack.push(BufferAction::DeleteLine {
            idx,
            content: removed.clone(),
        });
        self.redo_stack.clear();
        self.is_dirty = true;
        Some(removed)
    }

    /// Replaces the line at 0-based `idx`, returning the previous content.
    pub fn replace_line(&mut self, idx: usize, content: String) -> Option<String> {
        let slot = self.lines.get_mut(idx)?;
        let old = std::mem::replace(slot, content.clone());
        self.undo_stack.push(BufferAction::ReplaceLine {
            idx,
            old: old.clone(),
            new: content,
        });
        self.redo_stack.clear();
        self.is_dirty = true;
        Some(old)
    }

    /// Splices a range of lines: deletes `delete_count` lines starting at `idx`
    /// and inserts `new_lines` in their place.
    pub fn splice_lines(
        &mut self,
        idx: usize,
        delete_count: usize,
        new_lines: Vec<String>,
    ) -> Vec<String> {
        let start = idx.min(self.lines.len());
        let end = (start.saturating_add(delete_count)).min(self.lines.len());
        let deleted: Vec<String> = self.lines.drain(start..end).collect();

        for (i, line) in new_lines.iter().enumerate() {
            self.lines.insert(start.saturating_add(i), line.clone());
        }

        self.undo_stack.push(BufferAction::SpliceLines {
            idx: start,
            deleted: deleted.clone(),
            inserted: new_lines,
        });
        self.redo_stack.clear();
        self.is_dirty = true;
        deleted
    }

    /// Can an undo operation be performed?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Reverts the most recent edit action. Returns true if an action was undone.
    pub fn undo(&mut self) -> bool {
        let action = match self.undo_stack.pop() {
            Some(a) => a,
            None => return false,
        };

        match &action {
            BufferAction::InsertLine { idx, .. } => {
                if *idx < self.lines.len() {
                    self.lines.remove(*idx);
                }
            }
            BufferAction::DeleteLine { idx, content } => {
                let insert_pos = (*idx).min(self.lines.len());
                self.lines.insert(insert_pos, content.clone());
            }
            BufferAction::ReplaceLine { idx, old, .. } => {
                if let Some(slot) = self.lines.get_mut(*idx) {
                    *slot = old.clone();
                }
            }
            BufferAction::SpliceLines {
                idx,
                deleted,
                inserted,
            } => {
                let end = (idx.saturating_add(inserted.len())).min(self.lines.len());
                self.lines.drain(*idx..end);
                for (offset, line) in deleted.iter().enumerate() {
                    let pos = idx.saturating_add(offset).min(self.lines.len());
                    self.lines.insert(pos, line.clone());
                }
            }
        }

        self.redo_stack.push(action);
        self.is_dirty = true;
        true
    }

    /// Can a redo operation be performed?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Re-applies the most recently undone edit action.
    pub fn redo(&mut self) -> bool {
        let action = match self.redo_stack.pop() {
            Some(a) => a,
            None => return false,
        };

        match &action {
            BufferAction::InsertLine { idx, content } => {
                let insert_pos = (*idx).min(self.lines.len());
                self.lines.insert(insert_pos, content.clone());
            }
            BufferAction::DeleteLine { idx, .. } => {
                if *idx < self.lines.len() {
                    self.lines.remove(*idx);
                }
            }
            BufferAction::ReplaceLine { idx, new, .. } => {
                if let Some(slot) = self.lines.get_mut(*idx) {
                    *slot = new.clone();
                }
            }
            BufferAction::SpliceLines {
                idx,
                deleted,
                inserted,
            } => {
                let end = (idx.saturating_add(deleted.len())).min(self.lines.len());
                self.lines.drain(*idx..end);
                for (offset, line) in inserted.iter().enumerate() {
                    let pos = idx.saturating_add(offset).min(self.lines.len());
                    self.lines.insert(pos, line.clone());
                }
            }
        }

        self.undo_stack.push(action);
        self.is_dirty = true;
        true
    }

    /// Converts the buffer contents to a single multi-line string.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            out.push_str(line);
            if i.saturating_add(1) < self.lines.len() {
                out.push('\n');
            }
        }
        out
    }

    /// Whether the buffer has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Resets the dirty tracking flag.
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Clears all lines from the buffer, recording the change in the undo stack.
    pub fn clear(&mut self) {
        if self.lines.is_empty() {
            return;
        }
        let count = self.lines.len();
        let _ = self.splice_lines(0, count, Vec::new());
    }

    /// Appends a new line at the end of the buffer.
    pub fn append_line(&mut self, content: String) {
        self.insert_line(self.lines.len(), content);
    }

    /// Returns the number of edit actions stored in the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of undone actions available in the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

// ----------------------------------------------------------------------------
// 3-Way Take Merge
// ----------------------------------------------------------------------------

/// Choice of how to resolve a merge conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeChoice {
    /// Take the original base lines.
    Base,
    /// Take the left contributor lines.
    Left,
    /// Take the right contributor lines.
    Right,
    /// Take both, with left lines appearing before right lines.
    BothLeftFirst,
    /// Take both, with right lines appearing before left lines.
    BothRightFirst,
}

/// Resolution state for a merge conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeResolution {
    /// Resolved via standard choice (Base / Left / Right / Both).
    Choice(TakeChoice),
    /// Resolved via arbitrary custom lines supplied by user/script.
    Custom(Vec<String>),
}

/// A block within a 3-way merge session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeBlock {
    /// A non-conflicting block: lines agree or edit was one-sided.
    Clean(Vec<String>),
    /// A conflicting block where left and right disagree.
    Conflict {
        id: usize,
        base: Vec<String>,
        left: Vec<String>,
        right: Vec<String>,
        resolution: Option<TakeResolution>,
    },
}

/// An interactive 3-way merge session supporting Take resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSession {
    blocks: Vec<MergeBlock>,
}

impl MergeSession {
    /// Constructs a new 3-way merge session from base, left, and right lines.
    pub fn new(base: &[&str], left: &[&str], right: &[&str]) -> Self {
        let raw_blocks = merge3_blocks(base, left, right);
        let mut blocks = Vec::with_capacity(raw_blocks.len());
        let mut conflict_id = 0usize;

        for raw in raw_blocks {
            match raw {
                Merge3Block::Clean(lines) => {
                    blocks.push(MergeBlock::Clean(lines));
                }
                Merge3Block::Conflict { base, left, right } => {
                    blocks.push(MergeBlock::Conflict {
                        id: conflict_id,
                        base,
                        left,
                        right,
                        resolution: None,
                    });
                    conflict_id += 1;
                }
            }
        }

        Self { blocks }
    }

    /// Returns all merge blocks in order.
    pub fn blocks(&self) -> &[MergeBlock] {
        &self.blocks
    }

    /// Total number of conflict blocks.
    pub fn conflict_count(&self) -> usize {
        self.blocks
            .iter()
            .filter(|b| matches!(b, MergeBlock::Conflict { .. }))
            .count()
    }

    /// Number of conflict blocks that have not yet been resolved.
    pub fn unresolved_count(&self) -> usize {
        self.blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    MergeBlock::Conflict {
                        resolution: None,
                        ..
                    }
                )
            })
            .count()
    }

    /// Number of conflict blocks that have been resolved.
    pub fn resolved_count(&self) -> usize {
        self.conflict_count()
            .saturating_sub(self.unresolved_count())
    }

    /// Returns a list of all conflict IDs in appearance order.
    pub fn conflict_ids(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                MergeBlock::Conflict { id, .. } => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// ID of the first unresolved conflict block, if any.
    pub fn first_unresolved(&self) -> Option<usize> {
        self.blocks.iter().find_map(|b| match b {
            MergeBlock::Conflict {
                id,
                resolution: None,
                ..
            } => Some(*id),
            _ => None,
        })
    }

    /// Finds the next unresolved conflict strictly after `from_id`.
    pub fn next_unresolved(&self, from_id: usize) -> Option<usize> {
        self.blocks.iter().find_map(|b| match b {
            MergeBlock::Conflict {
                id,
                resolution: None,
                ..
            } if *id > from_id => Some(*id),
            _ => None,
        })
    }

    /// Finds the previous unresolved conflict strictly before `from_id`.
    pub fn prev_unresolved(&self, from_id: usize) -> Option<usize> {
        self.blocks.iter().rev().find_map(|b| match b {
            MergeBlock::Conflict {
                id,
                resolution: None,
                ..
            } if *id < from_id => Some(*id),
            _ => None,
        })
    }

    /// Returns a reference to a conflict block by `conflict_id`.
    pub fn conflict_by_id(&self, conflict_id: usize) -> Option<&MergeBlock> {
        self.blocks.iter().find(|b| match b {
            MergeBlock::Conflict { id, .. } => *id == conflict_id,
            _ => false,
        })
    }

    /// Returns a mutable reference to a conflict block by `conflict_id`.
    pub fn conflict_by_id_mut(&mut self, conflict_id: usize) -> Option<&mut MergeBlock> {
        self.blocks.iter_mut().find(|b| match b {
            MergeBlock::Conflict { id, .. } => *id == conflict_id,
            _ => false,
        })
    }

    /// Resolves conflict `conflict_id` by taking Left.
    pub fn take_left(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(conflict_id, TakeResolution::Choice(TakeChoice::Left))
    }

    /// Resolves conflict `conflict_id` by taking Right.
    pub fn take_right(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(conflict_id, TakeResolution::Choice(TakeChoice::Right))
    }

    /// Resolves conflict `conflict_id` by taking Base.
    pub fn take_base(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(conflict_id, TakeResolution::Choice(TakeChoice::Base))
    }

    /// Resolves conflict `conflict_id` by taking Both with Left lines first.
    pub fn take_both_left_first(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(
            conflict_id,
            TakeResolution::Choice(TakeChoice::BothLeftFirst),
        )
    }

    /// Resolves conflict `conflict_id` by taking Both with Right lines first.
    pub fn take_both_right_first(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(
            conflict_id,
            TakeResolution::Choice(TakeChoice::BothRightFirst),
        )
    }

    /// Resolves conflict `conflict_id` with custom lines.
    pub fn take_custom(&mut self, conflict_id: usize, lines: Vec<String>) -> bool {
        self.resolve_conflict(conflict_id, TakeResolution::Custom(lines))
    }

    /// Reverts a conflict's resolution back to unresolved (`None`).
    pub fn clear_resolution(&mut self, conflict_id: usize) -> bool {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { id, resolution, .. } = block
                && *id == conflict_id
            {
                *resolution = None;
                return true;
            }
        }
        false
    }

    /// Bulk action: resolve all conflicts by taking Both (Left first).
    pub fn take_all_both_left_first(&mut self) {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { resolution, .. } = block {
                *resolution = Some(TakeResolution::Choice(TakeChoice::BothLeftFirst));
            }
        }
    }

    /// Bulk action: resolve all conflicts by taking Both (Right first).
    pub fn take_all_both_right_first(&mut self) {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { resolution, .. } = block {
                *resolution = Some(TakeResolution::Choice(TakeChoice::BothRightFirst));
            }
        }
    }

    /// Bulk action: clears all conflict resolutions, returning them to unresolved.
    pub fn clear_all_resolutions(&mut self) {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { resolution, .. } = block {
                *resolution = None;
            }
        }
    }

    /// Returns true if all conflict blocks have been resolved.
    pub fn is_fully_resolved(&self) -> bool {
        self.unresolved_count() == 0
    }

    /// Resolves the conflict block identified by `conflict_id`.
    /// Returns true if the conflict was found and updated.
    pub fn resolve_conflict(&mut self, conflict_id: usize, resolution: TakeResolution) -> bool {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict {
                id,
                resolution: res,
                ..
            } = block
                && *id == conflict_id
            {
                *res = Some(resolution);
                return true;
            }
        }
        false
    }

    /// Bulk action: resolve all conflicts by taking the left side.
    pub fn take_all_left(&mut self) {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { resolution, .. } = block {
                *resolution = Some(TakeResolution::Choice(TakeChoice::Left));
            }
        }
    }

    /// Bulk action: resolve all conflicts by taking the right side.
    pub fn take_all_right(&mut self) {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { resolution, .. } = block {
                *resolution = Some(TakeResolution::Choice(TakeChoice::Right));
            }
        }
    }

    /// Bulk action: resolve all conflicts by taking base.
    pub fn take_all_base(&mut self) {
        for block in &mut self.blocks {
            if let MergeBlock::Conflict { resolution, .. } = block {
                *resolution = Some(TakeResolution::Choice(TakeChoice::Base));
            }
        }
    }

    /// Renders the resulting lines.
    /// Unresolved conflicts are rendered with standard conflict markers.
    pub fn render_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for block in &self.blocks {
            match block {
                MergeBlock::Clean(lines) => {
                    out.extend(lines.iter().cloned());
                }
                MergeBlock::Conflict {
                    base,
                    left,
                    right,
                    resolution,
                    ..
                } => match resolution {
                    Some(TakeResolution::Choice(TakeChoice::Base)) => {
                        out.extend(base.iter().cloned());
                    }
                    Some(TakeResolution::Choice(TakeChoice::Left)) => {
                        out.extend(left.iter().cloned());
                    }
                    Some(TakeResolution::Choice(TakeChoice::Right)) => {
                        out.extend(right.iter().cloned());
                    }
                    Some(TakeResolution::Choice(TakeChoice::BothLeftFirst)) => {
                        out.extend(left.iter().cloned());
                        out.extend(right.iter().cloned());
                    }
                    Some(TakeResolution::Choice(TakeChoice::BothRightFirst)) => {
                        out.extend(right.iter().cloned());
                        out.extend(left.iter().cloned());
                    }
                    Some(TakeResolution::Custom(lines)) => {
                        out.extend(lines.iter().cloned());
                    }
                    None => {
                        out.push("<<<<<<< left".to_string());
                        out.extend(left.iter().cloned());
                        out.push("=======".to_string());
                        out.extend(right.iter().cloned());
                        out.push(">>>>>>> right".to_string());
                    }
                },
            }
        }
        out
    }

    /// Converts the rendered merge result into an editable [`TextBuffer`].
    pub fn into_buffer(self) -> TextBuffer {
        TextBuffer::from_lines(self.render_lines())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_buffer_edits_and_undo_redo() {
        let mut buf = TextBuffer::from_text("one\ntwo\nthree");
        assert_eq!(buf.len_lines(), 3);

        buf.insert_line(1, "inserted".to_string());
        assert_eq!(buf.lines(), &["one", "inserted", "two", "three"]);

        buf.replace_line(2, "two_mod".to_string());
        assert_eq!(buf.lines(), &["one", "inserted", "two_mod", "three"]);

        buf.delete_line(0);
        assert_eq!(buf.lines(), &["inserted", "two_mod", "three"]);

        assert!(buf.undo());
        assert_eq!(buf.lines(), &["one", "inserted", "two_mod", "three"]);

        assert!(buf.undo());
        assert_eq!(buf.lines(), &["one", "inserted", "two", "three"]);

        assert!(buf.undo());
        assert_eq!(buf.lines(), &["one", "two", "three"]);

        assert!(buf.redo());
        assert_eq!(buf.lines(), &["one", "inserted", "two", "three"]);
    }

    #[test]
    fn test_merge_session_clean_edits() {
        let base = ["line1", "line2", "line3"];
        let left = ["line1_mod", "line2", "line3"];
        let right = ["line1", "line2", "line3_mod"];

        let session = MergeSession::new(&base, &left, &right);
        assert_eq!(session.conflict_count(), 0);
        assert!(session.is_fully_resolved());
        assert_eq!(
            session.render_lines(),
            vec!["line1_mod", "line2", "line3_mod"]
        );
    }

    #[test]
    fn test_merge_session_conflict_resolution() {
        let base = ["common", "base_val", "tail"];
        let left = ["common", "left_val", "tail"];
        let right = ["common", "right_val", "tail"];

        let mut session = MergeSession::new(&base, &left, &right);
        assert_eq!(session.conflict_count(), 1);
        assert_eq!(session.unresolved_count(), 1);

        // Before resolution: contains conflict markers
        let unres = session.render_lines();
        assert!(unres.contains(&"<<<<<<< left".to_string()));

        // Resolve taking Left
        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::Left)));
        assert_eq!(session.unresolved_count(), 0);
        assert_eq!(session.render_lines(), vec!["common", "left_val", "tail"]);

        // Resolve taking Right
        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::Right)));
        assert_eq!(session.render_lines(), vec!["common", "right_val", "tail"]);

        // Resolve taking BothLeftFirst
        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::BothLeftFirst)));
        assert_eq!(
            session.render_lines(),
            vec!["common", "left_val", "right_val", "tail"]
        );

        // Resolve taking Base
        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::Base)));
        assert_eq!(session.render_lines(), vec!["common", "base_val", "tail"]);

        // Resolve taking BothRightFirst
        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::BothRightFirst)));
        assert_eq!(
            session.render_lines(),
            vec!["common", "right_val", "left_val", "tail"]
        );

        // Resolve custom
        assert!(
            session.resolve_conflict(0, TakeResolution::Custom(vec!["custom_val".to_string()]))
        );
        assert_eq!(session.render_lines(), vec!["common", "custom_val", "tail"]);
    }

    #[test]
    fn test_merge_session_bulk_take() {
        let base = ["a", "b"];
        let left = ["a_l", "b_l"];
        let right = ["a_r", "b_r"];

        let mut session = MergeSession::new(&base, &left, &right);
        session.take_all_left();
        assert!(session.is_fully_resolved());
        assert_eq!(session.render_lines(), vec!["a_l", "b_l"]);

        let mut session_r = MergeSession::new(&base, &left, &right);
        session_r.take_all_right();
        assert!(session_r.is_fully_resolved());
        assert_eq!(session_r.render_lines(), vec!["a_r", "b_r"]);

        let mut session_b = MergeSession::new(&base, &left, &right);
        session_b.take_all_base();
        assert!(session_b.is_fully_resolved());
        assert_eq!(session_b.render_lines(), vec!["a", "b"]);
    }

    #[test]
    fn test_text_buffer_splice() {
        let mut buf = TextBuffer::from_text("a\nb\nc");
        let deleted = buf.splice_lines(1, 1, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(deleted, vec!["b"]);
        assert_eq!(buf.lines(), &["a", "x", "y", "c"]);
        assert!(buf.undo());
        assert_eq!(buf.lines(), &["a", "b", "c"]);
    }

    #[test]
    fn test_merge_session_navigation_and_take_helpers() {
        let base = ["header", "conf1_base", "mid", "conf2_base", "footer"];
        let left = ["header", "conf1_left", "mid", "conf2_left", "footer"];
        let right = ["header", "conf1_right", "mid", "conf2_right", "footer"];

        let mut session = MergeSession::new(&base, &left, &right);
        assert_eq!(session.conflict_count(), 2);
        assert_eq!(session.unresolved_count(), 2);
        assert_eq!(session.resolved_count(), 0);
        assert_eq!(session.conflict_ids(), vec![0, 1]);
        assert_eq!(session.first_unresolved(), Some(0));
        assert_eq!(session.next_unresolved(0), Some(1));
        assert_eq!(session.next_unresolved(1), None);
        assert_eq!(session.prev_unresolved(1), Some(0));
        assert_eq!(session.prev_unresolved(0), None);

        // Take left on 0
        assert!(session.take_left(0));
        assert_eq!(session.resolved_count(), 1);
        assert_eq!(session.first_unresolved(), Some(1));

        // Take right on 1
        assert!(session.take_right(1));
        assert_eq!(session.resolved_count(), 2);
        assert!(session.is_fully_resolved());
        assert_eq!(
            session.render_lines(),
            vec!["header", "conf1_left", "mid", "conf2_right", "footer"]
        );

        // Clear resolution on 0
        assert!(session.clear_resolution(0));
        assert_eq!(session.unresolved_count(), 1);
        assert_eq!(session.first_unresolved(), Some(0));

        // Take both on 0
        assert!(session.take_both_left_first(0));
        assert_eq!(
            session.render_lines(),
            vec![
                "header",
                "conf1_left",
                "conf1_right",
                "mid",
                "conf2_right",
                "footer"
            ]
        );

        // Clear all and bulk take both right first
        session.take_all_both_right_first();
        assert_eq!(
            session.render_lines(),
            vec![
                "header",
                "conf1_right",
                "conf1_left",
                "mid",
                "conf2_right",
                "conf2_left",
                "footer"
            ]
        );

        session.clear_all_resolutions();
        assert_eq!(session.unresolved_count(), 2);
    }

    #[test]
    fn test_text_buffer_clear_and_append() {
        let mut buf = TextBuffer::from_text("one\ntwo");
        assert_eq!(buf.len_lines(), 2);
        buf.append_line("three".to_string());
        assert_eq!(buf.lines(), &["one", "two", "three"]);
        assert_eq!(buf.undo_count(), 1);

        buf.clear();
        assert!(buf.is_empty());
        assert!(buf.undo());
        assert_eq!(buf.lines(), &["one", "two", "three"]);
    }
}
