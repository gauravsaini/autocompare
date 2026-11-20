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
// Status bar + cursor (BT-STATUS P1-2/P1-3; paint owned by BT-DIFF/views.rs)
// ----------------------------------------------------------------------------
//
// BC text tab ke neeche 3-zone status bar hai (left flags, mid Same,
// right hints) plus cursor strip (`1:1` + focused-line preview). Ye section
// sirf headless model hai: paint (`views.rs`, BT-DIFF wave) isko padhega,
// yahan koi egui code nahi.

/// 3-zone status bar model: left flags, mid verdict, right hints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusZones {
    /// Left zone: filter label plus honest counts.
    pub left: String,
    /// Mid zone: focused row ka verdict label.
    pub mid: String,
    /// Right zone: diff-nav badge plus key hints.
    pub right: String,
}

/// 1-based cursor position; column hamesha 1 (row cursor only, col cursor nahi).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CursorPos {
    /// 1-based model row.
    pub row: usize,
    /// 1-based column; hamesha 1.
    pub col: usize,
}

/// Focused-line preview strip model: focused row ke dono panes ka text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FocusedLinePreview {
    /// Line-number label: dono taraf barabar hon to `3`, alag hon to
    /// `L3 / R5`, ek taraf hon to `L7` / `R7`, numbers na hon to `-`.
    pub line_no: String,
    /// Row verdict: `Same`, `Changed`, `Deleted`, `Inserted`.
    pub kind: String,
    /// Left pane text, [`STATUS_PREVIEW_MAX_CHARS`] par cut.
    pub left: String,
    /// Right pane text, same cap.
    pub right: String,
}

/// Preview text cap (chars); lambi lines `...` suffix se cut.
pub const STATUS_PREVIEW_MAX_CHARS: usize = 120;

/// Display label of a text filter (status left zone).
pub fn text_filter_label(filter: TextFilter) -> &'static str {
    match filter {
        TextFilter::All => "All",
        TextFilter::Differences => "Differences",
        TextFilter::Same => "Same",
        TextFilter::Context => "Context",
    }
}

/// Verdict label of one text row kind.
pub fn text_row_kind_label(kind: TextRowKind) -> &'static str {
    match kind {
        TextRowKind::Equal => "Same",
        TextRowKind::Changed => "Changed",
        TextRowKind::Deleted => "Deleted",
        TextRowKind::Inserted => "Inserted",
    }
}

/// Focused row ka verdict label (status mid zone): no selection ya OOB
/// index par `no selection`, kabhi jhootha verdict nahi.
pub fn focused_kind_label(rows: &[TextRow], selected: Option<usize>) -> &'static str {
    selected
        .and_then(|idx| rows.get(idx).map(|row| text_row_kind_label(row.kind)))
        .unwrap_or("no selection")
}

/// 0-based model selection se 1-based cursor; column hamesha 1.
/// `None` ka matlab koi cursor nahi (paint `-` dikhaye).
pub fn cursor_pos_for(selected: Option<usize>) -> Option<CursorPos> {
    selected.map(|idx| CursorPos {
        row: idx.saturating_add(1),
        col: 1,
    })
}

/// Cursor strip label: `row:col` (`3:1`), koi cursor na ho to `-`.
pub fn cursor_label(pos: Option<CursorPos>) -> String {
    match pos {
        Some(at) => format!("{}:{}", at.row, at.col),
        None => "-".to_string(),
    }
}

/// Long preview text ko cap par cut; chhoti text jaisi hai waisi.
fn preview_cut(text: &str) -> String {
    if text.chars().count() <= STATUS_PREVIEW_MAX_CHARS {
        return text.to_string();
    }
    let mut out = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if count >= STATUS_PREVIEW_MAX_CHARS {
            break;
        }
        out.push(ch);
        count = count.saturating_add(1);
    }
    out.push_str("...");
    out
}

/// Focused row ka preview strip model; no selection ya OOB par `None`.
pub fn focused_line_preview_for(
    rows: &[TextRow],
    selected: Option<usize>,
) -> Option<FocusedLinePreview> {
    let idx = selected?;
    let row = rows.get(idx)?;
    let line_no = match (row.left_no, row.right_no) {
        (Some(l), Some(r)) if l == r => format!("{l}"),
        (Some(l), Some(r)) => format!("L{l} / R{r}"),
        (Some(l), None) => format!("L{l}"),
        (None, Some(r)) => format!("R{r}"),
        (None, None) => "-".to_string(),
    };
    Some(FocusedLinePreview {
        line_no,
        kind: text_row_kind_label(row.kind).to_string(),
        left: preview_cut(&row.left),
        right: preview_cut(&row.right),
    })
}

/// Teenon zones ek saath: left me filter + honest counts, mid me focused
/// verdict, right me nav badge + key hints. `mid` aur `nav_badge` paint
/// [`focused_kind_label`] aur diff-nav badge se banakar deta hai taaki nav
/// math ek jagah rahe.
pub fn status_zones_for(
    counts: &FilterCounts,
    filter: TextFilter,
    mid: &str,
    nav_badge: &str,
) -> StatusZones {
    StatusZones {
        left: format!(
            "{}: {} rows ({} shown)",
            text_filter_label(filter),
            counts.total,
            counts.shown
        ),
        mid: mid.to_string(),
        right: format!("{nav_badge} | F3 next/prev"),
    }
}

/// Status mid-zone (verdict) text role: Body (13 default proportional).
/// Verdict side counts/hints (Small 11-weak) se ek step upar taaki
/// dim-contrast gap weight/size se mite (color frozen). Paint
/// wiring-pending (views.rs status bar isko adopt karega). Pure;
/// unit-tested.
pub const fn status_verdict_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Status side zones (left counts, right nav-badge + hints) text role:
/// Small (11 weak proportional, tabular intent — counts stable).
/// Verdict (Body) se ek step neeche taaki hierarchy mid > sides
/// rahe. Koi naya size/token nahi. Pure; unit-tested.
pub const fn status_side_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::caption()
}

// ----------------------------------------------------------------------------
// Cursor plus selection (MODEL9 headless; paint wiring next wave)
// ----------------------------------------------------------------------------
//
// Gap: BC ka shared row cursor (`1:1`, bina selection `-`) dono panes ke
// pane tops + status verdicts chalata hai; ours me selection clamp
// (`paint_text_body` inline), diff-nav badge (`views::diff_nav_*`) aur
// pane-top follow alag-alag call-sites par bikhre hain. Ye section
// single-source headless model hai: sanitize + initial (honest
// empty-vs-loaded), pane top jo selection ka peecha karta hai, aur
// diff-nav badge jo status verdicts khilata hai. Paint (`views.rs`
// text/status/action-bar) agle wave me isko adopt karega; yahan koi
// egui code nahi, koi naya size/color/token nahi (roles existing
// TypoRole reuse, badge `find_count_label` shape reuse).

/// Stale/OOB selection ko honest `None` me badalta hai: empty model ya
/// out-of-range index par koi fake cursor nahi. Paint aaj yehi gate
/// inline lagata hai (`model.rows.get(sel).is_none()`); ye headless
/// single source hai. Pure; unit-tested.
pub fn sanitize_selection(total_rows: usize, selected: Option<usize>) -> Option<usize> {
    match selected {
        Some(idx) if idx < total_rows => Some(idx),
        _ => None,
    }
}

/// Loaded model ki honest initial selection: pehla difference,
/// equal-only ya empty par `None` (koi fake "row 1" nahi).
/// `build_text_rows` ka `selected: None` isi se khulta hai jab paint
/// pehli diff par khada hona chahe. Pure; unit-tested.
pub fn initial_selection_for(rows: &[TextRow]) -> Option<usize> {
    diff_indices(rows).first().copied()
}

/// Pane top jo selection ka peecha karta hai: selected window
/// `[top, top + rows_per_page)` ke andar ho to `top` untouched rehta hai
/// (clamped), neeche ho to window ki aakhri row banta hai, upar ho to
/// pehli. Bina selection top sirf clamp hota hai. Empty/zero-page par 0,
/// taaki khali model ka top kabhi jhootha na ho. [`virtual_row_window`]
/// contract reuse. Pure; unit-tested.
pub fn pane_top_for_selection(
    top: usize,
    rows_per_page: usize,
    selected: Option<usize>,
    total_rows: usize,
) -> usize {
    if total_rows == 0 || rows_per_page == 0 {
        return 0;
    }
    let top = top.min(total_rows);
    let Some(sel) = sanitize_selection(total_rows, selected) else {
        return top;
    };
    if sel < top {
        return sel;
    }
    if sel >= top.saturating_add(rows_per_page) {
        return sel
            .saturating_add(1)
            .saturating_sub(rows_per_page)
            .min(total_rows);
    }
    top
}

/// Kul difference rows (model order `is_diff_row` count). Khali ya
/// equal-only par 0. `views::diff_nav_total` ka headless judvaan — paint
/// agle wave me isko adopt karega. Pure; unit-tested.
pub fn diff_nav_total_for(rows: &[TextRow]) -> usize {
    diff_indices(rows).len()
}

/// Selected row ki diffs me 1-based position: selected diff ho to
/// `Some(pos)`, nahi to `None` (bina selection/equal/OOB par koi fake
/// count nahi — pehle sanitize, phir gate). Pure; unit-tested.
pub fn diff_nav_position_for(rows: &[TextRow], selected: Option<usize>) -> Option<usize> {
    let sel = sanitize_selection(rows.len(), selected)?;
    if !rows.get(sel).is_some_and(is_diff_row) {
        return None;
    }
    let mut pos = 0usize;
    let mut idx = 0usize;
    while idx <= sel && idx < rows.len() {
        if rows.get(idx).is_some_and(is_diff_row) {
            pos = pos.saturating_add(1);
        }
        idx = idx.saturating_add(1);
    }
    if pos == 0 { None } else { Some(pos) }
}

/// N/M diff-nav badge: "2/5", no hit par "0/0" (`find_count_label` shape
/// reuse — find/tablefind badge precedent). Status right zone aur
/// action-bar isi se khate hain taaki nav math ek jagah rahe. Pure;
/// unit-tested.
pub fn diff_nav_badge_for(rows: &[TextRow], selected: Option<usize>) -> String {
    find_count_label(
        diff_nav_position_for(rows, selected),
        diff_nav_total_for(rows),
    )
}

/// Cursor strip head line (`cursor | L3/R5 kind`) role: Body (status
/// verdict precedent reuse — focused verdict jaisi). Paint
/// `chrome_status_role` composition; koi naya size nahi. Pure;
/// unit-tested.
pub const fn cursor_strip_head_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Cursor strip preview lines role: Mono (pane body precedent reuse —
/// preview wahi 12px diff text hai, chrome nahi). Pure; unit-tested.
pub const fn cursor_strip_preview_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
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

/// Drag-handle hit width (px): mirrors `views::SPLIT_HIT_W` (8px grab
/// strip with a centered 1px visual) so headless hit math stays in
/// sync with paint without importing egui. Geometry only, no
/// color/typo. Pure token; unit-tested.
pub const SPLIT_HANDLE_HIT_W: f32 = 8.0;

/// Cursor affordance id for the split handle: paint maps this to its
/// resize-column cursor (`egui::CursorIcon::ResizeColumn`). Headless
/// `&str` keeps this module egui-free. Pure; unit-tested.
pub fn split_cursor_affordance() -> &'static str {
    "resize-column"
}

/// Absolute pointer position to split fraction: `x / avail`, clamped
/// into the draggable range. Non-finite `x`, non-finite or
/// non-positive `avail` fall back to [`SPLIT_DEFAULT_FRACTION`] so
/// headless callers never divide by zero or feed NaN into paint.
/// Pure; unit-tested.
pub fn split_fraction_from_x(x: f32, avail: f32) -> f32 {
    if !x.is_finite() || !avail.is_finite() || avail <= 0.0 {
        return SPLIT_DEFAULT_FRACTION;
    }
    clamp_split_fraction(x / avail)
}

/// Drag delta to fraction delta: `dx / avail` before clamping.
/// [`split_apply_drag`] adds this to the current fraction; this helper
/// exposes the delta alone for handle-drag models. Non-finite `dx`,
/// non-finite or non-positive `avail` yield `0.0` (no move). Pure;
/// unit-tested.
pub fn split_drag_delta_fraction(dx: f32, avail: f32) -> f32 {
    if !dx.is_finite() || !avail.is_finite() || avail <= 0.0 {
        return 0.0;
    }
    dx / avail
}

/// Handle hit x-range for `fraction` over `avail` row pixels:
/// `(x0, x1)` with `x0 <= x1`, clamped into `[0, avail]`. The center
/// sits at `avail * clamped_fraction` with [`SPLIT_HANDLE_HIT_W`]
/// spread around it; near the row edges the strip pins inside instead
/// of spilling out. Degenerate (non-finite/non-positive) `avail`
/// yields `(0.0, 0.0)`; non-finite `fraction` falls back to
/// [`SPLIT_DEFAULT_FRACTION`]. Pure; unit-tested.
pub fn split_handle_x_range(avail: f32, fraction: f32) -> (f32, f32) {
    if !avail.is_finite() || avail <= 0.0 {
        return (0.0, 0.0);
    }
    let clamped = clamp_split_fraction(fraction);
    let center_base = if clamped.is_finite() {
        clamped
    } else {
        SPLIT_DEFAULT_FRACTION
    };
    let center = (avail * center_base).clamp(0.0, avail);
    let half = (SPLIT_HANDLE_HIT_W / 2.0).clamp(0.0, avail);
    let x0 = (center - half).clamp(0.0, avail);
    let x1 = (center + half).clamp(0.0, avail);
    if x0 <= x1 { (x0, x1) } else { (x1, x0) }
}

/// Hit test: `x` lies inside [`split_handle_x_range`]. Non-finite `x`
/// or degenerate `avail` never hits. Pure; unit-tested.
pub fn split_handle_contains_x(x: f32, avail: f32, fraction: f32) -> bool {
    if !x.is_finite() || !avail.is_finite() || avail <= 0.0 {
        return false;
    }
    let (x0, x1) = split_handle_x_range(avail, fraction);
    x >= x0 && x <= x1
}

/// Dual-pane widths for `avail_w`: chrome
/// ([`crate::theme::DUAL_PANE_CHROME_W`]) comes off first, the rest
/// splits by `fraction` (clamped). At 960 the pair is 460/460, at the
/// 720 fallback 340/340; chrome wider than `avail_w` yields `(0, 0)`
/// so paint stacks instead of underflowing. Pure; unit-tested.
pub fn dual_pane_widths(avail_w: u32, fraction: f32) -> (u32, u32) {
    let free = avail_w.saturating_sub(crate::theme::DUAL_PANE_CHROME_W);
    let clamped = clamp_split_fraction(fraction);
    let share = if clamped.is_finite() {
        clamped
    } else {
        SPLIT_DEFAULT_FRACTION
    };
    let left = if free > 0 {
        (free as f32 * share).min(free as f32) as u32
    } else {
        0
    };
    (left.min(free), free.saturating_sub(left.min(free)))
}

/// Dual-pane x positions for `avail_w` at `fraction`: `(left_x,
/// right_x, gutter_x)` on the shared headless budget (`OUTER_PAD` 12
/// lead, `PANE_GAP` 4 between panes, 16px overview gutter, 8px right
/// margin = [`crate::theme::DUAL_PANE_CHROME_W`] 40). Widths come from
/// [`dual_pane_widths`]; at 960 the triple is (12, 476, 936), at the
/// 720 fallback (12, 356, 696). Chrome wider than `avail_w` yields the
/// stacked origin (12, 12, 12) so paint never underflows. Pure;
/// unit-tested. Single source for the screenshot narrow route and the
/// live dual-pane painters.
pub fn dual_pane_x_positions(avail_w: u32, fraction: f32) -> (u32, u32, u32) {
    const OUTER_PAD: u32 = 12;
    const PANE_GAP: u32 = 4;
    let (left_w, right_w) = dual_pane_widths(avail_w, fraction);
    if left_w == 0 && right_w == 0 {
        return (OUTER_PAD, OUTER_PAD, OUTER_PAD);
    }
    let left_x = OUTER_PAD;
    let right_x = left_x.saturating_add(left_w).saturating_add(PANE_GAP);
    let gutter_x = right_x.saturating_add(right_w);
    (left_x, right_x, gutter_x)
}

// ----------------------------------------------------------------------------
// Text-center exquisite geometry (headless): center gutter + viewport fill.
// Paint-neutral this wave — `views.rs` adopts in a later wave; these lock
// the budget math headless so the BC gap (no gutter, black void, one-blob
// accents) has a measured target. All margins reuse the SPACE scale via
// theme tokens; no new px literals, no color moves.
// ----------------------------------------------------------------------------

/// Text-center gutter width in px (headless): single source over
/// [`crate::theme::TEXT_CENTER_GUTTER_W`] (48.0 = header 28 + gutter bar
/// 12 + 2 x tight gap 4). Fixed column between the two panes carrying
/// per-hunk copy arrows; equal rows keep it blank click-to-select.
/// Pure; unit-tested.
pub fn text_center_gutter_w() -> f32 {
    crate::theme::TEXT_CENTER_GUTTER_W
}

/// Text-center chrome in px: dual-pane chrome (40) plus the fixed center
/// gutter (48) = 88. Panes split what remains; chrome wider than
/// `avail_w` yields zero free so paint stacks instead of underflowing.
/// Pure; unit-tested.
pub fn textcenter_chrome_w() -> u32 {
    crate::theme::DUAL_PANE_CHROME_W.saturating_add(text_center_gutter_w() as u32)
}

/// Text-center pane widths for `avail_w` at `fraction`: `(left_w,
/// right_w)` after the 88px chrome comes off, split by the clamped
/// fraction (same [`clamp_split_fraction`] contract as
/// [`dual_pane_widths`]). At 960 the pair is 436/436, at the 720
/// fallback 316/316; chrome wider than `avail_w` yields `(0, 0)`.
/// Pure; unit-tested.
pub fn textcenter_pane_widths(avail_w: u32, fraction: f32) -> (u32, u32) {
    let free = avail_w.saturating_sub(textcenter_chrome_w());
    let clamped = clamp_split_fraction(fraction);
    let share = if clamped.is_finite() {
        clamped
    } else {
        SPLIT_DEFAULT_FRACTION
    };
    let left = if free > 0 {
        (free as f32 * share).min(free as f32) as u32
    } else {
        0
    };
    (left.min(free), free.saturating_sub(left.min(free)))
}

/// Text-center x positions for `avail_w` at `fraction`: `(left_x,
/// gutter_x, right_x)` on the shared headless budget (OUTER_PAD 12 lead,
/// PANE_GAP 4 pane-to-gutter on each side, widths from
/// [`textcenter_pane_widths`]). At 960 the triple is (12, 452, 504), at
/// the 720 fallback (12, 332, 384). Degenerate input yields the stacked
/// origin (12, 12, 12) so paint never underflows. Pure; unit-tested.
pub fn textcenter_x_positions(avail_w: u32, fraction: f32) -> (u32, u32, u32) {
    const OUTER_PAD: u32 = 12;
    const PANE_GAP: u32 = 4;
    let (left_w, right_w) = textcenter_pane_widths(avail_w, fraction);
    if left_w == 0 && right_w == 0 {
        // Degenerate: chrome ne poora budget kha liya — stacked origin.
        return (OUTER_PAD, OUTER_PAD, OUTER_PAD);
    }
    let gutter_w = text_center_gutter_w() as u32;
    let left_x = OUTER_PAD;
    let gutter_x = left_x.saturating_add(left_w).saturating_add(PANE_GAP);
    let right_x = gutter_x.saturating_add(gutter_w).saturating_add(PANE_GAP);
    (left_x, gutter_x, right_x)
}

/// Viewport fill: how many blank ruled rows the empty area below short
/// content needs so the viewport fills instead of leaving a black void
/// (BC ruled-empty-area precedent). Slots are `ceil(panes_h / row_h)`,
/// trailing is slots minus `visible_len` — short files get positive
/// fill, full/overflow gets zero. Mirrors the paint-side trailing-blank
/// math; non-finite/non-positive budgets yield 0 so headless callers
/// never divide by zero. Pure; unit-tested.
pub fn textcenter_fill_rows(panes_h: f32, row_h: f32, visible_len: usize) -> usize {
    if !panes_h.is_finite() || !row_h.is_finite() || panes_h <= 0.0 || row_h <= 0.0 {
        return 0;
    }
    let slots = (panes_h / row_h).ceil().clamp(0.0, 100_000.0) as usize;
    slots.saturating_sub(visible_len)
}

/// Center-gutter hunk head: first row of a contiguous difference block.
/// Reuses [`section_bounds`] so the gate lives in one place — Equal rows
/// and out-of-bounds indices are never heads, continuation rows are
/// never heads. Paint shows one copy-arrow pair per hunk head and keeps
/// the rest of the gutter blank click-to-select. Pure; unit-tested.
pub fn textcenter_hunk_head(rows: &[TextRow], idx: usize) -> bool {
    match section_bounds(rows, idx) {
        Some((start, _)) => start == idx,
        None => false,
    }
}

/// Inline accent span pair for one row side: half-open char offsets
/// into the left (`accent_a`) and right (`accent_b`) text. Clippy
/// `type_complexity` gate ke liye named alias taaki
/// [`textcenter_accent_spans`] signature simple rahe.
pub type CharSpanPair<'a> = (&'a [(usize, usize)], &'a [(usize, usize)]);

/// Inline accent spans for one row: half-open char offsets hugging the
/// changed words exactly (zero char padding — sharp inline diff, never
/// a full-width wash). Only `Changed` rows carry spans; every other kind
/// reads blank. Emphasis is block + the 1px
/// [`crate::theme::TEXT_INLINE_SPAN_UNDERLINE_W`] line. Pure;
/// unit-tested.
pub fn textcenter_accent_spans(row: &TextRow) -> CharSpanPair<'_> {
    if row.kind == TextRowKind::Changed {
        (row.accent_a.as_slice(), row.accent_b.as_slice())
    } else {
        (&[], &[])
    }
}

/// Virtualized row window for `show_rows` callers: clamps `(top,
/// top+per_page)` into `[0, total]`. Empty or zero-page input yields
/// `(0, 0)`; overshoot pins to `total`. Mirrors
/// [`crate::tree::RowPager::visible_range`] for the text/table/hex
/// painters that slice without a pager. Pure; unit-tested.
pub fn virtual_row_window(total_rows: usize, top: usize, rows_per_page: usize) -> (usize, usize) {
    if total_rows == 0 || rows_per_page == 0 {
        return (0, 0);
    }
    let start = top.min(total_rows);
    let end = start.saturating_add(rows_per_page).min(total_rows);
    (start, end)
}

/// Ellipsis clip to `max_chars` characters: fits return as-is, longer
/// text keeps `max_chars - 3` chars plus `"..."`. Char-boundary safe
/// (unicode width never splits). Tiny budgets (`<= 3`) keep the first
/// `max_chars` chars with no room for dots. Pure; unit-tested.
pub fn clip_text_end(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return text.chars().take(max_chars).collect();
    }
    let keep = max_chars.saturating_sub(3);
    let mut out: String = text.chars().take(keep).collect();
    out.push_str("...");
    out
}

/// Width clip: longest ellipsis-clipped prefix whose `measure` fits
/// `budget_px`. Fits return as-is. Shrinks one char at a time so the
/// result is the longest honest label, never raw-cut pixels. Empty or
/// zero-budget text yields `""`. Pure; unit-tested with a stub measure.
pub fn clip_to_width(text: &str, budget_px: u32, measure: impl Fn(&str) -> u32) -> String {
    if measure(text) <= budget_px {
        return text.to_string();
    }
    if measure("...") > budget_px {
        return String::new();
    }
    let mut keep = text.chars().count();
    while keep > 0 {
        keep = keep.saturating_sub(1);
        let head: String = text.chars().take(keep).collect();
        let shown = format!("{head}...");
        if measure(&shown) <= budget_px {
            return shown;
        }
    }
    "...".to_string()
}

/// Screen-reader/keyboard label for the split handle: honest left-pane
/// percent plus the draggable bounds, so the drag target stays
/// operable without a pointer. Non-finite input falls back to the
/// default share. Pure; unit-tested.
pub fn split_accessibility_label(fraction: f32) -> String {
    let clamped = clamp_split_fraction(fraction);
    let share = if clamped.is_finite() {
        clamped
    } else {
        SPLIT_DEFAULT_FRACTION
    };
    let pct = (share * 100.0).round().clamp(0.0, 100.0) as u32;
    format!("Splitter: left pane {pct} percent. Drag to resize between 20 and 80 percent.")
}

/// Minimum panes-list height (px): names the legacy `.max(100.0)` floor
/// the text/merge painters used inline, so headless tests lock it.
/// Below this the minimap/gutter math degrades; at or above it panes
/// fill the remaining viewport instead of shrinking to content.
pub const TEXT_PANES_MIN_H: f32 = 100.0;

/// Remaining-height budget for the text/merge panes row: paint captures
/// `ui.available_height()` *before* the panes `horizontal` (toolbar +
/// file-info bar already spent above) and the row keeps at least
/// [`TEXT_PANES_MIN_H`]. Non-finite/non-positive input pins to the
/// floor so paint never feeds NaN into layout. Pure; unit-tested.
pub fn text_panes_height(avail_h: f32) -> f32 {
    if avail_h.is_finite() && avail_h > 0.0 {
        avail_h.max(TEXT_PANES_MIN_H)
    } else {
        TEXT_PANES_MIN_H
    }
}

// ----------------------------------------------------------------------------
// Viewmodel Density + Row-Height Tokens (MODEL10 headless)
// ----------------------------------------------------------------------------
//
// Gap / Rhythm Audit: Text rows (`views.rs` paint) live `row_height_for_line`
// par chalte hain, fixtures (`screenshot.rs`) 24px rows par, aur folder
// tree (`tree.rs`) 24px `FOLDER_ROW_HEIGHT` par. Mono 12pt line-height 15.6px
// (max 13pt 16.9px), Body 13pt line-height 18.2px, Caption 12pt 14.4px.
// Row-height agar line-height se chhota ho to text ascenders/descenders
// clip ho jaate hain.
//
// Ye section density tokens + row-height formulas ka single source hai:
// - `Compact`: maximum information density, text 18.0px (table parity),
//   folder 20.0px (body 18.2px se upar — zero clip floor).
// - `Comfortable`: default 24.0px (`SPACE_XL`, screenshot/tree parity).
// - `Spacious`: relaxed 28.0px (`CHROME_HEADER_BAR_H` parity).
// - Viewport audits at canonical 960x600 and 720px fallback prove zero
//   vertical or horizontal text clipping. Pure; unit-tested.

/// Compact text row height (px): 18.0.
/// Parity with table grid row height (`views_table::TABLE_GRID_ROW_H`).
/// Accommodates `TypoRole::Mono` (15.6px line-height) with 2.4px padding,
/// and `TYPO_MONO_SIZE_MAX` (16.9px line-height) with 1.1px padding,
/// ensuring zero vertical glyph clipping.
pub const TEXT_ROW_HEIGHT_COMPACT: f32 = 18.0;

/// Comfortable text row height (px): 24.0.
/// Parity with screenshot fixtures (`screenshot.rs` 24px row_h) and [`crate::theme::SPACE_XL`].
pub const TEXT_ROW_HEIGHT_COMFORTABLE: f32 = 24.0;

/// Spacious text row height (px): 28.0.
/// Relaxed vertical rhythm matching [`crate::theme::CHROME_HEADER_BAR_H`].
pub const TEXT_ROW_HEIGHT_SPACIOUS: f32 = 28.0;

/// Compact folder row height (px): 20.0 (`SPACE_LG` 16.0 + `SPACE_XS` 4.0).
/// Minimum safe height for folder paths (`TypoRole::Body` 18.2px line-height)
/// with 1.8px padding, preventing any descender/ascender clipping.
pub const FOLDER_ROW_HEIGHT_COMPACT: f32 = 20.0;

/// Comfortable folder row height (px): 24.0.
/// Parity with [`crate::tree::FOLDER_ROW_HEIGHT`] and [`crate::theme::SPACE_XL`].
pub const FOLDER_ROW_HEIGHT_COMFORTABLE: f32 = 24.0;

/// Spacious folder row height (px): 28.0.
/// Relaxed folder tree rhythm matching [`crate::theme::CHROME_HEADER_BAR_H`].
pub const FOLDER_ROW_HEIGHT_SPACIOUS: f32 = 28.0;

/// Viewmodel density mode governing row heights and vertical rhythm across
/// text diff and folder tree views.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RowDensity {
    /// Dense packing: maximum visible rows (18px text, 20px folder).
    Compact,
    /// Canonical default: balanced readability and touch targets (24px text, 24px folder).
    #[default]
    Comfortable,
    /// Relaxed packing: spacious readability and touch targets (28px text, 28px folder).
    Spacious,
}

/// Alias for folder tree density parametrization (`tree.rs` parity).
pub type FolderDensity = RowDensity;
/// Alias for text view density parametrization.
pub type TextDensity = RowDensity;

impl RowDensity {
    /// Text row height in pixels for this density.
    pub const fn text_row_height(self) -> f32 {
        match self {
            Self::Compact => TEXT_ROW_HEIGHT_COMPACT,
            Self::Comfortable => TEXT_ROW_HEIGHT_COMFORTABLE,
            Self::Spacious => TEXT_ROW_HEIGHT_SPACIOUS,
        }
    }

    /// Folder row height in pixels for this density.
    pub const fn folder_row_height(self) -> f32 {
        match self {
            Self::Compact => FOLDER_ROW_HEIGHT_COMPACT,
            Self::Comfortable => FOLDER_ROW_HEIGHT_COMFORTABLE,
            Self::Spacious => FOLDER_ROW_HEIGHT_SPACIOUS,
        }
    }

    /// Vertical padding budget (px) for standard 12pt mono text (15.6px line-height).
    pub fn text_row_pad(self) -> f32 {
        let pad = (self.text_row_height() - crate::theme::TypoRole::Mono.line_height_px()) / 2.0;
        pad.max(0.0)
    }

    /// Vertical padding budget (px) for 13pt body folder text (18.2px line-height).
    pub fn folder_row_pad(self) -> f32 {
        let pad = (self.folder_row_height() - crate::theme::TypoRole::Body.line_height_px()) / 2.0;
        pad.max(0.0)
    }

    /// Display label for UI density toggles and menus.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Comfortable => "Comfortable",
            Self::Spacious => "Spacious",
        }
    }

    /// Number of fully visible text rows in `avail_h`. Non-finite or non-positive yields 0.
    pub fn text_viewport_capacity(self, avail_h: f32) -> usize {
        let h = self.text_row_height();
        if !avail_h.is_finite() || avail_h <= 0.0 || h <= 0.0 {
            0
        } else {
            (avail_h / h).floor() as usize
        }
    }

    /// Number of fully visible folder rows in `avail_h`. Non-finite or non-positive yields 0.
    pub fn folder_viewport_capacity(self, avail_h: f32) -> usize {
        let h = self.folder_row_height();
        if !avail_h.is_finite() || avail_h <= 0.0 || h <= 0.0 {
            0
        } else {
            (avail_h / h).floor() as usize
        }
    }

    /// Ruled trailing blank rows to fill viewport below `visible_len` rows.
    pub fn text_fill_rows(self, panes_h: f32, visible_len: usize) -> usize {
        textcenter_fill_rows(panes_h, self.text_row_height(), visible_len)
    }
}

/// Returns the text row height in pixels for the given density.
pub fn text_row_height_for_density(density: RowDensity) -> f32 {
    density.text_row_height()
}

/// Returns the folder row height in pixels for the given density.
pub fn folder_row_height_for_density(density: RowDensity) -> f32 {
    density.folder_row_height()
}

/// Clip risk detector: returns true if `row_h < line_h`, meaning glyph
/// ascenders or descenders would be cropped by the row boundary.
/// Non-finite or non-positive dimensions also report clip risk.
pub fn row_height_has_clip_risk(row_h: f32, line_h: f32) -> bool {
    if !row_h.is_finite() || !line_h.is_finite() || row_h <= 0.0 || line_h <= 0.0 {
        return true;
    }
    row_h < line_h
}

/// Absolute minimum row height (px) required for `line_h` to avoid clipping.
/// Adds the minimum 1.0px floor so lines never touch the bounding box.
pub fn min_safe_row_height_for_line(line_h: f32) -> f32 {
    if !line_h.is_finite() || line_h <= 0.0 {
        0.0
    } else {
        line_h + 1.0
    }
}

/// Character capacity for a text pane of width `pane_w` given character advance `advance_px`.
/// Non-finite or degenerate arguments yield 0. Pure; unit-tested.
pub fn text_pane_char_capacity(pane_w: u32, advance_px: f32) -> usize {
    if !advance_px.is_finite() || advance_px <= 0.0 || pane_w == 0 {
        0
    } else {
        (pane_w as f32 / advance_px).floor() as usize
    }
}

/// Audit result for viewmodel row heights and density at canonical and fallback viewports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportDensityAudit {
    /// Canonical 960x600 text pane width (px) at 50/50 split.
    pub canonical_text_pane_w: u32,
    /// 720px fallback text pane width (px) at 50/50 split.
    pub fallback_text_pane_w: u32,
    /// Canonical 960x600 textcenter pane width (px) at 50/50 split.
    pub canonical_textcenter_pane_w: u32,
    /// 720px fallback textcenter pane width (px) at 50/50 split.
    pub fallback_textcenter_pane_w: u32,
    /// Canonical 960x600 folder path column available width at depth 0 (px).
    pub canonical_folder_path_w_d0: f32,
    /// 720px fallback folder path column available width at depth 0 (px).
    pub fallback_folder_path_w_d0: f32,
    /// 720px fallback folder path column available width at depth 3 (px).
    pub fallback_folder_path_w_d3: f32,
    /// Text visible capacity at 960x600 (panes_h = 476.0px) for [Compact, Comfortable, Spacious].
    pub text_capacity_600: [usize; 3],
    /// Text visible capacity at 720x450 fallback (panes_h = 326.0px) for [Compact, Comfortable, Spacious].
    pub text_capacity_450: [usize; 3],
    /// Folder visible capacity at 960x600 for [Compact, Comfortable, Spacious].
    pub folder_capacity_600: [usize; 3],
    /// Folder visible capacity at 720x450 fallback for [Compact, Comfortable, Spacious].
    pub folder_capacity_450: [usize; 3],
    /// Zero clipping confirmed across all density modes and viewports.
    pub zero_clipping_verified: bool,
}

/// Runs a comprehensive headless audit of row-height and density tokens at
/// 960x600 and 720px fallback. Verifies that zero text clipping occurs.
pub fn audit_viewmodel_density() -> ViewportDensityAudit {
    let (c_l, _) = dual_pane_widths(crate::theme::VIEWPORT_W, 0.50);
    let (f_l, _) = dual_pane_widths(crate::theme::VIEWPORT_MIN_W, 0.50);
    let (tc_c_l, _) = textcenter_pane_widths(crate::theme::VIEWPORT_W, 0.50);
    let (tc_f_l, _) = textcenter_pane_widths(crate::theme::VIEWPORT_MIN_W, 0.50);

    // Available panes height for 600px canonical and 450px fallback viewports
    // (excluding 124px header/toolbar/status chrome)
    let panes_h_600 = 476.0;
    let panes_h_450 = 326.0;

    let text_capacity_600 = [
        RowDensity::Compact.text_viewport_capacity(panes_h_600),
        RowDensity::Comfortable.text_viewport_capacity(panes_h_600),
        RowDensity::Spacious.text_viewport_capacity(panes_h_600),
    ];
    let text_capacity_450 = [
        RowDensity::Compact.text_viewport_capacity(panes_h_450),
        RowDensity::Comfortable.text_viewport_capacity(panes_h_450),
        RowDensity::Spacious.text_viewport_capacity(panes_h_450),
    ];
    let folder_capacity_600 = [
        RowDensity::Compact.folder_viewport_capacity(panes_h_600),
        RowDensity::Comfortable.folder_viewport_capacity(panes_h_600),
        RowDensity::Spacious.folder_viewport_capacity(panes_h_600),
    ];
    let folder_capacity_450 = [
        RowDensity::Compact.folder_viewport_capacity(panes_h_450),
        RowDensity::Comfortable.folder_viewport_capacity(panes_h_450),
        RowDensity::Spacious.folder_viewport_capacity(panes_h_450),
    ];

    let mut zero_clipping = true;

    // Check vertical clipping against typographic line heights
    let mono_line_h = crate::theme::TypoRole::Mono.line_height_px(); // 15.6
    let mono_max_line_h =
        crate::theme::TYPO_MONO_SIZE_MAX * crate::theme::TypoRole::Mono.line_height(); // 16.9
    let body_line_h = crate::theme::TypoRole::Body.line_height_px(); // 18.2

    for &d in &[
        RowDensity::Compact,
        RowDensity::Comfortable,
        RowDensity::Spacious,
    ] {
        if row_height_has_clip_risk(d.text_row_height(), mono_line_h) {
            zero_clipping = false;
        }
        if row_height_has_clip_risk(d.text_row_height(), mono_max_line_h) {
            zero_clipping = false;
        }
        if row_height_has_clip_risk(d.folder_row_height(), body_line_h) {
            zero_clipping = false;
        }
    }

    let folder_side_w = crate::tree::folder_grid_side_px(); // 452.0
    let path_min_w = crate::tree::folder_column_min_px(crate::tree::FolderColumn::Path); // 184.0

    // Check 960 and 720 horizontal clearances
    let c_path_d0 =
        crate::theme::VIEWPORT_W as f32 - folder_side_w - crate::tree::tree_indent_for_depth(0);
    let f_path_d0 =
        crate::theme::VIEWPORT_MIN_W as f32 - folder_side_w - crate::tree::tree_indent_for_depth(0);
    let f_path_d3 =
        crate::theme::VIEWPORT_MIN_W as f32 - folder_side_w - crate::tree::tree_indent_for_depth(3);

    if c_path_d0 < path_min_w || f_path_d0 < path_min_w || f_path_d3 < path_min_w {
        zero_clipping = false;
    }

    ViewportDensityAudit {
        canonical_text_pane_w: c_l,
        fallback_text_pane_w: f_l,
        canonical_textcenter_pane_w: tc_c_l,
        fallback_textcenter_pane_w: tc_f_l,
        canonical_folder_path_w_d0: c_path_d0,
        fallback_folder_path_w_d0: f_path_d0,
        fallback_folder_path_w_d3: f_path_d3,
        text_capacity_600,
        text_capacity_450,
        folder_capacity_600,
        folder_capacity_450,
        zero_clipping_verified: zero_clipping,
    }
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
// Find box (headless; paint wiring: `views::paint_text_with_find`)
// ----------------------------------------------------------------------------
//
// WIRING (Wave AE-FINDPAINT): the text paint keeps one `FindState`
// per text view (`views::paint_text_with_find`). The find bar writes
// `set_query_cased(&rows, &query, ignore_case)` on each edit, the
// ignore-case checkbox calls `set_ignore_case`, and each frame calls
// `refresh_if_stale` so a fresh compare never shows stale counts.
// Prev/Next call `prev()` / `next()` and paint syncs the returned
// model row into `selected` with the same highlight the F3 path uses.
// `count_label()` feeds the "N/M" badge. No egui code here.

/// N/M badge text for the text find bar: "2/5", no hit par "0/0".
///
/// Pure; paint-side `views::text_find_badge_label` ka model-side judvaan —
/// `FindState::count_label` badge label ke liye isko use karta hai.
/// Unit-tested.
pub fn find_count_label(position: Option<usize>, total: usize) -> String {
    match position {
        Some(pos) => format!("{pos}/{total}"),
        None => format!("0/{total}"),
    }
}

/// One pane text me needle hai ya nahi: khali needle kabhi hit nahi.
/// Table (`views_table`) / folder (`tree`) find precedent jaisa case flag.
fn find_text_contains(hay: &str, needle: &str, ignore_case: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    if ignore_case {
        hay.to_lowercase().contains(&needle.to_lowercase())
    } else {
        hay.contains(needle)
    }
}

/// Case-flag substring matches of `query` over both panes.
///
/// Returns original-model row indices in order. Empty or
/// whitespace-only queries match nothing (table/folder precedent).
/// Pure; unit-tested.
pub fn find_match_indices_cased(rows: &[TextRow], query: &str, ignore_case: bool) -> Vec<usize> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut idx = 0usize;
    for row in rows {
        let hit = find_text_contains(&row.left, query, ignore_case)
            || find_text_contains(&row.right, query, ignore_case);
        if hit {
            out.push(idx);
        }
        idx = idx.saturating_add(1);
    }
    out
}

/// Case-insensitive substring matches of `query` over both panes.
///
/// [`find_match_indices_cased`] ka `ignore_case = true` wrapper (App
/// default `true` se match; purane callers/tests unchanged). Pure;
/// unit-tested.
pub fn find_match_indices(rows: &[TextRow], query: &str) -> Vec<usize> {
    find_match_indices_cased(rows, query, true)
}

/// Headless find-box state: current query, its matches, cursor into them.
#[derive(Debug, Clone, Default)]
pub struct FindState {
    query: String,
    matches: Vec<usize>,
    cursor: Option<usize>,
    ignore_case: bool,
    rows_len: usize,
}

impl FindState {
    /// Recomputes matches for a new query; cursor rests on the first hit.
    /// Case-insensitive (App default `true` se match; purane callers unchanged).
    pub fn set_query(&mut self, rows: &[TextRow], query: &str) {
        self.set_query_cased(rows, query, true);
    }

    /// Recomputes matches for a new query + case flag; cursor rests on
    /// the first hit. Table/folder find precedent jaisa flag.
    pub fn set_query_cased(&mut self, rows: &[TextRow], query: &str, ignore_case: bool) {
        self.query = query.to_string();
        self.ignore_case = ignore_case;
        self.rows_len = rows.len();
        self.matches = find_match_indices_cased(rows, query, ignore_case);
        self.cursor = if self.matches.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Case flag badalne par same query se recompute; cursor first hit par.
    /// Flag same ho to kuch nahi badalta.
    pub fn set_ignore_case(&mut self, rows: &[TextRow], ignore_case: bool) {
        if self.ignore_case != ignore_case {
            let query = self.query.clone();
            self.set_query_cased(rows, &query, ignore_case);
        }
    }

    /// Model badalne (naya compare) par stale matches refresh karta hai:
    /// row count badla to same query + flag se recompute, cursor first
    /// hit par; count same ho to cursor untouched rehta hai. Returns true
    /// jab recompute hua. Paint har frame call karta hai (ek usize
    /// compare; O(n) sirf stale hone par).
    pub fn refresh_if_stale(&mut self, rows: &[TextRow]) -> bool {
        if self.rows_len != rows.len() {
            let query = self.query.clone();
            let flag = self.ignore_case;
            self.set_query_cased(rows, &query, flag);
            true
        } else {
            false
        }
    }

    /// Current query text.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Case flag (App default `true`; naye `FindState::default()` me
    /// `set_query` turant `true` set karta hai).
    pub fn ignore_case(&self) -> bool {
        self.ignore_case
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
    /// no match par "0/0". Pure; paint bar badge ke liye use karta hai
    /// (`views::paint_text_find_state_bar`).
    pub fn count_label(&self) -> String {
        find_count_label(self.position(), self.match_count())
    }

    /// Currently selected match as a model row index.
    pub fn current_row(&self) -> Option<usize> {
        self.cursor.and_then(|c| self.matches.get(c).copied())
    }

    /// Advances with wrap; `None` when there are no matches.
    pub fn advance(&mut self) -> Option<usize> {
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
pub struct MergeHunkPick {
    /// Which conflict block this pick resolves.
    pub conflict_index: usize,
    /// Which side to take for that block.
    pub choice: TakeChoice,
}

/// Applies staged per-hunk picks, highest index first so earlier spans
/// stay valid. Duplicate picks for one hunk keep the last staged
/// choice. Errors when a pick names no real conflict. Pure.
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
pub struct ThreePaneDecision {
    /// Kaunsa hunk resolve hota hai.
    pub hunk: usize,
    /// Kaunsi side li jaye.
    pub side: TakeChoice,
}

/// Teen panes ka poora plan: hunks plus staged decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreePanePlan {
    /// Aligned hunks, order me.
    pub hunks: Vec<ThreePaneHunk>,
    /// Staged per-hunk decisions (ek hunk par aakhri staged jeetta hai).
    pub decisions: Vec<ThreePaneDecision>,
}

/// Per-hunk progress over the plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
pub const PAGE_SIZE: usize = 5_000;

/// A load-more cursor over a truncated row list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
pub fn page_window(total: usize, page: usize, page_size: usize) -> (usize, usize) {
    if page_size == 0 || total == 0 {
        return (0, 0);
    }
    let start = page.saturating_mul(page_size).min(total);
    let end = start.saturating_add(page_size).min(total);
    (start, end)
}

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

// ----------------------------------------------------------------------------
// Hex/table zone-role helpers (typography second; paint wiring stays with
// the orchestrator).
// Gap EXQ-L viewmodel half: hex goto bar + table key/sheet bars ke zone
// roles alag-alag call-sites me bikhre hain (`views_hex`/`views_table`
// local roles) — status bar ke `status_verdict/side_text_role` precedent
// jaisa single zone contract chahiye taaki label(13-strong) > value(12)
// > hint(11-weak) hierarchy har zone me ek jaisi rahe. Geometry yahan
// nahi (bars apni `*_BAR_H`/`*_GAP`/`*_PAD` geometry khud rakhte hain);
// ye sirf family/weight/line-height/spacing hai — har accessor existing
// [`crate::theme::TypoRole`] ko reuse karta hai, koi naya size/token/color
// nahi. Colors frozen (koi Color32/Visuals nahi). Pure; unit-tested.
// ----------------------------------------------------------------------------

/// Hex goto zone label role: Body (find-bar label precedent reuse).
/// Paint-side `.strong()` composition; badge (Small) se ek step upar.
pub const fn hex_goto_zone_label_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Hex goto zone value role: Mono (offset ruler precedent reuse).
pub const fn hex_goto_zone_value_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Hex goto zone badge role: Small (find-bar badge precedent reuse).
pub const fn hex_goto_zone_badge_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Small
}

/// Table key zone label role: Body (section-label precedent reuse).
pub const fn table_key_zone_label_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Table key zone value role: Mono (grid-cell precedent reuse).
pub const fn table_key_zone_value_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Table sheet zone label role: Body (sheet-bar label precedent reuse).
pub const fn table_sheet_zone_label_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Table sheet zone value role: Mono (sheet-name precedent reuse).
pub const fn table_sheet_zone_value_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Zone badge role (hex goto / table key / table sheet badges): Small.
/// Teenon badges ek hi role se taaki N/M + source badges kabhi drift na
/// karen. Pure.
pub const fn zone_badge_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Small
}

/// Zone role ki paint weight spelling (`strong`/`weak`/`default`/
/// `regular-mono`) — hierarchy carrier (egui me tracking nahi).
/// [`crate::theme::TypoRole::weight`] ka zone alias; koi naya token nahi.
pub const fn zone_weight(role: crate::theme::TypoRole) -> &'static str {
    role.weight()
}

/// Zone role ki font family (`monospace` vs `proportional`).
/// [`crate::theme::TypoRole::family`] ka zone alias. Pure.
pub const fn zone_family(role: crate::theme::TypoRole) -> &'static str {
    role.family()
}

/// Zone role ki line-height px (`size * multiplier`, Wave V tight rows).
/// [`crate::theme::TypoRole::line_height_px`] ka zone alias — paint ab bhi
/// shared measured mono row height se lagata hai, ye headless intent hai.
pub const fn zone_line_height_px(role: crate::theme::TypoRole) -> f32 {
    role.line_height_px()
}

/// Zone role ki letter-tracking em (headers 0.02, rest 0.0; egui paint
/// 0.0 no-op rehta hai). [`crate::theme::TypoRole::letter_spacing_em`]
/// ka zone alias. Pure.
pub const fn zone_letter_spacing_em(role: crate::theme::TypoRole) -> f32 {
    role.letter_spacing_em()
}

/// Kya zone role tabular numerals chahta hai (Mono + Small only).
/// [`crate::theme::TypoRole::is_tabular`] ka zone alias. Pure.
pub const fn zone_is_tabular(role: crate::theme::TypoRole) -> bool {
    role.is_tabular()
}

// ----------------------------------------------------------------------------
// Pathmeta strip + segmented status (exq-pathmeta headless; paint wiring
// stays with the orchestrator / screenshot fixture).
// Gap: BC per-pane path bar me dropdown + folder icon + second metadata line
// (timestamp/bytes/format/rules) + bottom 4-segment status
// (cursor/format/result/edit) hai; ours me single field + Browse + single
// summary line tha. Ye section sirf headless model hai: path field text,
// second-line meta, history dropdown label aur 4-segment status — koi egui,
// koi I/O, koi naya size/color/token nahi (roles existing TypoRole reuse).
// ----------------------------------------------------------------------------

/// Ek pane ka path + second-line metadata (honest-empty: missing parts
/// khali rehte hain, koi fake "0 B"/"just now" nahi).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathPaneMeta {
    /// Pane ka path text (clip paint `clip_to_width` se karta hai).
    pub path: String,
    /// Modified timestamp label ("modified @…"), khali jab unknown.
    pub timestamp: String,
    /// Byte-size label ("10 B"), khali jab unknown.
    pub bytes: String,
    /// Format label ("Text: exact"), khali jab unknown.
    pub format: String,
    /// Rules label ("wrap:off"), khali jab unknown.
    pub rules: String,
}

/// Compact second-line meta: timestamp | bytes | format | rules. Khali parts
/// girte hain (missing file par koi fake nahi). Pure; unit-tested.
pub fn format_path_pane_meta(
    timestamp: &str,
    bytes: &str,
    format_label: &str,
    rules: &str,
) -> String {
    let mut parts = Vec::new();
    for part in [timestamp, bytes, format_label, rules] {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }
    parts.join(" | ")
}

/// Ek pane ki second-line meta string. Pure; unit-tested.
pub fn path_pane_meta_line(meta: &PathPaneMeta) -> String {
    format_path_pane_meta(&meta.timestamp, &meta.bytes, &meta.format, &meta.rules)
}

/// History dropdown ka honest label: current khali ho to "history" (koi fake
/// path nahi), warna current path. Pure; unit-tested.
pub fn path_history_label(current: &str) -> String {
    let trimmed = current.trim();
    if trimmed.is_empty() {
        "history".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Bottom 4-segment status: cursor | format | result | edit. Har segment
/// khali ho sakta hai; line join khali parts girakar banta hai taaki koi fake
/// segment na dikhe. Paint 4 zones me baant-ta hai, ye headless line deta hai.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathSegmentedStatus {
    /// Cursor segment ("3:1" / "L3 R5" / "-").
    pub cursor: String,
    /// Format segment ("Text: exact").
    pub format: String,
    /// Result segment ("4 equal, 1 changed").
    pub result: String,
    /// Edit segment ("edit: clean" / "edit: dirty").
    pub edit: String,
}

/// 4 segments se honest status line (khali parts girte hain). Pure; unit-tested.
pub fn path_segmented_status_for(
    cursor: &str,
    format: &str,
    result: &str,
    edit: &str,
) -> PathSegmentedStatus {
    PathSegmentedStatus {
        cursor: cursor.trim().to_string(),
        format: format.trim().to_string(),
        result: result.trim().to_string(),
        edit: edit.trim().to_string(),
    }
}

/// Segmented status ki single-line form (screenshot clip + tests ke liye).
/// Khali segments girte hain; sab khali hon to "-". Pure; unit-tested.
pub fn path_segmented_status_line(status: &PathSegmentedStatus) -> String {
    let mut parts = Vec::new();
    for part in [
        status.cursor.as_str(),
        status.format.as_str(),
        status.result.as_str(),
        status.edit.as_str(),
    ] {
        if !part.is_empty() {
            parts.push(part);
        }
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" | ")
    }
}

/// Path field text role: Body (path chrome precedent reuse).
pub const fn path_strip_path_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Second-line meta role: caption (Small 11 weak, status-side precedent reuse).
pub const fn path_strip_meta_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::caption()
}

/// Segmented cursor role: Body (verdict precedent — cursor sabse upar).
pub const fn path_segment_cursor_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Segmented side roles (format/result/edit): Small (side precedent reuse).
pub const fn path_segment_side_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::caption()
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
        assert_eq!(state.advance(), Some(2));
        assert_eq!(state.position(), Some(2));
        // Wrap forward to the first hit.
        assert_eq!(state.advance(), Some(1));
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
        assert_eq!(state.advance(), None);
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
        state.advance();
        assert_eq!(state.count_label(), "2/2");
        // Wrap forward to the first hit.
        state.advance();
        assert_eq!(state.count_label(), "1/2");
        // Empty query reports 0/0 and no navigation.
        state.set_query(&rows, "absent");
        assert_eq!(state.count_label(), "0/0");
        assert_eq!(state.advance(), None);
        assert_eq!(state.count_label(), "0/0");
    }

    #[test]
    fn find_match_indices_cased_honors_the_case_flag() {
        let rows = find_rows();
        assert_eq!(find_match_indices_cased(&rows, "gamma", true), vec![1, 2]);
        assert_eq!(find_match_indices_cased(&rows, "GAMMA", true), vec![1, 2]);
        // Case-sensitive: sirf exact-case pane hit hota hai.
        assert_eq!(find_match_indices_cased(&rows, "gamma", false), vec![1]);
        assert_eq!(find_match_indices_cased(&rows, "GAMMA", false), vec![2]);
        assert_eq!(find_match_indices_cased(&rows, "DELTA", false), vec![1]);
        assert_eq!(
            find_match_indices_cased(&rows, "Delta", false),
            Vec::<usize>::new()
        );
        // Khali/whitespace needle kabhi match nahi (table/folder precedent).
        assert_eq!(
            find_match_indices_cased(&rows, "", true),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_match_indices_cased(&rows, "   ", false),
            Vec::<usize>::new()
        );
        // Wrapper hamesha case-insensitive hai.
        assert_eq!(find_match_indices(&rows, "GAMMA"), vec![1, 2]);
    }

    #[test]
    fn find_state_case_flag_recomputes_with_cursor_on_first_hit() {
        let rows = find_rows();
        let mut state = FindState::default();
        state.set_query_cased(&rows, "gamma", false);
        assert!(!state.ignore_case());
        assert_eq!(state.match_rows(), &[1]);
        assert_eq!(state.count_label(), "1/1");
        state.set_ignore_case(&rows, true);
        assert!(state.ignore_case());
        assert_eq!(state.match_rows(), &[1, 2]);
        assert_eq!(state.count_label(), "1/2");
        // Same flag dobara: cursor untouched rehta hai.
        state.advance();
        assert_eq!(state.count_label(), "2/2");
        state.set_ignore_case(&rows, true);
        assert_eq!(state.count_label(), "2/2");
        // Purana set_query hamesha case-insensitive hai.
        state.set_query(&rows, "GAMMA");
        assert!(state.ignore_case());
        assert_eq!(state.match_rows(), &[1, 2]);
    }

    #[test]
    fn find_state_refresh_resets_only_on_new_model() {
        let rows = find_rows();
        let mut state = FindState::default();
        state.set_query(&rows, "gamma");
        state.advance();
        assert_eq!(state.count_label(), "2/2");
        // Same row count: no recompute, cursor untouched.
        assert!(!state.refresh_if_stale(&rows));
        assert_eq!(state.count_label(), "2/2");
        // Naya compare (count badla): recompute, cursor first hit par.
        let mut longer = rows.clone();
        longer.push(TextRow {
            kind: TextRowKind::Equal,
            left: "gamma again".to_string(),
            right: String::new(),
            ..TextRow::default()
        });
        assert!(state.refresh_if_stale(&longer));
        assert_eq!(state.match_count(), 3);
        assert_eq!(state.count_label(), "1/3");
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

    #[test]
    fn split_handle_hit_width_matches_paint_strip() {
        // Paint grab strip (`views::SPLIT_HIT_W`) 8px hai: model usi par
        // bana taaki hit math paint se alag na ho. Visual 1px centered
        // rehta hai, isliye inset dono taraf barabar banta hai.
        assert_eq!(SPLIT_HANDLE_HIT_W, 8.0);
        assert_eq!((SPLIT_HANDLE_HIT_W / 2.0) * 2.0, SPLIT_HANDLE_HIT_W);
    }

    #[test]
    fn split_handle_x_range_centers_on_fraction() {
        // 800px row, center split: 400 +/- 4px grab strip.
        assert_eq!(split_handle_x_range(800.0, 0.50), (396.0, 404.0));
        // Range hamesha ordered aur row ke andar rehta hai.
        let (lo, hi) = split_handle_x_range(800.0, 0.30);
        assert!(lo <= hi);
        assert!(lo >= 0.0 && hi <= 800.0);
        assert!((hi - lo - SPLIT_HANDLE_HIT_W).abs() < 0.001);
        // Clamp: 0.0 fraction 0.20 par tikti hai (160 +/- 4).
        let (clo, chi) = split_handle_x_range(800.0, 0.0);
        assert!((clo - 156.0).abs() < 0.01);
        assert!((chi - 164.0).abs() < 0.01);
        // Degenerate avail kabhi NaN/negative range nahi deta.
        assert_eq!(split_handle_x_range(0.0, 0.50), (0.0, 0.0));
        assert_eq!(split_handle_x_range(-10.0, 0.50), (0.0, 0.0));
        assert_eq!(split_handle_x_range(f32::NAN, 0.50), (0.0, 0.0));
        // Non-finite fraction default share (50%) par girta hai.
        assert_eq!(split_handle_x_range(800.0, f32::NAN), (396.0, 404.0));
    }

    #[test]
    fn split_handle_contains_x_hits_only_strip() {
        // Center hit, kinare miss.
        assert!(split_handle_contains_x(400.0, 800.0, 0.50));
        assert!(split_handle_contains_x(396.0, 800.0, 0.50));
        assert!(split_handle_contains_x(404.0, 800.0, 0.50));
        assert!(!split_handle_contains_x(395.9, 800.0, 0.50));
        assert!(!split_handle_contains_x(404.1, 800.0, 0.50));
        // NaN pointer kabhi hit nahi; degenerate row me koi hit nahi.
        assert!(!split_handle_contains_x(f32::NAN, 800.0, 0.50));
        assert!(!split_handle_contains_x(0.0, 0.0, 0.50));
    }

    #[test]
    fn split_fraction_from_x_clamps_and_guards() {
        assert_eq!(split_fraction_from_x(400.0, 800.0), 0.50);
        assert_eq!(split_fraction_from_x(-50.0, 800.0), SPLIT_MIN_FRACTION);
        assert_eq!(split_fraction_from_x(900.0, 800.0), SPLIT_MAX_FRACTION);
        // Guard: kharab input default share deta hai, NaN kabhi bahar nahi.
        assert_eq!(
            split_fraction_from_x(f32::NAN, 800.0),
            SPLIT_DEFAULT_FRACTION
        );
        assert_eq!(split_fraction_from_x(400.0, 0.0), SPLIT_DEFAULT_FRACTION);
        assert_eq!(
            split_fraction_from_x(400.0, f32::INFINITY),
            SPLIT_DEFAULT_FRACTION
        );
        let guarded = split_fraction_from_x(f32::NAN, f32::NAN);
        assert!(guarded.is_finite());
    }

    #[test]
    fn split_drag_delta_fraction_guards_and_scales() {
        // 80px / 800px travel = +0.10 share.
        assert!((split_drag_delta_fraction(80.0, 800.0) - 0.10).abs() < 0.000_001);
        assert!((split_drag_delta_fraction(-40.0, 800.0) + 0.05).abs() < 0.000_001);
        // Authority ke saath consistency: fraction + delta == apply_drag.
        let delta = split_drag_delta_fraction(80.0, 800.0);
        assert_eq!(split_apply_drag(0.50, 80.0, 800.0), 0.50 + delta);
        // Guard: zero/NaN travel par koi move nahi.
        assert_eq!(split_drag_delta_fraction(80.0, 0.0), 0.0);
        assert_eq!(split_drag_delta_fraction(f32::NAN, 800.0), 0.0);
        assert_eq!(split_drag_delta_fraction(80.0, f32::NAN), 0.0);
    }

    #[test]
    fn split_cursor_and_a11y_label_read_honestly() {
        // Cursor affordance paint ke ResizeColumn se judta hai.
        assert_eq!(split_cursor_affordance(), "resize-column");
        // Label me honest percent + 20..80 bounds dono hote hain.
        let label = split_accessibility_label(0.50);
        assert!(label.contains("50"));
        assert!(label.contains("20") && label.contains("80"));
        let min_label = split_accessibility_label(0.0);
        assert!(min_label.contains("20"));
        let max_label = split_accessibility_label(9.9);
        assert!(max_label.contains("80"));
        // NaN label default 50% par girta hai, crash nahi.
        assert!(split_accessibility_label(f32::NAN).contains("50"));
    }

    #[test]
    fn text_panes_height_edge_guards() {
        // Edge fixture (exq8b bottom-waste fix): panes row fills remaining
        // height; degenerate input pins to the named floor, kabhi NaN nahi.
        assert_eq!(TEXT_PANES_MIN_H, 100.0);
        assert_eq!(text_panes_height(600.0), 600.0);
        assert_eq!(text_panes_height(99.9), TEXT_PANES_MIN_H);
        assert_eq!(text_panes_height(0.0), TEXT_PANES_MIN_H);
        assert_eq!(text_panes_height(-50.0), TEXT_PANES_MIN_H);
        assert_eq!(text_panes_height(f32::NAN), TEXT_PANES_MIN_H);
        assert_eq!(text_panes_height(f32::INFINITY), TEXT_PANES_MIN_H);
        let guarded = text_panes_height(f32::NAN);
        assert!(guarded.is_finite() && guarded > 0.0);
    }

    #[test]
    fn dual_pane_widths_honor_chrome_budget() {
        assert_eq!(dual_pane_widths(960, 0.50), (460, 460));
        assert_eq!(dual_pane_widths(720, 0.50), (340, 340));
        let (left, right) = dual_pane_widths(960, 0.25);
        assert_eq!(
            left.saturating_add(right)
                .saturating_add(crate::theme::DUAL_PANE_CHROME_W),
            960
        );
        assert!(left < right);
        assert_eq!(dual_pane_widths(960, 9.9), dual_pane_widths(960, 0.80));
        assert_eq!(dual_pane_widths(960, -9.9), dual_pane_widths(960, 0.20));
        assert_eq!(dual_pane_widths(960, f32::NAN), (460, 460));
        assert_eq!(dual_pane_widths(30, 0.50), (0, 0));
        assert_eq!(dual_pane_widths(0, 0.50), (0, 0));
    }

    #[test]
    fn dual_pane_positions_align_with_widths() {
        assert_eq!(dual_pane_x_positions(960, 0.50), (12, 476, 936));
        assert_eq!(dual_pane_x_positions(720, 0.50), (12, 356, 696));
        let (left_x, right_x, gutter_x) = dual_pane_x_positions(960, 0.25);
        let (left_w, right_w) = dual_pane_widths(960, 0.25);
        assert_eq!(right_x, left_x.saturating_add(left_w).saturating_add(4));
        assert_eq!(gutter_x, right_x.saturating_add(right_w));
        assert_eq!(dual_pane_x_positions(30, 0.50), (12, 12, 12));
        assert_eq!(dual_pane_x_positions(0, 0.50), (12, 12, 12));
    }

    #[test]
    fn virtual_row_window_clamps_for_show_rows() {
        assert_eq!(virtual_row_window(100, 0, 20), (0, 20));
        assert_eq!(virtual_row_window(100, 90, 20), (90, 100));
        assert_eq!(virtual_row_window(100, 200, 20), (100, 100));
        assert_eq!(virtual_row_window(0, 0, 20), (0, 0));
        assert_eq!(virtual_row_window(100, 0, 0), (0, 0));
        assert_eq!(virtual_row_window(15, 10, 20), (10, 15));
    }

    #[test]
    fn clip_text_end_ellipsis_and_boundaries() {
        assert_eq!(clip_text_end("abc", 10), "abc");
        assert_eq!(clip_text_end("abcdef", 6), "abcdef");
        assert_eq!(clip_text_end("abcdefg", 6), "abc...");
        assert_eq!(clip_text_end("abcdef", 3), "abc");
        assert_eq!(clip_text_end("abcdef", 0), "");
        assert_eq!(clip_text_end("caf\u{e9} \u{1f980}rust", 7), "caf\u{e9}...");
        assert_eq!(clip_text_end("caf\u{e9}", 4), "caf\u{e9}");
    }

    #[test]
    fn clip_to_width_keeps_longest_fitting_label() {
        let measure = |s: &str| s.len() as u32;
        assert_eq!(clip_to_width("abc", 10, measure), "abc");
        assert_eq!(clip_to_width("abcdef", 6, measure), "abcdef");
        assert_eq!(clip_to_width("abcdef", 5, measure), "ab...");
        assert_eq!(clip_to_width("abcdef", 3, measure), "...");
        assert_eq!(clip_to_width("abcdef", 2, measure), "");
        assert_eq!(clip_to_width("", 0, measure), "");
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

    // ---- status bar + cursor (BT-STATUS P1-2/P1-3) ----

    #[test]
    fn status_filter_labels_cover_every_filter() {
        assert_eq!(text_filter_label(TextFilter::All), "All");
        assert_eq!(text_filter_label(TextFilter::Differences), "Differences");
        assert_eq!(text_filter_label(TextFilter::Same), "Same");
        assert_eq!(text_filter_label(TextFilter::Context), "Context");
    }

    #[test]
    fn status_kind_labels_match_verdicts() {
        assert_eq!(text_row_kind_label(TextRowKind::Equal), "Same");
        assert_eq!(text_row_kind_label(TextRowKind::Changed), "Changed");
        assert_eq!(text_row_kind_label(TextRowKind::Deleted), "Deleted");
        assert_eq!(text_row_kind_label(TextRowKind::Inserted), "Inserted");
    }

    #[test]
    fn status_focused_kind_is_honest_on_empty_selection() {
        let rows = nav_rows();
        assert_eq!(focused_kind_label(&rows, None), "no selection");
        assert_eq!(focused_kind_label(&rows, Some(99)), "no selection");
        assert_eq!(focused_kind_label(&rows, Some(0)), "Same");
        assert_eq!(focused_kind_label(&rows, Some(1)), "Changed");
        assert_eq!(focused_kind_label(&rows, Some(4)), "Deleted");
    }

    #[test]
    fn status_cursor_labels_row_col_one_based() {
        assert_eq!(cursor_pos_for(None), None);
        assert_eq!(cursor_pos_for(Some(0)), Some(CursorPos { row: 1, col: 1 }));
        assert_eq!(cursor_pos_for(Some(2)), Some(CursorPos { row: 3, col: 1 }));
        assert_eq!(cursor_label(cursor_pos_for(Some(2))), "3:1");
        assert_eq!(cursor_label(None), "-");
    }

    #[test]
    fn status_zones_compose_left_mid_right() {
        let rows = nav_rows();
        let counts = filter_counts(&rows, TextFilter::Differences);
        let zones = status_zones_for(&counts, TextFilter::Differences, "Changed", "2/3");
        assert_eq!(zones.left, "Differences: 5 rows (3 shown)");
        assert_eq!(zones.mid, "Changed");
        assert!(zones.right.contains("2/3"));
        assert!(zones.right.contains("F3"));
    }

    #[test]
    fn status_zone_roles_keep_verdict_above_sides() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_SMALL_SIZE, TypoRole};
        assert_eq!(status_verdict_text_role(), TypoRole::Body);
        assert_eq!(status_verdict_text_role().size(), TYPO_BODY_SIZE);
        assert_eq!(status_side_text_role(), TypoRole::Small);
        assert_eq!(status_side_text_role(), TypoRole::caption());
        assert_eq!(status_side_text_role().size(), TYPO_SMALL_SIZE);
        // Hierarchy: verdict (Body 13) sides (Small 11) se ek step upar;
        // sides tabular intent rakhte hain (counts stable).
        assert!(status_verdict_text_role().size() > status_side_text_role().size());
        assert!(status_side_text_role().is_tabular());
    }

    #[test]
    fn model9_empty_model_is_honest() {
        let rows: Vec<TextRow> = Vec::new();
        assert_eq!(sanitize_selection(0, None), None);
        assert_eq!(sanitize_selection(0, Some(0)), None);
        assert_eq!(initial_selection_for(&rows), None);
        assert_eq!(diff_nav_total_for(&rows), 0);
        assert_eq!(diff_nav_position_for(&rows, None), None);
        assert_eq!(diff_nav_position_for(&rows, Some(0)), None);
        assert_eq!(diff_nav_badge_for(&rows, None), "0/0");
        assert_eq!(diff_nav_badge_for(&rows, Some(3)), "0/0");
        assert_eq!(pane_top_for_selection(0, 20, None, 0), 0);
        assert_eq!(pane_top_for_selection(5, 20, Some(3), 0), 0);
        assert_eq!(pane_top_for_selection(0, 0, Some(0), 5), 0);
        assert_eq!(cursor_label(cursor_pos_for(None)), "-");
        assert_eq!(focused_kind_label(&rows, None), "no selection");
        assert_eq!(focused_line_preview_for(&rows, None), None);
    }

    #[test]
    fn model9_loaded_selection_drives_pane_top_and_badge() {
        let rows = nav_rows();
        // OOB sanitize hota hai, andar wala untouched rehta hai.
        assert_eq!(sanitize_selection(rows.len(), Some(99)), None);
        assert_eq!(sanitize_selection(rows.len(), Some(4)), Some(4));
        assert_eq!(sanitize_selection(rows.len(), None), None);
        // Pehli diff se khulta hai; equal-only/empty par None.
        assert_eq!(initial_selection_for(&rows), Some(1));
        let flat = vec![TextRow::default()];
        assert_eq!(initial_selection_for(&flat), None);
        // Badge: equal par fake pos nahi, tail diff par "2/3".
        assert_eq!(diff_nav_total_for(&rows), 3);
        assert_eq!(diff_nav_position_for(&rows, Some(0)), None);
        assert_eq!(diff_nav_position_for(&rows, Some(1)), Some(1));
        assert_eq!(diff_nav_position_for(&rows, Some(4)), Some(3));
        assert_eq!(diff_nav_position_for(&rows, Some(99)), None);
        assert_eq!(diff_nav_badge_for(&rows, None), "0/3");
        assert_eq!(diff_nav_badge_for(&rows, Some(1)), "1/3");
        assert_eq!(diff_nav_badge_for(&rows, Some(4)), "3/3");
        // Pane top: per_page 2, top 0, tail selected -> top 3 (window (3,5)).
        assert_eq!(pane_top_for_selection(0, 2, Some(4), 5), 3);
        // Visible rehta hai to untouched; upar jump par pehli row.
        assert_eq!(pane_top_for_selection(3, 2, Some(4), 5), 3);
        assert_eq!(pane_top_for_selection(3, 2, Some(3), 5), 3);
        assert_eq!(pane_top_for_selection(0, 2, Some(0), 5), 0);
        assert_eq!(pane_top_for_selection(3, 2, Some(1), 5), 1);
        assert_eq!(pane_top_for_selection(9, 2, None, 5), 5);
    }

    #[test]
    fn model9_cursor_roles_reuse_existing_sizes() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_MONO_SIZE};
        assert_eq!(cursor_strip_head_role(), crate::theme::TypoRole::Body);
        assert_eq!(cursor_strip_head_role().size(), TYPO_BODY_SIZE);
        assert_eq!(cursor_strip_preview_role(), crate::theme::TypoRole::Mono);
        assert_eq!(cursor_strip_preview_role().size(), TYPO_MONO_SIZE);
        // Hierarchy: head (Body 13) preview (Mono 12) se ek step upar;
        // preview tabular rehta hai (diff text stable).
        assert!(cursor_strip_head_role().size() > cursor_strip_preview_role().size());
        assert!(cursor_strip_preview_role().is_tabular());
    }

    #[test]
    fn status_preview_covers_number_shapes() {
        let rows = nav_rows();
        assert_eq!(focused_line_preview_for(&rows, None), None);
        assert_eq!(focused_line_preview_for(&rows, Some(99)), None);
        // Bina numbers wali equal row: line_no "-" hota hai.
        let plain = focused_line_preview_for(&rows, Some(0)).unwrap_or_default();
        assert_eq!(plain.line_no, "-");
        assert_eq!(plain.kind, "Same");
        // Dono taraf alag numbers: "L3 / R5".
        let both = vec![TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(5),
            left: "aaa".to_string(),
            right: "bbb".to_string(),
            ..Default::default()
        }];
        let preview = focused_line_preview_for(&both, Some(0)).unwrap_or_default();
        assert_eq!(preview.line_no, "L3 / R5");
        assert_eq!(preview.kind, "Changed");
        assert_eq!(preview.left, "aaa");
        // Dono taraf barabar: sirf number.
        let same_no = vec![TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(3),
            right_no: Some(3),
            ..Default::default()
        }];
        assert_eq!(
            focused_line_preview_for(&same_no, Some(0))
                .unwrap_or_default()
                .line_no,
            "3"
        );
        // Sirf ek taraf: "L7" / "R7".
        let left_only = vec![TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(7),
            ..Default::default()
        }];
        assert_eq!(
            focused_line_preview_for(&left_only, Some(0))
                .unwrap_or_default()
                .line_no,
            "L7"
        );
        let right_only = vec![TextRow {
            kind: TextRowKind::Inserted,
            right_no: Some(7),
            right: "new".to_string(),
            ..Default::default()
        }];
        assert_eq!(
            focused_line_preview_for(&right_only, Some(0))
                .unwrap_or_default()
                .line_no,
            "R7"
        );
    }

    #[test]
    fn status_preview_cuts_long_lines_at_cap() {
        let long: String = "x".repeat(STATUS_PREVIEW_MAX_CHARS + 10);
        let rows = vec![TextRow {
            kind: TextRowKind::Equal,
            left: long,
            right: "ok".to_string(),
            ..Default::default()
        }];
        let cut = focused_line_preview_for(&rows, Some(0)).unwrap_or_default();
        assert!(cut.left.ends_with("..."));
        assert_eq!(cut.left.chars().count(), STATUS_PREVIEW_MAX_CHARS + 3);
        assert_eq!(cut.right, "ok");
        // Exact cap par koi suffix nahi.
        let exact: String = "y".repeat(STATUS_PREVIEW_MAX_CHARS);
        let exact_rows = vec![TextRow {
            kind: TextRowKind::Equal,
            left: exact.clone(),
            ..Default::default()
        }];
        let kept = focused_line_preview_for(&exact_rows, Some(0)).unwrap_or_default();
        assert_eq!(kept.left, exact);
    }

    #[test]
    fn exq_l_zone_roles_keep_label_above_value_and_badge() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_MONO_SIZE, TYPO_SMALL_SIZE, TypoRole};
        assert_eq!(hex_goto_zone_label_role(), TypoRole::Body);
        assert_eq!(hex_goto_zone_value_role(), TypoRole::Mono);
        assert_eq!(hex_goto_zone_badge_role(), TypoRole::Small);
        assert_eq!(table_key_zone_label_role(), TypoRole::Body);
        assert_eq!(table_key_zone_value_role(), TypoRole::Mono);
        assert_eq!(table_sheet_zone_label_role(), TypoRole::Body);
        assert_eq!(table_sheet_zone_value_role(), TypoRole::Mono);
        assert_eq!(zone_badge_role(), TypoRole::Small);
        assert_eq!(hex_goto_zone_label_role().size(), TYPO_BODY_SIZE);
        assert_eq!(hex_goto_zone_value_role().size(), TYPO_MONO_SIZE);
        assert_eq!(zone_badge_role().size(), TYPO_SMALL_SIZE);
        // Hierarchy: label(13) > value(12) > badge(11); value tabular hai.
        assert!(hex_goto_zone_label_role().size() > hex_goto_zone_value_role().size());
        assert!(hex_goto_zone_value_role().size() > zone_badge_role().size());
        assert!(zone_is_tabular(hex_goto_zone_value_role()));
        assert!(zone_is_tabular(zone_badge_role()));
        assert!(!zone_is_tabular(hex_goto_zone_label_role()));
        // Family/weight/spacing aliases TypoRole se drift nahi karte.
        assert_eq!(zone_family(TypoRole::Mono), "monospace");
        assert_eq!(zone_family(TypoRole::Body), "proportional");
        assert_eq!(zone_weight(TypoRole::Body), TypoRole::Body.weight());
        assert_eq!(
            zone_line_height_px(TypoRole::Mono),
            TypoRole::Mono.line_height_px()
        );
        assert_eq!(
            zone_letter_spacing_em(TypoRole::Heading),
            TypoRole::Heading.letter_spacing_em()
        );
    }

    #[test]
    fn pathmeta_second_line_joins_nonempty_parts_only() {
        assert_eq!(
            format_path_pane_meta("2026-01-02 03:04", "10 B", "Text: exact", "wrap:off"),
            "2026-01-02 03:04 | 10 B | Text: exact | wrap:off"
        );
        assert_eq!(format_path_pane_meta("", "10 B", "", ""), "10 B");
        assert_eq!(format_path_pane_meta("  ", " ", "", ""), "");
        let meta = PathPaneMeta {
            path: "fixtures/left.rs".to_string(),
            timestamp: "2026-01-02 03:04".to_string(),
            bytes: "10 B".to_string(),
            format: "Text: exact".to_string(),
            rules: String::new(),
        };
        assert_eq!(
            path_pane_meta_line(&meta),
            "2026-01-02 03:04 | 10 B | Text: exact"
        );
        assert_eq!(path_pane_meta_line(&PathPaneMeta::default()), "");
    }

    #[test]
    fn pathmeta_history_label_stays_honest_on_empty() {
        assert_eq!(path_history_label(""), "history");
        assert_eq!(path_history_label("   "), "history");
        assert_eq!(path_history_label("fixtures/left.rs"), "fixtures/left.rs");
    }

    #[test]
    fn pathmeta_segmented_line_drops_empty_segments() {
        let full = path_segmented_status_for("3:1", "Text: exact", "4 equal", "edit: clean");
        assert_eq!(
            path_segmented_status_line(&full),
            "3:1 | Text: exact | 4 equal | edit: clean"
        );
        let partial = path_segmented_status_for("3:1", "", "4 equal", "");
        assert_eq!(path_segmented_status_line(&partial), "3:1 | 4 equal");
        assert_eq!(
            path_segmented_status_line(&PathSegmentedStatus::default()),
            "-"
        );
    }

    #[test]
    fn pathmeta_roles_reuse_existing_tokens() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_SMALL_SIZE, TypoRole};
        assert_eq!(path_strip_path_role(), TypoRole::Body);
        assert_eq!(path_strip_meta_role(), TypoRole::caption());
        assert_eq!(path_segment_cursor_role(), TypoRole::Body);
        assert_eq!(path_segment_side_role(), TypoRole::caption());
        assert_eq!(path_strip_path_role().size(), TYPO_BODY_SIZE);
        assert_eq!(path_strip_meta_role().size(), TYPO_SMALL_SIZE);
        assert!(path_segment_cursor_role().size() > path_segment_side_role().size());
    }

    #[test]
    fn pathmeta_edge_empty_vs_long_clips_honestly() {
        // Edge: empty left (honest-empty meta) vs very long right path
        // (ellipsis, never raw mid-glyph cut).
        let empty = PathPaneMeta::default();
        assert_eq!(path_pane_meta_line(&empty), "");
        assert_eq!(path_history_label(&empty.path), "history");
        let long_path = "fixtures/very/long/nested/dir/structure/that/keeps/going/right.rs";
        let stub = |s: &str| s.chars().count() as u32 * 6;
        let clipped = clip_to_width(long_path, 300, stub);
        assert!(clipped.ends_with("..."));
        assert!(stub(&clipped) <= 300);
        // Segmented edge: sirf cursor ho to bhi line honest rehti hai.
        let cursor_only = path_segmented_status_for("L- R5", "", "", "");
        assert_eq!(path_segmented_status_line(&cursor_only), "L- R5");
    }

    // ---- text-center exquisite geometry (headless) ----

    #[test]
    fn textcenter_gutter_reuses_space_scale() {
        use crate::theme::{
            CHROME_HEADER_BAR_H, GEO_TIGHT_GAP, GUTTER_BAR_WIDE, TEXT_CENTER_GUTTER_W,
        };
        // 28 + 12 + 4 + 4 = 48: do chhote copy buttons fit, koi naya literal nahi.
        assert_eq!(TEXT_CENTER_GUTTER_W, 48.0);
        assert_eq!(text_center_gutter_w(), TEXT_CENTER_GUTTER_W);
        assert_eq!(
            text_center_gutter_w(),
            CHROME_HEADER_BAR_H + GUTTER_BAR_WIDE + GEO_TIGHT_GAP + GEO_TIGHT_GAP
        );
    }

    #[test]
    fn textcenter_pane_widths_split_free_budget() {
        // Chrome 88 (40 dual + 48 center): 960 -> 436/436, 720 -> 316/316.
        assert_eq!(textcenter_chrome_w(), 88);
        assert_eq!(textcenter_pane_widths(960, 0.5), (436, 436));
        assert_eq!(textcenter_pane_widths(720, 0.5), (316, 316));
        // Clamp: fraction bahar ho to 20/80 edge par tikta hai.
        let (lo_l, lo_r) = textcenter_pane_widths(960, 0.0);
        assert_eq!(lo_l, 174);
        assert_eq!(lo_l.saturating_add(lo_r), 872);
        // Degenerate: chrome se tang window par (0, 0) — paint stacks.
        assert_eq!(textcenter_pane_widths(80, 0.5), (0, 0));
        assert_eq!(textcenter_pane_widths(88, 0.5), (0, 0));
    }

    #[test]
    fn textcenter_x_positions_pin_gutter_between_panes() {
        // 960: left 12, gutter 12+436+4=452, right 452+48+4=504.
        assert_eq!(textcenter_x_positions(960, 0.5), (12, 452, 504));
        // 720 fallback: left 12, gutter 12+316+4=332, right 332+48+4=384.
        assert_eq!(textcenter_x_positions(720, 0.5), (12, 332, 384));
        // Order invariant: gutter hamesha panes ke beech.
        let (lx, gx, rx) = textcenter_x_positions(960, 0.3);
        assert!(lx < gx && gx < rx);
        // Degenerate: stacked origin, kabhi underflow nahi.
        assert_eq!(textcenter_x_positions(80, 0.5), (12, 12, 12));
        assert_eq!(textcenter_x_positions(0, 0.5), (12, 12, 12));
    }

    #[test]
    fn textcenter_fill_rows_fills_viewport_not_content() {
        // 6 rows of 24px in a 600px viewport: 25 slots - 6 = 19 ruled blanks.
        assert_eq!(textcenter_fill_rows(600.0, 24.0, 6), 19);
        // Full/overflow: zero, kabhi negative nahi.
        assert_eq!(textcenter_fill_rows(600.0, 24.0, 25), 0);
        assert_eq!(textcenter_fill_rows(600.0, 24.0, 40), 0);
        assert_eq!(textcenter_fill_rows(100.0, 24.0, 0), 5);
        // Guards: degenerate budgets yield 0, kabhi panic/NaN nahi.
        assert_eq!(textcenter_fill_rows(0.0, 24.0, 6), 0);
        assert_eq!(textcenter_fill_rows(600.0, 0.0, 6), 0);
        assert_eq!(textcenter_fill_rows(f32::NAN, 24.0, 6), 0);
        assert_eq!(textcenter_fill_rows(600.0, f32::NAN, 6), 0);
        assert_eq!(textcenter_fill_rows(f32::INFINITY, 24.0, 6), 0);
    }

    #[test]
    fn textcenter_hunk_head_marks_block_starts_only() {
        let diff = |kind: TextRowKind| TextRow {
            kind,
            ..TextRow::default()
        };
        let rows = vec![
            diff(TextRowKind::Equal),
            diff(TextRowKind::Changed),
            diff(TextRowKind::Changed),
            diff(TextRowKind::Deleted),
            diff(TextRowKind::Equal),
        ];
        assert!(!textcenter_hunk_head(&rows, 0));
        assert!(textcenter_hunk_head(&rows, 1));
        assert!(!textcenter_hunk_head(&rows, 2));
        // Index 3 head nahi: Changed+Deleted contiguous ek hi section hai.
        assert!(!textcenter_hunk_head(&rows, 3));
        assert!(!textcenter_hunk_head(&rows, 4));
        // Alag blocks (Equal separator) me do heads.
        let split = vec![
            diff(TextRowKind::Changed),
            diff(TextRowKind::Equal),
            diff(TextRowKind::Inserted),
        ];
        assert!(textcenter_hunk_head(&split, 0));
        assert!(!textcenter_hunk_head(&split, 1));
        assert!(textcenter_hunk_head(&split, 2));
        // OOB kabhi head nahi.
        assert!(!textcenter_hunk_head(&rows, 99));
        assert!(!textcenter_hunk_head(&[], 0));
    }

    #[test]
    fn textcenter_accent_spans_only_changed_rows_carry_them() {
        use crate::theme::TEXT_INLINE_SPAN_UNDERLINE_W;
        // Underline 1px visual precedent; spans char-hugging, zero pad.
        assert_eq!(TEXT_INLINE_SPAN_UNDERLINE_W, 1.0);
        let changed = TextRow {
            kind: TextRowKind::Changed,
            accent_a: vec![(10, 15)],
            accent_b: vec![(10, 13)],
            ..TextRow::default()
        };
        let (a, b) = textcenter_accent_spans(&changed);
        assert_eq!(a, &[(10, 15)]);
        assert_eq!(b, &[(10, 13)]);
        for kind in [
            TextRowKind::Equal,
            TextRowKind::Deleted,
            TextRowKind::Inserted,
        ] {
            let row = TextRow {
                kind,
                accent_a: vec![(0, 2)],
                accent_b: vec![(0, 2)],
                ..TextRow::default()
            };
            let (a, b) = textcenter_accent_spans(&row);
            assert!(a.is_empty() && b.is_empty());
        }
    }

    // ---- text-center edge fixtures ----

    #[test]
    fn textcenter_edge_empty_vs_nonempty_is_all_inserted() {
        // Edge fixture: empty left vs 3-line right — sab Inserted,
        // right-only numbers, gutter floor 4, pane right se banta hai.
        let cmp = cmp_text(
            &[],
            &["alpha", "beta", "gamma"],
            vec![Op::Insert { len: 3 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 3);
        for (i, row) in model.rows.iter().enumerate() {
            assert_eq!(row.kind, TextRowKind::Inserted);
            assert_eq!(row.left_no, None);
            assert_eq!(row.right_no, Some(i + 1));
            assert!(row.left.is_empty());
        }
        assert_eq!(model.pane_chars, 5); // "gamma"
        assert_eq!(model.gutter_chars, 4);
        // Viewport fill: 3 rows visible hon to bhi viewport bharta hai.
        assert!(textcenter_fill_rows(600.0, 24.0, model.rows.len()) > 0);
        // Center gutter: inserted block ka head sirf pehla row.
        assert!(textcenter_hunk_head(&model.rows, 0));
        assert!(!textcenter_hunk_head(&model.rows, 1));
    }

    #[test]
    fn textcenter_edge_tail_insertion_pairs_numbers_honestly() {
        // Edge fixture: equal head + tail insertion — pehle Equal with
        // paired numbers, phir Inserted right-only; accents kahin nahi.
        let cmp = cmp_text(
            &["alpha", "beta"],
            &["alpha", "beta", "gamma", "delta"],
            vec![Op::Equal { len: 2 }, Op::Insert { len: 2 }],
            true,
        );
        let model = build_text_rows(&cmp);
        assert_eq!(model.rows.len(), 4);
        for (i, row) in model.rows.iter().take(2).enumerate() {
            assert_eq!(row.kind, TextRowKind::Equal);
            assert_eq!(row.left_no, Some(i + 1));
            assert_eq!(row.right_no, Some(i + 1));
        }
        for (i, row) in model.rows.iter().skip(2).enumerate() {
            assert_eq!(row.kind, TextRowKind::Inserted);
            assert_eq!(row.left_no, None);
            assert_eq!(row.right_no, Some(i + 3));
            let (a, b) = textcenter_accent_spans(row);
            assert!(a.is_empty() && b.is_empty());
        }
        // Tail block head: index 2; gutter x-order bana rehta hai.
        assert!(textcenter_hunk_head(&model.rows, 2));
        let (lx, gx, rx) = textcenter_x_positions(960, 0.5);
        assert!(lx < gx && gx < rx);
    }

    // ---- viewmodel density + row-height tests (MODEL10) ----

    #[test]
    fn density_tokens_strictly_greater_than_line_heights() {
        use crate::theme::{CHROME_HEADER_BAR_H, SPACE_XL, TYPO_MONO_SIZE_MAX, TypoRole};
        use crate::tree::FOLDER_ROW_HEIGHT;

        // Text row heights strictly exceed mono line heights (no vertical clip)
        let mono_line_h = TypoRole::Mono.line_height_px(); // 15.6
        let mono_max_line_h = TYPO_MONO_SIZE_MAX * TypoRole::Mono.line_height(); // 16.9
        assert!(TEXT_ROW_HEIGHT_COMPACT > mono_line_h);
        assert!(TEXT_ROW_HEIGHT_COMPACT > mono_max_line_h);
        assert!(TEXT_ROW_HEIGHT_COMFORTABLE > TEXT_ROW_HEIGHT_COMPACT);
        assert!(TEXT_ROW_HEIGHT_SPACIOUS > TEXT_ROW_HEIGHT_COMFORTABLE);

        // Folder row heights strictly exceed body line height (no vertical clip)
        let body_line_h = TypoRole::Body.line_height_px(); // 18.2
        assert!(FOLDER_ROW_HEIGHT_COMPACT > body_line_h);
        assert!(FOLDER_ROW_HEIGHT_COMFORTABLE > FOLDER_ROW_HEIGHT_COMPACT);
        assert!(FOLDER_ROW_HEIGHT_SPACIOUS > FOLDER_ROW_HEIGHT_COMFORTABLE);

        // Parity with existing design tokens
        assert_eq!(TEXT_ROW_HEIGHT_COMFORTABLE, SPACE_XL);
        assert_eq!(TEXT_ROW_HEIGHT_COMFORTABLE, 24.0);
        assert_eq!(FOLDER_ROW_HEIGHT_COMFORTABLE, FOLDER_ROW_HEIGHT);
        assert_eq!(FOLDER_ROW_HEIGHT_COMFORTABLE, 24.0);
        assert_eq!(TEXT_ROW_HEIGHT_SPACIOUS, CHROME_HEADER_BAR_H);
        assert_eq!(TEXT_ROW_HEIGHT_SPACIOUS, 28.0);
        assert_eq!(FOLDER_ROW_HEIGHT_SPACIOUS, 28.0);
    }

    #[test]
    fn density_enum_methods_and_labels() {
        assert_eq!(RowDensity::default(), RowDensity::Comfortable);
        assert_eq!(RowDensity::Compact.label(), "Compact");
        assert_eq!(RowDensity::Comfortable.label(), "Comfortable");
        assert_eq!(RowDensity::Spacious.label(), "Spacious");

        assert_eq!(
            RowDensity::Compact.text_row_height(),
            TEXT_ROW_HEIGHT_COMPACT
        );
        assert_eq!(
            RowDensity::Comfortable.text_row_height(),
            TEXT_ROW_HEIGHT_COMFORTABLE
        );
        assert_eq!(
            RowDensity::Spacious.text_row_height(),
            TEXT_ROW_HEIGHT_SPACIOUS
        );

        assert_eq!(
            RowDensity::Compact.folder_row_height(),
            FOLDER_ROW_HEIGHT_COMPACT
        );
        assert_eq!(
            RowDensity::Comfortable.folder_row_height(),
            FOLDER_ROW_HEIGHT_COMFORTABLE
        );
        assert_eq!(
            RowDensity::Spacious.folder_row_height(),
            FOLDER_ROW_HEIGHT_SPACIOUS
        );

        // Standalone functions
        assert_eq!(
            text_row_height_for_density(RowDensity::Compact),
            TEXT_ROW_HEIGHT_COMPACT
        );
        assert_eq!(
            folder_row_height_for_density(RowDensity::Compact),
            FOLDER_ROW_HEIGHT_COMPACT
        );

        // Text padding budgets
        assert!((RowDensity::Compact.text_row_pad() - 1.2).abs() < 0.01);
        assert!((RowDensity::Comfortable.text_row_pad() - 4.2).abs() < 0.01);
        assert!((RowDensity::Spacious.text_row_pad() - 6.2).abs() < 0.01);

        // Folder padding budgets
        assert!((RowDensity::Compact.folder_row_pad() - 0.9).abs() < 0.01);
        assert!((RowDensity::Comfortable.folder_row_pad() - 2.9).abs() < 0.01);
        assert!((RowDensity::Spacious.folder_row_pad() - 4.9).abs() < 0.01);
    }

    #[test]
    fn clip_risk_detector_and_min_safe_height() {
        // row_h >= line_h -> no clip risk
        assert!(!row_height_has_clip_risk(18.0, 15.6));
        assert!(!row_height_has_clip_risk(24.0, 18.2));
        assert!(!row_height_has_clip_risk(18.2, 18.2));

        // row_h < line_h -> clip risk
        assert!(row_height_has_clip_risk(15.0, 15.6));
        assert!(row_height_has_clip_risk(18.0, 18.2));

        // Degenerate inputs report clip risk defensively
        assert!(row_height_has_clip_risk(f32::NAN, 15.6));
        assert!(row_height_has_clip_risk(18.0, f32::NAN));
        assert!(row_height_has_clip_risk(0.0, 15.6));
        assert!(row_height_has_clip_risk(-5.0, 15.6));
        assert!(row_height_has_clip_risk(18.0, -1.0));

        // min_safe_row_height_for_line adds 1.0px floor
        assert_eq!(min_safe_row_height_for_line(15.6), 16.6);
        assert_eq!(min_safe_row_height_for_line(18.2), 19.2);
        assert_eq!(min_safe_row_height_for_line(0.0), 0.0);
        assert_eq!(min_safe_row_height_for_line(-4.0), 0.0);
        assert_eq!(min_safe_row_height_for_line(f32::NAN), 0.0);
    }

    #[test]
    fn viewport_capacity_and_fill_rows() {
        let panes_h = 476.0;

        // Text capacities
        assert_eq!(
            RowDensity::Compact.text_viewport_capacity(panes_h),
            26 // 476 / 18 = 26.44
        );
        assert_eq!(
            RowDensity::Comfortable.text_viewport_capacity(panes_h),
            19 // 476 / 24 = 19.83
        );
        assert_eq!(
            RowDensity::Spacious.text_viewport_capacity(panes_h),
            17 // 476 / 28 = 17.0
        );

        // Folder capacities
        assert_eq!(
            RowDensity::Compact.folder_viewport_capacity(panes_h),
            23 // 476 / 20 = 23.8
        );
        assert_eq!(
            RowDensity::Comfortable.folder_viewport_capacity(panes_h),
            19 // 476 / 24 = 19.83
        );
        assert_eq!(
            RowDensity::Spacious.folder_viewport_capacity(panes_h),
            17 // 476 / 28 = 17.0
        );

        // Degenerate capacities yield 0
        assert_eq!(RowDensity::Compact.text_viewport_capacity(0.0), 0);
        assert_eq!(RowDensity::Compact.text_viewport_capacity(-10.0), 0);
        assert_eq!(RowDensity::Compact.text_viewport_capacity(f32::NAN), 0);

        // Fill rows helper
        assert_eq!(
            RowDensity::Comfortable.text_fill_rows(600.0, 6),
            textcenter_fill_rows(600.0, 24.0, 6)
        );

        // Text pane char capacity
        assert_eq!(text_pane_char_capacity(460, 7.2), 63);
        assert_eq!(text_pane_char_capacity(340, 7.2), 47);
        assert_eq!(text_pane_char_capacity(0, 7.2), 0);
        assert_eq!(text_pane_char_capacity(460, 0.0), 0);
        assert_eq!(text_pane_char_capacity(460, f32::NAN), 0);
    }

    #[test]
    fn audit_viewmodel_density_proves_zero_clipping_at_960_and_720() {
        let audit = audit_viewmodel_density();
        assert!(audit.zero_clipping_verified);

        // Widths
        assert_eq!(audit.canonical_text_pane_w, 460);
        assert_eq!(audit.fallback_text_pane_w, 340);
        assert_eq!(audit.canonical_textcenter_pane_w, 436);
        assert_eq!(audit.fallback_textcenter_pane_w, 316);

        // Folder clearances
        let min_path_w = crate::tree::folder_column_min_px(crate::tree::FolderColumn::Path);
        assert!(audit.canonical_folder_path_w_d0 >= min_path_w);
        assert!(audit.fallback_folder_path_w_d0 >= min_path_w);
        assert!(audit.fallback_folder_path_w_d3 >= min_path_w);

        // Capacities
        assert_eq!(audit.text_capacity_600, [26, 19, 17]);
        assert_eq!(audit.text_capacity_450, [18, 13, 11]);
        assert_eq!(audit.folder_capacity_600, [23, 19, 17]);
        assert_eq!(audit.folder_capacity_450, [16, 13, 11]);
    }
}
