//! Table key-compare view model: headless-first pure state over
//! the key-aligned tolerant table engine.
//!
//! Observed behavior parity (clean-room, no copied strings): a table
//! session parses two delimited files, suggests key columns whose
//! cells are non-empty and unique, aligns rows by the selected key,
//! and compares data cells with numeric tolerance, case folding and
//! space trimming. With no key columns the rows compare positionally
//! and the view says so honestly. This module holds that state without
//! egui or I/O so it stays unit-testable; painting and engine driving
//! stay the orchestrator's wiring job.
//!
//! Engine mapping: [`TableKeyView::to_compare_options`] builds the
//! [`autocompare_diff::TableCompareOptions`] that
//! `diff_table_by_key_with_options` consumes, and
//! [`TableKeyView::to_key_options`] builds the
//! [`autocompare_compare::TableKeyOptions`] that
//! `compare_table_files_by_key` consumes.

#![forbid(unsafe_code)]

use autocompare_compare::TableKeyOptions;
use autocompare_diff::{
    Table, TableCompareOptions, TableKeyDiff, cells_equal_with_options, detect_key_columns,
    diff_table_by_key_with_options,
};
use eframe::egui;

/// Note shown when no key columns are selected: rows compare
/// positionally, so reordered rows read as deletes plus inserts.
pub const POSITIONAL_NOTE: &str = "no key columns: rows compare positionally";

/// Honest summary counts of one key-aligned table diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableKeySummary {
    /// Rows whose key matched and every data cell matched.
    pub matching: usize,
    /// Rows whose key matched but at least one data cell differed.
    pub modified: usize,
    /// Rows present only on the left.
    pub left_only: usize,
    /// Rows present only on the right.
    pub right_only: usize,
    /// Whether rows aligned by key (false when positional fallback).
    pub keyed: bool,
    /// Whether the positional fallback note applies.
    pub positional_note: bool,
}

impl TableKeySummary {
    /// Total aligned rows behind this summary.
    pub fn total(&self) -> usize {
        self.matching
            .saturating_add(self.modified)
            .saturating_add(self.left_only)
            .saturating_add(self.right_only)
    }

    /// One-line counts mirroring the CLI's honest wording.
    pub fn line(&self) -> String {
        format!(
            "{} matching, {} modified, {} left-only, {} right-only{}",
            self.matching,
            self.modified,
            self.left_only,
            self.right_only,
            if self.keyed {
                " (keyed)"
            } else {
                " (positional)"
            }
        )
    }
}

/// Headless table key-compare state: selector plus tolerant options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableKeyView {
    /// Selected alignment key columns (empty = positional compare).
    pub key_columns: Vec<usize>,
    /// Numeric tolerance; invalid values sanitize to [`None`].
    pub numeric_tolerance: Option<f64>,
    /// Fold case before comparing text and keys.
    pub ignore_case: bool,
    /// Trim surrounding space before comparing text and keys.
    pub trim_space: bool,
    /// Last key suggestion from [`detect_key_columns`].
    pub suggested_keys: Vec<usize>,
    /// Summary of the last diff run.
    pub summary: TableKeySummary,
}

impl TableKeyView {
    /// Fresh view: positional compare with exact text equality.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sanitizes a tolerance the way the engine does: only finite
    /// values `>= 0` compare numerically, anything else (including
    /// `NaN`, infinite and negative) means exact text equality.
    pub fn sanitize_tolerance(tolerance: Option<f64>) -> Option<f64> {
        match tolerance {
            Some(t) if t.is_finite() && t >= 0.0 => Some(t),
            _ => None,
        }
    }

    /// Sets the numeric tolerance after sanitizing it.
    pub fn set_tolerance(&mut self, tolerance: Option<f64>) {
        self.numeric_tolerance = Self::sanitize_tolerance(tolerance);
    }

    /// Toggles one column in the key selector, keeping it sorted.
    pub fn toggle_key_column(&mut self, column: usize) {
        let mut next: Vec<usize> = Vec::new();
        let mut removed = false;
        for existing in self.key_columns.iter() {
            if *existing == column {
                removed = true;
            } else {
                next.push(*existing);
            }
        }
        if !removed {
            next.push(column);
            next.sort();
        }
        self.key_columns = next;
    }

    /// Refreshes [`Self::suggested_keys`] from the left table. Test-only:
    /// the file-driven wiring never holds parsed tables (engine consumes
    /// paths), so only unit tests drive the headless suggestion path.
    #[allow(dead_code)]
    pub fn suggest_for(&mut self, left: &Table) {
        self.suggested_keys = detect_key_columns(left);
    }

    /// Adopts the last suggestion as the selected key. Test-only: see
    /// [`Self::suggest_for`]; the file-driven wiring toggles key columns
    /// manually via [`Self::toggle_key_column`].
    #[allow(dead_code)]
    pub fn apply_suggestion(&mut self) {
        self.key_columns = self.suggested_keys.clone();
    }

    /// Whether rows align positionally (no key selected).
    pub fn is_positional(&self) -> bool {
        self.key_columns.is_empty()
    }

    /// Positional note for the view, if it applies.
    pub fn positional_note(&self) -> Option<&'static str> {
        if self.is_positional() {
            Some(POSITIONAL_NOTE)
        } else {
            None
        }
    }

    /// Engine-tolerant options behind this view. Single source for
    /// [`Self::to_key_options`], so both mappings stay in sync.
    pub fn to_compare_options(&self) -> TableCompareOptions {
        TableCompareOptions {
            key_columns: self.key_columns.clone(),
            numeric_tolerance: Self::sanitize_tolerance(self.numeric_tolerance),
            ignore_case: self.ignore_case,
            trim_space: self.trim_space,
        }
    }

    /// File-compare key options behind this view. Built from
    /// [`Self::to_compare_options`] so the two mappings never drift.
    pub fn to_key_options(&self) -> TableKeyOptions {
        let opts = self.to_compare_options();
        TableKeyOptions {
            key_columns: opts.key_columns,
            numeric_tolerance: opts.numeric_tolerance,
            ignore_case: opts.ignore_case,
            trim_space: opts.trim_space,
        }
    }

    /// Compares two cells under this view's tolerant options. Test-only:
    /// runtime cell comparison runs inside the engine diff; unit tests
    /// probe the view's tolerance folding through this wrapper.
    #[allow(dead_code)]
    pub fn cells_equal(&self, left: &str, right: &str) -> bool {
        cells_equal_with_options(left, right, &self.to_compare_options())
    }

    /// Runs the key-aligned diff over two parsed tables and records
    /// the summary. Pure and total. Test-only: see [`Self::suggest_for`];
    /// the file-driven wiring summarizes via [`summarize`] on engine output.
    #[allow(dead_code)]
    pub fn refresh(&mut self, left: &Table, right: &Table) {
        let opts = self.to_compare_options();
        let diff = diff_table_by_key_with_options(left, right, &opts);
        self.summary = summarize(&diff, self.is_positional());
    }

    /// Honest one-line status for the status bar.
    pub fn status_line(&self) -> String {
        let mut line = format!("table keyed: {}", self.summary.line());
        if let Some(note) = self.positional_note() {
            line.push_str(&format!(" | note: {note}"));
        }
        line
    }
}

/// Builds a [`TableKeySummary`] from an engine diff. `positional` is
/// true when the diff ran without key columns.
pub fn summarize(diff: &TableKeyDiff, positional: bool) -> TableKeySummary {
    TableKeySummary {
        matching: diff.matching_rows,
        modified: diff.modified_rows,
        left_only: diff.left_only_rows,
        right_only: diff.right_only_rows,
        keyed: !positional,
        positional_note: positional,
    }
}

// ----------------------------------------------------------------------------
// Table grid (headless; paint wiring stays with the orchestrator).
// Gap 3: the table tab paints har row ko ek hi monospace string me.
// Ye section pure grid spec hai: header labels, char-width se column
// widths, padded cell text, aur per-cell cursor. No egui, no I/O;
// paint har visible row ko `table_column_widths` + `pad_table_cell`
// se columns me lay out kare aur `TableCellCursor` se nav kare.
// ----------------------------------------------------------------------------

/// Upper bound for ek column ki char width, taaki ek lambi cell poori
/// grid ko faila na de. Paint isse aage ka text kaat kar dikhaye.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const TABLE_MAX_COL_CHARS: usize = 32;

/// Minimum column width taaki header kabhi dab na jaye.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const TABLE_MIN_COL_CHARS: usize = 3;

/// Ek cell ka grid position: data-row index plus column index.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableCellCursor {
    /// Data row (header ko count nahi karta).
    pub row: usize,
    /// Column index.
    pub col: usize,
}

/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
impl TableCellCursor {
    /// Grid dimensions ke andar cursor, warna `None`.
    pub fn clamp(self, row_count: usize, col_count: usize) -> Option<Self> {
        if self.row < row_count && self.col < col_count {
            Some(self)
        } else {
            None
        }
    }

    /// Ek row upar, top par clamp.
    pub fn move_up(self) -> Self {
        Self {
            row: self.row.saturating_sub(1),
            col: self.col,
        }
    }

    /// Ek row neeche, caller `clamp` se bound kare.
    pub fn move_down(self) -> Self {
        Self {
            row: self.row.saturating_add(1),
            col: self.col,
        }
    }

    /// Ek column left, first par clamp.
    pub fn move_left(self) -> Self {
        Self {
            row: self.row,
            col: self.col.saturating_sub(1),
        }
    }

    /// Ek column right, caller `clamp` se bound kare.
    pub fn move_right(self) -> Self {
        Self {
            row: self.row,
            col: self.col.saturating_add(1),
        }
    }
}

/// Column widths chars me: har column ka max(header, cells), min par
/// floor aur max par cap. Kabhi khali nahi jab header khali nahi.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn table_column_widths(header: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = Vec::new();
    for cell in header.iter() {
        widths.push(
            cell.chars()
                .count()
                .clamp(TABLE_MIN_COL_CHARS, TABLE_MAX_COL_CHARS),
        );
    }
    for row in rows.iter() {
        for (idx, cell) in row.iter().enumerate() {
            let Some(slot) = widths.get_mut(idx) else {
                continue;
            };
            let want = cell
                .chars()
                .count()
                .clamp(TABLE_MIN_COL_CHARS, TABLE_MAX_COL_CHARS);
            if want > *slot {
                *slot = want;
            }
        }
    }
    widths
}

/// Ek cell ko width tak spaces se pad karta hai. Lamba text cap par
/// kat-ta hai taaki columns align rahen. Char-based hai taaki
/// multibyte text par bhi column na tute.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn pad_table_cell(text: &str, width: usize) -> String {
    let capped: String = text.chars().take(width.max(1)).collect();
    let len = capped.chars().count();
    if len >= width {
        capped
    } else {
        let mut out = capped;
        for _ in len..width {
            out.push(' ');
        }
        out
    }
}

/// Ek row ki padded cells Grid paint ke liye: har width tak
/// `pad_table_cell`, missing cell `-` taaki ragged rows me bhi
/// columns align rahen. `table_column_widths` ke saath jodta hai.
/// Pure; unit-tested.
pub fn pad_table_row(cells: &[String], widths: &[usize]) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, width) in widths.iter().enumerate() {
        let cell = cells.get(idx).map(String::as_str).unwrap_or("-");
        out.push(pad_table_cell(cell, *width));
    }
    out
}

/// Header row ka display text, widths ke hisab se padded.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn format_table_header(header: &[String], widths: &[usize]) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, cell) in header.iter().enumerate() {
        let width = widths.get(idx).copied().unwrap_or(TABLE_MIN_COL_CHARS);
        out.push(pad_table_cell(cell, width));
    }
    out
}

/// Data cell ka display text: missing cell `-` dikhta hai taaki ragged
/// rows me bhi columns align rahen.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn table_cell_display(rows: &[Vec<String>], pos: TableCellCursor) -> Option<String> {
    let row = rows.get(pos.row)?;
    let width_hint = row.len().max(1);
    let _ = width_hint;
    match row.get(pos.col) {
        Some(cell) => Some(cell.clone()),
        None => Some(String::from("-")),
    }
}

/// Cell text ka find matcher: khali needle kabhi match nahi karta.
fn table_cell_contains(cell: &str, needle: &str, ignore_case: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    if ignore_case {
        cell.to_lowercase().contains(&needle.to_lowercase())
    } else {
        cell.contains(needle)
    }
}

/// Agli cell `(row, col)` jisme needle hai, `from_row` se (inclusive),
/// row-major order me, wrap ke saath. Khali needle/rows par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn table_find_next(
    rows: &[Vec<String>],
    needle: &str,
    from_row: usize,
    ignore_case: bool,
) -> Option<(usize, usize)> {
    if needle.is_empty() || rows.is_empty() {
        return None;
    }
    let total = rows.len();
    let mut row = from_row % total.max(1);
    for _ in 0..total {
        let cells = rows.get(row)?;
        for (col, cell) in cells.iter().enumerate() {
            if table_cell_contains(cell, needle, ignore_case) {
                return Some((row, col));
            }
        }
        row = row.saturating_add(1) % total.max(1);
    }
    None
}

/// Pichhli cell `(row, col)` jisme needle hai, `from_row` se peeche
/// (exclusive), wrap ke saath. Khali needle/rows par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn table_find_prev(
    rows: &[Vec<String>],
    needle: &str,
    from_row: usize,
    ignore_case: bool,
) -> Option<(usize, usize)> {
    if needle.is_empty() || rows.is_empty() {
        return None;
    }
    let total = rows.len();
    let mut row = if from_row == 0 {
        total.saturating_sub(1)
    } else {
        (from_row.saturating_sub(1)) % total.max(1)
    };
    for _ in 0..total {
        let cells = rows.get(row)?;
        for (col, cell) in cells.iter().enumerate().rev() {
            if table_cell_contains(cell, needle, ignore_case) {
                return Some((row, col));
            }
        }
        row = if row == 0 {
            total.saturating_sub(1)
        } else {
            row.saturating_sub(1)
        };
    }
    None
}

/// Table find bar ki geometry (text/folder precedent reuse, koi naya
/// token nahi): bar `28.0` px uncha, needle field `120.0` px chauda,
/// badge right-end se `4.0` px pad par.
pub const TABLE_FIND_BAR_H: f32 = 28.0;
/// Table find needle field ki chaudai (text precedent reuse).
pub const TABLE_FIND_NEEDLE_W: f32 = 120.0;
/// Table find badge ka right-end pad (text precedent reuse).
pub const TABLE_FIND_BADGE_PAD: f32 = 4.0;

/// Sab matching cells row-major order me: har `(row, col)` jisme
/// needle hai. Khali needle par khali vec. `table_find_next` ka
/// poora-hit judvaan (N/M count `len()` se aata hai). Pure; unit-tested.
pub fn table_find_match_rows(
    rows: &[Vec<String>],
    needle: &str,
    ignore_case: bool,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    for (row_idx, cells) in rows.iter().enumerate() {
        for (col_idx, cell) in cells.iter().enumerate() {
            if table_cell_contains(cell, needle, ignore_case) {
                out.push((row_idx, col_idx));
            }
        }
    }
    out
}

/// N/M badge text table find bar ke liye: `Some(2), 5 -> "2/5"`,
/// koi hit nahi par `"0/0"`. Text precedent mirror. Pure; unit-tested.
pub fn table_find_badge_label(position: Option<usize>, total: usize) -> String {
    match position {
        Some(pos) => format!("{pos}/{total}"),
        None => format!("0/{total}"),
    }
}

/// Table window ke andar find row: needle field + Prev/Next + N/M
/// badge. Text find bar precedent ka geometry/typo reuse (wahi bar
/// height, wahi needle chaudai, wahi button labels, badge right me
/// ek pad ke saath small weak typo me). Returns
/// `(prev_clicked, next_clicked)`; cursor/badge advance App state
/// (`table_find_next_row` / `table_find_prev_row`) me hota hai.
/// Caller signature same rehti hai (table tab wiring isi ko chalati
/// hai, `views.rs` caller untouched).
pub fn paint_table_find_bar(ui: &mut egui::Ui, needle: &mut String, badge: &str) -> (bool, bool) {
    let mut prev = false;
    let mut next = false;
    let avail = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), TABLE_FIND_BAR_H));
        ui.label(egui::RichText::new("Find:").small().weak());
        ui.add(
            egui::TextEdit::singleline(needle)
                .desired_width(TABLE_FIND_NEEDLE_W)
                .hint_text("needle"),
        );
        if ui.small_button("◀ Prev").clicked() {
            prev = true;
        }
        if ui.small_button("Next ▶").clicked() {
            next = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(TABLE_FIND_BADGE_PAD);
            ui.label(egui::RichText::new(badge).small().weak());
        });
    });
    (prev, next)
}

// ----------------------------------------------------------------------------
// Per-hunk merge (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #10: table tab me har modified/orphan row par
// take-left/take-right. Ye pure row choice hai; file apply (I/O)
// wiring ka kaam hai aur OPEN rehta hai. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Per-row merge choice: kaunsi side ki row rakhi jaye.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableMergeSide {
    /// Left row rakho.
    #[default]
    Left,
    /// Right row rakho.
    Right,
}

/// Ek row ka merge result: chosen side ki cells. `None` side matlab
/// orphan ka missing side: dusri side milti hai; dono `None` par khali row.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn table_resolve_row(
    left_row: Option<&[String]>,
    right_row: Option<&[String]>,
    side: TableMergeSide,
) -> Vec<String> {
    let chosen = match side {
        TableMergeSide::Left => left_row.or(right_row),
        TableMergeSide::Right => right_row.or(left_row),
    };
    chosen.unwrap_or(&[]).to_vec()
}

// ----------------------------------------------------------------------------
// Paging (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #3: lambi table me page-wise move. Chhota pager top
// data-row rakhta hai; paint PageUp/PageDown se isko chalata hai.
// Pure math, saturating, kabhi panic nahi. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Page-wise scroll state ek table grid ke liye (header fixed rehta hai).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TablePager {
    /// Kul data rows (header ko count nahi karta).
    pub total_rows: usize,
    /// Ek page me kitni rows.
    pub rows_per_page: usize,
    /// Visible window ki pehli data row.
    pub top: usize,
}

/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
impl TablePager {
    /// Naya pager: page size kam se kam 1, top 0.
    pub fn new(total_rows: usize, rows_per_page: usize) -> Self {
        Self {
            total_rows,
            rows_per_page: rows_per_page.max(1),
            top: 0,
        }
    }

    /// Kul pages (khali grid = 0).
    pub fn page_count(self) -> usize {
        if self.total_rows == 0 {
            return 0;
        }
        self.total_rows
            .saturating_sub(1)
            .saturating_div(self.rows_per_page.max(1))
            .saturating_add(1)
    }

    /// Aakhri valid top.
    pub fn last_top(self) -> usize {
        let count = self.page_count();
        if count == 0 {
            return 0;
        }
        count
            .saturating_sub(1)
            .saturating_mul(self.rows_per_page.max(1))
    }

    /// Top ko valid range me rakhta hai.
    pub fn clamp_top(&mut self) {
        let last = self.last_top();
        if self.top > last {
            self.top = last;
        }
    }

    /// Ek page aage, aakhri page par clamp.
    pub fn next_page(&mut self) {
        self.top = self
            .top
            .saturating_add(self.rows_per_page.max(1))
            .min(self.last_top());
    }

    /// Ek page peeche, 0 par clamp.
    pub fn prev_page(&mut self) {
        self.top = self.top.saturating_sub(self.rows_per_page.max(1));
    }

    /// Visible window `[top, end)`: end kabhi total se aage nahi.
    pub fn visible_range(self) -> (usize, usize) {
        let end = self
            .top
            .saturating_add(self.rows_per_page.max(1))
            .min(self.total_rows);
        (self.top, end.max(self.top))
    }

    /// `row` dikhe isliye top adjust karta hai. Returns whether top moved.
    pub fn bring_into_view(&mut self, row: usize) -> bool {
        if self.total_rows == 0 {
            return false;
        }
        let row = row.min(self.total_rows.saturating_sub(1));
        let (start, end) = self.visible_range();
        if row >= start && row < end {
            return false;
        }
        if row < self.top {
            self.top = row;
        } else {
            self.top = row
                .saturating_add(1)
                .saturating_sub(self.rows_per_page.max(1));
        }
        self.clamp_top();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tables() -> (Table, Table) {
        let left = Table {
            header: vec!["ID".to_string(), "Price".to_string()],
            rows: vec![
                vec!["1".to_string(), "9.99".to_string()],
                vec!["2".to_string(), "5.00".to_string()],
                vec!["3".to_string(), "7.50".to_string()],
            ],
        };
        let right = Table {
            header: vec!["ID".to_string(), "Price".to_string()],
            rows: vec![
                vec!["1".to_string(), "10.04".to_string()],
                vec!["2".to_string(), "5.00".to_string()],
                vec!["4".to_string(), "1.00".to_string()],
            ],
        };
        (left, right)
    }

    #[test]
    fn key_suggestion_finds_unique_id_column() {
        let (left, _) = sample_tables();
        let mut view = TableKeyView::new();
        view.suggest_for(&left);
        assert_eq!(view.suggested_keys, vec![0]);
        view.apply_suggestion();
        assert_eq!(view.key_columns, vec![0]);
        assert!(!view.is_positional());
    }

    #[test]
    fn keyed_diff_counts_match_modify_and_orphans() {
        let (left, right) = sample_tables();
        let mut view = TableKeyView::new();
        view.key_columns = vec![0];
        view.refresh(&left, &right);
        assert_eq!(view.summary.matching, 1);
        assert_eq!(view.summary.modified, 1);
        assert_eq!(view.summary.left_only, 1);
        assert_eq!(view.summary.right_only, 1);
        assert!(view.summary.keyed);
        assert!(!view.summary.positional_note);
        assert!(view.summary.line().contains("(keyed)"));
    }

    #[test]
    fn empty_keys_fall_back_to_positional_with_note() {
        let (left, right) = sample_tables();
        let mut view = TableKeyView::new();
        assert!(view.is_positional());
        assert_eq!(view.positional_note(), Some(POSITIONAL_NOTE));
        view.refresh(&left, &right);
        assert!(!view.summary.keyed);
        assert!(view.summary.positional_note);
        assert_eq!(
            view.summary.total(),
            view.summary
                .matching
                .saturating_add(view.summary.modified)
                .saturating_add(view.summary.left_only)
                .saturating_add(view.summary.right_only)
        );
        assert!(view.summary.line().contains("(positional)"));
        assert!(view.status_line().contains(POSITIONAL_NOTE));
    }

    #[test]
    fn tolerance_bridges_small_numeric_gaps() {
        let (left, right) = sample_tables();
        let mut view = TableKeyView::new();
        view.key_columns = vec![0];
        view.set_tolerance(Some(0.1));
        view.refresh(&left, &right);
        // 9.99 vs 10.04 is within 0.1: only the missing-key rows differ.
        assert_eq!(view.summary.matching, 2);
        assert_eq!(view.summary.modified, 0);
        assert_eq!(view.summary.left_only, 1);
        assert_eq!(view.summary.right_only, 1);
    }

    #[test]
    fn invalid_tolerances_sanitize_to_exact_text() {
        assert_eq!(TableKeyView::sanitize_tolerance(None), None);
        assert_eq!(TableKeyView::sanitize_tolerance(Some(0.0)), Some(0.0));
        assert_eq!(TableKeyView::sanitize_tolerance(Some(1.5)), Some(1.5));
        assert_eq!(TableKeyView::sanitize_tolerance(Some(f64::NAN)), None);
        assert_eq!(TableKeyView::sanitize_tolerance(Some(f64::INFINITY)), None);
        assert_eq!(
            TableKeyView::sanitize_tolerance(Some(f64::NEG_INFINITY)),
            None
        );
        assert_eq!(TableKeyView::sanitize_tolerance(Some(-0.5)), None);
        let mut view = TableKeyView::new();
        view.set_tolerance(Some(f64::NAN));
        assert_eq!(view.numeric_tolerance, None);
    }

    #[test]
    fn nan_cells_never_match_numerically() {
        let mut view = TableKeyView::new();
        view.set_tolerance(Some(0.01));
        // "NaN" vs "nan" differ as text and never match numerically.
        assert!(!view.cells_equal("NaN", "nan"));
        assert!(!view.cells_equal("inf", "Infinity"));
        assert!(view.cells_equal("abc", "abc"));
        assert!(view.cells_equal("1.001", "1.002"));
        assert!(!view.cells_equal("1.001", "1.02"));
    }

    #[test]
    fn case_and_trim_flags_fold_text() {
        let mut view = TableKeyView::new();
        assert!(!view.cells_equal("Ready", "ready"));
        assert!(!view.cells_equal("  x", "x"));
        view.ignore_case = true;
        view.trim_space = true;
        assert!(view.cells_equal("Ready", "ready"));
        assert!(view.cells_equal("  x ", "X"));
    }

    #[test]
    fn key_selector_toggles_columns_sorted() {
        let mut view = TableKeyView::new();
        view.toggle_key_column(2);
        view.toggle_key_column(0);
        assert_eq!(view.key_columns, vec![0, 2]);
        view.toggle_key_column(0);
        assert_eq!(view.key_columns, vec![2]);
        view.toggle_key_column(2);
        assert!(view.key_columns.is_empty());
        assert!(view.is_positional());
    }

    #[test]
    fn options_mapping_carries_all_flags() {
        let mut view = TableKeyView::new();
        view.key_columns = vec![0, 1];
        view.set_tolerance(Some(0.25));
        view.ignore_case = true;
        view.trim_space = true;
        let cmp_opts = view.to_compare_options();
        assert_eq!(cmp_opts.key_columns, vec![0, 1]);
        assert_eq!(cmp_opts.numeric_tolerance, Some(0.25));
        assert!(cmp_opts.ignore_case);
        assert!(cmp_opts.trim_space);
        let key_opts = view.to_key_options();
        assert_eq!(key_opts.key_columns, vec![0, 1]);
        assert_eq!(key_opts.numeric_tolerance, Some(0.25));
        assert!(key_opts.ignore_case);
        assert!(key_opts.trim_space);
    }

    #[test]
    fn key_options_mirror_compare_options() {
        let mut view = TableKeyView::new();
        view.key_columns = vec![0, 2];
        view.set_tolerance(Some(0.5));
        view.ignore_case = true;
        let cmp_opts = view.to_compare_options();
        let key_opts = view.to_key_options();
        assert_eq!(key_opts.key_columns, cmp_opts.key_columns);
        assert_eq!(key_opts.numeric_tolerance, cmp_opts.numeric_tolerance);
        assert_eq!(key_opts.ignore_case, cmp_opts.ignore_case);
        assert_eq!(key_opts.trim_space, cmp_opts.trim_space);
    }

    #[test]
    fn empty_tables_suggest_no_key() {
        let empty = Table::default();
        let mut view = TableKeyView::new();
        view.suggest_for(&empty);
        assert!(view.suggested_keys.is_empty());
        view.refresh(&empty, &empty);
        assert_eq!(view.summary.total(), 0);
    }

    #[test]
    fn column_widths_span_header_and_cells_with_cap() {
        let header = vec!["ID".to_string(), "Price".to_string()];
        let rows = vec![
            vec!["1".to_string(), "9.99".to_string()],
            vec!["222".to_string(), "5".to_string()],
        ];
        let widths = table_column_widths(&header, &rows);
        assert_eq!(widths.len(), 2);
        // "ID"(2) floor 3 par, cells me 3 max.
        assert_eq!(widths.first().copied(), Some(3));
        // "Price"(5) sabse chauda header hai.
        assert_eq!(widths.get(1).copied(), Some(5));
        for width in widths.iter() {
            assert!(*width >= TABLE_MIN_COL_CHARS);
            assert!(*width <= TABLE_MAX_COL_CHARS);
        }
        // Lambi cell cap par rukti hai.
        let long = "x".repeat(TABLE_MAX_COL_CHARS + 20);
        let wide_rows = vec![vec![long]];
        let wide = table_column_widths(&["c".to_string()], &wide_rows);
        assert_eq!(wide.first().copied(), Some(TABLE_MAX_COL_CHARS));
        // Extra cells header ke bahar widths nahi badhate.
        let ragged = vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]];
        let narrow = table_column_widths(&["h".to_string()], &ragged);
        assert_eq!(narrow.len(), 1);
    }

    #[test]
    fn padded_cells_keep_columns_aligned() {
        assert_eq!(pad_table_cell("ID", 5), "ID   ");
        assert_eq!(pad_table_cell("Price", 5), "Price");
        // Cap se lamba text kat-ta hai.
        assert_eq!(pad_table_cell("abcdef", 4), "abcd");
        let header = vec!["ID".to_string(), "Price".to_string()];
        let widths = table_column_widths(&header, &[]);
        let line = format_table_header(&header, &widths);
        assert_eq!(line.len(), 2);
        assert_eq!(line.first().map(String::len), widths.first().copied());
        assert_eq!(line.get(1).map(String::len), widths.get(1).copied());
    }

    #[test]
    fn table_find_scans_cells_row_major_with_wrap() {
        let rows = vec![
            vec!["ID".to_string(), "Price".to_string()],
            vec!["apple".to_string(), "9.99".to_string()],
            vec!["Banana".to_string(), "5.00".to_string()],
        ];
        assert_eq!(table_find_next(&rows, "apple", 0, false), Some((1, 0)));
        // Case-sensitive: "banana" chhota nahi milega, ignore_case par milega.
        assert_eq!(table_find_next(&rows, "banana", 0, false), None);
        assert_eq!(table_find_next(&rows, "banana", 0, true), Some((2, 0)));
        // Wrap: aakhri row ke baad pehli match.
        assert_eq!(table_find_next(&rows, "ID", 1, false), Some((0, 0)));
        // Prev: peeche scan, wrap ke saath.
        assert_eq!(table_find_prev(&rows, "apple", 2, false), Some((1, 0)));
        assert_eq!(table_find_prev(&rows, "apple", 0, false), Some((1, 0)));
        // Khali needle/rows: None.
        assert_eq!(table_find_next(&rows, "", 0, false), None);
        assert_eq!(table_find_next(&[], "x", 0, false), None);
        assert_eq!(table_find_prev(&rows, "", 1, false), None);
        assert_eq!(table_find_prev(&[], "x", 1, false), None);
        assert_eq!(table_find_next(&rows, "zzz", 0, false), None);
    }

    #[test]
    fn table_find_match_rows_collects_all_hits() {
        let rows = vec![
            vec!["ID".to_string(), "Price".to_string()],
            vec!["apple".to_string(), "9.99".to_string()],
            vec!["Banana".to_string(), "apple pie".to_string()],
        ];
        assert_eq!(
            table_find_match_rows(&rows, "apple", false),
            vec![(1, 0), (2, 1)]
        );
        // Case-sensitive: chhota "banana" nahi milega, ignore_case par milega.
        assert!(table_find_match_rows(&rows, "banana", false).is_empty());
        assert_eq!(table_find_match_rows(&rows, "banana", true), vec![(2, 0)]);
        // Khali needle/rows: khali vec.
        assert!(table_find_match_rows(&rows, "", false).is_empty());
        assert!(table_find_match_rows(&[], "x", false).is_empty());
        assert!(table_find_match_rows(&rows, "zzz", false).is_empty());
        // Geometry tokens text precedent reuse karte hain.
        assert_eq!(TABLE_FIND_BAR_H, 28.0);
        assert_eq!(TABLE_FIND_NEEDLE_W, 120.0);
        assert_eq!(TABLE_FIND_BADGE_PAD, 4.0);
    }

    #[test]
    fn table_find_badge_labels_position_over_total() {
        assert_eq!(table_find_badge_label(Some(2), 5), "2/5");
        assert_eq!(table_find_badge_label(Some(1), 1), "1/1");
        assert_eq!(table_find_badge_label(None, 0), "0/0");
        assert_eq!(table_find_badge_label(None, 4), "0/4");
    }

    #[test]
    fn table_merge_picks_chosen_side_with_orphan_fallback() {
        let left = vec!["1".to_string(), "9.99".to_string()];
        let right = vec!["1".to_string(), "10.04".to_string()];
        assert_eq!(
            table_resolve_row(Some(&left), Some(&right), TableMergeSide::Left),
            left
        );
        assert_eq!(
            table_resolve_row(Some(&left), Some(&right), TableMergeSide::Right),
            right
        );
        // Orphan: missing side par dusri side milti hai.
        assert_eq!(
            table_resolve_row(Some(&left), None, TableMergeSide::Right),
            left
        );
        assert_eq!(
            table_resolve_row(None, Some(&right), TableMergeSide::Left),
            right
        );
        assert!(table_resolve_row(None, None, TableMergeSide::Left).is_empty());
    }

    #[test]
    fn table_pager_pages_and_brings_rows_into_view() {
        let mut pager = TablePager::new(25, 10);
        assert_eq!(pager.page_count(), 3);
        assert_eq!(pager.visible_range(), (0, 10));
        pager.next_page();
        assert_eq!(pager.top, 10);
        pager.next_page();
        assert_eq!(pager.top, 20);
        pager.next_page();
        assert_eq!(pager.top, 20);
        pager.prev_page();
        assert_eq!(pager.top, 10);
        assert!(!pager.bring_into_view(12));
        assert!(pager.bring_into_view(24));
        assert_eq!(pager.top, 15);
        let mut empty = TablePager::new(0, 10);
        assert_eq!(empty.page_count(), 0);
        assert!(!empty.bring_into_view(0));
        let one = TablePager::new(3, 0);
        assert_eq!(one.rows_per_page, 1);
    }

    #[test]
    fn cell_cursor_nav_stays_inside_grid() {
        let rows = vec![
            vec!["1".to_string(), "9.99".to_string()],
            vec!["2".to_string(), "5.00".to_string()],
        ];
        let start = TableCellCursor { row: 0, col: 0 };
        assert!(start.clamp(2, 2).is_some());
        assert!(TableCellCursor { row: 5, col: 0 }.clamp(2, 2).is_none());
        assert!(TableCellCursor { row: 0, col: 9 }.clamp(2, 2).is_none());
        assert_eq!(start.move_left(), start);
        assert_eq!(start.move_up(), start);
        let right = start.move_right();
        assert_eq!(right.col, 1);
        let down = right.move_down();
        assert_eq!((down.row, down.col), (1, 1));
        // Missing cell `-` dikhta hai, maujood cell waisa ka waisa.
        assert_eq!(
            table_cell_display(&rows, TableCellCursor { row: 0, col: 0 }),
            Some("1".to_string())
        );
        assert_eq!(
            table_cell_display(&rows, TableCellCursor { row: 1, col: 5 }),
            Some("-".to_string())
        );
        assert_eq!(
            table_cell_display(&rows, TableCellCursor { row: 9, col: 0 }),
            None
        );
    }

    #[test]
    fn pad_table_row_pads_and_marks_missing() {
        let widths = vec![3, 5];
        let cells = vec!["1".to_string(), "9.99".to_string()];
        assert_eq!(pad_table_row(&cells, &widths), vec!["1  ", "9.99 "]);
        // Ragged row: missing cell `-` se bharta hai.
        let ragged = vec!["only".to_string()];
        assert_eq!(pad_table_row(&ragged, &widths), vec!["onl", "-    "]);
        // Lamba text cap par kat-ta hai, widths barkarar.
        let long = vec!["x".repeat(TABLE_MAX_COL_CHARS + 10)];
        let wide = vec![TABLE_MAX_COL_CHARS];
        let padded = pad_table_row(&long, &wide);
        assert_eq!(padded.len(), 1);
        assert_eq!(padded.first().map(String::len), Some(TABLE_MAX_COL_CHARS));
        // Khali widths: khali row.
        assert!(pad_table_row(&cells, &[]).is_empty());
    }
}
