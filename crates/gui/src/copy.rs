//! In-place live editor copy mutations.
//!
//! Provides pure mutations for transferring text rows Left-to-Right or Right-to-Left.

#![forbid(unsafe_code)]
#![allow(dead_code)]

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
}
