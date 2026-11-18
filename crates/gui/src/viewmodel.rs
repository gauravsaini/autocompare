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

use autocompare_compare::{FolderComparison, FolderStatus, TextComparison};
use autocompare_diff::{Op, PictureDiff, RgbaImage, inline_diff};

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
}
