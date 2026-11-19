//! Hierarchical folder tree drill-down logic.
//!
//! Provides pure filtering and indentation mapping for folder rows,
//! plus headless display-filter, compare-intent staging and sync
//! preview state. Engine reuse only: glyphs come from
//! `viewmodel::build_folder_rows` (compare crate results), sync
//! preview mirrors `autocompare-compare` plan rules over those glyphs
//! without touching the filesystem.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::BTreeSet;

use autocompare_compare::{SyncAction, SyncDirection, SyncOptions, SyncStep};

use crate::viewmodel::FolderRow;

/// One node in the hierarchical folder tree view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderTreeNode {
    /// Underlying folder entry row.
    pub row: FolderRow,
    /// Tree depth (nesting level, starting at 0 for root entries).
    pub depth: usize,
    /// Whether this directory is currently expanded.
    pub is_expanded: bool,
    /// Whether this directory contains any sub-items.
    pub has_children: bool,
}

/// Builds a filtered, indented hierarchical view of folder rows based on expanded directories.
///
/// An entry is visible if all its ancestor directories are contained in `expanded_dirs`.
/// Root entries (depth 0) are always visible.
pub fn folder_tree(rows: &[FolderRow], expanded_dirs: &BTreeSet<String>) -> Vec<FolderTreeNode> {
    let mut dirs_with_children = BTreeSet::new();
    for r in rows {
        if let Some(pos) = r.rel_path.rfind('/')
            && let Some(parent) = r.rel_path.get(..pos)
        {
            dirs_with_children.insert(parent.to_string());
        }
    }

    let mut nodes = Vec::new();
    for row in rows {
        let depth = row.rel_path.chars().filter(|&c| c == '/').count();
        let mut visible = true;

        if depth > 0 {
            let mut current = row.rel_path.as_str();
            while let Some(pos) = current.rfind('/') {
                let Some(parent) = current.get(..pos) else {
                    break;
                };
                if !expanded_dirs.contains(parent) {
                    visible = false;
                    break;
                }
                current = parent;
            }
        }

        if visible {
            let is_expanded = row.is_dir && expanded_dirs.contains(&row.rel_path);
            let has_children = row.is_dir && dirs_with_children.contains(&row.rel_path);
            nodes.push(FolderTreeNode {
                row: row.clone(),
                depth,
                is_expanded,
                has_children,
            });
        }
    }

    nodes
}

/// Canonical indent step for one tree depth level: one MD spacing
/// step from `crate::theme` (12.0 on the 4/8/12/16/24 scale), so
/// gutter / markers / separator stay aligned. Pure; paint in
/// `views.rs` keeps its legacy 14.0 until the owning agent adopts
/// this token (disjoint ownership: this file must not touch paint).
pub fn tree_indent_step() -> f32 {
    crate::theme::GEO_TREE_INDENT_STEP
}

/// Indent in px for a tree depth: `depth * step`, saturating on overflow.
pub fn tree_indent_for_depth(depth: usize) -> f32 {
    (depth as f32) * tree_indent_step()
}

/// Toggles one directory in the expanded set.
pub fn toggle_expanded(expanded_dirs: &mut BTreeSet<String>, dir: &str) {
    if expanded_dirs.contains(dir) {
        expanded_dirs.remove(dir);
    } else {
        expanded_dirs.insert(dir.to_string());
    }
}

/// Expands every directory row.
pub fn expand_all(expanded_dirs: &mut BTreeSet<String>, rows: &[FolderRow]) {
    for row in rows {
        if row.is_dir {
            expanded_dirs.insert(row.rel_path.clone());
        }
    }
}

/// Collapses the whole tree.
pub fn collapse_all(expanded_dirs: &mut BTreeSet<String>) {
    expanded_dirs.clear();
}

/// Display filters for the folder view. All fields are independent
/// checkboxes; counts stay honest because filtering never rewrites
/// glyphs, it only hides rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FolderDisplay {
    /// Hide orphan rows (`L` / `R` glyphs).
    pub hide_orphans: bool,
    /// Show only left-newer rows (`>` glyph).
    pub left_newer_only: bool,
    /// Show only right-newer rows (`<` glyph).
    pub right_newer_only: bool,
    /// Ignore hierarchy and show a flat list.
    pub flatten: bool,
}

/// Whether one row passes the active display filter.
pub fn filter_pass(row: &FolderRow, display: &FolderDisplay) -> bool {
    if display.hide_orphans && (row.glyph == 'L' || row.glyph == 'R') {
        return false;
    }
    if display.left_newer_only && row.glyph != '>' {
        return false;
    }
    if display.right_newer_only && row.glyph != '<' {
        return false;
    }
    true
}

/// Honest per-glyph counts over the full row set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FolderCounts {
    pub total: usize,
    pub same: usize,
    pub different: usize,
    pub newer_left: usize,
    pub newer_right: usize,
    pub left_only: usize,
    pub right_only: usize,
}

impl FolderCounts {
    /// Orphan rows (`L` + `R`).
    pub const fn orphans(self) -> usize {
        self.left_only.saturating_add(self.right_only)
    }

    /// One-line honest summary, same shape as the CLI counts.
    pub fn summary(self) -> String {
        format!(
            "{} entries: {} same, {} different, {} newer left, {} newer right, {} left-only, {} right-only",
            self.total,
            self.same,
            self.different,
            self.newer_left,
            self.newer_right,
            self.left_only,
            self.right_only
        )
    }
}

/// Counts every row once, by glyph. Unknown glyphs count toward
/// `total` only, so the summary never hides rows.
pub fn count_folder_rows(rows: &[FolderRow]) -> FolderCounts {
    let mut counts = FolderCounts {
        total: rows.len(),
        ..FolderCounts::default()
    };
    for row in rows {
        match row.glyph {
            '=' => counts.same = counts.same.saturating_add(1),
            '!' => counts.different = counts.different.saturating_add(1),
            '>' => counts.newer_left = counts.newer_left.saturating_add(1),
            '<' => counts.newer_right = counts.newer_right.saturating_add(1),
            'L' => counts.left_only = counts.left_only.saturating_add(1),
            'R' => counts.right_only = counts.right_only.saturating_add(1),
            _ => {}
        }
    }
    counts
}

/// Filtered tree view. Flatten ignores `expanded_dirs` and shows
/// matching rows as a flat list (depth 0); otherwise rows are
/// pre-filtered and then laid out with [`folder_tree`], so ancestor
/// expansion rules stay the same.
pub fn folder_tree_filtered(
    rows: &[FolderRow],
    expanded_dirs: &BTreeSet<String>,
    display: &FolderDisplay,
) -> Vec<FolderTreeNode> {
    if display.flatten {
        let mut nodes = Vec::new();
        for row in rows {
            if filter_pass(row, display) {
                nodes.push(FolderTreeNode {
                    row: row.clone(),
                    depth: 0,
                    is_expanded: false,
                    has_children: false,
                });
            }
        }
        return nodes;
    }
    let kept: Vec<FolderRow> = rows
        .iter()
        .filter(|row| filter_pass(row, display))
        .cloned()
        .collect();
    folder_tree(&kept, expanded_dirs)
}

/// Stages a double-clicked file node as compare intent. Directories
/// never stage; the previous staging stays in that case.
pub fn stage_compare_intent(staged: &mut Option<String>, node: &FolderTreeNode) {
    if node.row.is_dir {
        return;
    }
    *staged = Some(node.row.rel_path.clone());
}

/// Clears a staged compare intent.
pub fn clear_compare_intent(staged: &mut Option<String>) {
    *staged = None;
}

/// Sync preview mode for the folder view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SyncModeChoice {
    /// Newer side wins both ways, no deletes.
    #[default]
    UpdateBoth,
    /// Mirror left to right, prune right-only entries.
    Mirror,
}

impl SyncModeChoice {
    /// Label for the mode select control.
    pub const fn label(self) -> &'static str {
        match self {
            Self::UpdateBoth => "Update Both",
            Self::Mirror => "Mirror",
        }
    }
}

/// Maps the preview mode to engine sync options (same mapping as
/// `SyncPlanner`: Both = NewerWins without prune, Mirror =
/// LeftToRight with prune).
pub fn sync_options_for(mode: SyncModeChoice) -> SyncOptions {
    match mode {
        SyncModeChoice::UpdateBoth => SyncOptions {
            direction: SyncDirection::NewerWins,
            prune: false,
        },
        SyncModeChoice::Mirror => SyncOptions {
            direction: SyncDirection::LeftToRight,
            prune: true,
        },
    }
}

/// Dry-run sync preview over already-compared rows. Mirrors the
/// engine `plan_sync` file/dir rules by glyph, without filesystem
/// access: `=` does nothing, `!` is skipped, newer/only sides copy
/// toward the losing side, Mirror prunes right-only entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncPreview {
    /// Actions that Apply would run, parents before children.
    pub steps: Vec<SyncStep>,
    /// Entries left alone: (relative path, reason).
    pub skipped: Vec<(String, String)>,
}

impl SyncPreview {
    /// Whether Apply has anything to do.
    pub const fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.skipped.is_empty()
    }

    /// Honest one-line summary.
    pub fn summary(&self) -> String {
        format!(
            "{} step(s) planned, {} skipped",
            self.steps.len(),
            self.skipped.len()
        )
    }
}

/// Whether `rel` names something strictly inside `dir_rel`.
fn is_under(rel: &str, dir_rel: &str) -> bool {
    let mut prefix = dir_rel.to_string();
    prefix.push('/');
    rel.starts_with(&prefix)
}

/// Builds the sync preview for one mode over folder rows.
pub fn sync_preview(rows: &[FolderRow], mode: SyncModeChoice) -> SyncPreview {
    let mut preview = SyncPreview::default();
    let mut doomed: Vec<String> = Vec::new();
    for row in rows {
        let mut under_doomed = false;
        for dir in &doomed {
            if is_under(&row.rel_path, dir) {
                under_doomed = true;
                break;
            }
        }
        if under_doomed {
            continue;
        }
        if row.is_dir {
            if mode == SyncModeChoice::Mirror && row.glyph == 'R' {
                preview.steps.push(SyncStep {
                    rel_path: row.rel_path.clone(),
                    is_dir: true,
                    action: SyncAction::DeleteRight,
                });
                doomed.push(row.rel_path.clone());
            }
            continue;
        }
        match row.glyph {
            '=' => {}
            '!' => {
                preview.skipped.push((
                    row.rel_path.clone(),
                    "content differs; no side wins".to_string(),
                ));
            }
            '>' => {
                preview.steps.push(SyncStep {
                    rel_path: row.rel_path.clone(),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                });
            }
            '<' => {
                if mode == SyncModeChoice::UpdateBoth {
                    preview.steps.push(SyncStep {
                        rel_path: row.rel_path.clone(),
                        is_dir: false,
                        action: SyncAction::CopyToLeft,
                    });
                } else {
                    preview
                        .skipped
                        .push((row.rel_path.clone(), "destination is newer".to_string()));
                }
            }
            'L' => {
                preview.steps.push(SyncStep {
                    rel_path: row.rel_path.clone(),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                });
            }
            'R' => {
                if mode == SyncModeChoice::UpdateBoth {
                    preview.steps.push(SyncStep {
                        rel_path: row.rel_path.clone(),
                        is_dir: false,
                        action: SyncAction::CopyToLeft,
                    });
                } else {
                    preview.steps.push(SyncStep {
                        rel_path: row.rel_path.clone(),
                        is_dir: false,
                        action: SyncAction::DeleteRight,
                    });
                }
            }
            _ => {
                preview.skipped.push((
                    row.rel_path.clone(),
                    "unknown status; left alone".to_string(),
                ));
            }
        }
    }
    preview
}

/// Explicit Apply consent gate: Apply runs only with consent and a
/// non-empty step list. The preview itself never mutates anything.
pub fn can_apply(preview: &SyncPreview, consented: bool) -> bool {
    consented && !preview.steps.is_empty()
}

// ----------------------------------------------------------------------------
// Folder columns (headless; wiring owned by views.rs / agent A)
// ----------------------------------------------------------------------------
//
// Gap 6: the folder view paints names only; size/time need side-by-side
// columns. This section is the pure column spec: header labels, fixed
// char widths, and cell text per row. Paint (agent A) lays these out as
// columns; indent keeps the frozen [`tree_indent_step`] (12.0, one
// MD spacing step) and colors stay frozen — this file changes no hues.
// WIRING: for each visible `FolderTreeNode`, paint one row with
// `folder_cell_text(&node.row, col)` per `folder_column_defs()` entry;
// the Path cell is offset by `tree_indent_for_depth(node.depth)`.

/// One folder column: header plus fixed width in characters
/// (`0` means flex: the path takes the remaining width).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderColumnDef {
    /// Which cell this column shows.
    pub column: FolderColumn,
    /// Header label paint shows above the column.
    pub header: &'static str,
    /// Fixed width in characters; `0` flexes.
    pub width_chars: usize,
}

/// Folder view columns: path plus per-side size and modified time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderColumn {
    /// Relative path (flex width, indented by tree depth).
    Path,
    /// Left size bytes, `None` renders as `-`.
    LeftSize,
    /// Right size bytes, `None` renders as `-`.
    RightSize,
    /// Left modified time, `None` renders as `-`.
    LeftTime,
    /// Right modified time, `None` renders as `-`.
    RightTime,
}

/// Fixed width of each size column: largest byte counts stay readable.
pub const FOLDER_SIZE_CHARS: usize = 10;
/// Fixed width of each time column: fits `format_time` output exactly
/// (`"YYYY-MM-DD HH:MM"` is 16 characters).
pub const FOLDER_TIME_CHARS: usize = 16;

/// Column spec in paint order: flex path, sizes, times.
pub fn folder_column_defs() -> [FolderColumnDef; 5] {
    [
        FolderColumnDef {
            column: FolderColumn::Path,
            header: "Path",
            width_chars: 0,
        },
        FolderColumnDef {
            column: FolderColumn::LeftSize,
            header: "L size",
            width_chars: FOLDER_SIZE_CHARS,
        },
        FolderColumnDef {
            column: FolderColumn::RightSize,
            header: "R size",
            width_chars: FOLDER_SIZE_CHARS,
        },
        FolderColumnDef {
            column: FolderColumn::LeftTime,
            header: "L modified",
            width_chars: FOLDER_TIME_CHARS,
        },
        FolderColumnDef {
            column: FolderColumn::RightTime,
            header: "R modified",
            width_chars: FOLDER_TIME_CHARS,
        },
    ]
}

/// Cell text for one folder row and column. Missing sides render as
/// `"-"` so columns stay aligned for orphan rows. Pure; unit-tested.
pub fn folder_cell_text(row: &FolderRow, column: FolderColumn) -> String {
    match column {
        FolderColumn::Path => row.rel_path.clone(),
        FolderColumn::LeftSize => row.left_size.clone().unwrap_or_else(|| "-".to_string()),
        FolderColumn::RightSize => row.right_size.clone().unwrap_or_else(|| "-".to_string()),
        FolderColumn::LeftTime => row.left_time.clone().unwrap_or_else(|| "-".to_string()),
        FolderColumn::RightTime => row.right_time.clone().unwrap_or_else(|| "-".to_string()),
    }
}

/// Pads `text` to exactly `width` chars with spaces, cutting longer
/// text so side-by-side columns kabhi misalign na hon. Char-based hai
/// taaki multibyte paths par bhi width stable rahe.
pub fn pad_folder_cell(text: &str, width: usize) -> String {
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

/// Header line for the folder column grid: `Path | L size | ...`.
/// Path flex hai (no pad), baaki fixed widths par padded hain.
pub fn folder_header_line() -> String {
    let mut parts: Vec<String> = Vec::new();
    for def in folder_column_defs() {
        if def.width_chars == 0 {
            parts.push(def.header.to_string());
        } else {
            parts.push(pad_folder_cell(def.header, def.width_chars));
        }
    }
    parts.join(" | ")
}

/// One folder row as an aligned column line. Glyph prefix ke saath
/// path, phir fixed side columns. Missing sides `-` dikhte hain
/// (via [`folder_cell_text`]), isliye orphan rows bhi align rahte hain.
pub fn folder_format_row(row: &FolderRow) -> String {
    let mut parts: Vec<String> = Vec::new();
    for def in folder_column_defs() {
        let text = folder_cell_text(row, def.column);
        if def.width_chars == 0 {
            parts.push(format!("{} {}", row.glyph, text));
        } else {
            parts.push(pad_folder_cell(&text, def.width_chars));
        }
    }
    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(path: &str, is_dir: bool) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        }
    }

    fn make_glyph_row(path: &str, is_dir: bool, glyph: char) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph,
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: glyph != '=',
        }
    }

    #[test]
    fn collapsed_tree_shows_only_root_entries() {
        let rows = vec![
            make_row("src", true),
            make_row("src/lib.rs", false),
            make_row("README.md", false),
        ];
        let expanded = BTreeSet::new();
        let tree = folder_tree(&rows, &expanded);
        assert_eq!(tree.len(), 2);
        let Some(first) = tree.first() else {
            return;
        };
        assert_eq!(first.row.rel_path, "src");
        assert_eq!(first.depth, 0);
        assert!(first.has_children);
        assert!(!first.is_expanded);

        let Some(second) = tree.get(1) else {
            return;
        };
        assert_eq!(second.row.rel_path, "README.md");
        assert_eq!(second.depth, 0);
    }

    #[test]
    fn expanded_directory_reveals_children() {
        let rows = vec![
            make_row("src", true),
            make_row("src/lib.rs", false),
            make_row("src/sub", true),
            make_row("src/sub/mod.rs", false),
        ];
        let mut expanded = BTreeSet::new();
        expanded.insert("src".to_string());

        let tree = folder_tree(&rows, &expanded);
        assert_eq!(tree.len(), 3);
        let Some(sub) = tree.get(2) else {
            return;
        };
        assert_eq!(sub.row.rel_path, "src/sub");
        assert_eq!(sub.depth, 1);
        assert!(sub.has_children);
        assert!(!sub.is_expanded);

        // Now expand nested directory
        expanded.insert("src/sub".to_string());
        let tree_full = folder_tree(&rows, &expanded);
        assert_eq!(tree_full.len(), 4);
        let Some(last) = tree_full.last() else {
            return;
        };
        assert_eq!(last.row.rel_path, "src/sub/mod.rs");
        assert_eq!(last.depth, 2);
    }

    #[test]
    fn toggle_expand_and_collapse_helpers() {
        let mut expanded = BTreeSet::new();
        toggle_expanded(&mut expanded, "src");
        assert!(expanded.contains("src"));
        toggle_expanded(&mut expanded, "src");
        assert!(!expanded.contains("src"));

        let rows = vec![make_row("a", true), make_row("b", true)];
        expand_all(&mut expanded, &rows);
        assert!(expanded.contains("a"));
        assert!(expanded.contains("b"));
        collapse_all(&mut expanded);
        assert!(expanded.is_empty());
    }

    #[test]
    fn indent_step_matches_spacing_scale() {
        use crate::theme::{GEO_TREE_INDENT_STEP, SPACE_MD};
        assert_eq!(tree_indent_step(), GEO_TREE_INDENT_STEP);
        assert_eq!(tree_indent_step(), SPACE_MD);
        assert_eq!(tree_indent_step(), 12.0);
        assert_eq!(tree_indent_for_depth(0), 0.0);
        assert_eq!(tree_indent_for_depth(1), 12.0);
        assert_eq!(tree_indent_for_depth(2), 24.0);
    }

    #[test]
    fn hide_orphans_drops_left_and_right_only() {
        let rows = vec![
            make_glyph_row("same.txt", false, '='),
            make_glyph_row("left.txt", false, 'L'),
            make_glyph_row("right.txt", false, 'R'),
            make_glyph_row("newer.txt", false, '>'),
        ];
        let display = FolderDisplay {
            hide_orphans: true,
            ..FolderDisplay::default()
        };
        let tree = folder_tree_filtered(&rows, &BTreeSet::new(), &display);
        assert_eq!(tree.len(), 2);
        assert!(
            tree.iter()
                .all(|n| n.row.glyph != 'L' && n.row.glyph != 'R')
        );
    }

    #[test]
    fn left_newer_only_keeps_only_greater_glyph() {
        let rows = vec![
            make_glyph_row("a.txt", false, '>'),
            make_glyph_row("b.txt", false, '<'),
            make_glyph_row("c.txt", false, '='),
        ];
        let display = FolderDisplay {
            left_newer_only: true,
            ..FolderDisplay::default()
        };
        let tree = folder_tree_filtered(&rows, &BTreeSet::new(), &display);
        assert_eq!(tree.len(), 1);
        let Some(node) = tree.first() else {
            return;
        };
        assert_eq!(node.row.glyph, '>');
    }

    #[test]
    fn right_newer_only_keeps_only_less_glyph() {
        let rows = vec![
            make_glyph_row("a.txt", false, '>'),
            make_glyph_row("b.txt", false, '<'),
        ];
        let display = FolderDisplay {
            right_newer_only: true,
            ..FolderDisplay::default()
        };
        let tree = folder_tree_filtered(&rows, &BTreeSet::new(), &display);
        assert_eq!(tree.len(), 1);
        let Some(node) = tree.first() else {
            return;
        };
        assert_eq!(node.row.glyph, '<');
    }

    #[test]
    fn both_newer_filters_together_show_nothing_honestly() {
        let rows = vec![
            make_glyph_row("a.txt", false, '>'),
            make_glyph_row("b.txt", false, '<'),
        ];
        let display = FolderDisplay {
            left_newer_only: true,
            right_newer_only: true,
            ..FolderDisplay::default()
        };
        let tree = folder_tree_filtered(&rows, &BTreeSet::new(), &display);
        assert!(tree.is_empty());
    }

    #[test]
    fn flatten_ignores_hierarchy_and_expansion() {
        let rows = vec![
            make_row("src", true),
            make_row("src/lib.rs", false),
            make_row("README.md", false),
        ];
        let display = FolderDisplay {
            flatten: true,
            ..FolderDisplay::default()
        };
        let tree = folder_tree_filtered(&rows, &BTreeSet::new(), &display);
        assert_eq!(tree.len(), 3);
        assert!(tree.iter().all(|n| n.depth == 0));
    }

    #[test]
    fn counts_cover_every_glyph_honestly() {
        let rows = vec![
            make_glyph_row("same.txt", false, '='),
            make_glyph_row("diff.txt", false, '!'),
            make_glyph_row("nl.txt", false, '>'),
            make_glyph_row("nr.txt", false, '<'),
            make_glyph_row("lo.txt", false, 'L'),
            make_glyph_row("ro.txt", false, 'R'),
        ];
        let counts = count_folder_rows(&rows);
        assert_eq!(counts.total, 6);
        assert_eq!(counts.same, 1);
        assert_eq!(counts.different, 1);
        assert_eq!(counts.newer_left, 1);
        assert_eq!(counts.newer_right, 1);
        assert_eq!(counts.left_only, 1);
        assert_eq!(counts.right_only, 1);
        assert_eq!(counts.orphans(), 2);
        assert!(counts.summary().contains("6 entries"));
        assert!(counts.summary().contains("1 left-only"));
    }

    #[test]
    fn double_click_file_stages_intent_dir_keeps_old() {
        let file_node = FolderTreeNode {
            row: make_glyph_row("a/b.txt", false, '!'),
            depth: 1,
            is_expanded: false,
            has_children: false,
        };
        let dir_node = FolderTreeNode {
            row: make_glyph_row("a", true, '='),
            depth: 0,
            is_expanded: true,
            has_children: true,
        };
        let mut staged: Option<String> = None;
        stage_compare_intent(&mut staged, &dir_node);
        assert_eq!(staged, None);
        stage_compare_intent(&mut staged, &file_node);
        assert_eq!(staged, Some("a/b.txt".to_string()));
        stage_compare_intent(&mut staged, &dir_node);
        assert_eq!(staged, Some("a/b.txt".to_string()));
        clear_compare_intent(&mut staged);
        assert_eq!(staged, None);
    }

    #[test]
    fn sync_mode_labels_and_options_match_engine() {
        assert_eq!(SyncModeChoice::UpdateBoth.label(), "Update Both");
        assert_eq!(SyncModeChoice::Mirror.label(), "Mirror");
        let both = sync_options_for(SyncModeChoice::UpdateBoth);
        assert_eq!(both.direction, SyncDirection::NewerWins);
        assert!(!both.prune);
        let mirror = sync_options_for(SyncModeChoice::Mirror);
        assert_eq!(mirror.direction, SyncDirection::LeftToRight);
        assert!(mirror.prune);
    }

    #[test]
    fn sync_preview_update_both_copies_newer_and_orphans() {
        let rows = vec![
            make_glyph_row("same.txt", false, '='),
            make_glyph_row("diff.txt", false, '!'),
            make_glyph_row("nl.txt", false, '>'),
            make_glyph_row("nr.txt", false, '<'),
            make_glyph_row("lo.txt", false, 'L'),
            make_glyph_row("ro.txt", false, 'R'),
        ];
        let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
        assert_eq!(preview.steps.len(), 4);
        assert_eq!(preview.skipped.len(), 1);
        assert!(preview.summary().contains("4 step(s)"));
        assert!(can_apply(&preview, true));
        assert!(!can_apply(&preview, false));
    }

    #[test]
    fn sync_preview_mirror_prunes_right_only() {
        let rows = vec![
            make_glyph_row("nl.txt", false, '>'),
            make_glyph_row("nr.txt", false, '<'),
            make_glyph_row("ro.txt", false, 'R'),
        ];
        let preview = sync_preview(&rows, SyncModeChoice::Mirror);
        let mut saw_copy_right = false;
        let mut saw_delete_right = false;
        let mut saw_skip_newer = false;
        for step in &preview.steps {
            if step.action == SyncAction::CopyToRight {
                saw_copy_right = true;
            }
            if step.action == SyncAction::DeleteRight {
                saw_delete_right = true;
            }
        }
        for skipped in &preview.skipped {
            if skipped.0 == "nr.txt" {
                saw_skip_newer = true;
            }
        }
        assert!(saw_copy_right);
        assert!(saw_delete_right);
        assert!(saw_skip_newer);
        assert!(!can_apply(&preview, false));
    }

    #[test]
    fn sync_preview_empty_never_applies() {
        let rows = vec![make_glyph_row("same.txt", false, '=')];
        let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
        assert!(preview.is_empty());
        assert!(!can_apply(&preview, true));
    }

    #[test]
    fn sync_preview_pruned_dir_hides_children() {
        let rows = vec![
            make_glyph_row("gone", true, 'R'),
            make_glyph_row("gone/old.txt", false, 'R'),
        ];
        let preview = sync_preview(&rows, SyncModeChoice::Mirror);
        assert_eq!(preview.steps.len(), 1);
        let Some(step) = preview.steps.first() else {
            return;
        };
        assert_eq!(step.rel_path, "gone");
        assert!(step.is_dir);
    }

    #[test]
    fn column_defs_keep_path_flex_and_fixed_side_widths() {
        let defs = folder_column_defs();
        assert_eq!(defs.len(), 5);
        let Some(path) = defs.first() else {
            return;
        };
        assert_eq!(path.column, FolderColumn::Path);
        assert_eq!(path.header, "Path");
        assert_eq!(path.width_chars, 0);
        for def in defs.iter().skip(1) {
            assert!(def.width_chars > 0);
        }
        assert_eq!(FOLDER_SIZE_CHARS, 10);
        assert_eq!(FOLDER_TIME_CHARS, 16);
        // Time columns fit the formatted timestamp exactly.
        let stamp = crate::viewmodel::format_time(std::time::UNIX_EPOCH);
        assert_eq!(stamp.chars().count(), FOLDER_TIME_CHARS);
    }

    #[test]
    fn indent_token_stays_on_spacing_scale_with_columns() {
        use crate::theme::{GEO_TREE_INDENT_STEP, SPACE_MD};
        assert_eq!(tree_indent_step(), GEO_TREE_INDENT_STEP);
        assert_eq!(tree_indent_step(), SPACE_MD);
        assert_eq!(tree_indent_step(), 12.0);
        assert_eq!(tree_indent_for_depth(2), 24.0);
        assert_eq!(folder_column_defs().len(), 5);
    }

    #[test]
    fn cell_text_renders_missing_sides_as_dash() {
        let row = make_glyph_row("only-left.txt", false, 'L');
        assert_eq!(folder_cell_text(&row, FolderColumn::Path), "only-left.txt");
        // Left-only fixture rows carry no sizes/times: dashes keep columns aligned.
        assert_eq!(folder_cell_text(&row, FolderColumn::LeftSize), "-");
        assert_eq!(folder_cell_text(&row, FolderColumn::RightTime), "-");
        let mut both = make_glyph_row("both.txt", false, '=');
        both.left_size = Some("12".to_string());
        both.right_time = Some("2026-01-02 03:04".to_string());
        assert_eq!(folder_cell_text(&both, FolderColumn::Path), "both.txt");
        assert_eq!(folder_cell_text(&both, FolderColumn::LeftSize), "12");
        assert_eq!(
            folder_cell_text(&both, FolderColumn::RightTime),
            "2026-01-02 03:04"
        );
        assert_eq!(folder_cell_text(&both, FolderColumn::RightSize), "-");
    }

    #[test]
    fn folder_find_matches_paths_with_wrap_and_case() {
        let rows = vec![
            make_glyph_row("src/lib.rs", false, '='),
            make_glyph_row("README.md", false, '='),
            make_glyph_row("Src/main.rs", false, '!'),
        ];
        assert_eq!(folder_find_next(&rows, "src", 0, false), Some(0));
        // Case-sensitive: "Src" alag hai.
        assert_eq!(folder_find_next(&rows, "SRC", 0, false), None);
        assert_eq!(folder_find_next(&rows, "SRC", 0, true), Some(0));
        // Wrap forward: aakhri ke baad pehli.
        assert_eq!(folder_find_next(&rows, "src", 1, true), Some(2));
        // Prev: peeche scan.
        assert_eq!(folder_find_prev(&rows, "src", 2, true), Some(0));
        assert_eq!(folder_find_prev(&rows, "src", 0, false), Some(0));
        // Khali/gayab: None.
        assert_eq!(folder_find_next(&rows, "", 0, false), None);
        assert_eq!(folder_find_next(&[], "x", 0, false), None);
        assert_eq!(folder_find_prev(&rows, "", 1, false), None);
        assert_eq!(folder_find_next(&rows, "zzz", 0, false), None);
    }

    #[test]
    fn folder_merge_resolves_one_row_action() {
        // Changed/newer/orphan rows: side ke hisab se copy direction.
        assert_eq!(
            folder_resolve_row('>', false, FolderMergeSide::Left),
            Some(SyncAction::CopyToRight)
        );
        assert_eq!(
            folder_resolve_row('<', false, FolderMergeSide::Right),
            Some(SyncAction::CopyToLeft)
        );
        assert_eq!(
            folder_resolve_row('L', false, FolderMergeSide::Left),
            Some(SyncAction::CopyToRight)
        );
        assert_eq!(
            folder_resolve_row('R', false, FolderMergeSide::Right),
            Some(SyncAction::CopyToLeft)
        );
        // Same/diff/manual aur dirs: koi one-click action nahi.
        assert_eq!(folder_resolve_row('=', false, FolderMergeSide::Left), None);
        assert_eq!(folder_resolve_row('!', false, FolderMergeSide::Left), None);
        assert_eq!(folder_resolve_row('>', true, FolderMergeSide::Left), None);
        assert_eq!(folder_resolve_row('R', true, FolderMergeSide::Right), None);
    }

    #[test]
    fn row_pager_pages_and_brings_nodes_into_view() {
        let mut pager = RowPager::new(25, 10);
        assert_eq!(pager.page_count(), 3);
        assert_eq!(pager.visible_range(), (0, 10));
        pager.next_page();
        assert_eq!(pager.top, 10);
        pager.next_page();
        assert_eq!(pager.top, 20);
        assert_eq!(pager.visible_range(), (20, 25));
        pager.next_page();
        assert_eq!(pager.top, 20);
        pager.prev_page();
        assert_eq!(pager.top, 10);
        assert!(!pager.bring_into_view(12));
        assert!(pager.bring_into_view(24));
        assert_eq!(pager.top, 15);
        let mut empty = RowPager::new(0, 10);
        assert_eq!(empty.page_count(), 0);
        assert!(!empty.bring_into_view(0));
    }

    #[test]
    fn folder_gutter_jump_wraps_over_changed_rows() {
        let rows = vec![
            make_glyph_row("same.txt", false, '='),
            make_glyph_row("diff.txt", false, '!'),
            make_glyph_row("same2.txt", false, '='),
            make_glyph_row("newer.txt", false, '>'),
        ];
        assert_eq!(folder_next_changed(&rows, 0), Some(1));
        assert_eq!(folder_next_changed(&rows, 1), Some(3));
        // Wrap: aakhri changed ke baad pehla changed.
        assert_eq!(folder_next_changed(&rows, 3), Some(1));
        assert_eq!(folder_prev_changed(&rows, 3), Some(1));
        assert_eq!(folder_prev_changed(&rows, 1), Some(3));
        assert_eq!(folder_prev_changed(&rows, 0), Some(3));
        // Gutter click: changed par wahi, unchanged par agla changed.
        assert_eq!(folder_gutter_target(&rows, 1), Some(1));
        assert_eq!(folder_gutter_target(&rows, 0), Some(1));
        assert_eq!(folder_gutter_target(&rows, 2), Some(3));
        assert_eq!(folder_gutter_target(&rows, 99), None);
        // Sab same: koi target nahi.
        let clean = vec![
            make_glyph_row("a.txt", false, '='),
            make_glyph_row("b.txt", false, '='),
        ];
        assert_eq!(folder_next_changed(&clean, 0), None);
        assert_eq!(folder_prev_changed(&clean, 1), None);
        assert_eq!(folder_gutter_target(&clean, 0), None);
        assert_eq!(folder_next_changed(&[], 0), None);
        assert_eq!(folder_prev_changed(&[], 0), None);
    }

    #[test]
    fn folder_lines_keep_side_columns_aligned() {
        let header = folder_header_line();
        assert!(header.contains("Path"));
        assert!(header.contains("L size"));
        assert!(header.contains("R size"));
        assert!(header.contains("L modified"));
        // Fixed parts padded hain: separator count defs-1 hai.
        assert_eq!(header.split(" | ").count(), 5);
        let orphan = make_glyph_row("only-left.txt", false, 'L');
        let line = folder_format_row(&orphan);
        assert!(line.starts_with("L only-left.txt"));
        assert_eq!(line.split(" | ").count(), 5);
        // Dono sides wali row me values dikhti hain.
        let mut both = make_glyph_row("both.txt", false, '=');
        both.left_size = Some("12".to_string());
        both.right_size = Some("34".to_string());
        both.left_time = Some("2026-01-02 03:04".to_string());
        both.right_time = Some("2026-01-03 04:05".to_string());
        let full = folder_format_row(&both);
        assert!(full.contains("both.txt"));
        assert!(full.contains("12"));
        assert!(full.contains("34"));
        // Pad helper lamba text kaat-ta hai, chhota pad karta hai.
        assert_eq!(pad_folder_cell("ab", 4), "ab  ");
        assert_eq!(pad_folder_cell("abcdef", 4), "abcd");
    }
}

// ----------------------------------------------------------------------------
// Gutter-click jump (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #3/#4: folder gutter me change markers par click ya
// next/prev control seedha agle changed row par jump kare. Pure index
// math hai: `changed` flag par next/prev scan, wrapping ke saath.
// No egui, no I/O.
// ----------------------------------------------------------------------------

/// Agle changed row ka index `from` se aage (exclusive), wrap ke saath.
/// Koi changed row nahi to `None`. Gutter next-change isi ko use karta hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn folder_next_changed(rows: &[FolderRow], from: usize) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let total = rows.len();
    let mut idx = from.saturating_add(1) % total.max(1);
    for _ in 0..total {
        let row = rows.get(idx)?;
        if row.changed {
            return Some(idx);
        }
        idx = idx.saturating_add(1) % total.max(1);
    }
    None
}

/// Pichhle changed row ka index `from` se peeche (exclusive), wrap ke saath.
/// Koi changed row nahi to `None`. Gutter prev-change isi ko use karta hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn folder_prev_changed(rows: &[FolderRow], from: usize) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let total = rows.len();
    let mut idx = if from == 0 {
        total.saturating_sub(1)
    } else {
        (from.saturating_sub(1)) % total.max(1)
    };
    for _ in 0..total {
        let row = rows.get(idx)?;
        if row.changed {
            return Some(idx);
        }
        idx = if idx == 0 {
            total.saturating_sub(1)
        } else {
            idx.saturating_sub(1)
        };
    }
    None
}

/// Gutter row-click target: `row_idx` khud changed hai to wahi, warna
/// uske baad ka agla changed (wrap ke saath). Paint click ko index
/// me badalkar ye helper deta hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn folder_gutter_target(rows: &[FolderRow], row_idx: usize) -> Option<usize> {
    let row = rows.get(row_idx)?;
    if row.changed {
        return Some(row_idx);
    }
    folder_next_changed(rows, row_idx)
}

/// Path ka find matcher: khali needle kabhi match nahi karta.
fn folder_name_contains(path: &str, needle: &str, ignore_case: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    if ignore_case {
        path.to_lowercase().contains(&needle.to_lowercase())
    } else {
        path.contains(needle)
    }
}

/// Agla folder row jiske path me needle hai, `from` se (inclusive),
/// wrap ke saath. Khali needle/rows par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn folder_find_next(
    rows: &[FolderRow],
    needle: &str,
    from: usize,
    ignore_case: bool,
) -> Option<usize> {
    if needle.is_empty() || rows.is_empty() {
        return None;
    }
    let total = rows.len();
    let mut idx = from % total.max(1);
    for _ in 0..total {
        let row = rows.get(idx)?;
        if folder_name_contains(&row.rel_path, needle, ignore_case) {
            return Some(idx);
        }
        idx = idx.saturating_add(1) % total.max(1);
    }
    None
}

/// Pichhla folder row jiske path me needle hai, `from` se peeche
/// (exclusive), wrap ke saath. Khali needle/rows par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn folder_find_prev(
    rows: &[FolderRow],
    needle: &str,
    from: usize,
    ignore_case: bool,
) -> Option<usize> {
    if needle.is_empty() || rows.is_empty() {
        return None;
    }
    let total = rows.len();
    let mut idx = if from == 0 {
        total.saturating_sub(1)
    } else {
        (from.saturating_sub(1)) % total.max(1)
    };
    for _ in 0..total {
        let row = rows.get(idx)?;
        if folder_name_contains(&row.rel_path, needle, ignore_case) {
            return Some(idx);
        }
        idx = if idx == 0 {
            total.saturating_sub(1)
        } else {
            idx.saturating_sub(1)
        };
    }
    None
}

// ----------------------------------------------------------------------------
// Per-hunk merge (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #10: folder tab me har row par take-left/take-right.
// `sync_preview` poori list ka plan banata hai; ye helper ek row ka
// faisla expose karta hai taaki paint per-row merge button dikha sake.
// '=' aur '!' kabhi action nahi dete (same / manual resolve), baaki
// UpdateBoth rules follow karte hain. File apply (I/O) OPEN rehta hai.
// ----------------------------------------------------------------------------

/// Per-row merge choice: kaunsi side ka content rakha jaye.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FolderMergeSide {
    /// Left content rakho (right par copy).
    #[default]
    Left,
    /// Right content rakho (left par copy).
    Right,
}

/// Ek folder row ka merge action, `None` jab koi copy na ho.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn folder_resolve_row(glyph: char, is_dir: bool, side: FolderMergeSide) -> Option<SyncAction> {
    if is_dir || glyph == '=' || glyph == '!' {
        return None;
    }
    match side {
        FolderMergeSide::Left => Some(SyncAction::CopyToRight),
        FolderMergeSide::Right => Some(SyncAction::CopyToLeft),
    }
}

// ----------------------------------------------------------------------------
// Paging (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #3: lambe folder tree me page-wise move. Chhota pager
// visible-node top rakhta hai; paint PageUp/PageDown se isko chalata hai.
// Pure math, saturating, kabhi panic nahi. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Page-wise scroll state ek folder tree ke liye.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowPager {
    /// Kul visible nodes.
    pub total_rows: usize,
    /// Ek page me kitne nodes.
    pub rows_per_page: usize,
    /// Visible window ka pehla node.
    pub top: usize,
}

/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
impl RowPager {
    /// Naya pager: page size kam se kam 1, top 0.
    pub fn new(total_rows: usize, rows_per_page: usize) -> Self {
        Self {
            total_rows,
            rows_per_page: rows_per_page.max(1),
            top: 0,
        }
    }

    /// Kul pages (khali tree = 0).
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

// ----------------------------------------------------------------------------
// Folder session state: expand + filter + intent + sync mode in one place
// ----------------------------------------------------------------------------

/// Local headless state of one folder session. Widgets live in the
/// host view; every transition here is unit-testable without a window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderSessionState {
    /// Directories currently expanded in the tree view.
    pub expanded: BTreeSet<String>,
    /// Active display filter.
    pub display: FolderDisplay,
    /// Staged double-click target (engine-relative file path).
    pub staged: Option<String>,
    /// Active sync preview mode.
    pub sync_mode: SyncModeChoice,
    /// Explicit Apply consent for the current preview.
    pub consented: bool,
}

impl FolderSessionState {
    /// Flips one directory in the expanded set.
    pub fn toggle(&mut self, dir: &str) {
        toggle_expanded(&mut self.expanded, dir);
    }

    /// Rows the tree view paints: filtered, then hierarchical or flat.
    pub fn visible_tree(&self, rows: &[FolderRow]) -> Vec<FolderTreeNode> {
        folder_tree_filtered(rows, &self.expanded, &self.display)
    }

    /// Honest counts over all rows.
    pub fn counts(&self, rows: &[FolderRow]) -> FolderCounts {
        count_folder_rows(rows)
    }

    /// Stages a double-clicked node; returns whether intent was staged.
    /// Directory clicks stage nothing and keep a previous staging.
    pub fn stage_double_click(&mut self, node: &FolderTreeNode) -> bool {
        stage_compare_intent(&mut self.staged, node);
        !node.row.is_dir
    }

    /// Takes the staged intent for the host view to open.
    pub fn take_staged(&mut self) -> Option<String> {
        self.staged.take()
    }

    /// Switches the preview mode. Consent never survives a mode
    /// switch: every preview needs its own explicit Apply consent.
    pub fn set_sync_mode(&mut self, mode: SyncModeChoice) {
        self.sync_mode = mode;
        self.consented = false;
    }

    /// Builds the dry-run preview for the current mode over folder rows.
    pub fn preview(&self, rows: &[FolderRow]) -> SyncPreview {
        sync_preview(rows, self.sync_mode)
    }

    /// Whether Apply may run now: consented and steps planned.
    pub fn can_apply_now(&self, preview: &SyncPreview) -> bool {
        can_apply(preview, self.consented)
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    fn glyph_row(path: &str, is_dir: bool, glyph: char) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph,
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: glyph != '=',
        }
    }

    fn node(path: &str, is_dir: bool, glyph: char) -> FolderTreeNode {
        FolderTreeNode {
            row: glyph_row(path, is_dir, glyph),
            depth: 0,
            is_expanded: false,
            has_children: false,
        }
    }

    #[test]
    fn session_toggle_and_counts() {
        let mut session = FolderSessionState::default();
        session.toggle("src");
        assert!(session.expanded.contains("src"));
        session.toggle("src");
        assert!(!session.expanded.contains("src"));

        let rows = vec![
            glyph_row("a.txt", false, '='),
            glyph_row("b.txt", false, 'L'),
        ];
        let counts = session.counts(&rows);
        assert_eq!(counts.total, 2);
        assert_eq!(counts.orphans(), 1);
    }

    #[test]
    fn session_filter_and_flatten_visible_tree() {
        let rows = vec![
            glyph_row("src", true, '='),
            glyph_row("src/a.txt", false, '>'),
            glyph_row("orphan.txt", false, 'R'),
        ];
        let mut session = FolderSessionState::default();
        // Collapsed: only roots visible.
        assert_eq!(session.visible_tree(&rows).len(), 2);
        session.toggle("src");
        assert_eq!(session.visible_tree(&rows).len(), 3);
        // Hide orphans drops one row.
        session.display.hide_orphans = true;
        assert_eq!(session.visible_tree(&rows).len(), 2);
        // Flatten ignores expansion but keeps the filter.
        session.display.flatten = true;
        let flat = session.visible_tree(&rows);
        assert_eq!(flat.len(), 2);
        assert!(flat.iter().all(|n| n.depth == 0));
    }

    #[test]
    fn session_double_click_stages_files_only() {
        let mut session = FolderSessionState::default();
        assert!(!session.stage_double_click(&node("src", true, '=')));
        assert_eq!(session.staged, None);
        assert!(session.stage_double_click(&node("src/a.txt", false, '!')));
        assert_eq!(session.staged, Some("src/a.txt".to_string()));
        // Dir click keeps previous staging.
        assert!(!session.stage_double_click(&node("src", true, '=')));
        assert_eq!(session.staged, Some("src/a.txt".to_string()));
        // Take clears.
        assert_eq!(session.take_staged(), Some("src/a.txt".to_string()));
        assert_eq!(session.staged, None);
    }

    #[test]
    fn session_mode_switch_resets_consent_and_gates_apply() {
        let rows = vec![glyph_row("nl.txt", false, '>')];
        let mut session = FolderSessionState::default();
        assert_eq!(session.sync_mode, SyncModeChoice::UpdateBoth);
        let preview = session.preview(&rows);
        assert!(!session.can_apply_now(&preview));
        session.consented = true;
        assert!(session.can_apply_now(&preview));
        // Mode switch drops consent.
        session.set_sync_mode(SyncModeChoice::Mirror);
        assert!(!session.consented);
        assert!(!session.can_apply_now(&session.preview(&rows)));
    }

    #[test]
    fn session_mirror_preview_prunes_right_only() {
        let rows = vec![
            glyph_row("keep.txt", false, '='),
            glyph_row("gone.txt", false, 'R'),
        ];
        let mut session = FolderSessionState::default();
        session.set_sync_mode(SyncModeChoice::Mirror);
        session.consented = true;
        let preview = session.preview(&rows);
        assert_eq!(preview.steps.len(), 1);
        assert!(session.can_apply_now(&preview));
        assert!(preview.summary().contains("1 step(s)"));
    }
}
