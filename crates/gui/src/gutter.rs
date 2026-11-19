//! Thumbnail gutter geometry calculations.
//!
//! Pure mapping of diff row ranges to vertical gutter bar segments.
//! Bar widths, minimum segment height and merge gap come from
//! `crate::theme` spacing tokens (4/8/12/16/24 scale) so markers,
//! separator and text rows stay aligned.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use crate::viewmodel::{TextRow, TextRowKind};

/// A vertical span in the thumbnail gutter representing a difference hunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GutterSegment {
    /// Top y-coordinate in logical pixels.
    pub y_start: f32,
    /// Bottom y-coordinate in logical pixels.
    pub y_end: f32,
    /// Kind of difference.
    pub kind: TextRowKind,
}

/// Pure geometric layout of the thumbnail gutter.
#[derive(Debug, Clone, PartialEq)]
pub struct GutterGeometry {
    /// Total viewport height for the gutter.
    pub total_height: f32,
    /// Individual colored bar segments.
    pub segments: Vec<GutterSegment>,
}

/// Pure function computing thumbnail gutter bar geometry for a list of rows.
///
/// Maps diff rows onto vertical screen space `[0.0, height]`.
/// Consecutive rows of the same diff kind are merged when possible.
pub fn gutter_geometry(total_rows: usize, height: f32, rows: &[TextRow]) -> GutterGeometry {
    if total_rows == 0 || height <= 0.0 {
        return GutterGeometry {
            total_height: height.max(0.0),
            segments: Vec::new(),
        };
    }

    let mut segments = Vec::new();
    let total_f = total_rows as f32;

    for (idx, row) in rows.iter().enumerate() {
        if idx >= total_rows {
            break;
        }
        if row.kind == TextRowKind::Equal {
            continue;
        }

        let y_start = (idx as f32 / total_f) * height;
        let mut y_end = ((idx + 1) as f32 / total_f) * height;
        if y_end < y_start + crate::theme::GUTTER_MIN_SEGMENT_H {
            y_end = (y_start + crate::theme::GUTTER_MIN_SEGMENT_H).min(height);
        }

        if let Some(last) = segments.last_mut() {
            let last_seg: &mut GutterSegment = last;
            if last_seg.kind == row.kind
                && y_start <= last_seg.y_end + crate::theme::GUTTER_MERGE_GAP
            {
                last_seg.y_end = last_seg.y_end.max(y_end);
                continue;
            }
        }

        segments.push(GutterSegment {
            y_start,
            y_end,
            kind: row.kind,
        });
    }

    GutterGeometry {
        total_height: height,
        segments,
    }
}

/// Width of the thumbnail gutter bar in logical pixels. Hiding line
/// numbers narrows the chrome ([`crate::theme::GUTTER_BAR_WIDE`] to
/// [`crate::theme::GUTTER_BAR_NARROW`]) while row counts stay
/// honest; the geometry itself still comes from `gutter_geometry`.
pub fn gutter_bar_width(show_line_numbers: bool) -> f32 {
    if show_line_numbers {
        crate::theme::GUTTER_BAR_WIDE
    } else {
        crate::theme::GUTTER_BAR_NARROW
    }
}

/// Gutter width scaled from the mono font size. Calibrated so
/// [`crate::theme::TYPO_MONO_SIZE`] (12.0) reproduces
/// [`gutter_bar_width`]; other sizes scale linearly. Non-finite or
/// non-positive sizes fall back to the unscaled width so paint never
/// feeds NaN into layout. Pure; unit-tested.
pub fn gutter_bar_width_for_font(font_size: f32, show_line_numbers: bool) -> f32 {
    use crate::theme::TYPO_MONO_SIZE;
    let base = gutter_bar_width(show_line_numbers);
    if font_size.is_finite() && font_size > 0.0 && TYPO_MONO_SIZE > 0.0 {
        base * (font_size / TYPO_MONO_SIZE)
    } else {
        base
    }
}

// ----------------------------------------------------------------------------
// Click-to-jump pure mapping (headless; wiring owned by views.rs / agent A)
// ----------------------------------------------------------------------------
//
// WIRING (agent A / orchestrator): in `views.rs` gutter paint, on a click
// at local y within a gutter rect of `height` px over `total_rows` model
// rows, call `gutter_jump_target(&rows, total_rows, height, y)` and move
// the text selection / scroll to the returned model row. No egui code
// lives here so this file stays disjoint from paint.

/// Maps a gutter-local y (px) to a model row index.
///
/// `None` when there is nothing to map: zero rows, non-positive or
/// non-finite height, non-finite y, or y outside `[0.0, height)`.
/// Otherwise the fraction `y / height` scales to `[0, total_rows)`.
/// Pure; unit-tested.
pub fn gutter_row_at_y(y: f32, total_rows: usize, height: f32) -> Option<usize> {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return None;
    }
    if !y.is_finite() || y < 0.0 || y >= height {
        return None;
    }
    let row = ((y / height) * total_rows as f32).floor() as usize;
    if row < total_rows { Some(row) } else { None }
}

/// Hit-tests a gutter-local y against already-computed `segments`.
///
/// Returns the first segment with `y_start <= y < y_end` (the final
/// segment is inclusive of its end so clicks on its bottom edge land).
/// `None` for non-finite y or no containing segment. Pure; unit-tested.
pub fn gutter_segment_at_y(segments: &[GutterSegment], y: f32) -> Option<usize> {
    if !y.is_finite() {
        return None;
    }
    let mut idx = 0usize;
    for seg in segments {
        let last = idx.saturating_add(1) >= segments.len();
        let inside = if last {
            y >= seg.y_start && y <= seg.y_end
        } else {
            y >= seg.y_start && y < seg.y_end
        };
        if inside {
            return Some(idx);
        }
        idx = idx.saturating_add(1);
    }
    None
}

/// Click-to-jump target for a gutter click at y.
///
/// Maps y to a row with [`gutter_row_at_y`]; when that row is an equal
/// row (or the click lands on no segment), walks forward to the next
/// difference, else falls back to the previous difference before the
/// mapped row. `None` means "no diff to jump to". Pure; unit-tested.
pub fn gutter_jump_target(
    rows: &[TextRow],
    total_rows: usize,
    height: f32,
    y: f32,
) -> Option<usize> {
    let row = gutter_row_at_y(y, total_rows, height)?;
    if rows.get(row).is_some_and(|r| r.kind != TextRowKind::Equal) {
        return Some(row);
    }
    let mut i = row.saturating_add(1);
    while i < rows.len() {
        if rows.get(i).is_some_and(|r| r.kind != TextRowKind::Equal) {
            return Some(i);
        }
        i = i.saturating_add(1);
    }
    let mut j = row.min(rows.len());
    while j > 0 {
        j = j.saturating_sub(1);
        if rows.get(j).is_some_and(|r| r.kind != TextRowKind::Equal) {
            return Some(j);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_row(kind: TextRowKind) -> TextRow {
        TextRow {
            kind,
            left_no: None,
            right_no: None,
            left: String::new(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        }
    }

    #[test]
    fn empty_or_equal_rows_yield_no_segments() {
        let geom = gutter_geometry(0, 100.0, &[]);
        assert!(geom.segments.is_empty());

        let rows = vec![
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Equal),
        ];
        let geom2 = gutter_geometry(2, 200.0, &rows);
        assert!(geom2.segments.is_empty());
    }

    #[test]
    fn difference_rows_map_to_coordinates_and_merge() {
        let rows = vec![
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Inserted),
        ];
        // 4 rows, height = 400.0. Row height = 100.0.
        let geom = gutter_geometry(4, 400.0, &rows);
        assert_eq!(geom.segments.len(), 2);

        let Some(first) = geom.segments.first() else {
            return;
        };
        assert_eq!(first.kind, TextRowKind::Deleted);
        assert_eq!(first.y_start, 100.0);
        assert_eq!(first.y_end, 300.0);

        let Some(second) = geom.segments.get(1) else {
            return;
        };
        assert_eq!(second.kind, TextRowKind::Inserted);
        assert_eq!(second.y_start, 300.0);
        assert_eq!(second.y_end, 400.0);
    }

    #[test]
    fn bar_width_narrows_when_line_numbers_hide() {
        use crate::theme::{GUTTER_BAR_NARROW, GUTTER_BAR_WIDE};
        assert_eq!(gutter_bar_width(true), GUTTER_BAR_WIDE);
        assert_eq!(gutter_bar_width(false), GUTTER_BAR_NARROW);
        assert_eq!(gutter_bar_width(true), 12.0);
        assert_eq!(gutter_bar_width(false), 6.0);
        assert!(gutter_bar_width(false) < gutter_bar_width(true));
    }

    #[test]
    fn gutter_tokens_match_spacing_scale() {
        use crate::theme::{
            GUTTER_BAR_NARROW, GUTTER_BAR_WIDE, GUTTER_MERGE_GAP, GUTTER_MIN_SEGMENT_H, SPACE_MD,
        };
        assert_eq!(GUTTER_BAR_WIDE, SPACE_MD);
        assert_eq!(GUTTER_BAR_NARROW, 6.0);
        assert_eq!(GUTTER_MIN_SEGMENT_H, 2.0);
        assert_eq!(GUTTER_MERGE_GAP, 2.0);
    }

    #[test]
    fn bar_width_for_font_scales_from_mono_size() {
        use crate::theme::{TYPO_MONO_SIZE, TYPO_MONO_SIZE_MAX};
        // Calibrated: mono size reproduces the unscaled widths.
        assert_eq!(gutter_bar_width_for_font(TYPO_MONO_SIZE, true), 12.0);
        assert_eq!(gutter_bar_width_for_font(TYPO_MONO_SIZE, false), 6.0);
        // Upper mono bound (13px) scales linearly.
        assert_eq!(gutter_bar_width_for_font(TYPO_MONO_SIZE_MAX, true), 13.0);
        assert_eq!(gutter_bar_width_for_font(TYPO_MONO_SIZE_MAX, false), 6.5);
        // Bad sizes fall back instead of poisoning layout.
        assert_eq!(gutter_bar_width_for_font(0.0, true), 12.0);
        assert_eq!(gutter_bar_width_for_font(-4.0, false), 6.0);
        assert_eq!(gutter_bar_width_for_font(f32::NAN, true), 12.0);
        assert_eq!(gutter_bar_width_for_font(f32::INFINITY, false), 6.0);
    }

    #[test]
    fn row_at_y_maps_fraction_to_row_index() {
        // 4 rows over 400px: each row owns a 100px band.
        assert_eq!(gutter_row_at_y(0.0, 4, 400.0), Some(0));
        assert_eq!(gutter_row_at_y(99.9, 4, 400.0), Some(0));
        assert_eq!(gutter_row_at_y(100.0, 4, 400.0), Some(1));
        assert_eq!(gutter_row_at_y(250.0, 4, 400.0), Some(2));
        assert_eq!(gutter_row_at_y(399.9, 4, 400.0), Some(3));
    }

    #[test]
    fn row_at_y_rejects_empty_and_out_of_range() {
        assert_eq!(gutter_row_at_y(10.0, 0, 400.0), None);
        assert_eq!(gutter_row_at_y(10.0, 4, 0.0), None);
        assert_eq!(gutter_row_at_y(10.0, 4, -50.0), None);
        assert_eq!(gutter_row_at_y(-1.0, 4, 400.0), None);
        assert_eq!(gutter_row_at_y(400.0, 4, 400.0), None);
        assert_eq!(gutter_row_at_y(999.0, 4, 400.0), None);
        assert_eq!(gutter_row_at_y(f32::NAN, 4, 400.0), None);
        assert_eq!(gutter_row_at_y(10.0, 4, f32::NAN), None);
        assert_eq!(gutter_row_at_y(f32::INFINITY, 4, 400.0), None);
    }

    #[test]
    fn segment_hit_test_finds_containing_segment() {
        let rows = vec![
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Inserted),
        ];
        let geom = gutter_geometry(4, 400.0, &rows);
        assert_eq!(geom.segments.len(), 2);
        // First segment covers rows 1..3 -> y 100..300.
        assert_eq!(gutter_segment_at_y(&geom.segments, 150.0), Some(0));
        assert_eq!(gutter_segment_at_y(&geom.segments, 100.0), Some(0));
        // Second segment covers row 3 -> y 300..400.
        assert_eq!(gutter_segment_at_y(&geom.segments, 350.0), Some(1));
        assert_eq!(gutter_segment_at_y(&geom.segments, 400.0), Some(1));
        // Equal-run gap and outside hit nothing.
        assert_eq!(gutter_segment_at_y(&geom.segments, 50.0), None);
        assert_eq!(gutter_segment_at_y(&geom.segments, 500.0), None);
        assert_eq!(gutter_segment_at_y(&geom.segments, f32::NAN), None);
        assert_eq!(gutter_segment_at_y(&[], 150.0), None);
    }

    #[test]
    fn jump_target_snaps_equal_clicks_to_nearest_diff() {
        let rows = vec![
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Inserted),
        ];
        // Click on the diff row itself stays.
        assert_eq!(gutter_jump_target(&rows, 4, 400.0, 150.0), Some(1));
        // Click on equal row 0 walks forward to row 1.
        assert_eq!(gutter_jump_target(&rows, 4, 400.0, 50.0), Some(1));
        // Click on equal row 2 walks forward to row 3.
        assert_eq!(gutter_jump_target(&rows, 4, 400.0, 250.0), Some(3));
        // No diffs at all: no target.
        let equals = vec![
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Equal),
        ];
        assert_eq!(gutter_jump_target(&equals, 2, 200.0, 50.0), None);
        // Out-of-range click: no target.
        assert_eq!(gutter_jump_target(&rows, 4, 400.0, 999.0), None);
        assert_eq!(gutter_jump_target(&[], 0, 400.0, 50.0), None);
    }
}
