//! Thumbnail gutter geometry calculations.
//!
//! Pure mapping of diff row ranges to vertical gutter bar segments.
//! Bar widths, minimum segment height and merge gap come from the
//! spacing scale (4/8/12/16/24) so markers, separator and text rows
//! stay aligned.
//!
//! Two widths exist: the legacy paint bar ([`gutter_bar_width`], 12/6,
//! kept so `views.rs`/`views_hex.rs` chrome and tests stay stable) and
//! the overview ruler ([`gutter_ruler_width`], 16/8) that paint adopts
//! for the Beyond Compare-style ruler: wider track, taller minimum
//! segments and a slop-expanded hit test. Both sit on the same scale;
//! only `gutter.rs` owns them.

#![forbid(unsafe_code)]

use crate::viewmodel::{TextRow, TextRowKind};

/// Overview-ruler track width with line numbers: one LG step.
/// Reuses [`crate::theme::SPACE_LG`] so the ruler never drifts off scale.
///
/// Layout budget (960px canvas): 12 margin + 460 pane + 4 gap + 460
/// pane + 16 ruler + 8 margin = 960. The ruler absorbs the old 4px
/// inter-chrome gap plus 2px of right margin, so panes keep 460px.
pub const GUTTER_RULER_WIDE: f32 = crate::theme::SPACE_LG;

/// Overview-ruler track width without line numbers: one SM step.
/// Reuses [`crate::theme::SPACE_SM`].
///
/// Same budget as [`GUTTER_RULER_WIDE`]: +2px over the legacy 6px bar,
/// absorbed by the right margin. Narrow stays under wide.
pub const GUTTER_RULER_NARROW: f32 = crate::theme::SPACE_SM;

/// Minimum overview-ruler segment height: one XS step.
/// Reuses [`crate::theme::SPACE_XS`].
///
/// Single-row hunks in tall files used to collapse to 2px dots with no
/// click target; 4px keeps every hunk visible and pointer-hittable.
/// Consecutive diff rows still fuse via the merge gap below.
pub const GUTTER_SEGMENT_MIN_H: f32 = crate::theme::SPACE_XS;

/// Hit-test slop: each segment grows by this many px per side so
/// adjacent 4px bands stay separately clickable with an ~8px target.
/// Reuses [`crate::theme::GUTTER_MERGE_GAP`] (same 2px beat as segment
/// fusion). Geometry only; colors untouched.
pub const GUTTER_HIT_SLOP: f32 = crate::theme::GUTTER_MERGE_GAP;

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
/// Thin hunks grow to [`GUTTER_SEGMENT_MIN_H`] so every difference
/// stays visible and clickable on the ruler; segment colors still
/// come from paint (frozen), never from here.
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
        if y_end < y_start + GUTTER_SEGMENT_MIN_H {
            y_end = (y_start + GUTTER_SEGMENT_MIN_H).min(height);
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

/// Overview-ruler track width in logical pixels: [`GUTTER_RULER_WIDE`]
/// with line numbers, [`GUTTER_RULER_NARROW`] without. The Beyond
/// Compare-style ruler paint adopts this (wider than the legacy
/// [`gutter_bar_width`] bar); the allocate rect doubles as the click
/// target so no extra hit rect is needed. Pure; unit-tested.
pub fn gutter_ruler_width(show_line_numbers: bool) -> f32 {
    if show_line_numbers {
        GUTTER_RULER_WIDE
    } else {
        GUTTER_RULER_NARROW
    }
}

/// Ruler width scaled from the mono font size. Calibrated so
/// [`crate::theme::TypoRole::content_grid`] (12.0) reproduces
/// [`gutter_ruler_width`]; other sizes scale linearly. Non-finite or
/// non-positive sizes fall back to the unscaled width so paint never
/// feeds NaN into layout. Pure; unit-tested.
pub fn gutter_ruler_width_for_font(font_size: f32, show_line_numbers: bool) -> f32 {
    let grid_size = crate::theme::TypoRole::content_grid().size();
    let base = gutter_ruler_width(show_line_numbers);
    if font_size.is_finite() && font_size > 0.0 && grid_size > 0.0 {
        base * (font_size / grid_size)
    } else {
        base
    }
}
/// Gutter text role: grid content (fixed mono 12) so gutter-adjacent chrome reads
/// in the same family as pane rows. Adopts the [`crate::theme::TypoRole::content_grid`]
/// alias (== Mono, value-stable). Typography only; bar widths stay
/// in [`gutter_bar_width`]/[`gutter_bar_width_for_font`], ruler widths
/// in [`gutter_ruler_width`]/[`gutter_ruler_width_for_font`].
pub const fn gutter_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::content_grid()
}

/// Gutter text size: the shared grid token via alias (single source, 12.0).
pub const fn gutter_text_size() -> f32 {
    crate::theme::TypoRole::content_grid().size()
}

/// Gutter line-height source: the shared measured mono row height.
pub const fn gutter_line_height_source() -> &'static str {
    crate::theme::TYPO_LINE_HEIGHT_SOURCE
}

/// Gutter width scaled from the mono font size. Calibrated so
/// [`crate::theme::TypoRole::content_grid`] (12.0) reproduces
/// [`gutter_bar_width`]; other sizes scale linearly. Non-finite or
/// non-positive sizes fall back to the unscaled width so paint never
/// feeds NaN into layout. Pure; unit-tested.
pub fn gutter_bar_width_for_font(font_size: f32, show_line_numbers: bool) -> f32 {
    let grid_size = crate::theme::TypoRole::content_grid().size();
    let base = gutter_bar_width(show_line_numbers);
    if font_size.is_finite() && font_size > 0.0 && grid_size > 0.0 {
        base * (font_size / grid_size)
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
    let frac = y / height;
    if !frac.is_finite() || frac < 0.0 || frac >= 1.0 {
        return None;
    }
    gutter_row_from_frac(frac, total_rows)
}

/// Hit-tests a gutter-local y against already-computed `segments`.
///
/// Exact test (zero slop): the first segment with
/// `y_start <= y < y_end` (the final segment is inclusive of its end
/// so clicks on its bottom edge land). `None` for non-finite y or no
/// containing segment. Pure; unit-tested.
pub fn gutter_segment_at_y(segments: &[GutterSegment], y: f32) -> Option<usize> {
    gutter_segment_at_y_with_slop(segments, y, 0.0)
}

/// Slop-expanded hit test for the overview ruler.
///
/// Each segment grows by `slop` px on both sides (low side clamped at
/// 0.0) so 4px bands stay finger-clickable without merging their
/// paint. Non-finite or non-positive slop behaves exactly like
/// [`gutter_segment_at_y`]. First match wins, final segment keeps its
/// inclusive end. Pure; unit-tested.
pub fn gutter_segment_at_y_with_slop(
    segments: &[GutterSegment],
    y: f32,
    slop: f32,
) -> Option<usize> {
    if !y.is_finite() {
        return None;
    }
    let pad = if slop.is_finite() && slop > 0.0 {
        slop
    } else {
        0.0
    };
    let mut idx = 0usize;
    for seg in segments {
        let last = idx.saturating_add(1) >= segments.len();
        let lo = (seg.y_start - pad).max(0.0);
        let hi = seg.y_end + pad;
        let inside = if last {
            y >= lo && y <= hi
        } else {
            y >= lo && y < hi
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

/// Maps a model row index back to its gutter-local y (px).
///
/// Inverse of [`gutter_row_at_y`]: `(row / total_rows) * height`.
/// `None` for empty totals, non-positive or non-finite heights, or
/// out-of-range rows. Paint uses it for the selection/viewport
/// marker so the indicator sits exactly on the clicked row's band.
/// Pure; unit-tested.
pub fn gutter_y_for_row(row: usize, total_rows: usize, height: f32) -> Option<f32> {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return None;
    }
    if row >= total_rows {
        return None;
    }
    let y = (row as f32 / total_rows as f32) * height;
    if y.is_finite() && y >= 0.0 && y < height {
        Some(y)
    } else {
        None
    }
}

/// Band center for a model row: `((row + 0.5) / total_rows) * height`.
///
/// Companion to [`gutter_y_for_row`] (band top): markers centered on
/// the band read aligned on 4px ruler segments, and
/// `gutter_row_at_y(center) == row` round-trips so offset<->row holds
/// in both directions. Same `None` contract as [`gutter_y_for_row`].
/// Pure; unit-tested.
pub fn gutter_row_center_y(row: usize, total_rows: usize, height: f32) -> Option<f32> {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return None;
    }
    if row >= total_rows {
        return None;
    }
    let y = ((row as f32 + 0.5) / total_rows as f32) * height;
    if y.is_finite() && y >= 0.0 && y < height {
        Some(y)
    } else {
        None
    }
}

/// Selection-marker band `(y0, y1)` px: the row band center
/// ([`gutter_row_center_y`]) grown to one [`GUTTER_SEGMENT_MIN_H`] so the
/// tick exactly covers a min band and never straddles two rows.
/// Clamped to `[0.0, height]`. `None` for empty totals, non-positive or
/// non-finite heights, or out-of-range rows. Pure; unit-tested.
pub fn gutter_selection_marker_band(
    row: usize,
    total_rows: usize,
    height: f32,
) -> Option<(f32, f32)> {
    let cy = gutter_row_center_y(row, total_rows, height)?;
    let half = GUTTER_SEGMENT_MIN_H / 2.0;
    let mut y0 = cy - half;
    let mut y1 = cy + half;
    if y0 < 0.0 {
        y0 = 0.0;
        y1 = GUTTER_SEGMENT_MIN_H.min(height);
    }
    if y1 > height {
        y1 = height;
        y0 = (height - GUTTER_SEGMENT_MIN_H).max(0.0);
    }
    if y0.is_finite() && y1.is_finite() && y0 < y1 {
        Some((y0, y1))
    } else {
        None
    }
}

/// Computes thumbnail gutter bar geometry for three-way merge lines containing conflict markers.
///
/// Scans lines for `<<<<<<<` to `>>>>>>>` spans and maps each conflict region to a
/// [`GutterSegment`] with kind [`TextRowKind::Changed`], respecting [`GUTTER_SEGMENT_MIN_H`]
/// and [`crate::theme::GUTTER_MERGE_GAP`].
pub fn merge_gutter_geometry(lines: &[String], height: f32) -> GutterGeometry {
    if lines.is_empty() || !height.is_finite() || height <= 0.0 {
        return GutterGeometry {
            total_height: if height.is_finite() {
                height.max(0.0)
            } else {
                0.0
            },
            segments: Vec::new(),
        };
    }

    let ranges = merge_conflict_line_ranges(lines);
    let total_f = lines.len() as f32;
    let mut segments = Vec::new();

    for (start, end) in ranges {
        let y_start = (start as f32 / total_f) * height;
        let mut y_end = (end as f32 / total_f) * height;
        if y_end < y_start + GUTTER_SEGMENT_MIN_H {
            y_end = (y_start + GUTTER_SEGMENT_MIN_H).min(height);
        }

        if let Some(last) = segments.last_mut() {
            let last_seg: &mut GutterSegment = last;
            if y_start <= last_seg.y_end + crate::theme::GUTTER_MERGE_GAP {
                last_seg.y_end = last_seg.y_end.max(y_end);
                continue;
            }
        }

        segments.push(GutterSegment {
            y_start,
            y_end,
            kind: TextRowKind::Changed,
        });
    }

    GutterGeometry {
        total_height: height,
        segments,
    }
}

/// Finds all half-open `[start, end)` line ranges of conflict blocks in `lines`.
pub fn merge_conflict_line_ranges(lines: &[String]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut in_conflict = false;
    let mut start_idx = 0usize;

    for (idx, line) in lines.iter().enumerate() {
        if line.starts_with("<<<<<<<") && !in_conflict {
            in_conflict = true;
            start_idx = idx;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            in_conflict = false;
            ranges.push((start_idx, idx.saturating_add(1)));
        }
    }
    if in_conflict {
        ranges.push((start_idx, lines.len()));
    }
    ranges
}

/// Pure helper mapping a ruler y-coordinate directly to a 0-based row index clamped to `0..total_rows`.
pub fn gutter_ruler_hit_test(y: f32, height: f32, total_rows: usize) -> Option<usize> {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 || !y.is_finite() || y < 0.0 {
        return None;
    }
    let frac = (y / height).clamp(0.0, 1.0);
    gutter_row_from_frac(frac, total_rows)
}

// ----------------------------------------------------------------------------
// Viewport marker geometry (headless; paint adoption `views.rs` owner ke paas).
// Ruler par visible scroll window ka band: top_row..top+visible map hota
// hai [0,height] me. Min-height floor taaki chhota window bhi dikhe.
// ----------------------------------------------------------------------------

/// Viewport marker min height px: one XS step (ruler segment floor parity).
pub const GUTTER_VIEWPORT_MIN_H: f32 = crate::theme::SPACE_XS;

/// Visible scroll window ka ruler band `(y0, y1)`: `top_row` se
/// `top_row + visible_rows` tak `[0.0, total]` clamp karke map.
/// Band min-height floor rakhta hai (neeche se katkar, height ke andar).
/// `None`: khali totals, non-positive/non-finite height, ya window khaali.
/// Pure; unit-tested.
pub fn gutter_viewport_rect(
    total_rows: usize,
    height: f32,
    top_row: usize,
    visible_rows: usize,
) -> Option<(f32, f32)> {
    if total_rows == 0 || visible_rows == 0 {
        return None;
    }
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    let total_f = total_rows as f32;
    let top = top_row.min(total_rows) as f32;
    let bottom = top_row.saturating_add(visible_rows).min(total_rows) as f32;
    if bottom <= top {
        return None;
    }
    let y0 = (top / total_f) * height;
    let mut y1 = (bottom / total_f) * height;
    if y1 < y0 + GUTTER_VIEWPORT_MIN_H {
        y1 = (y0 + GUTTER_VIEWPORT_MIN_H).min(height);
    }
    if !y0.is_finite() || !y1.is_finite() || y0 < 0.0 || y1 > height || y0 >= y1 {
        return None;
    }
    Some((y0, y1))
}

// ----------------------------------------------------------------------------
// Virtualized range geometry (headless; O(ranges), no per-row vec).
// Tall inputs virtualize: callers pass half-open `[start, end)` model-row
// spans with one kind (hunk/diff spans) instead of materializing one
// entry per row, so a 1M-row file maps from a handful of ranges.
// Floors, merge gaps and the ruler hit contract match
// [`gutter_geometry`] exactly; `ranges` in row order give identical
// segments. Pure; unit-tested.
// ----------------------------------------------------------------------------

/// Computes thumbnail gutter bars directly from diff row ranges.
///
/// Each `(start, end, kind)` names a half-open span of model rows sharing
/// one [`TextRowKind`]. Spans clamp to `total_rows`; empty or fully
/// out-of-range spans contribute nothing. Band math (fraction mapping,
/// [`GUTTER_SEGMENT_MIN_H`] floor, [`crate::theme::GUTTER_MERGE_GAP`]
/// fusion) mirrors [`gutter_geometry`] so virtualized paint and the
/// row-wise path never disagree. Pure; unit-tested.
pub fn gutter_geometry_from_ranges(
    total_rows: usize,
    height: f32,
    ranges: &[(usize, usize, TextRowKind)],
) -> GutterGeometry {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return GutterGeometry {
            total_height: height.max(0.0),
            segments: Vec::new(),
        };
    }
    let mut segments: Vec<GutterSegment> = Vec::new();
    let total_f = total_rows as f32;
    for (start, end, kind) in ranges {
        if *start >= *end || *start >= total_rows {
            continue;
        }
        let stop = (*end).min(total_rows);
        if stop <= *start {
            continue;
        }
        let y_start = (*start as f32 / total_f) * height;
        let mut y_end = (stop as f32 / total_f) * height;
        if y_end < y_start + GUTTER_SEGMENT_MIN_H {
            y_end = (y_start + GUTTER_SEGMENT_MIN_H).min(height);
        }
        if let Some(last) = segments.last_mut() {
            let last_seg: &mut GutterSegment = last;
            if last_seg.kind == *kind && y_start <= last_seg.y_end + crate::theme::GUTTER_MERGE_GAP
            {
                last_seg.y_end = last_seg.y_end.max(y_end);
                continue;
            }
        }
        segments.push(GutterSegment {
            y_start,
            y_end,
            kind: *kind,
        });
    }
    GutterGeometry {
        total_height: height,
        segments,
    }
}

// ----------------------------------------------------------------------------
// Center-gutter arrows per hunk (headless; paint owner ke paas).
// BC twin panes ke beech har hunk par ek directional arrow rehta hai, har
// row par nahi — continuation rows blank click-to-select rehti hain taaki
// gutter fixed width me rahe. Geometry first: column width one XL step
// (24.0, [`crate::copy::COPY_HIT_MIN`] parity — koi chhota tap target
// nahi), arrow box one LG step (16.0) center me. Har segment (= ek hunk)
// ka ek center ([`gutter_arrow_center_y`]) hota hai; paint arrow box
// [`gutter_arrow_rect`] me banata hai. Kaunsa row arrow pata hai yeh
// [`crate::copy::copy_hunk_head`] batata hai (head-only visibility).
// Colors frozen: yahan koi hue nahi, sirf shape.
// WIRING (paint): center column `gutter_center_width()` chaudi; har
// segment ke `gutter_arrow_center_y(&seg)` par `gutter_arrow_rect(cx, cy)`
// me arrow; `copy_hunk_head` false wali rows blank (click-to-select).
// ----------------------------------------------------------------------------

/// Center-gutter column width px: one XL step (copy hit floor parity).
pub const GUTTER_CENTER_W: f32 = crate::theme::SPACE_XL;

/// Center arrow box size px: one LG step (24px column me 4px pad/side).
pub const GUTTER_ARROW_BOX: f32 = crate::theme::SPACE_LG;

/// Center-gutter column width px (single source). Pure; unit-tested.
pub fn gutter_center_width() -> f32 {
    GUTTER_CENTER_W
}

/// Center arrow box size px (single source). Pure; unit-tested.
pub fn gutter_arrow_box() -> f32 {
    GUTTER_ARROW_BOX
}

/// Ek hunk segment ke arrow ka center y px: band ka midpoint.
/// Non-finite band par `None` taaki paint NaN kabhi layout me na daale.
/// Pure; unit-tested.
pub fn gutter_arrow_center_y(seg: &GutterSegment) -> Option<f32> {
    if !seg.y_start.is_finite() || !seg.y_end.is_finite() {
        return None;
    }
    let cy = (seg.y_start + seg.y_end) / 2.0;
    if cy.is_finite() { Some(cy) } else { None }
}

/// Center arrow ka square rect `(x0, y0, x1, y1)` px: `GUTTER_ARROW_BOX`
/// ka box `(center_x, center_y)` par centered. Non-finite input par
/// `None`. Pure; unit-tested.
pub fn gutter_arrow_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    if !center_x.is_finite() || !center_y.is_finite() {
        return None;
    }
    let half = GUTTER_ARROW_BOX / 2.0;
    Some((
        center_x - half,
        center_y - half,
        center_x + half,
        center_y + half,
    ))
}

/// Har segment ke arrow center y px, segment order me. Non-finite band
/// skip hota hai (paint uss hunk par arrow nahi banata). Pure; unit-tested.
pub fn gutter_arrow_centers(segments: &[GutterSegment]) -> Vec<f32> {
    let mut out = Vec::new();
    for seg in segments {
        if let Some(cy) = gutter_arrow_center_y(seg) {
            out.push(cy);
        }
    }
    out
}

// ----------------------------------------------------------------------------
// GUTTER6: line-number column + minimap overview (headless).
// Twin-pane gutter me line numbers ek fixed tabular column me hote
// hain (digit count ke saath chauda hota hai), aur right-edge
// thumbnail overview me same/diff dono ke bands ek hi band-math se
// bante hain taaki same aur diff segment heights match karen. Center
// arrows ke tap-targets arrow box se bade hote hain (slop-expanded
// hit rect). Geometry first: sab SPACE scale reuse, koi naya literal
// nahi. Typography second: numbers existing content_grid (Mono 12
// tabular) role me, koi naya size nahi. Colors frozen: sirf shape.
// WIRING (paint owner ke paas): numbers column
// `gutter_linenum_column_width(max_no, show)` chaudi; text rows
// `gutter_text_row_height(line_h)` unche; overview
// `gutter_overview_geometry(..)` se same+diff bands; arrow hit
// `gutter_arrow_hit_contains(..)` se.
// ----------------------------------------------------------------------------

/// Line-number column per-extra-digit step: one XS step.
///
/// Column `GUTTER_RULER_WIDE` (16) se shuru hota hai aur har extra
/// digit par 4px badhta hai (20/24/28/..), taaki 1-digit files patli
/// aur 4-digit files chaudi column paen bina scale chhode.
pub const GUTTER_LINENUM_PAD: f32 = crate::theme::SPACE_XS;

/// Text-row vertical pad per side: merge-gap reuse (2px).
///
/// Text row height = measured line height + 2 * pad, taaki padding
/// text rows se hamesha ek fixed beat par rahe (koi ad-hoc pad nahi).
pub const GUTTER_TEXT_ROW_PAD: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Center-arrow hit-test slop per side: merge-gap reuse (2px).
///
/// 16px arrow box 20px tap-target banta hai bina paint hile; ruler
/// slop ([`GUTTER_HIT_SLOP`]) se alag beat par, same scale par.
pub const GUTTER_ARROW_HIT_SLOP: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Line-number column extra-digit cap: 5 steps (16 -> 36 max).
///
/// Bahut badi files par column poori pane na kha le; digits isse
/// zyada hon to width cap par atki rehti hai (numbers clip nahi,
/// column bas aur chaudi nahi hoti).
pub const GUTTER_LINENUM_MAX_EXTRA: usize = 5;

/// Line-number trailing pad (chars): number column aur code ke beech
/// do spaces ([`crate::views::CONTENT_LEAD`] two-space beat parity —
/// number gutter code se chipke na lage). `views::gutter_text` isi se
/// right-aligned number + trail banata hai; blank sides utne hi spaces
/// rakhte hain taaki column rhythm na toote. Pure; unit-tested.
pub const GUTTER_NUMBER_TRAIL_PAD: usize = 2;

/// Number-cell text: right-aligned number in `width` plus the trailing
/// pad, or blanks when the row has no such side. Single source for
/// `views::gutter_text` plus the wrap continuation/marker gutters so the
/// number column keeps one tabular width (digits seedh me rahen, code se
/// chipke nahi). Pure; unit-tested.
pub fn gutter_number_cell(no: Option<usize>, width: usize) -> String {
    let trail = " ".repeat(GUTTER_NUMBER_TRAIL_PAD);
    match no {
        Some(n) => format!("{n:>width$}{trail}"),
        None => format!("{}{trail}", " ".repeat(width)),
    }
}

/// Number-cell right-aligned tabular contract: numbers right-align in a
/// fixed Mono tabular column ([`gutter_linenum_is_tabular`]). Paint never
/// left-pads or proportionally drifts. Pure; unit-tested.
pub fn gutter_number_cell_is_right_aligned_tabular() -> bool {
    gutter_linenum_is_tabular()
}

/// Line-number column ka decimal digit count: `max_line_no` me kitne
/// digits hain (`0` par bhi 1, khali file me bhi ek column). Pure.
pub fn gutter_linenum_digits(max_line_no: usize) -> usize {
    let mut digits = 1usize;
    let mut n = max_line_no;
    while n >= 10 {
        n /= 10;
        digits = digits.saturating_add(1);
    }
    digits
}

/// Line-number column width px: numbers chhupe par ruler narrow track
/// ([`GUTTER_RULER_NARROW`]), dikhen par [`GUTTER_RULER_WIDE`] + har
/// extra digit par [`GUTTER_LINENUM_PAD`] ([`GUTTER_LINENUM_MAX_EXTRA`]
/// cap). Sab 4-scale par (16/20/24/28/32/36). Pure; unit-tested.
pub fn gutter_linenum_column_width(max_line_no: usize, show_line_numbers: bool) -> f32 {
    if !show_line_numbers {
        return GUTTER_RULER_NARROW;
    }
    let extra = gutter_linenum_digits(max_line_no)
        .saturating_sub(1)
        .min(GUTTER_LINENUM_MAX_EXTRA) as f32;
    GUTTER_RULER_WIDE + extra * GUTTER_LINENUM_PAD
}

/// Line-number text role: grid content (fixed mono 12 tabular), wahi
/// role jo gutter text padhta hai. Typography second, koi naya size
/// nahi. Pure; unit-tested.
pub const fn gutter_linenum_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::content_grid()
}

/// Line-number column tabular hai (digits seedh me rahen). Pure.
pub const fn gutter_linenum_is_tabular() -> bool {
    crate::theme::TypoRole::content_grid().is_tabular()
}

/// Text-row vertical pad px per side (single source). Pure.
pub fn gutter_text_row_pad() -> f32 {
    GUTTER_TEXT_ROW_PAD
}

/// Padding samet text-row height px: `line_height_px + 2 * pad`.
///
/// `None`: non-finite/non-positive line height ya overflow. Paint isi
/// se row unchai banata hai taaki padding vs text rows ek beat par
/// rahen. Pure; unit-tested.
pub fn gutter_text_row_height(line_height_px: f32) -> Option<f32> {
    if !line_height_px.is_finite() || line_height_px <= 0.0 {
        return None;
    }
    let h = line_height_px + 2.0 * GUTTER_TEXT_ROW_PAD;
    if h.is_finite() && h > 0.0 {
        Some(h)
    } else {
        None
    }
}

/// Thumbnail overview geometry: same+diff dono ke bands ek hi band-math
/// se ([`gutter_geometry`] jaisi fraction mapping, [`GUTTER_SEGMENT_MIN_H`]
/// floor, [`crate::theme::GUTTER_MERGE_GAP`] fusion), farq sirf itna ki
/// Equal runs skip nahi hote — woh background bands (kind Equal) bante
/// hain taaki paint same/diff heights ek scale par khinche. Kinds alag
/// hone par merge nahi hota, isliye bands ruler ko gap-bina dhakte
/// hain. Pure; unit-tested.
pub fn gutter_overview_geometry(
    total_rows: usize,
    height: f32,
    rows: &[TextRow],
) -> GutterGeometry {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return GutterGeometry {
            total_height: if height.is_finite() {
                height.max(0.0)
            } else {
                0.0
            },
            segments: Vec::new(),
        };
    }
    let mut segments: Vec<GutterSegment> = Vec::new();
    let total_f = total_rows as f32;
    let mut idx = 0usize;
    while idx < rows.len() && idx < total_rows {
        let kind = match rows.get(idx) {
            Some(row) => row.kind,
            None => break,
        };
        let mut end = idx.saturating_add(1);
        while end < rows.len() && end < total_rows {
            match rows.get(end) {
                Some(next) if next.kind == kind => {
                    end = end.saturating_add(1);
                }
                _ => break,
            }
        }
        let y_start = (idx as f32 / total_f) * height;
        let mut y_end = (end as f32 / total_f) * height;
        if y_end < y_start + GUTTER_SEGMENT_MIN_H {
            y_end = (y_start + GUTTER_SEGMENT_MIN_H).min(height);
        }
        if let Some(last) = segments.last_mut() {
            let last_seg: &mut GutterSegment = last;
            if last_seg.kind == kind && y_start <= last_seg.y_end + crate::theme::GUTTER_MERGE_GAP {
                last_seg.y_end = last_seg.y_end.max(y_end);
                idx = end;
                continue;
            }
        }
        segments.push(GutterSegment {
            y_start,
            y_end,
            kind,
        });
        idx = end;
    }
    GutterGeometry {
        total_height: height,
        segments,
    }
}

/// Center arrow ka slop-expanded hit rect `(x0, y0, x1, y1)` px:
/// [`gutter_arrow_rect`] box har taraf [`GUTTER_ARROW_HIT_SLOP`] badha
/// hua (16px box -> 20px target). Non-finite input par `None` taaki
/// paint NaN kabhi layout me na daale. Pure; unit-tested.
pub fn gutter_arrow_hit_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let (x0, y0, x1, y1) = gutter_arrow_rect(center_x, center_y)?;
    let s = GUTTER_ARROW_HIT_SLOP;
    let (lx0, ly0, lx1, ly1) = (x0 - s, y0 - s, x1 + s, y1 + s);
    if lx0.is_finite() && ly0.is_finite() && lx1.is_finite() && ly1.is_finite() {
        Some((lx0, ly0, lx1, ly1))
    } else {
        None
    }
}

/// Arrow hit-test: point `(x, y)` slop-expanded hit rect ke andar hai
/// ya nahi (edges inclusive). Non-finite par false. Pure; unit-tested.
pub fn gutter_arrow_hit_contains(center_x: f32, center_y: f32, x: f32, y: f32) -> bool {
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    match gutter_arrow_hit_rect(center_x, center_y) {
        Some((x0, y0, x1, y1)) => x >= x0 && x <= x1 && y >= y0 && y <= y1,
        None => false,
    }
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
    fn gutter_text_typo_follows_mono_tokens() {
        use crate::theme::{TYPO_LINE_HEIGHT_SOURCE, TYPO_MONO_SIZE, TypoRole};
        assert_eq!(gutter_text_role(), TypoRole::content_grid());
        assert_eq!(gutter_text_role(), TypoRole::Mono);
        assert!(gutter_text_role().is_mono());
        assert!(!gutter_text_role().is_strong());
        assert_eq!(gutter_text_size(), TypoRole::content_grid().size());
        assert_eq!(gutter_text_size(), TYPO_MONO_SIZE);
        assert_eq!(gutter_text_size(), 12.0);
        assert_eq!(gutter_line_height_source(), TYPO_LINE_HEIGHT_SOURCE);
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

    #[test]
    fn y_for_row_inverts_row_at_y() {
        // 4 rows over 400px: each row owns a 100px band.
        assert_eq!(gutter_y_for_row(0, 4, 400.0), Some(0.0));
        assert_eq!(gutter_y_for_row(1, 4, 400.0), Some(100.0));
        assert_eq!(gutter_y_for_row(3, 4, 400.0), Some(300.0));
        // Bad input maps nothing.
        assert_eq!(gutter_y_for_row(4, 4, 400.0), None);
        assert_eq!(gutter_y_for_row(0, 0, 400.0), None);
        assert_eq!(gutter_y_for_row(1, 4, 0.0), None);
        assert_eq!(gutter_y_for_row(1, 4, -50.0), None);
        assert_eq!(gutter_y_for_row(1, 4, f32::NAN), None);
        // Round-trip: marker y falls back inside the same row band.
        let mut row = 0usize;
        while row < 4 {
            if let Some(y) = gutter_y_for_row(row, 4, 400.0) {
                assert_eq!(gutter_row_at_y(y + 0.5, 4, 400.0), Some(row));
            }
            row = row.saturating_add(1);
        }
    }

    #[test]
    fn ruler_tokens_match_spacing_scale() {
        use crate::theme::{GUTTER_MERGE_GAP, SPACE_LG, SPACE_SM, SPACE_XS};
        assert_eq!(GUTTER_RULER_WIDE, SPACE_LG);
        assert_eq!(GUTTER_RULER_WIDE, 16.0);
        assert_eq!(GUTTER_RULER_NARROW, SPACE_SM);
        assert_eq!(GUTTER_RULER_NARROW, 8.0);
        assert_eq!(GUTTER_SEGMENT_MIN_H, SPACE_XS);
        assert_eq!(GUTTER_SEGMENT_MIN_H, 4.0);
        assert_eq!(GUTTER_HIT_SLOP, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_HIT_SLOP, 2.0);
        // Ladder: narrow bar < narrow ruler < wide bar < wide ruler,
        // segments above the old 2px dot floor.
        const {
            assert!(GUTTER_RULER_NARROW < GUTTER_RULER_WIDE);
            assert!(crate::theme::GUTTER_BAR_NARROW < GUTTER_RULER_NARROW);
            assert!(crate::theme::GUTTER_BAR_WIDE < GUTTER_RULER_WIDE);
            assert!(crate::theme::GUTTER_MIN_SEGMENT_H < GUTTER_SEGMENT_MIN_H);
        }
    }

    #[test]
    fn ruler_width_widens_track_but_stays_narrower_without_numbers() {
        assert_eq!(gutter_ruler_width(true), 16.0);
        assert_eq!(gutter_ruler_width(false), 8.0);
        assert!(gutter_ruler_width(false) < gutter_ruler_width(true));
        // Legacy bar stays put so views/hex chrome tests hold.
        assert!(gutter_bar_width(true) < gutter_ruler_width(true));
        assert!(gutter_bar_width(false) < gutter_ruler_width(false));
    }

    #[test]
    fn ruler_width_for_font_scales_from_mono_size() {
        use crate::theme::TYPO_MONO_SIZE;
        assert_eq!(gutter_ruler_width_for_font(TYPO_MONO_SIZE, true), 16.0);
        assert_eq!(gutter_ruler_width_for_font(TYPO_MONO_SIZE, false), 8.0);
        // Upper mono bound (13px) scales linearly: 16*13/12, 8*13/12.
        let wide13 = gutter_ruler_width_for_font(13.0, true);
        let narrow13 = gutter_ruler_width_for_font(13.0, false);
        assert!((wide13 - 17.333_333).abs() < 0.01);
        assert!((narrow13 - 8.666_667).abs() < 0.01);
        // Bad sizes fall back instead of poisoning layout.
        assert_eq!(gutter_ruler_width_for_font(0.0, true), 16.0);
        assert_eq!(gutter_ruler_width_for_font(-4.0, false), 8.0);
        assert_eq!(gutter_ruler_width_for_font(f32::NAN, true), 16.0);
        assert_eq!(gutter_ruler_width_for_font(f32::INFINITY, false), 8.0);
    }

    #[test]
    fn min_segment_height_keeps_tall_file_hunks_visible() {
        // 1000 rows over 400px: one row owns 0.4px, far below visible.
        let mut rows = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            rows.push(make_test_row(TextRowKind::Equal));
            i = i.saturating_add(1);
        }
        if let Some(row) = rows.get_mut(500) {
            row.kind = TextRowKind::Deleted;
        }
        let geom = gutter_geometry(1000, 400.0, &rows);
        assert_eq!(geom.segments.len(), 1);
        let Some(seg) = geom.segments.first() else {
            return;
        };
        assert_eq!(seg.kind, TextRowKind::Deleted);
        assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
    }

    #[test]
    fn adjacent_diffs_fuse_into_one_clickable_band() {
        // Same tall file, two neighbouring diffs: min-height growth
        // overlaps them so they fuse into a single band.
        let mut rows = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            rows.push(make_test_row(TextRowKind::Equal));
            i = i.saturating_add(1);
        }
        if let Some(row) = rows.get_mut(500) {
            row.kind = TextRowKind::Deleted;
        }
        if let Some(row) = rows.get_mut(501) {
            row.kind = TextRowKind::Deleted;
        }
        let geom = gutter_geometry(1000, 400.0, &rows);
        assert_eq!(geom.segments.len(), 1);
        let Some(seg) = geom.segments.first() else {
            return;
        };
        assert!((seg.y_end - seg.y_start) >= GUTTER_SEGMENT_MIN_H);
    }

    #[test]
    fn slop_hit_test_expands_bands_without_moving_paint() {
        // Tall file: one diff row owns a 4px band at y 200..204.
        let mut rows = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            rows.push(make_test_row(TextRowKind::Equal));
            i = i.saturating_add(1);
        }
        if let Some(row) = rows.get_mut(500) {
            row.kind = TextRowKind::Deleted;
        }
        let geom = gutter_geometry(1000, 400.0, &rows);
        assert_eq!(geom.segments.len(), 1);
        // Neighbour pixels just outside the band: exact misses...
        assert_eq!(gutter_segment_at_y(&geom.segments, 199.0), None);
        assert_eq!(gutter_segment_at_y(&geom.segments, 205.0), None);
        // ...slop reaches them, zero/bad slop matches the exact test.
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, 199.0, GUTTER_HIT_SLOP),
            Some(0)
        );
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, 205.0, GUTTER_HIT_SLOP),
            Some(0)
        );
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, 150.0, GUTTER_HIT_SLOP),
            None
        );
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, 202.0, 0.0),
            gutter_segment_at_y(&geom.segments, 202.0)
        );
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, 202.0, -3.0),
            gutter_segment_at_y(&geom.segments, 202.0)
        );
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, 202.0, f32::NAN),
            gutter_segment_at_y(&geom.segments, 202.0)
        );
        assert_eq!(
            gutter_segment_at_y_with_slop(&geom.segments, f32::NAN, 4.0),
            None
        );
        assert_eq!(gutter_segment_at_y_with_slop(&[], 202.0, 4.0), None);
    }

    #[test]
    fn row_center_y_round_trips_both_directions() {
        // 4 rows over 400px: band centers at 50/150/250/350.
        assert_eq!(gutter_row_center_y(0, 4, 400.0), Some(50.0));
        assert_eq!(gutter_row_center_y(1, 4, 400.0), Some(150.0));
        assert_eq!(gutter_row_center_y(3, 4, 400.0), Some(350.0));
        // Bad input maps nothing, same contract as y_for_row.
        assert_eq!(gutter_row_center_y(4, 4, 400.0), None);
        assert_eq!(gutter_row_center_y(0, 0, 400.0), None);
        assert_eq!(gutter_row_center_y(1, 4, 0.0), None);
        assert_eq!(gutter_row_center_y(1, 4, f32::NAN), None);
        // Round-trip both ways: center -> row -> top.
        let mut row = 0usize;
        while row < 4 {
            if let Some(center) = gutter_row_center_y(row, 4, 400.0) {
                assert_eq!(gutter_row_at_y(center, 4, 400.0), Some(row));
                assert_eq!(gutter_y_for_row(row, 4, 400.0), Some(row as f32 * 100.0));
            }
            row = row.saturating_add(1);
        }
    }

    #[test]
    fn selection_marker_band_covers_one_min_band() {
        // 4 rows over 400px: row 1 center 150, band 148..152.
        assert_eq!(
            gutter_selection_marker_band(1, 4, 400.0),
            Some((148.0, 152.0))
        );
        // Band height is exactly one min segment, never two rows.
        if let Some((y0, y1)) = gutter_selection_marker_band(2, 4, 400.0) {
            assert!((y1 - y0 - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
            assert_eq!(gutter_row_center_y(2, 4, 400.0), Some((y0 + y1) / 2.0));
        } else {
            panic!("band must exist");
        }
        // Edge rows clamp inside the track instead of spilling out.
        assert_eq!(
            gutter_selection_marker_band(0, 1000, 400.0),
            Some((0.0, 4.0))
        );
        assert_eq!(
            gutter_selection_marker_band(999, 1000, 400.0),
            Some((396.0, 400.0))
        );
        // Bad input maps nothing, same contract as center-y.
        assert_eq!(gutter_selection_marker_band(4, 4, 400.0), None);
        assert_eq!(gutter_selection_marker_band(0, 0, 400.0), None);
        assert_eq!(gutter_selection_marker_band(1, 4, 0.0), None);
        assert_eq!(gutter_selection_marker_band(1, 4, f32::NAN), None);
    }

    #[test]
    fn merge_gutter_geometry_detects_conflict_hunks() {
        let lines = vec![
            "h1".to_string(),
            "<<<<<<<".to_string(),
            "L".to_string(),
            "=======".to_string(),
            "R".to_string(),
            ">>>>>>>".to_string(),
            "clean".to_string(),
        ];
        let geom = merge_gutter_geometry(&lines, 140.0);
        assert_eq!(geom.segments.len(), 1);
        let Some(seg) = geom.segments.first() else {
            return;
        };
        assert_eq!(seg.kind, TextRowKind::Changed);
        assert!(seg.y_start >= 0.0);
        assert!(seg.y_end <= 140.0);
    }

    #[test]
    fn merge_conflict_line_ranges_extracts_hunks() {
        let lines = vec![
            "h1".to_string(),
            "<<<<<<<".to_string(),
            "L".to_string(),
            ">>>>>>>".to_string(),
            "h2".to_string(),
        ];
        let ranges = merge_conflict_line_ranges(&lines);
        assert_eq!(ranges, vec![(1, 4)]);
    }

    #[test]
    fn gutter_ruler_hit_test_clamps_and_maps() {
        assert_eq!(gutter_ruler_hit_test(0.0, 100.0, 10), Some(0));
        assert_eq!(gutter_ruler_hit_test(50.0, 100.0, 10), Some(5));
        assert_eq!(gutter_ruler_hit_test(99.9, 100.0, 10), Some(9));
        assert_eq!(gutter_ruler_hit_test(150.0, 100.0, 10), Some(9));
        assert_eq!(gutter_ruler_hit_test(-5.0, 100.0, 10), None);
        assert_eq!(gutter_ruler_hit_test(10.0, 0.0, 10), None);
    }

    #[test]
    fn viewport_rect_maps_visible_window_to_ruler_band() {
        // 4 rows over 400px: rows 1..3 own y 100..300.
        assert_eq!(gutter_viewport_rect(4, 400.0, 1, 2), Some((100.0, 300.0)));
        // Full file maps the whole ruler.
        assert_eq!(gutter_viewport_rect(4, 400.0, 0, 4), Some((0.0, 400.0)));
        // Over-long window clamps to the file end.
        assert_eq!(gutter_viewport_rect(4, 400.0, 2, 100), Some((200.0, 400.0)));
    }

    #[test]
    fn viewport_rect_floors_tiny_windows_and_rejects_bad_input() {
        // 1000 rows over 400px: one row owns 0.4px, floor lifts to 4px.
        assert_eq!(
            gutter_viewport_rect(1000, 400.0, 500, 1),
            Some((200.0, 204.0))
        );
        assert_eq!(gutter_viewport_rect(0, 400.0, 0, 10), None);
        assert_eq!(gutter_viewport_rect(4, 400.0, 0, 0), None);
        assert_eq!(gutter_viewport_rect(4, 0.0, 0, 2), None);
        assert_eq!(gutter_viewport_rect(4, f32::NAN, 0, 2), None);
        assert_eq!(gutter_viewport_rect(4, 400.0, 99, 2), None);
        assert_eq!(GUTTER_VIEWPORT_MIN_H, 4.0);
        assert_eq!(GUTTER_VIEWPORT_MIN_H, crate::theme::SPACE_XS);
    }

    #[test]
    fn ranges_match_row_wise_geometry_and_virtualize_tall_files() {
        // Same 4-row scene as `difference_rows_map_to_coordinates_and_merge`.
        let rows = vec![
            make_test_row(TextRowKind::Equal),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Deleted),
            make_test_row(TextRowKind::Inserted),
        ];
        let row_wise = gutter_geometry(4, 400.0, &rows);
        let ranged = gutter_geometry_from_ranges(
            4,
            400.0,
            &[(1, 3, TextRowKind::Deleted), (3, 4, TextRowKind::Inserted)],
        );
        assert_eq!(ranged, row_wise);
        // Tall file virtualizes: one range, no 1M-row vec, same 4px floor.
        let tall = gutter_geometry_from_ranges(
            1_000_000,
            400.0,
            &[(500_000, 500_001, TextRowKind::Deleted)],
        );
        assert_eq!(tall.segments.len(), 1);
        let Some(seg) = tall.segments.first() else {
            return;
        };
        assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        // Empty/out-of-range spans contribute nothing; bad input is empty.
        assert!(
            gutter_geometry_from_ranges(4, 400.0, &[(2, 2, TextRowKind::Deleted)])
                .segments
                .is_empty()
        );
        assert!(
            gutter_geometry_from_ranges(4, 400.0, &[(9, 12, TextRowKind::Deleted)])
                .segments
                .is_empty()
        );
        assert!(
            gutter_geometry_from_ranges(0, 400.0, &[(0, 1, TextRowKind::Deleted)])
                .segments
                .is_empty()
        );
        assert!(
            gutter_geometry_from_ranges(4, 0.0, &[(0, 1, TextRowKind::Deleted)])
                .segments
                .is_empty()
        );
    }
}

#[cfg(test)]
mod exq_tree2_tests {
    use super::*;

    #[test]
    fn tree2_center_arrows_per_hunk() {
        // Mirrors checks/gui-exq-tree2/round-1/comparison/edge-deep-nest.json.
        assert_eq!(gutter_center_width(), 24.0);
        assert_eq!(gutter_center_width(), crate::theme::SPACE_XL);
        assert_eq!(gutter_arrow_box(), 16.0);
        assert_eq!(gutter_arrow_box(), crate::theme::SPACE_LG);
        let seg = GutterSegment {
            y_start: 100.0,
            y_end: 300.0,
            kind: TextRowKind::Deleted,
        };
        assert_eq!(gutter_arrow_center_y(&seg), Some(200.0));
        assert_eq!(
            gutter_arrow_rect(12.0, 200.0),
            Some((4.0, 192.0, 20.0, 208.0))
        );
        assert_eq!(gutter_arrow_rect(f32::NAN, 200.0), None);
        assert_eq!(gutter_arrow_rect(12.0, f32::INFINITY), None);
        let bad = GutterSegment {
            y_start: f32::NAN,
            y_end: 300.0,
            kind: TextRowKind::Deleted,
        };
        assert_eq!(gutter_arrow_center_y(&bad), None);
        // Har segment (= ek hunk) ka ek center, order me.
        let segs = vec![
            GutterSegment {
                y_start: 100.0,
                y_end: 300.0,
                kind: TextRowKind::Deleted,
            },
            GutterSegment {
                y_start: 300.0,
                y_end: 400.0,
                kind: TextRowKind::Inserted,
            },
        ];
        assert_eq!(gutter_arrow_centers(&segs), vec![200.0, 350.0]);
        assert!(gutter_arrow_centers(&[]).is_empty());
    }
}

// ----------------------------------------------------------------------------
// GUTTERCOPY8: arrow glyph breathing + padded minimap track (headless).
// BC center arrows ka chevron box se chhota hota hai (glyph saans leta
// hai, 20px hit untouched), aur minimap bands ruler edges se chipakte
// nahi — top/bottom track pad me baithte hain taaki pehli/aakhri band
// kabhi clip na ho. Geometry first: sab SPACE scale reuse, koi naya
// literal nahi. Typography second: kuch nahi (koi text yahan nahi).
// Colors frozen: sirf shape.
// WIRING (paint owner ke paas): chevron `gutter_arrow_glyph_rect(cx, cy)`
// me, overview `gutter_overview_geometry_padded(..)` se, arrow x
// `gutter_arrow_center_x(col_x)` se.
// ----------------------------------------------------------------------------

/// Arrow glyph inset per side: merge-gap reuse (2px).
///
/// 16px box me 12px chevron; 20px slop hit-target untouched rehta hai.
pub const GUTTER_ARROW_GLYPH_INSET: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Minimap track pad top/bottom: one XS step (ruler-edge rhythm).
///
/// Pehli/aakhri overview band ruler kinare se 4px andar shuru/khatm hoti
/// hai; bahut chhoti height par inner track ke bajay khali geometry.
pub const GUTTER_MINIMAP_TRACK_PAD: f32 = crate::theme::SPACE_XS;

/// Center arrow ka glyph rect `(x0, y0, x1, y1)` px: [`gutter_arrow_rect`]
/// box har taraf [`GUTTER_ARROW_GLYPH_INSET`] sikuda hua (16px box ->
/// 12px chevron). Non-finite ya ulta rect par `None`. Pure; unit-tested.
pub fn gutter_arrow_glyph_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let (x0, y0, x1, y1) = gutter_arrow_rect(center_x, center_y)?;
    let s = GUTTER_ARROW_GLYPH_INSET;
    let (lx0, ly0, lx1, ly1) = (x0 + s, y0 + s, x1 - s, y1 - s);
    if lx0.is_finite()
        && ly0.is_finite()
        && lx1.is_finite()
        && ly1.is_finite()
        && lx0 < lx1
        && ly0 < ly1
    {
        Some((lx0, ly0, lx1, ly1))
    } else {
        None
    }
}

/// 24px center column me arrow ka center x px: `col_x + CENTER_W / 2`.
/// Non-finite par `None` taaki paint NaN kabhi layout me na daale.
/// Pure; unit-tested.
pub fn gutter_arrow_center_x(col_x: f32) -> Option<f32> {
    if !col_x.is_finite() {
        return None;
    }
    let cx = col_x + GUTTER_CENTER_W / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// Center-column marker leading inset px: leftover `(column - marker)`
/// equally split per side so a marker strip sits centered (BC center-bar
/// rhythm). Non-finite/non-positive ya marker >= column par 0 (paint kabhi
/// NaN/negative inset na khaye). Geometry only; colors untouched.
/// TEXT3 center-gutter (`views.rs`) isi se apne do-arrow row ko 48px me
/// center karta hai. Pure; unit-tested.
pub fn center_row_marker_inset(column_w: f32, marker_w: f32) -> f32 {
    if !column_w.is_finite() || !marker_w.is_finite() || column_w <= 0.0 || marker_w <= 0.0 {
        return 0.0;
    }
    ((column_w - marker_w) / 2.0).max(0.0)
}

/// Minimap inner track height px: `height - 2 * pad`. `None`:
/// non-finite/non-positive height, ya track pad se dhaka (inner <= 0).
/// Pure; unit-tested.
pub fn gutter_minimap_inner_height(height: f32) -> Option<f32> {
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    let inner = height - 2.0 * GUTTER_MINIMAP_TRACK_PAD;
    if inner.is_finite() && inner > 0.0 {
        Some(inner)
    } else {
        None
    }
}

/// Padded thumbnail overview: [`gutter_overview_geometry`] wahi band-math
/// (fraction mapping, [`GUTTER_SEGMENT_MIN_H`] floor, merge-gap fusion,
/// same+diff bands) par `[pad, height - pad]` inner track me. Pehli band
/// pad par shuru, aakhri `height - pad` par khatm — ruler edges kabhi
/// clip nahi hote. Na-mumkin track par khali geometry. Pure; unit-tested.
pub fn gutter_overview_geometry_padded(
    total_rows: usize,
    height: f32,
    rows: &[TextRow],
) -> GutterGeometry {
    if total_rows == 0 {
        return GutterGeometry {
            total_height: if height.is_finite() {
                height.max(0.0)
            } else {
                0.0
            },
            segments: Vec::new(),
        };
    }
    let Some(inner) = gutter_minimap_inner_height(height) else {
        return GutterGeometry {
            total_height: if height.is_finite() {
                height.max(0.0)
            } else {
                0.0
            },
            segments: Vec::new(),
        };
    };
    let pad = GUTTER_MINIMAP_TRACK_PAD;
    let mut segments: Vec<GutterSegment> = Vec::new();
    let total_f = total_rows as f32;
    let mut idx = 0usize;
    while idx < rows.len() && idx < total_rows {
        let kind = match rows.get(idx) {
            Some(row) => row.kind,
            None => break,
        };
        let mut end = idx.saturating_add(1);
        while end < rows.len() && end < total_rows {
            match rows.get(end) {
                Some(next) if next.kind == kind => {
                    end = end.saturating_add(1);
                }
                _ => break,
            }
        }
        let y_start = pad + (idx as f32 / total_f) * inner;
        let mut y_end = pad + (end as f32 / total_f) * inner;
        if y_end < y_start + GUTTER_SEGMENT_MIN_H {
            y_end = (y_start + GUTTER_SEGMENT_MIN_H).min(pad + inner);
        }
        if let Some(last) = segments.last_mut() {
            let last_seg: &mut GutterSegment = last;
            if last_seg.kind == kind && y_start <= last_seg.y_end + crate::theme::GUTTER_MERGE_GAP {
                last_seg.y_end = last_seg.y_end.max(y_end);
                idx = end;
                continue;
            }
        }
        segments.push(GutterSegment {
            y_start,
            y_end,
            kind,
        });
        idx = end;
    }
    GutterGeometry {
        total_height: height,
        segments,
    }
}

#[cfg(test)]
mod exq_guttercopy8_tests {
    use super::*;

    fn prow(kind: TextRowKind) -> TextRow {
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
    fn guttercopy8_glyph_box_breathes_inside_hit_box() {
        // Mirrors checks/gui-exq-guttercopy8/round-1/comparison/edge-padded-minimap-tall.json.
        assert_eq!(GUTTER_ARROW_GLYPH_INSET, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_ARROW_GLYPH_INSET, 2.0);
        // 16px box (4..20, 192..208) me 12px chevron (6..18, 194..206).
        assert_eq!(
            gutter_arrow_glyph_rect(12.0, 200.0),
            Some((6.0, 194.0, 18.0, 206.0))
        );
        // Glyph chevron hit box ke andar: hit lage jahan glyph na ho.
        assert!(gutter_arrow_hit_contains(12.0, 200.0, 3.0, 200.0));
        assert_eq!(gutter_arrow_glyph_rect(f32::NAN, 200.0), None);
        assert_eq!(gutter_arrow_glyph_rect(12.0, f32::INFINITY), None);
    }

    #[test]
    fn guttercopy8_arrow_centers_in_column() {
        assert_eq!(gutter_arrow_center_x(0.0), Some(12.0));
        assert_eq!(gutter_arrow_center_x(100.0), Some(112.0));
        assert_eq!(gutter_arrow_center_x(f32::NAN), None);
        assert_eq!(gutter_arrow_center_x(f32::INFINITY), None);
    }

    #[test]
    fn text3_center_row_marker_inset_splits_leftover() {
        // TEXT3 center-gutter (views.rs 48px column, 36px two-arrow row)
        // ka headless contract: leftover barabar banta hai.
        assert_eq!(center_row_marker_inset(48.0, 36.0), 6.0);
        assert_eq!(center_row_marker_inset(24.0, 16.0), 4.0);
        assert_eq!(center_row_marker_inset(48.0, 60.0), 0.0);
        assert_eq!(center_row_marker_inset(f32::NAN, 36.0), 0.0);
        assert_eq!(center_row_marker_inset(48.0, f32::NAN), 0.0);
        assert_eq!(center_row_marker_inset(0.0, 36.0), 0.0);
        assert_eq!(center_row_marker_inset(-8.0, 36.0), 0.0);
    }

    #[test]
    fn guttercopy8_minimap_track_pads_tiny_heights_out() {
        assert_eq!(GUTTER_MINIMAP_TRACK_PAD, crate::theme::SPACE_XS);
        assert_eq!(GUTTER_MINIMAP_TRACK_PAD, 4.0);
        assert_eq!(gutter_minimap_inner_height(400.0), Some(392.0));
        // 8px height pad se dhaka: inner 0, None.
        assert_eq!(gutter_minimap_inner_height(8.0), None);
        assert_eq!(gutter_minimap_inner_height(0.0), None);
        assert_eq!(gutter_minimap_inner_height(-50.0), None);
        assert_eq!(gutter_minimap_inner_height(f32::NAN), None);
    }

    #[test]
    fn guttercopy8_padded_overview_sits_inside_track() {
        // 6 rows E D D E E I @600px, pad 4: bands [4, 596] me, 4 segments.
        let rows = vec![
            prow(TextRowKind::Equal),
            prow(TextRowKind::Deleted),
            prow(TextRowKind::Deleted),
            prow(TextRowKind::Equal),
            prow(TextRowKind::Equal),
            prow(TextRowKind::Inserted),
        ];
        let over = gutter_overview_geometry_padded(6, 600.0, &rows);
        assert_eq!(over.segments.len(), 4);
        assert_eq!(over.total_height, 600.0);
        assert!(
            over.segments
                .first()
                .is_some_and(|s| s.y_start == GUTTER_MINIMAP_TRACK_PAD)
        );
        assert!(
            over.segments
                .get(3)
                .is_some_and(|s| s.y_end == 600.0 - GUTTER_MINIMAP_TRACK_PAD)
        );
        // Bands contiguous: har agli band pichhli ke end par shuru.
        let mut i = 1usize;
        while i < over.segments.len() {
            let (prev_end, next_start) = match (over.segments.get(i - 1), over.segments.get(i)) {
                (Some(p), Some(n)) => (p.y_end, n.y_start),
                _ => break,
            };
            assert!((next_start - prev_end).abs() < 0.01);
            i = i.saturating_add(1);
        }
        // Diff bands wahi (Equal runs background, Deleted/Inserted same math).
        let kinds: Vec<TextRowKind> = over.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TextRowKind::Equal,
                TextRowKind::Deleted,
                TextRowKind::Equal,
                TextRowKind::Inserted
            ]
        );
        // Na-mumkin track / khali input par khali.
        assert!(
            gutter_overview_geometry_padded(6, 8.0, &rows)
                .segments
                .is_empty()
        );
        assert!(
            gutter_overview_geometry_padded(0, 600.0, &rows)
                .segments
                .is_empty()
        );
        assert!(
            gutter_overview_geometry_padded(6, f32::NAN, &rows)
                .segments
                .is_empty()
        );
    }

    #[test]
    fn guttercopy8_tall_file_bands_stay_inside_track() {
        // 1000 rows @400px, ek diff row 500 par: band [4,396] ke andar, 4px floor.
        let mut rows = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            rows.push(prow(TextRowKind::Equal));
            i = i.saturating_add(1);
        }
        if let Some(row) = rows.get_mut(500) {
            row.kind = TextRowKind::Deleted;
        }
        let over = gutter_overview_geometry_padded(1000, 400.0, &rows);
        let mut found = false;
        for seg in &over.segments {
            assert!(seg.y_start >= GUTTER_MINIMAP_TRACK_PAD);
            assert!(seg.y_end <= 400.0 - GUTTER_MINIMAP_TRACK_PAD);
            if seg.kind == TextRowKind::Deleted {
                assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.01);
                found = true;
            }
        }
        assert!(found);
    }
}

#[cfg(test)]
mod exq_gutter6_tests {
    use super::*;

    fn diff_row(kind: TextRowKind) -> TextRow {
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
    fn gutter6_numbers_column_scales_with_digits_on_scale() {
        // Mirrors checks/gui-exq-gutter6/round-1/comparison/edge-tall-same-diff.json.
        assert_eq!(GUTTER_LINENUM_PAD, crate::theme::SPACE_XS);
        assert_eq!(GUTTER_LINENUM_PAD, 4.0);
        assert_eq!(gutter_linenum_digits(0), 1);
        assert_eq!(gutter_linenum_digits(9), 1);
        assert_eq!(gutter_linenum_digits(99), 2);
        assert_eq!(gutter_linenum_digits(1000), 4);
        assert_eq!(gutter_linenum_column_width(9, false), 8.0);
        assert_eq!(gutter_linenum_column_width(9, true), 16.0);
        assert_eq!(gutter_linenum_column_width(99, true), 20.0);
        assert_eq!(gutter_linenum_column_width(999, true), 24.0);
        assert_eq!(gutter_linenum_column_width(9999, true), 28.0);
        assert_eq!(gutter_linenum_column_width(99_999, true), 32.0);
        // Cap: 7 digits bhi 36 par atke.
        assert_eq!(gutter_linenum_column_width(9_999_999, true), 36.0);
        assert!(gutter_linenum_column_width(999, true) > gutter_linenum_column_width(99, true));
    }

    #[test]
    fn gutter6_linenum_typo_reuses_grid_mono_tabular() {
        use crate::theme::{TYPO_MONO_SIZE, TypoRole};
        assert_eq!(gutter_linenum_role(), TypoRole::content_grid());
        assert_eq!(gutter_linenum_role(), TypoRole::Mono);
        assert_eq!(gutter_linenum_role().size(), TYPO_MONO_SIZE);
        assert_eq!(gutter_linenum_role().size(), 12.0);
        assert!(gutter_linenum_is_tabular());
        assert_eq!(
            gutter_linenum_is_tabular(),
            TypoRole::content_grid().is_tabular()
        );
    }

    #[test]
    fn gutter10_number_trail_pad_matches_lead_beat() {
        // Mirrors checks/gui-exq-text10/comparison/tail-insertion.json:
        // number column aur code ke beech do spaces (two-space lead
        // beat parity) taaki numbers code se chipke na lagen; blank
        // sides utna hi blank rakhen.
        assert_eq!(GUTTER_NUMBER_TRAIL_PAD, 2);
    }

    #[test]
    fn gutter_number_cell_right_aligns_tabular() {
        // Mirrors checks/gui-exq-text4/comparison/text-narrow-720b.json:
        // right-aligned number + 2-space trail, blanks same width, tabular.
        assert_eq!(gutter_number_cell(Some(1), 4), "   1  ".to_string());
        assert_eq!(gutter_number_cell(Some(12), 4), "  12  ".to_string());
        assert_eq!(gutter_number_cell(None, 4), "      ".to_string());
        assert_eq!(
            gutter_number_cell(None, 4).chars().count(),
            gutter_number_cell(Some(1), 4).chars().count()
        );
        assert!(gutter_number_cell_is_right_aligned_tabular());
        assert_eq!(
            gutter_number_cell_is_right_aligned_tabular(),
            gutter_linenum_is_tabular()
        );
    }

    #[test]
    fn gutter6_text_row_height_pads_line_height_symmetrically() {
        assert_eq!(GUTTER_TEXT_ROW_PAD, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_TEXT_ROW_PAD, 2.0);
        assert_eq!(gutter_text_row_pad(), 2.0);
        // Mono 12 @1.30 = 15.6 line -> 19.6 row.
        let h = gutter_text_row_height(15.6);
        assert!(h.is_some_and(|v| (v - 19.6).abs() < 0.01));
        assert_eq!(gutter_text_row_height(0.0), None);
        assert_eq!(gutter_text_row_height(-4.0), None);
        assert_eq!(gutter_text_row_height(f32::NAN), None);
        assert_eq!(gutter_text_row_height(f32::INFINITY), None);
    }

    #[test]
    fn gutter6_overview_covers_same_and_diff_with_shared_bands() {
        // 6 rows over 600px: E D D E E I -> runs E(0..100) D(100..300)
        // E(300..500) I(500..600); diff-only geometry se 2 segments,
        // overview se 4 (same background samet), band-math identical.
        let rows = vec![
            diff_row(TextRowKind::Equal),
            diff_row(TextRowKind::Deleted),
            diff_row(TextRowKind::Deleted),
            diff_row(TextRowKind::Equal),
            diff_row(TextRowKind::Equal),
            diff_row(TextRowKind::Inserted),
        ];
        let diff_only = gutter_geometry(6, 600.0, &rows);
        assert_eq!(diff_only.segments.len(), 2);
        let over = gutter_overview_geometry(6, 600.0, &rows);
        assert_eq!(over.segments.len(), 4);
        assert_eq!(over.total_height, 600.0);
        let kinds: Vec<TextRowKind> = over.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TextRowKind::Equal,
                TextRowKind::Deleted,
                TextRowKind::Equal,
                TextRowKind::Inserted
            ]
        );
        // Diff bands dono paths par identical (same/diff heights match).
        assert_eq!(over.segments.get(1), diff_only.segments.first());
        assert_eq!(over.segments.get(3), diff_only.segments.get(1));
        // Bands contiguous: koi gap nahi (ruler poora dhaka).
        assert!(over.segments.first().is_some_and(|s| s.y_start == 0.0));
        assert!(over.segments.get(3).is_some_and(|s| s.y_end == 600.0));
        // Khali/kharab input par khali.
        assert!(
            gutter_overview_geometry(0, 600.0, &rows)
                .segments
                .is_empty()
        );
        assert!(gutter_overview_geometry(6, 0.0, &rows).segments.is_empty());
        assert!(
            gutter_overview_geometry(6, f32::NAN, &rows)
                .segments
                .is_empty()
        );
        let equals = vec![diff_row(TextRowKind::Equal), diff_row(TextRowKind::Equal)];
        assert_eq!(
            gutter_overview_geometry(2, 200.0, &equals).segments.len(),
            1
        );
    }

    #[test]
    fn gutter6_arrow_hit_rect_expands_box_without_moving_paint() {
        assert_eq!(GUTTER_ARROW_HIT_SLOP, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_ARROW_HIT_SLOP, 2.0);
        // 16px box (4..20, 192..208) -> 20px hit (2..22, 190..210).
        assert_eq!(
            gutter_arrow_hit_rect(12.0, 200.0),
            Some((2.0, 190.0, 22.0, 210.0))
        );
        // Box edge ke bahar 1px: box miss, hit lage.
        assert!(
            !gutter_arrow_rect(12.0, 200.0).is_some_and(|(x0, y0, x1, y1)| {
                3.0 >= x0 && 3.0 <= x1 && 200.0 >= y0 && 200.0 <= y1
            })
        );
        assert!(gutter_arrow_hit_contains(12.0, 200.0, 3.0, 200.0));
        assert!(gutter_arrow_hit_contains(12.0, 200.0, 12.0, 200.0));
        assert!(!gutter_arrow_hit_contains(12.0, 200.0, 1.0, 200.0));
        assert!(!gutter_arrow_hit_contains(12.0, 200.0, 12.0, 211.0));
        assert!(!gutter_arrow_hit_contains(12.0, 200.0, f32::NAN, 200.0));
        assert_eq!(gutter_arrow_hit_rect(f32::NAN, 200.0), None);
        assert!(!gutter_arrow_hit_contains(f32::NAN, 200.0, 12.0, 200.0));
    }
}

// ----------------------------------------------------------------------------
// GUTTER10: line-number text origin + padded overview click mapping (headless).
// BC number gutter me digits right kinare se chipakte nahi — ek fixed
// inset par right-aligned hote hain; aur padded minimap (track pad ke
// andar) clickable hai: pad kinaaron par click sabse najdeeki row par
// girta hai, viewport band bhi inner track me rehta hai. Geometry first:
// sab SPACE/GUTTER scale reuse (inset = merge-gap, pad = minimap track
// pad), koi naya literal nahi. Typography second: numbers wahi
// content_grid (Mono 12 tabular) role me, koi naya size nahi. Colors
// frozen: sirf shape.
// WIRING (paint owner ke paas): number text `gutter_linenum_text_x(..)`
// par right-aligned; padded clicks `gutter_padded_row_at_y(..)` se,
// markers `gutter_padded_y_for_row(..)` se, viewport band
// `gutter_viewport_rect_padded(..)` se.
// ----------------------------------------------------------------------------

/// Line-number right inset px: merge-gap reuse (2px).
///
/// Digits column ke right kinare se 2px andar khatm hote hain taaki
/// ruler edge se na chipken; column width se independent fixed beat.
pub const GUTTER_LINENUM_INSET: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Line-number text ka right-aligned x px: `col_x + col_w - inset`.
/// `None`: non-finite input, ya column inset se chaudi nahi (origin
/// column ke bahar girta). Pure; unit-tested.
pub fn gutter_linenum_text_x(col_x: f32, col_w: f32) -> Option<f32> {
    if !col_x.is_finite() || !col_w.is_finite() {
        return None;
    }
    if col_w <= GUTTER_LINENUM_INSET {
        return None;
    }
    let x = col_x + col_w - GUTTER_LINENUM_INSET;
    if x.is_finite() { Some(x) } else { None }
}

/// Padded minimap me gutter-local y (px) se model row index.
///
/// [`gutter_row_at_y`] ka padded twin: bands `[pad, height - pad]` inner
/// track me hain, isliye fraction wahi se napta hai. Pad kinaaron par
/// click (`[0, pad)` / `(height - pad, height)`) sabse najdeeki row
/// (pehli/aakhri) par girta hai taaki poora ruler clickable rahe;
/// ruler ke bahar (`y < 0` / `y >= height`) `None`. Pure; unit-tested.
pub fn gutter_padded_row_at_y(y: f32, total_rows: usize, height: f32) -> Option<usize> {
    if total_rows == 0 {
        return None;
    }
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    if !y.is_finite() || y < 0.0 || y >= height {
        return None;
    }
    let inner = gutter_minimap_inner_height(height)?;
    let pad = GUTTER_MINIMAP_TRACK_PAD;
    let clamped = y.clamp(pad, pad + inner);
    let frac = (clamped - pad) / inner;
    if !frac.is_finite() {
        return None;
    }
    gutter_row_from_frac(frac.clamp(0.0, 1.0), total_rows)
}

/// Padded minimap me model row ka band-top y (px).
///
/// [`gutter_y_for_row`] ka padded twin: `pad + (row / total) * inner`.
/// `None` contract wahi (khali total, out-of-range row, kharab height
/// ya na-mumkin inner track). Paint isi se selection marker lagata hai.
/// Pure; unit-tested.
pub fn gutter_padded_y_for_row(row: usize, total_rows: usize, height: f32) -> Option<f32> {
    if total_rows == 0 || row >= total_rows {
        return None;
    }
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    let inner = gutter_minimap_inner_height(height)?;
    let y = GUTTER_MINIMAP_TRACK_PAD + (row as f32 / total_rows as f32) * inner;
    if y.is_finite() && y >= GUTTER_MINIMAP_TRACK_PAD && y < height {
        Some(y)
    } else {
        None
    }
}

/// Padded inner track me visible scroll window ka band `(y0, y1)`.
///
/// [`gutter_viewport_rect`] ka padded twin: wahi row-window mapping par
/// `[pad, pad + inner]` me, [`GUTTER_VIEWPORT_MIN_H`] floor track ke
/// andar (neeche se katkar). `None` contract wahi. Pure; unit-tested.
pub fn gutter_viewport_rect_padded(
    total_rows: usize,
    height: f32,
    top_row: usize,
    visible_rows: usize,
) -> Option<(f32, f32)> {
    if total_rows == 0 || visible_rows == 0 {
        return None;
    }
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    let inner = gutter_minimap_inner_height(height)?;
    let pad = GUTTER_MINIMAP_TRACK_PAD;
    let total_f = total_rows as f32;
    let top = top_row.min(total_rows) as f32;
    let bottom = top_row.saturating_add(visible_rows).min(total_rows) as f32;
    if bottom <= top {
        return None;
    }
    let mut y0 = pad + (top / total_f) * inner;
    let mut y1 = pad + (bottom / total_f) * inner;
    if y1 < y0 + GUTTER_VIEWPORT_MIN_H {
        y1 = (y0 + GUTTER_VIEWPORT_MIN_H).min(pad + inner);
    }
    if y1 - y0 < GUTTER_VIEWPORT_MIN_H {
        y0 = (y1 - GUTTER_VIEWPORT_MIN_H).max(pad);
    }
    if !y0.is_finite() || !y1.is_finite() || y0 < pad || y1 > pad + inner || y0 >= y1 {
        return None;
    }
    Some((y0, y1))
}

/// 720px narrow canvas symmetric side layout `(margin, left, center, right, margin)`
/// px: 12 + 338 + 20 + 338 + 12 = 720. Panes barabar, 20px side column beech
/// me symmetric insets samet. Single source over [`gutter_text_layout`].
/// Pure; unit-tested.
pub fn gutter_side_layout_720() -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        GUTTER_SIDE_MARKER_COL_W,
        crate::theme::VIEWPORT_MIN_W as f32,
    )
}

/// 720px narrow canvas side-column x px: `margin + pane` from the symmetric
/// side layout (12 + 338 = 350). Column is [`GUTTER_SIDE_MARKER_COL_W`] wide (20).
/// Pure; unit-tested.
pub fn gutter_side_column_x_720() -> f32 {
    let (margin, pane, _, _, _) = gutter_side_layout_720();
    margin + pane
}

/// 720px narrow canvas side-marker cx px: column center (350 + 20/2 = 360 =
/// canvas middle), so the marker sits centered in the gutter. `None` only
/// when the column math goes non-finite. Pure; unit-tested.
pub fn gutter_side_arrow_cx_720() -> Option<f32> {
    let x = gutter_side_column_x_720();
    let w = GUTTER_SIDE_MARKER_COL_W;
    if !x.is_finite() || !w.is_finite() {
        return None;
    }
    let cx = x + w / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// 720px narrow canvas side glyph rect `(x0, y0, x1, y1)` px: 12px chevron
/// ([`gutter_arrow_glyph_rect`]) centered at canvas middle (360) for the
/// given row-center y. `None` for non-finite y. Pure; unit-tested.
pub fn gutter_side_glyph_720(center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_side_arrow_cx_720()?;
    gutter_arrow_glyph_rect(cx, center_y)
}

/// 720px narrow canvas side hit band `(lo, hi)` px: side column + symmetric slop on
/// both sides (350 - 2 .. 370 + 2 = 348..372), canvas center 360 ke around
/// barabar. Pure; unit-tested.
pub fn gutter_side_hit_band_720() -> (f32, f32) {
    let x = gutter_side_column_x_720();
    let w = GUTTER_SIDE_MARKER_COL_W;
    let s = GUTTER_SIDE_HIT_SLOP;
    (x - s, x + w + s)
}

/// 720px narrow canvas center-column x px: `margin + pane` from the symmetric
/// text layout (12 + 336 = 348). Column is [`GUTTER_CENTER_W`] wide (24).
/// Pure; unit-tested.
pub fn gutter_center_column_x_720() -> f32 {
    let (margin, pane, _, _, _) = gutter_text_layout_720();
    margin + pane
}

/// 720px narrow canvas center-arrow cx px: column center (348 + 24/2 = 360 =
/// canvas middle), so the glyph sits centered in the gutter. `None` only
/// when the column math goes non-finite. Pure; unit-tested.
pub fn gutter_center_arrow_cx_720() -> Option<f32> {
    gutter_arrow_center_x(gutter_center_column_x_720())
}

/// 720px narrow canvas center glyph rect `(x0, y0, x1, y1)` px: 12px chevron
/// ([`gutter_arrow_glyph_rect`]) centered at canvas middle (360) for the
/// given row-center y. `None` for non-finite y. Pure; unit-tested.
pub fn gutter_center_glyph_720(center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_center_arrow_cx_720()?;
    gutter_arrow_glyph_rect(cx, center_y)
}

#[cfg(test)]
mod exq_gutter10_tests {
    use super::*;

    #[test]
    fn gutter10_text_x_right_aligns_on_fixed_inset() {
        // Mirrors checks/gui-exq-gutter10/round-1/comparison/edge-padded-click-viewport.json.
        assert_eq!(GUTTER_LINENUM_INSET, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_LINENUM_INSET, 2.0);
        // 16px column (0..16) me text x 14 par; 20px column (100..120) me 118 par.
        assert_eq!(gutter_linenum_text_x(0.0, 16.0), Some(14.0));
        assert_eq!(gutter_linenum_text_x(100.0, 20.0), Some(118.0));
        // Column inset se chaudi nahi: origin bahar girta, None.
        assert_eq!(gutter_linenum_text_x(0.0, 2.0), None);
        assert_eq!(gutter_linenum_text_x(0.0, 0.0), None);
        assert_eq!(gutter_linenum_text_x(f32::NAN, 16.0), None);
        assert_eq!(gutter_linenum_text_x(0.0, f32::INFINITY), None);
    }

    #[test]
    fn gutter10_padded_row_at_y_clamps_pad_edges() {
        // 6 rows @600px: pad 4, inner 592, track [4, 596].
        assert_eq!(gutter_padded_row_at_y(4.0, 6, 600.0), Some(0));
        // Pad kinare ke andar click sabse najdeeki row par.
        assert_eq!(gutter_padded_row_at_y(0.0, 6, 600.0), Some(0));
        assert_eq!(gutter_padded_row_at_y(2.0, 6, 600.0), Some(0));
        assert_eq!(gutter_padded_row_at_y(599.0, 6, 600.0), Some(5));
        assert_eq!(gutter_padded_row_at_y(597.0, 6, 600.0), Some(5));
        // Beech ki band seedh map hoti hai (row 1: [102.67, 201.33)).
        assert_eq!(gutter_padded_row_at_y(150.0, 6, 600.0), Some(1));
        // Ruler ke bahar: None (gutter_row_at_y wahi contract).
        assert_eq!(gutter_padded_row_at_y(-1.0, 6, 600.0), None);
        assert_eq!(gutter_padded_row_at_y(600.0, 6, 600.0), None);
        assert_eq!(gutter_padded_row_at_y(999.0, 6, 600.0), None);
        assert_eq!(gutter_padded_row_at_y(f32::NAN, 6, 600.0), None);
        // Khali/kharab input: None (8px height par inner track na-mumkin).
        assert_eq!(gutter_padded_row_at_y(4.0, 0, 600.0), None);
        assert_eq!(gutter_padded_row_at_y(4.0, 6, 0.0), None);
        assert_eq!(gutter_padded_row_at_y(4.0, 6, 8.0), None);
        assert_eq!(gutter_padded_row_at_y(4.0, 6, f32::NAN), None);
    }

    #[test]
    fn gutter10_padded_markers_round_trip_and_viewport_stays_inside() {
        // Band-top: row 0 -> 4.0, row 5 -> 4 + 5*592/6 = 497.333.
        assert_eq!(gutter_padded_y_for_row(0, 6, 600.0), Some(4.0));
        let y5 = gutter_padded_y_for_row(5, 6, 600.0);
        assert!(y5.is_some_and(|v| (v - 497.333).abs() < 0.01));
        assert_eq!(gutter_padded_y_for_row(6, 6, 600.0), None);
        assert_eq!(gutter_padded_y_for_row(0, 0, 600.0), None);
        assert_eq!(gutter_padded_y_for_row(0, 6, 8.0), None);
        // Round-trip: marker y wapas usi row par gire.
        let mut row = 0usize;
        while row < 6 {
            if let Some(y) = gutter_padded_y_for_row(row, 6, 600.0) {
                assert_eq!(gutter_padded_row_at_y(y + 0.5, 6, 600.0), Some(row));
            }
            row = row.saturating_add(1);
        }
        // Viewport: rows 1..3 -> (102.67, 300.0); poori file -> poora inner track.
        let vp = gutter_viewport_rect_padded(6, 600.0, 1, 2);
        assert!(
            vp.is_some_and(|(a, b)| (a - 102.666_667).abs() < 0.01 && (b - 300.0).abs() < 0.01)
        );
        assert_eq!(
            gutter_viewport_rect_padded(6, 600.0, 0, 6),
            Some((4.0, 596.0))
        );
        // Chhoti window floor samet track ke andar: 1000 rows @400px me ek row 0.392px.
        assert_eq!(
            gutter_viewport_rect_padded(1000, 400.0, 500, 1),
            Some((200.0, 204.0))
        );
        // Band hamesha inner track me, kabhi kinare clip nahi.
        assert!(
            gutter_viewport_rect_padded(6, 600.0, 4, 2).is_some_and(|(a, b)| {
                a >= GUTTER_MINIMAP_TRACK_PAD && b <= 600.0 - GUTTER_MINIMAP_TRACK_PAD
            })
        );
        // Viewport bottom edge floor: row 999 keeps min height without escaping pad.
        assert_eq!(
            gutter_viewport_rect_padded(1000, 400.0, 999, 1),
            Some((392.0, 396.0))
        );
        // Khali/kharab input: None.
        assert_eq!(gutter_viewport_rect_padded(0, 600.0, 0, 2), None);
        assert_eq!(gutter_viewport_rect_padded(6, 600.0, 0, 0), None);
        assert_eq!(gutter_viewport_rect_padded(6, 8.0, 0, 2), None);
        assert_eq!(gutter_viewport_rect_padded(6, 600.0, 99, 2), None);
    }

    #[test]
    fn gutter10_narrow_720_marker_alignment_from_char_metrics() {
        // Mirrors checks/gui-exq-gutter10/round-1/comparison/gutter-narrow-720.json.
        let fixture_raw = include_str!(
            "../../../checks/gui-exq-gutter10/round-1/comparison/gutter-narrow-720.json"
        );
        assert!(fixture_raw.contains("\"arrow_cx_720\": 360.0"));
        assert!(fixture_raw.contains("\"center_column_x_720\": 348.0"));
        assert!(fixture_raw.contains("\"column_x_720\": 350.0"));
        assert!(fixture_raw.contains("\"linenum_col_1digit\": 16.0"));
        assert!(fixture_raw.contains("\"linenum_text_x_1digit\": 26.0"));

        // Narrow layout 720 symmetric: 12 + 338 + 20 + 338 + 12 = 720.
        assert_eq!(gutter_side_layout_720(), (12.0, 338.0, 20.0, 338.0, 12.0));
        let (m0, left, center, right, m1) = gutter_side_layout_720();
        assert_eq!(m0, m1);
        assert_eq!(left, right);
        assert!((m0 + left + center + right + m1 - 720.0).abs() < 0.001);
        assert_eq!(center, GUTTER_SIDE_MARKER_COL_W);

        // Column x 350, cx 360 canvas middle; glyph + hit symmetric about 360.
        assert_eq!(gutter_side_column_x_720(), 350.0);
        assert_eq!(gutter_side_arrow_cx_720(), Some(360.0));
        assert_eq!(
            gutter_side_marker_rect(360.0, 200.0),
            Some((352.0, 192.0, 368.0, 208.0))
        );
        assert_eq!(
            gutter_side_hit_rect(360.0, 200.0),
            Some((350.0, 190.0, 370.0, 210.0))
        );
        assert_eq!(
            gutter_side_glyph_720(200.0),
            Some((354.0, 194.0, 366.0, 206.0))
        );
        assert_eq!(gutter_side_glyph_720(f32::NAN), None);
        assert_eq!(gutter_side_hit_band_720(), (348.0, 372.0));
        let (lo, hi) = gutter_side_hit_band_720();
        assert!(((360.0 - lo) - (hi - 360.0)).abs() < 0.001);

        // Center gutter 24px layout: 12 + 336 + 24 + 336 + 12 = 720; cx also 360.
        assert_eq!(gutter_center_column_x_720(), 348.0);
        assert_eq!(gutter_center_arrow_cx_720(), Some(360.0));
        assert_eq!(
            gutter_center_glyph_720(200.0),
            Some((354.0, 194.0, 366.0, 206.0))
        );
        assert_eq!(gutter_center_glyph_720(f32::NAN), None);

        // Char metrics at 720:
        // Left pane (12..350): 1-digit line numbers col 16px -> text_x 26px (col 12 + col 16 - inset 2).
        assert_eq!(gutter_linenum_column_width(9, true), 16.0);
        assert_eq!(gutter_linenum_text_x(12.0, 16.0), Some(26.0));
        assert_eq!(gutter_text_gutter_width(9, true), 32.0);
        assert!(gutter_text_gutter_width(9, true) < left);
        // Right pane (370..708): 1-digit line numbers col 16px -> text_x 384px (col 370 + col 16 - inset 2).
        assert_eq!(gutter_linenum_text_x(370.0, 16.0), Some(384.0));

        // Marker visibility only on changed rows (diff kind != Equal).
        assert!(!gutter_side_marker_visible(TextRowKind::Equal));
        assert!(gutter_side_marker_visible(TextRowKind::Deleted));
        assert!(gutter_side_marker_visible(TextRowKind::Inserted));
        assert!(gutter_side_marker_visible(TextRowKind::Changed));

        let rows = vec![
            TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Deleted,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Changed,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Inserted,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
        ];
        assert_eq!(gutter_side_changed_rows(&rows), vec![1, 3, 5]);
        let geom = gutter_geometry(rows.len(), 800.0, &rows);
        assert_eq!(geom.segments.len(), 3);
    }
}

// ----------------------------------------------------------------------------
// GUTTER11 tall: 1200px canvas symmetric layout + tall scroll marker lock + viewport floor.
// BC center ~20px gutter column with yellow markers only on changed rows over tall
// scroll; tall-1200 canvas maintains exact symmetry around canvas center cx=600.0.
// Geometry first: side marker column 20px (SPACE_LG + SPACE_XS) layout 12+578+20+578+12=1200;
// center column 24px (SPACE_XL) layout 12+576+24+576+12=1200; both center at cx=600.0.
// Marker tall lock: cx stays strictly locked at 600.0 across all rows over long scroll.
// Viewport tall floor: min-h 4.0 floor maintained over tall scroll, bottom edge
// never escapes track. Typography second: no new font sizes. Colors frozen: shape only.
// WIRING (paint owner ke paas): tall canvas side layout `gutter_side_layout_1200()` se,
// center layout `gutter_text_layout_1200()` se, side marker cx `gutter_side_arrow_cx_1200()`,
// center marker cx `gutter_center_arrow_cx_1200()`, glyphs `gutter_side_glyph_1200()` /
// `gutter_center_glyph_1200()`, marker scroll lock `gutter_marker_tall_lock_cx_1200()`,
// and tall viewport floor `gutter_viewport_tall_floor_1200()`.
// ----------------------------------------------------------------------------

/// 1200px tall canvas symmetric side layout `(margin, left, center, right, margin)`
/// px: 12 + 578 + 20 + 578 + 12 = 1200. Panes barabar, 20px side column beech
/// me symmetric insets samet. Single source over [`gutter_text_layout`].
/// Pure; unit-tested.
pub fn gutter_side_layout_1200() -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        GUTTER_SIDE_MARKER_COL_W,
        GUTTER_CANVAS_WIDE_1200,
    )
}

/// 1200px tall canvas side-column x px: `margin + pane` from the symmetric
/// side layout (12 + 578 = 590). Column is [`GUTTER_SIDE_MARKER_COL_W`] wide (20).
/// Pure; unit-tested.
pub fn gutter_side_column_x_1200() -> f32 {
    let (margin, pane, _, _, _) = gutter_side_layout_1200();
    margin + pane
}

/// 1200px tall canvas side-marker cx px: column center (590 + 20/2 = 600 =
/// canvas middle), so the marker sits centered in the gutter. `None` only
/// when the column math goes non-finite. Pure; unit-tested.
pub fn gutter_side_arrow_cx_1200() -> Option<f32> {
    let x = gutter_side_column_x_1200();
    let w = GUTTER_SIDE_MARKER_COL_W;
    if !x.is_finite() || !w.is_finite() {
        return None;
    }
    let cx = x + w / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// 1200px tall canvas side glyph rect `(x0, y0, x1, y1)` px: 12px chevron
/// ([`gutter_arrow_glyph_rect`]) centered at canvas middle (600) for the
/// given row-center y. `None` for non-finite y. Pure; unit-tested.
pub fn gutter_side_glyph_1200(center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_side_arrow_cx_1200()?;
    gutter_arrow_glyph_rect(cx, center_y)
}

/// 1200px tall canvas side hit band `(lo, hi)` px: side column + symmetric slop on
/// both sides (590 - 2 .. 610 + 2 = 588..612), canvas center 600 ke around
/// barabar. Pure; unit-tested.
pub fn gutter_side_hit_band_1200() -> (f32, f32) {
    let x = gutter_side_column_x_1200();
    let w = GUTTER_SIDE_MARKER_COL_W;
    let s = GUTTER_SIDE_HIT_SLOP;
    (x - s, x + w + s)
}

/// 1200px tall canvas symmetric text layout `(margin, left, center, right, margin)`
/// px: 12 + 576 + 24 + 576 + 12 = 1200. Panes barabar, 24px center column beech
/// me symmetric insets samet. Single source over [`gutter_text_layout`].
/// Pure; unit-tested.
pub fn gutter_text_layout_1200() -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        GUTTER_CENTER_W,
        GUTTER_CANVAS_WIDE_1200,
    )
}

/// 1200px tall canvas center-column x px: `margin + pane` from the symmetric
/// text layout (12 + 576 = 588). Column is [`GUTTER_CENTER_W`] wide (24).
/// Pure; unit-tested.
pub fn gutter_center_column_x_1200() -> f32 {
    let (margin, pane, _, _, _) = gutter_text_layout_1200();
    margin + pane
}

/// 1200px tall canvas center-arrow cx px: column center (588 + 24/2 = 600 =
/// canvas middle), so the glyph sits centered in the gutter. `None` only
/// when the column math goes non-finite. Pure; unit-tested.
pub fn gutter_center_arrow_cx_1200() -> Option<f32> {
    gutter_arrow_center_x(gutter_center_column_x_1200())
}

/// 1200px tall canvas center glyph rect `(x0, y0, x1, y1)` px: 12px chevron
/// ([`gutter_arrow_glyph_rect`]) centered at canvas middle (600) for the
/// given row-center y. `None` for non-finite y. Pure; unit-tested.
pub fn gutter_center_glyph_1200(center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_center_arrow_cx_1200()?;
    gutter_arrow_glyph_rect(cx, center_y)
}

/// 1200px tall canvas marker horizontal center lock over long scroll:
/// regardless of row index during scroll, the marker center `cx` remains
/// strictly locked at the canvas middle (600.0 px). Returns `Some(600.0)`
/// for valid row indices (`row < total_rows`), `None` when `total_rows == 0`
/// or `row >= total_rows`. Pure; unit-tested.
pub fn gutter_marker_tall_lock_cx_1200(row: usize, total_rows: usize) -> Option<f32> {
    if total_rows == 0 || row >= total_rows {
        return None;
    }
    gutter_side_arrow_cx_1200()
}

/// 1200px tall canvas center-marker horizontal center lock over long scroll:
/// center column arrow cx locked at 600.0 across all valid row indices.
/// Pure; unit-tested.
pub fn gutter_center_marker_tall_lock_cx_1200(row: usize, total_rows: usize) -> Option<f32> {
    if total_rows == 0 || row >= total_rows {
        return None;
    }
    gutter_center_arrow_cx_1200()
}

/// 1200px tall scroll viewport band `(y0, y1)` in logical pixels with min-h floor:
/// wraps [`gutter_viewport_rect_padded`] over 1200.0px height, guaranteeing
/// the visible window band never collapses below [`GUTTER_VIEWPORT_MIN_H`] (4.0px)
/// and stays within `[GUTTER_MINIMAP_TRACK_PAD, 1200.0 - GUTTER_MINIMAP_TRACK_PAD]`.
/// Pure; unit-tested.
pub fn gutter_viewport_tall_floor_1200(
    total_rows: usize,
    top_row: usize,
    visible_rows: usize,
) -> Option<(f32, f32)> {
    gutter_viewport_rect_padded(total_rows, GUTTER_CANVAS_WIDE_1200, top_row, visible_rows)
}

#[cfg(test)]
mod exq_gutter11_tests {
    use super::*;

    #[test]
    fn gutter11_tall_1200_center_column_and_marker_lock() {
        // Mirrors checks/gui-exq-gutter11/round-1/comparison/gutter-tall-1200.json.
        let fixture_raw = include_str!(
            "../../../checks/gui-exq-gutter11/round-1/comparison/gutter-tall-1200.json"
        );
        assert!(fixture_raw.contains("\"arrow_cx_1200\": 600.0"));
        assert!(fixture_raw.contains("\"center_arrow_cx_1200\": 600.0"));
        assert!(fixture_raw.contains("\"center_column_x_1200\": 588.0"));
        assert!(fixture_raw.contains("\"column_x_1200\": 590.0"));
        assert!(fixture_raw.contains("\"marker_tall_lock_cx\": 600.0"));

        // Tall side layout 1200 symmetric: 12 + 578 + 20 + 578 + 12 = 1200.
        assert_eq!(gutter_side_layout_1200(), (12.0, 578.0, 20.0, 578.0, 12.0));
        let (m0, left, center, right, m1) = gutter_side_layout_1200();
        assert_eq!(m0, m1);
        assert_eq!(left, right);
        assert!((m0 + left + center + right + m1 - 1200.0).abs() < 0.001);
        assert_eq!(center, GUTTER_SIDE_MARKER_COL_W);

        // Column x 590, cx 600 canvas middle; glyph + hit symmetric about 600.
        assert_eq!(gutter_side_column_x_1200(), 590.0);
        assert_eq!(gutter_side_arrow_cx_1200(), Some(600.0));
        assert_eq!(
            gutter_side_marker_rect(600.0, 600.0),
            Some((592.0, 592.0, 608.0, 608.0))
        );
        assert_eq!(
            gutter_side_hit_rect(600.0, 600.0),
            Some((590.0, 590.0, 610.0, 610.0))
        );
        assert_eq!(
            gutter_side_glyph_1200(600.0),
            Some((594.0, 594.0, 606.0, 606.0))
        );
        assert_eq!(gutter_side_glyph_1200(f32::NAN), None);
        assert_eq!(gutter_side_hit_band_1200(), (588.0, 612.0));
        let (lo, hi) = gutter_side_hit_band_1200();
        assert!(((600.0 - lo) - (hi - 600.0)).abs() < 0.001);

        // Center gutter 24px layout: 12 + 576 + 24 + 576 + 12 = 1200; cx also 600.
        assert_eq!(gutter_text_layout_1200(), (12.0, 576.0, 24.0, 576.0, 12.0));
        assert_eq!(gutter_center_column_x_1200(), 588.0);
        assert_eq!(gutter_center_arrow_cx_1200(), Some(600.0));
        assert_eq!(
            gutter_center_glyph_1200(600.0),
            Some((594.0, 594.0, 606.0, 606.0))
        );
        assert_eq!(gutter_center_glyph_1200(f32::NAN), None);

        // Marker tall lock: cx stays 600 across long scroll rows 0..1200.
        let total_rows = 1200usize;
        for row in [0, 10, 100, 500, 600, 1100, 1199] {
            assert_eq!(
                gutter_marker_tall_lock_cx_1200(row, total_rows),
                Some(600.0)
            );
            assert_eq!(
                gutter_center_marker_tall_lock_cx_1200(row, total_rows),
                Some(600.0)
            );
        }
        assert_eq!(gutter_marker_tall_lock_cx_1200(1200, total_rows), None);
        assert_eq!(gutter_marker_tall_lock_cx_1200(0, 0), None);
        assert_eq!(
            gutter_center_marker_tall_lock_cx_1200(1200, total_rows),
            None
        );
        assert_eq!(gutter_center_marker_tall_lock_cx_1200(0, 0), None);

        // Viewport tall floor over 1200px scroll:
        // Mid-viewport (row 600, 1 visible):
        let vp_mid = gutter_viewport_tall_floor_1200(1200, 600, 1);
        assert!(vp_mid.is_some());
        if let Some((y0, y1)) = vp_mid {
            assert!((y1 - y0 - GUTTER_VIEWPORT_MIN_H).abs() < 0.01);
            assert!(y0 >= GUTTER_MINIMAP_TRACK_PAD);
            assert!(y1 <= 1200.0 - GUTTER_MINIMAP_TRACK_PAD);
            assert!((y0 - 598.0).abs() < 2.5);
        }

        // Bottom-viewport (row 1199, 1 visible): floor >= 4.0, locked within [4.0, 1196.0].
        assert_eq!(
            gutter_viewport_tall_floor_1200(1200, 1199, 1),
            Some((1192.0, 1196.0))
        );

        // Top-viewport (row 0, 1 visible):
        assert_eq!(
            gutter_viewport_tall_floor_1200(1200, 0, 1),
            Some((4.0, 8.0))
        );

        // Full file visible:
        assert_eq!(
            gutter_viewport_tall_floor_1200(1200, 0, 1200),
            Some((4.0, 1196.0))
        );

        // Invalid inputs:
        assert_eq!(gutter_viewport_tall_floor_1200(0, 0, 1), None);
        assert_eq!(gutter_viewport_tall_floor_1200(1200, 0, 0), None);
        assert_eq!(gutter_viewport_tall_floor_1200(1200, 1200, 1), None);
    }
}

// ----------------------------------------------------------------------------
// TEXT5: separate number gutter + icon column + symmetric center divider.
// BC text compare me line numbers alag column me, change icons alag column
// me, aur beech me arrows wala divider dono panes se barabar gap par rehta
// hai; yahan numbers code ke andar inline paint hote the, icon ke liye koi
// reserve nahi tha, aur panes divider se chipke lagte the. Geometry first:
// number column (digits se chauda, SPACE scale) + icon reserve (arrow-box
// parity) = poora text gutter; center divider side gaps barabar (arrow box
// + 2 * side gap = center width); 960 par panes barabar. Hit-slop dono
// taraf barabar. Row center Mono line-height pitch par lock. Typography
// second: sab wahi content_grid (Mono 12 tabular) role, koi naya size nahi.
// Colors frozen: sirf shape.
// WIRING (paint owner ke paas): number column
// `gutter_linenum_column_width(..)` chaudi, icon strip
// `gutter_icon_column_width()` chaudi, poora gutter
// `gutter_text_gutter_width(..)` chauda; center column
// `gutter_center_width()` me arrow `gutter_arrow_box()` ka, side gaps
// `gutter_center_side_gap()` ke; clicks `gutter_slop_band(..)` se.
// ----------------------------------------------------------------------------

/// Change-icon column reserve px: one LG step (arrow-box parity, 16.0).
///
/// Numbers ke baad icons ke liye alag fixed strip taaki number column
/// digits ke saath chauda ho par icon kabhi number/code se na takraye.
/// [`crate::theme::SPACE_LG`] reuse, koi naya literal nahi.
pub const GUTTER_ICON_COL_W: f32 = crate::theme::SPACE_LG;

/// Center-divider side gap px per side: one XS step (4.0).
///
/// Divider ke dono taraf barabar gap taaki panes divider se chipke na
/// lagen: `GUTTER_ARROW_BOX + 2 * side gap == GUTTER_CENTER_W`
/// (16 + 2*4 == 24). [`crate::theme::SPACE_XS`] reuse.
pub const GUTTER_CENTER_SIDE_GAP: f32 = crate::theme::SPACE_XS;

/// Icon column reserve px (single source). Pure.
pub fn gutter_icon_column_width() -> f32 {
    GUTTER_ICON_COL_W
}

/// Center-divider side gap px per side (single source). Pure.
pub fn gutter_center_side_gap() -> f32 {
    GUTTER_CENTER_SIDE_GAP
}

/// Poora text gutter width px: number column + icon reserve.
///
/// Numbers alag, icons alag — dono judkar ek tabular gutter banate hain
/// taaki paint numbers inline na ghole. Pure; unit-tested.
pub fn gutter_text_gutter_width(max_line_no: usize, show_line_numbers: bool) -> f32 {
    gutter_linenum_column_width(max_line_no, show_line_numbers) + GUTTER_ICON_COL_W
}

/// Mono line-height px (measured Mono row pitch, 12 * 1.30 = 15.6).
///
/// Row center math isi pitch par lock hai taaki gutter markers text rows
/// se ek beat par rahen. Pure; unit-tested.
pub fn gutter_mono_line_height_px() -> f32 {
    crate::theme::TypoRole::content_grid().line_height_px()
}

/// Mono pitch par row center y px: `(row + 0.5) * line_h`.
///
/// [`gutter_row_center_y`] viewport-fraction math ka line-pitch twin:
/// paint text rows isi pitch par bichhata hai, isliye markers yahi se
/// align hote hain. `None`: out-of-range row ya non-finite/non-positive
/// pitch. Pure; unit-tested.
pub fn gutter_row_center_y_for_line_height(
    row: usize,
    total_rows: usize,
    line_h: f32,
) -> Option<f32> {
    if total_rows == 0 || row >= total_rows {
        return None;
    }
    if !line_h.is_finite() || line_h <= 0.0 {
        return None;
    }
    let y = (row as f32 + 0.5) * line_h;
    if y.is_finite() && y >= 0.0 {
        Some(y)
    } else {
        None
    }
}

/// Slop-expanded band `(lo, hi)` px: dono taraf barabar `slop`.
///
/// Hit-slop symmetric hai — low side `0.0` par clamp ke siwa dono taraf
/// same pad, taaki click target badhe par paint na hile. Non-finite ya
/// non-positive slop par band jaisa-ka-taisa. Pure; unit-tested.
pub fn gutter_slop_band(seg: &GutterSegment, slop: f32) -> Option<(f32, f32)> {
    if !seg.y_start.is_finite() || !seg.y_end.is_finite() {
        return None;
    }
    if seg.y_end < seg.y_start {
        return None;
    }
    let pad = if slop.is_finite() && slop > 0.0 {
        slop
    } else {
        0.0
    };
    let lo = (seg.y_start - pad).max(0.0);
    let hi = seg.y_end + pad;
    if lo.is_finite() && hi.is_finite() && lo <= hi {
        Some((lo, hi))
    } else {
        None
    }
}

/// 960px canvas par symmetric text layout `(margin, left, center, right,
/// margin)` px: bahari margins barabar, dono panes barabar, divider
/// beech me barabar gaps samet.
///
/// `margin` = [`crate::theme::GEO_OUTER_MARGIN`] (12), `center` =
/// [`GUTTER_CENTER_W`] (24), panes me bachi jagah aadhi-aadhi:
/// (960 - 2*12 - 24) / 2 = 456. Panes divider se chipakte nahi (side
/// gaps center ke andar symmetric). Pure; unit-tested.
pub fn gutter_text5_layout_960() -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        GUTTER_CENTER_W,
        GUTTER_CANVAS_NORMAL_960,
    )
}

/// Kisi bhi canvas width par symmetric text layout: margins bahar barabar,
/// panes andar barabar. Na-mumkin budget (center + margins se chhota
/// canvas, kharab input) par panes zero. Pure; unit-tested.
pub fn gutter_text_layout(margin: f32, center: f32, avail_w: f32) -> (f32, f32, f32, f32, f32) {
    if !margin.is_finite() || !center.is_finite() || !avail_w.is_finite() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    if margin < 0.0 || center < 0.0 || avail_w <= 0.0 {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    let rest = avail_w - 2.0 * margin - center;
    if !rest.is_finite() || rest <= 0.0 {
        return (margin.max(0.0), 0.0, center, 0.0, margin.max(0.0));
    }
    let pane = rest / 2.0;
    (margin, pane, center, pane, margin)
}

#[cfg(test)]
mod exq_text5_tests {
    use super::*;

    #[test]
    fn text5_gutter_splits_numbers_and_icons_on_space_scale() {
        // Mirrors checks/gui-exq-text5/comparison/text-gutter-960.json.
        assert_eq!(GUTTER_ICON_COL_W, crate::theme::SPACE_LG);
        assert_eq!(GUTTER_ICON_COL_W, 16.0);
        assert_eq!(gutter_icon_column_width(), 16.0);
        // 1-digit number col 16 + icon 16 = 32; 3-digit 24 + 16 = 40.
        assert_eq!(gutter_linenum_column_width(9, true), 16.0);
        assert_eq!(gutter_text_gutter_width(9, true), 32.0);
        assert_eq!(gutter_linenum_column_width(120, true), 24.0);
        assert_eq!(gutter_text_gutter_width(120, true), 40.0);
        // Numbers chhupe par bhi icon reserve rehta hai (narrow 8 + 16 = 24).
        assert_eq!(gutter_text_gutter_width(120, false), 24.0);
    }

    #[test]
    fn text5_center_divider_gaps_are_symmetric_on_space_scale() {
        // Mirrors checks/gui-exq-text5/comparison/text-gutter-960.json.
        assert_eq!(GUTTER_CENTER_SIDE_GAP, crate::theme::SPACE_XS);
        assert_eq!(GUTTER_CENTER_SIDE_GAP, 4.0);
        assert_eq!(gutter_center_side_gap(), 4.0);
        // Arrow box + 2 * side gap == center width (16 + 8 == 24).
        assert_eq!(gutter_arrow_box(), crate::theme::SPACE_LG);
        assert!(
            (gutter_arrow_box() + 2.0 * gutter_center_side_gap() - gutter_center_width()).abs()
                < 0.001
        );
        assert_eq!(gutter_center_width(), crate::theme::SPACE_XL);
        assert_eq!(gutter_center_width(), 24.0);
    }

    #[test]
    fn text5_layout_960_is_symmetric_with_equal_panes() {
        // Mirrors checks/gui-exq-text5/comparison/text-gutter-960.json:
        // 12 + 456 + 24 + 456 + 12 = 960.
        assert_eq!(gutter_text5_layout_960(), (12.0, 456.0, 24.0, 456.0, 12.0));
        let (m0, left, center, right, m1) = gutter_text5_layout_960();
        assert_eq!(m0, m1);
        assert_eq!(left, right);
        assert!((m0 + left + center + right + m1 - 960.0).abs() < 0.001);
        // Generic twin bhi symmetric rehta hai; na-mumkin par zero panes.
        assert_eq!(
            gutter_text_layout(12.0, 24.0, 960.0),
            gutter_text5_layout_960()
        );
        assert_eq!(
            gutter_text_layout(12.0, 24.0, 40.0),
            (12.0, 0.0, 24.0, 0.0, 12.0)
        );
        assert_eq!(
            gutter_text_layout(12.0, 24.0, f32::NAN),
            (0.0, 0.0, 0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn text5_hit_slop_expands_symmetrically() {
        // Mirrors checks/gui-exq-text5/comparison/text-gutter-960.json:
        // band 200..204 + slop 2 -> 198..206 (dono taraf 2).
        let seg = GutterSegment {
            y_start: 200.0,
            y_end: 204.0,
            kind: TextRowKind::Deleted,
        };
        assert_eq!(
            gutter_slop_band(&seg, crate::theme::GUTTER_MERGE_GAP),
            Some((198.0, 206.0))
        );
        let Some((lo, hi)) = gutter_slop_band(&seg, GUTTER_HIT_SLOP) else {
            panic!("band must exist");
        };
        assert!(((seg.y_start - lo) - (hi - seg.y_end)).abs() < 0.001);
        assert_eq!(GUTTER_HIT_SLOP, 2.0);
        // Zero/kharab slop par band jaisa-ka-taisa; ulta band None.
        assert_eq!(gutter_slop_band(&seg, 0.0), Some((200.0, 204.0)));
        assert_eq!(gutter_slop_band(&seg, f32::NAN), Some((200.0, 204.0)));
        let bad = GutterSegment {
            y_start: 300.0,
            y_end: 100.0,
            kind: TextRowKind::Deleted,
        };
        assert_eq!(gutter_slop_band(&bad, 2.0), None);
    }

    #[test]
    fn text5_row_center_locks_to_mono_line_height() {
        // Mirrors checks/gui-exq-text5/comparison/text-gutter-960.json:
        // Mono 12 @1.30 = 15.6 pitch; row 1 center = 1.5 * 15.6 = 23.4.
        assert!((gutter_mono_line_height_px() - 15.6).abs() < 0.001);
        assert_eq!(
            gutter_mono_line_height_px(),
            crate::theme::TypoRole::content_grid().line_height_px()
        );
        assert!(
            gutter_row_center_y_for_line_height(1, 4, 15.6)
                .is_some_and(|v| (v - 23.4).abs() < 0.01)
        );
        assert!(
            gutter_row_center_y_for_line_height(0, 4, 15.6).is_some_and(|v| (v - 7.8).abs() < 0.01)
        );
        // Pitch badle to center usi pitch par (19.6 text-row pitch par row 0 = 9.8).
        assert!(
            gutter_row_center_y_for_line_height(0, 4, 19.6).is_some_and(|v| (v - 9.8).abs() < 0.01)
        );
        assert_eq!(gutter_row_center_y_for_line_height(4, 4, 15.6), None);
        assert_eq!(gutter_row_center_y_for_line_height(0, 0, 15.6), None);
        assert_eq!(gutter_row_center_y_for_line_height(0, 4, 0.0), None);
        assert_eq!(gutter_row_center_y_for_line_height(0, 4, f32::NAN), None);
    }
}

// ----------------------------------------------------------------------------
// GUTTER6 narrow: 720px canvas symmetric layout + char-metric numbers.
// BC twin panes 720 par bhi barabar rehte hain (margins bahar barabar,
// panes andar barabar, divider beech me barabar gaps samet); number column
// digits (chars) se chauda hota hai, icon reserve fixed LG rehta hai, slop
// dono taraf barabar. Geometry first: sab SPACE scale reuse, koi naya
// literal nahi (720 canvas fixture width hai). Typography second: numbers
// wahi content_grid (Mono 12 tabular) chars, koi naya size nahi.
// WIRING (paint owner ke paas): narrow canvas `gutter_text_layout_720()`
// se, number cell char count `gutter_number_cell_width_chars(..)` se.
// ----------------------------------------------------------------------------

/// 720px narrow canvas par symmetric text layout `(margin, left, center,
/// right, margin)` px: 12 + 336 + 24 + 336 + 12 = 720. Panes barabar,
/// divider symmetric gaps samet. Pure; unit-tested.
pub fn gutter_text_layout_720() -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        GUTTER_CENTER_W,
        crate::theme::VIEWPORT_MIN_W as f32,
    )
}

/// Number-cell char width (chars): digit count + trailing pad.
///
/// Px column ([`gutter_linenum_column_width`]) digits se chauda hota hai,
/// text cell ([`gutter_number_cell`]) utne hi chars ki hoti hai (digits +
/// [`GUTTER_NUMBER_TRAIL_PAD`] spaces) taaki px aur char metrics ek beat
/// par rahen. Pure; unit-tested.
pub fn gutter_number_cell_width_chars(digits: usize) -> usize {
    digits.saturating_add(GUTTER_NUMBER_TRAIL_PAD)
}

#[cfg(test)]
mod exq_gutter6_narrow_tests {
    use super::*;

    #[test]
    fn gutter6_narrow_720_stays_symmetric_with_char_metrics() {
        // Mirrors checks/gui-exq-gutter6/round-1/comparison/gutter-narrow-720.json.
        // Narrow layout symmetric: 12 + 336 + 24 + 336 + 12 = 720.
        assert_eq!(gutter_text_layout_720(), (12.0, 336.0, 24.0, 336.0, 12.0));
        let (m0, left, center, right, m1) = gutter_text_layout_720();
        assert_eq!(m0, m1);
        assert_eq!(left, right);
        assert!((m0 + left + center + right + m1 - 720.0).abs() < 0.001);
        assert_eq!(
            gutter_text_layout_720(),
            gutter_text_layout(crate::theme::GEO_OUTER_MARGIN, GUTTER_CENTER_W, 720.0)
        );
        // Number gutter widths char metrics: digits -> px col, chars = digits + trail.
        assert_eq!(gutter_linenum_digits(9), 1);
        assert_eq!(gutter_linenum_column_width(9, true), 16.0);
        assert_eq!(gutter_number_cell_width_chars(1), 3);
        assert_eq!(gutter_number_cell_width_chars(4), 6);
        assert_eq!(
            gutter_number_cell(Some(1), 4).chars().count(),
            gutter_number_cell_width_chars(4)
        );
        assert_eq!(
            gutter_number_cell(None, 4).chars().count(),
            gutter_number_cell_width_chars(4)
        );
        assert!(gutter_number_cell_is_right_aligned_tabular());
        // Icon col fixed LG, center gaps fixed XS, arrow box centered.
        assert_eq!(gutter_icon_column_width(), crate::theme::SPACE_LG);
        assert_eq!(gutter_icon_column_width(), 16.0);
        assert_eq!(gutter_center_side_gap(), crate::theme::SPACE_XS);
        assert_eq!(gutter_center_side_gap(), 4.0);
        assert!(
            (gutter_arrow_box() + 2.0 * gutter_center_side_gap() - gutter_center_width()).abs()
                < 0.001
        );
        // Slop symmetric dono taraf: 200..204 + 2 -> 198..206.
        let seg = GutterSegment {
            y_start: 200.0,
            y_end: 204.0,
            kind: TextRowKind::Deleted,
        };
        assert_eq!(
            gutter_slop_band(&seg, GUTTER_HIT_SLOP),
            Some((198.0, 206.0))
        );
        let Some((lo, hi)) = gutter_slop_band(&seg, GUTTER_HIT_SLOP) else {
            return;
        };
        assert!(((seg.y_start - lo) - (hi - seg.y_end)).abs() < 0.001);
        // 720 par text gutter pane me samae: 1-digit 32 / 3-digit 40 << 336 pane.
        assert_eq!(gutter_text_gutter_width(9, true), 32.0);
        assert_eq!(gutter_text_gutter_width(120, true), 40.0);
        assert!(gutter_text_gutter_width(120, true) < left);
    }
}

// ----------------------------------------------------------------------------
// GUTTER7 round 2: single-source jump mapping + hairline viewport border +
// diff-ordered bands + symmetric 720 narrow slop (headless).
// BC minimap bands diff order me padhe jate hain (pehle Deleted, phir
// Inserted — readout diff order se match), click-to-jump tall heights par
// bhi wahi single frac->row helper se map hota hai, aur viewport rect ka
// border hairline hota hai taaki ruler bands dhaken nahi. Geometry first:
// sab SPACE/GUTTER/underline scale reuse, koi naya literal nahi.
// Typography second: kuch nahi (koi text yahan nahi). Colors frozen: sirf
// shape (33 touches untouched).
// WIRING (paint owner ke paas): clicks `gutter_row_at_y` /
// `gutter_padded_row_at_y` / `gutter_jump_target` se (sab
// `gutter_row_from_frac` single source), viewport rect stroke
// `gutter_viewport_border_w()` se, band readout
// `gutter_overview_diff_kinds(..)` diff order me.
// ----------------------------------------------------------------------------

/// Viewport rect border width px: hairline underline reuse (1.0).
///
/// Ruler viewport band ka stroke hairline rehta hai taaki chhoti window
/// ka band bhi ruler bands ko dhake nahi. Paint stroke isi se aaye;
/// geometry bands ([`gutter_viewport_rect`]) untouched rehte hain.
pub const GUTTER_VIEWPORT_BORDER_W: f32 = crate::theme::TEXT_INLINE_SPAN_UNDERLINE_W;

/// Viewport rect border width px (single source). Pure.
pub fn gutter_viewport_border_w() -> f32 {
    GUTTER_VIEWPORT_BORDER_W
}

/// Single-source frac->row: `floor(frac * total)` clamped to last row.
///
/// `gutter_row_at_y`, [`gutter_ruler_hit_test`] aur
/// [`gutter_padded_row_at_y`] sab yahi se map hote hain taaki tall heights
/// par click-to-jump ek hi math se gire. `None`: khali total ya
/// non-finite/negative frac. `frac >= 1` aakhri row par clamp hota hai
/// (callers `[0, height)` guard khud rakhte hain). Pure; unit-tested.
pub fn gutter_row_from_frac(frac: f32, total_rows: usize) -> Option<usize> {
    if total_rows == 0 {
        return None;
    }
    if !frac.is_finite() || frac < 0.0 {
        return None;
    }
    let clamped = frac.clamp(0.0, 1.0);
    let row = (clamped * total_rows as f32).floor() as usize;
    Some(row.min(total_rows.saturating_sub(1)))
}

/// Diff band kinds row order me (merged runs, Equal skip).
///
/// [`gutter_geometry`] segments ke kinds ka row-order twin: lagatar same
/// kind ek band, Equal runs skip. Minimap readout isi order se match
/// hota hai. Pure; unit-tested.
pub fn gutter_diff_band_kinds(rows: &[TextRow]) -> Vec<TextRowKind> {
    let mut out: Vec<TextRowKind> = Vec::new();
    let mut idx = 0usize;
    while idx < rows.len() {
        let kind = match rows.get(idx) {
            Some(row) => row.kind,
            None => break,
        };
        if kind == TextRowKind::Equal {
            idx = idx.saturating_add(1);
            continue;
        }
        if out.last().is_none_or(|last| *last != kind) {
            out.push(kind);
        }
        idx = idx.saturating_add(1);
    }
    out
}

/// Overview segments me diff kinds segment order me (Equal background skip).
///
/// Minimap readout contract: yeh [`gutter_diff_band_kinds`] ke barabar
/// hona chahiye — bands diff order me, same/diff heights ek scale par.
/// Pure; unit-tested.
pub fn gutter_overview_diff_kinds(geom: &GutterGeometry) -> Vec<TextRowKind> {
    let mut out: Vec<TextRowKind> = Vec::new();
    for seg in &geom.segments {
        if seg.kind != TextRowKind::Equal && out.last().is_none_or(|last| *last != seg.kind) {
            out.push(seg.kind);
        }
    }
    out
}

// ----------------------------------------------------------------------------
// GUTTER8 round-2: changed-only center markers + SPACE-width column at 960.
// BC center gutter shows change markers ONLY at changed rows (Equal rows
// blank click-to-select), column ~20px between panes from SPACE tokens,
// arrow glyph centered in the column at 960px wide. Geometry first: marker
// visibility gated on the changed-row flag (single source), column width
// and glyph insets all SPACE scale reuse, no new literals. Typography
// second: nothing (no text here). Colors frozen: shape only (33 touches).
// WIRING (paint owner ke paas): per-row marker `gutter_marker_visible(..)`
// se gate karo (false -> blank, no arrow); changed rows
// `gutter_changed_rows(..)` se; center column x
// `gutter_center_column_x_960()` se, arrow cx `gutter_center_arrow_cx_960()`
// se, glyph `gutter_center_glyph_960(..)` se.
// ----------------------------------------------------------------------------

/// Center marker visibility for one row: changed rows only.
///
/// Single source for paint gating: `Equal` rows stay blank (no arrow),
/// every diff kind (`Deleted`/`Inserted`/`Changed`) gets a marker.
/// Pure; unit-tested.
pub fn gutter_marker_visible(kind: TextRowKind) -> bool {
    kind != TextRowKind::Equal
}

/// Changed-row indices in row order: every row paint must mark.
///
/// `Equal` rows never appear here, so markers land ONLY on changed rows.
/// Pure; unit-tested.
pub fn gutter_changed_rows(rows: &[TextRow]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    let mut idx = 0usize;
    while idx < rows.len() {
        match rows.get(idx) {
            Some(row) => {
                if gutter_marker_visible(row.kind) {
                    out.push(idx);
                }
                idx = idx.saturating_add(1);
            }
            None => break,
        }
    }
    out
}

/// 960px canvas center-column x px: `margin + pane` from the symmetric
/// text layout (12 + 456 = 468). Column is [`GUTTER_CENTER_W`] wide
/// ([`crate::theme::SPACE_XL`], 24). Pure; unit-tested.
pub fn gutter_center_column_x_960() -> f32 {
    let (margin, pane, _, _, _) = gutter_text5_layout_960();
    margin + pane
}

/// 960px canvas center-arrow cx px: column center (468 + 24/2 = 480 =
/// canvas middle), so the glyph sits centered in the gutter. `None` only
/// when the column math goes non-finite. Pure; unit-tested.
pub fn gutter_center_arrow_cx_960() -> Option<f32> {
    gutter_arrow_center_x(gutter_center_column_x_960())
}

/// 960px canvas center glyph rect `(x0, y0, x1, y1)` px: 12px chevron
/// ([`gutter_arrow_glyph_rect`]) centered at canvas middle (480) for the
/// given row-center y. `None` for non-finite y. Pure; unit-tested.
pub fn gutter_center_glyph_960(center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_center_arrow_cx_960()?;
    gutter_arrow_glyph_rect(cx, center_y)
}

#[cfg(test)]
mod exq_gutter8_tests {
    use super::*;

    fn mrow(kind: TextRowKind) -> TextRow {
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

    fn mixed_960_rows() -> Vec<TextRow> {
        vec![
            mrow(TextRowKind::Equal),
            mrow(TextRowKind::Deleted),
            mrow(TextRowKind::Equal),
            mrow(TextRowKind::Changed),
            mrow(TextRowKind::Equal),
            mrow(TextRowKind::Inserted),
            mrow(TextRowKind::Equal),
            mrow(TextRowKind::Equal),
        ]
    }

    #[test]
    fn gutter8_markers_only_on_changed_rows() {
        // Mirrors checks/gui-exq-gutter8/round-1/comparison/gutter-mixed-960.json.
        let rows = mixed_960_rows();
        assert!(!gutter_marker_visible(TextRowKind::Equal));
        assert!(gutter_marker_visible(TextRowKind::Deleted));
        assert!(gutter_marker_visible(TextRowKind::Inserted));
        assert!(gutter_marker_visible(TextRowKind::Changed));
        assert_eq!(gutter_changed_rows(&rows), vec![1, 3, 5]);
        assert_eq!(gutter_changed_rows(&rows).len(), 3);
        // Segments agree: diff-only geometry yields exactly the changed bands.
        let geom = gutter_geometry(rows.len(), 800.0, &rows);
        assert_eq!(geom.segments.len(), 3);
        let kinds: Vec<TextRowKind> = geom.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TextRowKind::Deleted,
                TextRowKind::Changed,
                TextRowKind::Inserted
            ]
        );
        assert_eq!(
            gutter_overview_diff_kinds(&gutter_overview_geometry(8, 800.0, &rows)),
            kinds
        );
        // Every Equal row maps to no segment: markers never land there.
        let mut idx = 0usize;
        while idx < rows.len() {
            if let Some(row) = rows.get(idx) {
                if row.kind == TextRowKind::Equal {
                    if let Some(cy) = gutter_row_center_y(idx, rows.len(), 800.0) {
                        assert_eq!(gutter_segment_at_y(&geom.segments, cy), None);
                    }
                }
            }
            idx = idx.saturating_add(1);
        }
        assert!(gutter_changed_rows(&[]).is_empty());
    }

    #[test]
    fn gutter8_center_width_from_space_tokens_centered_at_960() {
        // Mirrors checks/gui-exq-gutter8/round-1/comparison/gutter-mixed-960.json:
        // column 24 (SPACE_XL), 12 + 456 + 24 + 456 + 12 = 960, cx 480.
        assert_eq!(GUTTER_CENTER_W, crate::theme::SPACE_XL);
        assert_eq!(gutter_center_width(), 24.0);
        assert_eq!(gutter_text5_layout_960(), (12.0, 456.0, 24.0, 456.0, 12.0));
        assert_eq!(gutter_center_column_x_960(), 468.0);
        assert_eq!(gutter_center_arrow_cx_960(), Some(480.0));
        // Glyph 12px chevron centered in the 24px gutter at canvas middle.
        assert_eq!(
            gutter_center_glyph_960(200.0),
            Some((474.0, 194.0, 486.0, 206.0))
        );
        assert_eq!(gutter_center_glyph_960(f32::NAN), None);
        // Arrow box + 2 * side gap == center width (16 + 8 == 24).
        assert!(
            (gutter_arrow_box() + 2.0 * gutter_center_side_gap() - gutter_center_width()).abs()
                < 0.001
        );
        // Hit target ~20px (16 box + 2 slop/side) centered at 480: 470..490.
        assert_eq!(
            gutter_arrow_hit_rect(480.0, 200.0),
            Some((470.0, 190.0, 490.0, 210.0))
        );
    }
}

#[cfg(test)]
mod exq_gutter7_tests {
    use super::*;

    fn trow(kind: TextRowKind) -> TextRow {
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

    fn tall_1200_rows() -> Vec<TextRow> {
        let mut rows = Vec::new();
        let mut i = 0usize;
        while i < 1200 {
            rows.push(trow(TextRowKind::Equal));
            i = i.saturating_add(1);
        }
        if let Some(row) = rows.get_mut(600) {
            row.kind = TextRowKind::Deleted;
        }
        if let Some(row) = rows.get_mut(1199) {
            row.kind = TextRowKind::Inserted;
        }
        rows
    }

    #[test]
    fn gutter7_single_source_frac_maps_all_entry_points() {
        // Mirrors checks/gui-exq-gutter7/round-1/comparison/gutter-tall-1200.json.
        assert_eq!(gutter_row_from_frac(0.0, 1200), Some(0));
        assert_eq!(gutter_row_from_frac(0.5, 1200), Some(600));
        assert_eq!(gutter_row_from_frac(1.0, 1200), Some(1199));
        assert_eq!(gutter_row_from_frac(-0.1, 1200), None);
        assert_eq!(gutter_row_from_frac(f32::NAN, 1200), None);
        assert_eq!(gutter_row_from_frac(0.5, 0), None);
        // Tall: 1200 rows @400px, y 200 -> frac 0.5 -> row 600 (diff).
        assert_eq!(gutter_row_at_y(200.0, 1200, 400.0), Some(600));
        assert_eq!(gutter_ruler_hit_test(200.0, 400.0, 1200), Some(600));
        // Ruler clamps tall overshoot to last row; row_at_y rejects it.
        assert_eq!(gutter_ruler_hit_test(500.0, 400.0, 1200), Some(1199));
        assert_eq!(gutter_row_at_y(400.0, 1200, 400.0), None);
        // Same frac, same row: entry points disagree nahi karte.
        assert_eq!(
            gutter_row_at_y(200.0, 1200, 400.0),
            gutter_ruler_hit_test(200.0, 400.0, 1200)
        );
    }

    #[test]
    fn gutter7_jump_mapping_hits_diffs_at_tall_heights() {
        // Mirrors checks/gui-exq-gutter7/round-1/comparison/gutter-tall-1200.json.
        let rows = tall_1200_rows();
        // Diff par click wahi rehta hai.
        assert_eq!(gutter_jump_target(&rows, 1200, 400.0, 200.0), Some(600));
        // Equal head par click aage diff par girta hai (row 0 -> 600).
        assert_eq!(gutter_jump_target(&rows, 1200, 400.0, 0.0), Some(600));
        // Aakhri band par click aakhri diff par girta hai.
        assert_eq!(gutter_jump_target(&rows, 1200, 400.0, 399.9), Some(1199));
        // Ruler hit-test single source se round-trip karta hai.
        if let Some(y) = gutter_y_for_row(600, 1200, 400.0) {
            assert_eq!(gutter_row_at_y(y + 0.1, 1200, 400.0), Some(600));
        } else {
            panic!("marker y must exist");
        }
    }

    #[test]
    fn gutter7_viewport_border_is_hairline_and_bands_stay_inside() {
        // Mirrors checks/gui-exq-gutter7/round-1/comparison/gutter-tall-1200.json.
        assert_eq!(
            GUTTER_VIEWPORT_BORDER_W,
            crate::theme::TEXT_INLINE_SPAN_UNDERLINE_W
        );
        assert_eq!(GUTTER_VIEWPORT_BORDER_W, 1.0);
        assert_eq!(gutter_viewport_border_w(), 1.0);
        // Tall viewport band track ke andar, border hairline (bands dhake nahi).
        assert_eq!(
            gutter_viewport_rect(1200, 400.0, 600, 12),
            Some((200.0, 204.0))
        );
        assert_eq!(
            gutter_viewport_rect_padded(1200, 400.0, 600, 12),
            Some((200.0, 204.0))
        );
        if let Some((y0, y1)) = gutter_viewport_rect(1200, 400.0, 600, 12) {
            assert!((y1 - y0 - GUTTER_VIEWPORT_MIN_H).abs() < 0.01);
            assert!(y0 >= 0.0 && y1 <= 400.0);
        } else {
            panic!("viewport band must exist");
        }
    }

    #[test]
    fn gutter7_bands_match_diff_order_and_slop_symmetric_at_720() {
        // Mirrors checks/gui-exq-gutter7/round-1/comparison/gutter-tall-1200.json
        // plus 720 narrow layout parity.
        let rows = tall_1200_rows();
        let diff_only = gutter_geometry(1200, 400.0, &rows);
        let over = gutter_overview_geometry(1200, 400.0, &rows);
        let over_padded = gutter_overview_geometry_padded(1200, 400.0, &rows);
        // Diff bands row order me: Deleted phir Inserted.
        assert_eq!(
            gutter_diff_band_kinds(&rows),
            vec![TextRowKind::Deleted, TextRowKind::Inserted]
        );
        // Overview readout diff order se match karta hai (donon tracks).
        assert_eq!(
            gutter_overview_diff_kinds(&over),
            gutter_diff_band_kinds(&rows)
        );
        assert_eq!(
            gutter_overview_diff_kinds(&over_padded),
            gutter_diff_band_kinds(&rows)
        );
        assert_eq!(
            gutter_overview_diff_kinds(&over),
            diff_only
                .segments
                .iter()
                .map(|s| s.kind)
                .collect::<Vec<_>>()
        );
        // 720 narrow symmetric + slop dono taraf barabar.
        assert_eq!(gutter_text_layout_720(), (12.0, 336.0, 24.0, 336.0, 12.0));
        let seg = GutterSegment {
            y_start: 200.0,
            y_end: 204.0,
            kind: TextRowKind::Deleted,
        };
        assert_eq!(
            gutter_slop_band(&seg, GUTTER_HIT_SLOP),
            Some((198.0, 206.0))
        );
        let Some((lo, hi)) = gutter_slop_band(&seg, GUTTER_HIT_SLOP) else {
            return;
        };
        assert!(((seg.y_start - lo) - (hi - seg.y_end)).abs() < 0.001);
        assert!(gutter_text_gutter_width(120, true) < 336.0);
    }

    #[test]
    fn gutter7_tall_marker_symmetric_around_row_center() {
        // Mirrors checks/gui-exq-gutter7/round-1/comparison/gutter-tall-1200.json:
        // changed-row marker band symmetric around its row center on the
        // tall fixture (no tall drift), slop symmetric on the real band.
        let rows = tall_1200_rows();
        let geom = gutter_geometry(1200, 400.0, &rows);
        let Some(seg) = geom.segments.first() else {
            return;
        };
        let Some(cy) = gutter_row_center_y(600, 1200, 400.0) else {
            return;
        };
        let Some((y0, y1)) = gutter_selection_marker_band(600, 1200, 400.0) else {
            return;
        };
        assert!(((cy - y0) - (y1 - cy)).abs() < 0.001);
        assert!((y1 - y0 - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        let Some((lo, hi)) = gutter_slop_band(seg, GUTTER_HIT_SLOP) else {
            return;
        };
        assert!(((seg.y_start - lo) - (hi - seg.y_end)).abs() < 0.001);
    }
}

// ----------------------------------------------------------------------------
// GUTTER9 round-1: gutter-side changed-only markers + SPACE-width column at 960.
// BC text compare center gutter ~20px with yellow '=>' markers ONLY at changed
// rows; views-side already narrows to 20px (SPACE_LG + SPACE_XS). Gutter-side
// locks the same rhythm: marker box 16px (SPACE_LG) centered in a 20px column
// with 2px symmetric inset/slop (GUTTER_MERGE_GAP), markers gated on the
// changed-row flag so Equal rows stay blank. Geometry first: all widths from
// SPACE/GUTTER scale, no new literals. Typography second: nothing (no text
// here). Colors frozen: shape only.
// WIRING (paint owner ke paas): per-row marker `gutter_side_marker_visible(..)`
// se gate karo (false -> blank); changed rows `gutter_side_changed_rows(..)`
// se; 960 column x `gutter_side_column_x_960()` se, arrow cx
// `gutter_side_arrow_cx_960()` se, marker box `gutter_side_marker_rect(..)`
// se, hit `gutter_side_hit_rect(..)` / `gutter_side_hit_contains(..)` se,
// band `gutter_side_hit_band_960()` se, glyph `gutter_side_glyph_960(..)` se.
// ----------------------------------------------------------------------------

/// Gutter-side marker column width px: SPACE_LG + SPACE_XS (20.0).
///
/// Views-side [`crate::views::TEXT_CENTER_GUTTER_W`] parity so gutter-side and
/// views-side stay symmetric at 960px wide. No new literal, only SPACE scale.
pub const GUTTER_SIDE_MARKER_COL_W: f32 = crate::theme::SPACE_LG + crate::theme::SPACE_XS;

/// Gutter-side marker box px: one LG step (16.0, arrow-box parity).
///
/// [`GUTTER_ARROW_BOX`] twin so side markers read the same size as center
/// arrows without drifting off scale.
pub const GUTTER_SIDE_MARKER_BOX: f32 = crate::theme::SPACE_LG;

/// Gutter-side marker inset px per side: merge-gap reuse (2.0).
///
/// `(COL - BOX) / 2 == 2`, symmetric so the 16px marker centers in the 20px
/// column ( BC center-bar rhythm ).
pub const GUTTER_SIDE_MARKER_INSET: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Gutter-side hit slop px per side: merge-gap reuse (2.0), symmetric.
///
/// 16px box -> 20px tap-target bina paint hile, dono taraf barabar.
pub const GUTTER_SIDE_HIT_SLOP: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Gutter-side marker column width px (single source). Pure.
pub fn gutter_side_marker_width() -> f32 {
    GUTTER_SIDE_MARKER_COL_W
}

/// Gutter-side marker box px (single source). Pure.
pub fn gutter_side_marker_box() -> f32 {
    GUTTER_SIDE_MARKER_BOX
}

/// Gutter-side marker inset px per side (single source). Pure.
pub fn gutter_side_marker_inset() -> f32 {
    GUTTER_SIDE_MARKER_INSET
}

/// Gutter-side hit slop px per side (single source). Pure.
pub fn gutter_side_hit_slop() -> f32 {
    GUTTER_SIDE_HIT_SLOP
}

/// Gutter-side marker visibility for one row: changed rows only.
///
/// Single source over [`gutter_marker_visible`] so side and center gates never
/// disagree: `Equal` rows stay blank, every diff kind gets a marker.
/// Pure; unit-tested.
pub fn gutter_side_marker_visible(kind: TextRowKind) -> bool {
    gutter_marker_visible(kind)
}

/// Gutter-side changed-row indices in row order.
///
/// Single source over [`gutter_changed_rows`] so markers land ONLY on changed
/// rows at 960px wide. Pure; unit-tested.
pub fn gutter_side_changed_rows(rows: &[TextRow]) -> Vec<usize> {
    gutter_changed_rows(rows)
}

/// 960px canvas symmetric side layout `(margin, left, center, right, margin)`
/// px: 12 + 458 + 20 + 458 + 12 = 960. Panes barabar, 20px side column beech
/// me symmetric insets samet. Single source over [`gutter_text_layout`].
/// Pure; unit-tested.
pub fn gutter_side_layout_960() -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        GUTTER_SIDE_MARKER_COL_W,
        GUTTER_CANVAS_NORMAL_960,
    )
}

/// 960px canvas side-column x px: `margin + pane` from the symmetric side
/// layout (12 + 458 = 470). Column is [`GUTTER_SIDE_MARKER_COL_W`] wide (20).
/// Pure; unit-tested.
pub fn gutter_side_column_x_960() -> f32 {
    let (margin, pane, _, _, _) = gutter_side_layout_960();
    margin + pane
}

/// 960px canvas side-marker cx px: column center (470 + 20/2 = 480 = canvas
/// middle), so the marker sits centered in the gutter. `None` only when the
/// column math goes non-finite. Pure; unit-tested.
pub fn gutter_side_arrow_cx_960() -> Option<f32> {
    let x = gutter_side_column_x_960();
    let w = GUTTER_SIDE_MARKER_COL_W;
    if !x.is_finite() || !w.is_finite() {
        return None;
    }
    let cx = x + w / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// Gutter-side marker box rect `(x0, y0, x1, y1)` px: [`GUTTER_SIDE_MARKER_BOX`]
/// centered at `(center_x, center_y)`. Non-finite input par `None` taaki paint
/// NaN kabhi layout me na daale. Pure; unit-tested.
pub fn gutter_side_marker_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    if !center_x.is_finite() || !center_y.is_finite() {
        return None;
    }
    let half = GUTTER_SIDE_MARKER_BOX / 2.0;
    Some((
        center_x - half,
        center_y - half,
        center_x + half,
        center_y + half,
    ))
}

/// Gutter-side slop-expanded hit rect `(x0, y0, x1, y1)` px: marker box har
/// taraf [`GUTTER_SIDE_HIT_SLOP`] badha hua (16px box -> 20px target), dono
/// taraf symmetric. Non-finite par `None`. Pure; unit-tested.
pub fn gutter_side_hit_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let (x0, y0, x1, y1) = gutter_side_marker_rect(center_x, center_y)?;
    let s = GUTTER_SIDE_HIT_SLOP;
    let (lx0, ly0, lx1, ly1) = (x0 - s, y0 - s, x1 + s, y1 + s);
    if lx0.is_finite() && ly0.is_finite() && lx1.is_finite() && ly1.is_finite() {
        Some((lx0, ly0, lx1, ly1))
    } else {
        None
    }
}

/// Gutter-side hit-test: point `(x, y)` slop-expanded hit rect ke andar hai
/// ya nahi (edges inclusive). Non-finite par false. Pure; unit-tested.
pub fn gutter_side_hit_contains(center_x: f32, center_y: f32, x: f32, y: f32) -> bool {
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    match gutter_side_hit_rect(center_x, center_y) {
        Some((x0, y0, x1, y1)) => x >= x0 && x <= x1 && y >= y0 && y <= y1,
        None => false,
    }
}

/// 960px canvas side glyph rect `(x0, y0, x1, y1)` px: 12px chevron
/// ([`gutter_arrow_glyph_rect`]) centered at canvas middle (480) for the
/// given row-center y. `None` for non-finite y. Pure; unit-tested.
pub fn gutter_side_glyph_960(center_y: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_side_arrow_cx_960()?;
    gutter_arrow_glyph_rect(cx, center_y)
}

/// 960px canvas side hit band `(lo, hi)` px: side column + symmetric slop on
/// both sides (470 - 2 .. 490 + 2 = 468..492), canvas center 480 ke around
/// barabar. Pure; unit-tested.
pub fn gutter_side_hit_band_960() -> (f32, f32) {
    let x = gutter_side_column_x_960();
    let w = GUTTER_SIDE_MARKER_COL_W;
    let s = GUTTER_SIDE_HIT_SLOP;
    (x - s, x + w + s)
}

#[cfg(test)]
mod exq_gutter9_tests {
    use super::*;

    fn srow(kind: TextRowKind) -> TextRow {
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

    fn mixed_960_rows() -> Vec<TextRow> {
        vec![
            srow(TextRowKind::Equal),
            srow(TextRowKind::Deleted),
            srow(TextRowKind::Equal),
            srow(TextRowKind::Changed),
            srow(TextRowKind::Equal),
            srow(TextRowKind::Inserted),
            srow(TextRowKind::Equal),
            srow(TextRowKind::Equal),
        ]
    }

    #[test]
    fn gutter9_side_markers_only_on_changed_rows() {
        // Mirrors checks/gui-exq-gutter9/round-1/comparison/gutter-markers-960.json.
        let rows = mixed_960_rows();
        assert!(!gutter_side_marker_visible(TextRowKind::Equal));
        assert!(gutter_side_marker_visible(TextRowKind::Deleted));
        assert!(gutter_side_marker_visible(TextRowKind::Inserted));
        assert!(gutter_side_marker_visible(TextRowKind::Changed));
        assert_eq!(gutter_side_changed_rows(&rows), vec![1, 3, 5]);
        assert_eq!(gutter_side_changed_rows(&rows), gutter_changed_rows(&rows));
        // Segments agree: diff-only geometry yields exactly the changed bands.
        let geom = gutter_geometry(rows.len(), 800.0, &rows);
        assert_eq!(geom.segments.len(), 3);
        // Every Equal row maps to no segment: side markers never land there.
        let mut idx = 0usize;
        while idx < rows.len() {
            if let Some(row) = rows.get(idx) {
                if row.kind == TextRowKind::Equal {
                    if let Some(cy) = gutter_row_center_y(idx, rows.len(), 800.0) {
                        assert_eq!(gutter_segment_at_y(&geom.segments, cy), None);
                    }
                }
            }
            idx = idx.saturating_add(1);
        }
        assert!(gutter_side_changed_rows(&[]).is_empty());
    }

    #[test]
    fn gutter9_side_box_from_space_tokens_hit_symmetric_at_960() {
        // Mirrors checks/gui-exq-gutter9/round-1/comparison/gutter-markers-960.json:
        // col 20 (LG+XS), box 16 (LG), inset 2 + slop 2 (merge-gap), symmetric.
        assert_eq!(
            GUTTER_SIDE_MARKER_COL_W,
            crate::theme::SPACE_LG + crate::theme::SPACE_XS
        );
        assert_eq!(gutter_side_marker_width(), 20.0);
        assert_eq!(GUTTER_SIDE_MARKER_BOX, crate::theme::SPACE_LG);
        assert_eq!(gutter_side_marker_box(), 16.0);
        assert_eq!(GUTTER_SIDE_MARKER_INSET, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(gutter_side_marker_inset(), 2.0);
        assert_eq!(GUTTER_SIDE_HIT_SLOP, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(gutter_side_hit_slop(), 2.0);
        // Inset centers the box: (col - box) / 2 == inset, both sides equal.
        assert!(
            (center_row_marker_inset(gutter_side_marker_width(), gutter_side_marker_box())
                - gutter_side_marker_inset())
            .abs()
                < 0.001
        );
        assert_eq!(
            center_row_marker_inset(gutter_side_marker_width(), gutter_side_marker_box()),
            2.0
        );
        // Symmetric layout at 960: 12 + 458 + 20 + 458 + 12 = 960.
        assert_eq!(gutter_side_layout_960(), (12.0, 458.0, 20.0, 458.0, 12.0));
        let (m0, left, center, right, m1) = gutter_side_layout_960();
        assert_eq!(m0, m1);
        assert_eq!(left, right);
        assert!((m0 + left + center + right + m1 - 960.0).abs() < 0.001);
        // Column x 470, cx 480 canvas middle; glyph + hit symmetric about 480.
        assert_eq!(gutter_side_column_x_960(), 470.0);
        assert_eq!(gutter_side_arrow_cx_960(), Some(480.0));
        assert_eq!(
            gutter_side_marker_rect(480.0, 200.0),
            Some((472.0, 192.0, 488.0, 208.0))
        );
        assert_eq!(
            gutter_side_hit_rect(480.0, 200.0),
            Some((470.0, 190.0, 490.0, 210.0))
        );
        assert_eq!(
            gutter_side_glyph_960(200.0),
            Some((474.0, 194.0, 486.0, 206.0))
        );
        assert_eq!(gutter_side_glyph_960(f32::NAN), None);
        assert_eq!(gutter_side_hit_band_960(), (468.0, 492.0));
        let (lo, hi) = gutter_side_hit_band_960();
        assert!(((480.0 - lo) - (hi - 480.0)).abs() < 0.001);
        // Hit box misses, slop hits: symmetric both sides.
        assert!(gutter_side_hit_contains(480.0, 200.0, 471.0, 200.0));
        assert!(gutter_side_hit_contains(480.0, 200.0, 480.0, 200.0));
        assert!(!gutter_side_hit_contains(480.0, 200.0, 467.0, 200.0));
        assert!(!gutter_side_hit_contains(480.0, 200.0, 480.0, 211.0));
        assert_eq!(gutter_side_marker_rect(f32::NAN, 200.0), None);
        assert_eq!(gutter_side_hit_rect(480.0, f32::INFINITY), None);
    }
}

// ----------------------------------------------------------------------------
// EXQUISITE TEXT38: Center gutter geometry (center 20px lane alignment,
// copy arrows centered, row height TypoRole Mono rhythm, 1200px no h-scroll +
// narrow700 edge).
//
// In twin-pane comparative editing, the center gutter forms a focused vertical
// boundary dividing left and right panes. Visual parity with Beyond Compare
// requires this center lane to maintain an exact 20px width (`SPACE_LG + SPACE_XS`),
// centered at the canvas midpoint (`cx = avail_w / 2.0`). Directional copy
// arrows sit centered within the 20px lane with 2px side margins (`GUTTER_MERGE_GAP`),
// matching editor row line pitch via `TypoRole::Mono` (15.6px line height, 1.30
// line-height multiplier, 20.0px compact row height). Symmetrical panes on 1200px
// canvas (578px each) and 700px narrow canvas (328px each) fit with zero
// horizontal scroll.
// ----------------------------------------------------------------------------

/// Center gutter lane width px: one LG step + one XS step (16.0 + 4.0 = 20.0).
/// Single source matching `GUTTER_SIDE_MARKER_COL_W`.
pub const TEXT38_CENTER_LANE_W: f32 = crate::theme::SPACE_LG + crate::theme::SPACE_XS;

/// Center gutter copy arrow box size px: one LG step (16.0).
pub const TEXT38_COPY_ARROW_BOX: f32 = crate::theme::SPACE_LG;

/// Center gutter copy arrow side padding px: `(20.0 - 16.0) / 2.0 = 2.0` (`GUTTER_MERGE_GAP`).
pub const TEXT38_COPY_ARROW_PAD_X: f32 = crate::theme::GUTTER_MERGE_GAP;

/// Canvas width for wide comparison view (px): 1200.0 via [`GUTTER_CANVAS_WIDE_1200`].
pub const TEXT38_CANVAS_WIDE_W: f32 = GUTTER_CANVAS_WIDE_1200;

/// Canvas width for narrow comparison view edge (<720 breakpoint, px): 700.0 via [`GUTTER_CANVAS_NARROW_700`].
pub const TEXT38_CANVAS_NARROW_W: f32 = GUTTER_CANVAS_NARROW_700;

/// Center gutter lane width px (single source). Pure; unit-tested.
pub fn text38_center_lane_w() -> f32 {
    TEXT38_CENTER_LANE_W
}

/// Computes symmetric dual-pane layout with 20px center lane:
/// `(margin, left_pane, center_lane, right_pane, margin)`.
/// Delegates to [`gutter_text_layout`]. Pure; unit-tested.
pub fn text38_center_lane_layout(avail_w: f32) -> (f32, f32, f32, f32, f32) {
    gutter_text_layout(
        crate::theme::GEO_OUTER_MARGIN,
        TEXT38_CENTER_LANE_W,
        avail_w,
    )
}

/// Column start x for 20px center lane: `margin + left_pane`.
/// `None` for non-finite or invalid widths. Pure; unit-tested.
pub fn text38_center_lane_col_x(avail_w: f32) -> Option<f32> {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return None;
    }
    let (margin, pane, center, _, _) = text38_center_lane_layout(avail_w);
    if center <= 0.0 || pane <= 0.0 {
        return None;
    }
    let col_x = margin + pane;
    if col_x.is_finite() { Some(col_x) } else { None }
}

/// Center midpoint cx for 20px center lane: `col_x + 20.0 / 2.0`.
/// Always aligns at `avail_w / 2.0`. Pure; unit-tested.
pub fn text38_center_lane_cx(avail_w: f32) -> Option<f32> {
    let col_x = text38_center_lane_col_x(avail_w)?;
    let cx = col_x + TEXT38_CENTER_LANE_W / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// Verifies whether the 20px center lane is exactly aligned to the canvas midpoint (`avail_w / 2.0`).
/// Pure; unit-tested.
pub fn text38_center_lane_is_aligned(avail_w: f32) -> bool {
    let Some(cx) = text38_center_lane_cx(avail_w) else {
        return false;
    };
    (cx - avail_w / 2.0).abs() < 0.001
}

/// Copy arrow box size px (16.0). Pure; unit-tested.
pub fn text38_copy_arrow_box() -> f32 {
    TEXT38_COPY_ARROW_BOX
}

/// Copy arrow horizontal padding within 20px center lane (2.0). Pure; unit-tested.
pub fn text38_copy_arrow_pad_x() -> f32 {
    TEXT38_COPY_ARROW_PAD_X
}

/// Copy arrow bounding rect `(x0, y0, x1, y1)` centered at `(center_x, center_y)`.
/// Pure; unit-tested.
pub fn text38_copy_arrow_rect(center_x: f32, center_y: f32) -> Option<(f32, f32, f32, f32)> {
    if !center_x.is_finite() || !center_y.is_finite() {
        return None;
    }
    let half = TEXT38_COPY_ARROW_BOX / 2.0;
    Some((
        center_x - half,
        center_y - half,
        center_x + half,
        center_y + half,
    ))
}

/// Places a 16x16 copy arrow centered vertically within `(row_top, row_top + row_h)`
/// and horizontally within `(col_x, col_x + col_w)`.
/// Pure; unit-tested.
pub fn text38_copy_arrow_in_row(
    row_top: f32,
    row_h: f32,
    col_x: f32,
    col_w: f32,
) -> Option<(f32, f32, f32, f32)> {
    if !row_top.is_finite() || !row_h.is_finite() || !col_x.is_finite() || !col_w.is_finite() {
        return None;
    }
    if row_h <= 0.0 || col_w <= 0.0 {
        return None;
    }
    let cx = col_x + col_w / 2.0;
    let cy = row_top + row_h / 2.0;
    text38_copy_arrow_rect(cx, cy)
}

/// Tests whether point `(x, y)` falls within the copy arrow hitbox centered at `(center_x, center_y)`.
/// Includes symmetric 2.0px hit-slop so 16px box maps to 20px hit target. Pure; unit-tested.
pub fn text38_copy_arrow_hit_contains(center_x: f32, center_y: f32, x: f32, y: f32) -> bool {
    if !center_x.is_finite() || !center_y.is_finite() || !x.is_finite() || !y.is_finite() {
        return false;
    }
    let half = (TEXT38_COPY_ARROW_BOX + 2.0 * crate::theme::GUTTER_MERGE_GAP) / 2.0;
    let x0 = center_x - half;
    let x1 = center_x + half;
    let y0 = center_y - half;
    let y1 = center_y + half;
    x >= x0 && x <= x1 && y >= y0 && y <= y1
}

/// Monospace typographic role for text rows. Reuses [`crate::theme::TypoRole::Mono`].
pub const fn text38_mono_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Line-height multiplier for monospace text rows (1.30).
pub const fn text38_mono_line_height() -> f32 {
    crate::theme::TYPO_LINE_HEIGHT_MONO
}

/// Measured monospace line-height px (12.0 * 1.30 = 15.6). Pure; unit-tested.
pub fn text38_mono_line_height_px() -> f32 {
    crate::theme::TypoRole::Mono.line_height_px()
}

/// Compact text row height px: `SPACE_LG + SPACE_XS` = 20.0. Pure; unit-tested.
pub fn text38_row_height_compact() -> f32 {
    crate::theme::SPACE_LG + crate::theme::SPACE_XS
}

/// Padded text row height px: `15.6 + 2 * GUTTER_TEXT_ROW_PAD` = 19.6. Pure; unit-tested.
pub fn text38_row_height_padded() -> Option<f32> {
    gutter_text_row_height(text38_mono_line_height_px())
}

/// Maps row index to vertical center y px under given row height rhythm: `(row + 0.5) * row_h`.
/// Pure; unit-tested.
pub fn text38_row_center_y(row: usize, total_rows: usize, row_h: f32) -> Option<f32> {
    if total_rows == 0 || row >= total_rows || !row_h.is_finite() || row_h <= 0.0 {
        return None;
    }
    let cy = (row as f32 + 0.5) * row_h;
    if cy.is_finite() { Some(cy) } else { None }
}

/// Maps row index to row top y px under given row height rhythm: `row * row_h`.
/// Pure; unit-tested.
pub fn text38_row_top_y(row: usize, total_rows: usize, row_h: f32) -> Option<f32> {
    if total_rows == 0 || row >= total_rows || !row_h.is_finite() || row_h <= 0.0 {
        return None;
    }
    let y = row as f32 * row_h;
    if y.is_finite() { Some(y) } else { None }
}

/// Total vertical height for `total_rows` at `row_h` rhythm. Pure; unit-tested.
pub fn text38_total_height(total_rows: usize, row_h: f32) -> f32 {
    if total_rows == 0 || !row_h.is_finite() || row_h <= 0.0 {
        0.0
    } else {
        total_rows as f32 * row_h
    }
}

/// Total canvas width for 1200px wide layout (px). Pure.
pub fn text38_canvas_w() -> f32 {
    TEXT38_CANVAS_WIDE_W
}

/// Total canvas width for 700px narrow layout edge (px). Pure.
pub fn text38_narrow_w() -> f32 {
    TEXT38_CANVAS_NARROW_W
}

/// Available width at 1200px after outer margins: `1200.0 - 2 * 12.0 = 1176.0`. Pure.
pub fn text38_avail_w() -> f32 {
    TEXT38_CANVAS_WIDE_W - 2.0 * crate::theme::GEO_OUTER_MARGIN
}

/// Available width at 700px after outer margins: `700.0 - 2 * 12.0 = 676.0`. Pure.
pub fn text38_narrow_avail_w() -> f32 {
    TEXT38_CANVAS_NARROW_W - 2.0 * crate::theme::GEO_OUTER_MARGIN
}

/// Determines whether content width exceeds available width, requiring horizontal scrolling.
/// Non-finite inputs return false. Pure; unit-tested.
pub fn text38_needs_hscroll(avail_w: f32, content_w: f32) -> bool {
    if !avail_w.is_finite() || !content_w.is_finite() {
        return false;
    }
    content_w > avail_w
}

/// Returns true if content fits within 1200px wide available width without horizontal scroll.
pub fn text38_wide1200_fits(content_w: f32) -> bool {
    !text38_needs_hscroll(text38_avail_w(), content_w)
}

/// Returns true if content fits within 700px narrow available width without horizontal scroll.
pub fn text38_narrow700_fits(content_w: f32) -> bool {
    !text38_needs_hscroll(text38_narrow_avail_w(), content_w)
}

#[cfg(test)]
mod exq_text38_tests {
    use super::*;

    #[test]
    fn text38_center_20px_lane_alignment() {
        // Center lane width is 20.0px (SPACE_LG + SPACE_XS)
        assert_eq!(
            TEXT38_CENTER_LANE_W,
            crate::theme::SPACE_LG + crate::theme::SPACE_XS
        );
        assert_eq!(text38_center_lane_w(), 20.0);

        // Wide 1200px layout: 12 + 578 + 20 + 578 + 12 = 1200
        let (m0, l, c, r, m1) = text38_center_lane_layout(1200.0);
        assert_eq!(m0, 12.0);
        assert_eq!(m1, 12.0);
        assert_eq!(l, 578.0);
        assert_eq!(r, 578.0);
        assert_eq!(c, 20.0);
        assert!((m0 + l + c + r + m1 - 1200.0).abs() < 0.001);
        assert_eq!(text38_center_lane_col_x(1200.0), Some(590.0));
        assert_eq!(text38_center_lane_cx(1200.0), Some(600.0));
        assert!(text38_center_lane_is_aligned(1200.0));

        // Canonical 960px layout: 12 + 458 + 20 + 458 + 12 = 960
        let (m0, l, c, r, m1) = text38_center_lane_layout(960.0);
        assert_eq!(l, 458.0);
        assert_eq!(r, 458.0);
        assert_eq!(c, 20.0);
        assert!((m0 + l + c + r + m1 - 960.0).abs() < 0.001);
        assert_eq!(text38_center_lane_col_x(960.0), Some(470.0));
        assert_eq!(text38_center_lane_cx(960.0), Some(480.0));
        assert!(text38_center_lane_is_aligned(960.0));

        // Narrow 700px layout: 12 + 328 + 20 + 328 + 12 = 700
        let (m0, l, c, r, m1) = text38_center_lane_layout(700.0);
        assert_eq!(l, 328.0);
        assert_eq!(r, 328.0);
        assert_eq!(c, 20.0);
        assert!((m0 + l + c + r + m1 - 700.0).abs() < 0.001);
        assert_eq!(text38_center_lane_col_x(700.0), Some(340.0));
        assert_eq!(text38_center_lane_cx(700.0), Some(350.0));
        assert!(text38_center_lane_is_aligned(700.0));

        // Invalid or non-finite inputs
        assert_eq!(text38_center_lane_col_x(f32::NAN), None);
        assert_eq!(text38_center_lane_cx(f32::NAN), None);
        assert!(!text38_center_lane_is_aligned(f32::NAN));
        assert_eq!(text38_center_lane_col_x(-100.0), None);
        assert_eq!(text38_center_lane_cx(0.0), None);
    }

    #[test]
    fn text38_copy_arrows_centered_in_lane() {
        assert_eq!(TEXT38_COPY_ARROW_BOX, crate::theme::SPACE_LG);
        assert_eq!(text38_copy_arrow_box(), 16.0);
        assert_eq!(TEXT38_COPY_ARROW_PAD_X, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(text38_copy_arrow_pad_x(), 2.0);

        // Arrow rect centered at (cx, cy)
        assert_eq!(
            text38_copy_arrow_rect(600.0, 100.0),
            Some((592.0, 92.0, 608.0, 108.0))
        );
        assert_eq!(text38_copy_arrow_rect(f32::NAN, 100.0), None);

        // Arrow placed in row: col 590..610, row 40..60 (20px high)
        let in_row = text38_copy_arrow_in_row(40.0, 20.0, 590.0, 20.0);
        assert_eq!(in_row, Some((592.0, 42.0, 608.0, 58.0)));
        assert_eq!(text38_copy_arrow_in_row(0.0, 0.0, 590.0, 20.0), None);

        // Hit testing with 2px slop: box is 592..608, 2px slop gives 590..610
        assert!(text38_copy_arrow_hit_contains(600.0, 100.0, 600.0, 100.0));
        assert!(text38_copy_arrow_hit_contains(600.0, 100.0, 590.0, 90.0));
        assert!(text38_copy_arrow_hit_contains(600.0, 100.0, 610.0, 110.0));
        assert!(!text38_copy_arrow_hit_contains(600.0, 100.0, 589.0, 100.0));
        assert!(!text38_copy_arrow_hit_contains(600.0, 100.0, 611.0, 100.0));
        assert!(!text38_copy_arrow_hit_contains(
            f32::NAN,
            100.0,
            600.0,
            100.0
        ));
    }

    #[test]
    fn text38_row_height_typo_mono_rhythm() {
        use crate::theme::TypoRole;
        assert_eq!(text38_mono_role(), TypoRole::Mono);
        assert_eq!(text38_mono_line_height(), 1.30);
        assert!((text38_mono_line_height_px() - 15.6).abs() < 0.01);

        assert_eq!(text38_row_height_compact(), 20.0);
        assert!((text38_row_height_padded().unwrap_or(0.0) - 19.6).abs() < 0.01);

        // Mapping row indices
        assert_eq!(text38_row_top_y(0, 10, 20.0), Some(0.0));
        assert_eq!(text38_row_top_y(1, 10, 20.0), Some(20.0));
        assert_eq!(text38_row_top_y(9, 10, 20.0), Some(180.0));
        assert_eq!(text38_row_top_y(10, 10, 20.0), None);

        assert_eq!(text38_row_center_y(0, 10, 20.0), Some(10.0));
        assert_eq!(text38_row_center_y(1, 10, 20.0), Some(30.0));
        assert_eq!(text38_row_center_y(9, 10, 20.0), Some(190.0));
        assert_eq!(text38_row_center_y(10, 10, 20.0), None);

        assert_eq!(text38_total_height(10, 20.0), 200.0);
        assert_eq!(text38_total_height(0, 20.0), 0.0);
        assert_eq!(text38_total_height(10, f32::NAN), 0.0);
    }

    #[test]
    fn text38_wide1200_no_hscroll_and_narrow700_edge() {
        assert_eq!(text38_canvas_w(), 1200.0);
        assert_eq!(text38_narrow_w(), 700.0);
        assert_eq!(text38_avail_w(), 1176.0);
        assert_eq!(text38_narrow_avail_w(), 676.0);

        // Center lane fits easily
        assert!(text38_wide1200_fits(text38_center_lane_w()));
        assert!(text38_narrow700_fits(text38_center_lane_w()));

        // Content fitting within available width
        assert!(!text38_needs_hscroll(text38_avail_w(), 1176.0));
        assert!(text38_needs_hscroll(text38_avail_w(), 1177.0));
        assert!(!text38_needs_hscroll(text38_narrow_avail_w(), 676.0));
        assert!(text38_needs_hscroll(text38_narrow_avail_w(), 677.0));

        // Non-finite
        assert!(!text38_needs_hscroll(f32::NAN, 100.0));
        assert!(!text38_needs_hscroll(100.0, f32::NAN));
    }

    #[test]
    fn text38_fixtures_parity() {
        let wide =
            include_str!("../../../checks/gui-exq-text38/round-1/comparison/text38-wide-1200.json");
        assert!(wide.contains("\"name\": \"text38-wide-1200\""));
        assert!(wide.contains("\"canvas_w\": 1200.0"));
        assert!(wide.contains("\"avail_w\": 1176.0"));
        assert!(wide.contains("\"center_lane_w\": 20.0"));
        assert!(wide.contains("\"center_col_x\": 590.0"));
        assert!(wide.contains("\"center_cx\": 600.0"));
        assert!(wide.contains("\"left_pane_w\": 578.0"));
        assert!(wide.contains("\"right_pane_w\": 578.0"));
        assert!(wide.contains("\"copy_arrow_box\": 16.0"));
        assert!(wide.contains("\"copy_arrow_pad_x\": 2.0"));
        assert!(wide.contains("\"mono_line_height\": 1.3"));
        assert!(wide.contains("\"mono_line_height_px\": 15.6"));
        assert!(wide.contains("\"row_height_compact\": 20.0"));
        assert!(wide.contains("\"row_height_padded\": 19.6"));
        assert!(wide.contains("\"needs_hscroll\": false"));

        let narrow = include_str!(
            "../../../checks/gui-exq-text38/round-1/comparison/text38-narrow-700.json"
        );
        assert!(narrow.contains("\"name\": \"text38-narrow-700\""));
        assert!(narrow.contains("\"canvas_w\": 700.0"));
        assert!(narrow.contains("\"avail_w\": 676.0"));
        assert!(narrow.contains("\"center_lane_w\": 20.0"));
        assert!(narrow.contains("\"center_col_x\": 340.0"));
        assert!(narrow.contains("\"center_cx\": 350.0"));
        assert!(narrow.contains("\"left_pane_w\": 328.0"));
        assert!(narrow.contains("\"right_pane_w\": 328.0"));
        assert!(narrow.contains("\"copy_arrow_box\": 16.0"));
        assert!(narrow.contains("\"copy_arrow_pad_x\": 2.0"));
        assert!(narrow.contains("\"mono_line_height\": 1.3"));
        assert!(narrow.contains("\"mono_line_height_px\": 15.6"));
        assert!(narrow.contains("\"row_height_compact\": 20.0"));
        assert!(narrow.contains("\"row_height_padded\": 19.6"));
        assert!(narrow.contains("\"needs_hscroll\": false"));
    }
}

// ----------------------------------------------------------------------------
// EXQUISITE GEO55: Gutter geometry parity across canvas budgets
// (narrow <720px / 700px, normal 960px, wide 1200px).
//
// In comparative side-by-side editing, text panes and gutter columns (line
// numbers, omit markers, and overview rulers) must share canvas width without
// clipping or double-padding.
// Clean-room observation: Beyond Compare 5 allocates outer margins, twin text
// panes, and centered affordance columns with deterministic, single-sourced
// geometry. In narrow viewports (<720px, specifically 700px), equal pane
// widths are maintained while preserving necessary gutter affordances.
// In normal (960px) and wide (1200px) views, extra horizontal space expands
// panes symmetrically while gutters remain anchored to the design scale.
//
// Geometry rules:
// 1. Single-source width: all width calculations delegate to existing constants
//    (`GEO_OUTER_MARGIN`, `SPACE_XS`, `SPACE_SM`, `SPACE_MD`, `SPACE_LG`,
//    `SPACE_XL`, `TEXT38_CENTER_LANE_W`, `GUTTER_LINENUM_PAD`).
// 2. No double-padding: inter-element padding is strictly unified so adjacent
//    gutter boundaries and pane text content never compound padding.
// 3. Strict alignment: line number cells are right-aligned tabular strings with
//    two-space trailing pad; centered marker glyphs and chevrons align exactly
//    to gutter column center and canvas midpoint (`avail_w / 2.0`).
// 4. Zero new colors, zero new sizes, zero typography edits.
// ----------------------------------------------------------------------------

/// Canvas width for narrow comparison view edge (<720 breakpoint, px): 700.0.
pub const GUTTER_CANVAS_NARROW_700: f32 = 700.0;

/// Canvas width for canonical comparison view (px): 960.0.
pub const GUTTER_CANVAS_NORMAL_960: f32 = 960.0;

/// Canvas width for wide comparison view (px): 1200.0.
pub const GUTTER_CANVAS_WIDE_1200: f32 = 1200.0;

/// Omit / status marker gutter column width px: one LG step (16.0).
/// Single-source reuse of [`crate::theme::SPACE_LG`].
pub const GUTTER_OMIT_MARKER_COL_W: f32 = crate::theme::SPACE_LG;

/// Omit / status marker box size px: one MD step (12.0).
/// Single-source reuse of [`crate::theme::SPACE_MD`].
pub const GUTTER_OMIT_MARKER_BOX: f32 = crate::theme::SPACE_MD;

/// Gutter outer margin px (single source delegating to [`crate::theme::GEO_OUTER_MARGIN`]). Pure.
pub fn gutter_geo_outer_margin() -> f32 {
    crate::theme::GEO_OUTER_MARGIN
}

/// Gutter space XS step px (single source delegating to [`crate::theme::SPACE_XS`]). Pure.
pub fn gutter_geo_space_xs() -> f32 {
    crate::theme::SPACE_XS
}

/// Gutter space SM step px (single source delegating to [`crate::theme::SPACE_SM`]). Pure.
pub fn gutter_geo_space_sm() -> f32 {
    crate::theme::SPACE_SM
}

/// Gutter space MD step px (single source delegating to [`crate::theme::SPACE_MD`]). Pure.
pub fn gutter_geo_space_md() -> f32 {
    crate::theme::SPACE_MD
}

/// Gutter space LG step px (single source delegating to [`crate::theme::SPACE_LG`]). Pure.
pub fn gutter_geo_space_lg() -> f32 {
    crate::theme::SPACE_LG
}

/// Gutter space XL step px (single source delegating to [`crate::theme::SPACE_XL`]). Pure.
pub fn gutter_geo_space_xl() -> f32 {
    crate::theme::SPACE_XL
}

/// Gutter merge gap px (single source delegating to [`crate::theme::GUTTER_MERGE_GAP`]). Pure.
pub fn gutter_geo_merge_gap() -> f32 {
    crate::theme::GUTTER_MERGE_GAP
}

/// Gutter ruler wide track width (px): one LG step ([`crate::theme::SPACE_LG`]). Pure.
pub fn gutter_geo_ruler_wide() -> f32 {
    crate::theme::SPACE_LG
}

/// Gutter ruler narrow track width (px): one SM step ([`crate::theme::SPACE_SM`]). Pure.
pub fn gutter_geo_ruler_narrow() -> f32 {
    crate::theme::SPACE_SM
}

/// Gutter overview segment min height (px): one XS step ([`crate::theme::SPACE_XS`]). Pure.
pub fn gutter_geo_segment_min_h() -> f32 {
    crate::theme::SPACE_XS
}

/// Gutter viewport min height (px): one XS step ([`crate::theme::SPACE_XS`]). Pure.
pub fn gutter_geo_viewport_min_h() -> f32 {
    crate::theme::SPACE_XS
}

/// Gutter center column width (px): one XL step ([`crate::theme::SPACE_XL`]). Pure.
pub fn gutter_geo_center_w() -> f32 {
    crate::theme::SPACE_XL
}

/// Gutter center arrow box size (px): one LG step ([`crate::theme::SPACE_LG`]). Pure.
pub fn gutter_geo_arrow_box() -> f32 {
    crate::theme::SPACE_LG
}

/// Gutter center lane width (px): 20px via `SPACE_LG + SPACE_XS`. Pure.
pub fn gutter_geo_center_lane_w() -> f32 {
    crate::theme::SPACE_LG + crate::theme::SPACE_XS
}

/// Gutter side marker column width (px): 20px via `SPACE_LG + SPACE_XS`. Pure.
pub fn gutter_geo_side_marker_col_w() -> f32 {
    crate::theme::SPACE_LG + crate::theme::SPACE_XS
}

/// Gutter side marker box size (px): one LG step ([`crate::theme::SPACE_LG`]). Pure.
pub fn gutter_geo_side_marker_box() -> f32 {
    crate::theme::SPACE_LG
}

/// Gutter line number pad per extra digit (px): one XS step ([`crate::theme::SPACE_XS`]). Pure.
pub fn gutter_geo_linenum_pad() -> f32 {
    crate::theme::SPACE_XS
}

/// Gutter minimap track pad top/bottom (px): one XS step ([`crate::theme::SPACE_XS`]). Pure.
pub fn gutter_geo_minimap_track_pad() -> f32 {
    crate::theme::SPACE_XS
}

/// Gutter center side gap (px): one XS step ([`crate::theme::SPACE_XS`]). Pure.
pub fn gutter_geo_center_side_gap() -> f32 {
    crate::theme::SPACE_XS
}

/// Available canvas width between left and right outer margins.
/// Returns None if canvas_w is non-finite or smaller than `2 * GEO_OUTER_MARGIN`. Pure; unit-tested.
pub fn gutter_canvas_avail_w(canvas_w: f32) -> Option<f32> {
    if !canvas_w.is_finite() || canvas_w <= 2.0 * crate::theme::GEO_OUTER_MARGIN {
        return None;
    }
    Some(canvas_w - 2.0 * crate::theme::GEO_OUTER_MARGIN)
}

/// Geometric budget distribution across margins, gutters, and twin panes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GutterCanvasBudget {
    /// Total canvas width in logical pixels.
    pub canvas_w: f32,
    /// Outer margin on left and right edges (GEO_OUTER_MARGIN = 12.0).
    pub outer_margin: f32,
    /// Available width between margins (`canvas_w - 2 * outer_margin`).
    pub avail_w: f32,
    /// Center divider / lane width (e.g. 20.0 or 24.0).
    pub center_lane_w: f32,
    /// Left pane line-number column width.
    pub left_linenum_w: f32,
    /// Right pane line-number column width.
    pub right_linenum_w: f32,
    /// Left and right individual text pane content width.
    pub pane_w: f32,
    /// Total width allocated across all columns and margins.
    pub total_allocated_w: f32,
    /// True if content fits with pane_w > 0 without clipping.
    pub is_valid: bool,
}

/// Pure budget calculation distributing canvas width across margins, gutters, and twin panes.
///
/// Ensures exact pixel budget:
/// `2 * outer_margin + 2 * pane_w + center_lane_w + left_linenum_w + right_linenum_w == canvas_w`.
/// If canvas is too small to fit gutters, `is_valid` is false and `pane_w` is 0.0.
/// Pure; unit-tested.
pub fn gutter_compute_canvas_budget(
    canvas_w: f32,
    max_line_no: usize,
    show_line_numbers: bool,
    center_lane_w: f32,
) -> Option<GutterCanvasBudget> {
    if !canvas_w.is_finite() || canvas_w <= 0.0 || !center_lane_w.is_finite() || center_lane_w < 0.0
    {
        return None;
    }
    let outer_margin = crate::theme::GEO_OUTER_MARGIN;
    let avail_w = canvas_w - 2.0 * outer_margin;
    if avail_w <= 0.0 {
        return None;
    }
    let linenum_w = if show_line_numbers {
        gutter_linenum_column_width(max_line_no, true)
    } else {
        0.0
    };
    let total_gutters = center_lane_w + 2.0 * linenum_w;
    let rem = avail_w - total_gutters;
    let (pane_w, is_valid) = if rem > 0.0 {
        (rem / 2.0, true)
    } else {
        (0.0, false)
    };
    let total_allocated_w = 2.0 * outer_margin + 2.0 * pane_w + total_gutters;
    Some(GutterCanvasBudget {
        canvas_w,
        outer_margin,
        avail_w,
        center_lane_w,
        left_linenum_w: linenum_w,
        right_linenum_w: linenum_w,
        pane_w,
        total_allocated_w,
        is_valid,
    })
}

/// Gutter budget for narrow 700px canvas edge (<720 breakpoint). Pure; unit-tested.
pub fn gutter_budget_narrow_700(
    max_line_no: usize,
    show_line_numbers: bool,
) -> Option<GutterCanvasBudget> {
    gutter_compute_canvas_budget(
        GUTTER_CANVAS_NARROW_700,
        max_line_no,
        show_line_numbers,
        TEXT38_CENTER_LANE_W,
    )
}

/// Gutter budget for canonical 960px normal canvas. Pure; unit-tested.
pub fn gutter_budget_normal_960(
    max_line_no: usize,
    show_line_numbers: bool,
) -> Option<GutterCanvasBudget> {
    gutter_compute_canvas_budget(
        GUTTER_CANVAS_NORMAL_960,
        max_line_no,
        show_line_numbers,
        TEXT38_CENTER_LANE_W,
    )
}

/// Gutter budget for 1200px wide canvas. Pure; unit-tested.
pub fn gutter_budget_wide_1200(
    max_line_no: usize,
    show_line_numbers: bool,
) -> Option<GutterCanvasBudget> {
    gutter_compute_canvas_budget(
        GUTTER_CANVAS_WIDE_1200,
        max_line_no,
        show_line_numbers,
        TEXT38_CENTER_LANE_W,
    )
}

/// Returns horizontal center `col_x + col_w / 2.0` of a gutter column.
/// None for non-finite or non-positive values. Pure; unit-tested.
pub fn gutter_mark_centered_cx(col_x: f32, col_w: f32) -> Option<f32> {
    if !col_x.is_finite() || !col_w.is_finite() || col_w <= 0.0 {
        return None;
    }
    let cx = col_x + col_w / 2.0;
    if cx.is_finite() { Some(cx) } else { None }
}

/// Returns vertical center `row_y + row_h / 2.0` of a row.
/// None for non-finite or non-positive values. Pure; unit-tested.
pub fn gutter_mark_centered_cy(row_y: f32, row_h: f32) -> Option<f32> {
    if !row_y.is_finite() || !row_h.is_finite() || row_h <= 0.0 {
        return None;
    }
    let cy = row_y + row_h / 2.0;
    if cy.is_finite() { Some(cy) } else { None }
}

/// Bounding rect for a mark centered at `(cx, cy)` with square dimensions `box_size`.
/// None for non-finite or non-positive box_size. Pure; unit-tested.
pub fn gutter_center_mark_rect(cx: f32, cy: f32, box_size: f32) -> Option<(f32, f32, f32, f32)> {
    if !cx.is_finite() || !cy.is_finite() || !box_size.is_finite() || box_size <= 0.0 {
        return None;
    }
    let half = box_size / 2.0;
    Some((cx - half, cy - half, cx + half, cy + half))
}

/// Returns true if `mark_cx` aligns with the horizontal midpoint of `canvas_w` (`canvas_w / 2.0`).
/// Pure; unit-tested.
pub fn gutter_mark_is_canvas_midpoint_aligned(canvas_w: f32, mark_cx: f32) -> bool {
    if !canvas_w.is_finite() || canvas_w <= 0.0 || !mark_cx.is_finite() {
        return false;
    }
    (mark_cx - canvas_w / 2.0).abs() < 0.001
}

/// Right-aligned horizontal text x coordinate within a column `col_x..col_x + col_w`.
/// Decrements `pad_right` from right edge. Returns None if invalid or out of bounds. Pure; unit-tested.
pub fn gutter_linenum_right_aligned_x(col_x: f32, col_w: f32, pad_right: f32) -> Option<f32> {
    if !col_x.is_finite() || !col_w.is_finite() || !pad_right.is_finite() {
        return None;
    }
    if col_w <= pad_right || pad_right < 0.0 {
        return None;
    }
    let x = col_x + col_w - pad_right;
    if x.is_finite() { Some(x) } else { None }
}

/// Omit / status marker gutter column width px (single source). Pure.
pub fn gutter_omit_marker_col_w() -> f32 {
    GUTTER_OMIT_MARKER_COL_W
}

/// Omit / status marker box size px (single source). Pure.
pub fn gutter_omit_marker_box() -> f32 {
    GUTTER_OMIT_MARKER_BOX
}

/// Horizontal padding for omit marker within its 16px column: `(16 - 12) / 2 = 2.0` (`GUTTER_MERGE_GAP`). Pure.
pub fn gutter_omit_marker_pad() -> f32 {
    (GUTTER_OMIT_MARKER_COL_W - GUTTER_OMIT_MARKER_BOX) / 2.0
}

/// Omit marker bounding rect `(x0, y0, x1, y1)` centered within `(col_x, col_x + 16.0)` and `(row_y, row_y + row_h)`.
/// Pure; unit-tested.
pub fn gutter_omit_marker_rect(col_x: f32, row_y: f32, row_h: f32) -> Option<(f32, f32, f32, f32)> {
    let cx = gutter_mark_centered_cx(col_x, GUTTER_OMIT_MARKER_COL_W)?;
    let cy = gutter_mark_centered_cy(row_y, row_h)?;
    gutter_center_mark_rect(cx, cy, GUTTER_OMIT_MARKER_BOX)
}

/// Hit-tests whether point `(x, y)` falls within the omit marker hitbox with 2.0px slop.
/// Pure; unit-tested.
pub fn gutter_omit_marker_hit_contains(col_x: f32, row_y: f32, row_h: f32, x: f32, y: f32) -> bool {
    let Some(rect) = gutter_omit_marker_rect(col_x, row_y, row_h) else {
        return false;
    };
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    let slop = crate::theme::GUTTER_MERGE_GAP;
    let (x0, y0, x1, y1) = rect;
    x >= x0 - slop && x <= x1 + slop && y >= y0 - slop && y <= y1 + slop
}

/// Verifies that layout does not double-pad at boundaries.
/// Outer margin must equal `GEO_OUTER_MARGIN` (12.0) and padding tokens must belong to standard space scale.
/// Pure; unit-tested.
pub fn gutter_verify_no_double_padding(outer_margin: f32, pad_step: f32) -> bool {
    if (outer_margin - crate::theme::GEO_OUTER_MARGIN).abs() >= 0.001 {
        return false;
    }
    (pad_step - crate::theme::SPACE_XS).abs() < 0.001
        || (pad_step - crate::theme::SPACE_SM).abs() < 0.001
        || (pad_step - crate::theme::SPACE_MD).abs() < 0.001
        || (pad_step - crate::theme::SPACE_LG).abs() < 0.001
        || (pad_step - crate::theme::GUTTER_MERGE_GAP).abs() < 0.001
}

/// Verifies that a budget has no clipping (pane_w > 0) and exactly sums to canvas width.
/// Pure; unit-tested.
pub fn gutter_budget_has_no_clipping_or_overflow(budget: &GutterCanvasBudget) -> bool {
    budget.is_valid
        && budget.pane_w > 0.0
        && (budget.total_allocated_w - budget.canvas_w).abs() < 0.001
}

#[cfg(test)]
mod exq_geo55_tests {
    use super::*;

    #[test]
    fn geo55_single_source_token_delegations() {
        assert_eq!(GUTTER_CANVAS_NARROW_700, 700.0);
        assert_eq!(GUTTER_CANVAS_NORMAL_960, 960.0);
        assert_eq!(GUTTER_CANVAS_WIDE_1200, 1200.0);

        assert_eq!(gutter_geo_outer_margin(), crate::theme::GEO_OUTER_MARGIN);
        assert_eq!(gutter_geo_outer_margin(), 12.0);
        assert_eq!(gutter_geo_space_xs(), crate::theme::SPACE_XS);
        assert_eq!(gutter_geo_space_xs(), 4.0);
        assert_eq!(gutter_geo_space_sm(), crate::theme::SPACE_SM);
        assert_eq!(gutter_geo_space_sm(), 8.0);
        assert_eq!(gutter_geo_space_md(), crate::theme::SPACE_MD);
        assert_eq!(gutter_geo_space_md(), 12.0);
        assert_eq!(gutter_geo_space_lg(), crate::theme::SPACE_LG);
        assert_eq!(gutter_geo_space_lg(), 16.0);
        assert_eq!(gutter_geo_space_xl(), crate::theme::SPACE_XL);
        assert_eq!(gutter_geo_space_xl(), 24.0);
        assert_eq!(gutter_geo_merge_gap(), crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(gutter_geo_merge_gap(), 2.0);

        assert_eq!(gutter_canvas_avail_w(700.0), Some(676.0));
        assert_eq!(gutter_canvas_avail_w(960.0), Some(936.0));
        assert_eq!(gutter_canvas_avail_w(1200.0), Some(1176.0));
        assert_eq!(gutter_canvas_avail_w(20.0), None);
        assert_eq!(gutter_canvas_avail_w(f32::NAN), None);
    }

    #[test]
    fn geo55_budget_narrow_700_parity() {
        // Narrow 700px canvas budget with 1-digit line numbers:
        // 12 margin + 16 linenum + 312 pane + 20 center + 312 pane + 16 linenum + 12 margin = 700.
        let Some(budget_lines) = gutter_budget_narrow_700(7, true) else {
            assert!(false, "narrow budget with lines");
            return;
        };
        assert_eq!(budget_lines.canvas_w, 700.0);
        assert_eq!(budget_lines.outer_margin, 12.0);
        assert_eq!(budget_lines.avail_w, 676.0);
        assert_eq!(budget_lines.center_lane_w, 20.0);
        assert_eq!(budget_lines.left_linenum_w, 16.0);
        assert_eq!(budget_lines.right_linenum_w, 16.0);
        assert_eq!(budget_lines.pane_w, 312.0);
        assert_eq!(budget_lines.total_allocated_w, 700.0);
        assert!(budget_lines.is_valid);
        assert!(gutter_budget_has_no_clipping_or_overflow(&budget_lines));

        // Center lane midpoint check:
        // col_x = 12 + 16 + 312 = 340.0; cx = 340 + 20/2 = 350.0 (700 / 2).
        let center_col_x =
            budget_lines.outer_margin + budget_lines.left_linenum_w + budget_lines.pane_w;
        assert_eq!(center_col_x, 340.0);
        let Some(center_cx) = gutter_mark_centered_cx(center_col_x, budget_lines.center_lane_w)
        else {
            assert!(false, "cx");
            return;
        };
        assert_eq!(center_cx, 350.0);
        assert!(gutter_mark_is_canvas_midpoint_aligned(700.0, center_cx));

        // Narrow 700px canvas without line numbers:
        // 12 margin + 328 pane + 20 center + 328 pane + 12 margin = 700.
        let Some(budget_no_lines) = gutter_budget_narrow_700(7, false) else {
            assert!(false, "narrow budget no lines");
            return;
        };
        assert_eq!(budget_no_lines.left_linenum_w, 0.0);
        assert_eq!(budget_no_lines.right_linenum_w, 0.0);
        assert_eq!(budget_no_lines.pane_w, 328.0);
        assert_eq!(budget_no_lines.total_allocated_w, 700.0);
        assert!(budget_no_lines.is_valid);
        assert!(gutter_budget_has_no_clipping_or_overflow(&budget_no_lines));
    }

    #[test]
    fn geo55_budget_normal_960_parity() {
        // Normal 960px canvas budget with 1-digit line numbers:
        // 12 margin + 16 linenum + 442 pane + 20 center + 442 pane + 16 linenum + 12 margin = 960.
        let Some(budget_lines) = gutter_budget_normal_960(5, true) else {
            assert!(false, "normal budget with lines");
            return;
        };
        assert_eq!(budget_lines.canvas_w, 960.0);
        assert_eq!(budget_lines.pane_w, 442.0);
        assert_eq!(budget_lines.total_allocated_w, 960.0);
        assert!(gutter_budget_has_no_clipping_or_overflow(&budget_lines));

        let center_col_x =
            budget_lines.outer_margin + budget_lines.left_linenum_w + budget_lines.pane_w;
        assert_eq!(center_col_x, 470.0);
        let Some(center_cx) = gutter_mark_centered_cx(center_col_x, budget_lines.center_lane_w)
        else {
            assert!(false, "cx");
            return;
        };
        assert_eq!(center_cx, 480.0);
        assert!(gutter_mark_is_canvas_midpoint_aligned(960.0, center_cx));

        // Normal 960px canvas without line numbers:
        // 12 margin + 458 pane + 20 center + 458 pane + 12 margin = 960.
        let Some(budget_no_lines) = gutter_budget_normal_960(5, false) else {
            assert!(false, "normal budget no lines");
            return;
        };
        assert_eq!(budget_no_lines.pane_w, 458.0);
        assert_eq!(budget_no_lines.total_allocated_w, 960.0);
        assert!(gutter_budget_has_no_clipping_or_overflow(&budget_no_lines));
    }

    #[test]
    fn geo55_budget_wide_1200_parity() {
        // Wide 1200px canvas budget with 1-digit line numbers:
        // 12 margin + 16 linenum + 562 pane + 20 center + 562 pane + 16 linenum + 12 margin = 1200.
        let Some(budget_lines) = gutter_budget_wide_1200(9, true) else {
            assert!(false, "wide budget with lines");
            return;
        };
        assert_eq!(budget_lines.canvas_w, 1200.0);
        assert_eq!(budget_lines.pane_w, 562.0);
        assert_eq!(budget_lines.total_allocated_w, 1200.0);
        assert!(gutter_budget_has_no_clipping_or_overflow(&budget_lines));

        let center_col_x =
            budget_lines.outer_margin + budget_lines.left_linenum_w + budget_lines.pane_w;
        assert_eq!(center_col_x, 590.0);
        let Some(center_cx) = gutter_mark_centered_cx(center_col_x, budget_lines.center_lane_w)
        else {
            assert!(false, "cx");
            return;
        };
        assert_eq!(center_cx, 600.0);
        assert!(gutter_mark_is_canvas_midpoint_aligned(1200.0, center_cx));

        // Wide 1200px canvas without line numbers:
        // 12 margin + 578 pane + 20 center + 578 pane + 12 margin = 1200.
        let Some(budget_no_lines) = gutter_budget_wide_1200(9, false) else {
            assert!(false, "wide budget no lines");
            return;
        };
        assert_eq!(budget_no_lines.pane_w, 578.0);
        assert_eq!(budget_no_lines.total_allocated_w, 1200.0);
        assert!(gutter_budget_has_no_clipping_or_overflow(&budget_no_lines));
    }

    #[test]
    fn geo55_omit_marker_and_alignment_parity() {
        assert_eq!(GUTTER_OMIT_MARKER_COL_W, crate::theme::SPACE_LG);
        assert_eq!(gutter_omit_marker_col_w(), 16.0);
        assert_eq!(GUTTER_OMIT_MARKER_BOX, crate::theme::SPACE_MD);
        assert_eq!(gutter_omit_marker_box(), 12.0);
        assert_eq!(gutter_omit_marker_pad(), 2.0);

        // Omit marker placed in row: col 100..116, row 50..70 (20px high)
        // cx = 100 + 16/2 = 108.0, cy = 50 + 20/2 = 60.0. Box 12x12 -> x: 102..114, y: 54..66.
        let rect = gutter_omit_marker_rect(100.0, 50.0, 20.0);
        assert_eq!(rect, Some((102.0, 54.0, 114.0, 66.0)));
        assert_eq!(gutter_omit_marker_rect(f32::NAN, 50.0, 20.0), None);

        // Hit testing with 2px slop: box is 102..114 x 54..66; slop makes it 100..116 x 52..68.
        assert!(gutter_omit_marker_hit_contains(
            100.0, 50.0, 20.0, 108.0, 60.0
        ));
        assert!(gutter_omit_marker_hit_contains(
            100.0, 50.0, 20.0, 100.0, 52.0
        ));
        assert!(gutter_omit_marker_hit_contains(
            100.0, 50.0, 20.0, 116.0, 68.0
        ));
        assert!(!gutter_omit_marker_hit_contains(
            100.0, 50.0, 20.0, 99.0, 60.0
        ));
        assert!(!gutter_omit_marker_hit_contains(
            100.0, 50.0, 20.0, 117.0, 60.0
        ));

        // Right-aligned line number x:
        // col 10..30 (width 20), pad_right 4 -> x = 26.
        assert_eq!(gutter_linenum_right_aligned_x(10.0, 20.0, 4.0), Some(26.0));
        assert_eq!(gutter_linenum_right_aligned_x(10.0, 20.0, 25.0), None);
        assert_eq!(gutter_linenum_right_aligned_x(f32::NAN, 20.0, 4.0), None);

        // No double padding verification
        assert!(gutter_verify_no_double_padding(12.0, 4.0));
        assert!(gutter_verify_no_double_padding(12.0, 8.0));
        assert!(gutter_verify_no_double_padding(12.0, 2.0));
        assert!(!gutter_verify_no_double_padding(16.0, 4.0)); // wrong outer margin
        assert!(!gutter_verify_no_double_padding(12.0, 5.0)); // non-token pad
    }

    #[test]
    fn geo55_narrow_700_edge_fixture() {
        let fixture_raw =
            include_str!("../../../checks/gui-exq-geo55/round-1/comparison/gutter-narrow-700.json");
        assert!(fixture_raw.contains("\"name\": \"gutter-narrow-700\""));
        assert!(fixture_raw.contains("\"canvas_w\": 700.0"));
        assert!(fixture_raw.contains("\"outer_margin\": 12.0"));
        assert!(fixture_raw.contains("\"avail_w\": 676.0"));
        assert!(fixture_raw.contains("\"center_lane_w\": 20.0"));
        assert!(fixture_raw.contains("\"center_lane_col_x\": 340.0"));
        assert!(fixture_raw.contains("\"center_lane_cx\": 350.0"));
        assert!(fixture_raw.contains("\"is_canvas_midpoint_aligned\": true"));
        assert!(fixture_raw.contains("\"linenum_col_w_1_digit\": 16.0"));
        assert!(fixture_raw.contains("\"pane_w_with_1_digit_linenum\": 312.0"));
        assert!(fixture_raw.contains("\"total_allocated_w_1_digit\": 700.0"));
        assert!(fixture_raw.contains("\"pane_w_without_linenum\": 328.0"));
        assert!(fixture_raw.contains("\"total_allocated_w_without_linenum\": 700.0"));
        assert!(fixture_raw.contains("\"copy_arrow_box\": 16.0"));
        assert!(fixture_raw.contains("\"copy_arrow_pad_x\": 2.0"));
        assert!(fixture_raw.contains("\"omit_marker_col_w\": 16.0"));
        assert!(fixture_raw.contains("\"omit_marker_box\": 12.0"));
        assert!(fixture_raw.contains("\"omit_marker_pad\": 2.0"));
        assert!(fixture_raw.contains("\"has_clip\": false"));
        assert!(fixture_raw.contains("\"has_double_padding\": false"));

        // Match against live helpers
        let Some(budget_with_lines) = gutter_budget_narrow_700(7, true) else {
            assert!(false, "live budget with lines");
            return;
        };
        assert_eq!(budget_with_lines.canvas_w, 700.0);
        assert_eq!(budget_with_lines.avail_w, 676.0);
        assert_eq!(budget_with_lines.center_lane_w, 20.0);
        assert_eq!(budget_with_lines.pane_w, 312.0);
        assert_eq!(budget_with_lines.total_allocated_w, 700.0);

        let Some(budget_no_lines) = gutter_budget_narrow_700(7, false) else {
            assert!(false, "live budget no lines");
            return;
        };
        assert_eq!(budget_no_lines.pane_w, 328.0);
        assert_eq!(budget_no_lines.total_allocated_w, 700.0);

        assert_eq!(gutter_omit_marker_col_w(), 16.0);
        assert_eq!(gutter_omit_marker_box(), 12.0);
        assert_eq!(gutter_omit_marker_pad(), 2.0);
        assert!(gutter_mark_is_canvas_midpoint_aligned(700.0, 350.0));
    }
}

#[cfg(test)]
mod exq_gutter59_tests {
    use super::*;

    #[test]
    fn gutter59_canvas_widths_ride_single_source_tokens() {
        // Gutter-owned canvas consts stay on the canonical budgets...
        assert_eq!(GUTTER_CANVAS_NORMAL_960, crate::theme::VIEWPORT_W as f32);
        assert_eq!(GUTTER_CANVAS_NORMAL_960, 960.0);
        assert_eq!(GUTTER_CANVAS_WIDE_1200, 1200.0);
        assert_eq!(GUTTER_CANVAS_NARROW_700, 700.0);
        // ...and the narrow layouts ride the viewport-min token (720).
        let narrow_min = crate::theme::VIEWPORT_MIN_W as f32;
        assert_eq!(narrow_min, 720.0);
        for layout in [gutter_side_layout_720(), gutter_text_layout_720()] {
            let (m0, left, center, right, m1) = layout;
            assert!((m0 + left + center + right + m1 - narrow_min).abs() < 0.001);
        }
        for layout in [gutter_side_layout_960(), gutter_text5_layout_960()] {
            let (m0, left, center, right, m1) = layout;
            assert!((m0 + left + center + right + m1 - GUTTER_CANVAS_NORMAL_960).abs() < 0.001);
        }
        for layout in [gutter_side_layout_1200(), gutter_text_layout_1200()] {
            let (m0, left, center, right, m1) = layout;
            assert!((m0 + left + center + right + m1 - GUTTER_CANVAS_WIDE_1200).abs() < 0.001);
        }
    }

    #[test]
    fn gutter59_token_layouts_match_literal_budgets() {
        // Byte-identical budgets: token adoption moves no pixel.
        assert_eq!(gutter_side_layout_720(), (12.0, 338.0, 20.0, 338.0, 12.0));
        assert_eq!(gutter_text_layout_720(), (12.0, 336.0, 24.0, 336.0, 12.0));
        assert_eq!(gutter_side_layout_960(), (12.0, 458.0, 20.0, 458.0, 12.0));
        assert_eq!(gutter_text5_layout_960(), (12.0, 456.0, 24.0, 456.0, 12.0));
        assert_eq!(gutter_side_layout_1200(), (12.0, 578.0, 20.0, 578.0, 12.0));
        assert_eq!(gutter_text_layout_1200(), (12.0, 576.0, 24.0, 576.0, 12.0));
        // Tall floor still pins the 1200 track: top band at pad, full file fills inner track.
        assert_eq!(
            gutter_viewport_tall_floor_1200(1200, 0, 1),
            Some((4.0, 8.0))
        );
        assert_eq!(
            gutter_viewport_tall_floor_1200(1200, 0, 1200),
            Some((4.0, 1196.0))
        );
    }

    #[test]
    fn gutter59_spacing_typo_tokens_unchanged() {
        // Zero new sizes: every width still rides SPACE / merge-gap / grid tokens.
        assert_eq!(GUTTER_CENTER_W, crate::theme::SPACE_XL);
        assert_eq!(
            GUTTER_SIDE_MARKER_COL_W,
            crate::theme::SPACE_LG + crate::theme::SPACE_XS
        );
        assert_eq!(GUTTER_SIDE_HIT_SLOP, crate::theme::GUTTER_MERGE_GAP);
        assert_eq!(gutter_text_role(), crate::theme::TypoRole::content_grid());
        assert_eq!(gutter_text_size(), crate::theme::TYPO_MONO_SIZE);
    }

    #[test]
    fn gutter59_canvas_budgets_fixture() {
        // Mirrors checks/gui-exq-gutter59/round-1/comparison/gutter-canvas-budgets.json.
        let fixture_raw = include_str!(
            "../../../checks/gui-exq-gutter59/round-1/comparison/gutter-canvas-budgets.json"
        );
        assert!(fixture_raw.contains("\"name\": \"gutter-canvas-budgets\""));
        assert!(fixture_raw.contains("\"canvas_960\": 960.0"));
        assert!(fixture_raw.contains("\"canvas_720\": 720.0"));
        assert!(fixture_raw.contains("\"canvas_1200\": 1200.0"));
        // Live helpers reproduce the fixture budgets exactly.
        assert_eq!(gutter_side_layout_720(), (12.0, 338.0, 20.0, 338.0, 12.0));
        assert_eq!(gutter_text_layout_720(), (12.0, 336.0, 24.0, 336.0, 12.0));
        assert_eq!(gutter_side_layout_960(), (12.0, 458.0, 20.0, 458.0, 12.0));
        assert_eq!(gutter_text5_layout_960(), (12.0, 456.0, 24.0, 456.0, 12.0));
        assert_eq!(gutter_side_layout_1200(), (12.0, 578.0, 20.0, 578.0, 12.0));
        assert_eq!(gutter_text_layout_1200(), (12.0, 576.0, 24.0, 576.0, 12.0));
    }
}

#[cfg(test)]
mod exq_agyxswarm80_gutter_tests {
    use super::*;

    #[test]
    fn gutter_geometry_space_scale_tokens_hold() {
        use crate::theme::{SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, SPACE_XS};

        // XS step (4.0)
        assert_eq!(gutter_geo_space_xs(), SPACE_XS);
        assert_eq!(gutter_geo_space_xs(), 4.0);
        assert_eq!(GUTTER_SEGMENT_MIN_H, SPACE_XS);
        assert_eq!(GUTTER_VIEWPORT_MIN_H, SPACE_XS);
        assert_eq!(GUTTER_LINENUM_PAD, SPACE_XS);
        assert_eq!(GUTTER_MINIMAP_TRACK_PAD, SPACE_XS);
        assert_eq!(GUTTER_CENTER_SIDE_GAP, SPACE_XS);
        assert_eq!(gutter_geo_segment_min_h(), SPACE_XS);
        assert_eq!(gutter_geo_viewport_min_h(), SPACE_XS);
        assert_eq!(gutter_geo_linenum_pad(), SPACE_XS);
        assert_eq!(gutter_geo_minimap_track_pad(), SPACE_XS);
        assert_eq!(gutter_geo_center_side_gap(), SPACE_XS);

        // SM step (8.0)
        assert_eq!(gutter_geo_space_sm(), SPACE_SM);
        assert_eq!(gutter_geo_space_sm(), 8.0);
        assert_eq!(GUTTER_RULER_NARROW, SPACE_SM);
        assert_eq!(gutter_geo_ruler_narrow(), SPACE_SM);

        // MD step (12.0)
        assert_eq!(gutter_geo_space_md(), SPACE_MD);
        assert_eq!(gutter_geo_space_md(), 12.0);
        assert_eq!(crate::theme::GUTTER_BAR_WIDE, SPACE_MD);
        assert_eq!(GUTTER_OMIT_MARKER_BOX, SPACE_MD);
        assert_eq!(gutter_geo_outer_margin(), crate::theme::GEO_OUTER_MARGIN);
        assert_eq!(crate::theme::GEO_OUTER_MARGIN, SPACE_MD);

        // LG step (16.0)
        assert_eq!(gutter_geo_space_lg(), SPACE_LG);
        assert_eq!(gutter_geo_space_lg(), 16.0);
        assert_eq!(GUTTER_RULER_WIDE, SPACE_LG);
        assert_eq!(GUTTER_ARROW_BOX, SPACE_LG);
        assert_eq!(GUTTER_ICON_COL_W, SPACE_LG);
        assert_eq!(GUTTER_SIDE_MARKER_BOX, SPACE_LG);
        assert_eq!(GUTTER_OMIT_MARKER_COL_W, SPACE_LG);
        assert_eq!(TEXT38_COPY_ARROW_BOX, SPACE_LG);
        assert_eq!(gutter_geo_ruler_wide(), SPACE_LG);
        assert_eq!(gutter_geo_arrow_box(), SPACE_LG);
        assert_eq!(gutter_geo_side_marker_box(), SPACE_LG);

        // XL step (24.0)
        assert_eq!(gutter_geo_space_xl(), SPACE_XL);
        assert_eq!(gutter_geo_space_xl(), 24.0);
        assert_eq!(GUTTER_CENTER_W, SPACE_XL);
        assert_eq!(gutter_geo_center_w(), SPACE_XL);

        // Additive 20px step (SPACE_LG + SPACE_XS)
        assert_eq!(GUTTER_SIDE_MARKER_COL_W, SPACE_LG + SPACE_XS);
        assert_eq!(TEXT38_CENTER_LANE_W, SPACE_LG + SPACE_XS);
        assert_eq!(gutter_geo_center_lane_w(), SPACE_LG + SPACE_XS);
        assert_eq!(gutter_geo_side_marker_col_w(), SPACE_LG + SPACE_XS);

        // Strict monotonic spacing scale
        assert!(SPACE_XS < SPACE_SM);
        assert!(SPACE_SM < SPACE_MD);
        assert!(SPACE_MD < SPACE_LG);
        assert!(SPACE_LG < SPACE_XL);
    }

    #[test]
    fn gutter_geometry_canvas_budget_space_reuse() {
        use crate::theme::{GEO_OUTER_MARGIN, SPACE_LG, SPACE_SM, SPACE_XL, SPACE_XS};

        assert_eq!(GUTTER_RULER_NARROW, SPACE_SM);
        assert_eq!(GUTTER_CENTER_W, SPACE_XL);

        // Narrow 700 budget: 12 outer margin + twin 312 panes + 20 center lane + twin 16 line numbers = 700
        let Some(b700) = gutter_budget_narrow_700(7, true) else {
            panic!("b700 must be valid");
        };
        assert_eq!(b700.canvas_w, GUTTER_CANVAS_NARROW_700);
        assert_eq!(b700.outer_margin, GEO_OUTER_MARGIN);
        assert_eq!(b700.center_lane_w, SPACE_LG + SPACE_XS);
        assert_eq!(b700.left_linenum_w, SPACE_LG);
        assert_eq!(b700.right_linenum_w, SPACE_LG);
        assert_eq!(b700.total_allocated_w, GUTTER_CANVAS_NARROW_700);

        // Normal 960 budget: 12 outer margin + twin 442 panes + 20 center lane + twin 16 line numbers = 960
        let Some(b960) = gutter_budget_normal_960(5, true) else {
            panic!("b960 must be valid");
        };
        assert_eq!(b960.canvas_w, GUTTER_CANVAS_NORMAL_960);
        assert_eq!(b960.outer_margin, GEO_OUTER_MARGIN);
        assert_eq!(b960.center_lane_w, SPACE_LG + SPACE_XS);
        assert_eq!(b960.left_linenum_w, SPACE_LG);
        assert_eq!(b960.right_linenum_w, SPACE_LG);
        assert_eq!(b960.total_allocated_w, GUTTER_CANVAS_NORMAL_960);

        // Wide 1200 budget: 12 outer margin + twin 562 panes + 20 center lane + twin 16 line numbers = 1200
        let Some(b1200) = gutter_budget_wide_1200(9, true) else {
            panic!("b1200 must be valid");
        };
        assert_eq!(b1200.canvas_w, GUTTER_CANVAS_WIDE_1200);
        assert_eq!(b1200.outer_margin, GEO_OUTER_MARGIN);
        assert_eq!(b1200.center_lane_w, SPACE_LG + SPACE_XS);
        assert_eq!(b1200.left_linenum_w, SPACE_LG);
        assert_eq!(b1200.right_linenum_w, SPACE_LG);
        assert_eq!(b1200.total_allocated_w, GUTTER_CANVAS_WIDE_1200);

        // Single-source canvas consts
        assert_eq!(TEXT38_CANVAS_WIDE_W, GUTTER_CANVAS_WIDE_1200);
        assert_eq!(TEXT38_CANVAS_NARROW_W, GUTTER_CANVAS_NARROW_700);
    }

    #[test]
    fn gutter_geometry_hit_slop_and_merge_tokens_hold() {
        use crate::theme::GUTTER_MERGE_GAP;

        assert_eq!(gutter_geo_merge_gap(), GUTTER_MERGE_GAP);
        assert_eq!(gutter_geo_merge_gap(), 2.0);
        assert_eq!(GUTTER_HIT_SLOP, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_TEXT_ROW_PAD, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_ARROW_HIT_SLOP, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_ARROW_GLYPH_INSET, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_LINENUM_INSET, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_SIDE_MARKER_INSET, GUTTER_MERGE_GAP);
        assert_eq!(GUTTER_SIDE_HIT_SLOP, GUTTER_MERGE_GAP);
        assert_eq!(TEXT38_COPY_ARROW_PAD_X, GUTTER_MERGE_GAP);
    }

    #[test]
    fn gutter_geometry_pure_safe_and_frozen_colors() {
        // Verification that pure safe calculation contracts hold:
        assert!(gutter_verify_no_double_padding(12.0, 4.0));
        assert!(gutter_verify_no_double_padding(12.0, 8.0));
        assert!(gutter_verify_no_double_padding(12.0, 12.0));
        assert!(gutter_verify_no_double_padding(12.0, 16.0));
        assert!(gutter_verify_no_double_padding(12.0, 2.0));
        assert!(!gutter_verify_no_double_padding(14.0, 4.0));
        assert!(!gutter_verify_no_double_padding(12.0, 5.0));

        // Invalid geometry inputs fail safely without panic:
        assert_eq!(gutter_canvas_avail_w(f32::NAN), None);
        assert_eq!(gutter_canvas_avail_w(-10.0), None);
        assert_eq!(gutter_mark_centered_cx(f32::NAN, 20.0), None);
        assert_eq!(gutter_mark_centered_cy(10.0, f32::NAN), None);
        assert_eq!(gutter_center_mark_rect(f32::NAN, 10.0, 16.0), None);
        assert_eq!(gutter_linenum_right_aligned_x(f32::NAN, 20.0, 4.0), None);
        assert_eq!(gutter_omit_marker_rect(f32::NAN, 10.0, 20.0), None);
    }
}
