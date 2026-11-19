//! Pure view-model builders: engine results turned into row models.
//!
//! This module is the GUI's test surface. It holds no egui types, no
//! I/O and no panics, so the same hostile-input guarantees the engine
//! crates give us hold here too: a bad path, a huge file or an
//! inconsistent hand-built image becomes a smaller model, never a
//! crash.
//!
//! The builders mirror the CLI renderers' semantics
//! (`crates/cli/src/render_text.rs`, `render_folder.rs`,
//! `render_picture.rs`) so both clients tell the same story: equal
//! runs pair line numbers, a delete run followed by an insert run
//! becomes paired "changed" rows carrying word-level accents, folder
//! entries get the same status glyphs, and picture stats use the same
//! wording.

use std::time::{SystemTime, UNIX_EPOCH};

// ponytail: re-export modular headless helpers for Home, gutter, copy, Take, tree, and theme contrast
#[allow(unused_imports)]
pub use crate::copy::{CopyDirection, CopyError, apply_copy, copy_line, copy_section};
#[allow(unused_imports)]
pub use crate::gutter::{GutterGeometry, GutterSegment, gutter_geometry};
#[allow(unused_imports)]
pub use crate::home::{HomeItem, HomeModel, Tab, home_model};
#[allow(unused_imports)]
pub use crate::take::{TakeChoice, TakeError, resolve_take};
#[allow(unused_imports)]
pub use crate::theme::{
    relative_luminance, theme_contrast_ok, theme_contrast_ratio, theme_mode_contrast_ok,
};
#[allow(unused_imports)]
pub use crate::tree::{FolderTreeNode, folder_tree};

use autocompare_compare::{
    BinaryComparison, FolderComparison, FolderStatus, SyncAction, SyncPlan, TableComparison,
    TextComparison, TreeMergeAction, TreeMergePlan,
};
use autocompare_diff::{AudioDiff, Op, PictureDiff, RgbaImage, diff_cells, inline_diff};

/// Cap on modeled text rows. Comparisons longer than this stop early
/// and set [`TextRows::truncated`]; painting virtualizes rows anyway,
/// so the cap bounds model memory (20_000 rows of two strings each),
/// not per-frame work.
pub const MAX_ROWS: usize = 20_000;

// ----------------------------------------------------------------------------
// Text
// ----------------------------------------------------------------------------

/// How one aligned text row compares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextRowKind {
    /// Present and equal on both sides.
    #[default]
    Equal,
    /// Only on the left side (was deleted).
    Deleted,
    /// Only on the right side (was inserted).
    Inserted,
    /// Paired by a delete+insert run; carries inline accents.
    Changed,
}

/// One aligned row of the side-by-side text view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextRow {
    /// Comparison verdict for this row.
    pub kind: TextRowKind,
    /// Left line number, when the row has a left side.
    pub left_no: Option<usize>,
    /// Right line number, when the row has a right side.
    pub right_no: Option<usize>,
    /// Left text (empty when the row has no left side).
    pub left: String,
    /// Right text (empty when the row has no right side).
    pub right: String,
    /// Accent ranges in `left`: half-open character offsets from
    /// `inline_diff`, zero-length ranges meaning "nothing".
    pub accent_a: Vec<(usize, usize)>,
    /// Accent ranges in `right`, same convention.
    pub accent_b: Vec<(usize, usize)>,
}

/// The full model of one text comparison.
#[derive(Debug, Clone, Default)]
pub struct TextRows {
    /// Aligned rows, at most [`MAX_ROWS`].
    pub rows: Vec<TextRow>,
    /// More rows existed beyond the cap; the model stops there.
    pub truncated: bool,
    /// The edit script exceeded the engine's budget and degraded to a
    /// coarse replace; shown as a note, like the CLI's.
    pub coarse: bool,
    /// Widest line in characters; painting pads every pane to this so
    /// the two columns stay aligned.
    pub pane_chars: usize,
    /// Digit width of the largest line number (minimum 4, like the
    /// CLI's 4-wide numbers).
    pub gutter_chars: usize,
    /// Selected row index (original model index); navigation moves it.
    pub selected: Option<usize>,
    /// Current display filter.
    pub filter: TextFilter,
}

/// Walks the edit script the way the CLI's `blocks()` does: equal
/// runs pair both sides, a delete run followed by an insert run pairs
/// index-wise into changed rows (leftovers become deleted/inserted
/// rows), and lone runs keep their own kind. Row numbering follows
/// the CLI exactly.
pub fn build_text_rows(cmp: &TextComparison) -> TextRows {
    let mut walk = TextWalk {
        left_no: 0,
        right_no: 0,
        left_idx: 0,
        right_idx: 0,
        rows: Vec::new(),
        truncated: false,
    };
    let mut ops = cmp.script.ops.iter().cloned().peekable();
    while !walk.full() {
        let Some(op) = ops.next() else {
            break;
        };
        match op {
            Op::Equal { len } => walk.equal_run(&cmp.left_lines, &cmp.right_lines, len),
            Op::Delete { len } => {
                let ins_len = match ops.peek() {
                    Some(Op::Insert { len }) => Some(*len),
                    _ => None,
                };
                if ins_len.is_some() {
                    ops.next();
                }
                match ins_len {
                    Some(ins) => walk.changed_run(&cmp.left_lines, &cmp.right_lines, len, ins),
                    None => walk.one_sided_run(&cmp.left_lines, len, true),
                }
            }
            Op::Insert { len } => {
                let del_len = match ops.peek() {
                    Some(Op::Delete { len }) => Some(*len),
                    _ => None,
                };
                if del_len.is_some() {
                    ops.next();
                }
                match del_len {
                    Some(del) => walk.changed_run(&cmp.left_lines, &cmp.right_lines, del, len),
                    None => walk.one_sided_run(&cmp.right_lines, len, false),
                }
            }
        }
    }
    let truncated = walk.truncated;
    let rows = walk.rows;

    let mut pane_chars = 1usize; // one char keeps degenerate panes usable
    let mut gutter_chars = 4usize;
    for row in &rows {
        pane_chars = pane_chars
            .max(row.left.chars().count())
            .max(row.right.chars().count());
        if let Some(no) = row.left_no {
            gutter_chars = gutter_chars.max(digits(no));
        }
        if let Some(no) = row.right_no {
            gutter_chars = gutter_chars.max(digits(no));
        }
    }

    TextRows {
        rows,
        truncated,
        coarse: !cmp.script.exact,
        pane_chars,
        gutter_chars,
        selected: None,
        filter: TextFilter::All,
    }
}

/// Running state of the script walk. Line numbers and consumption
/// indices advance one row at a time; all arithmetic is saturating.
struct TextWalk {
    left_no: usize,
    right_no: usize,
    left_idx: usize,
    right_idx: usize,
    rows: Vec<TextRow>,
    truncated: bool,
}

impl TextWalk {
    fn full(&self) -> bool {
        self.rows.len() >= MAX_ROWS
    }

    /// Pushes a row, or notes truncation when the cap is reached.
    fn push(&mut self, row: TextRow) {
        if self.full() {
            self.truncated = true;
        } else {
            self.rows.push(row);
        }
    }

    fn equal_run(&mut self, left: &[String], right: &[String], len: usize) {
        for _ in 0..len {
            if self.full() {
                self.truncated = true;
                return;
            }
            let l = line_at(left, self.left_idx);
            let r = line_at(right, self.right_idx);
            self.left_idx = self.left_idx.saturating_add(1);
            self.right_idx = self.right_idx.saturating_add(1);
            self.left_no = self.left_no.saturating_add(1);
            self.right_no = self.right_no.saturating_add(1);
            self.push(TextRow {
                kind: TextRowKind::Equal,
                left_no: Some(self.left_no),
                right_no: Some(self.right_no),
                left: l,
                right: r,
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            });
        }
    }

    /// A lone delete or insert run: one row per line, one side only.
    fn one_sided_run(&mut self, lines: &[String], len: usize, left_side: bool) {
        for _ in 0..len {
            if self.full() {
                self.truncated = true;
                return;
            }
            let text = line_at(
                lines,
                if left_side {
                    self.left_idx
                } else {
                    self.right_idx
                },
            );
            if left_side {
                self.left_idx = self.left_idx.saturating_add(1);
                self.left_no = self.left_no.saturating_add(1);
            } else {
                self.right_idx = self.right_idx.saturating_add(1);
                self.right_no = self.right_no.saturating_add(1);
            }
            let (kind, left_no, right_no, left, right) = if left_side {
                (
                    TextRowKind::Deleted,
                    Some(self.left_no),
                    None,
                    text,
                    String::new(),
                )
            } else {
                (
                    TextRowKind::Inserted,
                    None,
                    Some(self.right_no),
                    String::new(),
                    text,
                )
            };
            self.push(TextRow {
                kind,
                left_no,
                right_no,
                left,
                right,
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            });
        }
    }

    /// A delete run paired with an adjacent insert run: index-wise
    /// pairs become changed rows carrying `inline_diff` accents,
    /// leftovers keep their own kind, exactly like the CLI's changed
    /// blocks.
    fn changed_run(&mut self, left: &[String], right: &[String], del: usize, ins: usize) {
        let left_run = take_run(left, self.left_idx, del);
        self.left_idx = self.left_idx.saturating_add(del);
        let right_run = take_run(right, self.right_idx, ins);
        self.right_idx = self.right_idx.saturating_add(ins);

        let pairs = left_run.len().min(right_run.len());
        for i in 0..pairs {
            if self.full() {
                self.truncated = true;
                return;
            }
            let l = line_at(&left_run, i);
            let r = line_at(&right_run, i);
            self.left_no = self.left_no.saturating_add(1);
            self.right_no = self.right_no.saturating_add(1);
            let (mut accent_a, mut accent_b) = (Vec::new(), Vec::new());
            for span in inline_diff(&l, &r) {
                accent_a.push((span.a_start, span.a_end));
                accent_b.push((span.b_start, span.b_end));
            }
            self.push(TextRow {
                kind: TextRowKind::Changed,
                left_no: Some(self.left_no),
                right_no: Some(self.right_no),
                left: l,
                right: r,
                accent_a,
                accent_b,
            });
        }
        // Leftover left lines: deleted rows, left numbering only.
        for i in pairs..left_run.len() {
            if self.full() {
                self.truncated = true;
                return;
            }
            self.left_no = self.left_no.saturating_add(1);
            self.push(TextRow {
                kind: TextRowKind::Deleted,
                left_no: Some(self.left_no),
                right_no: None,
                left: line_at(&left_run, i),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            });
        }
        // Leftover right lines: inserted rows, right numbering only.
        for i in pairs..right_run.len() {
            if self.full() {
                self.truncated = true;
                return;
            }
            self.right_no = self.right_no.saturating_add(1);
            self.push(TextRow {
                kind: TextRowKind::Inserted,
                left_no: None,
                right_no: Some(self.right_no),
                left: String::new(),
                right: line_at(&right_run, i),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            });
        }
    }
}

/// The line at `idx`, or an empty string when the script points past
/// the lines (a defensive engine result stays paintable).
fn line_at(lines: &[String], idx: usize) -> String {
    lines.get(idx).cloned().unwrap_or_default()
}

/// Up to `len` lines starting at `start`, clamped to what exists.
fn take_run(lines: &[String], start: usize, len: usize) -> Vec<String> {
    let end = start.saturating_add(len).min(lines.len());
    let mut out = Vec::new();
    let mut i = start;
    while i < end {
        if let Some(line) = lines.get(i) {
            out.push(line.clone());
        }
        i = i.saturating_add(1);
    }
    out
}

/// Decimal digits of `n` (1 for zero).
fn digits(n: usize) -> usize {
    n.to_string().len()
}

// ----------------------------------------------------------------------------
// Text navigation, selection and display filters (headless)
// ----------------------------------------------------------------------------

/// Display filter for the text view: which rows stay visible.
/// `Context` shows every difference plus `CONTEXT_RADIUS` equal rows
/// around each hunk, like a unified diff with context.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextFilter {
    /// Show every row.
    #[default]
    All,
    /// Show only changed/deleted/inserted rows.
    Differences,
    /// Show only equal rows.
    Same,
    /// Show differences plus surrounding equal rows.
    Context,
}

/// How many equal rows `TextFilter::Context` keeps around each hunk.
pub const CONTEXT_RADIUS: usize = 2;

/// Honest counts for the current filter: total model rows plus what
/// the filter shows, split by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FilterCounts {
    /// Rows in the full model.
    pub total: usize,
    /// Rows visible under the current filter.
    pub shown: usize,
    /// Difference rows in the full model.
    pub diffs: usize,
    /// Equal rows in the full model.
    pub sames: usize,
}

/// Whether a row counts as a difference (anything but `Equal`).
pub fn is_diff_row(row: &TextRow) -> bool {
    !matches!(row.kind, TextRowKind::Equal)
}

/// Original-model indices of all difference rows, in order.
pub fn diff_indices(rows: &[TextRow]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    for row in rows {
        if is_diff_row(row) {
            out.push(idx);
        }
        idx = idx.saturating_add(1);
    }
    out
}

/// Next difference strictly after `from` (`None` means from before start).
/// Wraps to the first difference when past the end, so Prev/Next can
/// cycle like a find loop.
pub fn next_diff_idx(rows: &[TextRow], from: Option<usize>) -> Option<usize> {
    let start = match from {
        Some(i) => i.saturating_add(1),
        None => 0,
    };
    let mut idx = start;
    while idx < rows.len() {
        if rows.get(idx).is_some_and(is_diff_row) {
            return Some(idx);
        }
        idx = idx.saturating_add(1);
    }
    // Wrap: first difference from the top.
    let mut idx = 0usize;
    while idx < start.min(rows.len()) {
        if rows.get(idx).is_some_and(is_diff_row) {
            return Some(idx);
        }
        idx = idx.saturating_add(1);
    }
    None
}

/// Previous difference strictly before `from` (`None` means from past end).
/// Wraps to the last difference when before the start.
pub fn prev_diff_idx(rows: &[TextRow], from: Option<usize>) -> Option<usize> {
    let end = match from {
        Some(0) | None => rows.len(),
        Some(i) => i.min(rows.len()),
    };
    // Search backwards in [0, end).
    let mut idx = end;
    while idx > 0 {
        idx = idx.saturating_sub(1);
        if rows.get(idx).is_some_and(is_diff_row) {
            // Only accept rows strictly before `from`; when `from` is
            // None we start from the end so the first hit is the last diff.
            match from {
                Some(f) if idx >= f => continue,
                _ => return Some(idx),
            }
        }
    }
    // Wrap: last difference in the model.
    if from.is_some() {
        let mut idx = rows.len();
        while idx > end {
            idx = idx.saturating_sub(1);
            if rows.get(idx).is_some_and(is_diff_row) {
                return Some(idx);
            }
        }
    }
    None
}

/// Moves selection to the next difference; no-op when no diffs.
pub fn select_next_diff(rows: &[TextRow], selected: &mut Option<usize>) {
    if let Some(next) = next_diff_idx(rows, *selected) {
        *selected = Some(next);
    }
}

/// Moves selection to the previous difference; no-op when no diffs.
/// When nothing is selected, Prev jumps to the last difference.
pub fn select_prev_diff(rows: &[TextRow], selected: &mut Option<usize>) {
    let from = match *selected {
        Some(i) => Some(i),
        None => {
            if rows.is_empty() {
                None
            } else {
                Some(rows.len())
            }
        }
    };
    if let Some(prev) = prev_diff_idx(rows, from) {
        *selected = Some(prev);
    }
}

/// Contiguous non-equal hunk containing `idx`: `(start, end)` with
/// `start <= idx < end`. `None` when out of bounds or on an equal row.
pub fn section_bounds(rows: &[TextRow], idx: usize) -> Option<(usize, usize)> {
    rows.get(idx)?;
    if !rows.get(idx).is_some_and(is_diff_row) {
        return None;
    }
    let mut start = idx;
    while start > 0 {
        let prev = start.saturating_sub(1);
        match rows.get(prev) {
            Some(r) if is_diff_row(r) => start = prev,
            _ => break,
        }
    }
    let mut end = idx.saturating_add(1);
    while end < rows.len() {
        match rows.get(end) {
            Some(r) if is_diff_row(r) => end = end.saturating_add(1),
            _ => break,
        }
    }
    Some((start, end))
}

/// Original-model indices visible under `filter`, in order.
/// `Context` keeps `CONTEXT_RADIUS` equal rows around every hunk.
pub fn filtered_indices(rows: &[TextRow], filter: TextFilter) -> Vec<usize> {
    match filter {
        TextFilter::All => {
            let mut out = Vec::with_capacity(rows.len());
            let mut i = 0usize;
            while i < rows.len() {
                out.push(i);
                i = i.saturating_add(1);
            }
            out
        }
        TextFilter::Differences => diff_indices(rows),
        TextFilter::Same => {
            let mut out = Vec::new();
            let mut i = 0usize;
            for row in rows {
                if !is_diff_row(row) {
                    out.push(i);
                }
                i = i.saturating_add(1);
            }
            out
        }
        TextFilter::Context => {
            let mut keep = vec![false; rows.len()];
            let mut i = 0usize;
            while i < rows.len() {
                if rows.get(i).is_some_and(is_diff_row) {
                    let lo = i.saturating_sub(CONTEXT_RADIUS);
                    let hi = i
                        .saturating_add(CONTEXT_RADIUS)
                        .saturating_add(1)
                        .min(rows.len());
                    let mut j = lo;
                    while j < hi {
                        if let Some(flag) = keep.get_mut(j) {
                            *flag = true;
                        }
                        j = j.saturating_add(1);
                    }
                }
                i = i.saturating_add(1);
            }
            let mut out = Vec::new();
            let mut k = 0usize;
            for flag in &keep {
                if *flag {
                    out.push(k);
                }
                k = k.saturating_add(1);
            }
            out
        }
    }
}

/// Honest counts: full-model totals plus visible count under filter.
pub fn filter_counts(rows: &[TextRow], filter: TextFilter) -> FilterCounts {
    let mut diffs = 0usize;
    let mut sames = 0usize;
    for row in rows {
        if is_diff_row(row) {
            diffs = diffs.saturating_add(1);
        } else {
            sames = sames.saturating_add(1);
        }
    }
    let shown = filtered_indices(rows, filter).len();
    FilterCounts {
        total: rows.len(),
        shown,
        diffs,
        sames,
    }
}

// ----------------------------------------------------------------------------
// Text view prefs (paint-side hook, headless)
// ----------------------------------------------------------------------------

/// Paint-side view prefs for the text panes. Mirrors the `App` toggle
/// state (`word_wrap`, `show_line_numbers`, `syntax_highlight`) without
/// importing egui, so both `views::paint_text_with_prefs` and the
/// `App::view_text_for_paint` clone-hook can share one implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextViewPrefs {
    /// Cap panes at 80 chars with wrapping.
    pub word_wrap: bool,
    /// Show the line-number gutter; off means numbers are hidden and
    /// the gutter narrows to width 1.
    pub show_line_numbers: bool,
    /// Show inline diff accents; off means accent spans are cleared.
    pub syntax_highlight: bool,
}

impl Default for TextViewPrefs {
    /// Matches `App::default`: full-width panes, numbers and accents on.
    fn default() -> Self {
        Self {
            word_wrap: false,
            show_line_numbers: true,
            syntax_highlight: true,
        }
    }
}

/// Width panes paint at under `prefs`. Word wrap caps the model width
/// at 80 so long lines stay readable without horizontal scrolling.
pub fn effective_pane_chars_for(pane_chars: usize, word_wrap: bool) -> usize {
    if word_wrap {
        pane_chars.min(80)
    } else {
        pane_chars
    }
}

/// Gutter width panes paint at under `prefs`. Hiding line numbers
/// narrows the gutter to 1 so counts stay honest while chrome shrinks.
pub fn effective_gutter_chars_for(gutter_chars: usize, show_line_numbers: bool) -> usize {
    if show_line_numbers { gutter_chars } else { 1 }
}

/// Returns a paint-ready copy of text rows with `prefs` applied: line
/// numbers hidden when off, inline accents cleared when syntax
/// highlighting is off, pane width capped when word wrap is on.
/// Selection and display filter pass through untouched, so counts
/// (`filter_counts`) stay honest under any toggle combination.
pub fn view_text_with_prefs(model: &TextRows, prefs: &TextViewPrefs) -> TextRows {
    let mut out = TextRows {
        rows: model.rows.clone(),
        truncated: model.truncated,
        coarse: model.coarse,
        pane_chars: effective_pane_chars_for(model.pane_chars, prefs.word_wrap),
        gutter_chars: effective_gutter_chars_for(model.gutter_chars, prefs.show_line_numbers),
        selected: model.selected,
        filter: model.filter,
    };
    if !prefs.show_line_numbers {
        for row in &mut out.rows {
            row.left_no = None;
            row.right_no = None;
        }
    }
    if !prefs.syntax_highlight {
        for row in &mut out.rows {
            row.accent_a.clear();
            row.accent_b.clear();
        }
    }
    out
}

// ----------------------------------------------------------------------------
// Folder
// ----------------------------------------------------------------------------

/// One row of the folder listing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderRow {
    /// Path relative to the compared roots, '/'-separated.
    pub rel_path: String,
    /// Whether the node is a directory.
    pub is_dir: bool,
    /// Status glyph, like the CLI's: `= ! > < L R`.
    pub glyph: char,
    /// Left size in bytes, formatted; `None` when absent on the left.
    pub left_size: Option<String>,
    /// Right size in bytes, formatted; `None` when absent on the right.
    pub right_size: Option<String>,
    /// Left modified time as "YYYY-MM-DD HH:MM" UTC; `None` when absent.
    pub left_time: Option<String>,
    /// Right modified time; `None` when absent.
    pub right_time: Option<String>,
    /// Whether the status differs from `Same`.
    pub changed: bool,
}

/// Builds one row per entry plus the one-line summary (with the
/// depth-limit note appended when the walk was truncated).
pub fn build_folder_rows(cmp: &FolderComparison, truncated: bool) -> (Vec<FolderRow>, String) {
    let mut rows = Vec::new();
    let mut same = 0usize;
    let mut different = 0usize;
    let mut newer_left = 0usize;
    let mut newer_right = 0usize;
    let mut left_only = 0usize;
    let mut right_only = 0usize;
    for entry in &cmp.entries {
        let (glyph, changed) = match entry.status {
            FolderStatus::Same => ('=', false),
            FolderStatus::Different => ('!', true),
            FolderStatus::NewerLeft => ('>', true),
            FolderStatus::NewerRight => ('<', true),
            FolderStatus::LeftOnly => ('L', true),
            FolderStatus::RightOnly => ('R', true),
        };
        match entry.status {
            FolderStatus::Same => same = same.saturating_add(1),
            FolderStatus::Different => different = different.saturating_add(1),
            FolderStatus::NewerLeft => newer_left = newer_left.saturating_add(1),
            FolderStatus::NewerRight => newer_right = newer_right.saturating_add(1),
            FolderStatus::LeftOnly => left_only = left_only.saturating_add(1),
            FolderStatus::RightOnly => right_only = right_only.saturating_add(1),
        }
        rows.push(FolderRow {
            rel_path: entry.rel_path.clone(),
            is_dir: entry.is_dir,
            glyph,
            left_size: entry.left.map(|n| n.size.to_string()),
            right_size: entry.right.map(|n| n.size.to_string()),
            left_time: entry.left.map(|n| format_time(n.modified)),
            right_time: entry.right.map(|n| format_time(n.modified)),
            changed,
        });
    }
    let total = cmp.entries.len();
    let mut summary = format!(
        "{total} entries: {same} same, {different} different, \
         {newer_left} newer left, {newer_right} newer right, \
         {left_only} left-only, {right_only} right-only"
    );
    if truncated {
        summary
            .push_str("\nnote: the walk stopped at the depth limit; deeper entries are not listed");
    }
    (rows, summary)
}

// ----------------------------------------------------------------------------
// Picture
// ----------------------------------------------------------------------------

/// Summary lines for a picture comparison, mirroring the CLI's
/// renderer. One deviation: when the overlap is empty the differing
/// share is reported as 0.00% instead of the NaN the naive division
/// would produce.
pub fn build_picture_summary(d: &PictureDiff) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("left:  {}x{}", d.width_l, d.height_l));
    lines.push(format!("right: {}x{}", d.width_r, d.height_r));
    if d.identical {
        lines.push("no differences".to_string());
        return lines;
    }
    lines.push(format!(
        "overlap: {}x{} ({} pixels)",
        d.overlap_w, d.overlap_h, d.total
    ));
    let pct = if d.total == 0 {
        0.0
    } else {
        d.differing as f64 / d.total as f64 * 100.0
    };
    lines.push(format!("differing pixels: {} ({pct:.2}%)", d.differing));
    if let Some((x0, y0, x1, y1)) = d.bbox {
        lines.push(format!("difference bounds: ({x0},{y0})..({x1},{y1})"));
    }
    if d.width_l != d.width_r || d.height_l != d.height_r {
        lines.push("note: image sizes differ; only the overlap was compared".to_string());
    }
    lines
}

/// Raw non-premultiplied RGBA bytes of a heatmap, `(bytes, width,
/// height)`. An empty `Vec<u8>` means "nothing to display": either
/// the image is empty or the pixel buffer disagrees with the declared
/// dimensions (only possible in hand-built images).
pub fn build_heatmap_rgba(heatmap: &RgbaImage) -> (Vec<u8>, u32, u32) {
    let pixel_count = u64::from(heatmap.width).saturating_mul(u64::from(heatmap.height));
    let byte_count = pixel_count.saturating_mul(4);
    let byte_count = usize::try_from(byte_count).unwrap_or(usize::MAX);
    if heatmap.pixels.len() != byte_count {
        return (Vec::new(), heatmap.width, heatmap.height);
    }
    (heatmap.pixels.clone(), heatmap.width, heatmap.height)
}

// ----------------------------------------------------------------------------
// Hex
// ----------------------------------------------------------------------------

/// Summary lines for a byte-level comparison, mirroring the CLI's
/// `render_hex.rs` wording without color codes.
pub fn build_hex_summary(cmp: &BinaryComparison) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("left:  {} bytes", cmp.left_size));
    lines.push(format!("right: {} bytes", cmp.right_size));
    match cmp.first_diff {
        None => {
            lines.push("no differences".to_string());
        }
        Some(off) => {
            lines.push(format!("first difference at offset {off:#010x}"));
            lines.push(format!("{} differing run(s)", cmp.hunks.len()));
            for hunk in &cmp.hunks {
                lines.push(format!(
                    "{:#010x}: {} vs {} byte(s)",
                    hunk.offset,
                    hunk.left.len(),
                    hunk.right.len()
                ));
            }
        }
    }
    if cmp.truncated {
        lines.push("note: more differing runs exist (hunk limit reached)".to_string());
    }
    lines
}

// ----------------------------------------------------------------------------
// Text three-way merge
// ----------------------------------------------------------------------------

/// The merged-text model: capped output lines plus conflict status.
#[derive(Debug, Clone, Default)]
pub struct MergeSummary {
    /// Merged output lines, at most [`MAX_ROWS`].
    pub lines: Vec<String>,
    /// Whether any line opens a conflict marker.
    pub conflicted: bool,
    /// Whether lines beyond the cap were cut off.
    pub truncated: bool,
}

/// Builds the merge summary from already-merged output lines (the
/// engine's `merge3` result). Conflict detection is the same rule the
/// CLI uses: a line starting with `<<<<<<<`.
pub fn build_merge_summary(merged: &[String]) -> MergeSummary {
    let mut lines = Vec::new();
    let mut conflicted = false;
    let mut truncated = false;
    for line in merged {
        if lines.len() >= MAX_ROWS {
            truncated = true;
            break;
        }
        if line.starts_with("<<<<<<<") {
            conflicted = true;
        }
        lines.push(line.clone());
    }
    MergeSummary {
        lines,
        conflicted,
        truncated,
    }
}

// ----------------------------------------------------------------------------
// Table
// ----------------------------------------------------------------------------

/// One painted row of a table comparison.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableRow {
    /// Status glyph: `=` equal, `!` changed, `L` left-only, `R` right-only.
    pub glyph: char,
    /// Row label (` hdr` or a right-aligned 1-based number).
    pub label: String,
    /// Cell text joined with ` | `; changed cells read `[old => new]`.
    pub text: String,
    /// Whether the row differs.
    pub changed: bool,
}

/// The table model: rows plus a truncation flag.
#[derive(Debug, Clone, Default)]
pub struct TableModel {
    /// Painted rows, at most [`MAX_ROWS`].
    pub rows: Vec<TableRow>,
    /// Whether rows beyond the cap were cut off.
    pub truncated: bool,
}

fn table_row_label(n: usize) -> String {
    format!("{n:>4}")
}

fn table_cells(left: &[String], right: &[String]) -> String {
    let changed = diff_cells(left, right);
    let mut cells = Vec::new();
    let mut idx = 0usize;
    loop {
        if left.get(idx).is_none() && right.get(idx).is_none() {
            break;
        }
        let l = left.get(idx).cloned().unwrap_or_default();
        let r = right.get(idx).cloned().unwrap_or_default();
        if changed.contains(&idx) {
            cells.push(format!("[{l} => {r}]"));
        } else {
            cells.push(r);
        }
        idx = idx.saturating_add(1);
        if idx > 10_000 {
            break;
        }
    }
    cells.join(" | ")
}

fn table_plain(cells: &[String]) -> String {
    cells.join(" | ")
}

/// Builds table rows mirroring the CLI walk: the header once, then one
/// row per data row with delete+insert runs paired index-wise.
pub fn build_table_rows(cmp: &TableComparison) -> TableModel {
    let mut rows: Vec<TableRow> = Vec::new();
    let mut truncated = false;
    let mut push = |row: TableRow, truncated: &mut bool| {
        if rows.len() >= MAX_ROWS {
            *truncated = true;
        } else {
            rows.push(row);
        }
    };
    if !cmp.header_l.is_empty() || !cmp.header_r.is_empty() {
        if cmp.header_l == cmp.header_r {
            push(
                TableRow {
                    glyph: '=',
                    label: " hdr".to_string(),
                    text: table_plain(&cmp.header_r),
                    changed: false,
                },
                &mut truncated,
            );
        } else {
            push(
                TableRow {
                    glyph: '!',
                    label: " hdr".to_string(),
                    text: table_cells(&cmp.header_l, &cmp.header_r),
                    changed: true,
                },
                &mut truncated,
            );
        }
    }
    let mut lrow = 0usize;
    let mut rrow = 0usize;
    let mut ops = cmp.script.ops.iter().cloned().peekable();
    while let Some(op) = ops.next() {
        if truncated {
            break;
        }
        match op {
            Op::Equal { len } => {
                for _ in 0..len {
                    if truncated {
                        break;
                    }
                    let cells = cmp.rows_r.get(rrow).cloned().unwrap_or_default();
                    let label = table_row_label(rrow.saturating_add(1));
                    lrow = lrow.saturating_add(1);
                    rrow = rrow.saturating_add(1);
                    push(
                        TableRow {
                            glyph: '=',
                            label,
                            text: table_plain(&cells),
                            changed: false,
                        },
                        &mut truncated,
                    );
                }
            }
            Op::Delete { len } => {
                let lend = lrow.saturating_add(len).min(cmp.rows_l.len());
                let mut left_rows = Vec::new();
                let mut i = lrow;
                while i < lend {
                    if let Some(r) = cmp.rows_l.get(i) {
                        left_rows.push(r.clone());
                    }
                    i = i.saturating_add(1);
                }
                let lbase = lrow;
                lrow = lrow.saturating_add(len);
                if let Some(Op::Insert { len: ins }) = ops.peek().cloned() {
                    ops.next();
                    let rend = rrow.saturating_add(ins).min(cmp.rows_r.len());
                    let mut right_rows = Vec::new();
                    let mut j = rrow;
                    while j < rend {
                        if let Some(r) = cmp.rows_r.get(j) {
                            right_rows.push(r.clone());
                        }
                        j = j.saturating_add(1);
                    }
                    let rbase = rrow;
                    rrow = rrow.saturating_add(ins);
                    let pairs = left_rows.len().min(right_rows.len());
                    let mut k = 0usize;
                    while k < pairs {
                        if truncated {
                            break;
                        }
                        let l = left_rows.get(k).cloned().unwrap_or_default();
                        let r = right_rows.get(k).cloned().unwrap_or_default();
                        let label = table_row_label(rbase.saturating_add(k).saturating_add(1));
                        push(
                            TableRow {
                                glyph: '!',
                                label,
                                text: table_cells(&l, &r),
                                changed: true,
                            },
                            &mut truncated,
                        );
                        k = k.saturating_add(1);
                    }
                    let mut e = pairs;
                    while e < left_rows.len() {
                        if truncated {
                            break;
                        }
                        let cells = left_rows.get(e).cloned().unwrap_or_default();
                        let label = table_row_label(lbase.saturating_add(e).saturating_add(1));
                        push(
                            TableRow {
                                glyph: 'L',
                                label,
                                text: table_plain(&cells),
                                changed: true,
                            },
                            &mut truncated,
                        );
                        e = e.saturating_add(1);
                    }
                    let mut e = pairs;
                    while e < right_rows.len() {
                        if truncated {
                            break;
                        }
                        let cells = right_rows.get(e).cloned().unwrap_or_default();
                        let label = table_row_label(rbase.saturating_add(e).saturating_add(1));
                        push(
                            TableRow {
                                glyph: 'R',
                                label,
                                text: table_plain(&cells),
                                changed: true,
                            },
                            &mut truncated,
                        );
                        e = e.saturating_add(1);
                    }
                } else {
                    let mut k = 0usize;
                    while k < left_rows.len() {
                        if truncated {
                            break;
                        }
                        let cells = left_rows.get(k).cloned().unwrap_or_default();
                        let label = table_row_label(lbase.saturating_add(k).saturating_add(1));
                        push(
                            TableRow {
                                glyph: 'L',
                                label,
                                text: table_plain(&cells),
                                changed: true,
                            },
                            &mut truncated,
                        );
                        k = k.saturating_add(1);
                    }
                }
            }
            Op::Insert { len } => {
                let rend = rrow.saturating_add(len).min(cmp.rows_r.len());
                let mut right_rows = Vec::new();
                let mut j = rrow;
                while j < rend {
                    if let Some(r) = cmp.rows_r.get(j) {
                        right_rows.push(r.clone());
                    }
                    j = j.saturating_add(1);
                }
                let rbase = rrow;
                rrow = rrow.saturating_add(len);
                let mut k = 0usize;
                while k < right_rows.len() {
                    if truncated {
                        break;
                    }
                    let cells = right_rows.get(k).cloned().unwrap_or_default();
                    let label = table_row_label(rbase.saturating_add(k).saturating_add(1));
                    push(
                        TableRow {
                            glyph: 'R',
                            label,
                            text: table_plain(&cells),
                            changed: true,
                        },
                        &mut truncated,
                    );
                    k = k.saturating_add(1);
                }
            }
        }
    }
    TableModel { rows, truncated }
}

// ----------------------------------------------------------------------------
// Folder sync (dry-run plan view)
// ----------------------------------------------------------------------------

/// Summary lines for a sync plan, mirroring the CLI's `print_plan`
/// wording without color codes. The GUI shows the dry-run plan only;
/// applying stays a CLI operation.
pub fn build_sync_summary(plan: &SyncPlan) -> Vec<String> {
    let mut lines = Vec::new();
    if plan.truncated {
        lines.push(
            "note: the walk stopped at the depth limit; deeper entries are not planned".to_string(),
        );
    }
    if plan.steps.is_empty() && plan.skipped.is_empty() {
        lines.push("nothing to do: the trees already match the plan".to_string());
        return lines;
    }
    for step in &plan.steps {
        let glyph = match step.action {
            SyncAction::CopyToRight => ">",
            SyncAction::CopyToLeft => "<",
            SyncAction::DeleteRight | SyncAction::DeleteLeft => "x",
        };
        let dir_mark = if step.is_dir { "/" } else { "" };
        lines.push(format!("{glyph} {}{dir_mark}", step.rel_path));
    }
    for (path, reason) in &plan.skipped {
        lines.push(format!("~ {path} ({reason})"));
    }
    lines.push(format!(
        "{} step(s) planned, {} skipped",
        plan.steps.len(),
        plan.skipped.len()
    ));
    lines
}

// ----------------------------------------------------------------------------
// Folder merge (dry-run plan view)
// ----------------------------------------------------------------------------

/// Summary lines for a three-way folder-merge plan, mirroring the
/// CLI's `print_plan` wording without color codes.
pub fn build_folder_merge_summary(plan: &TreeMergePlan) -> Vec<String> {
    let mut lines = Vec::new();
    if plan.steps.is_empty() && plan.skipped.is_empty() {
        lines.push("nothing to do: the merge plan is empty".to_string());
        return lines;
    }
    for step in &plan.steps {
        let glyph = match step.action {
            TreeMergeAction::TakeLeft => "L",
            TreeMergeAction::TakeRight => "R",
            TreeMergeAction::MergedText => "M",
            TreeMergeAction::Conflict => "C",
            TreeMergeAction::MakeDir => "+",
        };
        let dir_mark = if matches!(step.action, TreeMergeAction::MakeDir) {
            "/"
        } else {
            ""
        };
        lines.push(format!("{glyph} {}{dir_mark}", step.rel_path));
    }
    for (path, reason) in &plan.skipped {
        lines.push(format!("~ {path} ({reason})"));
    }
    for path in &plan.conflicts {
        lines.push(format!("! {path} (conflict)"));
    }
    lines.push(format!(
        "{} step(s) planned, {} omitted, {} conflict(s)",
        plan.steps.len(),
        plan.skipped.len(),
        plan.conflicts.len()
    ));
    lines
}

// ----------------------------------------------------------------------------
// Audio (media compare)
// ----------------------------------------------------------------------------

/// Summary lines for an audio comparison, mirroring the CLI's
/// `render_audio.rs` wording without color codes.
pub fn build_audio_summary(d: &AudioDiff) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "left:  {:.3}s, {} Hz, {} channel(s)",
        d.duration_l, d.sample_rate_l, d.channels_l
    ));
    lines.push(format!(
        "right: {:.3}s, {} Hz, {} channel(s)",
        d.duration_r, d.sample_rate_r, d.channels_r
    ));
    if d.config_mismatch {
        lines.push(
            "note: stream configurations differ; sample comparison is not meaningful and was skipped"
                .to_string(),
        );
    } else if d.identical {
        lines.push("no differences".to_string());
    } else {
        match d.first_diff_at {
            Some(at) => lines.push(format!("first difference at {at:.3}s")),
            None => lines.push("first difference: -".to_string()),
        }
        let pct = if d.windows_total > 0 {
            d.windows_differing as f64 / d.windows_total as f64 * 100.0
        } else {
            0.0
        };
        lines.push(format!(
            "differing 10ms windows: {}/{} ({pct:.2}%)",
            d.windows_differing, d.windows_total
        ));
    }
    lines
}

// ----------------------------------------------------------------------------
// Civil date
// ----------------------------------------------------------------------------

/// Formats a timestamp as "YYYY-MM-DD HH:MM" in UTC, on the Unix
/// epoch base (1970-01-01T00:00Z; `UNIX_EPOCH` formats as
/// "1970-01-01 00:00"). Times before the epoch render as "-".
/// A compact reimplementation of the CLI's helper: pure integer
/// math, no calendar crates.
pub fn format_time(t: SystemTime) -> String {
    let secs = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => return "-".to_string(),
    };
    let days = secs / 86_400;
    let tod = secs % 86_400;
    // u64 days always fit in i64; the downstream math is total.
    let days = i64::try_from(days).unwrap_or(i64::MAX);
    let (y, m, d) = civil_from_days(days);
    let hh = tod / 3_600;
    let mm = (tod % 3_600) / 60;
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// Days since 1970-01-01 to a civil date: Howard Hinnant's
/// `civil_from_days` algorithm, valid for any representable day count.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use autocompare_compare::{FolderEntry, NodeInfo};
    use autocompare_diff::{DiffScript, RgbaImage, picture_heatmap};

    use super::*;

    fn cmp_text(left: &[&str], right: &[&str], ops: Vec<Op>, exact: bool) -> TextComparison {
        TextComparison {
            left_lines: left.iter().map(|s| s.to_string()).collect(),
            right_lines: right.iter().map(|s| s.to_string()).collect(),
            script: DiffScript { ops, exact },
        }
    }

    // ---- build_text_rows ----

    #[test]
    fn identical_files_model_all_equal_rows_with_paired_numbers() {
        let cmp = cmp_text(
            &["alpha", "beta", "gamma"],
            &["alpha", "beta", "gamma"],
            vec![Op::Equal { len: 3 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 3);
        for (i, row) in model.rows.iter().enumerate() {
            assert_eq!(row.kind, TextRowKind::Equal);
            assert_eq!(row.left_no, Some(i + 1));
            assert_eq!(row.right_no, Some(i + 1));
            assert!(row.accent_a.is_empty());
            assert!(row.accent_b.is_empty());
            assert_eq!(row.left, row.right);
        }
        assert!(!model.truncated);
        assert!(!model.coarse);
        assert_eq!(model.pane_chars, 5); // "gamma"
        assert_eq!(model.gutter_chars, 4); // minimum width
    }

    #[test]
    fn delete_then_insert_pairs_into_changed_rows_with_in_bounds_accents() {
        let cmp = cmp_text(
            &["the quick brown fox"],
            &["the quick red fox"],
            vec![Op::Delete { len: 1 }, Op::Insert { len: 1 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 1);
        let row = model.rows.first().cloned().unwrap_or_default();
        assert_eq!(row.kind, TextRowKind::Changed);
        assert_eq!(row.left_no, Some(1));
        assert_eq!(row.right_no, Some(1));
        assert_eq!(row.left, "the quick brown fox");
        assert_eq!(row.right, "the quick red fox");
        // inline_diff covers the replaced words exactly: "brown" is
        // characters 10..15 on the left, "red" is 10..13 on the right.
        assert_eq!(row.accent_a, vec![(10, 15)]);
        assert_eq!(row.accent_b, vec![(10, 13)]);
        // In bounds: every range is half-open within the char count.
        for &(s, e) in row.accent_a.iter().chain(row.accent_b.iter()) {
            assert!(s <= e);
            assert!(e <= row.left.chars().count().max(row.right.chars().count()));
        }
    }

    #[test]
    fn pure_deletion_makes_deleted_rows_with_left_numbers_only() {
        let cmp = cmp_text(
            &["a", "b", "c"],
            &["c"],
            vec![Op::Delete { len: 2 }, Op::Equal { len: 1 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 3);
        let first = model.rows.first().cloned().unwrap_or_default();
        assert_eq!(first.kind, TextRowKind::Deleted);
        assert_eq!(first.left_no, Some(1));
        assert_eq!(first.right_no, None);
        assert_eq!(first.left, "a");
        let second = model.rows.get(1).cloned().unwrap_or_default();
        assert_eq!(second.kind, TextRowKind::Deleted);
        assert_eq!(second.left_no, Some(2));
        let third = model.rows.get(2).cloned().unwrap_or_default();
        assert_eq!(third.kind, TextRowKind::Equal);
        assert_eq!(third.left_no, Some(3));
        assert_eq!(third.right_no, Some(1));
    }

    #[test]
    fn changed_run_leftovers_keep_their_own_kind_and_numbers() {
        // Delete 2 + Insert 1: one changed pair, one leftover delete.
        let cmp = cmp_text(
            &["x1", "y2"],
            &["z3"],
            vec![Op::Delete { len: 2 }, Op::Insert { len: 1 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 2);
        let changed = model.rows.first().cloned().unwrap_or_default();
        assert_eq!(changed.kind, TextRowKind::Changed);
        assert_eq!(changed.left_no, Some(1));
        assert_eq!(changed.right_no, Some(1));
        assert_eq!(changed.left, "x1");
        assert_eq!(changed.right, "z3");
        let leftover = model.rows.get(1).cloned().unwrap_or_default();
        assert_eq!(leftover.kind, TextRowKind::Deleted);
        assert_eq!(leftover.left_no, Some(2));
        assert_eq!(leftover.right_no, None);
        assert_eq!(leftover.left, "y2");
    }

    #[test]
    fn lone_insert_run_makes_inserted_rows_with_right_numbers_only() {
        let cmp = cmp_text(
            &["a"],
            &["a", "new"],
            vec![Op::Equal { len: 1 }, Op::Insert { len: 1 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 2);
        let ins = model.rows.get(1).cloned().unwrap_or_default();
        assert_eq!(ins.kind, TextRowKind::Inserted);
        assert_eq!(ins.left_no, None);
        assert_eq!(ins.right_no, Some(2));
        assert_eq!(ins.right, "new");
    }

    #[test]
    fn row_cap_stops_the_model_and_notes_truncation() {
        let n = MAX_ROWS + 2;
        let lines: Vec<String> = (0..n).map(|i| format!("line {i}")).collect();
        let cmp = TextComparison {
            left_lines: lines.clone(),
            right_lines: lines,
            script: DiffScript {
                ops: vec![Op::Equal { len: n }],
                exact: true,
            },
        };
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), MAX_ROWS);
        assert!(model.truncated);
    }

    #[test]
    fn coarse_script_sets_the_coarse_flag() {
        let cmp = cmp_text(&["a"], &["b"], vec![Op::Equal { len: 1 }], false);
        let model = build_text_rows(&cmp);
        assert!(model.coarse);
    }

    #[test]
    fn empty_script_models_nothing() {
        let cmp = cmp_text(&[], &[], vec![], true);
        let model = build_text_rows(&cmp);
        assert!(model.rows.is_empty());
        assert!(!model.truncated);
        assert_eq!(model.pane_chars, 1); // floor
    }

    // ---- build_folder_rows ----

    fn node(size: u64, secs: u64) -> NodeInfo {
        NodeInfo {
            size,
            modified: UNIX_EPOCH + Duration::from_secs(secs),
        }
    }

    #[test]
    fn folder_rows_cover_every_status_with_glyphs_and_counts() {
        let entries = vec![
            FolderEntry {
                rel_path: "same.txt".to_string(),
                is_dir: false,
                left: Some(node(10, 0)),
                right: Some(node(10, 0)),
                status: FolderStatus::Same,
            },
            FolderEntry {
                rel_path: "different.txt".to_string(),
                is_dir: false,
                left: Some(node(10, 0)),
                right: Some(node(20, 0)),
                status: FolderStatus::Different,
            },
            FolderEntry {
                rel_path: "newer_left.txt".to_string(),
                is_dir: false,
                left: Some(node(10, 100)),
                right: Some(node(10, 0)),
                status: FolderStatus::NewerLeft,
            },
            FolderEntry {
                rel_path: "newer_right.txt".to_string(),
                is_dir: false,
                left: Some(node(10, 0)),
                right: Some(node(10, 100)),
                status: FolderStatus::NewerRight,
            },
            FolderEntry {
                rel_path: "left_only/".to_string(),
                is_dir: true,
                left: Some(node(0, 0)),
                right: None,
                status: FolderStatus::LeftOnly,
            },
            FolderEntry {
                rel_path: "right_only.bin".to_string(),
                is_dir: false,
                left: None,
                right: Some(node(7, 0)),
                status: FolderStatus::RightOnly,
            },
        ];
        let cmp = FolderComparison {
            entries,
            truncated: false,
        };
        let (rows, summary) = build_folder_rows(&cmp, false);
        assert_eq!(rows.len(), 6);

        let same = rows.first().cloned().unwrap_or_default();
        assert_eq!(same.glyph, '=');
        assert!(!same.changed);
        assert_eq!(same.left_size, Some("10".to_string()));
        assert_eq!(same.right_size, Some("10".to_string()));
        assert_eq!(same.left_time, Some("1970-01-01 00:00".to_string()));

        let different = rows.get(1).cloned().unwrap_or_default();
        assert_eq!(different.glyph, '!');
        assert!(different.changed);

        let newer_left = rows.get(2).cloned().unwrap_or_default();
        assert_eq!(newer_left.glyph, '>');
        assert!(newer_left.changed);

        let newer_right = rows.get(3).cloned().unwrap_or_default();
        assert_eq!(newer_right.glyph, '<');

        let left_only = rows.get(4).cloned().unwrap_or_default();
        assert_eq!(left_only.glyph, 'L');
        assert!(left_only.is_dir);
        assert_eq!(left_only.right_size, None);
        assert_eq!(left_only.right_time, None);
        assert_eq!(left_only.left_size, Some("0".to_string()));

        let right_only = rows.get(5).cloned().unwrap_or_default();
        assert_eq!(right_only.glyph, 'R');
        assert_eq!(right_only.left_size, None);
        assert_eq!(right_only.right_size, Some("7".to_string()));

        assert_eq!(
            summary,
            "6 entries: 1 same, 1 different, 1 newer left, \
             1 newer right, 1 left-only, 1 right-only"
        );
    }

    #[test]
    fn folder_truncation_appends_the_note_to_the_summary() {
        let cmp = FolderComparison {
            entries: Vec::new(),
            truncated: true,
        };
        let (_, summary) = build_folder_rows(&cmp, true);
        assert!(summary.contains("depth limit"));
        assert!(summary.contains("0 entries: 0 same"));
    }

    // ---- build_picture_summary ----

    fn diff(
        (width_l, height_l): (u32, u32),
        (width_r, height_r): (u32, u32),
        differing: u64,
        total: u64,
        bbox: Option<(u32, u32, u32, u32)>,
        identical: bool,
    ) -> PictureDiff {
        PictureDiff {
            width_l,
            height_l,
            width_r,
            height_r,
            overlap_w: width_l.min(width_r),
            overlap_h: height_l.min(height_r),
            differing,
            total,
            bbox,
            identical,
        }
    }

    #[test]
    fn picture_summary_matches_the_cli_wording_exactly() {
        let d = diff((2, 2), (3, 3), 1, 4, Some((1, 0, 1, 0)), false);
        assert_eq!(
            build_picture_summary(&d),
            vec![
                "left:  2x2".to_string(),
                "right: 3x3".to_string(),
                "overlap: 2x2 (4 pixels)".to_string(),
                "differing pixels: 1 (25.00%)".to_string(),
                "difference bounds: (1,0)..(1,0)".to_string(),
                "note: image sizes differ; only the overlap was compared".to_string(),
            ]
        );
    }

    #[test]
    fn identical_pictures_summarize_without_overlap_lines() {
        let d = diff((1, 1), (1, 1), 0, 1, None, true);
        assert_eq!(
            build_picture_summary(&d),
            vec![
                "left:  1x1".to_string(),
                "right: 1x1".to_string(),
                "no differences".to_string(),
            ]
        );
    }

    #[test]
    fn empty_overlap_reports_zero_percent_not_nan() {
        let d = diff((0, 0), (1, 1), 0, 0, None, false);
        let lines = build_picture_summary(&d);
        assert!(lines.contains(&"differing pixels: 0 (0.00%)".to_string()));
    }

    // ---- build_heatmap_rgba ----

    fn image(width: u32, height: u32, pixels: Vec<u8>) -> RgbaImage {
        RgbaImage {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn heatmap_of_differing_single_pixel_is_red() {
        let a = image(1, 1, vec![0, 0, 0, 255]);
        let b = image(1, 1, vec![255, 255, 255, 255]);
        let heatmap = picture_heatmap(&a, &b);
        let (bytes, w, h) = build_heatmap_rgba(&heatmap);
        assert_eq!(bytes, vec![255, 0, 0, 255]);
        assert_eq!((w, h), (1, 1));
    }

    #[test]
    fn heatmap_of_equal_single_pixel_is_dimmed_grayscale() {
        let a = image(1, 1, vec![10, 10, 10, 255]);
        let heatmap = picture_heatmap(&a, &a.clone());
        let (bytes, w, h) = build_heatmap_rgba(&heatmap);
        // luma 10 dimmed to 30% rounds to 3; alpha stays opaque.
        assert_eq!(bytes, vec![3, 3, 3, 255]);
        assert_eq!((w, h), (1, 1));
    }

    #[test]
    fn empty_heatmap_models_empty_bytes_without_panicking() {
        let (bytes, w, h) = build_heatmap_rgba(&image(0, 0, Vec::new()));
        assert!(bytes.is_empty());
        assert_eq!((w, h), (0, 0));
    }

    #[test]
    fn inconsistent_heatmap_models_empty_bytes() {
        // Four bytes declared as 2x1: the guard rejects the mismatch.
        let (bytes, w, h) = build_heatmap_rgba(&image(2, 1, vec![255, 0, 0, 255]));
        assert!(bytes.is_empty());
        assert_eq!((w, h), (2, 1));
    }

    // ---- format_time ----

    #[test]
    fn unix_epoch_formats_as_1970_01_01_00_00() {
        assert_eq!(format_time(UNIX_EPOCH), "1970-01-01 00:00");
    }

    #[test]
    fn known_timestamps_format_exactly_in_utc() {
        // 2026-01-01T00:00Z = 1767225600 (2025-01-01 is 1735689600,
        // plus the 365 days of the non-leap year 2025).
        let t = UNIX_EPOCH + Duration::from_secs(1_767_225_600);
        assert_eq!(format_time(t), "2026-01-01 00:00");
        // 01:02 the same day.
        let t = UNIX_EPOCH + Duration::from_secs(1_767_225_600 + 3_720);
        assert_eq!(format_time(t), "2026-01-01 01:02");
        // Leap day: 2024-02-29T00:00Z = 1709164800 (2024-01-01 is
        // 1704067200 plus 59 days).
        let t = UNIX_EPOCH + Duration::from_secs(1_709_164_800);
        assert_eq!(format_time(t), "2024-02-29 00:00");
    }

    #[test]
    fn pre_epoch_time_formats_as_a_dash() {
        let t = UNIX_EPOCH
            .checked_sub(Duration::from_secs(1))
            .unwrap_or(UNIX_EPOCH);
        assert_eq!(format_time(t), "-");
    }

    // ---- hex ----

    #[test]
    fn hex_identical_models_no_differences() {
        use autocompare_compare::BinaryComparison;
        let cmp = BinaryComparison {
            left_size: 13,
            right_size: 13,
            first_diff: None,
            hunks: Vec::new(),
            truncated: false,
        };
        let lines = build_hex_summary(&cmp);
        assert!(lines.iter().any(|l| l.contains("no differences")));
        assert!(lines.iter().any(|l| l.contains("13 bytes")));
    }

    #[test]
    fn hex_difference_models_offset_and_run() {
        use autocompare_compare::{BinaryComparison, BinaryHunk};
        let cmp = BinaryComparison {
            left_size: 13,
            right_size: 13,
            first_diff: Some(12),
            hunks: vec![BinaryHunk {
                offset: 12,
                left: vec![0x41],
                right: vec![0x42],
            }],
            truncated: true,
        };
        let lines = build_hex_summary(&cmp);
        assert!(lines.iter().any(|l| l.contains("first difference at")));
        assert!(lines.iter().any(|l| l.contains("1 differing run(s)")));
        assert!(lines.iter().any(|l| l.contains("hunk limit")));
    }

    // ---- merge ----

    #[test]
    fn merge_clean_models_lines_without_conflict() {
        let merged = vec!["alpha".to_string(), "bravo".to_string()];
        let summary = build_merge_summary(&merged);
        assert!(!summary.conflicted);
        assert!(!summary.truncated);
        assert_eq!(summary.lines.len(), 2);
    }

    #[test]
    fn merge_conflict_markers_flag_conflicted() {
        let merged = vec![
            "alpha".to_string(),
            "<<<<<<< left".to_string(),
            "bravo".to_string(),
            "=======".to_string(),
            "BRAVO".to_string(),
            ">>>>>>> right".to_string(),
        ];
        let summary = build_merge_summary(&merged);
        assert!(summary.conflicted);
        assert_eq!(summary.lines.len(), 6);
    }

    // ---- table ----

    #[test]
    fn table_equal_rows_model_plain() {
        use autocompare_compare::TableComparison;
        use autocompare_diff::DiffScript;
        let cmp = TableComparison {
            header_l: vec!["a".to_string()],
            header_r: vec!["a".to_string()],
            rows_l: vec![vec!["x".to_string()]],
            rows_r: vec![vec!["x".to_string()]],
            script: DiffScript {
                ops: vec![Op::Equal { len: 1 }],
                exact: true,
            },
        };
        let model = build_table_rows(&cmp);
        assert!(!model.truncated);
        assert!(model.rows.iter().any(|r| r.glyph == '=' && !r.changed));
    }

    #[test]
    fn table_changed_cells_show_old_new() {
        use autocompare_compare::TableComparison;
        use autocompare_diff::DiffScript;
        let cmp = TableComparison {
            header_l: vec!["h".to_string()],
            header_r: vec!["h".to_string()],
            rows_l: vec![vec!["20".to_string()]],
            rows_r: vec![vec!["25".to_string()]],
            script: DiffScript {
                ops: vec![Op::Delete { len: 1 }, Op::Insert { len: 1 }],
                exact: true,
            },
        };
        let model = build_table_rows(&cmp);
        let changed = model
            .rows
            .iter()
            .find(|r| r.glyph == '!')
            .cloned()
            .unwrap_or_default();
        assert!(changed.changed);
        assert!(changed.text.contains("[20 => 25]"));
    }

    // ---- sync ----

    #[test]
    fn sync_empty_plan_models_nothing_to_do() {
        use autocompare_compare::SyncPlan;
        let plan = SyncPlan::default();
        let lines = build_sync_summary(&plan);
        assert!(lines.iter().any(|l| l.contains("nothing to do")));
    }

    #[test]
    fn sync_plan_models_steps_skips_and_counts() {
        use autocompare_compare::{SyncAction, SyncPlan, SyncStep};
        let plan = SyncPlan {
            steps: vec![SyncStep {
                rel_path: "a.txt".to_string(),
                is_dir: false,
                action: SyncAction::CopyToRight,
            }],
            skipped: vec![("b.txt".to_string(), "destination is newer".to_string())],
            truncated: true,
        };
        let lines = build_sync_summary(&plan);
        assert!(lines.iter().any(|l| l.contains("depth limit")));
        assert!(lines.iter().any(|l| l.contains("a.txt")));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("1 step(s) planned, 1 skipped"))
        );
    }

    // ---- folder merge ----

    #[test]
    fn folder_merge_empty_plan_models_nothing_to_do() {
        use autocompare_compare::TreeMergePlan;
        let plan = TreeMergePlan::default();
        let lines = build_folder_merge_summary(&plan);
        assert!(lines.iter().any(|l| l.contains("merge plan is empty")));
    }

    #[test]
    fn folder_merge_conflict_models_markers_and_counts() {
        use autocompare_compare::{TreeMergeAction, TreeMergePlan, TreeMergeStep};
        let plan = TreeMergePlan {
            steps: vec![TreeMergeStep {
                rel_path: "m.txt".to_string(),
                action: TreeMergeAction::Conflict,
                merged: None,
            }],
            conflicts: vec!["m.txt".to_string()],
            skipped: Vec::new(),
        };
        let lines = build_folder_merge_summary(&plan);
        assert!(lines.iter().any(|l| l.contains("C m.txt")));
        assert!(lines.iter().any(|l| l.contains("(conflict)")));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("1 step(s) planned, 0 omitted, 1 conflict(s)"))
        );
    }

    // ---- audio ----

    #[test]
    fn audio_identical_models_no_differences() {
        use autocompare_diff::AudioDiff;
        let d = AudioDiff {
            sample_rate_l: 44_100,
            sample_rate_r: 44_100,
            channels_l: 1,
            channels_r: 1,
            duration_l: 1.0,
            duration_r: 1.0,
            config_mismatch: false,
            first_diff_at: None,
            windows_total: 100,
            windows_differing: 0,
            identical: true,
        };
        let lines = build_audio_summary(&d);
        assert!(lines.iter().any(|l| l.contains("no differences")));
    }

    #[test]
    fn audio_mismatch_models_not_comparable_note() {
        use autocompare_diff::AudioDiff;
        let d = AudioDiff {
            sample_rate_l: 44_100,
            sample_rate_r: 48_000,
            channels_l: 1,
            channels_r: 1,
            duration_l: 1.0,
            duration_r: 1.0,
            config_mismatch: true,
            first_diff_at: None,
            windows_total: 0,
            windows_differing: 0,
            identical: false,
        };
        let lines = build_audio_summary(&d);
        assert!(lines.iter().any(|l| l.contains("not meaningful")));
    }

    #[test]
    fn audio_difference_models_windows_share() {
        use autocompare_diff::AudioDiff;
        let d = AudioDiff {
            sample_rate_l: 44_100,
            sample_rate_r: 44_100,
            channels_l: 2,
            channels_r: 2,
            duration_l: 1.0,
            duration_r: 1.0,
            config_mismatch: false,
            first_diff_at: Some(0.5),
            windows_total: 100,
            windows_differing: 1,
            identical: false,
        };
        let lines = build_audio_summary(&d);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("first difference at 0.500s"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("differing 10ms windows: 1/100"))
        );
    }

    // ---- text navigation / filter (P0) ----

    fn nav_rows() -> Vec<TextRow> {
        vec![
            TextRow {
                kind: TextRowKind::Equal,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: "a".to_string(),
                right: "b".to_string(),
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: "c".to_string(),
                right: "d".to_string(),
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Equal,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Deleted,
                left: "x".to_string(),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn next_prev_diff_cycle_with_wrap() {
        let rows = nav_rows();
        assert_eq!(next_diff_idx(&rows, None), Some(1));
        assert_eq!(next_diff_idx(&rows, Some(1)), Some(2));
        assert_eq!(next_diff_idx(&rows, Some(2)), Some(4));
        // Wrap past the end back to the first diff.
        assert_eq!(next_diff_idx(&rows, Some(4)), Some(1));
        // Prev from past-end lands on the last diff.
        assert_eq!(prev_diff_idx(&rows, Some(rows.len())), Some(4));
        assert_eq!(prev_diff_idx(&rows, Some(4)), Some(2));
        assert_eq!(prev_diff_idx(&rows, Some(2)), Some(1));
        // Wrap before the start lands on the last diff.
        assert_eq!(prev_diff_idx(&rows, Some(1)), Some(4));
        // No diffs: both directions stay None.
        let flat = vec![TextRow {
            kind: TextRowKind::Equal,
            ..Default::default()
        }];
        assert_eq!(next_diff_idx(&flat, None), None);
        assert_eq!(prev_diff_idx(&flat, Some(1)), None);
    }

    #[test]
    fn selection_follows_navigation() {
        let rows = nav_rows();
        let mut sel = None;
        select_next_diff(&rows, &mut sel);
        assert_eq!(sel, Some(1));
        select_next_diff(&rows, &mut sel);
        assert_eq!(sel, Some(2));
        select_prev_diff(&rows, &mut sel);
        assert_eq!(sel, Some(1));
        // Prev with nothing selected jumps to the last diff.
        let mut sel2 = None;
        select_prev_diff(&rows, &mut sel2);
        assert_eq!(sel2, Some(4));
    }

    #[test]
    fn section_bounds_cover_contiguous_hunks() {
        let rows = nav_rows();
        assert_eq!(section_bounds(&rows, 1), Some((1, 3)));
        assert_eq!(section_bounds(&rows, 2), Some((1, 3)));
        assert_eq!(section_bounds(&rows, 4), Some((4, 5)));
        assert_eq!(section_bounds(&rows, 0), None);
        assert_eq!(section_bounds(&rows, 99), None);
    }

    #[test]
    fn filters_show_honest_subsets_and_counts() {
        let rows = nav_rows();
        assert_eq!(filtered_indices(&rows, TextFilter::All).len(), 5);
        assert_eq!(
            filtered_indices(&rows, TextFilter::Differences),
            vec![1, 2, 4]
        );
        assert_eq!(filtered_indices(&rows, TextFilter::Same), vec![0, 3]);
        // Context radius 2 around hunks [1,2] and [4] covers everything here.
        assert_eq!(filtered_indices(&rows, TextFilter::Context).len(), 5);
        let counts = filter_counts(&rows, TextFilter::Differences);
        assert_eq!(counts.total, 5);
        assert_eq!(counts.shown, 3);
        assert_eq!(counts.diffs, 3);
        assert_eq!(counts.sames, 2);
        let counts_all = filter_counts(&rows, TextFilter::All);
        assert_eq!(counts_all.shown, 5);
        // Long equal run with one diff in the middle: context trims tails.
        let mut long = Vec::new();
        let mut i = 0usize;
        while i < 11 {
            let kind = if i == 5 {
                TextRowKind::Changed
            } else {
                TextRowKind::Equal
            };
            long.push(TextRow {
                kind,
                ..Default::default()
            });
            i = i.saturating_add(1);
        }
        assert_eq!(
            filtered_indices(&long, TextFilter::Context),
            vec![3, 4, 5, 6, 7]
        );
    }

    #[test]
    fn text_prefs_default_matches_app_toggles() {
        let prefs = TextViewPrefs::default();
        assert!(!prefs.word_wrap);
        assert!(prefs.show_line_numbers);
        assert!(prefs.syntax_highlight);
    }

    #[test]
    fn text_prefs_effective_widths_cap_and_narrow() {
        assert_eq!(effective_pane_chars_for(200, true), 80);
        assert_eq!(effective_pane_chars_for(200, false), 200);
        assert_eq!(effective_pane_chars_for(10, true), 10);
        assert_eq!(effective_gutter_chars_for(4, true), 4);
        assert_eq!(effective_gutter_chars_for(4, false), 1);
    }

    #[test]
    fn text_prefs_view_hides_numbers_accents_and_keeps_counts() {
        let model = TextRows {
            rows: vec![TextRow {
                kind: TextRowKind::Changed,
                left_no: Some(3),
                right_no: Some(3),
                left: "alpha".to_string(),
                right: "alphi".to_string(),
                accent_a: vec![(4, 5)],
                accent_b: vec![(4, 5)],
            }],
            truncated: false,
            coarse: false,
            pane_chars: 200,
            gutter_chars: 4,
            selected: Some(0),
            filter: TextFilter::Differences,
        };
        let prefs = TextViewPrefs {
            word_wrap: true,
            show_line_numbers: false,
            syntax_highlight: false,
        };
        let view = view_text_with_prefs(&model, &prefs);
        let Some(row) = view.rows.first() else {
            return;
        };
        assert_eq!(row.left_no, None);
        assert_eq!(row.right_no, None);
        assert!(row.accent_a.is_empty());
        assert!(row.accent_b.is_empty());
        assert_eq!(view.pane_chars, 80);
        assert_eq!(view.gutter_chars, 1);
        // Selection + filter pass through so counts stay honest.
        assert_eq!(view.selected, Some(0));
        assert_eq!(view.filter, TextFilter::Differences);
        assert_eq!(
            filter_counts(&view.rows, view.filter),
            filter_counts(&model.rows, model.filter)
        );
        // Default prefs leave the model untouched.
        let plain = view_text_with_prefs(&model, &TextViewPrefs::default());
        assert_eq!(plain.pane_chars, 200);
        assert_eq!(plain.gutter_chars, 4);
        let Some(kept) = plain.rows.first() else {
            return;
        };
        assert_eq!(kept.left_no, Some(3));
        assert_eq!(kept.accent_a, vec![(4, 5)]);
    }
}
