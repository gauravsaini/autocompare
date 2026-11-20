//! In-place live editor copy mutations.
//!
//! Provides pure mutations for transferring text rows Left-to-Right or Right-to-Left.

#![forbid(unsafe_code)]

use crate::viewmodel::{TextFilter, TextRowKind, TextRows};

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

// ----------------------------------------------------------------------------
// Copy affordance geometry + status (headless; paint owner ke paas).
// Geometry first: 24px square hit floor (toolbar precedent
// `home::HOME_TOOLBAR_BUTTON_H` parity, SPACE_XL single-source).
// Typography second: status line Small weak precedent reuse.
// Colors untouched.
// ----------------------------------------------------------------------------

/// Copy arrow hit floor px: 24 square (koi chhota tap target nahi).
pub const COPY_HIT_MIN: f32 = crate::theme::SPACE_XL;

/// Copy arrow hit size px (single source). Pure; headless-tested.
pub fn copy_hit_size() -> f32 {
    COPY_HIT_MIN
}

/// Copy status line: kitni rows kis direction me gayin. Zero par honest
/// "nothing to copy", kabhi jhootha count nahi. Pure; headless-tested.
pub fn copy_status_text(copied: usize, dir: CopyDirection) -> String {
    if copied == 0 {
        return String::from("nothing to copy");
    }
    let arrow = match dir {
        CopyDirection::LeftToRight => "left\u{2192}right",
        CopyDirection::RightToLeft => "right\u{2192}left",
    };
    if copied == 1 {
        format!("copied 1 row {arrow}")
    } else {
        format!("copied {copied} rows {arrow}")
    }
}

/// Copy ke baad focus: `idx` ke baad wala next difference (copy ki hui
/// row Equal ho gayi, isliye wahi `idx` dobara nahi). Koi diff na bache
/// to `None` (paint selection wahi rakhta hai). Pure; headless-tested.
pub fn copy_focus_after(model: &TextRows, idx: usize) -> Option<usize> {
    crate::viewmodel::next_diff_idx(&model.rows, Some(idx))
}

// ----------------------------------------------------------------------------
// Center-gutter per-hunk arrow visibility (headless; paint owner ke paas).
// BC center bar har hunk par ek arrow dikhata hai, har row par nahi —
// continuation rows blank click-to-select rehti hain. Yeh section uska
// headless rule hai: [`copy_hunk_head`] contiguous diff block ka pehla row
// hai (Equal ke baad ya index 0 par pehla diff); paint arrow sirf wahi
// banata hai, geometry [`crate::gutter::gutter_arrow_rect`] se. Geometry
// first (24px hit floor upar frozen hai), typo second (status line Small
// weak precedent reuse — yahan koi naya size nahi), colors frozen.
// WIRING (paint): center column me `copy_center_arrow_visible(model, idx)`
// true par hi arrow button, false par blank (click row select karta hai).
// ----------------------------------------------------------------------------

/// Center-gutter hunk head: contiguous diff block ka pehla row.
/// `idx` OOB ya Equal par false; pehla diff (idx 0) ya Equal ke baad wala
/// pehla diff true. Pure; headless-tested.
pub fn copy_hunk_head(model: &TextRows, idx: usize) -> bool {
    let Some(row) = model.rows.get(idx) else {
        return false;
    };
    if row.kind == TextRowKind::Equal {
        return false;
    }
    if idx == 0 {
        return true;
    }
    match model.rows.get(idx.saturating_sub(1)) {
        Some(prev) => prev.kind == TextRowKind::Equal,
        None => true,
    }
}

/// Center arrow dikhe ya blank: [`copy_hunk_head`] ka paint alias.
/// Continuations blank rehti hain taaki gutter fixed width me rahe bina
/// per-row button clutter ke. Pure; headless-tested.
pub fn copy_center_arrow_visible(model: &TextRows, idx: usize) -> bool {
    copy_hunk_head(model, idx)
}

// ----------------------------------------------------------------------------
// CopyTake6 actions + nav (headless; paint owner ke paas). BC parity:
// inline copy left/right arrows, next/prev diff nav, display filter
// All/Diffs/Same. Geometry first (24px hit floor + tight gaps, scale
// reuse), typography second (action text Small weak status precedent
// reuse — `copy_status_text` wahi tone me paint hota hai). Colors frozen.
// WIRING (paint): center-gutter arrow button par `copy_arrow_label(dir)`
// glyph; nav row me `copy_nav_pair_width` Prev/Next pair; filter
// segmented me `copy_filter_options` + `copy_filter_short_label`.
// ----------------------------------------------------------------------------

/// Inline copy arrow glyph for paint: `>` LeftToRight, `<` RightToLeft.
/// Text precedent, koi naya asset nahi. Pure; headless-tested.
pub fn copy_arrow_label(dir: CopyDirection) -> &'static str {
    match dir {
        CopyDirection::LeftToRight => ">",
        CopyDirection::RightToLeft => "<",
    }
}

/// Next/prev diff nav button height px: copy hit floor parity (24).
pub fn copy_nav_button_h() -> f32 {
    COPY_HIT_MIN
}

/// Nav pair gap px: tight rhythm (4).
pub fn copy_nav_gap() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Prev + Next pair width px: 2 x 24 + 4 = 52. Pure; headless-tested.
pub fn copy_nav_pair_width() -> f32 {
    COPY_HIT_MIN + COPY_HIT_MIN + crate::theme::GEO_TIGHT_GAP
}

/// BC display filter options: All / Differences / Same (Context paint me
/// alag toggle hai, segmented me nahi). Pure; headless-tested.
pub fn copy_filter_options() -> [TextFilter; 3] {
    [TextFilter::All, TextFilter::Differences, TextFilter::Same]
}

/// Segmented filter short label (BC parity): "All/Diffs/Same".
/// `Context` alag toggle par apna naam rakhta hai. Pure; headless-tested.
pub fn copy_filter_short_label(filter: TextFilter) -> &'static str {
    match filter {
        TextFilter::All => "All",
        TextFilter::Differences => "Diffs",
        TextFilter::Same => "Same",
        TextFilter::Context => "Context",
    }
}

/// Filter segment button height px: copy hit floor parity (24).
pub fn copy_filter_button_h() -> f32 {
    COPY_HIT_MIN
}

/// Filter segment gap px: segmented control, buttons adjacent (0.0).
/// Zero gap spacing scale ke bahar hai hi nahi — koi naya size nahi.
pub fn copy_filter_segment_gap() -> f32 {
    0.0
}

/// Copy action text role: Small weak proportional (status line precedent
/// reuse). Hierarchy size + weight se, tracking zero rehta hai.
pub const fn copy_action_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::chrome_status()
}

/// Copy action text size px (single source: Small, 11.0).
pub fn copy_action_text_size() -> f32 {
    copy_action_text_role().size()
}

/// Copy action text line-height px (Small role se).
pub fn copy_action_text_line_height_px() -> f32 {
    copy_action_text_role().line_height_px()
}

/// Copy action text numeric weight (Small role se, 600).
pub fn copy_action_text_weight_value() -> u16 {
    copy_action_text_role().weight_value()
}

/// Copy action text tracking em (Small role se, zero).
pub fn copy_action_text_tracking_em() -> f32 {
    copy_action_text_role().letter_spacing_em()
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

    #[test]
    fn copy_hit_floor_matches_toolbar_rhythm() {
        assert_eq!(copy_hit_size(), 24.0);
        assert_eq!(COPY_HIT_MIN, crate::theme::SPACE_XL);
        assert_eq!(COPY_HIT_MIN, crate::home::HOME_TOOLBAR_BUTTON_H);
    }

    #[test]
    fn copy_status_text_counts_honestly_per_direction() {
        assert_eq!(
            copy_status_text(0, CopyDirection::LeftToRight),
            "nothing to copy"
        );
        assert_eq!(
            copy_status_text(1, CopyDirection::LeftToRight),
            "copied 1 row left\u{2192}right"
        );
        assert_eq!(
            copy_status_text(2, CopyDirection::RightToLeft),
            "copied 2 rows right\u{2192}left"
        );
    }

    #[test]
    fn copy_focus_after_moves_past_copied_row() {
        let mut model = section_model();
        // Diffs at 1, 2: copy row 1, focus should land on 2.
        copy_line(&mut model, 1, CopyDirection::LeftToRight).unwrap_or_default();
        assert_eq!(copy_focus_after(&model, 1), Some(2));
        // Last diff copied: no next diff.
        copy_line(&mut model, 2, CopyDirection::LeftToRight).unwrap_or_default();
        assert_eq!(copy_focus_after(&model, 2), None);
    }
}

#[cfg(test)]
mod exq_copytake6_tests {
    use super::*;
    use crate::theme::TypoRole;

    #[test]
    fn copytake6_arrows_point_along_copy_direction() {
        assert_eq!(copy_arrow_label(CopyDirection::LeftToRight), ">");
        assert_eq!(copy_arrow_label(CopyDirection::RightToLeft), "<");
    }

    #[test]
    fn copytake6_nav_pair_shares_hit_floor_and_tight_gap() {
        assert_eq!(copy_nav_button_h(), COPY_HIT_MIN);
        assert_eq!(copy_nav_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(copy_nav_pair_width(), 52.0);
        assert_eq!(
            copy_nav_pair_width(),
            copy_nav_button_h() + copy_nav_button_h() + copy_nav_gap()
        );
    }

    #[test]
    fn copytake6_filter_options_and_short_labels() {
        assert_eq!(
            copy_filter_options(),
            [TextFilter::All, TextFilter::Differences, TextFilter::Same]
        );
        assert_eq!(copy_filter_short_label(TextFilter::All), "All");
        assert_eq!(copy_filter_short_label(TextFilter::Differences), "Diffs");
        assert_eq!(copy_filter_short_label(TextFilter::Same), "Same");
        assert_eq!(copy_filter_short_label(TextFilter::Context), "Context");
        assert_eq!(copy_filter_button_h(), COPY_HIT_MIN);
        assert_eq!(copy_filter_segment_gap(), 0.0);
    }

    #[test]
    fn copytake6_action_text_reuses_small_status_role() {
        assert_eq!(copy_action_text_role(), TypoRole::chrome_status());
        assert_eq!(copy_action_text_role(), TypoRole::Small);
        assert_eq!(copy_action_text_size(), 11.0);
        assert_eq!(
            copy_action_text_line_height_px(),
            TypoRole::Small.line_height_px()
        );
        assert_eq!(
            copy_action_text_weight_value(),
            TypoRole::Small.weight_value()
        );
        assert_eq!(copy_action_text_tracking_em(), 0.0);
    }

    #[test]
    fn copytake6_narrow_fixture_numbers_hold() {
        // Mirrors checks/gui-exq-copytake6/comparison/copytake-narrow-240.json:
        // 240px avail par nav pair (52) fit, filter segmented adjacent.
        assert_eq!(copy_nav_pair_width(), 52.0);
        assert_eq!(copy_filter_options().len(), 3);
        assert!(copy_nav_pair_width() < 240.0);
    }
}

// ----------------------------------------------------------------------------
// GUTTERCOPY8: copy arrow visual-vs-hit + hunk pitch (headless).
// BC center arrows 24px tap-target me 16px chevron rakhte hain (visual
// chhota, hit bada); paas-paas hunks ke arrows ek rhythm (pitch) par
// taaki stacked arrows collide na karen. Geometry first: hit floor +
// tight gap reuse, koi naya literal nahi. Colors frozen.
// WIRING (paint): center arrow button visual `copy_arrow_visual_size`,
// hit `copy_hit_size`; stacked arrows `copy_hunk_arrow_pitch(row_h)` pitch.
// ----------------------------------------------------------------------------

/// Stacked hunk arrows ke beech gap px: tight rhythm (4).
pub const COPY_HUNK_ARROW_GAP: f32 = crate::theme::GEO_TIGHT_GAP;

/// Copy arrow visual size px: 24 hit me 16 chevron (gutter arrow-box parity).
pub fn copy_arrow_visual_size() -> f32 {
    COPY_HIT_MIN - 2.0 * COPY_HUNK_ARROW_GAP
}

/// Paas-paas hunks ke arrows ka vertical pitch px: `row_h + gap`.
/// `None`: non-finite/non-positive row height. Pure; headless-tested.
pub fn copy_hunk_arrow_pitch(row_h: f32) -> Option<f32> {
    if !row_h.is_finite() || row_h <= 0.0 {
        return None;
    }
    let pitch = row_h + COPY_HUNK_ARROW_GAP;
    if pitch.is_finite() { Some(pitch) } else { None }
}

// ----------------------------------------------------------------------------
// EXQ-MERGE2: copy button hit-slop + row height geometry/typo helpers (headless).
// BC merge parity: copy arrow buttons expand by hit-slop so clicks near
// the button register reliably; row height is tied strictly to TypoRole::Mono
// line-height (15.6px) with zero ad-hoc magic sizes. Geometry first:
// COPY_BUTTON_HIT_SLOP uses SPACE_XS token (4px), expanding 24px hit floor
// to 32px target; typography second: copy_row_height reuses Mono line_height_px.
// Colors frozen: no color touches.
// WIRING (paint): copy button hit detection uses copy_button_hit_rect /
// copy_button_hit_contains; row layouts use copy_row_height.
// ----------------------------------------------------------------------------

/// Copy button hit-slop px per side: one XS step (`SPACE_XS` = 4.0).
/// Expands 24px button hit target by 4px on each side -> 32px effective target.
pub const COPY_BUTTON_HIT_SLOP: f32 = crate::theme::SPACE_XS;

/// Returns the copy button hit-slop px. Pure; headless-tested.
pub fn copy_button_hit_slop() -> f32 {
    COPY_BUTTON_HIT_SLOP
}

/// Copy button effective hit size including hit-slop on both sides:
/// `copy_hit_size() + 2 * COPY_BUTTON_HIT_SLOP` (24 + 8 = 32.0). Pure; headless-tested.
pub fn copy_hit_size_with_slop() -> f32 {
    copy_hit_size() + 2.0 * COPY_BUTTON_HIT_SLOP
}

/// Copy button hit rect `(x0, y0, x1, y1)` px centered at `(center_x, center_y)`.
/// Base `copy_hit_size()` (24px) har taraf `COPY_BUTTON_HIT_SLOP` (4px) se expand hota hai
/// (32px square target). Non-finite input par `None`. Pure; headless-tested.
pub fn copy_button_hit_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    if !center_x.is_finite() || !center_y.is_finite() {
        return None;
    }
    let half = (copy_hit_size() / 2.0) + COPY_BUTTON_HIT_SLOP;
    let (x0, y0, x1, y1) = (
        center_x - half,
        center_y - half,
        center_x + half,
        center_y + half,
    );
    if x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite() {
        Some((x0, y0, x1, y1))
    } else {
        None
    }
}

/// Point-in-rect hit test: checks if `(x, y)` falls within the copy button's
/// slop-expanded hit rect centered at `(center_x, center_y)` (edges inclusive).
/// Non-finite inputs return `false`. Pure; headless-tested.
pub fn copy_button_hit_contains(center_x: f32, center_y: f32, x: f32, y: f32) -> bool {
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    match copy_button_hit_rect(center_x, center_y) {
        Some((x0, y0, x1, y1)) => x >= x0 && x <= x1 && y >= y0 && y <= y1,
        None => false,
    }
}

/// Expands an arbitrary copy button bounding rect `(x0, y0, x1, y1)`
/// by `COPY_BUTTON_HIT_SLOP` on each edge.
/// Returns `None` if any coordinate is non-finite. Pure; headless-tested.
pub fn copy_hit_rect_with_slop(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<(f32, f32, f32, f32)> {
    if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
        return None;
    }
    let s = COPY_BUTTON_HIT_SLOP;
    let res = (x0 - s, y0 - s, x1 + s, y1 + s);
    if res.0.is_finite() && res.1.is_finite() && res.2.is_finite() && res.3.is_finite() {
        Some(res)
    } else {
        None
    }
}

/// Typographic role for copy row text: Mono (12 regular monospace, tabular).
/// Content role single source parity. Pure.
pub const fn copy_row_typo_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Copy row height px derived directly from `TypoRole::Mono.line_height_px()`.
/// Parity with Beyond Compare monospace line height (15.6px). Pure; headless-tested.
pub fn copy_row_height() -> f32 {
    copy_row_typo_role().line_height_px()
}

/// Total vertical height px for a block of `line_count` copy rows:
/// `line_count * copy_row_height()`.
/// Khali block par 0.0. Pure; headless-tested.
pub fn copy_block_height(line_count: usize) -> f32 {
    line_count as f32 * copy_row_height()
}

/// Calculates how many full copy rows fit in `available_h`.
/// Non-finite or non-positive input returns 0. Pure; headless-tested.
pub fn copy_rows_fit(available_h: f32) -> usize {
    if !available_h.is_finite() || available_h <= 0.0 {
        return 0;
    }
    let rh = copy_row_height();
    if rh <= 0.0 {
        return 0;
    }
    (available_h / rh).floor() as usize
}

/// Vertical pitch for stacked hunk arrows using default `copy_row_height()`.
/// Pure; headless-tested.
pub fn copy_arrow_default_pitch() -> Option<f32> {
    copy_hunk_arrow_pitch(copy_row_height())
}

// ----------------------------------------------------------------------------
// EXQ-MERGE3: copy button symmetric hit-slop padding (headless).
// BC parity: the 24px copy arrow hit floor expands by the same XS slop
// (4px) on every side -> symmetric 32px target, so clicks near any
// edge register identically. Geometry first (SPACE_XS reuse, no new
// literal), typography second (row height already Mono-sourced above),
// colors frozen.
// WIRING (paint): hit rects use `copy_button_hit_rect` (already
// symmetric); `copy_button_sym_pad` documents the per-side pad,
// `copy_button_padded_size` the symmetric target,
// `copy_button_row_vpad` the Mono-rhythm centering.
// ----------------------------------------------------------------------------

/// Copy button symmetric pad px per side: one XS step single source.
/// Equal on left/right/top/bottom — no lopsided tap target.
pub fn copy_button_sym_pad() -> f32 {
    COPY_BUTTON_HIT_SLOP
}

/// Copy button symmetric target size px: hit floor + pad on both sides
/// (24 + 2 * 4 = 32). Must equal [`copy_hit_size_with_slop`].
pub fn copy_button_padded_size() -> f32 {
    copy_hit_size() + 2.0 * copy_button_sym_pad()
}

/// Copy button vertical centering pad px over one Mono row:
/// half the leftover of the hit floor over [`copy_row_height`]
/// ((24 - 15.6) / 2 = 4.2). Proves the button centers on the same
/// Mono rhythm take uses. Pure; headless-tested.
pub fn copy_button_row_vpad() -> f32 {
    (copy_hit_size() - copy_row_height()) / 2.0
}

/// True when a slop-expanded copy rect is symmetric: each edge moved
/// exactly `COPY_BUTTON_HIT_SLOP` from the visual rect.
/// Pure; headless-tested.
pub fn copy_hit_rect_is_symmetric(x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    if x0 > x1 || y0 > y1 {
        return false;
    }
    match copy_hit_rect_with_slop(x0, y0, x1, y1) {
        Some((hx0, hy0, hx1, hy1)) => {
            let s = copy_button_sym_pad();
            ((x0 - hx0) - s).abs() < 0.0001
                && ((y0 - hy0) - s).abs() < 0.0001
                && ((hx1 - x1) - s).abs() < 0.0001
                && ((hy1 - y1) - s).abs() < 0.0001
        }
        None => false,
    }
}

#[cfg(test)]
mod exq_tree2_tests {
    use super::*;
    use crate::viewmodel::{TextRow, TextRowKind};

    fn hunk_row(kind: TextRowKind) -> TextRow {
        TextRow {
            kind,
            ..Default::default()
        }
    }

    #[test]
    fn tree2_copy_arrows_head_each_hunk_only() {
        // Mirrors checks/gui-exq-tree2/round-1/comparison/edge-deep-nest.json:
        // Changed+Changed ek hunk (dusra blank), Equal kabhi head nahi,
        // Equal ke baad diff phir head. OOB/empty kabhi head nahi.
        let model = TextRows {
            rows: vec![
                hunk_row(TextRowKind::Changed),
                hunk_row(TextRowKind::Changed),
                hunk_row(TextRowKind::Equal),
                hunk_row(TextRowKind::Inserted),
            ],
            ..Default::default()
        };
        assert!(copy_hunk_head(&model, 0));
        assert!(!copy_hunk_head(&model, 1));
        assert!(!copy_hunk_head(&model, 2));
        assert!(copy_hunk_head(&model, 3));
        assert!(!copy_hunk_head(&model, 4));
        assert!(copy_center_arrow_visible(&model, 0));
        assert!(!copy_center_arrow_visible(&model, 1));
        assert!(!copy_center_arrow_visible(&model, 2));
        assert!(copy_center_arrow_visible(&model, 3));
        let empty = TextRows::default();
        assert!(!copy_hunk_head(&empty, 0));
    }
}

#[cfg(test)]
mod exq_guttercopy8_tests {
    use super::*;

    #[test]
    fn guttercopy8_arrow_visual_matches_gutter_box_and_pitch_rhythm() {
        // Mirrors checks/gui-exq-guttercopy8/round-1/comparison/edge-padded-minimap-tall.json.
        assert_eq!(COPY_HUNK_ARROW_GAP, crate::theme::GEO_TIGHT_GAP);
        assert_eq!(COPY_HUNK_ARROW_GAP, 4.0);
        // 24 hit me 12 nahi, 16 chevron: gutter arrow-box parity.
        assert_eq!(copy_arrow_visual_size(), 16.0);
        assert_eq!(copy_arrow_visual_size(), crate::gutter::GUTTER_ARROW_BOX);
        assert!(copy_arrow_visual_size() < copy_hit_size());
        // Pitch: 19.6 row par 23.6; kharab input par None.
        assert!(copy_hunk_arrow_pitch(19.6).is_some_and(|p| (p - 23.6).abs() < 0.01));
        assert_eq!(copy_hunk_arrow_pitch(0.0), None);
        assert_eq!(copy_hunk_arrow_pitch(-4.0), None);
        assert_eq!(copy_hunk_arrow_pitch(f32::NAN), None);
        assert_eq!(copy_hunk_arrow_pitch(f32::INFINITY), None);
    }
}

#[cfg(test)]
mod exq_merge2_tests {
    use super::*;
    use crate::theme::{SPACE_XS, TypoRole};

    #[test]
    fn merge2_row_height() {
        let role = copy_row_typo_role();
        assert_eq!(role, TypoRole::Mono);
        assert_eq!(role.size(), 12.0);
        let expected_lh = role.line_height_px();
        assert!((expected_lh - 15.6).abs() < 0.01);
        assert_eq!(copy_row_height(), expected_lh);
        assert!((copy_row_height() - 15.6).abs() < 0.01);

        // Block height scales cleanly
        assert_eq!(copy_block_height(0), 0.0);
        assert!((copy_block_height(1) - 15.6).abs() < 0.01);
        assert!((copy_block_height(10) - 156.0).abs() < 0.01);

        // Rows fit calculations
        assert_eq!(copy_rows_fit(0.0), 0);
        assert_eq!(copy_rows_fit(-10.0), 0);
        assert_eq!(copy_rows_fit(f32::NAN), 0);
        assert_eq!(copy_rows_fit(15.5), 0);
        assert_eq!(copy_rows_fit(15.6), 1);
        assert_eq!(copy_rows_fit(31.2), 2);
        assert_eq!(copy_rows_fit(100.0), 6);

        // Arrow pitch with default row height
        let pitch = copy_arrow_default_pitch();
        assert!(pitch.is_some());
        if let Some(p) = pitch {
            assert!((p - (15.6 + COPY_HUNK_ARROW_GAP)).abs() < 0.01);
            assert!((p - 19.6).abs() < 0.01);
        }
    }

    #[test]
    fn merge2_hit_slop() {
        assert_eq!(COPY_BUTTON_HIT_SLOP, SPACE_XS);
        assert_eq!(COPY_BUTTON_HIT_SLOP, 4.0);
        assert_eq!(copy_button_hit_slop(), 4.0);

        // Hit size without and with slop
        assert_eq!(copy_hit_size(), 24.0);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        assert_eq!(
            copy_hit_size_with_slop(),
            copy_hit_size() + 2.0 * COPY_BUTTON_HIT_SLOP
        );

        // Hit rect expansion centered at (100.0, 50.0)
        // Base 24px box centered would be (88.0, 38.0, 112.0, 62.0)
        // With 4px slop, box expands to (84.0, 34.0, 116.0, 66.0)
        let rect = copy_button_hit_rect(100.0, 50.0);
        assert_eq!(rect, Some((84.0, 34.0, 116.0, 66.0)));

        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
        }

        // Hit contains tests
        assert!(copy_button_hit_contains(100.0, 50.0, 100.0, 50.0));
        assert!(copy_button_hit_contains(100.0, 50.0, 84.0, 34.0)); // corners
        assert!(copy_button_hit_contains(100.0, 50.0, 116.0, 66.0));
        assert!(copy_button_hit_contains(100.0, 50.0, 85.0, 40.0)); // inside slop
        assert!(!copy_button_hit_contains(100.0, 50.0, 83.9, 50.0)); // just outside
        assert!(!copy_button_hit_contains(100.0, 50.0, 116.1, 50.0));
        assert!(!copy_button_hit_contains(100.0, 50.0, 100.0, 33.9));
        assert!(!copy_button_hit_contains(100.0, 50.0, 100.0, 66.1));

        // Invalid inputs
        assert_eq!(copy_button_hit_rect(f32::NAN, 50.0), None);
        assert_eq!(copy_button_hit_rect(100.0, f32::INFINITY), None);
        assert!(!copy_button_hit_contains(f32::NAN, 50.0, 100.0, 50.0));
        assert!(!copy_button_hit_contains(100.0, 50.0, f32::NAN, 50.0));

        // Arbitrary rect expansion
        let expanded = copy_hit_rect_with_slop(10.0, 20.0, 34.0, 44.0);
        assert_eq!(expanded, Some((6.0, 16.0, 38.0, 48.0)));
        assert_eq!(copy_hit_rect_with_slop(f32::NAN, 20.0, 34.0, 44.0), None);
    }

    #[test]
    fn merge2_narrow_fixture() {
        // Mirrors checks/gui-exq-merge2/comparison/merge-3way-narrow.json
        assert_eq!(COPY_BUTTON_HIT_SLOP, 4.0);
        assert_eq!(COPY_BUTTON_HIT_SLOP, SPACE_XS);
        assert_eq!(copy_hit_size(), 24.0);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        assert!((copy_row_height() - 15.6).abs() < 0.01);
        assert_eq!(copy_row_height(), TypoRole::Mono.line_height_px());
    }
}

// ----------------------------------------------------------------------------
// EXQ-COPY7: copy-side arrow column + symmetric hit rects + copy-line row
// affordance at 960 wide (headless; paint owner ke paas). BC center gutter
// me per-hunk copy arrows hit-slop ke saath rehte hain (click paas se bhi
// lage), arrow column ek hi glyph chaudha hai, har copy-line row Mono
// rhythm par hai. Geometry first (SPACE tokens + char advance reuse, koi
// naya literal nahi), typography second (Mono 15.6 single source), colors
// frozen. WIRING (paint): center column width `copy_arrow_column_width`,
// arrow hit `copy_arrow_hit_rect` / `copy_arrow_hit_contains`, row layout
// `copy_line_row_height` (== `copy_row_height`).
// ----------------------------------------------------------------------------

/// Copy arrow advance px per char: take label advance reuse (7.0 body-13
/// estimate parity, koi naya literal nahi). Pure; headless-tested.
pub fn copy_arrow_advance_px() -> f32 {
    crate::take::TAKE_LABEL_ADVANCE_PX
}

/// Copy arrow glyph width in chars: single ">" / "<" (see [`copy_arrow_label`]).
pub const COPY_ARROW_GLYPH_CHARS: usize = 1;

/// Copy-side arrow column width px: 1 glyph * advance + 2 x section-gap
/// side bearings (7 + 16 = 23). Char metrics se, SPACE tokens se; slop
/// hit (32) hamesha cover karta hai taaki arrows cramped na lagen.
/// Pure; headless-tested.
pub fn copy_arrow_column_width() -> f32 {
    COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * crate::theme::GEO_SECTION_GAP
}

/// Copy-line row affordance height px: one Mono line (15.6) — [`copy_row_height`]
/// ka paint alias taaki copy-line action wahi rhythm pakde. Pure; headless-tested.
pub fn copy_line_row_height() -> f32 {
    copy_row_height()
}

/// Copy-side arrow hit rect `(x0, y0, x1, y1)` px centered at `(cx, cy)`:
/// [`copy_button_hit_rect`] ka alias — 24px hit floor + 4px XS slop har
/// taraf = symmetric 32px target. Non-finite par `None`. Pure; headless-tested.
pub fn copy_arrow_hit_rect(cx: f32, cy: f32) -> Option<(f32, f32, f32, f32)> {
    copy_button_hit_rect(cx, cy)
}

/// Copy-side arrow hit test: `(x, y)` slop hit rect me hai ya nahi
/// (edges inclusive). Non-finite par false. Pure; headless-tested.
pub fn copy_arrow_hit_contains(cx: f32, cy: f32, x: f32, y: f32) -> bool {
    copy_button_hit_contains(cx, cy, x, y)
}

/// True when the copy-side arrow hit rect is symmetric: centered square
/// (32 x 32) jisme har taraf pad barabar ho. `copy_button_hit_rect` par
/// delegate karta hai. Pure; headless-tested.
pub fn copy_arrow_hit_is_symmetric(cx: f32, cy: f32) -> bool {
    match copy_arrow_hit_rect(cx, cy) {
        Some((x0, y0, x1, y1)) => {
            let w = x1 - x0;
            let h = y1 - y0;
            let half = (copy_hit_size() / 2.0) + COPY_BUTTON_HIT_SLOP;
            (w - copy_hit_size_with_slop()).abs() < 0.0001
                && (h - copy_hit_size_with_slop()).abs() < 0.0001
                && ((cx - x0) - half).abs() < 0.0001
                && ((x1 - cx) - half).abs() < 0.0001
                && ((cy - y0) - half).abs() < 0.0001
                && ((y1 - cy) - half).abs() < 0.0001
        }
        None => false,
    }
}

#[cfg(test)]
mod exq_copy7_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, SPACE_XS, TypoRole};

    #[test]
    fn copy7_arrow_hit_rects_symmetric_via_space_tokens() {
        // 24 hit floor (XL) + 4 XS slop har taraf = 32 square, centered.
        assert_eq!(COPY_BUTTON_HIT_SLOP, SPACE_XS);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        let rect = copy_arrow_hit_rect(100.0, 50.0);
        assert_eq!(rect, Some((84.0, 34.0, 116.0, 66.0)));
        assert_eq!(rect, copy_button_hit_rect(100.0, 50.0));
        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
            // Har taraf pad barabar: left == right == top == bottom == 16.
            assert!(((100.0 - x0) - 16.0).abs() < 0.0001);
            assert!(((x1 - 100.0) - 16.0).abs() < 0.0001);
            assert!(((50.0 - y0) - 16.0).abs() < 0.0001);
            assert!(((y1 - 50.0) - 16.0).abs() < 0.0001);
        }
        assert!(copy_arrow_hit_is_symmetric(100.0, 50.0));
        assert!(!copy_arrow_hit_is_symmetric(f32::NAN, 50.0));
        // Edges inclusive, bahar false.
        assert!(copy_arrow_hit_contains(100.0, 50.0, 84.0, 34.0));
        assert!(copy_arrow_hit_contains(100.0, 50.0, 116.0, 66.0));
        assert!(!copy_arrow_hit_contains(100.0, 50.0, 83.9, 50.0));
        assert!(!copy_arrow_hit_contains(100.0, 50.0, 100.0, 66.1));
        assert!(!copy_arrow_hit_contains(f32::NAN, 50.0, 100.0, 50.0));
    }

    #[test]
    fn copy7_arrow_column_width_from_char_metrics() {
        // 1 glyph * 7.0 advance + 2 x 8.0 section-gap bearings = 23.0.
        assert_eq!(COPY_ARROW_GLYPH_CHARS, 1);
        assert_eq!(copy_arrow_advance_px(), crate::take::TAKE_LABEL_ADVANCE_PX);
        assert_eq!(copy_arrow_advance_px(), 7.0);
        assert_eq!(
            copy_arrow_column_width(),
            COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * GEO_SECTION_GAP
        );
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        // Hit (32) column (23) ko cover karta hai — arrows cramped nahi.
        assert!(copy_hit_size_with_slop() > copy_arrow_column_width());
        assert!(copy_hit_size() > copy_arrow_column_width());
    }

    #[test]
    fn copy7_copy_line_row_height_from_mono_at_960() {
        // Row affordance Mono line-height single source (15.6), 960 wide par bhi.
        assert_eq!(copy_line_row_height(), copy_row_height());
        assert_eq!(copy_line_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
    }

    #[test]
    fn copy7_wide_fixture() {
        // Mirrors checks/gui-exq-copy7/round-1/comparison/copy-wide-960.json.
        let raw =
            include_str!("../../../checks/gui-exq-copy7/round-1/comparison/copy-wide-960.json");
        assert!(raw.contains("\"name\":\"copy-wide-960\""));
        assert!(raw.contains("\"avail_w\":960.0"));
        assert!(raw.contains("\"arrow_column_width\":23.0"));
        assert!(raw.contains("\"arrow_hit_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!((copy_hit_size_with_slop() - 32.0).abs() < 0.0001);
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!(copy_arrow_hit_is_symmetric(480.0, 300.0));
    }
}

#[cfg(test)]
mod exq_merge3_tests {
    use super::*;
    use crate::theme::{SPACE_XS, TypoRole};

    #[test]
    fn merge3_copy_sym_pad() {
        assert_eq!(copy_button_sym_pad(), COPY_BUTTON_HIT_SLOP);
        assert_eq!(copy_button_sym_pad(), SPACE_XS);
        assert_eq!(copy_button_sym_pad(), 4.0);
        assert_eq!(copy_button_padded_size(), 32.0);
        assert_eq!(copy_button_padded_size(), copy_hit_size_with_slop());
        assert!(copy_hit_rect_is_symmetric(10.0, 20.0, 34.0, 44.0));
        assert!(!copy_hit_rect_is_symmetric(f32::NAN, 20.0, 34.0, 44.0));
        assert!(!copy_hit_rect_is_symmetric(34.0, 20.0, 10.0, 44.0));
    }

    #[test]
    fn merge3_copy_row_vpad_matches_take_rhythm() {
        assert!((copy_row_height() - 15.6).abs() < 0.01);
        assert_eq!(copy_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_button_row_vpad() - 4.2).abs() < 0.01);
        assert_eq!(
            copy_button_row_vpad(),
            (copy_hit_size() - TypoRole::Mono.line_height_px()) / 2.0
        );
        assert_eq!(copy_button_row_vpad(), crate::take::take_button_vpad());
    }

    #[test]
    fn merge3_wide_fixture() {
        let raw = include_str!("../../../checks/gui-exq-merge3/comparison/merge-wide-960.json");
        assert!(raw.contains("\"name\":\"merge-wide-960\""));
        assert!(raw.contains("\"copy_button_sym_pad\":4.0"));
        assert!(raw.contains("\"copy_button_padded_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!((copy_button_sym_pad() - 4.0).abs() < 0.0001);
        assert!((copy_button_padded_size() - 32.0).abs() < 0.0001);
        assert!((copy_button_row_vpad() - 4.2).abs() < 0.01);
    }
}

// ----------------------------------------------------------------------------
// EXQ-COPY8: copy-side narrow-720 hit symmetry + Mono-baseline action (headless).
// BC center gutter 720px narrow par bhi generous hit-slop rakhta hai (arrow
// tight nahi lagta), copy-line action row baseline se bhatakti nahi.
// Geometry first: 24px XL hit floor + 4px XS slop har taraf = symmetric 32px
// target, narrow center avail/2 par, column 23 char metrics se (1 glyph x 7.0
// advance + 2 x 8.0 section-gap bearings). Typography second: copy-line
// action (Small 13.2 line) Mono row (15.6) me centered — dy single source.
// Colors frozen. WIRING (paint): narrow center `copy_center_x_for(720)`,
// arrow hit `copy_arrow_hit_rect(cx, cy)`, action y `copy_line_action_y`.
// ----------------------------------------------------------------------------

/// Narrow-pane center x px for a given avail width: `avail_w / 2`.
/// `None`: non-finite/non-positive avail. Pure; headless-tested.
pub fn copy_center_x_for(avail_w: f32) -> Option<f32> {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return None;
    }
    let cx = avail_w / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// Copy-line action vertical offset px inside one Mono row: half the leftover
/// of the Small action line over the Mono row ((15.6 - 13.2) / 2 = 1.2).
/// Action text Mono baseline rhythm par tikti hai. Pure; headless-tested.
pub fn copy_line_action_dy() -> f32 {
    (copy_line_row_height() - copy_action_text_line_height_px()) / 2.0
}

/// Copy-line action text top y px for a row starting at `row_top`:
/// `row_top + copy_line_action_dy`. Non-finite par `None`. Pure; headless-tested.
pub fn copy_line_action_y(row_top: f32) -> Option<f32> {
    if !row_top.is_finite() {
        return None;
    }
    let y = row_top + copy_line_action_dy();
    if y.is_finite() { Some(y) } else { None }
}

/// True when the arrow column + slop hit both fit in `avail_w` without
/// h-scroll (column 23 + hit 32 lazily 720 me). Non-finite/non-positive
/// avail par false. Pure; headless-tested.
pub fn copy_narrow_column_fits(avail_w: f32) -> bool {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return false;
    }
    copy_arrow_column_width() <= avail_w && copy_hit_size_with_slop() <= avail_w
}

#[cfg(test)]
mod exq_copy8_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, SPACE_XL, SPACE_XS, TypoRole};

    #[test]
    fn copy8_hit_slop_symmetric_at_narrow_720() {
        // SPACE tokens se: 24 XL floor + 4 XS slop har taraf = 32 square.
        assert_eq!(copy_hit_size(), SPACE_XL);
        assert_eq!(copy_button_sym_pad(), SPACE_XS);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        // Narrow center avail/2: 720 -> 360.
        assert_eq!(copy_center_x_for(720.0), Some(360.0));
        assert_eq!(copy_center_x_for(0.0), None);
        assert_eq!(copy_center_x_for(f32::NAN), None);
        assert_eq!(copy_center_x_for(f32::INFINITY), None);
        // Symmetric hit rect at narrow center (360,300): 344..376 x 284..316.
        let rect = copy_arrow_hit_rect(360.0, 300.0);
        assert_eq!(rect, Some((344.0, 284.0, 376.0, 316.0)));
        assert!(copy_arrow_hit_is_symmetric(360.0, 300.0));
        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
            assert!(((360.0 - x0) - 16.0).abs() < 0.0001);
            assert!(((x1 - 360.0) - 16.0).abs() < 0.0001);
            assert!(((300.0 - y0) - 16.0).abs() < 0.0001);
            assert!(((y1 - 300.0) - 16.0).abs() < 0.0001);
        }
        // Edges inclusive, just outside false — tight panes me bhi generous.
        assert!(copy_arrow_hit_contains(360.0, 300.0, 344.0, 284.0));
        assert!(copy_arrow_hit_contains(360.0, 300.0, 376.0, 316.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 343.9, 300.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 376.1, 300.0));
        assert!(!copy_narrow_column_fits(f32::NAN));
        assert!(copy_narrow_column_fits(720.0));
    }

    #[test]
    fn copy8_line_action_aligned_to_mono_baseline() {
        // Typo second: Small action line 13.2 Mono row 15.6 me centered.
        assert_eq!(copy_line_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!((copy_action_text_line_height_px() - 13.2).abs() < 0.01);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert_eq!(
            copy_line_action_dy(),
            (copy_line_row_height() - copy_action_text_line_height_px()) / 2.0
        );
        assert_eq!(copy_line_action_y(f32::NAN), None);
        assert_eq!(copy_line_action_y(f32::INFINITY), None);
        assert!(copy_line_action_y(100.0).is_some_and(|y| (y - 101.2).abs() < 0.01));
    }

    #[test]
    fn copy8_column_width_from_char_metrics_at_narrow() {
        // 1 glyph x 7.0 advance + 2 x 8.0 section-gap bearings = 23.0.
        assert_eq!(copy_arrow_advance_px(), 7.0);
        assert_eq!(
            copy_arrow_column_width(),
            COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * GEO_SECTION_GAP
        );
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!(copy_hit_size_with_slop() > copy_arrow_column_width());
        assert!(copy_narrow_column_fits(720.0));
    }

    #[test]
    fn copy8_narrow_fixture() {
        // Mirrors checks/gui-exq-copy8/round-1/comparison/copy-narrow-720.json.
        let raw =
            include_str!("../../../checks/gui-exq-copy8/round-1/comparison/copy-narrow-720.json");
        assert!(raw.contains("\"name\":\"copy-narrow-720\""));
        assert!(raw.contains("\"avail_w\":720.0"));
        assert!(raw.contains("\"arrow_column_width\":23.0"));
        assert!(raw.contains("\"arrow_hit_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"action_dy\":1.2"));
        assert_eq!(copy_center_x_for(720.0), Some(360.0));
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!((copy_hit_size_with_slop() - 32.0).abs() < 0.0001);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert!(copy_arrow_hit_is_symmetric(360.0, 300.0));
    }
}

// ----------------------------------------------------------------------------
// EXQ-COPY9: wide-960 arrow rhythm + Mono-baseline line actions (headless).
// BC copy dialog 960 wide par per-hunk arrows ek hi rhythm par rehte hain
// (hit symmetric, column char metrics se, stacked pitch Mono + gap),
// copy-line actions Mono baseline se bhatakti nahi. Geometry first: 24px
// XL hit floor + 4px XS slop har taraf = symmetric 32px target, wide
// center avail/2 par, column 23 char metrics se (1 glyph x 7.0 advance +
// 2 x 8.0 section-gap bearings), pitch 19.6 (15.6 Mono row + 4.0 tight
// gap). Typography second: copy-line action (Small 13.2 line) Mono row
// (15.6) me centered — dy 1.2 single source. Colors frozen.
// WIRING (paint): wide center `copy_center_x_for(960)`, arrow hit
// `copy_arrow_hit_rect(cx, cy)`, stacked pitch `copy_wide_arrow_pitch`,
// action y `copy_wide_action_y`, fit `copy_wide_column_fits`.
// ----------------------------------------------------------------------------

/// Wide column fit without h-scroll: arrow column (23) + slop hit (32)
/// `avail_w` me fit hote hain ya nahi. Narrow fit single source ka wide
/// alias taaki 960 wide rhythm wahi rule pakde. Pure; headless-tested.
pub fn copy_wide_column_fits(avail_w: f32) -> bool {
    copy_narrow_column_fits(avail_w)
}

/// Wide stacked-arrow pitch px: one Mono row + tight gap (15.6 + 4.0).
/// `None`: non-finite/non-positive row (single source delegate).
/// Pure; headless-tested.
pub fn copy_wide_arrow_pitch() -> Option<f32> {
    copy_arrow_default_pitch()
}

/// Wide copy-line action top y px for a row starting at `row_top`:
/// `row_top + dy` (dy 1.2 Mono-baseline centering). Non-finite par `None`.
/// Pure; headless-tested.
pub fn copy_wide_action_y(row_top: f32) -> Option<f32> {
    copy_line_action_y(row_top)
}

// ----------------------------------------------------------------------------
// EXQ-COPY10: narrow-720 arrow rhythm lock + Mono-baseline line actions (headless).
// BC copy-side 720 narrow par bhi per-hunk arrows ek hi rhythm par rehte hain
// (hit symmetric, column char metrics se, stacked pitch Mono + gap lock),
// copy-line actions Mono baseline se bhatakti nahi, copy button 32px hit
// centered rehta hai. Geometry first: 24px XL hit floor + 4px XS slop har
// taraf = symmetric 32px target, narrow center avail/2 (720 -> 360) par,
// column 23 char metrics se (1 glyph x 7.0 advance + 2 x 8.0 section-gap
// bearings), pitch 19.6 lock (15.6 Mono row + 4.0 tight gap). Typography
// second: copy-line action (Small 13.2 line) Mono row (15.6) me centered —
// dy 1.2 single source. Colors frozen.
// WIRING (paint): narrow center `copy_center_x_for(720)`, arrow hit
// `copy_arrow_hit_rect(cx, cy)`, stacked pitch `copy_narrow_arrow_pitch`,
// action y `copy_narrow_action_y`, fit `copy_narrow_column_fits`.
// ----------------------------------------------------------------------------

/// Narrow stacked-arrow pitch px: one Mono row + tight gap lock
/// (15.6 + 4.0 = 19.6). Narrow single source ka pitch alias taaki 720
/// narrow rhythm wahi rule pakde. Pure; headless-tested.
pub fn copy_narrow_arrow_pitch() -> Option<f32> {
    copy_arrow_default_pitch()
}

/// Narrow copy-line action top y px for a row starting at `row_top`:
/// `row_top + dy` (dy 1.2 Mono-baseline centering). Non-finite par `None`.
/// Pure; headless-tested.
pub fn copy_narrow_action_y(row_top: f32) -> Option<f32> {
    copy_line_action_y(row_top)
}

// ----------------------------------------------------------------------------
// EXQ-COPY11: tall-1200 arrow pitch lock + centered hits + steady dy (headless).
// BC copy-side lambe canvas par bhi per-hunk arrows ek hi rhythm par rehte
// hain (hit symmetric, column char metrics se, stacked pitch Mono + gap lock),
// copy-line actions Mono baseline se bhatakti nahi, copy button tall center
// par rehta hai. Geometry first: 24px XL hit floor + 4px XS slop har taraf =
// symmetric 32px target, tall center avail/2 (1200 -> 600) par, column 23 char
// metrics se (1 glyph x 7.0 advance + 2 x 8.0 section-gap bearings), pitch 19.6
// lock (15.6 Mono row + 4.0 tight gap). Typography second: copy-line action
// (Small 13.2 line) Mono row (15.6) me centered — dy 1.2 single source.
// Colors frozen.
// WIRING (paint): tall center `copy_tall_center_y_for(1200)`, arrow hit
// `copy_arrow_hit_rect(cx, cy)`, stacked pitch `copy_tall_arrow_pitch`,
// action y `copy_tall_action_y`, fit `copy_tall_column_fits`.
// ----------------------------------------------------------------------------

/// Tall center y px for a given avail height: `avail_h / 2`.
/// `None`: non-finite/non-positive avail. Pure; headless-tested.
pub fn copy_tall_center_y_for(avail_h: f32) -> Option<f32> {
    if !avail_h.is_finite() || avail_h <= 0.0 {
        return None;
    }
    let cy = avail_h / 2.0;
    if cy.is_finite() { Some(cy) } else { None }
}

/// Tall stacked-arrow pitch px: one Mono row + tight gap lock
/// (15.6 + 4.0 = 19.6). Tall single source ka pitch alias taaki 1200
/// tall rhythm wahi rule pakde. Pure; headless-tested.
pub fn copy_tall_arrow_pitch() -> Option<f32> {
    copy_arrow_default_pitch()
}

/// Tall copy-line action top y px for a row starting at `row_top`:
/// `row_top + dy` (dy 1.2 Mono-baseline centering). Non-finite par `None`.
/// Pure; headless-tested.
pub fn copy_tall_action_y(row_top: f32) -> Option<f32> {
    copy_line_action_y(row_top)
}

/// Tall column fit without h-scroll: arrow column (23) + slop hit (32)
/// `avail_w` me fit hote hain ya nahi. Narrow fit single source ka tall
/// alias taaki 720 wide rhythm wahi rule pakde. Pure; headless-tested.
pub fn copy_tall_column_fits(avail_w: f32) -> bool {
    copy_narrow_column_fits(avail_w)
}

#[cfg(test)]
mod exq_copy10_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, SPACE_XL, SPACE_XS, TypoRole};

    #[test]
    fn copy10_hit_rects_symmetric_from_space_scale_at_narrow_720() {
        // Geometry first: 24 XL floor + 4 XS slop har taraf = 32 square.
        assert_eq!(copy_hit_size(), SPACE_XL);
        assert_eq!(copy_hit_size(), COPY_HIT_MIN);
        assert_eq!(copy_button_hit_slop(), SPACE_XS);
        assert_eq!(copy_button_sym_pad(), SPACE_XS);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        assert_eq!(
            copy_hit_size_with_slop(),
            copy_hit_size() + 2.0 * COPY_BUTTON_HIT_SLOP
        );
        // Copy button 32px hit centered: sym pad single source.
        assert_eq!(copy_button_padded_size(), 32.0);
        assert_eq!(copy_button_padded_size(), copy_hit_size_with_slop());
        // Narrow center avail/2: 720 -> 360.
        assert_eq!(copy_center_x_for(720.0), Some(360.0));
        assert_eq!(copy_center_x_for(0.0), None);
        assert_eq!(copy_center_x_for(f32::NAN), None);
        // Symmetric hit rect at narrow center (360,300): 344..376 x 284..316.
        let rect = copy_arrow_hit_rect(360.0, 300.0);
        assert_eq!(rect, Some((344.0, 284.0, 376.0, 316.0)));
        assert_eq!(rect, copy_button_hit_rect(360.0, 300.0));
        assert!(copy_arrow_hit_is_symmetric(360.0, 300.0));
        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
            assert!(((360.0 - x0) - 16.0).abs() < 0.0001);
            assert!(((x1 - 360.0) - 16.0).abs() < 0.0001);
            assert!(((300.0 - y0) - 16.0).abs() < 0.0001);
            assert!(((y1 - 300.0) - 16.0).abs() < 0.0001);
        }
        // Edges inclusive, just outside false — tight panes me bhi generous.
        assert!(copy_arrow_hit_contains(360.0, 300.0, 344.0, 284.0));
        assert!(copy_arrow_hit_contains(360.0, 300.0, 376.0, 316.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 343.9, 300.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 376.1, 300.0));
        assert!(!copy_narrow_column_fits(f32::NAN));
        assert!(copy_narrow_column_fits(720.0));
    }

    #[test]
    fn copy10_column_width_char_metrics_and_pitch_lock_at_narrow() {
        // Arrow column x char metrics se: 1 glyph x 7.0 + 2 x 8.0 = 23.0.
        assert_eq!(copy_arrow_advance_px(), 7.0);
        assert_eq!(
            copy_arrow_column_width(),
            COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * GEO_SECTION_GAP
        );
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!(copy_hit_size_with_slop() > copy_arrow_column_width());
        // Stacked pitch lock: 15.6 Mono row + 4.0 tight gap = 19.6.
        let pitch = copy_narrow_arrow_pitch();
        assert!(pitch.is_some_and(|p| (p - 19.6).abs() < 0.01));
        assert_eq!(pitch, copy_arrow_default_pitch());
        assert_eq!(pitch, copy_wide_arrow_pitch());
        assert!(copy_narrow_column_fits(720.0));
    }

    #[test]
    fn copy10_line_action_dy_inside_mono_row_at_narrow() {
        // Typo second: Small action line 13.2 Mono row 15.6 me centered.
        assert_eq!(copy_line_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!((copy_action_text_line_height_px() - 13.2).abs() < 0.01);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert_eq!(copy_narrow_action_y(f32::NAN), None);
        assert_eq!(copy_narrow_action_y(f32::INFINITY), None);
        assert!(copy_narrow_action_y(100.0).is_some_and(|y| (y - 101.2).abs() < 0.01));
        assert_eq!(copy_narrow_action_y(100.0), copy_line_action_y(100.0));
        assert_eq!(copy_narrow_action_y(100.0), copy_wide_action_y(100.0));
    }

    #[test]
    fn copy10_narrow_fixture() {
        // Mirrors checks/gui-exq-copy10/round-1/comparison/copy-narrow-720.json.
        let raw =
            include_str!("../../../checks/gui-exq-copy10/round-1/comparison/copy-narrow-720.json");
        assert!(raw.contains("\"name\":\"copy-narrow-720\""));
        assert!(raw.contains("\"avail_w\":720.0"));
        assert!(raw.contains("\"arrow_column_width\":23.0"));
        assert!(raw.contains("\"arrow_hit_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"action_dy\":1.2"));
        assert!(raw.contains("\"arrow_pitch\":19.6"));
        assert_eq!(copy_center_x_for(720.0), Some(360.0));
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!((copy_hit_size_with_slop() - 32.0).abs() < 0.0001);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert!(copy_arrow_hit_is_symmetric(360.0, 300.0));
        assert!(copy_narrow_column_fits(720.0));
    }
}

#[cfg(test)]
mod exq_copy9_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, SPACE_XL, SPACE_XS, TypoRole};

    #[test]
    fn copy9_hit_rects_symmetric_from_space_scale_at_wide_960() {
        // Geometry first: 24 XL floor + 4 XS slop har taraf = 32 square.
        assert_eq!(copy_hit_size(), SPACE_XL);
        assert_eq!(copy_button_sym_pad(), SPACE_XS);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        // Wide center avail/2: 960 -> 480.
        assert_eq!(copy_center_x_for(960.0), Some(480.0));
        assert_eq!(copy_center_x_for(0.0), None);
        assert_eq!(copy_center_x_for(f32::NAN), None);
        // Symmetric hit rect at wide center (480,300): 464..496 x 284..316.
        let rect = copy_arrow_hit_rect(480.0, 300.0);
        assert_eq!(rect, Some((464.0, 284.0, 496.0, 316.0)));
        assert!(copy_arrow_hit_is_symmetric(480.0, 300.0));
        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
            assert!(((480.0 - x0) - 16.0).abs() < 0.0001);
            assert!(((x1 - 480.0) - 16.0).abs() < 0.0001);
            assert!(((300.0 - y0) - 16.0).abs() < 0.0001);
            assert!(((y1 - 300.0) - 16.0).abs() < 0.0001);
        }
        // Edges inclusive, just outside false.
        assert!(copy_arrow_hit_contains(480.0, 300.0, 464.0, 284.0));
        assert!(copy_arrow_hit_contains(480.0, 300.0, 496.0, 316.0));
        assert!(!copy_arrow_hit_contains(480.0, 300.0, 463.9, 300.0));
        assert!(!copy_arrow_hit_contains(480.0, 300.0, 496.1, 300.0));
        assert!(!copy_wide_column_fits(f32::NAN));
        assert!(copy_wide_column_fits(960.0));
    }

    #[test]
    fn copy9_line_actions_mono_baseline_aligned_at_wide() {
        // Typo second: Small action line 13.2 Mono row 15.6 me centered.
        assert_eq!(copy_line_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!((copy_action_text_line_height_px() - 13.2).abs() < 0.01);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert_eq!(copy_wide_action_y(f32::NAN), None);
        assert!(copy_wide_action_y(100.0).is_some_and(|y| (y - 101.2).abs() < 0.01));
    }

    #[test]
    fn copy9_column_width_char_metrics_and_wide_pitch() {
        // 1 glyph x 7.0 advance + 2 x 8.0 section-gap bearings = 23.0.
        assert_eq!(copy_arrow_advance_px(), 7.0);
        assert_eq!(
            copy_arrow_column_width(),
            COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * GEO_SECTION_GAP
        );
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!(copy_hit_size_with_slop() > copy_arrow_column_width());
        // Stacked rhythm: 15.6 Mono row + 4.0 tight gap = 19.6.
        let pitch = copy_wide_arrow_pitch();
        assert!(pitch.is_some_and(|p| (p - 19.6).abs() < 0.01));
        assert_eq!(pitch, copy_arrow_default_pitch());
        assert!(copy_wide_column_fits(960.0));
    }

    #[test]
    fn copy9_wide_fixture() {
        // Mirrors checks/gui-exq-copy9/round-1/comparison/copy-wide-960.json.
        let raw =
            include_str!("../../../checks/gui-exq-copy9/round-1/comparison/copy-wide-960.json");
        assert!(raw.contains("\"name\":\"copy-wide-960\""));
        assert!(raw.contains("\"avail_w\":960.0"));
        assert!(raw.contains("\"arrow_column_width\":23.0"));
        assert!(raw.contains("\"arrow_hit_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"action_dy\":1.2"));
        assert!(raw.contains("\"arrow_pitch\":19.6"));
        assert_eq!(copy_center_x_for(960.0), Some(480.0));
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!((copy_hit_size_with_slop() - 32.0).abs() < 0.0001);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert!(copy_arrow_hit_is_symmetric(480.0, 300.0));
        assert!(copy_wide_column_fits(960.0));
    }
}

#[cfg(test)]
mod exq_copy11_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, SPACE_XL, SPACE_XS, TypoRole};

    #[test]
    fn copy11_hit_rects_symmetric_centered_tall() {
        // Geometry first: 24 XL floor + 4 XS slop har taraf = 32 square.
        assert_eq!(copy_hit_size(), SPACE_XL);
        assert_eq!(copy_button_sym_pad(), SPACE_XS);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        // Tall center avail/2: 1200 -> 600; narrow width 720 -> 360.
        assert_eq!(copy_tall_center_y_for(1200.0), Some(600.0));
        assert_eq!(copy_center_x_for(720.0), Some(360.0));
        assert_eq!(copy_tall_center_y_for(0.0), None);
        assert_eq!(copy_tall_center_y_for(f32::NAN), None);
        assert_eq!(copy_tall_center_y_for(f32::INFINITY), None);
        // Symmetric hit rect at tall center (360,600): 344..376 x 584..616.
        let rect = copy_arrow_hit_rect(360.0, 600.0);
        assert_eq!(rect, Some((344.0, 584.0, 376.0, 616.0)));
        assert_eq!(rect, copy_button_hit_rect(360.0, 600.0));
        assert!(copy_arrow_hit_is_symmetric(360.0, 600.0));
        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
            assert!(((360.0 - x0) - 16.0).abs() < 0.0001);
            assert!(((x1 - 360.0) - 16.0).abs() < 0.0001);
            assert!(((600.0 - y0) - 16.0).abs() < 0.0001);
            assert!(((y1 - 600.0) - 16.0).abs() < 0.0001);
        }
        // Edges inclusive, just outside false — tall band me bhi generous.
        assert!(copy_arrow_hit_contains(360.0, 600.0, 344.0, 584.0));
        assert!(copy_arrow_hit_contains(360.0, 600.0, 376.0, 616.0));
        assert!(!copy_arrow_hit_contains(360.0, 600.0, 343.9, 600.0));
        assert!(!copy_arrow_hit_contains(360.0, 600.0, 376.1, 600.0));
        assert!(!copy_arrow_hit_contains(360.0, 600.0, 360.0, 583.9));
        assert!(!copy_arrow_hit_contains(360.0, 600.0, 360.0, 616.1));
        assert!(copy_tall_column_fits(720.0));
        assert!(!copy_tall_column_fits(f32::NAN));
    }

    #[test]
    fn copy11_pitch_locked_over_tall_canvas() {
        // Arrow column x char metrics se: 1 glyph x 7.0 + 2 x 8.0 = 23.0.
        assert_eq!(copy_arrow_advance_px(), 7.0);
        assert_eq!(
            copy_arrow_column_width(),
            COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * GEO_SECTION_GAP
        );
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!(copy_hit_size_with_slop() > copy_arrow_column_width());
        // Stacked pitch lock over tall canvas: 15.6 Mono row + 4.0 gap = 19.6.
        let pitch = copy_tall_arrow_pitch();
        assert!(pitch.is_some_and(|p| (p - 19.6).abs() < 0.01));
        assert_eq!(pitch, copy_arrow_default_pitch());
        assert_eq!(pitch, copy_narrow_arrow_pitch());
        assert_eq!(pitch, copy_wide_arrow_pitch());
        assert_eq!(pitch, copy_hunk_arrow_pitch(copy_row_height()));
        // Stacked positions stay on pitch: y(k) = 600 + k * 19.6.
        if let Some(p) = pitch {
            let y0 = 600.0;
            let y1 = y0 + p;
            let y2 = y0 + 2.0 * p;
            assert!(((y1 - y0) - 19.6).abs() < 0.01);
            assert!(((y2 - y1) - 19.6).abs() < 0.01);
            assert!(copy_arrow_hit_is_symmetric(360.0, y1));
            assert!(copy_arrow_hit_is_symmetric(360.0, y2));
        }
    }

    #[test]
    fn copy11_action_dy_steady_over_long_scroll() {
        // Typo second: Small action line 13.2 Mono row 15.6 me centered.
        assert_eq!(copy_line_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!((copy_action_text_line_height_px() - 13.2).abs() < 0.01);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert_eq!(copy_tall_action_y(f32::NAN), None);
        assert_eq!(copy_tall_action_y(f32::INFINITY), None);
        assert!(copy_tall_action_y(100.0).is_some_and(|y| (y - 101.2).abs() < 0.01));
        assert_eq!(copy_tall_action_y(100.0), copy_line_action_y(100.0));
        assert_eq!(copy_tall_action_y(100.0), copy_narrow_action_y(100.0));
        // Long scroll: dy steady at tall offsets (600, 1100).
        assert!(copy_tall_action_y(600.0).is_some_and(|y| (y - 601.2).abs() < 0.01));
        assert!(copy_tall_action_y(1100.0).is_some_and(|y| (y - 1101.2).abs() < 0.01));
    }

    #[test]
    fn copy11_tall_fixture() {
        // Mirrors checks/gui-exq-copy11/round-1/comparison/copy-tall-1200.json.
        let raw =
            include_str!("../../../checks/gui-exq-copy11/round-1/comparison/copy-tall-1200.json");
        assert!(raw.contains("\"name\":\"copy-tall-1200\""));
        assert!(raw.contains("\"avail_w\":720.0"));
        assert!(raw.contains("\"avail_h\":1200.0"));
        assert!(raw.contains("\"arrow_column_width\":23.0"));
        assert!(raw.contains("\"arrow_hit_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"action_dy\":1.2"));
        assert!(raw.contains("\"arrow_pitch\":19.6"));
        assert_eq!(copy_center_x_for(720.0), Some(360.0));
        assert_eq!(copy_tall_center_y_for(1200.0), Some(600.0));
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!((copy_hit_size_with_slop() - 32.0).abs() < 0.0001);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert!(copy_arrow_hit_is_symmetric(360.0, 600.0));
        assert!(copy_tall_column_fits(720.0));
    }
}

// ----------------------------------------------------------------------------
// EXQ-COPY12: narrow-720 hit bands + pitch lock + per-hunk action baseline.
// BC copy-side 720 narrow par bhi hit band centered rehta hai (slop hit
// symmetric, column char metrics se), stacked arrows locked pitch par ride
// karte hain scroll par bhi, per-hunk action Mono baseline par tikti hai.
// Geometry first: 24px XL hit floor + 4px XS slop har taraf = symmetric 32px
// target, narrow center avail/2 (720 -> 360) par, column 23 char metrics se
// (1 glyph x 7.0 advance + 2 x 8.0 section-gap bearings), pitch 19.6 lock
// (15.6 Mono row + 4.0 tight gap). Typography second: copy-line action
// (Small 13.2 line) Mono row (15.6) me centered — dy 1.2 single source.
// Colors frozen.
// WIRING (paint): narrow center `copy_plus_center_x_for(720)`, arrow hit
// `copy_arrow_hit_rect(cx, cy)`, stacked pitch `copy_plus_arrow_pitch`,
// action y `copy_plus_action_y`, fit `copy_plus_column_fits`.
// ----------------------------------------------------------------------------

/// Narrow-plus center x px for a given avail width: `avail_w / 2`.
/// Narrow single source delegate taaki 720 hit band wahi rule pakde.
/// `None`: non-finite/non-positive avail. Pure; headless-tested.
pub fn copy_plus_center_x_for(avail_w: f32) -> Option<f32> {
    copy_center_x_for(avail_w)
}

/// Narrow-plus stacked-arrow pitch px: one Mono row + tight gap lock
/// (15.6 + 4.0 = 19.6). Narrow single source delegate taaki 720 pitch
/// wahi rhythm pakde. Pure; headless-tested.
pub fn copy_plus_arrow_pitch() -> Option<f32> {
    copy_arrow_default_pitch()
}

/// Narrow-plus copy-line action top y px for a row starting at `row_top`:
/// `row_top + dy` (dy 1.2 Mono-baseline centering). Non-finite par `None`.
/// Pure; headless-tested.
pub fn copy_plus_action_y(row_top: f32) -> Option<f32> {
    copy_line_action_y(row_top)
}

/// Narrow-plus column fit without h-scroll: arrow column (23) + slop hit
/// (32) `avail_w` me fit hote hain ya nahi. Narrow fit single source ka
/// plus alias taaki 720 narrow rhythm wahi rule pakde. Pure; headless-tested.
pub fn copy_plus_column_fits(avail_w: f32) -> bool {
    copy_narrow_column_fits(avail_w)
}

#[cfg(test)]
mod exq_copy12_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, SPACE_XL, SPACE_XS, TypoRole};

    #[test]
    fn copy12_hit_bands_centered_at_narrow_720() {
        // Geometry first: 24 XL floor + 4 XS slop har taraf = 32 square.
        assert_eq!(copy_hit_size(), SPACE_XL);
        assert_eq!(copy_button_sym_pad(), SPACE_XS);
        assert_eq!(copy_hit_size_with_slop(), 32.0);
        // Narrow-plus center avail/2 delegate: 720 -> 360.
        assert_eq!(copy_plus_center_x_for(720.0), Some(360.0));
        assert_eq!(copy_plus_center_x_for(720.0), copy_center_x_for(720.0));
        assert_eq!(copy_plus_center_x_for(0.0), None);
        assert_eq!(copy_plus_center_x_for(f32::NAN), None);
        assert_eq!(copy_plus_center_x_for(f32::INFINITY), None);
        // Symmetric hit band at narrow center (360,300): 344..376 x 284..316.
        let rect = copy_arrow_hit_rect(360.0, 300.0);
        assert_eq!(rect, Some((344.0, 284.0, 376.0, 316.0)));
        assert!(copy_arrow_hit_is_symmetric(360.0, 300.0));
        if let Some((x0, y0, x1, y1)) = rect {
            assert_eq!(x1 - x0, 32.0);
            assert_eq!(y1 - y0, 32.0);
            assert!(((360.0 - x0) - 16.0).abs() < 0.0001);
            assert!(((x1 - 360.0) - 16.0).abs() < 0.0001);
            assert!(((300.0 - y0) - 16.0).abs() < 0.0001);
            assert!(((y1 - 300.0) - 16.0).abs() < 0.0001);
        }
        // Edges inclusive, just outside false — narrow band me bhi generous.
        assert!(copy_arrow_hit_contains(360.0, 300.0, 344.0, 284.0));
        assert!(copy_arrow_hit_contains(360.0, 300.0, 376.0, 316.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 343.9, 300.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 376.1, 300.0));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 360.0, 283.9));
        assert!(!copy_arrow_hit_contains(360.0, 300.0, 360.0, 316.1));
        assert!(copy_plus_column_fits(720.0));
        assert_eq!(copy_plus_column_fits(720.0), copy_narrow_column_fits(720.0));
        assert!(!copy_plus_column_fits(f32::NAN));
    }

    #[test]
    fn copy12_pitch_locked_and_action_baseline_narrow() {
        // Arrow column x char metrics se: 1 glyph x 7.0 + 2 x 8.0 = 23.0.
        assert_eq!(copy_arrow_advance_px(), 7.0);
        assert_eq!(
            copy_arrow_column_width(),
            COPY_ARROW_GLYPH_CHARS as f32 * copy_arrow_advance_px() + 2.0 * GEO_SECTION_GAP
        );
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!(copy_hit_size_with_slop() > copy_arrow_column_width());
        // Pitch lock narrow: 15.6 Mono row + 4.0 tight gap = 19.6, scroll steady.
        let pitch = copy_plus_arrow_pitch();
        assert!(pitch.is_some_and(|p| (p - 19.6).abs() < 0.01));
        assert_eq!(pitch, copy_arrow_default_pitch());
        assert_eq!(pitch, copy_narrow_arrow_pitch());
        assert_eq!(pitch, copy_wide_arrow_pitch());
        assert_eq!(pitch, copy_tall_arrow_pitch());
        assert_eq!(pitch, copy_hunk_arrow_pitch(copy_row_height()));
        // Stacked positions stay on pitch over scroll: y(k) = 300 + k * 19.6.
        if let Some(p) = pitch {
            let y0 = 300.0;
            let y1 = y0 + p;
            let y2 = y0 + 2.0 * p;
            assert!(((y1 - y0) - 19.6).abs() < 0.01);
            assert!(((y2 - y1) - 19.6).abs() < 0.01);
            assert!(copy_arrow_hit_is_symmetric(360.0, y1));
            assert!(copy_arrow_hit_is_symmetric(360.0, y2));
        }
        // Typo second: Small action line 13.2 Mono row 15.6 me centered.
        assert_eq!(copy_line_row_height(), TypoRole::Mono.line_height_px());
        assert!((copy_line_row_height() - 15.6).abs() < 0.01);
        assert!((copy_action_text_line_height_px() - 13.2).abs() < 0.01);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert_eq!(copy_plus_action_y(f32::NAN), None);
        assert_eq!(copy_plus_action_y(f32::INFINITY), None);
        assert!(copy_plus_action_y(100.0).is_some_and(|y| (y - 101.2).abs() < 0.01));
        assert_eq!(copy_plus_action_y(100.0), copy_line_action_y(100.0));
        assert_eq!(copy_plus_action_y(100.0), copy_narrow_action_y(100.0));
        // Narrow scroll: dy steady at offsets (300, 600, 1100).
        assert!(copy_plus_action_y(300.0).is_some_and(|y| (y - 301.2).abs() < 0.01));
        assert!(copy_plus_action_y(600.0).is_some_and(|y| (y - 601.2).abs() < 0.01));
        assert!(copy_plus_action_y(1100.0).is_some_and(|y| (y - 1101.2).abs() < 0.01));
    }

    #[test]
    fn copy12_narrow_fixture() {
        // Mirrors checks/gui-exq-copy12/round-1/comparison/copy-narrow-720.json.
        let raw =
            include_str!("../../../checks/gui-exq-copy12/round-1/comparison/copy-narrow-720.json");
        assert!(raw.contains("\"name\":\"copy-narrow-720\""));
        assert!(raw.contains("\"avail_w\":720.0"));
        assert!(raw.contains("\"arrow_column_width\":23.0"));
        assert!(raw.contains("\"arrow_hit_size\":32.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"action_dy\":1.2"));
        assert!(raw.contains("\"arrow_pitch\":19.6"));
        assert_eq!(copy_plus_center_x_for(720.0), Some(360.0));
        assert!((copy_arrow_column_width() - 23.0).abs() < 0.0001);
        assert!((copy_hit_size_with_slop() - 32.0).abs() < 0.0001);
        assert!((copy_line_action_dy() - 1.2).abs() < 0.01);
        assert!(copy_arrow_hit_is_symmetric(360.0, 300.0));
        assert!(copy_plus_column_fits(720.0));
        // Head start reused, not redone: tall fixture present alongside.
        let tall =
            include_str!("../../../checks/gui-exq-copy12/round-1/comparison/copy-tall-1200.json");
        assert!(tall.contains("\"name\":\"copy-tall-1200\""));
        assert!(tall.contains("\"arrow_pitch\":19.6"));
    }
}
