//! Hierarchical folder tree drill-down logic.
//!
//! Provides pure filtering and indentation mapping for folder rows,
//! plus headless display-filter, compare-intent staging and sync
//! preview state. Engine reuse only: glyphs come from
//! `viewmodel::build_folder_rows` (compare crate results), sync
//! preview mirrors `autocompare-compare` plan rules over those glyphs
//! without touching the filesystem.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use autocompare_compare::{
    SyncAction, SyncLiveLog, SyncMode, SyncOptions, SyncPlan, SyncReport, SyncStep, gui_action_for,
    gui_skip_reason_for, gui_sync_options, is_sync_conflict_reason, sync_conflict_next,
    sync_conflict_prev,
};

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

impl FolderTreeNode {
    /// Whether this tree node represents an orphan entry (exists on only one side: glyph 'L' or 'R').
    pub fn is_orphan(&self) -> bool {
        folder_row_is_orphan(&self.row)
    }

    /// Whether this tree node is missing on the given side (`FolderSide::Left` or `FolderSide::Right`).
    pub fn is_missing_on_side(&self, side: FolderSide) -> bool {
        folder_row_is_missing_on_side(&self.row, side)
    }

    /// Whether this tree node's cell is dimmed on the given side (missing side of an orphan entry).
    pub fn is_dimmed(&self, side: FolderSide) -> bool {
        tree_cell_is_dimmed(&self.row, side)
    }

    /// Returns the [`crate::theme::TypoRole`] for this tree node cell on the given side:
    /// - Missing side: [`crate::theme::TypoRole::Small`] (hierarchy rank 4, `is_weak() == true`)
    /// - Present content: [`crate::theme::TypoRole::Mono`] (hierarchy rank 3, `is_weak() == false`)
    pub fn cell_typo_role(&self, side: FolderSide) -> crate::theme::TypoRole {
        tree_cell_typo_role(&self.row, side)
    }

    /// Horizontal indent offset px for this node: `depth * tree_indent_step()`.
    pub fn indent_px(&self) -> f32 {
        tree_indent_for_depth(self.depth)
    }

    /// Separated path/name column start offset px: indent + expander cell + status cell + gap.
    pub fn path_x_separated(&self) -> f32 {
        folder_row_path_x_separated(self.depth)
    }

    /// Horizontal positions px for vertical ancestor indent guides (`0..depth`).
    pub fn indent_guide_xs(&self) -> Vec<f32> {
        tree_indent_guide_xs(self.depth)
    }
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

// ----------------------------------------------------------------------------
// Folder drill-down row geometry (first): fixed expander cell + indent.
// BC pattern: har row me expander/glyph/path ek hi column origin se shuru
// hote hain — file rows (bina expander) ka path dir rows se aage-peeche
// nahi hota. Paint har row me expander cell reserve karta hai (button ya
// blank), phir `tree_indent_for_depth(depth)` offset lagta hai.
// ----------------------------------------------------------------------------

/// Folder drill-down expander cell width px: standard 16px glyph cell
/// (`home::SESSION_TREE_EXPANDER_W` parity) — `▶/▼` button ya blank
/// jagah, har row me fixed taaki glyph/path columns align rahen.
pub const FOLDER_EXPANDER_W: f32 = 16.0;

/// Folder drill-down expander cell width px (single source).
/// Pure; unit-tested.
pub fn folder_expander_width() -> f32 {
    FOLDER_EXPANDER_W
}

/// Drill-down row ka path-column x-offset px: indent + fixed expander
/// cell. Paint path cell isi offset se shuru karta hai (glyph cell ke
/// baad), taaki file/dir rows ek baseline par rahen. Pure; unit-tested.
pub fn folder_row_x_offset(depth: usize) -> f32 {
    tree_indent_for_depth(depth) + folder_expander_width()
}

// ----------------------------------------------------------------------------
// Expand/collapse hit geometry (headless; paint owner ke paas).
// Expander cell fixed 16px (`FOLDER_EXPANDER_W`); hit poori row height
// (24px) par taaki chevron chhota tap target na bane.
// ----------------------------------------------------------------------------

/// Expander hit height px: poori row (`FOLDER_ROW_HEIGHT` parity).
pub const FOLDER_EXPANDER_HIT_H: f32 = FOLDER_ROW_HEIGHT;

/// Expander hit rect `(x0, y0, x1, y1)` px: indent se expander cell tak,
/// `row_top` se poori row height tak. Pure; unit-tested.
pub fn folder_expander_hit_rect(depth: usize, row_top: f32) -> (f32, f32, f32, f32) {
    let x0 = tree_indent_for_depth(depth);
    let x1 = x0 + folder_expander_width();
    (x0, row_top, x1, row_top + FOLDER_EXPANDER_HIT_H)
}

/// Expander hit test: `x` cell ke andar hai ya nahi. Non-finite `x`
/// kabhi hit nahi. Pure; unit-tested.
pub fn folder_expander_contains_x(depth: usize, x: f32) -> bool {
    if !x.is_finite() {
        return false;
    }
    let x0 = tree_indent_for_depth(depth);
    x >= x0 && x <= x0 + folder_expander_width()
}

// ----------------------------------------------------------------------------
// Folder indent guides + separated status marks (headless; paint owner ke paas).
// BC folder tree me har depth level par vertical guide rehti hai aur status
// mark (`=`, `>`, `R` ...) apne fixed cell me path se alag padhta hai —
// hamara purana `glyph + space + path` flat tha. Geometry first: guides
// existing [`tree_indent_step`] (12.0, MD scale) par, status cell
// `SPACE_LG` (16.0) + `SPACE_SM` gap (8.0) par. Typography second: status
// mono body-strong (path body-regular se weight se alag; colors frozen).
// Legacy [`folder_row_x_offset`] frozen hai (dusri waves ke tests us par
// hain); paint adopt kare to path [`folder_row_path_x_separated`] se shuru
// kare, status [`tree_status_mark_x`] par fixed cell me.
// WIRING (paint): har visible node ke liye `tree_indent_guide_xs(depth)`
// par vertical guides, `tree_status_mark_x(depth)` par status glyph
// `tree_status_typo()` me, path `folder_row_path_x_separated(depth)` se.
// ----------------------------------------------------------------------------

/// Folder status-mark cell width px: one LG step (expander parity 16.0).
pub const TREE_STATUS_MARK_W: f32 = crate::theme::SPACE_LG;

/// Folder status-mark to path gap px: one SM step.
pub const TREE_STATUS_GAP: f32 = crate::theme::SPACE_SM;

/// Status-mark cell width px (single source). Pure; unit-tested.
pub fn tree_status_mark_width() -> f32 {
    TREE_STATUS_MARK_W
}

/// Status-mark to path gap px (single source). Pure; unit-tested.
pub fn tree_status_gap() -> f32 {
    TREE_STATUS_GAP
}

/// Status-mark cell x-offset px: indent + fixed expander cell ke baad.
/// Legacy [`folder_row_x_offset`] ka glyph origin yahi hai, par ab glyph
/// fixed [`TREE_STATUS_MARK_W`] cell me paint hota hai taaki path se alag
/// padhe. Pure; unit-tested.
pub fn tree_status_mark_x(depth: usize) -> f32 {
    tree_indent_for_depth(depth) + folder_expander_width()
}

/// Separated path-column x-offset px: status cell + gap ke baad.
/// Legacy [`folder_row_x_offset`] frozen hai; yeh separated layout ka
/// uttaradhikari hai (status 16px cell + 8px gap = +24px path shift).
/// Pure; unit-tested.
pub fn folder_row_path_x_separated(depth: usize) -> f32 {
    tree_status_mark_x(depth) + tree_status_mark_width() + tree_status_gap()
}

/// Ek ancestor level ki indent guide x px: uss level ke expander cell ka
/// center (`level * step + 16/2`). Guides har ancestor ke expander center
/// se guzarti hain isliye chevron column se align rehti hain. Pure.
pub fn tree_indent_guide_x(level: usize) -> f32 {
    (level as f32) * tree_indent_step() + folder_expander_width() / 2.0
}

/// Saare ancestor guides ke x px (`0..depth`): depth 0 par khaali (koi
/// guide nahi), depth N par N guides. Pure; unit-tested.
pub fn tree_indent_guide_xs(depth: usize) -> Vec<f32> {
    let mut xs = Vec::new();
    let mut level = 0usize;
    while level < depth {
        xs.push(tree_indent_guide_x(level));
        level = level.saturating_add(1);
    }
    xs
}

/// Status-mark typo: mono body-strong — path (`Body` regular) se weight se
/// alag, rang se nahi (colors frozen). Pure; unit-tested.
pub fn tree_status_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_BODY_SIZE,
        strong: true,
    }
}

// ----------------------------------------------------------------------------
// TREE2 (round-2): indent step from SPACE tokens + orphan dimming via TypoRole
// Small + Size/Modified column alignment at 960px wide (headless).
// BC tree rhythm:
// 1. Indent step strictly derived from SPACE tokens (GEO_TREE_INDENT_STEP == SPACE_MD == 12.0).
// 2. Depth rhythm: depth * 12.0px indent, with separated path at depth * 12.0 + 40.0px.
// 3. Orphan dimming via TypoRole::Small (rank 4, is_weak() == true), missing side renders "-"
//    without touching colors (colors frozen).
// 4. Size/Modified columns aligned at 960px pane width: Size at 738.0, Modified at 832.0.
//    Fixed trailing columns total 222.0px.
// ----------------------------------------------------------------------------

/// Canonical tree-side round-2 indent step derived directly from SPACE tokens
/// (`crate::theme::GEO_TREE_INDENT_STEP` == `crate::theme::SPACE_MD` == 12.0).
/// Pure; unit-tested.
pub const fn tree_indent_step_from_space() -> f32 {
    crate::theme::SPACE_MD
}

/// Tree depth indent rhythm in px for a given depth level:
/// depth * tree_indent_step() (12.0px per depth level). Pure; unit-tested.
pub fn tree_indent_rhythm_px(depth: usize) -> f32 {
    tree_indent_for_depth(depth)
}

/// Tree row path horizontal origin (px) including expander + status mark + gap:
/// depth * tree_indent_step() + 40.0. Matches [`folder_row_path_x_separated`].
/// Depth 0..=5: 40.0, 52.0, 64.0, 76.0, 88.0, 100.0. Pure; unit-tested.
pub fn tree_row_path_x(depth: usize) -> f32 {
    folder_row_path_x_separated(depth)
}

/// Returns whether a tree row cell is dimmed for an orphan entry (missing on that side).
/// Dimmed cells render placeholder dashes "-" using TypoRole::Small without color edits.
/// Pure; unit-tested.
pub fn tree_cell_is_dimmed(row: &FolderRow, side: FolderSide) -> bool {
    folder_table_cell_is_dimmed(row, side)
}

/// Returns the TypoRole for a tree row cell:
/// Missing sides on orphan rows use `TypoRole::Small` (rank 4, is_weak() == true),
/// while present entries use `TypoRole::Mono` (rank 3, tabular content).
/// Pure; unit-tested.
pub fn tree_cell_typo_role(row: &FolderRow, side: FolderSide) -> crate::theme::TypoRole {
    folder_table_cell_typo_role(row, side)
}

/// Size and Modified column alignment at 960px pane width:
/// Returns (name_x, size_x, mod_x) at 960.0px: (0.0, 738.0, 832.0).
/// Size and Modified columns align at strictly fixed horizontal positions across all depths.
/// Pure; unit-tested.
pub fn tree_wide_960_column_xs() -> (f32, f32, f32) {
    folder_table_column_xs(960.0)
}

/// Verifies that Size and Modified columns stay strictly fixed at (738.0, 832.0)
/// regardless of tree depth (depth 0 to 5+). Pure; unit-tested.
pub fn tree_columns_aligned_at_960(_depth: usize) -> (f32, f32, f32) {
    folder_table_column_xs(960.0)
}

/// `visible_count` rows wale ped ki kul height px: rows x row height.
/// Khali ped par 0 (paint khali list chhupata hai). Pure; unit-tested.
pub fn folder_tree_content_height(visible_count: usize) -> f32 {
    visible_count as f32 * FOLDER_ROW_HEIGHT
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

/// Entry-kind filter for the folder view: files, dirs, or both.
/// Paint filter-bar `Type` chunta isi se banta hai (agle wave me).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FolderEntryKind {
    /// Files aur dirs dono dikhte hain.
    #[default]
    All,
    /// Sirf files (`is_dir == false`).
    FilesOnly,
    /// Sirf directories (`is_dir == true`).
    DirsOnly,
}

impl FolderEntryKind {
    /// Filter-bar label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All types",
            Self::FilesOnly => "Files only",
            Self::DirsOnly => "Dirs only",
        }
    }
}

/// Status filter for the folder view: one glyph family at a time.
/// Legacy `hide_orphans` / `*_newer_only` checkboxes ke saath judta
/// hai (dono lagte hain); paint filter-bar `Status` chunta agle wave
/// me isi se banega.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FolderStatusFilter {
    /// Koi status rok nahi (glyph-blind).
    #[default]
    All,
    /// Sirf same rows (`=`).
    Same,
    /// Sirf content-differ rows (`!`).
    Different,
    /// Sirf left-newer rows (`>`).
    NewerLeft,
    /// Sirf right-newer rows (`<`).
    NewerRight,
    /// Sirf left-only rows (`L`).
    LeftOnly,
    /// Sirf right-only rows (`R`).
    RightOnly,
    /// Har badli row (`=` ke siva sab).
    Changed,
}

impl FolderStatusFilter {
    /// Filter-bar label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All statuses",
            Self::Same => "Same",
            Self::Different => "Different",
            Self::NewerLeft => "Newer left",
            Self::NewerRight => "Newer right",
            Self::LeftOnly => "Left only",
            Self::RightOnly => "Right only",
            Self::Changed => "Changed",
        }
    }

    /// Kya `glyph` is status me aata hai. Unknown glyph sirf `All`
    /// me dikhta hai taaki koi filter anjaan pankti kabhi galat
    /// shreni me na gin le; summary filtered rows par honest rehti hai.
    pub const fn matches_glyph(self, glyph: char) -> bool {
        match self {
            Self::All => true,
            Self::Same => glyph == '=',
            Self::Different => glyph == '!',
            Self::NewerLeft => glyph == '>',
            Self::NewerRight => glyph == '<',
            Self::LeftOnly => glyph == 'L',
            Self::RightOnly => glyph == 'R',
            Self::Changed => matches!(glyph, '!' | '>' | '<' | 'L' | 'R'),
        }
    }
}

/// Display filters for the folder view. Legacy checkbox fields
/// (`hide_orphans`, `left_newer_only`, `right_newer_only`, `flatten`)
/// untouched hain; naye `name_query` / `kind` / `status` unke saath
/// judte hain (har shart lagti hai). Filtering kabhi glyph rewrite
/// nahi karti, sirf rows chhupti hain isliye counts honest rehti hain.
/// `String` field ki wajah se `Copy` nahi, sirf `Clone`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderDisplay {
    /// Hide orphan rows (`L` / `R` glyphs).
    pub hide_orphans: bool,
    /// Show only left-newer rows (`>` glyph).
    pub left_newer_only: bool,
    /// Show only right-newer rows (`<` glyph).
    pub right_newer_only: bool,
    /// Ignore hierarchy and show a flat list.
    pub flatten: bool,
    /// Name substring (case-insensitive contains on `rel_path`).
    /// Khali ya whitespace-only = koi naam rok nahi.
    pub name_query: String,
    /// File/dir chunta.
    pub kind: FolderEntryKind,
    /// Glyph-family chunta.
    pub status: FolderStatusFilter,
}

impl FolderDisplay {
    /// Koi filter laga hai ya sab default par hain.
    pub fn is_active(&self) -> bool {
        self.active_count() > 0
    }

    /// Lage hue independent filters ki ginti (filter-bar badge `N`
    /// isi se aata hai; zyada se zyada 7).
    pub fn active_count(&self) -> usize {
        let mut count = 0usize;
        if self.hide_orphans {
            count = count.saturating_add(1);
        }
        if self.left_newer_only {
            count = count.saturating_add(1);
        }
        if self.right_newer_only {
            count = count.saturating_add(1);
        }
        if self.flatten {
            count = count.saturating_add(1);
        }
        if !self.name_query.trim().is_empty() {
            count = count.saturating_add(1);
        }
        if self.kind != FolderEntryKind::All {
            count = count.saturating_add(1);
        }
        if self.status != FolderStatusFilter::All {
            count = count.saturating_add(1);
        }
        count
    }

    /// Saare filters saaf karke default par lao (filter-bar Clear
    /// button isi ko chalata hai).
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Naam filter: khali/whitespace kabhi rokta nahi, warna
/// case-insensitive contains on `rel_path`.
fn folder_name_pass(rel_path: &str, query: &str) -> bool {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }
    rel_path.to_lowercase().contains(&trimmed.to_lowercase())
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
    match display.kind {
        FolderEntryKind::All => {}
        FolderEntryKind::FilesOnly => {
            if row.is_dir {
                return false;
            }
        }
        FolderEntryKind::DirsOnly => {
            if !row.is_dir {
                return false;
            }
        }
    }
    if !display.status.matches_glyph(row.glyph) {
        return false;
    }
    if !folder_name_pass(&row.rel_path, &display.name_query) {
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
    let kept: Vec<FolderRow> = filtered_folder_rows(rows, display);
    folder_tree(&kept, expanded_dirs)
}

/// Display filter lagakar rows ko order me rakhta hai (flatten se pehle
/// ka step). Dual-tree paint (`views::paint_folder_dual`) unfiltered
/// `viewmodel::folder_tree` se tree banata hai, isliye App isko dekar
/// filtered dual tree pata hai — paint API same rehta hai. Pure, no I/O.
pub fn filtered_folder_rows(rows: &[FolderRow], display: &FolderDisplay) -> Vec<FolderRow> {
    rows.iter()
        .filter(|row| filter_pass(row, display))
        .cloned()
        .collect()
}

// ----------------------------------------------------------------------------
// Folder filter bar: affordance state + honest empty states (headless).
// ----------------------------------------------------------------------------
//
// Bahar se dekhe folder compare ki filter patti teen cheezen karti
// hai: naam/type/status chunta, laga hua filter ginti samait dikhana,
// aur kuch na bache to saaf khali sandesh. Ye section uska headless
// spec hai taaki paint (agle wave, dusre agent ke paas) har frame same
// numbers, same shabd use kare. Geometry first: bar height, padding,
// gap sab MD spacing scale (4/8/12/16/24) par. Typography second:
// mono + size + weight, `FolderTypo` reuse, `TypoRole` Body/Caption
// precedent ke naam se, bina theme import ke. Colors frozen: yahan
// koi hue nahi, farak sirf weight se (active chip strong).
//
// WIRING (paint, agle wave): bar `folder_filter_bar_height()` par
// `folder_filter_typo()` me; active chip `folder_filter_active_typo()`
// me; Clear button `FolderDisplay::clear()` chalata hai; khali view
// `folder_empty_hint(session.empty_state(rows))` dikhata hai; counts
// line `filtered_counts_summary` wahi rehti hai (filtered rows par).

/// Filter bar ki height (px): 24.0, MD spacing scale par (row jaisi).
pub const FOLDER_FILTER_BAR_HEIGHT: f32 = 24.0;
/// Filter bar ka ek taraf ka horizontal padding (px): 8.0, MD scale par.
pub const FOLDER_FILTER_BAR_PAD_X: f32 = 8.0;
/// Filter bar controls ke beech ka gap (px): 8.0, MD scale par.
pub const FOLDER_FILTER_BAR_GAP: f32 = 8.0;

/// Filter bar ki height (px). Header/row se alag function taaki paint
/// teeno ko ek saath na badle.
pub fn folder_filter_bar_height() -> f32 {
    FOLDER_FILTER_BAR_HEIGHT
}

/// Filter bar ka ek taraf ka horizontal padding (px).
pub fn folder_filter_bar_pad_x() -> f32 {
    FOLDER_FILTER_BAR_PAD_X
}

/// Filter bar controls ke beech ka gap (px).
pub fn folder_filter_bar_gap() -> f32 {
    FOLDER_FILTER_BAR_GAP
}

/// Filter bar text ki typography: mono body-regular (naam query aur
/// chunte ek baseline par).
pub fn folder_filter_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_BODY_SIZE,
        strong: false,
    }
}

/// Active-filter chip ki typography: mono caption-strong (laga hua
/// filter ek nazar me; farak weight se hai, rang se nahi).
pub fn folder_filter_active_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_SIDE_SIZE,
        strong: true,
    }
}

/// Khali folder view ka honest kaaran: ginati kabhi jhooth na bole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderEmptyState {
    /// Koi compare hi nahi hua (model khaali).
    NoResults,
    /// Model hai par filter ke baad kuch dikhne ko nahi bacha.
    NoMatch,
    /// Model me rows hain par active filters sab chhupa rahe hain.
    /// `NoMatch` legacy generic hai (frozen — dusri waves ke tests us
    /// par hain); yeh filter-specific wording ke liye hai, paint
    /// `folder_empty_state_full` se aata hai.
    FilterPruned,
    /// Compare hua, model me rows hain, sab '=' (do identical folders,
    /// koi diff nahi). Filter-pruned aur never-compared se alag wording
    /// paata hai; paint `folder_empty_state_full_rows` se aata hai.
    AllSame,
}

/// Khali view ka kaaran chuno: `total` model rows, `visible` filtered
/// tree nodes, `has_filters` filter-bar affordance state se. `None`
/// matlab view khaali nahi (kuch dikh raha hai).
pub fn folder_empty_state(
    total_rows: usize,
    visible_rows: usize,
    has_filters: bool,
) -> Option<FolderEmptyState> {
    if total_rows == 0 {
        return Some(FolderEmptyState::NoResults);
    }
    if visible_rows == 0 && has_filters {
        return Some(FolderEmptyState::NoMatch);
    }
    None
}

/// Khali view ka kaaran model-maujoodgi ke saath: `has_model == false`
/// matlab kabhi compare hi nahi hua (never-compared → `NoResults`,
/// legacy compare-first line). Model hai par zero rows → `NoResults`;
/// rows hain par active filters sab chhupa rahe hain → filter-specific
/// `FilterPruned`; kuch dikh raha hai → `None`. Legacy
/// `folder_empty_state` frozen hai (dusri waves ke tests us par hain);
/// yeh uska filter-specific uttaradhikari hai — paint adopt kare to
/// pruned aur never-compared alag wording paate hain.
pub fn folder_empty_state_full(
    has_model: bool,
    total_rows: usize,
    visible_rows: usize,
    display: &FolderDisplay,
) -> Option<FolderEmptyState> {
    if !has_model {
        return Some(FolderEmptyState::NoResults);
    }
    if total_rows == 0 {
        return Some(FolderEmptyState::NoResults);
    }
    if visible_rows == 0 && display.is_active() {
        return Some(FolderEmptyState::FilterPruned);
    }
    None
}

/// Kya model ki har row '=' hai (do identical folders, koi diff nahi).
/// Khaali slice par `false` taaki kabhi-compare-nahi aur compare-hua-par
/// khaali kabhi same na bole. Pure; unit-tested.
pub fn folder_all_same(rows: &[FolderRow]) -> bool {
    !rows.is_empty() && rows.iter().all(|row| row.glyph == '=')
}

/// Khali view ka kaaran rows dekhkar: `folder_empty_state_full` ka
/// rows-aware uttaradhikari (wahi has_model/total/visible/display chain,
/// teesra arm joda). `!has_model` → `NoResults`; khaali model →
/// `NoResults`; ped khaali aur sab rows '=' → `AllSame` (identical
/// folders — filter chahe laga ho, honest kaaran same-pan hai);
/// ped khaali aur filters lage → `FilterPruned`; warna `None`.
/// Legacy `folder_empty_state_full` frozen hai (glyph-blind hai, uske
/// tests us par hain); paint adopt kare to identical aur pruned alag
/// wording paate hain.
pub fn folder_empty_state_full_rows(
    has_model: bool,
    rows: &[FolderRow],
    visible_rows: usize,
    display: &FolderDisplay,
) -> Option<FolderEmptyState> {
    if !has_model {
        return Some(FolderEmptyState::NoResults);
    }
    if rows.is_empty() {
        return Some(FolderEmptyState::NoResults);
    }
    if visible_rows == 0 && folder_all_same(rows) {
        return Some(FolderEmptyState::AllSame);
    }
    if visible_rows == 0 && display.is_active() {
        return Some(FolderEmptyState::FilterPruned);
    }
    None
}

/// Khali view ka paint-stable English hint. `None` par khaali string
/// (paint kuch nahi dikhata); dono khali states ke shabd tay hain
/// taaki snapshots stable rahen.
pub fn folder_empty_hint(state: Option<FolderEmptyState>) -> &'static str {
    match state {
        None => "",
        Some(FolderEmptyState::NoResults) => "folder: no results yet — compare first",
        Some(FolderEmptyState::NoMatch) => "folder: no rows match filters — clear filters",
        Some(FolderEmptyState::FilterPruned) => "folder: filters hide every row — clear filters",
        Some(FolderEmptyState::AllSame) => "folder: folders match — all same, nothing to sync",
    }
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

/// Sync preview mode for the folder view (BC parity subset).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SyncModeChoice {
    /// Copy right-newer/right-only rows left (no deletes).
    UpdateLeft,
    /// Copy left-newer/left-only rows right (no deletes).
    UpdateRight,
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
            Self::UpdateLeft => "Update Left",
            Self::UpdateRight => "Update Right",
            Self::UpdateBoth => "Update Both",
            Self::Mirror => "Mirror",
        }
    }

    /// All picker modes in display order.
    pub const fn all() -> [Self; 4] {
        [
            Self::UpdateLeft,
            Self::UpdateRight,
            Self::UpdateBoth,
            Self::Mirror,
        ]
    }

    /// Engine mode behind this choice (thin wiring: options live in
    /// `autocompare-compare` ToolSpec, never here).
    pub const fn engine_mode(self) -> SyncMode {
        match self {
            Self::UpdateLeft => SyncMode::UpdateLeft,
            Self::UpdateRight => SyncMode::UpdateRight,
            Self::UpdateBoth => SyncMode::Both,
            Self::Mirror => SyncMode::Mirror,
        }
    }
}

/// Maps the preview mode to engine sync options via the compare
/// ToolSpec (`gui_sync_options`): zero business logic here, the rules
/// live next to the engine (`plan_file` / `plan_dir`).
pub fn sync_options_for(mode: SyncModeChoice) -> SyncOptions {
    gui_sync_options(mode.engine_mode())
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

    /// Engine plan view of this preview (same steps/skips, never
    /// truncated): lets the session log through ToolSpec `SyncLiveLog`
    /// with zero format logic here.
    pub fn to_plan(&self) -> SyncPlan {
        SyncPlan {
            steps: self.steps.clone(),
            skipped: self.skipped.clone(),
            truncated: false,
        }
    }
}

/// Whether `rel` names something strictly inside `dir_rel`.
fn is_under(rel: &str, dir_rel: &str) -> bool {
    let mut prefix = dir_rel.to_string();
    prefix.push('/');
    rel.starts_with(&prefix)
}

/// Builds the sync preview for one mode over folder rows. Thin
/// wiring: every per-row verdict comes from the compare ToolSpec
/// (`gui_action_for` / `gui_skip_reason_for`), same rules as engine
/// `plan_sync` without filesystem access. Pruned-dir subtree
/// suppression stays here (structural: children die with the dir).
pub fn sync_preview(rows: &[FolderRow], mode: SyncModeChoice) -> SyncPreview {
    let opts = sync_options_for(mode);
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
        if let Some(action) = gui_action_for(row.glyph, row.is_dir, &opts) {
            preview.steps.push(SyncStep {
                rel_path: row.rel_path.clone(),
                is_dir: row.is_dir,
                action,
            });
            if row.is_dir {
                doomed.push(row.rel_path.clone());
            }
            continue;
        }
        if let Some(reason) = gui_skip_reason_for(row.glyph, row.is_dir, &opts) {
            preview
                .skipped
                .push((row.rel_path.clone(), reason.to_string()));
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

// ----------------------------------------------------------------------------
// Gap #6 polish: folder column paint geometry + typography (headless).
// ----------------------------------------------------------------------------
//
// Bahar se dekhe folder compare me har row ek hi baseline par rehti hai:
// naam wali column flex hoti hai, size/time wali fixed. Ye section us
// layout ka headless spec hai taaki paint (dusre agent ke paas) har frame
// same numbers use kare. Geometry first: row heights, padding, col gap,
// alignment, min widths. Typography second: mono + size + weight per
// column, `TypoRole` precedent ke naam se (Body/Caption), bina theme
// import ke taaki ownership disjoint rahe. Colors frozen: yahan koi hue
// nahi, sirf shape aur type.
//
// WIRING (paint, agent A): header `folder_header_height()` par
// `folder_header_typo()` me, har row `folder_row_height()` par
// `folder_typo_for_column(col)` me; Path cell x me
// `tree_indent_for_depth(depth)` ka offset jodo, phir flex width me
// paint karo; fixed cols `folder_column_min_px(col)` se kam nahi,
// `folder_column_align(col)` se align karo, cell padding
// `folder_cell_pad_x()` dono taraf. Headless text helpers
// (`pad_folder_cell`, `folder_format_row`) left-pad rehte hain taaki
// snapshots stable rahen; right-align sirf paint ka numeric polish hai.

/// Ek folder column cell ka horizontal alignment (paint ke liye).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderCellAlign {
    /// Naam/dates: left se padho.
    Left,
    /// Byte counts: right se milao taaki ankde ek line me aayen.
    Right,
}

/// Folder row ki height (px): 24.0, MD spacing scale par.
pub const FOLDER_ROW_HEIGHT: f32 = 24.0;
/// Folder header ki height (px): rows jaisi, ek baseline.
pub const FOLDER_HEADER_HEIGHT: f32 = 24.0;
/// Cell ka ek taraf ka horizontal padding (px): 8.0, MD scale par.
pub const FOLDER_CELL_PAD_X: f32 = 8.0;
/// Fixed columns ke beech ka gap (px): 8.0, MD scale par.
pub const FOLDER_COL_GAP: f32 = 8.0;
/// Nominal mono advance (px per char): min-px math ke liye.
pub const FOLDER_MONO_ADVANCE: f32 = 7.0;
/// Path flex column ki min readable width (chars me).
pub const FOLDER_PATH_MIN_CHARS: usize = 24;
/// Path/body text size (pt): `TypoRole::Body` precedent.
pub const FOLDER_BODY_SIZE: f32 = 13.0;
/// Side columns text size (pt): `TypoRole::Caption` precedent.
pub const FOLDER_SIDE_SIZE: f32 = 12.0;
/// Header text size (pt): caption-strong precedent.
pub const FOLDER_HEADER_SIZE: f32 = 12.0;

/// Paint row height (px). Header se alag function taaki paint dono ko
/// ek saath na badle.
pub fn folder_row_height() -> f32 {
    FOLDER_ROW_HEIGHT
}

/// Density-aware folder row height (px): compact 20px, comfortable 24px, spacious 28px.
/// Reuses [`crate::viewmodel::folder_row_height_for_density`]. Pure; unit-tested.
pub fn folder_row_height_for_density(density: crate::viewmodel::FolderDensity) -> f32 {
    crate::viewmodel::folder_row_height_for_density(density)
}

/// Paint header height (px).
pub fn folder_header_height() -> f32 {
    FOLDER_HEADER_HEIGHT
}

/// Cell ka ek taraf ka horizontal padding (px).
pub fn folder_cell_pad_x() -> f32 {
    FOLDER_CELL_PAD_X
}

/// Fixed columns ke beech ka gap (px).
pub fn folder_col_gap() -> f32 {
    FOLDER_COL_GAP
}

/// Ek column ka paint alignment: Path aur times left, sizes right.
/// Sizes right isliye taaki byte counts ka ikai column seedha rahe;
/// times fixed-width mono hain isliye left padhne me aasan.
pub fn folder_column_align(column: FolderColumn) -> FolderCellAlign {
    match column {
        FolderColumn::Path => FolderCellAlign::Left,
        FolderColumn::LeftSize | FolderColumn::RightSize => FolderCellAlign::Right,
        FolderColumn::LeftTime | FolderColumn::RightTime => FolderCellAlign::Left,
    }
}

/// Ek column ki min paint width (px): fixed cols ke liye
/// `chars * advance + 2 * pad`, Path ke liye flex-min
/// (`FOLDER_PATH_MIN_CHARS` se). Non-finite risk nahi: sab const
/// positive hain, `width_chars` saturating cast hai.
pub fn folder_column_min_px(column: FolderColumn) -> f32 {
    let chars: f32 = match column {
        FolderColumn::Path => FOLDER_PATH_MIN_CHARS as f32,
        FolderColumn::LeftSize | FolderColumn::RightSize => FOLDER_SIZE_CHARS as f32,
        FolderColumn::LeftTime | FolderColumn::RightTime => FOLDER_TIME_CHARS as f32,
    };
    chars * FOLDER_MONO_ADVANCE + FOLDER_CELL_PAD_X * 2.0
}

/// Ek folder column ki typography: mono + size + weight.
/// Paint isko `TypoRole` se map kare: Path body-regular, sides
/// caption-regular, header caption-strong. Colors yahan nahi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FolderTypo {
    /// Mono font me paint karo (sizes/times/paths sab mono).
    pub mono: bool,
    /// Text size (pt).
    pub size: f32,
    /// Bold/strong weight.
    pub strong: bool,
}

/// Body/side cell ki typo: Path body-regular mono, baaki
/// caption-regular mono.
pub fn folder_typo_for_column(column: FolderColumn) -> FolderTypo {
    match column {
        FolderColumn::Path => FolderTypo {
            mono: true,
            size: FOLDER_BODY_SIZE,
            strong: false,
        },
        FolderColumn::LeftSize
        | FolderColumn::RightSize
        | FolderColumn::LeftTime
        | FolderColumn::RightTime => FolderTypo {
            mono: true,
            size: FOLDER_SIDE_SIZE,
            strong: false,
        },
    }
}

/// Header ki typo: caption-strong mono (column labels ek nazar me).
pub fn folder_header_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_HEADER_SIZE,
        strong: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autocompare_compare::SyncDirection;

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
        assert_eq!(SyncModeChoice::UpdateLeft.label(), "Update Left");
        assert_eq!(SyncModeChoice::UpdateRight.label(), "Update Right");
        assert_eq!(SyncModeChoice::UpdateBoth.label(), "Update Both");
        assert_eq!(SyncModeChoice::Mirror.label(), "Mirror");
        assert_eq!(
            SyncModeChoice::all(),
            [
                SyncModeChoice::UpdateLeft,
                SyncModeChoice::UpdateRight,
                SyncModeChoice::UpdateBoth,
                SyncModeChoice::Mirror,
            ]
        );
        assert_eq!(
            SyncModeChoice::UpdateLeft.engine_mode(),
            SyncMode::UpdateLeft
        );
        assert_eq!(
            SyncModeChoice::UpdateRight.engine_mode(),
            SyncMode::UpdateRight
        );
        assert_eq!(SyncModeChoice::UpdateBoth.engine_mode(), SyncMode::Both);
        assert_eq!(SyncModeChoice::Mirror.engine_mode(), SyncMode::Mirror);
        let left = sync_options_for(SyncModeChoice::UpdateLeft);
        assert_eq!(left.direction, SyncDirection::RightToLeft);
        assert!(!left.prune);
        let right = sync_options_for(SyncModeChoice::UpdateRight);
        assert_eq!(right.direction, SyncDirection::LeftToRight);
        assert!(!right.prune);
        let both = sync_options_for(SyncModeChoice::UpdateBoth);
        assert_eq!(both.direction, SyncDirection::NewerWins);
        assert!(!both.prune);
        let mirror = sync_options_for(SyncModeChoice::Mirror);
        assert_eq!(mirror.direction, SyncDirection::LeftToRight);
        assert!(mirror.prune);
    }

    #[test]
    fn sync_preview_four_modes_cover_update_left_right() {
        let rows = vec![
            make_glyph_row("same.txt", false, '='),
            make_glyph_row("diff.txt", false, '!'),
            make_glyph_row("nl.txt", false, '>'),
            make_glyph_row("nr.txt", false, '<'),
            make_glyph_row("lo.txt", false, 'L'),
            make_glyph_row("ro.txt", false, 'R'),
        ];
        // UpdateRight: left-newer + left-only go right; right side stays.
        let right = sync_preview(&rows, SyncModeChoice::UpdateRight);
        assert_eq!(right.steps.len(), 2);
        assert_eq!(right.skipped.len(), 3);
        // UpdateLeft: right-newer + right-only go left; left side stays.
        let left = sync_preview(&rows, SyncModeChoice::UpdateLeft);
        assert_eq!(left.steps.len(), 2);
        assert_eq!(left.skipped.len(), 3);
        // Directions stay honest per mode.
        assert_eq!(
            sync_direction_for_row(
                &make_glyph_row("nr.txt", false, '<'),
                SyncModeChoice::UpdateLeft
            ),
            SyncRowDirection::CopyToLeft
        );
        assert_eq!(
            sync_direction_for_row(
                &make_glyph_row("nr.txt", false, '<'),
                SyncModeChoice::UpdateRight
            ),
            SyncRowDirection::Manual
        );
        assert_eq!(
            sync_direction_for_row(
                &make_glyph_row("ro.txt", false, 'R'),
                SyncModeChoice::UpdateLeft
            ),
            SyncRowDirection::CopyToLeft
        );
        assert_eq!(
            sync_direction_for_row(
                &make_glyph_row("lo.txt", false, 'L'),
                SyncModeChoice::UpdateLeft
            ),
            SyncRowDirection::None
        );
        assert_eq!(
            sync_direction_for_row(
                &make_glyph_row("ro.txt", false, 'R'),
                SyncModeChoice::UpdateRight
            ),
            SyncRowDirection::None
        );
    }

    #[test]
    fn sync_conflict_nav_walks_preview_conflicts() {
        let rows = vec![
            make_glyph_row("same.txt", false, '='),
            make_glyph_row("diff.txt", false, '!'),
            make_glyph_row("nl.txt", false, '>'),
            make_glyph_row("nr.txt", false, '<'),
        ];
        let preview = sync_preview(&rows, SyncModeChoice::Mirror);
        // Mirror: diff (content) + nr (newer dest) conflict, nl copies.
        assert_eq!(sync_preview_conflict_count(&preview), 2);
        let first = sync_preview_next_conflict(&preview, None);
        let Some(first) = first else {
            return;
        };
        assert_eq!(first.0, "diff.txt");
        let second = sync_preview_next_conflict(&preview, Some(&first.0));
        let Some(second) = second else {
            return;
        };
        assert_eq!(second.0, "nr.txt");
        assert!(sync_preview_next_conflict(&preview, Some(&second.0)).is_none());
        let back = sync_preview_prev_conflict(&preview, Some(&second.0));
        let Some(back) = back else {
            return;
        };
        assert_eq!(back.0, "diff.txt");
        let last = sync_preview_prev_conflict(&preview, None);
        let Some(last) = last else {
            return;
        };
        assert_eq!(last.0, "nr.txt");
    }

    #[test]
    fn sync_live_log_lines_cover_plan_and_report() {
        let rows = vec![
            make_glyph_row("nl.txt", false, '>'),
            make_glyph_row("diff.txt", false, '!'),
        ];
        let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
        let report = SyncReportView {
            copied: 1,
            deleted: 0,
            failed: vec![("nl.txt".to_string(), "source vanished".to_string())],
        };
        let lines = sync_live_log_lines(&preview, &report);
        assert!(lines.first().is_some_and(|line| line.contains("step(s)")));
        assert!(lines.iter().any(|line| line.contains("copy L->R nl.txt")));
        assert!(lines.iter().any(|line| line.contains("diff.txt")));
        assert!(lines.iter().any(|line| line.contains("SyncReport:")));
        assert!(lines.iter().any(|line| line.contains("failed nl.txt")));
        assert_eq!(sync_log_visible_lines(200.0), 7);
        assert_eq!(sync_log_visible_lines(10.0), 0);
        assert_eq!(sync_log_visible_lines(f32::NAN), 0);
        let typo = sync_log_typo();
        assert!(typo.mono && !typo.strong && typo.size == FOLDER_SIDE_SIZE);
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
    fn drill_row_offset_is_indent_plus_fixed_expander() {
        assert_eq!(folder_expander_width(), 16.0);
        assert_eq!(FOLDER_EXPANDER_W, 16.0);
        assert_eq!(folder_row_x_offset(0), 16.0);
        assert_eq!(folder_row_x_offset(1), 28.0);
        assert_eq!(folder_row_x_offset(2), 40.0);
    }

    #[test]
    fn expander_hit_rect_covers_full_row_height() {
        assert_eq!(folder_expander_hit_rect(0, 0.0), (0.0, 0.0, 16.0, 24.0));
        assert_eq!(folder_expander_hit_rect(1, 24.0), (12.0, 24.0, 28.0, 48.0));
        assert_eq!(FOLDER_EXPANDER_HIT_H, FOLDER_ROW_HEIGHT);
        assert!(folder_expander_contains_x(0, 8.0));
        assert!(!folder_expander_contains_x(0, 17.0));
        assert!(!folder_expander_contains_x(1, 11.9));
        assert!(folder_expander_contains_x(1, 12.0));
        assert!(!folder_expander_contains_x(0, f32::NAN));
        assert_eq!(folder_tree_content_height(0), 0.0);
        assert_eq!(folder_tree_content_height(3), 72.0);
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

    #[test]
    fn folder_col_geometry_tokens_stay_on_scale() {
        // Geometry first: heights/padding/gap spacing scale (4/8/12/16/24) par.
        assert_eq!(folder_row_height(), 24.0);
        assert_eq!(folder_row_height(), FOLDER_ROW_HEIGHT);
        assert_eq!(folder_header_height(), 24.0);
        assert_eq!(folder_header_height(), FOLDER_HEADER_HEIGHT);
        assert_eq!(folder_cell_pad_x(), 8.0);
        assert_eq!(folder_col_gap(), 8.0);
        // Indent frozen rehta hai: columns usko hilate nahi.
        assert_eq!(tree_indent_step(), 12.0);
        // Alignment: path left, numeric sizes right, fixed-width times left.
        assert_eq!(
            folder_column_align(FolderColumn::Path),
            FolderCellAlign::Left
        );
        assert_eq!(
            folder_column_align(FolderColumn::LeftSize),
            FolderCellAlign::Right
        );
        assert_eq!(
            folder_column_align(FolderColumn::RightSize),
            FolderCellAlign::Right
        );
        assert_eq!(
            folder_column_align(FolderColumn::LeftTime),
            FolderCellAlign::Left
        );
        assert_eq!(
            folder_column_align(FolderColumn::RightTime),
            FolderCellAlign::Left
        );
        // Min widths: time > size (16 chars vs 10), path flex-min sabse bada.
        let size_px = folder_column_min_px(FolderColumn::LeftSize);
        let time_px = folder_column_min_px(FolderColumn::LeftTime);
        let path_px = folder_column_min_px(FolderColumn::Path);
        assert!(time_px > size_px);
        assert!(path_px > time_px);
        assert_eq!(size_px, 10.0 * FOLDER_MONO_ADVANCE + 16.0);
        assert_eq!(time_px, 16.0 * FOLDER_MONO_ADVANCE + 16.0);
        // Dono sides same width: columns kabhi misalign na hon.
        assert_eq!(size_px, folder_column_min_px(FolderColumn::RightSize));
        assert_eq!(time_px, folder_column_min_px(FolderColumn::RightTime));
    }

    #[test]
    fn folder_col_typo_mono_sizes_with_header_strong() {
        // Typography second: sab mono, header strong, baaki regular.
        let path = folder_typo_for_column(FolderColumn::Path);
        assert!(path.mono);
        assert_eq!(path.size, FOLDER_BODY_SIZE);
        assert!(!path.strong);
        for col in [
            FolderColumn::LeftSize,
            FolderColumn::RightSize,
            FolderColumn::LeftTime,
            FolderColumn::RightTime,
        ] {
            let typo = folder_typo_for_column(col);
            assert!(typo.mono);
            assert_eq!(typo.size, FOLDER_SIDE_SIZE);
            assert!(!typo.strong);
        }
        let header = folder_header_typo();
        assert!(header.mono);
        assert_eq!(header.size, FOLDER_HEADER_SIZE);
        assert!(header.strong);
        // Body bada, sides/header caption: TypoRole Body/Caption precedent.
        // Function outputs se compare (const assert se clippy bachta hai).
        assert!(path.size > folder_typo_for_column(FolderColumn::LeftSize).size);
        assert_eq!(
            folder_typo_for_column(FolderColumn::LeftSize).size,
            header.size
        );
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
    /// Conflict-nav cursor: currently focused conflict path.
    pub sync_conflict: Option<String>,
    /// Live sync log (plan lines + apply outcome, BC parity).
    pub live_log: SyncLiveLog,
    /// Active drill-down directory scope.
    pub drill_down: FolderDrillDownScope,
}

impl FolderSessionState {
    /// Flips one directory in the expanded set.
    pub fn toggle(&mut self, dir: &str) {
        toggle_expanded(&mut self.expanded, dir);
    }

    /// Active drill-down scope ko update karta hai: directory me drill karta hai.
    pub fn drill_into(&mut self, dir: &str) {
        self.drill_down.drill_into(dir);
    }

    /// Drill-down scope me ek level up jaata hai.
    pub fn drill_up(&mut self) -> bool {
        self.drill_down.drill_up()
    }

    /// Drill-down scope ko reset karta hai.
    pub fn reset_drill_down(&mut self) {
        self.drill_down.reset();
    }

    /// Scoped visible tree: drill-down scope + display filters + expanded set.
    /// Global expanded paths seedhe do (`folder_tree_scoped_global` unhe
    /// scope-relative karta hai), isliye drill ke andar bhi nested dirs
    /// khulte hain.
    pub fn scoped_visible_tree(&self, rows: &[FolderRow]) -> Vec<FolderTreeNode> {
        folder_tree_scoped_global(rows, &self.drill_down, &self.expanded, &self.display)
    }

    /// Rows the tree view paints: filtered, then hierarchical or flat.
    pub fn visible_tree(&self, rows: &[FolderRow]) -> Vec<FolderTreeNode> {
        folder_tree_filtered(rows, &self.expanded, &self.display)
    }

    /// Honest counts over all rows.
    pub fn counts(&self, rows: &[FolderRow]) -> FolderCounts {
        count_folder_rows(rows)
    }

    /// Active display filter lagakar rakhi hui rows (order same).
    /// Dual-tree paint isi ko deta hai taaki dual panes filtered
    /// dikhen; paint API same rehta hai.
    pub fn filtered_rows(&self, rows: &[FolderRow]) -> Vec<FolderRow> {
        filtered_folder_rows(rows, &self.display)
    }

    /// Honest counts over the filtered rows (dual summary isi par).
    pub fn filtered_counts(&self, rows: &[FolderRow]) -> FolderCounts {
        count_folder_rows(&self.filtered_rows(rows))
    }

    /// One-line honest summary over the filtered rows.
    pub fn filtered_counts_summary(&self, rows: &[FolderRow]) -> String {
        self.filtered_counts(rows).summary()
    }

    /// Koi filter laga hai (filter-bar badge isi se jalti hai).
    pub fn has_active_filters(&self) -> bool {
        self.display.is_active()
    }

    /// Lage hue filters ki ginti (badge `N`).
    pub fn active_filter_count(&self) -> usize {
        self.display.active_count()
    }

    /// Saare filters saaf karo (filter-bar Clear button ka headless roop).
    pub fn clear_filters(&mut self) {
        self.display.clear();
    }

    /// Khali view ka honest kaaran: model rows vs filtered visible
    /// tree par. Bina model (`rows` khaali) hamesha `NoResults`.
    pub fn empty_state(&self, rows: &[FolderRow]) -> Option<FolderEmptyState> {
        folder_empty_state(
            rows.len(),
            self.visible_tree(rows).len(),
            self.display.is_active(),
        )
    }

    /// Khali view ka paint-stable hint line.
    pub fn empty_hint(&self, rows: &[FolderRow]) -> &'static str {
        folder_empty_hint(self.empty_state(rows))
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

    /// Switches the preview mode. Consent, conflict cursor and live
    /// log never survive a mode switch: every preview needs its own
    /// explicit Apply consent, fresh nav and fresh log.
    pub fn set_sync_mode(&mut self, mode: SyncModeChoice) {
        self.sync_mode = mode;
        self.consented = false;
        self.sync_conflict = None;
        self.live_log.clear();
    }

    /// Builds the dry-run preview for the current mode over folder rows.
    pub fn preview(&self, rows: &[FolderRow]) -> SyncPreview {
        sync_preview(rows, self.sync_mode)
    }

    /// Whether Apply may run now: consented and steps planned.
    pub fn can_apply_now(&self, preview: &SyncPreview) -> bool {
        can_apply(preview, self.consented)
    }

    /// Moves the conflict cursor to the next conflict (wrap nahi).
    /// Returns the newly focused path, if any.
    pub fn next_sync_conflict(&mut self, preview: &SyncPreview) -> Option<String> {
        let found = sync_preview_next_conflict(preview, self.sync_conflict.as_deref())
            .map(|item| item.0.clone());
        self.sync_conflict = found.clone();
        found
    }

    /// Moves the conflict cursor to the previous conflict (wrap nahi).
    /// Returns the newly focused path, if any.
    pub fn prev_sync_conflict(&mut self, preview: &SyncPreview) -> Option<String> {
        let found = sync_preview_prev_conflict(preview, self.sync_conflict.as_deref())
            .map(|item| item.0.clone());
        self.sync_conflict = found.clone();
        found
    }

    /// Clears the conflict cursor (new compare / filter change).
    pub fn clear_sync_conflict(&mut self) {
        self.sync_conflict = None;
    }

    /// Logs a dry-run preview into the live log (ToolSpec lines).
    pub fn log_sync_preview(&mut self, preview: &SyncPreview) {
        self.live_log.push_plan(&preview.to_plan());
    }

    /// Logs an apply outcome into the live log (ToolSpec lines).
    pub fn log_sync_report(&mut self, report: &SyncReportView) {
        self.live_log.push_report(&report.to_report());
    }

    /// Clears the live log (new compare / mode switch outside
    /// `set_sync_mode` paths).
    pub fn clear_live_log(&mut self) {
        self.live_log.clear();
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
    fn session_filtered_rows_keep_order_for_dual_paint() {
        let rows = vec![
            glyph_row("same.txt", false, '='),
            glyph_row("left.txt", false, 'L'),
            glyph_row("newer.txt", false, '>'),
        ];
        let mut session = FolderSessionState::default();
        // Bina filter: sab rows, order same.
        let kept = session.filtered_rows(&rows);
        assert_eq!(kept.len(), 3);
        let Some(middle) = kept.get(1) else {
            return;
        };
        assert_eq!(middle.rel_path, "left.txt");
        // Hide orphans: dual paint ko orphan-free list milta hai.
        session.display.hide_orphans = true;
        let kept = session.filtered_rows(&rows);
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().all(|r| r.glyph != 'L' && r.glyph != 'R'));
        // Left-newer only: sirf '>' glyph.
        session.display.hide_orphans = false;
        session.display.left_newer_only = true;
        let kept = session.filtered_rows(&rows);
        assert_eq!(kept.len(), 1);
        let Some(first) = kept.first() else {
            return;
        };
        assert_eq!(first.glyph, '>');
    }

    #[test]
    fn session_filtered_counts_match_filtered_tree() {
        let rows = vec![
            glyph_row("same.txt", false, '='),
            glyph_row("left.txt", false, 'L'),
            glyph_row("right.txt", false, 'R'),
            glyph_row("newer.txt", false, '>'),
        ];
        let mut session = FolderSessionState::default();
        // Bina filter: poori ginti.
        assert_eq!(session.filtered_counts(&rows).total, 4);
        assert!(session.filtered_counts_summary(&rows).contains("4 entries"));
        // Hide orphans: ginti filtered tree par.
        session.display.hide_orphans = true;
        let counts = session.filtered_counts(&rows);
        assert_eq!(counts.total, 2);
        assert_eq!(counts.orphans(), 0);
        assert!(session.filtered_counts_summary(&rows).contains("2 entries"));
        // Purana unfiltered counts untouched rehta hai.
        assert_eq!(session.counts(&rows).total, 4);
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

    #[test]
    fn session_conflict_cursor_and_live_log() {
        let rows = vec![
            glyph_row("diff.txt", false, '!'),
            glyph_row("nl.txt", false, '>'),
            glyph_row("nr.txt", false, '<'),
        ];
        let mut session = FolderSessionState::default();
        let preview = sync_preview(&rows, SyncModeChoice::Mirror);
        // Cursor walks conflicts in plan order, then clears.
        assert_eq!(session.sync_conflict, None);
        assert_eq!(
            session.next_sync_conflict(&preview),
            Some("diff.txt".to_string())
        );
        assert_eq!(
            session.next_sync_conflict(&preview),
            Some("nr.txt".to_string())
        );
        assert_eq!(session.next_sync_conflict(&preview), None);
        assert_eq!(
            session.prev_sync_conflict(&preview),
            Some("nr.txt".to_string())
        );
        session.clear_sync_conflict();
        assert_eq!(session.sync_conflict, None);
        // Live log records plan + report honestly.
        session.log_sync_preview(&preview);
        assert!(!session.live_log.is_empty());
        assert!(
            session
                .live_log
                .lines()
                .iter()
                .any(|line| line.contains("copy L->R nl.txt"))
        );
        session.log_sync_report(&SyncReportView {
            copied: 1,
            deleted: 0,
            failed: Vec::new(),
        });
        assert!(
            session
                .live_log
                .lines()
                .iter()
                .any(|line| line.contains("SyncReport:"))
        );
        session.clear_live_log();
        assert!(session.live_log.is_empty());
        // Mode switch resets cursor + log + consent together.
        session.log_sync_preview(&preview);
        session.next_sync_conflict(&preview);
        session.consented = true;
        session.set_sync_mode(SyncModeChoice::UpdateLeft);
        assert!(!session.consented);
        assert_eq!(session.sync_conflict, None);
        assert!(session.live_log.is_empty());
    }
}

// ----------------------------------------------------------------------------
// Dual-tree folder panes (headless; paint: `views::paint_folder_dual`).
// Gap #19 kadam 1: single list se synced left/right trees ki taraf pehla
// kadam. Dono pane ek hi node list dekhte hain; select aur scroll ek shared
// state me rehte hain taaki panes kabhi alag na hon. Per-row action menu
// stub hai: har label me "(stub)" saaf hai, koi I/O nahi. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Ek dual-tree row par stub action: compare ya copy direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualRowAction {
    /// Dono sides file view me kholo (stub: sirf intent stage hota hai).
    Compare,
    /// Left side ka content right par lao (stub: koi file nahi badalti).
    CopyLeftToRight,
    /// Right side ka content left par lao (stub: koi file nahi badalti).
    CopyRightToLeft,
}

impl DualRowAction {
    /// Menu label; har label stub saaf rakhta hai.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Compare => "Compare (stub)",
            Self::CopyLeftToRight => "Copy L to R (stub)",
            Self::CopyRightToLeft => "Copy R to L (stub)",
        }
    }
}

/// Ek row ke liye action menu: file rows par Compare + dono copy
/// direction, dir rows par khaali (dirs par sirf expand hota hai).
pub fn dual_row_actions(row: &FolderRow) -> Vec<DualRowAction> {
    if row.is_dir {
        Vec::new()
    } else {
        vec![
            DualRowAction::Compare,
            DualRowAction::CopyLeftToRight,
            DualRowAction::CopyRightToLeft,
        ]
    }
}

/// Left pane cell: left size + left time, missing side `-`.
/// Fixed widths via [`pad_folder_cell`] taaki mono paint me side
/// columns align rahen (orphan `-` bhi padded).
pub fn dual_left_cell(row: &FolderRow) -> String {
    format!(
        "{} | {}",
        pad_folder_cell(
            &folder_cell_text(row, FolderColumn::LeftSize),
            FOLDER_SIZE_CHARS
        ),
        pad_folder_cell(
            &folder_cell_text(row, FolderColumn::LeftTime),
            FOLDER_TIME_CHARS
        )
    )
}

/// Right pane cell: right size + right time, missing side `-`.
/// Fixed widths via [`pad_folder_cell`] taaki mono paint me side
/// columns align rahen (orphan `-` bhi padded).
pub fn dual_right_cell(row: &FolderRow) -> String {
    format!(
        "{} | {}",
        pad_folder_cell(
            &folder_cell_text(row, FolderColumn::RightSize),
            FOLDER_SIZE_CHARS
        ),
        pad_folder_cell(
            &folder_cell_text(row, FolderColumn::RightTime),
            FOLDER_TIME_CHARS
        )
    )
}

/// Dono panes ka shared select + scroll + stub-action state. Scroll ek
/// hi [`RowPager`] me hai, isliye synced scroll construction se milta
/// hai: dono pane `visible_range` se ek hi window paint karte hain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DualTreeState {
    /// Dono panes me highlight wali row (relative path).
    pub selected: Option<String>,
    /// Dono panes ka shared scroll window.
    pub pager: RowPager,
    /// Staged stub action: (action, relative path). Koi I/O nahi.
    pub pending: Option<(DualRowAction, String)>,
}

impl DualTreeState {
    /// Naya shared state: page size kam se kam 1, top 0, koi select nahi.
    pub fn new(rows_per_page: usize) -> Self {
        Self {
            selected: None,
            pager: RowPager::new(0, rows_per_page),
            pending: None,
        }
    }

    /// Dono panes me ek saath select.
    pub fn select(&mut self, rel_path: &str) {
        self.selected = Some(rel_path.to_string());
    }

    /// Dono panes ka select saaf.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Dono panes ke liye ek jaisa select check.
    pub fn is_selected(&self, rel_path: &str) -> bool {
        self.selected.as_deref() == Some(rel_path)
    }

    /// Visible node count sync karo (paint har frame chalata hai).
    pub fn sync_total(&mut self, total: usize) {
        self.pager.total_rows = total;
        self.pager.clamp_top();
    }

    /// Dono panes ka scroll ek saath: clamp ke saath shared top.
    pub fn sync_scroll(&mut self, top: usize) {
        self.pager.top = top;
        self.pager.clamp_top();
    }

    /// Dono panes ki ek hi visible window.
    pub fn visible_range(&self) -> (usize, usize) {
        self.pager.visible_range()
    }

    /// `row` dikhe isliye shared top adjust karo. Returns whether top moved.
    pub fn bring_into_view(&mut self, row: usize) -> bool {
        self.pager.bring_into_view(row)
    }

    /// Stub action stage karo (menu se); koi file nahi badalti.
    pub fn stage_action(&mut self, action: DualRowAction, rel_path: &str) {
        self.pending = Some((action, rel_path.to_string()));
    }

    /// Staged stub action saaf karo.
    pub fn clear_action(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod dual_tree_tests {
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

    #[test]
    fn dual_tree_select_syncs_both_panes() {
        let mut state = DualTreeState::new(10);
        assert_eq!(state.selected, None);
        state.select("src/a.txt");
        // Dono pane ek hi state padhte hain: ek jaisa select.
        assert!(state.is_selected("src/a.txt"));
        assert!(!state.is_selected("src/b.txt"));
        state.clear_selection();
        assert!(!state.is_selected("src/a.txt"));
        assert_eq!(state.selected, None);
    }

    #[test]
    fn dual_tree_scroll_syncs_and_clamps() {
        let mut state = DualTreeState::new(10);
        state.sync_total(25);
        state.sync_scroll(10);
        // Dono pane ek hi window dekhte hain.
        assert_eq!(state.visible_range(), (10, 20));
        // Aakhri page se aage clamp.
        state.sync_scroll(99);
        assert_eq!(state.visible_range(), (20, 25));
        // Khali tree: window band.
        state.sync_total(0);
        assert_eq!(state.visible_range(), (0, 0));
    }

    #[test]
    fn dual_tree_bring_into_view_moves_shared_top() {
        let mut state = DualTreeState::new(10);
        state.sync_total(25);
        assert!(!state.bring_into_view(5));
        assert_eq!(state.visible_range(), (0, 10));
        assert!(state.bring_into_view(24));
        assert_eq!(state.visible_range(), (15, 25));
        // Khali tree: top hilta nahi.
        let mut empty = DualTreeState::new(10);
        empty.sync_total(0);
        assert!(!empty.bring_into_view(0));
    }

    #[test]
    fn dual_row_actions_cover_files_only() {
        let file = glyph_row("a.txt", false, '!');
        let actions = dual_row_actions(&file);
        assert_eq!(
            actions,
            vec![
                DualRowAction::Compare,
                DualRowAction::CopyLeftToRight,
                DualRowAction::CopyRightToLeft,
            ]
        );
        // Har label stub saaf rakhta hai.
        for action in actions {
            assert!(action.label().contains("(stub)"));
        }
        let dir = glyph_row("src", true, '=');
        assert!(dual_row_actions(&dir).is_empty());
    }

    #[test]
    fn dual_row_action_stage_and_clear() {
        let mut state = DualTreeState::new(10);
        assert_eq!(state.pending, None);
        state.stage_action(DualRowAction::Compare, "a.txt");
        assert_eq!(
            state.pending,
            Some((DualRowAction::Compare, "a.txt".to_string()))
        );
        state.clear_action();
        assert_eq!(state.pending, None);
    }

    #[test]
    fn dual_side_cells_keep_missing_sides_as_dash() {
        let orphan = glyph_row("only-left.txt", false, 'L');
        assert!(dual_left_cell(&orphan).contains('-'));
        assert!(dual_right_cell(&orphan).contains('-'));
        let mut both = glyph_row("both.txt", false, '=');
        both.left_size = Some("12".to_string());
        both.left_time = Some("2026-01-02 03:04".to_string());
        both.right_size = Some("34".to_string());
        both.right_time = Some("2026-01-03 04:05".to_string());
        let left = dual_left_cell(&both);
        assert!(left.contains("12"));
        assert!(left.contains("2026-01-02 03:04"));
        let right = dual_right_cell(&both);
        assert!(right.contains("34"));
        assert!(right.contains("2026-01-03 04:05"));
    }

    #[test]
    fn dual_side_cells_are_fixed_width() {
        let orphan = glyph_row("only-left.txt", false, 'L');
        // `size | time`: padded size + separator + padded time.
        let left = dual_left_cell(&orphan);
        let mut parts = left.split(" | ");
        let Some(size) = parts.next() else {
            return;
        };
        let Some(time) = parts.next() else {
            return;
        };
        assert_eq!(size.chars().count(), FOLDER_SIZE_CHARS);
        assert_eq!(time.chars().count(), FOLDER_TIME_CHARS);
        // Dono sides wali row me values padded widths me rehti hain.
        let mut both = glyph_row("both.txt", false, '=');
        both.left_size = Some("12".to_string());
        both.left_time = Some("2026-01-02 03:04".to_string());
        both.right_size = Some("34".to_string());
        both.right_time = Some("2026-01-03 04:05".to_string());
        let left_text = dual_left_cell(&both);
        let mut left_parts = left_text.split(" | ");
        let Some(lsize) = left_parts.next() else {
            return;
        };
        let Some(ltime) = left_parts.next() else {
            return;
        };
        assert_eq!(lsize.chars().count(), FOLDER_SIZE_CHARS);
        assert_eq!(ltime.chars().count(), FOLDER_TIME_CHARS);
        assert!(lsize.contains("12"));
        assert!(ltime.contains("2026-01-02 03:04"));
        let right_text = dual_right_cell(&both);
        let mut right_parts = right_text.split(" | ");
        let Some(rsize) = right_parts.next() else {
            return;
        };
        let Some(rtime) = right_parts.next() else {
            return;
        };
        assert_eq!(rsize.chars().count(), FOLDER_SIZE_CHARS);
        assert_eq!(rtime.chars().count(), FOLDER_TIME_CHARS);
    }
}

// ----------------------------------------------------------------------------
// Folder2 dual-pane fill + filter row geometry (headless; paint `views.rs`).
// BC pattern, outside observation only: dono panes + gap hamesha row bharte
// hain (tang window scroll karti hai, crush nahi); filter row naam field +
// Type/Status chunte (+ active chip + Clear jab lage) ek fit-check par.
// Geometry first: widths, heights, chrome, offsets, wrap — sab maujooda
// rhythm tokens se derive. Typography second: koi naya size nahi — bar
// `folder_filter_typo` (body-regular mono), chip `folder_filter_active_typo`
// (caption-strong mono) reuse; Clear button paint-default. Colors frozen.
// WIRING (paint): panes `folder_dual_pane_widths(avail, split)` se (yeh
// `split_picture_widths` parity hai); rows `folder_dual_visible_rows` se;
// filter bar `folder_filter_row_content_width(active)` se wrap faisla karke
// `folder_filter_row_height(wrapped)` par, controls
// `folder_filter_row_offsets(active)` se.
// ----------------------------------------------------------------------------

/// Dual-pane min half width px: `views::MIN_HALF` parity (wahan private hai,
/// isliye own const + parity note; paint wahi floor rakhta hai taaki tang
/// window scroll kare, fields crush na hon).
pub const FOLDER_DUAL_MIN_HALF: f32 = 180.0;
/// Dual-pane beech gap px: section rhythm ([`crate::theme::GEO_SECTION_GAP`]).
pub const FOLDER_DUAL_GAP: f32 = crate::theme::GEO_SECTION_GAP;
/// Dual-pane split handle hit width px: model single-source
/// ([`crate::viewmodel::SPLIT_HANDLE_HIT_W`]).
pub const FOLDER_DUAL_HANDLE_W: f32 = crate::viewmodel::SPLIT_HANDLE_HIT_W;
/// Dual-pane page bar height px: standard hit row
/// ([`crate::theme::CHROME_HEADER_BAR_H`]).
pub const FOLDER_DUAL_PAGE_BAR_H: f32 = crate::theme::CHROME_HEADER_BAR_H;

/// Dono folder panes ki widths px: `views::split_picture_widths` parity
/// (gap ghata kar, `MIN_HALF` floor, leftover fill taaki pair hamesha row
/// bhare). Fraction shared clamp ([`crate::viewmodel::clamp_split_fraction`],
/// 0.20..=0.80); non-finite fraction default 0.50, non-finite avail floor
/// pair. Pure; unit-tested.
pub fn folder_dual_pane_widths(avail: f32, fraction: f32) -> (f32, f32) {
    if !avail.is_finite() {
        return (FOLDER_DUAL_MIN_HALF, FOLDER_DUAL_MIN_HALF);
    }
    let total = (avail - FOLDER_DUAL_GAP).max(2.0 * FOLDER_DUAL_MIN_HALF);
    let share = if fraction.is_finite() {
        crate::viewmodel::clamp_split_fraction(fraction)
    } else {
        crate::viewmodel::SPLIT_DEFAULT_FRACTION
    };
    let left = (total * share).clamp(FOLDER_DUAL_MIN_HALF, total - FOLDER_DUAL_MIN_HALF);
    (left, total - left)
}

/// Dual-pane width waste px: `avail` me panes + gap ke baad bachi jagah.
/// [`folder_dual_pane_widths`] se aaye widths par 0 (pair hamesha row bharta
/// hai); bahar se aaye widths par dead-space check. Non-finite par 0 (koi
/// phantom waste nahi). Pure; unit-tested.
pub fn folder_dual_width_waste(avail: f32, left_w: f32, right_w: f32) -> f32 {
    if !avail.is_finite() || !left_w.is_finite() || !right_w.is_finite() {
        return 0.0;
    }
    (avail - (left_w + right_w + FOLDER_DUAL_GAP)).max(0.0)
}

/// Dual-pane rows se upar ka chrome px: page bar + header + 3 tight gaps
/// (summary/note ke baad, header se pehle — paint `paint_folder_dual` order
/// parity). Pending stub row par ek row + gap aur.
pub fn folder_dual_chrome_height(has_pending: bool) -> f32 {
    let mut chrome =
        FOLDER_DUAL_PAGE_BAR_H + FOLDER_HEADER_HEIGHT + crate::theme::GEO_TIGHT_GAP * 3.0;
    if has_pending {
        chrome += FOLDER_ROW_HEIGHT + crate::theme::GEO_TIGHT_GAP;
    }
    chrome
}

/// Dual-pane me `avail_h` px me fit poori rows: chrome ke baad bachi jagah
/// me kitne row-height rows. Tang/khali/non-finite budget par 0. Pure;
/// headless-tested.
pub fn folder_dual_visible_rows(avail_h: f32, has_pending: bool) -> usize {
    if !avail_h.is_finite() {
        return 0;
    }
    let inner = avail_h - folder_dual_chrome_height(has_pending);
    if inner < FOLDER_ROW_HEIGHT {
        return 0;
    }
    (inner / FOLDER_ROW_HEIGHT) as usize
}

/// Dual-pane content height px: chrome (page bar + headers, khaali ped par
/// bhi dikhte hain) + visible rows. Khaali tree par chrome-only (0 nahi —
/// panes ka frame rehta hai). Pure; headless-tested.
pub fn folder_dual_content_height(visible_count: usize, has_pending: bool) -> f32 {
    folder_dual_chrome_height(has_pending) + visible_count as f32 * FOLDER_ROW_HEIGHT
}

/// Dual-pane ko scroll window chahiye: content budget se bahar. Non-finite
/// par false. Pure; headless-tested.
pub fn folder_dual_needs_scroll(visible_count: usize, avail_h: f32, has_pending: bool) -> bool {
    if !avail_h.is_finite() {
        return false;
    }
    folder_dual_content_height(visible_count, has_pending) > avail_h
}

/// Dual-pane height waste px: content ke baad viewport me bachi jagah.
/// Overflow par 0 (scroll), non-finite par 0. Paint isi se dead-space check
/// karta hai. Pure; headless-tested.
pub fn folder_dual_height_waste(avail_h: f32, visible_count: usize, has_pending: bool) -> f32 {
    if !avail_h.is_finite() {
        return 0.0;
    }
    (avail_h - folder_dual_content_height(visible_count, has_pending)).max(0.0)
}

/// Filter naam field width px: paint `views::FIND_NEEDLE_W` parity (120).
pub const FOLDER_FILTER_NAME_W: f32 = 120.0;
/// Type combo longest label chars: "Files only" (10).
pub const FOLDER_FILTER_KIND_CHARS: usize = 10;
/// Status combo longest label chars: "All statuses" (12).
pub const FOLDER_FILTER_STATUS_CHARS: usize = 12;
/// Combo/button frame pad px: button frame precedent (text pad + frame dono
/// taraf; `home::HOME_CARD_CTA_FRAME_PAD` = 24 parity).
pub const FOLDER_FILTER_COMBO_PAD: f32 = 24.0;
/// Active chip longest text chars: "7 active" (8, `active_count` max 7).
pub const FOLDER_FILTER_CHIP_CHARS: usize = 8;
/// Clear button label chars: "Clear filters" (13).
pub const FOLDER_FILTER_CLEAR_CHARS: usize = 13;

/// Type combo width px: longest label x mono advance + frame pad.
pub fn folder_filter_kind_width() -> f32 {
    FOLDER_FILTER_KIND_CHARS as f32 * FOLDER_MONO_ADVANCE + FOLDER_FILTER_COMBO_PAD
}

/// Status combo width px: longest label x mono advance + frame pad.
pub fn folder_filter_status_width() -> f32 {
    FOLDER_FILTER_STATUS_CHARS as f32 * FOLDER_MONO_ADVANCE + FOLDER_FILTER_COMBO_PAD
}

/// Active chip width px: count label mono me, koi frame nahi.
pub fn folder_filter_chip_width() -> f32 {
    FOLDER_FILTER_CHIP_CHARS as f32 * FOLDER_MONO_ADVANCE
}

/// Clear button width px: label + button frame pad.
pub fn folder_filter_clear_width() -> f32 {
    FOLDER_FILTER_CLEAR_CHARS as f32 * FOLDER_MONO_ADVANCE + FOLDER_FILTER_COMBO_PAD
}

/// Filter row content width px: leading pad + naam + kind + status (+ chip +
/// Clear jab active). Paint `paint_folder_filter_bar` order parity: pad,
/// field, Type, Status, [chip, Clear], beech gaps. Pure; headless-tested.
pub fn folder_filter_row_content_width(has_active: bool) -> f32 {
    let mut width = FOLDER_FILTER_BAR_PAD_X
        + FOLDER_FILTER_NAME_W
        + FOLDER_FILTER_BAR_GAP
        + folder_filter_kind_width()
        + FOLDER_FILTER_BAR_GAP
        + folder_filter_status_width();
    if has_active {
        width += FOLDER_FILTER_BAR_GAP
            + folder_filter_chip_width()
            + FOLDER_FILTER_BAR_GAP
            + folder_filter_clear_width();
    }
    width
}

/// Filter row ko wrap chahiye: content `avail_w` se bahar. Non-finite par
/// false (koi jhootha wrap nahi). Pure; headless-tested.
pub fn folder_filter_row_needs_wrap(avail_w: f32, has_active: bool) -> bool {
    if !avail_w.is_finite() {
        return false;
    }
    folder_filter_row_content_width(has_active) > avail_w
}

/// Filter row height px: single row bar, wrapped par do bar + gap. Pure;
/// headless-tested.
pub fn folder_filter_row_height(wrapped: bool) -> f32 {
    if wrapped {
        FOLDER_FILTER_BAR_HEIGHT * 2.0 + FOLDER_FILTER_BAR_GAP
    } else {
        FOLDER_FILTER_BAR_HEIGHT
    }
}

/// Filter row control x-offsets px `(name, kind, status, chip, clear)`:
/// leading pad se, beech gaps ke saath, paint order parity. Inactive par
/// chip/clear 0.0 (paint unhe dikhata nahi). Pure; headless-tested.
pub fn folder_filter_row_offsets(has_active: bool) -> (f32, f32, f32, f32, f32) {
    let name_x = FOLDER_FILTER_BAR_PAD_X;
    let kind_x = name_x + FOLDER_FILTER_NAME_W + FOLDER_FILTER_BAR_GAP;
    let status_x = kind_x + folder_filter_kind_width() + FOLDER_FILTER_BAR_GAP;
    if !has_active {
        return (name_x, kind_x, status_x, 0.0, 0.0);
    }
    let chip_x = status_x + folder_filter_status_width() + FOLDER_FILTER_BAR_GAP;
    let clear_x = chip_x + folder_filter_chip_width() + FOLDER_FILTER_BAR_GAP;
    (name_x, kind_x, status_x, chip_x, clear_x)
}

#[cfg(test)]
mod folder2_tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn dual_pane_widths_fill_row_with_floor() {
        // Tokens: floor 180 (views MIN_HALF parity), gap section rhythm,
        // handle viewmodel single-source.
        assert_eq!(FOLDER_DUAL_MIN_HALF, 180.0);
        assert_eq!(FOLDER_DUAL_GAP, crate::theme::GEO_SECTION_GAP);
        assert_eq!(FOLDER_DUAL_GAP, 8.0);
        assert_eq!(FOLDER_DUAL_HANDLE_W, crate::viewmodel::SPLIT_HANDLE_HIT_W);
        // 960 canonical: (960 - 8) x 0.5 = 476 each, waste 0.
        assert_eq!(folder_dual_pane_widths(960.0, 0.5), (476.0, 476.0));
        let (left, right) = folder_dual_pane_widths(960.0, 0.5);
        assert_eq!(folder_dual_width_waste(960.0, left, right), 0.0);
        // 720 fallback: (720 - 8) x 0.5 = 356 each.
        assert_eq!(folder_dual_pane_widths(720.0, 0.5), (356.0, 356.0));
        // Clamp: 0.0 -> 0.20 floor (952 x 0.2 = 190.4).
        let (clamped, _) = folder_dual_pane_widths(960.0, 0.0);
        assert!((clamped - 190.4).abs() < 0.01);
        // Tang 300: total (292).max(360) = 360 -> floor pair, crush nahi.
        assert_eq!(folder_dual_pane_widths(300.0, 0.5), (180.0, 180.0));
        assert_eq!(folder_dual_width_waste(300.0, 180.0, 180.0), 0.0);
        // Guards: non-finite kabhi NaN layout nahi deta.
        assert_eq!(folder_dual_pane_widths(f32::NAN, 0.5), (180.0, 180.0));
        assert_eq!(folder_dual_pane_widths(960.0, f32::NAN), (476.0, 476.0));
        assert_eq!(folder_dual_width_waste(f32::NAN, 476.0, 476.0), 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn dual_vertical_fill_chrome_and_rows() {
        assert_eq!(FOLDER_DUAL_PAGE_BAR_H, 28.0);
        assert_eq!(FOLDER_DUAL_PAGE_BAR_H, crate::theme::CHROME_HEADER_BAR_H);
        // Chrome: 28 + 24 + 3x4 = 64; pending stub row +28 = 92.
        assert_eq!(folder_dual_chrome_height(false), 64.0);
        assert_eq!(folder_dual_chrome_height(true), 92.0);
        // 200px budget, no pending: (200 - 64) / 24 = 5 poori rows.
        assert_eq!(folder_dual_visible_rows(200.0, false), 5);
        assert_eq!(folder_dual_visible_rows(60.0, false), 0);
        assert_eq!(folder_dual_visible_rows(f32::NAN, false), 0);
        // Content: chrome + rows; khaali ped par chrome-only frame rehta hai.
        assert_eq!(folder_dual_content_height(0, false), 64.0);
        assert_eq!(folder_dual_content_height(5, false), 184.0);
        // 184 <= 200: scroll nahi, waste 16; 6 rows par scroll, waste 0.
        assert!(!folder_dual_needs_scroll(5, 200.0, false));
        assert_eq!(folder_dual_height_waste(200.0, 5, false), 16.0);
        assert!(folder_dual_needs_scroll(6, 200.0, false));
        assert_eq!(folder_dual_height_waste(200.0, 6, false), 0.0);
        assert!(!folder_dual_needs_scroll(5, f32::NAN, false));
        assert_eq!(folder_dual_height_waste(f32::NAN, 5, false), 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn filter_row_geometry_fits_and_wraps() {
        // Widths: kind 10x7+24 = 94, status 12x7+24 = 108, chip 8x7 = 56,
        // clear 13x7+24 = 115; naam field paint FIND_NEEDLE_W parity.
        assert_eq!(FOLDER_FILTER_NAME_W, 120.0);
        assert_eq!(FOLDER_FILTER_NAME_W, crate::views::FIND_NEEDLE_W);
        assert_eq!(folder_filter_kind_width(), 94.0);
        assert_eq!(folder_filter_status_width(), 108.0);
        assert_eq!(folder_filter_chip_width(), 56.0);
        assert_eq!(folder_filter_clear_width(), 115.0);
        // Row: 8+120+8+94+8+108 = 346; active +8+56+8+115 = 533.
        assert_eq!(folder_filter_row_content_width(false), 346.0);
        assert_eq!(folder_filter_row_content_width(true), 533.0);
        // Offsets tile bina overlap: kind 136, status 238, chip 354, clear 418.
        assert_eq!(
            folder_filter_row_offsets(false),
            (8.0, 136.0, 238.0, 0.0, 0.0)
        );
        assert_eq!(
            folder_filter_row_offsets(true),
            (8.0, 136.0, 238.0, 354.0, 418.0)
        );
        // 400px narrow: inactive fits, active wraps (24 -> 56).
        assert!(!folder_filter_row_needs_wrap(400.0, false));
        assert!(folder_filter_row_needs_wrap(400.0, true));
        assert_eq!(folder_filter_row_height(false), 24.0);
        assert_eq!(folder_filter_row_height(true), 56.0);
        assert!(!folder_filter_row_needs_wrap(f32::NAN, true));
        // Typo second: koi naya size nahi — bar body mono, chip caption-strong.
        let bar = folder_filter_typo();
        assert!(bar.mono && !bar.strong && bar.size == FOLDER_BODY_SIZE);
        let chip = folder_filter_active_typo();
        assert!(chip.mono && chip.strong && chip.size == FOLDER_SIDE_SIZE);
    }

    #[test]
    fn folder2_narrow_edge_fixture() {
        // Edge fixture `checks/gui-exq-folder2/round-1/comparison/dual-narrow-400.json`:
        // 400px window (avail 376), active filters — row wraps, panes floor par.
        let avail = 376.0;
        assert!(folder_filter_row_needs_wrap(avail, true));
        assert_eq!(folder_filter_row_height(true), 56.0);
        assert!(!folder_filter_row_needs_wrap(avail, false));
        let (left, right) = folder_dual_pane_widths(avail, 0.5);
        assert_eq!((left, right), (184.0, 184.0));
        assert!(left >= FOLDER_DUAL_MIN_HALF && right >= FOLDER_DUAL_MIN_HALF);
        assert_eq!(folder_dual_width_waste(avail, left, right), 0.0);
        // Vertical: 200px me 5 rows, waste 16 — dead space nahi badhta.
        assert_eq!(folder_dual_visible_rows(200.0, false), 5);
        assert_eq!(folder_dual_height_waste(200.0, 5, false), 16.0);
    }
}

// ----------------------------------------------------------------------------
// Three-way tree merge plan (headless; paint wiring next wave).
// Gap #7 model half (tree side): `TriplePaths` (base/left/right) se folder
// 3-way plan. Teen snapshots ki presence union se sorted rows bante hain,
// har row par ek take-side decision lagti hai, summary helper paint ki
// progress line deta hai. Conflict rule sirf presence se (content compare
// bina): base maujood + ek side ne hataya (delete-vs-keep), ya base gayab
// + dono sides ne joda (added-both-sides). Dirs kabhi conflict nahi
// (kisi bhi side par maujood dir auto-keep). Apply/I/O OPEN hai (agla
// wave ya host): ye section sirf plan + decisions + summary deta hai.
// WIRING (next wave): paint `three_way_tree_rows` se rows, per-row
// Take buttons `ThreeWayDecision` stage karen, header
// `summary().summary_line()` se. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Teen snapshots me ek path ki presence row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreeWayTreeRow {
    /// Roots se relative path, '/'-separated.
    pub rel_path: String,
    /// Kisi bhi side par directory.
    pub is_dir: bool,
    /// Base snapshot me maujood.
    pub has_base: bool,
    /// Left snapshot me maujood.
    pub has_left: bool,
    /// Right snapshot me maujood.
    pub has_right: bool,
}

/// Ek tree row par take-side decision: kaunsi side ka copy rakha jaye.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThreeWaySide {
    /// Left side ka copy rakho.
    #[default]
    Left,
    /// Base side ka copy rakho (badlav wapas lo).
    Base,
    /// Right side ka copy rakho.
    Right,
}

impl ThreeWaySide {
    /// Menu/paint label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "Take Left",
            Self::Base => "Take Base",
            Self::Right => "Take Right",
        }
    }
}

/// Ek tree row par staged decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeWayDecision {
    /// Kaunsi row (relative path).
    pub rel_path: String,
    /// Kaunsi side li jaye.
    pub side: ThreeWaySide,
}

/// Teen snapshots ki union se sorted presence rows. Har path ek baar;
/// `is_dir` pehli mili side se. Pure; unit-tested.
pub fn three_way_tree_rows(
    base: &[FolderRow],
    left: &[FolderRow],
    right: &[FolderRow],
) -> Vec<ThreeWayTreeRow> {
    let mut paths = BTreeSet::new();
    for row in base.iter().chain(left.iter()).chain(right.iter()) {
        paths.insert(row.rel_path.clone());
    }
    let mut rows = Vec::new();
    for path in paths {
        let on_base = base.iter().find(|row| row.rel_path == path);
        let on_left = left.iter().find(|row| row.rel_path == path);
        let on_right = right.iter().find(|row| row.rel_path == path);
        let probe = on_base.or(on_left).or(on_right);
        if let Some(found) = probe {
            rows.push(ThreeWayTreeRow {
                rel_path: path,
                is_dir: found.is_dir,
                has_base: on_base.is_some(),
                has_left: on_left.is_some(),
                has_right: on_right.is_some(),
            });
        }
    }
    rows
}

/// Kya ye row manual pick maangti hai (files only; dirs auto-keep).
/// Delete-vs-keep (base tha, ek side ne hataya) ya added-both-sides
/// (base nahi tha, dono ne joda). Pure; unit-tested.
pub fn is_three_way_conflict(row: &ThreeWayTreeRow) -> bool {
    if row.is_dir {
        return false;
    }
    if row.has_base {
        row.has_left != row.has_right
    } else {
        row.has_left && row.has_right
    }
}

/// Conflict rows ke relative paths, order me. Pure; unit-tested.
pub fn three_way_conflicts(rows: &[ThreeWayTreeRow]) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        if is_three_way_conflict(row) {
            out.push(row.rel_path.clone());
        }
    }
    out
}

/// Per-row progress over the tree plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThreeWayTreeSummary {
    /// Kul rows.
    pub total: usize,
    /// Asli row naam wale distinct staged picks.
    pub decided: usize,
    /// Faisle ke intezar me rows.
    pub pending: usize,
    /// Kul conflict rows.
    pub conflicts: usize,
    /// Koi conflict bina decision ke bacha hai.
    pub conflicted: bool,
}

impl ThreeWayTreeSummary {
    /// Honest one-line progress.
    pub fn summary_line(self) -> String {
        format!(
            "{} path(s), {} decided, {} pending, {} conflict(s){}",
            self.total,
            self.decided,
            self.pending,
            self.conflicts,
            if self.conflicted { " (unresolved)" } else { "" }
        )
    }
}

/// Rows + decisions par honest progress: unknown path naam wale picks
/// kabhi decided nahi ginte. Pure; unit-tested.
pub fn three_way_tree_summary(
    rows: &[ThreeWayTreeRow],
    decisions: &[ThreeWayDecision],
) -> ThreeWayTreeSummary {
    let total = rows.len();
    let mut seen: Vec<String> = Vec::new();
    for decision in decisions {
        let names_real_row = rows.iter().any(|row| row.rel_path == decision.rel_path);
        if names_real_row && !seen.contains(&decision.rel_path) {
            seen.push(decision.rel_path.clone());
        }
    }
    let decided = seen.len();
    let mut conflicts = 0usize;
    let mut unresolved = 0usize;
    for row in rows {
        if is_three_way_conflict(row) {
            conflicts = conflicts.saturating_add(1);
            if !seen.contains(&row.rel_path) {
                unresolved = unresolved.saturating_add(1);
            }
        }
    }
    ThreeWayTreeSummary {
        total,
        decided,
        pending: total.saturating_sub(decided),
        conflicts,
        conflicted: unresolved > 0,
    }
}

#[cfg(test)]
mod three_way_tree_tests {
    use super::*;

    fn snapshot_row(path: &str, is_dir: bool) -> FolderRow {
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

    fn decide(path: &str, side: ThreeWaySide) -> ThreeWayDecision {
        ThreeWayDecision {
            rel_path: path.to_string(),
            side,
        }
    }

    #[test]
    fn three_way_rows_union_sorted_with_presence() {
        let base = vec![
            snapshot_row("a.txt", false),
            snapshot_row("gone.txt", false),
        ];
        let left = vec![
            snapshot_row("a.txt", false),
            snapshot_row("new-l.txt", false),
        ];
        let right = vec![
            snapshot_row("a.txt", false),
            snapshot_row("new-r.txt", false),
        ];
        let rows = three_way_tree_rows(&base, &left, &right);
        assert_eq!(rows.len(), 4);
        // Sorted union: har path ek baar.
        let paths: Vec<String> = rows.iter().map(|row| row.rel_path.clone()).collect();
        assert_eq!(
            paths,
            vec![
                "a.txt".to_string(),
                "gone.txt".to_string(),
                "new-l.txt".to_string(),
                "new-r.txt".to_string(),
            ]
        );
        // Teenon me maujood: saare flags.
        let Some(shared) = rows.first() else {
            return;
        };
        assert!(shared.has_base && shared.has_left && shared.has_right);
        assert!(!shared.is_dir);
        // Base-only: sirf base flag.
        let Some(gone) = rows.get(1) else {
            return;
        };
        assert!(gone.has_base && !gone.has_left && !gone.has_right);
        // Teenon khaali: koi row nahi.
        let empty: Vec<FolderRow> = Vec::new();
        assert!(three_way_tree_rows(&empty, &empty, &empty).is_empty());
    }

    #[test]
    fn three_way_conflict_covers_delete_and_double_add() {
        let dir = ThreeWayTreeRow {
            rel_path: "src".to_string(),
            is_dir: true,
            has_base: true,
            has_left: true,
            has_right: false,
        };
        // Dirs kabhi conflict nahi (auto-keep).
        assert!(!is_three_way_conflict(&dir));
        let kept_everywhere = ThreeWayTreeRow {
            rel_path: "keep.txt".to_string(),
            is_dir: false,
            has_base: true,
            has_left: true,
            has_right: true,
        };
        assert!(!is_three_way_conflict(&kept_everywhere));
        // Delete-vs-keep: base tha, ek side ne hataya.
        let deleted_left = ThreeWayTreeRow {
            rel_path: "del.txt".to_string(),
            is_dir: false,
            has_base: true,
            has_left: false,
            has_right: true,
        };
        assert!(is_three_way_conflict(&deleted_left));
        // Added-both-sides: base nahi tha, dono ne joda.
        let double_added = ThreeWayTreeRow {
            rel_path: "new.txt".to_string(),
            is_dir: false,
            has_base: false,
            has_left: true,
            has_right: true,
        };
        assert!(is_three_way_conflict(&double_added));
        // Single-side add clean hai.
        let single_added = ThreeWayTreeRow {
            rel_path: "one.txt".to_string(),
            is_dir: false,
            has_base: false,
            has_left: true,
            has_right: false,
        };
        assert!(!is_three_way_conflict(&single_added));
        // Side labels paint-ready hain.
        assert_eq!(ThreeWaySide::Left.label(), "Take Left");
        assert_eq!(ThreeWaySide::Base.label(), "Take Base");
        assert_eq!(ThreeWaySide::Right.label(), "Take Right");
    }

    #[test]
    fn three_way_summary_counts_decisions_honestly() {
        let base = vec![
            snapshot_row("keep.txt", false),
            snapshot_row("del.txt", false),
        ];
        let left = vec![snapshot_row("keep.txt", false)];
        let right = vec![
            snapshot_row("keep.txt", false),
            snapshot_row("del.txt", false),
        ];
        let rows = three_way_tree_rows(&base, &left, &right);
        // del.txt delete-vs-keep conflict hai.
        assert_eq!(three_way_conflicts(&rows), vec!["del.txt".to_string()]);
        let none = three_way_tree_summary(&rows, &[]);
        assert_eq!(none.total, 2);
        assert_eq!(none.decided, 0);
        assert_eq!(none.pending, 2);
        assert_eq!(none.conflicts, 1);
        assert!(none.conflicted);
        assert!(none.summary_line().contains("1 conflict(s)"));
        // Conflict decide: conflicted mit-ta hai.
        let one = three_way_tree_summary(&rows, &[decide("del.txt", ThreeWaySide::Right)]);
        assert_eq!((one.decided, one.pending), (1, 1));
        assert!(!one.conflicted);
        // Unknown path kabhi decided nahi ginta.
        let unknown = three_way_tree_summary(
            &rows,
            &[ThreeWayDecision {
                rel_path: "ghost.txt".to_string(),
                side: ThreeWaySide::Left,
            }],
        );
        assert_eq!(unknown.decided, 0);
        assert!(unknown.conflicted);
        // Duplicate picks: aakhri jeetta, ek hi ginta hai.
        let dup = three_way_tree_summary(
            &rows,
            &[
                decide("keep.txt", ThreeWaySide::Left),
                decide("keep.txt", ThreeWaySide::Right),
                decide("del.txt", ThreeWaySide::Base),
            ],
        );
        assert_eq!((dup.decided, dup.pending), (2, 0));
        assert_eq!(dup.conflicts, 1);
        assert!(!dup.conflicted);
        assert!(dup.summary_line().contains("2 decided"));
    }
}

// ----------------------------------------------------------------------------
// Folder sync direction actions (headless; paint adoption agle wave me).
// Bahar se dekhe folder compare me har changed row par ek direction
// affordance hota hai: kaunsa side kidhar copy hoga, kya prune hoga.
// Ye section uska headless spec hai: per-row direction, badge text,
// multi-select model aur real dry-run summary. Geometry first (badge
// min width mono advance par, row height/pad/gap frozen tokens reuse),
// typography second (mono + size + weight, FolderTypo reuse, theme
// import nahi taaki ownership disjoint rahe). Colors frozen: yahan
// koi hue nahi. Paint agle wave me inhi se badges + selection +
// summary line banayega; is wave me koi egui code nahi.
// ----------------------------------------------------------------------------

/// Ek folder row ka sync direction: preview step ka per-row roop.
/// `sync_preview` ke rules se mel khata hai taaki badge kabhi plan
/// se alag na bole (dirs: sirf Mirror + right-only prune).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SyncRowDirection {
    /// Koi sync step nahi (same row ya plain dir).
    #[default]
    None,
    /// Left side ka content right par copy hoga.
    CopyToRight,
    /// Right side ka content left par copy hoga.
    CopyToLeft,
    /// Right copy prune hogi (Mirror mode).
    DeleteRight,
    /// Content differ ya unknown: manual resolve chahiye, auto step nahi.
    Manual,
}

impl SyncRowDirection {
    /// Paint badge text: fixed 2-char max, mono me paint hota hai.
    /// `None` par `=` (kuch karna nahi), manual par `?`.
    pub const fn badge(self) -> &'static str {
        match self {
            Self::None => "=",
            Self::CopyToRight => "->",
            Self::CopyToLeft => "<-",
            Self::DeleteRight => "X",
            Self::Manual => "?",
        }
    }

    /// Menu/summary label (paint agle wave me use karega).
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "No action",
            Self::CopyToRight => "Copy L to R",
            Self::CopyToLeft => "Copy R to L",
            Self::DeleteRight => "Delete on right",
            Self::Manual => "Manual resolve",
        }
    }

    /// Plan step action, `None`/`Manual` par kuch nahi.
    pub const fn action(self) -> Option<SyncAction> {
        match self {
            Self::None | Self::Manual => None,
            Self::CopyToRight => Some(SyncAction::CopyToRight),
            Self::CopyToLeft => Some(SyncAction::CopyToLeft),
            Self::DeleteRight => Some(SyncAction::DeleteRight),
        }
    }
}

/// Ek row ka sync direction: ToolSpec verdict ka paint roop.
/// Planned action seedha direction banta hai; skipped rows me
/// conflict (`differs`/`newer`) aur unknown manual hain, one-sided
/// no-op skips (`only on the ... side`) `None`. Dirs par ToolSpec
/// kabhi copy nahi deta (sirf Mirror + right-only prune).
pub fn sync_direction_for_row(row: &FolderRow, mode: SyncModeChoice) -> SyncRowDirection {
    let opts = sync_options_for(mode);
    if let Some(action) = gui_action_for(row.glyph, row.is_dir, &opts) {
        return match action {
            SyncAction::CopyToRight => SyncRowDirection::CopyToRight,
            SyncAction::CopyToLeft => SyncRowDirection::CopyToLeft,
            SyncAction::DeleteRight => SyncRowDirection::DeleteRight,
            // Unreachable via GUI modes (no left-prune mode exists).
            SyncAction::DeleteLeft => SyncRowDirection::Manual,
        };
    }
    if let Some(reason) = gui_skip_reason_for(row.glyph, row.is_dir, &opts)
        && (is_sync_conflict_reason(reason) || reason == "unknown status; left alone")
    {
        return SyncRowDirection::Manual;
    }
    SyncRowDirection::None
}

/// Ek row ka badge text: `sync_direction_for_row` ka paint roop.
/// Fixed strings taaki snapshots stable rahen.
pub fn sync_badge_for_row(row: &FolderRow, mode: SyncModeChoice) -> &'static str {
    sync_direction_for_row(row, mode).badge()
}

/// Folder sync ke liye multi-select model: kaunsi rows par direction
/// action lagna hai. Single `DualTreeState.selected` se alag hai:
/// ye set scoped dry-run + future per-row Apply ke liye hai.
/// Sirf real row paths tikte hain (`retain_real_rows` stale paths
/// hatata hai). Pure, no I/O.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderSyncSelection {
    paths: BTreeSet<String>,
}

impl FolderSyncSelection {
    /// Khali selection.
    pub fn new() -> Self {
        Self {
            paths: BTreeSet::new(),
        }
    }

    /// Ek path select karo.
    pub fn select(&mut self, rel_path: &str) {
        self.paths.insert(rel_path.to_string());
    }

    /// Ek path ka select flip karo; returns ab selected hai ya nahi.
    pub fn toggle(&mut self, rel_path: &str) -> bool {
        if self.paths.contains(rel_path) {
            self.paths.remove(rel_path);
            false
        } else {
            self.paths.insert(rel_path.to_string());
            true
        }
    }

    /// Ek path ka select hatao; returns kuch hata ya nahi.
    pub fn remove(&mut self, rel_path: &str) -> bool {
        self.paths.remove(rel_path)
    }

    /// Poora selection saaf karo.
    pub fn clear(&mut self) {
        self.paths.clear();
    }

    /// Kya ye path selected hai.
    pub fn contains(&self, rel_path: &str) -> bool {
        self.paths.contains(rel_path)
    }

    /// Selected paths ki ginti.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Kya kuch selected nahi.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Ordered path list (BTreeSet order, paint-stable).
    pub fn ordered(&self) -> Vec<String> {
        self.paths.iter().cloned().collect()
    }

    /// Stale paths hatao jo `rows` me nahi (naya compare / swap ke
    /// baad caller isi se selection honest rakhta hai).
    pub fn retain_real_rows(&mut self, rows: &[FolderRow]) {
        let mut live = BTreeSet::new();
        for row in rows {
            if self.paths.contains(row.rel_path.as_str()) {
                live.insert(row.rel_path.clone());
            }
        }
        self.paths = live;
    }
}

/// Dry-run preview ke real counts: steps actions se, skipped alag.
/// Summary line isi se banta hai taaki ginti kabhi jhooth na bole.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncSummaryCounts {
    /// `CopyToRight` steps.
    pub copies_right: usize,
    /// `CopyToLeft` steps.
    pub copies_left: usize,
    /// Delete steps (left/right dono, preview aaj sirf right deta hai).
    pub deletes: usize,
    /// Plan me na aaye entries (manual/unknown).
    pub skipped: usize,
}

impl SyncSummaryCounts {
    /// Kul planned steps (copies + deletes).
    pub fn planned(self) -> usize {
        self.copies_right
            .saturating_add(self.copies_left)
            .saturating_add(self.deletes)
    }
}

/// Preview se real counts gino. Pure; unit-tested.
pub fn summarize_sync_preview(preview: &SyncPreview) -> SyncSummaryCounts {
    let mut counts = SyncSummaryCounts {
        copies_right: 0,
        copies_left: 0,
        deletes: 0,
        skipped: preview.skipped.len(),
    };
    for step in &preview.steps {
        match step.action {
            SyncAction::CopyToRight => {
                counts.copies_right = counts.copies_right.saturating_add(1);
            }
            SyncAction::CopyToLeft => {
                counts.copies_left = counts.copies_left.saturating_add(1);
            }
            SyncAction::DeleteRight | SyncAction::DeleteLeft => {
                counts.deletes = counts.deletes.saturating_add(1);
            }
        }
    }
    counts
}

/// Honest one-line dry-run summary: real counts, koi estimate nahi.
/// Paint agle wave me isi ko summary line me dikhayega.
pub fn format_sync_summary(counts: SyncSummaryCounts) -> String {
    format!(
        "dry-run: {} L->R, {} R->L, {} delete(s), {} skipped",
        counts.copies_right, counts.copies_left, counts.deletes, counts.skipped
    )
}

/// Selection par scoped preview: sirf selected rows ka plan, wahi
/// dir-prune rules ke saath. Khali selection par khali preview.
pub fn sync_preview_for_selection(
    rows: &[FolderRow],
    mode: SyncModeChoice,
    selection: &FolderSyncSelection,
) -> SyncPreview {
    if selection.is_empty() {
        return SyncPreview::default();
    }
    let mut scoped = Vec::new();
    for row in rows {
        if selection.contains(row.rel_path.as_str()) {
            scoped.push(row.clone());
        }
    }
    sync_preview(&scoped, mode)
}

/// Sync badge cell ki fixed width (chars me): badges `->`/`<-`
/// 2-char hain, `=`/`X`/`?` 1-char (mono me pad hote hain).
pub const SYNC_BADGE_CHARS: usize = 2;

/// Badge cell ki min paint width (px): `chars * advance + 2 * pad`,
/// column math [`folder_column_min_px`] precedent par.
pub fn sync_badge_min_px() -> f32 {
    SYNC_BADGE_CHARS as f32 * FOLDER_MONO_ADVANCE + FOLDER_CELL_PAD_X * 2.0
}

/// Badge cell ki typography: mono caption-regular, delete strong
/// (prune ek nazar me alag dikhe; color frozen hai isliye weight
/// se farak). Paint agle wave me map karega.
pub fn sync_badge_typo(direction: SyncRowDirection) -> FolderTypo {
    match direction {
        SyncRowDirection::DeleteRight => FolderTypo {
            mono: true,
            size: FOLDER_SIDE_SIZE,
            strong: true,
        },
        _ => FolderTypo {
            mono: true,
            size: FOLDER_SIDE_SIZE,
            strong: false,
        },
    }
}

/// Dry-run summary line ki typography: mono caption-regular.
pub fn sync_summary_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_SIDE_SIZE,
        strong: false,
    }
}

// ----------------------------------------------------------------------------
// Sync conflict nav + live log (headless; paint wiring next wave).
// BC parity: dry-run ke baad conflicting rows par Next/Prev jump aur
// apply ke dauran live log. Dono ToolSpec se aate hain (compare crate):
// conflict predicate + next/prev wahi, log lines `SyncLiveLog` se.
// Ye section sirf preview-level thin wrappers deta hai taaki paint
// `SyncPreview` par seedha kaam kare. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Preview ke conflict rows (ToolSpec predicate par): `(path, reason)`.
/// Paint ka conflict badge/gutter isi se banta hai.
pub fn sync_preview_conflicts(preview: &SyncPreview) -> Vec<&(String, String)> {
    let mut out = Vec::new();
    for skipped in &preview.skipped {
        if is_sync_conflict_reason(&skipped.1) {
            out.push(skipped);
        }
    }
    out
}

/// Preview me kul conflicts.
pub fn sync_preview_conflict_count(preview: &SyncPreview) -> usize {
    sync_preview_conflicts(preview).len()
}

/// Preview me agla conflict `current` ke baad (wrap nahi: aakhri ke
/// baad `None`). `None` matlab pehla conflict.
pub fn sync_preview_next_conflict<'a>(
    preview: &'a SyncPreview,
    current: Option<&str>,
) -> Option<&'a (String, String)> {
    sync_conflict_next(&preview.skipped, current)
}

/// Preview me pichhla conflict `current` se pehle (wrap nahi: pehle
/// se pehle `None`). `None` matlab aakhri conflict.
pub fn sync_preview_prev_conflict<'a>(
    preview: &'a SyncPreview,
    current: Option<&str>,
) -> Option<&'a (String, String)> {
    sync_conflict_prev(&preview.skipped, current)
}

/// Preview + report se live-log lines (BC parity): plan summary, per
/// step/skip, phir report summary + failures. Paint `lines()` ko
/// scroll view me dikhata hai. Thin wrapper: lines ToolSpec
/// `SyncLiveLog` banata hai, yahan koi format string nahi.
pub fn sync_live_log_lines(preview: &SyncPreview, report: &SyncReportView) -> Vec<String> {
    let mut log = SyncLiveLog::new();
    for line in &preview_log_lines(preview) {
        log.push(line);
    }
    for line in &report_log_lines(report) {
        log.push(line);
    }
    log.lines().to_vec()
}

/// Preview ka log hissa: summary + steps + skips (plan order).
fn preview_log_lines(preview: &SyncPreview) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(preview.summary());
    for step in &preview.steps {
        let verb = match step.action {
            SyncAction::CopyToRight => "copy L->R",
            SyncAction::CopyToLeft => "copy R->L",
            SyncAction::DeleteRight => "delete R",
            SyncAction::DeleteLeft => "delete L",
        };
        lines.push(format!("{verb} {}", step.rel_path));
    }
    for (rel, reason) in &preview.skipped {
        lines.push(format!("skip {rel} -- {reason}"));
    }
    lines
}

/// Apply report ka headless view (GUI `SyncReport` mirror na banaye:
/// compare report se paint-line banane ke liye chhota struct).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReportView {
    /// Safal copies.
    pub copied: usize,
    /// Safal deletes.
    pub deleted: usize,
    /// Asafal steps: (path, reason).
    pub failed: Vec<(String, String)>,
}

impl SyncReportView {
    /// Honest one-line summary (engine `SyncReport::summary` parity).
    pub fn summary(&self) -> String {
        format!(
            "SyncReport: {} copied, {} deleted, {} failed",
            self.copied,
            self.deleted,
            self.failed.len()
        )
    }

    /// Engine report view of this summary: lets the session log
    /// through ToolSpec `SyncLiveLog` with zero format logic here.
    pub fn to_report(&self) -> SyncReport {
        SyncReport {
            copied: self.copied,
            deleted: self.deleted,
            failed: self.failed.clone(),
        }
    }
}

/// Report ka log hissa: summary + failures.
fn report_log_lines(report: &SyncReportView) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(report.summary());
    for (rel, reason) in &report.failed {
        lines.push(format!("failed {rel} -- {reason}"));
    }
    lines
}

/// Live-log view geometry: kitni lines `avail_h` px me fit hongi
/// (row height + tight gap par). Tang/non-finite budget par 0.
pub fn sync_log_visible_lines(avail_h: f32) -> usize {
    if !avail_h.is_finite() {
        return 0;
    }
    let row = FOLDER_ROW_HEIGHT + crate::theme::GEO_TIGHT_GAP;
    if avail_h < FOLDER_ROW_HEIGHT {
        return 0;
    }
    (avail_h / row) as usize
}

/// Live-log line typography: mono caption-regular (summary line parity).
pub fn sync_log_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_SIDE_SIZE,
        strong: false,
    }
}

// ----------------------------------------------------------------------------
// FolderCompare drill-down scope & breadcrumbs (leaf-3.2 exquisite parity):
// Manages subfolder drill-down navigation, breadcrumb hierarchy, and
// scoped tree generation. Pure headless model; colors untouched.
// ----------------------------------------------------------------------------

/// One breadcrumb segment in a drill-down path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillDownCrumb {
    /// Human-readable segment name (e.g. "Root", "src", "gui").
    pub name: String,
    /// Target relative path when clicking this crumb (e.g. "" or "src" or "src/gui").
    pub target_path: String,
    /// Whether this crumb is the currently active/deepest location.
    pub is_current: bool,
}

/// State tracking the current drill-down directory scope in FolderCompare.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderDrillDownScope {
    /// Current directory prefix being drilled into ("" = root, full view).
    pub current_dir: String,
}

impl FolderDrillDownScope {
    /// New scope at repository/folder root.
    pub fn new() -> Self {
        Self {
            current_dir: String::new(),
        }
    }

    /// Whether currently at root (no drill-down active).
    pub fn is_root(&self) -> bool {
        self.current_dir.trim().is_empty()
    }

    /// Current nesting depth of drill-down (0 at root).
    pub fn depth(&self) -> usize {
        let trimmed = self.current_dir.trim();
        if trimmed.is_empty() {
            0
        } else {
            trimmed
                .chars()
                .filter(|&c| c == '/')
                .count()
                .saturating_add(1)
        }
    }

    /// Drills into a directory path.
    pub fn drill_into(&mut self, rel_path: &str) {
        let trimmed = rel_path.trim().trim_matches('/');
        if !trimmed.is_empty() {
            self.current_dir = trimmed.to_string();
        }
    }

    /// Navigates up one directory level. Returns whether scope changed.
    pub fn drill_up(&mut self) -> bool {
        if self.is_root() {
            return false;
        }
        let trimmed = self.current_dir.trim().trim_matches('/');
        match trimmed.rfind('/') {
            Some(pos) => {
                if let Some(parent) = trimmed.get(..pos) {
                    self.current_dir = parent.to_string();
                } else {
                    self.current_dir.clear();
                }
            }
            None => {
                self.current_dir.clear();
            }
        }
        true
    }

    /// Resets drill-down back to root.
    pub fn reset(&mut self) {
        self.current_dir.clear();
    }

    /// Builds the list of breadcrumbs for the current drill-down scope.
    pub fn breadcrumbs(&self) -> Vec<DrillDownCrumb> {
        let mut crumbs = Vec::new();
        let is_root = self.is_root();
        crumbs.push(DrillDownCrumb {
            name: "Root".to_string(),
            target_path: String::new(),
            is_current: is_root,
        });

        if !is_root {
            let trimmed = self.current_dir.trim().trim_matches('/');
            let mut accum = String::new();
            let segments: Vec<&str> = trimmed.split('/').collect();
            let total = segments.len();
            for (idx, seg) in segments.into_iter().enumerate() {
                if !accum.is_empty() {
                    accum.push('/');
                }
                accum.push_str(seg);
                let is_current = idx.saturating_add(1) == total;
                crumbs.push(DrillDownCrumb {
                    name: seg.to_string(),
                    target_path: accum.clone(),
                    is_current,
                });
            }
        }

        crumbs
    }
}

/// Builds a filtered, indented hierarchical view of folder rows scoped to `scope.current_dir`.
/// If at root, behaves identically to [`folder_tree_filtered`]. When scoped, only rows
/// inside `current_dir` are included, and their depth is computed relative to the scoped root.
pub fn folder_tree_scoped(
    rows: &[FolderRow],
    scope: &FolderDrillDownScope,
    expanded_dirs: &BTreeSet<String>,
    display: &FolderDisplay,
) -> Vec<FolderTreeNode> {
    if scope.is_root() {
        return folder_tree_filtered(rows, expanded_dirs, display);
    }

    let prefix = format!("{}/", scope.current_dir.trim().trim_matches('/'));
    let mut scoped_rows = Vec::new();

    for r in rows {
        if r.rel_path.starts_with(&prefix)
            && let Some(sub) = r.rel_path.get(prefix.len()..)
            && !sub.is_empty()
        {
            let mut rel_row = r.clone();
            rel_row.rel_path = sub.to_string();
            if filter_pass(&rel_row, display) {
                scoped_rows.push(rel_row);
            }
        }
    }

    folder_tree(&scoped_rows, expanded_dirs)
}

/// Auto-detects the comparison Tab from a file path extension.
pub fn detect_tab_for_path(path: &str) -> crate::home::Tab {
    let lower = path.to_lowercase();
    let ext = match lower.rsplit_once('.') {
        Some((_, ext)) => ext.trim(),
        None => return crate::home::Tab::Text,
    };
    match ext {
        "csv" | "tsv" | "tab" | "xlsx" | "ods" => crate::home::Tab::Table,
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "ico" | "svg" => {
            crate::home::Tab::Picture
        }
        "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => crate::home::Tab::Audio,
        "bin" | "exe" | "dll" | "so" | "dylib" | "o" | "obj" | "class" | "dat" | "raw" => {
            crate::home::Tab::Hex
        }
        _ => crate::home::Tab::Text,
    }
}

/// Drill-down compare intent containing target comparison tab and child paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillDownCompareIntent {
    /// Relative path within folder comparison.
    pub rel_path: String,
    /// Destination tab detected from file format/extension.
    pub target_tab: crate::home::Tab,
    /// Full left path if left root provided.
    pub left_path: String,
    /// Full right path if right root provided.
    pub right_path: String,
}

/// Builds a drill-down compare intent for a node when double-clicked.
/// Directories return `None`.
pub fn drill_down_intent_for_node(
    left_root: &str,
    right_root: &str,
    node: &FolderTreeNode,
) -> Option<DrillDownCompareIntent> {
    if node.row.is_dir {
        return None;
    }
    let rel = node.row.rel_path.trim().trim_matches('/');
    let target_tab = detect_tab_for_path(rel);
    let left_clean = left_root.trim().trim_end_matches('/');
    let right_clean = right_root.trim().trim_end_matches('/');
    let left_path = if left_clean.is_empty() {
        rel.to_string()
    } else {
        format!("{left_clean}/{rel}")
    };
    let right_path = if right_clean.is_empty() {
        rel.to_string()
    } else {
        format!("{right_clean}/{rel}")
    };

    Some(DrillDownCompareIntent {
        rel_path: rel.to_string(),
        target_tab,
        left_path,
        right_path,
    })
}

// ----------------------------------------------------------------------------
// Drill-down geometry-first contract (widths, heights, margins, padding, align):
// Rhythm tokens reuse; zero magic numbers; colors untouched.
// ----------------------------------------------------------------------------

/// Drill-down breadcrumb bar height px: standard header bar (`CHROME_HEADER_BAR_H` = 28.0).
pub const DRILL_DOWN_BAR_H: f32 = crate::theme::CHROME_HEADER_BAR_H;
/// Drill-down breadcrumb button height px: standard hit floor (24.0).
pub const DRILL_DOWN_CRUMB_H: f32 = 24.0;
/// Drill-down breadcrumb horizontal padding px: section gap (`GEO_SECTION_GAP` = 8.0).
pub const DRILL_DOWN_PAD_X: f32 = crate::theme::GEO_SECTION_GAP;
/// Drill-down breadcrumb item gap px: tight gap (`GEO_TIGHT_GAP` = 4.0).
pub const DRILL_DOWN_GAP: f32 = crate::theme::GEO_TIGHT_GAP;
/// Drill-down breadcrumb separator width px: standard 12px separator space.
pub const DRILL_DOWN_SEP_W: f32 = 12.0;

/// Breadcrumb bar height px. Pure.
pub fn drill_down_bar_height() -> f32 {
    DRILL_DOWN_BAR_H
}

/// Breadcrumb button hit height px. Pure.
pub fn drill_down_crumb_height() -> f32 {
    DRILL_DOWN_CRUMB_H
}

/// Breadcrumb horizontal pad px. Pure.
pub fn drill_down_pad_x() -> f32 {
    DRILL_DOWN_PAD_X
}

/// Breadcrumb item gap px. Pure.
pub fn drill_down_gap() -> f32 {
    DRILL_DOWN_GAP
}

/// Breadcrumb button hit area px² for a given label width. Pure.
pub fn drill_down_crumb_hit_area(label_w: f32) -> f32 {
    (label_w + DRILL_DOWN_PAD_X * 2.0) * DRILL_DOWN_CRUMB_H
}

// ----------------------------------------------------------------------------
// Drill-down typography-second contract (family, weight, line-height, spacing):
// Single-source from `TypoRole` tokens; zero new point sizes; colors untouched.
// ----------------------------------------------------------------------------

/// Ancestor breadcrumb typography: mono body regular.
pub fn drill_down_crumb_ancestor_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_BODY_SIZE,
        strong: false,
    }
}

/// Current active breadcrumb typography: mono body strong.
pub fn drill_down_crumb_current_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_BODY_SIZE,
        strong: true,
    }
}

/// Breadcrumb separator typography: mono caption regular.
pub fn drill_down_crumb_separator_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_SIDE_SIZE,
        strong: false,
    }
}

/// Drill-down intent hint typography: mono caption regular.
pub fn drill_down_hint_typo() -> FolderTypo {
    FolderTypo {
        mono: true,
        size: FOLDER_SIDE_SIZE,
        strong: false,
    }
}

#[cfg(test)]
mod sync_direction_tests {
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

    #[test]
    fn row_directions_match_preview_rules() {
        // Same: kabhi step nahi.
        assert_eq!(
            sync_direction_for_row(
                &glyph_row("same.txt", false, '='),
                SyncModeChoice::UpdateBoth
            ),
            SyncRowDirection::None
        );
        // Content differ: manual, dono modes me.
        assert_eq!(
            sync_direction_for_row(&glyph_row("d.txt", false, '!'), SyncModeChoice::Mirror),
            SyncRowDirection::Manual
        );
        // Left-newer hamesha L->R.
        assert_eq!(
            sync_direction_for_row(&glyph_row("n.txt", false, '>'), SyncModeChoice::Mirror),
            SyncRowDirection::CopyToRight
        );
        // Right-newer: UpdateBoth me R->L, Mirror me manual.
        assert_eq!(
            sync_direction_for_row(&glyph_row("n.txt", false, '<'), SyncModeChoice::UpdateBoth),
            SyncRowDirection::CopyToLeft
        );
        assert_eq!(
            sync_direction_for_row(&glyph_row("n.txt", false, '<'), SyncModeChoice::Mirror),
            SyncRowDirection::Manual
        );
        // Left-only hamesha L->R.
        assert_eq!(
            sync_direction_for_row(&glyph_row("l.txt", false, 'L'), SyncModeChoice::Mirror),
            SyncRowDirection::CopyToRight
        );
        // Right-only: UpdateBoth me R->L, Mirror me prune.
        assert_eq!(
            sync_direction_for_row(&glyph_row("r.txt", false, 'R'), SyncModeChoice::UpdateBoth),
            SyncRowDirection::CopyToLeft
        );
        assert_eq!(
            sync_direction_for_row(&glyph_row("r.txt", false, 'R'), SyncModeChoice::Mirror),
            SyncRowDirection::DeleteRight
        );
        // Unknown glyph manual (preview skip karta hai).
        assert_eq!(
            sync_direction_for_row(&glyph_row("u.txt", false, '~'), SyncModeChoice::UpdateBoth),
            SyncRowDirection::Manual
        );
        // Dirs: sirf Mirror + right-only prune, baaki none.
        assert_eq!(
            sync_direction_for_row(&glyph_row("gone", true, 'R'), SyncModeChoice::Mirror),
            SyncRowDirection::DeleteRight
        );
        assert_eq!(
            sync_direction_for_row(&glyph_row("gone", true, 'R'), SyncModeChoice::UpdateBoth),
            SyncRowDirection::None
        );
        assert_eq!(
            sync_direction_for_row(&glyph_row("src", true, '='), SyncModeChoice::Mirror),
            SyncRowDirection::None
        );
    }

    #[test]
    fn badges_are_fixed_short_strings() {
        assert_eq!(SyncRowDirection::None.badge(), "=");
        assert_eq!(SyncRowDirection::CopyToRight.badge(), "->");
        assert_eq!(SyncRowDirection::CopyToLeft.badge(), "<-");
        assert_eq!(SyncRowDirection::DeleteRight.badge(), "X");
        assert_eq!(SyncRowDirection::Manual.badge(), "?");
        // Row helper direction se mel khata hai.
        assert_eq!(
            sync_badge_for_row(&glyph_row("a.txt", false, '>'), SyncModeChoice::UpdateBoth),
            "->"
        );
        assert_eq!(
            sync_badge_for_row(&glyph_row("same.txt", false, '='), SyncModeChoice::Mirror),
            "="
        );
        // Labels khali nahi, actions honest hain.
        assert_eq!(
            SyncRowDirection::CopyToRight.action(),
            Some(SyncAction::CopyToRight)
        );
        assert_eq!(
            SyncRowDirection::CopyToLeft.action(),
            Some(SyncAction::CopyToLeft)
        );
        assert_eq!(
            SyncRowDirection::DeleteRight.action(),
            Some(SyncAction::DeleteRight)
        );
        assert_eq!(SyncRowDirection::None.action(), None);
        assert_eq!(SyncRowDirection::Manual.action(), None);
    }

    #[test]
    fn selection_toggle_and_retain() {
        let mut sel = FolderSyncSelection::new();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
        sel.select("b.txt");
        sel.select("a.txt");
        assert_eq!(
            sel.ordered(),
            vec!["a.txt".to_string(), "b.txt".to_string()]
        );
        assert!(sel.contains("a.txt"));
        assert!(!sel.contains("zzz.txt"));
        // Toggle: selected hata, unselected jodta hai.
        assert!(!sel.toggle("a.txt"));
        assert!(!sel.contains("a.txt"));
        assert!(sel.toggle("c.txt"));
        assert!(sel.contains("c.txt"));
        assert!(sel.remove("c.txt"));
        assert!(!sel.remove("c.txt"));
        // Retain: stale paths girte hain.
        let rows = vec![
            glyph_row("b.txt", false, '>'),
            glyph_row("fresh.txt", false, 'L'),
        ];
        sel.retain_real_rows(&rows);
        assert_eq!(sel.ordered(), vec!["b.txt".to_string()]);
        sel.clear();
        assert!(sel.is_empty());
    }

    #[test]
    fn summary_counts_are_real() {
        let rows = vec![
            glyph_row("same.txt", false, '='),
            glyph_row("diff.txt", false, '!'),
            glyph_row("nl.txt", false, '>'),
            glyph_row("nr.txt", false, '<'),
            glyph_row("lo.txt", false, 'L'),
            glyph_row("ro.txt", false, 'R'),
        ];
        let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
        let counts = summarize_sync_preview(&preview);
        // UpdateBoth: nl + lo L->R (2), nr + ro R->L (2), diff skipped (1).
        assert_eq!(counts.copies_right, 2);
        assert_eq!(counts.copies_left, 2);
        assert_eq!(counts.deletes, 0);
        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.planned(), 4);
        let line = format_sync_summary(counts);
        assert!(line.contains("2 L->R"));
        assert!(line.contains("2 R->L"));
        assert!(line.contains("0 delete(s)"));
        assert!(line.contains("1 skipped"));
        // Mirror: nl + lo L->R (2), ro prune (1 delete), diff + nr skipped (2).
        let mirror = sync_preview(&rows, SyncModeChoice::Mirror);
        let counts = summarize_sync_preview(&mirror);
        assert_eq!(counts.copies_right, 2);
        assert_eq!(counts.copies_left, 0);
        assert_eq!(counts.deletes, 1);
        assert_eq!(counts.skipped, 2);
        let line = format_sync_summary(counts);
        assert!(line.contains("2 L->R"));
        assert!(line.contains("1 delete(s)"));
        assert!(line.contains("2 skipped"));
    }

    #[test]
    fn scoped_preview_only_covers_selection() {
        let rows = vec![
            glyph_row("nl.txt", false, '>'),
            glyph_row("nr.txt", false, '<'),
            glyph_row("same.txt", false, '='),
        ];
        // Khali selection: khali preview.
        let empty = FolderSyncSelection::new();
        assert!(sync_preview_for_selection(&rows, SyncModeChoice::UpdateBoth, &empty).is_empty());
        // Sirf ek row: usi ka step.
        let mut one = FolderSyncSelection::new();
        one.select("nr.txt");
        let scoped = sync_preview_for_selection(&rows, SyncModeChoice::UpdateBoth, &one);
        assert_eq!(scoped.steps.len(), 1);
        let Some(step) = scoped.steps.first() else {
            return;
        };
        assert_eq!(step.rel_path, "nr.txt");
        assert_eq!(step.action, SyncAction::CopyToLeft);
        // Stale path select me ho to bhi plan me nahi aata.
        let mut stale = FolderSyncSelection::new();
        stale.select("ghost.txt");
        stale.select("nl.txt");
        let scoped = sync_preview_for_selection(&rows, SyncModeChoice::UpdateBoth, &stale);
        assert_eq!(scoped.steps.len(), 1);
        let Some(step) = scoped.steps.first() else {
            return;
        };
        assert_eq!(step.rel_path, "nl.txt");
    }

    #[test]
    fn sync_geo_typo_tokens_stay_on_scale() {
        // Geometry first: badge min width advance+pad math par.
        assert_eq!(SYNC_BADGE_CHARS, 2);
        assert_eq!(
            sync_badge_min_px(),
            2.0 * FOLDER_MONO_ADVANCE + FOLDER_CELL_PAD_X * 2.0
        );
        assert_eq!(sync_badge_min_px(), 2.0 * 7.0 + 16.0);
        // Row height/pad frozen tokens reuse hote hain.
        assert_eq!(folder_row_height(), 24.0);
        assert_eq!(folder_cell_pad_x(), 8.0);
        // Typography second: sab mono, delete strong, baaki regular.
        let plain = sync_badge_typo(SyncRowDirection::CopyToRight);
        assert!(plain.mono);
        assert_eq!(plain.size, FOLDER_SIDE_SIZE);
        assert!(!plain.strong);
        let del = sync_badge_typo(SyncRowDirection::DeleteRight);
        assert!(del.mono);
        assert!(del.strong);
        let summary = sync_summary_typo();
        assert!(summary.mono);
        assert_eq!(summary.size, FOLDER_SIDE_SIZE);
        assert!(!summary.strong);
    }
}

#[cfg(test)]
mod folderfilter_tests {
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

    fn sample_rows() -> Vec<FolderRow> {
        vec![
            glyph_row("src", true, '='),
            glyph_row("src/a.txt", false, '>'),
            glyph_row("src/b.txt", false, '='),
            glyph_row("orphan.txt", false, 'R'),
            glyph_row("left-only.txt", false, 'L'),
            glyph_row("diff.txt", false, '!'),
            glyph_row("newer-right.txt", false, '<'),
        ]
    }

    #[test]
    fn name_filter_matches_case_insensitive_and_trims() {
        let rows = sample_rows();
        let mut display = FolderDisplay {
            name_query: String::from("SRC/"),
            ..FolderDisplay::default()
        };
        let kept = filtered_folder_rows(&rows, &display);
        assert_eq!(kept.len(), 2);
        let Some(first) = kept.first() else {
            return;
        };
        assert_eq!(first.rel_path, "src/a.txt");
        // Whitespace-only = koi rok nahi.
        display.name_query = String::from("   ");
        assert_eq!(filtered_folder_rows(&rows, &display).len(), rows.len());
        // Khali = sab.
        display.name_query = String::new();
        assert_eq!(filtered_folder_rows(&rows, &display).len(), rows.len());
        // Koi mail nahi = honest khaali.
        display.name_query = String::from("zzz-no-such-file");
        assert!(filtered_folder_rows(&rows, &display).is_empty());
    }

    #[test]
    fn kind_filter_splits_files_and_dirs() {
        let rows = sample_rows();
        let display = FolderDisplay {
            kind: FolderEntryKind::FilesOnly,
            ..FolderDisplay::default()
        };
        let kept = filtered_folder_rows(&rows, &display);
        assert!(!kept.is_empty());
        assert!(kept.iter().all(|row| !row.is_dir));
        let display = FolderDisplay {
            kind: FolderEntryKind::DirsOnly,
            ..FolderDisplay::default()
        };
        let kept = filtered_folder_rows(&rows, &display);
        assert_eq!(kept.len(), 1);
        let Some(only) = kept.first() else {
            return;
        };
        assert_eq!(only.rel_path, "src");
        assert_eq!(FolderEntryKind::FilesOnly.label(), "Files only");
        assert_eq!(FolderEntryKind::DirsOnly.label(), "Dirs only");
        assert_eq!(FolderEntryKind::All.label(), "All types");
    }

    #[test]
    fn status_filter_covers_every_glyph_honestly() {
        let rows = sample_rows();
        let with = |status: FolderStatusFilter| FolderDisplay {
            status,
            ..FolderDisplay::default()
        };
        assert!(
            filtered_folder_rows(&rows, &with(FolderStatusFilter::Same))
                .iter()
                .all(|row| row.glyph == '=')
        );
        assert_eq!(
            filtered_folder_rows(&rows, &with(FolderStatusFilter::Different)).len(),
            1
        );
        let kept = filtered_folder_rows(&rows, &with(FolderStatusFilter::NewerLeft));
        assert_eq!(kept.len(), 1);
        let Some(first) = kept.first() else {
            return;
        };
        assert_eq!(first.glyph, '>');
        assert_eq!(
            filtered_folder_rows(&rows, &with(FolderStatusFilter::NewerRight)).len(),
            1
        );
        assert_eq!(
            filtered_folder_rows(&rows, &with(FolderStatusFilter::LeftOnly)).len(),
            1
        );
        assert_eq!(
            filtered_folder_rows(&rows, &with(FolderStatusFilter::RightOnly)).len(),
            1
        );
        let kept = filtered_folder_rows(&rows, &with(FolderStatusFilter::Changed));
        assert!(kept.iter().all(|row| row.glyph != '='));
        assert_eq!(kept.len(), 5);
        // Unknown glyph sirf All me dikhta hai.
        let mut odd = sample_rows();
        odd.push(glyph_row("odd.txt", false, '~'));
        assert!(
            filtered_folder_rows(&odd, &with(FolderStatusFilter::Changed))
                .iter()
                .all(|row| row.rel_path != "odd.txt")
        );
        assert_eq!(
            filtered_folder_rows(&odd, &with(FolderStatusFilter::All)).len(),
            odd.len()
        );
        assert!(FolderStatusFilter::Changed.label().contains("Changed"));
    }

    #[test]
    fn legacy_and_new_filters_narrow_together() {
        let rows = sample_rows();
        let display = FolderDisplay {
            hide_orphans: true,
            status: FolderStatusFilter::Changed,
            ..FolderDisplay::default()
        };
        // Orphans pehle hi gaye, Changed `=` hatata hai.
        let kept = filtered_folder_rows(&rows, &display);
        assert!(
            kept.iter()
                .all(|row| row.glyph != '=' && row.glyph != 'L' && row.glyph != 'R')
        );
        assert_eq!(kept.len(), 3);
        // Naam aur jod do: sirf src ped bachta hai.
        let display = FolderDisplay {
            name_query: String::from("src/"),
            ..display
        };
        let kept = filtered_folder_rows(&rows, &display);
        assert_eq!(kept.len(), 1);
        let Some(first) = kept.first() else {
            return;
        };
        assert_eq!(first.rel_path, "src/a.txt");
    }

    #[test]
    fn active_count_and_clear_cover_all_seven() {
        let mut display = FolderDisplay::default();
        assert!(!display.is_active());
        assert_eq!(display.active_count(), 0);
        display.hide_orphans = true;
        display.left_newer_only = true;
        display.right_newer_only = true;
        display.flatten = true;
        display.name_query = String::from("a");
        display.kind = FolderEntryKind::FilesOnly;
        display.status = FolderStatusFilter::Changed;
        assert!(display.is_active());
        assert_eq!(display.active_count(), 7);
        display.clear();
        assert_eq!(display, FolderDisplay::default());
        assert!(!display.is_active());
        assert_eq!(display.active_count(), 0);
    }

    #[test]
    fn empty_states_stay_honest() {
        assert_eq!(
            folder_empty_state(0, 0, false),
            Some(FolderEmptyState::NoResults)
        );
        assert_eq!(
            folder_empty_state(0, 0, true),
            Some(FolderEmptyState::NoResults)
        );
        assert_eq!(
            folder_empty_state(5, 0, true),
            Some(FolderEmptyState::NoMatch)
        );
        assert_eq!(folder_empty_state(5, 2, true), None);
        assert_eq!(folder_empty_state(5, 5, false), None);
        assert!(folder_empty_hint(None).is_empty());
        assert!(folder_empty_hint(Some(FolderEmptyState::NoResults)).contains("no results yet"));
        assert!(
            folder_empty_hint(Some(FolderEmptyState::NoMatch)).contains("no rows match filters")
        );
    }

    #[test]
    fn filter_pruned_wording_differs_from_never_compared() {
        // Never-compared (legacy line, frozen): compare-first guidance.
        let idle = folder_empty_hint(Some(FolderEmptyState::NoResults));
        assert!(idle.contains("no results yet"));
        // Filter-pruned (naya variant): prune-specific wording.
        let pruned = folder_empty_hint(Some(FolderEmptyState::FilterPruned));
        assert!(pruned.contains("hide every row"));
        assert!(pruned.contains("clear filters"));
        // Dono alag hain — ek hi idle hint nahi.
        assert_ne!(idle, pruned);
        // Legacy generic barkarar (dusri waves ke tests us par hain).
        assert!(
            folder_empty_hint(Some(FolderEmptyState::NoMatch)).contains("no rows match filters")
        );
    }

    #[test]
    fn empty_state_full_routes_pruned_vs_never_compared() {
        let rows = sample_rows();
        let total = rows.len();
        let active = FolderDisplay {
            name_query: String::from("zzz-no-such-file"),
            ..FolderDisplay::default()
        };
        assert!(active.is_active());
        // Never-compared: model hi nahi — legacy line.
        assert_eq!(
            folder_empty_state_full(false, 0, 0, &active),
            Some(FolderEmptyState::NoResults)
        );
        assert_eq!(
            folder_empty_state_full(false, 0, 0, &FolderDisplay::default()),
            Some(FolderEmptyState::NoResults)
        );
        // Model hai par zero rows (compare hua, kuch nahi) — legacy line.
        assert_eq!(
            folder_empty_state_full(true, 0, 0, &FolderDisplay::default()),
            Some(FolderEmptyState::NoResults)
        );
        // Rows hain par filter sab chhupa raha — filter-specific variant.
        assert_eq!(
            folder_empty_state_full(true, total, 0, &active),
            Some(FolderEmptyState::FilterPruned)
        );
        // Kuch dikh raha — khaali nahi.
        assert_eq!(folder_empty_state_full(true, total, 2, &active), None);
        assert_eq!(
            folder_empty_state_full(true, total, total, &FolderDisplay::default()),
            None
        );
    }

    #[test]
    fn identical_wording_differs_from_pruned_and_never_compared() {
        // Never-compared (legacy line, frozen): compare-first guidance.
        let idle = folder_empty_hint(Some(FolderEmptyState::NoResults));
        assert!(idle.contains("no results yet"));
        // Filter-pruned: prune-specific wording.
        let pruned = folder_empty_hint(Some(FolderEmptyState::FilterPruned));
        assert!(pruned.contains("hide every row"));
        // Identical folders (naya variant): same-specific wording.
        let same = folder_empty_hint(Some(FolderEmptyState::AllSame));
        assert!(same.contains("same"));
        assert!(same.contains("nothing to sync"));
        // Teenon alag hain — koi shared idle hint nahi.
        assert_ne!(idle, pruned);
        assert_ne!(idle, same);
        assert_ne!(pruned, same);
        // Identical line filter guidance nahi deta, idle line same nahi bolti.
        assert!(!same.contains("clear filters"));
        assert!(!same.contains("no results yet"));
        assert!(!idle.contains("identical"));
        // Legacy generic barkarar (dusri waves ke tests us par hain).
        assert!(
            folder_empty_hint(Some(FolderEmptyState::NoMatch)).contains("no rows match filters")
        );
    }

    #[test]
    fn empty_state_full_rows_routes_identical_vs_pruned_vs_never_compared() {
        let mixed = sample_rows();
        let total = mixed.len();
        let active = FolderDisplay {
            name_query: String::from("zzz-no-such-file"),
            ..FolderDisplay::default()
        };
        assert!(active.is_active());
        let calm = FolderDisplay::default();
        // Never-compared: model hi nahi — legacy line.
        assert_eq!(
            folder_empty_state_full_rows(false, &mixed, 0, &active),
            Some(FolderEmptyState::NoResults)
        );
        // Model hai par zero rows — legacy line.
        let no_rows: Vec<FolderRow> = Vec::new();
        assert_eq!(
            folder_empty_state_full_rows(true, &no_rows, 0, &calm),
            Some(FolderEmptyState::NoResults)
        );
        // Identical folders (sab '='): diff-seeking filter par ped khaali
        // to honest kaaran same-pan hai, prune nahi.
        let identical = vec![
            glyph_row("a.txt", false, '='),
            glyph_row("src", true, '='),
            glyph_row("src/b.txt", false, '='),
        ];
        let changed_only = FolderDisplay {
            status: FolderStatusFilter::Changed,
            ..FolderDisplay::default()
        };
        assert!(changed_only.is_active());
        assert!(folder_all_same(&identical));
        assert!(!folder_all_same(&mixed));
        assert!(!folder_all_same(&no_rows));
        assert_eq!(
            folder_empty_state_full_rows(true, &identical, 0, &changed_only),
            Some(FolderEmptyState::AllSame)
        );
        // Naam filter par bhi identical hi jeetta hai (clear karne par bhi
        // kuch karne ko nahi milega).
        assert_eq!(
            folder_empty_state_full_rows(true, &identical, 0, &active),
            Some(FolderEmptyState::AllSame)
        );
        // Mixed rows par wahi filter prune deta hai (legacy chain barkarar).
        assert_eq!(
            folder_empty_state_full_rows(true, &mixed, 0, &active),
            Some(FolderEmptyState::FilterPruned)
        );
        // Kuch dikh raha — khaali nahi (identical browse par bhi None).
        assert_eq!(
            folder_empty_state_full_rows(true, &identical, identical.len(), &calm),
            None
        );
        assert_eq!(
            folder_empty_state_full_rows(true, &mixed, total, &calm),
            None
        );
    }

    #[test]
    fn filter_bar_geo_typo_tokens_stay_on_scale() {
        // Geometry first: bar/pad/gap spacing scale par.
        assert_eq!(folder_filter_bar_height(), 24.0);
        assert_eq!(folder_filter_bar_height(), FOLDER_FILTER_BAR_HEIGHT);
        assert_eq!(folder_filter_bar_pad_x(), 8.0);
        assert_eq!(folder_filter_bar_gap(), 8.0);
        // Frozen tokens hilte nahi.
        assert_eq!(tree_indent_step(), 12.0);
        assert_eq!(folder_row_height(), 24.0);
        // Typography second: bar body-regular mono, chip caption-strong mono.
        let bar = folder_filter_typo();
        assert!(bar.mono);
        assert_eq!(bar.size, FOLDER_BODY_SIZE);
        assert!(!bar.strong);
        let chip = folder_filter_active_typo();
        assert!(chip.mono);
        assert_eq!(chip.size, FOLDER_SIDE_SIZE);
        assert!(chip.strong);
    }

    #[test]
    fn session_filter_helpers_wire_counts_and_hints() {
        let rows = sample_rows();
        let mut session = FolderSessionState::default();
        assert!(!session.has_active_filters());
        assert_eq!(session.active_filter_count(), 0);
        assert_eq!(session.empty_state(&rows), None);
        assert!(session.empty_hint(&rows).is_empty());
        // Naam filter: ginati filtered tree par.
        session.display.name_query = String::from("zzz-no-such-file");
        assert!(session.has_active_filters());
        assert_eq!(session.active_filter_count(), 1);
        assert_eq!(session.filtered_counts(&rows).total, 0);
        assert!(session.filtered_counts_summary(&rows).contains("0 entries"));
        assert_eq!(session.empty_state(&rows), Some(FolderEmptyState::NoMatch));
        assert!(session.empty_hint(&rows).contains("clear filters"));
        // Clear: sab wapas.
        session.clear_filters();
        assert!(!session.has_active_filters());
        assert_eq!(session.filtered_counts(&rows).total, rows.len());
        assert_eq!(session.empty_state(&rows), None);
        // Khali model: hamesha NoResults.
        let empty: Vec<FolderRow> = Vec::new();
        assert_eq!(
            session.empty_state(&empty),
            Some(FolderEmptyState::NoResults)
        );
        // Flatten + status bhi visible tree par lagte hain.
        session.display.status = FolderStatusFilter::LeftOnly;
        session.display.flatten = true;
        let flat = session.visible_tree(&rows);
        assert_eq!(flat.len(), 1);
        assert!(flat.iter().all(|node| node.depth == 0));
    }

    #[test]
    fn drill_down_scope_navigation_and_breadcrumbs() {
        let mut scope = FolderDrillDownScope::new();
        assert!(scope.is_root());
        assert_eq!(scope.depth(), 0);
        let root_crumbs = scope.breadcrumbs();
        assert_eq!(root_crumbs.len(), 1);
        assert_eq!(root_crumbs.first().map(|c| c.name.as_str()), Some("Root"));
        assert_eq!(
            root_crumbs.first().map(|c| c.target_path.as_str()),
            Some("")
        );
        assert_eq!(root_crumbs.first().map(|c| c.is_current), Some(true));

        scope.drill_into("src/gui/views");
        assert!(!scope.is_root());
        assert_eq!(scope.depth(), 3);
        let crumbs = scope.breadcrumbs();
        assert_eq!(crumbs.len(), 4);
        assert_eq!(crumbs.first().map(|c| c.name.as_str()), Some("Root"));
        assert_eq!(crumbs.first().map(|c| c.is_current), Some(false));
        assert_eq!(crumbs.get(1).map(|c| c.name.as_str()), Some("src"));
        assert_eq!(crumbs.get(1).map(|c| c.target_path.as_str()), Some("src"));
        assert_eq!(crumbs.get(1).map(|c| c.is_current), Some(false));
        assert_eq!(crumbs.get(2).map(|c| c.name.as_str()), Some("gui"));
        assert_eq!(
            crumbs.get(2).map(|c| c.target_path.as_str()),
            Some("src/gui")
        );
        assert_eq!(crumbs.get(2).map(|c| c.is_current), Some(false));
        assert_eq!(crumbs.get(3).map(|c| c.name.as_str()), Some("views"));
        assert_eq!(
            crumbs.get(3).map(|c| c.target_path.as_str()),
            Some("src/gui/views")
        );
        assert_eq!(crumbs.get(3).map(|c| c.is_current), Some(true));

        assert!(scope.drill_up());
        assert_eq!(scope.current_dir, "src/gui");
        assert_eq!(scope.depth(), 2);

        assert!(scope.drill_up());
        assert_eq!(scope.current_dir, "src");
        assert_eq!(scope.depth(), 1);

        assert!(scope.drill_up());
        assert!(scope.is_root());
        assert_eq!(scope.depth(), 0);

        assert!(!scope.drill_up()); // already at root

        scope.drill_into("test");
        assert_eq!(scope.depth(), 1);
        scope.reset();
        assert!(scope.is_root());
    }

    #[test]
    fn drill_down_scoped_folder_tree_filtering() {
        let rows = vec![
            FolderRow {
                rel_path: "root.txt".to_string(),
                is_dir: false,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            },
            FolderRow {
                rel_path: "src/a.txt".to_string(),
                is_dir: false,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            },
            FolderRow {
                rel_path: "src/gui/b.rs".to_string(),
                is_dir: false,
                glyph: '!',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: true,
            },
            FolderRow {
                rel_path: "docs/readme.md".to_string(),
                is_dir: false,
                glyph: 'L',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: true,
            },
        ];

        let mut scope = FolderDrillDownScope::new();
        let display = FolderDisplay::default();
        let mut expanded = BTreeSet::new();
        expanded.insert("src".to_string());
        expanded.insert("src/gui".to_string());
        expanded.insert("docs".to_string());

        // At root with expanded dirs, all 4 rows present
        let root_tree = folder_tree_scoped(&rows, &scope, &expanded, &display);
        assert_eq!(root_tree.len(), 4);

        // Scoped to "src", only 2 items from src, rel_paths stripped of "src/"
        // "a.txt" has depth 0 (relative root), "gui/b.rs" has parent "gui"
        scope.drill_into("src");
        let mut scoped_expanded = BTreeSet::new();
        scoped_expanded.insert("gui".to_string());
        let scoped_tree = folder_tree_scoped(&rows, &scope, &scoped_expanded, &display);
        assert_eq!(scoped_tree.len(), 2);
        assert_eq!(
            scoped_tree.first().map(|n| n.row.rel_path.as_str()),
            Some("a.txt")
        );
        assert_eq!(scoped_tree.first().map(|n| n.depth), Some(0));
        assert_eq!(
            scoped_tree.get(1).map(|n| n.row.rel_path.as_str()),
            Some("gui/b.rs")
        );
        assert_eq!(scoped_tree.get(1).map(|n| n.depth), Some(1));
    }

    #[test]
    fn drill_down_detect_tab_and_compare_intent() {
        assert_eq!(detect_tab_for_path("data.csv"), crate::home::Tab::Table);
        assert_eq!(detect_tab_for_path("sheet.xlsx"), crate::home::Tab::Table);
        assert_eq!(detect_tab_for_path("image.png"), crate::home::Tab::Picture);
        assert_eq!(detect_tab_for_path("photo.jpeg"), crate::home::Tab::Picture);
        assert_eq!(detect_tab_for_path("sound.wav"), crate::home::Tab::Audio);
        assert_eq!(detect_tab_for_path("music.mp3"), crate::home::Tab::Audio);
        assert_eq!(detect_tab_for_path("binary.bin"), crate::home::Tab::Hex);
        assert_eq!(detect_tab_for_path("native.dylib"), crate::home::Tab::Hex);
        assert_eq!(detect_tab_for_path("code.rs"), crate::home::Tab::Text);
        assert_eq!(detect_tab_for_path("README"), crate::home::Tab::Text);

        let file_node = FolderTreeNode {
            row: FolderRow {
                rel_path: "assets/logo.png".to_string(),
                is_dir: false,
                glyph: '!',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: true,
            },
            depth: 1,
            is_expanded: false,
            has_children: false,
        };
        let dir_node = FolderTreeNode {
            row: FolderRow {
                rel_path: "assets".to_string(),
                is_dir: true,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            },
            depth: 0,
            is_expanded: true,
            has_children: true,
        };

        assert!(drill_down_intent_for_node("/left", "/right", &dir_node).is_none());
        let intent = drill_down_intent_for_node("/left", "/right", &file_node);
        assert!(intent.is_some());
        if let Some(intent) = intent {
            assert_eq!(intent.rel_path, "assets/logo.png");
            assert_eq!(intent.target_tab, crate::home::Tab::Picture);
            assert_eq!(intent.left_path, "/left/assets/logo.png");
            assert_eq!(intent.right_path, "/right/assets/logo.png");
        }
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn drill_down_geometry_and_typography_contracts() {
        use crate::theme::{CHROME_HEADER_BAR_H, GEO_SECTION_GAP, GEO_TIGHT_GAP};
        assert_eq!(DRILL_DOWN_BAR_H, CHROME_HEADER_BAR_H);
        assert_eq!(drill_down_bar_height(), 28.0);
        assert_eq!(DRILL_DOWN_CRUMB_H, 24.0);
        assert_eq!(drill_down_crumb_height(), 24.0);
        assert_eq!(DRILL_DOWN_PAD_X, GEO_SECTION_GAP);
        assert_eq!(drill_down_pad_x(), 8.0);
        assert_eq!(DRILL_DOWN_GAP, GEO_TIGHT_GAP);
        assert_eq!(drill_down_gap(), 4.0);
        assert_eq!(DRILL_DOWN_SEP_W, 12.0);

        let hit_area = drill_down_crumb_hit_area(50.0);
        assert_eq!(hit_area, (50.0 + 16.0) * 24.0);

        let ancestor_typo = drill_down_crumb_ancestor_typo();
        assert!(ancestor_typo.mono);
        assert_eq!(ancestor_typo.size, FOLDER_BODY_SIZE);
        assert!(!ancestor_typo.strong);

        let current_typo = drill_down_crumb_current_typo();
        assert!(current_typo.mono);
        assert_eq!(current_typo.size, FOLDER_BODY_SIZE);
        assert!(current_typo.strong);

        let sep_typo = drill_down_crumb_separator_typo();
        assert!(sep_typo.mono);
        assert_eq!(sep_typo.size, FOLDER_SIDE_SIZE);
        assert!(!sep_typo.strong);

        let hint_typo = drill_down_hint_typo();
        assert!(hint_typo.mono);
        assert_eq!(hint_typo.size, FOLDER_SIDE_SIZE);
        assert!(!hint_typo.strong);
    }

    #[test]
    fn folder_session_state_drill_down_integration() {
        let mut session = FolderSessionState::default();
        assert!(session.drill_down.is_root());
        session.drill_into("src");
        assert_eq!(session.drill_down.current_dir, "src");
        assert!(session.drill_up());
        assert!(session.drill_down.is_root());
        session.drill_into("gui");
        session.reset_drill_down();
        assert!(session.drill_down.is_root());
    }
}

#[cfg(test)]
mod exq_tree2_tests {
    use super::*;

    #[test]
    fn tree2_guides_status_and_path_geometry() {
        // Mirrors checks/gui-exq-tree2/round-1/comparison/edge-deep-nest.json:
        // step 12 (MD), expander 16; guides har ancestor expander center
        // (level*12 + 8); status cell 16 + gap 8 ke baad separated path.
        assert_eq!(tree_status_mark_width(), 16.0);
        assert_eq!(tree_status_mark_width(), crate::theme::SPACE_LG);
        assert_eq!(tree_status_gap(), 8.0);
        assert_eq!(tree_status_gap(), crate::theme::SPACE_SM);
        assert_eq!(tree_status_mark_x(0), 16.0);
        assert_eq!(tree_status_mark_x(1), 28.0);
        assert_eq!(tree_status_mark_x(2), 40.0);
        assert_eq!(folder_row_path_x_separated(0), 40.0);
        assert_eq!(folder_row_path_x_separated(1), 52.0);
        assert_eq!(folder_row_path_x_separated(3), 76.0);
        // Legacy flat offset frozen rehta hai (paint adopt kare to separated).
        assert_eq!(folder_row_x_offset(1), 28.0);
        assert_eq!(tree_indent_guide_x(0), 8.0);
        assert_eq!(tree_indent_guide_x(1), 20.0);
        assert_eq!(tree_indent_guide_x(2), 32.0);
        assert!(tree_indent_guide_xs(0).is_empty());
        assert_eq!(tree_indent_guide_xs(1), vec![8.0]);
        assert_eq!(tree_indent_guide_xs(3), vec![8.0, 20.0, 32.0]);
    }

    #[test]
    fn tree2_status_typo_weight_separates_without_color() {
        // Status strong (path regular se weight se alag); sizes frozen.
        let status = tree_status_typo();
        let path = folder_typo_for_column(FolderColumn::Path);
        assert!(status.mono && status.strong);
        assert_eq!(status.size, FOLDER_BODY_SIZE);
        assert_eq!(status.size, path.size);
        assert!(!path.strong);
    }

    #[test]
    fn tree2_round2_indent_from_space_tokens() {
        assert_eq!(tree_indent_step(), crate::theme::GEO_TREE_INDENT_STEP);
        assert_eq!(tree_indent_step(), crate::theme::SPACE_MD);
        assert_eq!(tree_indent_step_from_space(), 12.0);
        assert_eq!(tree_indent_rhythm_px(0), 0.0);
        assert_eq!(tree_indent_rhythm_px(1), 12.0);
        assert_eq!(tree_indent_rhythm_px(2), 24.0);
        assert_eq!(tree_indent_rhythm_px(3), 36.0);
        assert_eq!(tree_indent_rhythm_px(4), 48.0);
        assert_eq!(tree_indent_rhythm_px(5), 60.0);

        assert_eq!(tree_row_path_x(0), 40.0);
        assert_eq!(tree_row_path_x(1), 52.0);
        assert_eq!(tree_row_path_x(2), 64.0);
        assert_eq!(tree_row_path_x(3), 76.0);
        assert_eq!(tree_row_path_x(4), 88.0);
        assert_eq!(tree_row_path_x(5), 100.0);
    }

    #[test]
    fn tree2_round2_orphan_dimming_via_typo_small() {
        let orphan_left = FolderRow {
            rel_path: "left_only.txt".to_string(),
            is_dir: false,
            glyph: 'L',
            left_size: Some("1024".to_string()),
            right_size: None,
            left_time: Some("2026-03-01 10:00".to_string()),
            right_time: None,
            changed: true,
        };
        let orphan_right = FolderRow {
            rel_path: "right_only.txt".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: Some("2048".to_string()),
            left_time: None,
            right_time: Some("2026-03-01 11:00".to_string()),
            changed: true,
        };

        // Left-only: right side is missing and dimmed
        assert!(!tree_cell_is_dimmed(&orphan_left, FolderSide::Left));
        assert!(tree_cell_is_dimmed(&orphan_left, FolderSide::Right));
        assert_eq!(
            tree_cell_typo_role(&orphan_left, FolderSide::Left),
            crate::theme::TypoRole::Mono
        );
        assert_eq!(
            tree_cell_typo_role(&orphan_left, FolderSide::Right),
            crate::theme::TypoRole::Small
        );
        assert!(tree_cell_typo_role(&orphan_left, FolderSide::Right).is_weak());
        assert!(!tree_cell_typo_role(&orphan_left, FolderSide::Left).is_weak());
        assert_eq!(
            tree_cell_typo_role(&orphan_left, FolderSide::Right).rank(),
            4
        );
        assert_eq!(
            tree_cell_typo_role(&orphan_left, FolderSide::Left).rank(),
            3
        );

        // Right-only: left side is missing and dimmed
        assert!(tree_cell_is_dimmed(&orphan_right, FolderSide::Left));
        assert!(!tree_cell_is_dimmed(&orphan_right, FolderSide::Right));
        assert_eq!(
            tree_cell_typo_role(&orphan_right, FolderSide::Left),
            crate::theme::TypoRole::Small
        );
        assert_eq!(
            tree_cell_typo_role(&orphan_right, FolderSide::Right),
            crate::theme::TypoRole::Mono
        );
        assert!(tree_cell_typo_role(&orphan_right, FolderSide::Left).is_weak());
        assert!(!tree_cell_typo_role(&orphan_right, FolderSide::Right).is_weak());
        assert_eq!(
            tree_cell_typo_role(&orphan_right, FolderSide::Left).rank(),
            4
        );
        assert_eq!(
            tree_cell_typo_role(&orphan_right, FolderSide::Right).rank(),
            3
        );

        // FolderTreeNode methods parity check
        let node_left = FolderTreeNode {
            row: orphan_left.clone(),
            depth: 2,
            is_expanded: false,
            has_children: false,
        };
        assert!(node_left.is_orphan());
        assert!(!node_left.is_missing_on_side(FolderSide::Left));
        assert!(node_left.is_missing_on_side(FolderSide::Right));
        assert!(!node_left.is_dimmed(FolderSide::Left));
        assert!(node_left.is_dimmed(FolderSide::Right));
        assert_eq!(
            node_left.cell_typo_role(FolderSide::Right),
            crate::theme::TypoRole::Small
        );
        assert!(node_left.cell_typo_role(FolderSide::Right).is_weak());
        assert_eq!(node_left.cell_typo_role(FolderSide::Right).rank(), 4);
        assert_eq!(node_left.indent_px(), 24.0);
        assert_eq!(node_left.path_x_separated(), 64.0);
        assert_eq!(node_left.indent_guide_xs(), vec![8.0, 20.0]);
    }

    #[test]
    fn tree2_round2_columns_aligned_at_960() {
        assert_eq!(folder_table_fixed_cols_w(), 222.0);
        let (name_x, size_x, mod_x) = folder_table_column_xs(960.0);
        assert_eq!(name_x, 0.0);
        assert_eq!(size_x, 738.0);
        assert_eq!(mod_x, 832.0);

        let (tn_x, ts_x, tm_x) = tree_wide_960_column_xs();
        assert_eq!(tn_x, name_x);
        assert_eq!(ts_x, size_x);
        assert_eq!(tm_x, mod_x);

        let size_min_w = folder_table_col_min_px(FolderTableCol::Size);
        let mod_min_w = folder_table_col_min_px(FolderTableCol::Modified);
        assert_eq!(size_min_w, 86.0);
        assert_eq!(mod_min_w, 128.0);
        assert_eq!(mod_x + mod_min_w, 960.0);
        assert_eq!(size_x + size_min_w + FOLDERTABLE_COL_GAP, mod_x);

        // Columns stay aligned across depths 0 to 5
        for depth in 0..=5 {
            let (nx, sx, mx) = tree_columns_aligned_at_960(depth);
            assert_eq!(nx, 0.0);
            assert_eq!(sx, 738.0);
            assert_eq!(mx, 832.0);
        }
    }

    #[test]
    fn exq_tree2_round2_tree_wide_960_and_orphan_dimming() {
        // Headless test proving edge fixture comparison/tree-wide-960.json
        let fixture_raw =
            include_str!("../../../checks/gui-exq-tree2/round-1/comparison/tree-wide-960.json");
        assert!(fixture_raw.contains("\"name\": \"tree-wide-960\""));
        assert!(fixture_raw.contains("\"width_px\": 960.0"));
        assert!(fixture_raw.contains("\"fixed_cols_total_w_px\": 222.0"));
        assert!(fixture_raw.contains("\"x_px\": 738.0"));
        assert!(fixture_raw.contains("\"x_px\": 832.0"));
        assert!(fixture_raw.contains("\"indent_step_px\": 12.0"));
        assert!(fixture_raw.contains("\"weak_typo_role\": \"Small\""));

        let (name_x, size_x, mod_x) = tree_wide_960_column_xs();
        assert_eq!(name_x, 0.0);
        assert_eq!(size_x, 738.0);
        assert_eq!(mod_x, 832.0);

        let expected_indents = [0.0, 12.0, 24.0, 36.0, 48.0, 60.0];
        let expected_paths = [40.0, 52.0, 64.0, 76.0, 88.0, 100.0];

        for (depth, (&exp_indent, &exp_path)) in expected_indents
            .iter()
            .zip(expected_paths.iter())
            .enumerate()
        {
            assert_eq!(tree_indent_rhythm_px(depth), exp_indent);
            assert_eq!(tree_row_path_x(depth), exp_path);
            assert_eq!(tree_indent_guide_xs(depth).len(), depth);
        }

        // Orphan dimming check
        let orphan_l = FolderRow {
            rel_path: "src/gui/orphan_left.rs".to_string(),
            is_dir: false,
            glyph: 'L',
            left_size: Some("2048".to_string()),
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        };
        let orphan_r = FolderRow {
            rel_path: "src/gui/orphan_right.rs".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: Some("4096".to_string()),
            left_time: None,
            right_time: None,
            changed: true,
        };

        assert_eq!(
            tree_cell_typo_role(&orphan_l, FolderSide::Right),
            crate::theme::TypoRole::Small
        );
        assert!(tree_cell_typo_role(&orphan_l, FolderSide::Right).is_weak());
        assert_eq!(
            folder_table_cell_text(&orphan_l, FolderSide::Right, FolderTableCol::Size),
            "-"
        );

        assert_eq!(
            tree_cell_typo_role(&orphan_r, FolderSide::Left),
            crate::theme::TypoRole::Small
        );
        assert!(tree_cell_typo_role(&orphan_r, FolderSide::Left).is_weak());
        assert_eq!(
            folder_table_cell_text(&orphan_r, FolderSide::Left, FolderTableCol::Size),
            "-"
        );
    }
}

// ----------------------------------------------------------------------------
// TREE4: folder grid columns + indent-guide span + gutter arrows (headless).
// BC pattern, outside observation only: size/time apne fixed grid columns
// me right-aligned block me padhte hain (count kabhi path se wrap nahi hota),
// indent guides poori row height span karti hain taaki levels juden, aur
// gutter me prev/next changed arrows hote hain. Geometry first: side block
// right-aligned (tang window par path-min ke baad left-stack fallback, paint
// scroll karta hai, crush nahi), guide span row height, arrow cell expander
// parity. Typography second: arrow typo header-strong reuse, koi naya size
// nahi. Colors frozen: yahan koi hue nahi.
// WIRING (paint): har visible node ke liye path
// `folder_row_path_x_separated(depth)` se `folder_path_avail_px` tak (truncate
// jab `folder_path_needs_truncate`), side cols `folder_grid_column_xs` se
// `folder_column_align` me; guides `tree_indent_guide_xs(depth)` par
// `tree_indent_guide_row_span(row_top)` tak; gutter arrows
// `folder_gutter_arrow_hit_rect(row_top, is_next)` par
// `folder_gutter_arrow_typo()` me.
// ----------------------------------------------------------------------------

/// Folder gutter prev/next arrow cell width px: expander parity (16.0 LG).
pub const FOLDER_GUTTER_ARROW_W: f32 = crate::theme::SPACE_LG;

/// Gutter arrow cell width px (single source). Pure; unit-tested.
pub fn folder_gutter_arrow_width() -> f32 {
    FOLDER_GUTTER_ARROW_W
}

/// Gutter arrow hit rect `(x0, y0, x1, y1)` px: prev arrow row ke upar aadhe
/// me, next arrow neeche aadhe me — ek 16px cell me stacked, poori row
/// height cover. Non-finite `row_top` par origin fallback (koi NaN nahi).
/// Pure; unit-tested.
pub fn folder_gutter_arrow_hit_rect(row_top: f32, is_next: bool) -> (f32, f32, f32, f32) {
    let top = if row_top.is_finite() { row_top } else { 0.0 };
    let half = FOLDER_ROW_HEIGHT / 2.0;
    if is_next {
        (
            0.0,
            top + half,
            FOLDER_GUTTER_ARROW_W,
            top + FOLDER_ROW_HEIGHT,
        )
    } else {
        (0.0, top, FOLDER_GUTTER_ARROW_W, top + half)
    }
}

/// Gutter arrow typo: header-strong reuse (caption-strong mono) — koi naya
/// size nahi, farak sirf weight se. Pure; unit-tested.
pub fn folder_gutter_arrow_typo() -> FolderTypo {
    folder_header_typo()
}

/// Side block width px: 4 fixed cols (L/R size + L/R time) + unke beech 3
/// gaps. Paint isi block ko right-align karta hai. Pure; unit-tested.
pub fn folder_grid_side_px() -> f32 {
    let size = folder_column_min_px(FolderColumn::LeftSize);
    let time = folder_column_min_px(FolderColumn::LeftTime);
    size + size + time + time + FOLDER_COL_GAP * 3.0
}

/// Side block ka x-origin px: right-aligned (`avail_w - side`), par path +
/// path-min se takraye to left-stack fallback (tang window scroll karti hai,
/// columns crush nahi hote). Non-finite `avail_w` par fallback. Pure.
fn folder_grid_side_x0(avail_w: f32, depth: usize) -> f32 {
    let floor = folder_row_path_x_separated(depth) + folder_column_min_px(FolderColumn::Path);
    if !avail_w.is_finite() {
        return floor;
    }
    let right = avail_w - folder_grid_side_px();
    if right < floor { floor } else { right }
}

/// Grid column x-origins px `(path, lsize, rsize, ltime, rtime)`: path
/// separated offset se, side block tile hota hai (har agli = pichhli + min +
/// gap, bina overlap). Paint har cell isi x se `min_px` width me karta hai.
/// Pure; unit-tested.
pub fn folder_grid_column_xs(avail_w: f32, depth: usize) -> (f32, f32, f32, f32, f32) {
    let path_x = folder_row_path_x_separated(depth);
    let size = folder_column_min_px(FolderColumn::LeftSize);
    let time = folder_column_min_px(FolderColumn::LeftTime);
    let side_x0 = folder_grid_side_x0(avail_w, depth);
    let rsize_x = side_x0 + size + FOLDER_COL_GAP;
    let ltime_x = rsize_x + size + FOLDER_COL_GAP;
    let rtime_x = ltime_x + time + FOLDER_COL_GAP;
    (path_x, side_x0, rsize_x, ltime_x, rtime_x)
}

/// Path cell ki avail width px: side block se pehle bachi jagah (min 0).
/// Non-finite budget par 0. Pure; unit-tested.
pub fn folder_path_avail_px(avail_w: f32, depth: usize) -> f32 {
    if !avail_w.is_finite() {
        return 0.0;
    }
    (folder_grid_side_x0(avail_w, depth) - folder_row_path_x_separated(depth)).max(0.0)
}

/// Path truncate faisla: `path_chars` mono advance me avail se bahar.
/// Khali path / non-finite budget par kabhi truncate nahi. Pure.
pub fn folder_path_needs_truncate(avail_w: f32, depth: usize, path_chars: usize) -> bool {
    if path_chars == 0 || !avail_w.is_finite() {
        return false;
    }
    path_chars as f32 * FOLDER_MONO_ADVANCE > folder_path_avail_px(avail_w, depth)
}

/// Ek row par indent guides ka vertical span `(y0, y1)` px: poori row height,
/// taaki guides upar-neeche rows se juden. Pure; unit-tested.
pub fn tree_indent_guide_row_span(row_top: f32) -> (f32, f32) {
    let top = if row_top.is_finite() { row_top } else { 0.0 };
    (top, top + FOLDER_ROW_HEIGHT)
}

#[cfg(test)]
mod exq_tree4_tests {
    use super::*;

    fn sized_row(path: &str, glyph: char) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir: false,
            glyph,
            left_size: Some("2048".to_string()),
            left_time: Some("2026-01-02 03:04".to_string()),
            right_size: None,
            right_time: None,
            changed: glyph != '=',
        }
    }

    #[test]
    fn tree4_gutter_arrows_stack_full_row_height() {
        // Cell expander parity (16 LG); prev upar aadha, next neeche aadha.
        assert_eq!(folder_gutter_arrow_width(), 16.0);
        assert_eq!(folder_gutter_arrow_width(), crate::theme::SPACE_LG);
        assert_eq!(folder_gutter_arrow_width(), folder_expander_width());
        assert_eq!(
            folder_gutter_arrow_hit_rect(0.0, false),
            (0.0, 0.0, 16.0, 12.0)
        );
        assert_eq!(
            folder_gutter_arrow_hit_rect(0.0, true),
            (0.0, 12.0, 16.0, 24.0)
        );
        assert_eq!(
            folder_gutter_arrow_hit_rect(24.0, true),
            (0.0, 36.0, 16.0, 48.0)
        );
        // Dono halves milkar poori row (24): koi dead tap nahi.
        let prev = folder_gutter_arrow_hit_rect(48.0, false);
        let next = folder_gutter_arrow_hit_rect(48.0, true);
        assert_eq!(prev.3, next.1);
        assert_eq!(next.3 - prev.1, FOLDER_ROW_HEIGHT);
        // NaN guard: origin fallback, kabhi NaN rect nahi.
        let nan_prev = folder_gutter_arrow_hit_rect(f32::NAN, false);
        assert!(nan_prev.0.is_finite() && nan_prev.1.is_finite());
        // Typo second: header-strong reuse, koi naya size nahi.
        let arrow = folder_gutter_arrow_typo();
        let header = folder_header_typo();
        assert_eq!(arrow.size, header.size);
        assert_eq!(arrow.size, FOLDER_HEADER_SIZE);
        assert!(arrow.mono && arrow.strong);
    }

    #[test]
    fn tree4_grid_side_block_right_aligns_at_960() {
        // Side: 86+86+128+128 + 3x8 = 452; 960 par x0 = 508.
        assert_eq!(folder_grid_side_px(), 452.0);
        let (path, lsize, rsize, ltime, rtime) = folder_grid_column_xs(960.0, 0);
        assert_eq!(path, 40.0);
        assert_eq!((lsize, rsize, ltime, rtime), (508.0, 602.0, 696.0, 832.0));
        // Tile bina overlap: har agli = pichhli + min + gap; aakhri edge par.
        let size = folder_column_min_px(FolderColumn::LeftSize);
        let time = folder_column_min_px(FolderColumn::LeftTime);
        assert_eq!(rsize, lsize + size + FOLDER_COL_GAP);
        assert_eq!(ltime, rsize + size + FOLDER_COL_GAP);
        assert_eq!(rtime, ltime + time + FOLDER_COL_GAP);
        assert_eq!(rtime + time, 960.0);
        // Depth ke saath path aage sarakta hai, side block sthir rehta hai.
        let (deep_path, deep_lsize, _, _, _) = folder_grid_column_xs(960.0, 3);
        assert_eq!(deep_path, 76.0);
        assert_eq!(deep_lsize, lsize);
        // Path avail: 508 - 40 = 468; chhota naam fit, lamba truncate.
        assert_eq!(folder_path_avail_px(960.0, 0), 468.0);
        assert!(!folder_path_needs_truncate(960.0, 0, 9));
        assert!(folder_path_needs_truncate(960.0, 0, 67));
        assert!(!folder_path_needs_truncate(960.0, 0, 0));
        assert!(!folder_path_needs_truncate(f32::NAN, 0, 67));
        assert_eq!(folder_path_avail_px(f32::NAN, 0), 0.0);
    }

    #[test]
    fn tree4_narrow_window_stacks_without_crush() {
        // 400px: right-align takrayega (400-452 < 40+184), left-stack 224 par.
        let (path, lsize, rsize, ltime, rtime) = folder_grid_column_xs(400.0, 0);
        assert_eq!(path, 40.0);
        assert_eq!((lsize, rsize, ltime, rtime), (224.0, 318.0, 412.0, 548.0));
        // Columns kabhi crush nahi: min widths barkarar, paint scroll karta hai.
        assert_eq!(
            rsize - lsize,
            folder_column_min_px(FolderColumn::LeftSize) + FOLDER_COL_GAP
        );
        assert_eq!(folder_path_avail_px(400.0, 0), 184.0);
        assert!(folder_path_needs_truncate(400.0, 0, 27));
        assert!(!folder_path_needs_truncate(400.0, 0, 26));
        // Non-finite budget bhi stacked fallback, NaN-free.
        let stacked = folder_grid_column_xs(f32::NAN, 1);
        assert!(stacked.0.is_finite() && stacked.1.is_finite());
        assert_eq!(stacked.0, folder_row_path_x_separated(1));
    }

    #[test]
    fn tree4_guide_span_covers_full_row_height() {
        assert_eq!(tree_indent_guide_row_span(0.0), (0.0, 24.0));
        assert_eq!(tree_indent_guide_row_span(48.0), (48.0, 72.0));
        assert_eq!(tree_indent_guide_row_span(f32::NAN), (0.0, 24.0));
        // Span row height ke barabar: guides rows ke beech tootti nahi.
        let span = tree_indent_guide_row_span(24.0);
        assert_eq!(span.1 - span.0, folder_row_height());
    }

    #[test]
    fn tree4_edge_deep_nesting_plus_orphan_newer() {
        // Fixture checks/gui-exq-tree4/round-1/comparison/edge-deep-nesting-orphan-newer.json:
        // 5-deep chain + orphan-newer (L, left sides set, right dashes).
        let chain = [
            "a",
            "a/d1",
            "a/d1/d2",
            "a/d1/d2/d3",
            "a/d1/d2/d3/d4",
            "a/d1/d2/d3/d4/deep.txt",
        ];
        for (depth, rel) in chain.iter().enumerate() {
            assert_eq!(tree_indent_guide_xs(depth).len(), depth);
            assert_eq!(
                folder_row_path_x_separated(depth),
                depth as f32 * tree_indent_step() + 40.0
            );
            let _ = rel;
        }
        // Sabse gehre level par 5 guides; aakhri guide level-4 expander center.
        assert_eq!(tree_indent_guide_xs(5), vec![8.0, 20.0, 32.0, 44.0, 56.0]);
        // Orphan-newer: right sides dash par padded widths barkarar (align).
        let orphan = sized_row("orphan-newer.txt", 'L');
        assert_eq!(folder_cell_text(&orphan, FolderColumn::RightSize), "-");
        assert_eq!(folder_cell_text(&orphan, FolderColumn::RightTime), "-");
        assert_eq!(
            pad_folder_cell(
                &folder_cell_text(&orphan, FolderColumn::RightSize),
                FOLDER_SIZE_CHARS
            )
            .chars()
            .count(),
            FOLDER_SIZE_CHARS
        );
        // Orphan aur normal row ke grid xs ek jaise: columns kabhi hilte nahi.
        // Gehre depth (5) par bhi side block sthir, sirf path aage sarakta hai.
        let (deep_path, deep_lsize, _, _, deep_rtime) = folder_grid_column_xs(960.0, 5);
        assert_eq!(deep_path, 100.0);
        assert_eq!(deep_lsize, 508.0);
        assert_eq!(deep_rtime, 832.0);
        let both = FolderRow {
            right_size: Some("4096".to_string()),
            right_time: Some("2026-01-03 04:05".to_string()),
            ..sized_row("both.txt", '=')
        };
        assert_eq!(
            dual_right_cell(&orphan).chars().count(),
            dual_right_cell(&both).chars().count()
        );
    }
}

// ----------------------------------------------------------------------------
// TREE5: drill-down scope + filter combined (headless).
// BC pattern, outside observation only: drilled folder me filter lagane par
// bhi expand/filter ek saath kaam karte hain — global expanded set scope ke
// andar bhi nested dirs kholta hai, aur breadcrumb + filter bar + header ka
// stacked chrome rows ko dhakta nahi (tang window scroll karti hai, crush
// nahi). Geometry first: scope-relative expanded mapping, stacked chrome
// (drill bar + filter row + header + tight gaps), scoped visible rows /
// content / scroll. Typography second: koi naya size nahi — bar/crumb/header
// typos reuse. Colors frozen: yahan koi hue nahi.
// WIRING (paint): scoped view `folder_tree_scoped_global` se (global
// expanded seedha do, mapping andar hota hai); chrome
// `folder_scoped_chrome_height(wrapped)` se; rows
// `folder_scoped_visible_rows` se; guides/grid/arrow helpers scope-relative
// depth par waise hi lagte hain (TREE2/TREE4 tokens reuse).
// ----------------------------------------------------------------------------

/// Global expanded set ko scope-relative karo: scope ke andar wali dirs ka
/// prefix utarta hai, scope-bahar aur scope-khud girte hain (scoped view me
/// unki row hoti nahi). Root par clone (legacy `folder_tree_scoped` parity).
/// Pure; unit-tested.
pub fn folder_scope_relative_expanded(
    expanded: &BTreeSet<String>,
    scope: &FolderDrillDownScope,
) -> BTreeSet<String> {
    if scope.is_root() {
        return expanded.clone();
    }
    let prefix = format!("{}/", scope.current_dir.trim().trim_matches('/'));
    let mut out = BTreeSet::new();
    for dir in expanded {
        if let Some(rel) = dir.strip_prefix(prefix.as_str())
            && !rel.trim().is_empty()
        {
            out.insert(rel.to_string());
        }
    }
    out
}

/// Scoped tree jo GLOBAL expanded set leta hai: mapping andar hota hai,
/// phir legacy [`folder_tree_scoped`] (frozen) chalti hai. Root par legacy
/// ke barabar. `FolderSessionState::scoped_visible_tree` isi se banta hai.
/// Pure; unit-tested.
pub fn folder_tree_scoped_global(
    rows: &[FolderRow],
    scope: &FolderDrillDownScope,
    expanded_dirs: &BTreeSet<String>,
    display: &FolderDisplay,
) -> Vec<FolderTreeNode> {
    let relative = folder_scope_relative_expanded(expanded_dirs, scope);
    folder_tree_scoped(rows, scope, &relative, display)
}

/// Scoped view ka chrome px: drill bar + gap + filter row + gap + header +
/// gap (3 tight gaps, dual chrome parity). Drill bar root par bhi dikhta hai
/// (Root crumb), isliye scope se azad. Paint isi ke baad rows lagata hai.
/// Pure; headless-tested.
pub fn folder_scoped_chrome_height(filter_wrapped: bool) -> f32 {
    DRILL_DOWN_BAR_H
        + crate::theme::GEO_TIGHT_GAP
        + folder_filter_row_height(filter_wrapped)
        + crate::theme::GEO_TIGHT_GAP
        + FOLDER_HEADER_HEIGHT
        + crate::theme::GEO_TIGHT_GAP
}

/// Scoped view me `avail_h` px me fit poori rows: chrome ke baad bachi jagah
/// me kitne row-height rows. Tang/khali/non-finite budget par 0. Pure.
pub fn folder_scoped_visible_rows(avail_h: f32, filter_wrapped: bool) -> usize {
    if !avail_h.is_finite() {
        return 0;
    }
    let inner = avail_h - folder_scoped_chrome_height(filter_wrapped);
    if inner < FOLDER_ROW_HEIGHT {
        return 0;
    }
    (inner / FOLDER_ROW_HEIGHT) as usize
}

/// Scoped content height px: chrome + visible rows (khaali ped par bhi
/// drill bar + filter + header ka frame rehta hai). Pure; headless-tested.
pub fn folder_scoped_content_height(visible_count: usize, filter_wrapped: bool) -> f32 {
    folder_scoped_chrome_height(filter_wrapped) + visible_count as f32 * FOLDER_ROW_HEIGHT
}

/// Scoped view ko scroll window chahiye: content budget se bahar. Non-finite
/// par false. Pure; headless-tested.
pub fn folder_scoped_needs_scroll(
    visible_count: usize,
    avail_h: f32,
    filter_wrapped: bool,
) -> bool {
    if !avail_h.is_finite() {
        return false;
    }
    folder_scoped_content_height(visible_count, filter_wrapped) > avail_h
}

/// Scoped height waste px: content ke baad viewport me bachi jagah. Overflow
/// par 0 (scroll), non-finite par 0. Pure; headless-tested.
pub fn folder_scoped_height_waste(avail_h: f32, visible_count: usize, filter_wrapped: bool) -> f32 {
    if !avail_h.is_finite() {
        return 0.0;
    }
    (avail_h - folder_scoped_content_height(visible_count, filter_wrapped)).max(0.0)
}

#[cfg(test)]
mod exq_tree5_tests {
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

    fn sized_row(path: &str, glyph: char) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir: false,
            glyph,
            left_size: Some("2048".to_string()),
            left_time: Some("2026-01-02 03:04".to_string()),
            right_size: None,
            right_time: None,
            changed: glyph != '=',
        }
    }

    fn deep_rows() -> Vec<FolderRow> {
        vec![
            glyph_row("a", true, '='),
            glyph_row("a/d1", true, '='),
            glyph_row("a/d1/d2", true, '='),
            glyph_row("a/d1/d2/d3", true, '='),
            glyph_row("a/d1/d2/d3/d4", true, '='),
            sized_row("a/d1/d2/d3/d4/deep.txt", '>'),
            sized_row("orphan-newer.txt", 'L'),
        ]
    }

    #[test]
    fn tree5_relative_expanded_maps_global_to_scope() {
        let mut global = BTreeSet::new();
        global.insert("a".to_string());
        global.insert("a/d1".to_string());
        global.insert("a/d1/d2".to_string());
        global.insert("docs".to_string());
        // Root par clone: mapping kuch nahi badalta.
        let root = FolderDrillDownScope::new();
        assert_eq!(folder_scope_relative_expanded(&global, &root), global);
        // Scope "a/d1": andar wali dirs prefix utarkar, bahar/scope-khud girte hain.
        let mut scope = FolderDrillDownScope::new();
        scope.drill_into("a/d1");
        assert_eq!(
            folder_scope_relative_expanded(&global, &scope),
            BTreeSet::from(["d2".to_string()])
        );
        // Khali set par khali mapping, kabhi panic nahi.
        assert!(folder_scope_relative_expanded(&BTreeSet::new(), &scope).is_empty());
    }

    #[test]
    fn tree5_global_scoped_tree_opens_nested_with_global_set() {
        let rows = deep_rows();
        let display = FolderDisplay::default();
        let mut global = BTreeSet::new();
        global.insert("a".to_string());
        global.insert("a/d1".to_string());
        global.insert("a/d1/d2".to_string());
        global.insert("a/d1/d2/d3".to_string());
        global.insert("a/d1/d2/d3/d4".to_string());
        // Scope "a/d1/d2": 3 scoped rows (d3, d3/d4, d3/d4/deep.txt), depth relative.
        let mut scope = FolderDrillDownScope::new();
        scope.drill_into("a/d1/d2");
        let tree = folder_tree_scoped_global(&rows, &scope, &global, &display);
        assert_eq!(tree.len(), 3);
        assert_eq!(tree.first().map(|n| n.row.rel_path.as_str()), Some("d3"));
        assert_eq!(tree.first().map(|n| n.depth), Some(0));
        assert_eq!(
            tree.get(2).map(|n| n.row.rel_path.as_str()),
            Some("d3/d4/deep.txt")
        );
        assert_eq!(tree.get(2).map(|n| n.depth), Some(2));
        // Session wiring: global expanded seedha scoped view kholta hai.
        let mut session = FolderSessionState {
            expanded: global,
            ..FolderSessionState::default()
        };
        session.drill_into("a/d1/d2");
        let scoped = session.scoped_visible_tree(&rows);
        assert_eq!(scoped.len(), 3);
        // Root par legacy barabar: orphan-newer dikhta hai.
        session.reset_drill_down();
        assert_eq!(session.scoped_visible_tree(&rows).len(), 7);
    }

    #[test]
    fn tree5_scoped_filter_keeps_orphan_newer_honest() {
        // Fixture checks/gui-exq-tree5/round-1/comparison/edge-deep-nesting-orphan-newer.json:
        // scope "a/d1/d2" + Changed filter — orphan scope-bahar, same scope-andar nahi.
        let rows = deep_rows();
        let mut scope = FolderDrillDownScope::new();
        scope.drill_into("a/d1/d2");
        let mut global = BTreeSet::new();
        global.insert("a".to_string());
        global.insert("a/d1".to_string());
        global.insert("a/d1/d2".to_string());
        global.insert("a/d1/d2/d3".to_string());
        global.insert("a/d1/d2/d3/d4".to_string());
        let changed = FolderDisplay {
            status: FolderStatusFilter::Changed,
            ..FolderDisplay::default()
        };
        let tree = folder_tree_scoped_global(&rows, &scope, &global, &changed);
        // Sirf deep.txt (glyph '>'): orphan-newer scope-bahar, dirs '=' chhan jaate hain.
        assert_eq!(tree.len(), 1);
        assert_eq!(
            tree.first().map(|n| n.row.rel_path.as_str()),
            Some("d3/d4/deep.txt")
        );
        // Naam filter scope-relative paths par: "deep" milta hai, "orphan" scope-bahar.
        let named = FolderDisplay {
            name_query: String::from("deep"),
            ..FolderDisplay::default()
        };
        let tree = folder_tree_scoped_global(&rows, &scope, &global, &named);
        assert_eq!(tree.len(), 1);
        let missing = FolderDisplay {
            name_query: String::from("orphan"),
            ..FolderDisplay::default()
        };
        assert!(folder_tree_scoped_global(&rows, &scope, &global, &missing).is_empty());
        // Orphan-newer ke missing sides dash par padded widths barkarar (grid align).
        let orphan = sized_row("orphan-newer.txt", 'L');
        assert_eq!(folder_cell_text(&orphan, FolderColumn::RightSize), "-");
        assert_eq!(
            pad_folder_cell(
                &folder_cell_text(&orphan, FolderColumn::RightTime),
                FOLDER_TIME_CHARS
            )
            .chars()
            .count(),
            FOLDER_TIME_CHARS
        );
    }

    #[test]
    fn tree5_scoped_chrome_stacks_without_overlap() {
        // Chrome: 28 + 4 + 24 + 4 + 24 + 4 = 88; wrapped 56 par 120.
        assert_eq!(folder_scoped_chrome_height(false), 88.0);
        assert_eq!(folder_scoped_chrome_height(true), 120.0);
        // 200px budget, unwrapped: (200 - 88) / 24 = 4 poori rows.
        assert_eq!(folder_scoped_visible_rows(200.0, false), 4);
        assert_eq!(folder_scoped_visible_rows(80.0, false), 0);
        assert_eq!(folder_scoped_visible_rows(f32::NAN, false), 0);
        // Content: chrome + rows; khaali ped par chrome-only frame rehta hai.
        assert_eq!(folder_scoped_content_height(0, false), 88.0);
        assert_eq!(folder_scoped_content_height(4, false), 184.0);
        // 184 <= 200: scroll nahi, waste 16; 5 rows par scroll, waste 0.
        assert!(!folder_scoped_needs_scroll(4, 200.0, false));
        assert_eq!(folder_scoped_height_waste(200.0, 4, false), 16.0);
        assert!(folder_scoped_needs_scroll(5, 200.0, false));
        assert_eq!(folder_scoped_height_waste(200.0, 5, false), 0.0);
        assert!(!folder_scoped_needs_scroll(4, f32::NAN, false));
        assert_eq!(folder_scoped_height_waste(f32::NAN, 4, false), 0.0);
    }

    #[test]
    fn tree5_scoped_geometry_reuses_tree2_tree4_tokens() {
        // Guides scope-relative depth par: count == depth, path x monotonic.
        assert_eq!(tree_indent_guide_xs(0).len(), 0);
        assert_eq!(tree_indent_guide_xs(2), vec![8.0, 20.0]);
        assert_eq!(folder_row_path_x_separated(0), 40.0);
        assert_eq!(folder_row_path_x_separated(2), 64.0);
        // Grid side block scoped depth par bhi sthir (960): path aage, side sthir.
        let (_, lsize_shallow, _, _, _) = folder_grid_column_xs(960.0, 0);
        let (_, lsize_scoped, _, _, rtime_scoped) = folder_grid_column_xs(960.0, 2);
        assert_eq!(lsize_shallow, lsize_scoped);
        assert_eq!(rtime_scoped, 832.0);
        // Gutter arrows + expander hit poori row (24): drill scope me bhi.
        assert_eq!(
            folder_gutter_arrow_hit_rect(48.0, false),
            (0.0, 48.0, 16.0, 60.0)
        );
        assert_eq!(folder_expander_hit_rect(2, 24.0), (24.0, 24.0, 40.0, 48.0));
        // Typo second: koi naya size nahi — crumb/body + header reuse.
        assert_eq!(
            drill_down_crumb_current_typo().size,
            folder_typo_for_column(FolderColumn::Path).size
        );
        assert_eq!(folder_gutter_arrow_typo().size, folder_header_typo().size);
    }

    #[test]
    fn tree_density_row_height_matches_viewmodel_tokens() {
        assert_eq!(
            folder_row_height_for_density(crate::viewmodel::RowDensity::Compact),
            crate::viewmodel::FOLDER_ROW_HEIGHT_COMPACT
        );
        assert_eq!(
            folder_row_height_for_density(crate::viewmodel::RowDensity::Comfortable),
            FOLDER_ROW_HEIGHT
        );
        assert_eq!(
            folder_row_height_for_density(crate::viewmodel::RowDensity::Spacious),
            crate::viewmodel::FOLDER_ROW_HEIGHT_SPACIOUS
        );
    }
}

// ----------------------------------------------------------------------------
// FOLDERTABLE10: Folder table column Grid + depth indent + TypoRole row height (headless).
// BC parity gap: folder drill-down rows were previously painted as full-width
// harsh bands, without separate Size/Date columns, with minimal depth indent,
// and without dimmed orphan rows.
// BC folder compare pattern:
// 1. Column Grid with 3 columns: Name, Size, Modified.
// 2. Column gap: SECTION_GAP (8.0px = crate::theme::GEO_SECTION_GAP = SPACE_SM).
// 3. Row height: derived from TypoRole line-height (TypoRole::Mono.line_height_px() = 15.6px + vertical padding = 17.6px, or density-aware).
// 4. Name column is indented per depth (folder_table_name_indent_px(depth)),
//    while Size and Modified columns align at FIXED column origins across all depths.
// 5. Orphan rows are dimmed on their missing side without editing colors (colors frozen).
// ----------------------------------------------------------------------------

/// Folder table column in standard 3-column view: Name, Size, Modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderTableCol {
    /// Relative filename or directory name (indented by depth, flex width).
    Name,
    /// Size in bytes (fixed width, right-aligned, "-" if absent or dir).
    Size,
    /// Last modified timestamp (fixed width, left-aligned, "-" if absent).
    Modified,
}

/// Standard 3-column table definition: Name, Size, Modified.
pub const FOLDER_TABLE_COLS: [FolderTableCol; 3] = [
    FolderTableCol::Name,
    FolderTableCol::Size,
    FolderTableCol::Modified,
];

/// Folder table column Grid gap (px): SECTION_GAP parity (8.0px = crate::theme::GEO_SECTION_GAP = SPACE_SM).
pub const FOLDERTABLE_COL_GAP: f32 = crate::theme::GEO_SECTION_GAP;

/// Folder table cell horizontal padding (px): SPACE_SM parity (8.0px = crate::theme::SPACE_SM).
pub const FOLDERTABLE_CELL_PAD_X: f32 = crate::theme::SPACE_SM;

/// Folder table row vertical padding (px): symmetric 1.0px top + bottom.
pub const FOLDERTABLE_ROW_PAD_Y: f32 = 1.0;

/// Column header labels in paint order: "Name", "Size", "Modified".
pub const FOLDERTABLE_HEADERS: [&str; 3] = ["Name", "Size", "Modified"];

/// Primary TypoRole for folder table content: Mono (12.0pt, tabular, content role).
pub const FOLDERTABLE_ROW_ROLE: crate::theme::TypoRole = crate::theme::TypoRole::Mono;

/// Column header font weight name: `"strong"`.
pub const fn folder_table_header_weight_name() -> &'static str {
    "strong"
}

/// Column header font weight numeric value: 600 (`TYPO_WEIGHT_SEMIBOLD`).
pub const fn folder_table_header_weight_value() -> u16 {
    crate::theme::TYPO_WEIGHT_SEMIBOLD
}

/// Returns the column header string for a given table column.
pub const fn folder_table_col_header(col: FolderTableCol) -> &'static str {
    match col {
        FolderTableCol::Name => "Name",
        FolderTableCol::Size => "Size",
        FolderTableCol::Modified => "Modified",
    }
}

/// Returns the 3 table column headers in order.
pub fn folder_table_headers() -> [&'static str; 3] {
    FOLDERTABLE_HEADERS
}

/// Verifies that the table column grid uses SECTION_GAP.
pub const fn folder_table_grid_has_section_gap() -> bool {
    FOLDERTABLE_COL_GAP == crate::theme::GEO_SECTION_GAP
}

/// Verifies that table cell padding strictly equals SPACE_SM (8.0px).
pub const fn folder_table_cell_has_space_padding() -> bool {
    FOLDERTABLE_CELL_PAD_X == crate::theme::SPACE_SM
}

/// Returns horizontal alignment for each folder table column:
/// Name: Left, Size: Right (numeric digits align), Modified: Left.
pub const fn folder_table_col_align(col: FolderTableCol) -> FolderCellAlign {
    match col {
        FolderTableCol::Name => FolderCellAlign::Left,
        FolderTableCol::Size => FolderCellAlign::Right,
        FolderTableCol::Modified => FolderCellAlign::Left,
    }
}

/// Content line height in px derived directly from TypoRole: 12.0 * 1.30 = 15.6px.
pub fn folder_table_line_height_px() -> f32 {
    FOLDERTABLE_ROW_ROLE.line_height_px()
}

/// Computes row height from line height with symmetric vertical padding (+ 2.0px).
/// Pure, non-finite guard.
pub fn folder_table_row_height_for_line(line_h: f32) -> f32 {
    if !line_h.is_finite() || line_h <= 0.0 {
        return FOLDER_ROW_HEIGHT;
    }
    line_h + 2.0 * FOLDERTABLE_ROW_PAD_Y
}

/// Base folder table row height (px) derived directly from TypoRole line-height:
/// 15.6 + 2.0 = 17.6px. Reuses [`folder_table_row_height_for_line`].
pub fn folder_table_row_height() -> f32 {
    folder_table_row_height_for_line(folder_table_line_height_px())
}

/// Density-aware folder table row height (px).
pub fn folder_table_density_row_height(density: crate::viewmodel::FolderDensity) -> f32 {
    folder_row_height_for_density(density)
}

/// Minimum width (px) for a table column.
pub fn folder_table_col_min_px(col: FolderTableCol) -> f32 {
    let chars: f32 = match col {
        FolderTableCol::Name => FOLDER_PATH_MIN_CHARS as f32,
        FolderTableCol::Size => FOLDER_SIZE_CHARS as f32,
        FolderTableCol::Modified => FOLDER_TIME_CHARS as f32,
    };
    chars * FOLDER_MONO_ADVANCE + FOLDERTABLE_CELL_PAD_X * 2.0
}

/// Total width of the fixed trailing columns (Size + gap + Modified):
/// 86.0 + 8.0 + 128.0 = 222.0px.
pub fn folder_table_fixed_cols_w() -> f32 {
    folder_table_col_min_px(FolderTableCol::Size)
        + FOLDERTABLE_COL_GAP
        + folder_table_col_min_px(FolderTableCol::Modified)
}

/// Available width for the Name column given pane available width `avail_w`.
/// Budget = avail_w - fixed_cols_w - FOLDERTABLE_COL_GAP.
/// Clamped to at least min_px(Name) = 184.0px.
pub fn folder_table_name_avail_w(avail_w: f32) -> f32 {
    let floor = folder_table_col_min_px(FolderTableCol::Name);
    if !avail_w.is_finite() {
        return floor;
    }
    let rem = avail_w - folder_table_fixed_cols_w() - FOLDERTABLE_COL_GAP;
    if rem < floor { floor } else { rem }
}

/// Exact column x-origins in a 3-column table Grid `(name_x, size_x, mod_x)`.
/// Name starts at origin (0.0).
/// Size starts at `name_w + FOLDERTABLE_COL_GAP`.
/// Modified starts at `size_x + size_w + FOLDERTABLE_COL_GAP`.
/// Note: Size and Modified columns align at FIXED x across all depths!
pub fn folder_table_column_xs(avail_w: f32) -> (f32, f32, f32) {
    let name_w = folder_table_name_avail_w(avail_w);
    let size_w = folder_table_col_min_px(FolderTableCol::Size);
    let name_x = 0.0;
    let size_x = name_w + FOLDERTABLE_COL_GAP;
    let mod_x = size_x + size_w + FOLDERTABLE_COL_GAP;
    (name_x, size_x, mod_x)
}

/// Name column content horizontal offset (px) for a given tree depth:
/// depth * tree_indent_step() + expander cell + status mark + gap.
/// Matches [`folder_row_path_x_separated(depth)`].
pub fn folder_table_name_indent_px(depth: usize) -> f32 {
    folder_row_path_x_separated(depth)
}

/// Whether the Name column content fits without truncation at a given depth and character count.
pub fn folder_table_name_needs_truncate(avail_w: f32, depth: usize, name_chars: usize) -> bool {
    if name_chars == 0 || !avail_w.is_finite() {
        return false;
    }
    let name_col_w = folder_table_name_avail_w(avail_w);
    let indent = folder_table_name_indent_px(depth);
    let text_w = name_chars as f32 * FOLDER_MONO_ADVANCE;
    indent + text_w > name_col_w
}

/// Side indicator for folder compare: Left pane or Right pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderSide {
    Left,
    Right,
}

/// Returns whether a folder row is an orphan (exists on only one side).
/// Glyph 'L' means exists on Left only; glyph 'R' means exists on Right only.
pub fn folder_row_is_orphan(row: &FolderRow) -> bool {
    row.glyph == 'L' || row.glyph == 'R'
}

/// Returns whether a row is missing on the specified side.
/// If Left: glyph 'R' means missing on Left.
/// If Right: glyph 'L' means missing on Right.
pub fn folder_row_is_missing_on_side(row: &FolderRow, side: FolderSide) -> bool {
    match side {
        FolderSide::Left => row.glyph == 'R',
        FolderSide::Right => row.glyph == 'L',
    }
}

/// Returns whether a cell or row is dimmed for an orphan entry.
/// Missing sides are dimmed (rendered as weak/dimmed placeholder dashes "-").
pub fn folder_table_cell_is_dimmed(row: &FolderRow, side: FolderSide) -> bool {
    folder_row_is_missing_on_side(row, side)
}

/// Returns the formatted cell text for a row on a given side and table column.
/// Name returns filename (with trailing "/" for dirs).
/// Size returns byte count or "-" if dir or missing.
/// Modified returns timestamp or "-" if missing.
pub fn folder_table_cell_text(row: &FolderRow, side: FolderSide, col: FolderTableCol) -> String {
    match col {
        FolderTableCol::Name => {
            let mark = if row.is_dir { "/" } else { "" };
            format!("{}{}", row.rel_path, mark)
        }
        FolderTableCol::Size => match side {
            FolderSide::Left => row.left_size.clone().unwrap_or_else(|| "-".to_string()),
            FolderSide::Right => row.right_size.clone().unwrap_or_else(|| "-".to_string()),
        },
        FolderTableCol::Modified => match side {
            FolderSide::Left => row.left_time.clone().unwrap_or_else(|| "-".to_string()),
            FolderSide::Right => row.right_time.clone().unwrap_or_else(|| "-".to_string()),
        },
    }
}

/// Returns the 3 cell strings `[Name, Size, Modified]` for a given row and side.
pub fn folder_table_row_cells(row: &FolderRow, side: FolderSide) -> [String; 3] {
    [
        folder_table_cell_text(row, side, FolderTableCol::Name),
        folder_table_cell_text(row, side, FolderTableCol::Size),
        folder_table_cell_text(row, side, FolderTableCol::Modified),
    ]
}

/// TypoRole for a folder table cell:
/// Missing sides and dim orphans use TypoRole::Small (rank 4, is_weak() == true).
/// Present content uses TypoRole::Mono (rank 3, tabular content).
pub fn folder_table_cell_typo_role(row: &FolderRow, side: FolderSide) -> crate::theme::TypoRole {
    if folder_row_is_missing_on_side(row, side) {
        crate::theme::TypoRole::Small
    } else {
        crate::theme::TypoRole::Mono
    }
}

#[cfg(test)]
mod exq_foldertable10_tests {
    use super::*;

    fn make_test_row(
        path: &str,
        is_dir: bool,
        glyph: char,
        lsize: Option<&str>,
        rsize: Option<&str>,
        ltime: Option<&str>,
        rtime: Option<&str>,
    ) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph,
            left_size: lsize.map(|s| s.to_string()),
            right_size: rsize.map(|s| s.to_string()),
            left_time: ltime.map(|s| s.to_string()),
            right_time: rtime.map(|s| s.to_string()),
            changed: glyph != '=',
        }
    }

    #[test]
    fn folder_table_grid_has_section_gap_and_headers() {
        assert!(folder_table_grid_has_section_gap());
        assert_eq!(FOLDERTABLE_COL_GAP, crate::theme::GEO_SECTION_GAP);
        assert_eq!(FOLDERTABLE_COL_GAP, 8.0);
        assert!(folder_table_cell_has_space_padding());
        assert_eq!(FOLDERTABLE_CELL_PAD_X, 8.0);
        assert_eq!(folder_table_headers(), ["Name", "Size", "Modified"]);
        assert_eq!(folder_table_col_header(FolderTableCol::Name), "Name");
        assert_eq!(folder_table_col_header(FolderTableCol::Size), "Size");
        assert_eq!(
            folder_table_col_header(FolderTableCol::Modified),
            "Modified"
        );
        assert_eq!(
            folder_table_col_align(FolderTableCol::Name),
            FolderCellAlign::Left
        );
        assert_eq!(
            folder_table_col_align(FolderTableCol::Size),
            FolderCellAlign::Right
        );
        assert_eq!(
            folder_table_col_align(FolderTableCol::Modified),
            FolderCellAlign::Left
        );
        assert_eq!(folder_table_header_weight_name(), "strong");
        assert_eq!(folder_table_header_weight_value(), 600);
    }

    #[test]
    fn folder_table_row_height_derived_from_typo_mono() {
        assert_eq!(
            folder_table_line_height_px(),
            crate::theme::TypoRole::Mono.line_height_px()
        );
        assert!((folder_table_line_height_px() - 15.6).abs() < 0.01);
        assert!((folder_table_row_height() - 17.6).abs() < 0.001);
        assert!((folder_table_row_height_for_line(15.6) - 17.6).abs() < 0.001);
        assert_eq!(
            folder_table_row_height_for_line(f32::NAN),
            FOLDER_ROW_HEIGHT
        );
        assert_eq!(folder_table_row_height_for_line(0.0), FOLDER_ROW_HEIGHT);
        assert_eq!(
            folder_table_density_row_height(crate::viewmodel::FolderDensity::Compact),
            crate::viewmodel::FOLDER_ROW_HEIGHT_COMPACT
        );
        assert_eq!(
            folder_table_density_row_height(crate::viewmodel::FolderDensity::Comfortable),
            FOLDER_ROW_HEIGHT
        );
        assert_eq!(
            folder_table_density_row_height(crate::viewmodel::FolderDensity::Spacious),
            crate::viewmodel::FOLDER_ROW_HEIGHT_SPACIOUS
        );
    }

    #[test]
    fn folder_table_column_xs_and_fixed_trailing_alignment() {
        assert_eq!(folder_table_col_min_px(FolderTableCol::Size), 86.0);
        assert_eq!(folder_table_col_min_px(FolderTableCol::Modified), 128.0);
        assert_eq!(folder_table_col_min_px(FolderTableCol::Name), 184.0);
        assert_eq!(folder_table_fixed_cols_w(), 222.0);

        let (name_x, size_x, mod_x) = folder_table_column_xs(960.0);
        assert_eq!(name_x, 0.0);
        assert_eq!(size_x, 738.0);
        assert_eq!(mod_x, 832.0);
        assert_eq!(
            mod_x + folder_table_col_min_px(FolderTableCol::Modified),
            960.0
        );

        // Verify that size_x and mod_x are independent of tree depth:
        let indent0 = folder_table_name_indent_px(0);
        let indent1 = folder_table_name_indent_px(1);
        let indent5 = folder_table_name_indent_px(5);
        assert_eq!(indent0, 40.0);
        assert_eq!(indent1, 52.0);
        assert_eq!(indent5, 100.0);
        assert!(indent5 > indent0);

        // Truncate checks
        assert!(!folder_table_name_needs_truncate(960.0, 0, 10));
        assert!(folder_table_name_needs_truncate(960.0, 5, 120));
        assert!(!folder_table_name_needs_truncate(960.0, 0, 0));
        assert!(!folder_table_name_needs_truncate(f32::NAN, 0, 50));
    }

    #[test]
    fn folder_table_orphan_rows_dimmed_without_color_edits() {
        let orphan_l = make_test_row(
            "orphan-l.txt",
            false,
            'L',
            Some("1024"),
            None,
            Some("2026-02-15 08:30"),
            None,
        );
        let orphan_r = make_test_row(
            "orphan-r.txt",
            false,
            'R',
            None,
            Some("8192"),
            None,
            Some("2026-02-20 16:45"),
        );
        let normal = make_test_row(
            "same.txt",
            false,
            '=',
            Some("512"),
            Some("512"),
            Some("2026-01-01 12:00"),
            Some("2026-01-01 12:00"),
        );

        assert!(folder_row_is_orphan(&orphan_l));
        assert!(folder_row_is_orphan(&orphan_r));
        assert!(!folder_row_is_orphan(&normal));

        // Missing sides
        assert!(!folder_row_is_missing_on_side(&orphan_l, FolderSide::Left));
        assert!(folder_row_is_missing_on_side(&orphan_l, FolderSide::Right));
        assert!(folder_row_is_missing_on_side(&orphan_r, FolderSide::Left));
        assert!(!folder_row_is_missing_on_side(&orphan_r, FolderSide::Right));
        assert!(!folder_row_is_missing_on_side(&normal, FolderSide::Left));
        assert!(!folder_row_is_missing_on_side(&normal, FolderSide::Right));

        // Dimmed checks (missing side is dimmed)
        assert!(!folder_table_cell_is_dimmed(&orphan_l, FolderSide::Left));
        assert!(folder_table_cell_is_dimmed(&orphan_l, FolderSide::Right));
        assert!(folder_table_cell_is_dimmed(&orphan_r, FolderSide::Left));
        assert!(!folder_table_cell_is_dimmed(&orphan_r, FolderSide::Right));

        // TypoRole: missing side uses Small (which has is_weak() == true), present uses Mono
        assert_eq!(
            folder_table_cell_typo_role(&orphan_l, FolderSide::Left),
            crate::theme::TypoRole::Mono
        );
        assert_eq!(
            folder_table_cell_typo_role(&orphan_l, FolderSide::Right),
            crate::theme::TypoRole::Small
        );
        assert!(folder_table_cell_typo_role(&orphan_l, FolderSide::Right).is_weak());
        assert!(!folder_table_cell_typo_role(&orphan_l, FolderSide::Left).is_weak());

        // Text cells
        let r_cells = folder_table_row_cells(&orphan_l, FolderSide::Right);
        assert_eq!(r_cells, ["orphan-l.txt", "-", "-"]);
        let l_cells = folder_table_row_cells(&orphan_l, FolderSide::Left);
        assert_eq!(l_cells, ["orphan-l.txt", "1024", "2026-02-15 08:30"]);
    }

    #[test]
    fn exq_foldertable10_edge_deep_nest_and_dimmed_orphans() {
        let chain = [
            ("root", true, '='),
            ("root/sub1", true, '='),
            ("root/sub1/sub2", true, '='),
            ("root/sub1/sub2/sub3", true, '='),
            ("root/sub1/sub2/sub3/sub4", true, '='),
            ("root/sub1/sub2/sub3/sub4/deep-file.txt", false, '>'),
        ];

        for (depth, (path, is_dir, glyph)) in chain.iter().enumerate() {
            let row = make_test_row(
                path,
                *is_dir,
                *glyph,
                Some("4096"),
                Some("2048"),
                Some("2026-03-01 12:00"),
                Some("2026-03-01 10:00"),
            );
            assert_eq!(
                folder_table_name_indent_px(depth),
                depth as f32 * tree_indent_step() + 40.0
            );
            let cells_left = folder_table_row_cells(&row, FolderSide::Left);
            let cells_right = folder_table_row_cells(&row, FolderSide::Right);
            if *is_dir {
                assert!(cells_left[0].ends_with('/'));
            }
            assert_eq!(cells_left[1], "4096");
            assert_eq!(cells_right[1], "2048");
        }

        // Deepest depth 5 indent
        assert_eq!(folder_table_name_indent_px(5), 100.0);

        // Columns layout at 960px:
        let (name_x, size_x, mod_x) = folder_table_column_xs(960.0);
        assert_eq!(name_x, 0.0);
        assert_eq!(size_x, 738.0);
        assert_eq!(mod_x, 832.0);

        // Orphan rows dimmed on missing sides:
        let orphan_l = make_test_row(
            "orphan-left.txt",
            false,
            'L',
            Some("1024"),
            None,
            Some("2026-02-15 08:30"),
            None,
        );
        let orphan_r = make_test_row(
            "orphan-right.txt",
            false,
            'R',
            None,
            Some("8192"),
            None,
            Some("2026-02-20 16:45"),
        );

        assert_eq!(
            folder_table_cell_text(&orphan_l, FolderSide::Right, FolderTableCol::Size),
            "-"
        );
        assert_eq!(
            folder_table_cell_text(&orphan_l, FolderSide::Right, FolderTableCol::Modified),
            "-"
        );
        assert!(folder_table_cell_is_dimmed(&orphan_l, FolderSide::Right));
        assert_eq!(
            folder_table_cell_typo_role(&orphan_l, FolderSide::Right),
            crate::theme::TypoRole::Small
        );

        assert_eq!(
            folder_table_cell_text(&orphan_r, FolderSide::Left, FolderTableCol::Size),
            "-"
        );
        assert_eq!(
            folder_table_cell_text(&orphan_r, FolderSide::Left, FolderTableCol::Modified),
            "-"
        );
        assert!(folder_table_cell_is_dimmed(&orphan_r, FolderSide::Left));
        assert_eq!(
            folder_table_cell_typo_role(&orphan_r, FolderSide::Left),
            crate::theme::TypoRole::Small
        );
    }
}

// ----------------------------------------------------------------------------
// TREE3 (round-3): even row rhythm + SPACE guides + char-boundary ellipsis.
// BC folder compare rows have even rhythm with indent guides and ellipsis
// that never cuts mid-word; ours had uneven row gaps and truncation
// artifacts. Geometry FIRST: row height locked to TypoRole Mono
// line-height, guide x-positions from SPACE tokens, orphan label ellipsis
// clamps to char boundaries at 960px wide. Typography SECOND: Mono content
// role reuse, Small weak for missing sides (tree2 precedent), no new sizes.
// Colors FROZEN: no hue moves here.
// WIRING (paint): har visible node ke liye row `tree_row_top(i)` se
// `tree_row_height()` tak, guides `tree_guide_xs_from_space(depth)` par,
// orphan path `tree_orphan_ellipsis_label(label, avail_w, depth)` se.
// ----------------------------------------------------------------------------

/// Tree row content role: Mono (12pt tabular, 15.6px line-height).
pub const TREE_ROW_LINE_ROLE: crate::theme::TypoRole = crate::theme::TypoRole::Mono;

/// Tree row vertical pad (px): symmetric 1.0 top + bottom (table parity).
pub const TREE_ROW_PAD_Y: f32 = FOLDERTABLE_ROW_PAD_Y;

/// Tree indent guide center offset (px): one SM step (expander 16 / 2).
pub const TREE_GUIDE_CENTER_DX: f32 = crate::theme::SPACE_SM;

/// Content line-height px locked to TypoRole Mono (12.0 * 1.30 = 15.6).
/// Pure; unit-tested.
pub fn tree_row_line_height_px() -> f32 {
    TREE_ROW_LINE_ROLE.line_height_px()
}

/// Even tree row height px locked to Mono line-height + symmetric pad:
/// 15.6 + 2.0 = 17.6. Legacy [`FOLDER_ROW_HEIGHT`] (24.0) frozen hai;
/// paint adopt kare to yahi even rhythm deta hai. Pure; unit-tested.
pub fn tree_row_height() -> f32 {
    folder_table_row_height()
}

/// Row height from an explicit line height (pad reuse, non-finite guard).
/// Pure; unit-tested.
pub fn tree_row_height_for_line(line_h: f32) -> f32 {
    folder_table_row_height_for_line(line_h)
}

/// Row top px for `row_idx`: `idx * tree_row_height()` — even rhythm,
/// koi gap drift nahi. Non-finite risk nahi (saturating cast). Pure.
pub fn tree_row_top(row_idx: usize) -> f32 {
    row_idx as f32 * tree_row_height()
}

/// Row vertical span `(y0, y1)` px for `row_idx`: poori locked height.
/// Pure; unit-tested.
pub fn tree_row_span(row_idx: usize) -> (f32, f32) {
    let top = tree_row_top(row_idx);
    (top, top + tree_row_height())
}

/// Indent step px from SPACE tokens (MD step, 12.0). Pure; unit-tested.
pub fn tree_guide_step_px() -> f32 {
    crate::theme::SPACE_MD
}

/// Ek ancestor level ki guide x px SPACE tokens se:
/// `level * SPACE_MD + SPACE_SM` (expander-center parity 8.0).
/// [`tree_indent_guide_x`] ka token-spelled twin. Pure; unit-tested.
pub fn tree_guide_x_from_space(level: usize) -> f32 {
    (level as f32) * tree_guide_step_px() + TREE_GUIDE_CENTER_DX
}

/// Saare ancestor guides ke x px SPACE tokens se (`0..depth`).
/// Pure; unit-tested.
pub fn tree_guide_xs_from_space(depth: usize) -> Vec<f32> {
    let mut xs = Vec::new();
    let mut level = 0usize;
    while level < depth {
        xs.push(tree_guide_x_from_space(level));
        level = level.saturating_add(1);
    }
    xs
}

/// Orphan/name label ke liye max chars (char-boundary budget):
/// `(name_avail - indent) / mono_advance`, floor, min 0.
/// Non-finite `avail_w` par 0. Pure; unit-tested.
pub fn tree_label_max_chars(avail_w: f32, depth: usize) -> usize {
    if !avail_w.is_finite() {
        return 0;
    }
    let name_avail = folder_table_name_avail_w(avail_w);
    let indent = folder_table_name_indent_px(depth);
    if name_avail <= indent {
        return 0;
    }
    ((name_avail - indent) / FOLDER_MONO_ADVANCE) as usize
}

/// Kya orphan label ko ellipsis chahiye (char count budget se bahar).
/// Khali label par kabhi nahi. Pure; unit-tested.
pub fn tree_orphan_label_needs_ellipsis(label: &str, avail_w: f32, depth: usize) -> bool {
    if label.is_empty() || !avail_w.is_finite() {
        return false;
    }
    label.chars().count() > tree_label_max_chars(avail_w, depth)
}

/// Word-safe separator: path/label word boundary (mid-word cut se bachao).
fn tree_label_is_sep(ch: char) -> bool {
    matches!(ch, '/' | ' ' | '-' | '_' | '.')
}

/// Orphan label ellipsis (`…`) jo char boundaries par clamp hota hai aur
/// mid-word cut se bachta hai: budget-1 chars lekar aakhri separator par
/// wapas (agar upper half me mile), warna char clamp; ek-char `…` jodta
/// hai taaki paint kabhi surrogate/byte beech me na kaate. Khali label ya
/// fit hone par as-is; budget 0 par khaali; budget 1 par sirf `…`.
/// Pure; unit-tested.
pub fn tree_orphan_ellipsis_label(label: &str, avail_w: f32, depth: usize) -> String {
    let max = tree_label_max_chars(avail_w, depth);
    if label.is_empty() {
        return String::new();
    }
    if !avail_w.is_finite() || max == 0 {
        if label.chars().count() == 0 {
            return String::new();
        }
        if tree_label_max_chars(avail_w, depth) == 0 && avail_w.is_finite() {
            return String::new();
        }
        return label.to_string();
    }
    let total = label.chars().count();
    if total <= max {
        return label.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let budget = max.saturating_sub(1);
    let prefix: String = label.chars().take(budget).collect();
    let prefix_len = prefix.chars().count();
    let mut cut: Option<usize> = None;
    let mut idx = 0usize;
    for ch in prefix.chars() {
        if tree_label_is_sep(ch) {
            cut = Some(idx.saturating_add(1));
        }
        idx = idx.saturating_add(1);
    }
    let half = budget / 2;
    let mut keep = prefix_len;
    if let Some(c) = cut
        && c >= half
        && c < prefix_len
    {
        keep = c;
    }
    let head: String = prefix.chars().take(keep).collect();
    let mut out = head;
    out.push('…');
    out
}

#[cfg(test)]
mod exq_tree3_tests {
    use super::*;

    #[test]
    fn tree3_row_height_locked_to_mono_line_height() {
        assert!((tree_row_line_height_px() - 15.6).abs() < 0.01);
        assert_eq!(
            tree_row_line_height_px(),
            crate::theme::TypoRole::Mono.line_height_px()
        );
        assert!((tree_row_height() - 17.6).abs() < 0.001);
        assert_eq!(tree_row_height(), folder_table_row_height());
        assert!((tree_row_height_for_line(15.6) - 17.6).abs() < 0.001);
        assert_eq!(tree_row_height_for_line(f32::NAN), FOLDER_ROW_HEIGHT);
        assert_eq!(TREE_ROW_PAD_Y, FOLDERTABLE_ROW_PAD_Y);
        assert_eq!(TREE_ROW_PAD_Y, 1.0);
        // Even rhythm: tops tile without drift.
        assert_eq!(tree_row_top(0), 0.0);
        assert_eq!(tree_row_top(1), tree_row_height());
        assert_eq!(tree_row_span(0), (0.0, tree_row_height()));
        let span = tree_row_span(2);
        assert!((span.1 - span.0 - tree_row_height()).abs() < 0.001);
        assert_eq!(span.0, tree_row_top(2));
    }

    #[test]
    fn tree3_guides_from_space_tokens() {
        assert_eq!(tree_guide_step_px(), crate::theme::SPACE_MD);
        assert_eq!(tree_guide_step_px(), 12.0);
        assert_eq!(TREE_GUIDE_CENTER_DX, crate::theme::SPACE_SM);
        assert_eq!(TREE_GUIDE_CENTER_DX, 8.0);
        assert_eq!(tree_guide_x_from_space(0), 8.0);
        assert_eq!(tree_guide_x_from_space(1), 20.0);
        assert_eq!(tree_guide_x_from_space(2), 32.0);
        assert_eq!(tree_guide_x_from_space(0), tree_indent_guide_x(0));
        assert_eq!(tree_guide_x_from_space(3), tree_indent_guide_x(3));
        assert_eq!(tree_guide_xs_from_space(0), tree_indent_guide_xs(0));
        assert_eq!(tree_guide_xs_from_space(3), tree_indent_guide_xs(3));
        assert_eq!(tree_guide_xs_from_space(3), vec![8.0, 20.0, 32.0]);
    }

    #[test]
    fn tree3_orphan_ellipsis_char_boundary_at_960() {
        // 960 canonical: name_avail 730; depth0 budget (730-40)/7 = 98.
        assert_eq!(folder_table_name_avail_w(960.0), 730.0);
        assert_eq!(tree_label_max_chars(960.0, 0), 98);
        assert_eq!(tree_label_max_chars(960.0, 2), 95);
        assert_eq!(tree_label_max_chars(f32::NAN, 0), 0);
        let short = "orphan-left.txt";
        assert!(!tree_orphan_label_needs_ellipsis(short, 960.0, 0));
        assert_eq!(tree_orphan_ellipsis_label(short, 960.0, 0), short);
        let long_label = "src/gui/very-long-orphan-left-filename-that-keeps-going-and-going-and-going-past-ninety-chars.rs";
        assert!(tree_orphan_label_needs_ellipsis(long_label, 960.0, 5));
        let clipped = tree_orphan_ellipsis_label(long_label, 960.0, 5);
        // Char boundary: char count within budget, ends with single ellipsis.
        let max5 = tree_label_max_chars(960.0, 5);
        assert!(clipped.chars().count() <= max5);
        assert!(clipped.chars().count() >= max5 / 2);
        assert!(clipped.ends_with('…'));
        // Mid-word safe: separator par cut, beech me word nahi tootta.
        let mid = "aaa-bbb-ccc-ddd-eee-fff-ggg-hhh-iii-jjj-kkk-lll-mmm-nnn-ooo-ppp-qqq-rrr-sss-ttt-uuu-vvv-www-xxx-yyy-zzz-end.rs";
        let mid_clip = tree_orphan_ellipsis_label(mid, 960.0, 5);
        assert!(mid_clip.ends_with('…'));
        let body = mid_clip.trim_end_matches('…');
        let next_ch = mid.chars().nth(body.chars().count());
        let last_kept = body.chars().last();
        let boundary_ok =
            next_ch.is_none_or(tree_label_is_sep) || last_kept.is_none_or(tree_label_is_sep);
        assert!(boundary_ok);
        // Non-finite budget never truncates, empty stays empty.
        assert_eq!(
            tree_orphan_ellipsis_label(long_label, f32::NAN, 0),
            long_label
        );
        assert_eq!(tree_orphan_ellipsis_label("", 960.0, 0), "");
    }

    #[test]
    fn exq_tree3_round1_tree_narrow_720_and_row_rhythm() {
        // Headless test proving edge fixture comparison/tree-narrow-720.json
        let fixture_raw =
            include_str!("../../../checks/gui-exq-tree3/round-1/comparison/tree-narrow-720.json");
        assert!(fixture_raw.contains("\"name\": \"tree-narrow-720\""));
        assert!(fixture_raw.contains("\"width_px\": 720.0"));
        assert!(fixture_raw.contains("\"line_height_px\": 15.6"));
        assert!(fixture_raw.contains("\"row_height_px\": 17.6"));
        assert!(fixture_raw.contains("\"indent_step_px\": 12.0"));
        assert!(fixture_raw.contains("\"x_px\": 498.0"));
        assert!(fixture_raw.contains("\"x_px\": 592.0"));
        // Narrow geometry: name_avail 490, size 498, mod 592, right edge 720.
        assert_eq!(folder_table_name_avail_w(720.0), 490.0);
        let (name_x, size_x, mod_x) = folder_table_column_xs(720.0);
        assert_eq!((name_x, size_x, mod_x), (0.0, 498.0, 592.0));
        assert_eq!(
            mod_x + folder_table_col_min_px(FolderTableCol::Modified),
            720.0
        );
        // Row rhythm even at narrow width: tops tile by 17.6.
        assert!((tree_row_height() - 17.6).abs() < 0.001);
        assert_eq!(tree_row_top(3), tree_row_height() * 3.0);
        // Guides unchanged narrow: SPACE tokens, depth rhythm intact.
        assert_eq!(tree_guide_xs_from_space(2), vec![8.0, 20.0]);
        assert_eq!(folder_row_path_x_separated(2), 64.0);
        // Narrow ellipsis budget: (490-64)/7 = 60 chars at depth 2.
        assert_eq!(tree_label_max_chars(720.0, 2), 60);
        let long_label =
            "src/gui/narrow-orphan-very-long-filename-that-must-clip-at-720px-width.rs";
        assert!(tree_orphan_label_needs_ellipsis(long_label, 720.0, 2));
        let clipped = tree_orphan_ellipsis_label(long_label, 720.0, 2);
        assert!(clipped.chars().count() <= 60);
        assert!(clipped.chars().count() >= 30);
        assert!(clipped.ends_with('…'));
    }
}
