//! In-place live editor copy mutations.
//!
//! Provides pure mutations for transferring text rows Left-to-Right or Right-to-Left.

#![forbid(unsafe_code)]

use crate::viewmodel::{TextRowKind, TextRows};

/// Direction for copying text lines between panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyDirection {
    LeftToRight,
    RightToLeft,
}

/// Error type when mutating diff rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyError {
    IndexOutOfBounds,
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexOutOfBounds => write!(f, "row index out of bounds"),
        }
    }
}

impl std::error::Error for CopyError {}

/// Applies a copy mutation on a specific text row in the model.
///
/// If copying LeftToRight: sets right text to left text, sets kind to Equal, clears accents.
/// If copying RightToLeft: sets left text to right text, sets kind to Equal, clears accents.
pub fn apply_copy(
    model: &mut TextRows,
    row_idx: usize,
    dir: CopyDirection,
) -> Result<(), CopyError> {
    let Some(row) = model.rows.get_mut(row_idx) else {
        return Err(CopyError::IndexOutOfBounds);
    };

    match dir {
        CopyDirection::LeftToRight => {
            row.right = row.left.clone();
            row.right_no = row.left_no;
            row.kind = TextRowKind::Equal;
            row.accent_a.clear();
            row.accent_b.clear();
        }
        CopyDirection::RightToLeft => {
            row.left = row.right.clone();
            row.left_no = row.right_no;
            row.kind = TextRowKind::Equal;
            row.accent_a.clear();
            row.accent_b.clear();
        }
    }
    Ok(())
}

/// Copies a single line: thin wrapper over [`apply_copy`] so the UI has
/// one named entry point per action and no duplicated mutation logic.
pub fn copy_line(
    model: &mut TextRows,
    row_idx: usize,
    dir: CopyDirection,
) -> Result<(), CopyError> {
    apply_copy(model, row_idx, dir)
}

/// Copies the whole contiguous difference section containing `row_idx`.
/// Reuses [`apply_copy`] per row, so section and line copies can never
/// drift apart. Returns how many rows were copied. An equal row is a
/// no-op `Ok(0)`; only an out-of-bounds index is an error.
pub fn copy_section(
    model: &mut TextRows,
    row_idx: usize,
    dir: CopyDirection,
) -> Result<usize, CopyError> {
    if model.rows.get(row_idx).is_none() {
        return Err(CopyError::IndexOutOfBounds);
    }
    let Some((start, end)) = crate::viewmodel::section_bounds(&model.rows, row_idx) else {
        return Ok(0);
    };
    let mut copied = 0usize;
    let mut idx = start;
    while idx < end {
        // `apply_copy` cannot fail here: bounds come from the model.
        if apply_copy(model, idx, dir).is_ok() {
            copied = copied.saturating_add(1);
        }
        idx = idx.saturating_add(1);
    }
    Ok(copied)
}

/// Copies all difference rows in the model in the given direction.
///
/// Skips Equal rows, applies [`apply_copy`] to all non-equal rows.
/// Returns how many rows were copied.
pub fn copy_all(model: &mut TextRows, dir: CopyDirection) -> Result<usize, CopyError> {
    let mut copied = 0usize;
    let mut idx = 0usize;
    while idx < model.rows.len() {
        if let Some(row) = model.rows.get(idx)
            && row.kind != TextRowKind::Equal
            && apply_copy(model, idx, dir).is_ok()
        {
            copied = copied.saturating_add(1);
        }
        idx = idx.saturating_add(1);
    }
    Ok(copied)
}

/// Counts how many difference rows can be copied in the model.
pub fn copyable_diff_count(model: &TextRows) -> usize {
    let mut count = 0usize;
    for row in &model.rows {
        if row.kind != TextRowKind::Equal {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Returns whether the specified row index is a copyable difference row.
pub fn can_copy_row(model: &TextRows, row_idx: usize) -> bool {
    match model.rows.get(row_idx) {
        Some(row) => row.kind != TextRowKind::Equal,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewmodel::TextRow;

    #[test]
    fn apply_copy_left_to_right() {
        let mut model = TextRows {
            rows: vec![TextRow {
                kind: TextRowKind::Deleted,
                left_no: Some(1),
                right_no: None,
                left: "alpha".to_string(),
                right: String::new(),
                accent_a: vec![(0, 2)],
                accent_b: Vec::new(),
            }],
            truncated: false,
            coarse: false,
            pane_chars: 10,
            gutter_chars: 4,
            selected: None,
            filter: crate::viewmodel::TextFilter::All,
        };

        apply_copy(&mut model, 0, CopyDirection::LeftToRight).unwrap_or_default();
        let Some(row) = model.rows.first() else {
            return;
        };
        assert_eq!(row.kind, TextRowKind::Equal);
        assert_eq!(row.left, "alpha");
        assert_eq!(row.right, "alpha");
        assert_eq!(row.right_no, Some(1));
        assert!(row.accent_a.is_empty());
    }

    #[test]
    fn apply_copy_out_of_bounds_returns_error() {
        let mut model = TextRows::default();
        let err = apply_copy(&mut model, 10, CopyDirection::RightToLeft);
        assert_eq!(err, Err(CopyError::IndexOutOfBounds));
    }

    fn section_model() -> TextRows {
        TextRows {
            rows: vec![
                TextRow {
                    kind: TextRowKind::Equal,
                    left: "same".to_string(),
                    right: "same".to_string(),
                    left_no: Some(1),
                    right_no: Some(1),
                    ..Default::default()
                },
                TextRow {
                    kind: TextRowKind::Changed,
                    left: "a".to_string(),
                    right: "b".to_string(),
                    left_no: Some(2),
                    right_no: Some(2),
                    ..Default::default()
                },
                TextRow {
                    kind: TextRowKind::Changed,
                    left: "c".to_string(),
                    right: "d".to_string(),
                    left_no: Some(3),
                    right_no: Some(3),
                    ..Default::default()
                },
            ],
            truncated: false,
            coarse: false,
            pane_chars: 4,
            gutter_chars: 4,
            selected: None,
            filter: crate::viewmodel::TextFilter::All,
        }
    }

    #[test]
    fn copy_line_reuses_single_row_mutation() {
        let mut model = section_model();
        let res = copy_line(&mut model, 1, CopyDirection::LeftToRight);
        assert!(res.is_ok());
        let second = model.rows.get(1).cloned().unwrap_or_default();
        assert_eq!(second.kind, TextRowKind::Equal);
        assert_eq!(second.right, "a");
        // Neighbour hunk row untouched: line scope only.
        let third = model.rows.get(2).cloned().unwrap_or_default();
        assert_eq!(third.kind, TextRowKind::Changed);
    }

    #[test]
    fn copy_section_copies_whole_hunk_and_reports_count() {
        let mut model = section_model();
        let count = copy_section(&mut model, 1, CopyDirection::LeftToRight).unwrap_or_default();
        assert_eq!(count, 2);
        for idx in [1usize, 2usize] {
            let row = model.rows.get(idx).cloned().unwrap_or_default();
            assert_eq!(row.kind, TextRowKind::Equal);
            assert_eq!(row.right, row.left);
        }
        // Equal row section is a no-op zero.
        let mut model2 = section_model();
        let zero = copy_section(&mut model2, 0, CopyDirection::LeftToRight).unwrap_or_default();
        assert_eq!(zero, 0);
        // Out of bounds stays an error.
        let err = copy_section(&mut model2, 99, CopyDirection::LeftToRight);
        assert_eq!(err, Err(CopyError::IndexOutOfBounds));
    }

    #[test]
    fn copy_all_copies_every_diff_row_and_preserves_equals() {
        let mut model = section_model();
        assert_eq!(copyable_diff_count(&model), 2);
        let count = copy_all(&mut model, CopyDirection::LeftToRight).unwrap_or_default();
        assert_eq!(count, 2);
        assert_eq!(copyable_diff_count(&model), 0);
        for row in &model.rows {
            assert_eq!(row.kind, TextRowKind::Equal);
            assert_eq!(row.left, row.right);
        }
    }

    #[test]
    fn can_copy_row_identifies_diffs_and_bounds() {
        let model = section_model();
        assert!(!can_copy_row(&model, 0)); // Equal
        assert!(can_copy_row(&model, 1)); // Changed
        assert!(can_copy_row(&model, 2)); // Changed
        assert!(!can_copy_row(&model, 3)); // Equal
        assert!(!can_copy_row(&model, 999)); // OOB
    }
}
