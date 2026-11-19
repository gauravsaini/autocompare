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
pub use crate::gutter::{
    GutterGeometry, GutterSegment, gutter_bar_width, gutter_bar_width_for_font, gutter_geometry,
};
#[allow(unused_imports)]
pub use crate::home::{
    HomeItem, HomeModel, Tab, home_card_button_size, home_card_desc_is_small_weak,
    home_card_title_size, home_model,
};
#[allow(unused_imports)]
pub use crate::take::{TakeChoice, TakeError, resolve_take};
#[allow(unused_imports)]
pub use crate::theme::{
    TYPO_BODY_SIZE, TYPO_FULL_PANE_CHARS, TYPO_HEADING_SIZE, TYPO_HOME_TITLE_SIZE,
    TYPO_LETTER_SPACING, TYPO_MONO_SIZE, TYPO_MONO_SIZE_MAX, TYPO_SMALL_SIZE, TYPO_WRAP_PANE_CHARS,
    TypoRole, relative_luminance, theme_contrast_ok, theme_contrast_ratio, theme_mode_contrast_ok,
};
#[allow(unused_imports)]
pub use crate::tree::{FolderTreeNode, folder_tree};

use autocompare_compare::{
    BinaryComparison, FolderComparison, FolderStatus, TableComparison, TextComparison,
};
use autocompare_diff::{Op, PictureDiff, RgbaImage, diff_cells, inline_diff};

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
/// at [`crate::theme::TYPO_WRAP_PANE_CHARS`] (80) so long lines stay
/// readable without horizontal scrolling; wrap off keeps the full
/// model width (200 in the documented dense case, see
/// [`crate::theme::TYPO_FULL_PANE_CHARS`]).
pub fn effective_pane_chars_for(pane_chars: usize, word_wrap: bool) -> usize {
    if word_wrap {
        pane_chars.min(crate::theme::TYPO_WRAP_PANE_CHARS)
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
// Picture zoom + pan (headless math; wiring owned by views.rs)
// ----------------------------------------------------------------------------

/// Zoom percent bounds paint honors: 25% (= 0.25x) ..= 800% (= 8.0x).
/// Wider than `App`'s current 25..=400 setter clamp on purpose: the
/// math already supports 8x so a later `app.rs` widening needs no
/// paint change; `App` state stays the source of truth today.
pub const PICTURE_ZOOM_MIN_PCT: u32 = 25;
/// See [`PICTURE_ZOOM_MIN_PCT`].
pub const PICTURE_ZOOM_MAX_PCT: u32 = 800;
/// One zoom step: 25 percentage points.
pub const PICTURE_ZOOM_STEP_PCT: u32 = 25;
/// Zoom the Reset actions restore.
pub const PICTURE_ZOOM_RESET_PCT: u32 = 100;

/// Clamps a raw zoom percent into 25..=800. Pure; unit-tested.
pub fn clamp_picture_zoom(pct: u32) -> u32 {
    pct.clamp(PICTURE_ZOOM_MIN_PCT, PICTURE_ZOOM_MAX_PCT)
}

/// One zoom-in step (+25pp), saturating at 800. Pure; unit-tested.
pub fn picture_zoom_in(pct: u32) -> u32 {
    clamp_picture_zoom(pct.saturating_add(PICTURE_ZOOM_STEP_PCT))
}

/// One zoom-out step (-25pp), saturating at 25. Pure; unit-tested.
pub fn picture_zoom_out(pct: u32) -> u32 {
    clamp_picture_zoom(pct.saturating_sub(PICTURE_ZOOM_STEP_PCT))
}

/// Zoom the keyboard/button Reset restores (100%). Pure; unit-tested.
pub fn picture_zoom_reset() -> u32 {
    PICTURE_ZOOM_RESET_PCT
}

/// Zoom percent as a 0.25..=8.0 scale factor (clamped first, so wild
/// state never feeds NaN geometry into paint). Pure; unit-tested.
pub fn picture_zoom_factor(pct: u32) -> f32 {
    f32::from(clamp_picture_zoom(pct) as u16) / 100.0
}

/// Clamps a pan offset to the content bounds: each axis stays inside
/// `+/-content` (ceiled, non-finite/empty content pins to 0) and
/// inside `+/-limit` (`App::PICTURE_PAN_LIMIT` at the paint site), so
/// drag/nudge never parks the image outside its own frame. Pure;
/// unit-tested.
pub fn clamp_picture_pan(
    pan_x: i32,
    pan_y: i32,
    content_w: f32,
    content_h: f32,
    limit: i32,
) -> (i32, i32) {
    let bound = |content: f32| -> i32 {
        let nonneg = if content.is_finite() && content > 0.0 {
            content.ceil().clamp(0.0, i32::MAX as f32) as i32
        } else {
            0
        };
        nonneg.min(limit.max(0))
    };
    let bx = bound(content_w);
    let by = bound(content_h);
    (
        pan_x.clamp(bx.saturating_neg(), bx),
        pan_y.clamp(by.saturating_neg(), by),
    )
}

/// Drag-to-pan accumulation: adds the drag delta (paint passes the
/// negated grab delta so the image follows the cursor), then clamps
/// to the content bounds. Pure; unit-tested.
pub fn picture_pan_drag(
    pan_x: i32,
    pan_y: i32,
    dx: i32,
    dy: i32,
    content_w: f32,
    content_h: f32,
    limit: i32,
) -> (i32, i32) {
    clamp_picture_pan(
        pan_x.saturating_add(dx),
        pan_y.saturating_add(dy),
        content_w,
        content_h,
        limit,
    )
}

// ----------------------------------------------------------------------------
// Pane split fraction (headless math; paint + App delegate here)
// ----------------------------------------------------------------------------

/// Draggable split bounds: left pane share stays inside 20%..=80% so
/// neither pane collapses; default centers at 50%. Single source for
/// [`crate::views`] (which re-exports this math so `App` keeps one
/// path) and the dual-tree/picture width helpers. Per-run state only,
/// no filesystem writes.
pub const SPLIT_MIN_FRACTION: f32 = 0.20;
/// See [`SPLIT_MIN_FRACTION`].
pub const SPLIT_MAX_FRACTION: f32 = 0.80;
/// See [`SPLIT_MIN_FRACTION`].
pub const SPLIT_DEFAULT_FRACTION: f32 = 0.50;

/// Clamp a raw split fraction into the draggable range. Pure;
/// unit-tested.
pub fn clamp_split_fraction(fraction: f32) -> f32 {
    fraction.clamp(SPLIT_MIN_FRACTION, SPLIT_MAX_FRACTION)
}

/// Drag delta applied to a split fraction: `dx` pixels over `avail`
/// pixels of travel, then clamped. Non-positive `avail` keeps the
/// current fraction so headless callers never divide by zero. Pure;
/// unit-tested.
pub fn split_apply_drag(fraction: f32, dx: f32, avail: f32) -> f32 {
    if avail <= 0.0 {
        return clamp_split_fraction(fraction);
    }
    clamp_split_fraction(fraction + dx / avail)
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
// Retired builders (Wave K cleanup)
// ----------------------------------------------------------------------------
//
// `build_sync_summary`, `build_folder_merge_summary` aur `build_audio_summary`
// hata diye gaye: inke callers repo me kahin nahi the (sirf yahin ke unit
// tests). Replacement structured models `app.rs` me hain:
// `SyncChecklist::from_plan`, `FolderMergeChecklist::from_plan`,
// `AudioChecklist::from_diff` aur `PictureCompareModel::from_diff`
// (picture wala `build_picture_summary` ab bhi `from_diff` ke andar use hota
// hai, isliye wahi rakha hai). Caller migration: naya code seedha un
// checklist/model types ko use kare, purani `Vec<String>` summary wapas na laye.

// ----------------------------------------------------------------------------
// Find box (headless; wiring owned by views.rs / agent A)
// ----------------------------------------------------------------------------
//
// WIRING (agent A / orchestrator): the text paint keeps one `FindState`
// per text view. The find box writes `set_query(&rows, &query)` on each
// edit; Enter / Shift+Enter call `next()` / `prev()` and scroll the
// returned model row into view with the same highlight the F3 path
// uses. `count_label()` feeds the "N/M" badge (agla paint wave). No
// egui code here.

/// N/M badge text for the text find bar: "2/5", no hit par "0/0".
///
/// Pure; paint-side `views::text_find_badge_label` ke model-side judvaan —
/// agla paint wave isko badge label ke liye use karega. Unit-tested.
#[allow(dead_code)] // wiring-pending: paint (agla wave) calls this via FindState
pub fn find_count_label(position: Option<usize>, total: usize) -> String {
    match position {
        Some(pos) => format!("{pos}/{total}"),
        None => format!("0/{total}"),
    }
}

/// Case-insensitive substring matches of `query` over both panes.
///
/// Returns original-model row indices in order. Empty or
/// whitespace-only queries match nothing. Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: paint (agent A) calls this via FindState
pub fn find_match_indices(rows: &[TextRow], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    let mut out = Vec::new();
    let mut idx = 0usize;
    for row in rows {
        let hit =
            row.left.to_lowercase().contains(&needle) || row.right.to_lowercase().contains(&needle);
        if hit {
            out.push(idx);
        }
        idx = idx.saturating_add(1);
    }
    out
}

/// Headless find-box state: current query, its matches, cursor into them.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // wiring-pending: paint (agent A) owns one per text view
pub struct FindState {
    query: String,
    matches: Vec<usize>,
    cursor: Option<usize>,
}

#[allow(dead_code)] // wiring-pending: see struct
impl FindState {
    /// Recomputes matches for a new query; cursor rests on the first hit.
    pub fn set_query(&mut self, rows: &[TextRow], query: &str) {
        self.query = query.to_string();
        self.matches = find_match_indices(rows, query);
        self.cursor = if self.matches.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Current query text.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Number of matched rows ("m" in "n of m").
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// All matched model row indices, in order.
    pub fn match_rows(&self) -> &[usize] {
        &self.matches
    }

    /// 1-based position of the cursor ("n" in "n of m"), if any match.
    pub fn position(&self) -> Option<usize> {
        self.cursor
            .and_then(|c| self.matches.get(c).map(|_| c.saturating_add(1)))
    }

    /// N/M badge text ("n of m" label): `position()` over `match_count()`,
    /// no match par "0/0". Pure; agla paint wave badge ke liye use karega.
    pub fn count_label(&self) -> String {
        find_count_label(self.position(), self.match_count())
    }

    /// Currently selected match as a model row index.
    pub fn current_row(&self) -> Option<usize> {
        self.cursor.and_then(|c| self.matches.get(c).copied())
    }

    /// Advances with wrap; `None` when there are no matches.
    pub fn next(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let next = match self.cursor {
            Some(c) => (c.saturating_add(1)) % self.matches.len().max(1),
            None => 0,
        };
        self.cursor = Some(next);
        self.matches.get(next).copied()
    }

    /// Steps back with wrap; `None` when there are no matches.
    pub fn prev(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let len = self.matches.len();
        let prev = match self.cursor {
            Some(0) | None => len.saturating_sub(1),
            Some(c) => c.saturating_sub(1),
        };
        self.cursor = Some(prev);
        self.matches.get(prev).copied()
    }
}

// ----------------------------------------------------------------------------
// Merge per-hunk pick (headless; wiring owned by views.rs / agent A)
// ----------------------------------------------------------------------------
//
// WIRING (agent A / orchestrator): the merge paint lists
// `merge_conflict_hunks(&lines)` as one card per hunk, each with
// Left/Right/Base/Both buttons staging a `MergeHunkPick`. Apply-all
// calls `apply_merge_picks(&lines, &picks)`; the summary line reads
// `merge_pick_summary(&lines, &picks)`. Single-hunk resolve stays on
// `resolve_take`. No egui code here.

/// One conflict block in a merged line stream: half-open line span
/// plus its conflict index for [`crate::take::resolve_take`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (agent A) lists one card per hunk
pub struct MergeHunk {
    /// Conflict index (0-based among well-formed blocks).
    pub conflict_index: usize,
    /// First line of the `<<<<<<<` marker.
    pub start: usize,
    /// One past the `>>>>>>>` marker.
    pub end: usize,
}

/// Well-formed conflict blocks in scan order. Malformed blocks (no
/// closing marker) are skipped so paint never offers an unresolvable
/// hunk. Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: paint (agent A) lists hunk cards
pub fn merge_conflict_hunks(lines: &[String]) -> Vec<MergeHunk> {
    let mut hunks = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let is_open = lines.get(i).is_some_and(|l| l.starts_with("<<<<<<<"));
        if !is_open {
            i = i.saturating_add(1);
            continue;
        }
        let start = i;
        let mut j = i.saturating_add(1);
        let mut closed = false;
        while j < lines.len() {
            if lines.get(j).is_some_and(|l| l.starts_with(">>>>>>>")) {
                closed = true;
                j = j.saturating_add(1);
                break;
            }
            j = j.saturating_add(1);
        }
        if closed {
            hunks.push(MergeHunk {
                conflict_index: hunks.len(),
                start,
                end: j,
            });
            i = j;
        } else {
            break;
        }
    }
    hunks
}

/// One staged per-hunk choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (agent A) stages button picks
pub struct MergeHunkPick {
    /// Which conflict block this pick resolves.
    pub conflict_index: usize,
    /// Which side to take for that block.
    pub choice: TakeChoice,
}

/// Applies staged per-hunk picks, highest index first so earlier spans
/// stay valid. Duplicate picks for one hunk keep the last staged
/// choice. Errors when a pick names no real conflict. Pure.
#[allow(dead_code)] // wiring-pending: paint (agent A) Apply-all calls this
pub fn apply_merge_picks(
    lines: &[String],
    picks: &[MergeHunkPick],
) -> Result<Vec<String>, TakeError> {
    let mut dedup: Vec<(usize, TakeChoice)> = Vec::new();
    for pick in picks {
        let mut found = false;
        for slot in dedup.iter_mut() {
            if slot.0 == pick.conflict_index {
                slot.1 = pick.choice;
                found = true;
                break;
            }
        }
        if !found {
            dedup.push((pick.conflict_index, pick.choice));
        }
    }
    dedup.sort_by_key(|item| std::cmp::Reverse(item.0));
    let mut current: Vec<String> = lines.to_vec();
    for (index, choice) in dedup {
        current = resolve_take(&current, index, choice)?;
    }
    Ok(current)
}

/// Honest per-hunk progress over the live merged lines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (agent A) summary line
pub struct MergePickSummary {
    /// Well-formed conflict blocks in the lines.
    pub total: usize,
    /// Distinct staged picks naming a real block.
    pub resolved: usize,
    /// Blocks still awaiting a pick.
    pub pending: usize,
    /// Whether any block is still unresolved.
    pub conflicted: bool,
}

/// Summarizes staged picks against the current lines. Picks naming no
/// real block do not count as resolved. Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: paint (agent A) summary line
pub fn merge_pick_summary(lines: &[String], picks: &[MergeHunkPick]) -> MergePickSummary {
    let total = merge_conflict_hunks(lines).len();
    let mut seen: Vec<usize> = Vec::new();
    for pick in picks {
        if pick.conflict_index < total && !seen.contains(&pick.conflict_index) {
            seen.push(pick.conflict_index);
        }
    }
    let resolved = seen.len();
    MergePickSummary {
        total,
        resolved,
        pending: total.saturating_sub(resolved),
        conflicted: resolved < total,
    }
}

// ----------------------------------------------------------------------------
// Three-pane merge plan (headless; paint wiring next wave)
// ----------------------------------------------------------------------------
//
// Gap #7 model half (text side): `TriplePaths` (base/left/right) se 3-pane
// merge plan. `merge_conflict_hunks` already-merged marker lines par kaam
// karta hai; ye section usse ek kadam pehle ka plan hai: base/left/right
// line slices seedha hunk rows me align hote hain, har hunk par ek
// take-side decision (`TakeChoice` reuse) lagti hai, aur summary helper
// paint ki progress line deta hai. Decided hunks splice hote hain,
// undecided conflicts marker block banate hain taaki existing
// `merge_conflict_hunks` / `merge_pick_summary` downstream chalte rahen.
// WIRING (next wave): paint `ThreePanePlan::new` se plan banakar per-hunk
// cards dikhaye, card buttons `decide` call karen, output `merge_lines` se,
// summary line `summary().summary_line()` se. No egui, no I/O.

/// One aligned hunk ka change shape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (next wave) lists one card per hunk
pub enum ThreePaneKind {
    /// Teenon sides ek jaise.
    #[default]
    Same,
    /// Sirf left base se alag (right base jaisa).
    LeftChange,
    /// Sirf right base se alag (left base jaisa).
    RightChange,
    /// Dono ek jaise badle (clean auto-merge).
    BothSameChange,
    /// Dono alag-alag badle (manual pick chahiye).
    Conflict,
}

/// Teen panes ki ek aligned hunk row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (next wave) lists one card per hunk
pub struct ThreePaneHunk {
    /// Hunk order (0-based, `decisions` isi se judte hain).
    pub index: usize,
    /// Is hunk ka change shape.
    pub kind: ThreePaneKind,
    /// Base side ki lines (chhoti side par kam entries).
    pub base: Vec<String>,
    /// Left side ki lines.
    pub left: Vec<String>,
    /// Right side ki lines.
    pub right: Vec<String>,
    /// Sirf `Conflict` par true.
    pub conflicted: bool,
}

/// Ek hunk par staged take-side decision (`TakeChoice` reuse:
/// Left/Right/Base/Both).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (next wave) stages button picks
pub struct ThreePaneDecision {
    /// Kaunsa hunk resolve hota hai.
    pub hunk: usize,
    /// Kaunsi side li jaye.
    pub side: TakeChoice,
}

/// Teen panes ka poora plan: hunks plus staged decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (next wave) keeps one per merge tab
pub struct ThreePanePlan {
    /// Aligned hunks, order me.
    pub hunks: Vec<ThreePaneHunk>,
    /// Staged per-hunk decisions (ek hunk par aakhri staged jeetta hai).
    pub decisions: Vec<ThreePaneDecision>,
}

/// Per-hunk progress over the plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (next wave) summary line
pub struct ThreePaneSummary {
    /// Kul hunks.
    pub total: usize,
    /// Asli hunk naam wale distinct staged picks.
    pub decided: usize,
    /// Faisle ke intezar me hunks.
    pub pending: usize,
    /// Koi conflict bina decision ke bacha hai.
    pub conflicted: bool,
}

#[allow(dead_code)] // wiring-pending: paint (next wave) summary line
impl ThreePaneSummary {
    /// Honest one-line progress.
    pub fn summary_line(self) -> String {
        format!(
            "{} hunk(s), {} decided, {} pending{}",
            self.total,
            self.decided,
            self.pending,
            if self.conflicted { ", conflicted" } else { "" }
        )
    }
}

/// Ek line position ka shape: missing tail `None` hai, isliye chhoti
/// side ki extra lines apne side ka change banti hain. Pure.
#[allow(dead_code)] // wiring-pending: `three_pane_hunks` helper
fn three_pane_line_kind(
    base: Option<&str>,
    left: Option<&str>,
    right: Option<&str>,
) -> ThreePaneKind {
    if left == base && right == base {
        ThreePaneKind::Same
    } else if right == base {
        ThreePaneKind::LeftChange
    } else if left == base {
        ThreePaneKind::RightChange
    } else if left == right {
        ThreePaneKind::BothSameChange
    } else {
        ThreePaneKind::Conflict
    }
}

/// Base/left/right slices ko maximal same-shape runs me align karta hai.
/// Missing tails `None` hain (koi padding line nahi), har side apni
/// maujood lines rakhta hai. Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: `ThreePanePlan::new` + paint (next wave)
pub fn three_pane_hunks(base: &[String], left: &[String], right: &[String]) -> Vec<ThreePaneHunk> {
    let total = base.len().max(left.len()).max(right.len());
    let mut hunks: Vec<ThreePaneHunk> = Vec::new();
    let mut i = 0usize;
    while i < total {
        let kind = three_pane_line_kind(
            base.get(i).map(String::as_str),
            left.get(i).map(String::as_str),
            right.get(i).map(String::as_str),
        );
        let continues = hunks.last().is_some_and(|hunk| hunk.kind == kind);
        if !continues {
            hunks.push(ThreePaneHunk {
                index: hunks.len(),
                kind,
                base: Vec::new(),
                left: Vec::new(),
                right: Vec::new(),
                conflicted: kind == ThreePaneKind::Conflict,
            });
        }
        if let Some(hunk) = hunks.last_mut() {
            if let Some(line) = base.get(i) {
                hunk.base.push(line.clone());
            }
            if let Some(line) = left.get(i) {
                hunk.left.push(line.clone());
            }
            if let Some(line) = right.get(i) {
                hunk.right.push(line.clone());
            }
        }
        i = i.saturating_add(1);
    }
    hunks
}

/// Decisions ko merged lines me badalta hai. Decided hunk `TakeChoice`
/// se splice hota hai (duplicate picks me aakhri staged jeetta hai);
/// undecided non-conflict auto-resolve hota hai (badli side jeetti hai,
/// dono-same par left), undecided conflict marker block banata hai taaki
/// `merge_conflict_hunks` use downstream dekh sake. Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: paint (next wave) output pane
pub fn three_pane_merge_lines(
    hunks: &[ThreePaneHunk],
    decisions: &[ThreePaneDecision],
) -> Vec<String> {
    let mut out = Vec::new();
    for hunk in hunks {
        let pick = decisions
            .iter()
            .rev()
            .find(|decision| decision.hunk == hunk.index)
            .map(|decision| decision.side);
        match pick {
            Some(side) => match side {
                TakeChoice::Left => out.extend(hunk.left.iter().cloned()),
                TakeChoice::Right => out.extend(hunk.right.iter().cloned()),
                TakeChoice::Base => out.extend(hunk.base.iter().cloned()),
                TakeChoice::Both => {
                    out.extend(hunk.left.iter().cloned());
                    out.extend(hunk.right.iter().cloned());
                }
            },
            None => match hunk.kind {
                ThreePaneKind::Same => out.extend(hunk.base.iter().cloned()),
                ThreePaneKind::LeftChange => out.extend(hunk.left.iter().cloned()),
                ThreePaneKind::RightChange => out.extend(hunk.right.iter().cloned()),
                ThreePaneKind::BothSameChange => out.extend(hunk.left.iter().cloned()),
                ThreePaneKind::Conflict => {
                    out.push("<<<<<<< left".to_string());
                    out.extend(hunk.left.iter().cloned());
                    out.push("||||||| base".to_string());
                    out.extend(hunk.base.iter().cloned());
                    out.push("=======".to_string());
                    out.extend(hunk.right.iter().cloned());
                    out.push(">>>>>>> right".to_string());
                }
            },
        }
    }
    out
}

/// Hunks + decisions par honest progress: unknown hunk naam wale picks
/// kabhi decided nahi ginte. Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: paint (next wave) summary line
pub fn three_pane_summary(
    hunks: &[ThreePaneHunk],
    decisions: &[ThreePaneDecision],
) -> ThreePaneSummary {
    let total = hunks.len();
    let mut seen: Vec<usize> = Vec::new();
    for decision in decisions {
        if hunks.get(decision.hunk).is_some() && !seen.contains(&decision.hunk) {
            seen.push(decision.hunk);
        }
    }
    let decided = seen.len();
    let mut conflicted = false;
    for hunk in hunks {
        if hunk.conflicted && !seen.contains(&hunk.index) {
            conflicted = true;
            break;
        }
    }
    ThreePaneSummary {
        total,
        decided,
        pending: total.saturating_sub(decided),
        conflicted,
    }
}

#[allow(dead_code)] // wiring-pending: paint (next wave) keeps one per merge tab
impl ThreePanePlan {
    /// Teen slices se naya plan (koi decision staged nahi).
    pub fn new(base: &[String], left: &[String], right: &[String]) -> Self {
        Self {
            hunks: three_pane_hunks(base, left, right),
            decisions: Vec::new(),
        }
    }

    /// Ek hunk par decision stage karta hai (aakhri staged jeetta hai).
    /// Unknown hunk index chup-chaap ignore hota hai, kabhi jhootha
    /// decided count nahi banata.
    pub fn decide(&mut self, hunk: usize, side: TakeChoice) {
        if self.hunks.get(hunk).is_none() {
            return;
        }
        for decision in self.decisions.iter_mut() {
            if decision.hunk == hunk {
                decision.side = side;
                return;
            }
        }
        self.decisions.push(ThreePaneDecision { hunk, side });
    }

    /// Current decisions se merged output lines.
    pub fn merge_lines(&self) -> Vec<String> {
        three_pane_merge_lines(&self.hunks, &self.decisions)
    }

    /// Current decisions par honest progress.
    pub fn summary(&self) -> ThreePaneSummary {
        three_pane_summary(&self.hunks, &self.decisions)
    }
}

// ----------------------------------------------------------------------------
// Truncation paging (headless; wiring owned by views.rs / agent A)
// ----------------------------------------------------------------------------
//
// WIRING (agent A / orchestrator): where paint shows the "showing first
// N" note (`MAX_ROWS` cap), keep one `PageCursor` per view. Page
// buttons call `page_window`, the load-more button calls `load_more()`.
// The paint window itself still slices `model.rows`; this only tracks
// the honest `(start, end)` window. No egui code here.

/// Default page step for load-more paging over truncated models.
#[allow(dead_code)] // wiring-pending: paint (agent A) load-more step
pub const PAGE_SIZE: usize = 5_000;

/// A load-more cursor over a truncated row list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // wiring-pending: paint (agent A) keeps one per view
pub struct PageCursor {
    /// Full model length the window views.
    pub total: usize,
    /// Rows currently shown (window end).
    pub shown: usize,
    /// Rows each load-more step adds.
    pub page_size: usize,
}

/// Clamped `(start, end)` window for `page` (0-based) over `total`
/// rows with `page_size` rows per page. Zero page size shows nothing.
/// Pure; unit-tested.
#[allow(dead_code)] // wiring-pending: paint (agent A) page buttons
pub fn page_window(total: usize, page: usize, page_size: usize) -> (usize, usize) {
    if page_size == 0 || total == 0 {
        return (0, 0);
    }
    let start = page.saturating_mul(page_size).min(total);
    let end = start.saturating_add(page_size).min(total);
    (start, end)
}

#[allow(dead_code)] // wiring-pending: see struct
impl PageCursor {
    /// New cursor showing the first page over `total` rows.
    pub fn new(total: usize, page_size: usize) -> Self {
        let size = if page_size == 0 { PAGE_SIZE } else { page_size };
        Self {
            total,
            shown: total.min(size),
            page_size: size,
        }
    }

    /// Number of pages covering `total` at this page size (min 1).
    pub fn page_count(&self) -> usize {
        if self.total == 0 {
            return 1;
        }
        let pages = self.total.div_ceil(self.page_size.max(1));
        pages.max(1)
    }

    /// Whether more rows wait beyond the current window.
    pub fn has_more(&self) -> bool {
        self.shown < self.total
    }

    /// Rows still hidden beyond the window.
    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.shown)
    }

    /// Current `(0, shown)` window.
    pub fn window(&self) -> (usize, usize) {
        (0, self.shown.min(self.total))
    }

    /// Grows the window by one page; returns the new window.
    pub fn load_more(&mut self) -> (usize, usize) {
        self.shown = self.shown.saturating_add(self.page_size).min(self.total);
        self.window()
    }

    /// Retargets the cursor at a new model length, keeping the page size.
    pub fn retarget(&mut self, total: usize) {
        self.total = total;
        self.shown = self.shown.min(total);
        if self.shown == 0 {
            self.shown = total.min(self.page_size);
        }
    }
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

    // ---- retired builder tests (Wave K) ----
    //
    // `build_sync_summary`, `build_folder_merge_summary`, `build_audio_summary`
    // ke 7 tests yahin hata diye gaye kyunki functions hi remove hue.
    // Replacement coverage `app.rs` me barkarar hai (Wave J-B):
    // `SyncChecklist::from_plan`, `FolderMergeChecklist::from_plan`,
    // `AudioChecklist::from_diff`, `PictureCompareModel::from_diff`.

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
        use crate::theme::{TYPO_FULL_PANE_CHARS, TYPO_WRAP_PANE_CHARS};
        assert_eq!(TYPO_WRAP_PANE_CHARS, 80);
        assert_eq!(TYPO_FULL_PANE_CHARS, 200);
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

    fn find_rows() -> Vec<TextRow> {
        vec![
            TextRow {
                kind: TextRowKind::Equal,
                left: "alpha beta".to_string(),
                right: "alpha beta".to_string(),
                ..TextRow::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: "gamma delta".to_string(),
                right: "gamma DELTA".to_string(),
                ..TextRow::default()
            },
            TextRow {
                kind: TextRowKind::Inserted,
                left: String::new(),
                right: "GAMMA rays".to_string(),
                ..TextRow::default()
            },
            TextRow {
                kind: TextRowKind::Equal,
                left: "plain line".to_string(),
                right: "plain line".to_string(),
                ..TextRow::default()
            },
        ]
    }

    #[test]
    fn find_matches_case_insensitive_over_both_panes() {
        let rows = find_rows();
        assert_eq!(find_match_indices(&rows, "gamma"), vec![1, 2]);
        assert_eq!(find_match_indices(&rows, "GAMMA"), vec![1, 2]);
        assert_eq!(find_match_indices(&rows, "delta"), vec![1]);
        assert_eq!(find_match_indices(&rows, "absent"), Vec::<usize>::new());
        assert_eq!(find_match_indices(&rows, ""), Vec::<usize>::new());
        assert_eq!(find_match_indices(&rows, "   "), Vec::<usize>::new());
    }

    #[test]
    fn find_state_cycles_next_prev_with_wrap() {
        let rows = find_rows();
        let mut state = FindState::default();
        state.set_query(&rows, "gamma");
        assert_eq!(state.query(), "gamma");
        assert_eq!(state.match_count(), 2);
        assert_eq!(state.current_row(), Some(1));
        assert_eq!(state.position(), Some(1));
        assert_eq!(state.next(), Some(2));
        assert_eq!(state.position(), Some(2));
        // Wrap forward to the first hit.
        assert_eq!(state.next(), Some(1));
        // Wrap backward from the first hit to the last.
        assert_eq!(state.prev(), Some(2));
        assert_eq!(state.prev(), Some(1));
    }

    #[test]
    fn find_state_empty_query_has_no_position() {
        let rows = find_rows();
        let mut state = FindState::default();
        state.set_query(&rows, "absent");
        assert_eq!(state.match_count(), 0);
        assert_eq!(state.current_row(), None);
        assert_eq!(state.position(), None);
        assert_eq!(state.next(), None);
        assert_eq!(state.prev(), None);
        // Re-query replaces old matches instead of accumulating.
        state.set_query(&rows, "alpha");
        assert_eq!(state.match_rows(), &[1 - 1]);
        assert_eq!(state.current_row(), Some(0));
    }

    #[test]
    fn find_count_label_formats_n_of_m() {
        assert_eq!(find_count_label(Some(2), 5), "2/5");
        assert_eq!(find_count_label(Some(1), 1), "1/1");
        assert_eq!(find_count_label(None, 5), "0/5");
        assert_eq!(find_count_label(None, 0), "0/0");
    }

    #[test]
    fn find_state_count_label_tracks_cursor() {
        let rows = find_rows();
        let mut state = FindState::default();
        state.set_query(&rows, "gamma");
        assert_eq!(state.count_label(), "1/2");
        state.next();
        assert_eq!(state.count_label(), "2/2");
        // Wrap forward to the first hit.
        state.next();
        assert_eq!(state.count_label(), "1/2");
        // Empty query reports 0/0 and no navigation.
        state.set_query(&rows, "absent");
        assert_eq!(state.count_label(), "0/0");
        assert_eq!(state.next(), None);
        assert_eq!(state.count_label(), "0/0");
    }

    fn merged_two_conflicts() -> Vec<String> {
        vec![
            "clean header".to_string(),
            "<<<<<<< left".to_string(),
            "L1".to_string(),
            "||||||| base".to_string(),
            "B1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>> right".to_string(),
            "middle".to_string(),
            "<<<<<<< left".to_string(),
            "L2".to_string(),
            "=======".to_string(),
            "R2".to_string(),
            ">>>>>>> right".to_string(),
            "footer".to_string(),
        ]
    }

    #[test]
    fn merge_hunks_list_well_formed_blocks_in_order() {
        let lines = merged_two_conflicts();
        let hunks = merge_conflict_hunks(&lines);
        assert_eq!(hunks.len(), 2);
        let Some(first) = hunks.first() else {
            return;
        };
        assert_eq!(first.conflict_index, 0);
        assert_eq!(first.start, 1);
        assert_eq!(first.end, 8);
        let Some(second) = hunks.get(1) else {
            return;
        };
        assert_eq!(second.conflict_index, 1);
        assert_eq!(second.start, 9);
        assert_eq!(second.end, 14);
        // Malformed tail block is skipped, never offered as a hunk.
        let mut bad = lines.clone();
        bad.push("<<<<<<< dangling".to_string());
        assert_eq!(merge_conflict_hunks(&bad).len(), 2);
        assert!(merge_conflict_hunks(&["clean".to_string()]).is_empty());
    }

    #[test]
    fn merge_picks_apply_per_hunk_independently() {
        let lines = merged_two_conflicts();
        let picks = vec![
            MergeHunkPick {
                conflict_index: 0,
                choice: TakeChoice::Left,
            },
            MergeHunkPick {
                conflict_index: 1,
                choice: TakeChoice::Right,
            },
        ];
        let out = apply_merge_picks(&lines, &picks).unwrap_or_default();
        assert_eq!(
            out,
            vec![
                "clean header".to_string(),
                "L1".to_string(),
                "middle".to_string(),
                "R2".to_string(),
                "footer".to_string(),
            ]
        );
        // Single-hunk pick leaves the other block conflicted.
        let one = apply_merge_picks(
            &lines,
            &[MergeHunkPick {
                conflict_index: 1,
                choice: TakeChoice::Both,
            }],
        )
        .unwrap_or_default();
        assert!(one.iter().any(|l| l.starts_with("<<<<<<<")));
        assert!(one.contains(&"L2".to_string()));
        assert!(one.contains(&"R2".to_string()));
    }

    #[test]
    fn merge_picks_dedup_and_reject_unknown() {
        let lines = merged_two_conflicts();
        // Last staged choice wins for a repeated hunk.
        let dup = vec![
            MergeHunkPick {
                conflict_index: 0,
                choice: TakeChoice::Left,
            },
            MergeHunkPick {
                conflict_index: 0,
                choice: TakeChoice::Right,
            },
            MergeHunkPick {
                conflict_index: 1,
                choice: TakeChoice::Base,
            },
        ];
        let out = apply_merge_picks(&lines, &dup).unwrap_or_default();
        assert!(out.contains(&"R1".to_string()));
        assert!(!out.contains(&"L1".to_string()));
        // Unknown conflict index errors instead of guessing.
        let bad = apply_merge_picks(
            &lines,
            &[MergeHunkPick {
                conflict_index: 7,
                choice: TakeChoice::Left,
            }],
        );
        assert!(bad.is_err());
    }

    #[test]
    fn merge_pick_summary_counts_progress_honestly() {
        let lines = merged_two_conflicts();
        let none = merge_pick_summary(&lines, &[]);
        assert_eq!(none.total, 2);
        assert_eq!(none.resolved, 0);
        assert_eq!(none.pending, 2);
        assert!(none.conflicted);
        let one = merge_pick_summary(
            &lines,
            &[MergeHunkPick {
                conflict_index: 0,
                choice: TakeChoice::Left,
            }],
        );
        assert_eq!((one.resolved, one.pending), (1, 1));
        assert!(one.conflicted);
        // Unknown picks never inflate resolved.
        let unknown = merge_pick_summary(
            &lines,
            &[MergeHunkPick {
                conflict_index: 9,
                choice: TakeChoice::Left,
            }],
        );
        assert_eq!(unknown.resolved, 0);
        let full = merge_pick_summary(
            &lines,
            &[
                MergeHunkPick {
                    conflict_index: 0,
                    choice: TakeChoice::Left,
                },
                MergeHunkPick {
                    conflict_index: 1,
                    choice: TakeChoice::Right,
                },
            ],
        );
        assert_eq!((full.resolved, full.pending), (2, 0));
        assert!(!full.conflicted);
    }

    #[test]
    fn page_window_clamps_to_model_bounds() {
        assert_eq!(page_window(10_000, 0, 5_000), (0, 5_000));
        assert_eq!(page_window(10_000, 1, 5_000), (5_000, 10_000));
        assert_eq!(page_window(10_000, 2, 5_000), (10_000, 10_000));
        assert_eq!(page_window(7_000, 1, 5_000), (5_000, 7_000));
        assert_eq!(page_window(0, 0, 5_000), (0, 0));
        assert_eq!(page_window(10_000, 0, 0), (0, 0));
    }

    #[test]
    fn page_cursor_load_more_grows_to_total() {
        let mut cursor = PageCursor::new(12_000, 5_000);
        assert_eq!(cursor.window(), (0, 5_000));
        assert!(cursor.has_more());
        assert_eq!(cursor.remaining(), 7_000);
        assert_eq!(cursor.page_count(), 3);
        assert_eq!(cursor.load_more(), (0, 10_000));
        assert!(cursor.has_more());
        assert_eq!(cursor.load_more(), (0, 12_000));
        assert!(!cursor.has_more());
        assert_eq!(cursor.remaining(), 0);
        // Extra load-more never overshoots.
        assert_eq!(cursor.load_more(), (0, 12_000));
    }

    #[test]
    fn page_cursor_retarget_tracks_new_models() {
        let mut cursor = PageCursor::new(12_000, 5_000);
        cursor.load_more();
        cursor.retarget(3_000);
        assert_eq!(cursor.window(), (0, 3_000));
        assert!(!cursor.has_more());
        // Empty model shows an empty window on a single page.
        cursor.retarget(0);
        assert_eq!(cursor.window(), (0, 0));
        assert_eq!(cursor.page_count(), 1);
        assert!(!cursor.has_more());
        // Zero page size falls back to the default step.
        let fallback = PageCursor::new(9_000, 0);
        assert_eq!(fallback.page_size, PAGE_SIZE);
        assert_eq!(fallback.window(), (0, PAGE_SIZE.min(9_000)));
    }

    #[test]
    fn picture_zoom_steps_clamp_quarter_to_8x() {
        assert_eq!((PICTURE_ZOOM_MIN_PCT, PICTURE_ZOOM_MAX_PCT), (25, 800));
        assert_eq!(clamp_picture_zoom(0), 25);
        assert_eq!(clamp_picture_zoom(100), 100);
        assert_eq!(clamp_picture_zoom(9_000), 800);
        assert_eq!(picture_zoom_in(100), 125);
        assert_eq!(picture_zoom_in(800), 800);
        assert_eq!(picture_zoom_out(100), 75);
        assert_eq!(picture_zoom_out(25), 25);
        assert_eq!(picture_zoom_reset(), 100);
        assert_eq!(picture_zoom_factor(25), 0.25);
        assert_eq!(picture_zoom_factor(100), 1.0);
        assert_eq!(picture_zoom_factor(800), 8.0);
        assert_eq!(picture_zoom_factor(9_000), 8.0);
    }

    #[test]
    fn picture_pan_drag_clamps_to_content_bounds() {
        // Inside content passes through; overshoot pins to +/-content.
        assert_eq!(clamp_picture_pan(10, -20, 400.0, 300.0, 4096), (10, -20));
        assert_eq!(
            clamp_picture_pan(500, -500, 400.0, 300.0, 4096),
            (400, -300)
        );
        // The hard limit caps even huge content.
        assert_eq!(
            clamp_picture_pan(9_000, 0, 100_000.0, 100_000.0, 4096),
            (4096, 0)
        );
        // Empty/non-finite content pins to zero (never NaN geometry).
        assert_eq!(clamp_picture_pan(5, 5, 0.0, 0.0, 4096), (0, 0));
        assert_eq!(
            clamp_picture_pan(5, 5, f32::NAN, f32::INFINITY, 4096),
            (0, 0)
        );
        // Drag accumulates then clamps.
        assert_eq!(
            picture_pan_drag(0, 0, 50, -30, 400.0, 300.0, 4096),
            (50, -30)
        );
        assert_eq!(
            picture_pan_drag(390, 0, 50, 0, 400.0, 300.0, 4096),
            (400, 0)
        );
    }

    #[test]
    fn split_fraction_clamps_to_drag_range() {
        assert_eq!((SPLIT_MIN_FRACTION, SPLIT_MAX_FRACTION), (0.20, 0.80));
        assert_eq!(SPLIT_DEFAULT_FRACTION, 0.50);
        assert_eq!(clamp_split_fraction(0.50), 0.50);
        assert_eq!(clamp_split_fraction(0.0), SPLIT_MIN_FRACTION);
        assert_eq!(clamp_split_fraction(1.0), SPLIT_MAX_FRACTION);
        assert_eq!(clamp_split_fraction(-5.0), SPLIT_MIN_FRACTION);
        assert_eq!(clamp_split_fraction(9.9), SPLIT_MAX_FRACTION);
    }

    #[test]
    fn split_drag_delta_moves_fraction() {
        // Half the travel moves half the fraction, then clamps.
        assert_eq!(split_apply_drag(0.50, 80.0, 800.0), 0.60);
        assert_eq!(split_apply_drag(0.50, -400.0, 800.0), SPLIT_MIN_FRACTION);
        assert_eq!(split_apply_drag(0.50, 400.0, 800.0), SPLIT_MAX_FRACTION);
        // Non-positive travel keeps the (clamped) current fraction.
        assert_eq!(split_apply_drag(0.50, 80.0, 0.0), 0.50);
        assert_eq!(split_apply_drag(9.9, 0.0, 0.0), SPLIT_MAX_FRACTION);
    }

    // ---- three-pane merge plan (Gap #7 model half) ----

    fn pane_lines(items: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        for item in items {
            out.push((*item).to_string());
        }
        out
    }

    #[test]
    fn three_pane_hunks_split_by_change_shape() {
        let base = pane_lines(&["same", "l-old", "r-old", "both-old", "clash"]);
        let left = pane_lines(&["same", "L2", "r-old", "BOTH", "LC"]);
        let right = pane_lines(&["same", "l-old", "R3", "BOTH", "RC"]);
        let hunks = three_pane_hunks(&base, &left, &right);
        assert_eq!(hunks.len(), 5);
        let kinds: Vec<ThreePaneKind> = hunks.iter().map(|hunk| hunk.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ThreePaneKind::Same,
                ThreePaneKind::LeftChange,
                ThreePaneKind::RightChange,
                ThreePaneKind::BothSameChange,
                ThreePaneKind::Conflict,
            ]
        );
        for (idx, hunk) in hunks.iter().enumerate() {
            assert_eq!(hunk.index, idx);
            assert_eq!(hunk.conflicted, hunk.kind == ThreePaneKind::Conflict);
        }
        // Same hunk teenon sides rakhta hai; conflict hunk har side ki line.
        let Some(first) = hunks.first() else {
            return;
        };
        assert_eq!(first.base, pane_lines(&["same"]));
        assert_eq!(first.left, pane_lines(&["same"]));
        assert_eq!(first.right, pane_lines(&["same"]));
    }

    #[test]
    fn three_pane_hunks_coalesce_runs_and_cover_tails() {
        // Lagatar same-shape lines ek hunk me judte hain.
        let base = pane_lines(&["a", "b", "c"]);
        let left = pane_lines(&["a", "B", "C"]);
        let right = pane_lines(&["a", "b", "c"]);
        let hunks = three_pane_hunks(&base, &left, &right);
        assert_eq!(hunks.len(), 2);
        let Some(second) = hunks.get(1) else {
            return;
        };
        assert_eq!(second.kind, ThreePaneKind::LeftChange);
        assert_eq!(second.left, pane_lines(&["B", "C"]));
        // Chhoti side ki extra lines us side ka change hain, padding nahi.
        let short = pane_lines(&["only"]);
        let empty: Vec<String> = Vec::new();
        let tail = three_pane_hunks(&empty, &short, &empty);
        assert_eq!(tail.len(), 1);
        let Some(hunk) = tail.first() else {
            return;
        };
        assert_eq!(hunk.kind, ThreePaneKind::LeftChange);
        assert!(hunk.base.is_empty());
        assert_eq!(hunk.left, pane_lines(&["only"]));
        // Teenon khaali: koi hunk nahi, jhootha plan nahi.
        assert!(three_pane_hunks(&empty, &empty, &empty).is_empty());
    }

    #[test]
    fn three_pane_merge_lines_auto_and_decided() {
        let base = pane_lines(&["same", "l-old", "clash"]);
        let left = pane_lines(&["same", "L2", "LC"]);
        let right = pane_lines(&["same", "l-old", "RC"]);
        let plan = ThreePanePlan::new(&base, &left, &right);
        assert_eq!(plan.hunks.len(), 3);
        // Bina decision: auto sides + undecided conflict marker block.
        let auto = plan.merge_lines();
        assert!(auto.contains(&"same".to_string()));
        assert!(auto.contains(&"L2".to_string()));
        assert!(auto.iter().any(|line| line.starts_with("<<<<<<<")));
        assert!(auto.iter().any(|line| line.starts_with(">>>>>>>")));
        // Marker block downstream hunks me dikhta hai.
        assert_eq!(merge_conflict_hunks(&auto).len(), 1);
        // Conflict par Left decide: markers gayab, left line rahti hai.
        let mut decided = plan.clone();
        decided.decide(2, TakeChoice::Left);
        let out = decided.merge_lines();
        assert!(out.contains(&"LC".to_string()));
        assert!(!out.iter().any(|line| line.starts_with("<<<<<<<")));
        assert!(merge_conflict_hunks(&out).is_empty());
        // Both decide: dono sides judti hain.
        let mut both = plan.clone();
        both.decide(2, TakeChoice::Both);
        let both_out = both.merge_lines();
        assert!(both_out.contains(&"LC".to_string()));
        assert!(both_out.contains(&"RC".to_string()));
    }

    #[test]
    fn three_pane_plan_decide_dedup_and_reject_unknown() {
        let base = pane_lines(&["a"]);
        let left = pane_lines(&["L"]);
        let right = pane_lines(&["R"]);
        let mut plan = ThreePanePlan::new(&base, &left, &right);
        // Aakhri staged jeetta hai (dedup, ek hi entry).
        plan.decide(0, TakeChoice::Left);
        plan.decide(0, TakeChoice::Right);
        assert_eq!(plan.decisions.len(), 1);
        let Some(decision) = plan.decisions.first() else {
            return;
        };
        assert_eq!(decision.side, TakeChoice::Right);
        assert_eq!(plan.merge_lines(), pane_lines(&["R"]));
        // Unknown hunk ignore: decisions nahi badhte, summary jhooth nahi.
        plan.decide(7, TakeChoice::Left);
        assert_eq!(plan.decisions.len(), 1);
        let summary = plan.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.decided, 1);
        assert_eq!(summary.pending, 0);
    }

    #[test]
    fn three_pane_summary_counts_progress_honestly() {
        let base = pane_lines(&["same", "clash"]);
        let left = pane_lines(&["same", "LC"]);
        let right = pane_lines(&["same", "RC"]);
        let plan = ThreePanePlan::new(&base, &left, &right);
        assert_eq!(plan.hunks.len(), 2);
        let none = plan.summary();
        assert_eq!((none.total, none.decided, none.pending), (2, 0, 2));
        assert!(none.conflicted);
        assert!(none.summary_line().contains("0 decided"));
        assert!(none.summary_line().contains("conflicted"));
        // Same hunk decide karne se conflict nahi mit-ta.
        let mut half = plan.clone();
        half.decide(0, TakeChoice::Base);
        let mid = half.summary();
        assert_eq!((mid.decided, mid.pending), (1, 1));
        assert!(mid.conflicted);
        // Dono decide: koi pending/conflict nahi.
        half.decide(1, TakeChoice::Base);
        let full = half.summary();
        assert_eq!((full.decided, full.pending), (2, 0));
        assert!(!full.conflicted);
        assert!(full.summary_line().contains("2 decided"));
        // Unknown picks decided nahi ginte.
        let unknown = three_pane_summary(
            &plan.hunks,
            &[ThreePaneDecision {
                hunk: 9,
                side: TakeChoice::Left,
            }],
        );
        assert_eq!(unknown.decided, 0);
        assert_eq!(unknown.pending, 2);
        assert!(unknown.conflicted);
    }
}
