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

use crate::theme::TypoRole;

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
    pub fn suggest_for(&mut self, left: &Table) {
        self.suggested_keys = detect_key_columns(left);
    }

    /// Adopts the last suggestion as the selected key. Test-only: see
    /// [`Self::suggest_for`]; the file-driven wiring toggles key columns
    /// manually via [`Self::toggle_key_column`].
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
    pub fn cells_equal(&self, left: &str, right: &str) -> bool {
        cells_equal_with_options(left, right, &self.to_compare_options())
    }

    /// Runs the key-aligned diff over two parsed tables and records
    /// the summary. Pure and total. Test-only: see [`Self::suggest_for`];
    /// the file-driven wiring summarizes via [`summarize`] on engine output.
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
// Key-column affordance (pure state queries + thin paint; wiring stays with
// the orchestrator). Existing `TableKeyView` reuse: toggle/select state,
// `POSITIONAL_NOTE`, `positional_note()` same rahte hain. Geometry first:
// fixed 1-char marker har toggle ke start me (indicator column align),
// `TABLE_KEY_BAR_H` uncha bar, `TABLE_KEY_COL_GAP` gap, badge right-end par
// `TABLE_KEY_BADGE_PAD` ke saath (find-bar badge geometry reuse).
// Typography second: section labels Body-strong (helper +
// paint-side `.strong()`), header/selected key cells mono-strong,
// plain cells mono, hint/badge `(suggested)` Small-role, positional
// note + empty note Body-role (weak hata — weight se contrast, color
// frozen); koi size-override/color add nahi. No I/O.
// ----------------------------------------------------------------------------

/// Key-col header bar ki unchai (find-bar precedent reuse).
pub const TABLE_KEY_BAR_H: f32 = 28.0;
/// Per-column key toggle ke beech ka gap (default item-spacing scale).
pub const TABLE_KEY_COL_GAP: f32 = 8.0;
/// Key badge ka right-end pad (find-bar badge precedent reuse).
pub const TABLE_KEY_BADGE_PAD: f32 = 4.0;

/// Key toggle ka fixed 1-char indicator: selected vs deselected hamesha
/// alag dikhta hai aur hamesha button text ke start me rehta hai, isliye
/// indicator column align rehta hai. Pure; unit-tested.
pub fn key_column_marker(selected: bool) -> &'static str {
    if selected { "●" } else { "○" }
}

/// Kya `col` is waqt key me selected hai. Pure; unit-tested.
pub fn is_key_column(view: &TableKeyView, col: usize) -> bool {
    view.key_columns.contains(&col)
}

/// Kya `col` aakhri suggestion me tha. Pure; unit-tested.
pub fn is_suggested_key(view: &TableKeyView, col: usize) -> bool {
    view.suggested_keys.contains(&col)
}

/// Key badge text: keyed par `key: 0,2`, positional par `positional`.
/// Khali header par bhi safe (sirf view state par). Pure; unit-tested.
pub fn table_key_badge_label(view: &TableKeyView) -> String {
    if view.is_positional() {
        String::from("positional")
    } else {
        let mut out = String::from("key:");
        for (pos, col) in view.key_columns.iter().enumerate() {
            if pos == 0 {
                out.push(' ');
            } else {
                out.push(',');
            }
            out.push_str(&col.to_string());
        }
        out
    }
}

/// Key-col header affordance row: har header column ek
/// `selectable_label(marker + name)` toggle hai (selected par
/// `.strong().monospace()`, deselected par `.monospace()`), click par
/// `toggle_key_column(col)`; suggested-par-Small-role
/// `(suggested)` hint; right-end me `table_key_badge_label` badge
/// Small-role me (find-bar badge geometry reuse); positional hone
/// par `POSITIONAL_NOTE` note line me Body-role (honest,
/// `positional_note()` reuse, koi naya wording nahi). Caller signature
/// stable rehti hai (table tab wiring isi ko chalati hai).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_key_bar(ui: &mut egui::Ui, view: &mut TableKeyView, header: &[String]) {
    let avail = ui.available_width();
    let mut clicked: Option<usize> = None;
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), TABLE_KEY_BAR_H));
        ui.spacing_mut().item_spacing.x = TABLE_KEY_COL_GAP;
        ui.label(table_section_label_text_role().rich("Key:").strong());
        for (col, name) in header.iter().enumerate() {
            let selected = is_key_column(view, col);
            let suggested = is_suggested_key(view, col);
            let marker = key_column_marker(selected);
            let text = format!("{marker} {name}");
            let rich = if selected {
                egui::RichText::new(text).monospace().strong()
            } else {
                egui::RichText::new(text).monospace()
            };
            if ui.selectable_label(selected, rich).clicked() {
                clicked = Some(col);
            }
            if suggested && !selected {
                ui.label(TypoRole::Small.rich("(suggested)"));
            }
            ui.add_space(TABLE_KEY_COL_GAP);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(TABLE_KEY_BADGE_PAD);
            ui.label(TypoRole::Small.rich(table_key_badge_label(view)));
        });
    });
    if let Some(col) = clicked {
        view.toggle_key_column(col);
    }
    // `positional_note()` khud sirf positional par `Some` deta hai,
    // isliye ek `if let` hi honest-note gate hai (outer check redundant).
    if let Some(note) = view.positional_note() {
        ui.label(TypoRole::Body.rich(note));
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
pub const TABLE_MAX_COL_CHARS: usize = 32;

/// Minimum column width taaki header kabhi dab na jaye.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const TABLE_MIN_COL_CHARS: usize = 3;

/// Ek cell ka grid position: data-row index plus column index.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableCellCursor {
    /// Data row (header ko count nahi karta).
    pub row: usize,
    /// Column index.
    pub col: usize,
}

/// Test-only until paint wiring lands (orchestrator owns paint).
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

/// Table window ke andar Find controls ka single source: needle field +
/// Prev/Next + N/M badge (text find-bar precedent ka geometry/typo reuse:
/// wahi bar height, wahi needle chaudai, wahi button labels, badge right me
/// ek pad ke saath Small-role me; label Body-strong, needle Mono family).
/// `paint_table_find_bar` aur
/// `paint_table_toolbar` dono isi ko chalate hain taaki do Find
/// implementations kabhi drift na karen (G-B2 dedup). Returns
/// `(prev_clicked, next_clicked)`; cursor/badge advance caller App state
/// (`table_find_next_row` / `table_find_prev_row`) me hota hai. Private:
/// callers stable signatures rakhte hain (`views.rs` caller untouched).
fn paint_table_find_controls(ui: &mut egui::Ui, needle: &mut String, badge: &str) -> (bool, bool) {
    let mut prev = false;
    let mut next = false;
    ui.spacing_mut().item_spacing.x = TABLE_KEY_COL_GAP;
    ui.label(table_section_label_text_role().rich("Find:").strong());
    ui.add(
        egui::TextEdit::singleline(needle)
            .desired_width(TABLE_FIND_NEEDLE_W)
            .font(egui::TextStyle::Monospace)
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
        ui.label(TypoRole::Small.rich(badge));
    });
    (prev, next)
}

/// Table window ke andar find row: single-source Find controls
/// (`paint_table_find_controls` — toolbar wahi body chalata hai).
/// Caller signature same rehti hai (table tab wiring isi ko chalati
/// hai, `views.rs` caller untouched).
pub fn paint_table_find_bar(ui: &mut egui::Ui, needle: &mut String, badge: &str) -> (bool, bool) {
    let avail = ui.available_width();
    let mut out = (false, false);
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), TABLE_FIND_BAR_H));
        out = paint_table_find_controls(ui, needle, badge);
    });
    out
}

// ----------------------------------------------------------------------------
// Table find match highlight (AU-TABLEFIND): current-vs-other distinction +
// grid geometry. Gap: find bar sirf cursor/badge chalata hai
// (`table_find_match_rows` + `table_find_badge_label`), per-match highlight
// nahi dikhta — current-vs-other farak nahi hai. Ye section pure
// span/format helpers hai; Grid paint wiring orchestrator ke paas rehti hai
// (caller signature stable, `views.rs` untouched).
// Geometry first: match spans char indices me hain (unpadded cell text par),
// row mapping grid order me hai (`table_find_match_rows` wahi row-major
// order), key-col ke saath alignment grid se aati hai (`pad_table_row`
// trailing pad se span indices nahi hilte — `table_find_span_in_padded`
// sirf cap par clamp karta hai). Typography second: same mono `FontId`
// (koi naya size nahi), emphasis sirf block-vs-line se: current = inverted
// block (no underline), other = underline-only line (no block). Colors
// frozen: caller ke apne `fg/bg` reuse + `TABLE_FIND_UNDERLINE_W`
// underline, koi naya hue/token nahi. No egui ctx, no I/O. Pure;
// unit-tested.
// ----------------------------------------------------------------------------

/// Table find underline width (px): `INTRA_SPAN_UNDERLINE_W` precedent
/// mirror (`views.rs`), same value — koi naya size/token nahi.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const TABLE_FIND_UNDERLINE_W: f32 = 1.0;

/// Ek cell me needle ke char-index spans: `(start, end)` char offsets,
/// natural order me (sorted, clamped). Khali needle par empty; overlapping
/// matches allowed (har char offset se ek window). Case-fold ascii-only hai
/// taaki indices multibyte text par bhi sahi rahen. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_cell_spans(cell: &str, needle: &str, ignore_case: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let cell_chars: Vec<char> = cell.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() || needle_chars.len() > cell_chars.len() {
        return out;
    }
    let last = cell_chars.len().saturating_sub(needle_chars.len());
    for start in 0..=last {
        let mut hit = true;
        for (off, nch) in needle_chars.iter().enumerate() {
            let same = match cell_chars.get(start.saturating_add(off)) {
                Some(cch) if ignore_case => cch.eq_ignore_ascii_case(nch),
                Some(cch) => cch == nch,
                None => false,
            };
            if !same {
                hit = false;
                break;
            }
        }
        if hit {
            out.push((start, start.saturating_add(needle_chars.len())));
        }
    }
    out
}

/// Ek row ki sab hits `(col, start, end)` me, column order me.
/// `table_find_match_rows` ka span judvaan. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_row_spans(
    cells: &[String],
    needle: &str,
    ignore_case: bool,
) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    for (col_idx, cell) in cells.iter().enumerate() {
        for (start, end) in table_find_cell_spans(cell, needle, ignore_case) {
            out.push((col_idx, start, end));
        }
    }
    out
}

/// Poore grid ki sab hits `(row, col, start, end)` me, row-major order me —
/// row mapping Grid paint order jaisi hai (data row index + column index).
/// Key-col ke saath alignment: spans unpadded cell text par hain aur Grid
/// `pad_table_row` se trailing pad karta hai, isliye span indices padded
/// cells me nahi hilte (`table_find_span_in_padded` sirf cap par clamp
/// karta hai). Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_grid_spans(
    rows: &[Vec<String>],
    needle: &str,
    ignore_case: bool,
) -> Vec<(usize, usize, usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    for (row_idx, cells) in rows.iter().enumerate() {
        for (col_idx, start, end) in table_find_row_spans(cells, needle, ignore_case) {
            out.push((row_idx, col_idx, start, end));
        }
    }
    out
}

/// Kya `(row, col)` hit current match hai. Current N/M cursor se aata hai
/// (`table_find_current_cell`); `None` par koi current nahi (sab other).
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_is_current(found: (usize, usize), current: Option<(usize, usize)>) -> bool {
    match current {
        Some(at) => at == found,
        None => false,
    }
}

/// N/M cursor se current match cell: `table_find_match_rows` list me index.
/// Khali list par `None`; bada cursor wrap hota hai (modulo, saturating).
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_current_cell(
    matches: &[(usize, usize)],
    cursor: usize,
) -> Option<(usize, usize)> {
    if matches.is_empty() {
        return None;
    }
    matches.get(cursor % matches.len().max(1)).copied()
}

/// Padded cell me span clamp: padding trailing-only hai isliye andar ke
/// spans nahi hilte; cap (`TABLE_MAX_COL_CHARS`) ke bahar ka hissa kat-ta
/// hai, poora bahar ho to `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_span_in_padded(start: usize, end: usize, width: usize) -> Option<(usize, usize)> {
    if start >= end || start >= width {
        return None;
    }
    Some((start, end.min(width)))
}

/// Kya hit key column me hai — `is_key_column` reuse. Key hits ko bhi wahi
/// underline/block highlight milta hai (koi alag color nahi); alignment
/// grid se aati hai, isliye key-col highlight kabhi column se bahar nahi
/// jata. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_hit_in_key_col(view: &TableKeyView, col: usize) -> bool {
    is_key_column(view, col)
}

/// Ek cell-hit ka paint format: same mono `FontId` (size untouched),
/// caller ke apne `fg/bg` reuse. Current = inverted block (no underline),
/// other = underline-only line (no block) — block-vs-line se clearly
/// distinct. `TABLE_FIND_UNDERLINE_W` reuse, koi naya token/color nahi.
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_hit_format(
    font_id: egui::FontId,
    fg: egui::Color32,
    bg: egui::Color32,
    is_current: bool,
) -> egui::TextFormat {
    if is_current {
        let mut fmt = egui::TextFormat::simple(font_id, bg);
        fmt.background = fg;
        fmt
    } else {
        let mut fmt = egui::TextFormat::simple(font_id, fg);
        fmt.background = bg;
        fmt.underline = egui::Stroke::new(TABLE_FIND_UNDERLINE_W, fg);
        fmt
    }
}

/// Ek cell ka find-aware layout: non-hit hissa plain mono, har hit
/// `table_find_hit_format` se (current block, others line). Khali needle ya
/// zero spans par single plain section. Overlapping spans me pehla hit
/// jeetta hai (consumed range dobara paint nahi hota) taaki job text cell
/// ke barabar rahe. Headless-first: `LayoutJob` ctx ke bina banta hai,
/// isliye unit-test me sections inspect hote hain. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_cell_job(
    cell: &str,
    needle: &str,
    ignore_case: bool,
    is_current: bool,
    font_id: egui::FontId,
    fg: egui::Color32,
    bg: egui::Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let plain = egui::TextFormat::simple(font_id.clone(), fg);
    let spans = table_find_cell_spans(cell, needle, ignore_case);
    if spans.is_empty() {
        job.append(cell, 0.0, plain);
        return job;
    }
    let chars: Vec<char> = cell.chars().collect();
    let mut cursor = 0usize;
    for (start, end) in spans {
        if start < cursor {
            continue;
        }
        if start > cursor
            && let Some(gap) = chars.get(cursor..start)
        {
            let text: String = gap.iter().collect();
            job.append(&text, 0.0, plain.clone());
        }
        if let Some(hit) = chars.get(start..end) {
            let text: String = hit.iter().collect();
            job.append(
                &text,
                0.0,
                table_find_hit_format(font_id.clone(), fg, bg, is_current),
            );
        }
        if end > cursor {
            cursor = end;
        }
    }
    if let Some(tail) = chars.get(cursor..)
        && !tail.is_empty()
    {
        let text: String = tail.iter().collect();
        job.append(&text, 0.0, plain);
    }
    job
}

// ----------------------------------------------------------------------------
// Table cell grid (cell boxes + key highlight + find underline).
// Gap AX2 §10.3: table tab pipe-string rows paint karta hai — `views.rs`
// `paint_table` data grid `table_row_cells` se cell boxes me baant-ta hai,
// par key-col highlight + find underline nahi dikhta. Ye section cell-level
// paint adoption hai: key-col cells strong typo me, find hits
// `table_find_cell_job` LayoutJob me (`TABLE_FIND_UNDERLINE_W` underline),
// key bar `paint_table_key_bar` se live. Find bar caller ke paas rehta hai
// (`paint_table_find_bar` pehle se live hai) — grid `TableCellFindCtx` se
// needle/current leta hai. Geometry first: Grid columns glyph + ek per data
// col (`table_cell_grid_columns`), spacing `SECTION_GAP`/`TIGHT_GAP` mirror
// (`TABLE_GRID_COL_GAP`/`TABLE_GRID_ROW_GAP`, same values), glyph placeholder
// `CONTENT_LEAD` mirror. Typography second: header `.monospace().strong()`
// (existing header precedent), key data cells `.monospace().strong()`, plain
// `.monospace()`, find hits job sections (same mono `FontId`, koi naya size
// nahi). Colors frozen: caller ke `fg/bg` reuse, koi naya hue/token nahi;
// key emphasis sirf `.strong()` role hai (header/key-bar precedent), hit
// emphasis block-vs-line (`table_find_hit_format` reuse). Row-status glyph /
// changed tint caller-side rehta hai (views.rs paint ownership) — grid me
// glyph placeholder hai. No I/O. Paint fns test-only until wiring lands
// (orchestrator owns paint); pure helpers unit-tested.
// ----------------------------------------------------------------------------

/// Cell grid ka data Grid id (header grid alag id rakhta hai taaki dono
/// Grids ek Compose me takrayen nahi).
pub const TABLE_CELL_GRID_ID: &str = "table_cell_grid";
/// Cell grid ka header Grid id.
pub const TABLE_CELL_HEADER_GRID_ID: &str = "table_cell_header_grid";
/// Grid column gap: `SECTION_GAP` mirror (same value, koi naya rhythm nahi).
pub const TABLE_GRID_COL_GAP: f32 = 8.0;
/// Grid row gap: `TIGHT_GAP` mirror (same value, koi naya rhythm nahi).
pub const TABLE_GRID_ROW_GAP: f32 = 4.0;
/// Glyph placeholder: `CONTENT_LEAD` mirror (same two spaces, koi naya text
/// nahi) — header grid precedent jaisa neutral first column.
pub const TABLE_CELL_GLYPH_PAD: &str = "  ";

/// Grid columns: glyph column plus ek per data column. Khali widths par
/// sirf glyph (views.rs `table_grid_columns` wahi shape — owned contract
/// taaki cell-grid geometry cell paint ke saath rahe). Pure; unit-tested.
pub fn table_cell_grid_columns(widths: &[usize]) -> usize {
    widths.len().saturating_add(1)
}

/// Ek data cell ka paint faisla: key-col identity + find-hit state se ek
/// format. Hit format jeetta hai taaki block/line signal saaf rahe (key
/// identity key bar + badge se barkarar rehti hai). Pure; unit-tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCellPaint {
    /// Plain mono cell (na key, na hit).
    Plain,
    /// Key-col cell: mono + strong (header/key-bar precedent).
    KeyStrong,
    /// Current find hit: inverted block (no underline).
    FindCurrent,
    /// Other find hit: underline-only line (no block).
    FindOther,
}

/// `TableCellPaint` ka pure faisla: hit ho to current/other, warna key ho
/// to strong, warna plain. Pure; unit-tested.
pub fn table_cell_paint_kind(is_key: bool, has_hit: bool, is_current: bool) -> TableCellPaint {
    if has_hit {
        if is_current {
            TableCellPaint::FindCurrent
        } else {
            TableCellPaint::FindOther
        }
    } else if is_key {
        TableCellPaint::KeyStrong
    } else {
        TableCellPaint::Plain
    }
}

/// Cell grid ka find input: needle + case flag + resolved current cell.
/// `views.rs FindPaintCtx` precedent (Copy taaki grid loop me cheap reuse
/// ho). Caller find bar chalata hai; grid sirf highlight karta hai.
#[derive(Debug, Clone, Copy)]
pub struct TableCellFindCtx<'a> {
    /// Needle text (khali par koi hit nahi).
    pub needle: &'a str,
    /// Case-insensitive match ho ya nahi.
    pub ignore_case: bool,
    /// Current match cell (`table_find_current_cell` se).
    pub current: Option<(usize, usize)>,
}

/// Cell grid ka caller palette: same mono `FontId` + caller ke `fg/bg`
/// (`table_find_hit_format` precedent — views_table.rs me koi naya hue
/// nahi). Pure data; unit-tested decides nahi, paint use karta hai.
#[derive(Debug, Clone)]
pub struct TableCellStyle {
    /// Same mono font (koi naya size nahi).
    pub font: egui::FontId,
    /// Caller ka text color.
    pub fg: egui::Color32,
    /// Caller ka cell background.
    pub bg: egui::Color32,
}

/// Ek key-aware grid cell: mono hamesha, key col par strong (header +
/// key-bar selected precedent, koi naya color nahi).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_key_cell(ui: &mut egui::Ui, view: &TableKeyView, cell: &str, col: usize) {
    let mut rich = egui::RichText::new(cell).monospace();
    if is_key_column(view, col) {
        rich = rich.strong();
    }
    ui.label(rich);
}

/// Ek find-aware grid cell: khali needle par plain mono, warna
/// `table_find_cell_job` LayoutJob (current block, others
/// `TABLE_FIND_UNDERLINE_W` line).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_find_cell(
    ui: &mut egui::Ui,
    cell: &str,
    needle: &str,
    ignore_case: bool,
    is_current: bool,
    style: &TableCellStyle,
) {
    if needle.is_empty() {
        ui.label(egui::RichText::new(cell).monospace());
    } else {
        ui.label(table_find_cell_job(
            cell,
            needle,
            ignore_case,
            is_current,
            style.font.clone(),
            style.fg,
            style.bg,
        ));
    }
}

/// Ek data cell box: key + find dono se (`table_cell_paint_kind`
/// precedence — hit jeetta hai, key bar/badge identity rakhte hain).
/// `row`/`col` grid coords hain (data-row index + column index).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_data_cell(
    ui: &mut egui::Ui,
    view: &TableKeyView,
    cell: &str,
    row: usize,
    col: usize,
    find: TableCellFindCtx<'_>,
    style: &TableCellStyle,
) {
    let has_hit = !table_find_cell_spans(cell, find.needle, find.ignore_case).is_empty();
    let is_current = table_find_is_current((row, col), find.current);
    match table_cell_paint_kind(is_key_column(view, col), has_hit, is_current) {
        TableCellPaint::Plain => {
            ui.label(egui::RichText::new(cell).monospace());
        }
        TableCellPaint::KeyStrong => {
            paint_table_key_cell(ui, view, cell, col);
        }
        TableCellPaint::FindCurrent => {
            paint_table_find_cell(ui, cell, find.needle, find.ignore_case, true, style);
        }
        TableCellPaint::FindOther => {
            paint_table_find_cell(ui, cell, find.needle, find.ignore_case, false, style);
        }
    }
}

/// Cell-level table grid: key bar (live `paint_table_key_bar`) + header
/// Grid (strong mono cells) + data Grid (har cell box
/// `paint_table_data_cell` se — key strong, find underline/block). Widths
/// `table_column_widths` se, padding `pad_table_row` se (spans padded text
/// par nahi hilte — trailing pad only). Khali grid par honest note
/// (views.rs wording reuse, koi naya wording nahi); header-less widths par
/// single-string fallback (views.rs fallback precedent — column geometry
/// header ke bina undefined hai).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_cell_grid(
    ui: &mut egui::Ui,
    view: &mut TableKeyView,
    header: &[String],
    rows: &[Vec<String>],
    find: TableCellFindCtx<'_>,
    style: &TableCellStyle,
) {
    paint_table_key_bar(ui, view, header);
    let widths = table_column_widths(header, rows);
    if header.is_empty() && rows.is_empty() {
        ui.label(TypoRole::Body.rich("(both tables are empty)"));
        return;
    }
    if widths.is_empty() {
        for cells in rows.iter() {
            ui.label(egui::RichText::new(cells.join(" | ")).monospace());
        }
        return;
    }
    let cols = table_cell_grid_columns(&widths);
    paint_table_header_row(ui, header, &widths);
    egui::Grid::new(TABLE_CELL_GRID_ID)
        .num_columns(cols)
        .spacing([TABLE_GRID_COL_GAP, TABLE_GRID_ROW_GAP])
        .show(ui, |ui| {
            for (row_idx, cells) in rows.iter().enumerate() {
                ui.label(egui::RichText::new(TABLE_CELL_GLYPH_PAD).monospace());
                let padded = pad_table_row(cells, &widths);
                for (col_idx, cell) in padded.iter().enumerate() {
                    paint_table_data_cell(ui, view, cell, row_idx, col_idx, find, style);
                }
                ui.end_row();
            }
        });
}

// ----------------------------------------------------------------------------
// Table header row + compact toolbar (BQ wave: G-B1 + G-B2 + G-B3).
// Gap livecomp-BM TABLE G-B1/G-B2/G-B3: header data row ki tarah paint hota
// hai (`=` marker + dashed `----` rule, `views.rs` paint_table), Find + nav
// duplicate rows hain (app.rs Table branch + paint_table dono me Find +
// Prev/Next), cells Grid ke bajay flow me hain. Ye section paint pieces hai
// (caller wiring orchestrator ke paas rehti hai — signatures stable,
// `views.rs` untouched): header Grid me real header row (strong,
// small-caps intent — egui me small-caps nahi, isliye Mono strong stand-in;
// TypoRole alias reuse, koi naya size), Find + page nav ek compact toolbar
// row me (geometry find-bar precedent reuse), data Grid `TABLE_CELL_GRID_ID`
// reuse (header ka apna `TABLE_CELL_HEADER_GRID_ID`, dono kabhi takrate
// nahi). Colors frozen: koi Color32/Visuals yahan nahi. No I/O. Paint fns
// test-only until wiring lands (orchestrator owns paint); pure helpers
// unit-tested.
// ----------------------------------------------------------------------------

/// Header text role: grid-content Mono (gutter/hex `*_text_role`
/// precedent). Small-caps intent paint-side `.strong()` composition hai
/// (folder caption-strong precedent — naya size/token nahi; egui me
/// small-caps/letter-tracking nahi, `TYPO_LETTER_SPACING` 0.0). Pure;
/// unit-tested.
pub const fn table_header_text_role() -> TypoRole {
    TypoRole::Mono
}

/// Section-label role: key/find row labels (`Key:`, `Find:`) Body me
/// (13 default proportional), paint-side `.strong()` composition
/// (header Mono+strong precedent). Labels hint/badge (Small 11-weak) se
/// ek step upar taaki flat hierarchy tute; input values Mono family me
/// (needle field `.font(Monospace)`) taaki label-vs-input confuse na ho.
/// Koi naya size/token/color nahi. Pure; unit-tested.
pub const fn table_section_label_text_role() -> TypoRole {
    TypoRole::Body
}

/// Real header row Grid me: glyph placeholder + har header cell
/// [`table_header_text_role`] strong me. Koi `=` marker nahi, koi dashed
/// separator row nahi (G-B1: header data row nahi). Khali header par no-op
/// (honest — data grid khud empty note dikhata hai). `TABLE_CELL_GRID_ID`
/// ko chhoota nahi (header ka apna id, compose me takrao nahi).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_header_row(ui: &mut egui::Ui, header: &[String], widths: &[usize]) {
    if header.is_empty() {
        return;
    }
    // Column count data Grid wala single source hai (`widths` hamesha
    // `table_column_widths(header, rows)` se banta hai, jo har header cell
    // ke liye ek slot rakhta hai — isliye ye `header.len() + 1` ke barabar
    // hai, drift-proof).
    let cols = table_cell_grid_columns(widths);
    egui::Grid::new(TABLE_CELL_HEADER_GRID_ID)
        .num_columns(cols)
        .spacing([TABLE_GRID_COL_GAP, TABLE_GRID_ROW_GAP])
        .show(ui, |ui| {
            ui.label(table_header_text_role().rich(TABLE_CELL_GLYPH_PAD));
            for cell in format_table_header(header, widths) {
                ui.label(table_header_text_role().rich(cell).strong());
            }
            ui.end_row();
        });
}

/// Column letters (G-B4 groundwork): 0->A ... 25->Z, 26->AA, 27->AB —
/// spreadsheet order. Header-letter row ka label text hai; paint-side
/// [`table_header_text_role`] strong me (BR wave me, `views.rs`
/// NO-TOUCH is wave me). Koi naya size/token nahi. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_column_letter(col: usize) -> String {
    let mut out = String::new();
    let mut n = col.saturating_add(1);
    while n > 0 {
        let rem = n.saturating_sub(1) % 26;
        out.insert(0, (b'A'.saturating_add(rem as u8)) as char);
        n = n.saturating_sub(1) / 26;
    }
    out
}

/// Row-number gutter label (G-B4 groundwork): 0-based data row -> 1-based
/// label ("1", "2", ...). Glyph-pad gutter column
/// ([`TABLE_CELL_GLYPH_PAD`] precedent) ka text hai; paint-side Mono me
/// (gutter `gutter_text_role` precedent — wahan se copy nahi, sirf role
/// reuse). Koi naya size/token nahi. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_row_gutter_label(row: usize) -> String {
    row.saturating_add(1).to_string()
}

/// Page label text pager se: `views.rs` paint wording reuse
/// (`page {n} of {m}, showing {k} row(s)`), koi naya wording nahi.
/// [`TablePager::page_count`] + [`TablePager::visible_range`] reuse,
/// saturating only. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn format_table_page_label(pager: TablePager) -> String {
    let per_page = pager.rows_per_page.max(1);
    let page = pager.top.saturating_div(per_page).saturating_add(1);
    let pages = pager.page_count().max(1);
    let (start, end) = pager.visible_range();
    let showing = end.saturating_sub(start);
    format!("page {page} of {pages}, showing {showing} row(s)")
}

/// Compact table toolbar ka click result: Find pair + page pair, ek row.
/// Caller cursor/pager advance App state me karta hai
/// (`table_find_next_row` / `pager.next_page` precedent). Copy; unit-tested
/// defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableToolbarAction {
    /// Find Prev daba (needle cursor peeche).
    pub find_prev: bool,
    /// Find Next daba (needle cursor aage).
    pub find_next: bool,
    /// Page Prev daba (pager ek page peeche).
    pub page_prev: bool,
    /// Page Next daba (pager ek page aage).
    pub page_next: bool,
}

/// Compact table toolbar: Find needle + Prev/Next + page Prev/Next +
/// page label + N/M badge — sab ek [`TABLE_FIND_BAR_H`] row me (G-B2: ab do
/// Find bars / do nav rows nahi — caller ek hi toolbar chalata hai).
/// Find segment [`paint_table_find_bar`] wali single-source body hai (koi
/// duplicate Find row/badge nahi); button labels existing wording reuse
/// karte hain (page pair, koi naya wording nahi); bar height, needle
/// chaudai, col gap, badge pad find-bar precedent reuse. Badge right-end
/// small weak me, page label Small role me (status precedent). Returns
/// find + page clicks.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn paint_table_toolbar(
    ui: &mut egui::Ui,
    needle: &mut String,
    badge: &str,
    page_label: &str,
) -> TableToolbarAction {
    let mut action = TableToolbarAction::default();
    let avail = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), TABLE_FIND_BAR_H));
        let (find_prev, find_next) = paint_table_find_controls(ui, needle, badge);
        if find_prev {
            action.find_prev = true;
        }
        if find_next {
            action.find_next = true;
        }
        if ui.small_button("◀ Prev page").clicked() {
            action.page_prev = true;
        }
        if ui.small_button("Next page ▶").clicked() {
            action.page_next = true;
        }
        ui.label(TypoRole::Small.rich(page_label));
    });
    action
}

// ----------------------------------------------------------------------------
// Table xlsx/tsv source + sheet selector (headless; paint wiring stays with
// the orchestrator).
// Gap EXQ-L table half: table tab csv-only dikhta hai — tsv delimiter +
// multi-sheet xlsx sheet switch key-column parity ke liye chahiye. Engine
// (`compare::detect_table_delimiter`, `Workbook::from_xlsx`) parse kab ka
// karta hai; ye section uska GUI-side headless judvaan hai: path se source
// kind, kind se delimiter, sheet names par clamp/label, key-column reuse.
// Geometry first: sheet bar key-bar geometry reuse (wahi bar height, wahi
// col gap, wahi badge pad — koi naya rhythm nahi). Typography second:
// label Body-strong, sheet names mono (+strong selected), badge Small-role
// (key-bar hierarchy reuse); koi naya size/token/color nahi. No I/O
// (sheet bytes engine ke paas rehte hain); colors frozen (koi
// Color32/Visuals nahi). Paint caller ke locals par chalti hai (`&mut
// Option<usize>` selection + `&[String]` names), App/viewer struct me koi
// field edit nahi hota. Pure; unit-tested.
// ----------------------------------------------------------------------------

/// Table input source: path extension se (engine `detect_table_delimiter`
/// + `Workbook::from_xlsx` ka GUI-side judvaan — parse yahan nahi hota).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableSourceKind {
    /// Comma-delimited (`.csv` ya default).
    #[default]
    Csv,
    /// Tab-delimited (`.tsv` / `.tab`).
    Tsv,
    /// Multi-sheet workbook (`.xlsx`).
    Xlsx,
    /// Bina pehchane extension (csv jaisa treat hota hai).
    Unknown,
}

/// Path se source kind: extension lowercase karke (`tsv`/`tab` -> Tsv,
/// `xlsx` -> Xlsx, `csv` -> Csv, baaki/empty -> Unknown). Query/params
/// nahi hat-te (dialog paths hamesha plain hote hain). Pure.
pub fn table_source_kind_for_path(path: &str) -> TableSourceKind {
    let trimmed = path.trim();
    let ext = trimmed.rsplit('.').next().unwrap_or("").trim();
    if ext.len() == trimmed.len() {
        return TableSourceKind::Unknown;
    }
    let mut lower = String::new();
    for ch in ext.chars() {
        lower.push(ch.to_ascii_lowercase());
    }
    match lower.as_str() {
        "tsv" | "tab" => TableSourceKind::Tsv,
        "xlsx" => TableSourceKind::Xlsx,
        "csv" => TableSourceKind::Csv,
        _ => TableSourceKind::Unknown,
    }
}

/// Source ka field delimiter: Tsv par tab, baaki par comma (xlsx sheets
/// engine parse karta hai — delimiter wahan irrelevant hai, badge honest
/// `xlsx` rehta hai). Pure.
pub fn table_delimiter_for_source(kind: TableSourceKind) -> char {
    match kind {
        TableSourceKind::Tsv => '\t',
        TableSourceKind::Csv | TableSourceKind::Xlsx | TableSourceKind::Unknown => ',',
    }
}

/// Source badge text sheet bar ke right-end ke liye: `csv`/`tsv`/`xlsx`/
/// unknown par `auto`. Pure.
pub fn table_source_badge_label(kind: TableSourceKind) -> &'static str {
    match kind {
        TableSourceKind::Csv => "csv",
        TableSourceKind::Tsv => "tsv",
        TableSourceKind::Xlsx => "xlsx",
        TableSourceKind::Unknown => "auto",
    }
}

/// Sheet selector bar height: key-bar precedent reuse (koi naya token nahi).
pub const TABLE_SHEET_BAR_H: f32 = TABLE_KEY_BAR_H;
/// Sheet buttons ke beech gap: key-col gap precedent reuse.
pub const TABLE_SHEET_COL_GAP: f32 = TABLE_KEY_COL_GAP;
/// Sheet badge right-end pad: key badge precedent reuse.
pub const TABLE_SHEET_BADGE_PAD: f32 = TABLE_KEY_BADGE_PAD;

/// Sheet bar section-label role: Body (key-bar label precedent reuse).
/// Paint-side `.strong()` composition; hint/badge Small se ek step upar.
/// Koi naya size/token/color nahi. Pure.
pub const fn table_sheet_label_text_role() -> TypoRole {
    TypoRole::Body
}

/// Sheet name role: Mono (key-bar cell precedent reuse — selected par
/// `.strong()` composition, plain par mono). Koi naya size/token nahi.
pub const fn table_sheet_value_text_role() -> TypoRole {
    TypoRole::Mono
}

/// Sheet names ki ginti (workbook sheets ya single csv/tsv par 1).
/// Khali par 0 (paint honest empty note dikhata hai). Pure.
pub fn table_sheet_count(names: &[String]) -> usize {
    names.len()
}

/// Sheet index clamp: valid par `Some(index)`, khali/out-of-range par
/// `None` (paint bahar select par honest no-op). Pure.
pub fn table_sheet_clamp(names: &[String], index: usize) -> Option<usize> {
    if names.get(index).is_some() {
        Some(index)
    } else {
        None
    }
}

/// Sheet label text: `names[index]` ka clone, OOB par `None`.
/// Pure.
pub fn table_sheet_label(names: &[String], index: usize) -> Option<String> {
    names.get(index).cloned()
}

/// Sheet selector bar: label + per-sheet single-select + source badge.
/// Key bar (`paint_table_key_bar`) ka geometry/typo mirror (wahi bar
/// height, wahi col gap, wahi badge right-end Small-role me; label
/// Body-strong, names Mono (+strong selected), marker reuse
/// `key_column_marker(selected)` taaki indicator column align rahe).
/// Selection caller ke `&mut Option<usize>` me likhti hai (App fields
/// untouched); `None` = koi sheet select nahi (honest empty).
/// Wiring-pending until orchestrator wires the table tab caller.
pub fn paint_table_sheet_bar(
    ui: &mut egui::Ui,
    selected: &mut Option<usize>,
    sheets: &[String],
    source: TableSourceKind,
) {
    let avail = ui.available_width();
    let mut clicked: Option<usize> = None;
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), TABLE_SHEET_BAR_H));
        ui.spacing_mut().item_spacing.x = TABLE_SHEET_COL_GAP;
        ui.label(table_sheet_label_text_role().rich("Sheet:").strong());
        for (idx, name) in sheets.iter().enumerate() {
            let is_selected = *selected == Some(idx);
            let marker = key_column_marker(is_selected);
            let text = format!("{marker} {name}");
            let rich = if is_selected {
                egui::RichText::new(text).monospace().strong()
            } else {
                egui::RichText::new(text).monospace()
            };
            if ui.selectable_label(is_selected, rich).clicked() {
                clicked = Some(idx);
            }
            ui.add_space(TABLE_SHEET_COL_GAP);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(TABLE_SHEET_BADGE_PAD);
            ui.label(TypoRole::Small.rich(table_source_badge_label(source)));
        });
    });
    if let Some(idx) = clicked {
        *selected = table_sheet_clamp(sheets, idx);
    }
}

// ----------------------------------------------------------------------------
// Table cell geometry (pure rect math; paint wiring stays with the
// orchestrator).
// Gap AY re-scope: table body Grid paint `views.rs` me hai
// (`paint_table_keyed:1398`, `paint_table:1866` — `table_grid_columns` +
// `table_row_cells` se glyph + padded mono cells). Ye section us Grid ka
// headless judvaan hai: padded-cell box, key-col vertical box, aur
// find-underline strips — taaki agla views.rs wave `ui.painter()` se key
// boxes + hit underlines draw kare bina yahan egui ctx laaye. Geometry
// first: origin + px widths + row/gap rhythm se boxes; underline motai
// `TABLE_FIND_UNDERLINE_W` reuse (koi naya size/token nahi). Typography
// second: kuch nahi (rect math me font nahi — `char_w` caller measure
// karke deta hai). Colors frozen (koi Color32/Visuals nahi). No egui ctx,
// no I/O. `.get()` + saturating ops only. Paint fns ki tarah test-only
// until wiring lands (orchestrator owns paint); pure helpers unit-tested.
// ----------------------------------------------------------------------------

/// Padded cell box px me: Grid origin + rhythm se ek data cell ka paint
/// rect. `char_w` yahan nahi — `col_widths_px` caller px me deta hai
/// (font measure paint-side rehta hai). Pure data; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableCellRect {
    /// Left edge px.
    pub x: f32,
    /// Top edge px.
    pub y: f32,
    /// Width px (`col_widths_px[col]`).
    pub w: f32,
    /// Height px (`row_h`).
    pub h: f32,
}

/// Cell Grid ka px rhythm: origin + row height + gaps. Paint
/// `TABLE_GRID_COL_GAP`/`TABLE_GRID_ROW_GAP` precedent se banata hai;
/// struct me bandhne se call-sites 4 args me rehte hain (clippy
/// `too_many_arguments` bina `allow` ke clean). Pure data; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableCellGrid {
    /// Grid top-left x px.
    pub origin_x: f32,
    /// Grid top-left y px.
    pub origin_y: f32,
    /// Ek row ki unchai px.
    pub row_h: f32,
    /// Data cols ke beech gap px.
    pub col_gap: f32,
    /// Data rows ke beech gap px.
    pub row_gap: f32,
}

/// Ek data cell ka px box: `grid` origin + rhythm hai, `col_widths_px`
/// har data col ki px chaudai (glyph column isme nahi — paint usko
/// `CONTENT_LEAD` precedent se alag rakhta hai). Kharab input
/// (non-finite, `row_h <= 0`, negative gap, range-bahar `col`) par
/// `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_cell_rect(
    grid: TableCellGrid,
    col: usize,
    row: usize,
    col_widths_px: &[f32],
) -> Option<TableCellRect> {
    if !grid.origin_x.is_finite()
        || !grid.origin_y.is_finite()
        || !grid.row_h.is_finite()
        || !grid.col_gap.is_finite()
        || !grid.row_gap.is_finite()
    {
        return None;
    }
    if grid.row_h <= 0.0 || grid.col_gap < 0.0 || grid.row_gap < 0.0 {
        return None;
    }
    let w = *col_widths_px.get(col)?;
    if !w.is_finite() || w <= 0.0 {
        return None;
    }
    let mut x = grid.origin_x;
    let mut idx = 0usize;
    while idx < col {
        let cw = col_widths_px.get(idx)?;
        if !cw.is_finite() || *cw < 0.0 {
            return None;
        }
        x += *cw + grid.col_gap;
        idx = idx.saturating_add(1);
    }
    let stride = grid.row_h + grid.row_gap;
    let y = grid.origin_y + row as f32 * stride;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(TableCellRect {
        x,
        y,
        w,
        h: grid.row_h,
    })
}

/// Ek key column ka vertical box: us col ki `x`/`w` se, `y = origin_y`
/// se, unchai poore data window par (`total_rows * row_h` + gaps).
/// Caller `is_key_column` se gate karta hai (geometry khud neutral hai).
/// Khali grid ya range-bahar col par `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_key_col_box_rect(
    grid: TableCellGrid,
    col: usize,
    col_widths_px: &[f32],
    total_rows: usize,
) -> Option<TableCellRect> {
    if total_rows == 0 {
        return None;
    }
    let head = table_cell_rect(grid, col, 0, col_widths_px)?;
    let rows_f = total_rows as f32;
    if !rows_f.is_finite() || rows_f <= 0.0 {
        return None;
    }
    let gaps = (total_rows.saturating_sub(1)) as f32 * grid.row_gap;
    let h = rows_f * grid.row_h + gaps;
    if !h.is_finite() || h <= 0.0 {
        return None;
    }
    Some(TableCellRect {
        x: head.x,
        y: grid.origin_y,
        w: head.w,
        h,
    })
}

/// Sab key cols ke vertical boxes, `view.key_columns` order me.
/// Range-bahar col skip hota hai (panic nahi); khali keys par khali vec.
/// `is_key_column` ka box judvaan — paint isi list par key boxes draw
/// karta hai. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_key_box_rects(
    view: &TableKeyView,
    grid: TableCellGrid,
    col_widths_px: &[f32],
    total_rows: usize,
) -> Vec<TableCellRect> {
    let mut out = Vec::new();
    for col in view.key_columns.iter() {
        if let Some(box_rect) = table_key_col_box_rect(grid, *col, col_widths_px, total_rows) {
            out.push(box_rect);
        }
    }
    out
}

/// Ek find-hit span ka underline strip: cell ke bottom par
/// `TABLE_FIND_UNDERLINE_W` uncha box, `x`/`w` char offsets se
/// (`x = cell.x + start * char_w`, `w = (end - start) * char_w`,
/// cell right par clamp). Span clamp `table_find_span_in_padded` reuse
/// (cap ke bahar/ulta par `None`). Kharab `char_w`/cell par `None`.
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_underline_rect(
    cell: TableCellRect,
    span_start: usize,
    span_end: usize,
    width_chars: usize,
    char_w: f32,
) -> Option<TableCellRect> {
    if !cell.x.is_finite() || !cell.y.is_finite() || !cell.w.is_finite() || !cell.h.is_finite() {
        return None;
    }
    if cell.w <= 0.0 || cell.h <= 0.0 {
        return None;
    }
    if !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    if !TABLE_FIND_UNDERLINE_W.is_finite() || TABLE_FIND_UNDERLINE_W <= 0.0 {
        return None;
    }
    let (start, end) = table_find_span_in_padded(span_start, span_end, width_chars)?;
    let x = cell.x + start as f32 * char_w;
    let mut w = (end.saturating_sub(start)) as f32 * char_w;
    let right = cell.x + cell.w;
    if !x.is_finite() || !w.is_finite() {
        return None;
    }
    if x < cell.x || x >= right {
        return None;
    }
    if x + w > right {
        w = right - x;
    }
    if w <= 0.0 {
        return None;
    }
    let mut y = cell.y + cell.h - TABLE_FIND_UNDERLINE_W;
    if y < cell.y {
        y = cell.y;
    }
    Some(TableCellRect {
        x,
        y,
        w,
        h: TABLE_FIND_UNDERLINE_W,
    })
}

/// Ek cell ke sab spans ke underline strips, span order me. Kharab
/// `char_w` par khali vec (koi fake rect nahi); clamp-bahar spans skip
/// hote hain. `table_find_row_spans`/`table_find_grid_spans` ke `(start,
/// end)` isi me aate hain. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn table_find_underline_rects_for_cell(
    cell: TableCellRect,
    spans: &[(usize, usize)],
    width_chars: usize,
    char_w: f32,
) -> Vec<TableCellRect> {
    let mut out = Vec::new();
    if spans.is_empty() {
        return out;
    }
    if !char_w.is_finite() || char_w <= 0.0 {
        return out;
    }
    for span in spans.iter() {
        if let Some(rect) = table_find_underline_rect(cell, span.0, span.1, width_chars, char_w) {
            out.push(rect);
        }
    }
    out
}

// ----------------------------------------------------------------------------
// Per-hunk merge (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #10: table tab me har modified/orphan row par
// take-left/take-right. Ye pure row choice hai; file apply (I/O)
// wiring ka kaam hai aur OPEN rehta hai. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Per-row merge choice: kaunsi side ki row rakhi jaye.
/// Test-only until paint wiring lands (orchestrator owns paint).
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

    #[test]
    fn table_key_marker_distinguishes_selected() {
        let on = key_column_marker(true);
        let off = key_column_marker(false);
        assert_ne!(on, off);
        // Dono single-char: indicator column align rehta hai.
        assert_eq!(on.chars().count(), 1);
        assert_eq!(off.chars().count(), 1);
    }

    #[test]
    fn table_key_badge_labels_keyed_vs_positional() {
        let plain = TableKeyView::new();
        assert_eq!(table_key_badge_label(&plain), "positional");
        let mut keyed = TableKeyView::new();
        keyed.toggle_key_column(2);
        keyed.toggle_key_column(0);
        assert_eq!(table_key_badge_label(&keyed), "key: 0,2");
        let mut single = TableKeyView::new();
        single.toggle_key_column(1);
        assert_eq!(table_key_badge_label(&single), "key: 1");
        // Deselect par wapas positional.
        single.toggle_key_column(1);
        assert_eq!(table_key_badge_label(&single), "positional");
    }

    #[test]
    fn table_key_bar_state_helpers_are_honest() {
        let mut view = TableKeyView::new();
        assert!(!is_key_column(&view, 0));
        assert!(view.positional_note().is_some());
        view.toggle_key_column(1);
        assert!(is_key_column(&view, 1));
        assert!(!is_key_column(&view, 0));
        assert!(view.positional_note().is_none());
        let (left, _) = sample_tables();
        view.suggest_for(&left);
        assert!(is_suggested_key(&view, 0));
        assert!(!is_suggested_key(&view, 5));
        // Geometry tokens find-bar precedent reuse karte hain.
        assert_eq!(TABLE_KEY_BAR_H, 28.0);
        assert_eq!(TABLE_KEY_BADGE_PAD, 4.0);
        assert_eq!(TABLE_KEY_COL_GAP, 8.0);
    }

    #[test]
    fn tablefind_cell_spans_empty_needle_gives_no_hits() {
        assert!(table_find_cell_spans("apple pie", "", false).is_empty());
        assert!(table_find_cell_spans("", "apple", false).is_empty());
        assert!(table_find_cell_spans("", "", false).is_empty());
        // Needle cell se lambi ho to bhi empty (panic nahi).
        assert!(table_find_cell_spans("hi", "hello world", false).is_empty());
    }

    #[test]
    fn tablefind_cell_spans_case_modes() {
        // Case-sensitive: sirf exact case.
        assert_eq!(
            table_find_cell_spans("Banana banana", "banana", false),
            vec![(7, 13)]
        );
        // Ignore-case: dono milte hain, indices original cell par.
        assert_eq!(
            table_find_cell_spans("Banana banana", "banana", true),
            vec![(0, 6), (7, 13)]
        );
        // Multibyte: char indices hain (byte nahi).
        assert_eq!(
            table_find_cell_spans("café au lait", "au", false),
            vec![(5, 7)]
        );
    }

    #[test]
    fn tablefind_cell_spans_overlapping_ok() {
        // Overlap allowed: har offset se ek window, sorted + clamped.
        assert_eq!(
            table_find_cell_spans("aaa", "aa", false),
            vec![(0, 2), (1, 3)]
        );
        let spans = table_find_cell_spans("apple pie apple", "apple", false);
        assert_eq!(spans, vec![(0, 5), (10, 15)]);
        for (start, end) in spans {
            assert!(start < end);
            assert!(end <= "apple pie apple".chars().count());
        }
    }

    #[test]
    fn tablefind_grid_spans_map_rows_and_cols() {
        let rows = vec![
            vec!["ID".to_string(), "Price".to_string()],
            vec!["apple".to_string(), "9.99".to_string()],
            vec!["Banana".to_string(), "apple pie".to_string()],
        ];
        assert_eq!(
            table_find_grid_spans(&rows, "apple", false),
            vec![(1, 0, 0, 5), (2, 1, 0, 5)]
        );
        // Row-level: (col, start, end) column order me.
        let row_last: &[String] = rows.get(2).map(Vec::as_slice).unwrap_or(&[]);
        let row_first: &[String] = rows.first().map(Vec::as_slice).unwrap_or(&[]);
        assert_eq!(
            table_find_row_spans(row_last, "apple", false),
            vec![(1, 0, 5)]
        );
        assert!(table_find_row_spans(row_first, "apple", false).is_empty());
        // Khali needle: khali grid spans (koi fake count nahi).
        assert!(table_find_grid_spans(&rows, "", false).is_empty());
        // Row-major order barkarar (row 1 col 0, phir row 2 col 1).
        let spans = table_find_grid_spans(&rows, "a", true);
        let mut prev = (0usize, 0usize);
        for (row, col, _, _) in spans {
            assert!((row, col) >= prev);
            prev = (row, col);
        }
    }

    #[test]
    fn tablefind_current_cell_tracks_cursor_with_wrap() {
        let matches = vec![(1, 0), (2, 1), (2, 0)];
        assert_eq!(table_find_current_cell(&matches, 0), Some((1, 0)));
        assert_eq!(table_find_current_cell(&matches, 2), Some((2, 0)));
        // Bada cursor wrap hota hai.
        assert_eq!(table_find_current_cell(&matches, 3), Some((1, 0)));
        assert_eq!(table_find_current_cell(&matches, 4), Some((2, 1)));
        // Khali list: None.
        assert_eq!(table_find_current_cell(&[], 0), None);
        // Current-vs-other distinction.
        assert!(table_find_is_current((1, 0), Some((1, 0))));
        assert!(!table_find_is_current((2, 1), Some((1, 0))));
        assert!(!table_find_is_current((1, 0), None));
    }

    #[test]
    fn tablefind_current_other_formats_differ() {
        let font = egui::FontId::monospace(14.0);
        let fg = egui::Color32::WHITE;
        let bg = egui::Color32::BLACK;
        let current = table_find_hit_format(font.clone(), fg, bg, true);
        let other = table_find_hit_format(font.clone(), fg, bg, false);
        // Same mono FontId, koi naya size nahi.
        assert_eq!(current.font_id, other.font_id);
        assert_eq!(current.font_id, font);
        // Block-vs-line: current inverted block (no underline),
        // other underline-only line (no block).
        assert_eq!((current.color, current.background), (bg, fg));
        assert_eq!(current.underline.width, 0.0);
        assert_eq!((other.color, other.background), (fg, bg));
        assert_eq!(other.underline.width, TABLE_FIND_UNDERLINE_W);
        assert_eq!(other.underline.color, fg);
        // Underline precedent mirror: same 1px value.
        assert_eq!(TABLE_FIND_UNDERLINE_W, 1.0);
    }

    #[test]
    fn tablefind_padded_spans_stay_aligned_with_key_cols() {
        // Trailing pad: andar ke spans nahi hilte.
        assert_eq!(table_find_span_in_padded(0, 5, 8), Some((0, 5)));
        assert_eq!(table_find_span_in_padded(3, 5, 8), Some((3, 5)));
        // Cap par clamp (TABLE_MAX_COL_CHARS precedent).
        assert_eq!(
            table_find_span_in_padded(0, 99, TABLE_MAX_COL_CHARS),
            Some((0, TABLE_MAX_COL_CHARS))
        );
        // Poora bahar ya ulta span: None.
        assert_eq!(table_find_span_in_padded(8, 10, 8), None);
        assert_eq!(table_find_span_in_padded(5, 5, 8), None);
        assert_eq!(table_find_span_in_padded(6, 3, 8), None);
        // Key-col hits ko wahi highlight: alignment grid se.
        let mut view = TableKeyView::new();
        view.toggle_key_column(0);
        assert!(table_find_hit_in_key_col(&view, 0));
        assert!(!table_find_hit_in_key_col(&view, 1));
        // Padded cells par spans wahi rehte hain (pad trailing-only).
        let widths = vec![8, 8];
        let cells = vec!["apple".to_string(), "9.99".to_string()];
        let padded = pad_table_row(&cells, &widths);
        for (col, start, end) in table_find_row_spans(&cells, "apple", false) {
            let width = widths.get(col).copied().unwrap_or(0);
            let clamped = table_find_span_in_padded(start, end, width);
            assert_eq!(clamped, Some((start, end)));
            let _ = padded.get(col);
        }
    }

    #[test]
    fn tablefind_cell_job_marks_current_vs_others() {
        let font = egui::FontId::monospace(14.0);
        let fg = egui::Color32::WHITE;
        let bg = egui::Color32::BLACK;
        // Current cell: hit inverted block.
        let job = table_find_cell_job("apple pie", "apple", false, true, font.clone(), fg, bg);
        assert_eq!(job.text, "apple pie");
        assert_eq!(job.sections.len(), 2);
        let hit = job.sections.first();
        assert!(hit.is_some());
        if let Some(sec) = hit {
            assert_eq!((sec.format.color, sec.format.background), (bg, fg));
            assert_eq!(sec.format.underline.width, 0.0);
            assert_eq!(
                job.text.get(sec.byte_range.start.0..sec.byte_range.end.0),
                Some("apple")
            );
        }
        // Other cell: hit underline-only line.
        let job = table_find_cell_job("apple pie", "apple", false, false, font.clone(), fg, bg);
        assert_eq!(job.text, "apple pie");
        assert_eq!(job.sections.len(), 2);
        if let Some(sec) = job.sections.first() {
            assert_eq!((sec.format.color, sec.format.background), (fg, bg));
            assert_eq!(sec.format.underline.width, TABLE_FIND_UNDERLINE_W);
        }
        // Khali needle: single plain section.
        let job = table_find_cell_job("apple", "", false, false, font.clone(), fg, bg);
        assert_eq!(job.text, "apple");
        assert_eq!(job.sections.len(), 1);
        // Overlap me pehla hit jeetta hai: job text cell ke barabar.
        let job = table_find_cell_job("aaa", "aa", false, false, font, fg, bg);
        assert_eq!(job.text, "aaa");
    }

    #[test]
    fn tablecell_grid_columns_count_glyph_plus_data() {
        // Glyph column + ek per data column; khali par sirf glyph.
        assert_eq!(table_cell_grid_columns(&[]), 1);
        assert_eq!(table_cell_grid_columns(&[3, 5]), 3);
        assert_eq!(table_cell_grid_columns(&[4]), 2);
        // Geometry mirrors: views.rs SECTION_GAP/TIGHT_GAP/CONTENT_LEAD values.
        assert_eq!(TABLE_GRID_COL_GAP, 8.0);
        assert_eq!(TABLE_GRID_ROW_GAP, 4.0);
        assert_eq!(TABLE_CELL_GLYPH_PAD, "  ");
        assert_ne!(TABLE_CELL_GRID_ID, TABLE_CELL_HEADER_GRID_ID);
    }

    #[test]
    fn tablecell_paint_kind_precedence_hit_wins_over_key() {
        assert_eq!(
            table_cell_paint_kind(false, false, false),
            TableCellPaint::Plain
        );
        assert_eq!(
            table_cell_paint_kind(true, false, false),
            TableCellPaint::KeyStrong
        );
        assert_eq!(
            table_cell_paint_kind(false, true, false),
            TableCellPaint::FindOther
        );
        assert_eq!(
            table_cell_paint_kind(false, true, true),
            TableCellPaint::FindCurrent
        );
        // Hit format jeetta hai taaki block/line signal saaf rahe; key
        // identity key bar + badge se barkarar rehti hai.
        assert_eq!(
            table_cell_paint_kind(true, true, false),
            TableCellPaint::FindOther
        );
        assert_eq!(
            table_cell_paint_kind(true, true, true),
            TableCellPaint::FindCurrent
        );
    }

    #[test]
    fn tablecell_find_ctx_resolves_current_cell_in_key_grid() {
        let rows = vec![
            vec!["1".to_string(), "anna".to_string()],
            vec!["2".to_string(), "bob".to_string()],
        ];
        let mut view = TableKeyView::new();
        view.toggle_key_column(0);
        let find = TableCellFindCtx {
            needle: "bob",
            ignore_case: false,
            current: None,
        };
        let matches = table_find_match_rows(&rows, find.needle, find.ignore_case);
        assert_eq!(matches, vec![(1, 1)]);
        let current = table_find_current_cell(&matches, 0);
        assert_eq!(current, Some((1, 1)));
        assert!(table_find_is_current((1, 1), current));
        assert!(!table_find_is_current((0, 0), current));
        // Key-col hit ko wahi underline/block milta hai (koi alag color nahi).
        assert!(table_find_hit_in_key_col(&view, 0));
        assert!(!table_find_hit_in_key_col(&view, 1));
        // Kind mapping grid coords par: key col plain cell strong, hit cell
        // current-None par other (underline line).
        assert_eq!(
            table_cell_paint_kind(is_key_column(&view, 0), false, false),
            TableCellPaint::KeyStrong
        );
        let spans = table_find_cell_spans("bob", find.needle, find.ignore_case);
        assert_eq!(spans, vec![(0, 3)]);
        assert_eq!(
            table_cell_paint_kind(is_key_column(&view, 1), !spans.is_empty(), false),
            TableCellPaint::FindOther
        );
    }

    #[test]
    fn tablecell_geo_cell_rect_maps_col_row_with_gaps() {
        let widths = vec![100.0, 60.0];
        let grid = TableCellGrid {
            origin_x: 10.0,
            origin_y: 20.0,
            row_h: 18.0,
            col_gap: 8.0,
            row_gap: 4.0,
        };
        // (col 0, row 0) origin par; (col 1) pehle col + gap ke baad.
        assert_eq!(
            table_cell_rect(grid, 0, 0, &widths),
            Some(TableCellRect {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 18.0,
            })
        );
        assert_eq!(
            table_cell_rect(grid, 1, 2, &widths),
            Some(TableCellRect {
                x: 118.0,
                y: 64.0,
                w: 60.0,
                h: 18.0,
            })
        );
        // Range-bahar col / kharab rhythm par None (panic nahi).
        assert_eq!(table_cell_rect(grid, 5, 0, &widths), None);
        assert_eq!(table_cell_rect(grid, 0, 0, &[]), None);
        let flat = TableCellGrid { row_h: 0.0, ..grid };
        assert_eq!(table_cell_rect(flat, 0, 0, &widths), None);
        let neg_gap = TableCellGrid {
            col_gap: -1.0,
            ..grid
        };
        assert_eq!(table_cell_rect(neg_gap, 0, 0, &widths), None);
        let nan_origin = TableCellGrid {
            origin_x: f32::NAN,
            ..grid
        };
        assert_eq!(table_cell_rect(nan_origin, 0, 0, &widths), None);
    }

    #[test]
    fn tablecell_geo_key_box_covers_key_cols_only() {
        let widths = vec![100.0, 60.0];
        let grid = TableCellGrid {
            origin_x: 10.0,
            origin_y: 20.0,
            row_h: 18.0,
            col_gap: 8.0,
            row_gap: 4.0,
        };
        // Col 0 ka box poore 3 rows par: h = 3*18 + 2*4 = 62.
        assert_eq!(
            table_key_col_box_rect(grid, 0, &widths, 3),
            Some(TableCellRect {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 62.0,
            })
        );
        // Khali grid / range-bahar col par None.
        assert_eq!(table_key_col_box_rect(grid, 0, &widths, 0), None);
        assert_eq!(table_key_col_box_rect(grid, 7, &widths, 3), None);
        // View ke key cols hi boxes banate hain (order barkarar).
        let mut view = TableKeyView::new();
        assert!(table_key_box_rects(&view, grid, &widths, 3).is_empty());
        view.toggle_key_column(1);
        view.toggle_key_column(0);
        let boxes = table_key_box_rects(&view, grid, &widths, 3);
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes.first().map(|b| b.x), Some(10.0));
        assert_eq!(boxes.get(1).map(|b| b.x), Some(118.0));
        for box_rect in boxes {
            assert_eq!(box_rect.y, 20.0);
            assert_eq!(box_rect.h, 62.0);
        }
    }

    #[test]
    fn tablecell_geo_find_underline_clamps_to_cell() {
        let cell = TableCellRect {
            x: 10.0,
            y: 20.0,
            w: 80.0,
            h: 18.0,
        };
        // char_w 8: span (0,5) -> x=10, w=40, bottom strip 1px.
        assert_eq!(
            table_find_underline_rect(cell, 0, 5, 8, 8.0),
            Some(TableCellRect {
                x: 10.0,
                y: 37.0,
                w: 40.0,
                h: TABLE_FIND_UNDERLINE_W,
            })
        );
        // Cap par clamp: span (6,99) width 8 par (6,8) me kat-ta hai.
        let clamped = table_find_underline_rect(cell, 6, 99, 8, 8.0);
        assert!(clamped.is_some());
        if let Some(rect) = clamped {
            assert!(rect.x + rect.w <= cell.x + cell.w + 0.001);
            assert_eq!(rect.h, TABLE_FIND_UNDERLINE_W);
        }
        // Poora bahar / ulta span / kharab char_w par None.
        assert_eq!(table_find_underline_rect(cell, 8, 10, 8, 8.0), None);
        assert_eq!(table_find_underline_rect(cell, 5, 5, 8, 8.0), None);
        assert_eq!(table_find_underline_rect(cell, 6, 3, 8, 8.0), None);
        assert_eq!(table_find_underline_rect(cell, 0, 5, 8, 0.0), None);
        // Cell list: order barkarar, bahar spans skip.
        let spans = vec![(0, 2), (8, 10), (4, 6)];
        let rects = table_find_underline_rects_for_cell(cell, &spans, 8, 8.0);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects.first().map(|r| r.x), Some(10.0));
        assert_eq!(rects.get(1).map(|r| r.x), Some(42.0));
        assert!(table_find_underline_rects_for_cell(cell, &[], 8, 8.0).is_empty());
        assert!(table_find_underline_rects_for_cell(cell, &spans, 8, 0.0).is_empty());
    }

    #[test]
    fn table_header_role_reuses_mono_grid_content() {
        // TypoRole reuse: header Mono grid content hai (gutter/hex
        // `*_text_role` precedent); small-caps intent paint-side `.strong()`
        // composition hai (folder caption-strong precedent) — koi naya
        // size/token nahi.
        assert_eq!(table_header_text_role(), TypoRole::Mono);
        assert_eq!(table_header_text_role(), TypoRole::content_grid());
        assert!(table_header_text_role().is_mono());
        assert_eq!(
            table_header_text_role().zoomed_point_size(0),
            crate::theme::TYPO_MONO_SIZE
        );
        // Grid ids kabhi takrate nahi (header vs data Compose).
        assert_ne!(TABLE_CELL_GRID_ID, TABLE_CELL_HEADER_GRID_ID);
    }

    #[test]
    fn table_section_label_role_steps_above_hint() {
        // Hierarchy: section labels Body (13) hint/badge Small (11) se
        // ek step upar, mono content se alag family; status notes Body
        // (weak hata — weight se contrast, color frozen).
        use crate::theme::{TYPO_BODY_SIZE, TYPO_SMALL_SIZE};
        assert_eq!(table_section_label_text_role(), TypoRole::Body);
        assert_eq!(table_section_label_text_role().size(), TYPO_BODY_SIZE);
        assert!(table_section_label_text_role().size() > TYPO_SMALL_SIZE);
        assert!(!table_section_label_text_role().is_mono());
        assert!(table_section_label_text_role().is_chrome());
        assert_eq!(TypoRole::Body.size(), TypoRole::Small.size() + 2.0);
    }

    #[test]
    fn table_page_label_mirrors_pager_state() {
        let mut pager = TablePager::new(25, 10);
        assert_eq!(
            format_table_page_label(pager),
            "page 1 of 3, showing 10 row(s)"
        );
        pager.next_page();
        assert_eq!(
            format_table_page_label(pager),
            "page 2 of 3, showing 10 row(s)"
        );
        pager.next_page();
        assert_eq!(
            format_table_page_label(pager),
            "page 3 of 3, showing 5 row(s)"
        );
        // Khali grid: honest 0 rows, pages floor 1 (views.rs precedent).
        let empty = TablePager::new(0, 10);
        assert_eq!(
            format_table_page_label(empty),
            "page 1 of 1, showing 0 row(s)"
        );
    }

    #[test]
    fn table_column_letters_follow_spreadsheet_order() {
        assert_eq!(table_column_letter(0), "A");
        assert_eq!(table_column_letter(1), "B");
        assert_eq!(table_column_letter(25), "Z");
        assert_eq!(table_column_letter(26), "AA");
        assert_eq!(table_column_letter(27), "AB");
        // Har letter non-empty uppercase ASCII hai (paint-side Mono strong).
        for col in 0..60usize {
            let label = table_column_letter(col);
            assert!(!label.is_empty());
            assert!(label.chars().all(|c| c.is_ascii_uppercase()));
        }
    }

    #[test]
    fn table_row_gutter_labels_are_one_based_and_grids_share_columns() {
        assert_eq!(table_row_gutter_label(0), "1");
        assert_eq!(table_row_gutter_label(9), "10");
        // Header/data Grid column count ek hi source se (dedup: header Grid
        // ab `table_cell_grid_columns` reuse karta hai).
        let header = vec!["ID".to_string(), "Price".to_string()];
        let widths = table_column_widths(&header, &[]);
        assert_eq!(table_cell_grid_columns(&widths), header.len() + 1);
    }

    #[test]
    fn table_toolbar_defaults_quiet_and_reuses_find_geometry() {
        // Koi click nahi to sab false (caller App state untouched rakhta hai).
        let action = TableToolbarAction::default();
        assert!(!action.find_prev);
        assert!(!action.find_next);
        assert!(!action.page_prev);
        assert!(!action.page_next);
        // Toolbar geometry find-bar precedent reuse karti hai (single bar —
        // G-B2 duplicate rows ka fix, koi naya token nahi).
        assert_eq!(TABLE_FIND_BAR_H, TABLE_KEY_BAR_H);
        assert_eq!(TABLE_FIND_NEEDLE_W, 120.0);
        assert_eq!(TABLE_FIND_BADGE_PAD, TABLE_KEY_BADGE_PAD);
        assert_eq!(TABLE_KEY_COL_GAP, 8.0);
    }

    #[test]
    fn table_source_kind_and_sheet_helpers_stay_honest() {
        assert_eq!(table_source_kind_for_path("a.csv"), TableSourceKind::Csv);
        assert_eq!(table_source_kind_for_path("a.TSV"), TableSourceKind::Tsv);
        assert_eq!(table_source_kind_for_path("a.tab"), TableSourceKind::Tsv);
        assert_eq!(table_source_kind_for_path("a.xlsx"), TableSourceKind::Xlsx);
        assert_eq!(
            table_source_kind_for_path("a.txt"),
            TableSourceKind::Unknown
        );
        assert_eq!(table_source_kind_for_path(""), TableSourceKind::Unknown);
        assert_eq!(table_delimiter_for_source(TableSourceKind::Tsv), '\t');
        assert_eq!(table_delimiter_for_source(TableSourceKind::Csv), ',');
        assert_eq!(table_delimiter_for_source(TableSourceKind::Xlsx), ',');
        assert_eq!(table_source_badge_label(TableSourceKind::Xlsx), "xlsx");
        // Sheet clamp/label: OOB par None (kabhi panic nahi).
        let names = vec!["S1".to_string(), "S2".to_string()];
        assert_eq!(table_sheet_count(&names), 2);
        assert_eq!(table_sheet_clamp(&names, 1), Some(1));
        assert_eq!(table_sheet_clamp(&names, 9), None);
        assert_eq!(table_sheet_clamp(&[], 0), None);
        assert_eq!(table_sheet_label(&names, 0), Some("S1".to_string()));
        assert_eq!(table_sheet_label(&names, 9), None);
        // Geometry parity: sheet bar key-bar rhythm reuse karta hai.
        assert_eq!(TABLE_SHEET_BAR_H, TABLE_KEY_BAR_H);
        assert_eq!(TABLE_SHEET_COL_GAP, TABLE_KEY_COL_GAP);
        assert_eq!(TABLE_SHEET_BADGE_PAD, TABLE_KEY_BADGE_PAD);
        assert_eq!(table_sheet_label_text_role(), TypoRole::Body);
        assert_eq!(table_sheet_value_text_role(), TypoRole::Mono);
    }
}
