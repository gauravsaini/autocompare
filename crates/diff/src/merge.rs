//! Editable text buffer and 2-way / 3-way Take merge sessions.
//!
//! Layer 0: no panics, no unwrap/expect in non-test code, no unsafe.

use crate::diff_lines;
use crate::merge3::{Merge3Block, merge3_blocks, merge3_blocks_with_algorithm};
use crate::text_diff::DiffAlgorithm;

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
    Group(Vec<BufferAction>),
}

/// An in-memory editable text buffer organized by lines, supporting undo/redo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextBuffer {
    lines: Vec<String>,
    undo_stack: Vec<BufferAction>,
    redo_stack: Vec<BufferAction>,
    transaction_depth: usize,
    current_transaction: Vec<BufferAction>,
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
            transaction_depth: 0,
            current_transaction: Vec::new(),
            is_dirty: false,
        }
    }

    /// Creates a buffer directly from a vector of line strings.
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self {
            lines,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            transaction_depth: 0,
            current_transaction: Vec::new(),
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

    fn record_action(&mut self, action: BufferAction) {
        if self.transaction_depth > 0 {
            self.current_transaction.push(action);
        } else {
            self.undo_stack.push(action);
            self.redo_stack.clear();
        }
        self.is_dirty = true;
    }

    /// Begins an undo transaction grouping multiple edits into one undo step.
    pub fn begin_transaction(&mut self) {
        self.transaction_depth = self.transaction_depth.saturating_add(1);
    }

    /// Commits the active undo transaction.
    pub fn commit_transaction(&mut self) {
        if self.transaction_depth == 0 {
            return;
        }
        self.transaction_depth = self.transaction_depth.saturating_sub(1);
        if self.transaction_depth == 0 && !self.current_transaction.is_empty() {
            let actions = std::mem::take(&mut self.current_transaction);
            self.undo_stack.push(BufferAction::Group(actions));
            self.redo_stack.clear();
        }
    }

    /// Inserts a new line at 0-based `idx`. If `idx >= len`, appends to the end.
    pub fn insert_line(&mut self, idx: usize, content: String) {
        let insert_idx = idx.min(self.lines.len());
        self.lines.insert(insert_idx, content.clone());
        self.record_action(BufferAction::InsertLine {
            idx: insert_idx,
            content,
        });
    }

    /// Deletes a line at 0-based `idx`, returning the deleted content.
    pub fn delete_line(&mut self, idx: usize) -> Option<String> {
        if idx >= self.lines.len() {
            return None;
        }
        let removed = self.lines.remove(idx);
        self.record_action(BufferAction::DeleteLine {
            idx,
            content: removed.clone(),
        });
        Some(removed)
    }

    /// Replaces the line at 0-based `idx`, returning the previous content.
    pub fn replace_line(&mut self, idx: usize, content: String) -> Option<String> {
        let slot = self.lines.get_mut(idx)?;
        let old = std::mem::replace(slot, content.clone());
        self.record_action(BufferAction::ReplaceLine {
            idx,
            old: old.clone(),
            new: content,
        });
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

        self.record_action(BufferAction::SpliceLines {
            idx: start,
            deleted: deleted.clone(),
            inserted: new_lines,
        });
        deleted
    }

    /// Inserts text at a specific character position within a line.
    pub fn insert_text_at(&mut self, line_idx: usize, char_col: usize, text: &str) -> bool {
        if line_idx >= self.lines.len() {
            return false;
        }
        let current_line = match self.lines.get(line_idx) {
            Some(l) => l,
            None => return false,
        };
        let mut chars: Vec<char> = current_line.chars().collect();
        let col = char_col.min(chars.len());
        for (i, ch) in text.chars().enumerate() {
            chars.insert(col.saturating_add(i), ch);
        }
        let new_line: String = chars.into_iter().collect();
        self.replace_line(line_idx, new_line);
        true
    }

    /// Deletes `len` characters starting at `char_col` within a line.
    pub fn delete_text_at(&mut self, line_idx: usize, char_col: usize, len: usize) -> bool {
        if line_idx >= self.lines.len() {
            return false;
        }
        let current_line = match self.lines.get(line_idx) {
            Some(l) => l,
            None => return false,
        };
        let mut chars: Vec<char> = current_line.chars().collect();
        if char_col >= chars.len() {
            return false;
        }
        let end = (char_col.saturating_add(len)).min(chars.len());
        chars.drain(char_col..end);
        let new_line: String = chars.into_iter().collect();
        self.replace_line(line_idx, new_line);
        true
    }

    /// Searches the buffer for occurrences of `query`, returning (line_idx, char_col).
    pub fn find_text(&self, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        let target_query = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        for (line_idx, line) in self.lines.iter().enumerate() {
            let line_to_search = if case_sensitive {
                line.clone()
            } else {
                line.to_lowercase()
            };

            let mut start_byte = 0usize;
            while let Some(pos) = line_to_search
                .get(start_byte..)
                .and_then(|s| s.find(&target_query))
            {
                let actual_byte = start_byte.saturating_add(pos);
                let char_col = line.get(..actual_byte).map_or(0, |s| s.chars().count());
                results.push((line_idx, char_col));
                start_byte = actual_byte.saturating_add(target_query.len().max(1));
            }
        }
        results
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

        self.apply_undo_action(&action);
        self.redo_stack.push(action);
        self.is_dirty = true;
        true
    }

    fn apply_undo_action(&mut self, action: &BufferAction) {
        match action {
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
            BufferAction::Group(actions) => {
                for act in actions.iter().rev() {
                    self.apply_undo_action(act);
                }
            }
        }
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

        self.apply_redo_action(&action);
        self.undo_stack.push(action);
        self.is_dirty = true;
        true
    }

    fn apply_redo_action(&mut self, action: &BufferAction) {
        match action {
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
            BufferAction::Group(actions) => {
                for act in actions {
                    self.apply_redo_action(act);
                }
            }
        }
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
// Conflict Marker Styling
// ----------------------------------------------------------------------------

/// Formatting configuration for merge conflict markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictMarkerStyle {
    /// Left marker label (e.g. `<<<<<<< left` or `<<<<<<< THEIRS`).
    pub left_marker: String,
    /// Base marker label (e.g. `||||||| base`), if diff3 style is desired.
    pub base_marker: Option<String>,
    /// Separator marker label (e.g. `=======`).
    pub separator: String,
    /// Right marker label (e.g. `>>>>>>> right` or `>>>>>>> OURS`).
    pub right_marker: String,
}

impl Default for ConflictMarkerStyle {
    fn default() -> Self {
        Self::git_default()
    }
}

impl ConflictMarkerStyle {
    /// Standard Git 2-way / 3-way conflict markers.
    pub fn git_default() -> Self {
        Self {
            left_marker: "<<<<<<< left".to_string(),
            base_marker: None,
            separator: "=======".to_string(),
            right_marker: ">>>>>>> right".to_string(),
        }
    }

    /// Git diff3 style including the common ancestor base block.
    pub fn git_diff3() -> Self {
        Self {
            left_marker: "<<<<<<< left".to_string(),
            base_marker: Some("||||||| base".to_string()),
            separator: "=======".to_string(),
            right_marker: ">>>>>>> right".to_string(),
        }
    }

    /// Beyond Compare style conflict markers.
    pub fn beyond_compare() -> Self {
        Self {
            left_marker: "<<<<<<< Left".to_string(),
            base_marker: None,
            separator: "=======".to_string(),
            right_marker: ">>>>>>> Right".to_string(),
        }
    }

    /// Custom conflict marker labels.
    pub fn custom(left: &str, base: Option<&str>, sep: &str, right: &str) -> Self {
        Self {
            left_marker: left.to_string(),
            base_marker: base.map(|s| s.to_string()),
            separator: sep.to_string(),
            right_marker: right.to_string(),
        }
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

impl TakeChoice {
    /// BC-parity alias: Center is the ancestor/base side.
    /// `TakeChoice::Center` is exactly `Base`; no new behavior.
    #[allow(non_upper_case_globals)]
    pub const Center: Self = Self::Base;

    /// True for the center/ancestor side (`Base` / `Center`).
    pub fn is_center(self) -> bool {
        matches!(self, Self::Base)
    }
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

/// Location and metadata of a conflict chunk in the 3-way merge output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictChunkInfo {
    pub id: usize,
    pub output_line_start: usize,
    pub output_line_count: usize,
    pub is_resolved: bool,
}

/// An interactive 3-way merge session supporting Take resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSession {
    blocks: Vec<MergeBlock>,
}

impl MergeSession {
    /// Constructs a new 3-way merge session from base, left, and right lines.
    pub fn new(base: &[&str], left: &[&str], right: &[&str]) -> Self {
        // ODD Phase 6: orchestration span (line counts + conflict count).
        let odd_span = tracing::info_span!(
            "diff.merge_session",
            component = "rs.diff/merge",
            base_lines = base.len() as u64,
            left_lines = left.len() as u64,
            right_lines = right.len() as u64,
            conflicts = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        let out = Self::new_inner(base, left, right);
        odd_span.record("conflicts", out.conflict_count() as u64);
        out
    }

    fn new_inner(base: &[&str], left: &[&str], right: &[&str]) -> Self {
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

    /// Constructs a 3-way merge session with explicit diff algorithm
    /// (Myers minimal vs Patience readable). Headless; never panics.
    pub fn new_with_algorithm(
        base: &[&str],
        left: &[&str],
        right: &[&str],
        algorithm: DiffAlgorithm,
    ) -> Self {
        // ODD Phase 6: orchestration span (counts + algorithm, never contents).
        let algo_name = match algorithm {
            DiffAlgorithm::Myers => "myers",
            DiffAlgorithm::Patience => "patience",
        };
        let odd_span = tracing::info_span!(
            "diff.merge_session",
            component = "rs.diff/merge",
            algorithm = algo_name,
            base_lines = base.len() as u64,
            left_lines = left.len() as u64,
            right_lines = right.len() as u64,
            conflicts = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        let out = Self::new_with_algorithm_inner(base, left, right, algorithm);
        odd_span.record("conflicts", out.conflict_count() as u64);
        out
    }

    fn new_with_algorithm_inner(
        base: &[&str],
        left: &[&str],
        right: &[&str],
        algorithm: DiffAlgorithm,
    ) -> Self {
        let raw_blocks = merge3_blocks_with_algorithm(base, left, right, algorithm);
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

    /// ID of the first conflict, if any.
    pub fn first_conflict(&self) -> Option<usize> {
        self.conflict_ids().first().copied()
    }

    /// ID of the last conflict, if any.
    pub fn last_conflict(&self) -> Option<usize> {
        self.conflict_ids().last().copied()
    }

    /// ID of the next conflict after `from_id`.
    pub fn next_conflict(&self, from_id: usize) -> Option<usize> {
        self.conflict_ids().into_iter().find(|&id| id > from_id)
    }

    /// ID of the previous conflict before `from_id`.
    pub fn prev_conflict(&self, from_id: usize) -> Option<usize> {
        self.conflict_ids()
            .into_iter()
            .rev()
            .find(|&id| id < from_id)
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

    /// Resolves conflict `conflict_id` by taking Center (ancestor/base, BC parity).
    pub fn take_center(&mut self, conflict_id: usize) -> bool {
        self.take_base(conflict_id)
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

    /// Bulk action: resolve all conflicts by taking center (BC parity for base).
    pub fn take_all_center(&mut self) {
        self.take_all_base();
    }

    /// Renders the resulting lines using standard default conflict markers.
    pub fn render_lines(&self) -> Vec<String> {
        self.render_lines_with_style(&ConflictMarkerStyle::git_default())
    }

    /// Renders the resulting lines with custom conflict marker styling (e.g. diff3, beyond compare).
    pub fn render_lines_with_style(&self, style: &ConflictMarkerStyle) -> Vec<String> {
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
                        out.push(style.left_marker.clone());
                        out.extend(left.iter().cloned());
                        if let Some(ref base_lbl) = style.base_marker {
                            out.push(base_lbl.clone());
                            out.extend(base.iter().cloned());
                        }
                        out.push(style.separator.clone());
                        out.extend(right.iter().cloned());
                        out.push(style.right_marker.clone());
                    }
                },
            }
        }
        out
    }

    /// Computes metadata and output line ranges for all conflict chunks.
    pub fn conflict_chunks(&self) -> Vec<ConflictChunkInfo> {
        let mut chunks = Vec::new();
        let mut line_cursor = 0usize;

        for block in &self.blocks {
            match block {
                MergeBlock::Clean(lines) => {
                    line_cursor = line_cursor.saturating_add(lines.len());
                }
                MergeBlock::Conflict {
                    id,
                    base,
                    left,
                    right,
                    resolution,
                } => {
                    let is_resolved = resolution.is_some();
                    let count = match resolution {
                        Some(TakeResolution::Choice(TakeChoice::Base)) => base.len(),
                        Some(TakeResolution::Choice(TakeChoice::Left)) => left.len(),
                        Some(TakeResolution::Choice(TakeChoice::Right)) => right.len(),
                        Some(TakeResolution::Choice(
                            TakeChoice::BothLeftFirst | TakeChoice::BothRightFirst,
                        )) => left.len().saturating_add(right.len()),
                        Some(TakeResolution::Custom(lines)) => lines.len(),
                        None => {
                            // Markers: left_marker, left lines, separator, right lines, right_marker
                            left.len().saturating_add(right.len()).saturating_add(3)
                        }
                    };

                    chunks.push(ConflictChunkInfo {
                        id: *id,
                        output_line_start: line_cursor,
                        output_line_count: count,
                        is_resolved,
                    });
                    line_cursor = line_cursor.saturating_add(count);
                }
            }
        }

        chunks
    }

    /// Converts the rendered merge result into an editable [`TextBuffer`].
    pub fn into_buffer(self) -> TextBuffer {
        TextBuffer::from_lines(self.render_lines())
    }
}

// ----------------------------------------------------------------------------
// 2-Way Text Merge
// ----------------------------------------------------------------------------

/// Choice of how to resolve a 2-way merge conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Take2Choice {
    /// Take the left contributor lines.
    Left,
    /// Take the right contributor lines.
    Right,
    /// Take both, with left lines appearing before right lines.
    BothLeftFirst,
    /// Take both, with right lines appearing before left lines.
    BothRightFirst,
}

/// Resolution state for a 2-way merge conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Take2Resolution {
    /// Resolved via choice (Left, Right, Both).
    Choice(Take2Choice),
    /// Resolved via arbitrary custom lines.
    Custom(Vec<String>),
}

/// A block within a 2-way merge session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merge2Block {
    /// Non-conflicting matching lines.
    Clean(Vec<String>),
    /// Differing lines between left and right.
    Conflict {
        id: usize,
        left: Vec<String>,
        right: Vec<String>,
        resolution: Option<Take2Resolution>,
    },
}

/// Location and metadata of a conflict chunk in the 2-way merge output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictChunk2Info {
    pub id: usize,
    pub output_line_start: usize,
    pub output_line_count: usize,
    pub is_resolved: bool,
}

/// An interactive 2-way merge session supporting Take resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merge2Session {
    blocks: Vec<Merge2Block>,
}

impl Merge2Session {
    /// Constructs a new 2-way merge session from left and right lines.
    pub fn new(left: &[&str], right: &[&str]) -> Self {
        let script = diff_lines(left, right);
        let mut blocks = Vec::new();
        let mut conflict_id = 0usize;
        let mut l_pos = 0usize;
        let mut r_pos = 0usize;
        let mut idx = 0usize;

        while let Some(op) = script.ops.get(idx) {
            match op {
                crate::Op::Equal { len } => {
                    let mut lines = Vec::with_capacity(*len);
                    for i in 0..*len {
                        let line = left.get(l_pos.saturating_add(i)).copied().unwrap_or("");
                        lines.push(line.to_string());
                    }
                    blocks.push(Merge2Block::Clean(lines));
                    l_pos = l_pos.saturating_add(*len);
                    r_pos = r_pos.saturating_add(*len);
                }
                crate::Op::Delete { len } => {
                    let mut l_lines = Vec::with_capacity(*len);
                    for i in 0..*len {
                        let line = left.get(l_pos.saturating_add(i)).copied().unwrap_or("");
                        l_lines.push(line.to_string());
                    }
                    l_pos = l_pos.saturating_add(*len);

                    if let Some(crate::Op::Insert { len: ins }) =
                        script.ops.get(idx.saturating_add(1))
                    {
                        let mut r_lines = Vec::with_capacity(*ins);
                        for i in 0..*ins {
                            let line = right.get(r_pos.saturating_add(i)).copied().unwrap_or("");
                            r_lines.push(line.to_string());
                        }
                        r_pos = r_pos.saturating_add(*ins);
                        blocks.push(Merge2Block::Conflict {
                            id: conflict_id,
                            left: l_lines,
                            right: r_lines,
                            resolution: None,
                        });
                        conflict_id += 1;
                        idx += 1;
                    } else {
                        blocks.push(Merge2Block::Conflict {
                            id: conflict_id,
                            left: l_lines,
                            right: Vec::new(),
                            resolution: None,
                        });
                        conflict_id += 1;
                    }
                }
                crate::Op::Insert { len } => {
                    let mut r_lines = Vec::with_capacity(*len);
                    for i in 0..*len {
                        let line = right.get(r_pos.saturating_add(i)).copied().unwrap_or("");
                        r_lines.push(line.to_string());
                    }
                    r_pos = r_pos.saturating_add(*len);
                    blocks.push(Merge2Block::Conflict {
                        id: conflict_id,
                        left: Vec::new(),
                        right: r_lines,
                        resolution: None,
                    });
                    conflict_id += 1;
                }
            }
            idx += 1;
        }

        Self { blocks }
    }

    /// Returns all merge blocks in order.
    pub fn blocks(&self) -> &[Merge2Block] {
        &self.blocks
    }

    /// Total number of conflict blocks.
    pub fn conflict_count(&self) -> usize {
        self.blocks
            .iter()
            .filter(|b| matches!(b, Merge2Block::Conflict { .. }))
            .count()
    }

    /// Number of conflict blocks that have not yet been resolved.
    pub fn unresolved_count(&self) -> usize {
        self.blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    Merge2Block::Conflict {
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
                Merge2Block::Conflict { id, .. } => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// ID of the first unresolved conflict block, if any.
    pub fn first_unresolved(&self) -> Option<usize> {
        self.blocks.iter().find_map(|b| match b {
            Merge2Block::Conflict {
                id,
                resolution: None,
                ..
            } => Some(*id),
            _ => None,
        })
    }

    /// Resolves conflict `conflict_id` by taking Left.
    pub fn take_left(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(conflict_id, Take2Resolution::Choice(Take2Choice::Left))
    }

    /// Resolves conflict `conflict_id` by taking Right.
    pub fn take_right(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(conflict_id, Take2Resolution::Choice(Take2Choice::Right))
    }

    /// Resolves conflict `conflict_id` by taking Both (Left first).
    pub fn take_both_left_first(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(
            conflict_id,
            Take2Resolution::Choice(Take2Choice::BothLeftFirst),
        )
    }

    /// Resolves conflict `conflict_id` by taking Both (Right first).
    pub fn take_both_right_first(&mut self, conflict_id: usize) -> bool {
        self.resolve_conflict(
            conflict_id,
            Take2Resolution::Choice(Take2Choice::BothRightFirst),
        )
    }

    /// Resolves conflict `conflict_id` with custom lines.
    pub fn take_custom(&mut self, conflict_id: usize, lines: Vec<String>) -> bool {
        self.resolve_conflict(conflict_id, Take2Resolution::Custom(lines))
    }

    /// Clears a conflict's resolution.
    pub fn clear_resolution(&mut self, conflict_id: usize) -> bool {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict { id, resolution, .. } = block
                && *id == conflict_id
            {
                *resolution = None;
                return true;
            }
        }
        false
    }

    /// Bulk action: take left on all conflicts.
    pub fn take_all_left(&mut self) {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict { resolution, .. } = block {
                *resolution = Some(Take2Resolution::Choice(Take2Choice::Left));
            }
        }
    }

    /// Bulk action: take right on all conflicts.
    pub fn take_all_right(&mut self) {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict { resolution, .. } = block {
                *resolution = Some(Take2Resolution::Choice(Take2Choice::Right));
            }
        }
    }

    /// Bulk action: take both (left first) on all conflicts.
    pub fn take_all_both_left_first(&mut self) {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict { resolution, .. } = block {
                *resolution = Some(Take2Resolution::Choice(Take2Choice::BothLeftFirst));
            }
        }
    }

    /// Bulk action: take both (right first) on all conflicts.
    pub fn take_all_both_right_first(&mut self) {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict { resolution, .. } = block {
                *resolution = Some(Take2Resolution::Choice(Take2Choice::BothRightFirst));
            }
        }
    }

    /// Clears all conflict resolutions.
    pub fn clear_all_resolutions(&mut self) {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict { resolution, .. } = block {
                *resolution = None;
            }
        }
    }

    /// Returns true if all conflicts are resolved.
    pub fn is_fully_resolved(&self) -> bool {
        self.unresolved_count() == 0
    }

    /// Resolves conflict `conflict_id`.
    pub fn resolve_conflict(&mut self, conflict_id: usize, resolution: Take2Resolution) -> bool {
        for block in &mut self.blocks {
            if let Merge2Block::Conflict {
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

    /// Renders the resulting lines using default conflict markers.
    pub fn render_lines(&self) -> Vec<String> {
        self.render_lines_with_style(&ConflictMarkerStyle::git_default())
    }

    /// Renders the resulting lines with custom conflict marker styling.
    pub fn render_lines_with_style(&self, style: &ConflictMarkerStyle) -> Vec<String> {
        let mut out = Vec::new();
        for block in &self.blocks {
            match block {
                Merge2Block::Clean(lines) => {
                    out.extend(lines.iter().cloned());
                }
                Merge2Block::Conflict {
                    left,
                    right,
                    resolution,
                    ..
                } => match resolution {
                    Some(Take2Resolution::Choice(Take2Choice::Left)) => {
                        out.extend(left.iter().cloned());
                    }
                    Some(Take2Resolution::Choice(Take2Choice::Right)) => {
                        out.extend(right.iter().cloned());
                    }
                    Some(Take2Resolution::Choice(Take2Choice::BothLeftFirst)) => {
                        out.extend(left.iter().cloned());
                        out.extend(right.iter().cloned());
                    }
                    Some(Take2Resolution::Choice(Take2Choice::BothRightFirst)) => {
                        out.extend(right.iter().cloned());
                        out.extend(left.iter().cloned());
                    }
                    Some(Take2Resolution::Custom(lines)) => {
                        out.extend(lines.iter().cloned());
                    }
                    None => {
                        out.push(style.left_marker.clone());
                        out.extend(left.iter().cloned());
                        out.push(style.separator.clone());
                        out.extend(right.iter().cloned());
                        out.push(style.right_marker.clone());
                    }
                },
            }
        }
        out
    }

    /// Computes metadata and output line ranges for all 2-way conflict chunks.
    pub fn conflict_chunks(&self) -> Vec<ConflictChunk2Info> {
        let mut chunks = Vec::new();
        let mut line_cursor = 0usize;

        for block in &self.blocks {
            match block {
                Merge2Block::Clean(lines) => {
                    line_cursor = line_cursor.saturating_add(lines.len());
                }
                Merge2Block::Conflict {
                    id,
                    left,
                    right,
                    resolution,
                } => {
                    let is_resolved = resolution.is_some();
                    let count = match resolution {
                        Some(Take2Resolution::Choice(Take2Choice::Left)) => left.len(),
                        Some(Take2Resolution::Choice(Take2Choice::Right)) => right.len(),
                        Some(Take2Resolution::Choice(
                            Take2Choice::BothLeftFirst | Take2Choice::BothRightFirst,
                        )) => left.len().saturating_add(right.len()),
                        Some(Take2Resolution::Custom(lines)) => lines.len(),
                        None => left.len().saturating_add(right.len()).saturating_add(3),
                    };

                    chunks.push(ConflictChunk2Info {
                        id: *id,
                        output_line_start: line_cursor,
                        output_line_count: count,
                        is_resolved,
                    });
                    line_cursor = line_cursor.saturating_add(count);
                }
            }
        }

        chunks
    }

    /// Converts the rendered merge result into an editable [`TextBuffer`].
    pub fn into_buffer(self) -> TextBuffer {
        TextBuffer::from_lines(self.render_lines())
    }
}

// ----------------------------------------------------------------------------
// Conflict Marker Parsing
// ----------------------------------------------------------------------------

/// Parses lines containing conflict markers into structured [`MergeBlock`] items.
pub fn parse_conflict_markers(lines: &[&str]) -> Vec<MergeBlock> {
    let mut blocks = Vec::new();
    let mut clean_accum = Vec::new();
    let mut conflict_id = 0usize;
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines.get(i).copied().unwrap_or("");
        if line.starts_with("<<<<<<<") {
            if !clean_accum.is_empty() {
                blocks.push(MergeBlock::Clean(std::mem::take(&mut clean_accum)));
            }

            let mut left_lines = Vec::new();
            let mut base_lines = Vec::new();
            let mut right_lines = Vec::new();
            let mut in_base = false;
            let mut in_right = false;
            i += 1;

            while i < lines.len() {
                let cline = lines.get(i).copied().unwrap_or("");
                if cline.starts_with("|||||||") {
                    in_base = true;
                } else if cline.starts_with("=======") {
                    in_base = false;
                    in_right = true;
                } else if cline.starts_with(">>>>>>>") {
                    break;
                } else if in_right {
                    right_lines.push(cline.to_string());
                } else if in_base {
                    base_lines.push(cline.to_string());
                } else {
                    left_lines.push(cline.to_string());
                }
                i += 1;
            }

            blocks.push(MergeBlock::Conflict {
                id: conflict_id,
                base: base_lines,
                left: left_lines,
                right: right_lines,
                resolution: None,
            });
            conflict_id += 1;
        } else {
            clean_accum.push(line.to_string());
        }
        i += 1;
    }

    if !clean_accum.is_empty() {
        blocks.push(MergeBlock::Clean(clean_accum));
    }

    blocks
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
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
    fn test_text_buffer_transaction() {
        let mut buf = TextBuffer::from_text("first\nsecond");
        buf.begin_transaction();
        buf.insert_line(1, "a".to_string());
        buf.insert_line(2, "b".to_string());
        buf.commit_transaction();

        assert_eq!(buf.lines(), &["first", "a", "b", "second"]);
        assert_eq!(buf.undo_count(), 1);

        assert!(buf.undo());
        assert_eq!(buf.lines(), &["first", "second"]);

        assert!(buf.redo());
        assert_eq!(buf.lines(), &["first", "a", "b", "second"]);
    }

    #[test]
    fn test_text_buffer_find_and_edit_at() {
        let mut buf = TextBuffer::from_text("Hello World\nAnother hello");
        let matches = buf.find_text("hello", false);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches.first(), Some(&(0, 0)));
        assert_eq!(matches.get(1), Some(&(1, 8)));

        assert!(buf.insert_text_at(0, 5, ", Amazing"));
        assert_eq!(buf.line(0), Some("Hello, Amazing World"));

        assert!(buf.delete_text_at(0, 5, 9));
        assert_eq!(buf.line(0), Some("Hello World"));
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

        let unres = session.render_lines();
        assert!(unres.contains(&"<<<<<<< left".to_string()));

        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::Left)));
        assert_eq!(session.unresolved_count(), 0);
        assert_eq!(session.render_lines(), vec!["common", "left_val", "tail"]);

        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::Right)));
        assert_eq!(session.render_lines(), vec!["common", "right_val", "tail"]);

        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::BothLeftFirst)));
        assert_eq!(
            session.render_lines(),
            vec!["common", "left_val", "right_val", "tail"]
        );

        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::Base)));
        assert_eq!(session.render_lines(), vec!["common", "base_val", "tail"]);

        assert!(session.resolve_conflict(0, TakeResolution::Choice(TakeChoice::BothRightFirst)));
        assert_eq!(
            session.render_lines(),
            vec!["common", "right_val", "left_val", "tail"]
        );

        assert!(
            session.resolve_conflict(0, TakeResolution::Custom(vec!["custom_val".to_string()]))
        );
        assert_eq!(session.render_lines(), vec!["common", "custom_val", "tail"]);
    }

    #[test]
    fn test_merge_session_diff3_marker_style() {
        let base = ["base_val"];
        let left = ["left_val"];
        let right = ["right_val"];

        let session = MergeSession::new(&base, &left, &right);
        let style = ConflictMarkerStyle::git_diff3();
        let rendered = session.render_lines_with_style(&style);

        assert_eq!(
            rendered,
            vec![
                "<<<<<<< left",
                "left_val",
                "||||||| base",
                "base_val",
                "=======",
                "right_val",
                ">>>>>>> right"
            ]
        );
    }

    #[test]
    fn test_merge2_session() {
        let left = ["common1", "left_only", "common2"];
        let right = ["common1", "right_only", "common2"];

        let mut session = Merge2Session::new(&left, &right);
        assert_eq!(session.conflict_count(), 1);
        assert_eq!(session.unresolved_count(), 1);

        assert!(session.take_left(0));
        assert_eq!(
            session.render_lines(),
            vec!["common1", "left_only", "common2"]
        );

        assert!(session.take_right(0));
        assert_eq!(
            session.render_lines(),
            vec!["common1", "right_only", "common2"]
        );

        assert!(session.take_both_left_first(0));
        assert_eq!(
            session.render_lines(),
            vec!["common1", "left_only", "right_only", "common2"]
        );
    }

    #[test]
    fn test_conflict_marker_parsing_roundtrip() {
        let lines = [
            "header",
            "<<<<<<< left",
            "my_left",
            "||||||| base",
            "my_base",
            "=======",
            "my_right",
            ">>>>>>> right",
            "footer",
        ];
        let blocks = parse_conflict_markers(&lines);
        assert_eq!(blocks.len(), 3); // Clean(header), Conflict, Clean(footer)
        if let Some(MergeBlock::Conflict {
            base, left, right, ..
        }) = blocks.get(1)
        {
            assert_eq!(base, &["my_base"]);
            assert_eq!(left, &["my_left"]);
            assert_eq!(right, &["my_right"]);
        } else {
            panic!("Expected conflict block");
        }
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

        assert!(session.take_left(0));
        assert_eq!(session.resolved_count(), 1);
        assert_eq!(session.first_unresolved(), Some(1));

        assert!(session.take_right(1));
        assert_eq!(session.resolved_count(), 2);
        assert!(session.is_fully_resolved());
        assert_eq!(
            session.render_lines(),
            vec!["header", "conf1_left", "mid", "conf2_right", "footer"]
        );

        assert!(session.clear_resolution(0));
        assert_eq!(session.unresolved_count(), 1);
        assert_eq!(session.first_unresolved(), Some(0));

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

    #[test]
    fn test_merge_take_edge_empty_base_and_chunks() {
        let base: [&str; 0] = [];
        let left = ["only"];
        let right = ["only"];
        let session = MergeSession::new(&base, &left, &right);
        assert_eq!(session.conflict_count(), 0);
        assert!(session.is_fully_resolved());
        assert_eq!(session.render_lines(), vec!["only"]);

        let left_b = ["a"];
        let right_b = ["b"];
        let mut divergent = MergeSession::new(&base, &left_b, &right_b);
        assert_eq!(divergent.conflict_count(), 1);
        assert_eq!(divergent.unresolved_count(), 1);
        let chunks = divergent.conflict_chunks();
        assert_eq!(chunks.len(), 1);
        if let Some(chunk) = chunks.first() {
            assert_eq!(chunk.output_line_start, 0);
            assert_eq!(chunk.output_line_count, 5);
            assert!(!chunk.is_resolved);
        } else {
            panic!("expected one conflict chunk");
        }

        assert!(divergent.take_both_left_first(0));
        assert_eq!(divergent.render_lines(), vec!["a", "b"]);
        assert!(divergent.is_fully_resolved());

        assert!(divergent.take_both_right_first(0));
        assert_eq!(divergent.render_lines(), vec!["b", "a"]);

        assert!(divergent.take_custom(0, Vec::new()));
        assert_eq!(divergent.render_lines(), Vec::<String>::new());

        assert!(divergent.clear_resolution(0));
        assert_eq!(divergent.unresolved_count(), 1);

        let base2 = ["x"];
        let empty: [&str; 0] = [];
        let both_deleted = MergeSession::new(&base2, &empty, &empty);
        assert_eq!(both_deleted.conflict_count(), 0);
        assert_eq!(both_deleted.render_lines(), Vec::<String>::new());
    }

    #[test]
    fn test_take_center_alias_and_algorithm_parity() {
        assert_eq!(TakeChoice::Center, TakeChoice::Base);
        assert!(TakeChoice::Center.is_center());
        assert!(!TakeChoice::Left.is_center());
        let base = ["common", "base_val", "tail"];
        let left = ["common", "left_val", "tail"];
        let right = ["common", "right_val", "tail"];
        for algo in [
            crate::text_diff::DiffAlgorithm::Myers,
            crate::text_diff::DiffAlgorithm::Patience,
        ] {
            let mut session = MergeSession::new_with_algorithm(&base, &left, &right, algo);
            assert_eq!(session.conflict_count(), 1);
            assert!(session.take_center(0));
            assert_eq!(session.render_lines(), vec!["common", "base_val", "tail"]);
            let mut centered = MergeSession::new_with_algorithm(&base, &left, &right, algo);
            centered.take_all_center();
            assert!(centered.is_fully_resolved());
            assert_eq!(centered.render_lines(), vec!["common", "base_val", "tail"]);
        }
    }
}
