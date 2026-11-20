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
    let y0 = pad + (top / total_f) * inner;
    let mut y1 = pad + (bottom / total_f) * inner;
    if y1 < y0 + GUTTER_VIEWPORT_MIN_H {
        y1 = (y0 + GUTTER_VIEWPORT_MIN_H).min(pad + inner);
    }
    if !y0.is_finite() || !y1.is_finite() || y0 < pad || y1 > pad + inner || y0 >= y1 {
        return None;
    }
    Some((y0, y1))
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
        // Khali/kharab input: None.
        assert_eq!(gutter_viewport_rect_padded(0, 600.0, 0, 2), None);
        assert_eq!(gutter_viewport_rect_padded(6, 600.0, 0, 0), None);
        assert_eq!(gutter_viewport_rect_padded(6, 8.0, 0, 2), None);
        assert_eq!(gutter_viewport_rect_padded(6, 600.0, 99, 2), None);
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
    gutter_text_layout(crate::theme::GEO_OUTER_MARGIN, GUTTER_CENTER_W, 960.0)
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
    gutter_text_layout(crate::theme::GEO_OUTER_MARGIN, GUTTER_CENTER_W, 720.0)
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
